//! FonTrasParola (0x1000c0d0) and FonTrasConfini (0x1000e8f0) of
//! LTTS7Portuguese.dll: letters → phone codes, then the changes across a
//! word boundary.

use super::super::common::{at, field::*, Phone, Word};
use super::{scan, Portuguese, FONETICA, VOWEL};

/// Letter sets FonTrasParola tests with `strchr`.
const FRONT: u32 = 0x1002c3bc; // eiyèéìíýêî
const VOICELESS: u32 = 0x1002c450; // cfkpqçsßtxz
const VOICED: u32 = 0x1002c444; // bdglmnrwv
const ACCENTED_VOWELS: u32 = 0x1002c3a0; // àáèéìíòóùúýâêîôû
/// 0x1000c040: what an unstressed word (grade 3) loses besides the
/// acute/circumflex accents.
const GRAVES: u32 = 0x1002c348; // àèìòù

/// Whole-word pronunciations FonTrasParola tries first and the word starts
/// it tries next (`{word, phones}` pairs): Portugal 0x100296a8 (0x13) and
/// 0x100299b0 (0x48a), Brazil 0x10029740 (0x19) and 0x10029808 (0x35).
const WORDS: [(u32, u32); 2] = [(0x100296a8, 0x13), (0x10029740, 0x19)];
const STARTS: [(u32, u32); 2] = [(0x100299b0, 0x48a), (0x10029808, 0x35)];

/// The word lists FonTrasParola tests (`ELQSearchString` mode 0x12: the
/// word starts with an entry).
const E_I: (u32, u32) = (0x1002be98, 0x17);
const E_OPEN: (u32, u32) = (0x1002be00, 0x26);
const E_CLOSED: (u32, u32) = (0x1002bf20, 0x3a);
const O_CLOSED: (u32, u32) = (0x1002c008, 8);
const O_OPEN: (u32, u32) = (0x1002c028, 0x15);
const O_U: (u32, u32) = (0x1002c07c, 4);
const X_S: (u32, u32) = (0x1002c100, 6);
const X_Z: (u32, u32) = (0x1002c118, 0x4d);
const X_SH: (u32, u32) = (0x1002c250, 0x20);
/// Patterns 0x1000bea0 aligns on the current letter: (table, count).
const X_KS: (u32, u32) = (0x1002c2d0, 0x1e);
const GU_SAID: (u32, u32) = (0x1002bef4, 0xb);
const QU_SAID: (u32, u32) = (0x1002c090, 0x1c);
/// Brazilian `dezasseis`, `dezassete`, `dezanove` (their a is e).
const DEZA: [u32; 3] = [0x1001dc54, 0x1001c5b4, 0x1001c5a0];

/// Phone codes the module writes.
mod ph {
    pub const A: u32 = 0x441;
    pub const A_: u32 = 0x10441;
    pub const AR: u32 = 0x481;
    pub const AR_: u32 = 0x10481;
    pub const E: u32 = 0x741;
    pub const E_: u32 = 0x10741;
    pub const EE_: u32 = 0x106c1;
    pub const EE: u32 = 0x6c1;
    pub const HEH: u32 = 0x501;
    pub const I: u32 = 0x7c1;
    pub const I_: u32 = 0x107c1;
    pub const O: u32 = 0x361;
    pub const O_: u32 = 0x10361;
    pub const OA: u32 = 0x2e1;
    pub const OA_: u32 = 0x102e1;
    pub const U: u32 = 0x3e1;
    pub const U_: u32 = 0x103e1;
    pub const J: u32 = 0x10c2;
    pub const W: u32 = 0x401ec2;
    pub const P: u32 = 0x1ff2;
    pub const T: u32 = 0x1bf2;
    pub const K: u32 = 0xff2;
    pub const B: u32 = 0x1fe2;
    pub const D: u32 = 0x1be2;
    pub const G: u32 = 0xfe2;
    pub const TSH: u32 = 0x1712;
    pub const DG: u32 = 0x1702;
    pub const F: u32 = 0x1d52;
    pub const V: u32 = 0x1d42;
    pub const S: u32 = 0x1952;
    pub const Z: u32 = 0x1942;
    pub const SH: u32 = 0x1752;
    pub const ZG: u32 = 0x1742;
    pub const M: u32 = 0x1fc2;
    pub const N: u32 = 0x19c2;
    pub const NG: u32 = 0xfc2;
    pub const NH: u32 = 0x11c2;
    pub const L: u32 = 0x18a2;
    pub const LDARK: u32 = 0x4018a2;
    pub const LH: u32 = 0x10a2;
    pub const R: u32 = 0x1b82;
    pub const RR: u32 = 0xd42;
}

/// A vowel's nasal form: bit 28.
const NASAL: u32 = 0x10000000;

impl Portuguese {
    fn among(&self, set: u32, c: u8) -> bool {
        c != 0 && self.s(set).contains(&c)
    }

