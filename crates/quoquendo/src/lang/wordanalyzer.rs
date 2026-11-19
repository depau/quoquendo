//! The morphological analyser some language modules link statically
//! ("WordAnalyzer"; LTTS7Russian.dll 0x1000ff80–0x10010d40, run by
//! LesElaboraParola through 0x10010cd0; LTTS7Danish.dll analyzeWord
//! 0x1001b9f0): a graph of nodes (Russian 0x4e1 at 0x100975d8, `{id, name,
//! start +8, final +9, transition count +0xc, transitions +0x10}`, 0x14
//! bytes) that strips morphs off the end of the word from the start nodes
//! (Russian `MAIN_END`) until a final node (`MAIN_START`) is reached with
//! nothing left. A transition (12 bytes) has
//! the next nodes at +4 (`{count, ids*}`) and a morph list at +8 (`{?,
//! count +4, entries +8}`), sorted by the morphs' reversed spelling. A
//! morph (12 bytes) is `{spelling*, id +4, class +8 (i16), length +0xa
//! (i8), flag +0xb (i8)}`; its spelling string goes on past the length with
//! a second string (`gloss`).
//!
//! At most 10 analyses are kept (0x100102d0), ranked by 0x10010190; each is
//! a list of at most 10 segments, reversed at the end (0x100103c0) so the
//! root comes first.

use super::common::Pe;

/// The read-only analyser object the module's init builds (Russian
/// 0x100094a0): its node table, and the acceptance test it installs at
/// +0x34 (`None`: the default the analyser installs when it is 0, a
/// non-empty path: Russian 0x10010250, Danish 0x1001b0b0). The other hooks
/// are the defaults: ranking +0x38 (Russian 0x10010190, Danish 0x1001aff0),
/// no +0x3c callback (Danish's 0x1001bb30 returns 0), +0x40 0 (no rest
/// segment), no cache (+0x44 0).
pub struct Analyzer {
    pub nodes: u32,
    pub count: u32,
    pub accept: Option<fn(&Pe, &[Seg]) -> bool>,
}

/// At most this many analyses, and segments in one.
const MAX: usize = 10;

/// One stripped morph: the remaining length after it (`start`), before it
/// (`end`), and the morph record's address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seg {
    pub start: i32,
    pub end: i32,
    pub morph: u32,
}

/// The analyses of a word (the 0x50c-byte result object: count +4, slots
/// of 0x7c bytes from +8, their ranking at +0x4e0).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Analyses {
    pub slots: Vec<Vec<Seg>>,
    /// slot indices, best first
    pub order: Vec<usize>,
}

impl Analyses {
    /// 0x100100f0: the analysis ranked `k`.
    pub fn ranked(&self, k: usize) -> Option<&[Seg]> {
        self.order.get(k).map(|&s| self.slots[s].as_slice())
    }
}

/// A morph record's fields.
pub struct Morph<'a> {
    pe: &'a Pe,
    pub va: u32,
}

impl Morph<'_> {
    pub fn str_va(&self) -> u32 {
        self.pe.u32(self.va)
    }
    /// the spelling (its first `len` bytes; empty without one)
    pub fn spelling(&self) -> &[u8] {
        match self.str_va() {
            0 => &[],
            s => self.pe.cstr(s),
        }
    }
    pub fn id(&self) -> i32 {
        self.pe.u32(self.va + 4) as i32
    }
    pub fn class(&self) -> i16 {
        self.pe.u16(self.va + 8) as i16
    }
    pub fn len(&self) -> i8 {
        self.pe.u8(self.va + 0xa) as i8
    }
    pub fn flag(&self) -> i8 {
        self.pe.u8(self.va + 0xb) as i8
    }
    /// The second string after the spelling (`str + len + 1`; Russian:
    /// the morph's stressed form), when the spelling is not empty and it
    /// is not (0x10002f60).
    pub fn gloss(&self) -> Option<&[u8]> {
        let s = self.str_va();
        if s == 0 || self.pe.u8(s) == 0 {
            return None;
        }
        let at = (s as i64 + self.len() as i64 + 1) as u32;
        if self.pe.u8(at) == 0 {
            return None;
        }
        Some(self.pe.cstr(at))
    }
}

pub fn morph(pe: &Pe, va: u32) -> Morph<'_> {
    Morph { pe, va }
}

struct Run<'a> {
    pe: &'a Pe,
    a: &'a Analyzer,
    word: &'a [u8],
    path: Vec<Seg>,
    out: Analyses,
}

