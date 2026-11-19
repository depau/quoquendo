//! The letter-to-sound rules (0x1000d220): one switch over the letters of a
//! lower-case word, each case looking around the letter (neighbours, the
//! word's start and end, word lists) and writing phone codes.

use super::super::German;
use super::{cs, strncmp};

/// Character flag: punctuation (`DAT_100287cc` bit 8), vowel (bit 4).
const PUNCT: u32 = 0x100;
const VOWEL: u32 = 0x10;

/// Voiceless consonants "cfhkpqsßtxz" (0x10107eec), voiced "bdglmnrwv"
/// (0x10107ee0); together the consonants of the syllable checks.
const VOICELESS: u32 = 0x10107eec;
const VOICED: u32 = 0x10107ee0;
/// Muta cum liquida: "bcdfgkpt" (0x10107e9c) or "pt" (0x10107e94) + "h",
/// then "rl" (0x10107e98) and a vowel.
const MUTA: u32 = 0x10107e9c;
const PT: u32 = 0x10107e94;
const RL: u32 = 0x10107e98;
/// Endings after which a vowel is long: "gt", "gte", … (0x10107488, 0xb
/// pointers, whole rest of the word).
const GT: u32 = 0x10107488;

pub(super) struct L<'a> {
    de: &'a German,
    /// the buffer the word lives in (its bytes past the NUL are stale)
    b: &'a [u8],
    /// the word's offset in `b`
    o: usize,
    len: usize,
    i: usize,
    out: &'a mut Vec<u32>,
}

impl German {
    /// 0x1000d220 (cdecl codes, word, count, log handle): append the codes
    /// of the word at `o` in `b`.
    pub(super) fn lts(&self, out: &mut Vec<u32>, b: &[u8], o: usize) {
        let len = cs(b, o).len();
        let mut l = L { de: self, b, o, len, i: 0, out };
        if l.c(0) == 0 {
            return;
        }
        loop {
            let ch = l.c(0);
            if self.flags(ch) & PUNCT != 0 {
                // jump table 0x100126e0/0x100126d4 over 0x22..=0x2c
                match ch {
                    b'"' | b'\'' => {}
                    b'#' | b',' => l.p(0x13),
                    _ => l.p(0x17),
                }
            } else {
                l.letter(ch);
            }
            l.i += 1;
            if l.c(0) == 0 {
                return;
            }
        }
    }

    /// 0x1000cfb0: `ELQSearchString` on a copy of `s` with stress marks
    /// removed (0x10007f70).
    fn search_plain(&self, s: &[u8], table: u32, flags: u32, count: u32) -> bool {
        let plain: Vec<u8> = s.iter().map(|&c| self.unmark(c)).collect();
        self.search_string(&plain, table, flags, count).is_some()
    }
}