    /// 0x1000bf70 with 0x1000bee0: a pronunciation (symbols separated by
    /// spaces or `-`, at most 5 characters each; a longer one loses its
    /// 6th) as the variant's phone codes; an unknown symbol is 0x10, a
    /// codeless one among the first four 0.
    pub(super) fn pronunciation(&self, s: &[u8]) -> Vec<u32> {
        let mut out = Vec::new();
        let mut k = 0usize;
        while at(s, k) != 0 {
            let mut sym = Vec::new();
            while !matches!(at(s, k), b' ' | b'-' | 0) && sym.len() < 5 {
                sym.push(s[k]);
                k += 1;
            }
            if at(s, k) != 0 {
                k += 1;
            }
            let code = if sym.is_empty() {
                0x10
            } else {
                match self.phones.iter().position(|(p, _)| *p == sym) {
                    Some(idx) => match self.phones[idx].1 {
                        0 if idx > 3 => 0x10,
                        c => c,
                    },
                    None => 0x10,
                }
            };
            out.push(code);
        }
        out
    }

    /// 0x1000be00: `pattern` occurs in `w` with the first of its letters
    /// equal to `w[pos]` at `pos` (letters compared without their accents).
    fn aligned(&self, w: &[u8], pos: usize, pattern: &[u8]) -> bool {
        let k = pattern.iter().position(|&c| c == at(w, pos)).unwrap_or(pattern.len());
        if pos < k {
            return false;
        }
        let start = pos - k;
        (0..pattern.len()).all(|j| self.plain(at(w, start + j)) == pattern[j])
    }

    /// 0x1000bea0: one of a table's patterns is `aligned` at `pos`.
    fn aligned_any(&self, w: &[u8], pos: usize, (table, count): (u32, u32)) -> bool {
        (0..count).any(|i| self.aligned(w, pos, self.s(self.pe.u32(table + 4 * i))))
    }

    /// `ELQSearchString(word, table, 0x12, count)`: the word starts with one
    /// of the table's entries.
    fn starts_any(&self, w: &[u8], (table, count): (u32, u32)) -> bool {
        self.search(w, table, 0x12, count).is_some()
    }

    /// FonTrasParola: the word's dictionary pronunciation when one of its
    /// rows has one; else the module's own word list, or a word start's
    /// pronunciation and the rest letter by letter; else every letter in
    /// its context. `idx` is the word's index in its chunk.
    pub(super) fn tras_parola(&self, r: &Word, idx: usize) -> Vec<u32> {
        if let (true, Some(row)) = scan(&r.rows, FONETICA) {
            return self.pronunciation(super::col(row, FONETICA).unwrap_or(b""));
        }
        // 0x1000c040: an unstressed word loses its accents
        let mut w: Vec<u8> = r.text.clone();
        if r.f[GRADE] == 3 {
            for c in w.iter_mut() {
                if self.flags(*c) & super::ACCENTED != 0 || self.among(GRAVES, *c) {
                    *c = self.plain(*c);
                }
            }
        }
        let v = self.variant as usize;
        let entry = |t: u32, i: u32| (self.s(self.pe.u32(t + 8 * i)), self.s(self.pe.u32(t + 8 * i + 4)));
        let (wt, wn) = WORDS[v];
        if let Some(i) = (0..wn).find(|&i| entry(wt, i).0 == w.as_slice()) {
            return self.pronunciation(entry(wt, i).1);
        }
        let (st, sn) = STARTS[v];
        let mut out = Vec::new();
        let mut i = 0usize;
        if let Some(k) = (0..sn).find(|&k| w.starts_with(entry(st, k).0)) {
            let (start, phones) = entry(st, k);
            out = self.pronunciation(phones);
            i = start.len();
        }
        while at(&w, i) != 0 {
            i = self.letter(&w, i, idx, &mut out);
            i += 1;
        }
        out
    }

