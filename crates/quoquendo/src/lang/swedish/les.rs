//! The LES functions: LesElaboraParola (0x10007e30), LesAnalizza
//! (0x10009ce0 → 0x10009a10), LesAccenta (0x10001ad0),
//! LesConfiniProsodici (0x10008620), LesGradiAccentuali (0x10007180) and
//! the text hooks (LesNormalizzaAccento 0x10005040, LesPronunciabile
//! 0x10003bd0, LesGetSymbolName 0x10003920).

use super::{LexInfo, Swedish, ACCENTED, UPPER};
use crate::lang::common::{field::*, record, tok, Tok, Word};

const STEM: &[u8] = b"STEM";
const DERIV: &[u8] = b"DERIV_SUFFIX";
const FLEX: &[u8] = b"FLEX_SUFFIX";
const PREFIX: &[u8] = b"PREFIX";
/// The analyser's category index of STEM (0x1004a028[4]), which LesAccenta
/// looks for (0x10001a50, 0x10001a90).
const STEM_INDEX: i32 = 4;
/// Feature numbers (0x1004a03c): CASE, DEFINITENESS, GENDER, NUMBER, POS,
/// VERB_FORM; a value's absence is 0x26 (`&`, one past the 0x26 names).
const F_CASE: u8 = 0;
const F_DEF: u8 = 1;
const F_GENDER: u8 = 3;
const F_NUMBER: u8 = 4;
const F_POS: u8 = 5;
const F_VFORM: u8 = 7;
const NONE: u8 = 0x26;

/// The two-letter words (0x10017530, 8) and three-letter acronyms
/// (0x10017550, 14) LesPronunciabile spells.
const SPELL2: (u32, u32) = (0x10017530, 8);
const SPELL3: (u32, u32) = (0x10017550, 0xe);
/// Tens and units of a written-together number (0x10016214, 8;
/// 0x10016234, 9): `trettio|fem` takes the compound accent rule apart.
const TENS: (u32, u32) = (0x10016214, 8);
const UNITS: (u32, u32) = (0x10016234, 9);

/// LesGetSymbolName's switch (0x10003460): index bytes for `c + 0x80` at
/// 0x10003814, jump table 0x100036f4; each case returns a name, some only
/// when spelling (`mode`), `&` and `°` choose by it.
const NAMES_INDEX: u32 = 0x10003814;
const NAMES_JUMPS: u32 = 0x100036f4;

/// The fixed phrases 0x10007860 recognises (`antingen eller`, `i tid och
/// otid`…), built on its stack from immediates: (frame offset, count) per
/// length, 6 words down to 2 (the 6-word table is never reached: at most 5
/// words are joined).
const PHRASES_FN: (u32, u32) = (0x10007885, 0x10007b67);
const PHRASES: [(u32, u32, u16); 5] = [(0xc, 2, 6), (0x14, 5, 5), (0x50, 0x16, 4), (0xa8, 0x24, 3), (0x28, 10, 2)];
/// 0x10007680's one phrase (`mov edx, imm32` at 0x100076a4), for 3, 2 and
/// 1 words.
const PHRASE3_IMM: u32 = 0x100076a5;

impl Swedish {
    // --- text hooks ----------------------------------------------------------

    /// LesNormalizzaAccento (0x10005040): grave and acute a i o u lose
    /// their accent, ü too, è and ë become e (é stays).
    pub(super) fn normalizza_accento(&self, w: &mut Vec<u8>) {
        for c in w.iter_mut() {
            *c = match *c {
                0xc0 | 0xc1 => b'A',
                0xcc | 0xcd => b'I',
                0xd2 | 0xd3 => b'O',
                0xd9 | 0xda | 0xdc => b'U',
                0xe0 | 0xe1 => b'a',
                0xe8 | 0xeb => b'e',
                0xec | 0xed => b'i',
                0xf2 | 0xf3 => b'o',
                0xf9 | 0xfa | 0xfc => b'u',
                c => c,
            };
        }
    }

    /// LesPronunciabile's test (0x100039d0): an apostrophe or accented
    /// vowel reads; with `c` set nothing else does; the two- and
    /// three-letter lists are spelled; a word all in capitals after a
    /// record that is not (`prev_ok`) reads when it is a syllable
    /// (vowel-consonant-vowel or consonant-vowel-consonant in three
    /// letters, no three consonants at the start of four); else it reads
    /// when it has a vowel.
    pub(super) fn pronounceable(&self, w: &[u8], c: u8, prev_ok: bool) -> bool {
        let mut low = Vec::new();
        let mut caps = true;
        for &ch in w.iter().take(0x3f) {
            if ch == 0 {
                break;
            }
            if ch == b'\'' || self.flags(ch) & ACCENTED != 0 {
                return true;
            }
            if self.flags(ch) & UPPER == 0 {
                caps = false;
            }
            low.push(self.lower(ch));
        }
        if c != 0 {
            return false;
        }
        let n = low.len();
        if n == 2 && self.listed(&low, SPELL2) {
            return false;
        }
        if (n == 2 || n == 3) && self.listed(&low, SPELL3) {
            return false;
        }
        let v = |k: usize| self.is_vowel(at(&low, k));
        let k = |i: usize| self.is_consonant(at(&low, i));
        if caps && prev_ok {
            match n {
                2 => return false,
                3 => {
                    if v(0) && v(2) && !v(1) {
                        return true;
                    }
                    return k(0) && k(2) && v(1);
                }
                4 => {
                    let mut i = 0;
                    while i < 4 && v(i) {
                        i += 1;
                    }
                    if i < 2 && k(i) && k(i + 1) && k(i + 2) {
                        return false;
                    }
                }
                _ => {}
            }
        }
        low.iter().any(|&ch| self.is_vowel(ch))
    }

