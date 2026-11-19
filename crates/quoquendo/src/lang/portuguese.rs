//! Portuguese, ported from the original's language module
//! `LTTS7Portuguese.dll` (MECHANICS.md §4g). The module's exports
//! `PortuguesePt` (0x10009dd0) and `PortugueseBr` (0x10009d60) hand
//! LoqTTS7.dll a name → function table each (0x1001fb98, 0x1001fac0: 26
//! pairs; the `.lde`'s `Library` names the export). They differ in
//! ELQGetPhonemes (phone tables 0x1001f8d8 / 0x1001f8e0), LesGetSymbolName
//! (0x10003850 / 0x10003890), FonGetSamples and LesGetDefaultCurrency; the
//! FON code reads the variant from the engine (`**(ctx+0x44)`, set by
//! 0x10007a30 from the `Language` names: 0 `Portuguese`, 1 `Brazilian`).
//! Its tables are read from the module's own bytes at the addresses its
//! code uses, never copied; its lexical database is the language pack's
//! `Portuguese<Variant>.dbl`.

use super::common::{self, at, field::*, Endings, Pe, Phone, Word};

mod fon;
mod numeri;

/// The function tables, by variant: (table, its ELQGetPhonemes phone
/// table).
const TABLES: [(u32, u32); 2] = [
    (0x1001fb98, 0x1001f8d8), // PortuguesePt
    (0x1001fac0, 0x1001f8e0), // PortugueseBr
];

/// LesGetSymbolName's switches (0x10002df0, 0x10003320): (index bytes,
/// jump table).
const NAMES: [(u32, u32); 2] = [(0x10003220, 0x100030cc), (0x10003750, 0x100035fc)];

/// Character classes (ELQGetGraphemes 0x10009510, 0x1001ed98): 8 bytes per
/// character, lower case, upper case, then flags (0x01 letter, 0x04 digit,
/// 0x08 consonant, 0x10 vowel, 0x20 upper case, 0x40 lower case, 0x80 an
/// acute or circumflex vowel, 0x100 punctuation).
const CHARS: u32 = 0x1001ed98;
const LETTER: u32 = 0x01;
const CONSONANT: u32 = 0x08;
const VOWEL: u32 = 0x10;
const UPPER: u32 = 0x20;
const ACCENTED: u32 = 0x80;
const PUNCT: u32 = 0x100;

/// Stress positions by reversed ending (0x1001b3c0, 0x161 entries),
/// searched by 0x100011c0 (the code Italian's port shares).
const STRESS: Endings = Endings { at: 0x1001b3c0, count: 0x161 };

/// LesAnalizza's guess for words the database lacks (0x1000a470): first
/// the ending tables (`ELQSearchString` mode 8, the ending may be the
/// whole word), then endings shorter than the word (0x1000a410), the first
/// match deciding.
const GUESS_TABLES: [(u32, u32, Guess); 9] = [
    (0x10020f70, 0xae, Guess::Class(0x29)),
    (0x10021228, 0xb, Guess::Class(0xc9)),
    (0x10021288, 0x81, Guess::Class(0x15)),
    (0x10021254, 0xd, Guess::Class(0xca)),
    (0x1002148c, 7, Guess::Class(0xcb)),
    (0x100214a8, 0xe, Guess::Class(0xcc)),
    (0x100214e0, 0x6e, Guess::Class(0x2a)),
    (0x10021698, 0xb, Guess::Class(0xce)),
    (0x100216c4, 5, Guess::Class(0xd0)),
];

#[derive(Clone, Copy)]
enum Guess {
    Class(u8),
    /// class 0x2a, subclass 0x8d (an infinitive)
    Infinitive,
}

/// 0x1000a470's endings in its order (the strings' addresses in the
/// module, from `mov edx, imm32; call 0x1000a410`).
const GUESS_ENDINGS: &[(u32, Guess)] = &{
    use Guess::*;
    const INF: Guess = Infinitive;
    [
        (0x1001662c, INF), // er
        (0x1001659c, INF), // ir
        (0x10021a08, INF), // ôr
        (0x1001c44c, Class(0x33)), // mente
        (0x10021a04, Class(0x29)), // ste
        (0x10021a00, Class(0xc9)), // ido
        (0x10020434, Class(0x15)),
        (0x100219fc, Class(0x15)),
        (0x10020430, Class(0x15)),
        (0x100219f8, Class(0x15)),
        (0x100219f4, Class(0x15)),
        (0x1002042c, Class(0x15)),
        (0x100219f0, Class(0x15)),
        (0x100219ec, Class(0x15)),
        (0x100219e8, Class(0x15)),
        (0x100219e4, Class(0x15)),
        (0x10020424, Class(0x15)),
        (0x100219e0, Class(0x15)),
        (0x100219dc, Class(0x15)),
        (0x1001656c, Class(0x15)),
        (0x10016548, Class(0x15)),
        (0x100219d8, Class(0x15)),
        (0x100219d4, Class(0x15)),
        (0x100219d0, Class(0x15)),
        (0x100164f8, Class(0x15)),
        (0x10020420, Class(0x15)),
        (0x100219cc, Class(0x15)),
        (0x100219c8, Class(0x15)),
        (0x100219c4, Class(0x15)),
        (0x10016280, Class(0x15)),
        (0x10016238, Class(0x15)),
        (0x100219c0, Class(0x15)),
        (0x100219bc, Class(0x15)),
        (0x1001c634, Class(0x15)),
        (0x10020408, Class(0x15)),
        (0x100219b8, Class(0x15)),
        (0x10020404, Class(0x15)),
        (0x100219b4, Class(0x15)),
        (0x10020400, Class(0x15)),
        (0x100219b0, Class(0x15)),
        (0x100219ac, Class(0x15)), // ba … éis
        (0x100219a8, Class(0xca)),
        (0x100219a0, Class(0xca)),
        (0x1002199c, Class(0xca)),
        (0x10021998, Class(0xca)),
        (0x100165a0, Class(0xca)),
        (0x10021990, Class(0xca)),
        (0x1002041c, Class(0xca)),
        (0x1002198c, Class(0xca)),
        (0x10021988, Class(0xca)),
        (0x10021980, Class(0xca)),
        (0x1002197c, Class(0xca)),
        (0x10020410, Class(0xca)),
        (0x1002040c, Class(0xca)),
        (0x10021978, Class(0xca)),
        (0x10021974, Class(0xca)),
        (0x10016120, Class(0xca)), // dor … ã
        (0x1001674c, Class(0xcb)),
        (0x10021970, Class(0xcb)),
        (0x100165d4, Class(0xcb)),
        (0x1002196c, Class(0xcb)),
        (0x10021968, Class(0xcb)),
        (0x10021964, Class(0xcb)),
        (0x1002195c, Class(0xcb)),
        (0x10021958, Class(0xcb)),
        (0x10021954, Class(0xcb)),
        (0x10021950, Class(0xcb)),
        (0x1002194c, Class(0xcb)), // al … ês
        (0x10021948, Class(0xcc)),
        (0x10016638, Class(0xcc)),
        (0x10021940, Class(0xcc)),
        (0x1002193c, Class(0xcc)),
        (0x10021938, Class(0xcc)),
        (0x10021934, Class(0xcc)),
        (0x10016124, Class(0xcc)), // co … z
        (0x1001675c, Class(0xcd)),
        (0x10021930, Class(0xcd)),
        (0x10021928, Class(0xcd)), // ais eia eias
        (0x10021924, Class(0x2a)),
        (0x1002191c, Class(0x2a)),
        (0x10021918, Class(0x2a)),
        (0x10021910, Class(0x2a)),
        (0x10021908, Class(0x2a)),
        (0x10021900, Class(0x2a)),
        (0x100218fc, Class(0x2a)),
        (0x100218f8, Class(0x2a)), // ide … tra
        (0x100218f4, Class(0xce)),
        (0x1001da2c, Class(0xce)), // iro á
        (0x100165b8, Class(0xcf)),
        (0x100218f0, Class(0xcf)),
        (0x100218ec, Class(0xcf)), // i res uo
        (0x100218e8, Class(0xd0)),
        (0x100165b4, Class(0xd0)),
        (0x100218e4, Class(0xd0)),
        (0x100218dc, Class(0xd0)),
        (0x10016440, Class(0xd0)), // ado … or
        (0x10016724, INF),         // ar
        (0x100218d8, Class(0xcc)), // nha
        (0x100218d4, Class(0x15)),
        (0x100218d0, Class(0x15)),
        (0x10020428, Class(0x15)),
        (0x10016584, Class(0x15)),
        (0x10016500, Class(0x15)),
        (0x10020418, Class(0x15)),
        (0x100163c4, Class(0x15)),
        (0x100163bc, Class(0x15)), // da … sa
        (0x100166a4, Class(0xcb)),
        (0x1001665c, Class(0xcb)),
        (0x10016618, Class(0xcb)),
        (0x100218cc, Class(0xcb)),
        (0x100164ec, Class(0xcb)), // as e es is o
        (0x10016570, Class(0xca)),
        (0x100163c0, Class(0xca)),
        (0x10016144, Class(0xca)), // m s u
        (0x10016820, Class(0xcc)), // a
        (0x100163f0, Class(0xcf)), // r
    ]
};

