//! Where the front end's words lie in the text it was given, so what is
//! being said can be shown in the text itself.
//!
//! The front ends hand back their own spelling of each word: stress accents
//! added (`hò` for "Ho"), elisions joined (`l'àmico`), numbers and
//! abbreviations written out (`quattórdici` for "14", `dottór` for "dott.").
//! Words are found again by their letters and digits alone, ignoring case
//! and accents; the ones written out get the stretch of text they stand for.

use std::ops::Range;

use crate::g2p::Token;

/// A match further than this many of the text's words past the last one
/// must be confirmed by the next word following it there.
const NEAR: usize = 1;
/// How many of the text's words a word is looked for past the last one.
const FAR: usize = 64;

/// One letter or digit of the text, folded (lowercase, no accents; a letter
/// folding to two, `ß` → `ss`, gives two).
struct Ch {
    c: char,
    /// its bytes in the text
    at: Range<usize>,
    /// starts / ends one of the text's words (a run of letters and digits)
    first: bool,
    last: bool,
}

/// Byte ranges in `text` of each `Token::Word` in `tokens` (index = word
/// ordinal), found by a forward, case- and accent-insensitive search; a word
/// the front end rewrote (numbers, abbreviations, elisions) gets the stretch
/// of `text` between its matched neighbours (shared when several rewritten
/// words fall in one stretch, one each when the stretch has as many words).
/// Never panics; ranges are on char boundaries, non-decreasing.
pub fn word_spans(text: &str, tokens: &[Token]) -> Vec<Range<usize>> {
    let chars = letters(text);
    let words: Vec<Vec<char>> = tokens
        .iter()
        .filter_map(|t| match t {
            Token::Word { text, .. } => Some(text.chars().filter(|c| c.is_alphanumeric()).flat_map(fold).collect()),
            Token::Pause { .. } => None,
        })
        .collect();
    let at = |w: &[char], p: usize| {
        !w.is_empty() && p + w.len() <= chars.len() && chars[p].first && chars[p + w.len() - 1].last && w.iter().zip(&chars[p..]).all(|(a, b)| *a == b.c)
    };
    // the matched words, as ranges of `chars`
    let mut found: Vec<Option<Range<usize>>> = Vec::with_capacity(words.len());
    let mut from = 0;
    for (i, w) in words.iter().enumerate() {
        let next = words.get(i + 1);
        let mut skipped = 0;
        let mut hit = None;
        for p in (from..chars.len()).filter(|&p| chars[p].first) {
            if at(w, p) && (skipped <= NEAR || next.is_none_or(|n| at(n, p + w.len()))) {
                hit = Some(p..p + w.len());
                break;
            }
            skipped += 1;
            if skipped > FAR {
                break;
            }
        }
        if let Some(h) = &hit {
            from = h.end;
        }
        found.push(hit);
    }
    // the rewritten ones: the stretch between their neighbours
    let bytes = |r: &Range<usize>| chars[r.start].at.start..chars[r.end - 1].at.end;
    let mut out: Vec<Range<usize>> = Vec::with_capacity(words.len());
    let mut i = 0;
    while i < found.len() {
        if let Some(r) = &found[i] {
            out.push(bytes(r));
            i += 1;
            continue;
        }
        let j = (i..found.len()).find(|&j| found[j].is_some()).unwrap_or(found.len());
        let lo = if i == 0 { 0 } else { found[i - 1].as_ref().map_or(0, |r| r.end) };
        let hi = found.get(j).and_then(|r| r.as_ref()).map_or(chars.len(), |r| r.start);
        let runs: Vec<usize> = (lo..hi).filter(|&p| chars[p].first).collect();
        if runs.len() == j - i {
            for (k, &p) in runs.iter().enumerate() {
                let end = runs.get(k + 1).copied().unwrap_or(hi);
                let end = (p..end).rfind(|&q| chars[q].last).map_or(p + 1, |q| q + 1);
                out.push(bytes(&(p..end)));
            }
        } else {
            let span = if lo < hi {
                bytes(&(lo..hi))
            } else {
                let b = if lo == 0 { 0 } else { chars[lo - 1].at.end };
                b..b
            };
            out.extend(std::iter::repeat_n(span, j - i));
        }
        i = j;
    }
    out
}

