//! The engine's text rewriting: LTTS7Util.dll's text filters
//! (`ELQTextFilterApply` 0x1003a470 and what it calls), run by LoqTTS7.dll's
//! LesRun (0x10035b20 → 0x100356e0) over the whole text before the sentence
//! loop. The text becomes a raw word list (`ELQRawWordListAddText`
//! 0x1001bdb0), every `.lde` `RegExp` file and then every `Lexicon` file
//! rewrites it (each list last first), the list is compacted
//! (`ELQRawWordListCompact` 0x1001d7c0) and joined back into text
//! (`ELQRawWordListDetokenize` 0x1001d0e0), which LesRun hands the sentence
//! loop (0x10034110). Regular expressions are LTTS7Util's wide-character
//! port of Henry Spencer's (`ELQRxc*`, 0x1001e490–0x1001fcf0); its
//! character classes are read from LTTS7Util.dll's bytes (MECHANICS.md
//! §4f). Nothing of the DLL runs.

use super::common::Pe;

// --- LTTS7Util's character classes ------------------------------------------

/// LTTS7Util.dll's Unicode tables (sorted UTF-16 code units, searched by
/// bisection): `ELQwisdigit` 0x10041310 (table 0x10058ab0, 0xe9 entries,
/// past ASCII digits from 0xa0), `ELQwisspace` 0x10041350 (0x10058c84,
/// 0x19, past ` \t\n\r\v\b\f` from 0xa0), `ELQwispunct` 0x100413b0
/// (0x10058cb8, 0x1dc), `ELQwiscntrl` 0x10041400 (0x10059070, 0x61) and
/// `ELQwCaseChange` 0x100414b0 (to lower: pairs at 0x10059ee0, 0x369; past
/// ASCII from 0x7f).
pub struct Classes {
    digit: Vec<u16>,
    space: Vec<u16>,
    punct: Vec<u16>,
    cntrl: Vec<u16>,
    lower: Vec<(u16, u16)>,
}

fn u16s(pe: &Pe, va: u32, n: usize) -> Vec<u16> {
    (0..n).map(|i| pe.u16(va + 2 * i as u32)).collect()
}

impl Classes {
    pub fn from_util(util: &Pe) -> Option<Classes> {
        let c = Classes {
            digit: u16s(util, 0x10058ab0, 0xe9),
            space: u16s(util, 0x10058c84, 0x19),
            punct: u16s(util, 0x10058cb8, 0x1dc),
            cntrl: u16s(util, 0x10059070, 0x61),
            lower: (0..0x369).map(|i| (util.u16(0x10059ee0 + 4 * i), util.u16(0x10059ee0 + 4 * i + 2))).collect(),
        };
        // the tables start where the code says: '0'…'9', '!' and NUL
        (c.digit[0] == 0x30 && c.punct[0] == 0x21 && c.cntrl[0] == 0).then_some(c)
    }

    fn has(t: &[u16], c: char) -> bool {
        u16::try_from(c as u32).is_ok_and(|u| t.binary_search(&u).is_ok())
    }

    pub fn digit(&self, c: char) -> bool {
        c.is_ascii_digit() || (c as u32 >= 0xa0 && Self::has(&self.digit, c))
    }

    pub fn space(&self, c: char) -> bool {
        matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x08' | '\x0c') || (c as u32 >= 0xa0 && Self::has(&self.space, c))
    }

    pub fn punct(&self, c: char) -> bool {
        Self::has(&self.punct, c)
    }

    fn cntrl(&self, c: char) -> bool {
        Self::has(&self.cntrl, c)
    }

    /// `ELQwisalpha` 0x10041450: an ASCII letter, or anything that is not
    /// a space, a control, a digit or punctuation.
    pub fn alpha(&self, c: char) -> bool {
        c.is_ascii_alphabetic() || !(self.space(c) || self.cntrl(c) || self.digit(c) || self.punct(c))
    }

    pub fn lower(&self, c: char) -> char {
        let u = c as u32;
        if c.is_ascii_uppercase() {
            return c.to_ascii_lowercase();
        }
        if u < 0x7f || u > 0xffff {
            return c;
        }
        match self.lower.binary_search_by_key(&(u as u16), |p| p.0) {
            Ok(k) => char::from_u32(self.lower[k].1 as u32).unwrap_or(c),
            Err(_) => c,
        }
    }

    fn eq_nocase(&self, a: &[char], b: &[char]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(&x, &y)| self.lower(x) == self.lower(y))
    }
}

/// `ELQIsSentenceTermination` 0x10041790.
fn sentence_end(c: char) -> bool {
    matches!(c as u32, 0x2e | 0x3b | 0x3a | 0x21 | 0x3f | 0x6d4 | 0x61b | 0x61f | 0x3002 | 0xff61 | 0xff1a | 0xff1b | 0xff1f | 0xff01)
}

/// `ELQIsPhraseTermination` 0x10041820.
fn phrase_end(c: char) -> bool {
    matches!(c as u32, 0x2c | 0x28 | 0x29 | 0x2d | 0x60c | 0x3001 | 0xff64 | 0xff0c)
}

/// UTF-8 length, the unit the engine's offsets and limits count in.
fn u8len(s: &[char]) -> usize {
    s.iter().map(|c| c.len_utf8()).sum()
}

// --- Patterns: `%n`, `%w`, `%1-2n`, `%1` --------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pattern {
    /// `n w p s %`, or `0` for a reference to a matched token (`%1`)
    kind: char,
    neg: bool,
    min: usize,
    max: usize,
}

fn atoi(s: &[char]) -> usize {
    let mut n = 0usize;
    for &c in s.iter().skip_while(|c| **c == ' ') {
        match c.to_digit(10) {
            Some(d) => n = n.wrapping_mul(10).wrapping_add(d as usize),
            None => break,
        }
    }
    n
}

/// LTTS7Util 0x10016a50 (`ELQPatternCheck`): `%` then `!` (negated), an
/// optional length `a` or `a-b`, and a kind letter `w n p s`; `%%`; or a
/// reference `%<n>`. Lengths count UTF-8 bytes.
fn pattern(p: &[char]) -> Option<Pattern> {
    let mut pat = Pattern {
        kind: '\0',
        neg: false,
        min: 1,
        max: usize::MAX,
    };
    if p.first() != Some(&'%') {
        return None;
    }
    let mut s = &p[1..];
    if s == ['%'] {
        pat.kind = '%';
        pat.max = 1;
        return Some(pat);
    }
    if s.first() == Some(&'!') {
        pat.neg = true;
        s = &s[1..];
    }
    if s.is_empty() {
        return None;
    }
    let count = s.iter().position(|c| "wWnNpPsS".contains(*c)).unwrap_or(s.len());
    if count == s.len() - 1 {
        pat.kind = s[count].to_ascii_lowercase();
        if (1..=0x14).contains(&count) {
            let pre = &s[..count];
            match pre.iter().position(|&c| c == '-') {
                Some(d) => {
                    pat.min = atoi(&pre[..d]);
                    pat.max = atoi(&pre[d + 1..]);
                }
                None => {
                    pat.min = atoi(pre);
                    pat.max = pat.min;
                }
            }
            if pat.min == 0 || pat.max < pat.min {
                return None;
            }
        }
        return Some(pat);
    }
    if s.iter().all(|c| c.is_ascii_digit()) {
        let n = atoi(s);
        if n != 0 {
            pat.kind = '0';
            pat.min = n - 1;
            pat.max = 0;
            return Some(pat);
        }
    }
    None
}

