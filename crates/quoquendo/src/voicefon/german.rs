//! LTTS7Katrin.dll and LTTS7Stefan.dll (German): `FonVidAllofonoVoce`
//! 0x100011e0 in both (different code), `FonVidMetaFonema` 0x10002ea0
//! (Katrin: the default) and 0x10002ae0 (Stefan: unaspirated p t k said
//! as b d g).

use super::{meta_default, Port, Rules, Stage};

const TABLE: u32 = 0x1000aeb8;

pub(super) const KATRIN: Port = Port {
    voice: "Katrin",
    table: TABLE,
    functions: &[
        ("GetVersion", 0x10001150),
        ("ELQGetPhonemes", 0x10001060),
        ("FonVidAllofonoVoce", 0x100011e0),
        ("FonVidAllofonoAnalisi", 0x10001c50),
        ("FonVidMetaFonema", 0x10002ea0),
        ("FonVidAnalisi2Sintesi", 0x10002ec0),
    ],
    rules: Rules {
        allofono: Some(katrin),
        meta: meta_default,
    },
};

pub(super) const STEFAN: Port = Port {
    voice: "Stefan",
    table: TABLE,
    functions: &[
        ("GetVersion", 0x10001150),
        ("ELQGetPhonemes", 0x10001060),
        ("FonVidAllofonoVoce", 0x100011e0),
        ("FonVidAllofonoAnalisi", 0x10001880),
        ("FonVidMetaFonema", 0x10002ae0),
        ("FonVidAnalisi2Sintesi", 0x10002b70),
    ],
    rules: Rules {
        allofono: Some(stefan),
        meta: stefan_meta,
    },
};

/// The glottal stop both insert (`Hk`).
const HK: u32 = 0x800014;

/// Words whose vowels in hiatus get no glottal stop: (string address, whole
/// word) — `strcmp` when whole, else `strncmp` over the string's length.
const KATRIN_KEEP: &[(u32, bool)] = &[
    (0x1000af84, false), // ideàl
    (0x1000af7c, false), // ideal
    (0x1000af74, false), // ritu
    (0x1000af6c, false), // georgie
    (0x1000af64, true),  // pòinte
    (0x1000af5c, true),  // diàna
    (0x1000af50, false), // repertòire
    (0x1000af44, true),  // washington
    (0x1000af38, true),  // marihuàna
    (0x1000af30, false), // triùmph
    (0x1000af24, false), // loquendo
    (0x1000af20, false), // web
];

const STEFAN_KEEP: &[(u32, bool)] = &[
    (0x1000af8c, false), // ideàl
    (0x1000af84, false), // ideal
    (0x1000af7c, false), // kroàt
    (0x1000af74, false), // kroat
    (0x1000af6c, false), // ritu
    (0x1000af64, false), // georgie
    (0x1000af5c, true),  // diàna
    (0x1000af50, false), // repertòire
    (0x1000af44, true),  // washington
    (0x1000af38, true),  // marihuàna
    (0x1000af30, false), // triùmph
    (0x1000af24, false), // loquendo
    (0x1000af20, false), // web
];

fn kept(s: &Stage, keep: &[(u32, bool)], text: Option<&[u8]>) -> bool {
    keep.iter().any(|&(at, whole)| {
        let lit = s.pe.cstr(at);
        if whole {
            super::eq(text, lit)
        } else {
            super::starts(text, lit)
        }
    })
}

/// Two vowels in hiatus inside a word: a vowel (`& 0x11`, not `& 0x41`)
/// then a stressed one (`& 0x21`), the second not the word's last phone.
fn hiatus(s: &Stage, i: usize, w: u32, keep: &[(u32, bool)]) -> bool {
    let (prev, cur) = (s.code(i - 1), s.code(i));
    w == s.word(i + 1)
        && prev & 1 != 0
        && prev & 0x11 == 0x11
        && prev & 0x41 != 0x41
        && cur & 1 != 0
        && cur & 0x21 == 0x21
        && !kept(s, keep, s.text(w as i64))
}

