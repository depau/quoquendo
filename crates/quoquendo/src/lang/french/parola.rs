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
    pub(crate) fn tras_parola(&self, words: &[Word], idx: usize) -> Vec<u32> {
        let r = &words[idx];
        // a dictionary pronunciation
        let mut lex = Lex::of(r);
        if self.has_col(&mut lex, col::FONETICA) {
            if let Some(row) = lex.current() {
                return self.parse_phones(self.col(row, col::FONETICA).unwrap_or(&[]));
            }
        }
        let grade = r.f[GRADE];
        let bound = r.f[BOUND];
        let ii = idx as isize;
        let text = |k: isize| -> Option<&[u8]> {
            if k < 0 {
                None
            } else {
                words.get(k as usize).map(|r| &r.text[..])
            }
        };
        let mut next = text(ii + 1);
        let mut after_dash = false;
        if next == Some(b"-") {
            next = text(ii + 2);
            after_dash = true;
        }
        let prevw = if idx == 0 { None } else { text(ii - 1) };
        let w: Vec<u8> = r.text.iter().map(|&c| self.lower(c)).collect();
        let c = Ctx { fr: self, l: w.len() as isize - 1, w };
        let l = c.l;
        let g = |k: isize| c.g(k);
        let ww = &c.w[..];
        let mut out: Vec<u32> = Vec::new();
        let mut i: isize = 0;
        while i <= l {
            let ch = g(i);
            if ch == 0 {
                break;
            }
            let n1 = g(i + 1);
            let p = g(i - 1);
            match ch {
                b'b' => {
                    if n1 != b'b' {
                        if !c.voiceless(n1) {
                            if !(i != 0 && p == b'm' && (i == l || (i == l - 1 && n1 == b's'))) {
                                out.push(B_);
                            }
                        } else {
                            out.push(P_);
                        }
                    }
                }
                b'c' => self.letter_c(&c, &mut out, &mut i, idx, words, prevw),
                b'f' => {
                    if n1 != b'f' {
                        let v = ww == b"neuf" && (next == Some(b"ans") || next == Some(b"heures"));
                        out.push(if v { V_ } else { F_ });
                    }
                }
                b'g' => {
                    let mut b5 = n1;
                    if n1 == b'g' {
                        if c.at(i, b"reggi") {
                            out.push(D_);
                            i += 1;
                            continue;
                        }
                        if !c.at(i, self.s(0x1003f11c)) {
                            i += 1;
                            continue;
                        }
                        b5 = n1;
                    }
                    if c.front(b5) {
                        out.push(ZH);
                    } else if n1 != b'd' {
                        if n1 == b'u' && (c.front(g(i + 2)) || c.st(i + 1, b"ai")) && !c.at(i, b"aiguill") {
                            out.push(G_);
                            i += 1;
                        } else if n1 == b'n' {
                            if !c.at(i, b"diagn") {
                                out.push(GN);
                            } else {
                                out.push(G_);
                                out.push(N_);
                            }
                            i += 1;
                        } else if n1 != b't' {
                            let free = i == 0 || !c.cons(p) || !c.end(i);
                            if free || [&b"yang"[..], b"gang", b"erg", b"antigang"].contains(&ww) {
                                out.push(G_);
                            }
                        }
                    }
                }
                b'j' => out.push(ZH),
                b'k' => {
                    if n1 != b'k' {
                        out.push(K_);
                    }
                }
                b'l' => {
                    if n1 != b'l' {
                        out.push(L_);
                    }
                }
                b'm' => {
                    if n1 != b'm' && !c.at(i, self.s(0x1003f0e4)) && !c.at(i, self.s(0x1003f0dc)) {
                        out.push(M_);
                    }
                }
                b'p' => {
                    if n1 != b'p' {
                        if n1 == b'h' {
                            out.push(F_);
                            i += 1;
                        } else {
                            let a = !c.cons(n1) || !(i + 1 == l || (i + 1 == l - 1 && g(i + 2) == b's'));
                            let b = i < 2 || !c.st(i - 2, b"ou") || !c.end(i);
                            if a && b && !c.ats(i, &[b"compt", b"dompt", self.s(0x1003f0c4), b"sculpt"]) {
                                out.push(P_);
                            }
                        }
                    }
                }
                b'q' => {
                    if n1 != b'q' {
                        if !c.at(i, self.s(0x1003f0b4)) || c.at(i, self.s(0x1003f0ac)) {
                            if (!c.at(i, self.s(0x1003f0a4)) && !c.at(i, self.s(0x1003f09c))) || i != 1 {
                                out.push(K_);
                                if n1 == b'u' && (c.vow(g(i + 2)) || g(i + 2) == b'\'') {
                                    i += 1;
                                }
                            } else {
                                out.push(K_);
                                out.push(W_);
                                i = 2;
                            }
                        } else {
                            out.push(K_);
                            out.push(W_);
                            i += 1;
                        }
                    }
                }
                b'r' => {
                    if n1 != b'r' {
                        out.push(R_);
                        if n1 == b'h' {
                            i += 1;
                        }
                    }
                }
                b's' => self.letter_s(&c, &mut out, &mut i, idx, words, grade, bound, next, prevw),
                b't' => self.letter_t(&c, &mut out, &mut i, idx, grade, bound, next, prevw),
                b'v' | b'w' => {
                    if n1 != ch {
                        out.push(V_);
                    }
                }
                b'x' => self.letter_x(&c, &mut out, &mut i, bound, next),
                b'z' => {
                    if n1 != b'z' || i + 1 == l || (i + 1 == l - 1 && g(i + 2) == b's') {
                        if i == 0 || g(i - 1) != b't' {
                            if !c.end(i) {
                                out.push(Z_);
                            }
                        } else {
                            out.push(S_);
                        }
                    }
                }
                0x9c | b'e' => self.letter_e(&c, &mut out, &mut i, idx, grade, bound, prevw, after_dash),
                0xdf | 0xe7 => out.push(S_),
                0xe0 | 0xe1 | 0xe3 | 0xe4 | 0xe5 | 0xe6 | b'a' => self.letter_a(&c, &mut out, &mut i),
                0xe2 => out.push(AA),
                0xe8 | 0xea | 0xeb => out.push(EH),
                0xe9 => out.push(E_),
                0xec | 0xed | b'i' => self.letter_i(&c, &mut out, &mut i),
                0xee | 0xef => {
                    if !c.vow(n1) || (c.end(i + 1) && n1 == b'e') {
                        out.push(I_);
                    } else {
                        out.push(J_);
                    }
                }
                0xf0 | b'd' => {
                    if n1 != ch && i != l && (i != l - 1 || n1 != b's') {
                        out.push(if c.voiceless(n1) { T_ } else { D_ });
                    }
                }
                0xf1 | b'n' => {
                    if n1 != ch {
                        out.push(N_);
                    }
                }
                0xf2 | 0xf3 | 0xf5 | 0xf6 | 0xf8 | b'o' => self.letter_o(&c, &mut out, &mut i),
                0xf4 => out.push(O_),
                0xf9 | 0xfa | 0xfb | 0xfc | b'u' => self.letter_u(&c, &mut out, &mut i),
                0xfd | 0xff | b'y' => {
                    if n1 == b'o' {
                        out.push(I_);
                        out.push(J_);
                    } else if !c.vow(n1) {
                        let n2 = g(i + 2);
                        if (n1 == b'n' && !c.vow(n2) && !matches!(n2, b'p' | b'b' | b'm' | b'n')) || c.st(i + 1, b"mp") || c.st(i + 1, b"mb") {
                            out.push(EN);
                            i += 1;
                        } else {
                            out.push(I_);
                        }
                    } else {
                        out.push(J_);
                    }
                }
                _ => {}
            }
            i += 1;
        }
        self.tras_post(words, idx, &mut out);
        out
    }

    fn letter_c(&self, c: &Ctx, out: &mut Vec<u32>, i: &mut isize, idx: usize, words: &[Word], _prevw: Option<&[u8]>) {
        let ii = *i;
        let l = c.l;
        let g = |k: isize| c.g(k);
        let n1 = g(ii + 1);
        let p = g(ii - 1);
        if n1 == b'c' {
            if ii + 1 != l {
                if !c.front(g(ii + 2)) {
                    if ii + 1 == l || g(ii + 2) != b'h' {
                        return;
                    }
                    out.push(K_);
                    *i = ii + 1;
                } else {
                    out.push(K_);
                }
            }
            return;
        }
        // `est-ce`
        if c.w == b"ce" && idx > 1 && words[idx - 1].text == b"-" && words[idx - 2].text == b"est" {
            out.push(S_);
            *i += 1;
            return;
        }
        if n1 == b'k' {
            out.push(K_);
            *i += 1;
            return;
        }
        if c.at(ii, b"secon") {
            out.push(G_);
            return;
        }
        if c.front(n1) && !out.is_empty() && out[out.len() - 1] == S_ {
            // sc before e/i: the s says it
            return;
        }
        if c.front(n1) {
            out.push(S_);
            return;
        }
        if c.ats(ii, &[b"chnauff", b"schlitt", b"chlinguer", self.s(0x1003f208), self.s(0x1003f200), self.s(0x1003f1f8)]) {
            out.push(SH);
            *i += 1;
            return;
        }
        if c.st(ii + 1, b"hr") || c.st(ii + 1, b"hl") || c.st(ii + 1, b"hn") || c.st(ii + 1, b"hm") {
            out.push(K_);
            *i += 1;
            return;
        }
        if n1 == b'h' && (c.at(ii, self.s(0x1003f1e0)) || c.at(ii, self.s(0x1003f1d8))) && ii == 2 {
            out.push(K_);
            *i = 3;
            return;
        }
        if n1 == b'h' {
            let greek: [&[u8]; 20] = [
                b"psychia",
                b"psycha",
                b"psycho",
                b"schizo",
                self.s(0x1003f3bc),
                b"ichty",
                b"chton",
                self.s(0x1003f1a0),
                self.s(0x1003f198),
                b"broncho",
                b"brach",
                self.s(0x1003f180),
                b"tachy",
                b"eschatolo",
                b"eucharist",
                self.s(0x1003f158),
                b"chiro",
                b"chond",
                b"conchy",
                b"machia",
            ];
            if c.ats(ii, &greek) {
                out.push(K_);
                *i += 1;
                return;
            }
        }
        if n1 == b'h' && !c.at(ii, b"choeur") {
            out.push(SH);
            *i += 1;
            return;
        }
        let silent_final = ii != 0 && c.cons(p) && p != b's' && c.end(ii);
        if !silent_final {
            if n1 != b'\'' {
                if !c.st(ii + 1, b"qu") {
                    out.push(K_);
                }
            } else {
                out.push(S_);
            }
        }
    }
}

