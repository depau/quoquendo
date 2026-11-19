//! The LES side of LTTS7Polish.dll: the word classes (LesAnalizza), the
//! written stress (LesAccenta), the prosodic boundaries
//! (LesConfiniProsodici), the stress grades (LesGradiAccentuali), the
//! accent normalisation, LesPronunciabile and LesGetSymbolName.
//!
//! The engine's records live in an array that ends with a record whose
//! text is NULL (the engine's sentinel); reads past either end of the
//! sentence see that sentinel's zeros here.

use super::{Polish, ACCENTED, VOWEL};
use crate::lang::common::{at, field::*, lookup, Endings, Word};

/// LesAnalizza's closed-class words (0x1000e470: `FUN_1000e410`, a binary
/// search by `strcmp` over 0x201 entries `{ word, class, sub, info,
/// grade }` of 8 bytes).
const WORDS: u32 = 0x1003d960;
const WORD_COUNT: u32 = 0x201;

/// LesAnalizza's ending table for the words still unknown (0x1000e500 →
/// 0x1000e3d0: 0x334 entries `{ reversed ending, exception list, code }`,
/// searched as Italian's, `common::lookup`).
const ENDINGS: Endings = Endings { at: 0x1003b2f0, count: 0x334 };

/// LesAccenta's word lists (`ELQSearchString` flag 0x11, a linear exact
/// search) and the vowel each stresses, counted from the end: `dobra
/// etui…` 1, `rzeczpospolita…` 3, `bylibyśmy…` 4, `potrzeby…` 2.
const STRESS_LISTS: [(u32, u32, i32); 4] = [(0x1001e608, 5, 1), (0x1001e620, 0x163, 3), (0x1001ebb0, 0x37, 4), (0x1001ec90, 0x29, 2)];
/// `nie` and `nié` (0x1001ed3c, 0x1001ed38): a monosyllabic verb after
/// them is not stressed, the `nie` is.
const NIE: [u32; 2] = [0x1001ed3c, 0x1001ed38];

/// LesConfiniProsodici's multi-word expressions (0x1000cb60: 4, 3 and 2
/// words; 0x1000c980: 3, 2 and 1 words), each `(table, count)`.
const EXPR_4: (u32, u32) = (0x10021adc, 4);
const EXPR_3: (u32, u32) = (0x10021af0, 0x27);
const EXPR_2: (u32, u32) = (0x10021b90, 0x7f);
const INTERJ_3: (u32, u32) = (0x10021ab0, 1);
const INTERJ_2: (u32, u32) = (0x10021ab4, 1);
const INTERJ_1: (u32, u32) = (0x10021ab8, 9);
/// 0x1000ce00's state 2 keeps a group open over `a`, `al`, `de`, `del`
/// (0x10020f64, 0x10021dc0, 0x10020f50, 0x10021dbc) before a verb.
const KEEP_OPEN: [u32; 4] = [0x10020f64, 0x10021dc0, 0x10020f50, 0x10021dbc];
/// 0x1000c780: `môże` (0x10021d8c).
const MOZE: u32 = 0x10021d8c;

/// LesGetSymbolName's switch (0x100088f0): the character's signed value +
/// 0x80 indexes the bytes at 0x10008c90 (above 0xfe the default), which
/// index the jump table at 0x10008b70; each case is `mov eax, name; ret`,
/// or `test al, al; je default; mov eax, name; ret` for the names given
/// only in mode ≠ 0. The default (0x10008b67) answers `""`.
const SYMBOL_INDEX: u32 = 0x10008c90;
const SYMBOL_JUMPS: u32 = 0x10008b70;
/// LesPronunciabile (0x10008e00): `z`, `w` (0x100210dc, 0x100210d8) read
/// as words on their own.
const LONE_LETTERS: [u32; 2] = [0x100210dc, 0x100210d8];

fn rec(w: &[Word], i: isize) -> Option<&Word> {
    if i < 0 {
        None
    } else {
        w.get(i as usize)
    }
}

fn cls(w: &[Word], i: isize) -> u8 {
    rec(w, i).map_or(0, |r| r.f[CLASS])
}

fn sub(w: &[Word], i: isize) -> u8 {
    rec(w, i).map_or(0, |r| r.f[SUB])
}

