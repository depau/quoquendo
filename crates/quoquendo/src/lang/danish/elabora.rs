//! `LesElaboraParola` (0x10004d60), `LesNormalizzaAccento` (0x100019d0),
//! `LesPronunciabile` (0x10004640) and `LesGetSymbolName` (0x100034c0) of
//! LTTS7Danish.dll.

use super::{Danish, LexInfo, Morpheme, CONSONANT, VOWEL};
use crate::lang::common::{record, Tok, Word};
use crate::lang::wordanalyzer;

/// The analyser's root object as initAnalyzer fills it (0x1001bb40): the
/// morpheme types (`+0xc`, 5 names at 0x1008094c), the feature pairs
/// (`+0x24`, 0x48 × 8 bytes at 0x10073298) and the group descriptors
/// (`+0x30`, 0x218 × 4 bytes at 0x10080cd0).
const TYPES: u32 = 0x1008094c;
const TYPE_COUNT: i32 = 5;
const FEATURES: u32 = 0x10073298;
const FEATURE_COUNT: i32 = 0x48;
const GROUPINGS: u32 = 0x10080cd0;
/// getGroupingCount (0x1001bc30): 0x218, also the "no group" value.
pub(crate) const GROUP_COUNT: i16 = 0x218;
/// getStressGroupingDetail (0x1001bc70): 0x18-byte records at 0x10078790,
/// indexed by group without a bound check.
pub(crate) const GROUP_DETAILS: u32 = 0x10078790;
/// getGroupingOverridePatternDictionary (0x1001bc80): `{count, entries}`,
/// entries 0xc bytes `{?, count +4, pairs +8}`, pairs `{key u8, ?, group
/// u16}`.
const OVERRIDES: u32 = 0x1007d88c;

/// `STEM` (0x10023310) and `FLEX_SUFFIX` (0x10023320): the type names the
/// module compares with `ELQstricmp`.
pub(crate) const STEM: u32 = 0x10023310;
pub(crate) const FLEX_SUFFIX: u32 = 0x10023320;
pub(crate) const DERIV_SUFFIX: u32 = 0x1002332c;
pub(crate) const PREFIX: u32 = 0x10023318;
pub(crate) const COMPOUND_CONNECTOR: u32 = 0x100232fc;

/// `xxversionxx` (0x10026070) reads as the module's version string
/// (0x10026058).
const VERSION_KEY: u32 = 0x10026070;
const VERSION: u32 = 0x10026058;

/// LTTS7Util's `ELQstricmp`: A–Z folded.
pub(crate) fn stricmp_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_ascii_lowercase() == y.to_ascii_lowercase())
}

/// LesNormalizzaAccento 0x100019d0: drops `` ` `` 0x92 `'` 0xb4.
pub(super) fn normalizza_accento(w: &mut Vec<u8>) {
    w.retain(|&c| !matches!(c, b'`' | 0x92 | b'\'' | 0xb4));
}

