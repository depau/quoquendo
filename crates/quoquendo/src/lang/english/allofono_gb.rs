//! FonAllofonoLingua, EnglishGb and EnglishAu (0x100122c0; both function
//! tables, 0x100291c0 and 0x100292a8, point at it).
//!
//! Called on entry `i` of the chunk's phone list. It first links words: a
//! linking `Hr` after a word ending in r/re before a vowel, a `j` (0x10ce),
//! `w` (0x401ece) or `r` (0x9e2) glide between a word-final vowel and a
//! word-initial vowel. Then it rewrites entry `i` by its code: flaps and
//! glottal stops for t/d/k/p (0x19f2, 0x39f2, 0x19e2, 0x1fe2/0x1ff2/0x3ff2,
//! 0xfe2/0xff2/0x2ff2) before syllabic n (the following schwa+n merged into
//! 0x219c2), word-final and medial-before-consonant positions; d+j and t+j
//! coalescence; dark and syllabic l; weak forms of "the", "to", "be",
//! "have to", "has to", "had to"; and the dropping of an `Hr` from a
//! `\SAMPA=` word before a pause. The ELQList list (`ctx+0x54`) and its
//! index array (`ctx+0x10`, rebuilt in place after every insert/delete)
//! behave like a `Vec`: an insert after entry k shifts the rest up, a delete
//! of entry k brings k+1 into slot k.

use super::English;
use crate::lang::common::{field::GRADE, Phone, Word};

/// Entries past the list end: the engine reads stale array slots there.
const PAST_END: Phone = Phone { word: u32::MAX, code: 0, kind: 0 };
/// The kind an inserted entry gets (the stack byte after the pushed code).
const INSERTED: u8 = 0x23;
/// Pause codes `@` 0x17 and `#` 0x13.
const PAUSES: [u32; 2] = [0x17, 0x13];

/// Vowels before which a word-initial vowel gets a `j` glide 0x10ce.
const BEFORE_J: [u32; 10] = [0x7c1, 0x107c5, 0x781, 0x10781, 0xe445, 0x1e445, 0xe6c5, 0x1e6c5, 0xe2e5, 0x1e2e5];
/// Vowels before which a word-initial vowel gets a `w` glide 0x401ece.
const BEFORE_W: [u32; 8] = [0x3e1, 0x103e5, 0x3a1, 0x103a1, 0xac45, 0x1ac45, 0xad05, 0x1ad05];
/// Vowels before which a word-initial vowel gets an `r` 0x9e2.
const BEFORE_R: [u32; 19] = [
    0x10245, 0x245, 0x10261, 0x261, 0x102c1, 0x2c1, 0x10681, 0x681, 0x106c1, 0x6c1, 0x146c5, 0x46c5, 0x14785, 0x4785,
    0x143a5, 0x43a5, 0x102e5, 0x2e5, 0x501,
];
/// The vowels before a dark l (0x18a2 rule): BEFORE_J without 0x781/0x10781.
const BEFORE_DARK_L: [u32; 8] = [0x7c1, 0x107c5, 0xe445, 0x1e445, 0xe6c5, 0x1e6c5, 0xe2e5, 0x1e2e5];
/// Consonants that keep a medial stop unchanged.
const KEEP_MEDIAL: [u32; 5] = [0x18c2, 0x18a2, 0x952, 0x401ec2, 0x10c2];
/// Next words that make a final t/d a `j` coalescence.
const YOU: [&[u8]; 10] = [b"you", b"your", b"you've", b"you'd", b"you'll", b"yours", b"yer", b"yeh", b"yourself", b"yourselves"];

struct L<'a> {
    w: &'a [Word],
    first: usize,
    p: &'a mut Vec<Phone>,
}

