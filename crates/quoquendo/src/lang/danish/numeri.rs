//! Danish numbers: LTTS7Danish.dll's hooks into LoqTTS7.dll's generic
//! number reader (`loqnum`, MECHANICS.md §4n). The module classifies the
//! number (LesClassifyFunc 0x10008420), parses dates (LesParseData
//! 0x10008e20, the engine's parser), reads dates, fractions and codes
//! itself (LesReadData 0x100082e0, LesReadFrazione 0x10008200,
//! LesReadCodice 0x10007770, with its copy of the engine's cardinal builder
//! 0x10006400…0x10007160) and answers seven LesDo* hooks; the rest is the
//! engine's.

use super::super::common::{Pe, Tok};
use super::super::loqnum::{self as q, spell, stricmp, Card, Ctx, Do, Flags, Kind, Num, NumModule, Tables, NO_CURRENCY};
use super::super::textfilter::Classes;
use super::Danish;
use q::off;

/// `LesGetMorphTable` 0x100081e0, `LesGetClassifyTable` 0x100081c0,
/// `LesGetCurrencyTable` 0x100081d0, `LesGetOrdinalStringTable` 0x100081f0.
pub(super) const MORPHS: u32 = 0x10080928;
pub(super) const CLASSIFY: u32 = 0x100800d0;
pub(super) const CURRENCY: u32 = 0x10080118;
pub(super) const ORDINALS: u32 = 0x10080938;
/// Month names (0x24 `{name, month}`, 0x100083c0).
const MONTHS: u32 = 0x10026b90;
/// `f.Kr. e.Kr. ad bc` (after a year, 0x10026cdc ×4) and `i siden år før
/// efter inden indtil f.Kr. e.Kr. ad bc` (before it, 0x10026cb0 ×11).
const YEAR_AFTER: u32 = 0x10026cdc;
const YEAR_BEFORE: u32 = 0x10026cb0;
/// `s` 0x10024020, `'` 0x10026dec (`1980's`), the separators `-`
/// 0x10026de8, `.` 0x10026de4, `/` 0x10026de0, the modes `d` 0x10026d54
/// and `y` 0x100241c0.
const S: u32 = 0x10024020;
const APOSTROPHE: u32 = 0x10026dec;
const DASH: u32 = 0x10026de8;
const DOT: u32 = 0x10026de4;
const SLASH: u32 = 0x10026de0;
const MODE_D: u32 = 0x10026d54;
const MODE_Y: u32 = 0x100241c0;

impl Danish {
    pub(super) fn leggi_numero(&self, toks: &mut [Tok], i: usize, kind: u32, dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
        Some(q::leggi_numero(self, toks, i, kind, dmy))
    }

    /// 0x100083c0: the month a word names, else 0.
    fn month_of(&self, s: &[u8]) -> i32 {
        (0..0x24).find(|&k| stricmp(self.pe.cstr(self.pe.u32(MONTHS + 8 * k)), s)).map(|k| self.pe.u32(MONTHS + 8 * k + 4) as i32).unwrap_or(0)
    }

    fn word_in(&self, list: u32, n: u32, s: &[u8]) -> bool {
        (0..n).any(|k| stricmp(s, self.pe.cstr(self.pe.u32(list + 4 * k))))
    }

