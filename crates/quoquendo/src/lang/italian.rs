//! Italian, ported from the original's language module `LTTS7Italian.dll`
//! (MECHANICS.md §4b). The module's one export, `Italian` (0x1000b640),
//! hands LoqTTS7.dll a table of named functions (0x10031ff8); the ones the
//! FON stage runs are ported here, each under its address. Its tables (the
//! character classes, the ending lists, the function words, the exception
//! dictionary) are read from the module's own bytes at the addresses its
//! code uses, never copied.

use super::common::{among, at, exceptions, field::*, Endings, Pe, Word};

mod numeri;

/// The function table's address and the names it must hold there.
const FUNCTIONS: u32 = 0x10031ff8;

/// Character classes: 8 bytes per character, lower case, upper case, then
/// flags (0x10 vowel, 0x20 upper case, 0x80 accented vowel, 0x100
/// punctuation).
const CHARS: u32 = 0x10030ed8;
const VOWEL: u32 = 0x10;
const UPPER: u32 = 0x20;
const ACCENTED: u32 = 0x80;
const PUNCT: u32 = 0x100;

/// The language module, read from `LTTS7Italian.dll`'s bytes.
pub struct Italian {
    pe: Pe,
    /// (lower, upper, flags) per character, from `CHARS`
    chars: Vec<(u8, u8, u32)>,
    /// The engine's abbreviation table from LesGetAbbreviazioni's pairs.
    abbreviation_table: std::sync::OnceLock<Vec<super::common::Abbreviation>>,
}

/// Stress positions (1 = the last vowel, +30 = a closed e/o), searched by
/// 0x100011c0.
const STRESS: Endings = Endings { at: 0x10028078, count: 0x44a };
/// Word forms (verb endings, adverbs…), searched by 0x1000c4d0.
const FORMS: Endings = Endings { at: 0x10066668, count: 0x675 };

impl Italian {
    /// The module's tables, when `dll` is `LTTS7Italian.dll` as the pack
    /// ships it (7.13.0, the function table at 0x10031ff8).
    pub fn from_dll(dll: Vec<u8>) -> Option<Italian> {
        let pe = Pe::parse(dll)?;
        let name = |i: u32| pe.cstr(pe.u32(FUNCTIONS + 8 * i)).to_vec();
        if name(0) != b"GetVersion" || name(5) != b"FonTrasParola" || name(9) != b"LesGradiAccentuali" {
            return None;
        }
        let chars = (0..256u32)
            .map(|c| (pe.u8(CHARS + 8 * c), pe.u8(CHARS + 8 * c + 1), pe.u32(CHARS + 8 * c + 4)))
            .collect();
        Some(Italian { pe, chars, abbreviation_table: Default::default() })
    }

    fn flags(&self, c: u8) -> u32 {
        self.chars[c as usize].2
    }

    fn is_vowel(&self, c: u8) -> bool {
        self.flags(c) & VOWEL != 0
    }

    /// The vowel without its accent, case kept (0x1000da10).
    fn plain(&self, c: u8) -> u8 {
        if c == 0 {
            return 0;
        }
        let (mut a, upper) = if self.flags(c) & UPPER != 0 { (self.chars[c as usize].0, true) } else { (c, false) };
        a = match a {
            0xe0 | 0xe1 => b'a',
            0xe8 | 0xe9 => b'e',
            0xec | 0xed => b'i',
            0xf2 | 0xf3 => b'o',
            0xf9 | 0xfa => b'u',
            0xfd => b'y',
            _ => a,
        };
        if upper {
            self.chars[a as usize].1
        } else {
            a
        }
    }

    /// The string at `va` in the module.
    fn s(&self, va: u32) -> &[u8] {
        self.pe.cstr(va)
    }

    /// An ending table's answer for a reversed word (0x10001210 for
    /// `STRESS`, 0x1000c520 for `FORMS`).
    fn lookup(&self, t: Endings, rev: &[u8]) -> u16 {
        super::common::lookup(&self.pe, t, rev)
    }

    /// The exception list at `va` against a reversed word.
    fn list(&self, va: u32, rev: &[u8], code: u16) -> u16 {
        exceptions(self.s(va), rev, code)
    }
}

fn reversed(s: &[u8]) -> Vec<u8> {
    s.iter().rev().copied().collect()
}

// --- LesAnalizza (0x1000d890): word classes -----------------------------------

/// Function words (0x1006b3e8): 0x17b entries `{ word, class, subclass,
/// information }`, sorted.
const FUNCTION_WORDS: u32 = 0x1006b3e8;
const FUNCTION_WORDS_COUNT: i32 = 0x17b;

impl Italian {
    /// LesAnalizza: classes from the function-word list, the neighbours and
    /// the endings.
    pub fn analizza(&self, w: &mut [Word]) {
        self.function_words(w);
        self.in_context(w);
        let n = w.len();
        for i in 0..n {
            if w[i].f[CLASS] == 0x17 {
                self.by_form(w, n, i);
            }
        }
    }

    /// 0x1000cd90: the entry of a sorted 8-byte table holding `word`.
    fn search(&self, table: u32, count: i32, word: &[u8]) -> Option<u32> {
        let (mut lo, mut hi) = (0i32, count - 1);
        while lo <= hi {
            let mid = (lo + hi) / 2;
            match word.cmp(self.s(self.pe.u32(table + 8 * mid as u32))) {
                std::cmp::Ordering::Less => hi = mid - 1,
                std::cmp::Ordering::Greater => lo = mid + 1,
                std::cmp::Ordering::Equal => return Some(table + 8 * mid as u32),
            }
        }
        None
    }

    /// 0x1000cdf0: words not yet classified (0x17, or 0x15 without a
    /// subclass, or `e`, `un`, `di` of subclass `z`, a number's reading)
    /// take the function word list's class.
    fn function_words(&self, w: &mut [Word]) {
        for r in w.iter_mut() {
            let c = r.f[CLASS];
            let look = c == 0x17
                || (c == 0x15 && r.f[SUB] == 0xc8)
                || (r.f[SUB] == b'z' && [0x100186cc, 0x100173cc, 0x100186e4].iter().any(|&s| r.text == self.s(s)));
            if !look {
                continue;
            }
            if let Some(e) = self.search(FUNCTION_WORDS, FUNCTION_WORDS_COUNT, &r.text) {
                r.f[CLASS] = self.pe.u8(e + 4);
                r.f[SUB] = self.pe.u8(e + 5);
                r.f[INFO] = self.pe.u8(e + 6);
            }
        }
    }

    /// 0x1000c560: one of the short verbs (`di da fa sta va…`, reversed,
    /// at 0x100181d8…0x100347e8) that a clitic can follow.
    fn short_verb(&self, rev: &[u8]) -> bool {
        [
            0x100181d8, 0x10018c30, 0x10018bdc, 0x10018768, 0x10018710, 0x1006bfd0, 0x100181ac, 0x10018c08, 0x1006bfc8, 0x1006bfc0, 0x100347e8,
        ]
        .iter()
        .any(|&s| rev == self.s(s))
    }

    /// 0x1000c7b0: the form of a reversed word without its first `off`
    /// letters (a verb before its clitics): its verb subclass, 0xc8 for
    /// form 0xe, 0xc7 for none.
    fn host_form(&self, rev: &[u8], off: usize) -> u8 {
        let w = rev.get(off..).unwrap_or(&[]);
        if self.short_verb(w) {
            return 0x87;
        }
        match self.lookup(FORMS, w) {
            9 => 0x8c,
            8 => 0x8b,
            0xc => 0x87,
            0xb => 0x8a,
            7 => 0x88,
            0xd => 0x89,
            0xe => 0xc8,
            _ => 0xc7,
        }
    }

    /// 0x1000c820: a verb followed by clitics (`dammelo`, reversed): its
    /// subclass (0xc7 for none) and the clitics' length.
    fn clitics(&self, r: &[u8]) -> (u8, usize) {
        let p = |i: usize| at(r, i);
        let try_ = |off: usize| {
            let c = self.host_form(r, off);
            (c != 0xc7).then_some((c, off))
        };
        if p(1) == b'l' {
            if p(2) == b'l' {
                if let Some(x) = try_(3) {
                    return x;
                }
            }
            if let Some(x) = try_(2) {
                return x;
            }
            if p(2) == b'e' && p(3) == p(4) && matches!(p(3), b'c' | b'm' | b't') {
                if let Some(x) = try_(5) {
                    return x;
                }
            }
            if p(2) == b'e' && matches!(p(3), b'c' | b'm' | b's' | b't' | b'v') {
                if let Some(x) = try_(4) {
                    return x;
                }
            }
            if p(2) == b'e' && p(3) == b'i' && p(4) == b'l' && p(5) == b'g' {
                if let Some(x) = try_(6) {
                    return x;
                }
            }
        }
        if p(1) == b'n' {
            if p(2) == b'n' && p(0) == b'e' {
                if let Some(x) = try_(3) {
                    return x;
                }
            }
            if p(0) == b'e' {
                if let Some(x) = try_(2) {
                    return x;
                }
            }
        }
        if p(0) == b'e' && p(1) == b'n' && p(2) == b'e' {
            if p(3) == p(4) && matches!(p(3), b'c' | b'm' | b't') {
                if let Some(x) = try_(5) {
                    return x;
                }
            }
            if matches!(p(3), b'c' | b'm' | b's' | b't' | b'v') {
                if let Some(x) = try_(4) {
                    return x;
                }
            }
            if p(3) == b'i' && p(4) == b'l' && p(5) == b'g' {
                if let Some(x) = try_(6) {
                    return x;
                }
            }
        }
        if p(0) != b'i' {
            return (0xc7, 2);
        }
        let c1 = p(1);
        if c1 == p(2) && matches!(c1, b'c' | b'm' | b't') {
            if let Some(x) = try_(3) {
                return x;
            }
        }
        if matches!(p(1), b'c' | b'm' | b's' | b't' | b'v') {
            if let Some(x) = try_(2) {
                return x;
            }
        }
        if p(1) == b'c' {
            if p(2) == b'i' && p(3) == p(4) && matches!(p(3), b'm' | b't') {
                if let Some(x) = try_(5) {
                    return x;
                }
            }
            if p(2) == b'i' && matches!(p(3), b'm' | b't' | b'v') {
                if let Some(x) = try_(4) {
                    return x;
                }
            }
        }
        if p(1) == b'l' && p(2) == b'g' {
            if let Some(x) = try_(3) {
                return x;
            }
        }
        if p(1) != b's' {
            return (0xc7, 2);
        }
        if p(2) == b'i' && matches!(p(3), b'c' | b'v') {
            if let Some(x) = try_(4) {
                return x;
            }
        }
        if p(2) == b'e' && p(3) == b'l' {
            if let Some(x) = try_(4) {
                return x;
            }
        }
        if p(2) == b'i' && p(3) == b'l' && p(4) == b'g' {
            if let Some(x) = try_(5) {
                return x;
            }
        }
        (0xc7, 2)
    }

    /// 0x1000cb40: classify word `i` by its ending (`FORMS`).
    fn by_form(&self, w: &mut [Word], n: usize, i: usize) {
        let rev = reversed(&w[i].text);
        let r = &mut w[i].f;
        let (class, sub) = match self.lookup(FORMS, &rev) {
            2 => (0x2a, None),
            3 => (0x2a, Some(0x8d)),
            4 => (0x33, None),
            // alone (the word and the sentence's end) it is a noun
            5 => (if n == 2 { 0x2f } else { 0x2d }, None),
            6 => {
                let (c, _) = if rev.len() > 4 && matches!(rev[0], b'a' | b'e' | b'i' | b'o') { self.clitics(&rev) } else { (0xc7, 0) };
                if c != 0xc7 {
                    (0x36, Some(c))
                } else {
                    (0x17, None)
                }
            }
            7 => (0x2a, Some(0x88)),
            8 => (0x2a, Some(0x8b)),
            9 => (0x2a, Some(0x8c)),
            10 => (0x2a, Some(0x8e)),
            11 => (0x2a, Some(0x8a)),
            12 => (0x2a, Some(0x87)),
            13 => (0x2a, Some(0x89)),
            14 => (0x2d, None),
            _ => (0x2f, None),
        };
        r[CLASS] = class;
        if let Some(s) = sub {
            r[SUB] = s;
        }
    }

