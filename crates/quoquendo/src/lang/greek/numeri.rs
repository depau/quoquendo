//! Numbers: the reader LTTS7Greek.dll's `LesIniNumeri` (0x10007b60)
//! installs as slot 9 (0x10007740), the only one the engine calls, and the
//! slots 0–7 it and LTTS7Util's `ELQNum*` (`super::super::elqnum`) call
//! back (MECHANICS.md §4f). The module is built on the same skeleton as
//! Spanish's (`spanish::numeri`): the same nine recognizers in the same
//! order, the same renderer switch; the words, the cardinal's genders and
//! the ordinals are Greek. Strings are the module's Windows-1253 bytes;
//! where the C code writes through a `char *` into a buffer, the port keeps
//! the whole buffer and the offset that pointer has in it (`at`).

use super::Greek;
use crate::lang::common::Tok;
use crate::lang::elqnum::{self, NumObj, NumSlots};

/// `{ id, word }` pairs read by slot 0 (0x10002140): 0x4d of them.
const WORDS: u32 = 0x1001fea0;
const WORD_COUNT: u32 = 0x4d;
/// 0x100054f0: the word of 1–20 and the tens, through the index bytes at
/// 0x100056a8 and the jump table at 0x10005638 (each target is `push id;
/// call 0x10002140`, the id the byte after the push opcode; 0x10005631
/// returns `""`).
const UNIT_INDEX: u32 = 0x100056a8;
const UNIT_JUMPS: u32 = 0x10005638;
const UNIT_NONE: u32 = 0x10005631;
/// 0x10005a80: the stem of 13–19 (`δεκα`), the units' word after it.
const TEEN: u32 = 0x10020d18;
/// The currency names slot 5 (0x10005310) reads: 8 rows of `{ symbol,
/// unit, units, cent, cents }`; `"1"` (0x10020d14) takes the singular.
const CURRENCIES: u32 = 0x1001fe00;
const CURRENCY_ROWS: u32 = 8;
const ONE: u32 = 0x10020d14;
/// The area-code prefixes 0x10002050 searches (`ELQSearchString` mode 2).
const AREA_CODES: u32 = 0x1001fa20;
const AREA_CODE_COUNT: u32 = 0xf7;
/// 0x10001a40 / 0x10001d60: fractions and how they are read.
const FRACTIONS: [(u32, u32); 14] = [
    (0x10020200, 0x100201f8),
    (0x100201f4, 0x100201e8),
    (0x100201e4, 0x100201d8),
    (0x100201d4, 0x100201c8),
    (0x100201c4, 0x100201b8),
    (0x100201b4, 0x100201a8),
    (0x100201a4, 0x10020198),
    (0x10020194, 0x10020188),
    (0x10020180, 0x10020174),
    (0x1002016c, 0x10020160),
    (0x10020158, 0x10020148),
    (0x10020140, 0x10020130),
    (0x1002012c, 0x10020120),
    (0x1002011c, 0x1002010c),
];
/// Slot 9's currency words for its next token (0x10007824…): `δρχ` (twice),
/// `δράχμη`, `δραχμές`, `euro`, `eur`, `EUR`, `ευρώ`, `ΕΥΡΏ`.
const SLOT9_CURRENCY_WORDS: [u32; 9] = [0x10020ce8, 0x10020ce8, 0x10020ce0, 0x10020cd8, 0x10020cd0, 0x10020cc8, 0x10020ccc, 0x1001f2b0, 0x10020cc0];
/// Recognizer 3 (0x10003880): the drachma words (an amount without a comma
/// gets `f`, 0x10013bf0, the feminine mark) and the euro words (the next
/// token becomes `n`, the amount `€`, 0x1001f2b8).
const DRACHMA_WORDS: [u32; 4] = [0x10020ce8, 0x10020ce8, 0x10020ce0, 0x10020cd8];
const DRACHMA_MARK: u32 = 0x10013bf0;
const EURO_WORDS: [u32; 6] = [0x10020cd0, 0x10020ccc, 0x10020cc8, 0x1001f2b0, 0x10020cc0, 0x10020cb8];
const EURO: u32 = 0x1001f2b8;
/// Recognizer 2 (0x100036a0): `°`, `º`, and the string the token after
/// them must not be (`c`, 0x10020cac).
const DEGREE: u32 = 0x10020cb4;
const MASC_ORDINAL: u32 = 0x10020cb0;
const CELSIUS: u32 = 0x10020cac;
/// Recognizer 1 (0x10003250): `h` after an hour, `:` before its minutes.
const HOUR_MARK: u32 = 0x10013be8;
const HOUR_COLON: u32 = 0x10020ca8;
/// Recognizer 0 (0x100030f0): the date order a `YYYYMMDD` date needs.
const VXML_ORDER: u32 = 0x10013c98;
/// `":/-+"` (0x10020cf0), `".,:"` (0x10020d10), `","` (0x10020d00), `"0"`
/// (0x10020cfc), `"00"` (0x10020cf8).
const NOT_PLAIN: u32 = 0x10020cf0;
const TIME_SEPS: u32 = 0x10020d10;
const COMMA: u32 = 0x10020d00;
const ZERO: u32 = 0x10020cfc;
const ZERO2: u32 = 0x10020cf8;
/// The feminine forms dates and times put in: `πρώτη` (0x10020208) for the
/// 1st; `είς` over the last two letters of 3, 13, 23 (the immediates of
/// `mov [..-2], 0xf2dfe5` at 0x100022cc and 0x10004f9e); `ις` (0x10020204)
/// over the last letter of 4, 14; `μία` (the immediate at 0x10004f74) and
/// `είκοσι μία` (0x10020d04) for the hours 1 and 21.
const FIRST_F: u32 = 0x10020208;
const EIS_DATE: u32 = 0x100022d0;
const EIS_TIME: u32 = 0x10004fa2;
const IS_END: u32 = 0x10020204;
const ONE_F_TIME: u32 = 0x10004f76;
const TWENTY_ONE_F: u32 = 0x10020d04;
/// 0x100065d0, the ordinals: the hundreds' ending over a cardinal's last
/// two bytes, masculine and feminine.
const ORD_HUNDREDS_END: [u32; 2] = [0x10020e64, 0x10020e5c];
/// 10, 11, 12 and "ten times" (a five-digit 10…), then the endings after
/// 10–19 and after the units.
const ORD_TEN: u32 = 0x10020e3c;
const ORD_ELEVEN: u32 = 0x10020e4c;
const ORD_TWELVE: u32 = 0x10020e44;
const ORD_TEN_TIMES: u32 = 0x10020e54;
const ORD_END: [u32; 2] = [0x10020e38, 0x10020e34];
/// The tens 20–90, and the ending after them and the group words.
const ORD_TENS: [u32; 8] = [0x10020e2c, 0x10020e20, 0x10020e14, 0x10020e08, 0x10020e00, 0x10020df4, 0x10020de8, 0x10020ddc];
const ORD_TENS_END: [u32; 2] = [0x10020dd8, 0x10020dd4];
/// The units 1–9, and the "n times" forms 2–9 a four-digit number's
/// thousands digit takes.
const ORD_UNITS: [u32; 9] = [0x10020dcc, 0x10020dc0, 0x10020db0, 0x10020d9c, 0x10020d88, 0x10020d7c, 0x10020d6c, 0x10020d5c, 0x10020d4c];
const ORD_TIMES: [u32; 8] = [0x10020dc8, 0x10020db8, 0x10020da4, 0x10020d90, 0x10020d80, 0x10020d74, 0x10020d64, 0x10020d54];
/// The thousandth, millionth, billionth stems.
const ORD_GROUPS: [u32; 3] = [0x10020d20, 0x10020d28, 0x10020d38];

/// The C string at the start of `s`.
fn cs(s: &[u8]) -> &[u8] {
    &s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())]
}

