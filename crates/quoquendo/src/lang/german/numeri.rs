//! German number reading: `LesIniNumeri` (0x10007100) fills slots 0–7 and
//! 9; slot 9 (0x10006c20) is the reader the engine calls for a NUMBER token
//! (MECHANICS.md §4d "Numbers").
//!
//! Slot 9 classifies the token with nine recognizers (0x10005f20 sets their
//! order, the reading mode's own one first), each filling the module's
//! number object (`Obj`, 0xd4 bytes plus the module's byte at +0xd4), and
//! renders it (0x10006000). Every table is read from the DLL.

use super::German;
use crate::lang::common::Tok;
use crate::lang::elqnum::{self as q, Date, NumObj, NumSlots};

/// Slot 0's `{ id, char * }` pairs (0x53 of them); `""` (0x10026b34) for
/// an id it lacks.
const WORDS: u32 = 0x10027798;
const EMPTY: u32 = 0x10026b34;
/// Currency names slot 9 recognizes after a number: euro euros dollar
/// pfunde usd gbp jpy pfund eur yen.
const CURRENCY_NAMES: [u32; 10] = [0x10026b68, 0x10026b44, 0x10026b90, 0x10026b74, 0x100280a4, 0x1002809c, 0x100280a0, 0x10026b7c, 0x100280a8, 0x10026b60];
/// 0x100024a0's prepositions (am vom zum bis im dem den des einen einem):
/// a day after one of them is read in the dative (`n` appended).
const PREPOSITIONS: u32 = 0x100272f0;
/// 0x100025a0's area codes (0xf7 sorted strings), searched by
/// `ELQSearchString` flag 2 (an entry that starts the digits).
const AREA_CODES: u32 = 0x10027318;
/// Slot 5's currencies: 8 × `{ code, unit, units, cent, cents }`.
const CURRENCIES: u32 = 0x100276f8;
/// 0x10002e50's month names: 31 × `{ name, number }`.
const MONTHS: u32 = 0x10027a30;
/// 0x10002e80's words before a bare year: `bis im jahr jahre jahren seit
/// von zwischen`.
const YEAR_WORDS: u32 = 0x10027b28;
/// `jahr jahre jahren` (searched by suffix, flag 8).
const JAHR: u32 = 0x10027b48;
/// 0x10001e30's ordinals 1–19: (number, ordinal) string addresses.
const ORDINALS: [(u32, u32); 19] = [
    (0x10027d8c, 0x10027d84),
    (0x10027d80, 0x10027d78),
    (0x10027d74, 0x10027d6c),
    (0x10027d68, 0x10027d60),
    (0x10027d5c, 0x10027d54),
    (0x10027d50, 0x10027d48),
    (0x10027d44, 0x10027d3c),
    (0x10027d38, 0x10027d30),
    (0x10027d2c, 0x10027d24),
    (0x10027d20, 0x10027d18),
    (0x10027d14, 0x10027d0c),
    (0x10027d08, 0x10027d00),
    (0x10027cfc, 0x10027cf0),
    (0x10027cec, 0x10027ce0),
    (0x10027cdc, 0x10027cd0),
    (0x10027ccc, 0x10027cc0),
    (0x10027cbc, 0x10027cb0),
    (0x10027cac, 0x10027ca0),
    (0x10027c9c, 0x10027c90),
];
/// 0x10001740's fractions in its order (the four after the first compare
/// `1/2` again and are never reached): (fraction, reading).
const FRACTIONS: [(u32, u32); 14] = [
    (0x10027c88, 0x10027c7c),
    (0x10027c48, 0x10027c3c),
    (0x10027c38, 0x10027c2c),
    (0x10027c28, 0x10027c1c),
    (0x10027c18, 0x10027c08),
    (0x10027c04, 0x10027bf8),
    (0x10027bf4, 0x10027be8),
    (0x10027be4, 0x10027bd8),
    (0x10027bd0, 0x10027bc4),
    (0x10027bbc, 0x10027bac),
    (0x10027ba4, 0x10027b94),
    (0x10027b8c, 0x10027b7c),
    (0x10027b78, 0x10027b68),
    (0x10027b64, 0x10027b54),
];

