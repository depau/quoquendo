//! Polish, ported from the original's language module `LTTS7Polish.dll`
//! (MECHANICS.md §4l). The module's one export, `Polish` (0x1000e050),
//! accepts a descriptor whose `Language` list has `polish` and hands
//! LoqTTS7.dll a table of 30 named functions (0x10022d78); the ones the LES
//! and FON stages run are ported here, each under its address. Its tables
//! (character classes, word lists, the ending table, the phone symbols)
//! are read from the module's own bytes at the addresses its code uses; the
//! dictionary rows come from the language pack's `Polish.dbl`, as the
//! module's loader (0x1000d990) reads them.
//!
//! The module's text is Windows-1250: its `ELQGetLanguageNativeEncoding`
//! (0x1000e020) returns 0x4e2, and the engine converts its UTF-8 input to
//! that code page (`Les::code_page`).
//!
//! The module has no `LesElaboraParola`, `LesSetModalitaFrase` nor
//! `LesGestisciMaiuscole`: the engine's own run.

mod allofono;
mod confini;
mod les;
mod numeri;
mod parola;
#[cfg(test)]
mod trace;

use super::common::{Pe, Phone, Tok, Word};
use crate::g2p::Token;

/// The function table's address (30 `{ name, function }` pairs).
const FUNCTIONS: u32 = 0x10022d78;

/// Character classes (`ELQGetGraphemes` 0x1000d890 → 0x100222a0): 8 bytes
/// per character, lower case, upper case, then flags at +4 (1 letter, 2
/// part of an acronym, 4 digit, 8 consonant, 0x10 vowel, 0x20 upper case,
/// 0x40 lower case, 0x80 accented, 0x100 punctuation).
const CHARS: u32 = 0x100222a0;
pub(crate) const LETTER: u32 = 0x1;
pub(crate) const ACRONYM: u32 = 0x2;
pub(crate) const CONSONANT: u32 = 0x8;
pub(crate) const VOWEL: u32 = 0x10;
pub(crate) const UPPER: u32 = 0x20;
pub(crate) const ACCENTED: u32 = 0x80;
pub(crate) const PUNCT: u32 = 0x100;

/// The module's phone table (`ELQGetPhonemes` 0x1000d8a0 → 0x10022c78:
/// `{ entries, u16 count }`, entries `{ symbol, code }`).
const PHONES: u32 = 0x10022c78;

/// One dictionary row: `SELECT grafemica, fonetica FROM lessico`
/// (0x1000d990), `None` for SQL NULL.
pub type Row = [Option<Vec<u8>>; 2];

/// The language module, read from `LTTS7Polish.dll` and `Polish.dbl`.
pub struct Polish {
    pub(crate) pe: Pe,
    chars: Vec<(u8, u8, u32)>,
    /// the dictionary's rows, in table order
    dict: Vec<Row>,
    /// `SELECT grafemica,abbreviazione FROM lessico WHERE
    /// abbreviazione!="" ORDER BY abbreviazione COLLATE NOCASE DESC`
    /// (0x1000d990, `LesGetAbbreviazioni` 0x1000deb0) as (abbreviation,
    /// expansion)
    abbreviations: Vec<(Vec<u8>, Vec<u8>)>,
    abbreviation_table: std::sync::OnceLock<Vec<super::common::Abbreviation>>,
}

