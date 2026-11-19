//! FonTrasParola (0x1000ee40) and its letter-to-sound function 0x1000a960:
//! the word's spelling, rebuilt from its analysis (each morpheme's stress
//! spelling, +0xc, when it has one), letter by letter into phone codes;
//! the accented vowels (LesAccenta's slots) take accent 1 (`0x1xxxx`) or 2
//! (`0x3xxxx`), the morpheme boundaries (`B`) steer vowel length and
//! consonant clusters.

use super::{LexInfo, Swedish};
use crate::lang::common::{field::*, Word};

/// The letters 0x1000a720 counts as vowels, 0x1000a770 as consonants,
/// 0x1000a7c0 as front vowels.
const VOWELS: u32 = 0x10047458;
const CONSONANTS: u32 = 0x10047464;
const FRONT: u32 = 0x10047478;
/// 0x1000efbd…: category names compared (0x100462c0 COMPOUND_CONNECTOR,
/// 0x10016500 STEM, 0x100163a0 DERIV_SUFFIX, 0x10016274 PREFIX, 0x1001630c
/// FLEX_SUFFIX).
const CONNECTOR: u32 = 0x100462c0;
const STEM: u32 = 0x10016500;
const DERIV: u32 = 0x100163a0;
const PREFIX: u32 = 0x10016274;
const FLEX: u32 = 0x1001630c;
/// Words whose unstressed `a` reads `Aa` (0x1001afd0 "jag", 0x10028e3c
/// "av").
const JAG: u32 = 0x1001afd0;
const AV: u32 = 0x10028e3c;

impl Swedish {
    /// FonTrasParola (0x1000ee40): the morpheme boundaries of the word's
    /// best analysis (2 after a stem, derivational suffix, prefix or
    /// connector before a stem/prefix/suffix, 1 before an inflection), an
    /// unstressed word's accents cleared, then 0x1000a960.
    pub(super) fn tras_parola(&self, w: &[Word], i: usize) -> Vec<u32> {
        let r = &w[i];
        let mut li = LexInfo::of(r);
        let mut flags = [-1i32; 10];
        let sol = li.best().map(|s| s.to_vec());
        match &sol {
            None => {
                if r.f[GRADE] == 3 {
                    li.slots[0][1] = 0;
                }
            }
            Some(sol) => {
                let mut prev: &[u8] = b"";
                let mut has_prev = false;
                for k in 0..sol.len() {
                    if r.f[GRADE] == 3 {
                        li.slots[k][1] = 0;
                    }
                    let cat = self.category(self.m_cat(sol[k].m)).unwrap_or(b"");
                    if k > 0 && k < 10 {
                        let is = |a: &[u8], b: u32| has_str(a) && a.eq_ignore_ascii_case(self.s(b));
                        let p = if has_prev { prev } else { b"" };
                        if is(cat, CONNECTOR) {
                            flags[k] = 2;
                            flags[k - 1] = 2;
                        } else {
                            // 0x1000efbd…0x1000f087
                            let mut two = false;
                            let mut one = false;
                            if (is(p, STEM) || is(p, DERIV)) && (is(cat, STEM) || is(cat, PREFIX)) {
                                two = true;
                            } else if is(p, PREFIX) && is(cat, STEM) {
                                two = true;
                            } else if is(p, STEM) && is(cat, DERIV) {
                                two = true;
                            } else if is(cat, FLEX) && is(p, STEM) {
                                one = true;
                            }
                            if two {
                                flags[k - 1] = 2;
                            } else if one {
                                flags[k - 1] = 1;
                            }
                        }
                    }
                    prev = cat;
                    has_prev = true;
                }
            }
        }
        let mut l = L2s::new(self, &r.text, r.f[CLASS]);
        l.run(sol.as_deref(), &flags, &li);
        l.out.truncate(l.n);
        l.out
    }
}

fn has_str(a: &[u8]) -> bool {
    !a.is_empty()
}

/// The state of 0x1000a960.
struct L2s<'a> {
    sw: &'a Swedish,
    /// the original word (`local_24c`)
    word: Vec<u8>,
    /// the spelling worked on (`acStack_45 + 1`), NUL padded
    w: Vec<u8>,
    len: usize,
    /// `local_248`: [0] the vowel counter, [p] the boundary before letter p
    b: Vec<i32>,
    /// `aiStack_148`: the accent mark of each letter
    acc: Vec<i32>,
    mono: bool,
    cls: u8,
    out: Vec<u32>,
    n: usize,
}

/// What a letter's case does after writing its code: count it (`n++`,
/// LAB_1000e24b) or not.
enum Then {
    Inc,
    Skip,
}
use Then::*;

