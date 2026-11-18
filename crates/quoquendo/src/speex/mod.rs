//! Speex ultra-wideband decoding, ported from Speex 1.2's float decoder
//! (licence in LICENSE-speex) for the one frame layout Loquendo 7 voices use:
//! every frame of every voice is narrowband submode 7 + wideband submode 4 +
//! ultra-wideband submode 1 (492 + 352 + 36 = 880 bits, 110 bytes; checked
//! over all frames of 19 voice files). Perceptual enhancement and the output
//! high-pass are off, as LTTS7Codec210 has them, and the wideband layer hands
//! the band above twice its excitation (LTTS7Codec210.dll 0x1000f0e0).
//!
//! The arithmetic follows the C exactly, f32 where Speex uses `float` and f64
//! where its constants are C doubles, so the output is the C decoder's to the
//! last bit on the same libm.

mod tables;
use tables::*;

/// Samples per decoded frame at 32 kHz.
pub const FRAME_SAMPLES: usize = 640;
/// Bytes per frame in the voices' recordings.
pub const FRAME_BYTES: usize = 110;

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Bits<'_> {
    fn get(&mut self, n: usize) -> u32 {
        let mut d = 0;
        for _ in 0..n {
            let bit = self.data.get(self.pos / 8).map(|b| (b >> (7 - self.pos % 8)) & 1).unwrap_or(0);
            d = d << 1 | bit as u32;
            self.pos += 1;
        }
        d
    }
}

// --- shared pieces (lsp.c, filters.c, quant_lsp.c, cb_search.c) -------------

// The constants as Speex writes them (its SPX_PI_2 is a rounded π/2, kept).
#[allow(clippy::excessive_precision, clippy::approx_constant)]
fn spx_cos(x: f32) -> f32 {
    const C1: f32 = 0.9999932946;
    const C2: f32 = -0.4999124376;
    const C3: f32 = 0.0414877472;
    const C4: f32 = -0.0012712095;
    if (x as f64) < 1.5707963268 {
        let x = x * x;
        C1 + x * (C2 + x * (C3 + C4 * x))
    } else {
        let x = (std::f64::consts::PI - x as f64) as f32;
        let x = x * x;
        -(C1 + x * (C2 + x * (C3 + C4 * x)))
    }
}

fn lsp_to_lpc(freq: &[f32], ak: &mut [f32]) {
    let order = freq.len();
    let m = order / 2;
    let mut wp = vec![0f32; 4 * m + 2];
    let x: Vec<f32> = freq.iter().map(|&f| spx_cos(f)).collect();
    let (mut xin1, mut xin2) = (1f32, 1f32);
    for j in 0..=order {
        for i in 0..m {
            let n = 4 * i;
            let xout1 = xin1 - 2.0 * x[2 * i] * wp[n] + wp[n + 1];
            let xout2 = xin2 - 2.0 * x[2 * i + 1] * wp[n + 2] + wp[n + 3];
            wp[n + 1] = wp[n];
            wp[n + 3] = wp[n + 2];
            wp[n] = xin1;
            wp[n + 2] = xin2;
            xin1 = xout1;
            xin2 = xout2;
        }
        let xout1 = xin1 + wp[4 * m];
        let xout2 = xin2 - wp[4 * m + 1];
        if j > 0 {
            ak[j - 1] = (xout1 + xout2) * 0.5;
        }
        wp[4 * m] = xin1;
        wp[4 * m + 1] = xin2;
        xin1 = 0.0;
        xin2 = 0.0;
    }
}

fn lsp_interpolate(old: &[f32], new: &[f32], lsp: &mut [f32], sub: usize, nb_subframes: usize, margin: f32) {
    let len = lsp.len();
    let tmp = (1.0f32 + sub as f32) / nb_subframes as f32;
    for i in 0..len {
        lsp[i] = (1.0 - tmp) * old[i] + tmp * new[i];
    }
    if lsp[0] < margin {
        lsp[0] = margin;
    }
    let top = std::f64::consts::PI - margin as f64;
    if lsp[len - 1] as f64 > top {
        lsp[len - 1] = top as f32;
    }
    for i in 1..len - 1 {
        if lsp[i] < lsp[i - 1] + margin {
            lsp[i] = lsp[i - 1] + margin;
        }
        if lsp[i] > lsp[i + 1] - margin {
            lsp[i] = 0.5 * (lsp[i] + lsp[i + 1] - margin);
        }
    }
}