impl Polish {
    /// The module's tables, when `files.dll` is `LTTS7Polish.dll` as the
    /// pack ships it (the function table at 0x10022d78) and `files.dbl` its
    /// dictionary.
    pub fn from_files(files: &super::ModuleFiles) -> Option<Polish> {
        let pe = Pe::parse(files.dll.clone())?;
        let t = pe.function_table(FUNCTIONS, 30);
        let named = |i: usize, n: &[u8], a: u32| t.get(i).is_some_and(|(name, at)| name == n && *at == a);
        if !named(16, b"LesAnalizza", 0x1000e640) || !named(20, b"FonTrasParola", 0x10010f60) || !named(21, b"FonTrasConfini", 0x1000e9e0) || !named(22, b"FonAllofonoLingua", 0x100105a0) {
            return None;
        }
        let chars = (0..256u32)
            .map(|c| (pe.u8(CHARS + 8 * c), pe.u8(CHARS + 8 * c + 1), pe.u32(CHARS + 8 * c + 4)))
            .collect();
        let (dict, abbreviations) = match files.dbl.as_deref() {
            Some(d) => read_dictionary(d)?,
            None => (Vec::new(), Vec::new()),
        };
        Some(Polish {
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

    /// The character table's upper case.
    pub(crate) fn upper(&self, c: u8) -> u8 {
        self.chars[c as usize].1
    }

    /// The string at `va` in the module.
    pub(crate) fn s(&self, va: u32) -> &[u8] {
        self.pe.cstr(va)
    }

    /// LTTS7Util's `ELQDBTableFind` (0x10005df0) on the dictionary by
    /// `grafemica`: a binary search with `ELQstricmp` (ASCII case folded)
    /// over the rows in table order, then the equal rows around the hit.
    /// The first matching row.
    pub(crate) fn table_find(&self, key: &[u8]) -> Option<&Row> {
        let g = |i: usize| self.dict[i][0].as_deref().unwrap_or(&[]);
        let n = self.dict.len() as i32;
        let (mut lo, mut hi) = (0i32, n - 1);
        while lo <= hi {
            let mid = (lo + hi) / 2;
            match stricmp(key, g(mid as usize)) {
                c if c < 0 => hi = mid - 1,
                c if c > 0 => lo = mid + 1,
                _ => {
                    let mut i = mid - 1;
                    while i >= 0 && stricmp(key, g(i as usize)) == 0 {
                        i -= 1;
                    }
                    return Some(&self.dict[(i + 1) as usize]);
                }
            }
        }
        None
    }

    /// The module's phone symbols.
    pub(crate) fn phone_table(&self) -> impl Iterator<Item = (&[u8], u32)> {
        let ent = self.pe.u32(PHONES);
        let n = self.pe.u16(PHONES + 4) as u32;
        (0..n).map(move |k| (self.s(self.pe.u32(ent + 8 * k)), self.pe.u32(ent + 8 * k + 4)))
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

/// `lessico` rows (grafemica, fonetica, abbreviazione) as the module's 2
/// columns, and the abbreviations.
fn read_dictionary(dbl: &[u8]) -> Option<(Vec<Row>, Vec<(Vec<u8>, Vec<u8>)>)> {
    use crate::sqlite::Value;
    let rows = crate::sqlite::read_table(dbl, "lessico").ok()?;
    let v = |r: &Vec<Value>, i: usize| match r.get(i) {
        Some(Value::Text(t)) | Some(Value::Blob(t)) => Some(t.clone()),
        Some(Value::Int(n)) => Some(n.to_string().into_bytes()),
        Some(Value::Real(x)) => Some(x.to_string().into_bytes()),
        _ => None,
    };
    let dict = rows.iter().map(|r| [v(r, 0), v(r, 1)]).collect();
    let mut abbr: Vec<(Vec<u8>, Vec<u8>)> = rows
        .iter()
        .filter_map(|r| Some((v(r, 2).filter(|a| !a.is_empty())?, v(r, 0).unwrap_or_default())))
        .collect();
    // COLLATE NOCASE folds A–Z only; DESC, ties in table order
    let fold = |s: &[u8]| s.iter().map(|c| c.to_ascii_lowercase()).collect::<Vec<u8>>();
    abbr.sort_by(|a, b| fold(&b.0).cmp(&fold(&a.0)));
    Some((dict, abbr))
}

impl super::common::Les for Polish {
    fn letter(&self, c: u8) -> (bool, bool, u8) {
        let f = self.flags(c);
        (f & LETTER != 0, f & UPPER != 0, self.lower(c))
    }
    fn code_page(&self) -> u32 {
        // ELQGetLanguageNativeEncoding 0x1000e020
        0x4e2
    }
    fn analizza(&self, w: &mut [Word]) {
        self.analizza(w)
    }
    fn accenta(&self, w: &mut [Word]) {
        self.accenta(w)
    }
    fn confini(&self, w: &mut [Word], mode: i32) {
        self.confini(w, mode)
    }
    fn gradi(&self, w: &mut [Word]) {
        les::gradi(w)
    }
    fn is_letter(&self, c: u8) -> bool {
        // LesIsLetter 0x1000cdd0
        self.has(c, LETTER)
    }
    fn is_upper(&self, c: u8) -> bool {
        // LesIsUpper 0x1000cde0
        self.has(c, UPPER)
    }
    fn is_part_of_acronym(&self, c: u8) -> bool {
        // LesIsPartOfAcronym 0x1000cdb0
        self.has(c, ACRONYM)
    }
    fn lower(&self, c: u8) -> u8 {
        Polish::lower(self, c)
    }
    fn normalizza_accento(&self, w: &mut Vec<u8>) {
        les::normalizza_accento(w)
    }
    fn pronunciabile(&self, w: &[u8]) -> bool {
        self.pronunciabile(w)
    }
    fn symbol_name(&self, c: u8, mode: u8) -> Vec<u8> {
        self.symbol_name(c, mode)
    }
    fn abbreviazioni(&self) -> &[super::common::Abbreviation] {
        self.abbreviation_table.get_or_init(|| super::common::abbreviation_table(self, &self.abbreviations))
    }
    fn tras_parola(&self, w: &[Word], i: usize) -> Vec<u32> {
        self.tras_parola(w, i)
    }
    fn tras_confini_in(&self, sentence: &[Word], first: usize, i: usize, _prev_bound: u8, phones: &mut Vec<Phone>, next: &mut Vec<u32>) {
        self.tras_confini(&sentence[first..], i, phones, next)
    }
    fn allofono_in(&self, sentence: &[Word], first: usize, chunk: &[Word], phones: &mut Vec<Phone>, i: usize) -> usize {
        self.allofono(sentence, first, chunk, phones, i)
    }
    fn symbol(&self, code: u32) -> Option<&str> {
        Polish::symbol(self, code)
    }
    fn leggi_numero(&self, toks: &mut [Tok], i: usize, kind: u32, dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
        numeri::leggi_numero(self, toks, i, kind, dmy)
    }
}

impl super::Module for Polish {
    fn analyse(&self, text: &str, voice: Option<&crate::voicefon::VoiceFon>) -> Vec<Token> {
        super::common::analyse(self, text, voice)
    }
}
