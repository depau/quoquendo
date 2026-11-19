//! The voice library's own phone stage, after FON (MECHANICS.md §4e).
//!
//! When a phrase's phones are complete (the language module's FonTrasParola,
//! FonTrasConfini and FonAllofonoLingua have run), LoqTTS7.dll's phrase loop
//! (0x1002c300) hands them to the voice:
//!
//! 1. 0x1002da60 maps every phone into the voice's own inventory: the
//!    nearest phone of the voice (0x1003fdc0, one or two of them), its
//!    library feature word (0x10050d50) when the library is older than 7.3;
//! 2. the voice library's `FonVidAllofonoVoce` (voice +0x18) runs per
//!    phone and may rewrite, insert or delete phones;
//! 3. 0x1002d180 turns the list into the phones the voice says, through
//!    `FonVidMetaFonema` (voice +0x20; `FonVidAllofonoAnalisi`, +0x1c, only
//!    when the engine's analysis flag, FON ctx[4]+0x214, is set: never in
//!    synthesis), one call per phone: each writes table ids.
//!
//! Each voice ships its own functions: they are ported per voice
//! (`voicefon/*.rs`) and used only when the library's function table holds
//! exactly the ported addresses; any other library gets steps 1 and 3 with
//! the engine's default (0x1002d100: a phone's id is its table entry's).
//! Nothing of the library runs: its phone table and its word lists are read
//! from its bytes (`Pe`).

use crate::lang::common::{Pe, Phone, Word};
use crate::phoneset::{engine_features, phone_table, PhoneDef};

mod afroditi;
mod alan;
mod allison;
mod annika;
mod bernard;
mod dave;
mod elizabeth;
mod french;
mod german;
mod grace;
mod kate;
mod polish;
mod portuguese;
mod simon;
mod spanish;
mod steven;
mod susan;
mod sven;
mod willem;

/// The phone list's element kinds (0x1002d060, 0x1002c873).
pub const KIND_PHONE: u8 = 0x69;
pub const KIND_WORD_END: u8 = 0x77;
pub const KIND_PAUSE: u8 = 0x70;

/// A voice's FonVid functions, ported.
#[derive(Clone, Copy)]
struct Rules {
    /// `FonVidAllofonoVoce`: at phone `*i` of the list (may move `*i`)
    allofono: Option<fn(&mut Stage, &mut usize)>,
    /// `FonVidMetaFonema`: consume phones from `*i`, push table ids
    meta: fn(&Stage, &mut usize, &mut Vec<u8>),
}

/// A ported voice library: its name, the address of its function table
/// (`{name, function}` pairs its one export walks) and the (name, address)
/// pairs the port was made from; the library is accepted only when they
/// read back identical.
struct Port {
    voice: &'static str,
    table: u32,
    functions: &'static [(&'static str, u32)],
    rules: Rules,
}

const PORTS: &[Port] = &[
    german::KATRIN,
    german::STEFAN,
    spanish::JORGE,
    spanish::CARMEN,
    spanish::LEONOR,
    spanish::DIEGO,
    spanish::ESPERANZA,
    spanish::FRANCISCA,
    spanish::CARLOS,
    spanish::SOLEDAD,
    spanish::XIMENA,
    french::JULIETTE,
    french::FLORENCE,
    allison::ALLISON,
    dave::DAVE,
    steven::STEVEN,
    susan::SUSAN,
    kate::KATE,
    bernard::BERNARD,
    elizabeth::ELIZABETH,
    simon::SIMON,
    grace::GRACE,
    alan::ALAN,
    afroditi::AFRODITI,
    portuguese::AMALIA,
    portuguese::EUSEBIO,
    portuguese::GABRIELA,
    portuguese::FERNANDA,
    portuguese::FELIPE,
    annika::ANNIKA,
    sven::SVEN,
    willem::WILLEM,
    polish::KRZYSZTOF,
    polish::ZOSIA,
];

