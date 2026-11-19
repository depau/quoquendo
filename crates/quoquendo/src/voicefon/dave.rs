//! LTTS7Dave.dll (US English): `FonVidAllofonoVoce` 0x10001210 and
//! `FonVidMetaFonema` 0x10004cf0 (its default mapping 0x100035d0). The
//! words the code compares with are read from the library at the
//! addresses its code pushes (module `w`).

use super::{contains, eq, meta_default, Port, Rules, Stage};
use crate::lang::common::field;

pub(super) const DAVE: Port = Port {
    voice: "Dave",
    table: 0x1000c8d0,
    functions: &[
        ("GetVersion", 0x100010f0),
        ("ELQGetPhonemes", 0x10001000),
        ("FonVidAllofonoVoce", 0x10001210),
        ("FonVidAllofonoAnalisi", 0x10003640),
        ("FonVidMetaFonema", 0x10004cf0),
        ("FonVidIngloba", 0x10004e70),
        ("FonVidAnalisi2Sintesi", 0x10004de0),
    ],
    rules: Rules {
        allofono: Some(allofono),
        meta,
    },
};

// The library's phone codes this code names.
const E_STR: u32 = 0x20421; // `E
const E_: u32 = 0x20411; // E
const AE: u32 = 0x40411; // Ae
const AE_STR: u32 = 0x40421; // `Ae
const AA: u32 = 0x81091; // Aa:
const AA_STR: u32 = 0x810a1; // `Aa:
const AH: u32 = 0x21011; // Ah
const AH_STR: u32 = 0x21021; // `Ah
const OR: u32 = 0x20a91; // OR:
const OR_STR: u32 = 0x20aa1; // `OR:
const II: u32 = 0x2411; // i
const II_STR: u32 = 0x24a1; // `i:
const I_: u32 = 0x4411; // I
const I_STR: u32 = 0x4421; // `I
const OA: u32 = 0x121091; // Oa:
const OA_STR: u32 = 0x1210a1; // `Oa:
const UU_STR: u32 = 0x1030a1; // `u:
const U_: u32 = 0x105011; // U
const U_STR: u32 = 0x105021; // `U
const HEH: u32 = 0x10851; // HEh
const HAI: u32 = 0x2280893;
const HAI_STR: u32 = 0x22808a3;
const HAU_STR: u32 = 0x828808a3;
const HEI: u32 = 0x2220493;
const HEI_STR: u32 = 0x22204a3;
const HOU_STR: u32 = 0x829090a3;
const W: u32 = 0x81102404; // w
const J: u32 = 0x81080404; // j
const HW: u32 = 0x1102404;
const HJ: u32 = 0x1080404;
const HR: u32 = 0x1010404;
const HRR: u32 = 0x9010404; // HR
const HG: u32 = 0x1800014;
const HK: u32 = 0x80001c;
const P: u32 = 0x2002014;
const HUP: u32 = 0x22002014;
const T: u32 = 0x2010014;
const TH: u32 = 0x12010014;
const HUT: u32 = 0x22010014;
const K: u32 = 0x2100014;
const KH: u32 = 0x12100014;
const HUK: u32 = 0x22100014;
const HTH: u32 = 0x3010094;
const B: u32 = 0x3002014;
const D: u32 = 0x3010014;
const HUD: u32 = 0x23010014;
const G: u32 = 0x3100014;
const TS: u32 = 0x2011004;
const TSH: u32 = 0x2021004; // T$
const DZ: u32 = 0x301100c;
const DZH: u32 = 0x3021004; // Dg
const F: u32 = 0x2004104;
const V: u32 = 0x3004104;
const S: u32 = 0x10104;
const Z: u32 = 0x1010104;
const SH: u32 = 0x20104; // $
const ZH: u32 = 0x1020104; // Zg
const THF: u32 = 0x2008104; // Th
const DH: u32 = 0x3008104;
const H: u32 = 0x800104;
const M: u32 = 0x1002024;
const N: u32 = 0x1010024;
const HN: u32 = 0x9010024;
const NG: u32 = 0x1100024; // N
const L: u32 = 0x1010804;
const HL: u32 = 0x9010804;

