//! LesElaboraParola (0x10009850): the records for one word token, made by
//! the module from its dictionary, its compound splitter and its
//! morphological analyser (`morph.rs`).

use super::super::common::{search_compact, Pe, Word};
use super::morph::Analyser;
use super::{set_lexinfo, German, Morpheme, UPPER, VOWEL};

/// `strpbrk` set: an apostrophe makes a word pronounceable (0x10028078).
const APOSTROPHES: u32 = 0x10028078;
/// Vowels a pronounceable word may have (0x10028068).
const PLAIN_VOWELS: u32 = 0x10028068;
/// Vowels an accent mark may follow (0x10027dac).
const ACCENTABLE: u32 = 0x10027dac;
/// The empty string, the dictionary's "no value" (0x10026b34).
const EMPTY: u32 = 0x10026b34;
/// Compound parts: a compact sorted string list (0x1002a958, 0xd8481 bytes)
/// searched with ELQSearchCompactString flags 1.
const PARTS: u32 = 0x1002a958;
const PARTS_SIZE: usize = 0xd8481;
/// Vowels that keep `Xe` from being an umlaut (0x10102f6c), and `q`
/// (0x1001e338) before `ue`.
const UMLAUT_BLOCKERS: u32 = 0x10102f6c;
const Q: u32 = 0x1001e338;
/// Character flag 0x80 (makes a word pronounceable) and 0x08 (consonant).
const FLAG80: u32 = 0x80;
const CONSONANT: u32 = 0x08;
/// A record's text is cut at this many characters (0x10008e40).
const MAX_TEXT: usize = 0x3f;

/// Dictionary value → record byte (strcmp chains: grammaticale 0x1000a470,
/// gennum 0x1000a9f0, tempomodo 0x1000ad30, gradoaccentuale 0x1000ae50):
/// (string address, value), then the value when none matches.
const CLASSES: (&[(u32, u8)], u8) = (
    &[
        (0x10102eb4, 0x20),
        (0x10102eac, 0x20),
        (0x10102ea4, 0x20),
        (0x10102e9c, 0x20),
        (0x10102e94, 0x2a),
        (0x10102e8c, 0x2c),
        (0x10102e84, 0x2b),
        (0x10102e78, 0x2f),
        (0x10102e74, 0x33),
        (0x10102e68, 0x33),
        (0x10102e64, 0x29),
        (0x10102e58, 0x29),
        (0x10102e4c, 0x20),
        (0x10102e38, 0x20),
        (0x10102e30, 0x15),
        (0x10102e28, 0x18),
        (0x1002a918, 0x18),
        (0x10102e20, 0x1f),
        (0x10102e18, 0x19),
        (0x10102e10, 0x19),
        (0x10102e08, 0x1e),
        (0x10102dfc, 0x19),
        (0x10102df4, 0x82),
        (0x10102dec, 0x82),
        (0x10102de4, 0x84),
        (0x10102ddc, 0x2e),
    ],
    0xc8,
);
const INFOS: (&[(u32, u8)], u8) = (
    &[
        (0x10102f20, 0x0d),
        (0x10102f18, 0x0b),
        (0x10102f10, 0x0e),
        (0x10102f08, 0x0c),
        (0x10102f00, 0x10),
        (0x10102ef8, 0xcf),
        (0x10102ef0, 0xd0),
        (0x10102ee8, 0xd1),
        (0x10102ee0, 0x0f),
        (0x10102ed8, 0xcc),
        (0x10102ed0, 0xcd),
        (0x10102ec8, 0xce),
        (0x10102ec4, 0xc9),
        (0x10102ec0, 0xca),
        (0x10102ebc, 0xcb),
    ],
    9,
);
const SUBS: (&[(u32, u8)], u8) = (&[(0x10102f58, 0x88), (0x10102f4c, 0x88), (0x10102f40, 0x8e), (0x10102f34, 0x8d), (0x10102f28, 0x8b)], 9);
const GRADES: (&[(u32, u8)], u8) = (&[(0x10102f68, 0), (0x10102f64, 3)], 0x0e);