impl L<'_> {
    fn at(&self, k: usize) -> Phone {
        self.p.get(k).copied().unwrap_or(PAST_END)
    }
    fn code(&self, k: usize) -> u32 {
        self.at(k).code
    }
    fn word(&self, k: usize) -> u32 {
        self.at(k).word
    }
    fn same(&self, a: usize, b: usize) -> bool {
        self.word(a) == self.word(b)
    }
    fn vowel(&self, k: usize) -> bool {
        self.code(k) & 3 == 1
    }
    fn cons(&self, k: usize) -> bool {
        self.code(k) & 3 == 2
    }
    fn pause(&self, k: usize) -> bool {
        PAUSES.contains(&self.code(k))
    }
    fn set(&mut self, k: usize, code: u32) {
        self.p[k].code = code;
    }
    /// ELQListFindElement(k) + ELQListAddElementAfter.
    fn insert_after(&mut self, k: usize, word: u32, code: u32) {
        self.p.insert(k + 1, Phone { word, code, kind: INSERTED });
    }
    /// ELQListDeleteElement.
    fn delete(&mut self, k: usize) {
        self.p.remove(k);
    }

    /// An n at `n` closes its syllable: last of its word, not before a
    /// vowel, or before `I` 0x781 + `ng` 0xfc2 ending the word.
    fn n_ends_syllable(&self, n: usize) -> bool {
        !self.same(n, n + 1)
            || !self.vowel(n + 1)
            || (self.code(n + 1) == 0x781 && self.code(n + 2) == 0xfc2 && !self.same(n + 2, n + 3))
    }
    /// The phone after `i` is a syllabic n of the same word.
    fn syllabic_n(&self, i: usize) -> bool {
        self.code(i + 1) == 0x19c2 && self.same(i, i + 1) && self.n_ends_syllable(i + 1)
    }
    /// A consonant at `k` that leaves a medial stop unchanged.
    fn keeps_medial(&self, k: usize) -> bool {
        KEEP_MEDIAL.contains(&self.code(k)) || (self.code(k) == 0x19c2 && (!self.same(k, k + 1) || !self.vowel(k + 1)))
    }
    /// `i` inside its word, before a consonant other than KEEP_MEDIAL.
    fn medial(&self, i: usize) -> bool {
        i != 0 && self.same(i - 1, i) && self.same(i, i + 1) && self.cons(i + 1) && !self.keeps_medial(i + 1)
    }
    /// schwa 0x501 + n 0x19c2 follow, the n in the previous phone's word.
    fn schwa_n(&self, i: usize) -> bool {
        i != 0 && self.code(i + 1) == 0x501 && self.code(i + 2) == 0x19c2 && self.same(i - 1, i + 2)
    }
    /// The word ends in "on" with the n at `n` its last phone before a
    /// non-pause, or in "ons" with n+1 its last.
    fn on_ons(&self, text: &[u8], n: usize) -> bool {
        (text.ends_with(b"on") && !self.same(n, n + 1) && !self.pause(n + 1))
            || (text.ends_with(b"ons") && !self.same(n + 1, n + 2) && !self.pause(n + 2))
    }
    /// The stop at `i` becomes `code` and swallows the schwa of a following
    /// schwa+n into a syllabic n (LAB_10012dfd from the stop cases): returns
    /// the index to continue from.
    fn merge_schwa_n(&mut self, i: usize, code: u32) -> usize {
        self.set(i, code);
        self.delete(i + 1);
        self.set(i + 1, 0x219c2);
        i + 1
    }
    /// The stop at `i` becomes `code`, the following n syllabic (LAB_100131ad).
    fn before_syllabic_n(&mut self, i: usize, code: u32) -> usize {
        self.set(i, code);
        self.set(i + 1, 0x219c2);
        i + 1
    }
}

fn is(t: &[u8], list: &[&[u8]]) -> bool {
    list.iter().any(|s| t == *s)
}
fn contains(t: &[u8], s: &[u8]) -> bool {
    t.windows(s.len()).any(|x| x == s)
}