/// The classes 0x1000ce00 groups as verbs (`*`, `+`, `,`).
fn verb(c: u8) -> bool {
    matches!(c, 0x2a | 0x2b | 0x2c)
}

/// The verb subclasses 0x1000ce00 treats as not finite (0x88, 0x8e, 0x89).
fn nonfinite(s: u8) -> bool {
    matches!(s, 0x88 | 0x8e | 0x89)
}

impl Polish {
    /// LTTS7Util's `ELQSearchString` (0x10028ca0) with flag 1 (whole key):
    /// a binary search by `strcmp` over more than 3 entries, else linear;
    /// with flag 0x10 always linear.
    pub(crate) fn search_string(&self, key: &[u8], t: u32, flags: u32, n: u32) -> Option<u32> {
        let e = |k: u32| self.s(self.pe.u32(t + 4 * k));
        if n > 3 && flags & 0x1c == 0 {
            let (mut lo, mut hi) = (0i64, n as i64 - 1);
            while lo <= hi {
                let mid = (lo + hi) / 2;
                match key.cmp(e(mid as u32)) {
                    std::cmp::Ordering::Less => hi = mid - 1,
                    std::cmp::Ordering::Greater => lo = mid + 1,
                    std::cmp::Ordering::Equal => return Some(mid as u32),
                }
            }
            return None;
        }
        (0..n).find(|&k| key == e(k))
    }

    /// `LesAnalizza` (0x1000e640): 0x1000e470 gives the closed-class words
    /// (a record of class 0x17, a proper noun 0x15/0xc8, or subclass `z`)
    /// their class, subclass, information and grade from the word table;
    /// then 0x1000e600 classifies every record still 0x17 by its ending
    /// (0x1000e500: 1 adjective 0x29, 2 adverb 0x33, 3 verb 0x2a with
    /// subclass 0x8b, 5 homograph 0x2d, 6 verb with subclass 0x88, 7 verb,
    /// else 0x2f).
    pub(super) fn analizza(&self, w: &mut [Word]) {
        for r in w.iter_mut() {
            let (c, s) = (r.f[CLASS], r.f[SUB]);
            if !(c == 0x17 || (c == 0x15 && s == 0xc8) || s == 0x7a) {
                continue;
            }
            let (mut lo, mut hi) = (0i32, WORD_COUNT as i32 - 1);
            while lo <= hi {
                let mid = (lo + hi) / 2;
                let e = WORDS + 8 * mid as u32;
                match r.text.as_slice().cmp(self.s(self.pe.u32(e))) {
                    std::cmp::Ordering::Less => hi = mid - 1,
                    std::cmp::Ordering::Greater => lo = mid + 1,
                    std::cmp::Ordering::Equal => {
                        r.f[CLASS] = self.pe.u8(e + 4);
                        r.f[SUB] = self.pe.u8(e + 5);
                        r.f[INFO] = self.pe.u8(e + 6);
                        r.f[GRADE] = self.pe.u8(e + 7);
                        break;
                    }
                }
            }
        }
        for r in w.iter_mut() {
            if r.f[CLASS] != 0x17 {
                continue;
            }
            // the text is copied into 64 bytes and reversed
            let rev: Vec<u8> = r.text.iter().rev().copied().collect();
            match lookup(&self.pe, ENDINGS, &rev) {
                1 => r.f[CLASS] = 0x29,
                2 => r.f[CLASS] = 0x33,
                3 => {
                    r.f[SUB] = 0x8b;
                    r.f[CLASS] = 0x2a;
                }
                5 => r.f[CLASS] = 0x2d,
                6 => {
                    r.f[CLASS] = 0x2a;
                    r.f[SUB] = 0x88;
                }
                7 => r.f[CLASS] = 0x2a,
                _ => r.f[CLASS] = 0x2f,
            }
        }
    }

    /// 0x10001000: the vowels of a word, an `i` before another vowel not
    /// counted.
    fn vowels(&self, t: &[u8]) -> i32 {
        (0..t.len()).filter(|&k| self.has(t[k], VOWEL) && (t[k] != b'i' || !self.has(at(t, k + 1), VOWEL))).count() as i32
    }

