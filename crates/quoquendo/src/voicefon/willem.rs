//! LTTS7Willem.dll (Dutch): `FonVidAllofonoVoce` 0x10001210 (a handful of
//! foreign and learned words said the voice's way), `FonVidMetaFonema`
//! 0x100021a0 (`Gx`, `Hj` and `Hud` to their own ids, the rest by code)
//! with `FonVidAnalisi2Sintesi` 0x10002110 on every id.

use super::{eq, Port, Rules, Stage, KIND_PHONE};

pub(super) const WILLEM: Port = Port {
    voice: "Willem",
    table: 0x100099b0,
    functions: &[
        ("GetVersion", 0x100010f0),
        ("ELQGetPhonemes", 0x10001000),
        ("FonVidAllofonoVoce", 0x10001210),
        ("FonVidAllofonoAnalisi", 0x10001da0),
        ("FonVidMetaFonema", 0x100021a0),
        ("FonVidAnalisi2Sintesi", 0x10002110),
    ],
    rules: Rules {
        allofono: Some(allofono),
        meta,
    },
};

/// The words the stage knows (strings in the library).
const SINGAPORE: u32 = 0x10009a74;
const CONVENTIONELE: u32 = 0x10009a5c;
const JOHNSON: u32 = 0x10009b98;
const JUSTIN: u32 = 0x10009a6c;
const MATERIAAL: u32 = 0x10009b8c;
const MATERIELE: u32 = 0x10009b80;
const MATERIEEL: u32 = 0x10009b74;
const INTERESSANT: u32 = 0x10009ba8;
const BRITISH: u32 = 0x10009ba0;
const INITIA: u32 = 0x10009b64;
/// `initiaal` … `initiërende`, compared in this order (0x10001680…0x10001a19)
const INITIAL: [u32; 18] = [
    0x10009b58, 0x10009b4c, 0x10009b40, 0x10009b34, 0x10009b28, 0x10009b1c, 0x10009b0c, 0x10009b00, 0x10009af4, 0x10009ae8, 0x10009adc, 0x10009ad0, 0x10009ac4, 0x10009ab8, 0x10009aac, 0x10009aa0, 0x10009a94,
    0x10009a88,
];
const MILIEU: u32 = 0x10009a80;
const GRAHAM: u32 = 0x10009b6c;

/// 0x10001210: by the phone's code, in a word that does not continue a
/// compound (its record's boundary is not 4): `singapore`'s `HEhR` said
/// `R HEh` and its `g` dropped; `conventionele`'s n before `$` gets a `Hut`;
/// `johnson`'s j `Dg`, o `Ou` and h dropped, `justin`'s `Dg` a j;
/// `materiaal`/`materiële`/`materieel`'s e (between `m a t` and `R i j`)
/// dropped; `interessant` without its first schwa, `british`'s an i; the
/// i of `initia…` words before `n i Hut s`/`$` an `I`, `milieu`'s a schwa;
/// `graham`'s h an `I`.
fn allofono(s: &mut Stage, i: &mut usize) {
    let at = *i;
    let w = s.word(at);
    let text = s.text(w as i64).map(<[u8]>::to_vec);
    if s.rec(w as i64).is_some_and(|r| r.f[0x11] == 4) {
        return;
    }
    let is = |a: u32| eq(text.as_deref(), s.pe.cstr(a));
    let c = |k: isize| if k < 0 { 0 } else { s.code(k as usize) };
    let k = at as isize;
    let delete = |s: &mut Stage, i: &mut usize| {
        s.delete(*i);
        *i = i.wrapping_sub(1);
    };
    match s.code(at) {
        0x200501 if is(SINGAPORE) => {
            s.set(at, 0xd42);
            s.insert(at + 1, w, 0x501, KIND_PHONE);
        }
        0x19c2 if is(CONVENTIONELE) && c(k + 1) == 0x1752 => s.insert(at + 1, w, 0x59f2, KIND_PHONE),
        0x10c2 if is(JOHNSON) => s.set(at, 0x1702),
        0x1702 if is(JUSTIN) => s.set(at, 0x10c2),
        0xfe2 if is(SINGAPORE) => delete(s, i),
        0x741 => {
            let mat = |last: u32| c(k - 3) == 0x1fc2 && c(k - 2) == 0x441 && c(k - 1) == 0x19f2 && c(k + 1) == 0xd42 && c(k + 2) == 0x7c1 && c(k + 3) == 0x10c2 && c(k + 4) == last;
            if (is(MATERIAAL) && mat(0x10441)) || ((is(MATERIELE) || is(MATERIEEL)) && mat(0x10741)) {
                delete(s, i);
            }
        }
        0x2e1 if is(JOHNSON) => s.set(at, 0x7a1),
        0x501 if is(INTERESSANT) => delete(s, i),
        0x501 if is(BRITISH) => s.set(at, 0x7c1),
        0x7c1 => {
            if is(INITIA) && c(k + 1) == 0x19c2 && c(k + 2) == 0x107c1 && c(k + 3) == 0x59f2 && c(k + 4) == 0x1952 {
                s.set(at, 0x781);
            } else if INITIAL.iter().any(|&a| is(a)) && c(k + 1) == 0x19c2 && c(k + 2) == 0x7c1 && c(k + 3) == 0x59f2 && c(k + 4) == 0x1752 {
                s.set(at, 0x781);
            } else if is(MILIEU) {
                s.set(at, 0x501);
            }
        }
        0x942 => {
            if is(GRAHAM) {
                s.set(at, 0x781);
            }
            if is(JOHNSON) {
                delete(s, i);
            }
        }
        _ => {}
    }
}

/// 0x100021a0: `Gx` (0xf42) id 0x4d, `Hj` (0x10ce) 0x35, `Hud` (0x59e2)
/// 0x3a, any other the id of the first table entry with its code
/// (0x10001d30; else 1); then 0x10002110 (the analysis ids as synthesis
/// ones).
fn meta(s: &Stage, i: &mut usize, out: &mut Vec<u8>) {
    let id = match s.code(*i) {
        0xf42 => 0x4d,
        0x10ce => 0x35,
        0x59e2 => 0x3a,
        c => s.phones.iter().position(|p| p.base == c).map_or(1, |k| k as u8),
    };
    *i += 1;
    out.push(match id {
        0x36 | 0x5f => 0x35,
        0x3e => 0x3d,
        0x41 => 0x3a,
        0x42 => 0x40,
        0x49 => 0x48,
        0x4c => 0x4b,
        0x4e | 0x60 => 0x4d,
        0x4f => 0x50,
        0x5e => 0x5c,
        x => x,
    });
}
