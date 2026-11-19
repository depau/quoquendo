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

/// An ending table: sorted reversed endings, 12 bytes each `{ ending,
/// exception list, code }`; the code's bits 0–5 are the default answer,
/// 6–11 the answer when an exception matches without its own, 12–15 the
/// ending's length. The search below is the same code in
/// LTTS7Italian.dll and LTTS7Spanish.dll, at the same addresses.
#[derive(Clone, Copy)]
pub struct Endings {
    pub at: u32,
    pub count: u32,
}

/// Search an ending table for the longest ending of `rev` (a reversed
/// word): 0x100010c0, then 0x100011c0 / 0x1000c4d0 check its first
/// letter. The exception list's address (0 for none) and the code;
/// without a matching ending the code is 0x1041.
pub fn ending(pe: &Pe, t: Endings, rev: &[u8]) -> (u32, u16) {
    let key = |i: i32| pe.cstr(pe.u32(t.at + 12 * i as u32));
    let code = |i: i32| pe.u16(t.at + 12 * i as u32 + 8);
    let (mut lo, mut hi) = (0i32, t.count as i32 - 1);
    let mut found = None;
    while lo <= hi {
        let mid = (lo + hi) / 2;
        match rev.cmp(key(mid)) {
            std::cmp::Ordering::Less => hi = mid - 1,
            std::cmp::Ordering::Greater => lo = mid + 1,
            std::cmp::Ordering::Equal => {
                found = Some(mid);
                break;
            }
        }
    }
    let idx = match found {
        Some(i) => i,
        None if hi >= 0 => {
            let mut i = hi;
            let k0 = key(i);
            let mut k = 0usize;
            if at(rev, 0) == at(k0, 0) {
                loop {
                    k += 1;
                    if at(rev, k) != at(k0, k) {
                        break;
                    }
                }
            }
            if at(k0, k) != 0 && k != 0 {
                loop {
                    i -= 1;
                    while i > 0 && k < (code(i) >> 12) as usize {
                        i -= 1;
                    }
                    k = ((code(i) >> 12) as usize).saturating_sub(1);
                    if i <= 0 || at(key(i), k) == at(rev, k) {
                        break;
                    }
                }
            }
            i.max(0)
        }
        None => 0,
    };
    if at(rev, 0) != at(key(idx), 0) {
        return (0, 0x1041);
    }
    (pe.u32(t.at + 12 * idx as u32 + 4), code(idx))
}

/// An exception list against a reversed word (0x10001000): entries of
/// reversed letters matched from the ending's length on, `$` for the
/// word's start, then an optional number; the first match gives its
/// number, or the code's second answer; no match, its first.
pub fn exceptions(list: &[u8], rev: &[u8], code: u16) -> u16 {
    let start = (code >> 12) as usize;
    let first = at(rev, start);
    let mut e = 0usize;
    let mut dl = at(list, 0);
    if dl == 0 {
        return code & 0x3f;
    }
    loop {
        let mut s = start;
        if first == dl {
            loop {
                s += 1;
                e += 1;
                if at(rev, s) != at(list, e) {
                    break;
                }
            }
        }
        dl = at(list, e);
        let matched = match dl {
            b' ' => true,
            b'$' => {
                if at(rev, s) == 0 {
                    e += 1;
                    true
                } else {
                    false
                }
            }
            b'0'..=b'9' => true,
            _ => false,
        };
        if matched {
            let d = at(list, e);
            if !d.is_ascii_digit() {
                return (code >> 6) & 0x3f;
            }
            let d2 = at(list, e + 1);
            return if d2.is_ascii_digit() {
                (d - b'0') as u16 * 10 + (d2 - b'0') as u16
            } else {
                (d - b'0') as u16
            };
        }
        loop {
            let c = at(list, e);
            e += 1;
            if c == b' ' || c == 0 {
                break;
            }
        }
        dl = at(list, e);
        if dl == 0 {
            return code & 0x3f;
        }
    }
}

/// An ending table's answer for a reversed word (Italian 0x10001210,
/// 0x1000c520).
pub fn lookup(pe: &Pe, t: Endings, rev: &[u8]) -> u16 {
    let (list, code) = ending(pe, t, rev);
    if list == 0 {
        return code & 0x3f;
    }
    exceptions(pe.cstr(list), rev, code)
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
    /// the record's bytes; only 0x0c..0x18 are meaningful here (0x14..0x18
    /// stay as the module left them)
    pub f: [u8; 24],
    /// the module's own data behind the record's `+0x14` pointer (its
    /// LexInfo list: dictionary rows…); the engine never reads it
    pub data: Vec<u32>,
    /// the same `+0x14` list as lexicon rows (`char*[7]`: grafemica
    /// fonetica gradoaccentuale grammaticale gennum tempomodo altreinfo,
    /// NULL columns `None`), for the modules that keep rows there
    pub rows: Vec<Vec<Option<Vec<u8>>>>,
}