/// The words (string addresses in LTTS7Dave.dll's .rdata).
mod w {
    pub const OR: u32 = 0x1000cbf4;
    pub const WHATEVER: u32 = 0x1000c99c;
    pub const INTO: u32 = 0x1000c994;
    pub const ONTO: u32 = 0x1000c98c;
    pub const TO: u32 = 0x1000cbe0;
    pub const LY: u32 = 0x1000c988;
    pub const ELY: u32 = 0x1000c984;
    pub const THEIR: u32 = 0x1000caa4;
    pub const THERE_D: u32 = 0x1000c9a8;
    pub const COULDNT: u32 = 0x1000ca18;
    pub const WOULDNT: u32 = 0x1000ca0c;
    pub const SHOULDNT: u32 = 0x1000ca00;
    pub const AND: u32 = 0x1000ca98;
    pub const IT: u32 = 0x1000c9f4;
    pub const ITS: u32 = 0x1000cbf0;
    pub const IT_S: u32 = 0x1000c9ec;
    pub const YOU: u32 = 0x1000c9e8;
    pub const YOUR: u32 = 0x1000cb00;
    pub const YOU_VE: u32 = 0x1000c9e0;
    pub const YOU_D: u32 = 0x1000c9d8;
    pub const YOU_LL: u32 = 0x1000c9d0;
    pub const YOURS: u32 = 0x1000caf0;
    pub const YER: u32 = 0x1000c9cc;
    pub const YEH: u32 = 0x1000c9c8;
    pub const YOURSELF: u32 = 0x1000c9bc;
    pub const YOURSELVES: u32 = 0x1000c9b0;
    pub const I: u32 = 0x1000c330;
    pub const HE: u32 = 0x1000ca94;
    pub const HIS: u32 = 0x1000ca8c;
    pub const HIM: u32 = 0x1000ca90;
    pub const HER: u32 = 0x1000cb68;
    pub const HERS: u32 = 0x1000ca6c;
    pub const HE_D: u32 = 0x1000ca84;
    /// `he`ll` (with a backquote) in the library's d + and list
    pub const HE_LL_BQ: u32 = 0x1000c9f8;
    pub const HE_LL: u32 = 0x1000ca7c;
    pub const HE_S: u32 = 0x1000ca74;
    pub const EITHER: u32 = 0x1000ca2c;
    pub const NEITHER: u32 = 0x1000ca24;
    pub const ALWAYS: u32 = 0x1000ca34;
    pub const MY: u32 = 0x1000ca5c;
    pub const I_LL: u32 = 0x1000ca54;
    pub const THEY_D: u32 = 0x1000ca4c;
    pub const THEY_VE: u32 = 0x1000ca44;
    pub const SAINT: u32 = 0x1000ca3c;
    pub const TZ: u32 = 0x1000ca60;
    pub const FROM: u32 = 0x1000cb28;
    pub const OURS: u32 = 0x1000caac;
    pub const ARENT: u32 = 0x1000cb74;
    pub const WITH: u32 = 0x1000cabc;
    pub const WITHOUT: u32 = 0x1000cab4;
    pub const MUST: u32 = 0x1000ca9c;
    pub const HAVE: u32 = 0x1000cb54;
    pub const HAS: u32 = 0x1000cb50;
    pub const HAD: u32 = 0x1000cb4c;
    pub const THERE: u32 = 0x1000ca64;
    pub const OFF: u32 = 0x1000cac4;
    pub const ON: u32 = 0x1000cad8;
    pub const SORRY: u32 = 0x1000cad0;
    pub const WASNT: u32 = 0x1000cac8;
    pub const DO: u32 = 0x1000cae4;
    pub const YOU_RE: u32 = 0x1000caf8;
    pub const COULD: u32 = 0x1000cadc;
    pub const SHOULD: u32 = 0x1000cae8;
    pub const MILLIARD: u32 = 0x1000cb1c;
    pub const MILIARD: u32 = 0x1000cb14;
    pub const SOMEWHAT: u32 = 0x1000cb08;
    pub const THAT: u32 = 0x1000cb60;
    pub const AT: u32 = 0x1000cb5c;
    pub const NO: u32 = 0x1000cb48;
    pub const UP: u32 = 0x1000cb44;
    pub const WAS: u32 = 0x1000cb40;
    pub const WHAT: u32 = 0x1000cb38;
    pub const SOME: u32 = 0x1000cb30;
    pub const DID: u32 = 0x1000cbd4;
    pub const REMOVE: u32 = 0x1000cbd8;
    pub const ME: u32 = 0x1000cbe4;
    pub const BECAME: u32 = 0x1000cbe8;
    pub const THEY_RE: u32 = 0x1000cbcc;
    pub const THEM: u32 = 0x1000cbc4;
    pub const THEMSELVES: u32 = 0x1000cbb8;
    pub const OUTLET: u32 = 0x1000cbb0;
    pub const OUTLETS: u32 = 0x1000cba8;
    pub const ION: u32 = 0x1000cba4;
    pub const SEPARATELY: u32 = 0x1000cb98;
    pub const FOR: u32 = 0x1000cb94;
    pub const MENT: u32 = 0x1000cb8c;
    pub const EMBARRASSMENT: u32 = 0x1000cb7c;
    pub const WERENT: u32 = 0x1000cb6c;
}

