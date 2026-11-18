//! Reading the Loquendo TTS 7 data files.
//!
//! Three formats matter and all three are described in MECHANICS.md:
//!  * `.vde` / `.vcf` / `.lde`  plain INI-style descriptor files
//!  * `.sde`  the voice's unit inventory, a text file with every byte >= 0x80
//!    moved up by 0x80 (so `#` is 0xA3, space is 0xA0, `@` is 0xC0)
//!  * `<voice>.<rate>.<codec>.<count>.<n>.bin`  the recorded unit database

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const SDE_CHAR_OFFSET: u8 = 0x80;

/// Audio rate written into the voice file name, e.g. `Roberto.32000....bin`.
pub const VOICE_SAMPLE_RATE: u32 = 32_000;

/// `.sde` times are in 10 µs steps (they add up to the audio file's length
/// at 100 000 per second), see MECHANICS.md §3.
pub const SDE_TICKS_PER_SECOND: u64 = 100_000;

/// `.sde` ticks to samples exactly as the engine does it, in integers
/// (LoqTTS7.dll 0x1001e3a0, the pitch-mark finder):
/// `(t / 100000) * rate + ((t % 100000) * (rate / 10)) / 10000`.
pub fn ticks_to_samples(t: u64, rate: u64) -> u32 {
    ((t / SDE_TICKS_PER_SECOND) * rate + ((t % SDE_TICKS_PER_SECOND) * (rate / 10)) / 10_000) as u32
}

/// One phone of a recorded sentence. Every phone but the two edge silences
/// has an `L` (left half) and an `R` (right half) record: their lengths and
/// the pitch in Hz of each half.
#[derive(Clone, Debug, Default)]
pub struct Phone {
    pub sym: String,
    /// first phone of a word (the `.sde` joins words with `-`)
    pub word_start: bool,
    /// false for the two edge silences, which have no records
    pub timed: bool,
    /// sample positions in the audio file: start, the L/R boundary, end
    pub start: u32,
    pub mid: u32,
    pub end: u32,
    pub pitch_l: u16,
    pub pitch_r: u16,
    /// the L and R records as the engine keeps them: start and length in
    /// `.sde` ticks (10 µs)
    pub tick_l: u64,
    pub tick_r: u64,
    pub len_l: u64,
    pub len_r: u64,
    /// which word this phone belongs to, by its phones (`word_key`)
    pub word: u64,
}

/// A key for a word made of these phone symbols (FNV-1a over them).
pub fn word_key<'a>(syms: impl Iterator<Item = &'a str>) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for s in syms {
        for b in s.bytes().chain([b' ']) {
            h = (h ^ b as u64).wrapping_mul(0x100000001b3);
        }
    }
    h
}

/// Fill `word` for every phone, splitting at word starts.
pub fn mark_words(phones: &mut [Phone]) {
    let mut i = 0;
    while i < phones.len() {
        let mut j = i + 1;
        while j < phones.len() && !phones[j].word_start {
            j += 1;
        }
        let key = word_key(phones[i..j].iter().map(|p| p.sym.as_str()));
        for p in &mut phones[i..j] {
            p.word = key;
        }
        i = j;
    }
}

/// One recorded sentence (`U=` line) and its phones.
#[derive(Clone, Debug, Default)]
pub struct Unit {
    /// the `U=` tokens as written (`o-i` joins the last phone of a word to
    /// the first of the next)
    pub symbols: Vec<String>,
    pub phones: Vec<Phone>,
    /// `S=<c>` sentence type: `I` question, `E` exclamation (Gilded
    /// phrase lists use `e`, `x`, `f`)
    pub mood: Option<char>,
}

/// A (unit, phone) pair: where one recorded phone lives.
pub type At = (u32, u16);

