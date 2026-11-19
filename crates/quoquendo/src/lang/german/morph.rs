//! The module's morphological analyser (0x100164a0–0x10017780, reached
//! through the name table at 0x101274d0: initAnalyzer, analyzeWord,
//! getSolutionCount, getMorphemeCount, getMorphemeInfo…), which
//! LesElaboraParola runs on every word the dictionary doesn't have.
//!
//! A finite-state lexicon walked from the end of the word: states
//! (`STATES`, 0x14 bytes: +8 initial, +9 final, +0xc transition count,
//! +0x10 transitions), transitions (12 bytes: +4 → `{count, int* targets}`,
//! +8 → lexicon `{?, count, entries}`), lexicon entries (12 bytes, sorted
//! by reversed text: +0 text (`surface\0lexical`), +4 feature index, +8 u16
//! grouping, +0xa i8 length, +0xb i8 morpheme type). Every path from an
//! initial state to a final one that consumes the whole word is a solution;
//! the best 10 are kept, ordered by `compare` (0x10016820).

use super::super::common::Pe;
use super::Morpheme;

/// The analyser's context as initAnalyzer (0x100173f0 → 0x10017370) fills
/// it: the states and how many.
const STATES: u32 = 0x101293f0;
const STATE_COUNT: u32 = 0x116;
/// Morpheme type names (5: COMPOUND_CONNECTOR … STEM), by entry +0xb.
const TYPES: u32 = 0x1012d00c;
const TYPE_COUNT: i32 = 5;
/// Feature descriptors (8 bytes each), by entry +4.
const FEATURES: u32 = 0x1012d4b0;
const FEATURE_COUNT: i32 = 0x7f;
/// Grouping names (getGroupingDescr 0x10017450 → 0x10016790).
const GROUPINGS: u32 = 0x1012d8a8;
const GROUPING_COUNT: i32 = 0x26;
/// At most this many solutions and morphemes per solution.
const MAX: usize = 10;

/// One morpheme of a solution: (start, end, entry address).
type Morph = (i32, i32, u32);

/// analyzeWord's result (0x50c bytes): solution slots, their order.
#[derive(Default)]
struct Found {
    sols: Vec<Vec<Morph>>,
    order: Vec<usize>,
}

/// The analyser over the module's bytes.
pub(crate) struct Analyser<'a> {
    pe: &'a Pe,
}

/// C `strncmp` over `a` (a word, NUL past its end) and the module's string at `b`.
fn strncmp(pe: &Pe, a: &[u8], ai: i32, b: u32, n: u32) -> i32 {
    for k in 0..n {
        let x = byte(a, ai + k as i32) as i32;
        let y = pe.u8(b.wrapping_add(k)) as i32;
        if x != y {
            return x - y;
        }
        if x == 0 {
            return 0;
        }
    }
    0
}

fn byte(w: &[u8], i: i32) -> u8 {
    if i < 0 {
        0
    } else {
        w.get(i as usize).copied().unwrap_or(0)
    }
}

impl<'a> Analyser<'a> {
    pub(crate) fn new(pe: &'a Pe) -> Self {
        Analyser { pe }
    }

    fn i8(&self, va: u32) -> i8 {
        self.pe.u8(va) as i8
    }

    fn i32(&self, va: u32) -> i32 {
        self.pe.u32(va) as i32
    }

    fn state(&self, idx: u32) -> u32 {
        if idx < STATE_COUNT {
            STATES + idx * 0x14
        } else {
            0
        }
    }

    /// analyzeWord (0x10017490 → 0x10017220) and getMorphemeInfo
    /// (0x10017560) for every morpheme of every solution, in order.
    pub(crate) fn analyse(&self, word: &[u8]) -> Vec<Vec<Morpheme>> {
        let mut found = Found::default();
        let len = word.len() as u8 as i8;
        for i in 0..STATE_COUNT {
            let st = STATES + i * 0x14;
            if self.pe.u8(st + 8) != 0 {
                let mut sol = Vec::new();
                self.walk(st, word, len, &mut sol, &mut found);
            }
        }
        // 0x10016a50: each solution's morphemes from the word's start
        for s in &mut found.sols {
            s.reverse();
        }
        found.order.iter().map(|&k| found.sols[k].iter().map(|&m| self.info(m)).collect()).collect()
    }

