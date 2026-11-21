//! The voice recordings: `<voice>.32000.LTTS7Codec210.44000.10.bin` and
//! `<voice>.44000.LTTS7CodecMsx.58300.10.bin`.
//!
//! Both codecs are Speex ultra-wideband (the strings and decoder set-up in
//! their DLLs are libspeex's), [`speex`] decodes them. Layouts and decoder
//! settings are in MECHANICS.md §2 and §2b, each with the address it was read
//! from.

use std::path::Path;

use crate::speex::{self, msx, Variant};

pub struct AudioBank {
    data: Vec<u8>,
    first_frame: usize,
    pub frame_bytes: usize,
    pub frame_samples: usize,
    pub frames: usize,
    pub rate: u32,
    pub bitrate: u32,
    pub variant: Variant,
    key: u8,
    /// CodecMsx: the file's own codebooks
    books: Option<msx::Codebooks>,
    /// the decoder and the frames it keeps between requests
    stream: std::sync::Mutex<Stream>,
}

enum FrameDecoder {
    Codec210(Box<speex::Decoder>),
    Msx(Box<msx::Decoder>),
}

/// What `ELQDecode` keeps between requests (LTTS7Codec210 0x10004340,
/// LTTS7CodecMsx 0x100024d0): one decoder, the frame it decoded last and
/// the one before. Frame numbers start at 0xfffffffe, "none"
/// (ELQOpenDecoder 0x100046d3, 0x10002a22).
struct Stream {
    decoder: Option<FrameDecoder>,
    cur: u32,
    prev: u32,
    cur_buf: Vec<i16>,
    prev_buf: Vec<i16>,
}

impl Stream {
    fn new(frame_samples: usize) -> Stream {
        Stream {
            decoder: None,
            cur: 0xffff_fffe,
            prev: 0xffff_fffe,
            cur_buf: vec![0; frame_samples],
            prev_buf: vec![0; frame_samples],
        }
    }
}

