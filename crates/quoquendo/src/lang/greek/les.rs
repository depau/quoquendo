//! The LES side of LTTS7Greek.dll: the word records the engine builds
//! (LesElaboraParola), their classes (LesAnalizza), the written stress
//! (LesAccenta), the sentence type (LesSetModalitaFrase), the engine's own
//! LesConfiniProsodici (the module has none) and the stress grades
//! (LesGradiAccentuali).

use super::{col, Greek, Lex, ACCENTED, LETTER, VOWEL};
use crate::lang::common::{self, field::*, record, tok, Endings, Tok, Word};

/// LesAccenta's ending table for words in Latin letters (0x100011c0:
/// 0x27e entries of `{ reversed ending, exception list, code }`, searched
/// as Italian's, `common::lookup`).
const LATIN_ENDINGS: Endings = Endings { at: 0x1001d1a0, count: 0x27e };

/// The six unaccented words LesAccenta's 0x10001520 replaces (`δυο`,
/// `μια`, `μιαν`, `μιας`, `πια`, `πιο`), each followed by its accented
/// form, in the order of the pointers it builds on its stack.
const ACCENTED_WORDS: [(u32, u32); 6] = [
    (0x1001efc4, 0x1001efc0),
    (0x1001efbc, 0x1001efb8),
    (0x1001efb0, 0x1001efa8),
    (0x1001efa0, 0x1001ef98),
    (0x1001ef94, 0x1001ef90),
    (0x1001ef8c, 0x1001ef88),
];

/// The classes LesAnalizza takes from the dictionary for a proper noun and
/// LesGradiAccentuali leaves unstressed (function words).
const FUNCTION_CLASSES: [u8; 9] = [0x19, 0x1e, 0x18, 0x20, 0x21, 0x16, 0x22, 0x1f, 0x2e];

impl Greek {
    /// `LesElaboraParola` (0x10008cb0): the word token `toks[i]` as one
    /// record (0x17), or spelled letter by letter (LesGetSymbolName, mode 3
    /// when the token is a single character alone in the buffer, else 1)
    /// when it is alone (0x10008b20), `mode` asks for it, or LesPronunciabile
    /// rejects it and neither `spell` nor the token's expansion flag says to
    /// read it.
    pub(super) fn elabora_parola(&self, toks: &[Tok], i: usize, _idx: usize, spell: u8, mode: u8) -> Vec<Word> {
        let t = &toks[i].text;
        // 0x10008b20: at most one character and every other token a tag
        let alone = t.len() < 2 && toks.iter().enumerate().all(|(k, tk)| k == i || tk.kind == tok::TAG);
        let pron = self.pronunciabile(t);
        if !alone && mode == 0 && (pron || spell != 0 || toks[i].expanded) {
            return vec![record(t.clone(), 0x17, 200, 200)];
        }
        let m = if alone { 3 } else { 1 };
        t.iter()
            .enumerate()
            .map(|(k, &c)| record(self.symbol_name_mode(c, m), 0x15, if k + 1 == t.len() { 0x84 } else { 0x83 }, 9))
            .collect()
    }

    /// `LesPronunciabile` (0x100030d0 → 0x10002fc0): readable when it has a
    /// `` ` `` or `'`, mixes Latin and other letters, has an accented letter
    /// or a vowel that is not a Latin one; otherwise when its lower-case
    /// form has a vowel (0x10002f90). At most 0x3f characters are looked at.
    pub(super) fn pronunciabile(&self, w: &[u8]) -> bool {
        if w.iter().any(|&c| c == b'`' || c == b'\'') {
            return true;
        }
        let mut latin = false;
        for (k, &c) in w.iter().enumerate().take(0x3f) {
            if c.is_ascii_alphabetic() {
                if k == 0 {
                    latin = true;
                } else if !latin {
                    return true;
                }
            } else if self.has(c, LETTER) {
                if k == 0 {
                    latin = false;
                } else if latin {
                    return true;
                }
            }
        }
        let mut low = Vec::new();
        for &c in w.iter().take(0x3f) {
            if self.has(c, ACCENTED) || (self.has(c, VOWEL) && !b"aeiouyAEIOUY".contains(&c)) {
                return true;
            }
            low.push(self.lower(c));
        }
        low.iter().any(|&c| self.has(c, VOWEL))
    }

