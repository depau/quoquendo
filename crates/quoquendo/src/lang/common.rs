//! What every language module port shares: reading the module's DLL as data
//! (tables are located by the addresses its code uses) and the word records
//! the engine's LES stage hands the module.

use std::sync::Arc;

/// A PE image read as data: virtual addresses → bytes. Nothing in it runs.
#[derive(Clone)]
pub struct Pe {
    data: Arc<[u8]>,
    /// (virtual address, size, file offset, raw size)
    sections: Vec<(u32, u32, u32, u32)>,
}

fn u16_at(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?))
}

fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

impl Pe {
    pub fn parse(data: Vec<u8>) -> Option<Pe> {
        if data.get(0..2)? != b"MZ" {
            return None;
        }
        let nt = u32_at(&data, 0x3c)? as usize;
        if data.get(nt..nt + 4)? != b"PE\0\0" {
            return None;
        }
        let count = u16_at(&data, nt + 6)? as usize;
        let opt_size = u16_at(&data, nt + 20)? as usize;
        let base = u32_at(&data, nt + 24 + 28)?;
        let mut sections = Vec::new();
        for i in 0..count {
            let s = nt + 24 + opt_size + 40 * i;
            let (vsize, va, rsize, raw) = (u32_at(&data, s + 8)?, u32_at(&data, s + 12)?, u32_at(&data, s + 16)?, u32_at(&data, s + 20)?);
            sections.push((base.wrapping_add(va), vsize.max(rsize), raw, rsize));
        }
        Some(Pe { data: data.into(), sections })
    }

    /// The image's bytes from `va` to the end of its section (zeros past the
    /// raw data are not in the file and read as empty).
    pub fn at(&self, va: u32) -> &[u8] {
        self.sections
            .iter()
            .find(|(v, size, _, _)| va >= *v && va - v < *size)
            .and_then(|&(v, _, raw, rsize)| {
                let off = va - v;
                (off < rsize).then(|| &self.data[(raw + off) as usize..(raw + rsize) as usize])
            })
            .unwrap_or(&[])
    }

    pub fn u8(&self, va: u32) -> u8 {
        self.at(va).first().copied().unwrap_or(0)
    }

    pub fn u16(&self, va: u32) -> u16 {
        u16_at(self.at(va), 0).unwrap_or(0)
    }

    pub fn u32(&self, va: u32) -> u32 {
        u32_at(self.at(va), 0).unwrap_or(0)
    }

    /// The NUL-terminated string at `va` (without the NUL).
    pub fn cstr(&self, va: u32) -> &[u8] {
        let b = self.at(va);
        &b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())]
    }
}

/// C `strcmp` on two byte strings without their NULs: the sign of the
/// first difference, shorter first.
pub fn strcmp(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    a.cmp(b)
}

/// `strncmp(a, b, n) == 0` for NUL-free strings: the first `n` bytes agree,
/// a string ending before `n` only matching one that ends there too.
pub fn starts(a: &[u8], b: &[u8], n: usize) -> bool {
    let (x, y) = (&a[..a.len().min(n)], &b[..b.len().min(n)]);
    x == y
}

/// LTTS7Util.dll's `ELQSearchCompactString` (0x100289f0) over a blob of
/// NUL-separated strings, for the modes the language modules use:
/// `flags & 8`, an entry that ends the key; otherwise a binary search of the
/// sorted blob, `flags & 2` an entry the key starts with, else the key
/// itself (entries may be `key<sep>value`). The match's offset in the blob
/// (after the separator when `sep` is not 0).
///
/// The binary search stops when its bounds meet without looking at the
/// entry there, as the original does.
pub fn search_compact(key: &[u8], blob: &[u8], flags: u32, sep: u8) -> Option<usize> {
    let k0 = *key.first()?;
    let size = blob.len() as i64;
    if flags & 8 != 0 {
        let mut i = 0usize;
        if size <= 1 {
            return None;
        }
        loop {
            let e = &blob[i..];
            let len = e.iter().position(|&c| c == 0).unwrap_or(e.len());
            if len <= key.len() && key[key.len() - len..] == e[..len] {
                return Some(i);
            }
            i += 1 + len;
            if size <= i as i64 + 1 {
                return None;
            }
        }
    }
    let b = |i: i64| blob.get(i as usize).copied().unwrap_or(0);
    let (mut lo, mut hi) = (0i64, size - 1);
    if hi <= 0 {
        return None;
    }
    loop {
        let sum = hi + lo;
        let mut e = sum / 2;
        while e > 0 && b(e - 1) != 0 {
            e -= 1;
        }
        let mut k = 0i64;
        if b(e) == k0 {
            let mut c = k0;
            loop {
                if c == 0 || b(e + k) == sep {
                    break;
                }
                c = at(key, k as usize + 1);
                k += 1;
                if b(e + k) != c {
                    break;
                }
            }
        }
        if flags & 2 != 0 && b(e + k) == 0 {
            return Some(e as usize);
        }
        if at(key, k as usize) == 0 && b(e + k) == sep {
            return Some(if sep != 0 { (e + k + 1) as usize } else { e as usize });
        }
        if at(key, k as usize) < b(e + k) {
            hi = e - 1;
        } else {
            lo = sum / 2 + 1;
        }
        if hi <= lo {
            return None;
        }
    }
}