impl AudioBank {
    pub fn open(path: &Path) -> Result<AudioBank, String> {
        let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        AudioBank::parse(data).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The recordings from a `.bin` file's bytes.
    pub fn parse(data: Vec<u8>) -> Result<AudioBank, String> {
        if data.len() < 64 {
            return Err("too short".into());
        }
        // text header: XOR with byte0^byte1, then "LOQUENDOTTS7", field 1
        // (id u16le, value u16le) = the header length, then (id u16le, len
        // u16le, text) fields; field 5 names the codec
        let key = data[0] ^ data[1];
        let x = |i: usize| data[i] ^ key;
        let magic: Vec<u8> = (2..14).map(x).collect();
        if magic != b"LOQUENDOTTS7" {
            return Err("not a LOQUENDOTTS7 audio file".into());
        }
        let id = u16::from_le_bytes([x(14), x(15)]);
        if id != 1 {
            return Err("header length field missing".into());
        }
        let hdr = u16::from_le_bytes([x(16), x(17)]) as usize;
        if hdr > data.len() {
            return Err("truncated header".into());
        }
        let mut codec = Vec::new();
        let mut at = 18;
        while at + 4 <= hdr {
            let (id, len) = (u16::from_le_bytes([x(at), x(at + 1)]), u16::from_le_bytes([x(at + 2), x(at + 3)]) as usize);
            if id == 5 {
                codec = (at + 4..(at + 4 + len).min(hdr)).map(x).take_while(|&c| c != 0).collect();
            }
            at += 4 + len;
        }
        let variant = match codec.as_slice() {
            b"LTTS7Codec210" => Variant::Codec210,
            b"LTTS7CodecMsx" => Variant::Msx,
            other => return Err(format!("codec {} not supported", String::from_utf8_lossy(other))),
        };
        let be = |o: usize| -> Result<u32, String> {
            data.get(hdr + o..hdr + o + 4)
                .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
                .ok_or_else(|| "truncated header".to_string())
        };
        // Codec210: frames right after the 28 header bytes; CodecMsx: the
        // codebooks follow, frames start where header word 8 says
        let first_frame = match variant {
            Variant::Codec210 => hdr + 28,
            Variant::Msx => hdr + be(32)? as usize,
        };
        let mut bank = AudioBank {
            bitrate: be(0)?,
            frame_samples: be(8)? as usize,
            rate: be(12)?,
            frame_bytes: be(16)? as usize,
            frames: be(20)? as usize,
            key: (be(24)? & 0xff) as u8,
            first_frame,
            variant,
            books: None,
            data,
            stream: std::sync::Mutex::new(Stream::new(0)),
        };
        match variant {
            Variant::Codec210 if bank.rate != 32000 || bank.frame_samples != speex::FRAME_SAMPLES => {
                return Err(format!("{} Hz / {}-sample frames: not a layout LTTS7Codec210 voices use", bank.rate, bank.frame_samples));
            }
            Variant::Msx if bank.rate != 44000 || bank.frame_samples != msx::FRAME_SAMPLES || bank.frame_bytes != msx::FRAME_BYTES => {
                return Err(format!("{} Hz / {}-sample frames: not a layout LTTS7CodecMsx voices use", bank.rate, bank.frame_samples));
            }
            Variant::Msx => bank.books = Some(msx::Codebooks::parse(&bank.data, hdr, bank.frames)?),
            Variant::Codec210 => {}
        }
        if bank.first_frame + bank.frames * bank.frame_bytes > bank.data.len() {
            return Err("fewer frames in the file than the header says".into());
        }
        if variant == Variant::Codec210 && bank.frames > 0 {
            let first: Vec<u8> = bank.data[bank.first_frame..bank.first_frame + bank.frame_bytes].iter().map(|b| b ^ bank.key).collect();
            if !speex::Decoder::supports(&first) {
                return Err("Speex frames of a layout no Loquendo 7 voice uses".into());
            }
        }
        bank.stream = std::sync::Mutex::new(Stream::new(bank.frame_samples));
        Ok(bank)
    }

    /// Recordings of nothing at `rate`: every read is silence (for tests of
    /// what is built around the audio).
    #[cfg(test)]
    pub(crate) fn silent(rate: u32) -> AudioBank {
        AudioBank {
            data: Vec::new(),
            first_frame: 0,
            frame_bytes: 0,
            frame_samples: speex::FRAME_SAMPLES,
            frames: 0,
            rate,
            bitrate: 0,
            variant: Variant::Codec210,
            key: 0,
            books: None,
            stream: std::sync::Mutex::new(Stream::new(speex::FRAME_SAMPLES)),
        }
    }

    pub fn total_samples(&self) -> usize {
        self.frames * self.frame_samples
    }

    /// Output lags the original by this many samples, and this many frames
    /// are decoded before a seek target to settle the decoder: 509 and 6
    /// (LTTS7Codec210 ELQOpenDecoder 0x100045e0); 510 and 9 at 44 kHz
    /// (LTTS7CodecMsx ELQOpenDecoder 0x10002820).
    fn delay_and_warmup(&self) -> (u32, u32) {
        match self.variant {
            Variant::Codec210 => (509, 6),
            Variant::Msx => (510, 9),
        }
    }

    /// `len` samples of the original recording starting at sample `start`,
    /// decoded as `ELQDecode` does (LTTS7Codec210 0x10004530 → 0x10004340,
    /// LTTS7CodecMsx 0x10002780 → 0x100024d0): a request served from the
    /// frame decoded last or the one before is copied, the frame after the
    /// last is decoded by the same decoder, and anything else starts a fresh
    /// decoder that many warm-up frames earlier. The result therefore
    /// depends on the requests before it, as the original's does.
    pub fn read(&self, start: usize, len: usize) -> Vec<i16> {
        let (delay, warmup) = self.delay_and_warmup();
        let fs = self.frame_samples as u32;
        let mut st = self.stream.lock().unwrap_or_else(|e| e.into_inner());
        let mut out = Vec::with_capacity(len);
        let mut pos = (start as u32).wrapping_add(delay);
        while out.len() < len {
            let mut f = pos / fs;
            // CodecMsx: past the last frame the position loses the delay
            // again (0x100024f1)
            if self.variant == Variant::Msx && f >= self.frames as u32 {
                pos = pos.wrapping_sub(delay);
                f = pos / fs;
            }
            if f == st.cur || f == st.prev {
                let buf = if f == st.cur { &st.cur_buf } else { &st.prev_buf };
                let off = (pos - f * fs) as usize;
                let n = (len - out.len()).min(fs as usize - off);
                out.extend_from_slice(&buf[off..off + n]);
                pos = pos.wrapping_add(n as u32);
            } else if f == st.cur.wrapping_add(1) {
                self.next_frame(&mut st, f);
            } else {
                // the reset (0x10004230, 0x10002390): a new decoder
                st.decoder = Some(match &self.books {
                    Some(_) => FrameDecoder::Msx(Box::default()),
                    None => FrameDecoder::Codec210(Box::default()),
                });
                for k in f.saturating_sub(warmup)..=f {
                    self.next_frame(&mut st, k);
                }
            }
        }
        out
    }

    /// The frame after the current one: the current becomes the previous
    /// (0x100042f0, 0x10002480), and frame `k` is decoded into the current
    /// buffer, or silence past the end of the file.
    fn next_frame(&self, st: &mut Stream, k: u32) {
        let st = &mut *st;
        std::mem::swap(&mut st.prev_buf, &mut st.cur_buf);
        st.prev = st.cur;
        st.cur = k;
        let k = k as usize;
        if k >= self.frames {
            st.cur_buf.fill(0);
            return;
        }
        let at = self.first_frame + k * self.frame_bytes;
        let frame: Vec<u8> = self.data[at..at + self.frame_bytes].iter().map(|b| b ^ self.key).collect();
        match (st.decoder.as_mut(), &self.books) {
            (Some(FrameDecoder::Msx(d)), Some(books)) => d.decode(books, k, &frame, &mut st.cur_buf),
            (Some(FrameDecoder::Codec210(d)), None) => d.decode(&frame, &mut st.cur_buf),
            _ => unreachable!("the decoder is made with the bank's variant before any frame"),
        }
    }
}

/// The rate the original engine's audio comes out at here: the one the
/// reference harness asks for (oracle/loqref.c `ttsSetAudio(…, 32000, …)`).
pub const OUTPUT_RATE: u32 = 32_000;

/// The filter LoqTTS7.dll's resampler uses ("large filter", tables at
/// 0x1007ad88 and 0x1007ed88): 8192 taps of one wing, 256 per input
/// sample, of a 0.90-band lowpass under a Kaiser window, β = 9, scaled so the
/// largest is 32767 and rounded as `(int)(x + 0.5)`. Generated here
/// (Julius O. Smith's `makeFilter` design); a test pins it to the DLL's.
const NWING: usize = 8192;
const NPC: f64 = 256.0;

static LARGE_FILTER: std::sync::LazyLock<(Vec<i32>, Vec<i32>)> = std::sync::LazyLock::new(|| {
    // the modified Bessel function I0 as a series, to 1e-21 relative
    let izero = |x: f64| {
        let (mut sum, mut u, mut n) = (1f64, 1f64, 1f64);
        let halfx = x / 2.0;
        loop {
            let mut temp = halfx / n;
            n += 1.0;
            temp *= temp;
            u *= temp;
            sum += u;
            if u < 1e-21 * sum {
                return sum;
            }
        }
    };
    let (frq, beta) = (0.5 * 0.90, 9.0);
    let mut c = vec![2.0 * frq; NWING];
    for (i, v) in c.iter_mut().enumerate().skip(1) {
        let temp = std::f64::consts::PI * i as f64 / NPC;
        *v = (2.0 * temp * frq).sin() / temp;
    }
    let ibeta = 1.0 / izero(beta);
    let inm1 = 1.0 / (NWING - 1) as f64;
    for (i, v) in c.iter_mut().enumerate().skip(1) {
        let temp = i as f64 * inm1;
        let temp1 = (1.0 - temp * temp).max(0.0);
        *v *= izero(beta * temp1.sqrt()) * ibeta;
    }
    let maxh = c.iter().fold(0f64, |m, v| m.max(v.abs()));
    let scl = 32767.0 / maxh;
    let imp: Vec<i32> = c.iter().map(|v| (v * scl + 0.5) as i32).collect();
    let mut impd: Vec<i32> = imp.windows(2).map(|w| w[1] - w[0]).collect();
    impd.push(-imp[NWING - 1]);
    (imp, impd)
});

/// One wing of the filter at input `x0` and phase `ph` (15 fraction bits),
/// stepping `inc` through the input (FilterUD, 0x1001bae0): coefficients
/// interpolated from 7 more bits, each product rounded to 14 fraction bits.
fn filter_ud(x: &dyn Fn(isize) -> i32, x0: isize, ph: u32, inc: isize, dhb: u32) -> i32 {
    let (imp, impd) = &*LARGE_FILTER;
    let mut ho = (ph * dhb) >> 15;
    let mut end = NWING;
    if inc == 1 {
        end -= 1;
        if ph == 0 {
            ho += dhb;
        }
    }
    let (mut v, mut xp) = (0i32, x0);
    while ((ho >> 7) as usize) < end {
        let k = (ho >> 7) as usize;
        let mut t = imp[k] + ((impd[k] * (ho & 0x7f) as i32) >> 7);
        t *= x(xp);
        if t & 0x2000 != 0 {
            t += 0x2000;
        }
        v += t >> 14;
        ho += dhb;
        xp += inc;
    }
    v
}

/// Resample a whole utterance from `from` to `to` Hz as LoqTTS7.dll does
/// before its audio leaves the engine (VoiceResamplerRun 0x10028ee0, when
/// the rates differ by more than 100 Hz): Julius O. Smith's `resample`
/// (0x1001c370, SrcUD 0x1001be70), fixed point, large filter, interpolated
/// coefficients; `floor(n·to/from)` samples out, silence before and after.
pub fn resample(input: &[i16], from: u32, to: u32) -> Vec<i16> {
    if from.abs_diff(to) <= 100 {
        return input.to_vec();
    }
    let factor = to as f64 / from as f64;
    // LpScl 14746 for unity gain, scaled down with the factor (0x1001c3ad)
    let lp_scl = if factor < 1.0 { (14746.0 * factor + 0.5) as i32 } else { 14746 };
    let dtb = (1.0 / factor * 32768.0 + 0.5) as u64;
    let dhb = ((NPC).min(factor * NPC) * 128.0 + 0.5) as u32;
    let n_out = (input.len() as f64 * factor).floor() as usize;
    let x = |i: isize| if i >= 0 && (i as usize) < input.len() { input[i as usize] as i32 } else { 0 };
    (0..n_out as u64)
        .map(|m| {
            let time = m * dtb;
            let at = (time >> 15) as isize;
            let ph = (time & 0x7fff) as u32;
            let v = filter_ud(&x, at + 1, (time as u32).wrapping_neg() & 0x7fff, 1, dhb) + filter_ud(&x, at, ph, -1, dhb);
            (((v >> 2) * lp_scl + 0x1000) >> 13).clamp(-32768, 32767) as i16
        })
        .collect()
}

/// The output resampler as the engine streams it (VoiceResamplerRun
/// 0x10028ee0, MECHANICS.md §2c), operation by operation. SIG hands it one
/// phone at a time ([`StreamResampler::push`]: both half units as the joins
/// left them, a pause too) and ends every phrase with
/// [`StreamResampler::flush`] (0x10028e40). What it keeps:
///
/// - the input not consumed yet (state +0x10, its count +0x18);
/// - Julius O. Smith's `resample` state (0x1001c6d0): the 5120-sample input
///   history `X` (0x2800 bytes, zeroed), refilled from `Xoff` = 55 on
///   every call so what lies past the new data is whatever it held, and the
///   time `Time`, set to `Xoff << 15` on the first call only;
/// - the output queue (0x10023e20): each phone is owed `floor(n·factor)`
///   samples and gets them, first in first out, once that many are made;
///   a flush hands every phone what is there and drops the rest.
pub struct StreamResampler {
    /// `to / from` (0x10028bb3), in doubles as the x87 at precision 53 computes it
    factor: f64,
    passthrough: bool,
    pending: Vec<i16>,
    x: Vec<i16>,
    time: u32,
    first: bool,
    made: std::collections::VecDeque<i16>,
    owed: std::collections::VecDeque<usize>,
    /// what each chunk got, in push order (chunks still owed are not here)
    handed: Vec<usize>,
}

impl StreamResampler {
    /// samples of `X` (0x1001c78a: 0x2800 bytes per channel)
    const XSIZE: usize = 0x1400;
    /// `IBUFFSIZE`: readData fills `X` up to here (0x1001c045)
    const IBUF: i32 = 0x1000;
    /// the large filter's `Nmult` (0x1001c9af)
    const NMULT: u16 = 65;
    /// `LpScl` for unity gain, `trunc(1.0 · 14746.0)` (+0x30 is 1.0,
    /// 0x1001c8c4; 14746.0 at 0x100a5cd8; 0x1001c9a0)
    const LP_SCL: u16 = 14746;

