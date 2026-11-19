//! LTTS7Frida.dll (Danish): `FonVidAllofonoVoce` 0x10001210,
//! `FonVidMetaFonema` 0x10001c10. Unaspirated p t k before a consonant of
//! class 0x1e2 unreleased (`Hup Hut Huk`), the a of `dansk`/`danmark`
//! said `Ae`, a few schwa / r / s simplifications across word ends, `har`
//! without its a; a stød 0x9f2 after a word's last phone merges into it
//! (its vowel's stød id).

use super::{meta_default, Port, Rules, Stage, KIND_PAUSE, KIND_WORD_END};

pub(super) const FRIDA: Port = Port {
    voice: "Frida",
    table: 0x10009c28,
    functions: &[
        ("GetVersion", 0x10001150),
        ("ELQGetPhonemes", 0x10001060),
        ("FonVidAllofonoVoce", 0x10001210),
        ("FonVidAllofonoAnalisi", 0x10001930),
        ("FonVidMetaFonema", 0x10001c10),
        ("FonVidAnalisi2Sintesi", 0x10001ba0),
        ("FonVidIngloba", 0x10001be0),
        ("ELQVoiceChannelOpen", 0x10001070),
        ("ELQVoiceChannelClose", 0x10001100),
    ],
    rules: Rules {
        allofono: Some(frida),
        meta,
    },
};

/// `dansk` (0x10009cb4), `danmark` (0x10009cc0), `har` (0x10009cbc).
const DANSK: u32 = 0x10009cb4;
const DANMARK: u32 = 0x10009cc0;
const HAR: u32 = 0x10009cbc;

/// The kind of slot `i` (past the list: 0).
fn kind(s: &Stage, i: usize) -> u8 {
    s.list.get(i).map_or(0, |p| p.kind)
}

/// A word's last phone or a pause (`w`, `p`).
fn ends(k: u8) -> bool {
    k == KIND_WORD_END || k == KIND_PAUSE
}

/// A consonant of class 0x1e2 (`code & 3 == 2`, `& 0x1e3 == 0x1e2`).
fn class_1e2(c: u32) -> bool {
    c & 3 == 2 && c & 0x1e3 == 0x1e2
}

/// 0x10001210.
fn frida(s: &mut Stage, pi: &mut usize) {
    let i = *pi;
    let text = s.text(s.word(i) as i64).unwrap_or(&[]).to_vec();
    let cstr = |va: u32| s.pe.cstr(va).to_vec();
    // phone i-1 starts its word (index 1, or i-2 ends one)
    let prev_starts = i == 1 || (i >= 2 && ends(kind(s, i - 2)));
    match s.code(i) {
        // p, k inside a word before a 0x1e2 consonant: unreleased
        0x20001ff2 | 0x20000ff2 => {
            if i != 0 && !ends(kind(s, i)) && class_1e2(s.code(i + 1)) {
                s.set(i, if s.code(i) == 0x20001ff2 { 0x5ff2 } else { 0x4ff2 });
            }
        }
        0x200019f2 => {
            if i != 0 && class_1e2(s.code(i + 1)) {
                s.set(i, 0x59f2);
            }
        }
        // the stressed a of `dansk…` / `danmark…` (strncmp)
        0x10441 => {
            if i != 0 && prev_starts && (text.starts_with(&cstr(DANSK)) || text.starts_with(&cstr(DANMARK))) {
                s.set(i, 0x10681);
            }
        }
        0x245 => {
            if i != 0 && text.starts_with(&cstr(DANMARK)) {
                s.set(i, 0x441);
            }
        }
        // a word-final t of a two-phone word after a, before an unstressed
        // vowel or h: goes (its kind to the a)
        0x59f2 => {
            if i == 0 || s.code(i - 1) != 0x441 || !prev_starts || !ends(kind(s, i)) {
                return;
            }
            let n = s.code(i + 1);
            if n & 0x10003 == 1 || n == 0x952 {
                let k = kind(s, i);
                s.list[i - 1].kind = k;
                s.delete(i);
                *pi = i.wrapping_sub(1);
            }
        }
        // s, a word-final consonant with bit 0x10, s: the consonant goes
        // (its kind to the first s)
        0x1952 => {
            let k = kind(s, i + 1);
            let n = s.code(i + 1);
            if ends(k) && n & 3 == 2 && n & 0x10 != 0 && s.code(i + 2) == 0x1952 {
                s.list[i].kind = k;
                s.delete(i + 1);
            }
        }
        // the a of `har`
        0x241 => {
            if s.rec(s.word(i) as i64).is_some() && text == cstr(HAR) {
                s.delete(i);
            }
        }
        0x501 => schwa(s, i),
        _ => {}
    }
}