#[derive(Default)]
pub struct VoiceDatabase {
    /// Header line of the `.sde`, e.g. `14,4`: the voice's licence bit
    /// (byte 14, mask 4; LoqTTS7.dll 0x10002a00), unused for speech.
    pub format: String,
    pub codec_version: String,
    /// `N=<sentences>,<phones including both edge silences>`
    pub declared_units: usize,
    pub declared_instances: usize,
    pub units: Vec<Unit>,
    pub size_bytes: u64,
    pub path: PathBuf,
    pub audio_path: Option<PathBuf>,
    pub audio_size: u64,
    /// sample rate from the audio file name (`<voice>.32000.<codec>...`)
    pub audio_rate_hz: u32,
    pub audio: Option<crate::codec::AudioBank>,
    pub audio_error: Option<String>,
    /// how many recorded phones of each symbol there are
    pub phone_counts: HashMap<String, usize>,
    /// every timed phone, by relax level (0 exact, 1 without stress, 2
    /// without stress and length), keyed by symbol
    pub singles: [HashMap<String, Vec<At>>; 3],
    /// median recorded pitch of the voice (Hz), what pitch targets scale by
    pub median_pitch: f32,
    /// mean recorded duration of each phone symbol (samples)
    pub mean_duration: HashMap<String, f32>,
    /// the original's phone tables for this voice, when dumped (synth::PhoneTables)
    pub tables: Option<crate::synth::PhoneTables>,
}

/// Drop stress (level ≥ 1) and length (level 2) marks from a symbol.
pub fn relax(sym: &str, level: usize) -> &str {
    let mut s = sym;
    if level >= 1 {
        s = s.trim_start_matches('`');
    }
    if level >= 2 {
        s = s.trim_end_matches(':');
    }
    s
}

impl VoiceDatabase {
    pub fn audio_rate(&self) -> u32 {
        if self.audio_rate_hz > 0 {
            self.audio_rate_hz
        } else {
            VOICE_SAMPLE_RATE
        }
    }

    /// Recorded (timed) phones in the whole inventory.
    pub fn instance_count(&self) -> usize {
        self.phone_counts.values().sum()
    }

    /// The voice's phone alphabet, read back out of its recordings.
    pub fn alphabet(&self) -> Vec<String> {
        let mut v: Vec<String> = self.phone_counts.keys().cloned().collect();
        v.sort();
        v
    }

    pub fn knows_symbol(&self, symbol: &str) -> bool {
        self.phone_counts.contains_key(symbol)
    }

    /// Longest recorded sentence, in phones.
    pub fn longest_unit(&self) -> usize {
        self.units.iter().map(|u| u.phones.len()).max().unwrap_or(0)
    }

    fn build_index(&mut self) {
        for u in &mut self.units {
            mark_words(&mut u.phones);
        }
        let mut sums: HashMap<String, (f64, u32)> = HashMap::new();
        for p in self.units.iter().flat_map(|u| u.phones.iter()).filter(|p| p.timed) {
            let e = sums.entry(p.sym.clone()).or_default();
            e.0 += (p.end - p.start) as f64;
            e.1 += 1;
        }
        self.mean_duration = sums.into_iter().map(|(k, (s, n))| (k, (s / n as f64) as f32)).collect();
        let mut pitches: Vec<u16> = self
            .units
            .iter()
            .flat_map(|u| u.phones.iter())
            .flat_map(|p| [p.pitch_l, p.pitch_r])
            .filter(|p| *p > 0)
            .collect();
        pitches.sort_unstable();
        self.median_pitch = pitches.get(pitches.len() / 2).copied().unwrap_or(100) as f32;
        for (ui, u) in self.units.iter().enumerate() {
            for (pi, p) in u.phones.iter().enumerate() {
                if !p.timed {
                    continue;
                }
                *self.phone_counts.entry(p.sym.clone()).or_insert(0) += 1;
                for level in 0..3 {
                    let key = relax(&p.sym, level).to_string();
                    self.singles[level].entry(key).or_default().push((ui as u32, pi as u16));
                }
            }
        }
    }
}