    pub fn new(from: u32, to: u32) -> StreamResampler {
        StreamResampler {
            factor: to as f64 / from as f64,
            // VoiceResamplerRun resamples when the rates differ by more than
            // 100 Hz (0x10028efd)
            passthrough: from.abs_diff(to) <= 100,
            pending: Vec::new(),
            x: vec![0; Self::XSIZE],
            time: 0,
            first: true,
            made: Default::default(),
            owed: Default::default(),
            handed: Vec::new(),
        }
    }

    /// One chunk in (0x10028ee0), what the engine hands on for it out.
    pub fn push(&mut self, chunk: &[i16]) -> Vec<i16> {
        if self.passthrough {
            self.handed.push(chunk.len());
            return chunk.to_vec();
        }
        self.pending.extend_from_slice(chunk);
        self.owed.push_back((chunk.len() as f64 * self.factor).floor() as usize);
        self.run(false)
    }

    /// How many samples each chunk pushed so far was handed on, in push
    /// order; the chunks still waiting for theirs are not listed yet.
    pub fn handed(&self) -> &[usize] {
        &self.handed
    }

    /// The end of a phrase (0x10028e40): nothing without input waiting;
    /// else one more call with the flag set, then the input is dropped.
    pub fn flush(&mut self) -> Vec<i16> {
        if self.passthrough || self.pending.is_empty() {
            return Vec::new();
        }
        let out = self.run(true);
        self.pending.clear();
        out
    }