/// A vowel letter among the five before `i` (0x10011113…).
fn vowel_before(c: &Ctx, i: isize) -> bool {
    (1..=5).any(|k| i >= k && c.vow(c.g(i - k)))
}

impl French {
    #[allow(clippy::too_many_arguments)]
    fn letter_s(
        &self,
        c: &Ctx,
        out: &mut Vec<u32>,
        i: &mut isize,
        idx: usize,
        words: &[Word],
        grade: u8,
        bound: u8,
        next: Option<&[u8]>,
        prevw: Option<&[u8]>,
    ) {
        let ii = *i;
        let l = c.l;
        let g = |k: isize| c.g(k);
        let n1 = g(ii + 1);
        let p = g(ii - 1);
        if !(n1 != b's' || ii + 1 == l || (ii + 1 == l - 1 && g(ii + 2) == b's')) {
            return;
        }
        if ii != 0 {
            if c.ats(ii, &[self.s(0x1003f098), self.s(0x1003f090), b"asemanti", b"asepti"]) || c.st(ii + 1, b"y") {
                out.push(S_);
                return;
            }
            if c.at(ii, b"balsa") {
                out.push(Z_);
                return;
            }
            if c.vow(p) && c.vow(n1) && !c.prefix(ii - 1) {
                out.push(Z_);
                return;
            }
        }
        // 0x1001340c
        if n1 == b'h' {
            out.push(if ii == 0 || ii + 1 == l { SH } else { Z_ });
            *i += 1;
            return;
        }
        if c.st(ii + 1, b"ch") && !c.at(ii, b"schizo") && !c.at(ii, b"eschatolo") {
            out.push(SH);
            *i += 2;
            return;
        }
        if ii == l && ii == 0 {
            out.push(S_);
            return;
        }
        if ii != 0 && ii + 1 < l && p == b'x' && !c.at(ii, self.s(0x1001c368)) && !c.at(ii, self.s(0x1001c384)) {
            return;
        }
        let plus = c.w == b"plus";
        if plus {
            if let Some(pw) = prevw {
                if (pw == b"pas" || pw == b"jamais") && grade != 9 {
                    out.push(S_);
                    return;
                }
            }
        }
        let text = |k: usize| words.get(k).map(|r| &r.text[..]);
        let neg = |t: Option<&[u8]>| t == Some(b"ne") || t == Some(b"n'");
        if plus && (pause_after(bound) || next == Some(b"que") || next == Some(b"de")) {
            // `plus` said with its s, unless after ne/n'
            if idx < 2 || !neg(text(idx - 2)) {
                if idx < 3 || !neg(text(idx - 3)) {
                    out.push(S_);
                    return;
                }
            }
        }
        if ii == l && c.w != b"mars" && (ii < 2 || !c.st(ii - 2, self.s(0x1003f064))) {
            return;
        }
        out.push(S_);
    }

