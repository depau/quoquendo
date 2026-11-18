//! Where the app finds voices: its own voice folder (filled from Loquendo's
//! installers), the reference engine's Wine prefix, the usual install places.

use quoquendo::loquendo::{discover_voices, read_ini, resolve_data_dir, VoiceDescriptor};
use std::fs;
use std::path::{Path, PathBuf};

/// Where voices unpacked from their installers are kept:
/// `<data home>/quoquendo/LTTS7` (`$XDG_DATA_HOME`, `%APPDATA%` or
/// `~/.local/share`), laid out like an installed engine (`data/`, `bin/`).
pub fn voice_store() -> Option<PathBuf> {
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

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)?.flatten() {
        let (src, dst) = (e.path(), to.join(e.file_name()));
        if src.is_dir() {
            copy_tree(&src, &dst)?;
        } else {
            fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}

/// Take a voice straight from Loquendo's own installer (Inno Setup 5.2.3)
/// into `voice_store()`: its `.vcf`, its `voices/<Voice>/` folder (`.sde`,
/// recordings `.bin`, `.vde`, Gilded lists) and its library
/// `bin/LTTS7<Voice>.dll` (the phone table, `phoneset`). Nothing is run: the
/// installer is only read, by the `innoextract` program. A folder installs
/// every installer in it. Returns the voices installed.
pub fn install_voice(path: &Path) -> Result<Vec<String>, String> {
    if path.is_dir() {
        let mut all = Vec::new();
        let mut exes: Vec<PathBuf> = fs::read_dir(path).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).filter(|p| is_installer_path(p)).collect();
        exes.sort();
        for exe in exes {
            match install_voice(&exe) {
                Ok(v) => all.extend(v),
                Err(e) => eprintln!("{}: {e}", exe.display()),
            }
        }
        return Ok(all);
    }
    let store = voice_store().ok_or("no folder to keep voices in (HOME is not set)")?;
    let tmp = std::env::temp_dir().join(format!("quoquendo-install-{}", std::process::id()));
    let out = std::process::Command::new("innoextract")
        .args(["-s", "-d"])
        .arg(&tmp)
        .args(["-I", "data", "-I", "bin"])
        .arg(path)
        .output()
        .map_err(|e| format!("reading voice installers needs the innoextract program ({e})"))?;
    let result = (|| {
        if !out.status.success() {
            return Err(format!("innoextract: {}", String::from_utf8_lossy(&out.stderr).trim()));
        }
        let data = tmp.join("pf/Loquendo/LTTS7/data");
        let mut names = Vec::new();
        for e in fs::read_dir(&data).map_err(|_| "this installer holds no Loquendo voice".to_string())?.flatten() {
            let vcf = e.path();
            if vcf.extension().and_then(|x| x.to_str()) != Some("vcf") {
                continue;
            }
            let cv = read_ini(&vcf);
            let (Some(name), Some(rel)) = (cv.get("BaseVoice"), cv.get("VoicePath")) else { continue };
            let dir = data.join(rel);
            if !dir.is_dir() {
                continue;
            }
            let lib = quoquendo::phoneset::find_voice_library(&dir.join(format!("{name}.sde")), name);
            let io = |e: std::io::Error| e.to_string();
            copy_tree(&dir, &store.join("data").join(rel)).map_err(io)?;
            fs::copy(&vcf, store.join("data").join(e.file_name())).map_err(io)?;
            if let Some(lib) = lib {
                fs::create_dir_all(store.join("bin")).map_err(io)?;
                fs::copy(&lib, store.join("bin").join(lib.file_name().unwrap())).map_err(io)?;
            }
            // the voice's language folder (`languages/English/EnglishUs.dbl`,
            // `.gpr`, `.lde`…): the one holding `<MotherTongue>.lde`
            let tongue = VoiceDescriptor::from_files(&vcf, &dir.join(format!("{name}.vde"))).mother_tongue.to_lowercase();
            for lang in fs::read_dir(data.join("languages")).into_iter().flatten().flatten() {
                let has = fs::read_dir(lang.path()).into_iter().flatten().flatten().any(|f| f.file_name().to_string_lossy().to_lowercase() == format!("{tongue}.lde"));
                if has {
                    copy_tree(&lang.path(), &store.join("data/languages").join(lang.file_name())).map_err(io)?;
                }
            }
            names.push(name.clone());
        }
        if names.is_empty() {
            return Err("this installer holds no Loquendo voice".into());
        }
        Ok(names)
    })();
    let _ = fs::remove_dir_all(&tmp);
    result
}

/// Every voice the app can use: the ones taken from installers
/// (`voice_store()`) first, then any other installation found.
pub fn all_voices() -> Vec<VoiceDescriptor> {
    let mut out: Vec<VoiceDescriptor> = Vec::new();
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
    push(PathBuf::from(
        "/tmp/loq-ev/wp/drive_c/Program Files (x86)/Loquendo/LTTS7",
    ));
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