/// 0x1000b700: feminine word endings (mode 8), then feminine plural ones.
const FEMININE: (u32, u32) = (0x100216d8, 0x2c);
const FEMININE_PLURAL: (u32, u32) = (0x10021788, 0x24);

/// LesConfiniProsodici's expressions kept together (0x10008450; the
/// 6-word table 0x1001e1d4 is never reached: at most 5 words are joined)
/// and 0x10008270 (`pippo` placeholders): (words, table, count).
const LOCUTIONS: [(usize, u32, u32); 4] = [(5, 0x1001e1d8, 2), (4, 0x1001e1e0, 10), (3, 0x1001e208, 0xd), (2, 0x1001e240, 0x49)];
const PLACEHOLDERS: [(usize, u32, u32); 3] = [(3, 0x1001e1c8, 1), (2, 0x1001e1cc, 1), (1, 0x1001e1d0, 1)];
/// 0x10008710's one-entry list (`pippo`, 0x1001e1bc): a stand-in.
const PLACEHOLDER: u32 = 0x1001e1bc;

/// One row of the lexical database as LesAnalizza's query returns it
/// (`SELECT grafemica, fonetica, gradoaccentuale, grammaticale, specgramm,
/// gennum FROM lessico`, 0x10009640), NULL columns `None`.
type Row = Vec<Option<Vec<u8>>>;
const FONETICA: usize = 1;
const GRADO: usize = 2;
const GRAMMATICALE: usize = 3;
const SPECGRAMM: usize = 4;
const GENNUM: usize = 5;

/// The language module, read from `LTTS7Portuguese.dll`'s bytes.
pub struct Portuguese {
    pe: Pe,
    /// the FON code's variant (`**(ctx+0x44)`): 0 Portugal, 1 Brazil
    variant: i32,
    chars: Vec<(u8, u8, u32)>,
    /// the variant's phone symbols (ELQGetPhonemes), `{ symbol, code }`
    phones: Vec<(Vec<u8>, u32)>,
    /// the lexical database, sorted as the engine's table (by grafemica,
    /// ASCII letters folded)
    lex: Vec<Row>,
    /// LesGetAbbreviazioni's rows (0x10009b80): (abbreviazione, grafemica)
    /// of `SELECT grafemica, abbreviazione FROM lessico WHERE
    /// ifnull(abbreviazione,'') <> '' ORDER BY abbreviazione COLLATE NOCASE
    /// DESC`
    abbreviations: Vec<(Vec<u8>, Vec<u8>)>,
    abbreviation_table: std::sync::OnceLock<Vec<common::Abbreviation>>,
}

/// `ELQstricmp`'s folding (LTTS7Util.dll 0x100285e0): ASCII capitals only.
fn fold(c: u8) -> u8 {
    c.to_ascii_lowercase()
}

fn stricmp(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    a.iter().map(|&c| fold(c)).cmp(b.iter().map(|&c| fold(c)))
}

/// 0x1000a410: `s` ends with `suffix` and is longer.
fn ends(s: &[u8], suffix: &[u8]) -> bool {
    s.len() > suffix.len() && s.ends_with(suffix)
}

impl Portuguese {
    /// The module's tables, when the voice's language library is
    /// `LTTS7Portuguese.dll` as the pack ships it (7.15.0, the function
    /// tables at 0x1001fac0 and 0x1001fb98), for the voice's
    /// `MotherTongue`, with the language pack's lexical database and
    /// lexicons.
    pub fn from_files(files: &super::ModuleFiles) -> Option<Portuguese> {
        let pe = Pe::parse(files.dll.clone())?;
        let variant = match files.mother_tongue.to_ascii_lowercase().as_str() {
            "portuguesebr" => 1,
            _ => 0,
        };
        let (table, phone_table) = TABLES[variant as usize];
        let fns = pe.function_table(table, 26);
        let name = |i: usize| fns[i].0.as_slice();
        if name(0) != b"GetVersion" || name(5) != b"FonTrasParola" || name(13) != b"FonTrasConfini" || name(8) != b"LesGetSymbolName" {
            return None;
        }
        let chars = (0..256u32)
            .map(|c| (pe.u8(CHARS + 8 * c), pe.u8(CHARS + 8 * c + 1), pe.u32(CHARS + 8 * c + 4)))
            .collect();
        let (entries, count) = (pe.u32(phone_table), pe.u16(phone_table + 4) as u32);
        let phones = (0..count).map(|i| (pe.cstr(pe.u32(entries + 8 * i)).to_vec(), pe.u32(entries + 8 * i + 4))).collect();
        let mut lex: Vec<Row> = Vec::new();
        let mut abbreviations = Vec::new();
        if let Some(rows) = files.dbl.as_deref().and_then(|d| crate::sqlite::read_table(d, "lessico").ok()) {
            // the table's columns: grafemica fonetica grammaticale specgramm
            // gennum abbreviazione gradoaccentuale
            let col = |r: &Vec<crate::sqlite::Value>, k: usize| match r.get(k) {
                Some(crate::sqlite::Value::Text(t)) => Some(t.clone()),
                _ => None,
            };
            for r in &rows {
                lex.push(vec![col(r, 0), col(r, 1), col(r, 6), col(r, 2), col(r, 3), col(r, 4)]);
                if let Some(a) = col(r, 5).filter(|a| !a.is_empty()) {
                    abbreviations.push((a, col(r, 0).unwrap_or_default()));
                }
            }
            lex.sort_by(|a, b| stricmp(a[0].as_deref().unwrap_or(b""), b[0].as_deref().unwrap_or(b"")));
            abbreviations.sort_by(|a, b| stricmp(&b.0, &a.0));
        }
        Some(Portuguese {
            pe,
            variant,
            chars,
            phones,
            lex,
            abbreviations,
            abbreviation_table: Default::default(),
        })
    }

