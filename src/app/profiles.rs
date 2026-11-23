//! The Settings window: the active speech profile and its rules, plus the
//! backup/restore that carries every setting to another machine.

use super::*;
use quoquendo::profile::{Dates, Emoji, Joins, Ranges, Replacement, Year};

#[derive(Clone, Copy, PartialEq)]
enum Action {
    None,
    Duplicate,
    Delete,
    Import,
    Export,
}

impl Action {
    fn is(self) -> bool {
        self != Action::None
    }
}

/// Everything a backup carries.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Backup {
    #[serde(default)]
    profiles: ProfileBook,
    #[serde(default)]
    advanced: bool,
    #[serde(default)]
    original: bool,
    #[serde(default)]
    mirror_url: String,
    #[serde(default)]
    format: String,
}

impl App {
    pub(super) fn profile_language(&self) -> Language {
        self.language
            .or_else(|| self.voice.as_ref().map(|v| v.descriptor.language()))
            .unwrap_or(Language::Italian)
    }

    /// The join style the active profile picked.
    pub(super) fn profile_join(&self) -> quoquendo::synth::JoinStyle {
        match self.profiles.active_profile().joins {
            Joins::Smooth => quoquendo::synth::JoinStyle::Smooth,
            Joins::Original => quoquendo::synth::JoinStyle::Original,
        }
    }

