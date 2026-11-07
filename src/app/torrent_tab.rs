// SPDX-License-Identifier: GPL-3.0-or-later
//! Scheda "Torrent": link magnet, indirizzi di file .torrent e file .torrent locali.
use super::{card, open_folder, progress_row, App, Msg, ACCENT, ERR, OK, ON_ACCENT, WARN};
use crate::dm::{fmt_hms, fmt_size, fmt_speed};
use crate::torrent::{TState, TorrentItem};
use crate::tr;
use eframe::egui::{self, Color32, RichText};
use std::path::PathBuf;
use std::thread;

#[derive(Default)]
pub struct TorUi {
    pub input: String,
    /// info-hash del torrent per cui si sta confermando "elimina con i file"
    pub confirm_delete: Option<String>,
    pub notes: Vec<String>,
    pub up_val: u32,
}

fn state_text(s: TState) -> &'static str {
    match s {
        TState::Checking => tr!("Controllo file", "Checking files"),
        TState::Metadata => tr!("Ricerca informazioni", "Fetching info"),
        TState::Downloading => tr!("In corso", "Downloading"),
        TState::Finished => tr!("Completato", "Completed"),
        TState::Seeding => tr!("Completato · in condivisione", "Completed · seeding"),
        TState::Paused => tr!("In pausa", "Paused"),
        TState::Error => tr!("Errore", "Error"),
    }
}

fn state_color(s: TState, weak: Color32) -> Color32 {
    match s {
        TState::Checking | TState::Metadata => weak,
        TState::Downloading => ACCENT,
        TState::Finished | TState::Seeding => OK,
        TState::Paused => WARN,
        TState::Error => ERR,
    }
}