/// `lsp += step·cb` done in double, as the C macros (0.0039062*(x)) are.
fn add_lsp(lsp: &mut f32, step: f64, cb: i8) {
    *lsp = (*lsp as f64 + step * cb as f64) as f32;
}

const LSP_DIV_256: f64 = 0.0039062;
const LSP_DIV_512: f64 = 0.0019531;
const LSP_DIV_1024: f64 = 0.00097656;

fn lsp_unquant_high(bits: &mut Bits) -> [f32; 8] {
    let mut lsp: [f32; 8] = std::array::from_fn(|i| (0.3125 * i as f64 + 0.75) as f32);
    let id = bits.get(6) as usize;
    for i in 0..8 {
        add_lsp(&mut lsp[i], LSP_DIV_256, HIGH_LSP_CDBK[id * 8 + i]);
    }
    let id = bits.get(6) as usize;
    for i in 0..8 {
        add_lsp(&mut lsp[i], LSP_DIV_512, HIGH_LSP_CDBK2[id * 8 + i]);
    }
    lsp
}

/// split_cb_shape_sign_unquant: `exc += s·0.03125·shape`.
fn split_cb_unquant(exc: &mut [f32], bits: &mut Bits, subvect: usize, nb_subvect: usize, shape_bits: usize, have_sign: bool, cb: &[i8]) {
    let mut picks = [(0u32, 0usize); 8];
    for p in picks.iter_mut().take(nb_subvect) {
        let sign = if have_sign { bits.get(1) } else { 0 };
        *p = (sign, bits.get(shape_bits) as usize);
    }
    for (i, &(sign, ind)) in picks.iter().take(nb_subvect).enumerate() {
        let s: f64 = if sign != 0 { -1.0 } else { 1.0 };
        for j in 0..subvect {
            let e = &mut exc[subvect * i + j];
            *e = (*e as f64 + s * 0.03125 * cb[ind * subvect + j] as f64) as f32;
        }
    }
}

fn iir_mem16(x: &[f32], den: &[f32], y: &mut [f32], mem: &mut [f32]) {
    let ord = den.len();
    for i in 0..x.len() {
        let yi = x[i] + mem[0];
        let nyi = -yi;
        for j in 0..ord - 1 {
            mem[j] = mem[j + 1] + den[j] * nyi;
        }
        mem[ord - 1] = den[ord - 1] * nyi;
        y[i] = yi;
    }
}

/// The same filter, in place.
fn iir_mem16_inplace(xy: &mut [f32], den: &[f32], mem: &mut [f32]) {
    let x = xy.to_vec();
    iir_mem16(&x, den, xy, mem);
}

fn compute_rms(x: &[f32]) -> f32 {
    let mut sum = 0f32;
    for &v in x {
        sum += v * v;
    }
    (0.1 + (sum / x.len() as f32) as f64).sqrt() as f32
}

const H0: [f32; 64] = [
    3.596189e-05,
    -0.0001123515,
    -0.0001104587,
    0.0002790277,
    0.0002298438,
    -0.0005953563,
    -0.0003823631,
    0.00113826,
    0.0005308539,
    -0.001986177,
    -0.0006243724,
    0.003235877,
    0.0005743159,
    -0.004989147,
    -0.0002584767,
    0.007367171,
    -0.0004857935,
    -0.01050689,
    0.001894714,
    0.01459396,
    -0.004313674,
    -0.01994365,
    0.00828756,
    0.02716055,
    -0.01485397,
    -0.03764973,
    0.026447,
    0.05543245,
    -0.05095487,
    -0.09779096,
    0.1382363,
    0.4600981,
    0.4600981,
    0.1382363,
    -0.09779096,
    -0.05095487,
    0.05543245,
    0.026447,
    -0.03764973,
    -0.01485397,
    0.02716055,
    0.00828756,
    -0.01994365,
    -0.004313674,
    0.01459396,
    0.001894714,
    -0.01050689,
    -0.0004857935,
    0.007367171,
    -0.0002584767,
    -0.004989147,
    0.0005743159,
    0.003235877,
    -0.0006243724,
    -0.001986177,
    0.0005308539,
    0.00113826,
    -0.0003823631,
    -0.0005953563,
    0.0002298438,
    0.0002790277,
    -0.0001104587,
    -0.0001123515,
    3.596189e-05,
];