impl Run<'_> {
    fn node(&self, id: u32) -> u32 {
        self.a.nodes + 0x14 * id
    }

    /// strncmp(word + at, s, n) == 0, `word` being NUL-terminated
    fn same(&self, at: i32, s: u32, n: i32) -> bool {
        for k in 0..n {
            let a = self.word.get((at + k) as usize).copied().unwrap_or(0);
            let b = self.pe.u8(s + k as u32);
            if a != b {
                return false;
            }
            if a == 0 {
                return true;
            }
        }
        true
    }

    /// 0x10010190: the ranking of two analyses (1: `a` after `b`).
    fn cmp(&self, a: &[Seg], b: &[Seg]) -> i32 {
        if b.len() < a.len() {
            return 1;
        }
        if b.len() > a.len() {
            return -1;
        }
        if a.is_empty() {
            return 0;
        }
        let len = |m: u32| self.pe.u8(m + 0xa) as i8;
        let (ebx, ebp) = (b[0].morph, a[0].morph);
        if !(ebx != 0 && len(ebx) > 0) && ebp != 0 && len(ebp) > 0 {
            return 1;
        }
        if !(ebp != 0 && len(ebp) > 0) && ebx != 0 && len(ebx) > 0 {
            return -1;
        }
        let (al, cl) = (len(b[b.len() - 1].morph), len(a[a.len() - 1].morph));
        if cl < al {
            return 1;
        }
        if cl > al || ebp == 0 || ebx == 0 {
            return -1;
        }
        if self.pe.u16(ebp + 8) as i16 > self.pe.u16(ebx + 8) as i16 {
            1
        } else {
            -1
        }
    }

    /// The acceptance test (+0x34) on the current path.
    fn accept(&self) -> bool {
        match self.a.accept {
            Some(f) => f(self.pe, &self.path),
            None => !self.path.is_empty(),
        }
    }

    /// 0x10010270: bubble sort of the ranking.
    fn sort(&mut self) {
        let n = self.out.order.len();
        if n == 1 {
            return;
        }
        loop {
            let mut swapped = false;
            for k in 0..n - 1 {
                let (x, y) = (self.out.order[k], self.out.order[k + 1]);
                if self.cmp(&self.out.slots[x], &self.out.slots[y]) > 0 {
                    self.out.order.swap(k, k + 1);
                    swapped = true;
                }
            }
            if !swapped {
                break;
            }
        }
    }

    /// 0x100102d0: keep the current path.
    fn store(&mut self) {
        if self.path.is_empty() || !self.accept() {
            return;
        }
        let n = self.out.slots.len();
        if n < MAX {
            self.out.slots.push(self.path.clone());
            self.out.order.push(n);
            self.sort();
        } else {
            let worst = self.out.order[n - 1];
            if self.cmp(&self.path, &self.out.slots[worst]) < 0 {
                self.out.slots[worst] = self.path.clone();
                self.sort();
            }
        }
    }

    /// 0x1000ff80: the morph at the end of the first `rem` letters: the
    /// letters left, or -1.
    fn strip(&self, m: u32, rem: i8) -> i8 {
        let s = self.pe.u32(m);
        if s == 0 || self.pe.u8(s) == 0 {
            return rem;
        }
        let len = self.pe.u8(m + 0xa) as i8;
        let bl = rem.wrapping_sub(len);
        if bl >= 0 && self.same(bl as i32, s, len as i32) {
            return bl;
        }
        -1
    }

    /// 0x10010840: try one morph of a transition.
    fn try_morph(&mut self, rem: i8, m: u32, t: u32) -> bool {
        let r = self.strip(m, rem);
        if r < 0 {
            return false;
        }
        let s = self.pe.u32(m);
        let pushed = (s != 0 && self.pe.u8(s) != 0) || (self.pe.u8(m + 0xb) as i8) >= 0;
        if pushed {
            self.path.push(Seg {
                start: r as i32,
                end: rem as i32,
                morph: m,
            });
        }
        self.next_nodes(t, r);
        if pushed {
            self.path.pop();
        }
        true
    }

    fn next_nodes(&mut self, t: u32, rem: i8) {
        let list = self.pe.u32(t + 4);
        let n = self.pe.u32(list) as i32;
        let ids = self.pe.u32(list + 4);
        for k in 0..n.max(0) as u32 {
            let id = self.pe.u32(ids + 4 * k);
            if id < self.a.count {
                self.walk(self.node(id), rem);
            }
        }
    }

    /// 0x100106e0: the first morph from `from` on whose spelling ends like
    /// the `c` letters before `rem`, or -1 (binary search, then
    /// 0x10010600 back to the first of equals).
    fn find(&self, rem: i8, c: i8, ml: u32, from: i32) -> i32 {
        if c > rem {
            return -1;
        }
        let count = self.pe.u32(ml + 4) as i32;
        let ent = self.pe.u32(ml + 8);
        let e = |i: i32| ent + 12 * i as u32;
        if c <= 0 && count > 0 {
            return if (self.pe.u8(e(0) + 0xa) as i8) <= 0 { 0 } else { -1 };
        }
        let w = |i: i32| if i < 0 { 0 } else { self.word.get(i as usize).copied().unwrap_or(0) };
        let (mut lo, mut hi, mut found) = (from, count - 1, -1i32);
        let mut bl: i8 = 0;
        while lo <= hi && found < 0 {
            let mid = (hi + lo) / 2;
            let m = e(mid);
            let mut si = rem as i32 - 1;
            let len = self.pe.u8(m + 0xa) as i8;
            let diff: i32;
            if len <= 0 {
                diff = w(si) as i32;
            } else {
                let base = self.pe.u32(m) as i64;
                let mut di = base + len as i64 - 1;
                bl = 0;
                let mut mismatch = false;
                while bl < c {
                    if si < 0 || di < base || w(si) != self.pe.u8(di as u32) {
                        mismatch = true;
                        break;
                    }
                    bl += 1;
                    si -= 1;
                    di -= 1;
                }
                if !mismatch || bl >= c {
                    diff = 0;
                } else if si < 0 {
                    diff = -(self.pe.u8(di as u32) as i32);
                } else if di < base {
                    diff = w(si) as i32;
                } else {
                    diff = w(si) as i32 - self.pe.u8(di as u32) as i32;
                }
            }
            if diff < 0 {
                hi = mid - 1;
            } else if diff > 0 {
                lo = mid + 1;
            } else {
                found = mid;
            }
        }
        if bl < c {
            return -1;
        }
        if found < 0 {
            return found;
        }
        // 0x10010600
        let n = bl as i32;
        let key = rem as i32 - n;
        let mut idx = found;
        let mut step = (idx / 20) / (n * 30);
        if step <= 0 {
            step = 1;
        }
        while step >= 1 {
            if idx <= 0 || idx < from {
                return idx;
            }
            while idx > step - 1 {
                let m = e(idx - step);
                let len = self.pe.u8(m + 0xa) as i8 as i32;
                if len <= 0 || len < n {
                    break;
                }
                let s = self.pe.u32(m) + (len - n) as u32;
                if !self.same(key, s, n) {
                    break;
                }
                idx -= step;
            }
            step /= 2;
        }
        idx
    }

    /// 0x10010920: from node `n` with `rem` letters left.
    fn walk(&mut self, n: u32, rem: i8) -> i32 {
        if self.pe.u8(n + 9) != 0 {
            // a final node: complete when nothing is left (the dummy
            // segment for a rest needs +0x40, which is 0)
            if rem <= 0 {
                self.store();
            }
            return -1;
        }
        if rem < 0 {
            return rem as i32;
        }
        if self.path.len() >= MAX {
            return -1;
        }
        if self.out.slots.len() >= MAX {
            let worst = self.out.order[self.out.slots.len() - 1];
            if self.cmp(&self.path, &self.out.slots[worst]) >= 0 {
                return -1;
            }
        }
        let tc = self.pe.u32(n + 0xc) as i32;
        let tp = self.pe.u32(n + 0x10);
        for k in 0..tc.max(0) as u32 {
            let t = tp + 12 * k;
            let ml = self.pe.u32(t + 8);
            if ml == 0 || (self.pe.u32(ml + 4) as i32) < 1 {
                self.next_nodes(t, rem);
                continue;
            }
            let mut c: i8 = 0;
            let mut i: i32 = 0;
            loop {
                if i >= self.pe.u32(ml + 4) as i32 || c > rem {
                    break;
                }
                let m = self.pe.u32(ml + 8) + 12 * i as u32;
                if self.try_morph(rem, m, t) {
                    let len = self.pe.u8(m + 0xa) as i8;
                    if len > 0 {
                        c = len;
                    }
                    i += 1;
                } else {
                    c = c.wrapping_add(1);
                    i = self.find(rem, c, ml, i + 1);
                }
                if i < 0 {
                    break;
                }
            }
        }
        0
    }
}

impl Analyzer {
    /// 0x10010b90: every analysis of `word` (the engine's 8-bit text).
    pub fn analyse(&self, pe: &Pe, word: &[u8]) -> Analyses {
        let mut r = Run {
            pe,
            a: self,
            word,
            path: Vec::new(),
            out: Analyses::default(),
        };
        let len = word.len() as i32 as i8;
        for id in 0..self.count {
            let n = self.nodes + 0x14 * id;
            if pe.u8(n + 8) != 0 {
                r.path.clear();
                r.walk(n, len);
            }
        }
        for s in r.out.slots.iter_mut() {
            s.reverse();
        }
        r.out
    }
}
