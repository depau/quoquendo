//! LTTS7Steven.dll (US English): `FonVidAllofonoVoce` 0x100014e0 (with
//! its per-word replacement dictionary 0x10001350 / lookup 0x10001210)
//! and `FonVidMetaFonema` 0x10003ac0 (default mapping 0x100024a0).
//!
//! The literals are read from the library by address; the comment after
//! each address is what it holds.

use super::{meta_default, Port, Rules, Stage, KIND_PAUSE, KIND_PHONE, KIND_WORD_END};

pub(super) const STEVEN: Port = Port {
    voice: "Steven",
    table: 0x1000a910,
    functions: &[
        ("GetVersion", 0x10001150),
        ("ELQGetPhonemes", 0x10001060),
        ("FonVidAllofonoVoce", 0x100014e0),
        ("FonVidAllofonoAnalisi", 0x10002510),
        ("FonVidMetaFonema", 0x10003ac0),
        ("FonVidAnalisi2Sintesi", 0x100039f0),
        ("FonVidIngloba", 0x10003a80),
    ],
    rules: Rules {
        allofono: Some(steven),
        meta: steven_meta,
    },
};

/// The replacement dictionary (0x10001350's argument): 0x54-byte entries,
/// the word at +0, its codes from +0x10 up to a 0 (0x48: the replaced
/// word's last code), the stress grade it applies to at +0x50 (-1: any).
const DICT: u32 = 0x1000d000;
/// Its closing entry's word (`-1`).
const DICT_END: u32 = 0x1000a99c;

const IT: u32 = 0x1000aa4c; // it
const AT: u32 = 0x1000aa48; // at
const BELIEVE: u32 = 0x1000aa6c; // believe
const TO: u32 = 0x1000aa84; // to
const INTO: u32 = 0x1000aa7c; // into
const ONTO: u32 = 0x1000aa74; // onto
const TIONAL: u32 = 0x1000aa88; // tional
const CAN: u32 = 0x1000aa90; // can
const TZ: u32 = 0x1000aa68; // tz
const HAVE_TO: [u32; 3] = [0x1000aa60, 0x1000aa58, 0x1000aa50]; // have to, has to, had to
const MUST: u32 = 0x1000aa40; // must
const AND: u32 = 0x1000aa2c; // and
const THERE: u32 = 0x1000a9f4; // there
const HAVE: [u32; 3] = [0x1000aa38, 0x1000aa34, 0x1000aa30]; // have, has, had
/// After `and`, h is dropped (0x10001c41): have has had he him his her
/// he'd he'll he's hers.
const AND_H: [u32; 11] = [
    0x1000aa38, 0x1000aa34, 0x1000aa30, 0x1000aa28, 0x1000aa24, 0x1000aa20, 0x1000aa1c, 0x1000aa14, 0x1000aa0c,
    0x1000aa04, 0x1000a9fc,
];
/// `and` before these says Hud (0x10001f5a): i he his him her hers he'd
/// he'll he's.
const AND_D: [u32; 9] = [
    0x1000a360, 0x1000aa28, 0x1000aa20, 0x1000aa24, 0x1000aa1c, 0x1000a9fc, 0x1000aa14, 0x1000aa0c, 0x1000aa04,
];
const NT: u32 = 0x1000a9a4; // nt
const NSC: u32 = 0x1000a9a0; // nsc
/// A word-final d before these is Dg (0x100020af): you your you've you'd
/// you'll yours yer yeh yourself yourselves.
const YOU: [u32; 10] = [
    0x1000a9f0, 0x1000a9e8, 0x1000a9e0, 0x1000a9d8, 0x1000a9d0, 0x1000a9c8, 0x1000a9c4, 0x1000a9c0, 0x1000a9b4,
    0x1000a9a8,
];

fn is(s: &Stage, t: Option<&[u8]>, at: &[u32]) -> bool {
    at.iter().any(|&a| super::eq(t, s.pe.cstr(a)))
}

fn kind(s: &Stage, i: usize) -> u8 {
    s.list.get(i).map_or(0, |p| p.kind)
}

/// Kind `w` or `p` (the word's last element).
fn last(k: u8) -> bool {
    k == KIND_WORD_END || k == KIND_PAUSE
}

