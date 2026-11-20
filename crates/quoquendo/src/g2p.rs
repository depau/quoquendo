//! Text → symbols, for as many languages as the installed voices cover.
//!
//! The language is not guessed from the text: it comes from the voice's own
//! descriptor (`MotherTongue` / `MotherTongueAliases` in `<Voice>.vde`), e.g.
//! `Italian` / `Italian,it-IT,it` for Roberto or `German` / `German,de-DE,de`
//! for Katrin. Each front end writes into the symbol alphabet that language's
//! Loquendo voices declare in their `.sde` — and those alphabets really differ:
//! Italian has `Ts Gn Gl N M` and geminates `p: tt:`; German has `Hk Hut ph
//! Pf x c R OE: y: HAI` and `R` for /r/. A language with no rules here falls
//! back to a generic grapheme map (after transliterating Cyrillic),
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
    /// The languages whose module is ported (`crate::lang`): they have no
    /// built-in rules, only the original's module from the language pack.
    pub fn is_ported(self) -> bool {
        matches!(self, Language::Italian | Language::English | Language::Spanish | Language::French | Language::German | Language::Greek | Language::Portuguese | Language::Dutch | Language::Swedish | Language::Polish | Language::Danish | Language::Russian)
    }

    /// The Windows code page the language's text is written in, in the
    /// engine and in the voices' descriptors (`DemoSentence`): what the
    /// language modules' `ELQGetLanguageNativeEncoding` give.
    pub fn code_page(self) -> u32 {
        match self {
            Language::Greek => 1253,
            Language::Polish => 1250,
            Language::Russian => 1251,
            _ => 1252,
        }
    }

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
            Language::Italian => "Il mio nome è Roberto: sono una voce maschile e la mia madrelingua è l'italiano.",
            Language::German => "Ich heiße Katrin. Ich bin die weibliche Stimme von Loquendo, meine Muttersprache ist Deutsch.",
            Language::English => "Hello. I am a Loquendo voice, and this is the text you asked me to read aloud.",
            Language::French => "Bonjour. Je suis une voix de Loquendo, et voici le texte que vous m'avez donné.",
            Language::Spanish => "Hola. Soy una voz de Loquendo y este es el texto que me has pedido leer.",
            Language::Portuguese => "Olá. Eu sou uma voz do Loquendo e este é o texto que você pediu para eu ler.",
            Language::Catalan => "Hola. Sóc una veu de Loquendo i aquest és el text que m'has demanat llegir.",
            Language::Dutch => "Hallo. Ik ben een stem van Loquendo en dit is de tekst die je mij hebt gegeven.",
            Language::Swedish => "Hej. Jag är en röst från Loquendo och detta är texten du bad mig läsa.",
            Language::Danish => "Hej. Jeg er en stemme fra Loquendo, og dette er teksten du bad mig læse.",
            Language::Norwegian => "Hei. Jeg er en stemme fra Loquendo, og dette er teksten du ba meg lese.",
            Language::Finnish => "Hei. Olen Loquendo-ääni, ja tämä on teksti, jonka pyysit minua lukemaan.",
            Language::Polish => "Dzień dobry. Jestem głosem Loquendo i to jest tekst, który mam przeczytać.",
            Language::Russian => "Привет. Я голос Loquendo, и это текст, который вы попросили прочитать.",
            Language::Greek => "Γεια σας. Είμαι φωνή του Loquendo και αυτό είναι το κείμενο που διαβάζω.",
            Language::Turkish => "Merhaba. Ben bir Loquendo sesiyim ve bu okumamı istediğiniz metin.",
            Language::Generic => "Hello. This voice is driven by the Loquendo pipeline.",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Word { text: String, symbols: Vec<String> },
    Pause { ms: u32, reason: String },
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
    w.chars().map(|c| c.to_lowercase().next().unwrap_or(c)).filter(|c| c.is_alphabetic()).collect()
}

fn starts(w: &[char], i: usize, pat: &str) -> bool {
    pat.chars().enumerate().all(|(k, c)| w.get(i + k) == Some(&c))
}

fn plain(w: &[char], i: usize) -> Option<char> {
    w.get(i).map(|c| strip_accent(*c))
}

// --- entry point ------------------------------------------------------------

// --- the language's own data: its dictionary --------------------------------

/// What a language's installer files give the front end: the dictionary
/// (`<MotherTongue>.dbl`, SQLite table `lessico`: spelling, pronunciation…).
use std::collections::HashMap;

