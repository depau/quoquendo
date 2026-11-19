//! `FonTrasParola` (0x1000bc20) of LTTS7Danish.dll: a word's phone codes
//! from its LexInfo's marked respelling (+0x250, the letters' flags at
//! +0x14c) and the current solution's morphemes.
//!
//! A word with a `TD_Phonetizations` row (LesAnalizza's 0x10009140 keeps
//! them in the record's LexInfo +8 list, here `Word::rows`) is its first
//! row's non-empty `PH_Phonetic` parsed into codes (0x1000bb50). Otherwise
//! the respelling is read letter by letter: punctuation gives a pause code,
//! a doubled consonant is read once, and each letter gives its codes from
//! its neighbours, the marks LesAccenta moved into the letters' flags (`^`
//! stress 0x100, `^^` 0x200, `"` 0x400, `?` stød 0x800, `~` 0x1000, `:`
//! length 0x2000, `-` 0x20 after, 0x10 compound start) and the morphemes
//! (the letter's morpheme followed by a `FLEX_SUFFIX`/`DERIV_SUFFIX`).
//! Letters copied from a lexicon entry's respelling (flag 0x4000) and the
//! respelling symbols `£` (0xa3, schwa) and 0x80 go through one switch
//! (the one at 0x100131bb), the other letters through the spelling rules
//! (0x1000bfa6). A code's 0x10000 bit marks primary stress, 0x80000000
//! stød.
//!
//! The code tests single letters and short strings inline (`strchr("j")`,
//! `strncmp("re")`); the letter classes are the module's strings below.

use super::{Danish, LexInfo, Morpheme};
use crate::lang::common::Word;

/// `aeiouyàãäåæéêëöøûüAEIOUY…&£` (0x10027108): vowels and respelling
/// vowels.
const VOWELS: u32 = 0x10027108;
/// `çñrptkbdgsfvhcjlmnwxzq…` (0x10027250): consonants.
const CONSONANTS: u32 = 0x10027250;
/// `ouàãäåûüOU…&£` (0x1002714c): back vowels.
const BACK: u32 = 0x1002714c;
/// `aeiyæéêëöøAEIY…` (0x10027164): front vowels.
const FRONT: u32 = 0x10027164;
/// The phone table 0x1000bb50 reads symbols from (0x10024de0).
const PHONES: u32 = 0x10024de0;

/// The morpheme kinds the rules test (0x10023320, 0x1002332c, 0x10023350).
const FLEX: &[u8] = b"FLEX_SUFFIX";
const DERIV: &[u8] = b"DERIV_SUFFIX";
const PREFIX: &[u8] = b"PREFIX";
/// `POS` (0x10026f20), `PREP` (0x10026f0c): 0x1000b8b0's feature test.
const S_POS: u32 = 0x10026f20;
const S_PREP: u32 = 0x10026f0c;

/// Where a letter's step leaves the output: `Next` past the codes it
/// wrote (0x10016ffa), `Skip` with the last code slot still open
/// (0x10016ffb).
type Step = bool;
const NEXT: Step = true;
const SKIP: Step = false;

/// FonTrasParola's state: the LexInfo bytes from the letters on (+0x14c…
/// +0x294), the position, the output slots.
struct Tr<'a> {
    da: &'a Danish,
    /// the sentence's records and the word's index (0x1000b8b0 reads the
    /// next word)
    w: &'a [Word],
    i: usize,
    /// LexInfo +0x14c…+0x2a0
    mem: [u8; 0x154],
    /// the respelling's last index (strlen - 1, at most 0x3f): the bound
    /// of the neighbour tests and of the loop
    len: usize,
    sol: Option<&'a [Morpheme]>,
    /// the record's class (+0xe) and info (+0x10)
    class: u8,
    info: u8,
    p: usize,
    out: Vec<u32>,
    n: usize,
}

const MEM: isize = 0x14c;

impl Tr<'_> {
    /// The LexInfo byte at `off`.
    fn byte(&self, off: isize) -> u8 {
        let i = off - MEM;
        if i < 0 {
            0
        } else {
            self.mem.get(i as usize).copied().unwrap_or(0)
        }
    }
    /// The respelling byte at `q` (no bound: before the respelling are the
    /// last letters' flags).
    fn sp(&self, q: isize) -> u8 {
        self.byte(0x250 + q)
    }
    /// The byte `k` after the position, unchecked.
    fn raw(&self, k: isize) -> u8 {
        self.sp(self.p as isize + k)
    }
    /// The byte `k` after the position when it lies within the respelling
    /// and its terminator, else 0.
    fn at(&self, k: isize) -> u8 {
        let q = self.p as isize + k;
        if q < 0 || q as usize > self.len {
            0
        } else {
            self.sp(q)
        }
    }
    fn is(&self, k: isize, c: u8) -> bool {
        self.at(k) == c
    }
    /// Letter `q`'s flags (+0x14e) and morpheme (+0x14c).
    fn flags_of(&self, q: isize) -> u16 {
        u16::from_le_bytes([self.byte(0x14e + 4 * q), self.byte(0x14f + 4 * q)])
    }
    fn fl(&self, k: isize) -> u16 {
        self.flags_of(self.p as isize + k)
    }
    fn mo(&self, k: isize) -> i8 {
        self.byte(0x14c + 4 * (self.p as isize + k)) as i8
    }
    /// 0x1000b790: letter `q` stressed (0x100) or after `^`.
    fn stressed_at(&self, q: isize) -> bool {
        self.flags_of(q) & 0x100 != 0 || (q != 0 && self.sp(q - 1) == b'^')
    }
    /// 0x1000b820: letter `q` marked `~` (0x1000) or after `~`.
    fn tilde_at(&self, q: isize) -> bool {
        self.flags_of(q) & 0x1000 != 0 || (q != 0 && self.sp(q - 1) == b'~')
    }
    /// 0x1000b850: letter `q` long (0x2000) or before `:`.
    fn colon_at(&self, q: isize) -> bool {
        self.flags_of(q) & 0x2000 != 0 || self.sp(q + 1) == b':'
    }
    fn stressed(&self, k: isize) -> bool {
        self.stressed_at(self.p as isize + k)
    }
    fn tilde(&self, k: isize) -> bool {
        self.tilde_at(self.p as isize + k)
    }
    fn colon(&self, k: isize) -> bool {
        self.colon_at(self.p as isize + k)
    }
    /// 0x1000b7c0, 0x1000b7e0, 0x1000b800: the `^^`, `"`, `?` flags.
    fn f200(&self, k: isize) -> bool {
        self.fl(k) & 0x200 != 0
    }
    fn f400(&self, k: isize) -> bool {
        self.fl(k) & 0x400 != 0
    }
    fn f800(&self, k: isize) -> bool {
        self.fl(k) & 0x800 != 0
    }
    /// Stød on the letter: its flag or a `?` after it.
    fn stod(&self) -> bool {
        self.f800(0) || self.is(1, b'?')
    }
    /// Secondary stress: `"` (flag or before it) or `^^`.
    fn secondary(&self) -> bool {
        self.f400(0) || self.is(-1, b'"') || self.f200(0)
    }
    /// The next letter stressed in any way (the stressed/`"`/`^^` tests on
    /// position + 1).
    fn next_stressed(&self) -> bool {
        self.stressed(1) || self.is(0, b'^') || self.f400(1) || self.is(0, b'"') || self.f200(1)
    }
    fn hyphen(&self, k: isize) -> bool {
        self.fl(k) & 0x20 != 0
    }
    fn compound(&self) -> bool {
        self.fl(0) & 0x10 != 0
    }
    /// `strchr(set, c)` for c != 0.
    fn in_(&self, set: &[u8], c: u8) -> bool {
        c != 0 && set.contains(&c)
    }
    fn in_va(&self, set: u32, c: u8) -> bool {
        c != 0 && self.da.s(set).contains(&c)
    }
    fn vowel(&self, c: u8) -> bool {
        self.in_va(VOWELS, c)
    }
    fn consonant(&self, c: u8) -> bool {
        self.in_va(CONSONANTS, c)
    }
    /// `strncmp(&sp[p + k], s, len(s)) == 0`.
    fn starts(&self, k: isize, s: &[u8]) -> bool {
        s.iter().enumerate().all(|(j, &c)| self.raw(k + j as isize) == c)
    }
    /// `p == len - k || flag 0x20 at letter p + k`: letter `p + k` is the
    /// last one, or has a `-` after it.
    fn ends(&self, k: usize) -> bool {
        self.p + k == self.len || self.hyphen(k as isize)
    }
    /// The morpheme after morpheme `c` (the solution's `c + 1`), when `c`
    /// is a morpheme.
    fn after(&self, c: i8) -> Option<&Morpheme> {
        let sol = self.sol?;
        if c < 0 || (c as i32 + 1) as u32 >= sol.len() as u32 {
            return None;
        }
        sol.get(c as usize + 1)
    }
    fn kind_after(&self, c: i8, kind: &[u8]) -> bool {
        self.after(c).is_some_and(|m| m.kind == kind)
    }
    /// Morpheme `c` followed by a suffix.
    fn suffix(&self, c: i8) -> bool {
        self.kind_after(c, FLEX) || self.kind_after(c, DERIV)
    }
    /// The letter ends its morpheme (`mo(k) != mo(k+1)`) and a suffix
    /// follows it.
    fn before_suffix(&self) -> bool {
        let c = self.mo(0);
        c != self.mo(1) && c >= 0 && self.suffix(c)
    }

    fn e(&mut self, x: u32) {
        if self.out.len() <= self.n {
            self.out.resize(self.n + 1, 0);
        }
        self.out[self.n] = x;
    }
    /// One code, then the next slot.
    fn put(&mut self, x: u32) -> Step {
        self.e(x);
        NEXT
    }
    /// Two codes.
    fn put2(&mut self, a: u32, b: u32) -> Step {
        self.e(a);
        self.n += 1;
        self.e(b);
        NEXT
    }
    /// One code, the position moved past `k` more letters.
    fn put_skip(&mut self, x: u32, k: usize) -> Step {
        self.e(x);
        self.p += k;
        NEXT
    }
    /// One code written and counted, `k` more letters consumed, then on
    /// without opening a slot (`E; N; p += k; goto 0x10016ffb`).
    fn put_n(&mut self, x: u32, k: usize) -> Step {
        self.e(x);
        self.n += 1;
        self.p += k;
        SKIP
    }

    /// The loop at 0x1000bcf0.
    fn run(&mut self) {
        loop {
            let c = self.sp(self.p as isize);
            if c == 0 {
                break;
            }
            if self.letter(c) {
                self.n += 1;
            }
            self.p += 1;
            if self.p > self.len {
                break;
            }
        }
        self.out.truncate(self.n);
    }

    fn letter(&mut self, c: u8) -> Step {
        if self.da.has(c, super::PUNCT) {
            return match c {
                b'"' | b'\'' | b'(' | b')' | b':' | b'@' | b'~' => SKIP,
                b'#' | b',' => self.put(2),
                _ => self.put(3),
            };
        }
        if self.consonant(c) && !self.hyphen(0) {
            // a doubled consonant (but rr, cc) is read once
            let next = if self.p + 1 > self.len { 0 } else { self.raw(1) };
            if next == c && !matches!(c, b'r' | b'R' | b'c' | b'C') {
                if self.f800(0) || self.is(1, b'?') || self.f800(1) || self.is(2, b'?') {
                    return match c {
                        b'D' | b'd' => {
                            let x = if self.tilde(0) { self.put2(0x80001b42, 0x200019f2) } else { self.put(0x80001b42) };
                            self.p += 1;
                            x
                        }
                        b'L' | b'l' => self.put_skip(0x800018a2, 1),
                        b'M' | b'm' => self.put_skip(0x80001fc2, 1),
                        b'N' | b'n' => self.put_skip(if self.tilde(0) { 0x80000fc2 } else { 0x800019c2 }, 1),
                        _ => SKIP,
                    };
                }
                if c == b'd' && self.tilde(0) {
                    self.put2(0x1b42, 0x200019f2);
                    self.p += 1;
                    return NEXT;
                }
                return SKIP;
            }
        }
        // 0x1000b880: a letter of an entry's respelling, or £, 0x80
        let resp = c == 0xa3 || c == 0x80 || self.fl(0) & 0x4000 != 0;
        if resp {
            self.respelled(c)
        } else {
            self.spelled(c)
        }
    }
}

