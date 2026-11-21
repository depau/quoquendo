//! English, ported from the original's language module `LTTS7English.dll`
//! (MECHANICS.md §4c). One DLL serves three variants: its export `English`
//! (0x100019f0) reads the descriptor's `Language` list and hands the engine
//! one of three name → function tables (EnglishUs 0x100290d0, 30 pairs;
//! EnglishGb 0x100291c0 and EnglishAu 0x100292a8, 29 pairs). The module
//! reads the language pack's lexical database (`<MotherTongue>.dbl`, its
//! `lessico` table) and letter-to-sound model (`<MotherTongue>.gpr`, run by
//! LTTS7GPR.dll, `gpr.rs`); its own tables are read from the DLL's bytes at
//! the addresses its code uses, never copied.

mod allofono_gb;
mod allofono_us;
mod disambigua;
mod fon;
mod numeri;
mod omografi;
mod prosodia;
mod testo;
#[cfg(test)]
mod trace;

use super::common::{field::*, Endings, Pe, Phone, Word};
use crate::gpr::Gpr;

/// The three variants and their function tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    Us,
    Gb,
    Au,
}

impl Variant {
    /// The name → function table `English` (0x100019f0) hands the engine.
    fn table(self) -> (u32, u32) {
        match self {
            Variant::Us => (0x100290d0, 30),
            Variant::Gb => (0x100291c0, 29),
            Variant::Au => (0x100292a8, 29),
        }
    }

    /// The phone symbol table `ELQGetPhonemes` returns (US 0x10017a80 →
    /// 0x10043180, GB 0x10017290 → 0x10042ec0, AU 0x10011fb0 →
    /// 0x10042940): `{ entries, u16 count }`, entries `{ symbol, code }`.
    fn phonemes(self) -> u32 {
        match self {
            Variant::Us => 0x10043180,
            Variant::Gb => 0x10042ec0,
            Variant::Au => 0x10042940,
        }
    }
}

/// Character classes (0x10027d00): 8 bytes per character, the lower case
/// at +0, flags at +4 (0x10 vowel, 0x20 upper case).
const CHARS: u32 = 0x10027d00;
const UPPER: u32 = 0x20;

/// The columns the module selects from `lessico` (0x10027cd0, joined into
/// `SELECT … FROM lessico` by ELQLanguageLoadReadOnlyData 0x10001460).
pub(crate) const GRAFEMICA: usize = 0;
pub(crate) const FONETICA: usize = 1;
pub(crate) const GRADO: usize = 2;
pub(crate) const GRAMMATICALE: usize = 3;
pub(crate) const GENNUM: usize = 4;
pub(crate) const TEMPOMODO: usize = 5;

/// The lexical database's rows, in the table's own order: LTTS7Util.dll
/// reads the SELECT with `sqlite3_get_table` and searches it with a binary
/// search (`ELQDBTableFind` 0x10005df0) that ignores ASCII case.
pub(crate) struct Lexicon {
    rows: Vec<[Option<Vec<u8>>; 6]>,
}

/// LTTS7Util.dll's `ELQstricmp` (0x100285e0): ASCII letters folded only.
pub(crate) fn stricmp(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    let fold = |c: u8| if c.is_ascii_uppercase() { c + 0x20 } else { c };
    let n = a.len().max(b.len());
    for i in 0..=n {
        let (x, y) = (fold(super::common::at(a, i)), fold(super::common::at(b, i)));
        if x != y || x == 0 {
            return x.cmp(&y);
        }
    }
    std::cmp::Ordering::Equal
}

impl Lexicon {
    fn parse(dbl: &[u8]) -> Option<Lexicon> {
        use crate::sqlite::Value;
        let rows = crate::sqlite::read_table(dbl, "lessico").ok()?;
        let cell = |v: Option<&Value>| match v {
            Some(Value::Text(t)) => Some(t.clone()),
            Some(Value::Int(i)) => Some(i.to_string().into_bytes()),
            Some(Value::Real(r)) => Some(r.to_string().into_bytes()),
            Some(Value::Blob(b)) => Some(b.clone()),
            Some(Value::Null) | None => None,
        };
        Some(Lexicon {
            rows: rows
                .iter()
                .map(|r| [cell(r.first()), cell(r.get(1)), cell(r.get(2)), cell(r.get(3)), cell(r.get(4)), cell(r.get(5))])
                .collect(),
        })
    }

