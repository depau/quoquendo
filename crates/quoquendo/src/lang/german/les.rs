//! The LES functions the engine runs on a sentence's records:
//! LesGestisciMaiuscole (0x10008f40), LesAnalizza (0x1000c9f0), LesAccenta
//! (0x100014d0), LesConfiniProsodici (0x10009fc0), LesGradiAccentuali
//! (0x10008130).
//!
//! The engine's record array goes on past the sentence's records with
//! records whose text is NULL; here a record past the slice reads as such a
//! record (all bytes 0), and writes past it are dropped.

use super::super::common::{among, at, lookup, Endings, Word};
use super::{lexinfo, German, Morpheme};

/// Record field offsets the stages read and write.
const CLASS: usize = 0xe;
const SUB: usize = 0xf;
const INFO: usize = 0x10;
const BOUND: usize = 0x11;
const GRADE: usize = 0x12;

/// Character flag: an accented letter (bit 7 of the flags at 0x100287cc).
const ACCENTED: u32 = 0x80;

/// LesAccenta's ending table (0x100011c0): 0x115 entries of 12 bytes at
/// 0x10025af0 (`{ ending, exceptions, code }`, searched with the forward
/// spelling by the same code as LTTS7Italian.dll's 0x100010c0/0x10001000).
const STRESS_ENDINGS: Endings = Endings { at: 0x10025af0, count: 0x115 };

/// The word lists 0x100085b0 searches (pointers pushed by the code): kein,
/// keine, keinen, nicht.
const NEGATIONS: [u32; 4] = [0x1002a7bc, 0x1002a7b4, 0x1002a7ac, 0x1002a790];
/// The word list 0x10008610 searches (der dessen dem den die deren der das
/// denen welcher…; unsorted, as the code builds it).
const RELATIVES: [u32; 14] = [
    0x1001e5d8, 0x1002a804, 0x1001e5e0, 0x100272e0, 0x1002a800, 0x1002a7f8, 0x1001e5d8, 0x1002a7f4, 0x1002a7ec, 0x1002a7e4, 0x1002a7dc, 0x1002a7d4,
    0x1002a7cc, 0x1002a7c4,
];
/// 0x10008b20's two-word phrases (7) and one-word adverbs (22), pushed by
/// the code.
const PHRASES2: [u32; 7] = [0x1002a8c0, 0x1002a8b4, 0x1002a8a8, 0x1002a0dc, 0x1002a0d0, 0x1002a89c, 0x1002a890];
const WORDS2: [u32; 22] = [
    0x1002a008, 0x10029f94, 0x1002a884, 0x10029f78, 0x10029f54, 0x100283d4, 0x1002a87c, 0x1002a874, 0x1002a864, 0x1002a85c, 0x1002a850, 0x1002a840,
    0x10029f2c, 0x1002a838, 0x1002a82c, 0x10029ef0, 0x1002a824, 0x100267fc, 0x1002a81c, 0x1002a814, 0x1002a80c, 0x10029e1c,
];
/// 0x100088f0's phrase tables (pointer arrays in .data): 4, 3, 2 and 1
/// words, (address, count).
const PHRASES4: [(u32, u32); 4] = [(0x1002a450, 5), (0x1002a468, 0x12), (0x1002a4b0, 0x32), (0x1002a578, 0x39)];

/// Byte `off` of record `i` (0 past the slice).
fn b(w: &[Word], i: usize, off: usize) -> u8 {
    w.get(i).map_or(0, |r| r.f[off])
}

/// Record `i`'s text (None past the slice: a NULL text).
fn text(w: &[Word], i: usize) -> Option<&[u8]> {
    w.get(i).map(|r| r.text.as_slice())
}

fn set(w: &mut [Word], i: usize, off: usize, v: u8) {
    if let Some(r) = w.get_mut(i) {
        r.f[off] = v;
    }
}

/// LTTS7Util `ELQstricmp` (ASCII case folding only) == 0.
fn stricmp_eq(a: &[u8], b: &[u8]) -> bool {
    let f = |c: u8| if c.is_ascii_uppercase() { c + 0x20 } else { c };
    a.len() == b.len() && a.iter().zip(b).all(|(&x, &y)| f(x) == f(y))
}

impl German {
    /// LesGestisciMaiuscole (0x10008f40): every upper-case letter after a
    /// word's first (from the first on for the sentence's first word) goes
    /// to lower case; when they all were upper case (or there are none), the
    /// first letter too.
    pub(crate) fn gestisci_maiuscole(&self, w: &mut [Word]) {
        for (i, r) in w.iter_mut().enumerate() {
            let mut all = true;
            for k in usize::from(i != 0)..r.text.len() {
                let c = r.text[k];
                if self.is_upper(c) {
                    r.text[k] = self.lower(c);
                } else {
                    all = false;
                }
            }
            if all && !r.text.is_empty() {
                r.text[0] = self.lower(r.text[0]);
            }
        }
    }

    /// LesAnalizza (0x1000c9f0): word classes from the selected analyses
    /// (0x1000bd80 picks among a word's analyses, 0x1000c5a0 sets class,
    /// gender/number and verb form, 0x1000c720 fixes number words).
    pub(crate) fn analizza(&self, w: &mut [Word]) {
        let mut sel = vec![0usize; w.len()];
        self.choose(w, &mut sel);
        self.classes(w, &sel);
        self.numbers(w);
    }

    /// LesAccenta (0x100014d0): marks the stressed vowel of each word not
    /// analysed yet (`+0xc` 0xffff) and without accented letters, counted
    /// from the end by the ending table's answer; é and í become è and ì.
    /// Stops at the first empty word.
    pub(crate) fn accenta(&self, w: &mut [Word]) {
        for r in w.iter_mut() {
            if r.text.is_empty() {
                return;
            }
            let mut buf = r.text.clone();
            let cap = self.is_upper(buf[0]);
            if cap {
                buf[0] = self.lower(buf[0]);
            }
            if r.f[0xc] == 0xff && r.f[0xd] == 0xff && !r.text.iter().any(|&c| self.flags(c) & ACCENTED != 0) {
                // 0x10001250: the vowels of the original spelling
                let n = r.text.iter().filter(|&&c| self.is_vowel(c)).count() as u16;
                let mut k = n as i16;
                if k > 1 {
                    if r.text.len() > 0x3f {
                        return;
                    }
                    // 0x10001210: syllable from the end
                    let s = lookup(&self.pe, STRESS_ENDINGS, &buf);
                    let c = n.wrapping_sub(s).wrapping_add(1) as i16;
                    if c > 0 {
                        k = c;
                    }
                }
                self.accent(&mut buf, k as i32);
            }
            // 0x10001470
            for c in buf.iter_mut() {
                match *c {
                    0xe9 => *c = 0xe8,
                    0xed => *c = 0xec,
                    _ => {}
                }
            }
            if cap {
                buf[0] = self.upper(buf[0]);
            }
            r.text = buf;
        }
    }