    /// `FUN_10001830`: binary search (strcmp) of a sorted string table.
    fn listed(&self, key: &[u8], (table, n): (u32, u32)) -> bool {
        let (mut lo, mut hi) = (0i32, n as i32 - 1);
        while lo <= hi {
            let mid = (lo + hi) / 2;
            match key.cmp(self.s(self.pe.u32(table + 4 * mid as u32))) {
                std::cmp::Ordering::Less => hi = mid - 1,
                std::cmp::Ordering::Greater => lo = mid + 1,
                std::cmp::Ordering::Equal => return true,
            }
        }
        false
    }

    /// LesGetSymbolName (0x10003920 → 0x10003460).
    pub(super) fn symbol_name(&self, c: u8, mode: u8) -> Vec<u8> {
        let k = c as i8 as i32 + 0x80;
        if !(0..=0xfe).contains(&k) {
            return Vec::new();
        }
        let idx = self.pe.u8(NAMES_INDEX + k as u32) as u32;
        self.case_string(self.pe.u32(NAMES_JUMPS + 4 * idx), mode != 0)
    }

    /// One case of the switch, run on its few instructions: `test al,al`,
    /// `je`/`jne` (short or near), `mov eax, imm32`, `ret`.
    fn case_string(&self, mut pc: u32, al: bool) -> Vec<u8> {
        let mut zf = false;
        let mut eax = 0u32;
        for _ in 0..16 {
            let b = self.pe.at(pc);
            match b {
                [0x84, 0xc0, ..] => {
                    zf = !al;
                    pc += 2;
                }
                [0x74, r, ..] | [0x75, r, ..] => {
                    let take = (b[0] == 0x74) == zf;
                    pc = pc + 2;
                    if take {
                        pc = pc.wrapping_add(*r as i8 as i32 as u32);
                    }
                }
                [0x0f, 0x84, ..] | [0x0f, 0x85, ..] => {
                    let r = u32::from_le_bytes([b[2], b[3], b[4], b[5]]);
                    let take = (b[1] == 0x84) == zf;
                    pc += 6;
                    if take {
                        pc = pc.wrapping_add(r);
                    }
                }
                [0xb8, ..] => {
                    eax = u32::from_le_bytes([b[1], b[2], b[3], b[4]]);
                    pc += 5;
                }
                [0xc3, ..] => return if eax == 0 { Vec::new() } else { self.s(eax).to_vec() },
                _ => return Vec::new(),
            }
        }
        Vec::new()
    }

    // --- LesElaboraParola ----------------------------------------------------

    /// LesElaboraParola (0x10007e30): a number token followed by `-`/`:`
    /// and a word takes the word in (`5-åring`); a pronounceable word
    /// becomes one record (0x17, 0x17, 9) whose LexInfo holds the
    /// analyser's analyses of its lower case; else it is spelled, each
    /// letter's name a record (0x15, 0x83, the last 0x84).
    pub(super) fn elabora_parola(&self, toks: &mut [Tok], i: usize, idx: usize, spell: u8, mode: u8) -> Vec<Word> {
        if toks[i].kind == tok::NUMBER
            && toks.get(i + 1).is_some_and(|t| t.text == b"-" || t.text == b":")
            && toks.get(i + 2).is_some_and(|t| t.kind == tok::WORD)
        {
            let w = toks[i + 2].text.clone();
            toks[i].text.extend_from_slice(&w);
            toks[i + 1].text.clear();
            toks[i + 2].text.clear();
        }
        let t = toks[i].text.clone();
        // the record before (the sentence's so far): no capital after its
        // first letter (0x10003bd0); approximated by the token before
        let prev_ok = idx > 0 && {
            let mut k = i;
            let mut ok = true;
            while k > 0 {
                k -= 1;
                if toks[k].kind == tok::TAG || toks[k].text.is_empty() {
                    continue;
                }
                let p = &toks[k].text;
                ok = !p.iter().skip(1).any(|&c| self.is_upper(c));
                break;
            }
            ok
        };
        let pron = self.pronounceable(&t, 0, prev_ok);
        if mode == 0 && (pron || spell != 0) {
            let low: Vec<u8> = t.iter().map(|&c| self.lower(c)).collect();
            let analyses = self.analyse_word(&low);
            let mut r = record(t.iter().take(0x3f).copied().collect(), 0x17, 0x17, 9);
            r.f[0xc] = 0xff;
            r.f[0xd] = 0xff;
            LexInfo { slots: vec![[0; 6]; super::SLOTS], analyses }.store(&mut r);
            return vec![r];
        }
        let mut out = Vec::new();
        for k in 0..t.len() {
            let mut name = self.symbol_name(t[k], 1);
            if name.is_empty() {
                continue;
            }
            self.normalizza_accento(&mut name);
            let sub = if k + 1 == t.len() { 0x84 } else { 0x83 };
            let mut r = record(name, 0x15, sub, 9);
            r.f[0xc] = 0xff;
            r.f[0xd] = 0xff;
            out.push(r);
        }
        out
    }

    // --- LesAnalizza -----------------------------------------------------------

