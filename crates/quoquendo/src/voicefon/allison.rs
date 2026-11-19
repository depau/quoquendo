//! LTTS7Allison.dll (US English): `FonVidAllofonoVoce` 0x10001450 (with
//! its word replacements 0x10001210) and `FonVidMetaFonema` 0x100037a0.
//! The words it compares with are read from the library at the addresses
//! its code pushes.

use super::{contains, eq, meta_default, Port, Rules, Stage, KIND_PHONE};

pub(super) const ALLISON: Port = Port {
    voice: "Allison",
    table: 0x1000a8d8,
    functions: &[
        ("GetVersion", 0x100010f0),
        ("ELQGetPhonemes", 0x10001000),
        ("FonVidAllofonoVoce", 0x10001450),
        ("FonVidAllofonoAnalisi", 0x10002400),
        ("FonVidMetaFonema", 0x100037a0),
        ("FonVidAnalisi2Sintesi", 0x10003870),
        ("FonVidIngloba", 0x10003900),
    ],
    rules: Rules {
        allofono: Some(allofono),
        meta,
    },
};

// The library's phone codes this code names.
const N: u32 = 0x1010024;
const SH: u32 = 0x20104;
const TSH: u32 = 0x2021004;
const DZH: u32 = 0x3021004;
const U: u32 = 0x103011;
const HG: u32 = 0x1800014;
const HEH: u32 = 0x10851;
const H: u32 = 0x800104;
const HK: u32 = 0x80001c;
const HTH: u32 = 0x3010094;
const S: u32 = 0x10104;
const Z: u32 = 0x1010104;
const HR: u32 = 0x1010404;
const HRR: u32 = 0x9010404;
const L: u32 = 0x1010804;
const HL: u32 = 0x9010804;
const HN: u32 = 0x9010024;
const D: u32 = 0x3010014;
const HUD: u32 = 0x23010014;
const HUT: u32 = 0x22010014;
const TH: u32 = 0x12010014;
const TS: u32 = 0x2011004;

/// The words (string addresses in the library).
mod w {
    pub const TZ: u32 = 0x1000a998;
    pub const YOU: &[u32] = &[0x1000a9e4, 0x1000a9dc, 0x1000a9d4, 0x1000a9cc, 0x1000a9c4, 0x1000a9bc, 0x1000a9b8, 0x1000a9b4, 0x1000a9a8, 0x1000a99c];
    pub const NSC: u32 = 0x1000a9e8;
    pub const NT: u32 = 0x1000a9ec;
    pub const ONTO: u32 = 0x1000a9f0;
    pub const INTO: u32 = 0x1000a9f8;
    pub const TO: u32 = 0x1000aa00;
    pub const THERE: u32 = 0x1000aa04;
    /// he him his her hers he'd he'll he's, as `and` + next word checks them
    pub const I_HE: &[u32] = &[0x1000a334, 0x1000aa38, 0x1000aa30, 0x1000aa34, 0x1000aa2c, 0x1000aa0c, 0x1000aa24, 0x1000aa1c, 0x1000aa14];
    /// have has had he him his her he'd he'll he's hers, after `and`
    pub const AND_DROP_H: &[u32] = &[0x1000aa48, 0x1000aa44, 0x1000aa40, 0x1000aa38, 0x1000aa34, 0x1000aa30, 0x1000aa2c, 0x1000aa24, 0x1000aa1c, 0x1000aa14, 0x1000aa0c];
    pub const AND: u32 = 0x1000aa3c;
    pub const HAVE: &[u32] = &[0x1000aa48, 0x1000aa44, 0x1000aa40];
    pub const MUST: u32 = 0x1000aa50;
    pub const AT: u32 = 0x1000aa58;
    pub const HAVE_TO: &[u32] = &[0x1000aa6c, 0x1000aa64, 0x1000aa5c];
    pub const CAN: u32 = 0x1000aa74;
    pub const DOT: u32 = 0x1000aaa0;
    pub const IT: u32 = 0x1000aaa4;
    pub const BELIEVE: u32 = 0x1000aaa8;
    pub const TIONAL: u32 = 0x1000aab0;
}

/// The word replacements 0x10001210 reads: entries of 0x444 bytes from
/// 0x1000d000 (`word[0x40]`, `u32 codes[]` 0-terminated, `i32 grade` at
/// +0x440, -1 any) up to the word `-1` (0x1000a994).
const WORDS: u32 = 0x1000d000;
const WORDS_END: u32 = 0x1000a994;
const ENTRY: u32 = 0x444;

fn is(s: &Stage, t: Option<&[u8]>, at: u32) -> bool {
    eq(t, s.pe.cstr(at))
}

fn any(s: &Stage, t: Option<&[u8]>, at: &[u32]) -> bool {
    at.iter().any(|&a| is(s, t, a))
}

