//! The engine-wide number readers of `LTTS7Util.dll` (`ELQNum*`), which
//! every language module's own number readers call (MECHANICS.md §4b).
//!
//! Strings are the engine's Windows-1252 bytes without their NUL. Where the
//! C function writes through a `char *` into a caller's buffer, the port
//! takes the whole buffer and the offset that pointer has in it (`at`): the
//! C string is `out[at..]`, and what the function appends goes at the end.
//! [`add_separator`] looks at that string's tail, so the offset matters.
//!
//! The `ELQNumConvert*` functions call back into the language module through
//! the table its `LesIniNumeri` fills ([`NumSlots`]).

/// The language module's `LesIniNumeri` table as LTTS7Util uses it (slots
/// 0–6; slots 7 and 8 exist but only the module's own code calls them, and
/// slot 9 is the reader the engine calls). Every `out` is the whole output
/// buffer: the slot's C `char *` is its end at the call, so a slot appends.
pub trait NumSlots {
    /// Slot 0: the word (or separator) numbered `id`, `""` when the module
    /// has none. Util asks for 0 (the separator between pieces when a pause
    /// is wanted, ` # ` in English), 1 (the plain separator, a space), 3 (the
    /// decimal point), 4 (the decimal comma), 7 (minus), 8 (plus), 0xd
    /// ("and" between a currency's units and cents) and 0xf (zero).
    fn word(&self, id: u32) -> &[u8];
    /// Slot 1: a digit string read digit by digit. [`convert_real_number`]
    /// passes `flag` 1 for every piece of a mode-1 number, 0 for a long
    /// last piece. (The C slot also gets the table itself.)
    fn digits(&self, out: &mut Vec<u8>, digits: &[u8], flag: bool);
    /// Slot 2: the cardinal reading of a digit string (it may start with
    /// zeros; [`convert_integer`] hands it the string it was given).
    fn cardinal(&self, out: &mut Vec<u8>, digits: &[u8]);
    /// Slot 3: the word of one digit character.
    fn digit(&self, d: u8) -> &[u8];
    /// Slot 4: whether [`convert_integer`] reads its digits one by one
    /// (true) or as a cardinal (slot 2): `flag` is `convert_integer`'s own
    /// flag, `len` the digit count after the leading zeros and `lead0`
    /// whether there were leading zeros ([`set_pauses`]).
    fn by_digits(&self, flag: bool, len: usize, lead0: bool) -> bool;
    /// Slot 5: the currency's name after an amount ([`convert_currency`]):
    /// `amount` is the digits just read, `currency` the currency (the code
    /// the caller passed), `cents` whether they are the cents.
    fn currency(&self, out: &mut Vec<u8>, amount: &[u8], currency: &[u8], cents: bool);
    /// Slot 6: decimals read after the decimal point ([`convert_currency`]
    /// passes `flag` false). Returns the C function's `char`.
    fn decimals(&self, out: &mut Vec<u8>, digits: &[u8], flag: bool) -> bool;
}

/// The 0xd4-byte object the language module's number recognizers fill and
/// its renderer reads; it travels by value. Util knows `+0` (the kind of
/// number), `+4` (the text, 64 bytes), `+0x44` (the reading mode) and
/// `+0x48`; the rest belongs to the module, so the bytes are kept as they
/// are, with accessors.
#[derive(Clone, PartialEq, Eq)]
pub struct NumObj(pub [u8; 0xd4]);

impl Default for NumObj {
    fn default() -> Self {
        NumObj([0; 0xd4])
    }
}

impl std::fmt::Debug for NumObj {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NumObj(kind {} mode {} {:?})", self.kind(), self.mode(), String::from_utf8_lossy(self.text()))
    }
}