/// `strncpy(buf, s, 0x3f)` into a 0x40-byte local.
fn copy63(s: &[u8]) -> Vec<u8> {
    cs(s)[..cs(s).len().min(0x3f)].to_vec()
}

/// `strpbrk`: the position of the first character of `set` in `s`.
fn strpbrk(s: &[u8], set: &[u8]) -> Option<usize> {
    cs(s).iter().position(|c| set.contains(c))
}

/// The number of `.` and `,` in `s`.
fn dots_commas(s: &[u8]) -> (usize, usize) {
    (s.iter().filter(|&&c| c == b'.').count(), s.iter().filter(|&&c| c == b',').count())
}

/// `ELQNumAddSeparator` on the string `out[at..]`.
fn sep(out: &mut Vec<u8>, at: usize, w: &[u8]) {
    elqnum::add_separator(out, at.min(out.len()), w)
}

/// `atol` as the module's comparisons see it (unsigned).
fn atou(s: &[u8]) -> u32 {
    elqnum::atol(cs(s)) as u32
}

/// The module's number code over its tables (the slot table LTTS7Util
/// calls back is [`NumSlots`] on this).
struct Num<'a>(&'a Greek);

impl Num<'_> {
    fn s(&self, va: u32) -> &[u8] {
        self.0.pe.cstr(va)
    }

    /// Slot 0 (0x10002140): the word numbered `id`, `""` when none.
    fn w(&self, id: u32) -> &[u8] {
        (0..WORD_COUNT).find(|&k| self.0.pe.u32(WORDS + 8 * k) == id).map(|k| self.s(self.0.pe.u32(WORDS + 8 * k + 4))).unwrap_or(b"")
    }

    /// 0x100054f0: the word of `n` (1–20 and the tens), `""` otherwise.
    fn unit(&self, n: i32) -> &[u8] {
        if !(1..=0x5a).contains(&n) {
            return b"";
        }
        let target = self.0.pe.u32(UNIT_JUMPS + 4 * self.0.pe.u8(UNIT_INDEX + n as u32 - 1) as u32);
        if target == UNIT_NONE {
            return b"";
        }
        self.w(self.0.pe.u8(target + 1) as u32)
    }

    /// Slot 3 (0x10005e30): one digit's word.
    fn digit_word(&self, d: u8) -> &[u8] {
        match d {
            b'0' => self.w(0xf),
            b'1' => self.w(0x10),
            _ => self.unit(d as i32 - b'0' as i32),
        }
    }

    /// 0x10005710: the hundreds digit at `p` read into `out` (the string at
    /// `at`), then a space; false for `0`. The ending agrees with the
    /// thousands (`-ες` when the number from `p` on is 1000–999999) or the
    /// `gender` (1 masculine `-οι`, 2 feminine `-ες`, 0 neuter `-α`).
    fn hundreds(&self, out: &mut Vec<u8>, at: usize, p: &[u8], gender: u32) -> bool {
        let c = |i: usize| p.get(i).copied().unwrap_or(0);
        let value = atou(p);
        if c(0) == b'0' {
            return false;
        }
        match c(0) {
            b'1' => out.extend_from_slice(self.w(if c(1) == b'0' && c(2) == b'0' { 0x3a } else { 0x3b })),
            b'2' => out.extend_from_slice(self.w(0x3e)),
            b'3' => out.extend_from_slice(self.w(0x3f)),
            b'4' => out.extend_from_slice(self.w(0x40)),
            b'5' => out.extend_from_slice(self.w(0x41)),
            b'6' => out.extend_from_slice(self.w(0x42)),
            b'8' => out.extend_from_slice(self.w(0x43)),
            b'7' | b'9' => {
                out.extend_from_slice(self.w(if c(0) == b'7' { 0x1e } else { 0x21 }));
                out.extend_from_slice(self.w(0x3c));
            }
            _ => {}
        }
        if c(0) != b'1' {
            let end = if (value < 1000 || value > 999_999) && gender != 2 {
                if gender == 1 {
                    0x46
                } else {
                    0x45
                }
            } else {
                0x44
            };
            out.extend_from_slice(self.w(end));
        }
        let w1 = self.w(1).to_vec();
        sep(out, at, &w1);
        true
    }

    /// 0x10005a80: the reading of the two digits at `p` ("" for 00): the
    /// tens and a space, then the units; one, three and four take their
    /// feminine (or, `gender` 1 after a zero ten, masculine) form when the
    /// number from `p` on is 1000–999999 or a gender is given.
    fn two(&self, p: &[u8], gender: u32) -> Vec<u8> {
        let t0 = p.first().copied().unwrap_or(0);
        let t1 = if t0 == 0 { 0 } else { p.get(1).copied().unwrap_or(0) };
        let n2 = elqnum::atol(cs(&[t0, t1]));
        let value = atou(p);
        let mut b = Vec::new();
        if n2 == 0 {
            return b;
        }
        let w1 = self.w(1).to_vec();
        let tens = if t0 == b'1' {
            let id = match t1 {
                b'1' => Some(0x23),
                b'2' => Some(0x24),
                b'0' => Some(0x22),
                _ => None,
            };
            if let Some(id) = id {
                b.extend_from_slice(self.w(id));
                sep(&mut b, 0, &w1);
                return b;
            }
            b.extend_from_slice(self.s(TEEN));
            b'1'
        } else {
            b.extend_from_slice(self.unit((t0 as i32 - b'0' as i32) * 10));
            sep(&mut b, 0, &w1);
            t0
        };
        let u = t1 as i32 - b'0' as i32;
        if (value <= 999 || value >= 1_000_000) && gender != 1 && gender != 2 {
            b.extend_from_slice(self.unit(u));
        } else {
            match t1 {
                b'1' => b.extend_from_slice(self.w(if tens == b'0' && gender == 1 { 0x13 } else { 0x12 })),
                b'3' => b.extend_from_slice(self.w(0x18)),
                b'4' => b.extend_from_slice(self.w(0x1a)),
                _ => b.extend_from_slice(self.unit(u)),
            }
        }
        b
    }

    /// Slot 2 (0x10005e70): the cardinal reading of the digits `s`
    /// appended to `out` (the string starts at the end of `out`); an `m` or
    /// `f` in it (cut off there) makes the last three digits masculine or
    /// feminine. One digit alone reads as nothing (slot 4 sends it to slot
    /// 3).
    fn cardinal(&self, out: &mut Vec<u8>, s: &[u8]) {
        let at = out.len();
        let mut local = copy63(s);
        let mut gender = 0;
        if let Some(k) = local.iter().position(|&c| c == b'm') {
            gender = 1;
            local.truncate(k);
        } else if let Some(k) = local.iter().position(|&c| c == b'f') {
            gender = 2;
            local.truncate(k);
        }
        let len = local.len();
        let ch = |i: usize| local.get(i).copied().unwrap_or(0);
        let rest = |p: usize| local.get(p..).unwrap_or(&[]);
        let w = |id: u32| self.w(id).to_vec();
        // the C looks at the byte before the string's end, whatever it is
        let space_or_sep = |out: &mut Vec<u8>| {
            if out.last() != Some(&b' ') {
                sep(out, at, &w(1));
            }
        };
        // d: two digits were just read; e: the group so far is zero; c:
        // there were hundreds of thousands; a/b: a lone 01 million/thousand
        let (mut a, mut b, mut c, mut d, mut e) = (false, false, false, false, true);
        let mut p = 0usize;
        if len > 0xb {
            self.hundreds(out, at, rest(0), 0);
            p = 1;
        }
        if len > 10 {
            d = true;
            out.extend(self.two(rest(p), 0));
            p += 1;
        }
        if len > 9 {
            if ch(p) == b'1' && !d {
                out.extend(w(0x4d));
            } else {
                if ch(p) != b'1' && !d {
                    out.extend_from_slice(self.unit(ch(p) as i32 - b'0' as i32));
                }
                space_or_sep(out);
                out.extend(w(0x4e));
            }
            sep(out, at, &w(0));
            p += 1;
            d = false;
            e = true;
        }
        if len > 8 {
            e = !self.hundreds(out, at, rest(p), 0);
            p += 1;
        }
        if len > 7 {
            d = true;
            if e && ch(p) == b'0' && ch(p + 1) == b'1' {
                a = true;
            } else {
                out.extend(self.two(rest(p), 0));
            }
            if ch(p) != b'0' {
                e = false;
            }
            p += 1;
        }
        if len > 6 {
            if ch(p) != b'0' || !e {
                if a || (ch(p) == b'1' && !d) {
                    out.extend(w(0x4a));
                } else {
                    if ch(p) != b'1' && !d {
                        out.extend_from_slice(self.unit(ch(p) as i32 - b'0' as i32));
                    }
                    space_or_sep(out);
                    out.extend(w(0x4b));
                }
                sep(out, at, &w(0));
            }
            p += 1;
            d = false;
            e = true;
        }
        if len > 5 {
            c = true;
            e = !self.hundreds(out, at, rest(p), 0);
            p += 1;
        }
        if len > 4 {
            d = true;
            if e && ch(p) == b'0' && ch(p + 1) == b'1' {
                b = true;
            } else {
                out.extend(self.two(rest(p), 0));
            }
            if ch(p) != b'0' {
                e = false;
            }
            p += 1;
        }
        if len >= 4 {
            let cc = ch(p);
            if cc != b'0' || !e {
                if cc == b'1' && !d && !c {
                    // 0x10006296…: one thousand
                    out.extend(w(0x47));
                } else {
                    if !d {
                        // 0x10006487
                        match cc {
                            b'1' => out.extend(w(0x12)),
                            b'3' => out.extend(w(0x18)),
                            b'4' => out.extend(w(0x1a)),
                            _ => out.extend_from_slice(self.unit(cc as i32 - b'0' as i32)),
                        }
                    }
                    // 0x10006546
                    sep(out, at, &w(1));
                    out.extend(w(0x48));
                    sep(out, at, &w(if !c || b { 1 } else { 0 }));
                }
                sep(out, at, &w(1));
            }
            p += 1;
        }
        if len > 2 {
            self.hundreds(out, at, rest(p), gender);
            p += 1;
        }
        if len > 1 {
            out.extend(self.two(rest(p), gender));
        }
        let w0 = w(0);
        if !w0.is_empty() && out.ends_with(&w0) {
            out.truncate(out.len() - w0.len());
        }
    }

    /// 0x10002050: the length of the area code `digits` starts with (the
    /// entry `ELQSearchString` mode 2 finds), 0 when none.
    fn area_code(&self, digits: &[u8]) -> usize {
        let key = copy63(digits);
        let entry = |k: u32| self.s(self.0.pe.u32(AREA_CODES + 4 * k));
        let (mut lo, mut hi) = (0i64, AREA_CODE_COUNT as i64 - 1);
        while lo <= hi {
            let mid = (lo + hi) / 2;
            let e = entry(mid as u32);
            let n = e.len().min(key.len());
            let ord = match key[..n].cmp(&e[..n]) {
                std::cmp::Ordering::Equal if key.len() < e.len() => std::cmp::Ordering::Less,
                o => o,
            };
            match ord {
                std::cmp::Ordering::Less => hi = mid - 1,
                std::cmp::Ordering::Greater => lo = mid + 1,
                std::cmp::Ordering::Equal => return e.len(),
            }
        }
        0
    }

    /// Slot 1 (0x100020c0): a telephone number: the area code, then the
    /// rest (`ELQNumConvertPhoneNumber` steps 0 and 1), appended to `out`
    /// (step 0 clears the slot's string, which starts at `out`'s end).
    fn phone(&self, out: &mut Vec<u8>, digits: &[u8], pairs: bool) {
        let digits = cs(digits);
        let area = self.area_code(digits);
        let mut len = area;
        let mut s = Vec::new();
        elqnum::convert_phone_number(self, &mut s, digits, pairs, &mut len, 0);
        elqnum::convert_phone_number(self, &mut s, digits.get(area..).unwrap_or(&[]), pairs, &mut len, 1);
        out.extend(s);
    }

    /// Slot 5 (0x10005310): the currency's name after `amount`, a space
    /// before and after.
    fn currency_name(&self, out: &mut Vec<u8>, amount: &[u8], currency: &[u8], cents: bool) {
        let Some(k) = (0..CURRENCY_ROWS).find(|&k| self.s(self.0.pe.u32(CURRENCIES + 0x14 * k)) == cs(currency)) else {
            return;
        };
        let w1 = self.w(1).to_vec();
        let end = out.len();
        sep(out, end, &w1);
        let one = cs(amount) == self.s(ONE);
        let col = match (cents, one) {
            (false, true) => 1,
            (false, false) => 2,
            (true, true) => 3,
            (true, false) => 4,
        };
        out.extend_from_slice(self.s(self.0.pe.u32(CURRENCIES + 0x14 * k + 4 * col)));
        let end = out.len();
        sep(out, end, &w1);
    }

    /// Slot 6 (0x10006f50): decimals: leading zeros one by one, then up to
    /// three digits as a cardinal (unless `by_digits`), more digit by
    /// digit. Returns `ELQNumSetPauses`' pause flag.
    fn decimals(&self, out: &mut Vec<u8>, digits: &[u8], by_digits: bool) -> bool {
        let at = out.len();
        let digits = cs(digits);
        let (lead0, pause) = elqnum::set_pauses(digits);
        let w = self.w(if pause { 0 } else { 1 }).to_vec();
        sep(out, at, &w);
        let mut s = digits;
        while s.first() == Some(&b'0') {
            out.extend_from_slice(self.digit_word(b'0'));
            s = &s[1..];
            if s.first() != Some(&b'0') {
                break;
            }
            sep(out, at, &w);
        }
        let n = s.len() & 0xffff;
        if n == 0 {
            return pause;
        }
        if lead0 {
            sep(out, at, &w);
        }
        if n < 4 && n != 1 && !by_digits {
            self.cardinal(out, s);
            return pause;
        }
        for &d in &s[..n - 1] {
            out.extend_from_slice(self.digit_word(d));
            sep(out, at, &w);
        }
        out.extend_from_slice(self.digit_word(s[n - 1]));
        pause
    }

    /// Slot 7 (0x10002170): the parts of a date in the order `dmy`.
    fn date_parts(&self, dmy: &[u8], text: &[u8], obj: &mut NumObj) -> bool {
        let mut date = elqnum::Date::default();
        let ok = elqnum::extract_date(dmy, text, &mut date);
        obj.set_u32(0x98, date.day);
        obj.set_u32(0x9c, date.month);
        obj.set_u32(0xa0, date.year);
        if !ok {
            return false;
        }
        let min_year = if date.sep == b'/' { 0 } else { 0x50 };
        let ok = elqnum::validate_date(dmy, &mut date, min_year);
        obj.set_u32(0x98, date.day);
        obj.set_u32(0x9c, date.month);
        obj.set_u32(0xa0, date.year);
        ok
    }
}

