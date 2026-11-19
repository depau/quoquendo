//! `FonTrasParola` (0x1000a0e0) of LTTS7Greek.dll: a word's phone codes,
//! from its dictionary `fonetica` when it has one, else from its letters:
//! the spelling as Greek letters (0x10009910: words mostly in Latin letters
//! transliterated, an unstressed word's vowels without their accent), the
//! vowel and consonant digraphs collapsed, then each letter in context.

use super::{b, col, Greek, Lex, ACCENTED, CONSONANT, PUNCT, VOWEL};
use crate::lang::common::{field::*, Word};

/// Character sets the rules test (`strchr` on these strings).
mod set {
    /// voiced consonants `γδζλμνρ` (0x10022fdc)
    pub const VOICED: u32 = 0x10022fdc;
    /// `τ` (0x10022fd8), `ζ` (0x10022fd4)
    pub const TAU: u32 = 0x10022fd8;
    pub const ZETA: u32 = 0x10022fd4;
    /// `εέ` (0x10022f9c)
    pub const E: u32 = 0x10022f9c;
    /// the i vowels `ΐήίηιϊΰϋ` (0x10022fa8)
    pub const I: u32 = 0x10022fa8;
    /// `υύ` (0x10022fa4), `ΰύ` (0x10022fa0)
    pub const Y: u32 = 0x10022fa4;
    pub const Y_ACUTE: u32 = 0x10022fa0;
    /// `γκ` (0x10022f98)
    pub const VELAR: u32 = 0x10022f98;
    /// `βσφ` (0x10022f94)
    pub const LABIAL_FRIC: u32 = 0x10022f94;
    /// `ο` (0x100209ac, a letter name the compiler shares)
    pub const O: u32 = 0x100209ac;
    /// `ποιος`, `ποιες`, `ποια`, `ποιο` (0x10022fcc, 0x10022fc4,
    /// 0x10022fbc, 0x10022fb4)
    pub const POIOS: u32 = 0x10022fcc;
    pub const POIES: u32 = 0x10022fc4;
    pub const POIA: u32 = 0x10022fbc;
    pub const POIO: u32 = 0x10022fb4;
}

/// Word-start lists (`ELQSearchString` flag 0x12: an entry the word starts
/// with), as (table, count).
mod list {
    /// i before a vowel said as a glide (`έμοιαζε`, `ίδιο`…)
    pub const GLIDE: (u32, u32) = (0x10022c68, 0xf);
    /// i before an accented vowel said in full (`αμιάντου`…)
    pub const HIATUS: (u32, u32) = (0x10022ca8, 0x29);
    /// `λι` + vowel palatal (`έλιωσε`, `ήλιο`…)
    pub const LI: (u32, u32) = (0x10022d58, 0x1b);
    /// `μπ` said m p (`ιμπρέζα`…)
    pub const MP: (u32, u32) = (0x10022dc4, 6);
    /// `μπ` keeps its m (`γκίμπσον`…)
    pub const MB: (u32, u32) = (0x10022de0, 0x11);
    /// `νι` + vowel palatal (`ένοιες`…)
    pub const NI: (u32, u32) = (0x10022e28, 0x20);
    /// `κοινωνιών` (0x10022ca4)
    pub const KOINONION: (u32, u32) = (0x10022ca4, 1);
    /// `ντ` said n t (`αντέν`…)
    pub const NT: (u32, u32) = (0x10022ea8, 0x25);
    /// `ντ` keeps its n (`έντουαρντς`…)
    pub const ND: (u32, u32) = (0x10022f40, 0x11);
}

impl Greek {
    fn in_set(&self, set: u32, c: u8) -> bool {
        // strchr(set, c) for c != 0 (every call guards c == 0 or has a
        // letter there)
        c != 0 && self.s(set).contains(&c)
    }

