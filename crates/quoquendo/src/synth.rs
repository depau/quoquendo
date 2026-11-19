//! The back half of the pipeline: picking pieces of the voice's recordings
//! (Loquendo's CAT stage) and joining their audio (its SIG stage).
//!
//! Stages hand each other plain data, so each can be swapped:
//!   FON  -> `Vec<TargetPhrase>`  the sounds to say, phrase by phrase
//!   CAT  -> `Vec<Phrase>`        which stretches of which recordings to play
//!   SIG  -> `Speech`             the audio
//! `TargetPhrase` also has a text form (`format_transcription` /
//! `parse_transcription`), the notation the original engine uses for its
//! phonetic transcriptions, so any program can supply the FON stage.
//!
//! The builtin CAT picks, for every pair of neighbouring sounds, a place in
//! the recordings where they were spoken together, preferring matching
//! surroundings and unbroken runs (weights are mine, MECHANICS.md §6). The
//! SIG stage reproduces the original engine's joining (MECHANICS.md §7).

use crate::g2p::Token;
use crate::loquendo::{relax, VoiceDatabase};

/// The original refuses more than 0x800 half-phones ("demiphones") in one
/// phrase (LoqTTS7.dll 0x100472f0, "Too many demiphones in this phrase");
/// longer phrases are split so each part stays within it.
pub const MAX_DEMIPHONES_PER_PHRASE: usize = 0x800;

// --- joining ---
/// Smoothed joining only (estimated): cross-fade and slide, 5 ms each.
const FADE: usize = 160;
const SLIDE: usize = 160;

/// How pieces are joined (the SIG stage).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JoinStyle {
    /// Like the original engine: hard cuts, no fades (measured).
    Original,
    /// Waveform-aligned 5 ms cross-fades: softer joins (my own).
    Smooth,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub volume: f32,
    pub join: JoinStyle,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            volume: 1.0,
            join: JoinStyle::Original,
        }
    }
}

/// One stretch of a recording that goes into the output unchanged.
#[derive(Clone, Debug)]
pub struct Piece {
    /// the wanted sounds it supplies (halves at either end)
    pub covers: Vec<String>,
    pub unit: u32,
    pub from: u32,
    pub to: u32,
    /// its half-phones in the recorded sentence: the first (2·phone + side,
    /// side 0 left, 1 right) and how many; some halves last no time at all,
    /// so `from`..`to` alone doesn't say which
    pub first_half: u32,
    pub halves: u32,
    pub pitch: u16,
    /// joined to the previous piece at a phone edge, not mid-phone
    pub hard: bool,
}

impl Piece {
    /// Its half-phones: (phone index in the sentence, side).
    pub fn half_phones(&self) -> impl Iterator<Item = (usize, u8)> {
        (self.first_half..self.first_half + self.halves).map(|h| ((h / 2) as usize, (h % 2) as u8))
    }
}