    fn flags(&self, c: u8) -> u32 {
        self.chars[c as usize].2
    }

    fn is_vowel(&self, c: u8) -> bool {
        self.flags(c) & VOWEL != 0
    }

    /// 0x1000bd40: the vowel without its accent, case kept; `à` and `ò`
    /// (the module's stressed nasal a and o) become `ã` and `õ`.
    fn plain(&self, c: u8) -> u8 {
        if c == 0 {
            return 0;
        }
        let upper = self.flags(c) & UPPER != 0;
        let mut a = if upper { self.chars[c as usize].0 } else { c };
        a = match a {
            0xe0 => 0xe3,
            0xe1 | 0xe2 => b'a',
            0xe9 | 0xea => b'e',
            0xed | 0xee => b'i',
            0xf2 => 0xf5,
            0xf3 | 0xf4 => b'o',
            0xfa | 0xfb => b'u',
            0xfd => b'y',
            _ => a,
        };
        if upper {
            self.chars[a as usize].1
        } else {
            a
        }
    }

    /// 0x10008210: accented (0x80) characters without their accent.
    fn unaccented(&self, s: &[u8]) -> Vec<u8> {
        s.iter().map(|&c| if self.flags(c) & ACCENTED != 0 { self.plain(c) } else { c }).collect()
    }

    fn s(&self, va: u32) -> &[u8] {
        self.pe.cstr(va)
    }

    /// LTTS7Util.dll `ELQSearchString(s, table, mode, count)` (0x10028ca0):
    /// the index of the entry found, or `None`. More than 3 entries and no
    /// mode bit 0x1c: a binary search (exact with bit 1, else the entry
    /// starting `s`); else linear: bit 1 exact, bit 2 the entry starts
    /// `s`, bit 8 `s` ends with the entry.
    fn search(&self, s: &[u8], table: u32, mode: u32, count: u32) -> Option<usize> {
        let entry = |i: u32| self.s(self.pe.u32(table + 4 * i));
        if count > 3 && mode & 0x1c == 0 {
            let (mut lo, mut hi) = (0i32, count as i32 - 1);
            while lo <= hi {
                let mid = (lo + hi) / 2;
                let e = entry(mid as u32);
                let ord = if mode & 1 != 0 {
                    s.cmp(e)
                } else {
                    let n = e.len().min(s.len());
                    match s[..n].cmp(&e[..n]) {
                        std::cmp::Ordering::Equal if s.len() < e.len() => std::cmp::Ordering::Less,
                        o => o,
                    }
                };
                match ord {
                    std::cmp::Ordering::Less => hi = mid - 1,
                    std::cmp::Ordering::Greater => lo = mid + 1,
                    std::cmp::Ordering::Equal => return Some(mid as usize),
                }
            }
            return None;
        }
        (0..count)
            .find(|&i| {
                let e = entry(i);
                if mode & 1 != 0 {
                    s == e
                } else if mode & 2 != 0 {
                    s.starts_with(e)
                } else if mode & 8 != 0 {
                    s.ends_with(e)
                } else {
                    true
                }
            })
            .map(|i| i as usize)
    }

    /// The database rows whose grafemica is `key`, ASCII case folded
    /// (`ELQDBTableFind`, LTTS7Util.dll 0x10005df0).
    fn find(&self, key: &[u8]) -> Vec<Row> {
        let lo = self.lex.partition_point(|r| stricmp(r[0].as_deref().unwrap_or(b""), key) == std::cmp::Ordering::Less);
        self.lex[lo..]
            .iter()
            .take_while(|r| stricmp(r[0].as_deref().unwrap_or(b""), key) == std::cmp::Ordering::Equal)
            .cloned()
            .collect()
    }
}

/// A column is set and not empty (0x1000b490 compares with "").
fn filled(r: &Row, col: usize) -> bool {
    r.get(col).and_then(|c| c.as_deref()).is_some_and(|c| !c.is_empty())
}

/// 0x1000b490 then `ELQListGetCurrent`: the list's cursor after scanning it
/// from the head for a row with `col` filled (the last row when none is).
fn scan(rows: &[Row], col: usize) -> (bool, Option<&Row>) {
    match rows.iter().position(|r| filled(r, col)) {
        Some(k) => (true, Some(&rows[k])),
        None => (false, rows.last()),
    }
}

fn col(r: &Row, k: usize) -> Option<&[u8]> {
    r.get(k).and_then(|c| c.as_deref())
}

// --- LesAnalizza (0x1000bd10): word classes ---------------------------------

/// 0x1000ade0: `grammaticale` → class.
fn class_of(g: Option<&[u8]>) -> u8 {
    match g {
        Some(b"PRON") => 0x20,
        Some(b"VERBO") => 0x2a,
        Some(b"NONVERBO") => 0x2f,
        Some(b"AVV") => 0x33,
        Some(b"AGG") => 0x29,
        Some(b"SOST") => 0x15,
        Some(b"ARTIC") => 0x18,
        Some(b"ARTPRON") => 0x16,
        Some(b"CONG") => 0x1f,
        Some(b"PREPS") => 0x19,
        Some(b"PREPA") => 0x1e,
        Some(b"PRONCLI") => 0x21,
        Some(b"PRONPC") => 0x28,
        Some(b"PRONR") => 0x22,
        Some(b"VERBOE") => 0x2c,
        Some(b"VERBOA") => 0x2b,
        Some(b"OMOGRAFO") => 0x2d,
        Some(b"NEGAZ") => 0x2e,
        _ => 0xc8,
    }
}

/// 0x1000b1b0: `specgramm` → subclass.
fn sub_of(s: Option<&[u8]>) -> u8 {
    match s {
        Some(b"ECONG") => 0x82,
        Some(b"NUMU") => 0x79,
        Some(b"INTERR") => 0x78,
        Some(b"ENCLIT") => 0x8f,
        _ => 0xc8,
    }
}

/// 0x1000b2a0: `gennum` → gender and number.
fn gennum(s: Option<&[u8]>) -> u8 {
    match s {
        None => 9,
        Some(b"FESI") => 0xd,
        Some(b"MASI") => 0xb,
        Some(b"FEPL") => 0xe,
        Some(b"MAPL") => 0xc,
        Some(b"XPLU") => 0x10,
        Some(b"XSIN") => 0xf,
        Some(_) => 9,
    }
}

