//! Numbers: the reader LTTS7Polish.dll's `LesIniNumeri` (0x1000c450)
//! installs as slot 9 (0x1000bf00), the only one the engine calls, and the
//! slots 0–7 it and LTTS7Util's `ELQNum*` (`super::super::elqnum`) call
//! back (MECHANICS.md §4g). The module is built on the same skeleton as
//! Greek's (`greek::numeri`): the same nine recognizers in the same order
//! (0x1000b150), the same renderer switch (0x1000b230). Unlike Greek it
//! looks at the tokens around the number by their raw index (tags
//! included); its ordinals (0x10001cc0, 0x100065a0) look the number up as a
//! string among 0–100 and read anything else as a cardinal. Strings are the
//! module's Windows-1250 bytes.

use super::Polish;
use crate::lang::common::Tok;
use crate::lang::elqnum::{self, NumObj, NumSlots};

/// `{ id, word }` pairs read by slot 0 (0x10008690): 0x53 of them, `""`
/// (0x1001efd7) for any other id.
const WORDS: u32 = 0x1001f078;
const WORD_COUNT: u32 = 0x53;
const NO_WORD: u32 = 0x1001efd7;
/// The currency names slot 5 (0x1000a8c0) reads: 4 rows of `{ symbol,
/// unit, units, cent, cents }`; `"1"` (0x1002055c) takes the singular.
const CURRENCIES: u32 = 0x1001f028;
const CURRENCY_ROWS: u32 = 4;
const ONE: u32 = 0x1002055c;
/// The area-code prefixes 0x100085a0 searches (`ELQSearchString` mode 2).
const AREA_CODES: u32 = 0x1001f020;
const AREA_CODE_COUNT: u32 = 2;
/// 0x100016b0 / 0x100019d0: fractions and how they are read.
const FRACTIONS: [(u32, u32); 14] = [
    (0x1001f428, 0x1001f41c),
    (0x1001f418, 0x1001f408),
    (0x1001f404, 0x1001f3f4),
    (0x1001f3f0, 0x1001f3e4),
    (0x1001f3e0, 0x1001f3d0),
    (0x1001f3cc, 0x1001f3bc),
    (0x1001f3b8, 0x1001f3ac),
    (0x1001f3a8, 0x1001f398),
    (0x1001f390, 0x1001f380),
    (0x1001f378, 0x1001f364),
    (0x1001f35c, 0x1001f350),
    (0x1001f348, 0x1001f338),
    (0x1001f334, 0x1001f324),
    (0x1001f320, 0x1001f310),
];
/// Currency words after the number: `euro`, `EUR`, `eur` (the amount gets
/// `€`, 0x1001efe8), `dolar`, `dolary`, `USD`, `usd` (`$`, 0x1001f00c).
const EURO_WORDS: [u32; 3] = [0x1001e578, 0x10021118, 0x10021114];
const DOLLAR_WORDS: [u32; 4] = [0x1001f004, 0x1001effc, 0x10021110, 0x1002110c];
const EURO: u32 = 0x1001efe8;
const DOLLAR: u32 = 0x1001f00c;
/// Recognizer 2 (0x100094d0): `°`, `º` (Windows-1250 `ş`), and the string
/// the token after them must not be (`c`).
const DEGREE: u32 = 0x10021108;
const MASC_ORDINAL: u32 = 0x10021104;
const CELSIUS: u32 = 0x10021100;
/// Recognizer 1 (0x100090b0): `h` after an hour, `:` before its minutes.
const HOUR_MARK: u32 = 0x100210fc;
const HOUR_COLON: u32 = 0x100210f8;
/// Recognizer 0 (0x10008f50): the date order a `YYYYMMDD` date needs.
const VXML_ORDER: u32 = 0x100210f4;
/// `":/-+"` (0x10021120), `".,:"` (0x10021130).
const NOT_PLAIN: u32 = 0x10021120;
const TIME_SEPS: u32 = 0x10021130;
/// The cardinal's (0x1000b790) plural forms after 2, 3, 4: `miliardy `,
/// ` miliony `, ` tysiące `.
const MILIARDY: u32 = 0x1002114c;
const MILIONY: u32 = 0x10021140;
const TYSIACE: u32 = 0x10021134;
/// The ordinals' strings (0x10001cc0, 0x100065a0), each block in rising
/// addresses from 100 down: masculine `{ word, "n" }` pairs 100…0 (202
/// strings), the feminine words 100…0 (101), the genitive words 100…1
/// (100).
const ORD_MASC: u32 = 0x1001fc04;
const ORD_FEM: u32 = 0x1001f42c;
const ORD_GEN: u32 = 0x1002056c;

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