impl App {
    pub(super) fn torrent_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        for n in self.hub.take_notes() {
            self.tor_ui.notes.push(n);
        }
        if self.tor_ui.up_val == 0 {
            self.tor_ui.up_val = if self.cfg.torrent_up_kbps > 0 {
                self.cfg.torrent_up_kbps
            } else {
                512
            };
        }
        self.torrent_add_card(ui, ctx);
        ui.add_space(8.0);
        self.torrent_options_card(ui);
        ui.add_space(8.0);
        let items = self.hub.snapshot();
        self.torrent_list(ui, &items);
    }

    fn torrent_add_card(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let field_id = ui.scope_id().with("tor_field");
        card(ui, |ui| {
            let mut paste = false;
            let mut label_id = None;
            ui.horizontal(|ui| {
                label_id = Some(
                    ui.label(
                        RichText::new(tr!(
                            "Link magnet o indirizzi .torrent",
                            "Magnet links or .torrent addresses"
                        ))
                        .strong(),
                    )
                    .id,
                );
                paste = ui.small_button(tr!("Incolla", "Paste")).clicked();
                if ui.small_button(tr!("Svuota", "Clear")).clicked() {
                    self.tor_ui.input.clear();
                }
            });
            let resp = ui.add(
                egui::TextEdit::multiline(&mut self.tor_ui.input)
                    .id(field_id)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY)
                    .hint_text(tr!(
                        "Incolla qui link magnet:… o indirizzi di file .torrent, uno per riga",
                        "Paste magnet:… links or .torrent file addresses here, one per line"
                    )),
            );
            if let Some(id) = label_id {
                resp.labelled_by(id);
            }
            if paste {
                ui.memory_mut(|m| m.request_focus(field_id));
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::RequestPaste);
            }
            ui.add_space(6.0);
            egui::Grid::new("tor_opts")
                .num_columns(2)
                .spacing([14.0, 8.0])
                .show(ui, |ui| {
                    ui.label(tr!("Salva in", "Save to"));
                    ui.horizontal(|ui| {
                        if ui.button(tr!("Sfoglia…", "Browse…")).clicked() {
                            let send = self.sender(ctx);
                            let start = self.cfg.torrent_dir.clone();
                            thread::spawn(move || {
                                if let Some(p) = rfd::FileDialog::new()
                                    .set_title(tr!(
                                        "Scegli la cartella di destinazione",
                                        "Choose the destination folder"
                                    ))
                                    .set_directory(start)
                                    .pick_folder()
                                {
                                    send(Msg::TorFolder(p));
                                }
                            });
                        }
                        ui.add(egui::Label::new(self.cfg.torrent_dir.to_string_lossy()).truncate());
                    });
                    ui.end_row();
                });
            ui.add_space(8.0);
            let has_input = self.tor_ui.input.lines().any(|l| !l.trim().is_empty());
            ui.horizontal_wrapped(|ui| {
                let (fill, text) = if has_input {
                    (ACCENT, ON_ACCENT)
                } else {
                    (
                        Color32::from_rgb(44, 44, 49),
                        Color32::from_rgb(130, 130, 138),
                    )
                };
                let sense = if has_input {
                    egui::Sense::click()
                } else {
                    egui::Sense::hover()
                };
                let label = tr!("Aggiungi e scarica", "Add and download");
                let r = ui.add(
                    egui::Button::new(RichText::new(label).strong().color(text))
                        .fill(fill)
                        .sense(sense)
                        .min_size(egui::vec2(0.0, 32.0)),
                );
                r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, has_input, label));
                if r.clicked() {
                    let lines: Vec<String> = self
                        .tor_ui
                        .input
                        .lines()
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty())
                        .collect();
                    self.hub.add_lines(lines, self.cfg.torrent_dir.clone());
                    self.tor_ui.input.clear();
                }
                let open = tr!("Apri file .torrent…", "Open .torrent file…");
                if ui
                    .add(egui::Button::new(open).min_size(egui::vec2(0.0, 32.0)))
                    .clicked()
                {
                    let send = self.sender(ctx);
                    thread::spawn(move || {
                        if let Some(files) = rfd::FileDialog::new()
                            .set_title(tr!("Scegli i file .torrent", "Choose .torrent files"))
                            .add_filter("Torrent", &["torrent"])
                            .pick_files()
                        {
                            send(Msg::TorFiles(files));
                        }
                    });
                }
            });
            ui.label(
                RichText::new(tr!(
                    "Puoi anche trascinare qui i file .torrent, oppure aprire un link magnet dal browser: Grabbit lo aggiunge da solo.",
                    "You can also drag .torrent files here, or open a magnet link from your browser: Grabbit adds it by itself."
                ))
                .small()
                .color(ui.visuals().weak_text_color()),
            );
            ui.label(
                RichText::new(tr!(
                    "Scarica solo contenuti che hai il diritto di scaricare e condividere.",
                    "Only download content you have the right to download and share."
                ))
                .small()
                .color(ui.visuals().weak_text_color()),
            );
            let mut clear = false;
            for n in &self.tor_ui.notes {
                ui.label(RichText::new(n).small().color(ERR));
                clear = true;
            }
            if clear
                && ui
                    .small_button(tr!("Nascondi errori", "Dismiss errors"))
                    .clicked()
            {
                self.tor_ui.notes.clear();
            }
        });
    }

    fn torrent_options_card(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            ui.label(RichText::new(tr!("Opzioni torrent", "Torrent options")).strong());
            if ui
                .checkbox(
                    &mut self.cfg.torrent_seed,
                    tr!(
                        "Continua a condividere (seed) dopo il download",
                        "Keep sharing (seeding) after the download"
                    ),
                )
                .changed()
            {
                self.hub.set_seed_after(self.cfg.torrent_seed);
            }
            let before = self.cfg.torrent_up_kbps;
            let mut on = before > 0;
            ui.horizontal_wrapped(|ui| {
                if ui
                    .checkbox(&mut on, tr!("Limita l'upload a", "Limit upload to"))
                    .changed()
                {
                    self.cfg.torrent_up_kbps = if on { self.tor_ui.up_val.max(1) } else { 0 };
                }
                ui.add_enabled_ui(on, |ui| {
                    let mut v = if on {
                        self.cfg.torrent_up_kbps
                    } else {
                        self.tor_ui.up_val
                    };
                    let r = ui.add(
                        egui::DragValue::new(&mut v)
                            .range(1..=10_000_000)
                            .speed(8)
                            .suffix(" KB/s"),
                    );
                    r.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::Role::SpinButton,
                            true,
                            tr!(
                                "Limite di upload in KB al secondo",
                                "Upload limit in KB per second"
                            ),
                        )
                    });
                    if on && v != self.cfg.torrent_up_kbps {
                        self.cfg.torrent_up_kbps = v;
                    }
                });
            });
            if self.cfg.torrent_up_kbps > 0 {
                self.tor_ui.up_val = self.cfg.torrent_up_kbps;
            }
            if self.cfg.torrent_up_kbps != before {
                self.hub
                    .set_limits(self.cfg.speed_limit_kbps, self.cfg.torrent_up_kbps);
            }
            ui.label(
                RichText::new(tr!(
                    "Il limite di download è quello della scheda Download manager e vale anche per i torrent.",
                    "The download limit is the one in the Download manager tab and also applies to torrents."
                ))
                .small()
                .color(ui.visuals().weak_text_color()),
            );
        });
    }

    fn torrent_list(&mut self, ui: &mut egui::Ui, items: &[TorrentItem]) {
        if items.is_empty() {
            ui.add_space(6.0);
            ui.label(
                RichText::new(tr!(
                    "Nessun torrent. Incolla un link magnet qui sopra o apri un file .torrent per iniziare.",
                    "No torrents yet. Paste a magnet link above or open a .torrent file to start."
                ))
                .color(ui.visuals().weak_text_color()),
            );
            return;
        }
        for it in items {
            self.torrent_row(ui, it);
            ui.add_space(6.0);
        }
    }

    fn torrent_row(&mut self, ui: &mut egui::Ui, it: &TorrentItem) {
        let weak = ui.visuals().weak_text_color();
        egui::Frame::group(ui.style())
            .corner_radius(8)
            .inner_margin(10)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let col = state_color(it.state, weak);
                let state = state_text(it.state);
                let name = if it.name.is_empty() {
                    it.hash.clone()
                } else {
                    it.name.clone()
                };
                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(12.0, 14.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 5.0, col);
                    let r = ui.add(egui::Label::new(RichText::new(&name).strong()).truncate());
                    r.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::Role::Label,
                            true,
                            format!("{name}: {state}"),
                        )
                    });
                    ui.label(RichText::new(state).small().color(col));
                });
                if it.has_metadata && it.total > 0 {
                    progress_row(ui, it.fraction());
                }
                let mut parts: Vec<String> = Vec::new();
                if it.has_metadata && it.total > 0 {
                    parts.push(format!("{} / {}", fmt_size(it.done), fmt_size(it.total)));
                }
                if it.state == TState::Downloading {
                    parts.push(fmt_speed(it.down_rate as f64));
                    if let Some(e) = it.eta_secs() {
                        parts.push(tr!(
                            format!("ancora {}", fmt_hms(e)),
                            format!("{} left", fmt_hms(e))
                        ));
                    }
                }
                if it.up_rate > 0 {
                    parts.push(format!(
                        "{} {}",
                        tr!("upload", "upload"),
                        fmt_speed(it.up_rate as f64)
                    ));
                }
                if it.state != TState::Paused && it.state != TState::Error {
                    parts.push(tr!(
                        format!("{} connessioni, {} seed", it.peers, it.seeds),
                        format!("{} peers, {} seeds", it.peers, it.seeds)
                    ));
                }
                if !parts.is_empty() {
                    ui.label(RichText::new(parts.join(" · ")).small().color(weak));
                }
                if it.state == TState::Error && !it.error.is_empty() {
                    ui.label(RichText::new(&it.error).small().color(ERR));
                }
                ui.label(RichText::new(&it.save_path).small().color(weak));
                ui.horizontal_wrapped(|ui| {
                    match it.state {
                        TState::Paused | TState::Error => {
                            let l = if it.state == TState::Error {
                                tr!("Riprova", "Retry")
                            } else {
                                tr!("Riprendi", "Resume")
                            };
                            if ui.small_button(l).clicked() {
                                self.hub.resume(&it.hash);
                            }
                        }
                        TState::Finished => {
                            if ui
                                .small_button(tr!("Condividi di nuovo", "Seed again"))
                                .clicked()
                            {
                                self.hub.resume(&it.hash);
                            }
                        }
                        _ => {
                            let l = if it.state == TState::Seeding {
                                tr!("Smetti di condividere", "Stop seeding")
                            } else {
                                tr!("Pausa", "Pause")
                            };
                            if ui.small_button(l).clicked() {
                                self.hub.pause(&it.hash);
                            }
                        }
                    }
                    if ui
                        .small_button(tr!("Apri cartella", "Open folder"))
                        .clicked()
                    {
                        open_folder(&PathBuf::from(&it.save_path));
                    }
                    if ui
                        .small_button(tr!("Rimuovi dall'elenco", "Remove from list"))
                        .clicked()
                    {
                        self.hub.remove(&it.hash, false);
                    }
                    if self.tor_ui.confirm_delete.as_deref() == Some(&it.hash) {
                        if ui
                            .small_button(
                                RichText::new(tr!(
                                    "Conferma: elimina anche i file",
                                    "Confirm: delete files too"
                                ))
                                .color(ERR),
                            )
                            .clicked()
                        {
                            self.hub.remove(&it.hash, true);
                            self.tor_ui.confirm_delete = None;
                        }
                    } else if ui
                        .small_button(tr!("Elimina con i file…", "Delete with files…"))
                        .clicked()
                    {
                        self.tor_ui.confirm_delete = Some(it.hash.clone());
                    }
                });
            });
    }
}
