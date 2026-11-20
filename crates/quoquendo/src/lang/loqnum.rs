//! LoqTTS7.dll's generic number reader, `LesNumero2Testo` (0x1003ebf0,
//! MECHANICS.md §4n), for the language modules without a `LesIniNumeri`
//! (Danish, Russian). The engine classifies the number token, parses it into
//! an `ELQNum` object (LTTS7Util), has it read as a list of morph ids and
//! turns the morphs into words with the module's morph table. Each step is a
//! slot of the module's binder (LES 0x10036140, +0x50…+0x10c) that the
//! module may fill with its own function; [`NumModule`]'s defaults are the
//! engine's own.

use super::common::{from_8bit_cp, Pe, Tok};
use super::elqnum::atol;
use super::textfilter::{matches_whole, Classes};

/// No currency (the tables' "not found", 0x42a).
pub const NO_CURRENCY: i32 = 0x42a;

/// LTTS7Util's `ELQNum` object (`ELQNumIni` 0x10015220: 0x524 bytes, zeroed;
/// the morph list behind +0x210). Fields overlap as Util's accessors make
/// them: type +4, subtype +8, escape +0xc, parse mode +0x10, reading mode
/// +0x110, focus +0x214, skip +0x21c, ints +0x220…+0x234, strings +0x220
/// (code, international prefix), +0x228 (big integer), +0x22c (decimals),
/// +0x320 (area code), +0x32c (unit), +0x420 (telephone number); the string
/// setters are `strncpy(…, 0x100)` with the last byte cleared.
#[derive(Clone)]
pub struct Num {
    b: Box<[u8; 0x524]>,
    pub morphs: Vec<i32>,
}

impl Default for Num {
    fn default() -> Self {
        Num { b: Box::new([0; 0x524]), morphs: Vec::new() }
    }
}

pub mod off {
    pub const TYPE: usize = 4;
    pub const ESCAPE: usize = 0xc;
    pub const PARSE_MODE: usize = 0x10;
    pub const READING_MODE: usize = 0x110;
    pub const FOCUS: usize = 0x214;
    pub const SKIP: usize = 0x21c;
    /// integer / ordinal / currency / real / time hour / date day / fraction numerator / angle degrees / result first
    pub const I0: usize = 0x220;
    /// big-value flag / exponent / time minutes / date month / denominator / angle minutes / result second
    pub const I1: usize = 0x224;
    /// separator / time seconds / date year / angle seconds
    pub const I2: usize = 0x228;
    /// time hour end / date day end
    pub const I3: usize = 0x22c;
    pub const I4: usize = 0x230;
    pub const I5: usize = 0x234;
    pub const CURRENCY: usize = 0x32c;
    pub const S_CODE: usize = 0x220;
    pub const S_BIG: usize = 0x228;
    pub const S_DECIMALS: usize = 0x22c;
    pub const S_AREA: usize = 0x320;
    pub const S_UNIT: usize = 0x32c;
    pub const S_NUMBER: usize = 0x420;
}

impl Num {
    pub fn i(&self, at: usize) -> i32 {
        i32::from_le_bytes(self.b[at..at + 4].try_into().unwrap())
    }
    pub fn set_i(&mut self, at: usize, v: i32) {
        self.b[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
    /// The C string at `at`.
    pub fn s(&self, at: usize) -> Vec<u8> {
        let t = &self.b[at..];
        t[..t.iter().position(|&c| c == 0).unwrap_or(t.len())].to_vec()
    }
    /// `strncpy(at, s, 0x100)`, then the 0x100th byte cleared.
    pub fn set_s(&mut self, at: usize, s: &[u8]) {
        let n = s.iter().position(|&c| c == 0).unwrap_or(s.len()).min(0x100);
        let end = (at + 0x100).min(self.b.len());
        self.b[at..end].fill(0);
        self.b[at..at + n.min(end - at)].copy_from_slice(&s[..n.min(end - at)]);
        if at + 0xff < self.b.len() {
            self.b[at + 0xff] = 0;
        }
    }
    pub fn typ(&self) -> i32 {
        self.i(off::TYPE)
    }
    pub fn escape(&self) -> i32 {
        self.i(off::ESCAPE)
    }
    pub fn reading_mode(&self) -> Vec<u8> {
        self.s(off::READING_MODE)
    }
    pub fn parse_mode(&self) -> Vec<u8> {
        self.s(off::PARSE_MODE)
    }
    pub fn set_modes(&mut self, parse: &[u8], reading: &[u8]) {
        self.set_s(off::PARSE_MODE, parse);
        self.set_s(off::READING_MODE, reading);
    }
    /// `ELQNumSetContextSkipToken`.
    pub fn set_skip(&mut self, n: i32) {
        self.set_i(off::SKIP, n);
    }
    /// `ELQNumAddMorph` (0x100152e0).
    pub fn add(&mut self, id: i32) {
        self.morphs.push(id);
    }
    /// `ELQNumIntegerSetValue`: the value, the big flag cleared.
    pub fn set_integer(&mut self, v: i32) {
        self.set_i(off::I0, v);
        self.set_i(off::I1, 0);
    }
    /// `ELQNumIntegerSetBigvalue`: the digits, the big flag set.
    pub fn set_big(&mut self, s: &[u8]) {
        self.set_s(off::S_BIG, s);
        self.set_i(off::I1, 1);
    }
}

/// A `{int id, char *s}` (or `{char *s, int id}` for the classify table)
/// table as the module's `LesGet*Table` hands it: `{entries, count}`.
#[derive(Clone, Debug, Default)]
pub struct Tables {
    /// LesGetMorphTable (binder +0x110)
    pub morph: Vec<(i32, Vec<u8>)>,
    /// LesGetClassifyTable (+0x114): regular expression, escape
    pub classify: Vec<(Vec<u8>, i32)>,
    /// LesGetCurrencyTable (+0x118)
    pub currency: Vec<(i32, Vec<u8>)>,
    /// LesGetOrdinalStringTable (+0x11c)
    pub ordinal: Vec<(i32, Vec<u8>)>,
}

impl Tables {
    pub fn read(pe: &Pe, morph: u32, classify: u32, currency: u32, ordinal: u32) -> Tables {
        let pairs = |va: u32| -> Vec<(u32, u32)> {
            let (at, n) = (pe.u32(va), pe.u32(va + 4));
            (0..n).map(|k| (pe.u32(at + 8 * k), pe.u32(at + 8 * k + 4))).collect()
        };
        let ids = |va: u32| pairs(va).into_iter().map(|(id, s)| (id as i32, pe.cstr(s).to_vec())).collect();
        Tables {
            morph: ids(morph),
            classify: pairs(classify).into_iter().map(|(s, id)| (pe.cstr(s).to_vec(), id as i32)).collect(),
            currency: ids(currency),
            ordinal: ids(ordinal),
        }
    }

    /// 0x100389e0 (Russian 0x10004670, Danish 0x10007710): the first currency
    /// entry spelled `s` (any case), else [`NO_CURRENCY`].
    pub fn currency_of(&self, s: &[u8]) -> i32 {
        self.currency.iter().find(|(_, c)| stricmp(c, s)).map(|e| e.0).unwrap_or(NO_CURRENCY)
    }
}

/// `ELQstricmp` == 0 (ASCII case).
pub fn stricmp(a: &[u8], b: &[u8]) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// The number's sentence as the engine's token buffer holds it (0x40c
/// bytes a token; type 5 tags are skipped, type 7 ends it): the tokens from
/// the sentence's start to its closing punctuation, the number at `focus`.
pub struct Ctx<'a> {
    pub toks: &'a [Tok],
    pub focus: usize,
    /// the number token's text (the driver's string)
    pub text: Vec<u8>,
}

impl Ctx<'_> {
    fn kind(&self, k: usize) -> u32 {
        self.toks.get(k).map(|t| t.kind).unwrap_or(7)
    }
    /// 0x10038930 (Russian 0x10004560, Danish 0x10006110, limit 0xf): the
    /// text of the token `off` tokens from the number, tags not counted;
    /// forward it stops at the end (type 7) or index 14; `""` past either.
    pub fn tok(&self, off: i32) -> &[u8] {
        let mut k = self.focus;
        let mut off = off;
        if off < 0 {
            while off < 0 && k > 0 {
                k -= 1;
                if self.kind(k) != 5 {
                    off += 1;
                }
            }
        } else {
            while off > 0 && k <= 0xd {
                k += 1;
                if self.kind(k) == 7 {
                    break;
                }
                if self.kind(k) != 5 {
                    off -= 1;
                }
            }
        }
        if off == 0 {
            self.toks.get(k).map(|t| &t.text[..]).unwrap_or(b"")
        } else {
            b""
        }
    }
}

