//! `LesGradiAccentuali` (0x10009e60): stress grades (record +0x12; 0
//! stressed, 1 verb, 3 unstressed, 8 number, 9 `plus` after `ne`, 0x0f
//! punctuation), after 0x100073a0 has weakened boundaries inside fixed
//! phrases (+0x11 = 2).

use super::French;
use crate::lang::common::{field::*, Word};

fn verb(c: u8) -> bool {
    matches!(c, 0x2a | 0x2c | 0x2b | 0xd8)
}

/// The function-word classes the grades treat as clitic.
fn little(c: u8) -> bool {
    matches!(c, 0x19 | 0x1e | 0x18 | 0x20 | 0x21 | 0x16 | 0x22 | 0x1f | 0x82 | 0x2e)
}

fn tx(w: &[Word], i: usize) -> Option<&[u8]> {
    w.get(i).map(|r| &r.text[..])
}

fn f(w: &[Word], i: isize, k: usize) -> u8 {
    super::fld(w, i, k)
}

fn is(t: Option<&[u8]>, s: &[u8]) -> bool {
    t == Some(s)
}

fn any(t: Option<&[u8]>, set: &[&[u8]]) -> bool {
    t.is_some_and(|t| set.contains(&t))
}

impl French {
    pub(crate) fn gradi(&self, w: &mut [Word]) {
        gradi_pass(w);
        let n = w.len();
        let mut k = 0usize;
        while k < n && k < 200 && w[k].f[BOUND] != 0x17 {
            let c = w[k].f[CLASS];
            if c == b'2' || c == 0x37 {
                w[k].f[GRADE] = 0xf;
                k += 1;
                continue;
            }
            let elided = w[k].text.last() == Some(&b'\'');
            let ki = k as isize;
            // the next and previous words, over a 0x37 one
            let next = if k + 1 < n {
                if w[k + 1].f[CLASS] == 0x37 && k + 2 < n {
                    k + 2
                } else {
                    k + 1
                }
            } else {
                k
            };
            let prev = if k == 0 {
                0
            } else if f(w, ki - 1, CLASS) == 0x37 && k != 1 {
                k - 2
            } else {
                k - 1
            };
            if matches!(w[k].f[BOUND], 2 | 1 | 0 | 6 | 7 | 3 | 0x16 | 5) {
                if k != 0 && w[k].text == b"plus" && k - 1 != 0 {
                    // `ne … plus`: grade 9, and the next word is the one
                    // looked at below
                    let mut j = k - 1;
                    loop {
                        let t = &w[j].text[..];
                        if t == b"ne" || t == b"n'" {
                            w[k].f[GRADE] = 9;
                            k += 1;
                            break;
                        }
                        if t == b"pas" || t == b"que" || t == b"qui" {
                            break;
                        }
                        j -= 1;
                        if j == 0 {
                            break;
                        }
                    }
                }
                if k >= n {
                    break;
                }
                if self.grade_8(w, k, next, prev) {
                    w[k].f[GRADE] = 8;
                    k += 1;
                    continue;
                }
            }
            let t = w[k].text.clone();
            if [&b"\xe7a"[..], b"ni", b"or", b"ceci", b"cela", b"avec", b"jusqu'"].contains(&&t[..]) {
                w[k].f[GRADE] = 0;
                k += 1;
                continue;
            }
            let ki = k as isize;
            if w[k].f[SUB] == 0x83 {
                // a spelled letter: grades alternate along a spelled word
                if f(w, ki + 1, SUB) != 0x83 && (k == 0 || f(w, ki - 1, SUB) != 0x83) {
                    w[k].f[GRADE] = 0;
                } else {
                    let mut odd = false;
                    let mut j = k;
                    while j < n && w[j].f[SUB] == 0x83 {
                        odd = !odd;
                        j += 1;
                    }
                    w[k].f[GRADE] = if odd { 3 } else { 0 };
                }
                k += 1;
                continue;
            }
            let short_little = matches!(w[k].f[BOUND], 2 | 1 | 0 | 6 | 7 | 3 | 0x16) && little(w[k].f[CLASS]) && t.len() <= 4;
            w[k].f[GRADE] = if short_little || elided {
                3
            } else if verb(w[k].f[CLASS]) {
                1
            } else {
                0
            };
            k += 1;
        }
        if k < n {
            w[k].f[GRADE] = 0x10;
            w[k].f[BOUND] = 0x17;
        }
    }

