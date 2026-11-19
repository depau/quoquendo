//! FonTrasParola, one per variant (EnglishUs 0x10018140, EnglishGb
//! 0x100178f0, EnglishAu 0x10012170): a word's phones from the first of
//! its database rows with a pronunciation, else from the letter-to-sound
//! model (LTTS7GPR.dll `ELQGPRRun`), a possessive `'s` added, and a stress
//! placed when the model gave none.

use super::{English, Rows, Variant, FONETICA, GRAFEMICA};
use crate::lang::common::{field::*, Word};

/// 0x1000bf90: a pronunciation (`fonetica`, symbols separated by spaces or
/// `-`, at most 5 characters each) as codes; 0x10 for an unknown symbol.
fn parse(en: &English, s: &[u8]) -> Vec<u32> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < s.len() {
        let start = i;
        while i < s.len() && s[i] != b' ' && s[i] != b'-' && i - start < 5 {
            i += 1;
        }
        let sym = &s[start..i];
        if i < s.len() {
            i += 1;
        }
        out.push(en.code(sym));
    }
    out
}

/// The `'s` ending after the word's last phone (US 0x10017a90, GB
/// 0x100174e0, AU 0x10012000): `I z` after a sibilant, `s` after a
/// voiceless consonant, `z` otherwise (GB/AU: a final `Hr` becomes `z`).
fn possessive(v: Variant, out: &mut Vec<u32>) {
    let Some(&last) = out.last() else { return };
    if matches!(last, 0x1952 | 0x1942 | 0x1752 | 0x1712 | 0x1702 | 0x1742) {
        out.extend([0x781, 0x1942]);
        return;
    }
    let voiceless: &[u32] = match v {
        Variant::Us => &[0x3ff2, 0x5ff2, 0x39f2, 0x59f2, 0x9e2, 0x2ff2, 0x4ff2, 0x1d52, 0x1b52, 0x1982],
        Variant::Gb => &[0x3ff2, 0x1ff2, 0x5ff2, 0x39f2, 0x19f2, 0x59f2, 0x9e2, 0x2ff2, 0xff2, 0x4ff2, 0x1d52, 0x1b52],
        Variant::Au => &[0x3ff2, 0x1ff2, 0x5ff2, 0x39f2, 0x19f2, 0x59f2, 0x9e2, 0x2ff2, 0xff2, 0x4ff2, 0x1d52, 0x1b52, 0x1982],
    };
    if voiceless.contains(&last) {
        out.push(0x1952);
    } else if v != Variant::Us && last == 0x18c2 {
        *out.last_mut().unwrap() = 0x1942;
    } else {
        out.push(0x1942);
    }
}

/// A stressed vowel unstressed (US 0x10017b70, GB/AU 0x100172a0).
fn unstress(v: Variant, c: u32) -> u32 {
    match (v, c) {
        (_, 0x10781) => 0x781,
        (_, 0x103a1) => 0x3a1,
        (_, 0x10245) => 0x245,
        (_, 0x102c1) => 0x2c1,
        (_, 0x102e5) => 0x2e5,
        (_, 0x103e5) => 0x3e1,
        (_, 0x10681) => 0x681,
        (_, 0x106c1) => 0x6c1,
        (_, 0x1e2e5) => 0xe2e5,
        (_, 0x107c5) => 0x7c1,
        (_, 0x1ac45) => 0xac45,
        (_, 0x1e445) => 0xe445,
        (_, 0x1e6c5) => 0xe6c5,
        (Variant::Us, 0x1ab65) => 0xab65,
        (Variant::Us, 0x2104c5) => 0x2004c5,
        (Variant::Gb | Variant::Au, 0x10261) => 0x261,
        (Variant::Gb | Variant::Au, 0x104c5) => 0x4c5,
        (Variant::Gb | Variant::Au, 0x146c5) => 0x46c5,
        (Variant::Gb | Variant::Au, 0x143a5) => 0x43a5,
        (Variant::Gb | Variant::Au, 0x14785) => 0x4785,
        (Variant::Gb | Variant::Au, 0x1ad05) => 0xad05,
        _ => c,
    }
}

/// The stressed vowel a letter is read as when the word's phones begin
/// with consonants and none is stressed (US 0x10017d00, GB/AU 0x10017640);
/// 1 for a letter that is not a vowel.
fn letter_vowel(v: Variant, c: u8) -> u32 {
    match c {
        0x79 => 0x107c5,
        0x9c | 0xe8..=0xeb | 0x65 => 0x106c1,
        0xe0 | 0xe1 | 0xe3..=0xe6 | 0x61 => 0x10681,
        0xec..=0xef | 0x69 => 0x10781,
        0xf2..=0xf6 | 0xf8 | 0x6f => {
            if v == Variant::Us {
                0x10245
            } else {
                0x10261
            }
        }
        0xf9..=0xfc | 0x75 => 0x103a1,
        _ => 1,
    }
}