/// `strchr(set, c) != NULL` (true for `c == 0`, the terminator).
fn strchr(pe: &Pe, set: u32, c: u8) -> bool {
    c == 0 || pe.cstr(set).contains(&c)
}

/// The byte at `i`, 0 past the end.
fn at(s: &[u8], i: usize) -> u8 {
    s.get(i).copied().unwrap_or(0)
}

/// A record as 0x10008e40 makes it (`+0xc` 0xffff, `+0x13` 0).
fn record(mut text: Vec<u8>, class: u8, sub: u8, info: u8, grade: u8, bound: u8) -> Word {
    text.truncate(MAX_TEXT);
    let mut f = [0u8; 24];
    f[0xc] = 0xff;
    f[0xd] = 0xff;
    f[0xe] = class;
    f[0xf] = sub;
    f[0x10] = info;
    f[0x11] = bound;
    f[0x12] = grade;
    Word { text, tag: None, f, data: Vec::new(), rows: Vec::new() }
}

impl German {
    /// LesElaboraParola (0x10009850): `token` the token's text, `idx` its
    /// index in the sentence, `flag408` the token entry's byte +0x408,
    /// `spell`/`mode` the engine's arguments 6 and 7.
    pub(crate) fn elabora_parola(&self, token: &[u8], idx: usize, flag408: u8, spell: u8, mode: u8) -> Vec<Word> {
        let mut out = Vec::new();
        let pron = self.pronunciabile_c(token, 0);
        if mode != 0 || (!pron && spell == 0 && flag408 == 0) {
            // spelled out: one record per character with a name
            for (i, &c) in token.iter().enumerate() {
                let mut name = self.symbol_name_mode(c, 1);
                if name.is_empty() {
                    continue;
                }
                self.normalizza_accento(&mut name);
                let sub = if i + 1 == token.len() { 0x84 } else { 0x83 };
                out.push(record(name, 0x15, sub, 9, 0x0e, 0x15));
            }
            return out;
        }
        let upper = self.is_upper(at(token, 0));
        if let Some(w) = self.from_grammar(token, token) {
            out.push(w);
            return out;
        }
        if self.phonetic.find(token).0 > 0 {
            out.push(self.analysed(token));
            return out;
        }
        if upper || idx == 0 {
            let mut key = token.to_vec();
            if upper {
                self.lower_all(&mut key);
            } else if let Some(c) = key.first_mut() {
                *c = self.upper(*c);
            }
            if let Some(w) = self.from_grammar(&key, token) {
                out.push(w);
                return out;
            }
            if self.phonetic.find(&key).0 > 0 {
                out.push(self.analysed(token));
                return out;
            }
        }
        if token.len() >= 0x40 {
            out.push(self.analysed(token));
            return out;
        }
        // compound parts (0x1000b8c0): the token is rewritten in place
        let mut word = token.to_vec();
        let mut offsets = [token.len(); 11];
        offsets[0] = 0;
        let mut count = 1usize;
        self.split(&mut word, &mut count, &mut offsets);
        if count > 10 {
            count = 1;
        }
        offsets[count] = word.len();
        for k in 1..=count {
            let part = &word[offsets[k - 1].min(word.len())..];
            let n = offsets[k].wrapping_sub(offsets[k - 1]);
            let part = part[..n.min(part.len())].to_vec();
            let mut w = self.analysed(&part);
            if count > 1 && k != count {
                w.f[0x11] = 4;
            }
            out.push(w);
        }
        out
    }