    /// 0x100011c0 (EAX `n`, EDI the text): the `n`-th vowel from the end
    /// (counted as 0x10001000 does) gets its accented form (0x10001050);
    /// without one, the first letter; nothing for `n` ≤ 0.
    fn mark(&self, t: &mut [u8], n: i32) {
        if n <= 0 {
            return;
        }
        let (mut n, mut pos) = (n, 0usize);
        let mut k = t.len() as i32 - 1;
        while k >= 0 && n > 0 {
            let c = t[k as usize];
            if self.has(c, VOWEL) && (c != b'i' || !self.has(at(t, k as usize + 1), VOWEL)) {
                n -= 1;
                pos = k as usize;
            }
            k -= 1;
        }
        if let Some(c) = t.get_mut(pos) {
            *c = accented(*c);
        }
    }

    /// `LesAccenta` (0x10001280): every record not elided (+0xc 0xffff)
    /// whose text has no accented letter (0x10001240) gets one vowel marked
    /// (0x100011c0): the word lists' vowel; `arcy…` of three vowels, `eks…`
    /// of two, `wice…` of three the last; a verb ending in `byśmy`
    /// `byście` the fourth from the end, `śmy` `ście` `bym` `byś` `by` the
    /// third; any word ending in `eś`, `set`, `kroć` the third; else the
    /// last of one vowel, the second from the end of more; a verb of one
    /// vowel after `nie` none, and the `nie` grade 0. A text longer than
    /// 63 characters stops the function.
    pub(super) fn accenta(&self, w: &mut [Word]) {
        for i in 0..w.len() {
            if w[i].f[0xc] != 0xff || w[i].f[0xd] != 0xff {
                continue;
            }
            if w[i].text.iter().any(|&c| self.has(c, ACCENTED)) {
                continue;
            }
            let t = w[i].text.clone();
            let n = self.vowels(&t);
            let k = if n <= 0 {
                0
            } else {
                if t.len() > 0x3f {
                    return;
                }
                match self.stress(w, i, &t, n) {
                    Some(k) => k,
                    None => {
                        // nie + a verb of one vowel: grade 0 for the `nie`
                        w[i - 1].f[GRADE] = 0;
                        0
                    }
                }
            };
            self.mark(&mut w[i].text, k);
        }
    }

    /// LesAccenta's choice for record `i` (text `t`, `n` vowels): the vowel
    /// to mark from the end, `None` for a one-vowel verb after `nie`.
    fn stress(&self, w: &[Word], i: usize, t: &[u8], n: i32) -> Option<i32> {
        for &(table, count, k) in &STRESS_LISTS {
            if self.search_string(t, table, 0x11, count).is_some() {
                return Some(k);
            }
        }
        let starts = |p: &[u8]| t.len() > p.len() && t.starts_with(p);
        if (starts(b"arcy") && n == 3) || (starts(b"eks") && n == 2) || (starts(b"wice") && n == 3) {
            return Some(1);
        }
        // the reversed text in 64 bytes: `r(k)` its k-th byte
        let rev: Vec<u8> = t.iter().rev().copied().collect();
        let r = |k: usize| at(&rev, k);
        // `ends(e)`: the text ends with `e` and has more letters before it
        let ends = |e: &[u8]| {
            let n = e.len();
            (0..n).all(|k| r(k) == e[n - 1 - k]) && r(n) != 0
        };
        let class = w[i].f[CLASS];
        if class == 0x2a {
            if ends(b"by\x9cmy") || ends(b"by\x9ccie") {
                return Some(4);
            }
            if ends(b"\x9cmy") || ends(b"\x9ccie") || ends(b"bym") || ends(b"by\x9c") {
                return Some(3);
            }
        }
        if class == 0x2a && r(0) == b'y' {
            if ends(b"by") {
                return Some(3);
            }
        } else if ends(b"e\x9c") || ends(b"set") || ends(b"kro\xe6") {
            return Some(3);
        }
        if i != 0 {
            let prev = &w[i - 1].text;
            let nie = NIE.iter().any(|&a| prev.as_slice() == self.s(a));
            if nie && class == 0x2a {
                return if n == 1 { None } else { Some(2) };
            }
        }
        Some(if n == 1 { 1 } else { 2 })
    }