    pub(super) fn profiles_window(&mut self, ctx: &egui::Context) {
        if !self.profile_window {
            return;
        }
        let mut open = true;
        let names: Vec<String> = self.profiles.all().into_iter().map(|p| p.name).collect();
        let active_name = self.profiles.active.clone();
        let mut selected = active_name.clone();
        let original = self.profiles.active_profile();
        let mut p = original.clone();
        let lang = self.profile_language();
        let mut action = Action::None;
        let mut error = None;

        egui::Window::new("Settings")
            .open(&mut open)
            .default_size([760.0, 720.0])
            .min_width(520.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Profile");
                        egui::ComboBox::from_id_salt("profile_pick")
                            .selected_text(&selected)
                            .width(220.0)
                            .show_ui(ui, |ui| {
                                for name in &names {
                                    ui.selectable_value(&mut selected, name.clone(), name);
                                }
                            });
                        ui.add_space(8.0);
                        ui.label(RichText::new(if p.builtin { "Built-in, read-only" } else { "Custom" }).color(MUTED));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Name");
                        ui.add_enabled(!p.builtin, egui::TextEdit::singleline(&mut p.name).desired_width(220.0));
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Duplicate").clicked() {
                            action = Action::Duplicate;
                        }
                        let custom = !p.builtin;
                        if ui.add_enabled(custom, egui::Button::new("Delete")).clicked() {
                            action = Action::Delete;
                        }
                        if ui.add_enabled(custom, egui::Button::new("Export…")).clicked() {
                            action = Action::Export;
                        }
                        if ui.button("Import…").clicked() {
                            action = Action::Import;
                        }
                    });
                    if let Some(e) = &self.profile_error {
                        ui.label(RichText::new(e).color(RED));
                    }
                    if let Some(s) = &self.profile_status {
                        ui.label(RichText::new(s).color(GREEN));
                    }

                    ui.add_space(6.0);
                    ui.label(RichText::new("Try it").strong());
                    ui.add(
                        egui::TextEdit::singleline(&mut self.profile_try)
                            .hint_text("22/11/26, €10, @anna, 10-15")
                            .desired_width(f32::INFINITY),
                    );
                    if !self.profile_try.trim().is_empty() {
                        let applied = p.apply(&self.profile_try, lang);
                        ui.label(RichText::new(applied.text).color(AMBER));
                        for warning in applied.warnings {
                            ui.label(RichText::new(warning).color(RED).small());
                        }
                    }

                    ui.separator();
                    let enabled = !p.builtin;
                    ui.add_enabled_ui(enabled, |ui| {
                        egui::CollapsingHeader::new("Dates & numbers").default_open(true).show(ui, |ui| {
                            egui::Grid::new("profile_dates").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                                ui.label("Dates");
                                enum_combo(
                                    ui,
                                    "profile_dates_mode",
                                    &mut p.dates,
                                    &[
                                        (Dates::Full, "Day, month and year"),
                                        (Dates::Numeric, "Numbers"),
                                        (Dates::Verbatim, "As written"),
                                        (Dates::AsWritten, "Leave to the engine"),
                                    ],
                                );
                                ui.end_row();
                                ui.label("Two-digit years");
                                enum_combo(
                                    ui,
                                    "profile_year",
                                    &mut p.year,
                                    &[(Year::NextTen, "Next ten years, then the past"), (Year::Original, "The original's cutoff")],
                                );
                                ui.end_row();
                                ui.label("Ranges");
                                enum_combo(
                                    ui,
                                    "profile_ranges",
                                    &mut p.ranges,
                                    &[
                                        (Ranges::Words, "From 10 to 15"),
                                        (Ranges::Numbers, "10 15"),
                                        (Ranges::AsWritten, "Leave to the engine"),
                                    ],
                                );
                                ui.end_row();
                                ui.label("Formulas");
                                ui.checkbox(&mut p.math, "Read 2+2=4 as words");
                                ui.end_row();
                            });
                        });

                        egui::CollapsingHeader::new("Text & web").default_open(true).show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.checkbox(&mut p.inclusive, "Inclusive Italian (ə, ɜ, *)");
                                ui.checkbox(&mut p.units, "Units");
                                ui.checkbox(&mut p.handles, "Handles & hashtags");
                                ui.checkbox(&mut p.federated, "Fediverse, Matrix & Threads");
                                ui.checkbox(&mut p.emails, "Emails");
                                ui.checkbox(&mut p.links, "Links");
                                ui.checkbox(&mut p.cashtags, "Cashtags");
                                ui.checkbox(&mut p.currencies, "Currencies");
                                ui.checkbox(&mut p.crypto, "Crypto");
                            });
                            ui.horizontal(|ui| {
                                ui.label("Emoji");
                                enum_combo(ui, "profile_emoji", &mut p.emoji, &[(Emoji::Describe, "Describe"), (Emoji::Skip, "Skip")]);
                            });
                        });

                        egui::CollapsingHeader::new("Audio").default_open(true).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Joining");
                                enum_combo(
                                    ui,
                                    "profile_joins",
                                    &mut p.joins,
                                    &[(Joins::Original, "Like the original"), (Joins::Smooth, "Smoothed")],
                                );
                            });
                        });

                        egui::CollapsingHeader::new("Easter eggs").default_open(true).show(ui, |ui| {
                            ui.checkbox(&mut p.station_warning, "Italian station announcement");
                            ui.label(RichText::new(STATION_WARNING).small().color(MUTED));
                        });

                        egui::CollapsingHeader::new("Rules").default_open(true).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Language");
                                language_combo(ui, "profile_rules_lang", &mut p.rules_language);
                            });
                            ui.label("Say before");
                            ui.add(egui::TextEdit::multiline(&mut p.say_before).desired_rows(1).desired_width(f32::INFINITY));
                            ui.label("Say after");
                            ui.add(egui::TextEdit::multiline(&mut p.say_after).desired_rows(1).desired_width(f32::INFINITY));

                            ui.add_space(6.0);
                            ui.label(RichText::new("Replacements").strong());
                            egui::Grid::new("profile_replacements").num_columns(7).spacing([8.0, 4.0]).show(ui, |ui| {
                                for h in ["On", "Find", "Replace with", "Language", "Regex", "Match case", ""] {
                                    ui.label(RichText::new(h).small().color(MUTED));
                                }
                                ui.end_row();
                                let mut remove = None;
                                for (i, r) in p.replacements.iter_mut().enumerate() {
                                    ui.checkbox(&mut r.enabled, "");
                                    ui.add(egui::TextEdit::singleline(&mut r.find).desired_width(130.0));
                                    ui.add(egui::TextEdit::singleline(&mut r.replace).desired_width(130.0));
                                    language_combo(ui, &format!("profile_repl_lang_{i}"), &mut r.language);
                                    ui.checkbox(&mut r.regex, "");
                                    ui.checkbox(&mut r.match_case, "");
                                    if ui.small_button("×").clicked() {
                                        remove = Some(i);
                                    }
                                    ui.end_row();
                                }
                                if let Some(i) = remove {
                                    p.replacements.remove(i);
                                }
                            });
                            if ui.button("Add replacement").clicked() {
                                p.replacements.push(Replacement::default());
                            }
                        });
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Back up settings…").clicked() {
                            self.backup_settings();
                        }
                        if ui.button("Restore from file…").clicked() {
                            self.restore_settings();
                        }
                    });
                });
            });

        self.profile_window = open;
        if selected != active_name {
            self.profiles.set_active(&selected);
        }
        match action {
            Action::Duplicate => match self.profiles.duplicate(&active_name) {
                Some(name) => {
                    self.profile_status = Some(format!("Created {name}"));
                    self.profile_error = None;
                }
                None => self.profile_error = Some("no profile to duplicate".into()),
            },
            Action::Delete => {
                if confirm_delete(&p.name) {
                    match self.profiles.remove(&p.name) {
                        Ok(()) => {
                            self.profile_status = Some(format!("Deleted {}", p.name));
                            self.profile_error = None;
                        }
                        Err(e) => self.profile_error = Some(e),
                    }
                }
            }
            Action::Export => match self.profiles.export(&p.name) {
                Ok(json) => match save_json(&format!("{}.json", p.name), &json) {
                    Ok(Some(path)) => self.profile_status = Some(format!("Exported {}", path.display())),
                    Ok(None) => {}
                    Err(e) => self.profile_error = Some(e),
                },
                Err(e) => self.profile_error = Some(e),
            },
            Action::Import => match pick_json() {
                Some(json) => match self.profiles.import(&json) {
                    Ok(name) => {
                        self.profile_status = Some(format!("Imported {name}"));
                        self.profile_error = None;
                    }
                    Err(e) => self.profile_error = Some(e),
                },
                None => {}
            },
            Action::None => {}
        }
        if selected == active_name && !p.builtin && p != original && !action.is() {
            if let Err(e) = self.profiles.replace(&original.name, p) {
                error = Some(e);
            }
        }
        if let Some(e) = error {
            self.profile_error = Some(e);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn backup_settings(&mut self) {
        let backup = Backup {
            profiles: self.profiles.clone(),
            advanced: self.advanced,
            original: self.dev.original,
            mirror_url: self.mirror_url.clone(),
            format: self.format.ext().to_string(),
        };
        let json = serde_json::to_string_pretty(&backup).unwrap_or_default();
        match save_json("quoquendo-settings.json", &json) {
            Ok(Some(path)) => self.profile_status = Some(format!("Backed up to {}", path.display())),
            Ok(None) => {}
            Err(e) => self.profile_error = Some(e),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn restore_settings(&mut self) {
        let Some(json) = pick_json() else { return };
        match serde_json::from_str::<Backup>(&json) {
            Ok(backup) => {
                let json = serde_json::to_string(&backup.profiles).unwrap_or_default();
                match ProfileBook::from_json(&json) {
                    Ok(profiles) => self.profiles = profiles,
                    Err(e) => {
                        self.profile_error = Some(e);
                        return;
                    }
                }
                self.advanced = backup.advanced;
                self.dev.original = backup.original;
                self.mirror_url = backup.mirror_url;
                if let Some(f) = Format::from_ext(&backup.format) {
                    self.format = f;
                }
                self.relist();
                self.profile_status = Some("Settings restored".into());
                self.profile_error = None;
            }
            Err(e) => self.profile_error = Some(format!("not a settings backup: {e}")),
        }
    }
}

fn enum_combo<T: Copy + PartialEq>(ui: &mut egui::Ui, id: &str, value: &mut T, options: &[(T, &str)]) {
    let selected = options.iter().find(|(v, _)| v == value).map_or("", |(_, n)| *n);
    egui::ComboBox::from_id_salt(id).selected_text(selected).show_ui(ui, |ui| {
        for (v, name) in options {
            ui.selectable_value(value, *v, *name);
        }
    });
}

fn language_combo(ui: &mut egui::Ui, id: &str, value: &mut Option<String>) {
    let selected = value.as_deref().unwrap_or("All languages");
    egui::ComboBox::from_id_salt(id).selected_text(selected).show_ui(ui, |ui| {
        ui.selectable_value(value, None, "All languages");
        for l in Language::ALL {
            ui.selectable_value(value, Some(l.label().to_string()), l.label());
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn confirm_delete(name: &str) -> bool {
    matches!(
        rfd::MessageDialog::new()
            .set_title("Delete profile")
            .set_description(format!("Delete “{name}”? This cannot be undone."))
            .set_buttons(rfd::MessageButtons::YesNo)
            .show(),
        rfd::MessageDialogResult::Yes
    )
}

#[cfg(target_arch = "wasm32")]
fn confirm_delete(_name: &str) -> bool {
    false
}

#[cfg(not(target_arch = "wasm32"))]
fn save_json(name: &str, json: &str) -> Result<Option<std::path::PathBuf>, String> {
    let Some(path) = rfd::FileDialog::new().set_file_name(name).add_filter("JSON", &["json"]).save_file() else {
        return Ok(None);
    };
    std::fs::write(&path, json).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Some(path))
}

#[cfg(target_arch = "wasm32")]
fn save_json(_name: &str, _json: &str) -> Result<Option<std::path::PathBuf>, String> {
    Err("exporting files is not available in the browser yet".into())
}

#[cfg(not(target_arch = "wasm32"))]
fn pick_json() -> Option<String> {
    let path = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file()?;
    std::fs::read_to_string(&path).ok()
}

#[cfg(target_arch = "wasm32")]
fn pick_json() -> Option<String> {
    None
}
