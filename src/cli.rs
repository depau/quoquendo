//! The command line (desktop): unpacking installers, speaking to a file,
//! checks against the original engine.

use crate::app::sample_text;
use crate::dev::{engines, oracle, pipeline};
use crate::install;
use quoquendo::{g2p, loquendo, synth};

use g2p::{analyse, Language, Token};
use std::path::PathBuf;
use synth::Settings;

#[derive(Default)]
pub struct LaunchOptions {
    pub screenshot: Option<PathBuf>,
    /// `--dev`: developer mode on in the window
    pub dev: bool,
    selftest: bool,
    /// `--say <text> [--voice <name>] [--out <file.wav>]`
    say: Option<String>,
    voice: Option<String>,
    out: Option<PathBuf>,
    /// `--fon rules|original|<command>`, `--cat builtin|original`, `--join original|smooth`
    stages: pipeline::Stages,
    /// `--compare <text file, one sentence per line | text>`
    compare: Option<String>,
    /// `--fon-list <word file>`: print "word<TAB>transcription" per line, as
    /// the original engine's helper does (oracle/loqref.c, LOQ_FON_ONLY)
    fon_list: Option<PathBuf>,
    /// `--learn-lexicon <word file>`: the original's pronunciation of every word
    learn_lexicon: Option<PathBuf>,
    install: Option<PathBuf>,
    /// `--unpack <installer|folder> <LTTS7 out dir>`
    unpack: Option<(PathBuf, PathBuf)>,
    /// `--mirror-catalog <LTTS7 dir> <out catalog.json>`
    mirror_catalog: Option<(PathBuf, PathBuf)>,
    /// `--list-installers`: the pack's installer file names
    list_installers: bool,
}

pub fn parse_args() -> LaunchOptions {
    let mut opts = LaunchOptions::default();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--screenshot" {
            opts.screenshot = args.next().map(PathBuf::from);
        } else if let Some(rest) = a.strip_prefix("--screenshot=") {
            opts.screenshot = Some(PathBuf::from(rest));
        } else if a == "--dev" {
            opts.dev = true;
        } else if a == "--selftest" {
            opts.selftest = true;
        } else if a == "--say" {
            opts.say = args.next();
        } else if a == "--voice" {
            opts.voice = args.next();
        } else if a == "--out" {
            opts.out = args.next().map(PathBuf::from);
        } else if a == "--fon" {
            opts.stages.fon = match args.next().as_deref() {
                Some("original") => pipeline::Fon::Original,
                Some("rules") | None => pipeline::Fon::Rules,
                Some(cmd) => pipeline::Fon::Command(cmd.to_string()),
            };
        } else if a == "--cat" {
            if args.next().as_deref() == Some("original") {
                opts.stages.cat = pipeline::Cat::Original;
            }
        } else if a == "--join" {
            if args.next().as_deref() == Some("smooth") {
                opts.stages.sig = synth::JoinStyle::Smooth;
            }
        } else if a == "--compare" {
            opts.compare = args.next();
        } else if a == "--learn-lexicon" {
            opts.learn_lexicon = args.next().map(PathBuf::from);
        } else if a == "--install" {
            opts.install = args.next().map(PathBuf::from);
        } else if a == "--fon-list" {
            opts.fon_list = args.next().map(PathBuf::from);
        } else if a == "--unpack" {
            if let (Some(from), Some(to)) = (args.next(), args.next()) {
                opts.unpack = Some((from.into(), to.into()));
            }
        } else if a == "--mirror-catalog" {
            if let (Some(from), Some(to)) = (args.next(), args.next()) {
                opts.mirror_catalog = Some((from.into(), to.into()));
            }
        } else if a == "--list-installers" {
            opts.list_installers = true;
        } else if a == "--store" {
            if let Some(dir) = args.next() {
                install::set_store(dir.into());
            }
        }
    }
    opts
}

