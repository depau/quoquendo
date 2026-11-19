//! Language modules ported from the original's `LTTS7<Language>.dll`s
//! (MECHANICS.md §4b). Their tables are read from the DLL the language pack
//! installs, so the module's bytes are needed at run time; no code of them
//! runs.

pub mod common;
pub mod elqnum;
pub mod english;
pub mod french;
pub mod german;
pub mod italian;
pub mod spanish;

use crate::g2p::{Language, Token};
use std::sync::Arc;

/// A ported language module: the original's FON stage for its language.
pub trait Module: Send + Sync {
    /// Text → words with their phones, and pauses, as `g2p::analyse` gives;
    /// with `voice`, the phones the voice says after its own stage
    /// (`voicefon`), else the module's.
    fn analyse(&self, text: &str, voice: Option<&crate::voicefon::VoiceFon>) -> Vec<Token>;
}

/// What a language module is built from.
#[derive(Default)]
pub struct ModuleFiles {
    /// `LTTS7<Language>.dll`, the `.lde`'s `Library`
    pub dll: Vec<u8>,
    /// the voice's `MotherTongue` (`EnglishGb`, `SpanishMx`…): one module
    /// serves several variants
    pub mother_tongue: String,
    /// the `.lde`'s `LexicalDB` and `GPR` files
    pub dbl: Option<Vec<u8>>,
    pub gpr: Option<Vec<u8>>,
    /// the `.lde`'s `Lexicon[i]` files (the engine's text filter, applied
    /// before tokenizing: `common::TextFilter`)
    pub lexicons: Vec<Vec<u8>>,
    /// the voice's phone table as (symbol, engine feature word)
    pub phones: Vec<(String, u32)>,
}

/// The ported module for `lang`, when there is one and the files hold it.
pub fn load(lang: Language, files: &ModuleFiles) -> Option<Arc<dyn Module>> {
    match lang {
        Language::Italian => italian::Italian::from_dll(files.dll.clone()).map(|m| Arc::new(m) as Arc<dyn Module>),
        Language::Spanish => spanish::Spanish::from_files(files).map(|m| Arc::new(m) as Arc<dyn Module>),
        Language::English => english::English::from_files(files).map(|m| Arc::new(m) as Arc<dyn Module>),
        Language::German => german::German::new(files.dll.clone(), files.dbl.as_deref()).map(|m| Arc::new(m) as Arc<dyn Module>),
        Language::French => french::French::from_files(files.dll.clone(), files.dbl.as_deref()).map(|m| Arc::new(m) as Arc<dyn Module>),
        _ => None,
    }
}
