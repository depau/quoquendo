//! Test fixtures: the original module's own calls, traced under Wine
//! (`/tmp/qq-fon-de/lestrace.h`, `gen_fix.py`): every LesElaboraParola,
//! LES stage, FonTrasParola and FonAllofonoLingua call with its inputs and
//! outputs, on 20 000 held-out words (`fix/`) and the benchmark sentences
//! (`fixb/`). The tests that use them are `#[ignore]`d and skip when the
//! files are not there.

use super::super::common::{Phone, Word};
use super::{set_lexinfo, German, Morpheme};

pub const DIRS: [&str; 2] = ["/tmp/qq-fon-de/fixb", "/tmp/qq-fon-de/fix"];
const DLL: &str = "/tmp/qqv/LTTS7/bin/LTTS7German.dll";
const DBL: &str = "/tmp/qqv/LTTS7/data/languages/German/German.dbl";

/// The module from the voice store, when it is there.
pub fn german() -> Option<German> {
    let dll = std::fs::read(DLL).ok()?;
    let dbl = std::fs::read(DBL).ok()?;
    German::new(dll, Some(&dbl))
}

pub fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
}

/// `<24 bytes hex>:<text hex>`
pub fn record(s: &str) -> Word {
    let (b, t) = s.split_once(':').unwrap();
    let mut f = [0u8; 24];
    f.copy_from_slice(&unhex(b));
    Word { text: unhex(t), tag: None, f, data: Vec::new(), rows: Vec::new() }
}

pub fn records(s: &str) -> Vec<Word> {
    if s.is_empty() {
        return Vec::new();
    }
    s.split(';').map(record).collect()
}

/// solutions `|`, morphemes `,`, fields `.`; `-` none
pub fn lexinfo(s: &str) -> Vec<Vec<Morpheme>> {
    if s == "-" || s.is_empty() {
        return Vec::new();
    }
    s.split('|')
        .map(|sol| {
            sol.split(',')
                .filter(|m| !m.is_empty())
                .map(|m| {
                    let mut out = [0u32; 10];
                    for (k, v) in m.split('.').enumerate().take(10) {
                        out[k] = u32::from_str_radix(v, 16).unwrap();
                    }
                    out
                })
                .collect()
        })
        .collect()
}

/// Records with their LexInfo attached.
pub fn records_with_lex(recs: &str, lex: &str) -> Vec<Word> {
    let mut w = records(recs);
    for (r, l) in w.iter_mut().zip(lex.split(';')) {
        set_lexinfo(r, &lexinfo(l));
    }
    w
}

/// `<word>:<code>:<kind>` hex, `,`-separated
pub fn phones(s: &str) -> Vec<Phone> {
    if s.is_empty() {
        return Vec::new();
    }
    s.split(',')
        .map(|p| {
            let mut it = p.split(':').map(|v| u32::from_str_radix(v, 16).unwrap());
            Phone { word: it.next().unwrap(), code: it.next().unwrap(), kind: it.next().unwrap() as u8 }
        })
        .collect()
}

/// The lines of fixture `name` from every directory that has it.
pub fn lines(name: &str) -> Vec<String> {
    DIRS.iter()
        .filter_map(|d| std::fs::read_to_string(format!("{d}/{name}")).ok())
        .flat_map(|t| t.lines().map(str::to_string).collect::<Vec<_>>())
        .collect()
}

/// A record for messages: its text and bytes 0x0c..0x14.
pub fn show(w: &Word) -> String {
    format!("{} {:02x?}", String::from_utf8_lossy(&w.text.iter().map(|&c| c as char).collect::<String>().into_bytes()), &w.f[0xc..0x14])
}
