//! Saving speech as a file: WAV, FLAC, MP3 or Opus, each carrying the text
//! it says with the time of every word (when the words are known).
//!
//! - WAV: 16-bit PCM, a cue point per word labelled with its text (`cue `,
//!   `LIST`/`adtl`/`labl`), the text as its name (`LIST`/`INFO`).
//! - FLAC (`flacenc`): a `VORBIS_COMMENT` with `TITLE`, `ARTIST` (the voice)
//!   and `LYRICS`, enhanced LRC (`lrc`).
//! - MP3 (`audio2mp3-core`, 96 kbit/s): an ID3v2.4 tag with `TIT2`, `TPE1`,
//!   `TSSE`, `USLT` (the LRC) and `SYLT` (every word at its millisecond).
//! - Opus (`opus-pure`, 48 kHz, 32 kbit/s, in Ogg): `OpusTags` as FLAC's.

use std::ops::Range;

use crate::synth::{to_pcm16, Speech};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Format {
    Wav,
    Flac,
    Mp3,
    Opus,
}

impl Format {
    pub const ALL: [Format; 4] = [Format::Wav, Format::Flac, Format::Mp3, Format::Opus];

    pub fn ext(self) -> &'static str {
        match self {
            Format::Wav => "wav",
            Format::Flac => "flac",
            Format::Mp3 => "mp3",
            Format::Opus => "opus",
        }
    }

    pub fn mime(self) -> &'static str {
        match self {
            Format::Wav => "audio/wav",
            Format::Flac => "audio/flac",
            Format::Mp3 => "audio/mpeg",
            Format::Opus => "audio/ogg",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Format::Wav => "WAV",
            Format::Flac => "FLAC",
            Format::Mp3 => "MP3",
            Format::Opus => "Opus",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Format::Wav => "lossless · word markers",
            Format::Flac => "lossless · synced lyrics",
            Format::Mp3 => "small · synced lyrics",
            Format::Opus => "smallest · synced lyrics",
        }
    }

    /// The format a file extension names (any case; `ogg` is Opus).
    pub fn from_ext(ext: &str) -> Option<Format> {
        let ext = ext.to_ascii_lowercase();
        match ext.as_str() {
            "ogg" | "oga" => Some(Format::Opus),
            _ => Format::ALL.into_iter().find(|f| f.ext() == ext),
        }
    }
}

/// What a file says: the text spoken, where each of its words is in it
/// (byte ranges by word index, `Utterance::words`) and the voice's name.
pub struct Tagged<'a> {
    pub text: &'a str,
    pub words: &'a [Range<usize>],
    pub voice: &'a str,
}

/// The speech as a file in `format`, with `tags` embedded.
pub fn encode(speech: &Speech, format: Format, tags: Option<&Tagged>) -> Result<Vec<u8>, String> {
    match format {
        Format::Wav => Ok(wav(speech, tags)),
        Format::Flac => flac(speech, tags),
        Format::Mp3 => mp3(speech, tags),
        Format::Opus => opus(speech, tags),
    }
}

/// The software named in the files.
const SOFTWARE: &str = "quoquendo";
/// Longest title, in characters (the whole text is in the lyrics).
const TITLE_CHARS: usize = 200;

/// One word said: when it starts (sample), its phrase and its text.
struct Mark<'a> {
    at: usize,
    phrase: usize,
    text: &'a str,
}

/// Every word said with its original text, in order. A stretch of text
/// standing for several words (a number said as two) is marked once.
fn marks<'a>(speech: &Speech, tags: &Tagged<'a>) -> Vec<Mark<'a>> {
    let mut out = Vec::new();
    let (mut last_word, mut last_range): (Option<usize>, Option<&Range<usize>>) = (None, None);
    for s in &speech.timeline.sounds {
        if s.word.is_none() || s.word != last_word {
            last_word = s.word;
            let Some(range) = s.word.and_then(|w| tags.words.get(w)) else { continue };
            let Some(text) = tags.text.get(range.clone()).filter(|t| !t.trim().is_empty()) else {
                continue;
            };
            if last_range == Some(range) {
                continue;
            }
            last_range = Some(range);
            out.push(Mark {
                at: s.span.start,
                phrase: s.phrase,
                text,
            });
        }
    }
    out
}

