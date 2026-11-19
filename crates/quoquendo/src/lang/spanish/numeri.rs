//! Numbers: the reader LTTS7Spanish.dll's `LesIniNumeri` (0x10007ec0)
//! installs as slot 9 (0x10007490), the only one the engine calls, and the
//! slots 0–7 it and LTTS7Util's `ELQNum*` (`super::super::elqnum`) call
//! back (MECHANICS.md §4d). Strings are the engine's Windows-1252 bytes;
//! where the C code writes through a `char *` into a buffer, the port keeps
//! the whole buffer and the offset that pointer has in it (`at`).

use super::Spanish;
use crate::lang::common::Tok;
use crate::lang::elqnum::{self, NumObj, NumSlots};

/// `{ id, word }` pairs read by slot 0 (0x10003470): 0x57 of them.
const WORDS: u32 = 0x10024f80;
const WORD_COUNT: u32 = 0x57;
/// 0x100060f0: the unit/tens word of 1–0x5a, through the index bytes at
/// 0x100062a8 and the jump table at 0x10006238 (each target is `push id;
/// call 0x10003470`, the id the byte after the push opcode).
const UNIT_INDEX: u32 = 0x100062a8;
const UNIT_JUMPS: u32 = 0x10006238;
const UNIT_NONE: u32 = 0x10006231;
/// The currency names slot 5 (0x10005f10) reads: 11 rows of `{ symbol,
/// unit, units, cent, cents }`.
const CURRENCIES: u32 = 0x10024ea0;
/// The area-code prefixes 0x10002910 searches (`ELQSearchString` mode 2).
const AREA_CODES: u32 = 0x10024ac0;
const AREA_CODE_COUNT: u32 = 0xf7;
/// `"o"` (0x100188e4) and `"a"` (0x10018f84): masculine and feminine
/// endings.
const MASC: u32 = 0x100188e4;
const FEM: u32 = 0x10018f84;
/// Slot 9's currency words for its next token (0x10007490): compared
/// before the recognizers run.
const SLOT9_CURRENCY_WORDS: [u32; 26] = [
    0x10024330, 0x10024380, 0x1002430c, 0x10024304, 0x10024378, 0x100257fc, 0x100242e8, 0x100242e0, 0x100257f4, 0x100257f8, 0x100257a8, 0x100257a4,
    0x100257ec, 0x100257f0, 0x100242d4, 0x100242cc, 0x100242c0, 0x100242b8, 0x100257d0, 0x100257cc, 0x100257c4, 0x10025800, 0x100257e4, 0x100257dc,
    ESTERLINA, ESTERLINAS,
];
/// 0x10007490: `libra`, then `esterlina`/`esterlinas` in the token after.
const LIBRA: u32 = 0x10025540;
const ESTERLINA: u32 = 0x100257b8;
const ESTERLINAS: u32 = 0x100257ac;
/// 0x10004510 (recognizer 3): currency words of the next token and the
/// symbol each becomes (`$ £ € ¥ | _ ~`).
const R3_WORDS: [(u32, u8); 24] = [
    (0x10024330, 0x80),
    (0x10025800, 0x80),
    (0x100257fc, 0x80),
    (0x10024380, b'$'),
    (0x10024378, b'$'),
    (0x100257f8, b'$'),
    (0x100257f4, b'$'),
    (0x1002430c, 0xa5),
    (0x10024304, 0xa5),
    (0x100257f0, 0xa5),
    (0x100257ec, 0xa5),
    (0x100257e4, b'$'),
    (0x100257dc, b'$'),
    (0x100242e8, b'|'),
    (0x100242e0, b'|'),
    (0x100257d8, b'|'),
    (0x100257d4, b'|'),
    (0x100257d0, b'|'),
    (0x100257cc, b'|'),
    (0x100257c4, b'|'),
    (0x100242d4, b'_'),
    (0x100242cc, b'_'),
    (0x100242c0, b'~'),
    (0x100242b8, b'~'),
];
/// 0x10004ce7: currency words for the pound.
const R3_POUND: [u32; 4] = [ESTERLINA, ESTERLINAS, 0x100257a8, 0x100257a4];
/// 0x10004340 (recognizer 2): `°`, `º`, and the string the token after
/// them must not be (0x10018c2c).
const DEGREE: u32 = 0x100257a0;
const MASC_ORDINAL: u32 = 0x1002579c;
const CELSIUS: u32 = 0x10018c2c;
/// 0x10003ef0 (recognizer 1): `h` after an hour.
const HOUR_MARK: u32 = 0x10018b18;
/// 0x10003d90 (recognizer 0): the date order a `YYYYMMDD` date needs.
const VXML_ORDER: u32 = 0x10018c30;
/// 0x10001a80 / 0x10001da0: fractions and how they are read.
const FRACTIONS: [(u32, u32); 14] = [
    (0x1002532c, 0x10025324),
    (0x10025320, 0x10025314),
    (0x10025310, 0x10025304),
    (0x10025300, 0x100252f4),
    (0x100252f0, 0x100252e4),
    (0x100252e0, 0x100252d4),
    (0x100252d0, 0x100252c4),
    (0x100252c0, 0x100252b4),
    (0x100252ac, 0x100252a0),
    (0x10025298, 0x10025288),
    (0x10025280, 0x10025270),
    (0x10025268, 0x1002525c),
    (0x10025258, 0x1002524c),
    (0x10025248, 0x10025238),
];
/// 0x10002090: ordinal stems by digit 1–9 for thousands, hundreds, tens
/// and units.
const ORD_THOUSANDS: [u32; 9] = [0x100254d0, 0x100254c4, 0x100254b8, 0x100254a8, 0x10025498, 0x1002548c, 0x1002547c, 0x10025470, 0x10025460];
const ORD_HUNDREDS: [u32; 9] = [0x10025454, 0x10025448, 0x1002543c, 0x1002542c, 0x1002541c, 0x10025410, 0x10025400, 0x100253f0, 0x100253e0];
const ORD_TENS: [u32; 9] = [0x100253d8, 0x100253d0, 0x100253c4, 0x100253b8, 0x100253a8, 0x1002539c, 0x10025390, 0x10025384, 0x10025378];
const ORD_UNITS: [u32; 9] = [0x10025370, 0x10025368, 0x10025360, 0x10025358, 0x10025350, 0x10025348, 0x10025340, 0x10025338, 0x10025330];
/// `" "` (0x100242b0), `":/-+"` (0x10025808), `".,:"` (0x10025810).
const SPACE: u32 = 0x100242b0;
const NOT_PLAIN: u32 = 0x10025808;
const TIME_SEPS: u32 = 0x10025810;
/// 0x10002a00: the symbols whose single unit is `un` (`$ € ¥ ~ _`), and
/// those excepted from `una` (`£ |`).
const UN_CURRENCIES: [u32; 5] = [0x10024388, 0x10024338, 0x10024310, 0x100242c8, 0x100242dc];
const NOT_UNA: [u32; 2] = [0x10024360, 0x100242f0];

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

