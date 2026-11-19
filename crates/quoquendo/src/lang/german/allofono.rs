//! FonAllofonoLingua (0x10012830): the engine runs it on every entry of a
//! chunk's phone list once the list is complete; it rewrites phones by
//! their neighbours, inserts and deletes them.
//!
//! The module sees the list through the engine's indexed buffer (FON ctx
//! +0x10, one element pointer {word, code, kind} per entry, rebuilt in place
//! by LTTS7Util after every insert or delete) and edits it with
//! `ELQListFindElement`/`ELQListAddElementAfter`/`ELQListDeleteElement` on
//! FON ctx +0x54: a delete shifts the later entries down, so the slot it read
//! holds the next entry afterwards. Codes are the engine's feature words
//! (`& 3`: 1 vowel, 2 consonant; `& 0x1e3` the manner class).

use super::super::common::{Phone, Word};
use super::German;

/// The module's phone symbols (`{entries, count}`, 8 bytes an entry), as
/// 0x1000cb00 reads them.
const SYMBOLS: u32 = 0x10029300;
/// Word endings 0x1000d040 compares the word's text with (13 entries of 8
/// bytes, the string first); mode 8: the text ends with the entry.
const ENDINGS: u32 = 0x10107948;
const ENDING_COUNT: u32 = 0xd;
/// Words the 0x19e2 rule leaves alone: "jèmanden" (0x10107f0c),
/// "nìemanden" (0x10107f00).
const JEMANDEN: u32 = 0x10107f0c;
const NIEMANDEN: u32 = 0x10107f00;
/// The 0x501 rule spares words starting with these 7 bytes ("mádchen+",
/// 0x10107f30).
const MADCHEN: u32 = 0x10107f30;
/// The 0x3ff2 rule spares this word ("jospin", 0x10107ef8).
const JOSPIN: u32 = 0x10107ef8;

/// Phone strings 0x1000d180 looks for 3 entries after the current one
/// (0x101030a0 …), each with the pair of entries (offsets from the current
/// one) whose words must differ for it to count: right after the string.
const SPELLINGS: [(u32, usize); 13] = [
    (0x101030a0, 4), // d HEh
    (0x10103098, 4), // th HEh
    (0x10103090, 4), // t HEh
    (0x10103088, 4), // HEh n
    (0x10103080, 5), // d HEh n
    (0x10103074, 6), // th l I c
    (0x1010306c, 6), // t l I c
    (0x10103064, 6), // d l I c
    (0x1010301c, 7), // d l I c HEh
    (0x10103054, 7), // th l I c HEh
    (0x10103048, 7), // t l I c HEh
    (0x10107f24, 7), // th l I c AR
    (0x10107f18, 7), // t l I c AR
];
/// The 0xfe2 rule's list (0x100131db) asks for "th l I c AR" twice and
/// never for "t l I c AR".
const SPELLINGS_FE2: usize = 12;

/// Codes renamed outright (the compare tree at 0x1001595c, 0x10015949).
const RENAMES: [(u32, u32); 12] = [
    (0x100106c1, 0x100106c5),
    (0x100102e1, 0x10010365),
    (0x100006c1, 0x100006c5),
    (0x10010241, 0x10010445),
    (0x100002e1, 0x10000365),
    (0x1f445, 0x1e445),
    (0x10000241, 0x10000445),
    (0x1eae5, 0x1e2e5),
    (0xf445, 0xe445),
    (0x1bc45, 0x1ac45),
    (0xeae5, 0xe2e5),
    (0xbc45, 0xac45),
];

/// Kind byte of the 0x9f2 the module inserts before a vowel (0x10012a37):
/// it never sets it, so the element gets whatever its caller left in the
/// stack slot below the return address. Untraced that is the engine's
/// `ELQListGetNumberOfElements` call's `push esi` (the phrase loop's sentence
/// pointer, 0x1002c98c); in the traces the tracer's thunk (`pushal`, its own
/// address in EAX), whose low byte the fixtures hold.
const STALE_KIND: u8 = 0x90;
/// What a slot past the list's end points to: a recycled element, reset by
/// the list's release callback (LoqTTS7 0x1002d020).
const GHOST: Phone = Phone { word: 0, code: 0x10, kind: 0x6e };