/// The module's number code over its tables (the slot table LTTS7Util
/// calls back is [`NumSlots`] on this).
struct Num<'a>(&'a Polish);

impl Num<'_> {
    fn s(&self, va: u32) -> &[u8] {
        self.0.pe.cstr(va)
    }

    /// Slot 0 (0x10008690): the word numbered `id`, `""` when none.
    fn w(&self, id: u32) -> &[u8] {
        (0..WORD_COUNT).find(|&k| self.0.pe.u32(WORDS + 8 * k) == id).map(|k| self.s(self.0.pe.u32(WORDS + 8 * k + 4))).unwrap_or(self.s(NO_WORD))
    }

    /// The `k`th string of a block of consecutive C strings (each padded
    /// with NULs to the next one) starting at `va`.
    fn nth(&self, mut va: u32, k: usize) -> &[u8] {
        for _ in 0..k {
            va += self.s(va).len() as u32 + 1;
            while self.0.pe.u8(va) == 0 {
                va += 1;
            }
        }
        self.s(va)
    }

    /// The position (100 − n) of the key `"n"` equal to `s` among the
    /// masculine pairs, n from 100 down to `low`.
    fn ord_key(&self, s: &[u8], low: usize) -> Option<usize> {
        let mut va = ORD_MASC;
        for k in 0..=100 - low {
            let word = self.s(va).len() as u32 + 1;
            va += word;
            while self.0.pe.u8(va) == 0 {
                va += 1;
            }
            let key = self.s(va);
            if key == s {
                return Some(k);
            }
            va += key.len() as u32 + 1;
            while self.0.pe.u8(va) == 0 {
                va += 1;
            }
        }
        None
    }

    /// 0x1000aad0: the word of `n` (1–20 and the tens), `""` otherwise.
    fn unit(&self, n: i32) -> &[u8] {
        let id = match n {
            1 => 0x11,
            2 => 0x15,
            3 => 0x17,
            4 => 0x19,
            5 => 0x1b,
            6 => 0x1c,
            7 => 0x1d,
            8 => 0x1f,
            9 => 0x20,
            10..=20 => 0x22 + (n as u32 - 10),
            30 => 0x2d,
            40 | 50 | 60 | 70 | 80 | 90 => 0x34 + (n as u32 / 10 - 4),
            _ => return self.s(NO_WORD),
        };
        self.w(id)
    }

    /// Slot 3 (0x1000b750): one digit's word.
    fn digit_word(&self, d: u8) -> &[u8] {
        match d {
            b'0' => self.w(0xf),
            b'1' => self.w(0x10),
            _ => self.unit(d as i32 - b'0' as i32),
        }
    }

    /// 0x1000acf0: the hundreds digit at `p` read into `out` (the string at
    /// `at`), then a space unless it is `sto`; false for `0`.
    fn hundreds(&self, out: &mut Vec<u8>, at: usize, p: &[u8]) -> bool {
        let c = |i: usize| p.get(i).copied().unwrap_or(0);
        match c(0) {
            b'0' => return false,
            b'1' => out.extend_from_slice(self.w(if c(1) == b'0' && c(2) == b'0' { 0x3a } else { 0x3b })),
            b'2' => out.extend_from_slice(self.w(0x3e)),
            b'3' => out.extend_from_slice(self.w(0x3f)),
            b'4' => out.extend_from_slice(self.w(0x40)),
            b'5' => out.extend_from_slice(self.w(0x41)),
            b'6' => out.extend_from_slice(self.w(0x42)),
            b'8' => out.extend_from_slice(self.w(0x43)),
            d => {
                out.extend_from_slice(self.unit(d as i32 - b'0' as i32));
                out.extend_from_slice(self.w(0x3c));
            }
        }
        if c(0) != b'1' {
            let w1 = self.w(1).to_vec();
            sep(out, at, &w1);
        }
        true
    }

    /// 0x1000af40: the reading of the two digits at `p` ("" for 00): the
    /// tens, a space and the units.
    fn two(&self, p: &[u8]) -> Vec<u8> {
        let t0 = p.first().copied().unwrap_or(0);
        let t1 = if t0 == 0 { 0 } else { p.get(1).copied().unwrap_or(0) };
        let n = elqnum::atol(cs(&[t0, t1])) as u32;
        let mut b = Vec::new();
        if n == 0 {
            return b;
        }
        if t0 == b'1' || n < 10 {
            b.extend_from_slice(self.unit(n as i32));
            return b;
        }
        b.extend_from_slice(self.unit((t0 as i32 - b'0' as i32) * 10));
        let mid = match (t0 == b'2', t1) {
            (true, b'0') => 0x2e,
            (true, b'1' | b'8') => 0x30,
            (true, _) => 0x2f,
            (false, b'0') => 0x31,
            (false, b'1' | b'8') => 0x33,
            (false, _) => 0x32,
        };
        b.extend_from_slice(self.w(mid));
        if t1 != b'0' {
            b.extend_from_slice(self.unit(t1 as i32 - b'0' as i32));
        }
        b
    }

    /// Slot 2 (0x1000b790): the cardinal reading of the digits `s`
    /// appended to `out` (the string starts at the end of `out`). The
    /// group words agree with the group's last digit: 2–4 take
    /// `miliardy`/`miliony`/`tysiące` unless a ten digit 1 was read before
    /// (anywhere in the number), others the genitive plural.
    fn cardinal(&self, out: &mut Vec<u8>, s: &[u8]) {
        let at = out.len();
        let s = cs(s);
        let len = s.len();
        let ch = |i: usize| s.get(i).copied().unwrap_or(0);
        let rest = |p: usize| s.get(p..).unwrap_or(&[]);
        let w = |id: u32| self.w(id).to_vec();
        // the C looks at the byte before the string's end, whatever it is
        let space_or_sep = |out: &mut Vec<u8>| {
            if out.last() != Some(&b' ') {
                sep(out, at, &w(1));
            }
        };
        // d: two digits were just read; e: the group so far is zero; teen:
        // a ten digit 1 was read; c: there were hundreds of thousands;
        // a/b: a lone 01 million/thousand
        let (mut a, mut b, mut c, mut d, mut e, mut teen) = (false, false, false, false, true, false);
        let mut p = 0usize;
        if len > 0xb {
            self.hundreds(out, at, rest(0));
            p = 1;
        }
        if len > 10 {
            d = true;
            out.extend(self.two(rest(p)));
            if ch(p) == b'1' {
                teen = true;
            }
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
                if matches!(ch(p), b'2'..=b'4') && !teen {
                    out.extend_from_slice(self.s(MILIARDY));
                } else {
                    out.extend(w(0x4e));
                }
            }
            sep(out, at, &w(0));
            p += 1;
            d = false;
            e = true;
        }
        if len > 8 {
            e = !self.hundreds(out, at, rest(p));
            p += 1;
        }
        if len > 7 {
            d = true;
            if e && ch(p) == b'0' && ch(p + 1) == b'1' {
                a = true;
            } else {
                out.extend(self.two(rest(p)));
            }
            if ch(p) != b'0' {
                e = false;
            }
            if ch(p) == b'1' {
                teen = true;
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
                    if matches!(ch(p), b'2'..=b'4') && !teen {
                        out.extend_from_slice(self.s(MILIONY));
                    } else {
                        out.extend(w(0x4b));
                    }
                }
                sep(out, at, &w(0));
            }
            p += 1;
            d = false;
            e = true;
        }
        if len > 5 {
            c = true;
            e = !self.hundreds(out, at, rest(p));
            p += 1;
        }
        if len > 4 {
            d = true;
            if e && ch(p) == b'0' && ch(p + 1) == b'1' {
                b = true;
            } else {
                out.extend(self.two(rest(p)));
            }
            if ch(p) != b'0' {
                e = false;
            }
            if ch(p) == b'1' {
                teen = true;
            }
            p += 1;
        }
        if len >= 4 {
            let cc = ch(p);
            if cc != b'0' || !e {
                if cc == b'1' && !d && !c {
                    // 0x1000bcb0: tysiąc, a space unless the rest is 0xx/x00…
                    let (x1, x2, x3) = (ch(p + 1) != b'0', ch(p + 2) != b'0', ch(p + 3) != b'0');
                    out.extend(w(0x47));
                    let quiet = (!x1 && (!x2 || !x3)) || (!x2 && !x3 && ch(p + 1) == b'1');
                    if !quiet {
                        sep(out, at, &w(1));
                    }
                } else {
                    if !d {
                        out.extend_from_slice(self.unit(cc as i32 - b'0' as i32));
                    }
                    // 0x1000be2a
                    if b {
                        out.extend(w(0x47));
                    } else if matches!(cc, b'2'..=b'4') && !teen {
                        out.extend_from_slice(self.s(TYSIACE));
                    } else {
                        out.extend(w(0x48));
                    }
                    sep(out, at, &w(if !c || b { 1 } else { 0 }));
                }
            }
            p += 1;
        }
        if len > 2 {
            self.hundreds(out, at, rest(p));
            p += 1;
        }
        if len > 1 {
            out.extend(self.two(rest(p)));
        }
        let w0 = w(0);
        if !w0.is_empty() && out[at..].ends_with(&w0) {
            out.truncate(out.len() - w0.len());
        }
    }

    /// 0x100085a0: the length of the area code `digits` starts with (the
    /// entry `ELQSearchString` flag 2 finds; with only two entries it is a
    /// linear search for an entry that is a prefix), 0 when none.
    fn area_code(&self, digits: &[u8]) -> usize {
        let key = copy63(digits);
        (0..AREA_CODE_COUNT).map(|k| self.s(self.0.pe.u32(AREA_CODES + 4 * k))).find(|e| key.starts_with(e)).map_or(0, |e| e.len())
    }

    /// Slot 1 (0x10008610): a telephone number: the area code, then the
    /// rest (`ELQNumConvertPhoneNumber` steps 0 and 1), appended to `out`.
    fn phone(&self, out: &mut Vec<u8>, digits: &[u8], pairs: bool) {
        let digits = cs(digits);
        let area = self.area_code(digits);
        let mut len = area;
        let mut s = Vec::new();
        elqnum::convert_phone_number(self, &mut s, digits, pairs, &mut len, 0);
        elqnum::convert_phone_number(self, &mut s, digits.get(area..).unwrap_or(&[]), pairs, &mut len, 1);
        out.extend(s);
    }

    /// Slot 5 (0x1000a8c0): the currency's name after `amount`, a space
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

    /// Slot 6 (0x1000c290): decimals: leading zeros one by one, then up to
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

    /// Slot 7 (0x100086c0): the parts of a date in the order `dmy`.
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
    /// Slot 4 (0x10008660).
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

