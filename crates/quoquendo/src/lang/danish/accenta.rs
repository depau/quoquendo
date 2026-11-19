//! `LesAccenta` (0x100021c0) of LTTS7Danish.dll: the stress of each word's
//! current solution, and the marked respelling FonTrasParola reads
//! (0x100020d0 → 0x10001d30).
//!
//! A LexInfo keeps, from +0xc, one 0x20-byte entry per morpheme of the
//! current solution: two marks `A` (+0) and `B` (+0x10), each `{vowel,
//! kind, glottal, byte}`: the vowel counted from the morpheme's start (from
//! its end when negative, 0 none), the kind (1 `^` main stress, 2 `"`
//! secondary, 3 `^^`), and 1/2 for a stød (`?`) on that vowel or on the
//! consonant after it. The stress functions of the morphemes' groups
//! (getStressGroupingDetail 0x1001bc70) set them.

use super::elabora::{stricmp_eq, COMPOUND_CONNECTOR, DERIV_SUFFIX, FLEX_SUFFIX, GROUP_COUNT, GROUP_DETAILS, PREFIX, STEM};
use super::{Danish, LexInfo, Morpheme, CONSONANT, VOWEL};
use crate::lang::common::{field::*, Word};

/// getStressFunctionDescription (0x1001bc60): the stress functions' names
/// (0x100d9df0, `STRESSFUNC_FA0`…), compared with these (0x100010c0).
const FUNCTION_NAMES: u32 = 0x100d9df0;
const FA_CANC_AP_STEM: u32 = 0x10023278;
const FA_LIG: u32 = 0x10023264;
const FA_OR: u32 = 0x10023254;
const FA_SHIFT_SU_STEM_APB1: u32 = 0x10023230;
const FA0: u32 = 0x10023220;
const FA1: u32 = 0x10023210;
const FS_CANC: u32 = 0x100232e4;
const FS_CANC_SILLABA_ADIACENTE: u32 = 0x100232c0;
const FS_INS: u32 = 0x100232ac;
const FS_INS_POLI_ACC_B1: u32 = 0x10023290;
/// `or` (0x10023250): the ending FAor looks for.
const OR: u32 = 0x10023250;
/// `e` (0x100232f8): the connector after a one-syllable first stem.
const E: u32 = 0x100232f8;
/// `'’‘´` (0x1002333c), the characters an elided word starts with.
const ELISIONS: u32 = 0x1002333c;

/// The head dword of morpheme `j`'s mark (`b` false: A, true: B).
fn mark(j: i32, b: bool) -> usize {
    (3 + 8 * j + if b { 4 } else { 0 }) as usize
}

/// 0x10001970: a mark cleared (its three dwords and the low byte of the
/// fourth).
fn clear(h: &mut [i32; 0x53], k: usize) {
    if k + 3 < h.len() {
        h[k] = 0;
        h[k + 1] = 0;
        h[k + 2] = 0;
        h[k + 3] &= !0xff;
    }
}

/// A head dword, 0 outside the LexInfo (a morpheme index of -1).
fn get(h: &[i32; 0x53], k: i64) -> i32 {
    if (0..h.len() as i64).contains(&k) {
        h[k as usize]
    } else {
        0
    }
}

fn set(h: &mut [i32; 0x53], k: i64, v: i32) {
    if (0..h.len() as i64).contains(&k) {
        h[k as usize] = v;
    }
}

impl Danish {
    /// 0x100032d0: the index of the `n`-th vowel (from the end when
    /// negative), -1 when there is none.
    pub(crate) fn nth_vowel(&self, s: &[u8], n: i32) -> i32 {
        if n == 0 {
            return -1;
        }
        let mut seen = 0;
        if n > 0 {
            for (k, &c) in s.iter().enumerate() {
                if self.has(c, VOWEL) {
                    seen += 1;
                    if seen >= n {
                        return k as i32;
                    }
                }
            }
        } else {
            for k in (0..s.len()).rev() {
                if self.has(s[k], VOWEL) {
                    seen += 1;
                    if seen >= -n {
                        return k as i32;
                    }
                }
            }
        }
        -1
    }

    fn vowels(&self, s: &[u8]) -> i32 {
        s.iter().filter(|&&c| self.has(c, VOWEL)).count() as i32
    }