/// Katrin 0x100011e0: a glottal stop before a vowel-initial word after a
/// vowel or voiced consonant (the word stressed, or after boundary 4), or
/// after anything when the word is unstressed (grade 3) and not after
/// boundary 4, and inside a word between vowels in hiatus; then
/// `unreleased`.
fn katrin(s: &mut Stage, i: &mut usize) {
    let i = *i;
    if i != 0 {
        let w = s.word(i);
        let rec = s.rec(w as i64);
        let grade = rec.map_or(0, |r| r.f[0x12]);
        let prev_bound = s.rec(w as i64 - 1).map_or(0, |r| r.f[0x11]);
        let (prev, cur) = (s.code(i - 1), s.code(i));
        let insert = if s.word(i - 1) != w {
            let after = prev & 1 != 0 || (prev & 4 != 0 && prev & 0x1000004 == 0x1000004);
            (after && cur & 1 != 0 && (grade == 0 || prev_bound == 4)) || (cur & 1 != 0 && grade == 3 && prev_bound != 4)
        } else {
            hiatus(s, i, w, KATRIN_KEEP)
        };
        if insert {
            s.insert(i, w, HK, 0);
        }
    }
    unreleased(s, i);
}

/// Stefan 0x100011e0: Katrin's, but the sentence's first record counts as
/// after boundary 0x15 and never takes the unstressed word's stop.
fn stefan(s: &mut Stage, i: &mut usize) {
    let i = *i;
    if i == 0 {
        return;
    }
    let w = s.word(i);
    let first = s.first() + w as usize == 0;
    let grade = s.rec(w as i64).map_or(0, |r| r.f[0x12]);
    let prev_bound = if first { 0x15 } else { s.rec(w as i64 - 1).map_or(0, |r| r.f[0x11]) };
    let (prev, cur) = (s.code(i - 1), s.code(i));
    let insert = if s.word(i - 1) != w {
        let after = prev & 1 != 0 || (prev & 4 != 0 && prev & 0x1000004 == 0x1000004);
        (after && cur & 1 != 0 && (grade == 0 || (!first && prev_bound == 4))) || (cur & 1 != 0 && grade == 3 && !first && prev_bound != 4)
    } else {
        hiatus(s, i, w, STEFAN_KEEP)
    };
    if insert {
        s.insert(i, w, HK, 0);
    }
    unreleased(s, i);
}

/// The tail of both (Katrin 0x10001543, Stefan 0x100015da): b d g inside a
/// word, after a phone that is not a voiceless stop or fricative and before
/// a consonant other than v h j l R (d: nor z), are unreleased (`Hub Hud
/// Hug`); d also at the word's end.
fn unreleased(s: &mut Stage, i: usize) {
    if i == 0 {
        return;
    }
    let (w, cur, prev, next) = (s.word(i), s.code(i), s.code(i - 1), s.code(i + 1));
    if s.word(i - 1) != w {
        return;
    }
    let prev_ok = prev & 0x14 != 0x14 && ![0x20104, 0x10104, 0x80104, 0x2004104, 0x100104, 0x2011004, 0x3021004, 0x2005004].contains(&prev);
    let next_ok = next & 4 != 0 && ![0x3004104, 0x800104, 0x8108_0404, 0x1010804, 0x1200104].contains(&next);
    if !prev_ok || !next_ok {
        return;
    }
    match cur {
        0x3002014 => s.set(i, 0x23002014),
        0x3010014 if next != 0x1010104 || w != s.word(i + 1) => s.set(i, 0x23010014),
        0x3100014 => s.set(i, 0x23100014),
        _ => {}
    }
}

/// Stefan 0x10002ae0: p t k (unaspirated) as b d g (ids 0x3e 0x41 0x44).
fn stefan_meta(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    let id = match s.code(*i) {
        0x2002014 => 0x3e,
        0x2010014 => 0x41,
        0x2100014 => 0x44,
        _ => return meta_default(s, i, out),
    };
    out.push(id);
    *i += 1;
}
