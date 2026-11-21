//! LTTS 7.2's configuration (`global.ttsconf`): one XML file describing every
//! voice and language, where 7.0 kept a `.vcf` and a `.vde` per voice and an
//! `.lde` per language. Read only as far as `pack` needs to write a 7.0
//! `.vcf`/`.vde` for each voice of a 7.2 bundle (Federica's installer).

/// One `<voice>` entry, its `${name}`/`${mothertongue}` expanded.
#[derive(Debug, Default, PartialEq)]
pub struct VoiceEntry {
    pub name: String,
    pub mother_tongue: String,
    pub description: String,
    pub age: String,
    pub gender: String,
    pub demo_sentence: String,
    pub base_pitch: String,
    pub base_speed: String,
    /// every `<resource type="vocaldb">`, in order: a 7.2 voice may split its
    /// main database (`Federica_1`, `Federica_2`), then its Gilded ones
    pub vocal_dbs: Vec<String>,
    /// whether a `class="gilded"` database is among them
    pub gilded: bool,
    /// the `<languagealiases>` names in order (`Italian`, `it-IT`, `it`),
    /// without the Windows language id
    pub aliases: Vec<String>,
}

/// One tag: its name (`/voice` when closing) and attributes, unescaped.
struct Tag<'a> {
    name: &'a str,
    attrs: Vec<(&'a str, String)>,
}

impl Tag<'_> {
    fn get(&self, key: &str) -> &str {
        self.attrs.iter().find(|(k, _)| *k == key).map_or("", |(_, v)| v.as_str())
    }
}

/// XML's predefined and numeric character references.
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(end) = rest.find(';') else { break };
        let entity = &rest[1..end];
        let c = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => entity
                .strip_prefix("#x")
                .map(|h| u32::from_str_radix(h, 16))
                .or_else(|| entity.strip_prefix('#').map(str::parse))
                .and_then(Result::ok)
                .and_then(char::from_u32),
        };
        match c {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Every element tag of `xml` in order; comments, declarations and text
/// are skipped. Attribute values may hold `>`: tags end outside quotes.
fn tags(xml: &str) -> Vec<Tag<'_>> {
    let mut out = Vec::new();
    let b = xml.as_bytes();
    let mut i = 0;
    while let Some(at) = xml[i..].find('<').map(|k| i + k) {
        if xml[at..].starts_with("<!--") {
            i = xml[at..].find("-->").map_or(xml.len(), |k| at + k + 3);
            continue;
        }
        let mut end = at + 1;
        let mut quote = None;
        while end < b.len() {
            match (quote, b[end]) {
                (None, b'"' | b'\'') => quote = Some(b[end]),
                (Some(q), c) if c == q => quote = None,
                (None, b'>') => break,
                _ => {}
            }
            end += 1;
        }
        let body = xml[at + 1..end.min(xml.len())].trim_end_matches('/');
        i = end + 1;
        if body.starts_with('?') || body.starts_with('!') {
            continue;
        }
        let name_end = body.find(|c: char| c.is_whitespace()).unwrap_or(body.len());
        let mut attrs = Vec::new();
        let mut rest = &body[name_end..];
        while let Some(eq) = rest.find('=') {
            let key = rest[..eq].trim();
            let after = rest[eq + 1..].trim_start();
            let Some(q) = after.chars().next().filter(|c| *c == '"' || *c == '\'') else {
                break;
            };
            let Some(close) = after[1..].find(q) else { break };
            attrs.push((key, unescape(&after[1..1 + close])));
            rest = &after[close + 2..];
        }
        out.push(Tag {
            name: &body[..name_end],
            attrs,
        });
    }
    out
}

/// Every `<voice>` of a `global.ttsconf`.
pub fn voices(xml: &str) -> Vec<VoiceEntry> {
    let mut out: Vec<VoiceEntry> = Vec::new();
    let mut current: Option<VoiceEntry> = None;
    for t in tags(xml) {
        match (t.name, current.as_mut()) {
            ("voice", None) => {
                current = Some(VoiceEntry {
                    name: t.get("name").to_string(),
                    mother_tongue: t.get("mothertongue").to_string(),
                    ..Default::default()
                })
            }
            ("/voice", Some(_)) => out.extend(current.take()),
            ("info", Some(v)) => {
                v.description = t.get("description").to_string();
                v.age = t.get("age").to_string();
                v.gender = t.get("gender").to_string();
                v.demo_sentence = t.get("demosentence").to_string();
                v.base_pitch = t.get("basepitch").to_string();
                v.base_speed = t.get("basespeed").to_string();
            }
            ("resource", Some(v)) if t.get("type") == "vocaldb" => {
                v.vocal_dbs.push(t.get("name").replace("${name}", &v.name));
                v.gilded |= t.get("class") == "gilded";
            }
            ("alias", Some(v)) if t.get("type") != "WinLangid" => {
                let alias = t.get("name").replace("${mothertongue}", &v.mother_tongue).replace("${name}", &v.name);
                if !v.aliases.contains(&alias) {
                    v.aliases.push(alias);
                }
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Federica's entry as 7.2's `global.ttsconf` writes it: a main database
    /// split in two, `${…}` names, an escaped quote in the demo sentence.
    #[test]
    fn reads_a_split_voice() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<config><voices>
  <!-- a comment with <voice name="Nobody"> inside -->
  <voice name="Federica" mothertongue="Italian">
    <library name="LTTS7${name}" entrypoint="${name}" />
    <info description="Italian female voice" age="34" gender="female" demosentence="Il mio nome &#232; Federica: &apos;italiano&apos; &gt; tutto" basepitch="170" basespeed="128" />
    <resources path="modules/${name}">
      <resource class="base" type="vocaldb" name="${name}_1"><flavour name="${parent.name}.bin" /></resource>
      <resource class="base" type="vocaldb" name="${name}_2" />
      <resource class="gilded" type="vocaldb" name="${name}GildedPhrases" />
    </resources>
    <languagealiases>
      <alias type="internal" name="${mothertongue}" />
      <alias type="IETF" name="it-IT" />
      <alias type="WinLangid" name="1040" />
      <alias type="generic" name="it" />
    </languagealiases>
  </voice>
</voices></config>"#;
        let v = voices(xml);
        assert_eq!(v.len(), 1);
        let f = &v[0];
        assert_eq!(
            (f.name.as_str(), f.mother_tongue.as_str(), f.gender.as_str(), f.base_pitch.as_str()),
            ("Federica", "Italian", "female", "170")
        );
        assert_eq!(f.demo_sentence, "Il mio nome è Federica: 'italiano' > tutto");
        assert_eq!(f.vocal_dbs, ["Federica_1", "Federica_2", "FedericaGildedPhrases"]);
        assert!(f.gilded);
        assert_eq!(f.aliases, ["Italian", "it-IT", "it"]);
    }
}
