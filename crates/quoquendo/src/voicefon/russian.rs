//! LTTS7Dmitri.dll and LTTS7Olga.dll (Russian, the same code at the same
//! addresses): `FonVidAllofonoVoce` 0x10001210, `FonVidMetaFonema`
//! 0x100014b0 (the default 0x100013d0 but for 0x942 after a word's end).

use super::{meta_default, Port, Rules, Stage};

const FUNCTIONS: &[(&str, u32)] = &[
    ("FonVidAllofonoVoce", 0x10001210),
    ("FonVidAllofonoAnalisi", 0x10001440),
    ("FonVidMetaFonema", 0x100014b0),
    ("FonVidAnalisi2Sintesi", 0x10001460),
    ("FonVidIngloba", 0x10001480),
];

pub(super) const DMITRI: Port = Port {
    voice: "Dmitri",
    table: 0x10008728,
    functions: &[
        ("GetVersion", 0x100010f0),
        ("ELQGetPhonemes", 0x10001000),
        FUNCTIONS[0],
        FUNCTIONS[1],
        FUNCTIONS[2],
        FUNCTIONS[3],
        FUNCTIONS[4],
    ],
    rules: Rules {
        allofono: Some(allofono),
        meta: meta_default,
    },
};

pub(super) const OLGA: Port = Port {
    voice: "Olga",
    table: 0x10008750,
    functions: &[
        ("GetVersion", 0x10001150),
        ("ELQGetPhonemes", 0x10001060),
        FUNCTIONS[0],
        FUNCTIONS[1],
        FUNCTIONS[2],
        FUNCTIONS[3],
        FUNCTIONS[4],
    ],
    rules: Rules {
        allofono: Some(allofono),
        meta: meta_default,
    },
};

/// A word's last phone (`w`) or a pause (`p`).
fn ends(kind: u8) -> bool {
    kind == b'w' || kind == b'p'
}

/// 0x10001210: inside a word (neither this phone nor the one before ends
/// one) s/z soften before tʲ/nʲ, dʲ/nʲ; t softens before nʲ; `ə j ə` at a
/// word's end becomes the voice's unit 0xe6c5.
fn allofono(s: &mut Stage, i: &mut usize) {
    let i = *i;
    let kind = |s: &Stage, k: usize| s.list.get(k).map_or(0, |p| p.kind);
    let inside = |s: &Stage| !ends(kind(s, i)) && i != 0 && !ends(kind(s, i - 1));
    match s.code(i) {
        0x1952 if inside(s) && matches!(s.code(i + 1), 0x2019f2 | 0x2019c2) => s.set(i, 0x201952),
        0x1942 if inside(s) && matches!(s.code(i + 1), 0x2019e2 | 0x2019c2) => s.set(i, 0x201942),
        0x501 if s.code(i + 1) == 0x10c2 && s.code(i + 2) == 0x501 && ends(kind(s, i + 2)) => {
            s.delete(i);
            s.delete(i);
            s.set(i, 0xe6c5);
        }
        0x19f2 if s.code(i + 1) == 0x2019c2 && !ends(kind(s, i)) => s.set(i, 0x2019f2),
        _ => {}
    }
}
