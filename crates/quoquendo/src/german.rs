//! German letter-to-sound rules in the German voices' notation (Katrin's
//! phone table): long vowels `a:`…, short `a E I Oa U Ou Oe`, `HEh` for an
//! unstressed e, `AR` for r after a vowel, `c`/`x` for ch, `$` for sch and
//! st-/sp-, stress on the first syllable of the root. Final devoicing and
//! the glottal stop are applied across the sentence by `g2p`.

fn is_v(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'ä' | 'ö' | 'ü' | 'y')
}

const PREFIXES: &[&str] = &["be", "ge", "ver", "zer", "er", "ent", "emp"];

/// The word's phones with its stressed vowel marked; `Pfinal` stops are
/// written plain (b d g) and devoiced later.
pub fn transcribe(word: &str) -> Vec<String> {
    let w: Vec<char> = word.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect();
    let n = w.len();
    if n == 0 {
        return Vec::new();
    }
    let at = |i: isize| -> char { if i < 0 || i as usize >= n { '#' } else { w[i as usize] } };
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
            (!is_v(r1[0]) && r1.len() > 1 && is_v(r1[1]))
                || ["sp", "st", "sch", "pr", "tr", "br", "fr", "gr", "kl", "bl", "pl", "fl", "gl", "dr", "kr"].iter().any(|c| r.starts_with(c))
        })
        .map(|p| p.len())
        .unwrap_or(0);
    // short in these whatever their spelling suggests
    let short_word = matches!(s.as_str(), "ab" | "an" | "am" | "das" | "dass" | "des" | "es" | "hat" | "im" | "in" | "ist" | "bin" | "mit" | "ob" | "um" | "von" | "man" | "bis" | "was" | "weg" | "ich" | "un");
    let mut out: Vec<(String, bool)> = Vec::new(); // (phone, is a full vowel nucleus)
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
                let k = if doubled { i + 2 } else if n1 == 'h' { i + 2 } else { i + 1 };
                let cons = cons_after(k);
                // an e in an unstressed ending: -e, -en, -el, -er, -es, -et
                let tail: String = w[i..].iter().collect();
                let ending_e = c == 'e' && i > 0 && out.iter().any(|(_, v)| *v) && matches!(tail.as_str(), "e" | "en" | "el" | "er" | "es" | "et" | "em" | "ern" | "ens");
                if ending_e {
                    match tail.as_str() {
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
                            if matches!(prev, "t" | "k" | "g" | "f" | "p" | "b" | "d" | "s" | "z" | "Ts" | "$" | "x" | "c") {
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
                    out.push(("HEh".into(), false));
                } else if lengthened
                    || (cons == 1 && k + 1 < n && !matches!(w[k], 'x'))
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
                let back = out.last().map(|(s, _)| matches!(s.trim_end_matches(':'), "a" | "o" | "u" | "Oa" | "U" | "HAU")).unwrap_or(false);
                push(&mut out, if back { "x" } else { "c" });
                step = 2;
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
    // stress: the first full vowel after an unstressed prefix
    let first = out.iter().position(|(_, v)| *v);
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
        for (w, want) in [("schön", "$ `OE: n"), ("kinder", "k `I n d AR"), ("hamburg", "h `a m b U AR g"), ("reise", "R `HAI z HEh")] {
            assert_eq!(super::transcribe(w).join(" "), want, "{w}");
        }
    }
}