    /// LesAnalizza (0x10009a10): a word's class, subclass and information
    /// from the features of its best analysis's morphemes, the last first
    /// (stems, derivational and inflectional suffixes) until one with a part
    /// of speech sets the class (0x100098a0); a cardinal makes the number
    /// words before it `z`/`y`; then `ett` before a common-gender noun
    /// becomes `en`.
    pub(super) fn analizza(&self, w: &mut [Word]) {
        for i in 0..w.len() {
            if w[i].f[CLASS] == b'2' {
                continue;
            }
            let li = LexInfo::of(&w[i]);
            if let Some(sol) = li.best() {
                for e in sol.iter().rev() {
                    if w[i].f[CLASS] != 0x17 {
                        break;
                    }
                    let cat = self.category(self.m_cat(e.m)).unwrap_or(b"");
                    if !(cat.eq_ignore_ascii_case(DERIV) || cat.eq_ignore_ascii_case(STEM) || cat.eq_ignore_ascii_case(FLEX)) {
                        continue;
                    }
                    let fl = self.m_feat(e.m);
                    if !self.has_features(fl) {
                        continue;
                    }
                    let f = |k: u8| self.feature(fl, k).unwrap_or(NONE);
                    if let Some(pos) = self.feature(fl, F_POS) {
                        w[i].f[CLASS] = pos_class(pos);
                        if pos == 0x12 {
                            if i > 0 && w[i - 1].f[SUB] == b'y' {
                                w[i - 1].f[SUB] = b'z';
                            }
                            w[i].f[SUB] = b'y';
                        }
                    }
                    let (case, def, vform) = (f(F_CASE), f(F_DEF), f(F_VFORM));
                    if w[i].f[SUB] == 0x17 && !(case == NONE && def == NONE && vform == NONE) {
                        w[i].f[SUB] = sub_of(vform, case, def);
                    }
                    let (gender, number) = (f(F_GENDER), f(F_NUMBER));
                    if w[i].f[INFO] == 9 && !(gender == NONE && number == NONE) {
                        w[i].f[INFO] = info_of(gender, number);
                    }
                }
            }
            let mut j = i;
            while j > 0 {
                let p = j - 1;
                if !matches!(w[p].f[SUB], b'y' | b'z') {
                    break;
                }
                if w[p].text.eq_ignore_ascii_case(b"ett") && matches!(w[i].f[INFO], 0xcc | 0xcb | 0xcd) {
                    w[p].text = b"en".to_vec();
                    let mut l = LexInfo::of(&w[p]);
                    l.analyses.clear();
                    l.store(&mut w[p]);
                }
                j -= 1;
            }
        }
    }

    // --- LesAccenta ------------------------------------------------------------

    /// LesAccenta (0x10001ad0): per word without a grade yet, the accented
    /// vowels into its LexInfo slots, one per morpheme of its best analysis:
    /// a written accent wins; else the morpheme's stress group
    /// (0x10001100); a compound's first stressed part gets accent 2, its
    /// last the secondary (0x10001890). Spelled letters but the last pair
    /// are unstressed.
    pub(super) fn accenta(&self, w: &mut [Word]) {
        let mut last: i32 = -1;
        for i in 0..w.len() {
            if w[i].f[CLASS] == b'2' {
                continue;
            }
            if w[i].f[SUB] == 0x83 && (w.get(i + 1).map_or(0, |r| r.f[SUB]) != 0x84 || (i > 0 && w[i - 1].f[SUB] == 0x83)) {
                w[i].f[GRADE] = 3;
            }
            let mut li = LexInfo::of(&w[i]);
            if w[i].f[GRADE] != 0x0e {
                if last >= 0 {
                    li.slots[0][1] = 1;
                    li.slots[0][0] = last;
                    li.store(&mut w[i]);
                }
                continue;
            }
            let Some(sol) = li.best().map(|s| s.to_vec()) else {
                last = self.vowels_before_accent(&w[i].text);
                if last >= 0 {
                    set(&mut li.slots[0], 0, last + 1, 1);
                } else if self.nth_vowel(&w[i].text, 2) < 0 {
                    set(&mut li.slots[0], 0, 1, 1);
                } else {
                    set(&mut li.slots[0], 0, 1, 2);
                }
                li.store(&mut w[i]);
                continue;
            };
            for k in 0..sol.len() {
                let e = sol[k];
                let text = w[i].text.clone();
                let mut stress = slice(&text, e.start, e.end);
                let cat = self.category(self.m_cat(e.m)).unwrap_or(b"").to_vec();
                let mut grp = self.group(self.m_grp(e.m)).map(|g| g.to_vec());
                if grp.as_deref().is_some_and(|g| g.eq_ignore_ascii_case(b"GRP_s_no_stress")) {
                    let r = &w[i];
                    let next_punct = w.get(i + 1).map_or(0, |n| n.f[CLASS]) == b'2';
                    let keep = (next_punct && (i == 0 || w[i - 1].f[CLASS] == b'2'))
                        || sol.len() > 2
                        || (sol.len() == 2 && sol.get(k + 1).is_some_and(|n| self.m_has_str(n.m)))
                        || (r.text.eq_ignore_ascii_case(b"ett") && r.f[CLASS] == 0x15 && matches!(r.f[SUB], b'z' | b'y'));
                    if keep {
                        grp = None;
                    } else {
                        w[i].f[GRADE] = 3;
                    }
                }
                last = self.vowels_before_accent(&stress);
                if last >= 0 {
                    set(&mut li.slots[k], 0, last + 1, 1);
                    li.slots[k][2] = 1;
                    continue;
                }
                if let Some(alt) = self.m_alt(e.m) {
                    if !alt.is_empty() {
                        stress = alt.to_vec();
                    }
                }
                let g: i32 = if cat.eq_ignore_ascii_case(DERIV) || cat.eq_ignore_ascii_case(FLEX) {
                    (0..=k).rev().find(|&j| self.m_cat(sol[j].m) == STEM_INDEX).map_or(-1, |j| j as i32)
                } else if cat.eq_ignore_ascii_case(PREFIX) {
                    (k + 1..sol.len()).find(|&j| self.m_cat(sol[j].m) == STEM_INDEX).map_or(-1, |j| j as i32)
                } else {
                    0
                };
                let ge = sol[g.max(0) as usize];
                let grp2 = self.group(self.m_grp(ge.m)).unwrap_or(b"").to_vec();
                let gtext = slice(&text, ge.start, ge.end);
                let nv = self.count_vowels(&stress);
                let ng = self.count_vowels(&gtext);
                let ctx = Stress {
                    k,
                    g: g as usize,
                    stress: &stress,
                    cat: &cat,
                    grp: grp.as_deref().unwrap_or(b""),
                    grp2: &grp2,
                    nv,
                    ng,
                };
                if g >= 0 {
                    self.stress_rule(&mut li.slots, &ctx);
                }
            }
            if w[i].f[GRADE] != 3 {
                self.compound_stress(&sol, &mut li.slots);
            }
            li.store(&mut w[i]);
        }
    }

