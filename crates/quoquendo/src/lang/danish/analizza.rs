//! `LesAnalizza` (0x1000b620), `LesConfiniProsodici` (0x10005c50) and
//! `LesGradiAccentuali` (0x10004730) of LTTS7Danish.dll.
//!
//! LesAnalizza runs three passes over the sentence's records:
//! 0x10009140 gives each word its `TD_Phonetizations` rows (the LexInfo
//! +8 list, here `Word::rows`), 0x10009650 picks each word's analyser
//! solution (LexInfo `head[1]`) from its neighbours' parts of speech, and
//! 0x1000b460 sets class, subclass and information from the picked
//! solution's features.
//!
//! The analyser's feature lookups go through its API table (the module's
//! `getMorphFeatureValueString_byAttrString` 0x1001bf20 → 0x1001ae60,
//! `hasMorphFeature_byString` 0x1001bf50 → 0x1001ae90): a morpheme's
//! feature set is `{u8 count, (u8 attribute, u8 value)*}`, attribute names
//! at 0x10080960 (0xb), value names at 0x10080990 (0x56).

use super::{Danish, LexInfo, Morpheme};
use crate::lang::common::field::{BOUND, CLASS, GRADE, INFO, SUB};
use crate::lang::common::Word;

/// Attribute names (analyser root +0x14, 0xb of them; 0x1001ad70).
const ATTRS: (u32, u32) = (0x10080960, 0xb);
/// Value names (root +0x1c, 0x56 of them; 0x1001ad90). 0x56 is also the
/// "no value" code getFeatureValueCount (0x1001bc50) gives 0x100091c0.
const VALUES: (u32, u32) = (0x10080990, 0x56);
/// Grouping names (root +0x30, 0x218; getGroupingDescr 0x1001af60).
const GROUPINGS: (u32, i32) = (0x10080cd0, 0x218);

// Strings the code compares, by address.
const S_EMPTY: u32 = 0x10023339;
const S_STEM: u32 = 0x10023310;
const S_FLEX_SUFFIX: u32 = 0x10023320;
const S_POS: u32 = 0x10026f20;
const S_CASE: u32 = 0x10027004;
const S_GENDER: u32 = 0x100270c4;
const S_NUMBER: u32 = 0x100270bc;
const S_VERB_FORM: u32 = 0x10026ee4;
const S_DEFINITENESS: u32 = 0x100270f8;
const S_N: u32 = 0x10023fe8;
const S_M: u32 = 0x10023ff8;
const S_NF: u32 = 0x10026f08;
const S_NM: u32 = 0x10026f04;
const S_NN: u32 = 0x10026f00;
const S_NP: u32 = 0x10026fec;
const S_NPLUR: u32 = 0x10027084;
const S_ADJ: u32 = 0x10026efc;
const S_ADV: u32 = 0x10027018;
const S_ART: u32 = 0x10026ef0;
const S_CONJ: u32 = 0x10026ef4;
const S_NUM: u32 = 0x10027080;
const S_NUMCARD: u32 = 0x10027078;
const S_NUMNON: u32 = 0x10027070;
const S_PREP: u32 = 0x10026f0c;
const S_PRON: u32 = 0x10026f18;
const S_V: u32 = 0x10026f14;
const S_GEN: u32 = 0x10027000;
const S_GEN2: u32 = 0x10026ff8;
const S_NOM: u32 = 0x100270d8;
const S_ACC: u32 = 0x100270d4;
const S_DEF: u32 = 0x100270d0;
const S_IND: u32 = 0x100270cc;
const S_INF: u32 = 0x10026edc;
const S_PRS: u32 = 0x100270f4;
const S_PRT: u32 = 0x100270f0;
const S_SUP: u32 = 0x100270ec;
const S_PCPRF: u32 = 0x100270e4;
const S_PCPRS: u32 = 0x100270dc;
const S_IMP: u32 = 0x10026ee0;
const S_F: u32 = 0x100270b8;
const S_NE: u32 = 0x100270b4;
const S_NE2: u32 = 0x10027094;
const S_NEU: u32 = 0x10027090;
const S_M2: u32 = 0x10027098;
const S_UTR: u32 = 0x1002708c;
const S_SGL: u32 = 0x100270a4;
const S_SGL2: u32 = 0x1002709c;
const S_PLU: u32 = 0x100270b0;
const S_PLU2: u32 = 0x100270a8;
const S_FA1: u32 = 0x10026ff0;
/// "og" (0x1000b5b0): the record becomes class 0x82
const S_OG: u32 = 0x100239c0;
// Words 0x10009650 tests (ELQstricmp, A–Z folded).
const W_DER: u32 = 0x1002706c;
const W_MODALS: [u32; 7] = [0x10027064, 0x1002705c, 0x10027054, 0x1002704c, 0x10027044, 0x1002703c, 0x10027034];
const W_EN: u32 = 0x10027030;
const W_ET: u32 = 0x10026518;
const W_DEN: u32 = 0x1002702c;
const W_DET: u32 = 0x10027028;
const W_LIVE: u32 = 0x10027020;
const W_I: u32 = 0x100241dc;
const W_AT: u32 = 0x1002701c;
const W_FOR: u32 = 0x10027014;
const W_SAA: u32 = 0x10027010;
const W_HV: u32 = 0x1002700c;

