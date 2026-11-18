//! Language modules ported from the original's `LTTS7<Language>.dll`s
//! (MECHANICS.md §4b). Their tables are read from the DLL the language pack
//! installs, so the module's bytes are needed at run time; no code of them
//! runs.

pub mod common;
pub mod italian;

use crate::g2p::{Language, Token};
use std::sync::Arc;

/// A ported language module: the original's FON stage for its language.
pub trait Module: Send + Sync {
    /// Text → words with their phones, and pauses, as `g2p::analyse` gives.
    fn analyse(&self, text: &str) -> Vec<Token>;
}

/// What a language module is built from.
pub struct ModuleFiles {
    /// `LTTS7<Language>.dll`, the `.lde`'s `Library`
    pub dll: Vec<u8>,
    /// the voice's `MotherTongue` (`EnglishGb`, `SpanishMx`…): one module
    /// serves several variants
    pub mother_tongue: String,
    /// the `.lde`'s `LexicalDB` and `GPR` files
    pub dbl: Option<Vec<u8>>,
    pub gpr: Option<Vec<u8>>,
    /// the voice's phone table as (symbol, engine feature word)
    pub phones: Vec<(String, u32)>,
}

/// The ported module for `lang`, when there is one and the files hold it.
pub fn load(lang: Language, files: &ModuleFiles) -> Option<Arc<dyn Module>> {
    match lang {
        Language::Italian => italian::Italian::from_dll(files.dll.clone()).map(|m| Arc::new(m) as Arc<dyn Module>),
        _ => None,
    }
}