/// One entry of the sentence's phone list the FON phrase loop (0x1002c300)
/// builds: the record it belongs to, its code, and its kind (0x69 a phone,
/// 0x77 a word's last phone, 0x70 a pause, set at 0x1002c873).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Phone {
    pub word: u32,
    pub code: u32,
    pub kind: u8,
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
        (0..count)
            .map(|i| (self.cstr(self.u32(table + 8 * i)).to_vec(), self.u32(table + 8 * i + 4)))
            .collect()
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
    /// A number's words when the module has no number reader
    /// (`leggi_numero` gives `None`): the digits as they are.
    fn number_words(&self, digits: &str) -> Vec<Vec<u8>> {
        vec![digits.as_bytes().to_vec()]
    }
    /// `LesAnalizza`: word classes.
    fn analizza(&self, w: &mut [Word]);
    /// `LesAccenta`: the stress marked in the spelling.
    fn accenta(&self, w: &mut [Word]);
    /// `LesConfiniProsodici`: the boundary after each word; `mode` is the
    /// sentence type `modalita_frase` gives (0 `.`, 2 `?`, 3 `?` with an
    /// interrogative, 5 `:` `;`, 6 `!`, 7 other).
    fn confini(&self, w: &mut [Word], mode: i32);
    /// `LesGradiAccentuali`: stress grades.
    fn gradi(&self, w: &mut [Word]);
    /// `FonTrasParola`: one word's phone codes; the engine hands it the
    /// sentence's records and the word's index.
    fn tras_parola(&self, w: &[Word], i: usize) -> Vec<u32>;
    /// `FonTrasConfini`: called for every word of a chunk (its first
    /// included, then `phones` is empty) and for the closing punctuation's
    /// pause code, before `next` (that word's codes) is appended: may
    /// adjust the phones so far and the incoming codes. `prev_bound` is the
    /// boundary of the word before (skipping boundaries 0x16; 0x15 for the
    /// chunk's first). Default: none.
    fn tras_confini(&self, _prev_bound: u8, _phones: &mut [u32], _next: &mut Vec<u32>) {}
    /// FonTrasConfini as 0x1002d060 calls it: `sentence[first + i]` the
    /// incoming word (chunk index i), `phones` the engine's list so far
    /// (may be shortened, extended with `{word: i, code, kind: 0x69}`, or
    /// recoded) before `next` is appended. Default: `tras_confini` on the
    /// codes.
    fn tras_confini_in(&self, _sentence: &[Word], _first: usize, _i: usize, prev_bound: u8, phones: &mut Vec<Phone>, next: &mut Vec<u32>) {
        let mut codes: Vec<u32> = phones.iter().map(|p| p.code).collect();
        self.tras_confini(prev_bound, &mut codes, next);
        for (p, c) in phones.iter_mut().zip(codes) {
            p.code = c;
        }
    }
    /// The voice's symbol for a phone code.
    fn symbol(&self, code: u32) -> Option<&str>;
    /// `FonAllofonoLingua` (FON +0x24): once a chunk's phone list is
    /// complete (pauses included, plus a closing sentinel: the last
    /// entry's word, code 0, kind 0x70), the phrase loop runs `i = 0;
    /// while i < len - 1 { i = allofono(w, phones, i); i += 1 }`, the length
    /// re-read after every call. `w` is the chunk's records, `Phone.word`
    /// chunk-relative. The module may rewrite, insert or delete entries
    /// (deleting entry 0 returns `0usize.wrapping_sub(1)`). Default: none
    /// (the module has no such function).
    fn allofono(&self, _w: &[Word], _phones: &mut Vec<Phone>, i: usize) -> usize {
        i
    }
    /// What the phrase loop actually calls: `allofono` with the whole
    /// sentence too (FON ctx+0x20 its record count) and the chunk's first
    /// record index in it (chunk +0x18), for modules that read past the
    /// chunk. Default: `allofono(chunk, …)`.
    fn allofono_in(&self, _sentence: &[Word], _first: usize, chunk: &[Word], phones: &mut Vec<Phone>, i: usize) -> usize {
        self.allofono(chunk, phones, i)
    }
    /// `LesGestisciMaiuscole` (LES +0x130): run on the sentence's records
    /// right before LesAnalizza. Default: the engine's (`gestisci_maiuscole`).
    fn gestisci_maiuscole(&self, w: &mut [Word]) {
        gestisci_maiuscole(self, w)
    }
    /// `LesSetModalitaFrase` (LES +0x134): the sentence type, after
    /// LesAccenta. Default: the engine's (`modalita_frase`).
    fn modalita_frase(&self, w: &[Word]) -> i32 {
        modalita_frase(w)
    }
    /// `LesElaboraParola` (LES +0x44): the records for one word token, made
    /// by the module; `None`, the engine's own record (LAB_100381b0,
    /// `elabora_parola`). `idx` is the token's index in its sentence (0
    /// its first word), `flag408` the token entry's byte +0x408 (1: it
    /// comes from an expansion), `spell`/`mode` the engine's arguments 6
    /// and 7.
    fn elabora_parola(&self, _token: &[u8], _idx: usize, _flag408: u8, _spell: u8, _mode: u8) -> Option<Vec<Word>> {
        None
    }
    /// What the engine actually calls for a word token: `elabora_parola`
    /// with the whole token buffer (`toks[i]` the word, its text already
    /// through LesNormalizzaAccento), for modules that read the tokens
    /// after it; returns the records and how many following tokens it used
    /// (the engine's ctx+0x18). Default: `elabora_parola(text, …)`, none used.
    fn elabora_parola_at(&self, toks: &mut [Tok], i: usize, idx: usize, spell: u8, mode: u8) -> Option<(Vec<Word>, usize)> {
        self.elabora_parola(&toks[i].text, idx, toks[i].expanded as u8, spell, mode).map(|w| (w, 0))
    }
    /// `LesIsLetter` (LES +0x148). Default: the engine's (LAB_10035f80).
    fn is_letter(&self, c: u8) -> bool {
        engine_is_letter(c)
    }
    /// `LesIsUpper` (LES +0x14c). Default: the engine's (LAB_10035fd0).
    fn is_upper(&self, c: u8) -> bool {
        engine_is_upper(c)
    }
    /// `LesIsPartOfAcronym` (LES +0x144). Default: the engine's
    /// (LAB_10035f60, ASCII letters).
    fn is_part_of_acronym(&self, c: u8) -> bool {
        c.is_ascii_alphabetic()
    }
    /// The lower case of a character: byte 0 of the module's
    /// `ELQGetGraphemes` table (LES +0xc). Default: `letter(c).2`.
    fn lower(&self, c: u8) -> u8 {
        self.letter(c).2
    }
    /// `LesNormalizzaAccento` (LES +0x18), on a token or a symbol's name.
    /// Default: none.
    fn normalizza_accento(&self, _w: &mut Vec<u8>) {}
    /// `LesPronunciabile` (LES +0x20): whether a word can be read as it is
    /// (else the engine spells it). Default: yes (no such function).
    fn pronunciabile(&self, _w: &[u8]) -> bool {
        true
    }
    /// `LesPronunciabile` with its char argument: the engine passes 1 when
    /// it decides whether a dotted acronym's letters read as a word
    /// (0x10030620, from the initials path of 0x10031410). Default:
    /// `pronunciabile` whatever the flag.
    fn pronunciabile_flag(&self, w: &[u8], _flag: u8) -> bool {
        self.pronunciabile(w)
    }
    /// `LesGetSymbolName` (LES +0x30): the name of a letter or symbol
    /// (`mode` 0 in words, 1 or 2 when spelling everything). Default: none.
    fn symbol_name(&self, _c: u8, _mode: u8) -> Vec<u8> {
        Vec::new()
    }
    /// `LesEspandiAbbreviazioni` (LES +0x3c): the expansion of an
    /// abbreviation-like token (`S.`) given the token after it. Default:
    /// none.
    fn espandi_abbreviazione(&self, _token: &[u8], _next: &[u8]) -> Option<Vec<u8>> {
        None
    }
    /// The abbreviation table the engine builds from `LesGetAbbreviazioni`
    /// (LES +0x15c, `AbbreviationTable::new`). Default: empty.
    fn abbreviazioni(&self) -> &[Abbreviation] {
        &[]
    }
    /// The language's text filters (its `.lde`'s `Lexicon[i]`), applied
    /// to the text before the tokenizer. Default: none.
    fn text_filters(&self) -> &[TextFilter] {
        &[]
    }
    /// `LesIniNumeri` slot 9 (+0x24), the only number reader the engine
    /// calls itself (0x1003eb80): the reading of the number token `i`
    /// (space-separated words; a `#` piece marks the record before with
    /// subclass 0x7b), for reading type `kind` and date order `dmy`, and how
    /// many following tokens it used (the module's ctx+0x18). The module
    /// may rewrite later tokens. `None`: no such reader (`number_words`).
    fn leggi_numero(&self, _toks: &mut [Tok], _i: usize, _kind: u32, _dmy: &[u8]) -> Option<(Vec<u8>, usize)> {
        None
    }
    /// The module's `ELQGetLanguageNativeEncoding`: the code page the
    /// engine converts its UTF-8 input to (LES +0x150, default 1252).
    fn code_page(&self) -> u32 {
        1252
    }
}

/// The pauses the engine gives each kind of break (its ShortPauseLength
/// for the pauses LesConfiniProsodici adds, MediumPauseLength after a
/// comma, LongPauseLength at the end of a sentence).
const ADDED_PAUSE_MS: u32 = crate::g2p::SHORT_PAUSE_MS;
const COMMA_PAUSE_MS: u32 = crate::g2p::MEDIUM_PAUSE_MS;
const END_PAUSE_MS: u32 = crate::g2p::LONG_PAUSE_MS;

/// A word record as the engine makes it (0x100310f0): no boundary or grade
/// yet (0x15, 0x0e), `+0xc` 0xffff, 0 when the text ends with an
/// apostrophe (an elided word).
pub fn record(text: Vec<u8>, class: u8, sub: u8, info: u8) -> Word {
    let mut f = [0u8; 24];
    let elided = text.last() == Some(&b'\'');
    f[0xc] = if elided { 0 } else { 0xff };
    f[0xd] = if elided { 0 } else { 0xff };
    f[CLASS] = class;
    f[SUB] = sub;
    f[INFO] = info;
    f[BOUND] = 0x15;
    f[GRADE] = 0x0e;
    Word {
        text,
        tag: None,
        f,
        data: Vec::new(),
        rows: Vec::new(),
    }
}

/// One sentence for the LES stage: its records and its type
/// (`modalita_frase`, set when the LES stage runs).
pub struct Sentence {
    pub words: Vec<Word>,
    pub mode: i32,
}

/// A character in the engine's 8-bit text: Windows-1252, as the engine
/// converts its UTF-8 input.
pub fn to_8bit(c: char) -> Option<u8> {
    to_8bit_cp(c, 1252)
}