/// The words of a phrase the code looks at around the current phone.
struct Ctx {
    /// the phone's record index (element +0)
    w: u32,
    /// its text (`NULL`: none)
    t: Option<Vec<u8>>,
    /// the previous record's text (`""` for the chunk's first record)
    prev: Option<Vec<u8>>,
    /// the next record's text (`""` for the record at the chunk's count)
    next: Option<Vec<u8>>,
    /// record +0x12, the stress grade
    grade: u8,
    /// the record is the chunk's last word (`w == count - 2`)
    last: bool,
}

fn is(s: &Stage, t: &Option<Vec<u8>>, at: u32) -> bool {
    eq(t.as_deref(), s.pe.cstr(at))
}

fn any(s: &Stage, t: &Option<Vec<u8>>, at: &[u32]) -> bool {
    at.iter().any(|&a| is(s, t, a))
}

fn ends(s: &Stage, t: &Option<Vec<u8>>, at: u32) -> bool {
    t.as_deref().is_some_and(|t| t.ends_with(s.pe.cstr(at)))
}

/// The previous element's code, 0 before the list (`uVar < 0x400` guards).
fn prev_code(s: &Stage, i: usize) -> u32 {
    if i == 0 { 0 } else { s.code(i - 1) }
}

/// `ELQListDeleteElement` on the current element, then `*i -= 1`
/// (LAB_1000327e / LAB_10003286).
fn drop_current(s: &mut Stage, ip: &mut usize, i: usize) {
    s.delete(i);
    *ip = i.wrapping_sub(1);
}

/// have / has / had, he / him / his / her… (the h-dropping words).
const HAVE_HE: &[u32] = &[w::HAVE, w::HAS, w::HAD, w::HE, w::HIM, w::HIS, w::HER, w::HE_D, w::HE_LL, w::HE_S, w::HERS];