/// FonAllofonoLingua (0x100122c0) on entry `i`; returns the index the
/// engine continues from (+1).
pub(super) fn allofono(_en: &English, w: &[Word], first: usize, phones: &mut Vec<Phone>, i: usize) -> usize {
    let mut l = L { w, first, p: phones };
    let wi = l.word(i) as usize;
    let rec = l.w.get(l.first + wi);
    let text: Vec<u8> = rec.map(|r| r.text.clone()).unwrap_or_default();
    // record +0x1c: the next record's text
    let next: Vec<u8> = l.w.get(l.first + wi + 1).map(|r| r.text.clone()).unwrap_or_default();
    let tag: Option<Vec<u8>> = rec.and_then(|r| r.tag.clone());
    let grade = rec.map(|r| r.f[GRADE]).unwrap_or(0);
    let t = text.as_slice();
    let nx = next.as_slice();

    // Linking r: a word ending in r/re, its last phone a vowel, before a vowel.
    if !l.same(i, i + 1) && l.vowel(i) && (t.ends_with(b"re") || t.ends_with(b"r")) && l.vowel(i + 1) {
        l.insert_after(i, l.word(i), 0x18c2);
    }
    // Glides between a word-final and a word-initial vowel (inserted before i).
    if i != 0 && l.code(i - 1) != 0x18c2 && !l.same(i - 1, i) {
        let pc = l.code(i - 1);
        let glide = if BEFORE_J.contains(&pc) {
            Some(0x10ce)
        } else if BEFORE_W.contains(&pc) {
            Some(0x401ece)
        } else if BEFORE_R.contains(&pc) {
            Some(0x9e2)
        } else {
            None
        };
        if let Some(g) = glide {
            if l.vowel(i) {
                l.insert_after(i - 1, l.word(i), g);
            }
        }
    }

    match l.code(i) {
        0x19c2 => n(&mut l, i),
        0x19e2 => d(&mut l, i, t, nx),
        0x19f2 | 0x39f2 => tt(&mut l, i, t, nx),
        0x1d42 => {
            if t == b"have" && nx == b"to" {
                l.set(i, 0x1d52);
            }
            i
        }
        0x1fe2 => k(&mut l, i, t),
        0x1ff2 | 0x3ff2 => kk(&mut l, i, t),
        0x104c5 | 0x4c5 => {
            // LAB_10017202: an unstressed-grade-2 word's vowel becomes schwa
            if grade != 2 {
                return i;
            }
            l.set(i, 0x501);
            if l.vowel(i + 1) {
                l.insert_after(i, l.word(i), 0x18c2);
            }
            i
        }
        0x107c5 => {
            if t == b"be" && !l.pause(i + 1) {
                l.set(i, 0x7c1);
            }
            i
        }
        0x1952 => {
            if !l.same(i, i + 1) && l.code(i + 1) == 0x1752 {
                l.set(i, 0x1752);
            }
            i
        }
        0x9e2 => {
            if i != 0 {
                return i;
            }
            l.delete(0);
            0
        }
        0x7c1 => {
            if t == b"the" && l.cons(i + 1) && l.code(i + 1) != 0x10c2 {
                l.set(i, 0x501);
            }
            i
        }
        0x3e1 => {
            if !l.same(i, i + 1) && l.cons(i + 1) && l.code(i + 1) != 0x401ec2 && is(t, &[b"to", b"into", b"onto"]) {
                l.set(i, 0x501);
            }
            i
        }
        0x501 => schwa(&mut l, i, t),
        0xfe2 => p(&mut l, i, t),
        0x18f2 => {
            l.set(i, 0x18a2);
            i
        }
        0x1942 => {
            if l.same(i, i + 1) || l.code(i + 1) != 0x1752 {
                if t == b"has" && nx == b"to" {
                    l.set(i, 0x1952);
                }
            } else {
                l.set(i, 0x1752);
            }
            i
        }
        0x18c2 => {
            // an Hr of a \SAMPA= word before a pause goes
            if !tag.as_deref().is_some_and(|g| contains(g, b"\\SAMPA=")) {
                return i;
            }
            if i == 0 {
                return 0;
            }
            if !l.pause(i + 1) {
                return i;
            }
            l.delete(i);
            i.wrapping_sub(1)
        }
        0x18a2 => el(&mut l, i, t),
        0xff2 | 0x2ff2 => pp(&mut l, i, t),
        _ => i,
    }
}