    /// LesAccenta 0x100021c0.
    pub(super) fn accenta(&self, w: &mut [Word]) {
        // the stressed vowel the last word without an analysis had
        // (0x10001030), kept across words
        let mut last: i32 = -1;
        for i in 0..w.len() {
            // every record has a LexInfo in the engine (LesAllocLanguageLexInfo
            // per slot): records the engine made itself (spelled letters,
            // symbols) have one without solutions
            let mut li = LexInfo::of(&w[i]).unwrap_or_default();
            clear(&mut li.head, 3);
            clear(&mut li.head, 7);
            if w[i].f[CLASS] == b'2' {
                li.store(&mut w[i]);
                continue;
            }
            if w[i].f[SUB] == 0x83 {
                let next = w.get(i + 1).map_or(0, |r| r.f[SUB]);
                if next != 0x84 || (i > 0 && w[i - 1].f[SUB] == 0x83) {
                    w[i].f[GRADE] = 3;
                }
            }
            if w[i].f[GRADE] == 0x0e {
                let cur = li.head[1] as usize;
                let count = li.solutions.get(cur).map_or(0, |s| s.len());
                if count == 0 {
                    let text = w[i].text.clone();
                    last = self.before_accent(&text);
                    if last >= 0 {
                        li.head[3] = last + 1;
                        li.head[4] = 1;
                    } else {
                        let elided = i >= 2
                            && (w[i - 1].text.is_empty() || self.s(ELISIONS).contains(&w[i - 1].text[0]))
                            && matches!(w[i - 2].f[SUB], b'y' | b'{');
                        if !(elided && self.nth_vowel(&text, 3) < 0) {
                            li.head[3] = 1;
                            li.head[4] = 1;
                        }
                    }
                } else {
                    for j in 0..count as i32 {
                        clear(&mut li.head, mark(j, false));
                        clear(&mut li.head, mark(j, true));
                    }
                    let sol = li.solutions[cur].clone();
                    for fs in [false, true] {
                        for t in [STEM, DERIV_SUFFIX, FLEX_SUFFIX, PREFIX] {
                            self.stress_functions(fs, t, &mut li, &sol, w, i);
                        }
                    }
                }
            } else if last >= 0 {
                li.head[4] = 1;
                li.head[3] = last;
            }
            li.store(&mut w[i]);
        }
        self.marked_respellings(w);
    }

    /// 0x10001030: the vowels before the first accented letter, -1 without
    /// one.
    fn before_accent(&self, s: &[u8]) -> i32 {
        let mut n = 0;
        for &c in s {
            if self.has(c, super::ACCENTED) {
                return n;
            }
            if self.has(c, VOWEL) {
                n += 1;
            }
        }
        -1
    }

    /// 0x10003290: a morpheme's group, getGroupingCount when `j` is not one.
    fn group_of(sol: &[Morpheme], j: i32) -> i16 {
        if j >= 0 && (j as usize) < sol.len() {
            sol[j as usize].group
        } else {
            GROUP_COUNT
        }
    }

    /// 0x10001730: for every morpheme of type `t`, its group's stress
    /// function: FA (0x100010f0) or, with `fs`, FS (0x10001380). The stem a
    /// suffix belongs to is the nearest stem before it, a prefix's the next
    /// one.
    fn stress_functions(&self, fs: bool, t: u32, li: &mut LexInfo, sol: &[Morpheme], w: &[Word], i: usize) {
        let ty = self.s(t);
        let text = &w[i].text;
        for j in 0..sol.len() as i32 {
            let kind = &sol[j as usize].kind;
            if !stricmp_eq(kind, ty) {
                continue;
            }
            let g1 = Self::group_of(sol, j);
            let k: i32 = if stricmp_eq(kind, self.s(DERIV_SUFFIX)) || stricmp_eq(kind, self.s(FLEX_SUFFIX)) {
                (0..=j).rev().find(|&x| stricmp_eq(&sol[x as usize].kind, self.s(STEM))).unwrap_or(-1)
            } else if stricmp_eq(kind, self.s(PREFIX)) {
                (j + 1..sol.len() as i32).find(|&x| stricmp_eq(&sol[x as usize].kind, self.s(STEM))).unwrap_or(-1)
            } else if stricmp_eq(kind, self.s(STEM)) {
                j
            } else {
                0
            };
            // the stem's letters in the word (strncat from its start)
            let stem: Vec<u8> = match sol.get(k.max(0) as usize).filter(|_| k >= 0) {
                Some(m) => {
                    let from = (m.start.max(0) as usize).min(text.len());
                    let n = (m.end - m.start).max(0) as usize;
                    text[from..].iter().take(n).copied().collect()
                }
                None => Vec::new(),
            };
            let vowels = self.vowels(&stem);
            if fs {
                self.fs(li, j, k, g1, vowels);
            } else {
                self.fa(li, j, k, g1, &stem, vowels, w, i);
            }
        }
    }