    /// `ELQDBTableFind` (0x10005df0) on `grafemica`: the rows equal to
    /// `key` ignoring case, found by its binary search (which stops at the
    /// first equal row it meets, then widens both ways).
    pub(crate) fn find(&self, key: &[u8]) -> std::ops::Range<usize> {
        use std::cmp::Ordering::*;
        let g = |i: usize| self.rows[i][GRAFEMICA].as_deref().unwrap_or(b"");
        let (mut lo, mut hi) = (0i64, self.rows.len() as i64 - 1);
        while lo <= hi {
            let mid = (lo + hi) / 2;
            match stricmp(key, g(mid as usize)) {
                Less => hi = mid - 1,
                Greater => lo = mid + 1,
                Equal => {
                    let mut first = mid as usize;
                    while first > 0 && stricmp(key, g(first - 1)) == Equal {
                        first -= 1;
                    }
                    let mut end = mid as usize + 1;
                    while end < self.rows.len() && stricmp(key, g(end)) == Equal {
                        end += 1;
                    }
                    return first..end;
                }
            }
        }
        0..0
    }

    pub(crate) fn cell(&self, row: u32, col: usize) -> Option<&[u8]> {
        self.rows[row as usize][col].as_deref()
    }
}

/// A record's `+0x14` list (an `ELQList` of lexicon rows, LTTS7Util.dll),
/// kept in `Word::data` as `[current position + 1 (0: none), rows…]`:
/// adding sets the current row only in an empty list (0x10006940),
/// `GetNext` stops at the last row (0x10007580), deleting the current row
/// makes the one before it current, or the new head (0x10008020).
pub(crate) struct Rows;

impl Rows {
    pub(crate) fn len(w: &Word) -> usize {
        w.data.len().saturating_sub(1)
    }
    pub(crate) fn add(w: &mut Word, row: u32) {
        if w.data.is_empty() {
            w.data.push(0);
        }
        if let Some(p) = w.data[1..].iter().position(|&r| r == row) {
            w.data[0] = p as u32 + 1;
            return;
        }
        w.data.push(row);
        if w.data.len() == 2 {
            w.data[0] = 1;
        }
    }
    pub(crate) fn current(w: &Word) -> Option<u32> {
        match w.data.first() {
            Some(&c) if c > 0 => w.data.get(c as usize).copied(),
            _ => None,
        }
    }
    pub(crate) fn head(w: &mut Word) -> Option<u32> {
        if Self::len(w) == 0 {
            return None;
        }
        w.data[0] = 1;
        Some(w.data[1])
    }
    pub(crate) fn next(w: &mut Word) -> Option<u32> {
        let c = *w.data.first()? as usize;
        if c == 0 || c + 1 >= w.data.len() {
            return None;
        }
        w.data[0] += 1;
        Some(w.data[c + 1])
    }
    pub(crate) fn delete_current(w: &mut Word) {
        let Some(&c) = w.data.first() else { return };
        if c == 0 {
            return;
        }
        w.data.remove(c as usize);
        // the head deleted: the new head is current (0x10008115)
        w.data[0] = if c > 1 { c - 1 } else if w.data.len() > 1 { 1 } else { 0 };
    }
    pub(crate) fn delete_all(w: &mut Word) {
        w.data.clear();
    }
}

/// The language module, read from `LTTS7English.dll`'s bytes and the
/// language pack's database and model.
pub struct English {
    pub(crate) pe: Pe,
    pub(crate) variant: Variant,
    pub(crate) lex: Lexicon,
    pub(crate) gpr: Option<Gpr>,
    /// the variant's phone symbols, (symbol, code)
    symbols: Vec<(String, u32)>,
    chars: Vec<(u8, u32)>,
    /// the engine's abbreviation table from LesGetAbbreviazioni
    abbreviations: std::sync::OnceLock<Vec<super::common::Abbreviation>>,
}

