//! LTTS7Grace.dll (Australian English): `FonVidAllofonoVoce` 0x100014a0
//! (word replacements from the library's word table, 0x10001360 with the
//! lookup 0x10001210; linking r, flapped t, a few word rules) and
//! `FonVidMetaFonema` 0x10003ea0 (p t k said as other entries, glottal
//! onsets at a word start dropped; every id then through
//! `FonVidAnalisi2Sintesi` 0x10003dd0).
//!
//! The library predates 7.3: the list holds its own phone codes (`t`
//! 0x19f2, `r` 0x18c2, `w` 0x401ec2 …), and the rules read the elements'
//! kinds (`w` 0x77 a word's last phone, `p` 0x70 a pause).
//!
//! `local_22` (`'I'` when the FON ctx's byte +0x18 is 7..10, else `'D'`)
//! only gates the "are" rule; ctx+0x18 is the language module's stale
//! token count, below 7 in ordinary sentences: taken as [`MODE`] `'D'`.

use super::{eq, starts, contains, Port, Rules, Stage, KIND_PAUSE, KIND_PHONE, KIND_WORD_END};

pub(super) const GRACE: Port = Port {
    voice: "Grace",
    table: 0x1000b998,
    functions: &[
        ("GetVersion", 0x10001150),
        ("ELQGetPhonemes", 0x10001060),
        ("FonVidAllofonoVoce", 0x100014a0),
        ("FonVidAllofonoAnalisi", 0x10002b40),
        ("FonVidMetaFonema", 0x10003ea0),
        ("FonVidAnalisi2Sintesi", 0x10003dd0),
        ("FonVidIngloba", 0x10003e60),
    ],
    rules: Rules {
        allofono: Some(grace),
        meta: grace_meta,
    },
};

/// `local_22`: the FON ctx's byte +0x18 taken as below 7 (see the module
/// doc); the only rule it gates (0x501 "are", `== 'I'`) never fires.
const MODE: u8 = b'D';

/// A voice's addresses for the shared code (Grace, Alan).
pub(super) struct Lib {
    /// the word table (0x54-byte entries: name, up to 16 codes from +0x10
    /// ended by 0, the stress grade at +0x50 or -1 for any)
    pub words: u32,
    /// the table's end-marker name ("-1")
    pub words_end: u32,
    /// `FonVidAnalisi2Sintesi`'s index bytes and jump table (ids 0x2f..)
    pub a2s_index: u32,
    pub a2s_jump: u32,
    /// the last index `FonVidAnalisi2Sintesi` maps (id - 0x2f)
    pub a2s_max: u8,
}

const GRACE_LIB: Lib = Lib {
    words: 0x1000e000,
    words_end: 0x1000ba24,
    a2s_index: 0x10003e30,
    a2s_jump: 0x10003e08,
    a2s_max: 0x2d, // 0x10003dda `cmp ecx, 0x2d`
};

/// The element is a word's last phone or a pause (kind `w`/`p`).
pub(super) fn wp(s: &Stage, k: usize) -> bool {
    matches!(s.list.get(k).map(|p| p.kind), Some(KIND_WORD_END | KIND_PAUSE))
}

pub(super) fn vowel(c: u32) -> bool {
    c & 3 == 1
}

fn set_kind(s: &mut Stage, k: usize, kind: u8) {
    if let Some(p) = s.list.get_mut(k) {
        p.kind = kind;
    }
}

pub(super) fn kind(s: &Stage, k: usize) -> u8 {
    s.list.get(k).map_or(0, |p| p.kind)
}

