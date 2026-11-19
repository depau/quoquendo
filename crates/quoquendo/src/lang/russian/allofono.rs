//! `FonAllofonoLingua` (0x1000d570), once per entry of a chunk's phone
//! list: inside a word, vowels after soft and hard consonants, `ий` at a
//! word's end, voicing assimilation before the next consonant, double
//! consonants said once; at a word's start, the voicing of the previous
//! word's last consonant (and `ы` → `и` after a one-phone word).
//! Phone codes are the engine's feature words: `& 3` 2 a consonant, 1 a
//! vowel; 0x10 voiceless; 0x200002 soft.

use super::Russian;
use crate::lang::common::{Phone, Word};

/// The sonorants (and `j`, `в`), which neither voice nor devoice what is
/// before them.
const SONORANTS: [u32; 10] = [0x19c2, 0x2019c2, 0x1fc2, 0x201fc2, 0x4018a2, 0x2018a2, 0x19a2, 0x2019a2, 0x10c2, 0x1d42];

fn consonant(c: u32) -> bool {
    c & 3 == 2
}
fn vowel(c: u32) -> bool {
    c & 3 == 1
}
fn voiceless(c: u32) -> bool {
    c & 0x10 != 0
}
fn soft(c: u32) -> bool {
    c & 0x200002 == 0x200002
}
/// A voiced obstruent other than `в`.
fn voices(c: u32) -> bool {
    consonant(c) && !voiceless(c) && !SONORANTS.contains(&c)
}
/// A voiced obstruent, `в` included.
fn voices_v(c: u32) -> bool {
    (consonant(c) && !voiceless(c) && !SONORANTS[..9].contains(&c)) || c == 0x1d42
}

impl Russian {
    /// `FonAllofonoLingua` (0x1000d570) on entry `i` of `ph` (the chunk's
    /// phones, words chunk-relative, then the closing sentinel); returns
    /// the index the phrase loop goes on from (one before after a
    /// deletion).
    pub(super) fn allofono(&self, w: &[Word], ph: &mut Vec<Phone>, i: usize) -> usize {
        let word = ph[i].word;
        // the record texts: this word's and the previous one's (0x1000d570)
        let text: Vec<u8> = w.get(word as usize).map(|r| r.text.clone()).unwrap_or_default();
        let prev_text: Vec<u8> = if word == 0 { Vec::new() } else { w.get(word as usize - 1).map(|r| r.text.clone()).unwrap_or_default() };
        // 0x1000d520: the phones of the word before this one
        let at_in_word = {
            let mut n = 0;
            let mut k = i;
            while k > 0 && ph[k].word == ph[k - 1].word {
                n += 1;
                k -= 1;
            }
            n
        };
        if (i == 0 || word != ph[i - 1].word) && ph[i].code == 0x781 {
            ph[i].code = 0x7c1;
        }
        let mut idx = i;
        if i >= 1 && ph[i].word == ph[i - 1].word {
            idx = self.inside(ph, i, &text, at_in_word);
        }
        self.across(ph, idx, &prev_text, &text)
    }

