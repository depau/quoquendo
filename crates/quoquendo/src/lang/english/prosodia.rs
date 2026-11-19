//! LesConfiniProsodici (0x1000eb20) and LesGradiAccentuali (0x1000da10).

use super::{grade_of, omografi, English, Rows, Variant, GRADO};
use crate::lang::common::{field::*, Word};

/// LTTS7Util.dll `ELQSearchString` (0x10028ca0) with flag 1 on a table of
/// `n` string pointers: a binary search (strcmp) when `n` > 3, else a
/// linear one; whether `key` is there.
pub(super) fn search_string(en: &English, key: &[u8], table: u32, n: u32) -> bool {
    let entry = |i: u32| en.pe.cstr(en.pe.u32(table + 4 * i));
    if n > 3 {
        let (mut lo, mut hi) = (0i64, n as i64 - 1);
        while lo <= hi {
            let mid = (lo + hi) / 2;
            match key.cmp(entry(mid as u32)) {
                std::cmp::Ordering::Less => hi = mid - 1,
                std::cmp::Ordering::Greater => lo = mid + 1,
                std::cmp::Ordering::Equal => return true,
            }
        }
        false
    } else {
        (0..n).any(|i| key == entry(i))
    }
}

/// Fixed expressions (`on the other hand`, `as soon as`, `account for`…):
/// tables of 4-, 3- and 2-word phrases, (address, entries) each.
type Phrases = [(u32, u32); 3];
/// 0x1000de10: sentence adverbials.
const ADVERBIALS: Phrases = [(0x1002b4d8, 7), (0x1002b528, 0x15), (0x1002b4f4, 0xd)];
/// 0x1000dfe0: conjunctions.
const CONJUNCTIONS: Phrases = [(0x1002b57c, 1), (0x1002b580, 2), (0x1002b588, 1)];
/// 0x1000e1b0: phrasal verbs.
const PHRASAL: Phrases = [(0x1002b58c, 5), (0x1002b5a0, 0x2a), (0x1002b648, 0x6f)];

/// 0x1000de10, 0x1000dfe0, 0x1000e1b0: the longest phrase of `t` that
/// starts at word `u` (the words joined by spaces, at most 4), its length
/// in words.
fn phrase(en: &English, w: &[Word], u: usize, t: &Phrases) -> Option<u16> {
    let n = (w.len() - u).min(4);
    let mut s: Vec<u8> = Vec::new();
    for r in &w[u..u + n] {
        if !s.is_empty() {
            s.push(b' ');
        }
        s.extend_from_slice(&r.text);
    }
    let cut = |s: &mut Vec<u8>| {
        let p = s.iter().rposition(|&c| c == b' ').unwrap_or(0);
        s.truncate(p);
    };
    for (len, &(table, count)) in [4u16, 3, 2].iter().zip(t) {
        if (n as u16) < *len {
            continue;
        }
        if search_string(en, &s, table, count) {
            return Some(*len);
        }
        cut(&mut s);
    }
    None
}

/// Verb classes (VERBO, 0x36, VERBOA, VERBOE, VERBOMOD).
fn verb(c: u8) -> bool {
    matches!(c, 0x2a | 0x36 | 0x2b | 0x2c | 0xe5)
}
/// Participles and the like (0x88, 0x8e, 0x89).
fn participle(s: u8) -> bool {
    matches!(s, 0x88 | 0x8e | 0x89)
}
/// Pronouns, conjunctions and adverbs that open a group.
fn opener(c: u8) -> bool {
    matches!(c, 0x20 | 0x22 | 0x23 | 0x24 | 0x28 | 0x21 | 0x2e | 0x82 | 0x33)
}
/// Content words (0x37, 0xee, SOST 0x15, AGG 0x29).
fn content(c: u8) -> bool {
    matches!(c, 0x37 | 0xee | 0x15 | 0x29)
}