/// The switch at 0x100131bb: letters of an entry's respelling.
impl Tr<'_> {
    fn respelled(&mut self, c: u8) -> Step {
        match c {
            b'&' => self.resp_amp(),
            b'A' | b'a' => {
                if !self.stressed(0) {
                    if self.stod() {
                        self.put(0x80000685)
                    } else if self.colon(0) {
                        self.put(0x685)
                    } else {
                        self.put(0x441)
                    }
                } else if self.stod() {
                    self.put(0x80010685)
                } else if self.colon(0) {
                    self.put(0x10685)
                } else {
                    self.put(0x10441)
                }
            }
            b'B' | b'b' => self.put(0x20001ff2),
            b'C' | b'c' => self.put(0x1952),
            b'D' | b'd' => self.resp_d(),
            b'E' | b'e' => self.resp_e(),
            b'F' | b'f' => self.put(0x1d52),
            b'G' | b'g' => self.put(if self.stod() { 0x800010c2 } else { 0x20000ff2 }),
            b'H' | b'h' => self.put(0x952),
            b'I' | b'i' => self.vowel_codes([0x7c1, 0x800007c5, 0x7c1, 0x7c5, 0x107c1, 0x107c5, 0x800107c5]),
            b'J' | b'j' => self.letter_j(),
            b'K' | b'k' => self.letter_k(),
            b'L' | b'l' => self.letter_l(),
            b'M' | b'm' => self.letter_m(),
            b'N' | b'n' => {
                let x = match (self.stod(), self.tilde(0)) {
                    (false, false) => 0x19c2,
                    (false, true) => 0xfc2,
                    (true, false) => 0x800019c2,
                    (true, true) => 0x80000fc2,
                };
                self.put(x)
            }
            b'O' | b'o' => self.vowel_codes([0x361, 0x80000365, 0x361, 0x365, 0x10361, 0x10365, 0x80010365]),
            b'P' | b'p' => {
                if self.p == 0 {
                    if self.in_(b"\xe5", self.raw(1)) && (self.stressed_at(1) || self.sp(0) == b'^') && self.flags_of(1) & 0x20 != 0 {
                        self.put2(0x3ff2, 0x102c1);
                        self.p = 1;
                        return NEXT;
                    }
                    return self.put(0x3ff2);
                }
                self.p_plain()
            }
            b'R' | b'r' => self.resp_r(),
            b'S' | b's' => self.resp_s(),
            b'T' | b't' => self.resp_t(),
            b'U' | b'u' => self.resp_u(),
            b'V' | b'v' => self.put(if self.stod() { 0x80401ec2 } else { 0x1d42 }),
            b'W' | b'w' => self.put(if self.stod() { 0x80401ec2 } else { 0x401ec2 }),
            b'Y' | b'y' => self.vowel_codes([0x7e1, 0x800007e5, 0x7e1, 0x7e5, 0x107e1, 0x107e5, 0x800107e5]),
            0x80 => self.put(0x501),
            0xa3 => self.resp_schwa(),
            0xc4 | 0xe4 => self.vowel_codes([0x681, 0x80000685, 0x681, 0x685, 0x10681, 0x10685, 0x80010685]),
            0xc5 | 0xe5 => {
                if !self.stressed(0) {
                    if self.stod() {
                        self.put(0x800002e5)
                    } else if self.colon(0) {
                        self.put(0x2e5)
                    } else {
                        self.put(0x2e1)
                    }
                } else if self.stod() {
                    self.put(0x800102e5)
                } else if self.colon(0) {
                    self.put(0x102e5)
                } else {
                    self.put(0x102e1)
                }
            }
            0xc6 | 0xe6 => {
                if !self.stressed(0) {
                    if !self.secondary() {
                        if self.stod() {
                            self.put(0x800006c5)
                        } else if self.colon(0) {
                            self.put(0x685)
                        } else {
                            self.put(0x6c1)
                        }
                    } else if self.stod() {
                        self.put(0x800006c5)
                    } else if self.colon(0) {
                        self.put(0x6c5)
                    } else {
                        self.put(0x6c1)
                    }
                } else if self.stod() {
                    self.put(0x800106c5)
                } else if self.colon(0) {
                    self.put(0x106c5)
                } else {
                    self.put(0x106c1)
                }
            }
            0xd6 | 0xf6 => {
                if !self.stressed(0) {
                    if !self.secondary() {
                        self.put(if self.stod() { 0x800006e5 } else { 0x6e1 })
                    } else if self.stod() {
                        self.put(0x800006e5)
                    } else if self.colon(0) {
                        self.put(0x6e5)
                    } else {
                        self.put(0x6e1)
                    }
                } else if self.stod() {
                    self.put(0x800106e5)
                } else if self.colon(0) {
                    self.put(0x106e5)
                } else {
                    self.put(0x106e1)
                }
            }
            0xd8 | 0xf8 => self.resp_oe(),
            _ => SKIP,
        }
    }

    /// The common vowel pattern: `[plain, stød, secondary, secondary long,
    /// stressed, stressed long, stressed stød]`; unstressed without
    /// secondary stress has no long form.
    fn vowel_codes(&mut self, x: [u32; 7]) -> Step {
        let v = if !self.stressed(0) {
            if !self.secondary() {
                if self.stod() {
                    x[1]
                } else {
                    x[0]
                }
            } else if self.stod() {
                x[1]
            } else if self.colon(0) {
                x[3]
            } else {
                x[2]
            }
        } else if self.stod() {
            x[6]
        } else if self.colon(0) {
            x[5]
        } else {
            x[4]
        };
        self.put(v)
    }

    /// The spelling switch's `j`, `l`, `m` (0x1000bfa6 cases 0x4a, 0x4c,
    /// 0x4d), which the respelling switch jumps to.
    fn letter_j(&mut self) -> Step {
        self.put(if self.stod() { 0x800010c2 } else { 0x10c2 })
    }
    /// `k` aspirated first, at a compound start or before a stressed vowel
    /// (not after `s`).
    fn letter_k(&mut self) -> Step {
        let x = if self.p == 0
            || self.compound()
            || ((self.p == 0 || !self.in_(b"s", self.raw(-1))) && self.vowel(self.at(1)) && self.next_stressed())
        {
            0x2ff2
        } else {
            0x20000ff2
        };
        self.put(x)
    }
    fn letter_l(&mut self) -> Step {
        self.put(if self.stod() { 0x800018a2 } else { 0x18a2 })
    }
    fn letter_m(&mut self) -> Step {
        self.put(if self.stod() { 0x80001fc2 } else { 0x1fc2 })
    }

    /// `p` not first (0x1001111c): unaspirated after `s`, before a
    /// consonant or an unstressed vowel.
    fn p_plain(&mut self) -> Step {
        let x = if !self.compound()
            && ((self.p != 0 && self.in_(b"s", self.raw(-1))) || !self.vowel(self.at(1)) || !self.next_stressed())
        {
            0x20001ff2
        } else {
            0x3ff2
        };
        self.put(x)
    }

    fn resp_amp(&mut self) -> Step {
        let schwa_end = self.raw(1) == 0xa3 && self.at(2) == 0;
        if !self.stressed(0) {
            if self.stod() {
                return self.put(0x80000245);
            }
            if self.in_(b")", self.raw(1)) {
                return self.put(0x241);
            }
            if !self.secondary() {
                if !self.colon(0) || self.raw(1) != 0xa3 || self.at(2) != 0 {
                    if schwa_end {
                        self.put_skip(0x245, 1)
                    } else if !self.colon(0) {
                        self.put(0x241)
                    } else {
                        self.put(0x245)
                    }
                } else {
                    self.put_skip(0x245, 1)
                }
            } else if !self.colon(0) || self.raw(1) != 0xa3 || self.at(2) != 0 {
                if !self.colon(0) {
                    if self.raw(1) != 0xa3 || self.at(2) != 0 {
                        return self.put(0x241);
                    }
                    self.put_skip(0x245, 1)
                } else {
                    self.put(0x245)
                }
            } else {
                self.put_skip(0x245, 1)
            }
        } else {
            if self.stod() {
                return self.put(0x80010245);
            }
            if !self.colon(0) || self.raw(1) != 0xa3 || self.at(2) != 0 {
                if schwa_end {
                    self.put_skip(0x10245, 1)
                } else if !self.in_(b")", self.raw(1)) {
                    self.put(if self.colon(0) { 0x10245 } else { 0x10241 })
                } else {
                    self.put_skip(0x10241, 1)
                }
            } else {
                self.put_skip(0x10245, 1)
            }
        }
    }

    fn resp_d(&mut self) -> Step {
        if self.stod() {
            return self.put(0x80001b42);
        }
        let til = self.tilde(0);
        let j = self.in_(b"j", self.raw(1));
        if til && j && (self.tilde(1) || self.is(0, b'~')) {
            self.put2(0x1b42, 0x10c2);
            self.p += 1;
            return NEXT;
        }
        if til && j {
            return self.put_skip(0x1702, 1);
        }
        if til && self.in_(b"t", self.raw(1)) {
            let c = self.mo(0);
            if c != self.mo(1) && c >= 0 && self.suffix(c) {
                return self.put_skip(0x200019f2, 1);
            }
        }
        if til {
            return self.put(0x1b42);
        }
        if !self.in_(b"t", self.raw(1)) || self.compound() {
            return self.put(0x200019f2);
        }
        self.put_skip(0x200019f2, 1)
    }

    fn resp_e(&mut self) -> Step {
        if self.stressed(0) {
            return self.put(if self.stod() {
                0x80010745
            } else if self.colon(0) {
                0x10745
            } else {
                0x10741
            });
        }
        if self.secondary() {
            return self.put(if self.stod() {
                0x80000745
            } else if self.colon(0) {
                0x745
            } else {
                0x741
            });
        }
        if self.stod() {
            return self.put(0x80000745);
        }
        if self.colon(0) {
            return self.put(0x745);
        }
        if self.starts(1, b"rer") && self.at(4) == 0 {
            return self.put_n(0x481, 3);
        }
        let re = self.starts(1, b"re");
        if !re || self.ends(2) || self.hyphen(0) || self.hyphen(1) {
            if (self.class == 0x15 || self.class == 0x29) && re && self.ends(2) {
                return self.schwa_er();
            }
            if self.starts(1, b"r") && self.ends(1) && !self.hyphen(0) {
                return self.put_skip(0x481, 1);
            }
            if self.p == self.len || self.hyphen(0) || self.before_suffix() {
                return self.put(0x501);
            }
            return self.put(0x741);
        }
        self.put_n(0x481, 2)
    }

    /// 0x1000c92b: `-ere` of a verb said `ɐ ɐ`.
    fn schwa_er(&mut self) -> Step {
        self.e(0x481);
        self.n += 1;
        self.e(0x481);
        self.n += 1;
        self.p += 2;
        SKIP
    }

    fn resp_r(&mut self) -> Step {
        if self.stod() {
            if self.starts(1, b"re") {
                let c = self.mo(0);
                if self.len <= self.p + 2 || self.hyphen(2) || (self.mo(2) != self.mo(3) && c >= 0 && self.suffix(c)) {
                    self.put2(0x80000485, 0x481);
                    self.p += 2;
                    return NEXT;
                }
            }
            if self.in_(b"r", self.raw(1)) {
                return self.put_n(0x80000485, 1);
            }
            if self.in_(b"e", self.raw(1)) && !self.next_stressed() && !self.hyphen(0) && !self.before_suffix() {
                return self.put_skip(0x80000485, 1);
            }
            return self.put(0x80000485);
        }
        if self.consonant(self.at(-1)) && self.starts(1, b"e") && self.ends(1) && !self.next_stressed() {
            self.put2(0xd42, 0x481);
            self.p += 1;
            return NEXT;
        }
        if self.class == 0x15 && matches!(self.info, 0x10 | 0xce | 0xcb | 0x0e | 0x0c) && self.starts(1, b"er") && self.at(3) == 0 {
            return self.schwa_er();
        }
        if !self.consonant(self.at(-1)) && self.starts(1, b"er") && self.ends(2) {
            return self.put_n(0x481, 2);
        }
        if ((self.starts(1, b"e") && self.ends(1)) || (self.raw(1) == 0x80 && self.ends(1)))
            && self.consonant(self.at(-1))
            && !self.next_stressed()
            && !self.hyphen(0)
        {
            self.put2(0xd42, 0x481);
            self.p += 1;
            return NEXT;
        }
        if self.p == 0 || self.compound() || self.consonant(self.at(-1)) {
            return self.put(0xd42);
        }
        self.r_after_vowel()
    }

    /// Respelled `r` after a vowel, from its `e` test on (…0x10015c8a).
    fn r_after_vowel(&mut self) -> Step {
        if self.in_(b"e", self.raw(1)) && !self.next_stressed() && !self.hyphen(0) && !self.before_suffix() {
            return self.put_skip(0x481, 1);
        }
        // 0x10015c8a
        if self.raw(1) == 0xa3 && (self.len <= self.p + 1 || self.hyphen(1) || (self.mo(1) != self.mo(2) && self.mo(0) >= 0 && self.suffix(self.mo(0)))) {
            return self.put_skip(0x481, 1);
        }
        if self.in_(b"r", self.raw(1)) && !self.hyphen(0) {
            return self.put_skip(0xd42, 1);
        }
        let c = self.mo(0);
        if c != self.mo(1) && c >= 0 {
            if let Some(m) = self.after(c).filter(|m| m.kind == DERIV) {
                // `-inde` (0x100271b0), `-agtig` (0x100271a8)
                if m.text == b"inde" || m.text == b"agtig" {
                    return self.put(0x481);
                }
            }
        }
        self.r_final()
    }

    /// 0x10015f09: `r` after a vowel, not before `e`.
    fn r_final(&mut self) -> Step {
        if !self.vowel(self.at(-1)) {
            return self.put(0xd42);
        }
        if self.consonant(self.at(1)) || self.p == self.len || self.hyphen(0) {
            return self.put(0x481);
        }
        let c = self.mo(0);
        if c != self.mo(1) && c >= 0 && self.suffix(c) {
            return self.put(0x481);
        }
        self.put(0xd42)
    }

    fn resp_s(&mut self) -> Step {
        let til = self.tilde(0);
        if til && self.in_(b"j", self.raw(1)) {
            self.e(0x1952);
            self.n += 1;
            return self.put_n(0x10c2, 1);
        }
        let i = self.in_(b"i", self.raw(1));
        let st1 = self.stressed(1) || self.is(0, b'^');
        let long = self.colon(1) || self.is(2, b':');
        if til && i && long && st1 {
            self.put2(0x1952, 0x107c5);
            self.p += 1;
            return NEXT;
        }
        if til && i && long {
            self.put2(0x1952, 0x7c5);
            self.p += 1;
            return NEXT;
        }
        if til && i && !st1 {
            self.put2(0x1952, 0x7c1);
            self.p += 1;
            return NEXT;
        }
        if til && i {
            self.put2(0x1952, 0x107c1);
            self.p += 1;
            return NEXT;
        }
        if self.in_(b"j", self.raw(1)) && !self.hyphen(0) {
            return self.put_n(0x1001752, 1);
        }
        if !i || !self.vowel(self.at(2)) || self.stressed(1) || self.is(0, b'^') {
            return self.put(0x1952);
        }
        self.put_skip(0x1001752, 1)
    }

    fn resp_t(&mut self) -> Step {
        let til = self.tilde(0);
        if til && self.in_(b"h", self.raw(1)) {
            return self.put_n(0x39f2, 1);
        }
        if til && self.in_(b"j", self.raw(1)) {
            return self.put_n(0x1712, 1);
        }
        if self.p != 0
            && !self.compound()
            && ((self.p != 0 && self.in_(b"s", self.raw(-1))) || !self.vowel(self.at(1)) || !self.next_stressed())
        {
            if self.in_(b"r", self.raw(1)) {
                return self.put(0x39f2);
            }
            return self.put(0x200019f2);
        }
        self.put(0x39f2)
    }

    fn resp_u(&mut self) -> Step {
        if !self.stressed(0) {
            return self.put(if self.stod() {
                0x800003e5
            } else if self.colon(0) {
                0x3e5
            } else {
                0x3e1
            });
        }
        if self.stod() {
            return self.put(0x800103e5);
        }
        if self.colon(0) {
            return self.put(0x103e5);
        }
        if self.p == 0 {
            if let Some(x) = self.u_first_d() {
                return x;
            }
        }
        self.put(0x103e1)
    }

    /// A stressed first `u` before `d` (`ud`, 0x1000ea33/0x10014c7a).
    fn u_first_d(&mut self) -> Option<Step> {
        if self.starts(1, b"dd") {
            // 0x1000ea33
            self.e(0x103e1);
            self.n += 1;
            self.e(0x1b42);
            self.n += 1;
            self.e(0x200019f2);
            self.p = 2;
            return Some(NEXT);
        }
        let d = self.in_(b"d", self.raw(1));
        if d && (self.f800(1) || (2 <= self.len && self.raw(2) == b'?')) && !self.tilde_at(1) && self.sp(0) != b'~' {
            self.put2(0x103e1, 0x80001b42);
            self.p = 1;
            return Some(NEXT);
        }
        if d && !self.tilde_at(1) && self.sp(0) != b'~' && self.mo(0) == self.mo(1) {
            // 0x1000eae2
            self.put2(0x103e1, 0x1b42);
            self.p = 1;
            return Some(NEXT);
        }
        None
    }

    /// `£` (0xa3): schwa, or `ɐ` with an `r`.
    fn resp_schwa(&mut self) -> Step {
        if self.stressed(0) {
            if self.stod() {
                if self.in_(b"r", self.raw(1)) {
                    return self.put_n(0x80010485, 1);
                }
                return self.put(0x80010485);
            }
            if self.tilde(0) && self.in_(b"e", self.raw(1)) && (self.len <= self.p + 1 || self.hyphen(0) || self.before_suffix()) {
                return self.put_skip(0x10481, 1);
            }
            if self.tilde(0) {
                return self.put(0x10481);
            }
            if self.p == self.len || self.hyphen(0) || self.before_suffix() {
                return self.put(0x10481);
            }
            return self.put(0x102c1);
        }
        if self.secondary() {
            if self.stod() {
                return self.put(0x80000485);
            }
            if self.tilde(0) && self.in_(b"e", self.raw(1)) && (self.len <= self.p + 1 || self.hyphen(0) || self.before_suffix()) {
                return self.put_skip(0x481, 1);
            }
            if self.tilde(0) {
                return self.put(0x481);
            }
            return self.put(0x2c1);
        }
        if self.stod() {
            return self.put(0x80000485);
        }
        if self.tilde(0) && self.in_(b"e", self.raw(1)) && (self.len <= self.p + 1 || self.hyphen(0) || self.before_suffix()) {
            return self.put_skip(0x481, 1);
        }
        if self.colon(0) {
            return self.put(0x2e5);
        }
        if self.tilde(0) {
            return self.put(0x481);
        }
        if !self.in_(b"r", self.raw(1)) || self.is(2, b'r') {
            if (self.class == 0x15 || self.class == 0x29) && self.starts(1, b"e") && self.ends(1) {
                self.put2(0x481, 0x481);
                self.p += 1;
                return NEXT;
            }
            if !self.vowel(self.at(-1)) || !self.vowel(self.at(1)) {
                if self.p == self.len || self.hyphen(0) || self.before_suffix() {
                    return self.put(0x481);
                }
                return self.put(0x2c1);
            }
            return self.put(0x481);
        }
        self.put_skip(0x481, 1)
    }

    fn resp_oe(&mut self) -> Step {
        let til = self.tilde(0);
        if !self.stressed(0) {
            if self.stod() {
                return self.put(if til { 0x800006e5 } else { 0x80000765 });
            }
            if !self.secondary() {
                return self.put(if til { 0x6e1 } else { 0x761 });
            }
            if !til || !self.colon(0) {
                if til {
                    return self.put(0x6e1);
                }
                return self.put(if self.colon(0) { 0x765 } else { 0x761 });
            }
            return self.put(0x6e5);
        }
        if self.stod() {
            return self.put(if til { 0x800106e5 } else { 0x80010765 });
        }
        if !til || !self.colon(0) {
            if !til {
                return self.put(if self.colon(0) { 0x10765 } else { 0x10761 });
            }
            return self.put(0x106e1);
        }
        self.put(0x106e5)
    }
}