/// The parse/read slots by the type they give (binder +0x58…+0x84,
/// +0x8c…+0xb8): Telefonico 1, Codice 2, Intero 3, Ordinale 4/5 (5
/// feminine), Reale 6, Ora 7, Data 8, Valuta 9, Frazione 10, Angolare 0xb,
/// Misura 0xc, Risultato 0xd.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Telefonico,
    Codice,
    Intero,
    Ordinale,
    Reale,
    Ora,
    Data,
    Valuta,
    Frazione,
    Angolare,
    Misura,
    Risultato,
}

impl Kind {
    fn of(t: i32) -> Option<Kind> {
        Some(match t {
            1 => Kind::Telefonico,
            2 => Kind::Codice,
            3 => Kind::Intero,
            4 | 5 => Kind::Ordinale,
            6 => Kind::Reale,
            7 => Kind::Ora,
            8 => Kind::Data,
            9 => Kind::Valuta,
            10 => Kind::Frazione,
            0xb => Kind::Angolare,
            0xc => Kind::Misura,
            0xd => Kind::Risultato,
            _ => return None,
        })
    }
}

/// The `LesDo*` hooks the readers consult (binder +0xc0…+0x10c, in that
/// order; the engine's defaults 0x1003dec0 / LesDoGender 0x1003ded0 all
/// answer 0). Called `(tables, value, num, type)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Do {
    YearSplit,
    TenthInversion,
    AndBetweenTenthsUnits,
    AndBetweenHundredsTenths,
    AndBetweenThousandsHundreds,
    Truncation,
    TruncateTenth,
    OrdinalTenth,
    Composed,
    Gender,
    Plural,
    OverOnFraction,
    OrdinalDay,
    OfBetweenDayMonth,
    OfBetweenMonthYear,
    OClock,
    FeminineHour,
    Hour,
    Minutes,
    Seconds,
}

/// What the engine's number reader asks of a language module. The default
/// methods are LoqTTS7.dll's own functions for the slots the module leaves
/// empty.
pub trait NumModule {
    fn tables(&self) -> &Tables;
    /// LTTS7Util's character classes (for the classify expressions).
    fn classes(&self) -> Option<&Classes>;
    /// LoqTTS7.dll itself, for its own tables (the country codes).
    fn engine(&self) -> Option<&Pe>;
    /// `LesClassifyFunc` (+0x54): the escape for a number without one.
    /// Default 0x10038a20.
    fn classify(&self, n: &mut Num, c: &Ctx) -> i32 {
        default_classify(self, n, c)
    }
    /// `LesParse*` (+0x58…+0x84). Defaults 0x10038e40… (Angolare, Misura,
    /// Risultato: 0x1003dec0, no).
    fn parse(&self, k: Kind, n: &mut Num, c: &Ctx) -> bool {
        default_parse(self, k, n, c)
    }
    /// `LesRead*` (+0x8c…+0xb8). Defaults 0x1003c060….
    fn read(&self, k: Kind, n: &mut Num, c: &Ctx) {
        default_read(self, k, n, c)
    }
    /// `LesDo*` (+0xc0…+0x10c).
    fn do_hook(&self, _h: Do, _value: i32, _n: &Num) -> i32 {
        0
    }
    /// `LesNumlettFunc` (+0x50). Default 0x10038d90.
    fn numlett(&self, n: &Num) -> Vec<u8> {
        numlett(self.tables(), n)
    }
}

/// 0x10038d90: each morph's first morph-table string, run together.
pub fn numlett(t: &Tables, n: &Num) -> Vec<u8> {
    let mut out = Vec::new();
    for &m in &n.morphs {
        if let Some((_, s)) = t.morph.iter().find(|(id, _)| *id == m) {
            out.extend_from_slice(s);
        }
    }
    out
}

/// `LesNumero2Testo` (0x1003ebf0) as 0x1003f520 calls it for the number
/// token `toks[i]`: the reading (space-separated words, `#` a pause) and how
/// many following tokens it used (ctx+0x18). `kind` is the reading type the
/// tags ask for (`common::number_tag`), mapped to the escape as 0x1003f520
/// does; `dmy` the parse and reading mode (ctx+0x32c).
pub fn leggi_numero<M: NumModule + ?Sized>(m: &M, toks: &[Tok], i: usize, kind: u32, dmy: &[u8]) -> (Vec<u8>, usize) {
    use super::common::{ends_sentence, tok};
    let boundary = |t: &Tok| t.kind == tok::PUNCT && ends_sentence(&t.text);
    let start = toks[..i].iter().rposition(boundary).map(|k| k + 1).unwrap_or(0);
    let end = toks[i..].iter().position(boundary).map(|k| i + k + 1).unwrap_or(toks.len());
    let c = Ctx {
        toks: &toks[start..end],
        focus: i - start,
        text: toks[i].text.clone(),
    };
    let escape = match kind {
        1 => 1,
        2 => 2,
        4 | 5 => 4,
        6 => 5,
        7 => 7,
        8 => 8,
        9 => 0xb,
        10 => 6,
        0xb => 10,
        0xc => 9,
        _ => 0,
    };
    let mut n = Num::default();
    n.set_modes(dmy, dmy);
    n.set_i(off::ESCAPE, escape);
    n.set_i(off::FOCUS, c.focus as i32);
    if escape == 0 {
        let e = m.classify(&mut n, &c);
        n.set_i(off::ESCAPE, e);
    }
    let e = n.escape();
    let ok = match Kind::of(e) {
        Some(k) => m.parse(k, &mut n, &c),
        None => false,
    };
    n.set_i(off::TYPE, if ok { e } else { 0 });
    if !ok {
        // the fixed order (0x1003ed35)
        const ORDER: [(Kind, i32); 12] = [
            (Kind::Data, 8),
            (Kind::Ora, 7),
            (Kind::Valuta, 9),
            (Kind::Angolare, 0xb),
            (Kind::Frazione, 10),
            (Kind::Telefonico, 1),
            (Kind::Reale, 6),
            (Kind::Misura, 0xc),
            (Kind::Risultato, 0xd),
            (Kind::Intero, 3),
            (Kind::Ordinale, 4),
            (Kind::Codice, 2),
        ];
        match ORDER.iter().find(|(k, _)| m.parse(*k, &mut n, &c)) {
            Some(&(_, t)) => n.set_i(off::TYPE, t),
            // "Error parsing string": no reading
            None => return (Vec::new(), 0),
        }
    }
    if let Some(k) = Kind::of(n.typ()) {
        m.read(k, &mut n, &c);
    }
    let words = m.numlett(&n);
    // QQ_TRACE_NUM=1: the object as LOQ_TRACE_NUM logs the original's
    // (oracle/loqref.c), on stderr
    if std::env::var_os("QQ_TRACE_NUM").is_some() {
        let m: Vec<String> = n.morphs.iter().map(|v| v.to_string()).collect();
        let s = |b: Vec<u8>| -> String { b.iter().map(|&c| if (0x20..0x7f).contains(&c) { (c as char).to_string() } else { format!("\\x{c:02x}") }).collect() };
        eprintln!(
            "N type={} esc={} skip={} mode=\"{}\" i={},{},{} text=\"{}\" m={}",
            n.typ(),
            n.escape(),
            n.i(off::SKIP),
            s(n.reading_mode()),
            n.i(off::I0),
            n.i(off::I1),
            n.i(off::I2),
            s(c.text.clone()),
            m.join(",")
        );
    }
    (words, n.i(off::SKIP).max(0) as usize)
}

