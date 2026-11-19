//! LTTS7Elizabeth.dll (British English): `FonVidAllofonoVoce` 0x100014a0
//! (with its per-word replacement dictionary 0x10001350 / lookup
//! 0x10001210, shared in form with Simon's) and `FonVidMetaFonema`
//! 0x10003360 (default mapping 0x10002420).
//!
//! Literals are read from the library by address; the comment after each
//! address is what it holds.

use super::{meta_default, Port, Rules, Stage, KIND_PHONE, KIND_WORD_END};

pub(super) const ELIZABETH: Port = Port {
    voice: "Elizabeth",
    table: 0x1000a950,
    functions: &[
        ("GetVersion", 0x100010f0),
        ("ELQGetPhonemes", 0x10001000),
        ("FonVidAllofonoVoce", 0x100014a0),
        ("FonVidAllofonoAnalisi", 0x10002490),
        ("FonVidMetaFonema", 0x10003360),
        ("FonVidAnalisi2Sintesi", 0x10003290),
        ("FonVidIngloba", 0x10003320),
    ],
    rules: Rules {
        allofono: Some(elizabeth),
        meta: elizabeth_meta,
    },
};

/// The replacement dictionary (Elizabeth and Simon both at 0x1000d000):
/// 0x54-byte entries, the word at +0, its codes from +0x10 up to a 0
/// (0x48: the replaced word's last code), the stress grade it applies to
/// at +0x50 (-1: any).
pub(super) const DICT: u32 = 0x1000d000;
/// The closing entry's word (`-1`), the first 3 bytes compared.
const DICT_END: u32 = 0x1000aa0c;

const ANSWER: u32 = 0x1000aa1c; // answer
const ASK: u32 = 0x1000aa18; // ask
const IA: u32 = 0x1000aa14; // ia
const IO: u32 = 0x1000aa10; // io
const ON: u32 = 0x1000aaec; // on
const ONS: u32 = 0x1000aae8; // ons
const ACCEPT: u32 = 0x1000aaf0; // accept
const USED: u32 = 0x1000aa28; // used
const TO: u32 = 0x1000aa24; // to
const ARE: u32 = 0x1000aa3c; // are
const TRADITIONAL: u32 = 0x1000aa30; // traditional
const CON: u32 = 0x1000aa78; // con
const RECON: u32 = 0x1000aa48; // recon
const SEMICON: u32 = 0x1000aa40; // semicon
/// connect conceal concern control console: `con` words keeping their
/// vowel (0x10001900).
const CON_KEEP: [u32; 5] = [0x1000aa70, 0x1000aa68, 0x1000aa60, 0x1000aa58, 0x1000aa50];
/// Names whose final `-son` keeps HEh n (0x1000179f): emerson emmerson
/// ferguson fergusson jackson jefferson larson livingston nicholson
/// nicolson.
const SON_KEEP: [u32; 10] = [
    0x1000aae0, 0x1000aad4, 0x1000aac8, 0x1000aabc, 0x1000aab4, 0x1000aaa8, 0x1000aaa0, 0x1000aa94, 0x1000aa88,
    0x1000aa7c,
];

fn is(s: &Stage, t: Option<&[u8]>, at: &[u32]) -> bool {
    at.iter().any(|&a| super::eq(t, s.pe.cstr(a)))
}

fn starts(s: &Stage, t: Option<&[u8]>, at: u32) -> bool {
    super::starts(t, s.pe.cstr(at))
}

/// `_strncmp(text + len - n, lit, n) == 0` with `len >= n`.
fn ends(s: &Stage, t: Option<&[u8]>, at: u32) -> bool {
    t.is_some_and(|t| t.ends_with(s.pe.cstr(at)))
}

/// 0x10001210 (Elizabeth; Simon's is the same with its own end word): the
/// entry for `text` at stress grade `grade`: the first with the word,
/// skipping same-word entries for other grades (an any-grade entry ends the
/// skip); none when absent.
pub(super) fn lookup(s: &Stage, end: u32, text: Option<&[u8]>, grade: u8) -> Option<u32> {
    let text = text?;
    let name = |k: u32| s.pe.cstr(DICT + k * 0x54);
    let end = s.pe.cstr(end);
    let mut k = 0;
    while name(k) != end && name(k) != text {
        k += 1;
    }
    loop {
        let g = s.pe.u32(DICT + k * 0x54 + 0x50);
        if name(k) != text || g == u32::MAX || g == grade as u32 {
            break;
        }
        k += 1;
    }
    (name(k) == text).then_some(k)
}