    /// 0x10001000: the vowels before the first accented one, -1 when there
    /// is none.
    fn vowels_before_accent(&self, s: &[u8]) -> i32 {
        let mut n = 0;
        for &c in s {
            if self.flags(c) & ACCENTED != 0 {
                return n;
            }
            if self.is_vowel(c) {
                n += 1;
            }
        }
        -1
    }

    /// 0x10001080: the index of the `n`-th vowel (n > 0), -1 when fewer.
    fn nth_vowel(&self, s: &[u8], n: i32) -> i32 {
        let mut c = 0;
        for (i, &ch) in s.iter().enumerate() {
            if self.is_vowel(ch) {
                c += 1;
                if c >= n {
                    return i as i32;
                }
            }
        }
        -1
    }

    /// 0x1000a6f0: vowels in `s`.
    pub(super) fn count_vowels(&self, s: &[u8]) -> i32 {
        s.iter().filter(|&&c| self.is_vowel(c)).count() as i32
    }

    /// 0x10001100: the stress group's rule, for the morpheme's category:
    /// a stem's own position (`GRP_s_*`: last, penultimate, first…, accent
    /// 1 or 2); a suffix's or prefix's on the stem it belongs to (slot `g`).
    fn stress_rule(&self, slots: &mut [[i32; 6]], c: &Stress) {
        let is = |a: &[u8], b: &[u8]| a.eq_ignore_ascii_case(b);
        let (k, g, nv) = (c.k, c.g, c.nv);
        let grp = c.grp;
        if is(c.cat, STEM) {
            let s = &mut slots[k];
            match grp {
                _ if is(grp, b"GRP_s_last_1") => set(s, 0, nv, 1),
                _ if is(grp, b"GRP_s_penlast_1") || is(grp, b"s_penlast_1_shiftable") => set(s, 0, nv - 1, 1),
                _ if is(grp, b"GRP_s_second_1") => set(s, 0, 2, 1),
                _ if is(grp, b"GRP_s_second_2") => set(s, 0, 2, 2),
                _ if is(grp, b"GRP_s_third_1") => set(s, 0, 3, 1),
                _ if is(grp, b"GRP_s_fourth_1") => set(s, 0, 4, 1),
                _ if is(grp, b"GRP_s_first_1") => set(s, 0, 1, 1),
                _ if is(grp, b"GRP_s_first_2") => set(s, 0, 1, 2),
                _ if is(grp, b"GRP_s_first_2_second_1") => {
                    set(s, 0, 1, 2);
                    set(s, 3, 2, 1);
                }
                _ if is(grp, b"GRP_s_first_2_third_1") => {
                    set(s, 0, 1, 2);
                    set(s, 3, 3, 1);
                }
                _ if is(grp, b"GRP_s_first_2_fourth_1") => {
                    set(s, 0, 1, 2);
                    set(s, 3, 4, 1);
                }
                _ if is(grp, b"GRP_s_second_2_fourth_1") => {
                    set(s, 0, 2, 2);
                    set(s, 3, 4, 1);
                }
                _ if is(grp, b"GRP_s_terzlast_2") => set(s, 0, nv - 2, 2),
                _ if is(grp, b"GRP_s_terzlast_1") => set(s, 0, nv - 2, 1),
                _ if is(grp, b"GRP_s_last_2") => set(s, 0, nv, 2),
                _ if is(grp, b"GRP_s_penlast_2") => set(s, 0, nv - 1, 2),
                _ if is(grp, b"GRP_s_no_stress") => {
                    s[1] = 0;
                    s[4] = 0;
                }
                _ => {
                    let kind = if self.nth_vowel(c.stress, 2) < 0 { 1 } else { 2 };
                    set(s, 0, 1, kind);
                }
            }
        } else if is(c.cat, DERIV) {
            if is(grp, b"GRP_sd_last_1") {
                set(&mut slots[k], 0, nv, 1);
                slots[g][1] = 2;
            } else if is(grp, b"GRP_sd_penlast_1") {
                set(&mut slots[k], 0, nv - 1, 1);
                slots[g][1] = 2;
            } else if is(grp, b"GRP_sd_last_1d") {
                set(&mut slots[k], 0, nv, 1);
                slots[g][0] = 0;
                slots[g][1] = 0;
            } else if is(grp, b"GRP_sd_penlast_1d") {
                set(&mut slots[k], 0, nv - 1, 1);
                if slots[g][4] != 0 {
                    slots[g][4] = 0;
                } else {
                    slots[g][1] = 0;
                }
            } else if is(grp, b"GRP_sd_penlast_2d") || is(grp, b"GRP_sd_last_2d") {
                set(&mut slots[k], 0, nv, 2);
                slots[g][1] = 0;
            } else if k == g + 1 && is(grp, b"GRP_sd_d_last_1") {
                set(&mut slots[g], 0, c.ng, 1);
                slots[g][4] = 0;
            } else if is(grp, b"GRP_sd_d_2") {
                slots[g][1] = 2;
            }
        } else if is(c.cat, FLEX) {
            if is(grp, b"GRP_f_first_1") {
                set(&mut slots[g], 0, 1, 1);
            } else if k == g + 1 && is(grp, b"GRP_f_firstacc_1") {
                slots[g][1] = 1;
            } else if is(grp, b"GRP_f_last_1") {
                // the stem's own vowel count (read after `pop edi`: argument 6)
                slots[g][4] = 0;
                set(&mut slots[g], 0, c.ng, 1);
            } else if is(grp, b"GRP_f_first_2") {
                set(&mut slots[g], 0, 1, 2);
            } else if is(grp, b"GRP_f_last_1_or") {
                if is(c.grp2, b"GRP_s_penlast_1_shiftable") {
                    set(&mut slots[g], 0, c.ng, 1);
                }
            } else if is(grp, b"GRP_f_first_2_mono") && is(c.grp2, b"GRP_s_mono") {
                slots[g][1] = 2;
            }
        } else if is(c.cat, PREFIX) {
            if is(grp, b"GRP_p_first_2") {
                set(&mut slots[k], 0, 1, 2);
                if slots[g][4] == 0 {
                    slots[g][1] = 1;
                } else {
                    slots[g][1] = 0;
                    slots[g][4] = 1;
                }
            } else if is(grp, b"GRP_p_d_1") {
                slots[k][1] = 0;
                slots[g][1] = 1;
            }
        }
    }

