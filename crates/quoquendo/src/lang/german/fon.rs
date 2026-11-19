//! FonTrasParola (0x10015b50): one word's phone codes, from the module's
//! dictionary, its word lists and its letter-to-sound rules (0x1000d220).

mod lts;

use super::super::common::Word;
use super::German;

/// Character flags (`DAT_100287cc`, the char table's dword at +4).
const VOWEL: u32 = 0x10;
const UPPER: u32 = 0x20;
/// Accented (stress-marked) letter, mapped back by 0x10007f70.
const MARKED: u32 = 0x80;

/// The empty string the dictionary pronunciation is compared with (0x10026b34).
const EMPTY: u32 = 0x10026b34;
/// Words the stripping of prefixes and endings leaves alone: 0x40 pointers,
/// searched by `ELQSearchString` with flag 2 (0x10106500).
const WHOLE_WORDS: u32 = 0x10106500;
/// Prefixes `{ text, pronunciation }`, 0xde pairs, the first the word starts
/// with (0x10106600).
const PREFIXES: u32 = 0x10106600;
const PREFIX_COUNT: u32 = 0xde;
/// Endings `{ text, pronunciation }`, 0xf3 pairs, the first the word ends
/// with (0x10106cf0).
const ENDINGS: u32 = 0x10106cf0;
const ENDING_COUNT: u32 = 0xf3;
/// "ge" (0x1001e550), "gehen" (0x101043fc), "geben" (0x10107f3c), "zu"
/// (0x1001e204), "ig" (0x10107abc), "ch" (0x1001e5fc).
const GE: u32 = 0x1001e550;
const GEHEN: u32 = 0x101043fc;
const GEBEN: u32 = 0x10107f3c;
const ZU: u32 = 0x1001e204;
const IG: u32 = 0x10107abc;
const CH: u32 = 0x1001e5fc;

/// C `strncmp` over NUL-free slices (bytes past the end read as the NUL),
/// unsigned bytes.
pub(super) fn strncmp(a: &[u8], b: &[u8], n: usize) -> std::cmp::Ordering {
    for k in 0..n {
        let (x, y) = (a.get(k).copied().unwrap_or(0), b.get(k).copied().unwrap_or(0));
        if x != y {
            return x.cmp(&y);
        }
        if x == 0 {
            break;
        }
    }
    std::cmp::Ordering::Equal
}

/// The C string starting at `p` in a buffer (up to its NUL).
pub(super) fn cs(buf: &[u8], p: usize) -> &[u8] {
    let s = buf.get(p..).unwrap_or(&[]);
    &s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())]
}

fn strlen(buf: &[u8]) -> usize {
    cs(buf, 0).len()
}

/// `strcat` into a NUL-terminated buffer.
fn strcat(buf: &mut [u8], s: &[u8]) {
    let n = strlen(buf);
    buf[n..n + s.len()].copy_from_slice(s);
    buf[n + s.len()] = 0;
}

/// The size of the word buffers (stack arrays of 0x40 bytes; longer words
/// would run into the next one, so the model leaves room).
const BUF: usize = 0x200;