    /// `LesConfiniProsodici` (0x1000d470): the boundary after each word
    /// from the phrase grouping 0x1000ce00 makes (`groups`): 0, 1, 2 a
    /// phrase's last word, 5 (a 10, a pause, before a 1 or 6 when the
    /// phrase before is long, 0x1000c650, and may break, 0x1000c780); 3
    /// and 5 inside a group 3; 4 9; 6 1; 7 1 for function words, else 3; 8
    /// punctuation (as the engine's own, `common::confini_default`); class
    /// `7` 0x16. Then subclasses `y`/0x84 7, `z`/0x83 6, `{` 8.
    pub(super) fn confini(&self, w: &mut [Word], mode: i32) {
        let g = self.groups(w);
        let mut last = 200usize;
        let mut i = 0usize;
        while g[i].0 != 10 && i <= 199 {
            if w[i].f[BOUND] == 0x15 {
                let c = w[i].f[CLASS];
                match g[i].0 {
                    0 | 1 | 2 => {
                        if c == b'7' {
                            w[i].f[BOUND] = 0x16;
                        } else {
                            if g[i].0 == 1 && last != 200 && g[last].1 == 0 && mode != 3 && mode != 2 {
                                let b = w[last].f[BOUND];
                                if matches!(b, 3 | 5 | 7) && g[last].0 != 5 && !self.short(w, last, i) && breaks(self, w, last) {
                                    w[last].f[BOUND] = 10;
                                }
                            }
                            // (type 2 with flag 1 may set 10 here, which
                            // the 5 below overwrites)
                            w[i].f[BOUND] = 5;
                            last = i;
                        }
                    }
                    4 => {
                        w[i].f[BOUND] = 9;
                        last = i;
                    }
                    6 => {
                        if mode != 3 && mode != 2 && last != 200 {
                            let b = w[last].f[BOUND];
                            if matches!(b, 3 | 5 | 7) && !self.short(w, last, i) && g[i + 1].0 != 10 && w[last].f[CLASS] != cls(w, i as isize + 1) {
                                w[last].f[BOUND] = 10;
                            }
                        }
                        w[i].f[BOUND] = 1;
                    }
                    7 => {
                        w[i].f[BOUND] = if c == b'7' {
                            0x16
                        } else if matches!(c, 0x19 | 0x1e | 0x18 | 0x20 | 0x21 | 0x16 | 0x22 | 0x1f | 0x2e) {
                            1
                        } else {
                            3
                        };
                    }
                    8 => {
                        punct(w, i, mode);
                        if w[i].f[BOUND] != 0x16 {
                            last = i;
                        }
                    }
                    9 => w[i].f[BOUND] = 0x16,
                    _ => {
                        last = i;
                        w[i].f[BOUND] = 3;
                    }
                }
            }
            if matches!(w[i].f[BOUND], 3 | 5) {
                match w[i].f[SUB] {
                    0x79 | 0x84 => w[i].f[BOUND] = 7,
                    0x7a | 0x83 => w[i].f[BOUND] = 6,
                    0x7b => w[i].f[BOUND] = 8,
                    _ => {}
                }
            }
            i += 1;
        }
    }

