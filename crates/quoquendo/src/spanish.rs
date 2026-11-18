//! Spanish (Castilian) letter-to-sound rules in the Spanish voices'
//! notation (Jorge's phone table): written accents or the usual stress rule,
//! i/u gliding next to another vowel, c/z as `Th`, ll as `Gl`, ñ as `Gn`;
//! b/d/g are written as stops here and softened by `g2p` across the
//! sentence (`spanish_connected_speech`), as the original does.

fn vowel(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'á' | 'é' | 'í' | 'ó' | 'ú' | 'ü')
}

fn plain(c: char) -> char {
    match c {
        'á' => 'a',
        'é' => 'e',
        'í' => 'i',
        'ó' => 'o',
        'ú' | 'ü' => 'u',
        c => c,
    }
}

/// The word's phones with its stressed vowel marked.
pub fn transcribe(word: &str) -> Vec<String> {
    let w: Vec<char> = word.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect();
    let n = w.len();
    if n == 0 {
        return Vec::new();
    }
    let at = |i: isize| -> char { if i < 0 || i as usize >= n { '#' } else { w[i as usize] } };
    // syllable nuclei: vowels, with unaccented i/u next to another vowel gliding
    let mut out: Vec<(String, bool, bool)> = Vec::new(); // (phone, is nucleus, written accent)
    let mut i = 0usize;
    while i < n {
        let c = w[i];
        let (p, n1, n2) = (at(i as isize - 1), at(i as isize + 1), at(i as isize + 2));
        let mut push = |s: &str| out.push((s.to_string(), false, false));
        let mut step = 1;
        match c {
            'a' | 'e' | 'o' | 'á' | 'é' | 'ó' | 'í' | 'ú' => {
                out.push((plain(c).to_string(), true, matches!(c, 'á' | 'é' | 'ó' | 'í' | 'ú')));
            }
            'i' | 'u' => {
                // rising diphthongs glide (bue-, cia-); after a vowel (au, oi) i/u stay vowels
                let glide = vowel(n1) && n1 != c;
                if glide && !(c == 'u' && (p == 'q' || p == 'g') && matches!(n1, 'e' | 'i' | 'é' | 'í')) {
                    out.push((if c == 'i' { "j" } else { "w" }.into(), false, false));
                } else if c == 'u' && (p == 'q' || p == 'g') && matches!(n1, 'e' | 'i' | 'é' | 'í') {
                    // the silent u of que, gui
                } else {
                    out.push((c.to_string(), true, false));
                }
            }
            'ü' => out.push(("w".into(), false, false)),
            'y' => {
                if i + 1 == n {
                    out.push(("i".into(), !vowel(p), false));
                } else if vowel(n1) {
                    push("J");
                } else {
                    out.push(("i".into(), true, false));
                }
            }
            'b' | 'v' | 'w' => push("b"),
            'c' => {
                if n1 == 'h' {
                    push("T$");
                    step = 2;
                } else if matches!(n1, 'e' | 'i' | 'é' | 'í') {
                    push("Th");
                } else {
                    push("k");
                }
            }
            'z' => push("Th"),
            'q' => push("k"),
            'k' => push("k"),
            'd' => push("dd"),
            't' => push("tt"),
            'p' => push("p"),
            'f' => push("f"),
            'g' => {
                if matches!(n1, 'e' | 'i' | 'é' | 'í') {
                    push("x");
                } else {
                    push("g");
                }
            }
            'j' => push("x"),
            'h' => {}
            'l' => {
                if n1 == 'l' {
                    push("Gl");
                    step = 2;
                } else {
                    push("l");
                }
            }
            'm' => push("m"),
            'n' => push("n"),
            'ñ' => push("Gn"),
            'r' => {
                if n1 == 'r' {
                    push("r:");
                    step = 2;
                } else if i == 0 || matches!(p, 'n' | 'l' | 's') {
                    push("r:");
                } else {
                    push("hr");
                }
            }
            's' => push("s"),
            'x' => {
                push("k");
                push("s");
            }
            _ => {}
        }
        let _ = n2;
        i += step;
    }
    // stress: a written accent, else the last syllable for words ending in a
    // consonant other than n/s, else the one before
    let nuclei: Vec<usize> = (0..out.len()).filter(|&k| out[k].1).collect();
    let stressed = if let Some(&k) = nuclei.iter().find(|&&k| out[k].2) {
        Some(k)
    } else if nuclei.is_empty() {
        None
    } else {
        let last = *w.last().unwrap();
        let ends_open = vowel(last) || matches!(last, 'n' | 's') || (last == 'y' && false);
        if ends_open && nuclei.len() >= 2 {
            Some(nuclei[nuclei.len() - 2])
        } else {
            Some(*nuclei.last().unwrap())
        }
    };
    out.into_iter()
        .enumerate()
        .map(|(k, (s, _, _))| if Some(k) == stressed { format!("`{s}") } else { s })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn says_spanish_words() {
        for (w, want) in [("destino", "dd e s tt `i n o"), ("llamo", "Gl `a m o"), ("ciudad", "Th j u dd `a dd"), ("país", "p a `i s"), ("españa", "e s p `a Gn a")] {
            assert_eq!(super::transcribe(w).join(" "), want, "{w}");
        }
    }
}
