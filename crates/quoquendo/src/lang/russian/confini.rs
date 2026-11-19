//! `LesConfiniProsodici` (0x10003d80): the boundary after each word. A
//! first pass (0x10003a60) groups the words by class into kinds (fixed
//! expressions from 0x100037b0 kept whole); the second sets each word's
//! boundary by its kind, splitting long phrases (0x100035b0, 0x10003690)
//! with boundary 0x0a, punctuation by the sentence type.

use super::Russian;
use crate::lang::common::{field::*, Word};

/// Fixed expressions of five, four, three and two words (0x100037b0; the
/// first two lists hold a placeholder): (table, entries).
const EXPRESSIONS: [(u32, u32, u16); 4] = [(0x10066624, 1, 5), (0x10066628, 1, 4), (0x10066630, 0x17, 3), (0x10066690, 0x6d, 2)];
/// `,)(-.` (0x100668dc): the punctuation characters that make a pause.
const PAUSE_PUNCT: u32 = 0x100668dc;

fn in3(c: u8, a: u8, b: u8, d: u8) -> bool {
    c == a || c == b || c == d
}

impl Russian {
    /// LTTS7Util's `ELQSearchString` (0x10028ca0) with flag 1: over more
    /// than 3 entries a binary search, else a scan.
    fn search_string(&self, key: &[u8], table: u32, n: u32) -> bool {
        let entry = |i: i32| self.s(self.pe.u32(table + 4 * i as u32));
        if n > 3 {
            let (mut lo, mut hi) = (0i32, n as i32 - 1);
            while lo <= hi {
                let mid = (lo + hi) / 2;
                match key.cmp(entry(mid)) {
                    std::cmp::Ordering::Less => hi = mid - 1,
                    std::cmp::Ordering::Greater => lo = mid + 1,
                    std::cmp::Ordering::Equal => return true,
                }
            }
            return false;
        }
        (0..n as i32).any(|i| key == entry(i))
    }

    /// 0x100037b0: the fixed expression starting at word `u` (the words
    /// up to five, joined by spaces, then shortened a word at a time), its
    /// length in words.
    fn expression(&self, w: &[Word], u: usize) -> Option<u16> {
        let mut s: Vec<u8> = Vec::new();
        let mut n = 0u16;
        for r in &w[u..] {
            if n > 4 {
                break;
            }
            if n != 0 {
                s.push(b' ');
            }
            s.extend_from_slice(&r.text);
            n += 1;
        }
        let cut = |s: &mut Vec<u8>| {
            let mut p = s.len().saturating_sub(1);
            while p != 0 && s[p] != b' ' {
                p -= 1;
            }
            s.truncate(p);
        };
        if n < 2 {
            return None;
        }
        for &(table, count, len) in &EXPRESSIONS {
            if n < len {
                continue;
            }
            if self.search_string(&s, table, count) {
                return Some(len);
            }
            if len > 2 {
                cut(&mut s);
            }
        }
        None
    }

