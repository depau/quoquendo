//! German letter-to-sound rules in the German voices' notation (Katrin's
//! phone table): long vowels `a:`…, short `a E I Oa U Ou Oe`, `HEh` for an
//! unstressed e, `AR` for r after a vowel, `c`/`x` for ch, `$` for sch and
//! st-/sp-, stress on the first syllable of the root. Final devoicing and
//! the glottal stop are applied across the sentence by `g2p`.

fn is_v(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'ä' | 'ö' | 'ü' | 'y')
}

const PREFIXES: &[&str] = &["be", "ge", "ver", "zer", "er", "ent", "emp"];

/// Words that keep their stress as a compound's first part (Haupt|bahn|hof,
/// Ab|stand); the parts after it lose theirs.
const HEADS: &[&str] = &[
    "außer", "haupt", "bahn", "ab", "auf", "aus", "ein", "vor", "zug", "rat", "post", "fahr", "flug", "bus", "stadt",
];

/// Loanwords and names whose stress the rules can't see, as Katrin says them.
const EXCEPTIONS: &[(&str, &str)] = &[("berlin", "b E AR l `i: n"), ("minuten", "m I n `u: t Hn"), ("minute", "m I n `u: t HEh")];

/// Muta cum liquida: a stop then r/l starts the next syllable (Ka|trin).
fn onset_pair(a: char, b: char) -> bool {
    matches!(a, 'p' | 't' | 'k' | 'b' | 'd' | 'g' | 'f') && matches!(b, 'r' | 'l')
}

/// The word's phones; compounds are split at a known first part and their
/// later parts said without stress.
pub fn transcribe(word: &str) -> Vec<String> {
    let s: String = word.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect();
    if let Some((_, t)) = EXCEPTIONS.iter().find(|(w, _)| *w == s) {
        return t.split(' ').map(str::to_string).collect();
    }
    let head = HEADS.iter().find(|h| {
        let Some(rest) = s.strip_prefix(*h) else { return false };
        rest.chars().count() >= 3 && rest.chars().next().map(|c| !is_v(c)).unwrap_or(false) && rest.chars().any(is_v)
    });
    if let Some(h) = head {
        let mut out = transcribe_part(h);
        // a part ends like a word: b/d/g are devoiced
        if let Some(l) = out.last_mut() {
            if let Some(v) = match l.as_str() {
                "b" => Some("p"),
                "d" => Some("t"),
                "g" => Some("k"),
                _ => None,
            } {
                *l = v.into();
            }
        }
        let rest = transcribe(&s[h.len()..]);
        if out.last().map(|l| l == "AR").unwrap_or(false) && rest.first().map(|r| r == "R").unwrap_or(false) {
            out.pop();
        }
        out.extend(rest.into_iter().map(|p| p.trim_start_matches('`').to_string()));
        return out;
    }
    transcribe_part(&s)
}

