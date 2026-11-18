//! Text → symbols, for as many languages as the installed voices cover.
//!
//! The language is not guessed from the text: it comes from the voice's own
//! descriptor (`MotherTongue` / `MotherTongueAliases` in `<Voice>.vde`), e.g.
//! `Italian` / `Italian,it-IT,it` for Roberto or `German` / `German,de-DE,de`
//! for Katrin. Each front end writes into the symbol alphabet that language's
//! Loquendo voices declare in their `.sde` — and those alphabets really differ:
//! Italian has `Ts Gn Gl N M` and geminates `p: tt:`; German has `Hk Hut ph
//! Pf x c R OE: y: HAI` and `R` for /r/. A language with no rules here falls
//! back to a generic grapheme map (after transliterating Cyrillic and Greek),
//! so it is still audible.
//!
//! Every rule below is **estimated**: the engine's own letter-to-sound rules
//! live in `data/languages/<Lang>/<Lang>.lex`, which is encrypted (it starts
//! `CA FE DB 60` in both Italian and German), so nothing here is read from it.

use crate::phonetics::is_vowel;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Language {
    Italian,
    English,
    German,
    French,
    Spanish,
    Portuguese,
    Catalan,
    Dutch,
    Swedish,
    Danish,
    Norwegian,
    Finnish,
    Polish,
    Russian,
    Greek,
    Turkish,
    Generic,
}

impl Language {
    pub fn from_voice(mother_tongue: &str, aliases: &str) -> Language {
        let hay = format!("{} {}", mother_tongue, aliases).to_lowercase();
        let has = |n: &str| hay.contains(n);
        if has("italian") || has("it-it") {
            Language::Italian
        } else if has("german") || has("de-de") {
            Language::German
        } else if has("english") || has("en-us") || has("en-gb") || has("en-au") {
            Language::English
        } else if has("french") || has("fr-fr") || has("fr-ca") {
            Language::French
        } else if has("spanish") || has("es-es") || has("es-mx") || has("castilian") {
            Language::Spanish
        } else if has("brazilian") || has("portuguese") || has("pt-br") || has("pt-pt") {
            Language::Portuguese
        } else if has("catalan") || has("valencian") || has("ca-es") {
            Language::Catalan
        } else if has("dutch") || has("nl-nl") || has("flemish") {
            Language::Dutch
        } else if has("swedish") || has("sv-se") {
            Language::Swedish
        } else if has("danish") || has("da-dk") {
            Language::Danish
        } else if has("norwegian") || has("nb-no") || has("no-no") {
            Language::Norwegian
        } else if has("finnish") || has("fi-fi") {
            Language::Finnish
        } else if has("polish") || has("pl-pl") {
            Language::Polish
        } else if has("russian") || has("ru-ru") {
            Language::Russian
        } else if has("greek") || has("el-gr") {
            Language::Greek
        } else if has("turkish") || has("tr-tr") {
            Language::Turkish
        } else {
            Language::Generic
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Language::Italian => "Italian",
            Language::English => "English",
            Language::German => "German",
            Language::French => "French",
            Language::Spanish => "Spanish",
            Language::Portuguese => "Portuguese",
            Language::Catalan => "Catalan",
            Language::Dutch => "Dutch",
            Language::Swedish => "Swedish",
            Language::Danish => "Danish",
            Language::Norwegian => "Norwegian",
            Language::Finnish => "Finnish",
            Language::Polish => "Polish",
            Language::Russian => "Russian",
            Language::Greek => "Greek",
            Language::Turkish => "Turkish",
            Language::Generic => "generic",
        }
    }

    pub const ALL: [Language; 17] = [
        Language::Italian,
        Language::English,
        Language::German,
        Language::French,
        Language::Spanish,
        Language::Portuguese,
        Language::Catalan,
        Language::Dutch,
        Language::Swedish,
        Language::Danish,
        Language::Norwegian,
        Language::Finnish,
        Language::Polish,
        Language::Russian,
        Language::Greek,
        Language::Turkish,
        Language::Generic,
    ];

    pub fn sample_sentence(self) -> &'static str {
        match self {
            Language::Italian => {
                "Il mio nome è Roberto: sono una voce maschile e la mia madrelingua è l'italiano."
            }
            Language::German => {
                "Ich heiße Katrin. Ich bin die weibliche Stimme von Loquendo, meine Muttersprache ist Deutsch."
            }
            Language::English => {
                "Hello. I am a Loquendo voice, and this is the text you asked me to read aloud."
            }
            Language::French => {
                "Bonjour. Je suis une voix de Loquendo, et voici le texte que vous m'avez donné."
            }
            Language::Spanish => {
                "Hola. Soy una voz de Loquendo y este es el texto que me has pedido leer."
            }
            Language::Portuguese => {
                "Olá. Eu sou uma voz do Loquendo e este é o texto que você pediu para eu ler."
            }
            Language::Catalan => {
                "Hola. Sóc una veu de Loquendo i aquest és el text que m'has demanat llegir."
            }
            Language::Dutch => {
                "Hallo. Ik ben een stem van Loquendo en dit is de tekst die je mij hebt gegeven."
            }
            Language::Swedish => {
                "Hej. Jag är en röst från Loquendo och detta är texten du bad mig läsa."
            }
            Language::Danish => {
                "Hej. Jeg er en stemme fra Loquendo, og dette er teksten du bad mig læse."
            }
            Language::Norwegian => {
                "Hei. Jeg er en stemme fra Loquendo, og dette er teksten du ba meg lese."
            }
            Language::Finnish => {
                "Hei. Olen Loquendo-ääni, ja tämä on teksti, jonka pyysit minua lukemaan."
            }
            Language::Polish => {
                "Dzień dobry. Jestem głosem Loquendo i to jest tekst, który mam przeczytać."
            }
            Language::Russian => "Привет. Я голос Loquendo, и это текст, который вы попросили прочитать.",
            Language::Greek => "Γεια σας. Είμαι φωνή του Loquendo και αυτό είναι το κείμενο που διαβάζω.",
            Language::Turkish => "Merhaba. Ben bir Loquendo sesiyim ve bu okumamı istediğiniz metin.",
            Language::Generic => "Hello. This voice is driven by the Loquendo pipeline.",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Word {
        text: String,
        symbols: Vec<String>,
    },
    Pause {
        ms: u32,
        reason: String,
    },
}

impl Token {
    pub fn symbols(&self) -> &[String] {
        match self {
            Token::Word { symbols, .. } => symbols,
            Token::Pause { .. } => &[],
        }
    }
}

// --- character helpers ------------------------------------------------------

const ACCENTS: [(char, char); 24] = [
    ('à', 'a'),
    ('á', 'a'),
    ('â', 'a'),
    ('ä', 'a'),
    ('å', 'a'),
    ('ã', 'a'),
    ('è', 'e'),
    ('é', 'e'),
    ('ê', 'e'),
    ('ë', 'e'),
    ('ì', 'i'),
    ('í', 'i'),
    ('î', 'i'),
    ('ï', 'i'),
    ('ò', 'o'),
    ('ó', 'o'),
    ('ô', 'o'),
    ('ö', 'o'),
    ('õ', 'o'),
    ('ø', 'o'),
    ('ù', 'u'),
    ('ú', 'u'),
    ('ü', 'u'),
    ('ç', 'c'),
];

pub fn strip_accent(c: char) -> char {
    for (a, b) in ACCENTS {
        if c == a {
            return b;
        }
    }
    c
}

/// Lower-case letters only; accents are kept so each language can read them.
pub fn bare_word(w: &str) -> Vec<char> {
    w.chars()
        .map(|c| c.to_lowercase().next().unwrap_or(c))
        .filter(|c| c.is_alphabetic())
        .collect()
}

fn starts(w: &[char], i: usize, pat: &str) -> bool {
    pat.chars()
        .enumerate()
        .all(|(k, c)| w.get(i + k) == Some(&c))
}

fn plain(w: &[char], i: usize) -> Option<char> {
    w.get(i).map(|c| strip_accent(*c))
}

fn is_vowel_char(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')
}

/// Transliterate scripts we have no rules for into Latin letters, so Cyrillic
/// and Greek text still reaches the syllabary instead of vanishing.
fn transliterate(w: &[char]) -> Vec<char> {
    let mut out = Vec::with_capacity(w.len());
    for c in w {
        let mapped: String = match c {
            // Cyrillic
            'а' => "a".into(),
            'б' => "b".into(),
            'в' => "v".into(),
            'г' => "g".into(),
            'д' => "d".into(),
            'е' => "e".into(),
            'ё' => "o".into(),
            'ж' => "Zg".into(),
            'з' => "z".into(),
            'и' => "i".into(),
            'й' => "j".into(),
            'к' => "k".into(),
            'л' => "l".into(),
            'м' => "m".into(),
            'н' => "n".into(),
            'о' => "o".into(),
            'п' => "p".into(),
            'р' => "r".into(),
            'с' => "s".into(),
            'т' => "t".into(),
            'у' => "u".into(),
            'ф' => "f".into(),
            'х' => "x".into(),
            'ц' => "Ts".into(),
            'ч' => "T$".into(),
            'ш' => "$".into(),
            'щ' => "$".into(),
            'ъ' => "".into(),
            'ы' => "i".into(),
            'ь' => "".into(),
            'э' => "e".into(),
            'ю' => "u".into(),
            'я' => "a".into(),
            // Greek
            'α' => "a".into(),
            'β' => "v".into(),
            'γ' => "g".into(),
            'δ' => "d".into(),
            'ε' => "e".into(),
            'ζ' => "z".into(),
            'η' => "i".into(),
            'θ' => "th".into(),
            'ι' => "i".into(),
            'κ' => "k".into(),
            'λ' => "l".into(),
            'μ' => "m".into(),
            'ν' => "n".into(),
            'ξ' => "ks".into(),
            'ο' => "o".into(),
            'π' => "p".into(),
            'ρ' => "r".into(),
            'σ' | 'ς' => "s".into(),
            'τ' => "t".into(),
            'υ' => "i".into(),
            'φ' => "f".into(),
            'χ' => "x".into(),
            'ψ' => "ps".into(),
            'ω' => "o".into(),
            other => strip_accent(*other).to_string(),
        };
        out.extend(mapped.chars());
    }
    out
}

// --- entry point ------------------------------------------------------------

// --- English connected speech -------------------------------------------------

/// Weak forms the original engine uses for English function words inside a
/// sentence (read from its own transcriptions with Allison, MECHANICS.md §9);
/// the second form is the one before a vowel.
fn english_weak_form(word: &str) -> Option<(&'static str, &'static str)> {
    Some(match word {
        "the" => ("Dh HEh", "Dh i"),
        "a" => ("HEh", "HEh"),
        "an" => ("HEh n", "HEh n"),
        "of" => ("HEh v", "HEh v"),
        "to" => ("th HEh", "th u"),
        "and" => ("HEh n d", "HEh n d"),
        "you" => ("j u", "j u"),
        "is" => ("I z", "I z"),
        "at" => ("HEh Hut", "HEh th"),
        "for" => ("f HR", "f HR"),
        "was" => ("w HEh z", "w HEh z"),
        "has" => ("h HEh z", "h HEh z"),
        "am" => ("HEh m", "HEh m"),
        "i" => ("HAI", "HAI"),
        "my" => ("m HAI", "m HAI"),
        "we" => ("w i", "w i"),
        "she" => ("$ i", "$ i"),
        "he" => ("h i", "h i"),
        "our" => ("Aa: HR", "Aa: HR"),
        "will" => ("w Hl", "w Hl"),
        "with" => ("w I Th", "w I Th"),
        "would" => ("w U d", "w U d"),
        "while" => ("w HAI Hl", "w HAI Hl"),
        _ => return None,
    })
}

/// What the original does between English words (Allison's transcriptions,
/// oracle/bench-en.txt): function words take their weak forms; inside words
/// `s t` is said `s d`; a word-final t/k/p/d before another word's consonant
/// is unreleased (`Hut`, `Huk`, `Hup`, `Hud`), but `-nd` loses its d before
/// `the`-like sounds (`mind the` m `HAI n), a final z before s becomes s
/// (`please stand`); before `you`/`your` t becomes `T$` and d `Dg`;
/// `to` before a vowel after a vowel or n is flapped (`train to arrive` …n
/// HTh u). No linking sounds are added between words.
fn english_connected_speech(tokens: &mut [Token]) {
    let words: Vec<usize> = (0..tokens.len()).filter(|&i| matches!(tokens[i], Token::Word { .. })).collect();
    for &i in &words {
        if let Token::Word { symbols, .. } = &mut tokens[i] {
            for k in 1..symbols.len().saturating_sub(1) {
                if symbols[k - 1] == "s" && matches!(symbols[k].as_str(), "t" | "th") {
                    symbols[k] = "d".into();
                }
            }
        }
    }
    let starts_vowel = |t: &Token| t.symbols().first().map(|s| is_vowel(s)).unwrap_or(false);
    let mut prev_end: Option<String> = None;
    for (n, &i) in words.iter().enumerate() {
        let next = words.get(n + 1).copied();
        // joined to the next word only across a plain word gap
        let joined = next.map(|j| tokens[i + 1..j].iter().all(|t| matches!(t, Token::Pause { reason, .. } if reason == "word gap"))).unwrap_or(false);
        let next_vowel = joined && next.map(|j| starts_vowel(&tokens[j])).unwrap_or(false);
        let next_you = joined
            && next.map(|j| matches!(&tokens[j], Token::Word { text, .. } if text == "you" || text == "your")).unwrap_or(false);
        let after_voice = prev_end.as_deref().map(|e| is_vowel(e) || e == "n").unwrap_or(false);
        let next_first = next.and_then(|j| tokens[j].symbols().first().cloned()).filter(|_| joined);
        if let Token::Word { text, symbols } = &mut tokens[i] {
            if let Some((weak, before_vowel)) = english_weak_form(text) {
                let form = if next_vowel { before_vowel } else { weak };
                *symbols = form.split(' ').map(str::to_string).collect();
                if text == "to" && next_vowel && after_voice {
                    symbols[0] = "HTh".into();
                }
            }
            if joined && !next_vowel {
                let k = symbols.len();
                if next_you && k > 0 && matches!(symbols[k - 1].as_str(), "th" | "t" | "d") {
                    symbols[k - 1] = if symbols[k - 1] == "d" { "Dg" } else { "T$" }.into();
                } else if k >= 2 && symbols[k - 1] == "d" && symbols[k - 2] == "n" && matches!(next_first.as_deref(), Some("Dh" | "Th")) {
                    symbols.pop();
                } else if k > 0 && symbols[k - 1] == "z" && next_first.as_deref() == Some("s") {
                    symbols[k - 1] = "s".into();
                } else if let Some(last) = symbols.last_mut() {
                    let unreleased = match last.as_str() {
                        "th" | "t" => Some("Hut"),
                        "kh" | "k" => Some("Huk"),
                        "ph" | "p" => Some("Hup"),
                        "d" => Some("Hud"),
                        _ => None,
                    };
                    if let Some(u) = unreleased {
                        *last = u.to_string();
                    }
                }
            }
            prev_end = if joined { symbols.last().cloned() } else { None };
        }
    }
}

// --- the language's own data: dictionary and letter-to-sound model ---------

/// What a language's installer files give the front end: the dictionary
/// (`<MotherTongue>.dbl`, SQLite table `lessico`: spelling, pronunciation…)
/// and the letter-to-sound model (`<MotherTongue>.gpr`, `gpr`), whose
/// feature words are named by the voice's phone table.
use std::collections::HashMap;

pub struct LanguageData {
    dictionary: HashMap<String, Vec<String>>,
    gpr: Option<crate::gpr::Gpr>,
    names: HashMap<u32, String>,
}

static LANGUAGE_DATA: std::sync::RwLock<Option<HashMap<Language, std::sync::Arc<LanguageData>>>> = std::sync::RwLock::new(None);

/// Read a language's dictionary and letter-to-sound model from its files
/// (any may be missing) and use them from now on. `phones` is the voice's
/// phone table as (symbol, engine feature word), `synth::PhoneTables`.
/// Returns the number of dictionary words.
pub fn load_language_data(lang: Language, dbl: Option<&[u8]>, gpr: Option<&[u8]>, phones: &[(String, u32)]) -> usize {
    let mut dictionary = HashMap::new();
    if let Some(rows) = dbl.and_then(|d| crate::sqlite::read_table(d, "lessico").ok()) {
        for r in rows {
            let (Some(word), Some(pron)) = (r.first().and_then(|v| v.text()), r.get(1).and_then(|v| v.text())) else { continue };
            let syms: Vec<String> = pron.split(|c| c == ' ' || c == '-').filter(|s| !s.is_empty()).map(str::to_string).collect();
            if !syms.is_empty() {
                dictionary.entry(word.to_lowercase()).or_insert(syms);
            }
        }
    }
    let mut names = HashMap::new();
    for (name, f) in phones {
        names.entry(*f).or_insert_with(|| name.clone());
    }
    let n = dictionary.len();
    let data = LanguageData { dictionary, gpr: gpr.and_then(crate::gpr::Gpr::parse), names };
    LANGUAGE_DATA.write().unwrap().get_or_insert_with(HashMap::new).insert(lang, std::sync::Arc::new(data));
    n
}

fn language_data(lang: Language) -> Option<std::sync::Arc<LanguageData>> {
    LANGUAGE_DATA.read().unwrap().as_ref()?.get(&lang).cloned()
}

impl LanguageData {
    fn name(&self, f: u32) -> Option<String> {
        // the model sets a few extra bits the phone tables don't (w is
        // 0x401ec2 in EnglishUs.gpr, 0x1ec2 in Allison's table)
        [0u32, 0x400000, 0x200000, 0x600000, 0x1000_0000].iter().find_map(|m| self.names.get(&(f & !m)).cloned())
    }

