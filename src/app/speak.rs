//! The Speak tab: the voice and the text, Speak/Stop/Save, and what the
//! engine makes of the text (its sounds, the recorded pieces, the audio).
//! The text is analysed and rendered again in the background a moment after
//! the text, the voice or a setting changes, so Speak plays what is shown;
//! while it plays, the word, the sound and the piece being said are lit.

use super::*;
use crate::dev::pipeline::{self, Reference, Stages};
use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{pos2, vec2, Align2, FontId, Painter, Rect, Stroke};
use quoquendo::export::Tagged;
use quoquendo::g2p::Frontend;
use quoquendo::loquendo::VoiceDatabase;
use quoquendo::synth::{self, JoinStyle, Phrase, Speech, TargetPhrase, TimedSound};
use std::ops::Range;
use std::time::Duration;

/// How long the text must rest before it is analysed and rendered again (s).
const SETTLE: f64 = 0.25;

const WAVE_H: f32 = 96.0;
const STRIP_H: f32 = 22.0;
const AXIS_H: f32 = 18.0;

/// What a preview is made from: when any of it changes, it is made again.
#[derive(Clone, PartialEq)]
pub(super) struct Want {
    pub(super) voice: String,
    pub(super) text: String,
    pub(super) stages: Stages,
    pub(super) language: Option<Language>,
    pub(super) settings: Settings,
}

/// The text analysed and rendered.
pub(super) struct Preview {
    pub(super) want: Want,
    pub(super) targets: Vec<TargetPhrase>,
    /// byte ranges in `want.text` of the words the targets point at
    pub(super) words: Vec<Range<usize>>,
    pub(super) plan: Vec<Phrase>,
    pub(super) speech: Speech,
}

/// What the player plays.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Track {
    /// our speech of the text
    Ours,
    /// the original engine's (Original engine mode)
    #[cfg(not(target_arch = "wasm32"))]
    Original,
    /// a stretch of the recordings
    Clip,
}

/// The moment of our speech being said.
pub(super) struct Now {
    pub(super) t: f64,
    pub(super) sound: Option<usize>,
    pub(super) piece: Option<usize>,
    pub(super) word: Option<usize>,
}

/// FON, CAT and SIG for one text, off the window's thread on the desktop.
fn build(voice: &quoquendo::Voice, want: Want, reference: Option<Reference>) -> Result<Preview, String> {
    let other;
    let fe = match want.language {
        Some(l) => {
            other = Frontend::rules(l);
            &other
        }
        None => &voice.frontend,
    };
    let run = pipeline::run(&want.text, fe, &voice.descriptor.name, &voice.db, &want.stages, reference)?;
    let speech = synth::render(&run.plan, &voice.db, want.settings)?;
    Ok(Preview {
        want,
        targets: run.targets,
        words: run.words,
        plan: run.plan,
        speech,
    })
}

impl App {
    /// What the preview should be made from now; None without a voice or text.
    pub(super) fn want(&self) -> Option<Want> {
        let v = self.voice.as_ref()?;
        if self.text.trim().is_empty() {
            return None;
        }
        Some(Want {
            voice: v.descriptor.name.clone(),
            text: self.text.clone(),
            stages: self.stages_in_use(),
            language: self.language.filter(|_| self.advanced()),
            settings: Settings {
                volume: self.settings.volume,
                join: if self.advanced() { self.settings.join } else { JoinStyle::Original },
            },
        })
    }

    /// The stages the modes allow of the ones picked.
    fn stages_in_use(&self) -> Stages {
        let mut s = Stages::default();
        if !self.advanced() {
            return s;
        }
        s.sig = self.settings.join;
        #[cfg(not(target_arch = "wasm32"))]
        {
            use pipeline::{Cat, Fon};
            let picked = &self.dev.stages;
            s.fon = match &picked.fon {
                Fon::Original if self.dev.original => Fon::Original,
                Fon::Command(c) if !c.trim().is_empty() => Fon::Command(c.clone()),
                _ => Fon::Rules,
            };
            if self.dev.original && picked.cat == Cat::Original {
                s.cat = Cat::Original;
            }
        }
        s
    }

    /// Whether the preview shows the text, voice and settings as they are.
    pub(super) fn fresh(&self) -> bool {
        self.want().is_some_and(|w| self.preview.as_ref().is_some_and(|p| p.want == w))
    }