/// The classify table's first expression the whole text matches (the
/// patterns and the text converted from code page 1252 to UTF-16, as the
/// classifiers' `ELQTextEncodingConvert(0x4e4, …)` do), with its escape.
pub fn classify_by_table<M: NumModule + ?Sized>(m: &M, text: &[u8]) -> Option<i32> {
    let cl = m.classes()?;
    let wide = |s: &[u8]| -> Vec<char> { s.iter().map(|&b| from_8bit_cp(b, 1252)).collect() };
    let t = wide(text);
    m.tables().classify.iter().find(|(p, _)| matches_whole(cl, &wide(p), &t) == Some(true)).map(|e| e.1)
}

/// 0x10038a20, the engine's `LesClassifyFunc`: the classify table's escape;
/// a cardinal (3) becomes the ordinal table's escape of the next token
/// (unless 0 or 3); a real (6) or cardinal next to a currency is money (9).
pub fn default_classify<M: NumModule + ?Sized>(m: &M, _n: &mut Num, c: &Ctx) -> i32 {
    let t = m.tables();
    let mut id = classify_by_table(m, &c.text).unwrap_or(0);
    if id == 3 {
        if let Some(e) = t.ordinal.iter().find(|(_, s)| stricmp(s, c.tok(1))) {
            if e.0 != 0 {
                id = e.0;
            }
        }
        if id != 3 && id != 6 {
            return id;
        }
    } else if id != 6 {
        return id;
    }
    if t.currency_of(c.tok(1)) != NO_CURRENCY || t.currency_of(c.tok(-1)) != NO_CURRENCY {
        return 9;
    }
    id
}

// --- The engine's parsers -------------------------------------------------

fn digit(c: u8) -> bool {
    c.is_ascii_digit()
}

/// The value check the parsers share: at most ten digits after the sign,
/// and ten only up to 2147483647.
fn fits(s: &[u8]) -> bool {
    let sign = matches!(s.first(), Some(b'-' | b'+')) as usize;
    let digits = &s[sign.min(s.len())..];
    s.len() < sign + 10 || (s.len() == sign + 10 && digits[..10] <= b"2147483647"[..])
}

/// `strncmp(a, b, 10) > 0` on ten digits.
fn over_max(s: &[u8]) -> bool {
    s.len() >= 10 && s[..10] > b"2147483647"[..]
}

pub fn default_parse<M: NumModule + ?Sized>(m: &M, k: Kind, n: &mut Num, c: &Ctx) -> bool {
    let s = &c.text[..];
    match k {
        Kind::Telefonico => parse_telefonico(n, s, m.engine()),
        Kind::Codice => {
            n.set_s(off::S_CODE, s);
            true
        }
        Kind::Intero => parse_intero(n, s),
        Kind::Ordinale => parse_ordinale(n, c),
        Kind::Reale => parse_reale(n, s),
        Kind::Ora => parse_ora(n, s),
        Kind::Data => parse_data(n, s, parse_data_part),
        Kind::Valuta => parse_valuta(m, n, c),
        Kind::Frazione => parse_frazione(n, s),
        Kind::Angolare | Kind::Misura | Kind::Risultato => false,
    }
}

/// 0x10038e40: digits (a leading `+`); anything else ends it unless the
/// escape asked for a telephone number. `+`/`00` and a country code from the
/// table at LoqTTS7.dll 0x10071350 (0xda prefixes, `strncmp(prefix, number,
/// strlen(prefix))`, first match) make the international part; without the
/// engine's DLL no prefix matches.
fn parse_telefonico(n: &mut Num, s: &[u8], engine: Option<&Pe>) -> bool {
    let mut b = Vec::new();
    for &ch in s.iter().take(0x7f) {
        if digit(ch) {
            b.push(ch);
        } else if ch == b'+' && b.is_empty() {
            b.push(b'+');
        } else if n.escape() != 1 {
            return false;
        }
    }
    let skip = if b.first() == Some(&b'+') {
        1
    } else if b.starts_with(b"00") {
        2
    } else {
        n.set_s(off::S_NUMBER, &b);
        return true;
    };
    let lead: &[u8] = if skip == 1 { b"+" } else { b"00" };
    let rest = &b[skip..];
    let code = engine.and_then(|pe| (0..COUNTRY_CODES_N).map(|k| pe.cstr(pe.u32(COUNTRY_CODES + 4 * k))).find(|p| rest.starts_with(p)));
    match code {
        Some(p) => {
            n.set_s(off::S_CODE, &[lead, p].concat());
            n.set_s(off::S_NUMBER, &rest[p.len()..]);
        }
        None => {
            n.set_s(off::S_CODE, lead);
            n.set_s(off::S_NUMBER, rest);
        }
    }
    true
}

/// LoqTTS7.dll: the country codes' pointers and their count.
const COUNTRY_CODES: u32 = 0x10071350;
const COUNTRY_CODES_N: u32 = 0xda;

/// 0x10039010: digits (a leading sign), `,` and `.` counted; both, or
/// anything else, refuse unless the escape asked for an integer. More than
/// ten digits (or above 2147483647) make a big value.
fn parse_intero(n: &mut Num, s: &[u8]) -> bool {
    let (mut b, mut commas, mut dots) = (Vec::new(), 0, 0);
    for &ch in s.iter() {
        if b.len() > 0x7e {
            break;
        }
        if digit(ch) || ((ch == b'-' || ch == b'+') && b.is_empty()) {
            b.push(ch);
        } else if ch == b',' {
            commas += 1;
        } else if ch == b'.' {
            dots += 1;
        } else if n.escape() != 3 {
            return false;
        }
    }
    if dots != 0 && commas != 0 && n.escape() != 3 {
        return false;
    }
    let sign = matches!(b.first(), Some(b'-' | b'+')) as usize;
    if b.len() > sign + 10 || (b.len() == sign + 10 && over_max(&b[sign..])) {
        n.set_big(&b);
    } else {
        n.set_integer(atol(&b));
    }
    true
}