/// 0x10001210.
fn allofono(s: &mut Stage, ip: &mut usize) {
    let i = *ip;
    let w = s.word(i);
    let wi = w as i64;
    let count = s.count() as i64;
    // ctx+0x18 ('I' when 7..=10, else 'D'): the module's stale token count
    // from tokenization, below 7 in ordinary sentences; taken as 'D'
    let question = false;
    let c = Ctx {
        w,
        t: s.text(wi).map(<[u8]>::to_vec),
        prev: if w == 0 { Some(Vec::new()) } else { s.text(wi - 1).map(<[u8]>::to_vec) },
        next: if wi == count { Some(Vec::new()) } else { s.text(wi + 1).map(<[u8]>::to_vec) },
        grade: s.rec(wi).map_or(0, |r| r.f[0x12]),
        last: wi == count - 2,
    };
    // record +0x26: the next record's class (+0xe), '*' a verb
    let next_class = s.rec(wi + 1).map_or(0, |r| r.f[field::CLASS]);

    // 0x10001210..0x10001555: "or" after a word ending in a front (Hj) or
    // back (Hw) vowel/diphthong gets the glide before it
    if i != 0 && s.word(i - 1) != w && is(s, &c.t, w::OR) {
        let pc = s.code(i - 1);
        if [II, II_STR, I_, I_STR, HAI, HAI_STR, HEI, HEI_STR, 0x2321093, 0x23210a3].contains(&pc) {
            s.insert(i, w, HJ, 0);
        } else if [0x103011, UU_STR, U_, U_STR, 0x82880893, HAU_STR, 0x82909093, HOU_STR]
            .contains(&pc)
        {
            s.insert(i, w, HW, 0);
        }
    }

    let code = s.code(i);
    match code {
        TH => th(s, i, &c),
        HUT => {
            if i == 0 || w == s.word(i + 1) || s.code(i - 1) != S {
                return;
            }
            if [DH, THF, TH, D].contains(&s.code(i + 1)) {
                drop_current(s, ip, i);
            }
        }
        HUD => {
            // compares with 0x4e and 0x2d (table ids, never codes)
            if i == 0 || s.code(i - 1) != 0x4e || s.word(i) == s.word(i + 1) {
                return;
            }
            if is(s, &c.next, w::TO) && s.code(i + 1) == 0x2d {
                drop_current(s, ip, i);
            }
        }
        HL => {
            let pc = prev_code(s, i);
            if i != 0 && (pc == HN || pc == HRR) {
                s.set(i, HEH);
                s.insert(i + 1, s.word(i), L, 0);
                return;
            }
            if i < 2 || s.code(i - 2) != HEH || (pc != N && pc != HR) {
                return;
            }
            s.set(i, HEH);
            s.insert(i + 1, s.word(i), L, 0);
        }
        DH => {
            if c.t.is_none() {
                return;
            }
            let their = is(s, &c.t, w::THEIR);
            if their && c.grade == 3 && s.code(i + 1) == HRR {
                s.insert(i + 1, s.word(i), E_STR, 0);
                return;
            }
            let g02 = c.grade == 0 || c.grade == 2;
            if their && g02 && s.code(i + 1) == E_STR && s.code(i + 2) == HR {
                s.set(i + 2, HRR);
                return;
            }
            let thered = is(s, &c.t, w::THERE_D);
            if thered && g02 && s.code(i + 1) == E_STR && s.code(i + 2) == HR {
                s.set(i + 2, HRR);
                return;
            }
            if thered && c.grade == 3 && s.code(i + 1) == E_ && s.code(i + 2) == HR {
                s.set(i + 1, E_STR);
            }
        }
        DZ => {
            s.set(i, HUD);
            s.insert(i + 1, s.word(i), Z, 0);
        }
        D => d(s, ip, i, &c),
        HAI_STR => {
            if any(s, &c.t, &[w::EITHER, w::NEITHER]) {
                s.set(i, II_STR);
            }
        }
        HEI => {
            if is(s, &c.t, w::ALWAYS) {
                s.set(i, I_);
            }
        }
        HEI_STR => {
            if any(s, &c.t, &[w::THEY_D, w::THEY_VE]) && c.grade == 2 {
                s.set(i, HEI);
            } else if is(s, &c.t, w::SAINT) {
                s.set(i, HEH);
            }
        }
        HAI => {
            if is(s, &c.t, w::MY) && c.grade == 3 {
                s.set(i, HAI_STR);
            } else if is(s, &c.t, w::I_LL) {
                s.set(i, AA);
                s.set(i + 1, L);
                *ip = i + 1;
            }
        }
        TS => {
            if s.word(i) != s.word(i + 1) {
                if s.code(i + 1) == SH {
                    s.set(i, TSH);
                }
                return;
            }
            if c.t.as_deref().is_some_and(|t| contains(Some(t), s.pe.cstr(w::TZ))) {
                return;
            }
            if i == 0 || s.word(i - 1) != s.word(i) {
                return;
            }
            s.set(i, HUT);
            s.insert(i + 1, s.word(i), S, 0);
        }
        H => h(s, ip, i, &c),
        HR => {
            if s.code(i + 1) == HEH && is(s, &c.t, w::FROM) {
                s.delete(i);
                s.set(i, HRR);
                return;
            }
            if i != 0 && is(s, &c.t, w::OURS) {
                s.set(i - 1, HAU_STR);
                s.set(i, HRR);
                return;
            }
            if is(s, &c.t, w::ARENT) {
                s.insert(i + 1, s.word(i), HEH, 0);
                return;
            }
            if is(s, &c.t, w::THEIR) {
                s.set(i.wrapping_sub(1), E_STR);
                s.set(i, HRR);
            }
        }
        THF => {
            if c.t.is_some() {
                if is(s, &c.t, w::WITH) {
                    let nc = s.code(i + 1);
                    if nc & 1 != 0 || nc == B || nc == D {
                        s.set(i, DH);
                        return;
                    }
                }
                if is(s, &c.t, w::WITHOUT) {
                    s.set(i, DH);
                    return;
                }
                if is(s, &c.t, w::WITH) && s.code(i + 1) == DH {
                    drop_current(s, ip, i);
                    return;
                }
            }
            // LAB_1000227c
            if i == 0 || s.code(i + 1) != DH || is(s, &c.prev, w::WITH) {
                return;
            }
            s.set(i, HUT);
        }
        OA_STR => {
            if is(s, &c.t, w::OFF) && c.last {
                s.set(i, OA);
            }
        }
        AA_STR => {
            if is(s, &c.t, w::ON) && c.last {
                s.set(i, AA);
            } else if is(s, &c.t, w::SORRY) {
                s.set(i, OA_STR);
            } else if is(s, &c.t, w::WASNT) {
                s.set(i, AH_STR);
            }
        }
        U_ => {
            if c.t.is_none() {
                return;
            }
            if is(s, &c.t, w::DO) {
                let nc = s.code(i + 1);
                if nc & 4 != 0 && nc != J && nc != W {
                    s.set(i, HEH);
                    return;
                }
            }
            if (is(s, &c.t, w::YOUR) && c.grade == 2) || (is(s, &c.t, w::YOU_RE) && c.grade == 3) {
                s.set(i, OR);
                s.delete(i + 1);
                return;
            }
            if is(s, &c.t, w::COULD) && c.grade == 2 {
                s.set(i, U_STR);
            }
        }
        U_STR => {
            if c.t.is_none() {
                return;
            }
            if any(s, &c.t, &[w::YOUR, w::YOU_RE]) && c.grade == 0 {
                s.set(i, OR_STR);
                s.delete(i + 1);
                return;
            }
            let yours = is(s, &c.t, w::YOURS);
            if yours && c.grade == 3 && !c.last {
                drop_current(s, ip, i);
            } else if yours && (c.grade == 3 || c.grade == 0 || c.grade == 2) {
                // grade 3 here only at the last word (LAB_10001839)
                s.set(i, OA_STR);
            } else if is(s, &c.t, w::SHOULD) && c.grade == 3 {
                s.set(i, U_);
            }
        }
        AA => {
            if c.t.is_none() {
                return;
            }
            if any(s, &c.t, &[w::MILLIARD, w::MILIARD]) && s.code(i + 1) == HR {
                s.set(i, HRR);
                s.delete(i + 1);
            }
            if is(s, &c.t, w::SOMEWHAT) {
                s.set(i, AH);
            }
        }
        AH => {
            if is(s, &c.t, w::FROM) && c.grade == 2 {
                s.set(i, HEH);
            }
        }
        AH_STR => {
            if c.t.is_none() {
                return;
            }
            if any(s, &c.t, &[w::UP, w::WAS]) && c.last {
                s.set(i, AH);
            } else if is(s, &c.t, w::WHAT) && c.grade == 2 {
                s.set(i, AH);
            } else if is(s, &c.t, w::SOME) && s.word(i) == 0 {
                s.set(i, HEH);
            }
        }
        AE => {
            if c.t.is_none() {
                return;
            }
            if is(s, &c.t, w::THAT) && c.grade == 2 {
                s.set(i, AE_STR);
                return;
            }
            if !(is(s, &c.t, w::AT) && s.word(i) == 0) {
                if is(s, &c.t, w::HAVE) && w == 0 && question {
                    return;
                }
                if !any(s, &c.t, &[w::HAVE, w::HAS, w::HAD]) || next_class == b'*' {
                    return;
                }
                if any(s, &c.next, &[w::TO, w::NO]) {
                    return;
                }
            }
            s.set(i, HEH);
        }
        OR_STR => {
            if is(s, &c.t, w::HER) && c.last {
                s.set(i, HRR);
            }
        }
        I_STR => {
            if is(s, &c.t, w::DID) && c.last {
                s.set(i, I_);
            }
        }
        II => {
            if c.t.as_deref().is_some_and(|t| t.starts_with(s.pe.cstr(w::REMOVE))) {
                s.set(i, I_);
            }
        }
        II_STR => {
            if is(s, &c.t, w::ME) && !is(s, &c.prev, w::TO) && c.last {
                s.set(i, II);
            }
        }
        I_ => {
            if is(s, &c.t, w::ITS) && c.grade == 3 {
                s.set(i, I_STR);
            } else if is(s, &c.t, w::BECAME) {
                s.set(i, II);
            }
        }
        E_STR => {
            if is(s, &c.t, w::THEY_RE) && !c.last {
                s.set(i, E_);
                s.set(i + 1, HR);
            }
        }
        E_ => {
            if c.t.is_none() {
                return;
            }
            if is(s, &c.t, w::THEM) && c.grade == 2 {
                s.set(i, E_STR);
            } else if is(s, &c.t, w::THEMSELVES) && i != 0 && s.code(i - 1) == DH {
                s.set(i, HEH);
            } else if any(s, &c.t, &[w::OUTLET, w::OUTLETS]) && !c.last {
                s.set(i, I_STR);
            }
        }
        HEH => schwa(s, ip, i, &c),
        _ => {}
    }
}

