//! quoquendo: speak with Loquendo 7 (LTTS7) voices without the original
//! engine. Give it a voice's files (as its installer unpacks them) and text;
//! it runs the engine's own stages, rebuilt from its code:
//!
//! - FON (`g2p`): text → the voice's phone symbols, phrase by phrase;
//! - CAT (`synth::select_all`): which stretches of the recordings to play,
//!   with the original's scoring (`phoneset` builds its tables from the
//!   voice's own library);
//! - SIG (`synth::render`): Speex decoding (`codec`) and joining.
//!
//! ```no_run
//! let data = std::path::Path::new("Loquendo/LTTS7/data");
//! let voice = quoquendo::Voice::find(data, "Roberto")?;
//! let speech = voice.say("Attenzione! Allontanarsi dalla linea gialla.")?;
//! std::fs::write("out.wav", quoquendo::synth::to_wav_bytes(&speech)).unwrap();
//! # Ok::<(), String>(())
//! ```

pub mod codec;
pub mod french;
pub mod g2p;
pub mod german;
pub mod gpr;
pub mod inno;
pub mod loquendo;
pub mod pack;
pub mod phoneset;
pub mod phonetics;
pub mod sig;
pub mod spanish;
pub mod speex;
pub mod sqlite;
pub mod synth;
pub mod wise;

use std::collections::HashMap;
use std::path::Path;

pub use loquendo::{discover_voices, VoiceDatabase, VoiceDescriptor};
pub use synth::{JoinStyle, Settings, Speech, TargetPhrase};

/// A voice ready to speak: its description, its loaded recordings index and
/// its front end.
pub struct Voice {
    pub descriptor: VoiceDescriptor,
    pub db: VoiceDatabase,
    pub frontend: g2p::Frontend,
    /// the `MotherTongue` when no language data in its tree speaks it: the
    /// front end then runs on its built-in rules alone
    pub language_missing: Option<String>,
}

impl Voice {
    /// Load a voice described by its `.vcf`/`.vde` (`discover_voices`), with
    /// its learned lexicon when one was registered (`g2p::add_lexicon_dir`).
    pub fn open(descriptor: &VoiceDescriptor) -> Result<Voice, String> {
        let data = if descriptor.data_dir.as_os_str().is_empty() {
            descriptor
                .voice_path
                .ancestors()
                .find(|a| a.join("voices").is_dir())
                .ok_or_else(|| format!("{}: not inside an LTTS7 data folder", descriptor.voice_path.display()))?
                .to_path_buf()
        } else {
            descriptor.data_dir.clone()
        };
        let root = data.parent().unwrap_or(Path::new(".")).to_path_buf();
        let read = |rel: &str| std::fs::read(root.join(rel)).ok();
        let list = |rel: &str| -> Vec<String> {
            std::fs::read_dir(root.join(rel))
                .map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect())
                .unwrap_or_default()
        };
        let mut v = Voice::from_bytes(VoiceFiles::from_layout(&descriptor.name, &read, &list)?)?;
        v.db.path = descriptor.voice_path.join(format!("{}.sde", v.descriptor.name));
        v.db.audio_path = Some(descriptor.voice_path.join(&v.descriptor.bin_name));
        v.descriptor.data_dir = data;
        v.descriptor.voice_path = descriptor.voice_path.clone();
        if let Some(lexicon) = g2p::find_lexicon(&v.descriptor.mother_tongue) {
            v.frontend = v.frontend.clone().with_lexicon(&lexicon);
        }
        Ok(v)
    }

    /// Load the voice called `name` from an LTTS7 `data` folder.
    pub fn find(data_dir: &Path, name: &str) -> Result<Voice, String> {
        let d = discover_voices(data_dir)
            .into_iter()
            .find(|v| v.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("no voice {name} in {}", data_dir.display()))?;
        Voice::open(&d)
    }

    /// The FON stage: what the voice will say, in its own phone notation.
    pub fn transcribe(&self, text: &str) -> Vec<TargetPhrase> {
        synth::from_tokens(&g2p::analyse(text, &self.frontend))
    }

    /// Speak `text` the way the original engine does.
    pub fn say(&self, text: &str) -> Result<Speech, String> {
        self.say_with(
            text,
            Settings {
                volume: 1.0,
                join: JoinStyle::Original,
            },
        )
    }

    /// Speak `text` with other settings (volume, smoothed joins).
    pub fn say_with(&self, text: &str, settings: Settings) -> Result<Speech, String> {
        let plan = synth::select_all(&self.db, &self.transcribe(text));
        synth::render(&plan, &self.db, settings)
    }
}

