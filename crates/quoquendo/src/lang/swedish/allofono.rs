//! FonAllofonoLingua (0x1000e560): the engine runs it on every entry of a
//! chunk's phone list once the list is complete; it merges a retroflex
//! cluster's /r/ into the following dental (0x19a2 r + 0x1952 t → 0x2001752
//! and so on), spreads the retroflex over a following dental, and deletes a
//! few phones of "åkk", "jag" and "det".
//!
//! The module sees the list through the engine's indexed buffer (FON ctx
//! +0x10, one element pointer {word, code, kind} per entry) and deletes
//! entries with `ELQListDeleteElement` (`[ctx+0x54]`, thunk 0x1000f19e): a
//! delete shifts the later entries down, so the slot it named holds the next
//! entry afterwards. The entry's record is the chunk's (ctx+0x28 + ctx+0x38
//! * 0x1c, +0x18) plus the entry's word, in the records at ctx+0x2c; its
//! text (+4) is compared with `ELQstricmp` (thunk 0x1000f0d2). The function
//! always returns the index it was given.

use super::super::common::{Phone, Word};
use super::Swedish;

/// Words the rules compare the entry's word with: "åkk" (0x1003d8e0), "jag"
/// (0x1001afd0), "för" (0x100291c4), "det" (0x1001b018).
const AKK: u32 = 0x1003d8e0;
const JAG: u32 = 0x1001afd0;
const FOR: u32 = 0x100291c4;
const DET: u32 = 0x1001b018;

/// The merged (retroflex) codes the module writes.
const RT: u32 = 0x2001752;
const RTS: u32 = 0x20017f2;
const RTJ: u32 = 0x20017e2;
const RN: u32 = 0x20017c2;
const RL: u32 = 0x20016a2;
/// The previous-phone list every merge rule walks, in the module's order.
const MERGED: [u32; 5] = [RT, RTS, RTJ, RN, RL];

/// What a slot past the list's end reads as: a recycled element, reset by
/// the list's release callback (as in the German port).
const GHOST: Phone = Phone { word: 0, code: 0x10, kind: 0x6e };

/// The plain dental a 0x19a2 merges with, and its merged code.
fn merged(c: u32) -> Option<u32> {
    match c {
        0x1952 => Some(RT),
        0x19f2 => Some(RTS),
        0x19e2 => Some(RTJ),
        0x18a2 => Some(RL),
        0x19c2 => Some(RN),
        _ => None,
    }
}

/// The phone list as the module reads it through the indexed buffer.
struct List<'a>(&'a mut Vec<Phone>);

impl List<'_> {
    fn at(&self, k: usize) -> Phone {
        self.0.get(k).copied().unwrap_or(GHOST)
    }
    fn set(&mut self, k: usize, code: u32) {
        if let Some(p) = self.0.get_mut(k) {
            p.code = code;
        }
    }
    fn del(&mut self, k: usize) {
        if k < self.0.len() {
            self.0.remove(k);
        }
    }
    /// The merge every dental shares (e.g. 0x1000e69d, 0x1000eab7): when
    /// the previous entry (`i - 1 < 0x400`) holds one of `MERGED`, either it
    /// is already `to` (it is deleted and the slot, now this entry, gets
    /// `to`) or, in the same word, this entry becomes `to`. `None` when the
    /// previous entry holds none of them (or `i` is 0).
    fn merge(&mut self, i: usize, word: u32, to: u32) -> Option<()> {
        if i == 0 || i - 1 >= 0x400 {
            return None;
        }
        let p = self.at(i - 1);
        if !MERGED.contains(&p.code) {
            return None;
        }
        if p.code == to {
            self.del(i - 1);
            self.set(i - 1, to);
        } else if p.word == word {
            self.set(i, to);
        }
        Some(())
    }
}

impl Swedish {
    /// `ELQstricmp` (thunk 0x1000f0d2) of the entry's record text with the
    /// string at `va`: equal ignoring case.
    fn text_is(&self, text: &[u8], va: u32) -> bool {
        let s = self.s(va);
        text.len() == s.len() && text.iter().zip(s).all(|(a, b)| self.lower(*a) == self.lower(*b))
    }

