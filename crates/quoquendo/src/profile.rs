//! Speech profiles: what a voice says before the engine reads it.
//!
//! A profile is a small, portable JSON document: how dates and ranges are
//! read, which modern text conventions are spoken (units, handles, emoji,
//! inclusive Italian), how joins sound, and the user's own say-before /
//! say-after text and find-and-replace rules. Built-ins are read-only; a user
//! duplicates one and edits the copy.

use crate::g2p::Language;
use regex::{Captures, Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::ops::Range;

/// How a date is spoken.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dates {
    /// `22 novembre 2026`
    Full,
    /// `22 11 2026` (the engine reads the numbers)
    Numeric,
    /// `22 11 26` (the year as written)
    Verbatim,
    /// leave the date to the engine (Loquendo's own reading)
    #[default]
    AsWritten,
}

/// How a two-digit year becomes a four-digit one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Year {
    /// `current year + 10` and later are 20xx, older are 19xx
    NextTen,
    /// Loquendo's cutoff: `< 20` is 20xx, otherwise 19xx
    #[default]
    Original,
}

/// How a numeric range (`10-15`) is spoken.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ranges {
    /// `da 10 a 15` / `from 10 to 15`
    Words,
    /// `10 15`
    Numbers,
    /// leave it to the engine (`dieci meno quindici`)
    #[default]
    AsWritten,
}

/// What to do with an emoji.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Emoji {
    /// say a description (`treno`)
    Describe,
    /// drop it
    #[default]
    Skip,
}

/// How recorded pieces are joined.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Joins {
    /// cross-fade the cuts
    Smooth,
    /// the original's hard cuts
    #[default]
    Original,
}

/// One find-and-replace rule, optionally limited to one language.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Replacement {
    pub find: String,
    pub replace: String,
    /// `None` (or `All`) applies to every language
    pub language: Option<String>,
    pub regex: bool,
    pub match_case: bool,
    pub enabled: bool,
}

impl Default for Replacement {
    fn default() -> Self {
        Replacement {
            find: String::new(),
            replace: String::new(),
            language: None,
            regex: false,
            match_case: true,
            enabled: true,
        }
    }
}

/// A speech profile. All fields have defaults, so a partial JSON imports.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub name: String,
    /// not written to JSON: the book marks its built-ins
    #[serde(skip)]
    pub builtin: bool,
    pub dates: Dates,
    pub year: Year,
    pub ranges: Ranges,
    pub math: bool,
    pub inclusive: bool,
    pub units: bool,
    pub handles: bool,
    pub federated: bool,
    pub emails: bool,
    pub links: bool,
    pub cashtags: bool,
    pub currencies: bool,
    pub crypto: bool,
    pub emoji: Emoji,
    pub joins: Joins,
    pub say_before: String,
    pub say_after: String,
    /// `None` (or `All`) applies say-before / say-after to every language
    pub rules_language: Option<String>,
    pub replacements: Vec<Replacement>,
    /// the Italian station announcement easter egg
    pub station_warning: bool,
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            name: String::new(),
            builtin: false,
            dates: Dates::AsWritten,
            year: Year::Original,
            ranges: Ranges::AsWritten,
            math: false,
            inclusive: false,
            units: false,
            handles: false,
            federated: false,
            emails: false,
            links: false,
            cashtags: false,
            currencies: false,
            crypto: false,
            emoji: Emoji::Skip,
            joins: Joins::Original,
            say_before: String::new(),
            say_after: String::new(),
            rules_language: None,
            replacements: Vec::new(),
            station_warning: false,
        }
    }
}

impl Profile {
    /// The profile new installations use: modern conventions, smoothed joins.
    pub fn modern() -> Profile {
        Profile {
            name: "Modern".into(),
            builtin: true,
            dates: Dates::Full,
            year: Year::NextTen,
            ranges: Ranges::Words,
            math: true,
            inclusive: true,
            units: true,
            handles: true,
            federated: true,
            emails: true,
            links: true,
            cashtags: true,
            currencies: true,
            crypto: true,
            emoji: Emoji::Describe,
            joins: Joins::Smooth,
            ..Profile::default()
        }
    }

    /// The profile that reads like the original engine.
    pub fn faithful() -> Profile {
        Profile {
            name: "Faithful".into(),
            builtin: true,
            ..Profile::default()
        }
    }

    pub fn builtins() -> [Profile; 2] {
        [Profile::modern(), Profile::faithful()]
    }

    /// A copy under `name` that the user can edit.
    pub fn duplicate(&self, name: &str) -> Profile {
        Profile {
            name: name.to_string(),
            builtin: false,
            ..self.clone()
        }
    }

    /// Check a profile before it is saved or imported.
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("a profile needs a name".into());
        }
        if self.name.len() > 80 {
            return Err("a profile name must be at most 80 characters".into());
        }
        if let Some(lang) = self.language() {
            if lang_name(&lang).is_none() && !lang.eq_ignore_ascii_case("all") {
                return Err(format!("unknown language: {lang}"));
            }
        }
        for (i, r) in self.replacements.iter().enumerate() {
            if let Some(lang) = r.language.as_deref().filter(|l| !l.eq_ignore_ascii_case("all")) {
                if lang_name(lang).is_none() {
                    return Err(format!("replacement {}: unknown language: {lang}", i + 1));
                }
            }
            if r.enabled && !r.find.is_empty() && r.regex {
                Regex::new(&r.find).map_err(|e| format!("replacement {}: {e}", i + 1))?;
            }
        }
        Ok(())
    }

    /// The say-before / say-after language, normalised (`All` and empty mean every language).
    pub fn language(&self) -> Option<String> {
        self.rules_language
            .as_deref()
            .filter(|l| !l.trim().is_empty() && !l.eq_ignore_ascii_case("all"))
            .map(str::to_string)
    }

    /// This profile applied to `text` at the current year.
    pub fn apply(&self, text: &str, lang: Language) -> Applied {
        self.apply_in(text, lang, current_year())
    }

    /// `apply` with an explicit current year, for deterministic tests.
    pub fn apply_in(&self, text: &str, lang: Language, year: u32) -> Applied {
        let words = Words::for_language(lang);
        let mut m = Mapped::new(text);
        let mut warnings = Vec::new();

        // Say before / after, scoped to a language
        if language_matches(self.language().as_deref(), lang) {
            if !self.say_before.trim().is_empty() {
                m.prepend(&format!("{} ", self.say_before.trim()));
            }
            if !self.say_after.trim().is_empty() {
                m.append(&format!(" {}", self.say_after.trim()));
            }
        }

        // User replacements, top to bottom
        for r in &self.replacements {
            if !r.enabled || r.find.is_empty() || !language_matches(r.language.as_deref(), lang) {
                continue;
            }
            let re = match RegexBuilder::new(&r.find).case_insensitive(!r.match_case).build() {
                Ok(re) => re,
                Err(e) => {
                    warnings.push(format!("replacement “{}” skipped: {e}", r.find));
                    continue;
                }
            };
            m = m.rewrite(&re, &r.replace, r.regex);
        }

        // Mail first: its domain would otherwise look like a link.
        if self.emails {
            m = emails(m, lang, &words);
        }
        if self.federated {
            m = federated(m, lang, &words);
        }
        // Then links: their scheme and dots contain characters the later
        // steps would otherwise take apart.
        if self.links {
            m = links(m, lang, &words);
        }
        if self.handles {
            m = handles(m, lang, &words);
        }
        if self.cashtags {
            m = cashtags(m, lang);
        }
        if self.currencies {
            m = currencies(m, lang, &words);
        }
        if self.crypto {
            m = crypto(m, lang, &words);
        }
        if self.units {
            m = units(m, lang, &words);
        }
        if self.inclusive && lang == Language::Italian {
            m = inclusive(m);
        }
        m = emojis(m, lang, &words, self.emoji == Emoji::Describe);
        if self.dates != Dates::AsWritten {
            m = dates(m, self.dates, self.year, lang, year, &words);
        }
        if self.math {
            m = math(m, lang, &words);
        }
        if self.ranges != Ranges::AsWritten {
            m = ranges(m, self.ranges, lang, &words);
        }

        if self.station_warning && lang == Language::Italian && !m.text.contains(STATION_WARNING) {
            m.append(&format!(" {STATION_WARNING}"));
        }

        let m = m.tidy();
        Applied {
            text: m.text,
            chunks: m.chunks,
            warnings,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Result<Profile, String> {
        let mut p: Profile = serde_json::from_str(json).map_err(|e| format!("not a profile: {e}"))?;
        p.builtin = false;
        p.validate()?;
        if Profile::builtins().iter().any(|b| b.name.eq_ignore_ascii_case(&p.name)) {
            p.name = unique_name(&p.name, Profile::builtins().iter().map(|b| b.name.as_str()));
        }
        Ok(p)
    }
}

/// The station announcement, exactly as the original says it.
pub const STATION_WARNING: &str = "Attenzione! Allontanarsi dalla linea gialla.";

/// A profile file: the active profile's name and the user's custom profiles.
/// Built-ins are added when the book is read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfileBook {
    pub version: u32,
    pub active: String,
    pub profiles: Vec<Profile>,
}

