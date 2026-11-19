//! French number reading: `LesIniNumeri` (0x10006480) fills slots 0–7 and
//! 9; slot 9 (0x10005db0) is the reader the engine calls for a NUMBER token
//! (MECHANICS.md §4 "French numbers").
//!
//! Slot 9 has the German reader's shape: nine recognizers (0x10005050 sets
//! their order, the reading mode's own one first), each filling the
//! module's number object (`NumObj`, 0xd4 bytes), then the renderer
//! (0x10005130). Every table and string is read from the DLL.

use super::French;
use crate::lang::common::Tok;
use crate::lang::elqnum::{self as q, Date, NumObj, NumSlots};

/// Slot 0's `{ id, char * }` pairs (0x53 of them); `""` (0x1001c3f2) for
/// an id it lacks.
const WORDS: u32 = 0x1001d020;
const EMPTY: u32 = 0x1001c3f2;
/// Names after a number that make slot 9 read it as money (mode 0xc),
/// compared case-sensitively: euro dollars dollar livre (with `sterling`
/// three tokens on) eur usd USD JPY jpy gbp GBP yen EUR.
const LIVRE: u32 = 0x1001d6ec;
const STERLING: u32 = 0x1001d6e0;
const CURRENCY_NAMES: [u32; 12] = [0x1001c428, 0x1001c458, 0x1001c460, 0x1001d704, 0x1001d6fc, 0x1001d700, 0x1001d6f8, 0x1001d6f4, 0x1001d6d8, 0x1001d6dc, 0x1001c40c, 0x1001d708];
/// `:/-+`, the characters that keep a token from being an integer or a
/// telephone number.
const NOT_INTEGER: u32 = 0x1001d70c;
/// 0x10001cb0's area codes (0xf7 sorted strings), searched by
/// `ELQSearchString` flag 2 (an entry that starts the digits).
const AREA_CODES: u32 = 0x1001cba0;
/// Slot 5's currencies: 8 × `{ code, unit, units, cent, cents }`.
const CURRENCIES: u32 = 0x1001cf80;
/// `1`, `0`, `00`, `et`, `£`
const ONE: u32 = 0x1001d4f4;
const ZERO: u32 = 0x1001d500;
const ZERO2: u32 = 0x1001d4fc;
const ET: u32 = 0x1001d4f8;
const POUND: u32 = 0x1001c444;
/// 0x10001610's ordinals 1–20: (number, ordinal); `premièr` takes `e`
/// (0x1001d4e8) unless a suffix followed; others are a cardinal + `ième`.
const ORDINALS: [(u32, u32); 20] = [
    (0x1001d4f4, 0x1001d4ec),
    (0x1001d4e4, 0x1001d4d8),
    (0x1001d4d4, 0x1001d4c8),
    (0x1001d4c4, 0x1001d4b8),
    (0x1001d4b4, 0x1001d4a8),
    (0x1001d4a4, 0x1001d49c),
    (0x1001d498, 0x1001d48c),
    (0x1001d488, 0x1001d47c),
    (0x1001d478, 0x1001d46c),
    (0x1001d468, 0x1001d460),
    (0x1001d45c, 0x1001d454),
    (0x1001d450, 0x1001d444),
    (0x1001d440, 0x1001d434),
    (0x1001d430, 0x1001d424),
    (0x1001d420, 0x1001d414),
    (0x1001d410, 0x1001d404),
    (0x1001d400, 0x1001d3f4),
    (0x1001d3f0, 0x1001d3e4),
    (0x1001d3e0, 0x1001d3d4),
    (0x1001d3d0, 0x1001d3c4),
];
const ORDINAL_E: u32 = 0x1001d4e8;
const IEME: u32 = 0x1001d3bc;
/// 0x10003120's suffixes after a number that make it an ordinal: ème éme
/// eme è e.
const ORDINAL_SUFFIXES: [u32; 5] = [0x1001d6d4, 0x1001d6d0, 0x1001d6cc, 0x1001d59c, 0x1001d4e8];
/// 0x10001000's (and 0x10001320's) fractions: (fraction, reading).
const FRACTIONS: [(u32, u32); 14] = [
    (0x1001d3b8, 0x1001d3b0),
    (0x1001d3ac, 0x1001d39c),
    (0x1001d398, 0x1001d388),
    (0x1001d384, 0x1001d374),
    (0x1001d370, 0x1001d364),
    (0x1001d360, 0x1001d354),
    (0x1001d350, 0x1001d344),
    (0x1001d340, 0x1001d334),
    (0x1001d32c, 0x1001d320),
    (0x1001d318, 0x1001d308),
    (0x1001d300, 0x1001d2f4),
    (0x1001d2ec, 0x1001d2e0),
    (0x1001d2dc, 0x1001d2cc),
    (0x1001d2c8, 0x1001d2b8),
];
/// 0x10002e00: `h` after an hour, `:` joined to it
const H: u32 = 0x1001d6c8;
const COLON: u32 = 0x1001d6c4;
/// 0x10004430: ` heure `, ` heures `, `.,:`
const HEURE: u32 = 0x1001d720;
const HEURES: u32 = 0x1001d714;
const TIME_SEPS: u32 = 0x1001d728;
/// 0x10002ca0: the date order that accepts a VoiceXML date
const VXML_ORDER: u32 = 0x1001d6c0;
/// 0x10003330's currency signs: € $ ¥ (and £ above)
const EURO: u32 = 0x1001c430;
const DOLLAR: u32 = 0x1001c468;
const YEN: u32 = 0x1001c410;

