//! French letter-to-sound rules in Bernard's notation, for words the
//! language's dictionary (`French.dbl`) doesn't have. The original's own
//! rules are code (FonTrasParola, LTTS7French.dll 0x1000f730) that is not
//! ported; these are the usual rules of French spelling, checked against the
//! original's transcriptions of a word list (MECHANICS.md §4).

fn is_v(c: char) -> bool {
    matches!(
        c,
        'a' | 'e' | 'i' | 'o' | 'u' | 'y' | 'à' | 'â' | 'ä' | 'é' | 'è' | 'ê' | 'ë' | 'î' | 'ï' | 'ô' | 'ö' | 'ù' | 'û' | 'ü' | 'œ' | 'æ'
    )
}

const SCHWA: &str = "HEh";

/// The word's phones, the last full vowel stressed.
pub fn transcribe(word: &str) -> Vec<String> {
    let w: Vec<char> = word.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect();
    let n = w.len();
    let at = |i: isize| -> char {
        if i < 0 || i as usize >= n {
            '#'
        } else {
            w[i as usize]
        }
    };
    let mut out: Vec<&str> = Vec::new();
    let mut i: usize = 0;
    // the word ends here, ignoring silent final letters
    let rest = |i: usize| -> String { w[i..].iter().collect() };
    while i < n {
        let c = w[i];
        let (p, n1, n2) = (at(i as isize - 1), at(i as isize + 1), at(i as isize + 2));
        let r = rest(i);
        let nasal_ok = |k: usize| {
            // a nasal vowel: followed by n/m that isn't doubled or before a vowel
            let m = at(k as isize);
            (m == 'n' || m == 'm') && !is_v(at(k as isize + 1)) && at(k as isize + 1) != m && at(k as isize + 1) != 'h'
        };
        let step: usize;
        match c {
            'â' => {
                out.push("Aa");
                step = 1;
            }
            'a' | 'à' | 'ä' => {
                if r.starts_with("aim") || r.starts_with("ain") {
                    if nasal_ok(i + 2) {
                        out.push("E)");
                        step = 3;
                    } else {
                        out.push("E");
                        step = 2;
                    }
                } else if r == "aient" {
                    out.push("E");
                    step = 5;
                } else if r.starts_with("ay") && is_v(n2) {
                    out.extend(["E", "j"]);
                    step = 2;
                } else if r.starts_with("aî") || r.starts_with("ai") || r.starts_with("ay") && !is_v(n2) {
                    out.push("E");
                    step = 2;
                } else if r.starts_with("au") {
                    out.push("o");
                    step = 2;
                } else if (n1 == 'n' || n1 == 'm') && nasal_ok(i + 1) {
                    out.push("Aa)");
                    step = 2;
                } else {
                    out.push("a");
                    step = 1;
                }
            }
            'e' => {
                if r.starts_with("eau") {
                    out.push("o");
                    step = 3;
                } else if r.starts_with("eu") || r.starts_with("eû") {
                    // closed (OE) at the end or before s/z/t, open (Oe) before a sounded consonant
                    let after = at(i as isize + 2);
                    let end = i + 2 >= n || (matches!(after, 's' | 'x' | 't' | 'z') && i + 3 >= n) || after == 's' && is_v(at(i as isize + 3));
                    out.push(if end { "OE" } else { "Oe" });
                    step = 2;
                } else if r.starts_with("ei") {
                    if nasal_ok(i + 2) {
                        out.push("E)");
                        step = 3;
                    } else {
                        out.push("E");
                        step = 2;
                    }
                } else if (n1 == 'n' || n1 == 'm') && nasal_ok(i + 1) {
                    // -ent of verbs is silent after a consonant; -ien is j E)
                    if p == 'i' {
                        out.push("E)");
                        step = 2;
                    } else if r == "ent" && i >= 2 && (is_v(p) || !w[..i].iter().collect::<String>().ends_with("m")) {
                        step = 3;
                    } else {
                        out.push("Aa)");
                        step = 2;
                    }
                } else if i + 1 == n || (i + 2 == n && n1 == 's') {
                    // final e / es: silent (spoken in one-syllable words)
                    if out
                        .iter()
                        .all(|s| !matches!(*s, "a" | "e" | "E" | "i" | "o" | "Oa" | "u" | "y" | "OE" | "Oe" | "Aa)" | "E)" | "o)" | "Oe)"))
                    {
                        out.push(if n1 == 's' { "e" } else { SCHWA });
                    }
                    step = n - i;
                } else if (r == "er" || r == "ers" || r == "ez" || r == "et") && i > 0 {
                    out.push(if r == "et" { "E" } else { "e" });
                    step = r.chars().count();
                } else if i == 0 && n1 == n2 && !is_v(n1) {
                    out.push("e");
                    step = 1;
                } else if !is_v(n1)
                    && n1 != '#'
                    && (!is_v(n2) || n1 == n2)
                    && !(matches!(n1, 'b' | 'c' | 'd' | 'f' | 'g' | 'p' | 't' | 'v') && matches!(n2, 'r' | 'l'))
                    && (n2 != 'h')
                {
                    // closed syllable: E
                    out.push("E");
                    step = 1;
                } else if i == 0 && n1 == n2 && !is_v(n1) {
                    out.push("e");
                    step = 1;
                } else if n1 == 'x' {
                    out.push("E");
                    step = 1;
                } else if i == 0 && !is_v(n1) && is_v(n2) {
                    out.push("e");
                    step = 1;
                } else {
                    out.push(SCHWA);
                    step = 1;
                }
            }
            'é' => {
                out.push("e");
                step = 1;
            }
            'è' | 'ê' | 'ë' => {
                out.push("E");
                step = 1;
            }
            'i' | 'î' | 'ï' => {
                if (n1 == 'n' || n1 == 'm') && nasal_ok(i + 1) {
                    out.push("E)");
                    step = 2;
                } else if r.starts_with("ier") && is_v(at(i as isize + 3)) && i > 0 {
                    // future/conditional: the e between i and r falls
                    out.push("i");
                    step = 2;
                } else if is_v(n1) && c == 'i' && i > 0 && !(n1 == 'e' && matches!(&r[1..], "e" | "es" | "ent")) && !r.starts_with("ill") {
                    // after a consonant + r/l cluster the i stays and glides
                    if matches!(p, 'r' | 'l') && i >= 2 && !is_v(at(i as isize - 2)) && at(i as isize - 2) != p {
                        out.push("i");
                    }
                    out.push("j");
                    step = 1;
                } else if r.starts_with("ill")
                    && i > 0
                    && !w[..i].iter().collect::<String>().ends_with('v')
                    && !w[..i].iter().collect::<String>().ends_with("m")
                    && !w[..i].iter().collect::<String>().ends_with("tranqu")
                {
                    if out
                        .last()
                        .map(|s| matches!(*s, "a" | "E" | "e" | "u" | "Oe" | "OE" | "o" | "Oa" | "Hy"))
                        .unwrap_or(false)
                        && is_v(p)
                    {
                        out.push("j");
                    } else {
                        out.push("i");
                        out.push("j");
                    }
                    step = 3;
                } else if r == "il" && i > 0 && is_v(p) {
                    out.push("j");
                    step = 2;
                } else {
                    out.push("i");
                    step = 1;
                }
            }
            'o' | 'ô' | 'ö' => {
                if r.starts_with("oin") && nasal_ok(i + 2) {
                    out.extend(["w", "E)"]);
                    step = 3;
                } else if r.starts_with("oy") && is_v(n2) {
                    out.extend(["w", "a", "j"]);
                    step = 2;
                } else if r.starts_with("oi") || r.starts_with("oy") {
                    out.extend(["w", "a"]);
                    step = 2;
                } else if r.starts_with("ou") || r.starts_with("où") || r.starts_with("oû") {
                    if is_v(n2) && !r[2..].starts_with("ill") {
                        out.push("w");
                    } else {
                        out.push("u");
                    }
                    step = 2;
                } else if (n1 == 'n' || n1 == 'm') && nasal_ok(i + 1) {
                    out.push("o)");
                    step = 2;
                } else if c == 'ô' {
                    out.push("o");
                    step = 1;
                } else {
                    // closed o at the end of the sounded word or before s/z, open elsewhere
                    let tail: String = w[i + 1..].iter().collect();
                    let open_end = tail.is_empty() || matches!(tail.as_str(), "s" | "t" | "ts" | "x" | "p" | "ps");
                    out.push(if open_end || (n1 == 's' && is_v(n2)) && tail.len() <= 2 { "o" } else { "Oa" });
                    step = 1;
                }
            }
            'œ' => {
                out.push("Oe");
                step = if n1 == 'u' { 2 } else { 1 };
            }
            'u' | 'ù' | 'û' | 'ü' => {
                if (n1 == 'n' || n1 == 'm') && nasal_ok(i + 1) {
                    out.push("Oe)");
                    step = 2;
                } else if n1 == 'y' && is_v(n2) {
                    out.extend(["Hy", "i", "j"]);
                    step = 2;
                } else if is_v(n1) && p != 'q' && !(p == 'g' && matches!(n1, 'e' | 'i' | 'é' | 'è' | 'ê')) {
                    out.push("Hy");
                    step = 1;
                } else {
                    out.push("y");
                    step = 1;
                }
            }
            'y' => {
                if is_v(n1) {
                    out.push("j");
                } else if (n1 == 'n' || n1 == 'm') && nasal_ok(i + 1) {
                    out.push("E)");
                    step = 2;
                    i += step;
                    continue;
                } else {
                    out.push("i");
                }
                step = 1;
            }
            _ => {
                // consonants; the last ones of a word are mostly silent (not c, r, f, l)
                let last = i + 1 == n || (i + 2 == n && n1 == 's' && c != 's');
                if last && !matches!(c, 'c' | 'r' | 'f' | 'l' | 'q' | 'k') {
                    if c == 'x' && matches!(word, "six" | "dix") {
                        out.push("s");
                    }
                    step = n - i;
                    i += step;
                    continue;
                }
                match (c, n1) {
                    ('c', 'q') => {
                        step = 1;
                    }
                    ('s', 'h') => {
                        out.push("$");
                        step = 2;
                    }
                    ('c', 'h') => {
                        out.push(if matches!(n2, 'r' | 'l') { "k" } else { "$" });
                        step = 2;
                    }
                    ('b', 's' | 't') => {
                        out.push("p");
                        step = 1;
                    }
                    ('c', 'e' | 'i' | 'y' | 'é' | 'è' | 'ê') => {
                        out.push("s");
                        step = 1;
                    }
                    ('c', 'c') if matches!(n2, 'e' | 'i' | 'é' | 'è') => {
                        out.extend(["k", "s"]);
                        step = 2;
                    }
                    ('ç', _) => {
                        out.push("s");
                        step = 1;
                    }
                    ('g', 'n') => {
                        out.push("Gn");
                        step = 2;
                    }
                    ('g', 'u') if matches!(n2, 'e' | 'i' | 'é' | 'è' | 'ê') => {
                        out.push("g");
                        step = 2;
                    }
                    ('g', 'e' | 'i' | 'y' | 'é' | 'è' | 'ê') => {
                        out.push("Zg");
                        step = 1;
                    }
                    ('p', 'h') => {
                        out.push("f");
                        step = 2;
                    }
                    ('t', 'h') => {
                        out.push("tt");
                        step = 2;
                    }
                    ('q', 'u') => {
                        out.push("k");
                        step = 2;
                    }
                    ('s', 's') => {
                        out.push("s");
                        step = 2;
                    }
                    ('s', _) if is_v(p) && is_v(n1) => {
                        out.push("z");
                        step = 1;
                    }
                    ('t', 'i') if is_v(n2) && i > 0 && p != 's' && p != 'x' => {
                        out.push("s");
                        step = 1;
                    }
                    ('x', _) => {
                        if i <= 1 && p == 'e' && is_v(n1) {
                            out.extend(["g", "z"]);
                        } else {
                            out.extend(["k", "s"]);
                        }
                        step = 1;
                    }
                    ('h', _) => step = 1,
                    _ => {
                        let s = match c {
                            'b' => "b",
                            'c' | 'k' | 'q' => "k",
                            'd' => "dd",
                            'f' => "f",
                            'g' => "g",
                            'j' => "Zg",
                            'l' => "l",
                            'm' => "m",
                            'n' => "n",
                            'p' => "p",
                            'r' => "HRF",
                            's' => "s",
                            't' => "tt",
                            'v' => "v",
                            'w' => "w",
                            'z' => "z",
                            _ => "",
                        };
                        if !s.is_empty() {
                            out.push(s);
                        }
                        // a doubled consonant is said once
                        step = if n1 == c { 2 } else { 1 };
                    }
                }
                // final -er after a consonant cluster etc. handled above
            }
        }
        i += step;
    }
    let full = |s: &str| {
        matches!(
            s,
            "a" | "Aa" | "e" | "E" | "i" | "o" | "Oa" | "u" | "y" | "OE" | "Oe" | "Aa)" | "E)" | "o)" | "Oe)" | "Hy"
        )
    };
    let mut kept: Vec<&str> = Vec::with_capacity(out.len());
    for (k, sym) in out.iter().enumerate() {
        if *sym == SCHWA && k + 1 < out.len() && k >= 2 && !full(out[k - 1]) && full(out[k - 2]) && out[k - 1] != "w" {
            continue;
        }
        kept.push(sym);
    }
    let out = kept;
    let mut syms: Vec<String> = out.into_iter().map(str::to_string).collect();
    // stress the last full vowel
    if let Some(k) = syms.iter().rposition(|s| {
        matches!(
            s.as_str(),
            "a" | "Aa" | "e" | "E" | "i" | "o" | "Oa" | "u" | "y" | "OE" | "Oe" | "Aa)" | "E)" | "o)" | "Oe)"
        )
    }) {
        syms[k] = format!("`{}", syms[k]);
    }
    syms
}

#[cfg(test)]
mod tests {
    /// Words the original transcribes this way (Bernard).
    #[test]
    fn says_french_like_the_original() {
        for (w, want) in [
            ("maison", "m E z `o)"),
            ("chanter", "$ Aa) tt `e"),
            ("enfant", "Aa) f `Aa)"),
            ("fille", "f `i j"),
            ("oiseau", "w a z `o"),
            ("nation", "n a s j `o)"),
            ("parlais", "p a HRF l `E"),
            ("beaucoup", "b o k `u"),
        ] {
            assert_eq!(super::transcribe(w).join(" "), want, "{w}");
        }
    }
}
