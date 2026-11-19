//! German, ported from the original's language module `LTTS7German.dll`
//! (MECHANICS.md §4c). The module's one export, `German` (0x10007f00),
//! walks a name → function table at 0x10029498 (28 pairs) and hands the
//! engine the function it asks for. The ones the LES and FON stages run are
//! ported here, each under its address; the module's tables (characters,
//! word lists, its morphological analyser's lexicon) are read from the
//! DLL's bytes at the addresses its code uses, its dictionary from the
//! language pack's `German.dbl` with the module's own queries. Nothing is
//! copied, nothing runs.

mod allofono;
mod db;
mod elabora;
#[cfg(test)]
mod fixtures;
mod fon;
mod les;
mod morph;
mod numeri;

use super::common::{Pe, Phone, Word};
use db::Table;

/// The function table the export walks and how many pairs it holds.
const FUNCTIONS: u32 = 0x10029498;
const FUNCTION_COUNT: u32 = 0x1c;

/// Character classes: 8 bytes per character (lower case, upper case, two
/// unused, flags: 0x01 letter, 0x04 digit or hex digit, 0x10 vowel, 0x20
/// upper case, 0x40 lower case, 0x100 punctuation).
const CHARS: u32 = 0x100287c8;
pub(crate) const LETTER: u32 = 0x01;
pub(crate) const VOWEL: u32 = 0x10;
pub(crate) const UPPER: u32 = 0x20;
pub(crate) const PUNCT: u32 = 0x100;

/// The module's phone symbols (`{entries, count}`, 8 bytes an entry:
/// symbol, code), read by 0x1000cb00 to turn dictionary pronunciations into
/// codes.
const SYMBOLS: u32 = 0x10029300;

/// The language module, read from `LTTS7German.dll`'s bytes and the
/// language pack's dictionary.
pub struct German {
    pub(crate) pe: Pe,
    /// (lower, upper, flags) per character, from `CHARS`
    chars: Vec<(u8, u8, u32)>,
    /// `SELECT grafemica, gradoaccentuale, grammaticale, tempomodo, gennum
    /// FROM lessico WHERE grammaticale!="" AND gradoaccentuale!=""`
    /// (data +8, "GrammarLexical.German")
    pub(crate) grammar: Table,
    /// `SELECT grafemica, fonetica FROM lessico` (data +0xc,
    /// "PhoneticLexical.German")
    pub(crate) phonetic: Table,
    /// the abbreviations (data +0x10), as LesGetNumeroAbbreviazioni
    /// (0x10007d40) and LesGetAbbreviazioni (0x10007d90) hand them out:
    /// row i's (grafemica = the expansion, abbreviazione = as written)
    abbreviations: Table,
    /// the engine's table built from them, on first use
    abbreviation_table: std::sync::OnceLock<Vec<super::common::Abbreviation>>,
    /// (symbol, code) from `SYMBOLS`
    symbols: Vec<(Vec<u8>, u32)>,
}

impl German {
    /// The module, when `dll` is `LTTS7German.dll` as the pack ships it
    /// (its function table at 0x10029498) and `dbl` its dictionary.
    pub fn new(dll: Vec<u8>, dbl: Option<&[u8]>) -> Option<German> {
        let pe = Pe::parse(dll)?;
        let table = pe.function_table(FUNCTIONS, FUNCTION_COUNT);
        let name = |i: usize| table.get(i).map(|(n, _)| n.as_slice());
        if name(0) != Some(b"GetVersion") || name(15) != Some(b"FonTrasParola") || name(16) != Some(b"FonAllofonoLingua") {
            return None;
        }
        let chars = (0..256u32)
            .map(|c| (pe.u8(CHARS + 8 * c), pe.u8(CHARS + 8 * c + 1), pe.u32(CHARS + 8 * c + 4)))
            .collect();
        let rows = dbl.and_then(|d| crate::sqlite::read_table(d, "lessico").ok()).unwrap_or_default();
        let grammar = Table::grammar(&rows);
        let phonetic = Table::phonetic(&rows);
        let abbreviations = Table::abbreviations(&rows);
        let entries = pe.u32(SYMBOLS);
        let n = pe.u16(SYMBOLS + 4) as u32;
        let symbols = (0..n).map(|k| (pe.cstr(pe.u32(entries + 8 * k)).to_vec(), pe.u32(entries + 8 * k + 4))).collect();
        Some(German {
            pe,
            chars,
            grammar,
            phonetic,
            abbreviations,
            abbreviation_table: Default::default(),
            symbols,
        })
    }

    pub(crate) fn flags(&self, c: u8) -> u32 {
        self.chars[c as usize].2
    }

    pub(crate) fn is_upper(&self, c: u8) -> bool {
        self.flags(c) & UPPER != 0
    }

    pub(crate) fn is_vowel(&self, c: u8) -> bool {
        self.flags(c) & VOWEL != 0
    }

    /// The table's lower case of `c` (`DAT_100287c8`).
    pub(crate) fn lower(&self, c: u8) -> u8 {
        self.chars[c as usize].0
    }

    /// The table's upper case of `c` (`DAT_100287c9`).
    pub(crate) fn upper(&self, c: u8) -> u8 {
        self.chars[c as usize].1
    }

    /// The NUL-terminated string at `va` in the module.
    pub(crate) fn s(&self, va: u32) -> &[u8] {
        self.pe.cstr(va)
    }