/// Where a letter or symbol's name comes from (LesGetSymbolName 0x100034c0).
enum Name {
    /// a string in the module
    At(u32),
    /// a short name the compiler built as an immediate (at that code
    /// address)
    Lit(&'static [u8]),
    None,
}

impl Danish {
    /// LesGetSymbolName 0x100034c0: the name of a character, as a marked
    /// respelling (`^` stress, `?` stød, `:` length); punctuation and most
    /// symbols only when `mode` (its char argument) is not 0.
    pub(super) fn symbol_name(&self, c: u8, mode: u8) -> Vec<u8> {
        use Name::*;
        let m = mode != 0;
        let name = match c {
            b'!' if m => At(0x10025db8),
            b'#' if m => At(0x10025dac),
            b'$' => At(0x10025da4),
            b'%' => At(0x10025d9c),
            b'&' => At(0x10025d90),
            b'\'' if m => At(0x10025d84),
            b'(' | b')' if m => At(0x10025d70),
            b'*' => At(0x10025d64),
            b'+' => At(0x10025d5c),
            b'-' => At(0x10025d54),
            b'.' if m => At(0x10025d48),
            b'/' if m => At(0x10025d3c),
            // 0x10003a0a…: `nul`, `e?n`, `to?`
            b'0' => Lit(b"nul"),
            b'1' => Lit(b"e?n"),
            b'2' => Lit(b"to?"),
            b'3' => At(0x10025e0c),
            b'4' => At(0x10025e04),
            b'5' => At(0x10025dfc),
            b'6' => At(0x10025df4),
            b'7' => At(0x10025dec),
            b'8' => At(0x10025de4),
            b'9' => Lit(b"ni?"),
            b':' if m => At(0x10025d34),
            b';' if m => At(0x10025d28),
            b'<' => At(0x10025d14),
            b'=' => At(0x10025d08),
            b'>' => At(0x10025cf4),
            b'?' if m => At(0x10025ce0),
            b'@' if m => At(0x10025cd4),
            b'A' | b'a' => At(0x10025e64),
            b'B' | b'b' => Lit(b"be?"),
            b'C' | b'c' => Lit(b"se?"),
            b'D' | b'd' => Lit(b"de?"),
            b'E' | b'e' => At(0x10025e60),
            b'F' | b'f' => At(0x10025e5c),
            b'G' | b'g' => Lit(b"ge?"),
            b'H' | b'h' => Lit(b"h\xe5?"),
            b'I' | b'i' => At(0x10025e58),
            b'J' | b'j' => At(0x10025e50),
            b'K' | b'k' => Lit(b"k\xe5?"),
            b'L' | b'l' => At(0x10025e4c),
            b'M' | b'm' => At(0x10025e48),
            b'N' | b'n' => At(0x10025e44),
            b'O' | b'o' => At(0x10025e40),
            b'P' | b'p' => Lit(b"pe?"),
            b'Q' | b'q' => Lit(b"ku?"),
            b'R' | b'r' => At(0x10025e38),
            b'S' | b's' => At(0x10025e34),
            b'T' | b't' => Lit(b"te?"),
            b'U' | b'u' => At(0x10025e30),
            b'V' | b'v' => Lit(b"ve?"),
            b'W' | b'w' => At(0x10025e24),
            b'X' | b'x' => Lit(b"\xe6gs"),
            b'Y' | b'y' => At(0x10025e20),
            b'Z' | b'z' => Lit(b"s\xe6t"),
            b'[' if m => At(0x10025cb8),
            b'\\' if m => At(0x10025dc8),
            b']' if m => At(0x10025ca0),
            b'^' if m => At(0x10025c90),
            b'_' if m => At(0x10025c80),
            b'`' if m => At(0x10025c74),
            b'{' | b'}' if m => At(0x10025c48),
            b'|' if m => At(0x10025c60),
            b'~' if m => At(0x10025c40),
            0x82 | b',' if m => At(0x10025bc0),
            0x84 | 0x93 | 0xbb | b'"' if m => At(0x10025bac),
            0x85 if m => At(0x10025b8c),
            0x86 | 0x87 if m => At(0x10025ba4),
            0x8b if m => At(0x10025b78),
            0x91 | 0x94 if m => At(0x10025bac),
            0x92 if m => At(0x10025bc8),
            0x95 if m => At(0x10025bec),
            0x96 | 0x97 if m => At(0x10025bdc),
            0x99 if m => At(0x10025c00),
            0x9b if m => At(0x10025b64),
            0xa7 if m => At(0x10025c30),
            0xa9 | 0xae if m => At(0x10025c00),
            0xab if m => At(0x10025c1c),
            0xad if m => At(0x10025c0c),
            0xb0 if m => At(0x10025bf4),
            0xb7 if m => At(0x10025bec),
            0xc5 | 0xe5 => At(0x10025e14),
            0xc6 | 0xe6 => At(0x10025e1c),
            0xd8 | 0xf8 => At(0x10025e18),
            _ => None,
        };
        match name {
            At(va) => self.s(va).to_vec(),
            Lit(s) => s.to_vec(),
            None => Vec::new(),
        }
    }

