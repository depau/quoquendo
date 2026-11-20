//! Russian, ported from the original's language module `LTTS7Russian.dll`
//! (MECHANICS.md §4g). The module's one export, `Russian` (0x1000a1b0),
//! hands LoqTTS7.dll a table of 56 named functions (0x1009f270); the ones
//! the LES and FON stages run are ported here, each under its address. The
//! module has no dictionary file: its lexicon is the morphological
//! analyser it links (`wordanalyzer`, 37 555 morphs), whose analyses carry
//! each morph's stressed form. Its tables are read from the module's own
//! bytes at the addresses its code uses.
//!
//! The module's text is Windows-1251: its `ELQGetLanguageNativeEncoding`
//! (0x10009d70) returns 0x4e3, and the engine converts its UTF-8 input to
//! that code page (`Les::code_page`).

mod allofono;
mod confini;
mod fon;
mod les;
mod numeri;
mod parola;
#[cfg(test)]
mod trace;

use super::common::{Pe, Phone, Tok, Word};
use super::wordanalyzer::{Analyses, Analyzer, Seg};
use crate::g2p::Token;

/// The function table's address (56 `{ name, function }` pairs).
const FUNCTIONS: u32 = 0x1009f270;

/// Character classes (`ELQGetGraphemes` 0x10009c90 → 0x1009e760): 8 bytes
/// per character, lower case, upper case, then flags at +4 (1 letter, 2
/// part of an acronym, 8 consonant, 0x10 vowel, 0x20 upper case, 0x80
/// accented, 0x100 punctuation).
const CHARS: u32 = 0x1009e760;
pub(crate) const LETTER: u32 = 0x1;
pub(crate) const ACRONYM: u32 = 0x2;
pub(crate) const CONSONANT: u32 = 0x8;
pub(crate) const VOWEL: u32 = 0x10;
pub(crate) const UPPER: u32 = 0x20;
pub(crate) const ACCENTED: u32 = 0x80;
pub(crate) const PUNCT: u32 = 0x100;

/// The module's phone table (`ELQGetPhonemes` 0x10009ca0 → 0x1009f168:
/// `{ entries, u16 count }`, entries `{ symbol, code }`).
const PHONES: u32 = 0x1009f168;

/// The analyser object `ELQLanguageLoadReadOnlyData` builds (0x100094a0):
/// 0x4e1 nodes at 0x100975d8, the acceptance test 0x10008fc0. Its +0x24
/// is the morphs' feature table (0x135 entries of `{u8 count, (key,
/// value)*}` at 0x1008d6c0, 0x10010040).
const ANALYZER: Analyzer = Analyzer {
    nodes: 0x100975d8,
    count: 0x4e1,
    accept: Some(accept),
};
const FEATURES: u32 = 0x1008d6c0;
const FEATURE_COUNT: u32 = 0x135;

/// 0x10008fc0, the analyser's acceptance test (+0x34): the root segment
/// (the last one, flag < 0; 0x10010170) is no longer than the rest of the
/// word (0x10010160: the first segment's end).
fn accept(pe: &Pe, path: &[Seg]) -> bool {
    let Some(last) = path.last() else { return false };
    let root = if (pe.u8(last.morph + 0xb) as i8) < 0 { last.end - last.start } else { 0 };
    root <= path[0].end - root
}

/// The language module, read from `LTTS7Russian.dll`.
pub struct Russian {
    pub(crate) pe: Pe,
    chars: Vec<(u8, u8, u32)>,
    /// the number tables the engine's reader uses (`numeri`)
    num_tables: super::loqnum::Tables,
    /// LTTS7Util's character classes, for the number classifier's expressions
    classes: Option<super::textfilter::Classes>,
    /// LoqTTS7.dll, for the engine's own number tables
    engine: Option<Pe>,
}

