//! LTTS7Susan.dll (US English): `FonVidAllofonoVoce` 0x10001210 (per-word
//! connected-speech rules: function words' weak forms, flaps, glottal
//! stops, palatalisation before "you", elisions) and `FonVidMetaFonema`
//! 0x10004250 (p t k and a few others said as other table entries).
//!
//! The words the rules test are read from the library's bytes by address
//! (`.rdata`, the `mov edi, imm32` before each `repz cmpsb`, a whole-string
//! compare).

use super::{eq, Port, Rules, Stage};

pub(super) const SUSAN: Port = Port {
    voice: "Susan",
    table: 0x1000b8e8,
    functions: &[
        ("GetVersion", 0x10001150),
        ("ELQGetPhonemes", 0x10001060),
        ("FonVidAllofonoVoce", 0x10001210),
        ("FonVidAllofonoAnalisi", 0x10002ef0),
        ("FonVidMetaFonema", 0x10004250),
        ("FonVidAnalisi2Sintesi", 0x10004320),
        ("FonVidIngloba", 0x100043a0),
    ],
    rules: Rules {
        allofono: Some(allofono),
        meta,
    },
};

// String literals (`.rdata`).
const WELL: u32 = 0x1000bb64; // we'll
const DIFFERENT: u32 = 0x1000bb50; // different
const FORBIDDEN: u32 = 0x1000bb44; // forbidden
const FOR: u32 = 0x1000bb40; // for
const ITS: u32 = 0x1000bb3c; // its
const BECAUSE: u32 = 0x1000bb34; // because
const IT: u32 = 0x1000bb28; // it
const DOT: u32 = 0x1000bb24; // dot
const THEYRE: u32 = 0x1000baf4; // they're
const THEM: u32 = 0x1000baec; // them
const THAT: u32 = 0x1000bae4; // that
const CAN: u32 = 0x1000bae0; // can
const WHAT: u32 = 0x1000bad8; // what
const TO: u32 = 0x1000bad4; // to
const INTO: u32 = 0x1000bacc; // into
const ONTO: u32 = 0x1000bac4; // onto
const YOUR: u32 = 0x1000babc; // your
const YOURE: u32 = 0x1000bab4; // you're
const YOURS: u32 = 0x1000baac; // yours
const SHOULD: u32 = 0x1000baa4; // should
const COULD: u32 = 0x1000ba9c; // could
const HAVE_TO: u32 = 0x1000ba94; // have to
const HAS_TO: u32 = 0x1000ba8c; // has to
const HAD_TO: u32 = 0x1000ba84; // had to
const AT: u32 = 0x1000ba80; // at
const MUST: u32 = 0x1000ba78; // must
const HAVE: u32 = 0x1000ba70; // have
const HAS: u32 = 0x1000ba6c; // has
const HAD: u32 = 0x1000ba68; // had
const AND: u32 = 0x1000ba64; // and
const HE: u32 = 0x1000ba60; // he
const HIM: u32 = 0x1000ba5c; // him
const HIS: u32 = 0x1000ba58; // his
const HER: u32 = 0x1000ba54; // her
const HED: u32 = 0x1000ba4c; // he'd
const HELL: u32 = 0x1000ba44; // he'll
const HES: u32 = 0x1000ba3c; // he's
const HERS: u32 = 0x1000ba34; // hers
const THERE: u32 = 0x1000ba2c; // there
const OURS: u32 = 0x1000ba24; // ours
const OUR: u32 = 0x1000ba20; // our
const ARENT: u32 = 0x1000ba18; // aren't
const THEIR: u32 = 0x1000ba10; // their
const MY: u32 = 0x1000ba00; // my
const THEYD: u32 = 0x1000b9f8; // they'd
const THEYVE: u32 = 0x1000b9f0; // they've
const THERED: u32 = 0x1000b9e8; // there'd
const COULDNT: u32 = 0x1000b9dc; // couldn't
const WOULDNT: u32 = 0x1000b9d0; // wouldn't
const SHOULDNT: u32 = 0x1000b9c4; // shouldn't
const I: u32 = 0x1000b354; // i
const YOU: u32 = 0x1000b9c0; // you
const YOUVE: u32 = 0x1000b9b8; // you've
const YOUD: u32 = 0x1000b9b0; // you'd
const YOULL: u32 = 0x1000b9a8; // you'll
const YER: u32 = 0x1000b9a4; // yer
const YEH: u32 = 0x1000b9a0; // yeh
const YOURSELF: u32 = 0x1000b994; // yourself
const YOURSELVES: u32 = 0x1000b988; // yourselves
const DISCOTHEQUE: u32 = 0x1000b974; // discotheque

