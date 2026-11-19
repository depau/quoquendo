//! The English module's number readers: `LesIniNumeri` (0x1000beb0) and
//! the slot 9 reader the engine calls for a number token (0x1000b540), with
//! everything it reaches (MECHANICS.md §4c, "Numbers"). Util's `ELQNum*`
//! are `lang/elqnum.rs`; the module's slots 0–6 are [`NumSlots`] here.
//!
//! Strings are the engine's C strings without their NUL; a function that
//! writes through a `char *` into a caller's buffer takes the whole buffer
//! and the offset of that pointer (`at`). The number object travels as a
//! [`NumObj`] (0xd4 bytes, the module's fields at their offsets).

use super::{English, Variant};
use crate::lang::common::{tok, Pe, Tok};
use crate::lang::elqnum::{self as q, NumObj, NumSlots};

/// Slot 0's word table (0x10004c50): 0x5b `{ id, char * }` pairs.
const WORDS: u32 = 0x1002a088;
const WORD_COUNT: u32 = 0x5b;
/// 0x10008430 (a number 1–90's word): an index byte per number − 1
/// (0x100085e8, 0x5a of them) into the jump table 0x10008578, whose cases
/// are `push id; call slot 0`; the default case 0x10008571 gives "".
const NUM_INDEX: u32 = 0x100085e8;
const NUM_JUMPS: u32 = 0x10008578;
const NUM_NONE: u32 = 0x10008571;
/// The month names `{ char *, month }` 0x100045e0 searches (0x24 pairs).
const MONTHS: u32 = 0x10029e38;
/// The era words after a year (`ad`, `bc`, 2 pointers) and the words that
/// make a lone 4-digit number after them a year (10 pointers).
const ERAS: u32 = 0x10029f80;
const YEAR_WORDS: u32 = 0x10029f58;
/// The currency names slot 5 uses: 12 rows `{ code, unit, units, cent,
/// cents }` (0x10029f98).
const CURRENCIES: u32 = 0x10029f98;
/// Scale words after an amount (`$3 million`): 12 `{ word, replacement }`
/// (0x10029dd8).
const SCALES: u32 = 0x10029dd8;
/// The fraction denominators 0x100033f0 accepts (2 … 1000), and the
/// plural word 0x10002f10 gives each (same order).
const DENOMINATORS: [u32; 14] = [
    0x1002a64c, 0x1002a640, 0x1002a634, 0x1002a628, 0x1002a61c, 0x1002a610, 0x1002a600, 0x1002a5f4, 0x1002a5e8, 0x1002a5dc, 0x1002a5cc,
    0x1002a5bc, 0x1002a5a8, 0x1002a590,
];
const DENOMINATOR_WORDS: [u32; 14] = [
    0x1002a644, 0x1002a638, 0x1002a62c, 0x1002a620, 0x1002a614, 0x1002a604, 0x1002a5f8, 0x1002a5ec, 0x1002a5e0, 0x1002a5d0, 0x1002a5c0,
    0x1002a5ac, 0x1002a598, 0x1002a580,
];
/// 0x10003720's ordinal endings: `{ key, ending }` as its compare chain
/// has them (`1` first, at 0x10003809).
const ORDINALS: [(u32, u32); 28] = [
    (0x1002a57c, 0x1002a7a8),
    (0x1002a64c, 0x10029848),
    (0x1002a640, 0x1002a7a0),
    (0x1002a634, 0x1002a798),
    (0x1002a628, 0x1002a790),
    (0x1002a61c, 0x1002a788),
    (0x1002a610, 0x1002a780),
    (0x1002a600, 0x1002a778),
    (0x1002a5f4, 0x1002a770),
    (0x1002a5e8, 0x1002a768),
    (0x1002a764, 0x1002a758),
    (0x1002a754, 0x1002a74c),
    (0x1002a748, 0x1002a73c),
    (0x1002a738, 0x1002a72c),
    (0x1002a728, 0x1002a71c),
    (0x1002a5dc, 0x1002a710),
    (0x1002a70c, 0x1002a700),
    (0x1002a6fc, 0x1002a6f0),
    (0x1002a6ec, 0x1002a6e0),
    (0x1002a5cc, 0x1002a6d4),
    (0x1002a6d0, 0x1002a6c4),
    (0x1002a6c0, 0x1002a6b4),
    (0x1002a6b0, 0x1002a6a4),
    (0x1002a6a0, 0x1002a694),
    (0x1002a690, 0x1002a684),
    (0x1002a680, 0x1002a674),
    (0x1002a670, 0x1002a664),
    (0x1002a660, 0x1002a654),
];
/// The country-code prefixes 0x10004bb0 looks for (`ELQSearchString`
/// flags 0x12: an entry that starts the key, 3 entries).
const PHONE_PREFIXES: u32 = 0x10029f88;

/// The engine's settings the module reads through ctx+0x20: +0x34 the
/// date format (0x10 dmy, 0x11 mdy … 0x19 y; the US voice has 0x11, GB and
/// AU 0x10, seen in the traces), +0x30 the time format (7 by default,
/// 0x1a after `\{number=time_duration}`, LoqTTS7 0x10030670).
fn date_format(v: Variant) -> u32 {
    match v {
        Variant::Us => 0x11,
        Variant::Gb | Variant::Au => 0x10,
    }
}
const TIME_FORMAT: u32 = 7;

// --- C string helpers ---------------------------------------------------------

/// The C string at the start of `s`.
fn cs(s: &[u8]) -> &[u8] {
    &s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())]
}
fn strpbrk(s: &[u8], set: &[u8]) -> Option<usize> {
    s.iter().position(|c| set.contains(c))
}
fn strrchr(s: &[u8], c: u8) -> Option<usize> {
    s.iter().rposition(|&x| x == c)
}
/// `ELQstricmp`: ASCII case folding.
fn ieq(a: &[u8], b: &[u8]) -> bool {
    a.eq_ignore_ascii_case(b)
}
/// 0x10002ec0: `.` and `,` swapped.
fn swap_dot_comma(s: &mut [u8]) {
    for c in s.iter_mut() {
        *c = match *c {
            b'.' => b',',
            b',' => b'.',
            x => x,
        };
    }
}
/// The dots and commas of `s`.
fn count_marks(s: &[u8]) -> (u32, u32) {
    (s.iter().filter(|&&c| c == b'.').count() as u32, s.iter().filter(|&&c| c == b',').count() as u32)
}
/// 0x10004570: four digits.
fn four_digits(s: &[u8]) -> bool {
    s.len() == 4 && s.iter().all(u8::is_ascii_digit)
}
/// 0x100045a0: at least four of digits and `-./`.
fn date_like(s: &[u8]) -> bool {
    s.len() >= 4 && s.iter().all(|&c| c.is_ascii_digit() || b"-./".contains(&c))
}
fn digits_only(s: &[u8]) -> bool {
    q::is_number(s, b"", true)
}
/// `ELQltoa`
fn ltoa(n: u32) -> Vec<u8> {
    n.to_string().into_bytes()
}
/// The text after a sign the recognizers strip (`-`/`+`), and which.
fn unsigned(t: &[u8]) -> (&[u8], u8) {
    match t.first() {
        Some(b'-') => (&t[1..], b'-'),
        Some(b'+') => (&t[1..], b'+'),
        _ => (t, 0),
    }
}

// --- the module -------------------------------------------------------------

/// The module's data: its DLL and variant.
pub(crate) struct Numeri<'a> {
    pe: &'a Pe,
    variant: Variant,
}

impl<'a> Numeri<'a> {
    pub(crate) fn new(pe: &'a Pe, variant: Variant) -> Self {
        Numeri { pe, variant }
    }

