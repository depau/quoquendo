//! FonAllofonoLingua, EnglishUs (0x100182a0).
//!
//! The engine's phone list (`ctx+0x54`, an indexed `ELQList` whose index
//! array `ctx+0x10` LTTS7Util.dll rebuilds after every insertion or
//! deletion, FUN_100084b0) is modelled by the `Vec` itself: `AddElementAfter`
//! after `FindElement(i - 1)` is `insert(i, …)`, `DeleteElement(&arr[i])` is
//! `remove(i)`, and later reads of `arr[k]` in the same call see the rebuilt
//! array. Slots past the list's end hold the sentinel.
//!
//! First a glide is inserted between two vowels across a word boundary
//! (`Hy` 0x10ce / `y` 0x9e2 after front vowels, `w` 0x401ece after back
//! ones). Then the (possibly new) phone at `i` is rewritten by its code:
//! `n` assimilates to `N` before velars, `s`/`z` to following `S`, dark /
//! syllabic `r` and `l`, `k`/`g` unreleased, weak `the`/`to`, schwa
//! deletion into syllabic `n`/`r`/`l`, `d`/`t` unreleased, `t` flapped
//! (`HTh` 0x1982) or glottalised before `you`, aspiration of `t`/`p`/`k`.

use super::English;
use crate::lang::common::{field::GRADE, Phone, Word};

/// Vowels that end in a front glide (0x1001834a chain): `y` is inserted
/// before a following vowel of another word.
const FRONT: [u32; 10] = [0x7c1, 0x107c5, 0x781, 0x10781, 0xe445, 0x1e445, 0xe6c5, 0x1e6c5, 0xe2e5, 0x1e2e5];
/// Vowels that end in a back glide (0x100189f8): `w` is inserted.
const BACK: [u32; 8] = [0x3e1, 0x103e5, 0x3a1, 0x103a1, 0xac45, 0x1ac45, 0xab65, 0x1ab65];
/// Consonants that keep a stop released inside a word (r, l, 952, 10c2, w).
const SONORANT: [u32; 5] = [0x18c2, 0x18a2, 0x952, 0x10c2, 0x401ec2];
/// `you` and its forms (0x10027a38, 0x10042b10, 0x10042b08, 0x10042b00,
/// 0x10042af8, "yours", 0x10042af4, 0x10042af0, "yourself", "yourselves").
const YOU: [&[u8]; 10] =
    [b"you", b"your", b"you've", b"you'd", b"you'll", b"yours", b"yer", b"yeh", b"yourself", b"yourselves"];
/// `YOU` plus the `h` pronouns (0x1004254c, 0x1004347c, 0x10043478,
/// 0x1002b878, 0x10043470, 0x10043468, 0x10043460, 0x10043458).
const YOU_HE: [&[u8]; 18] = [
    b"you", b"your", b"you've", b"you'd", b"you'll", b"yours", b"yer", b"yeh", b"yourself", b"yourselves", b"he",
    b"his", b"him", b"her", b"hers", b"he'd", b"he'll", b"he's",
];

fn vowel(c: u32) -> bool {
    c & 3 == 1
}
fn cons(c: u32) -> bool {
    c & 3 == 2
}
fn stressed(c: u32) -> bool {
    c & 0x10003 == 0x10001
}
fn unstressed(c: u32) -> bool {
    c & 0x10003 == 1
}
fn is(t: &[u8], set: &[&[u8]]) -> bool {
    set.iter().any(|s| t == *s)
}
fn ends(t: &[u8], s: &[u8]) -> bool {
    t.ends_with(s)
}
fn ends_any(t: &[u8], set: &[&[u8]]) -> bool {
    set.iter().any(|s| t.ends_with(s))
}
fn contains(t: &[u8], s: &[u8]) -> bool {
    t.windows(s.len()).any(|x| x == s)
}

struct L<'a> {
    p: &'a mut Vec<Phone>,
}