    /// 0x10001890: with two or more stressed morphemes (a compound), the
    /// parts between the first and the last lose theirs (but the unit of a
    /// written number `trettiofem`), the first takes accent 2, the last's
    /// becomes the secondary one; one morpheme with both accents keeps
    /// them as 2 then 1.
    fn compound_stress(&self, sol: &[super::Entry], slots: &mut [[i32; 6]]) {
        let n = sol.len();
        let (mut first, mut last, mut cnt) = (-1i32, -1i32, 0);
        for i in 0..n {
            if slots[i][1] > 0 || slots[i][4] > 0 {
                cnt += 1;
                if first < 0 {
                    first = i as i32;
                } else {
                    last = i as i32;
                }
            }
        }
        if cnt < 2 {
            if first >= 0 {
                let s = &mut slots[first as usize];
                if s[1] > 0 && s[4] > 0 {
                    s[1] = 2;
                    s[4] = 1;
                }
            }
            return;
        }
        let mut comp = -1i32;
        for i in 0..n {
            if comp >= 0 {
                break;
            }
            let a = self.m_has_str(sol[i].m) && self.listed(self.m_str(sol[i].m), TENS);
            let b = a && sol.get(i + 1).is_some_and(|e| self.m_has_str(e.m) && self.listed(self.m_str(e.m), UNITS));
            if b {
                comp = i as i32;
            }
        }
        if first == comp {
            let s = &mut slots[first as usize];
            s[1] = 0;
            s[4] = 0;
            first += 1;
        }
        for j in first + 1..last {
            if comp < 0 || (comp > 0 && j != comp + 1) {
                slots[j as usize][1] = 0;
                slots[j as usize][4] = 0;
            }
        }
        let s = &mut slots[first as usize];
        if s[1] == 0 {
            s[0] = s[3];
            s[1] = s[4];
            s[2] = s[5];
        }
        s[1] = 2;
        s[4] = 0;
        let t = &mut slots[last as usize];
        if t[4] == 0 {
            t[3] = t[0];
            t[4] = t[1];
            t[5] = t[2];
        }
        t[4] = 1;
        t[1] = 0;
    }

    // --- LesGradiAccentuali ----------------------------------------------------

    /// LesGradiAccentuali (0x10007180): punctuation and `-` 0x0f, words
    /// without a grade 0.
    pub(super) fn gradi(&self, w: &mut [Word]) {
        for r in w.iter_mut().take(200) {
            if r.f[BOUND] == 0x17 {
                break;
            }
            if r.f[CLASS] == b'2' || r.f[CLASS] == b'7' {
                r.f[GRADE] = 0x0f;
            } else if r.f[GRADE] == 0x0e {
                r.f[GRADE] = 0;
            }
        }
    }

    // --- LesConfiniProsodici ---------------------------------------------------

