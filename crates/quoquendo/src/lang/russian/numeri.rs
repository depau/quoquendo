//! Russian numbers: LTTS7Russian.dll's hooks into LoqTTS7.dll's generic
//! number reader (`loqnum`, MECHANICS.md §4n). The module classifies the
//! number (LesClassifyFunc 0x10004860) and reads every type itself
//! (LesRead* 0x10005f70…0x10007d70) with its own copy of the engine's
//! cardinal builder (0x10004d40…0x100059e0: a separator morph after a
//! bare group, the Russian forms of 2, the hundreds and the thousands);
//! the engine parses and turns the morphs into words with the module's
//! morph table (0x100a2e30).

use super::super::loqnum::{self as q, spell, stricmp, Card, Ctx, Do, Flags, Kind, Num, NumModule, Tables, NO_CURRENCY};
use super::super::textfilter::Classes;
use super::super::common::{Pe, Tok};
use super::Russian;
use q::off;

/// `LesGetMorphTable` 0x100059c0, `LesGetClassifyTable` 0x100059a0,
/// `LesGetCurrencyTable` 0x100059b0, `LesGetOrdinalStringTable` 0x100059d0.
pub(super) const MORPHS: u32 = 0x100a2e30;
pub(super) const CLASSIFY: u32 = 0x100a2218;
pub(super) const CURRENCY: u32 = 0x100a2258;
pub(super) const ORDINALS: u32 = 0x100a2e40;
/// Month names (0x2b `{name, month}`, 0x100047d0).
const MONTHS: u32 = 0x100675e8;
/// `году год^у год годах г. г года` (after the number) and `году год^у`
/// (before it), 0x10067748 ×7 and 0x10067740 ×2.
const YEAR_AFTER: u32 = 0x10067748;
const YEAR_BEFORE: u32 = 0x10067740;
/// The modes the classifier sets: `d` 0x10067780, `y` 0x10067778; the
/// separators `-` 0x1006777c, `.` 0x10067774, `/` 0x10067770.
const MODE_D: u32 = 0x10067780;
const MODE_Y: u32 = 0x10067778;
const DASH: u32 = 0x1006777c;
const DOT: u32 = 0x10067774;
const SLASH: u32 = 0x10067770;
/// LesDoGender's `ext` reading mode (0x10067764).
const EXT: u32 = 0x10067764;

impl Russian {
    pub(super) fn leggi_numero(&self, toks: &mut [Tok], i: usize, kind: u32, dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
        Some(q::leggi_numero(self, toks, i, kind, dmy))
    }

    /// 0x100047d0: the month a word names, 0 for none (a lone `./-+:` too).
    fn month_of(&self, s: &[u8]) -> i32 {
        if s.len() == 1 && b"./-+:".contains(&s[0]) {
            return 0;
        }
        (0..0x2b).find(|&k| stricmp(self.s(self.pe.u32(MONTHS + 8 * k)), s)).map(|k| self.pe.u32(MONTHS + 8 * k + 4) as i32).unwrap_or(0)
    }

    fn word_in(&self, list: u32, n: u32, s: &[u8]) -> bool {
        (0..n).any(|k| stricmp(s, self.s(self.pe.u32(list + 4 * k))))
    }

