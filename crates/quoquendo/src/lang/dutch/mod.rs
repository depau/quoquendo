//! Dutch, ported from the original's language module `LTTS7Dutch.dll`
//! (MECHANICS.md §4g). The module's one export, `Dutch` (0x10001790),
//! hands LoqTTS7.dll a table of 22 named functions (0x100122a0); the ones
//! the LES and FON stages run are ported here, each under its address. Its
//! tables (character classes, the function-word table, the compound word
//! list, the phone symbols) are read from the module's own bytes at the
//! addresses its code uses; the dictionary rows come from the language
//! pack's `Dutch.dbl` and the letter-to-sound model from `Dutch.gpr`
//! (LTTS7GPR.dll's `ELQGPRRun`, `gpr.rs`).
//!
//! The module has no `LesAccenta`, `LesConfiniProsodici`,
//! `LesSetModalitaFrase` nor `ELQGetLanguageNativeEncoding`: the engine
//! skips the first and runs its own for the others (LoqTTS7.dll
//! 0x100409c0, 0x100350e0; Windows-1252).

mod allofono;
mod confini;
mod les;
mod numeri;
mod parola;
#[cfg(test)]
mod trace;

use super::common::{Pe, Phone, Tok, Word};
use crate::g2p::Token;
use crate::gpr::Gpr;

/// The function table's address (22 `{ name, function }` pairs).
const FUNCTIONS: u32 = 0x100122a0;

/// Character classes (`ELQGetGraphemes` 0x10001000 → 0x10011670): 8 bytes
/// per character, lower case, upper case, then flags at +4 (1 letter, 2
/// part of an acronym, 8 consonant, 0x10 vowel, 0x20 upper case, 0x80
/// accented, 0x100 punctuation).
const CHARS: u32 = 0x10011670;
pub(crate) const LETTER: u32 = 0x1;
pub(crate) const CONSONANT: u32 = 0x8;
pub(crate) const VOWEL: u32 = 0x10;
pub(crate) const UPPER: u32 = 0x20;
pub(crate) const ACCENTED: u32 = 0x80;

/// The module's phone table (`ELQGetPhonemes` 0x10001010 → 0x10012138:
/// `{ entries, u16 count }`, entries `{ symbol, code }`).
const PHONES: u32 = 0x10012138;

/// The language module, read from `LTTS7Dutch.dll`, `Dutch.dbl` and
/// `Dutch.gpr`.
pub struct Dutch {
    pub(crate) pe: Pe,
    chars: Vec<(u8, u8, u32)>,
    /// `SELECT grafemica, fonetica FROM lessico` (0x10001100), in table
    /// order; `None` for SQL NULL
    dict: Vec<(Vec<u8>, Option<Vec<u8>>)>,
    /// `SELECT grafemica,abbreviazione FROM lessico WHERE
    /// abbreviazione!="" ORDER BY abbreviazione COLLATE NOCASE DESC`
    /// (0x10001100, `LesGetAbbreviazioni` 0x10001620) as (abbreviation,
    /// expansion)
    abbreviations: Vec<(Vec<u8>, Vec<u8>)>,
    abbreviation_table: std::sync::OnceLock<Vec<super::common::Abbreviation>>,
    /// the channel's converter (FON channel +0x20c, 0x10006f40): none
    /// without a `GPR` file
    gpr: Option<Gpr>,
}

impl Dutch {
    /// The module's tables, when `files.dll` is `LTTS7Dutch.dll` as the pack
    /// ships it (the function table at 0x100122a0).
    pub fn from_files(files: &super::ModuleFiles) -> Option<Dutch> {
        let pe = Pe::parse(files.dll.clone())?;
        let t = pe.function_table(FUNCTIONS, 22);
        let named = |i: usize, n: &[u8], a: u32| t.get(i).is_some_and(|(name, at)| name == n && *at == a);
        if !named(11, b"LesAnalizza", 0x10007850)
            || !named(14, b"LesElaboraParola", 0x100074c0)
            || !named(15, b"FonTrasParola", 0x10008420)
            || !named(16, b"FonTrasConfini", 0x100089f0)
            || !named(17, b"FonAllofonoLingua", 0x10009980)
        {
            return None;
        }
        let chars = (0..256u32)
            .map(|c| (pe.u8(CHARS + 8 * c), pe.u8(CHARS + 8 * c + 1), pe.u32(CHARS + 8 * c + 4)))
            .collect();
        let (dict, abbreviations) = match files.dbl.as_deref() {
            Some(d) => read_dictionary(d)?,
            None => (Vec::new(), Vec::new()),
        };
        Some(Dutch {
            pe,
            chars,
            dict,
            abbreviations,
            abbreviation_table: Default::default(),
            gpr: files.gpr.as_deref().and_then(Gpr::parse),
        })
    }