    #[allow(clippy::too_many_arguments)]
    fn letter_t(
        &self,
        c: &Ctx,
        out: &mut Vec<u32>,
        i: &mut isize,
        idx: usize,
        grade: u8,
        bound: u8,
        next: Option<&[u8]>,
        prevw: Option<&[u8]>,
    ) {
        let ii = *i;
        let l = c.l;
        let g = |k: isize| c.g(k);
        let n1 = g(ii + 1);
        let p = g(ii - 1);
        if !(n1 != b't' || ii + 1 == l || (ii + 1 == l - 1 && g(ii + 2) == b's')) {
            return;
        }
        if c.at(ii, b"-vingt-") || c.at(ii, self.s(0x1003f420)) {
            return;
        }
        if c.at(ii, b"vingt-") && (g(ii + 2) == b't' || g(ii + 2) == b'd') {
            out.push(N_);
            return;
        }
        if ii != 0 && ii + 1 < l && p == b's' && n1 == b'h' && c.cons(g(ii + 2)) {
            return;
        }
        if c.at(ii, self.s(0x1003edd0)) && ii == 2 {
            out.push(T_);
            return;
        }
        if ii != 0 && p == b'b' {
            out.push(T_);
            return;
        }
        if grade == 1 {
            let verb_t = (c.at(ii, self.s(0x1003f04c)) && ii == l - 3)
                || (c.at(ii, b"tions") && ii == l - 4)
                || (c.at(ii, b"tient") && ii == l - 4)
                || (c.at(ii, self.s(0x1003f034)) && ii == l - 3)
                || c.at(ii, b"tienne")
                || (c.at(ii, b"tiendront") && ii == l - 8);
            if verb_t {
                out.push(T_);
                return;
            }
        }
        // 0x10013ac2
        if c.st(ii + 1, b"ier") && !c.at(ii, b"balbutier") && !c.at(ii, b"initier") {
            out.push(T_);
            return;
        }
        if ii == 3 && c.at(3, b"initi") {
            out.push(S_);
            return;
        }
        if c.at(ii, self.s(0x1003eff8)) || (c.st(ii + 1, b"ie") && ii != 0 && p == b't') {
            out.push(T_);
            return;
        }
        if (c.at(ii, self.s(0x1003efec)) || c.at(ii, self.s(0x1003efe4))) && c.end(ii + 2) {
            out.push(T_);
            return;
        }
        if c.st(ii + 1, b"iel") || c.st(ii + 1, b"iai") || (c.st(ii + 1, b"iat") && !c.prefix(ii)) {
            out.push(S_);
            return;
        }
        if ii != 0 && (p == b's' || p == b'x') {
            out.push(T_);
            return;
        }
        if [&b"ial"[..], b"ion", b"iaux", b"ieux", b"ieuse"].iter().any(|s| c.st(ii + 1, s)) {
            out.push(S_);
            return;
        }
        if c.st(ii + 1, b"ie") && ii != 0 && !c.end(ii + 2) && !c.prefix(ii) {
            let keep: [&[u8]; 8] =
                [self.s(0x1003efa8), b"antienne", b"kantien", b"koweitien", b"maintien", b"mozartien", b"appartien", b"soutien"];
            if !c.ats(ii, &keep) && c.w != b"entretien" && c.w != b"entretiens" {
                out.push(S_);
                return;
            }
        }
        if c.st(ii + 1, b"ie") && c.end(ii + 2) && (ii == 0 || p != b'r') {
            let keep: &[&[u8]] = &[
                b"garantie",
                b"h\xe9matie",
                b"r\xf4tie",
                b"sotie",
                b"tatie",
                b"\xe9pizootie",
                b"b\xe2tie",
                b"repentie",
                b"d\xe9mentie",
                b"sentie",
            ];
            if !c.ats(ii, keep) {
                out.push(S_);
                return;
            }
        }
        if (c.w == b"huit" || c.w == b"huits") && !pause_after(bound) {
            if let Some(nx) = next {
                if ![&b"virgule"[..], b"heures", b"ans", b"euros"].contains(&nx) {
                    return;
                }
            }
        }
        if idx != 0 && c.w == b"fait" && prevw == Some(b"en") && pause_after(bound) {
            out.push(T_);
            return;
        }
        if n1 != b'-' && !c.at(ii, b"huit") && !c.at(ii, b"sept") {
            if c.w == b"vingt" && next == Some(b"huit") {
                out.push(T_);
                return;
            }
            if (ii == l || (ii == l - 1 && n1 == b's')) && ![&b"cet"[..], b"set", b"ao\xfbt"].contains(&&c.w[..]) {
                return;
            }
        }
        out.push(T_);
    }

