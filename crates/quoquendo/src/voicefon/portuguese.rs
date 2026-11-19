//! The Portuguese voices' libraries. `FonVidAllofonoVoce` does nothing in
//! Amalia, Eusebio and Gabriela (0x10001210 `return 0`); Fernanda and
//! Felipe (0x100014a0) replace the words of their word table and adjust a
//! few phones. `FonVidMetaFonema`: Amalia the default (0x10001400 →
//! 0x10001220), Eusebio the default then its Analisi2Sintesi (0x100013d0),
//! Gabriela and Felipe a voiced `z` id for s before a voiced consonant,
//! Felipe and Fernanda an id for their tapped r (0x200018d2).

use super::grace::{lookup, Lib};
use super::{meta_default, Port, Rules, Stage, KIND_PHONE, KIND_WORD_END};

const fn port(voice: &'static str, table: u32, functions: &'static [(&'static str, u32)], allofono: Option<fn(&mut Stage, &mut usize)>, meta: fn(&Stage, &mut usize, &mut Vec<u8>)) -> Port {
    Port {
        voice,
        table,
        functions,
        rules: Rules { allofono, meta },
    }
}

pub(super) const AMALIA: Port = port(
    "Amalia",
    0x10008720,
    &[
        ("GetVersion", 0x100010f0),
        ("ELQGetPhonemes", 0x10001000),
        ("FonVidAllofonoVoce", 0x10001210),
        ("FonVidAllofonoAnalisi", 0x10001290),
        ("FonVidMetaFonema", 0x10001400),
        ("FonVidAnalisi2Sintesi", 0x10001420),
    ],
    None,
    meta_default,
);

pub(super) const EUSEBIO: Port = port(
    "Eusebio",
    0x10008718,
    &[
        ("GetVersion", 0x100010f0),
        ("ELQGetPhonemes", 0x10001000),
        ("FonVidAllofonoVoce", 0x10001210),
        ("FonVidAllofonoAnalisi", 0x10001290),
        ("FonVidMetaFonema", 0x10001450),
        ("FonVidAnalisi2Sintesi", 0x100013d0),
    ],
    None,
    eusebio_meta,
);

pub(super) const GABRIELA: Port = port(
    "Gabriela",
    0x10008620,
    &[
        ("GetVersion", 0x10001150),
        ("ELQGetPhonemes", 0x10001060),
        ("FonVidAllofonoVoce", 0x10001210),
        ("FonVidAllofonoAnalisi", 0x10001290),
        ("FonVidMetaFonema", 0x10001340),
        ("FonVidAnalisi2Sintesi", 0x100013f0),
    ],
    None,
    gabriela_meta,
);

pub(super) const FERNANDA: Port = port(
    "Fernanda",
    0x10008610,
    &[
        ("GetVersion", 0x100010f0),
        ("ELQGetPhonemes", 0x10001000),
        ("FonVidAllofonoVoce", 0x100014a0),
        ("FonVidAllofonoAnalisi", 0x10001670),
        ("FonVidMetaFonema", 0x10001720),
        ("FonVidAnalisi2Sintesi", 0x10001770),
    ],
    Some(fernanda),
    fernanda_meta,
);

pub(super) const FELIPE: Port = port(
    "Felipe",
    0x10008610,
    &[
        ("GetVersion", 0x100010f0),
        ("ELQGetPhonemes", 0x10001000),
        ("FonVidAllofonoVoce", 0x100014a0),
        ("FonVidAllofonoAnalisi", 0x10001760),
        ("FonVidMetaFonema", 0x100018c0),
        ("FonVidAnalisi2Sintesi", 0x100018a0),
    ],
    Some(felipe),
    felipe_meta,
);

/// The word tables 0x10001210 searches (0x54-byte entries from 0x1000b000,
/// ended by the name at `words_end`); the Analisi2Sintesi fields are unused.
const fn words(words_end: u32) -> Lib {
    Lib {
        words: 0x1000b000,
        words_end,
        a2s_index: 0,
        a2s_jump: 0,
        a2s_max: 0,
    }
}
const FERNANDA_WORDS: Lib = words(0x100086c4);
const FELIPE_WORDS: Lib = words(0x100086bc);

/// Engine codes the rules test.
const S: u32 = 0x1952;
const Z: u32 = 0x1942;
const B: u32 = 0x1fe2;
const D: u32 = 0x1be2;
const G: u32 = 0xfe2;
const T: u32 = 0x1bf2;
const TSH: u32 = 0x1712;
const I: u32 = 0x7c1;
const U: u32 = 0x3e1;
const W: u32 = 0x401ec2;
const J: u32 = 0x10c2;
const M: u32 = 0x1fc2;
const N: u32 = 0x19c2;
const ZG: u32 = 0x1742;
const R: u32 = 0x1b82;
/// Felipe's and Fernanda's tapped r before a voiceless consonant.
const R_TAP: u32 = 0x200018d2;

/// The voiced consonants an s takes voice before (b d dʒ g v ʒ m n l z).
fn voiced(c: u32) -> bool {
    matches!(c, D | 0x1702 | B | G | 0x1d42 | ZG | M | N | 0x18a2 | Z)
}

/// 0x10001350 (Felipe and Fernanda): at a word's first phone, a word of
/// the table (by its text and stress grade, 0x10001210) is replaced: all
/// its phones deleted, the entry's codes inserted (0x48 the deleted last
/// one's code), the last one ending the word; `*i` back on its first.
fn replace(s: &mut Stage, lib: &Lib, i: &mut usize) {
    let start = *i;
    let w = s.word(*i);
    let grade = s.rec(w as i64).map_or(0, |r| r.f[0x12]);
    let Some(e) = lookup(s, lib, s.text(w as i64), grade) else { return };
    let mut last = s.code(*i);
    while *i < s.list.len() && s.word(*i) == w && s.code(*i) != 0 {
        last = s.code(*i);
        s.delete(*i);
    }
    let entry = lib.words + e * 0x54 + 0x10;
    for j in 0..16 {
        let c = s.pe.u32(entry + j * 4);
        if c == 0 {
            break;
        }
        if *i < s.list.len() {
            s.insert(*i, w, if c == 0x48 { last } else { c }, KIND_PHONE);
            *i += 1;
        }
    }
    if let Some(p) = i.checked_sub(1).and_then(|k| s.list.get_mut(k)) {
        p.kind = KIND_WORD_END;
    }
    *i = start;
}

/// A word's first phone: the replacement first.
fn word_start(s: &mut Stage, lib: &Lib, i: &mut usize) {
    if *i == 0 || s.word(*i) != s.word(*i - 1) {
        replace(s, lib, i);
    }
}

/// `ELQListAddElementAfter` the current element: an `i` of its word.
fn add_i(s: &mut Stage, i: usize) {
    let w = s.word(i);
    s.insert(i + 1, w, I, KIND_PHONE);
}

/// Fernanda 0x100014a0: the word table; s voiced before a voiced
/// consonant; an `i` after b before `tʃ`, or in `s u b s tʃ`.
fn fernanda(s: &mut Stage, i: &mut usize) {
    word_start(s, &FERNANDA_WORDS, i);
    let k = *i;
    let w = s.word(k);
    match s.code(k) {
        S => {
            if voiced(s.code(k + 1)) {
                s.set(k, Z);
            }
        }
        B => {
            let sub = k >= 2 && s.code(k - 2) == S && s.code(k - 1) == U && s.word(k + 1) == w && s.code(k + 1) == S && s.code(k + 2) == TSH;
            if sub || (k + 1 < s.list.len() && s.word(k + 1) == w && s.code(k + 1) == TSH) {
                add_i(s, k);
            }
        }
        _ => {}
    }
}

/// Felipe 0x100014a0: the word table; r tapped before t tʃ f s; a word's
/// first `i` before s + a voiced consonant (or after a nasal u + w) said
/// e; an `i` after g before m/n, d before ʒ, b before t tʃ s.
fn felipe(s: &mut Stage, i: &mut usize) {
    word_start(s, &FELIPE_WORDS, i);
    let k = *i;
    let w = s.word(k);
    let next = s.code(k + 1);
    let consonant = |c: u32| c & 3 == 2 && c != W && c != J;
    match s.code(k) {
        R => {
            if matches!(next, T | TSH | 0x1d52 | S) {
                s.set(k, R_TAP);
            }
        }
        I => {
            let initial = k == 0 || w != s.word(k - 1);
            let plain = initial && next == S && consonant(s.code(k + 2));
            let after_nasal_u = k >= 2
                && matches!(s.code(k - 2), 0x100003e1 | 0x100103e1)
                && s.word(k - 1) != s.word(k - 2)
                && s.code(k - 1) == W
                && next == S
                && consonant(s.code(k + 2));
            if plain || after_nasal_u {
                s.set(k, 0x741);
            }
        }
        G => {
            if matches!(next, M | N) {
                add_i(s, k);
            }
        }
        D => {
            if next == ZG {
                add_i(s, k);
            }
        }
        B => {
            if matches!(next, T | TSH | S) {
                add_i(s, k);
            }
        }
        _ => {}
    }
}

/// Eusebio 0x10001450: the default id through Analisi2Sintesi 0x100013d0.
fn eusebio_meta(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    let at = out.len();
    meta_default(s, i, out);
    if let Some(id) = out.get_mut(at) {
        *id = match *id {
            0x21 | 0x22 => 0x2b,
            0x23 | 0x24 => 0x2d,
            0x25 | 0x26 => 0x2c,
            0x27 => 0x41,
            0x31 => 0x30,
            0x34 => 0x33,
            0x37 => 0x36,
            0x39 => 0x38,
            x => x,
        };
    }
}

/// Gabriela 0x10001340: s before a voiced consonant said as id 0x28.
fn gabriela_meta(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    if s.code(*i) == S && voiced(s.code(*i + 1)) {
        out.push(0x28);
        *i += 1;
        return;
    }
    meta_default(s, i, out);
}

/// Fernanda 0x10001720: the tapped r as id 0x33.
fn fernanda_meta(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    if s.code(*i) == R_TAP {
        out.push(0x33);
        *i += 1;
        return;
    }
    meta_default(s, i, out);
}

/// Felipe 0x100018c0: s before a voiced consonant id 0x28, the tapped r
/// 0x33, else the default; then Analisi2Sintesi 0x100018a0 (0x1c → 0x33,
/// 0x29 → 0x28, 0x2d → 0x2c).
fn felipe_meta(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    let at = out.len();
    if s.code(*i) == S && voiced(s.code(*i + 1)) {
        out.push(0x28);
        *i += 1;
    } else if s.code(*i) == R_TAP {
        out.push(0x33);
        *i += 1;
    } else {
        meta_default(s, i, out);
    }
    if let Some(id) = out.get_mut(at) {
        *id = match *id {
            0x1c => 0x33,
            0x29 => 0x28,
            0x2d => 0x2c,
            x => x,
        };
    }
}
