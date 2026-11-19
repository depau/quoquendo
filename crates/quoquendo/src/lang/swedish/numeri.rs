//! Swedish number reading: `LesIniNumeri` fills slots 0–7 and 9; slot 9
//! (0x10006ea0) is the reader the engine calls for a NUMBER token
//! (MECHANICS.md §4h "Numbers").
//!
//! Slot 9 classifies the token with nine recognizers (0x10005bd0 sets their
//! order, the reading mode's own one first), each filling the module's
//! number object (`NumObj`, 0xd4 bytes), and renders it (0x10006990). Every
//! table is read from the DLL.

use super::Swedish;
use crate::lang::common::Tok;
use crate::lang::elqnum::{self as q, Date, NumObj, NumSlots};

/// Slot 0's `{ id, char * }` pairs (0x53 of them); `""` (0x10016505) for
/// an id it lacks.
const WORDS: u32 = 0x10017298;
const EMPTY: u32 = 0x10016505;
/// 0x10003060's area codes (0xf7 sorted strings), searched by
/// `ELQSearchString` flag 2 (an entry that starts the digits).
const AREA_CODES: u32 = 0x10016e18;
/// Slot 5's currencies: 8 × `{ code, unit, units, cent, cents }`.
const CURRENCIES: u32 = 0x100171f8;
/// `hundra`, appended by 0x100063a0 after the first two digits of a year.
const HUNDRA: u32 = 0x10016624;
/// `till ` between the two years of a range (0x100068a0).
const TILL: u32 = 0x100179c8;
/// 0x10005110's separators `.,:`, ` och` after the hour and `och` before
/// the seconds.
const TIME_SEPS: u32 = 0x100179c4;
const OCH_AFTER_HOUR: u32 = 0x100179bc;
const OCH: u32 = 0x10017950;
/// The separators the phone and integer recognizers refuse: `:/-+`.
const NUM_SEPS: u32 = 0x100179b4;
/// 0x10003230's dash (`-`), the character it splits a year range at and
/// the token it looks for between two years.
const DASH: u32 = 0x100177a8;
/// 0x10002620's ordinals of 1–20 in the order it compares them:
/// (number, ordinal).
const ORDINALS: [(u32, u32); 20] = [
    (0x1001779c, 0x10017794),
    (0x10017790, 0x10017788),
    (0x10017784, 0x1001777c),
    (0x10017778, 0x10017770),
    (0x1001776c, 0x10017764),
    (0x10017760, 0x10017758),
    (0x10017754, 0x1001774c),
    (0x10017748, 0x10017740),
    (0x1001773c, 0x10017734),
    (0x10017730, 0x10017728),
    (0x10017724, 0x1001771c),
    (0x10017718, 0x10017710),
    (0x1001770c, 0x10017700),
    (0x100176fc, 0x100176f0),
    (0x100176ec, 0x100176e0),
    (0x100176dc, 0x100176d0),
    (0x100176cc, 0x100176c0),
    (0x100176bc, 0x100176b4),
    (0x100176b0, 0x100176a4),
    (0x100176a0, 0x10017694),
];
/// `0` appended to a prefix of the digits and `nde` after its reading
/// (0x10002620), the space after each part read.
const ORD_ZERO: u32 = 0x100177a4;
const ORD_NDE: u32 = 0x100177a0;
const ORD_SPACE: u32 = 0x1001669c;
/// 0x10002010's fractions in its order (0x10002330 tests the same list):
/// (fraction, reading).
const FRACTIONS: [(u32, u32); 14] = [
    (0x10017690, 0x10017688),
    (0x10017684, 0x10017674),
    (0x10017670, 0x10017660),
    (0x1001765c, 0x10017650),
    (0x1001764c, 0x1001763c),
    (0x10017638, 0x10017628),
    (0x10017624, 0x10017618),
    (0x10017614, 0x10017608),
    (0x10017600, 0x100175f4),
    (0x100175ec, 0x100175dc),
    (0x100175d4, 0x100175c4),
    (0x100175bc, 0x100175b0),
    (0x100175ac, 0x1001759c),
    (0x10017598, 0x10017588),
];
/// The ordinal recognizer's strings: `°`, `º`, `c` (after a degree sign:
/// Celsius, not an ordinal), `a`/`e` after `:` (`2:a`), and `-/,`.
const DEGREE: u32 = 0x100179b0;
const DEGREE_O: u32 = 0x100179ac;
const CELSIUS: u32 = 0x100179a8;
const COLON_A: u32 = 0x10017860;
const COLON_E: u32 = 0x10017848;
const ORD_REFUSE: u32 = 0x100179a4;
/// The date order (`b`) a VoiceXML date must have in mode 8.
const VXML_ORDER: u32 = 0x100179a0;

/// The module's number object (0xd4 bytes, `NumObj`'s layout).
#[derive(Clone)]
struct Obj(NumObj);

impl Obj {
    fn u32(&self, o: usize) -> u32 {
        self.0.u32(o)
    }
    fn set_u32(&mut self, o: usize, v: u32) {
        self.0.set_u32(o, v);
    }
    fn b(&self, o: usize) -> u8 {
        self.0.u8(o)
    }
    fn set_b(&mut self, o: usize, v: u8) {
        self.0.set_u8(o, v);
    }
    fn cstr(&self, o: usize) -> Vec<u8> {
        self.0.cstr(o).to_vec()
    }
    fn set_cstr(&mut self, o: usize, s: &[u8]) {
        self.0.set_cstr(o, cs(s));
    }
    /// Back to a plain number of `text` (`ELQNumSetDefaultType`).
    fn reset(&mut self, text: &[u8]) {
        q::set_default_type(&mut self.0, text);
    }
    /// The recognizers' sign handling: the text without a leading `-`/`+`;
    /// returns that sign.
    fn strip_sign(&mut self, local: &[u8]) -> u8 {
        match local.first() {
            Some(&c @ (b'-' | b'+')) => {
                self.set_cstr(4, &local[1..]);
                c
            }
            _ => {
                self.set_cstr(4, local);
                0
            }
        }
    }
}

fn cs(s: &[u8]) -> &[u8] {
    &s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())]
}

fn at(s: &[u8], k: usize) -> u8 {
    s.get(k).copied().unwrap_or(0)
}

/// `ELQstricmp(a, b) == 0` (ASCII case folding only).
fn ieq(a: &[u8], b: &[u8]) -> bool {
    let f = |c: u8| c.to_ascii_lowercase();
    a.len() == b.len() && a.iter().zip(b).all(|(&x, &y)| f(x) == f(y))
}

