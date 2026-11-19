//! LTTS7Sven.dll (Swedish): `FonVidAllofonoVoce` 0x10001440 (a word
//! dictionary 0x10001210 at each word's start, then Annika's stop rules),
//! `FonVidMetaFonema` 0x10002630 (the default 0x10001fc0, then the
//! analysis ids mapped to synthesis ones, 0x100025a0).

use super::annika::{aspirate, d_rule, release, DENTAL, LABIAL, VELAR};
use super::{meta_default, Port, Rules, Stage, KIND_PHONE, KIND_WORD_END};

pub(super) const SVEN: Port = Port {
    voice: "Sven",
    table: 0x10009a70,
    functions: &[
        ("GetVersion", 0x10001150),
        ("ELQGetPhonemes", 0x10001060),
        ("FonVidAllofonoVoce", 0x10001440),
        ("FonVidAllofonoAnalisi", 0x10002030),
        ("FonVidMetaFonema", 0x10002630),
        ("FonVidAnalisi2Sintesi", 0x100025a0),
        ("FonVidIngloba", 0x10002600),
    ],
    rules: Rules {
        allofono: Some(sven),
        meta: sven_meta,
    },
};

/// The word dictionary: 0x54-byte entries from 0x1000c000, the word at
/// +0, its codes from +0x10 up to a 0 (0x48: the replaced word's last
/// code), the stress grade it applies to at +0x50 (-1: any); closed by the
/// entry whose first 3 bytes are those at 0x10009afc (`-1`).
const DICT: u32 = 0x1000c000;
const DICT_END: u32 = 0x10009afc;
/// "bort" (0x10001440 compares 5 bytes) and "kanske" (7 bytes).
const BORT: u32 = 0x10009b00;
const KANSKE: u32 = 0x10009b08;

/// 0x10001210: a dictionary word's phones (every element of its record)
/// replaced by the entry's codes, the last one a word end.
fn replace(s: &mut Stage, i: usize) {
    let w = s.word(i);
    let Some(r) = s.rec(w as i64) else { return };
    let grade = r.f[0x12] as u32;
    let text = r.text.clone();
    let end = &s.pe.at(DICT_END)[..3];
    let mut k = 0u32;
    let found = loop {
        let e = DICT + k * 0x54;
        let head = s.pe.at(e);
        if head.len() >= 3 && &head[..3] == end {
            break None;
        }
        let g = s.pe.u32(e + 0x50);
        if s.pe.cstr(e) == text.as_slice() && (g == u32::MAX || g == grade) {
            break Some(e);
        }
        k += 1;
    };
    let Some(e) = found else { return };
    let mut last = s.code(i);
    while i < s.list.len() && s.word(i) == w {
        last = s.code(i);
        s.delete(i);
    }
    let mut at = e + 0x10;
    let mut j = i;
    loop {
        let c = s.pe.u32(at);
        if c == 0 {
            break;
        }
        s.insert(j, w, if c == 0x48 { last } else { c }, KIND_PHONE);
        j += 1;
        at += 4;
    }
    if let Some(p) = j.checked_sub(1).and_then(|j| s.list.get_mut(j)) {
        p.kind = KIND_WORD_END;
    }
}

/// 0x10001440.
fn sven(s: &mut Stage, pi: &mut usize) {
    let i = *pi;
    if i == 0 || s.word(i) != s.word(i - 1) {
        replace(s, i);
    }
    let text = s.text(s.word(i) as i64).map(<[u8]>::to_vec);
    match s.code(i) {
        0x10365 => {
            if text.as_deref() == Some(s.pe.cstr(BORT)) {
                s.set(i, 0x102e1);
            }
        }
        0x501752 => {
            if text.as_deref() == Some(s.pe.cstr(KANSKE)) {
                s.set(i, 0x2001752);
            }
        }
        0x1ff2 => {
            if !aspirate(s, i, 0x3ff2) {
                release(s, i, &LABIAL, 0x5ff2);
            }
        }
        0x1fe2 => release(s, i, &LABIAL, 0x5fe2),
        0x19f2 => {
            if !aspirate(s, i, 0x39f2) {
                release(s, i, &DENTAL, 0x59f2);
            }
        }
        0xfe2 => release(s, i, &VELAR, 0x4fe2),
        0xff2 => {
            aspirate(s, i, 0x2ff2);
            release(s, i, &VELAR, 0x4ff2);
        }
        0x19e2 => d_rule(s, i),
        _ => {}
    }
}

/// 0x10002630: the default id (0x10001fc0), then 0x100025a0's switch:
/// 0x50 → 0x45, 0x51 → 0x47, 0x5b → 0x57, 0x63 and 0x64 → 0x62.
fn sven_meta(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    let start = out.len();
    meta_default(s, i, out);
    for id in &mut out[start..] {
        *id = match *id {
            0x50 => 0x45,
            0x51 => 0x47,
            0x5b => 0x57,
            0x63 | 0x64 => 0x62,
            x => x,
        };
    }
}