/// The module's number object: `NumObj`'s 0xd4 bytes and the byte after
/// them (+0xd4, the dative flag), which the module copies along.
#[derive(Clone)]
struct Obj([u8; 0xd8]);

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
        n.0.copy_from_slice(&self.0[..0xd4]);
        n
    }
    /// 0x10003c90: back to a plain number of `text` (`ELQNumSetDefaultType`),
    /// the dative flag cleared.
    fn reset(&mut self, text: &[u8]) {
        self.set_b(0xd4, 0);
        let mut n = self.num();
        q::set_default_type(&mut n, text);
        self.0[..0xd4].copy_from_slice(&n.0);
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

/// 0x10002e20: four digits.
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

/// 0x10001650 (count 0xf): the index of the token `off` tokens from `idx`,
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

impl German {
    fn word(&self, id: u32) -> &[u8] {
        (0..0x53).find(|&k| self.pe.u32(WORDS + 8 * k) == id).map(|k| self.s(self.pe.u32(WORDS + 8 * k + 4))).unwrap_or_else(|| self.s(EMPTY))
    }

    /// 0x100056a0: the word of 1–20 and the tens 30–90, else `""`.
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

    /// 0x10005b20: the two digits at `p` (`ein und zwanzig`), `""` for 00.
    fn two(&self, p: &[u8]) -> Vec<u8> {
        let pair = [at(p, 0), at(p, 1)];
        let n = q::atol(&pair);
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

    /// 0x100058c0: the hundreds digit at `p` appended to `s`; false for 0.
    fn hundreds(&self, s: &mut Vec<u8>, p: &[u8]) -> bool {
        let d = at(p, 0);
        match d {
            b'0' => return false,
            b'1' => {
                s.extend_from_slice(self.word(0x3a));
                q::add_separator(s, 0, self.word(1));
            }
            b'2' => s.extend_from_slice(self.word(0x3e)),
            b'3' => s.extend_from_slice(self.word(0x3f)),
            b'5' => s.extend_from_slice(self.word(0x41)),
            b'7' => {
                s.extend_from_slice(self.word(0x1e));
                s.extend_from_slice(self.word(0x3c));
            }
            b'9' => {
                s.extend_from_slice(self.word(0x21));
                s.extend_from_slice(self.word(0x3c));
            }
            _ => {
                s.extend_from_slice(self.n_word(d as i8 as i32 - 0x30));
                s.extend_from_slice(self.word(0x3c));
            }
        }
        if d != b'1' {
            q::add_separator(s, 0, self.word(1));
        }
        true
    }

    /// Slot 2 (0x10006590): the cardinal of a digit string, position by
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
        if len >= 4 {
            let al = d(p);
            if !(al == b'0' && f11) {
                if al == b'1' && !f10 && !f12 {
                    let bl = d(p + 1) != b'0';
                    f13 = d(p + 2) != b'0';
                    f12 = d(p + 3) != b'0';
                    s.extend_from_slice(self.word(0x47));
                    if (bl || (f13 && f12)) && !(!f13 && !f12 && d(p + 1) == b'1') {
                        q::add_separator(&mut s, 0, w1);
                    }
                } else {
                    if !f10 {
                        s.extend_from_slice(self.n_word(al as i8 as i32 - 0x30));
                    }
                    s.extend_from_slice(self.word(if f1b { 0x47 } else { 0x48 }));
                    q::add_separator(&mut s, 0, if !f12 || f1b { w1 } else { w0 });
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
                s.extend_from_slice(self.digit(b'1'));
            } else {
                s.extend(self.two(rest(p)));
            }
        }
        if !w0.is_empty() && s.ends_with(w0) {
            s.truncate(s.len() - w0.len());
        }
        s
    }

    /// 0x100025a0: the length of the area code the digits start with, 0
    /// when none.
    fn area_code(&self, digits: &[u8]) -> usize {
        let digits = cs(digits);
        match self.search_string(digits, AREA_CODES, 2, 0xf7) {
            Some(k) => self.s(self.pe.u32(AREA_CODES + 4 * k)).len(),
            None => 0,
        }
    }

    /// 0x100024a0: whether the token is (the start of) one of the
    /// prepositions after which a day is dative.
    fn dative(&self, t: &[u8]) -> bool {
        let lc: Vec<u8> = cs(t).iter().map(|&c| self.lower(c)).collect();
        if lc.is_empty() {
            return false;
        }
        (0..10).any(|k| {
            let e = self.s(self.pe.u32(PREPOSITIONS + 4 * k));
            let mut j = 0;
            loop {
                if self.unmark(lc[j]) != at(e, j) {
                    return false;
                }
                if j + 1 == lc.len() {
                    return true;
                }
                j += 1;
            }
        })
    }

    /// 0x10002e50: a month name's number, 0 for none.
    fn month_no(&self, t: &[u8]) -> u32 {
        (0..31).find(|&k| ieq(self.s(self.pe.u32(MONTHS + 8 * k)), t)).map(|k| self.pe.u32(MONTHS + 8 * k + 4)).unwrap_or(0)
    }

    /// Slot 7 (0x100026d0): `ELQNumExtractDate` in the order `dmy`, then
    /// its check with two-digit years from 80 (from 0 after `.`).
    fn slot7(&self, dmy: &[u8], text: &[u8], o: &mut Obj) -> bool {
        let mut d = Date::default();
        let mut ok = q::extract_date(dmy, text, &mut d);
        if ok {
            let min = if d.sep == b'.' { 0 } else { 0x50 };
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
        let ok = q::extract_date(b"y", text, &mut d);
        o.set_u32(0x98, d.day);
        o.set_u32(0xa0, d.year);
        ok
    }

    /// 0x10002e80: a day before a month name (and a year after it), or a
    /// year after `bis`, after `im Jahr`…, after `und`/`bis` and a number,
    /// or after a month name.
    fn date_words(&self, c18: &mut usize, toks: &[Tok], i: usize, text: &[u8], o: &mut Obj) -> bool {
        let n1 = tok_text(toks, tok_at(toks, i, 1)).to_vec();
        let n2 = tok_text(toks, tok_at(toks, i, 2)).to_vec();
        let p1 = tok_text(toks, tok_at(toks, i, -1)).to_vec();
        let p2 = tok_text(toks, tok_at(toks, i, -2)).to_vec();
        o.set_u32(0x98, 0);
        o.set_u32(0x9c, 0);
        o.set_u32(0xa0, 0);
        let m = self.month_no(&n1);
        o.set_u32(0x9c, m);
        if (1..=12).contains(&m) {
            let mut d = Date::default();
            let ok = q::extract_date(b"d", text, &mut d);
            o.set_u32(0x98, d.day);
            if ok {
                *c18 = 1;
                if four_digits(&n2) {
                    let mut y = Date::default();
                    if q::extract_date(b"y", &n2, &mut y) {
                        *c18 = 2;
                    }
                    o.set_u32(0xa0, y.year);
                }
                return true;
            }
        }
        if ieq(&n1, self.s(self.pe.u32(CURRENCIES - 4))) && four_digits(text) && self.year_of(text, o) {
            return true;
        }
        if i == 0 {
            return false;
        }
        for k in 0..8 {
            if ieq(&p1, self.s(self.pe.u32(YEAR_WORDS + 4 * k))) && four_digits(text) && self.year_of(text, o) {
                return true;
            }
        }
        if self.search_string(&p1, JAHR, 8, 3) == Some(0) && four_digits(text) && self.year_of(text, o) {
            return true;
        }
        if i > 2 && (ieq(&p1, self.s(0x10027da8)) || ieq(&p1, self.s(0x100272e4))) && tok_type(toks, i - 2) == 1 && four_digits(text) && self.year_of(text, o) {
            return true;
        }
        let mut e = &p1;
        if i > 1 && [0x10027da4, 0x10027da0, 0x10027d9c].iter().any(|&a| ieq(&p1, self.s(a))) {
            e = &p2;
        }
        let m = self.month_no(e);
        (1..=12).contains(&m) && four_digits(text) && self.year_of(text, o)
    }

    /// Recognizer 0 (0x10003cb0): a date (kind 8).
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
                if t.len() != 2 {
                    if i > 0 {
                        o.set_b(0xd4, self.dative(tok_text(toks, i - 1)) as u8);
                    }
                    return true;
                }
            } else if mode == 8 {
                let mut d = Date::default();
                let ok = q::format_vxml10_date(&t, &mut d);
                o.set_u32(0x98, d.day);
                o.set_u32(0x9c, d.month);
                o.set_u32(0xa0, d.year);
                if ok && ieq(dmy, self.s(0x1001e62c)) {
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

    /// Recognizer 1 (0x10003ec0): a time (kind 7).
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

    /// Recognizer 2 (0x10004050): an ordinal (kind 4): in ordinal mode,
    /// or before `°`/`º` (not `C`) or an apostrophe not followed by a number.
    fn rec_ordinal(&self, toks: &[Tok], i: usize, o: &mut Obj) -> bool {
        let local = o.cstr(4);
        o.set_u32(0, 4);
        o.strip_sign(&local);
        let (dots, commas) = dots_commas(&o.cstr(4));
        if dots + commas == 0 {
            if o.u32(0x44) == 4 {
                if i > 1 {
                    let pt = tok_text(toks, i - 1);
                    if pt == self.s(0x10028098) || pt == self.s(0x10028084) {
                        o.set_b(0xd4, self.dative(tok_text(toks, i - 2)) as u8);
                    }
                }
                return true;
            }
            let nt = tok_text(toks, i + 1);
            if (nt == self.s(0x10028080) || nt == self.s(0x1002807c)) && !ieq(tok_text(toks, i + 2), self.s(0x1001e600)) {
                o.set_u32(0x48, 5);
                return true;
            }
            if nt.first() == Some(&b'\'') && tok_type(toks, i + 2) != 1 {
                o.set_u32(0x48, 5);
                return true;
            }
        }
        o.reset(&local);
        false
    }

    /// Recognizer 3 (0x100042b0): an amount of money (kind 0xc) with a
    /// currency sign before or after it or a currency name after it (which
    /// becomes `n` and is consumed).
    fn rec_currency(&self, c18: &mut usize, toks: &mut [Tok], i: usize, o: &mut Obj) -> bool {
        o.set_b(0x4c, 0);
        o.set_u32(0, 0xc);
        o.set_u32(0x48, 0xf);
        for k in [0x56, 0x57, 0x58, 0x5a] {
            o.set_b(k, 0);
        }
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
        let code = if q::is_currency(&nt) {
            nt.clone()
        } else if [0x10026b68, 0x10026b44, 0x100280a8].iter().any(|&a| ieq(&nt, self.s(a))) {
            self.s(0x10026b70).to_vec()
        } else if [0x10026b90, 0x10026b54, 0x100280a4].iter().any(|&a| ieq(&nt, self.s(a))) {
            self.s(0x10026b98).to_vec()
        } else if [0x10026b60, 0x100280a0].iter().any(|&a| ieq(&nt, self.s(a))) {
            self.s(0x10026b64).to_vec()
        } else if nt == self.s(0x10026b7c) || nt == self.s(0x10026b74) || ieq(&nt, self.s(0x1002809c)) {
            self.s(0x10026b84).to_vec()
        } else {
            o.reset(&local);
            return false;
        };
        o.set_cstr(0x4c, &code);
        if let Some(t) = toks.get_mut(i + 1) {
            t.text = b"n".to_vec();
        }
        *c18 = 1;
        true
    }

    /// Recognizer 4 (0x10004630): a real number (kind 9), only in mode 9.
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

    /// Recognizer 5 (0x100048e0): a fraction 0x10001b40 knows (kind 0xb).
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

    /// 0x10001740 (and 0x10001b40, the same list): a fraction's reading.
    fn fraction(&self, t: &[u8]) -> Option<&[u8]> {
        FRACTIONS.iter().find(|p| t == self.s(p.0)).map(|p| self.s(p.1))
    }

    /// Recognizer 6 (0x10004a00): a telephone number (kind 1): more than
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
                    if t.iter().any(|c| self.s(0x100280ac).contains(c)) || t.len() <= 6 || self.area_code(&t) == 0 {
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

    /// Recognizer 7 (0x10004df0): an integer (kind 0xa): up to five digits,
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
        let seps = |s: &[u8]| s.iter().any(|c| self.s(0x100280ac).contains(c));
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

    /// Recognizer 8 (0x10004c30): a code read digit by digit (kind 2), in
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

    /// 0x10001e30: the ordinal of a digit string (`dritte`, `ein und
    /// zwanzigste`).
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
            if out.last() == Some(&b' ') {
                out.pop();
            }
            out.extend_from_slice(self.s(0x10027c8c));
        }
        out
    }

    /// 0x10005c50: a date: the day's ordinal (dative after a preposition),
    /// the month, the year (`neunzehn hundert neunzig`, `zweitàusend`, an
    /// integer after 2000).
    fn date(&self, out: &mut Vec<u8>, day: u32, month: u32, year: u32, dative: bool) {
        if day > 0 {
            let mut b = self.ordinal(&ltoa(day));
            if dative {
                b.extend_from_slice(self.s(0x1001e3e8));
            }
            out.extend(b);
            q::add_separator(out, 0, self.word(1));
        }
        if (1..=12).contains(&month) {
            out.extend_from_slice(self.word(0x50 + month));
            q::add_separator(out, 0, self.word(1));
        }
        if year > 0 {
            let y = ltoa(year);
            if year > 2000 {
                let at = out.len();
                q::convert_integer(out, at, &y, false, self);
            } else if year == 2000 {
                out.extend_from_slice(self.s(0x100280cc));
            } else {
                out.extend(self.two(&y[..2.min(y.len())]));
                out.extend_from_slice(self.s(0x100280c0));
                out.extend(self.two(y.get(2..).unwrap_or(b"")));
            }
        }
    }

    /// 0x100051b0: a time `h:mm[:ss]` (`sieben Uhr fünf und vierzig`).
    fn time(&self, out: &mut Vec<u8>, text: &[u8]) {
        let seps = self.s(0x100280bc);
        let buf = cs(text);
        let Some(p) = buf.iter().position(|c| seps.contains(c)) else {
            return;
        };
        let h = &buf[..p];
        let h = if at(h, 0) == b'0' && (b'1'..=b'9').contains(&at(h, 1)) { &h[1..] } else { h };
        q::convert_integer(out, 0, h, false, self);
        out.extend_from_slice(self.s(0x100280b4));
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
            if dl == b'0' {
                if at(min, 1) == b'0' {
                    if !sec00 {
                        sep(out, 1);
                        int(out, sec);
                        sep(out, 0x5d);
                    }
                    return;
                }
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
        if at(rest, 2) == b'-' {
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

    /// 0x10002740: an amount of `cur` (`neun und vierzig euro und neunzig
    /// cent`).
    fn money(&self, out: &mut Vec<u8>, text: &[u8], cur: &[u8]) {
        let text = cs(text);
        let cur = cs(cur);
        let Some(p) = text.iter().position(|&c| self.s(0x10027d98).contains(&c)) else {
            if text == self.s(0x10027d8c) && [0x10026b98, 0x10026b70, 0x10026b64].iter().any(|&a| cur == self.s(a)) {
                out.extend_from_slice(self.word(0x11));
            } else if text == self.s(0x10027d8c) && cur == self.s(0x10026b84) {
                out.extend_from_slice(self.word(0x12));
            } else {
                self.units(out, text);
            }
            self.currency(out, text, cur, false);
            return;
        };
        let units = &text[..p];
        let dec = &text[p + 1..];
        let cents = |out: &mut Vec<u8>| {
            if dec[0] == b'0' {
                if dec[1] == b'1' {
                    out.extend_from_slice(self.word(0x11));
                } else {
                    out.extend_from_slice(self.digit(dec[1]));
                }
                self.currency(out, &dec[1..], cur, true);
            } else {
                self.cardinal(out, dec);
                self.currency(out, dec, cur, true);
            }
        };
        if dec.len() == 2 {
            if units == self.s(0x10027d94) {
                cents(out);
                return;
            }
            self.units(out, units);
            self.currency(out, units, cur, false);
            if dec == self.s(0x10027d90) {
                return;
            }
            for id in [1, 0xd, 1] {
                out.extend_from_slice(self.word(id));
            }
            cents(out);
            return;
        }
        self.units(out, units);
        for id in [1, 3, 1] {
            out.extend_from_slice(self.word(id));
        }
        self.decimals(out, dec, false);
        self.currency(out, dec, cur, false);
    }

    /// Adds `minus`/`plus` for a sign flag pair.
    fn sign(&self, out: &mut Vec<u8>, minus: u8, plus: u8) {
        if minus == 1 {
            q::add_separator(out, 0, self.word(7));
        } else if plus == 1 {
            q::add_separator(out, 0, self.word(8));
        }
    }

    /// 0x10006000: the object's reading.
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
                    self.decimals(&mut out, &o.cstr(0x8c), true);
                }
            }
            4 => {
                out = self.ordinal(&o.cstr(4));
                if o.b(0xd4) != 0 {
                    out.extend_from_slice(self.s(0x1001e3e8));
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
                    self.date(&mut out, o.u32(0x98), o.u32(0x9c), o.u32(0xa0), o.b(0xd4) != 0);
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

/// `LesIniNumeri` slot 9 (0x10006c20): the reading of the number token
/// `toks[i]` for the engine's reading type `kind` (5/6 ordinal: mode 4,
/// +0x48 5/6) and date order `dmy`, and how many following tokens it used.
pub(super) fn leggi_numero(de: &German, toks: &mut [Tok], i: usize, kind: u32, dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
    let text = toks.get(i)?.text.clone();
    let mut o = Obj([0; 0xd8]);
    o.reset(b"");
    if kind == 5 || kind == 6 {
        o.set_u32(0x44, 4);
        o.set_u32(0x48, if kind == 5 { 5 } else { 6 });
    } else {
        o.set_u32(0x44, kind);
    }
    let next = tok_text(toks, i + 1);
    if q::is_currency(next) || (i > 0 && q::is_currency(tok_text(toks, i - 1))) || CURRENCY_NAMES.iter().any(|&a| ieq(de.s(a), next)) {
        o.set_u32(0x44, 0xc);
    }
    o.set_cstr(4, &text);
    // 0x10005f20: the mode's own recognizer first
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
        0 => de.rec_date(&mut c18, toks, i, dmy, &mut o),
        1 => de.rec_time(&mut o),
        2 => de.rec_ordinal(toks, i, &mut o),
        3 => de.rec_currency(&mut c18, toks, i, &mut o),
        4 => de.rec_real(&mut o),
        5 => de.rec_fraction(&mut o),
        6 => de.rec_phone(&mut o),
        7 => de.rec_integer(&mut o),
        _ => de.rec_digits(&mut o),
    });
    if !hit {
        o.reset(&text);
    }
    Some((de.render(&o), c18))
}

impl NumSlots for German {
    /// Slot 0 (0x100026a0).
    fn word(&self, id: u32) -> &[u8] {
        German::word(self, id)
    }
    /// Slot 1 (0x10002620): a telephone number: its area code, then the
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
    /// Slot 2 (0x10006590).
    fn cardinal(&self, out: &mut Vec<u8>, digits: &[u8]) {
        let s = self.cardinal_c(digits);
        out.extend(s);
    }
    /// Slot 3 (0x10006550): `null`, `eins`, else 0x100056a0's word.
    fn digit(&self, d: u8) -> &[u8] {
        match d {
            b'0' => self.word(0xf),
            b'1' => self.word(0x10),
            _ => self.n_word(d as i8 as i32 - 0x30),
        }
    }
    /// Slot 4 (0x10002670): digit by digit unless a cardinal of 2–12
    /// digits (2–3 when it had leading zeros).
    fn by_digits(&self, flag: bool, len: usize, lead0: bool) -> bool {
        !(!flag && len < 13 && (len < 4 || !lead0) && len != 1)
    }
    /// Slot 5 (0x10005490): the currency's unit or cent name between
    /// spaces (singular after `1`).
    fn currency(&self, out: &mut Vec<u8>, amount: &[u8], currency: &[u8], cents: bool) {
        let Some(k) = (0..8).find(|&k| self.s(self.pe.u32(CURRENCIES + 0x14 * k)) == cs(currency)) else {
            return;
        };
        out.extend_from_slice(self.word(1));
        let one = cs(amount) == self.s(0x10027d8c);
        let f = match (cents, one) {
            (false, true) => 4,
            (false, false) => 8,
            (true, true) => 0xc,
            (true, false) => 0x10,
        };
        out.extend_from_slice(self.s(self.pe.u32(CURRENCIES + 0x14 * k + f)));
        out.extend_from_slice(self.word(1));
    }
    /// Slot 6 (0x10006f40): decimals: leading zeros as `null`, then up to
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

    /// Replays every slot-9 call traced by `/tmp/qq-num-de/*.les`
    /// (`LOQ_NUM_ONLY`), comparing the reading, the tokens used and the
    /// rewritten tokens.
    #[test]
    #[ignore]
    fn replay_slot9() {
        let Some(de) = super::super::fixtures::german() else { return };
        let mut files: Vec<_> = std::fs::read_dir("/tmp/qq-num-de").into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "les")).collect();
        files.sort();
        let (mut calls, mut bad) = (0, 0);
        for f in files {
            let raw = std::fs::read(&f).unwrap();
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
                let (got, used) = leggi_numero(&de, &mut toks, idx, kind, &dmy).unwrap();
                let texts: Vec<Vec<u8>> = toks.iter().map(|t| t.text.clone()).collect();
                if got != ret || used != c18 || (!after.is_empty() && texts != after) {
                    bad += 1;
                    eprintln!(
                        "{}: {:?} kind {kind}: got {:?} {used}, want {:?} {c18}{}",
                        f.display(),
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