/// Drive every installed voice through the whole pipeline and print what
/// happened. This is the check that the app is not Italian-specific: each
/// voice brings its own language and its own symbol inventory.
fn selftest() -> i32 {
    let voices = engines::all_voices();
    if voices.is_empty() {
        println!("no voices found");
        return 2;
    }
    println!("voices found: {}", voices.len());
    let mut failures = 0;
    for v in &voices {
        let lang = v.language();
        println!("\n--- {} ---", v.name);
        println!("  MotherTongue={:?} aliases={:?} -> {}", v.mother_tongue, v.mother_tongue_aliases, lang.label());
        println!(
            "  gender={} base_speed={} base_pitch={} rate={} gilded={}",
            v.gender, v.base_speed, v.base_pitch, v.audio_rate, v.gilded
        );
        println!("  vocal_dbs={:?}", v.vocal_dbs);

        let voice = match quoquendo::Voice::open(v) {
            Ok(voice) => voice,
            Err(e) => {
                println!("  FAIL inventory: {e}");
                failures += 1;
                continue;
            }
        };
        let db = &voice.db;
        println!(
            "  language files: {} dictionary words, {} lexicon words",
            voice.frontend.dictionary_len(),
            voice.frontend.lexicon_len()
        );
        if let Some(tongue) = &voice.language_missing {
            println!("  language data missing: needs {tongue}");
        }
        println!(
            "  inventory: declared {} units / {} takes; parsed {} units / {} takes",
            db.declared_units,
            db.declared_instances,
            db.units.len(),
            db.instance_count()
        );
        println!(
            "  alphabet: {} symbols; longest unit {} symbols; audio {} MB @ {} Hz",
            db.alphabet().len(),
            db.longest_unit(),
            db.audio_size / (1024 * 1024),
            db.audio_rate()
        );

        let text = sample_text(v);
        println!("  text: {text}");
        let tokens = analyse(&text, &voice.frontend);
        let words = tokens.iter().filter(|t| matches!(t, Token::Word { .. })).count();
        let symbols: Vec<String> = tokens.iter().flat_map(|t| t.symbols().iter().cloned()).collect();
        println!("  phonemes: {} words, {} symbols", words, symbols.len());
        println!("  first phonemes: {:?}", &symbols[..symbols.len().min(18)]);

        let known = symbols.iter().filter(|s| db.knows_symbol(s)).count();
        println!(
            "  in this voice's inventory: {}/{} ({}%)",
            known,
            symbols.len(),
            known * 100 / symbols.len().max(1)
        );
        if known < symbols.len() {
            let mut missing: Vec<&String> = symbols.iter().filter(|s| !db.knows_symbol(s)).collect();
            missing.sort();
            missing.dedup();
            println!("  missing: {:?}", missing);
        }

        if let Some(e) = &db.audio_error {
            println!("  FAIL recordings: {e}");
            failures += 1;
            continue;
        }
        let plan = synth::select_all(db, &synth::from_tokens(&tokens));
        let pieces: usize = plan.iter().map(|p| p.pieces.len()).sum();
        let missing: Vec<&String> = plan.iter().flat_map(|p| &p.missing).collect();
        println!(
            "  units: {} recorded pieces for {} sounds; never recorded: {:?}",
            pieces,
            symbols.len(),
            missing
        );
        let speech = match synth::render(&plan, db, Settings::default()) {
            Ok(s) => s,
            Err(e) => {
                println!("  FAIL render: {e}");
                failures += 1;
                continue;
            }
        };
        let out = std::env::temp_dir().join(format!("quoquendo-{}.wav", v.name));
        let _ = std::fs::write(&out, synth::to_wav_bytes(&speech));
        println!("  wrote {}", out.display());
        let peak = speech.samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        println!(
            "  audio: {:.2} s @ {} Hz, {} samples, peak {:.3}",
            speech.duration_s(),
            speech.rate,
            speech.samples.len(),
            peak
        );
        if peak < 0.05 {
            println!("  FAIL audio is silent");
            failures += 1;
        }
        if speech.samples.is_empty() {
            failures += 1;
        }
    }
    println!("\n{} voice(s), {} failure(s)", voices.len(), failures);
    if failures == 0 {
        0
    } else {
        1
    }
}

/// The installed voice called `name` (Roberto when none), loaded.
fn find_voice(name: Option<&str>) -> Result<quoquendo::Voice, String> {
    let voices = engines::all_voices();
    if voices.is_empty() {
        return Err("no voices found: install one with --install <installer.exe>".into());
    }
    let v = match name {
        Some(name) => voices.iter().find(|v| v.name.eq_ignore_ascii_case(name)),
        None => voices.iter().find(|v| v.name == "Roberto").or(voices.first()),
    }
    .ok_or("voice not found")?;
    quoquendo::Voice::open(v)
}