    /// 0x10028c00: one call of the resampler, its outputs to the queue
    /// (0x10023ec0), which hands on what it owes (0x10024010).
    fn run(&mut self, last_call: bool) -> Vec<i16> {
        let y = self.call();
        self.made.extend(y);
        let mut out = Vec::new();
        while let Some(&owed) = self.owed.front() {
            // a phone is handed on once its samples are there; at a flush
            // with what there is (ELQBufferPopForced: at most what is made)
            if self.made.len() < owed && !last_call {
                break;
            }
            let k = owed.min(self.made.len());
            out.extend(self.made.drain(..k));
            self.owed.pop_front();
            self.handed.push(k);
        }
        if last_call {
            // the queue is empty: what is left over is dropped (0x10024190)
            self.made.clear();
        }
        out
    }

    /// The resampler's own outputs for the input waiting: `floor(pending ·
    /// factor)`, 16 fewer when above 16 (0x10028c93); the input consumed
    /// leaves the buffer (0x10028cd2).
    fn call(&mut self) -> Vec<i16> {
        let mut n = (self.pending.len() as f64 * self.factor).floor() as u32;
        if n > 16 {
            n -= 16;
        }
        let (y, consumed) = self.resample(n);
        if consumed < self.pending.len() {
            self.pending.drain(..consumed);
        } else {
            self.pending.clear();
        }
        y
    }