/// The QMF synthesis joining a low and a high band (both `n/2` long) into `y`.
fn qmf_synth(x1: &[f32], x2: &[f32], y: &mut [f32], mem1: &mut [f32; 64], mem2: &mut [f32; 64]) {
    let (n2, m2) = (x1.len(), 32);
    let mut xx1 = vec![0f32; m2 + n2];
    let mut xx2 = vec![0f32; m2 + n2];
    for i in 0..n2 {
        xx1[i] = x1[n2 - 1 - i];
        xx2[i] = x2[n2 - 1 - i];
    }
    for i in 0..m2 {
        xx1[n2 + i] = mem1[2 * i + 1];
        xx2[n2 + i] = mem2[2 * i + 1];
    }
    let a = &H0;
    for i in (0..n2).step_by(2) {
        let (mut y0, mut y1, mut y2, mut y3) = (0f32, 0f32, 0f32, 0f32);
        let mut x10 = xx1[n2 - 2 - i];
        let mut x20 = xx2[n2 - 2 - i];
        for j in (0..m2).step_by(2) {
            let (a0, a1) = (a[2 * j], a[2 * j + 1]);
            let x11 = xx1[n2 - 1 + j - i];
            let x21 = xx2[n2 - 1 + j - i];
            y0 += a0 * (x11 - x21);
            y1 += a1 * (x11 + x21);
            y2 += a0 * (x10 - x20);
            y3 += a1 * (x10 + x20);
            let (a0, a1) = (a[2 * j + 2], a[2 * j + 3]);
            x10 = xx1[n2 + j - i];
            x20 = xx2[n2 + j - i];
            y0 += a0 * (x10 - x20);
            y1 += a1 * (x10 + x20);
            y2 += a0 * (x11 - x21);
            y3 += a1 * (x11 + x21);
        }
        y[2 * i] = 2.0 * y0;
        y[2 * i + 1] = 2.0 * y1;
        y[2 * i + 2] = 2.0 * y2;
        y[2 * i + 3] = 2.0 * y3;
    }
    for i in 0..m2 {
        mem1[2 * i + 1] = xx1[i];
        mem2[2 * i + 1] = xx2[i];
    }
}

// --- narrowband, submode 7 (nb_celp.c nb_decode) -----------------------------

const NB_ORDER: usize = 10;
const NB_FRAME: usize = 160;
const NB_SUB: usize = 40;
const NB_PITCH_START: usize = 17;
const NB_PITCH_END: usize = 144;
/// `st->exc` inside `excBuf` (2·pitch_end + subframe + 6).
const EXC: usize = 2 * NB_PITCH_END + NB_SUB + 6;
const EXC_GAIN_QUANT_SCAL3: [f32; 8] = [0.061130, 0.163546, 0.310413, 0.428220, 0.555887, 0.719055, 0.938694, 1.326874];

struct Narrowband {
    exc_buf: [f32; NB_FRAME + 2 * NB_PITCH_END + NB_SUB + 12],
    interp_qlpc: [f32; NB_ORDER],
    mem_sp: [f32; NB_ORDER],
    old_qlsp: [f32; NB_ORDER],
    pi_gain: [f32; 4],
    first: bool,
}

impl Narrowband {
    fn new() -> Self {
        Narrowband {
            exc_buf: [0.0; NB_FRAME + 2 * NB_PITCH_END + NB_SUB + 12],
            interp_qlpc: [0.0; NB_ORDER],
            mem_sp: [0.0; NB_ORDER],
            old_qlsp: [0.0; NB_ORDER],
            pi_gain: [0.0; 4],
            first: true,
        }
    }

    fn lsp_unquant(bits: &mut Bits) -> [f32; NB_ORDER] {
        let mut lsp: [f32; NB_ORDER] = std::array::from_fn(|i| (0.25 * i as f64 + 0.25) as f32);
        let id = bits.get(6) as usize;
        for i in 0..10 {
            add_lsp(&mut lsp[i], LSP_DIV_256, CDBK_NB[id * 10 + i]);
        }
        let id = bits.get(6) as usize;
        for i in 0..5 {
            add_lsp(&mut lsp[i], LSP_DIV_512, CDBK_NB_LOW1[id * 5 + i]);
        }
        let id = bits.get(6) as usize;
        for i in 0..5 {
            add_lsp(&mut lsp[i], LSP_DIV_1024, CDBK_NB_LOW2[id * 5 + i]);
        }
        let id = bits.get(6) as usize;
        for i in 0..5 {
            add_lsp(&mut lsp[i + 5], LSP_DIV_512, CDBK_NB_HIGH1[id * 5 + i]);
        }
        let id = bits.get(6) as usize;
        for i in 0..5 {
            add_lsp(&mut lsp[i + 5], LSP_DIV_1024, CDBK_NB_HIGH2[id * 5 + i]);
        }
        lsp
    }

