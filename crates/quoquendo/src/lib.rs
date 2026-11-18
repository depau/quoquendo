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
pub mod g2p;
pub mod loquendo;
pub mod phoneset;
pub mod phonetics;
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
        Ok(Voice { descriptor: descriptor.clone(), db })
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
        self.say_with(text, Settings { volume: 1.0, join: JoinStyle::Original })
    }

    /// Speak `text` with other settings (volume, smoothed joins).
    pub fn say_with(&self, text: &str, settings: Settings) -> Result<Speech, String> {
        let plan = synth::select_all(&self.db, &self.transcribe(text));
        synth::render(&plan, &self.db, settings)
    }
}