    /// The group's detail record (0x18 bytes) for `g`, clamped like
    /// 0x100010f0 / 0x10001380 do; `None` outside the groups.
    fn detail(&self, g: i16) -> Option<u32> {
        let g = if g >= GROUP_COUNT { GROUP_COUNT } else { g };
        (g >= 0 && g < GROUP_COUNT).then(|| GROUP_DETAILS + 0x18 * g as u32)
    }

    fn function_name(&self, f: i32) -> &[u8] {
        self.s(self.pe.u32(FUNCTION_NAMES + 4 * f as u32))
    }

    /// 0x100010f0: the group's marks A (+4 → `{vowel, …}`, kind 1) and B
    /// (+8, kind 2) on morpheme `j`, then its FA function (+0xc) on `j` or
    /// its stem `k`.
    #[allow(clippy::too_many_arguments)]
    fn fa(&self, li: &mut LexInfo, j: i32, k: i32, g1: i16, stem: &[u8], vowels: i32, w: &[Word], i: usize) {
        let Some(d) = self.detail(g1) else { return };
        let h = &mut li.head;
        let (ja, jb, ka) = (mark(j, false) as i64, mark(j, true) as i64, 3 + 8 * k as i64);
        let pa = self.pe.u32(d + 4);
        if pa != 0 && self.pe.u32(pa) != 0 {
            set(h, ja, self.pe.u32(pa) as i32);
            set(h, ja + 1, 1);
        }
        let pb = self.pe.u32(d + 8);
        if pb != 0 && self.pe.u32(pb) != 0 {
            set(h, jb, self.pe.u32(pb) as i32);
            set(h, jb + 1, 2);
        }
        let f = self.pe.u32(d + 0xc) as i32;
        if f < 0 {
            return;
        }
        let name = self.function_name(f);
        // 0x10001090: kind 1 becomes 3, anything else 0
        let promote = |h: &mut [i32; 0x53], at: i64| {
            let v = get(h, at + 1);
            set(h, at + 1, if v != 1 { 0 } else { 3 });
        };
        let next_class = w.get(i + 1).map(|r| r.f[CLASS]);
        // the next record's text pointer (+0x1c of this one's): set for
        // every record
        let next_text = w.get(i + 1).is_some();
        let before_last = (i as i64) < w.len() as i64 - 2;
        if name == self.s(FA_CANC_AP_STEM) {
        } else if name == self.s(FA_LIG) {
            if vowels <= 1 {
                promote(h, ja);
                return;
            }
            let v = get(h, ka);
            if v != -1 && v != vowels {
                set(h, jb, 1);
                set(h, jb + 1, 2);
                return;
            }
            promote(h, ka);
            set(h, ja, 1);
            set(h, ja + 1, 1);
        } else if name == self.s(FA_OR) {
            if stem.len() >= 2 && &stem[stem.len() - 2..] == self.s(OR) {
                set(h, ka, -1);
                set(h, ka + 1, 1);
            }
        } else if name == self.s(FA_SHIFT_SU_STEM_APB1) {
            set(h, ka, -1);
            set(h, ka + 1, 1);
        } else if name == self.s(FA0) {
            if before_last && next_text && next_class != Some(b'2') {
                set(h, ja, 0);
                set(h, ja + 1, 0);
            }
        } else if name == self.s(FA1) && before_last && next_class == Some(b'*') {
            set(h, ja, 0);
            set(h, ja + 1, 0);
        }
    }

