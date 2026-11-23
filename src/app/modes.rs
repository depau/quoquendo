//! Advanced mode: the voice's details beside the Speak tab and the stage
//! bar under it. Original engine (Wine) mode (desktop only) adds the
//! original engine: as stages, beside our audio, and in a comparison.

use super::*;
#[cfg(not(target_arch = "wasm32"))]
use super::speak::Track;
#[cfg(not(target_arch = "wasm32"))]
use crate::dev::pipeline;
#[cfg(not(target_arch = "wasm32"))]
use eframe::egui::vec2;

impl App {
    /// The voice's details, its inventory and its language data.
    pub(super) fn side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("details").resizable(false).exact_width(300.0).show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                let Some(voice) = &self.voice else { return };
                let v = &voice.descriptor;
                let grid = |ui: &mut egui::Ui, id: &str, rows: Vec<(&str, String)>| {
                    egui::Grid::new(id).num_columns(2).spacing([12.0, 3.0]).max_col_width(150.0).show(ui, |ui| {
                        for (k, val) in rows {
                            ui.label(RichText::new(k).monospace().small().color(MUTED));
                            ui.add(egui::Label::new(RichText::new(val).monospace().small()).wrap());
                            ui.end_row();
                        }
                    });
                };
                ui.add_space(4.0);
                egui::CollapsingHeader::new(RichText::new("Voice details").strong()).default_open(true).show(ui, |ui| {
                    grid(
                        ui,
                        "vd",
                        vec![
                            ("Language", v.language().label().to_string()),
                            ("MotherTongue", loquendo::from_latin1(&v.mother_tongue)),
                            ("Description", loquendo::from_latin1(&v.description)),
                            ("Gender / Age", format!("{} / {}", v.gender, v.age)),
                            ("Library", v.library.clone()),
                            ("BaseSpeed", v.base_speed.to_string()),
                            ("BasePitch", v.base_pitch.to_string()),
                            ("Sample rate", format!("{} Hz", speak::grouped(v.audio_rate as u64))),
                            ("Gilded", if v.gilded { "yes" } else { "no" }.into()),
                            ("Recordings file", v.bin_name.clone()),
                        ],
                    );
                });
                let db = &voice.db;
                egui::CollapsingHeader::new(RichText::new("Inventory").strong()).default_open(true).show(ui, |ui| {
                    let recordings = match &db.audio {
                        Some(a) => {
                            let secs = a.total_samples() / a.rate as usize;
                            format!("{}:{:02}:{:02} of Speex, {} kbit/s, {} Hz", secs / 3600, secs / 60 % 60, secs % 60, a.bitrate / 1000, speak::grouped(a.rate as u64))
                        }
                        None => db.audio_error.clone().unwrap_or_else(|| "not found".into()),
                    };
                    grid(
                        ui,
                        "inv",
                        vec![
                            ("File", db.path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default()),
                            ("Format", db.format.clone()),
                            ("Recorded sentences", speak::grouped(db.units.len() as u64)),
                            ("Timed sounds", format!("{} (+2 edge silences each = {})", db.instance_count(), db.declared_instances)),
                            (
                                "Questions / excl.",
                                format!(
                                    "{} / {}",
                                    db.units.iter().filter(|u| u.mood == Some('I')).count(),
                                    db.units.iter().filter(|u| u.mood == Some('E')).count()
                                ),
                            ),
                            ("Alphabet", format!("{} symbols", db.alphabet().len())),
                            ("Longest sentence", format!("{} sounds", db.longest_unit())),
                            ("Recordings", recordings),
                        ],
                    );
                    #[cfg(not(target_arch = "wasm32"))]
                    for (name, u, t) in &self.dev.extras {
                        ui.label(RichText::new(format!("{name}: {u} units, {t} takes")).small().color(MUTED));
                    }
                });
                egui::CollapsingHeader::new(RichText::new("Language data").strong()).default_open(true).show(ui, |ui| {
                    let fe = &voice.frontend;
                    match &voice.language_missing {
                        Some(t) => ui.label(RichText::new(format!("none: no {t} language folder")).small().color(AMBER)),
                        None if fe.has_module() => ui.label(RichText::new("the original's language module, ported").small()),
                        None => ui.label(RichText::new(format!("built-in rules, {} dictionary words", fe.dictionary_len())).small()),
                    };
                });
            });
        });
    }

    /// The stages: pronunciation (desktop), piece picking (Original engine
    /// mode) and joining; and the comparison with the original.
    pub(super) fn stage_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("stages").show(ctx, |ui| {
            ui.add_space(3.0);
            ui.horizontal(|ui| {
                #[cfg(not(target_arch = "wasm32"))]
                self.wine_stages(ui);
                ui.label("Joining");
                ui.label(match self.profile_join() {
                    quoquendo::synth::JoinStyle::Smooth => "Smoothed",
                    quoquendo::synth::JoinStyle::Original => "Like the original",
                });
            });
            #[cfg(not(target_arch = "wasm32"))]
            if self.dev.original {
                ui.horizontal(|ui| {
                    let busy = self.dev.job.as_ref().is_some_and(|j| j.compare);
                    if ui.add_enabled(self.dev.job.is_none() && self.voice.is_some(), egui::Button::new("Compare with the original")).clicked() {
                        if let Some(v) = &self.voice {
                            let (voice, text) = (v.descriptor.name.clone(), self.text.clone());
                            self.ask_original(&voice, &text, true, ui.input(|i| i.time));
                        }
                    }
                    if let Some(job) = self.dev.job.as_ref().filter(|_| busy) {
                        ui.spinner();
                        let secs = ui.input(|i| i.time) - job.started;
                        ui.label(RichText::new(format!("Comparing… {secs:.0} s")).color(MUTED));
                    }
                });
            }
            ui.add_space(3.0);
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl App {
    /// Pronunciation and, in Original engine mode, piece picking.
    fn wine_stages(&mut self, ui: &mut egui::Ui) {
        use pipeline::{Cat, Fon};
        let original = self.dev.original;
        ui.label("Pronunciation");
        let mut fon = match self.dev.stages.fon {
            Fon::Original if original => 2,
            Fon::Command(_) => 1,
            _ => 0,
        };
        let names = ["Voice's own", "Command", "Original engine (Wine)"];
        egui::ComboBox::from_id_salt("fon").selected_text(names[fon]).width(150.0).show_ui(ui, |ui| {
            for (i, name) in names.iter().enumerate().take(if original { 3 } else { 2 }) {
                ui.selectable_value(&mut fon, i, *name);
            }
        });
        if fon == 1 {
            ui.add(egui::TextEdit::singleline(&mut self.dev.fon_command).desired_width(220.0).font(egui::TextStyle::Monospace));
        }
        self.dev.stages.fon = match fon {
            1 => Fon::Command(self.dev.fon_command.clone()),
            2 => Fon::Original,
            _ => Fon::Rules,
        };
        ui.separator();
        if original {
            ui.label("Piece picking");
            let mut cat = self.dev.stages.cat;
            egui::ComboBox::from_id_salt("cat")
                .selected_text(if cat == Cat::Builtin { "Built-in" } else { "Original engine (Wine)" })
                .width(150.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut cat, Cat::Builtin, "Built-in");
                    ui.selectable_value(&mut cat, Cat::Original, "Original engine (Wine)");
                });
            self.dev.stages.cat = cat;
            ui.separator();
        }
    }

    /// Run the original engine on `text` in the background, unless its
    /// result (or failure) is known or a run is on. `compare` then compares
    /// it with ours in a report (at once when the result is known).
    pub(super) fn ask_original(&mut self, voice: &str, text: &str, compare: bool, now: f64) {
        if self.dev.job.is_some() || text.trim().is_empty() {
            return;
        }
        let key = (voice.to_string(), text.to_string());
        if self.dev.refs.contains_key(&key) {
            if compare {
                let r = self.dev.refs[&key].clone();
                self.report(std::slice::from_ref(&r));
            }
            return;
        }
        if !compare && self.dev.ref_errors.contains_key(&key) {
            return;
        }
        let (name, texts) = (key.0.clone(), vec![key.1.clone()]);
        self.dev.ref_errors.remove(&key);
        self.dev.job = Some(OracleJob {
            voice: key.0,
            texts: vec![key.1],
            handle: std::thread::spawn(move || crate::dev::oracle::run_many(&name, &texts)),
            compare,
            started: now,
        });
    }

    fn report(&mut self, refs: &[crate::dev::oracle::Reference]) {
        let Some(v) = &self.voice else { return };
        let out = crate::dev::oracle::oracle_dir().map(|d| d.join("out/compare"));
        self.dev.report = Some(match crate::dev::compare::compare(refs, v, out.as_deref()) {
            Ok((r, _)) => r,
            Err(e) => e,
        });
    }

    /// Take the original engine's finished run.
    pub(super) fn poll_original(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.dev.job else { return };
        if !job.handle.is_finished() {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
            return;
        }
        let job = self.dev.job.take().unwrap();
        match job.handle.join().unwrap_or_else(|_| Err("the original engine's run crashed".into())) {
            Ok(refs) => {
                if job.compare {
                    self.report(&refs);
                }
                for r in refs {
                    self.dev.refs.insert((job.voice.clone(), r.text.clone()), r);
                }
            }
            Err(e) => {
                if job.compare {
                    self.error = Some(e.clone());
                }
                for t in job.texts {
                    self.dev.ref_errors.insert((job.voice.clone(), t), e.clone());
                }
            }
        }
        ctx.request_repaint();
    }

    /// The original engine's result for the text shown.
    fn shown_reference(&self) -> Option<&crate::dev::oracle::Reference> {
        let p = self.preview.as_ref()?;
        self.dev.refs.get(&(p.want.voice.clone(), p.want.text.clone()))
    }

    /// The original engine's audio for the text shown, ready to play.
    pub(super) fn original_samples(&mut self) -> Option<Arc<Vec<f32>>> {
        let p = self.preview.as_ref()?;
        let key = (p.want.voice.clone(), p.want.text.clone());
        if let Some((k, s)) = &self.dev.original_samples {
            if *k == key {
                return Some(s.clone());
            }
        }
        let r = self.dev.refs.get(&key)?;
        let samples = Arc::new(r.audio.iter().map(|s| *s as f32 / 32768.0).collect::<Vec<f32>>());
        self.dev.original_samples = Some((key, samples.clone()));
        Some(samples)
    }

    pub(super) fn original_duration(&self) -> Option<f64> {
        self.shown_reference().map(|r| r.audio.len() as f64 / crate::dev::oracle::RATE as f64)
    }

    /// Our audio and the original engine's on one time axis, each with its
    /// sounds, the same sounds joined between them; a play button on each
    /// switches between them at the same moment.
    pub(super) fn original_lanes(&mut self, ui: &mut egui::Ui, position: Option<(Track, f64)>) {
        use speak::{our_marks, paint_axis, paint_chips, paint_playhead, paint_strip, paint_wave, Axis, Head};
        const WAVE: f32 = 72.0;
        const STRIP: f32 = 22.0;
        const LINKS: f32 = 16.0;
        const AXIS: f32 = 18.0;
        let samples = self.original_samples();
        let mut toggle = None;
        let playing = self.playing;
        let paused = self.player.is_paused();
        egui::Frame::group(ui.style()).fill(GROUND).show(ui, |ui| {
            ui.set_width(ui.available_width());
            let (Some(p), Some(v)) = (&self.preview, &self.voice) else {
                ui.allocate_space(vec2(ui.available_width(), 2.0 * (WAVE + STRIP) + LINKS + AXIS));
                return;
            };
            let key = (p.want.voice.clone(), p.want.text.clone());
            let reference = self.dev.refs.get(&key);
            let error = self.dev.ref_errors.get(&key);
            ui.horizontal(|ui| {
                if lane_button(ui, playing == Some(Track::Ours) && !paused, true) {
                    toggle = Some(Track::Ours);
                }
                ui.label(RichText::new("quoquendo").strong().small());
            });
            let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 2.0 * (WAVE + STRIP) + LINKS + AXIS), egui::Sense::hover());
            let painter = ui.painter_at(rect.expand(2.0));
            let ours_s = p.speech.duration_s() as f64;
            let orig_s = reference.map(|r| r.audio.len() as f64 / crate::dev::oracle::RATE as f64);
            let axis = Axis::new(rect, ours_s.max(orig_s.unwrap_or(0.0)));
            let row = |top: f32, h: f32| egui::Rect::from_min_size(egui::pos2(rect.left(), top), vec2(rect.width(), h));
            let our_wave = row(rect.top(), WAVE);
            let our_strip = row(our_wave.bottom(), STRIP);
            let links = row(our_strip.bottom(), LINKS);
            let their_strip = row(links.bottom(), STRIP);
            let their_wave = row(their_strip.bottom(), WAVE);
            let ticks = row(their_wave.bottom(), AXIS);
            let t = position.map(|(_, t)| t);
            let hover = self.hover_time(&response, rect.with_max_y(ticks.top()), &axis, ours_s.max(orig_s.unwrap_or(0.0)));
            let on = |track: Track| position.filter(|(k, _)| *k == track).map(|(_, t)| t);
            let ours = our_marks(&p.speech, &v.db);
            let theirs = reference.map(|r| original_marks(r, &v.db));
            paint_wave(&painter, our_wave, &axis, &p.speech.samples, p.speech.rate, on(Track::Ours), AMBER);
            paint_strip(&painter, our_strip, &axis, &ours, true);
            match (&theirs, &samples) {
                (Some(theirs), Some(s)) => {
                    paint_links(&painter, links, &axis, &ours, theirs);
                    paint_strip(&painter, their_strip, &axis, theirs, false);
                    paint_wave(&painter, their_wave, &axis, s, crate::dev::oracle::RATE, on(Track::Original), WAVE_ORIGINAL);
                }
                _ => {
                    let lane = links.union(their_wave).shrink2(vec2(8.0, 4.0));
                    let dash = egui::Stroke::new(1.0f32, STROKE.gamma_multiply(1.6));
                    let c = [lane.left_top(), lane.right_top(), lane.right_bottom(), lane.left_bottom(), lane.left_top()];
                    painter.extend(egui::Shape::dashed_line(&c, dash, 4.0, 3.0));
                    match error {
                        Some(e) => {
                            let galley = painter.layout(e.clone(), egui::FontId::proportional(13.0), RED, lane.width() - 24.0);
                            painter.galley(lane.center() - galley.size() / 2.0, galley, RED);
                        }
                        None => {
                            ui.put(
                                lane,
                                egui::Label::new(RichText::new("asking the original engine (under Wine)…").color(MUTED)).selectable(false),
                            );
                            let at = lane.center() - vec2(150.0, 0.0);
                            ui.put(egui::Rect::from_center_size(at, vec2(16.0, 16.0)), egui::Spinner::new());
                        }
                    }
                }
            }
            paint_axis(&painter, ticks, &axis);
            if let (Some(t), Some((track, _))) = (t, position) {
                let x = axis.x(t);
                let head = |lane: Track| if track == lane { Head::Playing } else { Head::Other };
                paint_playhead(&painter, our_wave.top(), our_strip.bottom(), x, head(Track::Ours));
                if reference.is_some() {
                    paint_playhead(&painter, their_strip.top(), their_wave.bottom(), x, head(Track::Original));
                }
            }
            if let Some(t) = hover {
                // one line across both lanes, at the same moment
                let bottom = if reference.is_some() { their_wave.bottom() } else { our_strip.bottom() };
                paint_playhead(&painter, our_wave.top(), bottom, axis.x(t), Head::Hover);
            }
            // each lane's sound at the playhead and under the pointer
            paint_chips(&painter, our_strip, &axis, &ours, t, hover, true);
            if let (Some(theirs), Some(_)) = (&theirs, &samples) {
                paint_chips(&painter, their_strip, &axis, theirs, t, hover, false);
            }
            ui.horizontal(|ui| {
                if lane_button(ui, playing == Some(Track::Original) && !paused, reference.is_some()) {
                    toggle = Some(Track::Original);
                }
                ui.label(RichText::new("original (Wine)").strong().small());
                if let Some(o) = orig_s {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let delta = ((ours_s - o) * 1000.0).round();
                        let sign = if delta < 0.0 { "−" } else { "+" };
                        ui.label(RichText::new(format!("{sign}{:.0} ms", delta.abs())).monospace().color(AMBER));
                        ui.label(RichText::new(format!("original {o:.2} s · ours {ours_s:.2} s ·")).monospace().color(MUTED));
                    });
                }
            });
        });
        if let Some(track) = toggle {
            if self.playing == Some(track) {
                // this lane: go on when paused, else stop
                if paused {
                    self.player.resume();
                } else {
                    self.stop();
                }
            } else {
                // the other lane playing or paused: carry on from (or stay
                // paused at) the same moment
                let from = position.map_or(0.0, |(_, t)| t);
                self.play(track, from);
                if paused {
                    self.player.pause();
                }
            }
        }
    }
}