/// Speak one text with one voice into a WAV file, without a window.
fn say(text: &str, voice: Option<&str>, stages: &pipeline::Stages, out: &std::path::Path) -> Result<(), String> {
    let v = find_voice(voice)?;
    let name = &v.descriptor.name;
    let run = pipeline::run(text, &v.frontend, name, &v.db, stages, None)?;
    eprintln!("{}", synth::format_transcription(&run.targets));
    for m in run.plan.iter().flat_map(|p| &p.missing) {
        eprintln!("never recorded by {name}: {m}");
    }
    let speech = synth::render(&run.plan, &v.db, Settings { volume: 1.0, join: stages.sig })?;
    std::fs::write(out, synth::to_wav_bytes(&speech)).map_err(|e| format!("{}: {e}", out.display()))?;
    eprintln!(
        "{name}: {:.2} s from {} pieces → {}",
        speech.duration_s(),
        run.plan.iter().map(|p| p.pieces.len()).sum::<usize>(),
        out.display()
    );
    Ok(())
}

/// Score every stage against the original engine (needs Wine, see oracle/run.sh).
fn compare(what: &str, voice: Option<&str>) -> Result<(), String> {
    let v = find_voice(voice)?;
    let texts: Vec<String> = match std::fs::read_to_string(what) {
        Ok(t) => t.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect(),
        Err(_) => vec![what.to_string()],
    };
    let out = oracle::oracle_dir().map(|d| d.join("out/compare"));
    let refs = oracle::run_many(&v.descriptor.name, &texts)?;
    let (report, _) = pipeline::compare(&refs, &v, out.as_deref())?;
    println!("{report}");
    if let Some(o) = out {
        println!("listen: {}/<n>-original.wav, -replay.wav, -ours.wav", o.display());
    }
    Ok(())
}

/// QQ_CHECK_MARKS=<trace.join>: our pitch marks against the original's
/// (oracle/loqref.c LOQ_TRACE_JOIN "M" lines).
fn check_marks(trace: &str, voice: Option<&str>) -> Result<(), String> {
    let v = find_voice(voice)?;
    let db = &v.db;
    let bank = db.audio.as_ref().ok_or("the voice has no recordings")?;
    let tab = db.tables.as_ref().ok_or("the voice has no phone tables")?;
    let mut at = std::collections::HashMap::new();
    for u in &db.units {
        for p in u.phones.iter().filter(|p| p.timed) {
            at.insert(p.tick_l, (p.sym.clone(), p.len_l));
            at.insert(p.tick_r, (p.sym.clone(), p.len_r));
        }
    }
    let (mut ok, mut n) = (0, 0);
    let text = std::fs::read_to_string(trace).map_err(|e| format!("{trace}: {e}"))?;
    for line in text.lines().filter(|l| l.starts_with('M')) {
        let kv: std::collections::HashMap<&str, &str> = line.split(' ').filter_map(|f| f.split_once('=')).collect();
        let t0: u64 = kv["t0"].parse().unwrap();
        let t1: u64 = kv["t1"].parse().unwrap();
        let f0: u32 = kv["f0"].parse().unwrap();
        let theirs: Vec<i64> = line.split(" marks ").nth(1).unwrap_or("").split(' ').filter_map(|x| x.parse().ok()).collect();
        let Some((sym, _)) = at.get(&t0) else { continue };
        let voicing = quoquendo::sig::Voicing::of_features(tab.features[tab.id(sym) as usize]);
        let start = loquendo::ticks_to_samples(t0, 32000) as usize;
        let len = (loquendo::ticks_to_samples(t1, 32000) - loquendo::ticks_to_samples(t0, 32000)) as usize;
        let buf = bank.read(start - 64, len + 64 + 600);
        let s = |i: isize| *buf.get((i + 64) as usize).unwrap_or(&0) as i32;
        let ours: Vec<i64> = quoquendo::sig::pitch_marks(&s, len, 32000, f0, voicing).iter().map(|&m| m as i64).collect();
        let rel: Vec<i64> = theirs.iter().map(|m| m - theirs[0]).collect();
        n += 1;
        if ours == rel {
            ok += 1;
        } else {
            println!("{sym} {voicing:?} f0={f0} len={len}\n  theirs {rel:?}\n  ours   {ours:?}");
        }
    }
    println!("{ok} of {n} half-phones with the same marks");
    Ok(())
}

