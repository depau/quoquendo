//! Spanish, ported from the original's language module `LTTS7Spanish.dll`
//! (MECHANICS.md §4c). The module's one export, `Spanish` (0x1000d060),
//! hands LoqTTS7.dll one of six name → function tables, picked by the
//! language descriptor's `Language` names (castilian, superspanish,
//! americanspanish, mexican, chilean, argentine); the FON functions are the
//! same in all of them and read the variant from the engine (ctx+0x44).
//! Its tables (character classes, stress endings, word lists, phone
//! symbols) are read from the module's own bytes at the addresses its code
//! uses, never copied; its lexical database is the language pack's
//! `Spanish<Variant>.dbl`.

use super::common::{self, at, field::*, Endings, Pe, Phone, Word};

/// The export's function tables, by the variant they serve: (table,
/// variant number the FON code tests, its ELQGetPhonemes phone table).
const TABLES: [(u32, i32, u32); 5] = [
    (0x100298b8, 0, 0x100292a0), // castilian
    (0x10029988, 1, 0x100292b0), // mexican
    (0x10029b28, 2, 0x10029298), // argentine
    (0x10029a58, 3, 0x100292a8), // chilean
    (0x100297e8, 4, 0x10029290), // americanspanish
];

/// Character classes (ELQGetGraphemes, 0x10028198): 8 bytes per character,
/// lower case, upper case, then flags (0x01 letter, 0x10 vowel, 0x20 upper
/// case, 0x80 accented vowel).
const CHARS: u32 = 0x10028198;
const LETTER: u32 = 0x01;
const VOWEL: u32 = 0x10;
const UPPER: u32 = 0x20;
const ACCENTED: u32 = 0x80;

/// Stress positions by reversed ending (0x100221b0, 0x28c entries), searched
/// by 0x100011c0 (the code Italian's port shares, `common::ending`).
const STRESS: Endings = Endings { at: 0x100221b0, count: 0x28c };

/// Letter sets FonTrasParola tests with `strchr`.
const CONSONANTS: u32 = 0x1002a264; // bcçdfghjklmnñpqrstvwxz
const VOWELS: u32 = 0x1002a27c; // aeiouáéíóúü
const VOICED: u32 = 0x1002a238; // bdglmnñrvw
const VOICELESS: u32 = 0x1002a22c; // cçfkjpqstxz
const NASALS: u32 = 0x1002a1fc; // mnñ
const ACUTE: u32 = 0x1002a290; // áéíóú

/// Word lists LesAnalizza's ending guess (0x1000b4d0) matches word endings
/// against (`ELQSearchString` mode 8): (table, count).
const INFINITIVE_CLITICS: (u32, u32) = (0x100277e8, 0x1d);
const GERUND_ENDINGS: (u32, u32) = (0x10027860, 0x20);
const VERB_ENDINGS: (u32, u32) = (0x10027688, 0x58);
const NOUN_ENDINGS: (u32, u32) = (0x100278e0, 0x2a);
const ADJECTIVE_ENDINGS: (u32, u32) = (0x10027988, 0x3d);

/// Phone codes (engine feature words) the module writes.
mod numeri;

mod ph {
    pub const A: u32 = 0x441;
    pub const A_: u32 = 0x10441;
    pub const E: u32 = 0x741;
    pub const E_: u32 = 0x10741;
    pub const I: u32 = 0x7c1;
    pub const I_: u32 = 0x107c1;
    pub const O: u32 = 0x361;
    pub const O_: u32 = 0x10361;
    pub const U: u32 = 0x3e1;
    pub const U_: u32 = 0x103e1;
    pub const W: u32 = 0x401ec2;
    pub const J: u32 = 0x10c2;
    pub const BH: u32 = 0x1ec2;
    pub const GH: u32 = 0xec2;
    pub const DH: u32 = 0x1ac2;
    pub const P: u32 = 0x1ff2;
    pub const T: u32 = 0x1bf2;
    pub const K: u32 = 0xff2;
    pub const B: u32 = 0x1fe2;
    pub const D: u32 = 0x1be2;
    pub const G: u32 = 0xfe2;
    pub const JJ: u32 = 0x11e2; // J
    pub const TS: u32 = 0x1912;
    pub const TSH: u32 = 0x1712; // T$
    pub const DG: u32 = 0x1702;
    pub const F: u32 = 0x1d52;
    pub const S: u32 = 0x1952;
    pub const Z: u32 = 0x1942;
    pub const SH: u32 = 0x1752; // $
    pub const ZG: u32 = 0x1742;
    pub const TH: u32 = 0x1b52;
    pub const X: u32 = 0xf52;
    pub const Y: u32 = 0x1142;
    pub const M: u32 = 0x1fc2;
    pub const N: u32 = 0x19c2;
    pub const MF: u32 = 0x1dc2; // M
    pub const NG: u32 = 0xfc2; // N
    pub const NY: u32 = 0x11c2; // Gn
    pub const NYI: u32 = 0x2019c2; // GN
    pub const L: u32 = 0x18a2;
    pub const LL: u32 = 0x10a2; // Gl
    pub const LLI: u32 = 0x2018a2; // GL
    pub const R: u32 = 0x1b82; // hr
    pub const RR: u32 = 0x19a6; // r:
    pub const RT: u32 = 0x19a2; // r
    pub const XC: u32 = 0x1152; // Chilean x before e/i
    pub const S2: u32 = 0x952;
    pub const Z2: u32 = 0x942;
    pub const SS: u32 = 0x18c2;
}

/// One row of the lexical database as LesAnalizza's query returns it
/// (`SELECT grafemica, fonetica, gradoaccentuale, grammaticale, specgramm,
/// gennum, tempomodo FROM lessico`, 0x1000c990), NULL columns `None`.
type Row = Vec<Option<Vec<u8>>>;
const FONETICA: usize = 1;
const GRADO: usize = 2;
const GRAMMATICALE: usize = 3;
const SPECGRAMM: usize = 4;
const GENNUM: usize = 5;
const TEMPOMODO: usize = 6;

/// The language module, read from `LTTS7Spanish.dll`'s bytes.
pub struct Spanish {
    pe: Pe,
    /// the variant number the FON code tests (`**(ctx+0x44)`): 0 castilian,
    /// 1 mexican, 2 argentine, 3 chilean, 4 american
    variant: i32,
    chars: Vec<(u8, u8, u32)>,
    /// the variant's phone symbols (ELQGetPhonemes), `{ symbol, code }`
    phones: Vec<(Vec<u8>, u32)>,
    /// the lexical database, sorted as the engine's table (by grafemica,
    /// ASCII letters folded)
    lex: Vec<Row>,
    sets: Sets,
    /// LesGetAbbreviazioni's rows (0x1000ce90): (abbreviazione, grafemica)
    /// of `SELECT grafemica, abbreviazione FROM lessico WHERE
    /// ifnull(abbreviazione,'') <> '' ORDER BY abbreviazione COLLATE NOCASE
    /// DESC`
    abbreviations: Vec<(Vec<u8>, Vec<u8>)>,
    abbreviation_table: std::sync::OnceLock<Vec<common::Abbreviation>>,
    /// the `.lde`'s lexicons (`Spanish.lex`) as the engine's text filters
    filters: Vec<common::TextFilter>,
}

struct Sets {
    consonants: Vec<u8>,
    vowels: Vec<u8>,
    voiced: Vec<u8>,
    voiceless: Vec<u8>,
    nasals: Vec<u8>,
    acute: Vec<u8>,
}

/// `ELQstricmp`'s folding (LTTS7Util.dll 0x100285e0): ASCII capitals only.
fn fold(c: u8) -> u8 {
    if c.is_ascii_uppercase() {
        c + 0x20
    } else {
        c
    }
}

fn stricmp(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    a.iter().map(|&c| fold(c)).cmp(b.iter().map(|&c| fold(c)))
}

/// `a` ends with `suffix` and is longer (0x1000b470).
fn ends(a: &[u8], suffix: &[u8]) -> bool {
    a.len() > suffix.len() && a.ends_with(suffix)
}

/// The byte before the ending `suffix` of `a` (when `ends(a, suffix)`).
fn before(a: &[u8], suffix: &[u8]) -> u8 {
    a[a.len() - suffix.len() - 1]
}

impl Spanish {
    /// The module's tables, when the voice's language library is
    /// `LTTS7Spanish.dll` as the pack ships it (the function tables at
    /// 0x100297e8…0x10029b28), for the voice's `MotherTongue`, with the
    /// language pack's lexical database and lexicons.
    pub fn from_files(files: &super::ModuleFiles) -> Option<Spanish> {
        let pe = Pe::parse(files.dll.clone())?;
        let (table, variant, phone_table) = match files.mother_tongue.to_ascii_lowercase().as_str() {
            "spanishes" => TABLES[0],
            "spanishmx" => TABLES[1],
            "spanishar" => TABLES[2],
            "spanishcl" => TABLES[3],
            _ => TABLES[4],
        };
        let names = pe.function_table(table, 26);
        let name = |i: usize| names[i].0.as_slice();
        if name(0) != b"GetVersion" || name(3) != b"FonTrasParola" || name(12) != b"FonAllofonoLingua" {
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
            // gennum tempomodo abbreviazione gradoaccentuale
            let col = |r: &Vec<crate::sqlite::Value>, k: usize| match r.get(k) {
                Some(crate::sqlite::Value::Text(t)) => Some(t.clone()),
                _ => None,
            };
            for r in &rows {
                lex.push(vec![col(r, 0), col(r, 1), col(r, 7), col(r, 2), col(r, 3), col(r, 4), col(r, 5)]);
                if let Some(a) = col(r, 6).filter(|a| !a.is_empty()) {
                    abbreviations.push((a, col(r, 0).unwrap_or_default()));
                }
            }
            lex.sort_by(|a, b| stricmp(a[0].as_deref().unwrap_or(b""), b[0].as_deref().unwrap_or(b"")));
            abbreviations.sort_by(|a, b| stricmp(&b.0, &a.0));
        }
        let sets = Sets {
            consonants: pe.cstr(CONSONANTS).to_vec(),
            vowels: pe.cstr(VOWELS).to_vec(),
            voiced: pe.cstr(VOICED).to_vec(),
            voiceless: pe.cstr(VOICELESS).to_vec(),
            nasals: pe.cstr(NASALS).to_vec(),
            acute: pe.cstr(ACUTE).to_vec(),
        };
        let mut es = Spanish {
            pe,
            variant,
            chars,
            phones,
            lex,
            sets,
            abbreviations,
            abbreviation_table: Default::default(),
            filters: Vec::new(),
        };
        es.filters = files.lexicons.iter().filter_map(|f| common::TextFilter::parse(&es, f)).collect();
        Some(es)
    }

