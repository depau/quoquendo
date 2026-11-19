//! The module's text functions the engine's tokenizer calls:
//! LesPronunciabile (0x10002e90), LesGetSymbolName (US 0x100021e0, GB/AU
//! 0x100021a0), LesNormalizzaAccento (0x1000c3b0), LesElaboraParola
//! (0x1000a600), LesGetAbbreviazioni (0x10001120, 0xfc entries).

use super::{English, Variant};
use crate::lang::common::{record, Abbreviation, Word};

/// Character flag (0x10027d00 +4): upper case.
const UPPER: u32 = 0x20;
/// Character flag (0x10027d00 +4): vowel.
const VOWEL: u32 = 0x10;

/// FUN_10002290's consonant: an ASCII letter other than `a e i o u y j`.
fn cons(c: u8) -> bool {
    let c = c.to_ascii_lowercase();
    (b'b'..=b'z').contains(&c) && !b"eiouyj".contains(&c)
}

/// One of `set` ignoring ASCII case (the C tests both cases of each).
fn among(c: u8, set: &[u8]) -> bool {
    set.contains(&c.to_ascii_lowercase())
}

/// FUN_10002290: whether the word's initial consonant cluster is one
/// English allows. Four leading consonants: only `sch` + l m n r t w and
/// `mc` + a three-consonant onset, and never five or a whole four-letter
/// word; three: `chl chr mc? pfl phl phn phr sch scl scr shr sht skr sph spl
/// spr str thr thw`; two: the listed onsets (`bl`, `ch`, `dw`, `sk`, …); a
/// word of two consonants alone is rejected.
fn initial_cluster(w: &[u8]) -> bool {
    let n = w.len();
    let at = |i: usize| w.get(i).copied().unwrap_or(0);
    let (c0, c1, c2, c3) = (at(0), at(1), at(2), at(3));
    if n > 3 && cons(c0) && cons(c1) && cons(c2) && cons(c3) {
        if n == 4 || cons(at(4)) {
            return false;
        }
        return match c0.to_ascii_lowercase() {
            b's' => among(c1, b"c") && among(c2, b"h") && among(c3, b"lmnrtw"),
            b'm' if among(c1, b"c") => match c2.to_ascii_lowercase() {
                b'b' => among(c3, b"r"),
                b'c' => among(c3, b"hlr"),
                b'f' => among(c3, b"l"),
                b'g' => among(c3, b"hlrw"),
                b'k' => among(c3, b"ln"),
                b'p' => among(c3, b"h"),
                b's' => among(c3, b"hlptw"),
                b'w' => among(c3, b"hr"),
                _ => false,
            },
            _ => false,
        };
    }
    if n > 2 && cons(c0) && cons(c1) && cons(c2) {
        if n == 3 {
            return false;
        }
        return match c0.to_ascii_lowercase() {
            b'c' => among(c1, b"h") && among(c2, b"lr"),
            b'm' => among(c1, b"c") && !among(c2, b"x"),
            b'p' => (among(c1, b"f") && among(c2, b"l")) || (among(c1, b"h") && among(c2, b"lnr")),
            b's' => {
                (among(c1, b"c") && among(c2, b"hlr"))
                    || (among(c1, b"h") && among(c2, b"rt"))
                    || (among(c1, b"k") && among(c2, b"r"))
                    || (among(c1, b"p") && among(c2, b"hlr"))
                    || (among(c1, b"t") && among(c2, b"r"))
            }
            b't' => among(c1, b"h") && among(c2, b"rw"),
            _ => false,
        };
    }
    if n < 2 || !cons(c0) || !cons(c1) {
        return true;
    }
    if n == 2 {
        return false;
    }
    let ok: &[u8] = match c0.to_ascii_lowercase() {
        b'b' | b'f' | b'v' => b"lr",
        b'c' => b"hlr",
        b'd' => b"hrw",
        b'g' | b'k' => b"hlnrw",
        b'l' => b"l",
        b'm' => b"c",
        b'p' => b"fhlnrst",
        b'r' => b"h",
        b's' => b"bchklmnpqrtvwz",
        b't' => b"hrsw",
        b'w' => b"hr",
        b'z' => b"hw",
        _ => b"",
    };
    among(c1, ok)
}

