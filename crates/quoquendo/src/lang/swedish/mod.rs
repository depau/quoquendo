//! Swedish, ported from the original's language module `LTTS7Swedish.dll`
//! (MECHANICS.md §4j). The module's one export, `Swedish` (0x1000a540),
//! checks that the descriptor's `Language` names `swedish` and walks a
//! name → function table at 0x10046ec0 (26 pairs). The functions the LES
//! and FON stages run are ported here, each under its address; the
//! module's tables (characters, its morphological analyser's grammar and
//! morphemes, word lists) are read from the DLL's bytes at the addresses
//! its code uses, the abbreviations from the language pack's
//! `Swedish.dbl` with the module's own query. Nothing is copied, nothing
//! runs.

mod allofono;
mod fon;
mod les;
mod morf;
mod numeri;
#[cfg(test)]
mod trace;

use super::common::{Pe, Phone, Word};
pub(crate) use morf::{Analyses, Entry};

/// The function table the export walks and how many pairs it holds.
const FUNCTIONS: u32 = 0x10046ec0;
const FUNCTION_COUNT: u32 = 26;

/// Character classes (`ELQGetGraphemes` 0x10009d00): 8 bytes per
/// character (lower case, upper case, two unused, flags: 0x01 letter, 0x04
/// digit, 0x08 consonant, 0x10 vowel, 0x20 upper case, 0x40 lower case,
/// 0x80 accented vowel, 0x100 punctuation).
const CHARS: u32 = 0x100462d8;
pub(crate) const LETTER: u32 = 0x01;
pub(crate) const CONSONANT: u32 = 0x08;
pub(crate) const VOWEL: u32 = 0x10;
pub(crate) const UPPER: u32 = 0x20;
pub(crate) const ACCENTED: u32 = 0x80;

/// The module's phone symbols (`ELQGetPhonemes` 0x10009d10: `{entries,
/// u16 count}` at 0x10046da8, 8 bytes an entry: symbol, code).
const SYMBOLS: u32 = 0x10046da8;

/// The language module, read from `LTTS7Swedish.dll`'s bytes and the
/// language pack's dictionary.
pub struct Swedish {
    pub(crate) pe: Pe,
    /// (lower, upper, flags) per character, from `CHARS`
    chars: Vec<(u8, u8, u32)>,
    /// the analyser's grammar (descriptor 0x10009f40 +0x10)
    nodes: Vec<morf::Node>,
    /// `SELECT grafemica,abbreviazione FROM lessico WHERE abbreviazione!=""
    /// ORDER BY abbreviazione COLLATE NOCASE DESC` (data +0xc,
    /// "Abbreviation_Swedish"): (as written, expansion), in that order
    /// (LesGetAbbreviazioni 0x1000a3d0 hands out row i)
    abbreviations: Vec<(Vec<u8>, Vec<u8>)>,
    /// the engine's table built from them, on first use
    abbreviation_table: std::sync::OnceLock<Vec<super::common::Abbreviation>>,
    /// (symbol, code) from `SYMBOLS`
    symbols: Vec<(Vec<u8>, u32)>,
}

impl Swedish {
    /// The module, when `dll` is `LTTS7Swedish.dll` as the pack ships it
    /// (its function table at 0x10046ec0) and `dbl` its dictionary.
    pub fn new(dll: Vec<u8>, dbl: Option<&[u8]>) -> Option<Swedish> {
        let pe = Pe::parse(dll)?;
        let table = pe.function_table(FUNCTIONS, FUNCTION_COUNT);
        let name = |i: usize| table.get(i).map(|(n, _)| n.as_slice());
        if name(0) != Some(b"GetVersion") || name(24) != Some(b"FonTrasParola") || name(25) != Some(b"FonAllofonoLingua") {
            return None;
        }
        let chars = (0..256u32)
            .map(|c| (pe.u8(CHARS + 8 * c), pe.u8(CHARS + 8 * c + 1), pe.u32(CHARS + 8 * c + 4)))
            .collect();
        let entries = pe.u32(SYMBOLS);
        let n = pe.u16(SYMBOLS + 4) as u32;
        let symbols = (0..n).map(|k| (pe.cstr(pe.u32(entries + 8 * k)).to_vec(), pe.u32(entries + 8 * k + 4))).collect();
        let nodes = morf::read_nodes(&pe);
        let abbreviations = abbreviations(dbl);
        Some(Swedish {
            pe,
            chars,
            nodes,
            abbreviations,
            abbreviation_table: Default::default(),
            symbols,
        })
    }

    pub(crate) fn flags(&self, c: u8) -> u32 {
        self.chars[c as usize].2
    }
    pub(crate) fn is_vowel(&self, c: u8) -> bool {
        self.flags(c) & VOWEL != 0
    }
    pub(crate) fn is_consonant(&self, c: u8) -> bool {
        self.flags(c) & CONSONANT != 0
    }
    pub(crate) fn is_upper(&self, c: u8) -> bool {
        self.flags(c) & UPPER != 0
    }
    /// The table's lower case of `c` (`DAT_100462d8`).
    pub(crate) fn lower(&self, c: u8) -> u8 {
        self.chars[c as usize].0
    }
    /// The NUL-terminated string at `va` in the module.
    pub(crate) fn s(&self, va: u32) -> &[u8] {
        self.pe.cstr(va)
    }

    /// The symbol of a phone code.
    pub fn symbol(&self, code: u32) -> Option<&str> {
        self.symbols.iter().find(|&&(_, c)| c == code).and_then(|(s, _)| std::str::from_utf8(s).ok())
    }
}