/// 0x10017e50: where a stress goes in phones that begin with a consonant:
/// after an initial `s` + stop (+ r/l) cluster, a stop + r/l, a fricative +
/// r/l…, else the first vowel; `n` past the end.
fn stress_at(p: &[u32], n: usize) -> usize {
    let at = |i: usize| p.get(i).copied().unwrap_or(0);
    let c0 = at(0);
    if c0 == 0x1952 {
        let c1 = at(1);
        let b = c1 as u8;
        let rl = |c: u32| c == 0x18a2 || c == 0x18c2;
        if (c1 & 0x1e3) == 0x1e2 && (c1 & 0x10) != 0 {
            if rl(at(2)) {
                if b & 3 == 2 {
                    return 3;
                }
            } else if b & 3 == 2 {
                return 2;
            }
        } else if c1 != 0x1d52 && c1 != 0x1fc2 && c1 != 0x19c2 {
            if b & 3 == 2 {
                return 1;
            }
        } else if b & 3 == 2 {
            return 2;
        }
    }
    if (c0 & 0x1e3) == 0x1e2 {
        let c1 = at(1);
        if c1 == 0x18a2 || c1 == 0x18c2 {
            if at(2) as u8 & 3 == 2 {
                return 2;
            }
        } else if c1 as u8 & 3 == 2 {
            return 1;
        }
    }
    if (c0 & 0x1e3) == 0x142 {
        let c1 = at(1);
        if c1 & 0x10 != 0 && c0 != 0x952 {
            if c1 == 0x18a2 || c1 == 0x18c2 {
                if at(2) as u8 & 3 == 2 {
                    return 2;
                }
            } else if c1 as u8 & 3 == 2 {
                return 1;
            }
        }
    }
    let mut i = 0usize;
    while at(i) as u8 & 3 != 1 {
        if n <= i {
            return 1;
        }
        i += 1;
    }
    if n <= i {
        return 1;
    }
    i
}

/// 0x10017ca0: stress the first vowel (`HEh` becomes `` `E ``, `i`/`u`
/// their long stressed forms).
fn stress_first(p: &mut [u32]) {
    for c in p.iter_mut() {
        if *c as u8 & 3 == 1 {
            *c = match *c {
                0x501 => 0x106c1,
                0x7c1 | 0x3e1 => *c | 0x10005,
                _ => *c | 0x10001,
            };
            return;
        }
    }
}

/// US 0x10017fe0, GB/AU 0x10017790: the word's stress, when the model gave
/// none or several.
fn stress(en: &English, v: Variant, p: &mut Vec<u32>, grade: u8, text: &[u8]) {
    let n = p.len();
    let len = text.len();
    let mut first = 0usize;
    let mut count = 0;
    for (i, &c) in p.iter().enumerate() {
        if c & 0x10003 == 0x10001 {
            if first == 0 {
                first = i;
            }
            count += 1;
        }
    }
    if count > 1 {
        for c in p.iter_mut().take(n).skip(first + 1) {
            *c = unstress(v, *c);
        }
        return;
    }
    if count == 1 || grade == 2 || grade == 3 {
        return;
    }
    if n == 0 || p[0] as u8 & 3 != 2 {
        stress_first(p);
        return;
    }
    let mut at = stress_at(p, n);
    let vowel_there = p.get(at).copied().unwrap_or(0) & 3 == 1;
    if vowel_there && at < n.wrapping_sub(1) {
        stress_first(p);
        return;
    }
    let mut code = p[0];
    let mut i = 0usize;
    while i <= len {
        let c = super::super::common::at(text, i);
        if en.flags(c) >> 4 & 1 != 0 {
            code = letter_vowel(v, c);
            break;
        }
        i += 1;
    }
    if vowel_there && i == len {
        stress_first(p);
        return;
    }
    if i > len {
        return;
    }
    if i == 0 {
        at = 0;
    }
    // inserted before the phone at `at` (the terminator counts as one)
    p.insert(at.min(n), code);
}

/// FonTrasParola: the word at `i` of the chunk.
pub(crate) fn tras_parola(en: &English, w: &[Word], i: usize) -> Vec<u32> {
    let v = en.variant;
    let mut r = w[i].clone();
    let mut out: Vec<u32> = Vec::new();
    let mut found = en.find_set(&mut r, FONETICA);
    let mut grafemica_equal = false;
    if found {
        match Rows::current(&r) {
            Some(cur) => {
                out = parse(en, en.cell(cur, FONETICA).unwrap_or(b""));
                if v == Variant::Gb {
                    grafemica_equal = super::stricmp(&r.text, en.cell(cur, GRAFEMICA).unwrap_or(b"")).is_eq();
                }
            }
            None => found = false,
        }
    }
    let mut word = r.text.clone();
    let mut flag = false;
    if !grafemica_equal {
        if word.last() == Some(&b'\'') {
            word.pop();
        } else if word.len() > 1 && word.ends_with(b"'s") {
            flag = true;
            word.truncate(word.len() - 2);
        }
    }
    if found {
        let class = r.f[CLASS];
        match v {
            Variant::Us if class == b'*' || class == b',' || class == b'+' => flag = false,
            Variant::Au if class != 0x15 => flag = false,
            _ => {}
        }
    }
    // US 0x10017f90, GB 0x100175f0, AU 0x10012120
    if let Some(g) = &en.gpr {
        if out.first().is_none_or(|&c| c == 0) {
            out = g.transcribe(&word);
        }
        if flag {
            possessive(v, &mut out);
        }
    }
    // a 0 ends the codes
    if let Some(z) = out.iter().position(|&c| c == 0) {
        out.truncate(z);
    }
    // (the record's text is the word as cut above: the engine cuts it in
    // place)
    if Rows::len(&r) == 0 || !found {
        stress(en, v, &mut out, r.f[GRADE], &word);
    }
    out
}