    /// 0x10001400: accent the `k`-th vowel from the end (`k` past 29 less
    /// 30; the first letter when there are fewer vowels).
    fn accent(&self, buf: &mut [u8], mut k: i32) {
        if k >= 0x1e {
            k -= 0x1e;
        }
        if k <= 0 {
            return;
        }
        let mut at = 0usize;
        let mut i = buf.len() as i32 - 1;
        while i >= 0 && k > 0 {
            if self.is_vowel(buf[i as usize]) {
                k -= 1;
                at = i as usize;
            }
            i -= 1;
        }
        if let Some(c) = buf.get_mut(at) {
            *c = accented(*c);
        }
    }

    /// LTTS7Util `ELQSearchString(key, list, 1, n) != -1`: an exact match,
    /// by binary search when the list has more than 3 entries.
    fn search_exact(&self, key: &[u8], list: impl Fn(usize) -> u32, n: usize) -> bool {
        let s = |i: usize| self.s(list(i));
        if n > 3 {
            let (mut lo, mut hi) = (0i64, n as i64 - 1);
            while lo <= hi {
                let mid = (lo + hi) / 2;
                match key.cmp(s(mid as usize)) {
                    std::cmp::Ordering::Less => hi = mid - 1,
                    std::cmp::Ordering::Greater => lo = mid + 1,
                    std::cmp::Ordering::Equal => return true,
                }
            }
            false
        } else {
            (0..n).any(|i| key == s(i))
        }
    }

    fn in_list(&self, key: &[u8], list: &[u32]) -> bool {
        self.search_exact(key, |i| list[i], list.len())
    }

    /// 0x10008890: a copy with accented letters (flag 0x80) unaccented by
    /// 0x10007f70.
    fn unaccent(&self, s: &[u8]) -> Vec<u8> {
        s.iter().map(|&c| if self.flags(c) & ACCENTED != 0 { self.plain(c) } else { c }).collect()
    }

    /// 0x10007f70: the letter without the module's stress accent (case
    /// kept).
    fn plain(&self, c: u8) -> u8 {
        if c == 0 {
            return 0;
        }
        let up = self.is_upper(c);
        let mut l = if up { self.lower(c) } else { c };
        l = match l {
            0xe0 => 0x61,
            0xe1 => 0xe4,
            0xe8 => 0x65,
            0xe9 => 0xeb,
            0xec => 0x69,
            0xed => 0xef,
            0xf2 => 0x6f,
            0xf3 => 0xf6,
            0xf9 => 0x75,
            0xfa => 0xfc,
            0xfd => 0x79,
            x => x,
        };
        if up {
            self.upper(l)
        } else {
            l
        }
    }

    /// Up to `max` words from `i` joined by spaces (0x10008b20 / 0x100088f0)
    /// and how many.
    fn joined(w: &[Word], i: usize, max: usize) -> (Vec<u8>, usize) {
        let mut s = Vec::new();
        let mut n = 0;
        while n < max {
            let Some(t) = text(w, i + n) else { break };
            if n > 0 {
                s.push(b' ');
            }
            s.extend_from_slice(t);
            n += 1;
        }
        (s, n)
    }

    /// 0x10008b20: words `i`, `i+1` a two-word phrase (2), or word `i` one
    /// of its adverbs (1).
    fn phrase2(&self, w: &[Word], i: usize) -> Option<u16> {
        let (s, n) = Self::joined(w, i, 2);
        if n == 2 && self.in_list(&self.unaccent(&s), &PHRASES2) {
            return Some(2);
        }
        let t = self.unaccent(text(w, i).unwrap_or(b""));
        self.in_list(&t, &WORDS2).then_some(1)
    }

    /// 0x100088f0: the longest phrase of up to 4 words from `i` in its
    /// tables, its word count.
    fn phrase4(&self, w: &[Word], i: usize) -> Option<u16> {
        let (s, n) = Self::joined(w, i, 4);
        let mut s = self.unaccent(&s);
        let table = |k: usize, s: &[u8]| {
            let (at, cnt) = PHRASES4[k];
            self.search_exact(s, |j| self.pe.u32(at + 4 * j as u32), cnt as usize)
        };
        // cut at the last space (the whole string when there is none past
        // the first byte)
        let cut = |s: &mut Vec<u8>| {
            let mut e = s.len().saturating_sub(1);
            while e != 0 && s[e] != b' ' {
                e -= 1;
            }
            s.truncate(e);
        };
        let mut k = match n {
            4 => 0,
            3 => 1,
            2 => 2,
            _ => 3,
        };
        while k < 3 {
            if table(k, &s) {
                return Some(4 - k as u16);
            }
            if k < 2 {
                cut(&mut s);
            }
            k += 1;
        }
        let t = self.unaccent(text(w, i).unwrap_or(b""));
        table(3, &t).then_some(1)
    }

    /// 0x100085b0: word `i` is kein/keine/keinen/nicht.
    fn negation(&self, w: &[Word], i: usize) -> bool {
        text(w, i).is_some_and(|t| self.in_list(t, &NEGATIONS))
    }

    /// 0x10008610: word `i` is a relative pronoun or article.
    fn relative(&self, w: &[Word], i: usize) -> bool {
        text(w, i).is_some_and(|t| self.in_list(t, &RELATIVES))
    }