    /// `LesGetSymbolName` (0x10002f50 → 0x10002400, the character in CL, the
    /// mode in AL): a letter's or symbol's name; mode 0 names only
    /// letters and a few symbols, 2 and 3 add "με τόνο"/"κεφαλαίο" forms.
    pub(super) fn symbol_name_mode(&self, c: u8, mode: u8) -> Vec<u8> {
        let m = match mode {
            0 => 0,
            2 => 2,
            3 => 3,
            _ => 1,
        };
        self.s(symbol_name_at(c)[m]).to_vec()
    }

    /// `LesAnalizza` (0x10009620 → 0x100094e0): for every word record, its
    /// dictionary rows (0x100093b0, `ELQDBTableFind` by `grafemica`, added
    /// to the record's list); a word the dictionary lacks or gives no
    /// `grammaticale` keeps class 0x17 → 0x2f (0x10008df0); otherwise the
    /// current row's `grammaticale` (0x10008e10), `specgramm` (0x100091b0)
    /// and `gennum` (0x10009260) set class, subclass and information, for
    /// an unknown word (0x17) or a proper noun (0x15 with subclass 0xc8)
    /// the dictionary calls a function word.
    pub(super) fn analizza(&self, w: &mut [Word]) {
        // the row count of the last lookup and the flag of the last word
        // that had rows carry over (the original's locals)
        let mut has_gram = false;
        for r in w.iter_mut() {
            if r.f[CLASS] == b'2' {
                continue;
            }
            let mut lex = Lex::of(r);
            let (first, count) = self.table_find(&r.text);
            for row in first..first + count {
                lex.add(row as u32);
            }
            let n = lex.len();
            if n > 0 {
                // 0x10009450: walks the list from its head to the first row
                // with a non-empty grammaticale (left current; at none the
                // walk ends on the last row)
                has_gram = false;
                let mut cur = lex.head();
                while let Some(row) = cur {
                    if self.col(row, col::GRAMMATICALE).is_some_and(|g| !g.is_empty()) {
                        has_gram = true;
                        break;
                    }
                    cur = lex.next();
                }
            }
            if (n == 0 || !has_gram) && r.f[CLASS] == 0x17 {
                r.f[CLASS] = 0x2f;
            } else if let Some(row) = lex.current() {
                let class = grammar_class(self.col(row, col::GRAMMATICALE));
                if r.f[CLASS] == 0x17 || (r.f[CLASS] == 0x15 && r.f[SUB] == 0xc8 && FUNCTION_CLASSES.contains(&class)) {
                    r.f[CLASS] = class;
                    r.f[SUB] = subclass(self.col(row, col::SPECGRAMM));
                    r.f[INFO] = gender_number(self.col(row, col::GENNUM));
                }
            }
            lex.store(r);
        }
    }

    /// `LesAccenta` (0x10001820): every record not elided (`+0xc` 0xffff)
    /// without an accented letter gets one: a word mostly in Latin
    /// lower-case letters (more than half, or one of two) through the
    /// Latin ending table (0x10001730; the channel's byte +4, 1 unless a
    /// `\#Ge` tag set it, picks it over 0x100017a0), else the Greek rule
    /// (0x10001520).
    pub(super) fn accenta(&self, w: &mut [Word]) {
        for r in w.iter_mut() {
            if r.f[0xc] != 0xff || r.f[0xd] != 0xff {
                continue;
            }
            // 0x100014e0: an accented letter before the end
            if r.text.iter().any(|&c| self.has(c, ACCENTED)) {
                continue;
            }
            let len = r.text.len();
            let latin = r.text.iter().filter(|c| c.is_ascii_lowercase()).count();
            if latin > len / 2 || (len == 2 && latin == 1) {
                if len > 1 {
                    self.accent_latin(&mut r.text);
                }
            } else {
                self.accent_greek(&mut r.text);
            }
        }
    }