/// A lane's play/stop button.
#[cfg(not(target_arch = "wasm32"))]
fn lane_button(ui: &mut egui::Ui, playing: bool, enabled: bool) -> bool {
    ui.add_enabled(enabled, egui::Button::new(if playing { "■" } else { "▶" }).small()).clicked()
}

/// The original engine's sounds, timed by the durations it reported; word
/// starts from its transcription. It reports each phone as its two
/// half-phones cut to whole ms (pauses exact), about 1 ms short per phone:
/// what that loses against its audio goes back evenly to every phone.
#[cfg(not(target_arch = "wasm32"))]
fn original_marks(r: &crate::dev::oracle::Reference, db: &quoquendo::loquendo::VoiceDatabase) -> Vec<speak::Mark> {
    use crate::dev::oracle::is_pause;
    let starts: Vec<bool> = r.target_phrases().iter().flat_map(|p| p.phones.iter().map(|t| t.word_start)).collect();
    let said = r.phones.iter().filter(|(s, _)| !is_pause(s)).count().max(1) as f64;
    let reported: u32 = r.phones.iter().map(|(_, ms)| ms).sum();
    let lost = (r.audio.len() as f64 / crate::dev::oracle::RATE as f64 - reported as f64 / 1000.0) / said;
    let mut k = 0;
    let mut t = 0.0;
    r.phones
        .iter()
        .map(|(sym, ms)| {
            let pause = is_pause(sym);
            let start = t;
            t += *ms as f64 / 1000.0 + if pause { 0.0 } else { lost };
            let word_start = !pause && {
                k += 1;
                starts.get(k - 1).copied().unwrap_or(false)
            };
            speak::Mark {
                sym: if pause { String::new() } else { sym.clone() },
                start,
                end: t,
                pause,
                word_start,
                known: pause || db.knows_symbol(sym),
            }
        })
        .collect()
}

/// A line from each of our sounds to the same sound in the original's,
/// amber where they start more than 40 ms apart.
#[cfg(not(target_arch = "wasm32"))]
fn paint_links(painter: &egui::Painter, rect: egui::Rect, axis: &speak::Axis, ours: &[speak::Mark], theirs: &[speak::Mark]) {
    let mut j = 0;
    for a in ours.iter().filter(|m| !m.pause && m.end > m.start) {
        let Some(k) = (j..theirs.len().min(j + 6)).find(|&k| !theirs[k].pause && theirs[k].sym == a.sym) else { continue };
        j = k + 1;
        let b = &theirs[k];
        let color = if (a.start - b.start).abs() > 0.04 { AMBER } else { WAVE_DIM };
        painter.line_segment([egui::pos2(axis.x(a.start), rect.top()), egui::pos2(axis.x(b.start), rect.bottom())], egui::Stroke::new(1.0f32, color));
    }
}