impl Default for ProfileBook {
    fn default() -> Self {
        ProfileBook {
            version: 1,
            active: Profile::modern().name,
            profiles: Vec::new(),
        }
    }
}

impl ProfileBook {
    pub fn all(&self) -> Vec<Profile> {
        let mut out = Profile::builtins().to_vec();
        let builtins: Vec<String> = out.iter().map(|p| p.name.to_lowercase()).collect();
        out.extend(
            self.profiles
                .iter()
                .filter(|p| !builtins.contains(&p.name.to_lowercase()))
                .cloned()
                .map(|mut p| {
                    p.builtin = false;
                    p
                }),
        );
        out
    }

    pub fn find(&self, name: &str) -> Option<Profile> {
        self.all().into_iter().find(|p| p.name.eq_ignore_ascii_case(name))
    }

    pub fn active_profile(&self) -> Profile {
        self.find(&self.active).unwrap_or_else(Profile::modern)
    }

    pub fn set_active(&mut self, name: &str) -> bool {
        if self.find(name).is_some() {
            self.active = name.to_string();
            true
        } else {
            false
        }
    }

    /// Make `name` the active profile; a built-in's name is accepted.
    pub fn duplicate(&mut self, name: &str) -> Option<String> {
        let source = self.find(name)?;
        let base = format!("{} copy", source.name);
        let name = unique_name(&base, self.all().iter().map(|p| p.name.as_str()));
        let mut copy = source.duplicate(&name);
        copy.builtin = false;
        self.profiles.push(copy);
        self.active = name.clone();
        Some(name)
    }

    pub fn upsert(&mut self, profile: Profile) -> Result<(), String> {
        profile.validate()?;
        if profile.builtin || Profile::builtins().iter().any(|b| b.name.eq_ignore_ascii_case(&profile.name)) {
            return Err("built-in profiles can't be changed: duplicate one instead".into());
        }
        match self.profiles.iter_mut().find(|p| p.name.eq_ignore_ascii_case(&profile.name)) {
            Some(p) => *p = profile,
            None => self.profiles.push(profile),
        }
        Ok(())
    }

    /// Save an edited profile under the name it had (`old` may differ when
    /// the name itself was edited).
    pub fn replace(&mut self, old: &str, profile: Profile) -> Result<(), String> {
        profile.validate()?;
        if Profile::builtins().iter().any(|b| b.name.eq_ignore_ascii_case(old))
            || profile.builtin
            || Profile::builtins().iter().any(|b| b.name.eq_ignore_ascii_case(&profile.name))
        {
            return Err("built-in profiles can't be changed: duplicate one instead".into());
        }
        if self
            .profiles
            .iter()
            .any(|p| !p.name.eq_ignore_ascii_case(old) && p.name.eq_ignore_ascii_case(&profile.name))
        {
            return Err(format!("a profile named {} already exists", profile.name));
        }
        let Some(p) = self.profiles.iter_mut().find(|p| p.name.eq_ignore_ascii_case(old)) else {
            return Err(format!("no profile named {old}"));
        };
        *p = profile;
        if self.active.eq_ignore_ascii_case(old) {
            self.active = p.name.clone();
        }
        Ok(())
    }

    pub fn rename(&mut self, old: &str, new: &str) -> Result<(), String> {
        let new = new.trim();
        if Profile::builtins().iter().any(|b| b.name.eq_ignore_ascii_case(old)) {
            return Err("built-in profiles can't be renamed: duplicate one instead".into());
        }
        if Profile::builtins().iter().any(|b| b.name.eq_ignore_ascii_case(new)) {
            return Err("that name belongs to a built-in profile".into());
        }
        if self.profiles.iter().any(|p| p.name.eq_ignore_ascii_case(new)) {
            return Err(format!("a profile named {new} already exists"));
        }
        let Some(p) = self.profiles.iter_mut().find(|p| p.name.eq_ignore_ascii_case(old)) else {
            return Err(format!("no profile named {old}"));
        };
        p.name = new.to_string();
        p.validate()?;
        if self.active.eq_ignore_ascii_case(old) {
            self.active = p.name.clone();
        }
        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> Result<(), String> {
        if Profile::builtins().iter().any(|b| b.name.eq_ignore_ascii_case(name)) {
            return Err("built-in profiles can't be deleted".into());
        }
        let before = self.profiles.len();
        self.profiles.retain(|p| !p.name.eq_ignore_ascii_case(name));
        if self.profiles.len() == before {
            return Err(format!("no profile named {name}"));
        }
        if self.active.eq_ignore_ascii_case(name) {
            self.active = Profile::modern().name;
        }
        Ok(())
    }

    pub fn import(&mut self, json: &str) -> Result<String, String> {
        let mut profile = Profile::from_json(json)?;
        if self.all().iter().any(|p| p.name.eq_ignore_ascii_case(&profile.name)) {
            profile.name = unique_name(&profile.name, self.all().iter().map(|p| p.name.as_str()));
        }
        let name = profile.name.clone();
        self.profiles.push(profile);
        self.active = name.clone();
        Ok(name)
    }

    pub fn export(&self, name: &str) -> Result<String, String> {
        self.find(name).map(|p| p.to_json()).ok_or_else(|| format!("no profile named {name}"))
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Result<ProfileBook, String> {
        let mut book: ProfileBook = serde_json::from_str(json).map_err(|e| format!("not a settings backup: {e}"))?;
        if book.version == 0 {
            book.version = 1;
        }
        book.profiles.retain(|p| p.validate().is_ok());
        for p in &mut book.profiles {
            p.builtin = false;
        }
        let mut seen: Vec<String> = Profile::builtins().iter().map(|p| p.name.to_lowercase()).collect();
        book.profiles.retain(|p| {
            let key = p.name.to_lowercase();
            if seen.contains(&key) {
                false
            } else {
                seen.push(key);
                true
            }
        });
        if book.find(&book.active).is_none() {
            book.active = Profile::modern().name;
        }
        Ok(book)
    }
}

fn unique_name<'a>(base: &str, taken: impl Iterator<Item = &'a str>) -> String {
    let taken: Vec<String> = taken.map(|s| s.to_lowercase()).collect();
    if !taken.contains(&base.to_lowercase()) {
        return base.to_string();
    }
    for n in 2.. {
        let candidate = format!("{base} {n}");
        if !taken.contains(&candidate.to_lowercase()) {
            return candidate;
        }
    }
    unreachable!()
}

fn language_matches(scope: Option<&str>, lang: Language) -> bool {
    match scope {
        None => true,
        Some(s) if s.trim().is_empty() || s.eq_ignore_ascii_case("all") => true,
        Some(s) => s.eq_ignore_ascii_case(lang.label()),
    }
}

fn lang_name(s: &str) -> Option<Language> {
    Language::ALL.into_iter().find(|l| l.label().eq_ignore_ascii_case(s.trim()))
}

/// std's clock panics in browsers: ask JavaScript's.
#[cfg(target_arch = "wasm32")]
fn current_year() -> u32 {
    js_sys::Date::new_0().get_full_year()
}

#[cfg(not(target_arch = "wasm32"))]
fn current_year() -> u32 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // days since 1970-01-01 to a Gregorian year (Howard Hinnant's civil_from_days)
    let z = (secs / 86_400) as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    year.clamp(1970, 9999) as u32
}