#[derive(Clone, Debug, Default)]
pub struct Phrase {
    pub phones: Vec<String>,
    pub pieces: Vec<Piece>,
    pub pause_ms: u32,
    /// `I` question, `E` exclamation (the `.sde` `S=` letters)
    pub mood: Option<char>,
    /// sounds the voice has never recorded at all
    pub missing: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Speech {
    pub samples: Vec<f32>,
    pub rate: u32,
}

impl Speech {
    pub fn duration_s(&self) -> f32 {
        self.samples.len() as f32 / self.rate as f32
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    pub sym: String,
    pub word_start: bool,
}

/// What the FON stage hands on: the sounds of one phrase and what follows it.
#[derive(Clone, Debug, PartialEq)]
pub struct TargetPhrase {
    pub phones: Vec<Target>,
    pub pause_ms: u32,
    /// ends a sentence (`@`) rather than a phrase inside it (`#`)
    pub full_stop: bool,
    pub mood: Option<char>,
}

/// Group the builtin front end's words into phrases at every real pause.
pub fn from_tokens(tokens: &[Token]) -> Vec<TargetPhrase> {
    let mut out = Vec::new();
    let mut cur: Vec<Target> = Vec::new();
    for t in tokens {
        match t {
            Token::Word { symbols, .. } => {
                for (i, s) in symbols.iter().enumerate() {
                    cur.push(Target {
                        sym: s.clone(),
                        word_start: i == 0,
                    });
                }
            }
            Token::Pause { ms, reason } if reason != "word gap" => {
                if !cur.is_empty() {
                    let mood = match reason.as_str() {
                        "question" => Some('I'),
                        "exclamation" => Some('E'),
                        _ => None,
                    };
                    out.push(TargetPhrase {
                        phones: std::mem::take(&mut cur),
                        pause_ms: *ms,
                        full_stop: reason != "phrase break",
                        mood,
                    });
                }
            }
            Token::Pause { .. } => {}
        }
    }
    if !cur.is_empty() {
        out.push(TargetPhrase {
            phones: cur,
            pause_ms: crate::g2p::LONG_PAUSE_MS,
            full_stop: true,
            mood: None,
        });
    }
    out
}

/// The original engine's transcription notation: phones separated by
/// spaces, words by `-`; a phrase ends with `#` (inside a sentence) or `@`
/// (end of sentence) followed by the pause in ms (`#120`), and a `?`/`!`
/// right after that marks a question/exclamation tune.
pub fn format_transcription(phrases: &[TargetPhrase]) -> String {
    let mut out = String::new();
    for p in phrases {
        for (i, t) in p.phones.iter().enumerate() {
            if i > 0 {
                out.push(if t.word_start { '-' } else { ' ' });
            }
            out.push_str(&t.sym);
        }
        out.push_str(&format!(" {}{}", if p.full_stop { '@' } else { '#' }, p.pause_ms));
        match p.mood {
            Some('I') => out.push('?'),
            Some('E') => out.push('!'),
            _ => {}
        }
        out.push(' ');
    }
    out.trim_end().to_string()
}

pub fn parse_transcription(text: &str) -> Vec<TargetPhrase> {
    let mut out = Vec::new();
    let mut cur: Vec<Target> = Vec::new();
    for tok in text.split_whitespace() {
        if let Some(edge) = tok.chars().next().filter(|c| *c == '#' || *c == '@') {
            let rest = &tok[1..];
            let mood = match rest.chars().last() {
                Some('?') => Some('I'),
                Some('!') => Some('E'),
                _ => None,
            };
            let digits: String = rest.chars().filter(|c| c.is_ascii_digit()).collect();
            let default = if edge == '@' { crate::g2p::LONG_PAUSE_MS } else { crate::g2p::SHORT_PAUSE_MS };
            if !cur.is_empty() {
                out.push(TargetPhrase {
                    phones: std::mem::take(&mut cur),
                    pause_ms: digits.parse().unwrap_or(default),
                    full_stop: edge == '@',
                    mood,
                });
            }
            continue;
        }
        let mut first = true;
        for part in tok.split('-') {
            if !part.is_empty() {
                cur.push(Target {
                    sym: part.to_string(),
                    word_start: !first || cur.is_empty(),
                });
            }
            first = false;
        }
    }
    if !cur.is_empty() {
        out.push(TargetPhrase {
            phones: cur,
            pause_ms: crate::g2p::LONG_PAUSE_MS,
            full_stop: true,
            mood: None,
        });
    }
    out
}

/// The builtin CAT stage: the original engine's own picking rules
/// (`select_like_original`).
pub fn select_all(db: &VoiceDatabase, phrases: &[TargetPhrase]) -> Vec<Phrase> {
    let mut out = Vec::new();
    for p in phrases {
        // two half-phones per phone
        let parts: Vec<&[Target]> = p.phones.chunks(MAX_DEMIPHONES_PER_PHRASE / 2).collect();
        for (i, part) in parts.iter().enumerate() {
            let last = i + 1 == parts.len();
            let edge = if last && p.full_stop { "@" } else { "#" };
            let pause = if last { p.pause_ms } else { crate::g2p::SHORT_PAUSE_MS };
            out.push(select_like_original(db, part, edge, if last { p.mood } else { None }, pause));
        }
    }
    out
}

/// One half of a recorded phone: (sentence, phone index, side 0 = left / 1 = right).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Half {
    unit: u32,
    phone: u16,
    side: u8,
}

impl Half {
    /// the next half in the same recording, if it is a timed one
    fn next(self, db: &VoiceDatabase) -> Option<Half> {
        let (phone, side) = if self.side == 0 { (self.phone, 1) } else { (self.phone + 1, 0) };
        db.units[self.unit as usize].phones.get(phone as usize).filter(|p| p.timed)?;
        Some(Half { unit: self.unit, phone, side })
    }
}

// --- the original engine's own piece picking (LoqTTS7.dll, MECHANICS.md §5) ---

/// Score of a recorded phone's prosodic class (row) standing for a wanted
/// one's (column): LoqTTS7.dll's table at 0x10070f28, read by 0x10045ad0.
const PROSODIC_CLASS_SCORE: [[i64; 16]; 16] = [
    [20, 0, 0, 0, 0, 2, 10, 5, 0, 0, 0, 0, 0, 0, 2, 0],
    [0, 20, 5, 2, 0, 0, 1, 0, 5, 1, 1, 0, 10, 2, 0, 0],
    [0, 5, 25, 0, 0, 0, 1, 0, 2, 0, 2, 0, 5, 0, 0, 0],
    [0, 2, 0, 20, 5, 0, 1, 0, 1, 5, 0, 1, 2, 10, 0, 0],
    [0, 0, 0, 5, 25, 0, 1, 0, 0, 2, 0, 2, 0, 5, 0, 0],
    [2, 0, 0, 0, 0, 25, 2, 5, 16, 16, 0, 0, 0, 0, 10, 0],
    [10, 1, 1, 1, 1, 2, 20, 10, 0, 0, 0, 0, 1, 1, 2, 0],
    [5, 0, 0, 0, 0, 5, 10, 25, 10, 10, 0, 0, 0, 0, 0, 0],
    [0, 5, 2, 1, 0, 16, 0, 10, 25, 2, 5, 0, 5, 1, 16, 0],
    [0, 1, 0, 5, 2, 16, 0, 10, 2, 25, 2, 5, 1, 5, 16, 0],
    [0, 1, 2, 0, 0, 0, 0, 0, 5, 0, 25, 0, 1, 0, 0, 0],
    [0, 0, 0, 1, 2, 0, 0, 0, 0, 5, 0, 25, 0, 1, 0, 0],
    [0, 10, 5, 2, 0, 0, 1, 0, 5, 1, 1, 0, 20, 2, 0, 0],
    [0, 2, 0, 10, 5, 0, 1, 0, 1, 5, 0, 1, 2, 20, 0, 0],
    [2, 0, 0, 0, 0, 10, 2, 0, 16, 16, 0, 0, 0, 0, 20, 0],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
];

/// Phone ids of the edge silences in every voice's table (`#` short, `@` long).
const ID_SHORT_PAUSE: u8 = 2;
/// Similarity at which context keeps extending: identical phones score 0x1e.
const SAME_PHONE: i64 = 30;

/// The CAT module's phone tables as LoqTTS7.dll builds them when a voice
/// loads (CatLoadVoiceReadOnlyData 0x1004bd60): a feature word per phone
/// (0x10050780), a weight class per phone ("TabelleFonetiche_", 0x100457b0)
/// and the phone-to-phone similarity matrix ("SomiglianzaFonetica_",
/// 0x10049a70 → 0x100495b0), all computed from the phone table in the voice's
/// own library (`phoneset`), exactly as the original does.
pub struct PhoneTables {
    pub index: std::collections::HashMap<String, u8>,
    pub features: Vec<u32>,
    pub class: Vec<u8>,
    /// row = wanted phone, column = recorded phone
    pub sim: Vec<u8>,
    /// each recorded sentence's phones, as table ids, and their prosodic classes
    pub unit_ids: Vec<Vec<u8>>,
    pub unit_classes: Vec<Vec<u8>>,
}

impl PhoneTables {
    /// Build them as the original does, from the voice library's bytes
    /// (`LTTS7<Voice>.dll`, `phoneset`): nothing but the voice is needed.
    /// Older libraries (Roberto, Allison, Katrin…) hold their own feature
    /// words, `@` = 0x28, which the engine converts (0x10050780); newer ones
    /// (Giulia, Steven, Carlos…) hold the engine's already, `@` = 0x17, and
    /// are taken as they are (CatLoadVoiceReadOnlyData 0x1004bd60 converts
    /// only below a version; 0x10050680 tests the `@` record for 0x28).
    pub fn from_library(lib: &[u8], db: &VoiceDatabase) -> Option<PhoneTables> {
        use crate::phoneset::*;
        let phones = phone_table(lib)?;
        let convert = phones.get(3).is_some_and(|p| p.base == 0x28);
        let features: Vec<u32> = phones.iter().map(|p| if convert { engine_features(p.base) } else { p.base }).collect();
        let class = features.iter().map(|&f| weight_class(f)).collect();
        let sim = features.iter().flat_map(|&a| features.iter().map(move |&c| similarity(a, c))).collect();
        let index = phones.iter().enumerate().map(|(i, p)| (p.name.clone(), i as u8)).collect();
        let mut t = PhoneTables {
            index,
            features,
            class,
            sim,
            unit_ids: Vec::new(),
            unit_classes: Vec::new(),
        };
        for u in &db.units {
            let ids: Vec<u8> = u.phones.iter().map(|p| t.id(&p.sym)).collect();
            // a `-` in the `U=` line ends a word (`s `o n o-i n…`)
            let word_end: Vec<bool> = (0..u.phones.len())
                .map(|j| u.phones.get(j + 1).map(|p| p.word_start).unwrap_or(false))
                .collect();
            t.unit_classes.push(t.label(&ids, &word_end, u.mood.unwrap_or('D')));
            t.unit_ids.push(ids);
        }
        Some(t)
    }

    /// (symbol, engine feature word) for every phone of the voice.
    pub fn phone_features(&self) -> Vec<(String, u32)> {
        let mut v: Vec<(String, u32)> = self.index.iter().map(|(n, &i)| (n.clone(), self.features[i as usize])).collect();
        v.sort_by_key(|(_, f)| *f);
        v.sort_by_key(|(n, _)| self.index[n]);
        v
    }

