//! FonTrasParola (0x10010f60): one word to phone codes.
//!
//! The word (stress marks stripped, 0x1000e780) is first looked up in the
//! dictionary; a non-empty `fonetica` is converted symbol by symbol
//! (0x1000e890). Otherwise the word (stress marks stripped only at grade 3,
//! 0x1000e960) may start with a spelling of the word-start table at
//! 0x1003f098 (0x10010e20, flag 2: the entry a prefix), whose phonetic
//! string gives the first codes, and every remaining letter is transcribed
//! by its left and right context.

use super::{Polish, CONSONANT, PUNCT, VOWEL};
use crate::lang::common::Word;

/// `{ spelling, phonetic }` pairs tried at the start of the word.
const WORD_START: u32 = 0x1003f098;
const WORD_START_N: u32 = 0x29;
/// `ELQSearchString` tables of string pointers (flags 0x12: an entry that
/// starts the word): `s` before `i` read /s/ (`sinus`…).
const S_SOFT: (u32, u32) = (0x1003efb4, 0xe);
/// `rz` read /r z/ (`marzn`, `mierzł`…).
const RZ_SPLIT: (u32, u32) = (0x1003eff0, 0xc);
/// `u` after `a`/`e` read as the glide (`au`, `eur`…).
const U_GLIDE: (u32, u32) = (0x1003ef98, 6);

// Context sets, as the code's own `strncmp` operand lists.
/// Voiceless obstruents.
const VOICELESS: &[&[u8]] = &[b"p", b"t", b"k", b"c", b"\xe6", b"cz", b"f", b"s", b"sz", b"\x9c", b"ch", b"h", b"x"];
/// Voiced obstruents (and `w`).
const VOICED: &[&[u8]] = &[b"b", b"d", b"g", b"dz", b"d\x9f", b"dzi", b"d\xbf", b"w", b"z", b"rz", b"\xbf", b"\x9f", b"zi"];
/// Sonorants.
const SONORANT: &[&[u8]] = &[b"m", b"n", b"r", b"l", b"\xb3", b"j", b"w"];
/// Fricatives after a nasal vowel.
const FRICATIVE: &[&[u8]] = &[
    b"w", b"f", b"s", b"z", b"sz", b"rz", b"\xbf", b"\x9c", b"\x9f", b"ch", b"h", b"si", b"s\xed", b"zi", b"z\xed", b"v",
];
/// After `n`: the sibilants and `h`.
const N_SIBILANT: &[&[u8]] = &[b"s", b"z", b"sz", b"rz", b"\xbf", b"\x9c", b"\x9f", b"ch", b"h", b"si", b"s\xed", b"zi", b"z\xed"];
/// Vowels before `j`.
const J_VOWEL: &[&[u8]] = &[b"a", b"e", b"o", b"y", b"u", b"i", b"\xe1", b"\xe9", b"\xf4", b"\xf3", b"\xfd", b"\xfa", b"\xed"];
/// Vowels before `ł` (no `ô`).
const L_VOWEL: &[&[u8]] = &[b"a", b"e", b"o", b"y", b"u", b"i", b"\xe1", b"\xe9", b"\xf3", b"\xfd", b"\xfa", b"\xed"];
/// Consonants after `j`.
const J_CONSONANT: &[&[u8]] = &[
    b"p", b"b", b"t", b"d", b"k", b"g", b"c", b"dz", b"\xe6", b"ci", b"c\xed", b"d\x9f", b"dzi", b"dz\xed", b"cz", b"d\xbf", b"w", b"f",
    b"s", b"z", b"sz", b"rz", b"\xbf", b"\x9c", b"si", b"s\xed", b"\x9f", b"zi", b"z\xed", b"h", b"ch", b"m", b"n", b"ni", b"n\xed",
    b"r", b"\xb3", b"l", b"j", b"x", b"v",
];
/// Vowels after `nci`/`ndzi`.
const NCI_VOWEL: &[&[u8]] = &[
    b"i", b"\xed", b"y", b"\xfd", b"e", b"\xe9", b"a", b"\xe1", b"o", b"\xf4", b"u", b"\xfa", b"\xf3", b"\xea", b"\xec", b"\xb9", b"\xe3",
];
/// After word-initial `z`: `ir`, `id`, … read /z/ + vowel.
const ZI: &[&[u8]] = &[
    b"ir", b"\xedr", b"id", b"\xedd", b"il", b"\xedl", b"i\xb3", b"\xed\xb3", b"in", b"\xedn", b"ig", b"\xedg", b"isz", b"\xedsz", b"i\x9c",
];
/// A following velar.
const VELAR: &[&[u8]] = &[b"k", b"g", b"jk", b"jg"];
/// A following palatal affricate.
const PALATAL: &[&[u8]] = &[b"\xe6", b"d\x9f", b"ci", b"dzi"];
/// The same with stressed `i`.
const PALATAL_I: &[&[u8]] = &[b"\xe6", b"d\x9f", b"ci", b"c\xed", b"dzi", b"dz\xed"];
/// A following dental.
const DENTAL: &[&[u8]] = &[b"t", b"d", b"c", b"dz", b"cz", b"d\xbf"];