/// 0x10001210, at a word's first phone: a word (with that stress grade,
/// or any) in the replacement list has all its phones replaced by the
/// entry's codes, 0x48 standing for the word's last phone; `*i` is left on
/// the first new one.
fn replace_word(s: &mut Stage, i: usize) {
    let w = s.word(i);
    let text = s.text(w as i64).map(<[u8]>::to_vec);
    let grade = s.rec(w as i64).map_or(0, |r| r.f[0x12] as i8 as i32);
    let end = s.pe.cstr(WORDS_END).to_vec();
    let mut k = 0u32;
    loop {
        let entry = WORDS + k * ENTRY;
        let word = s.pe.cstr(entry);
        // the `-1` sentinel: three bytes compared, its NUL too
        if word == end.as_slice() {
            break;
        }
        let g = s.pe.u32(entry + 0x440) as i32;
        if text.as_deref() == Some(word) && (g == -1 || g == grade) {
            break;
        }
        k += 1;
    }
    let entry = WORDS + k * ENTRY;
    let g = s.pe.u32(entry + 0x440) as i32;
    if text.as_deref() != Some(s.pe.cstr(entry)) || !(g == -1 || g == grade) {
        return;
    }
    let mut last = s.code(i);
    // the sentinel closing the list stays
    while s.word(i) == w && i + 1 < s.list.len() {
        last = s.code(i);
        s.delete(i);
    }
    let mut at = i;
    let mut c = entry + 0x40;
    loop {
        let code = s.pe.u32(c);
        if code == 0 {
            break;
        }
        s.insert(at, w, if code == 0x48 { last } else { code }, KIND_PHONE);
        at += 1;
        c += 4;
    }
    if at > 0 {
        s.list[at - 1].kind = super::KIND_WORD_END;
    }
}

/// 0x10001450.
fn allofono(s: &mut Stage, pi: &mut usize) {
    let i0 = *pi;
    let w = s.word(i0);
    let text = s.text(w as i64).map(<[u8]>::to_vec);
    let text = text.as_deref();
    let prev_text = if w == 0 { Some(&b""[..]) } else { s.text(w as i64 - 1) }.map(<[u8]>::to_vec);
    let prev_text = prev_text.as_deref();
    let next_text = if w as usize == s.count() { Some(&b""[..]) } else { s.text(w as i64 + 1) }.map(<[u8]>::to_vec);
    let next_text = next_text.as_deref();
    let grade = s.rec(w as i64).map_or(0, |r| r.f[0x12]);
    if i0 == 0 || w != s.word(i0 - 1) {
        replace_word(s, i0);
    }
    let i = *pi;
    let cur = s.code(i);
    let word = s.word(i);
    let next_word = s.word(i + 1);
    match cur {
        N => {
            // n + ʃ in a word with `nt`/`nsc`: tʃ
            if s.code(i + 1) == SH && (contains(text, s.pe.cstr(w::NT)) || contains(text, s.pe.cstr(w::NSC))) {
                s.set(i + 1, TSH);
                *pi += 1;
            }
            return;
        }
        U => {
            // `to`/`into`/`onto` before a glottal onset: the vowel weak
            if word != next_word && s.code(i + 1) == HG && any(s, text, &[w::TO, w::INTO, w::ONTO]) {
                s.set(i, HEH);
            }
            return;
        }
        H => {
            h(s, i, text, prev_text, grade);
            return;
        }
        0x40411 => {
            // `can` as the chunk's first word: weak
            if is(s, text, w::CAN) && word == 0 {
                s.set(i, HEH);
            }
            return;
        }
        0x4411 => {
            if contains(text, s.pe.cstr(w::BELIEVE)) && s.code(i + 1) == L {
                s.set(i, HEH);
                return;
            }
            if i == 0 || !is(s, text, w::IT) || !is(s, prev_text, w::DOT) {
                return;
            }
            // `dot it`
            s.set(i, HK);
            s.set(i + 1, 0x2280893);
            s.set(i - 1, HTH);
            *pi += 1;
            let k = *pi;
            s.insert(k + 1, s.word(k), TH, 0);
            s.insert(k + 2, s.word(k + 1), 0x24a1, 0);
            return;
        }
        HEH => {
            schwa(s, i, text, w);
            return;
        }
        TH => {
            if any(s, text, &[w::INTO, w::ONTO]) {
                s.set(i, HTH);
                return;
            }
            // `to` after n/ŋ of another word: flapped
            if i != 0 && matches!(s.code(i - 1), N | 0x1100024) && s.word(i - 1) != word && is(s, text, w::TO) {
                s.set(i, HTH);
            }
            return;
        }
        HUT => {
            if i == 0 {
                return;
            }
            let prev = s.code(i - 1);
            let next = s.code(i + 1);
            if prev == Z && word != next_word && next == 0x3008104 {
                s.delete(i);
                return;
            }
            if !(prev == 0x22002014 && word != next_word && next == 0x12002014) && !(prev == S && word != next_word && next == S) {
                return;
            }
            s.delete(i);
            *pi = i.wrapping_sub(1);
            return;
        }
        HL => {
            let after_syllabic = i != 0 && matches!(s.code(i - 1), HN | HRR);
            if !after_syllabic && !(i >= 2 && s.code(i - 2) == HEH && matches!(s.code(i - 1), N | HR)) {
                return;
            }
            s.set(i, HEH);
            s.insert(i + 1, word, L, 0);
            return;
        }
        TS => {
            if word != next_word || contains(text, s.pe.cstr(w::TZ)) || i == 0 || s.word(i - 1) != word {
                return;
            }
            s.set(i, HUT);
            s.insert(i + 1, word, S, 0);
            return;
        }
        HTH => {
            if i != 0 && (is(s, text, w::IT) || is(s, text, w::AT)) && s.code(i + 1) == H {
                s.set(i, HUT);
            }
            return;
        }
        D | 0x36 => {}
        _ => return,
    }
    // d: `and` before I/he…: unreleased
    if is(s, text, w::AND) && next_text.is_some() && any(s, next_text, w::I_HE) {
        s.set(i, HUD);
        return;
    }
    // after dʒ, before a consonant other than th: unreleased t
    if i != 0 && word != next_word && s.code(i - 1) == DZH {
        let next = s.code(i + 1);
        if next & 4 != 0 && next != TH {
            s.set(i, HUT);
            return;
        }
    }
    // d before you/your…: dʒ
    if word != next_word && any(s, next_text, w::YOU) {
        s.set(i, DZH);
    }
}

