// SPDX-License-Identifier: GPL-3.0-or-later
//! Schermata principale: un solo campo per video, file e torrent (il tipo si riconosce da solo)
//! e un'unica lista di download con filtri. Le impostazioni stanno in un pannello a parte.
use super::dm_tab::CONNECTIONS;
use super::{
    accent_toggle, card, edit_menu, empty_state, paste_fallback, App, Msg, ACCENT, DISABLED_FILL,
    DISABLED_TEXT, ERR, ON_ACCENT, WARN,
};
use crate::dm::State;
use crate::torrent::TState;
use crate::tr;
use eframe::egui::{self, Color32, RichText};
use std::thread;

/// Tipo di un link, deciso da Dropall (o scelto a mano).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Media,
    File,
    Torrent,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Media => tr!("Video e musica", "Video & music"),
            Kind::File => tr!("File", "File"),
            Kind::Torrent => "Torrent",
        }
    }

    /// (sfondo, testo): tinte scure con testo chiaro, contrasto oltre 7:1.
    fn colors(self) -> (Color32, Color32) {
        match self {
            Kind::Media => (
                Color32::from_rgb(54, 36, 22),
                Color32::from_rgb(253, 186, 116),
            ),
            Kind::File => (
                Color32::from_rgb(25, 48, 34),
                Color32::from_rgb(134, 239, 172),
            ),
            Kind::Torrent => (
                Color32::from_rgb(27, 38, 58),
                Color32::from_rgb(147, 197, 253),
            ),
        }
    }
}

/// Riconosce il tipo di una riga: magnet e .torrent, YouTube e Spotify, altrimenti un file.
pub fn detect(line: &str) -> Kind {
    let l = line.trim();
    if crate::torrent::is_torrent_source(l) {
        return Kind::Torrent;
    }
    let lower = l.to_lowercase();
    let rest = lower.split("://").nth(1).unwrap_or(&lower);
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit('@').next().unwrap_or(host);
    let host = host.split(':').next().unwrap_or(host);
    let is = |d: &str| host == d || host.ends_with(&format!(".{d}"));
    if is("youtube.com")
        || is("youtube-nocookie.com")
        || host == "youtu.be"
        || is("spotify.com")
        || host == "spotify.link"
    {
        Kind::Media
    } else {
        Kind::File
    }
}

/// Etichetta piccola con il tipo, davanti al nome nell'elenco. Il tipo è scritto a parole.
pub fn kind_tag(ui: &mut egui::Ui, kind: Kind) {
    let (bg, fg) = kind.colors();
    egui::Frame::new()
        .fill(bg)
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(7, 1))
        .show(ui, |ui| {
            ui.label(RichText::new(kind.name()).small().strong().color(fg));
        });
}

/// A che punto è un download: ancora da fare, finito bene o finito male.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    Active,
    Done,
    Failed,
}

/// Filtro dello storico (i download in corso stanno nel loro pannello).
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    Done,
    Failed,
}

impl Filter {
    /// Nello storico finiscono solo i download conclusi, con successo o con un errore.
    pub fn accepts(self, c: Class) -> bool {
        match self {
            Filter::All => c != Class::Active,
            Filter::Done => c == Class::Done,
            Filter::Failed => c == Class::Failed,
        }
    }
}

#[derive(Default)]
pub struct HomeUi {
    pub input: String,
    /// tipo scelto a mano al posto di quello riconosciuto
    pub forced: Option<Kind>,
    pub filter: Filter,
    pub settings: bool,
}