    fn card(&self) -> Card<'_, Russian> {
        Card { m: self, neuter_one: false }
    }

    /// 0x10004d40: a digit; `p9` 3 reads a 2 as morph 0x6b (two thousand).
    fn digit(&self, d: i32, f: Flags, p9: i32, n: &mut Num) {
        let c = self.card();
        let s = c.stem(d, f, n);
        let e = c.ending(d, f, n);
        let id = match d {
            0 => 0x18,
            2 if p9 == 3 => s + 0x6b,
            1..=9 => {
                n.add(s + q::unit(d));
                if self.do_hook(Do::Truncation, d, n) & 0xff != 0 {
                    n.add(2);
                    return;
                }
                e + q::unit(d) + 3
            }
            _ => return,
        };
        n.add(id);
        n.add(2);
    }

    /// 0x10005080: 0–99, as the engine's 0x1003aed0.
    fn tens(&self, v: i32, f: Flags, p9: i32, n: &mut Num) {
        let c = self.card();
        let s = c.stem(v, f, n);
        let e = c.ending(v, f, n);
        if v < 10 {
            return self.digit(v, f, p9, n);
        }
        let tens_stem = |t: i32| if t == 2 { 0x337 } else { 0x34c + 3 * (t - 3) };
        let tens_ending = |t: i32| if t == 2 { 0x33a } else { 0x361 };
        if self.do_hook(Do::TenthInversion, v, n) & 0xff != 0 {
            if v % 10 != 0 {
                self.digit(v % 10, Flags { comp: true, ..f }, p9, n);
            }
            if self.do_hook(Do::AndBetweenTenthsUnits, v, n) & 0xff != 0 {
                n.add(0x16);
                n.add(2);
            }
            match v / 10 {
                1 => {
                    n.add(s + 0x193);
                    n.add(e + 0x196);
                }
                t @ 2..=9 => {
                    n.add(s + tens_stem(t));
                    n.add(e + tens_ending(t));
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
        let plain = Flags { ord: false, alt: false, ..f };
        let (ls, le) = if self.do_hook(Do::OrdinalTenth, v, n) & 0xff != 0 {
            (c.stem(v, f, n), c.ending(v, f, n))
        } else {
            (c.stem(v, plain, n), c.ending(v, plain, n))
        };
        let t = v / 10;
        n.add(ls + tens_stem(t));
        if v % 10 == 0 {
            n.add(le + tens_ending(t));
            n.add(2);
            return;
        }
        if self.do_hook(Do::TruncateTenth, v, n) & 0xff == 0 {
            n.add(le + tens_ending(t));
            n.add(2);
            if self.do_hook(Do::AndBetweenTenthsUnits, v, n) & 0xff != 0 {
                n.add(0x16);
                n.add(2);
            }
        }
        self.digit(v % 10, Flags { comp: true, ..f }, p9, n);
    }

    /// 0x100056c0: 0–999 (200 is one morph, 0x397; 300 and 400 take
    /// 0x398, 500–900 0x386/0x387).
    fn hundreds(&self, v: i32, f: Flags, p9: i32, n: &mut Num) {
        if v == 0 {
            n.add(0x18);
            return;
        }
        if v < 100 {
            return self.tens(v, f, p9, n);
        }
        let c = self.card();
        let r = v % 100;
        let (ls, e) = if r != 0 {
            (0, c.ending(v, Flags { ord: false, alt: false, ..f }, n))
        } else {
            (c.stem(v, f, n), c.ending(v, f, n))
        };
        match v / 100 {
            1 => {
                n.add(ls + 0x373);
                n.add(e + 0x374);
            }
            2 => n.add(0x397),
            h @ (3 | 4) => {
                n.add(q::unit(h));
                n.add(ls + 0x398);
            }
            h @ 5..=9 => {
                n.add(q::unit(h));
                n.add(ls + 0x386);
                n.add(e + 0x387);
            }
            _ => {}
        }
        n.add(2);
        if self.do_hook(Do::AndBetweenHundredsTenths, v, n) & 0xff != 0 {
            n.add(0x16);
            n.add(2);
        }
        if r != 0 {
            self.tens(r, Flags { comp: true, ..f }, p9, n);
        }
    }

    /// 0x100059e0: any int; `sep` is the morph after a group (and before
    /// the hundreds when there are no thousands).
    pub(super) fn number(&self, v: i32, f: Flags, sep: i32, n: &mut Num) {
        let c = self.card();
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
        let group = |r: i32, g: i32, size: i32, n: &Num| -> i32 {
            let plural = g != 1;
            if r == g * size {
                c.ending(r, Flags { plural, comp: false, ..f }, n)
            } else {
                c.ending(r, Flags { ord: false, alt: false, plural, comp: false, ..f }, n)
            }
        };
        let b = v / 1_000_000_000;
        if b != 0 {
            let e = group(v, b, 1_000_000_000, n);
            local = true;
            match b {
                1 => {
                    n.add(0x3e5);
                    n.add(e + 0x3e6);
                }
                2..=4 => {
                    n.add(q::unit(b));
                    n.add(0x40a);
                }
                _ => {}
            }
            n.add(sep);
        }
        let r = v % 1_000_000_000;
        let m = r / 1_000_000;
        if m != 0 {
            let e = group(r, m, 1_000_000, n);
            local = true;
            if m == 1 {
                n.add(0x3bf);
                n.add(e + 0x3c0);
            } else {
                self.hundreds(m, Flags::default(), 6, n);
                n.add(match m % 10 {
                    1 => 0x3e3,
                    2..=4 => 0x3e4,
                    _ => 0x3d2,
                });
            }
            n.add(sep);
        }
        let r = r % 1_000_000;
        let t = r / 1000;
        if t == 0 {
            n.add(sep);
        } else {
            let e = group(r, t, 1000, n);
            local = true;
            if t == 1 {
                n.add(0x399);
                n.add(e + 0x39a);
                n.add(2);
            } else {
                self.hundreds(t, Flags::default(), 3, n);
                n.add(match t % 10 {
                    1 => 0x3bd,
                    2..=4 => 0x3be,
                    _ => 0x3ac,
                });
            }
            n.add(2);
            if self.do_hook(Do::AndBetweenThousandsHundreds, r, n) & 0xff != 0 {
                n.add(0x16);
                n.add(2);
            }
        }
        if r % 1000 != 0 {
            self.hundreds(r % 1000, Flags { comp: local, ..f }, 0, n);
        }
    }

    /// 0x10006b10: digits by pairs (a last three together), a `0` or a last
    /// digit spelled, each piece followed by `sep`.
    fn pairs(&self, s: &[u8], sep: i32, n: &mut Num) {
        let mut p = 0;
        while p < s.len() {
            let left = s.len() - p;
            if s[p] == b'0' || left == 1 {
                spell(s[p], n);
                p += 1;
            } else if left == 3 {
                self.number(q_atol(&s[p..p + 3]), Flags::default(), sep, n);
                p += 3;
            } else {
                self.number(q_atol(&s[p..p + 2]), Flags::default(), sep, n);
                p += 2;
            }
            n.add(sep);
        }
    }

    /// 0x10006e60: a month's name, else the number.
    fn month(&self, v: i32, n: &mut Num) {
        if (1..=12).contains(&v) {
            n.add(0x40a + v);
        } else {
            self.number(v, Flags::default(), 2, n);
        }
    }

    fn hook(&self, h: Do, v: i32, n: &Num) -> bool {
        self.do_hook(h, v, n) & 0xff != 0
    }

    /// 0x10006c40: the engine's 0x1003c580 with the module's builder.
    fn time(&self, n: &mut Num, h: i32, m: i32, s: i32) {
        let z = Flags::default();
        if h != -1 {
            let fem = self.do_hook(Do::FeminineHour, h, n) & 0xff;
            self.number(h, Flags { gender: fem, ..z }, 2, n);
            if self.hook(Do::OClock, h, n) {
                n.add(0x420);
                n.add(2);
            }
            if self.hook(Do::Hour, h, n) {
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
                self.number(m, z, 2, n);
            } else {
                if h != -1 {
                    n.add(1);
                }
                self.number(m, z, 2, n);
                n.add(2);
                if self.hook(Do::Minutes, m, n) {
                    n.add(if m != 1 { 0x418 } else { 0x41b });
                    n.add(1);
                }
                n.add(0x17);
                n.add(2);
            }
        }
        if s > 0 {
            self.number(s, z, 2, n);
            n.add(2);
            if self.hook(Do::Seconds, s, n) {
                n.add(if s != 1 { 0x417 } else { 0x41a });
                n.add(1);
            }
        }
    }

    /// 0x10006f90: a date in the reading mode's order. Days are ordinals
    /// when LesDoOrdinalDay says so; an unsplit year is an ordinal
    /// (genitive with `dmy` and `y`).
    fn date(&self, n: &mut Num, d: i32, mo: i32, y: i32, mode: &[u8]) {
        let z = Flags::default();
        let day = |n: &mut Num| {
            let od = self.hook(Do::OrdinalDay, d, n);
            self.number(d, Flags { ord: od, ..z }, 2, n);
            n.add(2);
        };
        // the year: split in centuries, else `whole`
        let year = |n: &mut Num, whole: Flags| {
            if self.hook(Do::YearSplit, y, n) {
                self.number(y / 100, z, 2, n);
                self.number(y % 100, z, 2, n);
            } else {
                self.number(y, whole, 2, n);
            }
            n.add(2);
        };
        let ord = Flags { ord: true, ..z };
        let ord_gen = Flags { ord: true, gender: 1, ..z };
        let month = |n: &mut Num| {
            if mo != -1 {
                self.month(mo, n);
                n.add(2);
            }
        };
        let of = |n: &mut Num, h: Do, v: i32| {
            if self.hook(h, v, n) {
                n.add(3);
                n.add(2);
            }
        };
        match mode {
            b"mdy" | b"ymd" | b"dym" | b"md" | b"my" | b"dm" | b"dmy" | b"d" | b"m" | b"y" => {}
            _ => {
                if d != -1 {
                    day(n);
                }
                if y != -1 {
                    year(n, z);
                }
                month(n);
                return;
            }
        }
        match mode {
            b"mdy" => {
                month(n);
                if d != -1 {
                    day(n);
                }
                if y != -1 {
                    year(n, ord);
                }
            }
            b"ymd" => {
                if y != -1 {
                    year(n, ord);
                }
                month(n);
                if d != -1 {
                    day(n);
                }
            }
            b"dym" => {
                if d != -1 {
                    day(n);
                }
                if y != -1 {
                    year(n, ord);
                }
                month(n);
            }
            b"dmy" => {
                if d != -1 {
                    day(n);
                    of(n, Do::OfBetweenDayMonth, d);
                }
                if mo != -1 {
                    month(n);
                    of(n, Do::OfBetweenMonthYear, mo);
                }
                if y != -1 {
                    year(n, ord_gen);
                }
            }
            b"md" => {
                month(n);
                if d != -1 {
                    day(n);
                }
            }
            b"dm" => {
                if d != -1 {
                    day(n);
                    of(n, Do::OfBetweenDayMonth, d);
                }
                month(n);
            }
            b"my" => {
                if mo != -1 {
                    month(n);
                    of(n, Do::OfBetweenMonthYear, mo);
                }
                if y != -1 {
                    year(n, ord);
                }
            }
            b"y" => {
                if y != -1 {
                    if self.hook(Do::YearSplit, y, n) {
                        self.number(y / 100, z, 2, n);
                        let p = self.hook(Do::Plural, y, n);
                        self.number(y % 100, Flags { plural: p, ..z }, 2, n);
                    } else {
                        self.number(y, ord_gen, 2, n);
                    }
                    n.add(2);
                }
            }
            b"m" => month(n),
            _ => {
                if d != -1 {
                    day(n);
                }
            }
        }
    }

    /// 0x10006330 (LesReadReale with the default reading mode) and
    /// LesReadMisura 0x10006890: a sign, the integer, the separator and the
    /// decimals (a number when 1–3 digits not starting with 0, else
    /// spelled), the exponent.
    fn real(&self, n: &mut Num, text: &[u8]) {
        let z = Flags::default();
        let mut int = n.i(off::I0);
        let dec = n.s(off::S_DECIMALS);
        let mut exp = n.i(off::I1);
        if int < 0 || (int == 0 && text.first() == Some(&b'-')) {
            n.add(9);
            n.add(2);
            int = int.wrapping_neg();
        }
        self.number(int, z, 2, n);
        if !dec.is_empty() {
            n.add(2);
            n.add(n.i(off::I2));
            n.add(2);
            if dec[0] != b'0' && dec.len() < 4 {
                self.number(q_atol(&dec), z, 2, n);
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
            self.number(exp, z, 2, n);
        }
    }
}

fn q_atol(s: &[u8]) -> i32 {
    super::super::elqnum::atol(s)
}

impl NumModule for Russian {
    fn tables(&self) -> &Tables {
        &self.num_tables
    }
    fn classes(&self) -> Option<&Classes> {
        self.classes.as_ref()
    }
    fn engine(&self) -> Option<&Pe> {
        self.engine.as_ref()
    }

    /// LesClassifyFunc 0x10004860: the classify table's escape (a cardinal
    /// followed by an ordinal-table string uses it up); a cardinal or real
    /// next to a currency is money (9); then dates: before a month name (`d`),
    /// a four-digit number before or after a word for "year" (`y`), after a
    /// month name (`y` if four digits, else `d` unless it starts with 0
    /// after a `- . /`).
    fn classify(&self, n: &mut Num, c: &Ctx) -> i32 {
        let t = &self.num_tables;
        let mut id = 0;
        if let Some(e) = q::classify_by_table(self, &c.text) {
            id = e;
            let mut check_money = id == 6;
            if id == 3 {
                check_money = true;
                if let Some(o) = t.ordinal.iter().find(|(_, s)| stricmp(s, c.tok(1))) {
                    n.set_skip(1);
                    if o.0 != 0 && o.0 != 3 {
                        id = o.0;
                        check_money = id == 6;
                    }
                }
            }
            if check_money && (t.currency_of(c.tok(1)) != NO_CURRENCY || t.currency_of(c.tok(-1)) != NO_CURRENCY) {
                return 9;
            }
            if id != 3 {
                return id;
            }
        }
        let (n1, p1, p2) = (c.tok(1), c.tok(-1), c.tok(-2));
        let four = c.text.len() == 4 && c.text.iter().all(u8::is_ascii_digit);
        let set = |n: &mut Num, mode: u32| {
            let m = self.s(mode).to_vec();
            n.set_modes(&m, &m);
            8
        };
        if (1..=12).contains(&self.month_of(n1)) {
            return set(n, MODE_D);
        }
        if self.word_in(YEAR_AFTER, 7, n1) && four {
            return set(n, MODE_Y);
        }
        if c.focus == 0 {
            return id;
        }
        if self.word_in(YEAR_BEFORE, 2, p1) && four {
            return set(n, MODE_Y);
        }
        let after_sep = p1 == self.s(DASH) || p1 == self.s(DOT) || p1 == self.s(SLASH);
        let before = if after_sep { p2 } else { p1 };
        if (1..=12).contains(&self.month_of(before)) {
            if four {
                return set(n, MODE_Y);
            }
            if after_sep && c.text.first() == Some(&b'0') {
                return id;
            }
            return set(n, MODE_D);
        }
        id
    }

    fn read(&self, k: Kind, n: &mut Num, c: &Ctx) {
        let z = Flags::default();
        match k {
            // LesReadTelefonico 0x10007960
            Kind::Telefonico => {
                for at in [off::S_CODE, off::S_AREA] {
                    let s = n.s(at);
                    if !s.is_empty() {
                        for &ch in &s {
                            spell(ch, n);
                            n.add(2);
                        }
                        n.add(2);
                    }
                }
                let s = n.s(off::S_NUMBER);
                if !s.is_empty() {
                    self.pairs(&s, 2, n);
                }
            }
            // LesReadCodice 0x10007a70 (the default reading mode: not 0x1f,
            // which spells)
            Kind::Codice => {
                let s = n.s(off::S_CODE);
                let mut p = 0;
                while p < s.len() {
                    let run: Vec<u8> = s[p..].iter().take_while(|c| c.is_ascii_digit()).copied().collect();
                    if run.is_empty() {
                        p += 1;
                        continue;
                    }
                    p += run.len();
                    if run.len() < 5 && run[0] != b'0' {
                        self.number(q_atol(&run), z, 1, n);
                    } else {
                        self.pairs(&run, 1, n);
                    }
                    n.add(1);
                }
            }
            // LesReadIntero 0x10007c30
            Kind::Intero => {
                if n.i(off::I1) == 0 {
                    self.number(n.i(off::I0), z, 2, n);
                } else {
                    self.pairs(&n.s(off::S_BIG), 2, n);
                }
            }
            // LesReadOrdinale 0x10005f70
            Kind::Ordinale => {
                let gender = if n.typ() == 5 { 2 } else { 1 };
                self.number(n.i(off::I0), Flags { ord: true, gender, ..z }, 2, n);
            }
            Kind::Reale | Kind::Misura => self.real(n, &c.text),
            // LesReadValuta 0x10006550
            Kind::Valuta => {
                let dec = n.s(off::S_DECIMALS);
                self.number(n.i(off::I0), z, 2, n);
                n.add(2);
                if !dec.is_empty() {
                    n.add(n.i(off::I2));
                    if dec.len() < 4 && dec[0] != b'0' {
                        self.number(q_atol(&dec), z, 2, n);
                        n.add(2);
                    } else {
                        n.add(2);
                        for &ch in &dec {
                            spell(ch, n);
                            n.add(2);
                        }
                    }
                }
            }
            // LesReadOra 0x10007cc0
            Kind::Ora => {
                let (h, m, s) = (n.i(off::I0), n.i(off::I1), n.i(off::I2));
                let (he, me, se) = (n.i(off::I3), n.i(off::I4), n.i(off::I5));
                self.time(n, h, m, s);
                if he != -1 || me != -1 || se != -1 {
                    n.add(2);
                    self.time(n, he, me, se);
                }
            }
            // LesReadData 0x10007d70
            Kind::Data => {
                let (d, mo, y) = (n.i(off::I0), n.i(off::I1), n.i(off::I2));
                let (de, me, ye) = (n.i(off::I3), n.i(off::I4), n.i(off::I5));
                let mode = n.reading_mode();
                self.date(n, d, mo, y, &mode);
                if de != -1 || me != -1 || ye != -1 {
                    n.add(2);
                    self.date(n, de, me, ye, &mode);
                }
            }
            // LesReadFrazione 0x10006680
            Kind::Frazione => {
                let (mut a, b) = (n.i(off::I0), n.i(off::I1));
                if a < 0 {
                    n.add(9);
                    n.add(2);
                    a = a.wrapping_neg();
                }
                self.number(a, Flags { gender: 1, ..z }, 2, n);
                n.add(2);
                n.add(0xf);
                n.add(2);
                self.number(b, Flags { gender: 1, ..z }, 2, n);
                n.add(2);
            }
            // LesReadAngolare 0x10006730
            Kind::Angolare => {
                let (mut d, mi, s) = (n.i(off::I0), n.i(off::I1), n.i(off::I2));
                let mut degrees = d != 0;
                if d < 0 || (d == 0 && c.text.first() == Some(&b'-')) {
                    n.add(9);
                    n.add(2);
                    d = d.wrapping_neg();
                    degrees = d != 0;
                }
                for (v, one, many, read) in [(d, 0x421, 0x424, degrees), (mi, 0x422, 0x425, mi != 0), (s, 0x423, 0x426, s != 0)] {
                    if read {
                        self.number(v, z, 2, n);
                        n.add(2);
                        n.add(if d == 1 { one } else { many });
                        n.add(1);
                    }
                }
            }
            // LesReadRisultato 0x10006a30
            Kind::Risultato => {
                self.number(n.i(off::I0), z, 2, n);
                n.add(2);
                n.add(0x15);
                n.add(2);
                self.number(n.i(off::I1), z, 2, n);
            }
        }
    }

    fn do_hook(&self, h: Do, v: i32, n: &Num) -> i32 {
        match h {
            // LesDoOrdinalTenth 0x100046b0: a round ten, not a round hundred
            Do::OrdinalTenth => (v % 10 == 0 && v % 100 != 0) as i32,
            // LesDoOrdinalDay 0x100046f0
            Do::OrdinalDay => 1,
            // LesDoGender 0x10004740: ordinals masculine (4) or feminine
            // (5), dates masculine; reals in the `ext` mode feminine for 1
            // and 2 (0x10004700)
            Do::Gender => match n.typ() {
                4 | 8 => 1,
                5 => 2,
                6 if n.reading_mode() == self.s(EXT) && (v == 1 || v == 2 || v % 10 == 1 || v % 10 == 2) => 2,
                _ => 0,
            },
            // LesDoComposed / LesDoPlural 0x100047c0, and the engine's
            // defaults for the rest
            _ => 0,
        }
    }
}
