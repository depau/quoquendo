//! Danish, ported from the original's language module `LTTS7Danish.dll`
//! (MECHANICS.md §4k). The module's one export, `Danish` (0x10003120),
//! accepts a descriptor whose `Language` list has `danish` and hands the
//! engine a table of 46 `{name, function}` pairs (0x100251b8); the ones
//! the LES and FON stages run are ported here, each under its address.
//!
//! Its words go through a morphological analyser linked into the module
//! ("WordAnalyzer", `lang::wordanalyzer`: initAnalyzer 0x1001bbc0, its
//! tables in the module's data): each solution is a list of morphemes
//! (stems, prefixes, suffixes, compound connectors) whose lexicon entries
//! carry a respelling (`godmorgen` → `gom\xe5\xe5n`, with `:` long, `~`
//! and `?` marks). LesElaboraParola keeps the solutions in the record's
//! LexInfo (`LexInfo`), LesAnalizza picks one and the word class,
//! LesAccenta builds the marked respelling FonTrasParola reads letter by
//! letter.
//!
//! Its tables are read from the module's own bytes at the addresses its
//! code uses; the abbreviations come from the language pack's `Danish.dbl`
//! as its loader (0x10002aa0) reads them.

mod accenta;
mod allofono;
mod analizza;
mod elabora;
mod numeri;
mod parola;
#[cfg(test)]
mod trace;

use super::common::{Pe, Phone, Tok, Word};
use super::wordanalyzer::Analyzer;
use crate::g2p::Token;

/// The function table's address (46 `{ name, function }` pairs).
const FUNCTIONS: u32 = 0x100251b8;

/// Character classes (`ELQGetGraphemes` 0x100023d0 → 0x10024200): 8 bytes
/// per character, lower case, upper case, then flags at +4 (1 letter, 2
/// part of an acronym, 4 digit, 8 consonant, 0x10 vowel, 0x20 upper case,
/// 0x40 lower case, 0x80 accented, 0x100 punctuation).
pub(crate) const CHARS: u32 = 0x10024200;
pub(crate) const LETTER: u32 = 0x1;
pub(crate) const ACRONYM: u32 = 0x2;
pub(crate) const DIGIT: u32 = 0x4;
pub(crate) const CONSONANT: u32 = 0x8;
pub(crate) const VOWEL: u32 = 0x10;
pub(crate) const UPPER: u32 = 0x20;
pub(crate) const LOWER: u32 = 0x40;
pub(crate) const ACCENTED: u32 = 0x80;
pub(crate) const PUNCT: u32 = 0x100;

/// The module's phone table (`ELQGetPhonemes` 0x100023e0 → 0x10024de0:
/// `{ entries, u16 count }`, entries `{ symbol, code }`, 124 of them).
const PHONES: u32 = 0x10024de0;

/// The analyser the module's initAnalyzer (0x1001bbc0 → 0x1001bb40) sets
/// up: 0x1ad nodes of 0x14 bytes at 0x10076308.
const ANALYZER_NODES: u32 = 0x10076308;
const ANALYZER_COUNT: u32 = 0x1ad;

/// One morpheme of an analyser solution as LesElaboraParola (0x10004a30)
/// copies it into the LexInfo buffer: getMorphemeInfo's (0x1001bd60) ten
/// fields, 0x28 bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Morpheme {
    /// +0 the lexicon entry's spelling (empty for an empty suffix)
    pub text: Vec<u8>,
    /// +4 the morpheme type's name (`STEM`, `FLEX_SUFFIX`, `DERIV_SUFFIX`,
    /// `PREFIX`, `COMPOUND_CONNECTOR`)
    pub kind: Vec<u8>,
    /// +8 the entry's feature pair (`{u32, char*}`, 8 bytes in the
    /// module's data): its two dwords, the pointer as a module address; 0s
    /// for none
    pub feat: [u32; 2],
    /// +0xc the entry's respelling (the string after its spelling), when
    /// it has one
    pub gloss: Option<Vec<u8>>,
    /// +0x10 the entry's group (`+8` of the lexicon entry; LesElaboraParola
    /// may change a stem's from the suffix after it)
    pub group: i16,
    /// +0x14 the group's descriptor (a module address, 0 for none)
    pub grouping: u32,
    /// +0x18 the entry's length (its `+0xa`)
    pub len: i32,
    /// +0x1c the respelling's length
    pub gloss_len: i32,
    /// +0x20, +0x24 where the morpheme starts and ends in the word
    pub start: i32,
    pub end: i32,
}