    fn card(&self) -> Card<'_, Danish> {
        Card { m: self, neuter_one: true }
    }

    fn hook(&self, h: Do, v: i32, n: &Num) -> bool {
        self.do_hook(h, v, n) & 0xff != 0
    }

    /// 0x100078a0: a date in the reading mode's order; days ordinals when
    /// LesDoOrdinalDay says so, years split in centuries by LesDoYearSplit
    /// (`hundrede` between unless a round hundred; `og et` for a 01).
    fn date(&self, n: &mut Num, d: i32, mo: i32, y: i32, mode: &[u8]) {
        let c = self.card();
        let z = Flags::default();
        let (mut century, mut yy, mut gender, mut hundred) = (-1, -1, 0, true);
        if y != -1 && y != 0 {
            century = y / 100;
            yy = y % 100;
            if yy == 0 {
                yy = 100;
                hundred = false;
            } else if yy == 1 {
                gender = 2;
            }
        }
        let day = |n: &mut Num, then: i32| {
            let od = self.hook(Do::OrdinalDay, d, n);
            c.number(d, Flags { ord: od, gender: 1, ..z }, n);
            n.add(then);
        };
        let month = |n: &mut Num| {
            c.month(mo, n);
            n.add(2);
        };
        // the year: `plural` from LesDoPlural for the `y` mode
        let year = |n: &mut Num, then: i32, plural: bool| {
            let split = self.hook(Do::YearSplit, y, n);
            let p = plural && self.hook(Do::Plural, y, n);
            let mut v = y;
            if split {
                c.number(century, z, n);
                if hundred {
                    n.add(0x386);
                    n.add(0x387);
                    n.add(1);
                }
                v = yy;
            }
            c.number(v, Flags { gender, plural: p, ..z }, n);
            n.add(then);
        };
        let of = |n: &mut Num, h: Do, v: i32| {
            if self.hook(h, v, n) {
                n.add(3);
                n.add(2);
            }
        };
        match mode {
            b"mdy" => {
                if mo != -1 {
                    month(n);
                }
                if d != -1 {
                    day(n, 1);
                }
                if y != -1 {
                    year(n, 2, false);
                }
            }
            b"ymd" => {
                if y != -1 {
                    year(n, 1, false);
                }
                if mo != -1 {
                    month(n);
                }
            }
            b"dmy" => {
                if d != -1 {
                    day(n, 2);
                    of(n, Do::OfBetweenDayMonth, d);
                }
                if mo != -1 {
                    month(n);
                    of(n, Do::OfBetweenMonthYear, mo);
                }
                if y != -1 {
                    year(n, 2, false);
                }
            }
            // as the module does: the month only
            b"md" | b"m" => {
                if mo != -1 {
                    month(n);
                }
            }
            b"dm" => {
                if d != -1 {
                    day(n, 2);
                    of(n, Do::OfBetweenDayMonth, d);
                }
                if mo != -1 {
                    month(n);
                }
            }
            b"my" => {
                if mo != -1 {
                    month(n);
                    of(n, Do::OfBetweenMonthYear, mo);
                }
                if y != -1 {
                    year(n, 2, false);
                }
            }
            b"y" => {
                if y != -1 {
                    year(n, 2, true);
                }
            }
            b"d" => {
                if d != -1 {
                    day(n, 2);
                }
            }
            // `dym` and any other mode
            _ => {
                if d != -1 {
                    day(n, 1);
                }
                if y != -1 {
                    year(n, 1, false);
                }
                if mo != -1 {
                    month(n);
                }
            }
        }
    }
}

impl NumModule for Danish {
    fn tables(&self) -> &Tables {
        &self.num_tables
    }
    fn classes(&self) -> Option<&Classes> {
        self.classes.as_ref()
    }
    fn engine(&self) -> Option<&Pe> {
        self.engine.as_ref()
    }

