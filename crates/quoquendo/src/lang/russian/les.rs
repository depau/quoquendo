//! The LES side of LTTS7Russian.dll after the records exist: the choice
//! between a word's analyses and its class (LesAnalizza 0x10008f90), the
//! stressed vowel (LesAccenta 0x10001280), the prosodic boundaries
//! (LesConfiniProsodici 0x10003d80, `confini.rs`) and the stress grades
//! (LesGradiAccentuali 0x10002d60).

use super::{Lex, Russian, ACCENTED, VOWEL};
use crate::lang::common::{field::*, Endings, Word};
use crate::lang::wordanalyzer::{morph, Seg};

/// LesAccenta's ending table for words the analyser does not know
/// (0x100011c0: 0xf9a entries of `{ reversed ending, exception list,
/// code }`, searched as Italian's, `common::lookup`).
const ENDINGS: Endings = Endings { at: 0x10059cc8, count: 0xf9a };
/// The stress of a root class: `{ i16 vowel, i16 movement pattern }` per
/// class (0x1008e068).
const ROOT_STRESS: u32 = 0x1008e068;
/// The movement patterns: 12-byte `{ i16 code, count +4, entries +8 }`,
/// entries `{ i16 suffix class, i8 shift }` (0x1008e8d8, 0x10009100).
const MOVEMENT: u32 = 0x1008e8d8;
/// Latin words read as Russian ones, for spelled records (0x10119b70:
/// 12-byte `{ word, reading, flag }`, their count at 0x10119ea0;
/// 0x100092c0).
const BRANDS: u32 = 0x10119b70;
const BRAND_COUNT: u32 = 0x10119ea0;

/// `не` (0x1001d2f0), `нет` (0x1001d2c0), `на` (0x1001d330), `быть`
/// (0x1009d830), `был` (0x10067d14), `было` (0x10067d0c), `были`
/// (0x10067d04), `была` (0x10067cfc), `плацу` (0x1009d828).
const NE: u32 = 0x1001d2f0;
const NET: u32 = 0x1001d2c0;
const NA: u32 = 0x1001d330;
const BYT: u32 = 0x1009d830;
const BYL: u32 = 0x10067d14;
const BYLO: u32 = 0x10067d0c;
const BYLI: u32 = 0x10067d04;
const BYLA: u32 = 0x10067cfc;
const PLATSU: u32 = 0x1009d828;

/// The punctuation classes the analyser gives (feature 6): `$ ' ( ) . -`.
fn punct(c: u8) -> bool {
    matches!(c, b'$' | b'\'' | b'(' | b')' | b'.' | b'-')
}

// --- stress helpers -----------------------------------------------------------

/// 0x10009000: the index of the `k`-th vowel (`ь ъ` are not) from the
/// start (`k` > 0) or the end (`k` < 0); -1 when there is none (from the
/// end, the last one found).
pub(super) fn vowel_at(l: &Russian, s: &[u8], k: i32) -> i32 {
    let len = s.len() as i32;
    if k == 0 || len == 0 {
        return -1;
    }
    let vowel = |c: u8| l.has(c, VOWEL) && c != 0xfc && c != 0xfa;
    let mut n = 0;
    let mut found = -1;
    if k < 0 {
        let mut i = len;
        loop {
            i -= 1;
            if i < 0 {
                return found;
            }
            if vowel(s[i as usize]) {
                n += 1;
                found = i;
                if n >= k.abs() {
                    return i;
                }
            }
        }
    }
    for i in 0..len {
        if vowel(s[i as usize]) {
            n += 1;
            found = i;
            if n >= k {
                return i;
            }
        }
    }
    found
}

/// 0x10009400: `s` with `^` before index `pos`, when it fits in `size`.
pub(super) fn mark_stress(s: &[u8], pos: i32, size: i32) -> Vec<u8> {
    let len = s.len() as i32;
    if pos >= 0 && pos < size && pos < len && len < size - 1 {
        let mut o = s[..pos as usize].to_vec();
        o.push(b'^');
        o.extend_from_slice(&s[pos as usize..]);
        o
    } else {
        s.to_vec()
    }
}

/// 0x100093c0: the spelling marks its own stress (`^`, `ё` or an accented
/// letter).
pub(super) fn has_stress_mark(l: &Russian, s: &[u8]) -> bool {
    s.iter().any(|&c| c == b'^' || c == 0xb8 || l.has(c, ACCENTED))
}