/// The engine's default `FonVidMetaFonema` (0x1002d100, and the voices'
/// own copies 0x10002390 Allison, 0x10001be0 Katrin…): the id of the first
/// table entry with the phone's code, else 1.
pub(crate) fn meta_default(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    let c = s.code(*i);
    out.push(s.phones.iter().position(|p| p.base == c).map_or(1, |k| k as u8));
    *i += 1;
}

/// One voice's phone stage, built from its library's bytes.
pub struct VoiceFon {
    pe: Pe,
    /// the library's phone table (`ELQGetPhonemes`), `{name, id, …, code}`
    phones: Vec<PhoneDef>,
    /// the library predates 7.3 (`GetVersion`'s minor number < 3, read by
    /// 0x10051210): its codes are library feature words, converted
    convert: bool,
    /// the mapping targets (voice +0x10, built by 0x1002d910): (table index,
    /// engine feature word) of every entry, or when `convert` only those
    /// whose library code lacks bit 3 or is a pause (0x18, 0x28)
    targets: Vec<(usize, u32)>,
    rules: Rules,
    /// the voice whose functions are ported, when this library is one
    pub ported: Option<&'static str>,
}

impl VoiceFon {
    /// The stage of the voice library `lib` (`LTTS7<Voice>.dll`).
    pub fn from_library(lib: &[u8]) -> Option<VoiceFon> {
        let phones = phone_table(lib)?;
        let pe = Pe::parse(lib.to_vec())?;
        let convert = minor_version(lib).is_some_and(|m| m < 3);
        let targets = phones
            .iter()
            .enumerate()
            .filter(|(_, p)| !convert || p.base & 8 == 0 || p.base == 0x18 || p.base == 0x28)
            .map(|(k, p)| (k, if convert { engine_features(p.base) } else { p.base }))
            .collect();
        let export = export_name(lib);
        let port = PORTS.iter().find(|p| {
            export.as_deref() == Some(p.voice.as_bytes())
                && p.functions
                    .iter()
                    .enumerate()
                    .all(|(k, &(name, addr))| pe.cstr(pe.u32(p.table + 8 * k as u32)) == name.as_bytes() && pe.u32(p.table + 8 * k as u32 + 4) == addr)
        });
        Some(VoiceFon {
            pe,
            phones,
            convert,
            targets,
            rules: port.map_or(Rules { allofono: None, meta: meta_default }, |p| p.rules),
            ported: port.map(|p| p.voice),
        })
    }

    /// The symbol of table id `id`.
    pub fn name(&self, id: u32) -> &str {
        self.phones.get(id as usize).map_or("", |p| p.name.as_str())
    }

    /// The whole stage on one chunk's phone list (language codes, closed by
    /// the sentinel: code 0, kind 0x70), whose records are
    /// `sentence[first..first + count]`: the phones the voice says, as
    /// (record, table id, kind 0x70 for a pause `#`/`@`, else 0x69).
    pub fn run(&self, sentence: &[Word], first: usize, count: usize, phones: &[Phone]) -> Vec<Phone> {
        let mut s = Stage {
            pe: &self.pe,
            phones: &self.phones,
            sentence,
            first,
            count,
            list: self.map(phones),
        };
        // 0x1002c300 after 0x1002da60: FonVidAllofonoVoce per phone, the
        // length re-read after every call
        if let Some(allofono) = self.rules.allofono {
            let mut i = 0usize;
            let mut n = s.list.len() - 1;
            if n != 0 {
                loop {
                    if s.code(i) != 0 {
                        allofono(&mut s, &mut i);
                    }
                    n = s.list.len() - 1;
                    i = i.wrapping_add(1);
                    if n >= 0x3fd || i >= n {
                        break;
                    }
                }
            }
        }
        // 0x1002d180: FonVidMetaFonema from each phone on
        let n = s.list.len() - 1;
        let mut out = Vec::new();
        let mut ids = Vec::new();
        let mut i = 0usize;
        while i < n {
            let word = s.list[i].word;
            let at = i;
            ids.clear();
            (self.rules.meta)(&s, &mut i, &mut ids);
            if i == at {
                i += 1;
            }
            out.extend(ids.iter().map(|&id| Phone {
                word,
                code: id as u32,
                kind: if id == 2 || id == 3 { KIND_PAUSE } else { KIND_PHONE },
            }));
            // the word's last output phone ends it when its record's
            // boundary is not 0 (an elided word runs into the next)
            if (i >= n || s.list[i].word != word) && s.rec(word as i64).is_some_and(|r| r.f[0x11] != 0) {
                if let Some(p) = out.last_mut().filter(|p| p.kind != KIND_PAUSE) {
                    p.kind = KIND_WORD_END;
                }
            }
        }
        // the phrase's last phone is not a word end, its last entry the pause
        let len = out.len();
        if len > 1 && out[len - 2].kind == KIND_WORD_END {
            out[len - 2].kind = KIND_PHONE;
        }
        out
    }

