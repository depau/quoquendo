//! LTTS7CodecMsx: the 44 kHz voices' codec (`<Voice>.44000.LTTS7CodecMsx…bin`),
//! read from LTTS7CodecMsx.dll (MECHANICS.md §2b). It is Speex ultra-wideband
//! again, but a later Speex (1.2's decoder loop: the excitation is played one
//! subframe late through the previous subframe's filter, and the pitch
//! contribution is summed tap by tap) with Loquendo's own changes:
//!
//! - frames carry no mode or submode bits: each layer's submode follows from
//!   the frame size (106 bytes: narrowband 8, wideband 5, ultra-wideband 2);
//! - every codebook but the pitch gains is trained per voice and stored in
//!   the file, in sets ("classes"); one byte per frame picks the set;
//! - narrowband LSPs take 7 + 4 + 4 bits, wideband 5 + 3, ultra-wideband 6 + 6.
//!
//! Frame: narrowband 472 bits, wideband 344, ultra-wideband 32 (848 = 106 bytes).

use super::*;

/// Samples per decoded frame (at the file's 44 kHz).
pub const FRAME_SAMPLES: usize = 640;
/// Bytes per frame in every CodecMsx file.
pub const FRAME_BYTES: usize = 106;

/// One codebook in the file: a table per class of `1 << bits` entries of `dim`.
#[derive(Clone, Debug, Default)]
struct Book {
    cb: Vec<i8>,
    dim: usize,
    bits: usize,
}

impl Book {
    fn entries(&self, class: u8) -> &[i8] {
        let n = self.dim << self.bits;
        &self.cb[class as usize * n..class as usize * n + n]
    }
}

/// The trained codebooks a CodecMsx file carries after its header
/// (ELQDecoderFileChanged 0x10001d40 lays them out).
#[derive(Clone, Debug, Default)]
pub struct Codebooks {
    /// per frame: which set of codebooks it uses
    classes: Vec<u8>,
    nb_lsp: [Book; 3],
    wb_lsp: [Book; 2],
    uwb_lsp: [Book; 2],
    nb_exc: Book,
    hexc: Book,
    nb_exc_scale: f64,
    hexc_scale: f64,
}

impl Codebooks {
    /// Read them from the file's bytes: `data[hdr..]` is the big-endian
    /// header after the text header; frames start `data_offset` after it.
    pub fn parse(data: &[u8], hdr: usize, frames: usize) -> Result<Codebooks, String> {
        let w = |i: usize| -> Result<usize, String> {
            data.get(hdr + 4 * i..hdr + 4 * i + 4)
                .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize)
                .ok_or_else(|| "truncated CodecMsx header".to_string())
        };
        if w(7)? != 2 {
            return Err(format!("CodecMsx header version {}: only 2 is known", w(7)?));
        }
        let n_classes = w(9)?;
        // per-class table sizes in bytes, in file order, and each one's
        // entry size (header words 10..=18 and 21..=29)
        let bytes = [w(10)?, w(11)?, w(12)?, w(13)?, w(14)?, w(15)?, w(16)?, w(17)?, w(18)?];
        let dims = [w(21)?, w(22)?, w(23)?, w(28)?, w(24)?, w(25)?, w(26)?, w(27)?, w(29)?];
        // the one layout every CodecMsx voice has (a 10th-order narrowband
        // filter, 8th-order bands above, the submodes 106-byte frames pick)
        if dims != [10, 5, 5, 5, 8, 8, 8, 8, 8] || bytes[3] != 5 << 6 || bytes[8] != 8 << 7 || w(19)? == 0 || w(20)? == 0 {
            return Err("CodecMsx codebooks of a layout no Loquendo 7 voice uses".into());
        }
        let mut at = hdr + 0x7c;
        let classes = data.get(at..at + frames).ok_or("truncated CodecMsx frame classes")?.to_vec();
        at += frames;
        let mut books: Vec<Book> = Vec::new();
        for (&b, &dim) in bytes.iter().zip(&dims) {
            let size = b * n_classes;
            let cb = data.get(at..at + size).ok_or("truncated CodecMsx codebooks")?.iter().map(|&x| x as i8).collect();
            at += size;
            let entries = b / dim;
            if b % dim != 0 || !entries.is_power_of_two() {
                return Err("CodecMsx codebook sizes are not whole tables".into());
            }
            books.push(Book { cb, dim, bits: entries.trailing_zeros() as usize });
        }
        if at != hdr + w(8)? {
            return Err("CodecMsx codebooks do not end where the frames start".into());
        }
        if classes.iter().any(|&c| c as usize >= n_classes) {
            return Err("CodecMsx frame uses a codebook set the file lacks".into());
        }
        let mut it = books.into_iter();
        let mut next = || it.next().unwrap();
        Ok(Codebooks {
            classes,
            nb_lsp: [next(), next(), next()],
            nb_exc: next(),
            wb_lsp: [next(), next()],
            uwb_lsp: [next(), next()],
            hexc: next(),
            nb_exc_scale: 1.0 / w(19)? as f64,
            hexc_scale: 1.0 / w(20)? as f64,
        })
    }
}