impl L<'_> {
    fn at(&self, k: isize) -> Option<&Phone> {
        if k < 0 {
            None
        } else {
            self.p.get(k as usize).or(self.p.last())
        }
    }
    /// The code, 0 outside the array (the C's `k < 0x400` guard).
    fn c(&self, k: isize) -> u32 {
        if k >= 0 && (k as usize) < self.p.len() { self.p[k as usize].code } else { 0 }
    }
    fn w(&self, k: isize) -> u32 {
        self.at(k).map_or(u32::MAX, |p| p.word)
    }
    fn set(&mut self, k: isize, code: u32) {
        if let Some(p) = self.p.get_mut(k as usize) {
            p.code = code;
        }
    }
    fn del(&mut self, k: isize) {
        if (k as usize) < self.p.len() {
            self.p.remove(k as usize);
        }
    }
    /// `arr[k-1].kind = arr[k].kind`.
    fn kind_back(&mut self, k: isize) {
        if k > 0 && (k as usize) < self.p.len() {
            self.p[k as usize - 1].kind = self.p[k as usize].kind;
        }
    }
    /// Delete phone `k` handing its kind to the phone before (when `k` > 0),
    /// then give the phone now at `k` the code `code`.
    fn merge(&mut self, k: isize, code: u32) {
        if k != 0 {
            self.kind_back(k);
        }
        self.del(k);
        self.set(k, code);
    }
}

pub(super) fn allofono(en: &English, w: &[Word], first: usize, phones: &mut Vec<Phone>, i: usize) -> usize {
    let _ = en;
    let mut l = L { p: phones };
    let n = i as isize;
    let ret = i;
    let cw = l.w(n);
    // chunk+4: the chunk's record count, up to its first punctuation record
    let count = w[first..].iter().position(|r| r.f[GRADE] == 0x0f).map_or(w.len() - first, |k| k + 1);
    let rec = |k: u32| w.get(first + k as usize).map_or(&b""[..], |r| &r.text[..]);
    let t = rec(cw);
    let pt: &[u8] = if cw == 0 { b"" } else { rec(cw - 1) };
    let nt: &[u8] = if cw as usize == count { b"" } else { rec(cw + 1) };
    // `*(ctx+0x20) - 2` (the sentence's records) and `*(chunk+4) - 2`
    let before_last = cw as usize + 2 == w.len() || cw as usize + 2 == count;

    // glides between vowels of two words (0x100182a0 … 0x10018b7b)
    if i != 0 && l.w(n - 1) != cw {
        let pc = l.c(n - 1);
        let c = l.c(n);
        let are_or = is(t, &[b"are", b"or"]);
        let glide = if FRONT.contains(&pc) && unstressed(c) && c != 0x501 {
            Some(0x10ce)
        } else if FRONT.contains(&pc) && c == 0x501 && !are_or {
            Some(0x10ce)
        } else if FRONT.contains(&pc) && stressed(c) {
            Some(0x9e2)
        } else if pt == b"and" && stressed(c) {
            Some(0x9e2)
        } else if stressed(c) && is(pt, &[b"to", b"into", b"onto"]) && !is(t, &[b"it", b"its", b"us"]) {
            Some(0x9e2)
        } else if BACK.contains(&pc) && vowel(c) && !are_or {
            Some(0x401ece)
        } else {
            None
        };
        if let Some(code) = glide {
            l.p.insert(i, Phone { word: cw, code, kind: 0x69 });
        }
    }

    // LAB_10018b83: by the code now at i
    let code = l.c(n);
    let nx = l.c(n + 1);
    let pv = l.c(n - 1);
    let wd_next = l.w(n) != l.w(n + 1);
    match code {
        0x19c2 => {
            if wd_next && matches!(nx, 0xfe2 | 0x2ff2 | 0xff2) {
                l.set(n, 0xfc2);
            }
            ret
        }
        0x1952 => {
            if wd_next && nx == 0x1752 {
                l.set(n, 0x1752);
            }
            ret
        }
        0x1942 => {
            if wd_next && nx == 0x1752 {
                l.set(n, 0x1752);
            } else if wd_next && nx == 0x1952 {
                l.set(n, 0x1952);
            }
            ret
        }
        0x18f2 => {
            l.set(n, 0x18a2);
            ret
        }
        0x18c2 => r_rule(&mut l, n, t),
        0x18a2 => l_rule(&mut l, n),
        0x10c2 => {
            if t == b"your" && nx == 0x501 && l.c(n + 2) == 0x18c2 && vowel(l.c(n + 3)) {
                l.set(n, 0x10c2);
                l.set(n + 1, 0x3a1);
                l.set(n + 2, 0x18c2);
                return i + 2;
            }
            ret
        }
        0xff2 | 0x2ff2 => velar(&mut l, n, t, code),
        0xfe2 | 0x1fe2 => {
            let to = if code == 0xfe2 { 0x4fe2 } else { 0x5fe2 };
            if i != 0
                && l.w(n) == l.w(n - 1)
                && !wd_next
                && cons(nx)
                && !SONORANT.contains(&nx)
            {
                l.set(n, to);
            } else if wd_next && cons(nx) && nx != 0x952 {
                l.set(n, to);
            }
            ret
        }
        0x3e1 => {
            if wd_next && cons(nx) && nx != 0x401ec2 && is(t, &[b"to", b"into", b"onto"]) {
                l.set(n, 0x501);
            }
            ret
        }
        0x7c1 => {
            if t == b"the" && cons(nx) && nx != 0x10c2 {
                l.set(n, 0x501);
            }
            ret
        }
        0x501 => schwa(&mut l, n, t, pv, before_last),
        0x19e2 => d_rule(&mut l, n, t, nt),
        0x10501 => {
            if nx == 0x18c2 {
                l.merge(n, 0x2104c5);
            } else {
                l.set(n, 0x501);
            }
            ret
        }
        0xe445 => {
            if t == b"while" && before_last {
                l.set(n, 0x1e445);
            }
            ret
        }
        0x3ff2 | 0x1ff2 => labial(&mut l, n, t, code),
        0x419e2 => {
            l.set(n, 0x1982);
            ret
        }
        0x200501 => {
            l.set(n, 0x218c2);
            ret
        }
        0x1d42 => {
            if wd_next && nx == 0x1d52 {
                l.set(n, 0x1d52);
            }
            ret
        }
        0x19f2 | 0x39f2 => t_rule(&mut l, n, t, pt, nt),
        _ => ret,
    }
}