/// The previous phone of the 0x19e2/0xfe2/0x1fe2 rules: manner class 0x1e2
/// or one of these codes.
const PAL: [u32; 8] = [0x1752, 0x1952, 0x1152, 0x1d52, 0xf52, 0x1912, 0x1702, 0x1d12];
/// Consonants that don't count as the next phone of those rules.
const NOT_NEXT: [u32; 5] = [0x1d42, 0x952, 0x10c2, 0x18a2, 0xd42];
/// Codes at 3 entries on that let a 0x501 + 0x19c2 merge go ahead when the
/// word goes on after them.
const AT3: [u32; 5] = [0x1952, 0x19f2, 0x39f2, 0x19e2, 0x501];

/// The phone list as the module reads it through the indexed buffer.
struct List<'a>(&'a mut Vec<Phone>);

impl List<'_> {
    fn at(&self, k: usize) -> Phone {
        self.0.get(k).copied().unwrap_or(GHOST)
    }
    fn w(&self, k: usize) -> u32 {
        self.at(k).word
    }
    fn c(&self, k: usize) -> u32 {
        self.at(k).code
    }
    /// The module's guarded read: 0x10 for an index past the buffer (0x400).
    fn g(&self, k: usize) -> u32 {
        if k >= 0x400 {
            0x10
        } else {
            self.c(k)
        }
    }
    fn set(&mut self, k: usize, code: u32) {
        if let Some(p) = self.0.get_mut(k) {
            p.code = code;
        }
    }
    /// `ELQListDeleteElement` on slot `k`, then a new code for the entry the
    /// slot holds afterwards.
    fn del_set(&mut self, k: usize, code: u32) {
        self.0.remove(k);
        self.set(k, code);
    }
    fn pal(&self, k: usize) -> bool {
        let c = self.g(k);
        c & 0x1e3 == 0x1e2 || PAL.contains(&c)
    }
    fn next_consonant(&self, k: usize) -> bool {
        let c = self.g(k);
        c & 3 == 2 && !NOT_NEXT.contains(&c)
    }
}

