//! Greek, ported from the original's language module `LTTS7Greek.dll`
//! (MECHANICS.md §4f). The module's one export, `Greek` (0x10008500),
//! hands LoqTTS7.dll a table of 32 named functions (0x10021d00); the ones
//! the LES and FON stages run are ported here, each under its address. Its
//! tables (character classes, word lists, the ending table, the phone
//! symbols) are read from the module's own bytes at the addresses its code
//! uses; the dictionary rows come from the language pack's `Greek.dbl`, as
//! the module's loader (0x10007d10) reads them.
//!
//! The module's text is Windows-1253: its `ELQGetLanguageNativeEncoding`
//! (0x10008360) returns 0x4e5, and the engine converts its UTF-8 input to
//! that code page (`Les::code_page`).
//!
//! The module has no `LesConfiniProsodici`, `FonTrasConfini` nor
//! `LesNormalizzaAccento`: the engine's own LesConfiniProsodici runs
//! (LoqTTS7.dll 0x100409c0, `les::confini`).

mod allofono;
mod les;
mod numeri;
mod parola;
#[cfg(test)]
mod trace;

use super::common::{Pe, Phone, Tok, Word};
use crate::g2p::Token;

/// The function table's address (32 `{ name, function }` pairs).
const FUNCTIONS: u32 = 0x10021d00;

/// Character classes (`ELQGetGraphemes` 0x10007c10 → 0x100212d0): 8 bytes
/// per character, lower case, upper case, then flags at +4 (1 letter, 2
/// part of an acronym, 4 digit, 8 consonant, 0x10 vowel, 0x20 upper case,
/// 0x40 lower case, 0x80 accented, 0x100 punctuation).
const CHARS: u32 = 0x100212d0;
pub(crate) const LETTER: u32 = 0x1;
pub(crate) const ACRONYM: u32 = 0x2;
pub(crate) const CONSONANT: u32 = 0x8;
pub(crate) const VOWEL: u32 = 0x10;
pub(crate) const UPPER: u32 = 0x20;
pub(crate) const ACCENTED: u32 = 0x80;
pub(crate) const PUNCT: u32 = 0x100;

/// The module's phone table (`ELQGetPhonemes` 0x10007c20 → 0x10021c40:
/// `{ entries, u16 count }`, entries `{ symbol, code }`).
const PHONES: u32 = 0x10021c40;

/// The dictionary columns the module selects (0x10007d10: `SELECT
/// grafemica, fonetica, gradoaccentuale, grammaticale, specgramm, gennum
/// FROM lessico`), in its order.
pub(crate) mod col {
    pub const GRAFEMICA: usize = 0;
    pub const FONETICA: usize = 1;
    pub const GRADOACCENTUALE: usize = 2;
    pub const GRAMMATICALE: usize = 3;
    pub const SPECGRAMM: usize = 4;
    pub const GENNUM: usize = 5;
}

/// One dictionary row: the module's 6 columns, `None` for SQL NULL.
pub type Row = [Option<Vec<u8>>; 6];

/// The language module, read from `LTTS7Greek.dll` and `Greek.dbl`.
pub struct Greek {
    pub(crate) pe: Pe,
    chars: Vec<(u8, u8, u32)>,
    /// the dictionary's rows, in table order
    dict: Vec<Row>,
    /// `SELECT grafemica, abbreviazione FROM lessico WHERE
    /// ifnull(abbreviazione,'') <> '' ORDER BY abbreviazione COLLATE NOCASE
    /// DESC` (0x10007d10, `LesGetAbbreviazioni` 0x10008240) as
    /// (abbreviation, expansion)
    abbreviations: Vec<(Vec<u8>, Vec<u8>)>,
    abbreviation_table: std::sync::OnceLock<Vec<super::common::Abbreviation>>,
}

impl Greek {
    /// The module's tables, when `files.dll` is `LTTS7Greek.dll` as the pack
    /// ships it (the function table at 0x10021d00) and `files.dbl` its
    /// dictionary.
    pub fn from_files(files: &super::ModuleFiles) -> Option<Greek> {
        let pe = Pe::parse(files.dll.clone())?;
        let t = pe.function_table(FUNCTIONS, 32);
        let named = |i: usize, n: &[u8], a: u32| t.get(i).is_some_and(|(name, at)| name == n && *at == a);
        if !named(11, b"LesAnalizza", 0x10009620) || !named(15, b"FonTrasParola", 0x1000a0e0) || !named(16, b"FonAllofonoLingua", 0x1000b470) {
            return None;
        }
        let chars = (0..256u32)
            .map(|c| (pe.u8(CHARS + 8 * c), pe.u8(CHARS + 8 * c + 1), pe.u32(CHARS + 8 * c + 4)))
            .collect();
        let (dict, abbreviations) = match files.dbl.as_deref() {
            Some(d) => read_dictionary(d)?,
            None => (Vec::new(), Vec::new()),
        };
        Some(Greek {
            pe,
            chars,
            dict,
            abbreviations,
            abbreviation_table: Default::default(),
        })
    }