/// `strcmp(t, lit) == 0` for any of the literals at `vas`.
fn is(s: &Stage, t: Option<&[u8]>, vas: &[u32]) -> bool {
    vas.iter().any(|&va| eq(t, s.pe.cstr(va)))
}

/// `strstr(t, lit)` (0x100044f0) with a literal pushed by the code itself
/// (the `push imm32` before `call 0x100044f0`).
fn has(s: &Stage, t: Option<&[u8]>, va: u32) -> bool {
    super::contains(t, s.pe.cstr(va))
}

// The `strstr` literals (pushed before `call 0x100044f0`).
const TZ: u32 = 0x1000ba04; // tz
const NT: u32 = 0x1000ba0c; // nt
const NSC: u32 = 0x1000ba08; // nsc
const BELIEVE: u32 = 0x1000bb2c; // believe
const TIONAL: u32 = 0x1000bb5c; // tional
// `_strncmp` (0x10004427) suffix literals
const LY: u32 = 0x1000b984; // ly
const ELY: u32 = 0x1000b980; // ely

/// Set element `k`'s record (the "couldn't" merge writes the word index).
fn set_word(s: &mut Stage, k: usize, w: u32) {
    if let Some(p) = s.list.get_mut(k) {
        p.word = w;
    }
}

/// 0x10001210 `FonVidAllofonoVoce`, dispatched on the phone's code.
fn allofono(s: &mut Stage, pi: &mut usize) {
    let i = *pi;
    let w = s.word(i);
    // the texts: the record's (+4), the previous record's ("" for the
    // chunk's first, 0x1000b38a) and the next one's ("" when the record
    // index equals the chunk's count)
    let cur: Option<Vec<u8>> = s.text(w as i64).map(<[u8]>::to_vec);
    let prev: Option<Vec<u8>> = if w == 0 { Some(Vec::new()) } else { s.text(w as i64 - 1).map(<[u8]>::to_vec) };
    let next: Option<Vec<u8>> =
        if w as usize == s.count() { Some(Vec::new()) } else { s.text(w as i64 + 1).map(<[u8]>::to_vec) };
    let (cur, prev, next) = (cur.as_deref(), prev.as_deref(), next.as_deref());
    let rec = s.rec(w as i64);
    let mark = rec.map_or(0, |r| r.f[0xe]); // +0xe
    let grade = rec.map_or(0, |r| r.f[0x12]); // +0x12
    let c = |s: &Stage, k: usize| s.code(k);
    let wd = |s: &Stage, k: usize| s.word(k);
    let im1 = i.wrapping_sub(1);

    match c(s, i) {
        // 0x10001d75: h after a word that ends in ... (weak "have", "him")
        0x800104 => {
            if i == 0 {
                return;
            }
            let prevc = c(s, im1);
            if wd(s, im1) != w && !is(s, cur, &[HAVE_TO, HAS_TO, HAD_TO]) {
                // 0x10001de3
                if grade == 3
                    && (prevc == 0x3010094
                        || prevc == 0x1010104
                        || (prevc & 0x1000004 != 0x1000004
                            && (prevc & 0x1004 == 0x1004 || prevc & 0x14 == 0x14 || prevc & 0x104 == 0x104)))
                {
                    s.delete(i);
                    if prev.is_none() || !is(s, prev, &[IT, AT]) {
                        return;
                    }
                    if c(s, *pi - 1) == 0x22010014 {
                        s.set(*pi - 1, 0x3010094);
                    }
                    return;
                }
            }
            // 0x10001ed9
            if prev.is_none() {
                return;
            }
            if is(s, prev, &[MUST]) && wd(s, im1) != w && is(s, cur, &[HAVE, HAS, HAD]) && w as usize != s.sentence_len().wrapping_sub(2) {
                s.delete(i);
                return;
            }
            // 0x10001f7f
            if is(s, prev, &[AND]) && is(s, cur, &[HAVE, HAS, HAD, HE, HIM, HIS, HER, HED, HELL, HES, HERS]) {
                s.delete(i);
                return;
            }
            if is(s, prev, &[THERE]) && is(s, cur, &[HAVE, HAS, HAD]) {
                s.delete(i);
            }
        }
        // 0x10001a1b
        0x20411 => {
            if (is(s, cur, &[THEYRE]) && grade == 3) || (is(s, cur, &[THEM]) && grade == 2) {
                s.set(i, 0x20421);
            }
        }
        // 0x10001842
        0x4411 => {
            if cur.is_some() {
                if is(s, cur, &[ITS]) && grade == 3 {
                    s.set(i, 0x4421);
                    return;
                }
                if is(s, cur, &[BECAUSE]) {
                    s.set(i, 0x2411);
                    return;
                }
                if has(s, cur, BELIEVE) && c(s, i + 1) == 0x1010804 {
                    s.set(i, 0x10851);
                    return;
                }
            }
            if i == 0 || !is(s, cur, &[IT]) || prev.is_none() || !is(s, prev, &[DOT]) {
                return;
            }
            // "dot it"
            s.set(i, 0x80001c);
            s.set(i + 1, 0x2280893);
            s.set(im1, 0x3010094);
            *pi += 1;
            let k = *pi;
            if k < s.list.len() {
                let wk = wd(s, k);
                s.insert(k + 1, wk, 0x12010014, 0);
            }
            if k + 1 < s.list.len() {
                let wk = wd(s, k + 1);
                s.insert(k + 2, wk, 0x24a1, 0);
            }
        }
        // 0x10001482
        0x10851 => {
            if i != 0
                && c(s, im1) == 0x20104
                && c(s, i + 1) == 0x1010024
                && c(s, i + 2) == 0x10851
                && c(s, i + 3) == 0x50
                && wd(s, im1) == wd(s, i + 3)
                && has(s, cur, TIONAL)
                && (w as usize == s.sentence_len().wrapping_sub(2) || w as usize == s.count().wrapping_sub(2))
            {
                s.delete(i);
                s.set(i, 0x9010024);
                return;
            }
            if c(s, i + 1) == 0x1010024 && is(s, cur, &[DIFFERENT, FORBIDDEN]) {
                s.delete(i);
                s.set(i, 0x9010024);
                return;
            }
            if c(s, i + 1) == 0x1010404
                && c(s, i + 2) == 0x10851
                && wd(s, i) == wd(s, i + 3)
                && (c(s, i + 3) == 0x1010804 || c(s, i + 3) == 0x1010404)
            {
                s.delete(i);
                s.set(i, 0x9010404);
                return;
            }
            if c(s, i + 1) == 0x1010404 && is(s, cur, &[DIFFERENT]) {
                s.delete(i);
                s.set(i, 0x9010404);
                return;
            }
            if i != 0 && is(s, cur, &[FOR]) && wd(s, i + 1) != wd(s, i + 2) && grade == 3 {
                s.delete(i);
                s.set(i, 0x9010404);
                return;
            }
            if i >= 3
                && c(s, i - 3) == 0x3010014
                && c(s, i - 2) == 0x21021
                && c(s, i - 1) == 0x3002014
                && c(s, i + 1) == 0x1010804
                && c(s, i + 2) == 0x81080404
                && c(s, i + 3) == 0x103011
            {
                s.delete(i);
                s.set(i, 0x9010804);
            }
        }
        // 0x100013a3
        0x24a1 => {
            if i != 0
                && c(s, im1) == 0x81102404
                && wd(s, i + 1) != wd(s, i + 2)
                && c(s, i + 1) == 0x1010804
                && is(s, cur, &[WELL])
                && wd(s, im1) == wd(s, i + 1)
                && grade == 0
                && mark == b'*'
            {
                s.delete(i);
                *pi -= 1;
            }
        }
        0x2411 => {
            if i != 0
                && c(s, im1) == 0x81102404
                && wd(s, i + 1) != wd(s, i + 2)
                && is(s, cur, &[WELL])
                && mark == b'*'
                && grade != 0
            {
                s.delete(i);
                *pi -= 1;
            }
        }
        0x36 | 0x3010014 => t_rules(s, pi, cur, next),
        // 0x10001b8f
        0x103011 => {
            if w != wd(s, i + 1) && c(s, i + 1) == 0x1800014 && is(s, cur, &[TO, INTO, ONTO]) {
                s.set(i, 0x10851);
            }
        }
        0x21011 => {
            if i != 0 && is(s, cur, &[WHAT]) && grade == 3 {
                s.set(i, 0x21021);
            }
        }
        0x40411 => {
            if is(s, cur, &[THAT]) && grade == 2 {
                s.set(i, 0x40421);
            } else if is(s, cur, &[CAN]) && w == 0 {
                s.set(i, 0x10851);
            }
        }
        0x105011 => {
            if (is(s, cur, &[YOUR]) && grade == 2) || (is(s, cur, &[YOURE]) && grade == 3) {
                s.set(i, 0x20a91);
                s.delete(*pi + 1);
            } else if is(s, cur, &[COULD]) && grade == 2 {
                s.set(i, 0x105021);
            }
        }
        // 0x10001bfe
        0x105021 => {
            if cur.is_none() {
                return;
            }
            if is(s, cur, &[YOUR, YOURE]) && grade == 0 {
                s.set(i, 0x20aa1);
                s.delete(*pi + 1);
                return;
            }
            if is(s, cur, &[YOURS]) && grade == 3 {
                s.delete(i);
            } else if is(s, cur, &[YOURS]) && (grade == 0 || grade == 2) {
                s.set(i, 0x1210a1);
            } else if is(s, cur, &[SHOULD]) && grade == 3 {
                s.set(i, 0x105011);
            }
        }
        // 0x1000250f
        0x3008104 => {
            if cur.is_none() {
                return;
            }
            let (n1, n2) = (c(s, i + 1), c(s, i + 2));
            if is(s, cur, &[THEIR]) && grade == 3 && n1 == 0x9010404 {
                s.insert(i + 1, w, 0x20421, 0);
            } else if is(s, cur, &[THEIR, THERED]) && (grade == 0 || grade == 2) && n1 == 0x20421 && n2 == 0x1010404 {
                s.set(i + 2, 0x9010404);
            } else if is(s, cur, &[THERED]) && grade == 3 && n1 == 0x20411 && n2 == 0x1010404 {
                s.set(i + 1, 0x20421);
            }
        }
        // 0x100023ef
        0x2220493 => {
            if i == 0 {
                return;
            }
            let ok = c(s, im1) == 0x3008104 && wd(s, i + 1) != wd(s, i + 2) && cur.is_some() && grade == 3;
            if ok && is(s, cur, &[THEYD]) && (c(s, i + 1) == 0x3010014 || c(s, i + 1) == 0x23010014) {
                s.set(i, 0x22204a3);
            } else if ok && is(s, cur, &[THEYVE]) && c(s, i + 1) == 0x3004104 {
                s.set(i, 0x22204a3);
            }
        }
        0x2280893 => {
            if is(s, cur, &[MY]) && grade == 3 {
                s.set(i, 0x22808a3);
            }
        }
        // 0x100022f5
        0x2011004 => {
            if w != wd(s, i + 1) || has(s, cur, TZ) || i == 0 || wd(s, im1) != wd(s, i) {
                return;
            }
            s.set(i, 0x22010014);
            let wi = wd(s, i);
            s.insert(i + 1, wi, 0x10104, 0);
        }
        // 0x10002273
        0x1010024 => {
            if c(s, i + 1) == 0x20104 && cur.is_some() && (has(s, cur, NT) || has(s, cur, NSC)) {
                s.set(i, 0x1010024);
                s.set(i + 1, 0x2021004);
                *pi += 1;
            }
        }
        // 0x1000215c
        0x1010404 => {
            if cur.is_none() {
                return;
            }
            if i != 0 && is(s, cur, &[OURS]) {
                s.set(im1, 0x828808a3);
                s.set(i, 0x9010404);
            } else if is(s, cur, &[OUR]) && grade == 3 {
                s.set(i, 0x9010404);
            } else if is(s, cur, &[ARENT]) {
                s.insert(i + 1, w, 0x10851, 0);
            } else if is(s, cur, &[THEIR]) {
                s.set(im1, 0x20421);
                s.set(i, 0x9010404);
            }
        }
        // 0x10002aca
        0x9010804 => {
            let after = i != 0 && (c(s, im1) == 0x9010024 || c(s, im1) == 0x9010404);
            if !after && !(i >= 2 && c(s, i - 2) == 0x10851 && (c(s, im1) == 0x1010024 || c(s, im1) == 0x1010404)) {
                return;
            }
            s.set(i, 0x10851);
            s.insert(i + 1, w, 0x1010804, 0);
        }
        // 0x100026e8
        0x3010094 => {
            if i != 0 && is(s, cur, &[IT, AT]) && c(s, i + 1) == 0x800104 {
                s.set(i, 0x22010014);
            }
        }
        // 0x10002bc8
        0x22010014 => {
            if i == 0 {
                return;
            }
            let p = c(s, im1);
            let t = cur.unwrap_or(b"");
            let n = t.len();
            if (p & 1 != 0 || p == 0x1010024 || p == 0x1010404)
                && wd(s, im1) == w
                && ((n > 1 && &t[n - 2..] == s.pe.cstr(LY)) || (n > 2 && &t[n - 3..] == s.pe.cstr(ELY)))
                && wd(s, i + 2) != wd(s, i + 3)
            {
                s.set(i, 0x1800014);
                return;
            }
            let split = wd(s, i) != wd(s, i + 1);
            if p == 0x1010104 && split && c(s, i + 1) == 0x3008104 {
                s.delete(i);
                return;
            }
            if (p == 0x22002014 && split && c(s, i + 1) == 0x12002014) || (p == 0x10104 && split && c(s, i + 1) == 0x10104) {
                s.delete(i);
                *pi -= 1;
            }
        }
        // 0x10002dc3
        0x12010014 => {
            if is(s, cur, &[INTO, ONTO, DISCOTHEQUE]) {
                s.set(i, 0x3010094);
                return;
            }
            // 0x10002e16
            if i == 0 {
                return;
            }
            let p = c(s, im1);
            if (p == 0x1010024 || p == 0x1100024) && wd(s, im1) != w && is(s, cur, &[TO]) {
                s.set(i, 0x3010094);
            }
        }
        _ => {}
    }
}