/// A character in the engine's 8-bit text in code page `cp`: the language
/// module's `ELQGetLanguageNativeEncoding` (1252 unless it says otherwise;
/// Greek 1253), into which LoqTTS7.dll converts its UTF-8 input.
pub fn to_8bit_cp(c: char, cp: u32) -> Option<u8> {
    let u = c as u32;
    const HIGH: [u32; 32] = [
        0x20ac, 0, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0x02c6, 0x2030, 0x0160, 0x2039, 0x0152, 0, 0x017d, 0, 0, 0x2018, 0x2019, 0x201c, 0x201d,
        0x2022, 0x2013, 0x2014, 0x02dc, 0x2122, 0x0161, 0x203a, 0x0153, 0, 0x017e, 0x0178,
    ];
    const GREEK: [u32; 128] = [
        0x20ac, 0, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0, 0x2030, 0, 0x2039, 0, 0, 0, 0, //
        0, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014, 0, 0x2122, 0, 0x203a, 0, 0, 0, 0, //
        0x00a0, 0x0385, 0x0386, 0x00a3, 0x00a4, 0x00a5, 0x00a6, 0x00a7, 0x00a8, 0x00a9, 0, 0x00ab, 0x00ac, 0x00ad, 0x00ae, 0x2015, //
        0x00b0, 0x00b1, 0x00b2, 0x00b3, 0x0384, 0x00b5, 0x00b6, 0x00b7, 0x0388, 0x0389, 0x038a, 0x00bb, 0x038c, 0x00bd, 0x038e, 0x038f, //
        0x0390, 0x0391, 0x0392, 0x0393, 0x0394, 0x0395, 0x0396, 0x0397, 0x0398, 0x0399, 0x039a, 0x039b, 0x039c, 0x039d, 0x039e, 0x039f, //
        0x03a0, 0x03a1, 0, 0x03a3, 0x03a4, 0x03a5, 0x03a6, 0x03a7, 0x03a8, 0x03a9, 0x03aa, 0x03ab, 0x03ac, 0x03ad, 0x03ae, 0x03af, //
        0x03b0, 0x03b1, 0x03b2, 0x03b3, 0x03b4, 0x03b5, 0x03b6, 0x03b7, 0x03b8, 0x03b9, 0x03ba, 0x03bb, 0x03bc, 0x03bd, 0x03be, 0x03bf, //
        0x03c0, 0x03c1, 0x03c2, 0x03c3, 0x03c4, 0x03c5, 0x03c6, 0x03c7, 0x03c8, 0x03c9, 0x03ca, 0x03cb, 0x03cc, 0x03cd, 0x03ce, 0,
    ];
    match (cp, u) {
        (_, 0..=0x7f) => Some(u as u8),
        (1253, _) => GREEK.iter().position(|&h| h == u && h != 0).map(|k| 0x80 + k as u8),
        // Windows' conversion keeps C1 controls as their own byte
        (_, 0x80..=0xff) => Some(u as u8),
        _ => HIGH.iter().position(|&h| h == u && h != 0).map(|k| 0x80 + k as u8),
    }
}

// --- TOP: LoqTTS7.dll's text handling, the same for every language -----------

/// The token types of the engine's lexer (0x1003e130), the entry's +0x400.
pub mod tok {
    pub const NONE: u32 = 0;
    pub const NUMBER: u32 = 1;
    pub const SYMBOL: u32 = 2;
    pub const WORD: u32 = 3;
    /// a letter and a dot at a word's start (`P.`, `U.S.A.`)
    pub const ABBR: u32 = 4;
    /// a control tag `\{…}`
    pub const TAG: u32 = 5;
    pub const PUNCT: u32 = 6;
    pub const URL: u32 = 8;
}

/// One token of the engine's token buffer: 0x40c bytes, its text at +0,
/// its type at +0x400, +0x408 set when it comes from an expansion.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tok {
    pub text: Vec<u8>,
    pub kind: u32,
    pub expanded: bool,
}

fn ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x08 | 0x0c)
}

/// The engine's `LesIsLetter` (LAB_10035f80): ASCII letters, 0xc0–0xd6,
/// 0xd8–0xf6, 0xf8–0xff and `ƒŠŒŽšœžŸ` (0x1009c9f4).
pub fn engine_is_letter(c: u8) -> bool {
    if c < 0x83 {
        c.is_ascii_alphabetic()
    } else {
        (0xc0..=0xd6).contains(&c) || (0xd8..=0xf6).contains(&c) || c >= 0xf8 || b"\x83\x8a\x8c\x8e\x9a\x9c\x9e\x9f".contains(&c)
    }
}

/// The engine's `LesIsUpper` (LAB_10035fd0): A–Z, 0xc0–0xd6, 0xd8–0xde
/// and `ŠŒŽŸ` (0x1009ca00).
pub fn engine_is_upper(c: u8) -> bool {
    c.is_ascii_uppercase() || (c >= 0x83 && ((0xc0..=0xd6).contains(&c) || (0xd8..=0xde).contains(&c) || b"\x8a\x8c\x8e\x9f".contains(&c)))
}

/// Util's `ELQNumIsCurrency` (LTTS7Util.dll 0x10015100): `$ £ € ¥` first.
pub fn is_currency(t: &[u8]) -> bool {
    matches!(at(t, 0), b'$' | 0xa3 | 0x80 | 0xa5)
}

/// Where a web address or mail address starts (0x10051290): up to the
/// first of ` \t\n\r\v\f;<>[]()'"` (0x100a1c60), an `@`, or one of the
/// prefixes at 0x100703e4.
fn url_start(t: &[u8]) -> bool {
    if at(t, 0) == b'\\' {
        return false;
    }
    let mut n = 0;
    while n < 0x1ff && n < t.len() && t[n] != 0 && !b" \t\n\r\x0b\x0c;<>[]()'\"".contains(&t[n]) {
        if t[n] == b'@' {
            return true;
        }
        n += 1;
    }
    const PREFIXES: [&[u8]; 8] = [b"file://", b"ftp.", b"ftp://", b"http://", b"https://", b"mailto://", b"news://", b"www."];
    PREFIXES.iter().any(|p| t[..n].starts_with(p))
}

/// Whether a web address goes on at `u` (0x1003e030): a letter or digit
/// there, or later before a space or one of `;<>[]()'"` (0x1009ef9c).
fn url_goes_on<L: Les + ?Sized>(l: &L, t: &[u8], u: usize) -> bool {
    let ok = |c: u8| c.is_ascii_digit() || l.is_letter(c);
    if ws(at(t, u)) {
        return false;
    }
    let mut k = u;
    loop {
        let c = at(t, k);
        if ok(c) {
            return true;
        }
        if c == 0 || ws(c) || b";<>[]()'\"".contains(&c) {
            return false;
        }
        k += 1;
    }
}

/// An initial or a run of initials (0x100300e0): one letter and a dot, or
/// letters each followed by a dot (`U.S.A.`, `a.c`), never two letters or
/// two dots in a row.
pub fn initials<L: Les + ?Sized>(l: &L, t: &[u8]) -> bool {
    if t.len() < 2 {
        return false;
    }
    if l.is_part_of_acronym(t[0]) && t[1] == b'.' && t.len() == 2 {
        return true;
    }
    match t.iter().position(|&c| c == b'.') {
        Some(d) if at(t, d + 1) != 0 => {}
        _ => return false,
    }
    let mut prev = false;
    for (k, &c) in t.iter().enumerate() {
        if c == b'.' {
            if at(t, k + 1) == b'.' {
                return false;
            }
            prev = false;
        } else {
            if !l.is_part_of_acronym(c) || prev {
                return false;
            }
            prev = true;
        }
    }
    true
}

/// What the lexer does after looking at one character.
#[derive(PartialEq)]
enum Go {
    /// `switchD_1003e5bd_caseD_5`: the first character sets the type
    Type,
    /// LAB_1003e2d6: go on while the character's type is the token's
    Same,
    /// LAB_1003e2dc: take it and go on
    Next,
    /// LAB_1003e2ef: the token ends before it
    End,
}