    /// LesPronunciabile 0x10004640 → 0x10004420 (char argument 0, as
    /// LesElaboraParola passes it): the word as a pattern of vowels `V` and
    /// consonants `C` (other characters left out); not readable without a
    /// vowel, a lone vowel other than those in `aiAI`, more than four
    /// vowels or consonants in a row, the patterns `^CC$` (0x10025e80) and
    /// `^CCC$` (0x10025e78), five consonants at the start or (up to seven
    /// characters) at the end; readable with more than six characters.
    pub(super) fn pronunciabile(&self, w: &[u8]) -> bool {
        let mut pat = vec![b'^'];
        let (mut vrun, mut crun, mut vmax, mut cmax) = (0u32, 0u32, 0u32, 0u32);
        let mut in_vowel = false;
        let (mut chars, mut vowels) = (0u32, 0u32);
        let mut k = 0usize;
        while k < w.len() && w[k] != 0 && pat.len() < 0x40 && k < 0x40 {
            chars += 1;
            let c = w[k];
            if self.has(c, VOWEL) {
                if !in_vowel || k == 0 {
                    in_vowel = true;
                    cmax = cmax.max(crun);
                    crun = 0;
                    vrun = 0;
                }
                vowels += 1;
                vrun += 1;
                pat.push(b'V');
            } else if self.has(c, CONSONANT) {
                if in_vowel || k == 0 {
                    in_vowel = false;
                    vmax = vmax.max(vrun);
                    vrun = 0;
                    crun = 0;
                }
                crun += 1;
                pat.push(b'C');
            }
            k += 1;
        }
        pat.push(b'$');
        vmax = vmax.max(vrun);
        cmax = cmax.max(crun);
        if vowels == 0 {
            return false;
        }
        if chars == 1 {
            // strstr("aiAI", word): the word is a piece of `aiAI`
            if vowels == 1 && !b"aiAI".windows(w.len().max(1)).any(|x| x == w) {
                return false;
            }
        } else if chars > 6 {
            return true;
        }
        if cmax >= 5 || vmax >= 5 {
            return false;
        }
        if pat == self.s(0x10025e80) || pat == self.s(0x10025e78) {
            return false;
        }
        if pat.starts_with(b"^CCCCC") {
            return false;
        }
        !(pat.windows(6).any(|x| x == b"CCCCC$") && chars <= 7)
    }

    /// LesElaboraParola 0x10004d60: the token (0xac read as `-`) lowered
    /// through the character table (at most 63 letters) and analysed
    /// (0x10004a30) when the engine's mode is 0 and it is readable, or
    /// spelled or comes from an expansion; otherwise spelled, a record per
    /// character name (mode 1; class 0x15, sub 0x83, the last 0x84).
    pub(super) fn elabora_parola(&self, toks: &[Tok], i: usize, _idx: usize, spell: u8, mode: u8) -> Vec<Word> {
        let t: Vec<u8> = toks[i].text.iter().map(|&c| if c == 0xac { b'-' } else { c }).collect();
        let ok = self.pronunciabile(&t);
        let mut out = Vec::new();
        if mode == 0 && (ok || spell != 0 || toks[i].expanded) {
            if t.is_empty() {
                return out;
            }
            let mut lowered: Vec<u8> = t.iter().take(63).map(|&c| self.lower(c)).collect();
            if stricmp_eq(&lowered, self.s(VERSION_KEY)) {
                lowered = self.s(VERSION).iter().take(0x40).copied().collect();
            }
            self.analyse_word(&lowered, spell, false, &mut out);
        } else {
            for (k, &c) in t.iter().enumerate() {
                let mut name = self.symbol_name(c, 1);
                if name.is_empty() {
                    continue;
                }
                normalizza_accento(&mut name);
                out.push(new_record(name, 0x15, if k + 1 == t.len() { 0x84 } else { 0x83 }));
            }
        }
        out
    }