/// 0x1000275f: t (and code 0x36): the negative contractions' nasal release,
/// a flapped t before "he"/"i" after "and", t before "you" palatalised.
fn t_rules(s: &mut Stage, pi: &mut usize, cur: Option<&[u8]>, next: Option<&[u8]>) {
    let i = *pi;
    if s.code(i + 1) == 0x10851
        && s.code(i + 2) == 0x1010024
        && i != 0
        && s.word(i - 1) == s.word(i + 2)
        && s.code(i - 1) & 0x21 == 0x21
    {
        if cur.is_none() {
            return tail(s, pi, next);
        }
        if is(s, cur, &[COULDNT, WOULDNT, SHOULDNT]) {
            s.set(i, 0x23010014);
            *pi += 1;
            let k = *pi;
            s.delete(k);
            s.set(k, 0x9010024);
            let wp = s.word(k - 1);
            set_word(s, k, wp);
        }
    }
    // 0x10002876
    if is(s, cur, &[AND]) && is(s, next, &[I, HE, HIS, HIM, HER, HERS, HED, HELL, HES]) {
        s.set(*pi, 0x23010014);
        return;
    }
    tail(s, pi, next);
}

/// 0x10002968
fn tail(s: &mut Stage, pi: &mut usize, next: Option<&[u8]>) {
    let i = *pi;
    if i != 0 && s.word(i) != s.word(i + 1) && s.code(i - 1) == 0x3021004 {
        let n = s.code(i + 1);
        if n & 4 != 0 && n != 0x12010014 {
            s.set(i, 0x22010014);
            return;
        }
    }
    if s.word(i) == s.word(i + 1) || next.is_none() {
        return;
    }
    if is(s, next, &[YOU, YOUR, YOUVE, YOUD, YOULL, YOURS, YER, YEH, YOURSELF, YOURSELVES]) {
        s.set(i, 0x3021004);
    }
}

/// 0x10004250 `FonVidMetaFonema`: p t k (0x2002014 0x2010014 0x2100014)
/// and 0x301100c as table ids 0x33 0x36 0x39 0x3c; h 0x80001c, 0x1080404,
/// 0x1102404 and 0x1800014 at the start of a word (the previous element
/// another record's) say nothing; the rest the default (0x10002e80: the
/// first table entry with the code, else 1).
fn meta(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    let id = match s.code(*i) {
        0x2002014 => 0x33,
        0x2010014 => 0x36,
        0x2100014 => 0x39,
        0x301100c => 0x3c,
        0x1102404 | 0x80001c | 0x1080404 | 0x1800014 => {
            if *i == 0 || s.word(*i) == s.word(*i - 1) {
                return super::meta_default(s, i, out);
            }
            *i += 1;
            return;
        }
        _ => return super::meta_default(s, i, out),
    };
    out.push(id);
    *i += 1;
}