/// LoqTTS7.dll's lexer (0x1003e130): the next token of `t` from `*pos`
/// (spaces skipped), and its type; `None` at the end. `plain` (argument 5)
/// takes `. : , " '` and `-` as symbols, `tags` (argument 6) reads `\…`
/// as a tag (always for `\{`). The date/time modes (arguments 7, 8) are
/// SSML say-as only and not ported.
pub fn lex<L: Les + ?Sized>(l: &L, t: &[u8], pos: &mut usize, mut plain: bool, tags: bool) -> Option<Tok> {
    let len = t.len();
    let letter = |c: u8| l.is_letter(c);
    let digit = |c: u8| c.is_ascii_digit();
    let mut u = *pos;
    while u < len && t[u] != 0 && ws(t[u]) {
        u += 1;
    }
    let start = u;
    let (mut state, mut cur, mut result) = (0u32, 0u32, 0u32);
    let (mut quotes, mut paren, mut stop, mut dash) = (0, false, false, false);
    while u < len {
        let mut c = t[u];
        if c == 0 {
            break;
        }
        if matches!(c, b'-' | 0x96 | 0x97 | 0xad) {
            c = b'-';
            dash = true;
        }
        let prev = |k: usize| if u >= k { t[u - k] } else { 0 };
        let next = at(t, u + 1);
        // LAB_1003ea68: a symbol, unless inside a tag
        let symbol = |state: u32, cur: &mut u32| {
            if state != 5 {
                *cur = 2;
            }
            Go::Type
        };
        // LAB_1003e9a6 / LAB_1003e8b6: punctuation
        let punct = |state: u32, cur: &mut u32| {
            if state == 5 {
                return Go::Type;
            }
            if state == 8 && url_goes_on(l, t, u) {
                return Go::Same;
            }
            *cur = 6;
            Go::Type
        };
        // LAB_1003e867: any other character
        let other = |c: u8, state: u32, cur: &mut u32, plain: bool| {
            if !b";!?()\xbf\xa1\"[]{}\xb6".contains(&c) {
                if state != 5 {
                    if state == 8 && url_goes_on(l, t, u) {
                        return Go::Same;
                    }
                    *cur = 2;
                }
                return Go::Type;
            }
            if plain {
                return symbol(state, cur);
            }
            if state == 6 && u > 2 && prev(1) == b'.' && letter(prev(2)) {
                return Go::End;
            }
            if c == 0xb6 {
                *cur = 6;
                return Go::Type;
            }
            punct(state, cur)
        };
        let go = if ws(c) && !paren {
            cur = 0;
            Go::Type
        } else if !plain && state == 0 && url_start(&t[u..]) {
            cur = 8;
            state = 8;
            result = 8;
            Go::Next
        } else if digit(c) {
            if state != 5 && state != 8 {
                cur = 1;
                if start + 0x28 < u {
                    stop = true;
                }
            }
            Go::Type
        } else if letter(c) {
            let mut g = Go::Type;
            if state != 5 && state != 8 {
                if next == b'.' && u == start && (u == 0 || ws(prev(1)) || b"\x94\x93\"\xab\xbb".contains(&prev(1))) {
                    cur = 4;
                } else if state == 4 && (u != start + 2 || !letter(next)) {
                    g = Go::Same;
                } else {
                    cur = 3;
                }
            }
            g
        } else if !paren && c == b'\\' {
            if !tags && next != b'{' {
                other(c, state, &mut cur, plain)
            } else if state != 5 {
                if next == b'\\' || ws(next) || (u != 0 && prev(1) == b'\\') {
                    cur = 2;
                } else {
                    cur = 5;
                }
                Go::Type
            } else {
                Go::End
            }
        } else if c == b'.' {
            if plain {
                symbol(state, &mut cur)
            } else {
                let n = if u + 1 < len { t[u + 1] } else { 0 };
                match state {
                    0..=3 => {
                        if n == 0 || ws(n) || b".:;,!\"?()[]{}\\".contains(&n) {
                            cur = 6;
                            Go::Type
                        } else if state == 1 {
                            if n > b'/' && n <= b'9' {
                                Go::Same
                            } else {
                                cur = 2;
                                Go::Type
                            }
                        } else if state == 3 && letter(n) {
                            state = 4;
                            result = 4;
                            cur = 4;
                            Go::Next
                        } else {
                            cur = 2;
                            Go::Type
                        }
                    }
                    4 => {
                        // switchD_1003e5bd_caseD_4
                        if n == 0 || ws(n) {
                            let mut word = t[start..u].to_vec();
                            word.push(b'.');
                            if initials(l, &word) {
                                cur = 4;
                                Go::Next
                            } else {
                                Go::End
                            }
                        } else if !letter(n) && !digit(n) {
                            if b".:;,!\"?()[]{}".contains(&n) {
                                cur = 4;
                                plain = true;
                                Go::Next
                            } else {
                                Go::End
                            }
                        } else {
                            cur = 4;
                            Go::Next
                        }
                    }
                    8 => {
                        if url_goes_on(l, t, u) {
                            Go::Same
                        } else {
                            Go::End
                        }
                    }
                    _ => Go::Type,
                }
            }
        } else if c == b':' || c == b',' {
            if plain {
                symbol(state, &mut cur)
            } else if state == 6 {
                if u > 2 && prev(1) == b'.' && letter(prev(2)) {
                    Go::End
                } else {
                    cur = 6;
                    Go::Type
                }
            } else if state == 5 {
                Go::Type
            } else if state == 8 {
                punct(state, &mut cur)
            } else if state == 1 {
                if digit(next) {
                    Go::Same
                } else {
                    Go::End
                }
            } else {
                cur = 6;
                Go::Type
            }
        } else if matches!(c, b'"' | 0x94 | 0x93 | 0xab | 0xbb) {
            if plain {
                symbol(state, &mut cur)
            } else if state == 5 {
                Go::Type
            } else if state == 8 && url_goes_on(l, t, u) {
                Go::Same
            } else {
                quotes += 1;
                cur = 6;
                if quotes == 2 {
                    stop = true;
                }
                Go::Type
            }
        } else if matches!(c, b'\'' | 0x92 | 0xb4 | b'`' | 0x91) {
            if !plain && state == 0 {
                cur = 6;
                state = 6;
                result = 6;
                Go::Next
            } else if !plain && state == 3 {
                Go::Type
            } else {
                symbol(state, &mut cur)
            }
        } else if c == b'(' {
            if state == 5 && !paren {
                paren = true;
                Go::Same
            } else {
                other(c, state, &mut cur, plain)
            }
        } else if c == b')' {
            if !paren {
                other(c, state, &mut cur, plain)
            } else {
                paren = false;
                Go::Type
            }
        } else if c == b'/' {
            if plain {
                symbol(state, &mut cur)
            } else {
                if !matches!(state, 1 | 5 | 8) {
                    cur = 2;
                }
                Go::Type
            }
        } else if c == b'-' || c == b'+' {
            if plain || (state != 8 && u != 0 && letter(prev(1)) && letter(next)) {
                symbol(state, &mut cur)
            } else if (c == b'-' && state == 1) || u == 0 || ws(prev(1)) {
                cur = if digit(next) {
                    1
                } else if c != b'+' {
                    6
                } else {
                    2
                };
                Go::Type
            } else if state == 6 {
                if u > 2 && prev(1) == b'.' && letter(prev(2)) {
                    Go::End
                } else {
                    cur = 6;
                    Go::Type
                }
            } else {
                punct(state, &mut cur)
            }
        } else {
            other(c, state, &mut cur, plain)
        };
        let go = match go {
            Go::Type if state != 0 => Go::Same,
            Go::Type => {
                result = cur;
                state = cur;
                Go::Next
            }
            g => g,
        };
        if go == Go::End || (go == Go::Same && cur != state) {
            break;
        }
        u += 1;
        if stop {
            break;
        }
    }
    *pos = u;
    if u == start {
        return None;
    }
    let mut text = t[start..u.min(start + 0x3ff)].to_vec();
    if dash {
        for c in text.iter_mut() {
            if matches!(*c, 0x96 | 0x97 | 0xad) {
                *c = b'-';
            }
        }
    }
    Some(Tok {
        text,
        kind: result,
        expanded: false,
    })
}

/// Every token of `t` (`lex` to the end), with where each starts and ends.
pub fn lex_spans<L: Les + ?Sized>(l: &L, t: &[u8], plain: bool, tags: bool) -> Vec<(Tok, usize, usize)> {
    let mut out = Vec::new();
    let mut pos = 0;
    loop {
        let mut s = pos;
        while s < t.len() && ws(t[s]) {
            s += 1;
        }
        match lex(l, t, &mut pos, plain, tags) {
            Some(tk) => out.push((tk, s, pos)),
            None => return out,
        }
    }
}

/// Every token of `t`.
pub fn lex_all<L: Les + ?Sized>(l: &L, t: &[u8], plain: bool, tags: bool) -> Vec<Tok> {
    lex_spans(l, t, plain, tags).into_iter().map(|(tk, _, _)| tk).collect()
}

/// A token as the abbreviation table compares it (0x10037a70): each
/// character through the module's lower-case table, then
/// LesNormalizzaAccento.
fn folded<L: Les + ?Sized>(l: &L, t: &[u8]) -> Vec<u8> {
    let mut w: Vec<u8> = t.iter().map(|&c| l.lower(c)).collect();
    l.normalizza_accento(&mut w);
    w
}

/// One entry of the engine's abbreviation table (LES +0x15c): the
/// abbreviation as the lexer splits it, folded, and what it reads as.
#[derive(Clone, Debug)]
pub struct Abbreviation {
    pub tokens: Vec<Vec<u8>>,
    pub expansion: Vec<u8>,
}

/// The abbreviation table from `LesGetAbbreviazioni`'s (abbreviation,
/// expansion) pairs, in the module's order.
pub fn abbreviation_table<L: Les + ?Sized>(l: &L, pairs: &[(Vec<u8>, Vec<u8>)]) -> Vec<Abbreviation> {
    pairs
        .iter()
        // an abbreviation ending in a space (English `gov `) never matches
        // a token (0x10037a70 compares the whole string)
        .filter(|(a, _)| !a.last().is_some_and(|&c| ws(c)))
        .map(|(a, e)| Abbreviation {
            tokens: lex_all(l, a, false, false).iter().map(|tk| folded(l, &tk.text)).collect(),
            expansion: e.clone(),
        })
        .filter(|a| !a.tokens.is_empty())
        .collect()
}