/// FUN_10002de0 with its char argument 0 (what the engine's
/// LesElaboraParola LAB_100381b0 and the module's own pass): a word in the
/// database, or ending in `'s`, is readable; otherwise a word shorter than
/// four letters with an upper-case letter after its first (FUN_10002250)
/// is not, nor one whose initial cluster `initial_cluster` (FUN_10002290)
/// rejects; the rest are readable when they have a vowel (FUN_10002220).
pub(super) fn pronunciabile(en: &English, w: &[u8]) -> bool {
    if !en.lex.find(w).is_empty() || w.ends_with(b"'s") {
        return true;
    }
    if w.len() < 4 && w.iter().skip(1).any(|&c| en.flags(c) & UPPER != 0) {
        return false;
    }
    initial_cluster(w) && w.iter().any(|&c| en.flags(c) & VOWEL != 0)
}

/// FUN_10001cc0: a character's name; `full` (AL) also names punctuation.
fn char_name(c: u8, full: bool) -> &'static [u8] {
    let only_full = |s: &'static [u8]| if full { s } else { b"" };
    match c {
        b'$' => b"dollars",
        b'%' => b"per-cent",
        b'&' if full => b"and",
        b'&' | 0xc6 | 0xc8..=0xcb | 0xe6 | 0xe8..=0xeb | b'E' | b'e' => b"e",
        b'\'' => only_full(b"apostrophe"),
        b'(' => only_full(b"opening-round-bracket"),
        b')' => only_full(b"closing-round-bracket"),
        b'*' => only_full(b"asterisk"),
        b'+' => b"plus",
        b',' => only_full(b"comma"),
        b'-' | b'~' => only_full(b"hyphen"),
        b'/' => only_full(b"slash"),
        b':' => only_full(b"colon"),
        b';' => only_full(b"semicolon"),
        b'<' => only_full(b"less-than"),
        b'=' => b"equals",
        b'>' => only_full(b"greater-than"),
        b'@' => only_full(b"at"),
        b'[' | b']' => only_full(b"square-bracket"),
        b'\\' => only_full(b"back-slash"),
        b'^' => only_full(b"caret"),
        b'_' => only_full(b"underscore"),
        b'`' => only_full(b"accent"),
        b'{' => b"opening-curly-bracket",
        b'|' => only_full(b"vertical-bar"),
        b'}' => b"closing-curly-bracket",
        0x80 => b"euro",
        0x93 | 0x94 | 0xab | 0xbb | b'"' => only_full(b"quotation-mark"),
        0xa1 | b'!' => only_full(b"exclamation-mark"),
        0xa3 => b"pounds",
        0xa5 => b"y\xe8n",
        0xa7 => b"paragraph",
        0xa9 => b"copyright",
        0xae => b"registered",
        0xb0 | 0xba => only_full(b"degrees"),
        0xb1 => b"plus-or-minus",
        0xb5 => b"micro",
        0xbc => b"one-fourth",
        0xbd => b"one-half",
        0xbe => b"three-fourths",
        0xbf | b'?' => only_full(b"question-mark"),
        0xc0..=0xc5 | 0xe0..=0xe5 | b'A' | b'a' => b"a",
        0xc7 | 0xe7 | b'C' | b'c' => b"c",
        0xcc..=0xcf | 0xec..=0xef | b'I' | b'i' => b"i",
        0xd1 | 0xf1 | b'N' | b'n' => b"n",
        0xd2..=0xd6 | 0xf2..=0xf6 | b'O' | b'o' => b"o",
        0xd9..=0xdc | 0xf9..=0xfc | b'U' | b'u' => b"u",
        0xdd | 0xfd | 0xff | b'Y' | b'y' => b"y",
        0xdf => b"beta",
        b'B' | b'D' | b'F'..=b'H' | b'J'..=b'M' | b'P'..=b'T' | b'V'..=b'X' | b'Z' => match c.to_ascii_lowercase() {
            l @ (b'b' | b'd' | b'f' | b'g' | b'h' | b'j' | b'k' | b'l' | b'm' | b'p' | b'q' | b'r' | b's' | b't' | b'v' | b'w' | b'x' | b'z') => {
                LETTERS[(l - b'a') as usize]
            }
            _ => b"",
        },
        b'b' | b'd' | b'f'..=b'h' | b'j'..=b'm' | b'p'..=b't' | b'v'..=b'x' | b'z' => LETTERS[(c - b'a') as usize],
        _ => b"",
    }
}