impl Spanish {
    fn s(&self, va: u32) -> &[u8] {
        self.pe.cstr(va)
    }

    /// Slot 0 (0x10003470): the word numbered `id`, `""` when none.
    fn num_word(&self, id: u32) -> &[u8] {
        (0..WORD_COUNT).find(|&k| self.pe.u32(WORDS + 8 * k) == id).map(|k| self.s(self.pe.u32(WORDS + 8 * k + 4))).unwrap_or(b"")
    }

    /// 0x100060f0: the word of the number `n` (1–19 and the tens; `un`
    /// for 1), `""` otherwise.
    fn unit(&self, n: i32) -> &[u8] {
        if !(1..=0x5a).contains(&n) {
            return b"";
        }
        let target = self.pe.u32(UNIT_JUMPS + 4 * self.pe.u8(UNIT_INDEX + n as u32 - 1) as u32);
        if target == UNIT_NONE {
            return b"";
        }
        self.num_word(self.pe.u8(target + 1) as u32)
    }

    /// 0x100065b0: the reading of the two digits at `p` ("" for 00).
    fn two(&self, p: &[u8]) -> Vec<u8> {
        let c0 = p.first().copied().unwrap_or(0);
        let c1 = if c0 == 0 { 0 } else { p.get(1).copied().unwrap_or(0) };
        let n = elqnum::atol(&[c0, c1]);
        let mut out = Vec::new();
        if n == 0 {
            return out;
        }
        if c0 == b'1' || n < 10 {
            out.extend_from_slice(self.unit(n));
            return out;
        }
        out.extend_from_slice(self.unit((c0 as i32 - b'0' as i32) * 10));
        let u = self.unit(c1 as i32 - b'0' as i32);
        let (zero, one_eight, other) = if c0 == b'2' { (0x2e, 0x30, 0x2f) } else { (0x31, 0x33, 0x32) };
        if c1 == b'0' {
            out.extend_from_slice(self.num_word(zero));
            return out;
        }
        out.extend_from_slice(self.num_word(if c1 == b'1' || c1 == b'8' { one_eight } else { other }));
        out.extend_from_slice(u);
        out
    }

    /// 0x10006310: the hundreds digit at `p` read into `out` (the string
    /// at `at`); false for `0`.
    fn hundreds(&self, out: &mut Vec<u8>, at: usize, p: &[u8]) -> bool {
        let c = |i: usize| p.get(i).copied().unwrap_or(0);
        if c(0) == b'0' {
            return false;
        }
        let d = c(0) as i32 - b'0' as i32;
        match c(0) {
            b'1' if c(1) == b'0' && c(2) == b'0' => out.extend_from_slice(self.num_word(0x3a)),
            b'1' => out.extend_from_slice(self.num_word(0x3b)),
            b'2' => out.extend_from_slice(self.num_word(0x3e)),
            b'3' => out.extend_from_slice(self.num_word(0x3f)),
            b'5' => out.extend_from_slice(self.num_word(0x41)),
            b'7' | b'9' => {
                out.extend_from_slice(self.num_word(if c(0) == b'7' { 0x1e } else { 0x21 }));
                out.extend_from_slice(self.num_word(0x3c));
            }
            _ => {
                out.extend_from_slice(self.unit(d));
                out.extend_from_slice(self.num_word(0x3c));
            }
        }
        let space = c(0) != b'1' || (c(1) == b'0' && (c(2) == b'2' || c(2) == b'3')) || (c(1) == b'1' && c(2) == b'0');
        if space {
            let w = self.num_word(1).to_vec();
            sep(out, at, &w);
        }
        true
    }