    /// The grammar table's first row for `key`, as a record of `text`.
    fn from_grammar(&self, key: &[u8], text: &[u8]) -> Option<Word> {
        let (n, row) = self.grammar.find(key);
        if n == 0 {
            return None;
        }
        let empty = self.s(EMPTY);
        let col = |c: usize| self.grammar.get(row, c).filter(|v| *v != empty);
        let conv = |c: usize, t: (&[(u32, u8)], u8), d: u8| col(c).map_or(d, |v| self.convert(v, t));
        let class = conv(2, CLASSES, 0x17);
        let sub = conv(3, SUBS, 200);
        let info = conv(4, INFOS, 200);
        let grade = conv(1, GRADES, 0x0e);
        Some(record(text.to_vec(), class, sub, info, grade, 0x15))
    }

    fn convert(&self, v: &[u8], (list, default): (&[(u32, u8)], u8)) -> u8 {
        list.iter().find(|&&(s, _)| self.s(s) == v).map_or(default, |&(_, r)| r)
    }

    /// 0x10008e00: lower-case every character.
    fn lower_all(&self, s: &mut [u8]) {
        for c in s {
            *c = self.lower(*c);
        }
    }

    /// 0x10009680: a record for `text` (class 0x17, sub 200) with the
    /// analyser's solutions on its spelling (lower case, ä ö ü ß → ae oe ue
    /// ss) as LexInfo, as many as 0x10008d70 keeps.
    fn analysed(&self, text: &[u8]) -> Word {
        let mut key = Vec::new();
        for &c in text.iter().take(0x3f) {
            let c = if self.flags(c) & UPPER != 0 { self.lower(c) } else { c };
            match c {
                0xf6 => key.extend_from_slice(b"oe"),
                0xfc => key.extend_from_slice(b"ue"),
                0xe4 => key.extend_from_slice(b"ae"),
                0xdf => key.extend_from_slice(b"ss"),
                _ => key.push(c),
            }
        }
        let sols = Analyser::new(&self.pe).analyse(&key);
        let keep = solutions(&sols);
        let mut w = record(text.to_vec(), 0x17, 200, 9, 0x0e, 0x15);
        set_lexinfo(&mut w, &sols[..keep]);
        w
    }

    /// LesPronunciabile (0x10003c60 → 0x10003a50); `c` its 4th argument
    /// (not 0: only an apostrophe, a flag-0x80 character or an unusual
    /// vowel makes the word pronounceable).
    pub(crate) fn pronunciabile_c(&self, s: &[u8], c: u8) -> bool {
        if s.iter().any(|c| self.s(APOSTROPHES).contains(c)) {
            return true;
        }
        let mut all_upper = true;
        let mut low = Vec::new();
        for &c in s.iter().take(0x3f) {
            let f = self.flags(c);
            if f & FLAG80 != 0 {
                return true;
            }
            if f & UPPER == 0 {
                all_upper = false;
            }
            if f & VOWEL != 0 && !strchr(&self.pe, PLAIN_VOWELS, c) {
                return true;
            }
            low.push(self.lower(c));
        }
        let n = low.len();
        let fl = |i: usize| self.flags(at(&low, i));
        if (n == 1 && (low[0] == b'j' || low[0] == b'y')) || c != 0 {
            return false;
        }
        if !all_upper {
            return self.has_vowel(&low);
        }
        match n {
            2 => false,
            3 => {
                if fl(0) & VOWEL != 0 && fl(2) & VOWEL != 0 && fl(1) & VOWEL == 0 {
                    return true;
                }
                fl(0) & CONSONANT != 0 && fl(2) & CONSONANT != 0 && fl(1) & VOWEL != 0
            }
            4 => {
                let mut i = 0;
                while i < 4 && fl(i) & VOWEL != 0 {
                    i += 1;
                }
                if i < 2 && fl(i) & CONSONANT != 0 && fl(i + 1) & CONSONANT != 0 && fl(i + 2) & CONSONANT != 0 {
                    return false;
                }
                self.has_vowel(&low)
            }
            _ => self.has_vowel(&low),
        }
    }

