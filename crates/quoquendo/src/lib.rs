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
pub mod phoneset;
pub mod phonetics;
pub mod sig;
pub mod spanish;
pub mod sqlite;
pub mod synth;

use std::path::Path;

pub use loquendo::{discover_voices, VoiceDatabase, VoiceDescriptor};
pub use synth::{JoinStyle, Settings, Speech, TargetPhrase};

/// A voice ready to speak: its description and its loaded recordings index.
pub struct Voice {
    pub descriptor: VoiceDescriptor,
    pub db: VoiceDatabase,
}

impl Voice {
    /// Load a voice described by its `.vcf`/`.vde` (`discover_voices`).
    pub fn open(descriptor: &VoiceDescriptor) -> Result<Voice, String> {
        let db = loquendo::load_sde(&descriptor.voice_path.join(format!("{}.sde", descriptor.name)))?;
        g2p::load_lexicon(descriptor.language(), &descriptor.mother_tongue);
        load_language_files(descriptor, &db);
        Ok(Voice {
            descriptor: descriptor.clone(),
            db,
        })
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
        synth::from_tokens(&g2p::analyse(text, self.descriptor.language()))
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
/// (a browser, a server holding them in memory). Only `sde` and `bin` are
/// required; without `library` the picker falls back to its approximation,
/// without `dbl`/`gpr` the front end uses its own rules.
#[derive(Default)]
pub struct VoiceFiles {
    /// `<Voice>.vde` (voice description, INI text)
    pub vde: Vec<u8>,
    /// `<Voice>.sde` (recordings index)
    pub sde: Vec<u8>,
    /// `<Voice>.<rate>.LTTS7Codec210.<…>.bin` (recordings)
    pub bin: Vec<u8>,
    /// `LTTS7<Voice>.dll` (its phone table is read as data, never run)
    pub library: Option<Vec<u8>>,
    /// `<MotherTongue>.dbl` and `.gpr` from `languages/<Language>/`
    pub dbl: Option<Vec<u8>>,
    pub gpr: Option<Vec<u8>>,
}

impl VoiceFiles {
    /// A voice's files straight from its Loquendo installer
    /// (`<Voice> - <Language>.exe`, Inno Setup 5.2.3), read not run.
    pub fn from_installer(exe: &[u8]) -> Result<VoiceFiles, String> {
        let low = |p: &str| p.to_lowercase();
        let files = inno::files(exe, |p| {
            let p = low(p);
            p.contains("/data/") && !p.contains("/gilded") || p.contains("/bin/ltts7")
        })?;
        let ends = |suffix: &str| files.iter().find(|(p, _)| low(p).ends_with(&low(suffix))).map(|(_, d)| d.clone());
        let vcf = files
            .iter()
            .find(|(p, _)| low(p).ends_with(".vcf"))
            .ok_or("this installer holds no Loquendo voice")?;
        let name = loquendo::parse_ini(&loquendo::latin1(&vcf.1))
            .get("BaseVoice")
            .cloned()
            .ok_or("the voice file names no voice")?;
        let vde = ends(&format!("/{name}/{name}.vde")).ok_or("the voice has no .vde")?;
        let tongue = loquendo::parse_ini(&loquendo::latin1(&vde)).get("MotherTongue").cloned().unwrap_or_default();
        // the recordings: the biggest .bin in the voice's folder
        let bin = files
            .iter()
            .filter(|(p, _)| low(p).contains(&format!("/voices/{}/", low(&name))) && low(p).ends_with(".bin"))
            .max_by_key(|(_, d)| d.len())
            .map(|(_, d)| d.clone())
            .ok_or("the voice has no recordings")?;
        Ok(VoiceFiles {
            sde: ends(&format!("/{name}/{name}.sde")).ok_or("the voice has no .sde")?,
            vde,
            bin,
            library: ends(&format!("/bin/LTTS7{name}.dll")),
            dbl: ends(&format!("/{tongue}.dbl")),
            gpr: ends(&format!("/{tongue}.gpr")),
        })
    }
}

impl Voice {
    /// Load a voice from its files' bytes.
    pub fn from_bytes(files: VoiceFiles) -> Result<Voice, String> {
        let ini = loquendo::parse_ini(&loquendo::latin1(&files.vde));
        let descriptor = VoiceDescriptor {
            // `"Library" = "LTTS7Katrin, Katrin"`: the voice's name is the second part
            name: ini
                .get("BaseVoice")
                .cloned()
                .or_else(|| ini.get("Library").and_then(|l| l.split(',').nth(1)).map(|n| n.trim().to_string()))
                .unwrap_or_default(),
            description: ini.get("BaseDescription").cloned().unwrap_or_default(),
            mother_tongue: ini.get("MotherTongue").cloned().unwrap_or_default(),
            mother_tongue_aliases: ini.get("MotherTongueAliases").cloned().unwrap_or_default(),
            gender: ini.get("Gender").cloned().unwrap_or_default(),
            ..Default::default()
        };
        let mut db = loquendo::parse_sde(&loquendo::latin1(&files.sde));
        db.audio = Some(codec::AudioBank::parse(files.bin)?);
        db.tables = files.library.as_deref().and_then(|l| synth::PhoneTables::from_library(l, &db));
        if let Some(t) = &db.tables {
            g2p::load_language_data(descriptor.language(), files.dbl.as_deref(), files.gpr.as_deref(), &t.phone_features());
        }
        Ok(Voice { descriptor, db })
    }
}

/// Read the voice's language files beside it (`data/languages/*/<MotherTongue>.dbl`
/// and `.gpr`, installed with the voice) into the front end.
pub fn load_language_files(descriptor: &VoiceDescriptor, db: &VoiceDatabase) -> usize {
    let Some(tables) = &db.tables else { return 0 };
    let Some(data) = descriptor.voice_path.parent().and_then(|p| p.parent()) else {
        return 0;
    };
    let find = |ext: &str| -> Option<Vec<u8>> {
        let want = format!("{}.{ext}", descriptor.mother_tongue).to_lowercase();
        for dir in std::fs::read_dir(data.join("languages")).ok()?.flatten() {
            for f in std::fs::read_dir(dir.path()).ok()?.flatten() {
                if f.file_name().to_string_lossy().to_lowercase() == want {
                    return std::fs::read(f.path()).ok();
                }
            }
        }
        None
    };
    let (dbl, gpr) = (find("dbl"), find("gpr"));
    if dbl.is_none() && gpr.is_none() {
        return 0;
    }
    g2p::load_language_data(descriptor.language(), dbl.as_deref(), gpr.as_deref(), &tables.phone_features())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Loading from bytes speaks exactly like loading from the folder.
    #[test]
    fn bytes_and_folder_say_the_same() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../oracle/prefix/drive_c/Program Files (x86)/Loquendo/LTTS7");
        let v = root.join("data/voices/Roberto");
        let Ok(sde) = std::fs::read(v.join("Roberto.sde")) else { return };
        let files = VoiceFiles {
            vde: std::fs::read(v.join("Roberto.vde")).unwrap(),
            sde,
            bin: std::fs::read(v.join("Roberto.32000.LTTS7Codec210.44000.10.bin")).unwrap(),
            library: std::fs::read(root.join("bin/LTTS7Roberto.dll")).ok(),
            ..Default::default()
        };
        let a = Voice::from_bytes(files).unwrap().say("Binario tre.").unwrap();
        let b = Voice::find(&root.join("data"), "Roberto").unwrap().say("Binario tre.").unwrap();
        assert!(!a.samples.is_empty());
        assert_eq!(a.samples, b.samples);
    }

    /// QQ_INSTALLER=<Voice - Language.exe> QQ_DATA_DIR=<its installed LTTS7>:
    /// the voice read from inside its installer says what the installed one says.
    #[test]
    fn installer_and_folder_say_the_same() {
        let (Some(exe), Some(dir)) = (std::env::var_os("QQ_INSTALLER"), std::env::var_os("QQ_DATA_DIR")) else {
            return;
        };
        let files = VoiceFiles::from_installer(&std::fs::read(exe).unwrap()).unwrap();
        let a = Voice::from_bytes(files).unwrap();
        let name = a.descriptor.name.clone();
        let a = a.say("Zwei, drei.").unwrap();
        let dir = loquendo::resolve_data_dir(Path::new(&dir)).unwrap();
        let v = Voice::find(&dir, &name).unwrap();
        load_language_files(&v.descriptor, &v.db);
        let b = v.say("Zwei, drei.").unwrap();
        assert!(!a.samples.is_empty());
        assert_eq!(a.samples, b.samples);
    }
}
