//! `LesConfiniProsodici` (0x1000ba30): the prosodic boundary after each
//! word (record +0x11), from the kinds of word groups 0x1000b1e0 finds.

use super::French;
use crate::lang::common::{field::*, Word};

/// Locutions 0x10009060 looks for (string addresses, as the arrays it
/// builds on its stack): three words, then two.
const LOC3_A: &[u32] = &[
    0x1001f918, 0x1001f90c, 0x1001f8fc, 0x1001f720, 0x1001f704, 0x1001f6c0, 0x1001f648, 0x1001f634, 0x1001f60c, 0x1001f5d4,
    0x1001f58c, 0x1001f528, 0x1001f290,
];
const LOC2_A: &[u32] = &[
    0x1001f490, 0x1001f474, 0x1001f458, 0x1001f464, 0x1001f8e4, 0x1001f3dc, 0x1001f3c4, 0x1001f8dc, 0x1001f39c, 0x1001f390,
    0x1001f378, 0x1001f344, 0x1001f334, 0x1001f328, 0x1001f8c4, 0x1001f2f4, 0x1001f2d8,
];
/// Locutions 0x100088f0 looks for: six words down to two.
const LOC6: &[u32] = &[0x1001f880];
const LOC5: &[u32] = &[0x1001f864];
const LOC4: &[u32] = &[
    0x1001f854, 0x1001f844, 0x1001f830, 0x1001f81c, 0x1001f8b0, 0x1001f80c, 0x1001f7f8, 0x1001f7e8, 0x1001f7d4, 0x1001f89c,
    0x1001f7c4,
];
const LOC3: &[u32] = &[
    0x1001f7b8, 0x1001f7a8, 0x1001f79c, 0x1001f790, 0x1001f780, 0x1001f770, 0x1001f760, 0x1001f750, 0x1001f740, 0x1001f730,
    0x1001f720, 0x1001f710, 0x1001f704, 0x1001f6f8, 0x1001f6e8, 0x1001f6d4, 0x1001f6c0, 0x1001f6b0, 0x1001f6a4, 0x1001f694,
    0x1001f684, 0x1001f66c, 0x1001f658, 0x1001f8b0, 0x1001f648, 0x1001f634, 0x1001f628, 0x1001f61c, 0x1001f60c, 0x1001f5fc,
    0x1001f5e8, 0x1001f5d4, 0x1001f5c8, 0x1001f5bc, 0x1001f5ac, 0x1001f5a0, 0x1001f58c, 0x1001f57c, 0x1001f56c, 0x1001f560,
    0x1001f54c, 0x1001f538, 0x1001f528, 0x1001f518, 0x1001f50c, 0x1001f500, 0x1001f4f4, 0x1001f4e4, 0x1001f4d8, 0x1001f89c,
    0x1001f4cc, 0x1001f4bc,
];
const LOC2: &[u32] = &[
    0x1001f4b4, 0x1001f4a8, 0x1001f49c, 0x1001f490, 0x1001f484, 0x1001f474, 0x1001f464, 0x1001f458, 0x1001f44c, 0x1001f440,
    0x1001f430, 0x1001f424, 0x1001f41c, 0x1001f414, 0x1001f40c, 0x1001f400, 0x1001f3f4, 0x1001f3e4, 0x1001f3dc, 0x1001f3d0,
    0x1001f3c4, 0x1001f3b8, 0x1001f3a8, 0x1001f39c, 0x1001f390, 0x1001f384, 0x1001f378, 0x1001f36c, 0x1001f360, 0x1001f358,
    0x1001f350, 0x1001f344, 0x1001f334, 0x1001f328, 0x1001f31c, 0x1001f310, 0x1001f304, 0x1001f2f4, 0x1001f2e8, 0x1001f2e0,
    0x1001f2d8, 0x1001f2c8, 0x1001f2c0, 0x1001f2b4, 0x1001f2ac, 0x1001f2a0, 0x1001f290,
];