    fn s(&self, va: u32) -> &'a [u8] {
        self.pe.cstr(va)
    }

    /// Slot 0 (0x10004c50)
    fn w(&self, id: u32) -> &'a [u8] {
        (0..WORD_COUNT).find(|&k| self.pe.u32(WORDS + 8 * k) == id).map(|k| self.s(self.pe.u32(WORDS + 8 * k + 4))).unwrap_or(b"")
    }

    /// 0x10008430: the word of 1–20 and the tens.
    fn num_word(&self, n: i32) -> &'a [u8] {
        let k = n.wrapping_sub(1) as u32;
        if k > 0x59 {
            return b"";
        }
        let target = self.pe.u32(NUM_JUMPS + 4 * self.pe.u8(NUM_INDEX + k) as u32);
        if target == NUM_NONE {
            return b"";
        }
        // the case is `push id` (6a id)
        self.w(self.pe.u8(target + 1) as u32)
    }

    /// 0x100045e0: the month of a name, 0 for none.
    fn month(&self, s: &[u8]) -> u32 {
        (0..0x24).find(|&k| ieq(s, self.s(self.pe.u32(MONTHS + 8 * k)))).map(|k| self.pe.u32(MONTHS + 8 * k + 4)).unwrap_or(0)
    }

    /// 0x100033f0: a fraction `n/d` with one of the denominators.
    fn fraction(&self, s: &[u8]) -> bool {
        if strpbrk(s, b".,").is_some() {
            return false;
        }
        let Some(p) = strpbrk(s, b"/") else {
            return false;
        };
        let d = &s[p + 1..];
        strpbrk(d, b"/").is_none() && DENOMINATORS.iter().any(|&a| d == self.s(a))
    }

    /// 0x100088d0: two digits read (`s` its first two characters).
    fn two(&self, s: &[u8]) -> Vec<u8> {
        let s2 = &s[..s.len().min(2)];
        let v = q::atol(s2);
        let mut b = Vec::new();
        if v == 0 {
            return b;
        }
        if s2[0] == b'1' || v < 10 {
            b.extend_from_slice(self.num_word(v));
        } else {
            b.extend_from_slice(self.num_word((s2[0] as i32 - b'0' as i32) * 10));
            if s2.get(1).is_some_and(|&c| c != b'0') {
                b.extend_from_slice(self.s(0x10029ab8));
                b.extend_from_slice(self.num_word(s2[1] as i32 - b'0' as i32));
            }
        }
        b
    }

    /// 0x10008650: the hundreds digit `s[0]` read into `out` (the cardinal's
    /// string at `at`); false for 0.
    fn hundreds(&self, out: &mut Vec<u8>, at: usize, s: &[u8]) -> bool {
        let c = s.first().copied().unwrap_or(0);
        if c == b'0' {
            return false;
        }
        let at1 = |k: usize| s.get(k).copied().unwrap_or(0);
        match c {
            b'1' => {
                let id = if at1(1) == b'0' && at1(2) == b'0' { 0x3a } else { 0x3b };
                out.extend_from_slice(self.w(id));
            }
            b'2' => out.extend_from_slice(self.w(0x3e)),
            b'3' => out.extend_from_slice(self.w(0x3f)),
            b'5' => out.extend_from_slice(self.w(0x41)),
            b'7' => {
                out.extend_from_slice(self.w(0x1e));
                out.extend_from_slice(self.w(0x3c));
            }
            b'9' => {
                out.extend_from_slice(self.w(0x21));
                out.extend_from_slice(self.w(0x3c));
            }
            _ => {
                out.extend_from_slice(self.num_word(c as i32 - b'0' as i32));
                out.extend_from_slice(self.w(0x3c));
            }
        }
        if c != b'1' {
            q::add_separator(out, at, self.w(1));
        }
        true
    }

    /// Slot 2 (0x100094a0): the cardinal of up to 15 digits.
    fn cardinal_at(&self, out: &mut Vec<u8>, digits: &[u8]) {
        let at = out.len();
        let d = cs(digits);
        let len = d.len();
        let g = |p: usize| d.get(p).copied().unwrap_or(0);
        let mut p = 0usize;
        let (mut f11, mut f12, mut f13) = (false, false, true);
        let (mut f19, mut f1a, mut f1b) = (false, false, false);
        let and = |out: &mut Vec<u8>, f12: bool, p: usize| {
            if f12 && !(g(p) == b'0' && g(p + 1) == b'0') {
                out.extend_from_slice(self.s(0x1002a8d4));
            }
        };
        let rest = |p: usize| &d[p.min(len)..];
        // trillions
        if len >= 15 {
            f12 = self.hundreds(out, at, rest(p));
            p += 1;
        }
        if len >= 14 {
            f11 = true;
            and(out, f12, p);
            out.extend_from_slice(&self.two(rest(p)));
            p += 1;
            f12 = false;
        }
        if len >= 13 {
            self.scale(out, at, g(p), f11, 0x4f, 0x50);
            p += 1;
            f11 = false;
            f13 = true;
        }
        // billions
        if len >= 12 {
            f12 = self.hundreds(out, at, rest(p));
            p += 1;
        }
        if len >= 11 {
            f11 = true;
            and(out, f12, p);
            out.extend_from_slice(&self.two(rest(p)));
            p += 1;
            f12 = false;
        }
        if len >= 10 {
            self.scale(out, at, g(p), f11, 0x4d, 0x4e);
            p += 1;
            f11 = false;
            f13 = true;
        }
        // millions
        if len >= 9 {
            f12 = self.hundreds(out, at, rest(p));
            f13 = !f12;
            p += 1;
        }
        if len >= 8 {
            f11 = true;
            if f13 && g(p) == b'0' && g(p + 1) == b'1' {
                f1a = true;
            } else {
                and(out, f12, p);
                out.extend_from_slice(&self.two(rest(p)));
            }
            if g(p) != b'0' {
                f13 = false;
            }
            f12 = false;
            p += 1;
        }
        if len >= 7 {
            let c = g(p);
            if !(c == b'0' && f13) {
                if f1a || (c == b'1' && !f11) {
                    out.extend_from_slice(self.w(0x4a));
                } else {
                    if !f11 {
                        out.extend_from_slice(self.num_word(c as i32 - b'0' as i32));
                    }
                    if out.last() != Some(&b' ') {
                        q::add_separator(out, at, self.w(1));
                    }
                    out.extend_from_slice(self.w(0x4b));
                }
                q::add_separator(out, at, self.w(0));
            }
            p += 1;
            f11 = false;
            f13 = true;
        }
        // thousands
        if len >= 6 {
            f19 = true;
            f12 = self.hundreds(out, at, rest(p));
            f13 = !f12;
            p += 1;
        }
        if len >= 5 {
            f11 = true;
            if f13 && g(p) == b'0' && g(p + 1) == b'1' {
                f1b = true;
            } else {
                and(out, f12, p);
                out.extend_from_slice(&self.two(rest(p)));
            }
            if g(p) != b'0' {
                f13 = false;
            }
            f12 = false;
            p += 1;
        }
        if len >= 4 {
            let c = g(p);
            if !(c == b'0' && f13) {
                if c == b'1' && !f11 && !f19 {
                    let bl = g(p + 1) != b'0';
                    f1a = g(p + 2) != b'0';
                    f19 = g(p + 3) != b'0';
                    out.extend_from_slice(self.w(0x47));
                    if (bl || (f1a && f19)) && !(!f1a && !f19 && g(p + 1) == b'1') {
                        q::add_separator(out, at, self.w(1));
                    }
                } else {
                    if !f11 {
                        out.extend_from_slice(self.num_word(c as i32 - b'0' as i32));
                    }
                    out.extend_from_slice(self.w(if f1b { 0x47 } else { 0x48 }));
                    let sep = if !f19 || f1b { 1 } else { 0 };
                    q::add_separator(out, at, self.w(sep));
                }
            }
            p += 1;
        }
        // hundreds
        if len >= 3 {
            f12 = self.hundreds(out, at, rest(p));
            p += 1;
        }
        if len >= 2 {
            and(out, f12, p);
            out.extend_from_slice(&self.two(rest(p)));
        }
        let _ = (f11, f13, f1a, f19);
        // a trailing pause separator goes
        let w0 = self.w(0);
        if out.len() - at >= w0.len() && out.ends_with(w0) {
            out.truncate(out.len() - w0.len());
        }
    }

    /// The units digit `c` of a scale (trillion 0x4f/0x50, billion
    /// 0x4d/0x4e) and the scale word, then the pause separator.
    fn scale(&self, out: &mut Vec<u8>, at: usize, c: u8, f11: bool, one: u32, many: u32) {
        if c == b'1' && !f11 {
            out.extend_from_slice(self.w(one));
        } else {
            if !f11 {
                out.extend_from_slice(self.num_word(c as i32 - b'0' as i32));
            }
            if out.last() != Some(&b' ') {
                q::add_separator(out, at, self.w(1));
            }
            out.extend_from_slice(self.w(many));
        }
        q::add_separator(out, at, self.w(0));
    }

    /// Slot 3 (0x10009450)
    fn digit_word(&self, d: u8) -> &'a [u8] {
        match d {
            b'0' => self.w(0xf),
            b'1' => self.w(0x10),
            b'2'..=b'9' => self.num_word((d - b'0') as i32),
            _ => b"",
        }
    }

    /// Slot 8 (0x10002ee0): `oh` for a zero when `flag`.
    fn oh(&self, out: &mut Vec<u8>, flag: bool) {
        if flag {
            out.extend_from_slice(self.s(0x1002a578));
        }
    }

    /// Slot 1 (0x1000b300): digit by digit, `oh` for 0, slot 0's word 0
    /// after each.
    fn digits_at(&self, out: &mut Vec<u8>, digits: &[u8]) {
        let at = out.len();
        for &c in cs(digits) {
            if c == b'0' {
                self.oh(out, true);
            } else {
                out.extend_from_slice(self.digit_word(c));
            }
            q::add_separator(out, at, self.w(0));
        }
    }

    /// Slot 6 (0x1000b3b0): decimals digit by digit, the separator first.
    fn decimals_at(&self, out: &mut Vec<u8>, digits: &[u8]) -> bool {
        let at = out.len();
        let d = cs(digits);
        let (lead0, pause) = q::set_pauses(d);
        let sep = self.w(if pause { 0 } else { 1 });
        q::add_separator(out, at, sep);
        let mut i = 0;
        while d.get(i) == Some(&b'0') {
            out.extend_from_slice(self.digit_word(b'0'));
            i += 1;
            if d.get(i) != Some(&b'0') {
                break;
            }
            q::add_separator(out, at, sep);
        }
        let r = &d[i..];
        let n = r.len() & 0xffff;
        if n != 0 {
            if lead0 {
                q::add_separator(out, at, sep);
            }
            for k in 1..n {
                out.extend_from_slice(self.digit_word(r[k - 1]));
                q::add_separator(out, at, sep);
            }
            out.extend_from_slice(self.digit_word(r[n - 1]));
        }
        pause
    }

    /// Slot 5 (0x100089d0): the currency's unit or cent name.
    fn currency_at(&self, out: &mut Vec<u8>, amount: &[u8], currency: &[u8], cents: bool) {
        let Some(k) = (0..12u32).find(|&k| self.s(self.pe.u32(CURRENCIES + 0x14 * k)) == cs(currency)) else {
            return;
        };
        let end = out.len();
        q::add_separator(out, end, self.w(1));
        let one = cs(amount) == self.s(0x1002a57c);
        let field = match (cents, one) {
            (true, true) => 0xc,
            (true, false) => 0x10,
            (false, true) => 4,
            (false, false) => 8,
        };
        out.extend_from_slice(self.s(self.pe.u32(CURRENCIES + 0x14 * k + field)));
        let end = out.len();
        q::add_separator(out, end, self.w(1));
    }
}

