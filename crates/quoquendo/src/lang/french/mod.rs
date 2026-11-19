//! French, ported from the original's language module `LTTS7French.dll`
//! (MECHANICS.md §4c). The module's one export, `French` (0x100071d0),
//! hands LoqTTS7.dll a table of 25 named functions (0x1001e998); the ones
//! the LES and FON stages run are ported here, each under its address. Its
//! tables (character classes, word lists, the ending table, the phone
//! symbols) are read from the module's own bytes at the addresses its code
//! uses; the voice dictionary rows come from the language pack's
//! `French.dbl`, as the module's loader (0x100069a0) reads them.
//!
//! The module has no `LesAccenta` and no `FonTrasConfini`: liaison, elision
//! and the dropping of a mute e across words are `FonAllofonoLingua`'s.
//!
//! Branches for `*(fon+0x44)+8 == 1` are the Canadian variant's (its own
//! phone table 0x1001e898 with `Ts`, `Dz`, `` `I ``…, FrenchCA.dbl): every
//! French voice of the pack runs with 0, so they are not ported.

mod allofono;
mod analizza;
mod confini;
mod gradi;
mod numeri;
mod parola;

use super::common::{field::*, Pe, Phone, Word};
use crate::g2p::Token;

/// The function table's address (25 `{ name, function }` pairs).
const FUNCTIONS: u32 = 0x1001e998;

/// Character classes (0x1001dcd8): 8 bytes per character, lower case,
/// upper case, then flags (1 letter, 4 digit, 0x10 vowel, 0x20 upper case,
/// 0x80 accented, 0x100 punctuation).
const CHARS: u32 = 0x1001dcd8;
const LETTER: u32 = 0x1;
const VOWEL: u32 = 0x10;
const UPPER: u32 = 0x20;

/// The dictionary columns the module selects (names at 0x1001dca0, in its
/// order; `FUN_100067a0` maps a name to its index).
pub(crate) mod col {
    pub const GRAFEMICA: usize = 0;
    pub const FONETICA: usize = 1;
    pub const GRADO: usize = 2;
    pub const GRAMMATICALE: usize = 3;
    pub const GENNUM: usize = 4;
    pub const TEMPOMODO: usize = 5;
    pub const ALTREINFO: usize = 6;
}

/// One dictionary row: the module's 7 columns, `None` for SQL NULL.
pub type Row = [Option<Vec<u8>>; 7];

/// The language module, read from `LTTS7French.dll` and `French.dbl`.
pub struct French {
    pe: Pe,
    chars: Vec<(u8, u8, u32)>,
    /// `SELECT grafemica, fonetica, gradoaccentuale, grammaticale, gennum,
    /// tempomodo, altreinfo FROM lessico` (0x100069a0), in table order
    dict: Vec<Row>,
}

impl French {
    /// The module's tables, when `dll` is `LTTS7French.dll` as the pack
    /// ships it (the function table at 0x1001e998) and `dbl` its
    /// dictionary.
    pub fn from_files(dll: Vec<u8>, dbl: Option<&[u8]>) -> Option<French> {
        let pe = Pe::parse(dll)?;
        let t = pe.function_table(FUNCTIONS, 25);
        let named = |i: usize, n: &[u8]| t.get(i).is_some_and(|(name, _)| name == n);
        if !named(14, b"LesAnalizza") || !named(19, b"FonAllofonoLingua") || !named(20, b"FonTrasParola") {
            return None;
        }
        let chars = (0..256u32)
            .map(|c| (pe.u8(CHARS + 8 * c), pe.u8(CHARS + 8 * c + 1), pe.u32(CHARS + 8 * c + 4)))
            .collect();
        let dict = match dbl {
            Some(d) => read_dictionary(d)?,
            None => Vec::new(),
        };
        Some(French { pe, chars, dict })
    }

    fn flags(&self, c: u8) -> u32 {
        self.chars[c as usize].2
    }

    fn is_vowel(&self, c: u8) -> bool {
        self.flags(c) & VOWEL != 0
    }

    /// The character table's lower case (what 0x1000e4f0 copies).
    fn lower(&self, c: u8) -> u8 {
        self.chars[c as usize].0
    }

    /// The string at `va` in the module.
    fn s(&self, va: u32) -> &[u8] {
        self.pe.cstr(va)
    }