/// 0x10001460: the stressed vowel of a word the analyser does not know,
/// counted from the end (-1 the last): one vowel or none, -1 / -2; else
/// the ending table (`common::lookup` on the reversed word, words of at
/// most 0x3f letters), 0 → -2.
fn ending_stress(l: &Russian, s: &[u8]) -> i32 {
    let n = s.iter().filter(|&&c| l.has(c, VOWEL)).count() as i16;
    let code = if n > 1 {
        if s.len() > 0x3f {
            return 0;
        }
        let rev: Vec<u8> = s.iter().rev().copied().collect();
        crate::lang::common::lookup(&l.pe, ENDINGS, &rev) as i16
    } else {
        n
    };
    match -(code as i32) {
        0 => -2,
        v => v,
    }
}

/// 0x10009170: a word in Latin letters in Cyrillic ones (`x` → `кс`).
fn transliterate(l: &Russian, s: &[u8], size: usize) -> Vec<u8> {
    let mut o = Vec::new();
    for &c in s {
        if o.len() >= size - 1 {
            break;
        }
        match l.lower(c) {
            b'a' | b'o' => o.push(0xe0),
            b'b' => o.push(0xe1),
            b'c' | b'k' | b'q' => o.push(0xea),
            b'd' => o.push(0xe4),
            b'e' => o.push(0xfd),
            b'f' => o.push(0xf4),
            b'g' => o.push(0xe3),
            b'h' => o.push(0xf5),
            b'i' => o.push(0xe8),
            b'j' => o.push(0xe9),
            b'l' => o.push(0xeb),
            b'm' => o.push(0xec),
            b'n' => o.push(0xed),
            b'p' => o.push(0xef),
            b'r' => o.push(0xf0),
            b't' => o.push(0xf2),
            b'u' => o.push(0xf3),
            b'v' | b'w' => o.push(0xe2),
            b'x' => o.extend([0xea, 0xf1]),
            b's' => o.push(0xf1),
            b'y' => o.push(0xff),
            b'z' => o.push(0xe7),
            low => o.push(low),
        }
    }
    o
}

impl Russian {
    /// 0x100092c0: a spelled record's (subclass 0x83/0x84) word read as a
    /// Russian one, and the entry's flag.
    fn brand(&self, w: &Word) -> Option<(Vec<u8>, u8)> {
        if !matches!(w.f[SUB], 0x84 | 0x83) {
            return None;
        }
        let n = self.pe.u32(BRAND_COUNT) as i32;
        let (mut lo, mut hi) = (0i32, n - 1);
        while lo <= hi {
            let mid = (lo + hi) / 2;
            let e = BRANDS + 12 * mid as u32;
            match w.text.as_slice().cmp(self.s(self.pe.u32(e))) {
                std::cmp::Ordering::Less => hi = mid - 1,
                std::cmp::Ordering::Greater => lo = mid + 1,
                std::cmp::Ordering::Equal => {
                    let r = self.s(self.pe.u32(e + 4));
                    if r.len() < 0x40 {
                        return Some((r.to_vec(), self.pe.u8(e + 8)));
                    }
                    return None;
                }
            }
        }
        None
    }

    /// 0x10009100: the shift movement pattern `pattern` gives a suffix of
    /// class `cls`.
    fn movement(&self, pattern: i32, cls: i16) -> i32 {
        let e = (MOVEMENT as i64 + pattern as i64 * 12) as u32;
        if self.pe.u16(e) as i16 as i32 != pattern {
            return 0;
        }
        let n = self.pe.u32(e + 4) as i32;
        let ent = self.pe.u32(e + 8);
        for k in 0..n.max(0) as u32 {
            if self.pe.u16(ent + 4 * k) as i16 == cls {
                return self.pe.u8(ent + 4 * k + 2) as i8 as i32;
            }
        }
        0
    }

    /// A morph's spelling for the stress: its stressed form when it has
    /// one (the LexInfo copy's +0xc), else its spelling.
    fn morph_text(&self, va: u32) -> Vec<u8> {
        let m = morph(&self.pe, va);
        match m.gloss() {
            Some(g) => g.to_vec(),
            None => m.spelling().to_vec(),
        }
    }

