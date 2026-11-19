//! FonTrasConfini (0x100089f0): before a word's codes join the chunk's
//! phone list, the list's last phone and the incoming codes are adjusted
//! across the boundary and inside the word.

use super::super::common::{Phone, Word};
use super::Dutch;

/// The words whose `-enen` FonTrasConfini rewrites (0x100089f0).
const NEGENEN: u32 = 0x10099aac;
const ZEVENEN: u32 = 0x10099aa4;

/// A stop or fricative (0x1e2 / 0x142 under the mask 0x1e3).
fn obstruent(c: u32) -> bool {
    matches!(c & 0x1e3, 0x1e2 | 0x142)
}

/// A stop or fricative not voiced (the voicing test is bit 4 of a
/// consonant code, low bits 10).
fn voiceless_obstruent(c: u32) -> bool {
    obstruent(c) && (c & 3 != 2 || c & 0x10 != 0)
}

fn voiced(c: u32) -> bool {
    c & 3 == 2 && c & 0x10 == 0
}

/// The glide FonTrasConfini puts between a word's last vowel and the next
/// word's first: `Hj` after front vowels, `V` after back rounded ones, `Hy`
/// after front rounded ones (0x10009040…).
fn glide(c: u32) -> Option<u32> {
    match c {
        0x10741 | 0x106c1 | 0x106c5 | 0x107c1 | 0x107c5 | 0x6c1 | 0x6c5 | 0x741 | 0x7c1 | 0x7c5 => Some(0x10ce),
        0x102e1 | 0x102e5 | 0x10361 | 0x103e1 | 0x3e1 | 0x2e1 | 0x2e5 | 0x361 => Some(0x1cc2),
        0x10761 | 0x107a1 | 0x107e1 | 0x107e5 | 0x106e5 | 0x6e5 | 0x761 | 0x7a1 | 0x7e1 => Some(0x201ec2),
        _ => None,
    }
}

/// The incoming codes as the module walks them: a 0-terminated array (a
/// read past the end gives 0).
struct Codes<'a>(&'a mut Vec<u32>);

impl Codes<'_> {
    fn g(&self, k: isize) -> u32 {
        if k < 0 {
            0
        } else {
            self.0.get(k as usize).copied().unwrap_or(0)
        }
    }
    fn set(&mut self, k: isize, c: u32) {
        if let Some(x) = usize::try_from(k).ok().and_then(|k| self.0.get_mut(k)) {
            *x = c;
        }
    }
    /// the shift-left loops: entry `k` removed
    fn delete(&mut self, k: isize) {
        if let Ok(k) = usize::try_from(k) {
            if k < self.0.len() {
                self.0.remove(k);
            }
        }
    }
}

