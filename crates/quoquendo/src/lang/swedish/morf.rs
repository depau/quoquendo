//! The module's morphological analyser (LesElaboraParola runs it on every
//! word, 0x10009770): a grammar of 0x1ba nodes at 0x100b9b58 walked from
//! the word's end to its start, each transition matching one morpheme of
//! its sorted list (stems, prefixes, derivational and inflectional
//! suffixes, compound connectors) against the letters before the current
//! position. The two best complete analyses are kept. The grammar
//! descriptor the loader builds (0x10009f40) names the tables used here.

use super::Swedish;

/// The grammar's nodes (descriptor +0/+4): 0x14 bytes each, +8 a start
/// node, +9 a final node, +0xc the transition count, +0x10 the
/// transitions (0xc bytes: +4 the successors `{count, int *nodes}`, +8 the
/// morpheme list `{cache index, count, entries}`).
pub(super) const NODES: u32 = 0x100b9b58;
pub(super) const NODE_COUNT: u32 = 0x1ba;
/// Category names (descriptor +8/+0xc, `FUN_10008c50`): COMPOUND_CONNECTOR,
/// DERIV_SUFFIX, FLEX_SUFFIX, PREFIX, STEM.
pub(super) const CATEGORIES: u32 = 0x1004a028;
pub(super) const CATEGORY_COUNT: u32 = 5;
/// Feature lists (descriptor +0x24/+0x28, `FUN_10008c70`): 8 bytes each,
/// `{(feature, value) byte pairs, count byte}`.
pub(super) const FEATURES: u32 = 0x1004a360;
pub(super) const FEATURE_COUNT: u32 = 0x53;
/// Stress group names (descriptor +0x2c/+0x30, `FUN_10008cd0`): GRP_….
pub(super) const GROUPS: u32 = 0x1004a5f8;
pub(super) const GROUP_COUNT: u32 = 0x23;
/// The letters a compound's dropped triple consonant cannot be
/// (`FUN_10008b00`): vowels.
const VOWELS: u32 = 0x100180ac;
/// At most this many morphemes in an analysis (0x10009500: `9 < count`).
const MAX_MORPHS: usize = 10;
/// Analyses kept (0x10008fe0).
const MAX_SOLUTIONS: usize = 2;

pub(super) struct Trans {
    succ: Vec<u32>,
    /// (entries, count) of the morpheme list, when it has one
    list: Option<(u32, i32)>,
}

pub(super) struct Node {
    start: bool,
    fin: bool,
    trans: Vec<Trans>,
}

/// The grammar's nodes, read from the module.
pub(super) fn read_nodes(sw_pe: &super::super::common::Pe) -> Vec<Node> {
    (0..NODE_COUNT)
        .map(|i| {
            let a = NODES + 0x14 * i;
            let n = sw_pe.u32(a + 0xc);
            let tp = sw_pe.u32(a + 0x10);
            let trans = (0..n)
                .map(|k| {
                    let t = tp + 0xc * k;
                    let s = sw_pe.u32(t + 4);
                    let sc = sw_pe.u32(s) as i32;
                    let sp = sw_pe.u32(s + 4);
                    let succ = (0..sc.max(0) as u32).map(|j| sw_pe.u32(sp + 4 * j)).collect();
                    let ml = sw_pe.u32(t + 8);
                    let list = (ml != 0).then(|| (sw_pe.u32(ml + 8), sw_pe.u32(ml + 4) as i32));
                    Trans { succ, list }
                })
                .collect();
            Node {
                start: sw_pe.u8(a + 8) != 0,
                fin: sw_pe.u8(a + 9) != 0,
                trans,
            }
        })
        .collect()
}

/// One morpheme of an analysis: the letters `start..end` of the word and
/// the grammar's entry (0x14 bytes at `m`: +0 its letters, +4 category,
/// +8 feature list, +0xc the spelling stress is placed on, +0x10 stress
/// group, +0x12 length).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    pub start: i32,
    pub end: i32,
    pub m: u32,
}

/// The analyses of a word, best first (the results buffer: count, two
/// 0x7c-byte solutions, their order at +0xfc).
pub(crate) type Analyses = Vec<Vec<Entry>>;