/// The word, NUL padded, with the `strncmp` contexts.
struct Ctx<'a> {
    w: &'a [u8],
    /// The last index.
    n: usize,
}

impl Ctx<'_> {
    fn c(&self, k: usize) -> u8 {
        self.w.get(k).copied().unwrap_or(0)
    }

    /// `strncmp(word + k, pat, strlen(pat)) == 0`.
    fn eq(&self, k: usize, pat: &[u8]) -> bool {
        pat.iter().enumerate().all(|(j, &p)| self.c(k + j) == p)
    }

    fn any(&self, k: usize, set: &[&[u8]]) -> bool {
        set.iter().any(|p| self.eq(k, p))
    }

    /// Voiceless after `i`, or `my` ending the word right after `i`.
    fn voiceless_my(&self, k: usize, i: usize) -> bool {
        self.any(k, VOICELESS) || (self.eq(k, b"my") && i + 2 == self.n)
    }

    /// A voiced obstruent that voices a preceding one (`w` and the
    /// sonorants excluded, `rz` kept).
    fn voicing(&self, k: usize) -> bool {
        self.any(k, VOICED) && (!self.any(k, SONORANT) || self.eq(k, b"rz"))
    }
}

impl Polish {
    pub(super) fn tras_parola(&self, w: &[Word], i: usize) -> Vec<u32> {
        let rec = &w[i];
        let key = self.strip(&rec.text);
        if let Some(row) = self.table_find(&key) {
            if let Some(f) = row[1].as_deref() {
                if !f.is_empty() {
                    let mut out = Vec::new();
                    self.phonetic(f, &mut out);
                    return until_zero(out);
                }
            }
        }
        let word = if rec.f[0x12] == 3 { key } else { rec.text.clone() };
        self.letters(&word, rec.f[0xf])
    }

    /// 0x1000e780(3, 0) / 0x1000e960 at grade 3: the stress marks removed.
    fn strip(&self, s: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(s.len());
        for &c in s {
            if c == 0 {
                break;
            }
            out.push(if self.has(c, super::ACCENTED) { self.unstress(c) } else { c });
        }
        out
    }

    /// 0x1000e660: a stressed letter's plain one (through the lower case).
    fn unstress(&self, c: u8) -> u8 {
        let up = self.has(c, super::UPPER);
        let mut c = if up { self.lower(c) } else { c };
        c = match c {
            0xc3 => 0xa5,
            0xc9 => 0x45,
            0xcc => 0xca,
            0xcd => 0x49,
            0xd4 => 0x4f,
            0xda => 0x55,
            0xdd => 0x59,
            0xe1 => 0x61,
            0xe3 => 0xb9,
            0xe9 => 0x65,
            0xec => 0xea,
            0xed => 0x69,
            0xf4 => 0x6f,
            0xfa => 0x75,
            0xfd => 0x79,
            c => c,
        };
        if up {
            self.upper(c)
        } else {
            c
        }
    }

    /// 0x1000e890: a phonetic string of symbols separated by space or `-`
    /// (at most 5 characters each; a longer one loses its sixth) to codes
    /// through the phone table (0x1000e800: 0x10 for an unknown symbol).
    fn phonetic(&self, s: &[u8], out: &mut Vec<u32>) {
        let at = |k: usize| s.get(k).copied().unwrap_or(0);
        let mut k = 0;
        while at(k) != 0 {
            let start = k;
            while !matches!(at(k), b' ' | b'-' | 0) && k - start < 5 {
                k += 1;
            }
            let sym = &s[start..k];
            if at(k) != 0 {
                k += 1;
            }
            out.push(self.phone_code(sym));
        }
    }