    /// Slot 2 (0x10006e10): the cardinal reading of the digits `s`,
    /// appended to `out` (the string starts at the end of `out`). One digit
    /// alone reads as nothing (slot 4 sends it to slot 3).
    pub(super) fn cardinal(&self, out: &mut Vec<u8>, s: &[u8]) {
        let at = out.len();
        let s = cs(s);
        let len = s.len();
        let ch = |i: usize| s.get(i).copied().unwrap_or(0);
        let w = |id: u32| self.num_word(id).to_vec();
        let last_space = |out: &Vec<u8>| out.last() == Some(&b' ');
        let (mut a, mut b, mut c, mut d, mut e) = (false, false, false, false, true);
        let mut p = 0usize;
        let rest = |p: usize| s.get(p..).unwrap_or(&[]);
        if len >= 12 {
            self.hundreds(out, at, rest(p));
            p += 1;
        }
        if len >= 11 {
            d = true;
            out.extend(self.two(rest(p)));
            p += 1;
        }
        if len >= 10 {
            let al = ch(p);
            if al == b'1' && !d {
                out.extend(w(0x4d));
            } else {
                if al != b'1' && !d {
                    out.extend_from_slice(self.unit(al as i32 - b'0' as i32));
                }
                if !last_space(out) {
                    sep(out, at, &w(1));
                }
                out.extend(w(0x4e));
            }
            sep(out, at, &w(0));
            p += 1;
            d = false;
            e = true;
        }
        if len >= 9 {
            e = !self.hundreds(out, at, rest(p));
            p += 1;
        }
        if len >= 8 {
            d = true;
            if e && ch(p) == b'0' && ch(p + 1) == b'1' {
                a = true;
            } else {
                out.extend(self.two(rest(p)));
            }
            if ch(p) != b'0' {
                e = false;
            }
            p += 1;
        }
        if len >= 7 {
            let al = ch(p);
            if !(al == b'0' && e) {
                if a || (al == b'1' && !d) {
                    out.extend(w(0x4a));
                } else {
                    if al != b'1' && !d {
                        out.extend_from_slice(self.unit(al as i32 - b'0' as i32));
                    }
                    if !last_space(out) {
                        sep(out, at, &w(1));
                    }
                    out.extend(w(0x4b));
                }
                sep(out, at, &w(0));
            }
            p += 1;
            d = false;
            e = true;
        }
        if len >= 6 {
            c = true;
            e = !self.hundreds(out, at, rest(p));
            p += 1;
        }
        if len >= 5 {
            d = true;
            if e && ch(p) == b'0' && ch(p + 1) == b'1' {
                b = true;
            } else {
                out.extend(self.two(rest(p)));
            }
            if ch(p) != b'0' {
                e = false;
            }
            p += 1;
        }
        if len >= 4 {
            let al = ch(p);
            if al == b'0' && e {
                // nothing in the thousands
            } else if al == b'1' && !d && !c {
                // 0x1000724d: one thousand and what follows
                let s1 = ch(p + 1) != b'0';
                let a2 = ch(p + 2) != b'0';
                let c2 = ch(p + 3) != b'0';
                out.extend(w(0x47));
                let space = if s1 || (a2 && c2) { a2 || c2 || ch(p + 1) != b'1' } else { false };
                if space {
                    sep(out, at, &w(1));
                }
            } else {
                if !d {
                    out.extend_from_slice(self.unit(al as i32 - b'0' as i32));
                }
                out.extend(w(if b { 0x47 } else { 0x48 }));
                sep(out, at, &w(if !c || b { 1 } else { 0 }));
            }
            p += 1;
        }
        if len >= 3 {
            self.hundreds(out, at, rest(p));
            p += 1;
        }
        if len >= 2 {
            out.extend(self.two(rest(p)));
            if ch(p + 1) == b'1' && ch(p) != b'1' {
                out.extend_from_slice(self.s(MASC));
            }
        }
        let w0 = w(0);
        if out.len() >= w0.len() && out.ends_with(&w0) {
            out.truncate(out.len() - w0.len());
        }
    }

    /// Slot 3 (0x10006dd0): one digit's word.
    fn digit_word(&self, d: u8) -> &[u8] {
        match d {
            b'0' => self.num_word(0xf),
            b'1' => self.num_word(0x10),
            _ => self.unit(d as i32 - b'0' as i32),
        }
    }