/// 0x10001210 (Alan the same): the word table entry for `t` said with
/// stress grade `grade` (passed in `al`): the first entry named `t` whose
/// grade is -1 or `grade`, scanning stopping at the end marker.
pub(super) fn lookup(s: &Stage, lib: &Lib, t: Option<&[u8]>, grade: u8) -> Option<u32> {
    let t = t?;
    let end = s.pe.cstr(lib.words_end);
    let name = |k: u32| s.pe.cstr(lib.words + k * 0x54);
    let mut k = 0;
    while name(k) != end && name(k) != t {
        k += 1;
    }
    loop {
        if name(k) != t {
            return None;
        }
        let g = s.pe.u32(lib.words + k * 0x54 + 0x50);
        if g == u32::MAX || g == grade as u32 {
            return Some(k);
        }
        k += 1;
    }
}

/// 0x10001360 (Alan the same): at a word's first phone, a word in the
/// table is replaced: its phones up to and including its `w`/`p` element
/// deleted, the entry's codes inserted (kind `i`; 0x48 keeps the deleted
/// last element's code), the last one taking the deleted element's kind;
/// `*i` left on it.
pub(super) fn replace(s: &mut Stage, lib: &Lib, i: &mut usize) {
    let w = s.word(*i);
    let grade = s.rec(w as i64).map_or(0, |r| r.f[0x12]);
    let Some(e) = lookup(s, lib, s.text(w as i64), grade) else {
        return;
    };
    while *i < s.list.len() && !wp(s, *i) {
        s.delete(*i);
    }
    if *i >= s.list.len() {
        return;
    }
    let (code, k) = (s.code(*i), kind(s, *i));
    s.delete(*i);
    let entry = lib.words + e * 0x54;
    for j in 0..16 {
        let c = s.pe.u32(entry + 0x10 + j * 4);
        if c == 0 {
            break;
        }
        if *i < s.list.len() {
            s.insert(*i, w, if c == 0x48 { code } else { c }, KIND_PHONE);
            *i += 1;
        }
    }
    set_kind(s, i.wrapping_sub(1), k);
    *i = i.wrapping_sub(1);
}

/// `FonVidAnalisi2Sintesi`: ids 0x2f..0x5c through the byte index and
/// jump table; a target `mov al, imm8` (0xb0) gives the new id, the plain
/// `ret` keeps it.
pub(super) fn a2s(s: &Stage, lib: &Lib, id: u8) -> u8 {
    let k = id.wrapping_sub(0x2f);
    if k > lib.a2s_max {
        return id;
    }
    let slot = s.pe.cstr(lib.a2s_index + k as u32).first().copied().unwrap_or(0);
    let target = s.pe.u32(lib.a2s_jump + slot as u32 * 4);
    let code = s.pe.cstr(target);
    if code.first() == Some(&0xb0) {
        code.get(1).copied().unwrap_or(id)
    } else {
        id
    }
}

/// The record texts the function reads: the element's word's (+4) and the
/// next record's ("" when the word index equals the chunk's count).
pub(super) struct Texts {
    pub cur: Option<Vec<u8>>,
    pub next: Option<Vec<u8>>,
    pub grade: u8,
}

pub(super) fn texts(s: &Stage, i: usize) -> Texts {
    let w = s.word(i);
    Texts {
        cur: s.text(w as i64).map(<[u8]>::to_vec),
        next: if w as usize == s.count() { Some(Vec::new()) } else { s.text(w as i64 + 1).map(<[u8]>::to_vec) },
        grade: s.rec(w as i64).map_or(0, |r| r.f[0x12]),
    }
}

/// `_strncmp(t + len - n, lit, n) == 0` with `len >= n`.
pub(super) fn ends(t: Option<&[u8]>, lit: &[u8]) -> bool {
    t.is_some_and(|t| t.ends_with(lit))
}