// --- output with a map back to the source text -----------------------------

/// One stretch of the transformed text and where it came from.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Chunk {
    out: Range<usize>,
    src: Range<usize>,
    changed: bool,
}

/// A profile's work on one text: what to say, where it came from, and anything
/// that could not be applied (an invalid regular expression).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Applied {
    pub text: String,
    pub warnings: Vec<String>,
    chunks: Vec<Chunk>,
}

impl Applied {
    /// The stretch of the original text a stretch of [`Self::text`] came from.
    pub fn source_range(&self, out: Range<usize>) -> Range<usize> {
        let end = out.end.min(self.text.len());
        let start = out.start.min(end);
        let out = start..end;
        let mut from = None;
        let mut to = None;
        for c in &self.chunks {
            if c.out.end <= out.start || c.out.start >= out.end {
                continue;
            }
            let a = out.start.max(c.out.start) - c.out.start;
            let b = out.end.min(c.out.end) - c.out.start;
            let map = |p: usize| {
                if c.out.len() == 0 {
                    c.src.start
                } else {
                    (c.src.start + p * c.src.len() / c.out.len()).min(c.src.end)
                }
            };
            let s = map(a);
            let e = map(b);
            from = Some(from.map_or(s, |v: usize| v.min(s)));
            to = Some(to.map_or(e, |v: usize| v.max(e)));
        }
        from.unwrap_or(0)..to.unwrap_or(0)
    }

    /// The parts the profile changed, as byte ranges of [`Self::text`].
    pub fn highlights(&self) -> Vec<Range<usize>> {
        let mut out: Vec<Range<usize>> = Vec::new();
        for c in self.chunks.iter().filter(|c| c.changed && c.out.start < c.out.end) {
            match out.last_mut() {
                Some(last) if last.end >= c.out.start => last.end = last.end.max(c.out.end),
                _ => out.push(c.out.clone()),
            }
        }
        out
    }

    /// The words of `tokens` in the original text.
    pub fn words(&self, tokens: &[crate::g2p::Token]) -> Vec<Range<usize>> {
        crate::text::word_spans(&self.text, tokens).into_iter().map(|r| self.source_range(r)).collect()
    }
}

#[derive(Clone, Debug)]
struct Mapped {
    text: String,
    chunks: Vec<Chunk>,
}

impl Mapped {
    fn new(text: &str) -> Mapped {
        Mapped {
            text: text.to_string(),
            chunks: if text.is_empty() {
                Vec::new()
            } else {
                vec![Chunk {
                    out: 0..text.len(),
                    src: 0..text.len(),
                    changed: false,
                }]
            },
        }
    }

    fn prepend(&mut self, s: &str) {
        let mut text = String::with_capacity(s.len() + self.text.len());
        text.push_str(s);
        text.push_str(&self.text);
        let add = s.len();
        for c in &mut self.chunks {
            c.out.start += add;
            c.out.end += add;
        }
        self.text = text;
        self.chunks.insert(
            0,
            Chunk {
                out: 0..add,
                src: 0..0,
                changed: true,
            },
        );
    }

    /// One space between words, none at either end.
    fn tidy(self) -> Mapped {
        let ws = Regex::new(r"\s+").unwrap();
        let m = self.rewrite(&ws, " ", false);
        let lead = Regex::new(r"^\s+").unwrap();
        let m = m.rewrite(&lead, "", false);
        let trail = Regex::new(r"\s+$").unwrap();
        m.rewrite(&trail, "", false)
    }

    fn append(&mut self, s: &str) {
        let at = self.text.len();
        self.text.push_str(s);
        self.chunks.push(Chunk {
            out: at..at + s.len(),
            src: at..at,
            changed: true,
        });
    }

    fn rewrite(&self, re: &Regex, replacement: &str, expand: bool) -> Mapped {
        let matches: Vec<(Range<usize>, String)> = re
            .captures_iter(&self.text)
            .filter_map(|caps| {
                let m = caps.get(0)?;
                let mut text = String::new();
                if expand {
                    caps.expand(replacement, &mut text);
                } else {
                    text.push_str(replacement);
                }
                Some((m.range(), text))
            })
            .collect();
        self.apply_matches(matches)
    }

    /// Where a byte range of this text came from in the original one.
    fn source_range(&self, out: Range<usize>) -> Range<usize> {
        let mut from = None;
        let mut to: Option<usize> = None;
        for c in &self.chunks {
            if c.out.end <= out.start || c.out.start >= out.end {
                continue;
            }
            let a = out.start.max(c.out.start) - c.out.start;
            let b = out.end.min(c.out.end) - c.out.start;
            let map = |p: usize| {
                if c.out.len() == 0 {
                    c.src.start
                } else {
                    (c.src.start + p * c.src.len() / c.out.len()).min(c.src.end)
                }
            };
            let s = map(a);
            let e = map(b);
            from = Some(from.map_or(s, |v: usize| v.min(s)));
            to = Some(to.map_or(e, |v: usize| v.max(e)));
        }
        from.unwrap_or(0)..to.unwrap_or(0)
    }

    fn rewrite_matches<F>(&self, re: &Regex, mut f: F) -> Mapped
    where
        F: FnMut(&Captures) -> Option<String>,
    {
        let matches: Vec<(Range<usize>, String)> = re
            .captures_iter(&self.text)
            .filter_map(|caps| Some((caps.get(0)?.range(), f(&caps)?)))
            .collect();
        self.apply_matches(matches)
    }

    /// Replace non-overlapping matches, keeping unchanged text and its map.
    fn apply_matches(&self, matches: Vec<(Range<usize>, String)>) -> Mapped {
        if matches.is_empty() {
            return self.clone();
        }
        let mut text = String::with_capacity(self.text.len());
        let mut chunks = Vec::new();
        let mut at = 0;
        for (range, replacement) in matches {
            if range.start < at || range.end > self.text.len() || range.start > range.end {
                continue;
            }
            self.copy_into(&mut text, &mut chunks, at..range.start, false);
            let out_start = text.len();
            text.push_str(&replacement);
            let src = self.source_range(range.clone());
            chunks.push(Chunk {
                out: out_start..text.len(),
                src,
                changed: true,
            });
            at = range.end;
        }
        self.copy_into(&mut text, &mut chunks, at..self.text.len(), false);
        Mapped { text, chunks }
    }

    fn copy_into(&self, text: &mut String, chunks: &mut Vec<Chunk>, range: Range<usize>, force_changed: bool) {
        if range.start > range.end || range.end > self.text.len() {
            return;
        }
        let out_start = text.len();
        text.push_str(&self.text[range.clone()]);
        for c in &self.chunks {
            let a = range.start.max(c.out.start);
            let b = range.end.min(c.out.end);
            if a < b {
                let shift = out_start + (a - range.start);
                chunks.push(Chunk {
                    out: shift..shift + (b - a),
                    src: c.src.clone(),
                    changed: c.changed || force_changed,
                });
            }
        }
        if range.start == range.end {
            chunks.push(Chunk {
                out: out_start..out_start,
                src: {
                    let at = self.source_range(range).start;
                    at..at
                },
                changed: force_changed,
            });
        }
    }
}

// --- localized words -------------------------------------------------------