    /// 0x1000cf20: the word after an article read as a verb with clitics
    /// (reversed `-lo -le -ne -mi…` after an infinitive-like stem), which
    /// keeps the article a pronoun. Sets at 0x1006bfe8 `cmstv`, 0x1006bfe0
    /// `aeio`, 0x1006bfd8 `aeir`, 0x1006bfd4 `mtv`.
    fn verb_like(&self, word: &[u8]) -> bool {
        let r = reversed(word);
        let p = |i: usize| at(&r, i);
        let (cmstv, aeio, aeir, mtv) = (self.s(0x1006bfe8), self.s(0x1006bfe0), self.s(0x1006bfd8), self.s(0x1006bfd4));
        let (a0, a1, a2, a3) = (p(0), p(1), p(2), p(3));
        let ending = |a0: u8, a1: u8| (among(aeio, a0) && a1 == b'l') || (a0 == b'e' && a1 == b'n');
        'd0a2: {
            if a1 == b'l' || a1 == b'n' {
                if a2 == b'e' {
                    let stem = if among(cmstv, a3) { p(4) == b'r' } else { a3 == b'i' && p(4) == b'l' && p(5) == b'g' && p(6) == b'r' };
                    if stem && ending(a0, a1) {
                        return true;
                    }
                    break 'd0a2;
                }
                if a2 == b'r' {
                    if among(aeio, a0) && a1 == b'l' {
                        return true;
                    }
                    if a0 == b'e' {
                        if a1 == b'n' {
                            return true;
                        }
                        break 'd0a2;
                    }
                }
                if a0 == b'i' && a1 == b'l' {
                    if a2 == b'g' {
                        return a3 == b'r';
                    }
                    return (a2 == b'r' && among(cmstv, a1)) || (a1 == b'c' && a2 == b'i' && a3 != 0 && p(4) == b'r' && among(mtv, a3));
                }
            }
        }
        if a1 == b'r' && a0 == b'e' && among(aeir, a2) {
            return true;
        }
        if a0 != b'i' {
            return a0 == b'r' && matches!(a1, b'a' | b'e' | b'i');
        }
        (a2 == b'r' && among(cmstv, a1)) || (a1 == b'c' && a2 == b'i' && a3 != 0 && p(4) == b'r' && among(mtv, a3))
    }

    /// 0x1000d180: classes from the neighbours (articles, `lo`/`la` as
    /// pronouns, elisions).
    fn in_context(&self, w: &mut [Word]) {
        let n = w.len();
        if n < 2 {
            return;
        }
        let mut i = 0usize;
        while i < n {
            let next_class = if i + 1 < n { w[i + 1].f[CLASS] } else { 0 };
            let prev_class = if i > 0 { w[i - 1].f[CLASS] } else { 0 };
            let prev_info = if i > 0 { w[i - 1].f[INFO] } else { 0 };
            match w[i].f[CLASS] {
                0 => {
                    let verb = i + 1 < n && (matches!(next_class, 0x18 | 0x19 | 0x1e | 0x16) || (i > 0 && prev_class == 0x21));
                    if verb {
                        w[i].f[CLASS] = 0x2a;
                        w[i].f[SUB] = 0x87;
                        w[i].f[INFO] = 0x17;
                    } else {
                        w[i].f[CLASS] = 0x1e;
                    }
                }
                1 => {
                    if i + 1 < n {
                        if next_class == 0x17 {
                            w[i + 1].f[CLASS] = 0x2f;
                        }
                        w[i + 1].f[INFO] = w[i].f[INFO];
                        w[i].f[CLASS] = 0x29;
                    }
                }
                2 => {
                    if i > 0 && (w[i - 1].text == self.s(0x10030d10) || prev_class == 0x1e) {
                        w[i].f[CLASS] = 0x15;
                        w[i].f[SUB] = 0xc8;
                    } else {
                        w[i].f[CLASS] = 0x1e;
                    }
                }
                3 => {
                    if i == 0 || !w[i - 1].text.contains(&b'\'') {
                        w[i].f[CLASS] = 0x33;
                    } else {
                        w[i].f[CLASS] = 0x15;
                        w[i].f[INFO] = 0xd;
                    }
                }
                4 => {
                    // re-examined under its new class (0x1000d506 jumps back
                    // without moving on)
                    let a = (i + 1 < n && matches!(next_class, 0x19 | 0x1e | 0x32)) || (i > 0 && matches!(prev_class, 0x1e | 0x18));
                    w[i].f[CLASS] = if a { 0x15 } else { 0x18 };
                    continue;
                }
                5 => {
                    if i > 0 && prev_info != 0x0e {
                        let after_number = matches!(prev_info, 0x0b | 0x0f | 0x0c)
                            || (i > 1 && prev_class != 0x32 && matches!(w[i - 2].f[INFO], 0x0b | 0x0f | 0x0c));
                        if after_number || (i + 1 < n && w[i + 1].text == b"punto") {
                            w[i].text = self.s(0x1006bff0).to_vec();
                        }
                    }
                    w[i].f[CLASS] = 0x15;
                }
                0x16 => {
                    if i + 1 < n {
                        self.article_or_pronoun(w, i);
                    }
                }
                0x17 => {
                    if w[i].text.iter().any(|c| b"kwyjx".contains(c)) {
                        w[i].f[CLASS] = 0x2f;
                        w[i].f[INFO] = 9;
                    } else if i == 0 {
                        i += 1;
                        continue;
                    } else if prev_class == 0x21 {
                        w[i].f[CLASS] = 0x2a;
                        w[i].f[INFO] = prev_info;
                    }
                }
                0x18 => {
                    if i + 1 < n && (next_class == 0x17 || next_class == b',' || next_class == b'+') && !self.verb_like(&w[i + 1].text) {
                        w[i + 1].f[INFO] = w[i].f[INFO];
                        w[i + 1].f[SUB] = 0xc8;
                        w[i + 1].f[CLASS] = 0x2f;
                    }
                }
                0x19 => {
                    if i + 1 < n && matches!(next_class, 0x17 | b',' | b'+') && !self.verb_like(&w[i + 1].text) {
                        w[i + 1].f[SUB] = 0xc8;
                        w[i + 1].f[CLASS] = 0x2f;
                    }
                }
                0x1e => {
                    if i + 1 < n && (next_class == 0x17 || next_class == b'*' || next_class == b'+') && !self.verb_like(&w[i + 1].text) {
                        w[i + 1].f[INFO] = w[i].f[INFO];
                        w[i + 1].f[SUB] = 0xc8;
                        w[i + 1].f[CLASS] = 0x2f;
                    }
                }
                0x23 => {
                    w[i].f[CLASS] = if i == 0 || prev_class != 0x19 { 0x24 } else { 0x28 };
                }
                _ => {}
            }
            // a pronoun after `me te ve` before `ne la le li lo l'`: the
            // first is a pronoun too
            if i > 0 && w[i].f[CLASS] == 0x21 {
                let p = &w[i - 1].text;
                let t = &w[i].text;
                if [0x10017b40, 0x10017414, 0x1003239c].iter().any(|&s| p == self.s(s))
                    && [0x10017b04, 0x10017b98, 0x10017b80, 0x10017b58, 0x10017b50, 0x100327f4].iter().any(|&s| t == self.s(s))
                {
                    w[i - 1].f[CLASS] = 0x21;
                }
            }
            i += 1;
        }
    }

    /// Case 0x16 of 0x1000d180: `lo`, `la`… as article or pronoun.
    fn article_or_pronoun(&self, w: &mut [Word], i: usize) {
        let n = w.len();
        let next = w[i + 1].text.clone();
        let (c0, c1) = (at(&next, 0), at(&next, 1));
        let this = w[i].text.clone();
        if this == self.s(0x10017b50) && this.len() > 1 {
            // `lo` keeps being an article before z, pn, ps, x, gn, s + consonant, i + vowel
            let article = c0 == b'z'
                || (c1 == b'n' && c0 == b'p')
                || c0 == b'x'
                || (c0 == b'p' && c1 == b's')
                || (c0 == b'g' && c1 == b'n')
                || (c0 == b's' && !self.is_vowel(c1))
                || (c0 == b'i' && self.is_vowel(c1));
            if !article {
                w[i].f[CLASS] = 0x21;
                return;
            }
        }
        if this == self.s(0x10017b98) && this.len() > 1 && self.is_vowel(c0) && !self.is_vowel(c1) {
            w[i].f[CLASS] = 0x21;
            return;
        }
        if i > 0 && matches!(w[i - 1].f[CLASS], 0x21 | 0x2e) {
            w[i].f[CLASS] = 0x21;
            w[i].f[INFO] = w[i - 1].f[INFO];
            return;
        }
        if w[i + 1].f[CLASS] == 0x17 {
            self.by_form(w, n, i + 1);
        }
        let nc = w[i + 1].f[CLASS];
        if nc == 0x2d {
            return;
        }
        if matches!(nc, 0x2c | 0x2b | 0x2a) {
            w[i].f[CLASS] = 0x21;
            return;
        }
        w[i].f[CLASS] = 0x18;
        if nc == 0x17 {
            w[i + 1].f[INFO] = w[i].f[INFO];
            w[i + 1].f[SUB] = 0xc8;
            w[i + 1].f[CLASS] = 0x2f;
        }
    }
}

// --- LesAccenta (0x10002aa0): where the stress falls ---------------------------

/// Homographs whose stress depends on the neighbours (0x1002bb78): 0x3b
/// entries `{ word, rule, stress if the rule holds, otherwise }`.
const HOMOGRAPHS: u32 = 0x1002bb78;
const HOMOGRAPHS_COUNT: i32 = 0x3b;

/// A vowel with the grave accent (0x100021c0): the open/plain stress mark.
fn grave(c: u8) -> u8 {
    match c {
        b'A' => 0xc0,
        b'E' => 0xc8,
        b'I' => 0xcc,
        b'O' => 0xd2,
        b'Y' => 0xdd,
        b'a' => 0xe0,
        b'e' => 0xe8,
        b'i' => 0xec,
        b'o' => 0xf2,
        b'y' => 0xfd,
        0xdc | b'U' => 0xd9,
        0xef => 0xed,
        0xfc | b'u' => 0xf9,
        _ => c,
    }
}

/// A vowel with the acute accent (0x100022e0): the closed stress mark.
fn acute(c: u8) -> u8 {
    match c {
        b'A' => 0xc1,
        b'E' => 0xc9,
        b'I' => 0xcd,
        b'O' => 0xd3,
        b'Y' => 0xdd,
        b'a' => 0xe1,
        b'e' => 0xe9,
        b'o' => 0xf3,
        b'y' => 0xfd,
        0xdc | b'U' => 0xda,
        0xef | b'i' => 0xed,
        0xfc | b'u' => 0xfa,
        _ => c,
    }
}

impl Italian {
    /// LesAccenta: mark the stressed vowel of every word that has no written
    /// accent, grave (open or plain) or acute (closed e/o).
    pub fn accenta(&self, w: &mut [Word]) {
        for i in 0..w.len() {
            let class = w[i].f[CLASS];
            if class != 0x32 && w[i].text.last() != Some(&b'\'') && !self.has_accent(&w[i].text) {
                let mut code = self.homograph(w, i);
                if code == 0 {
                    code = self.stress_of(class, &w[i].text);
                }
                place_accent(self, code as i16 as i32, &mut w[i].text);
            }
            // 0x10002a30: á í ú are written à ì ù
            for c in w[i].text.iter_mut() {
                *c = match *c {
                    0xe1 => 0xe0,
                    0xed => 0xec,
                    0xfa => 0xf9,
                    x => x,
                };
            }
        }
    }

    /// 0x10002a60: the word already has an accented vowel.
    fn has_accent(&self, t: &[u8]) -> bool {
        t.iter().any(|&c| self.flags(c) & ACCENTED != 0)
    }

    /// 0x10002430: a homograph's stress from its neighbours, 0 for other
    /// words.
    fn homograph(&self, w: &[Word], i: usize) -> u16 {
        let Some(e) = self.search(HOMOGRAPHS, HOMOGRAPHS_COUNT, &w[i].text) else { return 0 };
        let (rule, yes, no) = (self.pe.u8(e + 4), self.pe.u8(e + 5) as u16, self.pe.u8(e + 6) as u16);
        let this = &w[i].f;
        let prev = (i > 0).then(|| &w[i - 1]);
        let pc = prev.map(|p| p.f[CLASS]);
        let pi = prev.map(|p| p.f[INFO]);
        let holds = match rule {
            1 => pc == Some(0x21),
            2 => this[CLASS] != 0x15 && pc != Some(0x21),
            3 => !((pi != Some(0x0b) && pi != Some(0x0f)) && this[CLASS] != 0x15 && this[CLASS] != 0x17),
            4 => {
                let mut k = i;
                let mut c = w[k].f[INFO];
                if c == 0x0b {
                    true
                } else {
                    loop {
                        if matches!(c, 0x0f | 0x0e | 0x10) || k == 0 {
                            break c == 0x0b || c == 0x0f;
                        }
                        k -= 1;
                        c = w[k].f[INFO];
                        if c == 0x0b {
                            break true;
                        }
                    }
                }
            }
            5 => !(this[CLASS] != 0x15 && pi != Some(0x0c)),
            6 => this[CLASS] != b'*',
            7 => !(this[CLASS] != b'*' && prev.map_or(true, |p| !ends_che(&p.text) && p.f[CLASS] != 0x21)),
            8 => !(this[CLASS] != b'*' && (pc == Some(b',') || pi == Some(0x0e))),
            9 => !matches!(pc, Some(b'+' | b',')),
            10 => {
                let nc = w.get(i + 1).map_or(0, |n| n.f[CLASS]);
                !(this[CLASS] != b'*' && (pc != Some(b'+') || !matches!(nc, 0x18 | 0x19 | 0x1e)))
            }
            11 => prev.is_some_and(|p| p.text == self.s(0x1002bf60)),
            12 => matches!(pc, Some(0x19 | 0x1e | 0x18)),
            13 => pi == Some(0x0d),
            _ => return 0,
        };
        if holds {
            yes
        } else {
            no
        }
    }

    /// 0x10002790: the stress code of a word of class `class` (0 for a word
    /// without vowels or longer than 63 letters).
    fn stress_of(&self, class: u8, text: &[u8]) -> u16 {
        if text.iter().filter(|&&c| self.is_vowel(c)).count() == 0 || text.len() > 0x3f {
            return 0;
        }
        let mut rev = reversed(text);
        if class == b'6' {
            // a verb with clitics: the verb's own stress, counted past them
            let (_, k) = self.clitics(&rev);
            let extra = match k {
                4 | 5 => 2,
                6 => 3,
                _ => 1,
            };
            if self.short_verb(rev.get(k..).unwrap_or(&[])) {
                return extra + 1;
            }
            return self.stress(&rev[k.min(rev.len())..]) + extra;
        }
        if class == b'-' {
            if let Some(code) = self.participle(&mut rev) {
                return code;
            }
        } else if !matches!(class, b'*' | b'+' | b',') {
            return self.stress(&rev);
        }
        if at(&rev, 1) == b'n' && at(&rev, 0) == b'o' {
            let list = match at(&rev, 2) {
                b'a' => 0x1002bef0,
                b'i' => 0x1002bd50,
                _ => return self.stress(&rev),
            };
            // -ano/-ino verb forms: listed stems, else the stem's own stress
            let r = self.list(list, &rev, 0x3080);
            return if r == 0 { self.stress(&rev[2..]) + 1 } else { r };
        }
        self.stress(&rev)
    }

    /// 0x10001210: the stress table's answer for a reversed word.
    fn stress(&self, rev: &[u8]) -> u16 {
        self.lookup(STRESS, rev)
    }
}

/// 0x10002400: the word ends in `ché`.
fn ends_che(t: &[u8]) -> bool {
    t.len() > 2 && t.ends_with(&[b'c', b'h', 0xe9])
}

/// 0x10002990: put the accent on the vowel `code` counts from the end
/// (30 and up: the acute accent, counting from `code` − 30). A final `j`
/// is not counted as the stressed vowel itself.
fn place_accent(it: &Italian, code: i32, t: &mut [u8]) {
    let closed = code >= 0x1e;
    let mut n = if closed { code - 0x1e } else { code };
    if n <= 0 {
        return;
    }
    let mut idx = t.len() as i32 - 1;
    let mut pos = 0usize;
    while idx >= 0 && n > 0 {
        let c = t[idx as usize];
        if it.is_vowel(c) {
            if n > 1 || c != b'j' {
                n -= 1;
            }
            pos = idx as usize;
        }
        idx -= 1;
    }
    if let Some(c) = t.get_mut(pos) {
        *c = if closed { acute(*c) } else { grave(*c) };
    }
}

// --- FonTrasParola (0x1000f210): letters → phones -------------------------------

/// Vowels (0x1008c0e8) and the vowels that soften c/g (0x1008c26c).
const VOWELS: u32 = 0x1008c0e8;
const FRONT: u32 = 0x1008c26c;
/// Whole-word exceptions (0x1006c138, 0x1ea74 bytes, sorted `word=phones`).
const EXCEPTIONS: (u32, u32) = (0x1006c138, 0x1ea74);
/// The module's phone symbols (0x100318c0: `{ entries, count }`, entries
/// `{ symbol, code }`).
const PHONES: u32 = 0x100318c0;