    /// 0x100086b0: the stretches before `cur` (back from `last`, up to a
    /// 0x0a/0x0b boundary) and after it (to the sentence end) are not both
    /// at least 4 words long.
    fn short(&self, w: &[Word], cur: usize, last: usize) -> bool {
        let mut before = 1u32;
        let mut k = last;
        while k > 0 {
            k -= 1;
            match b(w, k, BOUND) {
                0x16 => {}
                0x0a | 0x0b => break,
                _ => before += 1,
            }
        }
        let mut after = 0u32;
        let mut k = cur + 1;
        while text(w, k).is_some() {
            match b(w, k, CLASS) {
                0x32 => break,
                0x37 => {}
                _ => after += 1,
            }
            k += 1;
        }
        let mut k = cur;
        while k > last {
            if b(w, k, CLASS) != 0x37 {
                after += 1;
            }
            k -= 1;
        }
        !(before >= 4 && after >= 4)
    }

    /// 0x10008760: a boundary may go after word `i` (the classes of `i` and
    /// `i+1` allow one, and neither is a negation, nor `i+1` a relative
    /// after a noun).
    fn breakable(&self, w: &[Word], i: usize) -> bool {
        if text(w, i + 1).is_none() {
            return false;
        }
        let c = b(w, i, CLASS);
        let n = b(w, i + 1, CLASS);
        let nf = b(w, i + 1, SUB);
        let verb = |x: u8| matches!(x, 0x2a..=0x2c);
        if (c == 0x18 && n == 0x15) || (c == 0x29 && n == 0x15) || (c == 0x15 && n == 0x29) {
            return false;
        }
        if verb(c) && (matches!(n, 0x33 | 0x29 | 0x18 | 0x15 | 0x19) || (verb(n) && nf == 0x8d)) {
            return false;
        }
        if c == 0x33 && verb(n) {
            return false;
        }
        if matches!(c, 0x2b | 0x2c) && n == 0x2a {
            return false;
        }
        if matches!(c, 0x19 | 0x1e) && matches!(n, 0x29 | 0x15) {
            return false;
        }
        if c == 0x21 && verb(n) {
            return false;
        }
        if self.negation(w, i) || self.negation(w, i + 1) {
            return false;
        }
        if c == 0x15 && self.relative(w, i + 1) {
            return false;
        }
        true
    }

    /// 0x10009040: the phrase structure of the sentence, (kind, sub) per
    /// word, ended by kind 10.
    fn phrases(&self, w: &[Word]) -> Vec<(u32, u32)> {
        let mut p: Vec<(u32, u32)> = Vec::new();
        let (mut n2, mut c2, mut n4, mut c4) = (0u16, 0u16, 0u16, 0u16);
        let mut start = 0usize;
        let mut end4 = 0usize;
        let mut state = 0u32;
        let mut i = 0usize;
        // 0x10008580
        let fill = |p: &mut Vec<(u32, u32)>, from: usize, to: usize| {
            for k in from..to {
                p[k] = (7, 0);
            }
        };
        let no_head = |e: u8, f: u8| matches!(e, 0x37 | 0x2f | 0x15 | 0x29) || matches!(f, 0x88 | 0x8e | 0x89);
        let verb = |e: u8| matches!(e, 0x2a..=0x2c);
        let lit = |f: u8| matches!(f, 0x88 | 0x8e | 0x89);
        while text(w, i).is_some() {
            p.push((5, 0));
            if n2 == 0 {
                if let Some(n) = self.phrase2(w, i) {
                    n2 = n;
                    c2 = 1;
                }
            } else if c2 < n2 {
                c2 += 1;
            } else if c2 == n2 {
                n2 = 0;
            }
            if n4 == 0 {
                if let Some(n) = self.phrase4(w, i) {
                    n4 = n;
                    c4 = 1;
                }
            } else if c4 < n4 {
                c4 += 1;
            } else if c4 == n4 {
                n4 = 0;
                end4 = i.wrapping_sub(1);
            }
            let e = b(w, i, CLASS);
            let f = b(w, i, SUB);
            let pm = i.wrapping_sub(1);
            let same_around = |w: &[Word]| f == 0x82 && b(w, pm, CLASS) == b(w, i + 1, CLASS) && b(w, pm, SUB) == b(w, i + 1, SUB);
            let mut classify = false;
            match state {
                1 => {
                    if matches!(e, 0x37 | 0x20 | 0x22 | 0x24 | 0x28 | 0x21 | 0x2e | 0x1f | 0x33) {
                        state = 1;
                    } else if verb(e) && !lit(f) {
                        state = 2;
                    } else {
                        state = 0;
                        classify = true;
                    }
                }
                2 => {
                    if matches!(e, 0x37 | 0x2a | 0x2b | 0x2c | 0x33) || (i > 0 && same_around(w)) {
                        state = 2;
                    } else if n2 == 0 && n4 == 0 {
                        fill(&mut p, start, i - 1);
                        p[i - 1].0 = 1;
                        if end4 == i - 1 {
                            p[i - 1].1 = 1;
                        }
                        state = 0;
                        classify = true;
                    }
                }
                3 => {
                    if matches!(e, 0x37 | 0x18 | 0x16 | 0x1f) {
                        state = 3;
                    } else if matches!(e, 0x2f | 0x15 | 0x29) || matches!(f, 0x88 | 0x8e | 0x89 | 0x8d | 0x8c) {
                        state = 5;
                    } else {
                        state = 0;
                        classify = true;
                    }
                }
                4 => {
                    if matches!(e, 0x18 | 0x16) {
                        state = 4;
                    } else if no_head(e, f) {
                        state = 7;
                    } else {
                        state = 0;
                        classify = true;
                    }
                }
                5 => {
                    if matches!(e, 0x37 | 0x2f | 0x15 | 0x29) || matches!(f, 0x88 | 0x8e | 0x89 | 0x8d | 0x8c) || same_around(w) {
                        state = 5;
                    } else if n2 == 0 && n4 == 0 {
                        fill(&mut p, start, i - 1);
                        p[i - 1].0 = 0;
                        if i != 0 && end4 == i - 1 {
                            p[i - 1].1 = 1;
                        } else if start == 0 {
                            p[i - 1].1 = 2;
                        }
                        state = 0;
                        classify = true;
                    }
                }
                6 => {
                    if matches!(e, 0x20 | 0x22 | 0x24 | 0x28 | 0x21 | 0x2e | 0x1f | 0x33) {
                        state = 1;
                    } else if verb(e) && !lit(f) && (i < 2 || !lit(b(w, i - 2, SUB))) {
                        state = 2;
                    } else if matches!(e, 0x18 | 0x16) {
                        state = 4;
                    } else if no_head(e, f) {
                        state = 7;
                    } else {
                        state = 0;
                        classify = true;
                    }
                }
                7 => {
                    if no_head(e, f) {
                        state = 7;
                    } else if n2 == 0 && n4 == 0 {
                        fill(&mut p, start, i - 1);
                        p[i - 1].0 = 2;
                        let mut one = false;
                        if start == 0 {
                            let e0 = b(w, 0, CLASS);
                            one = matches!(e0, 0x33 | 0x19 | 0x1e) || self.phrase2(w, 0).is_some_and(|n| (n as usize) < i);
                        }
                        if one || (i != 0 && end4 == i - 1) {
                            p[i - 1].1 = 1;
                        }
                        state = 0;
                        classify = true;
                    }
                }
                8 => {
                    p[i - 1].0 = 6;
                    state = 0;
                    classify = true;
                }
                _ => classify = true,
            }
            if classify {
                // 0x10009520
                start = i;
                if e == 0x1f {
                    state = 8;
                } else if matches!(e, 0x20 | 0x22 | 0x24 | 0x28 | 0x21 | 0x2e | 0x33) {
                    state = 1;
                } else if verb(e) && !lit(f) {
                    if !matches!(f, 0x8d | 0x8c | 0x8b) && i > 0 && b(w, i - 1, CLASS) == 0x2d {
                        p[i - 1].0 = 0;
                    }
                    if i != 0 && end4 == i - 1 {
                        p[i - 1].1 = 1;
                    }
                    state = 2;
                } else if matches!(e, 0x18 | 0x16) {
                    state = 3;
                } else if no_head(e, f) {
                    state = 5;
                } else if matches!(e, 0x19 | 0x1e) {
                    state = 6;
                } else if e == 0x32 {
                    p[i].0 = 8;
                }
            }
            i += 1;
        }
        p.push((10, 0));
        p
    }

