//! The LES side of LTTS7Dutch.dll: the word records (LesElaboraParola, with
//! its compound splitter), the readability test, the names of letters and
//! symbols, the function-word table (LesAnalizza) and the stress grades
//! (LesGradiAccentuali).

use super::super::common::{self, record, Tok, Word};
use super::{Dutch, ACCENTED, CONSONANT, UPPER, VOWEL};
use common::field::*;

/// `LesGetSymbolName`'s switch (0x100030b0): the byte index table and the
/// jump table its `jmp` reads, by the character + 0x80.
const NAME_INDEX: u32 = 0x10003450;
const NAME_JUMPS: u32 = 0x10003338;

/// LesAnalizza's table (0x100077d0): 0x11b `{ char *word, u8 class, sub,
/// info, pad }` sorted by `strcmp`.
const FUNCTION_WORDS: u32 = 0x100991a0;
const FUNCTION_WORDS_N: u32 = 0x11b;

/// The compound splitter's word list (0x10007930: `ELQSearchCompactString`
/// flags 1 over 0x848d2 bytes from `aagje`).
const WORDS: u32 = 0x100148c8;
const WORDS_LEN: usize = 0x848d2;

/// The two-vowel spellings of one syllable (0x10007090: 8 pointers at
/// 0x100140e8: ee ie oo aa uu eu ui ou).
const DIGRAPHS: u32 = 0x100140e8;

/// `is` (0x10013f94): unstressed before most boundaries (0x100071b0).
const IS: u32 = 0x10013f94;

/// `LesNormalizzaAccento` (0x10004c90): `` ` ``, `'`, 0x92 and 0xb4 become `'`.
pub(super) fn normalizza_accento(w: &mut [u8]) {
    for c in w.iter_mut() {
        if matches!(*c, b'`' | b'\'' | 0x92 | 0xb4) {
            *c = b'\'';
        }
    }
}

impl Dutch {
    /// `LesPronunciabile` (0x10003780 → 0x100035c0): readable when it has an
    /// apostrophe or an accented letter; with `flag` not otherwise; a word
    /// all in capitals of two letters is not, of three only vowel-consonant-
    /// vowel or consonant-vowel-consonant, of four not when three
    /// consonants follow at most one vowel; otherwise readable when it has
    /// a vowel (0x10003590). At most 0x3f characters are looked at.
    pub(super) fn pronunciabile(&self, w: &[u8], flag: u8) -> bool {
        let mut all_upper = true;
        let mut low = Vec::with_capacity(w.len().min(0x3f));
        for &c in w.iter().take(0x3f) {
            if c == b'\'' || self.has(c, ACCENTED) {
                return true;
            }
            if !self.has(c, UPPER) {
                all_upper = false;
            }
            low.push(self.lower(c));
        }
        if flag != 0 {
            return false;
        }
        let v = |k: usize| self.has(low.get(k).copied().unwrap_or(0), VOWEL);
        let cons = |k: usize| self.has(low.get(k).copied().unwrap_or(0), CONSONANT);
        let has_vowel = || low.iter().any(|&c| self.has(c, VOWEL));
        if !all_upper {
            return has_vowel();
        }
        match low.len() {
            2 => false,
            3 => (v(0) && v(2) && !v(1)) || (cons(0) && cons(2) && v(1)),
            4 => {
                let mut u = 0;
                while u < 4 && v(u) {
                    u += 1;
                }
                if u < 2 && cons(u) && cons(u + 1) && cons(u + 2) {
                    false
                } else {
                    has_vowel()
                }
            }
            _ => has_vowel(),
        }
    }

    /// `LesGetSymbolName` (0x10003550 → 0x100030b0, mode in AL): the
    /// switch's target for the character either returns its name or, for
    /// punctuation and most symbols, only when `mode` is not 0 (`test
    /// al,al; je`); the names are read where its `mov eax, imm32` points.
    pub(super) fn symbol_name(&self, c: u8, mode: u8) -> Vec<u8> {
        let k = (c as i8 as i32 + 0x80) as u32;
        if k > 0xfe {
            return Vec::new();
        }
        let target = self.pe.u32(NAME_JUMPS + 4 * self.pe.u8(NAME_INDEX + k) as u32);
        let code = self.pe.at(target);
        let at = if code.starts_with(&[0x84, 0xc0, 0x0f, 0x84]) {
            if mode == 0 {
                return Vec::new();
            }
            8
        } else {
            0
        };
        match code.get(at..at + 5) {
            Some(&[0xb8, a, b, c, d]) => self.s(u32::from_le_bytes([a, b, c, d])).to_vec(),
            _ => Vec::new(),
        }
    }