/// The switch at 0x1000bfa6: the spelling rules.
impl Tr<'_> {
    fn spelled(&mut self, c: u8) -> Step {
        match c {
            b'A' | b'a' => self.sp_a(),
            b'B' | b'b' => self.put(0x20001ff2),
            b'C' | b'c' => {
                if self.p == 0 || !self.in_(b"oua", self.raw(-1)) || !self.in_(b"c", self.raw(1)) {
                    if self.in_(b"h", self.raw(1)) {
                        return self.put_n(0x1001752, 1);
                    }
                    if !self.in_(b"aou", self.raw(1)) {
                        return self.put(0x1952);
                    }
                }
                self.put(0x20000ff2)
            }
            b'D' | b'd' => self.sp_d(),
            b'E' | b'e' => self.sp_e(),
            b'F' | b'f' => self.sp_f(),
            b'G' | b'g' => self.sp_g(),
            b'H' | b'h' => {
                if self.p != 0 || !self.in_(b"vj", self.sp(1)) {
                    return self.put(0x952);
                }
                SKIP
            }
            b'I' | b'i' => self.sp_i(),
            b'J' | b'j' => self.letter_j(),
            b'K' | b'k' => self.letter_k(),
            b'L' | b'l' => self.letter_l(),
            b'M' | b'm' => self.letter_m(),
            b'N' | b'n' => {
                if self.stod() {
                    if self.in_(b"g", self.raw(1)) {
                        return self.put_n(0x80000fc2, 1);
                    }
                    return self.put(0x800019c2);
                }
                if self.in_(b"g", self.raw(1)) {
                    let c = self.mo(0);
                    if self.len <= self.p + 1 || self.hyphen(1) || (self.mo(1) != self.mo(2) && c >= 0 && self.suffix(c)) {
                        return self.put_skip(0xfc2, 1);
                    }
                    return self.put(0xfc2);
                }
                self.put(if self.in_(b"k", self.raw(1)) { 0xfc2 } else { 0x19c2 })
            }
            b'O' | b'o' => self.sp_o(),
            b'P' | b'p' => {
                if self.p == 0 && self.in_(b"\xe5", self.sp(1)) && (self.stressed_at(1) || self.sp(0) == b'^') && self.flags_of(1) & 0x20 != 0 {
                    self.put2(0x3ff2, 0x102c1);
                    self.p = 1;
                    return NEXT;
                }
                if (self.hyphen(0) || !self.in_(b"pb", self.raw(1))) && (self.p != 0 || !self.in_(b"s", self.sp(1))) {
                    if !self.in_(b"h", self.raw(1)) || (self.p != 0 && !self.compound()) {
                        if self.p == 0 {
                            return self.put(0x3ff2);
                        }
                        return self.p_plain();
                    }
                    return self.put_n(0x1d52, 1);
                }
                SKIP
            }
            b'Q' | b'q' => {
                if self.starts(1, b"ue") {
                    return self.put_n(0x20000ff2, 2);
                }
                let x = if self.p == 0 || (self.vowel(self.at(1)) && (self.stressed(1) || self.is(0, b'^'))) {
                    0x2ff2
                } else {
                    0x20000ff2
                };
                self.put(x)
            }
            b'R' | b'r' => self.sp_r(),
            b'S' | b's' => {
                if self.starts(1, b"ch") && !self.hyphen(0) && !self.hyphen(1) {
                    return self.put_n(0x1001752, 2);
                }
                if (!self.in_(b"h", self.raw(1)) || self.hyphen(0))
                    && !self.in_(b"j", self.raw(1))
                    && (!self.in_(b"i", self.raw(1)) || !self.vowel(self.at(2)))
                {
                    return self.put(0x1952);
                }
                self.put_n(0x1001752, 1)
            }
            b'T' | b't' => self.sp_t(),
            b'U' | b'u' => self.sp_u(),
            b'V' | b'v' => {
                if self.stod() {
                    return self.put(0x80401ec2);
                }
                if self.p != 0 && self.in_(b"l", self.raw(-1)) && (self.p == self.len || self.hyphen(0)) {
                    return SKIP;
                }
                if self.p == 0 || (self.p == 1 && self.consonant(self.sp(0))) {
                    return self.put(0x1d42);
                }
                if !self.vowel(self.at(1)) && (self.p == 1 || !self.in_(b"ldr", self.raw(-1))) {
                    return self.put(0x401ec2);
                }
                self.put(0x1d42)
            }
            b'W' | b'w' => self.put(if self.stod() { 0x80401ec2 } else { 0x401ec2 }),
            b'X' | b'x' => {
                if self.p == 0 {
                    if self.len == 0 {
                        self.e(0x106c1);
                        self.n += 1;
                        return self.put2(0x20000ff2, 0x1952);
                    }
                    return self.put(0x1952);
                }
                if self.compound() {
                    return self.put(0x1952);
                }
                self.put2(0x20000ff2, 0x1952)
            }
            b'Y' | b'y' => self.sp_y(),
            b'Z' | b'z' => self.put(0x1952),
            0x80 => self.put(0x501),
            0xc0 | 0xe0 => self.put(0x10441),
            0xc3 | 0xe3 => self.put(if self.stressed(0) { 0x10441 } else { 0x441 }),
            0xc4 | 0xe4 => {
                if self.stressed(0) {
                    return self.put(if self.stod() { 0x80010685 } else { 0x106c1 });
                }
                self.put(if self.secondary() && self.stod() { 0x80000685 } else { 0x6c1 })
            }
            0xc5 | 0xe5 => {
                if !self.stressed(0) {
                    if !self.secondary() {
                        let long = self.p == 0 && self.len != 0 && self.consonant(self.sp(1)) && 1 < self.len && self.vowel(self.sp(2));
                        return self.put(if long { 0x2e5 } else { 0x2e1 });
                    }
                    return self.put(if self.stod() { 0x800002e5 } else { 0x2e1 });
                }
                if self.stod() || (!self.hyphen(0) && self.p == self.len) {
                    return self.put(0x800102e5);
                }
                if !self.hyphen(0) && self.consonant(self.at(1)) && self.vowel(self.at(2)) && !self.starts(1, b"re") {
                    return self.put(0x102e5);
                }
                self.put(0x102e1)
            }
            0xc6 | 0xe6 => self.sp_ae(),
            0xc7 | 0xe7 => self.put(0x1952),
            0xc8 | 0xc9 | 0xe8 | 0xe9 => self.put(if self.stod() { 0x80010745 } else { 0x10741 }),
            0xca | 0xcb | 0xea | 0xeb => self.put(if self.stressed(0) { 0x106c1 } else { 0x6c1 }),
            0xd1 | 0xf1 => self.put2(0x19c2, 0x10c2),
            0xd6 | 0xf6 => {
                if self.stressed(0) {
                    return self.put(if self.stod() { 0x800106e5 } else { 0x106e1 });
                }
                self.put(if self.secondary() && self.stod() { 0x800006e5 } else { 0x6e1 })
            }
            0xd8 | 0xf8 => self.sp_oe(),
            0xdb | 0xfb => self.put(if self.stressed(0) { 0x103e1 } else { 0x3e1 }),
            0xdc | 0xfc => self.put(if self.stressed(0) { 0x107e1 } else { 0x7e1 }),
            _ => SKIP,
        }
    }

    /// `a` before `r` + consonant (not `rr`).
    fn ar_cons(&self) -> bool {
        self.in_(b"r", self.raw(1)) && self.consonant(self.at(2)) && self.raw(2) != b'r'
    }

    fn sp_a(&mut self) -> Step {
        let r1 = self.in_(b"r", self.raw(1));
        if !self.stressed(0) {
            if !self.secondary() {
                if self.ar_cons() {
                    return self.put_n(0x245, 1);
                }
                if self.in_(b"a", self.raw(1)) {
                    return self.put_n(0x2e1, 1);
                }
                if (self.p == 0 || self.compound()) && self.in_(b"u", self.raw(1)) {
                    return self.put(0x241);
                }
                if self.starts(1, b"ng") {
                    return self.put(0x241);
                }
                if self.in_(b"e", self.raw(1)) {
                    return SKIP;
                }
                if self.in_(b"ij", self.raw(1)) {
                    return self.put(0x241);
                }
                return self.put(0x441);
            }
            if self.stod() {
                if r1 && ((self.p + 2 <= self.len && self.consonant(self.at(2))) || self.len <= self.p + 1) {
                    return self.put_n(0x80000245, 1);
                }
                return self.put(if r1 { 0x80000245 } else { 0x80000685 });
            }
            if self.ar_cons() {
                return self.put_skip(0x245, 1);
            }
            if self.in_(b"a", self.raw(1)) {
                return self.put_skip(0x2e1, 1);
            }
            if self.p == 0 && self.in_(b"fu", self.raw(1)) {
                return self.put(0x241);
            }
            if self.starts(1, b"ng") {
                return self.put(0x241);
            }
            if self.p != 0
                && self.in_(b"r", self.raw(-1))
                && self.consonant(self.at(1))
                && self.p + 2 <= self.len
                && self.vowel(self.at(2))
                && !self.starts(1, b"re")
            {
                return self.put(0x245);
            }
            if r1 && self.consonant(self.at(2)) {
                return self.put(0x10245);
            }
            return self.a_open(0x241, 0x685, 0x441);
        }
        if self.stod() {
            if r1 && ((self.p + 2 <= self.len && self.consonant(self.at(2))) || self.len <= self.p + 1) {
                return self.put_n(0x80010245, 1);
            }
            return self.put(if r1 { 0x80010245 } else { 0x80010685 });
        }
        if self.ar_cons() {
            return self.put_skip(0x10245, 1);
        }
        if self.in_(b"a", self.raw(1)) {
            return self.put_skip(0x102e1, 1);
        }
        if self.p == 0 && self.in_(b"fu", self.raw(1)) {
            return self.put(0x10241);
        }
        if self.starts(1, b"ng") {
            return self.put(0x10241);
        }
        if self.in_(b"e", self.raw(1)) {
            return self.put(0x10685);
        }
        if self.in_(b"ij", self.raw(1)) {
            return self.put(0x10241);
        }
        if self.in_(b"lns", self.raw(1)) && self.consonant(self.at(2)) {
            return self.put(0x10441);
        }
        if !(self.p == 0
            || !self.in_(b"r", self.raw(-1))
            || !self.consonant(self.at(1))
            || !self.vowel(self.at(2))
            || self.starts(1, b"re"))
        {
            return self.put(0x10245);
        }
        if r1 && self.consonant(self.at(2)) {
            return self.put(0x10245);
        }
        self.a_open(0x10241, 0x10685, 0x10441)
    }

    /// `a` before a consonant + vowel (long, `x[1]`), before a single
    /// final or hyphenated consonant (short `x[0]`), else `x[2]`.
    fn a_open(&mut self, short: u32, long: u32, other: u32) -> Step {
        let c1 = self.consonant(self.at(1));
        if !c1 || self.raw(1) == b'r' || !self.vowel(self.at(2)) {
            if !c1 || (self.p + 1 < self.len && !self.hyphen(1)) {
                return self.put(other);
            }
            return self.put(short);
        }
        self.put(long)
    }

    fn sp_d(&mut self) -> Step {
        if self.stod() {
            return self.put(0x80001b42);
        }
        if self.starts(1, b"de") && self.at(3) == 0 {
            self.put2(0x1b42, 0x1b42);
            self.n += 1;
            return self.put_n(0x501, 2);
        }
        if self.vowel(self.at(-1))
            && !self.hyphen(0)
            && self.consonant(self.at(1))
            && !self.in_(b"td", self.raw(1))
            && self.vowel(self.at(2))
        {
            return self.put(0x1b42);
        }
        // the silent d (0x10011934…0x10011b45)
        if self.p > 0 && self.in_(b"nl", self.raw(-1)) && (self.p == self.len || self.hyphen(0)) {
            return SKIP;
        }
        if !self.hyphen(0) && self.in_(b"dt", self.raw(1)) {
            return SKIP;
        }
        if self.p > 0 {
            let n = self.in_(b"n", self.raw(-1));
            if n && !self.hyphen(0) && self.in_(b"ln", self.raw(1)) {
                return SKIP;
            }
            if n && !self.compound() && (self.p == self.len || self.vowel(self.at(1))) {
                return SKIP;
            }
        }
        if self.consonant(self.at(-1)) && self.fl(0) & 0x30 == 0 && self.consonant(self.at(1)) {
            return SKIP;
        }
        if self.p + 1 >= self.len && self.p > 0 && self.in_(b"n", self.raw(-1)) && self.in_(b"e", self.raw(1)) {
            return SKIP;
        }
        if self.starts(1, b"st") {
            return SKIP;
        }
        if self.in_(b"ln", self.raw(1)) {
            return self.put(0x1b42);
        }
        if self.p != 0 {
            let nlr = self.in_(b"nlr", self.raw(-1));
            if nlr && self.in_(b"r", self.raw(1)) {
                return self.put(0x200019f2);
            }
            if nlr {
                let c = self.mo(0);
                if c != self.mo(1) && c >= 0 {
                    // `-ig` (0x10027188), `-isk` (0x10027184)
                    if let Some(m) = self.after(c).filter(|m| m.kind == DERIV) {
                        if m.text == b"ig" || m.text == b"isk" {
                            return self.put(0x200019f2);
                        }
                    }
                }
            }
            // 0x10011ced
            if nlr && !self.compound() && !self.in_(b"r", self.raw(1)) {
                let c = self.mo(0);
                if !(c != self.mo(1) && c >= 0 && self.suffix(c)) && !self.hyphen(0) {
                    return SKIP;
                }
            }
        }
        // 0x10011f1b
        if self.p == self.len
            && self.vowel(self.at(-1))
            && (self.stressed(-1) || self.is(-2, b'^') || self.f400(-1) || self.is(-2, b'"') || self.f200(-1))
        {
            return self.put(0x1b42);
        }
        self.put(0x200019f2)
    }
}