    /// LesConfiniProsodici (0x10009fc0); `mode` is the sentence type
    /// (LES ctx+8).
    pub(crate) fn confini(&self, w: &mut [Word], mode: i32) {
        let p = self.phrases(w);
        let mut last = 200usize;
        let mut i = 0usize;
        while p[i].0 != 10 && i < 200 {
            if b(w, i, BOUND) == 0x15 {
                let e = b(w, i, CLASS);
                let (kind, sub) = p[i];
                // 0x1000a108
                let open = |s: &Self, w: &mut [Word], last: &mut usize| {
                    if sub == 1 && !s.short(w, i, i) && s.breakable(w, i) {
                        set(w, i, BOUND, 0x0a);
                    } else {
                        set(w, i, BOUND, 5);
                    }
                    *last = i;
                };
                match kind {
                    0 => {
                        if e == 0x37 {
                            set(w, i, BOUND, 0x16);
                        } else {
                            open(self, w, &mut last);
                        }
                    }
                    1 => {
                        if e == 0x37 {
                            set(w, i, BOUND, 0x16);
                        } else {
                            if last != 200 && mode != 3 && mode != 2 {
                                let lb = b(w, last, BOUND);
                                if matches!(lb, 3 | 5 | 7) && p[last] == (0, 2) && !self.short(w, i, last) && self.breakable(w, last) {
                                    set(w, last, BOUND, 0x0a);
                                }
                            }
                            open(self, w, &mut last);
                        }
                    }
                    2 => {
                        if e == 0x37 {
                            set(w, i, BOUND, 0x16);
                        } else {
                            set(w, i, BOUND, 5);
                            last = i;
                        }
                    }
                    4 => {
                        set(w, i, BOUND, 9);
                        last = i;
                    }
                    6 => {
                        if mode != 3 && mode != 2 && last != 200 && !self.short(w, i, last) && b(w, last, CLASS) != b(w, i + 1, CLASS) {
                            set(w, last, BOUND, 0x0a);
                        }
                        set(w, i, BOUND, 1);
                    }
                    7 => {
                        if e == 0x37 {
                            set(w, i, BOUND, 0x16);
                        } else if matches!(e, 0x19 | 0x1e | 0x18 | 0x20 | 0x21 | 0x16 | 0x22 | 0x1f | 0x2e) {
                            set(w, i, BOUND, 1);
                        } else {
                            set(w, i, BOUND, 3);
                        }
                    }
                    8 => self.punctuation(w, i, mode, &mut last),
                    9 => set(w, i, BOUND, 0x16),
                    _ => {
                        last = i;
                        set(w, i, BOUND, 3);
                    }
                }
            }
            if matches!(b(w, i, BOUND), 3 | 5) {
                match b(w, i, SUB) {
                    0x79 | 0x84 => set(w, i, BOUND, 7),
                    0x7a | 0x83 => set(w, i, BOUND, 6),
                    0x7b => set(w, i, BOUND, 8),
                    _ => {}
                }
            }
            i += 1;
        }
        set(w, i, BOUND, 0x17);
    }

    /// LesConfiniProsodici's punctuation case (kind 8, 0x1000a212): the
    /// boundary at a punctuation mark and at the word before it.
    fn punctuation(&self, w: &mut [Word], i: usize, mode: i32, last: &mut usize) {
        let mut prev = 0usize;
        if i > 0 {
            prev = i - 1;
            while prev > 0 && b(w, prev, BOUND) == 0x16 {
                prev -= 1;
            }
        }
        if b(w, prev, CLASS) == 0x32 {
            prev = i;
        }
        let both = |w: &mut [Word], v: u8| {
            set(w, prev, BOUND, v);
            set(w, i, BOUND, v);
        };
        if b(w, i, SUB) == 0x7c {
            match mode {
                0 => both(w, 0x0f),
                2 => both(w, 0x11),
                3 => both(w, 0x12),
                5 => both(w, 0x10),
                6 => both(w, 0x13),
                _ => set(w, i, BOUND, 0x16),
            }
        } else {
            let t = text(w, i).unwrap_or(b"");
            let mut k = 0usize;
            if at(t, 1) != 0 {
                loop {
                    if among(b",)(-.", at(t, k)) {
                        break;
                    }
                    k += 1;
                    if at(t, k + 1) == 0 {
                        break;
                    }
                }
            }
            let mut c = at(t, k);
            if c == b'.' {
                c = b',';
            }
            match c {
                b'(' | b')' if i > 0 && b(w, i - 1, CLASS) != 0x32 && b(w, i + 1, CLASS) != 0x32 => both(w, 0x0b),
                b',' | b'-' => both(w, 0x0b),
                _ => set(w, i, BOUND, 0x16),
            }
        }
        if b(w, i, BOUND) != 0x16 {
            *last = i;
        }
    }