    /// 0x1002da60: each phone (up to the sentinel) as the voice's nearest
    /// one or two (0x1003fdc0), dropped when it has none. The list's slots
    /// keep their kinds: codes and records are written back in order, extra
    /// entries appended, the rest deleted; the sentinel closes it.
    fn map(&self, phones: &[Phone]) -> Vec<Phone> {
        let end = phones.iter().position(|p| p.code == 0).unwrap_or(phones.len());
        let mut mapped: Vec<(u32, u32)> = Vec::new();
        for p in &phones[..end] {
            for k in self.nearest(p.code) {
                mapped.push((p.word, self.code_of(k)));
            }
        }
        let last = phones.get(end).map_or(phones.last().map_or(0, |p| p.word), |p| p.word);
        mapped.push((last, 0));
        mapped
            .into_iter()
            .enumerate()
            .map(|(k, (word, code))| Phone {
                word,
                code,
                kind: phones.get(k).map_or(0, |p| p.kind),
            })
            .collect()
    }

    /// The code a mapping target stands for in the list: its engine code,
    /// or for an older library the table entry whose library code is
    /// 0x10050d50's (or that `| 0x2000004`), the first in table order.
    fn code_of(&self, k: usize) -> u32 {
        let e = self.targets[k].1;
        if !self.convert {
            return e;
        }
        let l = to_library(e);
        self.phones.iter().find(|p| p.base == l || p.base == l | 0x2000004).map_or(l, |p| p.base)
    }

