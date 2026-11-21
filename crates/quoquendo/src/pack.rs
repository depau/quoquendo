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

/// Unpack a voice installer, a language distribution or LTTS 7.2's
/// `Chinese.chd` into the files its voices or language need.
pub fn unpack_installer(exe: &[u8]) -> Result<Unpacked, String> {
    let files: Vec<(String, Vec<u8>)> = if is_inno(exe) {
        // 7.2 bundles keep voices and languages in `modules/`, described by
        // `global.ttsconf` (`place_ttsconf`)
        let keep = |p: &str| {
            let p = lower(p);
            p.strip_prefix(INNO_ROOT).is_some_and(|r| {
                r.starts_with("data/") || r.starts_with("modules/") || r == "global.ttsconf" || (r.starts_with("bin/ltts7") || r == "bin/loqtts7.dll") && r.ends_with(".dll")
            })
        };
        let files = inno::files(exe, keep)?.into_iter().map(|(p, d)| (p[INNO_ROOT.len()..].to_string(), d)).collect();
        place_ttsconf(files)
    } else if is_chinese_dictionary(exe) {
        chinese_language(exe)
    } else {
        place_wise(wise::files(exe)?)?
    };
    select(files)
}

/// LTTS 7.2's `Chinese.chd`: UTF-8 lines of a word and its toned pinyin
/// (`一\tyi1`). No installer carries it (Loquendo's Android build does).
fn is_chinese_dictionary(data: &[u8]) -> bool {
    let head = String::from_utf8_lossy(&data[..data.len().min(256)]);
    head.lines().next().and_then(|l| l.split_once('\t')).is_some_and(|(word, pinyin)| {
        word.chars().all(|c| c as u32 >= 0x3400) && !word.is_empty() && pinyin.trim_end().ends_with(|c: char| ('1'..='5').contains(&c))
    })
}

/// The Chinese language folder from its dictionary: `Chinese.chd` and an
/// `.lde` naming it as the `LexicalDB` (LTTS 7.2 configures the language in
/// `global.ttsconf` instead; there is no 7.0 one), read by `g2p::chinese`.
fn chinese_language(chd: &[u8]) -> Vec<(String, Vec<u8>)> {
    let lde = "; Language descriptor for Loquendo TTS, written by quoquendo for LTTS 7.2's Chinese.chd\r\n\r\n\"Language\" = \"Chinese,zh-CN,zh,Mandarin\"\r\n\"DefaultVariant\" = \"yes\"\r\n\"LexicalDB\" = \"Chinese.chd\"\r\n";
    vec![
        ("data/languages/Chinese/Chinese.lde".into(), lde.as_bytes().to_vec()),
        ("data/languages/Chinese/Chinese.chd".into(), chd.to_vec()),
    ]
}

/// A 7.2 bundle (`global.ttsconf`, `modules/<Name>/`) as the 7.0 tree the
/// rest reads: each voice whose module folder it carries gets a `.vcf` and
/// `.vde` written from its `<voice>` entry, its files in
/// `data/voices/<Name>/`, its library in `bin/`. The bundle's 7.2 language
/// modules and engine libraries stay out: they differ from 7.0's, which the
/// ported languages are matched to, so the voice reads its language from a
/// 7.0 installer's (or the mirror's). Other installers pass through.
fn place_ttsconf(files: Vec<(String, Vec<u8>)>) -> Vec<(String, Vec<u8>)> {
    let Some((_, conf)) = files.iter().find(|(p, _)| lower(p) == "global.ttsconf") else { return files };
    let voices: Vec<crate::ttsconf::VoiceEntry> = crate::ttsconf::voices(&String::from_utf8_lossy(conf))
        .into_iter()
        .filter(|v| {
            let module = format!("modules/{}/", lower(&v.name));
            files.iter().any(|(p, _)| lower(p).starts_with(&module))
        })
        .collect();
    let mut out = Vec::new();
    for v in &voices {
        let lang = crate::g2p::Language::from_voice(&v.mother_tongue, &v.aliases.join(","));
        let encode = |s: String| s.chars().map(|c| crate::lang::common::to_8bit_cp(c, lang.code_page()).unwrap_or(b'?')).collect::<Vec<u8>>();
        let name = &v.name;
        let mut vcf = format!(
            "; Voice Configurator File for Loquendo TTS, from LTTS 7.2's global.ttsconf\r\n\r\n\"Description\" = \"{}\"\r\n\"BaseVoice\" = \"{name}\"\r\n\"VoicePath\" = \"voices/{name}\"\r\n",
            v.description
        );
        for (i, db) in v.vocal_dbs.iter().enumerate() {
            vcf.push_str(&format!("\"VocalDBName[{i}]\" = \"{db}\"\r\n"));
        }
        let mut gender = v.gender.clone();
        if let Some(first) = gender.get_mut(..1) {
            first.make_ascii_uppercase();
        }
        let vde = format!(
            "; Voice descriptor file for Loquendo TTS, from LTTS 7.2's global.ttsconf\r\n\r\n\"BaseDescription\" = \"{}\"\r\n\"Library\" = \"LTTS7{name}, {name}\"\r\n\"BaseSpeed\" = \"{}\"\r\n\"BasePitch\" = \"{}\"\r\n\"Gender\" = \"{gender}\"\r\n\"Age\" = \"{}\"\r\n\"GildedTTS\" = \"{}\"\r\n\"MotherTongue\" = \"{}\"\r\n\"MotherTongueAliases\" = \"{}\"\r\n\"DemoSentence\" = \"{}\"\r\n",
            v.description,
            v.base_speed,
            v.base_pitch,
            v.age,
            if v.gilded { "yes" } else { "no" },
            v.mother_tongue,
            v.aliases.join(","),
            v.demo_sentence
        );
        out.push((format!("data/{name}.vcf"), encode(vcf)));
        out.push((format!("data/voices/{name}/{name}.vde"), encode(vde)));
    }
    for (path, data) in files {
        let low = lower(&path);
        let owner = voices.iter().find(|v| low.starts_with(&format!("modules/{}/", lower(&v.name))) || low == format!("bin/ltts7{}.dll", lower(&v.name)));
        if let Some(v) = owner {
            let place = match path.strip_prefix("modules/").and_then(|r| r.split_once('/')) {
                Some((_, file)) => format!("data/voices/{}/{file}", v.name),
                None => path,
            };
            out.push((place, data));
        }
    }
    out
}