    /// The words graded 8 (0x1000a0f0…): auxiliaries and determiners
    /// (`est`, `a`, `ces`, `mon`, `tous`, `si`…), numbers before
    /// `mille`/`cent` and the like.
    fn grade_8(&self, w: &[Word], k: usize, next: usize, prev: usize) -> bool {
        const AUX: &[&[u8]] = &[
            b"est", b"es", b"a", b"as", b"ai", b"ont", b"suis", b"sont", b"ces", b"cette", b"cet", b"mes", b"mon", b"nos", b"sa",
            b"ses", b"son", b"ta", b"tes", b"ton", b"tel", b"tous", b"y", b"si",
        ];
        const NUMBERS: &[&[u8]] = &[
            b"un", b"deux", b"trois", b"quatre", b"cinq", b"six", b"sept", b"huit", b"neuf", b"dix", b"onze", b"douze", b"treize",
            b"quinze", b"seize", b"vingt", b"trente", b"cent", b"mille",
        ];
        const UNITS: &[&[u8]] = &[b"deux", b"trois", b"cinq", b"six", b"sept", b"huit", b"neuf", b"dix", b"onze"];
        let t = &w[k].text[..];
        if AUX.contains(&t) {
            return true;
        }
        let ni = next as isize;
        let nt = tx(w, next);
        if t == b"dix" && is(nt, b"mille") {
            return true;
        }
        if t == b"vingt"
            && k != 0
            && is(tx(w, prev), b"quatre")
            && (w[k].f[SUB] == b'z' || f(w, k as isize + 1, CLASS) == 0x37)
            && !any(nt, &[b"quatorze", b"dixsept", b"dixhuit", b"dixneuf"])
        {
            return true;
        }
        // `cent`, `mille` before a number
        let number_after = |w: &[Word]| {
            any(nt, NUMBERS)
                && !(f(w, ni, SUB) != b'y' && (f(w, ni + 1, CLASS) == 0x37 || matches!(f(w, ni + 1, SUB), b'z' | b'y')))
        };
        if (t == b"cent" || t == b"mille") && nt.is_some() && number_after(w) {
            return true;
        }
        // a unit before `cent`, `cents`, `mille`
        UNITS.contains(&t)
            && any(nt, &[b"cent", b"cents", b"mille"])
            && !(f(w, ni, SUB) != b'y' && (f(w, ni + 1, CLASS) == 0x37 || matches!(f(w, ni + 1, SUB), b'z' | b'y')))
    }
}

/// 0x100073a0: inside fixed phrases and after clitics the boundary
/// becomes 2 (`tout`, `en`, `jamais`, `bien`, `plus ou moins`, numbers…).
fn gradi_pass(w: &mut [Word]) {
    let n = w.len();
    let mut k = 0usize;
    while k < n && k < 200 && w[k].f[BOUND] != 0x17 {
        let c = w[k].f[CLASS];
        if c == b'2' || c == 0x37 {
            w[k].f[GRADE] = 0xf;
        } else if matches!(w[k].f[BOUND], 2 | 1 | 6 | 7 | 3 | 0x16 | 5) && k + 1 < n {
            if let Some(two) = weakens(w, k) {
                if two {
                    w[k].f[BOUND] = 2;
                }
            }
        }
        k += 1;
    }
}

/// Whether word `k`'s boundary becomes 2 (`Some(true)`); `Some(false)`
/// and `None` leave it.
fn weakens(w: &[Word], k: usize) -> Option<bool> {
    let ki = k as isize;
    let t = &w[k].text[..];
    let n1 = tx(w, k + 1);
    let n2 = tx(w, k + 2);
    let n3 = tx(w, k + 3);
    let nc = f(w, ki + 1, CLASS);
    let ns = f(w, ki + 1, SUB);
    let class = w[k].f[CLASS];
    let sub = w[k].f[SUB];
    if is(n1, b"-") && !(class == 0x82 && f(w, ki + 2, CLASS) == 0xd3) {
        return Some(true);
    }
    if t == b"non" {
        return Some(false);
    }
    if [&b"aux"[..], b"sans", b"tout"].contains(&t) && nc != 0x1f && ns != b'y' {
        return Some(true);
    }
    if t == b"en" && nc != 0x1f && ns != b'y' && (k == 0 || f(w, ki - 1, CLASS) != 0x20 || !any(n1, &[b"un", b"huit", b"onze"])) {
        return Some(true);
    }
    if [&b"jamais"[..], b"moins", b"non", b"pas", b"puis", b"quand", b"tr\xe8s", b"trop", b"plus"].contains(&t) && nc != 0x1f {
        return Some(true);
    }
    if t == b"bien" && matches!(nc, 0x2a | 0x2c | 0x2b | 0xd8 | 0x29 | 0xdb | 0xdc | 0xde | 0xda | 0xe8 | 0xd3) {
        return Some(true);
    }
    if t == b"rien" && (matches!(nc, 0x19 | 0x1e) || matches!(ns, 0x8d | 0x8c)) {
        return Some(true);
    }
    if t == b"mais" {
        if matches!(nc, 0x20 | 0x18 | 0x33 | 0x19 | 0x1e) {
            return Some(true);
        }
    } else if t == b"quant" && matches!(nc, 0x19 | 0x1e) {
        return Some(true);
    } else if t == b"dont" && nc == 0x20 {
        return Some(true);
    }
    // a word repeated around en/à/ou/et (`plus ou plus`…)
    if k + 2 < 200 {
        if f(w, ki + 1, BOUND) != 0x17 {
            if is(n2, t) && any(n1, &[b"en", b"\xe0", b"ou", b"et"]) {
                return Some(true);
            }
            if let Some(two) = fixed_pairs(w, k) {
                return Some(two);
            }
        }
    } else if let Some(two) = fixed_pairs(w, k) {
        return Some(two);
    }
    if k + 2 < 200 && f(w, ki + 2, BOUND) != 0x17 && t == b"bout" && is(n1, b"\xe0") && is(n2, b"l'") && is(n3, b"autre") {
        return Some(true);
    }
    if t == b"depuis" && nc == 0x33 {
        return Some(true);
    }
    let others: &[&[u8]] = &[b"aucun", b"autres", b"autre"];
    if [&b"apr\xe8s"[..], b"chez", b"dans", b"d\xe8s", b"sous", b"devant"].contains(&t) {
        let free = !matches!(nc, 0x33 | 0x18 | 0x16 | 0x19 | 0x1e | 0x20) && !any(n1, others) && !is(n1, b"aussi") && ns != 0x8d;
        if !free {
            return Some(true);
        }
    }
    if t == b"plein" && !(nc != 0x2f && !any(n1, others) && !is(n1, b"avoir") && !is(n1, b"\xeatre")) {
        return Some(true);
    }
    if t == b"fait" {
        let free = (!matches!(nc, 0x2a | 0x2c | 0x2b | 0xd8) || sub == 0x8d) && !is(n1, b"un") && !is(n1, b"une");
        if !free {
            return Some(true);
        }
    }
    rest(w, k)
}