    // --- LesAnalizza --------------------------------------------------------------

    /// The morph features of an analysis (0x10007e70): from its last
    /// segment back, the first with flag 1 (before any with flag 3) and
    /// the first with flag 3; their morph ids.
    fn feature_ids(&self, an: &[Seg]) -> (Option<i32>, Option<i32>) {
        let (mut t1, mut t3) = (None, None);
        for s in an.iter().rev() {
            let m = morph(&self.pe, s.morph);
            match m.flag() {
                1 if t1.is_none() => t1 = Some(m.id()),
                3 => {
                    t3 = Some(m.id());
                    break;
                }
                _ => {}
            }
        }
        (t1, t3)
    }

    /// 0x10007e70: feature `key` of an analysis, `_` when it has none.
    fn an_feature(&self, an: Option<&[Seg]>, key: u8) -> u8 {
        let Some(an) = an.filter(|a| !a.is_empty()) else { return b'_' };
        let (t1, t3) = self.feature_ids(an);
        t3.and_then(|id| self.feature(id, key)).or_else(|| t1.and_then(|id| self.feature(id, key))).unwrap_or(b'_')
    }

    /// 0x10007f60: a word's part of speech (feature 6 of the analysis in
    /// use).
    fn pos(&self, w: &Word) -> u8 {
        self.an_feature(Lex::of(w).current(), 6)
    }

    /// 0x10008300: make the analysis of `w` matching most of `pattern`'s
    /// `{count, (key, value)*}` features the one in use (from the second
    /// on, the first that matches them all); `pos` its new part of speech.
    fn choose_by(&self, w: &mut Word, pattern: u32, pos: &mut u8) {
        let mut lex = Lex::of(w);
        if w.data.is_empty() {
            return;
        }
        let n = self.pe.u8(pattern) as u32;
        let pairs = self.pe.u32(pattern + 4);
        let mut best = lex.cur as i32;
        let mut score = 0;
        for k in 0..lex.an.slots.len() {
            if lex.an.slots.len() < 2 {
                break;
            }
            let an = lex.an.ranked(k);
            let s = (0..n)
                .filter(|&q| {
                    let v = self.an_feature(an, self.pe.u8(pairs + 2 * q));
                    v < 0x5f && v == self.pe.u8(pairs + 2 * q + 1)
                })
                .count() as i32;
            if score < s {
                score = s;
                best = k as i32;
                if n as i32 <= s {
                    break;
                }
            }
        }
        if best > 0 && best as u32 != lex.cur {
            lex.cur = best as u32;
            lex.store(w);
            *pos = self.pos(w);
        }
    }

    /// 0x10008230: make the analysis of `w` agreeing with `other`'s in use
    /// on most of `pattern`'s `{count, keys*}` features the one in use;
    /// `pos` its new part of speech.
    fn agree_with(&self, w: &mut Word, other: &Word, pattern: u32, pos: &mut u8) {
        let mut lex = Lex::of(w);
        if w.data.is_empty() || other.data.is_empty() {
            return;
        }
        let olex = Lex::of(other);
        let oan = olex.current();
        let n = self.pe.u32(pattern) as i32;
        let keys = self.pe.u32(pattern + 4);
        let mut best = lex.cur as i32;
        let mut score = 0;
        for k in 0..lex.an.slots.len() {
            if lex.an.slots.len() < 2 {
                break;
            }
            let an = lex.an.ranked(k);
            let s = (0..n.max(0) as u32)
                .filter(|&q| {
                    let key = self.pe.u8(keys + q);
                    let v = self.an_feature(an, key);
                    v < 0x5f && v == self.an_feature(oan, key)
                })
                .count() as i32;
            if score < s {
                score = s;
                best = k as i32;
                if n <= s {
                    break;
                }
            }
        }
        if best > 0 && best as u32 != lex.cur {
            lex.cur = best as u32;
            lex.store(w);
            *pos = self.pos(w);
        }
    }

    /// 0x10008070: the pattern a word after a preposition-like word (`7`)
    /// asks of the next: 29 `{ word, pattern }` pairs at 0x100a2f38 (count
    /// 0x100a2f34).
    fn word_pattern(&self, w: &Word) -> Option<u32> {
        let n = self.pe.u32(0x100a2f34);
        (0..n).find(|&k| w.text.as_slice() == self.s(self.pe.u32(0x100a2f38 + 8 * k))).map(|k| self.pe.u32(0x100a2f3c + 8 * k))
    }