/// n 0x19c2.
fn n(l: &mut L, i: usize) -> usize {
    if i != 0 && l.cons(i - 1) && l.same(i - 1, i) && l.n_ends_syllable(i) {
        l.set(i, 0x219c2);
    } else if [0xfe2, 0x2ff2, 0xff2].contains(&l.code(i + 1)) {
        l.set(i, 0xfc2);
    }
    i
}

/// d 0x19e2.
fn d(l: &mut L, i: usize, t: &[u8], nx: &[u8]) -> usize {
    if l.code(i + 1) == 0x1942 && l.same(i, i + 1) && !l.same(i + 1, i + 2) {
        l.delete(i);
        l.set(i, 0x1902);
        return i;
    }
    if i != 0 {
        if l.schwa_n(i) && t != b"london" && l.on_ons(t, i + 2) {
            return l.merge_schwa_n(i, 0x59e2);
        }
        if l.syllabic_n(i) {
            return l.before_syllabic_n(i, 0x59e2);
        }
    }
    let last = !l.same(i, i + 1);
    if last && is(nx, &YOU) {
        l.set(i, 0x1702);
        return i;
    }
    if l.cons(i + 1) && last && ![0x952, 0x10c2, 0x401ec2, 0x39f2, 0x19f2].contains(&l.code(i + 1)) {
        l.set(i, 0x59e2);
        return i;
    }
    let had_to = t == b"had" && nx == b"to";
    if last && [0x39f2, 0x19f2].contains(&l.code(i + 1)) && !had_to {
        l.set(i, 0x59e2);
        return i;
    }
    if l.medial(i) {
        l.set(i, 0x59e2);
        return i;
    }
    if had_to {
        l.set(i, 0x59f2);
    }
    i
}

/// Words starting "ext…" whose t is kept after `s`.
fn ext_kept(t: &[u8]) -> bool {
    t == b"extant"
        || t.starts_with(b"external")
        || is(t, &[b"extrados", b"extradoses", b"extragalactic"])
        || t.starts_with(b"extravasat")
        || t.starts_with(b"extrem")
}