    /// 0x10004a30: one word's record and its analyses. Without any, a word
    /// with a `-` after its first letter is split there (both parts
    /// analysed on their own); one without a `-` that is not readable
    /// (unless `spell`) becomes the names of its letters (0x100042b0),
    /// class 0x15 sub 0x84. The record keeps the best analyses
    /// (0x100048a0: up to the first with more morphemes than the best one)
    /// with their morphemes' fields (getMorphemeInfo 0x1001bd60), each
    /// stem's group overridden by the group of the morpheme after it
    /// (0x10004930), an inflection after an overridden stem without one.
    fn analyse_word(&self, word: &[u8], spell: u8, force_spell: bool, out: &mut Vec<Word>) {
        let res = self.analyzer.analyse(&self.pe, word);
        let mut text = word.to_vec();
        let (mut class, mut sub) = (0x17u8, 200u8);
        if res.slots.is_empty() {
            // the first `-` after the first letter
            let hyphen = word.iter().skip(1).position(|&c| c == b'-').map(|p| p + 1);
            match hyphen {
                Some(k) if k + 1 < word.len() => {
                    self.analyse_word(&word[..k], spell, force_spell, out);
                    self.analyse_word(&word[k + 1..], spell, force_spell, out);
                    return;
                }
                Some(_) => {}
                None => {
                    let ok = self.pronunciabile(word);
                    if force_spell || (!ok && spell == 0) {
                        text = self.spelled(word);
                        class = 0x15;
                        sub = 0x84;
                    }
                }
            }
        }
        // 0x100048a0
        let n = match res.slots.len() {
            0 => 0,
            total => {
                let c0 = res.ranked(0).map_or(0, |s| s.len());
                (1..total).find(|&k| res.ranked(k).map_or(0, |s| s.len()) > c0).unwrap_or(total)
            }
        };
        let mut li = LexInfo::default();
        for k in 0..n {
            let segs = res.ranked(k).unwrap_or(&[]);
            let mut sol: Vec<Morpheme> = segs.iter().map(|s| self.morpheme_info(s)).collect();
            let mut overridden = false;
            for j in 0..sol.len() {
                let kind = sol[j].kind.clone();
                let g = if stricmp_eq(&kind, self.s(STEM)) {
                    self.stem_group(&sol, j, &mut overridden)
                } else if stricmp_eq(&kind, self.s(FLEX_SUFFIX)) && overridden {
                    GROUP_COUNT
                } else {
                    sol[j].group
                };
                if sol[j].group != g {
                    sol[j].group = g;
                    sol[j].grouping = self.grouping(g);
                }
            }
            li.solutions.push(sol);
        }
        let mut r = new_record(text, class, sub);
        li.store(&mut r);
        out.push(r);
    }

    /// 0x100042b0 (mode 0, no separator, at most 0x3f bytes): the names of
    /// the word's characters run together; a final `'` stays itself.
    fn spelled(&self, word: &[u8]) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        for (k, &c) in word.iter().enumerate() {
            let name = if c == b'\'' && k + 1 == word.len() { b"'".to_vec() } else { self.symbol_name(c, 0) };
            if out.len() + name.len() >= 0x3f {
                break;
            }
            out.extend(name);
        }
        out
    }

    /// getGroupingDescr (0x1001bc20 → 0x1001af60): the group's descriptor,
    /// 0 outside `0..0x218`.
    pub(crate) fn grouping(&self, g: i16) -> u32 {
        if g >= 0 && g < GROUP_COUNT {
            self.pe.u32(GROUPINGS + 4 * g as u32)
        } else {
            0
        }
    }

    /// getMorphemeInfo 0x1001bd60 for one segment.
    fn morpheme_info(&self, s: &wordanalyzer::Seg) -> Morpheme {
        let mut m = Morpheme {
            group: GROUP_COUNT,
            start: s.start,
            end: s.end,
            ..Default::default()
        };
        if s.morph == 0 {
            return m;
        }
        let e = wordanalyzer::morph(&self.pe, s.morph);
        m.text = e.spelling().to_vec();
        let ty = e.flag() as i32;
        if (0..TYPE_COUNT).contains(&ty) {
            m.kind = self.s(self.pe.u32(TYPES + 4 * ty as u32)).to_vec();
        }
        let id = e.id();
        if (0..FEATURE_COUNT).contains(&id) {
            let f = FEATURES + 8 * id as u32;
            m.feat = [self.pe.u32(f), self.pe.u32(f + 4)];
        }
        m.gloss = e.gloss().map(|g| g.to_vec());
        m.gloss_len = m.gloss.as_ref().map_or(0, |g| g.len() as i32);
        m.group = e.class();
        m.grouping = self.grouping(m.group);
        m.len = e.len() as i32;
        m
    }