pub fn decode_sde_line(line: &str) -> String {
    line.chars()
        .map(|c| {
            let b = c as u32;
            if (0x80..0x100).contains(&b) {
                char::from_u32(b - SDE_CHAR_OFFSET as u32).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

/// `.sde` numbers: big-endian base 93, digits `"` (0x22) to `~` (0x7E).
pub fn base93(token: &str) -> Option<u64> {
    if token.is_empty() {
        return None;
    }
    let mut v: u64 = 0;
    for c in token.bytes() {
        if !(0x22..=0x7e).contains(&c) {
            return None;
        }
        v = v * 93 + (c - 0x22) as u64;
    }
    Some(v)
}

/// The voice index; sample positions are at `rate`, the recordings' rate
/// (32000 or 44000, from the `.bin` name), as the original's are.
pub fn parse_sde(text: &str, rate: u32) -> VoiceDatabase {
    let mut db = VoiceDatabase {
        audio_rate_hz: rate,
        ..Default::default()
    };
    let rate = rate as u64;
    let to_sample = |ticks: u64| ticks_to_samples(ticks, rate);
    // per sentence: the next phone to fill, the clock in ticks, and whether
    // the next record is a left half
    let mut next_phone = 1usize;
    let mut clock: u64 = 0;
    let mut half_start: u64 = 0;
    for raw in text.lines() {
        let line = decode_sde_line(raw.trim_end_matches('\r'));
        if line.is_empty() {
            continue;
        }
        if db.format.is_empty() {
            db.format = line.clone();
            continue;
        }
        if let Some(rest) = line.strip_prefix("LTTSComponentVersion=") {
            db.codec_version = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("N=") {
            let mut it = rest.split(',');
            db.declared_units = it.next().and_then(|v| v.trim().parse().ok()).unwrap_or(0);
            db.declared_instances = it.next().and_then(|v| v.trim().parse().ok()).unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("U=") {
            let symbols: Vec<String> = rest.split(' ').map(|s| s.to_string()).collect();
            let mut phones = Vec::new();
            for tok in &symbols {
                for (i, part) in tok.split('-').enumerate() {
                    phones.push(Phone {
                        sym: part.to_string(),
                        word_start: i > 0 || phones.len() == 1,
                        ..Default::default()
                    });
                }
            }
            db.units.push(Unit { symbols, phones, mood: None });
            next_phone = 1;
        } else if let Some(m) = line.strip_prefix("S=") {
            if let Some(u) = db.units.last_mut() {
                u.mood = m.chars().next();
            }
        } else if line.starts_with('L') || line.starts_with('R') {
            // `L<index> [!<start>] <length> <pitch>`
            let fields: Vec<&str> = line[1..].split(' ').skip(1).collect();
            let Some(u) = db.units.last_mut() else { continue };
            if fields.len() < 2 || next_phone + 1 >= u.phones.len() {
                continue;
            }
            if let Some(abs) = fields[0].strip_prefix('!') {
                clock = base93(abs).unwrap_or(clock);
            }
            let len = base93(fields[fields.len() - 2]).unwrap_or(0);
            let pitch = base93(fields[fields.len() - 1]).unwrap_or(0) as u16;
            let p = &mut u.phones[next_phone];
            if line.starts_with('L') {
                half_start = clock;
                p.start = to_sample(clock);
                p.pitch_l = pitch;
                p.tick_l = clock;
                p.len_l = len;
                clock += len;
                p.mid = to_sample(clock);
            } else {
                p.tick_r = clock;
                p.len_r = len;
                clock += len;
                p.end = to_sample(clock);
                p.pitch_r = pitch;
                p.timed = clock > half_start;
                next_phone += 1;
            }
        }
        // `P=` (paralinguistic clip lists) are not used yet
    }
    // the edge silences sit where their neighbour starts or ends
    for u in &mut db.units {
        let n = u.phones.len();
        if n >= 3 {
            let (s, e) = (u.phones[1].start, u.phones[n - 2].end);
            for (i, at) in [(0, s), (n - 1, e)] {
                u.phones[i].start = at;
                u.phones[i].mid = at;
                u.phones[i].end = at;
            }
        }
    }
    db.build_index();
    db
}

/// Read a byte string as Latin-1, one char per byte.
///
/// Every byte of an `.sde` is ≥ 0xA0 and the descriptors carry Latin-1
/// (`heiße`), so a UTF-8 read with lossy replacement would destroy the
/// characters the `.sde` shift is defined over. Latin-1 maps every byte
/// 1:1 and keeps those bytes recoverable.
pub fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|b| *b as char).collect()
}

/// `<voice>.<rate>.<codec>.<n>.<n>.bin` — the second field is the sample rate.
pub fn rate_from_bin_name(path: &Path) -> Option<u32> {
    let name = path.file_name()?.to_str()?;
    let parts: Vec<&str> = name.split('.').collect();
    parts.get(1).and_then(|r| r.parse::<u32>().ok())
}

/// A voice's extra material: the Gilded phrase and paralinguistic inventories
/// that some voices ship beside the main one.
#[derive(Clone, Debug, Default)]
pub struct VoiceExtras {
    /// (name, unit count, instance count)
    pub sets: Vec<(String, usize, usize)>,
}

pub fn load_extras(voice_dir: &Path) -> VoiceExtras {
    let mut extras = VoiceExtras::default();
    if let Ok(entries) = fs::read_dir(voice_dir) {
        let mut paths: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension().and_then(|e| e.to_str()) == Some("sde")
                    && p.file_stem().and_then(|s| s.to_str()).map(|s| !s.eq_ignore_ascii_case("")).unwrap_or(false)
            })
            .filter(|p| {
                // skip the voice's own main inventory, which the app already has
                !p.file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.chars().all(|c| c.is_alphanumeric()))
                    .unwrap_or(false)
                    || p.file_name().and_then(|n| n.to_str()).map(|n| n.contains("Gilded")).unwrap_or(false)
            })
            .collect();
        paths.sort();
        for p in paths {
            let Ok(bytes) = fs::read(&p) else { continue };
            let text = latin1(&bytes);
            let db = parse_sde(&text, VOICE_SAMPLE_RATE);
            let name = p.file_stem().and_then(|s| s.to_str()).unwrap_or("extras").to_string();
            extras.sets.push((name, db.units.len(), db.instance_count()));
        }
    }
    extras
}

/// A tiny INI reader for the `.vde` / `.vcf` / `.lde` descriptor files.
pub fn parse_ini(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for raw in text.lines() {
        let line = raw.trim().trim_end_matches('\r');
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim().trim_matches('"').to_string();
            let val = v.trim().trim_matches('"').to_string();
            out.insert(key, val);
        }
    }
    out
}

