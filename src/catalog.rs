//! Where voices come from: the pack's installers (`quoquendo::catalog`) and
//! the voice mirror, a copy of every installer's unpacked files behind a
//! CORS-friendly Worker (mirror/README.md).

use serde::Deserialize;
use std::collections::HashMap;

pub use quoquendo::catalog::{archive_url, installer_for, unsupported, Installer, INSTALLERS};

/// The mirror built in (`QQ_MIRROR_URL` at build time).
pub const DEFAULT_MIRROR: Option<&str> = option_env!("QQ_MIRROR_URL");

/// The mirror in use: the one set in the app, else the built-in one; in the
/// browser, else the page's own site (the Worker serves the app and the
/// voices from one origin, `mirror/worker`).
pub fn effective_mirror(setting: &str) -> Option<String> {
    let s = setting.trim();
    let m = if s.is_empty() { DEFAULT_MIRROR.unwrap_or("").trim() } else { s };
    #[cfg(target_arch = "wasm32")]
    let m = match m {
        "" => web_sys::window().and_then(|w| w.location().origin().ok()).unwrap_or_default(),
        m => m.to_string(),
    };
    (!m.is_empty()).then(|| m.trim_end_matches('/').to_string())
}

/// `v1/catalog.json` on the mirror.
#[derive(Clone, Debug, Deserialize)]
pub struct MirrorCatalog {
    pub voices: Vec<MirrorVoice>,
    pub languages: HashMap<String, MirrorLanguage>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MirrorVoice {
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub name: String,
    pub installer: String,
    #[serde(default)]
    pub language_folder: Option<String>,
    pub files: Vec<MirrorFile>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MirrorLanguage {
    pub files: Vec<MirrorFile>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MirrorFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

impl MirrorCatalog {
    /// Every file the voices of `installer` need (theirs and their
    /// languages'), or None when the mirror doesn't have that installer.
    pub fn files_for(&self, installer: &str) -> Option<Vec<MirrorFile>> {
        let voices: Vec<&MirrorVoice> = self.voices.iter().filter(|v| v.installer == installer).collect();
        if voices.is_empty() {
            return None;
        }
        let mut out: Vec<MirrorFile> = Vec::new();
        for v in voices {
            out.extend(v.files.iter().cloned());
            if let Some(lang) = v.language_folder.as_ref().and_then(|l| self.languages.get(l)) {
                out.extend(lang.files.iter().cloned());
            }
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out.dedup_by(|a, b| a.path == b.path);
        Some(out)
    }
}

/// The URL of a file on the mirror.
pub fn mirror_url(mirror: &str, path: &str) -> String {
    format!("{mirror}/v1/{}", encode_path(path))
}

/// Where to fetch an installer. archive.org sends no CORS header for `.exe`
/// files, so a browser goes through the mirror's proxy of it.
pub fn installer_url(file: &str, mirror: Option<&str>) -> Option<String> {
    if cfg!(target_arch = "wasm32") {
        mirror.map(|m| format!("{m}/v1/archive/{}", encode_path(file)))
    } else {
        Some(archive_url(file))
    }
}

fn encode_path(path: &str) -> String {
    let mut out = String::new();
    for b in path.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Human-readable byte count.
pub fn size_label(bytes: u64) -> String {
    match bytes {
        b if b >= 1 << 30 => format!("{:.1} GB", b as f64 / (1u64 << 30) as f64),
        b if b >= 1 << 20 => format!("{:.1} MB", b as f64 / (1u64 << 20) as f64),
        b if b >= 1 << 10 => format!("{:.0} kB", b as f64 / 1024.0),
        b => format!("{b} B"),
    }
}

/// Lowercase hex SHA-256 of `data`.
pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect()
}