fn cs(s: &[u8]) -> &[u8] {
    &s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())]
}

fn at(s: &[u8], k: usize) -> u8 {
    s.get(k).copied().unwrap_or(0)
}

/// `ELQstricmp(a, b) == 0` (ASCII case folding only).
fn ieq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.eq_ignore_ascii_case(y))
}

fn dots_commas(s: &[u8]) -> (usize, usize) {
    (s.iter().filter(|&&c| c == b'.').count(), s.iter().filter(|&&c| c == b',').count())
}

fn tok_text(toks: &[Tok], k: usize) -> &[u8] {
    toks.get(k).map(|t| &t.text[..]).unwrap_or(b"")
}

fn set_tok(toks: &mut [Tok], k: usize, s: &[u8]) {
    if let Some(t) = toks.get_mut(k) {
        t.text = s.to_vec();
    }
}

/// `strncmp(key, e, e.len())`.
fn strncmp_prefix(key: &[u8], e: &[u8]) -> std::cmp::Ordering {
    for (k, &c) in e.iter().enumerate() {
        let d = at(key, k);
        if d != c {
            return d.cmp(&c);
        }
    }
    std::cmp::Ordering::Equal
}

/// `ELQNumSetDefaultType`: back to a plain number of `text`.
fn reset(o: &mut NumObj, text: &[u8]) {
    q::set_default_type(o, text);
}

/// The recognizers' sign handling: the text without a leading `-`/`+`;
/// returns that sign.
fn strip_sign(o: &mut NumObj, local: &[u8]) -> u8 {
    match local.first() {
        Some(&c @ (b'-' | b'+')) => {
            o.set_cstr(4, &local[1..]);
            c
        }
        _ => {
            o.set_cstr(4, local);
            0
        }
    }
}

impl French {
    fn word(&self, id: u32) -> &[u8] {
        (0..0x53).find(|&k| self.pe.u32(WORDS + 8 * k) == id).map(|k| self.s(self.pe.u32(WORDS + 8 * k + 4))).unwrap_or_else(|| self.s(EMPTY))
    }

    /// 0x10004970: the word of 1–20 and the tens 30–90, else `""`.
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

    /// 0x10004e00: the two digits at `p` (`vingt et un`, `soixante et
    /// onze`, `quatre vingt un`), `""` for 00.
    fn two(&self, p: &[u8]) -> Vec<u8> {
        let (a, b) = (at(p, 0), at(p, 1));
        let n = q::atol(&[a, b]);
        let mut s = Vec::new();
        if n == 0 {
            return s;
        }
        let space = self.word(1);
        if a == b'1' || n < 10 {
            s.extend_from_slice(self.n_word(n));
        } else if a == b'7' || a == b'9' {
            s.extend_from_slice(self.n_word((a as i32 - 0x31) * 10));
            if b == b'1' && a == b'7' {
                s.extend_from_slice(self.word(0xd));
            }
            s.extend_from_slice(space);
            s.extend_from_slice(self.n_word(b as i8 as i32 - 0x26));
        } else {
            s.extend_from_slice(self.n_word((a as i32 - 0x30) * 10));
            match b {
                b'0' => {}
                b'1' if a == b'8' => {
                    s.extend_from_slice(space);
                    s.extend_from_slice(self.word(0x10));
                }
                b'1' => s.extend_from_slice(self.word(0x11)),
                _ => {
                    s.extend_from_slice(space);
                    s.extend_from_slice(self.n_word(b as i8 as i32 - 0x30));
                }
            }
        }
        s
    }

    /// 0x10004b90: the hundreds digit at `p` appended to `s` (`cent`,
    /// `deux cents`, `trois cent`); false for 0.
    fn hundreds(&self, s: &mut Vec<u8>, p: &[u8]) -> bool {
        let d = at(p, 0);
        let zeros = at(p, 1) == b'0' && at(p, 2) == b'0';
        match d {
            b'0' => return false,
            b'1' => {
                s.extend_from_slice(self.word(if zeros { 0x3a } else { 0x3b }));
                q::add_separator(s, 0, self.word(1));
                return true;
            }
            b'2' => s.extend_from_slice(self.word(0x15)),
            b'3' => s.extend_from_slice(self.word(0x17)),
            b'5' => s.extend_from_slice(self.word(0x1b)),
            b'7' => s.extend_from_slice(self.word(0x1e)),
            b'9' => s.extend_from_slice(self.word(0x21)),
            _ => s.extend_from_slice(self.n_word(d as i8 as i32 - 0x30)),
        }
        q::add_separator(s, 0, self.word(1));
        s.extend_from_slice(self.word(if zeros { 0x3c } else { 0x3b }));
        q::add_separator(s, 0, self.word(1));
        true
    }

