//! A Loquendo voice installer (Inno Setup or Wise) turned into the LTTS7
//! tree its voices need, without running it: each voice's `.vcf`, its
//! `voices/<Voice>/` folder, its library, and the language folder its
//! `.lde` resolves to with that language's library.

use crate::loquendo::{latin1, parse_ini};
use crate::{inno, wise};

/// What an installer holds, laid out under an LTTS7 root
/// (`data/…`, `bin/…`).
pub struct Unpacked {
    /// each `BaseVoice` it installs
    pub voices: Vec<String>,
    pub files: Vec<(String, Vec<u8>)>,
}

const INNO_MAGIC: &[u8] = b"rDlPtS\xcd\xe6\xd7{\x0b*";
const INNO_ROOT: &str = "pf/loquendo/ltts7/";

fn is_inno(exe: &[u8]) -> bool {
    exe.windows(INNO_MAGIC.len()).any(|w| w == INNO_MAGIC)
}

fn lower(s: &str) -> String {
    s.to_ascii_lowercase()
}

/// The `.lde` among `lde_texts` (path, text) for a voice: the first whose
/// `Language` list names `mother_tongue`, else the first naming one of
/// `aliases` (comma separated) in their order. Among several naming the same
/// one, the language's `DefaultVariant`.
pub fn language_descriptor(lde_texts: &[(String, String)], mother_tongue: &str, aliases: &str) -> Option<usize> {
    let inis: Vec<_> = lde_texts.iter().map(|(_, t)| parse_ini(t)).collect();
    let names = |i: usize| -> Vec<String> {
        inis[i]
            .get("Language")
            .map(|l| l.split(',').map(|s| lower(s.trim())).collect())
            .unwrap_or_default()
    };
    let default = |i: usize| inis[i].get("DefaultVariant").is_some_and(|v| v.eq_ignore_ascii_case("yes"));
    let wanted = std::iter::once(mother_tongue).chain(aliases.split(','));
    for want in wanted.map(|w| lower(w.trim())).filter(|w| !w.is_empty()) {
        let hits: Vec<usize> = (0..inis.len()).filter(|&i| names(i).contains(&want)).collect();
        if let Some(&first) = hits.first() {
            return Some(hits.iter().copied().find(|&i| default(i)).unwrap_or(first));
        }
    }
    None
}

/// The folder part of a path (`data/languages/English`).
fn folder(path: &str) -> &str {
    path.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
}

/// Unpack a voice installer into the files its voices need.
pub fn unpack_installer(exe: &[u8]) -> Result<Unpacked, String> {
    let files: Vec<(String, Vec<u8>)> = if is_inno(exe) {
        let keep = |p: &str| {
            let p = lower(p);
            p.strip_prefix(INNO_ROOT)
                .is_some_and(|r| r.starts_with("data/") || (r.starts_with("bin/ltts7") || r == "bin/loqtts7.dll") && r.ends_with(".dll"))
        };
        inno::files(exe, keep)?
            .into_iter()
            .map(|(p, d)| (p[INNO_ROOT.len()..].to_string(), d))
            .collect()
    } else {
        place_wise(wise::files(exe)?)?
    };
    select(files)
}

/// Wise cabinets are flat: the `.vcf` goes in `data/`, libraries in `bin/`,
/// the rest in the voice's folder.
fn place_wise(members: Vec<(String, Vec<u8>)>) -> Result<Vec<(String, Vec<u8>)>, String> {
    let vcf = members
        .iter()
        .find(|(n, _)| lower(n).ends_with(".vcf"))
        .ok_or("this installer holds no Loquendo voice")?;
    let voice_path = parse_ini(&latin1(&vcf.1))
        .get("VoicePath")
        .map(|p| p.replace('\\', "/").trim_matches('/').to_string())
        .ok_or("the voice file names no folder")?;
    Ok(members
        .into_iter()
        .filter_map(|(name, data)| {
            let low = lower(&name);
            let path = if low.ends_with(".vcf") {
                format!("data/{name}")
            } else if low.starts_with("ltts7") && low.ends_with(".dll") {
                format!("bin/{name}")
            } else if low.ends_with(".pdf") || low.ends_with(".ico") {
                return None;
            } else {
                format!("data/{voice_path}/{name}")
            };
            Some((path, data))
        })
        .collect())
}

