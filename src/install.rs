//! Where the app finds voices: its own voice folder (filled from Loquendo's
//! installers), the reference engine's Wine prefix, the usual install places.

use quoquendo::loquendo::{discover_voices, resolve_data_dir, VoiceDescriptor};
use std::fs;
use std::path::{Path, PathBuf};

static STORE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// `--store <dir>`: keep and find voices in this LTTS7 root only.
pub fn set_store(dir: PathBuf) {
    let _ = STORE.set(dir);
}

/// Where voices unpacked from their installers are kept:
/// `<data home>/quoquendo/LTTS7` (`$XDG_DATA_HOME`, `%APPDATA%` or
/// `~/.local/share`), laid out like an installed engine (`data/`, `bin/`).
pub fn voice_store() -> Option<PathBuf> {
    if let Some(s) = STORE.get() {
        return Some(s.clone());
    }
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
    Some(base.join("quoquendo/LTTS7"))
}

/// A voice installer, or a folder of them (`<Voice> - <Language>.exe`).
pub fn is_installer_path(path: &Path) -> bool {
    let is_exe = |p: &Path| p.extension().map(|e| e.eq_ignore_ascii_case("exe")).unwrap_or(false);
    if path.is_file() {
        return is_exe(path);
    }
    path.is_dir() && resolve_data_dir(path).is_none() && fs::read_dir(path).map(|r| r.flatten().any(|e| is_exe(&e.path()))).unwrap_or(false)
}

fn is_exe(p: &Path) -> bool {
    p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"))
}

/// Unpack a Loquendo voice installer (Inno Setup or Wise), or every
/// installer in a folder, into the LTTS7 tree at `root` (`quoquendo::pack`:
/// read, never run). Records which installer brought each voice in
/// `<root>/installers.json`. Returns the voices unpacked.
pub fn unpack(path: &Path, root: &Path) -> Result<Vec<String>, String> {
    if path.is_dir() {
        let mut exes: Vec<PathBuf> = fs::read_dir(path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .flatten()
            .map(|e| e.path())
            .filter(|p| is_exe(p))
            .collect();
        exes.sort();
        let mut all = Vec::new();
        for exe in exes {
            match unpack(&exe, root) {
                Ok(v) => all.extend(v),
                Err(e) => eprintln!("{e}"),
            }
        }
        return Ok(all);
    }
    let exe = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let unpacked = quoquendo::pack::unpack_installer(&exe).map_err(|e| format!("{}: {e}", path.display()))?;
    drop(exe);
    let io = |p: &Path, e: std::io::Error| format!("{}: {e}", p.display());
    for (rel, data) in &unpacked.files {
        let dst = root.join(rel);
        fs::create_dir_all(dst.parent().unwrap()).map_err(|e| io(&dst, e))?;
        fs::write(&dst, data).map_err(|e| io(&dst, e))?;
    }
    let index = root.join("installers.json");
    let mut map: serde_json::Map<String, serde_json::Value> =
        fs::read(&index).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let file = path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
    for v in &unpacked.voices {
        map.insert(v.clone(), file.clone().into());
    }
    let body = serde_json::to_vec_pretty(&map).map_err(|e| e.to_string())?;
    fs::write(&index, body).map_err(|e| io(&index, e))?;
    Ok(unpacked.voices)
}

/// Take voices straight from Loquendo's own installers into `voice_store()`.
pub fn install_voice(path: &Path) -> Result<Vec<String>, String> {
    let store = voice_store().ok_or("no folder to keep voices in (HOME is not set)")?;
    unpack(path, &store)
}

/// Every voice the app can use: the ones taken from installers
/// (`voice_store()`) first, then any other installation found.
pub fn all_voices() -> Vec<VoiceDescriptor> {
    let mut out: Vec<VoiceDescriptor> = Vec::new();
    // QQ_DATA_DIR=<LTTS7 folder or its data folder>, or --store <LTTS7
    // folder>: only that installation
    let only = std::env::var_os("QQ_DATA_DIR").map(PathBuf::from).or_else(|| STORE.get().cloned());
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
    if let Some(dir) = crate::oracle::oracle_dir() {
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
    for cand in engine_candidates() {
        if let Some(dir) = resolve_data_dir(&cand) {
            return Some(dir);
        }
    }
    None
}