impl NumSlots for Numeri<'_> {
    fn word(&self, id: u32) -> &[u8] {
        self.w(id)
    }
    fn digits(&self, out: &mut Vec<u8>, digits: &[u8], _flag: bool) {
        self.digits_at(out, digits)
    }
    fn cardinal(&self, out: &mut Vec<u8>, digits: &[u8]) {
        self.cardinal_at(out, digits)
    }
    fn digit(&self, d: u8) -> &[u8] {
        self.digit_word(d)
    }
    /// Slot 4 (0x10004c20)
    fn by_digits(&self, flag: bool, len: usize, lead0: bool) -> bool {
        flag || len > 0xf || (len > 3 && lead0) || len == 1
    }
    fn currency(&self, out: &mut Vec<u8>, amount: &[u8], currency: &[u8], cents: bool) {
        self.currency_at(out, amount, currency, cents)
    }
    fn decimals(&self, out: &mut Vec<u8>, digits: &[u8], _flag: bool) -> bool {
        self.decimals_at(out, digits)
    }
}

/// One call of slot 9: the token buffer and the engine's ctx+0x18.
struct Call<'a, 'b> {
    n: &'b Numeri<'a>,
    toks: &'b mut [Tok],
    c18: usize,
    df: u32,
}

impl Call<'_, '_> {
    fn kind(&self, k: usize) -> u32 {
        self.toks.get(k).map(|t| t.kind).unwrap_or(7)
    }
    /// 0x10001b90: the token `delta` real tokens (control tags skipped)
    /// from `idx`; past the end the buffer's end entry (no text).
    fn tok(&self, delta: i32, idx: usize) -> usize {
        let mut i = idx;
        let mut d = delta;
        if d < 0 {
            while d < 0 && i != 0 {
                i -= 1;
                if self.kind(i) != tok::TAG {
                    d += 1;
                }
            }
        } else if d > 0 {
            let mut u = i;
            loop {
                u += 1;
                if u >= 2 * 0xf + 1 || self.kind(i) == 7 {
                    break;
                }
                i += 1;
                if self.kind(i) != tok::TAG {
                    d -= 1;
                }
                if d <= 0 {
                    break;
                }
            }
        }
        i
    }
    fn text(&self, k: usize) -> Vec<u8> {
        self.toks.get(k).map(|t| t.text.clone()).unwrap_or_default()
    }
    fn clear(&mut self, k: usize) {
        if let Some(t) = self.toks.get_mut(k) {
            t.text.clear();
        }
    }

    /// Slot 7 (0x10004c80): `ELQNumExtractDate` step 0 then step 1 (no
    /// minimum year), the parts into the object's day/month/year.
    fn slot7(order: &[u8], text: &[u8], obj: &mut NumObj) -> bool {
        let mut d = q::Date::default();
        let ok = q::extract_date(order, text, &mut d) && q::validate_date(order, &mut d, 0);
        obj.set_u32(0x98, d.day);
        obj.set_u32(0x9c, d.month);
        obj.set_u32(0xa0, d.year);
        ok
    }

    // --- recognizers (0x10009150's table) ---

    /// 0x10005380: an ordinal (`21` before `st`), kind 4.
    fn r_ordinal(&mut self, idx: usize, obj: &mut NumObj) -> bool {
        let ni = self.tok(1, idx);
        let next = self.text(ni);
        obj.set_u32(0, 4);
        let l = obj.text().to_vec();
        let mut t = unsigned(&l).0.to_vec();
        swap_dot_comma(&mut t);
        obj.set_cstr(4, &t);
        let (dots, commas) = count_marks(&t);
        if dots + commas != 0 {
            q::set_default_type(obj, &l);
            return false;
        }
        if obj.mode() == 4 {
            return true;
        }
        if [0x1002a7e4, 0x1002a7e0, 0x1002a7dc, 0x1002a7d8].iter().any(|&a| next == self.n.s(a)) {
            self.c18 = ni - idx;
            obj.set_u32(0x48, 5);
            return true;
        }
        q::set_default_type(obj, &l);
        false
    }

    /// 0x10004d30: a date, kind 8 (+0x88: 8 numeric, 0xc a range of years,
    /// 0xd a decade `1990s`, 0xe with a month name).
    fn r_date(&mut self, idx: usize, dmy: &[u8], obj: &mut NumObj) -> bool {
        let ni = self.tok(1, idx);
        let next = self.text(ni);
        obj.set_u32(0, 8);
        obj.set_u32(0x88, 8);
        obj.set_u8(0x8c, 0);
        obj.set_u8(0x94, 0);
        obj.set_u8(0x48, 0);
        let order: Option<&[u8]> = match self.df {
            0x10 | 0 => Some(b"dmy"),
            0x11 => Some(b"mdy"),
            0x12 => Some(b"ymd"),
            0x14 => Some(self.n.s(0x1002a7d0)),
            0x13 => Some(self.n.s(0x10029bdc)),
            0x15 => Some(self.n.s(0x1002a7cc)),
            0x16 => Some(self.n.s(0x1002a7c8)),
            0x17 => Some(b"d"),
            0x18 => Some(b"m"),
            0x19 => Some(b"y"),
            _ => None,
        };
        if let Some(o) = order {
            obj.set_cstr(0x8c, o);
        }
        let l = obj.text().to_vec();
        obj.set_cstr(4, unsigned(&l).0);
        if ieq(&next, self.n.s(0x1002a3f0)) {
            self.c18 = ni - idx;
            obj.set_u32(0x88, 0xd);
            obj.set_u32(0x44, 8);
        } else if next == self.n.s(0x1002a7c4) {
            let n2 = self.tok(2, idx);
            if ieq(&self.text(n2), self.n.s(0x1002a3f0)) {
                self.c18 = n2 - idx;
                obj.set_u32(0x88, 0xd);
                obj.set_u32(0x44, 8);
            }
        }
        let mode = obj.mode();
        if mode == 0 || mode == 8 {
            let order = obj.cstr(0x8c).to_vec();
            let text = obj.text().to_vec();
            if Self::slot7(&order, &text, obj) {
                return true;
            }
            if self.r_years(idx, &text, obj) {
                obj.set_u32(0x88, 0xc);
                return true;
            }
            if obj.mode() == 8 {
                let mut d = q::Date::default();
                let ok = q::format_vxml10_date(&text, &mut d);
                obj.set_u32(0x98, d.day);
                obj.set_u32(0x9c, d.month);
                obj.set_u32(0xa0, d.year);
                if ok && ieq(dmy, self.n.s(0x10029da0)) {
                    return true;
                }
            }
            // 0x10004ce0: a year alone
            if obj.u32(0x88) == 0xd && text.iter().all(u8::is_ascii_digit) {
                let y = q::atol(&text) as u32;
                obj.set_u32(0xa0, y);
                if y > 0 && y <= 9999 {
                    return true;
                }
            }
            if self.r_month_date(idx, &text, obj) {
                obj.set_u32(0x88, 0xe);
                return true;
            }
        }
        q::set_default_type(obj, &l);
        false
    }

    /// 0x100048d0: a range of years (`1990-2000`, `1990 - 2000`,
    /// `between 1990 and 2000`) into +0xa4/+0xa8, the joiner at +0x94.
    fn r_years(&mut self, idx: usize, text: &[u8], obj: &mut NumObj) -> bool {
        let next = self.text(self.tok(1, idx));
        let nni = self.tok(2, idx);
        let nn = self.text(nni);
        let prev = self.text(self.tok(-1, idx));
        obj.set_u32(0xa8, 0);
        obj.set_u32(0xa4, 0);
        let l = &text[..text.len().min(0x3f)];
        let ok = |obj: &NumObj| {
            let (a, b) = (obj.u32(0xa4), obj.u32(0xa8));
            a > 0 && b <= 9999 && a < b
        };
        if let Some(p) = strpbrk(l, self.n.s(0x1002a7c0)) {
            if !four_digits(&l[..p]) {
                return false;
            }
            obj.set_u32(0xa4, q::atol(&l[..p]) as u32);
            if !four_digits(&l[p + 1..]) {
                return false;
            }
            obj.set_u32(0xa8, q::atol(&l[p + 1..]) as u32);
            return ok(obj);
        }
        if four_digits(l) && next == self.n.s(0x1002a7b4) && four_digits(&nn) {
            obj.set_u32(0xa4, q::atol(l) as u32);
            obj.set_u32(0xa8, q::atol(&nn) as u32);
            obj.set_u8(0x94, b'-');
            if !ok(obj) {
                return false;
            }
            self.c18 = nni - idx;
            return true;
        }
        if ieq(&prev, self.n.s(0x1002a7b8)) && four_digits(l) && ieq(&next, self.n.s(0x100272b8)) && four_digits(&nn) {
            obj.set_u32(0xa4, q::atol(l) as u32);
            obj.set_u32(0xa8, q::atol(&nn) as u32);
            obj.set_u8(0x94, b'&');
            if !ok(obj) {
                return false;
            }
            self.c18 = nni - idx;
            return true;
        }
        false
    }

    /// `ELQNumExtractDate` step 0 for 0x10004610: the parts go to the day
    /// (always) and the year when wanted (the month to a local).
    fn extract(order: &[u8], text: &[u8], obj: &mut NumObj, year: bool) -> bool {
        let mut d = q::Date::default();
        let ok = q::extract_date(order, text, &mut d);
        obj.set_u32(0x98, d.day);
        if year {
            obj.set_u32(0xa0, d.year);
        }
        ok
    }