    /// 0x1001c370 for `n_out` outputs from `pending`: the outputs and the
    /// input consumed. Its counts are 16-bit (`UHWORD`), as in Smith's code.
    fn resample(&mut self, n_out: u32) -> (Vec<i16>, usize) {
        let factor = self.factor;
        let n_out = n_out as i32;
        // outputs one pass may hold (0x1001c377, `_ftol2`)
        let obuf = (factor * 4096.0) as i32;
        let lp_scl = if factor < 1.0 { ((Self::LP_SCL as f64 * factor + 0.5) as i32) as u16 } else { Self::LP_SCL };
        // Xoff = (Nmult + 1)/2 · max(1, 1/factor) + 10, truncated (0x1001c3e9)
        let xoff = ((((Self::NMULT + 1) as f64 * 0.5) * (1.0 / factor).max(1.0) + 10.0) as i32) as u16;
        let mut nx = (0x800 - xoff).wrapping_mul(2);
        if self.first {
            self.time = (xoff as u32) << 15;
        }
        let (mut last, mut ycount, mut consumed) = (0i32, 0i32, 0i32);
        let (mut xread, mut pos) = (xoff, 0i32);
        let mut out = vec![0i16; n_out.max(0) as usize];
        let mut y = Vec::with_capacity(obuf.max(0) as usize);
        loop {
            if last == 0 {
                // readData 0x1001c030: from the start of the input on the
                // call's first read, into X from Xread up to IBUFFSIZE
                let in_count = self.pending.len() as i32;
                let n = (Self::IBUF - xread as i32).min(in_count - pos);
                for i in 0..n.max(0) {
                    self.x[xread as usize + i as usize] = self.pending[(pos + i) as usize];
                }
                pos += n;
                if pos >= in_count {
                    last = xread as i32 + (in_count - pos + n) - 1;
                }
                if last != 0 && last - (xoff as i32) < nx as i32 {
                    nx = (last - xoff as i32) as u16;
                    if nx == 0 {
                        break;
                    }
                }
            }
            // this pass: what is still wanted, unless more than the buffer
            // holds past the filter's reach (0x1001c4d9)
            let remaining = n_out - ycount;
            let edge = (2 * xoff as i32) as f64 * factor;
            let nout = if obuf as f64 - edge >= remaining as f64 { remaining as u16 } else { (obuf as u16).wrapping_sub(edge as i32 as u16) };
            let mut time = self.time;
            y.clear();
            nx = self.src_ud(&mut time, nout, lp_scl, &mut y);
            self.time = time.wrapping_sub((nx as u32) << 15);
            if last != 0 {
                last -= ((nx as u32 + xoff as u32) & 0xffff) as i32;
                if last == 0 {
                    last = 1;
                }
            }
            let mut produced = y.len() as i32;
            ycount += produced;
            if ycount > n_out {
                produced -= ycount - n_out;
                ycount = n_out;
            }
            out[(ycount - produced) as usize..ycount as usize].copy_from_slice(&y[..produced as usize]);
            // keep the history: X[i] = X[i + Nx] for Xoff − Nx + 4096 samples
            let keep = xoff as i32 - nx as i32 + Self::IBUF;
            if keep > 0 {
                self.x.copy_within(nx as usize..nx as usize + keep as usize, 0);
            }
            consumed += nx as i32;
            xread = (Self::IBUF - nx as i32) as u16;
            if ycount >= n_out {
                break;
            }
        }
        // the caller clears the first-call flag (0x1001c9ea)
        self.first = false;
        out.truncate(ycount as usize);
        (out, consumed as usize)
    }