    fn letter_x(&self, c: &Ctx, out: &mut Vec<u32>, i: &mut isize, bound: u8, next: Option<&[u8]>) {
        let ii = *i;
        let g = |k: isize| c.g(k);
        let n1 = g(ii + 1);
        if n1 == b'x' {
            return;
        }
        if c.at(ii, b"ixhuit") || c.at(ii, b"ixneuf") {
            out.push(Z_);
            return;
        }
        if c.at(ii, b"ixsept") || c.at(ii, b"ix-sept") {
            return;
        }
        if c.at(ii, b"ix-huit") || c.at(ii, b"ix-neuf") {
            out.push(Z_);
            return;
        }
        if c.at(ii, b"deuxi\xe8m") || c.at(ii, b"sixi\xe8m") || c.at(ii, b"dixi\xe8m") {
            out.push(Z_);
            return;
        }
        if c.w != b"six" && c.w != b"dix" {
            if c.at(ii, b"soixant") || c.at(ii, b"six") || c.at(ii, b"dix") {
                out.push(S_);
                return;
            }
            if !c.at(ii, b"sexag\xe9") {
                if c.end(ii) && c.ats(ii, &[b"crucifix", b"flux", b"perdrix", b"prix"]) {
                    return;
                }
                let gz_words: &[&[u8]] = &[
                    self.s(0x1003f098),
                    b"lex",
                    b"tox",
                    b"xiant",
                    b"xion",
                    b"xial",
                    b"xism",
                    b"xie",
                    b"xit",
                    b"axi",
                    b"oxy",
                ];
                // these say ks (the decompiler shows the branches swapped;
                // the original's output on the held-out words settles it)
                let ks = c.ats(ii, gz_words) || (c.at(ii, b"lux") && !c.end(ii));
                let mut voiced = false;
                if !ks && (c.w != b"ex" || pause_after(bound)) {
                    // ex- before a vowel (exact, hexagone, inexact…)
                    let ex_start = (ii == 1 && g(0) == b'e') || (ii == 2 && c.st(0, b"he"));
                    let inex = ii == 3 && (c.st(0, b"ine") || c.st(0, b"coe") || c.st(0, b"r\xe9e")) && c.vow(g(4));
                    voiced = (ex_start && c.vow(n1)) || inex;
                }
                if voiced {
                    out.push(G_);
                    out.push(Z_);
                } else {
                    out.push(K_);
                    out.push(S_);
                }
                return;
            }
        }
        if !pause_after(bound) && next != Some(b"virgule") {
            return;
        }
        out.push(S_);
    }
}