    /// 0x100080d0: the pattern a word of government feature 7 asks of the
    /// next.
    fn government(&self, w: &Word) -> Option<u32> {
        let at = match self.an_feature(Lex::of(w).current(), 7) {
            b'G' => 0x100a3134,
            b'T' => 0x100a313c,
            b'A' => 0x100a3144,
            b'K' => 0x100a314c,
            b'D' => 0x100a3154,
            b'Q' => 0x100a315c,
            b'>' => 0x100a3164,
            b'J' => 0x100a316c,
            b'C' => 0x100a3174,
            b'P' => 0x100a317c,
            b'=' => 0x100a3184,
            b'I' => 0x100a318c,
            b'B' => 0x100a3194,
            b'O' => 0x100a319c,
            b'<' => 0x100a31a4,
            b'L' => 0x100a31b4,
            b'E' => 0x100a31bc,
            b'R' => 0x100a31c4,
            b'?' => 0x100a31cc,
            b'M' => 0x100a31d4,
            b'F' => 0x100a31dc,
            b'S' => 0x100a31e4,
            b'@' => 0x100a31ec,
            b'H' => 0x100a31f4,
            b'N' => 0x100a31fc,
            _ => return None,
        };
        Some(at)
    }

    fn is(&self, w: &Word, va: u32) -> bool {
        w.text.as_slice() == self.s(va)
    }

    /// `LesAnalizza` (0x10008f90): 0x100083c0 picks among each word's
    /// analyses by its neighbours' parts of speech, then 0x10008e10 sets
    /// the class, subclass and information from the analysis in use.
    pub(super) fn analizza(&self, w: &mut [Word]) {
        self.disambigua(w);
        self.classes(w);
    }