/// `text` on one line, whitespace runs as one space.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The text as a title: one line, at most `TITLE_CHARS` characters.
fn title(text: &str) -> String {
    let line = one_line(text);
    match line.char_indices().nth(TITLE_CHARS) {
        Some((cut, _)) => format!("{}…", line[..cut].trim_end()),
        None => line,
    }
}

/// `mm:ss.xx` of a sample position.
fn stamp(at: usize, rate: u32) -> String {
    let cs = at as u64 * 100 / rate.max(1) as u64;
    format!("{:02}:{:02}.{:02}", cs / 6000, cs / 100 % 60, cs % 100)
}

/// Enhanced LRC of the speech: a line per phrase, `[mm:ss.xx]` at its first
/// word, then `<mm:ss.xx>word` for every word. Without words, the whole
/// text on one line at 0.
pub fn lrc(speech: &Speech, tags: &Tagged) -> String {
    let marks = marks(speech, tags);
    if marks.is_empty() {
        let text = one_line(tags.text);
        return if text.is_empty() { String::new() } else { format!("[00:00.00]{text}") };
    }
    let mut lines: Vec<String> = Vec::new();
    let mut phrase = None;
    for m in &marks {
        if phrase != Some(m.phrase) {
            phrase = Some(m.phrase);
            lines.push(format!("[{}]", stamp(m.at, speech.rate)));
        }
        let line = lines.last_mut().expect("a line was started");
        if !line.ends_with(']') {
            line.push(' ');
        }
        line.push_str(&format!("<{}>{}", stamp(m.at, speech.rate), one_line(m.text)));
    }
    lines.join("\n")
}

// --- WAV ---

/// A RIFF chunk: id, length, body, a pad byte to an even length.
fn chunk(out: &mut Vec<u8>, id: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(id);
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    if body.len() % 2 == 1 {
        out.push(0);
    }
}

/// A NUL-terminated string.
fn zstr(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

fn wav(speech: &Speech, tags: Option<&Tagged>) -> Vec<u8> {
    let rate = speech.rate;
    let mut body = b"WAVE".to_vec();
    let mut fmt = Vec::with_capacity(16);
    fmt.extend_from_slice(&1u16.to_le_bytes()); // PCM
    fmt.extend_from_slice(&1u16.to_le_bytes()); // mono
    fmt.extend_from_slice(&rate.to_le_bytes());
    fmt.extend_from_slice(&(rate * 2).to_le_bytes());
    fmt.extend_from_slice(&2u16.to_le_bytes());
    fmt.extend_from_slice(&16u16.to_le_bytes());
    chunk(&mut body, b"fmt ", &fmt);
    if let Some(t) = tags {
        let mut info = b"INFO".to_vec();
        chunk(&mut info, b"INAM", &zstr(&title(t.text)));
        chunk(&mut info, b"ISFT", &zstr(SOFTWARE));
        chunk(&mut body, b"LIST", &info);
    }
    let data: Vec<u8> = to_pcm16(&speech.samples).into_iter().flat_map(i16::to_le_bytes).collect();
    chunk(&mut body, b"data", &data);
    let marks = tags.map(|t| marks(speech, t)).unwrap_or_default();
    if !marks.is_empty() {
        let mut cue = (marks.len() as u32).to_le_bytes().to_vec();
        let mut adtl = b"adtl".to_vec();
        for (i, m) in marks.iter().enumerate() {
            let id = (i as u32 + 1).to_le_bytes();
            let at = (m.at as u32).to_le_bytes();
            cue.extend_from_slice(&id);
            cue.extend_from_slice(&at); // position
            cue.extend_from_slice(b"data");
            cue.extend_from_slice(&0u32.to_le_bytes()); // chunk start
            cue.extend_from_slice(&0u32.to_le_bytes()); // block start
            cue.extend_from_slice(&at); // sample offset
            let mut labl = id.to_vec();
            labl.extend(zstr(&one_line(m.text)));
            chunk(&mut adtl, b"labl", &labl);
        }
        chunk(&mut body, b"cue ", &cue);
        chunk(&mut body, b"LIST", &adtl);
    }
    let mut out = Vec::with_capacity(8 + body.len());
    chunk(&mut out, b"RIFF", &body);
    out
}

// --- FLAC, Opus: Vorbis comments ---

/// The `NAME=value` comments for FLAC and Opus.
fn comments(speech: &Speech, t: &Tagged) -> Vec<(&'static str, String)> {
    let mut c = vec![("TITLE", title(t.text)), ("ARTIST", t.voice.to_string()), ("ENCODER", SOFTWARE.to_string())];
    let lyrics = lrc(speech, t);
    if !lyrics.is_empty() {
        c.push(("LYRICS", lyrics));
    }
    c
}

/// A Vorbis comment header body (FLAC's `VORBIS_COMMENT` block).
fn vorbis_comment(comments: &[(&str, String)]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&(SOFTWARE.len() as u32).to_le_bytes());
    v.extend_from_slice(SOFTWARE.as_bytes());
    v.extend_from_slice(&(comments.len() as u32).to_le_bytes());
    for (name, value) in comments {
        let entry = format!("{name}={value}");
        v.extend_from_slice(&(entry.len() as u32).to_le_bytes());
        v.extend_from_slice(entry.as_bytes());
    }
    v
}