/// 0x10039180: digits, `,` `.` counted, `'` `°` `º` `ª` passed (others
/// only with escape 4/5); then `°`/`º` not followed by `c`, or `ª`, after
/// it is used up (skip 1).
fn parse_ordinale(n: &mut Num, c: &Ctx) -> bool {
    let s = &c.text[..];
    let (mut b, mut commas, mut dots) = (Vec::new(), 0, 0);
    let esc = n.escape();
    for &ch in s.iter() {
        if b.len() > 0x7e {
            break;
        }
        if digit(ch) {
            b.push(ch);
        } else if ch == b',' {
            commas += 1;
        } else if ch == b'.' {
            dots += 1;
        } else if !matches!(ch, b'\'' | 0xb0 | 0xba | 0xaa) && esc != 4 && esc != 5 {
            return false;
        }
    }
    if (dots != 0 && commas != 0 && esc != 4 && esc != 5) || b.len() > 10 || (b.len() == 10 && over_max(&b)) {
        return false;
    }
    n.set_i(off::I0, atol(&b));
    let (n1, n2) = (c.tok(1), c.tok(2));
    let degree = n1 == b"\xb0" || n1 == b"\xba";
    if (degree && !stricmp(n2, b"c")) || n1 == b"\xaa" {
        n.set_skip(1);
    }
    true
}

/// Where the parsers of reals and amounts put the decimal separator: its
/// index, `None` for no separator.
fn real_digits(n: &mut Num, s: &[u8], sep_at: Option<usize>, escape: i32, sep_field: usize) -> Option<(Vec<u8>, Vec<u8>)> {
    let (mut int, mut dec) = (Vec::new(), Vec::new());
    let mut after = false;
    let mut signed = false;
    for (k, &ch) in s.iter().enumerate().take(0x7f) {
        match ch {
            b'+' | b'-' if !signed && !after && int.is_empty() => {
                int.push(ch);
                signed = true;
            }
            b',' | b'.' => {
                if Some(k) == sep_at && !after {
                    n.set_i(sep_field, if ch == b',' { 6 } else { 5 });
                    after = true;
                }
            }
            b'0'..=b'9' => {
                if after {
                    dec.push(ch)
                } else {
                    int.push(ch)
                }
            }
            _ => {
                if n.escape() != escape {
                    return None;
                }
            }
        }
    }
    Some((int, dec))
}

/// 0x10039400: the separator is the only `.` or `,` (with one of each,
/// the later); two or more of both only with escape 6 (the later); digits
/// before it the integer, after it the decimals.
fn parse_reale(n: &mut Num, s: &[u8]) -> bool {
    let (mut dots, mut commas, mut dot, mut comma) = (0, 0, 0, 0);
    for (k, &ch) in s.iter().enumerate().take(0x7f) {
        if ch == b'.' {
            dots += 1;
            dot = k;
        } else if ch == b',' {
            commas += 1;
            comma = k;
        }
    }
    let sep = if dots >= 2 && commas >= 2 {
        if n.escape() != 6 {
            return false;
        }
        Some(dot.max(comma))
    } else if dots == 1 {
        Some(if commas == 1 && comma > dot { comma } else { dot })
    } else if commas == 1 {
        Some(comma)
    } else {
        None
    };
    let Some((int, dec)) = real_digits(n, s, sep, 6, off::I2) else { return false };
    if !fits(&int) || dec.len() > 10 || (dec.len() == 10 && over_max(&dec)) {
        return false;
    }
    n.set_i(off::I0, atol(&int));
    n.set_s(off::S_DECIMALS, &dec);
    n.set_i(off::I1, 0);
    true
}

/// 0x10039770: up to three fields (hours, minutes, seconds) split by
/// `, - . :` (the last digits only); other characters refuse unless the
/// escape asked for a time. Hours below 25, minutes and seconds below 60.
fn time_parts(n: &Num, s: &[u8]) -> Option<(i32, i32, i32)> {
    let mut v = [-1i32; 3];
    let mut k = 0;
    for (f, slot) in v.iter_mut().enumerate() {
        let mut b = Vec::new();
        while k < s.len() && b.len() <= 0x7e {
            let ch = s[k];
            k += 1;
            if digit(ch) {
                b.push(ch);
            } else if f < 2 && matches!(ch, b',' | b'-' | b'.' | b':') {
                break;
            } else if n.escape() != 7 {
                return None;
            }
        }
        if !b.is_empty() {
            *slot = atol(&b);
        }
    }
    (v[0] < 0x19 && v[1] < 0x3c && v[2] < 0x3c).then_some((v[0], v[1], v[2]))
}

/// One `-` (not first) among other separators: a range, its parts parsed
/// by `part`; else the whole string. Returns the parts.
fn range<T>(s: &[u8], part: impl Fn(&[u8]) -> Option<T>) -> Option<(T, Option<T>)> {
    let dashes = s.iter().filter(|&&c| c == b'-').count();
    let dash = s.iter().rposition(|&c| c == b'-').unwrap_or(0);
    let others = s.iter().filter(|&&c| c != b'-' && !digit(c)).count();
    if dashes == 1 && dash != 0 && others != 0 {
        let a = part(&s[..dash])?;
        if dash == s.len() - 1 {
            return Some((a, None));
        }
        let b = part(&s[dash + 1..])?;
        return Some((a, Some(b)));
    }
    Some((part(s)?, None))
}

/// 0x100399e0.
fn parse_ora(n: &mut Num, s: &[u8]) -> bool {
    for f in [off::I0, off::I1, off::I2, off::I3, off::I4, off::I5] {
        n.set_i(f, -1);
    }
    let Some((a, b)) = range(s, |p| time_parts(n, p)) else { return false };
    n.set_i(off::I0, a.0);
    n.set_i(off::I1, a.1);
    n.set_i(off::I2, a.2);
    if let Some(b) = b {
        n.set_i(off::I3, b.0);
        n.set_i(off::I4, b.1);
        n.set_i(off::I5, b.2);
    }
    true
}

/// 0x10039bd0 (Danish 0x100089b0): a date's (day, month, year) in the
/// parse mode's order (`dmy` …; without a letter, the free field), fields
/// split by `- . /`; a year below 20 is 20xx, up to 99 19xx, 100–999 +1000;
/// month 1–12, day 1–31 and possible in the month.
pub fn parse_data_part(n: &Num, s: &[u8]) -> Option<(i32, i32, i32)> {
    let mode = n.parse_mode();
    let (mut day, mut month, mut year) = (-1i32, -1i32, -1i32);
    let mut k = 0;
    for f in 0..3 {
        let mut b = Vec::new();
        while k < s.len() && b.len() <= 0x7e {
            let ch = s[k];
            k += 1;
            if digit(ch) {
                b.push(ch);
            } else if f < 2 && matches!(ch, b'-' | b'.' | b'/') {
                break;
            } else if n.escape() != 8 {
                return None;
            }
        }
        if b.is_empty() {
            continue;
        }
        let v = atol(&b);
        let letter = mode.get(f).copied();
        let slot = match (f, letter) {
            (0, None) | (_, Some(b'd')) => &mut day,
            (_, Some(b'm')) => &mut month,
            (_, Some(_)) => &mut year,
            (1, None) => {
                if month == -1 {
                    &mut month
                } else if day == -1 {
                    &mut day
                } else if year == -1 {
                    &mut year
                } else {
                    continue;
                }
            }
            _ => {
                if year == -1 {
                    &mut year
                } else if month == -1 {
                    &mut month
                } else if day == -1 {
                    &mut day
                } else {
                    continue;
                }
            }
        };
        *slot = v;
    }
    if (100..=999).contains(&year) {
        year += 1000;
    } else if (0..20).contains(&year) {
        year += 2000;
    } else if (20..=99).contains(&year) {
        year += 1900;
    }
    if year == 0 || month == 0 || day == 0 || day >= 0x20 || month >= 0xd {
        return None;
    }
    if month == 2 && ((year % 4 != 0 && year != -1 && day > 28) || day > 29) {
        return None;
    }
    if matches!(month, 4 | 6 | 9 | 11) && day >= 31 {
        return None;
    }
    Some((day, month, year))
}