    /// Take a finished preview, and make the next one once what it depends
    /// on has rested for `SETTLE` (at once when Speak waits for it).
    pub(super) fn schedule(&mut self, ctx: &egui::Context) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.job.as_ref().is_some_and(|(_, h)| h.is_finished()) {
                let (want, handle) = self.job.take().unwrap();
                let result = handle.join().unwrap_or_else(|_| Err("making the preview crashed".into()));
                self.accept(want, result);
            }
            if self.job.is_some() {
                ctx.request_repaint_after(Duration::from_millis(30));
            }
        }
        let Some(want) = self.want() else {
            self.pending = None;
            self.speak_queued = false;
            if self.text.trim().is_empty() {
                self.preview = None;
            }
            return;
        };
        let now = ctx.input(|i| i.time);
        let done = self.preview.as_ref().is_some_and(|p| p.want == want) || self.failed.as_ref().is_some_and(|(w, _)| *w == want);
        if done {
            self.pending = None;
        } else if self.pending.as_ref().is_none_or(|(w, _)| *w != want) {
            self.pending = Some((want.clone(), now));
        }
        if let Some((_, since)) = &self.pending {
            let left = SETTLE - (now - since);
            if left > 0.0 && !self.speak_queued {
                ctx.request_repaint_after(Duration::from_secs_f64(left));
                return;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.dev.original {
            self.ask_original(&want.voice, &want.text, false, now);
        }
        if done {
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.job.is_some() {
            return;
        }
        let Some(reference) = self.reference_for(&want, now) else { return };
        let Some(voice) = self.voice.clone() else { return };
        #[cfg(not(target_arch = "wasm32"))]
        {
            let w = want.clone();
            self.job = Some((want, std::thread::spawn(move || build(&voice, w, reference))));
            ctx.request_repaint_after(Duration::from_millis(30));
        }
        #[cfg(target_arch = "wasm32")]
        {
            let result = build(&voice, want.clone(), reference);
            self.accept(want, result);
            ctx.request_repaint();
        }
    }

    /// The original engine's result for `want` when one of its stages needs
    /// it: `Some(None)` when none does, None while the engine runs or when
    /// it failed (and then the preview fails with it).
    fn reference_for(&mut self, want: &Want, now: f64) -> Option<Option<Reference>> {
        #[cfg(not(target_arch = "wasm32"))]
        if want.stages.need_original() {
            let key = (want.voice.clone(), want.text.clone());
            if let Some(r) = self.dev.refs.get(&key) {
                return Some(Some(r.clone()));
            }
            if let Some(e) = self.dev.ref_errors.get(&key) {
                self.failed = Some((want.clone(), e.clone()));
            } else {
                self.ask_original(&want.voice, &want.text, false, now);
            }
            return None;
        }
        let _ = (want, now);
        Some(None)
    }

    fn accept(&mut self, want: Want, result: Result<Preview, String>) {
        // the audio playing or paused no longer matches: start over
        if self.playing.is_some_and(|t| t != Track::Clip) {
            self.player.stop();
            self.playing = None;
        }
        match result {
            Ok(p) => {
                self.preview = Some(p);
                self.failed = None;
            }
            Err(e) => {
                self.error = Some(e.clone());
                self.failed = Some((want, e));
            }
        }
        if self.speak_queued {
            if self.fresh() {
                self.speak_queued = false;
                self.play(Track::Ours, 0.0);
            } else if self.failed.as_ref().is_some_and(|(w, _)| self.want().as_ref() == Some(w)) {
                self.speak_queued = false;
            }
        }
    }

    fn speak(&mut self) {
        self.pinned = None;
        if self.fresh() {
            self.play(Track::Ours, 0.0);
        } else {
            self.speak_queued = true;
        }
    }

    /// Play a track from `from` seconds in.
    pub(super) fn play(&mut self, track: Track, from: f64) {
        self.pinned = None;
        let result = match track {
            Track::Ours => match &self.preview {
                Some(p) => self.player.play(&p.speech.samples, p.speech.rate, from),
                None => return,
            },
            #[cfg(not(target_arch = "wasm32"))]
            Track::Original => match self.original_samples() {
                Some(s) => self.player.play(&s, crate::dev::oracle::RATE, from),
                None => return,
            },
            Track::Clip => return,
        };
        match result {
            Ok(()) => self.playing = Some(track),
            Err(e) => self.error = Some(e),
        }
    }

    pub(super) fn stop(&mut self) {
        self.player.stop();
        self.playing = None;
        self.pinned = None;
        self.speak_queued = false;
    }

    fn save(&mut self) {
        let Some(p) = &self.preview else { return };
        let tags = Tagged {
            text: &p.want.text,
            words: &p.words,
            voice: &p.want.voice,
        };
        match crate::audio::save(&p.speech, self.format, &tags) {
            Ok(Some(saved)) => {
                let mut status = format!("Saved {} · {:.2} s · {}", saved.name, p.speech.duration_s(), catalog::size_label(saved.bytes as u64));
                match (self.format, p.words.len()) {
                    (_, 0) => {}
                    (Format::Wav, n) => status.push_str(&format!(" · {n} word markers embedded")),
                    _ => status.push_str(" · lyrics embedded"),
                }
                self.status = status.clone();
                self.status_good = Some(status);
            }
            Ok(None) => {}
            Err(e) => self.error = Some(e),
        }
    }

    /// The track playing and where (s).
    pub(super) fn position(&self) -> Option<(Track, f64)> {
        let track = self.playing?;
        Some((track, self.pinned.or_else(|| self.player.position())?))
    }

    /// The moment of our speech being said, while it plays.
    fn now(&self) -> Option<Now> {
        let (track, t) = self.position()?;
        if track != Track::Ours {
            return None;
        }
        let p = self.preview.as_ref()?;
        let sample = (t * p.speech.rate as f64) as usize;
        let tl = &p.speech.timeline;
        let sound = tl.sound_at(sample);
        Some(Now {
            t,
            sound,
            piece: tl.piece_at(sample),
            word: sound.and_then(|s| tl.sounds[s].word),
        })
    }

    /// While our speech plays over the text it was made from: where the
    /// words already said end, and the word being said.
    fn text_marks(&self, now: Option<&Now>) -> Option<(usize, Option<Range<usize>>)> {
        let now = now?;
        let p = self.preview.as_ref()?;
        if p.want.text != self.text || p.words.is_empty() {
            return None;
        }
        let text = &self.text;
        let ok = |r: &&Range<usize>| r.start <= r.end && r.end <= text.len() && text.is_char_boundary(r.start) && text.is_char_boundary(r.end);
        let current = now.word.and_then(|w| p.words.get(w)).filter(ok).cloned();
        let sounds = &p.speech.timeline.sounds;
        let upto = now.sound.unwrap_or(sounds.len()).min(sounds.len());
        let said = sounds[..upto]
            .iter()
            .filter_map(|s| s.word)
            .filter(|w| Some(*w) != now.word)
            .filter_map(|w| p.words.get(w))
            .filter(ok)
            .map(|r| r.end)
            .max()
            .unwrap_or(0);
        let said = current.as_ref().map_or(said, |c| said.min(c.start));
        Some((said, current))
    }

    pub(super) fn speak_tab(&mut self, ui: &mut egui::Ui) {
        self.voice_row(ui);
        ui.add_space(4.0);
        let now = self.now();
        let marks = self.text_marks(now.as_ref());
        let mut layouter = |ui: &egui::Ui, text: &str, wrap: f32| {
            let mut job = text_job(ui, text, marks.as_ref());
            job.wrap.max_width = wrap;
            ui.fonts(|f| f.layout_job(job))
        };
        let edit = ui.add(
            egui::TextEdit::multiline(&mut self.text)
                .desired_rows(4)
                .desired_width(f32::INFINITY)
                .hint_text("Type text in the voice's language…")
                .layouter(&mut layouter),
        );
        if edit.changed() {
            self.stop();
        }
        ui.add_space(4.0);
        self.controls(ui);
        ui.add_space(6.0);
        egui::ScrollArea::vertical().id_salt("analysis").auto_shrink([false, true]).show(ui, |ui| {
            self.sounds_section(ui, now.as_ref());
            self.pieces_section(ui, now.as_ref());
            self.audio_section(ui, now.as_ref());
        });
    }

    fn voice_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Voice");
            let current = self
                .chosen
                .as_ref()
                .and_then(|c| self.installed_voice(c))
                .map(voice_label)
                .unwrap_or_else(|| "pick a voice".into());
            let mut pick = None;
            let width = (if self.advanced() { 250.0f32 } else { 290.0 }).min(ui.available_size_before_wrap().x - 40.0).max(120.0);
            egui::ComboBox::from_id_salt("voice").selected_text(current).width(width).show_ui(ui, |ui| {
                for v in &self.installed {
                    let label = match &v.unsupported {
                        Some(u) => format!("{} — {u}", voice_label(v)),
                        None => voice_label(v),
                    };
                    if ui.selectable_label(self.chosen.as_deref() == Some(v.name.as_str()), label).clicked() {
                        pick = Some(v.name.clone());
                    }
                }
            });
            if let Some(p) = pick {
                self.choose(&p);
            }
            if self.advanced() {
                // a combo box doesn't move to the next line by itself
                if ui.available_size_before_wrap().x < 230.0 {
                    ui.end_row();
                } else {
                    ui.add_space(8.0);
                }
                ui.label("Language");
                let own = self.voice.as_ref().map(|v| v.descriptor.language()).unwrap_or(Language::Generic);
                let current = self.language.unwrap_or(own);
                let mut pick = None;
                egui::ComboBox::from_id_salt("lang").selected_text(current.label()).width(140.0).show_ui(ui, |ui| {
                    for l in Language::ALL {
                        if ui.selectable_label(current == l, l.label()).clicked() {
                            pick = Some(l);
                        }
                    }
                });
                if let Some(l) = pick {
                    self.language = (l != own).then_some(l);
                }
            }
            if ui.add_enabled(self.voice.is_some(), egui::Button::new("⟲ Sample text")).clicked() {
                if let Some(v) = &self.voice {
                    self.text = v.descriptor.sample_text();
                }
                self.stop();
            }
            if self.loading.is_some() {
                ui.spinner();
            }
            if let (Some(v), false) = (&self.voice, self.advanced()) {
                let codec = v.descriptor.bin_name.split('.').nth(2).unwrap_or("");
                ui.label(
                    RichText::new(format!("{} Hz · {codec} · {} recorded sentences", grouped(v.descriptor.audio_rate as u64), grouped(v.db.units.len() as u64)))
                        .small()
                        .color(MUTED),
                );
            }
        });
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        let ready = self.voice.is_some() && self.can_speak().is_none() && !self.text.trim().is_empty();
        let fresh = self.fresh();
        ui.horizontal_wrapped(|ui| {
            // Speak, Pause while the text (or the original) plays, Resume when paused
            let paused = self.player.is_paused();
            let speaking = self.playing.is_some_and(|t| t != Track::Clip) && !paused;
            let label = if paused {
                "▶ Resume"
            } else if speaking {
                "⏸ Pause"
            } else {
                "▶ Speak"
            };
            let speak = egui::Button::new(RichText::new(label).color(GROUND).strong()).fill(AMBER);
            if ui.add_enabled(ready || paused || speaking, speak).clicked() {
                if paused {
                    self.player.resume();
                } else if speaking {
                    self.player.pause();
                } else {
                    self.speak();
                }
            }
            if ui.add_enabled(self.playing.is_some() || self.speak_queued, egui::Button::new("■ Stop")).clicked() {
                self.stop();
            }
            ui.add_space(14.0);
            let mut format = self.format;
            if ui.available_size_before_wrap().x < 160.0 {
                ui.end_row();
            }
            egui::ComboBox::from_id_salt("format").selected_text(format.label()).width(64.0).show_ui(ui, |ui| {
                ui.set_min_width(230.0);
                let font = egui::TextStyle::Body.resolve(ui.style());
                for f in Format::ALL {
                    let mut job = LayoutJob::default();
                    job.append(f.label(), 0.0, TextFormat::simple(font.clone(), TEXT));
                    job.append(&format!("  · {}", f.hint()), 0.0, TextFormat::simple(FontId::proportional(font.size - 2.0), MUTED));
                    ui.selectable_value(&mut format, f, job);
                }
            });
            self.format = format;
            if ui.add_enabled(ready && fresh, egui::Button::new("Save")).clicked() {
                self.save();
            }
            if ui.available_size_before_wrap().x < 260.0 {
                ui.end_row();
            } else {
                ui.add_space(16.0);
            }
            ui.label("Volume");
            ui.add(egui::Slider::new(&mut self.settings.volume, 0.0..=1.0).fixed_decimals(2));
        });
    }

    fn sounds_section(&self, ui: &mut egui::Ui, now: Option<&Now>) {
        let Some(voice) = &self.voice else { return };
        let db = &voice.db;
        let (targets, map): (&[TargetPhrase], Vec<Option<usize>>) = match &self.preview {
            Some(p) => (&p.targets, sound_map(&p.targets, &p.speech.timeline.sounds)),
            None => (&[], Vec::new()),
        };
        let count: usize = targets.iter().map(|t| t.phones.len()).sum();
        let current_sound = now.and_then(|n| n.sound);
        let current_word = now.and_then(|n| n.word);
        section(
            ui,
            "sounds",
            |ui| {
                ui.label(RichText::new("Sounds").strong());
                ui.label(RichText::new(format!("· {count} sounds ·")).color(MUTED));
                ui.label(RichText::new("green").color(GREEN));
                ui.label(RichText::new("recorded,").color(MUTED));
                ui.label(RichText::new("red").color(RED));
                ui.label(RichText::new("never recorded").color(MUTED));
            },
            |ui| {
                egui::Frame::group(ui.style()).fill(GROUND).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(5.0, 4.0);
                        // the word being said gets a frame, painted under it
                        let under = ui.painter().add(egui::Shape::Noop);
                        let mut lit: Vec<Rect> = Vec::new();
                        let mut k = 0;
                        for phrase in targets {
                            for (i, t) in phrase.phones.iter().enumerate() {
                                if t.word_start && i > 0 {
                                    ui.label(RichText::new("|").monospace().color(STROKE.gamma_multiply(1.6)));
                                }
                                let color = if db.knows_symbol(&t.sym) { GREEN } else { RED };
                                let text = RichText::new(&t.sym).monospace();
                                let text = match (current_sound, map.get(k).copied().flatten()) {
                                    (Some(c), Some(a)) if a == c => text.color(GROUND).background_color(AMBER),
                                    (Some(c), Some(a)) if a < c => text.color(color.gamma_multiply(0.45)),
                                    _ => text.color(color),
                                };
                                let r = ui.label(text);
                                if current_word.is_some() && t.word == current_word {
                                    lit.push(r.rect);
                                }
                                k += 1;
                            }
                            let tune = match phrase.mood {
                                Some('I') => " ?",
                                Some('E') => " !",
                                _ => "",
                            };
                            let chip = RichText::new(format!(" pause {} ms{tune} ", phrase.pause_ms)).monospace().small().color(MUTED).background_color(WIDGET);
                            ui.add(egui::Label::new(chip).extend());
                        }
                        let shapes: Vec<egui::Shape> = rows(&lit)
                            .into_iter()
                            .map(|r| egui::Shape::rect_filled(r.expand2(vec2(3.0, 1.0)), 3.0, AMBER.gamma_multiply(0.16)))
                            .chain(rows(&lit).into_iter().map(|r| egui::Shape::rect_stroke(r.expand2(vec2(3.0, 1.0)), 3.0, Stroke::new(1.0f32, AMBER.gamma_multiply(0.7)))))
                            .collect();
                        ui.painter().set(under, egui::Shape::Vec(shapes));
                    });
                });
            },
        );
    }

    fn pieces_section(&mut self, ui: &mut egui::Ui, now: Option<&Now>) {
        let Some(voice) = self.voice.clone() else { return };
        let db = &voice.db;
        let plan: &[Phrase] = self.preview.as_ref().map_or(&[], |p| &p.plan);
        let count: usize = plan.iter().map(|p| p.pieces.len()).sum();
        let from: std::collections::BTreeSet<u32> = plan.iter().flat_map(|p| p.pieces.iter().map(|x| x.unit)).collect();
        let rate = db.audio_rate() as f32;
        let current = now.and_then(|n| n.piece);
        let follow = current.filter(|c| self.followed_piece != Some(*c));
        let mut play = None;
        let mut open = None;
        section(
            ui,
            "pieces",
            |ui| {
                ui.label(RichText::new("Pieces").strong());
                ui.label(RichText::new(format!("· {count} pieces from {} recorded sentences", from.len())).color(MUTED));
            },
            |ui| {
                egui::Frame::group(ui.style()).fill(GROUND).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let cols = piece_columns(ui.available_width());
                    table_row(ui, &cols, None, |i, ui| {
                        ui.label(RichText::new(["", "sounds", "sentence", "", "at", "length", "pitch"][i]).color(MUTED));
                    });
                    let stripe = ui.visuals().faint_bg_color;
                    egui::ScrollArea::vertical().id_salt("pieces").max_height(300.0).auto_shrink([false, true]).show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        let mut k = 0;
                        for phrase in plan {
                            for p in &phrase.pieces {
                                let fill = if current == Some(k) {
                                    Some(AMBER.gamma_multiply(0.16))
                                } else {
                                    (k % 2 == 0).then_some(stripe)
                                };
                                let rect = table_row(ui, &cols, fill, |i, ui| match i {
                                    0 => {
                                        if ui.small_button("▶").on_hover_text("Play this piece as recorded").clicked() {
                                            play = Some((p.unit, p.from, p.to));
                                        }
                                    }
                                    1 => {
                                        ui.label(RichText::new(p.covers.join(" ")).monospace().color(if p.hard { LILAC } else { GREEN }));
                                        if p.hard {
                                            badge(ui, "hard cut", LILAC);
                                        }
                                    }
                                    2 => {
                                        if ui.link(RichText::new(format!("#{}", p.unit)).monospace().color(ui.visuals().hyperlink_color)).clicked() {
                                            open = Some(p.unit as usize);
                                        }
                                    }
                                    3 => {
                                        let u = &db.units[p.unit as usize];
                                        let said = u.symbols[1..u.symbols.len().saturating_sub(1)].join(" ");
                                        ui.label(RichText::new(said).monospace().small().color(MUTED));
                                    }
                                    4 => {
                                        ui.label(RichText::new(format!("{:.2} s", p.from as f32 / rate)).monospace());
                                    }
                                    5 => {
                                        ui.label(RichText::new(format!("{:.0} ms", (p.to - p.from) as f32 * 1000.0 / rate)).monospace());
                                    }
                                    _ => {
                                        ui.label(RichText::new(format!("{} Hz", p.pitch)).monospace());
                                    }
                                });
                                if follow == Some(k) {
                                    ui.scroll_to_rect(rect, Some(egui::Align::Center));
                                }
                                k += 1;
                            }
                            if !phrase.missing.is_empty() {
                                table_row(ui, &cols, None, |i, ui| match i {
                                    1 => {
                                        ui.label(RichText::new(phrase.missing.join(" ")).monospace().color(RED));
                                    }
                                    3 => {
                                        ui.label(RichText::new("never recorded").small().color(RED));
                                    }
                                    _ => {}
                                });
                            }
                            if phrase.pause_ms > 0 {
                                let tune = match phrase.mood {
                                    Some('I') => "question ",
                                    Some('E') => "exclamation ",
                                    _ => "",
                                };
                                table_row(ui, &cols, None, |i, ui| {
                                    if i == 1 {
                                        ui.label(RichText::new(format!("— {tune}pause {} ms", phrase.pause_ms)).small().color(MUTED));
                                    }
                                });
                            }
                        }
                    });
                });
            },
        );
        if follow.is_some() {
            self.followed_piece = follow;
        }
        if let Some((unit, a, b)) = play {
            self.play_span(unit, a, b);
        }
        if let Some(unit) = open {
            self.open_sentence(unit);
        }
    }

    fn audio_section(&mut self, ui: &mut egui::Ui, now: Option<&Now>) {
        let position = self.position().filter(|(t, _)| *t != Track::Clip);
        let updating = self.want().is_some() && !self.fresh() && self.failed.as_ref().is_none_or(|(w, _)| self.want().as_ref() != Some(w));
        let has_text = !self.text.trim().is_empty();
        let duration = |track: Track| -> f64 {
            match track {
                #[cfg(not(target_arch = "wasm32"))]
                Track::Original => self.original_duration().unwrap_or(0.0),
                _ => self.preview.as_ref().map_or(0.0, |p| p.speech.duration_s() as f64),
            }
        };
        let readout = position.map(|(track, t)| (t, duration(track)));
        let summary = self.preview.as_ref().filter(|_| has_text && !updating).map(|p| format!("· {:.2} s · {} Hz", p.speech.duration_s(), grouped(p.speech.rate as u64)));
        section(
            ui,
            "audio",
            |ui| {
                ui.label(RichText::new("Audio").strong());
                if updating {
                    ui.label(RichText::new("· updating…").color(MUTED));
                    ui.spinner();
                } else if let Some(s) = &summary {
                    ui.label(RichText::new(s).color(MUTED));
                }
                if let Some((t, total)) = readout {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("/ {total:.2} s")).monospace().color(MUTED));
                        ui.label(RichText::new(format!("{t:.2} s")).monospace().color(AMBER));
                    });
                }
            },
            |ui| {
                #[cfg(not(target_arch = "wasm32"))]
                if self.dev.original {
                    self.original_lanes(ui, position);
                    return;
                }
                self.single_lane(ui, now);
            },
        );
    }

    /// Our waveform, its sounds and the time axis; under the pointer, a line
    /// and the sound at that moment.
    fn single_lane(&self, ui: &mut egui::Ui, now: Option<&Now>) {
        egui::Frame::group(ui.style()).fill(GROUND).show(ui, |ui| {
            ui.set_width(ui.available_width());
            let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), WAVE_H + STRIP_H + AXIS_H), egui::Sense::hover());
            let (Some(p), Some(v)) = (&self.preview, &self.voice) else { return };
            let painter = ui.painter_at(rect.expand(2.0));
            let duration = p.speech.duration_s() as f64;
            let axis = Axis::new(rect, duration);
            let wave = Rect::from_min_size(rect.min, vec2(rect.width(), WAVE_H));
            let strip = Rect::from_min_size(wave.left_bottom(), vec2(rect.width(), STRIP_H));
            let ticks = Rect::from_min_size(strip.left_bottom(), vec2(rect.width(), AXIS_H));
            let played = now.map(|n| n.t);
            let marks = our_marks(&p.speech, &v.db);
            let hover = self.hover_time(&response, wave.union(strip), &axis, duration);
            paint_wave(&painter, wave, &axis, &p.speech.samples, p.speech.rate, played, AMBER);
            paint_strip(&painter, strip, &axis, &marks, now.and_then(|n| n.sound), true);
            paint_axis(&painter, ticks, &axis);
            if let Some(t) = played {
                paint_playhead(&painter, wave.top(), strip.bottom(), axis.x(t), Head::Playing);
            }
            if let Some(t) = hover {
                paint_playhead(&painter, wave.top(), strip.bottom(), axis.x(t), Head::Hover);
                paint_hovered(&painter, strip, &axis, &marks, t, true);
            }
        });
    }

    /// The moment under the pointer when it is over `area` (or the one
    /// `--hover-at` pins for a screenshot), within `0..=duration`.
    pub(super) fn hover_time(&self, response: &egui::Response, area: Rect, axis: &Axis, duration: f64) -> Option<f64> {
        let t = match response.hover_pos() {
            Some(p) if area.contains(p) => axis.t(p.x),
            _ => self.hover_at?,
        };
        (0.0..=duration).contains(&t).then_some(t)
    }
}

