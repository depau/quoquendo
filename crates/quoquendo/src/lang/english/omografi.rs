//! LesGradiAccentuali's homographs: a word whose database rows disagree on
//! the stress grade settled by its neighbours (EnglishUs 0x1000ce10,
//! EnglishGb and EnglishAu 0x1000c830).
//!
//! Both pick one of the grades `ACC` (0, 0x1002b830), `ACD` (2,
//! 0x1002b834) or `DEA` (3, 0x1002b838) from the word's text, its class,
//! subclass and boundary and those of its neighbours, then keep only the
//! rows of that grade (0x1000c4a0): FonTrasParola later reads the first
//! row left. The texts are compared with plain `strcmp` (case-sensitive).

use super::{stricmp, English, Rows, FONETICA, GRADO};
use crate::lang::common::{field::*, Word};

/// `ACC` (0x1002b830), `ACD` (0x1002b834), `DEA` (0x1002b838) and the
/// grade each one sets.
const ACC: (&[u8], u8) = (b"ACC", 0);
const ACD: (&[u8], u8) = (b"ACD", 2);
const DEA: (&[u8], u8) = (b"DEA", 3);

/// A boundary that closes a phrase (9..=0x13, tested inline throughout).
fn closes(b: u8) -> bool {
    (9..=0x13).contains(&b)
}

/// 0x1000c4a0: keep only the rows whose `gradoaccentuale` is `grade`
/// (ELQstricmp). Deleting a row makes the one before it current; deleting
/// the head leaves no current row, which ends the scan (as in C). The
/// grade is set when a row is left, 0x0e otherwise.
fn keep(en: &English, r: &mut Word, (grade, g): (&[u8], u8)) {
    let mut cur = Rows::head(r);
    while let Some(row) = cur {
        if stricmp(en.cell(row, GRADO).unwrap_or(b""), grade).is_eq() {
            cur = Rows::next(r);
        } else {
            Rows::delete_current(r);
            cur = Rows::current(r);
        }
    }
    r.f[GRADE] = if Rows::head(r).is_some() { g } else { 0x0e };
}

fn bound(w: &[Word], i: usize) -> u8 {
    w.get(i).map_or(0, |r| r.f[BOUND])
}

/// 0x1000c660: from word `u` on, up to a boundary 0x17 (or 200 words, or
/// the sentence's end), some word has boundary 0x11 or 0x12.
fn clause_ahead(w: &[Word], u: usize) -> bool {
    let mut i = u;
    while i < w.len() && i < 200 && w[i].f[BOUND] != 0x17 {
        if matches!(w[i].f[BOUND], 0x11 | 0x12) {
            return true;
        }
        i += 1;
    }
    false
}

/// 0x1000c6a0: word `u` opens a phrase (first word, or the word before it
/// closes one).
fn opens(w: &[Word], u: usize) -> bool {
    u == 0 || closes(bound(w, u - 1))
}

/// 0x1000c6e0: one of the first `n` words closes a phrase.
fn closes_within(w: &[Word], n: usize) -> bool {
    (0..n).any(|i| closes(bound(w, i)))
}

/// 0x1000c5e0: 0x1000c6a0 and 0x1000c660.
fn opens_clause(w: &[Word], u: usize) -> bool {
    opens(w, u) && clause_ahead(w, u)
}

/// 0x1000c730: no word of the phrase before `u` is stressed (grade 0 or a
/// row whose `fonetica` has a stress mark '`').
fn none_stressed_before(en: &English, w: &mut [Word], u: usize) -> bool {
    for i in (0..u).rev() {
        if closes(w[i].f[BOUND]) {
            return true;
        }
        if w[i].f[GRADE] == 0 {
            return false;
        }
        let mut cur = Rows::head(&mut w[i]);
        while let Some(row) = cur {
            if en.cell(row, FONETICA).is_some_and(|f| f.contains(&b'`')) {
                return false;
            }
            cur = Rows::next(&mut w[i]);
        }
    }
    true
}

fn text(w: &[Word], i: usize) -> Option<&[u8]> {
    w.get(i).map(|r| r.text.as_slice())
}

fn is(t: Option<&[u8]>, words: &[&[u8]]) -> bool {
    t.is_some_and(|t| words.contains(&t))
}