impl German {
    /// FonAllofonoLingua (0x10012830) on entry `i` of `phones` (the chunk's
    /// list with its closing sentinel); the index the engine continues
    /// after (it adds 1).
    pub(crate) fn allofono(&self, w: &[Word], phones: &mut Vec<Phone>, i: usize) -> usize {
        let mut l = List(phones);
        // the entry's record (sentence +0x18 + word): grade +0x12, text +4,
        // and the previous record's boundary (+0x11; 0xf for the first). A
        // closing pause may name the record after the chunk's: only a vowel
        // uses them.
        let word = l.w(i);
        let rec = w.get(word as usize);
        let grade = rec.map_or(0, |r| r.f[0x12]);
        let text = rec.map_or(&[][..], |r| r.text.as_slice());
        let prev_bound = if word == 0 { 0x0f } else { w.get(word as usize - 1).map_or(0, |r| r.f[0x11]) };
        if i != 0 {
            let (p, n) = (l.at(i - 1), l.at(i + 1));
            // a glottal stop 0x9f2 before a vowel: at a word start after a
            // consonant with bit 0x10 (or 0x10c2) when the word is stressed
            // (grade 0 or 0x0c) or follows boundary 4; inside a word after
            // 0x501/0x481 unless it ends there
            let insert = if p.word != word {
                let pc = l.g(i - 1);
                (pc & 3 == 2 && pc & 0x10 != 0 || p.code == 0x10c2) && l.g(i) & 3 == 1 && (grade == 0 || grade == 0x0c || prev_bound == 4)
            } else {
                n.word == word && (p.code == 0x501 || p.code == 0x481) && l.g(i) & 3 == 1 && ![0x501, 0x1007c1, 0x481].contains(&l.g(i))
            };
            if insert {
                // ELQListFindElement on entry i - 1, ELQListAddElementAfter
                l.0.insert(i, Phone { word, code: 0x9f2, kind: STALE_KIND });
            }
        }
        let code = l.c(i);
        if let Some(&(_, to)) = RENAMES.iter().find(|&&(from, _)| from == code) {
            l.set(i, to);
            return i;
        }
        match code {
            // 0x6e1 + the high bits (0x1001595c): 0x7e5 and a 0x19c2 after it
            0x100106e1 | 0x100006e1 => {
                l.set(i, if code == 0x100106e1 { 0x107e5 } else { 0x7e5 });
                let word = l.w(i);
                l.0.insert(i + 1, Phone { word, code: 0x19c2, kind: 0x69 });
                i
            }
            0x19e2 => self.al_19e2(&mut l, i, text),
            0xfe2 => self.al_fe2(&mut l, i),
            0x1fe2 => self.al_1fe2(&mut l, i),
            // 0x10012f36
            0x19c2 => {
                if i != 0 && l.w(i - 1) == l.w(i) && l.g(i + 1) & 0x1e03 == 0xe02 {
                    l.set(i, 0xfc2);
                }
                i
            }
            // 0x10012ec0
            0xd42 => {
                if i != 0 && l.w(i - 1) == l.w(i) && l.g(i + 1) & 3 == 2 && l.c(i + 1) != 0x10c2 {
                    l.set(i, 0x481);
                }
                i
            }
            0x501 => self.al_501(&mut l, i, text),
            // 0x10012aca
            0x6c1 => {
                if i != 0 && l.c(i - 1) == 0x1d52 && l.c(i + 1) == 0x481 {
                    l.0.remove(i);
                }
                i
            }
            // 0x10012a9c: an entry 0x9f2 opening the list goes
            0x9f2 => {
                if i == 0 {
                    l.0.remove(0);
                }
                i
            }
            0xff2 | 0x2ff2 => self.al_ff2(&mut l, i),
            0x19f2 | 0x39f2 => self.al_19f2(&mut l, i),
            0x1ff2 | 0x3ff2 => self.al_1ff2(&mut l, i, text),
            _ => i,
        }
    }

    /// 0x19e2 (0x100133f9): 0x59f2 before 0x1942 of its word, inside a word
    /// after a `PAL` phone before a consonant (not 0x1942), at a word's end
    /// after a `PAL` phone before a consonant; the merge with a following
    /// 0x501 0x19c2 into 0x59e2 0x219c2, but not in "jèmanden", "nìemanden".
    fn al_19e2(&self, l: &mut List, i: usize, text: &[u8]) -> usize {
        let (w, n) = (l.w(i), l.at(i + 1));
        if w == n.word && n.code == 0x1942 {
            l.set(i, 0x59f2);
            return i;
        }
        if i == 0 {
            return 0;
        }
        let pw = l.w(i - 1);
        if pw == w && w == n.word && l.pal(i - 1) && l.next_consonant(i + 1) && n.code != 0x1942 {
            l.set(i, 0x59f2);
            return i;
        }
        if w != n.word {
            if l.pal(i - 1) && l.next_consonant(i + 1) {
                l.set(i, 0x59f2);
            }
            return i;
        }
        if !self.merge_ready(l, i) || !self.spelled(l, i, &SPELLINGS) {
            return i;
        }
        if text == self.s(JEMANDEN) || text == self.s(NIEMANDEN) {
            return i;
        }
        l.set(i, 0x59e2);
        l.del_set(i + 1, 0x219c2);
        i + 1
    }