const IT_MONTHS: [&str; 12] = [
    "gennaio",
    "febbraio",
    "marzo",
    "aprile",
    "maggio",
    "giugno",
    "luglio",
    "agosto",
    "settembre",
    "ottobre",
    "novembre",
    "dicembre",
];
const EN_MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const DE_MONTHS: [&str; 12] = [
    "Januar",
    "Februar",
    "März",
    "April",
    "Mai",
    "Juni",
    "Juli",
    "August",
    "September",
    "Oktober",
    "November",
    "Dezember",
];
const FR_MONTHS: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];
const ES_MONTHS: [&str; 12] = [
    "enero",
    "febrero",
    "marzo",
    "abril",
    "mayo",
    "junio",
    "julio",
    "agosto",
    "septiembre",
    "octubre",
    "noviembre",
    "diciembre",
];
const PT_MONTHS: [&str; 12] = [
    "janeiro",
    "fevereiro",
    "março",
    "abril",
    "maio",
    "junho",
    "julho",
    "agosto",
    "setembro",
    "outubro",
    "novembro",
    "dezembro",
];
const CA_MONTHS: [&str; 12] = [
    "gener", "febrer", "març", "abril", "maig", "juny", "juliol", "agost", "setembre", "octubre", "novembre", "desembre",
];
const NL_MONTHS: [&str; 12] = [
    "januari",
    "februari",
    "maart",
    "april",
    "mei",
    "juni",
    "juli",
    "augustus",
    "september",
    "oktober",
    "november",
    "december",
];
const SV_MONTHS: [&str; 12] = [
    "januari",
    "februari",
    "mars",
    "april",
    "maj",
    "juni",
    "juli",
    "augusti",
    "september",
    "oktober",
    "november",
    "december",
];
const DA_MONTHS: [&str; 12] = [
    "januar",
    "februar",
    "marts",
    "april",
    "maj",
    "juni",
    "juli",
    "august",
    "september",
    "oktober",
    "november",
    "december",
];
const NB_MONTHS: [&str; 12] = [
    "januar",
    "februar",
    "mars",
    "april",
    "mai",
    "juni",
    "juli",
    "august",
    "september",
    "oktober",
    "november",
    "desember",
];
const FI_MONTHS: [&str; 12] = [
    "tammikuuta",
    "helmikuuta",
    "maaliskuuta",
    "huhtikuuta",
    "toukokuuta",
    "kesäkuuta",
    "heinäkuuta",
    "elokuuta",
    "syyskuuta",
    "lokakuuta",
    "marraskuuta",
    "joulukuuta",
];
const PL_MONTHS: [&str; 12] = [
    "stycznia",
    "lutego",
    "marca",
    "kwietnia",
    "maja",
    "czerwca",
    "lipca",
    "sierpnia",
    "września",
    "października",
    "listopada",
    "grudnia",
];
const RU_MONTHS: [&str; 12] = [
    "января",
    "февраля",
    "марта",
    "апреля",
    "мая",
    "июня",
    "июля",
    "августа",
    "сентября",
    "октября",
    "ноября",
    "декабря",
];
const EL_MONTHS: [&str; 12] = [
    "Ιανουαρίου",
    "Φεβρουαρίου",
    "Μαρτίου",
    "Απριλίου",
    "Μαΐου",
    "Ιουνίου",
    "Ιουλίου",
    "Αυγούστου",
    "Σεπτεμβρίου",
    "Οκτωβρίου",
    "Νοεμβρίου",
    "Δεκεμβρίου",
];
const TR_MONTHS: [&str; 12] = [
    "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim", "Kasım", "Aralık",
];

struct Words {
    lang: Language,
}

impl Words {
    fn for_language(lang: Language) -> Words {
        Words { lang }
    }

    fn it(&self) -> bool {
        self.lang == Language::Italian
    }

    fn pick<'a>(&self, it: &'a str, en: &'a str) -> &'a str {
        if self.it() {
            it
        } else {
            en
        }
    }

    fn month(&self, m: usize) -> &'static str {
        let m = m.saturating_sub(1).min(11);
        match self.lang {
            Language::Italian => IT_MONTHS[m],
            Language::English => EN_MONTHS[m],
            Language::German => DE_MONTHS[m],
            Language::French => FR_MONTHS[m],
            Language::Spanish => ES_MONTHS[m],
            Language::Portuguese => PT_MONTHS[m],
            Language::Catalan | Language::Valencian | Language::Galician => CA_MONTHS[m],
            Language::Dutch => NL_MONTHS[m],
            Language::Swedish => SV_MONTHS[m],
            Language::Danish => DA_MONTHS[m],
            Language::Norwegian => NB_MONTHS[m],
            Language::Finnish => FI_MONTHS[m],
            Language::Polish => PL_MONTHS[m],
            Language::Russian => RU_MONTHS[m],
            Language::Greek => EL_MONTHS[m],
            Language::Turkish => TR_MONTHS[m],
            _ => EN_MONTHS[m],
        }
    }

    fn range_from(&self) -> &'static str {
        match self.lang {
            Language::Italian => "da",
            Language::German => "von",
            Language::French => "de",
            Language::Spanish | Language::Portuguese | Language::Catalan | Language::Valencian | Language::Galician => "de",
            Language::Dutch => "van",
            Language::Swedish | Language::Danish | Language::Norwegian => "från",
            Language::Polish => "od",
            Language::Russian => "от",
            Language::Greek => "από",
            Language::Turkish => "den",
            _ => "from",
        }
    }

    fn range_to(&self) -> &'static str {
        match self.lang {
            Language::Italian | Language::Spanish | Language::Portuguese | Language::Catalan | Language::Valencian | Language::Galician => "a",
            Language::German => "bis",
            Language::French => "à",
            Language::Dutch => "tot",
            Language::Swedish | Language::Danish | Language::Norwegian => "till",
            Language::Polish => "do",
            Language::Russian => "до",
            Language::Greek => "έως",
            Language::Turkish => "e",
            _ => "to",
        }
    }

    fn at(&self) -> &'static str {
        self.pick("chiocciola", "at")
    }

    fn on(&self) -> &'static str {
        self.pick("su", "on")
    }

    fn dot(&self) -> &'static str {
        match self.lang {
            Language::Italian | Language::Spanish | Language::Portuguese | Language::Catalan | Language::Valencian | Language::Galician => {
                if self.lang == Language::Portuguese {
                    "ponto"
                } else {
                    "punto"
                }
            }
            Language::German => "Punkt",
            Language::French => "point",
            Language::Dutch => "punt",
            Language::Swedish | Language::Danish | Language::Norwegian => "punkt",
            Language::Finnish => "piste",
            Language::Polish => "kropka",
            Language::Russian => "точка",
            Language::Greek => "τελεία",
            Language::Turkish => "nokta",
            _ => "dot",
        }
    }

    fn dash(&self) -> &'static str {
        self.pick("trattino", "dash")
    }

    fn underscore(&self) -> &'static str {
        "underscore"
    }

    fn slash(&self) -> &'static str {
        self.pick("barra", "slash")
    }

    fn question(&self) -> &'static str {
        self.pick("punto interrogativo", "question mark")
    }

    fn equals(&self) -> &'static str {
        self.pick("uguale", "equals")
    }

    fn ampersand(&self) -> &'static str {
        self.pick("e", "and")
    }

    fn hash(&self) -> &'static str {
        self.pick("cancelletto", "hash")
    }

    fn plus(&self) -> &'static str {
        self.pick("più", "plus")
    }

    fn minus(&self) -> &'static str {
        self.pick("meno", "minus")
    }

    fn times(&self) -> &'static str {
        self.pick("per", "times")
    }

    fn divide(&self) -> &'static str {
        self.pick("diviso", "divided by")
    }

    fn power(&self) -> &'static str {
        self.pick("elevato a", "to the power of")
    }

    fn percent(&self) -> &'static str {
        self.pick("per cento", "percent")
    }

    fn unit(&self, it: &'static str, en: &'static str) -> &'static str {
        self.pick(it, en)
    }

    fn currency(&self, unit: Currency, singular: bool) -> &'static str {
        match (unit, self.it(), singular) {
            (Currency::Euro, true, true) => "euro",
            (Currency::Euro, true, false) => "euro",
            (Currency::Dollar, true, true) => "dollaro",
            (Currency::Dollar, true, false) => "dollari",
            (Currency::Pound, true, true) => "sterlina",
            (Currency::Pound, true, false) => "sterline",
            (Currency::Yen, true, true) => "yen",
            (Currency::Yen, true, false) => "yen",
            (Currency::Franc, true, true) => "franco",
            (Currency::Franc, true, false) => "franchi",
            (Currency::Euro, false, _) => "euro",
            (Currency::Dollar, false, true) => "dollar",
            (Currency::Dollar, false, false) => "dollars",
            (Currency::Pound, false, true) => "pound",
            (Currency::Pound, false, false) => "pounds",
            (Currency::Yen, false, _) => "yen",
            (Currency::Franc, false, true) => "franc",
            (Currency::Franc, false, false) => "francs",
        }
    }

    fn crypto(&self, code: &str) -> &'static str {
        match code.to_ascii_uppercase().as_str() {
            "BTC" => "bitcoin",
            "ETH" => "ether",
            "USDT" => "tether",
            "SOL" => "solana",
            "XRP" => "ripple",
            "ADA" => "cardano",
            "DOT" => "polkadot",
            "DOGE" => "dogecoin",
            "BNB" => "BNB",
            _ => "crypto",
        }
    }

    fn emoji(&self, key: &str) -> &'static str {
        let it = self.it();
        match key {
            "🚆" => {
                if it {
                    "treno"
                } else {
                    "train"
                }
            }
            "🚄" => {
                if it {
                    "treno veloce"
                } else {
                    "high-speed train"
                }
            }
            "🚉" => {
                if it {
                    "stazione"
                } else {
                    "station"
                }
            }
            "⚠" => {
                if it {
                    "attenzione"
                } else {
                    "warning"
                }
            }
            "✅" => {
                if it {
                    "fatto"
                } else {
                    "check mark"
                }
            }
            "❌" => {
                if it {
                    "no"
                } else {
                    "cross mark"
                }
            }
            "❤" => {
                if it {
                    "cuore"
                } else {
                    "heart"
                }
            }
            "😀" => {
                if it {
                    "faccina sorridente"
                } else {
                    "smiling face"
                }
            }
            "😂" => {
                if it {
                    "faccina che ride"
                } else {
                    "face with tears of joy"
                }
            }
            "👍" => {
                if it {
                    "pollice in su"
                } else {
                    "thumbs up"
                }
            }
            "👎" => {
                if it {
                    "pollice in giù"
                } else {
                    "thumbs down"
                }
            }
            "🎉" => {
                if it {
                    "festa"
                } else {
                    "party"
                }
            }
            "🔥" => {
                if it {
                    "fuoco"
                } else {
                    "fire"
                }
            }
            "⭐" => {
                if it {
                    "stella"
                } else {
                    "star"
                }
            }
            "🇮🇹" => {
                if it {
                    "bandiera italiana"
                } else {
                    "Italian flag"
                }
            }
            "🇬🇧" => {
                if it {
                    "bandiera britannica"
                } else {
                    "British flag"
                }
            }
            "🇺🇸" => {
                if it {
                    "bandiera americana"
                } else {
                    "American flag"
                }
            }
            _ => "",
        }
    }
}