/// `ELQPatternGetIndex` 0x10016e50: a reference's token index.
fn pattern_index(p: &[char]) -> Option<usize> {
    pattern(p).filter(|p| p.kind == '0').map(|p| p.min)
}

/// `ELQPatternCheckSubString` 0x10016c30: whether `c` can follow the
/// pattern's last character `last`.
fn pattern_goes_on(last: char, c: char) -> bool {
    if c == '\0' || matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x08' | '\x0c') {
        return false;
    }
    let kind_or_digit = |c: char| c.is_ascii_digit() || "wWnNpPsS".contains(c);
    match last {
        '%' => c == '!' || kind_or_digit(c) || c == '-',
        '!' | '0'..='9' => kind_or_digit(c) || c == '-',
        '-' => kind_or_digit(c),
        _ => false,
    }
}

/// `ELQPatternMatch` 0x10017060 (0x10016ee0): a key token against a text
/// token.
fn pattern_match(cl: &Classes, key: &[char], text: &[char]) -> bool {
    if key.is_empty() {
        return text.is_empty();
    }
    let Some(p) = pattern(key) else {
        return cl.eq_nocase(key, text);
    };
    let n = u8len(text);
    if n < p.min || n > p.max {
        return p.neg;
    }
    let all = |f: &dyn Fn(char) -> bool| text.iter().all(|&c| f(c));
    match p.kind {
        '%' => (text == ['%']) != p.neg,
        'n' => all(&|c| cl.digit(c)) != p.neg,
        'p' => all(&|c| sentence_end(c) || phrase_end(c)) != p.neg,
        's' => all(&|c| cl.punct(c) && !sentence_end(c) && !phrase_end(c)) != p.neg,
        'w' => all(&|c| cl.alpha(c)) != p.neg,
        _ => p.neg,
    }
}

// --- The raw word list ---------------------------------------------------------

/// Token types (field 7): punctuation, special, number, word, pattern.
mod ty {
    pub const NULL: u32 = 0;
    pub const PUNCT: u32 = 1;
    pub const SPECIAL: u32 = 2;
    pub const NUMBER: u32 = 4;
    pub const WORD: u32 = 8;
    pub const PATTERN: u32 = 0x10;
}

/// End flags (field 6, stored through the list's set handler 0x1001a950):
/// phrase 1, sentence 3, paragraph 0xf, prompt 0x1f.
fn flag_value(f: u32) -> u32 {
    match f {
        2 => 3,
        4 => 0xf,
        0x10 => 0x1f,
        f => f,
    }
}

/// One entry of a raw word list: its word, the separator after it, its
/// tags (each `\…` and a space), end flags and type.
#[derive(Clone, Debug, Default, PartialEq)]
struct RawTok {
    word: Option<Vec<char>>,
    sep: Option<Vec<char>>,
    tag: Option<Vec<char>>,
    flags: u32,
    typ: u32,
}

fn some(v: Vec<char>) -> Option<Vec<char>> {
    (!v.is_empty()).then_some(v)
}

/// `ELQTokenListAddToCurrentValue` on a string field: appended.
fn add_to(a: &mut Option<Vec<char>>, b: &Option<Vec<char>>) {
    if let Some(b) = b {
        a.get_or_insert_with(Vec::new).extend_from_slice(b);
    }
}

/// The tag names LTTS7Util's tag parser knows (0x10081820, 58 entries).
const TAG_NAMES: [&str; 58] = [
    "persona", "voice", "language", "lang", "lexicon", "number", "Nm", "Nf", "Nt", "Nh", "Nd", "Nx", "Nr", "s", "spell", "stress", "ipa", "SAMPA",
    "pause", "sayas", "speed", "pitch", "volume", "emphasis", "audio", "balance", "timbre", "reverb", "dynamic", "robot", "evil", "whisper", "book",
    "demosentence", "@", "SpeedFast", "duration", "PitchDelta", "PitchContour", "$", "item", "unit", "PartOfSpeech", "ProsodicBoundary", "classif",
    "monot", "AccentDegree", "F", "fs", "floq", "algo", "selection", "ExcludeUnit", "omissis", "NoAllophone", "style", "licensing", "substitute",
];

/// The length of the tag starting at `t[0] == '\\'`: `\`, an optional `{`,
/// a known name and its value, up to the next space (a `{…}` tag up to its
/// `}`); `None` when it is not a tag.
fn tag_at(t: &[char]) -> Option<usize> {
    let curly = t.get(1) == Some(&'{');
    let start = 1 + curly as usize;
    let rest: String = t[start..].iter().take(32).collect();
    TAG_NAMES.iter().find(|n| rest.starts_with(**n))?;
    let mut k = start;
    while k < t.len() && !t[k].is_whitespace() {
        k += 1;
        if curly && t[k - 1] == '}' {
            break;
        }
    }
    Some(k)
}

struct AddText<'a> {
    cl: &'a Classes,
    pattern: bool,
    tags: bool,
}

/// Character states of `ELQRawWordListAddText`: number 2, word 3, space 4,
/// punctuation 5, tag 6, pattern 0xc/0xd, end 0xe.
fn boundary(new: u32, old: u32) -> bool {
    new == 0xe
        || matches!(
            new << 8 | old,
            0x203 | 0x204 | 0x205 | 0x206 | 0x208 | 0x20c | 0x20d | 0x302 | 0x304 | 0x305 | 0x306 | 0x308 | 0x30c | 0x30d | 0x502..=0x506 | 0x508
                | 0x50c | 0x50d | 0xc02..=0xc06 | 0xc08 | 0xc0c | 0xc0d
        )
}