impl Swedish {
    pub(super) fn m_str(&self, m: u32) -> &[u8] {
        let p = self.pe.u32(m);
        if p == 0 {
            b""
        } else {
            self.pe.cstr(p)
        }
    }
    pub(super) fn m_has_str(&self, m: u32) -> bool {
        self.pe.u32(m) != 0
    }
    pub(super) fn m_cat(&self, m: u32) -> i32 {
        self.pe.u32(m + 4) as i32
    }
    pub(super) fn m_feat(&self, m: u32) -> i32 {
        self.pe.u32(m + 8) as i32
    }
    /// +0xc: the spelling the stress is counted on, when set
    pub(super) fn m_alt(&self, m: u32) -> Option<&[u8]> {
        let p = self.pe.u32(m + 0xc);
        (p != 0).then(|| self.pe.cstr(p))
    }
    pub(super) fn m_grp(&self, m: u32) -> i16 {
        self.pe.u16(m + 0x10) as i16
    }
    pub(super) fn m_len(&self, m: u32) -> i8 {
        self.pe.u8(m + 0x12) as i8
    }

    /// A category's name (`FUN_10008c50`), `None` out of range.
    pub(super) fn category(&self, i: i32) -> Option<&[u8]> {
        (i >= 0 && (i as u32) < CATEGORY_COUNT).then(|| self.pe.cstr(self.pe.u32(CATEGORIES + 4 * i as u32)))
    }
    /// A stress group's name (`FUN_10008cd0`).
    pub(super) fn group(&self, i: i16) -> Option<&[u8]> {
        (i >= 0 && (i as u32) < GROUP_COUNT).then(|| self.pe.cstr(self.pe.u32(GROUPS + 4 * i as u32)))
    }
    /// A feature list's value for `feature` (`FUN_10008c70` then
    /// `FUN_10008c90`): the first pair with that feature.
    pub(super) fn feature(&self, list: i32, feature: u8) -> Option<u8> {
        if list < 0 || list as u32 >= FEATURE_COUNT {
            return None;
        }
        let a = FEATURES + 8 * list as u32;
        let pairs = self.pe.u32(a);
        let n = self.pe.u8(a + 4) as u32;
        (0..n).map(|k| (self.pe.u8(pairs + 2 * k), self.pe.u8(pairs + 2 * k + 1))).find(|&(f, _)| f == feature).map(|(_, v)| v)
    }
    /// Whether the feature list exists (`FUN_10008c70` not null).
    pub(super) fn has_features(&self, list: i32) -> bool {
        list >= 0 && (list as u32) < FEATURE_COUNT
    }

    /// The analyses of a lower-case word (0x10009770).
    pub(crate) fn analyse_word(&self, word: &[u8]) -> Analyses {
        let mut res = Results::default();
        let len = word.len() as u8 as i8;
        for (i, n) in self.nodes.iter().enumerate() {
            if n.start {
                let mut sol = Vec::new();
                self.walk(i as u32, word, len, &mut sol, &mut res);
            }
        }
        res.order
            .iter()
            .map(|&s| {
                let mut e = res.slots[s].clone();
                e.reverse();
                e
            })
            .collect()
    }

    /// 0x10009500: from `node` with the letters before `pos` still to
    /// analyse.
    fn walk(&self, node: u32, word: &[u8], pos: i8, sol: &mut Vec<Entry>, res: &mut Results) {
        let Some(n) = self.nodes.get(node as usize) else { return };
        if n.fin {
            // the descriptor's +0x3c (keep an unanalysed rest) is 0
            if pos <= 0 {
                self.add(sol, res);
            }
            return;
        }
        if pos < 0 || sol.len() >= MAX_MORPHS {
            return;
        }
        if res.order.len() >= MAX_SOLUTIONS && self.compare(sol, &res.slots[*res.order.last().unwrap()]) >= 0 {
            return;
        }
        for t in &n.trans {
            match t.list {
                Some((entries, count)) if count >= 1 => {
                    let mut k: i8 = 0;
                    let mut i: i32 = 0;
                    loop {
                        if i >= count || k > pos {
                            break;
                        }
                        let m = entries + 0x14 * i as u32;
                        if self.step(word, pos, m, t, sol, res) {
                            let l = self.m_len(m);
                            if l > 0 {
                                k = l;
                            }
                            i += 1;
                        } else {
                            k = k.wrapping_add(1);
                            // the descriptor has no cache (+0x40 = 0):
                            // 0x10009030 never answers
                            i = self.search(word, pos, k, entries, count, i + 1);
                        }
                        if i < 0 {
                            break;
                        }
                    }
                }
                _ => {
                    for &s in &t.succ {
                        self.walk(s, word, pos, sol, res);
                    }
                }
            }
        }
    }