    /// FonAllofonoLingua (0x1000e560) on entry `i` of `phones` (the chunk's
    /// list, words chunk-relative); the index the engine continues after
    /// (it adds 1).
    pub(super) fn allofono(&self, w: &[Word], phones: &mut Vec<Phone>, i: usize) -> usize {
        let mut l = List(phones);
        let cur = l.at(i);
        let word = cur.word;
        let text = w.get(word as usize).map_or(&[][..], |r| r.text.as_slice());
        let next = l.at(i + 1);
        match cur.code {
            // 0x1000e5da (0xff2) / 0x1000e606 (0xfe2): the whole phone goes
            // in "åkk" / "jag"; otherwise after 0x19e2 (0xfe2: or 0x20017e2)
            // + 'a' when the word ends here or a consonant follows
            0xff2 | 0xfe2 => {
                if self.text_is(text, if cur.code == 0xff2 { AKK } else { JAG }) {
                    l.del(i);
                    return i;
                }
                if i <= 1 {
                    return i;
                }
                let pp = l.at(i - 2).code;
                if !(pp == 0x19e2 || cur.code == 0xfe2 && pp == RTJ) {
                    return i;
                }
                if l.at(i - 1).code != 0x441 {
                    return i;
                }
                if word == next.word {
                    let c = if i + 1 < 0x400 { next.code } else { 0x10 };
                    if c & 3 != 2 {
                        return i;
                    }
                }
                l.del(i);
            }
            // 0x1000e695
            0x18a2 => {
                l.merge(i, word, RL);
            }
            // 0x1000e990: after a merged code, the merge; after a 0x1952 of
            // the same word, this one goes
            0x1952 => {
                if i == 0 {
                    return i;
                }
                if l.merge(i, word, RT).is_none() {
                    let p = l.at(i - 1);
                    if p.code == 0x1952 && p.word == word {
                        l.del(i);
                    }
                }
            }
            // 0x1000e78d: /r/ before a dental
            0x19a2 => {
                if self.text_is(text, FOR) && word != next.word {
                    if let Some(to) = merged(next.code) {
                        l.del(i);
                        l.set(i, to);
                        return i;
                    }
                }
                // 0x1000e87a
                if i + 1 < 0x400 && MERGED.contains(&next.code) {
                    l.del(i);
                    return i;
                }
                if word != next.word {
                    return i;
                }
                if let Some(to) = merged(next.code) {
                    l.del(i);
                    l.set(i, to);
                }
            }
            // 0x1000ea84
            0x19c2 => {
                if word != next.word && (next.code == 0xfe2 || next.code == 0xff2) {
                    l.set(i, 0xfc2);
                    return i;
                }
                l.merge(i, word, RN);
            }
            // 0x1000ebb8 (0x19f2) / 0x1000ecd7 (0x19e2)
            0x19f2 | 0x19e2 => {
                let s = cur.code == 0x19f2;
                if l.merge(i, word, if s { RTS } else { RTJ }).is_some() {
                    return i;
                }
                // 0x1000ec70: the 0x19f2 of "det" goes
                if s && self.text_is(text, DET) {
                    l.del(i);
                    return i;
                }
                // 0x1000eca0 / 0x1000edba: at a word end before another one
                if word != next.word && (next.code == 0x19e2 || next.code == 0x19f2) {
                    l.set(i, if s { 0x59f2 } else { 0x59e2 });
                }
            }
            // 0x1000edee: a merged sibilant spreads over the next word's
            // first 0x19e2/0x19f2
            RTJ | RTS => {
                if word != next.word && (next.code == 0x19e2 || next.code == 0x19f2) {
                    l.del(i);
                    l.set(i, if next.code == 0x19e2 { RTJ } else { RTS });
                }
            }
            _ => {}
        }
        i
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::common::{Phone, Word};
    use super::super::Swedish;

    fn swedish() -> Option<Swedish> {
        let dll = std::fs::read("/tmp/qq-ghidra/swedish/LTTS7Swedish.dll").ok()?;
        Swedish::new(dll, None)
    }

    /// `word="…"` with `\xHH` escapes (the trace read as Latin-1, one char
    /// a byte).
    fn unescape(s: &str) -> Vec<u8> {
        let b: Vec<u8> = s.chars().map(|c| c as u8).collect();
        let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
        let mut out = Vec::new();
        let mut k = 0;
        while k < b.len() {
            match (b[k], b.get(k + 1), b.get(k + 2).and_then(|&c| hex(c)), b.get(k + 3).and_then(|&c| hex(c))) {
                (b'\\', Some(b'x'), Some(h), Some(l)) => {
                    out.push(h << 4 | l);
                    k += 4;
                }
                (b'\\', Some(&c), ..) => {
                    out.push(c);
                    k += 2;
                }
                (c, ..) => {
                    out.push(c);
                    k += 1;
                }
            }
        }
        out
    }

    fn phones(s: &str) -> Vec<Phone> {
        s.split_whitespace()
            .map(|t| {
                let v: Vec<&str> = t.split(':').collect();
                Phone {
                    word: u32::from_str_radix(v[0], 16).unwrap(),
                    code: u32::from_str_radix(v[1], 16).unwrap(),
                    kind: u8::from_str_radix(v[2], 16).unwrap(),
                }
            })
            .collect()
    }

    /// (records from the chunk's FonTrasParola words, phones in, phones out).
    fn chunks(path: &str) -> Vec<(Vec<Word>, Vec<Phone>, Vec<Phone>)> {
        let Ok(b) = std::fs::read(path) else { return Vec::new() };
        let text: String = b.iter().map(|&c| c as char).collect();
        let mut out = Vec::new();
        let mut words: Vec<(usize, Vec<u8>)> = Vec::new();
        let mut cur: Option<(Vec<Word>, Option<Vec<Phone>>, Option<Vec<Phone>>)> = None;
        let flush = |cur: &mut Option<(Vec<Word>, Option<Vec<Phone>>, Option<Vec<Phone>>)>, out: &mut Vec<(Vec<Word>, Vec<Phone>, Vec<Phone>)>| {
            if let Some((w, Some(i), Some(o))) = cur.take() {
                out.push((w, i, o));
            }
        };
        for line in text.lines() {
            if let Some(r) = line.strip_prefix("> FonTrasParola ") {
                if cur.is_some() {
                    flush(&mut cur, &mut out);
                    words.clear();
                }
                let (Some(a), Some(z)) = (r.find("word=\""), r.rfind("\" idx=")) else { continue };
                let idx: usize = r[z + 6..].trim().parse().unwrap_or(0);
                words.push((idx, unescape(&r[a + 6..z])));
            } else if line.starts_with("> FonAllofonoLingua ") && line.ends_with("idx=0") {
                flush(&mut cur, &mut out);
                let n = words.iter().map(|(k, _)| k + 1).max().unwrap_or(0);
                let mut w: Vec<Word> = (0..n + 1)
                    .map(|_| Word { text: Vec::new(), tag: None, f: [0; 24], data: Vec::new(), rows: Vec::new() })
                    .collect();
                for (k, t) in words.drain(..) {
                    w[k].text = t;
                }
                cur = Some((w, None, None));
            } else if let Some(p) = line.strip_prefix("  Q  P ") {
                if let Some(c) = cur.as_mut() {
                    c.2 = Some(phones(p));
                }
            } else if let Some(p) = line.strip_prefix("  P ") {
                if let Some(c) = cur.as_mut() {
                    if c.1.is_none() {
                        c.1 = Some(phones(p));
                    }
                }
            }
        }
        flush(&mut cur, &mut out);
        out
    }

    /// Every traced chunk: the list after the engine's loop (the traces
    /// print the list without its closing sentinel: one call per printed
    /// entry, the length read again after every call).
    #[test]
    fn allofono_matches_the_module() {
        let Some(sv) = swedish() else { return };
        let (mut n, mut bad) = (0, 0);
        for path in ["/tmp/qq-sv/tr-annika.les", "/tmp/qq-sv/w2k.les"] {
            for (w, mut got, want) in chunks(path) {
                let input = got.clone();
                let mut k = 0usize;
                while k < got.len() {
                    k = sv.allofono(&w, &mut got, k) + 1;
                }
                n += 1;
                if got != want {
                    bad += 1;
                    if bad <= 20 {
                        let f = |v: &[Phone]| v.iter().map(|p| format!("{}:{:x}", p.word, p.code)).collect::<Vec<_>>().join(" ");
                        eprintln!("{:?}\n  in   {}\n  want {}\n  got  {}", w.iter().map(|r| String::from_utf8_lossy(&r.text).into_owned()).collect::<Vec<_>>(), f(&input), f(&want), f(&got));
                    }
                }
            }
        }
        eprintln!("FonAllofonoLingua: {bad} of {n} chunks differ");
        assert_eq!(bad, 0);
    }
}