    /// 0x1000ce00: the phrase grouping, `(type, flag)` per record and 10
    /// after the last (see `confini`), by a state machine over the word
    /// classes; 0x1000cb60 marks multi-word expressions, inside which a
    /// group is not closed.
    fn groups(&self, w: &[Word]) -> Vec<(u32, u32)> {
        let n = w.len();
        let mut out = vec![(5u32, 0u32); n + 2];
        if n == 0 {
            out[0].0 = 10;
            return out;
        }
        // fill [a, b) with (7, 0) (0x1000c620)
        fn fill(out: &mut [(u32, u32)], a: usize, b: usize) {
            for k in a..b {
                out[k] = (7, 0);
            }
        }
        let (mut expr_len, mut expr_pos) = (0usize, 0usize);
        let mut state = 0u32;
        let mut first = 0usize; // local_c: where the open group started
        let mut i = 0usize;
        loop {
            out[i] = (5, 0);
            if expr_len == 0 {
                if let Some(len) = self.expression(w, i) {
                    expr_len = len;
                    expr_pos = 1;
                }
            } else if expr_pos < expr_len {
                expr_pos += 1;
            } else if expr_pos == expr_len {
                expr_len = 0;
            }
            let ii = i as isize;
            let c = w[i].f[CLASS];
            let s = w[i].f[SUB];
            let prev_first = first;
            // `None`: classify the record afresh (LAB_1000d304)
            let mut classify = false;
            match state {
                1 => {
                    if matches!(c, b'7' | b' ' | b'"' | b'$' | b'(' | b'!' | b'.' | 0x1f | b'3') {
                        state = 1;
                    } else if verb(c) && !nonfinite(s) {
                        state = 2;
                    } else {
                        state = 0;
                        classify = true;
                    }
                }
                2 => {
                    let coord = s == 0x82 && cls(w, ii - 1) == cls(w, ii + 1) && sub(w, ii - 1) == sub(w, ii + 1);
                    if matches!(c, b'7' | 0x2a | 0x2b | 0x2c | b'3') || coord {
                        state = 2;
                    } else {
                        let keep = KEEP_OPEN.iter().any(|&a| w[i].text.as_slice() == self.s(a));
                        if keep && verb(cls(w, ii + 1)) {
                            state = 2;
                        } else if expr_len == 0 {
                            fill(&mut out, prev_first, i - 1);
                            out[i - 1].0 = 1;
                            state = 0;
                            classify = true;
                        }
                    }
                }
                3 => {
                    if matches!(c, b'7' | 0x18 | 0x16 | 0x1f) {
                        state = 3;
                    } else if matches!(c, b'/' | 0x15 | b' ' | b')') || matches!(s, 0x88 | 0x8e | 0x89 | 0x8d | 0x8c) {
                        state = 5;
                    } else {
                        state = 0;
                        classify = true;
                    }
                }
                4 => {
                    if matches!(c, 0x18 | 0x16 | 0x1f) {
                        state = 4;
                    } else if matches!(c, b'7' | b'/' | 0x15) || c == b' ' || c == b')' || nonfinite(s) {
                        state = 7;
                    } else {
                        state = 0;
                        classify = true;
                    }
                }
                5 => {
                    let coord = s == 0x82 && cls(w, ii - 1) == cls(w, ii + 1) && sub(w, ii - 1) == sub(w, ii + 1);
                    if matches!(c, b'7' | b'/' | 0x15 | b' ' | b')') || matches!(s, 0x88 | 0x8e | 0x89 | 0x8d | 0x8c) || coord {
                        state = 5;
                    } else if expr_len == 0 {
                        fill(&mut out, prev_first, i - 1);
                        out[i - 1].0 = 0;
                        state = 0;
                        classify = true;
                    }
                }
                6 => {
                    if matches!(c, b' ' | b'"' | b'$' | b'(' | b'!' | b'.' | 0x1f | b'3') {
                        state = 1;
                    } else if !verb(c) || nonfinite(s) || (i > 1 && nonfinite(sub(w, ii - 2))) {
                        if matches!(c, 0x18 | 0x16) {
                            state = 4;
                        } else if matches!(c, b'7' | b'/') || c == 0x15 || c == b' ' || c == b')' || nonfinite(s) {
                            state = 7;
                        } else {
                            state = 0;
                            classify = true;
                        }
                    } else {
                        state = 2;
                    }
                }
                7 => {
                    if matches!(c, b'/' | 0x15 | b' ' | b')' | b'7') || nonfinite(s) {
                        state = 7;
                    } else if expr_len == 0 {
                        fill(&mut out, prev_first, i - 1);
                        out[i - 1].0 = 2;
                        let flag = prev_first == 0
                            && (matches!(w[0].f[CLASS], b'3' | 0x19)
                                || self.expression_at(w, 0, &[EXPR_4, EXPR_3, EXPR_2], 5, 4).is_some_and(|l| l < i)
                                || self.expression_at(w, 0, &[INTERJ_3, INTERJ_2, INTERJ_1], 3, 3).is_some_and(|l| l < i));
                        if flag {
                            out[i - 1].1 = 1;
                        }
                        state = 0;
                        classify = true;
                    }
                }
                8 => {
                    out[i - 1].0 = 6;
                    state = 0;
                    classify = true;
                }
                _ => classify = true,
            }
            if classify {
                first = i;
                state = 0;
                if c == 0x1f {
                    state = 8;
                } else if matches!(c, b' ' | b'"' | b'$' | b'(' | b'!' | b'.' | b'3') {
                    state = 1;
                } else if verb(c) && !nonfinite(s) {
                    if !matches!(s, 0x8d | 0x8c | 0x8b) && i != 0 && cls(w, ii - 1) == b'-' {
                        out[i - 1].0 = 0;
                    }
                    state = 2;
                } else if matches!(c, 0x18 | 0x16) {
                    state = 3;
                } else if matches!(c, b'7' | b'/' | 0x15 | b')') || nonfinite(s) {
                    state = 5;
                } else if matches!(c, 0x19 | 0x1e) {
                    state = 6;
                } else if c == b'2' {
                    out[i].0 = 8;
                }
            }
            i += 1;
            if i >= n {
                out[i].0 = 10;
                return out;
            }
        }
    }

