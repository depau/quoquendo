//! The voice library: voices kept by the app, laid out as an LTTS7 tree
//! (`data/…`, `bin/…`), listed, loaded, imported from installers,
//! downloaded and deleted. On the desktop the tree is a folder
//! (`install::voice_store`) and work runs on threads; in the browser it is
//! the origin-private file system (`quoquendo/LTTS7/…`, with an index
//! `quoquendo/index.json`) and work runs as tasks on the page's thread.
//! Results come back over a channel the app polls every frame.

use crate::catalog::{self, Installer, MirrorCatalog, MirrorFile};

/// Where store work reports back: the app's channel, and its window to
/// wake up (nothing else redraws it while it waits).
#[derive(Clone)]
pub struct Tx {
    tx: std::sync::mpsc::Sender<StoreEvent>,
    ctx: eframe::egui::Context,
}

impl Tx {
    pub fn new(tx: std::sync::mpsc::Sender<StoreEvent>, ctx: eframe::egui::Context) -> Tx {
        Tx { tx, ctx }
    }

    pub fn send(&self, e: StoreEvent) -> Result<(), std::sync::mpsc::SendError<StoreEvent>> {
        let r = self.tx.send(e);
        self.ctx.request_repaint();
        r
    }
}

pub enum StoreEvent {
    Progress { task: String, done: u64, total: Option<u64> },
    Listed(Vec<InstalledVoice>),
    Loaded(Box<quoquendo::Voice>),
    /// an installer's bytes, ready to unpack (`import_installer`)
    Fetched { file: String, bytes: Vec<u8> },
    Installed(Vec<String>),
    Deleted(String),
    Failed(String),
    /// a download, import or unpack failed
    InstallFailed(String),
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct InstalledVoice {
    pub name: String,
    pub description: String,
    pub mother_tongue: String,
    pub gender: String,
    /// the MotherTongue whose language data no stored installer brought
    pub language_missing: Option<String>,
    /// why it can't speak yet (`codec LTTS7CodecMsx`, `no Catalan module`)
    pub unsupported: Option<String>,
    /// bytes of its own files (its language's are shared)
    pub bytes: u64,
    /// another LTTS7 tree it lives in (developer mode), not the store
    #[serde(skip)]
    pub root: Option<String>,
}

impl InstalledVoice {
    fn from_info(info: &quoquendo::VoiceInfo, bytes: u64, root: Option<String>) -> InstalledVoice {
        let d = &info.descriptor;
        InstalledVoice {
            name: d.name.clone(),
            description: quoquendo::loquendo::from_latin1(&d.description),
            mother_tongue: d.mother_tongue.clone(),
            gender: d.gender.clone(),
            language_missing: info.language_missing.clone(),
            unsupported: info.unsupported.clone(),
            bytes,
            root,
        }
    }