/// The abbreviation starting at token `i` (0x10037a70): the first entry
/// whose tokens match `i` and the tokens after it (tags skipped), and how
/// many tokens it covers.
fn match_abbreviation<'a, L: Les + ?Sized>(l: &L, table: &'a [Abbreviation], toks: &[Tok], i: usize) -> Option<(usize, &'a Abbreviation)> {
    let first = folded(l, &toks[i].text);
    'entries: for a in table {
        if a.tokens[0] != first {
            continue;
        }
        let mut j = i;
        for want in &a.tokens[1..] {
            j += 1;
            while j < toks.len() && toks[j].kind == tok::TAG {
                j += 1;
            }
            if j >= toks.len() || folded(l, &toks[j].text) != *want {
                continue 'entries;
            }
        }
        return Some((j + 1 - i, a));
    }
    None
}

/// Whether a punctuation token ends the sentence (0x10033910 over
/// 0x10030200): longer than 6 characters, or with `.` (one or two dots in
/// a row; three or more read as a comma), `!`, `?`, `:`, `;`, `¶`, or `)`
/// right before a dot; `(` and `)` otherwise read as commas.
pub fn ends_sentence(t: &[u8]) -> bool {
    if t.len() > 6 {
        return true;
    }
    let mut dots = 0;
    let mut marks: Vec<u8> = Vec::new();
    let flush = |dots: usize, marks: &mut Vec<u8>| {
        if dots > 2 {
            marks.push(b',');
        } else if dots > 0 {
            marks.push(b'.');
        }
    };
    for (k, &c) in t.iter().enumerate() {
        if c == b'.' {
            dots += 1;
            continue;
        }
        flush(dots, &mut marks);
        dots = 0;
        marks.push(match c {
            b'(' => b',',
            b')' if at(t, k + 1) == b'.' => 0,
            b')' => b',',
            c => c,
        });
    }
    flush(dots, &mut marks);
    marks.iter().any(|&c| among(b".!?:;\xb6", c))
}

/// The engine's own `LesElaboraParola` (LAB_100381b0): the word as one
/// record (0x17), or spelled letter by letter when LesPronunciabile
/// rejects it (or `mode` asks for spelling): each name a record (0x15,
/// 0x83, the last letter's 0x84).
pub fn elabora_parola<L: Les + ?Sized>(l: &L, t: &[u8], expanded: bool, spell: u8, mode: u8) -> Vec<Word> {
    let pron = l.pronunciabile(t);
    if mode == 0 && (pron || spell != 0 || expanded) {
        return vec![record(t.to_vec(), 0x17, 200, 200)];
    }
    spelled(l, t, mode, 0x15, 0x83, 0x84)
}

/// A text spelled out: each character's LesGetSymbolName, normalised, a
/// record (`class`, `sub`, info 9); the last character's `last`.
fn spelled<L: Les + ?Sized>(l: &L, t: &[u8], mode: u8, class: u8, sub: u8, last: u8) -> Vec<Word> {
    let mut out = Vec::new();
    for (k, &c) in t.iter().enumerate() {
        let mut name = l.symbol_name(c, mode);
        if name.is_empty() {
            continue;
        }
        l.normalizza_accento(&mut name);
        out.push(record(name, class, if k + 1 == t.len() { last } else { sub }, 9));
    }
    out
}

/// The engine's own `LesGestisciMaiuscole` (LAB_10034f60): from the first
/// record that is not punctuation on, every upper-case letter lowered
/// through the module's table; a record with one becomes a proper noun
/// (0x15) unless it is only its first letter and the record is the
/// sentence's first word or follows a quote (`"«»`, 0x1009c434).
pub fn gestisci_maiuscole<L: Les + ?Sized>(l: &L, w: &mut [Word]) {
    let first = w.iter().position(|r| r.f[CLASS] != b'2').unwrap_or(w.len());
    for i in first..w.len() {
        for k in 0..w[i].text.len() {
            let c = w[i].text[k];
            if !l.is_upper(c) {
                continue;
            }
            w[i].text[k] = l.lower(c);
            if k != 0 || (i != first && !b"\x94\x93\"\xab\xbb".contains(&at(&w[i - 1].text, 0))) {
                w[i].f[CLASS] = 0x15;
            }
        }
    }
}

/// The reading type the engine hands the number reader (0x10030670, tag
/// `number`, kept at ctx+0x20c for the token the tag stands before) and the
/// date order (ctx+0x32c, "dmy" unless a `number=date…` tag sets it):
/// `ordinalM`/`MasculineOrdinal`/`ordinal` 5, `ordinalF`/`FeminineOrdinal`
/// 6, `telephone`/`phone` 1, `time…` 7, `code`/`digits`/`code_digits` 2,
/// `cardinal`/`real…`/`integer` 10, `currency…` 0xc, `date[<order>]` 8;
/// otherwise 0. Text filters (`.lex`) put such tags before numbers (`1º`
/// → `\{number=ordinalM}1`).
fn number_tag(toks: &[Tok], i: usize) -> (u32, Vec<u8>) {
    let mut kind = 0;
    let mut dmy = b"dmy".to_vec();
    let mut k = i;
    while k > 0 && toks[k - 1].kind == tok::TAG {
        k -= 1;
    }
    for t in &toks[k..i] {
        let body = t.text.strip_prefix(b"\\{").unwrap_or(&t.text);
        let body = body.strip_suffix(b"}").unwrap_or(body);
        let Some(v) = body.strip_prefix(b"number=") else { continue };
        kind = match v {
            b"ordinalM" | b"MasculineOrdinal" | b"ordinal" => 5,
            b"ordinalF" | b"FeminineOrdinal" => 6,
            b"telephone" | b"phone" => 1,
            b"time" | b"time_duration" | b"time_default" => 7,
            b"code" | b"digits" | b"code_digits" => 2,
            b"cardinal" | b"real" | b"integer" | b"real_extended" | b"real_short" | b"real_decimal" | b"real_default" => 10,
            b"currency" | b"currency_extended" | b"currency_short" | b"currency_decimal" | b"currency_default" => 0xc,
            v if v.starts_with(b"date") => {
                if v.len() > 4 {
                    dmy = v[4..v.len().min(11)].to_vec();
                }
                8
            }
            _ => kind,
        };
    }
    (kind, dmy)
}