impl German {
    /// FonTrasParola (0x10015b50): the codes of record `i` of the chunk `w`
    /// (the engine hands it the chunk's records and the word's index).
    pub(crate) fn tras_parola(&self, w: &[Word], i: usize) -> Vec<u32> {
        let rec = &w[i];
        let grade = rec.f[0x12];
        let mut word = rec.text.clone();
        let mut out: Vec<u32> = Vec::new();
        // grade 0x0c/0x0d: a final "s" is transcribed on its own, last
        let mut s_apart = false;
        if (grade == 0xc || grade == 0xd) && word.last() == Some(&b's') {
            word.pop();
            s_apart = true;
        }
        let s_buf = b"s\0".to_vec();
        // the dictionary: marks removed, then also lower case
        let mut key = self.copy_word(&word, true, false);
        let (mut n, mut row) = self.phonetic.find(&key);
        if n == 0 {
            key = self.copy_word(&word, true, true);
            (n, row) = self.phonetic.find(&key);
        }
        if n == 1 {
            if let Some(f) = self.phonetic.get(row, 1).filter(|f| *f != self.s(EMPTY)) {
                self.parse_into(f, &mut out);
                if s_apart {
                    self.lts(&mut out, &s_buf, 0);
                }
                if matches!(grade, 2 | 3 | 0xd) {
                    unstress(&mut out);
                }
                return out;
            }
        }

        // letter to sound, with prefixes and endings apart
        let mut b = vec![0u8; BUF];
        let low = self.copy_word(&word, false, true);
        b[..low.len()].copy_from_slice(&low);
        let mut pre = 0usize;
        if self.search_string(&low, WHOLE_WORDS, 2, 0x40).is_none() {
            if let Some(k) = self.table_search(&low, PREFIXES, 2, PREFIX_COUNT) {
                let l = self.s(self.pe.u32(PREFIXES + 8 * k)).len();
                let mut j = l;
                while b[j] != 0 && self.flags(b[j]) & VOWEL == 0 {
                    j += 1;
                }
                if b[j] == 0 && b[l] != 0 {
                    pre = 0;
                } else {
                    pre = l;
                    self.parse_into(self.s(self.pe.u32(PREFIXES + 8 * k + 4)), &mut out);
                }
            }
            if strlen(&b) > pre + 2 && b[pre + 2] != 0 {
                let mut j = pre + 2;
                while b[j] != 0 && self.flags(b[j]) & VOWEL == 0 {
                    j += 1;
                }
                if b[j] != 0 {
                    let t = &b[pre..];
                    if strncmp(t, self.s(GE), 2).is_eq() && strncmp(t, self.s(GEHEN), 5).is_ne() && strncmp(t, self.s(GEBEN), 5).is_ne() {
                        out.extend([0xfe2, 0x501]);
                        pre += 2;
                    }
                    if strncmp(&b[pre..], self.s(ZU), 2).is_eq() {
                        out.extend([0x1912, 0x3a1]);
                        pre += 2;
                    }
                }
            }
        }
        // the ending, cut off and transcribed from the list
        let mut end1: Option<(&[u8], &[u8])> = None;
        let cut = |b: &mut Vec<u8>, k: u32| -> Option<(&[u8], &[u8])> {
            let suf = self.s(self.pe.u32(ENDINGS + 8 * k));
            let (lb, ls) = (strlen(b), suf.len());
            let ok = if lb > ls { suf[0] != b[lb - ls - 1] || lb == ls } else { lb == ls };
            if !ok {
                return None;
            }
            b[lb - ls] = 0;
            Some((suf, self.s(self.pe.u32(ENDINGS + 8 * k + 4))))
        };
        if let Some(k) = self.table_search(cs(&b, pre), ENDINGS, 8, ENDING_COUNT) {
            end1 = cut(&mut b, k);
        }
        let lb = strlen(&b);
        let mut ig = false;
        if lb > 2 && cs(&b, lb - 2) == self.s(IG) {
            ig = true;
            b[lb - 2] = 0;
        }
        let mut end2: Option<(&[u8], &[u8])> = None;
        match self.table_search(cs(&b, pre), ENDINGS, 8, ENDING_COUNT) {
            Some(k) => end2 = cut(&mut b, k),
            None => {
                if ig {
                    ig = false;
                    b[lb - 2] = b'i';
                }
            }
        }
        // a stem left without a vowel at its end takes its ending back
        let lb = strlen(&b);
        let back = if lb == 0 {
            false
        } else {
            let c = b[lb - 1];
            if self.flags(c) & VOWEL != 0 {
                true
            } else if c == b's' {
                match (end2, end1) {
                    (Some((e, _)), _) | (None, Some((e, _))) => strncmp(e, self.s(CH), 2).is_eq(),
                    _ => false,
                }
            } else {
                false
            }
        };
        if back && lb > 1 && !(b[lb - 1] == b'e' && b[lb - 2] == b'g') && !(b[lb - 1] == b'u' && b[lb - 2] == b'z') {
            if let Some((e, _)) = end2 {
                strcat(&mut b, e);
                end2 = None;
                if ig {
                    ig = false;
                    strcat(&mut b, self.s(IG));
                }
            } else if let Some((e, _)) = end1 {
                strcat(&mut b, e);
                end1 = None;
            }
        }
        let start = out.len();
        self.lts(&mut out, &b, pre);
        stress_one(&mut out, start, grade);
        if let Some((_, p)) = end2 {
            self.parse_into(p, &mut out);
            if ig {
                out.extend([0x781, 0x1152]);
            }
        }
        if let Some((_, p)) = end1 {
            self.parse_into(p, &mut out);
        }
        if matches!(grade, 2 | 3 | 0xd) {
            unstress(&mut out);
        }
        if (grade == 0xc || grade == 0xd) && s_apart {
            self.lts(&mut out, &s_buf, 0);
        }
        out
    }