    /// LesConfiniProsodici (0x10008620): the boundary after each word from
    /// the group kinds 0x10008190 gives: 5 inside a group, 3 at its end, 9,
    /// 1 before a following group, 0x0b at commas, the sentence end by its
    /// type; a pause (0x0a) before a new group when both halves are long
    /// (0x10007390) and the words may part (0x100074c0); then number words
    /// (`y`/`z`/`{`) end their groups 7/6/8.
    pub(super) fn confini(&self, w: &mut [Word], mode: i32) {
        let kinds = self.group_kinds(w);
        if kinds[0].0 == 10 {
            return;
        }
        let class = |w: &[Word], i: usize| w.get(i).map_or(0, |r| r.f[CLASS]);
        let mut last = 200usize;
        let mut i = 0usize;
        while i < w.len() && kinds[i].0 != 10 && i * 8 < 0x640 {
            let mut sub_rule = false;
            if w[i].f[BOUND] != 0x15 {
                sub_rule = matches!(w[i].f[BOUND], 3 | 5);
            } else {
                match kinds[i].0 {
                    0 => {
                        if w[i].f[CLASS] == b'7' {
                            w[i].f[BOUND] = 0x16;
                        } else {
                            w[i].f[BOUND] = 5;
                            last = i;
                            sub_rule = true;
                        }
                    }
                    1 => {
                        if w[i].f[CLASS] == b'7' {
                            w[i].f[BOUND] = 0x16;
                        } else {
                            if last != 200 && kinds[last].1 == 0 && mode != 3 && mode != 2 && kinds[last].0 != 5 && !self.too_short(w, last, i) && may_part(w, last) {
                                w[last].f[BOUND] = 0x0a;
                            }
                            w[i].f[BOUND] = 5;
                            last = i;
                            sub_rule = true;
                        }
                    }
                    2 => {
                        if kinds[i].1 == 1 && !self.too_short(w, i, i) && may_part(w, i) {
                            w[i].f[BOUND] = 0x0a;
                            last = i;
                        } else if w[i].f[CLASS] == b'7' {
                            w[i].f[BOUND] = 0x16;
                        } else {
                            w[i].f[BOUND] = 5;
                            last = i;
                            sub_rule = true;
                        }
                    }
                    4 => {
                        w[i].f[BOUND] = 9;
                        last = i;
                    }
                    6 => {
                        if mode != 3 && mode != 2 && last != 200 && !self.too_short(w, last, i) && may_part(w, i) && kinds.get(i + 1).map_or(10, |k| k.0) != 10 && class(w, last) != class(w, i + 1) {
                            w[last].f[BOUND] = 0x0a;
                        }
                        w[i].f[BOUND] = 1;
                    }
                    7 => {
                        let c = w[i].f[CLASS];
                        if c == b'7' {
                            w[i].f[BOUND] = 0x16;
                        } else if c == 0xd9 || c == 9 {
                            w[i].f[BOUND] = 1;
                        } else {
                            w[i].f[BOUND] = 3;
                            sub_rule = true;
                        }
                    }
                    8 => {
                        let mut j = 0usize;
                        if i > 0 {
                            j = i - 1;
                            while j > 0 && w[j].f[BOUND] == 0x16 {
                                j -= 1;
                            }
                        }
                        if w[j].f[CLASS] == b'2' {
                            j = i;
                        }
                        let b = if w[i].f[SUB] == b'|' {
                            match mode {
                                0 => Some(0x0f),
                                2 => Some(0x11),
                                3 => Some(0x12),
                                5 => Some(0x10),
                                6 => Some(0x13),
                                _ => None,
                            }
                        } else {
                            let s = &w[i].text;
                            let mut k = 0usize;
                            if at(s, 1) != 0 {
                                while !b",)(-.".contains(&at(s, k)) && at(s, k + 1) != 0 {
                                    k += 1;
                                }
                            }
                            let mut c = at(s, k);
                            if c == b'.' {
                                c = b',';
                            }
                            match c {
                                b',' | b'-' => Some(0x0b),
                                b'(' | b')' if !(i == 0 || class(w, i.wrapping_sub(1)) == b'2' || class(w, i + 1) == b'2') => Some(0x0b),
                                _ => None,
                            }
                        };
                        match b {
                            Some(b) => {
                                w[j].f[BOUND] = b;
                                w[i].f[BOUND] = b;
                                last = i;
                            }
                            None => w[i].f[BOUND] = 0x16,
                        }
                    }
                    9 => w[i].f[BOUND] = 0x16,
                    _ => {
                        w[i].f[BOUND] = 3;
                        last = i;
                        sub_rule = true;
                    }
                }
            }
            if sub_rule && matches!(w[i].f[BOUND], 3 | 5) {
                match w[i].f[SUB] {
                    b'y' | 0x84 => w[i].f[BOUND] = 7,
                    b'z' | 0x83 => w[i].f[BOUND] = 6,
                    b'{' => w[i].f[BOUND] = 8,
                    _ => {}
                }
            }
            i += 1;
        }
    }