/// t 0x19f2 and 0x39f2 (LAB_10014b57).
fn tt(l: &mut L, i: usize, t: &[u8], nx: &[u8]) -> usize {
    const TON: [&[u8]; 13] = [
        b"boston", b"exton", b"gregson", b"houston", b"lipton", b"livingston", b"middleton", b"milton", b"palmerston",
        b"whiston", b"washington", b"wellington", b"wilmington",
    ];
    const MIGHT: [&[u8]; 6] = [b"might", b"that", b"put", b"get", b"got", b"expect"];
    if i != 0 {
        if l.schwa_n(i) && !is(t, &TON) && l.on_ons(t, i + 2) {
            return l.merge_schwa_n(i, 0x59f2);
        }
        if l.syllabic_n(i) {
            return l.before_syllabic_n(i, 0x59f2);
        }
    }
    if l.code(i + 1) == 0x1952 && contains(t, b"tz") {
        l.delete(i);
        l.set(i, 0x1912);
        return i;
    }
    if l.code(i + 1) == 0x1952 && l.same(i, i + 1) && !l.same(i + 1, i + 2) {
        l.delete(i);
        l.set(i, 0x1912);
        return i;
    }
    let last = !l.same(i, i + 1);
    if last {
        if is(t, &MIGHT) {
            if !l.pause(i + 1) {
                l.set(i, 0x59f2);
                return i;
            }
        } else if !l.pause(i + 1) && !is(nx, &YOU) {
            l.set(i, 0x59f2);
            return i;
        }
    }
    if l.medial(i) {
        l.set(i, 0x59f2);
        return i;
    }
    let c1 = l.code(i + 1);
    if l.code(i) == 0x19f2 {
        if (i == 0 || !l.same(i - 1, i)) && c1 != 0x1952 && c1 != 0x1752 && c1 & 0x1e3 != 0x1e2 {
            l.set(i, 0x39f2);
            return i;
        }
        if i == 0 {
            return 0;
        }
        if l.pause(i + 1) || c1 == 0x952 || (last && is(nx, &YOU)) {
            l.set(i, 0x39f2);
            return i;
        }
        if l.code(i - 1) == 0x1952 && !ext_kept(t) {
            return i;
        }
        if [0x1d52, 0x1752].contains(&l.code(i - 1)) || !l.same(i - 1, i) || l.pause(i + 1) {
            return i;
        }
        l.set(i, 0x39f2);
        return i;
    }
    // 0x39f2
    if i == 0 {
        return 0;
    }
    let pc = l.code(i - 1);
    let mut keep = !(pc == 0x1752 || pc == 0x1952) || !l.same(i - 1, i + 1);
    if !keep
        && i > 2
        && t.starts_with(b"ex")
        && [0x2ff2, 0xff2].contains(&l.code(i - 2))
        && [0x6c1, 0x106c1].contains(&l.code(i - 3))
        && !ext_kept(t)
    {
        keep = true;
    }
    if !keep && is(t, &[b"boston", b"houston", b"livingston", b"palmerston", b"whiston"]) {
        keep = true;
    }
    if !keep && c1 != 0x952 && c1 != 0x1952 {
        l.set(i, 0x19f2);
        return i;
    }
    // LAB_10015ae8
    if pc == 0x1d52 && l.same(i - 1, i + 1) && c1 != 0x1952 {
        l.set(i, 0x19f2);
    }
    i
}

/// k 0x1fe2.
fn k(l: &mut L, i: usize, t: &[u8]) -> usize {
    if i != 0 {
        if l.schwa_n(i) && !is(t, &[b"lisbon", b"alban", b"albans"]) && l.on_ons(t, i + 2) {
            return l.merge_schwa_n(i, 0x5fe2);
        }
        if l.syllabic_n(i) {
            return l.before_syllabic_n(i, 0x5fe2);
        }
    }
    if l.medial(i) || (!l.same(i, i + 1) && l.cons(i + 1) && ![0x952, 0x10c2, 0x401ec2].contains(&l.code(i + 1))) {
        l.set(i, 0x5fe2);
    }
    i
}

/// "ex…" words keeping their k before s.
fn exp_kept(t: &[u8]) -> bool {
    is(t, &[b"explicative", b"explicatory"])
        || t.starts_with(b"exploit")
        || t.starts_with(b"explorative")
        || t == b"expo"
        || t.starts_with(b"express")
        || t == b"expurgatory"
}

/// p 0x1ff2 and 0x3ff2.
fn kk(l: &mut L, i: usize, t: &[u8]) -> usize {
    if i != 0 {
        if l.schwa_n(i) && !is(t, &[b"weapon", b"weapons"]) && l.on_ons(t, i + 2) {
            return l.merge_schwa_n(i, 0x5ff2);
        }
        if l.syllabic_n(i) {
            return l.before_syllabic_n(i, 0x5ff2);
        }
    }
    if l.medial(i) || (!l.same(i, i + 1) && !l.pause(i + 1)) {
        l.set(i, 0x5ff2);
        return i;
    }
    let c1 = l.code(i + 1);
    if l.code(i) == 0x3ff2 {
        if i == 0 {
            return 0;
        }
        if ![0x1752, 0x1952].contains(&l.code(i - 1)) || !l.same(i - 1, i + 1) {
            return i;
        }
        let ex = i >= 3
            && t.starts_with(b"ex")
            && [0x2ff2, 0xff2].contains(&l.code(i - 2))
            && [0x6c1, 0x106c1].contains(&l.code(i - 3))
            && !t.starts_with(b"expectation")
            && !t.starts_with(b"expert");
        if ex && !exp_kept(t) {
            return i;
        }
        if c1 != 0x952 && c1 != 0x1952 {
            l.set(i, 0x1ff2);
        }
        return i;
    }
    if l.code(i) != 0x1ff2 {
        return i;
    }
    if (i == 0 || !l.same(i - 1, i)) && l.same(i, i + 1) && c1 != 0x1952 && c1 & 0x1e3 != 0x1e2 {
        l.set(i, 0x3ff2);
        return i;
    }
    if i == 0 {
        return 0;
    }
    if l.pause(i + 1) || c1 == 0x952 {
        l.set(i, 0x3ff2);
        return i;
    }
    if l.code(i - 1) == 0x1952 && !t.starts_with(b"expectation") && !t.starts_with(b"expert") && !exp_kept(t) {
        return i;
    }
    if l.code(i - 1) == 0x1752 || !l.same(i - 1, i) || !l.same(i, i + 1) {
        return i;
    }
    l.set(i, 0x3ff2);
    i
}

