//! LTTS7Simon.dll (British English, the older code layout): its
//! `FonVidAllofonoVoce` 0x100014a0 (with the replacement dictionary
//! 0x10001350 / lookup 0x10001210, the same code as Elizabeth's, its end
//! word at 0x1000aa04) and `FonVidMetaFonema` 0x10002be0 (default mapping
//! 0x10001d50).
//!
//! Literals are read from the library by address; the comment after each
//! address is what it holds.

use super::elizabeth::{lookup, replace};
use super::{meta_default, Port, Rules, Stage, KIND_PHONE};

pub(super) const SIMON: Port = Port {
    voice: "Simon",
    table: 0x1000a978,
    functions: &[
        ("GetVersion", 0x10001150),
        ("ELQGetPhonemes", 0x10001060),
        ("FonVidAllofonoVoce", 0x100014a0),
        ("FonVidAllofonoAnalisi", 0x10001dc0),
        ("FonVidMetaFonema", 0x10002be0),
        ("FonVidAnalisi2Sintesi", 0x10002ce0),
        ("FonVidIngloba", 0x10002d80),
    ],
    rules: Rules {
        allofono: Some(simon),
        meta: simon_meta,
    },
};

/// The dictionary's closing entry's word (`-1`).
const DICT_END: u32 = 0x1000aa04;

const SUR: u32 = 0x1000aa08; // sur
const IO: u32 = 0x1000aa0c; // io
const IA: u32 = 0x1000aa10; // ia
const WITH: u32 = 0x1000aa14; // with
const TO: u32 = 0x1000aa1c; // to
const USED: u32 = 0x1000aa20; // used
const DIRECT: u32 = 0x1000aa28; // direct
const ACCEPT: u32 = 0x1000aa30; // accept
const OBTAIN: u32 = 0x1000aa38; // obtain
const TRADITIONAL: u32 = 0x1000aa40; // traditional
const ARE: u32 = 0x1000aa4c; // are

/// Hg Hj Hw: the glides that leave a word's dictionary check on.
const GLIDES: [u32; 3] = [0x1800014, 0x1080404, 0x1102404];

fn lit(s: &Stage, t: Option<&[u8]>, at: u32) -> bool {
    super::eq(t, s.pe.cstr(at))
}

fn starts(s: &Stage, t: Option<&[u8]>, at: u32) -> bool {
    super::starts(t, s.pe.cstr(at))
}

fn ends(s: &Stage, t: Option<&[u8]>, at: u32) -> bool {
    t.is_some_and(|t| t.ends_with(s.pe.cstr(at)))
}

/// Element `i` starts its word, or follows a glide (the dictionary's and
/// obtain/accept's condition).
fn starting(s: &Stage, i: usize) -> bool {
    i == 0 || s.word(i) != s.word(i - 1) || GLIDES.contains(&s.code(i - 1))
}

