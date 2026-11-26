// SPDX-License-Identifier: GPL-3.0-or-later
//! Avvertenze legali: finestra al primo avvio (con conferma) e consultabile in ogni momento.
use super::App;
use crate::tr;
use eframe::egui::{self, RichText};

/// Punti dell'avviso: (titolo, testo). Gli stessi concetti sono nel README.
fn points() -> [(&'static str, &'static str); 7] {
    [
        (
            tr!("Uno strumento neutro", "A neutral tool"),
            tr!(
                "Dropall è un programma gratuito e open source. Non ospita, non cerca, non indicizza e non distribuisce contenuti: scarica solo ciò che indichi tu.",
                "Dropall is free, open-source software. It does not host, search, index or distribute any content: it only downloads what you point it at."
            ),
        ),
        (
            tr!("La responsabilità è tua", "You are responsible"),
            tr!(
                "Sei l'unico responsabile di ciò che scarichi e condividi. Usa Dropall solo per contenuti che hai il diritto di scaricare: tuoi, di pubblico dominio, con una licenza che lo permette (per esempio Creative Commons) o per cui hai il permesso del titolare.",
                "You alone are responsible for what you download and share. Use Dropall only for content you have the right to download: your own, in the public domain, under a licence that allows it (for example Creative Commons), or with the rights holder's permission."
            ),
        ),
        (
            tr!("Diritto d'autore", "Copyright"),
            tr!(
                "Scaricare o condividere opere protette senza autorizzazione può essere illegale e portare a sanzioni civili e penali. Le regole cambiano da Paese a Paese: informati sulle leggi che valgono per te.",
                "Downloading or sharing protected works without permission may be illegal and lead to civil and criminal penalties. Rules differ from country to country: find out what applies to you."
            ),
        ),
        (
            tr!("Termini dei servizi", "Terms of service"),
            tr!(
                "YouTube, Spotify e altri siti possono vietare il download nei loro termini d'uso: rispettali. Dropall non aggira sistemi di protezione (DRM). Per Spotify legge solo i dati pubblici del link e cerca lo stesso brano altrove: questo non rende lecito scaricare opere protette.",
                "YouTube, Spotify and other sites may forbid downloading in their terms of use: respect them. Dropall does not bypass protection systems (DRM). For Spotify it only reads the public data of the link and looks for the same track elsewhere: this does not make downloading protected works lawful."
            ),
        ),
        (
            tr!("Torrent e privacy", "Torrents and privacy"),
            tr!(
                "Con i torrent scarichi anche condividendo con gli altri, e il tuo indirizzo IP è visibile agli altri partecipanti. Dropall non è un servizio di anonimato.",
                "With torrents you also share with others while you download, and your IP address is visible to other participants. Dropall is not an anonymity service."
            ),
        ),
        (
            tr!("Nessuna garanzia", "No warranty"),
            tr!(
                "Il programma è fornito «così com'è», senza garanzie di alcun tipo. Gli autori non rispondono di danni o conseguenze legali derivanti dal suo uso.",
                "The software is provided “as is”, without warranty of any kind. The authors are not liable for any damage or legal consequences arising from its use."
            ),
        ),
        (
            tr!("Nessun incoraggiamento", "No encouragement"),
            tr!(
                "Gli autori non approvano la violazione del diritto d'autore e non forniscono aiuto per scaricare contenuti da fonti illecite. Questo avviso non è una consulenza legale.",
                "The authors do not condone copyright infringement and give no help in downloading from unlawful sources. This notice is not legal advice."
            ),
        ),
    ]
}

impl App {
    /// Mostra la finestra: obbligatoria finché non è accettata, poi consultabile dal piè di pagina.
    pub(super) fn legal_window(&mut self, ctx: &egui::Context) {
        let must = !self.cfg.terms_accepted;
        if !must && !self.show_legal {
            return;
        }
        let mut open = true;
        let mut win = egui::Window::new(tr!("Avvertenze legali", "Legal notice"))
            .collapsible(false)
            .resizable(false)
            .default_width(560.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]);
        if !must {
            win = win.open(&mut open);
        }
        win.show(ctx, |ui| {
            ui.set_max_width(560.0);
            ui.label(
                RichText::new(tr!(
                    "Prima di usare Dropall, leggi con attenzione.",
                    "Please read this carefully before using Dropall."
                ))
                .strong(),
            );
            ui.add_space(4.0);
            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                for (title, body) in points() {
                    ui.add_space(4.0);
                    ui.label(RichText::new(title).strong());
                    ui.label(body);
                }
            });
            if must {
                ui.add_space(10.0);
                ui.checkbox(
                    &mut self.terms_check,
                    tr!(
                        "Confermo di usare Dropall solo per contenuti che ho il diritto di scaricare e condividere, e di rispettare le leggi e i termini dei servizi.",
                        "I confirm that I will use Dropall only for content I have the right to download and share, and that I will respect the law and the services' terms."
                    ),
                );
                ui.add_space(6.0);
                let ok = self.terms_check;
                let label = tr!("Accetto e continuo", "I agree and continue");
                let (fill, text) = if ok {
                    (super::ACCENT, super::ON_ACCENT)
                } else {
                    (super::DISABLED_FILL, super::DISABLED_TEXT)
                };
                let r = ui.add(
                    egui::Button::new(RichText::new(label).strong().color(text))
                        .fill(fill)
                        .min_size(egui::vec2(220.0, 38.0))
                        .sense(if ok {
                            egui::Sense::click()
                        } else {
                            egui::Sense::hover()
                        }),
                );
                r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ok, label));
                if r.clicked() {
                    self.cfg.terms_accepted = true;
                    self.cfg.save();
                    // i torrent arrivati prima dell'accettazione partono ora
                    let held = std::mem::take(&mut self.held_torrents);
                    if !held.is_empty() {
                        self.hub.add_lines(held, self.cfg.torrent_dir.clone());
                    }
                }
            }
        });
        if !must && !open {
            self.show_legal = false;
        }
    }
}
