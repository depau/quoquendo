//! Speex ultra-wideband decoding as Loquendo's two codecs do it.
//!
//! LTTS7Codec210.dll (the 32 kHz voices) is Speex 1.1 (licence in
//! LICENSE-speex) compiled for 32-bit x87, read from the DLL (MECHANICS.md
//! §2). Only the one frame layout Loquendo 7 voices use: narrowband submode
//! 7 + wideband submode 4 + ultra-wideband submode 1 (492 + 352 + 36 = 880
//! bits, 110 bytes; every frame of 19 voice files). LTTS7CodecMsx.dll (the 44
//! kHz voices) is in [`msx`].
//!
//! Precision follows the DLLs' code: each expression is evaluated on the FPU
//! at double precision and rounded to `float` when it is assigned (the
//! compiler stores every assignment with `fstp dword`), so here expressions
//! are computed in f64 and every assignment goes through [`f`].

pub mod msx;
mod tables;
use std::f64::consts::PI;
use tables::*;

/// Which of Loquendo's two Speex builds a recordings file is for, named in
/// its text header (field 5: `LTTS7Codec210` or `LTTS7CodecMsx`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    Codec210,
    Msx,
}

/// Samples per decoded frame at 32 kHz.
pub const FRAME_SAMPLES: usize = 640;
/// Bytes per frame in the voices' recordings.
pub const FRAME_BYTES: usize = 110;

/// An assignment to a `float`: the FPU's double-precision value rounded.
#[inline]
fn f(x: f64) -> f32 {
    x as f32
}

/// A sample to 16 bits: `floor(x + 0.5)`, clamped (0x1000f640, 0x10001500).
fn to_i16(v: f32) -> i16 {
    if v > 32767.0 {
        32767
    } else if v < -32768.0 {
        -32768
    } else {
        (v as f64 + 0.5).floor() as i16
    }
}

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

// --- shared pieces ------------------------------------------------------------

/// Speex's cosine approximation (0x10004af0).
#[allow(clippy::excessive_precision, clippy::approx_constant)]
fn spx_cos(x: f32) -> f32 {
    const C1: f64 = 0.9999933242797852;
    const C2: f64 = 0.49991244077682495;
    const C3: f64 = 0.04148774594068527;
    const C4: f64 = 0.0012712094467133284;
    let poly = |x2: f32| {
        let x2 = x2 as f64;
        ((C3 - C4 * x2) * x2 - C2) * x2 + C1
    };
    if (x as f64) < 1.5707963268 {
        f(poly(f(x as f64 * x as f64)))
    } else {
        let t = f(PI - x as f64) as f64;
        f(-poly(f(t * t)))
    }
}

/// lsp_to_lpc (0x10005080).
fn lsp_to_lpc(freq: &[f32], ak: &mut [f32]) {
    let order = freq.len();
    let m = order / 2;
    let mut wp = vec![0f32; 4 * m + 2];
    let x: Vec<f64> = freq.iter().map(|&v| spx_cos(v) as f64).collect();
    let (mut xin1, mut xin2) = (1f32, 1f32);
    for j in 0..=order {
        for i in 0..m {
            let n = 4 * i;
            let xout1 = f(xin1 as f64 - x[2 * i] * 2.0 * wp[n] as f64 + wp[n + 1] as f64);
            let xout2 = f(xin2 as f64 - x[2 * i + 1] * 2.0 * wp[n + 2] as f64 + wp[n + 3] as f64);
            wp[n + 1] = wp[n];
            wp[n + 3] = wp[n + 2];
            wp[n] = xin1;
            wp[n + 2] = xin2;
            xin1 = xout1;
            xin2 = xout2;
        }
        let xout1 = f(xin1 as f64 + wp[4 * m] as f64);
        let xout2 = f(xin2 as f64 - wp[4 * m + 1] as f64);
        if j > 0 {
            ak[j - 1] = f((xout1 as f64 + xout2 as f64) * 0.5);
        }
        wp[4 * m] = xin1;
        wp[4 * m + 1] = xin2;
        xin1 = 0.0;
        xin2 = 0.0;
    }
}