/// The classes LesAnalizza lets a dictionary row give a capitalised word
/// (0x1000b5fc) and LesGradiAccentuali takes as function words.
const FUNCTION: &[u8] = &[0x19, 0x1e, 0x18, 0x16, 0x20, 0x21, 0x28, 0x22, 0x2e, 0x1f];

impl Portuguese {
    fn analizza(&self, w: &mut [Word]) {
        self.classes(w);
        self.context(w);
        self.feminine_numbers(w);
    }

    /// 0x1000b520: each word's rows of the lexical database (added to its
    /// list, 0x1000b3f0); the class, subclass and gender of the row the
    /// list's cursor stops at (the first with `grammaticale`) when the
    /// word is unknown (class 0x17), a capitalised word (0x15/0xc8) the
    /// database calls a function word, or `e`/`de` read as a number word
    /// (subclass `z`); an unknown word without a classed row by its ending
    /// (0x1000a470).
    fn classes(&self, w: &mut [Word]) {
        let mut has_class = false;
        for i in 0..w.len() {
            if w[i].f[CLASS] == b'2' {
                continue;
            }
            let rows = self.find(&w[i].text);
            w[i].rows.extend(rows);
            let n = w[i].rows.len();
            if n > 0 {
                has_class = w[i].rows.iter().any(|r| filled(r, GRAMMATICALE));
            }
            let r = &mut w[i];
            if (n == 0 || !has_class) && r.f[CLASS] == 0x17 {
                self.guess(r);
                continue;
            }
            let Some(row) = scan(&r.rows, GRAMMATICALE).1.cloned() else { continue };
            let c = class_of(col(&row, GRAMMATICALE));
            let take = r.f[CLASS] == 0x17
                || (r.f[CLASS] == 0x15 && r.f[SUB] == 0xc8 && FUNCTION.contains(&c))
                || (r.f[SUB] == b'z' && (r.text == b"e" || r.text == b"de"));
            if take {
                r.f[CLASS] = c;
                r.f[SUB] = sub_of(col(&row, SPECGRAMM));
                r.f[INFO] = gennum(col(&row, GENNUM));
            }
        }
    }

    /// 0x1000a470: an unknown word's class by its ending.
    fn guess(&self, r: &mut Word) {
        let t = &r.text;
        let mut hit = GUESS_TABLES.iter().find(|&&(table, count, _)| self.search(t, table, 8, count).is_some()).map(|&(_, _, g)| g);
        if hit.is_none() {
            hit = GUESS_ENDINGS.iter().find(|&&(s, _)| ends(t, self.s(s))).map(|&(_, g)| g);
        }
        match hit {
            Some(Guess::Class(c)) => r.f[CLASS] = c,
            Some(Guess::Infinitive) => {
                r.f[CLASS] = 0x2a;
                r.f[SUB] = 0x8d;
            }
            None => {
                if r.f[CLASS] == 0x17 {
                    r.f[CLASS] = 0x2f;
                }
            }
        }
    }

    /// 0x1000b700: `para` a verb (before punctuation or a placeholder,
    /// after a clitic, before `para`/`de`/`e`/`por`, in `não para nem`);
    /// nouns and adjectives (not number words but peseta(s), esterlina(s))
    /// feminine by their endings (feminine plural 0xe, singular 0xd).
    fn context(&self, w: &mut [Word]) {
        let n = w.len();
        let text = |w: &[Word], k: isize| -> Option<Vec<u8>> {
            if k < 0 {
                None
            } else {
                w.get(k as usize).map(|r| r.text.clone())
            }
        };
        let class = |w: &[Word], k: isize| if k < 0 { 0 } else { w.get(k as usize).map_or(0, |r| r.f[CLASS]) };
        for i in 0..n {
            let ii = i as isize;
            let para = w[i].text == b"para";
            let next = text(w, ii + 1);
            if para && matches!(class(w, ii + 1), b'2' | b'7') {
                w[i].f[CLASS] = 0x2a;
            }
            if i > 1 && class(w, ii - 2) != b'7' {
                let prev = text(w, ii - 1).unwrap_or_default();
                if matches!(prev.as_slice(), b"a" | b"as" | b"lhe" | b"lhes" | b"me" | b"nos" | b"o" | b"os" | b"se" | b"te" | b"vos") && para {
                    w[i].f[CLASS] = 0x2a;
                }
            }
            if para && matches!(next.as_deref(), Some(b"para" | b"de" | b"e" | b"por")) {
                w[i].f[CLASS] = 0x2a;
            }
            if i > 0 && text(w, ii - 1).as_deref() == Some(b"n\xe3o") && para && next.as_deref() == Some(b"nem") {
                w[i].f[CLASS] = 0x2a;
            }
            let r = &mut w[i];
            if !matches!(r.f[CLASS], b')' | 0x15 | b'-' | 0xc9 | 0xca | 0xcb | 0xcc | 0xcd | b'/') {
                continue;
            }
            let t = &r.text;
            if matches!(r.f[SUB], b'y' | b'z' | b'{') && !matches!(t.as_slice(), b"peseta" | b"pesetas" | b"esterlina" | b"esterlinas") {
                continue;
            }
            if self.search(t, FEMININE.0, 8, FEMININE.1).is_some() {
                r.f[INFO] = 0xd;
            } else if self.search(t, FEMININE_PLURAL.0, 8, FEMININE_PLURAL.1).is_some() {
                r.f[INFO] = 0xe;
            } else if ends(t, b"ca") && !ends(t, b"arca") {
                r.f[INFO] = 0xd;
            }
        }
    }

    /// 0x10009fd0: a number word (subclass y/z) before a feminine word
    /// takes the feminine: meio meia, um uma, dois duas, duzentos…
    /// novecentos -as, any other word ending in `um` gets an `a`. Stops at
    /// an empty word; the last record is never changed.
    fn feminine_numbers(&self, w: &mut [Word]) {
        for i in 0..w.len().saturating_sub(1) {
            if w[i].text.is_empty() {
                return;
            }
            if !matches!(w[i].f[SUB], b'y' | b'z') || !matches!(w[i + 1].f[INFO], 0xe | 0xd) {
                continue;
            }
            let t = &mut w[i].text;
            let to: Option<&[u8]> = match t.as_slice() {
                b"meio" => Some(b"meia"),
                b"um" => Some(b"uma"),
                b"dois" => Some(b"duas"),
                b"duzentos" => Some(b"duzentas"),
                b"trezentos" => Some(b"trezentas"),
                b"quatrocentos" => Some(b"quatrocentas"),
                b"quinhentos" => Some(b"quinhentas"),
                b"seiscentos" => Some(b"seiscentas"),
                b"setecentos" => Some(b"setecentas"),
                b"oitocentos" => Some(b"oitocentas"),
                b"novecentos" => Some(b"novecentas"),
                _ => None,
            };
            match to {
                Some(f) => *t = f.to_vec(),
                None => {
                    if (3..0x3f).contains(&t.len()) && t.ends_with(b"um") {
                        t.push(b'a');
                    }
                }
            }
        }
    }
}

// --- LesAccenta (0x100015a0): the stressed vowel marked ----------------------