impl English {
    pub fn from_files(files: &super::ModuleFiles) -> Option<English> {
        let variant = match files.mother_tongue.to_ascii_lowercase().as_str() {
            "englishus" => Variant::Us,
            "englishgb" => Variant::Gb,
            "englishau" => Variant::Au,
            _ => return None,
        };
        let pe = Pe::parse(files.dll.clone())?;
        let (table, count) = variant.table();
        let names = pe.function_table(table, count);
        if names.first().map(|n| n.0.as_slice()) != Some(b"GetVersion") || !names.iter().any(|n| n.0 == b"FonAllofonoLingua") {
            return None;
        }
        let lex = Lexicon::parse(files.dbl.as_deref()?)?;
        let gpr = files.gpr.as_deref().and_then(Gpr::parse);
        let t = variant.phonemes();
        let (entries, n) = (pe.u32(t), pe.u16(t + 4) as u32);
        let symbols = (0..n)
            .map(|i| (pe.cstr(pe.u32(entries + 8 * i)).iter().map(|&c| c as char).collect(), pe.u32(entries + 8 * i + 4)))
            .collect();
        let chars = (0..256u32).map(|c| (pe.u8(CHARS + 8 * c), pe.u32(CHARS + 8 * c + 4))).collect();
        Some(English { pe, variant, lex, gpr, symbols, chars, abbreviations: Default::default() })
    }

    pub(crate) fn flags(&self, c: u8) -> u32 {
        self.chars[c as usize].1
    }

    /// The code of a phone symbol in the variant's table (0x1000bf00): 0x10
    /// for an unknown or empty one.
    pub(crate) fn code(&self, sym: &[u8]) -> u32 {
        if sym.is_empty() {
            return 0x10;
        }
        match self.symbols.iter().position(|(s, _)| s.as_bytes() == sym) {
            Some(i) => {
                let c = self.symbols[i].1;
                if c == 0 {
                    if i > 3 {
                        0x10
                    } else {
                        0
                    }
                } else {
                    c
                }
            }
            None => 0x10,
        }
    }

    /// A lexicon cell of a row.
    pub(crate) fn cell(&self, row: u32, col: usize) -> Option<&[u8]> {
        self.lex.cell(row, col)
    }

    /// 0x10001320: walk the record's rows from the head; the first one
    /// whose `col` is set and not empty becomes current.
    pub(crate) fn find_set(&self, w: &mut Word, col: usize) -> bool {
        let mut r = Rows::head(w);
        while let Some(row) = r {
            if self.cell(row, col).is_some_and(|v| !v.is_empty()) {
                return true;
            }
            r = Rows::next(w);
        }
        false
    }

    /// 0x100013b0: add the rows whose `grafemica` is `key`; the list's
    /// length.
    fn look_up(&self, w: &mut Word, key: &[u8]) -> usize {
        for row in self.lex.find(key) {
            Rows::add(w, row as u32);
        }
        Rows::len(w)
    }
}

// --- LesGestisciMaiuscole (0x1000c420), LesAnalizza (0x10011f90) -------------

/// `grammaticale` → word class (0x1000f390).
pub(crate) fn class_of(g: Option<&[u8]>) -> u8 {
    let Some(g) = g else { return 0xc8 };
    match g {
        b"" => 0xc8,
        b"PRON" => 0x20,
        b"PRONP" => 0x23,
        b"PRONR" => 0x22,
        b"VERBO" => 0x2a,
        b"VERBOE" => 0x2c,
        b"VERBOA" => 0x2b,
        b"VERBOMOD" => 0xe5,
        b"NONVERBO" => 0xee,
        b"AVV" => 0x33,
        b"AGG" => 0x29,
        b"SOST" => 0x15,
        b"ARTIC" => 0x18,
        b"CONG" => 0x1f,
        b"ECONG" => 0x82,
        b"PREP" => 0x19,
        b"SIGLA" => 0x83,
        b"NUM" => 0x7a,
        b"PREP_AVV_CONG_AGG" => 0xcf,
        b"PREP_AVV_AGG" => 0xd0,
        b"PREP_AVV_AGG_VERBO" => 0xd1,
        b"PREP_AVV" => 0xd2,
        b"PREP_CONG" => 0xd3,
        b"PREP_CONG_SOST" => 0xd4,
        b"AGG_PRON_AVV" => 0xd5,
        b"AGG_SOST" | b"SOST_AGG" => 0xd6,
        b"AGG_VERBO" => 0xcd,
        b"AGG_PRONP" => 0xd8,
        b"AGG_PRONR_CONG_AVV" => 0xd9,
        b"AGG_PRON" => 0xda,
        b"AGG_AVV" => 0xdb,
        b"AGG_AVV_PRON" => 0xdc,
        b"VERBO_AGG" => 0xcc,
        b"AGG_AVV_VERBO" => 0xdd,
        b"AGG_AVV_INTERJ_SOST_VERBO" => 0xde,
        b"AVV_CONG" => 0xdf,
        b"CONG_PREP_AVV" => 0xe1,
        b"PRON_AVV" => 0xe2,
        b"SOST_AGG_AVV" => 0xe3,
        b"SOST_VERBO" => 0xe4,
        b"VERBO_SOST" => 0xcb,
        b"DB_VERBO_SOST" => 0xe8,
        b"DB_VERBO_AGG" => 0xe9,
        b"DB_AGG_VERBO" => 0xea,
        b"DB_SOST_VERBO" => 0xeb,
        b"DB_AGG_SOST_VERBO" | b"DB_AGG_VERBO_SOST" | b"DB_VERBO_AGG_SOST" | b"DB_VERBO_SOST_AGG" | b"DB_VERBO_SOST_AGG_AVV"
        | b"DB_SOST_AGG_AVV_VERBO" => 0xec,
        b"DB_ARTIC_SIGLA" => 0xed,
        _ => 0x2d,
    }
}