/// One letter of the marked respelling (LexInfo +0x14c, 64 × 4 bytes): the
/// morpheme it comes from (-1 none), a byte, and its flags (1 stem, 2
/// inflection, 3 derivation, 4 prefix, 5 connector in the low bits; 0x10
/// compound start, 0x20 a `-` after it, 0x40 the first letter, 0x80 the
/// last, 0x100 `^` stress, 0x200 `^^`, 0x400 `"`, 0x800 `?`, 0x1000 `~`,
/// 0x2000 `:`, 0x4000 from an entry's respelling).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Letter {
    pub morph: i8,
    pub b1: u8,
    pub flags: u16,
}

impl Default for Letter {
    fn default() -> Letter {
        Letter { morph: -1, b1: 0, flags: 0 }
    }
}

/// A word record's LexInfo (LesAllocLanguageLexInfo 0x10002630, 0x294
/// bytes behind the record's +0x14).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LexInfo {
    /// its first 0x14c bytes as dwords (`[1]` the current solution, `[3]`…
    /// LesAccenta's stress data); `[0]` (the solutions buffer) and `[2]`
    /// (the phonetization rows' list) are pointers, kept 0
    pub head: [i32; 0x53],
    /// +0x14c
    pub letters: [Letter; 64],
    /// +0x250 the respelling, its marks moved into `letters`
    pub spelling: Vec<u8>,
    /// the solutions LesElaboraParola kept (buffer +4, 0x194 bytes each,
    /// their morphemes and the count at +0x190; at most 10)
    pub solutions: Vec<Vec<Morpheme>>,
}

impl Default for LexInfo {
    fn default() -> LexInfo {
        LexInfo {
            head: [0; 0x53],
            letters: [Letter::default(); 64],
            spelling: Vec::new(),
            solutions: Vec::new(),
        }
    }
}

/// Byte strings packed into dwords: the length, then 4 bytes per dword.
fn put_bytes(out: &mut Vec<u32>, b: &[u8]) {
    out.push(b.len() as u32);
    out.extend(b.chunks(4).map(|c| {
        let mut x = [0u8; 4];
        x[..c.len()].copy_from_slice(c);
        u32::from_le_bytes(x)
    }));
}

struct Reader<'a> {
    d: &'a [u32],
    i: usize,
}

impl Reader<'_> {
    fn u(&mut self) -> u32 {
        let x = self.d.get(self.i).copied().unwrap_or(0);
        self.i += 1;
        x
    }
    fn bytes(&mut self) -> Vec<u8> {
        let n = self.u() as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n.div_ceil(4) {
            v.extend(self.u().to_le_bytes());
        }
        v.truncate(n);
        v
    }
}

impl LexInfo {
    /// The LexInfo kept in a record's `data` (`store`), when it has one.
    pub(crate) fn of(w: &Word) -> Option<LexInfo> {
        if w.data.is_empty() {
            return None;
        }
        let mut r = Reader { d: &w.data, i: 0 };
        let mut l = LexInfo::default();
        for h in l.head.iter_mut() {
            *h = r.u() as i32;
        }
        for t in l.letters.iter_mut() {
            let x = r.u();
            *t = Letter {
                morph: x as u8 as i8,
                b1: (x >> 8) as u8,
                flags: (x >> 16) as u16,
            };
        }
        l.spelling = r.bytes();
        let ns = r.u();
        for _ in 0..ns {
            let nm = r.u();
            let mut sol = Vec::new();
            for _ in 0..nm {
                let text = r.bytes();
                let kind = r.bytes();
                let feat = [r.u(), r.u()];
                let gloss = (r.u() != 0).then(|| r.bytes());
                sol.push(Morpheme {
                    text,
                    kind,
                    feat,
                    gloss,
                    group: r.u() as i32 as i16,
                    grouping: r.u(),
                    len: r.u() as i32,
                    gloss_len: r.u() as i32,
                    start: r.u() as i32,
                    end: r.u() as i32,
                });
            }
            l.solutions.push(sol);
        }
        Some(l)
    }