impl NumSlots for Num<'_> {
    fn word(&self, id: u32) -> &[u8] {
        self.w(id)
    }
    fn digits(&self, out: &mut Vec<u8>, digits: &[u8], flag: bool) {
        self.phone(out, digits, flag)
    }
    fn cardinal(&self, out: &mut Vec<u8>, digits: &[u8]) {
        Num::cardinal(self, out, digits)
    }
    fn digit(&self, d: u8) -> &[u8] {
        self.digit_word(d)
    }
    /// Slot 4 (0x10002110).
    fn by_digits(&self, flag: bool, len: usize, lead0: bool) -> bool {
        flag || len > 0xc || len == 1 || (len > 3 && lead0)
    }
    fn currency(&self, out: &mut Vec<u8>, amount: &[u8], currency: &[u8], cents: bool) {
        self.currency_name(out, amount, currency, cents)
    }
    fn decimals(&self, out: &mut Vec<u8>, digits: &[u8], flag: bool) -> bool {
        Num::decimals(self, out, digits, flag)
    }
}

/// The engine's token buffer as the module sees it (0x10001950): a window
/// of at most 7 tokens before the current one, the end marked type 7.
struct Window<'t> {
    toks: &'t mut [Tok],
    base: usize,
}

impl Window<'_> {
    fn kind(&self, j: usize) -> u32 {
        self.toks.get(j).map_or(7, |t| t.kind)
    }
    fn text(&self, j: usize) -> &[u8] {
        self.toks.get(j).map_or(b"", |t| t.text.as_slice())
    }
    /// 0x10001950: the token `delta` non-tag tokens from `i` (no further
    /// back than the window's start, no further on than its 31st entry or
    /// an end marker).
    fn fetch(&self, i: usize, delta: i32) -> usize {
        let mut j = i;
        let mut d = delta;
        if d < 0 {
            while j > self.base {
                j -= 1;
                if self.kind(j) != 5 {
                    d += 1;
                }
                if d >= 0 {
                    break;
                }
            }
        } else if d > 0 {
            let mut n = i - self.base + 1;
            while n < 31 && self.kind(j) != 7 {
                j += 1;
                n += 1;
                if self.kind(j) != 5 {
                    d -= 1;
                }
                if d <= 0 {
                    break;
                }
            }
        }
        j
    }
}

