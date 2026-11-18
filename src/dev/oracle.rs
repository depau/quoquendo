//! The original Loquendo engine as a reference (and as swappable stages).
//!
//! `oracle/run.sh` runs the real engine under Wine through `oracle/loqref.exe`
//! and records, per utterance, what its stages produced (MECHANICS.md §8):
//!   `.fon`  FON output: phonetic transcription (words joined by `-`)
//!   `.ev`   events: 3 = each phone said with its duration in ms (pauses `#`,
//!           `@` included), 19 = each piece CAT chose: sentence + half-phones
//!   `.wav`  SIG output, 32 kHz mono
//! Results are cached under `oracle/cache`, so each text runs the engine once.

use std::path::{Path, PathBuf};

use quoquendo::loquendo::VoiceDatabase;
use quoquendo::synth::{Phrase, Piece, Target, TargetPhrase};

#[derive(Clone, Debug, Default)]
pub struct Reference {
    pub text: String,
    /// FON: the engine's transcription
    pub transcription: String,
    /// every phone it said, with its duration in ms
    pub phones: Vec<(String, u32)>,
    /// every piece it chose: (recorded sentence, number of half-phones)
    pub pieces: Vec<(u32, u32)>,
    /// SIG: its audio
    pub audio: Vec<i16>,
}

fn is_pause(sym: &str) -> bool {
    sym == "#" || sym == "@"
}

/// FNV-1a, for cache file names.
fn key(voice: &str, text: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in voice.bytes().chain([0]).chain(text.bytes()) {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    }
    format!("{voice}-{h:016x}")
}

/// The `oracle` folder: next to the current folder or any parent of the program.
pub fn oracle_dir() -> Option<PathBuf> {
    let mut starts = vec![std::env::current_dir().ok()?];
    if let Ok(exe) = std::env::current_exe() {
        starts.push(exe);
    }
    for s in starts {
        for dir in s.ancestors() {
            if dir.join("oracle/run.sh").is_file() {
                return Some(dir.join("oracle"));
            }
        }
    }
    None
}