/// 0x10001280: an open (acute) stressed vowel; `ã`/`õ` → `à`/`ò`.
fn open(c: u8) -> u8 {
    match c {
        0xc4 | 0x41 => 0xc1,
        0xcb | 0x45 => 0xc9,
        0xcf | 0x49 => 0xcd,
        0xd6 | 0x4f => 0xd3,
        0xdc | 0x55 => 0xda,
        0xe3 => 0xe0,
        0xe4 | 0x61 => 0xe2,
        0xeb | 0x65 => 0xe9,
        0xef | 0x69 | 0x79 => 0xed,
        0xf5 => 0xf2,
        0xf6 | 0x6f => 0xf3,
        0xfc | 0x75 => 0xfa,
        c => c,
    }
}

/// 0x100013b0: a closed (circumflex) stressed vowel.
fn closed(c: u8) -> u8 {
    match c {
        0xc4 | 0x41 => 0xc1,
        0xcb | 0xeb | 0x45 | 0x65 => 0xea,
        0xcf | 0x49 => 0xcd,
        0xd6 | 0xf6 | 0x4f | 0x6f => 0xf4,
        0xdc | 0x55 => 0xda,
        0xe3 => 0xe0,
        0xe4 | 0x61 => 0xe2,
        0xef | 0x69 | 0x79 => 0xed,
        0xf5 => 0xf2,
        0xfc | 0x75 => 0xfa,
        c => c,
    }
}

impl Portuguese {
    /// 0x100014d0: code n marks the n-th vowel from the end (the first
    /// vowel when there are fewer), +30 open.
    fn place(&self, code: i32, t: &mut [u8]) {
        let (is_open, mut n) = if code >= 0x1e { (true, code - 0x1e) } else { (false, code) };
        if n <= 0 {
            return;
        }
        let mut at_k = 0usize;
        let mut k = t.len() as isize - 1;
        while k >= 0 && n > 0 {
            if self.is_vowel(t[k as usize]) {
                n -= 1;
                at_k = k as usize;
            }
            k -= 1;
        }
        if at_k < t.len() {
            t[at_k] = if is_open { open(t[at_k]) } else { closed(t[at_k]) };
        }
    }

    /// LesAccenta: a word without an accent (record +0xc 0xffff) gets one on
    /// its stressed vowel from the ending table; `-mente` adverbs on their
    /// stem's, plus `mênte`. A word over 63 letters ends the pass.
    fn accenta(&self, w: &mut [Word]) {
        for r in w.iter_mut() {
            let accented = r.text.iter().any(|&c| self.flags(c) & ACCENTED != 0);
            if r.f[0xc] == 0xff && r.f[0xd] == 0xff && !accented {
                let vowels = r.text.iter().filter(|&&c| self.is_vowel(c)).count();
                if vowels < 1 {
                    continue;
                }
                if r.text.len() > 0x3f {
                    break;
                }
                let rev: Vec<u8> = r.text.iter().rev().copied().collect();
                if rev.len() > 5 && rev.starts_with(b"etnem") {
                    let code = common::lookup(&self.pe, STRESS, &rev[5..]) as i32;
                    self.place(code + 2, &mut r.text);
                    self.place(2, &mut r.text);
                } else {
                    let code = common::lookup(&self.pe, STRESS, &rev) as i32;
                    self.place(code, &mut r.text);
                }
            } else {
                let n = r.text.len();
                if n > 5 && r.text.ends_with(b"mente") {
                    r.text[n - 4] = 0xea;
                }
            }
        }
    }
}

// --- LesGradiAccentuali (0x10007ca0) -----------------------------------------

/// 0x10007c20: a row's `gradoaccentuale` as a grade (`DEA` unstressed).
fn row_grade(r: &Row) -> u8 {
    match col(r, GRADO) {
        Some(b"DEA") => 3,
        _ => 0,
    }
}

impl Portuguese {
    /// LesGradiAccentuali: punctuation grade 0x0f; a function word at
    /// boundary 0 1 2 3 6 7 or 0x16 unstressed (3) when at most 3 letters or
    /// quêm pâra quâl mêus súas; `é` after a comma-like boundary 0x0b
    /// unstressed; elided words and clitics (0x8f) unstressed at those
    /// boundaries or after one; else stressed (0). The word's rows of the
    /// other grade are dropped.
    fn gradi(&self, w: &mut [Word]) {
        let short_bound = |b: u8| matches!(b, 2 | 1 | 0 | 6 | 7 | 3 | 0x16);
        let mut k = 0usize;
        while k < w.len() && k <= 199 && w[k].f[BOUND] != 0x17 {
            let prev_bound = if k > 0 { w[k - 1].f[BOUND] } else { 0 };
            let r = &mut w[k];
            if r.f[GRADE] == 0x0e {
                let c = r.f[CLASS];
                if c == b'2' || c == b'7' {
                    r.f[GRADE] = 0x0f;
                } else {
                    let apostrophe = r.text.last() == Some(&b'\'');
                    let b = r.f[BOUND];
                    let function = FUNCTION.contains(&c);
                    let little = short_bound(b)
                        && function
                        && (r.text.len() <= 3 || matches!(r.text.as_slice(), b"qu\xeam" | b"p\xe2ra" | b"qu\xe2l" | b"m\xeaus" | b"s\xfaas"));
                    let g = if little
                        || (function && r.text == b"\xe9" && b == 0x0b && k != 0)
                        || ((apostrophe || r.f[SUB] == 0x8f) && (short_bound(b) || (k != 0 && short_bound(prev_bound))))
                    {
                        3
                    } else {
                        0
                    };
                    r.f[GRADE] = g;
                    r.rows.retain(|row| row_grade(row) == g);
                }
            }
            k += 1;
        }
        if k < w.len() {
            w[k].f[GRADE] = 0x10;
            w[k].f[BOUND] = 0x17;
        }
    }
}

// --- LesConfiniProsodici (0x10008f90): prosodic boundaries ------------------

/// Verb classes, the guessed ones included (VERBO, VERBOA, VERBOE, 0xc9
/// 0xcb–0xd0).
fn verb(c: u8) -> bool {
    matches!(c, b'*' | b'+' | b',' | 0xce | 0xcb | 0xcc | 0xcd | 0xc9 | 0xcf | 0xd0)
}

/// Noun-like classes 0x100087d0 keeps in a noun group.
fn nominal(c: u8) -> bool {
    matches!(c, 0x15 | b')' | 0xca | 0xcb | 0xcc | 0xcd | 0xcf | 0xd0)
}

impl Portuguese {
    /// 0x10008450 / 0x10008270: the longest expression of `tables` the
    /// words from `start` begin with (their texts joined by spaces,
    /// unaccented): its length in words.
    fn expression(&self, w: &[Word], start: usize, tables: &[(usize, u32, u32)]) -> Option<usize> {
        let most = tables[0].0;
        let words: Vec<&[u8]> = w.iter().skip(start).take(most).map(|r| r.text.as_slice()).collect();
        let mut s = self.unaccented(&words.join(&b' '));
        let mut n = words.len();
        if n < tables.last().map_or(1, |t| t.0) {
            return None;
        }
        for &(len, table, count) in tables {
            if n > len {
                continue;
            }
            if self.search(&s, table, 1, count).is_some() {
                return Some(len);
            }
            match s.iter().rposition(|&c| c == b' ') {
                Some(k) => s.truncate(k),
                None => s.clear(),
            }
            n -= 1;
        }
        None
    }

