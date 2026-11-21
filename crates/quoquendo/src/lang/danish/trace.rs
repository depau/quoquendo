//! Checks against the original's LES trace: `oracle/run.sh` with
//! `LOQ_TRACE_LES=1 LOQ_LES_DLL=LTTS7Danish.dll LOQ_LES_TABLE=100251b8
//! LOQ_LES_N=46 LOQ_LES_DA=1` (`oracle/lestrace.h`), which logs every call
//! of the module's functions with the word records (`W`), their LexInfo
//! (`L`, solutions `S`/`M`) around LesAnalizza and LesAccenta and at
//! FonTrasParola (`R` its record), and FonAllofonoLingua's phone lists
//! (`P` before the first call of a chunk, `Q` after the last).
//!
//! The traces are not in the repository (they come from the original):
//! `QQ_DA_TRACE=<file.les>` names one; without it the tests check nothing.

use super::{Danish, LexInfo, Letter, Morpheme};
use crate::lang::common::{Phone, Word};

/// A record as the trace shows it.
#[derive(Clone, Debug, Default)]
pub(super) struct TWord {
    pub f: [u8; 24],
    /// the text (+4) and the tag (+0)
    pub text: Vec<u8>,
    pub tag: Option<Vec<u8>>,
    pub lexinfo: Option<LexInfo>,
}

impl TWord {
    /// The record as `common::Word`, its LexInfo stored in `data`.
    pub fn word(&self) -> Word {
        let mut w = Word {
            text: self.text.clone(),
            tag: self.tag.clone(),
            f: self.f,
            data: Vec::new(),
            rows: Vec::new(),
        };
        if let Some(l) = &self.lexinfo {
            l.store(&mut w);
        }
        w
    }
}

/// One call of a module function.
#[derive(Clone, Debug, Default)]
pub(super) struct Call {
    pub name: String,
    pub ctx: Vec<u32>,
    pub rc: u32,
    /// the records before and after (LES functions)
    pub before: Vec<TWord>,
    pub after: Vec<TWord>,
    /// FonTrasParola: the word, its chunk index, its sentence index
    /// (`R`), its record with its LexInfo, and the codes it gave
    pub word: Vec<u8>,
    pub idx: u32,
    pub rec_index: u32,
    pub rec: Option<TWord>,
    pub out: Vec<u32>,
    /// FonAllofonoLingua: the chunk's first record in the sentence
    /// (`C ch18`), the sentence's record count (`ctx20`), the phone list
    /// before the chunk's first call (`P`) and after its last (`Q`)
    pub chunk_first: u32,
    pub ctx20: u32,
    pub phones_in: Vec<Phone>,
    pub phones_out: Vec<Phone>,
    /// strings `s="…"` (NormalizzaAccento, GetSymbolName…), before and after
    pub s_in: Option<Vec<u8>>,
    pub s_out: Option<Vec<u8>>,
}