    /// 0x1000cb60 at record `i`: up to five records' texts joined by
    /// spaces and unaccented (0x1000c920); four of them among the 4-word
    /// expressions, then three, then two: the expression's length. Five
    /// records (the sentence goes on) match nothing.
    fn expression(&self, w: &[Word], i: usize) -> Option<usize> {
        self.expression_at(w, i, &[EXPR_4, EXPR_3, EXPR_2], 5, 4)
    }

    /// 0x1000cb60 (`tables` of `top` = 4, 3, 2 words, up to `max` 5
    /// records joined) and 0x1000c980 (3/2/1 words, `max` 3) at record
    /// `i`: only `top`, `top - 1` or `top - 2` records joined are searched.
    fn expression_at(&self, w: &[Word], i: usize, tables: &[(u32, u32); 3], max: usize, top: usize) -> Option<usize> {
        let words: Vec<&[u8]> = w[i.min(w.len())..].iter().take(max).map(|r| r.text.as_slice()).collect();
        let count = words.len();
        if count != top && count != top - 1 && count != top - 2 {
            return None;
        }
        let mut s: Vec<u8> = words.join(&b' ').iter().map(|&c| self.unaccented(c)).collect();
        for (k, &(t, n)) in tables.iter().enumerate() {
            let len = top - k;
            if count < len {
                continue;
            }
            if self.search_string(&s, t, 1, n).is_some() {
                return Some(len);
            }
            // the last word dropped (the first space from the end)
            let p = s.iter().rposition(|&c| c == b' ').unwrap_or(0);
            s.truncate(p);
        }
        None
    }

    /// 0x1000e660 on accented letters (0x1000c920, 0x1000e780…): the
    /// letter without its stress mark (`ã` `ą`, `ě` `ę`, `ú` `u`…), in its
    /// case.
    pub(crate) fn unaccented(&self, c: u8) -> u8 {
        if !self.has(c, ACCENTED) {
            return c;
        }
        let up = self.has(c, super::UPPER);
        let l = if up { self.lower(c) } else { c };
        let m = match l {
            0xc3 => 0xa5,
            0xc9 => 0x45,
            0xcc => 0xca,
            0xcd => 0x49,
            0xd4 => 0x4f,
            0xda => 0x55,
            0xdd => 0x59,
            0xe1 => 0x61,
            0xe3 => 0xb9,
            0xe9 => 0x65,
            0xec => 0xea,
            0xed => 0x69,
            0xf4 => 0x6f,
            0xfa => 0x75,
            0xfd => 0x79,
            x => x,
        };
        if up {
            self.upper(m)
        } else {
            m
        }
    }