    /// 0xfe2 (0x10012f87): 0x4ff2 after a `PAL` phone of its word before a
    /// consonant; the merge with a following 0x501 0x19c2 into 0x4fe2
    /// 0x219c2.
    fn al_fe2(&self, l: &mut List, i: usize) -> usize {
        if i == 0 {
            return 0;
        }
        let (w, pw) = (l.w(i), l.w(i - 1));
        if pw == w && l.pal(i - 1) && l.next_consonant(i + 1) {
            l.set(i, 0x4ff2);
            return i;
        }
        if !self.merge_ready(l, i) || !self.spelled(l, i, &SPELLINGS[..SPELLINGS_FE2]) {
            return i;
        }
        l.set(i, 0x4fe2);
        l.del_set(i + 1, 0x219c2);
        i + 1
    }

    /// 0x1fe2 (0x10013ab0): with a following 0x1d52 of its word after a
    /// `PAL` phone, 0x1d12 in place of both; else as 0xfe2 with 0x5ff2, and
    /// the merge with a following 0x501 0x19c2 into 0x5fe2 0x21fc2.
    fn al_1fe2(&self, l: &mut List, i: usize) -> usize {
        let (w, n) = (l.w(i), l.at(i + 1));
        if w == n.word && n.code == 0x1d52 {
            if i == 0 {
                return 0;
            }
            if l.pal(i - 1) {
                l.del_set(i, 0x1d12);
                return i;
            }
        }
        if i == 0 {
            return 0;
        }
        let pw = l.w(i - 1);
        if pw == w && w == n.word && l.pal(i - 1) && l.next_consonant(i + 1) && n.code != 0x1d52 {
            l.set(i, 0x5ff2);
            return i;
        }
        if w != n.word {
            if l.pal(i - 1) && l.next_consonant(i + 1) {
                l.set(i, 0x5ff2);
            }
            return i;
        }
        if !self.merge_ready(l, i) || !(self.after_19c2(l, i) || self.any_spelling(l, i, &SPELLINGS)) {
            return i;
        }
        l.set(i, 0x5fe2);
        l.del_set(i + 1, 0x21fc2);
        i + 1
    }

    /// 0x501 (0x10012b14): it goes before 0x19c2 (which becomes 0x219c2)
    /// after a 0x102/0x142 class phone of its word, before 0x1fc2 (0x21fc2)
    /// after 0x1942, before 0x18a2 (0x218a2) at a word's end or before a
    /// final 0x19c2/0x39f2/0x19f2/0x1952 or 0x39f2/0x19f2 + a final 0x501.
    fn al_501(&self, l: &mut List, i: usize, text: &[u8]) -> usize {
        let (w, n) = (l.w(i), l.at(i + 1));
        if n.code == 0x19c2 && i != 0 {
            let p = l.at(i - 1);
            if p.word == w && w == n.word {
                let c = l.g(i - 1) & 0x1e3;
                let fricative = c == 0x102 || c == 0x142 && p.code != 0x952 && p.code != 0xd42;
                if fricative && self.not_after_cluster(l, i, p.word) {
                    if !strncmp_eq(text, self.s(MADCHEN), 7) && (n.word != l.w(i + 2) || self.ending(text) != 0) {
                        l.del_set(i, 0x219c2);
                        return i;
                    }
                }
            }
        }
        if l.c(i + 1) == 0x1fc2 {
            if i == 0 {
                return 0;
            }
            let (nw, p) = (l.w(i + 1), l.at(i - 1));
            if w == nw && p.word == w && p.code == 0x1942 && self.not_after_cluster(l, i, p.word) && (nw != l.w(i + 2) || self.ending(text) != 0) {
                l.del_set(i, 0x21fc2);
                return i;
            }
        }
        if i == 0 {
            return 0;
        }
        let (p, n) = (l.at(i - 1), l.at(i + 1));
        if n.code != 0x18a2 {
            return i;
        }
        if p.word == w && w == n.word && n.word != l.w(i + 2) {
            l.del_set(i, 0x218a2);
            return i;
        }
        if w == n.word && p.word == w {
            let s2 = l.at(i + 2);
            if [0x19c2, 0x39f2, 0x19f2, 0x1952].contains(&s2.code) && n.word == s2.word && s2.word != l.w(i + 3) {
                l.del_set(i, 0x218a2);
                return i;
            }
        }
        if p.word != w || w != n.word {
            return i;
        }
        let (s2, s3) = (l.at(i + 2), l.at(i + 3));
        if (s2.code == 0x39f2 || s2.code == 0x19f2) && s2.word == s3.word && s3.code == 0x501 && s3.word != l.w(i + 4) {
            l.del_set(i, 0x218a2);
        }
        i
    }