impl AddText<'_> {
    /// LTTS7Util 0x1001b870: a character's state after another.
    fn class(&self, c: char) -> u32 {
        if self.cl.digit(c) {
            2
        } else if self.cl.space(c) {
            4
        } else if c == '%' && self.pattern {
            0xc
        } else if self.cl.punct(c) {
            5
        } else {
            3
        }
    }

    /// The text as raw word list entries, all of it (the call's
    /// end-of-text flag set); `prompt`: the last entry ends the prompt.
    fn run(&self, t: &[char], prompt: bool) -> Vec<RawTok> {
        let mut out = Vec::new();
        let (mut word, mut sep) = (Vec::new(), Vec::new());
        let (mut tags_new, mut tags_due): (Vec<Vec<char>>, Vec<Vec<char>>) = (Vec::new(), Vec::new());
        let mut tag_seen = false;
        let mut state = 0u32;
        let mut i = 0usize;
        loop {
            let end = i >= t.len();
            let c = if end { '\0' } else { t[i] };
            // the tag parser sees characters in every state but XML
            if !end && self.tags && c == '\\' && state != 7 && state != 0xb {
                if let Some(n) = tag_at(&t[i..]) {
                    tags_new.push(t[i..i + n].to_vec());
                    tag_seen = true;
                    state = 6;
                    i += n;
                    continue;
                }
            }
            // a tag waits for the first word character after it
            if matches!(state, 2 | 3 | 5 | 0xc) && tags_due.is_empty() {
                tags_due = std::mem::take(&mut tags_new);
            }
            let new = if end {
                0xe
            } else {
                match state {
                    0xc if c == '%' => 0xc,
                    0xc | 0xd if pattern_goes_on(*word.last().unwrap_or(&'\0'), c) => 0xd,
                    _ => self.class(c),
                }
            };
            if boundary(new, state) && !(word.is_empty() && sep.is_empty() && tags_due.is_empty()) {
                let tag = (!tags_due.is_empty()).then(|| {
                    let mut s = Vec::new();
                    for t in tags_due.drain(..) {
                        s.extend(t);
                        s.push(' ');
                    }
                    s
                });
                if std::mem::take(&mut tag_seen) {
                    // after a tag, only the separator's first space stays
                    let mut seen = false;
                    sep.retain(|&c| {
                        let keep = c != ' ' || !seen;
                        seen |= c == ' ';
                        keep
                    });
                }
                out.push(self.entry(std::mem::take(&mut word), std::mem::take(&mut sep), tag, new == 0xe, prompt));
            }
            if end {
                // tags after the last word: a second end round (0x1001c6ee
                // hands them on) gives them an entry of their own
                if !tags_new.is_empty() {
                    let mut s = Vec::new();
                    for t in tags_new.drain(..) {
                        s.extend(t);
                        s.push(' ');
                    }
                    out.push(self.entry(Vec::new(), Vec::new(), Some(s), true, prompt));
                }
                break;
            }
            match new {
                4 => sep.push(c),
                6 => {}
                _ => word.push(c),
            }
            state = new;
            i += 1;
        }
        out
    }

    /// LTTS7Util 0x1001b3f0: the entry for a word and its separator, its
    /// type (0x1001b340) and end flags (0x1001b220).
    fn entry(&self, word: Vec<char>, sep: Vec<char>, tag: Option<Vec<char>>, end: bool, prompt: bool) -> RawTok {
        let typ = match word.first() {
            None => ty::NULL,
            Some(&c) if self.cl.digit(c) => ty::NUMBER,
            Some(_) if self.pattern && pattern(&word).is_some() => ty::PATTERN,
            Some(&c) if !self.cl.punct(c) => ty::WORD,
            Some(&c) if sentence_end(c) || phrase_end(c) => ty::PUNCT,
            Some(_) => ty::SPECIAL,
        };
        let (mut nl, mut sp) = (0, 0);
        if sep.first().is_some_and(|&c| self.cl.space(c)) {
            for &c in &sep {
                if c == '\n' {
                    nl += 1;
                } else if c == ' ' {
                    sp += 1;
                }
                if nl == 2 {
                    break;
                }
            }
        }
        let last = word.last().copied().unwrap_or('\0');
        let mut f = if sentence_end(last) {
            if nl == 0 && !end {
                if sp > 0 {
                    2
                } else {
                    0
                }
            } else {
                4
            }
        } else if phrase_end(last) && (nl > 0 || sp > 0) {
            1
        } else {
            0
        };
        if prompt && end {
            f = 0x10;
        }
        RawTok {
            word: some(word),
            sep: some(sep),
            tag,
            flags: flag_value(f),
            typ,
        }
    }
}

/// `ELQRawWordListDetokenize` 0x1001d0e0: each entry's tags (unless
/// `notags`; `{…}` ones dropped when `nomarks`), word, then `¶` after a
/// paragraph end and ` \{$` after a sentence end (unless `nomarks`, not
/// twice in a row), and its separator (a space after ` \{$` without one).
fn detokenize(list: &[RawTok], notags: bool, nomarks: bool) -> Vec<char> {
    let mut out: Vec<char> = Vec::new();
    let mut marked = false;
    for (k, e) in list.iter().enumerate() {
        if let Some(tag) = &e.tag {
            if !notags {
                for t in tag.split(|c| *c == ' ').filter(|t| !t.is_empty()) {
                    if !(nomarks && t.get(1) == Some(&'{')) {
                        out.extend_from_slice(t);
                        out.push(' ');
                    }
                }
            }
        }
        if let Some(w) = &e.word {
            out.extend_from_slice(w);
        }
        let f = e.flags;
        if f & 4 != 0 && f & 0x10 == 0 && !nomarks && (k != 0 || e.word.is_some()) && !out.is_empty() && out.last() != Some(&'¶') {
            out.push('¶');
        }
        if f & 2 != 0 && f & 0x10 == 0 && !nomarks && !marked {
            marked = true;
            out.extend(" \\{$".chars());
            match &e.sep {
                Some(s) => out.extend_from_slice(s),
                None => out.push(' '),
            }
        } else {
            marked = false;
            if let Some(s) = &e.sep {
                out.extend_from_slice(s);
            }
        }
    }
    out
}

/// `ELQRawWordListFindTextPosition` 0x1001d530: the entry holding text
/// offset `pos` of the regexp text, whether it is in its word (else its
/// separator) and the offset there. Paragraph ends not after `;.!?:` count
/// one character. Past the end: `None` (the caller's defaults stay: the
/// last entry, offset 0).
fn find_position(list: &[RawTok], pos: usize) -> Option<(usize, bool, usize)> {
    let mut acc = 0usize;
    for (k, e) in list.iter().enumerate() {
        if let Some(w) = &e.word {
            if pos < acc + w.len() {
                return Some((k, true, pos.saturating_sub(acc)));
            }
            acc += w.len();
        }
        if e.flags & 4 != 0 && !e.word.as_ref().and_then(|w| w.last()).is_some_and(|c| ";.!?:".contains(*c)) {
            acc += 1;
        }
        if let Some(s) = &e.sep {
            if pos < acc + s.len() {
                return Some((k, false, pos.saturating_sub(acc)));
            }
            acc += s.len();
        }
    }
    None
}

/// Insert `r` before entry `at` the way `ELQRawWordListCopy` (0x1001ce80,
/// arguments 0, 1, 1, 0, 0, 0) does: words, tags, separators, flags
/// without the prompt bit, types.
fn copy_before(list: &mut Vec<RawTok>, at: usize, r: Vec<RawTok>) -> usize {
    let n = r.len();
    let r = r.into_iter().map(|mut e| {
        if e.flags & 0x10 != 0 {
            e.flags -= 0x10;
        }
        e
    });
    list.splice(at..at, r);
    n
}

/// `ELQRawWordListCompact` 0x1001d7c0: an entry with neither word nor
/// separator goes (its flags to the entry before, its tags to the one
/// after); two neighbouring numbers or words with nothing between them and
/// no tag on the second become one.
fn compact(list: &mut Vec<RawTok>) {
    // the list's focus; None after deleting its head
    let mut focus: Option<usize> = if list.is_empty() { return } else { Some(0) };
    let next = |f: Option<usize>, len: usize| -> Option<usize> {
        match f {
            None => (len > 0).then_some(0),
            Some(i) => (i + 1 < len).then_some(i + 1),
        }
    };
    loop {
        let cur = focus.map(|i| list[i].clone()).unwrap_or_default();
        let mut carry = cur.flags;
        let moved;
        if cur.word.is_none() && cur.sep.is_none() {
            if carry != 0 {
                if let Some(p) = focus.and_then(|i| i.checked_sub(1)) {
                    list[p].flags |= carry;
                    carry = 0;
                }
            }
            let Some(n) = next(focus, list.len()) else { return };
            let mut tag = cur.tag.clone();
            add_to(&mut tag, &list[n].tag);
            list[n].tag = tag;
            if carry != 0 {
                list[n].flags |= carry;
            }
            match focus {
                Some(i) if n == i + 1 => {
                    list.remove(i);
                    focus = i.checked_sub(1);
                }
                // a null focus: the engine's delete of an unset element
                _ => return,
            }
            moved = true;
        } else {
            match next(focus, list.len()) {
                Some(n) => {
                    focus = Some(n);
                    moved = true;
                }
                None => moved = false,
            }
        }
        if !moved {
            return;
        }
        let Some(n) = focus else { continue };
        let ne = &list[n];
        if cur.typ == ne.typ && (cur.typ == ty::WORD || cur.typ == ty::NUMBER) && (cur.sep.is_none() || ne.word.is_none()) && ne.tag.is_none() && n > 0 {
            let ne = list.remove(n);
            let p = &mut list[n - 1];
            add_to(&mut p.word, &ne.word);
            add_to(&mut p.sep, &ne.sep);
            p.flags = ne.flags | cur.flags;
            focus = Some(n - 1);
        }
    }
}