impl Danish {
    pub(super) fn tras_parola(&self, w: &[Word], i: usize) -> Vec<u32> {
        let Some(rec) = w.get(i) else { return Vec::new() };
        // 0x1000b650: the first phonetization row with a non-empty
        // PH_Phonetic
        if let Some(ph) = rec.rows.iter().filter_map(|r| r.get(1)?.as_deref()).find(|p| !p.is_empty()) {
            return self.parse_phonetic(ph);
        }
        let Some(li) = LexInfo::of(rec) else { return Vec::new() };
        let mut mem = [0u8; 0x154];
        for (k, t) in li.letters.iter().enumerate() {
            mem[4 * k] = t.morph as u8;
            mem[4 * k + 1] = t.b1;
            mem[4 * k + 2..4 * k + 4].copy_from_slice(&t.flags.to_le_bytes());
        }
        let n = li.spelling.len().min(0x294 - 0x250);
        mem[0x250 - 0x14c..0x250 - 0x14c + n].copy_from_slice(&li.spelling[..n]);
        // strlen - 1 (the last letter's index), at most 0x3f
        let n = li.spelling.iter().position(|&c| c == 0).unwrap_or(li.spelling.len()) as u32;
        let len = n.wrapping_sub(1).min(0x3f) as usize;
        let mut t = Tr {
            da: self,
            w,
            i,
            mem,
            len,
            sol: li.solution(),
            class: rec.f[0xe],
            info: rec.f[0x10],
            p: 0,
            out: Vec::new(),
            n: 0,
        };
        t.run();
        t.out
    }