/// LSPs from the class's codebooks: the first stage spans every LSP, later
/// ones the next `dim` LSPs each (0x100046d0, 0x100048a0, 0x10004a10).
fn lsp_unquant<const N: usize>(bits: &mut Bits, books: &[Book], class: u8, init: impl Fn(usize) -> f32) -> [f32; N] {
    let mut lsp: [f32; N] = std::array::from_fn(init);
    let mut from = 0;
    for (k, b) in books.iter().enumerate() {
        let id = bits.get(b.bits) as usize;
        let e = &b.entries(class)[id * b.dim..id * b.dim + b.dim];
        let step = if k == 0 { LSP_DIV_256 } else { LSP_DIV_512 };
        for (i, &c) in e.iter().enumerate() {
            add_lsp(&mut lsp[from + i], step, c);
        }
        if k > 0 {
            from += b.dim;
        }
    }
    lsp
}

// --- narrowband, submode 8 (nb_decode 0x10003800) ------------------------------

const NB_ORDER: usize = 10;
const NB_FRAME: usize = 160;
const NB_SUB: usize = 40;
const NB_PITCH_START: usize = 17;
const NB_PITCH_END: usize = 144;
/// `st->exc` inside `excBuf`: 2·max_pitch + subframe + 6 samples in.
const EXC: usize = 2 * NB_PITCH_END + NB_SUB + 6;
const EXC_BUF: usize = EXC + NB_FRAME;
/// exc_gain_quant_scal3 (0x1001627c), as Codec210's.
const EXC_GAIN_QUANT_SCAL3: [f32; 8] = [0.061130, 0.163546, 0.310413, 0.428220, 0.555887, 0.719055, 0.938694, 1.326874];

struct Narrowband {
    exc_buf: [f32; EXC_BUF],
    /// the previous subframe's filter
    interp_qlpc: [f32; NB_ORDER],
    mem_sp: [f32; NB_ORDER],
    old_qlsp: [f32; NB_ORDER],
    pi_gain: [f32; 4],
    first: bool,
}

impl Narrowband {
    fn new() -> Self {
        Narrowband {
            exc_buf: [0.0; EXC_BUF],
            interp_qlpc: [0.0; NB_ORDER],
            mem_sp: [0.0; NB_ORDER],
            old_qlsp: [0.0; NB_ORDER],
            pi_gain: [0.0; 4],
            first: true,
        }
    }

    /// pitch_unquant_3tap (0x10003120), Speex 1.2's: each tap is added on
    /// its own into a zeroed buffer, from the excitation before `base`.
    fn pitch_unquant(&self, bits: &mut Bits, base: usize) -> [f32; NB_SUB] {
        let pitch = NB_PITCH_START + bits.get(7) as usize;
        let gi = bits.get(7) as usize;
        let g: [f32; 3] = std::array::from_fn(|k| f(GAIN_CDBK_NB[gi * 4 + k] as f64 * 0.015625 + 0.5));
        let mut out = [0f32; NB_SUB];
        for i in 0..3 {
            let pp = pitch + 1 - i;
            let gain = g[2 - i] as f64;
            let tmp1 = NB_SUB.min(pp);
            for (j, o) in out.iter_mut().enumerate().take(tmp1) {
                *o = f(self.exc_buf[base + j - pp] as f64 * gain + *o as f64);
            }
            let tmp3 = NB_SUB.min(pp + pitch);
            for (j, o) in out.iter_mut().enumerate().take(tmp3).skip(tmp1) {
                *o = f(self.exc_buf[base + j - pp - pitch] as f64 * gain + *o as f64);
            }
        }
        out
    }