/// The byte at `i` of a working word, 0 before its start or past its end.
#[inline]
fn g(w: &[u8], i: isize) -> u8 {
    if i < 0 {
        0
    } else {
        at(w, i as usize)
    }
}

fn starts_with(w: &[u8], s: &[u8], n: usize) -> bool {
    super::common::starts(w, s, n)
}

impl Italian {
    /// A word list of the module (0x1000db00): the word without accents and
    /// lower case, searched in the blob at `table` (`size` bytes) with
    /// `ELQSearchCompactString`'s `flags`; the matching entry.
    fn dict(&self, word: &[u8], flags: u32, size: u32, table: u32) -> Option<&[u8]> {
        let key: Vec<u8> = word.iter().take_while(|&&c| c != 0).map(|&c| self.chars[self.plain(c) as usize].0).collect();
        let blob = &self.pe.at(table)[..size as usize];
        let o = super::common::search_compact(&key, blob, flags, 0)?;
        let e = &blob[o..];
        Some(&e[..e.iter().position(|&c| c == 0).unwrap_or(e.len())])
    }

    fn among(&self, set: u32, c: u8) -> bool {
        among(self.s(set), c)
    }

    /// FonTrasParola: a word's phones as the module's feature codes, from
    /// its accented spelling (LesAccenta's) and stress grade (3 or 5:
    /// unstressed).
    pub fn tras_parola(&self, word: &[u8], grade: u8) -> Vec<u32> {
        // 0x1000dc90: the exception list, keyed without accents
        let key: Vec<u8> = word.iter().map(|&c| if self.flags(c) & ACCENTED != 0 { self.plain(c) } else { c }).collect();
        let blob = &self.pe.at(EXCEPTIONS.0)[..EXCEPTIONS.1 as usize];
        if let Some(o) = super::common::search_compact(&key, blob, 1, b'=') {
            let v = &blob[o..];
            return self.parse_phones(&v[..v.iter().position(|&c| c == 0).unwrap_or(v.len())]);
        }
        let mut w = prep(word);
        if grade == 3 || grade == 5 {
            if let Some(c) = w.iter_mut().find(|c| self.flags(**c) & ACCENTED != 0) {
                *c = self.plain(*c);
            }
        }
        let mut out: Vec<u32> = Vec::new();
        let mut i = 0usize;
        while i < w.len() {
            let c = w[i];
            let ii = i as isize;
            let gi = |k: isize| g(&w, ii + k);
            let fl = self.flags(c);
            let mut next = i + 1;
            if fl & PUNCT != 0 {
                match c {
                    b'"' | b'\'' => {}
                    b'#' | b',' => out.push(0x13),
                    _ => out.push(0x17),
                }
            } else if fl & ACCENTED != 0 {
                out.push(match c {
                    0xe8 => 0x106c1,
                    0xe9 => 0x10741,
                    0xf2 => 0x102e1,
                    0xf3 => 0x10361,
                    0xfd => 0x107c1,
                    _ => self.pe.u32(0x1006be7c + 4 * self.plain(c) as u32),
                });
            } else if c.is_ascii_lowercase() && c == gi(1) {
                // a doubled letter
                if fl & VOWEL != 0 {
                    let v = self.pe.u32(0x1006bf4c + 4 * c as u32);
                    if v != 0x10 {
                        out.extend([v, v]);
                        next = i + 2;
                    } else {
                        out.push(if c == b'i' { 0x7c1 } else { 0x3e1 });
                    }
                } else if i == 0 || gi(2) == 0 || (gi(2) == b'h' && gi(3) == 0) {
                    // at the start or the end of the word: the first is dropped
                } else {
                    let v = self.pe.u32(0x1006bee4 + 4 * c as u32);
                    if v != 0x10 {
                        out.push(v);
                        next = i + 2;
                    } else {
                        match c {
                            b'c' | b'g' => {
                                let soft = self.among(FRONT, gi(2));
                                next = i + 2;
                                if gi(2) == b'h' {
                                    next = i + 3;
                                }
                                let last = at(&w, next) == 0;
                                out.push(match (c, soft, last) {
                                    (b'c', true, true) => 0x1712,
                                    (b'c', true, false) => 0x1716,
                                    (b'c', false, true) => 0xff2,
                                    (b'c', false, false) => 0xff6,
                                    (_, true, true) => 0x1702,
                                    (_, true, false) => 0x1706,
                                    (_, false, true) => 0xfe2,
                                    (_, false, false) => 0xfe6,
                                });
                            }
                            b'x' => {
                                out.extend([0xff2, 0x1952]);
                                next = i + 2;
                            }
                            b'z' => {
                                out.push(if self.z_unvoiced(&w, i) { 0x1916 } else { 0x1906 });
                                next = i + 2;
                            }
                            _ => next = i + 2,
                        }
                    }
                }
            } else if c.is_ascii_lowercase() && self.pe.u32(0x1006bf4c + 4 * c as u32) != 0x10 {
                if c != b'h' {
                    out.push(self.pe.u32(0x1006bf4c + 4 * c as u32));
                }
            } else if c != b'h' {
                self.letter(&mut w, i, &mut out, &mut next);
            }
            i = next;
        }
        out
    }

    /// The letters FonTrasParola reads in context (its switch at 0x1000f62d).
    fn letter(&self, w: &mut [u8], i: usize, out: &mut Vec<u32>, next: &mut usize) {
        let c = w[i];
        let ii = i as isize;
        let gi = |w: &[u8], k: isize| g(w, ii + k);
        let vowel = |c: u8| self.among(VOWELS, c);
        let front = |c: u8| self.among(FRONT, c);
        match c {
            b'c' => {
                let b = gi(w, 1);
                if b == b'h' && gi(w, 2) == b'o' && gi(w, 3) == b'c' {
                    out.push(0x1752);
                    *next = i + 2;
                } else if front(b) || self.flags(b) & PUNCT != 0 {
                    out.push(0x1712);
                } else if b == b'q' || b == b'k' {
                    out.push(if gi(w, 2) == 0 { 0xff2 } else { 0xff6 });
                    *next = i + 2;
                } else {
                    out.push(0xff2);
                    if b == b'h' {
                        *next = i + 2;
                    }
                }
            }
            b'd' => {
                if gi(w, 1) != b'z' {
                    out.push(0x1be2);
                } else {
                    out.push(if i == 0 || gi(w, 2) == 0 { 0x1902 } else { 0x1906 });
                    *next = i + 2;
                }
            }
            b'g' => match gi(w, 1) {
                b'h' => {
                    out.push(0xfe2);
                    *next = i + 2;
                }
                b'l' => {
                    let gl = if i >= 1 && vowel(gi(w, -1)) {
                        if gi(w, 2) == b'i' && vowel(gi(w, 3)) {
                            true
                        } else {
                            let a = gi(w, 2);
                            let b = gi(w, 3);
                            matches!(a, b'i' | 0xec | b'\'')
                                && b != b'c'
                                && b != b'g'
                                && !(b == b't' && gi(w, 4) == b't' && !starts_with(w, self.s(0x1008c260), 8))
                                && !(b == b'f' && !starts_with(&w[1..], self.s(0x1008c258), 5))
                        }
                    } else if (i == 0 || gi(w, -1) == b'r')
                        && matches!(gi(w, 2), b'i' | 0xec)
                        && ((gi(w, 3) != 0 && vowel(gi(w, 3)) && gi(w, 4) != 0) || gi(w, 3) == 0)
                    {
                        out.push(0x10a2);
                        *next = if gi(w, 3) != 0 { i + 3 } else { i + 2 };
                        return;
                    } else {
                        false
                    };
                    if gl {
                        out.push(0x10a2);
                    } else {
                        out.extend([0xfe2, 0x18a2]);
                    }
                    *next = i + 2;
                }
                b'n' => {
                    out.push(0x11c2);
                    *next = i + 2;
                }
                b => out.push(if front(b) { 0x1702 } else { 0xfe2 }),
            },
            b'i' => {
                let n1 = gi(w, 1);
                if !vowel(n1) {
                    out.push(0x7c1);
                    return;
                }
                if i > 0 && (self.i_full(w, i) || (n1 == b'u' && vowel(gi(w, -1)) && vowel(gi(w, 2)))) {
                    out.push(0x7c1);
                    return;
                }
                if out.is_empty() {
                    out.push(0x10c2);
                    return;
                }
                match self.i_kind(w, *out.last().unwrap(), i) {
                    1 => out.push(0x10c2),
                    2 => out.push(0x7c1),
                    _ => {}
                }
            }
            b'l' => out.push(0x18a2),
            b'm' => out.push(if matches!(gi(w, 1), b't' | b'd') { 0x19c2 } else { 0x1fc2 }),
            b'n' => match gi(w, 1) {
                b'b' | b'p' => out.push(0x1fc2),
                b'c' | b'g' => out.push(if front(gi(w, 2)) { 0x19c2 } else { 0xfc2 }),
                b'f' | b'v' => out.push(0x1dc2),
                b'k' | b'q' => out.push(0xfc2),
                b's' if gi(w, 2) == b'c' && front(gi(w, 3)) => {
                    out.extend([0x19c2, 0x1752]);
                    *next = i + 3;
                }
                _ => out.push(0x19c2),
            },
            b'p' => {
                if gi(w, 1) == b'h' {
                    out.push(0x1d52);
                    *next = i + 2;
                } else {
                    out.push(0x1ff2);
                }
            }
            b's' => self.letter_s(w, i, out, next),
            b't' => {
                let b = gi(w, 1);
                if b == b'g' && matches!(gi(w, 2), b'i' | 0xec) {
                    out.push(0x1706);
                    *next = i + 2;
                } else if b == b's' || b == b'z' {
                    let between = i > 0 && vowel(gi(w, -1)) && vowel(gi(w, 2));
                    out.push(if between { 0x1916 } else { 0x1912 });
                    *next = i + 2;
                } else {
                    out.push(0x1bf2);
                }
            }
            b'u' => out.push(self.letter_u(w, i)),
            b'x' => out.extend([0xff2, 0x1952]),
            b'z' => out.push(self.letter_z(w, i)),
            0x9a => out.push(0x1752),
            0xdf => out.push(0x1956),
            0xe1..=0xe5 => out.push(0x10441),
            0xe6 => out.push(0x106c1),
            0xe7 => out.push(0x1952),
            0xea | 0xeb => out.push(0x10741),
            0xed | 0xee | 0xef | 0xfd | 0xff => out.push(0x107c1),
            0xf0 => out.push(0x1be2),
            0xf1 => out.push(0x11c2),
            0xf4 | 0xf6 => out.push(0x106e1),
            0xf5 | 0xf8 => out.push(0x10361),
            0xfa | 0xfb => out.push(0x103e1),
            0xfc => out.push(0x3e1),
            // "Skipping phonetic trascription for unknown char"
            _ => {}
        }
    }

    fn letter_s(&self, w: &mut [u8], i: usize, out: &mut Vec<u32>, next: &mut usize) {
        let ii = i as isize;
        let gi = |w: &[u8], k: isize| g(w, ii + k);
        let vowel = |c: u8| self.among(VOWELS, c);
        let voicing = |c: u8| among(b"bdgvmnrl", c);
        let b = gi(w, 1);
        if b == b'\'' && i >= 1 {
            out.push(0x1942);
            *next = i + 2;
        } else if b == b'h' {
            out.push(0x1752);
            *next = i + 2;
            if gi(w, 2) == b'i' {
                out.push(0x7c1);
                *next = i + 3;
            }
        } else if b == b'c' {
            if !self.among(FRONT, gi(w, 2)) {
                out.extend([0x1952, 0xff2]);
                *next = i + 2;
                if gi(w, 2) != 0 && gi(w, 3) == b'h' {
                    *next = i + 3;
                }
            } else {
                out.push(0x1752);
                *next = i + 2;
            }
        } else if i != 0 && !vowel(gi(w, -1)) {
            out.push(if voicing(b) || self.trans_prefix(w, i) { 0x1942 } else { 0x1952 });
        } else if i != 0 && vowel(b) {
            out.push(if self.s_unvoiced(w, i) { 0x1952 } else { 0x1942 });
        } else {
            out.push(if voicing(b) { 0x1942 } else { 0x1952 });
        }
    }

    fn letter_u(&self, w: &[u8], i: usize) -> u32 {
        let ii = i as isize;
        let gi = |k: isize| g(w, ii + k);
        let vowel = |c: u8| self.among(VOWELS, c);
        let front = |c: u8| self.among(FRONT, c);
        const W: u32 = 0x401ec2;
        const U: u32 = 0x3e1;
        let p = gi(1);
        if !vowel(p) {
            return U;
        }
        if i == 0 {
            return W;
        }
        let before_vowel = || if vowel(gi(2)) { W } else { U };
        match gi(-1) {
            b'b' => {
                if !front(p) {
                    W
                } else if i >= 4 && starts_with(&w[i - 4..], b"tri", 3) {
                    before_vowel()
                } else {
                    U
                }
            }
            b'c' => {
                if front(p) {
                    U
                } else {
                    W
                }
            }
            b'd' | b'z' => {
                if i == 1 || (i == 3 && starts_with(w, b"pad", 3)) {
                    U
                } else if gi(-2) != b'r' && gi(-2) != b'n' && !vowel(gi(2)) {
                    W
                } else {
                    U
                }
            }
            b'l' | b'r' | b't' | b'v' => U,
            b'g' => {
                if i < 2 || (gi(-2) != b'r' && gi(-2) != b'n') {
                    if !front(p) || !vowel(gi(2)) {
                        W
                    } else if i < 3 || !starts_with(&w[i - 3..], b"se", 2) {
                        U
                    } else {
                        W
                    }
                } else {
                    U
                }
            }
            b'm' => {
                if self.among(0x1008c244, p) {
                    U
                } else {
                    W
                }
            }
            b'n' => {
                if i >= 2 && gi(-2) == b'g' {
                    W
                } else if front(p) {
                    before_vowel()
                } else {
                    U
                }
            }
            b's' => {
                if matches!(p, b'i' | 0xec) {
                    U
                } else {
                    W
                }
            }
            b'q' => W,
            _ => {
                if !front(p) || !vowel(gi(2)) {
                    W
                } else {
                    U
                }
            }
        }
    }

