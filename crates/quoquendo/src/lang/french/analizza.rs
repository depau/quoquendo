//! `LesAnalizza` (0x1000e2c0): the dictionary rows and classes of a
//! sentence's words, 0x1000dfe0 then 0x1000d760.

use super::{col, fld, French, Lex};
use crate::lang::common::{field::*, Word};

/// The ending table 0x1000c160 searches (0x1003be58, 0x39c entries `{
/// reversed ending, exception list, code }`, the layout of
/// `common::Endings`).
const FORMS: crate::lang::common::Endings = crate::lang::common::Endings { at: 0x1003be58, count: 0x39c };

/// `grammaticale` → class (0x1000c2a0): the strings it compares, by
/// address, and the class each gives; anything else (and NULL) 0xc8.
const GRAM: &[(u32, u8)] = &[
    (0x1003ec84, 0x20), (0x1003ec7c, 0x20), (0x1003ec74, 0x20), (0x1003ec6c, 0x2a), (0x1003ec60, 0xd8), (0x1003ec58, 0x2c),
    (0x1003ec50, 0x2b), (0x1003ec44, 0x2f), (0x1003ec40, 0x33), (0x1003ec3c, 0x29), (0x1003ec30, 0x29), (0x1003ec28, 0x15),
    (0x1003ec1c, 0x15), (0x1003ec14, 0x18), (0x1003ec10, 0x18), (0x1003ec08, 0x1f), (0x1003ec00, 0x19), (0x1003ebf8, 0x19),
    (0x1003ebf0, 0x1e), (0x1003ebec, 0xd2), (0x1003ebe0, 0xe3), (0x1003ebd4, 0xe5), (0x1003ebcc, 0xe4), (0x1003ebc4, 0x23),
    (0x1003ebbc, 0x22), (0x1003ebb4, 0x82), (0x1003ebac, 0x82), (0x1003eba4, 0xd3), (0x1003eba0, 0xd3), (0x1003eb98, 0xd3),
    (0x1003eb90, 0x84), (0x1003eb88, 0xd4), (0x1003eb7c, 0x2e), (0x1003eb74, 0xd9), (0x1003eb6c, 0xda), (0x1003eb64, 0xe0),
    (0x1003eb58, 0xe1), (0x1003eb4c, 0xe2), (0x1003eb44, 0xdb), (0x1003eb3c, 0xdc), (0x1003eb30, 0xde), (0x1003eb24, 0xdd),
    (0x1003eb1c, 0x29), (0x1003eb10, 0xe7), (0x1003eb04, 0xe8), (0x1003eaf8, 0xe9), (0x1003eaec, 0xea), (0x1003ead4, 0xea),
    (0x1003eac0, 0xdb), (0x1003eab0, 0xdb), (0x1003ea9c, 0xea), (0x1003ea88, 0xee), (0x1003ea74, 0xee), (0x1003ea64, 0xea),
    (0x1003ea54, 0xea), (0x1003ea38, 0xea), (0x1003ea2c, 0x18), (0x1003ea20, 0x1e), (0x1003ea14, 0x19), (0x1003ea08, 0x1e),
    (0x1003e9f4, 0x18), (0x1003e9e0, 0x22), (0x1003e9c4, 0x22), (0x1003e9a8, 0x22),
];

impl French {
    /// Look a code mapper's string up: (address, value) pairs, a default.
    fn map(&self, s: Option<&[u8]>, pairs: &[(u32, u8)], default: u8, null: u8) -> u8 {
        let Some(s) = s else { return null };
        pairs.iter().find(|&&(a, _)| self.s(a) == s).map_or(default, |&(_, v)| v)
    }

    /// 0x1000c2a0
    pub(crate) fn gram_class(&self, row: u32) -> u8 {
        self.map(self.col(row, col::GRAMMATICALE), GRAM, 0xc8, 0xc8)
    }

    /// 0x1000d360
    fn tempo_code(&self, row: u32) -> u8 {
        self.map(self.col(row, col::TEMPOMODO), TEMPO, 0x09, 0xc8)
    }

    /// 0x1000d030
    fn gennum_code(&self, row: u32) -> u8 {
        self.map(self.col(row, col::GENNUM), GENNUM, 0x09, 0xc8)
    }