    pub fn can_speak(&self) -> bool {
        self.unsupported.is_none()
    }
}

/// The paths of a voice's own files: its `.vcf`, its folder and its library.
fn own_files(info: &quoquendo::VoiceInfo, list: &dyn Fn(&str) -> Vec<String>) -> Vec<String> {
    let mut out = vec![info.vcf.clone()];
    out.extend(list(&info.dir).into_iter().map(|n| format!("{}/{n}", info.dir)));
    out.extend(info.library.clone());
    out
}

/// The paths of a language folder's files, and its module.
#[cfg(target_arch = "wasm32")]
fn language_files(info: &quoquendo::VoiceInfo, list: &dyn Fn(&str) -> Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = info
        .language_dir()
        .map(|d| list(d).into_iter().map(|n| format!("{d}/{n}")).collect())
        .unwrap_or_default();
    out.extend(info.language_library.clone());
    out
}

/// Which mirror files are still missing, given what is stored.
fn missing(files: Vec<MirrorFile>, have: impl Fn(&MirrorFile) -> bool) -> Vec<MirrorFile> {
    files.into_iter().filter(|f| !have(f)).collect()
}

fn check(f: &MirrorFile, data: &[u8]) -> Result<(), String> {
    if data.len() as u64 != f.size || catalog::sha256_hex(data) != f.sha256 {
        return Err(format!("{}: checksum mismatch", f.path));
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;
    use crate::install;
    use std::io::Read;
    use std::path::{Path, PathBuf};

    fn store_root() -> Result<PathBuf, String> {
        install::voice_store().ok_or_else(|| "no folder to keep voices in (HOME is not set)".to_string())
    }

    fn reader(root: &Path) -> impl Fn(&str) -> Option<Vec<u8>> + '_ {
        move |rel| std::fs::read(root.join(rel)).ok()
    }

    fn lister(root: &Path) -> impl Fn(&str) -> Vec<String> + '_ {
        move |rel| {
            std::fs::read_dir(root.join(rel))
                .map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect())
                .unwrap_or_default()
        }
    }

    fn voices_in(root: &Path, mark: Option<String>) -> Vec<InstalledVoice> {
        let (read, list) = (reader(root), lister(root));
        quoquendo::list_voices(&read, &list)
            .iter()
            .map(|info| {
                let bytes = own_files(info, &list)
                    .iter()
                    .filter_map(|p| std::fs::metadata(root.join(p)).ok())
                    .map(|m| m.len())
                    .sum();
                InstalledVoice::from_info(info, bytes, mark.clone())
            })
            .collect()
    }

    /// The stored voices, and with `others` (developer mode) those of the
    /// other LTTS7 trees found (`dev::engines`).
    pub fn list(tx: Tx, others: Vec<PathBuf>) {
        std::thread::spawn(move || {
            let mut out = store_root().map(|r| voices_in(&r, None)).unwrap_or_default();
            for root in others {
                for v in voices_in(&root, Some(root.display().to_string())) {
                    if !out.iter().any(|o| o.name.eq_ignore_ascii_case(&v.name)) {
                        out.push(v);
                    }
                }
            }
            let _ = tx.send(StoreEvent::Listed(out));
        });
    }

    pub fn load(voice: &InstalledVoice, tx: Tx) {
        let (name, root) = (voice.name.clone(), voice.root.clone());
        std::thread::spawn(move || {
            let root = match root {
                Some(r) => PathBuf::from(r),
                None => match store_root() {
                    Ok(r) => r,
                    Err(e) => return drop(tx.send(StoreEvent::Failed(e))),
                },
            };
            let _ = tx.send(match quoquendo::Voice::find(&root.join("data"), &name) {
                Ok(v) => StoreEvent::Loaded(Box::new(v)),
                Err(e) => StoreEvent::Failed(e),
            });
        });
    }

    /// Remove a stored voice: its `.vcf`, folder and library. Language
    /// folders stay (other voices may use them).
    pub fn delete(name: String, tx: Tx) {
        std::thread::spawn(move || {
            let run = || -> Result<(), String> {
                let root = store_root()?;
                let (read, list) = (reader(&root), lister(&root));
                let info = quoquendo::inspect(&name, &read, &list)?;
                let _ = std::fs::remove_dir_all(root.join(&info.dir));
                let _ = std::fs::remove_file(root.join(&info.vcf));
                if let Some(lib) = &info.library {
                    let _ = std::fs::remove_file(root.join(lib));
                }
                install::record_installer(&root, "", std::slice::from_ref(&info.descriptor.name))
            };
            let _ = tx.send(match run() {
                Ok(()) => StoreEvent::Deleted(name),
                Err(e) => StoreEvent::Failed(e),
            });
        });
    }

    pub fn import_installer(file: String, bytes: Vec<u8>, tx: Tx) {
        std::thread::spawn(move || {
            let run = || install::unpack_bytes(&file, bytes, &store_root()?);
            let _ = tx.send(match run() {
                Ok(v) => StoreEvent::Installed(v),
                Err(e) => StoreEvent::InstallFailed(format!("{file}: {e}")),
            });
        });
    }

    fn get(url: &str, task: &str, tx: &Tx) -> Result<Vec<u8>, String> {
        let resp = ureq::get(url).call().map_err(|e| format!("{url}: {e}"))?;
        let total = resp.header("Content-Length").and_then(|l| l.parse::<u64>().ok());
        let mut body = resp.into_reader();
        let mut out = Vec::with_capacity(total.unwrap_or(0) as usize);
        let mut buf = vec![0u8; 256 * 1024];
        loop {
            let n = body.read(&mut buf).map_err(|e| format!("{url}: {e}"))?;
            if n == 0 {
                break;
            }
            out.extend_from_slice(&buf[..n]);
            let _ = tx.send(StoreEvent::Progress {
                task: task.to_string(),
                done: out.len() as u64,
                total,
            });
        }
        Ok(out)
    }

    /// From the mirror: Ok(None) when it can't serve this installer.
    fn from_mirror(mirror: &str, inst: &Installer, tx: &Tx) -> Result<Option<Vec<String>>, String> {
        let Ok(cat) = get(&format!("{mirror}/v1/catalog.json"), inst.label, tx) else { return Ok(None) };
        let Ok(cat) = serde_json::from_slice::<MirrorCatalog>(&cat) else { return Ok(None) };
        let Some(files) = cat.files_for(inst.file) else { return Ok(None) };
        let root = store_root()?;
        let need = missing(files, |f| std::fs::metadata(root.join(&f.path)).is_ok_and(|m| m.len() == f.size));
        let total: u64 = need.iter().map(|f| f.size).sum();
        let mut done = 0;
        for f in &need {
            let Ok(data) = get(&catalog::mirror_url(mirror, &f.path), inst.label, tx) else { return Ok(None) };
            check(f, &data)?;
            install::write_files(&root, &[(f.path.clone(), data)])?;
            done += f.size;
            let _ = tx.send(StoreEvent::Progress {
                task: inst.label.to_string(),
                done,
                total: Some(total),
            });
        }
        let voices: Vec<String> = cat.voices.iter().filter(|v| v.installer == inst.file).map(|v| v.name.clone()).collect();
        install::record_installer(&root, inst.file, &voices)?;
        Ok(Some(voices))
    }

    /// Download an installer's voices: their files from the mirror when it
    /// has them, else the installer itself from archive.org (then unpacked
    /// with `import_installer`).
    pub fn download(inst: &'static Installer, mirror: Option<String>, tx: Tx) {
        std::thread::spawn(move || {
            if let Some(m) = mirror {
                match from_mirror(&m, inst, &tx) {
                    Ok(Some(voices)) => return drop(tx.send(StoreEvent::Installed(voices))),
                    Ok(None) => {}
                    Err(e) => return drop(tx.send(StoreEvent::InstallFailed(e))),
                }
            }
            let url = catalog::installer_url(inst.file, None).unwrap_or_default();
            let _ = tx.send(match get(&url, inst.label, &tx) {
                Ok(bytes) => StoreEvent::Fetched {
                    file: inst.file.to_string(),
                    bytes,
                },
                Err(e) => StoreEvent::InstallFailed(e),
            });
        });
    }
}