    /// LTTS7Util's `ELQDBTableFind` (0x10005df0) on the dictionary by
    /// `grafemica`: a binary search with `ELQstricmp` (ASCII case folded)
    /// over the rows in table order, then the equal rows around the hit.
    /// The first matching row and their count.
    fn table_find(&self, key: &[u8]) -> (usize, usize) {
        let g = |i: usize| self.dict[i][col::GRAFEMICA].as_deref().unwrap_or(&[]);
        let n = self.dict.len() as i32;
        let (mut lo, mut hi) = (0i32, n - 1);
        while lo <= hi {
            let mid = (lo + hi) / 2;
            match stricmp(key, g(mid as usize)) {
                c if c < 0 => hi = mid - 1,
                c if c > 0 => lo = mid + 1,
                _ => {
                    let mut count = 1;
                    let mut i = mid - 1;
                    while i >= 0 && stricmp(key, g(i as usize)) == 0 {
                        count += 1;
                        i -= 1;
                    }
                    let mut j = mid + 1;
                    while j < n && stricmp(key, g(j as usize)) == 0 {
                        count += 1;
                        j += 1;
                    }
                    return ((i + 1) as usize, count);
                }
            }
        }
        (0, 0)
    }

    /// 0x1000d6b0: add the dictionary rows spelled `key` to a word's list;
    /// the list's length after.
    fn lookup(&self, lex: &mut Lex, key: &[u8]) -> usize {
        let (first, count) = self.table_find(key);
        for r in first..first + count {
            lex.add(r as u32);
        }
        lex.rows.len()
    }

    fn col(&self, row: u32, c: usize) -> Option<&[u8]> {
        self.dict[row as usize][c].as_deref()
    }

    /// 0x10006910: walk the list from its head to the first row whose
    /// column `c` is set and not empty (the list's current row then).
    fn has_col(&self, lex: &mut Lex, c: usize) -> bool {
        let mut r = lex.head();
        while let Some(row) = r {
            if self.col(row, c).is_some_and(|v| !v.is_empty()) {
                return true;
            }
            r = lex.next();
        }
        false
    }

    /// The module's phone symbols (0x1001e698: `{ entries, count }`,
    /// entries `{ symbol, code }`).
    fn phone_table(&self) -> impl Iterator<Item = (&[u8], u32)> {
        let ent = self.pe.u32(PHONES);
        let n = self.pe.u16(PHONES + 4) as u32;
        (0..n).map(move |k| (self.s(self.pe.u32(ent + 8 * k)), self.pe.u32(ent + 8 * k + 4)))
    }

    /// 0x1000e530: a symbol's code (0x10 for none; the table's first four
    /// entries, `^ # @` and the empty one, give 0).
    fn phone_code(&self, sym: &[u8]) -> u32 {
        if sym.is_empty() {
            return 0x10;
        }
        for (k, (s, code)) in self.phone_table().enumerate() {
            if s == sym {
                return if code == 0 && k > 3 { 0x10 } else { code };
            }
        }
        0x10
    }

    /// 0x1000e5c0: a dictionary pronunciation (symbols separated by spaces
    /// or `-`, at most 5 characters each) as codes.
    fn parse_phones(&self, text: &[u8]) -> Vec<u32> {
        let mut out = Vec::new();
        let mut i = 0usize;
        let at = |i: usize| text.get(i).copied().unwrap_or(0);
        while at(i) != 0 {
            let mut sym = Vec::new();
            while !matches!(at(i), b' ' | b'-' | 0) && sym.len() < 5 {
                sym.push(at(i));
                i += 1;
            }
            if at(i) != 0 {
                i += 1;
            }
            out.push(self.phone_code(&sym));
        }
        out
    }

    pub fn symbol(&self, code: u32) -> Option<&str> {
        self.phone_table()
            .find(|&(_, c)| c == code)
            .and_then(|(s, _)| std::str::from_utf8(s).ok())
    }

    /// The dictionary rows a word's record holds (record +0x14), to check
    /// them against the original's.
    #[doc(hidden)]
    pub fn word_rows(&self, w: &Word) -> Vec<Row> {
        Lex::of(w).rows.iter().map(|&r| self.dict[r as usize].clone()).collect()
    }
}

/// The French phone table (Canadian: 0x1001e898, not used).
const PHONES: u32 = 0x1001e698;

/// LTTS7Util's `ELQstricmp` (0x100285e0): only A–Z are folded.
fn stricmp(a: &[u8], b: &[u8]) -> i32 {
    let fold = |c: u8| if c.is_ascii_uppercase() { c + 0x20 } else { c } as i32;
    let mut i = 0;
    loop {
        let x = fold(a.get(i).copied().unwrap_or(0));
        let y = fold(b.get(i).copied().unwrap_or(0));
        if x == 0 || x != y {
            return x - y;
        }
        i += 1;
    }
}

/// `lessico` rows as the module's 7 columns (the table's are grafemica,
/// fonetica, gradoaccentuale, grammaticale, gennum, tempomodo,
/// abbreviazione, altreinfo).
fn read_dictionary(dbl: &[u8]) -> Option<Vec<Row>> {
    use crate::sqlite::Value;
    let rows = crate::sqlite::read_table(dbl, "lessico").ok()?;
    let v = |r: &Vec<Value>, i: usize| match r.get(i) {
        Some(Value::Text(t)) | Some(Value::Blob(t)) => Some(t.clone()),
        Some(Value::Int(n)) => Some(n.to_string().into_bytes()),
        Some(Value::Real(x)) => Some(x.to_string().into_bytes()),
        _ => None,
    };
    Some(rows.iter().map(|r| [v(r, 0), v(r, 1), v(r, 2), v(r, 3), v(r, 4), v(r, 5), v(r, 7)]).collect())
}