pub struct LanguageData {
    dictionary: HashMap<String, Vec<String>>,
}

/// A voice's front end: its language, and that language's own data
/// (dictionary and letter-to-sound model, or the ported language module).
/// One per voice: English Us/Gb/Au share a `Language` but not a dictionary.
#[derive(Clone)]
pub struct Frontend {
    pub lang: Language,
    data: Option<std::sync::Arc<LanguageData>>,
    /// the original's language module, ported (`lang`), reading its tables
    /// from the language pack's DLL
    module: Option<std::sync::Arc<dyn crate::lang::Module>>,
    /// the voice library's own phone stage after the module's FON
    /// (`voicefon`); the fitted rules already give the voice's phones
    voice: Option<std::sync::Arc<crate::voicefon::VoiceFon>>,
}

impl Frontend {
    /// The built-in rules alone.
    pub fn rules(lang: Language) -> Frontend {
        Frontend {
            lang,
            data: None,
            module: None,
            voice: None,
        }
    }

    /// Use the language module the voice's `.lde` names (its DLL's bytes)
    /// when it is one quoquendo has ported; otherwise keep this front end.
    pub fn with_module(mut self, files: &crate::lang::ModuleFiles) -> Frontend {
        if let Some(m) = crate::lang::load(self.lang, files) {
            self.module = Some(m);
        }
        self
    }

    /// Run the voice library's phone stage after the language module.
    pub fn with_voice(mut self, voice: crate::voicefon::VoiceFon) -> Frontend {
        self.voice = Some(std::sync::Arc::new(voice));
        self
    }

    /// The language module's FON alone, as the original's
    /// `ttsPhoneticTranscription` (`.fon`) gives it: no voice stage.
    pub fn without_voice(mut self) -> Frontend {
        self.voice = None;
        self
    }

    /// Whether the original's language module drives this front end.
    pub fn has_module(&self) -> bool {
        self.module.is_some()
    }

    /// Read a language's dictionary from its `LexicalDB` (may be missing).
    pub fn from_files(lang: Language, dbl: Option<&[u8]>) -> Frontend {
        let mut dictionary = HashMap::new();
        if let Some(rows) = dbl.and_then(|d| crate::sqlite::read_table(d, "lessico").ok()) {
            for r in rows {
                let (Some(word), Some(pron)) = (r.first().and_then(|v| v.text()), r.get(1).and_then(|v| v.text())) else {
                    continue;
                };
                let syms: Vec<String> = pron.split([' ', '-']).filter(|s| !s.is_empty()).map(str::to_string).collect();
                if !syms.is_empty() {
                    dictionary.entry(word.to_lowercase()).or_insert(syms);
                }
            }
        }
        let data = LanguageData { dictionary };
        Frontend {
            lang,
            data: Some(std::sync::Arc::new(data)),
            module: None,
            voice: None,
        }
    }

    /// Words in the language's dictionary.
    pub fn dictionary_len(&self) -> usize {
        self.data.as_ref().map(|d| d.dictionary.len()).unwrap_or(0)
    }
}

fn language_data_lookup(word: &str, fe: &Frontend) -> Option<Vec<String>> {
    fe.data.as_ref()?.dictionary.get(&word.to_lowercase()).cloned()
}

/// A word's phones by the built-in rules, for the languages without a
/// ported module (Norwegian…). The ported languages (Italian, English,
/// Spanish, French, German, Greek, Portuguese, Dutch, Swedish, Polish, Danish, Russian) have no rules: without their
/// module they read nothing, and the voice reports its language data missing.
pub fn word_to_symbols(word: &str, fe: &Frontend) -> Vec<String> {
    let lang = fe.lang;
    // the language's own data from the installer (`Frontend::from_files`)
    if let Some(p) = language_data_lookup(word, fe) {
        return p;
    }
    if lang.is_ported() {
        return Vec::new();
    }
    let raw = bare_word(word);
    if raw.is_empty() {
        return Vec::new();
    }
    let accent_final = word.chars().last().map(|c| ACCENTS.iter().any(|(a, _)| *a == c)).unwrap_or(false);
    let w = raw;
    let mut out = match lang {
        Language::Italian | Language::German | Language::Spanish | Language::English | Language::French | Language::Greek | Language::Portuguese | Language::Dutch | Language::Swedish | Language::Polish | Language::Danish | Language::Russian => unreachable!(),
        Language::Catalan => catalan(&w),
        Language::Norwegian => nordic(&w),
        Language::Finnish => finnish(&w),
        Language::Turkish | Language::Generic => generic(&w),
    };
    stress(&mut out, lang, accent_final);
    out
}