impl NumObj {
    pub fn u32(&self, o: usize) -> u32 {
        u32::from_le_bytes(self.0[o..o + 4].try_into().unwrap())
    }
    pub fn set_u32(&mut self, o: usize, v: u32) {
        self.0[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
    pub fn u8(&self, o: usize) -> u8 {
        self.0[o]
    }
    pub fn set_u8(&mut self, o: usize, v: u8) {
        self.0[o] = v;
    }
    /// The NUL-terminated string at `o` (within the object).
    pub fn cstr(&self, o: usize) -> &[u8] {
        let b = &self.0[o..];
        &b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())]
    }
    /// C `strcpy` into the object at `o` (cut at its end, NUL included
    /// when it fits).
    pub fn set_cstr(&mut self, o: usize, s: &[u8]) {
        let room = self.0.len() - o;
        let n = s.len().min(room);
        self.0[o..o + n].copy_from_slice(&s[..n]);
        if n < room {
            self.0[o + n] = 0;
        }
    }
    /// `+0`: the kind of number, the renderer's switch
    pub fn kind(&self) -> u32 {
        self.u32(0)
    }
    /// `+4`: the token's text
    pub fn text(&self) -> &[u8] {
        self.cstr(4)
    }
    /// `+0x44`: the reading mode
    pub fn mode(&self) -> u32 {
        self.u32(0x44)
    }
}

/// The C string at the start of `s` (up to its first NUL).
fn cs(s: &[u8]) -> &[u8] {
    &s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())]
}

/// The CRT's `atol` (Util 0x10043a08): blanks, a sign, digits; wraps.
pub fn atol(s: &[u8]) -> i32 {
    let s = cs(s);
    let mut i = 0;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let neg = i < s.len() && s[i] == b'-';
    if i < s.len() && (s[i] == b'-' || s[i] == b'+') {
        i += 1;
    }
    let mut v: i32 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        v = v.wrapping_mul(10).wrapping_add((s[i] - b'0') as i32);
        i += 1;
    }
    if neg {
        v.wrapping_neg()
    } else {
        v
    }
}

/// `ELQstrtok` (Util 0x100288b0) over a buffer and a position: the token at
/// `*pos` up to the first delimiter, the run of delimiters after it set to
/// NUL and skipped. `None` at the end. Unlike C `strtok` a delimiter at
/// `*pos` makes an empty token.
pub fn strtok(buf: &mut [u8], delims: &[u8], pos: &mut usize) -> Option<Vec<u8>> {
    let start = *pos;
    if buf.get(start).copied().unwrap_or(0) == 0 {
        return None;
    }
    let mut p = start;
    loop {
        if delims.contains(&buf[p]) {
            while p < buf.len() && buf[p] != 0 && delims.contains(&buf[p]) {
                buf[p] = 0;
                p += 1;
                *pos += 1;
            }
            break;
        }
        *pos += 1;
        p += 1;
        if p >= buf.len() || buf[p] == 0 {
            break;
        }
    }
    Some(cs(&buf[start..]).to_vec())
}

/// A C string as a NUL-terminated working buffer.
fn cbuf(s: &[u8], size: usize) -> Vec<u8> {
    let mut b = vec![0u8; size.max(s.len() + 1)];
    b[..s.len()].copy_from_slice(s);
    b
}

/// `ELQNumAddSeparator` (Util 0x10014360): appends `sep` to the string at
/// `out[at..]` unless that string already ends with it.
pub fn add_separator(out: &mut Vec<u8>, at: usize, sep: &[u8]) {
    let s = &out[at..];
    if sep.len() <= s.len() && &s[s.len() - sep.len()..] == sep {
        return;
    }
    out.extend_from_slice(sep);
}

/// `ELQNumSetPauses` (Util 0x100143f0): whether `digits` starts with
/// zeros, and whether its digits are read with pauses between them (more
/// than three significant digits, or a short number whose leading zeros
/// and first digit make more than three).
pub fn set_pauses(digits: &[u8]) -> (bool, bool) {
    let digits = cs(digits);
    let mut zeros = digits.iter().take_while(|&&c| c == b'0').count();
    let lead0 = zeros != 0;
    let rest = (digits.len() - zeros) & 0xffff;
    if rest < 4 {
        if rest != 0 {
            zeros += 1;
        }
        (lead0, zeros > 3)
    } else {
        (lead0, true)
    }
}

/// `ELQNumIsCurrency` (Util 0x10015100): starts with `$ £ € ¥`.
pub fn is_currency(s: &[u8]) -> bool {
    matches!(s.first(), Some(b'$' | 0xa3 | 0x80 | 0xa5))
}

