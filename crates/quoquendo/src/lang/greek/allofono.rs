//! `FonAllofonoLingua` (0x1000b470) of LTTS7Greek.dll: per phone of the
//! chunk, with the words around it as Greek letters (0x10009910: the
//! previous record's, the phone's own, the next record's), rewrites at word
//! ends and assimilation.

use super::Greek;
use crate::lang::common::{field::GRADE, Phone, Word};

/// `στην`, `την`, `τον`, `των` (0x10022f84, `ELQSearchString` flag 0x11:
/// the whole word).
const ARTICLES: (u32, u32) = (0x10022f84, 4);
/// `μας`, `με`, `μου` (0x10022d4c).
const CLITICS: (u32, u32) = (0x10022d4c, 3);
/// `δέν` (0x10022ff0), `ότι` (0x10022fec), `κι` (0x10022fe8), `δέ`
/// (0x10022fe4).
const DEN: u32 = 0x10022ff0;
const OTI: u32 = 0x10022fec;
const KI: u32 = 0x10022fe8;
const DE: u32 = 0x10022fe4;

/// k g x before a front vowel or j (0x10741 0x741 0x107c1 0x7c1 0x10c2).
fn front(c: u32) -> bool {
    matches!(c, 0x10741 | 0x741 | 0x107c1 | 0x7c1 | 0x10c2)
}

impl Greek {
    fn word_is(&self, w: &[u8], (table, count): (u32, u32)) -> bool {
        (0..count).any(|k| w == self.s(self.pe.u32(table + 4 * k)))
    }

    /// Record `k` of the chunk as Greek letters (empty past the chunk).
    fn letters_of(&self, w: &[Word], k: usize) -> Vec<u8> {
        w.get(k).map_or_else(Vec::new, |r| self.greek_letters(&r.text, r.f[GRADE]))
    }

    /// 0x1000b470 at phone `i`; returns the index the phrase loop goes on
    /// from (one back after a deletion, but not below 0).
    pub(super) fn allofono(&self, w: &[Word], ph: &mut Vec<Phone>, i: usize) -> usize {
        let cur = ph[i];
        let word = cur.word as usize;
        let prev_word = if word == 0 { Vec::new() } else { self.letters_of(w, word - 1) };
        let this_word = self.letters_of(w, word);
        let next_word = if word + 1 < w.len() { self.letters_of(w, word + 1) } else { Vec::new() };
        let next = ph[i + 1];
        let prev = (i != 0).then(|| ph[i - 1]);
        let delete = |ph: &mut Vec<Phone>| {
            ph.remove(i);
            i.saturating_sub(1)
        };
        match cur.code {
            0x19c2 => {
                if cur.word != next.word && self.word_is(&prev_word, ARTICLES) && next.code == 0xff2 {
                    ph[i].code = 0xfc2;
                    ph[i + 1].code = 0xfe2;
                } else if this_word == self.s(DEN) && cur.word != next.word && matches!(next.code, 0x1ac2 | 0x1b52) {
                    return delete(ph);
                }
            }
            0x10c2 => {
                if let Some(p) = prev {
                    if p.code & 3 == 2 && p.code & 0x1e03 == 0x1002 {
                        return delete(ph);
                    }
                }
            }
            0x1952 => {
                if cur.word != next.word
                    && (self.word_is(&next_word, CLITICS) || matches!(next.code, 0x1ac2 | 0xec2 | 0x1942 | 0x19a2 | 0x1fe2 | 0x1d42 | 0x1902 | 0x1142))
                {
                    ph[i].code = 0x1942;
                }
            }
            0xff2 if front(next.code) => ph[i].code = 0x11f2,
            0xf52 if front(next.code) => ph[i].code = 0x1152,
            0xfe2 if front(next.code) => ph[i].code = 0x11e2,
            0x7c1 | 0x107c1 => {
                if this_word == self.s(KI) && cur.word != next.word && next.code & 3 == 1 && !matches!(next.code, 0x7c1 | 0x107c1) {
                    return delete(ph);
                }
            }
            0x10361 => {
                if w.len() > 2 && this_word == self.s(OTI) {
                    ph[i].code = 0x361;
                }
            }
            0x1bf2 | 0x1ff2 => {
                if let Some(p) = prev {
                    let (to, not) = if cur.code == 0x1bf2 { (0x1be2, [0x1942, 0x1952]) } else { (0x1fe2, [0x19a2, 0x1952]) };
                    if p.word != cur.word && cur.word == next.word && p.code == 0x19c2 && self.word_is(&prev_word, ARTICLES) && !not.contains(&next.code) {
                        ph[i].code = to;
                    }
                }
            }
            0x1fc2 => {
                if matches!(next.code, 0x1d52 | 0x1d42 | 0x1952 | 0x1942) {
                    ph[i].code = 0x1dc2;
                }
            }
            0x10741 => {
                if w.len() > 2 && (this_word == self.s(DEN) || this_word == self.s(DE)) {
                    ph[i].code = 0x741;
                }
            }
            _ => {}
        }
        i
    }
}
