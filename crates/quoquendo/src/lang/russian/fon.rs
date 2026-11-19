//! `FonTrasParola` (0x1000a620): a word's phones, letter by letter from
//! the spelling LesAccenta stressed (LexInfo +0x10) and its stressed vowel
//! (LexInfo +0xc), each letter by its neighbours (the previous and next
//! letters skip a `^` mark) and, for the vowels before the stress, by
//! whether the next syllable is the stressed one (0x1000a350, which looks
//! into the next word after a preposition).

use super::{Lex, Russian, PUNCT};
use crate::lang::common::Word;
use crate::lang::wordanalyzer::morph;

/// The letter sets the rules test (strings of the module).
mod set {
    /// `уаоэыиеяюёьъ`
    pub const VOWELS: u32 = 0x1009f760;
    /// `ьъЬЪ`
    pub const SIGNS: u32 = 0x1009f758;
    /// `цкнгшщзхфвпрлджчсмтбй`
    pub const CONSONANTS: u32 = 0x1009f7dc;
    /// `яюеёиь`: a soft consonant before them
    pub const SOFTENING: u32 = 0x1009f7d0;
    /// `цкшщхфпчст`: voiceless
    pub const VOICELESS: u32 = 0x1009f7bc;
    /// `бгджз`: voiced
    pub const VOICED: u32 = 0x1009f7c8;
    /// `жшц`: always hard
    pub const HARD: u32 = 0x1009f7f4;
    pub const CH_SHCH: u32 = 0x1009f800;
    pub const TILDE: u32 = 0x1009f7fc;
    pub const SOFT_SIGN: u32 = 0x1001b9d0;
    pub const O: u32 = 0x1001d274;
    pub const N: u32 = 0x1001d334;
    pub const S: u32 = 0x1001cb9c;
    pub const T: u32 = 0x1001cb84;
    pub const D: u32 = 0x10020254;
    pub const Z: u32 = 0x1001f72c;
    pub const ZH: u32 = 0x1001f730;
    pub const CH: u32 = 0x1001bce4;
    pub const SH: u32 = 0x1001bcdc;
    pub const SHCH: u32 = 0x1001bcd8;
    pub const TS: u32 = 0x1001bd24;
    pub const I: u32 = 0x1001f724;
    pub const V: u32 = 0x100204e4;
    /// `ое`
    pub const OE: u32 = 0x10066e2c;
    /// Latin `oe`, `o` and `p` (0x1009f7a4, 0x1009deb8, 0x1009de98)
    pub const LATIN_OE: u32 = 0x1009f7a4;
    pub const LATIN_O: u32 = 0x1009deb8;
    pub const LATIN_P: u32 = 0x1009de98;
    /// `кч`, `рн`, `чцж`, `чц`, `нлс`, `жзсш`, `нгзврлджмб`, `нмнлр`
    pub const K_CH: u32 = 0x1009f7ac;
    pub const R_N: u32 = 0x1009f7a8;
    pub const CH_TS_ZH: u32 = 0x1009f7b0;
    pub const CH_TS: u32 = 0x1009f7b4;
    pub const N_L_S: u32 = 0x1009f7b8;
    pub const ZH_Z_S_SH: u32 = 0x1009f790;
    pub const VOICED_SONORANT: u32 = 0x1009f77c;
    pub const SONORANT: u32 = 0x1009f774;
}

/// Strings the rules compare the letters after or before with.
mod seq {
    /// `ств`, `ск`, `ося`, `ей`, `нц`, `тч`, `дч`, `под`, `ься`, `ьд`,
    /// `то`, `зд`, `ст`, `лу`, `~^`
    pub const STV: u32 = 0x100658b4;
    pub const SK: u32 = 0x1001cb90;
    pub const OSYA: u32 = 0x1009f7a0;
    pub const EY: u32 = 0x1009f7f8;
    pub const NTS: u32 = 0x1009f770;
    pub const TCH: u32 = 0x1009f79c;
    pub const DCH: u32 = 0x1009f798;
    pub const POD: u32 = 0x10067c54;
    pub const SYA: u32 = 0x1001b5e4;
    pub const SD: u32 = 0x1001b9c8;
    pub const TO: u32 = 0x1001ca28;
    pub const ZD: u32 = 0x1009f78c;
    pub const ST: u32 = 0x1009f788;
    pub const LU: u32 = 0x1001df6c;
    pub const TILDE_CARET: u32 = 0x1009f7d8;
}