// Solution patterns (`{ {attr*, value*}*, u8 count }`) 0x100093c0 and
// 0x100094d0 score solutions with.
const P_PRON: u32 = 0x10026f94;
const P_V: u32 = 0x10026f9c;
const P_PREP: u32 = 0x10026fa4;
/// POS N, NF, NM or NN (count 4: never complete, the best scoring wins)
const P_N: u32 = 0x10026fac;
const P_ADJ: u32 = 0x10026fb4;
const P_CONJ: u32 = 0x10026fbc;
const P_ART: u32 = 0x10026fc4;
const P_VIMP: u32 = 0x10026fcc;
const P_VINF: u32 = 0x10026fd4;

/// Fixed phrases 0x100054d0 finds (`ELQSearchString` mode 1, by word
/// count): 6 0x10026008, 5 0x1002600c, 4 0x10026010 (2 entries), 3
/// 0x10026018, 2 0x1002601c; 0x10005300's: 3 0x10025ffc, 2 0x10026000, 1
/// 0x10026004.
const PHRASES: [(usize, u32, u32); 5] = [(6, 0x10026008, 1), (5, 0x1002600c, 1), (4, 0x10026010, 2), (3, 0x10026018, 1), (2, 0x1002601c, 1)];
const PHRASES_PIPPO: [(usize, u32, u32); 3] = [(3, 0x10025ffc, 1), (2, 0x10026000, 1), (1, 0x10026004, 1)];

/// Verb classes, the adverbial classes and the nominal ones 0x10005780
/// groups words by.
const VERB: [u8; 8] = [0x2a, 0x2b, 0x2c, 0xda, 0xd4, 0xd1, 0xd2, 0xd3];
const ADVB: [u8; 3] = [0xd9, 0x09, 0xca];
const NOMINAL: [u8; 11] = [0x37, 0x15, 0x29, 0xd5, 0xd7, 0xcb, 0xcc, 0xcd, 0xce, 0xcf, 0xd0];

pub(super) type Feat = [u32; 2];

impl Danish {
    pub(super) fn analizza(&self, w: &mut [Word]) {
        self.phonetization_rows(w);
        let mut lis: Vec<Option<LexInfo>> = w.iter().map(LexInfo::of).collect();
        self.disambiguate(w, &mut lis);
        self.classes(w, &lis);
        for (r, l) in w.iter_mut().zip(&lis) {
            if let Some(l) = l {
                l.store(r);
            }
        }
    }

    /// 0x10009140 → 0x100090a0: every record but punctuation gets the
    /// dictionary's rows for its text (`ELQDBTableFind`), in table order.
    fn phonetization_rows(&self, w: &mut [Word]) {
        for r in w.iter_mut() {
            if r.f[CLASS] == b'2' || self.phonetizations.is_empty() {
                continue;
            }
            let from = self.phonetizations.partition_point(|(g, _)| g.as_slice() < r.text.as_slice());
            r.rows = self.phonetizations[from..].iter().take_while(|(g, _)| *g == r.text).map(|(g, p)| vec![Some(g.clone()), Some(p.clone())]).collect();
        }
    }

    // --- the analyser's feature lookups -----------------------------------

    /// 0x1001ad90: a value code's name.
    pub(super) fn value_name(&self, code: u8) -> Option<&[u8]> {
        ((code as u32) < VALUES.1).then(|| self.s(self.pe.u32(VALUES.0 + 4 * code as u32)))
    }

    /// 0x1001ade0: the value code of the first pair of `feat` whose
    /// attribute is `name`.
    pub(super) fn attr_code(&self, feat: Feat, name: &[u8]) -> Option<u8> {
        (0..(feat[0] as u8) as u32).find_map(|k| {
            let a = self.pe.u8(feat[1] + 2 * k) as u32;
            (a < ATTRS.1 && self.s(self.pe.u32(ATTRS.0 + 4 * a)) == name).then(|| self.pe.u8(feat[1] + 2 * k + 1))
        })
    }

    /// getMorphFeatureValueString_byAttrString (root +0x38): whether the
    /// attribute is there, and its value's name.
    fn feature(&self, feat: Feat, name: u32) -> Option<Option<&[u8]>> {
        self.attr_code(feat, self.s(name)).map(|c| self.value_name(c))
    }

    /// 0x10009060: both strings there and equal.
    fn eq(&self, v: Option<&[u8]>, s: u32) -> bool {
        v.is_some_and(|v| v == self.s(s))
    }