/// The records for one token (0x10031410, the paths plain text takes:
/// no SSML say-as, the engine's default settings). `skip` takes how many
/// following tokens are used up (a number reader's, `°C`).
fn token_records<L: Les + ?Sized>(l: &L, toks: &mut [Tok], i: usize, idx: usize, out: &mut Vec<Word>, skip: &mut usize) {
    let t = toks[i].text.clone();
    let expanded = toks[i].expanded;
    let next_real = |toks: &[Tok]| toks[i + 1..].iter().find(|tk| tk.kind != tok::TAG).cloned();
    match toks[i].kind {
        tok::NUMBER => {
            // a lone short token makes the reading's last word 0x7b (0x10031380)
            let mut alone = t.len() <= 1 && toks.iter().enumerate().all(|(k, tk)| k == i || matches!(tk.kind, tok::TAG | tok::PUNCT));
            let (kind, dmy) = number_tag(toks, i);
            let reading = match l.leggi_numero(toks, i, kind, &dmy) {
                Some((r, used)) => {
                    *skip = used;
                    r
                }
                None => l.number_words(&String::from_utf8_lossy(&t)).join(&b' '),
            };
            // each piece goes into the token's text and through
            // LesElaboraParola (LES +0x44, the engine's own when the module
            // has none); its last record then becomes a number word: class
            // 0x15, sub 'z', info 0x0f when the token's text was "1" (the
            // number itself for the first piece, the piece before after)
            let mut n = 0;
            for piece in reading.split(|&c| c == b' ').filter(|p| !p.is_empty()) {
                if piece == b"#" {
                    if let Some(r) = out.last_mut() {
                        r.f[SUB] = 0x7b;
                        r.f[BOUND] = 0x0d;
                        alone = true;
                    }
                    continue;
                }
                let info = if toks[i].text == b"1" { 0x0f } else { 0x10 };
                toks[i].text = piece.to_vec();
                let before = out.len();
                match l.elabora_parola_at(toks, i, idx, 0, 0) {
                    Some((r, _)) => out.extend(r),
                    None => out.extend(elabora_parola(l, piece, toks[i].expanded, 0, 0)),
                }
                if out.len() > before {
                    let r = out.last_mut().unwrap();
                    r.f[CLASS] = 0x15;
                    r.f[SUB] = b'z';
                    r.f[INFO] = info;
                }
                n += 1;
            }
            if n > 0 {
                out.last_mut().unwrap().f[SUB] = if alone { 0x7b } else { b'y' };
            }
        }
        tok::SYMBOL => {
            if t.len() >= 3 {
                if out.last().is_some_and(|r| r.f[CLASS] != b'2') {
                    out.push(record(t, b'2', b'|', 9));
                }
                return;
            }
            if is_currency(&t) && next_real(toks).is_some_and(|n| n.kind == tok::NUMBER) {
                return; // the number reader reads it
            }
            let mut mode = 0;
            if (t == b"\xb0" || t == b"\xba") && toks.get(i + 1).is_some_and(|n| n.text.eq_ignore_ascii_case(b"c")) {
                mode = 2;
                *skip = 1;
            }
            if at(&t, 0) == b'\\' {
                mode = 2;
            }
            if t == b"-" {
                out.push(record(t, 0x37, 9, 200));
                return;
            }
            for &c in &t {
                let mut name = l.symbol_name(c, mode);
                if !name.is_empty() {
                    l.normalizza_accento(&mut name);
                    out.push(record(name, 0x15, 200, 200));
                }
            }
        }
        tok::WORD => {
            let mut w = t;
            l.normalizza_accento(&mut w);
            toks[i].text = w.clone();
            match l.elabora_parola_at(toks, i, idx, 0, 0) {
                Some((r, used)) => {
                    out.extend(r);
                    *skip = used;
                }
                None => out.extend(elabora_parola(l, &w, expanded, 0, 0)),
            }
        }
        tok::ABBR if initials(l, &t) => {
            let next = next_real(toks).map(|n| n.text).unwrap_or_default();
            if let Some(e) = (!next.is_empty()).then(|| l.espandi_abbreviazione(&t, &next)).flatten() {
                let mut r = record(e, 0x29, 200, 200);
                r.f[INFO] = 0x0f;
                out.push(r);
                return;
            }
            let letters: Vec<u8> = t.iter().copied().filter(|&c| l.is_letter(c)).collect();
            if letters.len() > 1 && l.pronunciabile_flag(&letters, 1) {
                let mut w = letters;
                l.normalizza_accento(&mut w);
                out.push(record(w, 0x15, 0x84, 9));
            } else {
                out.extend(spelled(l, &letters, 0, 0x15, 0x83, 0x84));
            }
        }
        tok::ABBR | tok::URL => {
            // split into pieces (0x100380a0), each read or spelled
            let mut pos = 0;
            while let Some(mut piece) = lex(l, &t, &mut pos, true, true) {
                if piece.kind == tok::WORD {
                    if let Some((1, a)) = match_abbreviation(l, l.abbreviazioni(), std::slice::from_ref(&piece), 0) {
                        piece.text = a.expansion.clone();
                    }
                }
                if piece.kind == tok::TAG {
                    continue;
                }
                let readable = piece.kind != tok::SYMBOL && (!matches!(piece.kind, tok::WORD | tok::ABBR) || l.pronunciabile(&piece.text));
                if readable {
                    let mut w = piece.text;
                    l.normalizza_accento(&mut w);
                    out.push(record(w, 0x17, 200, 200));
                } else if piece.kind == tok::SYMBOL {
                    out.extend(spelled(l, &piece.text, 0, 0x17, 200, 200));
                } else {
                    out.extend(spelled(l, &piece.text, 0, 0x15, 0x83, 0x84));
                }
            }
        }
        tok::PUNCT => {
            out.push(record(t.clone(), b'2', if ends_sentence(&t) { b'|' } else { b'}' }, 9));
        }
        _ => {}
    }
}

/// LoqTTS7.dll's TOP and LES text handling (0x10034110 per sentence,
/// 0x10031410 per token): the language's text filters, the lexer, the
/// abbreviation table, then each token's records; a sentence ends at
/// punctuation that `ends_sentence`, and text ending without one gets a
/// closing `.` (0x100303b0: grade 0x0f, tag `\{pause=0}`).
pub fn sentences<L: Les + ?Sized>(l: &L, text: &str) -> Vec<Sentence> {
    let cp = l.code_page();
    let mut t: Vec<u8> = text.chars().filter_map(|c| to_8bit_cp(c, cp)).collect();
    for f in l.text_filters() {
        t = f.apply(l, &t);
    }
    let mut toks = lex_all(l, &t, false, false);
    // a currency code before its amount goes after it (0x10034110)
    for i in 1..toks.len() {
        const CODES: [&[u8]; 10] = [b"usd", b"eur", b"gbp", b"jpy", b"cad", b"sgd", b"hkd", b"aud", b"nzd", b"yen"];
        if toks[i].kind == tok::NUMBER && matches!(toks[i - 1].kind, tok::WORD | tok::ABBR) && CODES.iter().any(|c| toks[i - 1].text.eq_ignore_ascii_case(c)) {
            toks.swap(i - 1, i);
        }
    }
    let mut out = Vec::new();
    let mut cur: Vec<Word> = Vec::new();
    let mut skip = 0usize;
    let mut idx = 0usize;
    let mut i = 0usize;
    while i < toks.len() {
        if skip > 0 {
            skip -= 1;
            i += 1;
            continue;
        }
        let next_is_tag = toks.get(i + 1).is_some_and(|n| n.kind == tok::TAG);
        if toks[i].kind != tok::TAG && !toks[i].expanded && !next_is_tag {
            if let Some((n, a)) = match_abbreviation(l, l.abbreviazioni(), &toks, i) {
                let mut new = lex_all(l, &a.expansion, false, true);
                for tk in new.iter_mut() {
                    tk.expanded = true;
                }
                toks.splice(i..i + n, new);
                continue;
            }
        }
        token_records(l, &mut toks, i, idx, &mut cur, &mut skip);
        idx += 1;
        if toks[i].kind == tok::PUNCT && ends_sentence(&toks[i].text) && cur.iter().any(|r| r.f[CLASS] != b'2') {
            out.push(Sentence {
                words: std::mem::take(&mut cur),
                mode: 0,
            });
            idx = 0;
        }
        i += 1;
    }
    if cur.iter().any(|r| r.f[CLASS] != b'2') {
        let mut r = record(b".".to_vec(), b'2', b'|', 9);
        r.f[GRADE] = 0x0f;
        r.tag = Some(b"\\{pause=0}".to_vec());
        cur.push(r);
        out.push(Sentence { words: cur, mode: 0 });
    }
    out
}

// --- The language's text filters: `.lex` lexicons ----------------------------

/// One placeholder-or-token of a text filter entry's key.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Key {
    Text(Vec<u8>),
    /// `%n`: a number token
    Number,
    /// `%w`: a word token
    Word,
}

#[derive(Clone, Debug)]
struct Section {
    insensitive: bool,
    entries: Vec<(Vec<Key>, Vec<u8>)>,
}

/// A `.lex` lexicon, the text filter LTTS7Util.dll's `ELQTextFilter*`
/// applies to the text before the tokenizer: `CA FE` then each byte XOR
/// the byte before it in the file (the first 0x34; `ELQCryptBufferDecrypt`
/// 0x10004ba0), UTF-8 XML. Its `<regexp>` entries rewrite the text with
/// regular expressions; each `<section>` (`case="insensitive"` or not)
/// lists `"key" = "value"` replacements of token runs, `%n` standing for a
/// number, `%w` for a word, `%1`… in the value for what they matched.
#[derive(Clone, Debug, Default)]
pub struct TextFilter {
    regexps: Vec<(Vec<ReNode>, Vec<u8>)>,
    sections: Vec<Section>,
}

/// Decrypt an encrypted lexicon or database file (`CA FE` header).
pub fn decrypt(b: &[u8]) -> Vec<u8> {
    if !b.starts_with(&[0xca, 0xfe]) {
        return b.to_vec();
    }
    let mut prev = 0x34u8;
    b[2..]
        .iter()
        .map(|&c| {
            let p = c ^ prev;
            prev = c;
            p
        })
        .collect()
}

/// The quoted strings of an entry line: `"key" = "value"`.
fn quoted(line: &str) -> Option<(String, String)> {
    let mut parts = Vec::new();
    let mut cur: Option<String> = None;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match (&mut cur, c) {
            (None, '"') => cur = Some(String::new()),
            (Some(s), '\\') => {
                s.push('\\');
                if let Some(n) = chars.next() {
                    s.push(n);
                }
            }
            (Some(_), '"') => parts.push(cur.take().unwrap()),
            (Some(s), c) => s.push(c),
            (None, _) => {}
        }
    }
    (parts.len() == 2).then(|| (parts[0].clone(), parts[1].clone()))
}

fn bytes8(s: &str) -> Vec<u8> {
    s.chars().filter_map(to_8bit).collect()
}