    /// LesGradiAccentuali (0x10008130): stress grades (`+0x12`: 0 stressed,
    /// 3 unstressed, 2 after a 4 boundary, 0x0f punctuation), then the
    /// record after the last marked as the end (0x10, boundary 0x17).
    pub(crate) fn gradi(&self, w: &mut [Word]) {
        let s = |va: u32| self.s(va);
        let mut i = 0usize;
        let short_class = |c: u8| matches!(c, 0x19 | 0x1e | 0x18 | 0x20 | 0x21 | 0x16 | 0x22 | 0x1f | 0x82 | 0x2e);
        if b(w, 0, BOUND) != 0x17 {
            loop {
                if i >= 200 || text(w, i).is_none() {
                    break;
                }
                let e = b(w, i, CLASS);
                if e == 0x32 || e == 0x37 {
                    set(w, i, GRADE, 0x0f);
                    i += 1;
                } else {
                    let t = text(w, i).unwrap();
                    let apos = t.last() == Some(&b'\'');
                    let f = b(w, i, SUB);
                    if matches!(f, 0x79 | 0x7a)
                        && i > 1
                        && (b(w, i - 1, SUB) == 0x82 || stricmp_eq(s(0x10028028), text(w, i - 1).unwrap_or(b"")))
                        && b(w, i - 2, SUB) == 0x7a
                    {
                        set(w, i - 1, GRADE, 3);
                        set(w, i, GRADE, 3);
                        i += 1;
                    } else {
                        if b(w, i, BOUND) == 4 {
                            set(w, i, GRADE, 0);
                            i += 1;
                        }
                        if i != 0 && b(w, i - 1, BOUND) == 4 {
                            set(w, i, GRADE, 2);
                            i += 1;
                        } else {
                            let t = text(w, i).unwrap_or(b"").to_vec();
                            if t == s(0x1002a7a8) && i == 0 {
                                // "wèr" first
                                let short = short_class(b(w, 1, CLASS)) && text(w, 1).unwrap_or(b"").len() < 8;
                                set(w, 0, GRADE, if short { 0 } else { 3 });
                                i = 1;
                            } else {
                                if t == s(0x1002a7a4) || t == s(0x1001e594) {
                                    // ès, es
                                    if i == 0 {
                                        set(w, 0, GRADE, 3);
                                    } else {
                                        let n = b(w, i + 1, CLASS);
                                        set(w, i, GRADE, if n == 0x32 || n == 0x37 { 0 } else { 3 });
                                    }
                                } else {
                                    let bd = b(w, i, BOUND);
                                    let weak = matches!(bd, 2 | 1 | 0 | 6 | 7 | 3 | 0x16 | 5)
                                        && (short_class(b(w, i, CLASS)) || b(w, i, GRADE) == 3)
                                        && ![0x1002a7a0, 0x1002a79c, 0x1002a798, 0x1002a790, 0x1002a788, 0x100285c8, 0x1002a784]
                                            .iter()
                                            .any(|&a| t == s(a))
                                        && t.len() < 8;
                                    let g = if weak || b(w, i, SUB) == 0x83 || apos { 3 } else { 0 };
                                    set(w, i, GRADE, g);
                                }
                                i += 1;
                            }
                        }
                    }
                }
                if b(w, i, BOUND) == 0x17 {
                    break;
                }
            }
        }
        set(w, i, GRADE, 0x10);
        set(w, i, BOUND, 0x17);
    }
}

/// The analyser's feature attribute names (11) and value names (0x2c), the
/// tables 0x10017370 puts in its state (+0x14/+0x18, +0x1c/+0x20).
const FEATURE_NAMES: (u32, u8) = (0x1012d020, 0xb);
const FEATURE_VALUES: (u32, u8) = (0x1012d050, 0x2c);

/// Strings the analysis code compares with: morpheme kinds, feature
/// attributes and values.
const STEM: u32 = 0x10102f80;
const FLEX_SUFFIX: u32 = 0x10102f88;
const POS: u32 = 0x1002a920;
const GENDER: u32 = 0x10102fd4;
const NUMBER: u32 = 0x10102fcc;
const TENSE: u32 = 0x10103010;
const MOOD: u32 = 0x10103008;
const CASE: u32 = 0x10103000;
const ART: u32 = 0x1002a918;
const N: u32 = 0x100286b4;
const ADJ: u32 = 0x1002a91c;
const ADV: u32 = 0x10102f98;
const V: u32 = 0x10102f94;
const PRON: u32 = 0x10102eb4;
const NUM: u32 = 0x10102fa4;
/// The `{ (attribute, value) pairs, count }` lists 0x1000bba0 scores the
/// analyses with: POS=ART, POS=ADJ, POS=N.
const WANT_ART: u32 = 0x1002a950;
const WANT_ADJ: u32 = 0x1002a948;
const WANT_N: u32 = 0x1002a940;

/// C `strncmp(t + from, s, n) == 0`.
fn ncmp(t: &[u8], from: usize, s: &[u8], n: usize) -> bool {
    for j in 0..n {
        let (a, b) = (at(t, from + j), at(s, j));
        if a != b {
            return false;
        }
        if a == 0 {
            return true;
        }
    }
    true
}