    /// 0x1000ca80: copy a word, stress-marked letters back to plain ones
    /// (0x10007f70) when `plain`, upper case to lower when `lower`.
    fn copy_word(&self, w: &[u8], plain: bool, lower: bool) -> Vec<u8> {
        w.iter()
            .map(|&c| {
                let c = if self.flags(c) & MARKED != 0 && plain { self.unmark(c) } else { c };
                if self.flags(c) & UPPER != 0 && lower {
                    self.lower(c)
                } else {
                    c
                }
            })
            .collect()
    }

    /// 0x10007f70: a stress-marked vowel's plain letter, case kept.
    pub(super) fn unmark(&self, c: u8) -> u8 {
        if c == 0 {
            return 0;
        }
        let up = self.flags(c) & UPPER != 0;
        let c = if up { self.lower(c) } else { c };
        let c = match c {
            0xe0 => 0x61,
            0xe1 => 0xe4,
            0xe8 => 0x65,
            0xe9 => 0xeb,
            0xec => 0x69,
            0xed => 0xef,
            0xf2 => 0x6f,
            0xf3 => 0xf6,
            0xf9 => 0x75,
            0xfa => 0xfc,
            0xfd => 0x79,
            c => c,
        };
        if up {
            self.upper(c)
        } else {
            c
        }
    }

    /// 0x1000cb90 with 0x1000cb00: a pronunciation's symbols (separated by
    /// blanks or hyphens, at most 5 bytes each) as codes, 0x48 for unknown.
    fn parse_into(&self, s: &[u8], out: &mut Vec<u32>) {
        let at = |k: usize| s.get(k).copied().unwrap_or(0);
        let mut i = 0;
        while at(i) != 0 {
            let mut t = Vec::new();
            let mut c = at(i);
            while c != b' ' && c != b'-' && c != 0 && t.len() < 5 {
                t.push(c);
                i += 1;
                c = at(i);
            }
            if at(i) != 0 {
                i += 1;
            }
            out.push(if t.is_empty() { 0x48 } else { self.code_of(&t) });
        }
    }

    /// LTTS7Util `ELQSearchString` (0x10028ca0) over a table of `count`
    /// string pointers: a binary search when `count > 3` and no flag of
    /// 0x1c (flag 1 whole key, else the entry a prefix of the key), else a
    /// linear one (1 whole key, 2 entry a prefix, 8 entry a suffix).
    pub(super) fn search_string(&self, key: &[u8], table: u32, flags: u32, count: u32) -> Option<u32> {
        let entry = |k: u32| self.s(self.pe.u32(table + 4 * k));
        let cmp = |e: &[u8]| if flags & 1 == 0 { strncmp(key, e, e.len()) } else { key.cmp(e) };
        if count > 3 && flags & 0x1c == 0 {
            let (mut lo, mut hi) = (0i64, count as i64 - 1);
            while lo <= hi {
                let mid = (lo + hi) / 2;
                match cmp(entry(mid as u32)) {
                    std::cmp::Ordering::Less => hi = mid - 1,
                    std::cmp::Ordering::Greater => lo = mid + 1,
                    std::cmp::Ordering::Equal => return Some(mid as u32),
                }
            }
            return None;
        }
        (0..count).find(|&k| {
            let e = entry(k);
            if flags & 1 != 0 {
                key == e
            } else if flags & 2 != 0 {
                strncmp(key, e, e.len()).is_eq()
            } else {
                // flag 8 (flag 4, a substring search, is not used by the module)
                key.len() >= e.len() && key.ends_with(e)
            }
        })
    }