/// One word's letters as FonTrasParola walks them.
struct Walk<'a> {
    l: &'a Russian,
    f: &'a [u8],
    i: i32,
    /// the index of the last letter (at most 0x3f)
    last: i32,
    pos: i32,
}

impl Walk<'_> {
    fn c(&self, k: i32) -> u8 {
        if k < 0 {
            0
        } else {
            self.f.get(k as usize).copied().unwrap_or(0)
        }
    }
    fn prev(&self) -> u8 {
        if self.i == 0 {
            return 0;
        }
        if self.c(self.i - 1) == b'^' {
            if self.i > 1 {
                self.c(self.i - 2)
            } else {
                0
            }
        } else {
            self.c(self.i - 1)
        }
    }
    fn next(&self) -> u8 {
        if self.c(self.i + 1) == b'^' {
            self.c(self.i + 2)
        } else {
            self.c(self.i + 1)
        }
    }
    fn in_set(&self, c: u8, set: u32) -> bool {
        c != 0 && self.l.s(set).contains(&c)
    }
    /// the previous letter is one of `set`
    fn p(&self, set: u32) -> bool {
        self.in_set(self.prev(), set)
    }
    /// the next letter is one of `set`
    fn n(&self, set: u32) -> bool {
        self.in_set(self.next(), set)
    }
    /// strncmp(&form[at], s, n) == 0
    fn at(&self, at: i32, s: u32) -> bool {
        let s = self.l.s(s);
        (0..s.len()).all(|k| self.c(at + k as i32) == s[k])
    }
    /// the vowel carries the stress
    fn stressed(&self) -> bool {
        self.i == self.pos || (self.i >= 1 && self.c(self.i - 1) == b'^')
    }
    /// `local_70 <= i + 1`: the last letter or the one before
    fn near_end(&self) -> bool {
        self.last <= self.i + 1
    }
    fn is_last(&self) -> bool {
        self.i == self.last
    }
}

impl Russian {
    /// 0x1000a350: whether the syllable after letter `i` of word
    /// `sentence[k]` (spelled `text`) is the stressed one: the next vowel
    /// within four letters is the stressed vowel, follows a `^` or is `ё`;
    /// at the end of a preposition (part of speech `7`), the next word's
    /// first syllable decides.
    fn next_stressed(&self, sentence: &[Word], k: usize, text: &[u8], i: i32) -> bool {
        let c = |j: i32| if j < 0 { 0 } else { text.get(j as usize).copied().unwrap_or(0) };
        let mut n = 1;
        while c(i + n) != 0 {
            let v = c(i + n);
            if self.s(set::VOWELS).contains(&v) && !self.s(set::SIGNS).contains(&v) {
                break;
            }
            n += 1;
            if n > 4 {
                break;
            }
        }
        let j = i + n;
        let lex = Lex::of(&sentence[k]);
        if c(j) != 0 {
            return j == lex.pos || c(j - 1) == b'^' || c(j) == 0xb8;
        }
        let Some(an) = lex.current().filter(|a| !a.is_empty()) else { return false };
        if self.feature(morph(&self.pe, an[0].morph).id(), 6) != Some(b'7') {
            return false;
        }
        let Some(next) = sentence.get(k + 1) else { return false };
        let t = &next.text;
        if t.first().is_none_or(|&f| !self.s(set::VOWELS).contains(&f)) {
            return self.next_stressed(sentence, k + 1, t, 0);
        }
        !(lex.pos != 0 && t[0] != b'^' && t[0] != 0xb8)
    }