    /// 0x1003fdc0: the voice's phone(s) nearest to engine code `t`, as
    /// indices into `targets`.
    fn nearest(&self, t: u32) -> Vec<usize> {
        let e: Vec<u32> = self.targets.iter().map(|&(_, c)| c).collect();
        let n = e.len();
        let one = |i: usize| vec![i];
        let t0 = norm(t);
        let mut t = t0;
        let mut weak = 0usize;
        for (i, &c) in e.iter().enumerate() {
            let c = norm(c);
            if c == t {
                return one(i);
            }
            if c | 0xc == t | 0xc {
                weak = i;
            }
        }
        if weak > 0 {
            return one(weak);
        }
        let tone = t & 0xc;
        if tone == 0xc {
            return vec![];
        }
        let m = t & 0x1e3;
        if m == 0x1e2 && t & 0x1e03 == 0x802 {
            return vec![];
        }
        // the lowest score over the entries `keep` accepts (`< start`)
        let best = |start: i32, keep: &dyn Fn(u32) -> bool, score: &dyn Fn(u32) -> i32| -> Option<usize> {
            let mut b = start;
            let mut k = None;
            for (i, &c) in e.iter().enumerate() {
                if keep(c) {
                    let s = score(c);
                    if s < b {
                        b = s;
                        k = Some(i);
                    }
                }
            }
            k
        };
        if m == 0x102 {
            // an affricate: the same, else a stop and a fricative
            if let Some(i) = e.iter().position(|&c| c & 0x1ff3 == t & 0x1ff3) {
                return one(i);
            }
            if let Some(i) = e.iter().position(|&c| c & 0x1fe3 == t & 0x1fe3) {
                return one(i);
            }
            let a = (t & !1) | 0x1e2;
            let Some(stop) = best(0xf, &|c| c & 0x1e3 == 0x1e2, &|c| cdist(a, c)) else { return vec![] };
            let b = (t & 0xffff_ff5e) | 0x142;
            let Some(fric) = best(0xf, &|c| c & 0x1e3 == 0x142, &|c| cdist(b, c)) else { return vec![] };
            return vec![stop, fric];
        }
        match t & 3 {
            2 => {
                let found = if special(t) {
                    let mut top = 0;
                    let mut k = None;
                    for (i, &c) in e.iter().enumerate() {
                        if special(c) {
                            let s = 1 + (c & 0xc == tone) as i32 + (c & 0x10 == t & 0x10) as i32;
                            if s > top {
                                top = s;
                                k = Some(i);
                            }
                        }
                    }
                    k.or_else(|| e.iter().position(|&c| c & 0x1e3 == 0xa2 && c & 0x1e03 == 0x1802))
                } else {
                    if m == 0xc2 && t & 0x1e03 == 0x1a02 && t & 0x10 == 0 {
                        t = (t & 0xffff_ff5e) | 0x142;
                    }
                    let mut b = 5;
                    let mut k = None;
                    for (i, &c) in e.iter().enumerate() {
                        let c = norm(c);
                        if c & 3 == 2 {
                            let s = cdist(t, c);
                            if s < b {
                                b = s;
                                k = Some(i);
                            }
                        }
                    }
                    k
                };
                let Some(found) = found else { return vec![] };
                if t & 0x20002 == 0x20002 {
                    // aspirated: a vowel-like release before it
                    if let Some(v) = best(100, &|c| c & 0x1020_c003 == 1, &|c| vdist(c, 0x501)) {
                        return vec![v, found];
                    }
                }
                one(found)
            }
            1 => {
                let c3 = t & 0xc003;
                if c3 != 1 {
                    // a diphthong: the same, else two vowels
                    if let Some(i) = e.iter().position(|&c| c & 0xffe3 == t & 0xffe3) {
                        return one(i);
                    }
                    let Some(a) = best(100, &|c| c & 0x1020_c003 == 1, &|c| vdist(c & 0x107e3, t & 0x107e3)) else { return vec![] };
                    let mut y = match t & 0x3003 {
                        0x1001 => 0x141,
                        0x2001 => 0x181,
                        0x3001 => 0x1c1,
                        _ => 0x101,
                    };
                    y |= match c3 {
                        0x4001 => 0x401,
                        0xc001 => 0x601,
                        _ => 0x201,
                    };
                    if t & 0x803 == 0x801 {
                        y |= 0x21;
                    }
                    let Some(b) = best(100, &|c| c & 0x1020_c003 == 1, &|c| vdist(c, y)) else { return vec![] };
                    return vec![a, b];
                }
                let mut mask = if t & 0x200001 == 0x200001 { 0xc003 } else { 0x20c003 };
                if t & 0x1000_0000 == 0 {
                    mask |= 0x1000_0000;
                }
                let Some(a) = best(100, &|c| c & mask == 1, &|c| vdist(c, t)) else { return vec![] };
                if t & 0x200001 == 0x200001 && e[a] & 0x200001 != 0x200001 {
                    return match e.iter().position(|&c| special(c)) {
                        Some(b) => vec![a, b],
                        None => one(a),
                    };
                }
                if t & 0x1000_0000 != 0 && e[a] & 0x1000_0000 == 0 {
                    return match e.iter().position(|&c| c & 0x1e3 == 0x1c2 && c & 0x1e03 == 0x1802) {
                        Some(b) => vec![a, b],
                        None => one(a),
                    };
                }
                one(a)
            }
            // a pause: the targets' third or fourth entry
            3 => one(if tone == 4 { 3 } else { 2 }).into_iter().filter(|&i| i < n).collect(),
            _ => vec![],
        }
    }
}

