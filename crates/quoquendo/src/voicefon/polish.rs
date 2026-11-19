//! The Polish voices. LTTS7Krzysztof.dll: `FonVidAllofonoVoce` 0x10001210
//! does nothing, `FonVidMetaFonema` 0x100015c0 (→ 0x10001220) is the
//! default (the id of the first table entry with the phone's code, else 1).
//! LTTS7Zosia.dll: `FonVidAllofonoVoce` 0x10001060 adds a 0x501 between a
//! 0x1be2 or 0xfe2 and the plosive after it; `FonVidMetaFonema` 0x10001590
//! (→ 0x100011f0) the default. Both libraries' `FonVidAllofonoAnalisi`
//! (0x10001290, 0x10001260) runs only in the engine's analysis mode.

use super::{meta_default, Port, Rules, Stage, KIND_PHONE};

pub(super) const KRZYSZTOF: Port = Port {
    voice: "Krzysztof",
    table: 0x100087b0,
    functions: &[
        ("GetVersion", 0x10001150),
        ("ELQGetPhonemes", 0x10001060),
        ("FonVidAllofonoVoce", 0x10001210),
        ("FonVidAllofonoAnalisi", 0x10001290),
        ("FonVidMetaFonema", 0x100015c0),
        ("FonVidAnalisi2Sintesi", 0x100015e0),
    ],
    rules: Rules {
        allofono: None,
        meta: meta_default,
    },
};

pub(super) const ZOSIA: Port = Port {
    voice: "Zosia",
    table: 0x100087b8,
    functions: &[
        ("GetVersion", 0x10001740),
        ("ELQGetPhonemes", 0x10001650),
        ("FonVidAllofonoVoce", 0x10001060),
        ("FonVidAllofonoAnalisi", 0x10001260),
        ("FonVidMetaFonema", 0x10001590),
        ("FonVidAnalisi2Sintesi", 0x100015b0),
    ],
    rules: Rules {
        allofono: Some(zosia_allofono),
        meta: meta_default,
    },
};

/// Zosia's `FonVidAllofonoVoce` (0x10001060): a 0x501 is added after
/// (`ELQListFindElement` + `ELQListAddElementAfter`, in the phone's word)
/// - a 0x1be2 followed in its word by 0x1be2, 0x1001702 or 0x1902;
/// - a 0xfe2 followed by a 0x1be2 of the next word, or in its word by
///   0x1be2, 0x1001702 or 0x1902, except for the run 0x1001702 0x6c1
///   0x107c1 0x2019c2 0x1001702 0x6c1 0x1007c1 after it.
fn zosia_allofono(s: &mut Stage, i: &mut usize) {
    let i = *i;
    let (w, c) = (s.word(i), s.code(i));
    let same = s.word(i + 1) == w;
    let n = s.code(i + 1);
    let plosive = matches!(n, 0x1be2 | 0x1001702 | 0x1902);
    let add = match c {
        0x1be2 => same && plosive,
        0xfe2 if !same => n == 0x1be2,
        0xfe2 => {
            let run = [0x1001702, 0x6c1, 0x107c1, 0x2019c2, 0x1001702, 0x6c1, 0x1007c1];
            plosive && !(0..run.len()).all(|k| s.code(i + 1 + k) == run[k])
        }
        _ => false,
    };
    if add {
        s.insert(i + 1, w, 0x501, KIND_PHONE);
    }
}