    /// 0x10004610: a day or year next to a month name (`15 March`,
    /// `March 15`, `1066 AD`, `in 1984`).
    fn r_month_date(&mut self, idx: usize, text: &[u8], obj: &mut NumObj) -> bool {
        let ni = self.tok(1, idx);
        let next = self.text(ni);
        let prev = self.text(self.tok(-1, idx));
        let prev2 = self.text(self.tok(-2, idx));
        obj.set_u32(0x98, 0);
        obj.set_u32(0x9c, 0);
        obj.set_u32(0xa0, 0);
        let m = self.n.month(&next);
        obj.set_u32(0x9c, m);
        let d_ = self.n.s(0x1002a424);
        let y_ = self.n.s(0x1002a3dc);
        if (1..=12).contains(&m) && Self::extract(d_, text, obj, false) {
            self.c18 = ni - idx;
            return true;
        }
        // the other matches return through 0x100046c9, after the c18 store
        for k in 0..2 {
            if ieq(&next, self.n.s(self.n.pe.u32(ERAS + 4 * k))) && four_digits(text) && Self::extract(y_, text, obj, true) {
                return true;
            }
        }
        if idx == 0 {
            return false;
        }
        for k in 0..10 {
            if ieq(&prev, self.n.s(self.n.pe.u32(YEAR_WORDS + 4 * k))) && four_digits(text) && Self::extract(y_, text, obj, true) {
                return true;
            }
        }
        let joined = [0x1002a7b4, 0x1002a7b0, 0x10026714].iter().any(|&a| prev == self.n.s(a));
        let w = if joined { &prev2 } else { &prev };
        let m2 = self.n.month(w);
        if !(1..=12).contains(&m2) {
            return false;
        }
        if Self::extract(d_, text, obj, false) {
            return !(joined && self.text(idx).first() == Some(&b'0'));
        }
        four_digits(text) && Self::extract(y_, text, obj, true)
    }

    /// 0x100050d0: a time, kind 7.
    fn r_time(&mut self, obj: &mut NumObj) -> bool {
        obj.set_u8(0x8d, 0);
        obj.set_u8(0x8e, 0);
        let l = obj.text().to_vec();
        obj.set_u32(0x48, TIME_FORMAT);
        obj.set_u32(0, 7);
        obj.set_u8(0x4c, 0);
        for o in 0x8c..0x90 {
            obj.set_u8(o, 0);
        }
        let (t, sign) = unsigned(&l);
        match sign {
            b'-' => obj.set_u8(0x8d, 1),
            b'+' => obj.set_u8(0x8e, 1),
            _ => {}
        }
        let mut t = t.to_vec();
        swap_dot_comma(&mut t);
        obj.set_cstr(4, &t);
        let (dots, commas) = count_marks(&t);
        let mode = obj.mode();
        if dots != 0 {
            q::set_default_type(obj, &l);
            return false;
        }
        if commas == 0 {
            if matches!(mode, 7 | 8 | 0) && obj.u8(0x8d) == 0 && obj.u8(0x8e) == 0 && q::is_time(&t) {
                return true;
            }
            q::set_default_type(obj, &l);
            return false;
        }
        if commas == 1 {
            let p = t.iter().position(|&c| c == b',').unwrap();
            let (a, b) = (t[..p].to_vec(), t[p + 1..].to_vec());
            if matches!(mode, 7 | 8) && obj.u8(0x8d) == 0 {
                if obj.u8(0x8e) == 0 && q::is_time(&t) {
                    return true;
                }
                if obj.u8(0x8e) == 0 && q::is_time(&a) {
                    if a == self.n.s(0x1002a7d4) {
                        obj.set_u8(0x8f, 1);
                        obj.set_cstr(0x4c, &b);
                    }
                    obj.set_cstr(4, &a);
                    obj.set_u8(0x8c, 1);
                }
            }
        }
        q::set_default_type(obj, &l);
        false
    }

    /// 0x10005590: an amount of money, kind 12 (the currency at +0x4c).
    fn r_currency(&mut self, idx: usize, obj: &mut NumObj) -> bool {
        let ni = self.tok(1, idx);
        let nni = self.tok(2, idx);
        let pi = self.tok(-1, idx);
        let next = self.text(ni);
        let nn = self.text(nni);
        let prev = self.text(pi);
        obj.set_u8(0x56, 0);
        obj.set_u8(0x57, 0);
        obj.set_u8(0x58, 0);
        obj.set_u32(0, 0xc);
        obj.set_u32(0x48, 0xf);
        obj.set_u8(0x4c, 0);
        let l = obj.text().to_vec();
        let (t, sign) = unsigned(&l);
        match sign {
            b'-' => obj.set_u8(0x56, 1),
            b'+' => obj.set_u8(0x57, 1),
            _ => {}
        }
        let mut t = t.to_vec();
        swap_dot_comma(&mut t);
        obj.set_cstr(4, &t);
        if q::is_currency(&prev) {
            obj.set_cstr(0x4c, &prev);
            return true;
        }
        let s = |a: u32| self.n.s(a);
        let any = |w: &[u8], l: &[u32]| l.iter().any(|&a| ieq(w, s(a)));
        let dollar = s(0x10029c48);
        // (currency, uses the token after next too)
        let found: Option<(&[u8], bool)> = if any(&next, &[0x10029bf8, 0x10029bf0, 0x1002a820]) {
            Some((s(0x10029c00), false))
        } else if any(&next, &[0x10029c40, 0x10029c38, 0x1002a81c]) {
            Some((dollar, false))
        } else if (ieq(&next, s(0x1002a818)) && nn == dollar) || (next == dollar && nn == s(0x1002a818)) {
            Some((s(0x10029bb0), true))
        } else if any(&next, &[0x10029be8, 0x10029be0, 0x1002a814]) {
            Some((s(0x10029bec), false))
        } else if any(&next, &[0x10029c1c, 0x10029c14, 0x1002a810]) {
            Some((s(0x10029c24), false))
        } else if any(&next, &[0x10026f14, 0x10026ef8, 0x1002a80c]) {
            Some((s(0x10029bdc), false))
        } else if any(&next, &[0x10029bbc, 0x10029bb4, 0x1002a808]) {
            Some((s(0x10029bc4), false))
        } else if ieq(&next, s(0x1002a804)) {
            Some((s(0x10029b94), false))
        } else if (next == dollar && any(&nn, &[0x1002a428, 0x1002a800, 0x1002a7fc])) || (ieq(&next, s(0x1002a428)) && nn == dollar) {
            Some((s(0x10029b94), true))
        } else if ieq(&next, s(0x1002a7f8)) {
            Some((s(0x10029ae8), false))
        } else if ieq(&next, s(0x1002a7f4)) {
            Some((s(0x10029b6c), false))
        } else if any(&next, &[0x1002a7f0, 0x10029b40]) {
            Some((s(0x10029b40), false))
        } else if ieq(&next, s(0x1002a7ec)) {
            Some((s(0x10029b14), false))
        } else if (next == dollar && any(&nn, &[0x1002a42c, 0x1002a7e8])) || (any(&next, &[0x1002a42c, 0x1002a7e8]) && nn == dollar) {
            Some((s(0x10029b14), true))
        } else {
            None
        };
        if let Some((c, two)) = found {
            obj.set_cstr(0x4c, c);
            if two {
                self.clear(ni);
                self.c18 = nni - idx;
            } else {
                self.c18 = ni - idx;
            }
            return true;
        }
        if q::is_currency(&next) {
            obj.set_cstr(0x4c, &next);
            self.clear(ni);
            self.c18 = ni - idx;
            return true;
        }
        q::set_default_type(obj, &l);
        false
    }

    /// 0x10005c80: kind 9 (a mode-9 number in two parts).
    fn r_kind9(&mut self, obj: &mut NumObj) -> bool {
        obj.set_u8(0x8c, 0);
        obj.set_u32(0, 9);
        obj.set_u32(0x48, 9);
        obj.set_u8(0x4c, 0);
        for o in 0xcc..0xcf {
            obj.set_u8(o, 0);
        }
        let l = obj.text().to_vec();
        let (t, sign) = unsigned(&l);
        match sign {
            b'-' => obj.set_u8(0xcc, 1),
            b'+' => obj.set_u8(0xcd, 1),
            _ => {}
        }
        let mut t = t.to_vec();
        swap_dot_comma(&mut t);
        obj.set_cstr(4, &t);
        let (dots, commas) = count_marks(&t);
        let mode = obj.mode();
        if dots == 0 {
            if commas == 0 {
                if mode == 9 {
                    obj.set_cstr(0x4c, &t);
                    return true;
                }
            } else if commas == 1 {
                if mode == 9 {
                    let p = t.iter().position(|&c| c == b',').unwrap();
                    obj.set_cstr(0x4c, &t[p + 1..]);
                }
                if obj.u8(0x4c) != 0 {
                    obj.set_u8(0x8c, 0);
                    return true;
                }
            }
        } else if commas == 0 {
            if matches!(mode, 0 | 3) && q::is_big_integer(&t) {
                obj.set_u32(0x48, 3);
            } else if mode == 9 && q::is_big_integer(&t) {
                obj.set_cstr(0x4c, &t);
                return true;
            }
        } else if commas == 1 {
            if let (Some(p), Some(c)) = (strrchr(&t, b'.'), strrchr(&t, b',')) {
                if p < c && q::is_big_integer(&t[..c]) && mode == 9 {
                    obj.set_cstr(0x4c, &t[..c]);
                    obj.set_u32(0x48, 3);
                    obj.set_cstr(0x8c, &t[c + 1..]);
                    return true;
                }
            }
        }
        q::set_default_type(obj, &l);
        false
    }