    /// `FonTrasParola` (0x1000a620): the phone codes of
    /// `sentence[first + i]`, the `i`-th word of a chunk of `chunk_len`
    /// records starting at `first`.
    pub(super) fn tras_parola(&self, sentence: &[Word], first: usize, chunk_len: usize, i: usize) -> Vec<u32> {
        let k = first + i;
        let lex = Lex::of(&sentence[k]);
        let f = &lex.form;
        let mut w = Walk {
            l: self,
            f,
            i: 0,
            last: (f.len() as i32 - 1).min(0x3f),
            pos: lex.pos,
        };
        // LAB_1000a7ec: the last letter of one of the chunk's last two
        // words
        let final_ = |w: &Walk| w.is_last() && chunk_len <= i + 2;
        let mut out: Vec<u32> = Vec::new();
        while w.i <= w.last {
            let c = w.c(w.i);
            if c == 0 {
                break;
            }
            let o: Option<Vec<u32>> = if self.has(c, PUNCT) {
                match c {
                    b'"' | b'\'' => None,
                    b'#' | b',' => Some(vec![2]),
                    _ => Some(vec![3]),
                }
            } else {
                match c {
                    // ё
                    0xa8 | 0xb8 => Some(if w.p(set::CONSONANTS) { vec![0x10361] } else { vec![0x10c2, 0x10361] }),
                    // а
                    0xc0 | 0xe0 => Some(if w.stressed() {
                        vec![0x10441]
                    } else if w.p(set::CH_SHCH) && self.next_stressed(sentence, k, f, w.i) {
                        vec![0x781]
                    } else if w.i != 0 && !self.next_stressed(sentence, k, f, w.i) {
                        if w.p(set::TILDE) {
                            vec![0x441]
                        } else if final_(&w) {
                            vec![0x2c1]
                        } else {
                            vec![0x501]
                        }
                    } else {
                        vec![0x2c1]
                    }),
                    // б
                    0xc1 | 0xe1 => Some(if w.near_end() && w.n(set::SOFT_SIGN) {
                        vec![0x201ff2]
                    } else if w.n(set::SOFTENING) {
                        vec![0x201fe2]
                    } else if !w.n(set::VOICELESS) && !w.is_last() {
                        vec![0x1fe2]
                    } else {
                        vec![0x1ff2]
                    }),
                    // в
                    0xc2 | 0xe2 => {
                        if w.at(w.i + 1, seq::STV) {
                            None
                        } else if w.n(set::SOFT_SIGN) && w.near_end() {
                            Some(vec![0x201d52])
                        } else if w.n(set::SOFTENING) {
                            Some(vec![0x201d42])
                        } else if !w.n(set::VOICELESS) && !w.is_last() {
                            Some(vec![0x1d42])
                        } else {
                            Some(vec![0x1d52])
                        }
                    }
                    // г
                    0xc3 | 0xe3 => self.g(&w),
                    // д
                    0xc4 | 0xe4 => {
                        if w.p(set::Z) && w.n(set::N) {
                            None
                        } else if w.i != 0 && w.p(set::LATIN_P) && w.n(set::CH_TS) {
                            None
                        } else if w.n(set::S) || w.n(set::CH_TS_ZH) {
                            None
                        } else if w.n(set::Z) {
                            Some(vec![0x1902])
                        } else if w.near_end() && w.n(set::SOFT_SIGN) {
                            Some(vec![0x2019f2])
                        } else if w.n(set::SOFTENING) {
                            Some(vec![0x2019e2])
                        } else if !w.n(set::VOICELESS) && !w.is_last() {
                            Some(vec![0x19e2])
                        } else {
                            Some(vec![0x19f2])
                        }
                    }
                    // е
                    0xc5 | 0xe5 => Some(if w.i >= 2 && w.at(w.i - 2, seq::TILDE_CARET) {
                        vec![0x106c1]
                    } else if w.stressed() {
                        if w.i != 0 && w.p(set::TILDE) {
                            vec![0x106c1]
                        } else if w.p(set::CONSONANTS) {
                            vec![0x10741]
                        } else {
                            vec![0x10c2, 0x10741]
                        }
                    } else if w.i == 0 {
                        vec![0x10c2, 0x781]
                    } else if w.p(set::VOWELS) {
                        vec![0x10c2, 0x501]
                    } else if w.p(set::HARD) {
                        vec![0x5c1]
                    } else if w.p(set::TILDE) {
                        vec![0x6c1]
                    } else {
                        vec![0x781]
                    }),
                    // ж
                    0xc6 | 0xe6 => Some(if w.n(set::CH) {
                        vec![0x201752]
                    } else if w.i != 0 && w.p(set::D) && w.is_last() {
                        vec![0x1712]
                    } else if w.i != 0 && w.p(set::D) {
                        vec![0x1702]
                    } else if w.n(set::VOICELESS) || w.is_last() {
                        vec![0x1752]
                    } else if w.near_end() && w.n(set::SOFT_SIGN) {
                        vec![0x201752]
                    } else {
                        vec![0x1742]
                    }),
                    // з
                    0xc7 | 0xe7 => {
                        if w.n(set::SHCH) || (w.i != 0 && w.p(set::D)) {
                            None
                        } else if w.n(set::SH) {
                            Some(vec![0x1752])
                        } else if w.n(set::ZH) {
                            Some(vec![0x201742])
                        } else if w.n(set::CH) || w.at(w.i + 1, seq::DCH) {
                            Some(vec![0x201752])
                        } else if w.n(set::SOFT_SIGN) && w.near_end() {
                            Some(vec![0x201952])
                        } else if w.n(set::VOICELESS) || w.is_last() {
                            Some(vec![0x1952])
                        } else if w.n(set::SOFTENING) {
                            Some(vec![0x201942])
                        } else {
                            Some(vec![0x1942])
                        }
                    }
                    // и
                    0xc8 | 0xe8 => {
                        if w.stressed() {
                            Some(if w.i != 0 && w.p(set::HARD) {
                                vec![0x105c1]
                            } else if w.i != 0 && w.p(set::SOFT_SIGN) {
                                vec![0x10c2, 0x107c1]
                            } else {
                                vec![0x107c1]
                            })
                        } else if w.at(w.i + 1, seq::EY) {
                            None
                        } else if w.i != 0 && w.p(set::N) && w.i + 1 <= w.last && w.in_set(w.c(w.i + 1), set::VOWELS) {
                            None
                        } else if w.i != 0 && w.p(set::I) && (w.last < w.i - 2 || w.i < 2 || !w.in_set(w.c(w.i - 2), set::HARD)) {
                            Some(vec![0x7c1])
                        } else if w.i != 0 && w.p(set::HARD) {
                            Some(vec![0x5c1])
                        } else if w.i != 0 && w.p(set::SOFT_SIGN) {
                            Some(vec![0x10c2, 0x7c1])
                        } else {
                            Some(vec![0x781])
                        }
                    }
                    // й
                    0xc9 | 0xe9 => Some(vec![0x10c2]),
                    // к
                    0xca | 0xea => Some(if w.n(set::SOFTENING) {
                        vec![0x200ff2]
                    } else if w.n(set::VOICED) {
                        vec![0xfe2]
                    } else {
                        vec![0xff2]
                    }),
                    // л
                    0xcb | 0xeb => {
                        if w.at(w.i + 1, seq::NTS) {
                            None
                        } else if w.n(set::SOFTENING) {
                            Some(vec![0x2018a2])
                        } else {
                            Some(vec![0x4018a2])
                        }
                    }
                    // м
                    0xcc | 0xec => Some(if w.n(set::SOFTENING) { vec![0x201fc2] } else { vec![0x1fc2] }),
                    // н
                    0xcd | 0xed => Some(if w.n(set::SOFTENING) { vec![0x2019c2] } else { vec![0x19c2] }),
                    // о
                    0xce | 0xee => {
                        if w.stressed() {
                            if w.i != 0 && w.p(set::SOFT_SIGN) {
                                Some(vec![0x10c2, 0x10361])
                            } else if w.n(set::O) {
                                out.extend([0x10361, 0x501]);
                                w.i += 2;
                                continue;
                            } else {
                                Some(vec![0x10361])
                            }
                        } else if w.i != 0 && w.p(set::TILDE) {
                            Some(vec![0x361])
                        } else if w.n(set::O) && w.i + 1 == w.pos {
                            out.extend([0x2c1, 0x10361]);
                            w.i += 2;
                            continue;
                        } else if w.n(set::O) {
                            out.extend([0x2c1, 0x2c1]);
                            w.i += 2;
                            continue;
                        } else if w.i == 0 || self.next_stressed(sentence, k, f, w.i) {
                            Some(vec![0x2c1])
                        } else if final_(&w) {
                            Some(vec![0x2c1])
                        } else {
                            Some(vec![0x501])
                        }
                    }
                    // п
                    0xcf | 0xef => Some(if w.n(set::SOFTENING) {
                        vec![0x201ff2]
                    } else if w.n(set::VOICED) {
                        vec![0x1fe2]
                    } else {
                        vec![0x1ff2]
                    }),
                    // р
                    0xd0 | 0xf0 => Some(if w.n(set::SOFTENING) { vec![0x2019a2] } else { vec![0x19a2] }),
                    // с
                    0xd1 | 0xf1 => Some(self.s_(&w)),
                    // т
                    0xd2 | 0xf2 => {
                        if w.at(w.i + 1, seq::SYA) {
                            out.push(0x1912);
                            w.i += 3;
                            continue;
                        } else if w.at(w.i + 1, seq::SD) {
                            out.push(0x2019e2);
                            w.i += 3;
                            continue;
                        } else if w.i != 0 && w.p(set::N) && w.at(w.i + 1, seq::STV) {
                            None
                        } else if w.i != 0 && w.p(set::S) && w.n(set::N_L_S) && (w.last < w.i + 2 || w.c(w.i + 2) != 0xff) {
                            None
                        } else if w.n(set::S) || w.n(set::CH_TS) {
                            None
                        } else if w.n(set::SOFTENING) {
                            Some(vec![0x2019f2])
                        } else if w.n(set::VOICED) {
                            Some(vec![0x19e2])
                        } else {
                            Some(vec![0x19f2])
                        }
                    }
                    // у
                    0xd3 | 0xf3 => Some(if w.stressed() { vec![0x103e1] } else { vec![0x3e1] }),
                    // ф
                    0xd4 | 0xf4 => Some(if w.n(set::SOFTENING) {
                        vec![0x201d52]
                    } else if w.n(set::VOICED) {
                        vec![0x1d42]
                    } else {
                        vec![0x1d52]
                    }),
                    // х
                    0xd5 | 0xf5 => Some(if w.n(set::SOFTENING) { vec![0x200f52] } else { vec![0xf52] }),
                    // ц
                    0xd6 | 0xf6 => {
                        if w.n(set::TS) {
                            None
                        } else if w.n(set::VOICED_SONORANT) && !w.n(set::SONORANT) && !w.n(set::V) {
                            Some(vec![0x1902])
                        } else {
                            Some(vec![0x1912])
                        }
                    }
                    // ч
                    0xd7 | 0xf7 => {
                        if w.i == 0 && w.at(1, seq::TO) && (w.last < 3 || w.c(3) == 0) {
                            out.extend([0x1752, 0x19f2, 0x10361]);
                            w.i = 3;
                            continue;
                        }
                        if w.i != 0 && w.p(set::ZH_Z_S_SH) {
                            None
                        } else if w.i != 0 && w.p(set::TILDE) {
                            Some(vec![0x201712])
                        } else if w.i > 1 && (w.at(w.i - 2, seq::ZD) || w.at(w.i - 2, seq::ST)) {
                            None
                        } else if w.n(set::VOICED) {
                            Some(vec![0x201702])
                        } else if w.i > 1 && w.at(w.i - 2, seq::LU) && w.n(set::SH) {
                            Some(vec![0x19f2])
                        } else {
                            Some(vec![0x201712])
                        }
                    }
                    // ш
                    0xd8 | 0xf8 => Some(if w.n(set::VOICED) {
                        vec![0x1742]
                    } else if w.n(set::CH) {
                        vec![0x201752]
                    } else {
                        vec![0x1752]
                    }),
                    // щ
                    0xd9 | 0xf9 => Some(vec![0x201752]),
                    // ы
                    0xdb | 0xfb => Some(if w.stressed() { vec![0x105c1] } else { vec![0x5c1] }),
                    // э
                    0xdd | 0xfd => Some(if w.stressed() {
                        vec![0x106c1]
                    } else if i == 0 && w.i == 0 {
                        vec![0x6c1]
                    } else {
                        vec![0x5c1]
                    }),
                    // ю
                    0xde | 0xfe => Some(match (w.stressed(), w.p(set::CONSONANTS)) {
                        (true, true) => vec![0x103e1],
                        (true, false) => vec![0x10c2, 0x103e1],
                        (false, true) => vec![0x3e1],
                        (false, false) => vec![0x10c2, 0x3e1],
                    }),
                    // я
                    0xdf | 0xff => Some(if w.stressed() {
                        if w.p(set::CONSONANTS) {
                            vec![0x10441]
                        } else {
                            vec![0x10c2, 0x10441]
                        }
                    } else if w.i == 0 {
                        vec![0x10c2, 0x781]
                    } else if w.is_last() && (w.p(set::S) || w.p(set::CONSONANTS)) {
                        vec![0x501]
                    } else if w.p(set::CONSONANTS) {
                        vec![0x781]
                    } else if final_(&w) {
                        vec![0x10c2, 0x2c1]
                    } else {
                        vec![0x10c2, 0x501]
                    }),
                    _ => None,
                }
            };
            if let Some(o) = o {
                out.extend(o);
            }
            w.i += 1;
        }
        out
    }