    pub(crate) fn flags(&self, c: u8) -> u32 {
        self.chars[c as usize].2
    }

    pub(crate) fn has(&self, c: u8, f: u32) -> bool {
        self.flags(c) & f != 0
    }

    /// The character table's lower case.
    pub(crate) fn lower(&self, c: u8) -> u8 {
        self.chars[c as usize].0
    }

    /// The string at `va` in the module.
    pub(crate) fn s(&self, va: u32) -> &[u8] {
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

    pub(crate) fn col(&self, row: u32, c: usize) -> Option<&[u8]> {
        self.dict[row as usize][c].as_deref()
    }

    /// The module's phone symbols.
    fn phone_table(&self) -> impl Iterator<Item = (&[u8], u32)> {
        let ent = self.pe.u32(PHONES);
        let n = self.pe.u16(PHONES + 4) as u32;
        (0..n).map(move |k| (self.s(self.pe.u32(ent + 8 * k)), self.pe.u32(ent + 8 * k + 4)))
    }

    /// 0x100097b0: a symbol's code (0x10 for none; the table's first four
    /// entries give 0, or 0x10 when their code is 0 past them).
    pub(crate) fn phone_code(&self, sym: &[u8]) -> u32 {
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

    pub fn symbol(&self, code: u32) -> Option<&str> {
        self.phone_table()
            .find(|&(_, c)| c == code)
            .and_then(|(s, _)| std::str::from_utf8(s).ok())
    }
}

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

/// `lessico` rows as the module's 6 columns (the table's are grafemica,
/// fonetica, grammaticale, specgramm, gennum, abbreviazione,
/// gradoaccentuale), and the abbreviations.
fn read_dictionary(dbl: &[u8]) -> Option<(Vec<Row>, Vec<(Vec<u8>, Vec<u8>)>)> {
    use crate::sqlite::Value;
    let rows = crate::sqlite::read_table(dbl, "lessico").ok()?;
    let v = |r: &Vec<Value>, i: usize| match r.get(i) {
        Some(Value::Text(t)) | Some(Value::Blob(t)) => Some(t.clone()),
        Some(Value::Int(n)) => Some(n.to_string().into_bytes()),
        Some(Value::Real(x)) => Some(x.to_string().into_bytes()),
        _ => None,
    };
    let dict = rows.iter().map(|r| [v(r, 0), v(r, 1), v(r, 6), v(r, 2), v(r, 3), v(r, 4)]).collect();
    let mut abbr: Vec<(Vec<u8>, Vec<u8>)> = rows
        .iter()
        .filter_map(|r| Some((v(r, 5).filter(|a| !a.is_empty())?, v(r, 0).unwrap_or_default())))
        .collect();
    // COLLATE NOCASE folds A–Z only; DESC, ties in table order
    let fold = |s: &[u8]| s.iter().map(|c| c.to_ascii_lowercase()).collect::<Vec<u8>>();
    abbr.sort_by(|a, b| fold(&b.0).cmp(&fold(&a.0)));
    Some((dict, abbr))
}

/// A word's dictionary rows (record +0x14, an `ELQList` of row pointers,
/// LTTS7Util 0x10006940…): the rows and the list's current one. Kept in
/// `Word::data` as `[current + 1 (0 none), rows…]`.
#[derive(Default)]
pub(crate) struct Lex {
    rows: Vec<u32>,
    cur: Option<usize>,
}

impl Lex {
    pub(crate) fn of(w: &Word) -> Lex {
        match w.data.split_first() {
            Some((&c, rows)) => Lex {
                rows: rows.to_vec(),
                cur: (c as usize).checked_sub(1),
            },
            None => Lex::default(),
        }
    }

    pub(crate) fn store(&self, w: &mut Word) {
        w.data.clear();
        w.data.push(self.cur.map_or(0, |c| c as u32 + 1));
        w.data.extend(&self.rows);
    }

    /// `ELQListAddElement` (0x10006940): a row already in the list becomes
    /// the current one, otherwise it is appended and becomes it.
    pub(crate) fn add(&mut self, row: u32) {
        match self.rows.iter().position(|&r| r == row) {
            Some(p) => self.cur = Some(p),
            None => {
                self.rows.push(row);
                self.cur = Some(self.rows.len() - 1);
            }
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.rows.len()
    }

    /// `ELQListGetHead`.
    pub(crate) fn head(&mut self) -> Option<u32> {
        let r = *self.rows.first()?;
        self.cur = Some(0);
        Some(r)
    }

    /// `ELQListGetNext`: at the tail it fails and the current row stays.
    pub(crate) fn next(&mut self) -> Option<u32> {
        let c = self.cur?;
        let r = *self.rows.get(c + 1)?;
        self.cur = Some(c + 1);
        Some(r)
    }

    /// `ELQListGetCurrent`.
    pub(crate) fn current(&self) -> Option<u32> {
        self.cur.map(|c| self.rows[c])
    }

    /// `ELQListDeleteCurrentElement` (0x10008020): the row before becomes
    /// the current one; after deleting the head, the new head.
    pub(crate) fn delete_current(&mut self) {
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
}

/// The byte at `i`, 0 outside the string.
#[inline]
pub(crate) fn b(s: &[u8], i: isize) -> u8 {
    if i < 0 {
        0
    } else {
        s.get(i as usize).copied().unwrap_or(0)
    }
}

impl super::common::Les for Greek {
    fn letter(&self, c: u8) -> (bool, bool, u8) {
        let f = self.flags(c);
        (f & LETTER != 0, f & UPPER != 0, self.lower(c))
    }
    fn code_page(&self) -> u32 {
        // ELQGetLanguageNativeEncoding 0x10008360
        0x4e5
    }
    fn analizza(&self, w: &mut [Word]) {
        self.analizza(w)
    }
    fn accenta(&self, w: &mut [Word]) {
        self.accenta(w)
    }
    fn modalita_frase(&self, w: &[Word]) -> i32 {
        les::modalita_frase(w)
    }
    fn confini(&self, w: &mut [Word], mode: i32) {
        super::common::confini_default(w, mode)
    }
    fn gradi(&self, w: &mut [Word]) {
        self.gradi(w)
    }
    fn elabora_parola_at(&self, toks: &mut [Tok], i: usize, idx: usize, spell: u8, mode: u8) -> Option<(Vec<Word>, usize)> {
        Some((self.elabora_parola(toks, i, idx, spell, mode), 0))
    }
    fn is_letter(&self, c: u8) -> bool {
        // LesIsLetter 0x10008710
        self.has(c, LETTER)
    }
    fn is_upper(&self, c: u8) -> bool {
        // LesIsUpper 0x10008720
        self.has(c, UPPER)
    }
    fn is_part_of_acronym(&self, c: u8) -> bool {
        // LesIsPartOfAcronym 0x100086f0
        self.has(c, ACRONYM)
    }
    fn lower(&self, c: u8) -> u8 {
        Greek::lower(self, c)
    }
    fn pronunciabile(&self, w: &[u8]) -> bool {
        self.pronunciabile(w)
    }
    fn symbol_name(&self, c: u8, mode: u8) -> Vec<u8> {
        self.symbol_name_mode(c, mode)
    }
    fn abbreviazioni(&self) -> &[super::common::Abbreviation] {
        self.abbreviation_table.get_or_init(|| super::common::abbreviation_table(self, &self.abbreviations))
    }
    fn tras_parola(&self, w: &[Word], i: usize) -> Vec<u32> {
        self.tras_parola(w, i)
    }
    fn allofono(&self, w: &[Word], phones: &mut Vec<Phone>, i: usize) -> usize {
        self.allofono(w, phones, i)
    }
    fn symbol(&self, code: u32) -> Option<&str> {
        Greek::symbol(self, code)
    }
    fn leggi_numero(&self, toks: &mut [Tok], i: usize, kind: u32, dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
        numeri::leggi_numero(self, toks, i, kind, dmy)
    }
}

impl super::Module for Greek {
    fn analyse(&self, text: &str, voice: Option<&crate::voicefon::VoiceFon>) -> Vec<Token> {
        super::common::analyse(self, text, voice)
    }
}