/// `ELQNumIsTime` (Util 0x10013460): at most 8 characters, hours below 24
/// before the first `.,:`, then two (or five, `mm:ss`) characters of
/// minutes below 60, seconds below 60 after a further `.,:`.
pub fn is_time(s: &[u8]) -> bool {
    let s = cs(s);
    if s.len() > 8 {
        return false;
    }
    let Some(p) = s.iter().position(|c| b".,:".contains(c)) else {
        return false;
    };
    let (h, rest) = (&s[..p], &s[p + 1..]);
    if atol(h) as u32 > 23 || h.is_empty() {
        return false;
    }
    if rest.len() != 5 && rest.len() != 2 {
        return false;
    }
    if atol(rest) as u32 > 59 {
        return false;
    }
    if let Some(q) = rest.iter().position(|c| b".,:".contains(c)) {
        if atol(&rest[q + 1..]) as u32 > 59 {
            return false;
        }
    }
    true
}

/// `ELQNumIsBigInteger` (Util 0x10013570): digits grouped by threes with
/// dots (`1.234.567`), read from the right; the empty string passes.
pub fn is_big_integer(s: &[u8]) -> bool {
    let s = cs(s);
    let mut n = 0;
    for &c in s.iter().rev() {
        if n < 3 {
            if !c.is_ascii_digit() {
                return false;
            }
            n += 1;
        } else {
            if c != b'.' {
                return false;
            }
            n = 0;
        }
    }
    true
}

/// `ELQNumConvertInteger` (Util 0x10013ba0): the reading of the integer
/// `digits` appended to `out` (the C string at `at`). A trailing gender
/// mark (`m`/`f`, the first one found and what follows) is dropped. Leading
/// zeros are read one by one (slot 3); the rest is read by slot 2 or, when
/// slot 4 says so, digit by digit. Pieces read one by one are separated by
/// slot 0's word 0 when [`set_pauses`] wants pauses, word 1 otherwise.
/// Returns that pause flag.
pub fn convert_integer<S: NumSlots + ?Sized>(out: &mut Vec<u8>, at: usize, digits: &[u8], flag: bool, s: &S) -> bool {
    let digits = cs(digits);
    let mut len = digits.len();
    let mut local: Vec<u8> = digits[..digits.len().min(0x3f)].to_vec();
    if let Some(p) = local.iter().position(|&c| c == b'm').or_else(|| local.iter().position(|&c| c == b'f')) {
        len = len.wrapping_sub(1);
        local.truncate(p);
    }
    let (lead0, pause) = set_pauses(&local);
    let sep = s.word(if pause { 0 } else { 1 }).to_vec();
    let mut d = 0; // the digits' position in `local`
    if local.first() == Some(&b'0') {
        loop {
            let w = s.digit(b'0').to_vec();
            d += 1;
            out.extend_from_slice(&w);
            len = len.wrapping_sub(1);
            if local.get(d) != Some(&b'0') {
                break;
            }
            add_separator(out, at, &sep);
        }
    }
    if len != 0 {
        if lead0 {
            out.extend_from_slice(&sep);
        }
        if !s.by_digits(flag, len, lead0) {
            s.cardinal(out, digits);
        } else {
            let dig = |k: usize| local.get(d + k).copied().unwrap_or(0);
            for k in 1..len {
                let w = s.digit(dig(k - 1)).to_vec();
                out.extend_from_slice(&w);
                add_separator(out, at, &sep);
            }
            let w = s.digit(dig(len - 1)).to_vec();
            out.extend_from_slice(&w);
        }
    }
    pause
}

/// `ELQNumConvertBigInteger` (Util 0x100135d0): the groups of `digits`
/// between `,` and `.` joined and read by [`convert_integer`] (flag 0).
pub fn convert_big_integer<S: NumSlots + ?Sized>(out: &mut Vec<u8>, at: usize, digits: &[u8], s: &S) -> bool {
    let mut buf = cbuf(&cs(digits)[..cs(digits).len().min(0x40)], 0x41);
    let mut pos = 0;
    let mut joined = Vec::new();
    while let Some(t) = strtok(&mut buf, b",.", &mut pos) {
        joined.extend_from_slice(&t);
    }
    convert_integer(out, at, &joined, false, s)
}