    /// One character of FonTrasParola's loop (the switch at 0x1000c4e2,
    /// punctuation 0x1000c2cc): its codes, and the position of the last
    /// character it used.
    fn letter(&self, w: &[u8], i: usize, idx: usize, out: &mut Vec<u32>) -> usize {
        use ph::*;
        let v = self.variant;
        let b = |k: isize| -> u8 { if k < 0 { 0 } else { at(w, k as usize) } };
        let ii = i as isize;
        let c = w[i];
        let c1 = b(ii + 1);
        let c2 = b(ii + 2);
        let p1 = b(ii - 1);
        let p2 = b(ii - 2);
        // the last index
        let last = w.len() as isize - 1;
        let vowel = |c: u8| self.flags(c) & VOWEL != 0;
        let front = |c: u8| self.among(FRONT, c);
        let unvoiced = |c: u8| self.among(VOICELESS, c);
        let voiced = |c: u8| self.among(VOICED, c);
        let cons = |c: u8| unvoiced(c) || voiced(c);
        // m/n closing the syllable: a nasal vowel
        let m_coda = c1 == b'm' && !vowel(c2) && c2 != b'h' && c2 != b'n';
        let n_coda = c1 == b'n' && !vowel(c2) && c2 != b'h' && c2 != b'm';
        let nasal_coda = m_coda || n_coda;
        let at_end = |k: isize| ii == last + k;
        let after_s = out.last() == Some(&S);
        let mut push = |c: &[u32]| out.extend_from_slice(c);
        if self.flags(c) & super::PUNCT != 0 {
            match c {
                b'"' | b'\'' => {}
                b'#' | b',' => push(&[0x13]),
                _ => push(&[0x17]),
            }
            return i;
        }
        // the stressed and unstressed nasal a: Brazil's variants are a
        let nas_a_ = if v != 0 { 0x10010441 } else { 0x10010481 };
        let nas_a = if v != 0 { 0x10000441 } else { 0x10000481 };
        match c {
            0xe1 => push(&[A_]),
            0xe2 => {
                if nasal_coda {
                    push(&[nas_a_]);
                    return i + 1;
                } else if c1 == b'n' || c1 == b'm' {
                    // 0x1000c5a1: `-0x40 & 0x0fffffc0` (sic)
                    push(&[if v != 0 { 0x0fffffc0u32.wrapping_add(0x10481) } else { 0x10481 }]);
                } else {
                    push(&[A_]);
                }
            }
            0xe0 => match c1 {
                b'e' => {
                    push(&[nas_a_, J]);
                    return i + 1;
                }
                b'o' => {
                    push(&[nas_a_, W]);
                    return i + 1;
                }
                _ => push(&[nas_a_]),
            },
            0xe3 => match c1 {
                b'e' => {
                    push(&[nas_a, J]);
                    return i + 1;
                }
                b'o' => {
                    push(&[nas_a, W]);
                    return i + 1;
                }
                _ => push(&[nas_a]),
            },
            0xe5 => push(&[A]),
            0xe4 | 0xe6 | b'a' => {
                if v == 1 && DEZA.iter().any(|&s| self.aligned(w, i, self.s(s))) {
                    push(&[E]);
                } else if c1 == b'm' && at_end(-1) {
                    push(&[nas_a, W]);
                    return i + 1;
                } else if nasal_coda {
                    push(&[nas_a]);
                    return i + 1;
                } else if v != 0
                    || (c1 == b'l' && (cons(c2) || at_end(-1)))
                    || c1 == b'u'
                    || c1 == b'i'
                    || (c1 == b'r' && at_end(-1))
                    || (c1 == b'c' && c2 == b't')
                    || (self.among(0x1002c440, c1) && self.among(0x1002c43c, c2))
                {
                    push(&[A]);
                } else {
                    push(&[AR]);
                }
            }
            0xe8 | 0xe9 => {
                if v == 0 && i == 1 && last == 2 && w[0] == b't' && w[2] == b'm' {
                    push(&[AR_, J]);
                    return 2;
                } else if (c1 == b'm' || c1 == b'n') && at_end(-1) {
                    push(&[if v != 0 { 0x10010741 } else { 0x10010481 }, J]);
                    return i + 1;
                } else if nasal_coda {
                    push(&[0x10010741]);
                    return i + 1;
                } else {
                    push(&[EE_]);
                }
            }
            0xea => {
                if v == 0 {
                    if w == b"\xeax" || (ii + 2 <= last && c1 == b'x' && cons(c2)) {
                        push(&[AR_, J]);
                        return i;
                    }
                    if i == 1 && last == 2 && w[0] == b't' && w[2] == b'm' {
                        push(&[0x10010481]);
                        if idx == 0 {
                            push(&[J, 0x10000481, J]);
                        } else {
                            push(&[0x100007c1]);
                        }
                        return 2;
                    }
                }
                if (c1 == b'm' || c1 == b'n') && at_end(-1) {
                    push(&[if v != 0 { 0x10010741 } else { 0x10010481 }, J]);
                    return i + 1;
                } else if v == 0 && c1 == b'i' {
                    push(&[AR_, J]);
                    return i + 1;
                } else if nasal_coda {
                    push(&[0x10010741]);
                    return i + 1;
                } else {
                    push(&[E_]);
                }
            }
            0xeb | b'e' => return self.letter_e(w, i, idx, out),
            b'y' => push(&[I]),
            0xec | 0xed | 0xee => {
                if nasal_coda {
                    push(&[0x100107c1]);
                    return i + 1;
                }
                push(&[I_]);
            }
            0xfd => push(&[I_]),
            0xef | 0xff | b'i' => {
                if nasal_coda {
                    push(&[0x100007c1]);
                    return i + 1;
                } else if i > 1 && p1 == b'u' && (p2 == b'q' || p2 == b'g') && !vowel(c1) {
                    push(&[I]);
                } else if i > 0 && vowel(p1) && p1 != b'i' && !vowel(c1) {
                    push(&[J]);
                } else if !vowel(c1) || c1 == b'i' {
                    push(&[I]);
                } else {
                    push(&[J]);
                }
            }
            0xf3 => {
                if nasal_coda {
                    push(&[0x10010361]);
                    return i + 1;
                } else if c1 == b'u' {
                    push(&[O_]);
                } else {
                    push(&[OA_]);
                }
            }
            0xf4 => {
                if c1 != b'u' && nasal_coda {
                    push(&[0x10010361]);
                    return i + 1;
                }
                push(&[O_]);
            }
            0xf2 => {
                push(&[0x10010361]);
                if c1 == b'e' {
                    push(&[J]);
                    return i + 1;
                }
            }
            0xf6 | 0xf8 | b'o' => return self.letter_o(w, i, out),
            0xf5 => {
                push(&[0x10000361]);
                if c1 == b'e' {
                    push(&[J]);
                    return i + 1;
                }
            }
            0xf9 | 0xfa | 0xfb => {
                if nasal_coda {
                    push(&[0x100103e1]);
                    return i + 1;
                }
                push(&[U_]);
            }
            0xfc | b'u' => {
                if nasal_coda {
                    push(&[0x100003e1]);
                    return i + 1;
                }
                if v == 0 && i > 0 && matches!(p1, b'o' | 0xf3 | 0xf4) {
                    return i;
                }
                if i > 0 && vowel(p1) && p1 != b'u' && p1 != b'i' {
                    push(&[W]);
                } else if !vowel(c1) || c1 == b'u' || c1 == b'i' {
                    push(&[U]);
                } else {
                    push(&[W]);
                }
            }
            b'b' => {
                if c1 != b'b' {
                    push(&[B]);
                }
            }
            b'c' => {
                if v == 0 {
                    if i > 1 && p2 == b's' && p1 == b'e' && c1 == b'c' && front(c2) {
                        push(&[K]);
                        return i;
                    }
                    if i > 0 && matches!(p1, b'a' | 0xe1 | 0xe2 | b'e' | 0xe9 | 0xea) && matches!(c1, b't' | b'c' | 0xe7) {
                        return i;
                    }
                }
                if c1 == b'c' || (front(c1) && after_s) {
                    return i;
                }
                if front(c1) {
                    push(&[S]);
                } else if c1 == b'h' {
                    push(&[SH]);
                    return i + 1;
                } else {
                    push(&[K]);
                }
            }
            0xdf | 0xe7 => push(&[S]),
            0xf0 | b'd' => {
                if c1 == c {
                    return i;
                }
                if v == 0 {
                    push(&[D]);
                } else if matches!(c1, b'e' | 0xea | 0xe9) && (at_end(-1) || (at_end(-2) && c2 == b's') || &w[(i + 2).min(w.len())..] == self.s(0x1002c3b4)) {
                    push(&[DG, if self.among(ACCENTED_VOWELS, c1) { I_ } else { I }]);
                    return i + 1;
                } else if c1 == b'i' || c1 == 0xed || at_end(0) || (cons(c1) && c1 != b'r' && c1 != b'l') {
                    push(&[DG]);
                } else {
                    push(&[D]);
                }
            }
            b'f' => {
                if c1 != b'f' {
                    push(&[F]);
                }
            }
            b'g' => {
                if c1 != b'g' {
                    push(&[if front(c1) { ZG } else { G }]);
                }
                if c1 == b'u' && front(c2) && !self.aligned_any(w, i, GU_SAID) {
                    return i + 1;
                }
            }
            b'l' => {
                if c1 == b'l' {
                    return i;
                }
                if at_end(0) || cons(c1) {
                    push(&[if v != 0 { W } else { LDARK }]);
                } else if c1 == b'h' {
                    push(&[LH]);
                    return i + 1;
                } else {
                    push(&[L]);
                }
            }
            b'k' => {
                if c1 != b'k' {
                    push(&[K]);
                }
            }
            b'm' => {
                if c1 != b'm' {
                    push(&[if at_end(0) { N } else { M }]);
                }
            }
            0xf1 | b'n' => {
                if c1 == c {
                    return i;
                }
                if c1 == b'h' {
                    push(&[NH]);
                    return i + 1;
                }
                push(&[N]);
            }
            b'p' => {
                if c1 == b'p' {
                    return i;
                }
                if v == 0 && i > 0 {
                    let e = matches!(p1, b'e' | 0xe9 | 0xea);
                    if e && (c1 == b'c' || c1 == 0xe7) {
                        return i;
                    }
                    let o = matches!(p1, b'o' | 0xf3 | 0xf4);
                    let said = if o && !(i > 1 && p2 == b'c') {
                        false
                    } else if e {
                        (i > 1 && matches!(p2, b'p' | b's' | b'r')) || (i > 2 && p2 == b'l' && b(ii - 3) == b'c')
                    } else {
                        true
                    };
                    if !said && c1 == b't' {
                        return i;
                    }
                }
                push(&[P]);
            }
            b'q' => {
                if c1 == b'q' {
                    return i;
                }
                if c1 == b'u' && front(c2) {
                    push(&[K]);
                    if !self.aligned_any(w, i, QU_SAID) {
                        return i + 1;
                    }
                } else if c1 == b'u' && vowel(c2) {
                    push(&[K, W]);
                    return i + 1;
                } else {
                    push(&[K]);
                }
            }
            b'r' => {
                if i == 0 {
                    push(&[RR]);
                } else if c1 == b'r' {
                    push(&[RR]);
                    return i + 1;
                } else if (v == 0 || v == 1) && matches!(p1, b'm' | b'n' | b's' | b'l') {
                    push(&[RR]);
                } else {
                    push(&[R]);
                }
            }
            b's' => {
                if c1 == b's' {
                    return i;
                }
                if i > 0 && vowel(p1) && vowel(c1) {
                    push(&[Z]);
                } else if c1 == b'h' {
                    push(&[SH]);
                    return i + 1;
                } else if (c1 == b'c' && front(c2)) || c1 == 0xe7 {
                    if v == 0 {
                        push(&[SH]);
                    }
                    push(&[S]);
                    return i + 1;
                } else if after_s {
                } else if v != 0 {
                    push(&[S]);
                } else if at_end(0) || unvoiced(c1) {
                    push(&[SH]);
                } else if voiced(c1) {
                    push(&[ZG]);
                } else {
                    push(&[S]);
                }
            }
            b'j' => push(&[ZG]),
            b't' => {
                if c1 == b't' {
                    return i;
                }
                if v == 1 {
                    let rest = &w[i..];
                    let te = c1 == b'e' && (at_end(-1) || (at_end(-2) && c2 == b's') || &w[(i + 2).min(w.len())..] == self.s(0x1002c3b4));
                    if te || [0x1002c390, 0x10029300, 0x100292d4, 0x100292ec].iter().any(|&s| rest == self.s(s)) {
                        push(&[TSH, I]);
                        return i + 1;
                    }
                    if c1 == b'i' || c1 == 0xed || at_end(0) || (cons(c1) && c1 != b'r' && c1 != b'l') {
                        push(&[TSH]);
                    } else {
                        push(&[T]);
                    }
                } else {
                    push(&[T]);
                }
            }
            b'v' | b'w' => {
                if c1 != c {
                    push(&[V]);
                }
            }
            b'x' => return self.letter_x(w, i, out),
            b'z' => {
                if c1 == b'z' {
                    return i;
                }
                if v == 0 {
                    if at_end(0) || unvoiced(c1) {
                        push(&[SH]);
                    } else if voiced(c1) {
                        push(&[ZG]);
                    } else {
                        push(&[Z]);
                    }
                } else if at_end(0) {
                    push(&[S]);
                } else {
                    push(&[Z]);
                }
            }
            // h, a space, and the characters the module doesn't know
            _ => {}
        }
        i
    }

