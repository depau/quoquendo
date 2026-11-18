//! The voice mirror's `catalog.json` (`--mirror-catalog`): every voice and
//! language folder of an LTTS7 tree, with each file's size and sha256.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Writes the catalog of the LTTS7 tree `ltts7` to `out`; returns the voice count.
pub fn write_catalog(ltts7: &Path, out: &Path) -> Result<usize, String> {
    let read = |p: &str| std::fs::read(ltts7.join(p)).ok();
    // `list` gives every entry (inspect walks `data/languages/<Folder>`); `files` only files
    let entries_of = |d: &str, only_files: bool| -> Vec<String> {
        std::fs::read_dir(ltts7.join(d))
            .map(|it| {
                it.filter_map(|e| e.ok())
                    .filter(|e| !only_files || e.file_type().is_ok_and(|t| t.is_file()))
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect()
            })
            .unwrap_or_default()
    };
    let list = |d: &str| entries_of(d, false);
    let files = |d: &str| entries_of(d, true);
    let installers: HashMap<String, String> = std::fs::read(ltts7.join("installers.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let entry = |path: &str| -> Result<Value, String> {
        let bytes = std::fs::read(ltts7.join(path)).map_err(|e| format!("{path}: {e}"))?;
        let sha = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect::<String>();
        Ok(json!({"path": path, "size": bytes.len(), "sha256": sha}))
    };
    let entries = |mut paths: Vec<String>| -> Result<Vec<Value>, String> {
        paths.sort();
        paths.dedup();
        paths.iter().map(|p| entry(p)).collect()
    };

    let mut infos = quoquendo::list_voices(&read, &list);
    infos.sort_by(|a, b| a.descriptor.name.cmp(&b.descriptor.name));
    let mut voices = Vec::new();
    let mut languages = BTreeMap::new();
    for v in &infos {
        let d = &v.descriptor;
        let mut paths = vec![v.vcf.clone()];
        paths.extend(files(&v.dir).into_iter().map(|n| format!("{}/{n}", v.dir)));
        paths.extend(v.library.clone());
        let folder = match (&v.language_missing, v.language_dir()) {
            (None, Some(dir)) => {
                let name = dir.rsplit('/').next().unwrap_or(dir).to_string();
                if !languages.contains_key(&name) {
                    let mut lp: Vec<String> = files(dir).into_iter().map(|n| format!("{dir}/{n}")).collect();
                    lp.extend(v.language_library.clone());
                    languages.insert(name.clone(), json!({"files": entries(lp)?}));
                }
                Some(name)
            }
            _ => None,
        };
        let installer = installers
            .get(&d.name)
            .cloned()
            .or_else(|| quoquendo::catalog::installer_for(&d.name).map(|i| i.file.to_string()));
        voices.push(json!({
            "name": d.name,
            "installer": installer,
            "mother_tongue": d.mother_tongue,
            "language_folder": folder,
            "gender": d.gender,
            "description": d.description,
            "codec": d.bin_name.split('.').nth(2),
            "rate": d.audio_rate,
            "unsupported": v.unsupported,
            "files": entries(paths)?,
        }));
    }
    let n = voices.len();
    let doc = json!({"version": 1, "voices": voices, "languages": languages});
    let text = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    std::fs::write(out, text).map_err(|e| format!("{}: {e}", out.display()))?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    #[test]
    fn mirror_catalog_of_unpacked_pack() {
        let root = std::path::Path::new("/tmp/qqv/LTTS7");
        if !root.exists() {
            return;
        }
        let n = super::write_catalog(root, std::path::Path::new("/tmp/qq-catalog.json")).unwrap();
        assert_eq!(n, 47);
    }
}
