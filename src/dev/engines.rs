//! Installations of the original engine the developer tools also read
//! voices from: the reference engine's Wine prefix and the usual install
//! places.

use crate::install::{store_override, voice_store};
use quoquendo::loquendo::{discover_voices, resolve_data_dir, VoiceDescriptor};
use std::path::PathBuf;

/// Every voice the command line can use: the ones taken from installers
/// (`voice_store()`) first, then any other installation found.
pub fn all_voices() -> Vec<VoiceDescriptor> {
    let mut out: Vec<VoiceDescriptor> = Vec::new();
    // QQ_DATA_DIR=<LTTS7 folder or its data folder>, or --store <LTTS7
    // folder>: only that installation
    let only = std::env::var_os("QQ_DATA_DIR").map(PathBuf::from).or_else(store_override);
    if let Some(dir) = only {
        return resolve_data_dir(&dir).map(|d| discover_voices(&d)).unwrap_or_default();
    }
    let dirs = [voice_store().and_then(|s| resolve_data_dir(&s)), resolve_data_dir_auto()];
    for d in dirs.into_iter().flatten() {
        for v in discover_voices(&d) {
            if !out.iter().any(|o| o.name.eq_ignore_ascii_case(&v.name)) {
                out.push(v);
            }
        }
    }
    out
}

/// Typical places the LTTS7 tree turns up, including the wine prefixes this
/// app is expected to be pointed at.
pub fn engine_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut push = |p: PathBuf| {
        if !out.contains(&p) {
            out.push(p);
        }
    };
    // the reference engine's private Wine prefix (oracle/run.sh installs
    // voices there), then the earlier extraction
    if let Some(dir) = super::oracle::oracle_dir() {
        push(dir.join("prefix/drive_c/Program Files (x86)/Loquendo/LTTS7"));
    }
    push(PathBuf::from("/tmp/loq-ev/wp/drive_c/Program Files (x86)/Loquendo/LTTS7"));
    if let Ok(home) = std::env::var("HOME") {
        for rel in [
            ".wine/drive_c/Program Files (x86)/Loquendo/LTTS7",
            ".wine/drive_c/Program Files/Loquendo/LTTS7",
            ".local/share/wineprefixes/default/drive_c/Program Files (x86)/Loquendo/LTTS7",
        ] {
            push(PathBuf::from(&home).join(rel));
        }
    }
    for p in [
        "/usr/lib/loquendo/LTTS7",
        "/opt/loquendo/LTTS7",
        "/Applications/Loquendo/LTTS7",
        "C:/Program Files (x86)/Loquendo/LTTS7",
    ] {
        push(PathBuf::from(p));
    }
    out
}

/// Find an installation without being told: the wine prefixes this app expects
/// first, then the usual install locations.
pub fn resolve_data_dir_auto() -> Option<PathBuf> {
    engine_candidates().iter().find_map(|c| resolve_data_dir(c))
}
