//! FonTrasConfini (0x1000e9e0): the voicing of a word's last consonant by
//! the word after it. The phrase loop calls it for every word of a chunk
//! and for every pause code, before the incoming codes are appended
//! (LoqTTS7.dll 0x1002d060); it reads the list so far through FON ctx +0x10
//! (`phones`, the count at its third argument), the incoming word's first
//! code (`next[0]`), the incoming record's text and the record before it
//! (its text, its boundary +0x11; 0x0f and `""` for a chunk's first word).
//!
//! Codes are the engine's feature words: `& 3` 2 a consonant, bit 0x10 set
//! a voiceless one.

use super::super::common::{field::BOUND, Phone, Word};
use super::Polish;

/// The words before which the last consonant changes when the next word
/// starts with a sonorant (`m n r l ł j`, not `rz`) or a vowel, each
/// `(list, count, from, to, at least)`: the list (`ELQSearchString` flag
/// 0x11, exact) holds the word before, `from` its last code, `at least`
/// the phones the list must hold (2 for `> 1`, 1 for `!= 0`).
/// `lúb` 0x1ff2 → 0x1fe2; `w wbréw przéciw naprzéciw` 0x1d52 → 0x1d42;
/// `ôd pôd …` 0x1bf2 → 0x1be2; `béz przéz pôprzez z` 0x1952 → 0x1942
/// (also before a voiced obstruent); `áż íż …` 0x2001752 → 0x2001742.
const BEFORE_SONORANT: [(u32, u32, u32, u32, usize); 5] = [
    (0x1003f024, 1, 0x1ff2, 0x1fe2, 2),
    (0x1003f02c, 4, 0x1d52, 0x1d42, 1),
    (0x1003f040, 9, 0x1bf2, 0x1be2, 2),
    (0x1003f068, 4, 0x1952, 0x1942, 1),
    (0x1003f07c, 6, 0x2001752, 0x2001742, 2),
];

/// The one-letter strings the code compares with `strncmp(…, 1)`:
/// `ł` (0x10024290), the vowels with their stressed forms `i í y ý e é a á
/// o ô u ú ó ę ě ą ă` (0x1003f230 …).
const L_STROKE: u32 = 0x10024290;
const VOWELS: [u32; 11] = [0x1003f230, 0x1003f22c, 0x1003f228, 0x1003f224, 0x1003f220, 0x1003f21c, 0x10020f14, 0x10020f48, 0x1003f218, 0x10020f60, 0x1003f214];
/// `í` (0x1003f230).
const I_ACUTE: u32 = 0x1003f230;

impl Polish {
    /// `m n r l ł j`, not `rz`.
    fn sonorant(&self, t: &[u8]) -> bool {
        let one = |c: u8| t.first() == Some(&c);
        (one(b'm') || one(b'n') || one(b'r') || one(b'l') || t.starts_with(self.s(L_STROKE)) || one(b'j')) && !t.starts_with(b"rz")
    }

    /// The vowels `i í y ý e é a á o ô u ú ó ę ě ą ă`.
    fn vowel_start(&self, t: &[u8]) -> bool {
        let plain = [b'i', b'y', b'e', b'a', b'o', b'u'];
        t.first().is_some_and(|c| plain.contains(c)) || VOWELS.iter().any(|&v| t.starts_with(self.s(v)))
    }

    /// The consonants after `i`/`í` that turn the 0x18a2 before the word
    /// into 0x2018a2 (`strncmp` with each string's length), in the code's
    /// order: its literals and the strings at 0x10024104 (`ć`), 0x1003f20c
    /// (`cí`), 0x1003f208 (`dź`), 0x1003f200 (`dzí`), 0x1003f1f8 (`dż`),
    /// 0x10024108 (`ż`), 0x10024298 (`ś`), 0x1003f1ec (`sí`), 0x10024294
    /// (`ź`), 0x1003f1e4 (`zí`), 0x1003f1e0 (`ní`).
    fn after_i(&self, t: &[u8]) -> bool {
        let s = |va: u32| self.s(va).to_vec();
        let lits: Vec<Vec<u8>> = vec![
            b"p".to_vec(),
            b"b".to_vec(),
            b"t".to_vec(),
            b"d".to_vec(),
            b"k".to_vec(),
            b"g".to_vec(),
            b"c".to_vec(),
            b"dz".to_vec(),
            s(0x10024104),
            b"ci".to_vec(),
            s(0x1003f20c),
            s(0x1003f208),
            b"dzi".to_vec(),
            s(0x1003f200),
            b"cz".to_vec(),
            s(0x1003f1f8),
            b"w".to_vec(),
            b"f".to_vec(),
            b"s".to_vec(),
            b"z".to_vec(),
            b"sz".to_vec(),
            b"rz".to_vec(),
            s(0x10024108),
            s(0x10024298),
            b"si".to_vec(),
            s(0x1003f1ec),
            s(0x10024294),
            b"zi".to_vec(),
            s(0x1003f1e4),
            b"h".to_vec(),
            b"ch".to_vec(),
            b"m".to_vec(),
            b"n".to_vec(),
            b"ni".to_vec(),
            s(0x1003f1e0),
            b"r".to_vec(),
            s(L_STROKE),
            b"l".to_vec(),
            b"j".to_vec(),
            b"x".to_vec(),
            b"v".to_vec(),
        ];
        lits.iter().any(|l| t.starts_with(l))
    }