    /// 0x10001380: the group's stød (`+8` of the marks' records) on
    /// morpheme `j`, then its FS function (+0x10) on the stem `k`.
    fn fs(&self, li: &mut LexInfo, j: i32, k: i32, g1: i16, vowels: i32) {
        let Some(d) = self.detail(g1) else { return };
        let h = &mut li.head;
        let (ja, jb) = (mark(j, false) as i64, mark(j, true) as i64);
        let (ka, kb) = (3 + 8 * k as i64, 7 + 8 * k as i64);
        let pa = self.pe.u32(d + 4);
        if pa != 0 && self.pe.u32(pa) != 0 && self.pe.u32(pa + 8) != 0 {
            set(h, ja + 2, self.pe.u32(pa + 8) as i32);
        }
        let pb = self.pe.u32(d + 8);
        if pb != 0 && self.pe.u32(pb) != 0 && self.pe.u32(pb + 8) != 0 {
            set(h, jb + 2, self.pe.u32(pb + 8) as i32);
        }
        let f = self.pe.u32(d + 0x10) as i32;
        if f < 0 {
            return;
        }
        let name = self.function_name(f);
        let at = |h: &[i32; 0x53], x: i64| get(h, x);
        if name == self.s(FS_CANC) {
            if at(h, ka + 2) != 0 {
                set(h, ka + 2, 0);
            }
            if at(h, kb + 2) != 0 {
                set(h, kb + 2, 0);
            }
        } else if name == self.s(FS_CANC_SILLABA_ADIACENTE) {
            let a = at(h, ka);
            if (a == vowels || a == -1) && at(h, ka + 2) != 0 {
                set(h, ka + 2, 0);
            }
            let b = at(h, kb);
            if (b == vowels || b == -1) && at(h, kb + 2) != 0 {
                set(h, kb + 2, 0);
            }
        } else if name == self.s(FS_INS) {
            if at(h, ka) != 0 && at(h, ka + 2) == 0 && at(h, kb + 2) == 0 {
                set(h, ka + 2, 1);
            }
        } else if name == self.s(FS_INS_POLI_ACC_B1) && vowels > 1 && at(h, ka + 2) == 0 && at(h, kb + 2) == 0 {
            let (a, b) = (at(h, ka), at(h, kb));
            if a == vowels || a == -1 {
                set(h, ka + 2, 1);
            } else if b == vowels || b == -1 {
                set(h, kb + 2, 1);
            }
        }
    }

    /// 0x100020d0: per word, the main stresses counted (0x10001570: only
    /// the first morpheme's stays main, the others' become `^^`); a word
    /// without one is unstressed (grade 3) unless it stands alone between
    /// punctuation (then its first vowel); then the marked respelling
    /// (0x10001d30).
    fn marked_respellings(&self, w: &mut [Word]) {
        for i in 0..w.len() {
            if w[i].f[CLASS] == b'2' {
                continue;
            }
            let mut li = LexInfo::of(&w[i]).unwrap_or_default();
            let cur = li.head[1] as usize;
            let sol = li.solutions.get(cur).cloned().unwrap_or_default();
            let mains = self.count_mains(&sol, &mut li);
            if mains < 1 {
                let next_punct = w.get(i + 1).is_some_and(|r| r.f[CLASS] == b'2');
                if next_punct && (i == 0 || w[i - 1].f[CLASS] == b'2') {
                    li.head[4] = 1;
                    li.head[3] = 1;
                } else {
                    w[i].f[GRADE] = 3;
                }
            }
            let text = w[i].text.clone();
            self.respell(&sol, &text, &mut li);
            li.store(&mut w[i]);
        }
    }

    /// 0x10001570.
    fn count_mains(&self, sol: &[Morpheme], li: &mut LexInfo) -> i32 {
        let h = &mut li.head;
        let (mut mains, mut stems, mut first) = (0, 0, 0usize);
        let mut found = false;
        for (j, m) in sol.iter().enumerate() {
            if m.kind == self.s(STEM) {
                stems += 1;
                if stems == 1 {
                    first = j;
                }
            }
            let (a, b) = (mark(j as i32, false), mark(j as i32, true));
            if get(h, a as i64 + 1) == 1 || get(h, b as i64 + 1) == 1 {
                if found {
                    set(h, a as i64 + 1, 3);
                } else {
                    mains += 1;
                    found = true;
                }
            }
        }
        if stems > 1 && self.nth_vowel(&sol[first].text, 2) < 0 {
            if let Some(m2) = sol.get(first + 1) {
                if m2.kind == self.s(COMPOUND_CONNECTOR) && stricmp_eq(&m2.text, self.s(E)) {
                    let (a, b) = (mark(first as i32, false) as i64, mark(first as i32, true) as i64);
                    if get(h, a + 2) != 0 {
                        set(h, a + 2, 0);
                    }
                    if get(h, b + 2) != 0 {
                        set(h, b + 2, 0);
                    }
                }
            }
        }
        mains
    }