    /// 0x1000e800.
    fn phone_code(&self, sym: &[u8]) -> u32 {
        if sym.is_empty() {
            return 0x10;
        }
        match self.phone_table().enumerate().find(|(_, (s, _))| *s == sym) {
            Some((k, (_, 0))) => {
                if k > 3 {
                    0x10
                } else {
                    0
                }
            }
            Some((_, (_, c))) => c,
            None => 0x10,
        }
    }

    /// `ELQSearchString(word, table, 0x12, count) != -1`.
    fn starts_with_any(&self, word: &[u8], (table, count): (u32, u32)) -> bool {
        (0..count).any(|k| word.starts_with(self.s(self.pe.u32(table + 4 * k))))
    }

    /// The letter-by-letter transcription.
    fn letters(&self, word: &[u8], sub: u8) -> Vec<u32> {
        let x = Ctx { w: word, n: word.len().wrapping_sub(1) };
        let n = x.n;
        let mut out = Vec::new();
        let mut i = 0;
        // 0x10010e20: the word-start table
        if let Some(k) = (0..WORD_START_N).find(|&k| word.starts_with(self.s(self.pe.u32(WORD_START + 8 * k)))) {
            self.phonetic(self.s(self.pe.u32(WORD_START + 8 * k + 4)), &mut out);
            out = until_zero(out);
            i = self.s(self.pe.u32(WORD_START + 8 * k)).len();
        }
        let prev = |i: usize| if i == 0 { 0 } else { x.c(i - 1) };
        let vowel = |c: u8| self.has(c, VOWEL);
        let cons = |c: u8| self.has(c, CONSONANT);
        while x.c(i) != 0 {
            let c = x.c(i);
            let k = i + 1;
            let mut emit = |v: &[u32]| out.extend_from_slice(v);
            if self.has(c, PUNCT) {
                match c {
                    b'"' | b'\'' => {}
                    b'#' | b',' => emit(&[0x13]),
                    _ => emit(&[0x17]),
                }
                i += 1;
                continue;
            }
            // a nasal vowel (ą ă ę ě): its code and the nasal's place
            let nasal = |v: u32, emit: &mut dyn FnMut(&[u32])| {
                if x.any(k, VELAR) {
                    emit(&[v, 0xfc2]);
                } else if x.any(k, PALATAL) {
                    emit(&[v, 0x2019c2]);
                } else if x.any(k, DENTAL) {
                    emit(&[v, 0x19c2]);
                } else if x.eq(k, b"p") || x.eq(k, b"b") {
                    emit(&[v, 0x1fc2]);
                } else {
                    emit(&[v, 0x101003e1]);
                }
            };
            // e é o ô: nasalised before n/ń or m + fricative
            let vn = |plain: u32, nasal: u32, emit: &mut dyn FnMut(&[u32])| {
                let nn = (x.eq(k, b"n") || x.eq(k, b"\xf1")) && x.any(k + 1, FRICATIVE);
                let mm = x.eq(k, b"m") && x.any(k + 1, FRICATIVE);
                emit(&[if nn || mm { nasal } else { plain }]);
            };
            // í i: after c s n z k g before a vowel, silent
            let ii = |plain: u32, emit: &mut dyn FnMut(&[u32])| {
                if i != 0 && b"csnzkg".contains(&prev(i)) && vowel(x.c(k)) {
                    return;
                }
                let p = prev(i);
                if i == 0 || (cons(p) && x.c(k) == b'i') || !cons(p) || !vowel(x.c(k)) {
                    emit(&[plain]);
                } else {
                    emit(&[0x10c2]);
                }
            };
            match c {
                b'b' | b'p' => {
                    if c == b'b' && i != n && !x.voiceless_my(k, i) {
                        emit(&[0x1fe2]);
                    } else {
                        emit(&[0x1ff2]);
                    }
                }
                b'f' => emit(&[0x1d52]),
                b'g' => {
                    if x.voiceless_my(k, i) || i == n {
                        emit(&[0xff2]);
                    } else if x.eq(k, b"i") || x.eq(k, b"\xed") {
                        emit(&[0x200fe2]);
                    } else {
                        emit(&[0xfe2]);
                    }
                }
                b'h' => {
                    if i == 0 || prev(i) != b'c' {
                        emit(&[0xf52]);
                    }
                }
                b'j' => {
                    if i != 0 && x.any(i - 1, J_VOWEL) && (i == n || x.any(k, J_CONSONANT)) {
                        emit(&[0x1007c1]);
                    } else if !x.eq(k, b"i") {
                        emit(&[0x10c2]);
                    }
                }
                b'k' => {
                    if x.eq(k, b"w") || x.eq(k, b"rz") {
                        emit(&[0xff2]);
                    } else if x.any(k, VOICED) {
                        emit(&[0xfe2]);
                    } else if x.eq(k, b"i") || x.eq(k, b"\xed") {
                        emit(&[0x200ff2]);
                    } else {
                        emit(&[0xff2]);
                    }
                }
                b'm' => emit(&[if x.eq(k, b"w") || x.eq(k, b"f") { 0x1dc2 } else { 0x1fc2 }]),
                b'q' => emit(&[0xff2]),
                b'v' => emit(&[if i == n { 0x1d52 } else { 0x1d42 }]),
                b'w' => {
                    let devoiced = if i != n && !x.voiceless_my(k, i) {
                        let p = |s: &[u8]| i >= 2 && x.eq(i - 2, s);
                        if i != 0 && b"ptkc\xe6".contains(&prev(i)) {
                            true
                        } else if p(b"cz") || (i != 0 && b"fs".contains(&prev(i))) {
                            true
                        } else if p(b"sz") || (i != 0 && prev(i) == 0x9c) || p(b"ch") {
                            true
                        } else {
                            (i != 0 && b"hx".contains(&prev(i))) || p(b"kr") || p(b"tr")
                        }
                    } else {
                        true
                    };
                    emit(&[if devoiced { 0x1d52 } else { 0x1d42 }]);
                }
                b'x' => emit(&[0xff2, 0x1952]),
                b'y' => emit(&[0x5c1]),
                b's' | 0x9a | 0xba => {
                    if x.eq(k, b"z") && x.voicing(k + 1) {
                        emit(&[0x2001742]);
                    } else if x.eq(k, b"z") {
                        emit(&[0x2001752]);
                    } else if x.voicing(k) {
                        emit(&[0x1942]);
                    } else if (x.eq(k, b"i") || x.eq(k, b"\xed")) && !(i == 0 && self.starts_with_any(word, S_SOFT)) {
                        emit(&[0x1001752]);
                    } else {
                        emit(&[0x1952]);
                    }
                }
                0x9c => emit(&[if x.voicing(k) { 0x1001742 } else { 0x1001752 }]),
                b'z' | 0x9e => {
                    if i != 0 && b"drsc".contains(&prev(i)) {
                        // part of dz rz sz cz
                    } else if i == 0 && (x.eq(1, b"irz") || x.eq(1, b"\xedrz")) {
                        emit(&[0x1001742]);
                    } else {
                        let zi = if i == 0 {
                            true
                        } else {
                            (i >= 3 && (x.eq(i - 3, b"nie") || x.eq(i - 3, b"bez")))
                                || (i >= 2 && (x.eq(i - 2, b"de") || x.eq(i - 2, b"ro")))
                        };
                        if zi && (x.any(k, ZI) || x.eq(k, b"\xed\x9c")) {
                            emit(&[0x1942]);
                        } else if x.eq(k, b"i") || x.eq(k, b"\xed") {
                            emit(&[0x1001742]);
                        } else if (x.voiceless_my(k, i)
                            && !x.eq(k, b"izmy")
                            && !x.eq(k, b"yzmy")
                            && !x.eq(k, b"azmy"))
                            || i == n
                        {
                            emit(&[0x1952]);
                        } else {
                            emit(&[0x1942]);
                        }
                    }
                }
                0x9f => {
                    if i == 0 || prev(i) != b'd' {
                        if i == n || x.voiceless_my(k, i) {
                            emit(&[0x1001752]);
                        } else {
                            emit(&[0x1001742]);
                        }
                    }
                }
                0xb3 => {
                    if i != 0 && x.any(i - 1, L_VOWEL) && (i == n || cons(x.c(k))) {
                        emit(&[0x1003e1]);
                    } else {
                        emit(&[0x401ec2]);
                    }
                }
                0xb9 => {
                    if x.eq(k, b"\xb3") {
                        emit(&[0x2e1]);
                    } else {
                        nasal(0x100002e1, &mut emit);
                    }
                }
                0xe3 => {
                    if x.eq(k, b"\xb3") {
                        emit(&[0x102e1]);
                    } else {
                        nasal(0x100102e1, &mut emit);
                    }
                }
                0xbf => {
                    if i == 0 || prev(i) != b'd' {
                        if i == n {
                            emit(&[0x2001752]);
                        } else if x.eq(k, b"d\xbf") && x.any(i + 3, VOICELESS) {
                            emit(&[0x2001752]);
                        } else if !x.voiceless_my(k, i) {
                            emit(&[0x2001742]);
                        } else {
                            emit(&[0x2001752]);
                        }
                    }
                }
                0xdf => emit(&[0x1952]),
                b'r' | 0xe0 | 0xf8 => {
                    let p1 = |set: &[u8]| i != 0 && set.contains(&prev(i));
                    let p2 = |s: &[u8]| i >= 2 && x.eq(i - 2, s);
                    let after_voiceless = p1(b"ptkc\xe6")
                        || p2(b"cz")
                        || p1(b"fs")
                        || p2(b"sz")
                        || p1(b"\x9c")
                        || p2(b"ch")
                        || p1(b"hx");
                    if after_voiceless && x.eq(k, b"z") {
                        emit(&[0x2001752]);
                    } else if x.eq(k, b"zmy") && i + 3 == n {
                        emit(&[0x2001752]);
                    } else if x.eq(k, b"z") && i + 1 == n {
                        emit(&[0x2001752]);
                    } else if x.eq(k, b"z") && x.any(k + 1, VOICELESS) {
                        emit(&[0x2001752]);
                    } else if x.eq(k, b"z") && self.starts_with_any(word, RZ_SPLIT) {
                        emit(&[0x19a2, 0x1942]);
                    } else if x.eq(k, b"z") {
                        emit(&[0x2001742]);
                    } else {
                        emit(&[0x19a2]);
                    }
                }
                0xe1 => emit(&[0x10441]),
                b'a' | 0xe2 | 0xe4 => emit(&[0x441]),
                b'l' | 0xe5 => {
                    if ((x.eq(k, b"i") || x.eq(k, b"\xed")) && !vowel(x.c(i + 2))) || x.eq(k, b"ii") {
                        emit(&[0x2018a2]);
                    } else {
                        emit(&[0x18a2]);
                    }
                }
                0xe6 => {
                    if x.eq(k, b"w") {
                        emit(&[0x1001712]);
                    } else if (sub == b'y' || sub == b'z') && (x.eq(k, b"dzi") || x.eq(k, b"set")) {
                    } else if !x.any(k, VOICED) {
                        emit(&[0x1001712]);
                    } else {
                        emit(&[0x1001702]);
                    }
                }
                b'c' | 0xe7 | 0xe8 => {
                    if x.eq(k, b"z") && x.voicing(k + 1) {
                        emit(&[0x2001702]);
                    } else if x.eq(k, b"z") {
                        emit(&[0x2001712]);
                    } else if x.voicing(k) {
                        emit(&[0x1902]);
                    } else if x.eq(k, b"i") || x.eq(k, b"\xed") {
                        emit(&[0x1001712]);
                    } else if x.eq(k, b"h") {
                        emit(&[0xf52]);
                    } else {
                        emit(&[0x1912]);
                    }
                }
                0xe9 => vn(0x106c1, 0x100106c1, &mut emit),
                b'e' | 0xeb => vn(0x6c1, 0x100006c1, &mut emit),
                0xf4 => vn(0x102e1, 0x100102e1, &mut emit),
                b'o' | 0xf5 | 0xf6 => vn(0x2e1, 0x100002e1, &mut emit),
                0xea | 0xec => {
                    let (plain, v, me) = if c == 0xea { (0x6c1, 0x100006c1, b"\xea") } else { (0x106c1, 0x100106c1, b"\xec") };
                    if (x.eq(k, b"\xb3") || x.eq(k, b"l") || i == n) && word != me {
                        emit(&[plain]);
                    } else if x.any(k, VELAR) {
                        emit(&[v, 0xfc2]);
                    } else if x.any(k, PALATAL_I) {
                        emit(&[v, 0x2019c2]);
                    } else if (sub == b'y' || sub == b'z') && x.eq(k, b"t") {
                        emit(&[v]);
                    } else {
                        nasal(v, &mut emit);
                    }
                }
                0xed => ii(0x107c1, &mut emit),
                b'i' | 0xee => ii(0x7c1, &mut emit),
                b'd' | 0xef | 0xf0 => {
                    if i != n && !x.voiceless_my(k, i) {
                        let vl2 = x.any(k + 1, VOICELESS);
                        let r = if x.eq(k, b"\x9f") && vl2 {
                            0x1001712
                        } else if x.eq(k, b"\x9fmy") && i + 3 == n {
                            0x1001712
                        } else if x.eq(k, b"\x9f") && i + 1 == n {
                            0x1001712
                        } else if x.eq(k, b"\x9f") || x.eq(k, b"zi") || x.eq(k, b"z\xed") {
                            0x1001702
                        } else if x.eq(k, b"z") && vl2 {
                            0x1912
                        } else if x.eq(k, b"zmy") && i + 3 == n {
                            0x1912
                        } else if x.eq(k, b"z") && i + 1 == n {
                            0x1912
                        } else if x.eq(k, b"z") {
                            0x1902
                        } else if x.eq(k, b"\xbf") && vl2 {
                            0x2001712
                        } else if x.eq(k, b"\xbfm") && i + 3 == n {
                            // strncmp(…, "żmy", 2)
                            0x2001712
                        } else if x.eq(k, b"\xbf") {
                            0x2001702
                        } else {
                            0x1be2
                        };
                        emit(&[r]);
                    } else {
                        emit(&[0x1bf2]);
                    }
                }
                b't' | 0xfe => emit(&[0x1bf2]),
                0xf1 => emit(&[if x.any(k, FRICATIVE) { 0x101007c1 } else { 0x11c2 }]),
                b'n' | 0xf2 => {
                    let pv = i != 0 && vowel(prev(i));
                    if x.eq(k, b"k") || x.eq(k, b"g") {
                        emit(&[0xfc2]);
                    } else if pv && x.any(k, N_SIBILANT) {
                        emit(&[0x101003e1]);
                    } else if pv && (x.eq(k, b"v") || x.eq(k, b"w") || x.eq(k, b"f")) {
                        emit(&[0x1dc2]);
                    } else if x.eq(k, b"i") || x.eq(k, b"\xed") {
                        emit(&[0x11c2]);
                    } else if x.eq(k, b"\xe6") || x.eq(k, b"d\x9f") {
                        emit(&[0x2019c2]);
                    } else if (x.eq(k, b"ci") && x.any(i + 3, NCI_VOWEL))
                        || (x.eq(k, b"dzi") && x.any(i + 4, NCI_VOWEL))
                        || (x.eq(k, b"c\xed") && x.any(i + 3, NCI_VOWEL))
                        || (x.eq(k, b"dz\xed") && x.any(i + 4, NCI_VOWEL))
                    {
                        emit(&[0x2019c2]);
                    } else {
                        emit(&[0x19c2]);
                    }
                }
                0xf3 => emit(&[0x3e1]),
                b'u' | 0xf9 | 0xfb | 0xfc => {
                    if self.starts_with_any(word, U_GLIDE) && (i == 1 || i == 2) {
                        emit(&[0x1003e1]);
                    } else {
                        emit(&[0x3e1]);
                    }
                }
                0xfa => emit(&[0x103e1]),
                0xfd => emit(&[0x105c1]),
                _ => {}
            }
            i += 1;
        }
        out
    }
}