    /// 0x10008190: a kind per record (5 a word inside a group, 7 the
    /// group's words before its end, 1/0/2 the end of a verb, noun or
    /// subject group, 6 before a conjunction, 8 punctuation; flag 1 when a
    /// subject group follows a sentence-initial adverb, preposition or
    /// fixed phrase), from a state machine over the classes; fixed phrases
    /// (0x10007860) are not split. Ended by kind 10.
    fn group_kinds(&self, w: &[Word]) -> Vec<(u32, u32)> {
        const A: [u8; 3] = [0xd9, 0x09, 0xca];
        const V: [u8; 8] = [0xda, 0x2a, 0x2b, 0x2c, 0xd4, 0xd1, 0xd2, 0xd3];
        const N: [u8; 9] = [0x37, 0xcb, 0xcc, 0xcd, 0x15, 0x29, 0xce, 0xcf, 0xd0];
        let class = |i: usize| w.get(i).map_or(0, |r| r.f[CLASS]);
        let mut k: Vec<(u32, u32)> = vec![(0, 0); w.len() + 1];
        let fill = |k: &mut Vec<(u32, u32)>, a: usize, b: usize| {
            for x in a..b {
                k[x] = (7, 0);
            }
        };
        let (mut state, mut loclen, mut locpos, mut prev) = (0u32, 0u16, 0u16, 0usize);
        let mut i = 0usize;
        while i < w.len() {
            k[i] = (5, 0);
            if loclen == 0 {
                if let Some(n) = self.phrase(w, i) {
                    loclen = n;
                    locpos = 1;
                }
            } else if locpos < loclen {
                locpos += 1;
            } else if locpos == loclen {
                loclen = 0;
            }
            let c = class(i);
            let mut to_start = false;
            match state {
                1 => {
                    if c == 0x37 || A.contains(&c) {
                        state = 1;
                    } else if V.contains(&c) {
                        state = 2;
                    } else {
                        state = 0;
                        to_start = true;
                    }
                }
                2 => {
                    if V.contains(&class(i + 1)) {
                        state = 2;
                    } else if loclen == 0 {
                        fill(&mut k, prev, i.wrapping_sub(1));
                        k[i - 1].0 = 1;
                        state = 0;
                        to_start = true;
                    }
                }
                3 => {
                    if c == 0x37 || c == 0x1f {
                        state = 3;
                    } else if N[1..].contains(&c) {
                        state = 5;
                    } else {
                        state = 0;
                        to_start = true;
                    }
                }
                4 => {
                    if c == 0x1f {
                        state = 4;
                    } else if N.contains(&c) {
                        state = 7;
                    } else {
                        state = 0;
                        to_start = true;
                    }
                }
                5 => {
                    let joined = i > 0 && w[i].f[SUB] == 0x82 && class(i - 1) == class(i + 1) && w[i - 1].f[SUB] == w.get(i + 1).map_or(0, |r| r.f[SUB]);
                    if N.contains(&c) || joined {
                        state = 5;
                    } else if loclen == 0 {
                        fill(&mut k, prev, i.wrapping_sub(1));
                        k[i - 1].0 = 0;
                        state = 0;
                        to_start = true;
                    }
                }
                6 => {
                    if A.contains(&c) {
                        state = 1;
                    } else if V.contains(&c) && i < 2 {
                        state = 2;
                    } else if N.contains(&c) {
                        state = 7;
                    } else {
                        state = 0;
                        to_start = true;
                    }
                }
                7 => {
                    if N.contains(&c) {
                        state = 7;
                    } else if loclen == 0 {
                        let p = prev;
                        fill(&mut k, p, i.wrapping_sub(1));
                        k[i - 1].0 = 2;
                        if p == 0 {
                            let c0 = class(0);
                            let flag = c0 == 0x33
                                || c0 == 0x19
                                || self.phrase(w, 0).is_some_and(|n| (n as usize) < i)
                                || self.phrase_pippo(w, 0).is_some_and(|n| (n as usize) < i);
                            if flag {
                                k[i - 1].1 = 1;
                            }
                        }
                        state = 0;
                        to_start = true;
                    }
                }
                8 => {
                    k[i - 1].0 = 6;
                    state = 0;
                    to_start = true;
                }
                _ => to_start = state == 0,
            }
            if to_start {
                prev = i;
                if c == 0x1f {
                    state = 8;
                } else if A.contains(&c) {
                    state = 1;
                } else if V.contains(&c) {
                    state = 2;
                } else if N.contains(&c) {
                    state = 5;
                } else if c == b'2' {
                    k[i].0 = 8;
                }
            }
            i += 1;
        }
        k[i].0 = 10;
        k
    }

    /// 0x10007390: whether a pause between `b` and the group ending at
    /// `cur` would leave a side shorter than 5 words or 22 letters (left:
    /// back from `b` to the last pause or comma; right: after `b` to the
    /// punctuation; `-` records not counted).
    fn too_short(&self, w: &[Word], b: usize, cur: usize) -> bool {
        let (mut lw, mut ll) = (1u32, w[b].text.len() as u32);
        let mut e = b;
        while e > 0 {
            let bound = w[e - 1].f[BOUND];
            e -= 1;
            if bound == 0x16 {
                continue;
            }
            if bound == 0x0a || bound == 0x0b {
                break;
            }
            ll += w[e].text.len() as u32;
            lw += 1;
        }
        let (mut rw, mut rl) = (0u32, 0u32);
        let mut d = cur + 1;
        while d < w.len() {
            let c = w[d].f[CLASS];
            if c == b'2' {
                break;
            }
            if c != b'7' {
                rw += 1;
                rl += w[d].text.len() as u32;
            }
            d += 1;
        }
        let mut k = cur;
        while k > b {
            if w[k].f[CLASS] != b'7' {
                rl += w[k].text.len() as u32;
                rw += 1;
            }
            k -= 1;
        }
        !(lw >= 5 && ll >= 0x16 && rw >= 5 && rl >= 0x16)
    }

    /// The table of fixed phrases the immediates at `PHRASES_FN` put at
    /// frame offset `off`.
    fn phrase_table(&self, off: u32, n: u32) -> Vec<&[u8]> {
        let mut imm = std::collections::HashMap::new();
        let mut pc = PHRASES_FN.0;
        while pc < PHRASES_FN.1 {
            let b = self.pe.at(pc);
            match b {
                [0xc7, 0x44, 0x24, d, ..] => {
                    imm.insert(*d as u32, u32::from_le_bytes([b[4], b[5], b[6], b[7]]));
                    pc += 8;
                }
                [0xc7, 0x84, 0x24, ..] => {
                    let d = u32::from_le_bytes([b[3], b[4], b[5], b[6]]);
                    imm.insert(d, u32::from_le_bytes([b[7], b[8], b[9], b[10]]));
                    pc += 11;
                }
                _ => break,
            }
        }
        (0..n).map(|k| imm.get(&(off + 4 * k)).map_or(&b""[..], |&a| self.s(a))).collect()
    }

    /// 0x10007860: the fixed phrase (2–5 words, the longest) starting at
    /// record `i`, its length.
    fn phrase(&self, w: &[Word], i: usize) -> Option<u16> {
        let words: Vec<&[u8]> = w[i..].iter().take(5).map(|r| r.text.as_slice()).collect();
        for &(off, n, len) in &PHRASES {
            if words.len() != len as usize && !(words.len() > len as usize && len < 5) {
                // joined words are cut from the longest down
            }
            if words.len() < len as usize || len == 6 {
                continue;
            }
            let key = words[..len as usize].join(&b' ');
            if search_string(&key, &self.phrase_table(off, n)) {
                return Some(len);
            }
        }
        None
    }

