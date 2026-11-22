//! Mandarin, for LinLin: text → words → pinyin → the phones she recorded.
//!
//! The words and their pinyin come from the language's own dictionary,
//! `Chinese.chd` (LTTS 7.2's, plain UTF-8 `词\tci2` lines, tone sandhi inside
//! words already applied: `一万 yi2wan4`), found in Loquendo's Android build.
//! Its module (`libLTTS7Chinese.so`) exists only for ARM there and is not
//! ported: how a syllable becomes phones is fitted to LinLin's 3 982 recorded
//! sentences, which write each as initial, glide and toned final
//! (`xuan2` → c Hy E2 n, `shi2` → Rs IiR2, `zheng3` → HRDz HEh)3 N). Text is
//! split into words by the longest dictionary match; typed pinyin with tone
//! numbers (`wo3 jiao4`, as her `DemoSentence`) and digits are read too.

use super::{Frontend, Token};
use std::collections::HashMap;

/// The dictionary of `Chinese.chd`: each word's syllables (`yi2`, `wan4`),
/// and the longest word in characters.
pub fn read_dictionary(text: &[u8]) -> (HashMap<String, Vec<String>>, usize) {
    let mut words = HashMap::new();
    let mut longest = 0;
    for line in String::from_utf8_lossy(text).lines() {
        let Some((word, pinyin)) = line.split_once('\t') else { continue };
        let syllables = syllables_of(&pinyin.trim().to_lowercase());
        if word.is_empty() || syllables.is_empty() {
            continue;
        }
        longest = longest.max(word.chars().count());
        words.entry(word.to_string()).or_insert(syllables);
    }
    (words, longest)
}

/// `yi2wan4` → `yi2`, `wan4`: each syllable ends in its tone digit. None
/// when anything is left over (not pinyin).
fn syllables_of(pinyin: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in pinyin.chars().filter(|c| !c.is_whitespace()) {
        cur.push(c);
        if matches!(c, '1'..='5') {
            out.push(std::mem::take(&mut cur));
        }
    }
    if cur.is_empty() {
        out
    } else {
        Vec::new()
    }
}

const DIGITS: [&str; 10] = ["ling2", "yi1", "er4", "san1", "si4", "wu3", "liu4", "qi1", "ba1", "jiu3"];

/// Up to four digits (`n` < 10 000) as syllables; `first` for the number's
/// leading group, where 10–19 say `shi2…`, not `yi1 shi2…`.
fn group(n: u64, first: bool) -> Vec<&'static str> {
    let units = ["qian1", "bai3", "shi2", ""];
    let digits = [n / 1000, n / 100 % 10, n / 10 % 10, n % 10];
    let mut out = Vec::new();
    let mut gap = false;
    for (i, &d) in digits.iter().enumerate() {
        if d == 0 {
            gap = !out.is_empty();
            continue;
        }
        if gap {
            out.push("ling2");
            gap = false;
        }
        match (i, d) {
            (2, 1) if first && out.is_empty() => {}
            // 两 before 千 and 百
            (0 | 1, 2) => out.push("liang3"),
            _ => out.push(DIGITS[d as usize]),
        }
        if !units[i].is_empty() {
            out.push(units[i]);
        }
    }
    out
}

/// Digits as Mandarin reads them: a number up to twelve digits (`2024` →
/// liang3 qian1 ling2 er4 shi2 si4), else digit by digit, as a leading zero
/// also asks (phone numbers).
fn number(digits: &str) -> Vec<String> {
    let value = digits
        .parse::<u64>()
        .ok()
        .filter(|_| digits.len() <= 12 && (digits == "0" || !digits.starts_with('0')));
    let Some(n) = value else {
        return digits.bytes().map(|b| DIGITS[(b - b'0') as usize].to_string()).collect();
    };
    if n == 0 {
        return vec!["ling2".into()];
    }
    let groups = [(n / 100_000_000, "yi4"), (n / 10_000 % 10_000, "wan4"), (n % 10_000, "")];
    let mut out: Vec<&str> = Vec::new();
    let mut gap = false;
    for (g, unit) in groups {
        if g == 0 {
            gap = !out.is_empty();
            continue;
        }
        // a zero between groups, or a group of fewer than four digits after one
        if gap || (!out.is_empty() && g < 1000) {
            out.push("ling2");
        }
        gap = false;
        match g {
            2 if !unit.is_empty() => out.push("liang3"),
            _ => out.extend(group(g, out.is_empty())),
        }
        if !unit.is_empty() {
            out.push(unit);
        }
    }
    out.into_iter().map(str::to_string).collect()
}

