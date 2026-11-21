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
    /// Central Catalan (Jordi, `CatalanCe`)
    Catalan,
    /// Valencian (Empar, `CatalanVa`): Catalan's rules without its vowel
    /// reduction, with `v` and `Dg`
    Valencian,
    Galician,
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
        } else if has("valencian") || has("catalanva") || has("ca-es-valencia") {
            Language::Valencian
        } else if has("catalan") || has("ca-es") {
            Language::Catalan
        } else if has("galician") || has("gl-es") {
            Language::Galician
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
            Language::Valencian => "Valencian",
            Language::Galician => "Galician",
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

    pub const ALL: [Language; 19] = [
        Language::Italian,
        Language::English,
        Language::German,
        Language::French,
        Language::Spanish,
        Language::Portuguese,
        Language::Catalan,
        Language::Valencian,
        Language::Galician,
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
            Language::Valencian => "Hola. Sóc una veu de Loquendo i este és el text que m'has demanat llegir.",
            Language::Galician => "Ola. Son unha voz de Loquendo e este é o texto que me pediches ler.",
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

/// Lower-case letters only (and Catalan's `·` of `l·l`); accents are kept so
/// each language can read them.
pub fn bare_word(w: &str) -> Vec<char> {
    w.chars().map(|c| c.to_lowercase().next().unwrap_or(c)).filter(|c| c.is_alphabetic() || *c == '·').collect()
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
        // the dictionaries write the language's phones, which the voice's
        // phone stage (not ported for these languages) turns into its own:
        // Galician's nasal vowels (`chegan` → T$ `e GH a) N) Carmela
        // recorded oral, her sentences transcribed `u N`, never `u) N`
        if lang == Language::Galician {
            return p.into_iter().map(|s| s.replace(')', "")).collect();
        }
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
    let final_stress = match lang {
        Language::Catalan | Language::Valencian => catalan_oxytone(&w),
        Language::Galician => galician_oxytone(&w),
        _ => accent_final,
    };
    let mut out = match lang {
        Language::Italian | Language::German | Language::Spanish | Language::English | Language::French | Language::Greek | Language::Portuguese | Language::Dutch | Language::Swedish | Language::Polish | Language::Danish | Language::Russian => unreachable!(),
        Language::Catalan => catalan(&w, false),
        Language::Valencian => catalan(&w, true),
        Language::Galician => galician(&w),
        Language::Norwegian => nordic(&w),
        Language::Finnish => finnish(&w),
        Language::Turkish | Language::Generic => generic(&w),
    };
    // the rules stress only what a written accent marks; an unmarked stressed
    // e/o is more often open than not in these languages' dictionaries
    let written = out.iter().any(|s| s.starts_with('`'));
    stress(&mut out, lang, final_stress);
    if !written && matches!(lang, Language::Catalan | Language::Valencian | Language::Galician) {
        for s in out.iter_mut().filter(|s| s.starts_with('`')) {
            if let Some(rest) = s.strip_prefix("`e").or_else(|| s.strip_prefix("`o")).map(str::to_string) {
                *s = format!("{}{rest}", if s.starts_with("`e") { "`E" } else { "`Oa" });
            }
        }
    }
    // Central Catalan reduces its unstressed a/e to the schwa `HEh` and o to
    // u (`academia` → HEh k HEh DH `E m i HEh); Valencian keeps them
    if lang == Language::Catalan {
        for s in out.iter_mut() {
            let reduced = match s.as_str() {
                "a" | "e" | "E" => "HEh",
                "o" | "Oa" => "u",
                _ => continue,
            };
            *s = reduced.to_string();
        }
    }
    out
}

fn push_all(out: &mut Vec<String>, items: &[&str]) {
    for s in items {
        out.push((*s).to_string());
    }
}

// --- Catalan, Valencian and Galician ------------------------------------------
//
// Fitted to the dictionaries of the language distributions (`CatalanCe.dbl`,
// `CatalanVa.dbl`, `Galician.dbl`) and written in their voices' phone sets,
// which have no plain `t` or `d`: t is `tt`, d `dd`, and b/d/g soften to
// `BH DH GH` after anything but a pause or a nasal. A written accent
// stresses its vowel and gives its quality (è `E`, ò `Oa`); the
// `*_oxytone` rules place the rest (`stress`).

/// A vowel letter of these orthographies, accented or not.
fn vowel_letter(c: Option<&char>) -> bool {
    c.is_some_and(|c| matches!(strip_accent(*c), 'a' | 'e' | 'i' | 'o' | 'u'))
}

/// Whether the letter at `i` is e or i (c, g and qu/gu change before them).
fn front_at(w: &[char], i: usize) -> bool {
    matches!(w.get(i).map(|c| strip_accent(*c)), Some('e' | 'i'))
}

/// Whether b/d/g/s soften here: after a vowel, a glide or a consonant
/// other than a nasal, not at the start of the word.
fn soft(out: &[String]) -> bool {
    !out.is_empty() && !matches!(out.last().map(String::as_str), Some("m" | "n" | "N"))
}

fn after_vowel(out: &[String]) -> bool {
    out.last().is_some_and(|s| is_vowel(s) || s == "j" || s == "w")
}

/// An accented vowel letter as its stressed phone (`None` for the rest).
fn accented(c: char) -> Option<&'static str> {
    Some(match c {
        'à' | 'á' => "`a",
        'è' => "`E",
        'é' => "`e",
        'í' => "`i",
        'ò' => "`Oa",
        'ó' => "`o",
        'ú' => "`u",
        _ => return None,
    })
}

/// i/y and u/ü: the vowel, or the glide (`j`, `w`) after a vowel or, with
/// `rising`, before one.
fn high(out: &mut Vec<String>, c: char, rising: bool) {
    let glide = rising || out.last().is_some_and(|s| is_vowel(s));
    let s = match (strip_accent(c), glide) {
        ('u', true) => "w",
        ('u', false) => "u",
        (_, true) => "j",
        (_, false) => "i",
    };
    out.push(s.into());
}

/// qu and gu: u is silent before e/i, `w` before a/o and as ü. Pushes the
/// `w` and says how many letters the pair takes (1 when u is not part of it).
fn u_after(out: &mut Vec<String>, w: &[char], i: usize) -> usize {
    let u = w.get(i + 1).copied();
    if !matches!(u, Some('u' | 'ü')) || !vowel_letter(w.get(i + 2)) {
        return 1;
    }
    if u == Some('ü') || !front_at(w, i + 2) {
        out.push("w".into());
    }
    2
}

/// Catalan stresses the last syllable of a word ending in a consonant or a
/// falling diphthong (-iu, -eus), except -s after a vowel and -en, -in(s).
fn catalan_oxytone(w: &[char]) -> bool {
    let c: Vec<char> = w.iter().rev().take(3).map(|c| strip_accent(*c)).collect();
    let v = |k: usize| c.get(k).is_some_and(|c| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u'));
    let diphthong = |k: usize| matches!(c.get(k), Some('i' | 'u')) && v(k + 1);
    match c.first() {
        None => false,
        Some('s') if c.get(1) == Some(&'n') => !matches!(c.get(2), Some('e' | 'i')),
        Some('s') => diphthong(1) || !v(1),
        Some('n') => !matches!(c.get(1), Some('e' | 'i')),
        Some(_) if v(0) => diphthong(0),
        Some(_) => true,
    }
}

/// Galician stresses the last syllable of a word ending in a consonant other
/// than n or s, or in a falling diphthong (`despois`, `falou`).
fn galician_oxytone(w: &[char]) -> bool {
    let c: Vec<char> = w.iter().rev().take(3).map(|c| strip_accent(*c)).collect();
    let v = |k: usize| c.get(k).is_some_and(|c| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u'));
    let diphthong = |k: usize| matches!(c.get(k), Some('i' | 'u')) && v(k + 1);
    match c.first() {
        None => false,
        Some('s' | 'n') => diphthong(1),
        Some(_) if v(0) => diphthong(0),
        Some(_) => true,
    }
}

/// Catalan (`valencian` false: Jordi's Central) or Valencian (Empar's). They
/// differ where the voices' phone sets do: Empar has no `Zg` (j and g before
/// e/i are `Dg`) and keeps v as `v`; Jordi merges v into b and drops the i of
/// -ix- (`caixa` → k `a $ HEh).
fn catalan(w: &[char], valencian: bool) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let n = w.len();
    let zh = if valencian { "Dg" } else { "Zg" };
    let polysyllable = w.iter().filter(|c| vowel_letter(Some(c))).count() > 1;
    let mut i = 0usize;
    while i < n {
        let c = w[i];
        let next = w.get(i + 1).copied();
        let last = i + 1 == n;
        let mut used = 1;
        if let Some(v) = accented(c) {
            out.push(v.into());
            i += 1;
            continue;
        }
        match c {
            'a' | 'e' | 'o' => out.push(c.to_string()),
            'ï' => out.push("i".into()),
            // -ig after a vowel (`roig`, `passeig`): `T$`
            'i' if next == Some('g') && i + 2 == n && vowel_letter(i.checked_sub(1).and_then(|p| w.get(p))) => {
                out.push("T$".into());
                used = 2;
            }
            // the i of -ix- after a vowel belongs to the palatal
            'i' if next == Some('x') && out.last().is_some_and(|s| is_vowel(s)) => {
                if valencian {
                    out.push("j".into());
                }
            }
            'i' | 'u' | 'ü' | 'y' => {
                let rising = out.is_empty() && vowel_letter(w.get(i + 1));
                high(&mut out, c, rising)
            }
            'v' if valencian => out.push("v".into()),
            'b' | 'v' => out.push(if last && c == 'b' { "p" } else if soft(&out) { "BH" } else { "b" }.into()),
            'd' => out.push(if last {
                "tt"
            } else if soft(&out) && !matches!(out.last().map(String::as_str), Some("l" | "Gl")) {
                "DH"
            } else {
                "dd"
            }.into()),
            // -nt(s): the t is silent (`cantant` → k HEh n tt `a n)
            't' if i > 0 && w[i - 1] == 'n' && (last || (i + 2 == n && next == Some('s'))) => {}
            't' if next == Some('x') => {
                out.push("T$".into());
                used = 2;
            }
            't' if starts(w, i, "tll") => {
                push_all(&mut out, &["Gl", "Gl"]);
                used = 3;
            }
            't' if next == Some('z') => {
                out.push("Dz".into());
                used = 2;
            }
            't' if next == Some('j') || (next == Some('g') && front_at(w, i + 2)) => {
                out.push("Dg".into());
                used = 2;
            }
            't' if next == Some('s') && i + 2 == n => {
                out.push("Ts".into());
                used = 2;
            }
            't' => out.push("tt".into()),
            'c' if front_at(w, i + 1) => out.push("s".into()),
            'c' | 'k' => out.push("k".into()),
            'ç' => out.push("s".into()),
            'q' => {
                out.push("k".into());
                used = u_after(&mut out, w, i);
            }
            'g' if front_at(w, i + 1) => out.push(zh.into()),
            // -ng final or before a consonant is `N` alone (`sang` → s `a N)
            'g' if i > 0 && w[i - 1] == 'n' && (last || !vowel_letter(next.as_ref()) && next != Some('u')) => {}
            'g' => {
                out.push(if last { "k" } else if soft(&out) { "GH" } else { "g" }.into());
                used = u_after(&mut out, w, i);
            }
            'j' => out.push(zh.into()),
            // l·l is one l (`al·legro` → a l `e GH hr o)
            'l' if next == Some('·') => {
                out.push("l".into());
                used = if w.get(i + 2) == Some(&'l') { 3 } else { 2 };
            }
            'l' if next == Some('l') => {
                out.push("Gl".into());
                used = 2;
            }
            'n' if next == Some('y') => {
                out.push("Gn".into());
                used = 2;
            }
            'n' => out.push(match next {
                Some('k' | 'q') => "N",
                Some('c' | 'g') if !front_at(w, i + 2) => "N",
                Some('p' | 'b' | 'm' | 'f' | 'v') => "m",
                _ => "n",
            }.into()),
            'r' if next == Some('r') => {
                out.push("r:".into());
                used = 2;
            }
            // infinitives and -dor drop their final r (`afixar` → HEh f i k s `a)
            'r' if last && polysyllable && matches!(w[i - 1], 'a' | 'e' | 'i' | 'o') => {}
            'r' => out.push(if out.is_empty() || last || !vowel_letter(next.as_ref()) || matches!(out.last().map(String::as_str), Some("n" | "l" | "s" | "z" | "Gl")) {
                "r:"
            } else {
                "hr"
            }.into()),
            's' if next == Some('s') => {
                out.push("s".into());
                used = 2;
            }
            's' => out.push(if after_vowel(&out) && vowel_letter(w.get(i + 1)) { "z" } else { "s" }.into()),
            'x' if out.is_empty() || (i >= 2 && w[i - 1] == 'i' && vowel_letter(w.get(i - 2))) => out.push("$".into()),
            'x' => push_all(&mut out, &["k", "s"]),
            'z' => out.push("z".into()),
            'f' | 'l' | 'm' | 'p' | 'w' => out.push(c.to_string()),
            _ => {}
        }
        i += used;
    }
    out
}

/// Galician, as Carmela recorded it: no vowel reduction, θ (`Th`) for z and
/// c before e/i, ll `J`, nh and a final n or m `N`, n before f `M`
/// (`triunfo` → tt hr i `u M f o), and oral vowels where the dictionary
/// writes nasal ones (`word_to_symbols`).
fn galician(w: &[char]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let n = w.len();
    let mut i = 0usize;
    while i < n {
        let c = w[i];
        let next = w.get(i + 1).copied();
        let last = i + 1 == n;
        let mut used = 1;
        if let Some(v) = accented(c) {
            out.push(v.into());
            i += 1;
            continue;
        }
        match c {
            'a' | 'e' | 'o' => out.push(c.to_string()),
            'i' | 'u' | 'ü' | 'y' => high(&mut out, c, vowel_letter(w.get(i + 1))),
            'b' | 'v' => out.push(if soft(&out) { "BH" } else { "b" }.into()),
            'd' => out.push(if soft(&out) && out.last().map(String::as_str) != Some("l") { "DH" } else { "dd" }.into()),
            't' => out.push("tt".into()),
            'c' if next == Some('h') => {
                out.push("T$".into());
                used = 2;
            }
            'c' | 'z' if c == 'z' || front_at(w, i + 1) => out.push("Th".into()),
            'c' | 'k' => out.push("k".into()),
            'q' => {
                out.push("k".into());
                used = u_after(&mut out, w, i);
            }
            'g' => {
                out.push(if soft(&out) { "GH" } else { "g" }.into());
                used = u_after(&mut out, w, i);
            }
            'j' => out.push("$".into()),
            // x is ʃ, but ks after e (`exame`, `anexo`) and before a consonant
            'x' if (i > 0 && strip_accent(w[i - 1]) == 'e') || (next.is_some() && !vowel_letter(next.as_ref())) => push_all(&mut out, &["k", "s"]),
            'x' => out.push("$".into()),
            'l' if next == Some('l') => {
                out.push("J".into());
                used = 2;
            }
            'ñ' => out.push("Gn".into()),
            'n' if next == Some('h') => {
                out.push("N".into());
                used = 2;
            }
            'n' => out.push(match next {
                None | Some('k' | 'q' | 'g') => "N",
                Some('c') if !front_at(w, i + 2) => "N",
                Some('p' | 'b' | 'm' | 'v') => "m",
                Some('f') => "M",
                _ => "n",
            }.into()),
            'm' => out.push(if last { "N" } else { "m" }.into()),
            'r' if next == Some('r') => {
                out.push("r:".into());
                used = 2;
            }
            'r' => out.push(if out.is_empty() || matches!(out.last().map(String::as_str), Some("n" | "N" | "l" | "s")) { "r:" } else { "hr" }.into()),
            'f' | 'l' | 'p' | 's' | 'w' => out.push(c.to_string()),
            _ => {}
        }
        i += used;
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

/// Stress a word's penultimate vowel, or its last with `final_stress`
/// (its first in Finnish and Turkish), unless the rules have already
/// stressed the vowel a written accent marks.
fn stress(symbols: &mut [String], lang: Language, final_stress: bool) {
    if symbols.iter().any(|s| s.starts_with('`')) {
        return;
    }
    let vowels: Vec<usize> = symbols.iter().enumerate().filter(|(_, s)| is_vowel(s)).map(|(i, _)| i).collect();
    if vowels.is_empty() {
        return;
    }
    let target = if final_stress {
        *vowels.last().unwrap()
    } else {
        match lang {
            Language::Finnish | Language::Turkish => vowels[0],
            _ if vowels.len() >= 2 => vowels[vowels.len() - 2],
            _ => vowels[0],
        }
    };
    symbols[target].insert(0, '`');
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
        // Catalan's l·l keeps its dot: the dictionaries write `al·legro`
        if lc.is_alphabetic() || lc.is_numeric() || lc == '·' {
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