/// `strchr(set, c) != NULL` for `c != 0`.
pub fn among(set: &[u8], c: u8) -> bool {
    c != 0 && set.contains(&c)
}

/// The byte at `i`, 0 past the end (the C strings' terminator and beyond).
#[inline]
pub fn at(s: &[u8], i: usize) -> u8 {
    s.get(i).copied().unwrap_or(0)
}

/// One word of a phrase as the engine's LES stage keeps it: a 24-byte
/// record (`+4` its text, `+0` a tag) whose byte fields the language module
/// reads and writes. The fields are kept at their offsets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
    /// the text, in the engine's 8-bit (Windows-1252) characters
    pub text: Vec<u8>,
    /// the tag the engine attached (`\{pause=0` before a sentence end)
    pub tag: Option<Vec<u8>>,
    /// the record's bytes; only 0x0c..0x18 are meaningful here
    pub f: [u8; 24],
}

/// Field offsets in a word record.
pub mod field {
    /// word class (0x17 unknown, 0x2f noun/other, 0x2a verb, 0x18 article…)
    pub const CLASS: usize = 0xe;
    /// subclass (verb form 0x87…, 0xc8 none)
    pub const SUB: usize = 0xf;
    /// further information (gender/number, punctuation kind…)
    pub const INFO: usize = 0x10;
    /// prosodic boundary after the word
    pub const BOUND: usize = 0x11;
    /// stress grade (0 stressed, 3 unstressed, 0x0e not set yet)
    pub const GRADE: usize = 0x12;
}

impl Word {
    pub fn class(&self) -> u8 {
        self.f[field::CLASS]
    }
}

impl Pe {
    /// A language module's function table: the `count` (name, function)
    /// pairs its one export walks (LTTS7Italian.dll 0x10031ff8, German
    /// 0x100294a8…), as (name, address).
    pub fn function_table(&self, table: u32, count: u32) -> Vec<(Vec<u8>, u32)> {
        (0..count).map(|i| (self.cstr(self.u32(table + 8 * i)).to_vec(), self.u32(table + 8 * i + 4))).collect()
    }
}

// --- The engine's side: LoqTTS7.dll, the same for every language ---------------

use self::field::*;
use crate::g2p::Token;

/// What LoqTTS7.dll asks of a language module, by the names it binds them
/// with (LES 0x10036140, FON 0x1002d5c0): each port implements these as
/// its module does, and `analyse` runs them the way the engine does.
pub trait Les {
    /// The module's character classes for the tokenizer: (a letter, upper
    /// case, its lower case).
    fn letter(&self, c: u8) -> (bool, bool, u8);
    /// A number's words, in order (the engine reads numbers with the
    /// readers `LesIniNumeri` installs).
    fn number_words(&self, digits: &str) -> Vec<Vec<u8>>;
    /// `LesAnalizza`: word classes.
    fn analizza(&self, w: &mut [Word]);
    /// `LesAccenta`: the stress marked in the spelling.
    fn accenta(&self, w: &mut [Word]);
    /// `LesConfiniProsodici`: the boundary after each word; `mode` is the
    /// sentence type (0 `.`, 3 `?`, 5 `:` `;`, 6 `!`).
    fn confini(&self, w: &mut [Word], mode: i32);
    /// `LesGradiAccentuali`: stress grades.
    fn gradi(&self, w: &mut [Word]);
    /// `FonTrasParola`: one word's phone codes.
    fn tras_parola(&self, word: &[u8], grade: u8) -> Vec<u32>;
    /// `FonTrasConfini`: the phones so far adjusted to the next word's
    /// first phone, given the previous word's boundary.
    fn tras_confini(&self, prev_bound: u8, phones: &mut [u32], first: u32);
    /// The voice's symbol for a phone code.
    fn symbol(&self, code: u32) -> Option<&str>;
}