/// The original engine's results for each text, running it once for all
/// texts not yet cached.
pub fn run_many(voice: &str, texts: &[String]) -> Result<Vec<Reference>, String> {
    let dir = oracle_dir().ok_or("oracle/run.sh not found (run from the quoquendo folder)")?;
    let cache = dir.join("cache");
    std::fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    let todo: Vec<&String> = texts.iter().filter(|t| !cache.join(format!("{}.wav", key(voice, t))).is_file()).collect();
    if !todo.is_empty() {
        let batch = cache.join("batch.txt");
        let lines: Vec<String> = todo.iter().map(|t| t.replace(['\n', '\r'], " ")).collect();
        std::fs::write(&batch, lines.join("\n") + "\n").map_err(|e| e.to_string())?;
        let out = std::process::Command::new(dir.join("run.sh"))
            .arg(voice)
            .arg(&batch)
            .arg(cache.join("batch"))
            .env("LOQ_LINES", "1")
            .output()
            .map_err(|e| format!("running the original engine: {e}"))?;
        if !String::from_utf8_lossy(&out.stdout).contains("ok") {
            return Err(format!("the original engine failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
        }
        for (i, t) in todo.iter().enumerate() {
            for ext in ["fon", "ev", "wav"] {
                let _ = std::fs::rename(cache.join(format!("batch-{i}.{ext}")), cache.join(format!("{}.{ext}", key(voice, t))));
            }
            let _ = std::fs::write(cache.join(format!("{}.txt", key(voice, t))), t.as_bytes());
        }
    }
    texts.iter().map(|t| load(&cache.join(key(voice, t)), t)).collect()
}

/// The original engine's transcription of every line of `words` (FON only,
/// fast: ~5 000 words a second), as (word, transcription). Some words crash
/// the engine; the helper writes each line as it goes, so the word after the
/// last one written is skipped and the rest is run again.
pub fn transcribe_words(voice: &str, words: &[String]) -> Result<Vec<(String, String)>, String> {
    let dir = oracle_dir().ok_or("oracle/run.sh not found (run from the quoquendo folder)")?;
    let cache = dir.join("cache");
    std::fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    let index: std::collections::HashMap<&str, usize> = words.iter().enumerate().map(|(i, w)| (w.as_str(), i)).collect();
    let mut done = Vec::new();
    let mut start = 0usize;
    while start < words.len() {
        let list = cache.join(format!("words-{voice}.txt"));
        std::fs::write(&list, words[start..].join("\n") + "\n").map_err(|e| e.to_string())?;
        let prefix = cache.join(format!("words-{voice}"));
        let _ = std::fs::remove_file(prefix.with_extension("fonlist"));
        let out = std::process::Command::new(dir.join("run.sh"))
            .arg(voice)
            .arg(&list)
            .arg(&prefix)
            .env("LOQ_FON_ONLY", "1")
            .output()
            .map_err(|e| format!("running the original engine: {e}"))?;
        let text = std::fs::read(prefix.with_extension("fonlist")).unwrap_or_default();
        let pairs: Vec<(String, String)> = String::from_utf8_lossy(&text)
            .lines()
            .filter_map(|l| l.split_once('\t'))
            .map(|(w, t)| (w.to_string(), t.trim().trim_end_matches('-').to_string()))
            .collect();
        let last = pairs.last().and_then(|(w, _)| index.get(w.as_str()).copied());
        done.extend(pairs);
        if String::from_utf8_lossy(&out.stdout).contains("ok") {
            break;
        }
        // crashed: skip the word after the last one written
        let next = last.map(|i| i + 1).unwrap_or(start);
        if next < start {
            return Err(format!("the original engine failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
        }
        eprintln!(
            "the original engine stopped at \"{}\"; skipping it",
            words.get(next).map(String::as_str).unwrap_or("")
        );
        start = next + 1;
    }
    Ok(done)
}

fn load(prefix: &Path, text: &str) -> Result<Reference, String> {
    let read = |ext: &str| std::fs::read(prefix.with_extension(ext)).map_err(|e| format!("{}.{ext}: {e}", prefix.display()));
    let mut r = Reference {
        text: text.to_string(),
        ..Default::default()
    };
    r.transcription = quoquendo::loquendo::latin1(&read("fon")?).trim().to_string();
    for line in read("ev")?.split(|b| *b == b'\n') {
        let line = quoquendo::loquendo::latin1(line);
        let field = |name: &str| line.split(' ').find_map(|f| f.strip_prefix(name)).map(str::to_string);
        if line.starts_with("3 ") {
            if let (Some(p), Some(v)) = (field("phone="), field("value=")) {
                r.phones.push((p, v.parse().unwrap_or(0)));
            }
        } else if line.starts_with("19 ") {
            let hex = field("hex=").unwrap_or_default();
            let bytes: Vec<u8> = (0..hex.len() / 2).filter_map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).ok()).collect();
            if bytes.len() >= 0x1c {
                // "0,<sentence>" at 4..12, half-phone count at 0x18
                let id: String = bytes[4..12].iter().take_while(|b| **b != 0).map(|b| *b as char).collect();
                let unit = id.split(',').nth(1).and_then(|s| s.parse().ok());
                let halves = u32::from_le_bytes([bytes[0x18], bytes[0x19], bytes[0x1a], bytes[0x1b]]);
                if let Some(unit) = unit {
                    r.pieces.push((unit, halves));
                }
            }
        }
    }
    let wav = read("wav")?;
    let data = wav.windows(4).position(|w| w == b"data").map(|p| p + 8).unwrap_or(44).min(wav.len());
    r.audio = wav[data..].chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect();
    Ok(r)
}

/// Where in recorded sentence `u` the half-phones `want` come from, as
/// (first half position, count) stretches; half position = 2·phone + side.
/// One unit-selection event may take two separate stretches of the same
/// sentence (e.g. "…dal binario" then "uno" further on), so the fewest,
/// longest stretches are chosen, preferring ones after the previous.
fn segments(u: &quoquendo::loquendo::Unit, want: &[&str]) -> Option<Vec<(usize, usize)>> {
    let half = |pos: usize| u.phones.get(pos / 2).filter(|p| p.timed).map(|p| p.sym.as_str());
    let mut out = Vec::new();
    let (mut k, mut prev_end) = (0usize, 0usize);
    while k < want.len() {
        let mut best: Option<(usize, bool, usize)> = None;
        for pos in 2..2 * u.phones.len() {
            if half(pos) != Some(want[k]) {
                continue;
            }
            let mut len = 0;
            while k + len < want.len() && half(pos + len) == Some(want[k + len]) {
                len += 1;
            }
            let key = (len, pos >= prev_end);
            if best.map(|b| key > (b.0, b.1)).unwrap_or(true) {
                best = Some((len, pos >= prev_end, pos));
            }
        }
        let (len, _, pos) = best?;
        out.push((pos, len));
        k += len;
        prev_end = pos + len;
    }
    Some(out)
}

impl Reference {
    /// The engine's FON stage as phrases: its phones and pauses, with word
    /// starts taken from its transcription and tunes from the text's marks.
    pub fn target_phrases(&self) -> Vec<TargetPhrase> {
        let words: Vec<Target> = quoquendo::synth::parse_transcription(&self.transcription)
            .into_iter()
            .flat_map(|p| p.phones)
            .collect();
        let mut moods = self.text.chars().filter_map(|c| match c {
            '?' => Some(Some('I')),
            '!' => Some(Some('E')),
            '.' => Some(None),
            _ => None,
        });
        let mut out = Vec::new();
        let mut cur: Vec<Target> = Vec::new();
        let mut w = 0;
        for (sym, ms) in &self.phones {
            if is_pause(sym) {
                if !cur.is_empty() {
                    let full_stop = sym == "@";
                    let mood = if full_stop { moods.next().flatten() } else { None };
                    out.push(TargetPhrase {
                        phones: std::mem::take(&mut cur),
                        pause_ms: *ms,
                        full_stop,
                        mood,
                    });
                }
                continue;
            }
            let word_start = match words.get(w) {
                Some(t) if t.sym == *sym => t.word_start,
                _ => cur.is_empty(),
            };
            w += 1;
            cur.push(Target { sym: sym.clone(), word_start });
        }
        if !cur.is_empty() {
            out.push(TargetPhrase {
                phones: cur,
                pause_ms: 0,
                full_stop: true,
                mood: None,
            });
        }
        out
    }

    /// The engine's CAT stage as phrases of pieces, each found in the voice's
    /// index by matching its half-phones against the phones said.
    pub fn phrases(&self, db: &VoiceDatabase) -> Result<Vec<Phrase>, String> {
        // the half-phones said, in order, and where each phrase ends
        let mut halves: Vec<&str> = Vec::new();
        let mut breaks: Vec<(usize, u32)> = Vec::new();
        for (sym, ms) in &self.phones {
            if is_pause(sym) {
                breaks.push((halves.len(), *ms));
            } else {
                halves.push(sym);
                halves.push(sym);
            }
        }
        let mut out: Vec<Phrase> = vec![Phrase::default()];
        let mut h = 0usize;
        let mut b = 0usize;
        for &(unit, n) in &self.pieces {
            while b < breaks.len() && breaks[b].0 <= h {
                let cur = out.last_mut().unwrap();
                cur.pause_ms += breaks[b].1;
                if !cur.pieces.is_empty() {
                    out.push(Phrase::default());
                }
                b += 1;
            }
            let n = n as usize;
            let want = &halves[h.min(halves.len())..(h + n).min(halves.len())];
            let u = db.units.get(unit as usize).ok_or(format!("sentence {unit} is not in this voice"))?;
            let segs = segments(u, want).ok_or(format!("piece from sentence {unit} not found in it"))?;
            let phrase = out.last_mut().unwrap();
            let mut k = 0;
            for (pos, len) in segs {
                let (first, last_pos) = (&u.phones[pos / 2], pos + len - 1);
                let last = &u.phones[last_pos / 2];
                // a phone starts at every left half; the first half may be a right one
                let covers: Vec<String> = (0..len).filter(|j| (pos + j) % 2 == 0 || *j == 0).map(|j| want[k + j].to_string()).collect();
                phrase.phones.extend((0..len).filter(|j| (pos + j) % 2 == 0).map(|j| want[k + j].to_string()));
                phrase.pieces.push(Piece {
                    covers,
                    unit,
                    from: if pos % 2 == 0 { first.start } else { first.mid },
                    to: if last_pos % 2 == 0 { last.mid } else { last.end },
                    pitch: if pos % 2 == 0 { first.pitch_l } else { first.pitch_r },
                    hard: false,
                });
                k += len;
            }
            h += n;
        }
        if let Some(last) = out.last_mut() {
            for &(_, ms) in &breaks[b..] {
                last.pause_ms += ms;
            }
        }
        out.retain(|p| !p.pieces.is_empty() || p.pause_ms > 0);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_keys_differ_by_voice_and_text() {
        assert_ne!(key("Roberto", "ciao"), key("Katrin", "ciao"));
        assert_ne!(key("Roberto", "ciao"), key("Roberto", "ciao."));
        assert_eq!(key("Roberto", "ciao"), key("Roberto", "ciao"));
    }

    #[test]
    fn engine_phones_become_phrases_with_word_starts() {
        let r = Reference {
            text: "il treno, ok.".into(),
            transcription: "i l-tt r `E n o-o k-".into(),
            phones: [
                ("i", 90),
                ("l", 40),
                ("tt", 60),
                ("r", 60),
                ("`E", 80),
                ("n", 50),
                ("o", 70),
                ("#", 120),
                ("o", 50),
                ("k", 60),
                ("@", 500),
            ]
            .iter()
            .map(|(s, m)| (s.to_string(), *m))
            .collect(),
            ..Default::default()
        };
        let p = r.target_phrases();
        assert_eq!(p.len(), 2);
        assert_eq!((p[0].pause_ms, p[0].full_stop, p[1].full_stop), (120, false, true));
        assert!(p[0].phones[2].word_start && !p[0].phones[3].word_start);
        assert!(p[1].phones[0].word_start);
    }
}