    /// FonTrasParola's `e` (0x1000cb5c).
    fn letter_e(&self, w: &[u8], i: usize, idx: usize, out: &mut Vec<u32>) -> usize {
        use ph::*;
        let v = self.variant;
        let b = |k: isize| -> u8 { if k < 0 { 0 } else { at(w, k as usize) } };
        let ii = i as isize;
        let (c1, c2, p1) = (b(ii + 1), b(ii + 2), b(ii - 1));
        let last = w.len() as isize - 1;
        let at_end = |k: isize| ii == last + k;
        let vowel = |c: u8| self.flags(c) & VOWEL != 0;
        let cons = |c: u8| self.among(VOICELESS, c) || self.among(VOICED, c);
        let m_coda = c1 == b'm' && !vowel(c2) && c2 != b'h' && c2 != b'n';
        let n_coda = c1 == b'n' && !vowel(c2) && c2 != b'h' && c2 != b'm';
        let mut push = |c: &[u32]| out.extend_from_slice(c);
        if v != 0 {
            if (c1 == b'm' || c1 == b'n') && at_end(-1) {
                push(&[0x10000741, J]);
                return i + 1;
            }
            if m_coda || n_coda {
                push(&[0x10000741]);
                return i + 1;
            }
            if ((c1 == 0xe1 || c1 == 0xe2) && i > 0 && (p1 == b't' || p1 == b'p')) || c1 == 0xf3 || c1 == 0xf4 {
                push(&[J]);
            } else if (c1 == b'a' && i > 0 && (p1 == b't' || p1 == b'p')) || c1 == b'o' {
                push(&[J]);
            } else if at_end(0) && (i == 0 || p1 != b'i') {
                push(&[I]);
            } else if at_end(-1) && c1 == b's' {
                push(&[I]);
            } else {
                push(&[E]);
            }
            return i;
        }
        if i <= 1 {
            if self.starts_any(w, E_I) {
                push(&[I]);
                return i;
            }
            if self.starts_any(w, E_OPEN) {
                push(&[EE]);
                return i;
            }
            if self.starts_any(w, E_CLOSED) {
                push(&[E]);
                return i;
            }
        }
        if w == b"ex" || (idx == 0 && c1 == b'x' && cons(c2)) {
            push(&[AR, J, SH]);
            return i + 1;
        }
        if c1 == b'i' {
            push(&[AR, J]);
            return i + 1;
        }
        if w == b"e" {
            push(&[I]);
            return i;
        }
        if c1 == b'u' {
            push(&[E]);
            return i;
        }
        if self.among(0x1002c440, c1) && self.among(0x1002c430, c2) {
            push(&[EE]);
            return i;
        }
        if (c1 == b'l' && (cons(c2) || at_end(-1))) || ((c1 == b'r' || c1 == b'x') && at_end(-1)) || (at_end(-2) && c1 == b'p' && c2 == b's') {
            push(&[EE]);
            return i;
        }
        // 0x1000ced7: the e of de-, re-… before b or a single consonant
        let mut to_ced7 = false;
        if i == 0 {
            if matches!(w.get(1), Some(b'b' | b'o' | 0xf2 | 0xf3 | 0xf4)) && cons(b(2)) {
                push(&[EE]);
                return i;
            }
            to_ced7 = true;
        } else {
            if (i == 1 || i == 3) && w.starts_with(self.s(0x10022aac)) {
                // tele- before a c d i j t x, telegui-, telerr-
                let tele = matches!(b(4), b'a' | b'c' | b'd' | b'i' | b'j' | b't' | b'x') || w.starts_with(self.s(0x1002c428)) || w.starts_with(self.s(0x1002c420));
                if tele {
                    push(&[EE]);
                    return i;
                }
            }
            if i == 1 && w[0] == b'h' {
                to_ced7 = true;
            }
        }
        if to_ced7 && (c1 == b'b' || (self.among(0x1002c414, c1) && vowel(c2))) {
            push(&[I]);
            return i;
        }
        if (c1 == b'm' && at_end(-1)) || (c1 == b'n' && c2 == b's' && at_end(-2)) {
            push(&[0x10000481, J]);
            return i + 1;
        }
        if c1 == b'n' && at_end(-1) {
            push(&[EE]);
            return i;
        }
        if i == 0 && matches!(b(1), b'p' | b'r' | b's') && vowel(b(2)) {
            push(&[E]);
            return i;
        }
        if c1 == b'n' && at_end(-1) {
            push(&[E]);
            return i;
        }
        if m_coda || n_coda {
            push(&[0x10000741]);
            return i + 1;
        }
        if matches!(c1, b'a' | 0xe1 | 0xe2 | 0xe0) || (matches!(c1, b'o' | 0xf3 | 0xf4 | 0xf2) && i > 0) {
            push(&[J]);
            return i;
        }
        if self.among(0x1002c40c, c1) && self.among(0x1002c404, c2) {
            push(&[J]);
            return i;
        }
        if i == 0 && self.among(0x1002c400, b(1)) && self.among(0x1002c3e8, b(2)) {
            return i;
        }
        push(&[HEH]);
        i
    }