/// The phrase as the voice functions see it (their `ctx`): the sentence's
/// records (+0x2c), the chunk's first record and record count (chunk
/// +0x18, +4) and the phone list (+0x10, element pointers; +0x54 the list).
pub struct Stage<'a> {
    pub pe: &'a Pe,
    pub phones: &'a [PhoneDef],
    sentence: &'a [Word],
    first: usize,
    count: usize,
    pub list: Vec<Phone>,
}

impl Stage<'_> {
    /// The record of list element `i` (chunk-relative; past the list: none).
    pub fn word(&self, i: usize) -> u32 {
        self.list.get(i).map_or(u32::MAX, |p| p.word)
    }

    /// The code of list element `i` (past the list: 0).
    pub fn code(&self, i: usize) -> u32 {
        self.list.get(i).map_or(0, |p| p.code)
    }

    pub fn set(&mut self, i: usize, code: u32) {
        if let Some(p) = self.list.get_mut(i) {
            p.code = code;
        }
    }

    /// The record `w` of the chunk (`w` may step outside it, into the
    /// sentence; `None` outside the sentence).
    pub fn rec(&self, w: i64) -> Option<&Word> {
        usize::try_from(self.first as i64 + w).ok().and_then(|k| self.sentence.get(k))
    }

    /// A record's text (`+4`), lower case as LesGestisciMaiuscole left it.
    pub fn text(&self, w: i64) -> Option<&[u8]> {
        self.rec(w).map(|r| r.text.as_slice())
    }

    /// The chunk's record count (chunk +4).
    pub fn count(&self) -> usize {
        self.count
    }

    /// The sentence's record count (FON ctx +0x20).
    pub fn sentence_len(&self) -> usize {
        self.sentence.len()
    }

    /// The chunk's first record's index in the sentence (chunk +0x18).
    pub fn first(&self) -> usize {
        self.first
    }

    /// `ELQListDeleteElement` on slot `i`.
    pub fn delete(&mut self, i: usize) {
        if i < self.list.len() {
            self.list.remove(i);
        }
    }

    /// `ELQListAddElementBefore` slot `i` (`AddElementAfter` slot `i - 1`).
    pub fn insert(&mut self, i: usize, word: u32, code: u32, kind: u8) {
        self.list.insert(i.min(self.list.len()), Phone { word, code, kind });
    }
}

/// `strcmp(a, b) == 0` (the `repe cmpsb` over `b` and its NUL).
pub fn eq(a: Option<&[u8]>, b: &[u8]) -> bool {
    a == Some(b)
}

/// `strncmp(a, b, b.len()) == 0`.
pub fn starts(a: Option<&[u8]>, b: &[u8]) -> bool {
    a.is_some_and(|a| a.starts_with(b))
}

/// `strstr(a, b) != NULL`.
pub fn contains(a: Option<&[u8]>, b: &[u8]) -> bool {
    a.is_some_and(|a| a.windows(b.len()).any(|w| w == b))
}