    /// Decode the low 0-4 kHz into `out` and its innovation into `innov_save`.
    fn decode(&mut self, bits: &mut Bits, out: &mut [f32], innov_save: &mut [f32]) {
        bits.get(1 + 4); // narrowband marker and submode (7, checked by the caller)
        self.exc_buf.copy_within(NB_FRAME..NB_FRAME + 2 * NB_PITCH_END + NB_SUB + 12, 0);
        let qlsp = Self::lsp_unquant(bits);
        if self.first {
            self.old_qlsp = qlsp;
        }
        let qe = bits.get(5);
        let ol_gain = (qe as f64 / 3.5).exp() as f32;
        for sub in 0..4 {
            let offset = NB_SUB * sub;
            let base = EXC + offset;
            self.exc_buf[base..base + NB_SUB].fill(0.0);
            // adaptive codebook: pitch_unquant_3tap with ltp_params_nb
            let pitch = NB_PITCH_START + bits.get(7) as usize;
            let gi = bits.get(7) as usize;
            let gain: [f32; 3] = std::array::from_fn(|k| (0.015625 * GAIN_CDBK_NB[gi * 4 + k] as f64 + 0.5) as f32);
            let mut exc32 = [0f32; NB_SUB];
            for i in 0..3 {
                let pp = pitch + 1 - i;
                let tmp1 = NB_SUB.min(pp);
                for (j, e) in exc32.iter_mut().enumerate().take(tmp1) {
                    *e += gain[2 - i] * self.exc_buf[base + j - pp];
                }
                let tmp3 = NB_SUB.min(pp + pitch);
                for (j, e) in exc32.iter_mut().enumerate().take(tmp3).skip(tmp1) {
                    *e += gain[2 - i] * self.exc_buf[base + j - pp - pitch];
                }
            }
            for e in exc32.iter_mut() {
                if !(*e >= -32000.0 && *e <= 32000.0) {
                    *e = if *e < -32000.0 {
                        -32000.0
                    } else if *e > 32000.0 {
                        32000.0
                    } else {
                        0.0
                    };
                }
            }
            // innovation: two split codebooks, the second at 0.454545 of the gain
            let ener = EXC_GAIN_QUANT_SCAL3[bits.get(3) as usize] * ol_gain;
            let mut innov = [0f32; NB_SUB];
            split_cb_unquant(&mut innov, bits, 5, 8, 6, false, &EXC_5_64_TABLE);
            innov.iter_mut().for_each(|v| *v *= ener);
            let mut innov2 = [0f32; NB_SUB];
            split_cb_unquant(&mut innov2, bits, 5, 8, 6, false, &EXC_5_64_TABLE);
            let ener2 = 0.454545f32 * ener;
            innov2.iter_mut().for_each(|v| *v *= ener2);
            for i in 0..NB_SUB {
                innov[i] += innov2[i];
                self.exc_buf[base + i] = exc32[i] + innov[i];
                innov_save[offset + i] = innov[i];
            }
        }
        // no enhancement: the output is the excitation one subframe back
        out[..NB_FRAME].copy_from_slice(&self.exc_buf[EXC - NB_SUB..EXC - NB_SUB + NB_FRAME]);
        let mut interp = [0f32; NB_ORDER];
        let mut ak = [0f32; NB_ORDER];
        for sub in 0..4 {
            lsp_interpolate(&self.old_qlsp, &qlsp, &mut interp, sub, 4, 0.002);
            lsp_to_lpc(&interp, &mut ak);
            let mut pi_g = 1f32;
            for i in (0..NB_ORDER).step_by(2) {
                pi_g += ak[i + 1] - ak[i];
            }
            self.pi_gain[sub] = pi_g;
            iir_mem16_inplace(&mut out[NB_SUB * sub..NB_SUB * (sub + 1)], &self.interp_qlpc, &mut self.mem_sp);
            self.interp_qlpc = ak;
        }
        self.old_qlsp = qlsp;
        self.first = false;
    }

    /// RMS of each subframe's excitation (SPEEX_GET_EXC).
    fn exc_rms(&self, sub: usize) -> f32 {
        compute_rms(&self.exc_buf[EXC + sub * NB_SUB..EXC + (sub + 1) * NB_SUB])
    }
}

// --- the sub-band layers (sb_celp.c sb_decode) --------------------------------

