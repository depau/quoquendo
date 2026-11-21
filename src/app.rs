//! The window, the same on the desktop and in the browser: a Speak tab
//! (voice, text, Speak/Stop, Save in a chosen format, and what the engine
//! makes of the text: its sounds, the recorded pieces and the audio,
//! previewed while typing and followed while speaking) and a Voices tab (the
//! library: installed voices, downloads, the pack's catalog). Advanced mode
//! adds the voice's details and the stage selectors; Original engine (Wine)
//! mode (desktop only) adds the original engine beside ours.

mod modes;
mod speak;

use crate::audio::Player;
use crate::catalog::{self, Installer, INSTALLERS};
use crate::store::{self, InstalledVoice, StoreEvent};
use eframe::egui::{self, Color32, RichText};
use quoquendo::export::Format;
use quoquendo::g2p::Language;
use quoquendo::loquendo;
use quoquendo::synth::Settings;
use speak::{Preview, Track, Want};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
#[cfg(target_arch = "wasm32")]
use crate::route::{self, Route};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const GROUND: Color32 = Color32::from_rgb(0x1a, 0x19, 0x17);
const PANEL: Color32 = Color32::from_rgb(0x22, 0x21, 0x1e);
const WIDGET: Color32 = Color32::from_rgb(0x2e, 0x2d, 0x29);
const HOVER: Color32 = Color32::from_rgb(0x3a, 0x39, 0x34);
const STROKE: Color32 = Color32::from_rgb(0x3c, 0x3a, 0x35);
const TEXT: Color32 = Color32::from_rgb(0xe4, 0xdf, 0xd3);
const MUTED: Color32 = Color32::from_rgb(0x94, 0x8e, 0x82);
const AMBER: Color32 = Color32::from_rgb(0xe9, 0xb2, 0x4c);
const GREEN: Color32 = Color32::from_rgb(0x8b, 0xc9, 0x6e);
const RED: Color32 = Color32::from_rgb(0xe8, 0x70, 0x5f);
const LILAC: Color32 = Color32::from_rgb(0xc9, 0xa8, 0xff);
/// a waveform's part not played yet
const WAVE_DIM: Color32 = Color32::from_rgb(0x72, 0x6e, 0x65);
/// the original engine's waveform
#[cfg(not(target_arch = "wasm32"))]
const WAVE_ORIGINAL: Color32 = Color32::from_rgb(0xb3, 0xad, 0xa0);

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Speak,
    Voices,
    Explore,
}

/// The recordings explorer's state: a phone and its takes, or one recorded
/// sentence.
#[derive(Default)]
struct Explore {
    filter: String,
    phone: Option<String>,
    unit: Option<usize>,
    page: usize,
    /// for the loaded voice: each phone with every take of it (sentence, phone index)
    index: Option<(String, Vec<(String, Vec<(u32, u16)>)>)>,
    /// the left list shows the phones (else the recorded sentences)
    phones: bool,
    /// scroll the sentence list to `unit` once (a link opened it)
    reveal: bool,
    sentence_filter: String,
    /// the sentences the filter keeps: (voice, filter) they were found for
    sentence_hits: Option<(String, String, Vec<usize>)>,
}

enum DownloadState {
    Queued,
    Running { done: u64, total: Option<u64> },
    /// cancelled, until its task has stopped and removed what it stored
    Cancelling,
    Unpacking,
    Failed(String),
}

struct Download {
    installer: &'static Installer,
    state: DownloadState,
    /// stops it while it runs
    cancel: store::Cancel,
}

/// A column of the voice tables.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Col {
    Voice,
    Language,
    Gender,
    Size,
    Status,
}

/// How a voice table is sorted: a column, then language, then name.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Sort {
    col: Col,
    desc: bool,
}

impl Default for Sort {
    fn default() -> Sort {
        Sort { col: Col::Language, desc: false }
    }
}

/// A cell's sort key: text compares case-insensitively, numbers as numbers.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Num(u64),
    Text(String),
}

fn text_key(s: &str) -> Key {
    Key::Text(s.to_lowercase())
}

impl Sort {
    /// A column heading: clicking it sorts by that column, again reverses.
    fn header(&mut self, ui: &mut egui::Ui, title: &str, col: Col) {
        let on = self.col == col;
        let mark = match (on, self.desc) {
            (false, _) => "",
            (true, false) => " ⏶",
            (true, true) => " ⏷",
        };
        let text = RichText::new(format!("{title}{mark}")).small().color(if on { TEXT } else { MUTED });
        let r = ui.add(egui::Label::new(text).sense(egui::Sense::click())).on_hover_cursor(egui::CursorIcon::PointingHand);
        if r.clicked() {
            *self = Sort { col, desc: on && !self.desc };
        }
    }

    /// Sort `rows` by the chosen column, ties by language then name.
    fn apply<T>(&self, rows: &mut [T], key: impl Fn(&T, Col) -> Key) {
        rows.sort_by(|a, b| {
            let o = key(a, self.col).cmp(&key(b, self.col));
            (if self.desc { o.reverse() } else { o })
                .then_with(|| key(a, Col::Language).cmp(&key(b, Col::Language)))
                .then_with(|| key(a, Col::Voice).cmp(&key(b, Col::Voice)))
        });
    }
}

/// The desktop's own state: commands, and the original engine under Wine.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct Dev {
    /// Original engine (Wine) mode
    original: bool,
    engine_dir: String,
    /// other LTTS7 trees whose voices are listed too
    roots: Vec<std::path::PathBuf>,
    /// the pronunciation and piece picking picked (joining is `App::settings`)
    stages: crate::dev::pipeline::Stages,
    fon_command: String,
    /// the original engine's results and failures
    refs: std::collections::HashMap<TextKey, crate::dev::oracle::Reference>,
    ref_errors: std::collections::HashMap<TextKey, String>,
    /// a run of the original engine, one at a time
    job: Option<OracleJob>,
    report: Option<String>,
    extras: Vec<(String, usize, usize)>,
    /// the original engine's audio, ready to play
    original_samples: Option<(TextKey, Arc<Vec<f32>>)>,
}

/// A voice and a text it says.
#[cfg(not(target_arch = "wasm32"))]
type TextKey = (String, String);

/// The original engine running under Wine on some texts.
#[cfg(not(target_arch = "wasm32"))]
struct OracleJob {
    voice: String,
    texts: Vec<String>,
    handle: std::thread::JoinHandle<Result<Vec<crate::dev::oracle::Reference>, String>>,
    /// then compare its results with ours in a report
    compare: bool,
    /// `ctx.input(|i| i.time)` when it started
    started: f64,
}

/// `--screenshot`: where to save it, and what to show.
#[cfg(not(target_arch = "wasm32"))]
struct Screenshot {
    path: std::path::PathBuf,
    /// the speech shown playing at this moment (s)
    play_at: Option<f64>,
    /// the frame everything to show was ready
    settled: Option<u64>,
    asked: bool,
}

/// A link to a voice that isn't installed: the catalog entry and the link.
#[cfg(target_arch = "wasm32")]
struct Offer {
    installer: &'static Installer,
    route: Route,
}

/// The page's address (browser only).
#[cfg(target_arch = "wasm32")]
#[derive(Default)]
struct Links {
    /// the hash last read from or written to the address bar
    written: Option<String>,
    /// the next write replaces the history entry (it follows a link or back/forward)
    replace_next: bool,
    /// `ctx.input(|i| i.time)` of the last write
    last_write: f64,
    /// a link opened before the voices were listed
    pending: Option<Route>,
    /// a link waiting for its voice's download (followed if still on Voices when it lands)
    awaiting: Option<Route>,
    offer: Option<Offer>,
}