/// lsp_interpolate (0x10005520) then lsp_enforce_margin (0x10005390).
fn lsp_interpolate(old: &[f32], new: &[f32], lsp: &mut [f32], sub: usize, nb_subframes: usize, margin: f32) {
    let tmp = f((sub as f64 + 1.0) / nb_subframes as f64) as f64;
    for i in 0..lsp.len() {
        lsp[i] = f(new[i] as f64 * tmp + old[i] as f64 * (1.0 - tmp));
    }
    let (len, m) = (lsp.len(), margin as f64);
    if lsp[0] < margin {
        lsp[0] = margin;
    }
    if PI - m < lsp[len - 1] as f64 {
        lsp[len - 1] = f(PI - m);
    }
    for i in 1..len - 1 {
        if (lsp[i] as f64) < lsp[i - 1] as f64 + m {
            lsp[i] = f(lsp[i - 1] as f64 + m);
        }
        if (lsp[i + 1] as f64 - m) < lsp[i] as f64 {
            lsp[i] = f((lsp[i] as f64 + lsp[i + 1] as f64 - m) * 0.5);
        }
    }
}

fn add_lsp(lsp: &mut f32, step: f64, cb: i8) {
    *lsp = f(cb as f64 * step + *lsp as f64);
}

const LSP_DIV_256: f64 = 0.0039062;
const LSP_DIV_512: f64 = 0.0019531;
const LSP_DIV_1024: f64 = 0.00097656;