fn learn_lexicon(file: &std::path::Path, voice: Option<&str>) -> Result<(), String> {
    let v = find_voice(voice)?.descriptor;
    let text = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let words: Vec<String> = text.lines().map(|l| l.trim().to_lowercase()).filter(|l| !l.is_empty()).collect();
    let pairs = oracle::transcribe_words(&v.name, &words)?;
    let dir = g2p::lexicon_dirs().into_iter().next().ok_or("no place for lexicons")?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.tsv", v.mother_tongue));
    let body: String = pairs.iter().filter(|(_, t)| !t.is_empty()).map(|(w, t)| format!("{w}\t{t}\n")).collect();
    std::fs::write(&path, body).map_err(|e| e.to_string())?;
    println!("{} words -> {}", pairs.len(), path.display());
    Ok(())
}

fn fon_list(list: &std::path::Path, voice: Option<&str>) {
    let fe = find_voice(voice).map(|v| v.frontend).unwrap_or_else(|_| g2p::Frontend::rules(Language::Italian));
    let text = std::fs::read_to_string(list).unwrap_or_default();
    let mut out = String::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let phrases = synth::from_tokens(&analyse(line, &fe));
        let words: Vec<String> = phrases.iter().flat_map(|p| &p.phones).fold(Vec::new(), |mut w: Vec<String>, t| {
            if t.word_start || w.is_empty() {
                w.push(t.sym.clone());
            } else {
                let last = w.last_mut().unwrap();
                last.push(' ');
                last.push_str(&t.sym);
            }
            w
        });
        out.push_str(&format!("{line}\t{}-\n", words.join("-")));
    }
    print!("{out}");
}

/// Run a command-line mode, if one was asked for: its exit code. None opens
/// the window.
pub fn run(opts: &LaunchOptions) -> Option<i32> {
    // optional pronunciation dictionaries learned from the original engine
    // (`--learn-lexicon`): beside the oracle folder, or beside the program
    if let Some(o) = oracle::oracle_dir() {
        g2p::add_lexicon_dir(o.parent().unwrap_or(&o).join("lexicons"));
    }
    if let Some(exe) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("lexicons"))) {
        g2p::add_lexicon_dir(exe);
    }
    let done = |r: Result<(), String>| match r {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    };
    if let Ok(trace) = std::env::var("QQ_CHECK_MARKS") {
        return Some(done(check_marks(&trace, opts.voice.as_deref())));
    }
    if opts.list_installers {
        for i in &quoquendo::catalog::INSTALLERS {
            println!("{}", i.file);
        }
        return Some(0);
    }
    if let Some((from, to)) = &opts.unpack {
        return Some(done(install::unpack(from, to).map(|names| println!("unpacked {} into {}", names.join(", "), to.display()))));
    }
    if let Some((from, to)) = &opts.mirror_catalog {
        return Some(done(crate::mirror::write_catalog(from, to).map(|n| println!("{n} voices -> {}", to.display()))));
    }
    if let Some(path) = &opts.install {
        let store = install::voice_store().unwrap_or_default();
        return Some(done(install::install_voice(path).map(|names| println!("installed {} into {}", names.join(", "), store.display()))));
    }
    if opts.selftest {
        return Some(selftest());
    }
    if let Some(text) = &opts.say {
        let out = opts.out.clone().unwrap_or_else(|| PathBuf::from("quoquendo-speech.wav"));
        return Some(done(say(text, opts.voice.as_deref(), &opts.stages, &out)));
    }
    if let Some(file) = &opts.learn_lexicon {
        return Some(done(learn_lexicon(file, opts.voice.as_deref())));
    }
    if let Some(list) = &opts.fon_list {
        fon_list(list, opts.voice.as_deref());
        return Some(0);
    }
    if let Some(what) = &opts.compare {
        return Some(done(compare(what, opts.voice.as_deref())));
    }
    None
}