/// `ELQNumConvertRealNumber` (Util 0x100136b0): `obj`'s text (a sign, then
/// pieces between `,.:/-+`) read into `out` (the C string at `at`). Mode
/// ([`NumObj::mode`]) 1 reads every piece by slot 1 with word 0 between;
/// otherwise each piece is an integer ([`convert_integer`], flag mode 2;
/// pieces after the first longer than four digits by slot 1), word 0
/// between pieces.
pub fn convert_real_number<S: NumSlots + ?Sized>(out: &mut Vec<u8>, at: usize, obj: &NumObj, s: &S) {
    let text = obj.text();
    let mode = obj.mode();
    let mut first = true;
    let body: &[u8] = match text.first() {
        Some(b'-') => {
            out.truncate(at);
            let w = s.word(7).to_vec();
            add_separator(out, at, &w);
            &text[1..]
        }
        Some(b'+') => {
            out.truncate(at);
            let w = s.word(8).to_vec();
            add_separator(out, at, &w);
            &text[1..]
        }
        _ => text,
    };
    let mut buf = cbuf(body, 0x41);
    let mut pos = 0;
    while let Some(tok) = strtok(&mut buf, b",.:/-+", &mut pos) {
        let more = buf.get(pos).copied().unwrap_or(0) != 0;
        if mode == 1 {
            s.digits(out, &tok, true);
            if more {
                let w = s.word(0).to_vec();
                add_separator(out, at, &w);
            }
            continue;
        }
        let flag = mode == 2;
        if !more || !first {
            if tok.len() > 4 {
                s.digits(out, &tok, false);
            } else {
                let end = out.len();
                convert_integer(out, end, &tok, flag, s);
            }
        } else {
            convert_integer(out, at, &tok, flag, s);
            first = false;
        }
        if more {
            let w = s.word(0).to_vec();
            add_separator(out, at, &w);
            // the delimiter before the position, which strtok has cleared
            let w = match buf[pos - 1] {
                b'+' => s.word(8).to_vec(),
                b',' => s.word(4).to_vec(),
                b'-' | b'/' | b':' => s.word(0).to_vec(),
                b'.' => s.word(3).to_vec(),
                _ => Vec::new(),
            };
            add_separator(out, at, &w);
        }
    }
}

/// `ELQNumConvertPhoneNumber` (Util 0x10013980), step `step` of reading a
/// telephone number. Step 0 clears `out`, reads the first `*len` digits as
/// the area code (an integer, flag set unless it is `0` + three digits
/// with a `1` second), word 0 after it, and sets `*len` to what is left.
/// Step 1 reads the `*len` digits of `digits`: by twos (a first three when
/// the count is odd), word 0 between, unless they end with `00` and
/// `pairs` is false, or there is one digit: then as one integer.
pub fn convert_phone_number<S: NumSlots + ?Sized>(s: &S, out: &mut Vec<u8>, digits: &[u8], pairs: bool, len: &mut usize, step: u32) {
    let mut d = cs(digits);
    if step == 0 {
        out.clear();
        if *len != 0 {
            let local = &d[..(*len).min(d.len())];
            let flag = (local.len() == 4 && local.get(1) != Some(&b'1')) || local.first() != Some(&b'0');
            convert_integer(out, 0, local, flag, s);
            let w = s.word(0).to_vec();
            add_separator(out, 0, &w);
            d = &d[(*len).min(d.len())..];
        }
        *len = d.len();
        return;
    }
    if step != 1 || *len == 0 {
        return;
    }
    let n = *len;
    if n != 1 {
        let tail = d.get(n - 2..).map(cs).unwrap_or(&[]);
        if tail != b"00" || pairs {
            if n & 1 != 0 {
                let three = &d[..3.min(d.len())];
                let end = out.len();
                convert_integer(out, end, three, true, s);
                let w = s.word(0).to_vec();
                add_separator(out, 0, &w);
                d = &d[3.min(d.len())..];
            }
            while !d.is_empty() {
                let two = &d[..2.min(d.len())];
                let end = out.len();
                convert_integer(out, end, two, false, s);
                if d.len() > 2 {
                    let w = s.word(0).to_vec();
                    add_separator(out, 0, &w);
                }
                d = &d[2.min(d.len())..];
            }
            return;
        }
    }
    let end = out.len();
    convert_integer(out, end, d, false, s);
}