/// `r` 0x18c2 (0x10018d00…0x1001a272): syllabic/dark `r` 0x218c2.
fn r_rule(l: &mut L, n: isize, t: &[u8]) -> usize {
    const RV: [u32; 5] = [0x10781, 0x103a1, 0x106c1, 0x1e445, 0x1ac45];
    let i = n as usize;
    if i != 0 {
        let (pv, nx) = (l.c(n - 1), l.c(n + 1));
        let hit = (l.w(n - 1) == l.w(n + 1) && cons(nx) && RV.contains(&pv))
            || (l.w(n) != l.w(n + 1) && RV.contains(&pv))
            || (l.w(n - 1) == l.w(n + 1) && nx == 0x501 && matches!(pv, 0x1e445 | 0x1ac45))
            || (matches!(pv, 0x1e445 | 0x1ac45)
                && nx == 0x781
                && l.c(n + 2) == 0xfc2
                && l.w(n - 1) == l.w(n + 2)
                && l.w(n + 2) != l.w(n + 3));
        if hit {
            l.set(n, 0x218c2);
            return i;
        }
    }
    if is(t, &[b"carefully", b"wirehair", b"wirehairs"]) || t.starts_with(b"environ") {
        l.set(n, 0x218c2);
    }
    i
}

/// `l` 0x18a2 (0x1001a2xx): dark `l` 0x218a2.
fn l_rule(l: &mut L, n: isize) -> usize {
    const LV: [u32; 7] = [0x1e2e5, 0x1e445, 0xe445, 0x1e6c5, 0xe6c5, 0x7c1, 0x107c5];
    if n == 0 {
        return 0;
    }
    let (pv, nx) = (l.c(n - 1), l.c(n + 1));
    if l.w(n - 1) == l.w(n + 1) && cons(nx) && nx != 0x10c2 && LV.contains(&pv) {
        l.set(n, 0x218a2);
        return n as usize;
    }
    if l.w(n) == l.w(n + 1) || !LV.contains(&pv) {
        if !cons(pv) || pv == 0x18c2 {
            return n as usize;
        }
        if vowel(nx) && !(nx == 0x781 && l.c(n + 2) == 0xfc2 && l.w(n + 2) != l.w(n + 3)) {
            return n as usize;
        }
    }
    l.set(n, 0x218a2);
    n as usize
}

/// `k`/`g` 0xff2 / 0x2ff2 (LAB_1001bb2f).
fn velar(l: &mut L, n: isize, t: &[u8], code: u32) -> usize {
    let i = n as usize;
    let nx = l.c(n + 1);
    let same_next = l.w(n) == l.w(n + 1);
    if i != 0 && l.w(n) == l.w(n - 1) && same_next && cons(nx) && !SONORANT.contains(&nx) {
        l.set(n, 0x4ff2);
        return i;
    }
    if !same_next && !matches!(nx, 0x17 | 0x13) {
        l.set(n, 0x4ff2);
        return i;
    }
    if code == 0x2ff2 {
        if i == 0 {
            return 0;
        }
        if l.w(n - 1) == l.w(n) && l.c(n - 1) == 0x1952 && same_next && nx != 0x952 {
            l.set(n, 0xff2);
        }
        return i;
    }
    if (i == 0 || l.w(n - 1) != l.w(n)) && same_next {
        l.set(n, 0x2ff2);
        return i;
    }
    if i == 0 {
        return 0;
    }
    if matches!(nx, 0x17 | 0x13) {
        l.set(n, 0x2ff2);
        return i;
    }
    if l.c(n - 1) == 0x1952
        && !(t == b"excalibur" || t.starts_with(b"exclud") || is(t, &[b"excursus", b"excursuses"]))
    {
        return i;
    }
    // LAB_1001be31
    if l.w(n - 1) != l.w(n) || !same_next {
        return i;
    }
    if cons(nx) && !SONORANT.contains(&nx) && !(nx == 0x19c2 && l.w(n + 1) != l.w(n + 2)) {
        if nx != 0x19c2 || vowel(l.c(n + 2)) {
            return i;
        }
    }
    l.set(n, 0x2ff2);
    i
}

