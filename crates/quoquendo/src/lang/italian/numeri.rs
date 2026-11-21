//! Italian numbers: `LesIniNumeri` (0x1000b4d0) of LTTS7Italian.dll and
//! every function its slot 9 reaches (MECHANICS.md §4b, "Numbers").
//!
//! Slot 9 (0x1000a810) is the reader the engine calls for a number token: it
//! picks the reading mode (the engine's `kind`; currency when a currency
//! symbol or name stands next to the token), fills the 0xd4-byte number
//! object ([`NumObj`]) with the first of nine recognizers that accepts the
//! token (0x1000a730 orders them by mode), and renders it (0x10009980)
//! through its own code and LTTS7Util's `ELQNum*` (`lang/elqnum.rs`), which
//! call back slots 0–6 ([`NumSlots`]). Words come from the module's word
//! table (slot 0, 0x1002d3d0), every other string from its bytes.
//!
//! Strings are C strings without their NUL. Where the C code writes at a
//! `char *` into the middle of a buffer, the port keeps the whole buffer and
//! the offset of that pointer (`at`), as `elqnum` does.

use super::super::common::{search_compact, Pe, Tok};
use super::super::elqnum::{self as util, NumObj, NumSlots};
use super::Italian;

/// Slot 0's table: 0x53 `{id, char *word}` pairs (0x10005680).
const WORDS: u32 = 0x1002d3d0;
const NWORDS: u32 = 0x53;
/// The empty string the module returns for "nothing".
const EMPTY: u32 = 0x1002c12c;
/// Slot 5's currencies: 18 × `{code, one, many, one cent, cents}`.
const CURRENCIES: u32 = 0x1002d268;
/// `"o"` (masculine ending, the `o` of `ventuno`) and `"a"`.
const O: u32 = 0x10017a8c;
const A: u32 = 0x10018c8c;
/// `"1"`, `"0"`, `"00"`, `"0,00"`, `"39"`, `","`, `"-"`, `".,:"`, `":/-+"`.
const ONE: u32 = 0x1002fc88;
const ZERO: u32 = 0x1002fc98;
const ZERO2: u32 = 0x1002fc90;
const ZERO_CENTS: u32 = 0x1002fc9c;
const ITALY: u32 = 0x1002fc8c;
const COMMA: u32 = 0x1002fc94;
const DASH: u32 = 0x1002fca4;
const TIME_SEPS: u32 = 0x1002fdd0;
const NOT_CARDINAL: u32 = 0x1002fcfc;
/// `"00800"` (the free-phone prefix), `"zero"`, `"primo "`, `"esim"`.
const FREEPHONE: u32 = 0x1002fdd4;
const ZERO_WORD: u32 = 0x1002c140;
const FIRST_DAY: u32 = 0x1002fca8;
const ESIM: u32 = 0x1002fc14;
/// The ordinal marks `°` `º`, `"c"` (`°C`), `"b"` (the order that takes a
/// VoiceXML date).
const DEGREE: u32 = 0x1002fcb4;
const MASC_ORDINAL: u32 = 0x1002fcb0;
const CELSIUS: u32 = 0x100186f8;
const VXML_ORDER: u32 = 0x100186fc;
/// The prefix lists 0x100044a0 searches (`ELQSearchCompactString` mode 2):
/// country codes, free-phone prefixes, Italian area codes, mobile prefixes.
const PREFIXES: [(u32, usize); 4] = [(0x1002ca70, 0x2ea), (0x1002cd5c, 0x17), (0x1002cd78, 0x469), (0x1002d1e8, 0x79)];
/// The ordinals 0x100040a0 spells whole: `{digits, stem}`.
const ORDINALS: [(u32, u32); 9] = [
    (0x1002fc88, 0x1002fc80),
    (0x1002fc7c, 0x1002fc74),
    (0x1002fc70, 0x1002fc68),
    (0x1002fc64, 0x1002fc5c),
    (0x1002fc58, 0x1002fc50),
    (0x1002fc4c, 0x1002fc44),
    (0x1002fc40, 0x1002fc38),
    (0x1002fc34, 0x1002fc2c),
    (0x1002fc24, 0x1002fc1c),
];
/// `"9"`: its stem `non` is an immediate in the code (0x10004349).
const NINE: u32 = 0x1002fc28;
/// The fractions 0x10003db0 knows and 0x10003a90 reads: `{text, reading}`.
const FRACTIONS: [(u32, u32); 14] = [
    (0x1002fc10, 0x1002f8a0),
    (0x1002fc0c, 0x1002fc00),
    (0x1002fbfc, 0x1002fbf0),
    (0x1002fbec, 0x1002fbe0),
    (0x1002fbdc, 0x1002fbd0),
    (0x1002fbcc, 0x1002fbc0),
    (0x1002fbbc, 0x1002fbb0),
    (0x1002fbac, 0x1002fba4),
    (0x1002fb9c, 0x1002fb90),
    (0x1002fb88, 0x1002fb78),
    (0x1002fb70, 0x1002fb60),
    (0x1002fb58, 0x1002fb48),
    (0x1002fb44, 0x1002fb38),
    (0x1002fb34, 0x1002fb28),
];
/// The currency names slot 9 (0x1000a810) looks for after the number.
const CURRENCY_NAMES: [u32; 34] = [
    0x1002c310, 0x1002c350, 0x1002c358, 0x1002c31c, 0x1002c328, 0x1002fcf8, 0x1002fcf4, 0x1002fcec, 0x1002fcf0, 0x1002fce0, 0x1002fcdc,
    0x1002fce8, 0x1002fce4, 0x1002c2f4, 0x10017c70, 0x1002c2e0, 0x1002fcd8, 0x1002c2b4, 0x1002fcd4, 0x1002c288, 0x1002fcd0, 0x1002c26c,
    0x1002fccc, 0x1002c240, 0x1002fcc8, 0x1002c234, 0x1002fcc4, 0x1002c220, 0x1002fcc0, 0x1002c1f8, 0x1002fcbc, 0x1002c1cc, 0x1002fcb8,
    0x1002c308,
];
/// The currency recognizer's (0x10005eb0) names after the number: `{names,
/// symbol or code, the following token becomes "n" (else "")}`.
const CURRENCY_WORDS: [(&[u32], u32, bool); 14] = [
    (&[0x1002c310, 0x1002fcf8, 0x1002fcf4], 0x1002c318, true),
    (&[0x1002c358, 0x1002c350, 0x1002fcf0, 0x1002fcec], 0x1002c360, true),
    (&[0x1002c308, 0x1002fce8, 0x1002fce4], 0, true),
    (&[0x1002c328, 0x1002c31c, 0x1002fce0, 0x1002fcdc], 0, false),
    (&[0x1002c2ec, 0x1002c2e4, 0x1002c2f4, 0x10017c70], 0x1002c2f4, false),
    (&[0x1002c2e0, 0x1002fcd8], 0x1002c2e0, false),
    (&[0x1002c2b4, 0x1002fcd4], 0x1002c2b4, false),
    (&[0x1002c288, 0x1002fcd0], 0x1002c288, false),
    (&[0x1002c26c, 0x1002fccc], 0x1002c26c, false),
    (&[0x1002c240, 0x1002fcc8], 0x1002c240, false),
    (&[0x1002c234, 0x1002fcc4], 0x1002c234, false),
    (&[0x1002c220, 0x1002fcc0], 0x1002c220, false),
    (&[0x1002c1f8, 0x1002fcbc], 0x1002c1f8, false),
    (&[0x1002c1cc, 0x1002fcb8], 0x1002c1cc, false),
];
/// `¥` and `£` (the yen and sterling symbols 0x10005eb0 stores are
/// immediates: 0xa5, 0xa3); the codes that take `una` (0x10004a40).
const YEN: u32 = 0x1002c30c;
const POUND: u32 = 0x1002c334;
const FEMININE: [u32; 4] = [POUND, 0x1002c2f4, 0x1002c288, 0x1002c1f8];