    /// 0x100080e0: the words before `a` (back to a pause) and after `b`
    /// (to punctuation) are not both more than 1 word and 14 / 19 letters.
    fn short_sides(w: &[Word], a: usize, b: usize) -> bool {
        let mut before_chars = w[a].text.len();
        let mut before_words = 1;
        for k in (0..a).rev() {
            let bound = w[k].f[BOUND];
            if bound == 0x16 {
                continue;
            }
            if bound == 0x0a || bound == 0x0b {
                break;
            }
            before_chars += w[k].text.len();
            before_words += 1;
        }
        let (mut after_words, mut after_chars) = (0, 0);
        for r in w.iter().skip(b + 1) {
            if r.f[CLASS] == b'2' {
                break;
            }
            if r.f[CLASS] != b'7' {
                after_words += 1;
                after_chars += r.text.len();
            }
        }
        if a < b {
            for r in &w[a + 1..=b] {
                if r.f[CLASS] != b'7' {
                    after_words += 1;
                    after_chars += r.text.len();
                }
            }
        }
        !(before_words > 1 && before_chars > 0xe && after_words > 1 && after_chars > 0x13)
    }

    /// 0x10008d40: a pause may follow the word at `i`.
    fn may_pause(&self, w: &[Word], i: usize) -> bool {
        let c = w[i].f[CLASS];
        let next = w.get(i + 1);
        let nc = next.map_or(0, |r| r.f[CLASS]);
        if (c == 0x18 && nc == 0x15) || (c == b')' && nc == 0x15) || (c == 0x15 && nc == b')') {
            return false;
        }
        let Some(next) = next else { return true };
        if verb(c) && (matches!(nc, b'3' | b')' | 0x18 | 0x15 | 0x19) || (verb(nc) && next.f[SUB] == 0x15)) {
            return false;
        }
        if c == b'3' && verb(nc) {
            return false;
        }
        if matches!(c, b'+' | b',') && nc == b'*' {
            return false;
        }
        if c == b'!' && verb(nc) {
            return false;
        }
        if w[i].text == b"que" || next.text == b"que" {
            return false;
        }
        // 0x10008710: the word (unaccented) in a one-entry list (`pippo`)
        !(self.unaccented(&w[i].text) == self.s(PLACEHOLDER) && verb(nc))
    }

    /// 0x100087d0: each word's group kind (0 noun group end, 1 verb group
    /// end, 2 …, 5 default, 6 conjunction, 7 inside, 8 punctuation, 10
    /// the end) and whether a pause is wanted after it.
    fn groups(&self, w: &[Word]) -> Vec<(u32, u32)> {
        let n = w.len();
        let mut arr = vec![(5u32, 0u32); n + 1];
        let class = |k: isize| if k < 0 { 0 } else { w.get(k as usize).map_or(0, |r| r.f[CLASS]) };
        let sub = |k: isize| if k < 0 { 0 } else { w.get(k as usize).map_or(0, |r| r.f[SUB]) };
        let fill = |arr: &mut Vec<(u32, u32)>, from: usize, to: usize| {
            for e in arr.iter_mut().take(to).skip(from) {
                *e = (7, 0);
            }
        };
        let (mut state, mut start) = (0u32, 0usize);
        let (mut mw_len, mut mw_pos) = (0usize, 0usize);
        for u in 0..n {
            let ui = u as isize;
            arr[u] = (5, 0);
            if mw_len == 0 {
                if let Some(l) = self.expression(w, u, &LOCUTIONS) {
                    mw_len = l;
                    mw_pos = 1;
                }
            } else if mw_pos < mw_len {
                mw_pos += 1;
            } else if mw_pos == mw_len {
                mw_len = 0;
            }
            let c = class(ui);
            let same_sides = sub(ui) == 0x82 && class(ui - 1) == class(ui + 1) && sub(ui - 1) == sub(ui + 1);
            // `true`: classify the word anew (0x10008c4a)
            let anew = match state {
                1 => {
                    if matches!(c, b'7' | b' ' | b'"' | b'$' | b'(' | b'!' | b'.' | 0x1f | b'3') {
                        state = 1;
                        false
                    } else if verb(c) {
                        state = 2;
                        false
                    } else {
                        state = 0;
                        true
                    }
                }
                2 => {
                    if c == b'7' || verb(c) || c == b'3' || same_sides {
                        state = 2;
                        false
                    } else if mw_len != 0 {
                        false
                    } else {
                        fill(&mut arr, start, u - 1);
                        arr[u - 1].0 = 1;
                        state = 0;
                        true
                    }
                }
                3 => {
                    if matches!(c, b'7' | 0x18 | 0x16 | 0x1f) {
                        state = 3;
                        false
                    } else if nominal(c) {
                        state = 5;
                        false
                    } else {
                        state = 0;
                        true
                    }
                }
                4 | 6 => {
                    if state == 6 && matches!(c, b' ' | b'"' | b'$' | b'(' | b'!' | b'.' | 0x1f | b'3') {
                        state = 1;
                        false
                    } else if state == 6 && verb(c) && u < 2 {
                        state = 2;
                        false
                    } else if matches!(c, 0x18 | 0x16) || (state == 4 && c == 0x1f) {
                        state = 4;
                        false
                    } else if c == b'7' || nominal(c) {
                        state = 7;
                        false
                    } else {
                        state = 0;
                        true
                    }
                }
                5 => {
                    if c == b'7' || nominal(c) || same_sides {
                        state = 5;
                        false
                    } else if mw_len != 0 {
                        false
                    } else {
                        fill(&mut arr, start, u - 1);
                        arr[u - 1].0 = 0;
                        state = 0;
                        true
                    }
                }
                7 => {
                    if nominal(c) || c == b'7' {
                        state = 7;
                        false
                    } else if mw_len != 0 {
                        false
                    } else {
                        fill(&mut arr, start, u - 1);
                        arr[u - 1].0 = 2;
                        if start == 0
                            && (w[0].f[CLASS] == b'3'
                                || w[0].f[CLASS] == 0x19
                                || self.expression(w, 0, &LOCUTIONS).is_some_and(|l| l < u)
                                || self.expression(w, 0, &PLACEHOLDERS).is_some_and(|l| l < u))
                        {
                            arr[u - 1].1 = 1;
                        }
                        state = 0;
                        true
                    }
                }
                8 => {
                    arr[u - 1].0 = 6;
                    state = 0;
                    true
                }
                _ => state == 0,
            };
            if anew {
                start = u;
                if c == 0x1f {
                    state = 8;
                } else if matches!(c, b' ' | b'"' | b'$' | b'(' | b'!' | b'.' | b'3') {
                    state = 1;
                } else if verb(c) {
                    state = 2;
                } else if matches!(c, 0x18 | 0x16) {
                    state = 3;
                } else if matches!(c, b'7' | 0x15 | b')' | 0xca) {
                    state = 5;
                } else if matches!(c, 0x19 | 0x1e) {
                    state = 6;
                } else if c == b'2' {
                    arr[u].0 = 8;
                }
            }
        }
        arr[n].0 = 10;
        arr
    }