/// A collapsible section: its heading row, open at first.
fn section(ui: &mut egui::Ui, id: &str, header: impl FnOnce(&mut egui::Ui), body: impl FnOnce(&mut egui::Ui)) {
    let id = ui.make_persistent_id(id);
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
        .show_header(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            header(ui);
        })
        .body(body);
    ui.add_space(4.0);
}

/// The Pieces table's column widths for `width`: play, sounds, sentence,
/// what it says (the slack, dropped when there is none), at, length, pitch.
fn piece_columns(width: f32) -> [f32; 7] {
    let fixed = 34.0 + 70.0 + 100.0 + 80.0 + 70.0;
    let (sounds, said) = match width - fixed - 200.0 {
        slack if slack >= 80.0 => (200.0, slack),
        _ => ((width - fixed).max(60.0), 0.0),
    };
    [34.0, sounds, 70.0, said, 100.0, 80.0, 70.0]
}

/// One table row of `widths`, filled with `fill`: `cell` draws each
/// column's contents (the last three right-aligned, all cut at their width).
fn table_row(ui: &mut egui::Ui, widths: &[f32], fill: Option<Color32>, mut cell: impl FnMut(usize, &mut egui::Ui)) -> Rect {
    let (rect, _) = ui.allocate_exact_size(vec2(widths.iter().sum(), 24.0), egui::Sense::hover());
    if let Some(c) = fill {
        ui.painter().rect_filled(rect, 2.0, c);
    }
    let mut x = rect.left();
    for (i, w) in widths.iter().enumerate() {
        let r = Rect::from_min_size(pos2(x, rect.top()), vec2(*w, rect.height())).shrink2(vec2(4.0, 0.0));
        x += w;
        if r.width() <= 0.0 {
            continue;
        }
        let layout = if i + 3 >= widths.len() { egui::Layout::right_to_left(egui::Align::Center) } else { egui::Layout::left_to_right(egui::Align::Center) };
        // a child that takes no room: the row has it already
        let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(r).layout(layout));
        ui.set_clip_rect(r.intersect(ui.clip_rect()));
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
        cell(i, &mut ui);
    }
    rect
}