impl<'a> L2s<'a> {
    fn new(sw: &'a Swedish, word: &[u8], cls: u8) -> L2s<'a> {
        L2s {
            sw,
            word: word.to_vec(),
            w: Vec::new(),
            len: 0,
            b: vec![-1; 72],
            acc: vec![0; 72],
            mono: false,
            cls,
            out: Vec::new(),
            n: 0,
        }
    }

    fn c(&self, k: usize) -> u8 {
        self.w.get(k).copied().unwrap_or(0)
    }
    /// `strncmp(&w[k], s, strlen(s)) == 0`
    fn sn(&self, k: usize, s: &[u8]) -> bool {
        for (j, &x) in s.iter().enumerate() {
            let y = self.c(k + j);
            if x != y {
                return false;
            }
        }
        true
    }
    fn bb(&self, k: usize) -> i32 {
        if k == 0 {
            return self.b[0];
        }
        self.b.get(k).copied().unwrap_or(-1)
    }
    fn set_of(&self, set: u32, k: usize) -> bool {
        // 0x1000a720 / a770 / a7c0: past `~ : '`, the letter in the set
        let mut k = k;
        if k >= self.len {
            return false;
        }
        while matches!(self.c(k), b'~' | b':' | b'\'') {
            k += 1;
        }
        let c = self.c(k);
        c != 0 && self.sw.s(set).contains(&c)
    }
    fn va(&self, k: usize) -> bool {
        self.set_of(VOWELS, k)
    }
    fn ca(&self, k: usize) -> bool {
        self.set_of(CONSONANTS, k)
    }
    fn fa(&self, k: usize) -> bool {
        self.set_of(FRONT, k)
    }
    /// 0x1000a810: the `s.len()` letters before `pos` are `s` (it copies
    /// the word only when the letter at `pos` is not `~ : '`).
    fn pre(&self, pos: usize, s: &[u8]) -> bool {
        if matches!(self.c(pos), b'~' | b':' | b'\'') {
            return false;
        }
        pos >= s.len() && self.sn(pos - s.len(), s)
    }
    fn emit(&mut self, code: u32) {
        if self.out.len() <= self.n {
            self.out.resize(self.n + 1, 0);
        }
        self.out[self.n] = code;
    }
    fn inc(&mut self) {
        self.n += 1;
    }
    fn last(&self, k: usize) -> bool {
        k + 1 == self.len
    }

    fn run(&mut self, sol: Option<&[super::Entry]>, flags: &[i32; 10], li: &LexInfo) {
        let sw = self.sw;
        self.mono = monosyllable(sw, &self.word);
        let mut primary: i32 = -1;
        let mut secondary: i32 = -1;
        let mut buf: Vec<u8> = Vec::new();
        self.b[0] = 0;
        match sol.filter(|s| !s.is_empty()) {
            None => match li.slots[0][1] {
                1 => primary = li.slots[0][0],
                2 => secondary = li.slots[0][0],
                _ => {}
            },
            Some(sol) => {
                let mut base = 0i32;
                for (k, e) in sol.iter().enumerate() {
                    let s = li.slots[k];
                    if s[1] == 1 {
                        if primary < 0 {
                            primary = base + s[0];
                        }
                    } else if s[1] == 2 {
                        if s[4] == 1 && primary < 0 {
                            primary = base + s[3];
                        }
                        if secondary < 0 {
                            secondary = base + s[0];
                        }
                    } else if s[4] == 1 && primary < 0 {
                        primary = base + s[3];
                    }
                    let part = match sw.m_alt(e.m) {
                        Some(a) if !a.is_empty() => a.to_vec(),
                        _ => sw.m_str(e.m).to_vec(),
                    };
                    buf.extend_from_slice(&part);
                    if let Some(&f) = flags.get(k) {
                        if f >= 0 && buf.len() < self.b.len() {
                            self.b[buf.len()] = f;
                        }
                    }
                    base = sw.count_vowels(&buf) as u16 as i16 as i32;
                }
                for c in buf.iter_mut() {
                    if sw.flags(*c) & super::UPPER != 0 {
                        *c = sw.lower(*c);
                    }
                }
            }
        }
        if buf.is_empty() {
            buf = self.word.clone();
        }
        self.len = buf.len();
        self.w = buf;
        self.w.extend_from_slice(&[0; 8]);
        let mut i = 0usize;
        while self.c(i) != 0 {
            let c = self.c(i);
            self.acc[i.min(71)] = 0;
            if !matches!(c, b'~' | b':' | b'\'') && self.va(i) {
                self.b[0] += 1;
                if self.b[0] == primary {
                    self.acc[i.min(71)] = 1;
                } else if self.b[0] == secondary {
                    self.acc[i.min(71)] = 2;
                }
            }
            let t = match self.acc[i.min(71)] {
                1 => self.accent1(i),
                2 => self.accent2(i),
                _ => self.plain(&mut i),
            };
            if let Inc = t {
                self.inc();
            }
            i += 1;
        }
    }

    fn rcluster(&self, i: usize, set: &[&[u8]]) -> bool {
        set.iter().any(|s| self.sn(i + 1, s))
    }
    /// the common `{consonant, not m/n} n a` test: `ca(i+1) && w[i+1] not m
    /// n && w[i+2] == 'n' && w[i+3] == 'a'`
    fn cna(&self, i: usize) -> bool {
        self.ca(i + 1) && self.c(i + 1) != b'm' && self.c(i + 1) != b'n' && self.c(i + 2) == b'n' && self.c(i + 3) == b'a'
    }
    /// `!ca(i+1) || !ca(i+2) || B[i+2] == 2`: not before two consonants of
    /// one morpheme
    fn open(&self, i: usize) -> bool {
        !self.ca(i + 1) || !self.ca(i + 2) || self.bb(i + 2) == 2
    }
    /// `cls == ')' && ca(i+2) && w[i+2] == 't' && B[i+2] == 1`: an
    /// adjective's neuter -t
    fn neuter_t(&self, i: usize) -> bool {
        self.cls == b')' && self.ca(i + 2) && self.c(i + 2) == b't' && self.bb(i + 2) == 1
    }
    /// `va(i+k) || i+k-1 == len-1 || B[i+k] == 2`
    fn after(&self, i: usize, k: usize) -> bool {
        self.va(i + k) || self.last(i + k - 1) || self.bb(i + k) == 2
    }

    /// The vowel with accent 1 (0x1000ac75).
    fn accent1(&mut self, i: usize) -> Then {
        let code = match self.c(i) {
            b'a' => {
                if (self.rcluster(i, &[b"ln", b"rn", b"rl", b"rs", b"rt"]) && self.after(i, 3)) || self.c(i + 1) == b'a' {
                    0x10245
                } else if self.neuter_t(i) {
                    0x10245
                } else if !self.cna(i) {
                    if self.open(i) {
                        if !self.sn(i + 1, b"x") {
                            0x10245
                        } else {
                            0x10441
                        }
                    } else {
                        0x10441
                    }
                } else {
                    0x10245
                }
            }
            b'e' => {
                if i != 0 && self.pre(i, b":") {
                    0x10745
                } else if self.rcluster(i, &[b"rt", b"rd", b"rs", b"rn", b"rl"]) {
                    0x10685
                } else if self.sn(i + 1, b"ln") || self.neuter_t(i) || self.sn(i + 1, b"dj") {
                    0x10745
                } else if !self.cna(i) {
                    if !self.ca(i + 1) || !self.ca(i + 2) || self.bb(i + 2) == 0 {
                        if self.sn(i + 1, b"x") || (self.sn(i + 1, b"m") && self.mono) {
                            0x106c1
                        } else {
                            0x10745
                        }
                    } else {
                        0x106c1
                    }
                } else {
                    0x10745
                }
            }
            b'i' => {
                if !self.va(i + 1) && !self.sn(i + 1, b"ln") && !self.neuter_t(i) {
                    if i == 0 || self.c(i - 1) != b':' {
                        if !self.cna(i) {
                            if self.open(i) {
                                if self.sn(i + 1, b"x") || (self.sn(i + 1, b"m") && self.mono) {
                                    0x10781
                                } else {
                                    0x107c5
                                }
                            } else {
                                0x10781
                            }
                        } else {
                            0x107c5
                        }
                    } else {
                        0x107c5
                    }
                } else {
                    0x107c5
                }
            }
            b'o' => self.o1(i),
            b'u' => {
                if (self.sn(i + 1, b"rl") || self.sn(i + 1, b"rn")) && self.va(i + 3) {
                    0x105e5
                } else if self.sn(i + 1, b"ln") {
                    0x105e5
                } else if i == 0 || self.c(i - 1) != b':' {
                    if self.sn(i + 1, b"m") && self.mono {
                        0x10561
                    } else if self.neuter_t(i) {
                        0x105e5
                    } else if !self.cna(i) {
                        if self.open(i) {
                            0x105e5
                        } else {
                            0x10561
                        }
                    } else {
                        0x105e5
                    }
                } else {
                    0x105e5
                }
            }
            b'y' => {
                if self.sn(i + 1, b"ln") || self.neuter_t(i) {
                    0x107e5
                } else if i == 0 || self.c(i - 1) != b':' {
                    if !self.cna(i) {
                        if self.open(i) {
                            if self.sn(i + 1, b"x") || (self.sn(i + 1, b"m") && self.mono) {
                                0x107a1
                            } else if self.rcluster(i, &[b"rn", b"rl", b"rs", b"rt"]) && self.after(i, 3) {
                                0x107e5
                            } else {
                                0x107e5
                            }
                        } else {
                            0x107a1
                        }
                    } else {
                        0x107e5
                    }
                } else {
                    0x107e5
                }
            }
            0xe4 => {
                if (self.rcluster(i, &[b"rn", b"rl", b"rd"]) && self.after(i, 3)) || (self.sn(i + 1, b"r") && self.after(i, 2)) {
                    0x10685
                } else if self.c(i + 1) == b'r' && self.c(i + 2) == b't' && self.bb(i + 2) == 1 {
                    0x10685
                } else if self.neuter_t(i) {
                    0x10685
                } else if self.sn(i + 1, b"r") && self.ca(i + 2) {
                    0x10681
                } else if self.sn(i + 1, b"dj") {
                    0x106c5
                } else if !self.cna(i) {
                    if (!self.ca(i + 1) || !self.ca(i + 2) || self.bb(i + 2) >= 0) && !self.sn(i + 1, b"j") && !self.sn(i + 1, b"x") {
                        0x106c5
                    } else {
                        0x106c1
                    }
                } else {
                    0x106c5
                }
            }
            0xe5 => {
                if (self.rcluster(i, &[b"ln", b"rn", b"rl", b"rs", b"rt", b"rd"]) && self.after(i, 3)) || self.neuter_t(i) {
                    0x10365
                } else if self.bb(i + 1) == 2 {
                    0x10365
                } else if !self.cna(i) {
                    if self.open(i) {
                        0x10365
                    } else {
                        0x102e1
                    }
                } else {
                    0x10365
                }
            }
            0xf6 => self.oe1(i),
            _ => return Skip,
        };
        self.emit(code);
        Inc
    }

    fn o1(&self, i: usize) -> u32 {
        if !(i == 0 || self.c(i - 1) != b'~') {
            return 0x103a1;
        }
        if self.sn(i + 1, b"rn") || self.sn(i + 1, b"rd") {
            return 0x103e5;
        }
        if self.sn(i + 1, b"rl") {
            return 0x10365;
        }
        if i != 0 && self.pre(i, b":") {
            return 0x103e5;
        }
        if self.sn(i + 1, b"ge") && (self.last(i + 2) || self.bb(i + 3) == 1 || self.bb(i + 3) == 2) {
            return 0x10365;
        }
        if self.cna(i) {
            return 0x10365;
        }
        if self.neuter_t(i) {
            return 0x103e5;
        }
        if self.pre(i, b"ss") && self.sn(i + 1, b"n") {
            return 0x102e1;
        }
        if self.sn(i + 1, b"x") {
            return 0x103e5;
        }
        if self.sn(i + 1, b"ln") {
            return 0x10365;
        }
        if (self.pre(i, b"s") || self.pre(i, b"t")) && self.sn(i + 1, b"on") {
            return 0x103e5;
        }
        if self.sn(i + 1, b"m") && self.mono {
            return 0x102e1;
        }
        if !self.ca(i + 1) || (!self.va(i + 2) && !self.last(i + 1) && self.bb(i + 2) != 2) {
            if self.last(i) || self.bb(i + 1) == 2 {
                return 0x103e5;
            }
            return 0x102e1;
        }
        0x103e5
    }

    fn oe1(&self, i: usize) -> u32 {
        let main = |s: &Self| -> u32 {
            if s.rcluster(i, &[b"rt", b"rd", b"rn", b"rl"]) {
                0x10765
            } else if s.sn(i + 1, b"r") && s.after(i, 2) {
                0x10765
            } else if s.neuter_t(i) {
                0x106e5
            } else if s.sn(i + 1, b"r") && s.ca(i + 2) {
                0x106e1
            } else if !s.cna(i) {
                if s.open(i) {
                    if !s.sn(i + 1, b"m") || !s.mono {
                        0x106e5
                    } else {
                        0x10761
                    }
                } else {
                    0x10761
                }
            } else {
                0x106e5
            }
        };
        if i == 0 {
            return main(self);
        }
        if self.pre(i, b":") && self.sn(i + 1, b"r") {
            return 0x10765;
        }
        if !self.pre(i, b":") {
            return main(self);
        }
        0x106e5
    }

    /// The vowel with accent 2 (0x1000bcf7…).
    fn accent2(&mut self, i: usize) -> Then {
        let code = match self.c(i) {
            b'a' => {
                let l = (self.rcluster(i, &[b"ln", b"rn", b"rl", b"rs", b"rt"]) && self.after(i, 3)) || (i != 0 && self.pre(i, b":"));
                if l || self.sn(i + 1, b"a") || self.sn(i + 1, b"v") {
                    0x30245
                } else if !self.cna(i) {
                    if self.open(i) {
                        if self.sn(i + 1, b"x") {
                            0x30441
                        } else if (i != 0 || self.c(1) == b'v') && !self.last(i) {
                            0x30245
                        } else {
                            0x30441
                        }
                    } else {
                        0x30441
                    }
                } else {
                    0x30245
                }
            }
            b'e' => {
                if i == 0 || !self.pre(i, b":") {
                    if self.rcluster(i, &[b"rt", b"rd", b"rs", b"rn", b"rl"]) {
                        0x30685
                    } else if self.sn(i + 1, b"ln") || self.sn(i + 1, b"dj") {
                        0x30745
                    } else if !self.cna(i) {
                        if self.open(i) {
                            if !self.sn(i + 1, b"x") {
                                0x30745
                            } else {
                                0x306c1
                            }
                        } else {
                            0x306c1
                        }
                    } else {
                        0x30745
                    }
                } else {
                    0x30745
                }
            }
            b'i' => {
                if !self.va(i + 1) && !self.sn(i + 1, b"ln") {
                    if !self.cna(i) {
                        if self.open(i) {
                            if !self.sn(i + 1, b"x") {
                                0x307c5
                            } else {
                                0x30781
                            }
                        } else {
                            0x30781
                        }
                    } else {
                        0x307c5
                    }
                } else {
                    0x307c5
                }
            }
            b'o' => self.o2(i),
            b'u' => {
                if (self.sn(i + 1, b"rl") || self.sn(i + 1, b"rn")) && self.va(i + 3) {
                    0x305e5
                } else if self.sn(i + 1, b"ln") {
                    0x305e5
                } else if i == 0 || self.c(i - 1) != b':' {
                    if !self.cna(i) {
                        if self.open(i) {
                            0x305e5
                        } else {
                            0x30561
                        }
                    } else {
                        0x305e5
                    }
                } else {
                    0x305e5
                }
            }
            b'y' => {
                if self.sn(i + 1, b"ln") {
                    0x307e5
                } else if self.rcluster(i, &[b"rn", b"rl", b"rs", b"rt"]) && self.va(i + 3) {
                    0x307e5
                } else if i == 0 || self.c(i - 1) != b':' {
                    if !self.cna(i) {
                        if self.open(i) {
                            if !self.sn(i + 1, b"x") {
                                0x307e5
                            } else {
                                0x307a1
                            }
                        } else {
                            0x307a1
                        }
                    } else {
                        0x307e5
                    }
                } else {
                    0x307e5
                }
            }
            0xe4 => {
                if (self.rcluster(i, &[b"rn", b"rl", b"rs", b"rt", b"rd"]) && self.after(i, 3)) || (self.sn(i + 1, b"r") && self.after(i, 2)) {
                    0x30685
                } else if self.c(i + 1) == b'r' && self.c(i + 2) == b't' && self.bb(i + 2) == 1 {
                    0x30685
                } else if self.sn(i + 1, b"r") && self.ca(i + 2) {
                    0x30681
                } else if self.sn(i + 1, b"dj") {
                    0x306c5
                } else if !self.cna(i) {
                    if self.open(i) && !self.sn(i + 1, b"j") && !self.sn(i + 1, b"x") {
                        0x306c5
                    } else {
                        0x306c1
                    }
                } else {
                    0x306c5
                }
            }
            0xe5 => {
                let cl = self.rcluster(i, &[b"ln", b"rn", b"rl", b"rs", b"rt", b"rd"]);
                let mut code = 0x30365;
                if (!cl || (!self.va(i + 3) && !self.last(i + 2) && self.bb(i + 3) != 2)) && (i == 0 || !self.pre(i, b":")) {
                    if self.cna(i) {
                        code = 0x30365;
                    } else if self.bb(i + 1) != 2 && self.ca(i + 1) && self.ca(i + 2) && self.bb(i + 2) != 2 {
                        code = 0x302e1;
                    }
                }
                code
            }
            0xf6 => {
                if self.rcluster(i, &[b"rt", b"rd", b"rn", b"rl"]) || (self.sn(i + 1, b"r") && self.va(i + 2)) {
                    0x30765
                } else if self.sn(i + 1, b"r") && self.ca(i + 2) {
                    0x306e1
                } else if !self.cna(i) {
                    if self.open(i) {
                        0x306e5
                    } else {
                        0x30761
                    }
                } else {
                    0x30765
                }
            }
            _ => return Skip,
        };
        self.emit(code);
        Inc
    }

    fn o2(&self, i: usize) -> u32 {
        // LAB_1000c50c
        let main = |s: &Self| -> u32 {
            if s.sn(i + 1, b"m") {
                return 0x302e1;
            }
            if s.sn(i + 1, b"x") {
                return 0x303a1;
            }
            if s.sn(i + 1, b"ln") {
                return 0x30365;
            }
            if (!s.pre(i, b"s") && !s.pre(i, b"t")) || !s.sn(i + 1, b"on") {
                if !s.ca(i + 1) || (!s.va(i + 2) && !s.last(i + 1) && s.bb(i + 2) != 2) {
                    if !s.last(i) && s.bb(i + 1) != 2 && !s.sn(i + 1, b"rd") && !s.sn(i + 1, b"rn") {
                        if s.sn(i + 1, b"rl") {
                            return 0x30365;
                        }
                        if !s.cna(i) {
                            return 0x302e1;
                        }
                        return 0x303e5;
                    }
                    return 0x303e5;
                }
                return 0x303e5;
            }
            0x303e5
        };
        if i == 0 {
            return main(self);
        }
        if self.c(i - 1) != b'~' {
            if !self.pre(i, b":") {
                return main(self);
            }
            return 0x303e5;
        }
        0x303a1
    }

    /// The letters without accent (0x1000ca8d…): consonants and unstressed
    /// vowels; some cases read a following letter too (`i` moves on).
    fn plain(&mut self, ip: &mut usize) -> Then {
        let i = *ip;
        let c = self.c(i);
        let n1 = self.c(i + 1);
        macro_rules! out {
            ($x:expr) => {{
                self.emit($x);
                return Inc;
            }};
        }
        match c {
            b'b' => {
                if n1 != c {
                    if self.sn(i + 1, b"t") || self.sn(i + 1, b"s") {
                        out!(0x1ff2)
                    }
                    out!(0x1fe2)
                }
                Skip
            }
            b'c' => {
                if self.pre(i, b"c") && (self.sn(i + 1, b"e") || self.sn(i + 1, b"i") || self.sn(i + 1, b"\xe9")) {
                    out!(0x1952)
                }
                if self.fa(i + 1) || self.pre(i, b"x") {
                    out!(0x1952)
                }
                if !self.sn(i + 1, b"k") {
                    if (i == 0 || self.bb(i) == 2) && self.sn(i + 1, b"h") && self.va(i + 2) {
                        self.emit(0x501752);
                        *ip += 1;
                        return Inc;
                    }
                    if !self.sn(i + 1, b"h") || !self.va(i + 2) {
                        out!(0xff2)
                    }
                    self.emit(0xff2);
                    *ip += 1;
                    return Inc;
                }
                Skip
            }
            b'f' => {
                if n1 != c {
                    out!(0x1d52)
                }
                Skip
            }
            b'g' => {
                if n1 == c {
                    return Skip;
                }
                if self.sn(i + 1, b"k") {
                    out!(0x4fe2)
                }
                if (!self.pre(i, b"l") && !self.pre(i, b"r")) || i == 0 || self.bb(i) == 2 || self.sn(i + 1, b"i") {
                    if self.sn(i + 1, b"n") && i != 0 && self.va(i - 1) {
                        out!(0xfc2)
                    }
                    if !self.pre(i, b"n") && (self.sn(i + 1, b"t") || self.sn(i + 1, b"s")) {
                        out!(0xff2)
                    }
                    if (i == 0 || self.bb(i) == 2) && self.fa(i + 1) {
                        out!(0x10c2)
                    }
                    if i != 0
                        && matches!(self.c(i - 1), b'a' | b'e' | b'o')
                        && self.bb(i) != 2
                        && self.sn(i + 1, b"e")
                        && (self.last(i + 1) || self.bb(i + 2) == 2 || self.bb(i + 2) == 1)
                    {
                        out!(0x2001752)
                    }
                    let to_d719 = i == 0 || self.bb(i) == 2;
                    if to_d719 && self.sn(i + 1, b"j") {
                        return Skip;
                    }
                    if i != 0 && self.pre(i, b"n") && self.bb(i) != 2 {
                        return Skip;
                    }
                    out!(0xfe2)
                }
                out!(0x10c2)
            }
            b'h' => {
                if n1 == c {
                    return Skip;
                }
                if i == 0 || self.c(i - 1) != b'~' {
                    if self.last(i) || ((i == 0 || self.bb(i) == 2) && self.sn(i + 1, b"j")) {
                        return Skip;
                    }
                    out!(0x952)
                }
                out!(0x501752)
            }
            b'j' => {
                if n1 == c {
                    return Skip;
                }
                if i > 2 && self.va(i - 3) && self.pre(i, b"ss") && self.va(i + 1) {
                    return Skip;
                }
                out!(0x10c2)
            }
            b'k' => {
                if i != 0 && self.c(i - 1) == b'~' {
                    out!(0xff2)
                }
                if n1 != c && !self.sn(i + 1, b"~k") {
                    if (i == 0 || self.bb(i) == 2) && self.fa(i + 1) {
                        out!(0x1001752)
                    }
                    if (i != 0 && self.bb(i) != 2) || !self.sn(i + 1, b"j") {
                        out!(0xff2)
                    }
                    self.emit(0x1001752);
                    self.inc();
                    *ip += 1;
                }
                Skip
            }
            b'l' => {
                if n1 != c || self.pre(i, b"r") {
                    if i < 2 || !self.pre(i, b"rr") || self.bb(i) != 2 {
                        if self.pre(i, b"r") && self.sn(i + 1, b"l") {
                            return Skip;
                        }
                        if !self.pre(i, b"r") || i == 0 || self.bb(i) >= 0 {
                            if (i == 0 || self.bb(i) == 2) && self.sn(i + 1, b"j") {
                                return Skip;
                            }
                            out!(0x18a2)
                        }
                        out!(0x20016a2)
                    }
                    out!(0x18a2)
                }
                Skip
            }
            b'm' => {
                if n1 != c {
                    out!(0x1fc2)
                }
                Skip
            }
            b'n' => {
                if n1 != c || (self.pre(i, b"r") && self.bb(i + 1) == 2) {
                    if self.sn(i + 1, b"g") || self.sn(i + 1, b"k") {
                        out!(0xfc2)
                    }
                    if i < 2 || !self.pre(i, b"rr") || self.bb(i) != 2 {
                        if self.pre(i, b"r") {
                            if i == 0 {
                                out!(0x19c2)
                            }
                            if self.bb(i) < 0 {
                                out!(0x20017c2)
                            }
                        }
                        if i != 0 && self.c(i - 1) == b'~' {
                            out!(0xfc2)
                        }
                    }
                    out!(0x19c2)
                }
                Skip
            }
            b'p' => {
                if n1 != c && ((i != 0 && self.bb(i) != 2) || !self.sn(i + 1, b"s")) {
                    if !self.sn(i + 1, b"h") {
                        out!(0x1ff2)
                    }
                    self.emit(0x1d52);
                    self.inc();
                    *ip += 1;
                }
                Skip
            }
            b'q' => out!(0xff2),
            b'r' => {
                if n1 == c && [b"d", b"l", b"n", b"t", b"s"].iter().any(|s| self.sn(i + 1, *s)) {
                    out!(0x19a2)
                }
                let pass = !self.pre(i, b"f\xf6") || self.bb(i + 1) != 2 || !self.ca(i + 1) || matches!(n1, b'n' | b'l' | b's' | b'd' | b't');
                if n1 != c && pass {
                    let free2 = self.bb(i + 1) < 0 && self.bb(i + 2) < 0;
                    if self.sn(i + 1, b"sch") && free2 {
                        self.emit(0x2001752);
                        self.inc();
                        *ip += 3;
                    } else if self.sn(i + 1, b"skj") && free2 {
                        self.emit(0x501752);
                        self.inc();
                        *ip += 3;
                    } else if self.sn(i + 1, b"ski") && free2 {
                        self.emit(0x501752);
                        self.inc();
                        *ip += 2;
                    } else if ![b"d", b"l", b"n", b"t", b"s"].iter().any(|s| self.sn(i + 1, *s)) || self.bb(i + 1) >= 0 {
                        out!(0x19a2)
                    }
                }
                Skip
            }
            b's' => self.s(ip),
            b't' => self.t(ip),
            b'v' => {
                if n1 != c {
                    if !self.sn(i + 1, b"s") || self.bb(i + 1) != 2 {
                        out!(0x1d42)
                    }
                    out!(0x1d52)
                }
                Skip
            }
            b'w' => out!(0x1d42),
            b'x' => {
                let cc = self.sn(i + 1, b"c");
                self.emit(0xff2);
                self.inc();
                if !cc {
                    out!(0x1952)
                }
                Skip
            }
            b'z' => {
                if n1 != c {
                    out!(0x1952)
                }
                Skip
            }
            0x9c | b'e' => self.e0(i),
            0xdf => {
                if n1 != c {
                    self.emit(0x1952);
                    self.inc();
                }
                out!(0x1952)
            }
            0xe3 | 0xe6 | b'a' => {
                let cacd = |s: &Self| -> u32 {
                    if s.word != s.sw.s(JAG) && s.word != s.sw.s(AV) {
                        0x441
                    } else {
                        0x241
                    }
                };
                if i == 0 {
                    out!(cacd(self))
                }
                if self.c(i - 1) == b'a' && matches!(self.acc[(i - 1).min(71)], 1 | 2) {
                    return Skip;
                }
                if self.c(i - 1) != b':' {
                    out!(cacd(self))
                }
                out!(0x241)
            }
            0xe4 => {
                if self.sn(i + 1, b"r") && self.ca(i + 2) {
                    out!(0x681)
                }
                out!(0x6c1)
            }
            0xe5 => out!(0x2e1),
            0xe7 => {
                if n1 != c {
                    if i == 0 || self.c(i - 1) != b'~' {
                        out!(0x1952)
                    }
                    out!(0x1001752)
                }
                Skip
            }
            0xe9 => out!(0x10745),
            0xee | 0xef | b'i' => {
                if i != 0 && self.va(i - 1) && self.pre(i, b"ss") && self.sn(i + 1, b"on") {
                    return Skip;
                }
                if (!self.pre(i, b"s") && !self.pre(i, b"t")) || !self.sn(i + 1, b"on") {
                    if i != 0 && self.va(i - 1) && self.pre(i, b"st") && self.sn(i + 1, b"on") {
                        return Skip;
                    }
                    out!(0x781)
                }
                Skip
            }
            0xf0 | b'd' => {
                if n1 != c || self.pre(i, b"r") {
                    if i < 2 || !self.pre(i, b"rr") || self.bb(i) != 2 {
                        if !self.pre(i, b"r") || i == 0 || self.bb(i) >= 0 {
                            if self.sn(i + 1, b"s") && !self.pre(i, b"r") {
                                out!(0x19f2)
                            }
                            if ((i == 0 || self.bb(i) == 2) && self.sn(i + 1, b"j"))
                                || (self.sn(i + 1, b"t") && self.bb(i + 1) == 2)
                                || (i != 0 && self.pre(i, b"n") && self.sn(i + 1, b"n") && self.bb(i + 1) != 2)
                            {
                                return Skip;
                            }
                            out!(0x19e2)
                        }
                        out!(0x20017e2)
                    }
                    out!(0x19e2)
                }
                Skip
            }
            0xf1 => out!(0x19c2),
            0xf5 | b'o' => {
                if i != 0 {
                    if self.c(i - 1) == b'~' {
                        out!(0x3a1)
                    }
                    if self.c(i - 1) == b'o' && self.acc[(i - 1).min(71)] == 1 {
                        return Skip;
                    }
                }
                if self.sn(i + 1, b"x") {
                    out!(0x3a1)
                }
                if (!self.pre(i, b"si") && !self.pre(i, b"ti")) || !self.sn(i + 1, b"n") {
                    if n1 == b'n' && (self.last(i + 1) || self.bb(i + 2) == 2) {
                        out!(0x2e1)
                    }
                    if (!self.ca(i + 1) || !self.va(i + 2)) && (!self.ca(i + 1) || n1 == b'm' || (!self.last(i + 1) && self.bb(i + 2) != 2)) {
                        if self.last(i) || self.bb(i + 1) == 2 {
                            out!(0x3a1)
                        }
                        let rna = self.sn(i + 1, b"rna") && (self.bb(i + 4) == 2 || self.last(i + 3));
                        let rnas = self.sn(i + 1, b"rnas") && (self.bb(i + 5) == 2 || self.last(i + 4));
                        if !rna && !rnas {
                            out!(0x2e1)
                        }
                        out!(0x3a1)
                    }
                    out!(0x3a1)
                }
                out!(0x3a1)
            }
            0xf6 | 0xf8 => {
                if self.rcluster(i, &[b"rt", b"rd", b"rn", b"rl", b"rs"]) || (self.sn(i + 1, b"r") && self.va(i + 2)) {
                    out!(0x6e1)
                }
                if self.sn(i + 1, b"r") && (self.bb(i + 2) == 2 || self.last(i + 1)) {
                    out!(0x6e1)
                }
                out!(0x761)
            }
            0xfb | b'u' => {
                if i == 0 || self.c(i - 1) != b'~' {
                    if (self.sn(i + 1, b"rl") || self.sn(i + 1, b"rn")) && self.va(i + 3) {
                        out!(0x5e1)
                    }
                    if self.sn(i + 1, b"ln") {
                        out!(0x5e1)
                    }
                    if self.open(i) {
                        if !self.pre(i, b"a") {
                            if i == 0 || !self.last(i) || !self.ca(i - 1) {
                                out!(0x5e1)
                            }
                            out!(0x561)
                        }
                        out!(0x561)
                    }
                    out!(0x561)
                }
                out!(0x5e1)
            }
            0xfc | b'y' => out!(0x7a1),
            _ => Skip,
        }
    }

    /// The unstressed `e` (0x9c `œ` too).
    fn e0(&mut self, i: usize) -> Then {
        if i != 0 && self.c(i - 1) == b'~' {
            self.emit(0x741);
            return Inc;
        }
        #[derive(PartialEq)]
        enum S {
            Cc02,
            Cbda,
            Cc65,
            Rest,
        }
        let mut s = if (i == 1 || (i != 0 && self.bb(i) == 2)) && i != 0 && self.ca(i - 1) {
            S::Cc02
        } else if i == 2 {
            S::Cbda
        } else if i > 1 {
            if self.bb(i - 1) == 2 {
                S::Cbda
            } else {
                S::Cc65
            }
        } else {
            S::Rest
        };
        loop {
            s = match s {
                S::Cbda => {
                    if i > 1 {
                        if self.ca(i - 1) && self.ca(i - 2) {
                            S::Cc02
                        } else {
                            S::Cc65
                        }
                    } else {
                        S::Rest
                    }
                }
                S::Cc02 => {
                    if self.rcluster(i, &[b"rn", b"rl", b"rs", b"rt"]) {
                        self.emit(0x681);
                        return Inc;
                    }
                    S::Cc65
                }
                S::Cc65 => {
                    if i > 1 && (self.pre(i, b"og") || self.pre(i, b"ag") || self.pre(i, b"eg")) && (self.last(i) || self.bb(i + 1) == 2) {
                        return Skip;
                    }
                    S::Rest
                }
                S::Rest => break,
            };
        }
        if i == 1 {
            if self.ca(0) {
                self.emit(0x741);
                return Inc;
            }
        } else if i == 0 {
            self.emit(0x6c1);
            return Inc;
        }
        if self.bb(i) == 2 {
            self.emit(0x6c1);
            return Inc;
        }
        let trettio = i == 2 && occurs(&self.word, i, self.sw.s(TRETTIO));
        self.emit(if trettio { 0x6c1 } else { 0x501 });
        Inc
    }

    fn s(&mut self, ip: &mut usize) -> Then {
        let i = *ip;
        let c = b's';
        if i != 0 {
            if self.c(i - 1) == c && self.bb(i) != 2 {
                return Skip;
            }
            if self.c(i - 1) == b'~' {
                self.emit(0x2001752);
                return Inc;
            }
        }
        if i > 1 && self.pre(i, b"rr") && self.bb(i) == 2 {
            self.emit(0x1952);
            return Inc;
        }
        if !self.pre(i, b"r") || i == 0 || self.bb(i) >= 0 {
            if (!self.pre(i, b"rd") && !self.pre(i, b"rt")) || (!self.last(i) && self.bb(i + 1) != 2) {
                let free1 = self.bb(i + 1) < 0;
                let free2 = free1 && self.bb(i + 2) < 0;
                if self.sn(i + 1, b"ch") && free1 {
                    let code = if (i == 0 || (i > 1 && self.bb(i) == 2)) && self.va(i + 1) { 0x501752 } else { 0x2001752 };
                    self.emit(code);
                    self.inc();
                    *ip += 2;
                } else if self.sn(i + 1, b"c") && free1 {
                    self.emit(0x2001752);
                    self.inc();
                    *ip += 1;
                } else if self.sn(i + 1, b"h") && self.bb(i + 1) != 2 {
                    self.emit(0x501752);
                    self.inc();
                    *ip += 1;
                } else if self.sn(i + 1, b"ion") {
                    self.emit(0x501752);
                    return Inc;
                } else if self.sn(i + 1, b"k") && self.fa(i + 2) && free2 {
                    self.emit(0x501752);
                    self.inc();
                    *ip += 1;
                } else if (self.sn(i + 1, b"tj") || self.sn(i + 1, b"kj")) && free2 {
                    self.emit(0x501752);
                    self.inc();
                    *ip += 2;
                } else {
                    if i != 0 && self.va(i - 1) && self.sn(i + 1, b"ti") && self.va(i + 3) {
                        self.emit(0x501752);
                        return Inc;
                    }
                    if (i != 0 && self.bb(i) != 2) || !self.sn(i + 1, b"j") {
                        self.emit(0x1952);
                        return Inc;
                    }
                    self.emit(0x501752);
                    self.inc();
                    *ip += 1;
                }
                return Skip;
            }
            self.emit(0x2001752);
            return Inc;
        }
        self.emit(0x2001752);
        Inc
    }

    fn t(&mut self, ip: &mut usize) -> Then {
        let i = *ip;
        let c = b't';
        if !(self.c(i + 1) != c || self.pre(i, b"r")) {
            return Skip;
        }
        if i != 0 && self.c(i - 1) == b'~' {
            self.emit(0x59f2);
            self.inc();
            self.emit(0x1952);
            return Inc;
        }
        #[derive(PartialEq)]
        enum S {
            E087,
            E0ba,
        }
        let start = if !self.pre(i, b"s") || !self.sn(i + 1, b"ion") {
            S::E087
        } else if i == 1 {
            return Skip;
        } else if i < 2 {
            S::E0ba
        } else if self.bb(i - 1) != 2 {
            S::E087
        } else {
            return Skip;
        };
        if start == S::E087 && i > 1 && self.pre(i, b"rr") && self.bb(i) == 2 {
            self.emit(0x19f2);
            return Inc;
        }
        // LAB_1000e0ba
        if !self.pre(i, b"r") || i == 0 || self.bb(i) >= 0 {
            let ion = self.sn(i + 1, b"ion") && (self.bb(i + 4) >= 0 || self.last(i + 3));
            let ions = self.sn(i + 1, b"ions") && (self.bb(i + 5) >= 0 || self.last(i + 4));
            if !ion && !ions {
                if self.sn(i + 1, b"j") && self.fa(i + 2) && self.bb(i + 1) != 2 {
                    self.emit(0x1001752);
                    self.inc();
                    *ip += 1;
                } else if self.sn(i + 1, b"j") && (i == 0 || self.bb(i) == 2) {
                    self.emit(0x1001752);
                    self.inc();
                    *ip += 1;
                } else if !self.sn(i + 1, b"ch") {
                    self.emit(0x19f2);
                    return Inc;
                } else {
                    self.emit(0x1712);
                    self.inc();
                    *ip += 2;
                }
                return Skip;
            }
            self.emit(0x501752);
            return Inc;
        }
        self.emit(0x20017f2);
        Inc
    }
}

/// "trettio", the word 0x1000a900 looks for in the unstressed-e rule
/// (`mov edi, 0x100161dc` at 0x1000cd14, the letter's position in EAX).
const TRETTIO: u32 = 0x100161dc;

/// 0x1000a8c0: one vowel position in the word (counted by 0x1000a720,
/// which skips `~ : '`).
fn monosyllable(sw: &Swedish, w: &[u8]) -> bool {
    let n = w.len();
    let mut count = 0;
    for k in 0..n {
        let mut j = k;
        while j < n && matches!(w[j], b'~' | b':' | b'\'') {
            j += 1;
        }
        if j < n && sw.s(VOWELS).contains(&w[j]) {
            count += 1;
        }
    }
    count == 1
}

/// 0x1000a900: `s` occurs in `w` covering position `pos`, aligned on the
/// first letter of `s` equal to `w[pos]`.
fn occurs(w: &[u8], pos: usize, s: &[u8]) -> bool {
    let at = |k: usize| w.get(k).copied().unwrap_or(0);
    let mut k = 0;
    while k < s.len() {
        if s[k] == at(pos) {
            break;
        }
        k += 1;
    }
    if !s.is_empty() && pos < k {
        return false;
    }
    let base = pos - k.min(pos);
    (0..s.len()).all(|j| at(base + j) == s[j])
}