impl Dutch {
    /// FonTrasConfini (0x100089f0) for the incoming word `rec` (its codes
    /// `next`) and the chunk's phone list so far (`phones`). Across the
    /// boundary: the last phone's voiced obstruent devoiced; when there are
    /// at least two, a final stop before an obstruent, nasal or affricate
    /// unreleased (`Hut Huk Hup Hud Hub`; t k p not before b d) and an `R`
    /// after a vowel (not schwa) before a consonant or pause the vocalic
    /// `HEhR`; a glide between vowels (`Hj V Hy` by the first one). Then
    /// along the last phone and the word: a fricative after a voiceless
    /// obstruent (`Gx` → `x`), a voiceless stop before a voiced one
    /// unreleased or voiced, n/m before velars/labiodentals `N`/`M`,
    /// alveolars before `j` palatal (the j gone), `l` before a consonant
    /// dark, schwa + `R` + consonant `HEhR`, `x R` → `HRB`, an `-en`/`-ens`
    /// ending's schwa `HEh)` (its n dropped), t between consonants dropped,
    /// a stop before an obstruent or nasal unreleased.
    pub(super) fn tras_confini(&self, rec: Option<&Word>, phones: &mut [Phone], next: &mut Vec<u32>) {
        let word: &[u8] = rec.map_or(&[], |r| r.text.as_slice());
        let count = phones.len();
        if let Some(p) = phones.last_mut() {
            if obstruent(p.code) && voiced(p.code) {
                p.code = match p.code {
                    0x1942 => 0x1952,
                    0xf42 => 0xf52,
                    0xfe2 => 0xff2,
                    0x1742 => 0x1752,
                    0x19e2 => 0x19f2,
                    0x1d42 => 0x1d52,
                    0x1fe2 => 0x1ff2,
                    x => x,
                };
            }
        }
        let mut n = Codes(next);
        if count > 1 {
            let c = n.g(0);
            if matches!(c & 0x1e3, 0x1e2 | 0x142 | 0x102 | 0x1c2) {
                let bd = c == 0x19e2 || c == 0x1fe2;
                let last = &mut phones[count - 1].code;
                *last = match *last {
                    0x19f2 if !bd => 0x59f2,
                    0xff2 if !bd => 0x4ff2,
                    0x19e2 => 0x59e2,
                    0x1fe2 => 0x5fe2,
                    0x1ff2 if !bd => 0x5ff2,
                    x => x,
                };
            }
            let before = phones[count - 2].code;
            if phones[count - 1].code == 0xd42 && before & 3 == 1 && before != 0x501 && (c & 3 == 2 || c == 0x13 || c == 0x17) {
                phones[count - 1].code = 0x200501;
            }
        }
        // position -1 is the list's last phone
        let get = |n: &Codes, phones: &[Phone], k: isize| if k == -1 { phones.last().map_or(0, |p| p.code) } else { n.g(k) };
        let mut k: isize = if count > 0 { -1 } else { 0 };
        let mut first = true;
        loop {
            if first {
                if k == -1 {
                    let c = get(&n, phones, -1);
                    if c & 3 == 1 && n.g(0) & 3 == 1 {
                        if let Some(gl) = glide(c) {
                            n.0.insert(0, gl);
                        }
                    }
                }
            } else {
                let cur = n.g(k);
                let prv = get(&n, phones, k - 1);
                let set_prv = |n: &mut Codes, phones: &mut [Phone], c: u32| {
                    if k == 0 {
                        if let Some(p) = phones.last_mut() {
                            p.code = c;
                        }
                    } else {
                        n.set(k - 1, c);
                    }
                };
                let mut tail = false;
                if cur & 0x1e3 == 0x142 && voiceless_obstruent(prv) {
                    if cur == 0xf42 {
                        n.set(k, 0xf52);
                    }
                } else if voiceless_obstruent(prv) && cur & 0x1e3 == 0x1e2 && voiced(cur) {
                    let bd = cur == 0x1fe2 || cur == 0x19e2;
                    match prv {
                        0xff2 => set_prv(&mut n, phones, 0x4ff2),
                        0x19f2 if bd => set_prv(&mut n, phones, 0x59f2),
                        0x1ff2 if bd => set_prv(&mut n, phones, 0x5fe2),
                        _ => {}
                    }
                } else if prv & 0x1e3 == 0x1c2 && matches!(cur & 0x1e3, 0x1c2 | 0x1e2 | 0x142) {
                    if prv == 0x19c2 && cur & 0x1e03 == 0xe02 {
                        set_prv(&mut n, phones, 0xfc2);
                    } else if prv == 0x1fc2 && cur & 0x1e03 == 0x1c02 {
                        set_prv(&mut n, phones, 0x1dc2);
                    }
                } else if cur == 0x10c2 {
                    let m = match prv {
                        0x1952 => Some(0x1752),
                        0xf52 => Some(0x1152),
                        0x1942 => Some(0x1742),
                        0x19c2 => Some(0x11c2),
                        0x19f2 => Some(0x11f2),
                        _ => None,
                    };
                    if let Some(m) = m {
                        set_prv(&mut n, phones, m);
                    }
                } else if prv == 0x18a2 && cur & 3 == 2 {
                    set_prv(&mut n, phones, 0x4018a2);
                } else {
                    tail = true;
                }
                // 0x10008ec3
                if tail && k > 1 && prv == 0x501 && cur == 0xd42 {
                    let n1 = n.g(k + 1);
                    if n1 & 3 == 2 || n1 == 0 || (k == 2 && n.g(0) == 0x19f2 && n.g(3) == 0x107a1 && n.g(4) == 0xf52) {
                        set_prv(&mut n, phones, 0x200501);
                        if n1 == 0xf42 {
                            n.set(k + 1, 0xf52);
                        }
                        n.delete(k);
                    }
                }
            }
            // the word itself from position k (at -1 the module reads the
            // word before its codes array: not one of the codes it acts on)
            if k >= 0 {
                self.inside(word, &mut n, k);
            }
            first = false;
            k += 1;
            if n.g(k) == 0 {
                break;
            }
        }
    }

