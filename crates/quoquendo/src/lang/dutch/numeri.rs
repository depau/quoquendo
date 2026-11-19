//! Dutch number reading: `LesIniNumeri` (0x10006ef0) fills slots 0–7 and
//! 9; slot 9 (0x10006820) is the reader the engine calls for a NUMBER token
//! (MECHANICS.md §4g "Numbers").
//!
//! The module is built on the same template as LTTS7German.dll's: slot 9
//! classifies the token with nine recognizers (0x10005bd0 sets their order,
//! the reading mode's own one first), each filling the number object
//! (`NumObj`, 0xd4 bytes), and renders it (0x10005cb0). Every table is read
//! from the DLL.

use super::Dutch;
use crate::lang::common::Tok;
use crate::lang::elqnum::{self as q, Date, NumObj, NumSlots};

/// Slot 0's `{ id, char * }` pairs (0x53 of them); `""` (0x1001166e) for
/// an id it lacks.
const WORDS: u32 = 0x10013810;
const EMPTY: u32 = 0x1001166e;
/// 0x10002720's area codes (0xf7 sorted strings), searched by
/// `ELQSearchString` flag 2 (an entry that starts the digits).
const AREA_CODES: u32 = 0x10013390;
/// Slot 5's currencies: 8 × `{ code, unit, units, cent, cents }`.
const CURRENCIES: u32 = 0x10013770;
/// 0x10002dd0's month names: 31 × `{ name, number }`.
const MONTHS: u32 = 0x10013aa8;
/// 0x10002e00's word before a bare year after the number (`voor`; its loop
/// runs once).
const VOOR: u32 = 0x1001376c;
/// 0x10002e00's words before a bare year: in jaar voor vanaf tot na uit
/// van sinds sedert.
const YEAR_WORDS: u32 = 0x10013ba0;
/// `-` `.` `/` (0x10002e00: a month name two tokens back through one).
const DATE_SEPS: [u32; 3] = [0x10013ddc, 0x10013dd8, 0x10013dd4];
/// `d`, `y` (`ELQNumExtractDate` orders) and the order 0x100037b0 accepts a
/// VoiceXML date in.
const ORDER_D: u32 = 0x10011560;
const ORDER_Y: u32 = 0x10011614;
const ORDER_VXML: u32 = 0x10011568;
/// 0x100020a0's ordinals 1–20: (number, ordinal) string addresses.
const ORDINALS: [(u32, u32); 20] = [
    (0x10013dc0, 0x10013db8),
    (0x10013db4, 0x10013dac),
    (0x10013da8, 0x10013da0),
    (0x10013d9c, 0x10013d94),
    (0x10013d90, 0x10013d88),
    (0x10013d84, 0x10013d7c),
    (0x10013d78, 0x10013d70),
    (0x10013d6c, 0x10013d64),
    (0x10013d60, 0x10013d58),
    (0x10013d54, 0x10013d4c),
    (0x10013d48, 0x10013d40),
    (0x10013d3c, 0x10013d30),
    (0x10013d2c, 0x10013d20),
    (0x10013d1c, 0x10013d10),
    (0x10013d0c, 0x10013d00),
    (0x10013cfc, 0x10013cf0),
    (0x10013cec, 0x10013ce0),
    (0x10013cdc, 0x10013cd0),
    (0x10013ccc, 0x10013cc0),
    (0x10013cbc, 0x10013cb0),
];
/// `ste` after a larger ordinal's cardinal.
const STE: u32 = 0x10013cac;
/// 0x10001a90's fractions in its order (0x10001db0 recognizes the same
/// list): (fraction, reading); `1/10` reads `eenelfde`, `2/3` nothing.
const FRACTIONS: [(u32, u32); 14] = [
    (0x10013ca8, 0x10013ca0),
    (0x10013c9c, 0x10013c90),
    (0x10013c8c, 0x10013c80),
    (0x10013c7c, 0x10013c70),
    (0x10013c6c, 0x10013c60),
    (0x10013c5c, 0x10013c50),
    (0x10013c4c, 0x10013c40),
    (0x10013c3c, 0x10013c30),
    (0x10013c28, 0x10013c1c),
    (0x10013c14, 0x10013c04),
    (0x10013bfc, 0x10013bf0),
    (0x10013be8, 0x10013bdc),
    (0x10013bd8, 0x1001166e),
    (0x10013bd4, 0x10013bc8),
];
/// `1`, `0`, `00` and `en` (0x100028c0, slot 5).
const ONE: u32 = 0x10013dc0;
const ZERO: u32 = 0x10013dcc;
const ZERO_ZERO: u32 = 0x10013dc8;
const EN: u32 = 0x10013dc4;
/// The time's hour word ` uur ` and its separators `.,:` (0x10004cd0).
const UUR: u32 = 0x1001406c;
const TIME_SEPS: u32 = 0x10014074;
/// `:/-+` (0x100044e0, 0x100048f0: not a phone number or plain integer).
const NOT_PLAIN: &[u8] = b":/-+";
/// The year's words (0x10005710): `000`, ` duizend`, ` duizend `,
/// ` honderd`, ` en `, ` honderd `.
const Y000: u32 = 0x100140a4;
const DUIZEND: u32 = 0x10014098;
const DUIZEND_: u32 = 0x1001408c;
const HONDERD: u32 = 0x10012ab8;
const EN_: u32 = 0x10014084;
const HONDERD_: u32 = 0x10014078;
/// Slot 9's and the currency recognizer's names: euro EUR eur dollars
/// dollar livre (+ `sterling` 3 tokens on) usd USD JPY jpy gbp GBP yen.
const EURO: [u32; 3] = [0x10012c28, 0x10014060, 0x1001405c];
const DOLLAR: [u32; 4] = [0x10012c50, 0x10014054, 0x10014050, 0x1001404c];
const YEN: [u32; 3] = [0x10012c20, 0x10014048, 0x10014044];
const LIVRE: u32 = 0x1001403c;
const STERLING: u32 = 0x10014030;
const GBP: [u32; 2] = [0x1001402c, 0x10014028];
/// The currency signs those names become: € $ ¥ £.
const EURO_SIGN: u32 = 0x10012c30;
const DOLLAR_SIGN: u32 = 0x10011528;
const YEN_SIGN: u32 = 0x10012c24;
const POUND_SIGN: u32 = 0x10012c44;