    /// 0x10001730: a Latin word's stressed vowel from the ending table
    /// (the n-th vowel from the end, 0x10001210 on the reversed word, at
    /// most 0x3f letters), marked by 0x10001400 with the Latin-1 acute
    /// (0x100012e0).
    fn accent_latin(&self, t: &mut [u8]) {
        if t.len() > 0x3f {
            return;
        }
        let rev: Vec<u8> = t.iter().rev().copied().collect();
        let mut n = common::lookup(&self.pe, LATIN_ENDINGS, &rev) as i16 as i32;
        if n <= 0 {
            return;
        }
        // 0x10001400: from the last letter back, the n-th vowel
        let mut pos = 0usize;
        let mut k = t.len() as i32 - 1;
        while k >= 0 && n > 0 {
            if self.has(t[k as usize], VOWEL) {
                n -= 1;
                pos = k as usize;
            }
            k -= 1;
        }
        t[pos] = latin_acute(t[pos]);
    }

    /// 0x10001520: the six words with their accented form; a doubled vowel
    /// (not `αι ει οι υι` nor `ου`) takes it on its second letter; else by
    /// vowel groups (0x10001460: `αι ει οι υι αυ ευ ου` count once): of
    /// one or two groups the first, of three the second, of more the
    /// third from the end, on the group's last letter (0x10001250).
    fn accent_greek(&self, t: &mut [u8]) {
        for (from, to) in ACCENTED_WORDS {
            if t == self.s(from) {
                let to = self.s(to);
                t.copy_from_slice(&to[..t.len()]);
                return;
            }
        }
        let at = |t: &[u8], i: usize| t.get(i).copied().unwrap_or(0);
        for i in 1..t.len() {
            let c = t[i];
            if self.has(c, VOWEL)
                && c == t[i - 1]
                && (!matches!(c, 0xe1 | 0xe5 | 0xef | 0xf5) || at(t, i + 1) != 0xe9)
                && (c != 0xef || at(t, i + 1) != 0xf5)
            {
                t[i] = greek_acute(c);
                return;
            }
        }
        let groups = self.vowel_groups(t, false);
        let mut n = 0i32;
        let mut i = 0usize;
        while i < t.len() {
            let c = t[i];
            if self.has(c, VOWEL) {
                if digraph(c, at(t, i + 1)) {
                    i += 1;
                }
                n += 1;
            }
            if self.has(t[i], VOWEL) {
                let hit = match groups {
                    1 | 2 => true,
                    3 => n == 2,
                    g if g >= 4 => n == g - 2,
                    _ => false,
                };
                if hit {
                    t[i] = greek_acute(t[i]);
                    return;
                }
            }
            i += 1;
        }
    }

    /// 0x10001460: the vowels of a word, `αι ει οι υι αυ ευ ου` counted
    /// once unless `each`.
    pub(super) fn vowel_groups(&self, t: &[u8], each: bool) -> i32 {
        let mut n = 0;
        let mut i = 0;
        while i < t.len() {
            let c = t[i];
            if self.has(c, VOWEL) {
                if !each && digraph(c, t.get(i + 1).copied().unwrap_or(0)) {
                    i += 1;
                }
                n += 1;
            }
            i += 1;
        }
        n
    }

    /// `LesGradiAccentuali` (0x10008740): punctuation (class `2`, `7`)
    /// 0x0f; a word without a grade yet: unstressed (3) when it is a
    /// function word (`FUNCTION_CLASSES`) of at most 5 letters, ends with
    /// an apostrophe or has subclass 0x8f, else stressed (0); its
    /// dictionary rows of the other `gradoaccentuale` (0x10008670: DEA 3,
    /// else 0) are dropped.
    pub(super) fn gradi(&self, w: &mut [Word]) {
        for i in 0..w.len().min(200) {
            let r = &mut w[i];
            let class = r.f[CLASS];
            if class == b'2' || class == b'7' {
                r.f[GRADE] = 0x0f;
                continue;
            }
            if r.f[GRADE] != 0x0e {
                continue;
            }
            let apostrophe = r.text.last() == Some(&b'\'');
            let bound = r.f[BOUND];
            let grade = if (matches!(bound, 2 | 1 | 0 | 6 | 7 | 3 | 0x16) || i != 0) && FUNCTION_CLASSES.contains(&class) && r.text.len() <= 5 {
                3
            } else if apostrophe || r.f[SUB] == 0x8f {
                3
            } else {
                0
            };
            r.f[GRADE] = grade;
            let mut lex = Lex::of(r);
            let mut cur = lex.head();
            while let Some(row) = cur {
                if row_grade(self.col(row, col::GRADOACCENTUALE)) == grade {
                    cur = lex.next();
                } else {
                    lex.delete_current();
                    cur = lex.current();
                }
            }
            lex.store(r);
        }
    }
}