/// schwa 0x501.
fn schwa(l: &mut L, i: usize, t: &[u8]) -> usize {
    const SON: [&[u8]; 10] = [
        b"emerson", b"emmerson", b"ferguson", b"fergusson", b"jackson", b"jefferson", b"larson", b"livingston",
        b"nicholson", b"nicolson",
    ];
    if i != 0 {
        // "…tional": schwa + n + schwa + l
        if l.code(i - 1) == 0x1752
            && l.code(i + 1) == 0x19c2
            && l.code(i + 2) == 0x501
            && l.code(i + 3) == 0x18a2
            && !l.same(i + 3, i + 4)
            && l.same(i - 1, i + 3)
            && t != b"traditional"
        {
            l.delete(i);
            l.set(i, 0x219c2);
            return i + 2;
        }
        if l.code(i + 1) == 0x19c2
            && l.same(i - 1, i + 1)
            && l.code(i - 1) & 0x1e3 == 0x142
            && l.on_ons(t, i + 1)
            && !is(t, &SON)
        {
            l.delete(i);
            l.set(i, 0x219c2);
            return i;
        }
    }
    // LAB_10012e25: schwa + final l → syllabic l
    if l.code(i + 1) == 0x18a2
        && (!l.same(i + 1, i + 2) || (l.code(i + 2) == 0x1942 && !l.same(i + 2, i + 3)))
    {
        l.delete(i);
        l.set(i, 0x218a2);
    }
    i
}

/// p 0xfe2.
fn p(l: &mut L, i: usize, t: &[u8]) -> usize {
    if i != 0 {
        if l.schwa_n(i) && l.on_ons(t, i + 2) {
            return merge_moving_word(l, i, 0x4fe2);
        }
        if l.syllabic_n(i) {
            return l.before_syllabic_n(i, 0x4fe2);
        }
    }
    if l.medial(i) || (!l.same(i, i + 1) && l.cons(i + 1) && ![0x952, 0x10c2, 0x401ec2].contains(&l.code(i + 1))) {
        l.set(i, 0x4fe2);
    }
    i
}

/// LAB_100130c7: like merge_schwa_n, the syllabic n also taking the stop's word.
fn merge_moving_word(l: &mut L, i: usize, code: u32) -> usize {
    let r = l.merge_schwa_n(i, code);
    l.p[i + 1].word = l.p[i].word;
    r
}