    /// LesClassifyFunc 0x10008420: the classify table's escape (a cardinal
    /// followed by an ordinal-table string uses it up), money next to a
    /// currency; then dates: before a month name (`d`); a four-digit year
    /// before `f.Kr.`/`s`/`'s` or after `i`, `siden`… (`y`); after a month
    /// name (`y` if four digits, else `d` unless it starts with 0 after a
    /// `- . /`).
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
        let (n1, n2, p1, p2) = (c.tok(1), c.tok(2), c.tok(-1), c.tok(-2));
        let four = c.text.len() == 4 && c.text.iter().all(u8::is_ascii_digit);
        let set = |n: &mut Num, mode: u32| {
            let m = self.pe.cstr(mode).to_vec();
            n.set_modes(&m, &m);
            8
        };
        if (1..=12).contains(&self.month_of(n1)) {
            return set(n, MODE_D);
        }
        if self.word_in(YEAR_AFTER, 4, n1) && four {
            return set(n, MODE_Y);
        }
        let s = self.pe.cstr(S);
        if (n1 == s || (n1 == self.pe.cstr(APOSTROPHE) && n2 == s)) && four {
            return set(n, MODE_Y);
        }
        if c.focus == 0 {
            return id;
        }
        if self.word_in(YEAR_BEFORE, 11, p1) && four {
            return set(n, MODE_Y);
        }
        let after_sep = p1 == self.pe.cstr(DASH) || p1 == self.pe.cstr(DOT) || p1 == self.pe.cstr(SLASH);
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
        let card = self.card();
        match k {
            // LesReadData 0x100082e0
            Kind::Data => {
                let (d, mo, y) = (n.i(off::I0), n.i(off::I1), n.i(off::I2));
                let (de, me, ye) = (n.i(off::I3), n.i(off::I4), n.i(off::I5));
                let mode = n.reading_mode();
                self.date(n, d, mo, y, &mode);
                if de != -1 || me != -1 || ye != -1 {
                    n.add(1);
                    self.date(n, de, me, ye, &mode);
                }
            }
            // LesReadFrazione 0x10008200: `en halv`, else the numerator and
            // the denominator as an ordinal with `del`/`dele`
            Kind::Frazione => {
                let (mut a, b) = (n.i(off::I0), n.i(off::I1));
                if a < 0 {
                    n.add(9);
                    n.add(2);
                    a = a.wrapping_neg();
                }
                if a == 1 && b == 2 {
                    n.add(0x19);
                    n.add(2);
                    n.add(0x56);
                    n.add(2);
                    return;
                }
                card.number(a, z, n);
                n.add(2);
                card.number(b, Flags { ord: true, gender: 1, ..z }, n);
                n.add(if a == 1 { 0x2c } else { 0x2d });
                n.add(2);
            }
            // LesReadCodice 0x10007770: each digit spelled; groups followed
            // by morph 2 (at most one non-digit) or 1
            Kind::Codice => {
                let s = n.s(off::S_CODE);
                let others = s.iter().filter(|c| !c.is_ascii_digit()).count();
                let mut p = 0;
                while p < s.len() {
                    let run: Vec<u8> = s[p..].iter().take_while(|c| c.is_ascii_digit()).copied().collect();
                    if run.is_empty() {
                        p += 1;
                        continue;
                    }
                    p += run.len();
                    for &ch in &run {
                        spell(ch, n);
                        n.add(2);
                    }
                    n.add(if others < 2 { 2 } else { 1 });
                }
            }
            _ => q::default_read(self, k, n, c),
        }
    }

    fn do_hook(&self, h: Do, v: i32, n: &Num) -> i32 {
        match h {
            // LesDoComposed / LesDoOrdinalDay 0x100030a0
            Do::Composed | Do::OrdinalDay => 1,
            // LesDoAndBetweenTenthsUnits 0x100030b0
            Do::AndBetweenTenthsUnits => (v % 10 != 0) as i32,
            // LesDoTenthInversion 0x100030d0
            Do::TenthInversion => (v.wrapping_abs() > 0x14) as i32,
            // LesDoYearSplit 0x100030e0
            Do::YearSplit => (v < 2000) as i32,
            // LesDoAndBetweenThousandsHundreds 0x100030f0: 2001–2099 in a
            // date, 2001–2010 otherwise
            Do::AndBetweenThousandsHundreds => {
                let top = if n.typ() == 8 { 0x62 } else { 9 };
                ((v.wrapping_sub(2001) as u32) <= top) as i32
            }
            _ => 0,
        }
    }
}
