//! LTTS7Alan.dll (Australian English): `FonVidAllofonoVoce` 0x100014a0 and
//! `FonVidMetaFonema` 0x100039c0, Grace's code (see `grace.rs`: the word
//! table 0x10001360/0x10001210, the t rules, the shared helpers) with Alan's
//! own table, literals and differences: "with" r only before a vowel, no
//! "the"/`d`/`4ff2` rules and no flapped t in "it", d before `w` across a
//! word joined into 0x1702, and its own MetaFonema ids.
//!
//! As in Grace, `local_26` (`'I'` for the FON ctx's byte +0x18 in 7..10)
//! only gates the dead "are" rule (0x1000aa84): the byte is the language
//! module's stale token count, taken as below 7 (`'D'`).

use super::grace::{kind, lookup, meta_common, replace, t_rules, texts, vowel, wp, Lib, TLits};
use super::{eq, starts, Port, Rules, Stage, KIND_PHONE};

pub(super) const ALAN: Port = Port {
    voice: "Alan",
    table: 0x1000a960,
    functions: &[
        ("GetVersion", 0x100010f0),
        ("ELQGetPhonemes", 0x10001000),
        ("FonVidAllofonoVoce", 0x100014a0),
        ("FonVidAllofonoAnalisi", 0x10002900),
        ("FonVidMetaFonema", 0x100039c0),
        ("FonVidAnalisi2Sintesi", 0x100038d0),
        ("FonVidIngloba", 0x10003970),
    ],
    rules: Rules {
        allofono: Some(alan),
        meta: alan_meta,
    },
};

const LIB: Lib = Lib {
    words: 0x1000d000,
    words_end: 0x1000aa14,
    a2s_index: 0x10003940,
    a2s_jump: 0x10003910,
    a2s_max: 0x2a, // 0x100038da `cmp ecx, 0x2a`
};

// Literals (the immediates in 0x100014a0).
const TIES: u32 = 0x1000aa18;
const TY: u32 = 0x1000aa20;
const TOR: u32 = 0x1000aa24;
const TING: u32 = 0x1000aa28;
const TIV: u32 = 0x1000aa30;
const TED: u32 = 0x1000aa34;
const ELY: u32 = 0x1000aa38;
const LY: u32 = 0x1000aa3c;
const DIRECT: u32 = 0x1000aa40;
const IO: u32 = 0x1000aa48;
const IA: u32 = 0x1000aa4c;
const TO: u32 = 0x1000aa50;
const USED: u32 = 0x1000aa54;
const WITH: u32 = 0x1000aa5c;
const SHIRE: u32 = 0x1000aa64;
const AUSTRA: u32 = 0x1000aa6c;
const TRADITIONAL: u32 = 0x1000aa74;
const LLY: u32 = 0x1000aa80;

const T: TLits = TLits { ly: LY, ely: ELY, ted: TED, tiv: TIV, ting: TING, tor: TOR, ty: TY, ties: TIES, it: None, to: TO };

fn set_kind(s: &mut Stage, k: usize, v: u8) {
    if let Some(p) = s.list.get_mut(k) {
        p.kind = v;
    }
}