/// Lower case for the insensitive sections: ASCII and Latin-1 letters.
fn lower1(c: u8) -> u8 {
    match c {
        b'A'..=b'Z' => c + 32,
        0xc0..=0xde if c != 0xd7 => c + 32,
        _ => c,
    }
}

impl TextFilter {
    /// Read a `.lex` file; keys are split into tokens by `l`'s lexer.
    pub fn parse<L: Les + ?Sized>(l: &L, file: &[u8]) -> Option<TextFilter> {
        let plain = decrypt(file);
        let text = String::from_utf8_lossy(&plain);
        let text = text.trim_start_matches('\u{feff}');
        if !text.contains("<loqlexicon") {
            return None;
        }
        let mut f = TextFilter::default();
        let mut in_regexp = false;
        let mut section: Option<Section> = None;
        let mut comment = false;
        for line in text.lines() {
            let line = line.trim();
            if comment {
                comment = !line.contains("-->");
                continue;
            }
            if line.starts_with("<!--") {
                comment = !line.contains("-->");
                continue;
            }
            if line.starts_with("<regexp") {
                in_regexp = true;
            } else if line.starts_with("</regexp") {
                in_regexp = false;
            } else if line.starts_with("<section") {
                section = Some(Section {
                    insensitive: line.contains("case=\"insensitive\""),
                    entries: Vec::new(),
                });
            } else if line.starts_with("</section") {
                f.sections.extend(section.take());
            } else if let Some((k, v)) = quoted(line) {
                if in_regexp {
                    if let Some(re) = re_parse(&bytes8(&k)) {
                        f.regexps.push((re, bytes8(&v)));
                    }
                } else if let Some(s) = section.as_mut() {
                    let key = Self::key(l, &bytes8(&k), s.insensitive);
                    if !key.is_empty() {
                        s.entries.push((key, bytes8(&v)));
                    }
                }
            }
        }
        Some(f)
    }

    fn key<L: Les + ?Sized>(l: &L, k: &[u8], insensitive: bool) -> Vec<Key> {
        let mut out = Vec::new();
        let mut lit = Vec::new();
        let flush = |lit: &mut Vec<u8>, out: &mut Vec<Key>| {
            for tk in lex_all(l, lit, false, false) {
                out.push(Key::Text(if insensitive { tk.text.iter().map(|&c| lower1(c)).collect() } else { tk.text }));
            }
            lit.clear();
        };
        let mut i = 0;
        while i < k.len() {
            if k[i] == b'%' && matches!(at(k, i + 1), b'n' | b'w') {
                flush(&mut lit, &mut out);
                out.push(if k[i + 1] == b'n' { Key::Number } else { Key::Word });
                i += 2;
            } else {
                lit.push(k[i]);
                i += 1;
            }
        }
        flush(&mut lit, &mut out);
        out
    }

    /// The text with every regexp, then every section, applied.
    pub fn apply<L: Les + ?Sized>(&self, l: &L, t: &[u8]) -> Vec<u8> {
        let mut t = t.to_vec();
        for (re, with) in &self.regexps {
            t = re_replace(re, with, &t);
        }
        for s in &self.sections {
            t = Self::section(l, s, &t);
        }
        t
    }

    fn section<L: Les + ?Sized>(l: &L, s: &Section, t: &[u8]) -> Vec<u8> {
        let toks = lex_spans(l, t, false, false);
        let mut out = Vec::new();
        let mut done = 0usize;
        let mut j = 0;
        'tokens: while j < toks.len() {
            for (key, value) in &s.entries {
                if j + key.len() > toks.len() {
                    continue;
                }
                let mut caps: Vec<&[u8]> = Vec::new();
                let ok = key.iter().zip(&toks[j..]).all(|(k, (tk, _, _))| match k {
                    Key::Text(want) => {
                        if s.insensitive {
                            tk.text.len() == want.len() && tk.text.iter().zip(want).all(|(&a, &b)| lower1(a) == b)
                        } else {
                            tk.text == *want
                        }
                    }
                    Key::Number => {
                        caps.push(&tk.text);
                        tk.kind == tok::NUMBER
                    }
                    Key::Word => {
                        caps.push(&tk.text);
                        tk.kind == tok::WORD
                    }
                });
                if !ok {
                    continue;
                }
                let (start, end) = (toks[j].1, toks[j + key.len() - 1].2);
                out.extend_from_slice(&t[done..start]);
                let mut k = 0;
                while k < value.len() {
                    if value[k] == b'%' && at(value, k + 1).is_ascii_digit() && at(value, k + 1) != b'0' {
                        out.extend_from_slice(caps.get((value[k + 1] - b'1') as usize).copied().unwrap_or(b""));
                        k += 2;
                    } else {
                        out.push(value[k]);
                        k += 1;
                    }
                }
                done = end;
                j += key.len();
                continue 'tokens;
            }
            j += 1;
        }
        out.extend_from_slice(&t[done..]);
        out
    }
}

/// A regular expression of a `<regexp>` entry: literals, `.`, classes
/// `[…]` (ranges, `^`), groups `(…)`, `\` escapes, `^` `$`, and `* + ?`
/// after an atom.
#[derive(Clone, Debug)]
enum ReAtom {
    Lit(u8),
    Any,
    Set(Vec<(u8, u8)>, bool),
    Group(Vec<ReNode>, usize),
    Bol,
    Eol,
}

#[derive(Clone, Debug)]
struct ReNode {
    atom: ReAtom,
    min: usize,
    max: usize,
}

fn re_parse(p: &[u8]) -> Option<Vec<ReNode>> {
    let mut groups = 0;
    let mut i = 0;
    let nodes = re_seq(p, &mut i, &mut groups)?;
    (i == p.len()).then_some(nodes)
}

fn re_seq(p: &[u8], i: &mut usize, groups: &mut usize) -> Option<Vec<ReNode>> {
    let mut out: Vec<ReNode> = Vec::new();
    while *i < p.len() && p[*i] != b')' {
        let c = p[*i];
        *i += 1;
        let atom = match c {
            b'(' => {
                *groups += 1;
                let g = *groups;
                let inner = re_seq(p, i, groups)?;
                if at(p, *i) != b')' {
                    return None;
                }
                *i += 1;
                ReAtom::Group(inner, g)
            }
            b'[' => {
                let neg = at(p, *i) == b'^';
                if neg {
                    *i += 1;
                }
                let mut set = Vec::new();
                let mut first = true;
                while *i < p.len() && (p[*i] != b']' || first) {
                    first = false;
                    let mut a = p[*i];
                    *i += 1;
                    if a == b'\\' && *i < p.len() {
                        a = p[*i];
                        *i += 1;
                    }
                    if at(p, *i) == b'-' && *i + 1 < p.len() && p[*i + 1] != b']' {
                        let b = p[*i + 1];
                        *i += 2;
                        set.push((a, b));
                    } else {
                        set.push((a, a));
                    }
                }
                if at(p, *i) != b']' {
                    return None;
                }
                *i += 1;
                ReAtom::Set(set, neg)
            }
            b'\\' => {
                let e = at(p, *i);
                *i += 1;
                ReAtom::Lit(e)
            }
            b'.' => ReAtom::Any,
            b'^' => ReAtom::Bol,
            b'$' => ReAtom::Eol,
            c => ReAtom::Lit(c),
        };
        let (min, max) = match at(p, *i) {
            b'*' => (0, usize::MAX),
            b'+' => (1, usize::MAX),
            b'?' => (0, 1),
            _ => (1, 1),
        };
        if (min, max) != (1, 1) {
            *i += 1;
        }
        out.push(ReNode { atom, min, max });
    }
    Some(out)
}

impl ReAtom {
    fn one(&self, c: u8) -> bool {
        match self {
            ReAtom::Lit(l) => *l == c,
            ReAtom::Any => c != b'\n',
            ReAtom::Set(s, neg) => s.iter().any(|&(a, b)| a <= c && c <= b) != *neg,
            _ => false,
        }
    }
}

type Caps = Vec<Option<(usize, usize)>>;

/// Every way `nodes` match `t` from `pos`, greedy first: (end, captures).
fn re_match(nodes: &[ReNode], t: &[u8], pos: usize, caps: &Caps, out: &mut Vec<(usize, Caps)>) {
    let Some((n, rest)) = nodes.split_first() else {
        out.push((pos, caps.clone()));
        return;
    };
    match &n.atom {
        ReAtom::Group(inner, g) => {
            let mut ends = Vec::new();
            re_match(inner, t, pos, caps, &mut ends);
            for (e, mut c) in ends {
                c[*g] = Some((pos, e));
                re_match(rest, t, e, &c, out);
            }
        }
        ReAtom::Bol => {
            if pos == 0 {
                re_match(rest, t, pos, caps, out)
            }
        }
        ReAtom::Eol => {
            if pos == t.len() {
                re_match(rest, t, pos, caps, out)
            }
        }
        a => {
            let mut k = 0;
            while k < n.max && pos + k < t.len() && a.one(t[pos + k]) {
                k += 1;
            }
            if k < n.min {
                return;
            }
            for c in (n.min..=k).rev() {
                re_match(rest, t, pos + c, caps, out);
            }
        }
    }
}