/// A small framed tag.
fn badge(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::none()
        .stroke(Stroke::new(1.0f32, color))
        .rounding(3.0)
        .inner_margin(egui::Margin::symmetric(4.0, 0.0))
        .show(ui, |ui| ui.label(RichText::new(text).small().color(color)));
}

/// The text, with the words already said dimmed and the word being said lit.
fn text_job(ui: &egui::Ui, text: &str, marks: Option<&(usize, Option<Range<usize>>)>) -> LayoutJob {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let plain = TextFormat::simple(font.clone(), TEXT);
    let mut job = LayoutJob::default();
    let cut = |at: usize| at <= text.len() && text.is_char_boundary(at);
    let marks = marks.map(|(said, current)| {
        let (a, b) = current.clone().map_or((*said, *said), |r| (r.start, r.end));
        (*said, a, b)
    });
    match marks {
        Some((said, a, b)) if said <= a && a <= b && cut(said) && cut(a) && cut(b) => {
            let lit = TextFormat {
                background: AMBER,
                ..TextFormat::simple(font.clone(), GROUND)
            };
            job.append(&text[..said], 0.0, TextFormat::simple(font, MUTED));
            job.append(&text[said..a], 0.0, plain.clone());
            job.append(&text[a..b], 0.0, lit);
            job.append(&text[b..], 0.0, plain);
        }
        _ => job.append(text, 0.0, plain),
    }
    job
}

