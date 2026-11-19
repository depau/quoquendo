//! LTTS7Annika.dll (Swedish): `FonVidAllofonoVoce` 0x10001210,
//! `FonVidMetaFonema` 0x10002140 (→ 0x10001b80, the default). Stops are
//! aspirated before a stressed vowel and unreleased before a consonant;
//! the shared rules are also Sven's (`voicefon/sven.rs`).

use super::{meta_default, Port, Rules, Stage};

pub(super) const ANNIKA: Port = Port {
    voice: "Annika",
    table: 0x10009a40,
    functions: &[
        ("GetVersion", 0x100010f0),
        ("ELQGetPhonemes", 0x10001000),
        ("FonVidAllofonoVoce", 0x10001210),
        ("FonVidAllofonoAnalisi", 0x10001bf0),
        ("FonVidMetaFonema", 0x10002140),
        ("FonVidAnalisi2Sintesi", 0x10002160),
        ("FonVidIngloba", 0x100021c0),
    ],
    rules: Rules {
        allofono: Some(annika),
        meta: meta_default,
    },
};

/// The consonants a stop stays released before inside its word (one set
/// per place: the stop's own pair, r, l and j).
pub(super) const LABIAL: [u32; 5] = [0x1ff2, 0x1fe2, 0x19a2, 0x18a2, 0x10c2];
pub(super) const VELAR: [u32; 5] = [0xff2, 0xfe2, 0x19a2, 0x18a2, 0x10c2];
pub(super) const DENTAL: [u32; 5] = [0x19f2, 0x19e2, 0x19a2, 0x18a2, 0x10c2];

/// A consonant code (`code & 3 == 2`).
fn consonant(c: u32) -> bool {
    c & 3 == 2
}

/// The aspiration rule (p 0x1ff2 → 0x3ff2, t 0x19f2 → 0x39f2, k 0xff2 →
/// 0x2ff2): before a stressed vowel (`code & 0x10003 == 0x10001`), unless
/// a phone of the same word precedes and it is s or the vowel belongs to
/// the next word. Whether it applied.
pub(super) fn aspirate(s: &mut Stage, i: usize, to: u32) -> bool {
    if s.code(i + 1) & 0x10003 != 0x10001 {
        return false;
    }
    let w = s.word(i);
    if i != 0 && s.word(i - 1) == w && (s.code(i - 1) == 0x1952 || w != s.word(i + 1)) {
        return false;
    }
    s.set(i, to);
    true
}

/// The release rule (unreleased `Hub Hug Huk Hup Hut Hud`): inside a word
/// (a phone of it on both sides) before a consonant outside `kept`, or
/// before the other stop of its place; at a word's end before a word
/// starting with a consonant.
pub(super) fn release(s: &mut Stage, i: usize, kept: &[u32; 5], to: u32) {
    if i == 0 {
        return;
    }
    let w = s.word(i);
    let inside = s.word(i - 1) == w && s.word(i + 1) == w;
    let nc = s.code(i + 1);
    if inside && consonant(nc) && !kept.contains(&nc) {
        s.set(i, to);
        return;
    }
    if inside && kept[..2].contains(&nc) {
        s.set(i, to);
        return;
    }
    if s.word(i + 1) == w || !consonant(nc) {
        return;
    }
    s.set(i, to);
}

/// The d rule (0x19e2 → 0x59e2, unreleased): inside a word before a
/// consonant other than its own place, r, l, j (not after n); `n d n`
/// loses its d; after n before a consonant other than n r j; at a word's
/// end before a consonant word.
pub(super) fn d_rule(s: &mut Stage, i: usize) {
    if i == 0 {
        return;
    }
    let w = s.word(i);
    let sp = s.word(i - 1) == w;
    let sn = s.word(i + 1) == w;
    let (pc, nc) = (s.code(i - 1), s.code(i + 1));
    if sp && sn && pc != 0x19c2 && consonant(nc) && !DENTAL.contains(&nc) {
        s.set(i, 0x59e2);
    }
    if sp && sn {
        if pc == 0x19c2 && nc == 0x19c2 {
            s.delete(i);
            return;
        }
        if nc == 0x19f2 || nc == 0x19e2 {
            s.set(i, 0x59e2);
            return;
        }
        if pc == 0x19c2 && consonant(nc) && !matches!(nc, 0x19c2 | 0x19a2 | 0x10c2) {
            s.set(i, 0x59e2);
            return;
        }
    }
    if sn || !consonant(nc) {
        return;
    }
    s.set(i, 0x59e2);
}

/// 0x10001210.
fn annika(s: &mut Stage, pi: &mut usize) {
    let i = *pi;
    match s.code(i) {
        0x1fe2 => release(s, i, &LABIAL, 0x5fe2),
        // `k `2a n x HEh`: the sh of "kanske" said 0x2001752
        0x501752 => {
            if i != 3 && (i < 4 || s.word(i - 4) == s.word(i - 3)) {
                return;
            }
            if s.word(i + 1) == s.word(i + 2) {
                return;
            }
            if s.code(i - 3) == 0x2ff2 && s.code(i - 2) == 0x30441 && s.code(i - 1) == 0x19c2 && s.code(i + 1) == 0x501 {
                s.set(i, 0x2001752);
            }
        }
        0x1ff2 => {
            if !aspirate(s, i, 0x3ff2) {
                release(s, i, &LABIAL, 0x5ff2);
            }
        }
        0xfe2 => release(s, i, &VELAR, 0x4fe2),
        0xff2 => {
            aspirate(s, i, 0x2ff2);
            release(s, i, &VELAR, 0x4ff2);
        }
        0x19e2 => d_rule(s, i),
        0x19f2 => {
            if aspirate(s, i, 0x39f2) || i == 0 {
                return;
            }
            let w = s.word(i);
            // `k t s` inside a word: the t goes
            if s.word(i - 1) == w && s.word(i + 1) == w && s.code(i - 1) == 0x4ff2 && s.code(i + 1) == 0x1952 {
                s.delete(i);
                return;
            }
            release(s, i, &DENTAL, 0x59f2);
        }
        _ => {}
    }
}