    /// 0x10003a60: each word's kind (5 by default; 7 inside a group, 1 a
    /// group of `*+,` words after `0xc9`, 0 a group of nouns `/` `0x15`
    /// `)`, 2 a prepositional group (its `x` 4 at the sentence start), 6
    /// the word before `. 0x1f 3`, 8 punctuation), and 10 after the last.
    fn kinds(&self, w: &[Word]) -> Vec<(u32, u32)> {
        let mut out: Vec<(u32, u32)> = Vec::with_capacity(w.len() + 1);
        let (mut len, mut at): (u16, u16) = (0, 0);
        let mut state = 0u32;
        let mut start = 0usize;
        let fill = |out: &mut Vec<(u32, u32)>, from: usize, to: usize| {
            for k in from..to {
                out[k] = (7, 0);
            }
        };
        for u in 0..w.len() {
            out.push((5, 0));
            if len == 0 {
                if let Some(l) = self.expression(w, u) {
                    len = l;
                    at = 1;
                }
            } else if at < len {
                at += 1;
            } else if at == len {
                len = 0;
            }
            let c = w[u].f[CLASS];
            let noun = in3(c, b'/', 0x15, b')');
            let adj = in3(c, b'*', b'+', b',');
            let end = in3(c, b'.', 0x1f, b'3');
            // the default state's classification (LAB_10003ccf), from
            // where a state gives up
            let mut classify = false;
            match state {
                1 => {
                    if adj {
                        state = 2;
                    } else {
                        classify = true;
                    }
                }
                2 => {
                    if adj {
                        state = 2;
                    } else if len == 0 {
                        fill(&mut out, start, u - 1);
                        out[u - 1].0 = 1;
                        classify = true;
                    }
                }
                3 => {
                    if end {
                        state = 3;
                    } else if noun {
                        state = 4;
                    } else {
                        classify = true;
                    }
                }
                4 => {
                    if noun {
                        state = 4;
                    } else if end && w[u - 1].f[CLASS] == w.get(u + 1).map_or(0, |n| n.f[CLASS]) {
                        state = 4;
                    } else if len == 0 {
                        fill(&mut out, start, u - 1);
                        out[u - 1].0 = 0;
                        classify = true;
                    }
                }
                5 => {
                    if adj && u < 2 {
                        state = 2;
                    } else if noun {
                        state = 7;
                    } else {
                        classify = true;
                    }
                }
                6 => {
                    if end {
                        state = 6;
                    } else if noun {
                        state = 7;
                    } else {
                        classify = true;
                    }
                }
                7 => {
                    if noun {
                        state = 7;
                    } else if len == 0 {
                        fill(&mut out, start, u - 1);
                        out[u - 1].0 = 2;
                        let first = w[0].f[CLASS];
                        if start == 0 && (first == b'3' || first == 0x19 || self.expression(w, 0).is_some_and(|l| (l as usize) < u)) {
                            out[u - 1].1 = 4;
                        }
                        classify = true;
                    }
                }
                8 => {
                    out[u - 1].0 = 6;
                    classify = true;
                }
                _ => classify = true,
            }
            if classify {
                state = 0;
                start = u;
                if end {
                    state = 8;
                } else if noun {
                    state = 4;
                } else if c == 0xc9 {
                    state = 1;
                } else if adj {
                    state = 2;
                } else if c == 0x19 || c == 0x1e {
                    state = 5;
                } else if c == b'2' {
                    out[u].0 = 8;
                }
            }
        }
        out.push((10, 0));
        out
    }

    /// 0x100035b0: whether the phrase from the boundary `l` back is too
    /// short to split before word `u + 1` (fewer than two words, at most 9
    /// letters, or at most one word left before punctuation).
    fn short_phrase(&self, w: &[Word], l: usize, u: usize) -> bool {
        let text = |k: usize| w.get(k).map(|r| r.text.as_slice());
        let mut len = text(l).map_or(0, |t| t.len());
        let mut count = 1;
        let mut k = l;
        while k > 0 {
            k -= 1;
            let b = w[k].f[BOUND];
            if b != 0x16 {
                if b == 0x0a || b == 0x0b {
                    break;
                }
                len += w[k].text.len();
                count += 1;
            }
        }
        let mut rest = 0;
        let mut j = u + 1;
        while j < w.len() {
            if w[j].f[CLASS] == b'2' {
                break;
            }
            rest += 1;
            j += 1;
        }
        if l < u {
            rest += u - l;
        }
        !(count > 1 && len > 9 && rest > 1)
    }

    /// 0x10003690: whether the words around `u` may be split after it.
    fn splittable(&self, w: &[Word], u: usize) -> bool {
        if u + 1 >= w.len() {
            return false;
        }
        let c2 = w[u].f[CLASS];
        let c3 = if u > 0 { w[u - 1].f[CLASS] } else { 0 };
        let c4 = w[u + 1].f[CLASS];
        if c2 == 0x1f && c3 != c4 {
            return true;
        }
        let adj2 = in3(c2, b'*', b'+', b',');
        let noun2 = in3(c2, b'/', 0x15, b')');
        let noun4 = in3(c4, b'/', 0x15, b')');
        c3 != 0x17
            && c3 != 0x19
            && c3 != 0x1e
            && c2 != 0x19
            && c2 != 0x1e
            && !(c3 == 0xc9 && adj2)
            && !(matches!(c3, b' ' | b'"' | b'$' | b'(' | b'!') && adj2)
            && !(adj2 && w[u + 1].f[SUB] == 0x8d)
            && !(noun2 && in3(c4, b'*', b'+', b','))
            && !(noun2 && noun4)
            && !(in3(c3, b'*', b'+', b',') && c2 == b'3')
    }