/// 0x10001450, case h (0x800104): the h of an unstressed word dropped
/// after a stop, fricative or HTh (`it`/`at` + h: their t flapped), and
/// after `must`/`and`/`there` in have/has/had, he/him… .
fn h(s: &mut Stage, i: usize, text: Option<&[u8]>, prev_text: Option<&[u8]>, grade: u8) {
    if i == 0 {
        return;
    }
    let (word, prev, prev_word) = (s.word(i), s.code(i - 1), s.word(i - 1));
    if prev_word != word && !(text.is_some() && any(s, text, w::HAVE_TO)) {
        let weak = prev == HTH
            || prev == Z
            || (prev & 0x1000004 != 0x1000004 && (prev & 0x1004 == 0x1004 || prev & 0x14 == 0x14 || prev & 0x104 == 0x104));
        if grade == 3 && weak {
            s.delete(i);
            if !(is(s, prev_text, w::IT) || is(s, prev_text, w::AT)) {
                return;
            }
            if s.code(i - 1) == HUT {
                s.set(i - 1, HTH);
            }
            return;
        }
    }
    if prev_text.is_none() {
        return;
    }
    if is(s, prev_text, w::MUST) && prev_word != word && any(s, text, w::HAVE) && s.word(i) as usize != s.sentence_len().wrapping_sub(2) {
        s.delete(i);
        return;
    }
    if (is(s, prev_text, w::AND) && any(s, text, w::AND_DROP_H)) || (is(s, prev_text, w::THERE) && any(s, text, w::HAVE)) {
        s.delete(i);
    }
}

/// 0x10001450, case HEh (0x10851): `-tional` at the sentence's end loses
/// its schwa (ʃ n̩), and HEh + r + HEh before l/r of the word is HR.
fn schwa(s: &mut Stage, i: usize, text: Option<&[u8]>, w: u32) {
    let word = s.word(i);
    if i != 0
        && s.code(i - 1) == SH
        && s.code(i + 1) == N
        && s.code(i + 2) == HEH
        && s.code(i + 3) == 0x50
        && s.word(i - 1) == s.word(i + 3)
        && contains(text, s.pe.cstr(w::TIONAL))
        && (w as usize == s.sentence_len().wrapping_sub(2) || w as usize == s.count().wrapping_sub(2))
    {
        s.delete(i);
        s.set(i, HN);
        return;
    }
    if s.code(i + 1) != HR || s.code(i + 2) != HEH || word != s.word(i + 3) || !matches!(s.code(i + 3), L | HR) {
        return;
    }
    s.delete(i);
    s.set(i, HRR);
}

/// 0x100037a0: unaspirated p t k said b d g, the affricate 0x301100c Ts;
/// a word-initial glide or glottal onset (Hw Hj Hg Hk, not the chunk's
/// first phone) is not said.
fn meta(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    let c = s.code(*i);
    let id = match c {
        0x2002014 => 0x33,
        0x2010014 => 0x36,
        0x2100014 => 0x39,
        0x301100c => 0x3c,
        0x1102404 | 0x80001c | 0x1080404 | HG if *i != 0 && s.word(*i) != s.word(*i - 1) => {
            *i += 1;
            return;
        }
        _ => return meta_default(s, i, out),
    };
    out.push(id);
    *i += 1;
}