/// The word analyser's feature accessors (the functions 0x100073f0 binds
/// by name from 0x101274d0 and LesAnalizza calls through module data
/// +0x14) and LesAnalizza's parts. A morpheme's field 1 is its kind
/// (STEM, FLEX_SUFFIX…), field 2 its feature set: a count byte, then at +4
/// the address of (attribute code, value code) byte pairs.
impl German {
    fn feature_pairs(&self, f: u32) -> impl Iterator<Item = (u8, u8)> + '_ {
        let p = self.pe.u32(f + 4);
        (0..self.pe.u8(f) as u32).map(move |k| (self.pe.u8(p + 2 * k), self.pe.u8(p + 2 * k + 1)))
    }

    /// getFeatureAttrString_byAttrCode (0x100176b0 → 0x100165a0).
    fn attr_name(&self, c: u8) -> Option<&[u8]> {
        (c < FEATURE_NAMES.1).then(|| self.s(self.pe.u32(FEATURE_NAMES.0 + 4 * c as u32)))
    }

    /// getFeatureValueString_byValueCode (0x100176c0 → 0x100165c0).
    fn value_name(&self, c: u8) -> Option<&[u8]> {
        (c < FEATURE_VALUES.1).then(|| self.s(self.pe.u32(FEATURE_VALUES.0 + 4 * c as u32)))
    }

    /// getMorphFeatureValueCode_byAttrString (0x100176f0 → 0x10016610): the
    /// value code of the first pair with attribute `attr`.
    fn value_code(&self, f: u32, attr: &[u8]) -> Option<u8> {
        if f == 0 {
            return None;
        }
        self.feature_pairs(f).find(|&(a, _)| self.attr_name(a) == Some(attr)).map(|(_, v)| v)
    }

    /// getMorphFeatureValueString_byAttrString (0x10017720 → 0x10016690):
    /// found, and its value's name (None past the value table).
    fn value_string(&self, f: u32, attr: u32) -> Option<Option<&[u8]>> {
        self.value_code(f, self.s(attr)).map(|v| self.value_name(v))
    }

    /// hasMorphFeature_byString (0x10017750 → 0x100166c0): the first pair
    /// with attribute `attr` has value `value`.
    fn has_feature(&self, f: u32, attr: u32, value: u32) -> bool {
        if f == 0 {
            return false;
        }
        let attr = self.s(attr);
        match self.feature_pairs(f).find(|&(a, _)| self.attr_name(a) == Some(attr)) {
            Some((_, v)) => self.value_name(v) == Some(self.s(value)),
            None => false,
        }
    }

    /// 0x10008040: morpheme `m` is of kind `kind`.
    fn kind(&self, m: &Morpheme, kind: u32) -> bool {
        self.s(m[1]) == self.s(kind)
    }

    /// 0x1000ba00: an analysis's value of `attr`, from its stem's features
    /// (the last stem), else its last flexion suffix's; "" when neither has
    /// it.
    fn morph_value(&self, a: &[Morpheme], attr: &[u8]) -> &[u8] {
        let (mut stem, mut suffix) = (None, None);
        for m in a.iter().rev() {
            if stem.is_some() {
                break;
            }
            if self.kind(m, FLEX_SUFFIX) {
                suffix.get_or_insert(m[2]);
            } else if self.kind(m, STEM) {
                stem = Some(m[2]);
            }
        }
        let code = self.value_code(stem.unwrap_or(0), attr).or_else(|| self.value_code(suffix.unwrap_or(0), attr));
        code.and_then(|c| self.value_name(c)).unwrap_or(b"")
    }

    /// 0x1000baf0: the POS of record `i`'s selected analysis (0x1000b9e0:
    /// none when the selection is past its analyses).
    fn pos(&self, w: &[Word], i: usize, sel: &[usize]) -> Vec<u8> {
        lexinfo(&w[i]).get(sel[i]).map_or(Vec::new(), |a| self.morph_value(a, self.s(POS)).to_vec())
    }

    /// 0x1000bcf0: the POS of each of record `i`'s analyses.
    fn pos_list(&self, w: &[Word], i: usize) -> Vec<Vec<u8>> {
        lexinfo(&w[i]).iter().map(|a| self.morph_value(a, self.s(POS)).to_vec()).collect()
    }

    /// 0x1000bb30: how many of `want`'s (attribute, value) pairs analysis
    /// `a` has.
    fn score(&self, a: &[Morpheme], want: u32) -> usize {
        let p = self.pe.u32(want);
        (0..self.pe.u8(want + 4) as u32).filter(|k| self.morph_value(a, self.s(self.pe.u32(p + 8 * k))) == self.s(self.pe.u32(p + 8 * k + 4))).count()
    }

    /// 0x1000bba0 (no list to avoid, selection kept): select record `rec`'s
    /// first best scoring analysis for `want`; its new POS when the
    /// selection changed (never to analysis 0).
    fn select(&self, w: &[Word], sel: &mut [usize], rec: Option<usize>, want: u32) -> Option<Vec<u8>> {
        let r = rec?;
        let need = self.pe.u8(want + 4) as usize;
        let (mut best, mut pick) = (0, sel[r]);
        for (k, a) in lexinfo(&w[r]).iter().enumerate() {
            let m = self.score(a, want);
            if m > best {
                best = m;
                pick = k;
                if m >= need {
                    break;
                }
            }
        }
        if pick > 0 && pick != sel[r] {
            sel[r] = pick;
            return Some(self.pos(w, r, sel));
        }
        None
    }

    /// 0x1000bd80: choose analyses from the neighbours' POS: an article
    /// before a noun (or adjective and noun), an adjective before a noun or
    /// pronoun, a capitalised noun. Records with boundary 4 are skipped
    /// without shifting the window, as in the original.
    fn choose(&self, w: &[Word], sel: &mut [usize]) {
        let has = |l: &[Vec<u8>], s: u32| l.iter().any(|x| x == self.s(s));
        let is = |p: &[u8], s: u32| p == self.s(s);
        let (mut cur, mut prev, mut next): (Option<usize>, Option<usize>, Option<usize>) = (None, None, None);
        let (mut cur_pos, mut prev_pos, mut next_pos) = (Vec::new(), Vec::new(), Vec::new());
        let (mut cur_l, mut prev_l, mut next_l): (Vec<Vec<u8>>, Vec<Vec<u8>>, Vec<Vec<u8>>) = Default::default();
        for i in 0..w.len() {
            if w[i].f[BOUND] == 4 {
                continue;
            }
            if i > 0 {
                prev = cur;
                prev_pos = cur_pos.clone();
                prev_l = cur_l.clone();
            }
            if cur.is_none() {
                cur = Some(i);
                cur_pos = self.pos(w, i, sel);
                cur_l = self.pos_list(w, i);
            } else {
                cur = next;
                cur_pos = next_pos.clone();
                cur_l = next_l.clone();
            }
            if i + 1 < w.len() && w[i + 1].f[CLASS] != 0x32 {
                next = Some(i + 1);
                next_pos = self.pos(w, i + 1, sel);
                next_l = self.pos_list(w, i + 1);
            } else {
                next = None;
                next_pos.clear();
                next_l.clear();
            }
            if prev.is_some() && !is(&prev_pos, ART) && has(&prev_l, ART) && (is(&cur_pos, N) || (is(&cur_pos, ADJ) && is(&next_pos, N))) {
                self.select(w, sel, prev, WANT_ART);
            } else if !is(&cur_pos, ADJ)
                && has(&cur_l, ADJ)
                && has(&cur_l, N)
                && !has(&cur_l, ADV)
                && !has(&cur_l, V)
                && (is(&next_pos, N) || is(&next_pos, PRON))
            {
                if let Some(p) = self.select(w, sel, cur, WANT_ADJ) {
                    cur_pos = p;
                }
            } else if !is(&cur_pos, N) && has(&cur_l, N) {
                let t = cur.and_then(|c| text(w, c)).unwrap_or(b"");
                if t.len() > 1 && self.is_upper(t[0]) && !self.is_upper(t[1]) {
                    if let Some(p) = self.select(w, sel, cur, WANT_N) {
                        cur_pos = p;
                    }
                }
            }
        }
    }

    /// 0x1000c5a0: class, gender/number and verb form of the records still
    /// without them, from their selected analysis's (last) stem and last
    /// flexion suffix; a number (POS=NUM) gets sub 0x79 and turns a 0x79
    /// before it into 0x7a. The NUM test reads the last morpheme the stem
    /// search looked at, kept from an earlier record when this one has a
    /// class already (the original's local is not reset per record).
    fn classes(&self, w: &mut [Word], sel: &[usize]) {
        let mut seen: Option<Morpheme> = None;
        for i in 0..w.len() {
            let Some(a) = lexinfo(&w[i]).get(sel[i]).cloned() else { continue };
            if a.is_empty() {
                continue;
            }
            let (mut stem, mut suffix) = (None, None);
            for m in a.iter().rev() {
                if stem.is_some() || w[i].f[CLASS] != 0x17 {
                    break;
                }
                seen = Some(*m);
                if self.kind(m, FLEX_SUFFIX) {
                    suffix.get_or_insert(m[2]);
                } else if self.kind(m, STEM) {
                    stem = Some(m[2]);
                }
            }
            let Some(m) = seen else { continue };
            let (stem, suffix) = (stem.unwrap_or(0), suffix.unwrap_or(0));
            if w[i].f[CLASS] == 0x17 {
                w[i].f[CLASS] = self.word_class(stem);
            }
            if w[i].f[INFO] == 9 {
                w[i].f[INFO] = self.gender_number(suffix, stem);
            }
            if w[i].f[SUB] == 0xc8 {
                w[i].f[SUB] = self.verb_form(suffix);
            }
            if self.has_feature(m[2], POS, NUM) {
                if i > 0 && w[i - 1].f[SUB] == 0x79 {
                    w[i - 1].f[SUB] = 0x7a;
                }
                w[i].f[SUB] = 0x79;
            }
        }
    }

    /// 0x1000c070: the class of a POS (N NUM ADJ ADV ART CONJ PREP PREPART
    /// PRON V).
    fn word_class(&self, f: u32) -> u8 {
        let Some(p) = self.value_string(f, POS) else { return 0xc8 };
        // N ADJ ADV ART CONJ NUM PREP PREPART PRON V
        let classes = [
            (N, 0x15),
            (ADJ, 0x29),
            (ADV, 0x33),
            (ART, 0x18),
            (0x10102fa8, 0x1f),
            (NUM, 0x15),
            (0x10102e18, 0x19),
            (0x10102f9c, 0x1e),
            (PRON, 0x20),
            (V, 0x2a),
        ];
        classes.iter().find(|&&(s, _)| p == Some(self.s(s))).map_or(0xc8, |&(_, c)| c)
    }

    /// 0x1000c1a0: gender and number (GENDER from the suffix, else the
    /// stem; NUMBER from the suffix).
    fn gender_number(&self, suffix: u32, stem: u32) -> u8 {
        let is = |x: Option<&[u8]>, s: u32| x == Some(self.s(s));
        let mut gender: Option<&[u8]> = Some(b"");
        let mut number: Option<&[u8]> = Some(b"");
        let mut g = self.value_string(suffix, GENDER).map(|v| gender = v).is_some();
        let n = self.value_string(suffix, NUMBER).map(|v| number = v).is_some();
        if !g {
            g = self.value_string(stem, GENDER).map(|v| gender = v).is_some();
        }
        if is(gender, 0x10102fc4) {
            // UNKNOWN
            g = false;
        }
        let (fem, mas, neu, sin, plu) = (0x10102fbc, 0x10102fb4, 0x10102fb0, 0x10102fb8, 0x10102fc0);
        if !g {
            return match n {
                false => 9,
                true if is(number, plu) => 0x10,
                true => 0x0f,
            };
        }
        for (gv, nv, r) in [(fem, sin, 0x0d), (fem, plu, 0x0e), (mas, sin, 0x0b), (mas, plu, 0x0c), (neu, sin, 0xd3), (neu, plu, 0xd2)] {
            if is(gender, gv) && is(number, nv) {
                return r;
            }
        }
        if is(gender, neu) {
            0xd4
        } else {
            9
        }
    }

    /// 0x1000c3a0: a verb form from MOOD and TENSE, else a case from CASE.
    fn verb_form(&self, f: u32) -> u8 {
        let is = |x: Option<&[u8]>, s: u32| x == Some(self.s(s));
        let tense = self.value_string(f, TENSE);
        let mood = self.value_string(f, MOOD);
        let case = self.value_string(f, CASE);
        let (ptc, pst, prs, imp, inf, ind) = (0x10102ffc, 0x10102ff8, 0x10102ff4, 0x10102ff0, 0x10102fec, 0x10102fe8);
        if let Some(m) = mood {
            return match tense {
                Some(t) if is(m, ptc) && is(t, pst) => 0x88,
                Some(t) if is(m, ptc) && is(t, prs) => 0x8e,
                _ if is(m, imp) => 0x87,
                _ if is(m, inf) => 0x8d,
                Some(t) if is(m, ind) && is(t, prs) => 0xd6,
                None if is(m, ind) => 0xd5,
                _ => 0xc8,
            };
        }
        let Some(c) = case else { return 0xc8 };
        // NOM GEN ACC DAT
        [(0x10102fe4, 0xd7), (0x10102fe0, 0xd8), (0x10102f68, 0xd9), (0x10102fdc, 0xda)].iter().find(|&&(s, _)| is(c, s)).map_or(0xc8, |&(_, r)| r)
    }

    /// 0x1000c720: before a number word (sub 0x79/0x7a), "eins" agrees with
    /// the word after the number; an article or a preposition with article
    /// before an ordinal (…ste, …te) gets an "n" on the ordinal, and so
    /// does "der" before a feminine one. Records with boundary 4 take the
    /// class, sub and info of the record after them. Stops at the first
    /// empty text.
    fn numbers(&self, w: &mut [Word]) {
        let n_end = self.s(0x1001e3e8); // "n"
        for i in 1..w.len() {
            let k = i - 1;
            let l0 = w[k].text.len();
            let l1 = w[k + 1].text.len();
            if l0 == 0 || l1 == 0 {
                return;
            }
            if matches!(w[k + 1].f[SUB], 0x79 | 0x7a) {
                if b(w, k + 2, BOUND) != 0x16 && w[k].text == self.s(0x10026acc) {
                    // "eins" → "eine" / "ein"
                    match b(w, k + 2, INFO) {
                        0x0e | 0x0d => w[k].text = self.s(0x10026ac0).to_vec(),
                        0x0c | 0x0b => w[k].text = b"ein".to_vec(),
                        _ => {}
                    }
                }
                let t1 = &w[k + 1].text;
                let ordinal = if l1 >= 3 && ncmp(t1, l1 - 3, self.s(0x10027c8c), 3) {
                    true
                } else {
                    l1 >= 2 && ncmp(t1, l1 - 2, self.s(0x1001e294), 2)
                };
                if ordinal {
                    if matches!(w[k].f[CLASS], 0x18 | 0x1e) {
                        let t = &w[k].text;
                        // es em en
                        if l0 >= 2 && [0x1001e594, 0x1001e5a8, 0x1001e5a4].iter().any(|&e| ncmp(t, l0 - 2, self.s(e), 2)) {
                            w[k + 1].text.extend_from_slice(n_end);
                        }
                        if matches!(w[k].f[INFO], 0x0e | 0x0d) && (l0 < 2 || !ncmp(&w[k].text, l0 - 2, self.s(0x10103018), 2)) {
                            // not …ie
                            w[k + 1].text.extend_from_slice(n_end);
                        }
                    }
                    if w[k].text == self.s(0x1001e5d8) && b(w, k + 2, INFO) == 0x0d {
                        // "der"
                        w[k + 1].text.extend_from_slice(n_end);
                        w[k].f[INFO] = 0x0d;
                    }
                }
            }
            if w[k].f[BOUND] == 4 {
                let (c, s, f) = (w[k + 1].f[CLASS], w[k + 1].f[SUB], w[k + 1].f[INFO]);
                for j in (0..=k).rev() {
                    if w[j].f[BOUND] != 4 {
                        break;
                    }
                    w[j].f[CLASS] = c;
                    w[j].f[SUB] = s;
                    w[j].f[INFO] = f;
                }
            }
        }
    }
}