    /// FonTrasParola's `o` (0x1000d54e).
    fn letter_o(&self, w: &[u8], i: usize, out: &mut Vec<u32>) -> usize {
        use ph::*;
        let v = self.variant;
        let b = |k: isize| -> u8 { if k < 0 { 0 } else { at(w, k as usize) } };
        let ii = i as isize;
        let (c1, c2) = (b(ii + 1), b(ii + 2));
        let last = w.len() as isize - 1;
        let at_end = |k: isize| ii == last + k;
        let vowel = |c: u8| self.flags(c) & VOWEL != 0;
        let cons = |c: u8| self.among(VOICELESS, c) || self.among(VOICED, c);
        let m_coda = c1 == b'm' && !vowel(c2) && c2 != b'h' && c2 != b'n';
        let n_coda = c1 == b'n' && !vowel(c2) && c2 != b'h' && c2 != b'm';
        let mut push = |c: &[u32]| out.extend_from_slice(c);
        if v != 0 {
            if m_coda || n_coda {
                push(&[0x10000361]);
                return i + 1;
            }
            if c1 == b'u' || c1 == b'i' {
                push(&[O]);
            } else if matches!(c1, 0xe1 | 0xe2 | b'a') {
                push(&[W]);
            } else if at_end(0) || (at_end(-1) && c1 == b's') {
                push(&[U]);
            } else {
                push(&[O]);
            }
            return i;
        }
        if i == 1 {
            if self.starts_any(w, O_CLOSED) {
                push(&[O]);
                return i;
            }
            if self.starts_any(w, O_OPEN) {
                push(&[OA]);
                return i;
            }
            if self.starts_any(w, O_U) {
                push(&[U]);
                return i;
            }
        }
        if c1 == b'u' || c1 == b'i' {
            push(&[O]);
            return i;
        }
        let rest = &w[(i + 1).min(w.len())..];
        if w == b"o" || w == b"os" || [(0x1002c3e0, 5), (0x1002c3d8, 4), (0x1002c3d0, 5), (0x1002c3c8, 4)].iter().any(|&(s, n)| strncmp(rest, self.s(s), n)) {
            push(&[U]);
            return i;
        }
        if i == 0 || (i == 1 && w[0] == b'o') {
            push(&[OA]);
            return i;
        }
        if (c1 == b'l' && (cons(c2) || at_end(-1))) || (c1 == b'r' && at_end(-1)) || (self.among(0x1002c440, c1) && self.among(0x1002c430, c2)) {
            push(&[OA]);
            return i;
        }
        if m_coda || n_coda {
            push(&[0x10000361]);
            return i + 1;
        }
        if matches!(c1, 0xe1 | 0xe2 | b'a') {
            push(&[W]);
            return i;
        }
        push(&[U]);
        i
    }