const GC_QUANT_BOUND: [f64; 16] = [
    0.97979, 1.28384, 1.68223, 2.20426, 2.88829, 3.78458, 4.95900, 6.49787, 8.51428, 11.15642, 14.61846, 19.15484, 25.09895, 32.88761, 43.09325, 56.46588,
];

/// One high band: 8 LPCs, 4 subframes of `sub` samples.
struct HighBand<const SUB: usize> {
    exc_buf: [f32; SUB],
    old_qlsp: [f32; 8],
    interp_qlpc: [f32; 8],
    mem_sp: [f32; 8],
    pi_gain: [f32; 4],
    g0_mem: [f32; 64],
    g1_mem: [f32; 64],
    first: bool,
}

impl<const SUB: usize> HighBand<SUB> {
    fn new() -> Self {
        HighBand {
            exc_buf: [0.0; SUB],
            old_qlsp: [0.0; 8],
            interp_qlpc: [0.0; 8],
            mem_sp: [0.0; 8],
            pi_gain: [0.0; 4],
            g0_mem: [0.0; 64],
            g1_mem: [0.0; 64],
            first: true,
        }
    }

    /// Decode the band above `low` (`2·SUB·4` samples of `out`: low half
    /// already decoded, high half holding the low layer's innovation), then
    /// join both halves. `exc_of` gives each subframe's high-band excitation
    /// from the bits, the filter ratio and the subframe's low innovation.
    fn decode(
        &mut self,
        bits: &mut Bits,
        out: &mut [f32],
        low_pi_gain: &[f32; 4],
        mut exc_of: impl FnMut(&mut Bits, usize, f32, &[f32]) -> [f32; SUB],
        mut innov_save: Option<&mut [f32]>,
    ) {
        let frame = SUB * 4;
        bits.get(1 + 3); // wideband marker and submode (checked by the caller)
        let qlsp = lsp_unquant_high(bits);
        if self.first {
            self.old_qlsp = qlsp;
        }
        let mut interp = [0f32; 8];
        let mut ak = [0f32; 8];
        for sub in 0..4 {
            let offset = SUB * sub;
            lsp_interpolate(&self.old_qlsp, &qlsp, &mut interp, sub, 4, 0.05f64 as f32);
            lsp_to_lpc(&interp, &mut ak);
            self.pi_gain[sub] = 1.0;
            let mut rh = 1f32;
            for i in (0..8).step_by(2) {
                rh += ak[i + 1] - ak[i];
                self.pi_gain[sub] += ak[i] + ak[i + 1];
            }
            let filter_ratio = ((low_pi_gain[sub] as f64 + 0.01) / (rh as f64 + 0.01)) as f32;
            let exc = exc_of(bits, sub, filter_ratio, &out[frame + offset..frame + offset + SUB]);
            if let Some(save) = innov_save.as_deref_mut() {
                let save = &mut save[2 * offset..2 * offset + 2 * SUB];
                save.fill(0.0);
                for i in 0..SUB {
                    // Loquendo's Speex: twice the excitation (Speex 1.2: once)
                    save[2 * i] = exc[i] + exc[i];
                }
            }
            iir_mem16(
                &self.exc_buf,
                &self.interp_qlpc,
                &mut out[frame + offset..frame + offset + SUB],
                &mut self.mem_sp,
            );
            self.exc_buf = exc;
            self.interp_qlpc = ak;
        }
        let (low, high) = out.split_at(frame);
        let (low, high) = (low.to_vec(), high[..frame].to_vec());
        qmf_synth(&low, &high, &mut out[..2 * frame], &mut self.g0_mem, &mut self.g1_mem);
        self.old_qlsp = qlsp;
        self.first = false;
    }
}

/// A Speex ultra-wideband decoder in Loquendo's configuration.
pub struct Decoder {
    nb: Narrowband,
    wb: HighBand<40>,
    uwb: HighBand<80>,
}

impl Default for Decoder {
    fn default() -> Self {
        Decoder {
            nb: Narrowband::new(),
            wb: HighBand::new(),
            uwb: HighBand::new(),
        }
    }
}

impl Decoder {
    /// Whether a frame has the layout this decoder reads.
    pub fn supports(frame: &[u8]) -> bool {
        let mut b = Bits { data: frame, pos: 0 };
        let nb = b.get(5);
        b.pos = 492;
        let wb = b.get(4);
        b.pos = 492 + 352;
        let uwb = b.get(4);
        frame.len() == FRAME_BYTES && nb == 7 && wb == 0b1100 && uwb == 0b1001
    }