    /// The word's phones before the language's word rules.
    fn raw(&self, word: &str) -> Option<Vec<String>> {
        if let Some(p) = self.dictionary.get(word) {
            return Some(p.clone());
        }
        let g = self.gpr.as_ref()?;
        let bytes: Vec<u8> = word.chars().map(|c| if (c as u32) < 256 { c as u8 } else { b'?' }).collect();
        Some(g.transcribe(&bytes).into_iter().filter_map(|f| self.name(f)).collect())
    }
}

fn language_data_lookup(word: &str, lang: Language) -> Option<Vec<String>> {
    let data = language_data(lang)?;
    let w = word.to_lowercase();
    if lang != Language::English {
        return data.dictionary.get(&w).cloned();
    }
    // English: `'s` is the stem plus its ending, without the stem's flap
    if let Some(stem) = w.strip_suffix("'s").filter(|s| !s.is_empty()) {
        let mut p = data.raw(stem)?;
        let last = p.last().map(String::as_str).unwrap_or("");
        let ending: &[&str] = match last {
            "s" | "z" | "$" | "Zg" | "T$" | "Dg" | "Ts" => &["I", "z"],
            "th" | "kh" | "ph" | "f" | "Th" => &["s"],
            _ => &["z"],
        };
        p.extend(ending.iter().map(|s| s.to_string()));
        return Some(english_word_rules(p, false));
    }
    Some(english_word_rules(data.raw(&w)?, true))
}

fn en_vowel(s: &str) -> bool {
    let s = s.trim_start_matches('`').trim_end_matches(':');
    matches!(s, "a" | "e" | "i" | "o" | "u" | "I" | "E" | "O" | "U" | "Ae" | "Aa" | "Ah" | "Oa" | "HEh" | "HEI" | "HAI" | "HOI" | "HOU" | "HAU" | "OR" | "HR" | "Oe")
        || (s.starts_with('H') && s[1..].starts_with(|c| "AEIOU".contains(c)))
}

/// The word rules the original applies after its dictionary and
/// letter-to-sound model for US English (measured against its transcription
/// of 128 374 words: 80.5% identical): `s` + stop unaspirated inside a word
/// (`s th` → `s t`); `t s` → `Ts`; `HEh Hr` → `HR` before a consonant or the
/// end; `HEh l` → `Hl` at the end or before z/d; `Hr` → `HR` after `HAI` and
/// stressed `I`/`E`; dark `Hl` after `i:`/`i`/`HEI` at the end; `th` flapped
/// (`HTh`) between a vowel (or n, r) and an unstressed vowel when the vowel
/// before is stressed or full, or in the last syllable before i/I/HR/Hl;
/// other stops before a consonant (not r, l, j, w) unreleased (`Hut`…).
pub fn english_word_rules(s: Vec<String>, flap: bool) -> Vec<String> {
    let mut out = s;
    let n = out.len();
    for i in 1..n {
        if out[i - 1] == "s" && matches!(out[i].as_str(), "th" | "kh" | "ph") && i + 1 < n {
            out[i] = out[i][..1].to_string();
        }
    }
    let mut res: Vec<String> = Vec::with_capacity(out.len());
    let mut i = 0;
    while i < out.len() {
        if matches!(out[i].as_str(), "th" | "t") && out.get(i + 1).map(String::as_str) == Some("s") {
            res.push("Ts".into());
            i += 2;
            continue;
        }
        res.push(out[i].clone());
        i += 1;
    }
    out = res;
    let mut res: Vec<String> = Vec::with_capacity(out.len());
    let mut i = 0;
    while i < out.len() {
        let nxt = out.get(i + 1).map(String::as_str);
        let nn = out.get(i + 2).map(String::as_str);
        if out[i] == "HEh" && nxt == Some("Hr") && nn.map(|x| !en_vowel(x)).unwrap_or(true) {
            res.push("HR".into());
            i += 2;
            continue;
        }
        if out[i] == "HEh" && nxt == Some("l") && !res.is_empty() && matches!(nn, None | Some("z") | Some("d")) {
            res.push("Hl".into());
            i += 2;
            continue;
        }
        res.push(out[i].clone());
        i += 1;
    }
    out = res;
    for i in 1..out.len() {
        let prev = out[i - 1].clone();
        let nxt = out.get(i + 1).cloned();
        if out[i] == "Hr" && (prev.trim_start_matches('`') == "HAI" || (matches!(prev.as_str(), "`I" | "`E") && !nxt.as_deref().map(en_vowel).unwrap_or(false))) {
            out[i] = "HR".into();
        }
        if out[i] == "l" && matches!(prev.trim_start_matches('`'), "i:" | "i" | "HEI") && matches!(nxt.as_deref(), None | Some("z") | Some("d")) {
            out[i] = "Hl".into();
        }
    }
    let stressed = |s: &str| s.starts_with('`');
    for i in 1..out.len() {
        let Some(nxt) = out.get(i + 1).cloned() else { continue };
        let prev = out[i - 1].clone();
        let prevv = out[..i].iter().rev().find(|y| en_vowel(y)).cloned();
        let last = !out[i + 2..].iter().any(|y| en_vowel(y));
        let vowelish = en_vowel(&nxt) || nxt == "HR" || nxt == "Hl";
        if flap
            && out[i] == "th"
            && (en_vowel(&prev) || matches!(prev.as_str(), "n" | "Hr" | "HR"))
            && vowelish
            && !stressed(&nxt)
            && (prevv.as_deref().map(|v| stressed(v) || !matches!(v.trim_start_matches('`'), "HEh" | "I")).unwrap_or(false)
                || (last && matches!(nxt.trim_start_matches('`'), "i" | "I" | "HR" | "Hl")))
        {
            out[i] = "HTh".into();
        } else if !en_vowel(&nxt) && !matches!(nxt.as_str(), "Hr" | "l" | "j" | "w" | "Hl" | "HR") {
            let u = match out[i].as_str() {
                "th" | "t" => Some("Hut"),
                "kh" | "k" => Some("Huk"),
                "ph" | "p" => Some("Hup"),
                "d" => Some("Hud"),
                "b" => Some("Hub"),
                "g" => Some("Hug"),
                _ => None,
            };
            if let Some(u) = u {
                out[i] = u.into();
            }
        }
    }
    out
}

// --- Spanish connected speech ------------------------------------------------

/// Little Spanish words said without stress (Jorge's transcriptions).
const SPANISH_UNSTRESSED: &[&str] = &[
    "el", "la", "los", "las", "lo", "de", "del", "con", "por", "para", "a", "al", "en", "y", "e", "o", "u", "que", "se", "me",
    "te", "le", "les", "nos", "su", "sus", "mi", "mis", "tu", "tus", "un", "una", "unos", "unas", "sin",
];

/// What the original does across Spanish words: little words lose their
/// stress (`y` is i); b/d/g are stops (b dd g) at the start of a phrase or
/// after a nasal (d also after l) and soft (BH DH GH) elsewhere, also inside
/// words; n takes the place of the next consonant (m before p/b/m, M before
/// f, N before k/g/x, GN before ll/y); su/tu/mi glide into a vowel.
fn spanish_connected_speech(tokens: &mut [Token]) {
    let words: Vec<usize> = (0..tokens.len()).filter(|&i| matches!(tokens[i], Token::Word { .. })).collect();
    for &i in &words {
        if let Token::Word { text, symbols } = &mut tokens[i] {
            if text == "y" {
                *symbols = vec!["i".into()];
            } else if SPANISH_UNSTRESSED.contains(&text.as_str()) {
                for s in symbols.iter_mut() {
                    *s = s.trim_start_matches('`').to_string();
                }
            }
        }
    }
    // walk the phones of each phrase (pauses other than word gaps break it)
    let mut prev: Option<String> = None;
    for t in tokens.iter_mut() {
        match t {
            Token::Pause { reason, .. } => {
                if reason != "word gap" {
                    prev = None;
                }
            }
            Token::Word { symbols, .. } => {
                for k in 0..symbols.len() {
                    let cur = symbols[k].clone();
                    let after_nasal = matches!(prev.as_deref(), Some("m" | "n" | "N" | "M" | "GN" | "Gn"));
                    let soft = match cur.as_str() {
                        "b" => (prev.is_some() && !after_nasal).then_some("BH"),
                        "dd" => (prev.is_some() && !after_nasal && prev.as_deref() != Some("l")).then_some("DH"),
                        "g" => (prev.is_some() && !after_nasal).then_some("GH"),
                        _ => None,
                    };
                    if let Some(s) = soft {
                        symbols[k] = s.into();
                    }
                    prev = Some(symbols[k].clone());
                }
            }
        }
    }
    // nasal assimilation and gliding, across word gaps too
    let mut flat: Vec<(usize, usize)> = Vec::new();
    for (ti, t) in tokens.iter().enumerate() {
        if let Token::Word { symbols, .. } = t {
            for k in 0..symbols.len() {
                flat.push((ti, k));
            }
        }
    }
    for f in 0..flat.len().saturating_sub(1) {
        let (ti, k) = flat[f];
        let (tj, kj) = flat[f + 1];
        let joined = ti == tj || tokens[ti + 1..tj].iter().all(|t| matches!(t, Token::Pause { reason, .. } if reason == "word gap"));
        if !joined {
            continue;
        }
        let next = tokens[tj].symbols()[kj].trim_start_matches('`').to_string();
        if let Token::Word { text, symbols } = &mut tokens[ti] {
            let cur = symbols[k].clone();
            if cur == "n" {
                let to = match next.as_str() {
                    "p" | "b" | "BH" | "m" => Some("m"),
                    "f" => Some("M"),
                    "k" | "g" | "GH" | "x" => Some("N"),
                    "Gl" | "J" => Some("GN"),
                    _ => None,
                };
                if let Some(to) = to {
                    symbols[k] = to.into();
                }
            }
            let glide_word = matches!(text.as_str(), "su" | "tu" | "mi" | "sus" | "tus" | "mis");
            if ti != tj && glide_word && k + 1 == symbols.len() && matches!(next.as_str(), "a" | "e" | "i" | "o" | "u") {
                symbols[k] = match cur.as_str() {
                    "u" => "w".into(),
                    "i" => "j".into(),
                    other => other.into(),
                };
            }
        }
    }
}

// --- French connected speech --------------------------------------------------

/// A French word the dictionary doesn't have whole: `est-ce` reads as its
/// parts, `m'appelle` as the elided little word (`me` without its vowel) and
/// the rest.
fn french_compound(word: &str) -> Option<Vec<String>> {
    let word = word.trim_matches(|c| c == '-' || c == '\'');
    if let Some((a, b)) = word.split_once('-') {
        let mut out = word_to_symbols(a, Language::French);
        // `est-ce` `E s: the inverted ce keeps only its consonant
        out.extend(if b == "ce" { vec!["s".to_string()] } else { word_to_symbols(b, Language::French) });
        return Some(out);
    }
    let (a, b) = word.split_once('\'')?;
    // jusqu', lorsqu', puisqu', quoiqu': the word itself (stress kept) and k
    if a.ends_with("qu") && a.len() > 3 {
        // `lorsque` minus its e; only jusqu' keeps its stress
        let mut out = crate::french::transcribe(&format!("{a}e"));
        if out.last().map(|s| s.trim_start_matches('`') == "HEh").unwrap_or(false) {
            out.pop();
        }
        if a != "jusqu" {
            for s in out.iter_mut() {
                *s = s.trim_start_matches('`').to_string();
            }
        }
        out.extend(word_to_symbols(b, Language::French));
        return Some(out);
    }
    // l' d' j' m' n' s' t' c' qu' jusqu': the full word minus its last vowel
    let mut out = word_to_symbols(&format!("{a}e"), Language::French);
    out.pop();
    for s in out.iter_mut() {
        *s = s.trim_start_matches('`').to_string();
    }
    out.extend(word_to_symbols(b, Language::French));
    Some(out)
}

/// Little words the original reads without stress inside a sentence (Bernard's
/// transcriptions, oracle/bench-fr.txt): `le` l HEh, `les` l e, `nous` n u.
const FRENCH_UNSTRESSED: &[&str] = &[
    "le", "la", "les", "de", "des", "du", "un", "une", "et", "à", "au", "aux", "ce", "ces", "mes",
    "tes", "ses", "mon", "ton", "son", "ma", "ta", "sa", "nos", "vos", "je", "suis",
    "tu", "il", "ils", "elle", "elles", "on", "nous", "vous", "me", "te", "se", "ne", "que",
    "qui", "en", "y", "sur", "sous", "sont", "sans", "dans", "est", "pour", "par",
];

/// What the original does between French words: little words lose their
/// stress (`OE becomes HEh); liaison puts the silent final consonant at the
/// start of the next word when it starts with a vowel (`mes amis` m e | z a m
/// `i), and an -ain/-on adjective loses its nasal (`prochain arrêt` p HRF Oa $
/// `E | n a HRF `E); a word ending in consonant + r/l keeps its final e before
/// a consonant (`votre patience` v `Oa tt HRF HEh).
fn french_connected_speech(tokens: &mut [Token]) {
    let words: Vec<usize> = (0..tokens.len()).filter(|&i| matches!(tokens[i], Token::Word { .. })).collect();
    let starts_vowel = |t: &Token| t.symbols().first().map(|s| is_vowel(s)).unwrap_or(false);
    for (n, &i) in words.iter().enumerate() {
        let next = words.get(n + 1).copied();
        let joined = next.map(|j| tokens[i + 1..j].iter().all(|t| matches!(t, Token::Pause { reason, .. } if reason == "word gap"))).unwrap_or(false);
        let next_vowel = joined && next.map(|j| starts_vowel(&tokens[j])).unwrap_or(false);
        let next_consonant = joined && !next_vowel;
        let mut link: Option<&str> = None;
        if let Token::Word { text, symbols } = &mut tokens[i] {
            let w = text.as_str();
            if joined && FRENCH_UNSTRESSED.contains(&w) {
                for s in symbols.iter_mut() {
                    *s = match s.trim_start_matches('`') {
                        "OE" => "HEh".to_string(),
                        other => other.to_string(),
                    };
                }
            }
            if next_vowel {
                link = match w {
                    "les" | "des" | "mes" | "tes" | "ses" | "ces" | "nous" | "vous" | "ils" | "elles" | "aux" | "deux" | "trois" | "dans" | "chez" | "leurs" | "nos" | "vos" => Some("z"),
                    "est" | "sont" | "ont" | "petit" | "grand" | "quand" | "tout" | "c'est" | "font" | "vont" => Some("tt"),
                    "un" | "mon" | "ton" | "son" | "on" | "en" | "aucun" | "bien" | "rien" => Some("n"),
                    "prochain" | "certain" | "plein" | "ancien" | "moyen" | "bon" => {
                        if let Some(last) = symbols.last_mut() {
                            *last = last.trim_end_matches(')').replace("`o", "`Oa");
                        }
                        Some("n")
                    }
                    _ => None,
                };
            }
            let k = symbols.len();
            if next_consonant && k >= 2 && matches!(symbols[k - 1].as_str(), "HRF" | "l") && !is_vowel(&symbols[k - 2]) && symbols[k - 2] != "HRF" {
                symbols.push("HEh".to_string());
            }
        }
        if let (Some(l), Some(j)) = (link, next) {
            if let Token::Word { symbols, .. } = &mut tokens[j] {
                symbols.insert(0, l.to_string());
            }
        }
    }
}

// --- pronunciation dictionaries ---------------------------------------------

// ponytail: transcriptions kept as text and split on lookup; split up front the
// 635 000-word French dictionary took ~350 MB
type Lexicon = std::collections::HashMap<String, String>;

fn lexicons() -> &'static std::sync::Mutex<std::collections::HashMap<Language, std::sync::Arc<Lexicon>>> {
    static L: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<Language, std::sync::Arc<Lexicon>>>> = std::sync::OnceLock::new();
    L.get_or_init(Default::default)
}

