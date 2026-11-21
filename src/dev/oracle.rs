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
    /// every piece it chose: (database, recorded sentence, number of
    /// half-phones), from its `<db>,<sentence>` id
    pub pieces: Vec<(u32, u32, u32)>,
    /// every half-phone SIG played, in order (`LOQ_TRACE_UNITS`): its start
    /// and end in `.sde` ticks and whether it ends a run (algorithm 1)
    pub units: Vec<(u64, u64, bool)>,
    /// SIG: its audio, at `RATE`
    pub audio: Vec<i16>,
    /// every request to its decoders (`LOQ_TRACE_DECODE`): decoder, sample
    /// position, the samples returned
    pub decodes: Vec<(String, usize, Vec<i16>)>,
}

/// The original engine's output rate (Hz).
pub const RATE: u32 = 32000;

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
    // results come with their decoder and unit traces (`.dec`, `.units`);
    // older cached ones without them are run again
    let todo: Vec<&String> = texts.iter().filter(|t| !cache.join(format!("{}.units", key(voice, t))).is_file()).collect();
    if !todo.is_empty() {
        // one batch name per process: two runs at once (two voices, or two
        // checkouts' tools) must not read each other's files
        let name = format!("batch-{}", std::process::id());
        let batch = cache.join(format!("{name}.txt"));
        let lines: Vec<String> = todo.iter().map(|t| t.replace(['\n', '\r'], " ")).collect();
        std::fs::write(&batch, lines.join("\n") + "\n").map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(cache.join(format!("{name}.dec")));
        let out = std::process::Command::new(dir.join("run.sh"))
            .arg(voice)
            .arg(&batch)
            .arg(cache.join(&name))
            .env("LOQ_LINES", "1")
            .env("LOQ_TRACE_DECODE", "1")
            .env("LOQ_TRACE_UNITS", "1")
            .output()
            .map_err(|e| format!("running the original engine: {e}"))?;
        if !String::from_utf8_lossy(&out.stdout).contains("ok") {
            return Err(format!("the original engine failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
        }
        // the traces are one file each for the batch, `U <n>` before each
        // utterance
        let split = |ext: &str, prefix: &str| -> Vec<String> {
            let path = cache.join(format!("{name}.{ext}"));
            let trace = std::fs::read_to_string(&path).unwrap_or_default();
            let _ = std::fs::remove_file(&path);
            let mut per_text: Vec<String> = vec![String::new(); todo.len()];
            let mut cur: Option<usize> = None;
            for line in trace.lines() {
                if let Some(n) = line.strip_prefix("U ") {
                    cur = n.trim().parse().ok().filter(|&n: &usize| n < todo.len());
                } else if let (Some(i), true) = (cur, line.starts_with(prefix)) {
                    per_text[i].push_str(line);
                    per_text[i].push('\n');
                }
            }
            per_text
        };
        let decodes = split("dec", "D ");
        let units = split("units", "M ");
        let _ = std::fs::remove_file(&batch);
        for (i, t) in todo.iter().enumerate() {
            for ext in ["fon", "ev", "wav"] {
                let _ = std::fs::rename(cache.join(format!("{name}-{i}.{ext}")), cache.join(format!("{}.{ext}", key(voice, t))));
            }
            let _ = std::fs::write(cache.join(format!("{}.dec", key(voice, t))), &decodes[i]);
            let _ = std::fs::write(cache.join(format!("{}.units", key(voice, t))), &units[i]);
            let _ = std::fs::write(cache.join(format!("{}.txt", key(voice, t))), t.as_bytes());
        }
    }
    texts.iter().map(|t| load(&cache.join(key(voice, t)), t)).collect()
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
                // "<database>,<sentence>" at 4..12, half-phone count at 0x18
                let id: String = bytes[4..12].iter().take_while(|b| **b != 0).map(|b| *b as char).collect();
                let mut parts = id.split(',').map(|s| s.parse().ok());
                let halves = u32::from_le_bytes([bytes[0x18], bytes[0x19], bytes[0x1a], bytes[0x1b]]);
                if let (Some(Some(db)), Some(Some(unit))) = (parts.next(), parts.next()) {
                    r.pieces.push((db, unit, halves));
                }
            }
        }
    }
    let wav = read("wav")?;
    let data = wav.windows(4).position(|w| w == b"data").map(|p| p + 8).unwrap_or(44).min(wav.len());
    r.audio = wav[data..].chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect();
    // `D <codec> <decoder> <position> <samples> <rc> <cw> <mxcsr> <samples…>`
    for line in std::fs::read_to_string(prefix.with_extension("dec")).unwrap_or_default().lines() {
        let f: Vec<&str> = line.split(' ').collect();
        if let (Some(dec), Some(pos)) = (f.get(2), f.get(3).and_then(|p| p.parse().ok())) {
            r.decodes.push((dec.to_string(), pos, f.iter().skip(8).filter_map(|x| x.parse().ok()).collect()));
        }
    }
    // `M t0=<ticks> t1=<ticks> f0=… voc=… algo=<0|1> …`
    for line in std::fs::read_to_string(prefix.with_extension("units")).unwrap_or_default().lines() {
        let kv: std::collections::HashMap<&str, &str> = line.split(' ').filter_map(|f| f.split_once('=')).collect();
        if let (Some(t0), Some(t1), Some(algo)) = (kv.get("t0"), kv.get("t1"), kv.get("algo")) {
            r.units.push((t0.parse().unwrap_or(0), t1.parse().unwrap_or(0), *algo == "1"));
        }
    }
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
        // Greek asks with `;` (LTTS7Greek.dll's LesSetModalitaFrase
        // 0x10008920 gives it type 2, as `?`)
        let greek = self.text.chars().any(|c| ('\u{370}'..='\u{3ff}').contains(&c));
        let chars: Vec<char> = self.text.chars().collect();
        let mut moods = chars.iter().enumerate().filter_map(|(k, &c)| match c {
            '?' | '\u{37e}' => Some(Some('I')),
            ';' if greek => Some(Some('I')),
            '!' => Some(Some('E')),
            // a dot inside a number or an abbreviation ends no sentence
            '.' if chars.get(k + 1).is_none_or(|n| n.is_whitespace()) => Some(None),
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
                } else if let Some(p) = out.last_mut() {
                    // a phrase of only a pause (a sentence opening with
                    // `,`: `Mo., 14.02.` → @500 #120): its silence follows
                    // the one before
                    p.pause_ms += *ms;
                }
                continue;
            }
            let word_start = match words.get(w) {
                Some(t) if t.sym == *sym => t.word_start,
                _ => cur.is_empty(),
            };
            w += 1;
            cur.push(Target {
                sym: sym.clone(),
                word_start,
                word: None,
            });
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

    /// The engine's CAT stage as phrases of pieces. Each piece's half-phones
    /// are found in the voice's index by their `.sde` ticks when the units
    /// SIG played were traced, else by matching their sounds.
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
        let traced = self.units.len() == self.pieces.iter().map(|p| p.2 as usize).sum::<usize>();
        let mut out: Vec<Phrase> = vec![Phrase::default()];
        let mut h = 0usize;
        let mut b = 0usize;
        for &(dbi, unit, n) in &self.pieces {
            while b < breaks.len() && breaks[b].0 <= h {
                let m = out.len();
                if out[m - 1].pieces.is_empty() && m > 1 {
                    // a pause right after a pause: one silence
                    out[m - 2].pause_ms += breaks[b].1;
                } else {
                    out[m - 1].pause_ms += breaks[b].1;
                    if !out[m - 1].pieces.is_empty() {
                        out.push(Phrase::default());
                    }
                }
                b += 1;
            }
            // `<db>,<sentence>`: the voice's databases follow each other in `units`
            let unit = match (dbi, db.db_start.get(dbi as usize)) {
                (0, _) => unit,
                (_, Some(&first)) => first + unit,
                _ => return Err(format!("piece {dbi},{unit} from a database this voice didn't load")),
            };
            let n = n as usize;
            let want = &halves[h.min(halves.len())..(h + n).min(halves.len())];
            let u = db.units.get(unit as usize).ok_or(format!("sentence {dbi},{unit} is not in this voice"))?;
            let segs = if traced {
                // each half by its ticks (a half of no length shares its
                // start with the next); contiguous halves make one stretch
                let mut segs: Vec<(usize, usize)> = Vec::new();
                for &(t0, t1, _) in &self.units[h..h + n] {
                    let pos = u
                        .phones
                        .iter()
                        .enumerate()
                        .filter(|(_, p)| p.timed)
                        .find_map(|(i, p)| {
                            (p.tick_l == t0 && p.tick_l + p.len_l == t1)
                                .then_some(2 * i)
                                .or((p.tick_r == t0 && p.tick_r + p.len_r == t1).then_some(2 * i + 1))
                        })
                        .ok_or(format!("half at ticks {t0}–{t1} not in sentence {unit}"))?;
                    match segs.last_mut() {
                        Some((p0, len)) if *p0 + *len == pos => *len += 1,
                        _ => segs.push((pos, 1)),
                    }
                }
                segs
            } else {
                segments(u, want).ok_or(format!("piece from sentence {unit} not found in it"))?
            };
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
                    first_half: pos as u32,
                    halves: len as u32,
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

/// The benchmark for a voice's MotherTongue: `oracle/bench-<code>.txt`.
pub fn bench_for(mother_tongue: &str) -> Option<&'static str> {
    Some(match mother_tongue {
        "Italian" => "it",
        "EnglishUs" => "en",
        "EnglishGb" => "en-gb",
        "EnglishAu" => "en-au",
        "SpanishEs" => "es",
        t if t.starts_with("Spanish") => "es-am",
        "French" => "fr",
        "German" => "de",
        "PortuguesePt" => "pt-pt",
        "PortugueseBr" => "pt-br",
        "Greek" => "el",
        "Swedish" => "sv",
        "Danish" => "da",
        "Dutch" => "nl",
        "Polish" => "pl",
        "Russian" => "ru",
        "Catalan" => "ca",
        "Valencian" => "va",
        "Galician" => "gl",
        _ => return None,
    })
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