fn flac(speech: &Speech, tags: Option<&Tagged>) -> Result<Vec<u8>, String> {
    use flacenc::component::{BitRepr, MetadataBlockData};
    use flacenc::error::Verify;
    let pcm: Vec<i32> = to_pcm16(&speech.samples).into_iter().map(i32::from).collect();
    let config = flacenc::config::Encoder::default().into_verified().map_err(|e| format!("FLAC: {e:?}"))?;
    let source = flacenc::source::MemSource::from_samples(&pcm, 1, 16, speech.rate as usize);
    let mut stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size).map_err(|e| format!("FLAC: {e:?}"))?;
    if let Some(t) = tags {
        // block type 4: VORBIS_COMMENT
        let block = MetadataBlockData::new_unknown(4, &vorbis_comment(&comments(speech, t))).map_err(|e| format!("FLAC: {e:?}"))?;
        stream.add_metadata_block(block);
    }
    let mut sink = flacenc::bitsink::ByteSink::new();
    stream.write(&mut sink).map_err(|e| format!("FLAC: {e:?}"))?;
    Ok(sink.as_slice().to_vec())
}

// --- MP3 ---

/// `n` in four 7-bit bytes, as ID3v2.4 sizes are written (at most 2²⁸ − 1).
fn synchsafe(n: usize) -> [u8; 4] {
    let n = n.min(0x0fff_ffff) as u32;
    [(n >> 21) as u8 & 0x7f, (n >> 14) as u8 & 0x7f, (n >> 7) as u8 & 0x7f, n as u8 & 0x7f]
}

/// One ID3v2.4 frame.
fn id3_frame(out: &mut Vec<u8>, id: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(id);
    out.extend_from_slice(&synchsafe(body.len()));
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(body);
}

/// UTF-8 (encoding 3) text frame body.
fn id3_text(s: &str) -> Vec<u8> {
    let mut v = vec![3];
    v.extend_from_slice(s.as_bytes());
    v
}

/// An ID3v2.4 tag: title, voice, software, the LRC as unsynchronised
/// lyrics and every word in synchronised lyrics (milliseconds).
fn id3(speech: &Speech, t: &Tagged) -> Vec<u8> {
    let mut frames = Vec::new();
    id3_frame(&mut frames, b"TIT2", &id3_text(&title(t.text)));
    id3_frame(&mut frames, b"TPE1", &id3_text(t.voice));
    id3_frame(&mut frames, b"TSSE", &id3_text(SOFTWARE));
    let lyrics = lrc(speech, t);
    if !lyrics.is_empty() {
        // UTF-8, undetermined language, empty description
        let mut uslt = b"\x03und\x00".to_vec();
        uslt.extend_from_slice(lyrics.as_bytes());
        id3_frame(&mut frames, b"USLT", &uslt);
    }
    let marks = marks(speech, t);
    if !marks.is_empty() {
        // UTF-8, undetermined language, milliseconds, lyrics, no description;
        // each word follows the one before with a space, a phrase on a new line
        let mut sylt = b"\x03und\x02\x01\x00".to_vec();
        for (i, m) in marks.iter().enumerate() {
            let lead = match i {
                0 => "",
                _ if marks[i - 1].phrase != m.phrase => "\n",
                _ => " ",
            };
            sylt.extend_from_slice(lead.as_bytes());
            sylt.extend_from_slice(one_line(m.text).as_bytes());
            sylt.push(0);
            let ms = (m.at as u64 * 1000 / speech.rate.max(1) as u64) as u32;
            sylt.extend_from_slice(&ms.to_be_bytes());
        }
        id3_frame(&mut frames, b"SYLT", &sylt);
    }
    let mut out = b"ID3\x04\x00\x00".to_vec();
    out.extend_from_slice(&synchsafe(frames.len()));
    out.extend(frames);
    out
}