/// 0x10001350: a dictionary word's elements (from `*i`, every one with its
/// record) are replaced by the entry's codes, the last one a word end.
pub(super) fn replace(s: &mut Stage, i: &mut usize, end: u32) {
    let start = *i;
    let w = s.word(start);
    let grade = s.rec(w as i64).map_or(0, |r| r.f[0x12]);
    let text = s.text(w as i64).map(<[u8]>::to_vec);
    let Some(k) = lookup(s, end, text.as_deref(), grade) else { return };
    let mut code = s.code(start);
    while *i < s.list.len() && s.word(*i) == w {
        code = s.code(*i);
        s.delete(*i);
    }
    let mut at = DICT + k * 0x54 + 0x10;
    loop {
        let c = s.pe.u32(at);
        if c == 0 {
            break;
        }
        s.insert(*i, w, if c == 0x48 { code } else { c }, KIND_PHONE);
        *i += 1;
        at += 4;
    }
    if let Some(p) = i.checked_sub(1).and_then(|j| s.list.get_mut(j)) {
        p.kind = KIND_WORD_END;
    }
    *i = start;
}

/// A pause (0x17 `@`, 0x13 `#`).
fn pause(c: u32) -> bool {
    c == 0x17 || c == 0x13
}

/// The repeated `-on`/`-ons` test (e.g. 0x10001e90): `X HEh n` closing the
/// word (`on` with a pause after n, `ons` with one after s).
fn on_ons(s: &Stage, i: usize, t: Option<&[u8]>) -> bool {
    s.code(i + 1) == 0x501
        && s.code(i + 2) == 0x19c2
        && s.word(i) == s.word(i + 2)
        && ((ends(s, t, ON) && pause(s.code(i + 3))) || (ends(s, t, ONS) && pause(s.code(i + 4))))
}

/// LAB_1000202f: `X HEh n` -> `X' Hn`, `*i` past X.
fn syllabic(s: &mut Stage, i: &mut usize, x: u32) {
    s.set(*i, x);
    *i += 1;
    s.delete(*i);
    s.set(*i, 0x219c2);
}

/// Elizabeth 0x100014a0.
fn elizabeth(s: &mut Stage, ip: &mut usize) {
    let i = *ip;
    let w = s.word(i);
    let wi = w as i64;
    let text = s.text(wi).map(<[u8]>::to_vec);
    let t = text.as_deref();
    let grade = s.rec(wi).map_or(0, |r| r.f[0x12]);
    // the next record's text ("" 0x1000a386 at the chunk's record count)
    let next = if w as usize == s.count() { Some(Vec::new()) } else { s.text(wi + 1).map(<[u8]>::to_vec) };
    let next = next.as_deref();
    // ctx+0x18 in 7..=10: 'I', else 'D' — the module's stale token count,
    // below 7 at FON time in ordinary sentences (as kate.rs/dave.rs): 'D'
    let question = false;

    if i == 0 || s.word(i - 1) != w || [0x9e2, 0x10ce, 0x100ece].contains(&s.code(i - 1)) {
        replace(s, ip, DICT_END);
    }
    // a dictionary word's final r before a non-vowel goes
    let i = *ip;
    if s.word(i) != s.word(i + 1)
        && s.code(i) == 0x18c2
        && s.code(i + 1) & 3 != 1
        && lookup(s, DICT_END, t, grade).is_some()
        && i < s.list.len()
    {
        s.delete(i);
        *ip = i.wrapping_sub(1);
        if i == 0 {
            // *i = -1: the code goes on reading before the list
            return;
        }
    }
    let i = *ip;
    let word_start = i == 0 || s.word(i) != s.word(i - 1);
    match s.code(i) {
        // `Aa: of answer/ask word-initially: `Ae
        0x10245 => {
            if word_start && (starts(s, t, ANSWER) || starts(s, t, ASK)) {
                s.set(i, 0x10681);
            }
        }
        0x4785 => {
            if i == 0 {
                return;
            }
            // word-final HIE in -ia: i HEh
            if s.word(i) != s.word(i + 1) && ends(s, t, IA) {
                s.set(i, 0x7c1);
                let w = s.word(i);
                // the added element's kind is stack garbage in the original
                s.insert(i + 1, w, 0x501, KIND_PHONE);
                return;
            }
            // HIE HEh closing a word in -io: i HOU
            if s.code(i + 1) == 0x501 && s.word(i + 1) != s.word(i + 2) && ends(s, t, IO) {
                s.set(i, 0x7c1);
                s.set(i + 1, 0xad05);
                *ip += 1;
            }
        }
        // p k t g d b before -on/-ons: unreleased, then Hn
        0x3ff2 if on_ons(s, i, t) => syllabic(s, ip, 0x5ff2),
        0x2ff2 if on_ons(s, i, t) => syllabic(s, ip, 0x4ff2),
        0x39f2 if on_ons(s, i, t) => syllabic(s, ip, 0x59f2),
        0xfe2 if on_ons(s, i, t) => syllabic(s, ip, 0x4fe2),
        0x19e2 if on_ons(s, i, t) => syllabic(s, ip, 0x59e2),
        0x1fe2 if on_ons(s, i, t) => syllabic(s, ip, 0x5fe2),
        // word-initial Ae of accept...: E
        0x681 => {
            if word_start && starts(s, t, ACCEPT) {
                s.set(i, 0x6c1);
            }
        }
        // used to: s Hut
        0x1942 => {
            if is(s, t, &[USED]) && is(s, next, &[TO]) {
                s.set(i, 0x1952);
                s.set(i + 1, 0x59f2);
                *ip += 1;
            }
        }
        0x501 => he(s, ip, w, t, question),
        _ => {}
    }
}