// --- Spencer's regular expressions, as LTTS7Util's ELQRxc has them -----------

mod op {
    pub const END: u16 = 0;
    pub const BOL: u16 = 1;
    pub const EOL: u16 = 2;
    pub const ANY: u16 = 3;
    pub const ANYOF: u16 = 4;
    pub const ANYBUT: u16 = 5;
    pub const BRANCH: u16 = 6;
    pub const BACK: u16 = 7;
    pub const EXACTLY: u16 = 8;
    pub const NOTHING: u16 = 9;
    pub const STAR: u16 = 10;
    pub const PLUS: u16 = 11;
    /// `\<` `\>` `\{` `\}`
    pub const WORDA: u16 = 12;
    pub const WORDZ: u16 = 13;
    pub const WSTART: u16 = 14;
    pub const WEND: u16 = 15;
    pub const OPEN: u16 = 20;
    pub const CLOSE: u16 = 30;
}

const HASWIDTH: u32 = 1;
const SIMPLE: u32 = 2;
const SPSTART: u32 = 4;

/// A compiled expression: Spencer's program as UTF-16 units (node = op,
/// next offset low byte, high byte, operand…), `reganch` and the
/// replacement.
#[derive(Clone, Debug)]
struct Rxc {
    prog: Vec<u16>,
    anchored: bool,
    replace: Vec<char>,
}

struct Comp<'a> {
    p: &'a [u16],
    at: usize,
    prog: Vec<u16>,
    npar: u16,
}

impl Comp<'_> {
    fn cur(&self) -> u16 {
        self.p.get(self.at).copied().unwrap_or(0)
    }
    fn node(&mut self, op: u16) -> usize {
        self.prog.extend([op, 0, 0]);
        self.prog.len() - 3
    }
    fn next(&self, n: usize) -> Option<usize> {
        let off = (self.prog[n + 2] << 8 | self.prog[n + 1]) as usize;
        if off == 0 {
            None
        } else if self.prog[n] == op::BACK {
            Some(n - off)
        } else {
            Some(n + off)
        }
    }
    fn tail(&mut self, p: usize, val: usize) {
        let mut scan = p;
        while let Some(n) = self.next(scan) {
            scan = n;
        }
        let off = if self.prog[scan] == op::BACK { scan - val } else { val - scan };
        self.prog[scan + 1] = (off & 0xff) as u16;
        self.prog[scan + 2] = (off >> 8 & 0xff) as u16;
    }
    fn optail(&mut self, p: usize, val: usize) {
        if self.prog[p] == op::BRANCH {
            self.tail(p + 3, val);
        }
    }
    fn insert(&mut self, op: u16, at: usize) {
        self.prog.splice(at..at, [op, 0, 0]);
    }

    fn reg(&mut self, paren: bool, flags: &mut u32) -> Option<usize> {
        *flags = HASWIDTH;
        let mut ret = None;
        let mut parno = 0;
        if paren {
            if self.npar > 9 {
                return None;
            }
            parno = self.npar;
            self.npar += 1;
            ret = Some(self.node(op::OPEN + parno));
        }
        let mut f = 0;
        let br = self.branch(&mut f)?;
        match ret {
            Some(r) => self.tail(r, br),
            None => ret = Some(br),
        }
        let ret = ret?;
        if f & HASWIDTH == 0 {
            *flags &= !HASWIDTH;
        }
        *flags |= f & SPSTART;
        while self.cur() == '|' as u16 {
            self.at += 1;
            let br = self.branch(&mut f)?;
            self.tail(ret, br);
            if f & HASWIDTH == 0 {
                *flags &= !HASWIDTH;
            }
            *flags |= f & SPSTART;
        }
        let ender = self.node(if paren { op::CLOSE + parno } else { op::END });
        self.tail(ret, ender);
        let mut br = Some(ret);
        while let Some(b) = br {
            self.optail(b, ender);
            br = self.next(b);
        }
        if paren {
            if self.cur() != ')' as u16 {
                return None;
            }
            self.at += 1;
        } else if self.at < self.p.len() {
            return None;
        }
        Some(ret)
    }

    fn branch(&mut self, flags: &mut u32) -> Option<usize> {
        *flags = 0;
        let ret = self.node(op::BRANCH);
        let mut chain: Option<usize> = None;
        while self.at < self.p.len() && self.cur() != '|' as u16 && self.cur() != ')' as u16 {
            let mut f = 0;
            let latest = self.piece(&mut f)?;
            *flags |= f & HASWIDTH;
            match chain {
                None => *flags |= f & SPSTART,
                Some(c) => self.tail(c, latest),
            }
            chain = Some(latest);
        }
        if chain.is_none() {
            self.node(op::NOTHING);
        }
        Some(ret)
    }

    fn piece(&mut self, flags: &mut u32) -> Option<usize> {
        let mut f = 0;
        let ret = self.atom(&mut f)?;
        let o = self.cur();
        let is_mult = |c: u16| c == '*' as u16 || c == '+' as u16 || c == '?' as u16;
        if !is_mult(o) {
            *flags = f;
            return Some(ret);
        }
        if f & HASWIDTH == 0 && o != '?' as u16 {
            return None;
        }
        // LTTS7Util's regpiece (0x1001eaf0): `*` SPSTART, `+` HASWIDTH and
        // SPSTART, `?` nothing
        *flags = match char::from_u32(o as u32) {
            Some('*') => SPSTART,
            Some('+') => HASWIDTH | SPSTART,
            _ => 0,
        };
        if o == '*' as u16 && f & SIMPLE != 0 {
            self.insert(op::STAR, ret);
        } else if o == '*' as u16 {
            self.insert(op::BRANCH, ret);
            let back = self.node(op::BACK);
            self.optail(ret, back);
            self.optail(ret, ret);
            let b = self.node(op::BRANCH);
            self.tail(ret, b);
            let n = self.node(op::NOTHING);
            self.tail(ret, n);
        } else if o == '+' as u16 && f & SIMPLE != 0 {
            self.insert(op::PLUS, ret);
        } else if o == '+' as u16 {
            let next = self.node(op::BRANCH);
            self.tail(ret, next);
            let back = self.node(op::BACK);
            self.tail(back, ret);
            let b = self.node(op::BRANCH);
            self.tail(next, b);
            let n = self.node(op::NOTHING);
            self.tail(ret, n);
        } else {
            self.insert(op::BRANCH, ret);
            let b = self.node(op::BRANCH);
            self.tail(ret, b);
            let n = self.node(op::NOTHING);
            self.tail(ret, n);
            self.optail(ret, n);
        }
        self.at += 1;
        if is_mult(self.cur()) {
            return None;
        }
        Some(ret)
    }

    fn atom(&mut self, flags: &mut u32) -> Option<usize> {
        *flags = 0;
        let c = self.cur();
        self.at += 1;
        let ch = |u: u16| char::from_u32(u as u32).unwrap_or('\0');
        match ch(c) {
            '^' => Some(self.node(op::BOL)),
            '$' => Some(self.node(op::EOL)),
            '.' => {
                *flags |= HASWIDTH | SIMPLE;
                Some(self.node(op::ANY))
            }
            '[' => {
                let ret = if self.cur() == '^' as u16 {
                    self.at += 1;
                    self.node(op::ANYBUT)
                } else {
                    self.node(op::ANYOF)
                };
                if self.cur() == ']' as u16 || self.cur() == '-' as u16 {
                    let c = self.cur();
                    self.prog.push(c);
                    self.at += 1;
                }
                while self.at < self.p.len() && self.cur() != ']' as u16 {
                    let c = self.cur();
                    self.at += 1;
                    if c == '-' as u16 {
                        let e = self.cur();
                        if e == ']' as u16 || e == 0 {
                            self.prog.push('-' as u16);
                        } else {
                            let s = self.p[self.at - 2];
                            if e < s {
                                return None;
                            }
                            for x in s + 1..=e {
                                self.prog.push(x);
                            }
                            self.at += 1;
                        }
                    } else {
                        self.prog.push(c);
                    }
                }
                self.prog.push(0);
                if self.cur() != ']' as u16 {
                    return None;
                }
                self.at += 1;
                *flags |= HASWIDTH | SIMPLE;
                Some(ret)
            }
            '(' => {
                let mut f = 0;
                let ret = self.reg(true, &mut f)?;
                *flags |= f & (HASWIDTH | SPSTART);
                Some(ret)
            }
            '\0' | '|' | ')' | '?' | '+' | '*' => None,
            '\\' => {
                let e = self.cur();
                if e == 0 {
                    return None;
                }
                let o = match ch(e) {
                    '<' => op::WORDA,
                    '>' => op::WORDZ,
                    '{' => op::WSTART,
                    '}' => op::WEND,
                    _ => op::EXACTLY,
                };
                let ret = self.node(o);
                self.prog.extend([e, 0]);
                self.at += 1;
                if o == op::EXACTLY {
                    *flags |= HASWIDTH | SIMPLE;
                }
                Some(ret)
            }
            _ => {
                self.at -= 1;
                let rest = &self.p[self.at..];
                let mut len = rest.iter().position(|&c| "^$.[()|?+*\\".contains(ch(c)) || c == 0).unwrap_or(rest.len());
                if len == 0 {
                    return None;
                }
                let ender = rest.get(len).copied().unwrap_or(0);
                if len > 1 && (ender == '*' as u16 || ender == '+' as u16 || ender == '?' as u16) {
                    len -= 1;
                }
                *flags |= HASWIDTH;
                if len == 1 {
                    *flags |= SIMPLE;
                }
                let ret = self.node(op::EXACTLY);
                let lit = rest[..len].to_vec();
                self.prog.extend(lit);
                self.prog.push(0);
                self.at += len;
                Some(ret)
            }
        }
    }
}