/// `ELQNumConvertCurrency` (Util 0x10013e00): the amount `amount` of
/// `currency` read into `out` (appended): the units (slot 2, or
/// [`convert_big_integer`] for dot-grouped digits) and slot 5's name; with
/// exactly two decimals, word 1, "and" (0xd), word 1, the cents and slot
/// 5's cent name (only cents when the units are `0`, nothing for `00`);
/// other decimals after word 1, the point (3), word 1, by slot 6.
pub fn convert_currency<S: NumSlots + ?Sized>(s: &S, out: &mut Vec<u8>, amount: &[u8], currency: &[u8]) {
    let amount = cs(amount);
    let units_of = |out: &mut Vec<u8>, a: &[u8]| {
        if !is_big_integer(a) {
            s.cardinal(out, a);
        } else {
            let end = out.len();
            convert_big_integer(out, end, a, s);
        }
    };
    let Some(p) = amount.iter().position(|c| b",.".contains(c)) else {
        units_of(out, amount);
        s.currency(out, amount, currency, false);
        return;
    };
    let units = &amount[..p];
    let dec = &amount[p + 1..];
    if dec.len() == 2 {
        if units == b"0" {
            if dec[0] != b'0' {
                s.cardinal(out, dec);
                s.currency(out, dec, currency, true);
            } else {
                let w = s.digit(dec[1]).to_vec();
                out.extend_from_slice(&w);
                s.currency(out, &dec[1..], currency, true);
            }
            return;
        }
        units_of(out, units);
        s.currency(out, units, currency, false);
        if dec != b"00" {
            for id in [1, 0xd, 1] {
                let w = s.word(id).to_vec();
                out.extend_from_slice(&w);
            }
            let cents = if dec[0] == b'0' {
                let w = s.digit(dec[1]).to_vec();
                out.extend_from_slice(&w);
                &dec[1..]
            } else {
                s.cardinal(out, dec);
                dec
            };
            s.currency(out, cents, currency, true);
        }
        return;
    }
    if units == b"0" {
        let w = s.word(0xf).to_vec();
        out.extend_from_slice(&w);
    } else {
        units_of(out, units);
    }
    for id in [1, 3, 1] {
        let w = s.word(id).to_vec();
        out.extend_from_slice(&w);
    }
    s.decimals(out, dec, false);
    s.currency(out, dec, currency, false);
}

/// A date's parts as [`extract_date`] finds them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Date {
    pub day: u32,
    pub month: u32,
    pub year: u32,
    /// the character after the first part (the separator the second part
    /// must also have before it)
    pub sep: u8,
}

/// `ELQNumExtractDate` (Util 0x10014460) step 0: the parts of the date `s`
/// (at most 12 characters, parts between `-/.`) in the order `order`
/// (`mdy dmy ymd ydm ym yd my md dm y m d`, any case). The second part
/// counts only after the same separator as the first (and, for `ydm`, the
/// third). True when the month (if the order has one) is 1–12, the day
/// 1–31, the year at most 2100 (9999 when absent) and nothing is left over.
/// `date` keeps what was read either way.
pub fn extract_date(order: &[u8], s: &[u8], date: &mut Date) -> bool {
    let s = cs(s);
    date.day = 0;
    date.month = 0;
    date.year = 9999;
    if s.len() > 12 {
        return false;
    }
    let mut buf = vec![0u8; 12];
    let n = s.len().min(11);
    buf[..n].copy_from_slice(&s[..n]);
    let mut pos = 0usize;
    let ch = |pos: usize| s.get(pos.wrapping_sub(1)).copied().unwrap_or(0);
    let o = order.to_ascii_lowercase();
    let mut tok = |buf: &mut Vec<u8>, pos: &mut usize| strtok(buf, b"-/.", pos).map(|t| atol(&t) as u32);
    #[derive(Clone, Copy)]
    enum P {
        D,
        M,
        Y,
    }
    let set = |date: &mut Date, p: P, v: u32| match p {
        P::D => date.day = v,
        P::M => date.month = v,
        P::Y => date.year = v,
    };
    // (parts, which of them must follow the first part's separator)
    let (parts, checked): (&[P], &[bool]) = match &o[..] {
        b"mdy" => (&[P::M, P::D, P::Y], &[false, true, false]),
        b"dmy" => (&[P::D, P::M, P::Y], &[false, true, false]),
        b"ymd" => (&[P::Y, P::M, P::D], &[false, true, false]),
        b"ydm" => (&[P::Y, P::D, P::M], &[false, false, true]),
        b"ym" => (&[P::Y, P::M], &[false, false]),
        b"yd" => (&[P::Y, P::D], &[false, false]),
        b"my" => (&[P::M, P::Y], &[false, false]),
        b"md" => (&[P::M, P::D], &[false, false]),
        b"dm" => (&[P::D, P::M], &[false, false]),
        b"y" => (&[P::Y], &[false]),
        b"m" => (&[P::M], &[false]),
        b"d" => (&[P::D], &[false]),
        _ => return false,
    };
    if matches!(&o[..], b"md" | b"dm" | b"m" | b"d") {
        date.year = 0;
    }
    let three = parts.len() == 3;
    for (k, (&p, &chk)) in parts.iter().zip(checked).enumerate() {
        let Some(v) = tok(&mut buf, &mut pos) else {
            continue;
        };
        if three && k == 0 {
            set(date, p, v);
            date.sep = ch(pos);
        } else if chk {
            if ch(pos) == date.sep {
                set(date, p, v);
            }
        } else {
            set(date, p, v);
        }
    }
    let has = |c: u8| o.contains(&c);
    if (date.month == 0 || date.month > 12) && has(b'm') {
        return false;
    }
    if (date.day == 0 || date.day > 31) && has(b'd') {
        return false;
    }
    if date.year > 2100 {
        return false;
    }
    s.get(pos).copied().unwrap_or(0) == 0
}