    /// Slot 2 (0x10005730): the cardinal of a digit string, position by
    /// position from the hundreds of billions, a trailing word 0 removed.
    fn cardinal_c(&self, digits: &[u8]) -> Vec<u8> {
        let digits = cs(digits);
        let len = digits.len();
        let d = |k: usize| at(digits, k);
        let rest = |k: usize| digits.get(k..).unwrap_or(b"");
        let mut s: Vec<u8> = Vec::new();
        let (mut f10, mut f13, mut f1b, mut f12) = (false, false, false, false);
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
        }
        let mut f11 = true;
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
            if al != b'0' || !f11 {
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
            if al != b'0' || !f11 {
                // 0x10005c44 (the digit's word) and 0x10005c76 (`mille`)
                let thousand = |s: &mut Vec<u8>, digit: bool, f1b: bool| {
                    if digit {
                        s.extend_from_slice(self.n_word(al as i8 as i32 - 0x30));
                    }
                    if !f1b {
                        q::add_separator(s, 0, w1);
                        s.extend_from_slice(self.word(0x48));
                    } else {
                        s.extend_from_slice(self.word(0x47));
                    }
                    q::add_separator(s, 0, if !f12 || f1b { w1 } else { w0 });
                };
                if al == b'1' {
                    if f10 {
                        thousand(&mut s, false, f1b);
                    } else if f12 {
                        thousand(&mut s, true, f1b);
                    } else {
                        let bl = d(p + 1);
                        let c3 = d(p + 2) != b'0';
                        let c4 = d(p + 3) != b'0';
                        s.extend_from_slice(self.word(0x47));
                        if (bl != b'0' || (c3 && c4)) && (c3 || c4 || bl != b'1') {
                            q::add_separator(&mut s, 0, w1);
                        }
                    }
                } else {
                    thousand(&mut s, !f10, f1b);
                }
                q::add_separator(&mut s, 0, w1);
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

    /// 0x10001cb0: the length of the area code the digits start with, 0
    /// when none (`ELQSearchString` flag 2: a binary search on prefixes).
    fn area_code(&self, digits: &[u8]) -> usize {
        let key = cs(digits);
        let entry = |k: i64| self.s(self.pe.u32(AREA_CODES + 4 * k as u32));
        let (mut lo, mut hi) = (0i64, 0xf6i64);
        while lo <= hi {
            let mid = (lo + hi) / 2;
            match strncmp_prefix(key, entry(mid)) {
                std::cmp::Ordering::Less => hi = mid - 1,
                std::cmp::Ordering::Greater => lo = mid + 1,
                std::cmp::Ordering::Equal => return entry(mid).len(),
            }
        }
        0
    }

    /// Slot 7 (0x10001de0): `ELQNumExtractDate` in the order `dmy`, then
    /// its check with two-digit years from 80 (from 0 after `/`).
    fn slot7(&self, dmy: &[u8], text: &[u8], o: &mut NumObj) -> bool {
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

    /// Recognizer 0 (0x10002ca0): a date (kind 8).
    fn rec_date(&self, dmy: &[u8], o: &mut NumObj) -> bool {
        o.set_u8(0x8c, 0);
        o.set_u32(0, 8);
        o.set_u32(0x88, 8);
        o.set_u8(0x94, 0);
        o.set_u8(0x48, 0);
        o.set_cstr(0x8c, cs(dmy));
        let local = o.cstr(4).to_vec();
        strip_sign(o, &local);
        let mode = o.mode();
        let t = o.cstr(4).to_vec();
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
                if ok && ieq(dmy, self.s(VXML_ORDER)) {
                    return true;
                }
            }
        }
        reset(o, &local);
        false
    }

    /// Recognizer 1 (0x10002e00): a time (kind 7): an hour 1–24 before
    /// `h` (and the minutes after it, joined by `:`), or `ELQNumIsTime`.
    fn rec_time(&self, c18: &mut usize, toks: &[Tok], i: usize, o: &mut NumObj) -> bool {
        let local = o.cstr(4).to_vec();
        o.set_u32(0, 7);
        o.set_u32(0x48, 7);
        o.set_u8(0x4c, 0);
        for k in 0x8c..0x90 {
            o.set_u8(k, 0);
        }
        match strip_sign(o, &local) {
            b'-' => o.set_u8(0x8d, 1),
            b'+' => o.set_u8(0x8e, 1),
            _ => {}
        }
        let t = o.cstr(4).to_vec();
        let (dots, commas) = dots_commas(&t);
        let n = q::atol(&t);
        if n < 0x19 && n > 0 && tok_text(toks, i + 1) == self.s(H) {
            let m = tok_text(toks, i + 2);
            let mut joined = t.clone();
            joined.extend_from_slice(self.s(COLON));
            if q::is_number(m, b"", true) && q::atol(m) < 0x3c && tok_text(toks, i + 3) != self.s(H) {
                joined.extend_from_slice(m);
                *c18 = 2;
            } else {
                *c18 = 1;
            }
            o.set_cstr(4, &joined);
            return true;
        }
        let mode = o.mode();
        if dots == 0 && commas == 0 && matches!(mode, 0 | 7 | 8) && o.u8(0x8d) == 0 && o.u8(0x8e) == 0 && q::is_time(&t) {
            return true;
        }
        reset(o, &local);
        false
    }

    /// Recognizer 2 (0x10003120): an ordinal (kind 4): in ordinal mode, or
    /// before `ème`/`éme`/`eme`/`è`/`e` (consumed, +0x48 5).
    fn rec_ordinal(&self, c18: &mut usize, toks: &[Tok], i: usize, o: &mut NumObj) -> bool {
        let local = o.cstr(4).to_vec();
        o.set_u32(0, 4);
        strip_sign(o, &local);
        let (dots, commas) = dots_commas(o.cstr(4));
        if dots + commas == 0 {
            if o.mode() == 4 {
                return true;
            }
            let nt = tok_text(toks, i + 1);
            if ORDINAL_SUFFIXES.iter().any(|&a| nt == self.s(a)) {
                o.set_u32(0x48, 5);
                *c18 = 1;
                return true;
            }
        }
        reset(o, &local);
        false
    }

    /// Recognizer 3 (0x10003330): an amount of money (kind 0xc) after a
    /// currency sign, before one or before a currency name (the sign
    /// replaces it and the next token becomes `n`; for `livre sterling`
    /// and `GBP` the token after it).
    fn rec_currency(&self, c18: &mut usize, toks: &mut [Tok], i: usize, o: &mut NumObj) -> bool {
        o.set_u8(0x4c, 0);
        o.set_u32(0, 0xc);
        o.set_u32(0x48, 0xf);
        for k in [0x56, 0x57, 0x58, 0x5a] {
            o.set_u8(k, 0);
        }
        let local = o.cstr(4).to_vec();
        match strip_sign(o, &local) {
            b'-' => o.set_u8(0x56, 1),
            b'+' => o.set_u8(0x57, 1),
            _ => {}
        }
        if dots_commas(o.cstr(4)).0 == 1 {
            o.set_u8(0x5a, 1);
        }
        if i > 0 && q::is_currency(tok_text(toks, i - 1)) {
            let p = tok_text(toks, i - 1).to_vec();
            o.set_cstr(0x4c, &p);
            return true;
        }
        let nt = tok_text(toks, i + 1).to_vec();
        let is = |a: u32| nt == self.s(a);
        let (code, used) = if q::is_currency(&nt) {
            (nt.clone(), i + 1)
        } else if is(0x1001c428) || is(0x1001d708) || is(0x1001d704) {
            (self.s(EURO).to_vec(), i + 1)
        } else if is(0x1001c460) || is(0x1001c458) || is(0x1001d700) || is(0x1001d6fc) {
            (self.s(DOLLAR).to_vec(), i + 1)
        } else if is(0x1001c40c) || is(0x1001d6f8) || is(0x1001d6f4) {
            (self.s(YEN).to_vec(), i + 1)
        } else if (is(LIVRE) && tok_text(toks, i + 2) == self.s(STERLING)) || is(0x1001d6dc) || is(0x1001d6d8) {
            (self.s(POUND).to_vec(), i + 2)
        } else {
            reset(o, &local);
            return false;
        };
        o.set_cstr(0x4c, &code);
        set_tok(toks, used, b"n");
        *c18 = 1;
        true
    }

    /// Recognizer 4 (0x10003870): a real number (kind 9), only in mode 9.
    fn rec_real(&self, o: &mut NumObj) -> bool {
        o.set_u8(0x8c, 0);
        o.set_u32(0, 9);
        o.set_u32(0x48, 9);
        o.set_u8(0x4c, 0);
        o.set_u8(0xcc, 0);
        o.set_u8(0xcd, 0);
        let local = o.cstr(4).to_vec();
        match strip_sign(o, &local) {
            b'-' => o.set_u8(0xcc, 1),
            b'+' => o.set_u8(0xcd, 1),
            _ => {}
        }
        let t = o.cstr(4).to_vec();
        let (dots, commas) = dots_commas(&t);
        let mode = o.mode();
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
                if o.u8(0x4c) != 0 {
                    o.set_u8(0x8c, 0);
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
            if pd < pc && q::is_big_integer(&t[..pc]) && mode == 9 {
                o.set_cstr(0x4c, &t[..pc]);
                o.set_u32(0x48, 3);
                o.set_cstr(0x8c, &t[pc + 1..]);
                true
            } else {
                false
            }
        } else {
            false
        };
        if !ok {
            reset(o, &local);
        }
        ok
    }

    /// 0x10001000 (and 0x10001320, the same list): a fraction's reading.
    fn fraction(&self, t: &[u8]) -> Option<&[u8]> {
        FRACTIONS.iter().find(|p| t == self.s(p.0)).map(|p| self.s(p.1))
    }

    /// Recognizer 5 (0x10003b20): a fraction 0x10001320 knows (kind 0xb).
    fn rec_fraction(&self, o: &mut NumObj) -> bool {
        let local = o.cstr(4).to_vec();
        o.set_u32(0, 0xb);
        o.set_u32(0x48, 0xb);
        o.set_u8(0x4c, 0);
        o.set_u8(0x4d, 0);
        match strip_sign(o, &local) {
            b'-' => o.set_u8(0x4c, 1),
            b'+' => o.set_u8(0x4d, 1),
            _ => {}
        }
        let t = o.cstr(4).to_vec();
        let (dots, commas) = dots_commas(&t);
        let ok = dots + commas == 0 && matches!(o.mode(), 0 | 8 | 0xb) && self.fraction(&t).is_some();
        if !ok {
            reset(o, &local);
        }
        ok
    }

    /// Recognizer 6 (0x10003c40): a telephone number (kind 1): more than
    /// six digits starting with an area code, or in phone mode any area
    /// code (a comma before an extension).
    fn rec_phone(&self, o: &mut NumObj) -> bool {
        o.set_u8(0x4d, 0);
        o.set_u32(0, 1);
        o.set_u32(0x48, 1);
        o.set_u8(0x4c, 0);
        let local = o.cstr(4).to_vec();
        strip_sign(o, &local);
        let t = o.cstr(4).to_vec();
        let (dots, commas) = dots_commas(&t);
        let mode = o.mode();
        let ok = if dots != 0 {
            false
        } else if commas == 0 {
            match mode {
                0 | 8 => !t.iter().any(|c| self.s(NOT_INTEGER).contains(c)) && t.len() > 6 && self.area_code(&t) != 0,
                1 => self.area_code(&t) != 0,
                _ => false,
            }
        } else if commas == 1 {
            let p = t.iter().position(|&c| c == b',').unwrap();
            let (num, ext) = (&t[..p], &t[p + 1..]);
            if mode == 1 && self.area_code(num) != 0 {
                o.set_cstr(4, num);
                o.set_u8(0x4c, 1);
                o.set_cstr(0x4d, ext);
                true
            } else {
                false
            }
        } else {
            false
        };
        if !ok {
            reset(o, &local);
        }
        ok
    }

    /// Recognizer 7 (0x10004050): an integer (kind 0xa): up to five digits,
    /// with a decimal comma, or grouped by dots (+0x48 3).
    fn rec_integer(&self, o: &mut NumObj) -> bool {
        o.set_u8(0x4c, 0);
        o.set_u8(0x8c, 0);
        o.set_u32(0, 0xa);
        o.set_u32(0x48, 0xa);
        o.set_u8(0xcc, 0);
        o.set_u8(0xcd, 0);
        o.set_u8(0xd0, 0);
        let local = o.cstr(4).to_vec();
        match strip_sign(o, &local) {
            b'-' => o.set_u8(0xcc, 1),
            b'+' => o.set_u8(0xcd, 1),
            _ => {}
        }
        let t = o.cstr(4).to_vec();
        let (dots, commas) = dots_commas(&t);
        let mode = o.mode();
        let seps = |s: &[u8]| s.iter().any(|c| self.s(NOT_INTEGER).contains(c));
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
            reset(o, &local);
        }
        ok
    }