    /// getMorphemeInfo (0x10017560): the 10 outputs 0x10009680 stores.
    fn info(&self, (start, end, e): Morph) -> Morpheme {
        let mut m = [0u32; 10];
        m[4] = GROUPING_COUNT as u32;
        m[8] = start as u32;
        m[9] = end as u32;
        if e == 0 {
            return m;
        }
        let s = self.pe.u32(e);
        m[0] = s;
        let t = self.i8(e + 0xb) as i32;
        m[1] = if (0..TYPE_COUNT).contains(&t) { self.pe.u32(TYPES + 4 * t as u32) } else { 0 };
        let f = self.i32(e + 4);
        m[2] = if (0..FEATURE_COUNT).contains(&f) { FEATURES + 8 * f as u32 } else { 0 };
        let len = self.i8(e + 0xa) as i32;
        if s != 0 && self.pe.u8(s) != 0 {
            let lex = s.wrapping_add(len as u32).wrapping_add(1);
            if self.pe.u8(lex) != 0 {
                m[3] = lex;
            }
        }
        let g = self.pe.u16(e + 8);
        m[4] = g as u32;
        let gs = g as i16 as i32;
        m[5] = if (0..GROUPING_COUNT).contains(&gs) { self.pe.u32(GROUPINGS + 4 * gs as u32) } else { 0 };
        m[6] = len as u32;
        if m[3] != 0 {
            m[7] = self.pe.cstr(m[3]).len() as u32;
        }
        m
    }

    /// 0x10016fb0: from state `st` with `len` characters of the word left.
    fn walk(&self, st: u32, word: &[u8], len: i8, sol: &mut Vec<Morph>, found: &mut Found) -> i32 {
        if self.pe.u8(st + 9) != 0 {
            // final: a solution when nothing is left (flag ctx+0x40 is 0)
            if len <= 0 {
                self.add(found, sol);
            }
            return -1;
        }
        if len < 0 {
            return len as i32;
        }
        if sol.len() >= MAX {
            return -1;
        }
        if found.order.len() >= MAX {
            let last = &found.sols[found.order[MAX - 1]];
            if self.compare(sol, last) >= 0 {
                return -1;
            }
        }
        let n = self.i32(st + 0xc);
        let trans = self.pe.u32(st + 0x10);
        for k in 0..n.max(0) as u32 {
            let t = trans + k * 12;
            let lex = self.pe.u32(t + 8);
            if lex == 0 || self.i32(lex + 4) <= 0 {
                self.targets(t, word, len, sol, found);
                continue;
            }
            let count = self.i32(lex + 4);
            let entries = self.pe.u32(lex + 8);
            let mut maxlen: i8 = 0;
            let mut i: i32 = 0;
            while i < count && maxlen <= len {
                let e = entries + 12 * i as u32;
                if self.try_entry(word, len, e, t, sol, found) {
                    let el = self.i8(e + 0xa);
                    if el > 0 {
                        maxlen = el;
                    }
                    i += 1;
                } else {
                    maxlen = maxlen.wrapping_add(1);
                    i = self.next_entry(word, len, maxlen, lex, i + 1);
                }
                if i < 0 {
                    break;
                }
            }
        }
        0
    }

    /// A transition's target states (0x10016fb0 at 0x100171a4, 0x10016ed0).
    fn targets(&self, t: u32, word: &[u8], len: i8, sol: &mut Vec<Morph>, found: &mut Found) {
        let list = self.pe.u32(t + 4);
        let n = self.i32(list);
        let arr = self.pe.u32(list + 4);
        for j in 0..n.max(0) as u32 {
            let st = self.state(self.pe.u32(arr + 4 * j));
            self.walk(st, word, len, sol, found);
        }
    }

    /// 0x10016ed0: entry `e` ends the word's first `len` characters: push it
    /// (unless it is an empty, untyped one) and go on to the targets.
    fn try_entry(&self, word: &[u8], len: i8, e: u32, t: u32, sol: &mut Vec<Morph>, found: &mut Found) -> bool {
        let start = self.match_entry(e, word, len);
        if start < 0 {
            return false;
        }
        let s = self.pe.u32(e);
        let push = (s != 0 && self.pe.u8(s) != 0) || self.i8(e + 0xb) >= 0;
        if push {
            sol.push((start as i32, len as i32, e));
        }
        self.targets(t, word, start, sol, found);
        if push {
            sol.pop();
        }
        true
    }

    /// 0x100164a0: where entry `e` starts if it ends the word's first `len`
    /// characters, else -1 (the fallback hook ctx+0x3c, 0x10017360, always
    /// fails).
    fn match_entry(&self, e: u32, word: &[u8], len: i8) -> i8 {
        let s = self.pe.u32(e);
        if s == 0 || self.pe.u8(s) == 0 {
            return len;
        }
        let el = self.i8(e + 0xa);
        let start = len.wrapping_sub(el);
        if start >= 0 && strncmp(self.pe, word, start as i32, s, el as i32 as u32) == 0 {
            return start;
        }
        -1
    }