    /// 0x100091c0: the solution's value for attribute `name` (its stem's,
    /// else its last inflection's after the stem), `""` for none.
    fn sol_value<'a>(&'a self, sol: Option<&[Morpheme]>, name: &[u8]) -> &'a [u8] {
        let mut code = None;
        if let Some(sol) = sol {
            let (mut stem, mut flex) = (None, None);
            for m in sol.iter().rev() {
                if m.kind == self.s(S_FLEX_SUFFIX) {
                    flex.get_or_insert(m.feat);
                } else if m.kind == self.s(S_STEM) {
                    stem = Some(m.feat);
                    break;
                }
            }
            code = stem.and_then(|f| self.attr_code(f, name)).or_else(|| flex.and_then(|f| self.attr_code(f, name)));
        }
        self.value_name(code.unwrap_or(VALUES.1 as u8)).unwrap_or(b"")
    }

    // --- 0x10009650 --------------------------------------------------------

    /// A pattern's `(attribute, value)` pairs and its count byte.
    fn pattern(&self, va: u32) -> (Vec<(&[u8], &[u8])>, i32) {
        let n = self.pe.u8(va + 4) as u32;
        let p = self.pe.u32(va);
        ((0..n).map(|k| (self.s(self.pe.u32(p + 8 * k)), self.s(self.pe.u32(p + 8 * k + 4)))).collect(), n as i32)
    }

    /// 0x10009350: how many of the pattern's pairs the solution has.
    fn score(&self, sol: &[Morpheme], pat: u32) -> i32 {
        self.pattern(pat).0.iter().filter(|(a, v)| self.sol_value(Some(sol), a) == *v).count() as i32
    }

    /// 0x100094d0: whether one of the record's solutions has all of `p1`
    /// and not all of `p2`.
    fn has_sol(&self, lis: &[Option<LexInfo>], rec: Option<usize>, p1: u32, p2: u32) -> bool {
        let Some(l) = rec.and_then(|r| lis[r].as_ref()) else { return false };
        let n1 = self.pattern(p1).1;
        l.solutions.iter().any(|s| {
            let s1 = self.score(s, p1);
            s1 > 0 && s1 >= n1 && (p2 == 0 || self.score(s, p2) < self.pattern(p2).1)
        })
    }

    /// 0x100093c0: the solution scoring best on `p1` (0 when it has all of
    /// `p2`) becomes the record's, unless it is the first one; then its
    /// part of speech goes to `out`.
    fn pick<'a>(&'a self, lis: &mut [Option<LexInfo>], rec: Option<usize>, p1: u32, p2: u32, out: &mut &'a [u8]) {
        let Some(r) = rec else { return };
        let Some(l) = lis[r].as_mut() else { return };
        let n1 = self.pattern(p1).1;
        let mut best = 0;
        let mut pick = l.head[1];
        for (u, s) in l.solutions.iter().enumerate() {
            let mut sc = self.score(s, p1);
            if p2 != 0 && self.score(s, p2) >= self.pattern(p2).1 {
                sc = 0;
            }
            if sc > best {
                best = sc;
                pick = u as i32;
                if sc >= n1 {
                    break;
                }
            }
        }
        if pick > 0 && pick != l.head[1] {
            l.head[1] = pick;
            *out = self.pos(lis, rec);
        }
    }

    /// 0x100092a0: the record's part of speech.
    fn pos<'a>(&'a self, lis: &[Option<LexInfo>], rec: Option<usize>) -> &'a [u8] {
        self.sol_value(rec.and_then(|r| lis[r].as_ref()).and_then(|l| l.solution()), self.s(S_POS))
    }

    /// 0x100095c0: the parts of speech of all the record's solutions.
    fn pos_list<'a>(&'a self, lis: &[Option<LexInfo>], rec: usize) -> Vec<&'a [u8]> {
        lis[rec].as_ref().map_or(Vec::new(), |l| l.solutions.iter().map(|s| self.sol_value(Some(s), self.s(S_POS))).collect())
    }

    /// 0x10009330 then getGroupingDescr: the name of the record's stem's
    /// grouping.
    fn stem_grouping(&self, lis: &[Option<LexInfo>], rec: Option<usize>) -> Option<&[u8]> {
        let sol = rec.and_then(|r| lis[r].as_ref()).and_then(|l| l.solution())?;
        let stem = sol.iter().rev().find(|m| m.kind == self.s(S_STEM))?;
        let g = stem.group as i32;
        (0..GROUPINGS.1).contains(&g).then(|| self.s(self.pe.u32(GROUPINGS.0 + 4 * g as u32)))
    }

    /// 0x10009650: each word's solution from the parts of speech of the
    /// words around it.
    fn disambiguate(&self, w: &[Word], lis: &mut [Option<LexInfo>]) {
        let n = w.len();
        let s = |va: u32| self.s(va);
        let ieq = |r: Option<usize>, va: u32| r.is_some_and(|r| w[r].text.eq_ignore_ascii_case(s(va)));
        let any = |l: &[&[u8]], va: u32| l.iter().any(|p| *p == s(va));
        let question = n > 1 && w[n - 1].text.first() == Some(&b'?');
        let empty: &[u8] = s(S_EMPTY);
        let (mut prev, mut next, mut last_rec): (Option<usize>, Option<usize>, Option<usize>) = (None, None, None);
        let (mut a0, mut a9c, mut last_pos) = (empty, empty, empty);
        let (mut l58, mut l88, mut l2c): (Vec<&[u8]>, Vec<&[u8]>, Vec<&[u8]>) = Default::default();
        for i in 0..n {
            if i != 0 {
                a0 = last_pos;
                prev = last_rec;
                l58 = l88.clone();
            }
            let (rec, mut a4);
            if last_rec.is_none() {
                rec = Some(i);
                a4 = self.pos(lis, rec);
                l88 = self.pos_list(lis, i);
            } else {
                rec = next;
                a4 = a9c;
                l88 = l2c.clone();
            }
            if i + 1 < n && w[i + 1].f[CLASS] != b'2' {
                next = Some(i + 1);
                a9c = self.pos(lis, next);
                l2c = self.pos_list(lis, i + 1);
            } else {
                next = None;
                a9c = empty;
                l2c.clear();
            }
            let is = |p: &[u8], va: u32| p == s(va);
            let nounish = |p: &[u8]| is(p, S_N) || is(p, S_NF) || is(p, S_NM) || is(p, S_NN);
            let en_et = ieq(prev, W_EN) || ieq(prev, W_ET);
            let no_adj_adv_prep_pron = |l: &[&[u8]]| !any(l, S_ADJ) && !any(l, S_ADV) && !any(l, S_PREP) && !any(l, S_PRON);
            if prev.is_some() {
                if !is(a0, S_ART) && any(&l58, S_ART) && (is(a4, S_N) || (is(a4, S_ADJ) && is(a9c, S_N))) {
                    self.pick(lis, prev, P_ART, 0, &mut a0);
                }
                if !is(a0, S_PRON) && ieq(prev, W_DER) {
                    self.pick(lis, prev, P_PRON, 0, &mut a0);
                }
            }
            if nounish(a0) && any(&l58, S_PREP) {
                self.pick(lis, prev, P_PREP, 0, &mut a0);
            }
            if prev.is_some() && rec.is_some() && (!is(a0, S_V) || !is(a4, S_V)) && W_MODALS.iter().any(|&m| ieq(prev, m)) && any(&l88, S_V) && self.has_sol(lis, rec, P_V, P_VIMP) {
                self.pick(lis, prev, P_V, 0, &mut a0);
                self.pick(lis, rec, P_V, P_VIMP, &mut a4);
            }
            if is(a4, S_N) && is(a9c, S_N) && en_et {
                self.pick(lis, rec, P_ADJ, 0, &mut a4);
            }
            if is(a4, S_ADJ) && is(a9c, S_ADJ) && en_et {
                self.pick(lis, next, P_N, 0, &mut a9c);
            }
            if !is(a4, S_ADJ) && (ieq(prev, W_DEN) || ieq(prev, W_DET)) {
                self.pick(lis, rec, P_ADJ, 0, &mut a4);
            }
            if !is(a4, S_N) && !is(a9c, S_N) && en_et {
                self.pick(lis, rec, P_N, 0, &mut a4);
            }
            let vimp = |lis: &[Option<LexInfo>], r: Option<usize>| self.has_sol(lis, r, P_VIMP, 0);
            let not_for = rec.is_none() || !ieq(rec, W_FOR);
            // (on the previous word, pattern, excluded pattern)
            let act: Option<(bool, u32, u32)> = 'c: {
                if !is(a4, S_V) && ieq(rec, W_LIVE) && ieq(prev, W_I) {
                    break 'c Some((false, P_V, 0));
                }
                if !is(a0, S_PRON) && any(&l58, S_V) && any(&l58, S_PRON) && is(a4, S_V) && is(a9c, S_V) {
                    break 'c Some((true, P_PRON, 0));
                }
                if !is(a4, S_V) && any(&l88, S_V) && (any(&l88, S_N) || any(&l88, S_NF) || any(&l88, S_ADJ)) && ieq(prev, W_AT) && !any(&l88, S_ADV) && !any(&l88, S_PREP) && !any(&l88, S_PRON) && self.has_sol(lis, rec, P_V, P_VIMP) && not_for {
                    break 'c Some((false, P_V, P_VIMP));
                }
                if is(a0, S_PRON) && !is(a4, S_N) && any(&l88, S_N) && vimp(lis, rec) && no_adj_adv_prep_pron(&l88) {
                    break 'c Some((false, P_N, 0));
                }
                if !is(a0, S_PREP) && !is(a0, S_N) && !is(a0, S_NF) && any(&l58, S_V) && (any(&l58, S_N) || any(&l58, S_NF)) && is(a4, S_V) && (any(&l2c, S_V) || any(&l2c, S_PRON)) && !any(&l2c, S_ADJ) && !any(&l2c, S_ADV) {
                    break 'c Some((true, P_N, 0));
                }
                if i == 1 && !question && any(&l58, S_N) && vimp(lis, prev) && rec.is_some() && !is(a4, S_CONJ) && !is(a4, S_V) {
                    break 'c Some((true, P_VIMP, 0));
                }
                let noun_imp = any(&l88, S_N) && vimp(lis, rec) && no_adj_adv_prep_pron(&l88);
                if is(a0, S_ADJ) && !is(a4, S_N) && noun_imp {
                    break 'c Some((false, P_N, 0));
                }
                if i != 0 && (is(a0, S_EMPTY) || is(a0, S_ADJ)) && is(a4, S_V) && noun_imp {
                    break 'c Some((false, P_N, 0));
                }
                if is(a0, S_CONJ) && !is(a4, S_N) && prev.is_some_and(|p| w[p].text.len() >= 2 && w[p].text[..2].eq_ignore_ascii_case(&s(W_HV)[..2])) && noun_imp {
                    break 'c Some((false, P_N, 0));
                }
                if (is(a0, S_ADJ) || is(a0, S_N)) && !is(a4, S_N) && next.is_none() && noun_imp {
                    break 'c Some((false, P_N, 0));
                }
                if !is(a4, S_V) && ieq(prev, W_AT) && any(&l88, S_V) && self.has_sol(lis, rec, P_VINF, 0) {
                    break 'c Some((false, P_VINF, 0));
                }
                if is(a0, S_N) && is(a4, S_N) && any(&l88, S_V) {
                    let c = self.sol_value(prev.and_then(|p| lis[p].as_ref()).and_then(|l| l.solution()), s(S_CASE));
                    if !is(c, S_GEN) && !is(c, S_GEN2) {
                        break 'c Some((false, P_V, P_VIMP));
                    }
                }
                if is(a0, S_V) && is(a4, S_V) && any(&l88, S_N) && !any(&l88, S_ADJ) && !any(&l88, S_PRON) && !any(&l88, S_ADV) && vimp(lis, rec) {
                    break 'c Some((false, P_N, 0));
                }
                if !is(a4, S_V) && any(&l88, S_V) && (any(&l88, S_N) || any(&l88, S_NF)) && no_adj_adv_prep_pron(&l88) && self.has_sol(lis, rec, P_V, P_VIMP) && not_for {
                    let fa1 = |d: Option<&[u8]>| d.is_some_and(|d| d.windows(4).any(|x| x == s(S_FA1)));
                    let d1 = self.stem_grouping(lis, prev);
                    let d2 = self.stem_grouping(lis, next);
                    if ieq(prev, W_AT)
                        || (is(a0, S_V) && fa1(d1))
                        || (is(a0, S_NP) && is(a9c, S_PRON))
                        || is(a0, S_PRON)
                        || is(a9c, S_ADV)
                        || ieq(prev, W_DEN)
                        || ieq(prev, W_DET)
                        || ieq(prev, W_DER)
                        || is(a0, S_N)
                        || is(a0, S_NF)
                        || (is(a9c, S_V) && fa1(d2))
                        || is(a9c, S_N)
                        || is(a9c, S_NF)
                        || is(a9c, S_PRON)
                        || is(a0, S_ADV)
                    {
                        break 'c Some((false, P_V, P_VIMP));
                    }
                }
                None
            };
            match act {
                Some((true, p1, p2)) => self.pick(lis, prev, p1, p2, &mut a0),
                Some((false, p1, p2)) => self.pick(lis, rec, p1, p2, &mut a4),
                None => {}
            }
            if rec.is_none() || !ieq(rec, W_SAA) {
                let np = |p: &[u8]| nounish(p) || is(p, S_PRON);
                let act: Option<(u32, u32)> = if !is(a4, S_V) && any(&l88, S_CONJ) && any(&l88, S_V) && !any(&l88, S_N) && self.has_sol(lis, rec, P_V, P_VIMP) && np(a0) && np(a9c) && not_for {
                    Some((P_V, P_VIMP))
                } else if !is(a4, S_ADJ) && any(&l88, S_ADJ) && any(&l88, S_N) && !any(&l88, S_ADV) && !any(&l88, S_V) && np(a9c) {
                    Some((P_ADJ, 0))
                } else if !is(a4, S_V) && any(&l88, S_V) && any(&l88, S_PRON) && !any(&l88, S_N) && !any(&l88, S_ADV) && self.has_sol(lis, rec, P_V, P_VIMP) && nounish(a9c) && !is(a0, S_EMPTY) && !is(a0, S_V) && !is(a0, S_PRON) && not_for {
                    Some((P_V, P_VIMP))
                } else if is(a0, S_N) && !is(a4, S_N) && any(&l88, S_V) && any(&l88, S_N) && !any(&l88, S_ADJ) && !any(&l88, S_ADV) && self.has_sol(lis, rec, P_V, P_VIMP) {
                    let c = self.sol_value(prev.and_then(|p| lis[p].as_ref()).and_then(|l| l.solution()), s(S_CASE));
                    (is(c, S_GEN) || is(c, S_GEN2)).then_some((P_N, 0))
                } else {
                    None
                };
                if let Some((p1, p2)) = act {
                    self.pick(lis, rec, p1, p2, &mut a4);
                }
            } else if !is(a4, S_CONJ) {
                self.pick(lis, rec, P_CONJ, 0, &mut a4);
            }
            last_pos = a4;
            last_rec = rec;
        }
    }

    // --- 0x1000b460 --------------------------------------------------------

    /// 0x1000b460: class (0x17 unset), information (9 unset) and subclass
    /// (0xc8 unset) from the picked solution's stem and last inflection;
    /// numbers get subclass `y` (one before it `y` → `z`), `og` class 0x82.
    fn classes(&self, w: &mut [Word], lis: &[Option<LexInfo>]) {
        // the last morpheme the scan looked at stays over records whose
        // class is set (0x1000b4dd)
        let mut last: Option<Feat> = None;
        for i in 0..w.len() {
            let Some(sol) = lis[i].as_ref().and_then(|l| l.solution()).filter(|s| !s.is_empty()) else { continue };
            let (mut stem, mut flex) = (None, None);
            if w[i].f[CLASS] == 0x17 {
                for m in sol.iter().rev() {
                    last = Some(m.feat);
                    if m.kind == self.s(S_FLEX_SUFFIX) {
                        flex.get_or_insert(m.feat);
                    } else if m.kind == self.s(S_STEM) {
                        stem = Some(m.feat);
                        break;
                    }
                }
            }
            let Some(lf) = last else { continue };
            let (stem, flex) = (stem.unwrap_or([0, 0]), flex.unwrap_or([0, 0]));
            if w[i].f[CLASS] == 0x17 {
                w[i].f[CLASS] = self.class_of(stem);
            }
            if w[i].f[INFO] == 9 {
                w[i].f[INFO] = self.info_of(flex, stem);
            }
            if w[i].f[SUB] == 0xc8 {
                w[i].f[SUB] = self.sub_of(flex);
            }
            if self.attr_code(lf, self.s(S_POS)).is_some_and(|c| self.value_name(c) == Some(self.s(S_NUM))) {
                if i > 0 && w[i - 1].f[SUB] == b'y' {
                    w[i - 1].f[SUB] = b'z';
                }
                w[i].f[SUB] = b'y';
            }
            if w[i].text == self.s(S_OG) {
                w[i].f[CLASS] = 0x82;
            }
        }
    }

    /// 0x1000acc0: the stem's part of speech as the record class.
    fn class_of(&self, stem: Feat) -> u8 {
        let Some(v) = self.feature(stem, S_POS) else { return 0xc8 };
        let e = |s| self.eq(v, s);
        if [S_N, S_NF, S_NM, S_NN, S_NP, S_NPLUR].into_iter().any(e) {
            0x15
        } else if e(S_ADJ) {
            0x29
        } else if e(S_ADV) {
            0x33
        } else if e(S_ART) {
            0x18
        } else if e(S_CONJ) {
            0x1f
        } else if e(S_NUM) || e(S_NUMCARD) || e(S_NUMNON) {
            0x15
        } else if e(S_PREP) {
            0x19
        } else if e(S_PRON) {
            0x20
        } else if e(S_V) {
            0x2a
        } else {
            0xc8
        }
    }

    /// 0x1000ae60: gender and number (the inflection's, or a noun stem's
    /// gender, `NPLUR` plural) as the information byte.
    fn info_of(&self, flex: Feat, stem: Feat) -> u8 {
        let empty = Some(self.s(S_EMPTY));
        let ga = self.feature(flex, S_GENDER);
        let nu = self.feature(flex, S_NUMBER);
        let pos = self.feature(stem, S_POS);
        let mut a = ga.unwrap_or(empty);
        let mut b = nu.unwrap_or(empty);
        let mut gender = ga.is_some();
        if !gender {
            if let Some(c) = pos {
                for (p, g) in [(S_NF, S_F), (S_NM, S_M), (S_NN, S_NE)] {
                    if self.eq(c, p) {
                        a = Some(self.s(g));
                        gender = true;
                        break;
                    }
                }
            }
        }
        if nu.is_none() {
            if pos.is_some_and(|c| self.eq(c, S_NPLUR)) {
                b = Some(self.s(S_PLU));
            } else if !gender {
                return 9;
            }
        }
        let e = |v: Option<&[u8]>, s: u32| self.eq(v, s);
        let plural = e(b, S_PLU) || e(b, S_PLU2);
        let single = e(b, S_SGL) || e(b, S_SGL2);
        if !gender {
            return if plural { 0x10 } else { 0x0f };
        }
        let f = e(a, S_F);
        let m = e(a, S_M) || e(a, S_M2);
        let ne = e(a, S_NE) || e(a, S_NE2) || e(a, S_NEU);
        let utr = e(a, S_UTR);
        match () {
            _ if f && single => 0x0d,
            _ if f && plural => 0x0e,
            _ if m && single => 0x0b,
            _ if m && plural => 0x0c,
            _ if ne && single => 0xcf,
            _ if ne && plural => 0xce,
            _ if ne => 0xd0,
            _ if utr && single => 0xcc,
            _ if utr && plural => 0xcb,
            _ if utr => 0xcd,
            _ => 9,
        }
    }

    /// 0x1000b2a0: verb form, else case, else definiteness of the
    /// inflection as the subclass.
    fn sub_of(&self, flex: Feat) -> u8 {
        let table = |v: Option<&[u8]>, t: &[(u32, u8)]| t.iter().find(|&&(s, _)| self.eq(v, s)).map_or(0xc8, |&(_, c)| c);
        if let Some(v) = self.feature(flex, S_VERB_FORM) {
            table(v, &[(S_INF, 0x8d), (S_PRS, 0xd1), (S_PRT, 0xd2), (S_SUP, 0xd3), (S_PCPRF, 0x88), (S_PCPRS, 0x8e), (S_IMP, 0x87)])
        } else if let Some(v) = self.feature(flex, S_CASE) {
            table(v, &[(S_NOM, 0xd5), (S_GEN, 0xd6), (S_ACC, 0xd7)])
        } else if let Some(v) = self.feature(flex, S_DEFINITENESS) {
            table(v, &[(S_DEF, 0xd8), (S_IND, 0xd9)])
        } else {
            0xc8
        }
    }

    // --- LesConfiniProsodici -------------------------------------------------

    /// LesConfiniProsodici (0x10005c50): each word's boundary 0x15 from
    /// its group kind (0x10005780): 5 inside a group, 3 at its end, 9, 1
    /// before a conjunction, the punctuation's; a pause 0x0a before a
    /// group when both sides are long enough (0x10005000) and the words may
    /// part (0x10005130); numbers (`y`/`z`/`{`) end their groups 7/6/8.
    pub(super) fn confini(&self, w: &mut [Word], mode: i32) {
        let kinds = self.group_kinds(w);
        let class = |w: &[Word], i: usize| w.get(i).map_or(0, |r| r.f[CLASS]);
        let mut last = 200usize;
        let mut i = 0usize;
        while kinds[i].0 != 10 && i < 200 {
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
                            if last != 200 && kinds[last].1 == 0 && mode != 3 && mode != 2 && kinds[last].0 != 5 && !too_short(w, last, i) && may_part(w, last) {
                                w[last].f[BOUND] = 0x0a;
                            }
                            w[i].f[BOUND] = 5;
                            last = i;
                            sub_rule = true;
                        }
                    }
                    2 => {
                        if kinds[i].1 == 1 && !too_short(w, i, i) && may_part(w, i) {
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
                        if mode != 3 && mode != 2 && last != 200 && !too_short(w, last, i) && may_part(w, i) && kinds[i + 1].0 != 10 && class(w, last) != class(w, i + 1) {
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
                            let t = &w[i].text;
                            let at = |k: usize| t.get(k).copied().unwrap_or(0);
                            let mut k = 0usize;
                            while at(k + 1) != 0 && !b",)(-.\0".contains(&at(k)) {
                                k += 1;
                            }
                            match at(k) {
                                b',' | b'-' | b'.' => Some(0x0b),
                                b'(' | b')' if !(i == 0 || class(w, i - 1) == b'2' || class(w, i + 1) == b'2') => Some(0x0b),
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
            if sub_rule {
                match w[i].f[SUB] {
                    b'y' | 0x84 => w[i].f[BOUND] = 7,
                    b'z' | 0x83 => w[i].f[BOUND] = 6,
                    b'{' => w[i].f[BOUND] = 8,
                    _ => {}
                }
            }
            i += 1;
        }
        if let Some(r) = w.get_mut(i) {
            r.f[BOUND] = 0x17;
        }
    }

    /// 0x10005780: a kind per record (5 inside a group, 7 a group's words
    /// before its end, 1/0/2 the end of a verb, nominal or adverbial+noun
    /// group, 6 before a conjunction or `og`, 8 punctuation; flag 1 when a
    /// group ending a sentence-initial adverb/preposition or fixed phrase),
    /// fixed phrases (0x100054d0) not split. Ended by kind 10.
    fn group_kinds(&self, w: &[Word]) -> Vec<(u32, u32)> {
        let class = |i: usize| w.get(i).map_or(0, |r| r.f[CLASS]);
        let mut k: Vec<(u32, u32)> = vec![(0, 0); w.len() + 1];
        let fill = |k: &mut Vec<(u32, u32)>, a: usize, b: usize| {
            for x in a..b {
                k[x] = (7, 0);
            }
        };
        let (mut state, mut loclen, mut locpos, mut start) = (0u32, 0usize, 0usize, 0usize);
        for i in 0..w.len() {
            k[i] = (5, 0);
            if loclen == 0 {
                if let Some(n) = self.phrase(w, i, &PHRASES, 5) {
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
                    if c == 0x37 || ADVB.contains(&c) {
                        state = 1;
                    } else if VERB.contains(&c) {
                        state = 2;
                    } else {
                        state = 0;
                        to_start = true;
                    }
                }
                2 => {
                    if VERB.contains(&class(i + 1)) {
                        state = 2;
                    } else if loclen == 0 {
                        fill(&mut k, start, i - 1);
                        k[i - 1].0 = 1;
                        state = 0;
                        to_start = true;
                    }
                }
                5 => {
                    let joined = i > 0 && w[i].f[SUB] == 0x82 && class(i - 1) == class(i + 1) && w[i - 1].f[SUB] == w.get(i + 1).map_or(0, |r| r.f[SUB]);
                    if NOMINAL.contains(&c) || joined {
                        state = 5;
                    } else if loclen == 0 {
                        fill(&mut k, start, i - 1);
                        k[i - 1].0 = 0;
                        state = 0;
                        to_start = true;
                    }
                }
                7 => {
                    if NOMINAL.contains(&c) {
                        state = 7;
                    } else if loclen == 0 {
                        fill(&mut k, start, i - 1);
                        k[i - 1].0 = 2;
                        if start == 0 {
                            let c0 = class(0);
                            if c0 == 0x33 || c0 == 0x19 || self.phrase(w, 0, &PHRASES, 5).is_some_and(|n| n < i) || self.phrase(w, 0, &PHRASES_PIPPO, 3).is_some_and(|n| n < i) {
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
                start = i;
                if c == 0x1f || c == 0x82 {
                    state = 8;
                } else if ADVB.contains(&c) {
                    state = 1;
                } else if VERB.contains(&c) {
                    state = 2;
                } else if NOMINAL.contains(&c) {
                    state = 5;
                } else if c == b'2' {
                    k[i].0 = 8;
                }
            }
        }
        k[w.len()].0 = 10;
        k
    }

    /// 0x100054d0 / 0x10005300: the fixed phrase starting at record `i`
    /// (its words joined by spaces, at most `max`, cut from the end), its
    /// word count.
    fn phrase(&self, w: &[Word], i: usize, tables: &[(usize, u32, u32)], max: usize) -> Option<usize> {
        let words: Vec<&[u8]> = w[i..].iter().take(max).map(|r| r.text.as_slice()).collect();
        tables.iter().filter(|&&(n, _, _)| n <= words.len()).find_map(|&(n, t, count)| {
            let key = words[..n].join(&b' ');
            (0..count).any(|k| self.s(self.pe.u32(t + 4 * k)) == key.as_slice()).then_some(n)
        })
    }

    // --- LesGradiAccentuali --------------------------------------------------

    /// LesGradiAccentuali (0x10004730): punctuation and `7` records 0x0f,
    /// words without a grade (0x0e) 0; the record after the last gets grade
    /// 0x10 and boundary 0x17.
    pub(super) fn gradi(&self, w: &mut [Word]) {
        let mut i = 0;
        while i < w.len() && i < 200 && w[i].f[BOUND] != 0x17 {
            let r = &mut w[i];
            if r.f[CLASS] == b'2' || r.f[CLASS] == b'7' {
                r.f[GRADE] = 0x0f;
            } else if r.f[GRADE] == 0x0e {
                r.f[GRADE] = 0;
            }
            i += 1;
        }
        if let Some(r) = w.get_mut(i) {
            r.f[GRADE] = 0x10;
            r.f[BOUND] = 0x17;
        }
    }
}

/// 0x10005000: whether a pause after `b` (the group ending at `cur`) would
/// leave a side shorter than 5 words or 22 letters (left: back from `b` to
/// a pause or comma; right: after `b` to the punctuation; `7` records not
/// counted on the right).
fn too_short(w: &[Word], b: usize, cur: usize) -> bool {
    let (mut lw, mut ll) = (1u32, w[b].text.len() as u32);
    let mut e = b;
    while e > 0 {
        e -= 1;
        let bound = w[e].f[BOUND];
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
    while d < w.len() && w[d].f[CLASS] != b'2' {
        if w[d].f[CLASS] != b'7' {
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
    !(lw > 4 && ll > 0x15 && rw > 4 && rl > 0x15)
}

/// 0x10005130: whether words `i` and `i + 1` may part (not article or
/// adjective + noun, a conjunction, preposition + its noun or pronoun, a
/// verb + its complement…).
fn may_part(w: &[Word], i: usize) -> bool {
    let c = w[i].f[CLASS];
    let n = w.get(i + 1).map_or(0, |r| r.f[CLASS]);
    if (c == 0x18 && (n == 0x15 || n == 0x29))
        || c == 0x1f
        || (c == 0x20 && n == 0x20)
        || (c == 0x29 && n == 0x15)
        || (c == 0x15 && matches!(n, 0x29 | 0x1f | 0x82))
        || (c == 0x20 && n == 0x1f)
        || (c == 0x19 && matches!(n, 0x15 | 0x29 | 0x20))
    {
        return false;
    }
    if i + 1 >= w.len() {
        return true;
    }
    if VERB.contains(&c) && (matches!(n, 0x33 | 0x29 | 0x18 | 0x15 | 0x19 | 0x1e) || VERB.contains(&n)) {
        return false;
    }
    if c == 0x33 && VERB.contains(&n) {
        return false;
    }
    if c == 0x20 && (n == 0x20 || n == 0x33) {
        return false;
    }
    if c == 0x33 && (n == 0x20 || n == 0x1f) {
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

#[cfg(test)]
mod tests {
    use super::super::trace;

    #[test]
    fn replay_analizza_confini_gradi() {
        let Some(calls) = trace::calls() else { return };
        let Some(m) = trace::module() else { return };
        let mut lexinfo = Vec::new();
        let (mut n, mut ok) = ([0usize; 3], [0usize; 3]);
        let mut shown = 0;
        for c in &calls {
            let k = match c.name.as_str() {
                "LesAnalizza" => 0,
                "LesConfiniProsodici" => 1,
                "LesGradiAccentuali" => 2,
                "LesAccenta" => {
                    lexinfo = c.after.iter().map(|w| w.lexinfo.clone()).collect();
                    continue;
                }
                _ => continue,
            };
            let mut ws: Vec<_> = c.before.iter().map(|t| t.word()).collect();
            match k {
                0 => m.analizza(&mut ws),
                1 => {
                    for (w, l) in ws.iter_mut().zip(&lexinfo) {
                        if let Some(l) = l {
                            l.store(w);
                        }
                    }
                    m.confini(&mut ws, c.ctx.get(2).copied().unwrap_or(0) as i32)
                }
                _ => m.gradi(&mut ws),
            }
            n[k] += 1;
            let same = ws.len() == c.after.len()
                && ws.iter().zip(&c.after).all(|(w, a)| w.f[0xc..0x14] == a.f[0xc..0x14] && (k != 0 || super::LexInfo::of(w).map(|l| l.head) == a.lexinfo.as_ref().map(|l| l.head)));
            if same {
                ok[k] += 1;
            } else if shown < 8 {
                shown += 1;
                eprintln!("{} mismatch:", c.name);
                for (w, a) in ws.iter().zip(&c.after) {
                    let h = |x: &[u8]| x.iter().map(|b| format!("{b:02x}")).collect::<String>();
                    eprintln!("  {:<20} ours {} {:?}  orig {} {:?}", String::from_utf8_lossy(&w.text), h(&w.f[0xc..0x14]), super::LexInfo::of(w).map(|l| l.head[1]), h(&a.f[0xc..0x14]), a.lexinfo.as_ref().map(|l| l.head[1]));
                }
            }
        }
        eprintln!("analizza {}/{} confini {}/{} gradi {}/{}", ok[0], n[0], ok[1], n[1], ok[2], n[2]);
        assert!(n[0] > 0);
        if std::env::var_os("QQ_DA_STRICT").is_some() {
            assert_eq!(ok, n);
        }
    }
}