    /// 0x10009440: morpheme `m` ending at `pos`; when it matches, it joins
    /// the analysis (unless it is an empty one without category) and the
    /// transition's successors go on from its start.
    fn step(&self, word: &[u8], pos: i8, m: u32, t: &Trans, sol: &mut Vec<Entry>, res: &mut Results) -> bool {
        let np = self.match_end(word, pos, m);
        if np < 0 {
            return false;
        }
        // FUN_10009010
        let push = !(self.m_str(m).is_empty() && self.m_cat(m) < 0);
        if push {
            sol.push(Entry {
                start: np as i32,
                end: pos as i32,
                m,
            });
        }
        for &s in &t.succ {
            self.walk(s, word, np, sol, res);
        }
        if push {
            sol.pop();
        }
        true
    }

    /// 0x10008ba0: where morpheme `m` starts when it ends at `pos`, -1 when
    /// it does not; a compound may have dropped the third of three equal
    /// consonants (0x10008b20).
    fn match_end(&self, word: &[u8], pos: i8, m: u32) -> i8 {
        let s = self.m_str(m);
        if !self.m_has_str(m) || s.is_empty() {
            return pos;
        }
        let len = self.m_len(m);
        let start = pos.wrapping_sub(len);
        if start >= 0 && strncmp(&word[start as usize..], s, len as usize) {
            return start;
        }
        if pos > 0 && start >= 0 {
            if let Some(k) = self.triple(s, len, &word[pos as usize..]) {
                if k > 0 {
                    let st = start.wrapping_add(k);
                    if st >= 0 && strncmp(&word[st as usize..], s, len.wrapping_sub(k) as u8 as usize) {
                        return st;
                    }
                }
            }
        }
        -1
    }

    /// 0x10008b20: the morpheme's letters before `next`, the rest of the
    /// word, repeat a consonant three times when joined: how many of the
    /// morpheme's last letters to drop.
    fn triple(&self, s: &[u8], len: i8, next: &[u8]) -> Option<i8> {
        if len < 2 {
            return None;
        }
        let n0 = at(next, 0);
        if n0 == 0 || self.pe.cstr(VOWELS).contains(&n0) {
            return None;
        }
        let c = at(s, len as usize - 1);
        if n0 != c {
            return None;
        }
        let n1 = at(next, 1);
        if n1 != 0 && n1 == n0 {
            return Some(1);
        }
        if c != at(s, len as usize - 2) {
            return None;
        }
        Some(1)
    }

    /// 0x100092e0: the next entry from `from` on (binary search over the
    /// list, sorted by reversed spelling) whose last `k` letters are the
    /// `k` letters before `pos`, or -1.
    fn search(&self, word: &[u8], pos: i8, k: i8, entries: u32, count: i32, from: i32) -> i32 {
        if k > pos {
            return -1;
        }
        if k <= 0 && count > 0 {
            return if self.m_len(entries) <= 0 { 0 } else { -1 };
        }
        let mut hi = count - 1;
        let mut lo = from;
        let mut found = -1;
        let mut j: i8 = 0;
        while lo <= hi && found < 0 {
            let mid = (lo + hi) / 2;
            let e = entries + 0x14 * mid as u32;
            let mut p = pos as i32 - 1;
            let el = self.m_len(e);
            let cmp: i32;
            if el <= 0 {
                cmp = word_at(word, p) as i32;
            } else {
                let s = self.m_str(e);
                let mut q = el as i32 - 1;
                j = 0;
                let mut c = 0i32;
                let mut differ = false;
                while j < k {
                    if p < 0 || q < 0 || word_at(word, p) != at(s, q as usize) {
                        differ = true;
                        c = if p < 0 {
                            -(at(s, q as usize) as i32)
                        } else if q < 0 {
                            word_at(word, p) as i32
                        } else {
                            word_at(word, p) as i32 - at(s, q as usize) as i32
                        };
                        break;
                    }
                    j += 1;
                    p -= 1;
                    q -= 1;
                }
                if !differ {
                    found = mid;
                    continue;
                }
                cmp = c;
            }
            if cmp < 0 {
                hi = mid - 1;
            } else if cmp > 0 {
                lo = mid + 1;
            } else {
                found = mid;
            }
        }
        if j < k {
            return -1;
        }
        if found >= 0 {
            found = self.gallop(found, entries, word, pos, j, from);
        }
        found
    }