/// Turn a string read as Latin-1 back into proper UTF-8, for showing to people
/// (`heiße` rather than `hei§e`). Unknown bytes become U+FFFD.
pub fn from_latin1(s: &str) -> String {
    let bytes: Vec<u8> = s.chars().map(|c| c as u32 as u8).collect();
    String::from_utf8_lossy(&bytes).to_string()
}

pub fn read_ini(path: &Path) -> HashMap<String, String> {
    fs::read(path).map(|b| parse_ini(&latin1(&b))).unwrap_or_default()
}

#[derive(Clone, Debug, Default)]
pub struct VoiceDescriptor {
    pub name: String,
    pub description: String,
    pub library: String,
    pub base_speed: i32,
    pub base_pitch: i32,
    pub gender: String,
    pub age: String,
    pub mother_tongue: String,
    pub mother_tongue_aliases: String,
    pub demo_sentence: String,
    /// the installation's `data` folder (empty for a voice loaded from bytes)
    pub data_dir: PathBuf,
    pub voice_path: PathBuf,
    pub bin_name: String,
    pub audio_rate: u32,
    /// every `VocalDBName[i]` the voice declares: `<voice>` first, then the
    /// Gilded phrase and paralinguistic inventories some voices ship
    pub vocal_dbs: Vec<String>,
    /// `GildedTTS = yes` in the `.vde`
    pub gilded: bool,
}