    /// 0x100083c0.
    fn disambigua(&self, w: &mut [Word]) {
        let n = w.len();
        let (mut c, mut p, mut nx_pos) = (b'_', b'_', b'_');
        let mut cur: Option<usize> = None;
        let mut prev: Option<usize>;
        let mut prev_v: Option<usize> = None;
        for j in 0..n {
            if j > 0 {
                p = c;
                prev_v = cur;
            }
            prev = prev_v;
            match cur {
                None => {
                    cur = Some(j);
                    c = self.pos(&w[j]);
                }
                Some(_) => {
                    cur = Some(j);
                    c = nx_pos;
                }
            }
            let next = if j + 1 < n {
                nx_pos = self.pos(&w[j + 1]);
                Some(j + 1)
            } else {
                nx_pos = b'_';
                None
            };
            let cu = j;
            // A: after a form of быть
            if p != b'^' {
                if let Some(pv) = prev {
                    if [BYL, BYLO, BYLI, BYLA].iter().any(|&s| self.is(&w[pv], s)) {
                        self.choose_by(&mut w[pv], 0x100a2f1c, &mut p);
                    }
                }
            }
            let mut skip_to_f = false;
            if p == 6 {
                match prev {
                    Some(pv) if self.is(&w[pv], 0x10067cf8) => {
                        // все + …
                        if c == b'/' {
                            let pat = if [0x10067cf0, 0x10067ce8, 0x10065e74].iter().any(|&s| self.is(&w[cu], s)) { 0x100a31ac } else { 0x100a31a4 };
                            self.choose_by(&mut w[pv], pat, &mut p);
                        } else if c == 7 && self.is(&w[cu], 0x10065d20) {
                            self.choose_by(&mut w[pv], 0x100a313c, &mut p);
                        } else {
                            let other = if c == 7 { next } else { Some(cu) };
                            if let Some(o) = other {
                                let ow = w[o].clone();
                                self.agree_with(&mut w[pv], &ow, 0x100a2e60, &mut p);
                            }
                        }
                        if p != 6 {
                            self.block_c(w, prev, c, &mut p);
                        }
                    }
                    _ => skip_to_f = true,
                }
            } else {
                self.block_c(w, prev, c, &mut p);
            }
            if !skip_to_f {
                // D: punctuation around
                if punct(p) {
                    if let Some(pv) = prev {
                        self.choose_by(&mut w[pv], 0x100a2f14, &mut p);
                    }
                }
                let mut to_f = false;
                if punct(p) && punct(c) {
                    self.choose_by(&mut w[cu], 0x100a2f1c, &mut c);
                }
                // E: government
                if p == b'/' {
                    if c == b'7' {
                        to_f = true;
                    } else if let Some(pat) = prev.and_then(|pv| self.government(&w[pv])) {
                        self.choose_by(&mut w[cu], pat, &mut c);
                    }
                } else if p == b'7' {
                    if c == b'7' {
                        to_f = true;
                    } else if c == 6 && (self.is(&w[cu], 0x10067c9c) || self.is(&w[cu], 0x10067c94)) {
                        let pv = prev.unwrap();
                        let pat = if [0x1001e194, 0x1001e104, 0x10067c88, 0x10067c58].iter().any(|&s| self.is(&w[pv], s)) { 0x100a31a4 } else { 0x100a31d4 };
                        self.choose_by(&mut w[cu], pat, &mut c);
                    } else if let Some(pat) = prev.and_then(|pv| self.word_pattern(&w[pv])) {
                        self.choose_by(&mut w[cu], pat, &mut c);
                    }
                } else if p == b'8' {
                    let pv = prev.unwrap();
                    if c != b'7' {
                        let o = w[pv].clone();
                        self.agree_with(&mut w[cu], &o, 0x100a2e4c, &mut c);
                    } else if nx_pos != b'7' && nx_pos != b'8' {
                        if let Some(nx) = next {
                            let o = w[pv].clone();
                            self.agree_with(&mut w[nx], &o, 0x100a2e4c, &mut nx_pos);
                        }
                    }
                } else {
                    to_f = true;
                }
                if !to_f {
                    continue;
                }
            }
            // F
            if c == b'^' && nx_pos != b'8' {
                if let Some(nx) = next {
                    self.choose_by(&mut w[nx], 0x100a2f14, &mut nx_pos);
                }
                continue;
            }
            if c == 6 && (nx_pos == b'8' || punct(nx_pos)) {
                if let Some(nx) = next {
                    let o = w[nx].clone();
                    self.agree_with(&mut w[cu], &o, 0x100a2e58, &mut c);
                }
                continue;
            }
            if p == 6 {
                if c != b'7' && c != 7 {
                    if let Some(pv) = prev {
                        let o = w[pv].clone();
                        self.agree_with(&mut w[cu], &o, 0x100a2e58, &mut c);
                    }
                }
            } else if punct(p) && punct(c) {
                self.choose_by(&mut w[cu], 0x100a2f24, &mut c);
            }
        }
    }

    /// 0x10008607: a noun after сам/сама/само/самое/самой/самом/самою/
    /// самого/самому takes the analysis with part of speech 6.
    fn block_c(&self, w: &mut [Word], prev: Option<usize>, c: u8, p: &mut u8) {
        if c != 6 {
            return;
        }
        let Some(pv) = prev else { return };
        const SAM: [u32; 9] = [0x10067ce4, 0x10067cdc, 0x10067cd4, 0x10067ccc, 0x10067cc4, 0x10067cbc, 0x10067cb4, 0x10067cac, 0x10067ca4];
        if SAM.iter().any(|&s| self.is(&w[pv], s)) {
            self.choose_by(&mut w[pv], 0x100a2f2c, p);
        }
    }

    /// 0x10008e10: class (0x10008a20), information (0x10008b60) and
    /// subclass (0x10008ca0) of an unknown-class word from the features of
    /// its analysis; a number word, or a word of feature 6 `/`, becomes
    /// class `y` and turns a `y` before it into `z`.
    fn classes(&self, w: &mut [Word]) {
        for j in 0..w.len() {
            let lex = Lex::of(&w[j]);
            let Some(an) = lex.current().filter(|a| !a.is_empty()) else { continue };
            let (mut t1, mut t3, mut last) = (None, None, None);
            if w[j].f[CLASS] == 0x17 {
                for s in an.iter().rev() {
                    let m = morph(&self.pe, s.morph);
                    last = Some(m.id());
                    match m.flag() {
                        1 if t1.is_none() => t1 = Some(m.id()),
                        3 => {
                            t3 = Some(m.id());
                            break;
                        }
                        _ => {}
                    }
                }
            }
            let to_y = match (t3, t1) {
                (Some(e3), Some(e1)) => {
                    if w[j].f[CLASS] == 0x17 {
                        w[j].f[CLASS] = self.class_of(e3, &w[j]);
                    }
                    if w[j].f[INFO] == 9 {
                        w[j].f[INFO] = self.info_of(e3, e1);
                    }
                    if w[j].f[SUB] == 0xc8 {
                        w[j].f[SUB] = self.sub_of(e3, e1);
                    }
                    last.is_some_and(|id| self.feature(id, 6) == Some(b'/'))
                }
                _ => matches!(w[j].f[SUB], b'y' | b'z' | b'{'),
            };
            if to_y {
                if j > 0 && w[j - 1].f[CLASS] == b'y' {
                    w[j - 1].f[CLASS] = b'z';
                }
                w[j].f[CLASS] = b'y';
            }
        }
    }