    /// 0x10005f40: a fraction, kind 11.
    fn r_fraction(&mut self, idx: usize, obj: &mut NumObj) -> bool {
        let prev = self.text(self.tok(-1, idx));
        obj.set_u32(0, 0xb);
        obj.set_u32(0x48, 0xb);
        obj.set_u8(0x4c, 0);
        obj.set_u8(0x4d, 0);
        let l = obj.text().to_vec();
        let (t, sign) = unsigned(&l);
        match sign {
            b'-' => obj.set_u8(0x4c, 1),
            b'+' => obj.set_u8(0x4d, 1),
            _ => {}
        }
        if prev == self.n.s(0x1002a7b4) {
            obj.set_u8(0x4c, 1);
            obj.set_u8(0x4d, 1);
        }
        let mut t = t.to_vec();
        swap_dot_comma(&mut t);
        obj.set_cstr(4, &t);
        let (dots, commas) = count_marks(&t);
        if dots + commas == 0 && matches!(obj.mode(), 8 | 0 | 0xb) && self.n.fraction(&t) {
            return true;
        }
        q::set_default_type(obj, &l);
        false
    }

    /// 0x10006100: a telephone number, kind 1 (following digit groups
    /// joined; a country code at +0x4c/+0x4d).
    fn r_phone(&mut self, idx: usize, obj: &mut NumObj) -> bool {
        let ni = self.tok(1, idx);
        let nni = self.tok(2, idx);
        let next = self.text(ni);
        let nn = self.text(nni);
        let prev = self.text(self.tok(-1, idx));
        obj.set_u32(0, 1);
        obj.set_u32(0x48, 1);
        obj.set_u8(0x4c, 0);
        obj.set_u8(0x8d, 0);
        obj.set_u8(0x4d, 0);
        let l = obj.text().to_vec();
        let mut t = unsigned(&l).0.to_vec();
        swap_dot_comma(&mut t);
        obj.set_cstr(4, &t);
        let (dots, commas) = count_marks(&t);
        if dots != 0 {
            q::set_default_type(obj, &l);
            return false;
        }
        let mode = obj.mode();
        if commas != 0 {
            if commas == 1 {
                let p = t.iter().position(|&c| c == b',').unwrap();
                let (a, b) = (&t[..p], &t[p + 1..]);
                if a != self.n.s(0x1002a7d4) && mode == 1 && self.phone_prefix(a) != 0 {
                    let (a, b) = (a.to_vec(), b.to_vec());
                    obj.set_cstr(4, &a);
                    obj.set_u8(0x4c, 1);
                    obj.set_cstr(0x4d, &b);
                    return true;
                }
            } else if t.len() > 5 {
                let set = self.n.s(0x1002a824);
                if let Some(p) = strpbrk(&t, set) {
                    if strpbrk(&t[p + 1..], set).is_some() {
                        return true;
                    }
                }
            }
            q::set_default_type(obj, &l);
            return false;
        }
        match mode {
            1 => {
                obj.set_u8(0x4c, 1);
                return true;
            }
            0 | 8 => {}
            _ => {
                q::set_default_type(obj, &l);
                return false;
            }
        }
        if strpbrk(&t, self.n.s(0x1002a844)).is_some() {
            return false;
        }
        if t.len() > 5 {
            if let Some(p) = strpbrk(&t, self.n.s(0x1002a83c)) {
                if date_like(&t[p + 1..]) {
                    return true;
                }
            }
        }
        if digits_only(&next) {
            if !date_like(&next) || ni != idx + 1 {
                q::set_default_type(obj, &l);
                return false;
            }
            let mut text = t.clone();
            text.extend_from_slice(&next);
            obj.set_cstr(4, &text);
            self.c18 = ni - idx;
            if digits_only(&nn) {
                if date_like(&nn) && nni == ni + 1 {
                    text.extend_from_slice(&nn);
                    obj.set_cstr(4, &text);
                    self.c18 = nni - idx;
                }
                let n3 = self.tok(3, idx);
                let t3 = self.text(n3);
                if digits_only(&t3) && date_like(&t3) && n3 == nni + 1 {
                    text.extend_from_slice(&t3);
                    obj.set_cstr(4, &text);
                    self.c18 = n3 - idx;
                }
            }
            return true;
        }
        if idx > 0
            && [0x1002a838, 0x1002a834].iter().any(|&a| prev == self.n.s(a))
            && [0x1002a830, 0x1002a82c].iter().any(|&a| next == self.n.s(a))
            && date_like(&nn)
        {
            return true;
        }
        q::set_default_type(obj, &l);
        false
    }

    /// 0x10004bb0: the length of the country-code prefix `s` starts with.
    fn phone_prefix(&self, s: &[u8]) -> usize {
        let key = &s[..s.len().min(10)];
        (0..3)
            .map(|k| self.n.s(self.n.pe.u32(PHONE_PREFIXES + 4 * k)))
            .find(|e| key.starts_with(e))
            .map(|e| e.len())
            .unwrap_or(0)
    }

    /// 0x100068b0: a cardinal or decimal number, kind 10 (integer part
    /// +0x4c, decimals +0x8c; +0x48 3 grouped, 0xc a range `3-5`).
    fn r_cardinal(&mut self, idx: usize, obj: &mut NumObj) -> bool {
        let next = self.text(self.tok(1, idx));
        let prev = self.text(self.tok(-1, idx));
        let prev2 = self.text(self.tok(-2, idx));
        obj.set_u8(0x4c, 0);
        obj.set_u32(0, 0xa);
        obj.set_u32(0x48, 0xa);
        obj.set_u8(0x8c, 0);
        for o in 0xcc..0xd1 {
            obj.set_u8(o, 0);
        }
        let l = obj.text().to_vec();
        let s = |a: u32| self.n.s(a);
        let (gt, lt) = (s(0x1002a854), s(0x1002a850));
        let side = |obj: &mut NumObj, w: &[u8]| {
            if w == gt {
                obj.set_u8(0xce, 1);
            } else if w == lt {
                obj.set_u8(0xcf, 1);
            }
        };
        if prev == s(0x1002a7b4) {
            obj.set_u8(0xcc, 1);
            side(obj, &prev2);
        } else if prev == s(0x10026920) {
            obj.set_u8(0xcd, 1);
            side(obj, &prev2);
        }
        side(obj, &prev);
        let (t, sign) = unsigned(&l);
        match sign {
            b'-' => obj.set_u8(0xcc, 1),
            b'+' => obj.set_u8(0xcd, 1),
            _ => {}
        }
        let mut t = t.to_vec();
        swap_dot_comma(&mut t);
        obj.set_cstr(4, &t);
        let (dots, commas) = count_marks(&t);
        let mode = obj.mode();
        let zero = s(0x1002a7d4);
        if dots != 0 {
            if commas == 0 {
                if matches!(mode, 0 | 3 | 0xa) && q::is_big_integer(&t) {
                    obj.set_cstr(0x4c, &t);
                    obj.set_u32(0x48, 3);
                    return true;
                }
            } else if commas == 1 {
                if let (Some(p), Some(c)) = (strrchr(&t, b'.'), strrchr(&t, b',')) {
                    if p < c && q::is_big_integer(&t[..c]) && matches!(mode, 0 | 3) {
                        obj.set_cstr(0x4c, &t[..c]);
                        obj.set_cstr(0x8c, &t[c + 1..]);
                        obj.set_u32(0x48, 3);
                        return true;
                    }
                }
            }
            q::set_default_type(obj, &l);
            return false;
        }
        if commas == 0 {
            match mode {
                3 | 0xa => {
                    obj.set_cstr(0x4c, &t);
                    if t == zero {
                        obj.set_u8(0xd0, 1);
                    }
                    return true;
                }
                0 | 8 => {
                    if t == zero {
                        obj.set_u8(0xd0, 1);
                    }
                    if strpbrk(&t, s(0x1002a848)).is_none() && t.len() < 6 {
                        obj.set_cstr(0x4c, &t);
                        return true;
                    }
                    if let Some(p) = strpbrk(&t, s(0x1002a7b4)) {
                        let (a, b) = (&t[..p], &t[p + 1..]);
                        let (x, y) = (q::atol(a) as u32, q::atol(b) as u32);
                        if x < y || (x > y && ieq(&next, s(0x10029c60))) {
                            let (a, b) = (a.to_vec(), b.to_vec());
                            obj.set_cstr(0x4c, &a);
                            obj.set_cstr(0x8c, &b);
                            obj.set_u32(0x48, 0xc);
                            return true;
                        }
                    }
                }
                _ => {}
            }
            q::set_default_type(obj, &l);
            return false;
        }
        if commas == 1 {
            let p = t.iter().position(|&c| c == b',').unwrap();
            let (a, b) = (t[..p].to_vec(), t[p + 1..].to_vec());
            if a == zero {
                obj.set_u8(0xd0, 1);
                obj.set_cstr(0x8c, &b);
                return true;
            }
            if matches!(mode, 0 | 7 | 8 | 0xa) {
                if strpbrk(&t, s(0x1002a848)).is_none() {
                    obj.set_cstr(0x4c, &a);
                }
                obj.set_cstr(0x8c, &b);
                return true;
            }
        }
        q::set_default_type(obj, &l);
        false
    }

    /// 0x10006690: a code read digit by digit, kind 2.
    fn r_digits(&mut self, obj: &mut NumObj) -> bool {
        obj.set_u32(0, 2);
        obj.set_u32(0x48, 2);
        obj.set_u8(0x8c, 0);
        obj.set_u8(0x4c, 0);
        obj.set_u8(0x8f, 0);
        obj.set_u8(0x8d, 0);
        obj.set_u8(0x8e, 0);
        let l = obj.text().to_vec();
        let (t, sign) = unsigned(&l);
        match sign {
            b'-' => obj.set_u8(0x8d, 1),
            b'+' => obj.set_u8(0x8e, 1),
            _ => {}
        }
        let mut t = t.to_vec();
        swap_dot_comma(&mut t);
        obj.set_cstr(4, &t);
        let (dots, commas) = count_marks(&t);
        if t.contains(&b':') {
            return false;
        }
        if dots == 0 {
            if commas == 0 {
                return true;
            }
            if commas == 1 {
                // these fields stay, but the recognizer gives up all the same
                let p = t.iter().position(|&c| c == b',').unwrap();
                let (a, b) = (t[..p].to_vec(), t[p + 1..].to_vec());
                let mode = obj.mode();
                if a == self.n.s(0x1002a7d4) {
                    obj.set_u8(0x8f, 1);
                } else if mode == 2 {
                    obj.set_u8(0x8c, 1);
                    obj.set_cstr(4, &a);
                }
                if mode == 2 {
                    obj.set_cstr(0x4c, &b);
                }
            }
        }
        q::set_default_type(obj, &l);
        false
    }