    /// The rules inside a word (the previous phone in the same word):
    /// the index after them.
    fn inside(&self, ph: &mut Vec<Phone>, i: usize, text: &[u8], at_in_word: usize) -> usize {
        let word = ph[i].word;
        let prev = ph[i - 1].code;
        let next = ph.get(i + 1).map_or(0, |p| p.code);
        let same_next = ph.get(i + 1).is_some_and(|p| p.word == word);
        let code = ph[i].code;
        let set = |ph: &mut Vec<Phone>, c: u32| {
            ph[i].code = c;
            i
        };
        let delete = |ph: &mut Vec<Phone>| {
            ph.remove(i);
            i - 1
        };
        if code == 0x10741 && consonant(prev) && !soft(prev) && prev != 0x10c2 {
            return set(ph, 0x106c1);
        }
        if code == 0x781 && !same_next && ((consonant(prev) && soft(prev)) || prev == 0x10c2) && text.last() == Some(&0xe5) {
            return set(ph, 0x501);
        }
        if code == 0x781 && consonant(prev) && soft(prev) {
            return set(ph, 0x7c1);
        }
        // ий at the end of a word
        if code == 0x10c2 && (prev == 0x7c1 || prev == 0x781) && !same_next && next != 0x17 && next != 0x13 {
            ph[i - 1].kind = ph[i].kind;
            return delete(ph);
        }
        if code == 0xf52 && same_next && next == 0x200ff2 {
            return set(ph, 0x200f52);
        }
        if code == 0xf52 && same_next && next == 0xfe2 {
            return set(ph, 0xf42);
        }
        if code == 0xfe2 && (prev == 0xf52 || prev == 0xf42) {
            ph[i - 1].kind = ph[i].kind;
            return delete(ph);
        }
        // voicing before the next consonant of the word
        let devoice = consonant(next) && voiceless(next) && same_next;
        let voice = voices(next) && same_next;
        let to = match code {
            0x19e2 if devoice => Some(0x19f2),
            0x1942 if devoice => Some(0x1952),
            0x1742 if devoice => Some(0x1752),
            0x2019f2 if voice => Some(0x2019e2),
            0x201ff2 if voice => Some(0x201fe2),
            0x201fe2 if devoice => Some(0x201ff2),
            0x2019e2 if devoice => Some(0x2019f2),
            0x200ff2 if voice => Some(0x200fe2),
            0x200fe2 if devoice => Some(0x200ff2),
            0x201d42 if devoice => Some(0x201d52),
            0x201d52 if voice => Some(0x201d42),
            0x201952 if voice => Some(0x201942),
            0x201942 if devoice => Some(0x201952),
            0x201752 if voice => Some(0x201742),
            0x201712 if voice => Some(0x201702),
            _ => None,
        };
        if let Some(c) = to {
            return set(ph, c);
        }
        if code == 0x19f2 && same_next && next == 0x2019f2 {
            return set(ph, 0x2019f2);
        }
        if code == 0x19c2 && same_next && next == 0x201712 {
            return set(ph, 0x2019c2);
        }
        // ссч → щ
        if code == 0x1952 && next == 0x1952 && ph.get(i + 2).is_some_and(|p| p.code == 0x201712 && p.word == word) && same_next {
            ph[i].code = 0x201752;
            ph[i].kind = ph[i + 1].kind;
            ph.remove(i + 1);
            ph[i].kind = ph[i + 1].kind;
            ph.remove(i + 1);
            return i;
        }
        if code == 0x201952 && same_next && next == 0x501 {
            return set(ph, 0x1952);
        }
        if code == 0x1ff2 && same_next && (next == 0x1ff2 || next == 0x201ff2) {
            return delete(ph);
        }
        if code == 0x1fe2 && same_next && (next == 0x1fe2 || next == 0x201fe2) {
            let keep = match at_in_word {
                1 => matches!(prev, 0x10361 | 0x2c1 | 0x501),
                2 => i >= 2 && ph[i - 2].code == 0x4018a2 && prev == 0x10361 && next == 0x201fe2 && ph.get(i + 2).is_some_and(|p| matches!(p.code, 0x7c1 | 0x781)),
                _ => false,
            };
            if !keep {
                return delete(ph);
            }
        }
        if code == 0x19f2 && same_next && (next == 0x19f2 || next == 0x2019f2) {
            let keep = match at_in_word {
                1 => matches!(prev, 0x10361 | 0x2c1 | 0x501),
                2 => i >= 2 && ph[i - 2].code == 0x200fe2 && matches!(prev, 0x10741 | 0x106c1 | 0x6c1) && next == 0x19f2 && ph.get(i + 2).is_some_and(|p| matches!(p.code, 0x2c1 | 0x501)),
                _ => false,
            };
            if !keep {
                return delete(ph);
            }
        }
        // double consonants said once
        let pair = |a: u32, b: u32| code == a && same_next && (next == a || next == b);
        if pair(0xff2, 0x200ff2) || pair(0xfe2, 0x200fe2) || pair(0x2019f2, 0x2019f2) || pair(0x1d52, 0x201d52) {
            return delete(ph);
        }
        if code == 0x1952 && same_next && matches!(next, 0x201752 | 0x1952 | 0x201952) {
            return delete(ph);
        }
        if code == 0x1742 && (prev == 0x1742 || prev == 0x201742) {
            return delete(ph);
        }
        if code == 0x1912 && same_next && next == 0x1912 {
            return set(ph, 0x19f2);
        }
        if pair(0x1fc2, 0x201fc2) || pair(0x4018a2, 0x2018a2) || pair(0x19a2, 0x2019a2) || pair(0x1752, 0x1752) {
            return delete(ph);
        }
        if code == 0x2019a2 && same_next && next == 0x19a2 {
            return delete(ph);
        }
        match code {
            0x341 => set(ph, if same_next { 0x501 } else { 0x2c1 }),
            0x741 => set(ph, 0x10741),
            0x441 => set(ph, 0x10441),
            0x9f2 => {
                let kind = ph[i].kind;
                if let Some(n) = ph.get_mut(i + 1) {
                    n.kind = kind;
                }
                delete(ph)
            }
            _ => i,
        }
    }