    /// SrcUD (0x1001be70): exactly `nout` outputs from `X` at `time`, which
    /// it moves on; returns how far its whole-sample part moved.
    fn src_ud(&self, time: &mut u32, nout: u16, lp_scl: u16, y: &mut Vec<i16>) -> u16 {
        let factor = self.factor;
        let dtb = (1.0 / factor * 32768.0 + 0.5) as i64 as u32;
        let dh = if 256.0 >= factor * 256.0 { factor * 256.0 } else { 256.0 };
        let dhb = ((dh * 128.0 + 0.5) as i64 as u32) & 0xffff;
        let start = *time >> 15;
        let x = &self.x;
        let xs = |i: isize| *x.get(i as usize).unwrap_or(&0) as i32;
        for _ in 0..nout {
            let at = (*time >> 15) as isize;
            let v = filter_ud(&xs, at + 1, time.wrapping_neg() & 0x7fff, 1, dhb).wrapping_add(filter_ud(&xs, at, *time & 0x7fff, -1, dhb));
            let v = ((v >> 2).wrapping_mul(lp_scl as i32).wrapping_add(0x1000)) >> 13;
            y.push(v.clamp(-32768, 32767) as i16);
            *time = time.wrapping_add(dtb);
        }
        ((*time >> 15).wrapping_sub(start)) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_other_files() {
        assert!(AudioBank::parse(vec![0u8; 400]).is_err());
    }

    /// The generated filter is LoqTTS7.dll's table at 0x1007ad88, value for
    /// value (FNV-1a of its 8192 16-bit words, and their sum).
    #[test]
    fn large_filter_is_the_dlls() {
        let (imp, impd) = &*LARGE_FILTER;
        let h = imp.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &s| (h ^ s as u16 as u64).wrapping_mul(0x100_0000_01b3));
        assert_eq!((h, imp.iter().sum::<i32>()), (0xeda1_f17a_afd3_39e8, 4_680_457));
        assert_eq!(impd[0], -1);
    }

    #[test]
    fn resampling_keeps_a_tone() {
        let tone: Vec<i16> = (0..44000).map(|i| (10000.0 * (i as f64 * 440.0 * 2.0 * std::f64::consts::PI / 44000.0).sin()) as i16).collect();
        let out = resample(&tone, 44000, 32000);
        assert_eq!(out.len(), 32000);
        let peak = out[1000..31000].iter().map(|v| v.abs()).max().unwrap();
        assert!((9500..=10500).contains(&peak), "{peak}");
    }