/// Keep each voice's files and its language's; drop everything else.
fn select(files: Vec<(String, Vec<u8>)>) -> Result<Unpacked, String> {
    let find = |path: &str| files.iter().position(|(p, _)| lower(p) == lower(path));
    let ldes: Vec<(String, String)> = files
        .iter()
        .filter(|(p, _)| {
            let p = lower(p);
            p.starts_with("data/languages/") && p.ends_with(".lde") && p.matches('/').count() == 3
        })
        .map(|(p, d)| (p.clone(), latin1(d)))
        .collect();
    let mut keep = vec![false; files.len()];
    let mut voices = Vec::new();
    for (i, (path, data)) in files.iter().enumerate() {
        let low = lower(path);
        if !(low.starts_with("data/") && low.ends_with(".vcf") && low.matches('/').count() == 1) {
            continue;
        }
        let vcf = parse_ini(&latin1(data));
        let (Some(name), Some(voice_path)) = (vcf.get("BaseVoice"), vcf.get("VoicePath")) else {
            continue;
        };
        let dir = format!("data/{}/", lower(voice_path.replace('\\', "/").trim_matches('/')));
        keep[i] = true;
        for (k, (p, _)) in files.iter().enumerate() {
            if lower(p).starts_with(&dir) {
                keep[k] = true;
            }
        }
        if let Some(k) = find(&format!("bin/LTTS7{name}.dll")) {
            keep[k] = true;
        }
        // the language: the folder its `.lde` is in, and that language's library
        let vde = find(&format!("{dir}{name}.vde")).map(|k| parse_ini(&latin1(&files[k].1))).unwrap_or_default();
        let tongue = vde.get("MotherTongue").map(String::as_str).unwrap_or("");
        let aliases = vde.get("MotherTongueAliases").map(String::as_str).unwrap_or("");
        if let Some(l) = language_descriptor(&ldes, tongue, aliases) {
            let lang_dir = format!("{}/", lower(folder(&ldes[l].0)));
            for (k, (p, _)) in files.iter().enumerate() {
                let p = lower(p);
                if p.starts_with(&lang_dir) && !p[lang_dir.len()..].contains('/') {
                    keep[k] = true;
                }
            }
            let library = parse_ini(&ldes[l].1).get("Library").and_then(|l| l.split(',').next().map(|s| s.trim().to_string()));
            if let Some(k) = library.and_then(|lib| find(&format!("bin/{lib}.dll"))) {
                keep[k] = true;
            }
            // the text filters read their character tables from LTTS7Util.dll,
            // the generic number reader its country codes from LoqTTS7.dll
            for engine in ["bin/LTTS7Util.dll", "bin/LoqTTS7.dll"] {
                if let Some(k) = find(engine) {
                    keep[k] = true;
                }
            }
        }
        voices.push(name.clone());
    }
    if voices.is_empty() {
        return Err("this installer holds no Loquendo voice".into());
    }
    let files = files.into_iter().zip(keep).filter(|(_, k)| *k).map(|(f, _)| f).collect();
    Ok(Unpacked { voices, files })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lde(language: &str, default: bool) -> String {
        format!(
            "\"Library\" = \"LTTS7English, English\"\n\"Language\" = \"{language}\"\n{}",
            if default { "\"DefaultVariant\" = \"yes\"\n" } else { "" }
        )
    }

    /// The mother tongue picks its own variant; a bare alias goes to the
    /// language's default variant.
    #[test]
    fn picks_the_voices_language() {
        let ldes = vec![
            ("data/languages/English/EnglishAu.lde".to_string(), lde("EnglishAu,en-AU,AU,Australian,en,English", false)),
            ("data/languages/English/EnglishGb.lde".to_string(), lde("English,en,en-GB,GB,British,EnglishGb", false)),
            ("data/languages/English/EnglishUs.lde".to_string(), lde("English,en,en-US,US,American,EnglishUs", true)),
        ];
        assert_eq!(language_descriptor(&ldes, "EnglishGb", ""), Some(1));
        assert_eq!(language_descriptor(&ldes, "englishus", ""), Some(2));
        assert_eq!(language_descriptor(&ldes, "Catalan", "ca-ES,English"), Some(2));
        assert_eq!(language_descriptor(&ldes, "Catalan", "ca-ES"), None);
    }

    /// QQ_INSTALLERS=<folder of installers>: every installer unpacks to at
    /// least one voice with its folder.
    #[test]
    fn unpacks_every_installer() {
        let Some(dir) = std::env::var_os("QQ_INSTALLERS") else { return };
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            if !e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("exe")) {
                continue;
            }
            let u = unpack_installer(&std::fs::read(e.path()).unwrap()).unwrap();
            assert!(u.files.iter().any(|(p, _)| p.ends_with(".sde")), "{}", e.path().display());
        }
    }
}