    /// LesGetSymbolName (0x100039e0 → 0x10003530): the name of character
    /// `c` as the export copies it (no normalisation; empty when none).
    /// `mode` (its 3rd argument) not 0 also names punctuation.
    pub(crate) fn symbol_name_mode(&self, c: u8, mode: u8) -> Vec<u8> {
        self.s(symbol_name(c, mode != 0)).to_vec()
    }

    /// 0x10003a20: any vowel.
    fn has_vowel(&self, s: &[u8]) -> bool {
        s.iter().any(|&c| self.flags(c) & VOWEL != 0)
    }

    /// LesNormalizzaAccento (0x10003410): an accent mark after a vowel
    /// accents it (grave 0x100031d0, acute 0x100032f0); elsewhere it stays
    /// an apostrophe before a vowel or at the end, else goes; then é → è,
    /// í → ì.
    pub(crate) fn normalizza_accento(&self, s: &mut Vec<u8>) {
        let mut out: Vec<u8> = Vec::new();
        for i in 0..s.len() {
            let c = s[i];
            if !matches!(c, 0x60 | 0x27 | 0x92 | 0xb4) {
                out.push(c);
                continue;
            }
            if i > 0 && strchr(&self.pe, ACCENTABLE, self.lower(s[i - 1])) {
                let prev = s[i - 1];
                if let Some(o) = out.last_mut() {
                    *o = if c == 0x60 { grave(prev) } else { acute(prev) };
                }
            } else {
                let next = at(s, i + 1);
                if next == 0 || strchr(&self.pe, ACCENTABLE, self.lower(next)) {
                    out.push(b'\'');
                }
            }
        }
        for c in &mut out {
            *c = match *c {
                0xe9 => 0xe8,
                0xed => 0xec,
                c => c,
            };
        }
        *s = out;
    }

    fn in_parts(&self, s: &[u8]) -> bool {
        let blob = self.pe.at(PARTS);
        search_compact(s, &blob[..PARTS_SIZE.min(blob.len())], 1, 0).is_some()
    }

    /// 0x1000b8c0: split `word` into compound parts, rewriting it (lower
    /// case, parts joined) and recording each part's start.
    fn split(&self, word: &mut Vec<u8>, count: &mut usize, offsets: &mut [usize; 11]) {
        if word.is_empty() {
            return;
        }
        let mut out = Vec::new();
        let mut ok = self.compound(word, &mut out);
        let mut source = None;
        if !ok {
            out.clear();
            let u = self.umlauts(word);
            ok = self.compound(&u, &mut out);
            source = Some(u);
        }
        if !ok {
            return;
        }
        if out.is_empty() {
            if let Some(u) = source {
                *word = u;
            }
            return;
        }
        let mut w = Vec::new();
        for &c in &out {
            if c == b'+' {
                *count += 1;
                if *count - 1 < offsets.len() {
                    offsets[*count - 1] = w.len();
                }
            } else {
                w.push(c);
            }
        }
        *word = w;
    }

    /// 0x1000b700: `ae oe ue` (not between other vowels; `ue` not after
    /// `q`) → ä ö ü, `àe òe ùe` → á ó ú, ß → ss.
    fn umlauts(&self, s: &[u8]) -> Vec<u8> {
        let mut d = Vec::new();
        let mut i = 0usize;
        while at(s, i) != 0 && d.len() < 0x3e {
            let c = s[i];
            d.push(c);
            let j = d.len() - 1;
            if at(s, i + 1) == b'e' && !strchr(&self.pe, UMLAUT_BLOCKERS, at(s, i + 2)) && (i == 0 || !strchr(&self.pe, UMLAUT_BLOCKERS, s[i - 1])) {
                let after_q = i > 0 && strchr(&self.pe, Q, s[i - 1]);
                let r = match c {
                    b'a' => Some(0xe4),
                    0xe0 => Some(0xe1),
                    b'o' => Some(0xf6),
                    0xf2 => Some(0xf3),
                    b'u' if !after_q => Some(0xfc),
                    0xf9 if !after_q => Some(0xfa),
                    _ => None,
                };
                if let Some(r) = r {
                    d[j] = r;
                    i += 1;
                }
            }
            if at(s, i) == 0xdf {
                d[j] = b's';
                d.push(b's');
            }
            i += 1;
        }
        d
    }