/// HEh (0x501 of 0x100014a0).
fn he(s: &mut Stage, ip: &mut usize, w: u32, t: Option<&[u8]>, question: bool) {
    let i = *ip;
    let mut done = false;
    if i != 0 {
        // fricative HEh n closing -on/-ons: Hn (not in the -son names)
        let p = s.code(i - 1);
        if s.code(i + 1) == 0x19c2 && s.word(i - 1) == s.word(i + 1) && p & 0x1e3 == 0x142 {
            let ok = (ends(s, t, ON) && pause(s.code(i + 2))) || (ends(s, t, ONS) && pause(s.code(i + 3)));
            if ok && !is(s, t, &SON_KEEP) {
                s.set(i, 0x219c2);
                s.delete(i + 1);
                done = true;
            }
        }
        // con-/recon-/semicon- after k before n: O (0x261)
        if !done && p == 0x2ff2 && s.code(i + 1) == 0x19c2 {
            let con = (i == 1 || s.word(i - 1) != s.word(i - 2))
                && starts(s, t, CON)
                && !CON_KEEP.iter().any(|&a| starts(s, t, a));
            let recon = i > 2 && (i == 3 || s.word(i - 3) != s.word(i - 4)) && starts(s, t, RECON);
            let semicon = i > 4 && (i == 5 || s.word(i - 5) != s.word(i - 6)) && starts(s, t, SEMICON);
            if con || recon || semicon {
                s.set(i, 0x261);
                done = true;
            }
        }
    }
    if !done && is(s, t, &[ARE]) && w < 2 && question && s.count() < 5 {
        s.set(i, 0x245);
    }
    // LAB_10001a5e: $ HEh n l (or HEh n HEh l) not in traditional: $ Hn l
    let i = *ip;
    if i != 0
        && s.code(i - 1) == 0x1752
        && s.code(i + 1) == 0x19c2
        && (s.code(i + 2) == 0x218a2 || (s.code(i + 2) == 0x501 && s.code(i + 3) == 0x18a2))
        && !is(s, t, &[TRADITIONAL])
    {
        s.set(i, 0x219c2);
        s.delete(i + 1);
    }
}

/// Elizabeth 0x10003360: k t p (unaspirated) as ids 0x3d 0x3b 0x39 (g d
/// b); Hj, 0x2f, Hg, Hw starting a word (not the first) are dropped; the
/// rest as 0x10002420 (the default).
fn elizabeth_meta(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    let id = match s.code(*i) {
        0xff2 => 0x3d,
        0x19f2 => 0x3b,
        0x1ff2 => 0x39,
        0x10ce | 0x2f | 0x9e2 | 0x100ece if *i != 0 && s.word(*i) != s.word(*i - 1) => {
            *i += 1;
            return;
        }
        _ => return meta_default(s, i, out),
    };
    out.push(id);
    *i += 1;
}
