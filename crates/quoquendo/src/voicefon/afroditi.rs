//! LTTS7Afroditi.dll (Greek): `FonVidAllofonoVoce` 0x10001210,
//! `FonVidMetaFonema` 0x100012d0 (→ 0x10001260, the default: the id of the
//! first table entry with the phone's code, else 1).

use super::{meta_default, Port, Rules, Stage};

pub(super) const AFRODITI: Port = Port {
    voice: "Afroditi",
    table: 0x10008588,
    functions: &[
        ("GetVersion", 0x100010f0),
        ("ELQGetPhonemes", 0x10001000),
        ("FonVidAllofonoVoce", 0x10001210),
        ("FonVidAllofonoAnalisi", 0x100012d0),
        ("FonVidMetaFonema", 0x100012d0),
        ("FonVidAnalisi2Sintesi", 0x100012f0),
    ],
    rules: Rules {
        allofono: Some(allofono),
        meta: meta_default,
    },
};

/// 0x10001210: a phone 0x1952 that ends its word (the next list element
/// belongs to another record) becomes 0x1942 when the next word starts with
/// 0x1fc2 or 0x19c2.
fn allofono(s: &mut Stage, i: &mut usize) {
    let i = *i;
    if s.code(i) != 0x1952 || s.word(i + 1) == s.word(i) {
        return;
    }
    if matches!(s.code(i + 1), 0x1fc2 | 0x19c2) {
        s.set(i, 0x1942);
    }
}