/// Verb classes (0x2a 0x2c 0x2b 0xd8).
fn verb(c: u8) -> bool {
    matches!(c, 0x2a | 0x2c | 0x2b | 0xd8)
}

/// Noun-like classes (0x2f 0x15 0xd9 0x29 0xdb 0xda 0xe7 0xe8).
fn noun(c: u8) -> bool {
    matches!(c, 0x2f | 0x15 | 0xd9 | 0x29 | 0xdb | 0xda | 0xe7 | 0xe8)
}

/// Participle and gerund subclasses (0x88 0x8e 0x89).
fn participle(s: u8) -> bool {
    matches!(s, 0x88 | 0x8e | 0x89)
}

/// Pronoun classes 0x1000b1e0 groups (0x20 0x22 0xe2 0x24 0x28 0x21).
fn pronoun(c: u8) -> bool {
    matches!(c, 0x20 | 0x22 | 0xe2 | 0x24 | 0x28 | 0x21)
}

/// The classes that open a clause group (state 1).
fn clause(c: u8) -> bool {
    pronoun(c) || matches!(c, 0x2e | 0x82 | 0xe6 | 0xe1 | 0xe0 | 0xe9 | 0x33)
}

/// Infinitive subclasses (0x8d 0x8c).
fn infinitive(s: u8) -> bool {
    matches!(s, 0x8d | 0x8c)
}

/// A record's text, `None` past the last record (NULL text).
fn tx(w: &[Word], i: isize) -> Option<&[u8]> {
    if i < 0 {
        return None;
    }
    w.get(i as usize).map(|r| &r.text[..])
}

fn f(w: &[Word], i: isize, k: usize) -> u8 {
    super::fld(w, i, k)
}

impl French {
    /// LTTS7Util's `ELQSearchString` (0x10028ca0) with flag 1 (whole
    /// strings): over more than 3 entries a binary search, else a scan.
    fn search_string(&self, key: &[u8], list: &[u32]) -> bool {
        let n = list.len() as i32;
        if n > 3 {
            let (mut lo, mut hi) = (0i32, n - 1);
            loop {
                let mid = (lo + hi) / 2;
                match key.cmp(self.s(list[mid as usize])) {
                    std::cmp::Ordering::Less => hi = mid - 1,
                    std::cmp::Ordering::Equal => return true,
                    std::cmp::Ordering::Greater => lo = mid + 1,
                }
                if hi < lo {
                    return false;
                }
            }
        }
        list.iter().any(|&a| self.s(a) == key)
    }

    /// The words from `u` to the end (at most 6) joined by spaces, and
    /// their count (0x10009060, 0x100088f0).
    fn joined(w: &[Word], u: usize) -> (Vec<u8>, usize) {
        let mut s = Vec::new();
        let mut n = 0;
        for r in &w[u..] {
            if n > 5 {
                break;
            }
            if n != 0 {
                s.push(b' ');
            }
            s.extend(&r.text);
            n += 1;
        }
        (s, n)
    }

    /// Cut a joined string at its last space (none: empty).
    fn drop_last(s: &mut Vec<u8>) {
        let p = s.iter().rposition(|&c| c == b' ').unwrap_or(0);
        s.truncate(p);
    }

    /// 0x10009060: a locution of 3 or 2 words starting at `u` — only when
    /// that many records are left in the sentence.
    fn locution_a(&self, w: &[Word], u: usize) -> Option<u16> {
        let (mut s, n) = Self::joined(w, u);
        match n {
            3 => {
                if self.search_string(&s, LOC3_A) {
                    return Some(3);
                }
                Self::drop_last(&mut s);
            }
            2 => {}
            _ => return None,
        }
        self.search_string(&s, LOC2_A).then_some(2)
    }