pub struct App {
    tab: Tab,
    tx: store::Tx,
    rx: Receiver<StoreEvent>,
    installed: Vec<InstalledVoice>,
    listed: bool,
    /// the voice picked (persisted), and the one loaded
    chosen: Option<String>,
    voice: Option<Arc<quoquendo::Voice>>,
    loading: Option<String>,
    downloads: Vec<Download>,
    unpacking: Option<String>,
    confirm_delete: Option<String>,
    filter: String,
    filter_language: String,
    filter_gender: String,
    sort_installed: Sort,
    sort_catalog: Sort,
    mirror_url: String,

    text: String,
    last_sample: String,
    settings: Settings,
    /// Advanced mode
    advanced: bool,
    /// the pronunciation's language when it isn't the voice's own (Advanced)
    language: Option<Language>,
    format: Format,
    /// the text analysed and rendered, and the last one that failed
    preview: Option<Preview>,
    failed: Option<(Want, String)>,
    /// what the preview should be made from, and since when
    pending: Option<(Want, f64)>,
    /// a preview being made
    #[cfg(not(target_arch = "wasm32"))]
    job: Option<(Want, std::thread::JoinHandle<Result<Preview, String>>)>,
    /// Speak was pressed before the preview had caught up with the text
    speak_queued: bool,
    player: Player,
    playing: Option<Track>,
    /// a moment the speech shows as playing without playing it (screenshots)
    pinned: Option<f64>,
    /// a moment shown under the pointer without a pointer (screenshots)
    hover_at: Option<f64>,
    /// the piece the Pieces list last scrolled to while speaking
    followed_piece: Option<usize>,
    explore: Explore,
    status: String,
    /// the status when it reports a success
    status_good: Option<String>,
    error: Option<String>,

    #[cfg(not(target_arch = "wasm32"))]
    dev: Dev,
    #[cfg(not(target_arch = "wasm32"))]
    screenshot: Option<Screenshot>,
    #[cfg(target_arch = "wasm32")]
    links: Links,
    frame_no: u64,
    quit_after: Option<u64>,
    logo: egui::TextureHandle,
}

/// What the app was started with.
#[derive(Default)]
pub struct Launch {
    #[cfg(not(target_arch = "wasm32"))]
    pub advanced: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub original: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub screenshot: Option<std::path::PathBuf>,
    #[cfg(not(target_arch = "wasm32"))]
    pub play_at: Option<f64>,
    #[cfg(not(target_arch = "wasm32"))]
    pub hover_at: Option<f64>,
}