    /// 0x1000bb50 with the phone table 0x10024de0: a phonetic string's
    /// symbols (up to 5 bytes, separated by ` ` or `-`) as codes (0x1000bac0,
    /// 0x10 for an unknown one).
    fn parse_phonetic(&self, s: &[u8]) -> Vec<u32> {
        let at = |i: usize| s.get(i).copied().unwrap_or(0);
        let mut out = Vec::new();
        let mut i = 0;
        while at(i) != 0 {
            let start = i;
            while !matches!(at(i), b' ' | b'-' | 0) && i - start < 5 {
                i += 1;
            }
            let sym = &s[start..i];
            if at(i) != 0 {
                i += 1;
            }
            out.push(self.code_of(sym));
        }
        out
    }

    /// 0x1000bac0 on the table at 0x10024de0: the first entry with the
    /// symbol; a code 0 counts only among the first four entries.
    fn code_of(&self, sym: &[u8]) -> u32 {
        if sym.is_empty() {
            return 0x10;
        }
        let ent = self.pe.u32(PHONES);
        let n = self.pe.u16(PHONES + 4) as u32;
        for k in 0..n {
            if self.s(self.pe.u32(ent + 8 * k)) == sym {
                let c = self.pe.u32(ent + 8 * k + 4);
                return if c != 0 || k <= 3 { c } else { 0x10 };
            }
        }
        0x10
    }
}