/// 0x1000ce10 (EnglishUs), word `u` of the sentence. Strings at
/// 0x1002b840..0x1002b8d8, 0x1002a7fc `can`, 0x1002a560 `at`, 0x100272b8
/// `and`, 0x10027a3c `what`, 0x1002b3f8 `where`, 0x1002b380 `so`,
/// 0x1002b3c0 `for`.
pub(super) fn us(en: &English, w: &mut [Word], u: usize) {
    let g = us_grade(en, w, u);
    keep(en, &mut w[u], g);
}

fn us_grade(en: &English, w: &mut [Word], u: usize) -> (&'static [u8], u8) {
    let b = w[u].f[BOUND];
    if closes(b) && none_stressed_before(en, w, u) {
        return ACC;
    }
    let t = w[u].text.as_slice();
    let prev = if u > 0 { text(w, u - 1) } else { None };
    let next = text(w, u + 1);
    let class = w[u].f[CLASS];
    if class == b'+' {
        let nc = w.get(u + 1).map_or(0, |r| r.f[CLASS]);
        if !matches!(nc, b'*' | 0xe5 | b'+' | b',') || next == Some(b"to") {
            return ACD;
        }
    }
    if t == b"have" && opens(w, u) && clause_ahead(w, u) {
        return ACD;
    }
    // 0x1000cf6d: `are`; every other path with u > 0 goes on to 0x1000d003.
    if t == b"are" && clause_ahead(w, u) && (opens(w, u) || (u == 1 && closes_within(w, 4))) {
        return ACD;
    }
    if u > 0 && is(prev, &[b"shall", b"should", b"can", b"could", b"will", b"might"]) && t == b"do" {
        return ACC;
    }
    if t == b"do" && is(next, &[b"it", b"be", b"with", b"so"]) {
        return ACC;
    }
    if (t == b"can" || t == b"do") && closes(b) {
        return ACC;
    }
    if t == b"do" && !clause_ahead(w, u) {
        return ACD;
    }
    if is(Some(t), &[b"can", b"what", b"where", b"what's", b"where's"]) && opens_clause(w, u) {
        return ACD;
    }
    if t == b"shall" && next == Some(b"not") {
        return ACD;
    }
    if matches!(w[u].f[SUB], 0x83 | 0x84) {
        return ACD;
    }
    if opens(w, u) && is(Some(t), &[b"and", b"as", b"or", b"for", b"at", b"some"]) {
        return ACD;
    }
    let quiet = matches!(class, b' ' | b'#' | b'"');
    if closes(b) {
        if is(Some(t), &[b"was", b"up", b"on", b"off", b"did", b"her", b"me"]) {
            return ACC;
        }
        if u > 0 && quiet && prev == Some(b"to") {
            return ACC;
        }
        if !quiet {
            return ACD;
        }
    }
    DEA
}

/// 0x1000c830 (EnglishGb, EnglishAu), word `u` of the sentence. Strings at
/// 0x1002b83c..0x1002b870, 0x10029ab4 `of`, 0x100272b8 `and`, 0x1002a560
/// `at`, 0x1002a7fc `can`.
pub(super) fn gb(en: &English, w: &mut [Word], u: usize) {
    let g = gb_grade(en, w, u);
    keep(en, &mut w[u], g);
}

fn gb_grade(en: &English, w: &mut [Word], u: usize) -> (&'static [u8], u8) {
    let b = w[u].f[BOUND];
    if (closes(b) && none_stressed_before(en, w, u)) || matches!(w[u].f[SUB], 0x83 | 0x84) {
        return ACC;
    }
    let t = w[u].text.as_slice();
    if closes(b) {
        if t == b"are" || t == b"will" {
            return ACC;
        }
        if !matches!(w[u].f[CLASS], b' ' | b'#' | b'"') {
            return ACD;
        }
    }
    if opens(w, u) && is(Some(t), &[b"of", b"and", b"as", b"at", b"are"]) {
        return ACD;
    }
    if u > 0 && t == b"on" && text(w, u - 1) == Some(b"straight") {
        return ACD;
    }
    if is(Some(t), &[b"have", b"has", b"had"]) {
        let nc = w.get(u + 1).map_or(0, |r| r.f[CLASS]);
        if matches!(nc, b',' | b'+' | b'*' | b'3' | 0xe5) {
            return DEA;
        }
        return if text(w, u + 1) == Some(b"not") { DEA } else { ACD };
    }
    if t == b"are" && clause_ahead(w, u) && (opens(w, u) || (u == 1 && closes_within(w, 4))) {
        return ACD;
    }
    if (t == b"can" || t == b"do") && opens_clause(w, u) {
        return ACD;
    }
    DEA
}