/// The codes up to a 0 (the caller reads to the terminator).
fn until_zero(mut v: Vec<u32>) -> Vec<u32> {
    if let Some(p) = v.iter().position(|&c| c == 0) {
        v.truncate(p);
    }
    v
}

/// FonTrasParola replayed against a `LOQ_TRACE_LES` trace of
/// LTTS7Polish.dll (`LOQ_LES_TABLE=10022d78`): each call's record taken
/// from the preceding `< LesGradiAccentuali` block.
///
///   QQ_PL_TRACE=<a.les>[:<b.les>…] [QQ_PL_LTTS7=<LTTS7 dir>] [QQ_PL_MAX=<examples>]
///   cargo test -p quoquendo --release polish_fontras_trace -- --ignored --nocapture
#[cfg(test)]
mod tests {
    use super::super::Polish;
    use crate::lang::common::Word;

    fn unescape(s: &str) -> Vec<u8> {
        let b = s.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < b.len() && b[i] != b'"' {
            if b[i] == b'\\' && i + 1 < b.len() {
                if b[i + 1] == b'x' && i + 3 < b.len() {
                    out.push(u8::from_str_radix(&s[i + 2..i + 4], 16).unwrap_or(0));
                    i += 4;
                } else {
                    out.push(b[i + 1]);
                    i += 2;
                }
            } else {
                out.push(b[i]);
                i += 1;
            }
        }
        out
    }

