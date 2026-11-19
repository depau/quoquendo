//! `FonAllofonoLingua` (0x10017400) of LTTS7Danish.dll: the chunk's phone
//! list rewritten by context. One long cascade of rules on phone codes
//! inside a word (stød/schwa assimilation, r/ð vocalisation, lengthened
//! vowel + schwa…); the first rule that matches rewrites codes, deletes
//! phones (ELQListDeleteElement on ctx+0x54, which first copies the deleted
//! phone's kind onto the one before it) and gives the index the phrase loop
//! goes on from. Then, for a phone of a word whose tag holds `\SAMPA=`
//! (0x10027280, `strstr` 0x1001c550), a second pass (0x1001a093) marks the
//! phone at that index.
//!
//! Notation in the comments: `st` = the phone starts its word (index 0 or
//! the previous phone's word differs), `end n` = phone `p+n` belongs to
//! another word (the pattern closes its word), `free n` = also `p+n` is not
//! the pause 0x17/0x13. Reads outside the list give a phone of no word and
//! code 0 (the original reads stale array slots there; no traced pattern
//! reaches them).

use super::Danish;
use crate::lang::common::{Phone, Word};

/// `\SAMPA=` (0x10027280).
const SAMPA: u32 = 0x10027280;

fn ph_at(ph: &[Phone], k: isize) -> Phone {
    if k < 0 {
        return Phone { word: u32::MAX - 1, code: 0, kind: 0 };
    }
    ph.get(k as usize).copied().unwrap_or(Phone { word: u32::MAX, code: 0, kind: 0 })
}

struct Ctx<'a> {
    ph: &'a mut Vec<Phone>,
    p: isize,
}