/// 0x1003a040 (Danish LesParseData 0x10008e20): needs a parse mode.
pub fn parse_data(n: &mut Num, s: &[u8], part: fn(&Num, &[u8]) -> Option<(i32, i32, i32)>) -> bool {
    for f in [off::I0, off::I1, off::I2, off::I3, off::I4, off::I5] {
        n.set_i(f, -1);
    }
    let Some((a, b)) = range(s, |p| part(n, p)) else { return false };
    n.set_i(off::I0, a.0);
    n.set_i(off::I1, a.1);
    n.set_i(off::I2, a.2);
    if let Some(b) = b {
        n.set_i(off::I3, b.0);
        n.set_i(off::I4, b.1);
        n.set_i(off::I5, b.2);
    }
    true
}

/// 0x1003a280: an amount (the separator as for reals, but with one of each
/// the comma unless it comes first; escape 9 for the rest), the currency of
/// the next token (then used up) or else the previous one.
fn parse_valuta<M: NumModule + ?Sized>(m: &M, n: &mut Num, c: &Ctx) -> bool {
    let s = &c.text[..];
    let (mut dots, mut commas, mut dot, mut comma) = (0, 0, 0, 0);
    for (k, &ch) in s.iter().enumerate().take(0x7f) {
        if ch == b'.' {
            dots += 1;
            dot = k;
        } else if ch == b',' {
            commas += 1;
            comma = k;
        }
    }
    let sep = if dots >= 2 && commas >= 2 {
        if n.escape() != 9 {
            return false;
        }
        Some(dot.max(comma))
    } else if dots == 1 {
        Some(if commas == 1 && dot <= comma { comma } else { dot })
    } else if commas == 1 {
        Some(comma)
    } else {
        None
    };
    let Some((int, dec)) = real_digits(n, s, sep, 9, off::I2) else { return false };
    if !fits(&int) || dec.len() > 10 || (dec.len() == 10 && over_max(&dec)) {
        return false;
    }
    n.set_i(off::I0, atol(&int));
    n.set_s(off::S_DECIMALS, &dec);
    n.set_i(off::I1, 0);
    n.set_i(off::CURRENCY, NO_CURRENCY);
    let t = m.tables();
    if let Some(e) = t.currency.iter().find(|(_, s)| stricmp(s, c.tok(1))) {
        if e.0 != NO_CURRENCY {
            n.set_i(off::CURRENCY, e.0);
            n.set_skip(1);
            return true;
        }
    }
    if let Some(e) = t.currency.iter().find(|(_, s)| stricmp(s, c.tok(-1))) {
        if e.0 != NO_CURRENCY {
            n.set_i(off::CURRENCY, e.0);
        }
    }
    true
}

/// 0x1003a720: `¼ ½ ¾`, or numerator `/` denominator (a second `/` only
/// with escape 10).
fn parse_frazione(n: &mut Num, s: &[u8]) -> bool {
    match s.first() {
        Some(0xbc) => return set_fraction(n, 1, 4),
        Some(0xbd) => return set_fraction(n, 1, 2),
        Some(0xbe) => return set_fraction(n, 3, 4),
        _ => {}
    }
    let mut parts: [Vec<u8>; 3] = Default::default();
    let mut f = 0usize;
    let (mut commas, mut dots) = (0, 0);
    let esc = n.escape();
    for &ch in s.iter().take(0x7f) {
        match ch {
            b'+' | b'-' => {
                if parts[f].is_empty() {
                    parts[f].push(ch)
                } else if esc != 10 {
                    return false;
                }
            }
            b',' if f < 2 => commas += 1,
            b'.' if f < 2 => dots += 1,
            b',' | b'.' => {}
            b'/' => {
                if f == 0 {
                    f = 1;
                } else if esc != 10 {
                    return false;
                } else {
                    f = 2;
                }
            }
            b'0'..=b'9' => parts[f].push(ch),
            _ => {
                if esc != 10 {
                    return false;
                }
            }
        }
    }
    if dots != 0 && commas != 0 && esc != 10 {
        return false;
    }
    if !fits(&parts[0]) || !fits(&parts[1]) {
        return false;
    }
    set_fraction(n, atol(&parts[0]), atol(&parts[1]))
}

fn set_fraction(n: &mut Num, a: i32, b: i32) -> bool {
    n.set_i(off::I0, a);
    n.set_i(off::I1, b);
    true
}

// --- The engine's readers (and the Danish module's copies of them) ---------

/// The morph of a unit digit `d` (1–9): 0x19 + 0x2a·(d−1); its ending
/// follows at +3.
pub fn unit(d: i32) -> i32 {
    0x19 + 0x2a * (d - 1)
}

/// The cardinal builder 0x1003aa30…0x1003b790 (Danish copies
/// 0x10006400…0x10007160, which differ only in reading a neuter `1` as
/// 0x20 when the gender flag is 2). Flags: `ord` ordinal, `gender` (the Gender
/// hook decides, 2 its own value), `plural`, `alt`, `comp` (a composed
/// part).
pub struct Card<'a, M: NumModule + ?Sized> {
    pub m: &'a M,
    pub neuter_one: bool,
}

#[derive(Clone, Copy, Default)]
pub struct Flags {
    pub ord: bool,
    pub gender: i32,
    pub plural: bool,
    pub alt: bool,
    pub comp: bool,
}