/// The library's first exported name (`Katrin`, `Allison`: the one export
/// that hands out its function table), from the PE export directory.
fn export_name(lib: &[u8]) -> Option<Vec<u8>> {
    let u32_at = |o: usize| lib.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let nt = u32_at(0x3c)? as usize;
    let opt = nt + 24;
    let count = u16::from_le_bytes(lib.get(nt + 6..nt + 8)?.try_into().ok()?) as usize;
    let opt_size = u16::from_le_bytes(lib.get(nt + 20..nt + 22)?.try_into().ok()?) as usize;
    // file offset of an RVA
    let off = |rva: u32| -> Option<usize> {
        (0..count).find_map(|i| {
            let s = opt + opt_size + 40 * i;
            let (vsize, va, rsize, raw) = (u32_at(s + 8)?, u32_at(s + 12)?, u32_at(s + 16)?, u32_at(s + 20)?);
            (rva >= va && rva - va < vsize.max(rsize) && rva - va < rsize).then(|| (raw + rva - va) as usize)
        })
    };
    let dir = off(u32_at(opt + 96)?)?;
    if u32_at(dir + 0x18)? == 0 {
        return None;
    }
    let name = off(u32_at(off(u32_at(dir + 0x20)?)?)?)?;
    let rest = lib.get(name..)?;
    Some(rest[..rest.iter().position(|&c| c == 0)?].to_vec())
}

/// The library's `GetVersion` minor number (`LTTSComponentVersion=7.x.y`,
/// `LTTSDLLVERSION7.x.y`), as 0x10051210 reads it.
fn minor_version(lib: &[u8]) -> Option<u32> {
    for key in [&b"LTTSComponentVersion=7."[..], b"LTTSDLLVERSION7."] {
        if let Some(at) = lib.windows(key.len()).position(|w| w == key) {
            let digits: Vec<u8> = lib[at + key.len()..].iter().take_while(|c| c.is_ascii_digit()).copied().collect();
            return std::str::from_utf8(&digits).ok()?.parse().ok();
        }
    }
    None
}

/// 0x1003fdc0's view of a code: a palatal/velar `0xc2` with the 0x100002
/// mark becomes a `0x1e02` with 0x400002 / 0x200002.
fn norm(x: u32) -> u32 {
    if x & 0x1e3 == 0xc2 && matches!(x & 0x1e03, 0xe02 | 0x1002) && x & 0x100002 == 0x100002 {
        let y = (x & 0x1ffd) | if x & 0x1e03 == 0xe02 { 0x400002 } else { 0x200002 };
        (y & !1) | 0x1e02
    } else {
        x
    }
}

/// 0x1003fd00.
fn special(x: u32) -> bool {
    if x & 3 != 2 {
        return false;
    }
    let (m, p) = (x & 0x1e3, x & 0x1e03);
    let v = x & 0x2000002 == 0x2000002;
    m == 0x1a2 || (m == 0x142 && p == 0xc02) || (m == 0x182 && p == 0x1a02) || (m == 0xc2 && p == 0x1602 && v) || (m == 0x182 && p == 0x1602 && v) || (m == 0xc2 && p == 0x1802)
}

/// 0x1003f980: a consonant's manner class (jump tables 0x1003fa04,
/// 0x1003fa98).
fn manner(x: u32) -> i32 {
    match x & 0x1e3 {
        0x142 => 7,
        0x62 | 0x82 | 0x1e2 => 5,
        0xa2 => 0xb,
        0xc2 => 8,
        0xe2 => 0xa,
        0x162 | 0x182 | 0x1a2 => 3,
        0x1c2 => 1,
        _ => 0,
    }
}

/// 0x1003f8d0: a consonant's place class.
fn place(x: u32) -> i32 {
    match x & 0x1e03 {
        0x602 => 0xe,
        0x802 => 0xf,
        0xa02 => 0xd,
        0xc02 => 0xb,
        0xe02 => 0xa,
        0x1002 => 8,
        0x1402 | 0x1602 => 7,
        0x1802 => 5,
        0x1a02 => 4,
        0x1c02 => 2,
        0x1e02 => 1,
        _ => 0,
    }
}