    /// `ELQSearchString(word, table, 0x12, count) != -1` (LTTS7Util
    /// 0x10028ca0): a linear search for an entry the word starts with.
    fn starts_with_any(&self, word: &[u8], (table, count): (u32, u32)) -> bool {
        (0..count).any(|k| word.starts_with(self.s(self.pe.u32(table + 4 * k))))
    }

    /// 0x10009720 (ECX `buf`, EAX `pos`): `pat` placed in `buf` so that its
    /// first letter equal to `buf[pos]` lies at `pos`, compared letter by
    /// letter with `buf` without accents (0x10009640).
    fn fits(&self, buf: &[u8], pos: usize, pat: u32) -> bool {
        let pat = self.s(pat);
        let k = pat.iter().position(|&c| c == b(buf, pos as isize)).unwrap_or(pat.len());
        if pos < k {
            return false;
        }
        let start = pos - k;
        (0..pat.len()).all(|j| self.plain(b(buf, (start + j) as isize)) == pat[j])
    }

    /// 0x10009640: a vowel without its accent (upper case kept).
    pub(super) fn plain(&self, c: u8) -> u8 {
        if c == 0 {
            return 0;
        }
        let up = self.has(c, super::UPPER);
        let l = if up { self.lower(c) } else { c };
        let p = match l {
            0xc0 => 0xfa,
            0xdc => 0xe1,
            0xdd => 0xe5,
            0xde => 0xe7,
            0xdf => 0xe9,
            0xe0 => 0xfb,
            0xfc => 0xef,
            0xfd => 0xf5,
            0xfe => 0xf9,
            l => l,
        };
        if up {
            self.chars[p as usize].1
        } else {
            p
        }
    }

    /// 0x10009910 (the FON channel's byte +0x44 is 1: no `\#Ge` tag): a
    /// word's letters in lower case (at most 0x3f); a word mostly in Latin
    /// lower-case letters (more than half, or one of two) transliterated
    /// into Greek ones, otherwise its vowels without their accent when
    /// `grade` is 3. At most 0x3a letters come out.
    pub(super) fn greek_letters(&self, word: &[u8], grade: u8) -> Vec<u8> {
        let low: Vec<u8> = word.iter().take(0x3f).map(|&c| if self.has(c, super::UPPER) { self.lower(c) } else { c }).collect();
        let latin = word.iter().take(0x3f).filter(|c| c.is_ascii_lowercase()).count();
        let n = low.len();
        let is_latin = latin > n / 2 || (n == 2 && latin == 1);
        let mut out = Vec::new();
        let at = |k: usize| low.get(k).copied().unwrap_or(0);
        let one = n == 1 && grade != 3;
        let mut k = 0usize;
        while k < n {
            if k > 0x3e || out.len() > 0x39 {
                break;
            }
            let c = low[k];
            if !is_latin {
                out.push(if grade == 3 && self.has(c, VOWEL) { self.plain(c) } else { c });
                k += 1;
                continue;
            }
            let thick = |c: u8| c != 0 && !self.has(c, CONSONANT);
            match c {
                b'a' => out.push(if one { 0xdc } else { 0xe1 }),
                b'b' => out.push(b'b'),
                b'c' if at(k + 1) == b'h' => {
                    out.push(0xf7);
                    k += 1;
                }
                b'c' => out.push(0xf3),
                b'd' if at(k + 1) == b'h' && thick(at(k + 2)) => {
                    out.push(0xe4);
                    k += 1;
                }
                b'd' => out.push(b'd'),
                b'e' => out.push(if one { 0xdd } else { 0xe5 }),
                b'f' => out.push(0xf6),
                b'g' => out.push(0xe3),
                b'h' => {
                    if k == 0 || !matches!(at(k - 1), b't' | b'd') || !thick(at(k + 1)) {
                        out.push(0xe7);
                    }
                }
                b'i' => out.push(if one { 0xdf } else { 0xe9 }),
                b'j' | b'x' => out.push(if c == b'j' { 0xee } else { 0xf7 }),
                b'k' if at(k + 1) == b's' => {
                    out.push(0xee);
                    k += 1;
                }
                b'k' | b'q' => out.push(0xea),
                b'l' => out.push(0xeb),
                b'm' => out.push(0xec),
                b'n' => out.push(0xed),
                b'o' => out.push(if one { 0xfc } else { 0xef }),
                b'p' => out.push(0xf0),
                b'r' => out.push(0xf1),
                b's' => out.push(if at(k + 1) != 0 { 0xf3 } else { 0xf2 }),
                b't' if at(k + 1) == b'h' && thick(at(k + 2)) => {
                    out.push(0xe8);
                    k += 1;
                }
                b't' => out.push(0xf4),
                b'u' | b'y' => out.push(if one { 0xfd } else { 0xf5 }),
                b'v' => out.push(0xe2),
                b'w' => out.push(0xf9),
                b'z' => out.push(0xe6),
                0xe1 => out.push(0xdc),
                0xe9 => out.push(0xdd),
                0xed => out.push(0xdf),
                0xf3 => out.push(0xfc),
                0xfa | 0xfd => out.push(0xfd),
                _ => {}
            }
            k += 1;
        }
        out
    }