/// The app's own dark look. egui keeps a light and a dark style and follows
/// the system's preference, so the theme is pinned: under a light
/// preference the light style (white bars, dark text) would show instead.
fn style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    let mut v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.extreme_bg_color = Color32::from_rgb(0x14, 0x13, 0x12);
    v.faint_bg_color = Color32::from_rgb(0x26, 0x25, 0x22);
    v.override_text_color = Some(TEXT);
    v.hyperlink_color = AMBER;
    v.selection.bg_fill = Color32::from_rgb(0x6b, 0x52, 0x24);
    v.selection.stroke = egui::Stroke::new(1.0f32, AMBER);
    v.window_stroke = egui::Stroke::new(1.0f32, STROKE);
    for w in [&mut v.widgets.inactive, &mut v.widgets.noninteractive] {
        w.bg_stroke = egui::Stroke::new(1.0f32, STROKE);
    }
    v.widgets.inactive.weak_bg_fill = WIDGET;
    v.widgets.inactive.bg_fill = WIDGET;
    v.widgets.hovered.weak_bg_fill = HOVER;
    v.widgets.hovered.bg_fill = HOVER;
    v.widgets.active.weak_bg_fill = HOVER;
    v.slider_trailing_fill = true;
    ctx.set_visuals(v);
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, launch: Launch) -> App {
        style(&cc.egui_ctx);
        // a screenshot shows the defaults and the launch options, not this
        // computer's saved settings
        #[cfg(not(target_arch = "wasm32"))]
        let fresh = launch.screenshot.is_some();
        #[cfg(target_arch = "wasm32")]
        let fresh = false;
        let get = |key: &str| cc.storage.filter(|_| !fresh).and_then(|s| s.get_string(key));
        let on = |key: &str| get(key).is_some_and(|v| v == "true");
        let (tx, rx) = channel();
        // the header's brand, uploaded once: shown 20 px tall in the bar
        let logo = {
            let image = image::load_from_memory(include_bytes!("../assets/brand/logo-header.png")).expect("logo-header.png").into_rgba8();
            let (w, h) = (image.width() as usize, image.height() as usize);
            let rgba = image.into_raw();
            cc.egui_ctx.load_texture("logo", egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba), egui::TextureOptions::LINEAR)
        };
        let mut app = App {
            tab: Tab::Speak,
            tx: store::Tx::new(tx, cc.egui_ctx.clone()),
            rx,
            installed: Vec::new(),
            listed: false,
            chosen: get("voice").filter(|v| !v.is_empty()),
            voice: None,
            loading: None,
            downloads: Vec::new(),
            unpacking: None,
            confirm_delete: None,
            filter: String::new(),
            filter_language: String::new(),
            filter_gender: String::new(),
            sort_installed: Sort::default(),
            sort_catalog: Sort::default(),
            mirror_url: get("mirror_url").unwrap_or_default(),
            text: get("text").unwrap_or_default(),
            last_sample: String::new(),
            settings: Settings::default(),
            advanced: on("advanced"),
            language: None,
            format: get("save_format").and_then(|e| Format::from_ext(&e)).unwrap_or(Format::Flac),
            preview: None,
            failed: None,
            pending: None,
            #[cfg(not(target_arch = "wasm32"))]
            job: None,
            speak_queued: false,
            player: Player::new(),
            playing: None,
            pinned: None,
            #[cfg(not(target_arch = "wasm32"))]
            hover_at: launch.hover_at,
            #[cfg(target_arch = "wasm32")]
            hover_at: None,
            followed_piece: None,
            explore: Explore::default(),
            status: "Looking for voices…".into(),
            status_good: None,
            error: None,
            #[cfg(not(target_arch = "wasm32"))]
            dev: Dev {
                original: on("original_engine"),
                ..Default::default()
            },
            #[cfg(not(target_arch = "wasm32"))]
            screenshot: launch.screenshot.clone().map(|path| Screenshot {
                path,
                play_at: launch.play_at,
                settled: None,
                asked: false,
            }),
            #[cfg(target_arch = "wasm32")]
            links: Links::default(),
            frame_no: 0,
            quit_after: None,
            logo,
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            app.dev.original |= launch.original;
            app.advanced |= launch.advanced || app.dev.original;
        }
        #[cfg(target_arch = "wasm32")]
        let _ = launch;
        app.relist();
        app
    }

    /// Advanced mode, on its own or under Original engine (Wine) mode.
    fn advanced(&self) -> bool {
        self.advanced || self.original()
    }

    /// Original engine (Wine) mode (desktop only).
    fn original(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        return self.dev.original;
        #[cfg(target_arch = "wasm32")]
        false
    }

    /// A link waiting for the voice list names a voice (browser only).
    fn link_names_voice(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        return self.links.pending.as_ref().is_some_and(|r| r.voice().is_some());
        #[cfg(not(target_arch = "wasm32"))]
        false
    }

    fn relist(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut roots = self.dev.roots.clone();
            if self.dev.original {
                for c in crate::dev::engines::engine_candidates() {
                    if loquendo::resolve_data_dir(&c).is_some() && !roots.contains(&c) {
                        roots.push(c);
                    }
                }
            }
            store::list(self.tx.clone(), roots);
        }
        #[cfg(target_arch = "wasm32")]
        store::list(self.tx.clone());
    }

    fn mirror(&self) -> Option<String> {
        catalog::effective_mirror(&self.mirror_url)
    }

    fn installed_voice(&self, name: &str) -> Option<&InstalledVoice> {
        self.installed.iter().find(|v| v.name.eq_ignore_ascii_case(name))
    }

    fn choose(&mut self, name: &str) {
        let Some(v) = self.installed_voice(name).cloned() else { return };
        self.player.stop();
        self.playing = None;
        self.chosen = Some(v.name.clone());
        self.voice = None;
        self.preview = None;
        self.failed = None;
        self.loading = Some(v.name.clone());
        self.status = format!("Loading {}…", v.name);
        store::load(&v, self.tx.clone());
    }

    /// Load the voice picked last time, else Roberto, else the first voice
    /// that can speak.
    fn pick_default(&mut self) {
        let count = self.installed.len();
        let pick = self
            .chosen
            .clone()
            .filter(|c| self.installed_voice(c).is_some())
            .or_else(|| self.installed.iter().find(|v| v.name == "Roberto" && v.can_speak()).map(|v| v.name.clone()))
            .or_else(|| self.installed.iter().find(|v| v.can_speak()).map(|v| v.name.clone()));
        match pick {
            Some(p) => self.choose(&p),
            None => {
                self.status = if count == 0 { String::new() } else { format!("{count} voices, none can speak yet") };
                if count == 0 {
                    self.tab = Tab::Voices;
                }
            }
        }
    }

    fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                StoreEvent::Progress { task, done, total } => {
                    if let Some(d) = self.downloads.iter_mut().find(|d| d.installer.label == task || d.installer.file == task) {
                        match d.state {
                            DownloadState::Unpacking => self.status = format!("Storing {task}… {}", catalog::size_label(done)),
                            DownloadState::Cancelling => {}
                            _ => d.state = DownloadState::Running { done, total },
                        }
                    } else {
                        self.status = format!("{task}: {}", catalog::size_label(done));
                    }
                }
                StoreEvent::Listed(list) => {
                    self.installed = list;
                    self.listed = true;
                    if self.voice.is_none() && self.loading.is_none() && !self.link_names_voice() {
                        self.pick_default();
                    }
                }
                StoreEvent::Loaded(voice) => {
                    if self.loading.as_deref().is_some_and(|l| l.eq_ignore_ascii_case(&voice.descriptor.name)) {
                        self.loading = None;
                        let sample = voice.descriptor.sample_text();
                        if self.text.trim().is_empty() || self.text == self.last_sample {
                            self.text = sample.clone();
                        }
                        self.last_sample = sample;
                        self.language = None;
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            self.dev.extras = loquendo::load_extras(&voice.descriptor.voice_path).sets;
                        }
                        self.error = voice.language_missing.as_ref().map(|t| format!("{} needs {t} language data", voice.descriptor.name));
                        self.status.clear();
                        self.voice = Some(Arc::new(*voice));
                    }
                }
                StoreEvent::Fetched { file, bytes } => {
                    // a picked file has no download entry; a cancelled
                    // download's bytes are dropped
                    match self.downloads.iter().position(|d| d.installer.file == file) {
                        Some(i) if matches!(self.downloads[i].state, DownloadState::Cancelling) => {
                            self.downloads.remove(i);
                            self.status = format!("Cancelled {file}");
                            continue;
                        }
                        Some(i) => self.downloads[i].state = DownloadState::Unpacking,
                        None => {}
                    }
                    let name = INSTALLERS.iter().find(|i| i.file == file).map_or(file.as_str(), |i| i.label);
                    self.status = format!("Unpacking {name}…");
                    self.unpacking = Some(file.clone());
                    store::import_installer(file, bytes, self.tx.clone());
                }
                StoreEvent::Cancelled(file) => {
                    self.downloads.retain(|d| d.installer.file != file);
                    self.status = format!("Cancelled {file}");
                }
                StoreEvent::Installed(voices) => {
                    self.unpacking = None;
                    self.downloads.retain(|d| matches!(d.state, DownloadState::Queued | DownloadState::Failed(_)));
                    self.status = format!("Installed {}", voices.join(", "));
                    self.error = None;
                    self.relist();
                }
                StoreEvent::InstallFailed(e) => {
                    self.unpacking = None;
                    if let Some(d) = self.downloads.iter_mut().find(|d| matches!(d.state, DownloadState::Running { .. } | DownloadState::Unpacking)) {
                        d.state = DownloadState::Failed(e.clone());
                    }
                    self.error = Some(e);
                }
                StoreEvent::Deleted(name) => {
                    if self.chosen.as_deref() == Some(name.as_str()) {
                        self.chosen = None;
                        self.voice = None;
                        self.preview = None;
                        self.failed = None;
                        self.player.stop();
                        self.playing = None;
                    }
                    self.status = format!("Deleted {name}");
                    self.relist();
                }
                StoreEvent::Failed(e) => {
                    self.loading = None;
                    self.error = Some(e);
                }
            }
            ctx.request_repaint();
        }
        // one download at a time, one unpack at a time
        let busy = self.unpacking.is_some()
            || self.downloads.iter().any(|d| matches!(d.state, DownloadState::Running { .. } | DownloadState::Cancelling | DownloadState::Unpacking));
        if !busy {
            if let Some(d) = self.downloads.iter_mut().find(|d| matches!(d.state, DownloadState::Queued)) {
                d.state = DownloadState::Running { done: 0, total: Some(d.installer.size) };
                d.cancel = store::Cancel::default();
                store::download(d.installer, catalog::effective_mirror(&self.mirror_url), self.tx.clone(), d.cancel.clone());
            }
        }
        if self.unpacking.is_some() || self.loading.is_some() || !self.downloads.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if self.playing.is_some() && self.pinned.is_none() && self.player.position().is_none() {
            self.playing = None;
        }
        if self.playing.is_some() && !self.player.is_paused() {
            ctx.request_repaint();
        }
    }

    fn can_speak(&self) -> Option<String> {
        let v = self.voice.as_ref()?;
        if let Some(u) = self.installed_voice(&v.descriptor.name).and_then(|i| i.unsupported.clone()) {
            return Some(u);
        }
        v.db.audio.is_none().then(|| v.db.audio_error.clone().unwrap_or_else(|| "no recordings".into()))
    }

    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("head").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add(egui::Image::new(&self.logo).max_height(20.0));
                ui.label(RichText::new(format!("v{VERSION}")).small().color(MUTED));
                ui.add_space(8.0);
                ui.selectable_value(&mut self.tab, Tab::Speak, "Speak");
                // a voice's recordings open from its row in Voices
                if ui.selectable_label(matches!(self.tab, Tab::Voices | Tab::Explore), "Voices").clicked() {
                    self.tab = Tab::Voices;
                }
                #[cfg(not(target_arch = "wasm32"))]
                if self.dev.original {
                    ui.separator();
                    ui.label("LTTS7 folder");
                    let width = (ui.available_width() - 160.0).clamp(120.0, 320.0);
                    let edit = ui.add(egui::TextEdit::singleline(&mut self.dev.engine_dir).desired_width(width).hint_text("LTTS7 folder or installer .exe"));
                    if ui.button("Load").clicked() || (edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                        self.load_engine_dir();
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.menu_button("Settings", |ui| {
                        let advanced = ui.checkbox(&mut self.advanced, "Advanced").changed();
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let original = ui.checkbox(&mut self.dev.original, "Original engine (Wine)").changed();
                            if advanced && !self.advanced {
                                self.dev.original = false;
                            }
                            if original && self.dev.original {
                                self.advanced = true;
                            }
                            if advanced || original {
                                self.relist();
                            }
                        }
                        #[cfg(target_arch = "wasm32")]
                        let _ = advanced;
                        ui.separator();
                        ui.label("Voice mirror URL");
                        let fallback = catalog::effective_mirror("").unwrap_or_else(|| "none: archive.org only".into());
                        ui.add(egui::TextEdit::singleline(&mut self.mirror_url).desired_width(260.0).hint_text(fallback));
                        ui.separator();
                        ui.hyperlink_to("About quoquendo", "https://github.com/depau/quoquendo");
                    });
                });
            });
            ui.add_space(2.0);
        });
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if self.unpacking.is_some() || self.loading.is_some() {
                    ui.spinner();
                }
                let blocked = self.can_speak().filter(|_| self.tab == Tab::Speak);
                let said = if let Some(e) = &self.error {
                    ui.label(RichText::new(e).color(RED));
                    if ui.small_button("Dismiss").clicked() {
                        self.error = None;
                    }
                    true
                } else if let (Some(b), Some(v)) = (blocked, &self.voice) {
                    ui.label(RichText::new(format!("{} can't speak: {b}", v.descriptor.name)).color(AMBER));
                    true
                } else if !self.status.is_empty() {
                    let good = self.status_good.as_ref() == Some(&self.status);
                    ui.label(RichText::new(&self.status).color(if good { GREEN } else { MUTED }));
                    true
                } else {
                    false
                };
                if said {
                    ui.label(RichText::new("·").color(MUTED));
                }
                let stored: u64 = self.installed.iter().filter(|v| v.root.is_none()).map(|v| v.bytes).sum();
                let place = if cfg!(target_arch = "wasm32") { "in this browser" } else { "used" };
                ui.label(
                    RichText::new(format!("{} voices installed · {} {place}", self.installed.len(), catalog::size_label(stored)))
                        .small()
                        .color(MUTED),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mode = if self.original() {
                        "Original engine (Wine)"
                    } else if self.advanced {
                        "Advanced"
                    } else {
                        ""
                    };
                    ui.label(RichText::new(mode).small().color(MUTED));
                });
            });
        });
    }

    fn voices_tab(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Installed");
            ui.label(RichText::new(format!("· {} voices", self.installed.len())).color(MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add_enabled(self.unpacking.is_none(), egui::Button::new("Import installer .exe…")).clicked() {
                    store::pick_installer(self.tx.clone());
                }
                if let Some(u) = &self.unpacking {
                    ui.spinner();
                    ui.label(RichText::new(format!("Unpacking {u}…")).color(AMBER));
                }
            });
        });
        egui::Frame::group(ui.style()).fill(GROUND).show(ui, |ui| {
            ui.set_width(ui.available_width());
            if self.installed.is_empty() {
                ui.label(RichText::new(if self.listed { "No voices installed yet" } else { "Looking…" }).color(MUTED));
                return;
            }
            let mut delete = None;
            let mut keep = false;
            let mut choose = None;
            let mut explore = None;
            // a voice missing its language: the mirror holds the language
            // folder, and a mirror download fetches only what isn't stored
            let mut fetch_language: Option<&'static Installer> = None;
            let has_mirror = self.mirror().is_some();
            let queued: Vec<&str> = self.downloads.iter().map(|d| d.installer.file).collect();
            let mut rows: Vec<&InstalledVoice> = self.installed.iter().collect();
            self.sort_installed.apply(&mut rows, |v, col| match col {
                Col::Voice => text_key(&v.name),
                Col::Language => text_key(&v.mother_tongue),
                Col::Gender => text_key(&v.gender),
                // ready, then needing language data, then unsupported
                Col::Status | Col::Size => Key::Num(if v.unsupported.is_some() { 2 } else { v.language_missing.is_some() as u64 }),
            });
            egui::Grid::new("installed").num_columns(5).striped(true).spacing([18.0, 6.0]).show(ui, |ui| {
                for (h, col) in [("Voice", Col::Voice), ("Language", Col::Language), ("Gender", Col::Gender), ("Status", Col::Status)] {
                    self.sort_installed.header(ui, h, col);
                }
                ui.label("");
                ui.end_row();
                for v in rows {
                    ui.label(&v.name);
                    ui.label(&v.mother_tongue);
                    ui.label(RichText::new(&v.gender).color(MUTED));
                    match (&v.unsupported, &v.language_missing) {
                        (Some(u), _) => ui.label(RichText::new(format!("{u} — unsupported yet")).color(MUTED)),
                        (None, Some(t)) => ui.label(RichText::new(format!("needs {t} language data")).color(AMBER)),
                        (None, None) => ui.label(RichText::new(format!("Ready · {}", catalog::size_label(v.bytes))).color(GREEN)),
                    };
                    ui.horizontal(|ui| {
                        if self.chosen.as_deref().is_some_and(|c| c.eq_ignore_ascii_case(&v.name)) {
                            ui.add_enabled(false, egui::Button::new("In use"));
                        } else if ui.add_enabled(v.can_speak(), egui::Button::new("Use")).clicked() {
                            choose = Some(v.name.clone());
                        }
                        if v.can_speak() && ui.button("Explore").on_hover_text("Hear this voice's recordings, phone by phone and sentence by sentence").clicked() {
                            explore = Some(v.name.clone());
                        }
                        let installer = catalog::installer_for(&v.name).filter(|i| !queued.contains(&i.file));
                        if let (Some(inst), Some(t), None, true) = (installer, &v.language_missing, &v.root, has_mirror) {
                            if ui.button(format!("Get {t} data")).clicked() {
                                fetch_language = Some(inst);
                            }
                        }
                        if v.root.is_some() {
                            ui.label(RichText::new("other installation").small().color(MUTED));
                        } else if self.confirm_delete.as_deref() == Some(v.name.as_str()) {
                            ui.label(RichText::new(format!("Delete {}? {}", v.name, catalog::size_label(v.bytes))).color(RED));
                            if ui.button(RichText::new("Delete").color(RED)).clicked() {
                                delete = Some(v.name.clone());
                            }
                            if ui.button("Keep").clicked() {
                                keep = true;
                            }
                        } else if ui.button("Delete").clicked() {
                            self.confirm_delete = Some(v.name.clone());
                        }
                    });
                    ui.end_row();
                }
            });
            if keep {
                self.confirm_delete = None;
            }
            if let Some(inst) = fetch_language {
                self.queue_download(inst);
            }
            if let Some(name) = delete {
                self.confirm_delete = None;
                store::delete(name, self.tx.clone());
            }
            if let Some(name) = choose {
                self.choose(&name);
                self.tab = Tab::Speak;
            }
            if let Some(name) = explore {
                if self.voice.as_ref().is_none_or(|v| !v.descriptor.name.eq_ignore_ascii_case(&name)) {
                    self.choose(&name);
                }
                self.tab = Tab::Explore;
            }
        });

        if !self.downloads.is_empty() {
            ui.add_space(10.0);
            ui.heading("Downloads");
            let mut remove = None;
            let mut retry = None;
            let mut stop = None;
            egui::Frame::group(ui.style()).fill(GROUND).show(ui, |ui| {
                ui.set_width(ui.available_width());
                egui::Grid::new("downloads").num_columns(4).spacing([18.0, 6.0]).show(ui, |ui| {
                    for (i, d) in self.downloads.iter().enumerate() {
                        ui.label(d.installer.label);
                        ui.label(RichText::new(format!("{} · {}", d.installer.language, catalog::size_label(d.installer.size))).color(MUTED));
                        match &d.state {
                            DownloadState::Queued => {
                                ui.label(RichText::new("Queued").color(MUTED));
                            }
                            DownloadState::Running { done, total } => {
                                let total = total.unwrap_or(d.installer.size).max(1);
                                ui.add(
                                    egui::ProgressBar::new(*done as f32 / total as f32)
                                        .desired_width(320.0)
                                        .fill(AMBER)
                                        .text(format!("{} of {}", catalog::size_label(*done), catalog::size_label(total))),
                                );
                            }
                            DownloadState::Cancelling => {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label(RichText::new("Cancelling…").color(MUTED));
                                });
                            }
                            DownloadState::Unpacking => {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label("Unpacking…");
                                });
                            }
                            DownloadState::Failed(e) => {
                                ui.label(RichText::new(format!("{e} — nothing was installed")).color(RED));
                            }
                        }
                        ui.horizontal(|ui| match d.state {
                            DownloadState::Queued => {
                                if ui.button("Cancel").clicked() {
                                    remove = Some(i);
                                }
                            }
                            DownloadState::Running { .. } => {
                                if ui.button("Cancel").on_hover_text("Stop this download; nothing of it is kept").clicked() {
                                    stop = Some(i);
                                }
                            }
                            DownloadState::Failed(_) => {
                                if ui.button("Retry").clicked() {
                                    retry = Some(i);
                                }
                                if ui.button("Dismiss").clicked() {
                                    remove = Some(i);
                                }
                            }
                            _ => {}
                        });
                        ui.end_row();
                    }
                });
            });
            if let Some(i) = retry {
                self.downloads[i].state = DownloadState::Queued;
            }
            if let Some(i) = stop {
                self.downloads[i].cancel.cancel();
                self.downloads[i].state = DownloadState::Cancelling;
            }
            if let Some(i) = remove {
                self.downloads.remove(i);
            }
        }

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.heading("Catalog");
            ui.label(RichText::new(format!("· {} voices", INSTALLERS.len())).color(MUTED));
        });
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("Filter by name or language").desired_width(220.0));
            let mut langs: Vec<&str> = INSTALLERS.iter().map(|i| i.language).collect();
            langs.sort();
            langs.dedup();
            egui::ComboBox::from_id_salt("lang_filter")
                .selected_text(if self.filter_language.is_empty() { "All languages" } else { &self.filter_language })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.filter_language, String::new(), "All languages");
                    for l in langs {
                        ui.selectable_value(&mut self.filter_language, l.to_string(), l);
                    }
                });
            egui::ComboBox::from_id_salt("gender_filter")
                .selected_text(if self.filter_gender.is_empty() { "Any gender" } else { &self.filter_gender })
                .show_ui(ui, |ui| {
                    for g in ["", "Female", "Male"] {
                        ui.selectable_value(&mut self.filter_gender, g.to_string(), if g.is_empty() { "Any gender" } else { g });
                    }
                });
        });
        let needle = self.filter.to_lowercase();
        let mut shown: Vec<&'static Installer> = INSTALLERS
            .iter()
            .filter(|i| needle.is_empty() || i.label.to_lowercase().contains(&needle) || i.language.to_lowercase().contains(&needle) || i.voice.to_lowercase().contains(&needle))
            .filter(|i| self.filter_language.is_empty() || i.language == self.filter_language)
            .filter(|i| self.filter_gender.is_empty() || i.gender == self.filter_gender)
            .collect();
        let installed = |i: &Installer| self.installed.iter().any(|v| v.root.is_none() && v.name.eq_ignore_ascii_case(i.voice));
        let pending = |i: &Installer| self.downloads.iter().any(|d| d.installer.file == i.file);
        self.sort_catalog.apply(&mut shown, |i, col| match col {
            Col::Voice => text_key(i.label),
            Col::Language => text_key(i.language),
            Col::Gender => text_key(i.gender),
            Col::Size => Key::Num(i.size),
            // installed, then downloading, then to download
            Col::Status => Key::Num(if installed(i) { 0 } else if pending(i) { 1 } else { 2 }),
        });
        let can_fetch = !cfg!(target_arch = "wasm32") || self.mirror().is_some();
        let mut queue = None;
        egui::Frame::group(ui.style()).fill(GROUND).show(ui, |ui| {
            ui.set_width(ui.available_width());
            egui::Grid::new("catalog").num_columns(5).striped(true).spacing([18.0, 5.0]).show(ui, |ui| {
                for (h, col) in [("Voice", Col::Voice), ("Language", Col::Language), ("Gender", Col::Gender), ("Size", Col::Size), ("Status", Col::Status)] {
                    self.sort_catalog.header(ui, h, col);
                }
                ui.end_row();
                for inst in shown {
                    // a voice no module can read: listed, but not offered
                    let dead = catalog::unsupported(inst.language);
                    // weaker than the theme's text, whatever that is
                    let ink = |t: &str| if dead.is_some() { RichText::new(t).weak() } else { RichText::new(t) };
                    ui.label(ink(inst.label));
                    ui.label(ink(inst.language));
                    ui.label(if dead.is_some() { RichText::new(inst.gender).weak() } else { RichText::new(inst.gender).color(MUTED) });
                    ui.label(ink(&catalog::size_label(inst.size)).monospace());
                    if installed(inst) {
                        ui.label(RichText::new("Installed").color(GREEN));
                    } else if pending(inst) {
                        ui.label(RichText::new("In downloads").color(MUTED));
                    } else if let Some(u) = &dead {
                        ui.label(RichText::new("Won't speak").weak().italics())
                            .on_hover_text(format!("{u}: quoquendo has no {} language data to read this voice with", inst.language));
                    } else if ui
                        .add_enabled(can_fetch, egui::Button::new("Download"))
                        .on_disabled_hover_text("Browsers can't fetch installers from archive.org: set a voice mirror in Settings, or import a downloaded installer.")
                        .clicked()
                    {
                        queue = Some(inst);
                    }
                    ui.end_row();
                }
            });
        });
        if let Some(inst) = queue {
            self.queue_download(inst);
        }
    }

    /// Add an installer to the downloads, unless it's there already.
    fn queue_download(&mut self, inst: &'static Installer) {
        if self.downloads.iter().all(|d| d.installer.file != inst.file) {
            self.downloads.push(Download {
                installer: inst,
                state: DownloadState::Queued,
                cancel: store::Cancel::default(),
            });
        }
    }
}