    /// A single z (0x1000f62d case `z`): voiceless `Ts` or voiced `Dz`.
    fn letter_z(&self, w: &mut [u8], i: usize) -> u32 {
        const TS: u32 = 0x1912;
        const DZ: u32 = 0x1902;
        let ii = i as isize;
        let vowel = |c: u8| self.among(VOWELS, c);
        if i == 0 {
            let zucc = starts_with(w, b"zucc", 4) && at(w, 4) != b'h';
            return if w == self.s(0x1008c234) || zucc { TS } else { DZ };
        }
        let prev = w[i - 1];
        match prev {
            b'l' => {
                if !starts_with(w, b"belzeb", 6) && !starts_with(w, b"elzev", 5) && w != self.s(0x1008c1bc) {
                    TS
                } else {
                    DZ
                }
            }
            b'n' => {
                if starts_with(w, self.s(0x1008c1f8), 5)
                    || starts_with(w, self.s(0x1008c1ec), 8)
                    || (starts_with(w, self.s(0x1008c1e4), 4) && matches!(at(w, 4), b'o' | b'i'))
                    || (w.len() >= 6 && w[..6] == [b'm', b'i', b'n', b'z', 0xf3, b'n'])
                {
                    return DZ;
                }
                // the stem before the n, looked up with the n cut off
                w[i - 1] = 0;
                let stem = &w[..i - 1];
                let r = if self.dict(stem, 1, 0x55, 0x1008abf0).is_some() || self.dict(stem, 8, 0xb, 0x1008ac48).is_some() {
                    DZ
                } else if !(i >= 4 && g(w, ii - 4) == b'b' && g(w, ii - 3) == b'i' && g(w, ii - 2) == b'a' && g(w, ii + 1) != 0xe0) {
                    if !(i >= 3 && g(w, ii - 3) == b'm' && matches!(g(w, ii - 2), b'a' | 0xe0)) {
                        TS
                    } else if (g(w, ii + 1) == b'i' && g(w, ii + 2) != 0)
                        || g(w, ii + 1) == 0xec
                        || starts_with(cut(w), b"rino", 4)
                        || starts_with(cut(w), b"costu", 5)
                    {
                        TS
                    } else {
                        DZ
                    }
                } else {
                    DZ
                };
                w[i - 1] = b'n';
                r
            }
            b'r' => {
                w[i - 1] = 0;
                let r = if self.dict(&w[..i - 1], 1, 0x3f, 0x1008abac).is_some() {
                    DZ
                } else if w[0] == b'm' && i == 3 && matches!(at(w, 1), b'a' | 0xe0) {
                    let (c4, c5) = (at(w, 4), at(w, 5));
                    if (matches!(c4, b'o' | b'a' | 0xf2 | 0xf3) && (c5 == 0 || c5 == b'l')) || c4 == b'i' || (c4 == 0xe0 && c5 == b'n') {
                        TS
                    } else {
                        DZ
                    }
                } else {
                    TS
                };
                w[i - 1] = b'r';
                r
            }
            p if !vowel(p) => {
                if i == 1 {
                    if !starts_with(w, b"tzig", 4) {
                        TS
                    } else {
                        DZ
                    }
                } else if i != 2 || !starts_with(w, b"ecz", 3) {
                    TS
                } else {
                    DZ
                }
            }
            _ => {
                let b = g(w, ii + 1);
                let check = starts_with(w, b"venezu", 6) || b == 0 || self.among(FRONT, b) || !vowel(b);
                if !check || starts_with(w, b"aziend", 6) || starts_with(w, self.s(0x1008c214), 6) {
                    return 0x1906;
                }
                if b == 0 {
                    TS
                } else if g(w, ii + 2) == 0 || vowel(g(w, ii + 2)) {
                    if b == b'h' && g(w, ii + 2) == 0 {
                        TS
                    } else {
                        0x1916
                    }
                } else {
                    // the word cut before the z
                    if &w[..i] == b"trape" {
                        0x1916
                    } else {
                        0x1906
                    }
                }
            }
        }
    }

    /// 0x1000d940: a transcription in the module's symbols → codes.
    fn parse_phones(&self, text: &[u8]) -> Vec<u32> {
        let ent = self.pe.u32(PHONES);
        let n = self.pe.u16(PHONES + 4) as u32;
        let code = |sym: &[u8]| -> u32 {
            if sym.is_empty() {
                return 0x48;
            }
            match (0..n).find(|&k| self.s(self.pe.u32(ent + 8 * k)) == sym) {
                Some(k) => match self.pe.u32(ent + 8 * k + 4) {
                    0 if k > 3 => 0x48,
                    v => v,
                },
                None => 0x48,
            }
        };
        let mut out = Vec::new();
        let mut j = 0usize;
        while j < text.len() {
            let start = j;
            while j < text.len() && text[j] != b' ' && text[j] != b'-' && j - start < 5 {
                j += 1;
            }
            out.push(code(&text[start..j]));
            if j < text.len() {
                j += 1;
            }
        }
        out
    }

    /// 0x1000eba0: `trans` + a listed stem (0x1008bd00): its s is voiced.
    fn trans_prefix(&self, w: &[u8], i: usize) -> bool {
        i > 3 && w[i - 4..i] == *b"tran" && self.dict(&w[i + 1..], 2, 0x22, 0x1008bd00).is_some()
    }
}

/// The working word up to its first NUL.
fn cut(w: &[u8]) -> &[u8] {
    &w[..w.iter().position(|&c| c == 0).unwrap_or(w.len())]
}

/// 0x1000db80: the spelling FonTrasParola reads: `y`/`j` as `i`, and a
/// final `-tion(s)` read the French way (`-scion`, with `à` → `è`).
fn prep(src: &[u8]) -> Vec<u8> {
    let mut dst: Vec<u8> = Vec::with_capacity(src.len() + 4);
    let mut i = 0usize;
    while i < src.len() {
        let s = |k: usize| at(src, k);
        if s(i) == b't'
            && s(i + 1) == b'i'
            && s(i + 2) == b'o'
            && s(i + 3) == b'n'
            && i != 0
            && s(i - 1) != b's'
            && (s(i + 4) == 0 || (s(i + 4) == b's' && s(i + 5) == 0))
        {
            if dst.last() == Some(&0xe0) {
                let n = dst.len();
                if n > 1 && matches!(dst[n - 2], b'c' | b'g') {
                    dst[n - 1] = b'h';
                    dst.push(0xe8);
                } else {
                    dst[n - 1] = 0xe8;
                }
            }
            dst.extend_from_slice(b"sc");
            i += 1;
        }
        let c = src[i];
        dst.push(if c == b'y' || c == b'j' { b'i' } else { c });
        i += 1;
    }
    dst
}

impl Italian {
    /// 0x1000ef50: an `i` before a vowel after the phone `prev`: 1 a glide
    /// `j`, 2 a full `i`, 0 nothing (it only softened c/g, or after `Gl`).
    fn i_kind(&self, w: &[u8], prev: u32, idx: usize) -> u32 {
        if !self.among(VOWELS, at(w, idx + 1)) {
            return 2;
        }
        if prev < 0x1753 {
            if prev == 0x1752 {
                // after $: `presciì-`, `idrosciì-` and `sci` + a listed stem keep it
                if !starts_with(w, self.s(0x1008c160), 7) && !starts_with(w, self.s(0x1008c154), 8) {
                    if idx != 2 {
                        return 0;
                    }
                    let rest = w.get(3..).unwrap_or(&[]);
                    if self.dict(rest, 2, 0x3e, 0x1008bf0c).is_none() {
                        return if self.dict(rest, 1, 0x3b, 0x1008bf4c).is_some() { 2 } else { 0 };
                    }
                }
                return 2;
            }
            if prev < 0x1707 {
                if prev == 0x1706 || prev == 0x10a2 {
                    return 0;
                }
                if prev == 0x1702 {
                    return if starts_with(w, self.s(0x1008c174), 9) { 2 } else { 0 };
                }
            } else {
                if prev == 0x1712 {
                    if starts_with(w, self.s(0x1008c16c), 7) {
                        return 2;
                    }
                    if idx == 3 && starts_with(w, b"arc", 3) {
                        let c4 = at(w, 4);
                        if !matches!(c4, b'e' | 0xe8 | 0xe9) || at(w, 5) != b'r' {
                            if !matches!(c4, b'o' | 0xf2 | 0xf3) || at(w, 5) != b'n' {
                                return 2;
                            }
                        }
                    }
                    return 0;
                }
                if prev == 0x1716 {
                    return 0;
                }
            }
            return 1;
        }
        if matches!(prev, 0x19a2 | 0x18a2 | 0x18a6 | 0x19a6) {
            2
        } else {
            1
        }
    }

    /// 0x1000ebf0: an `i` before a vowel kept a full vowel because of the
    /// word's start (a listed stem ending there, `bio-`, `uni-`, `semi-`…).
    fn i_full(&self, word: &[u8], idx: usize) -> bool {
        let mut b: Vec<u8> = word.iter().map(|&c| self.plain(c)).collect();
        if at(&b, idx + 2) == 0 {
            return false;
        }
        let c = b[idx];
        b[idx] = 0;
        if self.dict(&b[..idx], 1, 0x47, 0x1008bd28).is_some() {
            return true;
        }
        b[idx] = c;
        let rest = |k: usize| b.get(k..).unwrap_or(&[]);
        match idx {
            1 => {
                if at(&b, 0) != b'b' || at(&b, 2) != b'o' {
                    return false;
                }
                let r = rest(3);
                !(starts_with(r, b"nd", 2) || r == self.s(0x1008c14c) || r == self.s(0x100186e4) || starts_with(r, b"lc", 2) || starts_with(r, b"occo", 3))
            }
            2 => {
                let mut r = false;
                if starts_with(&b, b"un", 2) {
                    if starts_with(rest(3), b"on", 2) {
                        return false;
                    }
                    r = true;
                }
                if !starts_with(&b, b"sp", 2) {
                    return r;
                }
                if self.dict(rest(3), 1, 0x199, 0x1008bd70).is_none() {
                    return r;
                }
                true
            }
            3 => {
                let (s, n): (&[u8], usize) = if starts_with(&b, b"min", 3) {
                    if starts_with(rest(4), b"atur", 4) {
                        return false;
                    }
                    (b"er", 2)
                } else if starts_with(&b, b"per", 3) {
                    (b"od", 2)
                } else if starts_with(&b, b"amb", 3) {
                    (b"ent", 3)
                } else if starts_with(&b, b"sem", 3) {
                    if starts_with(rest(4), b"ol", 2) {
                        return false;
                    }
                    return rest(4) != self.s(0x100189ac);
                } else {
                    return false;
                };
                !starts_with(rest(4), s, n)
            }
            5 => starts_with(&b, b"frang", 5) && starts_with(rest(6), b"ond", 3),
            _ => false,
        }
    }

    /// 0x1000dd10: a `zz` (at `idx`) said voiceless `Ts:` rather than `Dz:`:
    /// rules on the letters around it and lists of exceptions.
    fn z_unvoiced(&self, word: &[u8], idx: usize) -> bool {
        let b: Vec<u8> = word.iter().map(|&c| self.plain(c)).collect();
        let p = b.get(idx + 2..).unwrap_or(&[]);
        let c1 = at(&b, idx + 2);
        if idx == 0 || c1 == 0 {
            return true;
        }
        let any = |list: &[&[u8]]| list.iter().any(|s| b.as_slice() == *s);
        let s = |va: u32| self.s(va);
        match b[idx - 1] {
            b'a' => {
                if idx == 3 {
                    if b[0] == b'a' && b[1] == b'm' {
                        return false;
                    }
                } else if idx == 1 {
                    if c1 == b'e' && at(&b, 4) == b'r' {
                        return false;
                    }
                } else if idx == 2 && among(b"bgln", b[0]) && matches!(c1, b'a' | b'e' | b'o') && b.as_slice() != b"lazzate" {
                    return false;
                }
                let prefix = [&b"ard"[..], b"urr", b"ann", b"ecc", b"itt", b"opp", b"uff"].iter().any(|x| starts_with(p, x, 3))
                    || starts_with(p, b"im", 2)
                    || starts_with(p, b"orr", 3)
                    || (idx >= 4 && starts_with(&b[idx - 4..], b"mag", 3) && starts_with(p, b"in", 2))
                    || (idx >= 2 && b[idx - 2] == b'b' && starts_with(p, b"ot", 2));
                if prefix {
                    return false;
                }
                !any(&[b"casalazzara", b"castellazzara", b"sannazzaro", s(0x1008c02c), s(0x1008c024), b"aviorazzo", s(0x1008c010), b"miazzina"])
            }
            b'e' => {
                if idx > 1 && b[idx - 2] == b'm' && !starts_with(&b, b"fermezz", 7) {
                    let listed = any(&[b"premezzo", b"vermezzo"]) || starts_with(&b, b"comezz", 6);
                    if !listed {
                        return false;
                    }
                }
                if idx >= 5 && starts_with(&b[idx - 5..], b"batt", 4) {
                    return false;
                }
                if idx == 3 {
                    if starts_with(&b, b"gr", 2) {
                        return false;
                    }
                } else if idx == 1 && at(&b, 4) == b'e' && at(&b, 5) == b'l' {
                    return false;
                }
                if starts_with(&b, b"bezzecc", 7) || (starts_with(&b, b"mu", 2) && starts_with(p, b"in", 2)) {
                    return false;
                }
                !any(&[b"cadrezzate", b"rezzato", s(0x1008bfb0), s(0x1008bfa8)])
            }
            b'i' => {
                if idx == 2 && at(&b, 5) == 0 {
                    return true;
                }
                if !matches!(c1, b'a' | b'e' | b'i' | b'o') {
                    return true;
                }
                // a listed prefix, then a listed stem ending before the i
                if let Some(m) = self.dict(&b, 2, 0x2f, 0x1008ac54) {
                    let n = m.len();
                    if n != 0 && idx.wrapping_sub(2) >= n && self.dict(&b[n..(idx - 1).max(n)], 1, 0xaaf, 0x1008ac88).is_some() {
                        return false;
                    }
                }
                self.dict(&b[..idx - 1], 1, 0xaaf, 0x1008ac88).is_none()
            }
            b'o' => {
                if idx == 2 {
                    if b[0] == b'r' {
                        if b.as_slice() != s(0x1008bfa0) {
                            return false;
                        }
                    } else if b[0] == b'g' || (b[0] == b'd' && c1 == b'i') {
                        return false;
                    }
                }
                if starts_with(&b, b"diroz", 5) || starts_with(&b, b"chioz", 5) {
                    return false;
                }
                b.as_slice() != s(0x1008bf88)
            }
            _ => {
                if idx != 2 {
                    return true;
                }
                if b[0] == b's' || b[0] == b'b' {
                    return false;
                }
                b[0] != b'z'
            }
        }
    }