/// `plus ou moins`, `d'ores et déjà` (0x10007b71).
fn fixed_pairs(w: &[Word], k: usize) -> Option<bool> {
    let ki = k as isize;
    if f(w, ki + 1, BOUND) == 0x17 || k + 1 >= 200 {
        return None;
    }
    let t = &w[k].text[..];
    let (n1, n2) = (tx(w, k + 1), tx(w, k + 2));
    if t == b"plus" && is(n1, b"ou") && is(n2, b"moins") {
        return Some(true);
    }
    if t == b"ores" && is(n1, b"et") && is(n2, b"d\xe9j\xe0") {
        return Some(true);
    }
    None
}

/// The checks from 0x10008232 on.
fn rest(w: &[Word], k: usize) -> Option<bool> {
    let ki = k as isize;
    let t = &w[k].text[..];
    let n1 = tx(w, k + 1);
    let nc = f(w, ki + 1, CLASS);
    let ns = f(w, ki + 1, SUB);
    let class = w[k].f[CLASS];
    let sub = w[k].f[SUB];
    let info = w[k].f[INFO];
    if t == b"y" || t == b"ces" {
        return Some(true);
    }
    if sub == b'y' && t != b"vingt" && (nc == 0x2f || is(n1, b"autres") || nc == 0x15) {
        return Some(true);
    }
    if t == b"vingt" || t == b"cent" {
        if is(n1, b"un") || is(n1, b"onze") {
            return Some(false);
        }
    }
    if matches!(sub, b'y' | b'z') && t != b"et" {
        return Some(true);
    }
    if t == b"vingt" && any(n1, &[b"autres", b"ans", b"hommes", b"heures"]) {
        return Some(true);
    }
    if t == b"autres" && any(n1, &[b"aussi", b"en", b"encore"]) {
        return Some(true);
    }
    if t == b"aucun" && is(n1, b"autre") {
        return Some(true);
    }
    if matches!(class, 0x18 | 0x16) && !(k != 0 && w[k - 1].text == b"quelqu'") && nc != 0x1f && ns != b'y' {
        return Some(true);
    }
    if class == 0x1e || matches!(class, 0xdb | 0xdc | 0xde) {
        return Some(true);
    }
    if matches!(class, 0x29 | 0xda | 0xe8 | 0xd3) && matches!(nc, 0x15 | 0x2f | 0x2d) {
        return Some(true);
    }
    if class == 0x20 && t != b"chacun" && t != b"quelq'un" && (k == 0 || f(w, ki - 1, CLASS) != 0x37) {
        return Some(true);
    }
    if matches!(class, 0x15 | 0x2f) && matches!(info, 0xe | 0xc | 0x10) && matches!(nc, 0x29 | 0x2f) {
        return Some(true);
    }
    if verb(class) && matches!(ns, 0x88 | 0x8d) {
        return Some(true);
    }
    if t == b"est" {
        return Some(true);
    }
    if verb(class) && info == 0xd1 && t.last() == Some(&b't') {
        return Some(true);
    }
    if class == 0xd8 && info == 0xce && ns == 0x8d {
        return Some(true);
    }
    if matches!(nc, 0x82 | 0xe6) && matches!(info, 0xe | 0xc | 0x10) && class == f(w, ki + 2, CLASS) {
        return Some(true);
    }
    Some(false)
}