/// `tempomodo` → subclass (0x10010000).
fn sub_of(t: Option<&[u8]>) -> u8 {
    match t {
        Some(b"PART_PASS") => 0x88,
        Some(b"INFINITO") => 0x8d,
        Some(b"GERUNDIO") => 0x8b,
        _ => 0xc8,
    }
}

/// `gennum` → information (0x1000feb0).
fn info_of(g: Option<&[u8]>) -> u8 {
    match g {
        Some(b"FESI") => 0x0d,
        Some(b"MASI") => 0x0b,
        Some(b"XPLU") => 0x10,
        Some(b"XSIN") | Some(b"XSIN3") => 0x0f,
        _ => 0x09,
    }
}

/// `gradoaccentuale` → stress grade (0x100100f0).
pub(crate) fn grade_of(g: Option<&[u8]>) -> u8 {
    match g {
        Some(b"ACC") => 0,
        Some(b"ACD") => 2,
        Some(b"DEA") => 3,
        _ => 0x0e,
    }
}

/// Ending table for words the database lacks (0x1000f1b0: 0x270 entries
/// at 0x10040410), read reversed by 0x1000f240.
const GUESS: Endings = Endings { at: 0x10040410, count: 0x270 };

impl English {
    /// LesGestisciMaiuscole (0x1000c420): every upper-case letter to its
    /// lower case.
    fn gestisci_maiuscole(&self, w: &mut [Word]) {
        for r in w {
            for c in r.text.iter_mut() {
                if self.flags(*c) & UPPER != 0 {
                    *c = self.chars[*c as usize].0;
                }
            }
        }
    }

    /// 0x1000f240: a word the database lacks gets a class from its ending.
    fn guess(&self, r: &mut Word) {
        if r.f[CLASS] != 0x17 {
            return;
        }
        let rev: Vec<u8> = r.text.iter().rev().copied().collect();
        r.f[CLASS] = match super::common::lookup(&self.pe, GUESS, &rev) {
            1 => 0x33,
            2 => 0x2a,
            3 => 0x29,
            5 => 0x2d,
            _ => 0xee,
        };
    }

    /// LesAnalizza (0x10011f90): the database's rows and classes
    /// (0x100119c0), then the context rules (0x100103c0).
    fn analizza(&self, w: &mut [Word]) {
        self.lexical(w);
        disambigua::run(self, w);
    }