    /// The recognizer `k` of 0x10009150's table.
    fn recognize(&mut self, k: usize, idx: usize, dmy: &[u8], obj: &mut NumObj) -> bool {
        match k {
            0 => self.r_ordinal(idx, obj),
            1 => self.r_date(idx, dmy, obj),
            2 => self.r_time(obj),
            3 => self.r_currency(idx, obj),
            4 => self.r_kind9(obj),
            5 => self.r_fraction(idx, obj),
            6 => self.r_phone(idx, obj),
            7 => self.r_cardinal(idx, obj),
            _ => self.r_digits(obj),
        }
    }

    // --- renderers (0x10009ea0) ---

    /// 0x10003720: the ordinal of `text` into `out` (cleared first).
    fn ordinal(&self, out: &mut Vec<u8>, text: &[u8]) {
        let n = self.n;
        out.clear();
        let z = text.iter().take_while(|&&c| c == b'0').count();
        let p = &text[z..];
        let len = p.len();
        let mut l = p[..len.min(0x3f)].to_vec();
        let g = |k: usize| p[k];
        let key: Vec<u8> = if len > 1 && g(len - 1) != b'0' && g(len - 2) != b'1' {
            l[len - 1] = b'0';
            q::convert_integer(out, 0, &l, false, n);
            if out.is_empty() {
                return;
            }
            out.extend_from_slice(n.s(0x10029ab8));
            vec![g(len - 1)]
        } else if len <= 2 {
            l
        } else if g(len - 1) == b'0' && g(len - 2) == b'0' {
            l.truncate(len - 2);
            q::convert_integer(out, 0, &l, false, n);
            if out.is_empty() {
                return;
            }
            out.extend_from_slice(n.s(0x10029ab8));
            vec![g(len - 2), g(len - 1)]
        } else {
            l[len - 1] = b'0';
            l[len - 2] = b'0';
            q::convert_integer(out, 0, &l, false, n);
            if out.is_empty() {
                return;
            }
            out.extend_from_slice(n.s(0x10029ab8));
            vec![g(len - 2), g(len - 1)]
        };
        if let Some(&(_, e)) = ORDINALS.iter().find(|&&(k, _)| key == n.s(k)) {
            out.extend_from_slice(n.s(e));
        }
    }

    /// 0x10008be0: a year (into its own buffer, which callers append).
    fn year(&self, y: u32) -> Vec<u8> {
        let n = self.n;
        let mut out = Vec::new();
        if y == 0 {
            return out;
        }
        let b = ltoa(y);
        let g = |k: usize| b.get(k).copied().unwrap_or(0);
        let one = |c: u8| n.num_word(c as i32 - b'0' as i32);
        if y - 1 <= 0x62 {
            out.extend_from_slice(&n.two(&b));
        } else if y <= 999 {
            out.extend_from_slice(one(g(0)));
            out.extend_from_slice(n.s(0x1002a8dc));
            if &b[1..] != n.s(0x1002a660) {
                out.extend_from_slice(n.s(0x1002a8d4));
                out.extend_from_slice(&n.two(&b[1..]));
            }
        } else if &b[1..] == n.s(0x1002a8d0) {
            out.extend_from_slice(one(g(0)));
            out.extend_from_slice(n.s(0x1002a8c4));
        } else if g(1) == b'0' {
            out.extend_from_slice(one(g(0)));
            out.extend_from_slice(n.s(0x10029930));
            if g(2) == b'0' {
                out.extend_from_slice(n.num_word(q::atol(&b[3..])));
            } else {
                out.extend_from_slice(&n.two(&b[2..]));
            }
        } else if g(2) == b'0' {
            out.extend_from_slice(&n.two(&b[..2]));
            out.extend_from_slice(n.s(0x1002a8dc));
            if g(3) != b'0' {
                out.extend_from_slice(n.s(0x1002a8d4));
                out.extend_from_slice(n.num_word(q::atol(&b[3..])));
            }
        } else {
            out.extend_from_slice(&n.two(&b[..2]));
            q::add_separator(&mut out, 0, n.w(1));
            out.extend_from_slice(&n.two(&b[2..]));
        }
        out
    }

    /// 0x10009230: a date (month, day, year as the order has them).
    fn date(&self, out: &mut Vec<u8>, obj: &NumObj) {
        let n = self.n;
        let (day, month, year) = (obj.u32(0x98), obj.u32(0x9c), obj.u32(0xa0));
        let order = obj.cstr(0x8c);
        let has = |c: u32| order.contains(&n.s(c)[0]);
        if (1..=12).contains(&month) && has(0x10029da8) {
            out.extend_from_slice(n.w(0x50 + month));
            q::add_separator(out, 0, n.w(1));
        }
        if day > 0 && has(0x1002a424) {
            let mut b = Vec::new();
            self.ordinal(&mut b, &ltoa(day));
            out.extend_from_slice(&b);
            q::add_separator(out, 0, n.w(1));
        }
        if has(0x1002a3dc) {
            out.extend_from_slice(&self.year(year));
        }
    }

    /// 0x10007c30: a time `h:mm[:ss]` (format 7).
    fn time(&self, out: &mut Vec<u8>, text: &[u8], idx: usize) {
        let n = self.n;
        let next = self.text(self.tok(1, idx));
        let nn = self.text(self.tok(2, idx));
        let mut b = text[..text.len().min(0x3f)].to_vec();
        let seps = n.s(0x1002a8c0);
        let Some(e) = strpbrk(&b, seps) else {
            return;
        };
        let mut h = b[..e].to_vec();
        if h.first() == Some(&b'0') && h.get(1).is_some_and(|c| (b'1'..=b'9').contains(c)) {
            h.remove(0);
        }
        q::convert_integer(out, 0, &h, false, n);
        b = b[e + 1..].to_vec();
        let ci = |out: &mut Vec<u8>, d: &[u8]| {
            let end = out.len();
            q::convert_integer(out, end, d, false, n);
        };
        let sep = |out: &mut Vec<u8>, id: u32| q::add_separator(out, 0, n.w(id));
        let g = |s: &[u8], k: usize| s.get(k).copied().unwrap_or(0);
        if let Some(e2) = strpbrk(&b, seps) {
            let m = b[..e2].to_vec();
            let s = b[e2 + 1..].to_vec();
            let m00 = g(&m, 0) == b'0' && g(&m, 1) == b'0';
            let s00 = g(&s, 0) == b'0' && g(&s, 1) == b'0';
            if !m.is_empty() && !m00 && !s.is_empty() && !s00 {
                sep(out, 1);
                ci(out, &m);
                sep(out, 1);
                sep(out, 0xd);
                sep(out, 1);
                ci(out, &s);
                sep(out, 0x5d);
                return;
            }
            if g(&m, 0) == b'0' {
                if m00 && s00 {
                    return;
                }
                if m00 {
                    sep(out, 1);
                    sep(out, 0xd);
                    sep(out, 1);
                    ci(out, &s);
                    sep(out, 0x5d);
                    return;
                }
            }
            if s00 {
                sep(out, 1);
                ci(out, &m);
            }
            return;
        }
        let m = b;
        if m.is_empty() {
            return;
        }
        if !(g(&m, 0) == b'0' && g(&m, 1) == b'0') {
            sep(out, 1);
            let mut rest: &[u8] = &m;
            if g(&m, 2) == b'-' {
                ci(out, &m[..2]);
                sep(out, 0);
                rest = &m[3..];
            }
            ci(out, rest);
            return;
        }
        let s = |a: u32| n.s(a);
        if [0x1002a8bc, 0x1002a8b8, 0x1002a8b0, 0x1002a8a8].iter().any(|&a| ieq(&next, s(a)))
            || (ieq(&next, s(0x1002a8a4)) && ieq(&nn, s(0x1002a8a0)))
            || (ieq(&next, s(0x1002a89c)) && ieq(&nn, s(0x1002a8a0)))
        {
            out.extend_from_slice(s(0x10029ab8));
        } else {
            out.extend_from_slice(s(0x1002a890));
        }
    }

    /// 0x100080a0: a duration `h:mm[:ss]` (format 0x1a).
    fn duration(&self, out: &mut Vec<u8>, text: &[u8]) {
        let n = self.n;
        let mut b = text.to_vec();
        let seps = n.s(0x1002a8c0);
        let Some(e) = strpbrk(&b, seps) else {
            return;
        };
        let mut h = b[..e].to_vec();
        if h.first() == Some(&b'0') && h.get(1).is_some_and(|c| (b'1'..=b'9').contains(c)) {
            h.remove(0);
        }
        q::convert_integer(out, 0, &h, false, n);
        let sep = |out: &mut Vec<u8>, id: u32| q::add_separator(out, 0, n.w(id));
        sep(out, 1);
        sep(out, if h == b"1" { 0x62 } else { 0x5f });
        let ci = |out: &mut Vec<u8>, d: &[u8]| {
            let end = out.len();
            q::convert_integer(out, end, d, false, n);
        };
        let g = |s: &[u8], k: usize| s.get(k).copied().unwrap_or(0);
        b = b[e + 1..].to_vec();
        if let Some(e2) = strpbrk(&b, seps) {
            let m = b[..e2].to_vec();
            let s = b[e2 + 1..].to_vec();
            let m00 = g(&m, 0) == b'0' && g(&m, 1) == b'0';
            let s00 = g(&s, 0) == b'0' && g(&s, 1) == b'0';
            if !m.is_empty() && !m00 && !s.is_empty() && !s00 {
                sep(out, 1);
                ci(out, &m);
                sep(out, 1);
                sep(out, 0x5e);
                sep(out, 1);
                sep(out, 0xd);
                sep(out, 1);
                ci(out, &s);
                sep(out, 0x5d);
                return;
            }
            if g(&m, 0) == b'0' {
                if m00 && s00 {
                    return;
                }
                if m00 {
                    sep(out, 1);
                    sep(out, 0xd);
                    sep(out, 1);
                    ci(out, &s);
                    sep(out, 0x5d);
                    return;
                }
            }
            if s00 {
                sep(out, 1);
                sep(out, 0xd);
                ci(out, &m);
            }
            return;
        }
        let m = b;
        if m.is_empty() || (g(&m, 0) == b'0' && g(&m, 1) == b'0') {
            return;
        }
        sep(out, 1);
        sep(out, 0xd);
        let mut rest: &[u8] = &m;
        if g(&m, 2) == b'-' {
            ci(out, &m[..2]);
            sep(out, 0);
            rest = &m[3..];
        }
        sep(out, 1);
        ci(out, rest);
        sep(out, 0x5e);
        sep(out, 1);
    }