impl<M: NumModule + ?Sized> Card<'_, M> {
    fn hook(&self, h: Do, v: i32, n: &Num) -> i32 {
        self.m.do_hook(h, v, n)
    }
    /// 0x1003aa30: the stem's variant.
    pub fn stem(&self, v: i32, f: Flags, n: &Num) -> i32 {
        if v == 0 || v >= 100 {
            return 0;
        }
        let base = if f.ord {
            1
        } else if f.alt {
            2
        } else {
            0
        };
        base + if f.comp && v < 0x14 && self.hook(Do::Composed, v, n) & 0xff != 0 { 0x15 } else { 0 }
    }
    /// 0x1003aaa0: the ending's variant.
    pub fn ending(&self, v: i32, f: Flags, n: &Num) -> i32 {
        if v == 0 {
            return 0;
        }
        let mut c = if f.ord {
            6
        } else if f.alt {
            0xc
        } else {
            0
        };
        if f.gender != 0 {
            c += match self.hook(Do::Gender, v, n) {
                1 => 2,
                2 => 4,
                _ => 0,
            };
        }
        if f.plural && self.hook(Do::Plural, v, n) & 0xff != 0 {
            c += 1;
        }
        if f.comp && v < 0x14 && self.hook(Do::Composed, v, n) & 0xff != 0 {
            c += 0x15;
        }
        c
    }
    /// 0x1003aba0: a digit.
    pub fn digit(&self, d: i32, f: Flags, n: &mut Num) {
        let s = self.stem(d, f, n);
        let e = self.ending(d, f, n);
        let id = match d {
            0 => 0x18,
            1 if self.neuter_one && f.gender == 2 => 0x20,
            1..=9 => {
                n.add(s + unit(d));
                if self.hook(Do::Truncation, d, n) & 0xff != 0 {
                    n.add(2);
                    return;
                }
                e + unit(d) + 3
            }
            _ => return,
        };
        n.add(id);
        n.add(2);
    }
    fn tens_stem(t: i32) -> i32 {
        if t == 2 {
            0x337
        } else {
            0x34c + 3 * (t - 3)
        }
    }
    fn tens_ending(t: i32) -> i32 {
        if t == 2 {
            0x33a
        } else {
            0x361
        }
    }
    /// 0x1003aed0: 0–99.
    pub fn tens(&self, v: i32, f: Flags, n: &mut Num) {
        let s = self.stem(v, f, n);
        let e = self.ending(v, f, n);
        if v < 10 {
            return self.digit(v, f, n);
        }
        if self.hook(Do::TenthInversion, v, n) & 0xff != 0 {
            if v % 10 != 0 {
                self.digit(v % 10, Flags { comp: true, ..f }, n);
            }
            if self.hook(Do::AndBetweenTenthsUnits, v, n) & 0xff != 0 {
                n.add(0x16);
                n.add(2);
            }
            match v / 10 {
                1 => {
                    n.add(s + 0x193);
                    n.add(e + 0x196);
                }
                t @ 2..=9 => {
                    n.add(s + Self::tens_stem(t));
                    n.add(e + Self::tens_ending(t));
                }
                _ => {}
            }
            n.add(2);
            return;
        }
        if v < 0x14 {
            let b = 0x193 + 0x2a * (v - 10);
            n.add(s + b);
            n.add(e + b + 3);
            n.add(2);
            return;
        }
        let (mut ls, mut le) = (self.stem(v, f, n), self.ending(v, f, n));
        if self.hook(Do::OrdinalTenth, v, n) & 0xff == 0 {
            ls = self.stem(v, Flags { ord: false, alt: false, ..f }, n);
            le = self.ending(v, Flags { ord: false, alt: false, ..f }, n);
        }
        let t = v / 10;
        if t <= 9 {
            n.add(ls + Self::tens_stem(t));
        }
        if v % 10 == 0 {
            n.add(le + Self::tens_ending(t));
            n.add(2);
            return;
        }
        if self.hook(Do::TruncateTenth, v, n) & 0xff == 0 {
            n.add(le + Self::tens_ending(t));
            n.add(2);
            if self.hook(Do::AndBetweenTenthsUnits, v, n) & 0xff != 0 {
                n.add(0x16);
                n.add(2);
            }
        }
        self.digit(v % 10, Flags { comp: true, ..f }, n);
    }
    /// 0x1003b4d0: 0–999.
    pub fn hundreds(&self, v: i32, f: Flags, n: &mut Num) {
        if v == 0 {
            n.add(0x18);
            return;
        }
        if v < 100 {
            return self.tens(v, f, n);
        }
        let r = v % 100;
        let mut e = self.ending(v, f, n);
        if r != 0 {
            e = self.ending(v, Flags { ord: false, alt: false, ..f }, n);
        }
        let ls = 0; // the stem variant is 0 from 100 on
        match v / 100 {
            1 => {
                n.add(ls + 0x373);
                n.add(e + 0x374);
            }
            h @ 2..=9 => {
                n.add(unit(h));
                n.add(2);
                n.add(ls + 0x386);
                n.add(e + 0x387);
            }
            _ => {}
        }
        n.add(2);
        if self.hook(Do::AndBetweenHundredsTenths, v, n) & 0xff != 0 {
            n.add(0x16);
            n.add(2);
        }
        if r != 0 {
            self.tens(r, Flags { comp: true, ..f }, n);
        }
    }
    /// The ending of a power group: the whole one's, or (not exact) its
    /// gender and plural only.
    fn group_ending(&self, v: i32, g: i32, unit_g: i32, f: Flags, n: &Num) -> i32 {
        let plural = g != 1;
        if v == g * unit_g {
            self.ending(v, Flags { plural, comp: false, ..f }, n)
        } else {
            self.ending(v, Flags { ord: false, alt: false, plural, comp: false, ..f }, n)
        }
    }
    /// 0x1003b790: any int.
    pub fn number(&self, v: i32, f: Flags, n: &mut Num) {
        let mut v = v;
        let mut local = false;
        if v < 0 {
            n.add(9);
            n.add(2);
            v = v.wrapping_neg();
        }
        if v == 0 {
            n.add(0x18);
            n.add(2);
            return;
        }
        let b = v / 1_000_000_000;
        if b != 0 {
            let c = self.group_ending(v, b, 1_000_000_000, f, n);
            local = true;
            match b {
                1 => {
                    n.add(0x3e5);
                    n.add(c + 0x3e6);
                }
                2..=4 => {
                    n.add(unit(b));
                    n.add(0x3f8);
                    n.add(c + 0x3f9);
                }
                _ => {}
            }
            n.add(1);
        }
        let r = v % 1_000_000_000;
        let mm = r / 1_000_000;
        if mm != 0 {
            let c = self.group_ending(r, mm, 1_000_000, f, n);
            local = true;
            if mm == 1 {
                n.add(0x3bf);
                n.add(c + 0x3c0);
            } else {
                self.hundreds(mm, Flags::default(), n);
                n.add(0x3d2);
                n.add(c + 0x3d3);
            }
            n.add(1);
        }
        let r = r % 1_000_000;
        let t = r / 1000;
        if t != 0 {
            let c = self.group_ending(r, t, 1000, f, n);
            local = true;
            if t == 1 {
                n.add(0x399);
                n.add(c + 0x39a);
                n.add(2);
            } else {
                self.hundreds(t, Flags::default(), n);
                n.add(0x3ac);
                n.add(c + 0x3ad);
                n.add(2);
            }
            if self.hook(Do::AndBetweenThousandsHundreds, r, n) & 0xff == 0 {
                n.add(1);
            } else {
                n.add(0x16);
                n.add(2);
            }
        }
        if r % 1000 != 0 {
            self.hundreds(r % 1000, Flags { comp: local, ..f }, n);
        }
    }
    /// 0x1003bf10: digits by pairs (a last three together), a `0` or a last
    /// digit spelled; each piece followed by morph 1 (more than five digits)
    /// or 2.
    pub fn pairs(&self, s: &[u8], n: &mut Num) {
        let sep = if s.len() > 5 { 1 } else { 2 };
        let mut p = 0;
        while p < s.len() {
            let left = s.len() - p;
            if s[p] == b'0' || left == 1 {
                spell(s[p], n);
                p += 1;
            } else if left == 3 {
                self.number(atol(&s[p..p + 3]), Flags::default(), n);
                p += 3;
            } else {
                self.number(atol(&s[p..p + 2]), Flags::default(), n);
                p += 2;
            }
            n.add(sep);
        }
    }
    /// 0x1003c840: a month's name (0x40b…0x416), else the number.
    pub fn month(&self, v: i32, n: &mut Num) {
        if (1..=12).contains(&v) {
            n.add(0x40a + v);
        } else {
            self.number(v, Flags::default(), n);
        }
    }
}

/// 0x1003bcf0 (Russian 0x100041d0, Danish 0x10006200): a character's
/// morphs: a digit's stem and ending, else its symbol (`*` 4, `+` 0xb, `,`
/// 6, `-` 8, `.` 5, `/` 7, `:` 0xc, `;` 0xd, `=` 0xa, others 1); then 2.
pub fn spell(c: u8, n: &mut Num) {
    let id = match c {
        b'0' => 0x18,
        b'1'..=b'9' => {
            let d = (c - b'0') as i32;
            n.add(unit(d));
            n.add(unit(d) + 3);
            n.add(2);
            return;
        }
        b'*' => 4,
        b'+' => 0xb,
        b',' => 6,
        b'-' => 8,
        b'.' => 5,
        b'/' => 7,
        b':' => 0xc,
        b';' => 0xd,
        b'=' => 0xa,
        _ => 1,
    };
    n.add(id);
    n.add(2);
}