    /// 0x100119c0: every word's rows from the database, and its class,
    /// subclass, information and grade from them.
    fn lexical(&self, w: &mut [Word]) {
        for u in 0..w.len() {
            if w[u].f[CLASS] == b'2' {
                continue;
            }
            let r = &mut w[u];
            let text = r.text.clone();
            let mut n = self.look_up(r, &text);
            let mut possessive = false;
            let mut has_gram = false;
            let mut found = false;
            if n == 0 {
                let mut t = text.clone();
                if t.last() == Some(&b'\'') {
                    t.pop();
                } else if t.len() > 1 && t.ends_with(b"'s") {
                    possessive = true;
                    t.truncate(t.len() - 2);
                    n = self.look_up(r, &t);
                    if n != 0 {
                        has_gram = self.find_set(r, GRAMMATICALE);
                        found = has_gram;
                    }
                }
            } else {
                has_gram = self.find_set(r, GRAMMATICALE);
                found = has_gram;
            }
            if !found {
                // LAB_10011b01
                if r.f[CLASS] == 0x17 {
                    if n > 0 {
                        Rows::head(r);
                    }
                    self.guess(r);
                    continue;
                }
            }
            if r.f[CLASS] == 0x17 {
                if n == 1 {
                    if let Some(cur) = Rows::current(r) {
                        if possessive {
                            let c = class_of(self.cell(cur, GRAMMATICALE));
                            if c != 0x15 && c != 0x7a && c != 0x83 {
                                Rows::delete_all(r);
                                continue;
                            }
                        }
                        r.f[CLASS] = class_of(self.cell(cur, GRAMMATICALE));
                        r.f[SUB] = sub_of(self.cell(cur, TEMPOMODO));
                        r.f[INFO] = info_of(self.cell(cur, GENNUM));
                        r.f[GRADE] = grade_of(self.cell(cur, GRADO));
                    }
                } else if n > 1 {
                    let mut combined = b"DB".to_vec();
                    let mut last: Vec<u8> = Vec::new();
                    let mut distinct = 0;
                    let mut cur = Rows::current(r);
                    let mut row = cur;
                    for _ in 0..n {
                        if let Some(rw) = cur {
                            row = Some(rw);
                            if let Some(g) = self.cell(rw, GRAMMATICALE) {
                                if g != last.as_slice() && !g.is_empty() {
                                    combined.push(b'_');
                                    combined.extend_from_slice(g);
                                    last = g.to_vec();
                                    distinct += 1;
                                }
                            }
                        }
                        cur = Rows::next(r);
                    }
                    if distinct > 1 {
                        r.f[CLASS] = class_of(Some(&combined));
                        continue;
                    }
                    r.f[CLASS] = class_of(Some(&last));
                    let row = Rows::current(r).or(row);
                    if let Some(row) = row {
                        r.f[SUB] = sub_of(self.cell(row, TEMPOMODO));
                        r.f[INFO] = info_of(self.cell(row, GENNUM));
                    }
                }
            } else {
                if r.f[CLASS] == 0x15 && r.f[SUB] == b'z' && stricmp(&r.text, b"and").is_eq() {
                    r.f[CLASS] = 0x82;
                    r.f[SUB] = 0xc8;
                    r.f[INFO] = 9;
                }
                if has_gram {
                    let mut ok = Rows::current(r).is_some();
                    while ok {
                        let cur = Rows::current(r);
                        let gram = cur.map(|c| class_of(self.cell(c, GRAMMATICALE))).unwrap_or(0xc8);
                        let sub = r.f[SUB];
                        let delete = if sub == 0x84 || sub == 0x83 {
                            gram != 0x83 && !((sub == b'y' || sub == b'z') && gram == b'z')
                        } else {
                            (sub == b'y' || sub == b'z') && gram != b'z'
                        };
                        if delete {
                            Rows::delete_current(r);
                            ok = Rows::current(r).is_some();
                        } else {
                            ok = Rows::next(r).is_some();
                        }
                    }
                }
            }
        }
    }

    /// FonAllofonoLingua (US 0x100182a0, GB/AU 0x100122c0) at phone `i` of
    /// the chunk that starts at record `first` of the sentence `w`.
    pub(crate) fn allofono(&self, w: &[Word], first: usize, phones: &mut Vec<Phone>, i: usize) -> usize {
        match self.variant {
            Variant::Us => allofono_us::allofono(self, w, first, phones, i),
            Variant::Gb | Variant::Au => allofono_gb::allofono(self, w, first, phones, i),
        }
    }

    pub(crate) fn symbol(&self, code: u32) -> Option<&str> {
        self.symbols.iter().find(|(s, c)| *c == code && !s.is_empty()).map(|(s, _)| s.as_str())
    }
}

// --- The engine's side (LoqTTS7.dll, `common`) calls these --------------------