    /// 0xff2 and 0x2ff2 (0x100141da): 0x4ff2 by the `head` rules (without
    /// the 0x1d42 case) or the merge; else between the two (0x1001465e).
    fn al_ff2(&self, l: &mut List, i: usize) -> usize {
        let (w, n) = (l.w(i), l.at(i + 1));
        if i != 0 && l.w(i - 1) == w && w == n.word {
            if l.g(i - 1) & 0x1e3 != 0x1e2 && l.g(i + 1) & 0x1e3 == 0x1c2 || matches!(l.g(i + 1) & 0x1e3, 0x1e2 | 0x102) {
                l.set(i, 0x4ff2);
                return i;
            }
        }
        if self.before_other_word(l, i, w, n) {
            l.set(i, 0x4ff2);
            return i;
        }
        if let Some(r) = self.merge_501(l, i, 0x4ff2, 0x219c2, false) {
            return r;
        }
        if l.c(i) == 0x2ff2 {
            down(l, i, &[0x1d42, 0x952, 0xd42], 0xff2, false)
        } else {
            up(l, i, &[0x1952, 0x1752], 0x2ff2)
        }
    }

    /// 0x19f2 and 0x39f2 (0x100151c5): 0x59f2 by the `head` rules or the
    /// merge; before a final 0x1952 both become 0x1912; else between the two
    /// (0x1001562b).
    fn al_19f2(&self, l: &mut List, i: usize) -> usize {
        if let Some(r) = self.head(l, i, 0x59f2) {
            return r;
        }
        if let Some(r) = self.merge_501(l, i, 0x59f2, 0x219c2, false) {
            return r;
        }
        let n = l.at(i + 1);
        if n.code == 0x1952 && n.word != l.w(i + 2) {
            l.del_set(i, 0x1912);
            return i;
        }
        if l.c(i) == 0x39f2 {
            down(l, i, &[0x1d42, 0x952, 0xd42], 0x19f2, false)
        } else {
            up(l, i, &[0x1952, 0x1752], 0x39f2)
        }
    }

    /// 0x1ff2 and 0x3ff2 (0x100149b0): before 0x1d52 of its word both
    /// become 0x1d12; 0x5ff2 by the `head` rules or the merge (into 0x5ff2
    /// 0x21fc2); else between the two (0x10014e3c).
    fn al_1ff2(&self, l: &mut List, i: usize, text: &[u8]) -> usize {
        let (w, n) = (l.w(i), l.at(i + 1));
        if w == n.word && n.code == 0x1d52 {
            l.del_set(i, 0x1d12);
            return i;
        }
        if let Some(r) = self.head(l, i, 0x5ff2) {
            return r;
        }
        if let Some(r) = self.merge_501(l, i, 0x5ff2, 0x21fc2, true) {
            return r;
        }
        if l.c(i) == 0x3ff2 {
            down(l, i, &[0x952, 0x1d42, 0xd42, 0x1d52], 0x1ff2, text == self.s(JOSPIN))
        } else {
            up(l, i, &[0x1952, 0x1752, 0x1d52], 0x3ff2)
        }
    }