/// `strncmp(a, b, n)` on C strings.
fn strncmp(a: &[u8], b: &[u8], n: usize) -> std::cmp::Ordering {
    for k in 0..n {
        let (x, y) = (at(a, k), at(b, k));
        if x != y {
            return x.cmp(&y);
        }
        if x == 0 {
            break;
        }
    }
    std::cmp::Ordering::Equal
}

/// Counts of `.` and `,`.
fn dots_commas(s: &[u8]) -> (usize, usize) {
    (s.iter().filter(|&&c| c == b'.').count(), s.iter().filter(|&&c| c == b',').count())
}

/// 0x10003200: four digits.
fn four_digits(s: &[u8]) -> bool {
    s.len() == 4 && s.iter().all(u8::is_ascii_digit)
}

fn ltoa(v: u32) -> Vec<u8> {
    v.to_string().into_bytes()
}

fn tok_text(toks: &[Tok], k: usize) -> &[u8] {
    toks.get(k).map(|t| &t.text[..]).unwrap_or(b"")
}

/// A token's type; past the buffer's end the engine's end type 7.
fn tok_type(toks: &[Tok], k: usize) -> u32 {
    toks.get(k).map(|t| t.kind).unwrap_or(7)
}

/// 0x10001f20 (count 0xf): the index of the token `off` tokens from `idx`,
/// tag tokens (type 5) not counted; forward it stops at the end (type 7) or
/// at index 31.
fn tok_at(toks: &[Tok], idx: usize, off: i32) -> usize {
    let mut e = idx;
    let mut off = off;
    if off < 0 {
        while e > 0 {
            e -= 1;
            if tok_type(toks, e) != 5 {
                off += 1;
            }
            if off >= 0 {
                break;
            }
        }
    } else if off > 0 {
        let mut n = idx + 1;
        while n < 31 && tok_type(toks, e) != 7 {
            e += 1;
            n += 1;
            if tok_type(toks, e) != 5 {
                off -= 1;
            }
            if off <= 0 {
                break;
            }
        }
    }
    e
}

impl Swedish {
    fn word(&self, id: u32) -> &[u8] {
        (0..0x53).find(|&k| self.pe.u32(WORDS + 8 * k) == id).map(|k| self.s(self.pe.u32(WORDS + 8 * k + 4))).unwrap_or_else(|| self.s(EMPTY))
    }

    /// 0x100056c0: the word of 1–20 and the tens 30–90, else `""`.
    fn n_word(&self, n: i32) -> &[u8] {
        let id = match n {
            1..=20 => [0x11, 0x15, 0x17, 0x19, 0x1b, 0x1c, 0x1d, 0x1f, 0x20, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c][n as usize - 1],
            30 => 0x2d,
            40 => 0x34,
            50 => 0x35,
            60 => 0x36,
            70 => 0x37,
            80 => 0x38,
            90 => 0x39,
            _ => return self.s(EMPTY),
        };
        self.word(id)
    }

    /// 0x10005af0: the two digits at `p` (`tjugofem`), `""` for 00.
    fn two(&self, p: &[u8]) -> Vec<u8> {
        let pair = [at(p, 0), at(p, 1)];
        let n = q::atol(&pair);
        let mut b = Vec::new();
        if n == 0 {
            return b;
        }
        if pair[0] != b'1' && n >= 10 {
            b.extend_from_slice(self.n_word((pair[0] as i8 as i32 - 0x30) * 10));
            b.extend_from_slice(self.n_word(pair[1] as i8 as i32 - 0x30));
        } else {
            b.extend_from_slice(self.n_word(n));
        }
        b
    }

    /// 0x100058e0: the hundreds digit at `p` appended to `s`; false for 0.
    fn hundreds(&self, s: &mut Vec<u8>, p: &[u8]) -> bool {
        let d = at(p, 0);
        match d {
            b'0' => return false,
            b'1' => s.extend_from_slice(self.word(if at(p, 1) == b'0' && at(p, 2) == b'0' { 0x3a } else { 0x3b })),
            b'2' => s.extend_from_slice(self.word(0x15)),
            b'3' => s.extend_from_slice(self.word(0x17)),
            b'5' => s.extend_from_slice(self.word(0x1b)),
            b'7' => s.extend_from_slice(self.word(0x1d)),
            b'9' => s.extend_from_slice(self.word(0x20)),
            _ => s.extend_from_slice(self.n_word(d as i8 as i32 - 0x30)),
        }
        if d != b'1' {
            s.extend_from_slice(self.word(0x3b));
        }
        true
    }

    /// 0x100063a0: a number read as a year: 1–99 by 0x10005af0, up to 1999
    /// as hundreds (`nittonhundraåttiosju`), else by slot 2.
    fn year_reading(&self, n: u32) -> Vec<u8> {
        let mut out = Vec::new();
        if n == 0 {
            return out;
        }
        let t = ltoa(n);
        if n - 1 < 99 {
            out.extend(self.two(&t));
        } else if n < 2000 {
            out.extend_from_slice(self.n_word(q::atol(&t[..2])));
            out.extend_from_slice(self.s(HUNDRA));
            out.extend(self.two(&t[2..]));
        } else {
            out.extend(self.cardinal_c(&t));
        }
        out
    }

