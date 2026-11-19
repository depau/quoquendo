//! `FonTrasParola` (0x1000f730): one word's phone codes. A word with a
//! `fonetica` in its dictionary rows is read from it (0x1000e5c0);
//! otherwise its letters, lowered through the character table
//! (0x1000e4f0), go through one rule block per letter, then 0x1000f180
//! puts the stress on.
//!
//! Positions are as the original keeps them: `l` is the index of the
//! last letter (the code's "length" is strlen − 1), so `i == l` is the
//! last letter and `i == l - 1 && w[l] == 's'` the one before a final s.

use super::{col, French, Lex};
use crate::lang::common::{field::*, Word};

/// Voiceless consonants (0x1003f464) and the vowels that soften c/g
/// (0x1003f234).
const VOICELESS: u32 = 0x1003f464;
const FRONT: u32 = 0x1003f234;
/// Voiced consonants, a string in the code.
const VOICED: &[u8] = b"bdglmnrwvjz";

// phone codes
const A: u32 = 0x441;
const AA: u32 = 0x241;
const E_: u32 = 0x741; // e
const EH: u32 = 0x6c1; // E
const I_: u32 = 0x7c1;
const O_: u32 = 0x361;
const OA: u32 = 0x2e1;
const OE_: u32 = 0x761;
const OEH: u32 = 0x6e1;
const Y_: u32 = 0x7e1;
const U_: u32 = 0x3e1;
const EN: u32 = 0x100006c1; // E)
const OEN: u32 = 0x100006e1; // Oe)
const AN: u32 = 0x10000241; // Aa)
const ON: u32 = 0x10000361; // o)
const SCHWA: u32 = 0x501;
const W_: u32 = 0x401ec2;
const HY: u32 = 0x201ec2;
const J_: u32 = 0x10c2;
const P_: u32 = 0x1ff2;
const T_: u32 = 0x1bf2;
const K_: u32 = 0xff2;
const B_: u32 = 0x1fe2;
const D_: u32 = 0x1be2;
const G_: u32 = 0xfe2;
const F_: u32 = 0x1d52;
const V_: u32 = 0x1d42;
const S_: u32 = 0x1952;
const Z_: u32 = 0x1942;
const SH: u32 = 0x1752;
const ZH: u32 = 0x1742;
const R_: u32 = 0xda2;
const M_: u32 = 0x1fc2;
const N_: u32 = 0x19c2;
const GN: u32 = 0x11c2;
const L_: u32 = 0x18a2;

/// A boundary with a pause after it (8, 0x0a–0x14).
fn pause_after(b: u8) -> bool {
    b == 8 || (0x0a..=0x14).contains(&b)
}

/// The phone is a vowel (the feature word's low bits 1, not the mute e).
fn is_vowel_code(c: u32) -> bool {
    c & 3 == 1 && c & 0x1c3 != 0x101
}

/// The phone is a consonant (low bits 2) and not a glide (0xc2).
fn is_cons_code(c: u32) -> bool {
    c & 3 == 2 && c & 0x1e3 != 0xc2
}

struct Ctx<'a> {
    fr: &'a French,
    w: Vec<u8>,
    /// index of the last letter (−1 for an empty word)
    l: isize,
}

impl Ctx<'_> {
    /// The letter at `k` (0 outside the word).
    fn g(&self, k: isize) -> u8 {
        super::b(&self.w, k)
    }
    fn vow(&self, c: u8) -> bool {
        self.fr.is_vowel(c)
    }
    /// `strchr(set, c)` for a letter that is not 0.
    fn voiceless(&self, c: u8) -> bool {
        c != 0 && self.fr.s(VOICELESS).contains(&c)
    }
    fn voiced(&self, c: u8) -> bool {
        c != 0 && VOICED.contains(&c)
    }
    fn front(&self, c: u8) -> bool {
        c != 0 && self.fr.s(FRONT).contains(&c)
    }
    /// A consonant of either list.
    fn cons(&self, c: u8) -> bool {
        self.voiceless(c) || self.voiced(c)
    }
    /// `strncmp(w + k, s, strlen(s)) == 0`.
    fn st(&self, k: isize, s: &[u8]) -> bool {
        s.iter().enumerate().all(|(j, &c)| self.g(k + j as isize) == c)
    }
    /// 0x1000e2f0: `pat` lies in the word with its first occurrence of the
    /// letter at `i` at `i` (no occurrence: `pat` ends right before `i`).
    fn at(&self, i: isize, pat: &[u8]) -> bool {
        let plen = pat.len() as isize;
        let mut k = 0isize;
        if plen != 0 {
            while k < plen {
                if pat[k as usize] == self.g(i) {
                    break;
                }
                k += 1;
            }
            if i < k {
                return false;
            }
        }
        let mut j = 0isize;
        while j < plen {
            if self.g(i - k + j) != pat[j as usize] {
                break;
            }
            j += 1;
        }
        j == plen
    }
    fn ats(&self, i: isize, pats: &[&[u8]]) -> bool {
        pats.iter().any(|p| self.at(i, p))
    }
    /// The word ends at `k`, or at `k + 1` with an s.
    fn end(&self, k: isize) -> bool {
        k == self.l || (k == self.l - 1 && self.g(k + 1) == b's')
    }
    /// 0x1000e350: the letters before `pos` (inclusive) are a learned
    /// prefix (bio-, anti-, para-, auto-, archi-, électro-…).
    fn prefix(&self, pos: isize) -> bool {
        let s = |a| self.fr.s(a);
        match pos {
            2 => self.g(2) == b'o' && self.at(2, s(0x1003edd8)),
            3 => {
                let c = self.g(3);
                (c == b'i' && (self.at(3, s(0x1003edd0)) || self.at(3, s(0x1003edc8))))
                    || (c == b'a' && self.at(3, s(0x1003edc0)))
                    || (c == b'o' && (self.at(3, s(0x1003edb8)) || self.at(3, s(0x1003edb0)) || self.at(3, s(0x1003eda8))))
            }
            4 => {
                let c = self.g(4);
                (c == b'o' && (self.at(4, s(0x1003eda0)) || self.at(4, s(0x1003ed98)) || self.at(4, s(0x1003ed90))))
                    || (c == b'i' && (self.at(4, s(0x1003ed88)) || self.at(4, s(0x1003ed80))))
            }
            6 => self.g(6) == b'o' && (self.at(6, s(0x1003ed78)) || self.st(0, s(0x1003ed70))),
            _ => false,
        }
    }
}

impl French {
    /// Not ported yet beyond the dictionary pronunciation: the letter rules
    /// (0x1000f730 switch, 0x1000f180 stress) are in progress, so the module
    /// is not wired into `lang::load`.
    pub(crate) fn tras_parola(&self, words: &[Word], idx: usize) -> Vec<u32> {
        let mut lex = Lex::of(&words[idx]);
        if self.has_col(&mut lex, col::FONETICA) {
            if let Some(row) = lex.current() {
                return self.parse_phones(self.col(row, col::FONETICA).unwrap_or(&[]));
            }
        }
        Vec::new()
    }
}