/// lsp_unquant_high (0x1000c0d0).
fn lsp_unquant_high(bits: &mut Bits) -> [f32; 8] {
    let mut lsp: [f32; 8] = std::array::from_fn(|i| f(i as f64 * 0.3125 + 0.75));
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

/// split_cb_shape_sign_unquant (0x100029c0): `exc += shape·(s·scale)`;
/// Codec210's scale is 0.03125, CodecMsx's 1/128 from the file (0x10001190).
fn split_cb_unquant(exc: &mut [f32], bits: &mut Bits, subvect: usize, nb_subvect: usize, shape_bits: usize, have_sign: bool, cb: &[i8], scale: f64) {
    let mut picks = [(0u32, 0usize); 8];
    for p in picks.iter_mut().take(nb_subvect) {
        let sign = if have_sign { bits.get(1) } else { 0 };
        *p = (sign, bits.get(shape_bits) as usize);
    }
    for (i, &(sign, ind)) in picks.iter().take(nb_subvect).enumerate() {
        let s = if sign != 0 { -1.0 } else { 1.0 } * scale;
        for j in 0..subvect {
            let e = &mut exc[subvect * i + j];
            *e = f(cb[ind * subvect + j] as f64 * s + *e as f64);
        }
    }
}

/// signal_mul (0x10002da0).
fn signal_mul(x: &mut [f32], scale: f32) {
    for v in x.iter_mut() {
        *v = f(*v as f64 * scale as f64);
    }
}

/// iir_mem2 (0x100030e0), in place.
fn iir_mem2(xy: &mut [f32], den: &[f32], mem: &mut [f32]) {
    let ord = den.len();
    for v in xy.iter_mut() {
        let yi = f(*v as f64 + mem[0] as f64);
        let nyi = f(-(yi as f64)) as f64;
        for j in 0..ord - 1 {
            mem[j] = f(den[j] as f64 * nyi + mem[j + 1] as f64);
        }
        mem[ord - 1] = f(nyi * den[ord - 1] as f64);
        *v = yi;
    }
}

/// compute_rms (0x10002ec0).
fn compute_rms(x: &[f32]) -> f32 {
    let mut sum = 0f32;
    for &v in x {
        sum = f(v as f64 * v as f64 + sum as f64);
    }
    f((sum as f64 / x.len() as f64 + 0.1).sqrt())
}

/// h0 as stored in LTTS7Codec210.dll at 0x100240a8 (float bits).
const H0: [u32; 64] = [
    0x3816d5c9, 0xb8eb9e41, 0xb8e7a610, 0x39924a77, 0x3971023a, 0xba1c11af, 0xb9c877e8, 0x3a9531ab, 0x3a0b2901, 0xbb022a85, 0xba23acec, 0x3b541102, 0x3a168db0,
    0xbba37c00, 0xb9878428, 0x3bf1684f, 0xb9feb21a, 0xbc2c2517, 0x3af8580d, 0x3c6f1b81, 0xbb8d59b8, 0xbca360de, 0x3c07c88c, 0x3cde7fcd, 0xbc735e11, 0xbd1a369a,
    0x3cd8a761, 0x3d630d23, 0xbd50b60e, 0xbdc846a0, 0x3e0d8dd1, 0x3eeb91fa, 0x3eeb91fa, 0x3e0d8dd1, 0xbdc846a0, 0xbd50b60e, 0x3d630d23, 0x3cd8a761, 0xbd1a369a,
    0xbc735e11, 0x3cde7fcd, 0x3c07c88c, 0xbca360de, 0xbb8d59b8, 0x3c6f1b81, 0x3af8580d, 0xbc2c2517, 0xb9feb21a, 0x3bf1684f, 0xb9878428, 0xbba37c00, 0x3a168db0,
    0x3b541102, 0xba23acec, 0xbb022a85, 0x3a0b2901, 0x3a9531ab, 0xb9c877e8, 0xba1c11af, 0x3971023a, 0x39924a77, 0xb8e7a610, 0xb8eb9e41, 0x3816d5c9,
];

/// The QMF filters: h0, and h1 (h0 with its odd taps negated).
fn qmf(odd_sign: f32) -> [f32; 64] {
    std::array::from_fn(|i| f32::from_bits(H0[i]) * if i % 2 == 1 { odd_sign } else { 1.0 })
}

/// fir_mem_up (0x10003960): one band up-sampled by two through `a`.
fn fir_mem_up(x: &[f32], a: &[f32; 64], y: &mut [f32], mem: &mut [f32; 64]) {
    let n = 2 * x.len();
    let m = 64;
    let mut xx = vec![0f32; m + n - 1];
    for i in 0..n / 2 {
        xx[2 * i] = x[n / 2 - 1 - i];
    }
    for i in (0..m - 1).step_by(2) {
        xx[n + i] = mem[i + 1];
    }
    let mac = |acc: f32, a: f32, x: f32| f(a as f64 * x as f64 + acc as f64);
    for i in (0..n).step_by(4) {
        let (mut y0, mut y1, mut y2, mut y3) = (0f32, 0f32, 0f32, 0f32);
        let mut x0 = xx[n - 4 - i];
        for j in (0..m).step_by(4) {
            let (a0, a1) = (a[j], a[j + 1]);
            let x1 = xx[n - 2 + j - i];
            y0 = mac(y0, a0, x1);
            y1 = mac(y1, a1, x1);
            y2 = mac(y2, a0, x0);
            y3 = mac(y3, a1, x0);
            let (a0, a1) = (a[j + 2], a[j + 3]);
            x0 = xx[n + j - i];
            y0 = mac(y0, a0, x0);
            y1 = mac(y1, a1, x0);
            y2 = mac(y2, a0, x1);
            y3 = mac(y3, a1, x1);
        }
        y[i] = y0;
        y[i + 1] = y1;
        y[i + 2] = y2;
        y[i + 3] = y3;
    }
    for i in (0..m - 1).step_by(2) {
        mem[i + 1] = xx[i];
    }
}

/// Outputs are held within ±32767 (0x1000c290 and the end of nb_decode).
fn clamp(v: f32) -> f32 {
    if v as f64 > 32767.0 {
        32767.0
    } else if (v as f64) < -32767.0 {
        -32767.0
    } else {
        v
    }
}

// --- narrowband, submode 7 (nb_decode 0x10009520) -----------------------------

const NB_ORDER: usize = 10;
const NB_FRAME: usize = 160;
const NB_SUB: usize = 40;
const NB_PITCH_START: usize = 17;
const NB_PITCH_END: usize = 144;
/// `st->exc` inside `excBuf`: after max_pitch + 1 samples of history.
const EXC: usize = NB_PITCH_END + 1;
const EXC_GAIN_QUANT_SCAL3: [f32; 8] = [0.061130, 0.163546, 0.310413, 0.428220, 0.555887, 0.719055, 0.938694, 1.326874];

struct Narrowband {
    exc_buf: [f32; EXC + NB_FRAME],
    innov: [f32; NB_FRAME],
    interp_qlpc: [f32; NB_ORDER],
    mem_sp: [f32; NB_ORDER],
    old_qlsp: [f32; NB_ORDER],
    pi_gain: [f32; 4],
    first: bool,
}

impl Narrowband {
    fn new() -> Self {
        Narrowband {
            exc_buf: [0.0; EXC + NB_FRAME],
            innov: [0.0; NB_FRAME],
            interp_qlpc: [0.0; NB_ORDER],
            mem_sp: [0.0; NB_ORDER],
            old_qlsp: [0.0; NB_ORDER],
            pi_gain: [0.0; 4],
            first: true,
        }
    }

    /// lsp_unquant_nb (0x1000b510).
    fn lsp_unquant(bits: &mut Bits) -> [f32; NB_ORDER] {
        let mut lsp: [f32; NB_ORDER] = std::array::from_fn(|i| f(i as f64 * 0.25 + 0.25));
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

    /// pitch_unquant_3tap (0x10006bb0) into the subframe at `base`.
    fn pitch_unquant(&mut self, bits: &mut Bits, base: usize) {
        let pitch = NB_PITCH_START + bits.get(7) as usize;
        let gi = bits.get(7) as usize;
        // gain_cdbk_nb holds 3 values per entry in Speex 1.1 (the first 3 of our 4)
        let g: [f64; 3] = std::array::from_fn(|k| f(GAIN_CDBK_NB[gi * 4 + k] as f64 * 0.015625 + 0.5) as f64);
        let mut e = [[0f64; NB_SUB]; 3];
        for (i, ei) in e.iter_mut().enumerate() {
            let pp = pitch + 1 - i;
            for (j, v) in ei.iter_mut().enumerate() {
                *v = if j < pp {
                    self.exc_buf[base + j - pp] as f64
                } else if j < pp + pitch {
                    self.exc_buf[base + j - pp - pitch] as f64
                } else {
                    0.0
                };
            }
        }
        for i in 0..NB_SUB {
            self.exc_buf[base + i] = f(1e-15 + g[0] * e[2][i] + g[1] * e[1][i] + g[2] * e[0][i]);
        }
    }

    /// Decode the low 0-4 kHz into `out`.
    fn decode(&mut self, bits: &mut Bits, out: &mut [f32]) {
        bits.get(1 + 4); // narrowband marker and submode (7, checked by the caller)
        self.exc_buf.copy_within(NB_FRAME..NB_FRAME + EXC, 0);
        let qlsp = Self::lsp_unquant(bits);
        if self.first {
            self.old_qlsp = qlsp;
        }
        let ol_gain = f((bits.get(5) as f64 / 3.5).exp()) as f64;
        let mut interp = [0f32; NB_ORDER];
        for sub in 0..4 {
            let offset = NB_SUB * sub;
            let base = EXC + offset;
            lsp_interpolate(&self.old_qlsp, &qlsp, &mut interp, sub, 4, 0.002);
            lsp_to_lpc(&interp, &mut self.interp_qlpc);
            let mut pi_g = 1f32;
            for i in (0..NB_ORDER).step_by(2) {
                pi_g = f(self.interp_qlpc[i + 1] as f64 - self.interp_qlpc[i] as f64 + pi_g as f64);
            }
            self.pi_gain[sub] = pi_g;
            self.exc_buf[base..base + NB_SUB].fill(0.0);
            self.pitch_unquant(bits, base);
            // innovation: the first codebook is what the band above folds
            let ener = f(EXC_GAIN_QUANT_SCAL3[bits.get(3) as usize] as f64 * ol_gain);
            let innov = &mut self.innov[offset..offset + NB_SUB];
            innov.fill(0.0);
            split_cb_unquant(innov, bits, 5, 8, 6, false, &EXC_5_64_TABLE, 0.03125);
            signal_mul(innov, ener);
            for i in 0..NB_SUB {
                self.exc_buf[base + i] = f(innov[i] as f64 + self.exc_buf[base + i] as f64);
            }
            let mut innov2 = [0f32; NB_SUB];
            split_cb_unquant(&mut innov2, bits, 5, 8, 6, false, &EXC_5_64_TABLE, 0.03125);
            signal_mul(&mut innov2, f(ener as f64 * 0.45454545454545453));
            for i in 0..NB_SUB {
                self.exc_buf[base + i] = f(innov2[i] as f64 + self.exc_buf[base + i] as f64);
            }
            let sp = &mut out[offset..offset + NB_SUB];
            sp.copy_from_slice(&self.exc_buf[base..base + NB_SUB]);
            iir_mem2(sp, &self.interp_qlpc, &mut self.mem_sp);
        }
        out.iter_mut().for_each(|v| *v = clamp(*v));
        self.old_qlsp = qlsp;
        self.first = false;
    }

    /// SPEEX_GET_EXC: this frame's excitation.
    fn exc(&self) -> &[f32] {
        &self.exc_buf[EXC..EXC + NB_FRAME]
    }
}

// --- the sub-band layers (sb_decode 0x1000e170) -------------------------------

/// One band above another: 8 LPCs, 4 subframes of `SUB` samples.
struct HighBand<const SUB: usize> {
    exc: Vec<f32>,
    high: Vec<f32>,
    old_qlsp: [f32; 8],
    interp_qlpc: [f32; 8],
    mem_sp: [f32; 8],
    pi_gain: [f32; 4],
    g0_mem: [f32; 64],
    g1_mem: [f32; 64],
    h0: [f32; 64],
    h1: [f32; 64],
    first: bool,
}

impl<const SUB: usize> HighBand<SUB> {
    fn new() -> Self {
        HighBand {
            exc: vec![0.0; SUB * 4],
            high: vec![0.0; SUB * 4],
            old_qlsp: [0.0; 8],
            interp_qlpc: [0.0; 8],
            mem_sp: [0.0; 8],
            pi_gain: [0.0; 4],
            g0_mem: [0.0; 64],
            g1_mem: [0.0; 64],
            h0: qmf(1.0),
            h1: qmf(-1.0),
            first: true,
        }
    }

    /// Decode the band above `low` (the lower layer's output) into `out`
    /// (twice as long). `exc_of` gives a subframe's excitation from the bits,
    /// the subframe and the filter ratio.
    fn decode(&mut self, bits: &mut Bits, low: &[f32], low_pi_gain: &[f32; 4], out: &mut [f32], mut exc_of: impl FnMut(&mut Bits, usize, f32, &mut [f32])) {
        bits.get(1 + 3); // wideband marker and submode (checked by the caller)
        self.exc.fill(0.0);
        let qlsp = lsp_unquant_high(bits);
        if self.first {
            self.old_qlsp = qlsp;
        }
        let mut interp = [0f32; 8];
        for sub in 0..4 {
            let offset = SUB * sub;
            lsp_interpolate(&self.old_qlsp, &qlsp, &mut interp, sub, 4, 0.05);
            lsp_to_lpc(&interp, &mut self.interp_qlpc);
            let ak = &self.interp_qlpc;
            self.pi_gain[sub] = 1.0;
            let mut rh = 1f32;
            for i in (0..8).step_by(2) {
                rh = f(ak[i + 1] as f64 - ak[i] as f64 + rh as f64);
                self.pi_gain[sub] = f(ak[i] as f64 + ak[i + 1] as f64 + self.pi_gain[sub] as f64);
            }
            let filter_ratio = f((low_pi_gain[sub] as f64 + 0.01) / (0.01 + rh as f64));
            let exc = &mut self.exc[offset..offset + SUB];
            exc_of(bits, sub, filter_ratio, exc);
            let sp = &mut self.high[offset..offset + SUB];
            sp.copy_from_slice(exc);
            iir_mem2(sp, &self.interp_qlpc, &mut self.mem_sp);
        }
        let n = SUB * 8;
        let mut y0 = vec![0f32; n];
        let mut y1 = vec![0f32; n];
        fir_mem_up(low, &self.h0, &mut y0, &mut self.g0_mem);
        fir_mem_up(&self.high, &self.h1, &mut y1, &mut self.g1_mem);
        for i in 0..n {
            out[i] = clamp(f((y0[i] as f64 - y1[i] as f64) * 2.0));
        }
        self.old_qlsp = qlsp;
        self.first = false;
    }

    /// SPEEX_GET_INNOV of a sub-band decoder (0x1000f0e0 case 0x66): twice
    /// the excitation on even samples, zero between.
    fn innov(&self) -> Vec<f32> {
        let mut e = vec![0f32; SUB * 8];
        for (i, &v) in self.exc.iter().enumerate() {
            e[2 * i] = f(v as f64 * 2.0);
        }
        e
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
        let mut nb_out = [0f32; 160];
        self.nb.decode(&mut bits, &mut nb_out);
        // wideband: innovation from the double codebook, scaled by the low
        // band's excitation energy (+1) over the filter ratio
        let nb_exc = self.nb.exc().to_vec();
        let nb_pi_gain = self.nb.pi_gain;
        let mut wb_out = [0f32; 320];
        self.wb.decode(&mut bits, &nb_out, &nb_pi_gain, &mut wb_out, |bits, sub, filter_ratio, exc| {
            let qgc = bits.get(4) as f64;
            let el = compute_rms(&nb_exc[sub * 40..sub * 40 + 40]) as f64;
            let gc = f((qgc * 0.27027027027027023 - 0.15556).exp()) as f64;
            let scale = f((el + 1.0) * (gc / filter_ratio as f64));
            split_cb_unquant(exc, bits, 8, 5, 7, true, &HEXC_TABLE, 0.03125);
            signal_mul(exc, scale);
            let mut innov2 = [0f32; 40];
            split_cb_unquant(&mut innov2, bits, 8, 5, 7, true, &HEXC_TABLE, 0.03125);
            let scale2 = scale as f64 * 0.4;
            for i in 0..40 {
                innov2[i] = f(innov2[i] as f64 * scale2);
                exc[i] = f(innov2[i] as f64 + exc[i] as f64);
            }
        });
        // ultra-wideband: no innovation, the wideband one folded up with
        // alternating signs
        let wb_innov = self.wb.innov();
        let wb_pi_gain = self.wb.pi_gain;
        let mut out = [0f32; FRAME_SAMPLES];
        self.uwb.decode(&mut bits, &wb_out, &wb_pi_gain, &mut out, |bits, sub, filter_ratio, exc| {
            let quant = bits.get(5) as f64;
            let g = f(f(((quant - 10.0) * 0.125).exp()) as f64 / filter_ratio as f64) as f64;
            let mut s = 1f32;
            for i in 0..80 {
                exc[i] = f(s as f64 * g * 0.7f32 as f64 * wb_innov[sub * 80 + i] as f64);
                s = f(s as f64 * -1.0);
            }
        });
        for (p, &v) in pcm.iter_mut().zip(out.iter()) {
            *p = to_i16(v);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    /// Frames of pseudo-random bits with the voices' layout headers.
    pub(crate) fn synthetic_frames(n: usize) -> Vec<Vec<u8>> {
        let mut seed: u32 = 12345;
        let set = |f: &mut [u8], pos: usize, bits: &[u8]| {
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

    /// The decoder's output for these frames, pinned once it matched the
    /// original engine's audio (MECHANICS.md §2).
    #[test]
    fn decodes_like_ltts7codec210() {
        let mut d = super::Decoder::default();
        let mut all = Vec::new();
        let mut pcm = [0i16; super::FRAME_SAMPLES];
        for f in synthetic_frames(200) {
            assert!(super::Decoder::supports(&f));
            d.decode(&f, &mut pcm);
            all.extend_from_slice(&pcm);
        }
        assert_eq!(fnv(&all), PINNED_HASH);
    }

    const PINNED_HASH: u64 = 0x46ff_f256_fa44_4381;
}