    /// 0x10016d70: the next entry from `from` whose last `maxlen`
    /// characters end the word's first `len`: a binary search, then back to
    /// the first such entry (0x10016c90); -1 when none.
    fn next_entry(&self, word: &[u8], len: i8, maxlen: i8, lex: u32, from: i32) -> i32 {
        if maxlen > len {
            return -1;
        }
        let count = self.i32(lex + 4);
        let entries = self.pe.u32(lex + 8);
        if maxlen <= 0 && count > 0 {
            return if self.i8(entries + 0xa) <= 0 { 0 } else { -1 };
        }
        let (mut lo, mut hi, mut found) = (from, count - 1, -1i32);
        let mut matched: i8 = 0;
        while lo <= hi && found < 0 {
            let mid = (hi + lo) / 2;
            let e = entries + 12 * mid as u32;
            let mut pos = len as i32 - 1;
            let el = self.i8(e + 0xa);
            let c: i32 = if el <= 0 {
                byte(word, pos) as i32
            } else {
                let s = self.pe.u32(e);
                let mut sp = el as i32 - 1;
                matched = 0;
                loop {
                    if matched >= maxlen {
                        break 0;
                    }
                    if pos < 0 || sp < 0 || byte(word, pos) != self.pe.u8(s.wrapping_add(sp as u32)) {
                        break if pos < 0 {
                            -(self.pe.u8(s.wrapping_add(sp as u32)) as i32)
                        } else if sp < 0 {
                            byte(word, pos) as i32
                        } else {
                            byte(word, pos) as i32 - self.pe.u8(s.wrapping_add(sp as u32)) as i32
                        };
                    }
                    matched += 1;
                    pos -= 1;
                    sp -= 1;
                }
            };
            if c < 0 {
                hi = mid - 1;
            } else if c > 0 {
                lo = mid + 1;
            } else {
                found = mid;
            }
        }
        if matched < maxlen {
            return -1;
        }
        if found >= 0 {
            found = self.back(len, found, lex, word, matched, from);
        }
        found
    }

    /// 0x10016c90: from entry `idx` back (in shrinking steps) over the
    /// entries whose last `matched` characters are the word's.
    fn back(&self, len: i8, idx: i32, lex: u32, word: &[u8], matched: i8, from: i32) -> i32 {
        let m = matched as i32;
        let sfx = len as i32 - m;
        let entries = self.pe.u32(lex + 8);
        let mut step = (idx / 20) / (m * 30);
        if step <= 0 {
            step = 1;
        }
        let mut at = idx;
        while step >= 1 {
            if !(at > 0 && at >= from) {
                break;
            }
            while at > step - 1 {
                let e = entries.wrapping_add(12 * (at - step) as u32);
                let el = self.i8(e + 0xa);
                if el <= 0 || el < matched {
                    break;
                }
                let s = self.pe.u32(e).wrapping_add((el as i32 - m) as u32);
                if strncmp(self.pe, word, sfx, s, m as u32) != 0 {
                    break;
                }
                at -= step;
            }
            step /= 2;
        }
        at
    }

    /// 0x10016960: keep a finished solution among the best `MAX`.
    fn add(&self, found: &mut Found, sol: &[Morph]) {
        if sol.is_empty() {
            return;
        }
        let n = found.order.len();
        if n < MAX {
            found.sols.push(sol.to_vec());
            found.order.push(n);
        } else {
            let last = found.order[n - 1];
            if self.compare(sol, &found.sols[last]) >= 0 {
                return;
            }
            found.sols[last] = sol.to_vec();
        }
        self.sort(found);
    }

    /// 0x10016900: bubble sort of the order by `compare`.
    fn sort(&self, found: &mut Found) {
        loop {
            let mut swapped = false;
            for i in 0..found.order.len() - 1 {
                if self.compare(&found.sols[found.order[i]], &found.sols[found.order[i + 1]]) > 0 {
                    found.order.swap(i, i + 1);
                    swapped = true;
                }
            }
            if !swapped {
                break;
            }
        }
    }

    /// 0x10016820: fewer morphemes first; then the one whose word-final
    /// morpheme has text; then the longer first morpheme; then the lower
    /// grouping of the final one. Morphemes are still word-final first here.
    fn compare(&self, a: &[Morph], b: &[Morph]) -> i32 {
        let (na, nb) = (a.len(), b.len());
        if nb < na {
            return 1;
        }
        if nb > na {
            return -1;
        }
        if na == 0 {
            return 0;
        }
        let (ea, eb) = (a[0].2, b[0].2);
        let has = |e: u32| e != 0 && self.i8(e + 0xa) > 0;
        if !has(eb) && has(ea) {
            return 1;
        }
        if !has(ea) && has(eb) {
            return -1;
        }
        let la = self.i8(a[na - 1].2 + 0xa);
        let lb = self.i8(b[nb - 1].2 + 0xa);
        if la < lb {
            return 1;
        }
        if la > lb {
            return -1;
        }
        if ea != 0 && eb != 0 && (self.pe.u16(ea + 8) as i16) > (self.pe.u16(eb + 8) as i16) {
            1
        } else {
            -1
        }
    }
}