/// `αι ει οι υι` and `αυ ευ ου` (0x10001460, 0x10001520).
fn digraph(c: u8, next: u8) -> bool {
    (matches!(c, 0xe1 | 0xe5 | 0xef | 0xf5) && next == 0xe9) || (matches!(c, 0xe1 | 0xe5 | 0xef) && next == 0xf5)
}

/// 0x10001250: a Greek vowel with its acute (tonos).
pub(super) fn greek_acute(c: u8) -> u8 {
    match c {
        0xe1 => 0xdc,
        0xe5 => 0xdd,
        0xe7 => 0xde,
        0xe9 => 0xdf,
        0xef => 0xfc,
        0xf5 => 0xfd,
        0xf9 => 0xfe,
        0xfa => 0xc0,
        0xfb => 0xe0,
        c => c,
    }
}

/// 0x100012e0: a Latin vowel with the Latin-1 acute (the byte, read in the
/// module's Windows-1253 as a Greek letter).
fn latin_acute(c: u8) -> u8 {
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
        c => c,
    }
}

/// 0x10008670: a row's `gradoaccentuale` as a grade (`DEA` 3, else 0).
fn row_grade(g: Option<&[u8]>) -> u8 {
    match g {
        Some(b"DEA") => 3,
        _ => 0,
    }
}

/// 0x10008e10: `grammaticale` → class (0xc8 for none or another name).
fn grammar_class(g: Option<&[u8]>) -> u8 {
    let Some(g) = g else { return 0xc8 };
    match g {
        b"PRON" => 0x20,
        b"VERBO" => 0x2a,
        b"NONVERBO" => 0x2f,
        b"AVV" => 0x33,
        b"AGG" => 0x29,
        b"SOST" => 0x15,
        b"ARTIC" => 0x18,
        b"ARTPRON" => 0x16,
        b"CONG" => 0x1f,
        b"PREPS" => 0x19,
        b"PREPA" => 0x1e,
        b"PRONCLI" => 0x21,
        b"PRONPC" => 0x28,
        b"PRONR" => 0x22,
        b"VERBOE" => 0x2c,
        b"VERBOA" => 0x2b,
        b"OMOGRAFO" => 0x2d,
        _ => 0xc8,
    }
}

/// 0x100091b0: `specgramm` → subclass.
fn subclass(g: Option<&[u8]>) -> u8 {
    let Some(g) = g else { return 0xc8 };
    match g {
        b"ECONG" => 0x82,
        b"NUMU" => b'y',
        b"INTERR" => b'x',
        _ => 0xc8,
    }
}

/// 0x10009260: `gennum` → information.
fn gender_number(g: Option<&[u8]>) -> u8 {
    let Some(g) = g else { return 9 };
    match g {
        b"FESI" => 0x0d,
        b"MASI" => 0x0b,
        b"FEPL" => 0x0e,
        b"MAPL" => 0x0c,
        b"XPLU" => 0x10,
        b"XSIN" => 0x0f,
        _ => 9,
    }
}