const LETTERS: [&[u8]; 26] = [
    b"a", b"b", b"c", b"d", b"e", b"f", b"g", b"h", b"i", b"j", b"k", b"l", b"m", b"n", b"o", b"p", b"q", b"r", b"s", b"t", b"u", b"v", b"w", b"x", b"y", b"z",
];

/// LesGetSymbolName: US 0x100021e0 over FUN_10002140 (`#` "pound"),
/// GB/AU 0x100021a0 over FUN_10002170 (`#` "hash"); `.` is "dot" when
/// `mode` is set, else nothing; the rest FUN_10001cc0.
fn symbol_name_us(c: u8, mode: u8, us: bool) -> Vec<u8> {
    match c {
        b'#' if us => b"pound".to_vec(),
        b'#' => b"hash".to_vec(),
        b'.' if mode != 0 => b"dot".to_vec(),
        b'.' => Vec::new(),
        c => char_name(c, mode != 0).to_vec(),
    }
}

pub(super) fn symbol_name(en: &English, c: u8, mode: u8) -> Vec<u8> {
    symbol_name_us(c, mode, en.variant == Variant::Us)
}

/// LesNormalizzaAccento (0x1000c3b0): `` ` `` `’` `´` become `'`, then
/// FUN_1000c250 drops accents (`Æ` → `E`, `Ç` → `S`, `ß` `ç` → `s`, `Ñ` →
/// `NI`, `ñ` → `ni`, `Ÿ` → `Y`), keeping at most 62 input characters.
pub(super) fn normalizza_accento(_en: &English, w: &mut Vec<u8>) {
    for c in w.iter_mut() {
        if matches!(*c, b'`' | b'\'' | 0x92 | 0xb4) {
            *c = b'\'';
        }
    }
    let mut out = Vec::with_capacity(w.len() + 1);
    for &c in w.iter() {
        if out.len() >= 0x3e {
            break;
        }
        match c {
            0x9f => out.push(b'Y'),
            0xc0..=0xc5 => out.push(b'A'),
            0xc6 | 0xc8..=0xcb => out.push(b'E'),
            0xc7 => out.push(b'S'),
            0xd1 => out.extend_from_slice(b"NI"),
            0xcc..=0xcf => out.push(b'I'),
            0xd2..=0xd6 | 0xd8 => out.push(b'O'),
            0xd9..=0xdc => out.push(b'U'),
            0xdf | 0xe7 => out.push(b's'),
            0xe0..=0xe5 => out.push(b'a'),
            0xe6 | 0xe8..=0xeb => out.push(b'e'),
            0xf0 | 0xf2..=0xf6 | 0xf8 => out.push(b'o'),
            0xf1 => out.extend_from_slice(b"ni"),
            0xec..=0xef => out.push(b'i'),
            0xf9..=0xfc => out.push(b'u'),
            0xff => out.push(b'y'),
            c => out.push(c),
        }
    }
    *w = out;
}