static LEXICON_DIRS: std::sync::RwLock<Vec<std::path::PathBuf>> = std::sync::RwLock::new(Vec::new());

/// Add a folder to look for pronunciation dictionaries in
/// (`<MotherTongue>.tsv`, lines `word<TAB>transcription`).
pub fn add_lexicon_dir(dir: impl Into<std::path::PathBuf>) {
    let dir = dir.into();
    let mut dirs = LEXICON_DIRS.write().unwrap();
    if !dirs.contains(&dir) {
        dirs.push(dir);
    }
}

/// The folders registered with `add_lexicon_dir`.
pub fn lexicon_dirs() -> Vec<std::path::PathBuf> {
    LEXICON_DIRS.read().unwrap().clone()
}

/// Load the dictionary for a voice's language, if one was learned.
/// Returns how many words it has.
pub fn load_lexicon(lang: Language, mother_tongue: &str) -> usize {
    // QQ_NO_LEXICON=1: measure the rules alone
    if std::env::var_os("QQ_NO_LEXICON").is_some() {
        return 0;
    }
    let mut all = lexicons().lock().unwrap();
    if let Some(l) = all.get(&lang) {
        return l.len();
    }
    for dir in lexicon_dirs() {
        let path = dir.join(format!("{mother_tongue}.tsv"));
        if let Ok(text) = std::fs::read_to_string(&path) {
            let lex: Lexicon = text
                .lines()
                .filter_map(|l| l.split_once('\t'))
                .filter(|(_, t)| t.chars().any(|c| c != ' ' && c != '-'))
                .map(|(w, t)| (w.to_string(), t.to_string()))
                .collect();
            let n = lex.len();
            all.insert(lang, std::sync::Arc::new(lex));
            return n;
        }
    }
    0
}

