//! Language modules ported from the original's `LTTS7<Language>.dll`s
//! (MECHANICS.md §4b). Their tables are read from the DLL the language pack
//! installs, so the module's bytes are needed at run time; no code of them
//! runs.

pub mod danish;
pub mod common;
pub mod dutch;
pub mod elqnum;
pub mod loqnum;
pub mod english;
pub mod french;
pub mod german;
pub mod greek;
pub mod italian;
pub mod polish;
pub mod portuguese;
pub mod russian;
pub mod spanish;
pub mod swedish;
pub mod textfilter;
pub mod wordanalyzer;

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
    /// the `.lde`'s `RegExp` and `Lexicon` files, in the engine's order, and
    /// LTTS7Util.dll: the text filters LesRun applies to the whole text
    /// before the sentence loop (`textfilter`)
    pub regexps: Vec<Vec<u8>>,
    pub lexicons: Vec<Vec<u8>>,
    pub util: Option<Vec<u8>>,
    /// LoqTTS7.dll: the generic number reader's data (`loqnum`)
    pub engine: Option<Vec<u8>>,
    /// the voice's phone table as (symbol, engine feature word)
    pub phones: Vec<(String, u32)>,
}

/// The ported module for `lang`, when there is one and the files hold it;
/// with the language's text filters in front of it.
pub fn load(lang: Language, files: &ModuleFiles) -> Option<Arc<dyn Module>> {
    let module = match lang {
        Language::Italian => italian::Italian::from_dll(files.dll.clone()).map(|m| Arc::new(m) as Arc<dyn Module>),
        Language::Spanish => spanish::Spanish::from_files(files).map(|m| Arc::new(m) as Arc<dyn Module>),
        Language::English => english::English::from_files(files).map(|m| Arc::new(m) as Arc<dyn Module>),
        Language::German => german::German::new(files.dll.clone(), files.dbl.as_deref()).map(|m| Arc::new(m) as Arc<dyn Module>),
        Language::French => french::French::from_files(files.dll.clone(), files.dbl.as_deref()).map(|m| Arc::new(m) as Arc<dyn Module>),
        Language::Greek => greek::Greek::from_files(files).map(|m| Arc::new(m) as Arc<dyn Module>),
        Language::Danish => danish::Danish::from_files(files).map(|m| Arc::new(m) as Arc<dyn Module>),
        Language::Portuguese => portuguese::Portuguese::from_files(files).map(|m| Arc::new(m) as Arc<dyn Module>),
        Language::Swedish => swedish::Swedish::new(files.dll.clone(), files.dbl.as_deref()).map(|m| Arc::new(m) as Arc<dyn Module>),
        Language::Dutch => dutch::Dutch::from_files(files).map(|m| Arc::new(m) as Arc<dyn Module>),
        Language::Polish => polish::Polish::from_files(files).map(|m| Arc::new(m) as Arc<dyn Module>),
        Language::Russian => russian::Russian::from_files(files).map(|m| Arc::new(m) as Arc<dyn Module>),
        _ => None,
    }?;
    let filters = files.util.as_deref().and_then(|u| textfilter::TextFilters::new(u, &files.regexps, &files.lexicons));
    Some(match filters {
        Some(filters) => Arc::new(Filtered { module, filters }),
        // a tree without LTTS7Util.dll (installed before it was kept): the
        // text goes to the module unfiltered
        None => module,
    })
}

/// A module behind its language's text filters: LesRun (LoqTTS7.dll
/// 0x10035b20) rewrites the whole text, then the sentence loop runs.
struct Filtered {
    module: Arc<dyn Module>,
    filters: textfilter::TextFilters,
}

impl Module for Filtered {
    fn analyse(&self, text: &str, voice: Option<&crate::voicefon::VoiceFon>) -> Vec<Token> {
        self.module.analyse(&self.filters.apply(text), voice)
    }
}