    /// Slot 2 (0x10005cf0): the cardinal of a digit string, position by
    /// position from the hundreds of billions, a trailing word 0 removed;
    /// 1100–1999 as hundreds.
    fn cardinal_c(&self, digits: &[u8]) -> Vec<u8> {
        let digits = cs(digits);
        let len = digits.len();
        let d = |k: usize| at(digits, k);
        let rest = |k: usize| digits.get(k..).unwrap_or(b"");
        let mut s: Vec<u8> = Vec::new();
        // local_71, local_72, local_6b, local_69 (millions "01"), local_6a
        // (thousands "01")
        let (mut f10, mut f11, mut f12, mut f13, mut f1b) = (false, true, false, false, false);
        let mut p = 0;
        let w0 = self.word(0);
        let w1 = self.word(1);
        let sep_unless_space = |s: &mut Vec<u8>| {
            if s.last() != Some(&b' ') {
                q::add_separator(s, 0, w1);
            }
        };
        if len == 4 && d(0) == b'1' && d(1) != b'0' {
            s.extend(self.year_reading(q::atol(digits) as u32));
            return s;
        }
        let mut thousands = len == 4;
        if len != 4 {
            if len >= 12 {
                self.hundreds(&mut s, rest(p));
                p += 1;
            }
            if len >= 11 {
                f10 = true;
                s.extend(self.two(rest(p)));
                p += 1;
            }
            if len >= 10 {
                let al = d(p);
                if al == b'1' && !f10 {
                    s.extend_from_slice(self.word(0x4d));
                } else {
                    if !f10 {
                        s.extend_from_slice(self.n_word(al as i8 as i32 - 0x30));
                    }
                    sep_unless_space(&mut s);
                    s.extend_from_slice(self.word(0x4e));
                }
                q::add_separator(&mut s, 0, w0);
                p += 1;
                f10 = false;
                f11 = true;
            }
            if len >= 9 {
                f11 = !self.hundreds(&mut s, rest(p));
                p += 1;
            }
            if len >= 8 {
                f10 = true;
                if f11 && d(p) == b'0' && d(p + 1) == b'1' {
                    f13 = true;
                } else {
                    s.extend(self.two(rest(p)));
                }
                if d(p) != b'0' {
                    f11 = false;
                }
                p += 1;
            }
            if len >= 7 {
                let al = d(p);
                if !(al == b'0' && f11) {
                    if f13 || (al == b'1' && !f10) {
                        s.extend_from_slice(self.word(0x4a));
                    } else {
                        if al != b'1' && !f10 {
                            s.extend_from_slice(self.n_word(al as i8 as i32 - 0x30));
                        }
                        sep_unless_space(&mut s);
                        s.extend_from_slice(self.word(0x4b));
                    }
                    q::add_separator(&mut s, 0, w0);
                }
                p += 1;
                f10 = false;
                f11 = true;
            }
            if len >= 6 {
                f12 = true;
                f11 = !self.hundreds(&mut s, rest(p));
                p += 1;
            }
            if len >= 5 {
                f10 = true;
                if f11 && d(p) == b'0' && d(p + 1) == b'1' {
                    f1b = true;
                } else {
                    s.extend(self.two(rest(p)));
                }
                if d(p) != b'0' {
                    f11 = false;
                }
                p += 1;
            }
            thousands = len >= 4;
        }
        if thousands {
            let al = d(p);
            if !(al == b'0' && f11) {
                if al == b'1' && !f10 && !f12 {
                    s.extend_from_slice(self.word(0x47));
                } else {
                    if !f10 {
                        s.extend_from_slice(self.n_word(al as i8 as i32 - 0x30));
                    }
                    s.extend_from_slice(self.word(if f1b { 0x47 } else { 0x48 }));
                    if f12 && !f1b {
                        q::add_separator(&mut s, 0, w0);
                    }
                }
            }
            p += 1;
        }
        if len >= 3 {
            self.hundreds(&mut s, rest(p));
            p += 1;
        }
        if len >= 2 {
            if d(p) == b'0' && d(p + 1) == b'1' {
                s.extend_from_slice(self.word(0x10));
            } else {
                s.extend(self.two(rest(p)));
            }
        }
        if !w0.is_empty() && s.ends_with(w0) {
            s.truncate(s.len() - w0.len());
        }
        s
    }

    /// `ELQSearchString` flag 2 over `count` sorted strings: a binary
    /// search for the entry the key starts with.
    fn search_prefix(&self, key: &[u8], table: u32, count: u32) -> Option<u32> {
        let entry = |k: u32| self.s(self.pe.u32(table + 4 * k));
        let (mut lo, mut hi) = (0i64, count as i64 - 1);
        while lo <= hi {
            let mid = (lo + hi) / 2;
            let e = entry(mid as u32);
            match strncmp(key, e, e.len()) {
                std::cmp::Ordering::Less => hi = mid - 1,
                std::cmp::Ordering::Greater => lo = mid + 1,
                std::cmp::Ordering::Equal => return Some(mid as u32),
            }
        }
        None
    }

    /// 0x10003060: the length of the area code the digits start with, 0
    /// when none.
    fn area_code(&self, digits: &[u8]) -> usize {
        match self.search_prefix(cs(digits), AREA_CODES, 0xf7) {
            Some(k) => self.s(self.pe.u32(AREA_CODES + 4 * k)).len(),
            None => 0,
        }
    }

    /// Slot 7 (0x10003190): `ELQNumExtractDate` in the order `dmy`, then
    /// its check with two-digit years from 80 (from 0 after `/`).
    fn slot7(&self, dmy: &[u8], text: &[u8], o: &mut Obj) -> bool {
        let mut d = Date::default();
        let mut ok = q::extract_date(dmy, text, &mut d);
        if ok {
            let min = if d.sep == b'/' { 0 } else { 0x50 };
            ok = q::validate_date(dmy, &mut d, min);
        }
        o.set_u32(0x98, d.day);
        o.set_u32(0x9c, d.month);
        o.set_u32(0xa0, d.year);
        ok
    }

    /// 0x10003230: a range of years, `1990-2000` or `1990 - 2000` (the
    /// latter uses the two tokens after it), into +0xa4/+0xa8.
    fn year_range(&self, c18: &mut usize, toks: &[Tok], i: usize, text: &[u8], o: &mut Obj) -> bool {
        let n1 = tok_text(toks, tok_at(toks, i, 1)).to_vec();
        let n2i = tok_at(toks, i, 2);
        let n2 = tok_text(toks, n2i).to_vec();
        o.set_u32(0xa8, 0);
        o.set_u32(0xa4, 0);
        let local = &cs(text)[..cs(text).len().min(0x3f)];
        let dash = self.s(DASH);
        if let Some(p) = local.iter().position(|&c| c == dash[0]) {
            if !four_digits(&local[..p]) {
                return false;
            }
            o.set_u32(0xa4, q::atol(&local[..p]) as u32);
            if !four_digits(&local[p + 1..]) {
                return false;
            }
            o.set_u32(0xa8, q::atol(&local[p + 1..]) as u32);
        } else if four_digits(local) && n1 == dash && four_digits(&n2) {
            let (a, b) = (q::atol(local) as u32, q::atol(&n2) as u32);
            o.set_u32(0xa4, a);
            o.set_u32(0xa8, b);
            o.set_b(0x94, b'-');
            if a != 0 && b < 10000 && a < b {
                *c18 = n2i - i;
                return true;
            }
            return false;
        }
        let (a, b) = (o.u32(0xa4), o.u32(0xa8));
        a != 0 && b < 10000 && a < b
    }