impl Ctx<'_> {
    fn c(&self, n: isize) -> u32 {
        ph_at(self.ph, self.p + n).code
    }
    fn w(&self, n: isize) -> u32 {
        ph_at(self.ph, self.p + n).word
    }
    fn set(&mut self, n: isize, code: u32) {
        if let Some(x) = self.ph.get_mut((self.p + n) as usize) {
            x.code = code;
        }
    }
    /// Codes `p..p+codes.len()` are `codes`, all in phone `p`'s word.
    fn seq(&self, codes: &[u32]) -> bool {
        codes.iter().enumerate().all(|(k, &c)| self.c(k as isize) == c) && self.w(codes.len() as isize - 1) == self.w(0)
    }
    /// Phone `p` starts its word.
    fn st(&self) -> bool {
        self.p == 0 || self.w(0) != self.w(-1)
    }
    /// Phone `p+n` is in another word than phone `p+n-1`.
    fn end(&self, n: isize) -> bool {
        self.w(n) != self.w(n - 1)
    }
    /// `end n` and phone `p+n` is not the pause 0x17 / 0x13.
    fn free(&self, n: isize) -> bool {
        self.end(n) && !matches!(self.c(n), 0x17 | 0x13)
    }
    /// ELQListDeleteElement of phone `p+n` (its kind goes to the phone
    /// before); the index becomes `p+n-1`.
    fn del(&mut self, n: isize) -> isize {
        let d = (self.p + n) as usize;
        if d != 0 {
            self.ph[d - 1].kind = self.ph[d].kind;
        }
        self.ph.remove(d);
        self.p + n - 1
    }
    /// Deletes phone `p+n` twice (it and the one after it).
    fn del2(&mut self, n: isize) -> isize {
        self.del(n);
        self.del(n)
    }

    /// The rule cascade 0x10017400..0x1001a093; returns the new index.
    fn rules(&mut self) -> isize {
        let p = self.p;
        // a stressed vowel (code & 0x10003 == 0x10001) before a word-final
        // schwa 0x501 (the vowel's own word is not checked): its long form
        if self.c(0) & 0x10003 == 0x10001 && self.c(1) == 0x501 && self.w(1) != self.w(2) {
            let to = match self.c(0) {
                c @ (0x107c1 | 0x107e1 | 0x10741 | 0x106c1 | 0x106e1 | 0x10681 | 0x10441 | 0x10761 | 0x10241 | 0x102e1 | 0x10361 | 0x103e1) => c | 4,
                c => c,
            };
            self.set(0, to);
            return p;
        }
        // 0x100175a8: st 2ff2 10441 19c2 + end/free 3: drop the 19c2
        if self.st() && self.seq(&[0x2ff2, 0x10441, 0x19c2]) && self.free(3) {
            return self.del(2);
        }
        if self.st() && self.seq(&[0x2ff2, 0x441, 0x19c2]) && self.end(3) {
            return self.del(2);
        }
        // 0x100177..: st 2ff2 3e1 19c2 501 free 4: drop 19c2 501
        if self.st() && self.seq(&[0x2ff2, 0x3e1, 0x19c2, 0x501]) && self.free(4) {
            return self.del2(2);
        }
        // 0x10017892
        if self.st() && self.seq(&[0x20001ff2, 0x18a2, 0x741, 0x401ec2]) && self.free(4) {
            return self.del(3);
        }
        // 0x100179b5
        if self.st() && self.seq(&[0x1952, 0x20000ff2, 0x3e1, 0x18a2, 0x501]) && self.free(5) {
            return self.del2(3);
        }
        if self.st() && self.seq(&[0x1d42, 0x7c1, 0x18a2, 0x501]) && self.free(4) {
            return self.del(3);
        }
        // 0x10017c1d
        if self.st() && self.seq(&[0x1d42, 0x741, 0x18a2]) && self.free(3) {
            return self.del(2);
        }
        if self.st() && self.seq(&[0x200019f2, 0x501]) && self.end(2) {
            self.set(1, 0x741);
            return p + 1;
        }
        if self.st() && self.c(0) == 0x1952 {
            if self.seq(&[0x1952, 0x20000ff2, 0x10441, 0x18a2]) && self.free(4) {
                return self.del(3);
            }
            if self.seq(&[0x1952, 0x20000ff2, 0x441, 0x18a2]) && self.free(4) {
                return self.del(3);
            }
        }
        // 0x10017f25 (only past index 0)
        if p != 0 && self.st() && self.seq(&[0x200019f2, 0x501, 0x19c2]) && self.free(3) {
            self.set(1, 0x219c2);
            return self.del(2);
        }
        // 0x10017fea
        if self.st() && self.seq(&[0x39f2, 0x741, 0x18a2]) && self.free(3) {
            return self.del(2);
        }
        if self.st() && (self.seq(&[0x10c2, 0x241, 0x10c2]) || self.seq(&[0x1952, 0x241, 0x10c2])) && self.free(3) {
            return self.del(2);
        }
        if self.st() && self.seq(&[0x1fc2, 0x241, 0x10c2]) && self.end(3) {
            return self.del(2);
        }
        if self.st() && self.seq(&[0x1d42, 0x10441, 0x1b42]) && self.free(3) {
            return self.del(2);
        }
        if self.st() && self.seq(&[0x1952, 0x481]) && self.end(2) {
            self.set(1, 0x2c1);
            return p + 1;
        }
        if self.seq(&[0x1b42, 0x501, 0x1b42]) && self.end(3) {
            self.set(0, 0x21b42);
            return self.del2(1);
        }
        if self.seq(&[0x1b42, 0x1b42, 0x501]) && self.end(3) {
            self.set(1, 0x21b42);
            return self.del(2);
        }
        if self.seq(&[0x20001ff2, 0x501, 0x19c2]) && self.end(3) {
            self.set(1, 0x21fc2);
            return self.del(2);
        }
        // not word-initial
        if !self.st() && self.seq(&[0x1fc2, 0x501, 0x19c2]) && self.end(3) {
            self.set(0, 0x21fc2);
            return self.del2(1);
        }
        if self.seq(&[0x481, 0x501, 0x19c2]) && self.end(3) {
            self.set(1, 0x219c2);
            return self.del(2);
        }
        if self.seq(&[0x501, 0x19c2]) && self.end(2) && matches!(self.c(2), 0x20000ff2 | 0x2ff2) {
            self.set(0, 0x20fc2);
            return self.del(1);
        }
        // 0x100186f3: inside a word, unless the previous phone is a
        // word-initial 1fc2
        if !self.st() && ((p != 1 && self.w(-1) == self.w(-2)) || self.c(-1) != 0x1fc2) && self.seq(&[0x501, 0x19c2]) && self.end(2) {
            self.set(0, 0x219c2);
            return self.del(1);
        }
        if self.seq(&[0x20000ff2, 0x501, 0x19c2]) && self.end(3) {
            self.set(1, 0x20fc2);
            return self.del(2);
        }
        if self.c(0) == 0x501 {
            if self.seq(&[0x501, 0x1fc2]) && self.end(2) {
                self.set(0, 0x21fc2);
                return self.del(1);
            }
            if self.seq(&[0x501, 0xfc2]) && self.end(2) && p >= 1 && self.c(-1) == 0x20000ff2 && !self.st() {
                self.set(0, 0x20fc2);
                return self.del(1);
            }
            if self.seq(&[0x501, 0x18a2]) && self.end(2) {
                self.set(0, 0x218a2);
                return self.del(1);
            }
            if self.seq(&[0x501, 0xd42]) && self.end(2) {
                self.set(0, 0x481);
                return self.del(1);
            }
            if self.seq(&[0x501, 0x481]) {
                self.set(0, 0x481);
                return self.del(1);
            }
            // 0x100189e9
            if (self.seq(&[0x501, 0x1b42, 0x501]) || self.seq(&[0x501, 0x200019f2, 0x501])) && self.end(3) {
                self.set(0, 0x21b42);
                return self.del2(1);
            }
            if self.st() && self.seq(&[0x501, 0x1b42]) && self.free(2) {
                self.set(0, 0x741);
                self.set(1, 0x200019f2);
                return p + 1;
            }
            // 0x10018bed
            if self.seq(&[0x501, 0x1b42]) && self.end(2) && (p == 0 || self.c(-1) != 0x481) {
                self.set(0, 0x21b42);
                return self.del(1);
            }
            if self.seq(&[0x501, 0x10c2]) && self.end(2) {
                self.set(0, 0x10c2);
                return self.del(1);
            }
        }
        // 0x10018cb6: a word-final 200019f2 / 1b42 after 200019f2 741 (or
        // 10741) at its word's start goes
        if matches!(self.c(0), 0x200019f2 | 0x1b42) && self.free(1) && p >= 2 {
            let w2 = self.w(-2) == self.w(0) && self.c(-2) == 0x200019f2;
            if self.c(-1) == 0x741 && w2 && (p == 2 || self.w(-2) != self.w(-3)) {
                return self.del(0);
            }
            if self.c(-1) == 0x10741 && w2 {
                return self.del(0);
            }
        }
        if self.seq(&[0x1b42, 0x501, 0x200019f2]) && self.end(3) {
            self.set(0, 0x21b42);
            return self.del2(1);
        }
        if self.seq(&[0x501, 0x200019f2]) && self.end(2) {
            self.set(0, 0x21b42);
            return self.del(1);
        }
        if self.seq(&[0x7c1, 0x10c2, 0x501]) && self.end(3) {
            self.set(1, 0x7c1);
            return self.del(2);
        }
        if self.seq(&[0x481, 0x501]) {
            return self.del(1);
        }
        if self.seq(&[0xd42, 0x2c1]) && self.end(2) {
            self.set(0, 0x481);
            return self.del(1);
        }
        if self.seq(&[0x102c1, 0xd42]) {
            self.set(0, 0x102e5);
            return self.del(1);
        }
        if self.st() && self.seq(&[0x9f2, 0x2c1, 0x401ec2]) && self.free(3) {
            return self.del(2);
        }
        if self.st() && self.seq(&[0x2c1, 0x401ec2]) && self.free(2) {
            return self.del(1);
        }
        if self.st() && self.seq(&[0x9f2, 0x441, 0x200019f2]) && self.free(3) {
            self.set(2, 0x59f2);
            return p + 2;
        }
        if self.st() && self.seq(&[0x441, 0x200019f2]) && self.free(2) {
            self.set(1, 0x59f2);
            return p + 1;
        }
        // after a d42 of the same word
        let after_d42 = p >= 1 && self.c(-1) == 0xd42 && self.w(-1) == self.w(0);
        match self.c(0) {
            0x741 if after_d42 => {
                self.set(0, 0x6c1);
                return p;
            }
            0x10745 if after_d42 => {
                self.set(0, 0x106c5);
                return p;
            }
            0x745 if after_d42 => {
                self.set(0, 0x6c5);
                return p;
            }
            _ => {}
        }
        // a word-final 3e1 after a word-initial 2c1 (of two-phone words)
        if self.c(0) == 0x3e1 && p >= 1 && self.c(-1) == 0x2c1 && self.w(-1) == self.w(0) && (p == 1 || self.w(-1) != self.w(-2)) && self.free(1) {
            return self.del(0);
        }
        // 0x1001940a: a word-final 3e1 after a stressed vowel
        if self.c(0) == 0x3e1 && p >= 1 && self.c(-1) & 0x10003 == 0x10001 && self.w(-1) == self.w(0) && self.free(1) {
            return self.del(0);
        }
        let to = match self.c(0) {
            0x10765 => 0x106e5,
            0x10761 => 0x106e1,
            0x761 => 0x6e1,
            0x80010765 => 0x800106e5,
            0x80000765 => 0x800006e5,
            _ => 0,
        };
        if to != 0 && after_d42 {
            self.set(0, to);
            return p;
        }
        if self.c(0) == 0x1b42 && p >= 2 && matches!(self.c(-1), 0x741 | 0x10741) && self.c(-2) == 0x1d42 && self.w(-2) == self.w(0) && self.free(1) {
            return self.del(0);
        }
        if self.st() && self.seq(&[0x9f2, 0x441, 0x1b42]) && self.free(3) {
            self.set(1, 0x681);
            return self.del(2);
        }
        if self.st() && self.seq(&[0x441, 0x1b42]) && self.free(2) {
            self.set(0, 0x681);
            return self.del(1);
        }
        if self.c(0) == 0x1b42 && p >= 1 && self.c(-1) == 0x2c1 && self.w(-1) == self.w(0) && (p == 1 || self.w(-1) != self.w(-2)) && self.free(1) {
            return self.del(0);
        }
        if self.c(0) == 0x219c2 && self.end(1) && self.c(1) == 0x20001ff2 {
            self.set(0, 0x21fc2);
            return p;
        }
        if self.c(0) == 0x1d52 {
            if self.free(1) && p >= 1 && self.c(-1) == 0x681 && self.w(-1) == self.w(0) && (p == 1 || self.w(-1) != self.w(-2)) {
                return self.del(0);
            }
            if self.seq(&[0x1d52, 0x19c2]) && self.end(2) {
                self.set(1, 0x219c2);
                return p + 1;
            }
            if self.seq(&[0x1d52, 0x1fc2]) && self.end(2) {
                self.set(1, 0x21fc2);
                return p + 1;
            }
        }
        // 0x100199fc: a word-final consonant cluster's schwa between a
        // consonant with bit 0x10 and one without goes
        if self.c(0) == 0x501
            && p >= 1
            && self.c(-1) & 3 == 2
            && self.c(-1) & 0x10 != 0
            && self.c(1) & 3 == 2
            && self.c(1) & 0x10 == 0
            && self.w(-1) == self.w(0)
            && self.w(1) == self.w(0)
            && self.end(2)
        {
            return self.del(0);
        }
        let same_prev = p >= 1 && self.w(-1) == self.w(0);
        if self.c(0) == 0x10c2 && self.end(1) && same_prev && matches!(self.c(-1), 0x7c1 | 0x7e1 | 0x6e1 | 0x7c5 | 0x7e5 | 0x3e5) {
            return self.del(0);
        }
        if self.c(0) == 0x401ec2 && same_prev && ((self.end(1) && matches!(self.c(-1), 0x7c5 | 0x7e5 | 0x3e5)) || matches!(self.c(-1), 0x3e1 | 0x2e1)) {
            return self.del(0);
        }
        if self.c(0) == 0x19c2 {
            let to = match self.c(1) {
                0x1d52 | 0x1d42 => 0x1dc2,
                0x20001ff2 => 0x1fc2,
                0x20000ff2 | 0x2ff2 if self.end(1) => 0xfc2,
                _ => 0,
            };
            if to != 0 {
                self.set(0, to);
                return p;
            }
        }
        // 0x10019e19
        if self.c(0) == 0x1b42 && p >= 2 && self.c(-1) == 0x6c1 && self.w(-1) == self.w(0) && self.c(-2) == 0x1fc2 && self.w(-2) == self.w(0) && (p == 2 || self.w(-2) != self.w(-3)) && self.free(1) {
            return self.del(0);
        }
        // ff2 vowels before a consonant of class 0x1e2 of another word
        if self.c(1) & 0x1e3 == 0x1e2 && self.end(1) {
            match self.c(0) {
                0x20001ff2 => {
                    self.set(0, 0x5ff2);
                    return p;
                }
                0x20000ff2 => {
                    self.set(0, 0x4ff2);
                    return p;
                }
                _ => {}
            }
        }
        if self.c(0) == 0x10741
            && p >= 1
            && self.c(-1) == 0xd42
            && self.w(-1) == self.w(2)
            && ((self.c(1) == 0x1952 && matches!(self.c(2), 0x20000ff2 | 0x20001ff2 | 0x200019f2)) || (self.c(1) == 0x1d52 && self.c(2) == 0x200019f2))
        {
            self.set(0, 0x106c1);
        }
        p
    }

    /// 0x1001a093: under `\SAMPA=`, at the rules' index.
    fn sampa(&mut self) -> isize {
        let p = self.p;
        if p < 0 || p as usize >= self.ph.len() {
            return p;
        }
        let to = match self.c(0) {
            0x1ff2 => 0x20001ff2,
            0x19f2 => 0x200019f2,
            0xff2 => 0x20000ff2,
            _ => 0,
        };
        if to != 0 {
            self.set(0, to);
            return p;
        }
        // before a 9f2 of the same word: 0x80000000 marks the phone and
        // the 9f2 goes
        if self.c(1) == 0x9f2 && self.w(1) == self.w(0) {
            let to = match self.c(0) {
                c @ (0x401ec2 | 0x10c2 | 0x18a2 | 0xfc2 | 0x19c2 | 0x1fc2 | 0x1b42 | 0x107c5 | 0x7c5 | 0x107e5 | 0x7e5 | 0x10745 | 0x745 | 0x106c5 | 0x6c5 | 0x106e5 | 0x6e5 | 0x10685 | 0x685 | 0x10765 | 0x765 | 0x10245 | 0x245 | 0x102e5 | 0x2e5 | 0x10365 | 0x365 | 0x103e5 | 0x3e5) => c | 0x8000_0000,
                0xd42 => 0x80000485,
                0x10445 => 0x80010685,
                0x445 => 0x80000685,
                0x102c5 if self.end(2) => 0x80010485,
                0x2c5 if self.end(2) => 0x80000485,
                0x102c5 => 0x800102e5,
                0x2c5 => 0x800002e5,
                _ => 0,
            };
            if to != 0 {
                self.set(0, to);
                self.del(1);
                return p;
            }
        }
        let other = self.end(1);
        let to = match self.c(0) {
            0x10445 => 0x10685,
            0x445 => 0x685,
            0x2c1 | 0x2c5 if other => 0x481,
            0x102c1 | 0x102c5 if other => 0x10481,
            0x2c5 => 0x2e5,
            0x102c5 => 0x102e5,
            _ => return p,
        };
        self.set(0, to);
        p
    }
}

