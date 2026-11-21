//! The swappable pipeline, named after the original engine's own modules
//! (its module table in LoqTTS7.dll reads TOP, LES, FON, CAT, SIG, ACU):
//!
//!   FON  text -> sounds to say      builtin rules | original engine | any command
//!   CAT  sounds -> recorded pieces  builtin selection | original engine
//!   SIG  pieces -> audio            like the original | smoothed
//!
//! The original engine and commands are desktop only; `compare` scores each
//! stage against the original.

use quoquendo::g2p::{analyse, Frontend};
use quoquendo::loquendo::VoiceDatabase;
use quoquendo::synth::{self, JoinStyle, Phrase, TargetPhrase};
use std::ops::Range;

#[cfg(not(target_arch = "wasm32"))]
pub use super::oracle::Reference;

/// The original engine's results: none in the browser.
#[cfg(target_arch = "wasm32")]
pub enum Reference {}

#[derive(Clone, Debug, PartialEq)]
pub enum Fon {
    /// quoquendo's own letter-to-sound rules
    Rules,
    /// the original engine's transcription (runs it under Wine)
    #[cfg(not(target_arch = "wasm32"))]
    Original,
    /// any program: gets the text on stdin, prints a transcription in the
    /// engine's notation (see `synth::parse_transcription`) on stdout
    #[cfg(not(target_arch = "wasm32"))]
    Command(String),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cat {
    Builtin,
    /// the pieces the original engine chose (it uses its own FON then)
    #[cfg(not(target_arch = "wasm32"))]
    Original,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stages {
    pub fon: Fon,
    pub cat: Cat,
    pub sig: JoinStyle,
}

impl Default for Stages {
    fn default() -> Self {
        Stages {
            fon: Fon::Rules,
            cat: Cat::Builtin,
            sig: JoinStyle::Original,
        }
    }
}

pub struct Run {
    pub targets: Vec<TargetPhrase>,
    /// byte ranges in the text of each word the targets point at; empty when
    /// FON was not ours (its targets know no words)
    pub words: Vec<Range<usize>>,
    pub plan: Vec<Phrase>,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_command(cmd: &str, text: &str) -> Result<String, String> {
    use std::io::Write;
    let shell = if cfg!(windows) { ("cmd", "/C") } else { ("sh", "-c") };
    let mut child = std::process::Command::new(shell.0)
        .args([shell.1, cmd])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("{cmd}: {e}"))?;
    child.stdin.take().unwrap().write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("{cmd} failed ({})", out.status));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

#[cfg(not(target_arch = "wasm32"))]
impl Stages {
    /// Whether any stage needs the original engine's results.
    pub fn need_original(&self) -> bool {
        self.fon == Fon::Original || self.cat == Cat::Original
    }
}

/// FON and CAT for one text; SIG is `synth::render` on `plan`. `reference`
/// is the original engine's result for `text` when a stage needs it (fetched
/// here, under Wine, when not given).
pub fn run(text: &str, fe: &Frontend, voice: &str, db: &VoiceDatabase, stages: &Stages, reference: Option<Reference>) -> Result<Run, String> {
    #[cfg(not(target_arch = "wasm32"))]
    let reference = match reference {
        Some(r) => Some(r),
        None if stages.need_original() => Some(super::oracle::run_many(voice, &[text.to_string()])?.remove(0)),
        None => None,
    };
    #[cfg(target_arch = "wasm32")]
    let _ = (voice, reference);
    let (targets, words) = match &stages.fon {
        Fon::Rules => {
            let tokens = analyse(text, fe);
            (synth::from_tokens(&tokens), quoquendo::text::word_spans(text, &tokens))
        }
        #[cfg(not(target_arch = "wasm32"))]
        Fon::Original => (reference.as_ref().unwrap().target_phrases(), Vec::new()),
        #[cfg(not(target_arch = "wasm32"))]
        Fon::Command(cmd) => (synth::parse_transcription(&run_command(cmd, text)?), Vec::new()),
    };
    let plan = match stages.cat {
        Cat::Builtin => synth::select_all(db, &targets),
        #[cfg(not(target_arch = "wasm32"))]
        Cat::Original => reference.as_ref().unwrap().phrases(db)?,
    };
    Ok(Run { targets, words, plan })
}