pub(crate) fn is_hanzi(c: char) -> bool {
    matches!(c, '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}' | '\u{f900}'..='\u{faff}' | '\u{20000}'..='\u{2ffff}')
}

/// One word of the text with its syllables.
struct Word {
    text: String,
    syllables: Vec<String>,
}

/// Split a run of text (no spaces or punctuation) into words: Chinese by the
/// longest dictionary match, typed pinyin as is, digits as a number. What
/// none of these reads (Latin words, characters the dictionary lacks) is
/// left out.
fn words(run: &str, fe: &Frontend) -> Vec<Word> {
    let chars: Vec<char> = run.chars().collect();
    let (dict, longest) = match &fe.data {
        Some(d) => (Some(&d.dictionary), d.longest),
        None => (None, 0),
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if is_hanzi(chars[i]) {
            let found = dict.and_then(|dict| {
                (1..=longest.min(chars.len() - i)).rev().find_map(|n| {
                    let w: String = chars[i..i + n].iter().collect();
                    dict.get(&w).map(|s| (n, w, s.clone()))
                })
            });
            match found {
                Some((n, text, syllables)) => {
                    out.push(Word { text, syllables });
                    i += n;
                }
                None => i += 1,
            }
            continue;
        }
        let start = i;
        while i < chars.len() && !is_hanzi(chars[i]) {
            i += 1;
        }
        let latin: String = chars[start..i].iter().collect();
        let pinyin = syllables_of(&latin);
        if !pinyin.is_empty() && pinyin.iter().all(|s| split(s).is_some()) {
            out.push(Word {
                text: latin,
                syllables: pinyin,
            });
            continue;
        }
        // digits within it, as numbers; the letters around them are not read
        let mut digits = String::new();
        for c in latin.chars().chain(std::iter::once(' ')) {
            if c.is_ascii_digit() {
                digits.push(c);
            } else if !digits.is_empty() {
                let d = std::mem::take(&mut digits);
                out.push(Word {
                    syllables: number(&d),
                    text: d,
                });
            }
        }
    }
    out
}

/// A syllable (`zhuang4`, `lv3`, `nü3`, `yuan2`, `xiar4`) as its initial,
/// its final in full form (`y`/`w` spellings undone, `iu` → `iou`, ü as
/// `v`, an erhua `r` dropped) and tone.
fn split(syllable: &str) -> Option<(&'static str, String, u8)> {
    let tone = syllable.chars().last()?.to_digit(10).filter(|t| (1..=5).contains(t))? as u8;
    let body = syllable[..syllable.len() - 1].replace("u:", "v").replace('ü', "v");
    const INITIALS: [&str; 23] = [
        "zh", "ch", "sh", "b", "p", "m", "f", "d", "t", "n", "l", "g", "k", "h", "j", "q", "x", "r", "z", "c", "s", "y", "w",
    ];
    let initial = INITIALS.iter().copied().find(|i| body.starts_with(i) && body.len() > i.len()).unwrap_or("");
    let rest = &body[initial.len()..];
    let mut fin = match initial {
        "y" => match rest {
            "u" | "ue" | "uan" | "un" => format!("v{}", &rest[1..]),
            r if r.starts_with('i') => r.to_string(),
            r => format!("i{r}"),
        },
        "w" => match rest {
            "u" => "u".to_string(),
            r => format!("u{r}"),
        },
        "j" | "q" | "x" if rest.starts_with('u') => format!("v{}", &rest[1..]),
        _ => rest.to_string(),
    };
    fin = match fin.as_str() {
        "iu" => "iou".into(),
        "ui" => "uei".into(),
        "un" => "uen".into(),
        _ => fin,
    };
    // erhua (`xiar4`): the final without its r
    if final_phones(&fin).is_none() && fin.len() > 1 && fin.ends_with('r') {
        fin.pop();
    }
    final_phones(&fin)?;
    let initial = match initial {
        "y" | "w" => "",
        i => i,
    };
    Some((initial, fin, tone))
}