    /// `FonTrasParola` (0x1000a0e0) for record `i` of the chunk `w`.
    pub(super) fn tras_parola(&self, w: &[Word], i: usize) -> Vec<u32> {
        let r = &w[i];
        if r.text.is_empty() {
            return Vec::new();
        }
        // a row with `fonetica` (0x10009450 leaves it current)
        let mut lex = Lex::of(r);
        let mut cur = lex.head();
        while let Some(row) = cur {
            if let Some(f) = self.col(row, col::FONETICA).filter(|f| !f.is_empty()) {
                return self.parse_phones(f);
            }
            cur = lex.next();
        }
        let orig = self.greek_letters(&r.text, r.f[GRADE]);
        let word = self.collapse(&orig);
        self.letters(&orig, &word)
    }

    /// 0x10009840: a dictionary pronunciation (symbols separated by spaces
    /// or `-`, at most 5 characters each) as codes (0x100097b0).
    fn parse_phones(&self, text: &[u8]) -> Vec<u32> {
        let mut out = Vec::new();
        let mut i = 0usize;
        let at = |i: usize| text.get(i).copied().unwrap_or(0);
        while at(i) != 0 {
            let mut sym = Vec::new();
            while !matches!(at(i), b' ' | b'-' | 0) && sym.len() < 5 {
                sym.push(at(i));
                i += 1;
            }
            if at(i) != 0 {
                i += 1;
            }
            out.push(self.phone_code(&sym));
        }
        out
    }