/// The contents of a voice's files, for loading it without a filesystem
/// (a browser, a server holding them in memory). Without `library` the
/// picker falls back to its approximation, without `dbl`/`gpr` the front end
/// uses its own rules.
#[derive(Default)]
pub struct VoiceFiles {
    /// `<Voice>.vcf` (voice configurator, INI text)
    pub vcf: Vec<u8>,
    /// `<Voice>.vde` (voice description, INI text)
    pub vde: Vec<u8>,
    /// `<Voice>.sde` (recordings index)
    pub sde: Vec<u8>,
    /// `<Voice>.<rate>.<codec>.<…>.bin` (recordings), and its file name
    pub bin: Vec<u8>,
    pub bin_name: String,
    /// `LTTS7<Voice>.dll` (its phone table is read as data, never run)
    pub library: Option<Vec<u8>>,
    /// the `LexicalDB` and `GPR` files its language's `.lde` names
    pub dbl: Option<Vec<u8>>,
    pub gpr: Option<Vec<u8>>,
    /// the voice's `MotherTongue` when no `.lde` in the tree speaks it
    pub language_missing: Option<String>,
}

fn ini(bytes: &[u8]) -> HashMap<String, String> {
    loquendo::parse_ini(&loquendo::latin1(bytes))
}

impl VoiceFiles {
    /// A voice's files from an LTTS7 tree: `read(path)` gives a file's bytes
    /// and `list(folder)` the names in a folder, both relative to the LTTS7
    /// root (`data/Roberto.vcf`, `bin`). Names match case-insensitively.
    pub fn from_layout(voice: &str, read: &dyn Fn(&str) -> Option<Vec<u8>>, list: &dyn Fn(&str) -> Vec<String>) -> Result<VoiceFiles, String> {
        let sorted = |dir: &str| {
            let mut names = list(dir);
            names.sort();
            names
        };
        let (vcf, cv) = sorted("data")
            .iter()
            .filter(|n| n.to_ascii_lowercase().ends_with(".vcf"))
            .filter_map(|n| read(&format!("data/{n}")))
            .map(|b| {
                let c = ini(&b);
                (b, c)
            })
            .find(|(_, c)| c.get("BaseVoice").is_some_and(|b| b.eq_ignore_ascii_case(voice)))
            .ok_or_else(|| format!("no voice {voice} here"))?;
        let base = cv["BaseVoice"].clone();
        let voice_path = cv.get("VoicePath").map(|p| p.replace('\\', "/").trim_matches('/').to_string());
        let dir = format!("data/{}", voice_path.unwrap_or_else(|| format!("voices/{base}")));
        let in_dir = sorted(&dir);
        let pick = |want: &str| {
            in_dir
                .iter()
                .find(|n| n.eq_ignore_ascii_case(want))
                .and_then(|n| read(&format!("{dir}/{n}")))
        };
        let vde = pick(&format!("{base}.vde")).ok_or("the voice has no .vde")?;
        let sde = pick(&format!("{base}.sde")).ok_or("the voice has no .sde")?;
        // the recordings: the biggest <Voice>.*.bin
        let prefix = format!("{}.", base.to_ascii_lowercase());
        let (bin_name, bin) = in_dir
            .iter()
            .filter(|n| {
                let l = n.to_ascii_lowercase();
                l.starts_with(&prefix) && l.ends_with(".bin")
            })
            .filter_map(|n| Some((n.clone(), read(&format!("{dir}/{n}"))?)))
            .max_by_key(|(_, d)| d.len())
            .ok_or("the voice has no recordings")?;
        let library = sorted("bin")
            .iter()
            .find(|n| n.eq_ignore_ascii_case(&format!("LTTS7{base}.dll")))
            .and_then(|n| read(&format!("bin/{n}")));
        // the language: the `.lde` that speaks the voice's MotherTongue
        let dv = ini(&vde);
        let tongue = dv.get("MotherTongue").cloned().unwrap_or_default();
        let aliases = dv.get("MotherTongueAliases").cloned().unwrap_or_default();
        let mut ldes = Vec::new();
        for lang in sorted("data/languages") {
            let folder = format!("data/languages/{lang}");
            for f in sorted(&folder).iter().filter(|f| f.to_ascii_lowercase().ends_with(".lde")) {
                let path = format!("{folder}/{f}");
                if let Some(b) = read(&path) {
                    ldes.push((path, loquendo::latin1(&b)));
                }
            }
        }
        let (dbl, gpr, language_missing) = match pack::language_descriptor(&ldes, &tongue, &aliases) {
            Some(i) => {
                let folder = ldes[i].0.rsplit_once('/').map(|(d, _)| d.to_string()).unwrap_or_default();
                let lde = loquendo::parse_ini(&ldes[i].1);
                let file = |key: &str| lde.get(key).and_then(|f| read(&format!("{folder}/{f}")));
                (file("LexicalDB"), file("GPR"), None)
            }
            None => (None, None, Some(tongue)),
        };
        Ok(VoiceFiles {
            vcf,
            vde,
            sde,
            bin,
            bin_name,
            library,
            dbl,
            gpr,
            language_missing,
        })
    }