impl Danish {
    /// 0x10017400 at phone `i`; returns the index the phrase loop goes on
    /// from (wrapping to `usize::MAX` for -1).
    pub(super) fn allofono(&self, w: &[Word], ph: &mut Vec<Phone>, i: usize) -> usize {
        let sampa = self.s(SAMPA);
        let tagged = w
            .get(ph[i].word as usize)
            .and_then(|r| r.tag.as_deref())
            .is_some_and(|t| t.windows(sampa.len()).any(|x| x == sampa));
        let mut cx = Ctx { ph, p: i as isize };
        let p = cx.rules();
        if !tagged {
            return p as usize;
        }
        cx.p = p;
        cx.sampa() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::super::trace;
    use crate::lang::common::Word;

    #[test]
    fn allofono_replays_trace() {
        let Some(calls) = trace::calls() else { return };
        let Some(m) = trace::module() else { return };
        // the traces carry no tags: no record has `\SAMPA=`
        let words: Vec<Word> = Vec::new();
        let (mut ok, mut n) = (0, 0);
        for c in calls.iter().filter(|c| c.name == "FonAllofonoLingua" && !c.phones_out.is_empty()) {
            n += 1;
            let mut ph = c.phones_in.clone();
            let mut i = 0usize;
            while i < ph.len() - 1 {
                i = m.allofono(&words, &mut ph, i).wrapping_add(1);
            }
            if ph == c.phones_out {
                ok += 1;
            } else if n - ok <= 10 {
                let f = |v: &[crate::lang::common::Phone]| v.iter().map(|p| format!("{}:{:x}:{:x}", p.word, p.code, p.kind)).collect::<Vec<_>>().join(" ");
                eprintln!("IN  {}\nOUT {}\nGOT {}\n", f(&c.phones_in), f(&c.phones_out), f(&ph));
            }
        }
        eprintln!("allofono: {ok}/{n}");
        assert!(n > 0);
        assert!(ok * 10000 >= n * 9995, "{ok}/{n}");
    }
}
