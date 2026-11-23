//! JNI bindings for the Android app: the static methods of the Kotlin
//! `object eu.depau.quoquendo.Native` (`System.loadLibrary("quoquendo_jni")`).
//!
//! A voice store is an LTTS7 root (`data/…`, `bin/…`) on the device. Every
//! export turns errors and panics into a `java.lang.RuntimeException`.

use std::ops::Range;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use jni::objects::{JClass, JObject, JString, JValue};
use jni::sys::{jboolean, jbyteArray, jint, jlong, jobject, jshortArray, jstring, JNI_TRUE};
use jni::JNIEnv;
use quoquendo::export::{self, Format, Tagged};
use quoquendo::g2p::{self, Language};
use quoquendo::profile::Profile;
use quoquendo::synth::{self, JoinStyle};
use quoquendo::{catalog, codec, Speech, Utterance, Voice, VoiceFiles};
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

/// A nullable `String` argument.
fn opt_jstr(env: &mut JNIEnv, s: &JString) -> Result<Option<String>, String> {
    if s.is_null() {
        Ok(None)
    } else {
        jstr(env, s).map(Some)
    }
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
                "demoSentence": d.sample_text(),
                "languageMissing": v.language_missing.is_some(),
                "unsupported": v.unsupported,
                "languageFolder": v.language_dir().and_then(|d| d.rsplit('/').next()),
                "files": files,
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
                "unsupported": catalog::unsupported(i.language),
            })
        })
        .collect();
    Value::Array(all).to_string()
}