    /// LesConfiniProsodici: each word's boundary from its group (`mode` the
    /// sentence type): 5 after a group, 3 inside, 1 after a conjunction or
    /// a little word, 0x16 none, 10 a pause between two long enough groups
    /// where `may_pause` allows, 0x0b at commas, the sentence end by its
    /// type; numbers 6/7/8.
    fn confini(&self, w: &mut [Word], mode: i32) {
        let mut arr = self.groups(w);
        let mut last = 200usize;
        let mut u = 0usize;
        loop {
            if arr[0].0 == 10 || u > 199 {
                if u < w.len() {
                    w[u].f[BOUND] = 0x17;
                }
                return;
            }
            if w[u].f[BOUND] == 0x15 {
                let class = w[u].f[CLASS];
                let mut kind = arr[u].0;
                if kind == 2 {
                    if arr[u].1 == 1 && !Self::short_sides(w, u, u) && self.may_pause(w, u) {
                        w[u].f[BOUND] = 10;
                        last = u;
                        kind = 100;
                    } else {
                        kind = 0;
                    }
                }
                match kind {
                    100 => {}
                    1 => {
                        if class == b'7' {
                            w[u].f[BOUND] = 0x16;
                        } else {
                            if last != 200 && arr[last].1 == 0 && mode != 3 && mode != 2 {
                                let b = w[last].f[BOUND];
                                if matches!(b, 3 | 5 | 7) && arr[last].0 != 5 && !Self::short_sides(w, last, u) && self.may_pause(w, last) {
                                    w[last].f[BOUND] = 10;
                                }
                            }
                            w[u].f[BOUND] = 5;
                            last = u;
                        }
                    }
                    0 => {
                        if class == b'7' {
                            w[u].f[BOUND] = 0x16;
                        } else {
                            w[u].f[BOUND] = 5;
                            last = u;
                        }
                    }
                    4 => {
                        w[u].f[BOUND] = 9;
                        last = u;
                    }
                    6 => {
                        if mode != 3 && mode != 2 && last != 200 {
                            let b = w[last].f[BOUND];
                            let next_class = w.get(u + 1).map_or(0, |r| r.f[CLASS]);
                            if matches!(b, 3 | 5 | 7) && !Self::short_sides(w, last, u) && arr[u + 1].0 != 10 && w[last].f[CLASS] != next_class {
                                w[last].f[BOUND] = 10;
                            }
                        }
                        w[u].f[BOUND] = 1;
                    }
                    7 => {
                        if class == b'7' {
                            w[u].f[BOUND] = 0x16;
                        } else if matches!(class, 0x19 | 0x1e | 0x16 | 0x18 | b' ' | b'!' | b'#' | b'"' | b'$' | b'(' | 0x1f | b'.') {
                            w[u].f[BOUND] = 1;
                        } else {
                            w[u].f[BOUND] = 3;
                        }
                    }
                    8 => self.punctuation(w, u, mode, &mut last),
                    9 => w[u].f[BOUND] = 0x16,
                    _ => {
                        last = u;
                        w[u].f[BOUND] = 3;
                    }
                }
            }
            if matches!(w[u].f[BOUND], 3 | 5) {
                match w[u].f[SUB] {
                    0x79 | 0x84 => w[u].f[BOUND] = 7,
                    0x7a | 0x83 => w[u].f[BOUND] = 6,
                    0x7b => w[u].f[BOUND] = 8,
                    _ => {}
                }
            }
            u += 1;
            arr[0].0 = arr[u].0;
        }
    }

    /// LesConfiniProsodici on a punctuation record (group kind 8): the
    /// sentence end by type (both it and the word before, past 0x16 ones),
    /// `,` `-` `.` (inside) 0x0b, brackets 0x0b between words, else 0x16.
    fn punctuation(&self, w: &mut [Word], u: usize, mode: i32, last: &mut usize) {
        let mut j = if u == 0 { 0 } else { u - 1 };
        while j != 0 && w[j].f[BOUND] == 0x16 {
            j -= 1;
        }
        if w[j].f[CLASS] == b'2' {
            j = u;
        }
        let both = |w: &mut [Word], b: u8| {
            w[j].f[BOUND] = b;
            w[u].f[BOUND] = b;
        };
        if w[u].f[SUB] == b'|' {
            match mode {
                0 => both(w, 0x0f),
                2 => both(w, 0x11),
                3 => both(w, 0x12),
                5 => both(w, 0x10),
                6 => both(w, 0x13),
                _ => w[u].f[BOUND] = 0x16,
            }
        } else {
            let t = &w[u].text;
            let mut k = 0usize;
            while at(t, k + 1) != 0 && !common::among(b",)(-.", at(t, k)) {
                k += 1;
            }
            let p = match at(t, k) {
                b'.' => b',',
                p => p,
            };
            match p {
                b'(' | b')' => {
                    let side = |k: usize| w.get(k).map_or(0, |r| r.f[CLASS]);
                    if u == 0 || side(u - 1) == b'2' || side(u + 1) == b'2' {
                        w[u].f[BOUND] = 0x16;
                    } else {
                        both(w, 0x0b);
                    }
                }
                b',' | b'-' => both(w, 0x0b),
                _ => w[u].f[BOUND] = 0x16,
            }
        }
        if w[u].f[BOUND] != 0x16 {
            *last = u;
        }
    }
}

// --- What the engine's tokenizer asks of the module -------------------------

/// 0x10002a20 / 0x10002b40: the grave / acute accent an apostrophe puts on
/// the vowel before it.
fn grave(c: u8) -> u8 {
    match c {
        0x41 => 0xc0,
        0x45 => 0xc8,
        0x49 | 0x4a => 0xcc,
        0x4f => 0xd2,
        0x59 => 0xdd,
        0x61 => 0xe0,
        0x65 => 0xe8,
        0x69 | 0x6a => 0xec,
        0x6f => 0xf2,
        0x79 => 0xfd,
        0xdc | 0x55 => 0xd9,
        0xef => 0xed,
        0xfc | 0x75 => 0xf9,
        c => c,
    }
}

fn acute(c: u8) -> u8 {
    match c {
        0x41 => 0xc1,
        0x45 => 0xc9,
        0x49 | 0x4a => 0xcd,
        0x4f => 0xd3,
        0x59 => 0xdd,
        0x61 => 0xe1,
        0x65 => 0xe9,
        0x6f => 0xf3,
        0x79 => 0xfd,
        0xdc | 0x55 => 0xda,
        0xef | 0x69 | 0x6a => 0xed,
        0xfc | 0x75 => 0xfa,
        c => c,
    }
}

/// The vowels 0x10002c60 tests (0x1001d878).
const AEIOUY: u32 = 0x1001d878;