/// The pauses the engine gives each kind of break (its ShortPauseLength
/// for the pauses LesConfiniProsodici adds, MediumPauseLength after a
/// comma, LongPauseLength at the end of a sentence).
const ADDED_PAUSE_MS: u32 = crate::g2p::SHORT_PAUSE_MS;
const COMMA_PAUSE_MS: u32 = crate::g2p::MEDIUM_PAUSE_MS;
const END_PAUSE_MS: u32 = crate::g2p::LONG_PAUSE_MS;

/// A word record as the engine's tokenizer makes it: no boundary or grade
/// yet (0x15, 0x0e), `+0xc` 0xffff (0 after an elided word).
pub fn record(text: Vec<u8>, class: u8, sub: u8, info: u8) -> Word {
    let mut f = [0u8; 24];
    f[0xc] = 0xff;
    f[0xd] = 0xff;
    f[CLASS] = class;
    f[SUB] = sub;
    f[INFO] = info;
    f[BOUND] = 0x15;
    f[GRADE] = 0x0e;
    Word { text, tag: None, f }
}

/// One sentence for the LES stage: its records and its type (0 statement,
/// 3 question, 5 `:`/`;`, 6 exclamation).
pub struct Sentence {
    pub words: Vec<Word>,
    pub mode: i32,
}

/// A character in the engine's 8-bit (Windows-1252) text.
pub fn to_8bit(c: char) -> Option<u8> {
    let u = c as u32;
    match u {
        0..=0xff => Some(u as u8),
        0x2019 | 0x2018 => Some(b'\''),
        0x201c | 0x201d => Some(b'"'),
        0x2013 | 0x2014 => Some(b'-'),
        0x2026 => Some(b'.'),
        _ => None,
    }
}

/// Split text into sentences of word records, approximating LoqTTS7.dll's
/// tokenizer (TOP/LES; not ported yet): lower case, a capitalised word
/// inside a sentence a proper noun (0x15), an elided word (`l'`) its own
/// record, numbers as their words (class 0x15, subclass `z`, the last
/// `y`), punctuation records (class `2`, subclass `|` ending a sentence,
/// `}` inside one).
pub fn sentences<L: Les + ?Sized>(l: &L, text: &str) -> Vec<Sentence> {
    let mut out = Vec::new();
    let mut cur: Vec<Word> = Vec::new();
    let chars: Vec<u8> = text.chars().filter_map(to_8bit).collect();
    let letter = |c: u8| l.letter(c).0;
    let mut i = 0usize;
    let end = |cur: &mut Vec<Word>, out: &mut Vec<Sentence>, p: u8| {
        if cur.iter().all(|w| w.f[CLASS] == b'2') {
            cur.clear();
            return;
        }
        let mode = match p {
            b'!' => 6,
            b'?' => 3,
            b':' | b';' => 5,
            _ => 0,
        };
        cur.push(record(vec![p], b'2', b'|', 9));
        out.push(Sentence { words: std::mem::take(cur), mode });
    };
    while i < chars.len() {
        let c = chars[i];
        if letter(c) {
            let start = i;
            while i < chars.len() && letter(chars[i]) {
                i += 1;
            }
            let mut t: Vec<u8> = chars[start..i].iter().map(|&c| l.letter(c).2).collect();
            let elided = i < chars.len() && chars[i] == b'\'' && chars.get(i + 1).is_some_and(|&n| letter(n));
            if elided {
                t.push(b'\'');
                i += 1;
            }
            let capital = l.letter(chars[start]).1;
            let first = cur.iter().all(|w| w.f[CLASS] == b'2');
            let mut r = record(t, if capital && !first { 0x15 } else { 0x17 }, 0xc8, 0xc8);
            if elided {
                r.f[0xc] = 0;
                r.f[0xd] = 0;
            }
            cur.push(r);
        } else if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let parts = l.number_words(std::str::from_utf8(&chars[start..i]).unwrap_or("0"));
            let n = parts.len();
            for (k, t) in parts.into_iter().enumerate() {
                cur.push(record(t, 0x15, if k + 1 == n { b'y' } else { b'z' }, 0x10));
            }
        } else if matches!(c, b'.' | b'!' | b'?' | b':' | b';') {
            let start = i;
            while i < chars.len() && chars[i] == c {
                i += 1;
            }
            if c == b'.' && i - start > 1 {
                cur.push(record(chars[start..i].to_vec(), b'2', b'}', 9));
            } else {
                end(&mut cur, &mut out, c);
            }
        } else if matches!(c, b',' | b'"' | b'(' | b')' | b'-') {
            cur.push(record(vec![c], b'2', b'}', 9));
            i += 1;
        } else {
            i += 1;
        }
    }
    if !cur.is_empty() {
        end(&mut cur, &mut out, b'.');
    }
    out
}