#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(target_arch = "wasm32")]
mod web {
    use super::*;
    use std::collections::HashMap;
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::{spawn_local, JsFuture};
    use web_sys::{FileSystemDirectoryHandle, FileSystemFileHandle};

    fn js(e: JsValue) -> String {
        e.as_string()
            .or_else(|| e.dyn_ref::<js_sys::Error>().map(|e| String::from(e.message())))
            .unwrap_or_else(|| format!("{e:?}"))
    }

    /// `index.json`: what each stored voice and language consists of.
    #[derive(Default, serde::Serialize, serde::Deserialize)]
    struct Index {
        voices: Vec<IndexVoice>,
        /// language folder → its files
        languages: HashMap<String, Vec<String>>,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct IndexVoice {
        info: InstalledVoice,
        files: Vec<String>,
        language: Option<String>,
    }

    async fn base() -> Result<FileSystemDirectoryHandle, String> {
        let storage = web_sys::window().ok_or("no window")?.navigator().storage();
        let root: FileSystemDirectoryHandle = JsFuture::from(storage.get_directory()).await.map_err(js)?.unchecked_into();
        sub(&root, "quoquendo", true).await
    }

    async fn sub(dir: &FileSystemDirectoryHandle, name: &str, create: bool) -> Result<FileSystemDirectoryHandle, String> {
        let o = web_sys::FileSystemGetDirectoryOptions::new();
        o.set_create(create);
        Ok(JsFuture::from(dir.get_directory_handle_with_options(name, &o)).await.map_err(js)?.unchecked_into())
    }