    /// Recognizer 0 (0x10003c60): a date (kind 8; +0x88 8 a date, 0xc a
    /// range of years). Outside mode 8 the order becomes `ymd`.
    fn rec_date(&self, c18: &mut usize, toks: &[Tok], i: usize, dmy: &mut Vec<u8>, o: &mut Obj) -> bool {
        o.set_u32(0, 8);
        o.set_u32(0x88, 8);
        o.set_b(0x8c, 0);
        o.set_b(0x94, 0);
        o.set_b(0x48, 0);
        let mode = o.u32(0x44);
        if mode != 8 {
            // `mov dword [dmy], 0x646d79`
            *dmy = b"ymd".to_vec();
        }
        o.set_cstr(0x8c, dmy);
        let local = o.cstr(4);
        o.strip_sign(&local);
        let t = o.cstr(4);
        if mode == 0 || mode == 8 {
            if self.slot7(dmy, &t, o) {
                return true;
            }
            if self.year_range(c18, toks, i, &t, o) {
                o.set_u32(0x88, 0xc);
                return true;
            }
            if mode == 8 {
                let mut d = Date::default();
                let ok = q::format_vxml10_date(&t, &mut d);
                o.set_u32(0x98, d.day);
                o.set_u32(0x9c, d.month);
                o.set_u32(0xa0, d.year);
                if ok && ieq(dmy, self.s(VXML_ORDER)) {
                    return true;
                }
            }
        }
        o.reset(&local);
        false
    }