    /// 0x1000d040: the same linear search over `{ text, value }` pairs
    /// (flag 2 the entry a prefix of the key, 8 a suffix).
    fn table_search(&self, key: &[u8], table: u32, flags: u32, count: u32) -> Option<u32> {
        (0..count).find(|&k| {
            let e = self.s(self.pe.u32(table + 8 * k));
            if flags & 2 != 0 {
                strncmp(key, e, e.len()).is_eq()
            } else {
                key.len() >= e.len() && key.ends_with(e)
            }
        })
    }
}

/// 0x1000ceb0 (EAX grade, ECX start, ESI codes): in a word of one stressed
/// syllable, a short vowel 0x361/0x441/0x741 between single consonants gets
/// bit 2; consonants lose bits 2–3.
fn stress_one(out: &mut [u32], start: usize, grade: u8) {
    let end = out.len();
    let mut stressed = 0;
    if grade != 3 {
        stressed = out[start..end].iter().filter(|&&c| c & 3 == 1 && c != 0x501 && c != 0x481).count();
    }
    for k in start..end {
        let c = out[k];
        if stressed == 1 && matches!(c, 0x361 | 0x441 | 0x741) && k + 1 < end && k != 0 && out[k - 1] & 3 == 2 {
            let u = out[k + 1];
            if u & 3 == 2 && u & 0xc != 4 && u & 0x1e3 != 0x102 && u != 0xfc2 && (k + 2 == end || out[k + 2] & 3 == 1) {
                out[k] = c | 4;
            }
        }
        let c = out[k];
        if c & 3 == 2 && c & 0xc == 4 {
            out[k] = c & !0xc;
        }
    }
}

/// 0x1000cc60 (ESI count, EDX codes): stressed vowels (bit 16) back to
/// unstressed.
fn unstress(out: &mut [u32]) {
    for c in out.iter_mut() {
        *c = match *c {
            0x10765 => 0x765,
            0x10445 => 0x445,
            0x103a1 => 0x3a1,
            0x102e1 => 0x2e1,
            0x10365 => 0x365,
            0x103e5 => 0x3e5,
            0x10441 => 0x441,
            0x106c1 => 0x6c1,
            0x106c5 => 0x6c5,
            0x106e1 => 0x6e1,
            0x10745 => 0x745,
            0x1e2e5 => 0xe2e5,
            0x107c5 => 0x7c5,
            0x10781 => 0x781,
            0x107a1 => 0x7a1,
            0x107e5 => 0x7e5,
            0x1ac45 => 0xac45,
            0x10010445 => 0x10000445,
            0x1e445 => 0xe445,
            0x10010365 => 0x10000365,
            0x100106c5 => 0x100006c5,
            c => c,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::*;

    /// Every traced FonTrasParola call: the chunk's records and the word's
    /// index in, the codes out.
    #[test]
    #[ignore]
    fn tras_parola_matches_the_module() {
        let Some(de) = german() else { return };
        let (mut n, mut bad) = (0, 0);
        for line in lines("tp.txt") {
            let c: Vec<&str> = line.split('\t').collect();
            let w = records(c[0]);
            let i: usize = c[1].parse().unwrap();
            let want: Vec<u32> = c.get(3).filter(|s| !s.is_empty()).map(|s| s.split(',').map(|v| u32::from_str_radix(v, 16).unwrap()).collect()).unwrap_or_default();
            let got = de.tras_parola(&w, i);
            n += 1;
            if got != want {
                bad += 1;
                if bad <= 30 {
                    let sym = |v: &[u32]| v.iter().map(|&k| de.symbol(k).map(str::to_string).unwrap_or(format!("{k:x}"))).collect::<Vec<_>>().join(" ");
                    eprintln!("{} (grade {:x}): want {}\n  got  {}", show(&w[i]), w[i].f[0x12], sym(&want), sym(&got));
                }
            }
        }
        eprintln!("FonTrasParola: {bad} of {n} calls differ");
        assert_eq!(bad, 0);
    }
}