    /// Keeps the LexInfo in the record's `data`.
    pub(crate) fn store(&self, w: &mut Word) {
        let d = &mut w.data;
        d.clear();
        d.extend(self.head.iter().map(|&h| h as u32));
        d.extend(self.letters.iter().map(|t| t.morph as u8 as u32 | (t.b1 as u32) << 8 | (t.flags as u32) << 16));
        put_bytes(d, &self.spelling);
        d.push(self.solutions.len() as u32);
        for sol in &self.solutions {
            d.push(sol.len() as u32);
            for m in sol {
                put_bytes(d, &m.text);
                put_bytes(d, &m.kind);
                d.extend(m.feat);
                match &m.gloss {
                    Some(g) => {
                        d.push(1);
                        put_bytes(d, g);
                    }
                    None => d.push(0),
                }
                d.extend([m.group as i32 as u32, m.grouping, m.len as u32, m.gloss_len as u32, m.start as u32, m.end as u32]);
            }
        }
    }

    /// The current solution (`[1]`), when there is one.
    pub(crate) fn solution(&self) -> Option<&[Morpheme]> {
        self.solutions.get(self.head[1] as usize).map(|s| s.as_slice())
    }
}

/// The language module, read from `LTTS7Danish.dll` and `Danish.dbl`.
pub struct Danish {
    pub(crate) pe: Pe,
    chars: Vec<(u8, u8, u32)>,
    /// the number tables the engine's reader uses (`numeri`)
    num_tables: super::loqnum::Tables,
    /// LTTS7Util's character classes, for the number classifier's expressions
    classes: Option<super::textfilter::Classes>,
    /// LoqTTS7.dll, for the engine's own number tables
    engine: Option<Pe>,
    pub(crate) analyzer: Analyzer,
    /// `SELECT AB_Abbreviation, AB_Expansion FROM TD_Abbreviations WHERE
    /// ifnull(AB_Abbreviation,'') <> '' ORDER BY AB_Abbreviation COLLATE
    /// NOCASE DESC` (0x10002aa0, `LesGetAbbreviazioni` 0x10002fe0)
    abbreviations: Vec<(Vec<u8>, Vec<u8>)>,
    abbreviation_table: std::sync::OnceLock<Vec<super::common::Abbreviation>>,
    /// `SELECT PH_Graphemic, PH_Phonetic FROM TD_Phonetizations ORDER BY
    /// PH_Graphemic COLLATE BINARY ASC` (0x10002aa0): whole-word
    /// pronunciations, a word's rows in its LexInfo +8 list
    pub(crate) phonetizations: Vec<(Vec<u8>, Vec<u8>)>,
}