fn comma_decimal(lang: Language) -> bool {
    !matches!(lang, Language::English | Language::Chinese | Language::Generic)
}

fn localize_number(s: &str, lang: Language) -> String {
    if comma_decimal(lang) {
        s.replace('.', ",")
    } else {
        s.to_string()
    }
}

fn clean_spaces(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for w in s.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(w);
    }
    out
}

// --- transforms ------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Currency {
    Euro,
    Dollar,
    Pound,
    Yen,
    Franc,
}

fn currency_of(s: &str) -> Option<Currency> {
    match s {
        "€" | "EUR" => Some(Currency::Euro),
        "$" | "USD" => Some(Currency::Dollar),
        "£" | "GBP" => Some(Currency::Pound),
        "¥" | "JPY" => Some(Currency::Yen),
        "CHF" => Some(Currency::Franc),
        _ => None,
    }
}

fn singular(amount: &str) -> bool {
    amount.replace(',', ".").parse::<f64>().is_ok_and(|n| (n - 1.0).abs() < f64::EPSILON)
}

fn date_spans(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let iso = Regex::new(r"\b([0-9]{4})-([0-9]{1,2})-([0-9]{1,2})\b").unwrap();
    for c in iso.captures_iter(text) {
        let y = c[1].parse::<i32>().unwrap_or(0);
        let m = c[2].parse::<i32>().unwrap_or(0);
        let d = c[3].parse::<i32>().unwrap_or(0);
        if (1..=12).contains(&m) && (1..=31).contains(&d) && (1900..=2999).contains(&y) {
            out.push(c.get(0).unwrap().range());
        }
    }
    let dmy = Regex::new(r"\b([0-9]{1,2})([./-])([0-9]{1,2})([./-])([0-9]{2,4})\b").unwrap();
    for c in dmy.captures_iter(text) {
        if c[2] != c[4] {
            continue;
        }
        let d = c[1].parse::<i32>().unwrap_or(0);
        let m = c[3].parse::<i32>().unwrap_or(0);
        let y = c[5].parse::<i32>().unwrap_or(0);
        if (1..=31).contains(&d) && (1..=12).contains(&m) && (c[5].len() == 2 || (1900..=2999).contains(&y)) {
            out.push(c.get(0).unwrap().range());
        }
    }
    out
}

fn overlaps(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start < b.end && b.start < a.end
}

fn dates(m: Mapped, style: Dates, year: Year, lang: Language, now: u32, words: &Words) -> Mapped {
    let iso = Regex::new(r"\b([0-9]{4})-([0-9]{1,2})-([0-9]{1,2})\b").unwrap();
    let m = m.rewrite_matches(&iso, |c| {
        let y = c[1].parse::<u32>().ok()?;
        let month = c[2].parse::<u32>().ok()?;
        let day = c[3].parse::<u32>().ok()?;
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) || !(1900..=2999).contains(&y) {
            return None;
        }
        Some(say_date(day, month, &y.to_string(), style, year, lang, now, words))
    });
    let dmy = Regex::new(r"\b([0-9]{1,2})([./-])([0-9]{1,2})([./-])([0-9]{2,4})\b").unwrap();
    m.rewrite_matches(&dmy, |c| {
        if c[2] != c[4] {
            return None;
        }
        let day = c[1].parse::<u32>().ok()?;
        let month = c[3].parse::<u32>().ok()?;
        let raw_year = &c[5];
        if raw_year.len() != 2 && raw_year.len() != 4 {
            return None;
        }
        let _y = raw_year.parse::<u32>().ok()?;
        if !(1..=31).contains(&day) || !(1..=12).contains(&month) {
            return None;
        }
        Some(say_date(day, month, raw_year, style, year, lang, now, words))
    })
}

fn say_date(day: u32, month: u32, raw_year: &str, style: Dates, year: Year, _lang: Language, now: u32, words: &Words) -> String {
    let full_year = expand_year(raw_year, year, now);
    let spoken_year = if raw_year.len() == 2 {
        match style {
            Dates::Verbatim => raw_year.to_string(),
            _ => full_year.to_string(),
        }
    } else {
        full_year.to_string()
    };
    match style {
        Dates::Full => format!("{} {} {}", day, words.month(month as usize), full_year),
        Dates::Numeric => format!("{} {} {}", day, month, full_year),
        Dates::Verbatim => format!("{} {} {}", day, month, spoken_year),
        Dates::AsWritten => String::new(),
    }
}

fn expand_year(raw: &str, style: Year, now: u32) -> u32 {
    if raw.len() == 4 {
        return raw.parse().unwrap_or(0);
    }
    let y = raw.parse::<u32>().unwrap_or(0);
    match style {
        Year::NextTen => {
            let candidate = 2000 + y;
            if candidate <= now.saturating_add(10) {
                candidate
            } else {
                1900 + y
            }
        }
        Year::Original => {
            if y < 20 {
                2000 + y
            } else {
                1900 + y
            }
        }
    }
}

fn ranges(m: Mapped, style: Ranges, _lang: Language, words: &Words) -> Mapped {
    let re = Regex::new(r"\b([0-9]{1,6})\s*[-–—]\s*([0-9]{1,6})\b").unwrap();
    let protected = date_spans(&m.text);
    let text = m.text.clone();
    m.rewrite_matches(&re, |c| {
        let r = c.get(0)?.range();
        if before_or_after(&text, &r) {
            return None;
        }
        if protected.iter().any(|d| overlaps(&r, d)) {
            return None;
        }
        let (a, b) = (&c[1], &c[2]);
        Some(match style {
            Ranges::Words => format!("{} {} {} {}", words.range_from(), a, words.range_to(), b),
            Ranges::Numbers => format!("{a} {b}"),
            Ranges::AsWritten => return None,
        })
    })
}