/// `LesSetModalitaFrase` (0x10008920): the sentence type from the first
/// character of its closing punctuation among `.!?:;¶` (0x10022390): `!`
/// 6, `.` 0, `;` `?` 2, or 3 when one of the first four records has
/// subclass `x`, `:` `¶` 5, else 7.
pub(super) fn modalita_frase(w: &[Word]) -> i32 {
    let mut wh = false;
    for (k, r) in w.iter().enumerate().take(200) {
        if r.f[SUB] == b'|' {
            let t = &r.text;
            let mut j = 0;
            while common::at(t, j + 1) != 0 && !b".!?:;\xb6".contains(&common::at(t, j)) {
                j += 1;
            }
            return match common::at(t, j) {
                b'!' => 6,
                b'.' => 0,
                b';' | b'?' => 2 + wh as i32,
                0xb6 | b':' => 5,
                _ => 7,
            };
        }
        if k < 4 && r.f[SUB] == b'x' {
            wh = true;
        }
    }
    0
}

/// 0x10002400's answers for modes 0, 1, 2, 3 (`""` 0x10020108 for none):
/// the switch on the character, its cases testing the mode (`!= 0`, `==
/// 2`, `== 3`); modes above 3 answer as 1.
fn symbol_name_at(c: u8) -> [u32; 4] {
    match c {
        0x21 => [0x10020108, 0x10020b6c, 0x10020b6c, 0x10020b6c],
        0x22 => [0x10020108, 0x10020b94, 0x10020b94, 0x10020b94],
        0x24 => [0x1001f2d0, 0x1001f2d0, 0x1001f2d0, 0x1001f2d0],
        0x25 => [0x10020c5c, 0x10020c5c, 0x10020c5c, 0x10020c5c],
        0x26 => [0x1001f244, 0x1001f244, 0x1001f244, 0x1001f244],
        0x27 => [0x10020108, 0x10020c74, 0x10020c74, 0x10020c74],
        0x28 => [0x10020108, 0x10020c30, 0x10020c30, 0x10020c30],
        0x29 => [0x10020108, 0x10020c1c, 0x10020c1c, 0x10020c1c],
        0x2a => [0x10020108, 0x10020c44, 0x10020c44, 0x10020c44],
        0x2b => [0x1001f258, 0x1001f258, 0x1001f258, 0x1001f258],
        0x2c => [0x10020108, 0x10020c68, 0x10020c68, 0x10020c68],
        0x2d => [0x10020108, 0x1001f264, 0x1001f264, 0x1001f264],
        0x2e => [0x10020108, 0x1001f27c, 0x1001f27c, 0x1001f27c],
        0x2f => [0x1001f26c, 0x1001f26c, 0x1001f26c, 0x1001f26c],
        0x3a => [0x10020108, 0x10020b84, 0x10020b84, 0x10020b84],
        0x3b => [0x10020108, 0x10020b78, 0x10020b78, 0x10020b78],
        0x3c => [0x10020108, 0x10020b54, 0x10020b54, 0x10020b54],
        0x3d => [0x10020c08, 0x10020c08, 0x10020c08, 0x10020c08],
        0x3e => [0x10020108, 0x10020b60, 0x10020b60, 0x10020b60],
        0x40 => [0x10020108, 0x10020c80, 0x10020c80, 0x10020c80],
        0x41 => [0x1001f0c0, 0x1001f0c0, 0x1001f0c0, 0x10020b40],
        0x42 => [0x10020b30, 0x10020b30, 0x10020b30, 0x10020b20],
        0x43 => [0x10020b10, 0x10020b10, 0x10020b10, 0x10020b04],
        0x44 => [0x10020af4, 0x10020af4, 0x10020af4, 0x10020ae4],
        0x45 => [0x10020ad8, 0x10020ad8, 0x10020ad8, 0x10020acc],
        0x46 => [0x10020abc, 0x10020abc, 0x10020abc, 0x10020ab0],
        0x47 => [0x10020aa0, 0x10020aa0, 0x10020aa0, 0x10020a90],
        0x48 => [0x10020a7c, 0x10020a7c, 0x10020a7c, 0x10020a6c],
        0x49 => [0x10020a5c, 0x10020a5c, 0x10020a5c, 0x10020a50],
        0x4a => [0x10020a3c, 0x10020a3c, 0x10020a3c, 0x10020a2c],
        0x4b => [0x10020a1c, 0x10020a1c, 0x10020a1c, 0x10020a0c],
        0x4c => [0x100209fc, 0x100209fc, 0x100209fc, 0x100209f0],
        0x4d => [0x100209e0, 0x100209e0, 0x100209e0, 0x100209d4],
        0x4e => [0x100209c4, 0x100209c4, 0x100209c4, 0x100209b8],
        0x4f => [0x100209ac, 0x100209ac, 0x100209ac, 0x100209a0],
        0x50 => [0x10020990, 0x10020990, 0x10020990, 0x10020984],
        0x51 => [0x10020970, 0x10020970, 0x10020970, 0x10020960],
        0x52 => [0x10020950, 0x10020950, 0x10020950, 0x10020944],
        0x53 => [0x10020934, 0x10020934, 0x10020934, 0x10020928],
        0x54 => [0x10020918, 0x10020918, 0x10020918, 0x1002090c],
        0x55 => [0x100208f8, 0x100208f8, 0x100208f8, 0x100208e8],
        0x56 => [0x100208d8, 0x100208d8, 0x100208d8, 0x100208cc],
        0x57 => [0x100208ac, 0x100208ac, 0x100208ac, 0x10020894],
        0x58 => [0x10020884, 0x10020884, 0x10020884, 0x10020874],
        0x59 => [0x10020860, 0x10020860, 0x10020860, 0x10020850],
        0x5a => [0x1002083c, 0x1002083c, 0x1002083c, 0x1002082c],
        0x5b => [0x10020108, 0x10020bcc, 0x10020bcc, 0x10020bcc],
        0x5c => [0x10020108, 0x10020ba0, 0x10020ba0, 0x10020ba0],
        0x5d => [0x10020108, 0x10020bb0, 0x10020bb0, 0x10020bb0],
        0x5f => [0x10020108, 0x10020c10, 0x10020c10, 0x10020c10],
        0x61 => [0x1001f0c0, 0x1001f0c0, 0x1001f0c0, 0x10020b4c],
        0x62 => [0x10020b30, 0x10020b30, 0x10020b30, 0x10020b34],
        0x63 => [0x10020b10, 0x10020b10, 0x10020b10, 0x10020b14],
        0x64 => [0x10020af4, 0x10020af4, 0x10020af4, 0x10020af8],
        0x65 => [0x10020ad8, 0x10020ad8, 0x10020ad8, 0x10020adc],
        0x66 => [0x10020abc, 0x10020abc, 0x10020abc, 0x10020ac0],
        0x67 => [0x10020aa0, 0x10020aa0, 0x10020aa0, 0x10020aa4],
        0x68 => [0x10020a7c, 0x10020a7c, 0x10020a7c, 0x10020a84],
        0x69 => [0x10020a5c, 0x10020a5c, 0x10020a5c, 0x10020a60],
        0x6a => [0x10020a3c, 0x10020a3c, 0x10020a3c, 0x10020a44],
        0x6b => [0x10020a1c, 0x10020a1c, 0x10020a1c, 0x10020a20],
        0x6c => [0x100209fc, 0x100209fc, 0x100209fc, 0x10020a00],
        0x6d => [0x100209e0, 0x100209e0, 0x100209e0, 0x100209e4],
        0x6e => [0x100209c4, 0x100209c4, 0x100209c4, 0x100209c8],
        0x6f => [0x100209ac, 0x100209ac, 0x100209ac, 0x100209b0],
        0x70 => [0x10020990, 0x10020990, 0x10020990, 0x10020994],
        0x71 => [0x10020970, 0x10020970, 0x10020970, 0x10020978],
        0x72 => [0x10020950, 0x10020950, 0x10020950, 0x10020954],
        0x73 => [0x10020934, 0x10020934, 0x10020934, 0x10020938],
        0x74 => [0x10020918, 0x10020918, 0x10020918, 0x1002091c],
        0x75 => [0x100208f8, 0x100208f8, 0x100208f8, 0x10020900],
        0x76 => [0x100208d8, 0x100208d8, 0x100208d8, 0x100208dc],
        0x77 => [0x100208ac, 0x100208ac, 0x100208ac, 0x100208b8],
        0x78 => [0x10020884, 0x10020884, 0x10020884, 0x10020888],
        0x79 => [0x10020860, 0x10020860, 0x10020860, 0x10020868],
        0x7a => [0x1002083c, 0x1002083c, 0x1002083c, 0x10020844],
        0x7b => [0x10020bf8, 0x10020bf8, 0x10020bf8, 0x10020bf8],
        0x7d => [0x10020be8, 0x10020be8, 0x10020be8, 0x10020be8],
        0x80 => [0x1001f2b0, 0x1001f2b0, 0x1001f2b0, 0x1001f2b0],
        0x92 => [0x10020108, 0x10020c88, 0x10020c88, 0x10020c88],
        0x93 => [0x10020108, 0x10020b94, 0x10020b94, 0x10020b94],
        0x94 => [0x10020108, 0x10020b94, 0x10020b94, 0x10020b94],
        0xa2 => [0x10020818, 0x10020818, 0x100207f4, 0x100207cc],
        0xa3 => [0x10020234, 0x10020234, 0x10020234, 0x10020234],
        0xa5 => [0x10020218, 0x10020218, 0x10020218, 0x10020218],
        0xa7 => [0x10020228, 0x10020228, 0x10020228, 0x10020228],
        0xa8 => [0x10020108, 0x10020c50, 0x10020c50, 0x10020c50],
        0xa9 => [0x1002021c, 0x1002021c, 0x1002021c, 0x1002021c],
        0xab => [0x10020108, 0x10020b94, 0x10020b94, 0x10020b94],
        0xb0 => [0x10020108, 0x10020210, 0x10020210, 0x10020210],
        0xb8 => [0x10020748, 0x10020748, 0x10020720, 0x100206f8],
        0xb9 => [0x100206c4, 0x100206c4, 0x100206a4, 0x1002067c],
        0xba => [0x10020644, 0x10020644, 0x100205f0, 0x1002056c],
        0xbb => [0x10020108, 0x10020b94, 0x10020b94, 0x10020b94],
        0xbc => [0x100204b8, 0x100204b8, 0x10020490, 0x10020460],
        0xbd => [0x100201f8, 0x100201f8, 0x100201f8, 0x100201f8],
        0xbe => [0x100203d8, 0x100203d8, 0x10020380, 0x100202f4],
        0xbf => [0x1002028c, 0x1002028c, 0x10020268, 0x10020240],
        0xc0 => [0x10020644, 0x10020644, 0x100205b0, 0x100205cc],
        0xc1 => [0x10020818, 0x10020818, 0x10020818, 0x100207e4],
        0xc2 => [0x100207b8, 0x100207b8, 0x100207b8, 0x100207a8],
        0xc3 => [0x10020794, 0x10020794, 0x10020794, 0x10020784],
        0xc4 => [0x10020770, 0x10020770, 0x10020770, 0x10020760],
        0xc5 => [0x10020748, 0x10020748, 0x10020748, 0x10020710],
        0xc6 => [0x100206e4, 0x100206e4, 0x100206e4, 0x100206d4],
        0xc7 => [0x100206c4, 0x100206c4, 0x100206c4, 0x10020694],
        0xc8 => [0x10020668, 0x10020668, 0x10020668, 0x10020658],
        0xc9 => [0x10020644, 0x10020644, 0x10020644, 0x100205a0],
        0xca => [0x10020558, 0x10020558, 0x10020558, 0x10020548],
        0xcb => [0x10020534, 0x10020534, 0x10020534, 0x10020524],
        0xcc => [0x10020514, 0x10020514, 0x10020514, 0x10020508],
        0xcd => [0x100204f8, 0x100204f8, 0x100204f8, 0x100204ec],
        0xce => [0x100204dc, 0x100204dc, 0x100204dc, 0x100204d0],
        0xcf => [0x100204b8, 0x100204b8, 0x100204b8, 0x1002047c],
        0xd0 => [0x10020990, 0x10020990, 0x10020990, 0x10020984],
        0xd1 => [0x10020450, 0x10020450, 0x10020450, 0x10020444],
        0xd3 => [0x10020430, 0x10020430, 0x10020430, 0x10020410],
        0xd4 => [0x10020400, 0x10020400, 0x10020400, 0x100203f0],
        0xd5 => [0x100203d8, 0x100203d8, 0x100203d8, 0x1002032c],
        0xd6 => [0x100202e4, 0x100202e4, 0x100202e4, 0x100202d8],
        0xd7 => [0x100202c8, 0x100202c8, 0x100202c8, 0x100202bc],
        0xd8 => [0x100202ac, 0x100202ac, 0x100202ac, 0x100202a0],
        0xd9 => [0x1002028c, 0x1002028c, 0x1002028c, 0x10020258],
        0xda => [0x10020644, 0x10020644, 0x10020614, 0x10020584],
        0xdb => [0x100203d8, 0x100203d8, 0x100203a8, 0x1002030c],
        0xdc => [0x10020818, 0x10020818, 0x100207f4, 0x10020804],
        0xdd => [0x10020748, 0x10020748, 0x10020720, 0x10020730],
        0xde => [0x100206c4, 0x100206c4, 0x100206a4, 0x100206b0],
        0xdf => [0x10020644, 0x10020644, 0x100205f0, 0x10020600],
        0xe0 => [0x100203d8, 0x100203d8, 0x1002033c, 0x1002035c],
        0xe1 => [0x10020818, 0x10020818, 0x10020818, 0x10020820],
        0xe2 => [0x100207b8, 0x100207b8, 0x100207b8, 0x100207c0],
        0xe3 => [0x10020794, 0x10020794, 0x10020794, 0x1002079c],
        0xe4 => [0x10020770, 0x10020770, 0x10020770, 0x10020778],
        0xe5 => [0x10020748, 0x10020748, 0x10020748, 0x10020750],
        0xe6 => [0x100206e4, 0x100206e4, 0x100206e4, 0x100206ec],
        0xe7 => [0x100206c4, 0x100206c4, 0x100206c4, 0x100206c8],
        0xe8 => [0x10020668, 0x10020668, 0x10020668, 0x10020670],
        0xe9 => [0x10020644, 0x10020644, 0x10020644, 0x1002064c],
        0xea => [0x10020558, 0x10020558, 0x10020558, 0x10020560],
        0xeb => [0x10020534, 0x10020534, 0x10020534, 0x1002053c],
        0xec => [0x10020514, 0x10020514, 0x10020514, 0x10020518],
        0xed => [0x100204f8, 0x100204f8, 0x100204f8, 0x100204fc],
        0xee => [0x100204dc, 0x100204dc, 0x100204dc, 0x100204e0],
        0xef => [0x100204b8, 0x100204b8, 0x100204b8, 0x100204c0],
        0xf0 => [0x10020990, 0x10020990, 0x10020990, 0x10020994],
        0xf1 => [0x10020450, 0x10020450, 0x10020450, 0x10020454],
        0xf2 => [0x10020430, 0x10020430, 0x10020430, 0x10020420],
        0xf3 => [0x10020430, 0x10020430, 0x10020430, 0x10020438],
        0xf4 => [0x10020400, 0x10020400, 0x10020400, 0x10020404],
        0xf5 => [0x100203d8, 0x100203d8, 0x100203d8, 0x100203e0],
        0xf6 => [0x100202e4, 0x100202e4, 0x100202e4, 0x100202e8],
        0xf7 => [0x100202c8, 0x100202c8, 0x100202c8, 0x100202cc],
        0xf8 => [0x100202ac, 0x100202ac, 0x100202ac, 0x100202b0],
        0xf9 => [0x1002028c, 0x1002028c, 0x1002028c, 0x10020294],
        0xfa => [0x10020644, 0x10020644, 0x10020614, 0x10020628],
        0xfb => [0x100203d8, 0x100203d8, 0x100203a8, 0x100203bc],
        0xfc => [0x100204b8, 0x100204b8, 0x10020490, 0x100204a0],
        0xfd => [0x100203d8, 0x100203d8, 0x10020380, 0x10020390],
        0xfe => [0x1002028c, 0x1002028c, 0x10020268, 0x10020278],
        _ => [0x10020108; 4],
    }
}
