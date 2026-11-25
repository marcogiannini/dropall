// SPDX-License-Identifier: GPL-3.0-or-later
//! Torrent: righe dell'elenco, opzioni e funzioni per aprire file .torrent.
use super::{card, open_folder, progress_row, App, Msg, ERR, OK, WARN};
use crate::dm::{fmt_hms, fmt_size, fmt_speed};
use crate::torrent::{TState, TorrentItem};
use crate::tr;
use eframe::egui::{self, Color32, RichText};
use std::path::PathBuf;
use std::thread;

#[derive(Default)]
pub struct TorUi {
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
        TState::Downloading => super::ACCENT_TEXT,
        TState::Finished | TState::Seeding => OK,
        TState::Paused => WARN,
        TState::Error => ERR,
    }
}

impl App {
    /// Finestra "apri file": mostra i .torrent (e, come ripiego, tutti i file).
    pub(super) fn pick_torrent_files(&self, ctx: &egui::Context) {
        let send = self.sender(ctx);
        thread::spawn(move || {
            if let Some(files) = rfd::FileDialog::new()
                .set_title(tr!("Scegli i file .torrent", "Choose .torrent files"))
                .add_filter("Torrent", &["torrent", "TORRENT"])
                .add_filter(tr!("Tutti i file", "All files"), &["*"])
                .pick_files()
            {
                send(Msg::TorFiles(files));
            }
        });
    }

    pub(super) fn torrent_options_card(&mut self, ui: &mut egui::Ui) {
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
                    "Il limite di download è quello generale (nelle Impostazioni, «Limite di velocità») e vale anche per i torrent.",
                    "The download limit is the general one (Settings, “Speed limit”) and also applies to torrents."
                ))
                .small()
                .color(ui.visuals().weak_text_color()),
            );
        });
    }

    pub(super) fn torrent_row(&mut self, ui: &mut egui::Ui, it: &TorrentItem) {
        let weak = ui.visuals().weak_text_color();
        egui::Frame::group(ui.style())
            .fill(super::CARD_BG)
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
                    super::home::kind_tag(ui, super::home::Kind::Torrent);
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
