//! LesAnalizza's context rules (0x100103c0): every record's class, in
//! order, resolved from its neighbours; a record that had a `DB_…` class
//! keeps only the lexicon rows compatible with its new class.

use super::{class_of, English, Rows, GRAMMATICALE};
use crate::lang::common::field::{CLASS, SUB};
use crate::lang::common::Word;

/// 0x1000f320: an adverb in `-ely` (class 0x2d, more than 3 letters
/// ending in "ely", 0x10042150), or a record already 0x33.
fn ely(r: &Word) -> bool {
    r.f[CLASS] == 0x33 || (r.f[CLASS] == 0x2d && r.text.len() > 3 && r.text.ends_with(b"ely"))
}

/// 0x10010210: a participle-looking word ("…ed" longer than 2 letters,
/// "…ing" longer than 3).
fn participle(t: &[u8]) -> bool {
    (t.len() > 2 && t.ends_with(b"ed")) || (t.len() > 3 && t.ends_with(b"ing"))
}

/// 0x100101a0: a mixed class to its first member.
fn first_member(c: u8) -> u8 {
    match c {
        0xce | 0xe3 => 0x15,
        0xcf..=0xd4 | 0xe6 | 0xe7 => 0x19,
        0xd5 | 0xd6 | 0xd8..=0xde => 0x29,
        0xdf | 0xe0 => 0x33,
        0xe1 => 0x1f,
        0xe2 => 0x20,
        _ => 0x2d,
    }
}

/// 0x10010260: a row of class `a` is compatible with the record's class
/// `b`: equal, or both in one of three groups (prepositional, adverbial,
/// verbal).
fn compatible(a: u8, b: u8) -> bool {
    const PREP: &[u8] = &[0xe7, 0xd2, 0xd0, 0xd1, 0xcf, 0xd3, 0xd4, 0xe6, 0x1e, 0x19];
    const AVV: &[u8] = &[0xdb, 0x33, 0xdf, 0xe0, 0xd9, 0xde, 0xdc, 0xdd, 0xd5];
    const VERBO: &[u8] = &[0xd1, 0x2a, 0x2b, 0xe0, 0x2c, 0xe5, 0xde, 0xcd, 0xdd, 0xcc];
    a == b || [PREP, AVV, VERBO].iter().any(|g| g.contains(&a) && g.contains(&b))
}

/// The mixed classes the default case resolves (0x10011711).
fn mixed(c: u8) -> bool {
    matches!(c, 0xcb..=0xd6 | 0xd8..=0xe4 | 0xe6 | 0xe7 | 0x2d)
}

/// The previous record, skipping 0x33 ones (never below index 0).
fn before(w: &[Word], i: usize) -> usize {
    let mut j = i - 1;
    while j > 0 && w[j].f[CLASS] == 0x33 {
        j -= 1;
    }
    j
}

fn is_in(t: &[u8], set: &[&str]) -> bool {
    set.iter().any(|s| s.as_bytes() == t)
}

/// Subjects and auxiliaries before a verb (0x1001091a chain).
const BEFORE_VERB: &[&str] = &[
    "to", "i", "you", "he", "she", "we", "they", "it", "will", "won't", "can", "can't", "cannot", "do", "don't", "does",
    "doesn't", "didn't", "might", "must", "mustn't", "could", "couldn't", "should", "shouldn't", "would", "wouldn't",
    "which", "who",
];

/// Auxiliaries before a past participle (0x10010f75 chain).
const BEFORE_PARTICIPLE: &[&str] = &[
    "he", "she", "it", "be", "is", "isn't", "are", "aren't", "was", "wasn't", "were", "weren't", "has", "hasn't", "had",
    "hadn't", "have", "haven't", "been", "being", "having",
];