    /// Decode the low 0-4 kHz of frame `class` into `out`.
    fn decode(&mut self, bits: &mut Bits, books: &Codebooks, class: u8, out: &mut [f32]) {
        self.exc_buf.copy_within(NB_FRAME.., 0);
        let qlsp: [f32; NB_ORDER] = lsp_unquant(bits, &books.nb_lsp, class, |i| f(i as f64 * 0.25 + 0.25));
        if self.first {
            self.old_qlsp = qlsp;
        }
        let ol_gain = f(x87::exp(bits.get(5) as f64 / 3.5)) as f64;
        for sub in 0..4 {
            let base = EXC + NB_SUB * sub;
            self.exc_buf[base..base + NB_SUB].fill(0.0);
            let exc32 = self.pitch_unquant(bits, base);
            let ener = f(EXC_GAIN_QUANT_SCAL3[bits.get(3) as usize] as f64 * ol_gain);
            let mut innov = [0f32; NB_SUB];
            split_cb_unquant(&mut innov, bits, 5, 8, 6, false, books.nb_exc.entries(class), books.nb_exc_scale);
            signal_mul(&mut innov, ener);
            for i in 0..NB_SUB {
                self.exc_buf[base + i] = f(exc32[i] as f64 + innov[i] as f64);
            }
            let mut innov2 = [0f32; NB_SUB];
            split_cb_unquant(&mut innov2, bits, 5, 8, 6, false, books.nb_exc.entries(class), books.nb_exc_scale);
            signal_mul(&mut innov2, f(ener as f64 * 0.454545));
            for i in 0..NB_SUB {
                self.exc_buf[base + i] = f(innov2[i] as f64 + self.exc_buf[base + i] as f64);
            }
        }
        // the excitation one subframe late, each subframe through the
        // previous subframe's filter
        out.copy_from_slice(&self.exc_buf[EXC - NB_SUB..EXC - NB_SUB + NB_FRAME]);
        let mut interp = [0f32; NB_ORDER];
        let mut ak = [0f32; NB_ORDER];
        for sub in 0..4 {
            lsp_interpolate(&self.old_qlsp, &qlsp, &mut interp, sub, 4, f(0.002));
            lsp_to_lpc(&interp, &mut ak);
            let mut pi_g = 1f32;
            for i in (0..NB_ORDER).step_by(2) {
                pi_g = f(self.interp_qlpc[i + 1] as f64 - self.interp_qlpc[i] as f64 + pi_g as f64);
            }
            self.pi_gain[sub] = pi_g;
            iir_mem2(&mut out[NB_SUB * sub..NB_SUB * (sub + 1)], &self.interp_qlpc, &mut self.mem_sp);
            self.interp_qlpc = ak;
        }
        self.old_qlsp = qlsp;
        self.first = false;
    }

    /// SPEEX_GET_EXC (ctl 0x10003ff0 case 0x65): this frame's excitation.
    fn exc(&self) -> &[f32] {
        &self.exc_buf[EXC..EXC + NB_FRAME]
    }
}

// --- the sub-band layers (sb_decode 0x10005080) --------------------------------