/// The recordings explorer and playing stretches of the recordings.
impl App {
    /// Play samples `from..to` of the voice's recordings as they are.
    fn play_span(&mut self, unit: u32, from: u32, to: u32) {
        // Gilded phrase databases have their own recordings
        let Some(bank) = self.voice.as_ref().and_then(|v| v.db.bank(unit)) else {
            self.error = Some("This voice's recordings can't be decoded.".into());
            return;
        };
        let pcm = bank.read(from as usize, to.saturating_sub(from) as usize);
        let samples: Vec<f32> = pcm.iter().map(|&s| s as f32 / 32768.0 * self.settings.volume).collect();
        match self.player.play(&samples, bank.rate, 0.0) {
            Ok(()) => self.playing = Some(Track::Clip),
            Err(e) => self.error = Some(e),
        }
    }

    /// Each phone of the voice with every recorded take of it.
    fn explore_index(&mut self) {
        let Some(v) = &self.voice else { return };
        if self.explore.index.as_ref().is_some_and(|(n, _)| *n == v.descriptor.name) {
            return;
        }
        let mut map: std::collections::BTreeMap<String, Vec<(u32, u16)>> = Default::default();
        for (u, unit) in v.db.units.iter().enumerate() {
            for (i, p) in unit.phones.iter().enumerate().filter(|(_, p)| p.timed) {
                map.entry(p.sym.clone()).or_default().push((u as u32, i as u16));
            }
        }
        // a first build (or one a link cleared) keeps the phone and sentence
        // picked; another voice's starts afresh
        let fresh = self.explore.index.is_none();
        self.explore = Explore {
            index: Some((v.descriptor.name.clone(), map.into_iter().collect())),
            phones: self.explore.phones,
            reveal: fresh && self.explore.reveal,
            phone: if fresh { self.explore.phone.take() } else { None },
            unit: if fresh { self.explore.unit } else { None },
            ..Default::default()
        };
    }

