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
}