/// The FON stage on text: the module's LES functions on every sentence,
/// then FonTrasParola word by word and FonTrasConfini at every word
/// boundary, as LoqTTS7.dll's phrase loop (0x1002c300, phones added by
/// 0x1002d060) runs them: phrases end at punctuation (code 0x17 `@` for
/// `|`, 0x13 `#` otherwise), a boundary 0x0a (or 8, 0x0c–0x0e) adds a
/// short pause, a word with boundary 0 (elision) shares the next one's.
pub fn analyse<L: Les + ?Sized>(l: &L, text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    for mut s in sentences(l, text) {
        let w = &mut s.words;
        l.analizza(w);
        l.accenta(w);
        l.confini(w, s.mode);
        l.gradi(w);
        let mut phrase: Vec<u32> = Vec::new();
        let mut phrase_start = 0usize;
        let mut word_syms: Vec<String> = Vec::new();
        let mut word_text: Vec<u8> = Vec::new();
        for i in 0..w.len() {
            let r = &w[i];
            let punct = r.f[CLASS] == b'2';
            let codes = if punct {
                vec![if r.f[SUB] == b'|' { 0x17 } else { 0x13 }]
            } else {
                l.tras_parola(&r.text, r.f[GRADE])
            };
            if let Some(&first) = codes.first() {
                if i > phrase_start && !phrase.is_empty() {
                    let mut j = i - 1;
                    while j > phrase_start && w[j].f[BOUND] == 0x16 {
                        j -= 1;
                    }
                    l.tras_confini(w[j].f[BOUND], &mut phrase, first);
                    // the assimilated phone belongs to the word before
                    if let (Some(&last), Some(sym)) = (phrase.last(), word_syms.last_mut().or_else(|| last_word_sym(&mut tokens))) {
                        *sym = l.symbol(last).unwrap_or("").to_string();
                    }
                }
            }
            if punct {
                flush_word(&mut tokens, &mut word_text, &mut word_syms);
                let (ms, reason) = if r.f[SUB] == b'|' {
                    let reason = match s.mode {
                        3 => "question",
                        6 => "exclamation",
                        _ => "sentence break",
                    };
                    (END_PAUSE_MS, reason)
                } else {
                    (COMMA_PAUSE_MS, "phrase break")
                };
                tokens.push(Token::Pause { ms, reason: reason.into() });
                phrase.clear();
                phrase_start = i + 1;
                continue;
            }
            phrase.extend(&codes);
            word_syms.extend(codes.iter().map(|&c| l.symbol(c).unwrap_or("").to_string()));
            word_text.extend(&r.text);
            if r.f[BOUND] != 0 {
                flush_word(&mut tokens, &mut word_text, &mut word_syms);
            }
            if matches!(r.f[BOUND], 0x0a | 0x08 | 0x0e | 0x0c | 0x0d) {
                phrase.push(0x13);
                tokens.push(Token::Pause {
                    ms: ADDED_PAUSE_MS,
                    reason: "phrase break".into(),
                });
            }
        }
        flush_word(&mut tokens, &mut word_text, &mut word_syms);
    }
    tokens
}

/// The last phone of the last word already emitted.
fn last_word_sym(tokens: &mut [Token]) -> Option<&mut String> {
    match tokens.last_mut() {
        Some(Token::Word { symbols, .. }) => symbols.last_mut(),
        _ => None,
    }
}

fn flush_word(tokens: &mut Vec<Token>, text: &mut Vec<u8>, syms: &mut Vec<String>) {
    if syms.is_empty() {
        text.clear();
        return;
    }
    tokens.push(Token::Word {
        text: text.iter().map(|&c| c as char).collect(),
        symbols: std::mem::take(syms),
    });
    text.clear();
}