/// `p`/`b` 0x3ff2 / 0x1ff2 (LAB_1001f119).
fn labial(l: &mut L, n: isize, t: &[u8], code: u32) -> usize {
    let i = n as usize;
    let nx = l.c(n + 1);
    let same_next = l.w(n) == l.w(n + 1);
    if i != 0 && l.w(n) == l.w(n - 1) && same_next && cons(nx) && !SONORANT.contains(&nx) {
        l.set(n, 0x5ff2);
        return i;
    }
    if !same_next && !matches!(nx, 0x17 | 0x13) {
        l.set(n, 0x5ff2);
        return i;
    }
    if t.len() > 5 && t.starts_with(b"uprigh") && (i == 1 || (i > 1 && l.w(n - 2) != l.w(n - 1))) {
        l.set(n, 0x5ff2);
        return i;
    }
    if code == 0x1ff2 {
        if i != 0 && l.w(n - 1) == l.w(n) && !matches!(nx, 0x17 | 0x13) && (l.c(n - 1) == 0x1952 || !same_next) {
            return i;
        }
        l.set(n, 0x3ff2);
        return i;
    }
    if i == 0 {
        return 0;
    }
    if l.w(n - 1) == l.w(n) && l.c(n - 1) == 0x1952 && same_next && nx != 0x952 {
        l.set(n, 0x1ff2);
    }
    i
}

/// Schwa 0x501 (0x10018cxx…0x100199ef): deleted into syllabic `n`, `r`, `l`.
fn schwa(l: &mut L, n: isize, t: &[u8], pv: u32, before_last: bool) -> usize {
    let i = n as usize;
    if i != 0 {
        let nl = l.c(n + 1) == 0x19c2 && l.c(n + 2) == 0x501 && l.c(n + 3) == 0x18a2 && l.w(n - 1) == l.w(n + 3);
        if nl && matches!(pv, 0x1742 | 0x1702 | 0x1952 | 0x1942 | 0x19e2 | 0x39f2 | 0x19f2) {
            l.kind_back(n);
            l.del(n);
            l.set(n, 0x219c2);
            return i;
        }
        if nl
            && pv == 0x1752
            && contains(t, b"tional")
            && !before_last
            && !contains(t, b"ntional")
            && !is(t, &[b"organisational", b"organizational"])
        {
            l.kind_back(n);
            l.del(n);
            l.set(n, 0x219c2);
            return i;
        }
    }
    let wd = |l: &L, k: isize| l.w(n + k);
    if l.c(n + 1) == 0x18c2 {
        let mut del = is(t, &[b"separate", b"catherine", b"catherine's"]);
        if !del && cons(l.c(n + 2)) && wd(l, 0) == wd(l, 2) && t != b"binoculars" && !ends(t, b"ellers") {
            del = true;
        }
        if del {
            l.merge(n, 0x218c2);
            return i;
        }
    }
    // LAB_10019121
    if l.c(n + 1) == 0x18c2 && wd(l, 1) != wd(l, 2) && !is(t, &[b"or", b"her", b"are", b"for"]) {
        if !ends(t, b"eller") && (i < 2 || l.c(n - 1) != 0x218c2) {
            l.merge(n, 0x218c2);
            return i;
        }
    }
    if l.c(n + 1) == 0x18c2
        && l.c(n + 2) == 0x501
        && wd(l, 0) == wd(l, 3)
        && wd(l, 3) != wd(l, 4)
        && matches!(l.c(n + 3), 0x18a2 | 0x18c2)
    {
        l.merge(n, 0x218c2);
        return i;
    }
    if l.c(n + 1) == 0x18c2 && is(t, &[b"comparable", b"corporate", b"her"]) {
        l.merge(n, 0x218c2);
        return i;
    }
    // LAB_1001946a
    if t == b"are" && (i < 2 || l.c(n - 1) != 0x218c2) {
        l.merge(n, 0x218c2);
        return i;
    }
    if is(t, &[b"usually", b"unusually", b"applejack"]) && l.c(n + 1) == 0x18a2 {
        l.merge(n, 0x218a2);
        return i;
    }
    // LAB_100195e5
    let not_rl = |l: &L| i < 2 || !matches!(l.c(n - 1), 0x219c2 | 0x218c2);
    if l.c(n + 1) == 0x18a2 && wd(l, 1) != wd(l, 2) && !is(t, &[b"lonsdale", b"schnitzel"]) && not_rl(l) {
        l.merge(n, 0x218a2);
        return i;
    }
    if l.c(n + 1) == 0x18a2 {
        let x = l.c(n + 2);
        let a = matches!(x, 0x39f2 | 0x19f2 | 0x19e2 | 0x1952 | 0x1942)
            && n >= 2
            && wd(l, -2) == wd(l, 2)
            && wd(l, 2) != wd(l, 3);
        let b = x == 0x781 && l.c(n + 3) == 0xfc2 && wd(l, -2) == wd(l, 3) && wd(l, 3) != wd(l, 4);
        if (a || b) && not_rl(l) {
            l.merge(n, 0x218a2);
            return i;
        }
    }
    // LAB_10019853
    if t == b"frantically" {
        l.merge(n, 0x218a2);
        return i;
    }
    if i == 0 {
        return 0;
    }
    if matches!(l.c(n - 1), 0x2ff2 | 0xff2)
        && l.c(n + 1) == 0x18a2
        && l.c(n + 2) == 0x7c1
        && wd(l, -1) == wd(l, 2)
        && ends(t, b"ically")
    {
        l.kind_back(n);
        l.del(n);
    }
    i
}