    /// 0x1000d520
    fn grade_code(&self, row: u32) -> u8 {
        self.map(self.col(row, col::GRADO), GRADE, 0x0e, 0x0e)
    }

    /// Class, subclass, information and grade from a row (0x1000e1fd…).
    fn from_row(&self, r: &mut Word, row: u32) {
        r.f[CLASS] = self.gram_class(row);
        r.f[SUB] = self.tempo_code(row);
        r.f[INFO] = self.gennum_code(row);
        r.f[0xc] = self.grade_code(row);
        r.f[0xd] = 0;
    }

    pub(crate) fn analizza(&self, w: &mut [Word]) {
        self.analizza_dict(w);
        self.analizza_context(w);
    }

    /// 0x1000dfe0: the dictionary's rows for every word (also without a
    /// final `s` for an unknown word), and the class they give.
    fn analizza_dict(&self, w: &mut [Word]) {
        // the count of the last lookup without the `s`, kept across words
        let mut stripped = 0usize;
        for k in 0..w.len() {
            let (sub, class) = (w[k].f[SUB], w[k].f[CLASS]);
            if matches!(sub, b'z' | b'{' | b'y') || class == b'2' {
                continue;
            }
            let mut lex = Lex::of(&w[k]);
            let text = w[k].text.clone();
            let n = self.lookup(&mut lex, &text);
            let found = n > 0 && self.has_col(&mut lex, col::GRAMMATICALE);
            let r = &mut w[k];
            if (n == 0 || !found) && class == 0x17 {
                if text.last() == Some(&b's') {
                    stripped = self.lookup(&mut lex, &text[..text.len() - 1]);
                }
                if stripped == 0 {
                    r.f[CLASS] = 0x17;
                    lex.store(r);
                    self.by_form(r);
                    continue;
                } else if stripped == 1 {
                    match lex.current() {
                        None => lex.delete_current(),
                        Some(row) => {
                            let c = self.gram_class(row);
                            if matches!(c, 0x29 | 0x15 | 0xe7 | 0xe8 | 0x2f) {
                                r.f[CLASS] = c;
                                r.f[INFO] = match self.gennum_code(row) {
                                    0x0d => 0x0e,
                                    0x0b => 0x0c,
                                    g => g,
                                };
                                r.f[0xc] = self.grade_code(row);
                                r.f[0xd] = 0;
                            } else {
                                lex.delete_current();
                            }
                        }
                    }
                } else {
                    lex.clear();
                    r.f[INFO] = 0x10;
                    r.f[CLASS] = 0xf0;
                }
            } else if n == 1 {
                if let Some(row) = lex.current() {
                    self.from_row(r, row);
                }
            } else if n > 1 {
                r.f[CLASS] = 0xf0;
            }
            lex.store(r);
        }
    }

    /// 0x1000c1a0: an unknown word's class from its ending (0x1000c160 on
    /// the reversed word): adverb, infinitive, participle, verb, 0x2d.
    fn by_form(&self, r: &mut Word) {
        if r.f[CLASS] != 0x17 {
            return;
        }
        let rev: Vec<u8> = r.text.iter().rev().copied().collect();
        match crate::lang::common::lookup(&self.pe, FORMS, &rev) {
            1 => r.f[CLASS] = 0x33,
            3 => {
                r.f[CLASS] = 0x2a;
                r.f[SUB] = 0x8d;
            }
            4 => {
                r.f[CLASS] = 0x2a;
                r.f[SUB] = 0x88;
            }
            5 => r.f[CLASS] = 0x2a,
            6 => r.f[CLASS] = 0x2d,
            _ => r.f[CLASS] = 0x2f,
        }
    }