/// 0x1003fb20: the distance between consonants `a` (esi) and `b` (edx).
fn cdist(a: u32, b: u32) -> i32 {
    let d1 = (manner(a) - manner(b)).abs();
    let mut d2 = (place(a) - place(b)).abs();
    let has = |x: u32, m: u32| x & m == m;
    if a & 0xffff_e000 != 0 || b & 0xffff_e000 != 0 {
        // Some(true): one closer, Some(false): one further
        let closer = if has(a, 0x100002) {
            Some(has(b, 0x100002) || b & 0x1e03 == 0x1e02)
        } else if has(a, 0x200002) {
            Some(has(b, 0x200002) || b & 0x1e03 == 0x1002)
        } else if has(a, 0x400002) {
            Some(has(b, 0x400002) || b & 0x1e03 == 0xe02)
        } else if has(a, 0x2000002) {
            Some(has(b, 0x2000002))
        } else if has(a, 0x1000002) {
            Some(has(b, 0x1000002))
        } else if has(b, 0x100002) {
            Some(a & 0x1e03 == 0x1e02)
        } else if has(b, 0x200002) {
            Some(a & 0x1e03 == 0x1002)
        } else if has(b, 0x400002) {
            Some(a & 0x1e03 == 0xe02)
        } else if has(b, 0x2000002) {
            Some(a & 0x1e03 == 0x1602)
        } else if has(b, 0x1000002) || a & 0x1e03 == 0x1002 {
            Some(false)
        } else {
            None
        };
        match closer {
            Some(true) if d2 > 0 => d2 -= 1,
            Some(false) => d2 += 1,
            _ => {}
        }
    }
    let mut r = d1 + d2;
    if (a ^ b) & 0x10 != 0 {
        r += 2;
    }
    if (a ^ b) & 0xc != 0 {
        r += 1;
    }
    r
}

/// 0x1003f750: a vowel's height class.
fn height(x: u32) -> i32 {
    match x & 0x1c3 {
        0x41 => 1,
        0x81 => 2,
        0xc1 => 4,
        0x101 => 5,
        0x141 => 6,
        0x181 => 8,
        0x1c1 => 9,
        _ => 0,
    }
}

/// 0x1003f7b0: a vowel's front/back class.
fn backness(x: u32) -> i32 {
    match x & 0x603 {
        0x201 => 6 + (x & 0x1000001 != 0x1000001) as i32,
        0x401 if x & 0x1000001 == 0x1000001 => 3,
        0x401 => 5 - (x & 0x2000001 != 0x2000001) as i32,
        0x601 => 2 - (x & 0x2000001 != 0x2000001) as i32,
        _ => 0,
    }
}

/// 0x1003f830: the distance between vowels `a` (eax) and `b` (ecx).
fn vdist(a: u32, b: u32) -> i32 {
    let mut r = (backness(b) - backness(a)).abs() + 2 * (height(b) - height(a)).abs();
    if (a ^ b) & 0x23 != 0 {
        r += 2;
    }
    if (a ^ b) & 0x10003 != 0 {
        r += 1;
    }
    if a & 0xe0003 != b & 0xe0003 && a & 0xe0003 != 0x80001 {
        r += 1;
    }
    if b & 0x1000_0000 != 0 && a & 0x1000_0000 == 0 {
        r += 5;
    }
    if b & 0x200001 == 0x200001 && a & 0x200001 != 0x200001 {
        r += 1;
    }
    r
}