fn lexicon_lookup(word: &str, lang: Language) -> Option<Vec<String>> {
    let lex = lexicons().lock().unwrap().get(&lang).cloned()?;
    let t = lex.get(&word.to_lowercase())?;
    Some(t.split(|c| c == ' ' || c == '-').filter(|s| !s.is_empty()).map(str::to_string).collect())
}

pub fn word_to_symbols(word: &str, lang: Language) -> Vec<String> {
    // the dictionary first: the original engine's own pronunciation
    if let Some(p) = lexicon_lookup(word, lang) {
        return p;
    }
    // then the language's own data from the installer (`load_language_data`)
    if let Some(p) = language_data_lookup(word, lang) {
        return p;
    }
    if lang == Language::French {
        if let Some(out) = french_compound(word) {
            return out;
        }
        if std::env::var_os("QQ_OLD_FRENCH").is_none() {
            return crate::french::transcribe(word);
        }
    }
    if lang == Language::Spanish {
        return crate::spanish::transcribe(word);
    }
    let raw = bare_word(word);
    if raw.is_empty() {
        return Vec::new();
    }
    let accent_final = word
        .chars()
        .last()
        .map(|c| ACCENTS.iter().any(|(a, _)| *a == c))
        .unwrap_or(false);
    let w: Vec<char> = match lang {
        Language::Russian | Language::Greek => transliterate(&raw),
        _ => raw.clone(),
    };
    let mut out = match lang {
        Language::Italian => italian(&w),
        Language::German => german(&w),
        Language::English => english(&w),
        Language::French => french(&w),
        Language::Spanish => spanish(&w),
        Language::Catalan => catalan(&w),
        Language::Portuguese => portuguese(&w),
        Language::Dutch => dutch(&w),
        Language::Swedish | Language::Danish | Language::Norwegian => nordic(&w),
        Language::Finnish => finnish(&w),
        Language::Polish => polish(&w),
        Language::Turkish | Language::Russian | Language::Greek | Language::Generic => {
            generic(&w)
        }
    };
    if lang == Language::Italian {
        italian_stress(&raw, &mut out, accent_final);
    } else {
        stress(&w, &mut out, lang, accent_final, &raw);
    }
    for sym in out.iter_mut() {
        if let Some(to) = voice_symbol(lang, sym) {
            *sym = to.to_string();
        }
    }
    out
}

/// The rules' generic symbols in the voices' own alphabets, read from the
/// recorded inventories (Allison for US English, Bernard for French): English
/// voices aspirate t/k/p (`th`, `kh`, `ph`), write r `Hr`, ash `Ae`, open o
/// `Oa:`; French voices write plain t/d as `tt`/`dd` and r `HRF`. The English
/// and French rules themselves are still the simple ones (MECHANICS.md §11).
fn voice_symbol(lang: Language, sym: &str) -> Option<&'static str> {
    let stressed = sym.starts_with('`');
    let base = sym.trim_start_matches('`');
    Some(match (lang, base, stressed) {
        (Language::French, "t", _) => "tt",
        (Language::French, "d", _) => "dd",
        (Language::French, "R" | "r", _) => "HRF",
        (Language::English, "t", _) => "th",
        (Language::English, "k", _) => "kh",
        (Language::English, "p", _) => "ph",
        (Language::English, "R" | "r", _) => "Hr",
        (Language::English, "a", false) => "Ae",
        (Language::English, "a", true) => "`Ae",
        (Language::English, "e", false) => "E",
        (Language::English, "e", true) => "`E",
        (Language::English, "O" | "o", false) => "Oa:",
        (Language::English, "O" | "o", true) => "`Oa:",
        _ => return None,
    })
}

fn push_all(out: &mut Vec<String>, items: &[&str]) {
    for s in items {
        out.push((*s).to_string());
    }
}

// --- Italian ----------------------------------------------------------------

/// A word-final `-ia` after a consonant is a full, usually stressed `i` in the
/// original engine (`geografia`, `allergia`: 94-98% after f/m/p/t/g), not a
/// glide; after c it only softens (`faccia`), after r/z it is mixed.
fn final_ia(w: &[char], i: usize) -> bool {
    i + 2 == w.len()
        && plain(w, i) == Some('i')
        && plain(w, i + 1) == Some('a')
        && i > 0
        && !matches!(plain(w, i - 1), Some('a' | 'e' | 'i' | 'o' | 'u' | 'c' | 'r' | 'z'))
        && !(i > 1 && plain(w, i - 1) == Some('g') && plain(w, i - 2) == Some('g'))
}

/// After c/g, an i followed by another vowel only marks the soft sound
/// (`ciao`, `giallo`, `faccia`) and is not spoken.
fn silent_i(w: &[char], i: usize) -> bool {
    plain(w, i) == Some('i') && plain(w, i + 1).map(is_vowel_char).unwrap_or(false)
}

fn italian(w: &[char]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    let n = w.len();
    while i < n {
        let c = plain(w, i).unwrap_or(' ');
        let next = plain(w, i + 1);
        let prev_vowel = out.last().map(|s| is_vowel(s)).unwrap_or(false);
        let mut used = 1;
        if starts(w, i, "gli") {
            push_all(&mut out, &["Gl"]);
            used = if plain(w, i + 3).map(is_vowel_char).unwrap_or(false) { 3 } else { 2 };
        } else if starts(w, i, "gn") {
            push_all(&mut out, &["Gn"]);
            used = 2;
        } else if starts(w, i, "sci") && plain(w, i + 3).map(is_vowel_char).unwrap_or(false) {
            // the i here only softens the c: `sciarpa` is /ʃarpa/
            push_all(&mut out, &["$"]);
            used = 3;
        } else if starts(w, i, "sc") && matches!(plain(w, i + 2), Some('e') | Some('i')) {
            // emit the fricative only; the vowel that follows speaks for itself
            push_all(&mut out, &["$"]);
            used = 2;
        } else if (starts(w, i, "cc") || starts(w, i, "gg")) && matches!(plain(w, i + 2), Some('e') | Some('i')) {
            // geminate palatal affricate: `faccia`, `legge`; Roberto writes
            // /tʃ/ as `T$` and /dʒ/ as `Dg` (`Ts`/`Dz` are /ts/ and /dz/)
            push_all(&mut out, &[if c == 'c' { "T$:" } else { "Dg:" }]);
            used = if silent_i(w, i + 2) { 3 } else { 2 };
        } else if starts(w, i, "ch") {
            push_all(&mut out, &["k"]);
            used = 2;
        } else if starts(w, i, "gh") {
            push_all(&mut out, &["g"]);
            used = 2;
        } else if starts(w, i, "qu") {
            push_all(&mut out, &["k", "w"]);
            used = 2;
        } else if starts(w, i, "zz") {
            // the original voices `-izz-` (`organizzare`) 1597 times in 2510,
            // other `zz` mostly not (oracle word list, MECHANICS.md §9)
            let voiced = i > 0 && plain(w, i - 1) == Some('i');
            push_all(&mut out, &[if voiced { "Dz:" } else { "Ts:" }]);
            used = 2;
        } else if starts(w, i, "cqu") {
            push_all(&mut out, &["k:", "w"]);
            used = 3;
        } else if next == Some(c) && "bcdfglmnprstv".contains(c) {
            // geminate: the engine writes length with a trailing colon
            let long = match c {
                't' => "tt:".to_string(),
                'd' => "dd:".to_string(),
                // the inventory writes geminates with the colon too; `c:`/`g:`
                // are not symbols any voice declares, so use the phone
                'c' => "k:".to_string(),
                'g' => "g:".to_string(),
                _ => format!("{c}:"),
            };
            out.push(long);
            used = 2;
        } else {
            match c {
                'c' => {
                    // `ci`/`ce` is the affricate; the i survives unless another
                    // vowel follows (`ciao` /tʃao/, `città` /tʃitˈta/)
                    if matches!(next, Some('e') | Some('i')) {
                        push_all(&mut out, &["T$"]);
                        used = if silent_i(w, i + 1) { 2 } else { 1 };
                    } else {
                        push_all(&mut out, &["k"]);
                        used = 1;
                    }
                }
                'g' => {
                    if matches!(next, Some('e') | Some('i')) {
                        push_all(&mut out, &["Dg"]);
                        used = if silent_i(w, i + 1) && !final_ia(w, i + 1) { 2 } else { 1 };
                    } else {
                        push_all(&mut out, &["g"]);
                        used = 1;
                    }
                }
                'q' => push_all(&mut out, &["k"]),
                'z' => {
                    // word-initial z is voiced (272 to 4), a single z between
                    // vowels is long (2745 to 203), after a consonant plain Ts
                    let between_vowels = prev_vowel && next.map(is_vowel_char).unwrap_or(false);
                    push_all(&mut out, &[if i == 0 { "Dz" } else if between_vowels { "Ts:" } else { "Ts" }]);
                }
                's' => {
                    let following_vowel = next.map(is_vowel_char).unwrap_or(false);
                    // voiced between vowels and before a voiced consonant (`sbaglio`)
                    let voiced_next = matches!(next, Some('b' | 'd' | 'g' | 'l' | 'm' | 'n' | 'r' | 'v'));
                    push_all(
                        &mut out,
                        &[if prev_vowel && following_vowel || voiced_next { "z" } else { "s" }],
                    );
                }
                'n' => {
                    // velar before a hard c/g (`banca`, `lungo`), not before ce/ci/ge/gi
                    let soft = matches!(plain(w, i + 2), Some('e') | Some('i'));
                    let velar = matches!(next, Some('k') | Some('q'))
                        || (matches!(next, Some('c') | Some('g')) && (!soft || plain(w, i + 2) == Some('h')))
                        || (matches!(next, Some('c') | Some('g')) && plain(w, i + 2) == Some('h'));
                    if velar {
                        push_all(&mut out, &["N"]);
                    } else if matches!(next, Some('f') | Some('v')) {
                        push_all(&mut out, &["M"]);
                    } else {
                        push_all(&mut out, &["n"]);
                    }
                }
                'x' => push_all(&mut out, &["k", "s"]),
                'h' => {}
                'j' | 'y' => push_all(&mut out, &["j"]),
                'w' => push_all(&mut out, &["w"]),
                'i' | 'u' => {
                    // before another vowel i/u glide (`binario`, `buono`), unless
                    // the word would be left with a single vowel (`mia`, `zio`)
                    let following_vowel = next.map(is_vowel_char).unwrap_or(false);
                    let vowels_before = (0..i).filter(|&k| plain(w, k).map(is_vowel_char).unwrap_or(false)).count();
                    let vowels_after = (i + 1..n).filter(|&k| plain(w, k).map(is_vowel_char).unwrap_or(false)).count();
                    // u glides only before o/i or after g (`buono`, `fuori`, `guerra`),
                    // so `due` keeps its two vowels
                    let after = if i > 0 { plain(w, i - 1) } else { None };
                    // the original keeps i a full vowel after r and l (`binario`,
                    // `Italia`: 8% and 3% glide) but glides it elsewhere (64-98%)
                    let u_glides = (c == 'i' && !matches!(after, Some('r') | Some('l')))
                        || (c == 'u' && (matches!(next, Some('o') | Some('i')) || after == Some('g')));
                    if following_vowel && u_glides && vowels_before + vowels_after >= 2 && !final_ia(w, i) {
                        push_all(&mut out, &[if c == 'i' { "j" } else { "w" }]);
                    } else {
                        let s = c.to_string();
                        out.push(s);
                    }
                }
                'a' | 'e' | 'o' => {
                    let s = c.to_string();
                    out.push(s);
                }
                'd' => push_all(&mut out, &["dd"]),
                't' => push_all(&mut out, &["tt"]),
                'b' | 'f' | 'l' | 'm' | 'p' | 'r' | 'v' => {
                    let s = c.to_string();
                    out.push(s);
                }
                _ => {}
            }
        }
        i += used.max(1);
    }
    out
}