    /// The digraphs of 0x1000a0e0's first pass: `αι` → ε (`αί` → έ), `ει`
    /// → ι, `οι` → η (`οί` → ή), `υι` → ι (not after ο), `αυ ευ ηυ` → αφ/αβ
    /// (φ before a voiceless consonant or at the end; `αύ` accents the α), a
    /// doubled consonant (but γγ, νν) once.
    fn collapse(&self, w: &[u8]) -> Vec<u8> {
        let n = w.len() as isize;
        let at = |k: isize| b(w, k);
        let mut out: Vec<u8> = Vec::new();
        // f / v after α ε η: a voiceless consonant follows (not before τζ),
        // or the υ ends the word
        let voiceless = |k: isize| {
            let nx = at(k + 1);
            ((nx == 0 || (!self.in_set(set::VOICED, nx) && (nx == 0 || !self.in_set(set::TAU, nx) || at(k + 2) == 0 || !self.in_set(set::ZETA, at(k + 2)))))
                && self.has(nx, CONSONANT))
                || k == n - 1
        };
        let mut k = 0isize;
        while at(k) != 0 {
            let c = at(k);
            let nx = at(k + 1);
            match c {
                0xe1 if nx == 0xe9 => {
                    out.push(0xe5);
                    k += 1;
                }
                0xe1 if nx == 0xdf => {
                    out.push(0xdd);
                    k += 1;
                }
                0xe5 if nx == 0xe9 || nx == 0xdf => {
                    out.push(nx);
                    k += 1;
                }
                0xef if nx == 0xe9 => {
                    out.push(0xe7);
                    k += 1;
                }
                0xef if nx == 0xdf => {
                    out.push(0xde);
                    k += 1;
                }
                0xf5 | 0xfd if k != 0 && matches!(at(k - 1), 0xe1 | 0xe5 | 0xe7) => {
                    if c == 0xfd {
                        let last = out.len() - 1;
                        out[last] = super::les::greek_acute(at(k - 1));
                    }
                    if voiceless(k) {
                        out.push(0xf6);
                        if nx == 0xf6 {
                            k += 1;
                        }
                    } else {
                        out.push(0xe2);
                        if nx == 0xe2 {
                            k += 1;
                        }
                    }
                }
                0xf5 if (nx == 0xe9 || nx == 0xdf) && !(k != 0 && at(k - 1) == 0xef) => {
                    out.push(nx);
                    k += 1;
                }
                0xe1 | 0xe5 | 0xef | 0xf5 | 0xfd => out.push(c),
                _ => {
                    out.push(c);
                    if self.has(c, CONSONANT) && c != 0xe3 && c != 0xed && nx == c {
                        k += 1;
                    }
                }
            }
            k += 1;
        }
        out
    }