/// 0x10001210, case th (0x12010014): flapped (HTh) in whatever/into/onto,
/// before h after a vowel, after r before a stressed vowel, in `to` after
/// a vowel or voiced non-nasal; -ly adverbs' t inside a word after a vowel,
/// n or r unreleased (Hut).
fn th(s: &mut Stage, i: usize, c: &Ctx) {
    if any(s, &c.t, &[w::WHATEVER, w::INTO, w::ONTO]) {
        s.set(i, HTH);
        return;
    }
    if i == 0 {
        return;
    }
    let pc = s.code(i - 1);
    let w = s.word(i);
    if pc & 1 != 0 && w != s.word(i + 1) && s.code(i + 1) == H {
        s.set(i, HTH);
        return;
    }
    if pc == HR && w != s.word(i + 1) && s.code(i + 1) & 0x21 == 0x21 {
        s.set(i, HTH);
        return;
    }
    if is(s, &c.t, w::TO)
        && s.word(i - 1) != w
        && ![M, N, HL, HUP, HUT, HUK, TSH, SH, F, S, Z, THF, TS, DZH].contains(&pc)
    {
        s.set(i, HTH);
        return;
    }
    if pc & 1 == 0 && pc != N && pc != HR {
        return;
    }
    if s.word(i - 1) != w {
        return;
    }
    if !ends(s, &c.t, w::LY) && !ends(s, &c.t, w::ELY) {
        return;
    }
    if s.word(i + 2) != s.word(i + 3) {
        s.set(i, HUT);
    }
}