/// Ending tables learned from the original engine's own transcriptions of
/// 87 421 Italian words (MECHANICS.md §9): which vowel takes the stress,
/// counted from the end (95.8% right on words held out of the learning), and
/// whether a stressed e/o is open (`E`, `Oa`) or closed (91.5%). A word uses
/// its longest ending in the table; `^` marks the start of the word.
fn italian_table(which: usize) -> &'static std::collections::HashMap<String, String> {
    use std::collections::HashMap;
    use std::sync::OnceLock;
    static TABLES: [OnceLock<HashMap<String, String>>; 2] = [OnceLock::new(), OnceLock::new()];
    TABLES[which].get_or_init(|| {
        let src = [include_str!("it-position.tsv"), include_str!("it-quality.tsv")][which];
        src.lines().filter_map(|l| l.split_once('\t')).map(|(k, v)| (k.to_string(), v.to_string())).collect()
    })
}

fn longest_ending(which: usize, word: &[char]) -> Option<&'static str> {
    let mut w: Vec<char> = vec!['^'];
    w.extend_from_slice(word);
    (1..=w.len().min(6)).rev().find_map(|l| {
        let key: String = w[w.len() - l..].iter().collect();
        italian_table(which).get(&key).map(|v| v.as_str())
    })
}

/// Stress for Italian: a written final accent wins (and says open/closed:
/// è ò open, é ó closed); otherwise the learned ending tables decide.
fn italian_stress(raw: &[char], symbols: &mut [String], accent_final: bool) {
    let vowels: Vec<usize> = (0..symbols.len()).filter(|&i| is_vowel(&symbols[i])).collect();
    if vowels.is_empty() {
        return;
    }
    let (target, quality) = if accent_final {
        let q = match raw.last() {
            Some('è') | Some('ò') => Some("open"),
            Some('é') | Some('ó') => Some("closed"),
            _ => None,
        };
        (*vowels.last().unwrap(), q)
    } else {
        let default = if vowels.len() >= 2 { 1 } else { 0 };
        let k: usize = longest_ending(0, raw).and_then(|v| v.parse().ok()).unwrap_or(default);
        let target = vowels[vowels.len() - 1 - k.min(vowels.len() - 1)];
        // the letters from the stressed vowel to the end, as the table keys them
        // (an i/u that glides into the next vowel is not a vowel here)
        let is_v = |c: char| "aeiouàèéìòù".contains(c);
        let groups: Vec<usize> = (0..raw.len())
            .filter(|&i| {
                is_v(raw[i])
                    && !("iu".contains(raw[i]) && i > 0 && i + 1 < raw.len() && is_v(raw[i + 1]) && !is_v(raw[i - 1]))
            })
            .collect();
        let q = groups
            .len()
            .checked_sub(1 + k)
            .map(|g| &raw[groups[g]..])
            .filter(|s| matches!(s.first(), Some('e') | Some('o')))
            .and_then(|s| longest_ending(1, s));
        (target, q)
    };
    let base = symbols[target].trim_start_matches('`').to_string();
    symbols[target] = match (base.as_str(), quality) {
        ("e", Some("open")) => "`E".into(),
        ("o", Some("open")) => "`Oa".into(),
        _ => format!("`{base}"),
    };
}

/// Words the original engine leaves unstressed inside a sentence (it does
/// stress them when they are said alone): articles, prepositions with and
/// without article, conjunctions, clitic pronouns.
const ITALIAN_UNSTRESSED: [&str; 63] = [
    "il", "lo", "la", "i", "gli", "le", "l", "un", "uno", "una", "di", "a", "da", "in", "con", "su",
    "per", "tra", "fra", "del", "dello", "della", "dei", "degli", "delle", "al", "allo", "alla",
    "ai", "agli", "alle", "dal", "dallo", "dalla", "dai", "dagli", "dalle", "nel", "nello", "nella",
    "nei", "negli", "nelle", "sul", "sullo", "sulla", "sui", "sugli", "sulle", "col", "coi", "e",
    "ed", "o", "od", "ma", "che", "se", "mi", "ti", "si", "ci", "non",
];

// --- German -----------------------------------------------------------------

fn german(w: &[char]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    let n = w.len();
    while i < n {
        let c = w[i];
        let mut used = 1;
        if starts(w, i, "tsch") {
            push_all(&mut out, &["T$"]);
            used = 4;
        } else if starts(w, i, "sch") {
            push_all(&mut out, &["$"]);
            used = 3;
        } else if starts(w, i, "pf") {
            push_all(&mut out, &["Pf"]);
            used = 2;
        } else if starts(w, i, "ch") {
            push_all(&mut out, &["x"]);
            used = 2;
        } else if starts(w, i, "ck") {
            push_all(&mut out, &["k"]);
            used = 2;
        } else if starts(w, i, "tz") {
            push_all(&mut out, &["Ts"]);
            used = 2;
        } else if starts(w, i, "sp") && i == 0 {
            push_all(&mut out, &["$", "p"]);
            used = 2;
        } else if starts(w, i, "st") && i == 0 {
            push_all(&mut out, &["$"]);
            used = 1;
        } else if starts(w, i, "eu") || starts(w, i, "äu") {
            // `eu`/`äu` is the /ɔʏ/ diphthong: Katrin's own symbol is HOI
            push_all(&mut out, &["HOI"]);
            used = if starts(w, i, "äu") { 2 } else { 2 };
        } else if starts(w, i, "au") {
            push_all(&mut out, &["AU"]);
            used = 2;
        } else if starts(w, i, "ei") || starts(w, i, "ai") {
            push_all(&mut out, &["HAI"]);
            used = 2;
        } else {
            match c {
                'ä' => push_all(&mut out, &["E:"]),
                'ö' => push_all(&mut out, &["OE:"]),
                'ü' => push_all(&mut out, &["y:"]),
                'ß' => push_all(&mut out, &["s"]),
                'a' => push_all(&mut out, &["a"]),
                'e' => push_all(&mut out, &["e"]),
                // Katrin has no short i/u: its lax closed vowels are I and U
                'i' => push_all(&mut out, &["I"]),
                'o' => push_all(&mut out, &["o"]),
                'u' => push_all(&mut out, &["U"]),
                'y' => push_all(&mut out, &["y:"]),
                'h' => {
                    // h after a vowel only lengthens it
                    match out.last() {
                        Some(prev) if is_vowel(prev) => {
                            if let Some(last) = out.last_mut() {
                                if !last.ends_with(':') {
                                    last.push(':');
                                }
                            }
                        }
                        _ => push_all(&mut out, &["h"]),
                    }
                }
                'n' => {
                    if matches!(plain(w, i + 1), Some('k') | Some('g')) {
                        push_all(&mut out, &["N"]);
                    } else {
                        push_all(&mut out, &["n"]);
                    }
                }
                'r' => push_all(&mut out, &["R"]),
                'w' => push_all(&mut out, &["v"]),
                'v' => push_all(&mut out, &["f"]),
                'j' => push_all(&mut out, &["y:"]),
                's' => {
                    // before a vowel German s is voiced: sagen, Häuser
                    if plain(w, i + 1).map(is_vowel_char).unwrap_or(false) {
                        push_all(&mut out, &["z"]);
                    } else {
                        push_all(&mut out, &["s"]);
                    }
                }
                'z' => push_all(&mut out, &["Ts"]),
                'c' => push_all(&mut out, &["k"]),
                'x' => push_all(&mut out, &["k", "s"]),
                'p' => push_all(&mut out, &["ph"]),
                't' => push_all(&mut out, &["t"]),
                'b' => push_all(&mut out, &["p"]),
                'd' => push_all(&mut out, &["t"]),
                'g' => push_all(&mut out, &["k"]),
                'k' => push_all(&mut out, &["k"]),
                'f' => push_all(&mut out, &["f"]),
                'l' => push_all(&mut out, &["l"]),
                'm' => push_all(&mut out, &["m"]),
                'q' => push_all(&mut out, &["k"]),
                _ => {}
            }
        }
        // a doubled consonant is not held twice in German, it just shortens
        // the vowel, which the duration model already does
        if used == 1 && plain(w, i + 1) == plain(w, i) && plain(w, i).is_some() {
            used = 2;
        }
        i += used.max(1);
    }
    // Katrin has no stressed short `e`/`o` (only `e:`/`o:` among the stressed
    // tense vowels), so a stressed e or o is lengthened to match its inventory
    for s in out.iter_mut() {
        if s.as_str() == "`e" {
            *s = "`e:".to_string();
        } else if s.as_str() == "`o" {
            *s = "`o:".to_string();
        }
    }
    out
}

// --- English ----------------------------------------------------------------

fn english(w: &[char]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    let n = w.len();
    while i < n {
        let c = plain(w, i).unwrap_or(' ');
        let next = plain(w, i + 1);
        let mut used = 1;
        if starts(w, i, "tch") {
            push_all(&mut out, &["T$"]);
            used = 3;
        } else if starts(w, i, "th") {
            push_all(&mut out, &["th"]);
            used = 2;
        } else if starts(w, i, "sh") {
            push_all(&mut out, &["$"]);
            used = 2;
        } else if starts(w, i, "ch") {
            push_all(&mut out, &["T$"]);
            used = 2;
        } else if starts(w, i, "ph") {
            push_all(&mut out, &["f"]);
            used = 2;
        } else if starts(w, i, "ng") {
            push_all(&mut out, &["N"]);
            used = 2;
        } else if starts(w, i, "ck") {
            push_all(&mut out, &["k"]);
            used = 2;
        } else if starts(w, i, "qu") {
            push_all(&mut out, &["k", "w"]);
            used = 2;
        } else if starts(w, i, "oo") {
            push_all(&mut out, &["u:"]);
            used = 2;
        } else if starts(w, i, "ee") {
            push_all(&mut out, &["i:"]);
            used = 2;
        } else if starts(w, i, "ea") {
            push_all(&mut out, &["i:"]);
            used = 2;
        } else if starts(w, i, "ou") {
            push_all(&mut out, &["AU"]);
            used = 2;
        } else if starts(w, i, "ai") || starts(w, i, "ay") {
            push_all(&mut out, &["EI"]);
            used = 2;
        } else {
            match c {
                'c' => {
                    if matches!(next, Some('e') | Some('i') | Some('y')) {
                        push_all(&mut out, &["s"]);
                    } else {
                        push_all(&mut out, &["k"]);
                    }
                }
                'g' => {
                    if matches!(next, Some('e') | Some('i') | Some('y')) {
                        push_all(&mut out, &["Dg"]);
                    } else {
                        push_all(&mut out, &["g"]);
                    }
                }
                'x' => push_all(&mut out, &["k", "s"]),
                'a' => push_all(&mut out, &["a"]),
                'e' => push_all(&mut out, &["e"]),
                'i' | 'y' => push_all(&mut out, &["I"]),
                'o' => push_all(&mut out, &["O"]),
                'u' => push_all(&mut out, &["U"]),
                'j' => push_all(&mut out, &["Dg"]),
                'w' => push_all(&mut out, &["w"]),
                'h' => push_all(&mut out, &["h"]),
                'r' => push_all(&mut out, &["R"]),
                'b' | 'd' | 'f' | 'k' | 'l' | 'm' | 'n' | 'p' | 's' | 't' | 'v' | 'z' => {
                    let s = c.to_string();
                    out.push(s);
                }
                _ => {}
            }
        }
        i += used.max(1);
    }
    out
}