/// What a recognizer works with: the window, the token's index, its
/// date order and what slot 9 reports as used (ctx+0x18).
struct Ctx<'t> {
    win: Window<'t>,
    i: usize,
    dmy: Vec<u8>,
    used: usize,
}

/// Slot 9 (0x10007740): the reading of the number token `i` for reading
/// type `kind` and date order `dmy`, and how many tokens after it it used.
pub(super) fn leggi_numero(g: &Greek, toks: &mut [Tok], i: usize, kind: u32, dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
    Some(Num(g).leggi_numero(toks, i, kind, dmy))
}

impl Num<'_> {
    fn leggi_numero(&self, toks: &mut [Tok], i: usize, kind: u32, dmy: &[u8]) -> (Vec<u8>, usize) {
        let text = toks[i].text.clone();
        let base = i.saturating_sub(7);
        let mut cx = Ctx {
            win: Window { toks, base },
            i,
            dmy: dmy.to_vec(),
            used: 0,
        };
        let next1 = cx.win.fetch(i, 1);
        let prev = cx.win.fetch(i, -1);
        let mut obj = NumObj::default();
        let mut mode = kind;
        if kind == 5 || kind == 6 {
            mode = 4;
            obj.set_u32(0x48, kind);
        }
        let n1 = cx.win.text(next1).to_vec();
        if elqnum::is_currency(&n1) || (i > base && elqnum::is_currency(cx.win.text(prev))) {
            mode = 0xc;
        }
        if SLOT9_CURRENCY_WORDS.iter().any(|&a| n1 == self.s(a)) {
            mode = 0xc;
        } else if n1.first() == Some(&b'%') {
            mode = 10;
        }
        obj.set_u32(0x44, mode);
        obj.set_cstr(4, &text);
        // 0x10005d50: the recognizers, the one the reading type names first
        let mut order: Vec<u8> = (0..9).collect();
        let first = match mode {
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
        if first > 0 {
            let r = order.remove(first);
            order.insert(0, r);
        }
        let mut found = false;
        for r in order {
            let ok = match r {
                0 => self.r_date(&mut cx, &mut obj),
                1 => self.r_time(&mut cx, &mut obj),
                2 => self.r_ordinal(&mut cx, &mut obj),
                3 => self.r_currency(&mut cx, &mut obj),
                4 => Self::r_mode9(&mut obj),
                5 => self.r_fraction(&mut obj),
                6 => self.r_phone(&mut obj),
                7 => self.r_real(&mut obj),
                _ => Self::r_code(&mut obj),
            };
            if ok {
                found = true;
                break;
            }
        }
        if !found {
            elqnum::set_default_type(&mut obj, &text);
        }
        (self.render(&obj), cx.used)
    }

    /// The common start of a recognizer: the text without its sign (the
    /// sign's flags at `minus`/`minus + 1` when given), and the original.
    fn strip_sign(obj: &mut NumObj, minus: Option<usize>) -> Vec<u8> {
        let buf = copy63(obj.text());
        let body = match buf.first() {
            Some(b'-') => {
                if let Some(m) = minus {
                    obj.set_u8(m, 1);
                }
                &buf[1..]
            }
            Some(b'+') => {
                if let Some(m) = minus {
                    obj.set_u8(m + 1, 1);
                }
                &buf[1..]
            }
            _ => &buf[..],
        };
        let body = body.to_vec();
        obj.set_cstr(4, &body);
        buf
    }

    fn fail(obj: &mut NumObj, buf: &[u8]) -> bool {
        elqnum::set_default_type(obj, buf);
        false
    }

    /// Recognizer 0 (0x100030f0): a date (kind 8).
    fn r_date(&self, cx: &mut Ctx, obj: &mut NumObj) -> bool {
        obj.set_cstr(0x8c, &cx.dmy);
        obj.set_u32(0, 8);
        obj.set_u32(0x88, 8);
        obj.set_u8(0x94, 0);
        obj.set_u8(0x48, 0);
        let buf = Self::strip_sign(obj, None);
        let mode = obj.mode();
        if mode == 0 || mode == 8 {
            let text = obj.text().to_vec();
            if self.date_parts(&cx.dmy, &text, obj) {
                return true;
            }
            if mode == 8 {
                let mut d = elqnum::Date::default();
                let ok = elqnum::format_vxml10_date(&text, &mut d);
                obj.set_u32(0x98, d.day);
                obj.set_u32(0x9c, d.month);
                obj.set_u32(0xa0, d.year);
                if ok && cx.dmy.eq_ignore_ascii_case(self.s(VXML_ORDER)) {
                    return true;
                }
            }
        }
        Self::fail(obj, &buf)
    }

    /// Recognizer 1 (0x10003250): a time (kind 7); an hour followed by `h`
    /// (and minutes) uses those tokens.
    fn r_time(&self, cx: &mut Ctx, obj: &mut NumObj) -> bool {
        let i = cx.i;
        let n1 = cx.win.fetch(i, 1);
        let n2 = cx.win.fetch(i, 2);
        let n3 = cx.win.fetch(i, 3);
        obj.set_u8(0x4c, 0);
        obj.set_u32(0, 7);
        obj.set_u32(0x48, 7);
        for o in 0x8c..0x90 {
            obj.set_u8(o, 0);
        }
        let buf = Self::strip_sign(obj, Some(0x8d));
        let text = obj.text().to_vec();
        let (dots, commas) = dots_commas(&text);
        let h = elqnum::atol(&text);
        let mode = obj.mode();
        let signed = obj.u8(0x8d) != 0 || obj.u8(0x8e) != 0;
        if h < 0x19 && h > 0 && cx.win.text(n1) == self.s(HOUR_MARK) {
            let t2 = cx.win.text(n2).to_vec();
            let t3 = cx.win.text(n3);
            let mut t = text.clone();
            t.extend_from_slice(self.s(HOUR_COLON));
            if elqnum::is_number(&t2, b"", true) && elqnum::atol(&t2) < 0x3c && t3 != self.s(HOUR_MARK) {
                t.extend_from_slice(&t2);
                cx.used = n2 - i;
            } else {
                cx.used = n1 - i;
            }
            obj.set_cstr(4, &t);
            return true;
        }
        if dots == 0 {
            if commas == 0 {
                if matches!(mode, 0 | 7 | 8) && !signed && elqnum::is_time(&text) {
                    return true;
                }
                return Self::fail(obj, &buf);
            }
            if commas == 1 {
                let k = text.iter().position(|&c| c == b',').unwrap();
                let (hours, minutes) = (text[..k].to_vec(), copy63(&text[k + 1..]));
                if (mode == 0 || (7..=8).contains(&mode)) && obj.u8(0x8d) == 0 {
                    if obj.u8(0x8e) == 0 && elqnum::is_time(&text) {
                        return true;
                    }
                    if !signed && elqnum::is_time(&hours) {
                        obj.set_cstr(4, &hours);
                        obj.set_u8(0x8c, 1);
                        obj.set_cstr(0x4c, &minutes);
                        return true;
                    }
                }
            }
            return Self::fail(obj, &buf);
        }
        if commas == 0 && dots < 3 && (mode == 0 || mode == 7) && !signed && elqnum::is_time(&text) {
            return true;
        }
        Self::fail(obj, &buf)
    }

    /// Recognizer 2 (0x100036a0): an ordinal (kind 4): a reading type 4,
    /// or `°`/`º` (not before `c`) or `'` after the number.
    fn r_ordinal(&self, cx: &mut Ctx, obj: &mut NumObj) -> bool {
        let i = cx.i;
        obj.set_u32(0, 4);
        let buf = Self::strip_sign(obj, None);
        let (dots, commas) = dots_commas(obj.text());
        if dots + commas != 0 {
            return Self::fail(obj, &buf);
        }
        let n1 = cx.win.fetch(i, 1);
        let n2 = cx.win.fetch(i, 2);
        if obj.mode() == 4 {
            return true;
        }
        let t1 = cx.win.text(n1);
        if (t1 == self.s(DEGREE) || t1 == self.s(MASC_ORDINAL)) && !cx.win.text(n2).eq_ignore_ascii_case(self.s(CELSIUS)) {
            obj.set_u32(0x48, 5);
            return true;
        }
        if t1.first() == Some(&b'\'') && cx.win.kind(n2) != 1 {
            obj.set_u32(0x48, 5);
            return true;
        }
        Self::fail(obj, &buf)
    }

    /// Recognizer 3 (0x10003880): an amount (kind 0xc): drachmas after it
    /// (the amount gets the feminine mark), a euro word after it (that token
    /// becomes `n` and is used), or a currency symbol before or after it.
    fn r_currency(&self, cx: &mut Ctx, obj: &mut NumObj) -> bool {
        let i = cx.i;
        obj.set_u32(0, 0xc);
        obj.set_u32(0x48, 0xf);
        for o in [0x56, 0x57, 0x58, 0x5a] {
            obj.set_u8(o, 0);
        }
        obj.set_u32(0x5c, 1);
        obj.set_u8(0x4c, 0);
        let buf = Self::strip_sign(obj, Some(0x56));
        let text = obj.text().to_vec();
        let (dots, commas) = dots_commas(&text);
        let big = !text.is_empty() && dots != 0 && elqnum::is_big_integer(&text);
        obj.set_u8(0x59, big as u8);
        if dots == 1 {
            if commas == 0 {
                let k = text.iter().position(|&c| c == b'.').unwrap();
                if text.len() - k - 1 != 3 {
                    obj.set_u8(0x5a, 1);
                }
            } else {
                obj.set_u8(0x5a, 1);
            }
        }
        let n1 = cx.win.fetch(i, 1);
        let prev = cx.win.fetch(i, -1);
        let t1 = cx.win.text(n1).to_vec();
        if commas == 0 && DRACHMA_WORDS.iter().any(|&a| t1 == self.s(a)) {
            let mut t = text.clone();
            t.extend_from_slice(self.s(DRACHMA_MARK));
            obj.set_cstr(4, &t);
            obj.set_u32(0x5c, 2);
            return true;
        }
        // the token right after the number in the buffer, tag or not
        let take = |cx: &mut Ctx| {
            if let Some(t) = cx.win.toks.get_mut(i + 1) {
                t.text = b"n".to_vec();
            }
            cx.used = 1;
            true
        };
        if EURO_WORDS.iter().any(|&a| t1 == self.s(a)) {
            obj.set_cstr(0x4c, self.s(EURO));
            return take(cx);
        }
        if i > cx.win.base && elqnum::is_currency(cx.win.text(prev)) {
            let p = cx.win.text(prev).to_vec();
            obj.set_cstr(0x4c, &p);
            return true;
        }
        if elqnum::is_currency(&t1) {
            obj.set_cstr(0x4c, &t1);
            return take(cx);
        }
        Self::fail(obj, &buf)
    }

    /// Recognizer 4 (0x10003d30): numbers of reading type 9 (kind 9).
    fn r_mode9(obj: &mut NumObj) -> bool {
        obj.set_u8(0x8c, 0);
        obj.set_u8(0x4c, 0);
        obj.set_u32(0, 9);
        obj.set_u32(0x48, 9);
        obj.set_u8(0xcc, 0);
        obj.set_u8(0xcd, 0);
        let buf = Self::strip_sign(obj, Some(0xcc));
        let text = obj.text().to_vec();
        let (dots, commas) = dots_commas(&text);
        let mode = obj.mode();
        if dots == 0 {
            if commas == 0 {
                if mode == 9 {
                    obj.set_cstr(0x4c, &text);
                    return true;
                }
                return Self::fail(obj, &buf);
            }
            if commas == 1 {
                if mode == 9 {
                    let k = text.iter().position(|&c| c == b',').unwrap();
                    obj.set_cstr(0x4c, &text[k + 1..]);
                }
                if !obj.cstr(0x4c).is_empty() {
                    obj.set_u8(0x8c, 0);
                    return true;
                }
            }
            return Self::fail(obj, &buf);
        }
        if commas == 0 {
            if (mode == 0 || mode == 3) && elqnum::is_big_integer(&text) {
                obj.set_u32(0x48, 3);
                return Self::fail(obj, &buf);
            }
            if mode == 9 && elqnum::is_big_integer(&text) {
                obj.set_cstr(0x4c, &text);
                return true;
            }
            return Self::fail(obj, &buf);
        }
        if commas == 1 {
            let (Some(p), Some(q)) = (text.iter().rposition(|&c| c == b'.'), text.iter().rposition(|&c| c == b',')) else {
                return Self::fail(obj, &buf);
            };
            if p < q {
                let int = copy63(&text[..q]);
                if elqnum::is_big_integer(&int) && mode == 9 {
                    obj.set_cstr(0x4c, &int);
                    obj.set_u32(0x48, 3);
                    obj.set_cstr(0x8c, &text[q + 1..]);
                    return true;
                }
            }
        }
        Self::fail(obj, &buf)
    }

    /// Recognizer 5 (0x10004010): a fraction of the list (kind 0xb).
    fn r_fraction(&self, obj: &mut NumObj) -> bool {
        obj.set_u32(0, 0xb);
        obj.set_u32(0x48, 0xb);
        obj.set_u8(0x4c, 0);
        obj.set_u8(0x4d, 0);
        let buf = Self::strip_sign(obj, Some(0x4c));
        let (dots, commas) = dots_commas(obj.text());
        if dots + commas == 0 && matches!(obj.mode(), 8 | 0 | 0xb) && self.fraction(obj.text()).is_some() {
            return true;
        }
        Self::fail(obj, &buf)
    }

    /// 0x10001a40: how the fraction `s` is read (0x10001d60: whether it is
    /// one).
    fn fraction(&self, s: &[u8]) -> Option<&[u8]> {
        FRACTIONS.iter().find(|&&(k, _)| s == self.s(k)).map(|&(_, v)| self.s(v))
    }

    /// Recognizer 6 (0x10004130): a telephone number (kind 1): more than six
    /// digits after an area code, or any with reading type 1.
    fn r_phone(&self, obj: &mut NumObj) -> bool {
        obj.set_u8(0x4d, 0);
        obj.set_u32(0, 1);
        obj.set_u32(0x48, 1);
        obj.set_u8(0x4c, 0);
        let buf = Self::strip_sign(obj, None);
        let text = obj.text().to_vec();
        let (dots, commas) = dots_commas(&text);
        let mode = obj.mode();
        if dots != 0 {
            return Self::fail(obj, &buf);
        }
        if commas == 0 {
            match mode {
                1 if self.area_code(&text) != 0 => return true,
                0 | 8 if strpbrk(&text, self.s(NOT_PLAIN)).is_none() && text.len() > 6 && self.area_code(&text) != 0 => {
                    obj.set_u8(0x4c, 0);
                    return true;
                }
                _ => {}
            }
            return Self::fail(obj, &buf);
        }
        if commas == 1 {
            let k = text.iter().position(|&c| c == b',').unwrap();
            let (num, rest) = (text[..k].to_vec(), copy63(&text[k + 1..]));
            if mode == 1 && self.area_code(&num) != 0 {
                obj.set_cstr(4, &num);
                obj.set_u8(0x4c, 1);
                obj.set_cstr(0x4d, &rest);
                return true;
            }
        }
        Self::fail(obj, &buf)
    }

    /// Recognizer 7 (0x10004510): integers and decimals (kind 10, or 3 for
    /// dot-grouped integers).
    fn r_real(&self, obj: &mut NumObj) -> bool {
        obj.set_u8(0x4c, 0);
        obj.set_u8(0x8c, 0);
        obj.set_u32(0, 10);
        obj.set_u32(0x48, 10);
        for o in [0xcc, 0xcd, 0xd0] {
            obj.set_u8(o, 0);
        }
        let buf = Self::strip_sign(obj, Some(0xcc));
        let text = obj.text().to_vec();
        let (dots, commas) = dots_commas(&text);
        let mode = obj.mode();
        let plain = strpbrk(&text, self.s(NOT_PLAIN)).is_none();
        if dots == 0 {
            if commas == 0 {
                match mode {
                    3 | 10 => {
                        obj.set_cstr(0x4c, &text);
                        return true;
                    }
                    0 | 8 if plain && text.len() < 6 => {
                        obj.set_cstr(0x4c, &text);
                        return true;
                    }
                    _ => return Self::fail(obj, &buf),
                }
            }
            if commas == 1 {
                let k = text.iter().position(|&c| c == b',').unwrap();
                let (int, dec) = (text[..k].to_vec(), copy63(&text[k + 1..]));
                if matches!(mode, 0 | 7 | 8 | 10) && plain {
                    obj.set_cstr(0x4c, &int);
                    obj.set_cstr(0x8c, &dec);
                    return true;
                }
            }
            return Self::fail(obj, &buf);
        }
        if commas == 0 {
            if matches!(mode, 0 | 3 | 10) && elqnum::is_big_integer(&text) {
                obj.set_cstr(0x4c, &text);
                obj.set_u32(0x48, 3);
                return true;
            }
            return Self::fail(obj, &buf);
        }
        if commas == 1 {
            let (Some(p), Some(q)) = (text.iter().rposition(|&c| c == b'.'), text.iter().rposition(|&c| c == b',')) else {
                return Self::fail(obj, &buf);
            };
            if p < q {
                let int = copy63(&text[..q]);
                let dec = &text[q + 1..];
                if elqnum::is_big_integer(&int) && ((mode == 0 && dec.len() <= 2) || mode == 3) {
                    obj.set_cstr(0x4c, &int);
                    obj.set_cstr(0x8c, dec);
                    obj.set_u32(0x48, 3);
                    return true;
                }
            }
        }
        Self::fail(obj, &buf)
    }

    /// Recognizer 8 (0x10004350): digits of reading type 2 (kind 2).
    fn r_code(obj: &mut NumObj) -> bool {
        obj.set_u32(0, 2);
        obj.set_u32(0x48, 2);
        for o in [0x8c, 0x4c, 0x8d, 0x8e] {
            obj.set_u8(o, 0);
        }
        let buf = Self::strip_sign(obj, Some(0x8d));
        let text = obj.text().to_vec();
        let (dots, commas) = dots_commas(&text);
        let mode = obj.mode();
        if dots == 0 && commas == 0 && mode == 2 {
            return true;
        }
        if dots == 0 && commas == 1 && mode == 2 {
            // 0x100044a0…: split, then rejected all the same
            let k = text.iter().position(|&c| c == b',').unwrap();
            obj.set_u8(0x8c, 1);
            obj.set_cstr(4, &text[..k]);
            obj.set_cstr(0x4c, &copy63(&text[k + 1..]));
        }
        Self::fail(obj, &buf)
    }

    /// The renderer (0x10007110): the reading of `obj` by its kind.
    fn render(&self, obj: &NumObj) -> Vec<u8> {
        let mut out = Vec::new();
        let w = |id: u32| self.w(id).to_vec();
        let sign = |out: &mut Vec<u8>, minus: usize| {
            if obj.u8(minus) == 1 {
                sep(out, 0, &w(7));
            } else if obj.u8(minus + 1) == 1 {
                sep(out, 0, &w(8));
            }
        };
        match obj.kind() {
            0 => elqnum::convert_real_number(&mut out, 0, obj, self),
            1 => {
                self.phone(&mut out, obj.text(), obj.u8(0x4c) != 0);
                let rest = obj.cstr(0x4d).to_vec();
                if !rest.is_empty() {
                    let mut o2 = NumObj::default();
                    o2.set_cstr(4, &rest);
                    o2.set_u32(0x44, obj.mode());
                    let end = out.len();
                    elqnum::convert_real_number(&mut out, end, &o2, self);
                }
            }
            2 => {
                sign(&mut out, 0x8d);
                let pause = elqnum::convert_integer(&mut out, 0, obj.text(), true, self);
                if obj.u8(0x4c) != 0 {
                    sep(&mut out, 0, &w(if pause { 0 } else { 1 }));
                    sep(&mut out, 0, &w(4));
                    self.decimals(&mut out, obj.cstr(0x4c), true);
                }
            }
            3 | 10 => {
                sign(&mut out, 0xcc);
                let end = out.len();
                let pause = if obj.u32(0x48) == 10 {
                    elqnum::convert_integer(&mut out, end, obj.cstr(0x4c), false, self)
                } else {
                    elqnum::convert_big_integer(&mut out, end, obj.cstr(0x4c), self);
                    true
                };
                if obj.u8(0x8c) != 0 {
                    sep(&mut out, 0, &w(if pause { 0 } else { 1 }));
                    sep(&mut out, 0, &w(4));
                    self.decimals(&mut out, obj.cstr(0x8c), false);
                }
            }
            4 => match obj.u32(0x48) {
                5 => self.ordinal(&mut out, obj.text(), true),
                6 => self.ordinal(&mut out, obj.text(), false),
                _ => {}
            },
            7 => {
                sign(&mut out, 0x8d);
                self.time(&mut out, obj.text());
                if obj.u8(0x4c) != 0 {
                    sep(&mut out, 0, &w(0));
                    sep(&mut out, 0, &w(4));
                    self.decimals(&mut out, obj.cstr(0x4c), false);
                }
            }
            8 => {
                if obj.u32(0x88) == 8 {
                    self.date(&mut out, obj.u32(0x98), obj.u32(0x9c), obj.u32(0xa0));
                }
            }
            9 => {
                sign(&mut out, 0xcc);
                elqnum::convert_integer(&mut out, 0, obj.cstr(0x4c), false, self);
                if obj.u8(0x8c) != 0 {
                    sep(&mut out, 0, &w(0));
                    sep(&mut out, 0, &w(4));
                    self.decimals(&mut out, obj.cstr(0x8c), false);
                }
            }
            0xb => {
                if let Some(f) = self.fraction(obj.text()) {
                    out.extend_from_slice(f);
                }
            }
            0xc => {
                let text = obj.text().to_vec();
                sign(&mut out, 0x56);
                if obj.u32(0x5c) == 2 {
                    // drachmas: the amount with its feminine mark
                    let fem = match (text.len(), text.first()) {
                        (2, Some(b'1')) => Some(0x12),
                        (2, Some(b'3')) => Some(0x18),
                        (2, Some(b'4')) => Some(0x1a),
                        _ => None,
                    };
                    if let Some(id) = fem {
                        out.extend(w(id));
                    } else if obj.u8(0x59) != 0 {
                        let end = out.len();
                        elqnum::convert_big_integer(&mut out, end, &text, self);
                    } else {
                        let end = out.len();
                        elqnum::convert_integer(&mut out, end, &text, false, self);
                    }
                } else {
                    let mut text = text;
                    if obj.u8(0x5a) != 0 {
                        for c in text.iter_mut() {
                            *c = match *c {
                                b',' => b'.',
                                b'.' => b',',
                                c => c,
                            };
                        }
                    }
                    self.amount(&mut out, &text, obj.cstr(0x4c));
                }
            }
            _ => {}
        }
        out
    }

    /// 0x100065d0: an ordinal, masculine or feminine, by groups of three
    /// digits from the highest: the hundreds as a cardinal with its last two
    /// letters replaced, the tens and units by their own stems, then the
    /// group's thousandth/millionth/billionth. More than twelve digits read
    /// as digits.
    fn ordinal(&self, out: &mut Vec<u8>, text: &[u8], masc: bool) {
        let g = if masc { 0 } else { 1 };
        let t = cs(text);
        let t = &t[t.iter().position(|&c| c != b'0').unwrap_or(t.len())..];
        let n = t.len();
        if n == 0 {
            return;
        }
        let w1 = self.w(1).to_vec();
        if n > 0xc {
            out.clear();
            elqnum::convert_integer(out, 0, t, true, self);
            return;
        }
        let mut group = (n as i32 - 1) / 3;
        let mut digits = match (n - 1) % 3 {
            0 => b"00".to_vec(),
            1 => b"0".to_vec(),
            _ => Vec::new(),
        };
        digits.extend_from_slice(&t[..t.len().min(0x3f)]);
        let c = |i: usize| digits.get(i).copied().unwrap_or(0);
        let mut p = 0usize;
        // the tens were 11 or 12: the units say nothing
        let mut teen = false;
        while group >= 0 {
            let h = c(p);
            if h != b'0' {
                self.cardinal(out, &[h, b'0', b'0']);
                out.truncate(out.len().saturating_sub(2));
                out.extend_from_slice(self.s(ORD_HUNDREDS_END[g]));
                sep(out, 0, &w1);
            }
            // the tens (0x10006784…)
            match c(p + 1) {
                b'1' => {
                    let ten_times = match c(p + 2) {
                        b'0' if n == 5 => {
                            out.extend_from_slice(self.s(ORD_TEN_TIMES));
                            true
                        }
                        b'1' => {
                            out.extend_from_slice(self.s(ORD_ELEVEN));
                            teen = true;
                            false
                        }
                        b'2' => {
                            out.extend_from_slice(self.s(ORD_TWELVE));
                            teen = true;
                            false
                        }
                        _ => {
                            out.extend_from_slice(self.s(ORD_TEN));
                            false
                        }
                    };
                    if !ten_times {
                        out.extend_from_slice(self.s(ORD_END[g]));
                        sep(out, 0, &w1);
                    }
                }
                d @ b'2'..=b'9' => {
                    out.extend_from_slice(self.s(ORD_TENS[(d - b'2') as usize]));
                    out.extend_from_slice(self.s(ORD_TENS_END[g]));
                    sep(out, 0, &w1);
                }
                _ => {}
            }
            // the units (0x10006a5a…)
            let four_thousands = n == 4 && group == 1;
            let u = c(p + 2);
            let mut said = true;
            let mut skip = false;
            match u {
                b'1' => {
                    if !teen {
                        if n as i32 == group * 3 + 1 && n != 1 {
                            skip = true;
                        } else {
                            out.extend_from_slice(self.s(ORD_UNITS[0]));
                        }
                    }
                }
                b'2' if teen => {}
                b'2'..=b'9' => {
                    let k = (u - b'1') as usize;
                    if four_thousands {
                        out.extend_from_slice(self.s(ORD_TIMES[k - 1]));
                        skip = true;
                    } else {
                        out.extend_from_slice(self.s(ORD_UNITS[k]));
                    }
                }
                _ => said = false,
            }
            if !skip {
                if said && !teen {
                    out.extend_from_slice(self.s(ORD_END[g]));
                }
                sep(out, 0, &w1);
            }
            // 0x10006dc5: the group's word
            p += 3;
            teen = false;
            if (1..=3).contains(&group) {
                out.extend_from_slice(self.s(ORD_GROUPS[group as usize - 1]));
                out.extend_from_slice(self.s(ORD_TENS_END[g]));
            }
            sep(out, 0, &w1);
            group -= 1;
        }
    }

    /// 0x10004ed0: a time `h:mm[:ss]` (also `.` or `,`); 1, 3, 4, 13, 14,
    /// 21, 23 o'clock take their feminine forms.
    fn time(&self, out: &mut Vec<u8>, text: &[u8]) {
        let w = |id: u32| self.w(id).to_vec();
        let seps = self.s(TIME_SEPS).to_vec();
        let buf = copy63(text);
        let Some(p) = strpbrk(&buf, &seps) else {
            return;
        };
        let mut hours = &buf[..p];
        if hours.first() == Some(&b'0') && hours.get(1).is_some_and(|&c| (b'1'..=b'9').contains(&c)) {
            hours = &hours[1..];
        }
        match elqnum::atol(hours) {
            1 => {
                out.clear();
                out.extend_from_slice(self.s(ONE_F_TIME));
            }
            21 => {
                out.clear();
                out.extend_from_slice(self.s(TWENTY_ONE_F));
            }
            h => {
                elqnum::convert_integer(out, 0, hours, false, self);
                match h {
                    3 | 13 | 23 => {
                        out.truncate(out.len().saturating_sub(2));
                        out.extend_from_slice(self.s(EIS_TIME));
                    }
                    4 | 14 => {
                        out.truncate(out.len().saturating_sub(1));
                        out.extend_from_slice(self.s(IS_END));
                    }
                    _ => {}
                }
            }
        }
        let after = &buf[p + 1..];
        let at = |x: &[u8], i: usize| x.get(i).copied().unwrap_or(0);
        let and = |out: &mut Vec<u8>| {
            sep(out, 0, &w(1));
            sep(out, 0, &w(0xd));
            sep(out, 0, &w(1));
        };
        let read = |out: &mut Vec<u8>, x: &[u8]| {
            let end = out.len();
            elqnum::convert_integer(out, end, x, false, self);
        };
        let Some(q) = strpbrk(after, &seps) else {
            if after.is_empty() || (after[0] == b'0' && at(after, 1) == b'0') {
                return;
            }
            and(out);
            let mut m = after;
            if at(after, 1) != 0 && at(after, 2) == b'-' {
                read(out, &after[..2]);
                sep(out, 0, &w(0));
                m = &after[3..];
            }
            read(out, m);
            return;
        };
        let minutes = &after[..q];
        let seconds = &after[q + 1..];
        let (m0, m1) = (at(minutes, 0), at(minutes, 1));
        let (s0, s1) = (at(seconds, 0), at(seconds, 1));
        let m_some = m0 != 0 && !(m0 == b'0' && m1 == b'0');
        let s_some = s0 != 0 && !(s0 == b'0' && s1 == b'0');
        if m_some && s_some {
            sep(out, 0, &w(1));
            read(out, minutes);
            sep(out, 0, &w(0));
            sep(out, 0, &w(0x5e));
            and(out);
            read(out, seconds);
            sep(out, 0, &w(0x5d));
            return;
        }
        let s00 = s0 == b'0' && s1 == b'0';
        if m0 == b'0' && m1 == b'0' {
            // 0x10005116: no minutes, the seconds
            if s00 {
                return;
            }
            and(out);
            read(out, seconds);
            sep(out, 0, &w(0x5d));
            return;
        }
        // 0x100051b5: the minutes, when the seconds are 00
        if !s00 {
            return;
        }
        and(out);
        read(out, minutes);
    }

    /// 0x100021e0: a date: the day (`πρώτη`, feminine 3s and 4s), the
    /// month's genitive, the year.
    fn date(&self, out: &mut Vec<u8>, day: u32, month: u32, year: u32) {
        let w = |id: u32| self.w(id).to_vec();
        if day != 0 {
            if day == 1 {
                out.extend_from_slice(self.s(FIRST_F));
            } else {
                elqnum::convert_integer(out, 0, day.to_string().as_bytes(), false, self);
                match day {
                    3 | 13 | 23 => {
                        out.truncate(out.len().saturating_sub(2));
                        out.extend_from_slice(self.s(EIS_DATE));
                    }
                    4 | 14 => {
                        out.truncate(out.len().saturating_sub(1));
                        out.extend_from_slice(self.s(IS_END));
                    }
                    _ => {}
                }
            }
            sep(out, 0, &w(1));
        }
        if (1..=12).contains(&month) {
            out.extend(w(0x50 + month));
            sep(out, 0, &w(1));
        }
        if year != 0 {
            let end = out.len();
            elqnum::convert_integer(out, end, year.to_string().as_bytes(), false, self);
        }
    }

    /// 0x100048b0: an amount of the currency symbol `cur`: the units (a
    /// dot-grouped number or a cardinal) and the currency's name; two
    /// decimals as `και` and the cents (the euro's names said after both),
    /// others after `τελεία`.
    fn amount(&self, out: &mut Vec<u8>, text: &[u8], cur: &[u8]) {
        let w = |id: u32| self.w(id).to_vec();
        let text = cs(text);
        let euro = cur == self.s(EURO);
        let units_of = |out: &mut Vec<u8>, units: &[u8]| {
            let end = out.len();
            if elqnum::is_big_integer(units) {
                elqnum::convert_big_integer(out, end, units, self);
            } else {
                self.cardinal(out, units);
            }
        };
        let Some(k) = strpbrk(text, self.s(COMMA)) else {
            units_of(out, text);
            self.currency_name(out, text, cur, false);
            return;
        };
        let units = &text[..k];
        let dec = &text[k + 1..];
        // the cents: a cardinal, or the digit after a zero
        let cents = |out: &mut Vec<u8>| {
            if dec[0] != b'0' {
                self.cardinal(out, dec);
                self.currency_name(out, dec, cur, euro);
            } else {
                out.extend_from_slice(self.digit_word(dec[1]));
                self.currency_name(out, &dec[1..], cur, euro);
            }
        };
        if dec.len() == 2 {
            if units == self.s(ZERO) {
                cents(out);
                return;
            }
            units_of(out, units);
            if euro {
                self.currency_name(out, units, cur, false);
            }
            if dec == self.s(ZERO2) {
                return;
            }
            out.extend(w(1));
            out.extend(w(0xd));
            out.extend(w(1));
            cents(out);
            return;
        }
        units_of(out, units);
        out.extend(w(1));
        out.extend(w(3));
        out.extend(w(1));
        self.decimals(out, dec, false);
        self.currency_name(out, dec, cur, false);
    }
}