    /// 0x1000d760: classes from the neighbours, then the rows that
    /// disagree with the class dropped, gender and number from endings and
    /// articles.
    fn analizza_context(&self, w: &mut [Word]) {
        // the word without its `s` (a stack buffer, kept across words)
        let mut stem: Vec<u8> = Vec::new();
        for k in 0..w.len() {
            let ki = k as isize;
            let len = w[k].text.len();
            if len == 0 {
                return;
            }
            let prev = |w: &[Word], f| fld(w, ki - 1, f);
            let next = |w: &[Word], f| fld(w, ki + 1, f);
            if w[k].text[len - 1] == b's' && w[k].f[CLASS] == 0xf0 && w[k].f[INFO] == 0x10 {
                stem = w[k].text[..len - 1].to_vec();
            }
            if w[k].f[CLASS] == 0xf0 {
                w[k].f[CLASS] = 0x2d;
            }
            let t = w[k].text.clone();
            let ends = |s: &[u8]| t.ends_with(s);
            let mut lex = Lex::of(&w[k]);
            let mut count = lex.rows.len();
            match w[k].f[CLASS] {
                0x18 => {
                    if t == self.s(0x1001c3b4) && k > 1 && fld(w, ki - 2, CLASS) == 0x82 && prev(w, CLASS) == 0x37 {
                        w[k].f[CLASS] = 0xd3;
                        w[k].f[INFO] = 9;
                    }
                }
                0x19 => {
                    if t == self.s(0x1001f270) && matches!(next(w, CLASS), 0x2a | 0x2c | 0x2b | 0xd8) {
                        w[k].f[CLASS] = 0x20;
                    }
                }
                0x2a => {
                    if w[k].f[SUB] == 0x88 && k != 0 && matches!(prev(w, CLASS), 0x18 | 0x29) {
                        w[k].f[CLASS] = 0x2f;
                        w[k].f[SUB] = 200;
                        w[k].f[INFO] = 200;
                    }
                }
                0x2d => {
                    if t == self.s(0x1001fdd4) {
                        // `t` (as in a-t-il)
                        w[k].f[INFO] = 9;
                        let c = if next(w, CLASS) == 0x37 { 0xd2 } else { 0x84 };
                        w[k].f[CLASS] = c;
                        w[k].f[SUB] = c;
                    } else if t == self.s(0x1001f940) || t == self.s(0x1001f93c) {
                        // que, qu'
                        if k < 2 || !matches!(prev(w, CLASS), 0x2a | 0x2c | 0x2b | 0xd8 | 0xe1 | 0xe0 | 0xe9 | 0x33) {
                            w[k].f[CLASS] = 0x22;
                            w[k].f[SUB] = 9;
                            w[k].f[INFO] = 0xc9;
                        } else {
                            w[k].f[CLASS] = 0x1f;
                            w[k].f[SUB] = 9;
                            w[k].f[INFO] = 9;
                        }
                    } else if t == self.s(0x1001f9ac) {
                        // tous
                        w[k].f[CLASS] =
                            if matches!(next(w, CLASS), 0x18 | 0xdc | 0xe4 | 0xdb | 0x15 | 0xd9) { 0xde } else { 0x20 };
                    } else {
                        let ent = len >= 4 && ends(b"ent");
                        let pc = prev(w, CLASS);
                        if k != 0 && matches!(pc, 0x22 | 0x20) {
                            // after a pronoun: a verb
                            w[k].f[CLASS] = 0x2a;
                            w[k].f[SUB] = 200;
                            w[k].f[INFO] = if ent { 0xd1 } else { 200 };
                        } else if ent && k >= 2 && pc == 0x18 && fld(w, ki - 2, CLASS) != 0x2f {
                            w[k].f[CLASS] = 0x2f;
                            w[k].f[SUB] = 200;
                            w[k].f[INFO] = 0xb;
                        } else if len >= 3 && t[len - 1] == b's' && k != 0 && matches!(pc, 0x18 | 0x1e) {
                            // a plural noun after an article
                            w[k].f[CLASS] = 0x2f;
                            w[k].f[SUB] = 200;
                            if w[k].f[INFO] == 0x10 {
                                count = self.lookup(&mut lex, &stem);
                                lex.head();
                            } else {
                                w[k].f[INFO] = 200;
                            }
                        } else if (len < 3 || t[len - 1] != b's' || k != 0)
                            && k != 0
                            && next(w, CLASS) == 0x2a
                            && next(w, INFO) == 0x8d
                        {
                            // before an infinitive
                            w[k].f[CLASS] = 0x2a;
                            w[k].f[SUB] = 200;
                            w[k].f[INFO] = 200;
                        }
                    }
                }
                0xd3 => w[k].f[SUB] = if matches!(next(w, SUB), b'z' | b'y') { 0x7a } else { 0x79 },
                0xd4 => w[k].f[SUB] = if matches!(next(w, SUB), 0x83 | 0x84) { 0x83 } else { 0x84 },
                0xee => {
                    w[k].f[CLASS] = if matches!(next(w, CLASS), 0x2f | 0x15 | 0xd9 | 0xe7 | 0x29) { 0xde } else { 0xe5 };
                }
                _ => {}
            }
            // the rows whose class differs go; the last row looked at gives
            // the fields when it agrees
            let class = w[k].f[CLASS];
            if let Some(mut row) = lex.head() {
                if count > 1 && class != 0x2d {
                    loop {
                        let ok = if self.gram_class(row) == class {
                            lex.next()
                        } else {
                            lex.delete_current();
                            lex.current()
                        };
                        match ok {
                            Some(r) => row = r,
                            None => break,
                        }
                    }
                    let c = self.gram_class(row);
                    if c == class || (class == 0x2f && matches!(c, 0x15 | 0x29)) {
                        self.from_row(&mut w[k], row);
                    }
                }
            }
            lex.store(&mut w[k]);
            let r = &mut w[k];
            if matches!(r.f[CLASS], 0x15 | 0x29 | 0x2f) {
                if matches!(r.f[INFO], 0xc8 | 0x10) {
                    if len >= 3 && ends(b"\xe9e") {
                        r.f[INFO] = 0xd;
                    } else if len >= 4 && ends(b"\xe9es") {
                        r.f[INFO] = 0xe;
                    } else if len >= 4 && (ends(b"eux") || ends(b"ais")) {
                        r.f[INFO] = 0xcb;
                    } else if len >= 4 && ends(b"aux") {
                        r.f[INFO] = 0xc;
                    } else if len > 1 && t[len - 1] == b's' {
                        r.f[INFO] = 0x10;
                    }
                }
                let i = r.f[INFO];
                if k != 0 && matches!(fld(w, ki - 1, CLASS), 0x18 | 0x1e) && !matches!(i, 0xb | 0xc | 0xd | 0xe) {
                    let p = fld(w, ki - 1, INFO);
                    let r = &mut w[k];
                    if matches!(p, 0xb | 0xd | 0xe | 0xc) {
                        r.f[INFO] = p;
                    } else if p == 0x10 {
                        r.f[INFO] = match i {
                            0xca => 0xe,
                            0xcb => 0xc,
                            _ => 0x10,
                        };
                    }
                }
            }
        }
    }
}