    /// 0x100088f0: a locution of 6 down to 2 words starting at `u`, the
    /// search starting at the number of records left.
    fn locution_b(&self, w: &[Word], u: usize) -> Option<u16> {
        let (mut s, n) = Self::joined(w, u);
        if !(2..=6).contains(&n) {
            return None;
        }
        let lists: [(&[u32], u16); 5] = [(LOC6, 6), (LOC5, 5), (LOC4, 4), (LOC3, 3), (LOC2, 2)];
        for &(list, len) in &lists[6 - n..] {
            if self.search_string(&s, list) {
                return Some(len);
            }
            if len > 2 {
                Self::drop_last(&mut s);
            }
        }
        None
    }

    /// 0x1000b1e0: per word a kind (4 by default; 0, 1, 2, 5 for the
    /// group a word closes, 6 inside a group, 7 punctuation; 9 after the
    /// last) and a flag (a locution ended right before).
    fn confini_kinds(&self, w: &[Word]) -> Vec<(i32, i32)> {
        let n = w.len();
        let mut out = vec![(4, 0); n + 1];
        out[n] = (9, 0);
        let (mut len_a, mut cnt_a) = (0u16, 0u16);
        let (mut len_b, mut cnt_b) = (0u16, 0u16);
        let mut after_loc = 0usize;
        let mut start = 0usize;
        let mut state = 0;
        // the group that ends before `u`: its words become 6, the last `k`
        let close = |out: &mut Vec<(i32, i32)>, start: usize, u: usize, k: i32, after_loc: usize| {
            for x in out.iter_mut().take(u - 1).skip(start) {
                x.0 = 6;
            }
            out[u - 1].0 = k;
            if after_loc != 0 && after_loc == u - 1 {
                out[u - 1].1 = 1;
            }
        };
        for u in 0..n {
            let ui = u as isize;
            out[u] = (4, 0);
            if len_a == 0 {
                if let Some(l) = self.locution_a(w, u) {
                    len_a = l;
                    cnt_a = 1;
                }
            } else if cnt_a < len_a {
                cnt_a += 1;
            } else if cnt_a == len_a {
                len_a = 0;
                after_loc = u;
            }
            if len_b == 0 {
                if let Some(l) = self.locution_b(w, u) {
                    len_b = l;
                    cnt_b = 1;
                }
            } else if cnt_b < len_b {
                cnt_b += 1;
            } else if cnt_b == len_b {
                len_b = 0;
            }
            let c = w[u].f[CLASS];
            let sub = w[u].f[SUB];
            let in_loc = len_a != 0 || len_b != 0;
            let same_sides =
                |w: &[Word]| f(w, ui - 1, CLASS) == f(w, ui + 1, CLASS) && f(w, ui - 1, SUB) == f(w, ui + 1, SUB);
            // what the state machine does with this word: classify it, or
            // jump straight to a state
            let mut classify = true;
            match state {
                1 => {
                    if !(c == 0x37 || c == 0x1f || clause(c)) {
                        if verb(c) && !participle(sub) {
                            state = 2;
                            classify = false;
                        } else {
                            state = 0;
                        }
                    } else {
                        state = 1;
                        classify = false;
                    }
                }
                2 => {
                    let open = !matches!(c, 0x37 | 0x2a | 0x2c | 0x2b | 0xd8 | 0x33) && (c != 0x82 || !same_sides(w));
                    if !open {
                        state = 2;
                        classify = false;
                    } else {
                        let t = &w[u].text[..];
                        let prep = [0x1001f950, 0x1001f94c, 0x1001f948, 0x1001f944, 0x1001f270, 0x1001da30, 0x1001d5ac]
                            .iter()
                            .any(|&a| self.s(a) == t);
                        if prep && verb(f(w, ui + 1, CLASS)) {
                            state = 2;
                            classify = false;
                        } else if !in_loc {
                            close(&mut out, start, u, 1, after_loc);
                            state = 0;
                        } else {
                            classify = false;
                        }
                    }
                }
                3 => {
                    if !matches!(c, 0x37 | 0x18 | 0x16 | 0x1f) {
                        if noun(c) || participle(sub) || infinitive(sub) {
                            state = 4;
                            classify = false;
                        } else {
                            state = 0;
                        }
                    } else {
                        state = 3;
                        classify = false;
                    }
                }
                4 => {
                    let open =
                        !(c == 0x37 || noun(c) || participle(sub) || infinitive(sub)) && (c != 0x82 || !same_sides(w));
                    if open {
                        if in_loc {
                            classify = false;
                        } else {
                            close(&mut out, start, u, 0, after_loc);
                            state = 0;
                        }
                    } else {
                        state = 4;
                        classify = false;
                    }
                }
                5 => {
                    if c == 0x1f || clause(c) {
                        state = 1;
                        classify = false;
                    } else if verb(c) && !participle(sub) && (u < 2 || !participle(f(w, ui - 2, SUB))) {
                        state = 2;
                        classify = false;
                    } else if c == 0x37 || noun(c) || participle(sub) {
                        state = 6;
                        classify = false;
                    } else {
                        state = 0;
                    }
                }
                6 => {
                    if noun(c) || participle(sub) || c == 0x37 {
                        state = 6;
                        classify = false;
                    } else if in_loc {
                        classify = false;
                    } else {
                        // the last word's kind; no locution flag here
                        for x in out.iter_mut().take(u - 1).skip(start) {
                            x.0 = 6;
                        }
                        out[u - 1].0 = 2;
                        state = 0;
                    }
                }
                7 => {
                    out[u - 1].0 = 5;
                    state = 0;
                }
                _ => classify = state == 0,
            }
            if !classify {
                continue;
            }
            start = u;
            if c == 0x1f {
                state = 7;
            } else if clause(c) {
                state = 1;
            } else if verb(c) && !participle(sub) {
                if !matches!(sub, 0x8d | 0x8c | 0x8b) && u != 0 && f(w, ui - 1, CLASS) == 0x2d {
                    out[u - 1].0 = 0;
                }
                state = 2;
            } else if matches!(c, 0x18 | 0x16) {
                state = 3;
            } else if c == 0x37 || noun(c) || participle(sub) {
                state = 4;
            } else if matches!(c, 0x19 | 0x1e) {
                state = 5;
            } else if c == b'2' {
                out[u].0 = 7;
            }
        }
        out
    }