    /// 0x1000e5c0: an s between vowels (at `idx`) said voiceless: listed
    /// words and prefixes (`ri-`, `pre-`, `tra-`… before a listed stem).
    fn s_unvoiced(&self, w: &[u8], idx: usize) -> bool {
        let s = |va: u32| self.s(va);
        let vowel = |c: u8| self.among(VOWELS, c);
        if self.dict(w, 2, 0xd6, 0x1008b738).is_some() {
            return true;
        }
        if idx > 2 && w[idx - 3..] == *s(0x1008c120) {
            return true;
        }
        let p = &w[idx..];
        if self.dict(p, 2, 0x22, 0x1008b864).is_some() {
            return self.dict(&w[..idx], 8, 0x25, 0x1008b888).is_some();
        }
        if w.len() < idx + 3 {
            return false;
        }
        if w[idx - 1] == b'o' && (starts_with(&p[1..], s(0x1008c11c), 3) || starts_with(&p[1..], b"aur", 3)) {
            return true;
        }
        if (starts_with(p, b"sant", 4) || starts_with(p, s(0x1008c108), 4)) && vowel(at(p, 4)) {
            return self.dict(w, 2, 0x52, 0x1008b810).is_none();
        }
        let c = at(p, 1);
        if matches!(c, b'e' | 0xe8 | 0xe9) && at(p, 2) == b'c' && at(p, 3) == b'c' && idx > 2 {
            let c4 = at(p, 4);
            if c4 != b'a' && c4 != b'o' && (c4 != b'h' || at(p, 5) != b'i' || vowel(at(p, 6))) {
                return false;
            }
            return true;
        }
        if matches!(c, b'o' | 0xf2 | 0xf3) && at(p, 2) == b'm' && at(p, 3) == b'm' && vowel(at(p, 4)) {
            return true;
        }
        if let Some(m) = self.dict(w, 2, 0x2b3, 0x1008b8b0) {
            if idx == m.len() {
                return true;
            }
        }
        let from = |k: usize| w.get(k..).unwrap_or(&[]);
        if idx == 1 && w[0] == b'a' {
            return self.dict(from(2), 2, 0x2a, 0x1008bb64).is_some();
        }
        let (w0, w1, w2) = (at(w, 0), at(w, 1), at(w, 2));
        match idx {
            2 => {
                if w0 == b'c' && w1 == b'o' {
                    return self.dict(from(3), 2, 0x15, 0x1008bb90).is_some();
                }
                if w0 == b'd' && w1 == b'i' {
                    let r = from(3);
                    return starts_with(r, s(0x1008c0e0), 5) || starts_with(r, s(0x1008c0d8), 5);
                }
                match (w0, w1) {
                    (b'b', b'i') => self.dict(from(3), 2, 0x37, 0x1008bba8).is_none(),
                    (b'd', b'e') => self.dict(from(3), 2, 0x35, 0x1008bbe0).is_none(),
                    (b'r', b'e') => self.dict(from(3), 2, 0x18, 0x1008bc18).is_none(),
                    (b'r', b'i') => self.dict(from(3), 2, 0x2f, 0x1008bc30).is_none(),
                    _ => false,
                }
            }
            3 => {
                if w0 == b'p' && w1 == b'r' {
                    if w2 == b'e' {
                        return self.dict(from(4), 2, 0x36, 0x1008bc60).is_none();
                    }
                    if matches!(w2, b'o' | 0xf2 | 0xf3) {
                        return self.dict(from(4), 2, 0x20, 0x1008bc98).is_none();
                    }
                }
                if w0 != b't' || w1 != b'r' {
                    return false;
                }
                if w2 == b'e' {
                    return self.dict(from(4), 2, 9, 0x1008bcb8).is_none();
                }
                if w2 != b'a' && w2 != b'i' {
                    return false;
                }
                if w.len() < 7 {
                    return true;
                }
                self.dict(from(4), 2, 0x12, 0x1008bcc4).is_none()
            }
            4 => {
                if starts_with(w, b"bona", 4) {
                    return at(w, 5) == 0xe9 && at(w, 6) == b'r' && at(w, 7) == b'a';
                }
                if starts_with(w, b"tele", 4) {
                    return !starts_with(from(5), s(0x1008c0c4), 3);
                }
                if starts_with(w, b"pari", 4) {
                    return !starts_with(from(5), s(0x1008c0b8), 3);
                }
                if starts_with(w, b"vero", 4) {
                    return starts_with(from(5), s(0x1008c0a8), 4);
                }
                if starts_with(w, b"ante", 4) {
                    let r = from(5);
                    return starts_with(r, s(0x1008c098), 5) || starts_with(r, b"ignan", 5);
                }
                if starts_with(w, b"irr", 3) {
                    return self.dict(from(3), 2, 0x12, 0x1008bcd8).is_none();
                }
                false
            }
            5 => {
                if starts_with(w, b"valle", 5) {
                    return self.dict(from(6), 2, 0x13, 0x1008bcec).is_some();
                }
                starts_with(w, s(0x1008c080), 7)
            }
            _ => false,
        }
    }
}

/// 0x10001290's exception lists (searched by `exceptions`, reversed words).
mod part {
    pub const EM_RAN: u32 = 0x1002b988;
    pub const ICIM_V: u32 = 0x1002b508;
    pub const ICIM_R: u32 = 0x1002b4b0;
    pub const ICIT_R: u32 = 0x1002b5d4;
    pub const ICIT_V: u32 = 0x1002b5e8;
    pub const ICITET: u32 = 0x1002ba7c;
    pub const ICIV_V: u32 = 0x1002b6b8;
    pub const ICIV_R: u32 = 0x1002b6b0;
    pub const IMOT: u32 = 0x1002bb6c;
    pub const IMET: u32 = 0x1002b4f4;
    pub const IMRE: u32 = 0x1002b760;
    pub const ENEC: u32 = 0x1002b43c;
    pub const ENEM: u32 = 0x1002b3f0;
    pub const ENES: u32 = 0x1002b478;
    pub const ENET: u32 = 0x1002b484;
    pub const ENEV: u32 = 0x1002b414;
    pub const ENOT: u32 = 0x1002baa8;
    pub const ENODN: u32 = 0x1002b8d0;
    pub const ENRE: u32 = 0x1002b6dc;
    pub const ISICR: u32 = 0x1002ba54;
    pub const ISICV: u32 = 0x1002ba60;
    pub const ISOTA: u32 = 0x1002b994;
    pub const ISOTI: u32 = 0x1002b9d0;
    pub const ISATA: u32 = 0x1002ba2c;
    pub const ISETA: u32 = 0x1002ba34;
    pub const ISRE: u32 = 0x1002b818;
    pub const ITRE: u32 = 0x1002b7b0;
    pub const IVET: u32 = 0x1002b894;
    pub const IVRE: u32 = 0x1002b880;
    pub const LECV: u32 = 0x1002b8bc;
    pub const LEMR: u32 = 0x1002b6cc;
    pub const LESV: u32 = 0x1002b964;
    pub const LETV: u32 = 0x1002b920;
    pub const LRE: u32 = 0x1002b71c;
    pub const ICRE: u32 = 0x1002b73c;
}