impl VoiceDescriptor {
    /// The language to run the front end for: the voice's own `MotherTongue`.
    pub fn language(&self) -> crate::g2p::Language {
        crate::g2p::Language::from_voice(&self.mother_tongue, &self.mother_tongue_aliases)
    }
    /// Fill from a `.vde` plus the `.vcf` that points at it.
    pub fn from_files(vcf: &Path, vde: &Path) -> VoiceDescriptor {
        let mut d = VoiceDescriptor::from_ini(&read_ini(vcf), &read_ini(vde));
        if d.name.is_empty() {
            d.name = vde.file_stem().and_then(|s| s.to_str()).unwrap_or("voice").to_string();
        }
        d.voice_path = vde.parent().unwrap_or(Path::new(".")).to_path_buf();
        if let Ok(entries) = fs::read_dir(&d.voice_path) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name.ends_with(".bin") && name.contains(".LTTS7Codec") && name.starts_with(&format!("{}.", d.name)) {
                    d.set_bin_name(&name);
                    break;
                }
            }
        }
        d
    }

    /// Fill from the `.vcf` and `.vde` settings (`parse_ini`).
    pub fn from_ini(cv: &HashMap<String, String>, dv: &HashMap<String, String>) -> VoiceDescriptor {
        let mut d = VoiceDescriptor {
            audio_rate: VOICE_SAMPLE_RATE,
            ..Default::default()
        };
        d.name = dv
            .get("BaseVoice")
            .cloned()
            .or_else(|| cv.get("BaseVoice").cloned())
            // `"Library" = "LTTS7Katrin, Katrin"`: the voice's name is the second part
            .or_else(|| dv.get("Library").and_then(|l| l.split(',').nth(1)).map(|n| n.trim().to_string()))
            .unwrap_or_default();
        d.description = cv
            .get("Description")
            .cloned()
            .or_else(|| dv.get("BaseDescription").cloned())
            .unwrap_or_default();
        d.library = dv.get("Library").cloned().unwrap_or_default();
        d.base_speed = dv.get("BaseSpeed").and_then(|v| v.parse().ok()).unwrap_or(0);
        d.base_pitch = dv.get("BasePitch").and_then(|v| v.parse().ok()).unwrap_or(0);
        d.gender = dv.get("Gender").cloned().unwrap_or_default();
        d.age = dv.get("Age").cloned().unwrap_or_default();
        d.mother_tongue = dv.get("MotherTongue").cloned().unwrap_or_default();
        d.mother_tongue_aliases = dv.get("MotherTongueAliases").cloned().unwrap_or_default();
        d.demo_sentence = dv.get("DemoSentence").cloned().unwrap_or_default();
        d.gilded = dv.get("GildedTTS").map(|v| v.eq_ignore_ascii_case("yes")).unwrap_or(false);
        // every VocalDBName[i], in the order the configurator lists them
        for i in 0..8 {
            if let Some(name) = cv.get(&format!("VocalDBName[{i}]")) {
                if !name.trim().is_empty() {
                    d.vocal_dbs.push(name.trim().to_string());
                }
            }
        }
        if d.vocal_dbs.is_empty() && !d.name.is_empty() {
            d.vocal_dbs.push(d.name.clone());
        }
        d
    }

    /// The recordings file's name, and the sample rate it carries
    /// (`<voice>.<rate>.<codec>.<count>.<n>.bin`).
    pub fn set_bin_name(&mut self, name: &str) {
        self.bin_name = name.to_string();
        if let Some(r) = name.split('.').nth(1).and_then(|r| r.parse::<u32>().ok()) {
            self.audio_rate = r;
        }
    }
}

/// Find every voice installed under an LTTS7 `data` folder.
pub fn discover_voices(data_dir: &Path) -> Vec<VoiceDescriptor> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(data_dir) {
        let mut vcfs: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("vcf"))
            .collect();
        vcfs.sort();
        for vcf in vcfs {
            let cv = read_ini(&vcf);
            let base = cv.get("BaseVoice").cloned().unwrap_or_default();
            let vpath = cv.get("VoicePath").cloned().unwrap_or_default();
            let mut vde = data_dir.join(&vpath);
            vde.push(format!("{}.vde", base));
            if !vde.is_file() {
                vde = vcf.with_extension("vde");
            }
            if vde.is_file() {
                let mut d = VoiceDescriptor::from_files(&vcf, &vde);
                d.data_dir = data_dir.to_path_buf();
                out.push(d);
            }
        }
    }
    out
}

/// Locate the `data` folder that holds `*.vcf` / `voices/` for a path the
/// person gives us: the LTTS7 root, its `data` folder, or a voice folder.
pub fn resolve_data_dir(path: &Path) -> Option<PathBuf> {
    let mut candidates = vec![path.to_path_buf(), path.join("data"), path.join("LTTS7/data"), path.join("Loquendo/LTTS7/data")];
    if let Some(parent) = path.parent() {
        candidates.push(parent.to_path_buf());
        candidates.push(parent.join("data"));
    }
    candidates.into_iter().find(|c| c.join("voices").is_dir())
}