    /// 0x10001bf0: `src` copied into `dst` (at most `max` bytes, letters
    /// from `at` below 0x40) with its marks moved into the letters' flags:
    /// `"` (0x4400) `^` (0x4100) `~` (0x5000) on the next letter, `:`
    /// (0x6000) `?` (0x4800) on the one before; `^` `"` `:` `?` only after a
    /// letter.
    fn parse_marks(li: &mut LexInfo, at: usize, src: &[u8], max: usize) -> Vec<u8> {
        let mut out = Vec::new();
        for &c in src {
            if out.len() + at >= 0x40 {
                break;
            }
            let next = at + out.len();
            let n = out.len();
            match c {
                b'"' if n > 0 => li.letters[next].flags |= 0x4400,
                b':' if n > 0 => li.letters[next - 1].flags |= 0x6000,
                b'?' if n > 0 => li.letters[next - 1].flags |= 0x4800,
                b'^' if n > 0 => li.letters[next].flags |= 0x4100,
                b'~' => li.letters[next].flags |= 0x5000,
                b'"' | b':' | b'?' | b'^' => {}
                _ => {
                    if n < max {
                        out.push(c);
                    }
                }
            }
        }
        out
    }

    /// 0x10001d30: the respelling of the current solution: each morpheme's
    /// respelling (marks parsed, its letters flagged 0x4000) or its
    /// spelling, while the whole stays below 64 letters; letters get the
    /// morpheme's index and type (1 stem, 2 inflection, 3 derivation, 4
    /// prefix, 5 connector), a compound part's first letter 0x10 (a `-`
    /// mark, 0x20, on the letter before), then its marks (0x10001ab0). A
    /// word without a solution is its own text, marks parsed.
    fn respell(&self, sol: &[Morpheme], text: &[u8], li: &mut LexInfo) {
        for l in li.letters.iter_mut() {
            l.morph = -1;
            l.flags = 0;
        }
        if sol.is_empty() {
            let s = Self::parse_marks(li, 0, text, 0x40);
            let n = s.len();
            for l in li.letters.iter_mut().take(n) {
                l.flags |= 1;
            }
            li.letters[0].flags |= 0x40;
            if n > 0 {
                li.letters[n - 1].flags |= 0x80;
            } else {
                // the bytes before the table: head +0x14a
                li.head[0x52] |= 0x80 << 16;
            }
            li.spelling = s.clone();
            self.mark_stress(li, &s, 0, 0);
            return;
        }
        let mut buf: Vec<u8> = Vec::new();
        let (mut cursor, mut start, mut prev) = (0usize, 0usize, 0u16);
        let mut glossed = false;
        for (j, m) in sol.iter().enumerate() {
            let s: Vec<u8> = match &m.gloss {
                Some(g) => {
                    glossed = true;
                    Self::parse_marks(li, cursor, g, 0x40)
                }
                None => {
                    glossed = false;
                    m.text.clone()
                }
            };
            if s.len() + buf.len() >= 0x40 {
                continue;
            }
            buf.extend_from_slice(&s);
            let t: u16 = if m.kind == self.s(STEM) {
                1
            } else if m.kind == self.s(FLEX_SUFFIX) {
                2
            } else if m.kind == self.s(DERIV_SUFFIX) {
                3
            } else if m.kind == self.s(PREFIX) {
                4
            } else if m.kind == self.s(COMPOUND_CONNECTOR) {
                5
            } else {
                0
            };
            if !s.is_empty() {
                start = cursor;
                for l in &mut li.letters[cursor..cursor + s.len()] {
                    l.flags |= t;
                    l.morph = j as i8;
                    if glossed {
                        l.flags |= 0x4000;
                    }
                }
                cursor += s.len();
                if t == 5 || (t == 1 && matches!(prev & 0xf, 5 | 4 | 1 | 3)) {
                    li.letters[start].flags |= 0x10;
                    if start > 0 {
                        li.letters[start - 1].flags |= 0x20;
                    }
                }
            }
            prev = t;
            self.mark_stress(li, &s, j as i32, start as i32);
        }
        li.letters[0].flags |= 0x40;
        if cursor > 0 {
            li.letters[cursor - 1].flags |= 0x80;
        }
        li.spelling = buf;
    }