    /// The first rules of 0x19f2 and 0x1ff2: `to` inside a word before
    /// 0x1c2/0x1d42 (not after 0x1e2) or before 0x1e2/0x102, and before
    /// another word starting with one of those classes.
    fn head(&self, l: &mut List, i: usize, to: u32) -> Option<usize> {
        let (w, n) = (l.w(i), l.at(i + 1));
        if i != 0 && l.w(i - 1) == w && w == n.word {
            let (pc, nc) = (l.g(i - 1) & 0x1e3, l.g(i + 1) & 0x1e3);
            if pc != 0x1e2 && (nc == 0x1c2 || n.code == 0x1d42) || nc == 0x1e2 || nc == 0x102 {
                l.set(i, to);
                return Some(i);
            }
        }
        if self.before_other_word(l, i, w, n) {
            l.set(i, to);
            return Some(i);
        }
        None
    }

    /// The next phone starts another word with class 0x1e2, 0x1c2 or 0x102,
    /// or is 0x1d52, 0x1d42, 0x1942 or 0x1152.
    fn before_other_word(&self, l: &List, i: usize, w: u32, n: Phone) -> bool {
        w != n.word && (matches!(l.g(i + 1) & 0x1e3, 0x1e2 | 0x1c2 | 0x102) || [0x1d52, 0x1d42, 0x1942, 0x1152].contains(&n.code))
    }

    /// The merge of the next 0x501 and the 0x19c2 after it into the entry
    /// (the entry becomes `to`, the 0x501 goes, the 0x19c2 becomes `next`).
    /// `strict`: the 0x1ff2 rule's test at 3 entries on.
    fn merge_501(&self, l: &mut List, i: usize, to: u32, next: u32, strict: bool) -> Option<usize> {
        if i == 0 || !self.merge_ready(l, i) {
            return None;
        }
        let go = if strict {
            self.after_19c2(l, i) || self.any_spelling(l, i, &SPELLINGS)
        } else {
            self.spelled(l, i, &SPELLINGS)
        };
        if !go {
            return None;
        }
        l.set(i, to);
        l.del_set(i + 1, next);
        Some(i + 1)
    }

    /// Entry `i` (not the first) is followed by 0x501 and 0x19c2 of its
    /// word, and unless the word goes back past entry `i` - 1, that one isn't
    /// a consonant.
    fn merge_ready(&self, l: &List, i: usize) -> bool {
        let (w, pw, n, s2) = (l.w(i), l.w(i - 1), l.at(i + 1), l.at(i + 2));
        if w != n.word || n.code != 0x501 || s2.code != 0x19c2 || n.word != s2.word {
            return false;
        }
        !((i == 1 || pw != w) && l.g(i.wrapping_sub(1)) & 3 == 2)
    }

    /// The common test at 3 entries on: entry `i` + 3 ends its word, or is
    /// one of `AT3` with the word ending after it, or one of `pats` ends a
    /// word there.
    fn spelled(&self, l: &List, i: usize, pats: &[(u32, usize)]) -> bool {
        let w3 = l.w(i + 3);
        l.w(i + 2) != w3 || AT3.contains(&l.g(i + 3)) && w3 != l.w(i + 4) || self.any_spelling(l, i, pats)
    }

    /// The 0x1ff2/0x1fe2 test at 3 entries on: entry `i` + 3 is no 0x19c2
    /// after a word boundary, or one of `AT3` ending its word before
    /// something other than 0x19c2.
    fn after_19c2(&self, l: &List, i: usize) -> bool {
        let s3 = l.at(i + 3);
        if l.w(i + 2) != s3.word {
            s3.code != 0x19c2
        } else {
            let s4 = l.at(i + 4);
            AT3.contains(&l.g(i + 3)) && s3.word != s4.word && s4.code != 0x19c2
        }
    }

    fn any_spelling(&self, l: &List, i: usize, pats: &[(u32, usize)]) -> bool {
        pats.iter().any(|&(pat, k)| self.spells(l, pat, i + 3) && l.w(i + k) != l.w(i + k + 1))
    }