#[cfg(test)]
mod tests {
    use super::super::trace;
    use crate::lang::common::Word;

    #[test]
    fn tras_parola_trace() {
        let (Some(calls), Some(da)) = (trace::calls(), trace::module()) else { return };
        let (mut ok, mut all) = (0, 0);
        let mut bad = Vec::new();
        // the sentence: the records of the last LesAccenta call (0x1000b8b0
        // reads the next word)
        let mut sentence: Vec<Word> = Vec::new();
        for c in &calls {
            if c.name == "LesAccenta" {
                sentence = c.after.iter().map(|r| r.word()).collect();
            }
            if c.name != "FonTrasParola" {
                continue;
            }
            let rec = c.rec.as_ref().unwrap();
            let i = c.rec_index as usize;
            let mut w = sentence.clone();
            let blank = Word {
                text: Vec::new(),
                tag: None,
                f: [0; 24],
                data: Vec::new(),
                rows: Vec::new(),
            };
            if w.len() <= i {
                w.resize(i + 1, blank);
            }
            w[i] = rec.word();
            let got = da.tras_parola(&w, i);
            all += 1;
            if got == c.out {
                ok += 1;
            } else if bad.len() < 40 {
                let sp = rec.lexinfo.as_ref().map(|l| String::from_utf8_lossy(&l.spelling).into_owned());
                bad.push(format!("{:?} {:?} got {:x?} want {:x?}", String::from_utf8_lossy(&c.word), sp, got, c.out));
            }
        }
        for b in &bad {
            eprintln!("{b}");
        }
        eprintln!("tras_parola: {ok}/{all}");
        assert_eq!(ok, all);
    }
}

/// 0x1000b8b0: the first vowel within four letters after `pos` stressed;
/// at the word's end, when the word is a preposition (its first
/// morpheme's `POS` is `PREP`), the next word's first vowel.
fn vowel_ahead_stressed(
    da: &Danish,
    byte: &dyn Fn(usize) -> u8,
    stressed: &dyn Fn(usize) -> bool,
    pos: usize,
    sol: Option<&[Morpheme]>,
    w: &[Word],
    i: usize,
) -> bool {
    let mut k = 1;
    while byte(pos + k) != 0 {
        if da.s(VOWELS).contains(&byte(pos + k)) {
            break;
        }
        k += 1;
        if k > 4 {
            break;
        }
    }
    if byte(pos + k) != 0 {
        return stressed(pos + k);
    }
    let Some(m) = sol.and_then(|s| s.first()) else { return false };
    if m.feat == [0, 0] || da.attr_code(m.feat, da.s(S_POS)).and_then(|c| da.value_name(c)) != Some(da.s(S_PREP)) {
        return false;
    }
    let Some(next) = w.get(i + 1) else { return false };
    let li = LexInfo::of(next);
    let text = &next.text;
    let at = |q: usize| text.get(q).copied().unwrap_or(0);
    let st = |q: usize| match &li {
        Some(l) => l.letters.get(q).is_some_and(|t| t.flags & 0x100 != 0) || (q != 0 && at(q - 1) == b'^'),
        None => false,
    };
    vowel_ahead_stressed(da, &at, &st, 0, li.as_ref().and_then(|l| l.solution()), w, i + 1)
}

impl Tr<'_> {
    /// 0x1000b8b0 on this word from `pos`.
    fn ahead_stressed(&self, pos: usize) -> bool {
        vowel_ahead_stressed(
            self.da,
            &|q| self.sp(q as isize),
            &|q| self.stressed_at(q as isize),
            pos,
            self.sol,
            self.w,
            self.i,
        )
    }

    /// Morpheme `c` itself of kind `kind`.
    fn morph_is(&self, c: i8, kind: &[u8]) -> Option<&Morpheme> {
        let sol = self.sol?;
        if c < 0 || c as u32 >= sol.len() as u32 {
            return None;
        }
        sol.get(c as usize).filter(|m| m.kind == kind)
    }

    /// Anything stressing letter `p - 1`.
    fn prev_stressed(&self) -> bool {
        self.stressed(-1) || self.is(-2, b'^') || self.f400(-1) || self.is(-2, b'"') || self.f200(-1)
    }

    fn sp_e(&mut self) -> Step {
        if self.stressed(0) {
            if self.stod() {
                if !self.in_(b"e", self.raw(1)) {
                    return self.put(if self.in_(b"gj", self.raw(1)) { 0x80010245 } else { 0x80010745 });
                }
                return self.put_n(0x80010745, 1);
            }
            if self.in_(b"e", self.raw(1)) {
                return self.put_skip(0x10745, 1);
            }
            if self.p != 0 && self.in_(b"r", self.raw(-1)) {
                return self.put(0x10241);
            }
            if self.starts(1, b"nn") {
                return self.put(0x106c1);
            }
            if (!self.f800(1) && !self.is(2, b'?')) || !self.consonant(self.at(1)) || self.in_(b"gj", self.raw(1)) {
                if self.in_(b"gj", self.raw(1)) {
                    return self.put(0x10241);
                }
                if !self.consonant(self.at(1)) || !self.vowel(self.at(2)) || self.starts(1, b"re") {
                    return self.put(0x10741);
                }
                return self.put(0x10745);
            }
            return self.put(0x10741);
        }
        if self.secondary() {
            if self.stod() {
                return self.put(0x80000745);
            }
            if self.in_(b"e", self.raw(1)) {
                return self.put_skip(0x7c1, 1);
            }
            if self.in_(b"gj", self.raw(1)) {
                return self.put(0x241);
            }
            if self.starts(1, b"nn") {
                return self.put(0x6c1);
            }
            if !self.consonant(self.at(1)) || !self.vowel(self.at(2)) || self.starts(1, b"re") {
                if self.p == 0 || !self.in_(b"r", self.raw(-1)) {
                    return self.put(0x741);
                }
                return self.put(0x241);
            }
            return self.put(0x745);
        }
        // `be-` (0x1002720c) before a stressed vowel
        let c = self.mo(0);
        if self.morph_is(c, PREFIX).is_some_and(|m| m.text == b"be") && self.ahead_stressed(self.p) {
            return self.put(0x741);
        }
        if self.starts(1, b"rer") && self.at(4) == 0 {
            return self.put_n(0x481, 3);
        }
        let re = self.starts(1, b"re");
        if (self.class == 0x15 || self.class == 0x29) && re && self.ends(2) {
            return self.schwa_er();
        }
        if !re || self.ends(2) || self.hyphen(0) || self.hyphen(1) {
            if self.p == 3 {
                // `und` (0x10027204) + `r`
                if !(self.sp(0) == b'u' && self.sp(1) == b'n' && self.sp(2) == b'd') || !self.in_(b"r", self.sp(4)) {
                    return self.e_unstressed();
                }
                self.e(0x481);
                self.p = 4;
                return NEXT;
            }
            if self.p != 1 || self.sp(0) != b'b' || self.in_(b"a", self.sp(2)) || !self.ahead_stressed(1) {
                return self.e_unstressed();
            }
            return self.put(0x741);
        }
        self.put_n(0x481, 2)
    }

    /// 0x1000ca67: unstressed `e`.
    fn e_unstressed(&mut self) -> Step {
        if self.in_(b"r", self.raw(1)) && !self.hyphen(0) {
            return self.put_n(0x481, 1);
        }
        if self.in_(b"e", self.raw(1)) {
            return self.put_n(0x7c1, 1);
        }
        // the `-et` inflection (0x10026518): `ə` and `d`
        let c = self.mo(0);
        if self.morph_is(c, FLEX).is_some_and(|m| m.text == b"et") {
            self.e(0x501);
            self.n += 1;
            return self.put_n(0x1b42, 1);
        }
        if self.p == 0 && self.in_(b"u", self.sp(1)) {
            return self.put(0x6e1);
        }
        if self.starts(1, b"lig") {
            return SKIP;
        }
        if !self.vowel(self.at(1)) {
            if self.p == 0 {
                return self.put(0x741);
            }
            if self.p != 1 || !self.consonant(self.sp(0)) {
                return self.put(0x501);
            }
        }
        self.put(0x741)
    }

    fn sp_f(&mut self) -> Step {
        if self.p == 0 && self.starts(1, b"or") && 2 < self.len {
            let x = if !self.stressed_at(1) && self.sp(0) != b'^' { 0x2e1 } else { 0x102e5 };
            self.put2(0x1d52, x);
            self.p = 2;
            return NEXT;
        }
        // `af-` (0x10023f7c)
        if self.morph_is(self.mo(0), PREFIX).is_some_and(|m| m.text == b"af") {
            return self.put(0x401ec2);
        }
        if self.p == 1 && self.sp(0) == b'a' {
            return self.put(0x401ec2);
        }
        self.put(0x1d52)
    }

    fn sp_g(&mut self) -> Step {
        if self.stod() {
            return self.put(0x800010c2);
        }
        if self.p != 0 && !self.hyphen(0) && self.in_(b"l", self.raw(1)) {
            return SKIP;
        }
        if self.p > 1 {
            if self.starts(-2, b"li") && (self.p == self.len || self.hyphen(0) || self.before_suffix()) {
                return SKIP;
            }
            if self.starts(-2, b"si")
                && self.in_(b"e", self.raw(1))
                && !self.stressed(1)
                && !self.is(0, b'^')
                && (self.len <= self.p + 1 || self.hyphen(1))
            {
                return SKIP;
            }
        }
        let front = |t: &Self, c: u8| t.in_va(FRONT, c);
        if self.p == self.len && front(self, self.at(-1)) && (self.f800(-1) || self.raw(0) == b'?') {
            return SKIP;
        }
        if front(self, self.at(-1)) && (self.p == self.len || self.hyphen(0) || self.before_suffix()) {
            return SKIP;
        }
        if front(self, self.at(-1))
            && (self.p == 0
                || ((!self.in_(b"e", self.raw(-1)) || self.prev_stressed()) && (!self.in_(b"i", self.raw(-1)) || !self.in_(b"t", self.raw(1)))))
            && !self.consonant(self.at(1))
            && !self.compound()
        {
            return self.put(0x10c2);
        }
        if self.p != 0 && self.in_(b"l", self.raw(-1)) {
            return self.put(0x10c2);
        }
        if self.p != 0 {
            if self.in_(b"l", self.raw(1)) {
                return self.put(0x10c2);
            }
            if self.in_(b"u", self.raw(-1)) {
                return SKIP;
            }
        }
        if self.p == 2 && self.sp(0) == b'b' && self.sp(1) == b'a' && 2 < self.len && self.consonant(self.sp(3)) && self.flags_of(2) & 0x20 == 0 {
            return self.put(0x401ec2);
        }
        if self.p != 0 && self.in_(b"o", self.raw(-1)) && self.in_(b"n", self.raw(1)) {
            let c = self.mo(0);
            if self.len <= self.p + 1 || self.hyphen(1) || (self.mo(1) != self.mo(2) && c >= 0 && self.suffix(c)) {
                return self.put(0x401ec2);
            }
        }
        let fw = self.fl(0);
        if !self.in_va(BACK, self.at(-1)) || fw & 0x10 != 0 || (self.p != self.len && fw & 0x20 == 0 && !self.vowel(self.at(1))) {
            return self.put(0x20000ff2);
        }
        self.put(0x401ec2)
    }
}