fn push_all(out: &mut Vec<String>, items: &[&str]) {
    for s in items {
        out.push((*s).to_string());
    }
}

// --- Catalan ------------------------------------------------------------------

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
        } else if starts(w, i, "ig") || starts(w, i, "tx") {
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

// --- Norwegian -----------------------------------------------------------------

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
                'b' | 'd' | 'f' | 'g' | 'k' | 'l' | 'm' | 'n' | 'p' | 'r' | 's' | 't' | 'v' | 'w' | 'z' => {
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

fn stress(symbols: &mut [String], lang: Language, accent_final: bool) {
    let vowels: Vec<usize> = symbols.iter().enumerate().filter(|(_, s)| is_vowel(s)).map(|(i, _)| i).collect();
    if vowels.is_empty() {
        return;
    }
    let target = if accent_final {
        *vowels.last().unwrap()
    } else {
        match lang {
            Language::Finnish | Language::Turkish => vowels[0],
            _ if vowels.len() >= 2 => vowels[vowels.len() - 2],
            _ => vowels[0],
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

pub fn analyse(text: &str, fe: &Frontend) -> Vec<Token> {
    // the original's own module, when the voice's language pack has it
    if let Some(m) = &fe.module {
        return m.analyse(text, fe.voice.as_deref());
    }
    let mut tokens: Vec<Token> = Vec::new();
    let mut word = String::new();

    macro_rules! flush {
        () => {
            if !word.is_empty() {
                let w = std::mem::take(&mut word);
                let symbols = word_to_symbols(&w, fe);
                if !symbols.is_empty() {
                    tokens.push(Token::Word { text: w, symbols });
                }
            }
        };
    }

    for c in text.chars() {
        let lc = c.to_lowercase().next().unwrap_or(c);
        if lc.is_alphabetic() || lc.is_numeric() {
            word.push(lc);
        } else if lc == '\'' || lc == '\u{2019}' {
            // an elision joins the two halves into one word
        } else if let Some(ms) = pause_for_punctuation(lc) {
            flush!();
            let reason = match lc {
                '?' => "question",
                '!' => "exclamation",
                '.' | '\n' => "sentence break",
                _ => "phrase break",
            };
            tokens.push(Token::Pause { ms, reason: reason.into() });
        } else if lc.is_whitespace() {
            flush!();
            tokens.push(Token::Pause {
                ms: 45,
                reason: "word gap".into(),
            });
        }
    }
    flush!();

    // collapse runs of pauses into the strongest one
    let mut collapsed: Vec<Token> = Vec::new();
    for t in tokens {
        if let Token::Pause { ms, reason } = &t {
            if let Some(Token::Pause { ms: prev, reason: prev_reason }) = collapsed.last_mut() {
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
        word_to_symbols(w, &Frontend::rules(l)).join(" ")
    }

    #[test]
    fn ported_languages_have_no_rules() {
        // their phones come only from the original's module
        for l in [Language::Italian, Language::English, Language::Spanish, Language::French, Language::German, Language::Portuguese, Language::Dutch] {
            assert!(syms("casa", l).is_empty(), "{l:?}");
        }
        assert!(!syms("casa", Language::Finnish).is_empty());
    }

    #[test]
    fn language_comes_from_the_voice_descriptor() {
        assert_eq!(Language::from_voice("German", "German,de-DE,de"), Language::German);
        assert_eq!(Language::from_voice("Italian", "Italian,it-IT,it"), Language::Italian);
        assert_eq!(Language::from_voice("English", "English,en-GB,en"), Language::English);
        assert_eq!(Language::from_voice("Klingon", "tlh"), Language::Generic);
    }

    #[test]
    fn words_and_pauses() {
        let t = analyse("O meu nome é Roberto.", &Frontend::rules(Language::Finnish));
        let words: Vec<&str> = t
            .iter()
            .filter_map(|x| match x {
                Token::Word { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        // accents are kept through tokenising, `bare_word` strips them when
        // the symbols are built
        assert_eq!(words, vec!["o", "meu", "nome", "é", "roberto"]);
    }
}