    /// FonTrasConfini's second half (0x100091b0…) at code `k` of the word.
    fn inside(&self, word: &[u8], n: &mut Codes, k: isize) {
        let u = n.g(k);
        if u == 0 {
            return;
        }
        if u == 0x501 && k > 0 && n.g(k - 1) == 0x1952 && n.g(k + 1) == 0x18a2 && n.g(k + 2) == 0x501 {
            n.delete(k);
            return;
        }
        if (u == 0xf52 || u == 0xf42) && n.g(k + 1) == 0xd42 {
            if k < 1 || n.g(k - 1) & 0x1e3 != 0x142 {
                n.set(k, 0xd52);
                n.delete(k + 1);
            } else {
                n.delete(k);
            }
            return;
        }
        if u == 0xd42 && k > 0 {
            let p = n.g(k - 1);
            if p & 3 == 1 && p != 0x501 && n.g(k + 1) & 3 == 2 {
                n.set(k, 0x200501);
                if n.g(k + 1) == 0xf42 {
                    n.set(k + 1, 0xf52);
                }
                return;
            }
            if n.g(k + 1) == 0xf52 && n.g(k + 2) != 0xd42 {
                n.set(k, 0xd52);
                n.delete(k + 1);
                return;
            }
        }
        if u == 0x501 {
            if word == self.s(NEGENEN) || word == self.s(ZEVENEN) {
                n.set(k, 0x10000501);
                n.0.truncate((k + 1) as usize);
                n.0.extend([0x6c1, 0x19c2]);
                return;
            }
            let l = word.len();
            let e = |c: u8| c == b'e' || c == 0xeb;
            let at = |i: usize| word[l - i];
            let n1 = n.g(k + 1);
            if n1 == 0 && l > 1 && at(1) == b'n' && e(at(2)) {
                n.set(k, 0x10000501);
            } else if n1 == 0x1952 && n.g(k + 2) == 0 && l > 2 && at(1) == b's' && at(2) == b'n' && e(at(3)) {
                n.set(k, 0x10000501);
            } else if n1 == 0x19c2 && n.g(k + 2) == 0x1952 && n.g(k + 3) == 0 && l > 2 && at(1) == b's' && at(2) == b'n' && e(at(3)) {
                n.set(k, 0x10000501);
                n.delete(k + 1);
            }
            return;
        }
        let cls = u & 0x1e3;
        let cons_like = cls == 0x1e2 || cls == 0x1c2 || (cls == 0x142 && u != 0xd42) || cls == 0x102;
        let bd = u == 0x1fe2 || u == 0x19e2;
        if k > 1 {
            let (p, pp) = (n.g(k - 1), n.g(k - 2));
            if u & 3 == 2 && !bd && u != 0x10c2 && u != 0xd42 && p == 0x19f2 && voiceless_obstruent(pp) {
                n.delete(k - 1);
                return;
            }
            if cons_like && p == 0x19f2 && !voiceless_obstruent(pp) {
                if u == 0x19f2 {
                    n.delete(k - 1);
                } else if !bd {
                    n.set(k - 1, 0x59f2);
                }
                return;
            }
            if cons_like && p & 0x1e3 == 0x1e2 {
                match p {
                    0x1fe2 => n.set(k - 1, 0x5fe2),
                    0xff2 if !bd => n.set(k - 1, 0x4ff2),
                    0x19e2 if u == 0x19f2 || u == 0x19e2 => n.delete(k - 1),
                    0x19e2 => n.set(k - 1, 0x59e2),
                    0x1ff2 if !bd => n.set(k - 1, 0x5ff2),
                    _ => {}
                }
                return;
            }
        }
        if matches!(u, 0x19f2 | 0x19e2 | 0x1942 | 0x1952 | 0x19c2) && n.g(k + 1) == 0x10c2 {
            n.set(
                k,
                match u {
                    0x1942 => 0x1742,
                    0x1952 => 0x1752,
                    0x19c2 => 0x11c2,
                    _ => 0x11f2,
                },
            );
            n.delete(k + 1);
        } else if u == 0x18a2 && n.g(k + 1) == 0 {
            n.set(k, 0x4018a2);
        }
    }
}