pub(super) fn run(en: &English, w: &mut [Word]) {
    let n = w.len();
    for i in 0..n {
        let c0 = w[i].f[CLASS];
        let db = (0xe8..=0xed).contains(&c0);
        let mut keep = true;
        if c0 == 0x2d && ely(&w[i]) {
            w[i].f[CLASS] = 0x33;
            w[i].f[SUB] = 0xc8;
        }
        let next_sub = w.get(i + 1).map(|r| r.f[SUB]);
        let next_class = w.get(i + 1).map(|r| r.f[CLASS]);
        let c = w[i].f[CLASS];
        match c {
            0x7a => {
                w[i].f[CLASS] = 0x15;
                w[i].f[SUB] = if next_sub == Some(0x7a) { 0x7a } else { 0x79 };
            }
            0x83 => {
                w[i].f[CLASS] = 0x15;
                w[i].f[SUB] = if next_sub == Some(0x83) { 0x83 } else { 0x84 };
            }
            // 0x18, 0x2b, 0x2c (0x100104c4, 0x10010677) test the record's own
            // class against the mixed classes and so never act.
            0x18 | 0x2b | 0x2c => {}
            0xcb | 0xe4 | 0xe8 | 0xea | 0xeb | 0xec => {
                w[i].f[CLASS] = match c {
                    0xea => 0x29,
                    0xec => 0xd6,
                    _ => 0x15,
                };
                if i > 0 {
                    let j = before(w, i);
                    let mut verb = false;
                    if i + 1 < n && w[j].f[CLASS] != 0x29 && is_in(&w[i].text, &["live", "lives"]) {
                        verb = matches!(
                            next_class,
                            Some(0xdb | 0x33 | 0xdf | 0xe0 | 0xd9 | 0xde | 0xdc | 0xdd | 0xd5 | 0xe7 | 0xd2 | 0xd0 | 0xd1 | 0xcf | 0xd3 | 0xd4 | 0xe6 | 0x1e | 0x19)
                        );
                    }
                    if verb || is_in(&w[j].text, BEFORE_VERB) {
                        w[i].f[CLASS] = 0x2a;
                    }
                }
            }
            0x2a | 0xe5 => {
                if i > 0 && is_in(&w[before(w, i)].text, BEFORE_PARTICIPLE) {
                    w[i].f[SUB] = 0x88;
                }
            }
            0xce => {
                w[i].f[CLASS] = 0x15;
                if i > 0 {
                    let one_more = w[i].text == b"minute" && i > 1 && w[i - 2].text == b"one" && w[i - 1].text == b"more";
                    if !one_more && is_in(&w[i - 1].text, &["very", "so", "most", "more", "as"]) {
                        w[i].f[CLASS] = 0x29;
                    }
                }
            }
            0xcc | 0xcd | 0xe9 => {
                w[i].f[CLASS] = 0x2a;
                if i > 0 {
                    let j = before(w, i);
                    if w[i].text == b"used" && i + 1 < n && w[i + 1].text == b"to" && !is_in(&w[j].text, &["be", "is", "was"]) {
                        w[i].f[CLASS] = 0x29;
                    }
                }
            }
            0xed => w[i].f[CLASS] = 0x18,
            0xd9 => w[i].f[CLASS] = 0xef,
            0xd5 => w[i].f[CLASS] = 0xda,
            _ => {
                keep = false;
                if mixed(c) {
                    if db {
                        w[i].f[CLASS] = 0x2d;
                    }
                    let p = if i > 0 { Some(w[i - 1].f[CLASS]) } else { None };
                    w[i].f[CLASS] = if p == Some(0x18) && next_class == Some(0xee) {
                        0xee
                    } else if matches!(p, Some(0x2b | 0x2c | 0xe5)) && participle(&w[i].text) {
                        0x2a
                    } else {
                        first_member(w[i].f[CLASS])
                    };
                }
            }
        }
        if db && keep {
            let r = &mut w[i];
            let mut ok = Rows::head(r).is_some();
            while ok {
                let Some(row) = Rows::current(r) else { break };
                if compatible(class_of(en.cell(row, GRAMMATICALE)), r.f[CLASS]) {
                    ok = Rows::next(r).is_some();
                } else {
                    Rows::delete_current(r);
                    ok = Rows::current(r).is_some();
                }
            }
        }
        if w[i].f[CLASS] == 0xd6 {
            w[i].f[CLASS] = 0x15;
        }
    }
}
