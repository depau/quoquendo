//! quoquendo — reopening the Loquendo 7 (LTTS7) voice pipeline.
//!
//! The app never guesses a language from the text: it reads the loaded voice's
//! own `MotherTongue`, runs that language's front end, and shows which symbols
//! the voice's inventory actually contains.

mod install;
mod oracle;
mod pipeline;

use quoquendo::{g2p, loquendo, synth};

use std::path::PathBuf;

use eframe::egui;
use g2p::{analyse, Language, Token};
use loquendo::{discover_voices, VoiceDatabase, VoiceDescriptor};
use synth::Settings;

const VERSION: &str = env!("CARGO_PKG_VERSION");

struct LaunchOptions {
    screenshot: Option<PathBuf>,
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
}

fn parse_args() -> LaunchOptions {
    let mut opts = LaunchOptions {
        screenshot: None,
        selftest: false,
        say: None,
        voice: None,
        out: None,
        stages: pipeline::Stages::default(),
        compare: None,
        fon_list: None,
        learn_lexicon: None,
        install: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--screenshot" {
            opts.screenshot = args.next().map(PathBuf::from);
        } else if let Some(rest) = a.strip_prefix("--screenshot=") {
            opts.screenshot = Some(PathBuf::from(rest));
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
        }
    }
    opts
}