    /// Recognizer 8 (0x10003e80): a code read digit by digit (kind 2), in
    /// mode 2 only (a decimal comma makes it fail).
    fn rec_digits(&self, o: &mut NumObj) -> bool {
        o.set_u32(0, 2);
        o.set_u32(0x48, 2);
        o.set_u8(0x8c, 0);
        o.set_u8(0x4c, 0);
        o.set_u8(0x8d, 0);
        o.set_u8(0x8e, 0);
        let local = o.cstr(4).to_vec();
        match strip_sign(o, &local) {
            b'-' => o.set_u8(0x8d, 1),
            b'+' => o.set_u8(0x8e, 1),
            _ => {}
        }
        let t = o.cstr(4).to_vec();
        let (dots, commas) = dots_commas(&t);
        if dots == 0 && commas == 0 && o.mode() == 2 {
            return true;
        }
        if dots == 0 && commas == 1 && o.mode() == 2 {
            let p = t.iter().position(|&c| c == b',').unwrap();
            o.set_u8(0x8c, 1);
            o.set_cstr(4, &t[..p]);
            o.set_cstr(0x4c, &t[p + 1..]);
        }
        reset(o, &local);
        false
    }

    /// 0x10001610: the ordinal of a digit string (`premier`/`première`,
    /// `troisième`, `vingt et unième`).
    fn ordinal(&self, out: &mut Vec<u8>, t: &[u8], suffix: bool) {
        let mut t = cs(t);
        while t.first() == Some(&b'0') {
            t = &t[1..];
        }
        if let Some(k) = ORDINALS.iter().position(|p| t == self.s(p.0)) {
            out.extend_from_slice(self.s(ORDINALS[k].1));
            if k == 0 && !suffix {
                out.extend_from_slice(self.s(ORDINAL_E));
            }
            return;
        }
        q::convert_integer(out, 0, t, false, self);
        if !out.is_empty() {
            out.extend_from_slice(self.s(IEME));
        }
    }