/// A language's flag as `[kind, rgb, numbers…]` arrays in a 30×21 box:
/// `["rect", rgb, x, y, w, h]`, `["poly", rgb, x1, y1, x2, y2, …]`,
/// `["circle", rgb, x, y, r]`, `["star", rgb, x, y, outer, inner]`;
/// empty for a language without one.
fn flag_json(language: &str) -> String {
    use quoquendo::flag::{self, Shape};
    let shapes: Vec<Value> = flag::of(language)
        .unwrap_or_default()
        .iter()
        .map(|s| match *s {
            Shape::Rect(x, y, w, h, c) => json!(["rect", c, x, y, w, h]),
            Shape::Poly(points, c) => {
                let mut v = vec![json!("poly"), json!(c)];
                v.extend(points.iter().flat_map(|&(x, y)| [json!(x), json!(y)]));
                Value::Array(v)
            }
            Shape::Circle(x, y, r, c) => json!(["circle", c, x, y, r]),
            Shape::Star(x, y, r, c) => json!(["star", c, x, y, r, r * flag::STAR_INNER]),
        })
        .collect();
    Value::Array(shapes).to_string()
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

/// Waveform outline values per second of speech (`analyse`'s `envelope`).
const ENVELOPE_PER_S: u32 = 100;

fn settings(smooth: bool) -> synth::Settings {
    synth::Settings {
        volume: 1.0,
        join: if smooth { JoinStyle::Smooth } else { JoinStyle::Original },
    }
}

/// The languages whose reading can replace a voice's own: its own first,
/// then those with built-in rules (the ported ones read nothing without
/// their module).
fn languages(voice: &Voice) -> Vec<Language> {
    let own = voice.descriptor.language();
    std::iter::once(own).chain(Language::ALL.into_iter().filter(|l| *l != own && !l.is_ported())).collect()
}

/// `text` read by `voice`, by another language's rules when `language`
/// names one (`Language::label`).
fn utterance(voice: &Voice, text: &str, language: Option<&str>) -> Utterance {
    let other = language.and_then(|name| Language::ALL.into_iter().find(|l| l.label() == name)).filter(|l| *l != voice.descriptor.language());
    match other {
        Some(l) => voice.utterance_with(text, &g2p::Frontend::rules(l)),
        None => voice.utterance(text),
    }
}

/// What `analyse` shows and plays.
struct Rendering {
    utterance: Utterance,
    plan: Vec<synth::Phrase>,
    speech: Result<Speech, String>,
    /// word ranges in the original text (before any profile transform)
    words: Vec<Range<usize>>,
}

fn render(voice: &Voice, text: &str, smooth: bool, language: Option<&str>, profile: Option<&str>) -> Result<Rendering, String> {
    let lang = language
        .and_then(|name| Language::ALL.into_iter().find(|l| l.label() == name))
        .unwrap_or_else(|| voice.descriptor.language());
    let profile = match profile.filter(|p| !p.trim().is_empty()) {
        Some(json) => Profile::from_json(json)?,
        None => Profile::faithful(),
    };
    let applied = profile.apply(text, lang);
    let utterance = utterance(voice, &applied.text, language);
    let plan = synth::select_all(&voice.db, &utterance.targets);
    let speech = synth::render(&plan, &voice.db, settings(smooth));
    let words = utterance.words.iter().map(|r| applied.source_range(r.clone())).collect();
    Ok(Rendering { utterance, plan, speech, words })
}

/// Per spoken word, in order: its first sample and its UTF-16 range in
/// `text`, flattened. A stretch of text several words share is marked once.
fn marks(text: &str, words: &[Range<usize>], speech: &Speech) -> Vec<i32> {
    let mut out: Vec<i32> = Vec::new();
    for (w, span) in speech.timeline.word_spans() {
        let Some(r) = words.get(w) else { continue };
        let u = quoquendo::text::utf16_range(text, r.clone());
        if out.len() >= 3 && out[out.len() - 2] == u.start as i32 && out[out.len() - 1] == u.end as i32 {
            continue;
        }
        out.extend([span.start as i32, u.start as i32, u.end as i32]);
    }
    out
}

/// Peak of every `step` samples, in 0..1.
fn peaks(samples: &[f32], step: usize) -> Vec<f32> {
    samples.chunks(step.max(1)).map(|c| c.iter().fold(0.0f32, |a, s| a.max(s.abs())).min(1.0)).collect()
}

fn analyse_json(voice: &Voice, text: &str, r: &Rendering) -> String {
    let db = &voice.db;
    let speech = r.speech.as_ref().ok();
    let secs = |sample: usize| speech.map_or(0.0, |s| sample as f64 / s.rate.max(1) as f64);
    let sounds: Vec<Value> = match speech {
        Some(s) => s
            .timeline
            .sounds
            .iter()
            .map(|t| {
                json!({
                    "sym": t.sym,
                    "known": t.pause || db.knows_symbol(&t.sym),
                    "atS": secs(t.span.start),
                    "endS": secs(t.span.end),
                    "word": t.word,
                    "phrase": t.phrase,
                    "wordStart": t.word_start,
                    "pause": t.pause,
                })
            })
            .collect(),
        // nothing rendered: the sounds wanted, untimed
        None => r
            .utterance
            .targets
            .iter()
            .enumerate()
            .flat_map(|(i, p)| {
                p.phones.iter().map(move |t| {
                    json!({ "sym": t.sym, "known": db.knows_symbol(&t.sym), "atS": 0, "endS": 0, "word": t.word, "phrase": i, "wordStart": t.word_start, "pause": false })
                })
            })
            .collect(),
    };
    // piece positions are samples of the recordings bank
    let rate = db.audio.as_ref().map_or(db.audio_rate(), |b| b.rate).max(1) as f32;
    let out_spans = speech.map(|s| s.timeline.pieces.as_slice()).unwrap_or_default();
    let mut pieces = Vec::new();
    for (i, phrase) in r.plan.iter().enumerate() {
        for p in &phrase.pieces {
            let sentence = db
                .units
                .get(p.unit as usize)
                .and_then(|u| u.symbols.get(1..u.symbols.len().saturating_sub(1)))
                .map(|s| s.join(" "))
                .unwrap_or_default();
            let out = out_spans.get(pieces.len()).cloned().unwrap_or(0..0);
            pieces.push(json!({
                "phrase": i,
                "covers": p.covers,
                "unit": p.unit,
                "sentence": sentence,
                "from": p.from,
                "to": p.to,
                "recS": p.from as f32 / rate,
                "lenMs": p.to.saturating_sub(p.from) as f32 * 1000.0 / rate,
                "pitch": p.pitch,
                "hard": p.hard,
                "atS": secs(out.start),
                "endS": secs(out.end),
            }));
        }
    }
    // per picked phrase: the pause after it and the sounds never recorded
    let picked: Vec<Value> = r
        .plan
        .iter()
        .map(|p| json!({ "pauseMs": p.pause_ms, "mood": p.mood.map(String::from), "missing": p.missing }))
        .collect();
    let spans: std::collections::HashMap<usize, Range<usize>> = speech.map(|s| s.timeline.word_spans().into_iter().collect()).unwrap_or_default();
    let words: Vec<Value> = r
        .words
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let u = quoquendo::text::utf16_range(text, w.clone());
            match spans.get(&i) {
                Some(s) => json!({ "start": u.start, "end": u.end, "at": s.start, "atS": secs(s.start), "endS": secs(s.end) }),
                None => json!({ "start": u.start, "end": u.end, "at": null, "atS": null, "endS": null }),
            }
        })
        .collect();
    let audio = match &r.speech {
        Ok(s) => {
            let step = (s.rate / ENVELOPE_PER_S).max(1) as usize;
            json!({ "rate": s.rate, "samples": s.samples.len(), "seconds": s.duration_s(), "step": step, "envelope": peaks(&s.samples, step) })
        }
        Err(e) => json!({ "error": e }),
    };
    json!({
        "sounds": sounds,
        "plan": picked,
        "pieces": pieces,
        "words": words,
        "sentences": db.units.len(),
        "audio": audio,
    })
    .to_string()
}