impl Portuguese {
    /// LesNormalizzaAccento (0x10002c60): an apostrophe or backquote (` '
    /// ’ ´) after a vowel puts its accent on it (grave for the backquote);
    /// elsewhere it stays an apostrophe before a vowel or at the end, else
    /// goes. Then grave vowels lose theirs (`à` becomes `å`, the crasis
    /// FonTrasParola reads).
    fn normalizza_accento(&self, w: &mut Vec<u8>) {
        let lower = |c: u8| self.chars[c as usize].0;
        let vowel = |c: u8| common::among(self.s(AEIOUY), lower(c)) || lower(c) == 0;
        let mut out: Vec<u8> = Vec::with_capacity(w.len());
        for k in 0..w.len() {
            let c = w[k];
            if matches!(c, b'`' | b'\'' | 0x92 | 0xb4) {
                if k != 0 && vowel(w[k - 1]) {
                    let v = if c == b'`' { grave(w[k - 1]) } else { acute(w[k - 1]) };
                    if let Some(p) = out.last_mut() {
                        *p = v;
                    }
                } else {
                    let n = at(w, k + 1);
                    if n == 0 || vowel(n) {
                        out.push(b'\'');
                    }
                }
            } else {
                out.push(c);
            }
        }
        for c in out.iter_mut() {
            *c = match *c {
                0xc0 | 0xe0 => 0xe5,
                0xc8 | 0xe8 => b'e',
                0xcc | 0xec => b'i',
                0xd2 | 0xf2 => b'o',
                0xd9 | 0xf9 => b'u',
                c => c,
            };
        }
        *w = out;
    }

    /// LesPronunciabile (0x10003af0 → 0x10003900): a word with an
    /// apostrophe, an accent or a vowel other than a e i o u y is
    /// pronounceable; a three-letter word with a capital inside not when
    /// its last two letters are consonants other than r/l/n/s + b c d f g p
    /// s t v, or its first two consonants other than b c d f g p s t v +
    /// r/l or s + b c d g p t; else when it has a vowel.
    fn pronunciabile(&self, w: &[u8]) -> bool {
        if w.iter().any(|&c| c == b'`' || c == b'\'') {
            return true;
        }
        let mut low = Vec::new();
        let mut inner_upper = false;
        for (k, &c) in w.iter().take(0x3f).enumerate() {
            let f = self.flags(c);
            if f & ACCENTED != 0 || (f & VOWEL != 0 && !b"aeiouyAEIOUY".contains(&c)) {
                return true;
            }
            if k > 0 && f & UPPER != 0 {
                inner_upper = true;
            }
            low.push(self.chars[c as usize].0);
        }
        let cons = |c: u8| self.flags(c) & CONSONANT != 0;
        let isin = |set: u32, c: u8| common::among(self.s(set), c);
        if inner_upper && low.len() == 3 {
            if cons(low[1]) && cons(low[2]) && (!isin(0x1001dbd0, low[1]) || !isin(0x1001dbc4, low[2])) {
                return false;
            }
            if cons(low[0]) && cons(low[1]) {
                let ok = (isin(0x1001dbc4, low[0]) && isin(0x1001dbc0, low[1])) || (isin(0x100163c0, low[0]) && isin(0x1001dbb8, low[1]));
                if !ok {
                    return false;
                }
            }
        }
        low.iter().any(|&c| self.is_vowel(c))
    }

    /// One case of a compiled `switch` returning a string: `mov eax, imm32;
    /// ret`, or `test al, al; je default; mov eax, imm32; ret` (only when
    /// `mode` is set; else the default "").
    fn case_string(&self, target: u32, mode: u8) -> Option<Vec<u8>> {
        let b = self.pe.at(target);
        match b {
            [0xb8, ..] => Some(self.s(self.pe.u32(target + 1)).to_vec()),
            [0x84, 0xc0, 0x74, _, 0xb8, ..] => Some(if mode != 0 { self.s(self.pe.u32(target + 5)).to_vec() } else { Vec::new() }),
            _ => None,
        }
    }

    /// LesGetSymbolName (0x10003850 → 0x10002df0 Portugal, 0x10003890 →
    /// 0x10003320 Brazil): a letter's or symbol's name, the punctuation's
    /// only when `mode` is set. The switch takes `c + 0x80` (signed) up to
    /// 0xfe through its index bytes and jump table (`NAMES`).
    fn symbol_name(&self, c: u8, mode: u8) -> Vec<u8> {
        let (index, jumps) = NAMES[self.variant as usize];
        let k = c as i8 as i32 + 0x80;
        if !(0..=0xfe).contains(&k) {
            return Vec::new();
        }
        let idx = self.pe.u8(index + k as u32) as u32;
        self.case_string(self.pe.u32(jumps + 4 * idx), mode).unwrap_or_default()
    }

    /// The variant's symbol for a phone code (ELQGetPhonemes).
    pub fn symbol(&self, code: u32) -> Option<&str> {
        self.phones.iter().find(|(_, c)| *c == code).and_then(|(s, _)| std::str::from_utf8(s).ok())
    }
}

// --- The engine's side (LoqTTS7.dll, `common`) calls these ------------------

impl common::Les for Portuguese {
    fn letter(&self, c: u8) -> (bool, bool, u8) {
        let f = self.flags(c);
        (f & LETTER != 0, f & UPPER != 0, self.chars[c as usize].0)
    }
    fn leggi_numero(&self, toks: &mut [common::Tok], i: usize, kind: u32, dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
        Portuguese::leggi_numero(self, toks, i, kind, dmy)
    }
    fn analizza(&self, w: &mut [Word]) {
        Portuguese::analizza(self, w)
    }
    fn accenta(&self, w: &mut [Word]) {
        Portuguese::accenta(self, w)
    }
    fn confini(&self, w: &mut [Word], mode: i32) {
        Portuguese::confini(self, w, mode)
    }
    fn gradi(&self, w: &mut [Word]) {
        Portuguese::gradi(self, w)
    }
    fn tras_parola(&self, w: &[Word], i: usize) -> Vec<u32> {
        Portuguese::tras_parola(self, &w[i], i)
    }
    fn tras_confini_in(&self, sentence: &[Word], first: usize, i: usize, _prev_bound: u8, phones: &mut Vec<Phone>, next: &mut Vec<u32>) {
        Portuguese::tras_confini(self, sentence, first, i, phones, next)
    }
    fn symbol(&self, code: u32) -> Option<&str> {
        Portuguese::symbol(self, code)
    }
    fn normalizza_accento(&self, w: &mut Vec<u8>) {
        Portuguese::normalizza_accento(self, w)
    }
    fn pronunciabile(&self, w: &[u8]) -> bool {
        Portuguese::pronunciabile(self, w)
    }
    fn symbol_name(&self, c: u8, mode: u8) -> Vec<u8> {
        Portuguese::symbol_name(self, c, mode)
    }
    fn abbreviazioni(&self) -> &[common::Abbreviation] {
        self.abbreviation_table.get_or_init(|| common::abbreviation_table(self, &self.abbreviations))
    }
}

impl super::Module for Portuguese {
    fn analyse(&self, text: &str, voice: Option<&crate::voicefon::VoiceFon>) -> Vec<crate::g2p::Token> {
        common::analyse(self, text, voice)
    }
}