    pub(crate) fn flags(&self, c: u8) -> u32 {
        self.chars[c as usize].2
    }

    pub(crate) fn has(&self, c: u8, f: u32) -> bool {
        self.flags(c) & f != 0
    }

    /// The character table's lower case (byte 0).
    pub(crate) fn lower(&self, c: u8) -> u8 {
        self.chars[c as usize].0
    }

    /// The character table's upper case (byte 1).
    pub(crate) fn upper(&self, c: u8) -> u8 {
        self.chars[c as usize].1
    }

    /// The string at `va` in the module.
    pub(crate) fn s(&self, va: u32) -> &[u8] {
        self.pe.cstr(va)
    }

    /// LTTS7Util's `ELQDBTableFind` (0x10005df0) on the dictionary by
    /// `grafemica`: a binary search with `ELQstricmp` (A–Z folded) over the
    /// rows in table order, then the equal rows around the hit. The first
    /// matching row and their count.
    fn table_find(&self, key: &[u8]) -> (usize, usize) {
        let g = |i: usize| self.dict[i].0.as_slice();
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

    /// The module's phone symbols.
    fn phone_table(&self) -> impl Iterator<Item = (&[u8], u32)> {
        let ent = self.pe.u32(PHONES);
        let n = self.pe.u16(PHONES + 4) as u32;
        (0..n).map(move |k| (self.s(self.pe.u32(ent + 8 * k)), self.pe.u32(ent + 8 * k + 4)))
    }

    /// 0x10007e80: a symbol's code (0x10 for none or empty; the table's
    /// first four entries give 0 when their code is 0, later ones 0x10).
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

type Dictionary = (Vec<(Vec<u8>, Option<Vec<u8>>)>, Vec<(Vec<u8>, Vec<u8>)>);

/// `lessico` (grafemica, fonetica, abbreviazione) as the module's two
/// queries read it.
fn read_dictionary(dbl: &[u8]) -> Option<Dictionary> {
    use crate::sqlite::Value;
    let rows = crate::sqlite::read_table(dbl, "lessico").ok()?;
    let v = |r: &Vec<Value>, i: usize| match r.get(i) {
        Some(Value::Text(t)) | Some(Value::Blob(t)) => Some(t.clone()),
        Some(Value::Int(n)) => Some(n.to_string().into_bytes()),
        Some(Value::Real(x)) => Some(x.to_string().into_bytes()),
        _ => None,
    };
    let dict = rows.iter().map(|r| (v(r, 0).unwrap_or_default(), v(r, 1))).collect();
    let mut abbr: Vec<(Vec<u8>, Vec<u8>)> = rows
        .iter()
        .filter_map(|r| Some((v(r, 2).filter(|a| !a.is_empty())?, v(r, 0).unwrap_or_default())))
        .collect();
    // COLLATE NOCASE folds A–Z only; DESC, ties in table order
    let fold = |s: &[u8]| s.iter().map(|c| c.to_ascii_lowercase()).collect::<Vec<u8>>();
    abbr.sort_by(|a, b| fold(&b.0).cmp(&fold(&a.0)));
    Some((dict, abbr))
}

impl super::common::Les for Dutch {
    fn letter(&self, c: u8) -> (bool, bool, u8) {
        let f = self.flags(c);
        (f & LETTER != 0, f & UPPER != 0, self.lower(c))
    }
    fn analizza(&self, w: &mut [Word]) {
        self.analizza(w)
    }
    fn accenta(&self, _w: &mut [Word]) {
        // no LesAccenta: the engine's slot stays empty (0x10036140)
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
    fn lower(&self, c: u8) -> u8 {
        Dutch::lower(self, c)
    }
    fn normalizza_accento(&self, w: &mut Vec<u8>) {
        les::normalizza_accento(w)
    }
    fn pronunciabile(&self, w: &[u8]) -> bool {
        self.pronunciabile(w, 0)
    }
    fn pronunciabile_flag(&self, w: &[u8], flag: u8) -> bool {
        self.pronunciabile(w, flag)
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
        self.tras_confini(sentence.get(first + i), phones, next)
    }
    fn allofono_in(&self, sentence: &[Word], first: usize, _chunk: &[Word], phones: &mut Vec<Phone>, i: usize) -> usize {
        self.allofono(sentence, first, phones, i)
    }
    fn symbol(&self, code: u32) -> Option<&str> {
        Dutch::symbol(self, code)
    }
    fn leggi_numero(&self, toks: &mut [Tok], i: usize, kind: u32, dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
        numeri::leggi_numero(self, toks, i, kind, dmy)
    }
}

impl super::Module for Dutch {
    fn analyse(&self, text: &str, voice: Option<&crate::voicefon::VoiceFon>) -> Vec<Token> {
        super::common::analyse(self, text, voice)
    }
}