impl L<'_> {
    /// The byte at `k` from the current letter (0 outside the buffer).
    fn c(&self, k: isize) -> u8 {
        let p = (self.o + self.i) as isize + k;
        if p < 0 {
            0
        } else {
            self.b.get(p as usize).copied().unwrap_or(0)
        }
    }

    fn p(&mut self, code: u32) {
        self.out.push(code);
    }

    fn s(&self, va: u32) -> &[u8] {
        self.de.s(va)
    }

    /// The C string at `k` from the current letter.
    fn rest(&self, k: isize) -> &[u8] {
        cs(self.b, ((self.o + self.i) as isize + k) as usize)
    }

    /// The buffer from `k` past the current letter (stale bytes included).
    fn from(&self, k: isize) -> &[u8] {
        self.b.get(((self.o + self.i) as isize + k) as usize..).unwrap_or(&[])
    }

    fn word(&self) -> &[u8] {
        cs(self.b, self.o)
    }

    /// `strncmp(word + i + k, s, n) == 0`
    fn n(&self, k: isize, s: &[u8]) -> bool {
        strncmp(self.from(k), s, s.len()).is_eq()
    }

    /// `strncmp(word + i + k, (string at va), n) == 0`
    fn nd(&self, k: isize, va: u32, n: usize) -> bool {
        strncmp(self.from(k), self.s(va), n).is_eq()
    }

    /// `strcmp(word, (string at va)) == 0`
    fn wd(&self, va: u32) -> bool {
        self.word() == self.s(va)
    }

    /// `strcmp(word + i + k, (string at va)) == 0`
    fn rd(&self, k: isize, va: u32) -> bool {
        self.rest(k) == self.s(va)
    }

    /// `c != 0 && strchr(set at va, c)`
    fn set(&self, va: u32, c: u8) -> bool {
        c != 0 && self.s(va).contains(&c)
    }

    fn v(&self, c: u8) -> bool {
        self.de.flags(c) & VOWEL != 0
    }

    fn cons(&self, c: u8) -> bool {
        self.set(VOICELESS, c) || self.set(VOICED, c)
    }

    /// `i == len - k` (unsigned, as the pointer arithmetic)
    fn end(&self, k: u32) -> bool {
        self.i as u32 == (self.len as u32).wrapping_sub(k)
    }

    /// `i < len - k` (unsigned)
    fn before(&self, k: u32) -> bool {
        (self.i as u32) < (self.len as u32).wrapping_sub(k)
    }

    fn last(&self) -> bool {
        self.end(1)
    }

    /// `ELQSearchString(word + i + k, table, flags, count) != -1`
    fn ss(&self, k: isize, table: u32, flags: u32, count: u32) -> bool {
        self.de.search_string(self.rest(k), table, flags, count).is_some()
    }

    /// `ELQSearchString(word, table, flags, count) != -1`
    fn sw(&self, table: u32, flags: u32, count: u32) -> bool {
        self.de.search_string(self.word(), table, flags, count).is_some()
    }

    /// 0x1000cfb0 on the word
    fn pw(&self, table: u32, flags: u32, count: u32) -> bool {
        self.de.search_plain(self.word(), table, flags, count)
    }

    /// 0x1000ca20 (EAX index, EBX word, EDI string): `s` is in the word
    /// around letter `at`, aligned on the first occurrence of that letter.
    fn around(&self, at: usize, va: u32) -> bool {
        let s = self.s(va);
        let w = &self.b[self.o..];
        let l = s.len();
        let mut j = 0;
        if l != 0 {
            while j < l && s[j] != w[at] {
                j += 1;
            }
            if at < j {
                return false;
            }
        }
        let mut k = 0;
        while k < l && w[at - j + k] == s[k] {
            k += 1;
        }
        k == l
    }

    /// The letter after `c1` is `h`-like lengthening: `c1` one of `hs` and
    /// then a consonant or the word's end.
    fn lengthened(&self, hs: &[u8]) -> bool {
        hs.contains(&self.c(1)) && (self.cons(self.c(2)) || self.end(2))
    }

    /// Two consonants follow, not a muta cum liquida before a vowel nor
    /// `qu`.
    fn cluster(&self) -> bool {
        let (c1, c2, c3) = (self.c(1), self.c(2), self.c(3));
        self.cons(c1)
            && self.cons(c2)
            && !(self.set(MUTA, c1) && self.set(RL, c2) && self.v(c3))
            && !(self.set(PT, c1) && c2 == b'h' && self.set(RL, c3) && self.v(self.c(4)))
            && !(c1 == b'q' && c2 == b'u')
    }

    /// The common closed-syllable test: `x`, or a cluster not starting
    /// with `ß`.
    fn closed(&self) -> bool {
        let c1 = self.c(1);
        c1 == b'x' || (c1 != 0xdf && self.cluster())
    }

    fn letter(&mut self, ch: u8) {
        match ch {
            b'b' => self.b_(),
            b'c' => self.c_(),
            b'f' => self.double(b'f', 0x1d56, 0x1d52),
            b'g' => self.g_(),
            b'h' => self.h_(),
            b'j' => self.p(0x10c2),
            b'k' => self.double(b'k', 0x2ff6, 0x2ff2),
            b'l' => self.double(b'l', 0x18a6, 0x18a2),
            b'm' => self.double(b'm', 0x1fc6, 0x1fc2),
            b'p' => self.p_(),
            b'q' => {
                if self.c(1) == b'u' {
                    self.p(0x2ff2);
                    self.p(0x1d42);
                    self.i += 1;
                } else {
                    self.p(0x2ff2);
                }
            }
            b'r' => self.r_(),
            b's' => self.s_(),
            b't' => self.t_(),
            b'v' => self.v_(),
            b'w' => {
                let c1 = self.c(1);
                self.p(if self.set(VOICELESS, c1) { 0x1d52 } else { 0x1d42 });
            }
            b'x' => {
                self.p(0x2ff2);
                self.p(0x1952);
            }
            b'y' => {
                let code = if self.c(1) == 0xdf {
                    0x7e5
                } else if self.i == 0 || self.c(-1) == b'o' {
                    0x10c2
                } else if self.last() {
                    0x781
                } else {
                    0x7a1
                };
                self.p(code);
            }
            b'z' => {
                if self.c(1) != b'z' {
                    self.p(0x1912);
                }
            }
            0xdf => self.p(0x1956),
            0xe0 => self.a_grave(),
            0xe1 => self.a_acute(),
            0xe2 | 0xe3 | 0xe5 | b'a' => self.a_(),
            0xe4 | 0xe6 => self.ae(),
            0xe7 => self.p(0x1952),
            0xe8 | 0xe9 => self.e_grave(),
            0xea | 0xeb | b'e' => self.e_(),
            0xec | 0xed => self.i_grave(),
            0xee | 0xef | 0xff | b'i' => self.i_(),
            0xf0 | b'd' => self.d_(),
            0xf1 | b'n' => self.n_(ch),
            0xf2 => self.o_grave(),
            0xf3 => self.o_acute(),
            0xf4 | 0xf6 | 0xf8 => self.oe(),
            0xf5 | b'o' => self.o_(),
            0xf9 => self.u_grave(),
            0xfa => self.u_acute(),
            0xfb | b'u' => self.u_(),
            0xfc => self.ue(),
            0xfd => self.y_acute(),
            // anything else is logged and skipped (0x101079b0)
            _ => {}
        }
    }

    /// A consonant written double is one phone (`double`), else `single`.
    fn double(&mut self, ch: u8, double: u32, single: u32) {
        if self.c(1) == ch {
            self.p(double);
            self.i += 1;
        } else {
            self.p(single);
        }
    }

    fn b_(&mut self) {
        if !self.last() {
            let c1 = self.c(1);
            if !self.set(VOICELESS, c1) && !self.n(1, b"sel") {
                if c1 == b'b' {
                    self.p(0x1fe6);
                    self.i += 1;
                } else {
                    self.p(0x1fe2);
                }
                return;
            }
        }
        self.p(0x3ff2);
    }

    fn c_(&mut self) {
        let (c1, c2) = (self.c(1), self.c(2));
        if c1 == b'k' {
            self.p(0x2ff6);
            self.i += 1;
            return;
        }
        if c1 == b'h' && c2 == b's' && self.end(3) {
            // "chs" at the end: [ks] after these (0x10010d31..0x10010fca)
            let i = self.i;
            const KS: [u32; 9] = [0x1001e2dc, 0x10107c14, 0x10107c10, 0x10107c0c, 0x1001e55c, 0x10107c08, 0x10107c04, 0x10107c00, 0x10107bfc];
            let mut ks = i >= 2 && KS.iter().any(|&va| self.nd(-2, va, 2));
            if !ks {
                // 0 → [ks], 1 → 0x10010f8b, 2 → the general case
                let st = if i == 1 {
                    if self.c(-1) != 0xf2 {
                        2
                    } else {
                        0
                    }
                } else if i < 2 {
                    1
                } else if self.n(-2, b"la") && !(i > 4 && (self.n(-5, b"walla") || self.nd(-5, 0x10107be4, 5))) {
                    0
                } else {
                    let f8b = !self.nd(-2, 0x10107be0, 2)
                        && !self.nd(-2, 0x10107bdc, 2)
                        && !self.nd(-2, 0x10107bd8, 2)
                        && !self.nd(-2, 0x10107bd4, 2)
                        && !self.n(-2, b"da")
                        && ((!self.n(-2, b"lu") && !self.nd(-2, 0x10107bcc, 2)) || (i != 2 && self.c(-3) == b'f'));
                    if f8b {
                        1
                    } else {
                        0
                    }
                };
                ks = match st {
                    0 => true,
                    1 => i > 3 && self.nd(-4, 0x10107bc4, 4),
                    _ => false,
                };
            }
            if ks {
                self.p(0x2ff2);
                self.i += 1;
                return;
            }
        }
        if c1 == b'h' {
            if (c2 == b's' && self.before(3) && !self.n(1, b"hsch")) || self.i == 0 {
                self.p(0x2ff2);
                self.i += 1;
                return;
            }
            let a = self.c(-1);
            let ach = self.set(0x10107bbc, a) || (a == b'u' && (self.i == 1 || !self.set(0x10107bb4, self.c(-2))));
            if ach {
                self.p(0xf52);
                self.i += 1;
                return;
            }
        }
        if c1 == b'h' {
            self.p(0x1152);
            self.i += 1;
        } else if self.set(0x10107bb4, c1) {
            self.p(0x1912);
        } else if self.nd(1, 0x10103018, 2) || self.nd(1, 0x10107bb0, 2) {
            self.p(0x1952);
        } else {
            self.p(0x2ff2);
        }
    }

    fn g_(&mut self) {
        let i = self.i;
        let c1 = self.c(1);
        if i > 1
            && self.c(-1) == b'i'
            && !self.set(0x10107d9c, self.c(-2))
            && !self.n(1, b"nal")
            && !self.nd(1, 0x10107ba8, 3)
            && (self.last() || (self.cons(c1) && c1 != b'r' && c1 != b'l'))
        {
            self.p(0x1152);
            return;
        }
        if self.last() || self.set(VOICELESS, c1) {
            self.p(0x2ff2);
        } else if c1 == b'g' {
            self.p(0xfe6);
            self.i += 1;
        } else {
            self.p(0xfe2);
        }
    }

    fn h_(&mut self) {
        if self.last() {
            return;
        }
        let i = self.i;
        let (c1, c2) = (self.c(1), self.c(2));
        if self.cons(c1) || (self.pw(0x10107850, 2, 0x10) && i > 0 && self.c(-1) != b'h') {
            return;
        }
        if i == 1 && self.cons(self.c(-1)) {
            return;
        }
        // "th": silent before these (0x10011436)
        if i != 0 && self.c(-1) == b't' {
            if self.end(3) && self.v(c1) && self.v(c2) && !self.wd(0x10107b9c) {
                return;
            }
            if (self.end(2) && self.v(c1)) || self.ss(1, 0x10107890, 1, 6) || self.de.search_plain(self.rest(1), 0x101078a8, 2, 0x28) {
                return;
            }
        }
        let a = self.c(-1);
        if i < 2 || (a != b'o' && a != 0xf2) || self.c(-2) == b'w' || c1 != b'e' {
            self.p(0x952);
        }
    }

    fn p_(&mut self) {
        match self.c(1) {
            b'p' => {
                self.p(0x3ff6);
                self.i += 1;
            }
            b'h' => {
                self.p(0x1d52);
                self.i += 1;
            }
            b'f' => {
                self.p(0x1d12);
                self.i += 1;
            }
            _ => self.p(0x3ff2),
        }
    }

    fn r_(&mut self) {
        let c1 = self.c(1);
        if c1 == b'r' {
            self.p(0xd46);
            self.i += 1;
        } else if self.v(c1) {
            self.p(0xd42);
        } else if self.out.is_empty() {
            self.p(0xd42);
        } else if self.out[self.out.len() - 1] & 0xc == 4 {
            self.p(0x481);
        } else if !self.cons(c1) && !self.last() {
            self.p(0xd42);
        } else {
            self.p(0x481);
        }
    }

    fn s_(&mut self) {
        let i = self.i;
        let (c1, c2) = (self.c(1), self.c(2));
        'aef: {
            if !(c1 == b'c' && c2 == b'h') {
                break 'aef;
            }
            if (self.n(1, b"chen") && self.end(5)) || (self.n(1, b"chens") && self.end(6)) {
                if i > 2 && [0x10107b90, 0x10107b8c, 0x10107b88, 0x10107b84, 0x10107b80].iter().any(|&va| self.nd(-3, va, 3)) {
                    break 'aef;
                }
                if i > 1 && [0x10107b7c, 0x10107b78, 0x10107b74, 0x10107b70, 0x10107b6c, 0x10107b68].iter().any(|&va| self.nd(-2, va, 2)) {
                    break 'aef;
                }
                if !((i < 3 || !self.nd(-3, 0x10107b64, 3)) && (i < 2 || !self.nd(-2, 0x10107b60, 2))) {
                    break 'aef;
                }
                if i > 2 && (self.nd(-3, 0x10107b5c, 3) || self.nd(-3, 0x10107b58, 3)) {
                    break 'aef;
                }
                if !((i < 2 || !self.nd(-2, 0x10107b54, 2)) && (i < 3 || !self.nd(-3, 0x10107b50, 3))) {
                    break 'aef;
                }
                if i > 4 && (self.n(-5, b"radie") || self.nd(-5, 0x10107b40, 5)) {
                    break 'aef;
                }
            }
            self.p(0x1752);
            self.i += 2;
            return;
        }
        if c1 == b'h' && (self.cons(c2) || self.end(2)) {
            self.p(0x1756);
            self.i += 1;
            return;
        }
        if c1 == b's' {
            let ss = !self.set(0x10107b34, c2)
                || (self.end(3) && self.nd(1, 0x1001e2b4, 2))
                || (self.end(4) && self.nd(1, 0x10027c8c, 3))
                || (self.end(5) && self.nd(1, 0x10107b2c, 4));
            if ss {
                self.p(0x1956);
                self.i += 1;
                return;
            }
        }
        if self.v(c1) && (i == 0 || self.v(self.c(-1))) {
            self.p(0x1942);
            return;
        }
        let between = i >= 2
            && !((!self.v(self.c(-2)) || !self.set(0x10107b24, self.c(-1)))
                && (i < 3 || !self.v(self.c(-3)) || ![b"mm", b"nn", b"ll", b"rr", b"ng"].iter().any(|s| self.n(-2, *s))));
        if between && self.v(c1) {
            self.p(0x1942);
            return;
        }
        let tp = c1 == b't' || c1 == b'p';
        if (i == 0 || self.c(-1) == b'd') && tp {
            self.p(0x1752);
        } else if ((i < 2 || !self.nd(-2, 0x10107bd4, 2)) && (i < 3 || !self.nd(-3, 0x10107b0c, 3))) || !tp {
            let a = self.c(-1);
            let s = i == 0
                || ((a != b't' && a != b'r' && a != 0xdf && (i < 2 || a != b's' || self.c(-2) != b's')) || !tp)
                || self.around(i, 0x10107b00)
                || self.end(2)
                || (self.end(4) && c2 == b'e' && (self.c(3) == b's' || self.c(3) == b'n'))
                || (self.end(6) && c2 == b'e' && self.c(3) == b'n' && self.c(4) == b'e' && self.c(5) == b'n');
            self.p(if s { 0x1952 } else { 0x1752 });
        } else {
            self.p(0x1752);
        }
    }

    fn t_(&mut self) {
        let c1 = self.c(1);
        if c1 == b't' {
            self.p(0x39f6);
            self.i += 1;
        } else if c1 == b's' && self.c(2) == b'c' && self.c(3) == b'h' {
            self.p(0x1712);
            self.i += 3;
        } else if self.i == 0 || self.c(-1) == b's' || c1 != b'i' || !self.set(0x10107af8, self.c(2)) {
            if (self.end(2) && c1 == b's') || c1 == b'z' {
                self.p(0x1912);
                self.i += 1;
            } else {
                self.p(0x39f2);
            }
        } else {
            self.p(0x1912);
        }
    }

    fn v_(&mut self) {
        let i = self.i;
        // 0x100125c1: [f]
        let f = if i == 0 {
            self.nd(1, 0x10107af4, 3) || self.n(1, b"ier")
        } else {
            i > 6 && (self.n(-7, b"hundert") || self.n(-7, b"tausend")) && (self.n(1, b"ier") || self.nd(1, 0x10107af4, 3))
        };
        if f {
            self.p(0x1d52);
            return;
        }
        let c1 = self.c(1);
        let open = !self.n(1, b"ater")
            && !self.nd(1, 0x10107ae0, 4)
            && (i == 0 || !self.n(-1, b"e") || !self.nd(1, 0x10107ad8, 5))
            && !self.n(1, b"iel")
            && !self.nd(1, 0x10107ad0, 3);
        if open {
            if i > 2 {
                let f = self.n(-3, b"hie")
                    || self.nd(-3, 0x10107ac8, 3)
                    || ((self.n(-3, b"ner") || self.nd(-3, 0x10107ac0, 3)) && (self.n(1, b"ig") || c1 == b'e'))
                    || ((self.n(-3, b"fre") || self.nd(-3, 0x10107ab4, 3)) && self.n(1, b"el"))
                    || self.n(-3, b"lar")
                    || self.nd(-3, 0x10107aac, 3);
                if f {
                    self.p(0x1d52);
                    return;
                }
            }
            let w = !self.n(1, b"etter")
                && !self.nd(1, 0x10107a9c, 5)
                && (i < 4 || !self.nd(-4, 0x10107a94, 4) || !self.n(1, b"urlaub"))
                && (i < 12 || !self.nd(-12, 0x10107a7c, 12) || !self.n(1, b"umfrage"))
                && (!self.nd(1, 0x10107a70, 3) || (i > 1 && self.n(-2, b"di")) || (i > 5 && self.n(-6, b"kontro")))
                && !self.n(1, b"eilchen")
                && !self.nd(1, 0x10107a58, 7);
            if w {
                if self.last() || self.cons(c1) {
                    self.p(0x1d52);
                    return;
                }
                let o2 = |s: &[u8]| self.n(1, s);
                let d = |va: u32, n: usize| self.nd(1, va, n);
                let wv = d(0x10107a54, 2)
                    || d(0x10107a50, 2)
                    || o2(b"ot")
                    || d(0x10107a48, 2)
                    || (i > 2 && self.n(-3, b"gra") && (d(0x10107a40, 2) || d(0x10107a3c, 2)))
                    || ((o2(b"os") || d(0x10107a38, 2)) && (self.end(3) || self.c(3) != b's'))
                    || (i > 1 && self.n(-2, b"fa") && o2(b"or"))
                    || o2(b"oka")
                    || d(0x10107a30, 3)
                    || d(0x10107a2c, 3)
                    || o2(b"oz")
                    || o2(b"olu")
                    || d(0x10107a20, 3)
                    || o2(b"olt")
                    || d(0x10107a18, 3)
                    || d(0x10107a14, 3)
                    || o2(b"olo")
                    || (i > 2 && self.n(-3, b"kon") && (o2(b"oi") || d(0x10107a08, 2)))
                    || d(0x10107a00, 6);
                if wv || self.end(2) || !self.set(0x101079f4, c1) {
                    self.p(0x1d42);
                    return;
                }
            }
        }
        self.p(0x1d52);
    }

    /// à (0xe0)
    fn a_grave(&mut self) {
        let c1 = self.c(1);
        if self.sw(0x101074c8, 1, 0x27) || self.pw(0x101074b4, 2, 5) {
            self.p(0x10441);
        } else if self.ss(1, GT, 1, 0xb) {
            self.p(0x10445);
        } else if self.lengthened(b"ah") {
            self.p(0x10445);
            self.i += 1;
        } else if [0x10107ed8, 0x10107ed0, 0x10107ecc, 0x10107ec4, 0x10107ebc, 0x10107eb4, 0x10107ea8].iter().any(|&va| self.wd(va)) {
            self.p(0x10445);
        } else if self.closed() {
            self.p(0x10441);
        } else if c1 == b'i' || c1 == b'y' {
            self.p(0x1e445);
            self.i += 1;
        } else if c1 == b'u' {
            self.p(0x1ac45);
            self.i += 1;
        } else {
            self.p(0x10445);
        }
    }

    /// á (0xe1)
    fn a_acute(&mut self) {
        let i = self.i;
        let c1 = self.c(1);
        if self.ss(1, GT, 1, 0xb) || (i >= 3 && self.n(-3, b"spr") && self.n(1, b"ch")) {
            self.p(0x106c5);
            return;
        }
        if i != 0 {
            let a = self.c(-1);
            if (a == b'n' && self.n(1, b"chst")) || (a == b'b' && self.n(1, b"rd")) || (a == b'm' && self.n(1, b"lde")) {
                self.p(0x106c5);
                return;
            }
        }
        if !(i == 0 || ((!self.n(1, b"rst") || !self.end(4)) && (!self.n(1, b"rt") || !self.end(3)))) {
            self.p(0x106c5);
        } else if self.lengthened(b"h") {
            self.p(0x106c5);
            self.i += 1;
        } else if self.closed() {
            self.p(0x106c1);
        } else if c1 == b'u' {
            self.p(0x1e2e5);
            self.i += 1;
        } else {
            self.p(0x106c5);
        }
    }

    /// a, â, ã, å
    fn a_(&mut self) {
        let i = self.i;
        let c1 = self.c(1);
        if self.ss(1, GT, 1, 0xb) || self.around(i, 0x101035f0) || self.sw(0x10107564, 1, 8) {
            self.p(0x445);
            return;
        }
        // a final "-in" or "-tg(s)" (0x1000da9b); at i == 0 neither
        let long = if self.end(2) { i != 0 && ((self.c(-1) == b'i' && c1 == b'n') || self.ta(c1)) } else { self.ta(c1) };
        if long {
            self.p(0x445);
        } else if self.lengthened(b"ah") {
            self.p(0x445);
            self.i += 1;
        } else if c1 == b'i' || c1 == b'y' {
            self.p(0xe445);
            self.i += 1;
        } else if c1 == b'u' {
            self.p(0xac45);
            self.i += 1;
        } else if self.end(2) && c1 == b'r' {
            self.p(0x445);
        } else {
            self.p(0x441);
        }
    }

    /// 0x1000da9b: after `t`, a final `g` or `gs`.
    fn ta(&self, c1: u8) -> bool {
        self.i != 0 && self.c(-1) == b't' && ((c1 == b'g' && self.end(2)) || (self.n(1, b"gs") && self.end(3)))
    }

    /// ä, æ
    fn ae(&mut self) {
        let i = self.i;
        let c1 = self.c(1);
        if self.ss(1, GT, 1, 0xb)
            || !((i < 3 || !self.n(-3, b"spr") || !self.n(1, b"ch")) && (i == 0 || self.c(-1) != b'n' || !self.n(1, b"chst")))
            || self.around(i, 0x10107e6c)
        {
            self.p(0x6c5);
        } else if c1 == b'u' {
            self.p(0xe2e5);
            self.i += 1;
        } else if self.lengthened(b"h") {
            self.p(0x6c5);
            self.i += 1;
        } else {
            self.p(0x6c1);
        }
    }

    /// è, é
    fn e_grave(&mut self) {
        let i = self.i;
        let (c1, c2) = (self.c(1), self.c(2));
        if self.ss(1, GT, 1, 0xb) {
            self.p(0x10745);
            return;
        }
        if self.sw(0x101075c0, 1, 0x1b) || self.around(i, 0x10103594) {
            self.p(0x106c1);
            return;
        }
        if self.lengthened(b"eh") && (!self.n(1, b"en") || !self.end(3)) {
            self.p(0x10745);
            self.i += 1;
            return;
        }
        let last = self.len as u32 - 1;
        if c1 == b'u'
            && ((i != 0 && ((self.c(-1) == b'i' && (self.end(2) || c2 == b's')) || (i as u32 + 2 == last && c2 == b'r')))
                || (i != 0 && i as u32 + 4 == last && self.n(1, b"uren")))
        {
            self.p(0x10765);
            self.i += 1;
            return;
        }
        if c1 == b'u' && c2 == b'x' {
            self.p(0x10765);
            self.i += 2;
            return;
        }
        if c1 == b'r' {
            let mut closed_check = false;
            if c2 != b'd' && (!self.before(2) || !self.v(c2)) {
                let st = if i == 0 { last > 2 && self.c(2) == b's' && self.c(3) == b't' } else { self.c(-1) == b'w' && self.before(3) && c2 == b't' };
                if !st && !self.wd(0x10107e5c) && !self.wd(0x10107e54) {
                    closed_check = true;
                }
            }
            if !closed_check {
                self.p(0x10745);
                return;
            }
        }
        if (c1 == b'x' || c1 == 0xdf || self.cluster()) && c1 != 0xdf && !self.around(i, 0x10107e4c) {
            self.p(0x106c1);
        } else if c1 == b'i' || c1 == b'y' {
            self.p(0x1e445);
            self.i += 1;
        } else if c1 == b'u' && (!self.end(3) || c2 != b'm') {
            self.p(0x1e2e5);
            self.i += 1;
        } else {
            self.p(0x10745);
        }
    }

    /// e, ê, ë (from the asm, 0x1000e1c7)
    fn e_(&mut self) {
        let i = self.i;
        let c1 = self.c(1);
        if self.ss(1, GT, 1, 0xb) {
            self.p(0x745);
            return;
        }
        // final -en, -ens
        if (self.end(2) || (self.end(3) && self.c(2) == b's')) && c1 == b'n' {
            let schwa = 'e2d1: {
                if i > 0 {
                    let a = self.c(-1);
                    if self.set(0x10107e44, a) {
                        break 'e2d1 true;
                    }
                    if self.set(0x10107e34, a) {
                        break 'e2d1 !self.wd(0x10104b2c) || self.g_schwa();
                    }
                }
                if (i > 1 && self.nd(-2, 0x1001e5fc, 2)) || (i > 2 && self.nd(-3, 0x10107e30, 3)) || (i > 3 && self.nd(-4, 0x10107e28, 4)) {
                    break 'e2d1 !self.wd(0x10104b2c) || self.g_schwa();
                }
                self.g_schwa()
            };
            if schwa {
                self.p(0x501);
                return;
            }
        }
        if self.ss(0, 0x10107638, 1, 0x31) && !(i > 0 && self.set(0x10107e24, self.c(-1)) && c1 == b'l' && self.c(2) == b't') {
            self.p(0x501);
            return;
        }
        if self.ss(0, 0x10107584, 1, 0xf)
            && (!self.wd(0x1001e3a0) || (i >= 4 && (self.nd(-4, 0x10107e1c, 4) || self.nd(-4, 0x10107e14, 4)) && self.nd(1, 0x10107e78, 2)))
        {
            self.p(0x481);
            self.i += 1;
            return;
        }
        if self.lengthened(b"eh") {
            self.p(0x745);
            self.i += 1;
            return;
        }
        if self.last() {
            self.p(0x501);
            return;
        }
        let w = self.word();
        let e55c = match i {
            2 => {
                if strncmp(w, self.s(0x10107e10), 2).is_eq() || self.around(2, 0x10107e08) {
                    self.p(0x501);
                    return;
                }
                true
            }
            3 => {
                if strncmp(w, self.s(0x10107e04), 3).is_eq() || strncmp(w, self.s(0x10107e00), 3).is_eq() || self.around(3, 0x10107df4) {
                    self.p(0x501);
                    return;
                }
                true
            }
            0 => false,
            _ => true,
        };
        if (e55c && self.c(-1) == b'w' && self.nd(1, 0x10107e78, 2))
            || self.nd(1, 0x10107e84, 2)
            || self.wd(0x1001e524)
            || self.wd(0x10107dec)
            || self.wd(0x10107de4)
        {
            self.p(0x745);
            return;
        }
        if c1 == b'i' || c1 == b'y' {
            self.p(0xe445);
            self.i += 1;
        } else if c1 == b'u' {
            self.p(0xe2e5);
            self.i += 1;
        } else if (c1 == b'x' || c1 == 0xdf || self.cluster()) && !self.around(i, 0x10107ddc) {
            self.p(0x6c1);
        } else {
            self.p(0x741);
        }
    }

    /// 0x1000e301: `-gen` (not `-ngen`) ends in a schwa.
    fn g_schwa(&self) -> bool {
        self.i > 0 && self.c(-1) == b'g' && (self.i == 1 || self.c(-2) != b'n')
    }

    /// ì, í
    fn i_grave(&mut self) {
        let i = self.i;
        let (c1, c2) = (self.c(1), self.c(2));
        if self.sw(0x10107700, 1, 0x15) || self.pw(0x10107754, 2, 4) {
            self.p(0x10781);
            return;
        }
        if [0x10107dd4, 0x10107dcc, 0x10107dc4].iter().any(|&va| self.wd(va)) {
            self.p(0x107c5);
            return;
        }
        let h = self.lengthened(b"h");
        let eh = c1 == b'e' && c2 == b'h' && (self.cons(self.c(3)) || self.end(3));
        if h || eh {
            self.p(0x107c5);
            self.i += if h { 1 } else { 2 };
            return;
        }
        if [0x10107dbc, 0x10107db0, 0x10107da8].iter().any(|&va| self.wd(va)) {
            self.p(0x10781);
            self.i += 1;
            return;
        }
        if c1 == b'e' && !(self.end(3) && c2 == b'n' && (i < 3 || !self.n(-3, b"sch")) && !self.wd(0x10107da0)) {
            self.p(0x107c5);
            self.i += 1;
            return;
        }
        self.p(if self.closed() { 0x10781 } else { 0x107c5 });
    }

    /// i, î, ï, ÿ (from the asm, 0x1000ec97)
    fn i_(&mut self) {
        let i = self.i;
        let (c1, c2) = (self.c(1), self.c(2));
        if self.lengthened(b"h") {
            self.p(0x7c5);
            self.i += 1;
            return;
        }
        if self.set(0x10107d9c, c1) && c2 == b'u' {
            let c3 = self.c(3);
            if self.end(3) || c3 == b'x' || (self.end(4) && self.set(0x1001e594, c3)) {
                self.p(0x10c2);
                return;
            }
        }
        if c1 == 0xe8 {
            self.p(0x1007c1);
            return;
        }
        // false: the glide 0x1007c1 (0x1000ef64); true: on at 0x1000ef79
        let w = self.word();
        let ef79 = 'g: {
            if (i > 0 && self.v(self.c(-1))) || !self.set(0x10107d94, c1) {
                break 'g true;
            }
            if i == 1 {
                if w[0] == b'd' {
                    break 'g true;
                }
            } else if i == 2 {
                if strncmp(w, self.s(0x1001e280), 2).is_eq() {
                    break 'g true;
                }
            } else {
                if i >= 5 && (self.nd(-5, 0x10107d8c, 5) || self.nd(-5, 0x10107d84, 5)) && self.nd(1, 0x1001e65c, 2) {
                    break 'g true;
                }
                if i >= 3 && self.nd(-3, 0x10107d80, 3) && (self.nd(1, 0x10107d78, 5) || self.nd(1, 0x10107d70, 5)) {
                    break 'g true;
                }
            }
            if c1 == b'a' && i >= 3 && (self.nd(-3, 0x10107d6c, 3) || self.nd(-3, 0x10107d68, 3)) {
                break 'g true;
            }
            if c1 == b'o' {
                if i >= 4 && self.nd(-4, 0x10107d60, 4) {
                    break 'g true;
                }
                if i >= 2 && self.nd(-2, 0x1001e384, 2) {
                    break 'g true;
                }
                if i > 0 && self.c(-1) == b'b' {
                    break 'g true;
                }
            }
            if i > 0 && self.c(-1) == b'd' && (self.nd(1, 0x10107d58, 4) || self.nd(1, 0x10107d50, 4)) {
                break 'g true;
            }
            false
        };
        if !ef79 {
            self.p(0x1007c1);
            return;
        }
        if !(i > 0 && self.c(-1) == b'r') && self.set(0x10107d4c, c1) {
            self.p(0x1007c1);
            return;
        }
        if c1 == b'e' {
            if self.end(2) || (self.end(3) && self.set(0x10107d48, c2)) {
                self.p(0x1007c1);
            } else {
                self.p(0x7c5);
                self.i += 1;
            }
            return;
        }
        if (i >= 3 && self.nd(-3, 0x1001e38c, 3) && self.nd(1, 0x10107d40, 5)) || (i >= 5 && self.nd(-5, 0x10107d38, 5) && c1 == b'k') {
            self.p(0x7c5);
        } else {
            self.p(0x781);
        }
    }

    fn d_(&mut self) {
        let i = self.i;
        let (c1, c2) = (self.c(1), self.c(2));
        let w = self.word();
        if c1 == b't' {
            self.p(0x39f6);
            self.i += 1;
        } else if i == 0 && w.get(1) == Some(&b's') && w.get(2) == Some(&b'c') && w.get(3) == Some(&b'h') {
            self.p(0x1702);
            self.i = 3;
        } else if c1 == b's' && !self.set(PT, c2) && !self.n(1, b"sch") && !self.v(c2) {
            self.p(0x1912);
            self.i += 1;
        } else if self.last() || self.set(VOICELESS, c1) || (i != 0 && c1 == b'm' && self.v(self.c(-1)) && self.v(c2)) {
            self.p(0x39f2);
        } else if c1 == b'd' {
            self.p(0x19e6);
            self.i += 1;
        } else {
            self.p(0x19e2);
        }
    }

    fn n_(&mut self, ch: u8) {
        let i = self.i;
        let c1 = self.c(1);
        if ch == b'n' && c1 == b'g' && self.end(2) {
            self.p(0xfc2);
            self.i += 1;
        } else if c1 == b'g' {
            self.p(0xfc2);
            if i < 3 || !self.n(-3, b"eva") || (!self.n(1, b"gel") && !self.nd(1, 0x10107b94, 3)) {
                self.i += 1;
            }
        } else if c1 == b'k' || (c1 == b'c' && self.c(2) != b'h') {
            self.p(0xfc2);
        } else if c1 == b'n' {
            self.p(0x19c6);
            self.i += 1;
        } else {
            self.p(0x19c2);
        }
    }

    /// `h` before and `ch` after the vowel (as in "hoch").
    fn h_ch(&self) -> bool {
        !(self.i == 0 || !self.before(2) || self.c(-1) != b'h' || self.c(1) != b'c' || self.c(2) != b'h')
    }

    /// ò
    fn o_grave(&mut self) {
        let i = self.i;
        let (c1, c2) = (self.c(1), self.c(2));
        if self.sw(0x10107768, 1, 0x19) {
            self.p(0x102e1);
        } else if self.ss(1, GT, 1, 0xb) {
            self.p(0x10365);
        } else if self.lengthened(b"oh") {
            self.p(0x10365);
            self.i += 1;
        } else if self.h_ch() || (i >= 2 && self.n(-2, b"ti") && c1 == b'n') {
            self.p(0x10365);
        } else if i == 1 && self.c(-1) == b'm' && (self.n(1, b"nd") || self.n(1, b"ntag")) {
            self.p(0x10365);
        } else if c1 == b'p' && c2 == b'h' {
            self.p(0x10365);
        } else if self.closed() {
            self.p(0x102e1);
        } else if c1 == b'i' || c1 == b'y' {
            self.p(0x1e2e5);
            self.i += 1;
        } else if c1 == b'u' && !self.set(0x10107d24, c2) {
            self.p(0x103e5);
            self.i += 1;
        } else {
            self.p(0x10365);
        }
    }

    /// ó
    fn o_acute(&mut self) {
        if self.ss(0, 0x101077cc, 1, 6) {
            self.p(0x10765);
        } else if self.lengthened(b"h") {
            self.p(0x10765);
            self.i += 1;
        } else if self.ss(1, GT, 1, 0xb) || self.h_ch() {
            self.p(0x10765);
        } else {
            self.p(if self.closed() { 0x106e1 } else { 0x10765 });
        }
    }

    /// ô, ö, ø
    fn oe(&mut self) {
        let c1 = self.c(1);
        if self.ss(0, 0x101077e4, 1, 6) {
            self.p(0x765);
        } else if self.lengthened(b"h") {
            self.p(0x765);
            self.i += 1;
        } else if self.ss(1, GT, 1, 0xb) || self.h_ch() || c1 == 0xdf {
            self.p(0x765);
        } else {
            self.p(if c1 == b'x' || self.cluster() { 0x6e1 } else { 0x765 });
        }
    }

    /// o, õ
    fn o_(&mut self) {
        let i = self.i;
        if self.wd(0x10107d1c) || self.wd(0x10107d10) || self.ss(1, GT, 1, 0xb) {
            self.p(0x365);
            return;
        }
        if i != 0 {
            if self.h_ch() {
                self.p(0x365);
                return;
            }
            if self.rd(-1, 0x10107d0c) || self.rd(-1, 0x1001e498) {
                self.p(0x2e1);
                return;
            }
        }
        if self.wd(0x10107d00) || self.wd(0x10107cf8) {
            self.p(0x2e1);
            return;
        }
        let (c1, c2) = (self.c(1), self.c(2));
        if self.end(2) && c1 == b's' {
            let short = (i > 5 && (self.n(-6, b"albatr") || self.nd(-6, 0x10107ce8, 6)))
                || (i > 2 && (self.n(-3, b"cha") || self.nd(-3, 0x10107ce4, 3)))
                || (i > 3 && (self.n(-4, b"cosm") || self.nd(-4, 0x10107cd4, 4)))
                || (i > 1 && (self.n(-2, b"ep") || self.nd(-2, 0x10107cd0, 2)))
                || (i > 3 && (self.n(-4, b"kosm") || self.nd(-4, 0x10107cc0, 4)))
                || (i > 1 && self.n(-2, b"th"));
            if short {
                self.p(0x2e1);
                return;
            }
        }
        if self.lengthened(b"oh") {
            self.p(0x365);
            self.i += 1;
        } else if c1 == b'i' || c1 == b'y' {
            self.p(0xe2e5);
            self.i += 1;
        } else if c1 == b'u' && !self.set(0x10107d24, c2) {
            self.p(0x3a1);
            self.i += 1;
        } else if self.wd(0x10027e58) {
            self.p(0x2e1);
        } else if ((!self.end(2) || !self.cons(c1)) || c1 == b'r') && c1 != 0xdf {
            if c1 == b'p' && c2 == b'h' {
                self.p(0x361);
            } else if c1 == b'x' {
                self.p(0x2e1);
            } else {
                let muta1 = self.set(MUTA, c1) && self.set(RL, c2) && self.v(self.c(3));
                if self.cons(c1) && self.cons(c2) && !muta1 {
                    let muta2 = self.set(PT, c1) && c2 == b'h' && self.set(RL, self.c(3)) && self.v(self.c(4));
                    if !muta2 && !(c1 == b'q' && c2 == b'u') {
                        self.p(0x2e1);
                        return;
                    }
                }
                self.p(if self.end(2) && c1 == b'r' { 0x365 } else { 0x361 });
            }
        } else {
            self.p(0x365);
        }
    }

    /// ù
    fn u_grave(&mut self) {
        let i = self.i;
        if self.sw(0x10107800, 1, 0x14) || self.around(i, 0x10103208) {
            self.p(0x103a1);
        } else if self.ss(1, GT, 1, 0xb) {
            self.p(0x103e5);
        } else if self.lengthened(b"h") {
            self.p(0x103e5);
            self.i += 1;
        } else if (i == 0 || !self.around(i, 0x10107cb8)) && self.closed() {
            self.p(0x103a1);
        } else {
            self.p(0x103e5);
        }
    }

    /// ú
    fn u_acute(&mut self) {
        let i = self.i;
        let c1 = self.c(1);
        if self.ss(1, GT, 1, 0xb) {
            self.p(0x107e5);
            return;
        }
        if self.pw(0x1010762c, 2, 3) || (c1 == 0xdf && !self.around(i, 0x10107cb0)) {
            self.p(0x107a1);
            return;
        }
        let long = self.n(1, b"cher")
            || (i > 1 && self.n(-2, b"fl") && self.n(1, b"che"))
            || (i != 0 && {
                let a = self.c(-1);
                (a == b'd' && self.n(1, b"ster"))
                    || (a == b'w' && (self.n(1, b"chs") || self.n(1, b"st")))
                    || (a == b'b' && (self.n(1, b"chner") || self.n(1, b"ste")))
            });
        if long {
            self.p(0x107e5);
        } else if [0x10107c80, 0x10107c78, 0x10107c70].iter().any(|&va| self.wd(va)) {
            self.p(0x107a1);
        } else if self.lengthened(b"h") {
            self.p(0x107e5);
            self.i += 1;
        } else {
            self.p(if self.closed() { 0x107a1 } else { 0x107e5 });
        }
    }

    /// u, û
    fn u_(&mut self) {
        if self.i != 0 && (self.rd(-1, 0x1010394c) || self.rest(-1) == b"tumen" || self.rd(-1, 0x1010393c)) {
            self.p(0x3e5);
        } else if self.ss(1, GT, 1, 0xb) {
            self.p(0x3e5);
        } else if self.lengthened(b"h") {
            self.p(0x3e5);
            self.i += 1;
        } else {
            self.p(0x3a1);
        }
    }

    /// ü
    fn ue(&mut self) {
        let i = self.i;
        const FORMS: [u32; 9] = [0x10107c60, 0x10107c58, 0x10107c50, 0x10107c48, 0x10107c40, 0x10107c38, 0x10107c34, 0x10107c2c, 0x10107c24];
        if self.ss(1, GT, 1, 0xb) || (i != 0 && FORMS.iter().any(|&va| self.rd(-1, va))) {
            self.p(0x7e5);
        } else if self.n(1, b"cher")
            || (i > 1 && self.n(-2, b"fl") && self.n(1, b"che"))
            || (i != 0 && self.c(-1) == b'w' && self.n(1, b"chs"))
            || self.c(1) == 0xdf
        {
            self.p(0x7e5);
        } else if self.lengthened(b"h") {
            self.p(0x7e5);
            self.i += 1;
        } else {
            self.p(0x7a1);
        }
    }

    /// ý
    fn y_acute(&mut self) {
        if self.wd(0x10107c1c) {
            self.p(0x107a1);
        } else if self.before(2) && self.c(1) == b'c' && self.c(2) == b'h' {
            self.p(0x107e5);
        } else {
            self.p(if self.closed() { 0x107a1 } else { 0x107e5 });
        }
    }
}
