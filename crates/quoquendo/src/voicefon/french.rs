//! LTTS7Juliette.dll and LTTS7Florence.dll (French): `FonVidAllofonoVoce`
//! Juliette 0x10001230, Florence 0x10001210; `FonVidMetaFonema` the
//! default in both (Juliette 0x10001480, Florence 0x10001d10).

use super::{contains, eq, meta_default, Port, Rules, Stage};

pub(super) const JULIETTE: Port = Port {
    voice: "Juliette",
    table: 0x10008980,
    functions: &[
        ("GetVersion", 0x10001150),
        ("ELQGetPhonemes", 0x10001060),
        ("FonVidAllofonoVoce", 0x10001230),
        ("FonVidAllofonoAnalisi", 0x100013b0),
        ("FonVidMetaFonema", 0x10001480),
        ("FonVidAnalisi2Sintesi", 0x100014a0),
    ],
    rules: Rules {
        allofono: Some(juliette),
        meta: meta_default,
    },
};

pub(super) const FLORENCE: Port = Port {
    voice: "Florence",
    table: 0x10009760,
    functions: &[
        ("GetVersion", 0x100010f0),
        ("ELQGetPhonemes", 0x10001000),
        ("FonVidAllofonoVoce", 0x10001210),
        ("FonVidAllofonoAnalisi", 0x10001630),
        ("FonVidMetaFonema", 0x10001d10),
        ("FonVidAnalisi2Sintesi", 0x10001d30),
    ],
    rules: Rules {
        allofono: Some(florence),
        meta: meta_default,
    },
};

/// Juliette 0x10001230: a word-final s before a word starting with a
/// voiced consonant (`& 0x3004104`) is z; a word-final dd after a nasal
/// vowel (`& 0x101`) before a consonant is n, else it voices the
/// voiceless stop or s ending the word before it (tt p k s → dd b g z).
fn juliette(s: &mut Stage, i: &mut usize) {
    let i = *i;
    if i == 0 {
        return;
    }
    let (w, next) = (s.word(i), s.code(i + 1));
    match s.code(i) {
        0x10104 if w != s.word(i + 1) && next & 0x3004104 == 0x3004104 => s.set(i, 0x1010104),
        0x3008014 => {
            if w != s.word(i + 1) && s.code(i - 1) & 0x101 == 0x101 && next & 4 != 0 {
                s.set(i, 0x1010024);
                return;
            }
            let prev = s.code(i - 1);
            if s.word(i - 1) != w && prev & 4 != 0 && prev & 0x1000004 != 0x1000004 {
                let voiced = match prev {
                    0x2008014 => 0x3008014,
                    0x10104 => 0x1010104,
                    0x2002014 => 0x3002014,
                    0x2100014 => 0x3100014,
                    _ => return,
                };
                s.set(i - 1, voiced);
            }
        }
        _ => {}
    }
}

/// Florence 0x10001210 (engine codes): word-final stops and fricatives
/// assimilate in voicing to the next word's after a nasalised vowel
/// (`& 0x10000000`): 0x1a02 stops and `dd` (0x1be2) before 19c2/1fc2 as
/// 0x19c2, b (0x1e02 stop) before another as 0x1fc2, the 0xe02 stop
/// before a consonant other than 0xc2 as 0xfc2 (not in `donc`,
/// `quelconque`, nor before `million…`, `mètre`, `mètres`); then 0xfc2
/// before 0xfe2 ending a word merges with it as 0x11c2, and 0x1942 takes
/// the next word's 0x1752/0x1742.
fn florence(s: &mut Stage, pi: &mut usize) {
    let i = *pi;
    let w = s.word(i);
    let rec = s.rec(w as i64);
    let sub = rec.map_or(0, |r| r.f[0xf]);
    let text = rec.map(|r| r.text.clone());
    let next_text = if w as usize == s.count() { Some(Vec::new()) } else { s.text(w as i64 + 1).map(<[u8]>::to_vec) };
    if i != 0 {
        let (cur, next, prev) = (s.code(i), s.code(i + 1), s.code(i - 1));
        let next_word = s.word(i + 1);
        let nasal = prev & 0x1000_0000 != 0;
        let stays = w == next_word || cur & 0x1e3 != 0x1e2 || cur & 0x1e03 != 0x1a02 || sub == b'y' || !nasal || (next & 0x1e3 != 0x1e2 && next & 0x1e3 != 0x142);
        if !stays {
            s.set(i, 0x19c2);
        } else if cur == 0x1be2 && sub != b'y' && nasal && matches!(next, 0x19c2 | 0x1fc2) {
            s.set(i, 0x19c2);
        } else if w != next_word {
            if cur & 0x1e3 == 0x1e2 && cur & 0x1e03 == 0x1e02 && nasal && next & 0x1e3 == 0x1e2 && next & 0x1e03 == 0x1e02 {
                s.set(i, 0x1fc2);
            } else if cur & 0x1e3 == 0x1e2 && cur & 0x1e03 == 0xe02 && nasal {
                let kept = |at: u32| eq(text.as_deref(), s.pe.cstr(at));
                let before = |at: u32| eq(next_text.as_deref(), s.pe.cstr(at));
                let skip = kept(0x10009838) // donc
                    || kept(0x1000982c) // quelconque
                    || (next & 3 == 2 && next & 0x1e3 != 0xc2 && (contains(next_text.as_deref(), s.pe.cstr(0x10009824)) || before(0x1000981c) || before(0x10009814)));
                if !skip && next & 3 == 2 && next & 0x1e3 != 0xc2 {
                    s.set(i, 0xfc2);
                }
            }
        }
    }
    let i = *pi;
    let cur = s.code(i);
    if cur == 0xfc2 {
        if i != 0 && s.word(i + 1) != s.word(i + 2) && s.code(i + 1) == 0xfe2 {
            s.delete(i);
            s.set(i, 0x11c2);
        }
    } else if cur == 0x1942 && i != 0 && s.word(i) != s.word(i + 1) {
        match s.code(i + 1) {
            0x1752 => s.set(i, 0x1752),
            0x1742 => s.set(i, 0x1742),
            _ => {}
        }
    }
}