/// 0x1000e3c0: each word's group category for LesConfiniProsodici, (kind,
/// detail), from a small state machine over the classes; 10 after the
/// last word.
fn categories(en: &English, w: &[Word]) -> Vec<(u32, u32)> {
    let n = w.len();
    let mut cat = vec![(5u32, 0u32); n + 1];
    cat[n] = (10, 0);
    let mut state = 0;
    // phrase lengths and how far into each phrase the walk is
    let (mut l24, mut c10, mut l20, mut c0c, mut l1c, mut c08) = (0u16, 0u16, 0u16, 0u16, 0u16, 0u16);
    let mut phrase_end = 0usize;
    let mut last = 0usize;
    let fill = |cat: &mut Vec<(u32, u32)>, from: usize, to: usize| {
        // 0x1000dc10
        for c in cat.iter_mut().take(to).skip(from) {
            *c = (7, 0);
        }
    };
    for u in 0..n {
        let r = &w[u];
        let (c, sub) = (r.f[CLASS], r.f[SUB]);
        let prev = |k: usize| if u >= k { Some(&w[u - k]) } else { None };
        let next = w.get(u + 1);
        let same_neighbours = || {
            prev(1).map(|p| (p.f[CLASS], p.f[SUB])) == next.map(|x| (x.f[CLASS], x.f[SUB]))
        };
        if l24 == 0 {
            if let Some(l) = phrase(en, w, u, &ADVERBIALS) {
                l24 = l;
                c10 = 1;
            }
        } else if c10 < l24 {
            c10 += 1;
        } else if c10 == l24 {
            l24 = 0;
            phrase_end = u;
        }
        if l20 == 0 {
            if let Some(l) = phrase(en, w, u, &CONJUNCTIONS) {
                l20 = l;
                c0c = 1;
            }
        } else if c0c < l20 {
            c0c += 1;
        } else if c0c == l20 {
            l20 = 0;
        }
        if l1c == 0 {
            if let Some(l) = phrase(en, w, u, &PHRASAL) {
                l1c = l;
                c08 = 1;
            }
        } else if c08 < l1c {
            c08 += 1;
        } else if c08 == l1c {
            l1c = 0;
        }
        let in_phrase = l24 != 0 || l20 != 0;
        // the state's rule: Some(next state) skips the classification below
        let go: Option<i32> = match state {
            1 => {
                if in_phrase {
                    state = 0;
                    None
                } else if c != 0x37 && !opener(c) && c != 0x1f {
                    if verb(c) && !participle(sub) {
                        Some(2)
                    } else {
                        state = 0;
                        None
                    }
                } else {
                    Some(1)
                }
            }
            2 => {
                let stays = !(c != 0x37 && !verb(c) && c != 0x33 && c != 0x2e && (sub != 0x82 || !same_neighbours()));
                if stays {
                    Some(2)
                } else {
                    let t = r.text.as_slice();
                    let linking = (t == b"of" || t == b"to" || t == b"at") && next.is_some_and(|x| verb(x.f[CLASS]));
                    if linking || l1c != 0 {
                        Some(2)
                    } else {
                        fill(&mut cat, last, u - 1);
                        cat[u - 1].0 = 1;
                        state = 0;
                        None
                    }
                }
            }
            3 => {
                if in_phrase {
                    state = 0;
                    None
                } else if !matches!(c, 0x37 | 0x18 | 0x16 | 0x1f) {
                    if matches!(c, 0xee | 0x15 | 0x29) || matches!(sub, 0x88 | 0x8e | 0x89 | 0x8d | 0x8c) {
                        Some(4)
                    } else {
                        state = 0;
                        None
                    }
                } else {
                    Some(3)
                }
            }
            4 => {
                if !content(c) && !matches!(sub, 0x88 | 0x8e | 0x89 | 0x8d | 0x8c) && (sub != 0x82 || !same_neighbours()) {
                    fill(&mut cat, last, u - 1);
                    state = 0;
                    cat[u - 1].0 = 0;
                    None
                } else {
                    Some(4)
                }
            }
            5 => {
                if in_phrase {
                    state = 0;
                    None
                } else if opener(c) || c == 0x1f {
                    Some(1)
                } else if verb(c) && !participle(sub) && (u < 2 || !participle(w[u - 2].f[SUB])) {
                    Some(2)
                } else if content(c) || participle(sub) {
                    Some(6)
                } else {
                    state = 0;
                    None
                }
            }
            6 => {
                if !matches!(c, 0xee | 0x15 | 0x29) && !participle(sub) && c != 0x37 {
                    fill(&mut cat, last, u - 1);
                    cat[u - 1].0 = 2;
                    state = 0;
                    None
                } else {
                    Some(6)
                }
            }
            7 => {
                cat[u - 1].0 = 6;
                state = 0;
                None
            }
            9 => {
                if !in_phrase {
                    fill(&mut cat, last, u - 1);
                    cat[u - 1] = (3, if phrase_end == u { 4 } else { 3 });
                    state = 0;
                    None
                } else {
                    Some(9)
                }
            }
            0 => None,
            s => Some(s),
        };
        if let Some(s) = go {
            state = s;
            continue;
        }
        last = u;
        if !in_phrase {
            if c == 0x1f {
                state = 7;
            } else if opener(c) {
                state = 1;
            } else if verb(c) && !participle(sub) {
                if !matches!(sub, 0x8d | 0x8c | 0x8b) && u != 0 && w[u - 1].f[CLASS] == b'-' {
                    cat[u - 1].0 = 0;
                }
                state = 2;
            } else if c == 0x18 || c == 0x16 {
                state = 3;
            } else if content(c) || participle(sub) {
                state = 4;
            } else if c == 0x19 || c == 0x1e {
                state = 5;
            } else if c == b'2' {
                cat[u].0 = 8;
            }
        } else {
            state = 9;
        }
    }
    cat
}