/// `\t`, `\r` and `\n` made the characters (LTTS7Util 0x1003b6c0), before
/// both expressions are set.
fn unescape(s: &[char]) -> Vec<char> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        if s[i] == '\\' && matches!(s.get(i + 1), Some('t' | 'r' | 'n')) {
            out.push(match s[i + 1] {
                't' => '\t',
                'r' => '\r',
                _ => '\n',
            });
            i += 2;
        } else {
            out.push(s[i]);
            i += 1;
        }
    }
    out
}

impl Rxc {
    /// `ELQRxcSetSearchExp` 0x1001e530 and `ELQRxcSetReplaceExp`.
    fn new(search: &[char], replace: &[char]) -> Option<Rxc> {
        let p: Vec<u16> = unescape(search).iter().map(|&c| c as u32 as u16).collect();
        let mut c = Comp {
            p: &p,
            at: 0,
            prog: Vec::new(),
            npar: 1,
        };
        let mut f = 0;
        c.reg(false, &mut f)?;
        let prog = c.prog;
        // one top-level choice starting with BOL anchors it
        let first_next = {
            let off = (prog[2] << 8 | prog[1]) as usize;
            prog.get(off).copied()
        };
        let anchored = first_next == Some(op::END) && prog.get(3) == Some(&op::BOL);
        Some(Rxc {
            prog,
            anchored,
            replace: unescape(replace),
        })
    }
}

struct Exec<'a> {
    prog: &'a [u16],
    s: &'a [u16],
    /// where the searched string starts (BOL, word starts look no further back)
    bol: usize,
    input: usize,
    startp: [Option<usize>; 10],
    endp: [Option<usize>; 10],
    cl: &'a Classes,
}

impl Exec<'_> {
    fn ch(&self, i: usize) -> u16 {
        self.s.get(i).copied().unwrap_or(0)
    }
    fn alpha(&self, u: u16) -> bool {
        u != 0 && self.cl.alpha(char::from_u32(u as u32).unwrap_or('\0'))
    }
    fn next(&self, n: usize) -> Option<usize> {
        let off = (self.prog[n + 2] << 8 | self.prog[n + 1]) as usize;
        if off == 0 {
            None
        } else if self.prog[n] == op::BACK {
            Some(n - off)
        } else {
            Some(n + off)
        }
    }
    fn operand(&self, n: usize) -> &[u16] {
        let o = &self.prog[n + 3..];
        &o[..o.iter().position(|&c| c == 0).unwrap_or(o.len())]
    }

    /// regrepeat (0x1001f290): `[…]` and `[^…]` both count the run of
    /// characters in the set (`wcsspn`).
    fn repeat(&self, n: usize) -> usize {
        let rest = &self.s[self.input.min(self.s.len())..];
        match self.prog[n] {
            op::ANY => rest.len(),
            op::ANYOF | op::ANYBUT => {
                let set = self.operand(n);
                rest.iter().take_while(|c| set.contains(c)).count()
            }
            op::EXACTLY => {
                let c = self.prog[n + 3];
                rest.iter().take_while(|&&x| x == c).count()
            }
            _ => 0,
        }
    }

    fn word_char(&self, u: u16) -> bool {
        self.alpha(u) || (u as u32) < 0x80 && (u as u8).is_ascii_digit() || u == '_' as u16
    }

    /// regmatch (0x1001f300).
    fn matches(&mut self, mut scan: usize) -> bool {
        loop {
            let next = self.next(scan);
            let o = self.prog[scan];
            match o {
                op::END => return true,
                op::BOL => {
                    if self.input != self.bol {
                        return false;
                    }
                }
                op::EOL => {
                    if self.ch(self.input) != 0 {
                        return false;
                    }
                }
                op::ANY => {
                    if self.ch(self.input) == 0 {
                        return false;
                    }
                    self.input += 1;
                }
                op::ANYOF | op::ANYBUT => {
                    let c = self.ch(self.input);
                    if c == 0 || self.operand(scan).contains(&c) != (o == op::ANYOF) {
                        return false;
                    }
                    self.input += 1;
                }
                op::EXACTLY => {
                    let lit = self.operand(scan).to_vec();
                    if self.s.get(self.input..self.input + lit.len()) != Some(&lit[..]) {
                        return false;
                    }
                    self.input += lit.len();
                }
                op::NOTHING | op::BACK => {}
                op::BRANCH => {
                    let Some(nx) = next else { return false };
                    if self.prog[nx] != op::BRANCH {
                        scan = scan + 3;
                        continue;
                    }
                    let mut b = scan;
                    loop {
                        let save = self.input;
                        if self.matches(b + 3) {
                            return true;
                        }
                        self.input = save;
                        match self.next(b) {
                            Some(n) if self.prog[n] == op::BRANCH => b = n,
                            _ => return false,
                        }
                    }
                }
                op::STAR | op::PLUS => {
                    let Some(nx) = next else { return false };
                    let nextch = if self.prog[nx] == op::EXACTLY { self.prog[nx + 3] } else { 0 };
                    let min = (o == op::PLUS) as usize;
                    let save = self.input;
                    let mut no = self.repeat(scan + 3);
                    while no >= min {
                        self.input = save + no;
                        if (nextch == 0 || self.ch(self.input) == nextch) && self.matches(nx) {
                            return true;
                        }
                        if no == 0 {
                            break;
                        }
                        no -= 1;
                    }
                    return false;
                }
                op::WORDA => {
                    if !self.word_char(self.ch(self.input)) {
                        return false;
                    }
                    if self.input > self.bol && self.word_char(self.ch(self.input - 1)) {
                        return false;
                    }
                }
                op::WORDZ => {
                    if self.word_char(self.ch(self.input)) {
                        return false;
                    }
                }
                op::WSTART => {
                    let c = self.ch(self.input);
                    if !self.alpha(c) && c != '`' as u16 {
                        return false;
                    }
                    if self.input > self.bol {
                        let p = self.ch(self.input - 1);
                        if self.alpha(p) || p == '`' as u16 {
                            return false;
                        }
                    }
                }
                op::WEND => {
                    let c = self.ch(self.input);
                    if self.alpha(c) || c == '`' as u16 || c == '\'' as u16 {
                        return false;
                    }
                }
                o if (op::OPEN + 1..op::OPEN + 10).contains(&o) || (op::CLOSE + 1..op::CLOSE + 10).contains(&o) => {
                    let save = self.input;
                    let Some(nx) = next else { return false };
                    if !self.matches(nx) {
                        return false;
                    }
                    let (k, v) = if o < op::CLOSE { ((o - op::OPEN) as usize, &mut self.startp) } else { ((o - op::CLOSE) as usize, &mut self.endp) };
                    if v[k].is_none() {
                        v[k] = Some(save);
                    }
                    return true;
                }
                _ => return false,
            }
            match next {
                Some(n) => scan = n,
                None => return false,
            }
        }
    }
}