    /// `QQ_RESAMPLE_TRACE=<prefix>.res`, written by `oracle/loqref.c` with
    /// `LOQ_TRACE_RESAMPLE=3`: every call of the engine's resampler
    /// (0x1001c370) replayed from its logged state, the same calls as one
    /// stream per utterance, and what the queue hands on against the
    /// engine's `<prefix>-<n>.wav`.
    #[test]
    fn replays_the_engines_resampler() {
        let Ok(path) = std::env::var("QQ_RESAMPLE_TRACE") else { return };
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        // (the C line ends in the factor, a float)
        let nums = |l: &str| -> Vec<i64> { l.split_whitespace().skip(1).filter_map(|t| t.parse().ok()).collect() };
        let pcm = |v: &[i64]| -> Vec<i16> { v.iter().map(|&s| s as i16).collect() };
        let wav = |n: i64| -> Option<Vec<i16>> {
            let b = std::fs::read(format!("{}-{n}.wav", path.trim_end_matches(".res"))).ok()?;
            Some(b[44..].chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect())
        };
        let (mut calls, mut alone, mut streamed, mut utts, mut wavs_same) = (0, 0, 0, 0, 0);
        let mut raw = StreamResampler::new(44000, 32000);
        let mut queue = StreamResampler::new(44000, 32000);
        let mut handed: Vec<i16> = Vec::new();
        let mut want: Option<Vec<i16>> = None;
        let mut utt: Option<i64> = None;
        let mut finish = |utt: Option<i64>, handed: &[i16]| {
            if let Some(theirs) = utt.and_then(wav) {
                utts += 1;
                if theirs == handed {
                    wavs_same += 1;
                } else {
                    let k = theirs.iter().zip(handed).take_while(|(a, b)| a == b).count();
                    eprintln!("utterance {utt:?}: {} samples, ours {}, first difference at {k}", theirs.len(), handed.len());
                }
            }
        };
        let mut i = 0;
        while i < lines.len() {
            let l = lines[i];
            match l.as_bytes().first() {
                Some(b'U') => {
                    finish(utt, &handed);
                    utt = Some(nums(l)[0]);
                    raw = StreamResampler::new(44000, 32000);
                    queue = StreamResampler::new(44000, 32000);
                    handed.clear();
                }
                Some(b'R') => {
                    let s = pcm(&nums(l)[1..]);
                    handed.extend(queue.push(&s));
                    raw.pending.extend_from_slice(&s);
                    want = Some(raw.call());
                }
                Some(b'F') => {
                    handed.extend(queue.flush());
                    if !raw.pending.is_empty() {
                        want = Some(raw.call());
                        raw.pending.clear();
                    }
                }
                Some(b'C') => {
                    let c = nums(l);
                    let input = pcm(&nums(lines[i + 1]));
                    let x0 = pcm(&nums(lines[i + 2]));
                    let o = nums(lines[i + 3]);
                    let y = pcm(&nums(lines[i + 4]));
                    let x1 = pcm(&nums(lines[i + 5]));
                    calls += 1;
                    // alone, from the state the engine had
                    let mut r = StreamResampler::new(44000, 32000);
                    assert_eq!(r.factor.to_bits(), (8.0f64 / 11.0).to_bits());
                    (r.x, r.time, r.first, r.pending) = (x0, c[2] as u32, c[3] == 1, input.clone());
                    let ours = r.call();
                    let same = ours == y && r.time == o[2] as u32 && input.len() - r.pending.len() == o[1] as usize && r.x == x1;
                    if same {
                        alone += 1;
                    } else if calls - alone <= 5 {
                        let k = ours.iter().zip(&y).take_while(|(a, b)| a == b).count();
                        eprintln!(
                            "call {calls} (line {}): {} outputs, ours {} (first difference at {k}); consumed {} ours {}; time {} ours {}; X same {}",
                            i + 1,
                            y.len(),
                            ours.len(),
                            o[1],
                            input.len() - r.pending.len(),
                            o[2],
                            r.time,
                            r.x == x1
                        );
                    }
                    if want.take().is_some_and(|w| w == y) {
                        streamed += 1;
                    }
                    i += 5;
                }
                _ => {}
            }
            i += 1;
        }
        finish(utt, &handed);
        eprintln!("{calls} calls: {alone} identical from the engine's state, {streamed} in one stream; {wavs_same} of {utts} utterances identical");
        assert_eq!((alone, streamed, wavs_same), (calls, calls, utts));
    }
}