/// Drive every installed voice through the whole pipeline and print what
/// happened. This is the check that the app is not Italian-specific: each
/// voice brings its own language and its own symbol inventory.
fn selftest() -> i32 {
    let voices = install::all_voices();
    if voices.is_empty() {
        println!("no voices found");
        return 2;
    }
    println!("voices found: {}", voices.len());
    let mut failures = 0;
    for v in &voices {
        let lang = v.language();
        println!("\n--- {} ---", v.name);
        println!(
            "  MotherTongue={:?} aliases={:?} -> {}",
            v.mother_tongue,
            v.mother_tongue_aliases,
            lang.label()
        );
        println!(
            "  gender={} base_speed={} base_pitch={} rate={} gilded={}",
            v.gender, v.base_speed, v.base_pitch, v.audio_rate, v.gilded
        );
        println!("  vocal_dbs={:?}", v.vocal_dbs);

        let sde = v.voice_path.join(format!("{}.sde", v.name));
        g2p::load_lexicon(v.language(), &v.mother_tongue);
        let db = match loquendo::load_sde(&sde) {
            Ok(db) => db,
            Err(e) => {
                println!("  FAIL inventory: {e}");
                failures += 1;
                continue;
            }
        };
        println!("  language files: {} dictionary words", quoquendo::load_language_files(v, &db));
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
        let tokens = analyse(&text, lang);
        let words = tokens
            .iter()
            .filter(|t| matches!(t, Token::Word { .. }))
            .count();
        let symbols: Vec<String> = tokens
            .iter()
            .flat_map(|t| t.symbols().iter().cloned())
            .collect();
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
            let mut missing: Vec<&String> =
                symbols.iter().filter(|s| !db.knows_symbol(s)).collect();
            missing.sort();
            missing.dedup();
            println!("  missing: {:?}", missing);
        }

        if let Some(e) = &db.audio_error {
            println!("  FAIL recordings: {e}");
            failures += 1;
            continue;
        }
        let plan = synth::select_all(&db, &synth::from_tokens(&tokens));
        let pieces: usize = plan.iter().map(|p| p.pieces.len()).sum();
        let missing: Vec<&String> = plan.iter().flat_map(|p| &p.missing).collect();
        println!("  units: {} recorded pieces for {} sounds; never recorded: {:?}", pieces, symbols.len(), missing);
        let speech = match synth::render(&plan, &db, Settings::default()) {
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
        let peak = speech
            .samples
            .iter()
            .fold(0.0f32, |m, s| m.max(s.abs()));
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

/// The voice's own demo sentence; Italian voices also get the station
/// warning, the benchmark for "does it sound like the stations".
fn sample_text(v: &VoiceDescriptor) -> String {
    let demo = v.demo_sentence.trim();
    let mut text = if demo.is_empty() { v.language().sample_sentence().to_string() } else { demo.to_string() };
    if v.language() == Language::Italian {
        if !text.ends_with(['.', '!', '?']) {
            text.push('.');
        }
        text.push_str(" Attenzione! Allontanarsi dalla linea gialla.");
    }
    text
}

/// The installed voice called `name` (Roberto when none), with its index.
fn find_voice(name: Option<&str>) -> Result<(VoiceDescriptor, VoiceDatabase), String> {
    let voices = install::all_voices();
    if voices.is_empty() {
        return Err("no voices found: install one with --install <installer.exe>".into());
    }
    let v = match name {
        Some(name) => voices.iter().find(|v| v.name.eq_ignore_ascii_case(name)),
        None => voices.iter().find(|v| v.name == "Roberto").or(voices.first()),
    }
    .ok_or("voice not found")?
    .clone();
    let db = loquendo::load_sde(&v.voice_path.join(format!("{}.sde", v.name)))?;
    g2p::load_lexicon(v.language(), &v.mother_tongue);
    quoquendo::load_language_files(&v, &db);
    Ok((v, db))
}

/// Speak one text with one voice into a WAV file, without a window.
fn say(text: &str, voice: Option<&str>, stages: &pipeline::Stages, out: &std::path::Path) -> Result<(), String> {
    let (v, db) = find_voice(voice)?;
    let run = pipeline::run(text, v.language(), &v.name, &db, stages, None)?;
    eprintln!("{}", synth::format_transcription(&run.targets));
    for m in run.plan.iter().flat_map(|p| &p.missing) {
        eprintln!("never recorded by {}: {m}", v.name);
    }
    let speech = synth::render(&run.plan, &db, Settings { volume: 1.0, join: stages.sig })?;
    std::fs::write(out, synth::to_wav_bytes(&speech)).map_err(|e| format!("{}: {e}", out.display()))?;
    eprintln!(
        "{}: {:.2} s from {} pieces → {}",
        v.name,
        speech.duration_s(),
        run.plan.iter().map(|p| p.pieces.len()).sum::<usize>(),
        out.display()
    );
    Ok(())
}

/// Score every stage against the original engine (needs Wine, see oracle/run.sh).
fn compare(what: &str, voice: Option<&str>) -> Result<(), String> {
    let (v, db) = find_voice(voice)?;
    let texts: Vec<String> = match std::fs::read_to_string(what) {
        Ok(t) => t.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect(),
        Err(_) => vec![what.to_string()],
    };
    let out = oracle::oracle_dir().map(|d| d.join("out/compare"));
    let refs = oracle::run_many(&v.name, &texts)?;
    let (report, _) = pipeline::compare(&refs, &v, &db, out.as_deref())?;
    println!("{report}");
    if let Some(o) = out {
        println!("listen: {}/<n>-original.wav, -replay.wav, -ours.wav", o.display());
    }
    Ok(())
}

struct RodioOut {
    _stream: rodio::OutputStream,
    sink: rodio::Sink,
}

fn main() -> eframe::Result<()> {
    let opts = parse_args();
    // optional pronunciation dictionaries learned from the original engine
    // (`--learn-lexicon`): beside the oracle folder, or beside the program
    if let Some(o) = oracle::oracle_dir() {
        g2p::add_lexicon_dir(o.parent().unwrap_or(&o).join("lexicons"));
    }
    if let Some(exe) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("lexicons"))) {
        g2p::add_lexicon_dir(exe);
    }

    if let Some(path) = &opts.install {
        match install::install_voice(path) {
            Ok(names) => {
                println!("installed {} into {}", names.join(", "), install::voice_store().unwrap_or_default().display());
                return Ok(());
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }
    if opts.selftest {
        std::process::exit(selftest());
    }
    if let Some(text) = &opts.say {
        let out = opts.out.clone().unwrap_or_else(|| PathBuf::from("quoquendo-speech.wav"));
        if let Err(e) = say(text, opts.voice.as_deref(), &opts.stages, &out) {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if let Some(file) = &opts.learn_lexicon {
        let run = || -> Result<(), String> {
            let (v, _) = find_voice(opts.voice.as_deref())?;
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
        };
        if let Err(e) = run() {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if let Some(list) = &opts.fon_list {
        let lang = find_voice(opts.voice.as_deref()).map(|(v, _)| v.language()).unwrap_or(Language::Italian);
        let text = std::fs::read_to_string(list).unwrap_or_default();
        let mut out = String::new();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let phrases = synth::from_tokens(&analyse(line, lang));
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
        return Ok(());
    }
    if let Some(what) = &opts.compare {
        if let Err(e) = compare(what, opts.voice.as_deref()) {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return Ok(());
    }

    let mut native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 860.0])
            .with_title("quoquendo — Loquendo 7 voice pipeline"),
        ..Default::default()
    };

    if opts.screenshot.is_some() {
        // A screenshot must not steal focus, and must not stop drawing when it
        // is not focused.
        native.viewport = native.viewport.with_active(false);
        #[cfg(target_os = "windows")]
        {
            native.viewport = native.viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
        #[cfg(target_os = "macos")]
        {
            use winit::platform::macos::EventLoopBuilderExtMacOS;
            native.event_loop_builder = Some(Box::new(|b| {
                b.with_activate_ignoring_other_apps(false);
            }));
        }
        native.run_and_return = false;
    }

    let app = Quoquendo::new(opts.screenshot.clone());
    eframe::run_native(
        "quoquendo",
        native,
        Box::new(|cc| Ok(Box::new(app.with_ctx(cc)))),
    )
}

struct Quoquendo {
    engine_dir: String,
    data_dir: Option<PathBuf>,
    voices: Vec<VoiceDescriptor>,
    selected: usize,
    db: Option<VoiceDatabase>,
    db_error: Option<String>,
    extras: Vec<(String, usize, usize)>,

    language_override: Option<Language>,
    text: String,

    settings: Settings,
    stages: pipeline::Stages,
    fon_command: String,
    targets: Vec<synth::TargetPhrase>,
    plan: Vec<synth::Phrase>,
    speech: Option<synth::Speech>,
    /// the original engine's results, by text
    refs: std::collections::HashMap<String, oracle::Reference>,
    /// a background run of the original engine; `true` = then compare
    job: Option<(std::thread::JoinHandle<Result<Vec<oracle::Reference>, String>>, bool)>,
    report: Option<String>,

    status: String,
    error: Option<String>,
    out: Option<RodioOut>,

    screenshot_path: Option<PathBuf>,
    frame_no: u64,
    quit_after: Option<u64>,
}

impl Quoquendo {
    fn new(screenshot: Option<PathBuf>) -> Quoquendo {
        let mut q = Quoquendo {
            engine_dir: String::new(),
            data_dir: None,
            voices: Vec::new(),
            selected: 0,
            db: None,
            db_error: None,
            extras: Vec::new(),
            language_override: None,
            text: String::new(),
            settings: Settings::default(),
            stages: pipeline::Stages::default(),
            fon_command: String::new(),
            targets: Vec::new(),
            plan: Vec::new(),
            speech: None,
            refs: Default::default(),
            job: None,
            report: None,
            status: "Looking for an LTTS7 installation…".into(),
            error: None,
            out: None,
            quit_after: screenshot.as_ref().map(|_| 124),
            screenshot_path: screenshot,
            frame_no: 0,
        };
        q.autodetect();
        q
    }

    fn with_ctx(mut self, _cc: &eframe::CreationContext<'_>) -> Quoquendo {
        self.status = format!("quoquendo {VERSION}");
        self
    }

    fn autodetect(&mut self) {
        let voices = install::all_voices();
        if !voices.is_empty() {
            self.status = format!("{} voice{}", voices.len(), if voices.len() == 1 { "" } else { "s" });
            self.voices = voices;
            let first = self.voices.iter().position(|v| v.name == "Roberto").unwrap_or(0);
            self.select_voice(first);
        } else {
            self.status = "No LTTS7 folder found — type its path and press Load.".into();
            self.text = "Type the path to a Loquendo LTTS7 folder.".into();
        }
    }

    fn load_engine(&mut self) {
        let path = PathBuf::from(self.engine_dir.trim());
        if install::is_installer_path(&path) {
            match install::install_voice(&path) {
                Ok(names) => {
                    self.engine_dir.clear();
                    self.autodetect();
                    self.status = format!("Installed {} from the installer. {}", names.join(", "), self.status);
                }
                Err(e) => self.error = Some(e),
            }
            return;
        }
        match resolve_data_dir_at(&path) {
            Some(data) => {
                self.voices = discover_voices(&data);
                self.data_dir = Some(data.clone());
                self.selected = 0;
                if self.voices.is_empty() {
                    self.status = format!("{}: no .vcf voice configurators here.", data.display());
                } else {
                    self.status = format!(
                        "{}: {} voice{}",
                        data.display(),
                        self.voices.len(),
                        if self.voices.len() == 1 { "" } else { "s" }
                    );
                    // Roberto (the station voice) first when it is installed
                    let first = self.voices.iter().position(|v| v.name == "Roberto").unwrap_or(0);
                    self.select_voice(first);
                }
            }
            None => {
                self.error = Some(format!("{} is not an LTTS7 data folder.", path.display()));
            }
        }
    }

    fn select_voice(&mut self, index: usize) {
        if index >= self.voices.len() {
            return;
        }
        self.selected = index;
        self.language_override = None;
        let v = self.voices[index].clone();
        self.speech = None;
        self.plan.clear();
        self.targets.clear();

        let dir = v.voice_path.clone();
        let main = dir.join(format!("{}.sde", v.name));
        let sde = if main.is_file() {
            main
        } else {
            match std::fs::read_dir(&dir).ok().and_then(|mut e| {
                e.find(|f| {
                    f.as_ref()
                        .map(|f| f.path().extension().and_then(|x| x.to_str()) == Some("sde"))
                        .unwrap_or(false)
                })
                .and_then(|r| r.ok())
                .map(|f| f.path())
            }) {
                Some(p) => p,
                None => {
                    self.db_error = Some(format!("no .sde inventory in {}", dir.display()));
                    self.db = None;
                    return;
                }
            }
        };
        match loquendo::load_sde(&sde) {
            Ok(db) => {
                g2p::load_lexicon(v.language(), &v.mother_tongue);
                quoquendo::load_language_files(&v, &db);
                self.extras = loquendo::load_extras(&dir).sets;
                self.db = Some(db);
                self.db_error = None;
                self.text = sample_text(&v);
                self.run_pipeline();
                self.render_audio();
            }
            Err(e) => {
                self.db = None;
                self.db_error = Some(e);
            }
        }
    }

    fn language(&self) -> Language {
        self.language_override
            .unwrap_or_else(|| self.voices.get(self.selected).map(|v| v.language()).unwrap_or(Language::Generic))
    }

    /// Run the original engine on `texts` in the background (it takes a few
    /// seconds under Wine; results are cached).
    fn start_job(&mut self, texts: Vec<String>, then_compare: bool) {
        if self.job.is_some() {
            return;
        }
        let voice = self.voices.get(self.selected).map(|v| v.name.clone()).unwrap_or_default();
        self.job = Some((std::thread::spawn(move || oracle::run_many(&voice, &texts)), then_compare));
        self.status = "Asking the original engine (under Wine)…".into();
    }

    fn poll_job(&mut self) {
        if !self.job.as_ref().is_some_and(|(h, _)| h.is_finished()) {
            return;
        }
        let (handle, then_compare) = self.job.take().unwrap();
        match handle.join().unwrap_or_else(|_| Err("the original engine's run crashed".into())) {
            Ok(refs) => {
                if then_compare {
                    if let (Some(db), Some(v)) = (&self.db, self.voices.get(self.selected)) {
                        let out = oracle::oracle_dir().map(|d| d.join("out/compare"));
                        self.report = Some(match pipeline::compare(&refs, v, db, out.as_deref()) {
                            Ok((r, _)) => r,
                            Err(e) => e,
                        });
                    }
                }
                for r in refs {
                    self.refs.insert(r.text.clone(), r);
                }
                self.run_pipeline();
                self.render_audio();
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn run_pipeline(&mut self) {
        self.speech = None;
        let Some(db) = &self.db else {
            self.plan.clear();
            self.status = "No voice inventory loaded.".into();
            return;
        };
        let reference = match self.refs.get(&self.text) {
            Some(r) => Some(r.clone()),
            None if self.stages.need_original() => {
                self.plan.clear();
                self.targets.clear();
                self.start_job(vec![self.text.clone()], false);
                return;
            }
            None => None,
        };
        let voice = self.voices.get(self.selected).map(|v| v.name.clone()).unwrap_or_default();
        match pipeline::run(&self.text, self.language(), &voice, db, &self.stages, reference) {
            Ok(run) => {
                self.targets = run.targets;
                self.plan = run.plan;
                self.error = None;
                self.status = format!(
                    "{} sounds · {} recorded pieces",
                    self.targets.iter().map(|t| t.phones.len()).sum::<usize>(),
                    self.plan.iter().map(|p| p.pieces.len()).sum::<usize>()
                );
            }
            Err(e) => {
                self.plan.clear();
                self.error = Some(e);
            }
        }
    }

    fn render_audio(&mut self) {
        let Some(db) = self.db.take() else {
            self.error = Some("Load a voice before synthesising.".into());
            return;
        };
        match synth::render(&self.plan, &db, self.settings) {
            Ok(speech) => {
                self.status = format!("{:.2} s of speech at {} Hz", speech.duration_s(), speech.rate);
                self.speech = Some(speech);
            }
            Err(e) => self.error = Some(e),
        }
        self.db = Some(db);
    }

    fn speak(&mut self) {
        if self.db.is_none() {
            self.error = Some("Load a voice before playing.".into());
            return;
        }
        if self.speech.is_none() {
            self.render_audio();
        }
        let Some(speech) = self.speech.clone() else {
            return;
        };
        let bytes = synth::to_wav_bytes(&speech);
        match rodio::OutputStream::try_default() {
            Ok((stream, handle)) => match rodio::Sink::try_new(&handle) {
                Ok(sink) => {
                    sink.append(rodio::Decoder::new(std::io::Cursor::new(bytes)).unwrap());
                    sink.play();
                    self.out = Some(RodioOut {
                        _stream: stream,
                        sink,
                    });
                    self.status = "Playing.".into();
                }
                Err(e) => self.error = Some(format!("audio sink: {e}")),
            },
            Err(e) => {
                self.error = Some(format!(
                    "no audio output device ({e}) — use Save WAV instead"
                ));
            }
        }
    }

    fn save_wav(&mut self) {
        if self.db.is_none() {
            self.error = Some("Load a voice before saving.".into());
            return;
        }
        if self.speech.is_none() {
            self.render_audio();
        }
        let Some(speech) = self.speech.clone() else {
            return;
        };
        let path = PathBuf::from("quoquendo-speech.wav");
        match std::fs::write(&path, synth::to_wav_bytes(&speech)) {
            Ok(_) => self.status = format!("Wrote {}", path.display()),
            Err(e) => self.error = Some(format!("{}: {e}", path.display())),
        }
    }
}

fn resolve_data_dir_at(path: &std::path::Path) -> Option<PathBuf> {
    loquendo::resolve_data_dir_from(path)
}

impl eframe::App for Quoquendo {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_no += 1;
        self.poll_job();
        if self.job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
        if let Some(report) = self.report.clone() {
            let mut open = true;
            egui::Window::new("Compared with the original engine")
                .open(&mut open)
                .default_size([760.0, 420.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::both().show(ui, |ui| {
                        ui.label(egui::RichText::new(report).monospace().size(11.0));
                    });
                });
            if !open {
                self.report = None;
            }
        }

        egui::TopBottomPanel::top("head").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("quoquendo");
                ui.label(format!("v{VERSION}"));
                ui.separator();
                ui.label("LTTS7 folder");
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut self.engine_dir)
                        .desired_width(420.0)
                        .hint_text("LTTS7 folder, or a voice installer .exe to add"),
                );
                if ui.button("Load").clicked()
                    || (edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                {
                    self.load_engine();
                }
            });
        });

        egui::SidePanel::left("voice").min_width(330.0).max_width(400.0).show(ctx, |ui| {
            ui.heading("Voice");
            if self.voices.is_empty() {
                ui.label("No voices found.");
            } else {
                let names: Vec<String> = self
                    .voices
                    .iter()
                    .map(|v| {
                        format!(
                            "{} — {} ({})",
                            v.name,
                            v.language().label(),
                            if v.gender.is_empty() { "?" } else { &v.gender }
                        )
                    })
                    .collect();
                let mut chosen = self.selected;
                let mut changed = false;
                egui::ComboBox::from_id_salt("voices")
                    .selected_text(names.get(self.selected).cloned().unwrap_or_default())
                    .width(300.0)
                    .show_ui(ui, |ui| {
                        for (i, n) in names.iter().enumerate() {
                            if ui.selectable_value(&mut chosen, i, n).clicked() {
                                changed = true;
                            }
                        }
                    });
                if changed && chosen != self.selected {
                    self.select_voice(chosen);
                }

                if let Some(v) = self.voices.get(self.selected) {
                    ui.separator();
                    egui::Grid::new("vtable").num_columns(2).show(ui, |ui| {
                        let mut row = |k: &str, val: String| {
                            ui.label(k);
                            ui.label(val);
                            ui.end_row();
                        };
                        row(
                            "Language",
                            format!(
                                "{} (from the voice)",
                                v.language().label()
                            ),
                        );
                        row(
                            "MotherTongue",
                            loquendo::from_latin1(&v.mother_tongue),
                        );
                        row(
                            "Description",
                            loquendo::from_latin1(&v.description),
                        );
                        row("Gender / Age", format!("{} / {}", v.gender, v.age));
                        row("Library", v.library.clone());
                        row("BaseSpeed", v.base_speed.to_string());
                        row("BasePitch", v.base_pitch.to_string());
                        row("Sample rate", format!("{} Hz", v.audio_rate));
                        row(
                            "Gilded",
                            if v.gilded { "yes".into() } else { "no".to_string() },
                        );
                        row("Recordings file", v.bin_name.clone());
                    });
                }
            }

            if let Some(db) = &self.db {
                ui.separator();
                ui.heading("Voice inventory");
                egui::Grid::new("dbtable").num_columns(2).show(ui, |ui| {
                    let mut row = |k: &str, val: String| {
                        ui.label(k);
                        ui.label(val);
                        ui.end_row();
                    };
                    row(
                        "File",
                        db.path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default(),
                    );
                    row("Format", db.format.clone());
                    row("Recorded sentences", db.units.len().to_string());
                    row(
                        "Timed sounds",
                        format!("{} (+2 edge silences each = {})", db.instance_count(), db.declared_instances),
                    );
                    row(
                        "Questions / exclamations",
                        format!(
                            "{} / {}",
                            db.units.iter().filter(|u| u.mood == Some('I')).count(),
                            db.units.iter().filter(|u| u.mood == Some('E')).count()
                        ),
                    );
                    row("Alphabet", format!("{} symbols", db.alphabet().len()));
                    row("Longest sentence", format!("{} sounds", db.longest_unit()));
                    match &db.audio {
                        Some(a) => {
                            let secs = a.total_samples() / a.rate as usize;
                            row(
                                "Recordings",
                                format!(
                                    "{}:{:02}:{:02} of Speex, {} kbit/s, {} Hz",
                                    secs / 3600,
                                    secs / 60 % 60,
                                    secs % 60,
                                    a.bitrate / 1000,
                                    a.rate
                                ),
                            );
                        }
                        None => row(
                            "Recordings",
                            db.audio_error.clone().unwrap_or_else(|| "not found".into()),
                        ),
                    }
                });
                if !self.extras.is_empty() {
                    ui.label("Extra inventories:");
                    for (name, u, t) in &self.extras {
                        ui.label(format!("  {name}: {u} units, {t} takes"));
                    }
                }
            }
            if let Some(e) = &self.db_error {
                ui.colored_label(egui::Color32::from_rgb(220, 90, 90), e);
            }
        });

        egui::TopBottomPanel::bottom("controls").show(ctx, |ui| {
            let before = (self.stages.clone(), self.fon_command.clone());
            ui.horizontal(|ui| {
                ui.label("Pronunciation");
                let mut fon = match self.stages.fon {
                    pipeline::Fon::Rules => 0,
                    pipeline::Fon::Original => 1,
                    pipeline::Fon::Command(_) => 2,
                };
                egui::ComboBox::from_id_salt("fon")
                    .selected_text(["Built-in rules", "Original engine (Wine)", "Command"][fon])
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut fon, 0, "Built-in rules");
                        ui.selectable_value(&mut fon, 1, "Original engine (Wine)").on_hover_text("Runs the real Loquendo engine as a separate program under Wine on this text and takes its pronunciation; quoquendo's own code does the rest. No Loquendo code is loaded into quoquendo.");
                        ui.selectable_value(&mut fon, 2, "Command");
                    });
                if fon == 2 {
                    ui.add(egui::TextEdit::singleline(&mut self.fon_command).desired_width(180.0).hint_text("prints a transcription"));
                }
                self.stages.fon = match fon {
                    1 => pipeline::Fon::Original,
                    2 => pipeline::Fon::Command(self.fon_command.clone()),
                    _ => pipeline::Fon::Rules,
                };
                ui.separator();
                ui.label("Piece picking");
                let mut cat = self.stages.cat;
                egui::ComboBox::from_id_salt("cat")
                    .selected_text(if cat == pipeline::Cat::Builtin { "Built-in" } else { "Original engine (Wine)" })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut cat, pipeline::Cat::Builtin, "Built-in");
                        ui.selectable_value(&mut cat, pipeline::Cat::Original, "Original engine (Wine)").on_hover_text("Runs the real Loquendo engine as a separate program under Wine and replays the recorded pieces it chose (with its own pronunciation) through quoquendo's decoder and joining. No Loquendo code is loaded into quoquendo.");
                    });
                self.stages.cat = cat;
                ui.separator();
                ui.label("Joining");
                let mut join = self.settings.join;
                egui::ComboBox::from_id_salt("sig")
                    .selected_text(if join == synth::JoinStyle::Original { "Like the original" } else { "Smoothed" })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut join, synth::JoinStyle::Original, "Like the original");
                        ui.selectable_value(&mut join, synth::JoinStyle::Smooth, "Smoothed");
                    });
                if join != self.settings.join {
                    self.settings.join = join;
                    self.stages.sig = join;
                    self.render_audio();
                }
            });
            if (self.stages.clone(), self.fon_command.clone()) != before && !matches!(self.stages.fon, pipeline::Fon::Command(ref c) if c.is_empty()) {
                self.run_pipeline();
                self.render_audio();
            }
            ui.horizontal(|ui| {
                ui.label("Volume");
                if ui.add(egui::Slider::new(&mut self.settings.volume, 0.0..=1.0)).changed() {
                    self.speech = None;
                }
                ui.separator();
                if ui.button("Analyse").clicked() {
                    self.run_pipeline();
                }
                if ui.button("Render").clicked() {
                    self.render_audio();
                }
                if ui.button("Speak").clicked() {
                    self.speak();
                }
                if ui.button("Save WAV").clicked() {
                    self.save_wav();
                }
                ui.separator();
                let busy = self.job.is_some();
                if ui.add_enabled(!busy, egui::Button::new("Compare with the original")).clicked() {
                    self.start_job(vec![self.text.clone()], true);
                }
                if busy {
                    ui.spinner();
                }
                if let Some(out) = &self.out {
                    if ui.button("Stop").clicked() {
                        out.sink.stop();
                    }
                }
            });
            ui.horizontal(|ui| {
                if let Some(e) = &self.error {
                    ui.colored_label(egui::Color32::from_rgb(220, 90, 90), e);
                } else {
                    ui.label(self.status.clone());
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Pipeline");

            // 1. text
            ui.horizontal(|ui| {
                ui.label("Language");
                let mut lang = self.language();
                egui::ComboBox::from_id_salt("lang")
                    .selected_text(lang.label())
                    .show_ui(ui, |ui| {
                        for l in Language::ALL {
                            if ui.selectable_value(&mut lang, l, l.label()).clicked() {
                                self.language_override = Some(l);
                                self.run_pipeline();
                            }
                        }
                    });
                if self.language_override.is_some() && ui.button("use voice's own").clicked() {
                    self.language_override = None;
                    self.run_pipeline();
                }
                if ui.button("voice's sentence").clicked() {
                    if let Some(v) = self.voices.get(self.selected) {
                        self.text = sample_text(v);
                        self.run_pipeline();
                    }
                }
            });

            let response = ui.add(
                egui::TextEdit::multiline(&mut self.text)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY)
                    .hint_text("Type text in the voice's language…"),
            );
            if response.changed() {
                self.run_pipeline();
            }

            ui.separator();

            // 2. phonemes
            let fon_name = match &self.stages.fon {
                pipeline::Fon::Rules => "built-in rules",
                pipeline::Fon::Original => "original engine under Wine",
                pipeline::Fon::Command(_) => "command",
            };
            ui.label(egui::RichText::new(format!("Text -> phonemes ({fon_name})")).strong());
            egui::ScrollArea::vertical()
                .max_height(120.0)
                .id_salt("phons")
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 5.0;
                        let db = self.db.as_ref();
                        for phrase in &self.targets {
                            for (i, t) in phrase.phones.iter().enumerate() {
                                if t.word_start && i > 0 {
                                    ui.label(egui::RichText::new("|").weak());
                                }
                                let known = db.map(|d| d.knows_symbol(&t.sym)).unwrap_or(false);
                                let colour = if known {
                                    egui::Color32::from_rgb(120, 200, 130)
                                } else {
                                    egui::Color32::from_rgb(225, 120, 120)
                                };
                                ui.label(egui::RichText::new(&t.sym).monospace().color(colour));
                            }
                            let tune = match phrase.mood {
                                Some('I') => " ?",
                                Some('E') => " !",
                                _ => "",
                            };
                            ui.label(egui::RichText::new(format!("· {} ms{tune} ·", phrase.pause_ms)).weak().small());
                        }
                    });
                });

            ui.separator();

            // 3. pieces of recording
            let who = self.voices.get(self.selected).map(|v| v.name.clone()).unwrap_or_default();
            ui.label(egui::RichText::new(format!("Phonemes -> pieces of {who}'s own recordings")).strong());
            if let Some(db) = &self.db {
                let pieces: usize = self.plan.iter().map(|p| p.pieces.len()).sum();
                let sounds: usize = self.plan.iter().map(|p| p.phones.len()).sum();
                let from: std::collections::BTreeSet<u32> =
                    self.plan.iter().flat_map(|p| p.pieces.iter().map(|x| x.unit)).collect();
                ui.label(format!(
                    "{sounds} sounds from {pieces} pieces of {} recorded sentences (out of {})",
                    from.len(),
                    db.units.len()
                ));
                let missing: Vec<&String> = self.plan.iter().flat_map(|p| &p.missing).collect();
                if !missing.is_empty() {
                    ui.colored_label(
                        egui::Color32::from_rgb(230, 170, 90),
                        format!("never recorded by this voice, skipped: {}", missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")),
                    );
                }
                let rate = db.audio_rate() as f32;
                egui::ScrollArea::vertical()
                    .max_height(230.0)
                    .id_salt("units")
                    .show(ui, |ui| {
                        egui::Grid::new("ugrid").num_columns(5).striped(true).show(ui, |ui| {
                            for h in ["sounds", "sentence", "at", "length", "pitch"] {
                                ui.label(egui::RichText::new(h).weak());
                            }
                            ui.end_row();
                            for phrase in &self.plan {
                                for p in &phrase.pieces {
                                    let u = &db.units[p.unit as usize];
                                    ui.label(egui::RichText::new(p.covers.join(" ")).monospace().color(
                                        if p.hard {
                                            egui::Color32::from_rgb(230, 170, 90)
                                        } else {
                                            egui::Color32::from_rgb(120, 200, 130)
                                        },
                                    ));
                                    let said: String = u.symbols[1..u.symbols.len().saturating_sub(1)].join(" ");
                                    let said: String = said.chars().take(34).collect();
                                    ui.label(egui::RichText::new(format!("#{} {said}", p.unit)).monospace().weak());
                                    ui.label(format!("{:.2} s", p.from as f32 / rate));
                                    ui.label(format!("{:.0} ms", (p.to - p.from) as f32 * 1000.0 / rate));
                                    ui.label(format!("{} Hz", p.pitch));
                                    ui.end_row();
                                }
                                if phrase.pause_ms > 0 {
                                    let tune = match phrase.mood {
                                        Some('I') => "question ",
                                        Some('E') => "exclamation ",
                                        _ => "",
                                    };
                                    ui.label(egui::RichText::new(format!("· {tune}pause {} ms ·", phrase.pause_ms)).weak().small());
                                    ui.end_row();
                                }
                            }
                        });
                    });
            }

            ui.separator();

            // 4. audio
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Audio").strong());
                if let Some(s) = &self.speech {
                    ui.label(format!("{:.2} s · {} Hz · {} samples", s.duration_s(), s.rate, s.samples.len()));
                } else {
                    ui.label(egui::RichText::new("press Render or Speak").weak());
                }
            });
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(ui.available_width(), 90.0), egui::Sense::hover());
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(24, 26, 30));
            if let Some(s) = &self.speech {
                let env = synth::envelope(s, (rect.width() as usize).max(16));
                let mid = rect.center().y;
                for (i, v) in env.iter().enumerate() {
                    let x = rect.left() + i as f32;
                    let h = v * (rect.height() * 0.45);
                    painter.line_segment(
                        [egui::pos2(x, mid - h), egui::pos2(x, mid + h)],
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 190, 230)),
                    );
                }
            }
        });

        // screenshot mode: keep drawing while unfocused, then save and exit
        if let Some(path) = self.screenshot_path.clone() {
            if self.frame_no == 90 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }
            if let Some(shot) = ctx.input(|i| {
                i.events.iter().find_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            }) {
                if let Some(dir) = path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                let (w, h) = (shot.size[0], shot.size[1]);
                let pixels: Vec<u8> = shot
                    .pixels
                    .iter()
                    .flat_map(|c| [c.r(), c.g(), c.b(), c.a()])
                    .collect();
                match image::save_buffer(&path, &pixels, w as u32, h as u32, image::ColorType::Rgba8) {
                    Ok(_) => {
                        self.status = format!("screenshot → {}", path.display());
                        self.quit_after = Some(self.frame_no + 2);
                    }
                    Err(e) => self.error = Some(format!("screenshot: {e}")),
                }
            }
            ctx.request_repaint();
        }

        if let Some(q) = self.quit_after {
            if self.frame_no >= q {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}