/// What the Voice details section shows, and the languages it can read with.
fn details_json(voice: &Voice) -> String {
    let d = &voice.descriptor;
    let db = &voice.db;
    let recordings = match &db.audio {
        Some(a) => {
            let secs = a.total_samples() / a.rate.max(1) as usize;
            format!("{}:{:02}:{:02} · Speex {} kbit/s", secs / 3600, secs / 60 % 60, secs % 60, a.bitrate / 1000)
        }
        None => db.audio_error.clone().unwrap_or_else(|| "not found".into()),
    };
    let language_data = match &voice.language_missing {
        Some(t) => format!("None: no {t} language folder"),
        None if voice.frontend.has_module() => "Original module, ported".into(),
        None => format!("Built-in rules, {} dictionary words", voice.frontend.dictionary_len()),
    };
    json!({
        "language": d.language().label(),
        "languages": languages(voice).iter().map(|l| l.label()).collect::<Vec<_>>(),
        "rows": [
            ["Language", d.language().label()],
            ["MotherTongue", quoquendo::loquendo::from_latin1(&d.mother_tongue)],
            ["Gender / Age", format!("{} / {}", d.gender, d.age)],
            ["Library", d.library],
            ["Sample rate", format!("{} Hz", grouped(d.audio_rate as usize))],
            ["Recorded sentences", grouped(db.units.len())],
            ["Timed sounds", grouped(db.instance_count())],
            ["Alphabet", format!("{} symbols", db.alphabet().len())],
            ["Recordings", recordings],
            ["Language data", language_data],
        ],
    })
    .to_string()
}

/// `80 896`: thousands grouped with a space, as the app shows numbers.
fn grouped(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

fn formats_json() -> String {
    let all: Vec<Value> =
        Format::ALL.iter().map(|f| json!({ "ext": f.ext(), "label": f.label(), "hint": f.hint(), "mime": f.mime() })).collect();
    Value::Array(all).to_string()
}

/// `text` spoken by `voice` as a file in the format with extension `ext`.
fn export_file(voice: &Voice, text: &str, ext: &str, smooth: bool, language: Option<&str>, profile: Option<&str>) -> Result<Vec<u8>, String> {
    let format = Format::from_ext(ext).ok_or_else(|| format!("no .{ext} format"))?;
    let r = render(voice, text, smooth, language, profile)?;
    let speech = r.speech?;
    let tags = Tagged {
        text,
        words: &r.words,
        voice: &voice.descriptor.name,
    };
    export::encode(&speech, format, Some(&tags))
}

/// A `eu.depau.quoquendo.Rendered` of `speech`.
fn rendered<'l>(env: &mut JNIEnv<'l>, speech: &Speech, marks: &[i32]) -> Result<JObject<'l>, String> {
    let err = |e: jni::errors::Error| e.to_string();
    let pcm = synth::to_pcm16(&speech.samples);
    let pcm_array = env.new_short_array(pcm.len() as i32).map_err(err)?;
    env.set_short_array_region(&pcm_array, 0, &pcm).map_err(err)?;
    let marks_array = env.new_int_array(marks.len() as i32).map_err(err)?;
    env.set_int_array_region(&marks_array, 0, marks).map_err(err)?;
    env.new_object(
        "eu/depau/quoquendo/Rendered",
        "([SI[I)V",
        &[JValue::Object(&pcm_array), JValue::Int(speech.rate as jint), JValue::Object(&marks_array)],
    )
    .map_err(err)
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

/// The built-in speech profiles, as the JSON the app stores and edits.
#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_builtinProfiles<'l>(mut env: JNIEnv<'l>, _class: JClass<'l>) -> jstring {
    guard(&mut env, std::ptr::null_mut(), |env| {
        ret_string(env, serde_json::to_string(&Profile::builtins()).unwrap_or_else(|_| "[]".into()))
    })
}

/// A profile applied to `text`, for the editor's Try it field.
#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_applyProfile<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    text: JString<'l>,
    profile: JString<'l>,
    language: JString<'l>,
) -> jstring {
    guard(&mut env, std::ptr::null_mut(), |env| {
        let (text, profile, language) = (jstr(env, &text)?, jstr(env, &profile)?, opt_jstr(env, &language)?);
        let profile = Profile::from_json(&profile)?;
        let lang = language
            .as_deref()
            .and_then(|name| Language::ALL.into_iter().find(|l| l.label() == name))
            .unwrap_or(Language::Italian);
        let applied = profile.apply(&text, lang);
        ret_string(env, json!({ "text": applied.text, "warnings": applied.warnings }).to_string())
    })
}