    /// FonTrasParola's `x` (0x1000e437).
    fn letter_x(&self, w: &[u8], i: usize, out: &mut Vec<u32>) -> usize {
        use ph::*;
        let v = self.variant;
        let b = |k: isize| -> u8 { if k < 0 { 0 } else { at(w, k as usize) } };
        let ii = i as isize;
        let (c1, c2) = (b(ii + 1), b(ii + 2));
        let last = w.len() as isize - 1;
        let vowel = |c: u8| self.flags(c) & VOWEL != 0;
        let mut push = |c: &[u32]| out.extend_from_slice(c);
        if c1 == b'x' {
            return i;
        }
        if w == b"\xe9x" || w == b"\xeax" || w == b"ex" {
            push(&[Z]);
        } else if self.starts_any(w, X_S) {
            push(&[S]);
        } else if self.starts_any(w, X_Z) {
            push(&[Z]);
        } else if self.starts_any(w, X_SH) {
            push(&[SH]);
        } else if ii == last || self.aligned_any(w, i, X_KS) {
            push(&[K, S]);
        } else if i == 0 {
            push(&[SH]);
        } else if v == 0 {
            if i == 1 && matches!(w[0], b'e' | 0xe9 | 0xea) && vowel(b(2)) {
                push(&[Z]);
            } else if self.among(VOICED, c1) {
                push(&[ZG]);
            } else {
                push(&[SH]);
            }
        } else if ii < last && c1 == b'c' && self.among(FRONT, c2) {
            push(&[S]);
            return i + 1;
        } else if self.among(VOICELESS, c1) || self.among(VOICED, c1) {
            push(&[S]);
        } else {
            push(&[SH]);
        }
        i
    }
}