    /// `LesConfiniProsodici` (0x10003d80); `mode` is the sentence type
    /// (FON ctx+8).
    pub(super) fn confini(&self, w: &mut [Word], mode: i32) {
        let kinds = self.kinds(w);
        let mut l: usize = 199;
        let mut u = 0usize;
        while u < 200 && kinds[u].0 != 10 {
            let split = |s: &Russian, w: &[Word], other: usize| !s.short_phrase(w, other, u) && s.splittable(w, u);
            let mut sub_rule = false;
            if w[u].f[BOUND] != 0x15 {
                sub_rule = matches!(w[u].f[BOUND], 3 | 5);
            } else {
                match kinds[u].0 {
                    0 | 2 => {
                        // (kind 0's own test sets 0x0a that the
                        // assignment below overwrites)
                        w[u].f[BOUND] = 5;
                        l = u;
                        sub_rule = true;
                    }
                    1 => {
                        if w[u].f[CLASS] == b'7' {
                            w[u].f[BOUND] = 0x16;
                        } else {
                            if l != 199 && matches!(w[l].f[BOUND], 3 | 5 | 7) && split(self, w, l) {
                                w[l].f[BOUND] = 0x0a;
                            }
                            w[u].f[BOUND] = 5;
                            l = u;
                            sub_rule = true;
                        }
                    }
                    4 => {
                        w[u].f[BOUND] = 9;
                        l = u;
                    }
                    6 => {
                        if l != 199 && split(self, w, l) {
                            w[l].f[BOUND] = 0x0a;
                        }
                        w[u].f[BOUND] = 1;
                        l = u;
                    }
                    7 => {
                        let c = w[u].f[CLASS];
                        if c == b'7' {
                            w[u].f[BOUND] = 0x16;
                        } else if matches!(c, 0x19 | 0x1e | 0x18 | 0x20 | 0x21 | 0x16 | 0x22 | 0x1f) {
                            w[u].f[BOUND] = 1;
                        } else {
                            w[u].f[BOUND] = 3;
                            sub_rule = true;
                        }
                    }
                    8 => {
                        let mut v = u.saturating_sub(1);
                        while v > 0 && w[v].f[BOUND] == 0x16 {
                            v -= 1;
                        }
                        if w[v].f[CLASS] == b'2' {
                            v = u;
                        }
                        let b = if w[u].f[SUB] == b'|' {
                            match mode {
                                0 => Some(0x0f),
                                2 => Some(0x11),
                                3 => Some(0x12),
                                5 => Some(0x10),
                                6 => Some(0x13),
                                _ => None,
                            }
                        } else {
                            let t = &w[u].text;
                            let mut k = 0;
                            if t.len() > 1 {
                                while k + 1 < t.len() && !self.s(PAUSE_PUNCT).contains(&t[k]) {
                                    k += 1;
                                }
                            }
                            let c = match t.get(k).copied().unwrap_or(0) {
                                b'.' => b',',
                                c => c,
                            };
                            match c {
                                b'(' | b')' => (u > 0 && w[u - 1].f[CLASS] != b'2' && w.get(u + 1).map_or(0, |n| n.f[CLASS]) != b'2').then_some(0x0b),
                                b',' | b'-' => Some(0x0b),
                                _ => None,
                            }
                        };
                        match b {
                            Some(b) => {
                                w[v].f[BOUND] = b;
                                w[u].f[BOUND] = b;
                                l = u;
                                sub_rule = matches!(w[u].f[BOUND], 3 | 5);
                            }
                            None => w[u].f[BOUND] = 0x16,
                        }
                    }
                    9 => w[u].f[BOUND] = 0x16,
                    _ => {
                        w[u].f[BOUND] = 3;
                        l = u;
                        sub_rule = true;
                    }
                }
            }
            if sub_rule {
                match w[u].f[SUB] {
                    b'y' | 0x84 => w[u].f[BOUND] = 7,
                    b'z' | 0x83 => w[u].f[BOUND] = 6,
                    b'{' => w[u].f[BOUND] = 8,
                    _ => {}
                }
            }
            u += 1;
        }
    }
}