    /// 0x100071a0: a fraction.
    fn fraction(&self, out: &mut Vec<u8>, text: &[u8]) {
        let n = self.n;
        let p = strpbrk(text, n.s(0x10026714)).unwrap_or(text.len());
        let (num, den) = (&text[..p], text.get(p + 1..).unwrap_or(b""));
        q::convert_integer(out, 0, num, false, n);
        q::add_separator(out, 0, n.w(1));
        if let Some(k) = DENOMINATORS.iter().position(|&a| den == n.s(a)) {
            let mut w = n.s(DENOMINATOR_WORDS[k]).to_vec();
            if num == n.s(0x1002a57c) {
                w.pop();
            }
            out.extend_from_slice(&w);
        }
    }

    /// 0x10007230: an amount of money (`cur` the currency code).
    fn money(&mut self, out: &mut Vec<u8>, idx: usize, text: &[u8], cur: &[u8]) {
        let n = self.n;
        let ni = self.tok(1, idx);
        let prev = self.text(self.tok(-1, idx));
        let prev2 = self.text(self.tok(-2, idx));
        let s = |a: u32| n.s(a);
        let sep = |out: &mut Vec<u8>, id: u32| q::add_separator(out, 0, n.w(id));
        let (gt, lt) = (s(0x1002a854), s(0x1002a850));
        // a sign token before: `-` and `+` both say minus (0x100073dc)
        if prev == s(0x1002a7b4) || prev == s(0x10026920) {
            if prev2 == gt {
                sep(out, 9);
            } else if prev2 == lt {
                sep(out, 10);
            }
            sep(out, 7);
        }
        if prev == gt {
            sep(out, 9);
        } else if prev == lt {
            sep(out, 10);
        }
        let mut scaled = false;
        let next = self.text(ni);
        if let Some(k) = (0..12).find(|&k| ieq(&next, s(n.pe.u32(SCALES + 8 * k)))) {
            self.c18 += ni - idx;
            if let Some(t) = self.toks.get_mut(ni) {
                t.text = s(n.pe.u32(SCALES + 8 * k + 4)).to_vec();
            }
            scaled = true;
        }
        let next = self.text(ni);
        let units = |out: &mut Vec<u8>, a: &[u8]| {
            let end = out.len();
            if q::is_big_integer(a) {
                q::convert_big_integer(out, end, a, n);
            } else {
                n.cardinal_at(out, a);
            }
        };
        let w = |out: &mut Vec<u8>, id: u32| out.extend_from_slice(n.w(id));
        let split = strpbrk(text, s(0x1002970c)).or_else(|| {
            let p = strpbrk(text, s(0x1002a7b0))?;
            (text.len() - p == 3 && strpbrk(&text[p + 1..], s(0x1002a7b0)).is_none()).then_some(p)
        });
        let Some(p) = split else {
            units(out, text);
            if scaled {
                w(out, 1);
                out.extend_from_slice(&next);
            }
            n.currency_at(out, text, cur, false);
            return;
        };
        let (int, dec) = (&text[..p], &text[p + 1..]);
        if scaled {
            units(out, int);
            w(out, 1);
            w(out, 3);
            w(out, 1);
            n.decimals_at(out, dec);
            w(out, 1);
            out.extend_from_slice(&next);
            n.currency_at(out, dec, cur, false);
            return;
        }
        if dec.len() != 2 {
            units(out, int);
            w(out, 1);
            w(out, 3);
            w(out, 1);
            n.decimals_at(out, dec);
            n.currency_at(out, dec, cur, false);
            return;
        }
        if int == s(0x1002a7d4) {
            if dec[0] == b'0' {
                out.extend_from_slice(n.digit_word(dec[1]));
                n.currency_at(out, &dec[1..], cur, true);
            } else {
                n.cardinal_at(out, dec);
                n.currency_at(out, dec, cur, true);
            }
            return;
        }
        units(out, int);
        n.currency_at(out, int, cur, false);
        if dec == s(0x1002a660) {
            return;
        }
        w(out, 1);
        w(out, 0xd);
        w(out, 1);
        if dec[0] == b'0' {
            out.extend_from_slice(n.digit_word(dec[1]));
            n.currency_at(out, &dec[1..], cur, true);
        } else {
            n.cardinal_at(out, dec);
            n.currency_at(out, dec, cur, true);
        }
    }

    /// The minus/plus words of a sign flag pair.
    fn sign(&self, out: &mut Vec<u8>, minus: u8, plus: u8) {
        if minus == 1 {
            q::add_separator(out, 0, self.n.w(7));
        } else if plus == 1 {
            q::add_separator(out, 0, self.n.w(8));
        }
    }

    /// 0x10009ea0: the reading of the number object.
    fn render(&mut self, idx: usize, obj: &NumObj) -> Vec<u8> {
        let n = self.n;
        let mut out = Vec::new();
        let mut obj2 = NumObj::default();
        obj2.set_u32(0x44, obj.mode());
        let text = obj.text().to_vec();
        let sep = |out: &mut Vec<u8>, id: u32| q::add_separator(out, 0, n.w(id));
        match obj.kind() {
            0 => q::convert_real_number(&mut out, 0, obj, n),
            1 => {
                n.digits_at(&mut out, &text);
                if obj.u8(0x4d) != 0 {
                    obj2.set_cstr(4, obj.cstr(0x4d));
                    let end = out.len();
                    q::convert_real_number(&mut out, end, &obj2, n);
                }
            }
            2 => {
                self.sign(&mut out, obj.u8(0x8d), obj.u8(0x8e));
                let pause = if obj.u8(0x8f) != 0 {
                    n.oh(&mut out, true);
                    true
                } else {
                    q::convert_integer(&mut out, 0, &text, true, n)
                };
                if obj.u8(0x4c) != 0 {
                    sep(&mut out, if pause { 0 } else { 1 });
                    sep(&mut out, 4);
                    n.decimals_at(&mut out, obj.cstr(0x4c));
                }
            }
            3 | 0xa => {
                if obj.u8(0xce) == 1 {
                    sep(&mut out, 9);
                } else if obj.u8(0xcf) == 1 {
                    sep(&mut out, 10);
                }
                self.sign(&mut out, obj.u8(0xcc), obj.u8(0xcd));
                let zero = n.s(0x1002a7d4);
                let mut pause = true;
                match obj.u32(0x48) {
                    0xc => {
                        let end = out.len();
                        q::convert_integer(&mut out, end, obj.cstr(0x4c), false, n);
                        out.extend_from_slice(n.s(0x1002a900));
                        let end = out.len();
                        q::convert_integer(&mut out, end, obj.cstr(0x8c), false, n);
                        return out;
                    }
                    0xa => {
                        let d = if obj.u8(0xd0) != 0 { zero } else { obj.cstr(0x4c) };
                        let end = out.len();
                        pause = q::convert_integer(&mut out, end, d, false, n);
                    }
                    _ => {
                        let d = if obj.u8(0xd0) != 0 { zero } else { obj.cstr(0x4c) };
                        let end = out.len();
                        q::convert_big_integer(&mut out, end, d, n);
                    }
                }
                if obj.u8(0x8c) != 0 {
                    sep(&mut out, if pause { 0 } else { 1 });
                    sep(&mut out, 4);
                    n.decimals_at(&mut out, obj.cstr(0x8c));
                }
            }
            4 => {
                if matches!(obj.u32(0x48), 5 | 6) {
                    self.ordinal(&mut out, &text);
                }
            }
            7 => {
                self.sign(&mut out, obj.u8(0x8d), obj.u8(0x8e));
                if obj.u32(0x48) == 0x1a {
                    self.duration(&mut out, &text);
                } else {
                    self.time(&mut out, &text, idx);
                }
                // the entry right after the number, by index
                let ni = idx + 1;
                if ni < self.toks.len() && ieq(&self.toks[ni].text, n.s(0x1002a8bc)) {
                    self.toks[ni].text = n.s(0x1002a8b0).to_vec();
                    self.toks[ni].kind = 4;
                }
            }
            8 => match obj.u32(0x88) {
                0 | 8 | 0xe => self.date(&mut out, obj),
                0xc => {
                    out.extend_from_slice(&self.year(obj.u32(0xa4)));
                    sep(&mut out, 1);
                    out.extend_from_slice(n.s(if obj.u8(0x94) == b'&' { 0x1002a8e8 } else { 0x1002a8f0 }));
                    out.extend_from_slice(&self.year(obj.u32(0xa8)));
                }
                0xd => {
                    out.extend_from_slice(&self.year(obj.u32(0xa0)));
                    if out.len() >= 3 {
                        if out.ends_with(n.s(0x1002a8fc)) {
                            out.pop();
                            out.extend_from_slice(n.s(0x1002a8f8));
                        } else if out.last() == Some(&b'x') {
                            out.extend_from_slice(n.s(0x1002a8f4));
                        } else {
                            out.extend_from_slice(n.s(0x1002a3f0));
                        }
                    }
                }
                _ => {}
            },
            9 => {
                self.sign(&mut out, obj.u8(0xcc), obj.u8(0xcd));
                q::convert_integer(&mut out, 0, obj.cstr(0x4c), false, n);
                if obj.u8(0x8c) != 0 {
                    sep(&mut out, 0);
                    sep(&mut out, 4);
                    n.decimals_at(&mut out, obj.cstr(0x8c));
                }
            }
            0xb => {
                self.sign(&mut out, obj.u8(0x4c), obj.u8(0x4d));
                if idx > 0 && digits_only(&self.text(idx - 1)) {
                    sep(&mut out, 0);
                    sep(&mut out, 0xd);
                    sep(&mut out, 0);
                }
                self.fraction(&mut out, &text);
            }
            0xc => {
                self.sign(&mut out, obj.u8(0x56), obj.u8(0x57));
                let cur = obj.cstr(0x4c).to_vec();
                self.money(&mut out, idx, &text, &cur);
            }
            _ => {}
        }
        out
    }
}