impl Italian {
    /// 0x10001290: the stress code of a participle (class `-`) from its
    /// reversed spelling, `None` when the function returns 0. `rev` is
    /// rewritten as the module does; 0x10001250 strips its first letters
    /// before the stress table when no exception answered.
    fn participle(&self, rev: &mut Vec<u8>) -> Option<u16> {
        use part::*;
        let r = |rev: &Vec<u8>, i: usize| at(rev, i);
        let v = |c: u8| self.flags(c) & VOWEL != 0;
        let set = |rev: &mut Vec<u8>, i: usize, c: u8| {
            if i < rev.len() {
                rev[i] = c;
            }
        };
        // 0x10001250
        let strip = |rev: &Vec<u8>, res: u16, off: usize, add: u16| -> u16 {
            if res == 0 {
                self.stress(rev.get(off..).unwrap_or(&[])) + add
            } else {
                res
            }
        };
        let x = |rev: &Vec<u8>, va: u32, code: u16| self.list(va, rev, code);
        // LAB_10001309 / LAB_10001682
        let tail1 = |rev: &mut Vec<u8>, va: u32| {
            let res = x(rev, va, 0x4080);
            set(rev, 1, b'e');
            Some(strip(rev, res, 1, 0))
        };
        // rev[3] = 'e', then strip 3 (+1)
        let tail3 = |rev: &mut Vec<u8>, res: u16| {
            set(rev, 3, b'e');
            Some(strip(rev, res, 3, 1))
        };
        let len = rev.len();
        let b1 = r(rev, 0);
        if !matches!(b1, b'a' | b'e' | b'i' | b'o') {
            return None;
        }
        match r(rev, 1) {
            b'c' => match r(rev, 2) {
                b'e' => (b1 == b'i' && r(rev, 3) == b't').then(|| x(rev, EM_RAN, 0x4083)),
                b'i' => {
                    if b1 != b'i' {
                        return None;
                    }
                    let (code, list) = match r(rev, 3) {
                        b'm' => {
                            if r(rev, 4) != b'r' {
                                if !v(r(rev, 4)) {
                                    return None;
                                }
                                let res = x(rev, ICIM_V, 0x40c0);
                                return Some(strip(rev, res, 4, 2));
                            }
                            (0x50c0, ICIM_R)
                        }
                        b't' => {
                            if r(rev, 4) == b'r' {
                                let res = x(rev, ICIT_R, 0x50c0);
                                if res != 2 {
                                    return tail3(rev, res);
                                }
                            }
                            let b = r(rev, 4);
                            if matches!(b, b'a' | b'i' | b'o') {
                                let res = x(rev, ICIT_V, 0x40c0);
                                return Some(strip(rev, res, 4, 2));
                            }
                            if b != b'e' || r(rev, 5) != b't' || !v(r(rev, 6)) {
                                return None;
                            }
                            return Some(x(rev, ICITET, 0x60c4));
                        }
                        b'v' => {
                            if r(rev, 4) != b'r' {
                                if !v(r(rev, 4)) {
                                    return None;
                                }
                                let res = x(rev, ICIV_V, 0x40c0);
                                return Some(strip(rev, res, 4, 2));
                            }
                            (0x5080, ICIV_R)
                        }
                        _ => return None,
                    };
                    let res = x(rev, list, code);
                    tail3(rev, res)
                }
                b'o' => {
                    if b1 != b'i' {
                        return None;
                    }
                    if r(rev, 3) == b'm' && r(rev, 4) == b'a' {
                        return Some(3);
                    }
                    (r(rev, 3) == b'd' && r(rev, 4) == b'n').then_some(3)
                }
                b'r' => {
                    if b1 != b'i' {
                        return None;
                    }
                    match r(rev, 3) {
                        b'a' | b'i' => Some(2),
                        b'e' => tail1(rev, ICRE),
                        _ => None,
                    }
                }
                _ => None,
            },
            b'l' => match r(rev, 2) {
                b'a' => (r(rev, 3) == b'l' && r(rev, 4) == b'o').then_some(4),
                b'e' => match r(rev, 3) {
                    b'c' if len > 6 => {
                        if r(rev, 4) == b'r' {
                            return tail3(rev, 0);
                        }
                        let res = x(rev, LECV, 0x4080);
                        Some(strip(rev, res, 4, 2))
                    }
                    b'i' if len > 6 && r(rev, 4) == b'l' && r(rev, 5) == b'g' => {
                        if v(r(rev, 6)) {
                            return Some(strip(rev, 0, 6, 3));
                        }
                        if r(rev, 6) == b'r' {
                            set(rev, 5, b'e');
                            return Some(strip(rev, 0, 5, 2));
                        }
                        None
                    }
                    b'm' if len > 6 => {
                        if r(rev, 4) == b'r' {
                            let res = x(rev, LEMR, 0x5080);
                            return tail3(rev, res);
                        }
                        Some(strip(rev, 0, 4, 2))
                    }
                    b's' => {
                        if v(r(rev, 4)) {
                            let res = x(rev, LESV, 0x4080);
                            return Some(strip(rev, res, 4, 2));
                        }
                        if len > 6 && r(rev, 4) == b'r' {
                            return tail3(rev, 0);
                        }
                        None
                    }
                    b't' => {
                        if v(r(rev, 4)) {
                            return Some(x(rev, LETV, 0x4083));
                        }
                        if len > 6 && r(rev, 4) == b'r' {
                            return tail3(rev, 0);
                        }
                        None
                    }
                    b'v' if len > 6 => {
                        if r(rev, 4) == b'r' {
                            return tail3(rev, 0);
                        }
                        Some(strip(rev, 0, 4, 2))
                    }
                    _ => None,
                },
                b'g' => {
                    if b1 != b'i' {
                        return None;
                    }
                    let b = r(rev, 3);
                    if b == b'o' {
                        if r(rev, 4) == b'd' && r(rev, 5) == b'n' {
                            return Some(3);
                        }
                        if r(rev, 4) == b'm' && r(rev, 5) == b'a' && (r(rev, 6) != b'c' || r(rev, 7) != 0) {
                            return Some(3);
                        }
                        if r(rev, 4) == b't' && v(r(rev, 5)) {
                            return Some(3);
                        }
                    }
                    if b == b'e' && r(rev, 4) == b't' {
                        return Some(3);
                    }
                    if b == b'r' && matches!(r(rev, 4), b'a' | b'e' | b'i') {
                        set(rev, 2, b'e');
                        return Some(strip(rev, 0, 2, 0));
                    }
                    None
                }
                b'r' => match r(rev, 3) {
                    b'a' | b'i' => Some(2),
                    b'e' => tail1(rev, LRE),
                    _ => None,
                },
                _ => None,
            },
            b'm' => {
                if b1 != b'i' {
                    return None;
                }
                let b = r(rev, 2);
                if b == b'o' {
                    if r(rev, 3) == b'd' && r(rev, 4) == b'n' {
                        return Some(3);
                    }
                    if r(rev, 3) == b't' && matches!(r(rev, 4), b'a' | b'i' | b'u') {
                        return Some(x(rev, IMOT, 0x4083));
                    }
                }
                if b == b'e' && r(rev, 3) == b't' && v(r(rev, 4)) {
                    return Some(x(rev, IMET, 0x4083));
                }
                if b != b'r' {
                    return None;
                }
                match r(rev, 3) {
                    b'a' | b'i' => Some(2),
                    b'e' => {
                        let res = x(rev, IMRE, 0x4080);
                        if res == 2 {
                            return Some(0x20);
                        }
                        set(rev, 1, b'e');
                        Some(strip(rev, res, 1, 0))
                    }
                    _ => None,
                }
            }
            b'n' => {
                let b2 = r(rev, 2);
                if b2 == b'e' {
                    let pre = r(rev, 3);
                    if !matches!(pre, b'c' | b'i' | b'm' | b's' | b't' | b'v') || b1 != b'e' {
                        return None;
                    }
                    let b4 = r(rev, 4);
                    return match pre {
                        b'i' => {
                            if b4 != b'l' || r(rev, 5) != b'g' {
                                return None;
                            }
                            if !v(r(rev, 6)) {
                                if r(rev, 6) != b'r' {
                                    return None;
                                }
                                set(rev, 5, b'e');
                                return Some(strip(rev, 0, 5, 2));
                            }
                            Some(strip(rev, 0, 6, 3))
                        }
                        b'm' => {
                            if len < 7 {
                                return None;
                            }
                            if b4 == b'r' {
                                return tail3(rev, 0);
                            }
                            let res = x(rev, ENEM, 0x40c0);
                            Some(strip(rev, res, 4, 2))
                        }
                        _ if b4 != b'r' => {
                            if !v(b4) {
                                return None;
                            }
                            Some(match pre {
                                b'c' => strip(rev, x(rev, ENEC, 0x4080), 4, 2),
                                b's' => strip(rev, x(rev, ENES, 0x4080), 4, 2),
                                b't' => x(rev, ENET, 0x4083),
                                _ => strip(rev, x(rev, ENEV, 0x4080), 4, 2),
                            })
                        }
                        _ => tail3(rev, 0),
                    };
                }
                if b2 == b'o' {
                    if b1 != b'e' {
                        return None;
                    }
                    let b = r(rev, 3);
                    if b == b'd' && r(rev, 4) == b'n' && matches!(r(rev, 5), b'e' | b'a') {
                        return Some(x(rev, ENODN, 0x5083));
                    }
                    if b == b'm' && r(rev, 4) == b'a' && r(rev, 5) == b'i' {
                        return Some(3);
                    }
                    if b != b't' || !matches!(r(rev, 4), b'a' | b'i' | b'u') {
                        return None;
                    }
                    let res = x(rev, ENOT, 0x4083);
                    return Some(if res == 2 { 0x20 } else { res });
                }
                if b2 != b'r' || b1 != b'e' {
                    return None;
                }
                match r(rev, 3) {
                    b'a' | b'i' => Some(2),
                    b'e' => tail1(rev, ENRE),
                    _ => None,
                }
            }
            b's' => {
                if b1 != b'i' {
                    return None;
                }
                let b = r(rev, 2);
                let (b3, b4) = (r(rev, 3), r(rev, 4));
                if b == b'o' && b3 == b'd' && b4 == b'n' && matches!(r(rev, 5), b'a' | b'e') {
                    return Some(3);
                }
                if b == b'r' {
                    return match b3 {
                        b'a' | b'i' => Some(2),
                        b'e' => tail1(rev, ISRE),
                        _ => None,
                    };
                }
                // LAB_10001e63
                if b == b'i' && b3 == b'c' {
                    if b4 == b'r' {
                        let res = x(rev, ISICR, 0x5080);
                        return tail3(rev, res);
                    }
                    if v(b4) {
                        return Some(x(rev, ISICV, 0x4084));
                    }
                }
                if b == b'o' && b3 == b't' {
                    match b4 {
                        b'a' => return Some(x(rev, ISOTA, 0x5083)),
                        b'u' => return Some(3),
                        b'i' => {
                            let res = x(rev, ISOTI, 0x5083);
                            return Some(if res == 2 { 0x20 } else { res });
                        }
                        _ => {}
                    }
                }
                if b == b'a' && b3 == b't' {
                    match b4 {
                        b'a' => return Some(x(rev, ISATA, 0x5083)),
                        b'u' | b'i' => return Some(3),
                        _ => {}
                    }
                }
                if b == b'e' && b3 == b't' {
                    match b4 {
                        b'a' => {
                            let res = x(rev, ISETA, 0x5083);
                            return Some(if res == 2 { 0x20 } else { res });
                        }
                        b'u' | b'i' => return Some(3),
                        _ => {}
                    }
                }
                if b == b'i' && b3 == b't' && matches!(b4, b'a' | b'u' | b'i') {
                    return Some(3);
                }
                if b != b'i' {
                    return None;
                }
                if b3 == b'v' && b4 == b'o' && ((r(rev, 5) == b'd' && r(rev, 6) == b'n') || r(rev, 5) == b't') {
                    return Some(4);
                }
                (b3 == b'l' && b4 == b'g' && r(rev, 5) == b'o').then_some(4)
            }
            b't' => {
                if b1 != b'i' {
                    return None;
                }
                if r(rev, 2) == b'o' && r(rev, 3) == b'd' && r(rev, 4) == b'n' {
                    return Some(3);
                }
                if r(rev, 2) != b'r' {
                    return None;
                }
                match r(rev, 3) {
                    b'a' | b'i' => Some(2),
                    b'e' => tail1(rev, ITRE),
                    _ => None,
                }
            }
            b'v' => {
                if b1 != b'i' {
                    return None;
                }
                let b = r(rev, 2);
                if b == b'o' && r(rev, 3) == b'd' && r(rev, 4) == b'n' {
                    return Some(3);
                }
                if b == b'e' && r(rev, 3) == b't' && v(r(rev, 4)) {
                    return Some(x(rev, IVET, 0x4083));
                }
                if b != b'r' {
                    return None;
                }
                match r(rev, 3) {
                    b'a' | b'i' => Some(2),
                    b'e' => tail1(rev, IVRE),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

// --- LesConfiniProsodici (0x1000c080), LesGradiAccentuali (0x1000b6b0) ---------

/// A word's class is one of `set`.
fn is(c: u8, set: &[u8]) -> bool {
    set.contains(&c)
}

/// Verb subclasses that end a verb group (participles, gerunds…).
const VERB_END: &[u8] = &[0x88, 0x8e, 0x89];

impl Italian {
    /// 0x1000baa0: a kind per word from the classes around it (verb groups,
    /// article + noun groups, prepositions…), which LesConfiniProsodici
    /// turns into boundaries: 6 inside a group, 0/1/2 at its end, 7 for
    /// punctuation.
    fn kinds(&self, w: &[Word]) -> Vec<u32> {
        let n = w.len();
        let mut out = vec![0u32; n + 1];
        let (mut state, mut start) = (0u32, 0usize);
        let class = |k: usize| w.get(k).map_or(0, |r| r.f[CLASS]);
        let sub = |k: usize| w.get(k).map_or(0, |r| r.f[SUB]);
        // 0x1000b950: the group so far is one unit, its last word ends it
        let close = |out: &mut [u32], start: usize, i: usize, kind: u32| {
            for o in out.iter_mut().take(i - 1).skip(start) {
                *o = 6;
            }
            out[i - 1] = kind;
        };
        let verbal = |c: u8, s: u8| is(c, b"*6+,") && !VERB_END.contains(&s);
        let same_neighbours = |i: usize| i > 0 && class(i - 1) == class(i + 1) && sub(i - 1) == sub(i + 1);
        for i in 0..n {
            out[i] = 4;
            let (c, s) = (class(i), sub(i));
            let mut restart = false;
            match state {
                1 => {
                    if is(c, b"7 \"$(!.\x1f3") {
                        state = 1;
                    } else if verbal(c, s) {
                        state = 2;
                    } else {
                        restart = true;
                    }
                }
                2 => {
                    if !is(c, b"7*6+,3") && (s != 0x82 || !same_neighbours(i)) {
                        let t = &w[i].text;
                        let prep = [0x1002f988, 0x10018c8c, 0x10032354, 0x10018c30, 0x1002f97c, 0x100186e4].iter().any(|&a| t == self.s(a));
                        if prep && is(class(i + 1), b"*6+,") {
                            state = 2;
                        } else {
                            close(&mut out, start, i, 1);
                            restart = true;
                        }
                    } else {
                        state = 2;
                    }
                }
                3 => {
                    if is(c, b"7\x18\x16\x1f") {
                        state = 3;
                    } else if is(c, b"/\x15)") || [0x88, 0x8e, 0x89, 0x8d, 0x8c].contains(&s) {
                        state = 4;
                    } else {
                        restart = true;
                    }
                }
                4 => {
                    if !is(c, b"7/\x15)") && ![0x88, 0x8e, 0x89, 0x8d, 0x8c].contains(&s) && (s != 0x82 || !same_neighbours(i)) {
                        close(&mut out, start, i, 0);
                        restart = true;
                    } else {
                        state = 4;
                    }
                }
                5 => {
                    if is(c, b" \"$(!.\x1f3") {
                        state = 1;
                    } else if verbal(c, s) && !(i > 1 && VERB_END.contains(&sub(i - 2))) {
                        state = 2;
                    } else if is(c, b"\x18\x16") {
                        state = 6;
                    } else if is(c, b"7/\x15)") || VERB_END.contains(&s) {
                        state = 7;
                    } else {
                        restart = true;
                    }
                }
                6 => {
                    if is(c, b"\x18\x16\x1f") {
                        state = 6;
                    } else if is(c, b"7/\x15)") || VERB_END.contains(&s) {
                        state = 7;
                    } else {
                        restart = true;
                    }
                }
                7 => {
                    if !is(c, b"/\x15)") && !VERB_END.contains(&s) && c != b'7' {
                        close(&mut out, start, i, 2);
                        restart = true;
                    } else {
                        state = 7;
                    }
                }
                8 => {
                    out[i - 1] = 5;
                    restart = true;
                }
                _ => restart = true,
            }
            if !restart {
                continue;
            }
            // 0x1000bf1c: a group starts here
            state = 0;
            start = i;
            if c == 0x1f {
                state = 8;
            } else if is(c, b" \"$(!.3") {
                state = 1;
            } else if verbal(c, s) {
                if ![0x8d, 0x8c, 0x8b].contains(&s) && i > 0 && class(i - 1) == b'-' {
                    out[i - 1] = 0;
                }
                state = 2;
            } else if is(c, b"\x18\x16") {
                state = 3;
            } else if is(c, b"7/\x15)") || VERB_END.contains(&s) {
                state = 4;
            } else if is(c, b"\x19\x1e") {
                state = 5;
            } else if c == b'2' {
                out[i] = 7;
            }
        }
        out[n] = 9;
        out
    }

    /// 0x1000b970: whether a pause after word `arg` would leave either side
    /// short (true): fewer than 2 words or 15 letters back to the boundary
    /// at `idx`, fewer than 2 words or 20 letters after it up to the
    /// punctuation.
    fn short_sides(&self, w: &[Word], idx: usize, arg: usize) -> bool {
        let (mut before_letters, mut before_words) = (w[idx].text.len(), 1usize);
        for k in (0..idx).rev() {
            let b = w[k].f[BOUND];
            if b != 0x16 {
                if b == 0x0a || b == 0x0b {
                    break;
                }
                before_letters += w[k].text.len();
                before_words += 1;
            }
        }
        let (mut after_letters, mut after_words) = (0usize, 0usize);
        let mut k = arg + 1;
        while k < w.len() && w[k].f[CLASS] != b'2' {
            if w[k].f[CLASS] != b'7' {
                after_words += 1;
                after_letters += w[k].text.len();
            }
            k += 1;
        }
        for r in w.iter().take(arg + 1).skip(idx + 1) {
            if r.f[CLASS] != b'7' {
                after_letters += r.text.len();
                after_words += 1;
            }
        }
        !(before_words >= 2 && before_letters >= 0xf && after_words >= 2 && after_letters >= 0x14)
    }

    /// LesConfiniProsodici: the boundary after every word (`BOUND`): 3/5
    /// inside a phrase, 0x0a an added pause, 0x0b a comma, 0x0f… the
    /// sentence's end by its type `mode` (0 statement, 2, 3 question, 5,
    /// 6 exclamation), 0x16 none, 0 joined (elision).
    pub fn confini(&self, w: &mut [Word], mode: i32) {
        let kinds = self.kinds(w);
        let mut last = 200usize;
        let bound_of = |w: &[Word], k: usize| w.get(k).map_or(0, |r| r.f[BOUND]);
        let mut i = 0usize;
        while kinds[i] != 9 && i < 200 {
            let apos = w[i].text.last() == Some(&b'\'');
            let class = w[i].f[CLASS];
            let mut sub_rule = false;
            if w[i].f[BOUND] == 0x15 {
                let can_pause = |w: &[Word], last: usize| {
                    mode != 3 && mode != 2 && last != 200 && matches!(bound_of(w, last), 3 | 5 | 7) && !self.short_sides(w, last, i)
                };
                match kinds[i] {
                    0 | 2 if class != b'7' => {
                        w[i].f[BOUND] = 5;
                        last = i;
                        sub_rule = true;
                    }
                    0 | 2 | 8 => w[i].f[BOUND] = 0x16,
                    1 => {
                        if class != b'7' {
                            if last != 200 && kinds[last] != 4 && can_pause(w, last) {
                                w[last].f[BOUND] = 0x0a;
                            }
                            w[i].f[BOUND] = 5;
                            last = i;
                            sub_rule = true;
                        } else {
                            w[i].f[BOUND] = 0x16;
                        }
                    }
                    3 => {
                        w[i].f[BOUND] = 9;
                        last = i;
                    }
                    5 => {
                        if can_pause(w, last) {
                            w[last].f[BOUND] = 0x0a;
                        }
                        w[i].f[BOUND] = 1;
                    }
                    6 => {
                        if class == b'7' {
                            w[i].f[BOUND] = 0x16;
                        } else if is(class, b"\x19\x1e\x18 !\x16\"\x1f.") {
                            w[i].f[BOUND] = 1;
                        } else {
                            w[i].f[BOUND] = 3;
                            sub_rule = true;
                        }
                    }
                    7 => {
                        // punctuation: the boundary of the last word before it
                        let mut k = i;
                        if i != 0 {
                            k = i - 1;
                            while k != 0 && w[k].f[BOUND] == 0x16 {
                                k -= 1;
                            }
                        }
                        if w[k].f[CLASS] == b'2' {
                            k = i;
                        }
                        let mark = if w[i].f[SUB] == b'|' {
                            match mode {
                                0 => Some(0x0f),
                                2 => Some(0x11),
                                3 => Some(0x12),
                                5 => Some(0x10),
                                6 => Some(0x13),
                                _ => None,
                            }
                        } else {
                            let t = &w[i].text;
                            let mut j = 0usize;
                            while at(t, j + 1) != 0 && !among(b",)(-.", at(t, j)) {
                                j += 1;
                            }
                            match at(t, j) {
                                b'(' | b')' if i != 0 && w[i - 1].f[CLASS] != b'2' && w.get(i + 1).map_or(0, |r| r.f[CLASS]) != b'2' => Some(0x0b),
                                b',' | b'-' | b'.' => Some(0x0b),
                                _ => None,
                            }
                        };
                        match mark {
                            Some(m) => {
                                w[k].f[BOUND] = m;
                                w[i].f[BOUND] = m;
                                last = i;
                            }
                            None => w[i].f[BOUND] = 0x16,
                        }
                    }
                    _ => {
                        w[i].f[BOUND] = 3;
                        last = i;
                        sub_rule = true;
                    }
                }
            } else if matches!(w[i].f[BOUND], 3 | 5) {
                sub_rule = true;
            }
            if sub_rule {
                // 0x1000c3a8: number words
                match w[i].f[SUB] {
                    b'y' | 0x84 => w[i].f[BOUND] = 7,
                    b'z' | 0x83 => w[i].f[BOUND] = 6,
                    b'{' => w[i].f[BOUND] = 8,
                    _ => {}
                }
            }
            if apos {
                w[i].f[BOUND] = 0;
            }
            i += 1;
        }
    }

    /// LesGradiAccentuali: the stress grade of every word (`GRADE`): 3
    /// unstressed (short function words inside a phrase, elided words), 0
    /// stressed, 0x0f punctuation.
    pub fn gradi(&self, w: &mut [Word]) {
        let all_spelled = w.iter().all(|r| r.f[SUB] == 0x83 || r.f[SUB] == 0x84 || r.f[CLASS] == b'2');
        for r in w.iter_mut().take(200) {
            let c = r.f[CLASS];
            if c == 0x32 || c == 0x37 {
                r.f[GRADE] = 0x0f;
                continue;
            }
            if r.f[GRADE] != 0x0e {
                continue;
            }
            let apos = r.text.len() >= 2 && r.text.last() == Some(&b'\'') && !self.has_accent(&r.text);
            // pronouns, and short articles, prepositions and conjunctions
            // (not `né`), inside a phrase lose their stress, except `però`
            // `ciò` `ossìa`
            let function = c == 0x21 || (is(c, b"\x19\x1e\x18 \x16\"\x1f.") && r.text.len() < 6 && r.text != self.s(0x10030398));
            let kept = [0x100303c8, 0x100303a8, 0x1003234c].iter().any(|&a| r.text == self.s(a));
            let unstressed = (matches!(r.f[BOUND], 2 | 1 | 0 | 6 | 7 | 3 | 0x16) && function && !kept)
                || apos
                || r.f[SUB] == 0x8f
                // 0x1000b8ce: a two-byte first letter name (`pì` `gì`, not `ù`)
                || (!all_spelled && r.f[SUB] == 0x83 && r.text.len() == 2);
            r.f[GRADE] = if unstressed { 3 } else { 0 };
        }
    }

    /// FonTrasConfini (0x1000f0e0): across a word boundary without a pause
    /// (the previous word's boundary 0, 1, 3 or 5), the last phone of the
    /// words so far assimilates to `first`, the next word's first phone:
    /// s → z before a voiced consonant, n → m M N before labials, labiodentals
    /// and velars.
    pub fn tras_confini(&self, prev_bound: u8, phones: &mut [u32], first: u32) {
        if !matches!(prev_bound, 0 | 1 | 3 | 5) {
            return;
        }
        let Some(last) = phones.last_mut() else { return };
        if *last == 0x1952 {
            if first & 3 == 2 && first & 0x1e3 != 0xc2 && first & 0x10 == 0 {
                *last = 0x1942;
            }
        } else if *last == 0x19c2 {
            *last = match first {
                0x1d52 | 0x1d42 => 0x1dc2,
                0x1fe2 | 0x1ff2 => 0x1fc2,
                0x10a2 | 0xfe2 | 0xff2 | 0x11c2 => 0xfc2,
                _ => return,
            };
        }
    }

    /// The module's symbol for a phone code (its table at 0x100318c0).
    pub fn symbol(&self, code: u32) -> Option<&str> {
        let ent = self.pe.u32(PHONES);
        let n = self.pe.u16(PHONES + 4) as u32;
        (0..n)
            .find(|&k| self.pe.u32(ent + 8 * k + 4) == code)
            .and_then(|k| std::str::from_utf8(self.s(self.pe.u32(ent + 8 * k))).ok())
    }
}

// --- LesPronunciabile (0x1000b2e0 → 0x1000b140), LesGetSymbolName ---------------

/// `ELQSearchCompactString` word lists (flags 1, no separator) the spelling
/// test uses: (address, size).
/// 0x10003350: words without a vowel read as words.
const NO_VOWEL_WORDS: (u32, u32) = (0x1002c368, 0xd);
/// 0x10003370: two-letter words.
const WORDS_2: (u32, u32) = (0x1002c378, 0xb2);
/// 0x10003390: three-letter words.
const WORDS_3: (u32, u32) = (0x1002c430, 0x141);
/// 0x100036a0, 0x100036c0, 0x100036e0: spelled words of 2, 2–3, 2–4 letters.
const SPELLED_2: (u32, u32) = (0x1002c644, 0x4);
const SPELLED_3: (u32, u32) = (0x1002c648, 0xbd);
const SPELLED_4: (u32, u32) = (0x1002c708, 0x1f);
/// 0x10009eb0: longer words read whatever their letters.
const WORDS_LONG: (u32, u32) = (0x1002c578, 0xc9);
/// 0x10003800, 0x100037c0, 0x10003770: consonant pairs, triples and
/// quadruples that may stand inside a word (keys of 2, 3, 4 letters).
const INNER_2: (u32, u32) = (0x1002c9b8, 0xb8);
const INNER_3: (u32, u32) = (0x1002c768, 0x249);
const INNER_4: (u32, u32) = (0x1002c728, 0x3d);
/// Literal strings of the consonant-run test 0x100074c0.
const DOUBLED_NOT: u32 = 0x1002fdb0; // "wxqk"
const H_AFTER: u32 = 0x1002fda8; // "gspck"
const END_AFTER_R: u32 = 0x1002fd98;
const END_AFTER_L: u32 = 0x1002fd88;
const END_AFTER_N: u32 = 0x1002fd7c;
const END_AFTER_M: u32 = 0x1002fd78;
const START_AFTER_S: u32 = 0x1002fd6c;
const START_AFTER_C: u32 = 0x1002fd64;
const START_AFTER_P: u32 = 0x1002fd5c;
const START_AFTER_T: u32 = 0x1002fd54;
const START_AFTER_K: u32 = 0x1002fd4c;
const INNER_5: u32 = 0x1002fd44; // "stscr"
/// Literal strings of the vowel-run tests 0x10007ab0, 0x100038b0,
/// 0x100039f0 and the three-letter test 0x10003390.
const VOWELS_6: u32 = 0x1002fdc8; // "cuoiai"
const WHOLE_4A: u32 = 0x1002fdc0; // "gioii"
const WHOLE_4B: u32 = 0x1002fdb8; // "guaii"
const ZOOIAT: u32 = 0x1002fb18;
const AUIA: u32 = 0x1002fb20;
const SAS: u32 = 0x1002fb14;
/// LesGetSymbolName's switch (0x10002dd0): index bytes for `c + 0x80`, then
/// the case addresses; the range check's `ja default` at 0x10002ddc.
const NAMES_INDEX: u32 = 0x10003198;
const NAMES_JUMPS: u32 = 0x10003078;
const NAMES_RANGE_JUMP: u32 = 0x10002ddc;
/// LesGetAbbreviazioni (0x1000b610): pairs `{ abbreviation, expansion }`;
/// their count is LesGetNumeroAbbreviazioni's (0x1000b600, `mov eax, n`).
const ABBREVIATIONS: u32 = 0x100319b8;
const ABBREVIATIONS_COUNT: u32 = 0x1000b600;
/// LesEspandiAbbreviazioni (0x10007430): the tokens it expands (`s.`,
/// `S.`) and 0x100072d0's saints list (`name=letter`, separator `=`).
const SAINT_TOKENS: [u32; 2] = [0x1002fd40, 0x1002fd3c];
const SAINTS: (u32, u32) = (0x1002d668, 0x2217);
const SAINT_INDEX: u32 = 0x10007410;
const SAINT_JUMPS: u32 = 0x100073f0;

/// `strchr(set, c) != NULL`: the terminator is found too.
fn strchr(set: &[u8], c: u8) -> bool {
    c == 0 || set.contains(&c)
}

/// Not a consonant for the spelling tests: outside `a`–`z`, or a vowel
/// (`j` and `y` included).
fn vowelish(c: u8) -> bool {
    !c.is_ascii_lowercase() || b"aeouijy".contains(&c)
}

fn ijy(c: u8) -> bool {
    matches!(c, b'i' | b'j' | b'y')
}

fn aeou(c: u8) -> bool {
    matches!(c, b'a' | b'e' | b'o' | b'u')
}

/// 0x10003700 / 0x10003840: the next run of exactly `n` `kind` marks in
/// `cv` from `p` on, else `cv`'s length.
fn next_run(cv: &[u8], mut p: usize, n: usize, kind: u8) -> usize {
    loop {
        while at(cv, p) != 0 && at(cv, p) != kind {
            p += 1;
        }
        if at(cv, p) == 0 {
            return cv.len();
        }
        let start = p;
        while at(cv, p) == kind {
            p += 1;
        }
        if p - start == n {
            return start;
        }
        if at(cv, p) == 0 {
            return cv.len();
        }
    }
}

/// `strncpy(buf, s + from, n)` read back as a C string.
fn piece(s: &[u8], from: usize, n: usize) -> Vec<u8> {
    s.get(from..).unwrap_or(&[]).iter().take(n).copied().collect()
}

impl Italian {
    /// `ELQSearchCompactString(key, list, 1, size, 0)` found the key.
    fn listed(&self, key: &[u8], (va, size): (u32, u32)) -> bool {
        super::common::search_compact(key, &self.pe.at(va)[..size as usize], 1, 0).is_some()
    }

    /// 0x1000b140: whether a word reads as written (else the engine spells
    /// it); `c` is the engine's char argument.
    pub fn pronunciabile_c(&self, word: &[u8], c: u8) -> bool {
        let mut lw = Vec::new();
        for &b in word.iter().take(0x3f) {
            if b == 0 {
                break;
            }
            if b == b'\'' || self.flags(b) & ACCENTED != 0 {
                return true;
            }
            lw.push(self.chars[b as usize].0);
        }
        let n = lw.len();
        if n == 1 && matches!(lw[0], b'y' | b'j') {
            return false;
        }
        // 0x100032e0: a vowel anywhere
        if !lw.iter().any(|&b| self.flags(b) & VOWEL != 0) {
            return self.listed(&lw, NO_VOWEL_WORDS);
        }
        if (n == 2 && self.listed(&lw, SPELLED_2))
            || ((2..=3).contains(&n) && self.listed(&lw, SPELLED_3))
            || ((2..=4).contains(&n) && self.listed(&lw, SPELLED_4))
        {
            return false;
        }
        if c == 0 {
            let len = word.iter().position(|&b| b == 0).unwrap_or(word.len());
            // 0x10003310: an upper-case letter after the first
            if len >= 4 || !word[..len].iter().skip(1).any(|&b| self.flags(b) & UPPER != 0) {
                return true;
            }
        }
        match n {
            0 => true,
            1 => matches!(lw[0], b'a' | b'e' | b'o' | b'i'),
            2 => self.listed(&lw, WORDS_2),
            3 => self.three_letters(&lw),
            _ => self.syllables(&lw),
        }
    }

    /// 0x10003390: a three-letter word.
    fn three_letters(&self, w: &[u8]) -> bool {
        if self.listed(w, WORDS_3) {
            return true;
        }
        let (b0, b1, b2) = (w[0], w[1], w[2]);
        if b0 == b1 && b1 == b2 {
            return false;
        }
        if vowelish(b0) {
            if vowelish(b1) {
                if vowelish(b2) {
                    return !aeou(b1);
                }
                return !aeou(b0) && !ijy(b1);
            }
            if vowelish(b2) {
                return true;
            }
            if matches!(b0, b'j' | b'y') {
                return false;
            }
            return matches!(b1, b's' | b'l' | b'r');
        }
        if vowelish(b1) {
            if w == self.s(SAS) {
                return false;
            }
            if b1 == b2 && aeou(b1) {
                return false;
            }
            return b0 != b'x';
        }
        let then_vowel = matches!(b2, b'a' | b'e' | b'o' | b'u' | b'i');
        match b0 {
            b'b' | b'c' | b'f' | b'g' | b'k' | b'p' => matches!(b1, b'l' | b'r') && then_vowel,
            b'd' | b't' => b1 == b'r' && then_vowel,
            _ => false,
        }
    }

    /// 0x10009eb0: a longer word, by its runs of consonants and vowels.
    fn syllables(&self, w: &[u8]) -> bool {
        if self.listed(w, WORDS_LONG) {
            return true;
        }
        let len = w.len();
        if (1..len).any(|i| w[i - 1] == w[i] && w[i] == at(w, i + 1)) {
            return false;
        }
        let cv: Vec<u8> = w.iter().map(|&c| if vowelish(c) { b'V' } else { b'C' }).collect();
        let longest = |k: u8| {
            let (mut best, mut run) = (0, 0);
            for &m in &cv {
                run = if m == k { run + 1 } else { 0 };
                best = best.max(run);
            }
            best
        };
        for n in (2..=longest(b'C')).rev() {
            let mut p = next_run(&cv, 0, n, b'C');
            while p < len {
                if !self.consonants(w, p, n) {
                    return false;
                }
                p = next_run(&cv, p + n, n, b'C');
            }
        }
        for n in (2..=longest(b'V')).rev() {
            let mut p = next_run(&cv, 0, n, b'V');
            while p < len {
                if !self.vowels(w, p, n) {
                    return false;
                }
                p = next_run(&cv, p + n, n, b'V');
            }
        }
        true
    }

    /// 0x100074c0: the `n` consonants at `pos` of `w` may stand there.
    fn consonants(&self, w: &[u8], pos: usize, n: usize) -> bool {
        let start = pos == 0;
        let end = pos + n == w.len();
        if start && end {
            return false;
        }
        let (c0, c1) = (at(w, pos), at(w, pos + 1));
        // the pairs read anywhere: s+f/t/l/c/k, tz ts gn ps, gh sh ph ch kh
        let pair = (c0 == b's' && matches!(c1, b'f' | b't' | b'l' | b'c' | b'k'))
            || (c0 == b't' && matches!(c1, b'z' | b's'))
            || (c0 == b'g' && c1 == b'n')
            || (c0 == b'p' && c1 == b's')
            || (c1 == b'h' && strchr(self.s(H_AFTER), c0));
        match n {
            2 if end => {
                if !vowelish(c0) && c0 == c1 && !strchr(self.s(DOUBLED_NOT), c0) {
                    return true;
                }
                if pair {
                    return true;
                }
                match c0 {
                    b'c' => c1 == b'k',
                    b'f' => c1 == b't',
                    b'l' => strchr(self.s(END_AFTER_L), c1),
                    b'm' => strchr(self.s(END_AFTER_M), c1),
                    b'n' => strchr(self.s(END_AFTER_N), c1),
                    b'r' => strchr(self.s(END_AFTER_R), c1),
                    b's' => matches!(c1, b'p' | b't' | b'b' | b'd' | b'k' | b'q'),
                    _ => false,
                }
            }
            2 if start => {
                if pair {
                    return true;
                }
                match c0 {
                    b'b' | b'f' | b'g' | b'v' => matches!(c1, b'l' | b'r'),
                    b'c' => strchr(self.s(START_AFTER_C), c1),
                    b'd' => c1 == b'r',
                    b'k' => strchr(self.s(START_AFTER_K), c1),
                    b'm' => c1 == b'n',
                    b'p' => strchr(self.s(START_AFTER_P), c1),
                    b's' => strchr(self.s(START_AFTER_S), c1),
                    b't' => strchr(self.s(START_AFTER_T), c1),
                    _ => false,
                }
            }
            2 => {
                pair || self.consonants(&piece(w, pos - 1, 3), 1, 2)
                    || self.consonants(&piece(w, pos, 3), 0, 2)
                    || self.listed(&piece(w, pos, 2), INNER_2)
                    || (c0 == b't' && c1 == b'g' && at(w, pos + 2) == b'i')
            }
            3 if end => matches!(c0, b'l' | b'r' | b'n' | b'm') && matches!(c1, b'g' | b'p' | b'c') && at(w, pos + 2) == b'h',
            3 if !start => self.listed(&piece(w, pos, 3), INNER_3),
            3 => {
                let w2 = at(w, 2);
                at(w, 0) == b's'
                    && match at(w, 1) {
                        b'b' | b'd' | b'k' | b'p' | b'q' | b't' => matches!(w2, b'l' | b'r'),
                        b'c' => matches!(w2, b'h' | b'l' | b'r'),
                        b'f' => w2 == b'r',
                        b'g' => matches!(w2, b'n' | b'h' | b'l' | b'r'),
                        _ => false,
                    }
            }
            4 => !start && !end && self.listed(&piece(w, pos, 4), INNER_4),
            5 => !start && !end && super::common::starts(&w[pos..], self.s(INNER_5), 5),
            _ => false,
        }
    }

    /// 0x10007ab0: the `n` vowels at `pos` of `w` may stand there.
    fn vowels(&self, w: &[u8], pos: usize, n: usize) -> bool {
        let start = pos == 0;
        let end = pos + n == w.len();
        let middle = !start && !end;
        let a = |i: usize| at(w, pos + i);
        let before = if pos > 0 { w[pos - 1] } else { 0 };
        match n {
            2 => {
                let (b0, b1) = (a(0), a(1));
                if b0 == b'u' && b1 == b'u' {
                    return false;
                }
                if w.len() != 4 || !end || b0 != b1 {
                    return true;
                }
                !matches!(b0, b'a' | b'o')
            }
            3 => {
                let (b0, b1, b2) = (a(0), a(1), a(2));
                if b1 == b2 && aeou(b1) {
                    if b0 != b'i' {
                        return b0 == b'u' && end && b1 == b'e';
                    }
                    return middle && b1 == b'o' && before == b'd' && a(3) == b'n';
                }
                if ijy(b1) && ijy(b2) {
                    if ijy(b0) {
                        return false;
                    }
                    return !start && (b1 == b'i' || b2 == b'i');
                }
                if b0 == b1 && matches!(b0, b'a' | b'e' | b'o') {
                    return middle && b0 == b'a' && before == b'v' && a(3) == b't' && b2 == b'u';
                }
                match b0 {
                    b'a' if matches!(b1, b'e' | b'o') => !start && (b2 == b'i' || b2 == b'u'),
                    b'a' if b1 != b'u' => true,
                    b'a' => !start && (b2 == b'i' || b2 == b'o'),
                    b'e' => match b1 {
                        b'a' => !start && b2 == b'i',
                        b'o' if b2 != b'a' => !start && (b2 == b'i' || b2 == b'u'),
                        b'o' => middle && before == b'l' && a(3) == b'n',
                        b'u' => !start && b2 == b'o',
                        _ => true,
                    },
                    b'i' => !(b1 == b'u' || ijy(b1)) || !start,
                    b'o' => match b1 {
                        b'a' => !start && (b2 == b'i' || b2 == b'u'),
                        b'e' if b2 == b'u' => middle && before == b'd' && a(3) == b'r',
                        b'e' => b2 == b'i' && !start,
                        b'u' => b2 == b'o' && !start,
                        _ => true,
                    },
                    b'u' => !start,
                    _ if !ijy(b1) => b2 != b'u',
                    _ => !start,
                }
            }
            4 => {
                let (b0, b1, b2, b3) = (a(0), a(1), a(2), a(3));
                if ijy(b0) && aeou(b1) && ijy(b2) && aeou(b3) {
                    return true;
                }
                if start {
                    return aeou(b0) && ijy(at(w, 1)) && at(w, 2) == b'u' && at(w, 3) == b'o';
                }
                if self.inner_four(w, pos) {
                    return true;
                }
                if !end {
                    return self.inner_four_b(w, pos);
                }
                w == self.s(WHOLE_4A) || w == self.s(WHOLE_4B)
            }
            5 => {
                if !middle {
                    if !end || start {
                        return false;
                    }
                    return (a(0) == b'o' && ijy(a(1)) && a(2) == b'a' && ijy(a(3)) && a(4) == b'o')
                        || (a(0) == b'u' && matches!(a(1), b'a' | b'e' | b'o') && a(3) == b'a' && ijy(a(2)) && ijy(a(4)));
                }
                ijy(a(0)) && a(1) == b'a' && ijy(a(2)) && a(3) == b'u' && a(4) == b'o'
            }
            6 => super::common::starts(w, self.s(VOWELS_6), 6) && matches!(at(w, 6), b'a' | b'e' | b'o') && end,
            _ => true,
        }
    }

    /// 0x100038b0: four vowels inside a word read as they are.
    fn inner_four(&self, w: &[u8], pos: usize) -> bool {
        let a = |i: usize| at(w, pos + i);
        match a(0) {
            b'i' | b'j' | b'y' => false,
            b'u' => {
                let b1 = a(1);
                if matches!(b1, b'a' | b'e' | b'o') {
                    ijy(a(2)) && aeou(a(3))
                } else if ijy(b1) {
                    match a(2) {
                        b'a' | b'e' | b'o' => a(3) == b'i',
                        c => ijy(c),
                    }
                } else {
                    false
                }
            }
            _ => {
                if !ijy(a(1)) {
                    return super::common::starts(w, self.s(ZOOIAT), 6);
                }
                match a(2) {
                    b'i' | b'j' | b'y' => true,
                    b'u' => a(3) != b'u',
                    _ => a(3) == b'i',
                }
            }
        }
    }

    /// 0x100039f0: four vowels inside a word, the other shapes.
    fn inner_four_b(&self, w: &[u8], pos: usize) -> bool {
        let s = &w[pos..];
        let a = |i: usize| at(s, i);
        match a(0) {
            b'i' => {
                let b1 = a(1);
                if !matches!(b1, b'a' | b'e' | b'o') {
                    return false;
                }
                let b2 = a(2);
                if matches!(b2, b'a' | b'e' | b'o') && b1 != b2 {
                    return a(3) == b'u';
                }
                b2 == b'u' && a(3) == b'o'
            }
            b'a' | b'e' | b'o' => {
                super::common::starts(s, self.s(AUIA), 4) || (ijy(a(1)) && matches!(a(2), b'a' | b'e' | b'o') && a(3) == b'u')
            }
            _ => false,
        }
    }

    /// LesGetSymbolName (0x100032a0 → 0x10002dd0): a letter's or symbol's
    /// name; `mode` is the engine's flag (punctuation is named only when
    /// set). The compiled switch is read from the module: each case is
    /// `mov eax, imm32; ret`, possibly behind `test al, al` and a jump.
    pub fn symbol_name(&self, c: u8, mode: u8) -> Vec<u8> {
        let k = c as i8 as i32 + 0x80;
        let mut pc = if (0..=0xfe).contains(&k) {
            self.pe.u32(NAMES_JUMPS + 4 * self.pe.u8(NAMES_INDEX + k as u32) as u32)
        } else {
            let b = self.pe.at(NAMES_RANGE_JUMP);
            (NAMES_RANGE_JUMP + 6).wrapping_add(i32::from_le_bytes([b[2], b[3], b[4], b[5]]) as u32)
        };
        let (mut eax, mut zf) = (0u32, false);
        for _ in 0..16 {
            let b = self.pe.at(pc);
            match *b {
                [0x84, 0xc0, ..] => {
                    zf = mode == 0;
                    pc += 2;
                }
                [0x0f, 0x84, r0, r1, r2, r3, ..] => {
                    pc += 6;
                    if zf {
                        pc = pc.wrapping_add(i32::from_le_bytes([r0, r1, r2, r3]) as u32);
                    }
                }
                [op @ (0x74 | 0x75), r, ..] => {
                    pc += 2;
                    if (op == 0x74) == zf {
                        pc = pc.wrapping_add(r as i8 as i32 as u32);
                    }
                }
                [0xb8, a0, a1, a2, a3, ..] => {
                    eax = u32::from_le_bytes([a0, a1, a2, a3]);
                    pc += 5;
                }
                [0xc3, ..] => return self.pe.cstr(eax).to_vec(),
                _ => break,
            }
        }
        Vec::new()
    }

    /// LesNormalizzaAccento (0x10002b50): an apostrophe or backquote (` ' ’
    /// ´) after a vowel becomes its accent (grave for the backquote), one
    /// before a final `s` goes (`moody's`), any other is a plain
    /// apostrophe; then acute á í ú become grave.
    pub fn normalizza_accento(&self, w: &mut Vec<u8>) {
        let mut out: Vec<u8> = Vec::with_capacity(w.len());
        for i in 0..w.len() {
            let c = w[i];
            if !matches!(c, b'`' | b'\'' | 0x92 | 0xb4) {
                out.push(c);
                continue;
            }
            let after_vowel = i > 0 && b"aeiouy".contains(&self.chars[w[i - 1] as usize].0);
            if !after_vowel {
                if self.chars[at(w, i + 1) as usize].0 != b's' || at(w, i + 2) != 0 {
                    out.push(b'\'');
                }
            } else if let Some(last) = out.last_mut() {
                *last = if c == b'`' { grave(w[i - 1]) } else { acute(w[i - 1]) };
            }
        }
        for c in out.iter_mut() {
            *c = match *c {
                0xc1 => 0xc0,
                0xcd => 0xcc,
                0xda => 0xd9,
                0xe1 => 0xe0,
                0xed => 0xec,
                0xfa => 0xf9,
                x => x,
            };
        }
        *w = out;
    }

    /// LesEspandiAbbreviazioni (0x10007430): `s.`/`S.` before a saint's
    /// name (0x100072d0: the name lowered, looked up in `SAINTS`; its
    /// letter picks the title).
    pub fn espandi_abbreviazione(&self, token: &[u8], next: &[u8]) -> Option<Vec<u8>> {
        if !SAINT_TOKENS.iter().any(|&a| token == self.s(a)) {
            return None;
        }
        let key: Vec<u8> = next.iter().take(63).map(|&c| self.chars[c as usize].0).collect();
        let blob = &self.pe.at(SAINTS.0)[..SAINTS.1 as usize];
        let o = super::common::search_compact(&key, blob, 1, b'=')?;
        // the letter's case (index bytes 0x10007410 for `letter - 'A'`,
        // jumps 0x100073f0), each `mov eax, title`
        let k = (blob[o] as i8 as i32).wrapping_sub(0x41);
        if !(0..=0x13).contains(&k) {
            return None;
        }
        let case = self.pe.u32(SAINT_JUMPS + 4 * self.pe.u8(SAINT_INDEX + k as u32) as u32);
        match *self.pe.at(case) {
            [0xb8, a0, a1, a2, a3, ..] => Some(self.s(u32::from_le_bytes([a0, a1, a2, a3])).to_vec()),
            _ => None,
        }
    }

    /// LesGetAbbreviazioni's pairs (0x1000b610), in the module's order.
    fn abbreviation_pairs(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
        let n = self.pe.u32(ABBREVIATIONS_COUNT + 1);
        (0..n)
            .map(|i| (self.s(self.pe.u32(ABBREVIATIONS + 8 * i)).to_vec(), self.s(self.pe.u32(ABBREVIATIONS + 8 * i + 4)).to_vec()))
            .collect()
    }
}

// --- The engine's side (LoqTTS7.dll, `common`) calls these --------------------

impl super::common::Les for Italian {
    fn letter(&self, c: u8) -> (bool, bool, u8) {
        // flags bit 0: a letter (digits have only 0x04)
        let f = self.flags(c);
        (f & 0x1 != 0, f & UPPER != 0, self.chars[c as usize].0)
    }
    fn leggi_numero(&self, toks: &mut [super::common::Tok], i: usize, kind: u32, dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
        self.leggi(toks, i, kind, dmy)
    }
    fn analizza(&self, w: &mut [Word]) {
        Italian::analizza(self, w)
    }
    fn accenta(&self, w: &mut [Word]) {
        Italian::accenta(self, w)
    }
    fn confini(&self, w: &mut [Word], mode: i32) {
        Italian::confini(self, w, mode)
    }
    fn gradi(&self, w: &mut [Word]) {
        Italian::gradi(self, w)
    }
    fn tras_parola(&self, w: &[Word], i: usize) -> Vec<u32> {
        Italian::tras_parola(self, &w[i].text, w[i].f[super::common::field::GRADE])
    }
    fn tras_confini(&self, prev_bound: u8, phones: &mut [u32], next: &mut Vec<u32>) {
        if let Some(&first) = next.first() {
            Italian::tras_confini(self, prev_bound, phones, first)
        }
    }
    fn symbol(&self, code: u32) -> Option<&str> {
        Italian::symbol(self, code)
    }
    fn normalizza_accento(&self, w: &mut Vec<u8>) {
        Italian::normalizza_accento(self, w)
    }
    fn pronunciabile(&self, w: &[u8]) -> bool {
        self.pronunciabile_c(w, 0)
    }
    fn pronunciabile_flag(&self, w: &[u8], flag: u8) -> bool {
        self.pronunciabile_c(w, flag)
    }
    fn symbol_name(&self, c: u8, mode: u8) -> Vec<u8> {
        Italian::symbol_name(self, c, mode)
    }
    fn espandi_abbreviazione(&self, token: &[u8], next: &[u8]) -> Option<Vec<u8>> {
        Italian::espandi_abbreviazione(self, token, next)
    }
    fn abbreviazioni(&self) -> &[super::common::Abbreviation] {
        self.abbreviation_table.get_or_init(|| super::common::abbreviation_table(self, &self.abbreviation_pairs()))
    }
}

impl super::Module for Italian {
    fn analyse(&self, text: &str, voice: Option<&crate::voicefon::VoiceFon>) -> Vec<crate::g2p::Token> {
        super::common::analyse(self, text, voice)
    }
}