/// A range/formula must not touch a sign, a dash, a letter or an underscore:
/// `-10-15`, `1-2-3`, `A10-15` are not ranges.
fn before_or_after(text: &str, r: &Range<usize>) -> bool {
    let bad = |c: char| c.is_alphanumeric() || matches!(c, '_' | '-' | '+' | '*' | '/' | '^' | '=');
    text[..r.start].chars().next_back().is_some_and(bad) || text[r.end..].chars().next().is_some_and(bad)
}

fn math(m: Mapped, lang: Language, words: &Words) -> Mapped {
    let re = Regex::new(r"\b[0-9]+(?:[.,][0-9]+)?(?:\s*[+\-*/^=]\s*[0-9]+(?:[.,][0-9]+)?)+\b").unwrap();
    let protected = date_spans(&m.text);
    let text = m.text.clone();
    m.rewrite_matches(&re, |c| {
        let r = c.get(0)?.range();
        if before_or_after(&text, &r) || protected.iter().any(|d| overlaps(&r, d)) {
            return None;
        }
        let s = c.get(0)?.as_str();
        // a bare subtraction is a range, not a formula
        if !s.chars().any(|ch| matches!(ch, '+' | '*' | '/' | '^' | '=')) {
            return None;
        }
        let mut out = String::with_capacity(s.len() + 16);
        let mut last_space = false;
        for ch in s.chars() {
            let op = match ch {
                '+' => Some(words.plus()),
                '-' => Some(words.minus()),
                '*' => Some(words.times()),
                '/' => Some(words.divide()),
                '^' => Some(words.power()),
                '=' => Some(words.equals()),
                _ => None,
            };
            match op {
                Some(w) => {
                    if !out.ends_with(' ') {
                        out.push(' ');
                    }
                    out.push_str(w);
                    out.push(' ');
                    last_space = true;
                }
                None => {
                    if ch == '.' || ch == ',' {
                        out.push(if comma_decimal(lang) { ',' } else { '.' });
                        last_space = false;
                    } else if ch.is_whitespace() {
                        if !last_space {
                            out.push(' ');
                        }
                        last_space = true;
                    } else {
                        out.push(ch);
                        last_space = false;
                    }
                }
            }
        }
        Some(clean_spaces(&out))
    })
}

fn links(m: Mapped, _lang: Language, words: &Words) -> Mapped {
    let re = Regex::new(r"(?i)\b(?:https?://|www\.)[^\s]+|\b[A-Za-z0-9][A-Za-z0-9.-]*\.[A-Za-z]{2,}(?:/[^\s]*)?").unwrap();
    m.rewrite_matches(&re, |c| {
        let s = c.get(0)?.as_str();
        let mut core = s;
        let mut tail = String::new();
        while let Some(ch) = core.chars().next_back() {
            if matches!(ch, '.' | ',' | ';' | ':' | '!' | '?') {
                tail.insert(0, ch);
                core = &core[..core.len() - ch.len_utf8()];
            } else {
                break;
            }
        }
        if core.is_empty() {
            return None;
        }
        Some(format!("{}{}", say_link(core, words), tail))
    })
}

fn say_link(s: &str, words: &Words) -> String {
    let s = s.strip_prefix("https://").or_else(|| s.strip_prefix("http://")).unwrap_or(s);
    let mut out = String::with_capacity(s.len() * 2);
    for ch in s.chars() {
        let word = match ch {
            '.' => Some(words.dot()),
            '/' => Some(words.slash()),
            ':' => Some(words.pick("due punti", "colon")),
            '?' => Some(words.question()),
            '=' => Some(words.equals()),
            '&' => Some(words.ampersand()),
            '#' => Some(words.hash()),
            '_' => Some(words.underscore()),
            '-' => Some(words.dash()),
            '%' => Some(words.percent()),
            _ => None,
        };
        match word {
            Some(w) => {
                if !out.ends_with(' ') {
                    out.push(' ');
                }
                out.push_str(w);
                out.push(' ');
            }
            None => out.push(ch),
        }
    }
    clean_spaces(&out)
}

fn emails(m: Mapped, _lang: Language, words: &Words) -> Mapped {
    let re = Regex::new(r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}").unwrap();
    let text = m.text.clone();
    m.rewrite_matches(&re, |c| {
        let r = c.get(0)?.range();
        if text[..r.start]
            .chars()
            .next_back()
            .is_some_and(|ch| ch.is_alphanumeric() || matches!(ch, '@' | '.' | '_' | '%' | '+' | '-'))
        {
            return None;
        }
        let (local, domain) = c.get(0)?.as_str().split_once('@')?;
        Some(clean_spaces(&format!(
            "{} {} {}",
            say_mail_part(local, words),
            words.at(),
            say_domain(domain, words)
        )))
    })
}

fn say_mail_part(s: &str, words: &Words) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for ch in s.chars() {
        let word = match ch {
            '.' => Some(words.dot()),
            '_' => Some(words.underscore()),
            '+' => Some(words.plus()),
            '-' => Some(words.dash()),
            _ => None,
        };
        match word {
            Some(w) => {
                if !out.ends_with(' ') {
                    out.push(' ');
                }
                out.push_str(w);
                out.push(' ');
            }
            None => out.push(ch),
        }
    }
    clean_spaces(&out)
}

fn say_domain(s: &str, words: &Words) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for ch in s.chars() {
        let word = match ch {
            '.' => Some(words.dot()),
            '-' => Some(words.dash()),
            '_' => Some(words.underscore()),
            ':' => Some(words.pick("due punti", "colon")),
            _ => None,
        };
        match word {
            Some(w) => {
                if !out.ends_with(' ') {
                    out.push(' ');
                }
                out.push_str(w);
                out.push(' ');
            }
            None => out.push(ch),
        }
    }
    clean_spaces(&out)
}

fn federated(m: Mapped, _lang: Language, words: &Words) -> Mapped {
    let re = Regex::new(r"@([A-Za-z0-9_.\-]+)@([A-Za-z0-9.\-]+\.[A-Za-z]{2,})").unwrap();
    let text = m.text.clone();
    let m = m.rewrite_matches(&re, |c| {
        let r = c.get(0)?.range();
        if text[..r.start]
            .chars()
            .next_back()
            .is_some_and(|ch| ch.is_alphanumeric() || matches!(ch, '@' | '.'))
        {
            return None;
        }
        Some(clean_spaces(&format!("{} {} {}", &c[1], words.on(), say_domain(&c[2], words))))
    });
    let re = Regex::new(r"@([A-Za-z0-9_.\-]+):([A-Za-z0-9.\-]+\.[A-Za-z]{2,})").unwrap();
    let text = m.text.clone();
    m.rewrite_matches(&re, |c| {
        let r = c.get(0)?.range();
        if text[..r.start]
            .chars()
            .next_back()
            .is_some_and(|ch| ch.is_alphanumeric() || matches!(ch, '@' | '.'))
        {
            return None;
        }
        Some(clean_spaces(&format!("{} {} {}", &c[1], words.on(), say_domain(&c[2], words))))
    })
}

fn handles(m: Mapped, _lang: Language, words: &Words) -> Mapped {
    let re = Regex::new(r"@[A-Za-z0-9_]{1,30}").unwrap();
    let text = m.text.clone();
    let m = m.rewrite_matches(&re, |c| {
        let r = c.get(0)?.range();
        if text[..r.start]
            .chars()
            .next_back()
            .is_some_and(|ch| ch.is_alphanumeric() || matches!(ch, '@' | '.'))
        {
            return None;
        }
        Some(format!("{} {}", words.at(), &c[0][1..]))
    });
    let re = Regex::new(r"#[A-Za-z][A-Za-z0-9_]*").unwrap();
    let text = m.text.clone();
    m.rewrite_matches(&re, |c| {
        let r = c.get(0)?.range();
        if text[..r.start].chars().next_back().is_some_and(|ch| ch.is_alphanumeric() || ch == '#') {
            return None;
        }
        Some(format!("hashtag {}", &c[0][1..]))
    })
}

fn cashtags(m: Mapped, _lang: Language) -> Mapped {
    let re = Regex::new(r"\$([A-Za-z]{1,5})\b").unwrap();
    m.rewrite_matches(&re, |c| {
        let letters: Vec<String> = c[1].chars().map(|ch| ch.to_ascii_uppercase().to_string()).collect();
        Some(format!("cashtag {}", letters.join(" ")))
    })
}