    /// 0x1000afc0: whether `word` (lower-cased, 63 characters at most) is a
    /// listed part or splits into listed parts; `out` gets the split as
    /// `part+part…` (empty when the word itself is listed).
    fn compound(&self, word: &[u8], out: &mut Vec<u8>) -> bool {
        let mut w: Vec<u8> = word.iter().take(0x3f).map(|&c| self.lower(c)).collect();
        if let Some(p) = w.iter().position(|&c| c == 0) {
            w.truncate(p);
        }
        if self.in_parts(&w) {
            return true;
        }
        let mut c = w.clone();
        let (mut l, mut r) = (Vec::new(), Vec::new());
        let mut found = false;
        let mut retried = false;
        let mut phase2 = c.len() < 6;
        if !phase2 {
            // longest listed right part first (0x1000b0b0)
            loop {
                let n = c.len() as i32;
                let mut p = 3;
                while p < n - 2 {
                    l = c[..p as usize].to_vec();
                    r = c[p as usize..].to_vec();
                    if self.in_parts(&r) || self.in_parts(&strip(&r)) {
                        found = true;
                        break;
                    }
                    p += 1;
                }
                if !found {
                    phase2 = true;
                    break;
                }
                let x = strip(&l);
                let mut t = vec![b'+'];
                t.extend_from_slice(&r);
                t.extend_from_slice(out);
                *out = t;
                if self.in_parts(&l) || self.in_parts(&x) {
                    let mut t = l.clone();
                    t.extend_from_slice(out);
                    *out = t;
                    return found;
                }
                if l.len() < 6 && !retried {
                    l.extend_from_slice(&r);
                    c = l.clone();
                    retried = true;
                } else {
                    c = l.clone();
                    found = false;
                }
                if c.len() < 6 {
                    phase2 = true;
                    break;
                }
            }
        }
        if !phase2 {
            return false;
        }
        // shortest right part first (0x1000b3ee)
        out.clear();
        c = w.clone();
        while c.len() >= 6 {
            let n = c.len() as i32;
            let mut p = n - 3;
            while p > 3 {
                l = c[..p as usize].to_vec();
                r = c[p as usize..].to_vec();
                if self.in_parts(&r) || self.in_parts(&strip(&r)) {
                    found = true;
                    break;
                }
                p -= 1;
            }
            if !found {
                return false;
            }
            let x = strip(&l);
            let mut t = vec![b'+'];
            t.extend_from_slice(&r);
            t.extend_from_slice(out);
            *out = t;
            if self.in_parts(&l) || self.in_parts(&x) {
                let mut t = l.clone();
                t.extend_from_slice(out);
                *out = t;
                return found;
            }
            c = l.clone();
            found = false;
        }
        false
    }
}

/// 0x1000aed0: the part without a flexion ending (-ens, -en, -er, -es,
/// -s, -n); empty when it has none.
fn strip(s: &[u8]) -> Vec<u8> {
    let n = s.len();
    if n < 2 {
        return Vec::new();
    }
    let cut = if n > 2 && s.ends_with(b"ens") {
        3
    } else if s[n - 2] == b'e' && matches!(s[n - 1], b'n' | b'r' | b's') {
        2
    } else if matches!(s[n - 1], b's' | b'n') {
        1
    } else {
        return Vec::new();
    };
    s[..n - cut].to_vec()
}

/// 0x10008d70: how many solutions to keep: up to the first with more
/// morphemes than the first.
fn solutions(sols: &[Vec<Morpheme>]) -> usize {
    let Some(first) = sols.first() else { return 0 };
    sols.iter().skip(1).position(|s| s.len() > first.len()).map_or(sols.len(), |i| i + 1)
}