    /// The code of a phone symbol (0x1000cb00): 0x48 when the symbol is
    /// empty or unknown, or names a code-0 entry past the first four.
    pub(crate) fn code_of(&self, sym: &[u8]) -> u32 {
        if sym.is_empty() {
            return 0x48;
        }
        match self.symbols.iter().position(|(s, _)| s == sym) {
            Some(k) if self.symbols[k].1 == 0 && k > 3 => 0x48,
            Some(k) => self.symbols[k].1,
            None => 0x48,
        }
    }

    /// The symbol of a phone code.
    pub fn symbol(&self, code: u32) -> Option<&str> {
        self.symbols.iter().find(|&&(_, c)| c == code).and_then(|(s, _)| std::str::from_utf8(s).ok())
    }
}

/// One morpheme of an analysis, as the module copies it into a record's
/// LexInfo (0x10009680, from the analyser's `getMorphemeInfo`): 10 dwords,
/// strings as their address in the module.
pub(crate) type Morpheme = [u32; 10];

/// A record's LexInfo (`+0x14` → buffer of 0xfd0 bytes: the number of
/// analyses at +0 and +0xfcc, analysis i's morphemes at +4 + i·0x194, 0x28
/// bytes each, its morpheme count at (i+1)·0x194), kept in `Word::data` as
/// [analyses, then per analysis its count and the morphemes' dwords].
pub(crate) fn lexinfo(w: &Word) -> Vec<Vec<Morpheme>> {
    let d = &w.data;
    let mut out = Vec::new();
    let n = d.first().copied().unwrap_or(0) as usize;
    let mut at = 1;
    for _ in 0..n {
        let count = d.get(at).copied().unwrap_or(0) as usize;
        at += 1;
        let mut sol = Vec::with_capacity(count);
        for _ in 0..count {
            let mut m = [0u32; 10];
            for (k, v) in m.iter_mut().enumerate() {
                *v = d.get(at + k).copied().unwrap_or(0);
            }
            at += 10;
            sol.push(m);
        }
        out.push(sol);
    }
    out
}

/// Store analyses as a record's LexInfo (see `lexinfo`).
pub(crate) fn set_lexinfo(w: &mut Word, sols: &[Vec<Morpheme>]) {
    let mut d = vec![sols.len() as u32];
    for s in sols {
        d.push(s.len() as u32);
        for m in s {
            d.extend_from_slice(m);
        }
    }
    w.data = d;
}

impl super::common::Les for German {
    fn letter(&self, c: u8) -> (bool, bool, u8) {
        let f = self.flags(c);
        (f & LETTER != 0, f & UPPER != 0, self.lower(c))
    }
    fn number_words(&self, digits: &str) -> Vec<Vec<u8>> {
        digits.bytes().map(|d| vec![d]).collect()
    }
    fn leggi_numero(&self, toks: &mut [super::common::Tok], i: usize, kind: u32, dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
        numeri::leggi_numero(self, toks, i, kind, dmy)
    }
    fn gestisci_maiuscole(&self, w: &mut [Word]) {
        German::gestisci_maiuscole(self, w)
    }
    fn analizza(&self, w: &mut [Word]) {
        German::analizza(self, w)
    }
    fn accenta(&self, w: &mut [Word]) {
        German::accenta(self, w)
    }
    fn elabora_parola(&self, token: &[u8], idx: usize, flag408: u8, spell: u8, mode: u8) -> Option<Vec<Word>> {
        Some(German::elabora_parola(self, token, idx, flag408, spell, mode))
    }
    fn confini(&self, w: &mut [Word], mode: i32) {
        German::confini(self, w, mode)
    }
    fn gradi(&self, w: &mut [Word]) {
        German::gradi(self, w)
    }
    fn tras_parola(&self, w: &[Word], i: usize) -> Vec<u32> {
        German::tras_parola(self, w, i)
    }
    fn allofono(&self, w: &[Word], phones: &mut Vec<Phone>, i: usize) -> usize {
        German::allofono(self, w, phones, i)
    }
    fn symbol(&self, code: u32) -> Option<&str> {
        German::symbol(self, code)
    }
    fn normalizza_accento(&self, w: &mut Vec<u8>) {
        German::normalizza_accento(self, w)
    }
    fn pronunciabile(&self, w: &[u8]) -> bool {
        // the hook carries no argument 4; the engine's traced calls pass 1
        // (26 of 34) or 0 — TODO(common): pass it through
        self.pronunciabile_c(w, 1)
    }
    fn pronunciabile_flag(&self, w: &[u8], flag: u8) -> bool {
        self.pronunciabile_c(w, flag)
    }
    fn symbol_name(&self, c: u8, mode: u8) -> Vec<u8> {
        self.symbol_name_mode(c, mode)
    }
    fn abbreviazioni(&self) -> &[super::common::Abbreviation] {
        self.abbreviation_table.get_or_init(|| {
            let pairs: Vec<(Vec<u8>, Vec<u8>)> = (0..self.abbreviations.rows.len())
                .map(|r| (self.abbreviations.get(r, 1).unwrap_or(b"").to_vec(), self.abbreviations.get(r, 0).unwrap_or(b"").to_vec()))
                .collect();
            super::common::abbreviation_table(self, &pairs)
        })
    }
}

impl super::Module for German {
    fn analyse(&self, text: &str, voice: Option<&crate::voicefon::VoiceFon>) -> Vec<crate::g2p::Token> {
        super::common::analyse(self, text, voice)
    }
}