/// The byte at `i` of a C string (its NUL past the end).
fn c(s: &[u8], i: usize) -> u8 {
    s.get(i).copied().unwrap_or(0)
}

fn add_sep(out: &mut Vec<u8>, at: usize, sep: &[u8]) {
    util::add_separator(out, at, sep)
}

/// `ELQltoa`
fn ltoa(v: u32) -> Vec<u8> {
    v.to_string().into_bytes()
}

/// `ELQstricmp(a, b) == 0`
fn stricmp_eq(a: &[u8], b: &[u8]) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// The engine's token buffer as the module reads it: past the last token an
/// empty one of type 7 (the end).
fn tok_at(toks: &[Tok], k: usize) -> (&[u8], u32) {
    toks.get(k).map(|t| (&t.text[..], t.kind)).unwrap_or((&[], 7))
}

/// 0x10002ce0: the token `delta` tokens away from `idx`, tags (type 5) not
/// counted; forwards it stops at the end (type 7) or at token 30. Its text,
/// type and index.
fn tok_rel(toks: &[Tok], idx: usize, delta: i32) -> (Vec<u8>, u32) {
    let mut i = idx;
    let mut d = delta;
    if d < 0 {
        while d < 0 {
            if i == 0 {
                break;
            }
            i -= 1;
            if tok_at(toks, i).1 != 5 {
                d += 1;
            }
        }
    } else if d > 0 {
        let mut u = i + 1;
        while u < 2 * 0xf + 1 && tok_at(toks, i).1 != 7 {
            i += 1;
            u += 1;
            if tok_at(toks, i).1 != 5 {
                d -= 1;
            }
            if d <= 0 {
                break;
            }
        }
    }
    let (t, k) = tok_at(toks, i);
    (t.to_vec(), k)
}

/// What a recognizer works on (the C arguments `ctx, idx, dmy, toks`): the
/// token buffer, the token, the date order and ctx+0x18 (tokens used).
struct Call<'a> {
    toks: &'a mut [Tok],
    idx: usize,
    dmy: &'a [u8],
    used: usize,
}

impl Call<'_> {
    fn set_text(&mut self, k: usize, s: &[u8]) {
        if let Some(t) = self.toks.get_mut(k) {
            t.text = s.to_vec();
        }
    }
}

/// The sign the recognizers strip: the text without it, and which (`-` 1,
/// `+` 2).
fn strip_sign(local: &[u8]) -> (&[u8], u8) {
    match local.first() {
        Some(b'-') => (&local[1..], 1),
        Some(b'+') => (&local[1..], 2),
        _ => (local, 0),
    }
}

fn count(s: &[u8], ch: u8) -> usize {
    s.iter().filter(|&&x| x == ch).count()
}

/// `strchr(s, ',')` split: before and after the first comma.
fn split_comma(s: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let p = s.iter().position(|&x| x == b',').unwrap_or(s.len());
    (s[..p].to_vec(), s.get(p + 1..).unwrap_or(&[]).to_vec())
}

impl Italian {
    fn pe(&self) -> &Pe {
        &self.pe
    }

    /// Slot 0 (0x10005680): the word numbered `id`.
    fn w(&self, id: u32) -> &[u8] {
        (0..NWORDS)
            .find(|&k| self.pe().u32(WORDS + 8 * k) == id)
            .map(|k| self.s(self.pe().u32(WORDS + 8 * k + 4)))
            .unwrap_or_else(|| self.s(EMPTY))
    }

