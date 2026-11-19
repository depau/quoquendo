//! FonAllofonoLingua (0x10009980): the chunk's phone list, phone by phone.

use super::super::common::{field::BOUND, Phone, Word};
use super::Dutch;

/// The tag that gives a word's phones in SAMPA (0x10009980 `strstr`).
const SAMPA: u32 = 0x10099ab4;

/// A stop or fricative not voiced (as FonTrasConfini's test).
fn voiceless_obstruent(c: u32) -> bool {
    matches!(c & 0x1e3, 0x1e2 | 0x142) && (c & 3 != 2 || c & 0x10 != 0)
}

impl Dutch {
    /// FonAllofonoLingua at phone `i` (records `sentence[first + word]`): in
    /// a word after a compound's part (the record before has boundary 4) a
    /// v/z after a voiceless obstruent devoiced; a glottal stop `Hg` before
    /// a stressed vowel after `a`, schwa or another word (an `R` ending that
    /// word becomes `HEhR`), before a vowel starting a compound's later
    /// part, and before a vowel after a word ending in `a` or schwa; `Aau`
    /// said `Oau`; under a `\SAMPA=` tag `Ah` → `Ou` and an `n` after a
    /// schwa (before an `s` ending the word, or ending its own) dropped,
    /// the schwa `HEh)`. Returns the index the phrase loop goes on from.
    pub(super) fn allofono(&self, sentence: &[Word], first: usize, phones: &mut Vec<Phone>, i: usize) -> usize {
        let word = phones[i].word as usize;
        let prev_bound = if word == 0 { 0x0f } else { sentence.get(first + word - 1).map_or(0, |r| r.f[BOUND]) };
        let tag = sentence.get(first + word).and_then(|r| r.tag.clone());
        if i > 1 && prev_bound == 4 {
            let x = phones[i - 1].code;
            if x & 0x1e3 == 0x142 && voiceless_obstruent(phones[i - 2].code) {
                match x {
                    0x1942 => phones[i - 1].code = 0x1952,
                    0x1d42 => phones[i - 1].code = 0x1d52,
                    _ => {}
                }
            }
        }
        if i != 0 {
            let cur = phones[i];
            let vowel = cur.code & 3 == 1;
            // the element is built in the function's frame with only its
            // record and code set: its kind byte is a stale local, 0xee in
            // every traced call (not a kind the phrase loop tests for)
            let glottal = |phones: &mut Vec<Phone>| {
                phones.insert(
                    i,
                    Phone {
                        word: cur.word,
                        code: 0x9e2,
                        kind: 0xee,
                    },
                )
            };
            let mut done = false;
            if vowel {
                let p = phones[i - 1];
                let other = p.word != cur.word;
                if cur.code & 0x10003 == 0x10001 && (p.code == 0x441 || p.code == 0x501 || other) {
                    if other && p.code == 0xd42 {
                        phones[i - 1].code = 0x200501;
                    }
                    glottal(phones);
                    done = true;
                } else if other && prev_bound == 4 {
                    glottal(phones);
                    done = true;
                }
            }
            if !done {
                let p = phones[i - 1];
                if p.word != cur.word && vowel && matches!(p.code, 0x441 | 0x501 | 0x10000501) {
                    glottal(phones);
                }
            }
        }
        match phones[i].code {
            0xba45 => phones[i].code = 0xbae5,
            0x1ba45 => phones[i].code = 0x1bae5,
            _ => {}
        }
        let sampa = self.s(SAMPA);
        if tag.is_some_and(|t| t.windows(sampa.len()).any(|w| w == sampa)) {
            match phones[i].code {
                0x2c1 => phones[i].code = 0x7a1,
                0x102c1 => phones[i].code = 0x107a1,
                0x19c2 if i != 0 => {
                    let w = |k: usize| phones.get(k).map_or(u32::MAX, |p| p.word);
                    let c = |k: usize| phones.get(k).map_or(0, |p| p.code);
                    let schwa_before = c(i - 1) == 0x501;
                    if w(i + 1) != w(i + 2) && schwa_before && c(i + 1) == 0x1952 {
                        phones[i - 1].code = 0x10000501;
                        phones.remove(i);
                        return i - 1;
                    }
                    if phones[i].word == w(i + 1) || !schwa_before {
                        return i;
                    }
                    phones[i - 1].code = 0x10000501;
                    phones.remove(i);
                    return i - 1;
                }
                _ => {}
            }
        }
        i
    }
}