/// 0x501 inside a word, after a consonant: `C ə n ə` (C of class 0x1c2 or
/// 0xa2, the second schwa word-final) and `C ə n` (C with bit 0x10, of
/// class 0x142) lose the schwa, the n becoming 0x219c2.
fn schwa(s: &mut Stage, i: usize) {
    if i == 0 {
        return;
    }
    let kp = kind(s, i - 1);
    if kp == KIND_WORD_END {
        return;
    }
    let pc = s.code(i - 1);
    let inside = kp != KIND_PAUSE && !ends(kind(s, i)) && !ends(kind(s, i + 1));
    if inside && ends(kind(s, i + 2)) && pc & 3 == 2 && matches!(pc & 0x1e3, 0x1c2 | 0xa2) && s.code(i + 1) == 0x19c2 && s.code(i + 2) == 0x501 {
        s.delete(i);
        s.set(i, 0x219c2);
        return;
    }
    if ends(kp) || ends(kind(s, i)) {
        return;
    }
    if pc & 3 == 2 && pc & 0x10 != 0 && pc & 0x1e3 == 0x142 && s.code(i + 1) == 0x19c2 {
        s.delete(i);
        s.set(i, 0x219c2);
    }
}

/// 0x10001c10: a stød 0x9f2 after a word's last phone or a pause says
/// nothing and turns the previous output id into its stød form (0x10001ba0:
/// 0x76 → 0x5c, 0x77 → 0x60, 0x78 → 0x53, 0x79 → 0x54, 0x7a → 0x55); any
/// other phone the default. The stage's `ids` hold only this call's output,
/// so the phone before the stød takes it here, together with its own.
fn meta(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    meta_default(s, i, out);
    if s.code(*i) == 0x9f2 && ends(kind(s, *i - 1)) {
        if let Some(id) = out.last_mut() {
            *id = match *id {
                0x76 => 0x5c,
                0x77 => 0x60,
                0x78 => 0x53,
                0x79 => 0x54,
                0x7a => 0x55,
                x => x,
            };
        }
        *i += 1;
    }
}

#[cfg(test)]
mod tests {
    use crate::lang::common::{Phone, Word};
    use crate::voicefon::VoiceFon;

    /// Every bench sentence's module phones (event 2 of `/tmp/qq-da/bench-<n>.ev`:
    /// kind at +8, code at +0xc) through the stage give the voice's (event 3).
    /// Words split after each word end; texts from `oracle/bench-da.txt`.
    #[test]
    fn frida_bench_events() {
        let store = std::env::var("QQ_STORE").unwrap_or_else(|_| "/tmp/qqv/LTTS7".into());
        let Ok(lib) = std::fs::read(format!("{store}/bin/LTTS7Frida.dll")) else { return };
        let vf = VoiceFon::from_library(&lib).unwrap();
        assert_eq!(vf.ported, Some("Frida"));
        let bench = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../oracle/bench-da.txt")).unwrap();
        let (mut checked, mut changed) = (0, 0);
        for (n, line) in bench.lines().enumerate() {
            let Ok(ev) = std::fs::read_to_string(format!("/tmp/qq-da/bench-{n}.ev")) else { continue };
            let mut module = Vec::new();
            let mut phones = Vec::new();
            let mut said = Vec::new();
            let mut word = 0u32;
            for l in ev.lines() {
                let name = l.split(" phone=").nth(1).and_then(|x| x.split(' ').next()).unwrap_or("");
                let hex = l.split(" hex=").nth(1).unwrap_or("");
                let b = |k: usize| u8::from_str_radix(&hex[2 * k..2 * k + 2], 16).unwrap();
                if l.starts_with("2 ") {
                    module.push(name.to_string());
                    let kind = b(8);
                    let code = u32::from_le_bytes([b(12), b(13), b(14), b(15)]);
                    let w = if kind == 0x70 { word.saturating_sub(1) } else { word };
                    phones.push(Phone { word: w, code, kind });
                    if kind == 0x77 {
                        word += 1;
                    }
                } else if l.starts_with("3 ") {
                    said.push(name.to_string());
                }
            }
            let last = phones.last().map_or(0, |p| p.word);
            phones.push(Phone { word: last, code: 0, kind: 0x70 });
            // Latin-1, lower case, punctuation off
            let mut texts: Vec<Vec<u8>> = line
                .split_whitespace()
                .map(|t| t.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).map(|c| c as u32 as u8).collect())
                .collect();
            texts.resize(word as usize + 2, Vec::new());
            let sentence: Vec<Word> = texts
                .into_iter()
                .map(|text| {
                    let mut f = [0u8; 24];
                    f[0x11] = 1;
                    Word { text, tag: None, f, data: Vec::new(), rows: Vec::new() }
                })
                .collect();
            let out = vf.run(&sentence, 0, sentence.len(), &phones);
            let got: Vec<String> = out.iter().map(|p| vf.name(p.code).to_string()).collect();
            assert_eq!(got, said, "bench-{n}");
            checked += 1;
            changed += usize::from(module != said);
        }
        // the voice changes bench 2 (`danske`), 4 and 6 (unreleased p)
        assert!(checked == 0 || (checked == 12 && changed == 3), "{checked} {changed}");
    }
}