/// Simon 0x100014a0.
fn simon(s: &mut Stage, ip: &mut usize) {
    let i = *ip;
    let w = s.word(i);
    let wi = w as i64;
    let text = s.text(wi).map(<[u8]>::to_vec);
    let t = text.as_deref();
    let grade = s.rec(wi).map_or(0, |r| r.f[0x12]);
    // the next record's text ("" 0x1000a3ae at the chunk's record count)
    let next = if w as usize == s.count() { Some(Vec::new()) } else { s.text(wi + 1).map(<[u8]>::to_vec) };
    let next = next.as_deref();
    // ctx+0x18 in 7..=10: 'I', else 'D' — the module's stale token count,
    // below 7 at FON time in ordinary sentences (as kate.rs/dave.rs): 'D'
    let question = false;

    if starting(s, i) {
        replace(s, ip, DICT_END);
    }
    // a dictionary word's final Hr before a consonant goes
    let i = *ip;
    if s.word(i) != s.word(i + 1)
        && s.code(i) == 0x1010404
        && s.code(i + 1) & 1 == 0
        && lookup(s, DICT_END, t, grade).is_some()
        && i < s.list.len()
    {
        s.delete(i);
        *ip = i.wrapping_sub(1);
        if i == 0 {
            return;
        }
    }
    let i = *ip;
    match s.code(i) {
        // th j u|`u:|`HUE: T$
        0x12010014 => {
            if s.code(i + 1) == 0x81080404 && [0x1030a1, 0x85050a3].contains(&s.code(i + 2)) {
                s.set(i, 0x2021004);
                s.delete(i + 1);
            }
        }
        // $ `HUE in ...sur...: `Oa:
        0x85050a3 => {
            if i != 0 && s.code(i - 1) == 0x20104 && super::contains(t, s.pe.cstr(SUR)) {
                s.set(i, 0x1210a1);
            }
        }
        // d j u|`u:|`HUE in one word: Dg
        0x3010014 => {
            if s.word(i) == s.word(i + 1)
                && s.code(i + 1) == 0x81080404
                && [0x103011, 0x1030a1, 0x85050a3].contains(&s.code(i + 2))
            {
                s.delete(i);
                s.set(i, 0x3021004);
            }
        }
        0x8404493 => {
            if i == 0 {
                return;
            }
            // word-final HIE in -ia: i HEh
            if s.word(i) != s.word(i + 1) && ends(s, t, IA) {
                s.set(i, 0x2411);
                let w = s.word(i);
                // the added element's kind is stack garbage in the original
                s.insert(i + 1, w, 0x10851, KIND_PHONE);
                return;
            }
            // HIE HEh closing a word in -io: i HEU
            if s.code(i + 1) == 0x10851 && s.word(i + 1) != s.word(i + 2) && ends(s, t, IO) {
                s.set(i, 0x2411);
                s.set(i + 1, 0x82810893);
                *ip += 1;
            }
        }
        // with before a vowel: Dh
        0x2008104 => {
            if lit(s, t, WITH) && s.code(i + 1) & 1 != 0 {
                s.set(i, 0x3008104);
            }
        }
        // used to: s Hut
        0x1010104 => {
            if lit(s, t, USED) && lit(s, next, TO) {
                s.set(i, 0x10104);
                s.set(i + 1, 0x22010014);
                *ip += 1;
            }
        }
        0x40411 => {
            if starts(s, t, ACCEPT) {
                s.set(i, 0x20411);
            }
        }
        // d I starting direct: HAI
        0x4411 => {
            if i != 0
                && (i == 1 || s.word(i - 1) != s.word(i - 2))
                && s.code(i - 1) == 0x3010014
                && starts(s, t, DIRECT)
            {
                s.set(i, 0x2280893);
            }
        }
        0x10851 => he(s, ip, w, t, question),
        _ => {}
    }
}

/// HEh (0x10851 of 0x100014a0).
fn he(s: &mut Stage, ip: &mut usize, w: u32, t: Option<&[u8]>, question: bool) {
    let i = *ip;
    if lit(s, t, ARE) && w < 2 && question && s.count() < 5 {
        s.set(i, 0x81091);
    } else if t.is_some_and(|t| t.len() > 5) {
        if starts(s, t, OBTAIN) && starting(s, i) && s.code(i + 1) == 0x23002014 {
            // obtain: Ao Hup
            s.set(i, 0x181011);
            s.set(i + 1, 0x22002014);
            *ip += 1;
        } else if starts(s, t, ACCEPT) && starting(s, i) {
            s.set(i, 0x20411);
        }
    }
    // LAB_100016b0: $ HEh n l (or HEh n HEh l) not in traditional: $ Hn l
    let i = *ip;
    if i != 0
        && s.code(i - 1) == 0x20104
        && s.code(i + 1) == 0x1010024
        && (s.code(i + 2) == 0x9010804 || (s.code(i + 2) == 0x10851 && s.code(i + 3) == 0x1010804))
        && !lit(s, t, TRADITIONAL)
    {
        s.set(i, 0x9010024);
        s.delete(i + 1);
    }
}

/// Simon 0x10002be0: t p Hud k Hub Hug as ids 0x3b 0x39 0x35 0x3d 0x32 0x38
/// (d b d g b g); Hw, 0x2f, Hj, Hg starting a word (not the first) are
/// dropped; the rest as 0x10001d50 (the default).
fn simon_meta(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    let id = match s.code(*i) {
        0x2010014 => 0x3b,
        0x2002014 => 0x39,
        0x23010014 => 0x35,
        0x2100014 => 0x3d,
        0x23002014 => 0x32,
        0x23100014 => 0x38,
        0x1102404 | 0x2f | 0x1080404 | 0x1800014 if *i != 0 && s.word(*i) != s.word(*i - 1) => {
            *i += 1;
            return;
        }
        _ => return meta_default(s, i, out),
    };
    out.push(id);
    *i += 1;
}