/// `d` 0x19e2 (0x10019xxx…LAB_1001b975).
fn d_rule(l: &mut L, n: isize, t: &[u8], nt: &[u8]) -> usize {
    let i = n as usize;
    let nx = l.c(n + 1);
    let pv = l.c(n - 1);
    let wd_next = l.w(n) != l.w(n + 1);
    let mut syl = false;
    if nx == 0x501
        && l.c(n + 2) == 0x19c2
        && i != 0
        && l.w(n - 1) == l.w(n + 2)
        && (stressed(pv) || pv == 0x18c2)
        && !is(t, &[b"couldn't", b"wouldn't", b"shouldn't"])
    {
        syl = true;
    }
    if !syl
        && nx == 0x501
        && l.c(n + 2) == 0x19c2
        && l.c(n + 3) == 0x501
        && l.c(n + 4) == 0x18a2
        && l.w(n) == l.w(n + 4)
    {
        syl = true;
    }
    if syl {
        // LAB_1001a56e
        l.set(n, 0x59e2);
        l.p[i].kind = l.p[i + 1].kind;
        l.del(n + 1);
        l.set(n + 1, 0x219c2);
        if let Some(word) = l.p.get(i).map(|p| p.word) {
            if let Some(p) = l.p.get_mut(i + 1) {
                p.word = word;
            }
        }
        return i + 1;
    }
    if i != 0 && pv == 0x19c2 && wd_next && matches!(nx, 0x1fc2 | 0x19c2 | 0x19e2 | 0x1b42 | 0x1b52 | 0x39f2 | 0x19f2) {
        l.kind_back(n);
        l.del(n);
        return i.wrapping_sub(1);
    }
    if t == b"and" && matches!(nx, 0x18a2 | 0x1fe2) {
        if i != 0 {
            l.kind_back(n);
        }
        l.del(n);
        return i;
    }
    if i != 0 {
        if pv == 0x1942 && wd_next && nx == 0x1b42 {
            l.kind_back(n);
            l.del(n);
            return i;
        }
        if pv == 0x1702 && wd_next && matches!(nx, 0x39f2 | 0x19f2) {
            l.kind_back(n);
            l.del(n);
            return i;
        }
    }
    // LAB_1001a818
    if cons(nx) && i != 0 && l.w(n) == l.w(n - 1) && !wd_next && !SONORANT.contains(&nx) {
        l.set(n, 0x59e2);
        return i;
    }
    if t == b"and" && wd_next && stressed(nx) && nt != b"i" {
        l.set(n, 0x59e2);
        return i;
    }
    if t == b"and"
        && cons(nx)
        && !matches!(nx, 0x18a2 | 0x1fe2 | 0x1fc2 | 0x19c2 | 0x19e2 | 0x1b42 | 0x39f2 | 0x19f2)
        && !is(nt, &YOU_HE)
    {
        l.set(n, 0x59e2);
        return i;
    }
    if i == 0 {
        return 0;
    }
    if wd_next
        && pv == 0x19c2
        && cons(nx)
        && t != b"and"
        && !matches!(nx, 0x1fc2 | 0x19c2 | 0x19e2 | 0x1b42 | 0x39f2 | 0x19f2 | 0x952)
        && !is(nt, &YOU)
    {
        l.set(n, 0x59e2);
        return i;
    }
    // LAB_1001b1d5
    if !matches!(pv, 0x1942 | 0x19c2 | 0x1702) && wd_next && cons(nx) && nx != 0x952 && !is(nt, &YOU) {
        l.set(n, 0x59e2);
        return i;
    }
    if pv == 0x1942
        && wd_next
        && cons(nx)
        && !matches!(nx, 0x952 | 0x1fc2 | 0x19c2 | 0x19e2 | 0x1b42 | 0x1952 | 0x39f2 | 0x19f2 | 0x1fe2 | 0xfe2)
        && !is(nt, &YOU)
    {
        l.set(n, 0x59e2);
        return i;
    }
    if pv == 0x19c2 && wd_next && nt == b"to" && matches!(nx, 0x39f2 | 0x19f2) {
        l.set(n, 0x59e2);
        return i;
    }
    if wd_next && pv == 0x1942 && matches!(nx, 0x1fc2 | 0x19c2 | 0x19e2 | 0x1952 | 0x39f2 | 0x19f2 | 0x1fe2 | 0xfe2) {
        // LAB_1001b975 stores 0x59f2
        l.set(n, 0x59f2);
    }
    i
}