impl super::common::Les for English {
    fn letter(&self, c: u8) -> (bool, bool, u8) {
        let f = self.flags(c);
        (f & 0x1 != 0, f & UPPER != 0, self.chars[c as usize].0)
    }
    fn number_words(&self, digits: &str) -> Vec<Vec<u8>> {
        english_number_words(digits)
    }
    fn leggi_numero(&self, toks: &mut [super::common::Tok], i: usize, kind: u32, dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
        Some(English::leggi_numero(self, toks, i, kind, dmy))
    }
    fn gestisci_maiuscole(&self, w: &mut [Word]) {
        English::gestisci_maiuscole(self, w)
    }
    fn analizza(&self, w: &mut [Word]) {
        English::analizza(self, w)
    }
    /// The English module has no LesAccenta.
    fn accenta(&self, _w: &mut [Word]) {}
    fn confini(&self, w: &mut [Word], mode: i32) {
        prosodia::confini(self, w, mode)
    }
    fn gradi(&self, w: &mut [Word]) {
        prosodia::gradi(self, w)
    }
    fn tras_parola(&self, w: &[Word], i: usize) -> Vec<u32> {
        fon::tras_parola(self, w, i)
    }
    fn symbol(&self, code: u32) -> Option<&str> {
        English::symbol(self, code)
    }
    fn pronunciabile(&self, w: &[u8]) -> bool {
        testo::pronunciabile(self, w)
    }
    fn pronunciabile_flag(&self, w: &[u8], flag: u8) -> bool {
        // FUN_10002de0: any nonzero flag reads as not pronounceable
        flag == 0 && testo::pronunciabile(self, w)
    }
    fn symbol_name(&self, c: u8, mode: u8) -> Vec<u8> {
        testo::symbol_name(self, c, mode)
    }
    fn normalizza_accento(&self, w: &mut Vec<u8>) {
        testo::normalizza_accento(self, w)
    }
    fn elabora_parola_at(&self, toks: &mut [super::common::Tok], i: usize, idx: usize, spell: u8, mode: u8) -> Option<(Vec<Word>, usize)> {
        testo::elabora_parola_at(self, toks, i, idx, spell, mode)
    }
    fn abbreviazioni(&self) -> &[super::common::Abbreviation] {
        self.abbreviations.get_or_init(|| testo::abbreviazioni(self))
    }
    fn lower(&self, c: u8) -> u8 {
        self.chars[c as usize].0
    }
    fn allofono_in(&self, sentence: &[Word], first: usize, _chunk: &[Word], phones: &mut Vec<Phone>, i: usize) -> usize {
        self.allofono(sentence, first, phones, i)
    }
}

impl super::Module for English {
    fn analyse(&self, text: &str, voice: Option<&crate::voicefon::VoiceFon>) -> Vec<crate::g2p::Token> {
        super::common::analyse(self, text, voice)
    }
}

/// A number's words (interim: plain cardinal reading until the module's
/// number readers, `LesIniNumeri` 0x1000beb0, are ported).
fn english_number_words(digits: &str) -> Vec<Vec<u8>> {
    const ONES: [&str; 20] = [
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve", "thirteen", "fourteen",
        "fifteen", "sixteen", "seventeen", "eighteen", "nineteen",
    ];
    const TENS: [&str; 10] = ["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"];
    fn below_1000(n: u64, out: &mut Vec<String>) {
        if n >= 100 {
            out.push(ONES[(n / 100) as usize].into());
            out.push("hundred".into());
        }
        let r = n % 100;
        if r == 0 && n >= 100 {
            return;
        }
        if r < 20 {
            out.push(ONES[r as usize].into());
        } else {
            out.push(TENS[(r / 10) as usize].into());
            if r % 10 != 0 {
                out.push(ONES[(r % 10) as usize].into());
            }
        }
    }
    let mut out = Vec::new();
    match digits.parse::<u64>() {
        Ok(n) if digits.len() <= 12 => {
            let groups = [(1_000_000_000, "billion"), (1_000_000, "million"), (1000, "thousand")];
            let mut rest = n;
            for (g, name) in groups {
                if rest >= g {
                    below_1000(rest / g, &mut out);
                    out.push(name.into());
                    rest %= g;
                }
            }
            if rest > 0 || out.is_empty() {
                below_1000(rest, &mut out);
            }
        }
        _ => out.extend(digits.chars().map(|d| ONES[d.to_digit(10).unwrap_or(0) as usize].to_string())),
    }
    out.into_iter().map(|s| s.into_bytes()).collect()
}