/// l 0x18a2.
fn el(l: &mut L, i: usize, t: &[u8]) -> usize {
    if (contains(t, b"feeling") || t.starts_with(b"ceiling") || t.starts_with(b"ealing"))
        && i != 0
        && l.code(i - 1) == 0x107c5
        && l.code(i + 1) == 0x781
        && l.code(i + 2) == 0xfc2
    {
        l.set(i, 0x18a2);
        return i;
    }
    if (t.starts_with(b"milie") || t.starts_with(b"millie"))
        && i != 0
        && BEFORE_J.contains(&l.code(i - 1))
        && l.code(i + 1) == 0x10c2
    {
        l.set(i, 0x18a2);
        return i;
    }
    if i == 0 {
        return 0;
    }
    // -ing / -ing + l endings that close the l's syllable
    let ing = |l: &L| l.code(i + 1) == 0x781 && l.code(i + 2) == 0xfc2 && !l.same(i + 2, i + 3);
    let ingl = |l: &L| {
        l.code(i + 1) == 0x781 && l.code(i + 2) == 0xfc2 && l.code(i + 3) == 0x18a2 && !l.same(i + 4, i + 5)
    };
    if l.cons(i - 1) && (!l.same(i, i + 1) || !l.vowel(i + 1) || ing(l) || ingl(l)) {
        l.set(i, 0x218a2);
        return i;
    }
    if BEFORE_DARK_L.contains(&l.code(i - 1)) && l.same(i - 1, i) {
        let kept = l.same(i, i + 1) && (l.vowel(i + 1) || l.code(i + 1) == 0x10c2) && !ing(l) && !ingl(l);
        if !kept {
            l.set(i, 0x218a2);
            return i;
        }
    }
    // LAB_10013919: l + schwa ending the word, in -ealer words
    if l.code(i + 1) != 0x501 {
        return i;
    }
    if l.same(i + 1, i + 2) && (l.same(i + 2, i + 3) || l.code(i + 2) != 0x1942) {
        return i;
    }
    let ealer = [&b"dealer"[..], b"healer", b"sealer", b"squealer", b"stealer"].iter().any(|s| contains(t, s))
        || (contains(t, b"wheeler") && t != b"wheeler" && t != b"wheelers");
    if ealer {
        l.set(i, 0x218a2);
    }
    i
}

/// t 0xff2 and 0x2ff2 (LAB_10016016).
fn pp(l: &mut L, i: usize, t: &[u8]) -> usize {
    let exc = |t: &[u8]| t == b"excalibur" || t.starts_with(b"exclud") || t == b"excursus" || t == b"excursuses";
    if i != 0 {
        if l.schwa_n(i) && l.on_ons(t, i + 2) {
            return merge_moving_word(l, i, 0x4ff2);
        }
        if l.syllabic_n(i) {
            return l.before_syllabic_n(i, 0x4ff2);
        }
    }
    if l.medial(i) || (!l.same(i, i + 1) && !l.pause(i + 1)) {
        l.set(i, 0x4ff2);
        return i;
    }
    let c1 = l.code(i + 1);
    if l.code(i) == 0x2ff2 {
        if i == 0 {
            return 0;
        }
        if ![0x1752, 0x1952].contains(&l.code(i - 1)) || !l.same(i - 1, i + 1) {
            return i;
        }
        if i > 2
            && t.starts_with(b"ex")
            && [0x2ff2, 0xff2].contains(&l.code(i - 2))
            && [0x6c1, 0x106c1].contains(&l.code(i - 3))
            && !exc(t)
        {
            return i;
        }
        if c1 != 0x952 && c1 != 0x1952 {
            l.set(i, 0xff2);
        }
        return i;
    }
    if l.code(i) != 0xff2 {
        return i;
    }
    if (i == 0 || !l.same(i - 1, i)) && l.same(i, i + 1) && c1 != 0x1952 && c1 & 0x1e3 != 0x1e2 {
        l.set(i, 0x2ff2);
        return i;
    }
    if i == 0 {
        return 0;
    }
    if l.pause(i + 1) || c1 == 0x952 {
        l.set(i, 0x2ff2);
        return i;
    }
    if l.code(i - 1) == 0x1952 && !exc(t) {
        return i;
    }
    if l.code(i - 1) == 0x1752 || !l.same(i - 1, i) || !l.same(i, i + 1) {
        return i;
    }
    if l.cons(i + 1) && l.keeps_medial(i + 1) {
        return i;
    }
    l.set(i, 0x2ff2);
    i
}