    fn flags(&self, c: u8) -> u32 {
        self.chars[c as usize].2
    }

    /// The vowel without its accent, case kept (0x1000d240).
    fn plain(&self, c: u8) -> u8 {
        if c == 0 {
            return 0;
        }
        let upper = self.flags(c) & UPPER != 0;
        let mut a = if upper { self.chars[c as usize].0 } else { c };
        a = match a {
            0xe0 | 0xe1 => b'a',
            0xe8 | 0xe9 => b'e',
            0xec | 0xed => b'i',
            0xf2 | 0xf3 => b'o',
            0xf9 | 0xfa => b'u',
            0xfd => b'y',
            _ => a,
        };
        if upper {
            self.chars[a as usize].1
        } else {
            a
        }
    }

    /// `ELQSearchString(s, table, 8, count) >= 0`: `s` ends with one of the
    /// table's strings (LTTS7Util.dll 0x10028ca0, linear in mode 8).
    fn ends_with_any(&self, s: &[u8], (table, count): (u32, u32)) -> bool {
        (0..count).any(|i| s.ends_with(self.pe.cstr(self.pe.u32(table + 4 * i))))
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

/// A column is set and not empty (0x1000c1c0 compares with "").
fn filled(r: &Row, col: usize) -> bool {
    r.get(col).and_then(|c| c.as_deref()).is_some_and(|c| !c.is_empty())
}

/// 0x1000c1c0 then `ELQListGetCurrent`: the list's cursor after scanning it
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

// --- LesAnalizza (0x1000c7d0): word classes ---------------------------------

/// 0x1000bb00: `grammaticale` → word class.
fn class_of(g: Option<&[u8]>) -> u8 {
    let Some(g) = g else { return 0xc8 };
    match g {
        b"PRON" => b' ',
        b"VERBO" => b'*',
        b"NONVERBO" => b'/',
        b"AVV" => b'3',
        b"AGG" => b')',
        b"SOST" => 0x15,
        b"ARTIC" => 0x18,
        b"ARTPRON" => 0x16,
        b"CONG" => 0x1f,
        b"PREPS" => 0x19,
        b"PREPA" => 0x1e,
        b"PRONCLI" => b'!',
        b"PRONPC" => b'(',
        b"PRONR" => b'"',
        b"VERBOE" => b',',
        b"VERBOA" => b'+',
        b"OMOGRAFO" => b'-',
        _ => 0xc8,
    }
}

/// 0x1000c0a0: a verb's `tempomodo` → subclass.
fn verb_sub(t: Option<&[u8]>) -> u8 {
    match t {
        Some(b"INFINITO") => 0x8d,
        Some(b"GERUNDIO") => 0x8b,
        _ => 0xc8,
    }
}

/// 0x1000bea0: `specgramm` → subclass.
fn spec_sub(s: Option<&[u8]>) -> u8 {
    match s {
        Some(b"ECONG") => 0x82,
        Some(b"NUMU") => b'y',
        Some(b"INTERR") => b'x',
        _ => 0xc8,
    }
}

/// 0x1000bf50: `gennum` → gender and number.
fn gennum(s: Option<&[u8]>) -> u8 {
    match s {
        Some(b"FESI") => 0x0d,
        Some(b"MASI") => 0x0b,
        Some(b"FEPL") => 0x0e,
        Some(b"MAPL") => 0x0c,
        Some(b"XPLU") => 0x10,
        Some(b"XSIN") => 0x0f,
        _ => 0x09,
    }
}

/// Function-word classes (preposition, article, pronoun, conjunction…) the
/// module tests together.
const FUNCTION: &[u8] = &[0x19, 0x1e, 0x18, 0x16, 0x21, 0x28, 0x22, 0x1f];

impl Spanish {
    /// LesAnalizza: the dictionary's classes (0x1000c250), gender of nouns
    /// by their endings (0x1000c4a0), numbers agreeing with what follows
    /// (0x1000ac20).
    fn analizza(&self, w: &mut [Word]) {
        self.classes(w);
        Self::genders(w);
        Self::agreement(w);
    }

    /// 0x1000c250.
    fn classes(&self, w: &mut [Word]) {
        for i in 0..w.len() {
            if w[i].f[CLASS] == b'2' {
                continue;
            }
            let rows = self.find(&w[i].text);
            w[i].rows = rows;
            let n = w[i].rows.len();
            let (has_grammar, cur) = scan(&w[i].rows, GRAMMATICALE);
            if (n == 0 || !has_grammar) && w[i].f[CLASS] == 0x17 {
                self.guess(&mut w[i]);
                continue;
            }
            let Some(row) = cur.cloned() else { continue };
            let cls = class_of(col(&row, GRAMMATICALE));
            let r = &mut w[i];
            let take = r.f[CLASS] == 0x17
                || (r.f[CLASS] == 0x15 && r.f[SUB] == 0xc8 && FUNCTION.contains(&cls))
                || (r.f[SUB] == b'z' && matches!(r.text.as_slice(), b"e" | b"un" | b"y" | b"de"));
            if !take {
                continue;
            }
            r.f[CLASS] = cls;
            r.f[SUB] = if cls == b'*' { verb_sub(col(&row, TEMPOMODO)) } else { spec_sub(col(&row, SPECGRAMM)) };
            r.f[INFO] = gennum(col(&row, GENNUM));
        }
    }

    /// 0x1000b4d0: a word the dictionary doesn't classify, by its ending.
    fn guess(&self, r: &mut Word) {
        let s = r.text.clone();
        let unaccented = |s: &[u8]| ![0xe1u8, 0xe9, 0xed, 0xf3, 0xfa, 0xfd].iter().any(|c| s.contains(c));
        let vowel_before = |s: &[u8], suf: &[u8], extra: &[u8]| {
            let c = before(s, suf);
            matches!(c, b'a' | b'e' | b'i' | b'o' | b'u') || extra.contains(&c)
        };
        let infinitive = (ends(&s, b"ar") && unaccented(&s))
            || (ends(&s, b"er") && unaccented(&s))
            || (ends(&s, b"ir") && unaccented(&s))
            || self.ends_with_any(&s, INFINITIVE_CLITICS)
            || (ends(&s, b"rme") && !ends(&s, b"forme"));
        if infinitive {
            r.f[CLASS] = 0x2a;
            r.f[SUB] = 0x8d;
            return;
        }
        if self.ends_with_any(&s, GERUND_ENDINGS) {
            r.f[CLASS] = 0x2a;
            r.f[SUB] = 0x8b;
            return;
        }
        let verb = self.ends_with_any(&s, VERB_ENDINGS)
            || (ends(&s, b"imos") && before(&s, b"imos") != b's')
            || (ends(&s, b"r\xedais") && vowel_before(&s, b"r\xedais", &[0xfc]))
            || (ends(&s, b"ara") && unaccented(&s))
            || (ends(&s, b"aras") && unaccented(&s));
        if verb {
            r.f[CLASS] = 0x2a;
            return;
        }
        // a word in -an is a verb; so are these
        let verb = ends(&s, b"an")
            || (ends(&s, b"se") && before(&s, b"se") != b'n' && unaccented(&s))
            || (ends(&s, b"le") && vowel_before(&s, b"le", &[0xe1, 0xe9, 0xed, 0xf3, 0xfa, 0xfc]))
            || (ends(&s, b"me") && vowel_before(&s, b"me", &[0xe1, 0xe9, 0xed, 0xf3, 0xfa, 0xfc]))
            || (ends(&s, b"ad") && before(&s, b"ad") != b'd');
        if verb {
            r.f[CLASS] = 0x2a;
            return;
        }
        let noun = self.ends_with_any(&s, NOUN_ENDINGS)
            || (ends(&s, b"eda") && unaccented(&s))
            || (ends(&s, b"edas") && unaccented(&s))
            || (ends(&s, b"is") && !matches!(before(&s, b"is"), b'a' | b'e' | 0xe1 | 0xe9))
            || ends(&s, b"i\xf3n")
            || ends(&s, b"ion");
        if noun {
            r.f[CLASS] = 0x15;
            return;
        }
        if ends(&s, b"mente") {
            r.f[CLASS] = 0x33;
            return;
        }
        let adjective = self.ends_with_any(&s, ADJECTIVE_ENDINGS)
            || [&b"iento"[..], b"ientos", b"ienta", b"ientas"].iter().any(|e| ends(&s, e) && before(&s, e) != b'm');
        if adjective {
            r.f[CLASS] = 0x29;
            return;
        }
        if r.f[CLASS] == 0x17 {
            r.f[CLASS] = 0x2f;
        }
    }

    /// 0x1000c4a0: after a `PRONPC`, a non-verb or homograph is a verb;
    /// nouns and adjectives get their gender from their ending.
    fn genders(w: &mut [Word]) {
        for i in 0..w.len() {
            if i > 0 && w[i - 1].f[CLASS] == b'(' && matches!(w[i].f[CLASS], b'/' | b'-') {
                w[i].f[CLASS] = 0x2a;
            }
            let r = &mut w[i];
            if !matches!(r.f[CLASS], b')' | 0x15 | b'-' | b'/') {
                continue;
            }
            let s = r.text.as_slice();
            if matches!(r.f[SUB], b'y' | b'z' | b'{')
                && !matches!(s, b"peseta" | b"pesetas" | b"esterlina" | b"esterlinas" | b"libra" | b"libras")
            {
                continue;
            }
            // the order of 0x1000c4a0's tests
            if ends(s, b"z") {
                r.f[INFO] = 0x0d;
                continue;
            }
            let order: [(&[u8], u8); 19] = [
                (b"ces", 0x0e),
                (b"a", 0x0d),
                (b"as", 0x0e),
                (b"ie", 0x0d),
                (b"is", 0x0d),
                (b"dad", 0x0d),
                (b"dades", 0x0e),
                (b"tad", 0x0d),
                (b"tades", 0x0e),
                (b"tud", 0x0d),
                (b"tudes", 0x0e),
                (b"ci\xf3n", 0x0d),
                (b"cion", 0x0d),
                (b"ciones", 0x0e),
                (b"si\xf3n", 0x0d),
                (b"sion", 0x0d),
                (b"siones", 0x0e),
                (b"umbre", 0x0d),
                (b"umbres", 0x0e),
            ];
            if let Some(&(_, g)) = order.iter().find(|(e, _)| ends(s, e)) {
                r.f[INFO] = g;
            }
        }
    }

    /// 0x1000ac20: a number word agrees with the noun after it (`uno` →
    /// `una`/`un`, `-cientos` → `-cientas`).
    fn agreement(w: &mut [Word]) {
        if w.len() < 2 {
            return;
        }
        for k in 0..w.len() - 1 {
            let n = w[k].text.len();
            if n == 0 {
                return;
            }
            if !matches!(w[k].f[SUB], b'y' | b'z') {
                continue;
            }
            let (next_info, next_sub, next_class) = (w[k + 1].f[INFO], w[k + 1].f[SUB], w[k + 1].f[CLASS]);
            let t = &mut w[k].text;
            if matches!(next_info, 0x0e | 0x0d) {
                let fem: Option<&[u8]> = match t.as_slice() {
                    b"uno" => Some(b"una"),
                    b"doscientos" => Some(b"doscientas"),
                    b"trescientos" => Some(b"trescientas"),
                    b"cuatrocientos" => Some(b"cuatrocientas"),
                    b"quinientos" => Some(b"quinientas"),
                    b"seiscientos" => Some(b"seiscientas"),
                    b"setecientos" => Some(b"setecientas"),
                    b"ochocientos" => Some(b"ochocientas"),
                    b"novecientos" => Some(b"novecientas"),
                    _ => None,
                };
                if let Some(f) = fem {
                    *t = f.to_vec();
                } else if n > 2 && t.ends_with(b"uno") {
                    t[n - 1] = b'a';
                }
                continue;
            }
            if matches!(next_sub, b'y' | b'z' | b'{') {
                let unit = matches!(
                    w[k + 1].text.as_slice(),
                    b"minuto"
                        | b"minutos"
                        | b"segundo"
                        | b"segundos"
                        | b"d\xf3lar"
                        | b"d\xf3lares"
                        | b"centavo"
                        | b"centavos"
                        | b"euro"
                        | b"euros"
                        | b"c\xe9ntimo"
                        | b"c\xe9ntimos"
                        | b"yen"
                        | b"yenes"
                        | b"sen"
                        | b"peso"
                        | b"pesos"
                        | b"real"
                        | b"reales"
                );
                if !unit {
                    continue;
                }
            }
            let t = &mut w[k].text;
            if matches!(next_class, 0x15 | b'/' | b')') {
                if t.as_slice() == b"uno" {
                    *t = b"un".to_vec();
                } else if n > 2 && t.ends_with(b"uno") {
                    t.truncate(n - 1);
                }
            }
        }
    }
}

// --- LesAccenta (0x10001600): where the stress falls -------------------------

/// 0x100013a0: the vowel with the acute accent (the stress mark).
fn acute(c: u8) -> u8 {
    match c {
        b'A' => 0xc1,
        b'E' => 0xc9,
        b'I' | b'J' => 0xcd,
        b'O' => 0xd3,
        b'Y' => 0xdd,
        b'a' => 0xe1,
        b'e' => 0xe9,
        b'o' => 0xf3,
        b'y' => 0xfd,
        0xdc | b'U' => 0xda,
        0xef | b'i' | b'j' => 0xed,
        0xfc | b'u' => 0xfa,
        _ => c,
    }
}

/// 0x10001280: the vowel with the grave accent.
fn grave(c: u8) -> u8 {
    match c {
        b'A' => 0xc0,
        b'E' => 0xc8,
        b'I' | b'J' => 0xcc,
        b'O' => 0xd2,
        b'Y' => 0xdd,
        b'a' => 0xe0,
        b'e' => 0xe8,
        b'i' | b'j' => 0xec,
        b'o' => 0xf2,
        b'y' => 0xfd,
        0xdc | b'U' => 0xd9,
        0xef => 0xed,
        0xfc | b'u' => 0xf9,
        _ => c,
    }
}

impl Spanish {
    fn vowels(&self, t: &[u8]) -> usize {
        t.iter().filter(|&&c| self.flags(c) & VOWEL != 0).count()
    }

    /// 0x100014c0: accent the `code`-th vowel from the end (30 and up: the
    /// acute accent, counting from `code` − 30); when there are fewer
    /// vowels, the first one, or the first letter when there is none.
    fn place(&self, code: i32, t: &mut [u8]) {
        let is_acute = code >= 30;
        let mut n = if is_acute { code - 30 } else { code };
        if n <= 0 {
            return;
        }
        let mut k = t.len() as i32 - 1;
        let mut at = 0usize;
        while k >= 0 && n > 0 {
            if self.flags(t[k as usize]) & VOWEL != 0 {
                n -= 1;
                at = k as usize;
            }
            k -= 1;
        }
        if let Some(c) = t.get_mut(at) {
            *c = if is_acute { acute(*c) } else { grave(*c) };
        }
    }

    /// LesAccenta: every word not elided and without a written accent gets
    /// one on its stressed vowel (one vowel: that one; else the stress
    /// ending table 0x100221b0; `-mente` adverbs on their stem and on
    /// `-ménte`); graves become acutes (0x10001550).
    fn accenta(&self, w: &mut [Word]) {
        for r in w.iter_mut() {
            let elided = !(r.f[0xc] == 0xff && r.f[0xd] == 0xff);
            let written = r.text.iter().any(|&c| self.flags(c) & ACCENTED != 0);
            if !elided && !written {
                let n = self.vowels(&r.text);
                let code = if n > 1 {
                    if r.text.len() > 0x3f {
                        return;
                    }
                    let rev: Vec<u8> = r.text.iter().rev().copied().collect();
                    if rev.starts_with(b"etnem") && rev.len() > 5 {
                        let stem = common::lookup(&self.pe, STRESS, &rev[5..]) as i32;
                        self.place(stem + 2 + 30, &mut r.text);
                        2
                    } else {
                        common::lookup(&self.pe, STRESS, &rev) as i32
                    }
                } else {
                    n as i32
                };
                self.place(code + 30, &mut r.text);
            } else {
                let n = r.text.len();
                if n > 5 && r.text.ends_with(b"mente") {
                    r.text[n - 4] = 0xe9;
                }
            }
            for c in r.text.iter_mut() {
                *c = match *c {
                    0xe0 => 0xe1,
                    0xe8 => 0xe9,
                    0xec => 0xed,
                    0xf2 => 0xf3,
                    0xf9 => 0xfa,
                    c => c,
                };
            }
        }
    }
}

// --- LesGradiAccentuali (0x100081e0): stress grades -------------------------

/// 0x10008160: a row's `gradoaccentuale` as a grade (`DEA` unstressed).
fn row_grade(r: &Row) -> u8 {
    match col(r, GRADO) {
        Some(b"DEA") => 3,
        _ => 0,
    }
}

impl Spanish {
    /// LesGradiAccentuali: punctuation grade 0x0f; a word not yet graded is
    /// unstressed (3) when it is a short function word (≤ 5 letters, its
    /// boundary one of 0 1 2 3 5 6 7 0x16), ends in an apostrophe or is a
    /// 0x8f form, else stressed (0); its dictionary rows of the other grade
    /// are dropped (the list walk stops after dropping its first row).
    fn gradi(&self, w: &mut [Word]) {
        let mut k = 0usize;
        while k < w.len() && k <= 199 && w[k].f[BOUND] != 0x17 {
            let r = &mut w[k];
            let class = r.f[CLASS];
            if class == b'2' || class == b'7' {
                r.f[GRADE] = 0x0f;
            } else if r.f[GRADE] == 0x0e {
                let apostrophe = r.text.last() == Some(&b'\'');
                let short_function = matches!(r.f[BOUND], 2 | 1 | 0 | 6 | 7 | 3 | 0x16 | 5) && FUNCTION.contains(&class) && r.text.len() <= 5;
                let g = if short_function || apostrophe || r.f[SUB] == 0x8f { 3 } else { 0 };
                r.f[GRADE] = g;
                let mut j = 0usize;
                while j < r.rows.len() {
                    if row_grade(&r.rows[j]) == g {
                        j += 1;
                    } else {
                        r.rows.remove(j);
                        if j == 0 {
                            break;
                        }
                    }
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

// --- FonTrasParola (0x1000d510): letters → phones ----------------------------

impl Spanish {
    /// 0x1000d380 with 0x1000d2f0: a dictionary pronunciation (symbols
    /// separated by spaces or `-`, at most 5 characters each) as the
    /// variant's phone codes; an unknown symbol is 0x10.
    fn pronunciation(&self, s: &[u8]) -> Vec<u32> {
        let mut out = Vec::new();
        let mut k = 0usize;
        while at(s, k) != 0 {
            let mut sym = Vec::new();
            while !matches!(at(s, k), b' ' | b'-' | 0) && sym.len() < 5 {
                sym.push(s[k]);
                k += 1;
            }
            if at(s, k) != 0 {
                k += 1;
            }
            let code = if sym.is_empty() {
                0x10
            } else {
                match self.phones.iter().position(|(p, _)| *p == sym) {
                    Some(idx) => match self.phones[idx].1 {
                        0 if idx > 3 => 0x10,
                        c => c,
                    },
                    None => 0x10,
                }
            };
            out.push(code);
        }
        out
    }

    /// 0x1000d450: the spelling FonTrasParola reads: an `h` between a/e/o
    /// and a vowel dropped (not in Mexico); an unstressed word (grade 3)
    /// loses its first accent.
    fn prep(&self, t: &[u8], grade: u8) -> Vec<u8> {
        let mut out = Vec::with_capacity(t.len());
        for k in 0..t.len() {
            let drop = self.variant != 1
                && k != 0
                && t[k] == b'h'
                && matches!(t[k - 1], b'a' | b'e' | b'o')
                && matches!(at(t, k + 1), b'a' | b'e' | b'o' | b'i' | b'u');
            if !drop {
                out.push(t[k]);
            }
        }
        if grade == 3 {
            if let Some(k) = out.iter().position(|&c| self.flags(c) & ACCENTED != 0) {
                out[k] = self.plain(out[k]);
            }
        }
        out
    }

    /// FonTrasParola: the word's dictionary pronunciation when one of its
    /// remaining rows has one, else its letters read one by one in their
    /// context.
    fn tras_parola(&self, r: &Word) -> Vec<u32> {
        if let (true, Some(row)) = scan(&r.rows, FONETICA) {
            return self.pronunciation(col(row, FONETICA).unwrap_or(b""));
        }
        let buf = self.prep(&r.text, r.f[GRADE]);
        // `local_50`: the word's length minus one (its last index)
        let len0 = r.text.len() as isize - 1;
        let mut out = Vec::new();
        let mut i = 0usize;
        while at(&buf, i) != 0 {
            self.letter(&buf, i as isize, len0, &mut out);
            i += 1;
        }
        out
    }

    /// One letter of FonTrasParola's switch (0x1000d68a).
    fn letter(&self, buf: &[u8], i: isize, len0: isize, out: &mut Vec<u32>) {
        use ph::*;
        let v = self.variant;
        let s = &self.sets;
        // the working word; before its start the stack holds a pointer
        // whose top byte is no letter
        let b = |k: isize| -> u8 { if k < 0 { 0 } else { at(buf, k as usize) } };
        let isin = |set: &[u8], c: u8| c != 0 && set.contains(&c);
        let sw = |k: isize, lit: &[u8]| lit.iter().enumerate().all(|(j, &c)| b(k + j as isize) == c);
        let (prev, next) = (b(i - 1), b(i + 1));
        let cons = |c: u8| isin(&s.consonants, c);
        let vow = |c: u8| isin(&s.vowels, c);
        let voiced = |c: u8| isin(&s.voiced, c);
        let voiceless = |c: u8| isin(&s.voiceless, c);
        let ei = |c: u8| matches!(c, b'i' | b'e' | 0xed | 0xe9);
        let mut push = |c: u32| out.push(c);
        match b(i) {
            b'b' => push(B),
            b'c' => {
                if i != 0 && prev == b'n' && cons(next) && !matches!(next, b'h' | b'l' | b'r') {
                } else if (v == 0 || v == 5) && ei(next) {
                    push(TH)
                } else if ei(next) {
                    push(S)
                } else if next == b'h' {
                    push(TSH)
                } else {
                    if i != 0 {
                        if (v == 0 || v == 2) && vow(prev) && next == b'n' {
                            return push(GH);
                        }
                        if vow(prev) && next == b'n' {
                            return push(K);
                        }
                        if vow(prev) && voiced(next) && next != b'l' && next != b'r' {
                            return push(GH);
                        }
                    }
                    if (i != 0 || next != b'n') && next != b'k' {
                        push(K)
                    }
                }
            }
            b'd' => {
                if i == 0 || matches!(prev, b'm' | b'n' | 0xf1 | b'l') {
                    push(D)
                } else if vow(prev) && voiceless(next) {
                    push(T)
                } else {
                    push(DH)
                }
            }
            b'f' => push(F),
            b'g' => {
                let hard = |c: u8| matches!(c, b'a' | b'o' | b'u' | 0xe1 | 0xf3 | 0xfa | 0xfc | b'r' | b'l');
                if (i == 0 || prev == b'n') && hard(next) {
                    return push(G);
                }
                if (v == 3 || v == 5) && ei(next) {
                    return push(XC);
                }
                if ei(next) {
                    return push(X);
                }
                let skip = if i == 0 { next == b'n' } else { prev == b'n' && i == len0 };
                if !skip {
                    push(if voiceless(next) { K } else { GH })
                }
            }
            b'h' => {
                if v == 1 {
                    let ua = |k: isize| sw(k, b"ua") || sw(k, b"u\xe1") || sw(k, b"ue") || sw(k, b"u\xe9");
                    if i == 0 {
                        if ua(1) {
                            push(G)
                        }
                    } else if !cons(prev) && ua(i + 1) {
                        push(GH)
                    }
                }
            }
            b'j' => {
                if !(i == 1 && b(0) == b'n') {
                    if (v == 3 || v == 5) && ei(next) {
                        push(XC)
                    } else {
                        push(X)
                    }
                }
            }
            b'k' | b'q' => push(K),
            b'l' => {
                if next == b'l' {
                } else if sw(i + 1, b"ch") || next == 0xf1 || sw(i + 1, b"tx") {
                    push(L)
                } else {
                    push(self.letter_l(i, b(i - 2), prev, next, len0))
                }
            }
            b'm' => push(M),
            b'n' => {
                if v == 4 && next == b'v' {
                    push(MF)
                } else if matches!(next, b'b' | b'p' | b'v') {
                    push(M)
                } else if i == 0 && next == b'j' {
                    push(NY)
                } else if next == b'g' {
                    push(NG)
                } else {
                    push(N)
                }
            }
            b'p' => {
                if i != 0 && prev == b'm' && cons(next) && next != b'l' && next != b'r' {
                    return;
                }
                if !voiced(next) || matches!(next, b'l' | b'n' | b'r') {
                    if i == 0 {
                        if matches!(next, b's' | b't') {
                            return;
                        }
                        if next == b'h' {
                            return push(F);
                        }
                    }
                    push(if next == b'n' { BH } else { P })
                } else {
                    push(BH)
                }
            }
            b'r' => {
                if next == b'r' {
                } else if i == 0 || prev != b'r' {
                    push(R)
                } else {
                    push(RR)
                }
            }
            b's' => self.letter_s(i, prev, next, b(i + 2), len0, &sw, out),
            b't' => {
                if i != 0 && matches!(prev, b'n' | b's') && cons(next) && !matches!(next, b'h' | b'l' | b'r' | b'x' | b'z') {
                    return;
                }
                if next == b'z' {
                    push(TS)
                } else if v == 0 && matches!(next, b'l' | b'n') {
                    push(DH)
                } else if !matches!(next, b'l' | b'n') && voiced(next) && next != b'r' {
                    push(DH)
                } else if !matches!(next, b'l' | b'n') && next == b'x' {
                    push(TSH)
                } else {
                    push(T)
                }
            }
            b'u' => {
                let w_next = |c: u8| matches!(c, b'a' | b'e' | b'i' | b'o' | 0xe1 | 0xe9 | 0xed | 0xf3);
                if i > 1 && sw(i - 2, b"fl") && matches!(next, 0xe9 | 0xed) {
                    return push(U);
                }
                if ((i != 0 && vow(prev)) || (i > 1 && vow(b(i - 2)) && prev == b'h')) && w_next(next) {
                    return push(W);
                }
                if i != 0 && matches!(prev, b'g' | b'c' | b'k' | b'q' | b'j' | b'h') && next == 0xe1 {
                    return push(W);
                }
                if i != 0 && matches!(prev, b'g' | b'q') && matches!(next, b'o' | 0xf3) {
                    return push(W);
                }
                if next == b'a' {
                    return push(W);
                }
                if i != 0 {
                    if vow(prev) && next == b'w' {
                        return push(U);
                    }
                    let pc = cons(prev) && !matches!(prev, b'g' | b'q');
                    if pc && matches!(next, b'e' | b'i' | b'o' | 0xe9 | 0xed | 0xf3) {
                        return push(W);
                    }
                    if pc && matches!(next, b'y' | 0xfd) && (cons(b(i + 2)) || i == len0 - 1) {
                        return push(W);
                    }
                }
                // 0x1000e39a
                if i != 0 && matches!(prev, b'g' | b'q') && matches!(next, b'e' | b'i' | 0xe9 | 0xed) {
                } else if next == b'i' && vow(b(i + 2)) {
                    push(U)
                } else if next == b'i' {
                    push(W)
                } else {
                    push(U)
                }
            }
            b'v' => {
                if i == len0 {
                    push(F)
                } else if i == 0 || !vow(prev) || !voiceless(next) {
                    if v != 4 && (i == 0 || matches!(prev, b'm' | b'n' | 0xf1)) {
                        push(B)
                    } else {
                        push(BH)
                    }
                } else {
                    push(P)
                }
            }
            b'w' => {
                if i != 0 && matches!(prev, b'a' | b'e' | b'o' | 0xe1 | 0xe9 | 0xf3) && !vow(next) {
                    push(U)
                } else if i == 0 || !cons(prev) || !cons(next) {
                    if i == 0 || prev != b'u' {
                        push(W)
                    }
                } else {
                    push(U)
                }
            }
            b'x' => {
                if i == 0 && cons(next) {
                    push(E);
                    push(S);
                    return;
                }
                let faf3 = if (v == 0 || v == 5) && i == 0 {
                    if vow(next) {
                        return push(SH);
                    }
                    true
                } else {
                    i == 0 || prev != b't'
                };
                if faf3 {
                    let soft = next == b's' || sw(i + 1, b"ce") || sw(i + 1, b"ci") || sw(i + 1, b"c\xe9") || sw(i + 1, b"c\xed");
                    if v != 0 && v != 5 && soft {
                        push(K)
                    } else {
                        push(K);
                        push(S);
                    }
                }
            }
            b'y' => push(self.letter_y(i, prev, next)),
            b'z' => {
                if i != 0 && prev == b't' || next == b'z' {
                } else if i != 0 && prev == b'z' {
                    push(TS)
                } else if v == 0 {
                    push(TH)
                } else if v == 5 && i == 0 {
                    push(S)
                } else if v == 5 && prev == b'n' {
                    push(TH)
                } else if i != 0 && prev == b'n' {
                    push(S)
                } else if (v == 2 || v == 3) && voiced(next) {
                    push(Z2)
                } else if (v == 2 || v == 3) && voiceless(next) {
                    push(S2)
                } else {
                    push(S)
                }
            }
            0x9c | 0xeb | b'e' => push(E),
            0xe0..=0xe2 => push(A_),
            0xe3..=0xe6 | b'a' => push(A),
            0xe7 => push(S),
            0xe8..=0xea => push(E_),
            0xec..=0xee | 0xfd => push(I_),
            0xef | b'i' => push(self.letter_i(buf, i, &sw)),
            0xf1 => push(NY),
            0xf2..=0xf4 => push(O_),
            0xf5 | 0xf6 | 0xf8 | b'o' => push(O),
            0xf9..=0xfb => push(U_),
            0xfc => {
                if !cons(next) && i != 0 && i != len0 {
                    push(W)
                } else {
                    push(U)
                }
            }
            _ => {}
        }
    }

    /// `l` not before `l`, `ch`, `ñ`, `tx` (0x1000ee..0x1000f0): `ll` is
    /// Gl in Spain, Zg in Argentina, Y elsewhere between vowels…
    fn letter_l(&self, i: isize, prev2: u8, prev: u8, next: u8, len0: isize) -> u32 {
        use ph::*;
        let v = self.variant;
        let s = &self.sets;
        let cons_next = i == len0 || (next != 0 && s.consonants.contains(&next));
        let prev_l = i != 0 && prev == b'l';
        let pv2 = i > 1 && prev2 != 0 && s.vowels.contains(&prev2);
        let nv = next != 0 && s.vowels.contains(&next);
        #[derive(Clone, Copy)]
        enum Lb {
            Start,
            Eebd,
            Eff1,
            F063,
            F08a,
            F094,
            After,
        }
        let mut lb = Lb::Start;
        loop {
            lb = match lb {
                Lb::Start => {
                    if v == 0 {
                        if i != 0 {
                            if prev_l && cons_next {
                                return LL;
                            }
                            Lb::Eebd
                        } else {
                            Lb::F08a
                        }
                    } else {
                        if v == 2 {
                            if i == 0 {
                                return L;
                            }
                            if prev_l && cons_next {
                                return ZG;
                            }
                        }
                        Lb::Eebd
                    }
                }
                Lb::Eebd => {
                    if prev_l && cons_next {
                        return L;
                    }
                    if v == 0 {
                        if i > 1 {
                            if pv2 && prev_l && nv {
                                return LL;
                            }
                            Lb::Eff1
                        } else {
                            Lb::F063
                        }
                    } else if v != 2 {
                        Lb::Eff1
                    } else if i > 1 {
                        if pv2 && prev_l && nv {
                            return ZG;
                        }
                        Lb::Eff1
                    } else {
                        Lb::F094
                    }
                }
                Lb::Eff1 => {
                    if pv2 && prev_l && nv {
                        return Y;
                    }
                    if v != 0 && v != 5 {
                        Lb::F08a
                    } else {
                        Lb::F063
                    }
                }
                Lb::F063 => {
                    if prev_l {
                        return LL;
                    }
                    Lb::F08a
                }
                Lb::F08a => {
                    if v == 2 {
                        Lb::F094
                    } else {
                        Lb::After
                    }
                }
                Lb::F094 => {
                    if i == 0 {
                        return L;
                    }
                    if prev_l {
                        return ZG;
                    }
                    Lb::After
                }
                Lb::After => return if prev_l { JJ } else { L },
            }
        }
    }

    /// `s` (0x1000f3..0x1000f6): dropped before `s` (and before soft `c`
    /// outside Spain); a prothetic `e` before a word-initial consonant…
    #[allow(clippy::too_many_arguments)]
    fn letter_s(&self, i: isize, prev: u8, next: u8, next2: u8, len0: isize, sw: &dyn Fn(isize, &[u8]) -> bool, out: &mut Vec<u32>) {
        use ph::*;
        let v = self.variant;
        let st = &self.sets;
        let isin = |set: &[u8], c: u8| c != 0 && set.contains(&c);
        if next == b'h' && (i == 0 || isin(&st.consonants, next2) || i == len0 - 1) {
            return out.push(SH);
        }
        if next == b's' {
            return;
        }
        if v != 0 && (sw(i + 1, b"ce") || sw(i + 1, b"ci") || sw(i + 1, b"c\xe9") || sw(i + 1, b"c\xed")) {
            return;
        }
        let ll_y = sw(i + 1, b"ll") || next == b'y';
        let pnc = i != 0 && !isin(&st.consonants, prev);
        #[derive(Clone, Copy)]
        enum Lb {
            Start,
            F479,
            F4bf,
            F524,
            F539,
            F552,
            F584,
            F599,
            Rest,
            E7,
        }
        let mut lb = Lb::Start;
        loop {
            lb = match lb {
                Lb::Start => {
                    if v == 2 {
                        if ll_y {
                            return;
                        }
                        Lb::F479
                    } else if v == 3 || v == 5 {
                        Lb::F479
                    } else {
                        Lb::F4bf
                    }
                }
                Lb::F479 => {
                    if i == 0 && isin(&st.voiceless, next) {
                        out.push(E);
                        out.push(S2);
                        return;
                    }
                    Lb::F4bf
                }
                Lb::F4bf => {
                    if v != 2 {
                        if v == 3 || v == 5 {
                            if i == 0 && ll_y {
                                Lb::F539
                            } else if i == 0 && next != 0 {
                                Lb::F524
                            } else {
                                Lb::F584
                            }
                        } else if i == 0 {
                            Lb::F552
                        } else {
                            Lb::F584
                        }
                    } else if i != 0 {
                        Lb::F599
                    } else if next == 0 {
                        Lb::E7
                    } else {
                        Lb::F524
                    }
                }
                Lb::F584 => {
                    if v == 2 || v == 3 || v == 5 {
                        Lb::F599
                    } else {
                        Lb::Rest
                    }
                }
                Lb::F599 => {
                    if pnc && isin(&st.voiceless, next) {
                        return out.push(S2);
                    }
                    Lb::Rest
                }
                Lb::Rest => {
                    if v == 2 {
                        if pnc && isin(&st.voiced, next) {
                            return out.push(Z2);
                        }
                    } else if (v == 3 || v == 5) && pnc {
                        if ll_y || isin(&st.voiced, next) {
                            return out.push(Z2);
                        }
                    }
                    Lb::E7
                }
                Lb::F524 => {
                    if isin(&st.voiced, next) {
                        Lb::F539
                    } else {
                        Lb::F552
                    }
                }
                Lb::F552 => {
                    if isin(&st.consonants, next) {
                        out.push(E);
                        out.push(S);
                        return;
                    }
                    Lb::F584
                }
                Lb::F539 => {
                    out.push(E);
                    out.push(Z2);
                    return;
                }
                Lb::E7 => return out.push(S),
            }
        }
    }

    /// `y` (0x1000fb..0x1000fe): J before a vowel at the start or after a
    /// nasal, Zg in Argentina, Y between vowels, else the vowel `i`.
    fn letter_y(&self, i: isize, prev: u8, next: u8) -> u32 {
        use ph::*;
        let v = self.variant;
        let st = &self.sets;
        let isin = |set: &[u8], c: u8| c != 0 && set.contains(&c);
        let vn = isin(&st.vowels, next);
        let mnn = isin(&st.nasals, prev);
        if v == 2 && ((i != 0 && mnn) || i == 0) && vn {
            return ZG;
        }
        if !((i != 0 && !mnn) || !vn) {
            return JJ;
        }
        let sr = i != 0 && matches!(prev, b's' | b'r') && vn;
        let bd = i != 0 && matches!(prev, b'b' | b'd') && vn;
        if (v == 2 || v == 5) && sr {
            return ZG;
        }
        if v == 1 || v == 3 {
            if sr {
                return JJ;
            }
        } else if sr {
            return Y;
        }
        if (v == 2 || v == 5) && bd {
            return ZG;
        }
        if v != 0 {
            if bd {
                return JJ;
            }
            if v == 2 {
                if i == 0 {
                    return I;
                }
                if isin(&st.vowels, prev) && vn {
                    return ZG;
                }
            }
        } else if bd {
            return Y;
        }
        // 0x1000feda
        if i == 0 {
            return I;
        }
        if isin(&st.vowels, prev) && vn {
            return Y;
        }
        if mnn || matches!(prev, b'b' | b'd' | b'r' | b's') || next == 0 {
            return I;
        }
        if vn {
            J
        } else {
            I
        }
    }

    /// `i` (0x1000d6..0x1000db): a full `i` in a few verb stems before an
    /// accented vowel (liar, desafiar, confiar…), else a glide `j` next to
    /// another vowel, else `i`.
    fn letter_i(&self, buf: &[u8], i: isize, sw: &dyn Fn(isize, &[u8]) -> bool) -> u32 {
        use ph::*;
        let st = &self.sets;
        let b = |k: isize| -> u8 { if k < 0 { 0 } else { at(buf, k as usize) } };
        let isin = |set: &[u8], c: u8| c != 0 && set.contains(&c);
        let (prev, next) = (b(i - 1), b(i + 1));
        let pre = |lit: &[u8]| sw(0, lit);
        let pre_any = |l: &[&[u8]]| l.iter().any(|x| pre(x));
        #[derive(Clone, Copy)]
        enum Lb {
            Start,
            L770,
            L840,
            L845,
            L8a3,
            L8e1,
            L930,
            L94c,
            L971,
            L976,
            L9db,
            L9e0,
            Lda19,
            Lda43,
            Lda7c,
            Lda85,
            Ldae5,
            Ldb14,
            Ldb2e,
            Ldb32,
            Ldb4f,
            Ldb99,
            After,
        }
        let mut lb = Lb::Start;
        loop {
            lb = match lb {
                Lb::Start => match i {
                    2 => {
                        if pre_any(&[b"al", b"av"]) {
                            Lb::L770
                        } else {
                            Lb::L930
                        }
                    }
                    4 => {
                        if pre(b"desl") {
                            Lb::L770
                        } else {
                            Lb::L976
                        }
                    }
                    1 => {
                        if pre_any(&[b"l", b"f", b"d"]) {
                            Lb::L770
                        } else {
                            Lb::L8a3
                        }
                    }
                    3 => {
                        if pre_any(&[b"rel", b"arr", b"env"]) {
                            Lb::L770
                        } else {
                            Lb::L845
                        }
                    }
                    5 => {
                        if pre_any(&[b"desaf", b"desav", b"reenv"]) {
                            Lb::L770
                        } else {
                            Lb::L8e1
                        }
                    }
                    _ => Lb::L840,
                },
                Lb::L770 => {
                    if next == 0xe1 {
                        return I;
                    }
                    Lb::L840
                }
                Lb::L840 => match i {
                    3 => Lb::L845,
                    1 => Lb::L8a3,
                    5 => Lb::L8e1,
                    2 => Lb::L930,
                    _ => Lb::L971,
                },
                Lb::L971 => {
                    if i > 3 {
                        Lb::L976
                    } else {
                        Lb::L9db
                    }
                }
                Lb::L845 => {
                    if pre_any(&[b"arr", b"env", b"esp"]) {
                        Lb::L94c
                    } else {
                        Lb::Lda85
                    }
                }
                Lb::Lda85 => {
                    if pre_any(&[b"roc", b"vac", b"vig", b"pol"]) {
                        Lb::Ldae5
                    } else {
                        Lb::Ldb32
                    }
                }
                Lb::L930 => {
                    if pre(b"av") {
                        Lb::L94c
                    } else {
                        Lb::Ldb32
                    }
                }
                Lb::L8e1 => {
                    if pre_any(&[b"desaf", b"desav", b"reenv"]) {
                        Lb::L94c
                    } else {
                        Lb::L976
                    }
                }
                Lb::L8a3 => {
                    if pre_any(&[b"c", b"f"]) {
                        Lb::L94c
                    } else {
                        Lb::L9e0
                    }
                }
                Lb::L94c => {
                    if next != 0xf3 {
                        Lb::L971
                    } else {
                        return I;
                    }
                }
                Lb::L9e0 => {
                    if pre(b"c") {
                        Lb::Lda19
                    } else {
                        Lb::Ldb32
                    }
                }
                Lb::Lda19 => {
                    if next == 0xe9 {
                        return I;
                    }
                    if i == 4 {
                        Lb::Lda43
                    } else {
                        Lb::Lda7c
                    }
                }
                Lb::Lda7c => match i {
                    3 => Lb::Lda85,
                    4 => Lb::Ldb14,
                    _ => Lb::Ldb2e,
                },
                Lb::L976 => {
                    if (sw(i - 4, b"conf") || sw(i - 4, b"graf")) && matches!(next, 0xe1 | 0xf3) {
                        return I;
                    }
                    Lb::L9db
                }
                Lb::L9db => {
                    if i == 1 {
                        Lb::L9e0
                    } else if i != 4 {
                        Lb::Lda7c
                    } else if pre(b"desv") {
                        Lb::Lda19
                    } else {
                        Lb::Lda43
                    }
                }
                Lb::Lda43 => {
                    if pre_any(&[b"ampl", b"porf"]) {
                        Lb::Ldae5
                    } else {
                        Lb::Ldb14
                    }
                }
                Lb::Ldb14 => {
                    if pre(b"zurr") {
                        Lb::Ldae5
                    } else {
                        Lb::Ldb32
                    }
                }
                Lb::Ldae5 => {
                    if isin(&st.acute, next) {
                        return I;
                    }
                    Lb::Ldb2e
                }
                Lb::Ldb2e => {
                    if i != 0 {
                        Lb::Ldb32
                    } else {
                        Lb::Ldb4f
                    }
                }
                Lb::Ldb32 => {
                    if isin(&st.vowels, prev) {
                        Lb::Ldb99
                    } else {
                        Lb::Ldb4f
                    }
                }
                Lb::Ldb4f => {
                    if i > 1 && isin(&st.vowels, b(i - 2)) && prev == b'h' {
                        Lb::Ldb99
                    } else {
                        Lb::After
                    }
                }
                Lb::Ldb99 => {
                    if matches!(next, b'a' | b'e' | b'o' | b'u' | 0xe1 | 0xe9 | 0xf3 | 0xfa) {
                        return J;
                    }
                    Lb::After
                }
                Lb::After => {
                    if (i != 0 && isin(&st.consonants, prev)) || i == 0 {
                        if matches!(next, b'a' | b'e' | b'o' | b'u' | 0xe1 | 0xe9 | 0xf3 | 0xfa) {
                            return J;
                        }
                        let h_cons = |v: &[u8]| sw(i + 1, v) && isin(&st.consonants, b(i + 3));
                        if [&b"ha"[..], b"he", b"ho", b"hu", b"h\xe1", b"h\xe9", b"h\xf3"].iter().any(|v| h_cons(v)) {
                            return J;
                        }
                        if sw(i + 1, b"h\xfa") && b(i + 3) != 0 {
                            return if isin(&st.consonants, b(i + 3)) { J } else { I };
                        }
                    }
                    return I;
                }
            }
        }
    }
}

// --- FonTrasConfini (0x100102d0), FonAllofonoLingua (0x10010820) -------------

/// 0x10010290: an a, e or o, stressed or not.
fn aeo(c: u32) -> bool {
    matches!(c, ph::A | ph::E | ph::O | ph::A_ | ph::E_ | ph::O_)
}

/// A consonant (the code's class bits 2).
fn consonant(c: u32) -> bool {
    c & 3 == 2
}

/// A voiceless consonant (feature bit 4).
fn voiceless(c: u32) -> bool {
    consonant(c) && c & 0x10 != 0
}

impl Spanish {
    /// FonTrasConfini: the incoming word loses one of two identical
    /// consonants in a row; then across the boundary (when phones precede)
    /// i/u become glides or vowels by what follows, voicing assimilates
    /// (p t k ↔ BH DH GH), n before p/b is m, s voices in Chile and
    /// Argentina; the incoming d/g soften after anything but a nasal or l,
    /// a lone `y` after a word is a glide.
    fn tras_confini(&self, phones: &mut [u32], next: &mut Vec<u32>) {
        use ph::*;
        let mut k = 0;
        while k < next.len() {
            if k + 1 < next.len() && next[k] == next[k + 1] && consonant(next[k]) {
                next.remove(k);
            }
            k += 1;
        }
        let n = phones.len();
        if n == 0 {
            return;
        }
        let v = self.variant;
        let first = next.first().copied().unwrap_or(0);
        let prev2 = if n > 1 { phones[n - 2] } else { 0 };
        let last = phones[n - 1];
        let voiced_first = consonant(first) && first & 0x10 == 0 && first != W && first != J;
        let new = if n > 1 && prev2 != W && last == I && (aeo(first) || first == U || first == U_) {
            Some(J)
        } else if last == J && !aeo(first) && first != U && first != U_ {
            Some(I)
        } else if n > 1 && prev2 != J && last == U && (aeo(first) || first == I || first == I_) {
            Some(W)
        } else if last == P && voiced_first {
            Some(BH)
        } else if last == T && voiced_first {
            Some(DH)
        } else if last == K && voiced_first {
            Some(GH)
        } else if last == BH && voiceless(first) {
            Some(P)
        } else if last == DH && voiceless(first) {
            Some(T)
        } else if last == GH && voiceless(first) {
            Some(K)
        } else if last == N && (first == P || first == B) {
            Some(M)
        } else if n < 2 {
            None
        } else if v == 3 && prev2 != K && last == S && (first == T || first == F) {
            Some(Z2)
        } else if matches!(v, 2 | 3 | 5) && prev2 != K && last == S && voiceless(first) && first != S {
            Some(S2)
        } else if matches!(v, 2 | 3 | 5) && prev2 != K && last == S && consonant(first) && first & 0x10 == 0 && first != J {
            Some(Z2)
        } else {
            None
        };
        if let Some(c) = new {
            phones[n - 1] = c;
        }
        // 0x10010660
        if first == I && next.len() == 1 {
            next[0] = J;
            return;
        }
        let last = phones[n - 1];
        if last != N {
            if !matches!(last, M | NG | MF | NY | NYI | L | LL | LLI) && first == D {
                next[0] = DH;
                return;
            }
            if !matches!(last, M | NG | MF | NY | NYI) && first == G {
                next[0] = GH;
                return;
            }
        }
        if v == 5 {
            if matches!(last, D | DH | B | BH | S | Z2 | S2 | R) && first == JJ {
                next[0] = ZG;
                return;
            }
            if last & 3 == 1 && first == LL && next.get(1).is_some_and(|&c| c & 3 == 1) {
                next[0] = Y;
            }
        } else if !matches!(last, N | M | NG | MF | NY | NYI | L | LL | LLI) && first == JJ {
            next[0] = Y;
        }
    }

    /// FonAllofonoLingua: one phone of the chunk's list in its context
    /// (z → s, the palatals before l/n, r after s/n/l, b/p, the variants'
    /// seseo and yeísmo…); `tt` + `Th` inside a word (but `etcétera`)
    /// becomes `Ts`. Returns the index to continue from.
    fn allofono(&self, p: &mut Vec<Phone>, i: usize) -> usize {
        use ph::*;
        let v = self.variant;
        let code = |k: usize| p.get(k).map_or(0, |e| e.code);
        let word = |k: usize| p.get(k).map(|e| e.word);
        let c = code(i);
        let (prev, next) = (if i > 0 { code(i - 1) } else { 0 }, code(i + 1));
        let palatal = |x: u32| matches!(x, TSH | SH | NY | LL | JJ | Y | ZG);
        let set = |p: &mut Vec<Phone>, x: u32| p[i].code = x;
        match c {
            Z => set(p, S),
            J => {
                if i != 0 && prev & 3 == 1 && next & 3 == 2 {
                    set(p, I)
                }
            }
            0xd52 => set(p, X),
            GH => {
                if next == S {
                    set(p, K)
                }
            }
            LL if v == 1 => set(p, Y),
            DG => {
                if i == 0 {
                    p[0].code = JJ;
                    return 0;
                }
                if matches!(prev, NY | N | LL | L) {
                    set(p, JJ)
                } else {
                    set(p, Y)
                }
            }
            ZG if v == 4 => set(p, Y),
            L => {
                if palatal(next) {
                    set(p, LLI)
                }
            }
            R => {
                if i == 0 || word(i - 1) != word(i) {
                    set(p, RR)
                } else if matches!(prev, S | Z2 | N | L) {
                    set(p, if v == 5 { SS } else { RR })
                }
            }
            RT => set(p, R),
            N => {
                if next == F || (v == 4 && next == BH) {
                    set(p, MF)
                } else if next & 0x1e03 == 0xe02 || next == XC {
                    set(p, NG)
                } else if palatal(next) {
                    set(p, NYI)
                }
            }
            TH if v == 1 || v == 4 => set(p, S),
            T => {
                let etcetera = i != 0
                    && (i == 1 || word(i - 2) != word(i - 1))
                    && prev == E
                    && code(i + 2) == E_
                    && code(i + 3) == T
                    && code(i + 4) == E
                    && code(i + 5) == R
                    && code(i + 6) == A
                    && word(i + 6) != word(i + 7);
                if next == TH && word(i + 1) == word(i) && !etcetera {
                    p.remove(i);
                    p[i].code = TS;
                    return i.saturating_sub(1);
                }
            }
            B => {
                if i != 0 {
                    if prev & 3 == 1 && next & 3 == 2 && next & 0x10 != 0 {
                        set(p, P)
                    } else if prev & 3 != 2 || prev & 0x1e3 != 0x1c2 {
                        set(p, BH)
                    }
                }
            }
            0x20401ed2 => set(p, W),
            _ => {}
        }
        i
    }

    /// The variant's symbol for a phone code (ELQGetPhonemes).
    pub fn symbol(&self, code: u32) -> Option<&str> {
        self.phones.iter().find(|(_, c)| *c == code).and_then(|(s, _)| std::str::from_utf8(s).ok())
    }
}

// --- The engine's side (LoqTTS7.dll, `common`) calls these ------------------

impl common::Les for Spanish {
    fn letter(&self, c: u8) -> (bool, bool, u8) {
        let f = self.flags(c);
        (f & LETTER != 0, f & UPPER != 0, self.chars[c as usize].0)
    }
    fn leggi_numero(&self, toks: &mut [common::Tok], i: usize, kind: u32, dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
        Spanish::leggi_numero(self, toks, i, kind, dmy)
    }
    fn analizza(&self, w: &mut [Word]) {
        Spanish::analizza(self, w)
    }
    fn accenta(&self, w: &mut [Word]) {
        Spanish::accenta(self, w)
    }
    fn confini(&self, w: &mut [Word], mode: i32) {
        Spanish::confini(self, w, mode)
    }
    fn gradi(&self, w: &mut [Word]) {
        Spanish::gradi(self, w)
    }
    fn tras_parola(&self, w: &[Word], i: usize) -> Vec<u32> {
        Spanish::tras_parola(self, &w[i])
    }
    fn tras_confini(&self, _prev_bound: u8, phones: &mut [u32], next: &mut Vec<u32>) {
        Spanish::tras_confini(self, phones, next)
    }
    fn allofono(&self, _w: &[Word], phones: &mut Vec<Phone>, i: usize) -> usize {
        Spanish::allofono(self, phones, i)
    }
    fn symbol(&self, code: u32) -> Option<&str> {
        Spanish::symbol(self, code)
    }
    fn normalizza_accento(&self, w: &mut Vec<u8>) {
        Spanish::normalizza_accento(self, w)
    }
    fn pronunciabile(&self, w: &[u8]) -> bool {
        Spanish::pronunciabile(self, w)
    }
    fn symbol_name(&self, c: u8, mode: u8) -> Vec<u8> {
        Spanish::symbol_name(self, c, mode)
    }
    fn abbreviazioni(&self) -> &[common::Abbreviation] {
        self.abbreviation_table.get_or_init(|| common::abbreviation_table(self, &self.abbreviations))
    }
    fn text_filters(&self) -> &[common::TextFilter] {
        &self.filters
    }
}

impl super::Module for Spanish {
    fn analyse(&self, text: &str, voice: Option<&crate::voicefon::VoiceFon>) -> Vec<crate::g2p::Token> {
        common::analyse(self, text, voice)
    }
}

// --- What the engine's tokenizer asks of the module -------------------------

/// LesGetSymbolName's switches: 0x10003690 (the letters' and symbols'
/// names: index bytes 0x10003a5c for `c + 0x80`, jump table 0x10003934)
/// and, outside Spain, 0x10003ba0 first (b v w: index bytes 0x10003be8 for
/// `c - 0x42`, jump table 0x10003bd8).
const NAMES_INDEX: u32 = 0x10003a5c;
const NAMES_JUMPS: u32 = 0x10003934;
const NAMES_AM_INDEX: u32 = 0x10003be8;
const NAMES_AM_JUMPS: u32 = 0x10003bd8;

impl Spanish {
    /// LesNormalizzaAccento (0x100017d0): an apostrophe or backquote (` ' ’
    /// ´) after a vowel puts its accent on it (grave for the backquote);
    /// elsewhere it stays an apostrophe before a vowel or at the end, else
    /// goes; then graves become acutes.
    fn normalizza_accento(&self, w: &mut Vec<u8>) {
        let lower = |c: u8| self.chars[c as usize].0;
        let mut out: Vec<u8> = Vec::with_capacity(w.len());
        for k in 0..w.len() {
            let c = w[k];
            if matches!(c, b'`' | b'\'' | 0x92 | 0xb4) {
                // `strchr("aeiouy", c)` also finds the terminator for c = 0
                let vowel = |c: u8| c == 0 || b"aeiouy".contains(&c);
                let after_vowel = k != 0 && vowel(lower(w[k - 1]));
                if !after_vowel {
                    let n = at(w, k + 1);
                    if n == 0 || vowel(lower(n)) {
                        out.push(b'\'');
                    }
                } else if let Some(p) = out.last_mut() {
                    *p = if c == b'`' { grave(*p) } else { acute(*p) };
                }
            } else {
                out.push(c);
            }
        }
        for c in out.iter_mut() {
            *c = match *c {
                0xc0 => 0xc1,
                0xc8 => 0xc9,
                0xcc => 0xcd,
                0xd2 => 0xd3,
                0xd9 => 0xda,
                0xe0 => 0xe1,
                0xe8 => 0xe9,
                0xec => 0xed,
                0xf2 => 0xf3,
                0xf9 => 0xfa,
                c => c,
            };
        }
        *w = out;
    }

    /// LesPronunciabile (0x10003d70 → 0x10003c90): a word with an
    /// apostrophe, an accent or a vowel other than a e i o u y is
    /// pronounceable; else when it has a vowel and isn't a lone `j`.
    fn pronunciabile(&self, w: &[u8]) -> bool {
        if w.iter().any(|&c| c == b'`' || c == b'\'') {
            return true;
        }
        let mut low = Vec::new();
        for &c in w.iter().take(0x3f) {
            let f = self.flags(c);
            if f & ACCENTED != 0 || (f & VOWEL != 0 && !b"aeiouyAEIOUY".contains(&c)) {
                return true;
            }
            low.push(self.chars[c as usize].0);
        }
        if low == b"j" {
            return false;
        }
        low.iter().any(|&c| self.flags(c) & VOWEL != 0)
    }

    /// One case of a compiled `switch` returning a string: `mov eax, imm32;
    /// ret`, or `test al, al; je default; mov eax, imm32; ret` (only when
    /// `mode` is set; else the default "").
    fn case_string(&self, target: u32, mode: u8) -> Option<Vec<u8>> {
        let b = self.pe.at(target);
        match b {
            [0xb8, ..] => Some(self.pe.cstr(self.pe.u32(target + 1)).to_vec()),
            [0x84, 0xc0, 0x74, _, 0xb8, ..] => Some(if mode != 0 { self.pe.cstr(self.pe.u32(target + 5)).to_vec() } else { Vec::new() }),
            _ => None,
        }
    }

    /// LesGetSymbolName (0x10003b60 in Spain, 0x10003c20 elsewhere): a
    /// letter's or symbol's name, the punctuation's only when `mode` is set.
    fn symbol_name(&self, c: u8, mode: u8) -> Vec<u8> {
        if self.variant != 0 {
            let k = c as i8 as i32 - 0x42;
            if (0..=0x35).contains(&k) {
                let idx = self.pe.u8(NAMES_AM_INDEX + k as u32) as u32;
                if let Some(s) = self.case_string(self.pe.u32(NAMES_AM_JUMPS + 4 * idx), mode) {
                    return s;
                }
            }
        }
        let k = c as i8 as i32 + 0x80;
        if !(0..=0xfe).contains(&k) {
            return Vec::new();
        }
        let idx = self.pe.u8(NAMES_INDEX + k as u32) as u32;
        self.case_string(self.pe.u32(NAMES_JUMPS + 4 * idx), mode).unwrap_or_default()
    }
}

// --- LesConfiniProsodici (0x1000a7f0): prosodic boundaries ------------------

/// Verb classes (VERBO, VERBOA, VERBOE).
fn verb(c: u8) -> bool {
    matches!(c, b'*' | b'+' | b',')
}

/// Verb forms that don't close a group (subclasses 0x88 0x8e 0x89).
fn open_form(s: u8) -> bool {
    matches!(s, 0x88 | 0x8e | 0x89)
}

/// Multi-word expressions LesConfiniProsodici keeps together (0x10008eb0):
/// (words, table, count), searched exactly, longest first.
const LOCUTIONS: [(usize, u32, u32); 4] = [(5, 0x10025ffc, 3), (4, 0x10026008, 0x11), (3, 0x10026050, 0x31), (2, 0x10026118, 0x3d)];
/// Interjections (0x10008cd0).
const INTERJECTIONS: [(usize, u32, u32); 3] = [(3, 0x10025fd0, 1), (2, 0x10025fd4, 1), (1, 0x10025fd8, 9)];
/// The stack table of verb forms 0x10009100 fills from immediates (0x15b
/// `mov [esp+n], imm32` from 0x1000911b) and searches.
const MODAL_FORMS_CODE: u32 = 0x1000911b;
const MODAL_FORMS_COUNT: usize = 0x15b;

impl Spanish {
    /// `ELQSearchString(s, table, 1, count)` (LTTS7Util.dll 0x10028ca0):
    /// an exact match, binary search over more than 3 entries.
    fn search_exact(&self, s: &[u8], table: u32, count: u32) -> bool {
        let entry = |i: u32| self.pe.cstr(self.pe.u32(table + 4 * i));
        if count > 3 {
            let (mut lo, mut hi) = (0i32, count as i32 - 1);
            while lo <= hi {
                let mid = (lo + hi) / 2;
                match s.cmp(entry(mid as u32)) {
                    std::cmp::Ordering::Less => hi = mid - 1,
                    std::cmp::Ordering::Greater => lo = mid + 1,
                    std::cmp::Ordering::Equal => return true,
                }
            }
            false
        } else {
            (0..count).any(|i| entry(i) == s)
        }
    }

    /// 0x10008c70: accents dropped.
    fn unaccented(&self, s: &[u8]) -> Vec<u8> {
        s.iter().map(|&c| if self.flags(c) & ACCENTED != 0 { self.plain(c) } else { c }).collect()
    }

    /// 0x10008eb0 / 0x10008cd0: the longest expression of `table` the
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
            if self.search_exact(&s, table, count) {
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

    /// 0x10009100: a form of deber, ser, estar, haber, poder, tener… (the
    /// word unaccented).
    fn modal(&self, t: &[u8]) -> bool {
        let key = self.unaccented(t);
        let mut forms = Vec::with_capacity(MODAL_FORMS_COUNT);
        let mut va = MODAL_FORMS_CODE;
        while forms.len() < MODAL_FORMS_COUNT {
            let b = self.pe.at(va);
            match (b.first(), b.get(1), b.get(2)) {
                (Some(0xc7), Some(0x04), Some(0x24)) => {
                    forms.push((0u32, self.pe.u32(va + 3)));
                    va += 7;
                }
                (Some(0xc7), Some(0x44), Some(0x24)) => {
                    forms.push((b[3] as u32, self.pe.u32(va + 4)));
                    va += 8;
                }
                (Some(0xc7), Some(0x84), Some(0x24)) => {
                    forms.push((self.pe.u32(va + 3), self.pe.u32(va + 7)));
                    va += 11;
                }
                _ => break,
            }
        }
        forms.sort();
        let entry = |i: usize| self.pe.cstr(forms[i].1);
        let (mut lo, mut hi) = (0i32, forms.len() as i32 - 1);
        while lo <= hi {
            let mid = (lo + hi) / 2;
            match key.as_slice().cmp(entry(mid as usize)) {
                std::cmp::Ordering::Less => hi = mid - 1,
                std::cmp::Ordering::Greater => lo = mid + 1,
                std::cmp::Ordering::Equal => return true,
            }
        }
        false
    }

    /// 0x10008b40: the words before `a` (back to a pause) and after `b` (to
    /// punctuation) are not both more than 5 words and 24 letters.
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
        !(before_words > 5 && before_chars > 0x18 && after_words > 5 && after_chars > 0x18)
    }

    /// 0x100083e0: the word at `i` opens a relative clause (cuyo, quien, a
    /// quien, de que, a los que…).
    fn relative(w: &[Word], i: usize) -> bool {
        let t = |k: usize| w.get(k).map(|r| r.text.as_slice());
        let Some(w0) = t(i) else { return false };
        if matches!(w0, b"cuya" | b"cuyo" | b"cuyas" | b"cuyos" | b"quien") {
            return true;
        }
        let Some(w1) = t(i + 1) else { return false };
        if w0 == b"a" && matches!(w1, b"quien" | b"quienes") {
            return true;
        }
        if matches!(w0, b"de" | b"del" | b"el" | b"la" | b"los" | b"las") && w1 == b"que" {
            return true;
        }
        if (w0 == b"con" && w1 == b"quien") || (w0 == b"por" && w1 == b"quienes") || (w0 == b"del" && w1 == b"cual") || (w0 == b"los" && w1 == b"quales") {
            return true;
        }
        let Some(w2) = t(i + 2) else { return false };
        (w0 == b"a" && matches!(w1, b"los" | b"las") && w2 == b"que")
            || (w0 == b"a" && w1 == b"los" && w2 == b"quales")
            || (w0 == b"de" && w1 == b"lo" && w2 == b"cual")
            || (w0 == b"con" && w1 == b"las" && w2 == b"quales")
    }

    /// 0x1000a660: a pause may follow the word at `i`.
    fn may_pause(&self, w: &[Word], i: usize) -> bool {
        let c = w[i].f[CLASS];
        let next = w.get(i + 1);
        let nc = next.map_or(0, |r| r.f[CLASS]);
        if (c == 0x18 && nc == 0x15) || (c == b')' && nc == 0x15) || (c == 0x15 && nc == b')') {
            return false;
        }
        let Some(next) = next else { return true };
        if verb(c) && (matches!(nc, b'3' | b')' | 0x18 | 0x15 | 0x19) || (verb(nc) && next.f[SUB] == 0x8d)) {
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
        if c == 0x15 && Self::relative(w, i + 1) {
            return false;
        }
        !(self.modal(&w[i].text) && verb(nc))
    }

    /// 0x1000a010: each word's group kind (0 open, 1 noun phrase end, 2
    /// verb group end, 4 …, 5 default, 6 conjunction, 7 inside, 8
    /// punctuation, 10 the end) and whether a pause is wanted after it.
    fn groups(&self, w: &[Word]) -> Vec<(u32, u32)> {
        let n = w.len();
        let mut arr = vec![(5u32, 0u32); n + 1];
        if n == 0 {
            arr[0].0 = 10;
            return arr;
        }
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
            let s = sub(ui);
            let same_sides = class(ui - 1) == class(ui + 1) && sub(ui - 1) == sub(ui + 1);
            #[derive(PartialEq)]
            enum Go {
                Done,
                F4f9,
            }
            let go = match state {
                1 => {
                    if matches!(c, b'7' | b' ' | b'"' | b'$' | b'(' | b'!' | b'.' | 0x1f | b'3') {
                        state = 1;
                        Go::Done
                    } else if verb(c) && !open_form(s) {
                        state = 2;
                        Go::Done
                    } else {
                        state = 0;
                        Go::F4f9
                    }
                }
                2 => {
                    if !matches!(c, b'7' | b'*' | b'+' | b',' | b'3') && !(s == 0x82 && same_sides) {
                        if matches!(w[u].text.as_slice(), b"a" | b"al" | b"de" | b"del") && verb(class(ui + 1)) {
                            state = 2;
                            Go::Done
                        } else if mw_len == 0 {
                            fill(&mut arr, start, u - 1);
                            arr[u - 1].0 = 1;
                            state = 0;
                            Go::F4f9
                        } else {
                            Go::Done
                        }
                    } else {
                        state = 2;
                        Go::Done
                    }
                }
                3 => {
                    if !matches!(c, b'7' | 0x18 | 0x16 | 0x1f) {
                        if matches!(c, b'/' | 0x15 | b')') || matches!(s, 0x88 | 0x8e | 0x89 | 0x8d | 0x8c) {
                            state = 5;
                            Go::Done
                        } else {
                            state = 0;
                            Go::F4f9
                        }
                    } else {
                        state = 3;
                        Go::Done
                    }
                }
                4 | 6 => {
                    // case 6 first: what case 4 shares from 0x1000a3db
                    let mut to4 = false;
                    if state == 6 {
                        if matches!(c, b' ' | b'"' | b'$' | b'(' | b'!' | b'.' | 0x1f | b'3') {
                            state = 1;
                            to4 = true;
                        } else if verb(c) && !open_form(s) && !(u > 1 && open_form(sub(ui - 2))) {
                            state = 2;
                            to4 = true;
                        }
                    }
                    if to4 {
                        Go::Done
                    } else if matches!(c, 0x18 | 0x16 | 0x1f) {
                        state = 4;
                        Go::Done
                    } else if matches!(c, b'7' | b'/' | 0x15 | b')') || open_form(s) {
                        state = 7;
                        Go::Done
                    } else {
                        state = 0;
                        Go::F4f9
                    }
                }
                5 => {
                    if !matches!(c, b'7' | b'/' | 0x15 | b')') && !matches!(s, 0x88 | 0x8e | 0x89 | 0x8d | 0x8c) && !(s == 0x82 && same_sides) {
                        if mw_len != 0 {
                            Go::Done
                        } else {
                            fill(&mut arr, start, u - 1);
                            arr[u - 1].0 = 0;
                            state = 0;
                            Go::F4f9
                        }
                    } else {
                        state = 5;
                        Go::Done
                    }
                }
                7 => {
                    if !matches!(c, b'/' | 0x15 | b')' | b'7') && !open_form(s) {
                        if mw_len != 0 {
                            Go::Done
                        } else {
                            fill(&mut arr, start, u - 1);
                            arr[u - 1].0 = 2;
                            let opens = start == 0
                                && w[0].f[CLASS] != b'3'
                                && w[0].f[CLASS] != 0x19
                                && self.expression(w, 0, &LOCUTIONS).is_none_or(|l| u <= l)
                                && self.expression(w, 0, &INTERJECTIONS).is_none_or(|l| u <= l);
                            if !(start != 0 || opens) {
                                arr[u - 1].1 = 1;
                            }
                            state = 0;
                            Go::F4f9
                        }
                    } else {
                        state = 7;
                        Go::Done
                    }
                }
                8 => {
                    arr[u - 1].0 = 6;
                    state = 0;
                    Go::F4f9
                }
                _ => {
                    if state == 0 {
                        Go::F4f9
                    } else {
                        Go::Done
                    }
                }
            };
            if go == Go::F4f9 {
                start = u;
                if c == 0x1f {
                    state = 8;
                } else if matches!(c, b' ' | b'"' | b'$' | b'(' | b'!' | b'.' | b'3') {
                    state = 1;
                } else if verb(c) && !open_form(s) {
                    if !matches!(s, 0x8d | 0x8c | 0x8b) && u != 0 && class(ui - 1) == b'-' {
                        arr[u - 1].0 = 0;
                    }
                    state = 2;
                } else if matches!(c, 0x18 | 0x16) {
                    state = 3;
                } else if matches!(c, b'7' | b'/' | 0x15 | b')') || open_form(s) {
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
                match arr[u].0 {
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
                    k @ (0 | 2) => {
                        if k == 2 && arr[u].1 == 1 && !Self::short_sides(w, u, u) && self.may_pause(w, u) {
                            w[u].f[BOUND] = 10;
                            last = u;
                        } else if class == b'7' {
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
                        } else if matches!(class, 0x19 | 0x1e | 0x18 | b' ' | b'!' | 0x16 | b'"' | 0x1f | b'.') {
                            w[u].f[BOUND] = 1;
                        } else {
                            w[u].f[BOUND] = 3;
                        }
                    }
                    8 => {
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
                            last = u;
                        }
                    }
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
}