/// A word's dictionary rows (record +0x14, an `ELQList` of row pointers,
/// LTTS7Util 0x100066e0…): the rows and the list's current one. Kept in
/// `Word::data` as `[current + 1 (0 none), rows…]`.
#[derive(Default)]
pub(crate) struct Lex {
    rows: Vec<u32>,
    cur: Option<usize>,
}

impl Lex {
    fn of(w: &Word) -> Lex {
        match w.data.split_first() {
            Some((&c, rows)) => Lex {
                rows: rows.to_vec(),
                cur: (c as usize).checked_sub(1),
            },
            None => Lex::default(),
        }
    }

    fn store(&self, w: &mut Word) {
        w.data.clear();
        w.data.push(self.cur.map_or(0, |c| c as u32 + 1));
        w.data.extend(&self.rows);
    }

    /// `ELQListAddElement` (0x10006940): a row already in the list becomes
    /// the current one, otherwise it is appended and becomes it.
    fn add(&mut self, row: u32) {
        match self.rows.iter().position(|&r| r == row) {
            Some(p) => self.cur = Some(p),
            None => {
                self.rows.push(row);
                self.cur = Some(self.rows.len() - 1);
            }
        }
    }

    /// `ELQListGetHead`.
    fn head(&mut self) -> Option<u32> {
        let r = *self.rows.first()?;
        self.cur = Some(0);
        Some(r)
    }

    /// `ELQListGetNext`: at the tail it fails and the current row stays.
    fn next(&mut self) -> Option<u32> {
        let c = self.cur?;
        let r = *self.rows.get(c + 1)?;
        self.cur = Some(c + 1);
        Some(r)
    }

    /// `ELQListGetCurrent`.
    fn current(&self) -> Option<u32> {
        self.cur.map(|c| self.rows[c])
    }

    /// `ELQListDeleteCurrentElement` (0x10008020): the row before becomes
    /// the current one; after deleting the head, the new head.
    fn delete_current(&mut self) {
        if let Some(c) = self.cur {
            self.rows.remove(c);
            self.cur = if c > 0 {
                Some(c - 1)
            } else if self.rows.is_empty() {
                None
            } else {
                Some(0)
            }
        }
    }

    /// `ELQListDeleteAll`.
    fn clear(&mut self) {
        self.rows.clear();
        self.cur = None;
    }
}

/// A record's field, 0 past the end of the records (the engine reads its
/// neighbours without bounds; past the last there are no words).
fn fld(w: &[Word], i: isize, f: usize) -> u8 {
    if i < 0 {
        return 0;
    }
    w.get(i as usize).map_or(0, |r| r.f[f])
}

/// A record's text (empty past the end).
fn text(w: &[Word], i: isize) -> &[u8] {
    if i < 0 {
        return &[];
    }
    w.get(i as usize).map_or(&[][..], |r| &r.text[..])
}

/// The byte at `i`, 0 outside the string.
#[inline]
fn b(s: &[u8], i: isize) -> u8 {
    if i < 0 {
        0
    } else {
        s.get(i as usize).copied().unwrap_or(0)
    }
}

/// `strchr(set, c) != NULL`, which holds for `c == 0` too (the terminator).
fn chr(set: &[u8], c: u8) -> bool {
    c == 0 || set.contains(&c)
}

impl super::common::Les for French {
    fn letter(&self, c: u8) -> (bool, bool, u8) {
        let f = self.flags(c);
        (f & LETTER != 0, f & UPPER != 0, self.lower(c))
    }
    fn number_words(&self, digits: &str) -> Vec<Vec<u8>> {
        numeri::number_words(self, digits)
    }
    fn analizza(&self, w: &mut [Word]) {
        self.analizza(w)
    }
    fn accenta(&self, _w: &mut [Word]) {}
    fn confini(&self, w: &mut [Word], mode: i32) {
        self.confini(w, mode)
    }
    fn gradi(&self, w: &mut [Word]) {
        self.gradi(w)
    }
    fn tras_parola(&self, w: &[Word], i: usize) -> Vec<u32> {
        self.tras_parola(w, i)
    }
    fn allofono(&self, w: &[Word], phones: &mut Vec<Phone>, i: usize) -> usize {
        self.allofono(w, phones, i)
    }
    fn symbol(&self, code: u32) -> Option<&str> {
        French::symbol(self, code)
    }
}

impl super::Module for French {
    fn analyse(&self, text: &str) -> Vec<Token> {
        super::common::analyse(self, text)
    }
}

#[allow(unused)]
fn _fields() -> [usize; 5] {
    [CLASS, SUB, INFO, BOUND, GRADE]
}