/// 0x10003530: the name of a character (its address; 0x10026b34 "" when
/// none); punctuation only when `punct` (AL), `&` either way.
fn symbol_name(c: u8, punct: bool) -> u32 {
    if !punct {
        match c {
            0x26 => return 0x10028028,
            0x21 | 0x22 | 0x27..=0x2a | 0x2c..=0x2f | 0x3a..=0x3c | 0x3e | 0x3f | 0x40 | 0x5b..=0x5f | 0x60 | 0x7b | 0x7d | 0x7e => return EMPTY,
            0x93 | 0x94 | 0xa1 | 0xab | 0xb0 | 0xba | 0xbb | 0xbf => return EMPTY,
            _ => {}
        }
    }
    match c {
        0xb0 | 0xba => 0x10027db4,
        0x80 => 0x10027dbc,
        0xa5 => 0x10027dc4,
        0xdf => 0x10027dd4,
        0xbe => 0x10027ddc,
        0xbd => 0x10027de8,
        0xbc => 0x10027df0,
        0xb5 => 0x10027dfc,
        0xb1 => 0x10027e00,
        0xae => 0x10027e0c,
        0xa9 => 0x10027e28,
        0xa7 => 0x10027e34,
        0xa3 => 0x10027e40,
        0x5a | 0x7a => 0x10027e50,
        0x59 | 0x79 | 0xdd | 0xfd | 0xff => 0x10027e58,
        0x58 | 0x78 => 0x10027e60,
        0x57 | 0x77 => 0x10027e64,
        0x56 | 0x76 => 0x10027e68,
        0xdc | 0xfc => 0x10027e6c,
        0x55 | 0x75 | 0xd9 | 0xda | 0xdb | 0xf9 | 0xfa | 0xfb => 0x10027e70,
        0x54 | 0x74 => 0x10027e74,
        0x53 | 0x73 => 0x10027e78,
        0x52 | 0x72 => 0x10027e7c,
        0x51 | 0x71 => 0x10027e80,
        0x50 | 0x70 => 0x10027e84,
        0xd6 | 0xf6 => 0x10027e88,
        0x4f | 0x6f | 0xd2 | 0xd3 | 0xd4 | 0xd5 | 0xf2 | 0xf3 | 0xf4 | 0xf5 => 0x10027e8c,
        0x4e | 0x6e | 0xd1 | 0xf1 => 0x10027e90,
        0x4d | 0x6d => 0x10027e94,
        0x4c | 0x6c => 0x10027e98,
        0x4b | 0x6b => 0x10027e9c,
        0x4a | 0x6a => 0x10027ea0,
        0x49 | 0x69 | 0xcc | 0xcd | 0xce | 0xcf | 0xec | 0xed | 0xee | 0xef => 0x10027ea8,
        0x48 | 0x68 => 0x10027eac,
        0x47 | 0x67 => 0x10027eb0,
        0x46 | 0x66 => 0x10027eb4,
        0x45 | 0x65 | 0xc6 | 0xc8 | 0xc9 | 0xca | 0xcb | 0xe6 | 0xe8 | 0xe9 | 0xea | 0xeb => 0x10027eb8,
        0x44 | 0x64 => 0x10027ebc,
        0x43 | 0x63 | 0xc7 | 0xe7 => 0x10027ec0,
        0x42 | 0x62 => 0x10027ec4,
        0xc4 | 0xe4 => 0x10027ec8,
        0x41 | 0x61 | 0xc0 | 0xc1 | 0xc2 | 0xc3 | 0xc5 | 0xe0 | 0xe1 | 0xe2 | 0xe3 | 0xe5 => 0x10027ecc,
        0x3c => 0x10027ed0,
        0x3e => 0x10027edc,
        0x2e => 0x10027ee8,
        0x21 | 0xa1 => 0x10027ef0,
        0x3f | 0xbf => 0x10027f04,
        0x2f => 0x10027f14,
        0x2c => 0x10027f24,
        0x3b => 0x10027f2c,
        0x3a => 0x10027f38,
        0x22 | 0x93 | 0x94 | 0xab | 0xbb => 0x10027f44,
        0x5c => 0x10027f58,
        0x5d => 0x10027f70,
        0x5b => 0x10027f80,
        0x7d => 0x10027f94,
        0x7b => 0x10027fac,
        0x3d => 0x10027fc4,
        0x2b => 0x10027fd8,
        0x5f => 0x10027fe0,
        0x2d => 0x10027ff0,
        0x29 => 0x10027ffc,
        0x28 => 0x1002800c,
        0x2a => 0x1002801c,
        0x26 => 0x1002802c,
        0x5e => 0x10028040,
        0x25 => 0x1002804c,
        0x24 => 0x10028054,
        0x40 => 0x1002805c,
        0x27 | 0x60 | 0x7e => 0x10028060,
        _ => EMPTY,
    }
}