/// 0x100014a0 `FonVidAllofonoVoce` (Alan).
fn alan(s: &mut Stage, pi: &mut usize) {
    let tx = texts(s, *pi);
    let cur = tx.cur.as_deref();
    let next = tx.next.as_deref();
    let grade = tx.grade;
    let len = cur.map_or(0, <[u8]>::len);
    let lit = |s: &Stage, va: u32| s.pe.cstr(va).to_vec();
    let ends = |s: &Stage, va: u32| cur.is_some_and(|t| t.ends_with(s.pe.cstr(va)));
    let is = |s: &Stage, t: Option<&[u8]>, va: u32| eq(t, s.pe.cstr(va));

    let i = *pi;
    let c0 = s.code(i);
    if c0 == 0x17 || c0 == 0x13 {
        return;
    }
    if i == 0 || wp(s, i - 1) || [0x9e2, 0x9f2, 0x10ce, 0x401ece].contains(&s.code(i - 1)) {
        replace(s, &LIB, pi);
    }
    let i = *pi;
    if wp(s, i) && s.code(i) == 0x18c2 && !vowel(s.code(i + 1)) && lookup(s, &LIB, cur, grade).is_some() {
        let k = kind(s, i);
        set_kind(s, i.wrapping_sub(1), k);
        s.delete(i);
        *pi = i.wrapping_sub(1);
    }
    let i = *pi;
    let c = |s: &Stage, k: usize| s.code(k);
    let wd = |s: &Stage, k: usize| s.word(k);
    match c(s, i) {
        0x1ff2 | 0x3ff2 => {
            if [0x17, 0x13].contains(&c(s, i + 1)) {
                s.set(i, 0x5ff2);
            }
        }
        0xff2 | 0x2ff2 => {
            if [0x17, 0x13].contains(&c(s, i + 1)) {
                s.set(i, 0x4ff2);
            }
        }
        0x1942 => {
            if is(s, cur, USED) && next.is_some() && is(s, next, TO) {
                s.set(i, 0x1952);
                s.set(i + 1, 0x59f2);
                *pi += 1;
            }
        }
        0x19e2 => {
            if i != 0 && wp(s, i) && c(s, i - 1) == 0x19c2 && [0x401ec2, 0x10c2].contains(&c(s, i + 1)) {
                s.set(i, 0x59e2);
                return;
            }
            if wd(s, i) == wd(s, i + 2) && c(s, i + 1) == 0x10c2 {
                s.delete(i);
                s.set(i, 0x1702);
            }
        }
        0x1b52 => {
            if is(s, cur, WITH) && vowel(c(s, i + 1)) {
                s.set(i, 0x1b42);
            }
        }
        0x1752 => {
            if len >= 5 && ends(s, SHIRE) && (wp(s, i + 1) || wp(s, i + 2)) {
                s.set(i + 1, 0x4785);
                *pi += 1;
            }
        }
        0x781 | 0xe445 => {
            if i != 0 && (i == 1 || wp(s, i - 2)) && c(s, i - 1) == 0x19e2 && len >= 6 && starts(cur, &lit(s, DIRECT)) {
                s.set(i, 0x501);
            }
        }
        0x261 => {
            if len >= 6
                && starts(cur, &lit(s, AUSTRA))
                && (i == 0 || wp(s, i - 1) || [0x9e2, 0x9f2, 0x10ce, 0x401ece].contains(&c(s, i - 1)))
            {
                s.set(i, 0x501);
            }
        }
        0x501 => {
            if len >= 3 && ends(s, LLY) && c(s, i + 1) == 0x18a2 && c(s, i + 2) == 0x7c1 && wp(s, i + 2) {
                s.set(i, 0x218a2);
                s.delete(i + 1);
                return;
            }
            if i != 0
                && c(s, i - 1) == 0x1752
                && c(s, i + 1) == 0x19c2
                && (c(s, i + 2) == 0x218a2 || (c(s, i + 2) == 0x501 && c(s, i + 3) == 0x18a2))
                && !(cur.is_some() && is(s, cur, TRADITIONAL))
            {
                s.set(i, 0x219c2);
                s.delete(i + 1);
            }
        }
        0x4785 => {
            if i == 0 {
                return;
            }
            if wp(s, i) && len > 1 && ends(s, IA) {
                s.set(i, 0x501);
                let w = wd(s, i);
                s.insert(i, w, 0x7c1, KIND_PHONE);
                *pi += 1;
                return;
            }
            if c(s, i + 1) == 0x501 && wp(s, i + 1) && len >= 2 && ends(s, IO) {
                s.set(i, 0x7c1);
                s.set(i + 1, 0xad05);
                *pi += 1;
            }
        }
        0x59e2 => {
            if i != 0
                && wp(s, i)
                && c(s, i - 1) == 0x19c2
                && [0x19c2, 0x1fc2, 0x1b42, 0x20001b52, 0x20001ff2, 0x200019f2, 0x20000ff2].contains(&c(s, i + 1))
            {
                let k = kind(s, i);
                set_kind(s, i - 1, k);
                s.delete(i);
            }
        }
        0x39f2 | 0x59f2 => t_rules(s, pi, cur, &T),
        _ => {}
    }
}

/// 0x100039c0 `FonVidMetaFonema` (Alan): 0x1ff2 0xff2 0x19f2 0x5ff2 0x4fe2
/// 0x59f2 as ids 0x3a 0x3e 0x3c 0x3b 0x38 0x3d; 0x2f 0x9e2 0x10ce 0x401ece
/// after a `w`/`p` element say nothing; the rest the default (0x10002890);
/// every id then through 0x100038d0.
fn alan_meta(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    meta_common(
        s,
        &LIB,
        i,
        out,
        &[(0x1ff2, 0x3a), (0xff2, 0x3e), (0x19f2, 0x3c), (0x5ff2, 0x3b), (0x4fe2, 0x38), (0x59f2, 0x3d)],
        &[0x2f, 0x9e2, 0x10ce, 0x401ece],
    );
}