    pub fn id(&self, sym: &str) -> u8 {
        self.index.get(sym).copied().unwrap_or(0)
    }

    fn sim(&self, wanted: u8, recorded: u8) -> i64 {
        self.sim[wanted as usize * self.features.len() + recorded as usize] as i64
    }

    /// Prosodic classes of a run of phones, LoqTTS7.dll's default
    /// CatVidCalcProsLab (0x1004bf00; no voice library replaces it): leading
    /// silences 15; up to the first stressed vowel 7, after it 0; from the last
    /// stressed vowel on 3 (a statement ending in `@`) or 1; the phone before
    /// the edge 4 or 2, the edge itself 4 (`@`, not a question) or 2; the first
    /// phone 5, or 8/9 when no vowel is stressed, 10/11 when it is the only one.
    /// The last phone of each word but the last comes in marked 6 (the wanted
    /// phrase's `w` mark, 0x10049cd0; the `-` joins of a recording's `U=` line)
    /// and keeps it, or becomes 12/13 at the end and 14 at the start.
    pub fn label(&self, ids: &[u8], word_end: &[bool], mood: char) -> Vec<u8> {
        let f: Vec<u32> = ids.iter().map(|&i| self.features[i as usize]).collect();
        label_prosody(&f, word_end, mood)
    }
}

/// See `PhoneTables::label`.
pub fn label_prosody(f: &[u32], word_end: &[bool], mood: char) -> Vec<u8> {
    let mut out: Vec<u8> = (0..f.len()).map(|k| if word_end.get(k) == Some(&true) { 6 } else { 15 }).collect();
    let s = f.iter().take_while(|&&x| x == 0x13 || x == 0x10).count();
    out[..s].fill(15);
    let (p, o) = (&f[s..], &mut out[s..]);
    let n = p.len();
    if n == 0 {
        return out;
    }
    let (mut c, mut last_stress) = (0u8, 0usize);
    for k in 0..n {
        if o[k] != 6 {
            o[k] = c.wrapping_sub(1) & 7;
        }
        if p[k] & 0x10003 == 0x10001 {
            c = 1;
            last_stress = k;
        }
    }
    if last_stress != 0 && p[last_stress - 1] & 3 == 2 && p[last_stress - 1] & 0x1e3 == 0xc2 {
        last_stress -= 1;
    }
    let mut u = n - 1;
    let mut statement = false;
    if p[n - 1] & 3 == 3 {
        statement = p[n - 1] == 0x17 && mood != 'I';
        o[n - 1] = if statement { 4 } else { 2 };
        if n == 1 {
            return out;
        }
        u = n - 2;
    }
    if u == 0 {
        o[0] = if statement { 11 } else { 10 };
        return out;
    }
    o[u] = if statement { 4 } else { 2 };
    loop {
        u -= 1;
        if u == 0 {
            break;
        }
        if u < last_stress {
            o[0] = if o[0] == 6 { 14 } else { 5 };
            return out;
        }
        o[u] = match (statement, o[u] == 6) {
            (true, true) => 13,
            (true, false) => 3,
            (false, true) => 12,
            (false, false) => 1,
        };
    }
    if last_stress == 0 {
        o[0] = if statement { 9 } else { 8 };
        return out;
    }
    o[0] = if o[0] == 6 { 14 } else { 5 };
    out
}

/// How far apart two phones are for the picker (LoqTTS7.dll 0x100464b0, on
/// their feature words): 0 the same, 1 two consonants of the same family
/// (`f & 0x1e3` 0x1e2/0x102/0x142…) and kind, 2 two consonants of the same
/// manner and voicing, else 1000.
fn phone_distance(a: u32, b: u32) -> i64 {
    if a == b {
        return 0;
    }
    let family = |f: u32| {
        let m = f & 0x1e3;
        (m == 0x1e2 && f & 0x1e03 != 0x802) || m == 0x102 || (m == 0x142 && matches!(f & 0x1e03, 0x1c02 | 0x1a02 | 0x1e02))
    };
    let kind = |f: u32| f & 0xc == 4;
    if family(a) && family(b) && kind(a) == kind(b) {
        return 1;
    }
    let manner = |m: u32| match m {
        0x1c2 => 0,
        0x1a2 => 1,
        0xa2 => 2,
        0x142 => 3,
        _ => 5,
    };
    if a & 3 == 2 && b & 3 == 2 && a & 0x1e3 != 0xc2 && b & 0x1e3 != 0xc2 && (a ^ b) & 0x10 == 0 && kind(a) == kind(b) && manner(a & 0x1e3) == manner(b & 0x1e3)
    {
        return 2;
    }
    1000
}

fn is_vowel_f(f: u32) -> bool {
    f & 3 == 1
}

fn is_unvoiced_consonant_f(f: u32) -> bool {
    f & 3 != 1 && f & 0x10 != 0
}

/// LoqTTS7.dll 0x10046670: the factor a candidate's score is multiplied by
/// (only for statement and question recordings, `D`/`I`; others get 1).
/// 10, or 5 for a right half that doesn't take the next left half too; +2 when
/// the recording goes on matching the wanted phones (class score >= 10) up to
/// a consonant-like phone (weight class < 6); +5 when the pick carries straight
/// on from the previous one and the recording keeps matching (distance <= 1)
/// until a phone of lower weight class than this one.
fn picker_factor(tab: &PhoneTables, w: &Wanted, ui: usize, pos: usize, i: usize, side: u8, one_half: bool, carries_on: bool) -> i64 {
    if i == 0 && side == 0 {
        return 10;
    }
    let (uid, ucls) = (&tab.unit_ids[ui], &tab.unit_classes[ui]);
    let nu = uid.len();
    let cls = |uk: usize, tk: usize| PROSODIC_CLASS_SCORE[(ucls[uk] & 15) as usize][(w.class(tk) & 15) as usize];
    let f = |id: u8| tab.features[id as usize];
    let mut m = if one_half && side == 1 { 5 } else { 10 };
    let (mut up, mut ti) = (pos + 1, i + 1);
    while up < nu {
        if cls(up, ti) < 10 || phone_distance(f(uid[up]), f(w.id(ti))) > 999 {
            break;
        }
        if tab.class[uid[up] as usize] < 6 {
            m += 2;
            break;
        }
        if w.id(ti) != uid[up] {
            break;
        }
        up += 1;
        ti += 1;
    }
    if carries_on {
        let own = tab.class[uid[pos] as usize];
        let (mut up, mut ti) = (pos + 1, i + 1);
        while up < nu {
            if phone_distance(f(uid[up]), f(w.id(ti))) > 1 || cls(up, ti) < 10 {
                return m;
            }
            if tab.class[uid[up] as usize] < own {
                return m + 5;
            }
            if w.id(ti) != uid[up] {
                return m;
            }
            up += 1;
            ti += 1;
        }
    }
    m
}

/// LoqTTS7.dll 0x100468c0: pitch and length continuity, for vowel and voiced
/// candidates. It compares the half picked last with the half just before the
/// candidate in its own recording: (100 − |Δpitch|)·3·2 when under 100 Hz
/// apart, plus 100 − (ratio − 100) when the lengths (each with the half after
/// it) are within 2×. Fricative-like consonants keep a third, nasals/liquids
/// 3/5, when at a phrase start or next to an unvoiced consonant.
fn prosody_original(tab: &PhoneTables, db: &VoiceDatabase, w: &Wanted, prev: Option<Half>, h: Half, i: usize) -> i64 {
    let Some(prev) = prev else { return 0 };
    let uid = &tab.unit_ids[h.unit as usize];
    let pos = h.phone as usize;
    let fc = tab.features[uid[pos] as usize];
    if !(is_vowel_f(fc) || fc & 0x10 == 0) {
        return 0;
    }
    let half_len = |x: Half| {
        let p = &db.units[x.unit as usize].phones[x.phone as usize];
        (if x.side == 0 { p.len_l } else { p.len_r }) as i64
    };
    let half_tick = |x: Half| {
        let p = &db.units[x.unit as usize].phones[x.phone as usize];
        if x.side == 0 {
            p.tick_l
        } else {
            p.tick_r
        }
    };
    let half_pitch = |x: Half| {
        let p = &db.units[x.unit as usize].phones[x.phone as usize];
        (if x.side == 0 { p.pitch_l } else { p.pitch_r }) as i64
    };
    let before = if h.side == 1 {
        Some(Half { side: 0, ..h })
    } else if pos > 0 && db.units[h.unit as usize].phones[pos - 1].timed {
        Some(Half {
            unit: h.unit,
            phone: h.phone - 1,
            side: 1,
        })
    } else {
        None
    };
    let (mut s, mut a, mut b) = (0i64, 0i64, 0i64);
    if let Some(bf) = before {
        let d = (half_pitch(prev) - half_pitch(bf)).abs();
        s = if d < 100 { (100 - d) * 3 } else { 0 } * 2;
        a = half_len(prev);
        b = half_len(bf);
    }
    // only when the half after the last pick follows it with no gap in time
    if let Some(nx) = prev.next(db).filter(|&nx| half_tick(prev) + half_len(prev) as u64 == half_tick(nx)) {
        a += half_len(nx);
        b += half_len(h);
    }
    if a != 0 && b != 0 {
        let ratio = a.max(b) * 100 / a.min(b);
        if ratio - 100 < 100 {
            s += 100 - (ratio - 100);
        }
    }
    let i2 = i + h.side as usize;
    let next_f = tab.features[*uid.get(pos + 1).unwrap_or(&0) as usize];
    let edge_ok = i2 == 0 || is_unvoiced_consonant_f(next_f) || is_unvoiced_consonant_f(tab.features[w.id(i2 - 1) as usize]);
    if edge_ok && fc & 3 == 2 {
        match fc & 0x1e3 {
            0x1e2 | 0x102 | 0x142 => return s / 3,
            0x1c2 | 0xa2 | 0x1a2 => s = s * 3 / 5,
            _ => {}
        }
    }
    s
}

/// Whether the original takes the next left half together with a right half
/// (0x10046b20 local_20 = 2): the recording has the wanted next phone there,
/// its prosodic class scores >= 10 and it is timed.
fn takes_two(tab: &PhoneTables, db: &VoiceDatabase, w: &Wanted, h: Half, i: usize) -> bool {
    let (uid, ucls) = (&tab.unit_ids[h.unit as usize], &tab.unit_classes[h.unit as usize]);
    let nx = h.phone as usize + 1;
    h.side == 1
        && nx < uid.len()
        && uid[nx] == w.id(i + 1)
        && PROSODIC_CLASS_SCORE[(ucls[nx] & 15) as usize][(w.class(i + 1) & 15) as usize] >= 10
        && db.units[h.unit as usize].phones[nx].timed
}

/// What the picker knows about the wanted phrase, in table terms: its phone
/// ids, then the edge silence, then 0 (as LoqTTS7.dll lays it out), and the
/// prosodic class of each.
struct Wanted {
    ids: Vec<u8>,
    classes: Vec<u8>,
    mood: char,
}

impl Wanted {
    fn new(tab: &PhoneTables, t: &[&Target], edge: &str, mood: char) -> Wanted {
        let mut ids: Vec<u8> = t.iter().map(|x| tab.id(&x.sym)).collect();
        ids.push(tab.id(edge));
        let word_end: Vec<bool> = (0..ids.len()).map(|k| t.get(k + 1).map(|x| x.word_start).unwrap_or(false)).collect();
        let classes = tab.label(&ids, &word_end, mood);
        ids.push(0);
        Wanted { ids, classes, mood }
    }
    fn id(&self, k: usize) -> u8 {
        self.ids.get(k).copied().unwrap_or(0)
    }
    fn class(&self, k: usize) -> u8 {
        self.classes.get(k).copied().unwrap_or(15)
    }
}

/// LoqTTS7.dll 0x10045ad0, the score of recorded phone `pos` of sentence `ui`
/// standing for wanted phone `i` (half `side`). Outward from the phone, each
/// neighbour adds its similarity (0..30) and its prosodic-class match
/// (`PROSODIC_CLASS_SCORE`), weighted by distance — left half: right 6, 3,
/// then 1…; left 10, 4, 2, then 1…; right half: right 10, 6, 3, then 1…;
/// left 6, 3, then 1… — and goes on only while the neighbour is the same
/// phone (30). The phone's own class match counts ×10. The left context is
/// scaled by the phone's weight class / 10 (0x100457b0), and so is the class
/// part when the context stops at the first neighbour on the far side. A
/// question recording counts ×1.5 (class part) when a question is wanted and
/// it matches back to its start, else everything ÷3. A recording saying the
/// whole phrase with the same tune scores 10 000 000 (5 000 000 for e/F).
fn context_score_original(tab: &PhoneTables, ui: usize, pos: usize, w: &Wanted, i: usize, side: u8, unit_mood: char) -> (i64, i64) {
    let (uid, ucls) = (&tab.unit_ids[ui], &tab.unit_classes[ui]);
    let nu = uid.len();
    let cls = |uk: usize, tk: usize| PROSODIC_CLASS_SCORE[(ucls[uk] & 15) as usize][(w.class(tk) & 15) as usize];
    let mut class_part = cls(pos, i) * 10;
    let mut right = 1i64;
    let mut left = 0i64;
    // to the right
    let right_w: &[i64] = if side == 0 { &[6, 3] } else { &[10, 6, 3] };
    let mut right_ext = 1;
    let mut step = 1;
    while pos + step < nu {
        let wgt = *right_w.get(step - 1).unwrap_or(&1);
        let s = tab.sim(w.id(i + step), uid[pos + step]);
        right += s * wgt;
        class_part += cls(pos + step, i + step) * wgt;
        right_ext = step + 1;
        if s < SAME_PHONE {
            break;
        }
        step += 1;
    }
    // to the left
    let left_w: &[i64] = if side == 0 { &[10, 4, 2] } else { &[6, 3] };
    let mut left_ext = 0;
    let mut step = 1;
    while step <= pos && step <= i + 1 {
        let wgt = *left_w.get(step - 1).unwrap_or(&1);
        let at_edge = i + 1 == step;
        let want = if at_edge { ID_SHORT_PAUSE } else { w.id(i - step) };
        let s = tab.sim(want, uid[pos - step]);
        left += s * wgt;
        if !at_edge {
            class_part += cls(pos - step, i - step) * wgt;
        }
        left_ext = step;
        if s < SAME_PHONE {
            break;
        }
        step += 1;
    }
    // ponytail: the decompiled left-half loop leaves its count one higher once
    // past the third neighbour (0x10045ad0 local_18); only the question rule
    // below reads it
    if side == 0 && left_ext >= 4 {
        left_ext += 1;
    }
    let weight = tab.class[w.id(i) as usize] as i64;
    if (side == 0 && left_ext == 1 && i != 0) || (side == 1 && right_ext == 2) {
        class_part = weight * class_part / 10;
    }
    right += weight * left / 10;
    if unit_mood == 'I' {
        if matches!(w.mood, 'I' | 'X') && pos == left_ext && uid.get(1) == Some(&w.id(0)) {
            class_part = class_part * 3 / 2;
        } else {
            class_part /= 3;
            right /= 3;
        }
    }
    // the whole phrase, edge to edge
    if right_ext + 1 >= nu && uid[0] == ID_SHORT_PAUSE && w.ids.get(right_ext) == Some(&0) && right_ext >= 1 {
        let last = uid[nu - 1];
        if w.id(right_ext - 1) == last && (last == 2 || last == 3) {
            let (u, t) = (unit_mood, w.mood);
            if u == t || (t == 'D' && u == 'd') || (t == 'I' && u == 'i') || (t == 'E' && u == 'e') || (t == 'F' && u == 'f') || (t == 'X' && u == 'x') {
                return (10_000_000, class_part);
            }
            if (u == 'e' && t == 'F') || (u == 'f' && t == 'E') {
                return (5_000_000, class_part);
            }
        }
    }
    (right + class_part, class_part)
}

/// Fallback for voices whose tables were not dumped from the original
/// (`PhoneTables`): 100 for the same symbol, 50 when only stress or length
/// differ, else 0; context keeps extending while it stays >= 30.
fn similarity(a: &str, b: &str) -> u32 {
    if a == b {
        100
    } else if relax(a, 2) == relax(b, 2) {
        50
    } else {
        0
    }
}

/// Context score of recorded phone `j` of sentence `u` standing for wanted
/// phone `i`, as LoqTTS7.dll 0x10045ad0 sums it: similarity of the
/// neighbours outward, weighted by distance, while each stays >= 30.
/// Left half: right side 6, 3, 1, 1…; left side 10, 4, 2, 1, 1…
/// Right half: right side 10, 6, 3, 1…; left side 6, 3, 1, 1…
fn context_score(u: &crate::loquendo::Unit, j: usize, t: &[&Target], i: usize, side: u8, edge: &str) -> i64 {
    let want = |k: isize| -> &str {
        if k < 0 {
            "#"
        } else if k as usize >= t.len() {
            edge
        } else {
            &t[k as usize].sym
        }
    };
    let have = |k: isize| -> &str {
        if k < 0 || k as usize >= u.phones.len() {
            "#"
        } else {
            &u.phones[k as usize].sym
        }
    };
    let (right_w, left_w): (&[i64], &[i64]) = if side == 0 {
        (&[6, 3, 1], &[10, 4, 2, 1])
    } else {
        (&[10, 6, 3, 1], &[6, 3, 1])
    };
    let mut score = 1i64;
    for (dir, weights) in [(1isize, right_w), (-1isize, left_w)] {
        for step in 1..=t.len() as isize + 1 {
            let s = similarity(have(j as isize + dir * step), want(i as isize + dir * step)) as i64;
            let w = *weights.get(step as usize - 1).unwrap_or(&1);
            score += s * w;
            let w_sym = want(i as isize + dir * step);
            if s < 30 || w_sym == "#" || w_sym == "@" {
                break;
            }
        }
    }
    score
}

/// The original's pitch/duration closeness to the previous pick
/// (LoqTTS7.dll 0x100468c0): (100 − |Δf0|)·3·2 when the jump is under 100 Hz,
/// plus 100 − (ratio − 100) when the two durations are within 2× of each other.
fn prosody_score(prev: Option<Half>, h: Half, db: &VoiceDatabase) -> i64 {
    let Some(prev) = prev else { return 0 };
    let pp = &db.units[prev.unit as usize].phones[prev.phone as usize];
    let p = &db.units[h.unit as usize].phones[h.phone as usize];
    let (f_prev, f) = (
        if prev.side == 0 { pp.pitch_l } else { pp.pitch_r } as i64,
        if h.side == 0 { p.pitch_l } else { p.pitch_r } as i64,
    );
    let mut s = 0;
    let d = (f_prev - f).abs();
    if f_prev > 0 && f > 0 && d < 100 {
        s += (100 - d) * 3 * 2;
    }
    let (a, b) = ((pp.end - pp.start) as i64, (p.end - p.start) as i64);
    if a > 0 && b > 0 {
        let ratio = a.max(b) * 100 / a.min(b);
        if ratio - 100 < 100 {
            s += 100 - (ratio - 100);
        }
    }
    s
}

/// The original's picking (LoqTTS7.dll 0x10047ec0 → 0x10046de0 →
/// 0x10046b20): walk the wanted half-phones left to right; at each, score
/// every recorded instance of the phone (context + prosody, ×1.5 for a
/// question unit when a question is wanted, ÷3 otherwise; +1 when it carries
/// straight on from the previous pick) and take the best; at a right half
/// also take the next phone's left half when the recording has that phone
/// next, so most joins fall in mid-phone.
pub fn select_like_original(db: &VoiceDatabase, t: &[Target], edge: &str, mood: Option<char>, pause_ms: u32) -> Phrase {
    let mut phrase = Phrase {
        pause_ms,
        mood,
        ..Default::default()
    };
    let level_of = |sym: &str| (0..3).find(|&l| db.singles[l].contains_key(relax(sym, l)));
    let t: Vec<&Target> = t
        .iter()
        .filter(|x| {
            let ok = level_of(&x.sym).is_some();
            if !ok {
                phrase.missing.push(x.sym.clone());
            }
            ok
        })
        .collect();
    phrase.phones = t.iter().map(|x| x.sym.clone()).collect();
    let n = t.len();
    if n == 0 {
        return phrase;
    }
    // without the voice's tables: a whole recorded sentence saying exactly
    // this phrase, with the same tune, beats everything (with them the
    // context score finds it, 0x10045ad0)
    let wanted: Vec<&str> = t.iter().map(|x| x.sym.as_str()).collect();
    for (ui, u) in db.units.iter().enumerate().filter(|_| db.tables.is_none()) {
        let inner: Vec<&str> = u.phones.iter().filter(|p| p.timed).map(|p| p.sym.as_str()).collect();
        if inner == wanted && u.mood == mood && u.phones.last().map(|p| p.sym.as_str()) == Some(edge) {
            let (a, b) = (&u.phones[1], &u.phones[u.phones.len() - 2]);
            phrase.pieces.push(Piece {
                covers: phrase.phones.clone(),
                unit: ui as u32,
                from: a.start,
                to: b.end,
                first_half: 2,
                halves: 2 * (u.phones.len() as u32 - 2),
                pitch: a.pitch_l,
                hard: false,
            });
            return phrase;
        }
    }
    let wanted_t = db.tables.as_ref().map(|tab| Wanted::new(tab, &t, edge, mood.unwrap_or('D')));
    // QQ_TRACE_CAT=1: every candidate's terms on stderr, in the format of
    // oracle/loqref.c LOQ_TRACE_CAT (units named by their first phone's index)
    let trace = std::env::var_os("QQ_TRACE_CAT").is_some();
    let unit_first: Vec<usize> = if trace {
        db.units
            .iter()
            .scan(0, |acc, u| {
                let at = *acc;
                *acc += u.phones.len();
                Some(at)
            })
            .collect()
    } else {
        Vec::new()
    };
    let mut path: Vec<Half> = Vec::with_capacity(2 * n);
    let mut k = 0;
    while k < 2 * n {
        let (i, side) = (k / 2, (k % 2) as u8);
        let level = level_of(&t[i].sym).unwrap();
        let prev = path.last().copied();
        let mut best: Option<(i64, Half)> = None;
        // every database of the voice, the last first (their instances come
        // in that order); once one holds a score of 5 000 000 or more (a
        // whole-phrase match) the rest aren't asked (0x10046de0)
        let mut asking = None;
        for &(unit, phone) in &db.singles[level][relax(&t[i].sym, level)] {
            let in_db = db.database_of(unit);
            if asking != Some(in_db) {
                if best.is_some_and(|b| b.0 >= 5_000_000) {
                    break;
                }
                asking = Some(in_db);
            }
            let h = Half { unit, phone, side };
            let u = &db.units[unit as usize];
            let mut score = match (&db.tables, &wanted_t) {
                (Some(tab), Some(w)) => {
                    let (s, class_part) = context_score_original(tab, unit as usize, phone as usize, w, i, side, u.mood.unwrap_or('D'));
                    if trace {
                        eprintln!("C {k} {} {phone} {s} {class_part}", unit_first[unit as usize]);
                    }
                    s
                }
                _ => {
                    let mut score = context_score(u, phone as usize, &t, i, side, edge);
                    match (u.mood == Some('I'), mood == Some('I')) {
                        (true, true) => score = score * 3 / 2,
                        (true, false) => score /= 3,
                        _ => {}
                    }
                    score
                }
            };
            let carries_on = prev.and_then(|p| p.next(db)) == Some(h);
            if carries_on {
                score += 1;
            }
            if let (Some(tab), Some(w)) = (&db.tables, &wanted_t) {
                // 0x10046b20: (context + prosody) × factor, the factor only for
                // statement and question recordings or whole-phrase matches
                let factor = if matches!(u.mood.unwrap_or('D'), 'D' | 'I') || score >= 5_000_000 {
                    picker_factor(tab, w, unit as usize, phone as usize, i, side, !takes_two(tab, db, w, h, i), carries_on)
                } else {
                    1
                };
                let pros = prosody_original(tab, db, w, prev, h, i);
                if trace {
                    eprintln!(
                        "F {k} {} {phone} {factor} {}",
                        unit_first[unit as usize],
                        if takes_two(tab, db, w, h, i) { 2 } else { 1 }
                    );
                    eprintln!("P {phone} {pros}");
                }
                score = (score + pros) * factor;
            } else {
                score += prosody_score(prev, h, db);
            }
            if best.map(|b| score > b.0).unwrap_or(true) {
                best = Some((score, h));
            }
        }
        let (best_score, h) = best.unwrap();
        path.push(h);
        k += 1;
        // a whole-phrase match takes all of its recording (0x10046de0)
        let phones = db.units[h.unit as usize].phones.len();
        if best_score >= 5_000_000 && phones > 2 {
            let mut last = h;
            while k < 2 * n {
                let Some(nx) = last.next(db) else { break };
                path.push(nx);
                last = nx;
                k += 1;
            }
            continue;
        }
        // at a right half, carry on into the next phone's left half when the
        // recording has the wanted phone there (0x10046b20, local count 2)
        if side == 1 && k < 2 * n {
            if let Some(nx) = h.next(db) {
                let sym = &db.units[nx.unit as usize].phones[nx.phone as usize].sym;
                let ok = match (&db.tables, &wanted_t) {
                    (Some(tab), Some(w)) => takes_two(tab, db, w, h, k / 2 - 1),
                    _ => sym == &t[k / 2].sym,
                };
                if ok {
                    path.push(nx);
                    k += 1;
                }
            }
        }
        // a pick ending on a left half may take that phone's right half too
        // (0x10046de0): unless the wanted phone is voiceless, the recording
        // ends there, its next phone is the wanted one, or this phone isn't
        // stronger than the next (weight class more than 1 above, distance
        // under 1000); or anyway when the recording ends right after and a
        // pause is wanted next
        if let (Some(tab), Some(w), Some(&last)) = (&db.tables, &wanted_t, path.last()) {
            if last.side == 0 && k < 2 * n && std::env::var_os("QQ_NO_EXTEND").is_none() {
                let uid = &tab.unit_ids[last.unit as usize];
                let pos = last.phone as usize;
                let i = k / 2;
                let wanted = tab.features[w.id(i) as usize];
                let voiceless = wanted & 3 != 1 && wanted & 0x10 != 0;
                let ends = uid.len() <= pos + 1;
                let next_ti = k.div_ceil(2);
                let next_is_wanted = !ends && uid[pos + 1] == w.id(next_ti) && !matches!(uid[pos + 1], 2 | 3);
                let weaker = ends
                    || tab.class[uid[pos] as usize] <= tab.class[uid[pos + 1] as usize] + 1
                    || phone_distance(tab.features[uid[pos + 1] as usize], tab.features[w.id(next_ti) as usize]) > 999;
                let extend = if voiceless || ends || next_is_wanted || weaker {
                    pos + 2 == uid.len() && matches!(w.id(next_ti), 2 | 3)
                } else {
                    true
                };
                if extend {
                    path.push(Half { side: 1, ..last });
                    k += 1;
                }
            }
        }
    }
    // runs of halves that carry straight on become pieces
    for (k, h) in path.iter().enumerate() {
        let p = &db.units[h.unit as usize].phones[h.phone as usize];
        let (from, to) = if h.side == 0 { (p.start, p.mid) } else { (p.mid, p.end) };
        let sym = &t[k / 2].sym;
        if k > 0 && path[k - 1].next(db) == Some(*h) {
            let last = phrase.pieces.last_mut().unwrap();
            last.to = to;
            last.halves += 1;
            if h.side == 0 {
                last.covers.push(sym.clone());
            }
        } else {
            let pitch = if h.side == 0 { p.pitch_l } else { p.pitch_r };
            phrase.pieces.push(Piece {
                covers: vec![sym.clone()],
                unit: h.unit,
                from,
                to,
                first_half: 2 * h.phone as u32 + h.side as u32,
                halves: 1,
                pitch,
                hard: h.side == 0 && k > 0,
            });
        }
    }
    phrase
}

/// The SIG stage: decode every piece and join them.
///
/// `Original`: each stretch is decoded with a fresh decoder (as the codec DLL
/// does on a seek) from its `.sde` time, and butted against the next with
/// a hard cut; phrases are separated by exactly their pause. `Smooth`: joins
/// are cross-faded over 5 ms after sliding the new piece up to ±5 ms so its
/// waveform lines up with the old, and phrase edges fade in/out.
///
/// Everything is joined at the recordings' rate; 44 kHz voices then go
/// through the original's resampler to 32 kHz as SIG streams into it
/// (`codec::StreamResampler`: one phone at a time, a flush after every
/// phrase).
pub fn render(plan: &[Phrase], db: &VoiceDatabase, settings: Settings) -> Result<Speech, String> {
    let bank = db
        .audio
        .as_ref()
        .ok_or_else(|| db.audio_error.clone().unwrap_or_else(|| "this voice's recordings were not found".into()))?;
    let rate = bank.rate;
    let ms = |m: u32| (m as usize) * rate as usize / 1000;
    let gain = settings.volume / 32768.0;
    let resampled = rate != crate::codec::OUTPUT_RATE;
    if settings.join == JoinStyle::Original {
        let (mut out, lens, ends) = join_like_original(plan, db, rate);
        if resampled {
            // the engine resamples one phone (two half units) at a time
            // and flushes at the end of every phrase
            let mut rs = crate::codec::StreamResampler::new(rate, crate::codec::OUTPUT_RATE);
            let (mut res, mut at, mut unit) = (Vec::new(), 0, 0);
            for &end in &ends {
                while unit < end {
                    let k = (end - unit).min(2);
                    let n: usize = lens[unit..unit + k].iter().sum();
                    res.extend(rs.push(&out[at..at + n]));
                    at += n;
                    unit += k;
                }
                res.extend(rs.flush());
            }
            out = res;
        }
        let out = out.into_iter().map(|v| v as f32 * gain).collect();
        return Ok(Speech {
            samples: out,
            rate: if resampled { crate::codec::OUTPUT_RATE } else { rate },
        });
    }
    let mut out: Vec<f32> = Vec::new();
    for phrase in plan {
        let mut tail: Vec<f32> = Vec::new();
        let phrase_start = out.len();
        for piece in &phrase.pieces {
            let from = piece.from as usize;
            let len = piece.to.saturating_sub(piece.from) as usize;
            let head = from.min(FADE + SLIDE);
            let pad = FADE + SLIDE;
            let bank = db.bank(piece.unit).unwrap_or(bank);
            let buf: Vec<f32> = bank.read(from - head, head + len + pad).into_iter().map(|s| s as f32 * gain).collect();
            let nominal_end = head + len;
            if tail.is_empty() {
                out.extend_from_slice(&buf[head..nominal_end]);
            } else {
                // best slide s: the new piece's sample head+s lands on the join
                let fade = FADE.min(tail.len()).min(len / 2).max(1);
                let lo = -(SLIDE.min(head) as isize);
                let hi = SLIDE.min(len.saturating_sub(fade + 1)) as isize;
                let mut best = (f32::MIN, 0isize);
                for s in lo..=hi {
                    let at = (head as isize + s) as usize;
                    let (mut xy, mut xx, mut yy) = (0.0f32, 1e-9f32, 1e-9f32);
                    for i in 0..fade {
                        let (x, y) = (tail[i], buf[at + i]);
                        xy += x * y;
                        xx += x * x;
                        yy += y * y;
                    }
                    let r = xy / (xx * yy).sqrt();
                    if r > best.0 {
                        best = (r, s);
                    }
                }
                let at = (head as isize + best.1) as usize;
                for i in 0..fade {
                    let w = (i as f32 + 0.5) / fade as f32;
                    out.push(tail[i] * (1.0 - w) + buf[at + i] * w);
                }
                out.extend_from_slice(&buf[(at + fade).min(nominal_end)..nominal_end]);
            }
            tail = buf[nominal_end..].to_vec();
        }
        if settings.join == JoinStyle::Smooth {
            let n = out.len() - phrase_start;
            let f = FADE.min(n / 2);
            for i in 0..f {
                let w = i as f32 / f as f32;
                out[phrase_start + i] *= w;
                let j = out.len() - 1 - i;
                out[j] *= w;
            }
        }
        out.extend(std::iter::repeat_n(0.0, ms(phrase.pause_ms)));
    }
    if resampled {
        let pcm: Vec<i16> = out.iter().map(|v| (v / gain).round().clamp(-32768.0, 32767.0) as i16).collect();
        let out = crate::codec::resample(&pcm, rate, crate::codec::OUTPUT_RATE).into_iter().map(|v| v as f32 * gain).collect();
        return Ok(Speech {
            samples: out,
            rate: crate::codec::OUTPUT_RATE,
        });
    }
    Ok(Speech { samples: out, rate })
}

/// Samples decoded on each side of a unit, as LoqTTS7.dll's unit fetch
/// (0x1001f580) asks for: `(rate/10 · 400) / 20000`, 64 at 32 kHz, 88 at 44.
pub fn unit_margin(rate: u32) -> usize {
    (rate as usize / 10 * 400) / 20000
}

/// The whole utterance joined the way the original's SIG does with
/// plain-copy units (LoqTTS7.dll 0x1001faa0). Every half-phone is a unit of
/// its own, decoded with `unit_margin` samples on each side (none before a
/// half at the recording's very start, 0x1001f580) in the order the engine
/// asks for them, and given its pitch marks (0x1001e3a0); pauses are
/// zero-filled units with no lead-in and a 4× tail. Halves inside one picked
/// run are copied (algorithm 0); the last half of a run is joined to the next
/// unit (algorithm 1, `sig::join`): pitch-synchronously between two voiced
/// halves, else cross-faded. Each unit then plays its own samples.
/// Returns the samples, each unit's length in them and, for every phrase,
/// the number of units up to its end.
fn join_like_original(plan: &[Phrase], db: &VoiceDatabase, rate: u32) -> (Vec<i16>, Vec<usize>, Vec<usize>) {
    use crate::sig::{join, pitch_marks, JoinUnit, Voicing};
    let m = unit_margin(rate);
    let mut out = Vec::new();
    let mut lens = Vec::new();
    let (mut units, mut ends) = (0, Vec::new());
    // the unit before, waiting for its join; whether it ends a run
    let mut prev: Option<(JoinUnit, bool)> = None;
    let mut push = |mut u: JoinUnit, ends_run: bool, out: &mut Vec<i16>, lens: &mut Vec<usize>| {
        if let Some((mut p, p_ends)) = prev.take() {
            if p_ends {
                join(&mut p, &mut u);
            }
            out.extend_from_slice(&p.buf[p.pre..p.pre + p.len]);
            lens.push(p.len);
        }
        prev = Some((u, ends_run));
    };
    for phrase in plan {
        // the phrase's half-phones in order: (sentence, half index, start,
        // length, F0, voicing); halves of no length are units too
        let mut halves = Vec::new();
        for p in &phrase.pieces {
            let u = &db.units[p.unit as usize];
            for (i, side) in p.half_phones() {
                let ph = &u.phones[i];
                let voicing = db
                    .tables
                    .as_ref()
                    .map(|t| Voicing::of_features(t.features[t.id(&ph.sym) as usize]))
                    .unwrap_or(Voicing::Other);
                let (a, b, f0) = if side == 0 { (ph.start, ph.mid, ph.pitch_l) } else { (ph.mid, ph.end, ph.pitch_r) };
                halves.push((p.unit, 2 * i + side as usize, a as usize, b.saturating_sub(a) as usize, f0 as u32, voicing));
            }
        }
        for (k, &(unit, half, start, len, f0, voicing)) in halves.iter().enumerate() {
            // a run goes on while the next half follows this one in the
            // recording (0x100472f0)
            let ends_run = halves.get(k + 1).is_none_or(|next| (next.0, next.1) != (unit, half + 1));
            let pre = if start == 0 { 0 } else { m.min(start) };
            let buf = db.bank(unit).map(|b| b.read(start - pre, pre + len + m)).unwrap_or_else(|| vec![0; pre + len + m]);
            let marks = {
                let s = |i: isize| *buf.get((i + pre as isize) as usize).unwrap_or(&0) as i32;
                pitch_marks(&s, len, rate, f0, voicing)
            };
            let unit = JoinUnit {
                buf,
                pre,
                len,
                post: m,
                marks,
                voiced: voicing == Voicing::Voiced,
            };
            push(unit, ends_run, &mut out, &mut lens);
            units += 1;
        }
        // a pause is two half-phone units of zeros (no lead-in, a 4× tail);
        // the first is joined to the second, the second to nothing: the
        // speech after a pause starts as recorded
        let len = phrase.pause_ms as usize * rate as usize / 1000;
        if len > 0 {
            for (half, ends_run) in [(len / 2, true), (len - len / 2, false)] {
                let unit = JoinUnit {
                    buf: vec![0; half + 4 * m],
                    pre: 0,
                    len: half,
                    post: 4 * m,
                    marks: vec![0, half],
                    voiced: false,
                };
                push(unit, ends_run, &mut out, &mut lens);
                units += 1;
            }
        }
        ends.push(units);
    }
    if let Some((p, _)) = prev {
        out.extend_from_slice(&p.buf[p.pre..p.pre + p.len]);
        lens.push(p.len);
    }
    (out, lens, ends)
}

/// A 16-bit mono PCM WAV of the rendered speech.
pub fn to_wav_bytes(speech: &Speech) -> Vec<u8> {
    let data_len = speech.samples.len() * 2;
    let mut out = Vec::with_capacity(44 + data_len);
    let rate = speech.rate;
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_len as u32).to_le_bytes());
    for s in &speech.samples {
        // round like the original's 16-bit output, so a replay can match it exactly
        let v = (s * 32768.0).round().clamp(-32768.0, 32767.0) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// Peak envelope for the waveform display, `buckets` values in 0..1.
pub fn envelope(speech: &Speech, buckets: usize) -> Vec<f32> {
    if buckets == 0 || speech.samples.is_empty() {
        return Vec::new();
    }
    let chunk = (speech.samples.len() / buckets).max(1);
    (0..buckets)
        .map(|b| {
            let from = (b * chunk).min(speech.samples.len());
            let to = ((b + 1) * chunk).min(speech.samples.len());
            speech.samples[from..to].iter().fold(0.0f32, |a, s| a.max(s.abs())).min(1.0)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::g2p::{analyse, Language};

    /// Two recorded sentences: "# a-b c @" and "# c d @".
    fn fake_db() -> VoiceDatabase {
        crate::loquendo::parse_sde(
            "14,4\nN=2,9\nU=# a-b c @\nL# !\" #- n\nR# #- n\nL$ #- n\nR$ #- n\nL% #- n\nR% #- n\n\
             U=# c d @\nL# !#- #- n\nR# #- n\nL$ #- n\nR$ #- n\n",
            crate::loquendo::VOICE_SAMPLE_RATE,
        )
    }

    fn targets(syms: &[&str]) -> Vec<Target> {
        syms.iter()
            .map(|s| Target {
                sym: s.to_string(),
                word_start: false,
            })
            .collect()
    }

    #[test]
    fn a_recorded_run_is_taken_whole() {
        let db = fake_db();
        let p = select_like_original(&db, &targets(&["a", "b", "c"]), "@", None, 0);
        assert_eq!(p.pieces.len(), 1);
        assert_eq!(p.pieces[0].covers, ["a", "b", "c"]);
        assert_eq!(p.pieces[0].from, db.units[0].phones[1].start);
        assert_eq!(p.pieces[0].to, db.units[0].phones[3].end);
    }

    #[test]
    fn new_sequences_join_in_mid_phone() {
        let db = fake_db();
        // b-c from sentence 0, c-d from sentence 1: joined in the middle of c
        let p = select_like_original(&db, &targets(&["b", "c", "d"]), "@", None, 0);
        assert_eq!(p.pieces.len(), 2);
        assert_eq!(p.pieces[0].to, db.units[0].phones[3].mid);
        assert_eq!(p.pieces[1].from, db.units[1].phones[1].mid);
        assert!(p.missing.is_empty());
    }

    #[test]
    fn unknown_sounds_are_reported() {
        let db = fake_db();
        let p = select_like_original(&db, &targets(&["a", "zz"]), "@", None, 0);
        // the unrecorded sound is reported and left out; the rest is said
        assert_eq!(p.missing, ["zz"]);
        assert_eq!(p.pieces.len(), 1);
    }

    #[test]
    fn phrases_split_at_punctuation_only() {
        let ph = from_tokens(&analyse("um dois, três?", &crate::g2p::Frontend::rules(Language::Portuguese)));
        assert_eq!(ph.len(), 2);
        assert!(!ph[0].full_stop && ph[1].mood == Some('I'), "comma keeps the tune open, ? makes a question");
    }

    #[test]
    fn transcriptions_round_trip_in_the_engines_notation() {
        // as the original engine writes it: words joined by '-', a trailing '-'
        let p = parse_transcription("i l-tt r `E n o-");
        assert_eq!(p.len(), 1);
        let syms: Vec<&str> = p[0].phones.iter().map(|t| t.sym.as_str()).collect();
        assert_eq!(syms, ["i", "l", "tt", "r", "`E", "n", "o"]);
        assert!(p[0].phones[0].word_start && p[0].phones[2].word_start && !p[0].phones[3].word_start);
        let text = "a-b c #120 d-e @500?";
        let q = parse_transcription(text);
        assert_eq!(q.len(), 2);
        assert_eq!((q[0].pause_ms, q[0].full_stop, q[1].mood), (120, false, Some('I')));
        assert_eq!(format_transcription(&q), text);
    }
}
