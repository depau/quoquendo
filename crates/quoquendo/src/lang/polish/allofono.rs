//! FonAllofonoLingua (0x100105a0): run on every entry of a chunk's phone
//! list once the list is complete. A voiceless obstruent before 0x1942 or
//! 0x1d42 that is followed by a voiced obstruent or a vowel becomes
//! voiced, a voiced one before a voiceless obstruent voiceless, and 0x1be2
//! 0x1be2 across words gets a 0x501 between them. For the phones of a
//! word whose tag holds `\SAMPA=` (0x1003f238, `strstr` 0x10016c90) a
//! second set of rules marks palatal and glide variants.
//!
//! The module sees the list through the engine's indexed buffer (FON ctx
//! +0x10, one element pointer {word, code, kind} per entry) and edits it
//! with `ELQListFindElement` + `ELQListAddElementAfter` (an entry after
//! the current one) and `ELQListDeleteElement` on FON ctx +0x54. Codes are
//! the engine's feature words: `& 3` 1 a vowel, 2 a consonant; bit 0x10 of
//! a consonant set, voiceless.

use super::super::common::{Phone, Word};
use super::Polish;

/// `\SAMPA=` (0x1003f238).
const SAMPA: u32 = 0x1003f238;

/// The kind byte of an entry the function adds: its local element
/// `{word, code, kind}` (esp+0x10) sets only the word and the code, so the
/// kind is what the stack held there; 0xe3 in every traced call.
const STACK_KIND: u8 = 0xe3;

/// Voiceless → voiced before 0x1942/0x1d42 + a voiced obstruent or a vowel.
const VOICE: [(u32, u32); 10] = [
    (0x1ff2, 0x1fe2),
    (0x1952, 0x1942),
    (0xff2, 0xfe2),
    (0x1912, 0x1902),
    (0x1bf2, 0x1be2),
    (0x1d52, 0x1d42),
    (0x2001712, 0x2001702),
    (0x1001712, 0x1001702),
    (0x1001752, 0x1001742),
    (0x2001752, 0x2001742),
];

/// Voiced → voiceless before a voiceless obstruent.
const DEVOICE: [(u32, u32); 3] = [(0x1942, 0x1952), (0x1001742, 0x1001752), (0x2001742, 0x2001752)];

fn voiceless(c: u32) -> bool {
    c & 3 == 2 && c & 0x10 != 0
}

fn voiced(c: u32) -> bool {
    c & 3 == 2 && c & 0x10 == 0
}

impl Polish {
    /// FonAllofonoLingua (0x100105a0) on entry `i` of `phones` (the chunk's
    /// list with its closing sentinel; `chunk` its records, `sentence` the
    /// sentence's, `first` the chunk's first record in it); the index the
    /// engine continues after (it adds 1).
    pub(super) fn allofono(&self, sentence: &[Word], first: usize, _chunk: &[Word], phones: &mut Vec<Phone>, i: usize) -> usize {
        // entries past the list read as no word and code 0
        let at = |p: &Vec<Phone>, k: usize| p.get(k).copied().unwrap_or(Phone { word: u32::MAX, code: 0, kind: 0 });
        let (me, next, after) = (at(phones, i), at(phones, i + 1), at(phones, i + 2));
        let tag = sentence.get(first + me.word as usize).and_then(|r| r.tag.clone());
        let c = me.code;
        if let Some(&(_, to)) = VOICE.iter().find(|&&(from, _)| from == c) {
            if (next.code == 0x1942 || next.code == 0x1d42) && (voiced(after.code) || after.code & 3 == 1) {
                phones[i].code = to;
            }
        } else if let Some(&(_, to)) = DEVOICE.iter().find(|&&(from, _)| from == c) {
            if voiceless(next.code) {
                phones[i].code = to;
            }
        } else if c == 0x1be2 && me.word != next.word && next.code == 0x1be2 {
            // ELQListFindElement on this entry, ELQListAddElementAfter
            phones.insert(i + 1, Phone { word: me.word, code: 0x501, kind: STACK_KIND });
        }
        let Some(tag) = tag else { return i };
        let sampa = self.s(SAMPA);
        if !tag.windows(sampa.len()).any(|w| w == sampa) {
            return i;
        }
        let me = phones[i];
        let next = at(phones, i + 1);
        let after = at(phones, i + 2);
        let same = me.word == next.word;
        let c = me.code;
        macro_rules! set {
            ($code:expr) => {
                phones[i].code = $code
            };
        }
        match c {
            0x401ec2 => {
                if i == 0 {
                    return 0;
                }
                let p = phones[i - 1].code;
                if p & 3 != 1 || p == 0x102e1 || p == 0x2e1 || (same && next.code & 3 != 2) {
                    return i;
                }
                set!(0x1003e1);
            }
            0x100006c1 | 0x100002e1 | 0x100102e1 | 0x100106c1 => {
                // a vowel before another phone: 0x101003e1 after it
                let n = next.code;
                if n & 0x1e3 == 0x1c2 || matches!(n, 0x101003e1 | 0x101007c1 | 0x1003e1 | 0x1007c1) || (n == 0x1bf2 && after.code == 0x19c2) {
                    return i;
                }
                phones.insert(i + 1, Phone { word: me.word, code: 0x101003e1, kind: STACK_KIND });
            }
            0x1fc2 => {
                if same && matches!(next.code, 0x1d42 | 0x1d52) {
                    set!(0x1dc2);
                }
            }
            0x11c2 => {
                if same && next.code & 0x1e3 == 0x142 {
                    set!(0x101007c1);
                }
            }
            0x19c2 => {
                if i == 0 {
                    return 0;
                }
                let p = phones[i - 1].code;
                let n = next.code;
                let vowel_mark = matches!(p, 0x100106c1 | 0x100006c1 | 0x100102e1);
                if p & 3 == 1 && same && matches!(n, 0x1d42 | 0x1d52) {
                    set!(0x1dc2);
                } else if (vowel_mark && same && matches!(n, 0x1001712 | 0x1001702)) || (!vowel_mark && p != 0x100002e1 && same && matches!(n, 0x1001712 | 0x1001702) && after.code & 3 == 1 && after.code != 0x107c1) {
                    set!(0x2019c2);
                } else if p & 3 == 1 && same && n & 0x1e3 == 0x142 && !matches!(n, 0x1d42 | 0x1d52) {
                    set!(0x101003e1);
                }
            }
            0xfe2 => {
                if !same && next.code == 0x10c2 {
                    set!(0x200ff2);
                } else if matches!(next.code, 0x7c1 | 0x10c2) {
                    set!(0x200fe2);
                }
            }
            0xff2 => {
                if next.code == 0x10c2 {
                    set!(0x200ff2);
                }
            }
            0x10c2 => {
                if i == 0 {
                    return 0;
                }
                let prev = phones[i - 1];
                if prev.word == me.word && matches!(prev.code, 0x200ff2 | 0x200fe2) {
                    // ELQListDeleteElement on this entry
                    phones.remove(i);
                    return i.wrapping_sub(1);
                }
                if prev.code & 3 == 1 && (!same || next.code & 3 == 2) {
                    set!(0x1007c1);
                }
            }
            _ => {}
        }
        i
    }
}