#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_flag<'l>(mut env: JNIEnv<'l>, _class: JClass<'l>, language: JString<'l>) -> jstring {
    guard(&mut env, std::ptr::null_mut(), |env| {
        let language = jstr(env, &language)?;
        ret_string(env, flag_json(&language))
    })
}

/// A flag that is a picture (`Trenitalia`) as PNG bytes, or null: see [`flag::image`].
#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_flagImage<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    language: JString<'l>,
) -> jbyteArray {
    guard(&mut env, std::ptr::null_mut(), |env| {
        let language = jstr(env, &language)?;
        match quoquendo::flag::image(&language) {
            Some(bytes) => Ok(env.byte_array_from_slice(bytes).map_err(|e| e.to_string())?.into_raw()),
            None => Ok(std::ptr::null_mut()),
        }
    })
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

/// `text` spoken by the voice for the TTS service: a `Rendered` with its
/// words' marks.
#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_synthesize<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    handle: jlong,
    text: JString<'l>,
    smooth: jboolean,
    language: JString<'l>,
    profile: JString<'l>,
) -> jobject {
    guard(&mut env, std::ptr::null_mut(), |env| {
        let (text, language, profile) = (jstr(env, &text)?, opt_jstr(env, &language)?, opt_jstr(env, &profile)?);
        let r = render(voice(handle)?, &text, smooth == JNI_TRUE, language.as_deref(), profile.as_deref())?;
        let speech = r.speech?;
        let marks = marks(&text, &r.words, &speech);
        Ok(rendered(env, &speech, &marks)?.into_raw())
    })
}

/// `text` spoken by the voice as a file (`Format::from_ext(ext)`), with the
/// text and its timing embedded.
#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_export<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    handle: jlong,
    text: JString<'l>,
    ext: JString<'l>,
    smooth: jboolean,
    language: JString<'l>,
    profile: JString<'l>,
) -> jbyteArray {
    guard(&mut env, std::ptr::null_mut(), |env| {
        let (text, ext, language, profile) = (jstr(env, &text)?, jstr(env, &ext)?, opt_jstr(env, &language)?, opt_jstr(env, &profile)?);
        let bytes = export_file(voice(handle)?, &text, &ext, smooth == JNI_TRUE, language.as_deref(), profile.as_deref())?;
        Ok(env.byte_array_from_slice(&bytes).map_err(|e| e.to_string())?.into_raw())
    })
}

#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_formats<'l>(mut env: JNIEnv<'l>, _class: JClass<'l>) -> jstring {
    guard(&mut env, std::ptr::null_mut(), |env| ret_string(env, formats_json()))
}

#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_details<'l>(mut env: JNIEnv<'l>, _class: JClass<'l>, handle: jlong) -> jstring {
    guard(&mut env, std::ptr::null_mut(), |env| {
        let out = details_json(voice(handle)?);
        ret_string(env, out)
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

/// `text` analysed and rendered: an `Analysed` with the analysis JSON and,
/// when it could be rendered, its audio.
#[no_mangle]
pub extern "system" fn Java_eu_depau_quoquendo_Native_analyse<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    handle: jlong,
    text: JString<'l>,
    smooth: jboolean,
    language: JString<'l>,
    profile: JString<'l>,
) -> jobject {
    guard(&mut env, std::ptr::null_mut(), |env| {
        let (text, language, profile) = (jstr(env, &text)?, opt_jstr(env, &language)?, opt_jstr(env, &profile)?);
        let v = voice(handle)?;
        let r = render(v, &text, smooth == JNI_TRUE, language.as_deref(), profile.as_deref())?;
        let json = env.new_string(analyse_json(v, &text, &r)).map_err(|e| e.to_string())?;
        let audio = match &r.speech {
            Ok(s) => rendered(env, s, &marks(&text, &r.words, s))?,
            Err(_) => JObject::null(),
        };
        env.new_object(
            "eu/depau/quoquendo/Analysed",
            "(Ljava/lang/String;Leu/depau/quoquendo/Rendered;)V",
            &[JValue::Object(&json), JValue::Object(&audio)],
        )
        .map(|o| o.into_raw())
        .map_err(|e| e.to_string())
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
    fn peaks_cover_every_sample() {
        assert_eq!(peaks(&[0.1, -0.5, 0.2, 0.3, -2.0], 2), vec![0.5, 0.3, 1.0]);
    }

    #[test]
    fn thousands_grouped() {
        assert_eq!(grouped(7), "7");
        assert_eq!(grouped(4329), "4 329");
        assert_eq!(grouped(318502), "318 502");
        assert_eq!(grouped(1234567), "1 234 567");
    }

    #[test]
    fn formats_have_extensions() {
        let v: Value = serde_json::from_str(&formats_json()).unwrap();
        assert_eq!(v.as_array().unwrap().len(), Format::ALL.len());
        assert_eq!(v[1]["ext"], "flac");
    }
}