pub fn default_read<M: NumModule + ?Sized>(m: &M, k: Kind, n: &mut Num, c: &Ctx) {
    let card = Card { m, neuter_one: false };
    let z = Flags::default();
    match k {
        // 0x1003c060
        Kind::Telefonico => {
            for at in [off::S_CODE, off::S_AREA] {
                let s = n.s(at);
                if !s.is_empty() {
                    for &ch in &s {
                        spell(ch, n);
                        n.add(2);
                    }
                    n.add(1);
                }
            }
            let s = n.s(off::S_NUMBER);
            if !s.is_empty() {
                card.pairs(&s, n);
            }
        }
        // 0x1003c160
        Kind::Codice => {
            let s = n.s(off::S_CODE);
            let others = s.iter().filter(|c| !digit(**c)).count();
            let mut p = 0;
            while p < s.len() {
                let run: Vec<u8> = s[p..].iter().take_while(|c| digit(**c)).copied().collect();
                if run.is_empty() {
                    p += 1;
                    continue;
                }
                p += run.len();
                if run.len() < 5 && run[0] != b'0' {
                    card.number(atol(&run), z, n);
                } else {
                    card.pairs(&run, n);
                }
                n.add(if others < 2 { 2 } else { 1 });
            }
        }
        // 0x1003c300
        Kind::Intero => {
            if n.i(off::I1) == 0 {
                card.number(n.i(off::I0), z, n);
            } else {
                card.pairs(&n.s(off::S_BIG), n);
            }
        }
        // 0x1003c390
        Kind::Ordinale => {
            let gender = if n.typ() == 5 { 2 } else { 1 };
            card.number(n.i(off::I0), Flags { ord: true, gender, ..z }, n);
        }
        // 0x1003c3e0, 0x1003dca0 (the same with the measure's fields)
        Kind::Reale | Kind::Misura => read_real(&card, n, &c.text),
        // 0x1003c790
        Kind::Ora => {
            let (h, mi, s) = (n.i(off::I0), n.i(off::I1), n.i(off::I2));
            let (he, me, se) = (n.i(off::I3), n.i(off::I4), n.i(off::I5));
            read_time(&card, n, h, mi, s);
            if he != -1 || me != -1 || se != -1 {
                n.add(1);
                read_time(&card, n, he, me, se);
            }
        }
        // 0x1003d1f0
        Kind::Data => {
            let (d, mo, y) = (n.i(off::I0), n.i(off::I1), n.i(off::I2));
            let (de, me, ye) = (n.i(off::I3), n.i(off::I4), n.i(off::I5));
            let mode = n.reading_mode();
            read_date(&card, n, d, mo, y, &mode);
            if de != -1 || me != -1 || ye != -1 {
                n.add(1);
                read_date(&card, n, de, me, ye, &mode);
            }
        }
        Kind::Valuta => read_amount(&card, n, c),
        // 0x1003da40
        Kind::Frazione => {
            let (mut a, mut b) = (n.i(off::I0), n.i(off::I1));
            if a < 0 {
                n.add(9);
                n.add(2);
                a = a.wrapping_neg();
            }
            if a == 1 {
                n.add(0x19);
                n.add(2);
            } else {
                card.number(a, z, n);
                n.add(2);
            }
            if b < 0 {
                n.add(9);
                n.add(2);
                b = b.wrapping_neg();
            }
            if b == 0 {
                n.add(0x12);
                n.add(0x18);
                return;
            }
            card.number(b, Flags { plural: a != 1, alt: true, ..z }, n);
        }
        // 0x1003db40
        Kind::Angolare => {
            let (mut d, mi, s) = (n.i(off::I0), n.i(off::I1), n.i(off::I2));
            let neg = d < 0 || (d == 0 && c.text.first() == Some(&b'-'));
            let mut read_degrees = d != 0;
            if neg {
                n.add(9);
                n.add(2);
                d = d.wrapping_neg();
                read_degrees = d != 0;
            }
            if read_degrees {
                card.number(d, z, n);
                n.add(2);
                n.add(if d == 1 { 0x421 } else { 0x424 });
                n.add(1);
            }
            if mi != 0 {
                card.number(mi, z, n);
                n.add(2);
                n.add(if d == 1 { 0x422 } else { 0x425 });
                n.add(1);
            }
            if s != 0 {
                card.number(s, z, n);
                n.add(2);
                n.add(if d == 1 { 0x423 } else { 0x426 });
                n.add(1);
            }
        }
        // 0x1003de40
        Kind::Risultato => {
            card.number(n.i(off::I0), z, n);
            n.add(2);
            n.add(0x15);
            n.add(2);
            card.number(n.i(off::I1), z, n);
        }
    }
}

/// 0x1003c3e0: a sign, the integer, the separator and the decimals (as a
/// number when 1–3 digits not starting with 0, else spelled), the exponent.
pub fn read_real<M: NumModule + ?Sized>(card: &Card<M>, n: &mut Num, text: &[u8]) {
    let z = Flags::default();
    let mut int = n.i(off::I0);
    let dec = n.s(off::S_DECIMALS);
    let mut exp = n.i(off::I1);
    if int < 0 || (int == 0 && text.first() == Some(&b'-')) {
        n.add(9);
        n.add(2);
        int = int.wrapping_neg();
    }
    card.number(int, z, n);
    if !dec.is_empty() {
        n.add(2);
        n.add(n.i(off::I2));
        n.add(2);
        if dec[0] != b'0' && dec.len() < 4 {
            card.number(atol(&dec), z, n);
            n.add(2);
        } else {
            for &ch in &dec {
                spell(ch, n);
                n.add(2);
            }
        }
    }
    if exp != 0 {
        n.add(0x14);
        n.add(2);
        if exp < 0 {
            n.add(9);
            n.add(2);
            exp = exp.wrapping_neg();
        }
        card.number(exp, z, n);
    }
}

/// 0x1003c580: hours (FeminineHour, OClock, Hour), minutes, seconds.
pub fn read_time<M: NumModule + ?Sized>(card: &Card<M>, n: &mut Num, h: i32, m: i32, s: i32) {
    let z = Flags::default();
    let hook = |hk: Do, v: i32, n: &Num| card.m.do_hook(hk, v, n) & 0xff != 0;
    if h != -1 {
        let fem = card.m.do_hook(Do::FeminineHour, h, n) & 0xff;
        card.number(h, Flags { gender: fem, ..z }, n);
        if hook(Do::OClock, h, n) {
            n.add(0x420);
            n.add(2);
        }
        if hook(Do::Hour, h, n) {
            n.add(if h != 1 { 0x419 } else { 0x41c });
            n.add(2);
        }
    }
    if m > 0 {
        if s < 1 {
            if h != -1 {
                n.add(0x17);
                n.add(2);
            }
            card.number(m, z, n);
        } else {
            if h != -1 {
                n.add(1);
            }
            card.number(m, z, n);
            n.add(2);
            if hook(Do::Minutes, m, n) {
                n.add(if m != 1 { 0x418 } else { 0x41b });
                n.add(1);
            }
            n.add(0x17);
            n.add(2);
        }
    }
    if s > 0 {
        card.number(s, z, n);
        n.add(2);
        if hook(Do::Seconds, s, n) {
            n.add(if s != 1 { 0x417 } else { 0x41a });
            n.add(1);
        }
    }
}

