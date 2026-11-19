//! FonTrasParola (0x10008420): one word's phone codes.

use super::super::common::{field::*, Word};
use super::Dutch;

/// The words FonTrasParola reads by itself (0x10008420).
const HET: u32 = 0x10014644;
const VAN: u32 = 0x10012894;
const EEN: u32 = 0x1001473c;
const EENS: u32 = 0x10014734;
/// the letter names `f` and `m` (LesGetSymbolName's `eff`, `emm`)
const EFF: u32 = 0x10013ea0;
const EMM: u32 = 0x10013e84;

/// The boundaries after which `het`, `van`, `een`, `eens` keep their full
/// vowel (and the grade-2 parts of a compound their stress): the pauses
/// and sentence ends.
fn strong(b: u8) -> bool {
    matches!(b, 0x0a | 0x08 | 0x0e | 0x0c | 0x0d | 0x0b | 0x0f..=0x14)
}

/// 0x10007fe0: every stressed vowel unstressed (those in its list:
/// 0x106e5 and 0x107e5 are not).
fn destress(codes: &mut [u32]) {
    for c in codes.iter_mut() {
        *c = match *c {
            0x10761 | 0x103e1 | 0x102e1 | 0x10241 | 0x102c1 | 0x102e5 | 0x10361 | 0x106c5 | 0x10441 | 0x106c1 | 0x10741 | 0x107e1 | 0x10781 | 0x107a1 | 0x107c1 | 0x107c5 => *c & 0xffff,
            0x1f6c5 => 0xf6c5,
            0x1ba45 => 0xba45,
            0x1bae5 => 0xbae5,
            0x1fee5 => 0xfee5,
            x => x,
        };
    }
}

/// 0x100081e0: a vowel stressed (`e`/schwa as stressed `E`).
fn stress(c: &mut u32) {
    *c = match *c {
        0x741 | 0x3e1 | 0x2e1 | 0x241 | 0x2c1 | 0x2e5 | 0x361 | 0x6c1 | 0x441 | 0x6c5 | 0x6e5 | 0x7e1 | 0x761 | 0x781 | 0x7a1 | 0x7c1 | 0x7c5 | 0xf6c5 | 0xba45 | 0xbae5 | 0xfee5 => *c | 0x10000,
        0x501 => 0x106c1,
        x => x,
    };
}

impl Dutch {
    /// FonTrasParola (0x10008420) for the chunk's record `i`: a dictionary
    /// row (`ELQDBTableFind` on `grafemica`, exactly one) with a non-empty
    /// `fonetica` gives its symbols (0x10007f10); `het`, `van`, `een`,
    /// `eens` have their own codes (weak but after a pause or sentence end,
    /// `het` as `Ou t` first in a chunk), `eff`/`emm` unstressed `E f`/`E m`;
    /// otherwise the word without accents on a/i/o/u/y (0x10007da0) through
    /// the letter-to-sound model, then one stressed vowel: the first one
    /// when there are none (unless the grade is 2 or 3) or several; grade 3
    /// none, grade 2 none unless after a pause or sentence end.
    pub(super) fn tras_parola(&self, w: &[Word], i: usize) -> Vec<u32> {
        let r = &w[i];
        let word = r.text.as_slice();
        let (bound, grade) = (r.f[BOUND], r.f[GRADE]);
        let (row, count) = self.table_find(word);
        if count == 1 {
            if let Some(f) = &self.dict[row].1 {
                if !f.is_empty() {
                    return self.fonetica(f);
                }
            }
        }
        if word == self.s(HET) {
            return vec![
                if strong(bound) {
                    0x106c1
                } else if i != 0 {
                    0x501
                } else {
                    0x7a1
                },
                0x19f2,
            ];
        }
        if word == self.s(VAN) {
            return vec![0x1d42, if strong(bound) { 0x10241 } else { 0x501 }, 0x19c2];
        }
        if word == self.s(EEN) {
            return vec![if strong(bound) { 0x10741 } else { 0x501 }, 0x19c2];
        }
        if word == self.s(EENS) {
            let v = if strong(bound) {
                0x10741
            } else if i == 0 {
                0x741
            } else {
                0x501
            };
            return vec![v, 0x19c2, 0x1952];
        }
        if word == self.s(EFF) && grade == 3 {
            return vec![0x6c1, 0x1d52];
        }
        if word == self.s(EMM) && grade == 3 {
            return vec![0x6c1, 0x1fc2];
        }
        let plain: Vec<u8> = word.iter().map(|&c| if self.has(c, super::ACCENTED) { self.deaccent(c) } else { c }).collect();
        let mut out = match &self.gpr {
            Some(g) => g.transcribe(&plain),
            None => Vec::new(),
        };
        if let Some(z) = out.iter().position(|&c| c == 0) {
            out.truncate(z);
        }
        let stressed = out.iter().filter(|&&c| c & 3 == 1 && c & 0x10003 == 0x10001).count();
        if stressed > 1 {
            destress(&mut out);
        }
        if stressed > 1 || (stressed == 0 && grade != 2 && grade != 3) {
            if let Some(c) = out.iter_mut().find(|c| **c & 3 == 1) {
                stress(c);
            }
        }
        if (grade == 2 && !strong(bound)) || grade == 3 {
            destress(&mut out);
        }
        out
    }

    /// 0x10001900: à á → a, ì í → i, ò ó → o, ù ú → u, ý → y (capitals
    /// through the table's lower and upper case); others unchanged.
    pub(super) fn deaccent(&self, c: u8) -> u8 {
        if c == 0 {
            return 0;
        }
        let up = self.has(c, super::UPPER);
        let l = if up { self.lower(c) } else { c };
        let m = match l {
            0xe0 | 0xe1 => b'a',
            0xec | 0xed => b'i',
            0xf2 | 0xf3 => b'o',
            0xf9 | 0xfa => b'u',
            0xfd => b'y',
            x => x,
        };
        if up {
            self.upper(m)
        } else {
            m
        }
    }

    /// 0x10007f10: a `fonetica` string's symbols (split at ` ` and `-`, at
    /// most 5 characters each: a longer one loses its sixth) as codes
    /// (0x10007e80; 0x10 for an unknown one).
    fn fonetica(&self, f: &[u8]) -> Vec<u32> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < f.len() {
            let mut sym = Vec::new();
            while i < f.len() && f[i] != b' ' && f[i] != b'-' && sym.len() < 5 {
                sym.push(f[i]);
                i += 1;
            }
            if i < f.len() {
                i += 1;
            }
            out.push(self.phone_code(&sym));
        }
        // the codes end at a 0 (the first table entries' code)
        if let Some(z) = out.iter().position(|&c| c == 0) {
            out.truncate(z);
        }
        out
    }
}