    /// 0x10007680: the same for its one phrase (`pippo`) of 3, 2 or 1 words.
    fn phrase_pippo(&self, w: &[Word], i: usize) -> Option<u16> {
        let p = self.s(self.pe.u32(PHRASE3_IMM));
        let words: Vec<&[u8]> = w[i..].iter().take(3).map(|r| r.text.as_slice()).collect();
        for len in (1..=words.len()).rev() {
            if words[..len].join(&b' ') == p {
                return Some(len as u16);
            }
        }
        None
    }
}

/// The arguments of 0x10001100.
struct Stress<'a> {
    k: usize,
    g: usize,
    stress: &'a [u8],
    cat: &'a [u8],
    grp: &'a [u8],
    grp2: &'a [u8],
    nv: i32,
    ng: i32,
}

/// 0x10001050 on a slot's first (`at` 0) or second (3) accent.
fn set(s: &mut [i32; 6], at: usize, pos: i32, kind: i32) {
    s[at] = pos;
    s[at + 1] = kind;
}

fn at(s: &[u8], i: usize) -> u8 {
    s.get(i).copied().unwrap_or(0)
}

fn slice(t: &[u8], a: i32, b: i32) -> Vec<u8> {
    let a = (a.max(0) as usize).min(t.len());
    let b = (b.max(0) as usize).min(t.len()).max(a);
    t[a..b].to_vec()
}

/// 0x100098a0: a part of speech (feature 5's value) as the record class.
fn pos_class(p: u8) -> u8 {
    match p {
        0x0d | 0x11 | 0x12 => 0x15,
        0x01 => 0x29,
        0x02 => 0x33,
        0x04 => 0x18,
        0x05 => 0x1f,
        0x17 => 0x19,
        0x18 => 0x20,
        0x21 => 0x2a,
        0x23 => 0x2b,
        0x25 => 0x2c,
        0x22 => 0,
        0x24 => 0xda,
        _ => 0xc8,
    }
}

/// 0x10009910: verb form, case and definiteness as the subclass.
fn sub_of(vform: u8, case: u8, def: u8) -> u8 {
    match vform {
        NONE => match case {
            NONE => match def {
                6 => 0xd8,
                9 => 0xd9,
                _ => 0xc8,
            },
            0x0f => 0xd5,
            0x07 => 0xd6,
            0 => 0xd7,
            _ => 0xc8,
        },
        0x0a => 0x8d,
        0x19 => 0xd1,
        0x1a => 0xd2,
        0x1e => 0xd3,
        0x14 => 0x88,
        0x15 => 0x8e,
        0x08 => 0x87,
        _ => 0xc8,
    }
}

/// 0x10009990: gender and number as the information byte.
fn info_of(gender: u8, number: u8) -> u8 {
    match gender {
        NONE => match number {
            0x16 => 0x10,
            0x1c => 0x0f,
            _ => 9,
        },
        0x20 => match number {
            NONE => 0xcd,
            0x16 => 0xcb,
            0x1c => 0xcc,
            _ => 0xcd,
        },
        0x0e => match number {
            NONE => 0xd0,
            0x16 => 0xce,
            0x1c => 0xcf,
            _ => 0xd0,
        },
        0x0c => match number {
            NONE => 0xca,
            0x16 => 0x0c,
            0x1c => 0x0b,
            _ => 0xca,
        },
        _ => 9,
    }
}

/// 0x100074c0: whether the words `i` and `i + 1` may part (not article or
/// adjective + noun, preposition + noun or pronoun, verb + its
/// complement…).
fn may_part(w: &[Word], i: usize) -> bool {
    const V: [u8; 8] = [0xda, 0x2a, 0x2b, 0x2c, 0xd4, 0xd1, 0xd2, 0xd3];
    let c = w[i].f[CLASS];
    let n = w.get(i + 1).map_or(0, |r| r.f[CLASS]);
    if (c == 0x18 && n == 0x15) || (c == 0x29 && n == 0x15) || (c == 0x15 && (n == 0x29 || n == 0x1f)) || (c == 0x20 && n == 0x1f) || (c == 0x19 && (n == 0x15 || n == 0x20)) {
        return false;
    }
    if i + 1 >= w.len() {
        return true;
    }
    if V.contains(&c) && (matches!(n, 0x33 | 0x29 | 0x18 | 0x15 | 0x19 | 0x1e) || V.contains(&n)) {
        return false;
    }
    if c == 0x33 && V.contains(&n) {
        return false;
    }
    if c == 0x20 && (n == 0x20 || n == 0x33) {
        return false;
    }
    if c == 0x33 && (n == 0x20 || n == 0x1f) {
        return false;
    }
    if c == 0x1f && (n == 0x1f || n == 0x33) {
        return false;
    }
    if (c == 0x19 || c == 0x1e) && (n == 0x20 || n == 0x1f) {
        return false;
    }
    if matches!(c, 0x2b | 0x2c | 0xda) && n == 0x2a {
        return false;
    }
    true
}

/// LTTS7Util.dll `ELQSearchString` (0x10028ca0) with flag 1: binary search
/// (strcmp) over more than 3 entries, else linear.
fn search_string(key: &[u8], table: &[&[u8]]) -> bool {
    if table.len() > 3 {
        let (mut lo, mut hi) = (0i64, table.len() as i64 - 1);
        while lo <= hi {
            let mid = (lo + hi) / 2;
            match key.cmp(table[mid as usize]) {
                std::cmp::Ordering::Less => hi = mid - 1,
                std::cmp::Ordering::Greater => lo = mid + 1,
                std::cmp::Ordering::Equal => return true,
            }
        }
        false
    } else {
        table.iter().any(|t| key == *t)
    }
}