    /// Open the loaded voice's recorded sentence `unit` in Explore, the
    /// sentence list scrolled to it.
    pub(crate) fn open_sentence(&mut self, unit: usize) {
        let Some(v) = &self.voice else { return };
        // an index of another voice would drop the selection: rebuild it
        if self.explore.index.as_ref().is_some_and(|(n, _)| *n != v.descriptor.name) {
            self.explore.index = None;
        }
        self.tab = Tab::Explore;
        self.explore.phones = false;
        self.explore.unit = Some(unit);
        self.explore.sentence_filter.clear();
        self.explore.reveal = true;
    }

    /// The recorded sentences the sentence filter keeps: a sentence number
    /// (`#12`, `12`) keeps the sentences whose number starts with it, phones
    /// (`a r t`) the sentences that say them in that order, word breaks
    /// ignored.
    fn explore_sentence_hits(&mut self) {
        let Some(v) = &self.voice else { return };
        let filter = self.explore.sentence_filter.trim().to_string();
        if self.explore.sentence_hits.as_ref().is_some_and(|(n, f, _)| *n == v.descriptor.name && *f == filter) {
            return;
        }
        let number = filter.trim_start_matches('#');
        let units = &v.db.units;
        let hits: Vec<usize> = if filter.is_empty() {
            (0..units.len()).collect()
        } else if !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()) {
            (0..units.len()).filter(|u| u.to_string().starts_with(number)).collect()
        } else {
            let want = format!(" {} ", filter.split_whitespace().collect::<Vec<_>>().join(" "));
            (0..units.len())
                .filter(|&u| {
                    let said: Vec<&str> = units[u].phones.iter().filter(|p| p.timed).map(|p| p.sym.as_str()).collect();
                    format!(" {} ", said.join(" ")).contains(&want)
                })
                .collect()
        };
        self.explore.sentence_hits = Some((v.descriptor.name.clone(), filter, hits));
    }

    fn explore_tab(&mut self, ui: &mut egui::Ui) {
        if let Some(name) = &self.loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!("Loading {name}…"));
            });
            return;
        }
        if self.voice.is_none() {
            if ui.button("◀ Voices").clicked() {
                self.tab = Tab::Voices;
            }
            ui.label(RichText::new("Pick a voice in Voices and press Explore to hear its recordings.").color(MUTED));
            return;
        }
        self.explore_index();
        self.explore_sentence_hits();
        let Some(voice) = &self.voice else { return };
        let db = &voice.db;
        let rate = db.audio_rate() as f32;
        let ms = |a: u32, b: u32| (b.saturating_sub(a)) as f32 * 1000.0 / rate;
        let Some((_, index)) = &self.explore.index else { return };
        let mut play: Option<(u32, u32, u32)> = None;
        let mut open_unit: Option<Option<usize>> = None;
        let mut pick_phone: Option<String> = None;
        let mut page = self.explore.page;
        let mut back = false;
        ui.horizontal(|ui| {
            back = ui.button("◀ Voices").clicked();
            ui.heading(format!("{}'s recordings", voice.descriptor.name));
            let secs = db.audio.as_ref().map(|a| a.total_samples() / a.rate as usize).unwrap_or(0);
            ui.label(
                RichText::new(format!(
                    "· {} recorded sentences · {} phones · {} takes · {}:{:02}:{:02} of audio",
                    db.units.len(),
                    index.len(),
                    db.instance_count(),
                    secs / 3600,
                    secs / 60 % 60,
                    secs % 60
                ))
                .color(MUTED),
            );
        });
        if db.audio.is_none() {
            ui.label(RichText::new("This voice's recordings can't be played yet.").color(AMBER));
        }
        ui.separator();
        let height = ui.available_height();
        ui.horizontal_top(|ui| {
            // a list to pick from: the phones, with how often each was
            // recorded, or the recorded sentences
            ui.vertical(|ui| {
                let width = if self.explore.phones { 170.0 } else { 380.0 };
                ui.set_width(width);
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.explore.phones, false, "Sentences");
                    ui.selectable_value(&mut self.explore.phones, true, "Phones");
                });
                if self.explore.phones {
                    ui.add(egui::TextEdit::singleline(&mut self.explore.filter).hint_text("Filter phones").desired_width(width - 10.0));
                    egui::ScrollArea::vertical().id_salt("phones").max_height(height - 70.0).show(ui, |ui| {
                        for (sym, takes) in index.iter().filter(|(s, _)| self.explore.filter.is_empty() || s.contains(self.explore.filter.as_str())) {
                            let chosen = self.explore.phone.as_deref() == Some(sym.as_str()) && self.explore.unit.is_none();
                            let label = RichText::new(format!("{sym:<6} {:>6}", takes.len())).monospace();
                            if ui.selectable_label(chosen, label).clicked() {
                                pick_phone = Some(sym.clone());
                            }
                        }
                    });
                    return;
                }
                ui.add(
                    egui::TextEdit::singleline(&mut self.explore.sentence_filter)
                        .hint_text("Phones in order (a r t) or a number")
                        .desired_width(width - 10.0),
                );
                let hits: &[usize] = self.explore.sentence_hits.as_ref().map_or(&[], |(_, _, h)| h.as_slice());
                ui.label(RichText::new(format!("{} of {} sentences", hits.len(), db.units.len())).small().color(MUTED));
                let row = ui.text_style_height(&egui::TextStyle::Body) + 4.0;
                let digits = db.units.len().saturating_sub(1).to_string().len();
                let mut list = egui::ScrollArea::vertical().id_salt("sentences").max_height(height - 90.0);
                if std::mem::take(&mut self.explore.reveal) {
                    if let Some(at) = self.explore.unit.and_then(|u| hits.iter().position(|&h| h == u)) {
                        let step = row + ui.spacing().item_spacing.y;
                        list = list.vertical_scroll_offset((at as f32 * step - (height - 90.0) / 2.0).max(0.0));
                    }
                }
                list.show_rows(ui, row, hits.len(), |ui, rows| {
                    for &u in &hits[rows] {
                        let unit = &db.units[u];
                        let said: Vec<&str> = unit.phones.iter().filter(|p| p.timed).map(|p| p.sym.as_str()).collect();
                        // a question or an exclamation is marked after the number
                        let mood = match unit.mood {
                            Some('I') => " ? ",
                            Some('E') => " ! ",
                            _ => "   ",
                        };
                        ui.horizontal(|ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                            let timed = || unit.phones.iter().filter(|p| p.timed);
                            if let (Some(a), Some(b)) = (timed().next(), timed().last()) {
                                if ui.small_button("▶").on_hover_text("Play this sentence").clicked() {
                                    play = Some((u as u32, a.start, b.end));
                                }
                            }
                            let text = RichText::new(format!("#{u:<digits$}{mood}{}", said.join(" "))).monospace();
                            if ui.selectable_label(self.explore.unit == Some(u), text).clicked() {
                                open_unit = Some(Some(u));
                            }
                        });
                    }
                });
            });
            ui.separator();
            ui.vertical(|ui| {
                if let Some(u) = self.explore.unit.filter(|&u| u < db.units.len()) {
                    // one recorded sentence: each phone plays on its own
                    let unit = &db.units[u];
                    let timed: Vec<&quoquendo::loquendo::Phone> = unit.phones.iter().filter(|p| p.timed).collect();
                    ui.horizontal(|ui| {
                        if ui.button("◀ Back").clicked() {
                            open_unit = Some(None);
                        }
                        if ui.add_enabled(u > 0, egui::Button::new("◀")).on_hover_text("Previous sentence").clicked() {
                            open_unit = Some(Some(u - 1));
                        }
                        ui.label(RichText::new(format!("Sentence #{u}")).strong());
                        if ui.add_enabled(u + 1 < db.units.len(), egui::Button::new("▶")).on_hover_text("Next sentence").clicked() {
                            open_unit = Some(Some(u + 1));
                        }
                        let mood = match unit.mood {
                            Some('I') => "question",
                            Some('E') => "exclamation",
                            _ => "statement",
                        };
                        ui.label(RichText::new(format!("· {mood} · {} phones", timed.len())).color(MUTED));
                        if let (Some(a), Some(b)) = (timed.first(), timed.last()) {
                            if ui.button(format!("▶ whole sentence ({:.2} s)", ms(a.start, b.end) / 1000.0)).clicked() {
                                play = Some((u as u32, a.start, b.end));
                            }
                        }
                    });
                    ui.label(RichText::new(unit.symbols.join(" ")).monospace().color(MUTED));
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        for p in &timed {
                            if p.word_start {
                                ui.label(RichText::new("|").color(MUTED));
                            }
                            let b = ui
                                .button(RichText::new(&p.sym).monospace().color(GREEN))
                                .on_hover_text(format!("{:.0} ms · {}–{} Hz · click to play", ms(p.start, p.end), p.pitch_l, p.pitch_r));
                            if b.clicked() {
                                play = Some((u as u32, p.start, p.end));
                            }
                        }
                    });
                } else if let Some((sym, takes)) = self.explore.phone.as_ref().and_then(|s| index.iter().find(|(x, _)| x == s)) {
                    // every take of one phone, with its neighbours
                    const PAGE: usize = 100;
                    let pages = takes.len().div_ceil(PAGE).max(1);
                    page = page.min(pages - 1);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("“{sym}”")).monospace().strong().color(AMBER));
                        ui.label(RichText::new(format!("· {} takes", takes.len())).color(MUTED));
                        if ui.add_enabled(page > 0, egui::Button::new("◀")).clicked() {
                            page -= 1;
                        }
                        ui.label(format!("{} / {pages}", page + 1));
                        if ui.add_enabled(page + 1 < pages, egui::Button::new("▶")).clicked() {
                            page += 1;
                        }
                    });
                    egui::ScrollArea::vertical().id_salt("takes").max_height(height - 60.0).show(ui, |ui| {
                        egui::Grid::new("takes_grid").num_columns(7).striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
                            for h in ["", "", "in context", "sentence", "length", "pitch", ""] {
                                ui.label(RichText::new(h).small().color(MUTED));
                            }
                            ui.end_row();
                            for &(u, i) in takes.iter().skip(page * PAGE).take(PAGE) {
                                let phones = &db.units[u as usize].phones;
                                let p = &phones[i as usize];
                                let prev = phones[..i as usize].iter().rev().find(|q| q.timed);
                                let next = phones[i as usize + 1..].iter().find(|q| q.timed);
                                if ui.small_button("▶").on_hover_text("Play this take").clicked() {
                                    play = Some((u, p.start, p.end));
                                }
                                if ui.small_button("▶▶").on_hover_text("Play it with its neighbours").clicked() {
                                    play = Some((u, prev.map(|q| q.start).unwrap_or(p.start), next.map(|q| q.end).unwrap_or(p.end)));
                                }
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    ui.label(RichText::new(prev.map(|q| q.sym.as_str()).unwrap_or("#")).monospace().color(MUTED));
                                    ui.label(RichText::new(&p.sym).monospace().color(GREEN));
                                    ui.label(RichText::new(next.map(|q| q.sym.as_str()).unwrap_or("@")).monospace().color(MUTED));
                                });
                                if ui.link(RichText::new(format!("#{u}")).monospace().color(ui.visuals().hyperlink_color)).clicked() {
                                    open_unit = Some(Some(u as usize));
                                }
                                ui.label(RichText::new(format!("{:.0} ms", ms(p.start, p.end))).monospace());
                                ui.label(RichText::new(format!("{}–{} Hz", p.pitch_l, p.pitch_r)).monospace());
                                ui.label(RichText::new(if p.word_start { "word start" } else { "" }).small().color(MUTED));
                                ui.end_row();
                            }
                        });
                    });
                } else {
                    ui.label(RichText::new("Pick a phone to hear every take of it, or a recorded sentence to hear it phone by phone.").color(MUTED));
                }
            });
        });
        self.explore.page = page;
        if back {
            self.tab = Tab::Voices;
        }
        if let Some(s) = pick_phone {
            self.explore.phone = Some(s);
            self.explore.unit = None;
            self.explore.page = 0;
        }
        if let Some(u) = open_unit {
            self.explore.unit = u;
        }
        if let Some((unit, a, b)) = play {
            self.play_span(unit, a, b);
        }
    }
}

