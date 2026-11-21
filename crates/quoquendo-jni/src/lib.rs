//! JNI bindings for the Android app: the static methods of the Kotlin
//! `object eu.depau.quoquendo.Native` (`System.loadLibrary("quoquendo_jni")`).
//!
//! A voice store is an LTTS7 root (`data/…`, `bin/…`) on the device. Every
//! export turns errors and panics into a `java.lang.RuntimeException`.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use jni::objects::{JClass, JShortArray, JString};
use jni::sys::{jbyteArray, jint, jlong, jshortArray, jstring};
use jni::JNIEnv;
use quoquendo::{catalog, codec, synth, Voice, VoiceFiles};
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
            // what deleting the voice removes
            let files: Vec<&String> = [Some(&v.vcf), Some(&v.dir), v.library.as_ref()].into_iter().flatten().collect();
            json!({
                "name": d.name,
                "description": d.description,
                "motherTongue": d.mother_tongue,
                "locales": locales,
                "gender": d.gender,
                "rate": d.audio_rate,
                "demoSentence": d.demo_sentence,
                "languageMissing": v.language_missing.is_some(),
                "unsupported": v.unsupported,
                "languageFolder": v.language_dir().and_then(|d| d.rsplit('/').next()),
                "files": files,
            })
        })
        .collect();
    Value::Array(voices).to_string()
}

/// Why an installer's voice can't speak yet, known from its name alone: its
/// language has no module in the pack (`Valencian Catalan` → `no Valencian module`).
fn catalog_unsupported(language: &str) -> Option<String> {
    language
        .split_whitespace()
        .find(|w| catalog::NO_MODULE.iter().any(|m| m.eq_ignore_ascii_case(w)))
        .map(|w| format!("no {w} module"))
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
                "unsupported": catalog_unsupported(i.language),
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

/// The recordings bank, or why there is none.
fn bank(voice: &Voice) -> Result<&codec::AudioBank, String> {
    let db = &voice.db;
    db.audio
        .as_ref()
        .ok_or_else(|| db.audio_error.clone().unwrap_or_else(|| "this voice's recordings were not found".into()))
}

/// The rate `synthesize` returns: banks at another rate are resampled to the
/// engine's output rate (`synth::render`).
fn output_rate(voice: &Voice) -> u32 {
    match &voice.db.audio {
        Some(_) => codec::OUTPUT_RATE,
        None => voice.db.audio_rate(),
    }
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
    // piece positions are samples of the recordings bank
    let rate = db.audio.as_ref().map_or(db.audio_rate(), |b| b.rate).max(1) as f32;
    let mut pieces = Vec::new();
    for (i, phrase) in plan.iter().enumerate() {
        for p in &phrase.pieces {
            let sentence = db
                .units
                .get(p.unit as usize)
                .and_then(|u| u.symbols.get(1..u.symbols.len().saturating_sub(1)))
                .map(|s| s.join(" "))
                .unwrap_or_default();
            pieces.push(json!({
                "phrase": i,
                "covers": p.covers,
                "unit": p.unit,
                "sentence": sentence,
                "from": p.from,
                "to": p.to,
                "atS": p.from as f32 / rate,
                "lenMs": p.to.saturating_sub(p.from) as f32 * 1000.0 / rate,
                "pitch": p.pitch,
                "hard": p.hard,
            }));
        }
    }
    // per picked phrase: the pause after it and the sounds never recorded
    let picked: Vec<Value> = plan
        .iter()
        .map(|p| json!({ "pauseMs": p.pause_ms, "mood": p.mood.map(String::from), "missing": p.missing }))
        .collect();
    let audio = match synth::render(&plan, db, synth::Settings::default()) {
        Ok(s) => json!({ "rate": s.rate, "samples": s.samples.len(), "seconds": s.duration_s(), "envelope": synth::envelope(&s, 256) }),
        Err(e) => json!({ "error": e }),
    };
    json!({
        "phrases": phrases,
        "plan": picked,
        "pieces": pieces,
        "sentences": db.units.len(),
        "audio": audio,
    })
    .to_string()
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
    guard(&mut env, 0, |_| Ok(output_rate(voice(handle)?) as jint))
}

/// The recordings bank's own rate (44 000 for CodecMsx voices): the rate of `pieceSamples`.
#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_audioRate<'l>(mut env: JNIEnv<'l>, _class: JClass<'l>, handle: jlong) -> jint {
    guard(&mut env, 0, |_| Ok(bank(voice(handle)?)?.rate as jint))
}

/// One recorded piece as it sits in the recordings: samples `from..to` of the
/// bank (an `analyse` piece's `from`/`to`), at `audioRate`. `unit` is the
/// piece's recorded sentence, checked against the inventory.
#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_pieceSamples<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    handle: jlong,
    unit: jint,
    from: jint,
    to: jint,
) -> jshortArray {
    guard(&mut env, std::ptr::null_mut(), |env| {
        let v = voice(handle)?;
        if unit < 0 || unit as usize >= v.db.units.len() {
            return Err(format!("no recorded sentence #{unit}"));
        }
        if from < 0 || to < from {
            return Err(format!("bad piece range {from}..{to}"));
        }
        // Gilded phrase databases have their own recordings
        let bank = v.db.bank(unit as u32).ok_or("this piece's recordings can't be decoded")?;
        let pcm = bank.read(from as usize, (to - from) as usize);
        let array = env.new_short_array(pcm.len() as i32).map_err(|e| e.to_string())?;
        env.set_short_array_region(&array, 0, &pcm).map_err(|e| e.to_string())?;
        Ok(array.into_raw())
    })
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
        let pcm = synth::to_pcm16(&speech.samples);
        let array = env.new_short_array(pcm.len() as i32).map_err(|e| e.to_string())?;
        env.set_short_array_region(&array, 0, &pcm).map_err(|e| e.to_string())?;
        Ok(array.into_raw())
    })
}

/// The 16-bit `pcm` at `rate` as an MP3 file (`synth::mp3_bytes`).
#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_encodeMp3<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    pcm: JShortArray<'l>,
    rate: jint,
) -> jbyteArray {
    guard(&mut env, std::ptr::null_mut(), |env| {
        let n = env.get_array_length(&pcm).map_err(|e| e.to_string())? as usize;
        let mut samples = vec![0i16; n];
        env.get_short_array_region(&pcm, 0, &mut samples).map_err(|e| e.to_string())?;
        let mp3 = synth::mp3_bytes(&samples, rate as u32)?;
        Ok(env.byte_array_from_slice(&mp3).map_err(|e| e.to_string())?.into_raw())
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
        assert_eq!(synth::to_pcm16(&[0.0, 1.0, -1.0, 2.0, 0.5 / 32768.0, -0.6 / 32768.0]), vec![0, 32767, -32768, 32767, 1, -1]);
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

    #[test]
    fn catalog_names_missing_modules() {
        assert_eq!(catalog_unsupported("Valencian Catalan").as_deref(), Some("no Valencian module"));
        assert_eq!(catalog_unsupported("Chinese").as_deref(), Some("no Chinese module"));
        assert_eq!(catalog_unsupported("Italian"), None);
    }
}