    /// The letter rules of 0x1000a0e0 (`orig`, the word before the digraphs
    /// were collapsed, is what the word lists and 0x10009720 look at, with
    /// the collapsed word's positions).
    fn letters(&self, orig: &[u8], w: &[u8]) -> Vec<u32> {
        // local_f0: the index of the word's last letter (strlen - 1)
        let len = w.len() as isize - 1;
        let at = |k: isize| b(w, k);
        let o = |c: u8| c != 0 && self.in_set(set::O, c);
        // an i vowel at `p`: `ΐήίηιϊΰϋ`, or `υ ύ` not after ο
        let ivowel = |p: isize| {
            let c = at(p);
            c != 0 && (self.in_set(set::I, c) || (self.in_set(set::Y, c) && (p == 0 || at(p - 1) == 0 || !o(at(p - 1)))))
        };
        let starts = |l: (u32, u32)| self.starts_with_any(orig, l);
        let mut out: Vec<u32> = Vec::new();
        let mut mb_initial = false; // local_ea: the word starts with μπ
        let mut nd_initial = false; // local_e9: the word starts with ντ
        let mut nn = false; // local_eb: after νν
        let mut i = 0isize;
        while at(i) != 0 {
            let c = at(i);
            let n1 = at(i + 1);
            let prev = at(i - 1);
            if self.has(c, PUNCT) {
                match c {
                    b'"' | b'\'' => {}
                    b'#' | b',' => out.push(0x13),
                    _ => out.push(0x17),
                }
                i += 1;
                continue;
            }
            match c {
                b'b' => out.push(0x1fe2),
                b'd' => out.push(0x1be2),
                0xc0 | 0xde | 0xdf | 0xe0 | 0xfd => out.push(0x107c1),
                0xdc => out.push(0x10441),
                0xdd => out.push(0x10741),
                0xe1 => out.push(0x441),
                0xe2 => out.push(0x1d42),
                0xe3 => {
                    // γ
                    if n1 != 0 && self.in_set(set::E, n1) {
                        out.push(0x1142);
                    } else if n1 != 0 && ivowel(i + 1) {
                        out.push(0x1142);
                        if !self.has(n1, ACCENTED) && i + 1 != len && (len < i + 2 || !self.has(at(i + 2), CONSONANT)) {
                            i += 1;
                        }
                    } else if i == 0 && at(1) != 0 && self.in_set(set::VELAR, at(1)) {
                        out.push(0xfe2);
                        if len >= 2 && at(2) != 0 && self.in_set(set::E, at(2)) {
                            out.push(0x10c2);
                        }
                        i = 1;
                    } else if n1 != 0 && self.in_set(set::VELAR, n1) {
                        if prev != 0xed {
                            out.push(0xfc2);
                        }
                        out.push(0xfe2);
                        if at(i + 2) != 0 && self.in_set(set::E, at(i + 2)) && len >= i + 2 {
                            out.push(0x10c2);
                        }
                        i += 1;
                    } else if n1 == 0xf7
                        && i + 2 <= len
                        && at(i + 2) != 0
                        && (self.in_set(set::I, at(i + 2)) || (self.in_set(set::Y, at(i + 2)) && !o(0xf7)) || self.in_set(set::E, at(i + 2)))
                    {
                        if i != 0 && prev != 0xed {
                            out.push(0x19c2);
                        }
                    } else if n1 == 0xf7 || n1 == 0xee {
                        if i != 0 && prev != 0xed {
                            out.push(0xfc2);
                        }
                    } else {
                        out.push(0xec2);
                    }
                }
                0xe4 => out.push(0x1ac2),
                0xe5 => out.push(0x741),
                0xe6 => out.push(0x1942),
                0xe7 | 0xe9 | 0xf5 | 0xfa | 0xfb => {
                    let glide = if self.has(n1, VOWEL) && starts(list::GLIDE) {
                        true
                    } else if self.has(n1, ACCENTED) && starts(list::HIATUS) {
                        false
                    } else if i + 2 <= len && (self.fits(orig, i as usize, set::POIOS) || self.fits(orig, i as usize, set::POIES)) {
                        true
                    } else if i + 1 <= len && (self.fits(orig, i as usize, set::POIA) || self.fits(orig, i as usize, set::POIO)) {
                        true
                    } else {
                        let full = (n1 != 0 && ivowel(i + 1))
                            || !self.has(n1, VOWEL)
                            || (!self.has(n1, ACCENTED) && (len < i + 2 || n1 == 0 || !o(n1) || at(i + 2) == 0 || !self.in_set(set::Y_ACUTE, at(i + 2))))
                            || (i != 0 && prev != 0 && ivowel(i - 1))
                            || len <= i;
                        !full
                    };
                    out.push(if glide { 0x10c2 } else { 0x7c1 });
                }
                0xe8 => out.push(0x1b52),
                0xea => {
                    out.push(0xff2);
                    if n1 != 0 && self.in_set(set::E, n1) {
                        out.push(0x10c2);
                    }
                }
                0xeb => {
                    // λ
                    let a = n1 != 0xe9 || len < i + 2 || ivowel(i + 2);
                    let b2 = at(i + 2);
                    let bb = ((!self.has(b2, ACCENTED) && (len < i + 3 || b2 == 0 || !o(b2) || at(i + 3) == 0 || !self.in_set(set::Y_ACUTE, at(i + 3))))
                        || i == 0
                        || self.has(prev, CONSONANT))
                        && (i != 0 || !self.has(at(2), VOWEL));
                    if !starts(list::LI) && (a || bb) {
                        out.push(0x18a2);
                    } else {
                        out.push(0x10a2);
                        out.push(0x10c2);
                        i += 1;
                    }
                }
                0xec => {
                    // μ
                    if n1 == 0xf0 && starts(list::MP) {
                        out.push(0x1fc2);
                        out.push(0x1ff2);
                        i += 1;
                    } else if n1 != 0 && self.in_set(set::LABIAL_FRIC, n1) {
                        out.push(0x1dc2);
                    } else if i == 0 && at(1) == 0xf0 {
                        mb_initial = true;
                    } else if n1 != 0xf0 || (!mb_initial && (!self.has(prev, CONSONANT) || prev == 0xec) && i + 1 != len && !starts(list::MB)) {
                        out.push(0x1fc2);
                    }
                }
                0xed => self.nu(&mut out, w, orig, &mut i, &mut nd_initial, &mut nn, ivowel),
                0xee => {
                    out.push(0xff2);
                    out.push(0x1952);
                }
                0xf2 => out.push(0x1952),
                0xef => {
                    if n1 == 0xf5 {
                        out.push(0x3e1);
                        i += 1;
                    } else if n1 == 0xfd {
                        out.push(0x103e1);
                        i += 1;
                    } else {
                        out.push(0x361);
                    }
                }
                0xf0 => out.push(if i != 0 && prev == 0xec && n1 != 0xf4 { 0x1fe2 } else { 0x1ff2 }),
                0xf1 => out.push(0x19a2),
                0xf3 => {
                    let s = n1 == 0 || (!self.in_set(set::VOICED, n1) && (n1 == 0 || !self.in_set(set::TAU, n1) || at(i + 2) == 0 || !self.in_set(set::ZETA, at(i + 2)))) || n1 == 0xeb;
                    out.push(if s { 0x1952 } else { 0x1942 });
                }
                0xf4 => {
                    if n1 == 0xe6 {
                        out.push(0x1902);
                        i += 1;
                    } else if n1 == 0xf3 || n1 == 0xf2 {
                        out.push(0x1912);
                        i += 1;
                    } else if i != 0 && prev == 0xed {
                        out.push(0x1be2);
                    } else {
                        out.push(0x1bf2);
                    }
                }
                0xf6 => out.push(0x1d52),
                0xf7 => out.push(0xf52),
                0xf8 => {
                    out.push(0x1ff2);
                    out.push(0x1952);
                }
                0xf9 => out.push(0x361),
                0xfc | 0xfe => out.push(0x10361),
                _ => {}
            }
            i += 1;
        }
        out
    }