    /// FonTrasConfini (0x1000e9e0) for chunk record `i` of `w` (the
    /// sentence from the chunk's first record): may recode the last phone
    /// of the list so far, add one, or merge the last two, before `next`
    /// is appended.
    pub(super) fn tras_confini(&self, w: &[Word], i: usize, phones: &mut Vec<Phone>, next: &mut Vec<u32>) {
        let text: &[u8] = w.get(i).map_or(&[], |r| r.text.as_slice());
        let prev: &[u8] = if i == 0 { &[] } else { w.get(i - 1).map_or(&[], |r| r.text.as_slice()) };
        let prev_bound = if i == 0 { 0x0f } else { w.get(i - 1).map_or(0, |r| r.f[BOUND]) };
        let first = next.first().copied().unwrap_or(0);
        // the incoming word starts with a voiced obstruent
        let voiced = first & 3 == 2 && first & 0x10 == 0;
        let n = phones.len();
        let last = |p: &Vec<Phone>| p.last().map_or(0, |p| p.code);
        let set = |p: &mut Vec<Phone>, c: u32| {
            if let Some(l) = p.last_mut() {
                l.code = c;
            }
        };
        if n > 1 {
            // a word ending in `j` + one letter, said as 0x7c1, before a
            // pause: a second 0x7c1 (ELQListAddElementAfter)
            if prev.len() != 1 && prev.len() >= 2 && prev[prev.len() - 2] == b'j' && last(phones) == 0x7c1 && matches!(prev_bound, 0x0a | 0x08 | 0x0e | 0x0c | 0x0d | 0x0b | 0x0f..=0x14) {
                phones.push(Phone {
                    word: (i as u32).wrapping_sub(1),
                    code: 0x7c1,
                    kind: 0x69,
                });
                return;
            }
        }
        let son_or_vowel = self.sonorant(text) || self.vowel_start(text);
        for (k, &(list, count, from, to, min)) in BEFORE_SONORANT.iter().enumerate() {
            if n >= min && last(phones) == from && self.search_string(prev, list, 0x11, count).is_some() && (son_or_vowel || (k == 3 && voiced)) {
                set(phones, to);
                return;
            }
        }
        // the incoming word starts with a voiced obstruent (not a sonorant,
        // or `rz`)
        let voicing = voiced && !self.sonorant(text);
        if n > 1 {
            let (p, l) = (phones[n - 2].code, phones[n - 1].code);
            // 0x100002e1 + 0x101003e1 at the end of the word before
            if p == 0x100002e1 && l == 0x101003e1 && (first == 0x1ff2 || first == 0x1fe2) {
                set(phones, 0x1fc2);
                return;
            }
            if p == 0x100002e1 && l == 0x101003e1 && (first == 0x401ec2 || first == 0x1003e1) {
                // the two become 0x2e1: the first recoded, the last deleted
                // (its kind copied to the one before, then
                // ELQListDeleteElement)
                phones[n - 2].code = 0x2e1;
                phones[n - 2].kind = phones[n - 1].kind;
                phones.pop();
                return;
            }
            if l == 0x1ff2 && voicing {
                set(phones, 0x1fe2);
                return;
            }
        }
        if n != 0 && last(phones) == 0x1d52 && voicing {
            set(phones, 0x1d42);
            return;
        }
        if n <= 1 {
            return;
        }
        let l = last(phones);
        let voice = [(0x1912, 0x1902), (0x1001712, 0x1001702), (0x1bf2, 0x1be2)];
        if let Some(&(_, to)) = voice.iter().find(|&&(from, _)| from == l) {
            if voicing {
                set(phones, to);
                return;
            }
        }
        if l == 0x19c2 && matches!(first, 0xfe2 | 0xff2 | 0x200fe2 | 0x200ff2) {
            set(phones, 0xfc2);
            return;
        }
        if l == 0x18a2 && (text.first() == Some(&b'i') || text.starts_with(self.s(I_ACUTE))) && self.after_i(&text[1..]) {
            set(phones, 0x2018a2);
            return;
        }
        let voice = [(0x1952, 0x1942), (0x1001752, 0x1001742), (0xff2, 0xfe2)];
        if let Some(&(_, to)) = voice.iter().find(|&&(from, _)| from == l) {
            if voicing {
                set(phones, to);
                return;
            }
        }
        if l == 0xff2 && matches!(first, 0x7c1 | 0x10c2) {
            set(phones, 0x200ff2);
            return;
        }
        if l == 0x2001752 && voicing {
            set(phones, 0x2001742);
            return;
        }
        if l == 0x2001752 && self.sonorant(text) {
            return;
        }
        if l == 0x2001712 && voicing {
            set(phones, 0x2001702);
        }
    }
}