    /// 0x100090c0: the word of the number `n` (1–20, the tens 30–90).
    fn num(&self, n: i32) -> &[u8] {
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
            _ => return self.s(EMPTY),
        };
        self.w(id)
    }

    /// 0x100044a0: the length of the entry of prefix list `sel` that `s`
    /// starts with, 0 when none.
    fn prefix_len(&self, sel: usize, s: &[u8]) -> usize {
        let (va, size) = PREFIXES[sel];
        let blob = &self.pe().at(va)[..size.min(self.pe().at(va).len())];
        let key = &s[..s.len().min(0x3f)];
        match search_compact(key, blob, 2, 0) {
            Some(off) => blob[off..].iter().position(|&x| x == 0).unwrap_or(blob.len() - off),
            None => 0,
        }
    }

    /// 0x100092e0: the hundreds digit `d[0]` (nothing for `0`), a space
    /// after but for `cento`. False for `0`.
    fn hundreds(&self, o: &mut Vec<u8>, d: &[u8]) -> bool {
        let ch = c(d, 0);
        if ch == b'0' {
            return false;
        }
        match ch {
            b'1' => {
                let id = if c(d, 1) == b'0' && c(d, 2) == b'0' { 0x3a } else { 0x3b };
                o.extend_from_slice(self.w(id));
            }
            b'2' => o.extend_from_slice(self.w(0x3e)),
            b'3' => o.extend_from_slice(self.w(0x3f)),
            b'5' => o.extend_from_slice(self.w(0x41)),
            b'7' => {
                o.extend_from_slice(self.w(0x1e));
                o.extend_from_slice(self.w(0x3c));
            }
            b'9' => {
                o.extend_from_slice(self.w(0x21));
                o.extend_from_slice(self.w(0x3c));
            }
            _ => {
                o.extend_from_slice(self.num(ch as i32 - 0x30));
                o.extend_from_slice(self.w(0x3c));
            }
        }
        if ch != b'1' {
            add_sep(o, 0, self.w(1));
        }
        true
    }

    /// 0x10009560: the two digits `d[0..2]` (`ventuno` without its `o`).
    fn tens(&self, d: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        let two = [c(d, 0), if c(d, 0) == 0 { 0 } else { c(d, 1) }];
        let n = util::atol(&two);
        if n == 0 {
            return b;
        }
        let (d0, d1) = (two[0], two[1]);
        if d0 == b'1' || n < 10 {
            b.extend_from_slice(self.num(n));
            return b;
        }
        b.extend_from_slice(self.num((d0 as i32 - 0x30) * 10));
        let u = self.num(d1 as i32 - 0x30);
        let (zero, glide, plain) = if d0 == b'2' { (0x2e, 0x30, 0x2f) } else { (0x31, 0x33, 0x32) };
        if d1 == b'0' {
            b.extend_from_slice(self.w(zero));
        } else if d1 == b'1' || d1 == b'8' {
            b.extend_from_slice(self.w(glide));
            b.extend_from_slice(u);
        } else {
            b.extend_from_slice(self.w(plain));
            b.extend_from_slice(u);
        }
        b
    }

    /// Slot 2 (0x1000a0b0): the cardinal of up to 12 digits, groups of
    /// thousands, millions and billions separated by word 0 (` # `), which
    /// is dropped at the end.
    fn cardinal_c(&self, d: &[u8]) -> Vec<u8> {
        let mut o = Vec::new();
        let len = d.len();
        let mut p = 0usize;
        let (mut f1a, mut f1b, mut f13, mut f11, mut f12) = (false, false, false, false, true);
        let last_is_space = |o: &Vec<u8>| o.last() == Some(&b' ');
        if len >= 12 {
            self.hundreds(&mut o, &d[p..]);
            p += 1;
        }
        if len >= 11 {
            f11 = true;
            o.extend(self.tens(&d[p..]));
            p += 1;
        }
        if len >= 10 {
            let ch = c(d, p);
            if ch == b'1' && !f11 {
                o.extend_from_slice(self.w(0x4d));
                add_sep(&mut o, 0, self.w(0));
            } else {
                if ch != b'1' && !f11 {
                    o.extend_from_slice(self.num(ch as i32 - 0x30));
                }
                if !last_is_space(&o) {
                    add_sep(&mut o, 0, self.w(1));
                }
                o.extend_from_slice(self.w(0x4e));
                add_sep(&mut o, 0, self.w(0));
            }
            p += 1;
            f11 = false;
            f12 = true;
        }
        if len >= 9 {
            f12 = !self.hundreds(&mut o, &d[p..]);
            p += 1;
        }
        if len >= 8 {
            f11 = true;
            if f12 && c(d, p) == b'0' && c(d, p + 1) == b'1' {
                f1a = true;
            } else {
                o.extend(self.tens(&d[p..]));
            }
            if c(d, p) != b'0' {
                f12 = false;
            }
            p += 1;
        }
        if len >= 7 {
            let ch = c(d, p);
            if !(ch == b'0' && f12) {
                let one = if f1a {
                    true
                } else if ch == b'1' {
                    !f11
                } else {
                    if !f11 {
                        o.extend_from_slice(self.num(ch as i32 - 0x30));
                    }
                    false
                };
                if one {
                    o.extend_from_slice(self.w(0x4a));
                } else {
                    if !last_is_space(&o) {
                        add_sep(&mut o, 0, self.w(1));
                    }
                    o.extend_from_slice(self.w(0x4b));
                }
                add_sep(&mut o, 0, self.w(0));
            }
            p += 1;
            f11 = false;
            f12 = true;
        }
        if len >= 6 {
            f13 = true;
            f12 = !self.hundreds(&mut o, &d[p..]);
            p += 1;
        }
        if len >= 5 {
            f11 = true;
            if f12 && c(d, p) == b'0' && c(d, p + 1) == b'1' {
                f1b = true;
            } else {
                o.extend(self.tens(&d[p..]));
            }
            if c(d, p) != b'0' {
                f12 = false;
            }
            p += 1;
        }
        if len >= 4 {
            let ch = c(d, p);
            if ch == b'0' && f12 {
            } else if ch == b'1' && !f11 && !f13 {
                // `mille`, a space unless `millecento`, `milleuno`…
                let bl = c(d, p + 1) != b'0';
                f1a = c(d, p + 2) != b'0';
                f13 = c(d, p + 3) != b'0';
                o.extend_from_slice(self.w(0x47));
                if (bl || (f1a && f13)) && !(!f1a && !f13 && c(d, p + 1) == b'1') {
                    add_sep(&mut o, 0, self.w(1));
                }
            } else {
                if !f11 {
                    o.extend_from_slice(self.num(ch as i32 - 0x30));
                }
                o.extend_from_slice(self.w(if f1b { 0x47 } else { 0x48 }));
                let sep = if !f13 || f1b { 1 } else { 0 };
                add_sep(&mut o, 0, self.w(sep));
            }
            p += 1;
        }
        if len >= 3 {
            self.hundreds(&mut o, &d[p..]);
            p += 1;
        }
        if len >= 2 {
            o.extend(self.tens(&d[p..]));
            if c(d, p + 1) == b'1' && c(d, p) != b'1' {
                o.extend_from_slice(self.s(O));
            }
        }
        let w0 = self.w(0);
        if o.len() >= w0.len() && o.ends_with(w0) {
            o.truncate(o.len() - w0.len());
        }
        o
    }

    /// Slot 7 (0x100056b0): `ELQNumExtractDate` in the order `dmy`, then its
    /// checks (two-digit years from 80 in the 1900s unless the date is
    /// written with `/`).
    fn date(&self, dmy: &[u8], s: &[u8], date: &mut util::Date) -> bool {
        if !util::extract_date(dmy, s, date) {
            return false;
        }
        let min = if date.sep == b'/' { 0 } else { 0x50 };
        util::validate_date(dmy, date, min)
    }

    // --- the recognizers (0x1000a730's table) ------------------------------

    /// 0x100058a0: a date (kind 8), in the order `dmy` or a VoiceXML
    /// `YYYYMMDD`.
    fn r_date(&self, call: &mut Call, obj: &mut NumObj) -> bool {
        obj.set_cstr(0x8c, call.dmy);
        let local = obj.text().to_vec();
        obj.set_u32(0, 8);
        obj.set_u32(0x88, 8);
        obj.set_u8(0x94, 0);
        obj.set_u8(0x48, 0);
        let (body, _) = strip_sign(&local);
        obj.set_cstr(4, body);
        let mode = obj.mode();
        if mode == 0 || mode == 8 {
            let mut date = util::Date::default();
            let ok = self.date(call.dmy, body, &mut date);
            obj.set_u32(0x98, date.day);
            obj.set_u32(0x9c, date.month);
            obj.set_u32(0xa0, date.year);
            if ok {
                return true;
            }
            if mode == 8 {
                let ok = util::format_vxml10_date(body, &mut date);
                obj.set_u32(0x98, date.day);
                obj.set_u32(0x9c, date.month);
                obj.set_u32(0xa0, date.year);
                if ok && stricmp_eq(call.dmy, self.s(VXML_ORDER)) {
                    return true;
                }
            }
        }
        util::set_default_type(obj, &local);
        false
    }

    /// 0x10005a00: a time (kind 7), `h.mm`, `h:mm[:ss]`, or a time with
    /// decimals after a comma.
    fn r_time(&self, _call: &mut Call, obj: &mut NumObj) -> bool {
        let local = obj.text().to_vec();
        obj.set_u8(0x4c, 0);
        obj.set_u32(0, 7);
        obj.set_u32(0x48, 7);
        for o in 0x8c..0x90 {
            obj.set_u8(o, 0);
        }
        let (body, sign) = strip_sign(&local);
        match sign {
            1 => obj.set_u8(0x8d, 1),
            2 => obj.set_u8(0x8e, 1),
            _ => {}
        }
        obj.set_cstr(4, body);
        let body = body.to_vec();
        let (dots, commas) = (count(&body, b'.'), count(&body, b','));
        let signed = obj.u8(0x8d) != 0 || obj.u8(0x8e) != 0;
        let mode = obj.mode();
        let ok = if dots != 0 {
            commas == 0 && dots < 3 && (mode == 0 || mode == 7) && !signed && mode != 10 && util::is_time(&body)
        } else if commas == 0 {
            matches!(mode, 7 | 8 | 0) && !signed && util::is_time(&body)
        } else if commas == 1 {
            let (l58, l98) = split_comma(&body);
            if !(mode == 0 || mode == 7 || mode == 8) || obj.u8(0x8d) != 0 {
                false
            } else if obj.u8(0x8e) == 0 && util::is_time(&body) {
                true
            } else if signed {
                false
            } else if util::is_time(&l58) {
                obj.set_cstr(4, &l58);
                obj.set_u8(0x8c, 1);
                obj.set_cstr(0x4c, &l98);
                true
            } else {
                false
            }
        } else {
            false
        };
        if !ok {
            util::set_default_type(obj, &local);
        }
        ok
    }

    /// 0x10005cc0: an ordinal (kind 4) when the mode says so or `°`/`º`
    /// follow (not `°C`), or an apostrophe and no number after.
    fn r_ordinal(&self, call: &mut Call, obj: &mut NumObj) -> bool {
        let local = obj.text().to_vec();
        obj.set_u32(0, 4);
        let (body, _) = strip_sign(&local);
        obj.set_cstr(4, body);
        if count(body, b'.') + count(body, b',') != 0 {
            util::set_default_type(obj, &local);
            return false;
        }
        if obj.mode() == 4 {
            return true;
        }
        let (next, _) = tok_rel(call.toks, call.idx, 1);
        let (t2, k2) = tok_rel(call.toks, call.idx, 2);
        let mark = next == self.s(DEGREE) || next == self.s(MASC_ORDINAL);
        if mark && !stricmp_eq(&t2, self.s(CELSIUS)) {
            obj.set_u32(0x48, 5);
            return true;
        }
        if c(&next, 0) == b'\'' && k2 != 1 {
            obj.set_u32(0x48, 5);
            return true;
        }
        util::set_default_type(obj, &local);
        false
    }

    /// 0x10005eb0: an amount of money (kind 12): a currency symbol before
    /// or after, or a currency name or code after (that token then used).
    fn r_currency(&self, call: &mut Call, obj: &mut NumObj) -> bool {
        obj.set_u32(0, 0xc);
        obj.set_u32(0x48, 0xf);
        for o in [0x56, 0x57, 0x58, 0x5a] {
            obj.set_u8(o, 0);
        }
        obj.set_u8(0x4c, 0);
        let local = obj.text().to_vec();
        let (body, sign) = strip_sign(&local);
        match sign {
            1 => obj.set_u8(0x56, 1),
            2 => obj.set_u8(0x57, 1),
            _ => {}
        }
        obj.set_cstr(4, body);
        let (mut dots, mut commas, mut last_dot, mut last_comma) = (0, 0, 0, 0);
        for (i, &ch) in body.iter().enumerate() {
            match ch {
                b'-' => (dots, commas, last_dot, last_comma) = (0, 0, 0, 0),
                b'.' => (dots, last_dot) = (dots + 1, i),
                b',' => (commas, last_comma) = (commas + 1, i),
                _ => {}
            }
        }
        if dots == 1 {
            if commas == 0 {
                if body.len() - last_dot != 4 {
                    obj.set_u8(0x5a, 1);
                }
            } else if last_comma < last_dot {
                obj.set_u8(0x5a, 1);
            }
        }
        let idx = call.idx;
        if idx > 0 {
            let prev = tok_at(call.toks, idx - 1).0.to_vec();
            if util::is_currency(&prev) {
                obj.set_cstr(0x4c, &prev);
                return true;
            }
        }
        let next = tok_at(call.toks, idx + 1).0.to_vec();
        if util::is_currency(&next) {
            obj.set_cstr(0x4c, &next);
            call.set_text(idx + 1, b"n");
            call.used = 1;
            return true;
        }
        for (names, code, n) in CURRENCY_WORDS {
            if names.iter().any(|&a| next == self.s(a)) {
                let code: Vec<u8> = match code {
                    0 if n => vec![0xa5],
                    0 => vec![0xa3],
                    a => self.s(a).to_vec(),
                };
                obj.set_cstr(0x4c, &code);
                call.set_text(idx + 1, if n { b"n" } else { b"" });
                call.used = 1;
                return true;
            }
        }
        util::set_default_type(obj, &local);
        false
    }

    /// 0x10006990: a number with decimals read as such (kind 9), only in
    /// mode 9.
    fn r_decimal(&self, _call: &mut Call, obj: &mut NumObj) -> bool {
        obj.set_u8(0x8c, 0);
        let local = obj.text().to_vec();
        obj.set_u32(0, 9);
        obj.set_u32(0x48, 9);
        obj.set_u8(0x4c, 0);
        obj.set_u8(0xcc, 0);
        obj.set_u8(0xcd, 0);
        let (body, sign) = strip_sign(&local);
        match sign {
            1 => obj.set_u8(0xcc, 1),
            2 => obj.set_u8(0xcd, 1),
            _ => {}
        }
        obj.set_cstr(4, body);
        let body = body.to_vec();
        let (dots, commas) = (count(&body, b'.'), count(&body, b','));
        let mode = obj.mode();
        let ok = if dots == 0 {
            if commas == 0 {
                if mode == 9 {
                    obj.set_cstr(0x4c, &body);
                }
                mode == 9
            } else if commas == 1 {
                if mode == 9 {
                    obj.set_cstr(0x4c, &split_comma(&body).1);
                }
                if obj.u8(0x4c) != 0 {
                    // la0, never filled
                    obj.set_u8(0x8c, 0);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else if commas == 0 {
            if (mode == 0 || mode == 3) && util::is_big_integer(&body) {
                obj.set_u32(0x48, 3);
                false
            } else if mode == 9 && util::is_big_integer(&body) {
                obj.set_cstr(0x4c, &body);
                true
            } else {
                false
            }
        } else if commas == 1 {
            let p = body.iter().rposition(|&x| x == b'.');
            let q = body.iter().rposition(|&x| x == b',');
            match (p, q) {
                (Some(p), Some(q)) if p < q => {
                    let units = &body[..q];
                    if util::is_big_integer(units) && mode == 9 {
                        obj.set_cstr(0x4c, units);
                        obj.set_u32(0x48, 3);
                        obj.set_cstr(0x8c, &body[q + 1..]);
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            }
        } else {
            false
        };
        if !ok {
            util::set_default_type(obj, &local);
        }
        ok
    }

    /// 0x10006c40: a fraction it knows (kind 11, `1/2`, `3/4`…).
    fn r_fraction(&self, _call: &mut Call, obj: &mut NumObj) -> bool {
        let local = obj.text().to_vec();
        obj.set_u32(0, 0xb);
        obj.set_u32(0x48, 0xb);
        obj.set_u8(0x4c, 0);
        obj.set_u8(0x4d, 0);
        let (body, sign) = strip_sign(&local);
        match sign {
            1 => obj.set_u8(0x4c, 1),
            2 => obj.set_u8(0x4d, 1),
            _ => {}
        }
        obj.set_cstr(4, body);
        let ok = count(body, b'.') + count(body, b',') == 0
            && matches!(obj.mode(), 8 | 0 | 0xb)
            && FRACTIONS.iter().any(|&(k, _)| body == self.s(k));
        if !ok {
            util::set_default_type(obj, &local);
        }
        ok
    }

    /// 0x10009770: a telephone number (kind 1).
    fn r_phone(&self, call: &mut Call, obj: &mut NumObj) -> bool {
        obj.set_u8(0x4d, 0);
        let local = obj.text().to_vec();
        obj.set_u32(0, 1);
        obj.set_u32(0x48, 1);
        obj.set_u8(0x4c, 0);
        let body: &[u8] = if local.first() == Some(&b'-') { &local[1..] } else { &local };
        obj.set_cstr(4, body);
        let body = body.to_vec();
        let (dots, commas) = (count(&body, b'.'), count(&body, b','));
        let mode = obj.mode();
        let ok = if dots != 0 {
            false
        } else if commas == 0 {
            if mode == 0 {
                if self.phone(call, obj) {
                    if let Some(t) = call.toks.first_mut() {
                        t.kind = 1;
                    }
                    true
                } else {
                    false
                }
            } else if mode == 1 {
                obj.set_u32(0x48, 8);
                true
            } else {
                false
            }
        } else if commas == 1 {
            let (l64, la4) = split_comma(&body);
            if mode == 1 && self.prefix_len(2, &l64) != 0 {
                obj.set_cstr(4, &l64);
                obj.set_u8(0x4c, 1);
                obj.set_cstr(0x4d, &la4);
                true
            } else {
                false
            }
        } else {
            false
        };
        if !ok {
            util::set_default_type(obj, &local);
        }
        ok
    }

    /// 0x10008360: whether the digits (and the digit groups after them,
    /// which it joins in) make a telephone number: an international prefix
    /// (`00`, `+`) and country code, a free-phone prefix, an area code or a
    /// mobile prefix, then enough digits. Its type in +0x48.
    fn phone(&self, call: &mut Call, obj: &mut NumObj) -> bool {
        let idx = call.idx;
        let mut pos = 0usize;
        let mut used = 0usize; // [esp+0x14]
        let mut n18 = 1i32;
        let mut n10 = 2i32;
        let mut state = 0;
        let text = |obj: &NumObj| obj.text().to_vec();
        // appends a following token: the text's length before is the new pos
        let join = |obj: &mut NumObj, pos: &mut usize, s: &[u8]| {
            let mut t = obj.text().to_vec();
            *pos = t.len();
            t.extend_from_slice(s);
            obj.set_cstr(4, &t);
        };
        let dash = self.s(DASH).to_vec();
        loop {
            match state {
                0 => {
                    let t = text(obj);
                    let r = self.prefix_len(1, &t);
                    if r != 0 {
                        pos += r;
                        state = 20;
                        obj.set_u32(0x48, 6);
                    } else if c(&t, 0) == b'0' && c(&t, 1) == b'0' {
                        pos += 2;
                        state = 10;
                    } else if c(&t, 0) == b'+' {
                        pos += 1;
                        state = 10;
                    } else {
                        state = 50;
                    }
                }
                10 => {
                    let t = text(obj);
                    let r = self.prefix_len(0, &t[pos.min(t.len())..]);
                    if r == 0 {
                        return false;
                    }
                    if r == 2 && c(&t, pos) == b'3' && c(&t, pos + 1) == b'9' {
                        pos += 2;
                        state = 40;
                        obj.set_u32(0x48, 3);
                    } else {
                        pos += r;
                        state = 30;
                        obj.set_u32(0x48, 2);
                    }
                }
                20 | 30 | 40 | 60 | 70 => {
                    let ch = c(&text(obj), pos);
                    if ch == 0 {
                        state += 1;
                    } else {
                        if ch == b'-' {
                            pos += 1;
                        }
                        state = if state == 40 { 50 } else { state + 2 };
                    }
                }
                21 | 31 | 41 => {
                    let (also, digits_only): (&[u8], bool) = if state == 21 { (&[], true) } else { (&dash, false) };
                    let next = if state == 41 { 50 } else { state + 1 };
                    let (ta, _) = tok_rel(call.toks, idx, 1);
                    let (tb, _) = tok_rel(call.toks, idx, 2);
                    if util::is_number(&ta, also, digits_only) {
                        join(obj, &mut pos, &ta);
                        used += 1;
                        n18 += 1;
                        n10 += 1;
                    } else {
                        if ta != dash || !util::is_number(&tb, also, digits_only) {
                            return false;
                        }
                        join(obj, &mut pos, &tb);
                        used += 2;
                        n18 += 2;
                        n10 += 2;
                    }
                    state = next;
                }
                22 => {
                    let t = text(obj);
                    let head = &t[..pos.min(t.len())];
                    let rest = t.len().saturating_sub(pos);
                    if !((head == self.s(FREEPHONE) && rest == 8) || rest == 6) {
                        return false;
                    }
                    obj.set_u32(0x48, 6);
                    break;
                }
                32 => {
                    if text(obj).len().saturating_sub(pos) < 4 {
                        return false;
                    }
                    break;
                }
                50 => {
                    let t = text(obj);
                    let s = &t[pos.min(t.len())..];
                    let r = self.prefix_len(2, s);
                    if r != 0 {
                        pos += r;
                        state = 60;
                        if obj.u32(0x48) == 3 {
                            obj.set_u32(0x48, 4);
                        }
                    } else {
                        let r = self.prefix_len(3, s);
                        if r == 0 {
                            return false;
                        }
                        pos += r;
                        let v = if obj.u32(0x48) != 3 { 7 } else { 5 };
                        obj.set_u32(0x48, v);
                        state = 70;
                    }
                }
                61 => {
                    let ta = tok_at(call.toks, idx + 1 + used).0.to_vec();
                    if util::is_number(&ta, &dash, false) {
                        join(obj, &mut pos, &ta);
                        used += 1;
                        n18 += 1;
                        n10 += 1;
                    } else {
                        let tb = tok_at(call.toks, used + idx + 2).0.to_vec();
                        if ta != dash || !util::is_number(&tb, &dash, false) {
                            return false;
                        }
                        join(obj, &mut pos, &tb);
                        used += 2;
                        n18 += 2;
                        n10 += 2;
                    }
                    state = 62;
                }
                71 => {
                    let (ta, _) = tok_rel(call.toks, idx, n18);
                    if util::is_number(&ta, &dash, false) {
                        join(obj, &mut pos, &ta);
                        used += 1;
                        n18 += 1;
                        n10 += 1;
                    } else {
                        let (tb, _) = tok_rel(call.toks, idx, n10);
                        if ta != dash || !util::is_number(&tb, &dash, false) {
                            return false;
                        }
                        join(obj, &mut pos, &tb);
                        used += 2;
                        n18 += 2;
                        n10 += 2;
                    }
                    state = 72;
                }
                62 | 72 => {
                    let t = text(obj);
                    let s = &t[pos.min(t.len())..];
                    let (lo, hi) = if state == 62 { (4, 9) } else { (5, 7) };
                    if s.contains(&b'-') || s.len() < lo || s.len() > hi {
                        return false;
                    }
                    break;
                }
                _ => return false,
            }
        }
        let t: Vec<u8> = text(obj).into_iter().filter(|&x| x != b'-').collect();
        obj.set_cstr(4, &t);
        call.used = used;
        true
    }

    /// 0x10006f30: a cardinal or a number with decimals (kind 10; +0x48 3
    /// for digits grouped with dots).
    fn r_cardinal(&self, _call: &mut Call, obj: &mut NumObj) -> bool {
        obj.set_u8(0x4c, 0);
        obj.set_u8(0x8c, 0);
        let local = obj.text().to_vec();
        obj.set_u32(0, 0xa);
        obj.set_u32(0x48, 0xa);
        for o in [0xcc, 0xcd, 0xd0] {
            obj.set_u8(o, 0);
        }
        let (body, sign) = strip_sign(&local);
        match sign {
            1 => obj.set_u8(0xcc, 1),
            2 => obj.set_u8(0xcd, 1),
            _ => {}
        }
        obj.set_cstr(4, body);
        let body = body.to_vec();
        let (dots, commas) = (count(&body, b'.'), count(&body, b','));
        let mode = obj.mode();
        let plain = |b: &[u8]| !b.iter().any(|x| self.s(NOT_CARDINAL).contains(x));
        let ok = if dots == 0 && commas == 0 {
            match mode {
                3 | 10 => {
                    obj.set_cstr(0x4c, &body);
                    true
                }
                0 | 8 if plain(&body) && body.len() < 6 => {
                    obj.set_cstr(0x4c, &body);
                    true
                }
                _ => false,
            }
        } else if dots == 0 && commas == 1 {
            let (l5c, l9c) = split_comma(&body);
            if matches!(mode, 0 | 7 | 8 | 10) && plain(&body) {
                obj.set_cstr(0x4c, &l5c);
                obj.set_cstr(0x8c, &l9c);
                true
            } else {
                false
            }
        } else if dots == 0 {
            false
        } else if commas == 0 {
            if matches!(mode, 0 | 3 | 10) && util::is_big_integer(&body) {
                obj.set_cstr(0x4c, &body);
                obj.set_u32(0x48, 3);
                true
            } else {
                false
            }
        } else if commas == 1 {
            let p = body.iter().rposition(|&x| x == b'.');
            let q = body.iter().rposition(|&x| x == b',');
            match (p, q) {
                (Some(p), Some(q)) if p < q => {
                    let units = &body[..q];
                    let dec = &body[q + 1..];
                    if util::is_big_integer(units) && ((mode == 0 && dec.len() <= 2) || mode == 3) {
                        obj.set_cstr(0x4c, units);
                        obj.set_cstr(0x8c, dec);
                        obj.set_u32(0x48, 3);
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            }
        } else {
            false
        };
        if !ok {
            util::set_default_type(obj, &local);
        }
        ok
    }

    /// 0x10006d60: a code read digit by digit (kind 2), only in mode 2.
    fn r_digits(&self, _call: &mut Call, obj: &mut NumObj) -> bool {
        let local = obj.text().to_vec();
        obj.set_u32(0, 2);
        obj.set_u32(0x48, 2);
        for o in [0x8c, 0x4c, 0x8d, 0x8e] {
            obj.set_u8(o, 0);
        }
        let (body, sign) = strip_sign(&local);
        match sign {
            1 => obj.set_u8(0x8d, 1),
            2 => obj.set_u8(0x8e, 1),
            _ => {}
        }
        obj.set_cstr(4, body);
        let body = body.to_vec();
        let (dots, commas) = (count(&body, b'.'), count(&body, b','));
        if dots == 0 && commas == 0 && obj.mode() == 2 {
            return true;
        }
        if dots == 0 && commas == 1 && obj.mode() == 2 {
            // the original fills the parts and then gives up all the same
            let (l54, l94) = split_comma(&body);
            obj.set_cstr(4, &l54);
            obj.set_u8(0x8c, 1);
            obj.set_cstr(0x4c, &l94);
        }
        util::set_default_type(obj, &local);
        false
    }

    // --- the renderer (0x10009980) and its readers ---------------------------

    fn sign_words(&self, out: &mut Vec<u8>, obj: &NumObj, minus: usize, plus: usize) {
        if obj.u8(minus) == 1 {
            add_sep(out, 0, self.w(7));
        } else if obj.u8(plus) == 1 {
            add_sep(out, 0, self.w(8));
        }
    }

    /// Word 0 or 1 (the pause flag), word 4 (`virgola`) and the decimals
    /// (slot 6).
    fn decimals_after(&self, out: &mut Vec<u8>, pause: bool, dec: &[u8], flag: bool) {
        add_sep(out, 0, self.w(if pause { 0 } else { 1 }));
        add_sep(out, 0, self.w(4));
        self.decimals(out, dec, flag);
    }

    fn render(&self, obj: &NumObj, out: &mut Vec<u8>) {
        out.clear();
        match obj.kind() {
            0 => util::convert_real_number(out, 0, obj, self),
            1 => {
                self.phone_render(out, obj);
                if obj.u8(0x4d) != 0 {
                    let mut o2 = NumObj::default();
                    o2.set_cstr(4, obj.cstr(0x4d));
                    o2.set_u32(0x44, obj.mode());
                    let at = out.len();
                    util::convert_real_number(out, at, &o2, self);
                }
            }
            2 => {
                self.sign_words(out, obj, 0x8d, 0x8e);
                let pause = util::convert_integer(out, 0, obj.text(), true, self);
                if obj.u8(0x4c) != 0 {
                    self.decimals_after(out, pause, obj.cstr(0x4c), true);
                }
            }
            3 | 10 => {
                self.sign_words(out, obj, 0xcc, 0xcd);
                let at = out.len();
                let pause = if obj.u32(0x48) == 10 {
                    util::convert_integer(out, at, obj.cstr(0x4c), false, self)
                } else {
                    util::convert_big_integer(out, at, obj.cstr(0x4c), self);
                    true
                };
                if obj.u8(0x8c) != 0 {
                    self.decimals_after(out, pause, obj.cstr(0x8c), false);
                }
            }
            4 => match obj.u32(0x48) {
                5 => self.ordinal(out, obj.text(), true),
                6 => self.ordinal(out, obj.text(), false),
                _ => {}
            },
            7 => {
                self.sign_words(out, obj, 0x8d, 0x8e);
                self.time_render(out, obj.text());
                if obj.u8(0x4c) != 0 {
                    // word 0 always (0x10009a78)
                    self.decimals_after(out, true, obj.cstr(0x4c), false);
                }
            }
            8 => {
                if obj.u32(0x88) == 8 {
                    self.date_render(out, obj.u32(0x98), obj.u32(0x9c), obj.u32(0xa0));
                }
            }
            9 => {
                self.sign_words(out, obj, 0xcc, 0xcd);
                let pause = util::convert_integer(out, 0, obj.cstr(0x4c), false, self);
                if obj.u8(0x8c) != 0 {
                    self.decimals_after(out, pause, obj.cstr(0x8c), false);
                }
            }
            0xb => {
                let text = obj.text();
                if let Some(&(_, r)) = FRACTIONS.iter().find(|&&(k, _)| text == self.s(k)) {
                    *out = self.s(r).to_vec();
                }
            }
            0xc => {
                self.sign_words(out, obj, 0x56, 0x57);
                let mut amount = obj.text().to_vec();
                if obj.u8(0x5a) != 0 {
                    for ch in amount.iter_mut() {
                        *ch = match *ch {
                            b',' => b'.',
                            b'.' => b',',
                            x => x,
                        };
                    }
                }
                self.currency_render(out, &amount, obj.cstr(0x4c));
            }
            _ => {}
        }
    }

    /// 0x100040a0: the ordinal of `d` (`primo`, `ventitreesimo`), `o` or
    /// `a` at the end.
    fn ordinal(&self, out: &mut Vec<u8>, d: &[u8], masc: bool) {
        let z = d.iter().take_while(|&&x| x == b'0').count();
        let d = &d[z..];
        if let Some(&(_, w)) = ORDINALS.iter().find(|&&(k, _)| d == self.s(k)) {
            out.clear();
            out.extend_from_slice(self.s(w));
        } else if d == self.s(NINE) {
            out.clear();
            out.extend_from_slice(b"non");
        } else {
            util::convert_integer(out, 0, d, false, self);
            if out.is_empty() {
                return;
            }
            while out.last() == Some(&b' ') {
                out.pop();
            }
            let l = d.len();
            if l > 1 && (d[l - 1] == b'3' || d[l - 1] == b'6') && d[l - 2] != b'1' {
                out.extend_from_slice(self.s(ESIM));
            } else {
                out.pop();
                out.extend_from_slice(self.s(ESIM));
            }
        }
        out.extend_from_slice(self.s(if masc { O } else { A }));
    }

    /// 0x10008020: a time, `h`, `h e mm`, `h # mm minuti # e ss secondi`.
    fn time_render(&self, out: &mut Vec<u8>, text: &[u8]) {
        let seps = self.s(TIME_SEPS).to_vec();
        let local = text.to_vec();
        let Some(p) = local.iter().position(|x| seps.contains(x)) else { return };
        let hours = &local[..p];
        let h: &[u8] = if c(hours, 0) == b'0' && (b'1'..=b'9').contains(&c(hours, 1)) { &hours[1..] } else { hours };
        if util::atol(h) == 1 {
            out.clear();
            out.extend_from_slice(b"una");
        } else {
            util::convert_integer(out, 0, h, false, self);
        }
        let after = &local[p + 1..];
        let e = |out: &mut Vec<u8>| {
            add_sep(out, 0, self.w(1));
            add_sep(out, 0, self.w(0xd));
            add_sep(out, 0, self.w(1));
        };
        let int = |out: &mut Vec<u8>, s: &[u8]| {
            let at = out.len();
            util::convert_integer(out, at, s, false, self);
        };
        match after.iter().position(|x| seps.contains(x)) {
            Some(q) => {
                let m = &after[..q];
                let s = &after[q + 1..];
                let full = c(m, 0) != 0
                    && !(c(m, 0) == b'0' && c(m, 1) == b'0')
                    && c(s, 0) != 0
                    && !(c(s, 0) == b'0' && c(s, 1) == b'0');
                if full {
                    add_sep(out, 0, self.w(0));
                    add_sep(out, 0, self.w(1));
                    int(out, m);
                    add_sep(out, 0, self.w(0x5e));
                    add_sep(out, 0, self.w(0));
                    add_sep(out, 0, self.w(1));
                    add_sep(out, 0, self.w(0xd));
                    add_sep(out, 0, self.w(1));
                    int(out, s);
                    add_sep(out, 0, self.w(0x5d));
                } else if c(m, 0) == b'0' && c(m, 1) == b'0' {
                    if !(c(s, 0) == b'0' && c(s, 1) == b'0') {
                        e(out);
                        int(out, s);
                        add_sep(out, 0, self.w(0x5d));
                    }
                } else if c(s, 0) == b'0' && c(s, 1) == b'0' {
                    e(out);
                    int(out, m);
                }
            }
            None => {
                let m = after;
                if c(m, 0) == 0 || (c(m, 0) == b'0' && c(m, 1) == b'0') {
                    return;
                }
                e(out);
                let mut rest = m;
                if c(m, 2) == b'-' {
                    int(out, &[c(m, 0), c(m, 1)]);
                    add_sep(out, 0, self.w(0));
                    rest = &m[3..];
                }
                int(out, rest);
            }
        }
    }

    /// 0x10005720: a date, `primo maggio duemila dieci`.
    fn date_render(&self, out: &mut Vec<u8>, day: u32, month: u32, year: u32) {
        if day > 0 {
            if day == 1 {
                out.extend_from_slice(self.s(FIRST_DAY));
            } else {
                util::convert_integer(out, 0, &ltoa(day), false, self);
            }
            add_sep(out, 0, self.w(1));
        }
        if (1..=12).contains(&month) {
            out.extend_from_slice(self.w(0x50 + month));
            add_sep(out, 0, self.w(1));
        }
        if year > 0 {
            let at = out.len();
            util::convert_integer(out, at, &ltoa(year), false, self);
        }
    }

    /// 0x10004550: a telephone number by its type (+0x48): international,
    /// Italian (`00 39`), free phone, fixed line (area code), mobile.
    fn phone_render(&self, out: &mut Vec<u8>, obj: &NumObj) {
        let t = obj.text().to_vec();
        let int = |out: &mut Vec<u8>, s: &[u8], flag: bool| {
            let at = out.len();
            util::convert_integer(out, at, s, flag, self);
        };
        let ty = obj.u32(0x48);
        let b: Vec<u8> = match ty {
            1 | 7 => t.clone(),
            2 => {
                int(out, self.s(ZERO2), true);
                add_sep(out, 0, self.w(1));
                let k = (c(&t, 0) != b'+') as usize + 1;
                let s = &t[k.min(t.len())..];
                let r = self.prefix_len(0, s);
                int(out, &s[..r.min(s.len())], true);
                add_sep(out, 0, self.w(0));
                int(out, &s[r.min(s.len())..], true);
                return;
            }
            4 | 5 => {
                int(out, self.s(ZERO2), true);
                add_sep(out, 0, self.w(1));
                int(out, self.s(ITALY), false);
                add_sep(out, 0, self.w(0));
                let k = (c(&t, 0) != b'+') as usize + 3;
                t[k.min(t.len())..].to_vec()
            }
            6 => {
                let r = self.prefix_len(1, &t);
                int(out, &t[..r.min(t.len())], true);
                add_sep(out, 0, self.w(0));
                int(out, &t[r.min(t.len())..], true);
                return;
            }
            8 => {
                int(out, &t, true);
                return;
            }
            _ => return,
        };
        if ty == 1 || ty == 4 {
            // a fixed line: the area code, `zero` alone before it
            let r = self.prefix_len(2, &b);
            let area = &b[..r.min(b.len())];
            if r == 3 && c(&b, 0) == b'0' && c(&b, 1) != b'0' {
                out.extend_from_slice(self.s(ZERO_WORD));
                int(out, &area[1..], false);
            } else {
                int(out, area, true);
            }
            add_sep(out, 0, self.w(0));
            int(out, &b[r.min(b.len())..], true);
        } else {
            // a mobile: the prefix digit by digit
            let r = self.prefix_len(3, &b);
            for &d in &b[..r.min(b.len())] {
                int(out, &[d], false);
                add_sep(out, 0, self.w(1));
            }
            add_sep(out, 0, self.w(0));
            int(out, &b[r.min(b.len())..], true);
        }
    }

    /// 0x10004a40: an amount of `cur` (a symbol or a code), a range `a-b`
    /// first, `uno`/`una`/`un` for one, the cents after `e`.
    fn currency_render(&self, out: &mut Vec<u8>, amount: &[u8], cur: &[u8]) {
        let mut a = amount.to_vec();
        if let Some(last) = a.iter().rposition(|&x| x == b'-').filter(|&k| k != 0) {
            a[last] = 0;
            let mut local = a[..last.min(0x3f)].to_vec();
            local.push(0);
            let mut pos = 0;
            let dash = self.s(DASH).to_vec();
            while let Some(tok) = util::strtok(&mut local, &dash, &mut pos) {
                util::convert_big_integer(out, 0, &tok, self);
                out.extend_from_slice(self.w(1));
            }
            a = a[last + 1..].to_vec();
        }
        let end = |out: &Vec<u8>| out.len();
        let units = |out: &mut Vec<u8>, s: &[u8]| {
            if util::is_big_integer(s) {
                let at = end(out);
                util::convert_big_integer(out, at, s, self);
            } else {
                self.cardinal(out, s);
            }
        };
        let one = |out: &mut Vec<u8>| {
            let id = if cur == self.s(YEN) {
                0x10
            } else if FEMININE.iter().any(|&f| cur == self.s(f)) {
                0x12
            } else {
                0x11
            };
            out.extend_from_slice(self.w(id));
        };
        let cents_digit = |out: &mut Vec<u8>, ch: u8| {
            if ch == b'1' {
                out.extend_from_slice(self.w(0x11));
            } else {
                out.extend_from_slice(self.digit(ch));
            }
        };
        if a == self.s(ZERO_CENTS) {
            out.extend_from_slice(self.w(0xf));
            self.currency(out, self.s(ZERO), cur, false);
            for id in [1, 0xd, 1, 0xf] {
                out.extend_from_slice(self.w(id));
            }
            self.currency(out, self.s(ZERO), cur, true);
            return;
        }
        let comma = self.s(COMMA).to_vec();
        let Some(p) = a.iter().position(|x| comma.contains(x)) else {
            if a == self.s(ONE) {
                one(out);
            } else {
                units(out, &a);
            }
            self.currency(out, &a, cur, false);
            return;
        };
        let units_s = a[..p].to_vec();
        let dec = a[p + 1..].to_vec();
        if dec.len() != 2 {
            units(out, &units_s);
            for id in [1, 3, 1] {
                out.extend_from_slice(self.w(id));
            }
            self.decimals(out, &dec, false);
            self.currency(out, &dec, cur, false);
            return;
        }
        let d2 = &dec[1..];
        if units_s == self.s(ZERO) {
            if dec[0] != b'0' {
                self.cardinal(out, &dec);
                self.currency(out, &dec, cur, true);
            } else {
                cents_digit(out, dec[1]);
                self.currency(out, d2, cur, true);
            }
            for id in [1, 2, 1] {
                out.extend_from_slice(self.w(id));
            }
            self.currency(out, self.s(ONE), cur, false);
            return;
        }
        if units_s == self.s(ONE) {
            one(out);
        } else {
            units(out, &units_s);
        }
        self.currency(out, &units_s, cur, false);
        if dec == self.s(ZERO2) {
            return;
        }
        for id in [1, 0xd, 1] {
            out.extend_from_slice(self.w(id));
        }
        if dec[0] != b'0' {
            self.cardinal(out, &dec);
            self.currency(out, &dec, cur, true);
        } else {
            cents_digit(out, dec[1]);
            self.currency(out, d2, cur, true);
        }
    }

    /// Slot 9 (0x1000a810): the reading of number token `idx` and how many
    /// following tokens it used.
    pub(super) fn leggi(&self, toks: &mut [Tok], idx: usize, kind: u32, dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
        let text = toks.get(idx)?.text.clone();
        let mut obj = NumObj::default();
        let mut mode = kind;
        if kind == 5 || kind == 6 {
            mode = 4;
            obj.set_u32(0x48, (kind != 5) as u32 + 5);
        }
        let (next, _) = tok_rel(toks, idx, 1);
        let currency = util::is_currency(&next)
            || (idx > 0 && util::is_currency(&tok_rel(toks, idx, -1).0))
            || CURRENCY_NAMES.iter().any(|&a| next == self.s(a));
        if currency {
            mode = 0xc;
        }
        obj.set_u32(0x44, mode);
        obj.set_cstr(4, &text[..text.len().min(0x40)]);
        type Rec = fn(&Italian, &mut Call, &mut NumObj) -> bool;
        let mut order: Vec<Rec> = vec![
            Italian::r_date,
            Italian::r_time,
            Italian::r_ordinal,
            Italian::r_currency,
            Italian::r_decimal,
            Italian::r_fraction,
            Italian::r_phone,
            Italian::r_cardinal,
            Italian::r_digits,
        ];
        // 0x1000a730: the mode's own recognizer first
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
        let f = order.remove(first);
        order.insert(0, f);
        let mut call = Call { toks, idx, dmy, used: 0 };
        if !order.iter().any(|r| r(self, &mut call, &mut obj)) {
            util::set_default_type(&mut obj, &text);
        }
        let mut out = Vec::new();
        self.render(&obj, &mut out);
        Some((out, call.used))
    }
}

impl NumSlots for Italian {
    fn word(&self, id: u32) -> &[u8] {
        self.w(id)
    }

    /// Slot 1 (0x100049b0): read as a telephone number after its area code.
    fn digits(&self, out: &mut Vec<u8>, digits: &[u8], flag: bool) {
        let r = self.prefix_len(2, digits);
        let mut len = r;
        let mut o = Vec::new();
        util::convert_phone_number(self, &mut o, digits, flag, &mut len, 0);
        util::convert_phone_number(self, &mut o, &digits[r.min(digits.len())..], flag, &mut len, 1);
        out.extend(o);
    }

    fn cardinal(&self, out: &mut Vec<u8>, digits: &[u8]) {
        out.extend(self.cardinal_c(digits));
    }

    /// Slot 3 (0x1000a070): `zero`, `uno`, else the number word.
    fn digit(&self, d: u8) -> &[u8] {
        match d {
            b'0' => self.w(0xf),
            b'1' => self.w(0x10),
            _ => self.num(d as i8 as i32 - 0x30),
        }
    }

    /// Slot 4 (0x10004a10)
    fn by_digits(&self, flag: bool, len: usize, lead0: bool) -> bool {
        flag || len > 12 || (len > 3 && lead0) || len == 1
    }

    /// Slot 5 (0x10008eb0): the currency's name (cents' name), one or many.
    fn currency(&self, out: &mut Vec<u8>, amount: &[u8], currency: &[u8], cents: bool) {
        let Some(k) = (0..18u32).find(|&k| self.s(self.pe().u32(CURRENCIES + 0x14 * k)) == currency) else { return };
        let e = CURRENCIES + 0x14 * k;
        let at = out.len();
        add_sep(out, at, self.w(1));
        let one = amount == self.s(ONE);
        let field = match (cents, one) {
            (false, true) => 4,
            (false, false) => 8,
            (true, true) => 0xc,
            (true, false) => 0x10,
        };
        out.extend_from_slice(self.s(self.pe().u32(e + field)));
        let at = out.len();
        add_sep(out, at, self.w(1));
    }

    /// Slot 6 (0x1000b310): decimals: leading zeros one by one, then up to
    /// three digits as a cardinal (one digit, or `flag`: digit by digit).
    fn decimals(&self, out: &mut Vec<u8>, digits: &[u8], flag: bool) -> bool {
        let (lead0, pause) = util::set_pauses(digits);
        let sep = self.w(if pause { 0 } else { 1 }).to_vec();
        let mut o = Vec::new();
        add_sep(&mut o, 0, &sep);
        let mut d = digits;
        if c(d, 0) == b'0' {
            loop {
                o.extend_from_slice(self.digit(b'0'));
                d = &d[1..];
                if c(d, 0) != b'0' {
                    break;
                }
                add_sep(&mut o, 0, &sep);
            }
        }
        let n = d.len() & 0xffff;
        if n != 0 {
            if lead0 {
                add_sep(&mut o, 0, &sep);
            }
            if n <= 3 && n != 1 && !flag {
                o.extend(self.cardinal_c(d));
            } else {
                for k in 1..n {
                    o.extend_from_slice(self.digit(d[k - 1]));
                    add_sep(&mut o, 0, &sep);
                }
                o.extend_from_slice(self.digit(d[n - 1]));
            }
        }
        out.extend(o);
        pause
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `"…"` field of a trace line (`\xNN`, `\\`, `\"` escapes).
    fn field(line: &str, name: &str) -> Option<Vec<u8>> {
        let start = line.find(&format!(" {name}=\""))? + name.len() + 3;
        let b = line.as_bytes();
        let mut out = Vec::new();
        let mut k = start;
        while k < b.len() && b[k] != b'"' {
            if b[k] == b'\\' {
                if b[k + 1] == b'x' {
                    out.push(u8::from_str_radix(&line[k + 2..k + 4], 16).unwrap());
                    k += 4;
                } else {
                    out.push(b[k + 1]);
                    k += 2;
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

    /// Replays every slot-9 call traced in `/tmp/qq-num-it/*.les`
    /// (`LOQ_NUM_ONLY`): the reading, the tokens used and the tokens after.
    #[test]
    #[ignore]
    fn replay_slot9() {
        let Ok(dll) = std::fs::read("/tmp/qqv/LTTS7/bin/LTTS7Italian.dll") else { return };
        let it = Italian::from_dll(dll).unwrap();
        let mut files: Vec<_> = std::fs::read_dir("/tmp/qq-num-it").into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "les")).collect();
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
                    after.push(tok(lines[k]));
                    k += 1;
                }
                calls += 1;
                let (got, used) = it.leggi(&mut toks, idx, kind, &dmy).unwrap();
                let same_toks = after.is_empty() || toks.iter().zip(&after).all(|(a, b)| a.text == b.text && a.kind == b.kind);
                if got != ret || used != c18 || !same_toks {
                    bad += 1;
                    eprintln!(
                        "{}: {:?} kind {kind}: got {:?} {used}, want {:?} {c18}{}",
                        f.display(),
                        String::from_utf8_lossy(&toks[idx].text),
                        String::from_utf8_lossy(&got),
                        String::from_utf8_lossy(&ret),
                        if same_toks { "" } else { " (tokens differ)" }
                    );
                }
            }
        }
        eprintln!("{bad} of {calls} slot-9 calls differ");
        assert_eq!(bad, 0);
    }
}