/// A final's phones, `%` where its vowel takes the tone.
fn final_phones(fin: &str) -> Option<&'static str> {
    Some(match fin {
        "a" => "a%",
        "o" => "Oa%",
        "e" => "HEh%",
        "ai" => "ai%",
        "ei" => "ei%",
        "ao" => "HAU%",
        "ou" => "HOU%",
        "an" => "a% n",
        "en" => "AR)% n",
        "ang" => "Aa)% N",
        "eng" => "HEh)% N",
        "ong" => "U)% N",
        "er" => "ARR%",
        "i" => "i%",
        "ia" => "j a%",
        "ie" => "j E%",
        "iao" => "j HAU%",
        "iou" => "j HOU%",
        "ian" => "j E% n",
        "in" => "i% n",
        "iang" => "j Aa)% N",
        "ing" => "i% N",
        "iong" => "j U)% N",
        "u" => "U%",
        "ua" => "w a%",
        "uo" => "w Oa%",
        "uai" => "w ai%",
        "uei" => "w ei%",
        "uan" => "w a% n",
        "uen" => "w AR)% n",
        "uang" => "w Aa)% N",
        "ueng" => "w HEh)% N",
        "v" => "y%",
        "ve" => "Hy E%",
        "van" => "Hy E% n",
        "vn" => "y% n",
        _ => return None,
    })
}

const INITIAL_PHONES: [(&str, &str); 21] = [
    ("b", "HB"),
    ("p", "ph"),
    ("m", "m"),
    ("f", "f"),
    ("d", "HD"),
    ("t", "th"),
    ("n", "n"),
    ("l", "l"),
    ("g", "HG"),
    ("k", "kh"),
    ("h", "h"),
    ("j", "HPDz"),
    ("q", "PTsh"),
    ("x", "c"),
    ("zh", "HRDz"),
    ("ch", "RTsh"),
    ("sh", "Rs"),
    ("r", "Rz"),
    ("z", "HDz"),
    ("c", "Tsh"),
    ("s", "s"),
];

/// Tone sandhi across words: 不 bu4 before a fourth tone is bu2; 一 yi1
/// before a fourth tone yi2, before the others yi4; of two third tones the
/// first is a second.
fn sandhi(words: &mut [Word]) {
    let tone_at = |words: &[Word], w: usize, s: usize| -> Option<u8> {
        let next = words[w].syllables.get(s + 1).or_else(|| words.get(w + 1).and_then(|n| n.syllables.first()))?;
        next.chars().last()?.to_digit(10).map(|t| t as u8)
    };
    for w in 0..words.len() {
        for s in 0..words[w].syllables.len() {
            let next = tone_at(words, w, s);
            let syl = &words[w].syllables[s];
            let changed = match (words[w].text.as_str(), syl.as_str(), next) {
                ("不", "bu4", Some(4)) => Some("bu2".to_string()),
                ("一", "yi1", Some(4)) => Some("yi2".to_string()),
                ("一", "yi1", Some(1..=3)) => Some("yi4".to_string()),
                (_, s, Some(3)) if s.ends_with('3') => Some(format!("{}2", &s[..s.len() - 1])),
                _ => None,
            };
            if let Some(c) = changed {
                words[w].syllables[s] = c;
            }
        }
    }
}