// --- French -----------------------------------------------------------------

fn french(w: &[char]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    let n = w.len();
    while i < n {
        let c = plain(w, i).unwrap_or(' ');
        let mut used = 1;
        if starts(w, i, "ch") {
            push_all(&mut out, &["$"]);
            used = 2;
        } else if starts(w, i, "gn") {
            push_all(&mut out, &["Gn"]);
            used = 2;
        } else if starts(w, i, "ph") {
            push_all(&mut out, &["f"]);
            used = 2;
        } else if starts(w, i, "qu") {
            push_all(&mut out, &["k"]);
            used = 2;
        } else if starts(w, i, "ou") {
            push_all(&mut out, &["u"]);
            used = 2;
        } else if starts(w, i, "oi") {
            push_all(&mut out, &["w", "a"]);
            used = 2;
        } else if starts(w, i, "eu") {
            push_all(&mut out, &["OE"]);
            used = 2;
        } else if starts(w, i, "ai") || starts(w, i, "ei") {
            push_all(&mut out, &["E"]);
            used = 2;
        } else if starts(w, i, "au") || starts(w, i, "eau") {
            push_all(&mut out, &["o"]);
            used = if starts(w, i, "eau") { 3 } else { 2 };
        } else {
            match c {
                'c' => {
                    if matches!(plain(w, i + 1), Some('e') | Some('i') | Some('y')) {
                        push_all(&mut out, &["s"]);
                    } else {
                        push_all(&mut out, &["k"]);
                    }
                }
                'g' => {
                    if matches!(plain(w, i + 1), Some('e') | Some('i') | Some('y')) {
                        push_all(&mut out, &["Zg"]);
                    } else {
                        push_all(&mut out, &["g"]);
                    }
                }
                'j' => push_all(&mut out, &["Zg"]),
                'r' => push_all(&mut out, &["R"]),
                'x' => {
                    if i + 1 == n {
                        // final x is silent in French
                    } else {
                        push_all(&mut out, &["k", "s"]);
                    }
                }
                'a' => push_all(&mut out, &["a"]),
                'e' => push_all(&mut out, &["e"]),
                'i' | 'y' => push_all(&mut out, &["i"]),
                'o' => push_all(&mut out, &["o"]),
                'u' => push_all(&mut out, &["y"]),
                'h' => {}
                'b' | 'd' | 'f' | 'k' | 'l' | 'm' | 'n' | 'p' | 's' | 't' | 'v' | 'z' => {
                    let s = c.to_string();
                    out.push(s);
                }
                _ => {}
            }
        }
        i += used.max(1);
    }
    // a final e is orthography, not a sound
    if out.len() > 1 && out.last().map(|s| s == "e").unwrap_or(false) {
        out.pop();
    }
    out
}

// --- Spanish and Catalan ----------------------------------------------------

fn spanish(w: &[char]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    let n = w.len();
    while i < n {
        let c = plain(w, i).unwrap_or(' ');
        let mut used = 1;
        if starts(w, i, "ch") {
            push_all(&mut out, &["T$"]);
            used = 2;
        } else if starts(w, i, "ll") {
            push_all(&mut out, &["j"]);
            used = 2;
        } else if starts(w, i, "ny") {
            push_all(&mut out, &["Gn"]);
            used = 2;
        } else if starts(w, i, "qu") {
            push_all(&mut out, &["k"]);
            used = 2;
        } else if starts(w, i, "gu") {
            push_all(&mut out, &["g"]);
            used = 2;
        } else if starts(w, i, "rr") {
            push_all(&mut out, &["r"]);
            used = 2;
        } else {
            match c {
                'c' => {
                    if matches!(plain(w, i + 1), Some('e') | Some('i')) {
                        push_all(&mut out, &["s"]);
                        used = 2;
                    } else {
                        push_all(&mut out, &["k"]);
                    }
                }
                'g' => {
                    if matches!(plain(w, i + 1), Some('e') | Some('i')) {
                        push_all(&mut out, &["x"]);
                        used = 2;
                    } else {
                        push_all(&mut out, &["g"]);
                    }
                }
                'z' => push_all(&mut out, &["s"]),
                'j' => push_all(&mut out, &["x"]),
                'y' => push_all(&mut out, &["j"]),
                'v' => push_all(&mut out, &["b"]),
                'h' => {}
                'x' => push_all(&mut out, &["k", "s"]),
                'a' => push_all(&mut out, &["a"]),
                'e' => push_all(&mut out, &["e"]),
                'i' => push_all(&mut out, &["i"]),
                'o' => push_all(&mut out, &["o"]),
                'u' => push_all(&mut out, &["u"]),
                'b' | 'd' | 'f' | 'k' | 'l' | 'm' | 'n' | 'p' | 'r' | 's' | 't' => {
                    let s = c.to_string();
                    out.push(s);
                }
                _ => {}
            }
        }
        i += used.max(1);
    }
    out
}

fn catalan(w: &[char]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    let n = w.len();
    while i < n {
        let c = plain(w, i).unwrap_or(' ');
        let mut used = 1;
        if starts(w, i, "ll") {
            push_all(&mut out, &["Gl"]);
            used = 2;
        } else if starts(w, i, "ny") {
            push_all(&mut out, &["Gn"]);
            used = 2;
        } else if starts(w, i, "ig") {
            push_all(&mut out, &["T$"]);
            used = 2;
        } else if starts(w, i, "tx") {
            push_all(&mut out, &["T$"]);
            used = 2;
        } else if starts(w, i, "qu") {
            push_all(&mut out, &["k"]);
            used = 2;
        } else if starts(w, i, "gu") {
            push_all(&mut out, &["g"]);
            used = 2;
        } else {
            match c {
                'c' => {
                    if matches!(plain(w, i + 1), Some('e') | Some('i')) {
                        push_all(&mut out, &["s"]);
                        used = 2;
                    } else {
                        push_all(&mut out, &["k"]);
                    }
                }
                'g' => {
                    if matches!(plain(w, i + 1), Some('e') | Some('i')) {
                        push_all(&mut out, &["Zg"]);
                        used = 2;
                    } else {
                        push_all(&mut out, &["g"]);
                    }
                }
                'j' => push_all(&mut out, &["Zg"]),
                'x' => push_all(&mut out, &["$"]),
                'z' => push_all(&mut out, &["z"]),
                'h' => {}
                'a' => push_all(&mut out, &["a"]),
                'e' => push_all(&mut out, &["e"]),
                'i' => push_all(&mut out, &["i"]),
                'o' => push_all(&mut out, &["o"]),
                'u' => push_all(&mut out, &["u"]),
                'b' | 'd' | 'f' | 'k' | 'l' | 'm' | 'n' | 'p' | 'r' | 's' | 't' | 'v' => {
                    let s = c.to_string();
                    out.push(s);
                }
                _ => {}
            }
        }
        i += used.max(1);
    }
    out
}

// --- Portuguese -------------------------------------------------------------

fn portuguese(w: &[char]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    let n = w.len();
    while i < n {
        let c = w[i];
        let mut used = 1;
        if starts(w, i, "ch") {
            push_all(&mut out, &["$"]);
            used = 2;
        } else if starts(w, i, "lh") {
            push_all(&mut out, &["Gl"]);
            used = 2;
        } else if starts(w, i, "nh") {
            push_all(&mut out, &["Gn"]);
            used = 2;
        } else if starts(w, i, "qu") {
            push_all(&mut out, &["k"]);
            used = 2;
        } else if starts(w, i, "gu") {
            push_all(&mut out, &["g"]);
            used = 2;
        } else if starts(w, i, "rr") {
            push_all(&mut out, &["R"]);
            used = 2;
        } else {
            match strip_accent(c) {
                'ã' => push_all(&mut out, &["a"]),
                'c' => {
                    if matches!(plain(w, i + 1), Some('e') | Some('i')) {
                        push_all(&mut out, &["s"]);
                        used = 2;
                    } else {
                        push_all(&mut out, &["k"]);
                    }
                }
                'g' => {
                    if matches!(plain(w, i + 1), Some('e') | Some('i')) {
                        push_all(&mut out, &["Zg"]);
                        used = 2;
                    } else {
                        push_all(&mut out, &["g"]);
                    }
                }
                'j' => push_all(&mut out, &["Zg"]),
                'r' => push_all(&mut out, &["R"]),
                's' => {
                    let prev_vowel = out.last().map(|s| is_vowel(s)).unwrap_or(false);
                    let next_vowel = plain(w, i + 1).map(is_vowel_char).unwrap_or(false);
                    push_all(&mut out, &[if prev_vowel && next_vowel { "z" } else { "s" }]);
                }
                'x' => push_all(&mut out, &["$"]),
                'z' => push_all(&mut out, &["z"]),
                'h' => {}
                'a' => push_all(&mut out, &["a"]),
                'e' => push_all(&mut out, &["e"]),
                'i' => push_all(&mut out, &["i"]),
                'o' => push_all(&mut out, &["o"]),
                'u' => push_all(&mut out, &["u"]),
                'b' | 'd' | 'f' | 'l' | 'm' | 'n' | 'p' | 't' | 'v' => {
                    let s = strip_accent(c).to_string();
                    out.push(s);
                }
                _ => {}
            }
        }
        // Portuguese marked nasality: ã õ
        if matches!(c, 'ã' | 'õ') {
            if let Some(last) = out.last_mut() {
                last.push(')');
            }
        }
        i += used.max(1);
    }
    out
}

// --- Dutch ------------------------------------------------------------------