/// `rects` merged into one per line of the wrapped layout.
fn rows(rects: &[Rect]) -> Vec<Rect> {
    let mut out: Vec<Rect> = Vec::new();
    for r in rects {
        match out.iter_mut().find(|o| (o.center().y - r.center().y).abs() < 2.0) {
            Some(o) => *o = o.union(*r),
            None => out.push(*r),
        }
    }
    out
}

/// Each target sound's place among the timeline's sounds, matched in order by
/// symbol: a sound that was never recorded may have none.
fn sound_map(targets: &[TargetPhrase], sounds: &[TimedSound]) -> Vec<Option<usize>> {
    let mut j = 0;
    targets
        .iter()
        .flat_map(|p| &p.phones)
        .map(|t| {
            let found = (j..sounds.len().min(j + 6)).find(|&k| !sounds[k].pause && sounds[k].sym == t.sym);
            if let Some(k) = found {
                j = k + 1;
            }
            found
        })
        .collect()
}

/// `n` with its thousands apart: 32 000.
pub(super) fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// One sound or pause on a strip, in seconds.
pub(super) struct Mark {
    pub(super) sym: String,
    pub(super) start: f64,
    pub(super) end: f64,
    pub(super) pause: bool,
    pub(super) word_start: bool,
    pub(super) known: bool,
}