    /// 0x1000c650 (EBX `a`, the argument `b`): whether the phrases around
    /// a break after `a` are short: the words back from `a` to a boundary
    /// 0x0a/0x0b (skipping 0x16s) and those after `b` to the punctuation
    /// (not class `7`), with the words from `a + 1` to `b`; short unless
    /// both have more than 3 words and more than 14 letters.
    fn short(&self, w: &[Word], a: usize, b: usize) -> bool {
        let (mut before_len, mut before_words) = (w[a].text.len(), 1usize);
        let mut j = a;
        while j > 0 {
            j -= 1;
            let bound = w[j].f[BOUND];
            if bound == 0x16 {
                continue;
            }
            if bound == 0x0a || bound == 0x0b {
                break;
            }
            before_len += w[j].text.len();
            before_words += 1;
        }
        let (mut after_len, mut after_words) = (0usize, 0usize);
        let mut j = b + 1;
        while j < w.len() && w[j].f[CLASS] != b'2' {
            if w[j].f[CLASS] != b'7' {
                after_words += 1;
                after_len += w[j].text.len();
            }
            j += 1;
        }
        for j in (a + 1..=b).rev() {
            if w[j].f[CLASS] != b'7' {
                after_len += w[j].text.len();
                after_words += 1;
            }
        }
        !(before_words > 3 && before_len > 0xe && after_words > 3 && after_len > 0xe)
    }

    /// `LesPronunciabile` (0x10008f30 → 0x10008e00): with `` ` `` or `'`,
    /// an accented letter or a vowel other than `aeiouy` (ą ę ó…) it
    /// reads; `z` and `w` alone read; else when it has a vowel (0x10008dd0,
    /// over its first 63 letters lower-cased).
    pub(super) fn pronunciabile(&self, t: &[u8]) -> bool {
        if t.iter().any(|&c| c == b'`' || c == b'\'') {
            return true;
        }
        let mut low = Vec::new();
        for &c in t.iter().take(0x3f) {
            if self.has(c, ACCENTED) {
                return true;
            }
            if self.has(c, VOWEL) && !b"aeiouyAEIOUY".contains(&c) {
                return true;
            }
            low.push(self.lower(c));
        }
        if low.len() == 1 && LONE_LETTERS.iter().any(|&a| low.as_slice() == self.s(a)) {
            return true;
        }
        low.iter().any(|&c| self.has(c, VOWEL))
    }

    /// `LesGetSymbolName` (0x10008d90 → 0x100088f0, mode in AL): the name
    /// of a letter or symbol, read through the switch's own tables.
    pub(super) fn symbol_name(&self, c: u8, mode: u8) -> Vec<u8> {
        let k = (c as i8 as i32) + 0x80;
        if k > 0xfe {
            return Vec::new();
        }
        let t = self.pe.u32(SYMBOL_JUMPS + 4 * self.pe.u8(SYMBOL_INDEX + k as u32) as u32);
        let code = self.pe.at(t);
        let name = match code {
            [0xb8, ..] => self.pe.u32(t + 1),
            [0x84, 0xc0, 0x0f, 0x84, _, _, _, _, 0xb8, ..] if mode != 0 => self.pe.u32(t + 9),
            [0x84, 0xc0, ..] => return Vec::new(),
            _ => return Vec::new(),
        };
        self.s(name).to_vec()
    }
}

/// 0x1000c780 (EAX `i`): whether a phrase may break after record `i`: yes
/// before the sentence's end; no between a conjunction 0x1f and a
/// preposition after a noun/pronoun/adjective, or the same class on both
/// sides; no for `môże` before a verb; else no for the pairs of classes
/// that hold together (`)`/0x15, `)`/`/`, verb + adverb/noun/preposition or
/// + an infinitive, … + a verb of subclass 0x8b, adverb + verb, a noun or
/// pronoun + preposition, pronoun + verb).
fn breaks(p: &Polish, w: &[Word], i: usize) -> bool {
    let ii = i as isize;
    if rec(w, ii + 1).is_none() {
        return true;
    }
    let c = w[i].f[CLASS];
    let (pc, nc, ns) = (cls(w, ii - 1), cls(w, ii + 1), sub(w, ii + 1));
    if i > 0 && c == 0x1f && ((matches!(pc, b'/' | 0x15 | b' ' | b')') && nc == 0x19) || pc == nc) {
        return false;
    }
    let moze = w[i].text.as_slice() == p.s(MOZE);
    if moze && verb(nc) {
        return false;
    }
    let holds = (c == b')' && nc == 0x15)
        || (c == 0x15 && nc == b')')
        || (c == b')' && nc == b'/')
        || (c == b'/' && nc == b')')
        || (verb(c) && (matches!(nc, b'3' | 0x15 | 0x19) || (verb(nc) && ns == 0x88)))
        || (matches!(c, b')' | 0x2a | 0x19 | b' ' | 0x15 | b'/') && verb(nc) && ns == 0x8b)
        || (c == b'3' && verb(nc))
        || (matches!(c, b'/' | 0x15 | b' ' | b')') && nc == 0x19);
    if holds {
        return false;
    }
    !(c == b' ' && verb(nc))
}