/// `t` 0x19f2 / `th` 0x39f2 (0x1001bxxx…end): unreleased, flapped
/// (`HTh` 0x1982), glottal before `you`, aspiration.
fn t_rule(l: &mut L, n: isize, t: &[u8], pt: &[u8], nt: &[u8]) -> usize {
    let i = n as usize;
    let len = t.len();
    let nx = l.c(n + 1);
    let w = |l: &L, k: isize| l.w(n + k);
    let flap = |l: &mut L| {
        l.set(n, 0x1982);
        i
    };
    // syllabic n after the t
    if nx == 0x501
        && l.c(n + 2) == 0x19c2
        && i != 0
        && (stressed(l.c(n - 1)) || l.c(n - 1) == 0x18c2)
        && w(l, -1) == w(l, 2)
        || nx == 0x501 && l.c(n + 2) == 0x19c2 && l.c(n + 3) == 0x501 && l.c(n + 4) == 0x18a2 && w(l, 0) == w(l, 4)
    {
        l.set(n, 0x59f2);
        l.p[i].kind = l.p[i + 1].kind;
        l.del(n + 1);
        l.set(n + 1, 0x219c2);
        return i + 1;
    }
    if nx == 0x1952 && contains(t, b"tz") {
        l.merge(n, 0x1912);
        return i;
    }
    if nx == 0x1952 {
        if i == 0 {
            l.del(n);
            l.set(n, 0x1912);
            return i;
        }
        if w(l, -1) != w(l, 0) {
            l.merge(n, 0x1912);
            return i;
        }
        if w(l, 1) != w(l, 2) || matches!(l.c(n + 2), 0x17 | 0x13) {
            l.merge(n, 0x1912);
            return i;
        }
    }
    if i != 0 && l.c(n - 1) == 0x1752 && w(l, 0) != w(l, 1) && nx == 0x1752 {
        l.kind_back(n);
        l.del(n);
        return i;
    }
    if t == b"must" && w(l, 0) != w(l, 1) && cons(nx) && !is(nt, &[b"have", b"has", b"had"]) {
        if i != 0 {
            l.kind_back(n);
        }
        l.del(n);
        return i;
    }
    if is(t, &[b"incidentally", b"satisfaction", b"satisfactions", b"inevitable", b"headquarter", b"headquarters"]) {
        return flap(l);
    }
    if (t.starts_with(b"meteo") || t.starts_with(b"alternativ") || t.starts_with(b"iter"))
        && matches!(nx, 0x7c1 | 0x781 | 0x501)
    {
        return flap(l);
    }
    if i != 0
        && stressed(l.c(n - 1))
        && unstressed(nx)
        && w(l, -1) == w(l, 1)
        && !ends(t, b"time")
        && !ends(t, b"times")
        && !t.starts_with(b"autops")
        && !is(t, &[b"mutated", b"contact"])
    {
        return flap(l);
    }
    if i > 1 {
        let pp = l.c(n - 2);
        if stressed(pp)
            && l.c(n - 1) == 0x19c2
            && unstressed(nx)
            && w(l, -1) == w(l, 1)
            && !is(
                t,
                &[b"enter", b"entering", b"entered", b"enters", b"contact", b"contacting", b"contacted", b"contacts"],
            )
            && !ends(t, b"ntime")
        {
            return flap(l);
        }
        if stressed(pp) && l.c(n - 1) == 0x18c2 && unstressed(nx) && w(l, -1) == w(l, 1) {
            return flap(l);
        }
    }
    if i != 0 {
        let pv = l.c(n - 1);
        let c2 = l.c(n + 2);
        let c3 = l.c(n + 3);
        let end2 = w(l, 2) != w(l, 3);
        let end3 = w(l, 3) != w(l, 4);
        let ed = matches!(nx, 0x501 | 0x781) && c2 == 0x19e2 && end2;
        let ing = nx == 0x781 && c2 == 0xfc2 && end2;
        let ive = nx == 0x781 && c2 == 0x1d42 && end2;
        let ives = nx == 0x781 && c2 == 0x1d42 && c3 == 0x1942 && end3;
        let hits = [
            unstressed(pv) && ends_any(t, &[b"ated", b"eted", b"ited", b"oted", b"uted"]) && ed,
            pv == 0x19c2
                && ends(t, b"nted")
                && !ends(t, b"printed")
                && !ends(t, b"presented")
                && !ends(t, b"lamented")
                && ed,
            pv == 0x18c2 && ends(t, b"rted") && ed,
            unstressed(pv) && ends_any(t, &[b"ating", b"eting", b"iting", b"oting", b"uting"]) && ing,
            pv == 0x19c2 && ends(t, b"nting") && ing,
            pv == 0x18c2 && ends(t, b"rting") && ing,
            unstressed(pv) && ends_any(t, &[b"ative", b"etive", b"itive", b"otive", b"utive"]) && ive,
            pv == 0x19c2 && ends(t, b"ntive") && ive,
            pv == 0x18c2 && ends(t, b"rtive") && ive,
            unstressed(pv) && ends_any(t, &[b"atives", b"etives", b"otives", b"utives"]) && ives,
            pv == 0x19c2 && ends(t, b"ntives") && ives,
            pv == 0x18c2 && ends(t, b"rtives") && ives,
            ends_any(t, &[b"eter", b"etre", b"ater", b"atre", b"ator", b"etor", b"itor", b"utor"])
                && nx == 0x501
                && c2 == 0x18c2
                && end2,
            ends_any(t, &[b"eters", b"etres", b"aters", b"atres", b"ators", b"etors", b"itors", b"utors"])
                && nx == 0x501
                && c2 == 0x18c2
                && c3 == 0x1942
                && end3,
            ends_any(t, &[b"ity", b"iety", b"erty"]) && nx == 0x7c1 && w(l, 1) != w(l, 2),
            ends_any(t, &[b"ities", b"ieties", b"erties"]) && nx == 0x7c1 && c2 == 0x1942 && end2,
            ends(t, b"itous") && nx == 0x501 && c2 == 0x1952 && end2,
            ends(t, b"ital") && nx == 0x501 && c2 == 0x18a2 && end2,
            ends(t, b"italism")
                && nx == 0x501
                && c2 == 0x18a2
                && c3 == 0x781
                && l.c(n + 4) == 0x1942
                && l.c(n + 5) == 0x501
                && l.c(n + 6) == 0x1fc2
                && w(l, 6) != w(l, 7),
        ];
        if hits.iter().any(|&h| h) {
            return flap(l);
        }
    }
    let inter = i >= 2 && l.c(n - 2) == 0x781 && l.c(n - 1) == 0x19c2;
    if inter && nx == 0x501 && l.c(n + 2) == 0x18c2 && t.starts_with(b"inter") {
        return flap(l);
    }
    // LAB_1001db45
    if nx == 0x501
        && l.c(n + 2) == 0x1fe2
        && l.c(n + 3) == 0x10781
        && l.c(n + 4) == 0x18a2
        && l.c(n + 5) == 0x781
        && matches!(l.c(n + 6), 0x39f2 | 0x19f2)
        && l.c(n + 7) == 0x7c1
        && ends(t, b"tability")
    {
        return flap(l);
    }
    if nx == 0x501
        && l.c(n + 2) == 0x18a2
        && l.c(n + 3) == 0x1e6c5
        && l.c(n + 4) == 0x1752
        && l.c(n + 5) == 0x501
        && l.c(n + 6) == 0x19c2
        && contains(t, b"tilation")
    {
        return flap(l);
    }
    if i > 2
        && l.c(n - 3) == 0x952
        && l.c(n - 2) == 0x245
        && l.c(n - 1) == 0x18c2
        && contains(t, b"heart")
        && w(l, 0) == w(l, 1)
    {
        return flap(l);
    }
    let wd_next = w(l, 0) != w(l, 1);
    if i != 0 {
        let pv = l.c(n - 1);
        if pv == 0x19c2 && w(l, -1) == w(l, 0) && t == b"seventy" {
            return flap(l);
        }
        if vowel(pv) && wd_next && vowel(nx) && t != b"dot" {
            return flap(l);
        }
        if pv == 0x18c2 && wd_next && (unstressed(nx) || nx == 0x952) {
            return flap(l);
        }
        if matches!(pv, 0x19c2 | 0x219c2) && wd_next && matches!(nx, 0x501 | 0x781) {
            return flap(l);
        }
        if vowel(pv) && wd_next && nx == 0x952 {
            return flap(l);
        }
        if pt == b"and" && t == b"to" {
            return flap(l);
        }
    }
    const MIGHT: [&[u8]; 6] = [b"might", b"that", b"put", b"get", b"got", b"expect"];
    let might = is(t, &MIGHT);
    if wd_next && !might && is(nt, &YOU) {
        l.set(n, 0x1712);
        return i;
    }
    let unrel = |l: &mut L| {
        l.set(n, 0x59f2);
        i
    };
    if cons(nx) && i != 0 && w(l, 0) == w(l, -1) && !matches!(nx, 0x18c2 | 0x18a2 | 0x952 | 0x401ec2) && !wd_next {
        return unrel(l);
    }
    if might && is(nt, &YOU) {
        return unrel(l);
    }
    if i != 0 {
        let pv = l.c(n - 1);
        if pv != 0x1952 && wd_next && cons(nx) && !is(nt, &YOU) {
            return unrel(l);
        }
        if pv == 0x1952 && wd_next && cons(nx) && !matches!(nx, 0x10c2 | 0x401ec2 | 0x952) {
            return unrel(l);
        }
        if cons(pv) && !matches!(pv, 0x19c2 | 0x18c2) && wd_next && (vowel(nx) || matches!(nx, 0x10c2 | 0x401ec2)) {
            return unrel(l);
        }
        if cons(pv) && wd_next && nx == 0x952 {
            return unrel(l);
        }
        if pv == 0x19c2 && wd_next && vowel(nx) && !matches!(nx, 0x501 | 0x781) {
            return unrel(l);
        }
    }
    if t == b"shetland" {
        return unrel(l);
    }
    if len > 2 && t.starts_with(b"out") && l.c(n - 1) == 0x1ac45 {
        return unrel(l);
    }
    if i != 0 {
        let pv = l.c(n - 1);
        if (vowel(pv) || matches!(pv, 0x19c2 | 0x18c2))
            && w(l, -1) == w(l, 0)
            && (ends(t, b"ly") || ends(t, b"ely"))
            && w(l, 2) != w(l, 3)
        {
            return unrel(l);
        }
    }
    let code = l.c(n);
    if code != 0x39f2 {
        if code != 0x19f2 {
            return i;
        }
        if i != 0 && w(l, -1) == w(l, 0) && !matches!(nx, 0x17 | 0x13) {
            if nx == 0x952 {
                l.set(n, 0x39f2);
                return i;
            }
            if i > 1 && l.c(n - 2) == 0x4ff2 && l.c(n - 1) == 0x1952 && nx == 0x107c5 {
                l.set(n, 0x39f2);
                return i;
            }
            if l.c(n - 1) == 0x1952 {
                return i;
            }
        }
        l.set(n, 0x39f2);
        return i;
    }
    if i != 0
        && w(l, -1) == w(l, 0)
        && l.c(n - 1) == 0x1952
        && w(l, 0) == w(l, 1)
        && nx != 0x952
        && !is(t, &[b"sixteen", b"sixteenth"])
    {
        l.set(n, 0x19f2);
        return i;
    }
    if i < 2
        || w(l, -1) != w(l, 0)
        || l.c(n - 1) != 0x1952
        || w(l, -2) != w(l, -1)
        || w(l, 0) != w(l, 1)
        || matches!(nx, 0x952 | 0x18a2)
        || (cons(l.c(n - 2)) && vowel(nx))
    {
        return i;
    }
    l.set(n, 0x19f2);
    i
}