    /// FonTrasParola's г (0x1000a620, cases 0xc3/0xe3).
    fn g(&self, w: &Walk) -> Option<Vec<u32>> {
        if w.p(set::TILDE) {
            return Some(vec![0xfe2]);
        }
        if w.n(set::SOFTENING) {
            return Some(vec![0x200fe2]);
        }
        if w.n(set::K_CH) {
            return Some(vec![0xf52]);
        }
        if w.p(set::R_N) && w.at(w.i + 1, seq::SK) {
            return None;
        }
        if w.n(set::VOICELESS) || w.is_last() {
            return Some(vec![0xff2]);
        }
        if w.p(set::OE) && w.at(w.i + 1, seq::OSYA) && (w.last < w.i + 4 || w.c(w.i + 4) == 0) {
            return Some(vec![0x1d42]);
        }
        if w.i + 1 >= w.last && w.p(set::OE) && w.n(set::O) {
            return Some(vec![0x1d42]);
        }
        if w.p(set::LATIN_OE) && w.n(set::LATIN_O) && w.near_end() {
            return Some(vec![0x1d42]);
        }
        Some(vec![0xfe2])
    }

    /// FonTrasParola's с (0x1000a620, cases 0xd1/0xf1).
    fn s_(&self, w: &Walk) -> Vec<u32> {
        if w.i != 0 && w.p(set::TILDE) {
            return vec![0x1952];
        }
        if w.n(set::CH_SHCH) || w.at(w.i + 1, seq::TCH) {
            return vec![0x201752];
        }
        if w.n(set::ZH) {
            return vec![0x1742];
        }
        if w.n(set::SH) {
            return vec![0x1752];
        }
        let before = w.i < 2 || (w.c(w.i - 2) != 0xf1 && w.c(w.i - 2) != 0xed);
        if before && w.i != 0 && (w.p(set::D) || w.p(set::T)) {
            return vec![0x1912];
        }
        if w.n(set::T) && w.i > 2 && w.at(w.i - 3, seq::POD) {
            return vec![0x1952];
        }
        if w.n(set::VOICED) {
            return vec![0x1942];
        }
        if w.n(set::SOFTENING) {
            vec![0x201952]
        } else {
            vec![0x1952]
        }
    }
}