/// Our speech's sounds.
pub(super) fn our_marks(speech: &Speech, db: &VoiceDatabase) -> Vec<Mark> {
    let rate = speech.rate as f64;
    speech
        .timeline
        .sounds
        .iter()
        .map(|s| Mark {
            sym: s.sym.clone(),
            start: s.span.start as f64 / rate,
            end: s.span.end as f64 / rate,
            pause: s.pause,
            word_start: s.word_start,
            known: s.pause || db.knows_symbol(&s.sym),
        })
        .collect()
}

/// The shared time axis: seconds to x.
pub(super) struct Axis {
    left: f32,
    width: f32,
    total: f64,
}

impl Axis {
    pub(super) fn new(rect: Rect, total: f64) -> Axis {
        Axis {
            left: rect.left() + 8.0,
            width: rect.width() - 16.0,
            total: total.max(0.01),
        }
    }

    pub(super) fn x(&self, t: f64) -> f32 {
        self.left + (t / self.total) as f32 * self.width
    }

    /// The moment at `x`.
    pub(super) fn t(&self, x: f32) -> f64 {
        ((x - self.left) / self.width) as f64 * self.total
    }
}

/// A waveform as its peaks, one per pixel: before `played` in amber, after
/// it dimmed, all in `idle` when it isn't playing.
pub(super) fn paint_wave(painter: &Painter, rect: Rect, axis: &Axis, samples: &[f32], rate: u32, played: Option<f64>, idle: Color32) {
    let mid = rect.center().y;
    painter.line_segment([pos2(axis.x(0.0), mid), pos2(axis.left + axis.width, mid)], Stroke::new(1.0f32, idle.gamma_multiply(0.5)));
    if samples.is_empty() {
        return;
    }
    let duration = samples.len() as f64 / rate as f64;
    let px = (axis.x(duration) - axis.x(0.0)).max(1.0) as usize;
    // each pixel's peak over at least 25 ms around it: the outline of the
    // syllables rather than every pitch period
    let reach = (samples.len() / px / 2).max(rate as usize / 80);
    let peaks: Vec<f32> = (0..px)
        .map(|i| {
            let mid = (2 * i + 1) * samples.len() / (2 * px);
            let a = mid.saturating_sub(reach);
            let b = (mid + reach).clamp(a + 1, samples.len());
            samples[a.min(b - 1)..b].iter().fold(0.0f32, |m, s| m.max(s.abs()))
        })
        .collect();
    let top = peaks.iter().copied().fold(0.05f32, f32::max);
    let half = rect.height() * 0.46;
    for (i, v) in peaks.iter().enumerate() {
        let x = axis.x(0.0) + i as f32 + 0.5;
        let t = (i as f64 + 0.5) / px as f64 * duration;
        let color = match played {
            Some(p) if t <= p => AMBER,
            Some(_) => WAVE_DIM,
            None => idle,
        };
        let h = (v / top * half).max(0.5);
        painter.line_segment([pos2(x, mid - h), pos2(x, mid + h)], Stroke::new(1.0f32, color));
    }
}