impl French {
    #[allow(clippy::too_many_arguments)]
    fn letter_e(
        &self,
        c: &Ctx,
        out: &mut Vec<u32>,
        i: &mut isize,
        _idx: usize,
        grade: u8,
        _bound: u8,
        prevw: Option<&[u8]>,
        after_dash: bool,
    ) {
        let ii = *i;
        let l = c.l;
        let g = |k: isize| c.g(k);
        let w = &c.w[..];
        let n1 = g(ii + 1);
        let p = g(ii - 1);
        if w == b"demi" && after_dash {
            return;
        }
        if c.st(ii + 1, b"aux") && c.end(ii + 3) {
            out.push(O_);
            *i = ii + 3;
            return;
        }
        // a verb's -ent
        if ii != 0 && ii + 2 == l && c.st(ii + 1, b"nt") && grade == 1 {
            *i = ii + 2;
            if c.vow(p) && ii > 1 && p == b'u' && g(ii - 2) == b'q' {
                out.push(SCHWA);
            }
            return;
        }
        if w == b"est" {
            out.push(EH);
            *i = ii + 2;
            return;
        }
        if w == b"eu" {
            out.push(Y_);
            *i = ii + 1;
            return;
        }
        if w == b"eue" || w == b"eus" || w == b"eues" {
            out.push(Y_);
            *i = ii + 2;
            return;
        }
        if c.at(ii, self.s(0x1003f420)) || w == b"et" {
            out.push(E_);
            return;
        }
        if c.st(ii + 1, b"au") {
            out.push(O_);
            *i = ii + 2;
            return;
        }
        if c.st(ii + 1, b"ux") && c.end(ii + 2) {
            out.push(OE_);
            *i = ii + 2;
            return;
        }
        if c.st(ii + 1, b"u") && (g(ii + 2) == b's' || g(ii + 2) == b'x') {
            out.push(OE_);
            *i = ii + 1;
            return;
        }
        if c.st(ii + 1, b"u") && c.end(ii + 1) {
            out.push(OE_);
            *i = ii + 1;
            return;
        }
        if c.st(ii + 1, b"u") {
            let closed = c.at(ii, b"meule") || c.at(ii, b"veule") || c.st(ii + 1, b"ut") || c.at(ii, b"euro");
            out.push(if closed { OE_ } else { OEH });
            *i = ii + 1;
            return;
        }
        if (c.st(ii + 1, b"in") && !c.vow(g(ii + 3))) || c.st(ii + 1, b"imp") || c.st(ii + 1, b"imb") {
            out.push(EN);
            *i = ii + 2;
            return;
        }
        if c.st(ii + 1, b"y") {
            if ii + 1 >= l || !c.vow(g(ii + 2)) {
                out.push(EH);
            } else {
                out.push(EH);
                out.push(J_);
            }
            *i = ii + 1;
            return;
        }
        if c.st(ii + 1, b"il") {
            out.push(EH);
            return;
        }
        if c.st(ii + 1, b"i") {
            out.push(EH);
            *i = ii + 1;
            return;
        }
        if (ii != 0 && c.vow(p) && c.st(ii + 1, b"n") && c.end(ii + 1)) || (c.at(ii, b"bien") && !c.st(ii + 1, b"nn")) {
            out.push(EN);
            *i = ii + 1;
            return;
        }
        if c.st(ii + 1, b"n") && !c.st(ii + 1, b"nn") && ii > 1 && c.vow(p) && g(ii - 2) != b'q' && grade == 1 {
            out.push(EN);
            *i = ii + 1;
            return;
        }
        let n2 = g(ii + 2);
        if c.st(ii + 1, b"n") && ((!c.vow(n2) && !matches!(n2, b'p' | b'b' | b'n')) || ii == 0) {
            let mut skip_to = ii;
            if w == b"sens" || !c.st(ii + 1, b"ns") || ii == 0 || c.vow(p) || !c.end(ii + 2) {
                out.push(AN);
            } else {
                out.push(EN);
                skip_to = ii + 2;
                *i = skip_to;
            }
            if skip_to == 0 {
                if c.st(1, b"nn") {
                    return;
                }
            }
            *i = skip_to + 1;
            return;
        }
        if c.st(ii + 1, b"mp") || c.st(ii + 1, b"mb") || (c.st(ii + 1, b"m") && ii == 0 && !c.vow(g(2))) {
            out.push(AN);
            if ii == 0 && c.st(1, b"mm") {
                return;
            }
            *i = ii + 1;
            return;
        }
        if c.at(ii, b"emment") && c.end(ii + 5) {
            out.push(A);
            *i = ii + 1;
            return;
        }
        if c.st(ii + 1, b"mm") && ii == 1 && c.st(0, b"r") {
            out.push(AN);
            *i = 2;
            return;
        }
        if c.st(ii + 1, b"an") {
            out.push(AN);
            *i = ii + 2;
            return;
        }
        if c.st(ii + 1, b"un") {
            out.push(OEN);
            *i = ii + 2;
            return;
        }
        if c.st(ii + 1, b"o") {
            return;
        }
        // the e's that are not followed by a letter group: final e, -er,
        // -ez, mute e between consonants
        if n1 == b'r' && c.end(ii + 1) && vowel_before(c, ii) && !c.at(ii, b"vers") {
            if w == b"fier" && grade != 1 {
                out.push(EH);
            } else if w == b"tiers" {
                out.push(EH);
            } else {
                out.push(E_);
                *i = ii + 1;
            }
            return;
        }
        if n1 == b'z' && c.end(ii + 1) {
            out.push(E_);
            return;
        }
        if ii == 0 {
            if l != 0 {
                let cons2 = c.cons(g(1)) && g(2) != 0 && c.cons(g(2)) && g(1) != g(2);
                if l < 2 || (!cons2 && g(1) != b'x' && !c.at(0, b"elle")) {
                    out.push(E_);
                } else {
                    out.push(EH);
                }
                return;
            }
        } else if ii == 1 && c.at(1, b"ress") {
            out.push(SCHWA);
            return;
        }
        // 0x1001095f
        let open = (!c.cons(n1) || c.vow(n2) || n2 == b'-' || ii == l || (ii == l - 1 && n1 == b's')) && n1 != b'x';
        if !open {
            let cluster = (n1 != 0 && b"bcdfgkptv".contains(&n1) && (n2 == b'r' || n2 == b'l')) || (n1 == b'c' && n2 == b'h');
            out.push(if !cluster || c.at(ii, b"tech") { EH } else { SCHWA });
            return;
        }
        if c.st(ii + 1, b"t") && c.end(ii + 1) {
            out.push(EH);
            *i = ii + 1;
            return;
        }
        let n = out.len();
        let last = |k: usize| if n >= k { out[n - k] } else { 0 };
        if ii != 0 && n == 1 && is_cons_code(out[0]) && ii < l - 1 && c.cons(n1) && c.vow(n2) {
            out.push(SCHWA);
            return;
        }
        let a2_fails = ii < 4
            || (g(ii - 4) != 0 && b"bcdfgkptv".contains(&g(ii - 4)))
            || g(ii - 2) != b'e'
            || g(ii - 3) != b'r'
            || !is_cons_code(last(1))
            || ii >= l - 1
            || !c.cons(n1)
            || !c.vow(n2);
        if !a2_fails {
            out.push(SCHWA);
            return;
        }
        if ii == 3 && g(1) == b'e' && g(0) == b'r' && is_cons_code(last(1)) && 3 < l - 1 && c.cons(g(4)) && c.vow(g(5)) {
            out.push(SCHWA);
            return;
        }
        if (c.st(ii + 1, b"r") || c.st(ii + 1, b"l") || c.st(ii + 1, b"n"))
            && ii + 1 < l
            && c.vow(n2)
            && n2 != b'a'
            && n2 != b'e'
            && ii + 2 < l
            && c.vow(g(ii + 3))
            && ii + 3 != l
            && (ii + 3 != l - 1 || g(ii + 4) != b's')
        {
            out.push(SCHWA);
            return;
        }
        if (c.at(ii, b"sem") && c.vow(n2)) || (ii != 0 && ii + 1 < l && p == b'g' && n1 == b'a') {
            return;
        }
        if n > 1 && is_cons_code(last(1)) && is_cons_code(last(2)) && !c.end(ii) {
            out.push(SCHWA);
            return;
        }
        if ii > 1 {
            let prev_kind = last(1) & 3;
            if prev_kind == 1 && !c.end(ii) {
                return;
            }
            if n != 0 && ii < l && prev_kind == 2 && c.cons(n1) && !(ii == l - 1 && n1 == b's') {
                if (last(1) == R_ && n1 != b'm') || last(1) == GN {
                    out.push(SCHWA);
                }
                return;
            }
        }
        // 0x10010e69
        if ii == 2 && n == 2 && is_cons_code(out[1]) && is_cons_code(out[0]) && (l == 2 || (l == 3 && g(3) == b's')) {
            out.push(SCHWA);
            return;
        }
        if [&b"de"[..], b"le", b"me", b"ne", b"que", b"se", b"te", b"e"].contains(&w) {
            out.push(SCHWA);
            return;
        }
        if w == b"ce" || w == b"je" {
            if prevw != Some(b"-") {
                out.push(SCHWA);
            }
            return;
        }
        if c.end(ii) && vowel_before(c, ii) {
            out.push(SCHWA);
            return;
        }
        if n1 == b'-' {
            out.push(SCHWA);
            return;
        }
        out.push(E_);
    }

