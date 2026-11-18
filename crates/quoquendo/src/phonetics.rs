//! Telling vowels from consonants in any voice's alphabet.
//!
//! Every Loquendo voice has its own symbols (Roberto: `T$`, `Gn`, `Gl`, `N`,
//! geminates `tt:`; Katrin: `Hk`, `Hut`, `OE:`, `y:`, diphthongs `HAI`...).
//! They share a few notations, which are peeled off before the lookup:
//! leading `` ` `` stressed, leading `H` devoiced/aspirated (or a diphthong),
//! trailing `:` long, trailing `)` or `~` nasalised.

/// The symbol without stress, length and nasal marks.
struct Peeled {
    nasal: bool,
    core: String,
}

fn peel(symbol: &str) -> Peeled {
    let s = symbol.trim_start_matches('`').trim_end_matches(':');
    // nasality is marked before the length mark (`o):`), so the colon goes first
    let nasal = s.ends_with(')') || s.ends_with('~');
    Peeled {
        nasal,
        core: s.trim_end_matches(')').trim_end_matches('~').to_string(),
    }
}

fn is_diphthong(s: &str) -> bool {
    matches!(s, "AI" | "AU" | "OI" | "Eh" | "Ey")
}

/// Reduce a symbol to the phone it sounds like, undoing the `H` marking.
fn base_phone(symbol: &str) -> String {
    let core = peel(symbol).core;
    if core.len() > 1 && core.starts_with('H') {
        let rest = &core[1..];
        if rest.len() == 2 && rest.starts_with('u') && !is_diphthong(rest) {
            // Hut, Hup, Huk, Hub, Hug, Hud: devoiced b/d/g/k/p/t
            return rest[1..].to_string();
        }
        return rest.to_string();
    }
    core
}

const VOWELS: [&str; 23] = [
    "a", "A", "e", "E", "i", "I", "o", "O", "u", "U", "y", "OE", "Oe", "AR", "Oa", "Ou", "AI",
    "AU", "OI", "EI", "HOI", "Eh", "Ey",
];

pub fn is_vowel(symbol: &str) -> bool {
    let p = peel(symbol);
    let core = base_phone(symbol);
    VOWELS.contains(&core.as_str()) || (p.nasal && !core.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undoes_the_h_marking() {
        assert_eq!(base_phone("Hk"), "k");
        assert_eq!(base_phone("Hut"), "t");
        assert_eq!(base_phone("HAI"), "AI");
        assert_eq!(base_phone("`HAU"), "AU");
        assert_eq!(base_phone("o):"), "o");
    }

    #[test]
    fn vowels_are_recognised_across_languages() {
        for s in ["a", "`e", "i:", "OE:", "y:", "HAI", "AR", "a):", "`Oa"] {
            assert!(is_vowel(s), "{s} should be a vowel");
        }
        for s in ["k", "tt", "Pf", "x", "Hn", "T$", "Dg"] {
            assert!(!is_vowel(s), "{s} should be a consonant");
        }
    }
}
