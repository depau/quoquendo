//! LTTS7Bernard.dll (French): `FonVidAllofonoVoce` 0x10001230,
//! `FonVidMetaFonema` 0x100014c0 (the default).

use super::{meta_default, Port, Rules, Stage};

pub(super) const BERNARD: Port = Port {
    voice: "Bernard",
    table: 0x10008960,
    functions: &[
        ("GetVersion", 0x100010f0),
        ("ELQGetPhonemes", 0x10001000),
        ("FonVidAllofonoVoce", 0x10001230),
        ("FonVidAllofonoAnalisi", 0x100013f0),
        ("FonVidMetaFonema", 0x100014c0),
        ("FonVidAnalisi2Sintesi", 0x100014e0),
    ],
    rules: Rules {
        allofono: Some(allofono),
        meta: meta_default,
    },
};

/// 0x10001230: a nasal vowel (`Aa) E) o) Oe)`, stressed or not) of a word
/// (not a number's: subclass `y`/`z`) written with a final `-e`/`-es`,
/// followed by a consonant that ends its word and a consonant opening the
/// next: the final e is said (`HEh` after that consonant).
fn allofono(s: &mut Stage, i: &mut usize) {
    let i = *i;
    if !matches!(s.code(i), 0x109111 | 0x81111 | 0x20511 | 0x20521 | 0x81121 | 0x109121 | 0x120511 | 0x120521) {
        return;
    }
    let Some(r) = s.rec(s.word(i) as i64) else { return };
    if matches!(r.f[0xf], b'y' | b'z') {
        return;
    }
    let t = r.text.as_slice();
    let final_e = t.ends_with(b"e") || (t.len() > 1 && t.ends_with(b"es"));
    if s.code(i + 1) & 4 != 0 && s.code(i + 2) & 4 != 0 && s.word(i + 1).wrapping_add(1) == s.word(i + 2) && final_e {
        let w = s.word(i + 1);
        s.insert(i + 2, w, 0x10851, 0);
    }
}