fn dutch(w: &[char]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    let n = w.len();
    while i < n {
        let c = plain(w, i).unwrap_or(' ');
        let mut used = 1;
        if starts(w, i, "ij") || starts(w, i, "ei") {
            push_all(&mut out, &["EI"]);
            used = 2;
        } else if starts(w, i, "au") || starts(w, i, "ou") {
            push_all(&mut out, &["AU"]);
            used = 2;
        } else if starts(w, i, "oo") {
            push_all(&mut out, &["o:"]);
            used = 2;
        } else if starts(w, i, "ee") {
            push_all(&mut out, &["e:"]);
            used = 2;
        } else if starts(w, i, "uu") {
            push_all(&mut out, &["y:"]);
            used = 2;
        } else if starts(w, i, "aa") {
            push_all(&mut out, &["a:"]);
            used = 2;
        } else if starts(w, i, "ch") {
            push_all(&mut out, &["x"]);
            used = 2;
        } else if starts(w, i, "qu") {
            push_all(&mut out, &["k"]);
            used = 2;
        } else {
            match c {
                'g' => push_all(&mut out, &["x"]),
                'j' => push_all(&mut out, &["y:"]),
                'w' => push_all(&mut out, &["v"]),
                'v' => push_all(&mut out, &["f"]),
                'z' => push_all(&mut out, &["s"]),
                'h' => push_all(&mut out, &["h"]),
                'r' => push_all(&mut out, &["R"]),
                'q' => push_all(&mut out, &["k"]),
                'x' => push_all(&mut out, &["k", "s"]),
                'a' => push_all(&mut out, &["a"]),
                'e' => push_all(&mut out, &["e"]),
                'i' => push_all(&mut out, &["i"]),
                'o' => push_all(&mut out, &["o"]),
                'u' => push_all(&mut out, &["u"]),
                'y' => push_all(&mut out, &["y:"]),
                'b' | 'd' | 'f' | 'k' | 'l' | 'm' | 'n' | 'p' | 's' | 't' => {
                    let s = c.to_string();
                    out.push(s);
                }
                _ => {}
            }
        }
        i += used.max(1);
    }
    out
}

// --- Swedish, Danish, Norwegian --------------------------------------------

fn nordic(w: &[char]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    let n = w.len();
    while i < n {
        let c = w[i];
        let mut used = 1;
        if starts(w, i, "sj") || starts(w, i, "sk") || starts(w, i, "stj") {
            push_all(&mut out, &["$"]);
            used = if starts(w, i, "stj") { 3 } else { 2 };
        } else if starts(w, i, "tj") || starts(w, i, "kj") {
            push_all(&mut out, &["$"]);
            used = 2;
        } else {
            match strip_accent(c) {
                'å' => push_all(&mut out, &["o:"]),
                'ä' => push_all(&mut out, &["E:"]),
                'ö' => push_all(&mut out, &["OE:"]),
                'æ' => push_all(&mut out, &["E:"]),
                'ø' => push_all(&mut out, &["OE:"]),
                'a' => push_all(&mut out, &["a"]),
                'e' => push_all(&mut out, &["e"]),
                'i' => push_all(&mut out, &["i"]),
                'o' => push_all(&mut out, &["o"]),
                'u' => push_all(&mut out, &["u"]),
                'y' => push_all(&mut out, &["y:"]),
                'j' => push_all(&mut out, &["y:"]),
                'r' => push_all(&mut out, &["R"]),
                'w' => push_all(&mut out, &["v"]),
                'h' => push_all(&mut out, &["h"]),
                'x' => push_all(&mut out, &["k", "s"]),
                'b' | 'd' | 'f' | 'g' | 'k' | 'l' | 'm' | 'n' | 'p' | 's' | 't' | 'v' => {
                    let s = strip_accent(c).to_string();
                    out.push(s);
                }
                _ => {}
            }
        }
        i += used.max(1);
    }
    out
}

// --- Finnish ----------------------------------------------------------------

fn finnish(w: &[char]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    let n = w.len();
    while i < n {
        let c = plain(w, i).unwrap_or(' ');
        let mut used = 1;
        if starts(w, i, "aa") {
            push_all(&mut out, &["a:"]);
            used = 2;
        } else if starts(w, i, "ii") {
            push_all(&mut out, &["i:"]);
            used = 2;
        } else if starts(w, i, "uu") {
            push_all(&mut out, &["u:"]);
            used = 2;
        } else if starts(w, i, "oo") {
            push_all(&mut out, &["o:"]);
            used = 2;
        } else if starts(w, i, "ee") {
            push_all(&mut out, &["e:"]);
            used = 2;
        } else if starts(w, i, "ei") {
            push_all(&mut out, &["EI"]);
            used = 2;
        } else {
            match c {
                'a' => push_all(&mut out, &["a"]),
                'e' => push_all(&mut out, &["e"]),
                'i' => push_all(&mut out, &["i"]),
                'o' => push_all(&mut out, &["o"]),
                'u' => push_all(&mut out, &["u"]),
                'y' => push_all(&mut out, &["y:"]),
                'j' => push_all(&mut out, &["y:"]),
                'h' => push_all(&mut out, &["h"]),
                'r' => push_all(&mut out, &["r"]),
                'v' => push_all(&mut out, &["v"]),
                'b' | 'd' | 'f' | 'g' | 'k' | 'l' | 'm' | 'n' | 'p' | 's' | 't' | 'z' => {
                    let s = c.to_string();
                    out.push(s);
                }
                _ => {}
            }
        }
        i += used.max(1);
    }
    out
}

// --- Polish -----------------------------------------------------------------

fn polish(w: &[char]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    let n = w.len();
    while i < n {
        let c = w[i];
        let mut used = 1;
        if starts(w, i, "sz") {
            push_all(&mut out, &["$"]);
            used = 2;
        } else if starts(w, i, "cz") {
            push_all(&mut out, &["T$"]);
            used = 2;
        } else if starts(w, i, "rz") {
            push_all(&mut out, &["Zg"]);
            used = 2;
        } else if starts(w, i, "dz") {
            push_all(&mut out, &["Dz"]);
            used = 2;
        } else if starts(w, i, "ch") {
            push_all(&mut out, &["x"]);
            used = 2;
        } else if starts(w, i, "ni") {
            push_all(&mut out, &["Gn"]);
            used = 2;
        } else {
            match strip_accent(c) {
                'ą' => push_all(&mut out, &["o):"]),
                'ę' => push_all(&mut out, &["E):"]),
                'ł' => push_all(&mut out, &["w"]),
                'j' => push_all(&mut out, &["y:"]),
                'w' => push_all(&mut out, &["v"]),
                'r' => push_all(&mut out, &["r"]),
                'h' => push_all(&mut out, &["x"]),
                'c' => push_all(&mut out, &["Ts"]),
                'z' => push_all(&mut out, &["z"]),
                'a' => push_all(&mut out, &["a"]),
                'e' => push_all(&mut out, &["e"]),
                'i' => push_all(&mut out, &["i"]),
                'o' => push_all(&mut out, &["o"]),
                'u' => push_all(&mut out, &["u"]),
                'y' => push_all(&mut out, &["i"]),
                'b' | 'd' | 'f' | 'g' | 'k' | 'l' | 'm' | 'n' | 'p' | 's' | 't' | 'v' => {
                    let s = strip_accent(c).to_string();
                    out.push(s);
                }
                _ => {}
            }
        }
        i += used.max(1);
    }
    out
}

// --- anything else ----------------------------------------------------------

fn generic(w: &[char]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0usize;
    let n = w.len();
    while i < n {
        let c = plain(w, i).unwrap_or(' ');
        let mut used = 1;
        if starts(w, i, "sch") {
            push_all(&mut out, &["$"]);
            used = 3;
        } else if starts(w, i, "ch") {
            push_all(&mut out, &["T$"]);
            used = 2;
        } else if starts(w, i, "sh") {
            push_all(&mut out, &["$"]);
            used = 2;
        } else if starts(w, i, "th") {
            push_all(&mut out, &["th"]);
            used = 2;
        } else if starts(w, i, "qu") {
            push_all(&mut out, &["k", "w"]);
            used = 2;
        } else if starts(w, i, "ts") {
            push_all(&mut out, &["Ts"]);
            used = 2;
        } else {
            match c {
                'c' => push_all(&mut out, &["k"]),
                'q' => push_all(&mut out, &["k"]),
                'x' => push_all(&mut out, &["k", "s"]),
                'j' => push_all(&mut out, &["y:"]),
                'h' => push_all(&mut out, &["h"]),
                'y' => push_all(&mut out, &["y:"]),
                'a' | 'e' | 'i' | 'o' | 'u' => {
                    let s = c.to_string();
                    out.push(s);
                }
                'b' | 'd' | 'f' | 'g' | 'k' | 'l' | 'm' | 'n' | 'p' | 'r' | 's' | 't' | 'v'
                | 'w' | 'z' => {
                    let s = c.to_string();
                    out.push(s);
                }
                _ => {}
            }
        }
        i += used.max(1);
    }
    out
}

// --- stress -----------------------------------------------------------------

fn stress(_w: &[char], symbols: &mut [String], lang: Language, accent_final: bool, raw: &[char]) {
    let vowels: Vec<usize> = symbols
        .iter()
        .enumerate()
        .filter(|(_, s)| is_vowel(s))
        .map(|(i, _)| i)
        .collect();
    if vowels.is_empty() {
        return;
    }
    let ends_with_vowel = raw.last().map(|c| is_vowel_char(strip_accent(*c))).unwrap_or(false);
    let target = if accent_final {
        *vowels.last().unwrap()
    } else {
        match lang {
            Language::Finnish | Language::Polish | Language::Turkish => vowels[0],
            Language::French => {
                if ends_with_vowel {
                    *vowels.last().unwrap()
                } else {
                    vowels[vowels.len().saturating_sub(2)]
                }
            }
            Language::German => {
                if vowels.len() > 1 && !ends_with_vowel {
                    vowels[0]
                } else {
                    vowels[vowels.len().saturating_sub(2).max(0)]
                }
            }
            _ => {
                if vowels.len() >= 2 {
                    vowels[vowels.len() - 2]
                } else {
                    vowels[0]
                }
            }
        }
    };
    let sym = symbols[target].clone();
    if !sym.starts_with('`') {
        symbols[target] = format!("`{sym}");
    }
}

// --- text → tokens ----------------------------------------------------------

/// The original engine's default pause lengths (its ShortPauseLength,
/// MediumPauseLength and LongPauseLength parameters: 50, 120 and 500 ms).
/// Its own transcription confirms 50 (`#`) and 500 (`@`) for Roberto.
pub const SHORT_PAUSE_MS: u32 = 50;
pub const MEDIUM_PAUSE_MS: u32 = 120;
pub const LONG_PAUSE_MS: u32 = 500;

fn pause_for_punctuation(c: char) -> Option<u32> {
    match c {
        ',' | ';' | ':' => Some(MEDIUM_PAUSE_MS),
        '.' | '!' | '?' | '\n' => Some(LONG_PAUSE_MS),
        '-' | '\u{2013}' | '\u{2014}' => Some(SHORT_PAUSE_MS),
        _ => None,
    }
}