    fn letter_a(&self, c: &Ctx, out: &mut Vec<u32>, i: &mut isize) {
        let ii = *i;
        let l = c.l;
        let g = |k: isize| c.g(k);
        if ii != 0 && (c.prefix(ii) || c.prefix(ii - 1)) {
            out.push(A);
            return;
        }
        if (c.st(ii + 1, b"in") && !c.vow(g(ii + 3))) || c.st(ii + 1, b"imp") || c.st(ii + 1, b"imb") || (c.st(ii + 1, b"im") && ii == l - 2)
        {
            out.push(EN);
            *i = ii + 2;
            return;
        }
        if c.st(ii + 1, b"ill") {
            out.push(A);
            return;
        }
        if c.st(ii + 1, b"il") && c.end(ii + 2) {
            out.push(A);
            out.push(J_);
            *i = ii + 2;
            return;
        }
        if c.st(ii + 1, b"ix") && ii + 2 == l {
            out.push(EH);
            *i = ii + 2;
            return;
        }
        if c.at(ii, self.s(0x1003f47c)) {
            // `pays`
            out.push(E_);
            return;
        }
        if c.st(ii + 1, b"yer") {
            out.push(E_);
            return;
        }
        if ii + 1 < l && c.st(ii + 1, b"y") && c.vow(g(ii + 2)) {
            out.push(EH);
            out.push(J_);
            *i = ii + 1;
            return;
        }
        if c.at(ii, self.s(0x1003f470)) && l > ii + 2 && c.vow(g(ii + 3)) {
            // `faisant`…
            out.push(SCHWA);
            *i = ii + 1;
            return;
        }
        let n2 = g(ii + 2);
        if c.st(ii + 1, b"i") || c.st(ii + 1, b"\xee") || c.st(ii + 1, b"y") {
            out.push(EH);
            *i = ii + 1;
        } else if c.st(ii + 1, b"n") && (c.cons(n2) || c.end(ii + 1)) && n2 != b'n' {
            out.push(AN);
            *i = ii + 1;
        } else if c.st(ii + 1, b"mp") || c.st(ii + 1, b"mb") {
            out.push(AN);
            *i = ii + 1;
        } else if c.st(ii + 1, b"ux") && !c.vow(g(ii + 3)) {
            out.push(O_);
            *i = ii + 2;
        } else if c.st(ii + 1, b"u") {
            out.push(if c.st(ii + 1, b"ur") { OA } else { O_ });
            *i = ii + 1;
        } else if c.st(ii + 1, b"en") || c.st(ii + 1, b"on") {
            out.push(AN);
            *i = ii + 2;
        } else if c.st(ii + 1, b"e") {
            out.push(E_);
            *i = ii + 1;
        } else if c.st(ii + 1, b"o\xfb") && c.w == b"ao\xfbt" {
            out.push(U_);
            *i = ii + 2;
        } else {
            out.push(A);
        }
    }
}

impl French {
    fn letter_i(&self, c: &Ctx, out: &mut Vec<u32>, i: &mut isize) {
        let ii = *i;
        let l = c.l;
        let g = |k: isize| c.g(k);
        let p = g(ii - 1);
        let n = out.len();
        if ii != 0 && (c.prefix(ii) || c.prefix(ii - 1)) && !c.at(ii, b"antille") {
            out.push(I_);
            return;
        }
        let ll = c.st(ii + 1, b"ll");
        // 0x100115b7: i + l (ville, mille, tranquille…)
        let il = |out: &mut Vec<u32>, i: &mut isize| {
            out.push(I_);
            out.push(L_);
            *i = ii + 2;
        };
        if ll && (ii == 0 || ii + 2 == l) {
            return il(out, i);
        }
        if c.w == b"mille" || (ll && ii >= 3 && c.st(ii - 3, self.s(0x1003f3a4))) {
            return il(out, i);
        }
        if ll && n != 0 {
            if ii > 3 && (c.st(ii - 4, b"fibr") || c.st(ii - 4, b"dist") || c.st(ii - 4, b"inst")) {
                return il(out, i);
            }
            if (ii > 2 && c.st(ii - 3, b"tit")) || c.st(ii + 1, b"llair") || c.st(ii + 1, b"llarit") {
                return il(out, i);
            }
            if c.ats(ii, &[b"tranquill", b"mill\xe9", b"morill", b"pupill", b"capill", b"pusill"]) {
                return il(out, i);
            }
            let ij: [&[u8]; 8] =
                [b"chevill", b"bouvill", b"gravill", b"pavill", b"recroquevill", b"\xe9couvill", b"sourcill", b"vacill"];
            if c.ats(ii, &ij) {
                out.push(I_);
                out.push(J_);
                *i = ii + 2;
                return;
            }
        }
        if !ll {
            if !c.st(ii + 1, b"l") || ii == 0 {
                let n2 = g(ii + 2);
                if c.st(ii + 1, b"n") && !c.vow(n2) && !matches!(n2, b'p' | b'b' | b'm' | b'n' | b'h') {
                    out.push(EN);
                    *i = ii + 1;
                } else if c.st(ii + 1, b"mp") || c.st(ii + 1, b"mb") {
                    out.push(EN);
                    *i = ii + 1;
                } else if c.st(ii + 1, b"e") && c.end(ii + 1) {
                    out.push(I_);
                    *i = ii + 1;
                } else {
                    if ii > 1 && n > 1 {
                        if c.at(ii, b"iement") && out[n - 1] & 3 == 2 {
                            out.push(I_);
                            *i = ii + 1;
                            return;
                        }
                        let u3 = out[n - 2];
                        let two = u3 & 3 == 2
                            && ((u3 & 0x1e3 != 0xc2 && g(ii - 2) != b'r' && p == b'r')
                                || (u3 & 0x1e3 != 0xc2 && g(ii - 2) != b'l' && p == b'l'))
                            && c.vow(g(ii + 1));
                        if two {
                            out.push(I_);
                            out.push(J_);
                            return;
                        }
                    }
                    let n1 = g(ii + 1);
                    if !c.vow(n1) || (n1 == b'e' && ii == l - 1) {
                        if ii != 0 && p == b'e' {
                            return;
                        }
                        out.push(I_);
                    } else {
                        out.push(J_);
                    }
                }
            } else if !c.vow(p) || !c.end(ii + 1) {
                out.push(I_);
            } else {
                out.push(J_);
                *i = ii + 1;
            }
            return;
        }
        if ii == 0 || !matches!(p, b'v' | b'g' | b'h' | b'l' | b'w' | b'x' | b'c') {
            if (c.st(ii + 1, b"llier") || c.st(ii + 1, self.s(0x1003f2e4))) && !c.at(ii, self.s(0x1003f2dc)) {
                out.push(I_);
                out.push(J_);
                *i = ii + 3;
                return;
            }
            if n != 0 && out[n - 1] & 3 == 2 {
                out.push(I_);
                if c.st(ii + 1, b"lli") {
                    out.push(L_);
                    *i = ii + 2;
                    return;
                }
            }
            out.push(J_);
            *i = ii + 2;
            return;
        }
        il(out, i)
    }

