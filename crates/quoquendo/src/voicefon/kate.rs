//! LTTS7Kate.dll (British English): `FonVidAllofonoVoce` 0x100014a0 (with
//! its word dictionary 0x10001210/0x10001350) and `FonVidMetaFonema`
//! 0x100039e0 (default mapping 0x10002530).

use super::{meta_default, Port, Rules, Stage, KIND_PHONE, KIND_WORD_END};

pub(super) const KATE: Port = Port {
    voice: "Kate",
    table: 0x1000a978,
    functions: &[
        ("GetVersion", 0x10001150),
        ("ELQGetPhonemes", 0x10001060),
        ("FonVidAllofonoVoce", 0x100014a0),
        ("FonVidAllofonoAnalisi", 0x100025a0),
        ("FonVidMetaFonema", 0x100039e0),
        ("FonVidAnalisi2Sintesi", 0x10003aa0),
        ("FonVidIngloba", 0x10003b30),
    ],
    rules: Rules {
        allofono: Some(allofono),
        meta,
    },
};

/// The word dictionary: 0x54-byte entries from 0x1000d000, `{char word[16];
/// u32 codes[16] (0-terminated; 0x48 = the word's last phone); i32 grade}`
/// (grade -1: any), ended by the entry whose word is the string at
/// 0x1000aa04 (`-1`).
const DICT: u32 = 0x1000d000;
const DICT_END: u32 = 0x1000aa04;
const ENTRY: u32 = 0x54;

// The literals 0x100014a0 compares with.
const IO: u32 = 0x1000aa08;
const IA: u32 = 0x1000aa0c;
const RE: u32 = 0x1000aa10;
const HER: u32 = 0x1000aa14;
const DIRECT: u32 = 0x1000aa18;
const ARE: u32 = 0x1000aa20;
/// Surnames in `-son` whose schwa stays (0x1000aa88 emerson … 0x1000aa24
/// nicolson, in the code's order).
const SONS: &[u32] = &[0x1000aa88, 0x1000aa7c, 0x1000aa70, 0x1000aa64, 0x1000aa5c, 0x1000aa50, 0x1000aa48, 0x1000aa3c, 0x1000aa30, 0x1000aa24];
const ONS: u32 = 0x1000aa90;
const ON: u32 = 0x1000aa94;
const TO: u32 = 0x1000aa98;
const USED: u32 = 0x1000aa9c;

const PAUSE: u32 = 0x18; // #
const PAUSE2: u32 = 0x28; // @
const SCHWA: u32 = 0x10851; // HEh
const N: u32 = 0x1010024;
const SYLLABIC_N: u32 = 0x9010024; // Hn
const R: u32 = 0x1010404; // Hr
const GLOTTAL: u32 = 0x1800014; // Hg

/// 0x10001210: the dictionary entry for `text` (EAX) and the record's
/// stress grade: the first entry with the word whose grade is -1 or equal
/// to `grade`, else -1 (`None`).
fn lookup(s: &Stage, text: Option<&[u8]>, grade: u8) -> Option<u32> {
    let text = text?;
    let end = s.pe.cstr(DICT_END);
    let word = |k: u32| s.pe.cstr(DICT + k * ENTRY);
    let mut k = 0;
    // the first entry with the word (stopping at the end entry)
    while word(k) != end && word(k) != text {
        k += 1;
    }
    // then the first of the following with the word and a matching grade
    while word(k) == text {
        let g = s.pe.u32(DICT + k * ENTRY + 0x50);
        if g == u32::MAX || g == grade as u32 {
            break;
        }
        k += 1;
    }
    (word(k) == text).then_some(k)
}

/// 0x10001350: a dictionary word's phones from `i` (its word's phones from
/// `i` on deleted) replaced by the entry's codes (0x48: the last deleted
/// phone's code), the last one ending the word (kind 0x77).
fn dictionary(s: &mut Stage, i: usize) {
    let w = s.word(i);
    let Some(k) = lookup(s, s.text(w as i64), s.rec(w as i64).map_or(0, |r| r.f[0x12])) else {
        return;
    };
    let mut last = s.code(i);
    while s.word(i) == w {
        last = s.code(i);
        s.delete(i);
    }
    let mut j = i;
    let mut at = DICT + k * ENTRY + 0x10;
    loop {
        let c = s.pe.u32(at);
        if c == 0 {
            break;
        }
        // ELQListFindElement on slot j: fails past the list
        if j < s.list.len() {
            s.insert(j, w, if c == 0x48 { last } else { c }, KIND_PHONE);
            j += 1;
        }
        at += 4;
    }
    if j > 0 {
        s.list[j - 1].kind = KIND_WORD_END;
    }
}