/// 0x10001280: the vowel with the module's stress accent (grave on a e i o
/// u y, acute on ä ë ï ö ü).
fn accented(c: u8) -> u8 {
    match c {
        0x41 => 0xc0,
        0x45 => 0xc8,
        0x49 => 0xcc,
        0x4f => 0xd2,
        0x55 => 0xd9,
        0x59 => 0xdd,
        0x61 => 0xe0,
        0x65 => 0xe8,
        0x69 => 0xec,
        0x6f => 0xf2,
        0x75 => 0xf9,
        0x79 => 0xfd,
        0xc4 => 0xc1,
        0xcb => 0xc9,
        0xcf => 0xcd,
        0xd6 => 0xd3,
        0xdc => 0xda,
        0xe4 => 0xe1,
        0xeb => 0xe9,
        0xef => 0xed,
        0xf6 => 0xf3,
        0xfc => 0xfa,
        x => x,
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::*;

    /// Every traced call of each LES stage: its input records (with their
    /// LexInfo) in, its output records (text, bytes 0x0c..0x14) out.
    #[test]
    #[ignore]
    fn les_stages_match_the_module() {
        let Some(de) = german() else { return };
        let mut stats = std::collections::BTreeMap::new();
        for line in lines("les.txt") {
            let mut cols = line.split('\t');
            let head: Vec<&str> = cols.next().unwrap().split(' ').collect();
            let (inp, out, lex) = (cols.next().unwrap_or(""), cols.next().unwrap_or(""), cols.next().unwrap_or(""));
            let mut got = records_with_lex(inp, lex);
            let want = records(out);
            match head[0] {
                "LesGestisciMaiuscole" => de.gestisci_maiuscole(&mut got),
                "LesAnalizza" => de.analizza(&mut got),
                "LesAccenta" => de.accenta(&mut got),
                "LesConfiniProsodici" => de.confini(&mut got, head[1].parse().unwrap()),
                "LesGradiAccentuali" => de.gradi(&mut got),
                _ => continue,
            }
            let e = stats.entry(head[0].to_string()).or_insert((0, 0));
            e.0 += 1;
            let same = got.len() == want.len() && got.iter().zip(&want).all(|(g, w)| g.text == w.text && g.f[0xc..0x14] == w.f[0xc..0x14]);
            if !same {
                e.1 += 1;
                if e.1 <= 10 {
                    eprintln!("{}:\n  in   {:?}\n  want {:?}\n  got  {:?}", head[0], records(inp).iter().map(show).collect::<Vec<_>>(), want.iter().map(show).collect::<Vec<_>>(), got.iter().map(show).collect::<Vec<_>>());
                }
            }
        }
        for (k, (n, bad)) in &stats {
            eprintln!("{k}: {bad} of {n} calls differ");
        }
        assert!(stats.values().all(|&(_, bad)| bad == 0));
    }
}