    /// The 0x501 rules' context: entry `i` - 2 belongs to the word of entry
    /// `i` - 1 (`pw`) and is not a consonant opening it (or the list).
    fn not_after_cluster(&self, l: &List, i: usize, pw: u32) -> bool {
        if i < 2 || l.w(i - 2) != pw {
            return false;
        }
        let starts = i == 2 || l.w(i - 3) != l.w(i - 2);
        !(starts && l.w(i - 2) == pw && l.g(i - 2) & 3 == 2)
    }

    /// 0x1000d180: whether the phones from entry `k` spell `pat` (codes by
    /// 0x1000cb90), each in the word of the entry before it. Its loop steps
    /// 1, 2, 3… entries, so it only checks entries k, k+1, k+3, … up to k +
    /// the number of symbols.
    fn spells(&self, l: &List, pat: u32, k: usize) -> bool {
        let s = self.s(pat);
        if s.is_empty() {
            return false;
        }
        let end = k + s.iter().filter(|&&c| c == b' ').count() + 1;
        let Some(codes) = self.symbol_codes(s) else { return false };
        let (mut k, mut n) = (k, 0usize);
        while k <= end {
            let e = l.at(k);
            if codes.get(n).copied().unwrap_or(0) != e.code || k > 0 && l.w(k - 1) != e.word {
                return false;
            }
            n += 1;
            k += n;
        }
        true
    }

    /// 0x1000cb90: the codes of the symbols of `s` (split at ' ' and '-',
    /// at most 5 bytes each, the byte after the fifth skipped), 0
    /// terminated; None when one is unknown.
    fn symbol_codes(&self, s: &[u8]) -> Option<Vec<u32>> {
        let mut out = Vec::new();
        let mut ok = true;
        let mut j = 0;
        while j < s.len() {
            let start = j;
            while j < s.len() && j - start < 5 && s[j] != b' ' && s[j] != b'-' {
                j += 1;
            }
            let c = self.symbol_code(&s[start..j]);
            if j < s.len() {
                j += 1;
            }
            ok &= c != 0x48;
            out.push(c);
        }
        out.push(0);
        ok.then_some(out)
    }

    /// 0x1000cb00 over the symbol table: the first entry spelled `sym`; a
    /// code 0 past the fourth entry counts as unknown (0x48).
    fn symbol_code(&self, sym: &[u8]) -> u32 {
        if sym.is_empty() {
            return 0x48;
        }
        let entries = self.pe.u32(SYMBOLS);
        let n = self.pe.u16(SYMBOLS + 4) as u32;
        for u in 0..n {
            if self.s(self.pe.u32(entries + 8 * u)) == sym {
                return match self.pe.u32(entries + 8 * u + 4) {
                    0 if u > 3 => 0x48,
                    c => c,
                };
            }
        }
        0x48
    }

    /// 0x1000d040 in mode 8 over `ENDINGS`: the first entry `text` ends
    /// with, or 0xffffffff.
    fn ending(&self, text: &[u8]) -> u32 {
        (0..ENDING_COUNT).find(|&k| text.ends_with(self.s(self.pe.u32(ENDINGS + 8 * k)))).unwrap_or(u32::MAX)
    }
}

/// From 0xff2/0x19f2/0x1ff2 to its other form `to` (0x2ff2/0x39f2/0x3ff2):
/// at a word's start unless before 0x1e2 class or `first`; inside a word not
/// after 0x1952/0x1752 before a phone of the word other than 0x1952 0x1752
/// 0x1d52 0x1942 0x1152 and 0x1e2/0x102 class; at a word's end before a
/// consonant other than 0x1e2/0x1c2/0x102 class and 0x142 class (but 0xf52,
/// 0x1742, 0xd42).
fn up(l: &mut List, i: usize, first: &[u32], to: u32) -> usize {
    let (w, n) = (l.w(i), l.at(i + 1));
    let same_prev = i != 0 && l.w(i - 1) == w;
    if !same_prev && !first.contains(&n.code) && l.g(i + 1) & 0x1e3 != 0x1e2 {
        l.set(i, to);
        return i;
    }
    if i == 0 {
        return 0;
    }
    let nc = l.g(i + 1);
    if same_prev
        && ![0x1952, 0x1752].contains(&l.c(i - 1))
        && w == n.word
        && ![0x1952, 0x1752, 0x1d52, 0x1942, 0x1152].contains(&n.code)
        && nc & 0x1e3 != 0x1e2
        && nc & 0x1e3 != 0x102
    {
        l.set(i, to);
        return i;
    }
    if w == n.word || matches!(nc & 0x1e3, 0x1e2 | 0x1c2 | 0x102) || nc & 3 == 1 {
        return i;
    }
    if nc & 0x1e3 == 0x142 && ![0xf52, 0x1742, 0xd42].contains(&n.code) {
        return i;
    }
    l.set(i, to);
    i
}