/// FUN_10007060: a record as the module adds it (bound 0x15, grade 0x0e),
/// its text cut to 63 characters.
fn add(text: &[u8], class: u8, sub: u8, info: u8) -> Word {
    record(text[..text.len().min(0x3f)].to_vec(), class, sub, info)
}

/// The letters of `t` spelled (LesElaboraParola's loops at 0x1000aef0 and
/// 0x1000b017): each name (LesGetSymbolName with mode 1, by the session's
/// Language: `englishus` the US names), normalised, a record 0x15 with
/// info 200, sub 0x83, the last character's 0x84 with `last_suffix`
/// appended; characters without a name are skipped.
fn spell(en: &English, t: &[u8], last_suffix: &[u8]) -> Vec<Word> {
    let mut out = Vec::new();
    for (k, &c) in t.iter().enumerate() {
        let mut name = symbol_name(en, c, 1);
        if name.is_empty() {
            continue;
        }
        normalizza_accento(en, &mut name);
        let last = k + 1 == t.len();
        if last {
            name.extend_from_slice(last_suffix);
        }
        out.push(add(&name, 0x15, if last { 0x84 } else { 0x83 }, 200));
    }
    out
}

/// LesElaboraParola (0x1000a600), the word path: with `mode` 0 and the
/// word readable (LesPronunciabile), `spell` set or the token from an
/// expansion, one record (0x17, 200, 200); a word in `'s` whose stem is
/// not readable has its stem spelled and `'s` (0x1002a908) appended to the
/// last name. Otherwise the word is spelled letter by letter.
pub(super) fn elabora_parola(en: &English, token: &[u8], _idx: usize, flag408: u8, spell_arg: u8, mode: u8) -> Option<Vec<Word>> {
    let pron = pronunciabile(en, token);
    if mode == 0 && (pron || spell_arg != 0 || flag408 != 0) {
        if token.len() >= 2 && token.ends_with(b"'s") {
            let stem = &token[..token.len() - 2];
            if !pronunciabile(en, stem) {
                return Some(spell(en, stem, b"'s"));
            }
        }
        return Some(vec![add(token, 0x17, 200, 200)]);
    }
    Some(spell(en, token, b""))
}

/// LesGetAbbreviazioni (0x10001120): LesGetNumeroAbbreviazioni (0x10001110)
/// pairs at 0x100288e0, the abbreviation at +0, its expansion at +4.
const ABBREVIATIONS: u32 = 0x100288e0;
const ABBREVIATION_COUNT: u32 = 0xfc;

pub(super) fn abbreviazioni(en: &English) -> Vec<Abbreviation> {
    let pairs: Vec<(Vec<u8>, Vec<u8>)> = (0..ABBREVIATION_COUNT)
        .map(|i| {
            let e = ABBREVIATIONS + 8 * i;
            (en.pe.cstr(en.pe.u32(e)).to_vec(), en.pe.cstr(en.pe.u32(e + 4)).to_vec())
        })
        .collect();
    crate::lang::common::abbreviation_table(en, &pairs)
}

#[cfg(test)]
mod probe {
    #[test]
    #[ignore]
    fn testo_probe() {
        let root = "/tmp/qqv/LTTS7";
        let read = |p: String| std::fs::read(&p).unwrap();
        let files = crate::lang::ModuleFiles {
            dll: read(format!("{root}/bin/LTTS7English.dll")),
            mother_tongue: "EnglishUs".into(),
            dbl: Some(read(format!("{root}/data/languages/English/EnglishUs.dbl"))),
            gpr: None,
            lexicons: Vec::new(),
            phones: Vec::new(),
        };
        let en = super::English::from_files(&files).unwrap();
        for w in std::env::var("QQ_W").unwrap().split(',') {
            let w = w.as_bytes();
            println!("{} find={:?} pron={}", String::from_utf8_lossy(w), en.lex.find(w), super::pronunciabile(&en, w));
        }
    }
}
