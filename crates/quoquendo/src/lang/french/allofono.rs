//! `FonAllofonoLingua` (0x1000e690), run by the engine on every entry of a
//! chunk's phone list once all its words have their phones: liaison (a
//! silent final consonant said before a vowel when the boundary between
//! the words is 2, with its mute e when two consonants come before),
//! denasalisation (`bon ami`, `prochain arrêt`), no liaison before an
//! h aspiré (the dictionary's `altreinfo` HNONMUTA), and the mute e
//! dropped between words.

use super::{col, French, Lex};
use crate::lang::common::{field::*, Phone, Word};

const SCHWA: u32 = 0x501;
const T_: u32 = 0x1bf2;
const N_: u32 = 0x19c2;
const R_: u32 = 0xda2;
const L_: u32 = 0x18a2;
const Z_: u32 = 0x1942;
const S_: u32 = 0x1952;
const J_: u32 = 0x10c2;

fn text(w: &[Word], i: usize) -> &[u8] {
    w.get(i).map_or(&[][..], |r| &r.text[..])
}

impl French {
    pub(crate) fn allofono(&self, w: &[Word], ph: &mut Vec<Phone>, k: usize) -> usize {
        let mut k = k;
        let wi = ph[k].word as usize;
        let Some(rec) = w.get(wi) else { return k };
        let t = &rec.text[..];
        let next = text(w, wi + 1);
        let next2 = text(w, wi + 2);
        let prev_word = if k == 0 { 0 } else { ph[k - 1].word as usize };
        let code = ph[k].code;
        // the first phone of a word starting with a vowel or a glide
        if wi != 0 && k != 0 && prev_word < wi && (code & 3 == 1 || code & 0x1e3 == 0xc2) {
            if t.first() == Some(&b'h') {
                let mut lex = Lex::of(rec);
                if self.has_col(&mut lex, col::ALTREINFO) {
                    if let Some(row) = lex.current() {
                        if super::stricmp(self.col(row, col::ALTREINFO).unwrap_or(&[]), b"HNONMUTA") == 0 {
                            return k;
                        }
                    }
                }
            }
            if k > 2 && ph[k - 1].code == SCHWA && ph[k - 3].word == ph[k - 1].word {
                ph.remove(k - 1);
                k -= 1;
            }
            let pr = &w[prev_word];
            if pr.f[BOUND] == 2 {
                let pt = &pr.text[..];
                let last = pt.last().copied().unwrap_or(0);
                let last2 = if pt.len() < 2 { 0 } else { pt[pt.len() - 2] };
                let pc = ph[k - 1].code;
                let mut add = 1u32;
                match last {
                    b'd' | b't' => {
                        if pc != T_ && pc != 0x1912 {
                            add = T_;
                        }
                    }
                    b'n' => {
                        if pc != N_ {
                            add = N_;
                            if pt != b"rien" && pt != b"bien" {
                                ph[k - 1].code = match pc {
                                    0x100006c1 => 0x6c1,
                                    0x10010361 => 0x102e1,
                                    0x100106c1 => 0x106c1,
                                    c => c,
                                };
                            }
                        }
                    }
                    b'r' => {
                        if pc == 0x10741 {
                            add = R_;
                            ph[k - 1].code = 0x106c1;
                        }
                    }
                    b's' => {
                        if last2 == b'e' || pc & 0x1e3 != 0x142 || pc & 0x1e03 != 0x1802 {
                            add = Z_;
                        }
                    }
                    b'x' | b'z' => {
                        if pc != Z_ && pc != S_ {
                            add = Z_;
                        }
                    }
                    b'y' => {
                        if pt == b"y" {
                            add = J_;
                        }
                    }
                    _ => {}
                }
                let pc = ph[k - 1].code;
                let mut schwa = 1u32;
                if k > 1 && pc & 3 == 2 && (ph[k - 2].code & 3 == 2 || (pc & 0x1e3 == 0x142 && pc & 0x1e03 == 0x1802)) {
                    schwa = SCHWA;
                }
                if add != 1 {
                    let mut at = k;
                    if schwa != 1 {
                        ph.insert(at, Phone { word: prev_word as u32, code: schwa, kind: 0x77 });
                        at += 1;
                    }
                    ph.insert(at, Phone { word: wi as u32, code: add, kind: 0x69 });
                }
            }
            return k;
        }
        if code == SCHWA {
            if k > 1 && ph[k - 2].word == ph[k].word {
                let u = ph[k - 1].code;
                let nx = ph.get(k + 1).map_or(Phone { word: 0, code: 0, kind: 0 }, |p| *p);
                let keep_cluster =
                    (u & 0x1e3 != 0x1e2 && (u & 0x1e3 != 0x142 || u & 0x1e03 != 0x1c02)) || (nx.code != R_ && nx.code != L_);
                let v = ph[k - 2].code;
                let v3 = v & 3;
                let keep = keep_cluster
                    && (v3 != 1 || v & 0x10000000 != 0 || ph[k].word != nx.word)
                    && (ph[k].word == nx.word || v3 != 1 || nx.code & 3 != 2)
                    && u & 3 != 1
                    && nx.code != 0x13
                    && nx.code != 0x17;
                if !keep {
                    ph.remove(k);
                    k = k.wrapping_sub(1);
                }
            }
            return k;
        }
        if code == T_ && k != 0 && t.last() == Some(&b'e') {
            let pc = ph[k - 1].code;
            let nasal = pc & 3 == 1 && pc & 0x10000000 != 0;
            let f = |i: usize, o: usize| w.get(i).map_or(0, |r| r.f[o]);
            if nasal && ((rec.f[SUB] == b'z' && f(wi + 1, SUB) == b'y') || (rec.f[SUB] == b'y' && f(wi + 1, CLASS) == 0x37 && f(wi + 2, SUB) == b'y')) {
                let nx = ph.get(k + 1).copied();
                let nx2 = ph.get(k + 2).copied();
                let starts = |c: u32| matches!(c & 0x1e3, 0x1e2 | 0x142);
                if let Some(nx) = nx {
                    if nx.code == SCHWA && nx2.is_some_and(|n2| ph[k].word != n2.word && starts(n2.code)) {
                        ph[k].code = N_;
                    } else if ph[k].word != nx.word && starts(nx.code) {
                        ph[k].code = N_;
                    }
                }
                return k;
            }
            if nasal && rec.f[SUB] == b'y' && (next == b"secondes" || (next == self.s(0x1003edec) && next2 == self.s(0x1001c450))) {
                ph[k].code = N_;
            }
        }
        k
    }
}
