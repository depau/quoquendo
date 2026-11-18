//! Language modules ported from the original's `LTTS7<Language>.dll`s
//! (MECHANICS.md §4b). Their tables are read from the DLL the language pack
//! installs, so the module's bytes are needed at run time; no code of them
//! runs.

pub mod common;
pub mod italian;
