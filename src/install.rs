//! The app's own voice folder, filled from Loquendo's installers (native).

use quoquendo::loquendo::resolve_data_dir;
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

/// The `--store` folder, when one was given.
pub fn store_override() -> Option<PathBuf> {
    STORE.get().cloned()
}

/// A voice installer, or a folder of them (`<Voice> - <Language>.exe`).
pub fn is_installer_path(path: &Path) -> bool {
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
    let file = path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
    unpack_bytes(&file, exe, root).map_err(|e| format!("{}: {e}", path.display()))
}

/// `unpack` for an installer's bytes (`file` is its name).
pub fn unpack_bytes(file: &str, exe: Vec<u8>, root: &Path) -> Result<Vec<String>, String> {
    let unpacked = quoquendo::pack::unpack_installer(&exe)?;
    drop(exe);
    write_files(root, &unpacked.files)?;
    record_installer(root, file, &unpacked.voices)?;
    Ok(unpacked.voices)
}

fn io(p: &Path, e: std::io::Error) -> String {
    format!("{}: {e}", p.display())
}

/// Write files given relative to the LTTS7 tree at `root`.
pub fn write_files(root: &Path, files: &[(String, Vec<u8>)]) -> Result<(), String> {
    for (rel, data) in files {
        let dst = root.join(rel);
        fs::create_dir_all(dst.parent().unwrap()).map_err(|e| io(&dst, e))?;
        fs::write(&dst, data).map_err(|e| io(&dst, e))?;
    }
    Ok(())
}

/// Note in `<root>/installers.json` that `file` brings `voices` (an empty
/// `file` forgets them).
pub fn record_installer(root: &Path, file: &str, voices: &[String]) -> Result<(), String> {
    let index = root.join("installers.json");
    let mut map: serde_json::Map<String, serde_json::Value> =
        fs::read(&index).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    for v in voices {
        if file.is_empty() {
            map.remove(v);
        } else {
            map.insert(v.clone(), file.into());
        }
    }
    let body = serde_json::to_vec_pretty(&map).map_err(|e| e.to_string())?;
    fs::create_dir_all(root).map_err(|e| io(root, e))?;
    fs::write(&index, body).map_err(|e| io(&index, e))
}

/// Take voices straight from Loquendo's own installers into `voice_store()`.
pub fn install_voice(path: &Path) -> Result<Vec<String>, String> {
    let store = voice_store().ok_or("no folder to keep voices in (HOME is not set)")?;
    unpack(path, &store)
}