/// The abbreviation query (0x10009f40), SQLite's stable sort with NOCASE
/// folding ASCII only.
fn abbreviations(dbl: Option<&[u8]>) -> Vec<(Vec<u8>, Vec<u8>)> {
    use crate::sqlite::Value;
    let rows = dbl.and_then(|d| crate::sqlite::read_table(d, "lessico").ok()).unwrap_or_default();
    let col = |r: &[Value], i: usize| match r.get(i) {
        Some(Value::Text(b)) => Some(b.clone()),
        _ => None,
    };
    let fold = |b: &[u8]| b.iter().map(u8::to_ascii_lowercase).collect::<Vec<u8>>();
    let mut t: Vec<(Vec<u8>, Vec<u8>)> = rows
        .iter()
        .filter_map(|r| {
            let a = col(r, 2).filter(|a| !a.is_empty())?;
            Some((a, col(r, 0).unwrap_or_default()))
        })
        .collect();
    t.sort_by(|a, b| fold(&b.0).cmp(&fold(&a.0)));
    t
}

/// A record's LexInfo (LesAllocLanguageLexInfo 0x10009e30: 0x608 bytes,
/// +0 the analyses, +4 0x40 stress slots of 0x18 bytes: `{pos, kind,
/// flag, pos2, kind2, flag2}`, the n-th vowel of the morpheme carrying
/// accent `kind` (1 or 2), a second accent in `pos2`/`kind2`). Kept in
/// `Word::data` as the 0x40·6 slot values, then the analyses (count, then
/// per analysis its morpheme count and `start, end, entry` triples).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LexInfo {
    pub slots: Vec<[i32; 6]>,
    pub analyses: Analyses,
}

const SLOTS: usize = 0x40;

impl LexInfo {
    pub(crate) fn of(w: &Word) -> LexInfo {
        let d = &w.data;
        let mut slots = vec![[0i32; 6]; SLOTS];
        if d.len() < SLOTS * 6 + 1 {
            return LexInfo { slots, analyses: Vec::new() };
        }
        for (i, s) in slots.iter_mut().enumerate() {
            for (k, v) in s.iter_mut().enumerate() {
                *v = d[i * 6 + k] as i32;
            }
        }
        let mut at = SLOTS * 6;
        let n = d[at] as usize;
        at += 1;
        let mut analyses = Vec::with_capacity(n);
        for _ in 0..n {
            let c = d[at] as usize;
            at += 1;
            let mut sol = Vec::with_capacity(c);
            for _ in 0..c {
                sol.push(Entry {
                    start: d[at] as i32,
                    end: d[at + 1] as i32,
                    m: d[at + 2],
                });
                at += 3;
            }
            analyses.push(sol);
        }
        LexInfo { slots, analyses }
    }

    pub(crate) fn store(&self, w: &mut Word) {
        let mut d = Vec::with_capacity(SLOTS * 6 + 1);
        for s in &self.slots {
            d.extend(s.iter().map(|&v| v as u32));
        }
        d.push(self.analyses.len() as u32);
        for sol in &self.analyses {
            d.push(sol.len() as u32);
            for e in sol {
                d.extend([e.start as u32, e.end as u32, e.m]);
            }
        }
        w.data = d;
    }

    /// The best analysis (`FUN_10008cf0(results, 0)`), when there is one.
    pub(crate) fn best(&self) -> Option<&[Entry]> {
        self.analyses.first().map(|v| v.as_slice())
    }
}

impl super::common::Les for Swedish {
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
    fn elabora_parola_at(&self, toks: &mut [super::common::Tok], i: usize, idx: usize, spell: u8, mode: u8) -> Option<(Vec<Word>, usize)> {
        Some((Swedish::elabora_parola(self, toks, i, idx, spell, mode), 0))
    }
    fn analizza(&self, w: &mut [Word]) {
        Swedish::analizza(self, w)
    }
    fn accenta(&self, w: &mut [Word]) {
        Swedish::accenta(self, w)
    }
    fn confini(&self, w: &mut [Word], mode: i32) {
        Swedish::confini(self, w, mode)
    }
    fn gradi(&self, w: &mut [Word]) {
        Swedish::gradi(self, w)
    }
    fn tras_parola(&self, w: &[Word], i: usize) -> Vec<u32> {
        Swedish::tras_parola(self, w, i)
    }
    fn allofono(&self, w: &[Word], phones: &mut Vec<Phone>, i: usize) -> usize {
        Swedish::allofono(self, w, phones, i)
    }
    fn symbol(&self, code: u32) -> Option<&str> {
        Swedish::symbol(self, code)
    }
    fn normalizza_accento(&self, w: &mut Vec<u8>) {
        Swedish::normalizza_accento(self, w)
    }
    fn pronunciabile(&self, w: &[u8]) -> bool {
        self.pronounceable(w, 0, true)
    }
    fn pronunciabile_flag(&self, w: &[u8], flag: u8) -> bool {
        self.pronounceable(w, flag, true)
    }
    fn symbol_name(&self, c: u8, mode: u8) -> Vec<u8> {
        Swedish::symbol_name(self, c, mode)
    }
    fn abbreviazioni(&self) -> &[super::common::Abbreviation] {
        self.abbreviation_table.get_or_init(|| super::common::abbreviation_table(self, &self.abbreviations))
    }
}

impl super::Module for Swedish {
    fn analyse(&self, text: &str, voice: Option<&crate::voicefon::VoiceFon>) -> Vec<crate::g2p::Token> {
        super::common::analyse(self, text, voice)
    }
}