/// 0x100031d0: a vowel with a grave accent.
fn grave(c: u8) -> u8 {
    match c {
        0x41 => 0xc0,
        0x45 => 0xc8,
        0x49 | 0x4a => 0xcc,
        0x4f => 0xd2,
        0x59 => 0xdd,
        0x61 => 0xe0,
        0x65 => 0xe8,
        0x69 | 0x6a => 0xec,
        0x6f => 0xf2,
        0x79 => 0xfd,
        0xdc | 0x55 => 0xd9,
        0xef => 0xed,
        0xfc | 0x75 => 0xf9,
        c => c,
    }
}

/// 0x100032f0: a vowel with an acute accent.
fn acute(c: u8) -> u8 {
    match c {
        0x41 => 0xc1,
        0x45 => 0xc9,
        0x49 | 0x4a => 0xcd,
        0x4f => 0xd3,
        0x59 => 0xdd,
        0x61 => 0xe1,
        0x65 => 0xe9,
        0x6f => 0xf3,
        0x79 => 0xfd,
        0xdc | 0x55 => 0xda,
        0xef | 0x69 | 0x6a => 0xed,
        0xfc | 0x75 => 0xfa,
        c => c,
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::*;
    use super::super::lexinfo;

    /// Every traced LesElaboraParola call: the records (text, bytes
    /// 0x0c..0x14) and their LexInfo.
    #[test]
    #[ignore]
    fn elabora_parola_matches_the_module() {
        let Some(de) = german() else { return };
        let (mut n, mut bad) = (0, 0);
        for line in lines("ep.txt") {
            let mut cols = line.split('\t');
            let head: Vec<&str> = cols.next().unwrap().split(' ').collect();
            let (recs, lex) = (cols.next().unwrap_or(""), cols.next().unwrap_or(""));
            let want = records_with_lex(recs, lex);
            let tok = unhex(head[0]);
            let p: Vec<u8> = head[1..].iter().map(|v| v.parse::<usize>().unwrap() as u8).collect();
            let got = de.elabora_parola(&tok, head[1].parse().unwrap(), p[2], p[3], p[4]);
            n += 1;
            let same = got.len() == want.len()
                && got.iter().zip(&want).all(|(g, w)| g.text == w.text && g.f[0xc..0x14] == w.f[0xc..0x14] && lexinfo(g) == lexinfo(w));
            if !same {
                bad += 1;
                if bad <= 20 {
                    eprintln!("{}: want {:?}\n  got  {:?}", String::from_utf8_lossy(&tok), want.iter().map(show).collect::<Vec<_>>(), got.iter().map(show).collect::<Vec<_>>());
                }
            }
        }
        eprintln!("LesElaboraParola: {bad} of {n} calls differ");
        assert_eq!(bad, 0);
    }
}