impl Tr<'_> {
    /// `i` short before a double or clustered consonant.
    fn i_short(&self, gt: bool) -> bool {
        self.starts(1, b"pp")
            || self.starts(1, b"kk")
            || self.starts(1, b"gg")
            || (self.vowel(self.at(3))
                && (self.starts(1, b"st") || self.starts(1, b"sk") || self.starts(1, b"dt") || self.starts(1, b"ft") || (gt && self.starts(1, b"gt"))))
    }

    fn sp_i(&mut self) -> Step {
        let nm_cluster = self.in_(b"nm", self.raw(1)) && self.consonant(self.at(2));
        let r_nd = self.p != 0 && self.in_(b"r", self.raw(-1)) && (self.starts(1, b"nd") || self.starts(1, b"ng"));
        if !self.stressed(0) {
            if !self.secondary() {
                return self.put(if nm_cluster { 0x741 } else { 0x7c1 });
            }
            if self.stod() || self.p == self.len {
                return self.put(0x800007c5);
            }
            if self.hyphen(0) {
                return self.put(0x7c1);
            }
            if r_nd {
                return self.put(0x6c1);
            }
            if self.starts(1, b"nn") || self.starts(1, b"mm") || nm_cluster || self.i_short(true) {
                return self.put(0x741);
            }
            if self.in_(b"e", self.raw(1)) {
                return self.put(0x7c5);
            }
            return self.put(0x7c1);
        }
        if self.stod() || self.p == self.len {
            return self.put(0x800107c5);
        }
        if self.hyphen(0) {
            return self.put(0x107c1);
        }
        if r_nd {
            return self.put(0x106c1);
        }
        if self.starts(1, b"nn") || self.starts(1, b"mm") || nm_cluster || self.i_short(false) {
            return self.put(0x10741);
        }
        if self.in_(b"e", self.raw(1)) {
            return self.put(0x107c5);
        }
        let open = |t: &Self| !t.consonant(t.at(1)) || !t.vowel(t.at(2)) || t.starts(1, b"re");
        let gn = |t: &Self| t.starts(1, b"gn") && t.vowel(t.at(3));
        // 0x1000c65b / 0x1000c6dc / 0x1000c64b
        let long = if !self.f800(1) && !self.is(2, b'?') {
            !open(self) || gn(self)
        } else if self.len < self.p + 1 {
            gn(self)
        } else if !self.consonant(self.at(1)) {
            !open(self) || gn(self)
        } else {
            false
        };
        self.put(if long { 0x107c5 } else { 0x107c1 })
    }

    fn sp_o(&mut self) -> Step {
        let st = self.stressed(0);
        if !st && !self.secondary() {
            return self.put(0x361);
        }
        let s = if st { 0x10000 } else { 0 };
        if self.stod() || (st && self.p == self.len) {
            return self.put(0x80000365 | s);
        }
        if self.hyphen(0) {
            return self.put(0x361 | s);
        }
        if !(self.p + 1 < self.len || !self.in_(b"g", self.raw(1))) {
            return self.put(0x80000365 | s);
        }
        if self.starts(1, b"ge") && self.ends(2) {
            return self.put(0x2e5 | s);
        }
        if self.in_(b"g", self.raw(1)) || self.in_(b"v", self.raw(1)) {
            return self.put(0x2e1 | s);
        }
        if (!self.consonant(self.at(1)) || !self.vowel(self.at(2)) || self.starts(1, b"re")) && !self.in_(b"e", self.raw(1)) {
            if st && self.p == 0 && self.len < 2 && self.len != 0 && self.consonant(self.sp(1)) {
                return self.put(0x102c1);
            }
            if !self.consonant(self.at(1)) || !self.consonant(self.at(2)) {
                return self.put(0x361 | s);
            }
            return self.put(0x2c1 | s);
        }
        self.put(0x365 | s)
    }

    fn sp_r(&mut self) -> Step {
        if self.stod() {
            if self.starts(1, b"re") && self.ends(2) && !self.stressed(2) && !self.is(1, b'^') && !self.f400(2) && !self.is(1, b'"') && !self.f200(2) {
                self.put2(0x80000485, 0x481);
                self.p += 2;
                return NEXT;
            }
            if self.in_(b"r", self.raw(1)) {
                return self.put_n(0x80000485, 1);
            }
            if self.in_(b"e", self.raw(1)) && !self.next_stressed() && !self.hyphen(0) && !self.before_suffix() {
                return self.put_skip(0x80000485, 1);
            }
            return self.put(0x80000485);
        }
        if self.starts(1, b"e") && self.ends(1) && self.consonant(self.at(-1)) && !self.next_stressed() && !self.hyphen(0) {
            self.put2(0xd42, 0x481);
            self.p += 1;
            return NEXT;
        }
        if self.p == 0 || self.compound() || (self.consonant(self.at(-1)) && !self.in_(b"r", self.raw(-1))) {
            return self.put(0xd42);
        }
        if self.class == 0x2a && self.starts(1, b"et") && self.at(3) == 0 && !self.next_stressed() {
            self.put2(0x481, 0x501);
            self.n += 1;
            return self.put_n(0x1b42, 2);
        }
        if self.starts(1, b"rere") && self.ends(4) {
            return self.put_n(0x481, 4);
        }
        if (self.starts(1, b"ere") || self.starts(1, b"rer")) && self.ends(3) {
            return self.put_n(0x481, 3);
        }
        if self.starts(1, b"re") && self.ends(2) {
            if self.stressed(2) || self.is(1, b'^') {
                return self.put_skip(0x10481, 2);
            }
            if self.f400(2) || self.is(1, b'"') || self.f200(2) {
                return self.put_skip(0x481, 2);
            }
        }
        if self.class == 0x15 && matches!(self.info, 0x10 | 0xce | 0xcb | 0x0e | 0x0c) && self.starts(1, b"er") && self.at(3) == 0 {
            return self.schwa_er();
        }
        if !self.consonant(self.at(-1)) && self.starts(1, b"er") && self.ends(2) {
            return self.put_n(0x481, 2);
        }
        if self.starts(1, b"re") && self.ends(2) {
            self.put2(0xd42, 0x481);
            self.n += 1;
            self.p += 2;
            return SKIP;
        }
        if self.starts(1, b"er") && self.ends(2) {
            return self.put_n(0x481, 2);
        }
        if self.starts(1, b"e") && self.ends(1) {
            if self.stressed(1) || self.is(0, b'^') {
                return self.put_skip(0x10481, 1);
            }
            if self.f400(1) || self.is(0, b'"') || self.f200(1) {
                return self.put_skip(0x481, 1);
            }
        }
        if self.in_(b"e", self.raw(1)) && self.consonant(self.at(2)) && !(self.vowel(self.at(-1)) && (self.stressed(-1) || self.is(-2, b'^'))) {
            return self.put_skip(0x481, 1);
        }
        if self.vowel(self.at(-1)) && self.starts(1, b"e") && self.ends(1) {
            return self.put_skip(0x481, 1);
        }
        if self.in_(b"e", self.raw(1)) && !self.next_stressed() && !self.hyphen(0) && !self.before_suffix() {
            return self.put_skip(0x481, 1);
        }
        if self.in_(b"r", self.raw(1)) && !self.hyphen(0) {
            return self.put_skip(0xd42, 1);
        }
        let c = self.mo(0);
        if c != self.mo(1) && c >= 0 {
            if let Some(m) = self.after(c).filter(|m| m.kind == DERIV) {
                if m.text == b"inde" || m.text == b"agtig" {
                    return self.put(0x481);
                }
            }
        }
        if self.vowel(self.at(-1)) && self.consonant(self.at(1)) {
            return self.put(0x481);
        }
        self.r_final()
    }
}