    /// 0x10001e50: a date: the day, the month's name, the year.
    fn date(&self, out: &mut Vec<u8>, day: u32, month: u32, year: u32) {
        if day != 0 {
            q::convert_integer(out, 0, day.to_string().as_bytes(), false, self);
            q::add_separator(out, 0, self.word(1));
        }
        if (1..=12).contains(&month) {
            out.extend_from_slice(self.word(0x50 + month));
            q::add_separator(out, 0, self.word(1));
        }
        if year != 0 {
            let end = out.len();
            q::convert_integer(out, end, year.to_string().as_bytes(), false, self);
        }
    }

    /// 0x10004430: a time `h:mm[:ss]` (`sept heures quarante cinq`).
    fn time(&self, out: &mut Vec<u8>, text: &[u8]) {
        let seps = self.s(TIME_SEPS);
        let buf = &cs(text)[..cs(text).len().min(0x3f)];
        let Some(p) = buf.iter().position(|c| seps.contains(c)) else {
            return;
        };
        let h = &buf[..p];
        let h = if at(h, 0) == b'0' && (b'1'..=b'9').contains(&at(h, 1)) { &h[1..] } else { h };
        q::convert_integer(out, 0, h, false, self);
        out.extend_from_slice(self.s(if q::atol(h) == 1 { HEURE } else { HEURES }));
        let rest = &buf[p + 1..];
        let int = |out: &mut Vec<u8>, s: &[u8]| {
            let end = out.len();
            q::convert_integer(out, end, s, false, self);
        };
        let sep = |out: &mut Vec<u8>, id: u32| q::add_separator(out, 0, self.word(id));
        let is00 = |s: &[u8]| at(s, 0) == b'0' && at(s, 1) == b'0';
        if let Some(k) = rest.iter().position(|c| seps.contains(c)) {
            let min = &rest[..k];
            let sec = &rest[k + 1..];
            if !min.is_empty() && !is00(min) && !sec.is_empty() && !is00(sec) {
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
            if at(min, 0) == b'0' {
                if is00(min) && is00(sec) {
                    return;
                }
                if at(min, 1) == b'0' {
                    sep(out, 1);
                    int(out, sec);
                    sep(out, 0x5d);
                    return;
                }
            }
            if !is00(sec) {
                return;
            }
            sep(out, 1);
            int(out, min);
            return;
        }
        if rest.is_empty() || is00(rest) {
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
            let end = out.len();
            q::convert_big_integer(out, end, s, self);
        } else {
            self.cardinal(out, s);
        }
    }

    /// 0x10001fa0: an amount of `cur` (`trois euro`, `quarante neuf euro
    /// quatre vingt dix`).
    fn money(&self, out: &mut Vec<u8>, text: &[u8], cur: &[u8]) {
        let text = cs(text);
        let cur = cs(cur);
        let one_pound = |t: &[u8]| t == self.s(ONE) && cur == self.s(POUND);
        let Some(p) = text.iter().position(|&c| c == b',') else {
            if one_pound(text) {
                out.extend_from_slice(self.word(0x12));
            } else {
                self.units(out, text);
            }
            self.currency(out, text, cur, false);
            return;
        };
        let units = &text[..p];
        let mut dec = text[p + 1..].to_vec();
        let c = at(&dec, 0);
        if c != 0 && c != b'0' && dec.len() == 1 {
            dec.push(b'0');
        }
        if c == 0 || dec.len() != 2 {
            self.units(out, units);
            for id in [1, 3, 1] {
                out.extend_from_slice(self.word(id));
            }
            let end = out.len();
            self.decimals_at(out, end, &dec);
            self.currency(out, &dec, cur, false);
            return;
        }
        if units == self.s(ZERO) {
            if c == b'0' {
                out.extend_from_slice(self.digit(dec[1]));
                self.currency(out, &dec[1..], cur, true);
            } else {
                self.cardinal(out, &dec);
                out.extend_from_slice(self.word(1));
                self.currency(out, &dec, cur, true);
            }
            return;
        }
        if one_pound(units) {
            out.extend_from_slice(self.word(0x12));
        } else {
            self.units(out, units);
        }
        self.currency(out, units, cur, false);
        if dec == self.s(ZERO2) {
            return;
        }
        out.extend_from_slice(self.word(1));
        if c == b'0' {
            out.extend_from_slice(self.s(ET));
            out.extend_from_slice(self.word(1));
            out.extend_from_slice(self.digit(dec[1]));
            self.currency(out, &dec[1..], cur, true);
            return;
        }
        self.cardinal(out, &dec);
    }

    /// Adds `moins`/`plus` and a space for a sign flag pair.
    fn sign(&self, out: &mut Vec<u8>, minus: u8, plus: u8) {
        let id = if minus == 1 {
            7
        } else if plus == 1 {
            8
        } else {
            return;
        };
        q::add_separator(out, 0, self.word(id));
        q::add_separator(out, 0, self.word(1));
    }

    /// Slot 6 with the output's C string starting at `at`.
    fn decimals_at(&self, out: &mut Vec<u8>, at: usize, digits: &[u8]) -> bool {
        let mut t = out.split_off(at);
        let pause = self.decimals_c(&mut t, digits);
        out.extend(t);
        pause
    }

    /// 0x100062c0 on the C string `t` (the slot's own buffer).
    fn decimals_c(&self, t: &mut Vec<u8>, digits: &[u8]) -> bool {
        let digits = cs(digits);
        let (lead0, pause) = q::set_pauses(digits);
        let sep = self.word(if pause { 0 } else { 1 });
        q::add_separator(t, 0, sep);
        let mut d = digits;
        while d.first() == Some(&b'0') {
            t.extend_from_slice(self.digit(b'0'));
            d = &d[1..];
            if d.first() != Some(&b'0') {
                break;
            }
            q::add_separator(t, 0, sep);
        }
        let n = d.len() & 0xffff;
        if n == 0 {
            return pause;
        }
        if lead0 {
            q::add_separator(t, 0, sep);
        }
        if n < 4 && n != 1 {
            self.cardinal(t, d);
            return pause;
        }
        for k in 1..n {
            t.extend_from_slice(self.digit(d[k - 1]));
            q::add_separator(t, 0, sep);
        }
        t.extend_from_slice(self.digit(d[n - 1]));
        pause
    }

    /// 0x10005130: the object's reading.
    fn render(&self, o: &NumObj) -> Vec<u8> {
        let mut out = Vec::new();
        let sep = |out: &mut Vec<u8>, id: u32| q::add_separator(out, 0, self.word(id));
        // the decimals after a number: word 0 or 1, `virgule`, slot 6
        let decimals = |out: &mut Vec<u8>, pause: bool, d: &[u8]| {
            sep(out, if pause { 0 } else { 1 });
            sep(out, 4);
            let end = out.len();
            self.decimals_at(out, end, d);
        };
        match o.kind() {
            0 => q::convert_real_number(&mut out, 0, o, self),
            1 => {
                self.digits(&mut out, o.cstr(4), o.u8(0x4c) != 0);
                if o.u8(0x4d) != 0 {
                    let mut o2 = NumObj::default();
                    o2.set_cstr(4, o.cstr(0x4d));
                    o2.set_u32(0x44, o.mode());
                    let end = out.len();
                    q::convert_real_number(&mut out, end, &o2, self);
                }
            }
            2 => {
                self.sign(&mut out, o.u8(0x8d), o.u8(0x8e));
                let pause = q::convert_integer(&mut out, 0, o.cstr(4), true, self);
                if o.u8(0x4c) != 0 {
                    decimals(&mut out, pause, o.cstr(0x4c));
                }
            }
            3 | 10 => {
                self.sign(&mut out, o.u8(0xcc), o.u8(0xcd));
                let end = out.len();
                let pause = if o.u32(0x48) == 10 {
                    q::convert_integer(&mut out, end, o.cstr(0x4c), false, self)
                } else {
                    q::convert_big_integer(&mut out, end, o.cstr(0x4c), self);
                    true
                };
                if o.u8(0x8c) != 0 {
                    decimals(&mut out, pause, o.cstr(0x8c));
                }
            }
            4 => match o.u32(0x48) {
                5 => self.ordinal(&mut out, o.cstr(4), true),
                6 => self.ordinal(&mut out, o.cstr(4), false),
                _ => {}
            },
            7 => {
                self.sign(&mut out, o.u8(0x8d), o.u8(0x8e));
                self.time(&mut out, o.cstr(4));
                if o.u8(0x4c) != 0 {
                    decimals(&mut out, true, o.cstr(0x4c));
                }
            }
            8 => {
                if o.u32(0x88) == 8 {
                    self.date(&mut out, o.u32(0x98), o.u32(0x9c), o.u32(0xa0));
                }
            }
            9 => {
                self.sign(&mut out, o.u8(0xcc), o.u8(0xcd));
                q::convert_integer(&mut out, 0, o.cstr(0x4c), false, self);
                if o.u8(0x8c) != 0 {
                    decimals(&mut out, true, o.cstr(0x8c));
                }
            }
            0xb => {
                if let Some(f) = self.fraction(o.cstr(4)) {
                    out = f.to_vec();
                }
            }
            0xc => {
                self.sign(&mut out, o.u8(0x56), o.u8(0x57));
                let mut t = o.cstr(4).to_vec();
                if o.u8(0x5a) != 0 {
                    for c in t.iter_mut() {
                        *c = match *c {
                            b',' => b'.',
                            b'.' => b',',
                            c => c,
                        };
                    }
                }
                self.money(&mut out, &t, o.cstr(0x4c));
            }
            _ => {}
        }
        out
    }
}

/// `LesIniNumeri` slot 9 (0x10005db0): the reading of the number token
/// `toks[i]` for the engine's reading type `kind` (5/6 ordinal: mode 4,
/// +0x48 5/6) and date order `dmy`, and how many following tokens it used.
pub(super) fn leggi_numero(fr: &French, toks: &mut [Tok], i: usize, kind: u32, dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
    let text = toks.get(i)?.text.clone();
    let mut o = NumObj::default();
    reset(&mut o, b"");
    if kind == 5 || kind == 6 {
        o.set_u32(0x44, 4);
        o.set_u32(0x48, if kind == 5 { 5 } else { 6 });
    } else {
        o.set_u32(0x44, kind);
    }
    let next = tok_text(toks, i + 1);
    let money = q::is_currency(next)
        || (i > 0 && q::is_currency(tok_text(toks, i - 1)))
        || (next == fr.s(LIVRE) && tok_text(toks, i + 3) == fr.s(STERLING))
        || CURRENCY_NAMES.iter().any(|&a| next == fr.s(a));
    if money {
        o.set_u32(0x44, 0xc);
    }
    o.set_cstr(4, cs(&text));
    // 0x10005050: the mode's own recognizer first
    let mut order: Vec<usize> = (0..9).collect();
    let first = match o.mode() {
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
        0 => fr.rec_date(dmy, &mut o),
        1 => fr.rec_time(&mut c18, toks, i, &mut o),
        2 => fr.rec_ordinal(&mut c18, toks, i, &mut o),
        3 => fr.rec_currency(&mut c18, toks, i, &mut o),
        4 => fr.rec_real(&mut o),
        5 => fr.rec_fraction(&mut o),
        6 => fr.rec_phone(&mut o),
        7 => fr.rec_integer(&mut o),
        _ => fr.rec_digits(&mut o),
    });
    if !hit {
        reset(&mut o, &text);
    }
    Some((fr.render(&o), c18))
}