fn voice_label(v: &InstalledVoice) -> String {
    let lang = Language::from_voice(&v.mother_tongue, "").label();
    let lang = if lang == Language::Generic.label() { v.mother_tongue.as_str() } else { lang };
    format!("{} — {lang} ({})", v.name, if v.gender.is_empty() { "?" } else { &v.gender })
}

#[cfg(not(target_arch = "wasm32"))]
impl App {
    fn load_engine_dir(&mut self) {
        let path = std::path::PathBuf::from(self.dev.engine_dir.trim());
        if crate::install::is_installer_path(&path) {
            let tx = self.tx.clone();
            self.unpacking = Some(path.display().to_string());
            std::thread::spawn(move || {
                let _ = tx.send(match crate::install::install_voice(&path) {
                    Ok(v) => StoreEvent::Installed(v),
                    Err(e) => StoreEvent::InstallFailed(e),
                });
            });
            return;
        }
        match loquendo::resolve_data_dir(&path).and_then(|d| d.parent().map(|p| p.to_path_buf())) {
            Some(root) => {
                if !self.dev.roots.contains(&root) {
                    self.dev.roots.push(root.clone());
                }
                self.status = format!("Reading voices from {}", root.display());
                self.relist();
            }
            None => self.error = Some(format!("{} is not an LTTS7 folder", path.display())),
        }
    }