/// 0x1003c970: a date in the reading mode's order (`mdy ymd dym dmy md dm
/// my y m d`; others read the day only).
pub fn read_date<M: NumModule + ?Sized>(card: &Card<M>, n: &mut Num, d: i32, mo: i32, y: i32, mode: &[u8]) {
    let z = Flags::default();
    let hook = |hk: Do, v: i32, n: &Num| card.m.do_hook(hk, v, n) & 0xff != 0;
    let day = |n: &mut Num, then: i32| {
        let od = hook(Do::OrdinalDay, d, n);
        card.number(d, Flags { ord: od, ..z }, n);
        n.add(then);
    };
    let year = |n: &mut Num, then: i32| {
        if hook(Do::YearSplit, y, n) {
            card.number(y / 100, z, n);
            card.number(y % 100, z, n);
        } else {
            card.number(y, z, n);
        }
        n.add(then);
    };
    match mode {
        b"mdy" => {
            if mo != -1 {
                card.month(mo, n);
                n.add(2);
            }
            if d != -1 {
                day(n, 1);
            }
            if y != -1 {
                year(n, 2);
            }
        }
        b"ymd" => {
            if y != -1 {
                year(n, 1);
            }
            if mo != -1 {
                card.month(mo, n);
                n.add(2);
            }
            if d != -1 {
                day(n, 2);
            }
        }
        b"dym" => {
            if d != -1 {
                day(n, 1);
            }
            if y != -1 {
                year(n, 1);
            }
            if mo != -1 {
                card.month(mo, n);
                n.add(2);
            }
        }
        b"dmy" | b"dm" => {
            if d != -1 {
                day(n, 2);
                if hook(Do::OfBetweenDayMonth, d, n) {
                    n.add(3);
                    n.add(2);
                }
            }
            if mo != -1 {
                card.month(mo, n);
                n.add(2);
                if mode == b"dmy" && hook(Do::OfBetweenMonthYear, mo, n) {
                    n.add(3);
                    n.add(2);
                }
            }
            if mode == b"dmy" && y != -1 {
                year(n, 2);
            }
        }
        b"md" => {
            if mo != -1 {
                card.month(mo, n);
                n.add(2);
            }
            if d != -1 {
                day(n, 2);
            }
        }
        b"my" => {
            if mo != -1 {
                card.month(mo, n);
                n.add(2);
                if hook(Do::OfBetweenMonthYear, mo, n) {
                    n.add(3);
                    n.add(2);
                }
            }
            if y != -1 {
                year(n, 2);
            }
        }
        b"y" => {
            if y != -1 {
                if hook(Do::YearSplit, y, n) {
                    card.number(y / 100, z, n);
                    let p = hook(Do::Plural, y, n);
                    card.number(y % 100, Flags { plural: p, ..z }, n);
                } else {
                    let p = hook(Do::Plural, y, n);
                    card.number(y, Flags { plural: p, ..z }, n);
                }
                n.add(2);
            }
        }
        b"m" => {
            if mo != -1 {
                card.month(mo, n);
                n.add(2);
            }
        }
        b"d" => {
            if d != -1 {
                day(n, 2);
            }
        }
        _ => {
            if d != -1 {
                day(n, 1);
            }
            if y != -1 {
                year(n, 1);
            }
            if mo != -1 {
                card.month(mo, n);
                n.add(2);
            }
        }
    }
}

/// 0x1003d2d0: an amount: the units and the currency's name (+5 unless one;
/// 0x42d after one adds 0x20), the cents (1–2 digits: "and", the cents and
/// 0x427/0x428; more: the separator and the decimals); an amount between a
/// one-letter currency and a number before it reads as a plain number.
pub fn read_amount<M: NumModule + ?Sized>(card: &Card<M>, n: &mut Num, c: &Ctx) {
    let z = Flags::default();
    let t = card.m.tables();
    let mut int = n.i(off::I0);
    let dec = n.s(off::S_DECIMALS);
    let cur = n.i(off::CURRENCY);
    let (next, prev, prev2) = (c.tok(1), c.tok(-1), c.tok(-2));
    let mut plain = false;
    if t.currency_of(next) == NO_CURRENCY && t.currency_of(prev) != NO_CURRENCY && prev.len() < 2 {
        let first = prev2.first().copied().unwrap_or(0);
        plain = digit(first) || first == b'-';
        if plain {
            for &ch in prev2.iter().skip(1) {
                if !(digit(ch) || ch == b'.' || ch == b',') {
                    plain = false;
                }
            }
        }
    }
    if int < 0 {
        n.add(9);
        n.add(2);
        int = int.wrapping_neg();
    }
    let sep = n.i(off::I2);
    let cents = |v: i32, n: &mut Num| {
        if v == 1 {
            n.add(0x19);
            n.add(2);
            n.add(0x427);
        } else {
            card.number(v, z, n);
            n.add(2);
            n.add(0x428);
        }
    };
    let short = |d: &[u8]| -> i32 {
        if d.len() == 1 {
            atol(d) * 10
        } else {
            atol(d)
        }
    };
    if plain {
        card.number(int, z, n);
        if !dec.is_empty() {
            n.add(2);
            n.add(sep);
            n.add(2);
            if dec.len() <= 3 && dec[0] != b'0' {
                card.number(atol(&dec), z, n);
                n.add(2);
            } else {
                for &ch in &dec {
                    spell(ch, n);
                    n.add(2);
                }
            }
        }
        return;
    }
    if int == 0 {
        if atol(&dec) == 0 {
            n.add(0x18);
            n.add(2);
            n.add(cur + 5);
            return;
        }
        if dec.len() <= 3 && dec[0] != b'0' {
            if dec.len() > 2 {
                n.add(0x18);
                n.add(2);
                n.add(sep);
                n.add(2);
                card.number(atol(&dec), z, n);
                n.add(2);
                n.add(cur + 5);
            } else {
                cents(short(&dec), n);
                n.add(2);
                n.add(3);
                n.add(2);
                n.add(cur);
            }
        } else {
            n.add(0x18);
            n.add(2);
            n.add(sep);
            n.add(2);
            for &ch in &dec {
                spell(ch, n);
                n.add(2);
            }
            n.add(cur + 5);
        }
        return;
    }
    if int == 1 {
        n.add(0x19);
        if cur == 0x42d {
            n.add(0x20);
        }
    } else {
        card.number(int, z, n);
    }
    n.add(2);
    n.add(if int == 1 { cur } else { cur + 5 });
    if dec.is_empty() {
        return;
    }
    n.add(2);
    if dec.len() <= 3 && dec[0] != b'0' {
        if dec.len() > 2 {
            n.add(sep);
            n.add(2);
            card.number(atol(&dec), z, n);
            n.add(2);
        } else {
            n.add(0x16);
            n.add(2);
            cents(short(&dec), n);
            n.add(2);
        }
    } else {
        n.add(sep);
        n.add(2);
        for &ch in &dec {
            spell(ch, n);
            n.add(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::common::roman_value;

    #[test]
    fn strings_overlap_as_in_util() {
        let mut n = Num::default();
        n.set_s(off::S_DECIMALS, b"50");
        n.set_big(b"12345678901");
        // the big value's strncpy clears the decimals it covers
        assert_eq!(n.s(off::S_DECIMALS), b"5678901");
        assert_eq!(n.i(off::I1), 1);
        n.set_integer(7);
        assert_eq!((n.i(off::I0), n.i(off::I1)), (7, 0));
    }

    #[test]
    fn roman_numerals_must_be_canonical() {
        assert_eq!(roman_value(b"XXI"), 21);
        assert_eq!(roman_value(b"xiv"), 14);
        assert_eq!(roman_value(b"MCMXCIX"), 1999);
        assert_eq!(roman_value(b"IIII"), 0);
        assert_eq!(roman_value(b"VX"), 0);
        assert_eq!(roman_value(b"Mix"), 1009);
        assert_eq!(roman_value(b"MMMM"), 0);
    }
}