    /// The folder holding `path` (under `quoquendo/`), and the file's name.
    async fn parent(path: &str, create: bool) -> Result<(FileSystemDirectoryHandle, String), String> {
        let mut dir = base().await?;
        let mut parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        let name = parts.pop().ok_or("empty path")?.to_string();
        for p in parts {
            dir = sub(&dir, p, create).await?;
        }
        Ok((dir, name))
    }

    async fn write(path: &str, data: &[u8]) -> Result<(), String> {
        let (dir, name) = parent(path, true).await?;
        let o = web_sys::FileSystemGetFileOptions::new();
        o.set_create(true);
        let file: FileSystemFileHandle = JsFuture::from(dir.get_file_handle_with_options(&name, &o)).await.map_err(js)?.unchecked_into();
        let w: web_sys::FileSystemWritableFileStream = JsFuture::from(file.create_writable()).await.map_err(js)?.unchecked_into();
        JsFuture::from(w.write_with_u8_array(data).map_err(js)?).await.map_err(js)?;
        JsFuture::from(w.close()).await.map_err(js)?;
        Ok(())
    }

    async fn read(path: &str) -> Option<Vec<u8>> {
        let (dir, name) = parent(path, false).await.ok()?;
        let file: FileSystemFileHandle = JsFuture::from(dir.get_file_handle(&name)).await.ok()?.unchecked_into();
        let file: web_sys::File = JsFuture::from(file.get_file()).await.ok()?.unchecked_into();
        let buf = JsFuture::from(file.array_buffer()).await.ok()?;
        Some(js_sys::Uint8Array::new(&buf).to_vec())
    }

    async fn remove(path: &str) {
        if let Ok((dir, name)) = parent(path, false).await {
            let _ = JsFuture::from(dir.remove_entry(&name)).await;
        }
    }

    async fn load_index() -> Index {
        match read("index.json").await {
            Some(b) => serde_json::from_slice(&b).unwrap_or_default(),
            None => Index::default(),
        }
    }

    async fn save_index(index: &Index) -> Result<(), String> {
        write("index.json", &serde_json::to_vec(index).map_err(|e| e.to_string())?).await
    }

    fn tree(path: &str) -> String {
        format!("LTTS7/{path}")
    }

    /// Recompute every stored voice's state once a language arrives or goes.
    async fn refresh(index: &mut Index) {
        let ldes: Vec<String> = index.languages.values().flatten().filter(|p| p.to_ascii_lowercase().ends_with(".lde")).cloned().collect();
        let mut files: HashMap<String, Vec<u8>> = HashMap::new();
        for p in &ldes {
            if let Some(b) = read(&tree(p)).await {
                files.insert(p.clone(), b);
            }
        }
        for v in index.voices.iter_mut() {
            let mut mine = files.clone();
            for p in v.files.iter().filter(|p| {
                let l = p.to_ascii_lowercase();
                l.ends_with(".vcf") || l.ends_with(".vde")
            }) {
                if let Some(b) = read(&tree(p)).await {
                    mine.insert(p.clone(), b);
                }
            }
            for p in &v.files {
                mine.entry(p.clone()).or_default();
            }
            for (_, lf) in index.languages.iter() {
                for p in lf {
                    mine.entry(p.clone()).or_default();
                }
            }
            if let Ok(info) = quoquendo::inspect(&v.info.name, &|p| mine.get(p).cloned(), &|d| list_map(&mine, d)) {
                v.info.language_missing = info.language_missing.clone();
                v.info.unsupported = info.unsupported.clone();
                v.language = info.language_dir().map(str::to_string);
            }
        }
    }