/// Replace every leftmost match of `re` in `t` with `with` (`\1`… its groups).
fn re_replace(re: &[ReNode], with: &[u8], t: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos <= t.len() {
        let mut found = None;
        let mut s = pos;
        while s <= t.len() {
            let mut ends = Vec::new();
            re_match(re, t, s, &vec![None; 10], &mut ends);
            if let Some(m) = ends.into_iter().next() {
                found = Some((s, m));
                break;
            }
            s += 1;
        }
        let Some((s, (e, caps))) = found else { break };
        out.extend_from_slice(&t[pos..s]);
        let mut k = 0;
        while k < with.len() {
            if with[k] == b'\\' && at(with, k + 1).is_ascii_digit() {
                if let Some(Some((a, b))) = caps.get((with[k + 1] - b'0') as usize) {
                    out.extend_from_slice(&t[*a..*b]);
                }
                k += 2;
            } else {
                out.push(with[k]);
                k += 1;
            }
        }
        if e == s {
            if s < t.len() {
                out.push(t[s]);
            }
            pos = s + 1;
        } else {
            pos = e;
        }
    }
    if pos < t.len() {
        out.extend_from_slice(&t[pos..]);
    }
    out
}

/// The FON stage on text: the module's LES functions on every sentence,
/// then FonTrasParola word by word and FonTrasConfini at every word
/// boundary, as LoqTTS7.dll's phrase loop (0x1002c300, phones added by
/// 0x1002d060) runs them: phrases end at punctuation (code 0x17 `@` for
/// `|`, 0x13 `#` otherwise), a boundary 0x0a (or 8, 0x0c–0x0e) adds a
/// short pause, a word with boundary 0 (elision) shares the next one's.
/// Then FonAllofonoLingua over each chunk's phone list, and the voice's
/// own phone stage when `voice` is given (`voicefon`: what the voice says,
/// in its phone table).
pub fn analyse<L: Les + ?Sized>(l: &L, text: &str, voice: Option<&crate::voicefon::VoiceFon>) -> Vec<Token> {
    let mut tokens = Vec::new();
    for mut s in sentences(l, text) {
        let w = &mut s.words;
        l.gestisci_maiuscole(w);
        l.analizza(w);
        l.accenta(w);
        s.mode = l.modalita_frase(w);
        l.confini(w, s.mode);
        l.gradi(w);
        let mut a = 0usize;
        while a < w.len() {
            // FON works on chunks of records up to a punctuation record or
            // one whose boundary adds a pause (0x1002c300: chunk +0x18 its
            // first record, +4 their count); record indices restart in each
            let b = (a..w.len()).find(|&i| w[i].f[CLASS] == b'2' || pause_bound(w[i].f[BOUND])).unwrap_or(w.len() - 1);
            fon_chunk(l, w, a, b, s.mode, voice, &mut tokens);
            a = b + 1;
        }
    }
    tokens
}

/// The boundaries after which the phrase loop adds a pause (0x1002c850).
fn pause_bound(b: u8) -> bool {
    matches!(b, 0x0a | 0x08 | 0x0e | 0x0c | 0x0d)
}

/// LoqTTS7.dll's `LesSetModalitaFrase` (0x100350e0, the engine's own;
/// run after LesAccenta): the sentence type from its closing
/// punctuation's first character among `.!?:;¶`: `!` 6, `.` 0, `?` 2, or
/// 3 when one of the first four records has subclass `x` (an
/// interrogative), `:` `;` `¶` 5, anything else 7.
pub fn modalita_frase(w: &[Word]) -> i32 {
    let Some(end) = w.iter().position(|r| r.f[SUB] == b'|') else { return 0 };
    let wh = w[..end.min(4)].iter().any(|r| r.f[SUB] == b'x');
    let t = &w[end].text;
    let mut k = 0;
    while at(t, k + 1) != 0 && !among(b".!?:;\xb6", at(t, k)) {
        k += 1;
    }
    match at(t, k) {
        b'!' => 6,
        b'.' => 0,
        b'?' => 2 + wh as i32,
        0xb6 | b':' | b';' => 5,
        _ => 7,
    }
}

/// One FON chunk: FonTrasParola and FonTrasConfini word by word, then
/// FonAllofonoLingua over the chunk's phone list, the voice's stage, then
/// its tokens.
fn fon_chunk<L: Les + ?Sized>(l: &L, sentence: &[Word], a: usize, b: usize, mode: i32, voice: Option<&crate::voicefon::VoiceFon>, tokens: &mut Vec<Token>) {
    let w = &sentence[a..=b];
    let mut phones: Vec<Phone> = Vec::new();
    let pause = |word: usize, code: u32| Phone {
        word: word as u32,
        code,
        kind: 0x70,
    };
    for i in 0..w.len() {
        let r = &w[i];
        let punct = r.f[CLASS] == b'2';
        let mut new = if punct {
            vec![if r.f[SUB] == b'|' { 0x17 } else { 0x13 }]
        } else {
            l.tras_parola(w, i)
        };
        let mut j = i.saturating_sub(1);
        while j > 0 && w[j].f[BOUND] == 0x16 {
            j -= 1;
        }
        let prev_bound = if i == 0 { 0x15 } else { w[j].f[BOUND] };
        l.tras_confini_in(sentence, a, i, prev_bound, &mut phones, &mut new);
        if punct {
            if let Some(&c) = new.first() {
                phones.push(pause(i, c));
            }
            continue;
        }
        let n = new.len();
        phones.extend(new.iter().enumerate().map(|(k, &code)| Phone {
            word: i as u32,
            code,
            // 0x1002d060: 0x77 on the last phone when the boundary byte
            // is not 0 (not on an elided word's)
            kind: if k + 1 == n && r.f[BOUND] != 0 { 0x77 } else { 0x69 },
        }));
        if matches!(r.f[BOUND], 0x0a | 0x08 | 0x0e | 0x0c | 0x0d) {
            phones.push(pause(i, 0x13));
        }
    }
    // the closing sentinel, then FonAllofonoLingua (the engine skips the
    // words whose record has feature 0x35; no module port sets it)
    let last = phones.last().map_or(0, |p| p.word as usize);
    phones.push(pause(last, 0));
    let mut k = 0usize;
    while k < phones.len() - 1 {
        k = l.allofono_in(sentence, a, w, &mut phones, k).wrapping_add(1);
    }
    if let Some(v) = voice {
        let said = v.run(sentence, a, w.len(), &phones);
        return emit(w, &said, mode, &|c| v.name(c).to_string(), tokens);
    }
    phones.pop();
    emit(w, &phones, mode, &|c| l.symbol(c).unwrap_or("").to_string(), tokens);
}

/// A chunk's phones as tokens: a word ends after a phone of kind 0x77
/// (the engine's word end, so an elided word shares the next one's, and a
/// word whose last phone FonAllofonoLingua deleted runs into the next), a
/// pause for each pause entry (an added one, a comma's, the sentence end's).
fn emit(w: &[Word], phones: &[Phone], mode: i32, symbol: &dyn Fn(u32) -> String, tokens: &mut Vec<Token>) {
    let mut text: Vec<u8> = Vec::new();
    let mut syms: Vec<String> = Vec::new();
    let mut next_rec = 0usize;
    for p in phones {
        let (o, c) = (p.word as usize, p.code);
        let r = &w[o];
        if p.kind == 0x70 {
            flush_word(tokens, &mut text, &mut syms);
            let (ms, reason) = if r.f[CLASS] != b'2' {
                (ADDED_PAUSE_MS, "phrase break")
            } else if r.f[SUB] == b'|' {
                let reason = match mode {
                    2 | 3 => "question",
                    6 => "exclamation",
                    _ => "sentence break",
                };
                (END_PAUSE_MS, reason)
            } else {
                (COMMA_PAUSE_MS, "phrase break")
            };
            tokens.push(Token::Pause { ms, reason: reason.into() });
            next_rec = next_rec.max(o + 1);
            continue;
        }
        while next_rec <= o {
            text.extend(&w[next_rec].text);
            next_rec += 1;
        }
        syms.push(symbol(c));
        if p.kind == 0x77 {
            flush_word(tokens, &mut text, &mut syms);
        }
    }
    flush_word(tokens, &mut text, &mut syms);
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