/// A mono MPEG-1 Layer III stream at 96 kbit/s (32, 44.1 or 48 kHz) whose
/// first frame is a Xing/Info frame with its exact length.
fn mp3_stream(pcm: &[i16], rate: u32) -> Result<Vec<u8>, String> {
    use audio2mp3_core::{Bitrate, EncoderConfig, Mp3Encoder};
    let err = |e: audio2mp3_core::Error| format!("MP3 at {rate} Hz: {e:?}");
    let mut enc = Mp3Encoder::new(EncoderConfig {
        sample_rate: rate,
        channels: 1,
        bitrate: Bitrate::Kbps96,
    })
    .map_err(err)?;
    enc.enable_info_frame().map_err(err)?;
    let mut out = Vec::with_capacity(pcm.len() / 2);
    enc.encode(pcm, &mut out).map_err(err)?;
    enc.flush(&mut out).map_err(err)?;
    let n = enc.info_frame_size();
    enc.info_frame_into(&mut out[..n]).map_err(err)?;
    Ok(out)
}

fn mp3(speech: &Speech, tags: Option<&Tagged>) -> Result<Vec<u8>, String> {
    let stream = mp3_stream(&to_pcm16(&speech.samples), speech.rate)?;
    let mut out = tags.map(|t| id3(speech, t)).unwrap_or_default();
    out.extend(stream);
    Ok(out)
}

// --- Opus ---

/// The rate Opus is coded at here.
const OPUS_RATE: u32 = 48_000;