    /// 0x10002910: the length of the area code `digits` starts with (the
    /// longest… the entry `ELQSearchString` mode 2 finds), 0 when none.
    fn area_code(&self, digits: &[u8]) -> usize {
        let key = copy63(digits);
        let entry = |k: u32| self.s(self.pe.u32(AREA_CODES + 4 * k));
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

    /// Slot 1 (0x10002980): a telephone number: the area code, then the
    /// rest (`ELQNumConvertPhoneNumber` steps 0 and 1).
    fn phone(&self, out: &mut Vec<u8>, digits: &[u8], pairs: bool) {
        let digits = cs(digits);
        let area = self.area_code(digits);
        let mut len = area;
        elqnum::convert_phone_number(&Slots(self), out, digits, pairs, &mut len, 0);
        elqnum::convert_phone_number(&Slots(self), out, digits.get(area..).unwrap_or(&[]), pairs, &mut len, 1);
    }

    /// Slot 5 (0x10005f10): the currency's name after `amount`.
    fn currency_name(&self, out: &mut Vec<u8>, amount: &[u8], currency: &[u8], cents: bool) {
        let Some(k) = (0..11u32).find(|&k| self.s(self.pe.u32(CURRENCIES + 0x14 * k)) == cs(currency)) else {
            return;
        };
        let at = out.len();
        let one = cs(amount) == b"1";
        let col = match (cents, one) {
            (false, true) => 1,
            (false, false) => 2,
            (true, true) => 3,
            (true, false) => 4,
        };
        let w1 = self.num_word(1).to_vec();
        sep(out, at, &w1);
        out.extend_from_slice(self.s(self.pe.u32(CURRENCIES + 0x14 * k + 4 * col)));
        sep(out, at, &w1);
    }

    /// Slot 6 (0x10007d00): decimals: leading zeros one by one, then up to
    /// three digits as a cardinal (unless `by_digits`), more digit by
    /// digit. Returns `ELQNumSetPauses`' pause flag.
    fn decimals(&self, out: &mut Vec<u8>, digits: &[u8], by_digits: bool) -> bool {
        let at = out.len();
        let digits = cs(digits);
        let (lead0, pause) = elqnum::set_pauses(digits);
        let w = self.num_word(if pause { 0 } else { 1 }).to_vec();
        sep(out, at, &w);
        let mut s = digits;
        while s.first() == Some(&b'0') {
            out.extend_from_slice(self.digit_word(b'0'));
            s = &s[1..];
            if s.first() == Some(&b'0') {
                sep(out, at, &w);
            }
        }
        let n = s.len() & 0xffff;
        if n == 0 {
            return pause;
        }
        if lead0 {
            sep(out, at, &w);
        }
        if n <= 3 && n != 1 && !by_digits {
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

    /// Slot 7 (0x100034a0): the parts of a date in the order `dmy`.
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

/// The module's slot table as LTTS7Util calls it back.
struct Slots<'a>(&'a Spanish);

impl NumSlots for Slots<'_> {
    fn word(&self, id: u32) -> &[u8] {
        self.0.num_word(id)
    }
    fn digits(&self, out: &mut Vec<u8>, digits: &[u8], flag: bool) {
        self.0.phone(out, digits, flag)
    }
    fn cardinal(&self, out: &mut Vec<u8>, digits: &[u8]) {
        self.0.cardinal(out, digits)
    }
    fn digit(&self, d: u8) -> &[u8] {
        self.0.digit_word(d)
    }
    /// Slot 4 (0x100029d0).
    fn by_digits(&self, flag: bool, len: usize, lead0: bool) -> bool {
        flag || len > 0xc || len == 1 || (len > 3 && lead0)
    }
    fn currency(&self, out: &mut Vec<u8>, amount: &[u8], currency: &[u8], cents: bool) {
        self.0.currency_name(out, amount, currency, cents)
    }
    fn decimals(&self, out: &mut Vec<u8>, digits: &[u8], flag: bool) -> bool {
        self.0.decimals(out, digits, flag)
    }
}

/// The engine's token buffer as the module sees it (0x10001990): a window
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
    /// 0x10001990: the token `delta` non-tag tokens from `i` (no further
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

impl Spanish {
    /// Slot 9 (0x10007490): the reading of the number token `i`.
    pub(super) fn leggi_numero(&self, toks: &mut [Tok], i: usize, kind: u32, dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
        let text = toks[i].text.clone();
        let base = i.saturating_sub(7);
        let mut cx = Ctx {
            win: Window { toks, base },
            i,
            dmy: dmy.to_vec(),
            used: 0,
        };
        let next1 = cx.win.fetch(i, 1);
        let next2 = cx.win.fetch(i, 2);
        let prev = cx.win.fetch(i, -1);
        let mut obj = NumObj::default();
        let mut mode = kind;
        if kind == 5 || kind == 6 {
            mode = 4;
            obj.set_u32(0x48, kind);
        }
        let n1 = cx.win.text(next1).to_vec();
        let is_word = |w: &[u8], a: u32| w == self.s(a);
        let currency = elqnum::is_currency(&n1)
            || (i > base && elqnum::is_currency(cx.win.text(prev)))
            || SLOT9_CURRENCY_WORDS.iter().any(|&a| is_word(&n1, a))
            || (is_word(&n1, LIBRA) && [ESTERLINA, ESTERLINAS].iter().any(|&a| is_word(cx.win.text(next2), a)));
        if currency {
            mode = 0xc;
        } else if n1.first() == Some(&b'%') {
            mode = 10;
        }
        obj.set_u32(0x44, mode);
        obj.set_cstr(4, &text);
        // 0x100067c0: the recognizers, the one the reading type names first
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
                4 => self.r_mode9(&mut obj),
                5 => self.r_fraction(&mut obj),
                6 => self.r_phone(&mut obj),
                7 => self.r_real(&mut obj),
                _ => self.r_code(&mut obj),
            };
            if ok {
                found = true;
                break;
            }
        }
        if !found {
            elqnum::set_default_type(&mut obj, &text);
        }
        Some((self.render(&obj), cx.used))
    }