/// The sounds under (or, `ticks_up` false, over) a waveform: a tick at each
/// boundary, longer where a word starts, each sound's symbol in the middle
/// of its span (green recorded, red never recorded, left out where it would
/// collide), pauses as blocks; `current` lit.
pub(super) fn paint_strip(painter: &Painter, rect: Rect, axis: &Axis, marks: &[Mark], current: Option<usize>, ticks_up: bool) {
    let font = FontId::monospace(11.0);
    let tick = |x: f32, long: bool| {
        let len = if long { 7.0 } else { 4.0 };
        let (a, b) = if ticks_up { (rect.top(), rect.top() + len) } else { (rect.bottom() - len, rect.bottom()) };
        painter.line_segment([pos2(x, a), pos2(x, b)], Stroke::new(1.0f32, MUTED));
    };
    let label_y = if ticks_up { rect.top() + 8.0 } else { rect.top() + 2.0 };
    let mut last_right = f32::NEG_INFINITY;
    for (i, m) in marks.iter().enumerate() {
        let (a, b) = (axis.x(m.start), axis.x(m.end));
        tick(a, m.word_start || m.pause);
        if m.pause {
            let block = Rect::from_min_max(pos2(a + 1.0, label_y + 1.0), pos2((b - 1.0).max(a + 2.0), label_y + 12.0));
            painter.rect_filled(block, 2.0, WIDGET);
            if block.width() > 40.0 {
                painter.text(block.center(), Align2::CENTER_CENTER, "pause", FontId::proportional(9.0), MUTED);
            }
            continue;
        }
        let lit = current == Some(i);
        let color = if lit {
            GROUND
        } else if m.known {
            GREEN
        } else {
            RED
        };
        let galley = painter.layout_no_wrap(m.sym.clone(), font.clone(), color);
        let w = galley.size().x;
        let left = (a + b) / 2.0 - w / 2.0;
        if left < last_right + 2.0 && !lit {
            continue;
        }
        let at = pos2(left, label_y);
        if lit {
            painter.rect_filled(Rect::from_min_size(at, galley.size()).expand2(vec2(2.0, 0.5)), 2.0, AMBER);
        }
        painter.galley(at, galley, color);
        last_right = left + w;
    }
    if let Some(m) = marks.last() {
        tick(axis.x(m.end), true);
    }
}