    fn letter_o(&self, c: &Ctx, out: &mut Vec<u32>, i: &mut isize) {
        let ii = *i;
        let l = c.l;
        let g = |k: isize| c.g(k);
        let p = g(ii - 1);
        if !((!c.prefix(ii) && !c.prefix(ii - 1)) || ii == l || (ii == l - 1 && g(ii + 1) == b's')) {
            out.push(OA);
            return;
        }
        if ii != 0 && c.at(ii, b"moell") {
            out.push(W_);
            out.push(A);
            *i = ii + 1;
            return;
        }
        if c.st(ii + 1, b"ue") && c.end(ii + 2) {
            out.push(U_);
            *i = ii + 2;
            return;
        }
        if c.st(ii + 1, b"in") && !c.vow(g(ii + 3)) {
            out.push(W_);
            return;
        }
        if c.st(ii + 1, b"ix") && c.end(ii + 2) {
            out.push(W_);
            out.push(A);
            *i = ii + 2;
            return;
        }
        if c.st(ii + 1, b"i") || c.st(ii + 1, b"\xee") || c.st(ii + 1, b"\xea") {
            out.push(W_);
            out.push(A);
            *i = ii + 1;
            return;
        }
        let y = c.st(ii + 1, b"y");
        if y && !c.end(ii + 1) && c.vow(g(ii + 2)) {
            out.push(W_);
            out.push(A);
            out.push(J_);
            *i = ii + 1;
            return;
        }
        if y && !c.end(ii + 1) && c.cons(g(ii + 2)) {
            out.push(OA);
            out.push(J_);
            *i = ii + 1;
            return;
        }
        if y && c.end(ii + 1) {
            out.push(W_);
            out.push(A);
            *i = ii + 1;
            return;
        }
        let n2 = g(ii + 2);
        if c.st(ii + 1, b"n") && !c.vow(n2) && !matches!(n2, b'h' | b'm' | b'n') {
            out.push(ON);
            *i = ii + 1;
            return;
        }
        if (c.st(ii + 1, b"m") && matches!(n2, b'b' | b'p' | b't')) || (c.at(ii, b"nom") && c.end(ii + 1)) {
            out.push(ON);
            *i = ii + 1;
            return;
        }
        if c.st(ii + 1, b"eu") {
            out.push(OEH);
            *i = ii + 2;
            return;
        }
        if c.st(ii + 1, b"ei") {
            out.push(OEH);
            *i = ii + 1;
            return;
        }
        if c.st(ii + 1, b"e") {
            if c.at(ii, b"coe") && ii == 1 {
                out.push(OA);
            } else {
                out.push(E_);
                *i = ii + 1;
            }
            return;
        }
        if c.st(ii + 1, b"uill") {
            out.push(U_);
            *i = ii + 1;
            return;
        }
        if !c.st(ii + 1, b"u") && !c.st(ii + 1, b"\xfb") {
            if g(ii + 1) == g(ii) {
                if ii != 0 && !c.ats(ii, &[b"coop", b"microo", b"coor", b"zoo"]) {
                    return;
                }
            } else {
                if c.end(ii) {
                    out.push(O_);
                    return;
                }
                if (c.st(ii + 1, b"t") || c.st(ii + 1, b"d") || c.st(ii + 1, b"z") || c.st(ii + 1, b"h")) && c.end(ii + 1) {
                    out.push(O_);
                    return;
                }
            }
            out.push(OA);
            return;
        }
        // ou, oû
        if !c.cons(n2) && !c.end(ii + 1) {
            if n2 == b'h' && c.end(ii + 2) {
                out.push(U_);
            } else if ii < 2 || !c.cons(g(ii - 2)) || (p != b'l' && p != b'r') {
                out.push(W_);
                *i = ii + 1;
            } else {
                out.push(U_);
                *i = ii + 1;
            }
            return;
        }
        out.push(U_);
        *i = if n2 != b'x' { ii + 1 } else { ii + 2 };
    }