impl Tr<'_> {
    fn sp_t(&mut self) -> Step {
        let ion = self.starts(1, b"ion");
        if ion && (self.stressed(1) || self.is(0, b'^')) {
            self.put2(0x1001752, 0x80010365);
            self.n += 1;
            return self.put_n(0x19c2, 3);
        }
        if ion && (self.f400(1) || self.is(0, b'"')) {
            self.put2(0x1001752, 0x80000365);
            self.n += 1;
            return self.put_n(0x19c2, 3);
        }
        let b = self.raw(1);
        if self.p == 0 {
            if self.in_(b"h", b) {
                self.e(0x39f2);
                self.p = 1;
                return NEXT;
            }
            return self.put(0x39f2);
        }
        if self.in_(b"i", b)
            && !self.stressed(1)
            && !self.is(0, b'^')
            && !self.f400(1)
            && !self.is(0, b'"')
            && !self.f200(1)
            && self.vowel(self.at(2))
        {
            return self.put_skip(0x1001752, 1);
        }
        if self.in_(b"n", self.raw(-1)) && !self.compound() && self.starts(1, b"lig") {
            let c = self.mo(0);
            if (self.mo(3) != self.mo(4) && c >= 0 && self.suffix(c)) || self.hyphen(3) || self.len <= self.p + 3 {
                return SKIP;
            }
        }
        if self.compound() {
            return self.put(0x39f2);
        }
        if !self.in_(b"s", self.raw(-1)) && self.vowel(self.at(1)) && self.next_stressed() {
            return self.put(0x39f2);
        }
        if self.in_(b"r", b) {
            return self.put(0x39f2);
        }
        self.put(0x200019f2)
    }

    fn sp_u(&mut self) -> Step {
        if !self.stressed(0) {
            if !self.secondary() {
                return self.put(if self.p == 1 && self.sp(0) == b'e' { 0x401ec2 } else { 0x3e1 });
            }
            if self.stod() {
                return self.put(0x800003e5);
            }
            if self.hyphen(0) {
                return self.put(0x3e1);
            }
            if self.in_(b"m", self.raw(1)) {
                let c = self.mo(0);
                if self.len <= self.p + 1 || self.hyphen(1) || (self.mo(1) != self.mo(2) && c >= 0 && self.suffix(c)) {
                    return self.put(0x2e1);
                }
            }
            if self.in_(b"mn", self.raw(1)) && self.consonant(self.at(2)) {
                return self.put(0x2e1);
            }
            let open_cv = |t: &Self| t.consonant(t.at(1)) && t.vowel(t.at(2)) && !t.starts(1, b"re");
            if !self.f800(1) && !self.is(2, b'?') {
                if open_cv(self) {
                    return self.put(0x3e5);
                }
            } else if self.p + 1 <= self.len {
                if self.consonant(self.at(1)) {
                    return self.put(0x3e1);
                }
                if open_cv(self) {
                    return self.put(0x3e5);
                }
            }
            if self.in_(b"e", self.raw(1)) {
                return self.put(0x3e5);
            }
            return self.put(0x3e1);
        }
        if self.stod() {
            return self.put(0x800103e5);
        }
        if self.p == 0 {
            if self.starts(1, b"dd") {
                self.e(0x103e1);
                self.n += 1;
                self.e(0x1b42);
                self.n += 1;
                self.e(0x200019f2);
                self.p = 2;
                return NEXT;
            }
            let d = self.in_(b"d", self.sp(1));
            if d && (self.flags_of(1) & 0x800 != 0 || (2 <= self.len && self.sp(2) == b'?')) {
                self.put2(0x103e1, 0x80001b42);
                self.p = 1;
                return NEXT;
            }
            if d && self.mo(0) == self.mo(1) {
                self.put2(0x103e1, 0x1b42);
                self.p = 1;
                return NEXT;
            }
        }
        // 0x1000eaff
        if self.hyphen(0) {
            return self.put(0x103e1);
        }
        if self.in_(b"m", self.raw(1)) {
            let c = self.mo(0);
            if self.len <= self.p + 1 || self.hyphen(1) || (self.mo(1) != self.mo(2) && c >= 0 && self.suffix(c)) {
                return self.put(0x102e1);
            }
        }
        if self.in_(b"mn", self.raw(1)) && self.consonant(self.at(2)) {
            return self.put(0x102e1);
        }
        let open_cv = |t: &Self| t.consonant(t.at(1)) && t.vowel(t.at(2)) && !t.starts(1, b"re");
        if self.f800(1) || self.is(2, b'?') {
            if self.p + 1 <= self.len && self.consonant(self.at(1)) {
                return self.put(0x103e1);
            }
            if self.p + 1 <= self.len && open_cv(self) {
                return self.put(0x103e5);
            }
        } else if open_cv(self) {
            return self.put(0x103e5);
        }
        // 0x1000eda7
        if self.in_(b"e", self.raw(1)) {
            return self.put(0x103e5);
        }
        if self.p == 0 && self.starts(1, b"nder") {
            return self.put(0x102e1);
        }
        self.put(0x103e1)
    }
}

impl Tr<'_> {
    /// `gt`, `kk` or a final `k` after the vowel.
    fn y_short(&self) -> bool {
        self.starts(1, b"gt") || self.starts(1, b"kk") || (self.starts(1, b"k") && self.ends(1))
    }
    /// Not `g` + vowel, nor a final `g`.
    fn y_no_g(&self) -> bool {
        (!self.in_(b"g", self.raw(1)) || !self.vowel(self.at(2))) && !(self.starts(1, b"g") && self.ends(1))
    }
    fn mn_cluster(&self) -> bool {
        self.in_(b"mn", self.raw(1)) && self.consonant(self.at(2))
    }

    fn sp_y(&mut self) -> Step {
        let open = |t: &Self| (t.consonant(t.at(1)) && t.vowel(t.at(2)) && !t.starts(1, b"re")) || t.in_(b"e", t.raw(1));
        if !self.stressed(0) {
            if !self.secondary() {
                let x = if self.p == 0 && self.len != 0 && self.vowel(self.sp(1)) { 0x10c2 } else { 0x7e1 };
                return self.put(x);
            }
            if self.stod() {
                if !self.mn_cluster() && !self.y_short() {
                    return self.put(0x800007e5);
                }
                return self.put(0x80000765);
            }
            if self.hyphen(0) {
                return self.put(0x7e1);
            }
            if open(self) || !self.y_no_g() {
                return self.put(0x7e5);
            }
            if self.starts(1, b"mf") || self.starts(1, b"nd") {
                return self.put(0x761);
            }
            return self.put(0x7e1);
        }
        if self.stod() {
            if !self.mn_cluster() && !self.y_short() {
                return self.put(0x800107e5);
            }
            return self.put(0x80010765);
        }
        if self.hyphen(0) {
            return self.put(0x107e1);
        }
        if open(self) || !self.y_no_g() {
            return self.put(0x107e5);
        }
        if self.mn_cluster() {
            return self.put(0x10761);
        }
        if self.y_short() {
            return self.put(0x761);
        }
        self.put(0x107e1)
    }

    fn sp_ae(&mut self) -> Step {
        let st = self.stressed(0);
        if !st && !self.secondary() {
            return self.put(0x6c1);
        }
        let s = if st { 0x10000 } else { 0 };
        if self.stod() || self.p == self.len {
            return self.put(0x800006c5 | s);
        }
        if self.hyphen(0) {
            return self.put(0x106c1);
        }
        let bl_t = |t: &Self, x: u32, y: u32, z: u32| {
            if t.starts(1, b"bl") {
                x
            } else if t.class != 0x2a || !t.consonant(t.at(1)) || !t.is(2, b't') {
                y
            } else {
                z
            }
        };
        if !st {
            // (0x1000d179…0x1000d208 test a stød on the next letter and
            // look the result up in the consonants: never taken)
            if (self.p == 0 || !self.in_(b"r", self.raw(-1)))
                && self.sp(self.p as isize) != 0
                && self.consonant(self.raw(1))
                && !self.in_(b"r", self.raw(1))
                && self.vowel(self.at(2))
            {
                return self.put(0x6c5);
            }
            if self.in_(b"r", self.raw(1)) && self.consonant(self.at(2)) {
                return self.put(0x681);
            }
            return self.put(bl_t(self, 0x6c5, 0x6c1, 0x441));
        }
        if (!self.f800(1) && !self.is(2, b'?')) || !self.consonant(self.at(1)) {
            if !self.in_(b"r", self.raw(1)) || !self.consonant(self.at(2)) {
                if (self.p != 0 && self.in_(b"r", self.raw(-1)))
                    || !self.consonant(self.at(1))
                    || self.raw(1) == b'r'
                    || !self.vowel(self.at(2))
                {
                    return self.put(bl_t(self, 0x106c5, 0x106c1, 0x10441));
                }
                return self.put(0x106c5);
            }
            return self.put(0x10681);
        }
        self.put(0x106c1)
    }

    fn sp_oe(&mut self) -> Step {
        let st = self.stressed(0);
        if !st && !self.secondary() {
            return self.put(0x761);
        }
        let s = if st { 0x10000 } else { 0 };
        if self.stod() {
            return self.put(0x80000765 | s);
        }
        if self.hyphen(0) {
            return self.put(0x761 | s);
        }
        if self.p == self.len {
            return self.put(0x80000765 | s);
        }
        let r1 = self.raw(1);
        if self.in_(b"e", r1) || self.in_(b"g", r1) {
            return self.put(0x765 | s);
        }
        if self.in_(b"n", r1) && self.vowel(self.at(2)) {
            return self.put(0x6e5 | s);
        }
        if self.in_(b"n", r1) {
            return self.put(if st { 0x106e1 } else { 0x761 });
        }
        if self.in_(b"mv", r1) && self.consonant(self.at(2)) && !self.in_(b"r", r1) {
            return self.put(0x6e1 | s);
        }
        if self.in_(b"j", r1) {
            return self.put(0x2c1 | s);
        }
        if !self.consonant(self.at(1)) || self.in_(b"n", r1) || !self.vowel(self.at(2)) {
            return self.put(0x761 | s);
        }
        self.put(0x765 | s)
    }
}