/// `\xNN` and `\"` `\\` escapes back to bytes.
pub(super) fn unescape(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' && i + 1 < b.len() {
            if b[i + 1] == b'x' && i + 3 < b.len() {
                out.push(u8::from_str_radix(&s[i + 2..i + 4], 16).unwrap_or(0));
                i += 4;
            } else {
                out.push(b[i + 1]);
                i += 2;
            }
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

/// The value of `key="…"` in a line.
pub(super) fn quoted(line: &str, key: &str) -> Option<Vec<u8>> {
    let pat = format!(" {key}=\"");
    let at = line.find(&pat)? + pat.len();
    let rest = &line[at..];
    let mut end = 0;
    let rb = rest.as_bytes();
    while end < rb.len() && rb[end] != b'"' {
        end += if rb[end] == b'\\' { 2 } else { 1 };
    }
    Some(unescape(&rest[..end.min(rest.len())]))
}

fn hex_bytes(s: &str) -> Vec<u8> {
    (0..s.len() / 2).map(|k| u8::from_str_radix(&s[2 * k..2 * k + 2], 16).unwrap_or(0)).collect()
}

fn record(words: &[&str]) -> [u8; 24] {
    let b = hex_bytes(&words.concat());
    let mut f = [0u8; 24];
    f[..b.len().min(24)].copy_from_slice(&b[..b.len().min(24)]);
    f
}

fn phones(s: &str) -> Vec<Phone> {
    s.split_whitespace()
        .filter_map(|p| {
            let mut it = p.split(':');
            Some(Phone {
                word: it.next()?.parse().ok()?,
                code: u32::from_str_radix(it.next()?, 16).ok()?,
                kind: u8::from_str_radix(it.next()?, 16).ok()?,
            })
        })
        .collect()
}

/// A morpheme line (`M i s0=… s4=… sc=… p8=… group grouping len gloss_len
/// start end`); `base` is the module's load address (the `B` line).
fn morpheme(line: &str, base: u32) -> Morpheme {
    let p8 = line.split(" p8=").nth(1).unwrap_or("-");
    let (p8, rest) = p8.split_once(' ').unwrap_or((p8, ""));
    let feat = if p8 == "-" {
        [0, 0]
    } else {
        let b = hex_bytes(p8);
        let ptr = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
        [u32::from_le_bytes([b[0], b[1], b[2], b[3]]), if ptr == 0 { 0 } else { ptr.wrapping_sub(base).wrapping_add(0x1000_0000) }]
    };
    let n: Vec<&str> = rest.split_whitespace().collect();
    let int = |k: usize| n.get(k).and_then(|x| x.parse::<i32>().ok()).unwrap_or(0);
    Morpheme {
        text: quoted(line, "s0").unwrap_or_default(),
        kind: quoted(line, "s4").unwrap_or_default(),
        feat,
        gloss: quoted(line, "sc"),
        group: int(0) as i16,
        grouping: n.get(1).and_then(|x| u32::from_str_radix(x, 16).ok()).unwrap_or(0),
        len: int(2),
        gloss_len: int(3),
        start: int(4),
        end: int(5),
    }
}

/// An `L` line: the LexInfo's head and letters.
fn lexinfo(line: &str) -> LexInfo {
    let mut l = LexInfo::default();
    let body = line.trim_start().strip_prefix("L ").unwrap_or(line);
    let mut parts = body.splitn(3, " | ");
    let head = hex_bytes(parts.next().unwrap_or(""));
    for (k, h) in l.head.iter_mut().enumerate() {
        if let Some(c) = head.get(4 * k..4 * k + 4) {
            *h = i32::from_le_bytes([c[0], c[1], c[2], c[3]]);
        }
    }
    // the pointers are not kept
    l.head[0] = 0;
    l.head[2] = 0;
    let letters = hex_bytes(parts.next().unwrap_or(""));
    for (k, t) in l.letters.iter_mut().enumerate() {
        if let Some(c) = letters.get(4 * k..4 * k + 4) {
            *t = Letter {
                morph: c[0] as i8,
                b1: c[1],
                flags: u16::from_le_bytes([c[2], c[3]]),
            };
        }
    }
    l.spelling = quoted(line, "sp").unwrap_or_default();
    l
}

/// Every call in a trace file.
pub(super) fn parse(path: &str) -> Vec<Call> {
    let text = std::fs::read(path).unwrap_or_default();
    let text: String = text.iter().map(|&c| c as char).collect();
    let mut calls: Vec<Call> = Vec::new();
    let mut base = 0x1000_0000u32;
    let mut cur: Option<Call> = None;
    // where W/L/S/M lines go: 0 before, 1 after, 2 the FonTrasParola record
    let mut side = 0;
    // the last P line, for the chunk's Q
    let mut chunk_in: Vec<Phone> = Vec::new();
    let mut chunk_first = 0u32;
    let mut ctx20 = 0u32;
    for line in text.lines() {
        if let Some(b) = line.strip_prefix("B ") {
            base = u32::from_str_radix(b.trim(), 16).unwrap_or(base);
            continue;
        }
        if let Some(rest) = line.strip_prefix("> ") {
            let mut it = rest.split_whitespace();
            let name = it.next().unwrap_or("").to_string();
            let mut c = Call { name, ..Default::default() };
            if let Some(cx) = line.split(" ctx=").nth(1) {
                c.ctx = cx.split(',').filter_map(|x| u32::from_str_radix(x.trim(), 16).ok()).collect();
            }
            if let Some(w) = quoted(line, "word") {
                c.word = w;
            }
            if let Some(i) = line.split(" idx=").nth(1) {
                c.idx = i.trim().parse().unwrap_or(0);
            }
            c.s_in = quoted(line, "s");
            side = if c.name == "FonTrasParola" { 2 } else { 0 };
            cur = Some(c);
            continue;
        }
        if let Some(rest) = line.strip_prefix("< ") {
            let Some(mut c) = cur.take() else { continue };
            if let Some(rc) = rest.split(" rc=").nth(1) {
                c.rc = u32::from_str_radix(rc.split_whitespace().next().unwrap_or("0"), 16).unwrap_or(0);
            }
            if let Some(o) = line.split(" out=").nth(1) {
                c.out = o.split_whitespace().next().unwrap_or("").split(',').filter_map(|x| u32::from_str_radix(x, 16).ok()).collect();
            }
            c.s_out = quoted(line, "s");
            if c.name == "FonAllofonoLingua" {
                c.phones_in = chunk_in.clone();
                c.chunk_first = chunk_first;
                c.ctx20 = ctx20;
            }
            side = 1;
            calls.push(c);
            continue;
        }
        let t = line.trim_start();
        let target: Option<&mut Call> = if side == 1 { calls.last_mut() } else { cur.as_mut() };
        let Some(c) = target else { continue };
        if let Some(w) = t.strip_prefix("W ") {
            let parts: Vec<&str> = w.split_whitespace().collect();
            let tw = TWord {
                f: record(&parts[1..7.min(parts.len())]),
                text: quoted(line, "s4").unwrap_or_default(),
                tag: quoted(line, "s0"),
                lexinfo: None,
            };
            if side == 1 { &mut c.after } else { &mut c.before }.push(tw);
        } else if let Some(r) = t.strip_prefix("R ") {
            let parts: Vec<&str> = r.split_whitespace().collect();
            c.rec_index = parts[0].parse().unwrap_or(0);
            c.rec = Some(TWord {
                f: record(&parts[1..]),
                text: c.word.clone(),
                ..Default::default()
            });
        } else if t.starts_with("L ") {
            let l = lexinfo(t);
            let w = match side {
                2 => c.rec.as_mut(),
                1 => c.after.last_mut(),
                _ => c.before.last_mut(),
            };
            if let Some(w) = w {
                w.lexinfo = Some(l);
            }
        } else if let Some(s) = t.strip_prefix("S ") {
            let w = match side {
                2 => c.rec.as_mut(),
                1 => c.after.last_mut(),
                _ => c.before.last_mut(),
            };
            if let Some(l) = w.and_then(|w| w.lexinfo.as_mut()) {
                let _ = s;
                l.solutions.push(Vec::new());
            }
        } else if t.starts_with("M ") {
            let m = morpheme(t, base);
            let w = match side {
                2 => c.rec.as_mut(),
                1 => c.after.last_mut(),
                _ => c.before.last_mut(),
            };
            if let Some(sol) = w.and_then(|w| w.lexinfo.as_mut()).and_then(|l| l.solutions.last_mut()) {
                sol.push(m);
            }
        } else if let Some(cl) = t.strip_prefix("C ") {
            let num = |k: &str| cl.split(&format!("{k}=")).nth(1).and_then(|x| x.split_whitespace().next()).and_then(|x| x.parse().ok()).unwrap_or(0);
            chunk_first = num("ch18");
            ctx20 = num("ctx20");
        } else if let Some(p) = t.strip_prefix("Q ") {
            if let Some(c) = calls.last_mut() {
                c.phones_out = phones(p.trim_start().strip_prefix("P").unwrap_or(p));
            }
        } else if let Some(p) = t.strip_prefix("P") {
            chunk_in = phones(p);
        }
    }
    calls
}

/// The trace named by `QQ_DA_TRACE`, parsed, when there is one.
pub(super) fn calls() -> Option<Vec<Call>> {
    let path = std::env::var("QQ_DA_TRACE").ok()?;
    Some(parse(&path))
}

/// The module from the voice store (`QQ_STORE`, default `/tmp/qqv/LTTS7`).
pub(super) fn module() -> Option<Danish> {
    let store = std::env::var("QQ_STORE").unwrap_or_else(|_| "/tmp/qqv/LTTS7".into());
    let dll = std::fs::read(format!("{store}/bin/LTTS7Danish.dll")).ok()?;
    let dbl = std::fs::read(format!("{store}/data/languages/Danish/Danish.dbl")).ok();
    Danish::from_files(&crate::lang::ModuleFiles {
        dll,
        dbl,
        ..Default::default()
    })
}

#[test]
fn trace_parses() {
    let Some(calls) = calls() else { return };
    let tp = calls.iter().filter(|c| c.name == "FonTrasParola").count();
    assert!(tp > 0);
    let with_li = calls.iter().filter(|c| c.name == "FonTrasParola" && c.rec.as_ref().is_some_and(|r| r.lexinfo.is_some())).count();
    assert_eq!(tp, with_li);
    // the LexInfo survives `store`/`of`
    for c in calls.iter().filter(|c| c.name == "FonTrasParola") {
        let r = c.rec.as_ref().unwrap();
        let w = r.word();
        assert_eq!(LexInfo::of(&w).as_ref(), r.lexinfo.as_ref());
    }
}