    fn report_window(&mut self, ctx: &egui::Context) {
        let Some(report) = self.dev.report.clone() else { return };
        let mut open = true;
        egui::Window::new("Compared with the original engine").open(&mut open).default_size([760.0, 420.0]).show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.label(RichText::new(report).monospace().size(11.0));
            });
        });
        if !open {
            self.dev.report = None;
        }
    }

    /// `--screenshot`: keep drawing while unfocused; once the preview (and
    /// in Original engine mode the original's result) is ready, show it
    /// playing at `--play-at` if asked, then save the picture and quit.
    fn screenshot(&mut self, ctx: &egui::Context) {
        let Some(shot) = &mut self.screenshot else { return };
        // drawn at the mockups' scale, whatever the screen's
        if ctx.pixels_per_point() != 1.0 {
            ctx.set_pixels_per_point(1.0);
        }
        let original_done = !self.dev.original || self.dev.job.is_none() && self.preview.as_ref().is_some_and(|p| {
            let key = (p.want.voice.clone(), p.want.text.clone());
            self.dev.refs.contains_key(&key) || self.dev.ref_errors.contains_key(&key)
        });
        let fresh = self.voice.is_some() && self.loading.is_none() && self.job.is_none() && self.pending.is_none() && self.preview.is_some();
        if shot.settled.is_none() && (fresh && original_done || ctx.input(|i| i.time) > 120.0) {
            shot.settled = Some(self.frame_no);
            if let Some(t) = shot.play_at {
                self.pinned = Some(t);
                self.playing = Some(Track::Ours);
            }
        }
        if !shot.asked && shot.settled.is_some_and(|f| self.frame_no > f + 30) {
            shot.asked = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
        }
        if let Some(image) = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        }) {
            let path = shot.path.clone();
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let (w, h) = (image.size[0], image.size[1]);
            let pixels: Vec<u8> = image.pixels.iter().flat_map(|c| [c.r(), c.g(), c.b(), c.a()]).collect();
            match image::save_buffer(&path, &pixels, w as u32, h as u32, image::ColorType::Rgba8) {
                Ok(_) => eprintln!("saved the screenshot to {}", path.display()),
                Err(e) => eprintln!("screenshot: {e}"),
            }
            self.quit_after = Some(self.frame_no + 2);
        }
        ctx.request_repaint();
    }
}