fn opus(speech: &Speech, tags: Option<&Tagged>) -> Result<Vec<u8>, String> {
    use opus_pure::{Application, OggOpusWriter, OpusEncoder, OpusHead, OpusTags, MAX_PACKET_BYTES};
    let err = |e: opus_pure::Error| format!("Opus: {e}");
    let pcm = to_pcm16(&speech.samples);
    let input: Vec<f32> = if speech.rate == OPUS_RATE {
        pcm
    } else {
        crate::codec::resample(&pcm, speech.rate, OPUS_RATE)
    }
    .into_iter()
    .map(|s| s as f32 / 32768.0)
    .collect();
    let mut enc = OpusEncoder::new(OPUS_RATE as i32, 1, Application::Voip).map_err(err)?;
    enc.bitrate_bps = 32_000;
    let mut ot = OpusTags {
        vendor: SOFTWARE.into(),
        comments: Vec::new(),
    };
    for (name, value) in tags.map(|t| comments(speech, t)).unwrap_or_default() {
        ot.push(name, &value).map_err(err)?;
    }
    let head = OpusHead::for_encoder(&enc, speech.rate);
    // 20 ms frames; the encoder's delay (the pre-skip) is flushed with
    // silence, and the last page's granule says where the audio ends
    // (RFC 7845 §4.2, §4.4)
    let frame = (OPUS_RATE / 50) as usize;
    let lookahead = head.pre_skip as usize;
    let frames = (input.len() + lookahead).div_ceil(frame).max(1);
    let final_granule = head.pre_skip as u64 + input.len() as u64;
    let mut w = OggOpusWriter::with_tags(Vec::new(), head, ot).map_err(err)?;
    let mut packet = vec![0u8; MAX_PACKET_BYTES];
    let mut block = vec![0.0f32; frame];
    for i in 0..frames {
        let start = (i * frame).min(input.len());
        let end = (start + frame).min(input.len());
        block[..end - start].copy_from_slice(&input[start..end]);
        block[end - start..].fill(0.0);
        let n = enc.encode(&block, frame, &mut packet).map_err(err)?;
        if i + 1 == frames {
            let duration = final_granule - w.granule() as u64;
            w.write_packet_with_duration(&packet[..n], duration as u32).map_err(err)?;
        } else {
            w.write_packet(&packet[..n]).map_err(err)?;
        }
    }
    w.finish().map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{TimedSound, Timeline};

    /// 1 s at 1 kHz: "Ho 14" (phrase 0, sounds at 0, 100, 300), a pause,
    /// "anni" (phrase 1, at 600), a pause; the number said as two words.
    fn speech(rate: u32) -> Speech {
        let k = rate as usize / 1000;
        let sound = |sym: &str, ms: Range<usize>, word: Option<usize>, phrase: usize| TimedSound {
            sym: sym.into(),
            span: ms.start * k..ms.end * k,
            pause: word.is_none(),
            word,
            word_start: true,
            phrase,
        };
        Speech {
            samples: (0..rate as usize).map(|i| (i as f32 * 0.05).sin() * 0.3).collect(),
            rate,
            timeline: Timeline {
                sounds: vec![
                    sound("O", 0..100, Some(0), 0),
                    sound("kw", 100..300, Some(1), 0),
                    sound("d", 300..450, Some(2), 0),
                    sound("", 450..600, None, 0),
                    sound("a", 600..900, Some(3), 1),
                    sound("", 900..1000, None, 1),
                ],
                pieces: vec![0..450 * k, 600 * k..900 * k],
            },
        }
    }

    const TEXT: &str = "Ho 14\nanni";

    fn tags() -> Tagged<'static> {
        Tagged {
            text: TEXT,
            words: &[0..2, 3..5, 3..5, 6..10],
            voice: "Roberto",
        }
    }

    /// The (id, body) chunks of a RIFF body after its form type.
    fn riff_chunks(mut b: &[u8]) -> Vec<(&[u8], &[u8])> {
        let mut out = Vec::new();
        while b.len() >= 8 {
            let len = u32::from_le_bytes(b[4..8].try_into().unwrap()) as usize;
            out.push((&b[..4], &b[8..8 + len]));
            b = &b[(8 + len + len % 2).min(b.len())..];
        }
        out
    }

    #[test]
    fn formats_by_extension() {
        assert_eq!(Format::from_ext("FLAC"), Some(Format::Flac));
        assert_eq!(Format::from_ext("ogg"), Some(Format::Opus));
        assert_eq!(Format::from_ext("Mp3"), Some(Format::Mp3));
        assert_eq!(Format::from_ext("aiff"), None);
        for f in Format::ALL {
            assert_eq!(Format::from_ext(f.ext()), Some(f));
        }
    }

    #[test]
    fn lrc_has_a_line_per_phrase_and_a_stamp_per_word() {
        let s = speech(1000);
        // the number's two words are one mark
        assert_eq!(lrc(&s, &tags()), "[00:00.00]<00:00.00>Ho <00:00.10>14\n[00:00.60]<00:00.60>anni");
        assert_eq!(stamp(32000 * 754 + 16000, 32000), "12:34.50");
        // no words known: the text on one line
        let none = Tagged { words: &[], ..tags() };
        assert_eq!(lrc(&s, &none), "[00:00.00]Ho 14 anni");
    }

    #[test]
    fn titles_are_one_short_line() {
        assert_eq!(title(" a\n b "), "a b");
        let long = "x".repeat(300);
        assert_eq!(title(&long).chars().count(), TITLE_CHARS + 1);
    }

    #[test]
    fn wav_has_a_labelled_cue_per_word() {
        let s = speech(1000);
        let f = encode(&s, Format::Wav, Some(&tags())).unwrap();
        assert_eq!(&f[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(f[4..8].try_into().unwrap()) as usize, f.len() - 8);
        assert_eq!(&f[8..12], b"WAVE");
        let chunks = riff_chunks(&f[12..]);
        let ids: Vec<&[u8]> = chunks.iter().map(|c| c.0).collect();
        assert_eq!(ids, [b"fmt ", b"LIST", b"data", b"cue ", b"LIST"].map(|x| &x[..]));
        assert_eq!(chunks[2].1.len(), 2 * s.samples.len());
        let info = riff_chunks(&chunks[1].1[4..]);
        assert_eq!(&chunks[1].1[..4], b"INFO");
        assert_eq!(info[0], (&b"INAM"[..], &b"Ho 14 anni\0"[..]));
        assert_eq!(info[1], (&b"ISFT"[..], &b"quoquendo\0"[..]));
        let cue = chunks[3].1;
        assert_eq!(u32::from_le_bytes(cue[..4].try_into().unwrap()), 3);
        assert_eq!(cue.len(), 4 + 3 * 24);
        // the second cue: id 2 at sample 100, in "data"
        let c2 = &cue[4 + 24..4 + 48];
        assert_eq!(u32::from_le_bytes(c2[..4].try_into().unwrap()), 2);
        assert_eq!(u32::from_le_bytes(c2[4..8].try_into().unwrap()), 100);
        assert_eq!(&c2[8..12], b"data");
        assert_eq!(u32::from_le_bytes(c2[20..24].try_into().unwrap()), 100);
        assert_eq!(&chunks[4].1[..4], b"adtl");
        let labels: Vec<&[u8]> = riff_chunks(&chunks[4].1[4..])
            .into_iter()
            .map(|(id, b)| {
                assert_eq!(id, b"labl");
                &b[4..]
            })
            .collect();
        assert_eq!(labels, [&b"Ho\0"[..], b"14\0", b"anni\0"]);
        // untagged: a plain WAV
        let plain = encode(&s, Format::Wav, None).unwrap();
        assert_eq!(plain.len(), 44 + 2 * s.samples.len());
    }

    #[test]
    fn id3_frames_are_sized_synchsafe() {
        assert_eq!(synchsafe(0x7f), [0, 0, 0, 0x7f]);
        assert_eq!(synchsafe(0x80), [0, 0, 1, 0]);
        assert_eq!(synchsafe(0x0fff_ffff), [0x7f; 4]);
        let s = speech(32000);
        let tag = id3(&s, &tags());
        assert_eq!(&tag[..6], b"ID3\x04\x00\x00");
        let size = |b: &[u8]| b.iter().fold(0usize, |a, &x| (a << 7) | x as usize);
        assert_eq!(size(&tag[6..10]), tag.len() - 10);
        let mut frames = Vec::new();
        let mut b = &tag[10..];
        while !b.is_empty() {
            let len = size(&b[4..8]);
            frames.push((std::str::from_utf8(&b[..4]).unwrap(), &b[10..10 + len]));
            b = &b[10 + len..];
        }
        let ids: Vec<&str> = frames.iter().map(|f| f.0).collect();
        assert_eq!(ids, ["TIT2", "TPE1", "TSSE", "USLT", "SYLT"]);
        assert_eq!(frames[1].1, b"\x03Roberto");
        assert!(frames[3].1.starts_with(b"\x03und\x00[00:00.00]<00:00.00>Ho"));
        let mut sylt = b"\x03und\x02\x01\x00".to_vec();
        for (text, ms) in [("Ho", 0u32), (" 14", 100), ("\nanni", 600)] {
            sylt.extend_from_slice(text.as_bytes());
            sylt.push(0);
            sylt.extend_from_slice(&ms.to_be_bytes());
        }
        assert_eq!(frames[4].1, &sylt[..]);
    }

    #[test]
    fn mp3_starts_with_its_tag_then_the_info_frame() {
        let s = speech(32000);
        let f = encode(&s, Format::Mp3, Some(&tags())).unwrap();
        let tag = id3(&s, &tags());
        assert_eq!(&f[..tag.len()], &tag[..]);
        let mpeg = &f[tag.len()..];
        assert_eq!(mpeg[0], 0xff);
        assert_eq!(mpeg[1] & 0xe0, 0xe0);
        assert!(mpeg[..200].windows(4).any(|w| w == b"Info" || w == b"Xing"));
    }

    #[test]
    fn flac_and_opus_carry_the_lyrics() {
        let s = speech(32000);
        let flac = encode(&s, Format::Flac, Some(&tags())).unwrap();
        assert_eq!(&flac[..4], b"fLaC");
        // STREAMINFO, then the VORBIS_COMMENT marked last
        assert_eq!(flac[4] & 0x7f, 0);
        let first = 8 + 34;
        assert_eq!(flac[first], 0x80 | 4);
        let len = u32::from_be_bytes([0, flac[first + 1], flac[first + 2], flac[first + 3]]) as usize;
        let block = &flac[first + 4..first + 4 + len];
        assert_eq!(block, &vorbis_comment(&comments(&s, &tags()))[..]);
        let lrc = lrc(&s, &tags());
        assert!(block.windows(lrc.len()).any(|w| w == lrc.as_bytes()));
        let opus = encode(&s, Format::Opus, Some(&tags())).unwrap();
        assert_eq!(&opus[..4], b"OggS");
        for needle in [&b"OpusHead"[..], b"OpusTags", b"LYRICS=[00:00.00]", b"ARTIST=Roberto"] {
            assert!(opus.windows(needle.len()).any(|w| w == needle), "{}", String::from_utf8_lossy(needle));
        }
    }
}