/// The same resolution starting from any path the person typed.
pub fn resolve_data_dir_from(path: &Path) -> Option<PathBuf> {
    resolve_data_dir(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_shifted_characters() {
        // 0xA3, 0xA0, 0xC0, 0xAE are '#', ' ', '@', '.' in the .sde encoding;
        // note 0xA0 is SPACE, which is why the high-bit shift was inferred
        let raw = "\u{a3}\u{a0}\u{c0}\u{ae}";
        assert_eq!(decode_sde_line(raw), "# @.");
        // and a plain '#' (ANSI) is written 0xA3
        assert_eq!(decode_sde_line("\u{a3}"), "#");
    }

    #[test]
    fn ticks_convert_like_the_engine() {
        assert_eq!(ticks_to_samples(100_000, 32_000), 32_000);
        // 671271794 ticks: Roberto's last phone end
        assert_eq!(ticks_to_samples(671_271_794, 32_000), 214_806_974);
        assert_eq!(ticks_to_samples(3, 32_000), 0);
        assert_eq!(ticks_to_samples(4, 32_000), 1);
    }

    #[test]
    fn base93_numbers() {
        assert_eq!(base93("\""), Some(0));
        assert_eq!(base93("~"), Some(92));
        assert_eq!(base93("#\""), Some(93));
        assert_eq!(base93("!x"), None);
    }

    #[test]
    fn reads_a_tiny_inventory() {
        // "# a-b @": edges # and @ have no records, a and b have an L/R pair;
        // "!" marks a's absolute start
        let text = "14,4\nLTTSComponentVersion=7.0.0\nN=1,4\nU=# a-b @\nS=I\nL# !,, #- n\nR# #- n\nL$ #- p\nR$ #- p\n";
        let db = parse_sde(text, VOICE_SAMPLE_RATE);
        assert_eq!(db.declared_units, 1);
        let u = &db.units[0];
        assert_eq!(u.mood, Some('I'));
        let syms: Vec<&str> = u.phones.iter().map(|p| p.sym.as_str()).collect();
        assert_eq!(syms, ["#", "a", "b", "@"]);
        assert!(u.phones[1].word_start && u.phones[2].word_start);
        assert!(u.phones[1].timed && u.phones[2].timed && !u.phones[0].timed);
        // "#-" = 1*93 + 11 = 104 ticks = 33 samples at 32 kHz
        let start = (base93(",,").unwrap() * 32 / 100) as u32;
        assert_eq!(u.phones[1].start, start);
        assert_eq!(u.phones[1].mid, ((base93(",,").unwrap() + 104) * 32 / 100) as u32);
        assert_eq!(u.phones[2].start, u.phones[1].end);
        assert_eq!(u.phones[1].pitch_l, base93("n").unwrap() as u16);
        assert_eq!(db.singles[0]["a"], vec![(0, 1)]);
        assert_eq!(u.phones[3].start, u.phones[2].end);
    }

    /// Roberto's recordings hold 335637 frames (bin header word 6); the last
    /// sentence of its index must end right there, which pins the base-93
    /// numbers and the 10 µs time unit to the real audio.
    #[test]
    fn roberto_index_ends_where_its_recordings_end() {
        const ROBERTO_FRAMES: usize = 335637;
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../oracle/prefix/drive_c/Program Files (x86)/Loquendo/LTTS7/data");
        let Ok(voice) = crate::Voice::find(&data, "Roberto") else { return };
        let db = voice.db;
        let bank = db.audio.as_ref().expect("Roberto's recordings open");
        assert_eq!(bank.frames, ROBERTO_FRAMES);
        let last = db.units.last().unwrap();
        let end = last.phones[last.phones.len() - 2].end as usize;
        // 706 samples (22 ms) of trailing silence remain after the last phone
        assert!(ROBERTO_FRAMES * 640 - end < 2 * 640, "index ends {} samples early", ROBERTO_FRAMES * 640 - end);
    }
}