    /// `from_layout` over files held in memory, keyed by their path under
    /// the LTTS7 root.
    pub fn from_map(voice: &str, files: &HashMap<String, Vec<u8>>) -> Result<VoiceFiles, String> {
        let read = |p: &str| files.get(p).cloned();
        let list = |dir: &str| -> Vec<String> {
            let prefix = format!("{dir}/");
            let mut names: Vec<String> = files
                .keys()
                .filter_map(|k| k.strip_prefix(&prefix))
                .map(|rest| rest.split('/').next().unwrap_or(rest).to_string())
                .collect();
            names.sort();
            names.dedup();
            names
        };
        VoiceFiles::from_layout(voice, &read, &list)
    }
}

impl Voice {
    /// Load a voice from its files' bytes.
    pub fn from_bytes(files: VoiceFiles) -> Result<Voice, String> {
        let mut descriptor = VoiceDescriptor::from_ini(&ini(&files.vcf), &ini(&files.vde));
        descriptor.set_bin_name(&files.bin_name);
        let mut db = loquendo::parse_sde(&loquendo::latin1(&files.sde));
        db.size_bytes = files.sde.len() as u64;
        db.audio_size = files.bin.len() as u64;
        db.audio_rate_hz = descriptor.audio_rate;
        if files.bin_name.contains("LTTS7CodecMsx") {
            db.audio_error = Some("codec LTTS7CodecMsx not supported yet".into());
        } else {
            match codec::AudioBank::parse(files.bin) {
                Ok(bank) => db.audio = Some(bank),
                Err(e) => db.audio_error = Some(e),
            }
        }
        db.tables = files.library.as_deref().and_then(|l| synth::PhoneTables::from_library(l, &db));
        let lang = descriptor.language();
        let frontend = match &db.tables {
            Some(t) if files.dbl.is_some() || files.gpr.is_some() => {
                g2p::Frontend::from_files(lang, files.dbl.as_deref(), files.gpr.as_deref(), &t.phone_features())
            }
            _ => g2p::Frontend::rules(lang),
        };
        Ok(Voice {
            descriptor,
            db,
            frontend,
            language_missing: files.language_missing,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oracle_root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../oracle/prefix/drive_c/Program Files (x86)/Loquendo/LTTS7")
    }

    /// Loading from bytes speaks exactly like loading from the folder.
    #[test]
    fn bytes_and_folder_say_the_same() {
        let root = oracle_root();
        let mut files = HashMap::new();
        for rel in [
            "data/Roberto.vcf",
            "data/voices/Roberto/Roberto.vde",
            "data/voices/Roberto/Roberto.sde",
            "data/voices/Roberto/Roberto.32000.LTTS7Codec210.44000.10.bin",
            "bin/LTTS7Roberto.dll",
            "data/languages/Italian/Italian.lde",
        ] {
            let Ok(b) = std::fs::read(root.join(rel)) else { return };
            files.insert(rel.to_string(), b);
        }
        let a = Voice::from_bytes(VoiceFiles::from_map("roberto", &files).unwrap()).unwrap();
        assert!(a.db.tables.is_some());
        let a = a.say("Binario tre.").unwrap();
        let b = Voice::find(&root.join("data"), "Roberto").unwrap().say("Binario tre.").unwrap();
        assert!(!a.samples.is_empty());
        assert_eq!(a.samples, b.samples);
    }

    /// QQ_INSTALLER=<Voice - Language.exe> QQ_DATA_DIR=<its installed LTTS7>:
    /// the voice unpacked from its installer says what the installed one says.
    #[test]
    fn installer_and_folder_say_the_same() {
        let (Some(exe), Some(dir)) = (std::env::var_os("QQ_INSTALLER"), std::env::var_os("QQ_DATA_DIR")) else {
            return;
        };
        let unpacked = pack::unpack_installer(&std::fs::read(exe).unwrap()).unwrap();
        let name = unpacked.voices[0].clone();
        let files: HashMap<String, Vec<u8>> = unpacked.files.into_iter().collect();
        let a = Voice::from_bytes(VoiceFiles::from_map(&name, &files).unwrap()).unwrap();
        let text = a.descriptor.demo_sentence.clone();
        let a = a.say(&text).unwrap();
        let dir = loquendo::resolve_data_dir(Path::new(&dir)).unwrap();
        let b = Voice::find(&dir, &name).unwrap().say(&text).unwrap();
        assert!(!a.samples.is_empty());
        assert_eq!(a.samples, b.samples);
    }
}
