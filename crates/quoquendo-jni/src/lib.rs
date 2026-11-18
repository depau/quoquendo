//! JNI bindings for the Android app: the static methods of the Kotlin
//! `object eu.depau.quoquendo.Native` (`System.loadLibrary("quoquendo_jni")`).
//!
//! A voice store is an LTTS7 root (`data/…`, `bin/…`) on the device. Every
//! export turns errors and panics into a `java.lang.RuntimeException`.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use jni::objects::{JClass, JString};
use jni::sys::{jint, jlong, jshortArray, jstring};
use jni::JNIEnv;
use quoquendo::{catalog, synth, Voice, VoiceFiles};
use serde_json::{json, Value};

/// Run `f`, turning an `Err` or a panic into a thrown `RuntimeException` and
/// `fallback` as the return value.
fn guard<'l, T>(env: &mut JNIEnv<'l>, fallback: T, f: impl FnOnce(&mut JNIEnv<'l>) -> Result<T, String>) -> T {
    let outcome = catch_unwind(AssertUnwindSafe(|| f(env)));
    let message = match outcome {
        Ok(Ok(v)) => return v,
        Ok(Err(e)) => e,
        Err(panic) => {
            let what = panic
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".into());
            format!("quoquendo panicked: {what}")
        }
    };
    // a pending Java exception (from a failed JNI call) takes precedence
    if !env.exception_check().unwrap_or(false) {
        let _ = env.throw_new("java/lang/RuntimeException", message);
    }
    fallback
}

fn jstr(env: &mut JNIEnv, s: &JString) -> Result<String, String> {
    env.get_string(s).map(|s| s.into()).map_err(|e| e.to_string())
}

fn ret_string(env: &mut JNIEnv, s: String) -> Result<jstring, String> {
    env.new_string(s).map(|s| s.into_raw()).map_err(|e| e.to_string())
}

/// `read`/`list` closures over an LTTS7 root on disk.
struct Store(PathBuf);

impl Store {
    fn read(&self, rel: &str) -> Option<Vec<u8>> {
        std::fs::read(self.0.join(rel)).ok()
    }
    fn list(&self, rel: &str) -> Vec<String> {
        std::fs::read_dir(self.0.join(rel))
            .map(|r| r.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect())
            .unwrap_or_default()
    }
}

/// A relative LTTS7 path that stays inside the store.
fn safe_rel(rel: &str) -> Result<&Path, String> {
    let p = Path::new(rel);
    if p.is_absolute() || p.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
        return Err(format!("{rel}: refusing a path outside the voice store"));
    }
    Ok(p)
}

fn unpack_installer(installer: &str, store: &str) -> Result<String, String> {
    let exe = std::fs::read(installer).map_err(|e| format!("{installer}: {e}"))?;
    let unpacked = quoquendo::pack::unpack_installer(&exe)?;
    drop(exe);
    let root = Path::new(store);
    for (rel, bytes) in &unpacked.files {
        let path = root.join(safe_rel(rel)?);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(json!({ "voices": unpacked.voices }).to_string())
}

fn list_voices(store: &str) -> String {
    let s = Store(PathBuf::from(store));
    let voices: Vec<Value> = quoquendo::list_voices(&|p| s.read(p), &|p| s.list(p))
        .into_iter()
        .map(|v| {
            let d = &v.descriptor;
            let locales: Vec<&str> = d
                .mother_tongue_aliases
                .split(',')
                .map(str::trim)
                .filter(|a| a.contains('-'))
                .collect();
            json!({
                "name": d.name,
                "description": d.description,
                "motherTongue": d.mother_tongue,
                "locales": locales,
                "gender": d.gender,
                "rate": d.audio_rate,
                "languageMissing": v.language_missing.is_some(),
                "unsupported": v.unsupported,
            })
        })
        .collect();
    Value::Array(voices).to_string()
}

fn catalog_json() -> String {
    let all: Vec<Value> = catalog::INSTALLERS
        .iter()
        .map(|i| {
            json!({
                "file": i.file,
                "label": i.label,
                "language": i.language,
                "voice": i.voice,
                "gender": i.gender,
                "size": i.size,
                "archiveUrl": catalog::archive_url(i.file),
            })
        })
        .collect();
    Value::Array(all).to_string()
}

fn load_voice(store: &str, name: &str) -> Result<Voice, String> {
    let s = Store(PathBuf::from(store));
    Voice::from_bytes(VoiceFiles::from_layout(name, &|p| s.read(p), &|p| s.list(p))?)
}

fn delete_voice(store: &str, name: &str) -> Result<(), String> {
    let s = Store(PathBuf::from(store));
    let info = quoquendo::inspect(name, &|p| s.read(p), &|p| s.list(p))?;
    let rm = |rel: &str, dir: bool| -> Result<(), String> {
        let path = s.0.join(safe_rel(rel)?);
        let r = if dir { std::fs::remove_dir_all(&path) } else { std::fs::remove_file(&path) };
        match r {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(format!("{}: {e}", path.display())),
            _ => Ok(()),
        }
    };
    rm(&info.dir, true)?;
    if let Some(lib) = &info.library {
        rm(lib, false)?;
    }
    rm(&info.vcf, false)
}

/// Samples in [-1, 1] to 16-bit PCM, rounded and clamped.
fn to_pcm16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|s| (s * 32768.0).round().clamp(i16::MIN as f32, i16::MAX as f32) as i16)
        .collect()
}

