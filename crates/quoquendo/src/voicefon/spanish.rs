//! The Spanish voices' libraries. `FonVidAllofonoVoce` does nothing in
//! Jorge 0x100011e0, Carmen 0x100011e0, Diego 0x100011e0 and Carlos
//! 0x10001210; `FonVidMetaFonema` is the default in Jorge 0x100013b0,
//! Carmen 0x10001300, Leonor 0x100013e0, Esperanza 0x100013d0 and Soledad
//! 0x100014c0.

use super::{meta_default, Port, Rules, Stage};

const fn port(voice: &'static str, table: u32, functions: &'static [(&'static str, u32)], allofono: Option<fn(&mut Stage, &mut usize)>, meta: fn(&Stage, &mut usize, &mut Vec<u8>)) -> Port {
    Port {
        voice,
        table,
        functions,
        rules: Rules { allofono, meta },
    }
}

pub(super) const JORGE: Port = port(
    "Jorge",
    0x10008878,
    &[
        ("GetVersion", 0x10001150),
        ("ELQGetPhonemes", 0x10001060),
        ("FonVidAllofonoVoce", 0x100011e0),
        ("FonVidAllofonoAnalisi", 0x10001260),
        ("FonVidMetaFonema", 0x100013b0),
        ("FonVidAnalisi2Sintesi", 0x100013d0),
    ],
    None,
    meta_default,
);

pub(super) const CARMEN: Port = port(
    "Carmen",
    0x10008850,
    &[
        ("GetVersion", 0x100010f0),
        ("ELQGetPhonemes", 0x10001000),
        ("FonVidAllofonoVoce", 0x100011e0),
        ("FonVidAllofonoAnalisi", 0x10001260),
        ("FonVidMetaFonema", 0x10001300),
        ("FonVidAnalisi2Sintesi", 0x10001320),
    ],
    None,
    meta_default,
);

pub(super) const LEONOR: Port = port(
    "Leonor",
    0x10008898,
    &[
        ("GetVersion", 0x10001150),
        ("ELQGetPhonemes", 0x10001060),
        ("FonVidAllofonoVoce", 0x100011e0),
        ("FonVidAllofonoAnalisi", 0x10001340),
        ("FonVidMetaFonema", 0x100013e0),
        ("FonVidAnalisi2Sintesi", 0x10001400),
    ],
    Some(leonor),
    meta_default,
);

pub(super) const DIEGO: Port = port(
    "Diego",
    0x100087a0,
    &[
        ("GetVersion", 0x100010f0),
        ("ELQGetPhonemes", 0x10001000),
        ("FonVidAllofonoVoce", 0x100011e0),
        ("FonVidAllofonoAnalisi", 0x10001260),
        ("FonVidMetaFonema", 0x10001280),
        ("FonVidAnalisi2Sintesi", 0x100012d0),
    ],
    None,
    diego_meta,
);

pub(super) const ESPERANZA: Port = port(
    "Esperanza",
    0x100087b0,
    &[
        ("GetVersion", 0x100010f0),
        ("ELQGetPhonemes", 0x10001000),
        ("FonVidAllofonoVoce", 0x100011e0),
        ("FonVidAllofonoAnalisi", 0x10001280),
        ("FonVidMetaFonema", 0x100013d0),
        ("FonVidAnalisi2Sintesi", 0x100013f0),
    ],
    Some(esperanza),
    meta_default,
);

pub(super) const FRANCISCA: Port = port(
    "Francisca",
    0x100087f0,
    &[
        ("GetVersion", 0x10001150),
        ("ELQGetPhonemes", 0x10001060),
        ("FonVidAllofonoVoce", 0x100011e0),
        ("FonVidAllofonoAnalisi", 0x100012a0),
        ("FonVidMetaFonema", 0x100012c0),
        ("FonVidAnalisi2Sintesi", 0x10001310),
    ],
    Some(francisca),
    francisca_meta,
);

pub(super) const CARLOS: Port = port(
    "Carlos",
    0x100085d8,
    &[
        ("GetVersion", 0x100010f0),
        ("ELQGetPhonemes", 0x10001000),
        ("FonVidAllofonoVoce", 0x10001210),
        ("FonVidAllofonoAnalisi", 0x10001290),
        ("FonVidMetaFonema", 0x100013e0),
        ("FonVidAnalisi2Sintesi", 0x10001370),
    ],
    None,
    synthesis_meta,
);

pub(super) const SOLEDAD: Port = port(
    "Soledad",
    0x10008608,
    &[
        ("GetVersion", 0x10001150),
        ("ELQGetPhonemes", 0x10001060),
        ("FonVidAllofonoVoce", 0x10001210),
        ("FonVidAllofonoAnalisi", 0x10001300),
        ("FonVidMetaFonema", 0x100014c0),
        ("FonVidAnalisi2Sintesi", 0x100014e0),
    ],
    Some(soledad),
    meta_default,
);

pub(super) const XIMENA: Port = port(
    "Ximena",
    0x100085f8,
    &[
        ("GetVersion", 0x100015c0),
        ("ELQGetPhonemes", 0x100014d0),
        ("FonVidAllofonoVoce", 0x10001060),
        ("FonVidAllofonoAnalisi", 0x10001130),
        ("FonVidMetaFonema", 0x10001490),
        ("FonVidAnalisi2Sintesi", 0x10001420),
    ],
    Some(ximena),
    synthesis_meta,
);

/// Leonor 0x100011e0: s before the trill `r:` merges with it (0x501014c);
/// DH (0x1008404) after a stressed `a` and before another word starting
/// with a vowel, a lateral, a nasal or a voiced fricative is dropped.
fn leonor(s: &mut Stage, i: &mut usize) {
    let i = *i;
    match s.code(i) {
        0x10104 if s.code(i + 1) == 0x5010044 => {
            s.list[i].kind = s.list[i + 1].kind;
            s.delete(i);
            s.set(i, 0x501014c);
        }
        0x1008404 if i != 0 && s.code(i - 1) == 0x80821 && s.word(i + 1) != s.word(i) => {
            let n = s.code(i + 1);
            if n & 1 != 0 || n & 0x804 == 0x804 || n & 0x24 == 0x24 || n & 0x44 == 0x44 {
                s.list[i].kind = s.list[i + 1].kind;
                s.delete(i);
            }
        }
        _ => {}
    }
}

/// Esperanza 0x100011e0: the palatal fricative `Y` (0x1080104) said as the
/// stop `J` (0x1080014).
fn esperanza(s: &mut Stage, i: &mut usize) {
    if s.code(*i) == 0x1080104 {
        s.set(*i, 0x1080014);
    }
}

/// Francisca 0x100011e0: `h` after a vowel and before f or t becomes
/// 0x1800104 (said as `h`, id 0x21, by 0x100012c0).
fn francisca(s: &mut Stage, i: &mut usize) {
    let i = *i;
    if s.code(i) == 0x800104 && i != 0 && s.code(i - 1) & 1 != 0 && matches!(s.code(i + 1), 0x2004104 | 0x2008014) {
        s.set(i, 0x1800104);
    }
}

/// Soledad 0x10001210 (engine codes): word-final s before DH/BH-like
/// 0x19a6/0x18a2 dropped; word-final `dd` (0x1bf2) before an unvoiced
/// consonant of the next word softened to DH (0x1ac2).
fn soledad(s: &mut Stage, i: &mut usize) {
    let i = *i;
    let other = s.word(i + 1) != s.word(i);
    match s.code(i) {
        0x1952 if other && matches!(s.code(i + 1), 0x19a6 | 0x18a2) => s.delete(i),
        0x1bf2 if other && s.code(i + 1) & 3 == 2 && s.code(i + 1) & 0x10 != 0 => s.set(i, 0x1ac2),
        _ => {}
    }
}

/// Ximena 0x10001060 (engine codes): 0x1142 after BH or DH of the same
/// word becomes 0x11e2.
fn ximena(s: &mut Stage, i: &mut usize) {
    let i = *i;
    if s.code(i) == 0x1142 && i != 0 && s.word(i - 1) == s.word(i) && matches!(s.code(i - 1), 0x1ec2 | 0x1ac2) {
        s.set(i, 0x11e2);
    }
}

/// Diego 0x10001280: 0x1800104 said as `h` (id 0x20).
fn diego_meta(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    if s.code(*i) == 0x1800104 {
        out.push(0x20);
        *i += 1;
    } else {
        meta_default(s, i, out);
    }
}

/// Francisca 0x100012c0: 0x1800104 said as `h` (id 0x21).
fn francisca_meta(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    if s.code(*i) == 0x1800104 {
        out.push(0x21);
        *i += 1;
    } else {
        meta_default(s, i, out);
    }
}

/// Carlos 0x100013e0, Ximena 0x10001490: the default id, then the
/// library's Analisi2Sintesi (0x10001370, 0x10001420): the analysis
/// allophones as the phones synthesis has (HDH→DH, HBH→BH, HGH→GH, HB→b,
/// Hdd→dd, HG→g, vs→s).
fn synthesis_meta(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    let at = out.len();
    meta_default(s, i, out);
    if let Some(id) = out.get_mut(at) {
        *id = match *id {
            0x13 => 0x12,
            0x14 => 0x10,
            0x15 => 0x11,
            0x1a => 0x19,
            0x1c => 0x1b,
            0x1e => 0x1d,
            0x24 => 0x23,
            x => x,
        };
    }
}