impl Danish {
    /// The module's tables, when `files.dll` is `LTTS7Danish.dll` as the
    /// pack ships it (the function table at 0x100251b8) and `files.dbl` its
    /// dictionary.
    pub fn from_files(files: &super::ModuleFiles) -> Option<Danish> {
        let pe = Pe::parse(files.dll.clone())?;
        let t = pe.function_table(FUNCTIONS, 46);
        let named = |i: usize, n: &[u8], a: u32| t.get(i).is_some_and(|(name, at)| name == n && *at == a);
        if !named(9, b"LesElaboraParola", 0x10004d60) || !named(13, b"FonTrasParola", 0x1000bc20) || !named(14, b"FonAllofonoLingua", 0x10017400) || !named(34, b"LesAnalizza", 0x1000b620) {
            return None;
        }
        let chars = (0..256u32)
            .map(|c| (pe.u8(CHARS + 8 * c), pe.u8(CHARS + 8 * c + 1), pe.u32(CHARS + 8 * c + 4)))
            .collect();
        let (abbreviations, phonetizations) = match files.dbl.as_deref() {
            Some(d) => read_dictionary(d)?,
            None => (Vec::new(), Vec::new()),
        };
        let num_tables = super::loqnum::Tables::read(&pe, numeri::MORPHS, numeri::CLASSIFY, numeri::CURRENCY, numeri::ORDINALS);
        Some(Danish {
            pe,
            chars,
            analyzer: Analyzer {
                nodes: ANALYZER_NODES,
                count: ANALYZER_COUNT,
                accept: None,
            },
            abbreviations,
            abbreviation_table: Default::default(),
            phonetizations,
            num_tables,
            classes: files.util.clone().and_then(Pe::parse).and_then(|u| super::textfilter::Classes::from_util(&u)),
            engine: files.engine.clone().and_then(Pe::parse),
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

    /// The module's phone symbols.
    fn phone_table(&self) -> impl Iterator<Item = (&[u8], u32)> {
        let ent = self.pe.u32(PHONES);
        let n = self.pe.u16(PHONES + 4) as u32;
        (0..n).map(move |k| (self.s(self.pe.u32(ent + 8 * k)), self.pe.u32(ent + 8 * k + 4)))
    }

    /// 0x1000bac0: a symbol's code (0x10 for none).
    pub(crate) fn phone_code(&self, sym: &[u8]) -> u32 {
        self.phone_table().find(|&(s, _)| s == sym).map_or(0x10, |(_, c)| c)
    }

    pub fn symbol(&self, code: u32) -> Option<&str> {
        self.phone_table()
            .find(|&(_, c)| c == code)
            .and_then(|(s, _)| std::str::from_utf8(s).ok())
    }
}

/// The abbreviations (`TD_Abbreviations`, NOCASE DESC) and the
/// phonetizations (`TD_Phonetizations`, BINARY ASC).
fn read_dictionary(dbl: &[u8]) -> Option<(Vec<(Vec<u8>, Vec<u8>)>, Vec<(Vec<u8>, Vec<u8>)>)> {
    use crate::sqlite::Value;
    let text = |r: &Vec<Value>, i: usize| match r.get(i) {
        Some(Value::Text(t)) | Some(Value::Blob(t)) => Some(t.clone()),
        Some(Value::Int(n)) => Some(n.to_string().into_bytes()),
        _ => None,
    };
    let mut abbr: Vec<(Vec<u8>, Vec<u8>)> = crate::sqlite::read_table(dbl, "TD_Abbreviations")
        .ok()?
        .iter()
        .filter_map(|r| Some((text(r, 0).filter(|a| !a.is_empty())?, text(r, 1).unwrap_or_default())))
        .collect();
    // COLLATE NOCASE folds A–Z only; DESC, ties in table order
    let fold = |s: &[u8]| s.iter().map(|c| c.to_ascii_lowercase()).collect::<Vec<u8>>();
    abbr.sort_by(|a, b| fold(&b.0).cmp(&fold(&a.0)));
    let mut ph: Vec<(Vec<u8>, Vec<u8>)> = crate::sqlite::read_table(dbl, "TD_Phonetizations")
        .ok()?
        .iter()
        .filter_map(|r| Some((text(r, 0)?, text(r, 1).unwrap_or_default())))
        .collect();
    ph.sort_by(|a, b| a.0.cmp(&b.0));
    Some((abbr, ph))
}

impl super::common::Les for Danish {
    fn letter(&self, c: u8) -> (bool, bool, u8) {
        let f = self.flags(c);
        (f & LETTER != 0, f & UPPER != 0, self.lower(c))
    }
    fn code_page(&self) -> u32 {
        // ELQGetLanguageNativeEncoding 0x100024b0
        0x4e4
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
        self.gradi(w)
    }
    fn elabora_parola_at(&self, toks: &mut [Tok], i: usize, idx: usize, spell: u8, mode: u8) -> Option<(Vec<Word>, usize)> {
        Some((self.elabora_parola(toks, i, idx, spell, mode), 0))
    }
    fn is_letter(&self, c: u8) -> bool {
        // LesIsLetter 0x10004fc0
        self.has(c, LETTER)
    }
    fn lower(&self, c: u8) -> u8 {
        Danish::lower(self, c)
    }
    fn normalizza_accento(&self, w: &mut Vec<u8>) {
        elabora::normalizza_accento(w)
    }
    fn pronunciabile(&self, w: &[u8]) -> bool {
        self.pronunciabile(w)
    }
    fn leggi_numero(&self, toks: &mut [super::common::Tok], i: usize, kind: u32, dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
        Danish::leggi_numero(self, toks, i, kind, dmy)
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
    fn tras_parola_in(&self, sentence: &[Word], first: usize, _w: &[Word], i: usize) -> Vec<u32> {
        // FonTrasParola reads the sentence's next record (0x1000b8b0)
        self.tras_parola(sentence, first + i)
    }
    fn allofono(&self, w: &[Word], phones: &mut Vec<Phone>, i: usize) -> usize {
        self.allofono(w, phones, i)
    }
    fn symbol(&self, code: u32) -> Option<&str> {
        Danish::symbol(self, code)
    }
}

impl super::Module for Danish {
    fn analyse(&self, text: &str, voice: Option<&crate::voicefon::VoiceFon>) -> Vec<Token> {
        super::common::analyse(self, text, voice)
    }
}