fn analyse(voice: &Voice, text: &str) -> String {
    let db = &voice.db;
    let targets = voice.transcribe(text);
    let phrases: Vec<Value> = targets
        .iter()
        .map(|p| {
            let phones: Vec<Value> = p
                .phones
                .iter()
                .map(|t| json!({ "sym": t.sym, "known": db.knows_symbol(&t.sym), "wordStart": t.word_start }))
                .collect();
            json!({ "phones": phones, "pauseMs": p.pause_ms, "mood": p.mood.map(String::from) })
        })
        .collect();
    let plan = synth::select_all(db, &targets);
    let rate = db.audio_rate().max(1) as f32;
    let mut pieces = Vec::new();
    for phrase in &plan {
        for p in &phrase.pieces {
            let sentence = db
                .units
                .get(p.unit as usize)
                .and_then(|u| u.symbols.get(1..u.symbols.len().saturating_sub(1)))
                .map(|s| s.join(" "))
                .unwrap_or_default();
            pieces.push(json!({
                "covers": p.covers,
                "unit": p.unit,
                "sentence": sentence,
                "atS": p.from as f32 / rate,
                "lenMs": p.to.saturating_sub(p.from) as f32 * 1000.0 / rate,
                "pitch": p.pitch,
                "hard": p.hard,
            }));
        }
    }
    let missing: Vec<&String> = plan.iter().flat_map(|p| &p.missing).collect();
    let envelope = synth::render(&plan, db, synth::Settings::default())
        .map(|s| synth::envelope(&s, 256))
        .unwrap_or_default();
    json!({ "phrases": phrases, "pieces": pieces, "missing": missing, "envelope": envelope }).to_string()
}

/// The voice behind a handle from `loadVoice`.
fn voice<'a>(handle: jlong) -> Result<&'a Voice, String> {
    if handle == 0 {
        return Err("no voice loaded".into());
    }
    // SAFETY: handles come from `loadVoice` (Box::into_raw) and live until `freeVoice`
    Ok(unsafe { &*(handle as *const Voice) })
}

#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_unpackInstaller<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    installer: JString<'l>,
    store: JString<'l>,
) -> jstring {
    guard(&mut env, std::ptr::null_mut(), |env| {
        let (installer, store) = (jstr(env, &installer)?, jstr(env, &store)?);
        let out = unpack_installer(&installer, &store)?;
        ret_string(env, out)
    })
}

#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_listVoices<'l>(mut env: JNIEnv<'l>, _class: JClass<'l>, store: JString<'l>) -> jstring {
    guard(&mut env, std::ptr::null_mut(), |env| {
        let store = jstr(env, &store)?;
        ret_string(env, list_voices(&store))
    })
}

#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_catalog<'l>(mut env: JNIEnv<'l>, _class: JClass<'l>) -> jstring {
    guard(&mut env, std::ptr::null_mut(), |env| ret_string(env, catalog_json()))
}

#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_loadVoice<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    store: JString<'l>,
    name: JString<'l>,
) -> jlong {
    guard(&mut env, 0, |env| {
        let (store, name) = (jstr(env, &store)?, jstr(env, &name)?);
        Ok(Box::into_raw(Box::new(load_voice(&store, &name)?)) as jlong)
    })
}

#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_freeVoice<'l>(mut env: JNIEnv<'l>, _class: JClass<'l>, handle: jlong) {
    guard(&mut env, (), |_| {
        if handle != 0 {
            // SAFETY: see `voice`; the caller frees each handle once
            drop(unsafe { Box::from_raw(handle as *mut Voice) });
        }
        Ok(())
    })
}

#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_sampleRate<'l>(mut env: JNIEnv<'l>, _class: JClass<'l>, handle: jlong) -> jint {
    guard(&mut env, 0, |_| Ok(voice(handle)?.db.audio_rate() as jint))
}

#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_synthesize<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    handle: jlong,
    text: JString<'l>,
) -> jshortArray {
    guard(&mut env, std::ptr::null_mut(), |env| {
        let text = jstr(env, &text)?;
        let speech = voice(handle)?.say(&text)?;
        let pcm = to_pcm16(&speech.samples);
        let array = env.new_short_array(pcm.len() as i32).map_err(|e| e.to_string())?;
        env.set_short_array_region(&array, 0, &pcm).map_err(|e| e.to_string())?;
        Ok(array.into_raw())
    })
}

#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_deleteVoice<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    store: JString<'l>,
    name: JString<'l>,
) {
    guard(&mut env, (), |env| {
        let (store, name) = (jstr(env, &store)?, jstr(env, &name)?);
        delete_voice(&store, &name)
    })
}

#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_analyse<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    handle: jlong,
    text: JString<'l>,
) -> jstring {
    guard(&mut env, std::ptr::null_mut(), |env| {
        let text = jstr(env, &text)?;
        let out = analyse(voice(handle)?, &text);
        ret_string(env, out)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_rounds_and_clamps() {
        assert_eq!(to_pcm16(&[0.0, 1.0, -1.0, 2.0, 0.5 / 32768.0, -0.6 / 32768.0]), vec![0, 32767, -32768, 32767, 1, -1]);
    }

    #[test]
    fn store_paths_stay_inside() {
        assert!(safe_rel("data/voices/Roberto/Roberto.sde").is_ok());
        assert!(safe_rel("../etc/passwd").is_err());
        assert!(safe_rel("/etc/passwd").is_err());
    }

    #[test]
    fn catalog_has_urls() {
        let v: Value = serde_json::from_str(&catalog_json()).unwrap();
        assert_eq!(v.as_array().unwrap().len(), catalog::INSTALLERS.len());
        assert!(v[0]["archiveUrl"].as_str().unwrap().starts_with("https://archive.org/"));
    }
}