/// Deep links (browser only): the address bar follows the page shown, and
/// an address opened, edited or reached with back/forward opens its page.
#[cfg(target_arch = "wasm32")]
impl App {
    /// Show the page a link names; a voice that isn't installed but is in
    /// the catalog is offered for download.
    fn open_route(&mut self, mut r: Route) {
        self.links.awaiting = None;
        self.links.offer = None;
        if !self.listed {
            self.links.pending = Some(r);
            return;
        }
        if let Some(name) = r.voice().map(str::to_string) {
            let Some(installed) = self.installed_voice(&name).map(|v| v.name.clone()) else {
                match INSTALLERS.iter().find(|i| i.voice.eq_ignore_ascii_case(&name)) {
                    Some(installer) => self.links.offer = Some(Offer { installer, route: r }),
                    None => self.error = Some(format!("No voice called “{name}”: it isn't installed and isn't in the catalog")),
                }
                if self.voice.is_none() && self.loading.is_none() {
                    self.pick_default();
                }
                return;
            };
            match &mut r {
                Route::Speak { voice, .. } => *voice = Some(installed.clone()),
                Route::Explore { voice, .. } => *voice = installed.clone(),
                Route::Voices => {}
            }
            self.ensure_chosen(&installed);
        }
        match r {
            Route::Voices => self.tab = Tab::Voices,
            Route::Speak { text, .. } => {
                self.tab = Tab::Speak;
                let text = match text {
                    Some(t) => {
                        // the voice on its way must not swap the linked text
                        // for its sample because it equals the old voice's
                        if self.loading.is_some() {
                            self.last_sample.clear();
                        }
                        t
                    }
                    None if self.voice.is_some() && self.loading.is_none() => self.last_sample.clone(),
                    // filled with the sample once the voice has loaded
                    None => String::new(),
                };
                if text != self.text {
                    self.text = text;
                    if self.loading.is_none() {
                        self.player.stop();
                    }
                }
            }
            Route::Explore { voice, phones, phone, sentence } => {
                self.tab = Tab::Explore;
                if self.explore.index.as_ref().is_some_and(|(n, _)| n.eq_ignore_ascii_case(&voice)) {
                    if self.explore.phone != phone {
                        self.explore.page = 0;
                    }
                    self.explore.phones = phones;
                    self.explore.phone = phone;
                    self.explore.unit = sentence;
                } else {
                    // `explore_index` keeps the selection when it builds the index
                    self.explore = Explore { phones, phone, unit: sentence, reveal: sentence.is_some(), ..Default::default() };
                }
            }
        }
    }

    /// Load `name` unless it is loaded or loading.
    fn ensure_chosen(&mut self, name: &str) {
        let loaded = self.voice.as_ref().is_some_and(|v| v.descriptor.name.eq_ignore_ascii_case(name));
        let coming = self.loading.as_deref().is_some_and(|l| l.eq_ignore_ascii_case(name));
        if !loaded && !coming {
            self.choose(name);
        }
    }

    /// The page shown.
    fn current_route(&self) -> Route {
        match (self.tab, &self.chosen) {
            (Tab::Speak, _) => Route::Speak {
                voice: self.chosen.clone(),
                text: (!self.text.trim().is_empty() && self.text != self.last_sample).then(|| self.text.clone()),
            },
            (Tab::Explore, Some(voice)) => Route::Explore {
                voice: voice.clone(),
                phones: self.explore.phones,
                phone: self.explore.phone.clone(),
                sentence: self.explore.unit,
            },
            (Tab::Voices | Tab::Explore, _) => Route::Voices,
        }
    }

    /// A link's voice isn't installed: offer to download it.
    fn offer_window(&mut self, ctx: &egui::Context) {
        let Some(offer) = &self.links.offer else { return };
        let inst = offer.installer;
        let can_fetch = self.mirror().is_some();
        let (mut download, mut dismiss) = (false, false);
        egui::Window::new("Voice not installed")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(format!("This link uses {} ({}, {}), which isn't installed.", inst.label, inst.language, inst.gender.to_lowercase()));
                ui.label(format!("Download it from the voice mirror? {}", catalog::size_label(inst.size)));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    download = ui
                        .add_enabled(can_fetch, egui::Button::new(RichText::new("Download").color(GROUND).strong()).fill(AMBER))
                        .on_disabled_hover_text("No voice mirror is set: set one in Settings, or import the installer in Voices.")
                        .clicked();
                    dismiss = ui.button("Not now").clicked();
                });
            });
        if download {
            if let Some(o) = self.links.offer.take() {
                self.queue_download(o.installer);
                self.tab = Tab::Voices;
                self.links.awaiting = Some(o.route);
            }
        } else if dismiss {
            self.links.offer = None;
        }
    }

    /// Open the address bar's link when it changes from outside, and write
    /// the page shown to it: a new history entry for another page, the same
    /// entry (at most every 0.5 s) for an edited text. This runs after the
    /// page was drawn, so a route opened here draws on another frame.
    fn sync_url(&mut self, ctx: &egui::Context) {
        let Some(window) = web_sys::window() else { return };
        let hash = window.location().hash().unwrap_or_default();
        if self.links.written.as_deref() != Some(hash.as_str()) {
            // first load, back/forward, an edited or pasted address
            self.links.written = Some(hash.clone());
            self.links.replace_next = true;
            if let Some(r) = route::parse(&hash) {
                self.open_route(r);
                ctx.request_repaint();
            }
        }
        if self.listed {
            if let Some(r) = self.links.pending.take() {
                self.open_route(r);
                ctx.request_repaint();
            }
        }
        if self.links.awaiting.is_some() && self.tab != Tab::Voices {
            self.links.awaiting = None;
        }
        let landed = self.links.awaiting.as_ref().and_then(Route::voice).is_some_and(|v| self.installed_voice(v).is_some());
        if landed {
            if let Some(r) = self.links.awaiting.take() {
                self.open_route(r);
                ctx.request_repaint();
            }
        }
        // a link's address stays while its voice loads or its offer is open
        if !self.listed || self.loading.is_some() || self.links.pending.is_some() || self.links.offer.is_some() {
            return;
        }
        let current = self.current_route();
        let want = route::to_hash(&current);
        if self.links.written.as_ref() == Some(&want) {
            self.links.replace_next = false;
            return;
        }
        let text_only = self.links.written.as_deref().and_then(route::parse).is_some_and(|w| w.same_page(&current));
        let now = ctx.input(|i| i.time);
        let since = now - self.links.last_write;
        if text_only && since < 0.5 {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(0.5 - since));
            return;
        }
        let Ok(history) = window.history() else { return };
        let null = wasm_bindgen::JsValue::NULL;
        let written = if self.links.replace_next || text_only {
            history.replace_state_with_url(&null, "", Some(&want))
        } else {
            history.push_state_with_url(&null, "", Some(&want))
        };
        // on failure the next frame tries again
        if written.is_ok() {
            self.links.written = Some(want);
            self.links.replace_next = false;
            self.links.last_write = now;
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_no += 1;
        self.poll(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.poll_original(ctx);
        self.schedule(ctx);
        self.top_bar(ctx);
        self.status_bar(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        self.report_window(ctx);
        if self.advanced() && self.tab == Tab::Speak {
            self.stage_bar(ctx);
            self.side_panel(ctx);
        }
        egui::CentralPanel::default().frame(egui::Frame::central_panel(&ctx.style()).fill(GROUND)).show(ctx, |ui| match self.tab {
            Tab::Speak => self.speak_tab(ui),
            Tab::Voices => {
                egui::ScrollArea::vertical().id_salt("voices").show(ui, |ui| self.voices_tab(ui));
            }
            Tab::Explore => self.explore_tab(ui),
        });
        #[cfg(target_arch = "wasm32")]
        {
            self.offer_window(ctx);
            self.sync_url(ctx);
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.screenshot(ctx);
        if let Some(q) = self.quit_after {
            if self.frame_no >= q {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    /// Scroll positions and open menus start fresh each time.
    fn persist_egui_memory(&self) -> bool {
        false
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // a screenshot's launch options aren't the user's settings
        #[cfg(not(target_arch = "wasm32"))]
        if self.screenshot.is_some() {
            return;
        }
        storage.set_string("advanced", self.advanced.to_string());
        #[cfg(not(target_arch = "wasm32"))]
        storage.set_string("original_engine", self.dev.original.to_string());
        storage.set_string("save_format", self.format.ext().to_string());
        storage.set_string("voice", self.chosen.clone().unwrap_or_default());
        storage.set_string("text", self.text.clone());
        storage.set_string("mirror_url", self.mirror_url.clone());
    }
}