    /// ν in 0x1000a0e0.
    #[allow(clippy::too_many_arguments)]
    fn nu(&self, out: &mut Vec<u32>, w: &[u8], orig: &[u8], i: &mut isize, nd_initial: &mut bool, nn: &mut bool, ivowel: impl Fn(isize) -> bool) {
        let len = w.len() as isize - 1;
        let at = |k: isize| b(w, k);
        let starts = |l: (u32, u32)| self.starts_with_any(orig, l);
        let n1 = at(*i + 1);
        let prev = at(*i - 1);
        let palatal_listed = starts(list::NI) && len >= *i + 2 && n1 != 0 && ivowel(*i + 1) && self.has(at(*i + 2), VOWEL) && !self.has(at(*i + 2), ACCENTED);
        if palatal_listed {
            out.push(0x11c2);
            out.push(0x10c2);
            *i += 1;
        } else if starts(list::KOINONION) {
            out.push(0x19c2);
        } else if *i != 0 && n1 == 0xf4 && starts(list::NT) {
            out.push(0x19c2);
            out.push(0x1bf2);
            *i += 1;
        } else if *i == 0 && at(1) == 0xf4 {
            *nd_initial = true;
            return;
        } else if n1 != 0xf4 || (!*nd_initial && (!self.has(prev, CONSONANT) || prev == 0xed) && *i + 1 != len && !starts(list::ND)) {
            let i2 = at(*i + 2);
            let pal = n1 != 0
                && ivowel(*i + 1)
                && !self.has(n1, ACCENTED)
                && *i + 2 <= len
                && (i2 == 0 || !(self.in_set(set::I, i2) || (self.in_set(set::Y, i2) && !self.in_set(set::O, n1))))
                && self.has(i2, VOWEL)
                && (self.has(i2, ACCENTED) || *i == 0 || *nn);
            if (n1 != 0 && self.in_set(set::VELAR, n1)) || n1 == 0xf7 {
                out.push(0xfc2);
            } else if pal {
                out.push(0x11c2);
                out.push(0x10c2);
                *i += 1;
            } else if n1 != 0xed {
                out.push(0x19c2);
            } else {
                *nn = true;
            }
        } else {
            return;
        }
        // LAB_1000b188
        if *i != 0 && at(*i - 1) == 0xed {
            *nn = false;
        }
    }
}