/// `tempomodo` → subclass (0x1000d360); else 0x09, NULL 0xc8.
const TEMPO: &[(u32, u8)] = &[
    (0x1003ed48, 0x88), (0x1003ed3c, 0x88), (0x1003ed30, 0x8e), (0x1003ed24, 0x8d), (0x1003ed18, 0x8b), (0x1003ed10, 0xd5),
    (0x1003ed04, 0xd6), (0x1003ecf8, 0xd7),
];

/// `gennum` → information (0x1000d030); else 0x09, NULL 0xc8.
const GENNUM: &[(u32, u8)] = &[
    (0x1003ecf0, 0x0d), (0x1003ece8, 0x0b), (0x1003ece0, 0x0e), (0x1003ecd8, 0x0c), (0x1003ecd0, 0x10), (0x1003ecc8, 0xcf),
    (0x1003ecc0, 0xd0), (0x1003ecb8, 0xd1), (0x1003ecb0, 0x0f), (0x1003eca8, 0xcc), (0x1003eca0, 0xcd), (0x1003ec98, 0xce),
    (0x1003ec94, 0xc9), (0x1003ec90, 0xca), (0x1003ec8c, 0xcb),
];

/// `gradoaccentuale` → grade (0x1000d520); else (and NULL) 0x0e.
const GRADE: &[(u32, u8)] = &[
    (0x1003ed6c, 0x00), (0x1003ed68, 0x01), (0x1003ed64, 0x02), (0x1003ed60, 0x03), (0x1003ed5c, 0x05), (0x1003ed58, 0x09),
    (0x1003ed54, 0x08),
];