/// From 0x2ff2/0x39f2/0x3ff2 back to `to` (0xff2/0x19f2/0x1ff2): before
/// another word's 0x1952/0x1752/0x952 or vowel, before a 0x142 class phone
/// of the word (but `not142`), or inside a word after 0x1952/0x1752 (but
/// when `spared`).
fn down(l: &mut List, i: usize, not142: &[u32], to: u32, spared: bool) -> usize {
    let (w, n) = (l.w(i), l.at(i + 1));
    if w != n.word && [0x1952, 0x1752, 0x952].contains(&n.code) {
        l.set(i, to);
        return i;
    }
    if i != 0 {
        if w == n.word && l.g(i + 1) & 0x1e3 == 0x142 && !not142.contains(&n.code) {
            l.set(i, to);
            return i;
        }
        if l.w(i - 1) == w && w == n.word {
            let nc = l.g(i + 1) & 0x1e3;
            if [0x1952, 0x1752].contains(&l.c(i - 1)) && !matches!(nc, 0x1e2 | 0x102 | 0x1c2) && n.code != 0x1d42 && !spared {
                l.set(i, to);
            }
            return i;
        }
    }
    if w == n.word || l.g(i + 1) & 3 != 1 {
        return i;
    }
    l.set(i, to);
    i
}

/// `strncmp(a, b, n) == 0` on NUL-terminated strings.
fn strncmp_eq(a: &[u8], b: &[u8], n: usize) -> bool {
    for k in 0..n {
        let (x, y) = (a.get(k).copied().unwrap_or(0), b.get(k).copied().unwrap_or(0));
        if x != y {
            return false;
        }
        if x == 0 {
            break;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::*;

    /// Every traced chunk: its records and its phone list (with the
    /// sentinel) in, the list after the engine's loop out.
    #[test]
    #[ignore]
    fn allofono_matches_the_module() {
        let Some(de) = german() else { return };
        let (mut n, mut bad) = (0, 0);
        for line in lines("al.txt") {
            let c: Vec<&str> = line.split('\t').collect();
            let w = records(c[0]);
            let mut got = phones(c[1]);
            let want = phones(c.get(2).copied().unwrap_or(""));
            // the phrase loop 0x1002c300: the length is read again after every call
            let mut k = 0usize;
            while got.len() > 1 && k < got.len() - 1 {
                k = de.allofono(&w, &mut got, k).wrapping_add(1);
            }
            n += 1;
            if got != want {
                bad += 1;
                if bad <= 20 {
                    let sym = |v: &[super::super::super::common::Phone]| v.iter().map(|p| format!("{}:{}", p.word, de.symbol(p.code).map(str::to_string).unwrap_or(format!("{:x}", p.code)))).collect::<Vec<_>>().join(" ");
                    eprintln!("{:?}\n  in   {}\n  want {}\n  got  {}", w.iter().map(show).collect::<Vec<_>>(), sym(&phones(c[1])), sym(&want), sym(&got));
                }
            }
        }
        eprintln!("FonAllofonoLingua: {bad} of {n} chunks differ");
        assert_eq!(bad, 0);
    }
}