fn currencies(m: Mapped, lang: Language, words: &Words) -> Mapped {
    let before = Regex::new(r"([€$£¥])\s*([0-9]+(?:[.,][0-9]+)?)").unwrap();
    let m = m.rewrite_matches(&before, |c| {
        let cur = currency_of(&c[1])?;
        let amount = &c[2];
        Some(format!("{} {}", localize_number(amount, lang), words.currency(cur, singular(amount))))
    });
    let after = Regex::new(r"([0-9]+(?:[.,][0-9]+)?)\s*([€$£¥])").unwrap();
    let m = m.rewrite_matches(&after, |c| {
        let cur = currency_of(&c[2])?;
        let amount = &c[1];
        Some(format!("{} {}", localize_number(amount, lang), words.currency(cur, singular(amount))))
    });
    let code_before = Regex::new(r"\b(EUR|USD|GBP|JPY|CHF|CAD|AUD)\s*([0-9]+(?:[.,][0-9]+)?)").unwrap();
    let m = m.rewrite_matches(&code_before, |c| {
        let cur = currency_of(&c[1])?;
        let amount = &c[2];
        Some(format!("{} {}", localize_number(amount, lang), words.currency(cur, singular(amount))))
    });
    let code_after = Regex::new(r"([0-9]+(?:[.,][0-9]+)?)\s*(EUR|USD|GBP|JPY|CHF|CAD|AUD)\b").unwrap();
    m.rewrite_matches(&code_after, |c| {
        let cur = currency_of(&c[2])?;
        let amount = &c[1];
        Some(format!("{} {}", localize_number(amount, lang), words.currency(cur, singular(amount))))
    })
}

fn crypto(m: Mapped, lang: Language, words: &Words) -> Mapped {
    let after = Regex::new(r"([0-9]+(?:[.,][0-9]+)?)\s*\b(BTC|ETH|USDT|SOL|XRP|ADA|DOT|DOGE|BNB)\b").unwrap();
    let m = m.rewrite_matches(&after, |c| {
        let amount = &c[1];
        let code = &c[2];
        Some(format!("{} {}", localize_number(amount, lang), words.crypto(code)))
    });
    let before = Regex::new(r"\b(BTC|ETH|USDT|SOL|XRP|ADA|DOT|DOGE|BNB)\s*([0-9]+(?:[.,][0-9]+)?)").unwrap();
    m.rewrite_matches(&before, |c| {
        let code = &c[1];
        let amount = &c[2];
        Some(format!("{} {}", localize_number(amount, lang), words.crypto(code)))
    })
}

fn units(m: Mapped, _lang: Language, words: &Words) -> Mapped {
    const UNITS: &[(&str, &str, &str)] = &[
        ("km/h", "chilometri orari", "kilometres per hour"),
        ("m/s", "metri al secondo", "metres per second"),
        ("kWh", "chilowattora", "kilowatt-hours"),
        ("GWh", "gigawattora", "gigawatt-hours"),
        ("MWh", "megawattora", "megawatt-hours"),
        ("Wh", "wattora", "watt-hours"),
        ("kW", "chilowatt", "kilowatts"),
        ("MW", "megawatt", "megawatts"),
        ("GW", "gigawatt", "gigawatts"),
        ("GHz", "gigahertz", "gigahertz"),
        ("MHz", "megahertz", "megahertz"),
        ("kHz", "chilohertz", "kilohertz"),
        ("Hz", "hertz", "hertz"),
        ("GB", "gigabyte", "gigabytes"),
        ("MB", "megabyte", "megabytes"),
        ("TB", "terabyte", "terabytes"),
        ("KB", "kilobyte", "kilobytes"),
        ("km²", "chilometri quadrati", "square kilometres"),
        ("m²", "metri quadrati", "square metres"),
        ("km", "chilometri", "kilometres"),
        ("cm", "centimetri", "centimetres"),
        ("mm", "millimetri", "millimetres"),
        ("kg", "chilogrammi", "kilograms"),
        ("mg", "milligrammi", "milligrams"),
        ("ml", "millilitri", "millilitres"),
        ("°C", "gradi Celsius", "degrees Celsius"),
        ("°F", "gradi Fahrenheit", "degrees Fahrenheit"),
        ("%", "per cento", "percent"),
    ];
    let mut m = m;
    for (unit, it, en) in UNITS {
        let re = Regex::new(&format!(r"(?i)(^|[^A-Za-z])(?:{})([^A-Za-z]|$)", regex::escape(unit))).unwrap();
        let word = words.unit(it, en);
        m = m.rewrite_matches(&re, |c| {
            let prefix = c.get(1).map(|m| m.as_str()).unwrap_or("");
            let suffix = c.get(2).map(|m| m.as_str()).unwrap_or("");
            Some(format!("{prefix} {word} {suffix}"))
        });
    }
    m
}

fn inclusive(m: Mapped) -> Mapped {
    let re = Regex::new(r"[əɜ]").unwrap();
    let m = m.rewrite(&re, "e", false);
    let re = Regex::new(r"(\p{L}+)\*").unwrap();
    m.rewrite(&re, "${1}e", true)
}

fn is_emoji_char(c: char) -> bool {
    matches!(c as u32, 0x1F000..=0x1FAFF | 0x2600..=0x27BF | 0x2B00..=0x2BFF) || matches!(c, '\u{FE0F}' | '\u{200D}' | '\u{20E3}')
}