/// Wise cabinets are flat: the `.vcf` goes in `data/`, libraries in `bin/`,
/// the rest in the voice's folder. A language distribution holds no `.vcf`:
/// its language files go in the folder the `.lcf`s name (`LanguagePath`,
/// both `Catalan.lcf` and `Valencian.lcf` say `Catalan`), the library in
/// `bin/`.
fn place_wise(members: Vec<(String, Vec<u8>)>) -> Result<Vec<(String, Vec<u8>)>, String> {
    let Some((_, vcf)) = members.iter().find(|(n, _)| lower(n).ends_with(".vcf")) else {
        return place_wise_language(members);
    };
    let voice_path = parse_ini(&latin1(vcf))
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

/// The language branch: `.lde/.dbl/.lex/.rex/.lcf` (and whatever other
/// language file the cabinet holds) under the `.lcf`'s folder, the library
/// in `bin/`; the manual and the logo dropped.
fn place_wise_language(members: Vec<(String, Vec<u8>)>) -> Result<Vec<(String, Vec<u8>)>, String> {
    let language_folder = || -> Option<String> {
        let lcf = members.iter().find(|(n, _)| lower(n).ends_with(".lcf"))?;
        parse_ini(&latin1(&lcf.1))
            .get("LanguagePath")?
            .replace('\\', "/")
            .trim_matches('/')
            .rsplit('/')
            .next()
            .map(str::to_string)
    };
    let Some(folder) = language_folder() else {
        return Err("this installer holds no Loquendo voice or language".into());
    };
    Ok(members
        .into_iter()
        .filter_map(|(name, data)| {
            let low = lower(&name);
            let path = if low.starts_with("ltts7") && low.ends_with(".dll") {
                format!("bin/{name}")
            } else if low.ends_with(".pdf") || low.ends_with(".ico") {
                return None;
            } else {
                format!("data/languages/{folder}/{name}")
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
    // a language distribution installs no voice: keep its language folders
    // whole (every file the `.lde`s and `.lcf`s may name is already placed
    // there), and the libraries they name.
    if voices.is_empty() {
        if ldes.is_empty() {
            return Err("this installer holds no Loquendo voice".into());
        }
        let mut kept: Vec<(String, Vec<u8>)> = Vec::new();
        for (path, data) in files {
            let low = lower(&path);
            let language_file = low.starts_with("data/languages/") && low.matches('/').count() == 3;
            if language_file || low.starts_with("bin/") && ldes.iter().any(|(_, t)| {
                let lib = parse_ini(t).get("Library").and_then(|l| l.split(',').next().map(|s| s.trim().to_lowercase()));
                lib.is_some_and(|lib| low == format!("bin/{lib}.dll"))
            }) {
                kept.push((path, data));
            }
        }
        return Ok(Unpacked { voices, files: kept });
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

    /// QQ_INSTALLERS=<folder of installers>: every voice installer unpacks to
    /// at least one voice with its folder; every language distribution to
    /// its language folder, dictionary and the library its `.lde` names.
    #[test]
    fn unpacks_every_installer() {
        let Some(dir) = std::env::var_os("QQ_INSTALLERS") else { return };
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            if !e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("exe")) {
                continue;
            }
            let u = unpack_installer(&std::fs::read(e.path()).unwrap()).unwrap();
            let name = e.path().display().to_string();
            if !u.voices.is_empty() {
                assert!(u.files.iter().any(|(p, _)| p.ends_with(".sde")), "{name}");
                continue;
            }
            let ldes: Vec<&(String, Vec<u8>)> = u.files.iter().filter(|(p, _)| p.ends_with(".lde")).collect();
            assert!(!ldes.is_empty(), "{name}");
            for (p, d) in ldes {
                assert!(p.starts_with("data/languages/") && p.matches('/').count() == 3, "{name}: {p}");
                assert!(u.files.iter().any(|(q, _)| q.starts_with(folder(p)) && q.ends_with(".dbl")), "{name}: {p}");
                let lib = parse_ini(&latin1(d)).get("Library").and_then(|l| l.split(',').next().map(|s| s.trim().to_string())).unwrap();
                assert!(u.files.iter().any(|(q, _)| lower(q) == lower(&format!("bin/{lib}.dll"))), "{name}: {lib}");
            }
        }
    }
}