/// A run of text between pauses as words with LinLin's phones. The
/// allophones her recordings show: d voiced (`d`) in a neutral tone or
/// after a nasal, a glottal onset (`Hg`) on a vowel-initial syllable inside
/// the run, `fu` as f Hfw U, and before the pause `in`/`ing` with a nasal
/// vowel (`i)4 n`, `I)2 N`).
pub fn analyse_run(run: &str, fe: &Frontend) -> Vec<Token> {
    let mut words = words(run, fe);
    sandhi(&mut words);
    let total: usize = words.iter().map(|w| w.syllables.len()).sum();
    let mut out = Vec::with_capacity(words.len());
    let mut seen = 0;
    let mut prev_nasal = false;
    for w in words {
        let mut symbols = Vec::new();
        for syl in &w.syllables {
            seen += 1;
            let Some((initial, fin, tone)) = split(syl) else { continue };
            let mut phones: Vec<String> = Vec::new();
            match INITIAL_PHONES.iter().find(|(i, _)| *i == initial).map(|(_, p)| *p) {
                Some("HD") if tone == 5 || prev_nasal => phones.push("d".into()),
                Some(p) => phones.push(p.into()),
                None => {
                    if fin.starts_with('i') && !fin.starts_with("ia") && !fin.starts_with("ie") && !fin.starts_with("io") {
                        // yi, yin, ying: the glide before the vowel
                        phones.push("j".into());
                    } else if fin == "u" {
                        phones.push("w".into());
                    } else if !fin.starts_with(['i', 'u', 'v']) && seen > 1 {
                        phones.push("Hg".into());
                    }
                }
            }
            let mut pattern = final_phones(&fin).unwrap_or("");
            // the apical vowels of zi ci si and zhi chi shi ri
            if fin == "i" {
                pattern = match initial {
                    "z" | "c" | "s" => "Ii%",
                    "zh" | "ch" | "sh" | "r" => "IiR%",
                    _ => pattern,
                };
            }
            if initial == "f" && fin == "u" {
                phones.push("Hfw".into());
            }
            let last = seen == total && tone != 5;
            // LinLin recorded er in every tone but the first: its second
            let tone = if fin == "er" && tone == 1 { 2 } else { tone };
            for part in pattern.split(' ') {
                let part = match (last, fin.as_str(), part) {
                    (true, "in", "i%") => "i)%",
                    (true, "ing", "i%") => "I)%",
                    _ => part,
                };
                phones.push(part.replace('%', &tone.to_string()));
            }
            prev_nasal = matches!(phones.last().map(String::as_str), Some("n" | "N"));
            symbols.extend(phones);
        }
        if !symbols.is_empty() {
            out.push(Token::Word { text: w.text, symbols });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fe(dict: &str) -> Frontend {
        let mut f = Frontend::rules(super::super::Language::Chinese);
        let (dictionary, longest) = read_dictionary(dict.as_bytes());
        f.data = Some(std::sync::Arc::new(super::super::LanguageData { dictionary, longest }));
        f
    }

    fn phones(text: &str, dict: &str) -> String {
        analyse_run(text, &fe(dict))
            .iter()
            .map(|t| t.symbols().join(" "))
            .collect::<Vec<_>>()
            .join(" | ")
    }

    /// Syllables as LinLin's recordings write them.
    #[test]
    fn syllables_as_recorded() {
        let dict = "三\tsan1\n十\tshi2\n下\txia4\n对\tdui4\n选\txuan3\n请\tqing3\n正\tzheng4\n二\ter4\n去\tqu4\n月\tyue4\n";
        assert_eq!(phones("三十", dict), "s a1 n | Rs IiR2");
        assert_eq!(phones("下对", dict), "c j a4 | HD w ei4");
        assert_eq!(phones("选", dict), "c Hy E3 n");
        // the run's end: ing's vowel nasal
        assert_eq!(phones("请", dict), "PTsh I)3 N");
        assert_eq!(phones("正去月", dict), "HRDz HEh)4 N | PTsh y4 | Hy E4");
        // a vowel-initial syllable inside the run takes the glottal onset
        assert_eq!(phones("三二", dict), "s a1 n | Hg ARR4");
    }

    /// The longest dictionary word wins; typed pinyin and digits are read.
    #[test]
    fn words_numbers_and_pinyin() {
        let dict = "中\tzhong1\n中国\tzhong1guo2\n国\tguo2\n人\tren2\n";
        let ws: Vec<String> = analyse_run("中国人", &fe(dict))
            .iter()
            .map(|t| match t {
                Token::Word { text, .. } => text.clone(),
                _ => String::new(),
            })
            .collect();
        assert_eq!(ws, ["中国", "人"]);
        assert_eq!(phones("wo3jiao4", ""), "w Oa3 HPDz j HAU4");
        assert_eq!(number("2024"), ["liang3", "qian1", "ling2", "er4", "shi2", "si4"]);
        assert_eq!(number("15"), ["shi2", "wu3"]);
        assert_eq!(number("100005"), ["shi2", "wan4", "ling2", "wu3"]);
        assert_eq!(number("20000"), ["liang3", "wan4"]);
        assert_eq!(number("0571"), ["ling2", "wu3", "qi1", "yi1"]);
    }

    /// 你好: two third tones, the first said as a second.
    #[test]
    fn third_tone_sandhi() {
        assert_eq!(phones("ni3hao3", ""), "n i2 h HAU3");
        assert_eq!(phones("不对", "不\tbu4\n对\tdui4\n"), "HB U2 | HD w ei4");
    }
}