    /// Decode one 110-byte frame into 640 samples at 32 kHz.
    pub fn decode(&mut self, frame: &[u8], pcm: &mut [i16]) {
        let mut bits = Bits { data: frame, pos: 0 };
        let mut out = [0f32; FRAME_SAMPLES];
        // wideband: narrowband in out[0..160], its innovation in out[160..320];
        // the wideband innovation goes to out[320..640] for the band above
        {
            let (wb_out, wb_innov) = out.split_at_mut(320);
            let (nb_out, nb_innov) = wb_out.split_at_mut(160);
            self.nb.decode(&mut bits, nb_out, nb_innov);
            let nb = &self.nb;
            let exc_rms: [f32; 4] = std::array::from_fn(|s| nb.exc_rms(s));
            let low_pi_gain = nb.pi_gain;
            self.wb.decode(
                &mut bits,
                wb_out,
                &low_pi_gain,
                |bits, sub, filter_ratio, _| {
                    let mut exc = [0f32; 40];
                    let qgc = bits.get(4) as usize;
                    let gc = (0.87360 * (GC_QUANT_BOUND[qgc] as f32) as f64) as f32;
                    let scale = gc * exc_rms[sub] / filter_ratio;
                    split_cb_unquant(&mut exc, bits, 8, 5, 7, true, &HEXC_TABLE);
                    exc.iter_mut().for_each(|v| *v *= scale);
                    let mut innov2 = [0f32; 40];
                    split_cb_unquant(&mut innov2, bits, 8, 5, 7, true, &HEXC_TABLE);
                    let scale2 = 0.4f32 * scale;
                    for i in 0..40 {
                        exc[i] += innov2[i] * scale2;
                    }
                    exc
                },
                Some(wb_innov),
            );
        }
        // ultra-wideband: no innovation, the wideband one folded up
        let low_pi_gain = self.wb.pi_gain;
        self.uwb.decode(
            &mut bits,
            &mut out,
            &low_pi_gain,
            |bits, _, filter_ratio, low_innov| {
                let quant = bits.get(5) as i32;
                let g = ((0.125f32 * (quant - 10) as f32) as f64).exp() as f32 / filter_ratio;
                let mut exc = [0f32; 80];
                for i in (0..80).step_by(2) {
                    exc[i] = 0.7f32 * low_innov[i] * g;
                    exc[i + 1] = -(0.7f32 * low_innov[i + 1] * g);
                }
                exc
            },
            None,
        );
        for (p, &v) in pcm.iter_mut().zip(out.iter()) {
            *p = if v > 32767.0 {
                32767
            } else if v < -32768.0 {
                -32768
            } else {
                (0.5 + v as f64).floor() as i16
            };
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    /// Frames of pseudo-random bits with the voices' layout headers.
    pub(crate) fn synthetic_frames(n: usize) -> Vec<Vec<u8>> {
        let mut seed: u32 = 12345;
        let mut set = |f: &mut [u8], pos: usize, bits: &[u8]| {
            for (k, &b) in bits.iter().enumerate() {
                let (i, s) = ((pos + k) / 8, 7 - (pos + k) % 8);
                f[i] = f[i] & !(1 << s) | b << s;
            }
        };
        (0..n)
            .map(|_| {
                let mut f: Vec<u8> = (0..super::FRAME_BYTES)
                    .map(|_| {
                        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                        (seed >> 24) as u8
                    })
                    .collect();
                set(&mut f, 0, &[0, 0, 1, 1, 1]);
                set(&mut f, 492, &[1, 1, 0, 0]);
                set(&mut f, 844, &[1, 0, 0, 1]);
                f
            })
            .collect()
    }

    pub(crate) fn fnv(samples: &[i16]) -> u64 {
        samples
            .iter()
            .fold(0xcbf2_9ce4_8422_2325u64, |h, &s| (h ^ s as u16 as u64).wrapping_mul(0x100_0000_01b3))
    }

    /// The output libspeex 1.2 (with Loquendo's change) gave for these frames.
    #[test]
    fn decodes_like_libspeex() {
        let mut d = super::Decoder::default();
        let mut all = Vec::new();
        let mut pcm = [0i16; super::FRAME_SAMPLES];
        for f in synthetic_frames(200) {
            assert!(super::Decoder::supports(&f));
            d.decode(&f, &mut pcm);
            all.extend_from_slice(&pcm);
        }
        assert_eq!(fnv(&all), LIBSPEEX_HASH);
    }

    const LIBSPEEX_HASH: u64 = 0x96c2_f690_f2d6_a398;
}