/// 0x1000dce0: the group from `e` back to the last pause and the words
/// after `u` up to the punctuation are too short to split (both need ≥ 2
/// words and more than 14 / 17 letters).
fn too_short(w: &[Word], u: usize, e: usize) -> bool {
    let mut left_len = w.get(e).map_or(0, |r| r.text.len());
    let mut left_words = 1;
    for j in (0..e).rev() {
        let b = w[j].f[BOUND];
        if b != 0x16 {
            if b == 10 || b == 11 {
                break;
            }
            left_len += w[j].text.len();
            left_words += 1;
        }
    }
    let (mut right_len, mut right_words) = (0usize, 0);
    for r in w.iter().skip(u + 1) {
        if r.f[CLASS] == b'2' {
            break;
        }
        if r.f[CLASS] != b'7' {
            right_words += 1;
            right_len += r.text.len();
        }
    }
    if e < u {
        for r in w[e + 1..=u].iter().rev() {
            if r.f[CLASS] != b'7' {
                right_len += r.text.len();
                right_words += 1;
            }
        }
    }
    !(left_words > 1 && left_len > 0xe && right_words > 1 && right_len > 0x11)
}

/// 0x1000e380: the word does not end in `st`.
fn not_st(w: &[Word], i: usize) -> bool {
    let t = &w[i].text;
    !(t.len() > 1 && t.ends_with(b"st"))
}

/// 0x1000dc40: a subordinating word (`because`… 0x1002b458, 12 entries;
/// `after`… 0x1002b488, 20) since the last boundary 1.
fn subordinate(en: &English, w: &[Word], u: usize) -> bool {
    let listed = |t: &[u8]| search_string(en, t, 0x1002b458, 12) || search_string(en, t, 0x1002b488, 20);
    if u == 0 {
        return false;
    }
    for i in (1..=u).rev() {
        if w[i].f[BOUND] == 1 {
            return false;
        }
        if listed(&w[i].text) {
            return true;
        }
    }
    listed(&w[0].text)
}