/// 0x10001210, case d (0x3010014).
fn d(s: &mut Stage, ip: &mut usize, i: usize, c: &Ctx) {
    let w = s.word(i);
    let pc = prev_code(s, i);
    if s.code(i + 1) == HEH && s.code(i + 2) == N {
        if i != 0 {
            // couldn't wouldn't shouldn't: d + HEh + n as Hud + Hn
            if s.word(i - 1) == s.word(i + 2)
                && pc & 0x21 == 0x21
                && any(s, &c.t, &[w::COULDNT, w::WOULDNT, w::SHOULDNT])
            {
                s.set(i, HUD);
                let j = i + 1;
                *ip = j;
                s.delete(j);
                s.set(j, HN);
                if j < s.list.len() {
                    s.list[j].word = s.word(j - 1);
                }
                return;
            }
            if and_after_n(s, ip, i, w, pc, c) {
                return;
            }
        }
    } else if and_after_n(s, ip, i, w, pc, c) {
        return;
    }
    if is(s, &c.t, w::AND) && c.next.is_some() {
        if any(s, &c.next, &[w::I, w::HE, w::HIS, w::HIM, w::HER, w::HERS, w::HE_D, w::HE_LL_BQ, w::HE_S]) {
            drop_current(s, ip, i);
            return;
        }
        if any(s, &c.next, &[w::IT, w::ITS, w::IT_S]) {
            s.set(i, HUD);
            return;
        }
    }
    // LAB_10002d33
    if i == 0 || pc != Z || w == s.word(i + 1) || s.code(i + 1) & 4 == 0 {
        return;
    }
    if any(
        s,
        &c.next,
        &[w::YOU, w::YOUR, w::YOU_VE, w::YOU_D, w::YOU_LL, w::YOURS, w::YER, w::YEH, w::YOURSELF, w::YOURSELVES],
    ) {
        s.set(i, HUD);
    }
}