/// LoqTTS7.dll 0x10050d50: an engine feature word as a library one (the
/// inverse of `phoneset::engine_features`).
pub fn to_library(p: u32) -> u32 {
    if p == 0 {
        return 0;
    }
    if p == 0x10 {
        return 0x48;
    }
    let b = p & 0xff;
    match p & 3 {
        3 => {
            if b & 0xc != 4 {
                0x18
            } else {
                0x28
            }
        }
        1 => {
            let lo = p & 0x603 == 0x401 && b & 0xc != 4;
            let mut u = match p & 0x1c3 {
                0x1c1 => 0x2001,
                0x181 => 0x4001,
                0x141 => 0x8001,
                0x101 if lo => 0x10041,
                0x101 => 0x10001,
                0xc1 => 0x20001,
                0x81 if lo => 0x40041,
                0x81 => 0x40001,
                0x41 => 0x80001,
                _ => 1,
            };
            match p & 0x603 {
                0x601 => u |= 0x401,
                0x401 => u |= 0x801,
                0x201 => u |= 0x1001,
                _ => {}
            }
            if b & 0x23 == 0x21 {
                u |= 0x100001;
            }
            let c = p & 0xc003;
            if c != 1 {
                u |= match p & 0x3003 {
                    0x3001 => 0x1000003,
                    0x2001 => 0x2000003,
                    0x1001 => 0x4000003,
                    1 => 0x8000003,
                    _ => 2,
                };
                match c {
                    0xc001 => u |= 0x200001,
                    0x4001 => u |= 0x400001,
                    0x8001 => u |= 0x800001,
                    _ => {}
                }
                if p & 0x803 == 0x801 {
                    u |= 0x8000_0001;
                }
            }
            if b & 0xc == 4 {
                u |= 0x81;
            }
            u |= if p & 0x10003 == 0x10001 { 0x21 } else { 0x11 };
            if p & 0x1000_0000 != 0 {
                u |= 0x101;
            }
            if p & 0x200001 == 0x200001 {
                u |= 0x201;
            }
            if p & 0x2000_0000 != 0 {
                u |= 0x4000_0004;
            }
            if p & 0x100001 == 0x100001 {
                u |= 0x41;
            }
            u
        }
        2 => {
            let m = p & 0x1e3;
            let mut u = match m {
                0x1e2 => 0x14,
                0x1c2 => 0x24,
                0x1a2 => 0x44,
                0x182 if p & 0x1e03 == 0x1802 => 0x94,
                0x182 => 0x84,
                0x142 => 0x104,
                0x102 => 0x1004,
                0xe2 => 0x204,
                0xc2 if (p & 0x1e03 == 0x1002 || p & 0x100002 == 0x100002) && b & 0xc != 0xc => 0x8000_0404,
                0xc2 => 0x404,
                0xa2 => 0x804,
                _ => 4,
            };
            u |= match p & 0x1e03 {
                0x1e02 => 0x2004,
                0x1c02 => 0x4004,
                0x1a02 if m == 0x182 => 0x10004,
                0x1a02 => 0x8004,
                0x1802 => 0x10004,
                0x1602 => 0x20004,
                0x1402 => 0x40004,
                0x1002 => 0x80004,
                0xe02 => 0x100004,
                0xc02 => 0x200004,
                0xa02 => 0x400004,
                0x802 => 0x800004,
                _ => 0,
            };
            if p & 0x10 == 0 {
                u |= 0x1000004;
            }
            if b & 0xc == 4 {
                u |= 0x4000004;
            }
            if p & 0x1000_0000 != 0 {
                u |= 0x101;
            }
            for (m, add) in [
                (0x2000_0000, 0x4000_0004),
                (0x20002, 0x8000004),
                (0x2002, 0x1000_0004),
                (0x4002, 0x2000_0004),
                (0x100002, 0x2004),
                (0x200002, 0x80004),
                (0x400002, 0x100004),
                (0x800002, 0x400004),
                (0x40002, 0x1000004),
            ] {
                if p & m == m {
                    u |= add;
                }
            }
            u
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0x10050d50 undoes 0x10050780 on every phone of the older libraries.
    #[test]
    fn library_codes_round_trip() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../oracle/prefix/drive_c/Program Files (x86)/Loquendo/LTTS7/bin");
        for v in ["Allison", "Katrin", "Roberto"] {
            let Ok(lib) = std::fs::read(root.join(format!("LTTS7{v}.dll"))) else { continue };
            let vf = VoiceFon::from_library(&lib).unwrap();
            assert!(vf.convert);
            for (k, _) in &vf.targets {
                let p = &vf.phones[*k];
                if p.base != 0 {
                    assert_eq!(vf.code_of(vf.targets.iter().position(|t| t.0 == *k).unwrap()) & !0x2000004, p.base & !0x2000004, "{v} {}", p.name);
                }
            }
        }
    }
}