    /// The common start of a recognizer: the text without its sign (the
    /// sign's flags at `minus`/`plus` when given), and the original.
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

    /// Recognizer 0 (0x10003d90): a date (kind 8).
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

    /// Recognizer 1 (0x10003ef0): a time (kind 7); an hour followed by `h`
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
        if h <= 0x18 && h > 0 && cx.win.text(n1) == self.s(HOUR_MARK) {
            let t2 = cx.win.text(n2).to_vec();
            let t3 = cx.win.text(n3);
            let mut t = text.clone();
            t.push(b':');
            if elqnum::is_number(&t2, b"", true) && elqnum::atol(&t2) <= 0x3b && t3 != self.s(HOUR_MARK) {
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
                if !(mode == 0 || (7..=8).contains(&mode)) {
                    return Self::fail(obj, &buf);
                }
                if obj.u8(0x8d) != 0 {
                    return Self::fail(obj, &buf);
                }
                if obj.u8(0x8e) == 0 && elqnum::is_time(&text) {
                    return true;
                }
                if signed || !elqnum::is_time(&hours) {
                    return Self::fail(obj, &buf);
                }
                obj.set_cstr(4, &hours);
                obj.set_cstr(0x4c, &minutes);
                obj.set_u8(0x8c, 1);
                return true;
            }
            return Self::fail(obj, &buf);
        }
        if commas == 0 && dots < 3 && mode == 0 && !signed && elqnum::is_time(&text) {
            return true;
        }
        if commas == 0 && dots < 3 && mode == 7 && !signed && elqnum::is_time(&text) {
            return true;
        }
        Self::fail(obj, &buf)
    }

    /// Recognizer 2 (0x10004340): an ordinal (kind 4): a reading type 4,
    /// or `°`/`º` (not before `c`) or `'` after the number.
    fn r_ordinal(&self, cx: &mut Ctx, obj: &mut NumObj) -> bool {
        let i = cx.i;
        let n1 = cx.win.fetch(i, 1);
        let n2 = cx.win.fetch(i, 2);
        obj.set_u32(0, 4);
        let buf = Self::strip_sign(obj, None);
        let (dots, commas) = dots_commas(obj.text());
        if dots + commas != 0 {
            return Self::fail(obj, &buf);
        }
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

    /// Recognizer 3 (0x10004510): an amount (kind 0xc) with a currency
    /// symbol before or after it, or a currency word after it (that token
    /// becomes `n` and is used up).
    fn r_currency(&self, cx: &mut Ctx, obj: &mut NumObj) -> bool {
        let i = cx.i;
        let n1 = cx.win.fetch(i, 1);
        let n2 = cx.win.fetch(i, 2);
        let prev = cx.win.fetch(i, -1);
        obj.set_u32(0, 0xc);
        obj.set_u32(0x48, 0xf);
        for o in [0x56, 0x57, 0x58, 0x5a] {
            obj.set_u8(o, 0);
        }
        obj.set_u8(0x4c, 0);
        let buf = Self::strip_sign(obj, Some(0x56));
        let text = obj.text().to_vec();
        if !text.is_empty() {
            let (dots, commas) = dots_commas(&text);
            if dots == 1 {
                if commas > 0 {
                    obj.set_u8(0x5a, 1);
                } else {
                    let k = text.iter().position(|&c| c == b'.').unwrap();
                    if text.len() - k - 1 != 3 {
                        obj.set_u8(0x5a, 1);
                    }
                }
            }
        }
        if i > cx.win.base && elqnum::is_currency(cx.win.text(prev)) {
            let p = cx.win.text(prev).to_vec();
            obj.set_cstr(0x4c, &p);
            return true;
        }
        let t1 = cx.win.text(n1).to_vec();
        let take = |cx: &mut Ctx, obj: &mut NumObj, sym: &[u8], tok: usize| {
            obj.set_cstr(0x4c, sym);
            cx.win.toks[tok].text = b"n".to_vec();
            cx.used = tok - i;
            true
        };
        if elqnum::is_currency(&t1) {
            return take(cx, obj, &t1, n1);
        }
        for &(a, sym) in &R3_WORDS {
            if t1 == self.s(a) {
                return take(cx, obj, &[sym], n1);
            }
        }
        if t1 == self.s(LIBRA) {
            let t2 = cx.win.text(n2).to_vec();
            if t2 == self.s(ESTERLINA) || t2 == self.s(ESTERLINAS) {
                return take(cx, obj, &[0xa3], n2);
            }
        }
        if R3_POUND.iter().any(|&a| t1 == self.s(a)) {
            return take(cx, obj, &[0xa3], n1);
        }
        Self::fail(obj, &buf)
    }

    /// Recognizer 4 (0x10004ea0): numbers of reading type 9 (kind 9).
    fn r_mode9(&self, obj: &mut NumObj) -> bool {
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
                if obj.cstr(0x4c).is_empty() {
                    return Self::fail(obj, &buf);
                }
                obj.set_u8(0x8c, 0);
                return true;
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
            if p >= q {
                return Self::fail(obj, &buf);
            }
            let int = copy63(&text[..q]);
            if !elqnum::is_big_integer(&int) || mode != 9 {
                return Self::fail(obj, &buf);
            }
            obj.set_cstr(0x4c, &int);
            obj.set_u32(0x48, 3);
            obj.set_cstr(0x8c, &text[q + 1..]);
            return true;
        }
        Self::fail(obj, &buf)
    }

    /// Recognizer 5 (0x10005140): a fraction of the list (kind 0xb).
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

    /// 0x10001a80: how the fraction `s` is read.
    fn fraction(&self, s: &[u8]) -> Option<&[u8]> {
        FRACTIONS.iter().find(|&&(k, _)| s == self.s(k)).map(|&(_, v)| self.s(v))
    }

    /// Recognizer 6 (0x10005260): a telephone number (kind 1): more than six
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
                1 => {
                    obj.set_u8(0x4c, 1);
                    if self.area_code(&text) != 0 {
                        return true;
                    }
                }
                0 | 8 => {
                    if strpbrk(&text, self.s(NOT_PLAIN)).is_none() && text.len() > 6 && self.area_code(&text) != 0 {
                        obj.set_u8(0x4c, 0);
                        return true;
                    }
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

    /// Recognizer 7 (0x10005640): integers and decimals (kind 10, or 3 for
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
            if p >= q {
                return Self::fail(obj, &buf);
            }
            let int = copy63(&text[..q]);
            if !elqnum::is_big_integer(&int) {
                return Self::fail(obj, &buf);
            }
            let dec = &text[q + 1..];
            if (mode == 0 && dec.len() <= 2) || mode == 3 {
                obj.set_cstr(0x4c, &int);
                obj.set_cstr(0x8c, dec);
                obj.set_u32(0x48, 3);
                return true;
            }
        }
        Self::fail(obj, &buf)
    }

    /// Recognizer 8 (0x10005480): digits of reading type 2 (kind 2).
    fn r_code(&self, obj: &mut NumObj) -> bool {
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
            // 0x1000558f: split, then rejected all the same
            let k = text.iter().position(|&c| c == b',').unwrap();
            obj.set_u8(0x8c, 1);
            obj.set_cstr(4, &text[..k]);
            obj.set_cstr(0x4c, &copy63(&text[k + 1..]));
        }
        Self::fail(obj, &buf)
    }

    /// The renderer (0x100068a0): the reading of `obj` by its kind.
    fn render(&self, obj: &NumObj) -> Vec<u8> {
        let s = Slots(self);
        let mut out = Vec::new();
        let w = |id: u32| self.num_word(id).to_vec();
        let sign = |out: &mut Vec<u8>, minus: usize| {
            if obj.u8(minus) == 1 {
                sep(out, 0, &w(7));
            } else if obj.u8(minus + 1) == 1 {
                sep(out, 0, &w(8));
            }
        };
        match obj.kind() {
            0 => elqnum::convert_real_number(&mut out, 0, obj, &s),
            1 => {
                self.phone(&mut out, obj.text(), obj.u8(0x4c) != 0);
                let rest = obj.cstr(0x4d).to_vec();
                if !rest.is_empty() {
                    let mut o2 = NumObj::default();
                    o2.set_cstr(4, &rest);
                    o2.set_u32(0x44, obj.mode());
                    let end = out.len();
                    elqnum::convert_real_number(&mut out, end, &o2, &s);
                }
            }
            2 => {
                sign(&mut out, 0x8d);
                let pause = elqnum::convert_integer(&mut out, 0, obj.text(), true, &s);
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
                    elqnum::convert_integer(&mut out, end, obj.cstr(0x4c), false, &s)
                } else {
                    elqnum::convert_big_integer(&mut out, end, obj.cstr(0x4c), &s);
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
                elqnum::convert_integer(&mut out, 0, obj.cstr(0x4c), false, &s);
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
                self.amount(&mut out, &text, obj.cstr(0x4c));
            }
            _ => {}
        }
        out
    }

    /// 0x10002090: an ordinal of up to four digits (more as a cardinal),
    /// masculine or feminine.
    fn ordinal(&self, out: &mut Vec<u8>, text: &[u8], masc: bool) {
        out.clear();
        let t = cs(text);
        if t.len() > 4 {
            elqnum::convert_integer(out, 0, t, false, &Slots(self));
            return;
        }
        let end = self.s(if masc { MASC } else { FEM }).to_vec();
        let space = self.s(SPACE).to_vec();
        let mut p = 0;
        // each place's stem, then `o`/`a`; a space after every place but
        // the units
        for (place, table) in [(4, &ORD_THOUSANDS), (3, &ORD_HUNDREDS), (2, &ORD_TENS), (1, &ORD_UNITS)] {
            if t.len() - p != place {
                continue;
            }
            let d = t[p];
            if d != b'0' {
                if (b'1'..=b'9').contains(&d) {
                    out.extend_from_slice(self.s(table[(d - b'1') as usize]));
                }
                out.extend_from_slice(&end);
            }
            if place > 1 {
                out.extend_from_slice(&space);
            }
            p += 1;
        }
    }

    /// 0x10005a00: a time `h:mm[:ss]` (also `.` or `,`).
    fn time(&self, out: &mut Vec<u8>, text: &[u8]) {
        let s = Slots(self);
        let w = |id: u32| self.num_word(id).to_vec();
        let seps = self.s(TIME_SEPS).to_vec();
        let buf = copy63(text);
        let Some(p) = strpbrk(&buf, &seps) else {
            return;
        };
        let hours = &buf[..p];
        let mut h = hours;
        let mut one = false;
        if h.first() == Some(&b'0') && h.get(1).is_some_and(|&c| c > b'0' && c <= b'9') {
            one = h[1] == b'1';
            h = &h[1..];
        }
        elqnum::convert_integer(out, 0, h, false, &s);
        sep(out, 0, &w(1));
        sep(out, 0, &w(if one { 0x62 } else { 0x5f }));
        let after = &buf[p + 1..];
        // what comes after a leading zero is read alone ("05" → cinco)
        let unpad = |x: &[u8]| -> (Vec<u8>, bool) {
            if x.first() == Some(&b'0') && x.get(1).is_some_and(|&c| c > b'0' && c <= b'9') {
                (x[1..].to_vec(), x[1] == b'1')
            } else {
                (x.to_vec(), false)
            }
        };
        let Some(q) = strpbrk(after, &seps) else {
            // no seconds
            let m = after;
            if m.is_empty() || (m[0] == b'0' && m.get(1) == Some(&b'0')) {
                return;
            }
            sep(out, 0, &w(0));
            let mut m = m;
            if m.len() > 1 && m.get(2) == Some(&b'-') {
                let end = out.len();
                elqnum::convert_integer(out, end, &m[..2], false, &s);
                sep(out, 0, &w(0));
                m = &m[3..];
            }
            let end = out.len();
            elqnum::convert_integer(out, end, m, false, &s);
            return;
        };
        let minutes = &after[..q];
        let seconds = &after[q + 1..];
        let zero2 = |x: &[u8]| x.first() == Some(&b'0') && x.get(1) == Some(&b'0');
        let m_empty = minutes.is_empty() || zero2(minutes);
        let s_empty = seconds.is_empty() || zero2(seconds);
        if !m_empty && !s_empty {
            sep(out, 0, &w(0));
            let (m, one) = unpad(minutes);
            let end = out.len();
            elqnum::convert_integer(out, end, &m, false, &s);
            sep(out, 0, &w(1));
            sep(out, 0, &w(if one { 0x61 } else { 0x5e }));
            sep(out, 0, &w(0));
            let (sec, one) = unpad(seconds);
            let end = out.len();
            elqnum::convert_integer(out, end, &sec, false, &s);
            sep(out, 0, &w(1));
            sep(out, 0, &w(if one { 0x60 } else { 0x5d }));
            return;
        }
        // 0x10005c41: minutes 00 and seconds: "y" seconds; else 0x10005d75:
        // the minutes, when the seconds are 00
        if zero2(minutes) {
            if zero2(seconds) {
                return;
            }
            sep(out, 0, &w(0));
            sep(out, 0, &w(0xd));
            let (sec, one) = unpad(seconds);
            sep(out, 0, &w(1));
            let end = out.len();
            elqnum::convert_integer(out, end, &sec, false, &s);
            sep(out, 0, &w(1));
            sep(out, 0, &w(if one { 0x60 } else { 0x5d }));
            return;
        }
        if !zero2(seconds) {
            return;
        }
        sep(out, 0, &w(0));
        let (m, one) = unpad(minutes);
        let end = out.len();
        elqnum::convert_integer(out, end, &m, false, &s);
        sep(out, 0, &w(1));
        sep(out, 0, &w(if one { 0x61 } else { 0x5e }));
    }

    /// 0x10003510: a date: day, ` de ` month, ` de ` year.
    fn date(&self, out: &mut Vec<u8>, day: u32, month: u32, year: u32) {
        let s = Slots(self);
        let w = |id: u32| self.num_word(id).to_vec();
        if day > 0 {
            elqnum::convert_integer(out, 0, day.to_string().as_bytes(), false, &s);
            sep(out, 0, &w(1));
            if month > 0 || year > 0 {
                sep(out, 0, &w(2));
            }
        }
        if (1..=12).contains(&month) {
            out.extend(w(0x50 + month));
            sep(out, 0, &w(1));
        }
        if year > 0 {
            sep(out, 0, &w(2));
            let end = out.len();
            elqnum::convert_integer(out, end, year.to_string().as_bytes(), false, &s);
        }
    }

    /// 0x10002a00: an amount of the currency symbol `cur`.
    fn amount(&self, out: &mut Vec<u8>, text: &[u8], cur: &[u8]) {
        let s = Slots(self);
        let w = |id: u32| self.num_word(id).to_vec();
        let is = |a: u32| cur == self.s(a);
        let text = cs(text);
        let units_of = |out: &mut Vec<u8>, units: &[u8]| {
            let end = out.len();
            if elqnum::is_big_integer(units) {
                elqnum::convert_big_integer(out, end, units, &s);
            } else {
                self.cardinal(out, units);
            }
        };
        // the units, `un`/`una` for 1 (0x10002ac7 / 0x100031ea)
        let units_read = |out: &mut Vec<u8>, units: &[u8]| {
            if units == b"1" && UN_CURRENCIES.iter().any(|&a| is(a)) {
                out.extend(w(0x11));
            } else if units == b"1" && !NOT_UNA.iter().any(|&a| is(a)) {
                out.extend(w(0x12));
            } else {
                units_of(out, units);
            }
        };
        let Some(k) = strpbrk(text, b",") else {
            units_read(out, text);
            self.currency_name(out, text, cur, false);
            return;
        };
        let units = &text[..k];
        let dec = &text[k + 1..];
        if dec.len() == 2 {
            if units == b"0" {
                if dec[0] == b'0' {
                    if dec[1] == b'1' {
                        out.extend(w(0x11));
                    } else {
                        out.extend_from_slice(self.digit_word(dec[1]));
                    }
                    self.currency_name(out, &dec[1..], cur, true);
                } else {
                    self.cardinal(out, dec);
                    self.currency_name(out, dec, cur, true);
                }
                out.extend(w(1));
                out.extend(w(2));
                out.extend(w(1));
                self.currency_name(out, b"1", cur, false);
                return;
            }
            units_read(out, units);
            self.currency_name(out, units, cur, false);
            if dec == b"00" {
                return;
            }
            out.extend(w(1));
            out.extend(w(0xd));
            out.extend(w(1));
            if dec[0] == b'0' {
                if dec[1] == b'1' {
                    out.extend(w(0x11));
                } else {
                    out.extend_from_slice(self.digit_word(dec[1]));
                }
                self.currency_name(out, &dec[1..], cur, true);
            } else {
                self.cardinal(out, dec);
                self.currency_name(out, dec, cur, true);
            }
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
/// (`LOQ_TRACE_LES=1 LOQ_NUM_ONLY=1 LOQ_LES_DLL=LTTS7Spanish.dll
/// LOQ_LES_TABLE=<variant table> LOQ_LES_N=26`): every `> NUM` call with
/// its `T` token buffer, compared with its `< NUM` reading, tokens used
/// (`c18`) and rewritten tokens (`U`).
///   QQ_ES_NUM_TRACE=<a.les>[:<b.les>…] [QQ_ES_MT=SpanishAm] [QQ_ES_LTTS7=<dir>]
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
    #[ignore]
    fn spanish_slot9_trace() {
        let Ok(paths) = std::env::var("QQ_ES_NUM_TRACE") else { return };
        let root = std::env::var("QQ_ES_LTTS7").unwrap_or_else(|_| "/tmp/qqv/LTTS7".into());
        let mt = std::env::var("QQ_ES_MT").unwrap_or_else(|_| "SpanishAm".into());
        let files = crate::lang::ModuleFiles {
            dll: std::fs::read(format!("{root}/bin/LTTS7Spanish.dll")).unwrap(),
            mother_tongue: mt,
            ..Default::default()
        };
        let es = Spanish::from_files(&files).expect("LTTS7Spanish.dll");
        let (mut calls, mut bad) = (0, 0);
        for path in paths.split(':') {
            let text = std::fs::read_to_string(path).unwrap();
            let lines: Vec<&str> = text.lines().collect();
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
                let ret = &lines[k];
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
                let (got, used) = es.leggi_numero(&mut toks, idx, kind, &dmy).unwrap();
                // the engine then leaves the last word in the token; the
                // `U` dump is taken before that, so compare the others
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
        }
        eprintln!("{calls} slot-9 calls, {bad} differ");
        assert_eq!(bad, 0);
    }
}