/// LesConfiniProsodici (0x1000eb20): each word's boundary (`+0x11`) from
/// its group category: 5 after a group, 3 inside one, 1 before a closer
/// link, 9 after a content word, 0x16 none (enclitic), punctuation 0x0b
/// (`,` `-` `(` `)`) or by sentence type (0x0f `.`, 0x11, 0x12 `?`, 0x10
/// `:`, 0x13 `!`); 10 (a short pause) splits a long group before a
/// pronoun, verb or article when both halves are long enough.
pub(super) fn confini(en: &English, w: &mut [Word], mode: i32) {
    let cat = categories(en, w);
    let mut prev = 200usize;
    let can_split = |w: &[Word], prev: usize| prev != 200 && w[prev].f[CLASS] != b'2';
    for u in 0..w.len().min(200) {
        if cat[u].0 == 10 {
            break;
        }
        if w[u].f[BOUND] == 0x15 {
            let c = w[u].f[CLASS];
            match cat[u].0 {
                0 | 2 => {
                    if !too_short(w, u, u) && not_st(w, u) {
                        w[u].f[BOUND] = 10;
                    }
                    if c == b'7' {
                        w[u].f[BOUND] = 0x16;
                    } else {
                        w[u].f[BOUND] = 5;
                        prev = u;
                    }
                }
                1 => {
                    if c == b'7' {
                        w[u].f[BOUND] = 0x16;
                    } else {
                        if can_split(w, prev) {
                            let b = w[prev].f[BOUND];
                            let first = cat[prev].1 == 0 && mode != 3 && mode != 2 && matches!(b, 3 | 5 | 7) && cat[prev].0 != 5 && !too_short(w, u, prev);
                            let split = if first && !subordinate(en, w, u) && not_st(w, prev) {
                                true
                            } else {
                                !(w[prev].f[CLASS] == b'2' || cat[prev].1 != 2 || mode == 3 || mode == 2)
                            };
                            if split {
                                w[prev].f[BOUND] = 10;
                            }
                        }
                        w[u].f[BOUND] = 5;
                        prev = u;
                    }
                }
                3 => {
                    let d = cat[u].1;
                    if d == 4 || d == 3 {
                        if prev != 200 && prev != 0 && w[prev].f[CLASS] != b'2' {
                            w[prev].f[BOUND] = 10;
                        }
                        w[u].f[BOUND] = if d == 4 { 10 } else { 3 };
                    } else {
                        w[u].f[BOUND] = 1;
                    }
                    prev = u;
                }
                4 => {
                    w[u].f[BOUND] = 9;
                    prev = u;
                }
                6 => {
                    if mode != 3 && mode != 2 && can_split(w, prev) && matches!(w[prev].f[BOUND], 3 | 5 | 7) && !too_short(w, u, prev) {
                        w[prev].f[BOUND] = 10;
                    }
                    w[u].f[BOUND] = 1;
                }
                7 => {
                    w[u].f[BOUND] = if c == b'7' {
                        0x16
                    } else if matches!(c, 0x19 | 0x1e | 0x18 | 0x20 | 0x21 | 0x16 | 0x22 | 0x1f | 0x82 | 0x2e) {
                        1
                    } else {
                        3
                    };
                }
                8 => {
                    let mut j = u.saturating_sub(1);
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
                        let mut i = 0;
                        while super::super::common::at(t, i + 1) != 0 && !b",)(-.".contains(&super::super::common::at(t, i)) {
                            i += 1;
                        }
                        let ch = match super::super::common::at(t, i) {
                            b'.' => b',',
                            x => x,
                        };
                        match ch {
                            b'(' | b')' => {
                                if u == 0 || w[u - 1].f[CLASS] == b'2' || w.get(u + 1).is_some_and(|x| x.f[CLASS] == b'2') {
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
                        prev = u;
                    }
                }
                9 => w[u].f[BOUND] = 0x16,
                _ => {
                    prev = u;
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
    }
}

/// 0x1000d970: the record's rows disagree on `gradoaccentuale` (one row
/// against the one before it).
fn grades_differ(en: &English, r: &mut Word) -> bool {
    let Some(mut prev) = Rows::head(r) else { return false };
    while let Some(row) = Rows::next(r) {
        if !super::stricmp(en.cell(row, GRADO).unwrap_or(b""), en.cell(prev, GRADO).unwrap_or(b"")).is_eq() {
            return true;
        }
        prev = row;
    }
    false
}

/// 0x1000c520: the first row's grade that is DEA, ACD or ACC; 0x0e when
/// none is.
fn first_grade(en: &English, r: &mut Word) -> u8 {
    let mut cur = Rows::head(r);
    while let Some(row) = cur {
        let g = grade_of(en.cell(row, GRADO));
        if g != 0x0e {
            return g;
        }
        cur = Rows::next(r);
    }
    0x0e
}

/// LesGradiAccentuali (0x1000da10): words whose rows disagree on the
/// stress grade (homographs: `the`, `a`, `and`…) are settled by their
/// neighbours (US 0x1000ce10, GB/AU 0x1000c830); the others take the grade
/// of their rows, 0 without one, punctuation 0x0f.
pub(super) fn gradi(en: &English, w: &mut [Word]) {
    for u in 0..w.len().min(200) {
        if grades_differ(en, &mut w[u]) {
            match en.variant {
                Variant::Us => omografi::us(en, w, u),
                Variant::Gb | Variant::Au => omografi::gb(en, w, u),
            }
        } else if w[u].f[GRADE] == 0x0e {
            let c = w[u].f[CLASS];
            w[u].f[GRADE] = if c == b'2' || c == b'7' {
                0x0f
            } else {
                match first_grade(en, &mut w[u]) {
                    0x0e => 0,
                    g => g,
                }
            };
        }
    }
}