    /// 0x10004930: a stem's group, overridden by the pattern dictionary when
    /// its group's detail names an entry (+0x16) that lists the next
    /// morpheme's group key (its detail's +0x14).
    fn stem_group(&self, sol: &[Morpheme], j: usize, overridden: &mut bool) -> i16 {
        let n = sol.len();
        if n == 0 || j >= n {
            return GROUP_COUNT;
        }
        let mut g = sol[j].group;
        if j >= n - 1 || g < 0 || g >= GROUP_COUNT {
            return g;
        }
        let detail = GROUP_DETAILS + 0x18 * g as u32;
        let idx = self.pe.u16(detail + 0x16) as u32;
        if idx == 0 || idx >= self.pe.u32(OVERRIDES) {
            return g;
        }
        let e = self.pe.u32(OVERRIDES + 4) + 0xc * idx;
        // getStressGroupingDetail takes a short: no bound check
        let next = GROUP_DETAILS.wrapping_add((sol[j + 1].group as i32 * 0x18) as u32);
        let key = self.pe.u8(next + 0x14);
        let pairs = self.pe.u32(e + 8);
        for k in 0..self.pe.u32(e + 4) as i32 {
            let p = pairs + 4 * k as u32;
            if self.pe.u8(p) == key {
                g = self.pe.u16(p + 2) as i16;
                *overridden = true;
            }
        }
        g
    }
}

/// A record as 0x100047a0 makes it: the text (at most 63 bytes), class,
/// sub, info 9, grade 0x0e, boundary 0x15.
fn new_record(mut text: Vec<u8>, class: u8, sub: u8) -> Word {
    text.truncate(0x3f);
    let mut r = record(text, class, sub, 9);
    r.f[crate::lang::common::field::GRADE] = 0x0e;
    r.f[crate::lang::common::field::BOUND] = 0x15;
    r
}

#[cfg(test)]
mod tests {
    use super::super::trace;
    use super::*;

    /// Every word LesAnalizza gets: our analysis gives the same solutions
    /// (LesElaboraParola's, before LesAnalizza changes anything).
    #[test]
    fn danish_elabora_parola_trace() {
        let (Some(calls), Some(m)) = (trace::calls(), trace::module()) else { return };
        let (mut ok, mut all) = (0, 0);
        let mut shown = 0;
        for c in calls.iter().filter(|c| c.name == "LesAnalizza") {
            for w in &c.before {
                let Some(li) = &w.lexinfo else { continue };
                // records of spelled tokens (a name per letter, sub 0x83 /
                // 0x84) come from their token, not from their text
                if w.f[0xe] == b'2' || matches!(w.f[0xf], 0x83 | 0x84) {
                    continue;
                }
                all += 1;
                let tok = Tok {
                    text: w.text.clone(),
                    kind: 0,
                    expanded: false,
                };
                let got = m.elabora_parola(&[tok], 0, 0, 0, 0);
                let gl = got.last().and_then(LexInfo::of);
                let same = got.len() == 1 && gl.as_ref().is_some_and(|g| g.solutions == li.solutions) && got[0].text == w.text && got[0].f[0xe..0x10] == [0x17, 200][..];
                if same {
                    ok += 1;
                } else if shown < 20 {
                    shown += 1;
                    eprintln!("{:?}: want {:?}\n got {:?}", String::from_utf8_lossy(&w.text), li.solutions, gl.map(|g| g.solutions));
                }
            }
        }
        eprintln!("LesElaboraParola: {ok}/{all}");
        assert_eq!(ok, all);
    }
}
