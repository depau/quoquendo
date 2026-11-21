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
                    lp.extend(v.util_library.clone());
                    lp.extend(v.engine_library.clone());
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

/// The mirror's files as a Markdown page (`--mirror-table`), for people who
/// want them outside the app: each voice and language folder of `catalog`
/// with every file linked on `base`, its size and sha256. Returns the file
/// count.
pub fn write_table(catalog: &Path, base: &str, out: &Path) -> Result<usize, String> {
    use crate::catalog::{archive_url, mirror_url, size_label};
    use std::fmt::Write;
    let text = std::fs::read(catalog).map_err(|e| format!("{}: {e}", catalog.display()))?;
    let doc: Value = serde_json::from_slice(&text).map_err(|e| format!("{}: {e}", catalog.display()))?;
    let base = base.trim_end_matches('/');
    let str_of = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let files_of = |v: &Value| v.get("files").and_then(Value::as_array).cloned().unwrap_or_default();
    let bytes_of = |fs: &[Value]| fs.iter().filter_map(|f| f.get("size").and_then(Value::as_u64)).sum::<u64>();
    let voices = doc.get("voices").and_then(Value::as_array).cloned().unwrap_or_default();
    let languages: Vec<(String, Value)> = doc
        .get("languages")
        .and_then(Value::as_object)
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    // GitHub's heading anchors: lower case, spaces as dashes
    let anchor = |s: &str| s.to_lowercase().replace(' ', "-");
    let file_rows = |md: &mut String, fs: &[Value]| {
        md.push_str("| File | Size | SHA-256 |\n|---|---:|---|\n");
        for f in fs {
            let path = str_of(f, "path");
            let size = f.get("size").and_then(Value::as_u64).unwrap_or(0);
            let _ = writeln!(md, "| [`{path}`]({}) | {} | `{}` |", mirror_url(base, &path), size_label(size), str_of(f, "sha256"));
        }
        md.push('\n');
    };

    let mut all: Vec<Value> = voices.iter().flat_map(&files_of).collect();
    all.extend(languages.iter().flat_map(|(_, l)| files_of(l)));
    all.sort_by_key(|f| str_of(f, "path"));
    all.dedup_by_key(|f| str_of(f, "path"));

    let mut md = String::new();
    let _ = writeln!(md, "# Mirrored files\n");
    let _ = writeln!(
        md,
        "Every file the voice mirror serves under `{base}/v1/`, as its [`catalog.json`]({base}/v1/catalog.json) lists \
         them: {} voices and {} language folders unpacked from Loquendo 7's installers and language distributions \
         (`quoquendo --unpack`), {} files, {} in all. The mirror workflow writes this page (`quoquendo --mirror-table`); \
         don't edit it by hand.\n",
        voices.len(),
        languages.len(),
        all.len(),
        size_label(bytes_of(&all))
    );
    let _ = writeln!(
        md,
        "Each path is relative to an LTTS7 folder: a voice needs its own files and its language folder's, placed as \
         the paths say (`curl --create-dirs -o \"LTTS7/<path>\" <link>`). The installers they came from are linked \
         from each voice.\n"
    );
    md.push_str("## Voices\n\n| Voice | Language | Installer | Files | Size |\n|---|---|---|---:|---:|\n");
    for v in &voices {
        let (name, folder, installer) = (str_of(v, "name"), str_of(v, "language_folder"), str_of(v, "installer"));
        let fs = files_of(v);
        let tongue = str_of(v, "mother_tongue");
        let language = if folder.is_empty() { tongue } else { format!("[{tongue}](#{})", anchor(&folder)) };
        let _ = writeln!(
            md,
            "| [{name}](#{}) | {language} | [{installer}]({}) | {} | {} |",
            anchor(&name),
            archive_url(&installer),
            fs.len(),
            size_label(bytes_of(&fs))
        );
    }
    md.push('\n');
    for v in &voices {
        let _ = writeln!(md, "### {}\n", str_of(v, "name"));
        file_rows(&mut md, &files_of(v));
    }
    md.push_str("## Languages\n\n");
    for (name, l) in &languages {
        let _ = writeln!(md, "### {name}\n");
        file_rows(&mut md, &files_of(l));
    }
    std::fs::write(out, md).map_err(|e| format!("{}: {e}", out.display()))?;
    Ok(all.len())
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