    /// 0x10009c70: 0 when both sides of a candidate pause are long enough
    /// (more than one word and 14 letters before, since the last pause;
    /// more than one word and 19 letters after, to the next punctuation).
    fn short_sides(&self, w: &[Word], idx: usize, arg: usize) -> bool {
        let mut len_a = w[idx].text.len();
        let mut cnt_a = 1;
        for j in (0..idx).rev() {
            let b = w[j].f[BOUND];
            if b != 0x16 {
                if matches!(b, 10 | 11) {
                    break;
                }
                len_a += w[j].text.len();
                cnt_a += 1;
            }
        }
        let (mut len_b, mut cnt_b) = (0usize, 0usize);
        let mut j = arg + 1;
        while j < w.len() && w[j].f[CLASS] != b'2' {
            if w[j].f[CLASS] != 0x37 {
                cnt_b += 1;
                len_b += w[j].text.len();
            }
            j += 1;
        }
        for j in (idx + 1..=arg).rev() {
            if w[j].f[CLASS] != 0x37 {
                len_b += w[j].text.len();
                cnt_b += 1;
            }
        }
        !(cnt_a > 1 && len_a > 0xe && cnt_b > 1 && len_b > 0x13)
    }

    /// 0x100092f0: whether a pause may follow word `idx` (no: before an
    /// infinitive after a preposition, between a noun and its verb, around
    /// `y`, `pas`, `ne`, `que`, comparisons, before `de`/`des`/`d'`/`du`).
    fn pause_ok(&self, w: &[Word], idx: usize) -> bool {
        let i = idx as isize;
        let Some(nt) = tx(w, i + 1) else { return false };
        let is = |t: Option<&[u8]>, a: u32| t.is_some_and(|t| t == self.s(a));
        let (de, des, d_, du, en, dans, a) = (0x1001f950, 0x1001f94c, 0x1001f948, 0x1001f944, 0x1001f270, 0x1001da30, 0x1001d5ac);
        let (que, qu_, qui, y) = (0x1001f940, 0x1001f93c, 0x1001f924, 0x1001dbf4);
        if [de, des, d_, du, en, dans, a].iter().any(|&x| is(Some(nt), x))
            && tx(w, i + 2).is_some()
            && (infinitive(f(w, i + 2, SUB))
                || (matches!(f(w, i + 2, CLASS), 0x19 | 0x1e | 0x18 | 0x20 | 0x21 | 0x16 | 0x22 | 0x1f | 0x82 | 0x2e)
                    && tx(w, i + 3).is_some()
                    && infinitive(f(w, i + 3, SUB))))
        {
            return false;
        }
        let c = f(w, i, CLASS);
        let nc = f(w, i + 1, CLASS);
        let ns = f(w, i + 1, SUB);
        if (noun(c) || matches!(c, 0x19 | 0x1e)) && verb(nc) {
            return false;
        }
        if pronoun(c) && infinitive(ns) {
            return false;
        }
        if verb(c) && pronoun(nc) && tx(w, i + 2).is_some() && infinitive(f(w, i + 2, SUB)) {
            return false;
        }
        if idx != 0 {
            let pc = f(w, i - 1, CLASS);
            if verb(pc) && pronoun(c) && infinitive(ns) {
                return false;
            }
            let que_next = is(Some(nt), que) || is(Some(nt), qu_);
            if (noun(pc) || verb(pc)) && que_next {
                return false;
            }
            if (noun(c) || verb(c)) && que_next {
                return false;
            }
        }
        let t = &w[idx].text[..];
        if t == b"faire" && infinitive(ns) {
            return false;
        }
        if t == self.s(y) || is(Some(nt), y) || is(tx(w, i - 1), y) {
            return false;
        }
        if is(Some(nt), 0x1001f264) || is(Some(nt), 0x1001f930) || is(Some(nt), 0x1001f92c) {
            return false;
        }
        if t == self.s(0x1001f928) && (is(Some(nt), qui) || is(Some(nt), qu_) || is(Some(nt), que)) {
            return false;
        }
        let compar = |t: Option<&[u8]>| t.is_some_and(|t| t == b"moins" || t == self.s(0x1001c3c4) || t == b"aussi");
        if compar(Some(t)) && nc == 0x29 && is(tx(w, i + 2), que) {
            return false;
        }
        if idx != 0 && compar(tx(w, i - 1)) && c == 0x29 && is(Some(nt), que) {
            return false;
        }
        ![de, des, d_, du].iter().any(|&x| is(Some(nt), x))
    }