/// The time axis: a tick and a label every round step.
pub(super) fn paint_axis(painter: &Painter, rect: Rect, axis: &Axis) {
    let y = rect.top() + 2.0;
    painter.line_segment([pos2(axis.x(0.0), y), pos2(axis.left + axis.width, y)], Stroke::new(1.0f32, STROKE));
    let step = [0.1, 0.2, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 30.0, 60.0]
        .into_iter()
        .find(|s| s / axis.total * axis.width as f64 >= 70.0)
        .unwrap_or(120.0);
    let mut t = 0.0;
    while t <= axis.total + 1e-9 {
        let x = axis.x(t);
        painter.line_segment([pos2(x, y), pos2(x, y + 3.0)], Stroke::new(1.0f32, MUTED));
        let label = if t == 0.0 { "0".to_string() } else { format!("{} s", trim_float(t)) };
        painter.text(pos2(x, y + 4.0), Align2::CENTER_TOP, label, FontId::monospace(10.0), MUTED);
        t += step;
    }
}

fn trim_float(t: f64) -> String {
    let s = format!("{t:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// What a line across the lanes marks.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Head {
    /// the playhead on the lane playing: solid, with a marker
    Playing,
    /// the playhead across another lane: dashed (Original engine mode)
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    Other,
    /// the pointer: like the playhead, muted
    Hover,
}

/// A line from `top` to `bottom` at `x`: the playhead or the pointer.
pub(super) fn paint_playhead(painter: &Painter, top: f32, bottom: f32, x: f32, head: Head) {
    let marker = |color: Color32| egui::Shape::convex_polygon(vec![pos2(x - 4.0, top), pos2(x + 4.0, top), pos2(x, top + 6.0)], color, Stroke::NONE);
    match head {
        Head::Playing => {
            painter.line_segment([pos2(x, top), pos2(x, bottom)], Stroke::new(1.5f32, TEXT));
            painter.add(marker(TEXT));
        }
        Head::Hover => {
            painter.line_segment([pos2(x, top), pos2(x, bottom)], Stroke::new(1.0f32, MUTED.gamma_multiply(0.8)));
            painter.add(marker(MUTED.gamma_multiply(0.8)));
        }
        Head::Other => {
            painter.extend(egui::Shape::dashed_line(&[pos2(x, top), pos2(x, bottom)], Stroke::new(1.0f32, TEXT.gamma_multiply(0.6)), 3.0, 3.0));
        }
    }
}

/// The sound at `t` on a strip painted by `paint_strip`: its span tinted
/// and a chip on the label row at `t` with its symbol (green recorded, red
/// never recorded) and the time, shown even where its label was left out.
pub(super) fn paint_hovered(painter: &Painter, rect: Rect, axis: &Axis, marks: &[Mark], t: f64, ticks_up: bool) {
    let Some(m) = marks.iter().find(|m| m.start <= t && t < m.end).or_else(|| marks.last().filter(|m| t == m.end)) else { return };
    let span = Rect::from_x_y_ranges(axis.x(m.start)..=axis.x(m.end).max(axis.x(m.start) + 1.0), rect.y_range());
    painter.rect_filled(span, 2.0, AMBER.gamma_multiply(0.18));
    let font = FontId::monospace(11.0);
    let mut job = LayoutJob::default();
    let (sym, color) = match (m.pause, m.known) {
        (true, _) => ("pause", MUTED),
        (false, true) => (m.sym.as_str(), GREEN),
        (false, false) => (m.sym.as_str(), RED),
    };
    job.append(sym, 0.0, TextFormat::simple(font.clone(), color));
    job.append(&format!("  {t:.2} s"), 0.0, TextFormat::simple(font, MUTED));
    let galley = painter.layout_job(job);
    let size = galley.size();
    let label_y = if ticks_up { rect.top() + 8.0 } else { rect.top() + 2.0 };
    let left = (axis.x(t) - size.x / 2.0).clamp(rect.left() + 3.0, (rect.right() - size.x - 3.0).max(rect.left() + 3.0));
    let chip = Rect::from_min_size(pos2(left, label_y), size).expand2(vec2(3.0, 1.0));
    painter.rect(chip, 3.0, PANEL, Stroke::new(1.0f32, AMBER.gamma_multiply(0.7)));
    painter.galley(pos2(left, label_y), galley, TEXT);
}