    /// 0x10001ab0: morpheme `j`'s marks A and B on the letters of its
    /// respelling `s` (from letter `start`): the vowel, its kind (0x100
    /// `^`, 0x400 `"`, 0x200 `^^`), and a stød (0x800) on it or, for A,
    /// the next consonant (0x10001a50), for B the letter after.
    fn mark_stress(&self, li: &mut LexInfo, s: &[u8], j: i32, start: i32) {
        if s.is_empty() {
            return;
        }
        let kind_flag = |k: i32| match k {
            1 => 0x100,
            2 => 0x400,
            3 => 0x200,
            _ => 0,
        };
        let a = mark(j, false) as i64;
        let (av, ak, ag) = (get(&li.head, a), get(&li.head, a + 1), get(&li.head, a + 2));
        if av != 0 {
            let p = self.nth_vowel(s, av);
            let pos = p + start;
            if pos >= 0 {
                self.flag(li, pos, kind_flag(ak));
                match ag {
                    1 => self.flag(li, pos, 0x800),
                    2 => {
                        let off = self.consonant_after(s, p);
                        self.flag(li, pos + off, 0x800)
                    }
                    _ => {}
                }
            }
        }
        let b = mark(j, true) as i64;
        let (bv, bk, bg) = (get(&li.head, b), get(&li.head, b + 1), get(&li.head, b + 2));
        if bv == 0 {
            return;
        }
        let pos = self.nth_vowel(s, bv) + start;
        if pos < 0 {
            return;
        }
        self.flag(li, pos, kind_flag(bk));
        match bg {
            1 => self.flag(li, pos, 0x800),
            2 => self.flag(li, pos + 1, 0x800),
            _ => {}
        }
    }

    fn flag(&self, li: &mut LexInfo, pos: i32, f: u16) {
        if f != 0 && (0..64).contains(&pos) {
            li.letters[pos as usize].flags |= f;
        }
    }

    /// 0x10001a50: how far after letter `p` the first consonant is (1 when
    /// there is none).
    fn consonant_after(&self, s: &[u8], p: i32) -> i32 {
        let mut k = p;
        while k < s.len() as i32 {
            let c = if k < 0 { 0 } else { s[k as usize] };
            if self.has(c, CONSONANT) {
                return k - p;
            }
            k += 1;
        }
        1
    }
}

#[cfg(test)]
mod tests {
    use super::super::trace;
    use super::*;

    /// Every LesAccenta call: from its records (and LexInfos) before, ours
    /// gives the records, the marks (head dwords 3…) and the respelling
    /// after.
    #[test]
    fn danish_accenta_trace() {
        let (Some(calls), Some(m)) = (trace::calls(), trace::module()) else { return };
        let (mut ok, mut all, mut shown) = (0, 0, 0);
        for c in calls.iter().filter(|c| c.name == "LesAccenta") {
            let mut w: Vec<Word> = c.before.iter().map(|t| t.word()).collect();
            m.accenta(&mut w);
            for (k, a) in c.after.iter().enumerate() {
                if a.f[CLASS] == b'2' {
                    continue;
                }
                all += 1;
                let got = LexInfo::of(&w[k]);
                let want = a.lexinfo.as_ref();
                let same_rec = w[k].f[0xc..0x14] == a.f[0xc..0x14];
                let same_li = match (&got, want) {
                    (Some(g), Some(x)) => g.head[1..] == x.head[1..] && g.spelling == x.spelling && g.letters[..g.spelling.len() + 1] == x.letters[..x.spelling.len() + 1],
                    _ => got.is_none() && want.is_none(),
                };
                if same_rec && same_li {
                    ok += 1;
                } else if shown < 15 {
                    shown += 1;
                    let (g, x) = (got.unwrap_or_default(), want.cloned().unwrap_or_default());
                    eprintln!(
                        "{:?} rec {:02x?}/{:02x?}\n  head {:?}\n  want {:?}\n  sp {:?}/{:?}\n  letters {:x?}\n  want    {:x?}",
                        String::from_utf8_lossy(&a.text),
                        &w[k].f[0xc..0x14],
                        &a.f[0xc..0x14],
                        &g.head[1..12],
                        &x.head[1..12],
                        String::from_utf8_lossy(&g.spelling),
                        String::from_utf8_lossy(&x.spelling),
                        g.letters[..g.spelling.len() + 1].iter().map(|l| (l.morph, l.flags)).collect::<Vec<_>>(),
                        x.letters[..x.spelling.len() + 1].iter().map(|l| (l.morph, l.flags)).collect::<Vec<_>>()
                    );
                }
            }
        }
        eprintln!("LesAccenta: {ok}/{all}");
        assert_eq!(ok, all);
    }
}