/// What a recognizer works with: the engine's token buffer (the module
/// indexes it directly, tags included), the token's index, its date order
/// and what slot 9 reports as used (ctx+0x18).
struct Ctx<'t> {
    toks: &'t mut [Tok],
    i: usize,
    dmy: Vec<u8>,
    used: usize,
}

impl Ctx<'_> {
    fn text(&self, j: usize) -> &[u8] {
        self.toks.get(j).map_or(b"", |t| t.text.as_slice())
    }
    fn kind(&self, j: usize) -> u32 {
        self.toks.get(j).map_or(0, |t| t.kind)
    }
}

/// Slot 9 (0x1000bf00): the reading of the number token `i` for reading
/// type `kind` and date order `dmy`, and how many tokens after it it used.
pub(super) fn leggi_numero(p: &Polish, toks: &mut [Tok], i: usize, kind: u32, dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
    Some(Num(p).leggi_numero(toks, i, kind, dmy))
}

impl Num<'_> {
    fn leggi_numero(&self, toks: &mut [Tok], i: usize, kind: u32, dmy: &[u8]) -> (Vec<u8>, usize) {
        let text = toks[i].text.clone();
        let mut cx = Ctx {
            toks,
            i,
            dmy: dmy.to_vec(),
            used: 0,
        };
        let mut obj = NumObj::default();
        let mut mode = kind;
        if kind == 5 || kind == 6 {
            mode = 4;
            obj.set_u32(0x48, kind);
        }
        let n1 = cx.text(i + 1).to_vec();
        if elqnum::is_currency(&n1)
            || (i != 0 && elqnum::is_currency(cx.text(i - 1)))
            || [EURO_WORDS[0], DOLLAR_WORDS[0], DOLLAR_WORDS[1], EURO_WORDS[2], DOLLAR_WORDS[3], DOLLAR_WORDS[2], EURO_WORDS[1]].iter().any(|&a| n1 == self.s(a))
        {
            mode = 0xc;
        }
        obj.set_u32(0x44, mode);
        obj.set_cstr(4, &text);
        // 0x1000b150: the recognizers, the one the reading type names first
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

    /// Recognizer 0 (0x10008f50): a date (kind 8).
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

    /// Recognizer 1 (0x100090b0): a time (kind 7); an hour followed by `h`
    /// (and minutes) uses those tokens.
    fn r_time(&self, cx: &mut Ctx, obj: &mut NumObj) -> bool {
        let i = cx.i;
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
        if h < 0x19 && h > 0 && cx.text(i + 1) == self.s(HOUR_MARK) {
            let t2 = cx.text(i + 2).to_vec();
            let mut t = text.clone();
            t.extend_from_slice(self.s(HOUR_COLON));
            if elqnum::is_number(&t2, b"", true) && elqnum::atol(&t2) < 0x3c && cx.text(i + 3) != self.s(HOUR_MARK) {
                t.extend_from_slice(&t2);
                cx.used = 2;
            } else {
                cx.used = 1;
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

    /// Recognizer 2 (0x100094d0): an ordinal (kind 4): a reading type 4,
    /// or `°`/`º` (not before `c`) or `'` after the number.
    fn r_ordinal(&self, cx: &mut Ctx, obj: &mut NumObj) -> bool {
        let i = cx.i;
        obj.set_u32(0, 4);
        let buf = Self::strip_sign(obj, None);
        let (dots, commas) = dots_commas(obj.text());
        if dots + commas != 0 {
            return Self::fail(obj, &buf);
        }
        if obj.mode() == 4 {
            return true;
        }
        let t1 = cx.text(i + 1);
        if (t1 == self.s(DEGREE) || t1 == self.s(MASC_ORDINAL)) && !cx.text(i + 2).eq_ignore_ascii_case(self.s(CELSIUS)) {
            obj.set_u32(0x48, 5);
            return true;
        }
        if t1.first() == Some(&b'\'') && cx.kind(i + 2) != 1 {
            obj.set_u32(0x48, 5);
            return true;
        }
        Self::fail(obj, &buf)
    }

    /// Recognizer 3 (0x10009680): an amount (kind 0xc): a currency symbol
    /// before or after it, or a euro/dollar word after it (the token after
    /// the number becomes `n` and is used).
    fn r_currency(&self, cx: &mut Ctx, obj: &mut NumObj) -> bool {
        let i = cx.i;
        obj.set_u32(0, 0xc);
        obj.set_u32(0x48, 0xf);
        for o in [0x56, 0x57, 0x58, 0x5a] {
            obj.set_u8(o, 0);
        }
        obj.set_u8(0x4c, 0);
        let buf = Self::strip_sign(obj, Some(0x56));
        let text = obj.text().to_vec();
        let (dots, commas) = dots_commas(&text);
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
        if i != 0 && elqnum::is_currency(cx.text(i - 1)) {
            let p = cx.text(i - 1).to_vec();
            obj.set_cstr(0x4c, &p);
            return true;
        }
        let t1 = cx.text(i + 1).to_vec();
        let cur = if elqnum::is_currency(&t1) {
            t1
        } else if EURO_WORDS.iter().any(|&a| t1 == self.s(a)) {
            self.s(EURO).to_vec()
        } else if DOLLAR_WORDS.iter().any(|&a| t1 == self.s(a)) {
            self.s(DOLLAR).to_vec()
        } else {
            return Self::fail(obj, &buf);
        };
        obj.set_cstr(0x4c, &cur);
        if let Some(t) = cx.toks.get_mut(i + 1) {
            t.text = b"n".to_vec();
        }
        cx.used = 1;
        true
    }

    /// Recognizer 4 (0x10009a20): numbers of reading type 9 (kind 9).
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

    /// Recognizer 5 (0x10009cd0): a fraction of the list (kind 0xb).
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

    /// 0x100016b0: how the fraction `s` is read (0x100019d0: whether it is
    /// one).
    fn fraction(&self, s: &[u8]) -> Option<&[u8]> {
        FRACTIONS.iter().find(|&&(k, _)| s == self.s(k)).map(|&(_, v)| self.s(v))
    }

    /// Recognizer 6 (0x10009df0): a telephone number (kind 1): more than six
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

    /// Recognizer 7 (0x1000a200): integers and decimals (kind 10, or 3 for
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

    /// Recognizer 8 (0x1000a030): digits of reading type 2 (kind 2).
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
            // 0x1000a160…: split, then rejected all the same
            let k = text.iter().position(|&c| c == b',').unwrap();
            obj.set_u8(0x8c, 1);
            obj.set_cstr(4, &text[..k]);
            obj.set_cstr(0x4c, &copy63(&text[k + 1..]));
        }
        Self::fail(obj, &buf)
    }

    /// The renderer (0x1000b230): the reading of `obj` by its kind.
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
                sign(&mut out, 0x56);
                let mut text = obj.text().to_vec();
                if obj.u8(0x5a) != 0 {
                    for c in text.iter_mut() {
                        *c = match *c {
                            b',' => b'.',
                            b'.' => b',',
                            c => c,
                        };
                    }
                }
                elqnum::convert_currency(self, &mut out, &text, obj.cstr(0x4c));
            }
            _ => {}
        }
        out
    }

    /// 0x10001cc0: the ordinal of `text` (masculine or feminine) in place
    /// of `out`'s whole string: the word of 0–100 when `text` is one of
    /// those numbers as written, else its cardinal.
    fn ordinal(&self, out: &mut Vec<u8>, text: &[u8], masc: bool) {
        out.clear();
        let text = cs(text);
        match self.ord_key(text, 0) {
            Some(k) if masc => out.extend_from_slice(self.nth(ORD_MASC, 2 * k)),
            Some(k) => out.extend_from_slice(self.nth(ORD_FEM, k)),
            None => {
                elqnum::convert_integer(out, 0, text, false, self);
            }
        }
    }

    /// 0x100065a0: the genitive ordinal of `text` without its leading
    /// zeros (1–100), else its cardinal, appended to `out`.
    fn genitive(&self, out: &mut Vec<u8>, text: &[u8]) {
        let t = cs(text);
        let t = &t[t.iter().position(|&c| c != b'0').unwrap_or(t.len())..];
        match self.ord_key(t, 1) {
            Some(k) => out.extend_from_slice(self.nth(ORD_GEN, k)),
            None => {
                let end = out.len();
                elqnum::convert_integer(out, end, t, false, self);
            }
        }
    }

    /// 0x1000a5b0: a time `h:mm[:ss]` (also `.` or `,`): the hour's
    /// feminine ordinal, the minutes and seconds as cardinals.
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
        self.ordinal(out, hours, false);
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
            and(out);
            read(out, seconds);
            sep(out, 0, &w(0x5d));
            return;
        }
        let s00 = s0 == b'0' && s1 == b'0';
        if m0 == b'0' && m1 == b'0' {
            // 0x1000a711: no minutes, the seconds
            if !s00 {
                and(out);
                read(out, seconds);
                sep(out, 0, &w(0x5d));
            }
            return;
        }
        // 0x1000a79e: the minutes, when the seconds are 00
        if s00 {
            and(out);
            read(out, minutes);
        }
    }

    /// 0x10008730: a date: the day's masculine ordinal, the month's
    /// genitive, the year's hundreds as a cardinal and the rest as a
    /// genitive ordinal.
    fn date(&self, out: &mut Vec<u8>, day: u32, month: u32, year: u32) {
        let w = |id: u32| self.w(id).to_vec();
        if day != 0 {
            self.ordinal(out, (day as i32).to_string().as_bytes(), true);
            sep(out, 0, &w(1));
        }
        if month.wrapping_sub(1) < 12 {
            out.extend(w(0x50 + month));
            sep(out, 0, &w(1));
        }
        if year != 0 {
            let y = year as i32;
            let end = out.len();
            elqnum::convert_integer(out, end, (y / 100 * 100).to_string().as_bytes(), false, self);
            self.genitive(out, (y % 100).to_string().as_bytes());
        }
    }
}

/// Replays the original's slot-9 calls traced by `oracle/loqref.c`
/// (`LOQ_TRACE_LES=1 LOQ_NUM_ONLY=1 LOQ_LES_DLL=LTTS7Polish.dll
/// LOQ_LES_TABLE=10022d78 LOQ_LES_N=30`, voice Krzysztof) on the benchmark
/// and a list of number texts, kept in `numeri.les` (each distinct call
/// once): every `> NUM` call with its `T` token buffer, compared with its
/// `< NUM` reading, tokens used (`c18`) and rewritten tokens (`U`). Needs
/// `LTTS7Polish.dll` (`QQ_LTTS7`, default `/tmp/qqv/LTTS7`); skips without
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
    fn polish_slot9_trace() {
        let root = std::env::var("QQ_LTTS7").unwrap_or_else(|_| "/tmp/qqv/LTTS7".into());
        let Ok(dll) = std::fs::read(format!("{root}/bin/LTTS7Polish.dll")) else { return };
        let files = crate::lang::ModuleFiles {
            dll,
            ..Default::default()
        };
        let pl = Polish::from_files(&files).expect("LTTS7Polish.dll");
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
            let (got, used) = leggi_numero(&pl, &mut toks, idx, kind, &dmy).unwrap();
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