    /// 0x10008a20: the class from part of speech (feature 6 of the root).
    fn class_of(&self, e3: i32, w: &Word) -> u8 {
        let Some(v) = self.feature(e3, 6) else { return 0xc8 };
        match v {
            b'$' | b'\'' | b'(' | b')' | b'-' | b'.' | b'/' => 0x15,
            6 => 0x29,
            7 => 0x33,
            0xc => 0x1f,
            b'7' => 0x19,
            b'1' => {
                if !self.is(w, NE) && !self.is(w, NET) {
                    0xc9
                } else {
                    0x2e
                }
            }
            0x1d => 0xca,
            b'8' => 0x20,
            b'^' => 0x2a,
            _ => 0xc8,
        }
    }

    /// 0x10008b60: the information byte (gender and number) from features
    /// 3 and 4 of the ending and 6 of the root.
    fn info_of(&self, e3: i32, e1: i32) -> u8 {
        let f3 = self.feature(e1, 3);
        let f4 = self.feature(e1, 4);
        let f6 = self.feature(e3, 6);
        let (mut found, mut g) = (f3.is_some(), f3.unwrap_or(b'_'));
        if f3.is_none() {
            match f6 {
                Some(b'\'') => (found, g) = (true, 0xf),
                Some(b'(') => (found, g) = (true, 0x22),
                Some(b')') => (found, g) = (true, 0x25),
                _ => {}
            }
        }
        let Some(v4) = f4 else {
            return if f6 == Some(b'.') { 0x35 } else { 9 };
        };
        if !found {
            return 9;
        }
        let sing = matches!(v4, 0x57 | 0x58);
        let plur = matches!(v4, 0x35 | 0x36);
        match g {
            0xf if sing => 0xd,
            0xf if plur => 0xe,
            0x22 | 0x23 | 0x25 | 0x26 if sing => 0xb,
            0x22 | 0x23 | 0x25 | 0x26 if plur => 0xc,
            _ => 9,
        }
    }

    /// 0x10008ca0: the subclass (verb forms from feature 8 of the
    /// ending, cases from feature 1).
    fn sub_of(&self, e3: i32, e1: i32) -> u8 {
        let f6 = self.feature(e3, 6);
        let f8 = self.feature(e1, 8);
        let f1 = self.feature(e1, 1);
        if f6 == Some(b'^') {
            match f8 {
                Some(0x17) => return 0x87,
                Some(0x33) => return 0x88,
                Some(0x16) => return 0x8b,
                Some(0x18) => return 0x8d,
                Some(0x3b) => return 0x8e,
                _ => {}
            }
        }
        match f1 {
            Some(0..=5) => 0xcb,
            Some(0xd | 0xe) => 0xcc,
            Some(0x13 | 0x14) => 0xce,
            Some(0x19..=0x1c) => 0xcd,
            Some(0x1f..=0x21) => 0xcf,
            Some(0x2a | 0x2b) => 0xd0,
            _ => 0xc8,
        }
    }

    // --- LesAccenta -----------------------------------------------------------------