/// Replays the original's slot-9 calls traced by `oracle/loqref.c`
/// (`LOQ_TRACE_LES=1 LOQ_NUM_ONLY=1 LOQ_LES_DLL=LTTS7Greek.dll
/// LOQ_LES_TABLE=10021d00 LOQ_LES_N=30`, voice Afroditi) on the benchmark
/// and a list of number texts, kept in `numeri.les` (each distinct call
/// once): every `> NUM` call with its `T` token buffer, compared with its
/// `< NUM` reading, tokens used (`c18`) and rewritten tokens (`U`). Needs
/// `LTTS7Greek.dll` (`QQ_LTTS7`, default `/tmp/qqv/LTTS7`); skips without
/// it.
#[cfg(test)]
mod tests {
    use super::*;

    fn unesc(s: &str) -> Vec<u8> {
        let b = s.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'\\' && b.get(i + 1) == Some(&b'x') {
                out.push(u8::from_str_radix(&s[i + 2..i + 4], 16).unwrap());
                i += 4;
            } else if b[i] == b'\\' {
                out.push(b[i + 1]);
                i += 2;
            } else {
                out.push(b[i]);
                i += 1;
            }
        }
        out
    }

    /// The quoted value of `key="…"` in a trace line.
    fn field(line: &str, key: &str) -> Option<Vec<u8>> {
        let at = line.find(&format!(" {key}=\""))? + key.len() + 3;
        let b = line.as_bytes();
        let mut j = at;
        while j < b.len() && b[j] != b'"' {
            j += if b[j] == b'\\' { 2 } else { 1 };
        }
        Some(unesc(&line[at..j]))
    }

    fn num(line: &str, key: &str) -> usize {
        line.split_whitespace().find_map(|w| w.strip_prefix(&format!("{key}="))).and_then(|v| v.parse().ok()).unwrap_or(0)
    }

    /// `  T <k> <type> <offset> <flag> t="…"`
    fn tok(line: &str) -> Tok {
        let w: Vec<&str> = line.split_whitespace().collect();
        Tok {
            text: field(line, "t").unwrap_or_default(),
            kind: w[2].parse().unwrap(),
            expanded: w[4] != "0",
        }
    }

    #[test]
    fn greek_slot9_trace() {
        let root = std::env::var("QQ_LTTS7").unwrap_or_else(|_| "/tmp/qqv/LTTS7".into());
        let Ok(dll) = std::fs::read(format!("{root}/bin/LTTS7Greek.dll")) else { return };
        let files = crate::lang::ModuleFiles {
            dll,
            ..Default::default()
        };
        let el = Greek::from_files(&files).expect("LTTS7Greek.dll");
        let text = include_str!("numeri.les");
        let lines: Vec<&str> = text.lines().collect();
        let (mut calls, mut bad) = (0, 0);
        let mut k = 0;
        while k < lines.len() {
            let line = lines[k];
            k += 1;
            if !line.starts_with("> NUM") {
                continue;
            }
            let mut toks = Vec::new();
            while k < lines.len() && lines[k].starts_with("  T ") {
                toks.push(tok(lines[k]));
                k += 1;
            }
            let ret = lines[k];
            k += 1;
            let mut after = Vec::new();
            while k < lines.len() && lines[k].starts_with("  U ") {
                after.push(tok(lines[k]));
                k += 1;
            }
            let (kind, idx) = (num(line, "type") as u32, num(line, "idx"));
            let dmy = field(line, "a3").unwrap_or_default();
            let want = field(ret, "ret").unwrap_or_default();
            let want_used = num(ret, "c18");
            let (got, used) = leggi_numero(&el, &mut toks, idx, kind, &dmy).unwrap();
            // the engine then leaves the last word in the token; the `U`
            // dump is taken before that, so compare the others
            let same_toks = toks.iter().zip(&after).enumerate().all(|(j, (a, b))| j == idx || a.text == b.text);
            calls += 1;
            if got != want || used != want_used || !same_toks {
                bad += 1;
                if bad <= 40 {
                    eprintln!(
                        "{:?} (type {kind}): ours {:?} used {used}, original {:?} used {want_used}{}",
                        String::from_utf8_lossy(&field(line, "a6").unwrap_or_default()),
                        String::from_utf8_lossy(&got),
                        String::from_utf8_lossy(&want),
                        if same_toks { "" } else { " (tokens differ)" }
                    );
                }
            }
        }
        eprintln!("{calls} slot-9 calls, {bad} differ");
        assert!(calls > 0);
        assert_eq!(bad, 0);
    }
}