/// The module's number object (`NumObj`'s 0xd4 bytes).
#[derive(Clone)]
struct Obj([u8; 0xd4]);

impl Obj {
    fn u32(&self, o: usize) -> u32 {
        u32::from_le_bytes(self.0[o..o + 4].try_into().unwrap())
    }
    fn set_u32(&mut self, o: usize, v: u32) {
        self.0[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn b(&self, o: usize) -> u8 {
        self.0[o]
    }
    fn set_b(&mut self, o: usize, v: u8) {
        self.0[o] = v;
    }
    fn cstr(&self, o: usize) -> Vec<u8> {
        cs(&self.0[o..]).to_vec()
    }
    fn set_cstr(&mut self, o: usize, s: &[u8]) {
        let s = cs(s);
        let n = s.len().min(self.0.len() - o);
        self.0[o..o + n].copy_from_slice(&s[..n]);
        if o + n < self.0.len() {
            self.0[o + n] = 0;
        }
    }
    fn num(&self) -> NumObj {
        let mut n = NumObj::default();
        n.0.copy_from_slice(&self.0);
        n
    }
    /// `ELQNumSetDefaultType`: back to a plain number of `text`.
    fn reset(&mut self, text: &[u8]) {
        let mut n = self.num();
        q::set_default_type(&mut n, text);
        self.0.copy_from_slice(&n.0);
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

/// Counts of `.` and `,`.
fn dots_commas(s: &[u8]) -> (usize, usize) {
    (s.iter().filter(|&&c| c == b'.').count(), s.iter().filter(|&&c| c == b',').count())
}

/// 0x10002da0: four digits.
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

/// 0x100019a0 (count 0xf): the index of the token `off` tokens from `idx`,
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

impl Dutch {
    fn word(&self, id: u32) -> &[u8] {
        (0..0x53).find(|&k| self.pe.u32(WORDS + 8 * k) == id).map(|k| self.s(self.pe.u32(WORDS + 8 * k + 4))).unwrap_or_else(|| self.s(EMPTY))
    }

    /// 0x100051e0: the word of 1–20 and the tens 30–90, else `""`.
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

    /// 0x100055e0: the two digits at `p` (`één en twintig`), `""` for 00.
    fn two(&self, p: &[u8]) -> Vec<u8> {
        let pair = [at(p, 0), if at(p, 0) == 0 { 0 } else { at(p, 1) }];
        let n = q::atol(cs(&pair));
        let mut b = Vec::new();
        if n == 0 {
            return b;
        }
        if pair[0] == b'1' || n < 10 {
            b.extend_from_slice(self.n_word(n));
            return b;
        }
        b.extend_from_slice(self.n_word(pair[1] as i8 as i32 - 0x30));
        if pair[1] != b'0' {
            b.extend_from_slice(self.word(0xd));
        }
        b.extend_from_slice(self.n_word((pair[0] as i8 as i32 - 0x30) * 10));
        b
    }

    /// 0x10005400: the hundreds digit at `p` appended to `s` (`honderd `,
    /// `twee honderd ` with a separator); false for 0.
    fn hundreds(&self, s: &mut Vec<u8>, p: &[u8]) -> bool {
        let d = at(p, 0);
        match d {
            b'0' => return false,
            b'1' => s.extend_from_slice(self.word(0x3a)),
            _ => s.extend_from_slice(self.n_word(d as i8 as i32 - 0x30)),
        }
        if d != b'1' {
            s.extend_from_slice(self.word(0x3b));
            q::add_separator(s, 0, self.word(1));
        }
        true
    }

    /// Slot 2 (0x10006220): the cardinal of a digit string, position by
    /// position from the hundreds of billions, a trailing word 0 removed.
    fn cardinal_c(&self, digits: &[u8]) -> Vec<u8> {
        let digits = cs(digits);
        let len = digits.len();
        let d = |k: usize| at(digits, k);
        let rest = |k: usize| digits.get(k..).unwrap_or(b"");
        let mut s: Vec<u8> = Vec::new();
        let (mut f10, mut f11, mut f12, mut f13, mut f1b) = (false, true, false, false, false);
        let mut p = 0;
        let w0 = self.word(0);
        let w1 = self.word(1);
        let sep_unless_space = |s: &mut Vec<u8>| {
            if s.last() != Some(&b' ') {
                q::add_separator(s, 0, w1);
            }
        };
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
                if al != b'1' && !f10 {
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
        if len >= 4 {
            let al = d(p);
            if !(al == b'0' && f11) {
                // 0x100065d0…0x10006658
                let sep = if al == b'1' && !f10 && !f12 {
                    s.extend_from_slice(self.word(0x47));
                    w1
                } else {
                    if !f10 {
                        s.extend_from_slice(self.n_word(al as i8 as i32 - 0x30));
                    }
                    if !f1b {
                        q::add_separator(&mut s, 0, w1);
                        s.extend_from_slice(self.word(0x48));
                    } else {
                        s.extend_from_slice(self.word(0x47));
                    }
                    if !f12 || f1b {
                        w1
                    } else {
                        w0
                    }
                };
                q::add_separator(&mut s, 0, sep);
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

    /// LTTS7Util's `ELQSearchString` with flag 2 over `count` sorted
    /// strings: a binary search for an entry the key starts with.
    fn area_code(&self, digits: &[u8]) -> usize {
        let key = cs(digits);
        let entry = |k: u32| self.s(self.pe.u32(AREA_CODES + 4 * k));
        let (mut lo, mut hi) = (0i64, 0xf7 - 1);
        while lo <= hi {
            let mid = (lo + hi) / 2;
            let e = entry(mid as u32);
            match key[..key.len().min(e.len())].cmp(e) {
                std::cmp::Ordering::Less => hi = mid - 1,
                std::cmp::Ordering::Greater => lo = mid + 1,
                std::cmp::Ordering::Equal => return e.len(),
            }
        }
        0
    }

    /// 0x10002dd0: a month name's number, 0 for none.
    fn month_no(&self, t: &[u8]) -> u32 {
        (0..31).find(|&k| ieq(self.s(self.pe.u32(MONTHS + 8 * k)), t)).map(|k| self.pe.u32(MONTHS + 8 * k + 4)).unwrap_or(0)
    }

    /// Slot 7 (0x10002850): `ELQNumExtractDate` in the order `dmy`, then
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

    /// `ELQNumExtractDate("y", text)` into the object's day and year.
    fn year_of(&self, text: &[u8], o: &mut Obj) -> bool {
        let mut d = Date::default();
        let ok = q::extract_date(self.s(ORDER_Y), text, &mut d);
        o.set_u32(0x98, d.day);
        o.set_u32(0xa0, d.year);
        ok
    }

    /// 0x10002e00: a day before a month name (and a year after it: the
    /// tokens used counted to them), a year before `voor`, after one of `in
    /// jaar voor vanaf tot na uit van sinds sedert`, or after a month name
    /// (two tokens back across `-` `.` `/`).
    fn date_words(&self, c18: &mut usize, toks: &[Tok], i: usize, text: &[u8], o: &mut Obj) -> bool {
        let i1 = tok_at(toks, i, 1);
        let i2 = tok_at(toks, i, 2);
        let n1 = tok_text(toks, i1).to_vec();
        let n2 = tok_text(toks, i2).to_vec();
        let p1 = tok_text(toks, tok_at(toks, i, -1)).to_vec();
        let p2 = tok_text(toks, tok_at(toks, i, -2)).to_vec();
        o.set_u32(0x98, 0);
        o.set_u32(0x9c, 0);
        o.set_u32(0xa0, 0);
        let m = self.month_no(&n1);
        o.set_u32(0x9c, m);
        if (1..=12).contains(&m) {
            let mut d = Date::default();
            let ok = q::extract_date(self.s(ORDER_D), text, &mut d);
            o.set_u32(0x98, d.day);
            if ok {
                *c18 = if i1 == i { 1 } else { i1 - i };
                if four_digits(&n2) {
                    let mut y = Date::default();
                    if q::extract_date(self.s(ORDER_Y), &n2, &mut y) {
                        *c18 = if i2 == i { 2 } else { i2 - i };
                    }
                    o.set_u32(0xa0, y.year);
                }
                return true;
            }
        }
        if ieq(&n1, self.s(self.pe.u32(VOOR))) && four_digits(text) && self.year_of(text, o) {
            return true;
        }
        if i == 0 {
            return false;
        }
        for k in 0..10 {
            if ieq(&p1, self.s(self.pe.u32(YEAR_WORDS + 4 * k))) && four_digits(text) && self.year_of(text, o) {
                return true;
            }
        }
        let e = if i > 1 && DATE_SEPS.iter().any(|&a| ieq(&p1, self.s(a))) { &p2 } else { &p1 };
        let m = self.month_no(e);
        (1..=12).contains(&m) && four_digits(text) && self.year_of(text, o)
    }

    /// Recognizer 0 (0x100037b0): a date (kind 8).
    fn rec_date(&self, c18: &mut usize, toks: &[Tok], i: usize, dmy: &[u8], o: &mut Obj) -> bool {
        o.set_b(0x8c, 0);
        let local = o.cstr(4);
        o.set_u32(0, 8);
        o.set_u32(0x88, 8);
        o.set_b(0x94, 0);
        o.set_b(0x48, 0);
        o.set_cstr(0x8c, dmy);
        o.strip_sign(&local);
        let mode = o.u32(0x44);
        let t = o.cstr(4);
        if mode == 0 || mode == 8 {
            if self.slot7(dmy, &t, o) {
                return true;
            }
            if mode == 8 {
                let mut d = Date::default();
                let ok = q::format_vxml10_date(&t, &mut d);
                o.set_u32(0x98, d.day);
                o.set_u32(0x9c, d.month);
                o.set_u32(0xa0, d.year);
                if ok && ieq(dmy, self.s(ORDER_VXML)) {
                    return true;
                }
            }
        }
        if self.date_words(c18, toks, i, &t, o) {
            o.set_u32(0x88, 0xe);
            return true;
        }
        o.reset(&local);
        false
    }

    /// Recognizer 1 (0x10003960): a time (kind 7).
    fn rec_time(&self, o: &mut Obj) -> bool {
        let local = o.cstr(4);
        o.set_u32(0, 7);
        o.set_u32(0x48, 7);
        o.set_b(0x4c, 0);
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
        let ok = if dots == 0 {
            commas == 0 && matches!(mode, 0 | 7 | 8) && unsigned && q::is_time(&t)
        } else {
            commas == 0 && dots < 3 && matches!(mode, 0 | 7) && unsigned && q::is_time(&t)
        };
        if !ok {
            o.reset(&local);
        }
        ok
    }

    /// Recognizer 2 (0x10003b00): an ordinal (kind 4), in ordinal mode only.
    fn rec_ordinal(&self, o: &mut Obj) -> bool {
        let local = o.cstr(4);
        o.set_u32(0, 4);
        o.strip_sign(&local);
        let (dots, commas) = dots_commas(&o.cstr(4));
        if dots + commas == 0 && o.u32(0x44) == 4 {
            return true;
        }
        o.reset(&local);
        false
    }

    /// Recognizer 3 (0x10003bf0): an amount of money (kind 0xc) with a
    /// currency sign before or after it, or a currency name after it (that
    /// token emptied and used).
    fn rec_currency(&self, c18: &mut usize, toks: &mut [Tok], i: usize, o: &mut Obj) -> bool {
        o.set_u32(0, 0xc);
        o.set_u32(0x48, 0xf);
        for k in [0x56, 0x57, 0x58, 0x5a] {
            o.set_b(k, 0);
        }
        o.set_b(0x4c, 0);
        let local = o.cstr(4);
        match o.strip_sign(&local) {
            b'-' => o.set_b(0x56, 1),
            b'+' => o.set_b(0x57, 1),
            _ => {}
        }
        let t = o.cstr(4);
        let (dots, commas) = dots_commas(&t);
        if dots == 1 {
            // a dot not grouping thousands: read as the decimal point
            let after = t.iter().position(|&c| c == b'.').map(|p| t.len() - p).unwrap_or(0);
            if commas > 0 || after.wrapping_sub(1) != 3 {
                o.set_b(0x5a, 1);
            }
        }
        if i > 0 && q::is_currency(tok_text(toks, i - 1)) {
            o.set_cstr(0x4c, tok_text(toks, i - 1));
            return true;
        }
        let nt = tok_text(toks, i + 1).to_vec();
        let is = |list: &[u32]| list.iter().any(|&a| nt == self.s(a));
        let code: Vec<u8> = if q::is_currency(&nt) {
            nt.clone()
        } else if is(&EURO) {
            self.s(EURO_SIGN).to_vec()
        } else if is(&DOLLAR) {
            self.s(DOLLAR_SIGN).to_vec()
        } else if is(&YEN) {
            self.s(YEN_SIGN).to_vec()
        } else if (nt == self.s(LIVRE) && tok_text(toks, i + 2) == self.s(STERLING)) || is(&GBP) {
            self.s(POUND_SIGN).to_vec()
        } else {
            o.reset(&local);
            return false;
        };
        o.set_cstr(0x4c, &code);
        if let Some(t) = toks.get_mut(i + 1) {
            t.text.clear();
        }
        *c18 = 1;
        true
    }

    /// Recognizer 4 (0x10004110): a real number (kind 9), only in mode 9.
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

    /// Recognizer 5 (0x100043c0): a fraction 0x10001db0 knows (kind 0xb).
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

    /// 0x10001a90 (and 0x10001db0, the same list): a fraction's reading.
    fn fraction(&self, t: &[u8]) -> Option<&[u8]> {
        FRACTIONS.iter().find(|p| t == self.s(p.0)).map(|p| self.s(p.1))
    }

    /// Recognizer 6 (0x100044e0): a telephone number (kind 1): more than
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
                0 | 8 => {
                    if t.iter().any(|c| NOT_PLAIN.contains(c)) || t.len() <= 6 || self.area_code(&t) == 0 {
                        false
                    } else {
                        o.set_b(0x4c, 0);
                        true
                    }
                }
                1 => self.area_code(&t) != 0,
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

    /// Recognizer 7 (0x100048f0): an integer (kind 0xa): up to five digits,
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
        let seps = |s: &[u8]| s.iter().any(|c| NOT_PLAIN.contains(c));
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

    /// Recognizer 8 (0x10004720): a code read digit by digit (kind 2), in
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
        o.reset(&local);
        false
    }

    /// 0x100020a0: the ordinal of a digit string: 1–20 from a table, else
    /// the cardinal and `ste`.
    fn ordinal(&self, t: &[u8]) -> Vec<u8> {
        let mut t = cs(t);
        while t.first() == Some(&b'0') {
            t = &t[1..];
        }
        if let Some(&(_, v)) = ORDINALS.iter().find(|p| t == self.s(p.0)) {
            return self.s(v).to_vec();
        }
        let mut out = Vec::new();
        q::convert_integer(&mut out, 0, t, false, self);
        if !out.is_empty() {
            out.extend_from_slice(self.s(STE));
        }
        out
    }

    /// 0x10005aa0: a date: the day's cardinal, the month (words 0x51–0x5c),
    /// the year (0x10005710).
    fn date(&self, out: &mut Vec<u8>, day: u32, month: u32, year: u32) {
        if day > 0 {
            let at = out.len();
            q::convert_integer(out, at, &ltoa(day), false, self);
            q::add_separator(out, 0, self.word(1));
        }
        if (1..=12).contains(&month) {
            out.extend_from_slice(self.word(0x50 + month));
            q::add_separator(out, 0, self.word(1));
        }
        if year > 0 {
            self.year(out, year);
        }
    }

    /// 0x10005710: a year: below 1000 the first two digits (sic); `x000`
    /// `x duizend`; `x0yz` `x duizend yz` (`duizend` alone for 1); else
    /// `ab honderd cd` (`ab honderd en d`, `ab honderd`).
    fn year(&self, out: &mut Vec<u8>, year: u32) {
        let y = ltoa(year);
        let d = |k: usize| at(&y, k);
        let one = |c: u8| self.n_word(c as i8 as i32 - 0x30);
        if (1..=999).contains(&year) {
            out.extend(self.two(&y));
            return;
        }
        if y.get(1..) == Some(self.s(Y000)) {
            out.extend_from_slice(one(d(0)));
            out.extend_from_slice(self.s(DUIZEND));
            return;
        }
        if d(1) == b'0' {
            if d(0) != b'1' {
                out.extend_from_slice(one(d(0)));
            }
            out.extend_from_slice(self.s(DUIZEND_));
            if d(2) == b'0' {
                out.extend_from_slice(self.n_word(q::atol(&y[3..])));
            } else {
                out.extend(self.two(&y[2..]));
            }
            return;
        }
        out.extend(self.two(&y[..2]));
        if d(2) == b'0' {
            out.extend_from_slice(self.s(HONDERD));
            if d(3) == b'0' {
                return;
            }
            out.extend_from_slice(self.s(EN_));
            out.extend_from_slice(self.n_word(q::atol(&y[3..])));
        } else {
            out.extend_from_slice(self.s(HONDERD_));
            out.extend(self.two(&y[2..]));
        }
    }

    /// 0x10004cd0: a time `h:mm[:ss]` (`zeven uur vijf en veertig`).
    fn time(&self, out: &mut Vec<u8>, text: &[u8]) {
        let seps = self.s(TIME_SEPS);
        let buf = cs(text);
        let Some(p) = buf.iter().position(|c| seps.contains(c)) else {
            return;
        };
        q::convert_integer(out, 0, &buf[..p], false, self);
        out.extend_from_slice(self.s(UUR));
        let rest = &buf[p + 1..];
        let int = |out: &mut Vec<u8>, s: &[u8]| {
            let at = out.len();
            q::convert_integer(out, at, s, false, self);
        };
        let sep = |out: &mut Vec<u8>, id: u32| q::add_separator(out, 0, self.word(id));
        if let Some(k) = rest.iter().position(|c| seps.contains(c)) {
            let min = &rest[..k];
            let sec = &rest[k + 1..];
            let dl = at(min, 0);
            if dl != 0 && !(dl == b'0' && at(min, 1) == b'0') && at(sec, 0) != 0 && !(at(sec, 0) == b'0' && at(sec, 1) == b'0') {
                sep(out, 0);
                sep(out, 1);
                int(out, min);
                sep(out, 0x5e);
                sep(out, 0);
                sep(out, 1);
                int(out, sec);
                sep(out, 0x5d);
                return;
            }
            let sec00 = at(sec, 0) == b'0' && at(sec, 1) == b'0';
            if dl == b'0' && at(min, 1) == b'0' {
                if !sec00 {
                    sep(out, 1);
                    int(out, sec);
                    sep(out, 0x5d);
                }
                return;
            }
            if sec00 {
                sep(out, 1);
                int(out, min);
            }
            return;
        }
        let al = at(rest, 0);
        if al == 0 || (al == b'0' && at(rest, 1) == b'0') {
            return;
        }
        sep(out, 1);
        let mut m = rest;
        if at(rest, 1) != 0 && at(rest, 2) == b'-' {
            int(out, &rest[..2]);
            sep(out, 0);
            m = &rest[3..];
        }
        int(out, m);
    }

    /// The units of an amount: dot-grouped by `ELQNumConvertBigInteger`,
    /// else slot 2.
    fn units(&self, out: &mut Vec<u8>, s: &[u8]) {
        if q::is_big_integer(s) {
            let at = out.len();
            q::convert_big_integer(out, at, s, self);
        } else {
            self.cardinal(out, s);
        }
    }

    /// 0x100028c0: an amount of `cur`: one decimal made two; with two
    /// decimals the units and the currency, then the cents as a cardinal
    /// without a name (`negen en veertig euro negentig`) or, below ten,
    /// `en` and the digit and the cent name; only cents for `0,xx`; with
    /// more decimals `punt` and slot 6.
    fn money(&self, out: &mut Vec<u8>, text: &[u8], cur: &[u8]) {
        let text = cs(text);
        let cur = cs(cur);
        let Some(p) = text.iter().position(|&c| c == b',') else {
            self.units(out, text);
            self.currency(out, text, cur, false);
            return;
        };
        let units = &text[..p];
        let mut dec = text[p + 1..].to_vec();
        if !dec.is_empty() && dec[0] != b'0' && dec.len() == 1 {
            dec.push(b'0');
        }
        if dec.len() == 2 {
            if units == self.s(ZERO) {
                if dec[0] == b'0' {
                    out.extend_from_slice(self.digit(dec[1]));
                    self.currency(out, &dec[1..], cur, true);
                } else {
                    self.cardinal(out, &dec);
                    out.extend_from_slice(self.word(1));
                    self.currency(out, &dec, cur, true);
                }
                return;
            }
            self.units(out, units);
            self.currency(out, units, cur, false);
            if dec == self.s(ZERO_ZERO) {
                return;
            }
            out.extend_from_slice(self.word(1));
            if dec[0] != b'0' {
                self.cardinal(out, &dec);
                return;
            }
            out.extend_from_slice(self.s(EN));
            out.extend_from_slice(self.word(1));
            out.extend_from_slice(self.digit(dec[1]));
            self.currency(out, &dec[1..], cur, true);
            return;
        }
        self.units(out, units);
        for id in [1, 3, 1] {
            out.extend_from_slice(self.word(id));
        }
        self.decimals(out, &dec, false);
        self.currency(out, &dec, cur, false);
    }

    /// Adds `min`/`plus` for a sign flag pair.
    fn sign(&self, out: &mut Vec<u8>, minus: u8, plus: u8) {
        if minus == 1 {
            q::add_separator(out, 0, self.word(7));
        } else if plus == 1 {
            q::add_separator(out, 0, self.word(8));
        }
    }

    /// 0x10005cb0: the object's reading.
    fn render(&self, o: &Obj) -> Vec<u8> {
        let mut out = Vec::new();
        let sep = |out: &mut Vec<u8>, id: u32| q::add_separator(out, 0, self.word(id));
        match o.u32(0) {
            0 => q::convert_real_number(&mut out, 0, &o.num(), self),
            1 => {
                self.digits(&mut out, &o.cstr(4), o.b(0x4c) != 0);
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
            8 => {
                if matches!(o.u32(0x88), 8 | 0xe) {
                    self.date(&mut out, o.u32(0x98), o.u32(0x9c), o.u32(0xa0));
                }
            }
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
                let mut t = o.cstr(4);
                if o.b(0x5a) != 0 {
                    for c in t.iter_mut() {
                        *c = match *c {
                            b',' => b'.',
                            b'.' => b',',
                            c => c,
                        };
                    }
                }
                self.money(&mut out, &t, &o.cstr(0x4c));
            }
            _ => {}
        }
        out
    }
}

/// `LesIniNumeri` slot 9 (0x10006820): the reading of the number token
/// `toks[i]` for the engine's reading type `kind` (5/6 ordinal: mode 4,
/// +0x48 5/6) and date order `dmy`, and how many following tokens it used.
/// Mode 0xc (money) when a currency sign is next to the number or one of
/// `euro dollars dollar eur usd USD JPY jpy gbp GBP yen EUR` (or `livre`
/// with `sterling` three tokens on) follows.
pub(super) fn leggi_numero(nl: &Dutch, toks: &mut [Tok], i: usize, kind: u32, dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
    let text = toks.get(i)?.text.clone();
    let mut o = Obj([0; 0xd4]);
    if kind == 5 || kind == 6 {
        o.set_u32(0x44, 4);
        o.set_u32(0x48, if kind == 5 { 5 } else { 6 });
    } else {
        o.set_u32(0x44, kind);
    }
    let next = tok_text(toks, i + 1);
    let named = |a: u32| next == nl.s(a);
    let money = q::is_currency(next)
        || (i > 0 && q::is_currency(tok_text(toks, i - 1)))
        || named(EURO[0])
        || named(DOLLAR[1])
        || named(DOLLAR[0])
        || (named(LIVRE) && tok_text(toks, i + 3) == nl.s(STERLING))
        || [0x1001405c, 0x1001404c, 0x10014050, 0x10014048, 0x10014044, 0x10014028, 0x1001402c, 0x10012c20, 0x10014060].into_iter().any(named);
    if money {
        o.set_u32(0x44, 0xc);
    }
    o.set_cstr(4, &text);
    // 0x10005bd0: the mode's own recognizer first (mode 8 keeps the order)
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
        0 => nl.rec_date(&mut c18, toks, i, dmy, &mut o),
        1 => nl.rec_time(&mut o),
        2 => nl.rec_ordinal(&mut o),
        3 => nl.rec_currency(&mut c18, toks, i, &mut o),
        4 => nl.rec_real(&mut o),
        5 => nl.rec_fraction(&mut o),
        6 => nl.rec_phone(&mut o),
        7 => nl.rec_integer(&mut o),
        _ => nl.rec_digits(&mut o),
    });
    if !hit {
        o.reset(&text);
    }
    Some((nl.render(&o), c18))
}

impl NumSlots for Dutch {
    /// Slot 0 (0x10002820).
    fn word(&self, id: u32) -> &[u8] {
        Dutch::word(self, id)
    }
    /// Slot 1 (0x100027a0): a telephone number: its area code, then the
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
    /// Slot 2 (0x10006220).
    fn cardinal(&self, out: &mut Vec<u8>, digits: &[u8]) {
        let s = self.cardinal_c(digits);
        out.extend(s);
    }
    /// Slot 3 (0x100061e0): `nul`, `één`, else 0x100051e0's word.
    fn digit(&self, d: u8) -> &[u8] {
        match d {
            b'0' => self.word(0xf),
            b'1' => self.word(0x10),
            _ => self.n_word(d as i8 as i32 - 0x30),
        }
    }
    /// Slot 4 (0x100027f0): digit by digit unless a cardinal of 2–12
    /// digits (2–3 when it had leading zeros).
    fn by_digits(&self, flag: bool, len: usize, lead0: bool) -> bool {
        !(!flag && len < 13 && (len < 4 || !lead0) && len != 1)
    }
    /// Slot 5 (0x10004fd0): the currency's unit or cent name between
    /// separators (singular after `1`).
    fn currency(&self, out: &mut Vec<u8>, amount: &[u8], currency: &[u8], cents: bool) {
        let Some(k) = (0..8).find(|&k| self.s(self.pe.u32(CURRENCIES + 0x14 * k)) == cs(currency)) else {
            return;
        };
        // the separators are added to the empty tail of `out` (0x10005061,
        // 0x100051a8): always appended
        out.extend_from_slice(self.word(1));
        let one = cs(amount) == self.s(ONE);
        let f = match (cents, one) {
            (false, true) => 4,
            (false, false) => 8,
            (true, true) => 0xc,
            (true, false) => 0x10,
        };
        out.extend_from_slice(self.s(self.pe.u32(CURRENCIES + 0x14 * k + f)));
        out.extend_from_slice(self.word(1));
    }
    /// Slot 6 (0x10006d30): decimals: leading zeros as `nul`, then up to
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
                        out.push(c);
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

    /// Replays every slot-9 call of the original traced in `numeri.les`
    /// (`LOQ_NUM_ONLY`), comparing the reading, the tokens used and the
    /// rewritten tokens.
    #[test]
    fn dutch_slot9_trace() {
        let root = std::env::var("QQ_NL_LTTS7").unwrap_or_else(|_| "/tmp/qqv/LTTS7".into());
        let Ok(dll) = std::fs::read(format!("{root}/bin/LTTS7Dutch.dll")) else { return };
        let files = crate::lang::ModuleFiles { dll, mother_tongue: "Dutch".into(), ..Default::default() };
        let nl = Dutch::from_files(&files).expect("LTTS7Dutch.dll");
        let text: String = include_bytes!("numeri.les").iter().map(|&c| c as char).collect();
        let lines: Vec<&str> = text.lines().collect();
        let (mut calls, mut bad) = (0, 0);
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
            let (got, used) = leggi_numero(&nl, &mut toks, idx, kind, &dmy).unwrap();
            // the trace lists the tokens up to the first emptied one
            let texts: Vec<Vec<u8>> = toks.iter().map(|t| t.text.clone()).take_while(|t| !t.is_empty()).collect();
            if got != ret || used != c18 || (!after.is_empty() && texts != after) {
                bad += 1;
                eprintln!(
                    "{:?} kind {kind}: got {:?} {used}, want {:?} {c18}{}",
                    String::from_utf8_lossy(&toks[idx].text),
                    String::from_utf8_lossy(&got),
                    String::from_utf8_lossy(&ret),
                    if texts != after && !after.is_empty() { " (tokens differ)" } else { "" }
                );
            }
        }
        eprintln!("{bad} of {calls} slot-9 calls differ");
        assert_eq!(bad, 0);
    }
}