/// `ELQNumExtractDate` (Util 0x10014460) step 1: a two-digit year at least
/// `min_year` becomes 19xx (20xx below 20); false for a year before 1600
/// when the order has one, February 30–31 and the 31st of a 30-day month.
pub fn validate_date(order: &[u8], date: &mut Date, min_year: u32) -> bool {
    let y = date.year;
    if min_year <= y && y < 100 {
        date.year = if y < 20 { 2000 } else { 1900 } + y;
    }
    if date.year < 1600 && order.to_ascii_lowercase().contains(&b'y') {
        return false;
    }
    if date.day >= 30 && date.month == 2 {
        return false;
    }
    !(date.day == 31 && matches!(date.month, 4 | 6 | 9 | 11))
}

/// `ELQNumFormatVXML10Date` (Util 0x10014ac0): an 8-digit `YYYYMMDD` date
/// (month at most 12, day at most 31 and possible in that month). Like the
/// C, `date` is cleared first and gets each part as it is read, so a failed
/// date keeps what was read before the failure.
pub fn format_vxml10_date(s: &[u8], date: &mut Date) -> bool {
    let s = cs(s);
    date.day = 0;
    date.month = 0;
    date.year = 0;
    if s.len() != 8 {
        return false;
    }
    let digits = |p: &[u8]| p.iter().take_while(|c| c.is_ascii_digit()).count() == p.len();
    if !digits(&s[..4]) {
        return false;
    }
    date.year = atol(&s[..4]) as u32;
    if !digits(&s[4..6]) {
        return false;
    }
    date.month = atol(&s[4..6]) as u32;
    if !digits(&s[6..8]) {
        return false;
    }
    date.day = atol(&s[6..8]) as u32;
    let (day, month) = (date.day, date.month);
    !(month > 12 || day > 31 || (day >= 30 && month == 2) || (day == 31 && matches!(month, 4 | 6 | 9 | 11)))
}

/// `ELQNumSetDefaultType` (Util 0x100150d0): `obj` becomes a plain number
/// (kind 0) of `text`, `+0x48` cleared.
pub fn set_default_type(obj: &mut NumObj, text: &[u8]) {
    obj.set_u32(0, 0);
    obj.set_cstr(4, cs(text));
    obj.set_u32(0x48, 0);
}

/// `ELQisnumber` (Util 0x10013400): a non-empty string of digits, or of
/// digits and characters of `also` (unless `digits_only`).
pub fn is_number(s: &[u8], also: &[u8], digits_only: bool) -> bool {
    let s = cs(s);
    !s.is_empty() && s.iter().all(|c| c.is_ascii_digit() || (!digits_only && also.contains(c)))
}