    /// Recognizer 1 (0x10003e00): a time (kind 7), also `h:mm,dd` (the
    /// decimals at +0x4c).
    fn rec_time(&self, o: &mut Obj) -> bool {
        let local = o.cstr(4);
        o.set_b(0x4c, 0);
        o.set_u32(0, 7);
        o.set_u32(0x48, 7);
        for k in 0x8c..0x90 {
            o.set_b(k, 0);
        }
        match o.strip_sign(&local) {
            b'-' => o.set_b(0x8d, 1),
            b'+' => o.set_b(0x8e, 1),
            _ => {}
        }
        let t = o.cstr(4);
        let (dots, commas) = dots_commas(&t);
        let mode = o.u32(0x44);
        let unsigned = o.b(0x8d) == 0 && o.b(0x8e) == 0;
        let ok = if dots != 0 {
            commas == 0 && dots < 3 && matches!(mode, 0 | 7) && unsigned && q::is_time(&t)
        } else if commas == 1 {
            let p = t.iter().position(|&c| c == b',').unwrap();
            if matches!(mode, 0 | 7 | 8) && o.b(0x8d) == 0 {
                if o.b(0x8e) == 0 && q::is_time(&t) {
                    true
                } else if unsigned && q::is_time(&t[..p]) {
                    o.set_cstr(4, &t[..p]);
                    o.set_b(0x8c, 1);
                    o.set_cstr(0x4c, &t[p + 1..]);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else if commas == 0 {
            matches!(mode, 0 | 7 | 8) && unsigned && q::is_time(&t)
        } else {
            false
        };
        if !ok {
            o.reset(&local);
        }
        ok
    }

    /// Recognizer 2 (0x100040c0): an ordinal (kind 4): in ordinal mode,
    /// before `°`/`º` (not `C`), an apostrophe not followed by a number, or
    /// `:a`/`:e` (which it consumes).
    fn rec_ordinal(&self, c18: &mut usize, toks: &[Tok], i: usize, o: &mut Obj) -> bool {
        let local = o.cstr(4);
        o.set_u32(0, 4);
        o.strip_sign(&local);
        let (dots, commas) = dots_commas(&o.cstr(4));
        if dots + commas == 0 {
            if o.u32(0x44) == 4 {
                return true;
            }
            let nt = tok_text(toks, i + 1);
            if (nt == self.s(DEGREE) || nt == self.s(DEGREE_O)) && !ieq(tok_text(toks, i + 2), self.s(CELSIUS)) {
                o.set_u32(0x48, 5);
                return true;
            }
            if nt.first() == Some(&b'\'') && tok_type(toks, i + 2) != 1 {
                o.set_u32(0x48, 5);
                return true;
            }
            if nt.first() == Some(&b':') {
                let n2 = tok_text(toks, i + 2);
                if (n2 == self.s(COLON_A) || n2 == self.s(COLON_E)) && !local.iter().any(|c| self.s(ORD_REFUSE).contains(c)) {
                    o.set_u32(0x48, 5);
                    *c18 = 2;
                    return true;
                }
            }
        }
        o.reset(&local);
        false
    }

    /// Recognizer 3 (0x10004360): an amount of money (kind 0xc) with a
    /// currency sign after or before it.
    fn rec_currency(&self, toks: &[Tok], i: usize, o: &mut Obj) -> bool {
        o.set_b(0x4c, 0);
        o.set_u32(0, 0xc);
        o.set_u32(0x48, 0xf);
        for k in [0x56, 0x57, 0x58] {
            o.set_b(k, 0);
        }
        let local = o.cstr(4);
        match o.strip_sign(&local) {
            b'-' => o.set_b(0x56, 1),
            b'+' => o.set_b(0x57, 1),
            _ => {}
        }
        let nt = tok_text(toks, i + 1);
        if q::is_currency(nt) {
            o.set_cstr(0x4c, nt);
            return true;
        }
        if i > 0 && q::is_currency(tok_text(toks, i - 1)) {
            o.set_cstr(0x4c, tok_text(toks, i - 1));
            return true;
        }
        o.reset(&local);
        false
    }

    /// Recognizer 4 (0x100044c0): a real number (kind 9), only in mode 9.
    fn rec_real(&self, o: &mut Obj) -> bool {
        o.set_b(0x8c, 0);
        o.set_u32(0, 9);
        o.set_u32(0x48, 9);
        o.set_b(0x4c, 0);
        o.set_b(0xcc, 0);
        o.set_b(0xcd, 0);
        let local = o.cstr(4);
        match o.strip_sign(&local) {
            b'-' => o.set_b(0xcc, 1),
            b'+' => o.set_b(0xcd, 1),
            _ => {}
        }
        let t = o.cstr(4);
        let (dots, commas) = dots_commas(&t);
        let mode = o.u32(0x44);
        let ok = if dots == 0 && commas == 0 {
            if mode == 9 {
                o.set_cstr(0x4c, &t);
            }
            mode == 9
        } else if dots == 0 {
            if commas == 1 {
                if mode == 9 {
                    let p = t.iter().position(|&c| c == b',').unwrap();
                    o.set_cstr(0x4c, &t[p + 1..]);
                }
                if o.b(0x4c) != 0 {
                    o.set_b(0x8c, 0);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else if commas == 0 {
            if matches!(mode, 0 | 3) && q::is_big_integer(&t) {
                o.set_u32(0x48, 3);
                false
            } else if mode == 9 && q::is_big_integer(&t) {
                o.set_cstr(0x4c, &t);
                true
            } else {
                false
            }
        } else if commas == 1 {
            let pd = t.iter().rposition(|&c| c == b'.').unwrap();
            let pc = t.iter().rposition(|&c| c == b',').unwrap();
            if pd < pc {
                let int = &t[..pc];
                if q::is_big_integer(int) && mode == 9 {
                    o.set_cstr(0x4c, int);
                    o.set_u32(0x48, 3);
                    o.set_cstr(0x8c, &t[pc + 1..]);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };
        if !ok {
            o.reset(&local);
        }
        ok
    }

    /// Recognizer 5 (0x10004770): a fraction 0x10002330 knows (kind 0xb).
    fn rec_fraction(&self, o: &mut Obj) -> bool {
        let local = o.cstr(4);
        o.set_u32(0, 0xb);
        o.set_u32(0x48, 0xb);
        o.set_b(0x4c, 0);
        o.set_b(0x4d, 0);
        match o.strip_sign(&local) {
            b'-' => o.set_b(0x4c, 1),
            b'+' => o.set_b(0x4d, 1),
            _ => {}
        }
        let t = o.cstr(4);
        let (dots, commas) = dots_commas(&t);
        let ok = dots + commas == 0 && matches!(o.u32(0x44), 0 | 8 | 0xb) && self.fraction(&t).is_some();
        if !ok {
            o.reset(&local);
        }
        ok
    }

    /// 0x10002010 (and 0x10002330, the same list): a fraction's reading.
    fn fraction(&self, t: &[u8]) -> Option<&[u8]> {
        FRACTIONS.iter().find(|p| t == self.s(p.0)).map(|p| self.s(p.1))
    }

    /// Recognizer 6 (0x10004890): a telephone number (kind 1): more than
    /// six digits starting with an area code, or in phone mode any area
    /// code (a comma before an extension).
    fn rec_phone(&self, o: &mut Obj) -> bool {
        o.set_b(0x4d, 0);
        o.set_u32(0, 1);
        o.set_u32(0x48, 1);
        o.set_b(0x4c, 0);
        let local = o.cstr(4);
        o.strip_sign(&local);
        let t = o.cstr(4);
        let (dots, commas) = dots_commas(&t);
        let mode = o.u32(0x44);
        let ok = if dots != 0 {
            false
        } else if commas == 0 {
            match mode {
                1 => self.area_code(&t) != 0,
                0 | 8 => {
                    if t.iter().any(|c| self.s(NUM_SEPS).contains(c)) || t.len() <= 6 || self.area_code(&t) == 0 {
                        false
                    } else {
                        o.set_b(0x4c, 0);
                        true
                    }
                }
                _ => false,
            }
        } else if commas == 1 {
            let p = t.iter().position(|&c| c == b',').unwrap();
            let (num, ext) = (&t[..p], &t[p + 1..]);
            if mode == 1 && self.area_code(num) != 0 {
                o.set_cstr(4, num);
                o.set_b(0x4c, 1);
                o.set_cstr(0x4d, ext);
                true
            } else {
                false
            }
        } else {
            false
        };
        if !ok {
            o.reset(&local);
        }
        ok
    }

    /// Recognizer 7 (0x10004ca0): an integer (kind 0xa): up to five digits,
    /// with a decimal comma, or grouped by dots (+0x48 3).
    fn rec_integer(&self, o: &mut Obj) -> bool {
        o.set_b(0x4c, 0);
        o.set_b(0x8c, 0);
        o.set_u32(0, 0xa);
        o.set_u32(0x48, 0xa);
        o.set_b(0xcc, 0);
        o.set_b(0xcd, 0);
        o.set_b(0xd0, 0);
        let local = o.cstr(4);
        match o.strip_sign(&local) {
            b'-' => o.set_b(0xcc, 1),
            b'+' => o.set_b(0xcd, 1),
            _ => {}
        }
        let t = o.cstr(4);
        let (dots, commas) = dots_commas(&t);
        let mode = o.u32(0x44);
        let seps = |s: &[u8]| s.iter().any(|c| self.s(NUM_SEPS).contains(c));
        let ok = if dots == 0 && commas == 0 {
            match mode {
                0 | 8 if !seps(&t) && t.len() < 6 => {
                    o.set_cstr(0x4c, &t);
                    true
                }
                3 | 10 => {
                    o.set_cstr(0x4c, &t);
                    true
                }
                _ => false,
            }
        } else if dots == 0 {
            if commas == 1 && matches!(mode, 0 | 7 | 8 | 10) && !seps(&t) {
                let p = t.iter().position(|&c| c == b',').unwrap();
                o.set_cstr(0x4c, &t[..p]);
                o.set_cstr(0x8c, &t[p + 1..]);
                true
            } else {
                false
            }
        } else if commas == 0 {
            if matches!(mode, 0 | 3 | 10) && q::is_big_integer(&t) {
                o.set_cstr(0x4c, &t);
                o.set_u32(0x48, 3);
                true
            } else {
                false
            }
        } else if commas == 1 {
            let pd = t.iter().rposition(|&c| c == b'.').unwrap();
            let pc = t.iter().rposition(|&c| c == b',').unwrap();
            let int = &t[..pc];
            let dec = &t[pc + 1..];
            if pd < pc && q::is_big_integer(int) && ((mode == 0 && dec.len() <= 2) || mode == 3) {
                o.set_cstr(0x4c, int);
                o.set_cstr(0x8c, dec);
                o.set_u32(0x48, 3);
                true
            } else {
                false
            }
        } else {
            false
        };
        if !ok {
            o.reset(&local);
        }
        ok
    }

    /// Recognizer 8 (0x10004ad0): a code read digit by digit (kind 2), in
    /// mode 2 only (a decimal comma makes it fail).
    fn rec_digits(&self, o: &mut Obj) -> bool {
        o.set_u32(0, 2);
        o.set_u32(0x48, 2);
        o.set_b(0x8c, 0);
        o.set_b(0x4c, 0);
        o.set_b(0x8d, 0);
        o.set_b(0x8e, 0);
        let local = o.cstr(4);
        match o.strip_sign(&local) {
            b'-' => o.set_b(0x8d, 1),
            b'+' => o.set_b(0x8e, 1),
            _ => {}
        }
        let t = o.cstr(4);
        let (dots, commas) = dots_commas(&t);
        if dots == 0 && commas == 0 && o.u32(0x44) == 2 {
            return true;
        }
        if dots == 0 && commas == 1 && o.u32(0x44) == 2 {
            let p = t.iter().position(|&c| c == b',').unwrap();
            o.set_b(0x8c, 1);
            o.set_cstr(4, &t[..p]);
            o.set_cstr(0x4c, &t[p + 1..]);
        }
        o.reset(&local);
        false
    }

    /// 0x10002620: the ordinal of a digit string: the tens and hundreds of
    /// a number up to 999 read as cardinals with a space after each
    /// (`tjugo femte`), a round ten or longer number ends in `nde`, and 1–20
    /// have their own words. `""` when a part reads as nothing.
    fn ordinal(&self, digits: &[u8]) -> Vec<u8> {
        fn strip(mut t: &[u8]) -> &[u8] {
            while t.first() == Some(&b'0') {
                t = &t[1..];
            }
            t
        }
        let mut t = strip(cs(digits));
        let mut out = Vec::new();
        let mut n = t.len();
        while n <= 3 && ((at(t, 0) > b'1' && at(t, 1) > b'0') || n > 2) {
            let mut buf = t[..t.len().min(0x3f)].to_vec();
            if n == 2 || n == 3 {
                buf.truncate(1);
            }
            if n == 3 {
                buf.extend_from_slice(self.s(ORD_ZERO));
            }
            buf.extend_from_slice(self.s(ORD_ZERO));
            q::convert_integer(&mut out, 0, &buf, false, self);
            if out.is_empty() {
                return out;
            }
            out.extend_from_slice(self.s(ORD_SPACE));
            if n == 2 || n == 3 {
                t = &t[1..];
            }
            t = strip(t);
            n = t.len();
        }
        if (at(t, 0) > b'2' && at(t, 1) == b'0') || n > 3 {
            let mut buf = t[..t.len().min(0x3f)].to_vec();
            buf.truncate(n - 1);
            buf.extend_from_slice(self.s(ORD_ZERO));
            q::convert_integer(&mut out, 0, &buf, false, self);
            if out.is_empty() {
                return out;
            }
            out.extend_from_slice(self.s(ORD_NDE));
            t = &t[n - 1..];
        }
        if let Some(&(_, w)) = ORDINALS.iter().find(|p| t == self.s(p.0)) {
            out.extend_from_slice(self.s(w));
        }
        out
    }

    /// 0x100066e0: a date: the day's ordinal, the month, the year (as
    /// hundreds 1100–1999, else an integer and word 0).
    fn date(&self, out: &mut Vec<u8>, day: u32, month: u32, year: u32) {
        if day != 0 {
            *out = self.ordinal(&ltoa(day));
            q::add_separator(out, 0, self.word(1));
        }
        if month.wrapping_sub(1) < 12 {
            out.extend_from_slice(self.word(0x50 + month));
            q::add_separator(out, 0, self.word(1));
        }
        if year.wrapping_sub(0x44c) < 900 {
            out.extend(self.year_reading(year));
        } else if year != 0 {
            let at = out.len();
            q::convert_integer(out, at, &ltoa(year), false, self);
            q::add_separator(out, 0, self.word(0));
        }
    }

    /// 0x100068a0: a range of years (`nittonhundranittio till tvåtusen`).
    fn year_range_reading(&self, out: &mut Vec<u8>, from: u32, to: u32) {
        out.extend(self.year_reading(from));
        q::add_separator(out, 0, self.word(1));
        out.extend_from_slice(self.s(TILL));
        out.extend(self.year_reading(to));
    }

    /// 0x10005110: a time `h:mm[:ss]` (`sju och fyrtiofem`).
    fn time(&self, out: &mut Vec<u8>, text: &[u8]) {
        let seps = self.s(TIME_SEPS);
        let buf = cs(text);
        let Some(p) = buf.iter().position(|c| seps.contains(c)) else {
            return;
        };
        let h = &buf[..p];
        q::convert_integer(out, 0, h, false, self);
        if q::atol(h) > 0 {
            out.extend_from_slice(self.s(OCH_AFTER_HOUR));
        }
        let int = |out: &mut Vec<u8>, s: &[u8]| {
            let at = out.len();
            q::convert_integer(out, at, s, false, self);
        };
        let sep = |out: &mut Vec<u8>, id: u32| q::add_separator(out, 0, self.word(id));
        let m = &buf[p + 1..];
        let Some(k) = m.iter().position(|c| seps.contains(c)) else {
            if at(m, 0) == 0 || (at(m, 0) == b'0' && at(m, 1) == b'0') {
                return;
            }
            sep(out, 1);
            let mut rest = m;
            if at(m, 2) == b'-' {
                int(out, &m[..2]);
                sep(out, 0);
                rest = &m[3..];
            }
            int(out, rest);
            return;
        };
        let (min, sec) = (&m[..k], &m[k + 1..]);
        let zero2 = |s: &[u8]| at(s, 0) == b'0' && at(s, 1) == b'0';
        // minutes `och` seconds
        let both = |out: &mut Vec<u8>, sec: &[u8]| {
            sep(out, 0);
            int(out, min);
            sep(out, 1);
            out.extend_from_slice(self.s(OCH));
            sep(out, 1);
            int(out, sec);
        };
        if at(min, 0) != 0 && !zero2(min) && at(sec, 0) != 0 && !zero2(sec) {
            both(out, sec);
            return;
        }
        if at(min, 0) == b'0' {
            if at(min, 1) == b'0' && zero2(sec) {
                both(out, &sec[1..]);
                return;
            }
            if at(min, 1) == b'0' {
                sep(out, 1);
                int(out, sec);
                return;
            }
        }
        if zero2(sec) {
            both(out, &sec[1..]);
        }
    }

    /// Adds `minus`/`plus` for a sign flag pair.
    fn sign(&self, out: &mut Vec<u8>, minus: u8, plus: u8) {
        if minus == 1 {
            q::add_separator(out, 0, self.word(7));
        } else if plus == 1 {
            q::add_separator(out, 0, self.word(8));
        }
    }

    /// 0x10006990: the object's reading.
    fn render(&self, o: &Obj) -> Vec<u8> {
        let mut out = Vec::new();
        let sep = |out: &mut Vec<u8>, id: u32| q::add_separator(out, 0, self.word(id));
        match o.u32(0) {
            0 => q::convert_real_number(&mut out, 0, &o.0, self),
            1 => {
                self.digits(&mut out, &o.cstr(4), o.u32(0x48) as u8 != 0);
                if o.b(0x4d) != 0 {
                    let mut o2 = NumObj::default();
                    o2.set_cstr(4, &o.cstr(0x4d));
                    o2.set_u32(0x44, o.u32(0x44));
                    let at = out.len();
                    q::convert_real_number(&mut out, at, &o2, self);
                }
            }
            2 => {
                self.sign(&mut out, o.b(0x8d), o.b(0x8e));
                let pause = q::convert_integer(&mut out, 0, &o.cstr(4), true, self);
                if o.b(0x4c) != 0 {
                    sep(&mut out, if pause { 0 } else { 1 });
                    sep(&mut out, 4);
                    self.decimals(&mut out, &o.cstr(0x4c), true);
                }
            }
            3 | 10 => {
                self.sign(&mut out, o.b(0xcc), o.b(0xcd));
                let at = out.len();
                let pause = if o.u32(0x48) == 10 {
                    q::convert_integer(&mut out, at, &o.cstr(0x4c), false, self)
                } else {
                    q::convert_big_integer(&mut out, at, &o.cstr(0x4c), self);
                    true
                };
                if o.b(0x8c) != 0 {
                    sep(&mut out, if pause { 0 } else { 1 });
                    sep(&mut out, 4);
                    self.decimals(&mut out, &o.cstr(0x8c), false);
                }
            }
            4 => {
                if matches!(o.u32(0x48), 5 | 6) {
                    out = self.ordinal(&o.cstr(4));
                }
            }
            7 => {
                self.sign(&mut out, o.b(0x8d), o.b(0x8e));
                self.time(&mut out, &o.cstr(4));
                if o.b(0x4c) != 0 {
                    sep(&mut out, 0);
                    sep(&mut out, 4);
                    self.decimals(&mut out, &o.cstr(0x4c), false);
                }
            }
            8 => match o.u32(0x88) {
                8 => self.date(&mut out, o.u32(0x98), o.u32(0x9c), o.u32(0xa0)),
                0xc => self.year_range_reading(&mut out, o.u32(0xa4), o.u32(0xa8)),
                _ => {}
            },
            9 => {
                self.sign(&mut out, o.b(0xcc), o.b(0xcd));
                q::convert_integer(&mut out, 0, &o.cstr(0x4c), false, self);
                if o.b(0x8c) != 0 {
                    sep(&mut out, 0);
                    sep(&mut out, 4);
                    self.decimals(&mut out, &o.cstr(0x8c), false);
                }
            }
            0xb => {
                if let Some(f) = self.fraction(&o.cstr(4)) {
                    out = f.to_vec();
                }
            }
            0xc => {
                self.sign(&mut out, o.b(0x56), o.b(0x57));
                q::convert_currency(self, &mut out, &o.cstr(4), &o.cstr(0x4c));
            }
            _ => {}
        }
        out
    }
}

/// `LesIniNumeri` slot 9 (0x10006ea0): the reading of the number token
/// `toks[i]` for the engine's reading type `kind` (5/6 ordinal: mode 4,
/// +0x48 5/6) and date order `dmy`, and how many following tokens it used.
pub(super) fn leggi_numero(sv: &Swedish, toks: &mut [Tok], i: usize, kind: u32, dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
    let text = toks.get(i)?.text.clone();
    let mut o = Obj(NumObj::default());
    o.reset(b"");
    if kind == 5 || kind == 6 {
        o.set_u32(0x44, 4);
        o.set_u32(0x48, if kind == 5 { 5 } else { 6 });
    } else {
        o.set_u32(0x44, kind);
    }
    o.set_cstr(4, &text);
    let mut dmy = cs(dmy).to_vec();
    // 0x10005bd0: the mode's own recognizer first
    let mut order: Vec<usize> = (0..9).collect();
    let first = match o.u32(0x44) {
        1 => 6,
        2 => 8,
        4 => 2,
        7 => 1,
        9 => 4,
        10 => 7,
        11 => 5,
        12 => 3,
        _ => 0,
    };
    let r = order.remove(first);
    order.insert(0, r);
    let mut c18 = 0;
    let hit = order.iter().any(|&r| match r {
        0 => sv.rec_date(&mut c18, toks, i, &mut dmy, &mut o),
        1 => sv.rec_time(&mut o),
        2 => sv.rec_ordinal(&mut c18, toks, i, &mut o),
        3 => sv.rec_currency(toks, i, &mut o),
        4 => sv.rec_real(&mut o),
        5 => sv.rec_fraction(&mut o),
        6 => sv.rec_phone(&mut o),
        7 => sv.rec_integer(&mut o),
        _ => sv.rec_digits(&mut o),
    });
    if !hit {
        o.reset(&text);
    }
    Some((sv.render(&o), c18))
}

impl NumSlots for Swedish {
    /// Slot 0 (0x10003160).
    fn word(&self, id: u32) -> &[u8] {
        Swedish::word(self, id)
    }
    /// Slot 1 (0x100030e0): a telephone number: its area code, then the
    /// rest (`ELQNumConvertPhoneNumber` steps 0 and 1).
    fn digits(&self, out: &mut Vec<u8>, digits: &[u8], flag: bool) {
        let digits = cs(digits);
        let area = self.area_code(digits);
        let mut len = area;
        let mut t = Vec::new();
        q::convert_phone_number(self, &mut t, digits, flag, &mut len, 0);
        q::convert_phone_number(self, &mut t, &digits[area.min(digits.len())..], flag, &mut len, 1);
        out.extend(t);
    }
    /// Slot 2 (0x10005cf0).
    fn cardinal(&self, out: &mut Vec<u8>, digits: &[u8]) {
        let s = self.cardinal_c(digits);
        out.extend(s);
    }
    /// Slot 3 (0x10005cb0): `noll`, `ett`, else 0x100056c0's word.
    fn digit(&self, d: u8) -> &[u8] {
        match d {
            b'0' => self.word(0xf),
            b'1' => self.word(0x10),
            _ => self.n_word(d as i8 as i32 - 0x30),
        }
    }
    /// Slot 4 (0x10003130): digit by digit unless a cardinal of 2–12
    /// digits (2–3 when it had leading zeros).
    fn by_digits(&self, flag: bool, len: usize, lead0: bool) -> bool {
        !(!flag && len < 13 && (len < 4 || !lead0) && len != 1)
    }
    /// Slot 5 (0x100054b0): the currency's unit or cent name between
    /// spaces (singular after `1`).
    fn currency(&self, out: &mut Vec<u8>, amount: &[u8], currency: &[u8], cents: bool) {
        let Some(k) = (0..8).find(|&k| self.s(self.pe.u32(CURRENCIES + 0x14 * k)) == cs(currency)) else {
            return;
        };
        out.extend_from_slice(self.word(1));
        let one = cs(amount) == b"1";
        let f = match (cents, one) {
            (false, true) => 4,
            (false, false) => 8,
            (true, true) => 0xc,
            (true, false) => 0x10,
        };
        out.extend_from_slice(self.s(self.pe.u32(CURRENCIES + 0x14 * k + f)));
        // `ELQNumAddSeparator` at the string's end: always appended
        out.extend_from_slice(self.word(1));
    }
    /// Slot 6 (0x10006520): decimals: leading zeros as `noll`, then up to
    /// three digits as a cardinal (unless `flag`), else digit by digit.
    fn decimals(&self, out: &mut Vec<u8>, digits: &[u8], flag: bool) -> bool {
        let digits = cs(digits);
        let (lead0, pause) = q::set_pauses(digits);
        let sep = self.word(if pause { 0 } else { 1 });
        let mut t = Vec::new();
        q::add_separator(&mut t, 0, sep);
        let mut d = digits;
        while d.first() == Some(&b'0') {
            t.extend_from_slice(self.digit(b'0'));
            d = &d[1..];
            if d.first() != Some(&b'0') {
                break;
            }
            q::add_separator(&mut t, 0, sep);
        }
        let n = d.len() & 0xffff;
        if n != 0 {
            if lead0 {
                q::add_separator(&mut t, 0, sep);
            }
            if n <= 3 && n != 1 && !flag {
                self.cardinal(&mut t, d);
            } else {
                for k in 1..n {
                    t.extend_from_slice(self.digit(d[k - 1]));
                    q::add_separator(&mut t, 0, sep);
                }
                t.extend_from_slice(self.digit(d[n - 1]));
            }
        }
        out.extend(t);
        pause
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `"…"` field of a trace line (`\xNN`, `\\`, `\"` escapes).
    fn field(line: &str, name: &str) -> Option<Vec<u8>> {
        let start = line.find(&format!("{name}=\""))? + name.len() + 2;
        let b = line.as_bytes();
        let mut out = Vec::new();
        let mut k = start;
        while k < b.len() && b[k] != b'"' {
            if b[k] == b'\\' {
                match b[k + 1] {
                    b'x' => {
                        out.push(u8::from_str_radix(&line[k + 2..k + 4], 16).unwrap());
                        k += 4;
                    }
                    c => {
                        out.push(match c {
                            b'n' => b'\n',
                            b't' => b'\t',
                            c => c,
                        });
                        k += 2;
                    }
                }
            } else {
                out.push(b[k]);
                k += 1;
            }
        }
        Some(out)
    }

    fn num(line: &str, name: &str) -> u32 {
        let s = &line[line.find(&format!("{name}=")).unwrap() + name.len() + 1..];
        s.split(|c: char| !c.is_ascii_digit()).next().unwrap().parse().unwrap()
    }

    fn tok(line: &str) -> Tok {
        let w: Vec<&str> = line.split_whitespace().collect();
        Tok { text: field(line, "t").unwrap(), kind: w[2].parse().unwrap(), expanded: w[4] != "0" }
    }

    /// Replays every slot-9 call traced in the `.les` files listed by
    /// `QQ_SV_NUM` (comma-separated), comparing the reading, the tokens used
    /// and the rewritten tokens. The module comes from `QQ_SV_LTTS7`.
    #[test]
    #[ignore]
    fn replay_slot9() {
        let Ok(list) = std::env::var("QQ_SV_NUM") else { return };
        let root = std::env::var("QQ_SV_LTTS7").unwrap_or_else(|_| "/tmp/qqv/LTTS7".into());
        let dll = std::fs::read(format!("{root}/bin/LTTS7Swedish.dll")).unwrap();
        let dbl = std::fs::read(format!("{root}/data/languages/Swedish/Swedish.dbl")).ok();
        let sv = Swedish::new(dll, dbl.as_deref()).unwrap();
        let (mut calls, mut bad) = (0, 0);
        for f in list.split(',') {
            let raw = std::fs::read(f).unwrap();
            let text: String = raw.iter().map(|&c| c as char).collect();
            let lines: Vec<&str> = text.lines().collect();
            let mut k = 0;
            while k < lines.len() {
                let l = lines[k];
                k += 1;
                if !l.starts_with("> NUM") {
                    continue;
                }
                let (kind, idx, dmy) = (num(l, "type"), num(l, "idx") as usize, field(l, "a3").unwrap());
                let mut toks = Vec::new();
                while k < lines.len() && lines[k].trim_start().starts_with("T ") {
                    toks.push(tok(lines[k]));
                    k += 1;
                }
                let r = lines[k];
                k += 1;
                let (ret, c18) = (field(r, "ret").unwrap(), num(r, "c18") as usize);
                let mut after = Vec::new();
                while k < lines.len() && lines[k].trim_start().starts_with("U ") {
                    after.push(field(lines[k], "t").unwrap());
                    k += 1;
                }
                calls += 1;
                let (got, used) = leggi_numero(&sv, &mut toks, idx, kind, &dmy).unwrap();
                let texts: Vec<Vec<u8>> = toks.iter().map(|t| t.text.clone()).collect();
                if got != ret || used != c18 || (!after.is_empty() && texts != after) {
                    bad += 1;
                    eprintln!(
                        "{f}: {:?} kind {kind}: got {:?} {used}, want {:?} {c18}{}",
                        String::from_utf8_lossy(&toks[idx].text),
                        String::from_utf8_lossy(&got),
                        String::from_utf8_lossy(&ret),
                        if texts != after && !after.is_empty() { " (tokens differ)" } else { "" }
                    );
                }
            }
        }
        eprintln!("{bad} of {calls} slot-9 calls differ");
        assert_eq!(bad, 0);
    }
}