impl Numeri<'_> {
    /// Slot 9 (0x1000b540): the reading of number token `idx` for reading
    /// type `kind` and date order `dmy`, and ctx+0x18 (following tokens
    /// used). The module may rewrite later tokens.
    pub(crate) fn leggi(&self, toks: &mut [Tok], idx: usize, kind: u32, dmy: &[u8]) -> (Vec<u8>, usize) {
        let mut c = Call { n: self, toks, c18: 0, df: date_format(self.variant) };
        let text = c.text(idx);
        let next = c.text(c.tok(1, idx));
        let nn = c.text(c.tok(2, idx));
        let prev = c.text(c.tok(-1, idx));
        let mut obj = NumObj::default();
        let mut mode = kind;
        if kind == 0 {
            if next == self.s(0x10026958) {
                mode = 10 + self.fraction(&text) as u32;
            }
        } else if kind == 5 || kind == 6 {
            mode = 4;
            obj.set_u32(0x48, if kind == 5 { 5 } else { 6 });
        }
        let s = |a: u32| self.s(a);
        // the currency words that make the number money (case sensitive)
        let words = [
            0x10029bf8, 0x10029bf0, 0x10029c38, 0x1002a81c, 0x10029bb0, 0x1002a814, 0x1002a93c, 0x1002a938, 0x1002a810, 0x10029b94, 0x1002a804,
            0x10029ae8, 0x1002a7f8, 0x10029b6c, 0x1002a7f4, 0x10029b40, 0x1002a7f0, 0x10029b14, 0x1002a7ec, 0x10029bb0, 0x1002a80c, 0x10029bdc,
            0x1002a808, 0x10029bc4,
        ];
        let (dollar, us) = (s(0x10029c48), s(0x1002a818));
        if q::is_currency(&next)
            || (idx > 0 && q::is_currency(&prev))
            || words.iter().any(|&a| next == s(a))
            || (next == us && nn == dollar)
            || (next == dollar && nn == us)
            || [0x10029c40, 0x1002a934, 0x1002a820].iter().any(|&a| next == s(a))
        {
            mode = 0xc;
        }
        obj.set_u32(0x44, mode);
        obj.set_cstr(4, &text);
        // 0x10009150: the recognizers, the mode's own first
        let mut order: Vec<usize> = (0..9).collect();
        let first = match mode {
            1 => 6,
            2 => 8,
            7 => 2,
            8 => 1,
            9 => 4,
            10 => 7,
            11 => 5,
            12 => 3,
            _ => 0,
        };
        let f = order.remove(first);
        order.insert(0, f);
        if !order.iter().any(|&k| c.recognize(k, idx, dmy, &mut obj)) {
            q::set_default_type(&mut obj, &text);
        }
        let out = c.render(idx, &obj);
        (out, c.c18)
    }
}

/// 0x10009230 on its own (LesElaboraParola's dates, `english/testo.rs`):
/// month, day and year as `order` has them, appended to `out`.
pub(crate) fn format_date(en: &English, out: &mut Vec<u8>, order: &[u8], day: u32, month: u32, year: u32) {
    let n = Numeri::new(&en.pe, en.variant);
    let c = Call { n: &n, toks: &mut [], c18: 0, df: 0 };
    let mut obj = NumObj::default();
    obj.set_u32(0x98, day);
    obj.set_u32(0x9c, month);
    obj.set_u32(0xa0, year);
    obj.set_cstr(0x8c, cs(order));
    c.date(out, &obj);
}

impl English {
    pub(crate) fn leggi_numero(&self, toks: &mut [Tok], i: usize, kind: u32, dmy: &[u8]) -> (Vec<u8>, usize) {
        Numeri::new(&self.pe, self.variant).leggi(toks, i, kind, dmy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trace string with `\"`, `\\`, `\xNN`.
    fn unq(s: &str) -> Vec<u8> {
        let b = s.as_bytes();
        let mut o = Vec::new();
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'\\' && i + 1 < b.len() {
                if b[i + 1] == b'x' {
                    o.push(u8::from_str_radix(&s[i + 2..i + 4], 16).unwrap());
                    i += 4;
                } else {
                    o.push(b[i + 1]);
                    i += 2;
                }
            } else {
                o.push(b[i]);
                i += 1;
            }
        }
        o
    }
    /// The quoted value of `key=` in a trace line.
    fn field(l: &str, key: &str) -> Option<Vec<u8>> {
        let p = l.find(&format!(" {key}=\""))? + key.len() + 3;
        let b = l.as_bytes();
        let mut e = p;
        while b[e] != b'"' {
            e += if b[e] == b'\\' { 2 } else { 1 };
        }
        Some(unq(&l[p..e]))
    }
    /// The number after the last `key=`.
    fn num(l: &str, key: &str) -> i64 {
        let p = l.rfind(&format!("{key}=")).unwrap() + key.len() + 1;
        l[p..].split(|c: char| !c.is_ascii_digit() && c != '-').next().unwrap().parse().unwrap()
    }
    fn toks(lines: &[&str]) -> Vec<Tok> {
        lines
            .iter()
            .map(|l| {
                let f: Vec<&str> = l.split_whitespace().collect();
                Tok { text: field(l, "t").unwrap(), kind: f[2].parse().unwrap(), expanded: f[4] != "0" }
            })
            .collect()
    }

    /// Replays every slot-9 call of the traces in `QQ_EN_NUM_TRACES`
    /// (`<us|gb|au>:<file.les>,…`; default the oracle runs under
    /// /tmp/qq-fon-en-num and /tmp/qq-num-en) on the module's DLL
    /// (`QQ_EN_DLL`): the reading, ctx+0x18 and the tokens after the call.
    #[test]
    #[ignore]
    fn english_numbers_trace() {
        let dll = std::env::var("QQ_EN_DLL").unwrap_or("/tmp/qqv/LTTS7/bin/LTTS7English.dll".into());
        let pe = Pe::parse(std::fs::read(dll).unwrap()).unwrap();
        let spec = std::env::var("QQ_EN_NUM_TRACES").unwrap_or(
            "us:/tmp/qq-fon-en-num/us1.les,gb:/tmp/qq-fon-en-num/gb1.les,au:/tmp/qq-fon-en-num/au1.les,us:/tmp/qq-num-en/bus.les,gb:/tmp/qq-num-en/bgb.les,au:/tmp/qq-num-en/bau.les"
                .into(),
        );
        let (mut total, mut bad) = (0, 0);
        for item in spec.split(',') {
            let (v, path) = item.split_once(':').unwrap();
            let variant = match v {
                "us" => Variant::Us,
                "gb" => Variant::Gb,
                _ => Variant::Au,
            };
            let n = Numeri::new(&pe, variant);
            let text = std::fs::read_to_string(path).unwrap();
            let lines: Vec<&str> = text.lines().collect();
            let mut i = 0;
            while i < lines.len() {
                if !lines[i].starts_with("> NUM") {
                    i += 1;
                    continue;
                }
                let call = lines[i];
                let mut j = i + 1;
                while j < lines.len() && lines[j].starts_with("  T ") {
                    j += 1;
                }
                let mut t = toks(&lines[i + 1..j]);
                let ret = lines[j];
                let mut k = j + 1;
                while k < lines.len() && lines[k].starts_with("  U ") {
                    k += 1;
                }
                let after = toks(&lines[j + 1..k]);
                let idx = num(call, "idx") as usize;
                let kind = num(call, "type") as u32;
                let dmy = field(call, "a3").unwrap_or_default();
                let before = String::from_utf8_lossy(&t[idx].text).into_owned();
                let (r, c18) = n.leggi(&mut t, idx, kind, &dmy);
                let want = field(ret, "ret").unwrap_or_default();
                let wc = num(ret, "c18") as usize;
                // the U lines stop at the first empty text past idx
                let ok_u = t.iter().zip(after.iter()).all(|(a, b)| a.text == b.text && a.kind == b.kind);
                total += 1;
                if r != want || c18 != wc || !ok_u {
                    bad += 1;
                    eprintln!(
                        "{v} {path}: {before:?} kind {kind}: got {:?} c18 {c18}, want {:?} c18 {wc}{}",
                        String::from_utf8_lossy(&r),
                        String::from_utf8_lossy(&want),
                        if ok_u { "" } else { " (tokens differ)" }
                    );
                }
                i = k;
            }
        }
        eprintln!("{bad} of {total} slot-9 calls differ");
        assert_eq!(bad, 0);
    }
}