fn emojis(m: Mapped, _lang: Language, words: &Words, describe: bool) -> Mapped {
    let text = m.text.clone();
    let mut matches: Vec<(Range<usize>, String)> = Vec::new();
    let mut at = 0;
    while at < text.len() {
        let c = text[at..].chars().next().unwrap();
        if !is_emoji_char(c) {
            at += c.len_utf8();
            continue;
        }
        let start = at;
        if ('\u{1F1E6}'..='\u{1F1FF}').contains(&c) {
            let mut end = at + c.len_utf8();
            if let Some(n) = text[end..].chars().next() {
                if ('\u{1F1E6}'..='\u{1F1FF}').contains(&n) {
                    end += n.len_utf8();
                }
            }
            let key: String = text[start..end].to_string();
            let desc = if describe { words.emoji(&key) } else { "" };
            matches.push((start..end, if desc.is_empty() { String::new() } else { format!(" {desc} ") }));
            at = end;
            continue;
        }
        let mut end = start + c.len_utf8();
        loop {
            let Some(n) = text[end..].chars().next() else { break };
            if matches!(n, '\u{FE0F}' | '\u{20E3}') || ('\u{1F3FB}'..='\u{1F3FF}').contains(&n) {
                end += n.len_utf8();
                continue;
            }
            if n == '\u{200D}' {
                end += n.len_utf8();
                if let Some(n) = text[end..].chars().next() {
                    end += n.len_utf8();
                    continue;
                }
            }
            break;
        }
        let key = text[start..end].replace('\u{FE0F}', "");
        let desc = if describe { words.emoji(&key) } else { "" };
        matches.push((start..end, if desc.is_empty() { String::new() } else { format!(" {desc} ") }));
        at = end;
    }
    m.apply_matches(matches)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modern() -> Profile {
        Profile::modern()
    }

    fn faithful() -> Profile {
        Profile::faithful()
    }

    fn text(p: &Profile, s: &str, lang: Language) -> String {
        p.apply_in(s, lang, 2026).text
    }

    #[test]
    fn modern_date_uses_the_rolling_year() {
        let p = modern();
        assert_eq!(text(&p, "22/11/26", Language::Italian), "22 novembre 2026");
        assert_eq!(text(&p, "22/11/36", Language::Italian), "22 novembre 2036");
        assert_eq!(text(&p, "22/11/37", Language::Italian), "22 novembre 1937");
        assert_eq!(text(&p, "2026-11-22", Language::Italian), "22 novembre 2026");
        assert_eq!(text(&p, "22.11.2026", Language::English), "22 November 2026");
    }

    #[test]
    fn date_modes_and_faithful() {
        let mut p = modern();
        p.dates = Dates::Numeric;
        assert_eq!(text(&p, "22/11/26", Language::Italian), "22 11 2026");
        p.dates = Dates::Verbatim;
        assert_eq!(text(&p, "22/11/26", Language::Italian), "22 11 26");
        p.dates = Dates::AsWritten;
        assert_eq!(text(&p, "22/11/26", Language::Italian), "22/11/26");
        assert_eq!(text(&faithful(), "22/11/26", Language::Italian), "22/11/26");
    }

    #[test]
    fn original_year_cutoff() {
        let p = faithful();
        assert_eq!(expand_year("19", p.year, 2026), 2019);
        assert_eq!(expand_year("20", p.year, 2026), 1920);
        assert_eq!(expand_year("99", p.year, 2026), 1999);
    }

    #[test]
    fn ranges_do_not_eat_dates() {
        let mut p = modern();
        p.dates = Dates::AsWritten;
        assert_eq!(text(&p, "22-11-26", Language::Italian), "22-11-26");
        assert_eq!(text(&p, "10-15", Language::Italian), "da 10 a 15");
        let mut n = modern();
        n.ranges = Ranges::Numbers;
        n.dates = Dates::AsWritten;
        assert_eq!(text(&n, "10 - 15", Language::Italian), "10 15");
    }

    #[test]
    fn math_only_reads_formulas() {
        let p = modern();
        assert_eq!(text(&p, "2+2=4", Language::Italian), "2 più 2 uguale 4");
        assert_eq!(text(&p, "10-15", Language::Italian), "da 10 a 15");
        assert_eq!(text(&p, "2*3", Language::Italian), "2 per 3");
    }

    #[test]
    fn links_and_social_are_spoken() {
        let p = modern();
        assert_eq!(text(&p, "a@b.com", Language::Italian), "a chiocciola b punto com");
        assert_eq!(text(&p, "@trenitalia@mastodon.uno", Language::Italian), "trenitalia su mastodon punto uno");
        assert_eq!(text(&p, "@anna #treni", Language::Italian), "chiocciola anna hashtag treni");
        assert_eq!(text(&p, "https://example.com/x", Language::Italian), "example punto com barra x");
    }

    #[test]
    fn money_and_units() {
        let p = modern();
        assert_eq!(text(&p, "€10", Language::Italian), "10 euro");
        assert_eq!(text(&p, "$5", Language::Italian), "5 dollari");
        assert_eq!(text(&p, "$1", Language::Italian), "1 dollaro");
        assert_eq!(text(&p, "1.5 BTC", Language::Italian), "1,5 bitcoin");
        assert_eq!(text(&p, "100 km/h", Language::Italian), "100 chilometri orari");
        assert_eq!(text(&p, "20°C", Language::Italian), "20 gradi Celsius");
        assert_eq!(text(&p, "50%", Language::Italian), "50 per cento");
    }

    #[test]
    fn cashtags_before_currency() {
        let p = modern();
        assert_eq!(text(&p, "$AAPL", Language::Italian), "cashtag A A P L");
    }

    #[test]
    fn inclusive_italian() {
        let p = modern();
        assert_eq!(text(&p, "ragazzə", Language::Italian), "ragazze");
        assert_eq!(text(&p, "ragazzɜ", Language::Italian), "ragazze");
        assert_eq!(text(&p, "ragazz*", Language::Italian), "ragazze");
        let mut f = modern();
        f.inclusive = false;
        assert_eq!(text(&f, "ragazz*", Language::Italian), "ragazz*");
    }

    #[test]
    fn emoji_described_or_skipped() {
        let p = modern();
        assert_eq!(text(&p, "Treno 🚆", Language::Italian), "Treno treno");
        assert_eq!(text(&p, "🇮🇹", Language::Italian), "bandiera italiana");
        let mut f = modern();
        f.emoji = Emoji::Skip;
        assert_eq!(text(&f, "Treno 🚆", Language::Italian), "Treno");
    }

    #[test]
    fn station_warning_is_off_by_default_and_appends() {
        let p = modern();
        assert!(!text(&p, "Ciao", Language::Italian).contains(STATION_WARNING));
        let mut p = modern();
        p.station_warning = true;
        assert!(text(&p, "Ciao", Language::Italian).ends_with(STATION_WARNING));
        assert!(!text(&p, "Ciao", Language::English).contains(STATION_WARNING));
    }

    #[test]
    fn replacements_are_ordered_and_scoped() {
        let mut p = faithful();
        p.replacements = vec![
            Replacement {
                find: "FS".into(),
                replace: "Ferrovie dello Stato".into(),
                language: Some("Italian".into()),
                ..Replacement::default()
            },
            Replacement {
                find: "dello Stato".into(),
                replace: "italiane".into(),
                language: None,
                ..Replacement::default()
            },
            Replacement {
                find: r"bin\.? ?(\d+)".into(),
                replace: "binario $1".into(),
                regex: true,
                ..Replacement::default()
            },
        ];
        assert_eq!(text(&p, "FS bin. 3", Language::Italian), "Ferrovie italiane binario 3");
        assert_eq!(text(&p, "FS bin. 3", Language::English), "FS binario 3");
    }

    #[test]
    fn invalid_regex_is_a_warning_not_a_panic() {
        let mut p = faithful();
        p.replacements = vec![Replacement {
            find: "(".into(),
            replace: "x".into(),
            regex: true,
            ..Replacement::default()
        }];
        let a = p.apply_in("hello", Language::English, 2026);
        assert_eq!(a.text, "hello");
        assert_eq!(a.warnings.len(), 1);
    }

    #[test]
    fn source_map_points_back_at_the_original() {
        let p = modern();
        let a = p.apply_in("del 22/11/26, bin. 3", Language::Italian, 2026);
        assert_eq!(&a.text[..], "del 22 novembre 2026, bin. 3");
        let at = a.text.find("22 novembre 2026").unwrap();
        assert_eq!(a.source_range(at..at + "22 novembre 2026".len()), 4..12);
        let changed = a.highlights();
        let want = at..at + "22 novembre 2026".len();
        assert!(changed.iter().any(|r| r.start <= want.start && r.end >= want.end));
    }

    #[test]
    fn profile_book_builtins_are_read_only_and_round_trip() {
        let mut book = ProfileBook::default();
        assert_eq!(book.active_profile().name, "Modern");
        assert!(book.duplicate("Modern").is_some());
        assert_eq!(book.active, "Modern copy");
        let copy = book.find("Modern copy").unwrap();
        assert!(!copy.builtin);
        assert!(book.upsert(Profile::modern()).is_err());
        assert!(book.remove("Modern").is_err());
        assert!(book.remove("Modern copy").is_ok());
        let json = book.to_json();
        assert_eq!(ProfileBook::from_json(&json).unwrap(), book);
        let p = Profile::from_json(&Profile::modern().to_json()).unwrap();
        assert_eq!(p.name, "Modern 2");
    }

    #[test]
    fn editing_a_copy_keeps_it_active_and_blocks_duplicate_names() {
        let mut book = ProfileBook::default();
        let copy = book.duplicate("Modern").unwrap();
        let mut edited = book.find(&copy).unwrap().duplicate("Mine");
        edited.builtin = false;
        assert!(book.replace(&copy, edited.clone()).is_ok());
        assert_eq!(book.active, "Mine");
        assert!(book.replace("Mine", Profile::faithful()).is_err());
        let mut other = edited;
        other.name = "Modern".into();
        assert!(book.replace("Mine", other).is_err());
    }

    #[test]
    fn imports_with_missing_fields_get_defaults() {
        let p = Profile::from_json(r#"{"name":"Mine","dates":"full"}"#).unwrap();
        assert_eq!(p.dates, Dates::Full);
        assert_eq!(p.year, Year::Original);
        assert!(!p.math);
    }

    #[test]
    fn trenitalia_flag_is_roberto_only() {
        assert_eq!(crate::flag::of_voice("Roberto", "Italian"), Some(crate::flag::trenitalia()));
        assert_eq!(crate::flag::of_voice("Marcello", "Italian"), crate::flag::of("Italian"));
        assert_ne!(crate::flag::trenitalia(), crate::flag::of("Italian").unwrap());
    }
}