    fn letter_u(&self, c: &Ctx, out: &mut Vec<u32>, i: &mut isize) {
        let ii = *i;
        let l = c.l;
        let g = |k: isize| c.g(k);
        let p = g(ii - 1);
        let n1 = g(ii + 1);
        if ii + 1 < l && c.st(ii + 1, b"ei") {
            out.push(OEH);
            *i = ii + 1;
            return;
        }
        let final_ui = ii != 0 && n1 == b'i' && c.end(ii + 1);
        if final_ui && c.cons(p) && p != b'q' && p != b'g' {
            out.push(HY);
            return;
        }
        // 0x10012163
        if c.at(ii, b"rui") {
            out.push(HY);
            return;
        }
        if c.st(ii + 1, b"y") && ii < l && c.vow(g(ii + 2)) {
            out.push(HY);
            out.push(I_);
            out.push(J_);
            *i = ii + 1;
            return;
        }
        let endings: [&[u8]; 9] = [b"erai", b"era", b"erons", b"erez", b"eront", b"erait", b"erions", b"eriez", b"eraient"];
        if endings.iter().any(|e| c.st(ii + 1, e) && c.end(ii + e.len() as isize)) {
            out.push(Y_);
            *i = ii + 1;
            return;
        }
        if !c.vow(n1) || (n1 == b'e' && c.end(ii + 1)) {
            if c.st(ii + 1, b"n") && ii + 1 <= l && !c.vow(g(ii + 2)) {
                out.push(OEN);
                *i = ii + 1;
            } else if c.st(ii + 1, b"m") && c.end(ii + 1) {
                out.push(OA);
            } else if !c.at(ii, b"triumv") {
                out.push(Y_);
            } else {
                out.push(OA);
            }
            return;
        }
        if ii < 2 || !c.cons(g(ii - 2)) || (p != b'l' && p != b'r') {
            out.push(HY);
        } else {
            out.push(Y_);
        }
    }

    /// 0x1000f180: the stress, and the glides between consonants.
    fn tras_post(&self, words: &[Word], idx: usize, out: &mut [u32]) {
        let r = &words[idx];
        let mut grade = r.f[GRADE];
        let bound = r.f[BOUND];
        let n = out.len();
        // before a hyphen (class 0x37; `*(fon+0x44)+4` is 1 for every
        // French voice) a short first part of a compound loses its stress
        // unless it has two vowels and three consonants
        if words.get(idx + 1).is_some_and(|nx| nx.f[CLASS] == 0x37) && r.f[SUB] != b'z' {
            let vowels = out.iter().filter(|&&c| is_vowel_code(c)).count();
            let consonants = out.iter().filter(|&&c| is_cons_code(c)).count();
            if !(vowels > 1 && consonants > 2) {
                grade = 3;
            }
        }
        let at = |out: &[u32], k: usize| out.get(k).copied().unwrap_or(0);
        // the previous record that is not joined by 0x16 (none before the
        // first)
        let mut prev: Option<usize> = idx.checked_sub(1);
        let mut k = idx;
        while k > 1 {
            match prev {
                Some(p) if words[p].f[BOUND] == 0x16 => {
                    k -= 1;
                    prev = Some(k - 1);
                }
                _ => break,
            }
        }
        let mut j = 0usize;
        if pause_after(bound) {
            while at(out, j) != 0 && !is_vowel_code(at(out, j)) {
                j += 1;
            }
            if at(out, j) == 0 {
                while j != 0 && at(out, j) & 0x1c3 != 0x101 && at(out, j) & 0x1e3 != 0xc2 {
                    j -= 1;
                }
                match prev {
                    None => {
                        if j != 0 && out[j] == SCHWA && bound != 0 && r.text != b"de" {
                            out[j] = 0x10761;
                            return;
                        }
                    }
                    Some(p) if j != 0 => {
                        let pb = words[p].f[BOUND];
                        let v = out[j];
                        if v == SCHWA && pb != 0 && r.text != b"de" {
                            out[j] = 0x10761;
                            return;
                        }
                        if v == J_ && pb != 0 {
                            out[j] = 0x107c1;
                            return;
                        }
                        if v == W_ && pb != 0 {
                            out[j] = 0x103e1;
                            return;
                        }
                    }
                    _ => {}
                }
                if j == 0 && at(out, 0) == SCHWA && at(out, 1) == 0 {
                    out[0] = 0x10761;
                    return;
                }
            }
        }
        if idx == 0 {
            while at(out, j) != 0 && !is_vowel_code(at(out, j)) {
                j += 1;
            }
            if at(out, j) == 0 {
                while j != 0 && at(out, j) & 0x1c3 != 0x101 && at(out, j) & 0x1e3 != 0xc2 {
                    j -= 1;
                }
                if j == 0 && at(out, 0) == SCHWA && at(out, 1) == 0 && bound != 0 && r.f[CLASS] != 0x37 {
                    out[0] = 0x10761;
                    return;
                }
            }
        }
        // a glide between consonants is a vowel
        let mut g = 0usize;
        while at(out, g) != 0 && at(out, g) & 0x1e3 != 0xc2 {
            g += 1;
        }
        let v = at(out, g);
        if v != 0 && at(out, g + 1) & 3 != 1 && g != 0 && out[g - 1] & 3 != 1 {
            if v == J_ {
                out[g] = I_;
            } else if v == W_ {
                out[g] = U_;
            }
        }
        if r.text == b"et" && r.f[SUB] == b'z' {
            grade = 3;
        }
        if grade == 3 || grade == 8 {
            return;
        }
        let mut k = n;
        while k != 0 && !is_vowel_code(at(out, k)) {
            k -= 1;
        }
        let v = at(out, k);
        if is_vowel_code(v) {
            let stressed = match v {
                0x741 | 0x3e1 | 0x241 | 0x2e1 | 0x361 | 0x441 | 0x6c1 | 0x6e1 | 0x10000241 | 0x761 | 0x7c1 | 0x7e1 | 0x10000361
                | 0x100006c1 | 0x100006e1 => Some(v | 0x10000),
                _ => None,
            };
            if let Some(s) = stressed {
                out[k] = s;
            }
        }
    }
}