    /// `LesElaboraParola` (0x100074c0). A readable word (or one the engine
    /// asks to keep, or an expansion's) becomes one record (0x17, 200, 200),
    /// or one per part of a compound the splitter finds (0x10007ce0, at most
    /// 10; every part but the last with boundary 4); otherwise each
    /// character's name (mode 1, through LesNormalizzaAccento) a record
    /// (0x15, sub 0x83, the last 0x84, info 9), unnamed ones skipped.
    pub(super) fn elabora_parola(&self, toks: &mut [Tok], i: usize, _idx: usize, spell: u8, mode: u8) -> Vec<Word> {
        let pron = self.pronunciabile(&toks[i].text, 0);
        let mut out = Vec::new();
        if mode == 0 && (pron || spell != 0 || toks[i].expanded) {
            let t = &mut toks[i].text;
            // longer than 0x40: cut at 0x3f
            if t.len() > 0x40 {
                t.truncate(0x3f);
            }
            let mut cuts = vec![0usize];
            if let Some(split) = self.compound(t) {
                // 0x10007ce0: the word written back without the `+`s, the
                // parts' starts noted
                let mut w = Vec::with_capacity(split.len());
                for &ch in &split {
                    if ch == b'+' {
                        cuts.push(w.len());
                    } else {
                        w.push(ch);
                    }
                }
                *t = w;
            }
            if cuts.len() > 10 {
                cuts.truncate(1);
            }
            if t.len() > 0x40 {
                t.truncate(0x3f);
            }
            let t = &toks[i].text;
            let n = cuts.len();
            cuts.push(t.len());
            for k in 0..n {
                let part = &t[cuts[k].min(t.len())..];
                let len = cuts[k + 1].saturating_sub(cuts[k]).min(part.len());
                let mut r = record(part[..len].to_vec(), 0x17, 200, 200);
                r.text.truncate(0x3f);
                if k + 1 < n {
                    r.f[BOUND] = 4;
                }
                out.push(r);
            }
            return out;
        }
        let t = toks[i].text.clone();
        for (k, &c) in t.iter().enumerate() {
            let mut name = self.symbol_name(c, 1);
            if name.is_empty() {
                continue;
            }
            normalizza_accento(&mut name);
            out.push(record(name, 0x15, if k + 1 == t.len() { 0x84 } else { 0x83 }, 9));
        }
        out
    }

    /// The compound splitter (0x10007930): the word in lower case (at most
    /// 63 characters, the character table's byte 0); none when the word
    /// list has it whole or it is shorter than 6. Otherwise, from the left
    /// part `a` (first the whole word): the first cut leaving at least 4
    /// letters on each side whose right part the list has (or, once a cut
    /// was found, whose right part without `en`/`er`/`s`/`n`, 0x10007870,
    /// does: a buffer only overwritten when an ending comes off) is taken;
    /// when the left part (or it without its ending) is a word the split is
    /// complete, else the search goes on in the left part. `None` when no
    /// split completes or the list has the whole word (the module then
    /// leaves the word as it is).
    pub(super) fn compound(&self, word: &[u8]) -> Option<Vec<u8>> {
        let list = &self.pe.at(WORDS)[..WORDS_LEN.min(self.pe.at(WORDS).len())];
        let has = |k: &[u8]| common::search_compact(k, list, 1, 0).is_some();
        let lower: Vec<u8> = word.iter().take(0x3f).map(|&c| self.lower(c)).collect();
        if has(&lower) {
            return None;
        }
        let mut a = lower.clone();
        let mut out: Vec<u8> = Vec::new();
        let mut found = false;
        // the right part without its ending (local_44): kept across cuts
        let mut right_stem: Vec<u8> = Vec::new();
        loop {
            if a.len() < 6 {
                return None;
            }
            let limit = a.len() as isize - 3;
            let mut n = 4isize;
            if limit <= n {
                return None;
            }
            let (left, right) = loop {
                let (l, r) = (a[..n as usize].to_vec(), a[n as usize..].to_vec());
                if has(&r) {
                    found = true;
                    break (l, r);
                }
                if found {
                    strip(&r, &mut right_stem);
                    if has(&right_stem) {
                        break (l, r);
                    }
                }
                n += 1;
                if n >= limit {
                    return None;
                }
            };
            let mut left_stem = Vec::new();
            strip(&left, &mut left_stem);
            let mut t = vec![b'+'];
            t.extend_from_slice(&right);
            t.extend_from_slice(&out);
            out = t;
            if has(&left) || has(&left_stem) {
                let mut t = left;
                t.extend_from_slice(&out);
                return Some(t);
            }
            a = left;
        }
    }