fn lines_of(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

impl App {
    pub(super) fn home(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
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
        if !self.tor_ui.notes.is_empty() {
            card(ui, |ui| {
                for n in &self.tor_ui.notes {
                    ui.label(RichText::new(n).color(ERR));
                }
                if ui.small_button(tr!("Nascondi", "Dismiss")).clicked() {
                    self.tor_ui.notes.clear();
                }
            });
            ui.add_space(8.0);
        }
        self.smart_card(ui, ctx);
        ui.add_space(10.0);
        self.list_section(ui);
    }

    fn kind_of(&self, line: &str) -> Kind {
        self.home.forced.unwrap_or_else(|| detect(line))
    }

    fn smart_card(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let field_id = ui.scope_id().with("smart_field");
        let lines = lines_of(&self.home.input);
        let kinds: Vec<Kind> = lines.iter().map(|l| self.kind_of(l)).collect();
        let has = |k: Kind| kinds.contains(&k);
        let (has_media, has_file, has_tor) =
            (has(Kind::Media), has(Kind::File), has(Kind::Torrent));
        let has_input = !lines.is_empty();

        let mut paste = false;
        card(ui, |ui| {
            let mut label_id = None;
            ui.horizontal(|ui| {
                label_id = Some(
                    ui.label(
                        RichText::new(tr!("Incolla un link", "Paste a link"))
                            .size(17.0)
                            .strong(),
                    )
                    .id,
                );
                paste = ui.small_button(tr!("Incolla", "Paste")).clicked();
                if ui.small_button(tr!("Svuota", "Clear")).clicked() {
                    self.home.input.clear();
                }
                if ui
                    .small_button(tr!("Apri file .torrent…", "Open .torrent file…"))
                    .on_hover_text(tr!(
                        "Scegli uno o più file .torrent dal computer",
                        "Choose one or more .torrent files from your computer"
                    ))
                    .clicked()
                {
                    self.pick_torrent_files(ctx);
                }
            });
            let resp = ui.add(
                egui::TextEdit::multiline(&mut self.home.input)
                    .id(field_id)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .hint_text(tr!(
                        "Un link di YouTube o Spotify, l'indirizzo di un file o un link magnet. Uno per riga.",
                        "A YouTube or Spotify link, a file address or a magnet link. One per line."
                    )),
            );
            paste_fallback(ui.ctx(), &resp);
            edit_menu(&resp, &mut self.home.input);
            if let Some(id) = label_id {
                resp.labelled_by(id);
            }

            // tipo riconosciuto + scelta manuale
            ui.horizontal_wrapped(|ui| {
                if has_input {
                    ui.label(
                        RichText::new(tr!("Riconosciuto:", "Detected:"))
                            .color(ui.visuals().weak_text_color()),
                    );
                    let mut shown = Vec::new();
                    for k in [Kind::Media, Kind::File, Kind::Torrent] {
                        let n = kinds.iter().filter(|x| **x == k).count();
                        if n > 0 {
                            shown.push((k, n));
                        }
                    }
                    for (k, n) in shown {
                        let (bg, fg) = k.colors();
                        egui::Frame::new()
                            .fill(bg)
                            .corner_radius(8)
                            .inner_margin(egui::Margin::symmetric(10, 3))
                            .show(ui, |ui| {
                                let text = if n > 1 {
                                    format!("{} ({n})", k.name())
                                } else {
                                    k.name().to_string()
                                };
                                ui.label(RichText::new(text).strong().color(fg));
                            });
                    }
                } else {
                    ui.label(
                        RichText::new(tr!(
                            "Dropall capisce da solo se è un video, un file o un torrent.",
                            "Dropall works out by itself whether it is a video, a file or a torrent."
                        ))
                        .color(ui.visuals().weak_text_color()),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let current = match self.home.forced {
                        None => tr!("Automatico", "Automatic"),
                        Some(k) => k.name(),
                    };
                    egui::ComboBox::from_id_salt("home_forced")
                        .selected_text(current)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.home.forced,
                                None,
                                tr!("Automatico", "Automatic"),
                            );
                            for k in [Kind::Media, Kind::File, Kind::Torrent] {
                                ui.selectable_value(&mut self.home.forced, Some(k), k.name());
                            }
                        });
                    ui.label(tr!("Tratta come", "Treat as"));
                });
            });

            // pulsante principale
            ui.add_space(4.0);
            let ready = self.tools.ytdlp.is_some() && !self.tools.busy;
            let media_blocked = has_media && (!ready || self.running);
            let enabled = has_input && !media_blocked && self.cfg.terms_accepted;
            ui.horizontal(|ui| {
                let (fill, text) = if enabled {
                    (ACCENT, ON_ACCENT)
                } else {
                    (DISABLED_FILL, DISABLED_TEXT)
                };
                let label = tr!("Scarica", "Download");
                let btn = egui::Button::new(
                    RichText::new(label).size(18.0).strong().color(text),
                )
                .fill(fill)
                .min_size(egui::vec2(200.0, 44.0))
                .sense(if enabled {
                    egui::Sense::click()
                } else {
                    egui::Sense::hover()
                });
                let r = ui.add(btn);
                r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, label));
                if r.clicked() {
                    self.submit(ctx);
                }
                if has_media && self.running {
                    ui.label(
                        RichText::new(tr!(
                            "Un download di video è già in corso: aspetta che finisca o annullalo.",
                            "A video download is already running: wait for it to finish or cancel it."
                        ))
                        .small()
                        .color(WARN),
                    );
                }
            });
            if has_file
                && lines.iter().zip(&kinds).any(|(l, k)| {
                    *k == Kind::File && !l.starts_with("http://") && !l.starts_with("https://")
                })
            {
                ui.label(
                    RichText::new(tr!(
                        "Gli indirizzi dei file devono iniziare con http:// o https://",
                        "File addresses must start with http:// or https://"
                    ))
                    .small()
                    .color(WARN),
                );
            }
        });
        if paste {
            ui.memory_mut(|m| m.request_focus(field_id));
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
        }

        // opzioni: solo quelle che servono ai tipi riconosciuti
        if has_media {
            ui.add_space(8.0);
            self.options_card(ui, ctx);
        }
        if has_file {
            ui.add_space(8.0);
            self.file_options_card(ui, ctx);
        }
        if has_tor {
            ui.add_space(8.0);
            self.torrent_folder_card(ui, ctx);
        }
    }

    /// Manda ogni riga al motore giusto.
    fn submit(&mut self, ctx: &egui::Context) {
        if !self.cfg.terms_accepted {
            return;
        }
        let lines = lines_of(&self.home.input);
        let (mut media, mut files, mut tor) = (Vec::new(), Vec::new(), Vec::new());
        for l in lines {
            match self.kind_of(&l) {
                Kind::Media => media.push(l),
                Kind::File => files.push(l),
                Kind::Torrent => tor.push(l),
            }
        }
        let mut leftover = String::new();
        if !tor.is_empty() {
            self.add_torrent_inputs(tor);
        }
        if !files.is_empty() {
            let queues = self.dm.snapshot().queues;
            if !queues.iter().any(|q| q.id == self.dm_ui.queue) {
                self.dm_ui.queue = queues.first().map(|q| q.id).unwrap_or(1);
            }
            let n = self.dm.add(
                &files.join("\n"),
                &self.cfg.dm_dir,
                self.dm_ui.queue,
                self.cfg.dm_connections,
            );
            if n > 0 {
                self.dm.start_queue(self.dm_ui.queue);
            } else {
                // niente di valido: le righe restano nel campo per correggerle
                leftover = files.join("\n");
            }
        }
        if !media.is_empty() {
            self.input = media.join("\n");
            self.start_download(ctx);
        }
        self.home.input = leftover;
    }

    fn file_options_card(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        card(ui, |ui| {
            ui.label(RichText::new(tr!("Opzioni per i file", "Options for files")).strong());
            egui::Grid::new("home_file_opts")
                .num_columns(2)
                .spacing([14.0, 8.0])
                .show(ui, |ui| {
                    ui.label(tr!("Salva in", "Save to"));
                    ui.horizontal(|ui| {
                        if ui
                            .button(tr!("Cambia cartella…", "Change folder…"))
                            .clicked()
                        {
                            let send = self.sender(ctx);
                            let start = self.cfg.dm_dir.clone();
                            thread::spawn(move || {
                                if let Some(p) = rfd::FileDialog::new()
                                    .set_title(tr!(
                                        "Scegli la cartella di destinazione",
                                        "Choose the destination folder"
                                    ))
                                    .set_directory(start)
                                    .pick_folder()
                                {
                                    send(Msg::DmFolder(p));
                                }
                            });
                        }
                        ui.add(egui::Label::new(self.cfg.dm_dir.to_string_lossy()).truncate());
                    });
                    ui.end_row();
                    ui.label(tr!("Connessioni", "Connections"));
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("home_conns")
                            .selected_text(self.cfg.dm_connections.to_string())
                            .show_ui(ui, |ui| {
                                for c in CONNECTIONS {
                                    ui.selectable_value(
                                        &mut self.cfg.dm_connections,
                                        c,
                                        c.to_string(),
                                    );
                                }
                            });
                        ui.label(
                            RichText::new(tr!(
                                "Più connessioni scaricano più in fretta se il server lo permette.",
                                "More connections download faster if the server allows it."
                            ))
                            .small()
                            .color(ui.visuals().weak_text_color()),
                        );
                    });
                    ui.end_row();
                });
        });
    }

    fn torrent_folder_card(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        card(ui, |ui| {
            ui.label(RichText::new(tr!("Opzioni per i torrent", "Options for torrents")).strong());
            ui.horizontal(|ui| {
                ui.label(tr!("Salva in", "Save to"));
                if ui
                    .button(tr!("Cambia cartella…", "Change folder…"))
                    .clicked()
                {
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
        });
    }

    /// Filtri e lista unica di tutti i download.
    fn list_section(&mut self, ui: &mut egui::Ui) {
        let snap = self.dm.snapshot();
        let tors = self.hub.snapshot();
        let tor_class = |s: TState| match s {
            TState::Finished | TState::Seeding => Class::Done,
            TState::Error => Class::Failed,
            _ => Class::Active,
        };
        let dm_class = |s: State| match s {
            State::Done => Class::Done,
            State::Failed => Class::Failed,
            _ => Class::Active,
        };
        let mut all: Vec<Class> = self.jobs.iter().map(|j| super::job_class(j.state)).collect();
        all.extend(snap.items.iter().map(|i| dm_class(i.state)));
        all.extend(tors.iter().map(|t| tor_class(t.state)));
        let count = |c: Class| all.iter().filter(|x| **x == c).count();
        let (active, done, failed) = (count(Class::Active), count(Class::Done), count(Class::Failed));

        if all.is_empty() {
            empty_state(
                ui,
                tr!("Ancora nessun download", "No downloads yet"),
                tr!(
                    "Incolla un link qui sopra: Dropall capisce da solo se è un video, un file o un torrent.",
                    "Paste a link above: Dropall works out by itself whether it is a video, a file or a torrent."
                ),
            );
            return;
        }

        // 1. avanzamento: subito sotto il pulsante Scarica
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr!("In corso", "In progress")).size(17.0).strong());
            ui.label(RichText::new(format!("({active})")).size(15.0).color(super::ACCENT_TEXT));
        });
        ui.add_space(4.0);
        let shown = self.rows(ui, &snap, &tors, |c| c == Class::Active, false);
        if shown == 0 {
            ui.label(
                RichText::new(tr!(
                    "Nessun download in corso.",
                    "Nothing downloading right now."
                ))
                .color(ui.visuals().weak_text_color()),
            );
        }
        ui.add_space(14.0);

        // 2. storico: finiti bene o male, con i filtri
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tr!("Storico", "History")).size(17.0).strong());
            ui.add_space(6.0);
            for (f, label, n) in [
                (Filter::All, tr!("Tutti", "All"), done + failed),
                (Filter::Done, tr!("Completati", "Completed"), done),
                (Filter::Failed, tr!("Con errori", "With errors"), failed),
            ] {
                let selected = self.home.filter == f;
                let text = RichText::new(format!("  {label} ({n})  "))
                    .size(15.0)
                    .strong();
                if accent_toggle(ui, selected, text).clicked() {
                    self.home.filter = f;
                }
            }
            ui.label(
                RichText::new(tr!(
                    "Puoi trascinare qui anche i file .torrent.",
                    "You can also drag .torrent files here."
                ))
                .small()
                .color(ui.visuals().weak_text_color()),
            );
        });
        ui.add_space(8.0);
        let filter = self.home.filter;
        let shown = self.rows(ui, &snap, &tors, |c| filter.accepts(c), true);
        if shown == 0 {
            empty_state(
                ui,
                tr!("Storico vuoto", "Nothing in the history"),
                tr!(
                    "Qui compaiono i download finiti, o quelli andati male.",
                    "Finished downloads, and those that went wrong, show up here."
                ),
            );
        }
    }

    /// Disegna i download che passano il filtro: prima video e musica, poi torrent, poi file.
    /// Restituisce quanti ne ha mostrati.
    fn rows(
        &mut self,
        ui: &mut egui::Ui,
        snap: &crate::dm::Snapshot,
        tors: &[crate::torrent::TorrentItem],
        wanted: impl Fn(Class) -> bool,
        history: bool,
    ) -> usize {
        let mut shown = self
            .jobs
            .iter()
            .filter(|j| wanted(super::job_class(j.state)))
            .count();
        self.queue(ui, &wanted, history);
        if shown > 0 {
            ui.add_space(6.0);
        }
        for it in tors.iter().filter(|t| {
            wanted(match t.state {
                TState::Finished | TState::Seeding => Class::Done,
                TState::Error => Class::Failed,
                _ => Class::Active,
            })
        }) {
            shown += 1;
            self.torrent_row(ui, it);
            ui.add_space(6.0);
        }
        // file, coda per coda (le frecce su/giù spostano dentro la propria coda)
        for q in &snap.queues {
            let mine: Vec<&crate::dm::Item> =
                snap.items.iter().filter(|i| i.queue == q.id).collect();
            let last = mine.len().saturating_sub(1);
            for (n, it) in mine.iter().enumerate() {
                let c = match it.state {
                    State::Done => Class::Done,
                    State::Failed => Class::Failed,
                    _ => Class::Active,
                };
                if wanted(c) {
                    shown += 1;
                    self.dm_row(ui, it, n == 0, n == last);
                    ui.add_space(6.0);
                }
            }
        }
        shown
    }

    /// Pannello delle impostazioni: limite di velocità, torrent e code dei file.
    pub(super) fn settings_panel(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        let snap = self.dm.snapshot();
        if !snap.queues.iter().any(|q| q.id == self.dm_ui.queue) {
            self.dm_ui.queue = snap.queues.first().map(|q| q.id).unwrap_or(1);
        }
        ui.label(
            RichText::new(tr!("Impostazioni", "Settings"))
                .size(20.0)
                .strong(),
        );
        ui.add_space(6.0);
        self.dm_speed_card(ui);
        ui.add_space(8.0);
        self.torrent_options_card(ui);
        ui.add_space(8.0);
        ui.label(RichText::new(tr!("Code dei file", "File queues")).strong());
        ui.label(
            RichText::new(tr!(
                "I nuovi file vanno nella coda selezionata. Ogni coda scarica un file dopo l'altro e si può pianificare.",
                "New files go to the selected queue. Each queue downloads one file after another and can be scheduled."
            ))
            .small()
            .color(ui.visuals().weak_text_color()),
        );
        ui.add_space(4.0);
        self.dm_queues(ui, &snap.queues, &snap.items);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_are_told_apart() {
        for l in [
            "https://www.youtube.com/watch?v=abc",
            "https://youtu.be/abc",
            "https://music.youtube.com/playlist?list=x",
            "https://open.spotify.com/track/1",
            "  HTTPS://WWW.YOUTUBE.COM/watch?v=abc  ",
        ] {
            assert_eq!(detect(l), Kind::Media, "{l}");
        }
        for l in [
            "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567",
            "https://example.org/archivio.torrent",
            "/home/utente/film.torrent",
        ] {
            assert_eq!(detect(l), Kind::Torrent, "{l}");
        }
        for l in [
            "https://example.org/documenti.zip",
            "http://127.0.0.1:8800/video.mp4",
            // il nome del sito dentro il percorso non lo rende un video
            "https://example.org/youtube.com/file.zip",
            "https://notyoutube.com/x",
        ] {
            assert_eq!(detect(l), Kind::File, "{l}");
        }
    }

    #[test]
    fn filters_split_done_from_the_rest() {
        // nello storico non entra mai ciò che è ancora in corso
        assert!(!Filter::All.accepts(Class::Active));
        assert!(Filter::All.accepts(Class::Done) && Filter::All.accepts(Class::Failed));
        assert!(Filter::Done.accepts(Class::Done) && !Filter::Done.accepts(Class::Failed));
        assert!(Filter::Failed.accepts(Class::Failed) && !Filter::Failed.accepts(Class::Done));
    }
}