    /// `LesAccenta` (0x10001280): every word record (not punctuation) gets
    /// its stressed spelling and vowel (0x10009520); a spelled letter not
    /// ending a spelled word, or the last of two, is unstressed.
    pub(super) fn accenta(&self, w: &mut [Word]) {
        let n = w.len();
        for k in 0..n {
            let mut lex = Lex::of(&w[k]);
            lex.pos = -1;
            let first = k == 0 || (w[k - 1].f[CLASS] == b'2' && w[k - 1].text.first() != Some(&b'"'));
            let last = k + 1 >= n || (w[k + 1].f[CLASS] == b'2' && w[k + 1].text.first() != Some(&b'"'));
            if w[k].f[CLASS] == b'2' {
                if !w[k].data.is_empty() {
                    lex.store(&mut w[k]);
                }
                continue;
            }
            if w[k].f[SUB] == 0x83 && (k + 1 >= n || w[k + 1].f[SUB] != 0x84 || (k > 0 && w[k - 1].f[SUB] == 0x83)) {
                w[k].f[GRADE] = 3;
            }
            let next = (k + 1 < n).then(|| w[k + 1].text.clone());
            let prev = (k > 0).then(|| w[k - 1].text.clone());
            self.stress(&mut w[k], &mut lex, first, last, next.as_deref(), prev.as_deref());
            lex.store(&mut w[k]);
        }
    }

    /// 0x10009520: the stressed spelling (`lex.form`) and vowel
    /// (`lex.pos`) of one word, and its grade (3 when it has none).
    fn stress(&self, w: &mut Word, lex: &mut Lex, first: bool, last: bool, next: Option<&[u8]>, prev: Option<&[u8]>) {
        let word = w.text.clone();
        let is = |s: &[u8], va: u32| s == self.s(va);
        let byt_forms = [BYT, BYL, BYLO, BYLI];
        let mut k: i32 = 0;
        let mut marked = false;
        let mut form: Option<Vec<u8>> = None;
        // не + быть: the negation takes the stress; быть after не does not;
        // на плацу
        if next.is_some() && is(&word, NE) && byt_forms.iter().any(|&s| is(next.unwrap(), s)) {
            k = 1;
            form = Some(word.clone());
        } else if prev.is_some_and(|p| is(p, NE)) && byt_forms.iter().any(|&s| is(&word, s)) {
            k = 0;
            form = Some(word.clone());
        } else if prev.is_some_and(|p| is(p, NA)) && is(&word, PLATSU) {
            k = 2;
            form = Some(word.clone());
        }
        let form = match form {
            Some(f) => f,
            None => match lex.current().filter(|a| !a.is_empty()) {
                None => {
                    let mut f = match self.brand(w) {
                        Some((r, 0)) => r,
                        _ => transliterate(self, &word, 0x40),
                    };
                    f.truncate(0x3f);
                    marked = has_stress_mark(self, &f);
                    if !marked {
                        k = ending_stress(self, &f);
                    }
                    f
                }
                Some(an) => {
                    let root = morph(&self.pe, an[0].morph);
                    let cls = root.class() as i32;
                    let mut f = self.morph_text(an[0].morph);
                    let mut suffix: i32 = -1;
                    if an.len() > 1 {
                        let m = morph(&self.pe, an[1].morph);
                        suffix = m.class() as i32;
                        f.extend(self.morph_text(an[1].morph));
                    }
                    marked = has_stress_mark(self, &f);
                    if !marked {
                        let e = (ROOT_STRESS as i64 + cls as i64 * 4) as u32;
                        let base = self.pe.u16(e) as i16 as i32;
                        let shift = if (suffix as i16) < 0 { 0 } else { self.movement(self.pe.u16(e + 2) as i16 as i32, suffix as i16) };
                        k = shift + base;
                    }
                    f
                }
            },
        };
        if first && last && !marked && k == 0 {
            k = ending_stress(self, &form);
        }
        if k != 0 {
            lex.pos = vowel_at(self, &form, k);
        }
        lex.form = form;
        w.f[GRADE] = if !marked && k == 0 { 3 } else { 0 };
    }
}

/// `LesGradiAccentuali` (0x10002d60): punctuation (`2`) and class `7`
/// grade 0x0f, a grade not yet set 0; the closing record (boundary 0x17)
/// grade 0x10.
pub(super) fn gradi(w: &mut [Word]) {
    let mut k = 0;
    while k < w.len() && k < 200 && w[k].f[BOUND] != 0x17 {
        if matches!(w[k].f[CLASS], b'2' | b'7') {
            w[k].f[GRADE] = 0x0f;
        } else if w[k].f[GRADE] == 0x0e {
            w[k].f[GRADE] = 0;
        }
        k += 1;
    }
}