    /// 0x10009200: back from `found` to the first entry with the same last
    /// `k` letters, in steps halving from `(found / 20) / (30 k)`.
    fn gallop(&self, mut found: i32, entries: u32, word: &[u8], pos: i8, k: i8, from: i32) -> i32 {
        let key = (pos as i32 - k as i32).max(0) as usize;
        let kk = k as i32;
        let mut step = if kk * 30 != 0 { (found / 20) / (kk * 30) } else { 0 };
        if step <= 0 {
            step = 1;
        }
        while step >= 1 {
            if found <= 0 || found < from {
                return found;
            }
            while found > step - 1 {
                let e = entries + 0x14 * (found - step) as u32;
                let el = self.m_len(e);
                if el <= 0 || el < k {
                    break;
                }
                let s = self.m_str(e);
                let off = (el as i32 - kk) as usize;
                if !strncmp(&word[key.min(word.len())..], &s[off.min(s.len())..], k as usize) {
                    break;
                }
                found -= step;
            }
            step /= 2;
        }
        found
    }

    /// 0x10008eb0: a complete analysis, kept when it is acceptable
    /// (0x1000a6b0) and better than the worst of two kept.
    fn add(&self, sol: &[Entry], res: &mut Results) {
        if sol.is_empty() || !self.accept(sol) {
            return;
        }
        if res.order.len() < MAX_SOLUTIONS {
            res.slots.push(sol.to_vec());
            res.order.push(res.slots.len() - 1);
            self.sort(res);
        } else {
            let w = *res.order.last().unwrap();
            if self.compare(sol, &res.slots[w]) < 0 {
                res.slots[w] = sol.to_vec();
                self.sort(res);
            }
        }
    }

    /// 0x1000a6b0: an unknown first part (category < 0) may not be longer
    /// than the rest of the word.
    fn accept(&self, sol: &[Entry]) -> bool {
        let last = sol[sol.len() - 1];
        let d = if self.m_cat(last.m) < 0 { last.end - last.start } else { 0 };
        d <= sol[0].end - d
    }

    /// 0x10008e50: bubble sort of the kept analyses.
    fn sort(&self, res: &mut Results) {
        loop {
            if res.order.len() == 1 {
                return;
            }
            let mut swapped = false;
            for i in 0..res.order.len() - 1 {
                if self.compare(&res.slots[res.order[i]], &res.slots[res.order[i + 1]]) > 0 {
                    res.order.swap(i, i + 1);
                    swapped = true;
                }
            }
            if !swapped {
                return;
            }
        }
    }

    /// 0x10008da0 (analyses still in matching order, the word's end
    /// first): fewer morphemes first; then one ending in an empty
    /// morpheme; then the longer first morpheme.
    fn compare(&self, a: &[Entry], b: &[Entry]) -> i32 {
        if b.len() < a.len() {
            return 1;
        }
        if b.len() > a.len() {
            return -1;
        }
        if a.is_empty() {
            return 0;
        }
        let b0 = self.m_len(b[0].m) > 0;
        let a0 = self.m_len(a[0].m) > 0;
        if !b0 && a0 {
            return 1;
        }
        if !a0 && b0 {
            return -1;
        }
        if self.m_len(b[b.len() - 1].m) > self.m_len(a[a.len() - 1].m) {
            1
        } else {
            -1
        }
    }
}

#[derive(Default)]
struct Results {
    slots: Vec<Vec<Entry>>,
    order: Vec<usize>,
}

fn at(s: &[u8], i: usize) -> u8 {
    s.get(i).copied().unwrap_or(0)
}

fn word_at(w: &[u8], p: i32) -> u8 {
    if p < 0 {
        0
    } else {
        at(w, p as usize)
    }
}

/// C `strncmp(a, b, n) == 0` on NUL-less byte strings.
fn strncmp(a: &[u8], b: &[u8], n: usize) -> bool {
    for i in 0..n {
        let (x, y) = (at(a, i), at(b, i));
        if x != y {
            return false;
        }
        if x == 0 {
            return true;
        }
    }
    true
}