/// Convert byte ranges to UTF-16 code unit ranges (for Kotlin/Android).
/// Positions inside a character count from its start.
pub fn utf16_range(text: &str, r: Range<usize>) -> Range<usize> {
    let floor = |b: usize| {
        let mut b = b.min(text.len());
        while !text.is_char_boundary(b) {
            b -= 1;
        }
        b
    };
    let a = floor(r.start);
    let b = floor(r.end).max(a);
    let start = text[..a].encode_utf16().count();
    start..start + text[a..b].encode_utf16().count()
}

/// The text's letters and digits, folded, with where each is. Chinese
/// writes no spaces, so each hanzi is a word of its own here (the front end's
/// words, 琳琳, span several).
fn letters(text: &str) -> Vec<Ch> {
    let mut out: Vec<Ch> = Vec::with_capacity(text.len());
    let mut in_word = false;
    for (i, c) in text.char_indices() {
        if !c.is_alphanumeric() {
            in_word = false;
            continue;
        }
        let hanzi = crate::g2p::is_hanzi(c);
        let at = i..i + c.len_utf8();
        for (k, f) in fold(c).enumerate() {
            out.push(Ch {
                c: f,
                at: at.clone(),
                first: k == 0 && (!in_word || hanzi),
                last: false,
            });
        }
        in_word = !hanzi;
    }
    for i in 0..out.len() {
        out[i].last = out.get(i + 1).is_none_or(|n| n.first);
    }
    out
}

/// A letter lowercase and without its accents.
fn fold(c: char) -> impl Iterator<Item = char> {
    c.to_lowercase().flat_map(|l| {
        let two = match l {
            'ß' => "ss",
            'æ' => "ae",
            'œ' => "oe",
            'ĳ' => "ij",
            _ => "",
        };
        let one = if two.is_empty() { Some(bare(l)) } else { None };
        two.chars().chain(one)
    })
}