/// Suffix `strncmp(text + len - n, lit, n)` with `len >= n`.
fn ends(text: &[u8], lit: &[u8]) -> bool {
    text.ends_with(lit)
}

fn pause(c: u32) -> bool {
    c == PAUSE2 || c == PAUSE
}

/// The `-on`/`-ons` test of 0x100014a0: a word ending in `on` with a pause
/// after the phone at `i + on`, or in `ons` with one after `i + on + 1`.
fn on_end(s: &Stage, i: usize, text: &[u8], on: usize) -> bool {
    (ends(text, s.pe.cstr(ON)) && pause(s.code(i + on))) || (ends(text, s.pe.cstr(ONS)) && pause(s.code(i + on + 1)))
}

/// A stop before `HEh n` in the same word of a `-on(s)` word before a
/// pause: the stop becomes `to` (unreleased), the schwa goes and the n is
/// syllabic (0x100014a0, the six stop cases; LAB_100024f6).
fn stop_on(s: &mut Stage, i: &mut usize, text: &[u8], to: u32) {
    if s.code(*i + 1) != SCHWA || s.code(*i + 2) != N || s.word(*i) != s.word(*i + 2) || !on_end(s, *i, text, 3) {
        return;
    }
    s.set(*i, to);
    *i += 1;
    s.delete(*i);
    s.set(*i, SYLLABIC_N);
}