impl Rxc {
    /// `ELQRxcFindReplace` 0x1001f7e0 on `s[from..]` (that substring is the
    /// whole string to it): the match's offset in `s`, its length and the
    /// replacement (`&`, `\0`…`\9` its groups, `\\` and `\&` the
    /// characters, any other `\` kept).
    fn find(&self, cl: &Classes, s: &[u16], from: usize) -> Option<(usize, usize, Vec<char>)> {
        let mut e = Exec {
            prog: &self.prog,
            s,
            bol: from,
            input: from,
            startp: [None; 10],
            endp: [None; 10],
            cl,
        };
        let mut at = from;
        loop {
            e.input = at;
            e.startp = [None; 10];
            e.endp = [None; 10];
            if e.matches(0) {
                e.startp[0] = Some(at);
                e.endp[0] = Some(e.input);
                break;
            }
            if self.anchored || at >= s.len() {
                return None;
            }
            at += 1;
        }
        let (a, b) = (e.startp[0]?, e.endp[0]?);
        let mut out = Vec::new();
        let r = &self.replace;
        let mut i = 0;
        let group = |k: usize, out: &mut Vec<char>| {
            if let (Some(x), Some(y)) = (e.startp[k], e.endp[k]) {
                if x < y {
                    out.extend(s[x..y].iter().map(|&u| char::from_u32(u as u32).unwrap_or('\u{fffd}')));
                }
            }
        };
        while i < r.len() {
            let c = r[i];
            i += 1;
            if c == '&' {
                group(0, &mut out);
                continue;
            }
            let mut c = c;
            if c == '\\' {
                if let Some(d) = r.get(i).and_then(|d| d.to_digit(10)) {
                    i += 1;
                    group(d as usize, &mut out);
                    continue;
                }
                if matches!(r.get(i), Some('\\' | '&')) {
                    c = r[i];
                    i += 1;
                }
            }
            out.push(c);
        }
        Some((a, b - a, out))
    }
}

// --- Filters ---------------------------------------------------------------------

/// One lexicon rule: its key as raw word list entries, its value.
#[derive(Clone, Debug)]
struct Entry {
    key: Vec<RawTok>,
    value: Vec<char>,
    index: usize,
}

#[derive(Clone, Debug)]
enum Rules {
    /// a `<regexp>` section (or a RegExp file): every expression at once
    Regexp(Vec<Rxc>),
    /// a `<section>`: rules in the engine's order (0x10038950: first key
    /// token, longer keys first, then file order)
    Lexicon(Vec<Entry>),
}

#[derive(Clone, Debug)]
struct Section {
    rules: Rules,
    /// `case="sensitive"`
    sensitive: bool,
    /// a regexp section applied before the lexicon sections (else
    /// `apply="last"`: after them)
    before: bool,
}

/// One text filter file (`ELQTextFilterIni` 0x10038770).
#[derive(Clone, Debug, Default)]
pub struct Filter {
    sections: Vec<Section>,
}

/// LTTS7Util 0x10039230: the rule line reader's states.
fn rule_lines(text: &[char], mut emit: impl FnMut(Vec<char>, Vec<char>)) {
    let ws = |c: char| matches!(c, ' ' | '\t' | '\r' | '\x0b' | '\x08' | '\x0c');
    let mut state = 0;
    let (mut key, mut value): (Option<Vec<char>>, Vec<char>) = (None, Vec::new());
    let mut k = 0;
    loop {
        let c = text.get(k).copied().unwrap_or('\0');
        let end = c == '\n' || c == '\0';
        state = match state {
            0 if c == '"' => {
                key = Some(Vec::new());
                value.clear();
                1
            }
            0 if c == ';' || c == '#' => 6,
            0 => 0,
            1 if c == '"' => 2,
            1 if end => 7,
            1 => {
                key.get_or_insert_with(Vec::new).push(c);
                1
            }
            2 | 3 if end => 7,
            2 if c == '=' => 3,
            3 if c == '"' => 4,
            2 | 3 if ws(c) => state,
            2 | 3 => 7,
            4 if c == '"' => 5,
            4 if end => 7,
            4 => {
                value.push(c);
                if c == '\\' {
                    8
                } else {
                    4
                }
            }
            5 if end => {
                if let Some(kk) = key.take() {
                    emit(kk, std::mem::take(&mut value));
                }
                0
            }
            5 if ws(c) => 5,
            5 => 7,
            6 | 7 if end => 0,
            6 | 7 => state,
            8 if end => 7,
            8 if c == '"' => 5,
            8 => {
                value.push(c);
                if ws(c) {
                    4
                } else if c == '(' {
                    9
                } else {
                    8
                }
            }
            9 if end => 7,
            9 => {
                value.push(c);
                if c == ')' {
                    8
                } else {
                    9
                }
            }
            _ => 0,
        };
        if state == 7 {
            key = None;
        }
        if k >= text.len() {
            break;
        }
        k += 1;
    }
}