impl Russian {
    /// The module's tables, when `files.dll` is `LTTS7Russian.dll` as the
    /// pack ships it (the function table at 0x1009f270).
    pub fn from_files(files: &super::ModuleFiles) -> Option<Russian> {
        let pe = Pe::parse(files.dll.clone())?;
        let t = pe.function_table(FUNCTIONS, 56);
        let named = |i: usize, n: &[u8], a: u32| t.get(i).is_some_and(|(name, at)| name == n && *at == a);
        if !named(14, b"FonTrasParola", 0x1000a620) || !named(15, b"FonAllofonoLingua", 0x1000d570) || !named(42, b"LesAnalizza", 0x10008f90) {
            return None;
        }
        let chars = (0..256u32)
            .map(|c| (pe.u8(CHARS + 8 * c), pe.u8(CHARS + 8 * c + 1), pe.u32(CHARS + 8 * c + 4)))
            .collect();
        let num_tables = super::loqnum::Tables::read(&pe, numeri::MORPHS, numeri::CLASSIFY, numeri::CURRENCY, numeri::ORDINALS);
        let classes = files.util.clone().and_then(Pe::parse).and_then(|u| super::textfilter::Classes::from_util(&u));
        let engine = files.engine.clone().and_then(Pe::parse);
        Some(Russian { pe, chars, num_tables, classes, engine })
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

    pub(crate) fn analyse_word(&self, w: &[u8]) -> Analyses {
        ANALYZER.analyse(&self.pe, w)
    }

    /// 0x10010d20 on the feature entry of morph `id` (0x10010040): the
    /// value of feature `key`.
    pub(crate) fn feature(&self, id: i32, key: u8) -> Option<u8> {
        if id < 0 || id as u32 >= FEATURE_COUNT {
            return None;
        }
        let e = FEATURES + 8 * id as u32;
        let n = self.pe.u8(e) as u32;
        let pairs = self.pe.u32(e + 4);
        (0..n).find(|&k| self.pe.u8(pairs + 2 * k) == key).map(|k| self.pe.u8(pairs + 2 * k + 1))
    }

    /// The module's phone symbols.
    fn phone_table(&self) -> impl Iterator<Item = (&[u8], u32)> {
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

/// A word's LexInfo (record +0x14, LesAllocLanguageLexInfo 0x10009eb0,
/// 0x54 bytes): `[0]` its analyses, `[1]` the one in use (a rank),
/// `[3]` the stressed vowel LesAccenta found (-1 none), `+0x10` the
/// spelling LesAccenta stressed. Kept in `Word::data`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Lex {
    pub an: Analyses,
    pub cur: u32,
    pub pos: i32,
    pub form: Vec<u8>,
}

impl Lex {
    pub(crate) fn of(w: &Word) -> Lex {
        let d = &w.data;
        if d.is_empty() {
            return Lex { pos: -1, ..Default::default() };
        }
        let mut k = 0;
        let mut next = || {
            k += 1;
            d[k - 1]
        };
        let pos = next() as i32;
        let cur = next();
        let ns = next() as usize;
        let order = (0..ns).map(|_| next() as usize).collect();
        let slots = (0..ns)
            .map(|_| {
                let n = next() as usize;
                (0..n)
                    .map(|_| Seg {
                        start: next() as i32,
                        end: next() as i32,
                        morph: next(),
                    })
                    .collect()
            })
            .collect();
        let nf = next() as usize;
        let form = (0..nf).map(|_| next() as u8).collect();
        Lex {
            an: Analyses { slots, order },
            cur,
            pos,
            form,
        }
    }

    pub(crate) fn store(&self, w: &mut Word) {
        let d = &mut w.data;
        d.clear();
        d.push(self.pos as u32);
        d.push(self.cur);
        d.push(self.an.slots.len() as u32);
        d.extend(self.an.order.iter().map(|&o| o as u32));
        for s in &self.an.slots {
            d.push(s.len() as u32);
            for g in s {
                d.extend([g.start as u32, g.end as u32, g.morph]);
            }
        }
        d.push(self.form.len() as u32);
        d.extend(self.form.iter().map(|&c| c as u32));
    }

    /// 0x10007e50: the analysis in use, when there is one.
    pub(crate) fn current(&self) -> Option<&[Seg]> {
        if (self.cur as usize) < self.an.slots.len() {
            self.an.ranked(self.cur as usize)
        } else {
            None
        }
    }
}

impl super::common::Les for Russian {
    fn letter(&self, c: u8) -> (bool, bool, u8) {
        let f = self.flags(c);
        (f & LETTER != 0, f & UPPER != 0, self.lower(c))
    }
    fn code_page(&self) -> u32 {
        // ELQGetLanguageNativeEncoding 0x10009d70
        0x4e3
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
    fn elabora_parola_at(&self, toks: &mut [Tok], i: usize, idx: usize, spell: u8, mode: u8) -> Option<(Vec<Word>, usize)> {
        Some((self.elabora_parola(toks, i, idx, spell, mode), 0))
    }
    fn is_letter(&self, c: u8) -> bool {
        // LesIsLetter 0x10003550
        self.has(c, LETTER)
    }
    fn is_upper(&self, c: u8) -> bool {
        // LesIsUpper 0x10003560
        self.has(c, UPPER)
    }
    fn is_part_of_acronym(&self, c: u8) -> bool {
        // LesIsPartOfAcronym 0x10003530
        self.has(c, ACRONYM)
    }
    fn lower(&self, c: u8) -> u8 {
        Russian::lower(self, c)
    }
    fn normalizza_accento(&self, w: &mut Vec<u8>) {
        parola::normalizza_accento(w)
    }
    fn pronunciabile(&self, w: &[u8]) -> bool {
        self.pronunciabile(w, 0)
    }
    fn pronunciabile_flag(&self, w: &[u8], flag: u8) -> bool {
        self.pronunciabile(w, flag)
    }
    fn leggi_numero(&self, toks: &mut [Tok], i: usize, kind: u32, dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
        Russian::leggi_numero(self, toks, i, kind, dmy)
    }
    fn symbol_name(&self, c: u8, mode: u8) -> Vec<u8> {
        self.symbol_name(c, mode)
    }
    fn tras_parola(&self, w: &[Word], i: usize) -> Vec<u32> {
        self.tras_parola(w, 0, w.len(), i)
    }
    fn tras_parola_in(&self, sentence: &[Word], first: usize, w: &[Word], i: usize) -> Vec<u32> {
        self.tras_parola(sentence, first, w.len(), i)
    }
    fn allofono(&self, w: &[Word], phones: &mut Vec<Phone>, i: usize) -> usize {
        self.allofono(w, phones, i)
    }
    fn symbol(&self, code: u32) -> Option<&str> {
        Russian::symbol(self, code)
    }
}

impl super::Module for Russian {
    fn analyse(&self, text: &str, voice: Option<&crate::voicefon::VoiceFon>) -> Vec<Token> {
        super::common::analyse(self, text, voice)
    }
}