/// One part's phones with its stressed vowel marked; final stops are
/// written plain (b d g) and devoiced later.
fn transcribe_part(word: &str) -> Vec<String> {
    let w: Vec<char> = word.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect();
    let n = w.len();
    if n == 0 {
        return Vec::new();
    }
    let at = |i: isize| -> char {
        if i < 0 || i as usize >= n {
            '#'
        } else {
            w[i as usize]
        }
    };
    let s: String = w.iter().collect();
    // the unstressed prefix, if any (only before a consonant-vowel root)
    let prefix = PREFIXES
        .iter()
        .find(|p| {
            if !s.starts_with(*p) || n <= p.len() + 2 {
                return false;
            }
            // only before a root: a consonant then a vowel, or an onset cluster
            let r: String = s[p.len()..].chars().take(3).collect();
            let r1: Vec<char> = r.chars().collect();
            let after_onset = s[p.len()..].trim_start_matches(|c: char| !is_v(c));
            if matches!(after_onset, "e" | "en" | "er" | "el" | "es" | "et" | "em" | "ern") {
                return false;
            }
            (!is_v(r1[0]) && r1.len() > 1 && is_v(r1[1]))
                || ["sp", "st", "sch", "pr", "tr", "br", "fr", "gr", "kl", "bl", "pl", "fl", "gl", "dr", "kr"]
                    .iter()
                    .any(|c| r.starts_with(c))
        })
        .map(|p| p.len())
        .unwrap_or(0);
    // short in these whatever their spelling suggests
    let short_word = matches!(
        s.as_str(),
        "ab" | "an"
            | "am"
            | "das"
            | "dass"
            | "des"
            | "es"
            | "hat"
            | "im"
            | "in"
            | "ist"
            | "bin"
            | "mit"
            | "ob"
            | "um"
            | "von"
            | "man"
            | "bis"
            | "was"
            | "weg"
            | "ich"
            | "un"
    );
    let mut out: Vec<(String, bool)> = Vec::new(); // (phone, is a full vowel nucleus)
    let mut ier: Option<usize> = None; // a stressed -ier- (spazieren, Papier)
    let mut i = 0usize;
    // consonants after position k up to the next vowel
    let cons_after = |k: usize| -> usize { (k..n).take_while(|&j| !is_v(w[j])).count() };
    while i < n {
        let c = w[i];
        let (p, n1, n2) = (at(i as isize - 1), at(i as isize + 1), at(i as isize + 2));
        let rest: String = w[i..].iter().collect();
        let mut step = 1;
        if is_v(c) {
            // diphthongs and long spellings
            let (_two, sym): (bool, Option<&str>) = match (c, n1) {
                ('e', 'i') | ('a', 'i') | ('e', 'y') | ('a', 'y') => (true, Some("HAI")),
                ('a', 'u') => (true, Some("HAU")),
                ('e', 'u') | ('ä', 'u') => (true, Some("HOI")),
                ('i', 'e') => (true, Some("i:")),
                _ => (false, None),
            };
            if let Some(sym) = sym {
                if (c, n1, n2) == ('i', 'e', 'r') && out.iter().any(|(_, v)| *v) {
                    ier = Some(out.len());
                }
                out.push((sym.into(), true));
                step = 2;
                if n2 == 'h' {
                    step = 3;
                }
            } else {
                let long_sym = match c {
                    'a' => "a:",
                    'e' => "e:",
                    'i' => "i:",
                    'o' => "o:",
                    'u' => "u:",
                    'ä' => "E:",
                    'ö' => "OE:",
                    'ü' | 'y' => "y:",
                    _ => "a:",
                };
                let short_sym = match c {
                    'a' => "a",
                    'e' => "E",
                    'i' => "I",
                    'o' => "Oa",
                    'u' => "U",
                    'ä' => "E",
                    'ö' => "Oe",
                    'ü' | 'y' => "Ou",
                    _ => "a",
                };
                let doubled = n1 == c;
                let lengthened = n1 == 'h' && !is_v(n2) || doubled;
                let k = if doubled || n1 == 'h' { i + 2 } else { i + 1 };
                let cons = cons_after(k);
                // an e in an unstressed ending: -e, -en, -el, -er, -es, -et
                let tail: String = w[i..].iter().collect();
                let ig = c == 'i' && n1 == 'g' && out.iter().any(|(_, v)| *v) && matches!(&tail[2..], "" | "e" | "en" | "er" | "es" | "em");
                let ending_e = c == 'e'
                    && i > 0
                    && out.iter().any(|(_, v)| *v)
                    && matches!(tail.as_str(), "e" | "en" | "el" | "er" | "es" | "et" | "em" | "ern" | "ens" | "eln");
                if ig && tail == "ig" {
                    out.push(("I".into(), true));
                    out.push(("c".into(), false));
                    step = 2;
                } else if ig {
                    out.push(("I".into(), true));
                } else if ending_e {
                    match tail.as_str() {
                        "eln" => {
                            out.push(("Hl".into(), false));
                            out.push(("n".into(), false));
                            step = 3;
                        }
                        "er" | "ern" => {
                            out.push(("AR".into(), false));
                            step = tail.len().min(2);
                            if tail == "ern" {
                                out.push(("n".into(), false));
                                step = 3;
                            }
                        }
                        "en" => {
                            // syllabic n after an obstruent, else e n
                            let prev = out.last().map(|(s, _)| s.as_str()).unwrap_or("");
                            if prev == "b" {
                                out.push(("Hm".into(), false));
                            } else if matches!(prev, "t" | "k" | "g" | "f" | "p" | "d" | "s" | "z" | "Ts" | "$" | "x" | "c") {
                                out.push(("Hn".into(), false));
                            } else {
                                out.push(("HEh".into(), false));
                                out.push(("n".into(), false));
                            }
                            step = 2;
                        }
                        _ => {
                            out.push(("HEh".into(), false));
                        }
                    }
                } else if i < prefix && c == 'e' {
                    // ver-, zer-, er-: the e and r are one vowel
                    if n1 == 'r' {
                        if i == 0 {
                            out.push(("E".into(), false));
                        }
                        out.push(("AR".into(), false));
                        step = 2;
                    } else {
                        out.push(("HEh".into(), false));
                    }
                } else if c == 'a' && i + 1 == n && out.iter().any(|(_, v)| *v) {
                    out.push(("a".into(), true));
                } else if lengthened
                    || (cons == 1 && k + 1 < n && !matches!(w[k], 'x'))
                    || (cons == 2 && k + 2 < n && onset_pair(w[k], w[k + 1]))
                    || (cons == 1 && k + 1 >= n && !short_word && !out.iter().any(|(_, v)| *v))
                    || cons == 0 && (prefix == 0 || i >= prefix) && !short_word
                {
                    out.push((long_sym.into(), true));
                    if doubled || n1 == 'h' {
                        step = 2;
                    }
                } else {
                    out.push((short_sym.into(), true));
                }
            }
        } else {
            let push = |out: &mut Vec<(String, bool)>, s: &str| out.push((s.to_string(), false));
            if rest.starts_with("sch") {
                push(&mut out, "$");
                step = 3;
            } else if rest.starts_with("tsch") {
                push(&mut out, "T$");
                step = 4;
            } else if rest.starts_with("chs") {
                push(&mut out, "k");
                push(&mut out, "s");
                step = 3;
            } else if c == 'c' && n1 == 'h' {
                let back = out
                    .last()
                    .map(|(s, _)| matches!(s.trim_end_matches(':'), "a" | "o" | "u" | "Oa" | "U" | "HAU"))
                    .unwrap_or(false);
                push(&mut out, if back { "x" } else { "c" });
                step = 2;
            } else if c == 'd' && n1 == 't' {
                // Stadt: dt is t
            } else if c == 'c' && n1 == 'k' {
                push(&mut out, "k");
                step = 2;
            } else if c == 'p' && n1 == 'f' {
                push(&mut out, "Pf");
                step = 2;
            } else if c == 'p' && n1 == 'h' {
                push(&mut out, "f");
                step = 2;
            } else if c == 't' && n1 == 'z' {
                push(&mut out, "Ts");
                step = 2;
            } else if c == 'n' && n1 == 'g' {
                push(&mut out, "N");
                step = 2;
            } else if c == 'n' && n1 == 'k' {
                push(&mut out, "N");
            } else if c == 'q' && n1 == 'u' {
                push(&mut out, "k");
                push(&mut out, "v");
                step = 2;
            } else if c == 's' && (i == 0 || i == prefix) && matches!(n1, 't' | 'p') {
                push(&mut out, "$");
            } else if c == 's' && n1 == 's' || c == 'ß' {
                push(&mut out, "s");
                step = if c == 's' { 2 } else { 1 };
            } else if c == 's' && is_v(n1) && (i == 0 || is_v(p) || matches!(p, 'l' | 'n' | 'r' | 'm')) {
                push(&mut out, "z");
            } else if c == 'h' && i > 0 && is_v(p) {
                // silent after a vowel (lengthening handled above)
            } else if c == 'r' {
                // r after a vowel and before a consonant or the end: vocalic
                let after_vowel = out.last().map(|(_, v)| *v).unwrap_or(false);
                if after_vowel && !is_v(n1) {
                    push(&mut out, "AR");
                } else {
                    push(&mut out, "R");
                }
                if n1 == 'r' {
                    step = 2;
                }
            } else {
                let sym = match c {
                    'b' => "b",
                    'd' => "d",
                    'g' => "g",
                    'p' => "p",
                    't' => "t",
                    'k' => "k",
                    'f' => "f",
                    'v' => "f",
                    'w' => "v",
                    'z' => "Ts",
                    'j' => "j",
                    'l' => "l",
                    'm' => "m",
                    'n' => "n",
                    'h' => "h",
                    's' => "s",
                    'x' => "k",
                    'c' => "k",
                    _ => "",
                };
                if !sym.is_empty() {
                    push(&mut out, sym);
                    if c == 'x' {
                        push(&mut out, "s");
                    }
                }
                if n1 == c {
                    step = 2;
                }
            }
        }
        i += step;
    }
    // b d g before a voiceless consonant are p t k (gibt, liegt)
    for k in 0..out.len().saturating_sub(1) {
        if matches!(out[k + 1].0.as_str(), "t" | "s" | "k" | "p" | "f" | "$" | "Ts" | "Pf") {
            if let Some(v) = match out[k].0.as_str() {
                "b" => Some("p"),
                "d" => Some("t"),
                "g" => Some("k"),
                _ => None,
            } {
                out[k].0 = v.into();
            }
        }
    }
    // stress: the first full vowel after an unstressed prefix, or -ier-
    // with the vowels before it short (Spa|zier|gang)
    let first = ier.or_else(|| out.iter().position(|(_, v)| *v));
    if let Some(k) = ier {
        for (s, v) in out[..k].iter_mut().filter(|(_, v)| *v) {
            *v = false;
            *s = match s.as_str() {
                "a:" => "a",
                "i:" => "I",
                "o:" | "Oa" => "o",
                "u:" => "u",
                "e:" => "e",
                other => other,
            }
            .into();
        }
    }
    let mut syms: Vec<String> = out.iter().map(|(s, _)| s.clone()).collect();
    if let Some(k) = first {
        syms[k] = format!("`{}", syms[k]);
    }
    syms
}

#[cfg(test)]
mod tests {
    #[test]
    fn says_german_words() {
        for (w, want) in [
            ("schön", "$ `OE: n"),
            ("kinder", "k `I n d AR"),
            ("hamburg", "h `a m b U AR g"),
            ("reise", "R `HAI z HEh"),
            ("bahnhof", "b `a: n h o: f"),
            ("geben", "g `e: b Hm"),
            ("zwanzig", "Ts v `a n Ts I c"),
            ("katrin", "k `a: t R I n"),
            ("spaziergang", "$ p a Ts `i: AR g a N"),
        ] {
            assert_eq!(super::transcribe(w).join(" "), want, "{w}");
        }
    }
}