    fn record(line: &str) -> Option<Word> {
        let mut it = line.split_whitespace();
        if it.next()? != "W" {
            return None;
        }
        it.next()?;
        let hex: String = (0..6).filter_map(|_| it.next()).collect();
        if hex.len() != 48 {
            return None;
        }
        let bytes: Vec<u8> = (0..24).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap_or(0)).collect();
        let text = line.find(" s4=\"").map(|p| unescape(&line[p + 5..])).unwrap_or_default();
        let mut w = crate::lang::common::record(text, 0, 0, 0);
        w.f[0xc..0x18].copy_from_slice(&bytes[0xc..0x18]);
        Some(w)
    }

    #[test]
    #[ignore]
    fn polish_fontras_trace() {
        let root = std::env::var("QQ_PL_LTTS7").unwrap_or_else(|_| "/tmp/qqv/LTTS7".into());
        let read = |p: String| std::fs::read(&p).unwrap_or_else(|e| panic!("{p}: {e}"));
        let files = crate::lang::ModuleFiles {
            dll: read(format!("{root}/bin/LTTS7Polish.dll")),
            mother_tongue: "Polish".into(),
            dbl: Some(read(format!("{root}/data/languages/Polish/Polish.dbl"))),
            ..Default::default()
        };
        let p = Polish::from_files(&files).expect("LTTS7Polish.dll");
        let max: usize = std::env::var("QQ_PL_MAX").ok().and_then(|s| s.parse().ok()).unwrap_or(40);
        let traces = std::env::var("QQ_PL_TRACE").expect("QQ_PL_TRACE");
        let (mut calls, mut bad, mut missing) = (0usize, 0usize, 0usize);
        for path in traces.split(':') {
            let text = String::from_utf8_lossy(&read(path.to_string())).into_owned();
            let lines: Vec<&str> = text.lines().collect();
            let mut sentence: Vec<Word> = Vec::new();
            let mut pos = 0;
            let mut k = 0;
            while k < lines.len() {
                let l = lines[k];
                if l.starts_with("< LesGradiAccentuali") {
                    sentence = lines[k + 1..].iter().take_while(|l| l.starts_with("  W ")).filter_map(|l| record(l.trim_start())).collect();
                    pos = 0;
                } else if l.starts_with("> FonTrasParola") {
                    let word = l.find(" word=\"").map(|q| unescape(&l[q + 7..])).unwrap_or_default();
                    let Some(end) = (k + 1..lines.len()).find(|&j| lines[j].starts_with("< FonTrasParola")) else { break };
                    let codes: Vec<u32> = lines[end]
                        .split(" out=")
                        .nth(1)
                        .map(|s| s.split_whitespace().next().unwrap_or("").split(',').filter_map(|c| u32::from_str_radix(c, 16).ok()).collect())
                        .unwrap_or_default();
                    match (pos..sentence.len()).find(|&j| sentence[j].text == word) {
                        Some(at) => {
                            pos = at + 1;
                            calls += 1;
                            let ours = p.tras_parola(&sentence[at..=at], 0);
                            if ours != codes {
                                bad += 1;
                                if bad <= max {
                                    let sym = |v: &[u32]| v.iter().map(|&c| p.symbol(c).unwrap_or("?").to_string()).collect::<Vec<_>>().join(" ");
                                    println!(
                                        "{:?} grade {} sub {:?}\n  orig {codes:x?} {}\n  ours {ours:x?} {}",
                                        String::from_utf8_lossy(&word),
                                        sentence[at].f[0x12],
                                        sentence[at].f[0xf] as char,
                                        sym(&codes),
                                        sym(&ours)
                                    );
                                }
                            }
                        }
                        None => missing += 1,
                    }
                    k = end;
                }
                k += 1;
            }
        }
        println!("FonTrasParola: {calls} calls, {bad} differ, {missing} without a record");
        assert_eq!(bad, 0);
    }
}
