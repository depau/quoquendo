//! The voice recordings: `<voice>.32000.LTTS7Codec210.44000.10.bin`.
//!
//! LTTS7Codec210 is Speex ultra-wideband (the strings and decoder set-up in
//! LTTS7Codec210.dll are libspeex's). Layout and decoder settings are in
//! MECHANICS.md §5, each with the address it was read from.

use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;

/// SpeexBits is a small C struct; it is only ever touched by libspeex.
#[repr(C, align(8))]
struct SpeexBits([u8; 64]);

extern "C" {
    fn speex_lib_get_mode(mode: c_int) -> *const c_void;
    fn speex_decoder_init(mode: *const c_void) -> *mut c_void;
    fn speex_decoder_destroy(state: *mut c_void);
    fn speex_decoder_ctl(state: *mut c_void, request: c_int, ptr: *mut c_void) -> c_int;
    fn speex_decode_int(state: *mut c_void, bits: *mut SpeexBits, out: *mut i16) -> c_int;
    fn speex_bits_init(bits: *mut SpeexBits);
    fn speex_bits_destroy(bits: *mut SpeexBits);
    fn speex_bits_read_from(bits: *mut SpeexBits, bytes: *const c_char, len: c_int);
}

const SPEEX_MODEID_UWB: c_int = 2;
const SPEEX_SET_ENH: c_int = 0;
const SPEEX_SET_SAMPLING_RATE: c_int = 24; // 0x18, LTTS7Codec210 0x10004230
/// Speex 1.2 high-passes its output; the older Speex inside LTTS7Codec210 has
/// no such filter (measured: with it off, 0-8 kHz matches the original engine
/// to 38-48 dB instead of 10 dB below 250 Hz).
const SPEEX_SET_HIGHPASS: c_int = 44;
/// Output lags the original by this many samples at 32 kHz (0x1fd, ELQOpenDecoder 0x100045e0).
const DECODER_DELAY_32K: usize = 509;
/// Frames decoded before a seek target to settle the decoder (6, same function).
const WARMUP_FRAMES_32K: usize = 6;

pub struct AudioBank {
    data: Vec<u8>,
    first_frame: usize,
    pub frame_bytes: usize,
    pub frame_samples: usize,
    pub frames: usize,
    pub rate: u32,
    pub bitrate: u32,
    key: u8,
}

impl AudioBank {
    pub fn open(path: &Path) -> Result<AudioBank, String> {
        let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        AudioBank::parse(data).map_err(|e| format!("{}: {e}", path.display()))
    }

    fn parse(data: Vec<u8>) -> Result<AudioBank, String> {
        if data.len() < 64 {
            return Err("too short".into());
        }
        // text header: XOR with byte0^byte1, then "LOQUENDOTTS7" and
        // (id u16le, len u16le) fields; field 1 carries the header length itself
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
        let be = |o: usize| -> Result<u32, String> {
            data.get(hdr + o..hdr + o + 4)
                .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
                .ok_or_else(|| "truncated header".to_string())
        };
        let bank = AudioBank {
            bitrate: be(0)?,
            frame_samples: be(8)? as usize,
            rate: be(12)?,
            frame_bytes: be(16)? as usize,
            frames: be(20)? as usize,
            key: (be(24)? & 0xff) as u8,
            first_frame: hdr + 28,
            data,
        };
        if bank.rate != 32000 || bank.frame_samples != 640 {
            return Err(format!(
                "{} Hz / {}-sample frames: only the 32 kHz voices are supported",
                bank.rate, bank.frame_samples
            ));
        }
        if bank.first_frame + bank.frames * bank.frame_bytes > bank.data.len() {
            return Err("fewer frames in the file than the header says".into());
        }
        Ok(bank)
    }

    pub fn total_samples(&self) -> usize {
        self.frames * self.frame_samples
    }

    /// `len` samples of the original recording starting at sample `start`.
    pub fn read(&self, start: usize, len: usize) -> Vec<i16> {
        let mut out = Vec::with_capacity(len);
        let pos = start + DECODER_DELAY_32K;
        let first = pos / self.frame_samples;
        let skip = pos - first * self.frame_samples;
        let from = first.saturating_sub(WARMUP_FRAMES_32K);
        let mut pcm = vec![0i16; self.frame_samples];
        let mut frame = vec![0u8; self.frame_bytes];
        // SAFETY: the decoder and bits live only inside this call, every buffer
        // is sized to what the UWB mode writes (640 samples, frame_bytes bytes).
        unsafe {
            let st = speex_decoder_init(speex_lib_get_mode(SPEEX_MODEID_UWB));
            let mut rate: c_int = self.rate as c_int;
            speex_decoder_ctl(st, SPEEX_SET_SAMPLING_RATE, &mut rate as *mut _ as *mut c_void);
            let mut enh: c_int = 0;
            speex_decoder_ctl(st, SPEEX_SET_ENH, &mut enh as *mut _ as *mut c_void);
            let mut highpass: c_int = 0;
            speex_decoder_ctl(st, SPEEX_SET_HIGHPASS, &mut highpass as *mut _ as *mut c_void);
            let mut bits = SpeexBits([0; 64]);
            speex_bits_init(&mut bits);
            let mut k = from;
            while out.len() < skip + len &&k < self.frames {
                let at = self.first_frame + k * self.frame_bytes;
                frame.copy_from_slice(&self.data[at..at + self.frame_bytes]);
                if self.key != 0 {
                    frame.iter_mut().for_each(|b| *b ^= self.key);
                }
                speex_bits_read_from(&mut bits, frame.as_ptr() as *const c_char, frame.len() as c_int);
                speex_decode_int(st, &mut bits, pcm.as_mut_ptr());
                if k >= first {
                    out.extend_from_slice(&pcm);
                }
                k += 1;
            }
            speex_bits_destroy(&mut bits);
            speex_decoder_destroy(st);
        }
        out.drain(..skip.min(out.len()));
        out.resize(len, 0);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_other_files() {
        assert!(AudioBank::parse(vec![0u8; 400]).is_err());
    }
}