/// Italian cardinal numbers written as one word, as the language does
/// ("duemilatrecentoquarantacinque"). Station announcements are full of them.
pub fn italian_number(n: u64) -> String {
    const SMALL: [&str; 20] = [
        "zero", "uno", "due", "tre", "quattro", "cinque", "sei", "sette", "otto", "nove", "dieci",
        "undici", "dodici", "tredici", "quattordici", "quindici", "sedici", "diciassette",
        "diciotto", "diciannove",
    ];
    const TENS: [&str; 10] = [
        "", "", "venti", "trenta", "quaranta", "cinquanta", "sessanta", "settanta", "ottanta",
        "novanta",
    ];
    fn below_1000(n: u64) -> String {
        let (h, r) = (n / 100, n % 100);
        let mut out = match h {
            0 => String::new(),
            1 => "cento".into(),
            _ => format!("{}cento", SMALL[h as usize]),
        };
        let rest = if r < 20 {
            if r == 0 { String::new() } else { SMALL[r as usize].to_string() }
        } else {
            let (t, u) = (r / 10, r % 10);
            let mut tens = TENS[t as usize].to_string();
            if u == 1 || u == 8 {
                tens.pop(); // ventuno, trentotto
            }
            match u {
                0 => tens,
                3 => tens + "tré",
                _ => tens + SMALL[u as usize],
            }
        };
        if !out.is_empty() && rest.starts_with("ott") {
            out.pop(); // centottanta
        }
        out + &rest
    }
    if n < 1000 {
        return if n == 0 { "zero".into() } else { below_1000(n) };
    }
    // the original reads big numbers as separate words per group:
    // 9542 "novemila cinquecento quarantadue", 2025 "duemila venticinque",
    // but 1001 "milleuno" and 100000 "centomila" (oracle, MECHANICS.md §9)
    let mut words: Vec<String> = Vec::new();
    let (millions, thousands, rest) = (n / 1_000_000, n / 1000 % 1000, n % 1000);
    if millions > 0 {
        words.push(if millions == 1 { "unmilione".into() } else { below_1000(millions) + "milioni" });
    }
    match thousands {
        0 => {}
        1 => words.push("mille".into()),
        t => words.push(below_1000(t) + "mila"),
    }
    if rest > 0 && rest < 10 && !words.is_empty() {
        let last = words.pop().unwrap();
        words.push(last + &below_1000(rest));
    } else {
        if rest >= 100 {
            words.push(below_1000(rest / 100 * 100));
        }
        if rest % 100 > 0 {
            words.push(below_1000(rest % 100));
        }
    }
    words.join(" ")
}

pub fn analyse(text: &str, lang: Language) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut word = String::new();

    macro_rules! flush {
        () => {
            if !word.is_empty() {
                let w = std::mem::take(&mut word);
                let spelled = match (lang, w.parse::<u64>()) {
                    (Language::Italian, Ok(n)) if w.len() <= 12 => italian_number(n),
                    _ => w.clone(),
                };
                for (k, part) in spelled.split(' ').enumerate() {
                if k > 0 {
                    tokens.push(Token::Pause { ms: 45, reason: "word gap".into() });
                }
                let mut symbols = word_to_symbols(part, lang);
                // number words are compounds (cento+settanta+sei): their s
                // starts a morpheme and stays voiceless, as the original says it
                if lang == Language::Italian && w.chars().all(|c| c.is_ascii_digit()) {
                    for s in symbols.iter_mut().filter(|s| s.as_str() == "z") {
                        *s = "s".into();
                    }
                }
                if lang == Language::Italian && ITALIAN_UNSTRESSED.contains(&w.as_str()) {
                    for s in symbols.iter_mut() {
                        // dropping the stress also drops the open quality
                        *s = match s.trim_start_matches('`') {
                            "E" => "e".to_string(),
                            "Oa" => "o".to_string(),
                            other => other.to_string(),
                        };
                    }
                }
                if !symbols.is_empty() {
                    tokens.push(Token::Word { text: if k == 0 { w.clone() } else { String::new() }, symbols });
                }
                }
            }
        };
    }

    for c in text.chars() {
        let lc = c.to_lowercase().next().unwrap_or(c);
        if lc.is_alphabetic() || lc.is_numeric() {
            word.push(lc);
        } else if matches!(lang, Language::French | Language::English) && (lc == '\'' || lc == '\u{2019}') {
            word.push('\'');
        } else if lang == Language::French && lc == '-' && !word.is_empty() {
            word.push('-');
        } else if lc == '\'' || lc == '\u{2019}' {
            // elision joins the two halves into one word, as the original
            // engine reads them (`l'esame` l e z `a m e, `d'acqua` dd `a k: w a)
        } else if let Some(ms) = pause_for_punctuation(lc) {
            flush!();
            let reason = match lc {
                '?' => "question",
                '!' => "exclamation",
                '.' | '\n' => "sentence break",
                _ => "phrase break",
            };
            tokens.push(Token::Pause {
                ms,
                reason: reason.into(),
            });
        } else if lc.is_whitespace() {
            flush!();
            tokens.push(Token::Pause {
                ms: 45,
                reason: "word gap".into(),
            });
        }
    }
    flush!();

    // Italian nasal assimilation across words, as the original does it:
    // `un pacco` u m, `non vieni` n o M, `in casa` i N
    if lang == Language::Italian {
        let words: Vec<usize> = (0..tokens.len()).filter(|&i| matches!(tokens[i], Token::Word { .. })).collect();
        for pair in words.windows(2) {
            let gap_only = tokens[pair[0] + 1..pair[1]]
                .iter()
                .all(|t| matches!(t, Token::Pause { reason, .. } if reason == "word gap"));
            let first = tokens[pair[1]].symbols().first().cloned().unwrap_or_default();
            let place = match first.trim_start_matches('`').chars().next() {
                Some('p' | 'b' | 'm') => Some("m"),
                Some('f' | 'v') => Some("M"),
                Some('k' | 'g') if !first.starts_with("Gl") && !first.starts_with("Gn") => Some("N"),
                _ => None,
            };
            if let (true, Some(place), Token::Word { symbols, .. }) = (gap_only, place, &mut tokens[pair[0]]) {
                if symbols.last().map(|s| s == "n") == Some(true) {
                    *symbols.last_mut().unwrap() = place.to_string();
                }
            }
        }
    }

    if lang == Language::English {
        english_connected_speech(&mut tokens);
    }
    if lang == Language::French {
        french_connected_speech(&mut tokens);
    }
    if lang == Language::Spanish {
        spanish_connected_speech(&mut tokens);
    }

    // collapse runs of pauses into the strongest one
    let mut collapsed: Vec<Token> = Vec::new();
    for t in tokens {
        if let Token::Pause { ms, reason } = &t {
            if let Some(Token::Pause {
                ms: prev,
                reason: prev_reason,
            }) = collapsed.last_mut()
            {
                *prev = (*prev).max(*ms);
                if reason != "word gap" {
                    *prev_reason = reason.clone();
                }
                continue;
            }
        }
        collapsed.push(t);
    }
    // a trailing word gap means nothing; trailing punctuation sets the tune
    if matches!(collapsed.last(), Some(Token::Pause { reason, .. }) if reason == "word gap") {
        collapsed.pop();
    }
    collapsed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn syms(w: &str, l: Language) -> String {
        word_to_symbols(w, l).join(" ")
    }

    #[test]
    fn italian_numbers() {
        assert_eq!(italian_number(3), "tre");
        assert_eq!(italian_number(21), "ventuno");
        assert_eq!(italian_number(33), "trentatré");
        assert_eq!(italian_number(180), "centottanta");
        // grouped like the original engine reads them
        assert_eq!(italian_number(9542), "novemila cinquecento quarantadue");
        assert_eq!(italian_number(2025), "duemila venticinque");
        assert_eq!(italian_number(1200), "mille duecento");
        assert_eq!(italian_number(176), "centosettantasei");
        assert_eq!(italian_number(1001), "milleuno");
        assert_eq!(italian_number(100_000), "centomila");
    }

    #[test]
    fn italian_words() {
        // Every expectation is the original engine's own transcription of the
        // word (its ttsPhoneticTranscription, PhoneticCoding=Loquendo).
        // `questo` is left out: the original says `k w `e s tt o`, the learned
        // ending table opens the e (one of the ~12% of words still differing).
        assert_eq!(syms("ciao", Language::Italian), "T$ `a o");
        assert_eq!(syms("gnocchi", Language::Italian), "Gn `Oa k: i");
        assert_eq!(syms("meglio", Language::Italian), "m `E Gl o");
        assert_eq!(syms("italiano", Language::Italian), "i tt a l i `a n o");
        assert_eq!(syms("Roberto", Language::Italian), "r o b `E r tt o");
        assert_eq!(syms("casa", Language::Italian), "k `a z a");
        assert_eq!(syms("scena", Language::Italian), "$ `E n a");
        assert_eq!(syms("città", Language::Italian), "T$ i tt: `a");
        assert_eq!(syms("becco", Language::Italian), "b `e k: o");
        assert_eq!(syms("maggio", Language::Italian), "m `a Dg: o");
        assert_eq!(syms("faccia", Language::Italian), "f `a T$: a");
        assert_eq!(syms("giallo", Language::Italian), "Dg `a l: o");
        assert_eq!(syms("legge", Language::Italian), "l `e Dg: e");
        assert_eq!(syms("binario", Language::Italian), "b i n `a r i o");
        assert_eq!(syms("mia", Language::Italian), "m `i a");
        assert_eq!(syms("buono", Language::Italian), "b w `Oa n o");
        assert_eq!(syms("duemila", Language::Italian), "dd u e m `i l a");
        assert_eq!(syms("sbaglio", Language::Italian), "z b `a Gl o");
    }

    #[test]
    fn german_words_use_the_german_alphabet() {
        // `R` and bare `t` are Katrin's symbols, not Roberto's; and Katrin
        // spells the closed lax vowel I (it has no short `i` at all)
        assert_eq!(syms("Katrin", Language::German), "k `a t R I n");
        assert_eq!(syms("Stimme", Language::German), "$ t `I m e");
        assert_eq!(syms("schön", Language::German), "$ `OE: n");
        // eu/äu is Katrin's own HOI diphthong, not an Italian `OI`
        assert_eq!(syms("neu", Language::German), "n `HOI");
        assert_eq!(syms("Häuser", Language::German), "h `HOI z e R");
    }

    #[test]
    fn italian_uses_its_own_r() {
        // Roberto's alphabet has no `R`; Katrin's has no `tt`
        let it = word_to_symbols("Roberto", Language::Italian);
        assert!(!it.iter().any(|s| s == "R"), "Italian must not emit R");
        let de = word_to_symbols("Katrin", Language::German);
        assert!(de.iter().any(|s| s == "R"), "German must use R");
        assert!(!de.iter().any(|s| s == "tt"), "German must not emit tt");
    }

    #[test]
    fn french_and_spanish_basics() {
        assert_eq!(syms("voix", Language::French), "v w `a");
        // `hola` has two vowels, so the penultimate one is stressed
        assert_eq!(syms("hola", Language::Spanish), "`o l a");
        assert_eq!(syms("chico", Language::Spanish), "T$ `i k o");
    }

    #[test]
    fn cyrillic_and_greek_reach_the_syllabary() {
        let ru = word_to_symbols("привет", Language::Russian);
        assert!(!ru.is_empty(), "Russian must produce symbols");
        assert!(ru.iter().any(|s| is_vowel(s)));
        let el = word_to_symbols("γεια", Language::Greek);
        assert!(!el.is_empty(), "Greek must produce symbols");
    }

    #[test]
    fn language_comes_from_the_voice_descriptor() {
        assert_eq!(
            Language::from_voice("German", "German,de-DE,de"),
            Language::German
        );
        assert_eq!(
            Language::from_voice("Italian", "Italian,it-IT,it"),
            Language::Italian
        );
        assert_eq!(
            Language::from_voice("English", "English,en-GB,en"),
            Language::English
        );
        assert_eq!(Language::from_voice("Klingon", "tlh"), Language::Generic);
    }

    #[test]
    fn words_and_pauses() {
        let t = analyse("Il mio nome è Roberto.", Language::Italian);
        let words: Vec<&str> = t
            .iter()
            .filter_map(|x| match x {
                Token::Word { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        // accents are kept through tokenising: the front end needs `è` to read
        // as /E/, while `bare_word` strips it when the symbols are built
        assert_eq!(words, vec!["il", "mio", "nome", "è", "roberto"]);
    }
}