/// `strncmp(a, b, n) == 0` on C strings (`a` ends at its end).
fn strncmp(a: &[u8], b: &[u8], n: usize) -> bool {
    for k in 0..n {
        let (x, y) = (at(a, k), at(b, k));
        if x != y {
            return false;
        }
        if x == 0 {
            return true;
        }
    }
    true
}

// --- FonTrasConfini (0x1000e8f0) --------------------------------------------

/// A vowel code (low bits 01).
fn vowel_code(c: u32) -> bool {
    c & 3 == 1
}

impl Portuguese {
    /// FonTrasConfini, called by the phrase loop before each word's codes
    /// (`next`) are appended to the chunk's phone list (`phones`): `w` is
    /// the sentence, `sentence[first + i]` the incoming word. Across the
    /// boundary a glide `w` is added after a nasal u (Brazil and Portugal)
    /// or between a/ɐ and o; Portugal's final ɨ goes before
    /// `vinte`/vowels; `s`/`ʃ` voice before a vowel or voiced consonant;
    /// then inside the word a nasal consonant after a nasal vowel before a
    /// stop, `ê` before a palatal said `ɐj`, `ʃs` loses its `s`.
    pub(super) fn tras_confini(&self, w: &[Word], first: usize, i: usize, phones: &mut Vec<Phone>, next: &mut Vec<u32>) {
        use ph::*;
        let s = first + i;
        let n0 = |next: &Vec<u32>| next.first().copied().unwrap_or(0);
        let add_w = |phones: &mut Vec<Phone>| {
            phones.push(Phone {
                word: i as u32,
                code: W,
                kind: crate::voicefon::KIND_PHONE,
            })
        };
        if self.variant != 0 {
            let n = phones.len();
            if i != 0 && n != 0 {
                let x = n0(next);
                let soft = |c: u32| vowel_code(c) || c == W || c == J;
                if n > 1 && phones[n - 1].code == S && soft(phones[n - 2].code) && soft(x) {
                    phones[n - 1].code = Z;
                } else if matches!(phones[n - 1].code, 0x100003e1 | 0x100103e1) && vowel_code(x) {
                    add_w(phones);
                }
            }
            return;
        }
        let prev_class = if i != 0 { w.get(s.wrapping_sub(1)).map_or(0, |r| r.f[CLASS]) } else { 0x17 };
        let x = n0(next);
        let n = phones.len();
        let mut tail = true;
        if n > 0 {
            let last = phones[n - 1].code;
            if matches!(last, 0x100003e1 | 0x100103e1) && vowel_code(x) {
                add_w(phones);
                tail = false;
            } else if matches!(last, 0x441 | 0x10441 | 0x481 | 0x10481) && (x == 0x10361 || x == 0x10010361) {
                add_w(phones);
                tail = false;
            } else if last == HEH {
                if s >= 1 && w.get(s - 1).is_some_and(|r| r.text == b"v\xednte") && w.get(s).is_some_and(|r| r.f[SUB] == 0x82) {
                    phones.pop();
                    tail = false;
                } else if x == OA_ || x == AR_ {
                    phones[n - 1].code = J;
                    tail = false;
                } else if matches!(prev_class, 0x20 | 0x19 | 0x1f) && vowel_code(x) {
                    if x == I || x == I_ {
                        phones.pop();
                    } else {
                        phones[n - 1].code = J;
                    }
                    tail = false;
                }
            }
        }
        if tail {
            let n = phones.len();
            if n > 1 && phones[n - 2].code == L && phones[n - 1].code == HEH && x == M {
                phones.pop();
                let m = phones.len();
                phones[m - 1].code = LDARK;
            }
        }
        // 0x1000eb9e
        if let Some(p) = phones.last_mut() {
            if p.code == SH && vowel_code(x) {
                p.code = Z;
            }
        }
        if let Some(p) = phones.last_mut() {
            if p.code == SH && x & 3 == 2 && (x >> 4) & 1 == 0 {
                p.code = ZG;
            }
        }
        let text = |k: usize| w.get(k).map(|r| r.text.as_slice());
        if text(s) == Some(b"c\xf4") && text(s + 1) == Some(b"-") && !next.is_empty() {
            set(next, 1, O);
        }
        if text(s) == Some(b"s\xea") && i >= 1 && prev_class == b'7' {
            set(next, 1, 0);
        }
        // inside the word
        let word = text(s).unwrap_or(b"");
        let g = |next: &Vec<u32>, k: isize| -> u32 { if k < 0 { 0 } else { next.get(k as usize).copied().unwrap_or(0) } };
        let mut k = 0usize;
        while g(next, k as isize) != 0 {
            let ki = k as isize;
            let (c, c1) = (g(next, ki), g(next, ki + 1));
            if c == HEH && k > 0 && c1 & 3 == 2 {
                let p = g(next, ki - 1);
                let hit = (p == M && c1 == LH)
                    || (p == T && c1 == M && g(next, ki + 2) == 0x10010741 && g(next, ki + 3) == T && g(next, ki + 4) == HEH && g(next, ki + 5) == 0)
                    || (k > 1 && vowel_code(g(next, ki - 2)) && p == L && c1 != R && c1 != L && vowel_code(g(next, ki + 2)));
                if hit && ![0x10022b68, 0x10022b4c, 0x1002c48c].iter().any(|&t| strncmp(word, self.s(t), 7)) {
                    next.remove(k);
                    if k > 0 && next[k - 1] == L {
                        next[k - 1] = LDARK;
                    }
                }
            }
            let c = g(next, ki);
            let c1 = g(next, ki + 1);
            if vowel_code(c) && c & NASAL != 0 && c1 & 3 == 2 && c1 & 0x1e3 == 0x1e2 {
                next.insert(k + 1, c1);
                let stop = g(next, ki + 2) & 0x1e03;
                next[k + 1] = if stop == 0xe02 {
                    NG
                } else if stop == 0x1e02 {
                    M
                } else {
                    N
                };
            } else if c == E_ && matches!(c1, ZG | SH | NH | LH) && vowel_code(g(next, ki + 2)) {
                next.insert(k + 1, c1);
                next[k] = AR_;
                next[k + 1] = J;
            } else if c == SH && c1 == S && ![0x1002c484, 0x1002c47c, 0x1002c474].iter().any(|&t| contains(word, self.s(t))) {
                next.remove(k + 1);
            }
            k += 1;
        }
    }
}

/// `next[k] = c` on the zero-terminated list (0 cuts it there).
fn set(next: &mut Vec<u32>, k: usize, c: u32) {
    if c == 0 {
        next.truncate(k);
    } else if k < next.len() {
        next[k] = c;
    } else {
        next.push(c);
    }
}

/// `strstr(a, b) != NULL` (0x1000f420).
fn contains(a: &[u8], b: &[u8]) -> bool {
    b.is_empty() || a.windows(b.len()).any(|x| x == b)
}