    /// LAB_1000ecad: at a word's first phone, the previous word's last
    /// phone (`p`) by this one (`c`).
    fn across(&self, ph: &mut [Phone], i: usize, prev_text: &[u8], text: &[u8]) -> usize {
        if i == 0 || ph[i].word == ph[i - 1].word {
            return i;
        }
        let snapshot = ph.to_vec();
        let (s, ph) = (&snapshot, ph);
        let p = s[i - 1].code;
        let c = s[i].code;
        let at = |k: isize| if k < 0 { None } else { s.get(k as usize) };
        let code = |k: isize| at(k).map_or(0, |e| e.code);
        // the phone `k` starts its word
        let starts = |k: isize| k == 0 || at(k).zip(at(k - 1)).is_some_and(|(a, b)| a.word != b.word);
        let i_ = i as isize;
        let voiced_or_vowel = (consonant(c) && !voiceless(c)) || vowel(c);
        let mut set_prev = |v: u32| {
            ph[i - 1].code = v;
            i
        };
        match p {
            0x501 => {
                let ao = |b: Option<&u8>| matches!(b, Some(0xee | 0xe0));
                if c == 0x2c1 && ao(prev_text.last()) && ao(text.first()) {
                    return set_prev(0x2c1);
                }
            }
            0x201ff2 if voices(c) => return set_prev(0x201fe2),
            0x1ff2 => {
                if matches!(code(i_ - 2), 0x501 | 0x2c1) && starts(i_ - 2) && voiced_or_vowel {
                    return set_prev(0x1fe2);
                }
                if voices(c) {
                    return set_prev(0x1fe2);
                }
            }
            0x19f2 => {
                if matches!(code(i_ - 2), 0x501 | 0x2c1) && matches!(code(i_ - 3), 0x19c2 | 0x1ff2) && starts(i_ - 3) && voiced_or_vowel {
                    return set_prev(0x19e2);
                }
                if code(i_ - 2) == 0x7c1 && code(i_ - 3) == 0x2019a2 && code(i_ - 4) == 0x1ff2 && starts(i_ - 4) && voiced_or_vowel {
                    return set_prev(0x1fe2);
                }
                if voices(c) {
                    return set_prev(0x19e2);
                }
            }
            0x2019f2 if voices(c) => return set_prev(0x2019e2),
            0x1912 if voices(c) => return set_prev(0x1902),
            0x201712 if voices(c) => return set_prev(0x201702),
            0xff2 if voices(c) => return set_prev(0xfe2),
            0x201d52 if voices_v(c) => return set_prev(0x201d42),
            0x201952 if voices_v(c) => return set_prev(0x201942),
            0x1952 => {
                let son = SONORANTS[..9].contains(&c);
                if starts(i_ - 1) && (vowel(c) || son) && s.get(i + 1).is_some_and(|n| n.word == s[i].word) {
                    return set_prev(0x1952);
                }
                if code(i_ - 2) == 0x7c1 && code(i_ - 3) == 0x201fe2 && starts(i_ - 3) && voiced_or_vowel {
                    return set_prev(0x1942);
                }
                if code(i_ - 2) == 0x107c1 && code(i_ - 3) == 0x2018a2 && code(i_ - 4) == 0x1fe2 && starts(i_ - 4) && voiced_or_vowel {
                    return set_prev(0x1942);
                }
                if matches!(code(i_ - 2), 0x7c1 | 0x5c1) && starts(i_ - 2) && voiced_or_vowel {
                    return set_prev(0x1942);
                }
                if voices(c) {
                    return set_prev(0x1942);
                }
                if c == 0x1742 {
                    return set_prev(0x1742);
                }
                if c != 0x1d42 && consonant(c) && !voiceless(c) && soft(c) && !SONORANTS[..9].contains(&c) {
                    return set_prev(0x201942);
                }
                if c == 0x1752 {
                    return set_prev(0x1752);
                }
                if c == 0x201712 || c == 0x201752 {
                    return set_prev(0x201752);
                }
            }
            0x201752 if voices(c) => return set_prev(0x201742),
            0x1752 if voices(c) => return set_prev(0x1742),
            0xf52 if voices(c) => return set_prev(0xf42),
            0x1712 if voices(c) => return set_prev(0x1702),
            0x5c1 => {
                if starts(i_ - 1) && ((consonant(c) && soft(c)) || c == 0x10c2) {
                    return set_prev(0x7c1);
                }
            }
            0x1d52 => {
                if consonant(c) && !voiceless(c) && !SONORANTS[..9].contains(&c) {
                    return set_prev(0x1d42);
                }
                if ((consonant(c) && !voiceless(c)) || vowel(c)) && starts(i_ - 1) {
                    return set_prev(0x1d42);
                }
                if c == 0x1d52 && s.get(i + 1).is_some_and(|n| n.word != s[i].word) {
                    let n = code(i_ + 1);
                    if (consonant(n) && !voiceless(n)) || vowel(n) {
                        return set_prev(0x1d42);
                    }
                }
            }
            _ => {}
        }
        // LAB_1000ff32: ы after a hard consonant
        if consonant(p) && !soft(p) && p != 0x10c2 && c == 0x7c1 {
            ph[i].code = 0x5c1;
        }
        i
    }
}