/// One band above another: 8 LPCs, 4 subframes of `SUB` samples.
struct HighBand<const SUB: usize> {
    /// the previous subframe's excitation and filter
    exc_prev: Vec<f32>,
    interp_qlpc: [f32; 8],
    high: Vec<f32>,
    old_qlsp: [f32; 8],
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
            exc_prev: vec![0.0; SUB],
            interp_qlpc: [0.0; 8],
            high: vec![0.0; SUB * 4],
            old_qlsp: [0.0; 8],
            mem_sp: [0.0; 8],
            pi_gain: [0.0; 4],
            g0_mem: [0.0; 64],
            g1_mem: [0.0; 64],
            h0: qmf(1.0),
            h1: qmf(-1.0),
            first: true,
        }
    }

    /// Decode the band above `low` into `out` (twice as long). `exc_of`
    /// fills a subframe's excitation from the bits, the subframe and the
    /// filter ratio.
    fn decode(&mut self, bits: &mut Bits, lsp: &[Book], class: u8, low: &[f32], low_pi_gain: &[f32; 4], out: &mut [f32], mut exc_of: impl FnMut(&mut Bits, usize, f32, &mut [f32])) {
        let qlsp: [f32; 8] = lsp_unquant(bits, lsp, class, |i| f(i as f64 * 0.3125 + 0.75));
        if self.first {
            self.old_qlsp = qlsp;
        }
        let mut interp = [0f32; 8];
        let mut ak = [0f32; 8];
        for sub in 0..4 {
            lsp_interpolate(&self.old_qlsp, &qlsp, &mut interp, sub, 4, f(0.05));
            lsp_to_lpc(&interp, &mut ak);
            let a = &self.interp_qlpc;
            self.pi_gain[sub] = 1.0;
            let mut rh = 1f32;
            for i in (0..8).step_by(2) {
                rh = f(a[i + 1] as f64 - a[i] as f64 + rh as f64);
                self.pi_gain[sub] = f(a[i] as f64 + a[i + 1] as f64 + self.pi_gain[sub] as f64);
            }
            let filter_ratio = f((low_pi_gain[sub] as f64 + 0.01) / (0.01 + rh as f64));
            let mut exc = vec![0f32; SUB];
            exc_of(bits, sub, filter_ratio, &mut exc);
            let sp = &mut self.high[SUB * sub..SUB * (sub + 1)];
            sp.copy_from_slice(&self.exc_prev);
            iir_mem2(sp, &self.interp_qlpc, &mut self.mem_sp);
            self.exc_prev = exc;
            self.interp_qlpc = ak;
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
}

/// A CodecMsx decoder (ELQOpenDecoder 0x10002820: ultra-wideband,
/// enhancement off).
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
    /// Decode frame number `k` of the file into 640 samples.
    pub fn decode(&mut self, books: &Codebooks, k: usize, frame: &[u8], pcm: &mut [i16]) {
        let class = books.classes[k];
        let mut bits = Bits { data: frame, pos: 0 };
        let mut nb_out = [0f32; 160];
        self.nb.decode(&mut bits, books, class, &mut nb_out);
        // wideband: the double codebook, scaled by the low band's excitation
        // energy (+1) over the filter ratio; what it adds is handed up
        let nb_exc = self.nb.exc().to_vec();
        let nb_pi_gain = self.nb.pi_gain;
        let mut wb_innov = [0f32; 320];
        let mut wb_out = [0f32; 320];
        self.wb.decode(&mut bits, &books.wb_lsp, class, &nb_out, &nb_pi_gain, &mut wb_out, |bits, sub, filter_ratio, exc| {
            let qgc = bits.get(4) as f64;
            let el = compute_rms(&nb_exc[sub * 40..sub * 40 + 40]) as f64;
            let gc = f(x87::exp(qgc * 0.27027027027027023 - 0.15556)) as f64;
            let scale = f((el + 1.0) * (gc / filter_ratio as f64));
            split_cb_unquant(exc, bits, 8, 5, 7, true, books.hexc.entries(class), books.hexc_scale);
            signal_mul(exc, scale);
            let mut innov2 = [0f32; 40];
            split_cb_unquant(&mut innov2, bits, 8, 5, 7, true, books.hexc.entries(class), books.hexc_scale);
            let scale2 = scale as f64 * 0.4;
            for i in 0..40 {
                innov2[i] = f(innov2[i] as f64 * scale2);
                exc[i] = f(innov2[i] as f64 + exc[i] as f64);
            }
            // innovation saved for the band above: even samples, no factor 2
            for i in 0..40 {
                wb_innov[sub * 80 + 2 * i] = exc[i];
            }
        });
        // ultra-wideband: no innovation, the wideband one folded up with
        // alternating signs
        let wb_pi_gain = self.wb.pi_gain;
        let mut out = [0f32; FRAME_SAMPLES];
        self.uwb.decode(&mut bits, &books.uwb_lsp, class, &wb_out, &wb_pi_gain, &mut out, |bits, sub, filter_ratio, exc| {
            let quant = bits.get(5) as f64;
            let g = f(f(x87::exp((quant - 10.0) * 0.125)) as f64 / filter_ratio as f64) as f64;
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