/// XML character entities.
fn entities(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

/// `name="value"` attributes of a start tag.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let mut rest = tag;
    while let Some(p) = rest.find(name) {
        let after = rest[p + name.len()..].trim_start();
        let boundary = p == 0 || rest[..p].ends_with(|c: char| c.is_whitespace());
        if boundary {
            if let Some(v) = after.strip_prefix('=') {
                let v = v.trim_start();
                if let Some(q) = v.chars().next().filter(|q| *q == '"' || *q == '\'') {
                    return v[1..].find(q).map(|e| v[1..1 + e].to_string());
                }
            }
        }
        rest = &rest[p + name.len()..];
    }
    None
}

/// Decrypt an encrypted lexicon or database file (`CA FE` header,
/// `ELQCryptBufferDecrypt` 0x10004ba0): each byte XOR the byte before it in
/// the file, the first 0x34.
pub fn decrypt(b: &[u8]) -> Vec<u8> {
    if !b.starts_with(&[0xca, 0xfe]) {
        return b.to_vec();
    }
    let mut prev = 0x34u8;
    b[2..]
        .iter()
        .map(|&c| {
            let p = c ^ prev;
            prev = c;
            p
        })
        .collect()
}

impl Filter {
    /// A RegExp (`regexp`) or Lexicon file: XML (`<loqlexicon>` with
    /// `<section case=…>` and `<regexp apply=…>` elements) or plain lines,
    /// which then form one section (`all`) of regular expressions when the
    /// file is a RegExp file (the engine's SetAllRegexp configuration).
    pub fn parse(cl: &Classes, file: &[u8], regexp: bool) -> Option<Filter> {
        let plain = decrypt(file);
        let text = String::from_utf8_lossy(&plain);
        let text = text.trim_start_matches('\u{feff}');
        let mut f = Filter::default();
        if !text.trim_start().starts_with('<') {
            f.push(cl, text, regexp, false, true);
            return (!f.sections.is_empty()).then_some(f);
        }
        let mut rest = text;
        let mut open: Option<(bool, bool, bool)> = None;
        let mut data = String::new();
        while !rest.is_empty() {
            let Some(lt) = rest.find('<') else {
                if open.is_some() {
                    data.push_str(&entities(rest));
                }
                break;
            };
            if open.is_some() {
                data.push_str(&entities(&rest[..lt]));
            }
            rest = &rest[lt..];
            if let Some(r) = rest.strip_prefix("<!--") {
                rest = r.find("-->").map_or("", |e| &r[e + 3..]);
                continue;
            }
            let gt = rest.find('>').unwrap_or(rest.len() - 1);
            let tag = &rest[1..gt];
            rest = &rest[(gt + 1).min(rest.len())..];
            let name = tag.trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("");
            if tag.starts_with('/') {
                if let Some((re, sens, before)) = open.take() {
                    f.push(cl, &std::mem::take(&mut data), re, sens, before);
                }
            } else if name == "section" || name == "regexp" {
                let re = name == "regexp";
                let sens = attribute(tag, "case").is_some_and(|v| v.eq_ignore_ascii_case("sensitive"));
                let before = !(re && attribute(tag, "apply").is_some_and(|v| v.eq_ignore_ascii_case("last")));
                open = Some((re, sens, before));
                data.clear();
            }
        }
        (!f.sections.is_empty()).then_some(f)
    }

    fn push(&mut self, cl: &Classes, data: &str, regexp: bool, sensitive: bool, before: bool) {
        let chars: Vec<char> = data.chars().collect();
        let mut rules = Vec::new();
        rule_lines(&chars, |k, v| rules.push((k, v)));
        let rules = if regexp {
            Rules::Regexp(rules.iter().filter_map(|(k, v)| Rxc::new(k, v)).collect())
        } else {
            let add = AddText { cl, pattern: true, tags: true };
            let mut entries: Vec<Entry> = rules
                .into_iter()
                .enumerate()
                .filter_map(|(index, (k, value))| {
                    let key = add.run(&k, false);
                    (!key.is_empty()).then_some(Entry { key, value, index })
                })
                .collect();
            entries.sort_by(|a, b| b.key.len().cmp(&a.key.len()).then(a.index.cmp(&b.index)));
            Rules::Lexicon(entries)
        };
        self.sections.push(Section { rules, sensitive, before });
    }
}

/// One expression's current match: start, length, replacement.
type Found = Option<(usize, usize, Vec<char>)>;

/// LTTS7Util 0x1003bce0: a regexp section over the list. All expressions
/// search the text at once; the earliest match (the longest of those
/// starting there, then the first expression) is replaced, and every
/// expression whose match started before its end searches again from
/// there, the rest of the text as a string of its own (`^` and word starts
/// match there). Each replacement is tokenized and spliced into the list
/// in place of the characters it replaces.
fn apply_regexps(cl: &Classes, rx: &[Rxc], list: &mut Vec<RawTok>) {
    let text = detokenize(list, true, true);
    if text.is_empty() || rx.is_empty() {
        return;
    }
    let s: Vec<u16> = text.iter().map(|&c| c as u32 as u16).collect();
    let mut found: Vec<Found> = rx.iter().map(|r| r.find(cl, &s, 0)).collect();
    let add = AddText { cl, pattern: false, tags: true };
    let mut delta: isize = 0;
    let mut last_end: Option<usize> = None;
    let mut any = false;
    loop {
        let mut best: Option<usize> = None;
        for (k, f) in found.iter().enumerate() {
            if let Some((st, len, _)) = f {
                if best.is_none_or(|b| {
                    let (bs, bl, _) = found[b].as_ref().unwrap();
                    st < bs || (st == bs && len > bl)
                }) {
                    best = Some(k);
                }
            }
        }
        let Some(b) = best else { break };
        let (st, len, repl) = found[b].clone().unwrap();
        let end = st + len;
        for f in found.iter_mut().enumerate() {
            if f.1.as_ref().is_some_and(|(s2, _, _)| *s2 <= end) {
                *f.1 = rx[f.0].find(cl, &s, end);
            }
        }
        // an empty match, or one inside the last replacement, ends the section
        let pos = (st as isize + delta) as usize;
        if len == 0 || last_end.is_some_and(|e| e > pos) {
            break;
        }
        let r_list = add.run(&repl, false);
        let r_text = detokenize(&r_list, true, true);
        if r_text.is_empty() {
            delta -= len as isize;
            last_end = Some(pos);
        } else {
            delta += r_text.len() as isize - len as isize;
            last_end = Some(pos + r_text.len());
        }
        splice(list, pos, len, r_list);
        any = true;
    }
    if any {
        compact(list);
    }
}