    /// `LesAnalizza` (0x10007850 → 0x100077d0): a word record (class 0x17,
    /// a proper noun 0x15 with sub 200, or sub `z`) found by `strcmp` in the
    /// function-word table (binary search, 0x10007770) takes its class,
    /// sub and info.
    pub(super) fn analizza(&self, w: &mut [Word]) {
        for r in w.iter_mut() {
            let (c, s) = (r.f[CLASS], r.f[SUB]);
            if !(c == 0x17 || (c == 0x15 && s == 0xc8) || s == b'z') {
                continue;
            }
            let (mut lo, mut hi) = (0i32, FUNCTION_WORDS_N as i32 - 1);
            while lo <= hi {
                let mid = (lo + hi) / 2;
                let e = FUNCTION_WORDS + 8 * mid as u32;
                match common::strcmp(&r.text, self.s(self.pe.u32(e))) {
                    std::cmp::Ordering::Less => hi = mid - 1,
                    std::cmp::Ordering::Greater => lo = mid + 1,
                    std::cmp::Ordering::Equal => {
                        r.f[CLASS] = self.pe.u8(e + 4);
                        r.f[SUB] = self.pe.u8(e + 5);
                        r.f[INFO] = self.pe.u8(e + 6);
                        break;
                    }
                }
            }
        }
    }

    /// 0x10007090: one syllable — one vowel letter, or a `j`/`w` before a
    /// vowel, or two vowels side by side spelling one sound (0x100140e8).
    fn monosyllable(&self, t: &[u8]) -> bool {
        let n = t.iter().filter(|&&c| self.has(c, VOWEL)).count();
        if n > 2 {
            return false;
        }
        if n == 1 {
            return true;
        }
        let (mut first, mut last, mut seen) = (0usize, 0usize, false);
        for (k, &c) in t.iter().enumerate() {
            if self.has(c, VOWEL) {
                if (c == b'j' || c == b'w') && self.has(t.get(k + 1).copied().unwrap_or(0), VOWEL) {
                    return true;
                }
                if seen {
                    last = k;
                } else {
                    seen = true;
                    first = k;
                }
            }
        }
        if last == first + 1 {
            let pair = [t[first], t[last]];
            return (0..8).any(|k| self.s(self.pe.u32(DIGRAPHS + 4 * k)) == pair);
        }
        false
    }

    /// `LesGradiAccentuali` (0x100071b0): punctuation 0x0f; a word whose
    /// grade is not set yet: 3 after `z`/`y` following an 0x82 and a `z`; a
    /// compound's later part (the record before has boundary 4) 2; else 3
    /// for a one-syllable function word (classes 0x19 0x1e 0x18 0x20 0x21
    /// 0x16 0x22 0x1f 0x2e) before boundary 0–3, 6, 7 or 0x16, for sub
    /// 0x83 and for an elided word, and `is` before those boundaries; else 0.
    pub(super) fn gradi(&self, w: &mut [Word]) {
        let weak = |b: u8| matches!(b, 0..=3 | 6 | 7 | 0x16);
        for i in 0..w.len().min(200) {
            let class = w[i].f[CLASS];
            if class == b'2' || class == b'7' {
                w[i].f[GRADE] = 0xf;
                continue;
            }
            if w[i].f[GRADE] != 0xe {
                continue;
            }
            let sub = w[i].f[SUB];
            let sub_at = |k: usize| w[k].f[SUB];
            if (sub == b'y' || sub == b'z') && i > 1 && sub_at(i - 1) == 0x82 && sub_at(i - 2) == b'z' {
                w[i].f[GRADE] = 3;
                continue;
            }
            if i != 0 && w[i - 1].f[BOUND] == 4 {
                w[i].f[GRADE] = 2;
                continue;
            }
            let b = w[i].f[BOUND];
            let t = &w[i].text;
            let elided = t.last() == Some(&b'\'');
            let function = matches!(class, 0x19 | 0x1e | 0x18 | 0x20 | 0x21 | 0x16 | 0x22 | 0x1f | 0x2e);
            let g = if (weak(b) && function && self.monosyllable(t)) || sub == 0x83 || elided { 3 } else { 0 };
            w[i].f[GRADE] = if t.as_slice() == self.s(IS) && weak(b) { 3 } else { g };
        }
    }
}

/// 0x10007870: `en`/`er` off the end, else `s`/`n`; `out` is only written
/// when something comes off.
fn strip(w: &[u8], out: &mut Vec<u8>) {
    let n = w.len();
    if n < 2 {
        return;
    }
    let cut = if w[n - 2] == b'e' && (w[n - 1] == b'n' || w[n - 1] == b'r') {
        2
    } else if w[n - 1] == b's' || w[n - 1] == b'n' {
        1
    } else {
        return;
    };
    *out = w[..n - cut].to_vec();
}