    pub(crate) fn confini(&self, w: &mut [Word], mode: i32) {
        let kinds = self.confini_kinds(w);
        let question = matches!(mode, 2 | 3);
        let mut last = 200usize;
        for arg in 0..w.len() {
            if kinds[arg].0 == 9 {
                break;
            }
            let ai = arg as isize;
            let elided = w[arg].text.last() == Some(&b'\'');
            if w[arg].f[BOUND] == 0x15 {
                let class = w[arg].f[CLASS];
                match kinds[arg].0 {
                    0 | 2 => {
                        if class == 0x37 {
                            w[arg].f[BOUND] = 0x16;
                        } else {
                            w[arg].f[BOUND] = 5;
                            last = arg;
                        }
                    }
                    1 => {
                        if class == 0x37 {
                            w[arg].f[BOUND] = 0x16;
                        } else {
                            // a pause (10) after the group before this one
                            let mut weak = question || last == 200;
                            if !question && last != 200 {
                                let cb = w[last].f[BOUND];
                                if !matches!(cb, 3 | 5 | 7) || self.short_sides(w, last, arg) || !self.pause_ok(w, last) {
                                    weak = true;
                                } else {
                                    let nt = tx(w, last as isize + 1);
                                    let q = nt.is_some_and(|t| t == self.s(0x1001f924) || t == self.s(0x1001f940));
                                    if q && kinds[last].0 == 0 {
                                        weak = true;
                                    } else {
                                        w[last].f[BOUND] = 10;
                                    }
                                }
                            }
                            if weak && !question && last != 200 && kinds[last].1 == 1 {
                                w[last].f[BOUND] = 10;
                            }
                            w[arg].f[BOUND] = 5;
                            last = arg;
                        }
                    }
                    3 => {
                        w[arg].f[BOUND] = 9;
                        last = arg;
                    }
                    5 => {
                        if !question && last != 200 {
                            let cb = w[last].f[BOUND];
                            if matches!(cb, 3 | 5 | 7)
                                && !self.short_sides(w, last, arg)
                                && w[last].f[CLASS] != f(w, ai + 1, CLASS)
                            {
                                w[last].f[BOUND] = 10;
                            }
                        }
                        w[arg].f[BOUND] = 1;
                    }
                    6 => {
                        if class == 0x37 {
                            w[arg].f[BOUND] = 0x16;
                        } else if matches!(class, 0x19 | 0x1e | 0x18 | 0x20 | 0x21 | 0x16 | 0x22 | 0x1f | 0x82 | 0x2e) {
                            w[arg].f[BOUND] = 1;
                        } else {
                            w[arg].f[BOUND] = 3;
                        }
                    }
                    7 => {
                        let mut p = arg.saturating_sub(1);
                        while p != 0 && w[p].f[BOUND] == 0x16 {
                            p -= 1;
                        }
                        if w[p].f[CLASS] == b'2' {
                            p = arg;
                        }
                        let both = |w: &mut [Word], b: u8| {
                            w[p].f[BOUND] = b;
                            w[arg].f[BOUND] = b;
                        };
                        if w[arg].f[SUB] == b'|' {
                            match mode {
                                0 => both(w, 0xf),
                                2 => both(w, 0x11),
                                3 => both(w, 0x12),
                                5 => both(w, 0x10),
                                6 => both(w, 0x13),
                                _ => w[arg].f[BOUND] = 0x16,
                            }
                        } else {
                            let t = &w[arg].text;
                            let mut k = 0usize;
                            while super::b(t, k as isize + 1) != 0 && !super::chr(b",)(-.", super::b(t, k as isize)) {
                                k += 1;
                            }
                            let mut ch = super::b(t, k as isize);
                            if ch == b'.' {
                                ch = b',';
                            }
                            match ch {
                                b'(' | b')' => {
                                    if arg == 0 || f(w, ai - 1, CLASS) == b'2' || f(w, ai + 1, CLASS) == b'2' {
                                        w[arg].f[BOUND] = 0x16;
                                    } else {
                                        both(w, 0xb);
                                    }
                                }
                                b',' | b'-' => both(w, 0xb),
                                _ => w[arg].f[BOUND] = 0x16,
                            }
                        }
                        if w[arg].f[BOUND] != 0x16 {
                            last = arg;
                        }
                    }
                    8 => w[arg].f[BOUND] = 0x16,
                    _ => {
                        last = arg;
                        w[arg].f[BOUND] = 3;
                    }
                }
            }
            if matches!(w[arg].f[BOUND], 3 | 5) {
                match w[arg].f[SUB] {
                    0x79 | 0x84 => w[arg].f[BOUND] = 7,
                    0x7a | 0x83 => w[arg].f[BOUND] = 6,
                    0x7b => w[arg].f[BOUND] = 8,
                    _ => {}
                }
            }
            if elided {
                w[arg].f[BOUND] = 0;
            }
        }
    }
}