/// LAB_10002b96: a word-final d after n before `and` is dropped.
fn and_after_n(s: &mut Stage, ip: &mut usize, i: usize, w: u32, pc: u32, c: &Ctx) -> bool {
    if i != 0 && w != s.word(i + 1) && pc == N && is(s, &c.next, w::AND) {
        drop_current(s, ip, i);
        return true;
    }
    false
}

/// 0x10001210, case h (0x800104): dropped in have/has/had after `must`
/// (not at the sentence's end) and after `there`, in have…/he… after
/// `and` and after p v n N m Ts `HOU `u:.
fn h(s: &mut Stage, ip: &mut usize, i: usize, c: &Ctx) {
    if i == 0 {
        return;
    }
    let hh = &[w::HAVE, w::HAS, w::HAD];
    if is(s, &c.prev, w::MUST)
        && s.word(i - 1) != s.word(i)
        && any(s, &c.t, hh)
        && c.w as i64 != s.sentence_len() as i64 - 2
    {
        drop_current(s, ip, i);
        return;
    }
    if is(s, &c.prev, w::AND) && any(s, &c.t, HAVE_HE) {
        drop_current(s, ip, i);
        return;
    }
    if is(s, &c.prev, w::THERE) && any(s, &c.t, hh) {
        drop_current(s, ip, i);
        return;
    }
    // LAB_1000263a
    if ![HUP, V, N, NG, M, TS, HOU_STR, UU_STR].contains(&s.code(i - 1)) {
        return;
    }
    if any(s, &c.t, HAVE_HE) {
        drop_current(s, ip, i);
    }
}

/// 0x10001210, case HEh (0x10851): -ion's schwa + n as Hn, `separately`'s
/// schwa + r as HR, for/or unstressed Oa:, -ment's schwa I, aren't/weren't
/// lose it.
fn schwa(s: &mut Stage, ip: &mut usize, i: usize, c: &Ctx) {
    if s.code(i + 1) == N && c.t.as_ref().is_some_and(|t| t.len() > 2) && ends(s, &c.t, w::ION) {
        if s.word(i + 1) != s.word(i + 2) && ![KH, G, 0x28, 0x18].contains(&s.code(i + 2)) {
            s.delete(i);
            s.set(i, HN);
            return;
        }
    }
    if s.code(i + 1) == N && c.t.as_deref().is_some_and(|t| contains(Some(t), s.pe.cstr(w::ION))) {
        if s.word(i + 1) == s.word(i + 2) && i != 0 && [SH, ZH, DZH, TSH].contains(&s.code(i - 1)) && !c.last {
            s.delete(i);
            s.set(i, HN);
            return;
        }
    }
    if s.code(i + 1) == HR {
        if c.t.is_some() {
            if is(s, &c.t, w::SEPARATELY) {
                s.delete(i);
                s.set(i, HRR);
                return;
            }
            for_or(s, i, c);
        }
    } else if c.t.is_some() {
        for_or(s, i, c);
    }
    // LAB_10001ab2
    if c.t.as_ref().is_some_and(|t| t.len() > 3) && ends(s, &c.t, w::MENT) && s.word(i + 2) != s.word(i + 3) {
        if !is(s, &c.t, w::EMBARRASSMENT) {
            s.set(i, I_);
            return;
        }
    }
    if any(s, &c.t, &[w::ARENT, w::WERENT]) {
        drop_current(s, ip, i);
    }
}

/// LAB_10001a6b: stressless (grade 3) for/or with Oa:.
fn for_or(s: &mut Stage, i: usize, c: &Ctx) {
    if any(s, &c.t, &[w::FOR, w::OR]) && c.grade == 3 {
        s.set(i, OA);
    }
}

/// 0x10004cf0: p t k (unaspirated) said b d g, Dz as Ts, Hud as Hut (ids
/// 0x33 0x36 0x39 0x3c 0x2e); a word-initial glide or glottal onset (Hw Hj
/// Hg Hk, not the list's first phone) is not said; else 0x100035d0, the
/// default.
fn meta(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    let id = match s.code(*i) {
        P => 0x33,
        T => 0x36,
        K => 0x39,
        DZ => 0x3c,
        HUD => 0x2e,
        HW | HK | HJ | HG => {
            if *i == 0 || s.word(*i) == s.word(*i - 1) {
                return meta_default(s, i, out);
            }
            *i += 1;
            return;
        }
        _ => return meta_default(s, i, out),
    };
    out.push(id);
    *i += 1;
}