/// The splice of 0x1003bce0: the entries from the one holding `pos` to
/// the one holding `pos + len` give way to `r`, keeping the text before
/// and after the match in them.
fn splice(list: &mut Vec<RawTok>, pos: usize, len: usize, mut r: Vec<RawTok>) {
    let (end_tok, end_in_word, end_off) = find_position(list, pos + len).unwrap_or((list.len() - 1, false, 0));
    let (start_tok, start_in_word, start_off) = find_position(list, pos).unwrap_or((list.len() - 1, true, 0));
    let same = start_tok == end_tok;
    if !start_in_word || start_off != 0 {
        // what of the start entry comes before the match
        let s = &mut list[start_tok];
        let mut e = RawTok::default();
        if let Some(w) = s.word.clone() {
            if !start_in_word || u8len(&w[..start_off.min(w.len())]) > 0x4e {
                e.word = Some(w);
            } else {
                let prefix = w[..start_off.min(w.len())].to_vec();
                let term = |c: Option<&char>| c.is_some_and(|c| ";:.?!".contains(*c));
                if s.flags & 4 != 0 && !prefix.is_empty() && term(w.last()) && !term(prefix.last()) {
                    s.flags -= 4;
                }
                e.word = some(prefix);
            }
        }
        if let Some(sep) = &s.sep {
            if !start_in_word && u8len(&sep[..start_off.min(sep.len())]) < 0x4f {
                e.sep = some(sep[..start_off.min(sep.len())].to_vec());
            }
        }
        // its offsets and flags, no type (a new entry's)
        e.flags = s.flags;
        r.insert(0, e);
    }
    // the start entry's tags go to the first new entry (none: lost)
    if let Some(first) = r.first_mut() {
        add_to(&mut first.tag, &list[start_tok].tag.clone());
    }
    if same {
        list[start_tok].tag = None;
    }
    // the entries before the end one go
    list.drain(start_tok..end_tok);
    let e = &mut list[start_tok];
    if !end_in_word {
        e.word = None;
        if end_off != 0 {
            if let Some(sep) = &e.sep {
                e.sep = some(sep[end_off.min(sep.len())..].to_vec());
            }
        }
    } else if end_off != 0 {
        if let Some(w) = &e.word {
            e.word = some(w[end_off.min(w.len())..].to_vec());
        }
    }
    copy_before(list, start_tok, r);
}

impl Filter {
    /// `ELQTextFilterApply` 0x1003a470 on the list: the regexp sections
    /// applied first, then from the first entry on, at each entry the
    /// first section with a rule whose key matches there (0x1003a220)
    /// replaces the entries it covers (they stay, emptied, with their end
    /// flags; the value's entries follow them, the last covered one's
    /// separator after them) and the search goes on after the value; then
    /// the regexp sections applied last.
    fn apply(&self, cl: &Classes, list: &mut Vec<RawTok>) {
        for s in self.sections.iter().filter(|s| s.before) {
            if let Rules::Regexp(rx) = &s.rules {
                apply_regexps(cl, rx, list);
            }
        }
        let mut at = 0usize;
        while at < list.len() {
            let Some((entry, caps)) = self.sections.iter().find_map(|s| match &s.rules {
                Rules::Lexicon(entries) => self.find_rule(cl, entries, s.sensitive, list, at),
                Rules::Regexp(_) => None,
            }) else {
                at += 1;
                continue;
            };
            let add = AddText { cl, pattern: true, tags: true };
            let mut r = add.run(&entry.value, false);
            // `%1`… take the matched tokens' words and types (0x1003a3c0)
            let mut typ = 0;
            for e in r.iter_mut().filter(|e| e.typ == ty::PATTERN) {
                if let Some(k) = e.word.as_deref().and_then(pattern_index) {
                    e.word = caps.get(k).map(|(w, t)| {
                        typ = *t;
                        w.clone()
                    });
                    e.typ = typ;
                }
            }
            // the covered entries emptied, the value after them (before the
            // last one when they reach the end), the last one's separator
            // after the value
            let n = entry.key.len();
            let mut cur = at;
            let mut saved_sep = None;
            for _ in 0..n {
                let e = &mut list[cur];
                e.word = None;
                e.typ = ty::NULL;
                saved_sep = e.sep.take();
                if cur + 1 < list.len() {
                    cur += 1;
                }
            }
            let rn = copy_before(list, cur, r);
            let focus = if rn > 0 { cur + rn - 1 } else { cur };
            add_to(&mut list[focus].sep, &saved_sep);
            at = focus + 1;
        }
        for s in self.sections.iter().filter(|s| !s.before) {
            if let Rules::Regexp(rx) = &s.rules {
                apply_regexps(cl, rx, list);
            }
        }
    }

    /// 0x10039f40 (sorted sections): the first rule whose first key token is
    /// the entry's word (case aside) and whose key matches from `at`
    /// (0x10039660), else the first of the rules starting with a pattern;
    /// with the tokens its patterns matched.
    fn find_rule<'a>(&self, cl: &Classes, entries: &'a [Entry], sensitive: bool, list: &[RawTok], at: usize) -> Option<(&'a Entry, Vec<(Vec<char>, u32)>)> {
        let word = list[at].word.as_deref();
        let starts = |e: &Entry| e.key[0].typ == ty::PATTERN;
        let first = entries.iter().filter(|e| !starts(e) && word.is_some_and(|w| e.key[0].word.as_deref().is_some_and(|k| cl.eq_nocase(k, w))));
        let patterned = entries.iter().filter(|e| starts(e));
        for e in first.chain(patterned) {
            if let Some(caps) = key_matches(cl, e, sensitive, list, at) {
                return Some((e, caps));
            }
        }
        None
    }
}

/// 0x10039660: a rule's key against the list from `at`: word by word
/// (patterns by `ELQPatternMatch`, else exactly or case aside), and between
/// them both a separator or neither.
fn key_matches(cl: &Classes, e: &Entry, sensitive: bool, list: &[RawTok], at: usize) -> Option<Vec<(Vec<char>, u32)>> {
    let mut caps = Vec::new();
    for (k, kt) in e.key.iter().enumerate() {
        let Some(kw) = &kt.word else { return Some(caps) };
        let t = list.get(at + k)?;
        let tw = t.word.as_ref()?;
        if kt.typ == ty::PATTERN {
            if !pattern_match(cl, kw, tw) {
                return None;
            }
            caps.push((tw.clone(), t.typ));
        } else if !(if sensitive { kw == tw } else { cl.eq_nocase(kw, tw) }) {
            return None;
        }
        if k + 1 < e.key.len() && kt.sep.is_some() != t.sep.is_some() {
            return None;
        }
    }
    Some(caps)
}

/// The text filters of one language as LesRun applies them: its RegExp
/// files, then its Lexicon files, each list last first.
pub struct TextFilters {
    cl: Classes,
    regexps: Vec<Filter>,
    lexicons: Vec<Filter>,
}

impl TextFilters {
    /// From LTTS7Util.dll (for its character tables) and the `.lde`'s
    /// `RegExp` and `Lexicon` files in their order (`X`, then `X[1]`, …;
    /// LoqTTS7.dll 0x1002fb00).
    pub fn new(util: &[u8], regexps: &[Vec<u8>], lexicons: &[Vec<u8>]) -> Option<TextFilters> {
        let cl = Classes::from_util(&Pe::parse(util.to_vec())?)?;
        let regexps = regexps.iter().filter_map(|f| Filter::parse(&cl, f, true)).collect();
        let lexicons = lexicons.iter().filter_map(|f| Filter::parse(&cl, f, false)).collect();
        Some(TextFilters { cl, regexps, lexicons })
    }

    /// The text LesRun hands the sentence loop (what LOQ_TRACE_TEXT logs
    /// on its `T` lines).
    pub fn apply(&self, text: &str) -> String {
        let chars: Vec<char> = text.chars().collect();
        let add = AddText {
            cl: &self.cl,
            pattern: false,
            tags: true,
        };
        let mut list = add.run(&chars, true);
        for f in self.regexps.iter().rev().chain(self.lexicons.iter().rev()) {
            f.apply(&self.cl, &mut list);
        }
        compact(&mut list);
        detokenize(&list, false, false).into_iter().collect()
    }
}