    fn list_map(files: &HashMap<String, Vec<u8>>, dir: &str) -> Vec<String> {
        let prefix = format!("{dir}/");
        let mut names: Vec<String> = files
            .keys()
            .filter_map(|k| k.strip_prefix(&prefix))
            .map(|rest| rest.split('/').next().unwrap_or(rest).to_string())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// Store an unpacked tree's files and note its voices and languages.
    async fn store(files: Vec<(String, Vec<u8>)>, tx: &Tx, task: &str) -> Result<Vec<String>, String> {
        if let Ok(storage) = web_sys::window().ok_or("no window").map(|w| w.navigator().storage()) {
            if let Ok(p) = storage.persist() {
                let _ = JsFuture::from(p).await;
            }
        }
        let map: HashMap<String, Vec<u8>> = files.iter().map(|(p, d)| (p.clone(), d.clone())).collect();
        let (read_map, list) = (|p: &str| map.get(p).cloned(), |d: &str| list_map(&map, d));
        let infos = quoquendo::list_voices(&read_map, &list);
        let total: u64 = files.iter().map(|(_, d)| d.len() as u64).sum();
        let mut done = 0;
        for (path, data) in &files {
            write(&tree(path), data).await?;
            done += data.len() as u64;
            let _ = tx.send(StoreEvent::Progress {
                task: task.to_string(),
                done,
                total: Some(total),
            });
        }
        let mut index = load_index().await;
        for info in &infos {
            if let Some(d) = info.language_dir() {
                let lf = language_files(info, &list);
                let entry = index.languages.entry(d.to_string()).or_default();
                entry.extend(lf);
                entry.sort();
                entry.dedup();
            }
        }
        for info in &infos {
            let own = own_files(info, &list);
            let bytes = own.iter().filter_map(|p| map.get(p)).map(|d| d.len() as u64).sum();
            index.voices.retain(|v| !v.info.name.eq_ignore_ascii_case(&info.descriptor.name));
            index.voices.push(IndexVoice {
                info: InstalledVoice::from_info(info, bytes, None),
                files: own,
                language: info.language_dir().map(str::to_string),
            });
        }
        index.voices.sort_by(|a, b| a.info.name.cmp(&b.info.name));
        refresh(&mut index).await;
        save_index(&index).await?;
        Ok(infos.iter().map(|i| i.descriptor.name.clone()).collect())
    }

    pub fn list(tx: Tx) {
        spawn_local(async move {
            let index = load_index().await;
            let _ = tx.send(StoreEvent::Listed(index.voices.into_iter().map(|v| v.info).collect()));
        });
    }

    pub fn load(voice: &InstalledVoice, tx: Tx) {
        let name = voice.name.clone();
        spawn_local(async move {
            let index = load_index().await;
            let Some(v) = index.voices.iter().find(|v| v.info.name.eq_ignore_ascii_case(&name)) else {
                return drop(tx.send(StoreEvent::Failed(format!("{name} is not stored"))));
            };
            // its files, every language's descriptor, and its language's files
            let mut want: Vec<&String> = v.files.iter().collect();
            want.extend(index.languages.values().flatten().filter(|p| p.to_ascii_lowercase().ends_with(".lde")));
            if let Some(l) = v.language.as_ref().and_then(|l| index.languages.get(l)) {
                want.extend(l.iter());
            }
            let mut files = HashMap::new();
            for p in want {
                if let Some(b) = read(&tree(p)).await {
                    files.insert(p.clone(), b);
                }
            }
            let result = quoquendo::VoiceFiles::from_map(&name, &files).and_then(quoquendo::Voice::from_bytes);
            let _ = tx.send(match result {
                Ok(v) => StoreEvent::Loaded(Box::new(v)),
                Err(e) => StoreEvent::Failed(e),
            });
        });
    }

    pub fn delete(name: String, tx: Tx) {
        spawn_local(async move {
            let mut index = load_index().await;
            if let Some(i) = index.voices.iter().position(|v| v.info.name.eq_ignore_ascii_case(&name)) {
                let v = index.voices.remove(i);
                for p in &v.files {
                    remove(&tree(p)).await;
                }
            }
            let _ = tx.send(match save_index(&index).await {
                Ok(()) => StoreEvent::Deleted(name),
                Err(e) => StoreEvent::Failed(e),
            });
        });
    }

    /// Unpack an installer in a Web Worker (`src/bin/qq_unpack_worker.rs`)
    /// so the page keeps drawing, then store it.
    async fn unpack_in_worker(bytes: Vec<u8>) -> Result<Vec<(String, Vec<u8>)>, String> {
        // the next message (or error) from the worker
        fn next(worker: &web_sys::Worker) -> js_sys::Promise {
            js_sys::Promise::new(&mut |resolve, reject| {
                let on_message = wasm_bindgen::closure::Closure::once_into_js(move |e: web_sys::MessageEvent| {
                    let _ = resolve.call1(&JsValue::NULL, &e.data());
                });
                let on_error = wasm_bindgen::closure::Closure::once_into_js(move |e: JsValue| {
                    let _ = reject.call1(&JsValue::NULL, &e);
                });
                worker.set_onmessage(Some(on_message.unchecked_ref()));
                worker.set_onerror(Some(on_error.unchecked_ref()));
            })
        }
        let worker = web_sys::Worker::new("./qq_unpack_worker_loader.js").map_err(js)?;
        let failed = |e: JsValue| format!("the unpacking worker failed: {}", js(e));
        // it says "ready" once its module has loaded and listens
        let ready = JsFuture::from(next(&worker)).await;
        if let Err(e) = ready {
            worker.terminate();
            return Err(failed(e));
        }
        let reply = next(&worker);
        let array = js_sys::Uint8Array::from(bytes.as_slice());
        drop(bytes);
        worker
            .post_message_with_transfer(&array, &js_sys::Array::of1(&array.buffer()))
            .map_err(js)?;
        let result = JsFuture::from(reply).await;
        worker.terminate();
        let msg = result.map_err(failed)?;
        if let Some(e) = js_sys::Reflect::get(&msg, &"error".into()).ok().and_then(|e| e.as_string()) {
            return Err(e);
        }
        let files: js_sys::Array = js_sys::Reflect::get(&msg, &"files".into()).map_err(js)?.unchecked_into();
        Ok(files
            .iter()
            .map(|pair| {
                let pair: js_sys::Array = pair.unchecked_into();
                (pair.get(0).as_string().unwrap_or_default(), js_sys::Uint8Array::new(&pair.get(1)).to_vec())
            })
            .collect())
    }

    pub fn import_installer(file: String, bytes: Vec<u8>, tx: Tx) {
        spawn_local(async move {
            let result = match unpack_in_worker(bytes).await {
                Ok(files) => store(files, &tx, &file).await,
                Err(e) => Err(e),
            };
            let _ = tx.send(match result {
                Ok(v) => StoreEvent::Installed(v),
                Err(e) => StoreEvent::InstallFailed(format!("{file}: {e}")),
            });
        });
    }

    /// `fetch` a URL whole, reporting progress.
    async fn get(url: &str, task: &str, tx: &Tx) -> Result<Vec<u8>, String> {
        let window = web_sys::window().ok_or("no window")?;
        let resp: web_sys::Response = JsFuture::from(window.fetch_with_str(url)).await.map_err(js)?.unchecked_into();
        if !resp.ok() {
            return Err(format!("{url}: HTTP {}", resp.status()));
        }
        let total = resp.headers().get("Content-Length").ok().flatten().and_then(|l| l.parse::<u64>().ok());
        let body = resp.body().ok_or("empty response")?;
        let reader: web_sys::ReadableStreamDefaultReader = body.get_reader().unchecked_into();
        let mut out = Vec::with_capacity(total.unwrap_or(0) as usize);
        loop {
            let chunk = JsFuture::from(reader.read()).await.map_err(js)?;
            let done = js_sys::Reflect::get(&chunk, &"done".into()).map(|d| d.is_truthy()).unwrap_or(true);
            if done {
                break;
            }
            let value = js_sys::Reflect::get(&chunk, &"value".into()).map_err(js)?;
            out.extend_from_slice(&js_sys::Uint8Array::new(&value).to_vec());
            let _ = tx.send(StoreEvent::Progress {
                task: task.to_string(),
                done: out.len() as u64,
                total,
            });
        }
        Ok(out)
    }

    async fn from_mirror(mirror: &str, inst: &Installer, tx: &Tx) -> Result<Option<Vec<String>>, String> {
        let Ok(cat) = get(&format!("{mirror}/v1/catalog.json"), inst.label, tx).await else { return Ok(None) };
        let Ok(cat) = serde_json::from_slice::<MirrorCatalog>(&cat) else { return Ok(None) };
        let Some(files) = cat.files_for(inst.file) else { return Ok(None) };
        let index = load_index().await;
        let stored: std::collections::HashSet<&String> = index.voices.iter().flat_map(|v| &v.files).chain(index.languages.values().flatten()).collect();
        let need = missing(files.clone(), |f| stored.contains(&f.path));
        let mut fetched = Vec::new();
        for f in &need {
            let Ok(data) = get(&catalog::mirror_url(mirror, &f.path), inst.label, tx).await else { return Ok(None) };
            check(f, &data)?;
            fetched.push((f.path.clone(), data));
        }
        // the whole set is stored together so the index knows each voice's
        // files and language; files already stored are read back for that
        for f in files.iter().filter(|f| stored.contains(&f.path)) {
            if let Some(d) = read(&tree(&f.path)).await {
                fetched.push((f.path.clone(), d));
            }
        }
        store(fetched, tx, inst.label).await.map(Some)
    }

    /// Download an installer's voices: their files from the mirror when it
    /// has them, else the installer through the mirror's archive.org proxy
    /// (archive.org itself sends no CORS header for `.exe` files).
    pub fn download(inst: &'static Installer, mirror: Option<String>, tx: Tx) {
        spawn_local(async move {
            if let Some(m) = &mirror {
                match from_mirror(m, inst, &tx).await {
                    Ok(Some(voices)) => return drop(tx.send(StoreEvent::Installed(voices))),
                    Ok(None) => {}
                    Err(e) => return drop(tx.send(StoreEvent::InstallFailed(e))),
                }
            }
            let Some(url) = catalog::installer_url(inst.file, mirror.as_deref()) else {
                return drop(tx.send(StoreEvent::InstallFailed(format!(
                    "no voice mirror is set, and browsers can't fetch {} from archive.org: download it yourself and import it",
                    inst.file
                ))));
            };
            let _ = tx.send(match get(&url, inst.label, &tx).await {
                Ok(bytes) => StoreEvent::Fetched {
                    file: inst.file.to_string(),
                    bytes,
                },
                Err(e) => StoreEvent::InstallFailed(e),
            });
        });
    }

    /// Ask for an installer file, then hand its bytes back as `Fetched`.
    pub fn pick_installer(tx: Tx) {
        spawn_local(async move {
            let Some(f) = rfd::AsyncFileDialog::new().add_filter("Loquendo voice installer", &["exe", "EXE"]).pick_file().await else {
                return;
            };
            let _ = tx.send(StoreEvent::Fetched {
                file: f.file_name(),
                bytes: f.read().await,
            });
        });
    }
}

/// Ask for an installer file, then hand its bytes back as `Fetched`.
#[cfg(not(target_arch = "wasm32"))]
pub fn pick_installer(tx: Tx) {
    let Some(path) = rfd::FileDialog::new().add_filter("Loquendo voice installer", &["exe", "EXE"]).pick_file() else {
        return;
    };
    std::thread::spawn(move || {
        let file = path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
        let _ = tx.send(match std::fs::read(&path) {
            Ok(bytes) => StoreEvent::Fetched { file, bytes },
            Err(e) => StoreEvent::InstallFailed(format!("{}: {e}", path.display())),
        });
    });
}
