//! The browser's deep links: a page of the app as a URL fragment.
//!
//! | Page | Hash |
//! |---|---|
//! | Speak, the voice's sample text | `#/speak/Roberto` |
//! | Speak, edited text | `#/speak/Roberto?text=Ciao%20a%20tutti` |
//! | Speak, no voice | `#/speak` (`?text=` allowed) |
//! | Voices | `#/voices` |
//! | Explore | `#/explore/Roberto` + `list=phones`, `phone=<sym>`, `sentence=<n>` |
//!
//! A voice is named by its catalog voice name, matched ignoring case.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    Speak { voice: Option<String>, text: Option<String> },
    Voices,
    Explore { voice: String, phones: bool, phone: Option<String>, sentence: Option<usize> },
}

impl Route {
    /// The voice the page shows.
    pub fn voice(&self) -> Option<&str> {
        match self {
            Route::Speak { voice, .. } => voice.as_deref(),
            Route::Voices => None,
            Route::Explore { voice, .. } => Some(voice),
        }
    }

    /// The same page, whatever the Speak text.
    pub fn same_page(&self, other: &Route) -> bool {
        let page = |r: &Route| match r {
            Route::Speak { voice, .. } => Route::Speak { voice: voice.clone(), text: None },
            r => r.clone(),
        };
        page(self) == page(other)
    }
}

/// The route a fragment names (the leading `#` is optional), if any.
pub fn parse(hash: &str) -> Option<Route> {
    let hash = hash.strip_prefix('#').unwrap_or(hash);
    let (path, query) = hash.split_once('?').unwrap_or((hash, ""));
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let mut text = None;
    let mut phones = false;
    let mut phone = None;
    let mut sentence = None;
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let v = decode(v);
        match k {
            "text" => text = Some(v).filter(|t| !t.is_empty()),
            "list" => phones = v == "phones",
            "phone" => phone = Some(v).filter(|p| !p.is_empty()),
            "sentence" => sentence = v.parse().ok(),
            _ => {}
        }
    }
    let voice = |s: &str| Some(decode(s)).filter(|v| !v.is_empty());
    match segs.as_slice() {
        ["speak"] => Some(Route::Speak { voice: None, text }),
        ["speak", v] => Some(Route::Speak { voice: Some(voice(v)?), text }),
        ["voices"] => Some(Route::Voices),
        ["explore", v] => Some(Route::Explore { voice: voice(v)?, phones, phone, sentence }),
        _ => None,
    }
}

/// The fragment for a route, starting with `#/`.
pub fn to_hash(route: &Route) -> String {
    let (mut s, params) = match route {
        Route::Speak { voice, text } => {
            let mut s = "#/speak".to_string();
            if let Some(v) = voice.as_deref().filter(|v| !v.is_empty()) {
                s += &format!("/{}", encode(v));
            }
            let text = text.as_deref().filter(|t| !t.is_empty()).map(|t| format!("text={}", encode(t)));
            (s, text.into_iter().collect::<Vec<_>>())
        }
        Route::Voices => ("#/voices".to_string(), Vec::new()),
        Route::Explore { voice, phones, phone, sentence } => {
            let mut params = Vec::new();
            if *phones {
                params.push("list=phones".to_string());
            }
            if let Some(p) = phone.as_deref().filter(|p| !p.is_empty()) {
                params.push(format!("phone={}", encode(p)));
            }
            if let Some(n) = sentence {
                params.push(format!("sentence={n}"));
            }
            (format!("#/explore/{}", encode(voice)), params)
        }
    };
    if !params.is_empty() {
        s += "?";
        s += &params.join("&");
    }
    s
}

/// Percent-encode everything but ASCII letters, digits and `-_.~`.
fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out += &format!("%{b:02X}");
        }
    }
    out
}

/// Undo `%XX` escapes; anything else (`+`, a bad escape) stays as it is.
fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let routes = [
            Route::Speak { voice: None, text: None },
            Route::Speak {
                voice: Some("Roberto".into()),
                text: Some("Attenzione! Allontanarsi dalla linea gialla. è \"ok\" & 50%".into()),
            },
            Route::Voices,
            Route::Explore { voice: "Roberto".into(), phones: true, phone: Some("a1".into()), sentence: Some(12) },
        ];
        for r in routes {
            assert_eq!(parse(&to_hash(&r)), Some(r.clone()), "{}", to_hash(&r));
        }
    }

    #[test]
    fn rejects_other_hashes() {
        for h in ["", "#", "#/nope", "#/explore"] {
            assert_eq!(parse(h), None, "{h}");
        }
    }

    #[test]
    fn empty_text_is_absent() {
        assert_eq!(parse("#/speak/Roberto?text="), Some(Route::Speak { voice: Some("Roberto".into()), text: None }));
    }

    #[test]
    fn hash_is_optional() {
        assert_eq!(parse("/voices"), Some(Route::Voices));
    }
}