/// 0x100014a0.
fn allofono(s: &mut Stage, i: &mut usize) {
    let w = s.word(*i);
    let owned = s.text(w as i64).map(<[u8]>::to_vec);
    let text = owned.as_deref();
    let t = text.unwrap_or(b"");
    let prev_text = if w == 0 { Some(&b""[..]) } else { s.text(w as i64 - 1) }.map(<[u8]>::to_vec);
    let next_text = if w as usize == s.count() { Some(&b""[..]) } else { s.text(w as i64 + 1) }.map(<[u8]>::to_vec);
    let grade = s.rec(w as i64).map_or(0, |r| r.f[0x12]);
    // ctx +0x18 in 7..=10: 'I', else 'D'. It is the module's stale scratch
    // token count (LesElaboraParola, the number readers), below 7 in
    // ordinary sentences at FON time: taken as 'D'.
    let interrogative = false;

    if *i == 0 || w != s.word(*i - 1) || s.code(*i - 1) == GLOTTAL {
        dictionary(s, *i);
    }
    // a dictionary word's final r before a consonant goes
    if s.word(*i) != s.word(*i + 1) && s.code(*i) == R && s.code(*i + 1) & 1 == 0 && lookup(s, text, grade).is_some() && *i < s.list.len() {
        s.delete(*i);
        match i.checked_sub(1) {
            Some(k) => *i = k,
            // *i = -1: the code reads before the list
            None => {
                *i = usize::MAX;
                return;
            }
        }
    }

    let i0 = *i;
    let cur = s.code(i0);
    match cur {
        0x12010014 => stop_on(s, i, t, 0x22010014),
        0x12100014 => stop_on(s, i, t, 0x22100014),
        0x3010014 => stop_on(s, i, t, 0x23010014),
        0x3100014 => stop_on(s, i, t, 0x23100014),
        0x12002014 => stop_on(s, i, t, 0x22002014),
        0x3002014 => stop_on(s, i, t, 0x23002014),
        // HIE
        0x8404493 => {
            if i0 == 0 {
                return;
            }
            if s.word(i0) != s.word(i0 + 1) && ends(t, s.pe.cstr(IA)) {
                s.set(i0, 0x2411);
                if i0 < s.list.len() {
                    // the element's kind byte is an uninitialised local
                    s.insert(i0 + 1, s.word(i0), SCHWA, KIND_PHONE);
                }
                return;
            }
            if s.code(i0 + 1) == SCHWA && s.word(i0 + 1) != s.word(i0 + 2) && ends(t, s.pe.cstr(IO)) {
                s.set(i0, 0x2411);
                s.set(i0 + 1, 0x82810893);
                *i += 1;
            }
        }
        // Hj, Hw at a word's start after a matching vowel, before a
        // stressed vowel: a glottal stop
        0x1080404 | 0x1102404 => {
            if i0 == 0 || s.word(i0 - 1) == s.word(i0) {
                return;
            }
            let after: &[u32] = if cur == 0x1080404 {
                &[0x2411, 0x24a1, 0x4411, 0x4421, 0x2280893, 0x22808a3, 0x2220493, 0x22204a3, 0x2321093, 0x23210a3]
            } else {
                &[0x103011, 0x1030a1, 0x105011, 0x105021, 0x82880893, 0x828808a3, 0x82810893, 0x828108a3]
            };
            if after.contains(&s.code(i0 - 1)) && s.code(i0 + 1) & 0x21 == 0x21 {
                s.set(i0, GLOTTAL);
            }
        }
        GLOTTAL => {
            if i0 == 0 || s.word(i0 - 1) == s.word(i0) {
                return;
            }
            let next = s.code(i0 + 1);
            if next & 0x11 != 0x11 || next == SCHWA {
                return;
            }
            let Some(p) = prev_text.as_deref() else { return };
            if p == s.pe.cstr(HER) || p == s.pe.cstr(RE) {
                s.set(i0, R);
                s.list[i0].word = s.word(i0 - 1);
            }
        }
        R => {
            if i0 == 0 || s.word(i0) == s.word(i0 + 1) || s.code(i0 + 1) & 0x21 != 0x21 {
                return;
            }
            s.set(i0, GLOTTAL);
            s.list[i0].word = s.word(i0 + 1);
        }
        0x4411 => {
            if i0 == 0 || (i0 != 1 && s.word(i0 - 1) == s.word(i0 - 2)) {
                return;
            }
            if s.code(i0 - 1) == 0x3010014 && t.starts_with(s.pe.cstr(DIRECT)) {
                s.set(i0, 0x2280893);
            }
        }
        0x1010104 => {
            if super::eq(text, s.pe.cstr(USED)) && super::eq(next_text.as_deref(), s.pe.cstr(TO)) {
                s.set(i0, 0x10104);
                s.set(i0 + 1, 0x22010014);
                *i += 1;
            }
        }
        SCHWA => {
            let syllabic = i0 != 0
                && s.code(i0 + 1) == N
                && s.word(i0 - 1) == s.word(i0 + 1)
                && s.code(i0 - 1) & 0x104 == 0x104
                && on_end(s, i0, t, 2)
                && !(text.is_some() && SONS.iter().any(|&a| t == s.pe.cstr(a)));
            if syllabic && (s.code(i0 - 1) != 0x20104 || (s.code(i0 + 2) != PAUSE && s.code(i0 + 3) != PAUSE)) {
                s.delete(i0);
                s.set(i0, SYLLABIC_N);
                return;
            }
            if super::eq(text, s.pe.cstr(ARE)) && w <= 1 && interrogative && s.count() <= 4 {
                s.set(i0, 0x81091);
            }
        }
        _ => {}
    }
}

/// 0x100039e0: Hg Hj Hw (and code 0x2f) starting a word (not the list's
/// first) are dropped; p t k (unaspirated) are said as b d g (ids 0x39
/// 0x3b 0x3d); the rest by 0x10002530 (the default).
fn meta(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    let c = s.code(*i);
    let id = match c {
        0x2002014 => 0x39,
        0x2010014 => 0x3b,
        0x2100014 => 0x3d,
        GLOTTAL | 0x2f | 0x1080404 | 0x1102404 if *i != 0 && s.word(*i) != s.word(*i - 1) => {
            *i += 1;
            return;
        }
        _ => return meta_default(s, i, out),
    };
    out.push(id);
    *i += 1;
}