/// Latin, Greek and Cyrillic letters without their diacritics.
fn bare(c: char) -> char {
    const TABLE: &[(&str, char)] = &[
        ("àáâãäåāăą", 'a'),
        ("çćĉċč", 'c'),
        ("ďđ", 'd'),
        ("èéêëēĕėęě", 'e'),
        ("ĝğġģ", 'g'),
        ("ĥħ", 'h'),
        ("ìíîïĩīĭįı", 'i'),
        ("ĵ", 'j'),
        ("ķ", 'k'),
        ("ĺļľŀł", 'l'),
        ("ñńņňŉ", 'n'),
        ("òóôõöøōŏő", 'o'),
        ("ŕŗř", 'r'),
        ("śŝşšș", 's'),
        ("ţťŧț", 't'),
        ("ùúûüũūŭůűų", 'u'),
        ("ŵ", 'w'),
        ("ýÿŷ", 'y'),
        ("źżž", 'z'),
        ("ά", 'α'),
        ("έ", 'ε'),
        ("ή", 'η'),
        ("ίϊΐ", 'ι'),
        ("ό", 'ο'),
        ("ύϋΰ", 'υ'),
        ("ώ", 'ω'),
        ("ς", 'σ'),
        ("ё", 'е'),
        ("й", 'и'),
    ];
    if c.is_ascii() {
        return c;
    }
    TABLE.iter().find(|(from, _)| from.contains(c)).map_or(c, |&(_, to)| to)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(list: &[&str]) -> Vec<Token> {
        let mut out = Vec::new();
        for w in list {
            match *w {
                "," | "." | "?" => out.push(Token::Pause {
                    ms: 250,
                    reason: "phrase break".into(),
                }),
                w => out.push(Token::Word {
                    text: w.into(),
                    symbols: vec!["a".into()],
                }),
            }
        }
        out
    }

    fn said<'a>(text: &'a str, tokens: &[&str]) -> Vec<&'a str> {
        let spans = word_spans(text, &words(tokens));
        for w in spans.windows(2) {
            assert!(w[0].start <= w[1].start && w[0].end <= w[1].end, "non-decreasing: {spans:?}");
        }
        spans.into_iter().map(|r| &text[r]).collect()
    }

    #[test]
    fn numbers_get_the_digits_they_stand_for() {
        // as the Italian module says it: stress marks added, 14 written out
        assert_eq!(said("Ho 14 anni.", &["hò", "quattórdici", "ànni", "."]), ["Ho", "14", "anni"]);
        // two words for one number share it
        assert_eq!(
            said("Nel 2024, a maggio", &["nèl", "duemìla", "ventiquàttro", ",", "a", "màggio"]),
            ["Nel", "2024", "2024", "a", "maggio"]
        );
    }

    #[test]
    fn elisions_and_abbreviations() {
        assert_eq!(said("L'amico è qui.", &["l'àmico", "è", "quì", "."]), ["L'amico", "è", "qui"]);
        assert_eq!(said("Il dott. Rossi", &["ìl", "dottór", "róssi"]), ["Il", "dott", "Rossi"]);
        // a rewritten word doesn't jump ahead to the same word later on
        assert_eq!(
            said("Il Sig. Rossi e il signor Bianchi", &["ìl", "signór", "róssi", "é", "ìl", "signór", "biànchi"]),
            ["Il", "Sig", "Rossi", "e", "il", "signor", "Bianchi"]
        );
    }

    #[test]
    fn case_accents_and_punctuation_are_ignored() {
        assert_eq!(said("PERCHÉ? Perché sì!", &["perché", "?", "perchè", "sì"]), ["PERCHÉ", "Perché", "sì"]);
        assert_eq!(said("Straße", &["strasse"]), ["Straße"]);
    }

    #[test]
    fn a_word_not_in_the_text_gets_the_stretch_between_its_neighbours() {
        assert_eq!(
            said("uno, 3,5 kg, due", &["uno", ",", "tré", "vìrgola", "cìnque", "chilogràmmi", ",", "due"]),
            ["uno", "3,5 kg", "3,5 kg", "3,5 kg", "3,5 kg", "due"]
        );
        // nothing between: an empty range where it would be
        let spans = word_spans("uno due", &words(&["uno", "xyz", "due"]));
        assert_eq!(spans, [0..3, 3..3, 4..7]);
        // at the very start and end
        assert_eq!(said("& uno", &["e", "uno"]), ["", "uno"]);
        assert_eq!(said("uno 5", &["uno", "cinque"]), ["uno", "5"]);
    }

    #[test]
    fn empty_text_and_no_words() {
        assert_eq!(word_spans("", &words(&["uno", "due"])), [0..0, 0..0]);
        assert!(word_spans("uno due", &[]).is_empty());
        assert!(word_spans("", &[]).is_empty());
    }

    #[test]
    fn multibyte_text() {
        let text = "Ζω στην Αθήνα, 東京 naïve";
        assert_eq!(
            said(text, &["ζω", "στην", "αθηνα", ",", "tokyo", "naive"]),
            ["Ζω", "στην", "Αθήνα", "東京", "naïve"]
        );
        let spans = word_spans(text, &words(&["ζω", "στην", "αθηνα", ",", "tokyo", "naive"]));
        // UTF-16: every one of these is one code unit
        let chars: Vec<char> = text.chars().collect();
        for r in spans {
            let u = utf16_range(text, r.clone());
            let s: String = chars[u].iter().collect();
            assert_eq!(s, &text[r]);
        }
        // astral characters take two units; ranges inside a char round down
        assert_eq!(utf16_range("a😀b", 5..6), 3..4);
        assert_eq!(utf16_range("a😀b", 2..99), 1..4);
        assert_eq!(utf16_range("a😀b", Range { start: 3, end: 1 }), 1..1);
    }

    #[test]
    fn chinese_words_inside_unspaced_text() {
        // as the Chinese front end splits it: dictionary words, Latin left out
        assert_eq!(
            said("我叫琳琳，我是Loquendo的女声录音员。", &["我", "叫", "琳琳", ",", "我", "是", "的", "女声", "录音员", "."]),
            ["我", "叫", "琳琳", "我", "是", "的", "女声", "录音员"]
        );
    }
}
