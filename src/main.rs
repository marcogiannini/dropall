// SPDX-License-Identifier: GPL-3.0-or-later
// Su Windows (release) niente finestra console dietro all'interfaccia.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod config;
mod dm;
mod downloader;
mod i18n;
mod ipc;
mod net;
mod selfupdate;
mod spotify;
mod tools;
mod torrent;
mod tray;

const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

fn window_icon() -> Option<egui_icon::IconData> {
    let img = image::load_from_memory_with_format(ICON_PNG, image::ImageFormat::Png).ok()?;
    let rgba = img.to_rgba8();
    Some(egui_icon::IconData {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

mod egui_icon {
    pub use eframe::egui::IconData;
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let after_update = args.iter().any(|a| a == selfupdate::AFTER_UPDATE_ARG);
    let items = ipc::items_from_args(&args);
    // Già aperto? Passa link magnet / file .torrent (o "mostra la finestra") e termina.
    if !after_update && ipc::forward(&items) {
        return Ok(());
    }
    let repaint = std::sync::Arc::new(std::sync::Mutex::new(None));
    let incoming = if after_update {
        None
    } else {
        ipc::listen(repaint.clone())
    };

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title("Grabbit")
        .with_inner_size([780.0, 780.0])
        .with_min_inner_size([640.0, 560.0]);
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Grabbit",
        options,
        Box::new(move |cc| {
            *repaint.lock().unwrap() = Some(cc.egui_ctx.clone());
            Ok(Box::new(app::App::new(cc, items, incoming)))
        }),
    )
}