// Grace's literals.
const WITH: u32 = 0x1000ba64;
const THE: u32 = 0x1000ba6c;
const USED: u32 = 0x1000ba7c;
const TO: u32 = 0x1000ba78;
const IT: u32 = 0x1000ba30;
const TRADITIONAL: u32 = 0x1000ba8c;
const SHIRE: u32 = 0x1000ba70;
const D: u32 = 0x1000b298;
const AUSTRA: u32 = 0x1000ba84;
const LLY: u32 = 0x1000ba98;
const IA: u32 = 0x1000ba60;
const IO: u32 = 0x1000ba5c;
const DIRECT: u32 = 0x1000ba28;
const LY: u32 = 0x1000ba58;
const ELY: u32 = 0x1000ba54;
const TED: u32 = 0x1000ba50;
const TIV: u32 = 0x1000ba4c;
const TING: u32 = 0x1000ba44;
const TOR: u32 = 0x1000ba40;
const TY: u32 = 0x1000ba3c;
const TIES: u32 = 0x1000ba34;

pub(super) const FLAP: u32 = 0x1982;

/// 0x100014a0 `FonVidAllofonoVoce` (Grace).
fn grace(s: &mut Stage, pi: &mut usize) {
    let lib = &GRACE_LIB;
    let tx = texts(s, *pi);
    let cur = tx.cur.as_deref();
    let next = tx.next.as_deref();
    let grade = tx.grade;
    let len = cur.map_or(0, <[u8]>::len);
    let lit = |s: &Stage, va: u32| s.pe.cstr(va).to_vec();
    let is = |s: &Stage, t: Option<&[u8]>, va: u32| eq(t, s.pe.cstr(va));

    let i = *pi;
    let c0 = s.code(i);
    if c0 == 0x17 || c0 == 0x13 {
        return;
    }
    if i == 0 || wp(s, i - 1) || [0x9e2, 0x9f2, 0x10ce, 0x401ece].contains(&s.code(i - 1)) {
        replace(s, lib, pi);
    }
    let i = *pi;
    if wp(s, i) && s.code(i) == 0x18c2 && !vowel(s.code(i + 1)) && lookup(s, lib, cur, grade).is_some() {
        let k = kind(s, i);
        set_kind(s, i.wrapping_sub(1), k);
        s.delete(i);
        *pi = i.wrapping_sub(1);
    }
    let i = *pi;
    let c = |s: &Stage, k: usize| s.code(k);
    let wd = |s: &Stage, k: usize| s.word(k);
    match c(s, i) {
        // 0x10001c42
        0x1b52 => {
            let n = c(s, i + 1);
            if is(s, cur, WITH) && (vowel(n) || n == 0x401ec2 || n == 0x10c2) {
                s.set(i, 0x1b42);
            }
        }
        0x1b42 => {
            if i != 0 && is(s, cur, THE) && (c(s, i - 1) == 0x19c2 || c(s, i - 1) == 0x1fc2) {
                s.set(i, 0x19c2);
            }
        }
        0x19e2 => {
            if i != 0 && wp(s, i) && c(s, i - 1) == 0x19c2 && [0x401ec2, 0x10c2].contains(&c(s, i + 1)) {
                s.set(i, 0x59e2);
            }
        }
        0x1752 => {
            if len >= 5 && ends(cur, &lit(s, SHIRE)) && (wp(s, i + 1) || wp(s, i + 2)) {
                s.set(i + 1, 0x4785);
                *pi += 1;
            }
        }
        0x1942 => {
            if is(s, cur, USED) && next.is_some() && is(s, next, TO) {
                s.set(i, 0x1952);
                s.set(i + 1, 0x59f2);
                *pi += 1;
            }
        }
        0x1702 => {
            if i != 0 && wp(s, i) && len > 0 && ends(cur, &lit(s, D)) && c(s, i + 1) == 0x10c2 {
                s.set(i, 0x19e2);
            }
        }
        0xff2 | 0x2ff2 => {
            if [0x17, 0x13].contains(&c(s, i + 1)) {
                s.set(i, 0x4ff2);
            }
        }
        0x1ff2 | 0x3ff2 => {
            if [0x17, 0x13].contains(&c(s, i + 1)) {
                s.set(i, 0x5ff2);
            }
        }
        // 0x10002a63
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
            // "are" (0x1000ba9c) only when MODE is 'I'
            let _ = MODE;
            if len > 2
                && ends(cur, &lit(s, LLY))
                && c(s, i + 1) == 0x18a2
                && c(s, i + 2) == 0x7c1
                && wp(s, i + 2)
            {
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
            if wp(s, i) && len > 1 && ends(cur, &lit(s, IA)) {
                s.set(i, 0x501);
                let w = wd(s, i);
                s.insert(i, w, 0x7c1, KIND_PHONE);
                *pi += 1;
                return;
            }
            if c(s, i + 1) == 0x501 && wp(s, i + 1) && len >= 2 && ends(cur, &lit(s, IO)) {
                s.set(i, 0x7c1);
                s.set(i + 1, 0xad05);
                *pi += 1;
            }
        }
        0x4ff2 => {
            if wp(s, i) && c(s, i + 1) == 0x10c2 {
                s.set(i, 0x2ff2);
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
        0x39f2 | 0x59f2 => t_rules(s, pi, cur, &GRACE_T),
        _ => {}
    }
}


/// The t rules' literals (Grace, Alan).
pub(super) struct TLits {
    pub ly: u32,
    pub ely: u32,
    pub ted: u32,
    pub tiv: u32,
    pub ting: u32,
    pub tor: u32,
    pub ty: u32,
    pub ties: u32,
    pub it: Option<u32>,
    pub to: u32,
}

const GRACE_T: TLits = TLits { ly: LY, ely: ELY, ted: TED, tiv: TIV, ting: TING, tor: TOR, ty: TY, ties: TIES, it: Some(IT), to: TO };

/// The tail after the dispatch (Grace 0x10001fbc on; t 0x39f2 0x59f2):
/// the t's flap (0x1982) between vowels, its glottal or pre-pause forms.
pub(super) fn t_rules(s: &mut Stage, pi: &mut usize, cur: Option<&[u8]>, tl: &TLits) {
    let c = |s: &Stage, k: usize| s.code(k);
    let wd = |s: &Stage, k: usize| s.word(k);
    let has = |s: &Stage, va: u32| contains(cur, s.pe.cstr(va));
    let len = cur.map_or(0, <[u8]>::len);
    let i = *pi;
    // ly/ely: a glottal 0x9e2
    if i != 0
        && (vowel(c(s, i - 1)) || c(s, i - 1) == 0x19c2 || c(s, i - 1) == 0x18c2)
        && wd(s, i - 1) == wd(s, i)
        && ((len > 1 && ends(cur, s.pe.cstr(tl.ly))) || (len > 2 && ends(cur, s.pe.cstr(tl.ely))))
        && wp(s, i + 2)
    {
        s.set(i, 0x9e2);
        return;
    }
    let p1 = if i >= 1 { c(s, i - 1) } else { 0 };
    let p2 = if i >= 2 { c(s, i - 2) } else { 0 };
    let n1 = c(s, i + 1);
    let flap = |s: &mut Stage| s.set(i, FLAP);
    // 0x1000213b
    if p1 & 0x10003 == 0x10001 && n1 & 0x10003 == 1 && wd(s, i - 1) == wd(s, i + 1) {
        return flap(s);
    }
    let rhotic = [0x18c2, 0x19c2, 0x219c2];
    let mut u = p1;
    if p2 & 0x10003 == 0x10001 && i >= 1 && rhotic.contains(&p1) && n1 & 0x10003 == 1 && wd(s, i - 2) == wd(s, i + 1) {
        return flap(s);
    }
    if p2 & 0x10003 == 0x10001 && i == 0 {
        u = 0;
    }
    let suffixes = |s: &mut Stage| {
        let n2 = s.code(i + 2);
        if (has(s, tl.ted) && (n2 == 0x19e2 || n2 == 0x59e2))
            || (has(s, tl.tiv) && n2 == 0x1d42)
            || (has(s, tl.ting) && n2 == 0xfc2)
            || (has(s, tl.tor) && n2 == 0x18c2)
        {
            s.set(i, FLAP);
        }
    };
    if u & 0x10003 == 1 && (n1 == 0x501 || n1 == 0x781) && wd(s, i - 1) == wd(s, i + 1) {
        if cur.is_none() {
            return;
        }
        return suffixes(s);
    }
    if p2 & 0x10003 == 1 && i >= 1 && rhotic.contains(&p1) && (n1 == 0x501 || n1 == 0x781) && wd(s, i - 2) == wd(s, i + 1) {
        if cur.is_none() {
            return;
        }
        return suffixes(s);
    }
    if i >= 1
        && [0x501, 0x781, 0x19c2, 0x18c2].contains(&p1)
        && n1 == 0x7c1
        && wd(s, i - 1) == wd(s, i + 1)
        && (has(s, tl.ty) || has(s, tl.ties))
    {
        return flap(s);
    }
    // 0x1000261c
    if n1 == 0x218a2 && !wp(s, i) {
        return flap(s);
    }
    let before = |s: &mut Stage, pi: &mut usize| {
        s.set(i, FLAP);
        if s.code(i + 1) == 0x952 {
            s.set(i + 1, 0x942);
            *pi += 1;
        }
    };
    if i != 0 && vowel(p1) && wp(s, i) && (vowel(n1) || n1 == 0x952) {
        return before(s, pi);
    }
    if let Some(it) = tl.it {
        if eq(cur, s.pe.cstr(it)) && [0x10c2, 0x10ce, 0x401ec2, 0x401ece].contains(&n1) {
            return flap(s);
        }
    }
    if i != 0 {
        if rhotic.contains(&p1) && wp(s, i) && (n1 & 0x10003 == 1 || n1 == 0x952) {
            return before(s, pi);
        }
        if eq(cur, s.pe.cstr(tl.to)) {
            if vowel(p1) && p1 & 0xc003 != 1 {
                return flap(s);
            }
            if [0x7c1, 0x107c5, 0x3e1, 0x103e5, 0x781, 0x10781].contains(&p1) {
                return flap(s);
            }
        }
    }
    if n1 == 0x17 || n1 == 0x13 {
        s.set(i, 0x59f2);
        return;
    }
    if n1 == 0x10c2 && wd(s, i) == wd(s, i + 2) && [0x103e5, 0x143a5, 0x43a5, 0x3e1].contains(&c(s, i + 2)) {
        s.set(i, 0x1712);
        s.delete(i + 1);
    }
}

/// 0x10003ea0 `FonVidMetaFonema` (Grace): p t k (0x1ff2 0x19f2 0xff2) as
/// ids 0x3a 0x3c 0x3e; the glottals 0x9e2 0x9f2 0x10ce 0x401ece after a
/// `w`/`p` element say nothing (the previous id is mapped again: a no-op);
/// the rest the default (0x10002ad0); every id then through 0x10003dd0.
fn grace_meta(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    meta_common(s, &GRACE_LIB, i, out, &[(0xff2, 0x3e), (0x19f2, 0x3c), (0x1ff2, 0x3a)], &[0x10ce, 0x9e2, 0x9f2, 0x401ece]);
}

/// The FonVidMetaFonema shape both share: `ids` written directly, the
/// `absorbed` codes after a `w`/`p` element writing nothing, the rest the
/// default; the id then through `FonVidAnalisi2Sintesi`.
pub(super) fn meta_common(s: &Stage, lib: &Lib, i: &mut usize, out: &mut Vec<u8>, ids: &[(u32, u8)], absorbed: &[u32]) {
    let code = s.code(*i);
    let id = if let Some(&(_, id)) = ids.iter().find(|&&(c, _)| c == code) {
        *i += 1;
        id
    } else if absorbed.contains(&code) && *i > 0 && wp(s, *i - 1) {
        *i += 1;
        return;
    } else {
        let mut v = Vec::new();
        super::meta_default(s, i, &mut v);
        v[0]
    };
    out.push(a2s(s, lib, id));
}