impl NumSlots for French {
    /// Slot 0 (0x10001db0).
    fn word(&self, id: u32) -> &[u8] {
        French::word(self, id)
    }
    /// Slot 1 (0x10001d30): a telephone number: its area code, then the
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
    /// Slot 2 (0x10005730).
    fn cardinal(&self, out: &mut Vec<u8>, digits: &[u8]) {
        let s = self.cardinal_c(digits);
        out.extend(s);
    }
    /// Slot 3 (0x100056f0): `zéro`, `un`, else 0x10004970's word.
    fn digit(&self, d: u8) -> &[u8] {
        match d {
            b'0' => self.word(0xf),
            b'1' => self.word(0x10),
            _ => self.n_word(d as i8 as i32 - 0x30),
        }
    }
    /// Slot 4 (0x10001d80): digit by digit unless a cardinal of 2–12
    /// digits (2–3 when it had leading zeros).
    fn by_digits(&self, flag: bool, len: usize, lead0: bool) -> bool {
        !(!flag && len < 13 && (len < 4 || !lead0) && len != 1)
    }
    /// Slot 5 (0x10004760): the currency's unit or cent name between
    /// spaces (singular for `1`).
    fn currency(&self, out: &mut Vec<u8>, amount: &[u8], currency: &[u8], cents: bool) {
        let Some(k) = (0..8).find(|&k| self.s(self.pe.u32(CURRENCIES + 0x14 * k)) == cs(currency)) else {
            return;
        };
        let start = out.len();
        let mut t = out.split_off(start);
        q::add_separator(&mut t, 0, self.word(1));
        let one = cs(amount) == self.s(ONE);
        let f = match (cents, one) {
            (false, true) => 4,
            (false, false) => 8,
            (true, true) => 0xc,
            (true, false) => 0x10,
        };
        t.extend_from_slice(self.s(self.pe.u32(CURRENCIES + 0x14 * k + f)));
        q::add_separator(&mut t, 0, self.word(1));
        out.extend(t);
    }
    /// Slot 6 (0x100062c0): decimals: leading zeros as `zéro`, then up to
    /// three digits as a cardinal, else digit by digit (the module never
    /// sets its flag).
    fn decimals(&self, out: &mut Vec<u8>, digits: &[u8], _flag: bool) -> bool {
        let mut t = Vec::new();
        let pause = self.decimals_c(&mut t, digits);
        out.extend(t);
        pause
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DLL: &str = "/tmp/qqv/LTTS7/bin/LTTS7French.dll";
    /// The oracle's slot-9 traces (`LOQ_NUM_ONLY`, see MECHANICS.md §8).
    const TRACES: &str = "/tmp/qq-num-fr";

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

    /// Replays every slot-9 call traced in `/tmp/qq-num-fr/*.les`
    /// (`LOQ_NUM_ONLY`), comparing the reading, the tokens used and the
    /// rewritten tokens; skips when the DLL or the traces are not there.
    #[test]
    fn french_replay_slot9() {
        let Some(fr) = std::fs::read(DLL).ok().and_then(|d| French::from_files(d, None)) else { return };
        let mut files: Vec<_> = std::fs::read_dir(TRACES).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "les")).collect();
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
                let (got, used) = leggi_numero(&fr, &mut toks, idx, kind, &dmy).unwrap();
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