/// LesConfiniProsodici's punctuation (type 8): the boundary of the
/// punctuation record and of the word before it (skipping 0x16s, not when
/// that is punctuation) by the sentence type at `|`, else 0x0b at `,` `-`
/// `.` and at `(` `)` between words, 0x16 otherwise.
fn punct(w: &mut [Word], i: usize, mode: i32) {
    let mut j = i;
    if i != 0 {
        j = i - 1;
        while j != 0 && w[j].f[BOUND] == 0x16 {
            j -= 1;
        }
    }
    if w[j].f[CLASS] == b'2' {
        j = i;
    }
    let set = |w: &mut [Word], v: u8| {
        w[j].f[BOUND] = v;
        w[i].f[BOUND] = v;
    };
    if w[i].f[SUB] == b'|' {
        match mode {
            0 => set(w, 0x0f),
            2 => set(w, 0x11),
            3 => set(w, 0x12),
            5 => set(w, 0x10),
            6 => set(w, 0x13),
            _ => w[i].f[BOUND] = 0x16,
        }
        return;
    }
    let t = &w[i].text;
    let mut k = 0;
    while at(t, k + 1) != 0 && !b",)(-.".contains(&at(t, k)) {
        k += 1;
    }
    let c = match at(t, k) {
        b'.' => b',',
        c => c,
    };
    match c {
        b'(' | b')' => {
            if i == 0 || w[i - 1].f[CLASS] == b'2' || w.get(i + 1).is_some_and(|r| r.f[CLASS] == b'2') {
                w[i].f[BOUND] = 0x16;
            } else {
                set(w, 0x0b);
            }
        }
        b',' | b'-' => set(w, 0x0b),
        _ => w[i].f[BOUND] = 0x16,
    }
}

/// `LesGradiAccentuali` (0x1000c510): punctuation and class `7` 0x0f; a
/// word of grade 3 (from the word table) after a boundary 0–3, 5–7 or
/// 0x16 keeps it when it has at most 7 letters; subclass 0x83 3; else 0.
pub(super) fn gradi(w: &mut [Word]) {
    for r in w.iter_mut().take(200) {
        let c = r.f[CLASS];
        if c == b'2' || c == b'7' {
            r.f[GRADE] = 0x0f;
            continue;
        }
        let b = r.f[BOUND];
        let keep = matches!(b, 0..=3 | 5..=7 | 0x16) && r.f[GRADE] == 3 && r.text.len() <= 7;
        r.f[GRADE] = if keep || r.f[SUB] == 0x83 { 3 } else { 0 };
    }
}

/// `LesNormalizzaAccento` (0x100088c0): `` ` ``, `'`, `’` and `´` become
/// `'`.
pub(super) fn normalizza_accento(t: &mut [u8]) {
    for c in t.iter_mut() {
        if matches!(*c, b'`' | b'\'' | 0x92 | 0xb4) {
            *c = b'\'';
        }
    }
}

/// 0x10001050: a vowel with its stress mark, in the module's notation
/// (`a` `á`, `ą` `ã`, `ę` `ě`, `o` `ô`, `ó` and `u` `ú`, …).
fn accented(c: u8) -> u8 {
    match c {
        0x41 => 0xc1,
        0x45 => 0xc9,
        0x49 => 0xcd,
        0x4f => 0xd4,
        0x59 => 0xdd,
        0x61 => 0xe1,
        0x65 => 0xe9,
        0x69 => 0xed,
        0x6f => 0xf4,
        0x79 => 0xfd,
        0xa5 => 0xc3,
        0xb9 => 0xe3,
        0xca => 0xcc,
        0xd3 | 0x55 => 0xda,
        0xea => 0xec,
        0xf3 | 0x75 => 0xfa,
        x => x,
    }
}