/// 0x10001210: the dictionary entry for the word `text` at stress grade
/// `grade`: the first entry with the word, skipping same-word entries for
/// other grades (an entry for any grade ends the skip); none when the word
/// is absent (the scan stops at the `-1` entry).
fn lookup(s: &Stage, text: Option<&[u8]>, grade: u8) -> Option<u32> {
    let text = text?;
    let name = |k: u32| s.pe.cstr(DICT + k * 0x54);
    let end = s.pe.cstr(DICT_END);
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

/// 0x10001350: a word in the dictionary has its phones (from `*i` up to
/// its `w`/`p` element, that one too unless a pause 0x13/0x17) replaced by
/// the entry's codes; the last one takes the last deleted element's kind.
fn replace(s: &mut Stage, i: &mut usize, text: Option<&[u8]>, grade: u8) {
    let Some(k) = lookup(s, text, grade) else { return };
    let start = *i;
    let w = s.word(start);
    let (mut code, mut k_last) = (0, 0);
    while *i < s.list.len() && !last(kind(s, *i)) {
        (code, k_last) = (s.code(*i), kind(s, *i));
        s.delete(*i);
    }
    let c = s.code(*i);
    if c != 0x17 && c != 0x13 {
        (code, k_last) = (c, kind(s, *i));
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
        p.kind = k_last;
    }
    *i = start;
}

/// 0x10002486: element `i` deleted, the previous one taking its kind, and
/// `*i` stepped back.
fn drop_back(s: &mut Stage, i: &mut usize) {
    let at = *i;
    let k = kind(s, at);
    if let Some(p) = at.checked_sub(1).and_then(|j| s.list.get_mut(j)) {
        p.kind = k;
    }
    if at < s.list.len() {
        s.delete(at);
        *i = at.wrapping_sub(1);
    }
}

/// `"and"` as the chunk's first word, not in the first chunk (`ctx+0x38`,
/// the chunk index, nonzero: the first chunk starts the sentence).
fn chunk_and(s: &Stage, w: u32, text: Option<&[u8]>) -> bool {
    is(s, text, &[AND]) && w == 0 && s.first() != 0
}

/// Steven 0x100014e0.
fn steven(s: &mut Stage, i: &mut usize) {
    let i0 = *i;
    let w = s.word(i0);
    let wi = w as i64;
    let text = s.text(wi).map(<[u8]>::to_vec);
    let text = text.as_deref();
    // the previous record's text ("" 0x1000a396 for the chunk's first),
    // the next one's ("" at the chunk's record count)
    let prev = if w == 0 { Some(Vec::new()) } else { s.text(wi - 1).map(<[u8]>::to_vec) };
    let prev = prev.as_deref();
    let next = if w as usize == s.count() { Some(Vec::new()) } else { s.text(wi + 1).map(<[u8]>::to_vec) };
    let next = next.as_deref();
    let grade = s.rec(wi).map_or(0, |r| r.f[0x12]);

    let n = s.code(i0 + 1);
    if n == 0x17 || n == 0x13 {
        return;
    }
    if i0 == 0 || last(kind(s, i0 - 1)) || [0x9e2, 0x9f2, 0x10ce, 0x100ece].contains(&s.code(i0 - 1)) {
        replace(s, i, text, grade);
    }
    let i0 = *i;
    let cur = s.code(i0);
    let k = kind(s, i0);
    let p = if i0 > 0 { s.code(i0 - 1) } else { 0 };
    let n = s.code(i0 + 1);
    match cur {
        // HTh in it/at before h: Hut
        0x1982 => {
            if i0 != 0 && is(s, text, &[IT, AT]) && n == 0x952 {
                s.set(i0, 0x59f2);
            }
        }
        // I in ...believe... before l: HEh
        0x781 => {
            if super::contains(text, s.pe.cstr(BELIEVE)) && n == 0x18a2 {
                s.set(i0, 0x501);
            }
        }
        // final u of to/into/onto before Hg: HEh
        0x3e1 => {
            if last(k) && n == 0x9e2 && is(s, text, &[TO, INTO, ONTO]) {
                s.set(i0, 0x501);
            }
        }
        0x501 => {
            // $ HEh n HEh 0x51 in ...tional... second to last: HEh n -> Hn
            if i0 != 0
                && p == 0x1752
                && n == 0x19c2
                && s.code(i0 + 2) == 0x501
                && s.code(i0 + 3) == 0x51
                && s.word(i0 - 1) == s.word(i0 + 3)
                && super::contains(text, s.pe.cstr(TIONAL))
                && (wi == s.sentence_len() as i64 - 2 || wi == s.count() as i64 - 2)
            {
                s.delete(i0);
                s.set(i0, 0x219c2);
                return;
            }
            // HEh r HEh l|r in one word: HEh r -> HR
            if n == 0x18c2
                && s.code(i0 + 2) == 0x501
                && s.word(i0) == s.word(i0 + 3)
                && [0x18a2, 0x18c2].contains(&s.code(i0 + 3))
            {
                s.delete(i0);
                s.set(i0, 0x218c2);
            }
        }
        // can as the chunk's first word: Ae -> HEh
        0x681 => {
            if is(s, text, &[CAN]) && w == 0 {
                s.set(i0, 0x501);
            }
        }
        // word-internal Ts (not ...tz...) after a phone: Hut s
        0x1912 => {
            if !last(k) && !super::contains(text, s.pe.cstr(TZ)) && i0 != 0 && !last(kind(s, i0 - 1)) {
                let w = s.word(i0);
                s.insert(i0, w, 0x59f2, KIND_PHONE);
                s.set(i0 + 1, 0x1952);
            }
        }
        0x952 => h(s, i0, w, text, prev, grade),
        // n before $ in ...nt.../...nsc...: $ -> T$, skipped
        0x19c2 => {
            if n == 0x1752 && (super::contains(text, s.pe.cstr(NT)) || super::contains(text, s.pe.cstr(NSC))) {
                s.set(i0, 0x19c2);
                s.set(i0 + 1, 0x1712);
                *i += 1;
            }
        }
        // to after a word ending in n/N: th -> HTh
        0x39f2 => {
            if i0 != 0 && (p == 0x19c2 || p == 0xfc2) && last(kind(s, i0 - 1)) && is(s, text, &[TO]) {
                s.set(i0, 0x1982);
            }
        }
        0x19e2 => {
            if text.is_some() {
                if chunk_and(s, w, text) {
                    return drop_back(s, i);
                }
                if is(s, text, &[AND]) && next.is_some() && is(s, next, &AND_D) {
                    s.set(i0, 0x59e2);
                    return;
                }
            }
            // 0x1000203b: final d after Dg before a vowel (not th): Hut
            if i0 != 0 && last(k) && p == 0x1702 && n & 3 == 2 && n != 0x39f2 {
                s.set(i0, 0x59f2);
                return;
            }
            // final d before you...: Dg
            if last(k) && is(s, next, &YOU) {
                s.set(i0, 0x1702);
            }
        }
        0x59e2 => {
            if chunk_and(s, w, text) {
                drop_back(s, i);
            }
        }
        // final Hut after z before a vowel, after Hup before ph, after s
        // before s: dropped
        0x59f2 => {
            if i0 != 0
                && last(k)
                && ((p == 0x1942 && n & 3 == 2) || (p == 0x5ff2 && n == 0x3ff2) || (p == 0x1952 && n == 0x1952))
            {
                drop_back(s, i);
            }
        }
        // l after HR/Hn or after HEh n|r: HEh l
        0x218a2 => {
            let after = i0 != 0 && (p == 0x219c2 || p == 0x218c2);
            let alt = i0 >= 2 && s.code(i0 - 2) == 0x501 && (p == 0x19c2 || p == 0x18c2);
            if after || alt {
                let w = s.word(i0);
                s.insert(i0, w, 0x501, KIND_PHONE);
                s.set(i0 + 1, 0x18a2);
            }
        }
        _ => {}
    }
}

/// The h cases (0x952): word-initial h of an unstressed word (grade 3, not
/// have/has/had to) after HTh, z, or a vowel-like phone is dropped
/// (it/at's Hut then HTh); have/has/had after must, and the pronouns after
/// and, have/has/had after there lose it too.
fn h(s: &mut Stage, i: usize, w: u32, text: Option<&[u8]>, prev: Option<&[u8]>, grade: u8) {
    if i == 0 {
        return;
    }
    let p = s.code(i - 1);
    if last(kind(s, i - 1)) && !is(s, text, &HAVE_TO) && grade == 3 {
        // 0x10001a48
        let drop = if p == 0x1982 || p == 0x1942 {
            true
        } else if p & 3 == 2 && (p >> 4) & 1 == 0 {
            false
        } else {
            [0x102, 0x1e2, 0x142].contains(&(p & 0x1e3))
        };
        if drop {
            // 0x10001b15
            s.delete(i);
            if is(s, prev, &[IT, AT]) && s.code(i - 1) == 0x59f2 {
                s.set(i - 1, 0x1982);
            }
            return;
        }
    }
    // 0x10001b9e
    if prev.is_none() {
        return;
    }
    if is(s, prev, &[MUST])
        && s.word(i - 1) != s.word(i)
        && is(s, text, &HAVE)
        && w as i64 != s.sentence_len() as i64 - 2
    {
        s.delete(i);
        return;
    }
    // 0x10001c41
    if is(s, prev, &[AND]) && is(s, text, &AND_H) {
        s.delete(i);
        return;
    }
    if is(s, prev, &[THERE]) && is(s, text, &HAVE) {
        s.delete(i);
    }
}

/// Steven 0x10003ac0: k Dz t p (unaspirated) as g Ts d b (ids 0x39 0x3c
/// 0x36 0x33); Hg Hk Hj Hw not first and not word-final are dropped; the
/// rest as 0x100024a0 (the default).
fn steven_meta(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    let id = match s.code(*i) {
        0xff2 => 0x39,
        0x1902 => 0x3c,
        0x19f2 => 0x36,
        0x1ff2 => 0x33,
        0x9e2 | 0x9f2 | 0x10ce | 0x100ece if *i != 0 && !last(kind(s, *i)) => {
            *i += 1;
            return;
        }
        _ => return meta_default(s, i, out),
    };
    out.push(id);
    *i += 1;
}
