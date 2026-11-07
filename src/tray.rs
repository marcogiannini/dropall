// SPDX-License-Identifier: GPL-3.0-or-later
//! Icona nell'area di notifica (tray): riduci a icona e i download continuano.
//! Linux: StatusNotifierItem (ksni). Windows/macOS: `tray-icon`.
use crate::tr;
use eframe::egui;
use std::sync::mpsc::{channel, Receiver, Sender};

pub enum TrayEvent {
    Show,
    Quit,
}

const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

fn icon_rgba(size: u32) -> Option<(Vec<u8>, u32, u32)> {
    let img = image::load_from_memory_with_format(ICON_PNG, image::ImageFormat::Png).ok()?;
    let img = img.resize(size, size, image::imageops::FilterType::Lanczos3);
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    Some((rgba.into_raw(), w, h))
}

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use ksni::blocking::{Handle, TrayMethods};

    struct SniTray {
        tx: Sender<TrayEvent>,
        ctx: egui::Context,
        busy: bool,
        icon: Vec<ksni::Icon>,
    }

    impl SniTray {
        fn send(&self, e: TrayEvent) {
            let _ = self.tx.send(e);
            self.ctx.request_repaint();
        }
    }

    impl ksni::Tray for SniTray {
        fn id(&self) -> String {
            "grabbit".into()
        }
        fn title(&self) -> String {
            "Grabbit".into()
        }
        fn icon_pixmap(&self) -> Vec<ksni::Icon> {
            self.icon.clone()
        }
        fn tool_tip(&self) -> ksni::ToolTip {
            ksni::ToolTip {
                title: "Grabbit".into(),
                description: if self.busy {
                    tr!("Download in corso", "Downloading").into()
                } else {
                    String::new()
                },
                ..Default::default()
            }
        }
        fn activate(&mut self, _x: i32, _y: i32) {
            self.send(TrayEvent::Show);
        }
        fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
            use ksni::menu::StandardItem;
            vec![
                StandardItem {
                    label: tr!("Mostra Grabbit", "Show Grabbit").into(),
                    activate: Box::new(|t: &mut SniTray| t.send(TrayEvent::Show)),
                    ..Default::default()
                }
                .into(),
                StandardItem {
                    label: tr!("Esci", "Quit").into(),
                    activate: Box::new(|t: &mut SniTray| t.send(TrayEvent::Quit)),
                    ..Default::default()
                }
                .into(),
            ]
        }
    }

    pub struct Tray {
        handle: Handle<SniTray>,
        rx: Receiver<TrayEvent>,
    }

    impl Tray {
        pub fn new(ctx: &egui::Context) -> Option<Tray> {
            let (tx, rx) = channel();
            let (rgba, w, h) = icon_rgba(64)?;
            // ARGB32, network byte order
            let mut data = Vec::with_capacity(rgba.len());
            for p in rgba.chunks_exact(4) {
                data.extend_from_slice(&[p[3], p[0], p[1], p[2]]);
            }
            let tray = SniTray {
                tx,
                ctx: ctx.clone(),
                busy: false,
                icon: vec![ksni::Icon {
                    width: w as i32,
                    height: h as i32,
                    data,
                }],
            };
            let handle = tray.spawn().ok()?;
            Some(Tray { handle, rx })
        }
        pub fn set_status(&mut self, busy: bool) {
            let _ = self.handle.update(|t| {
                t.busy = busy;
            });
        }
        pub fn events(&mut self) -> Vec<TrayEvent> {
            self.rx.try_iter().collect()
        }
    }

    impl Drop for Tray {
        fn drop(&mut self) {
            let _ = self.handle.shutdown();
        }
    }
}

#[cfg(any(windows, target_os = "macos"))]
mod imp {
    use super::*;
    use tray_icon::menu::{Menu, MenuEvent, MenuItem};
    use tray_icon::{
        Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    };

    pub struct Tray {
        icon: TrayIcon,
        rx: Receiver<TrayEvent>,
    }

    impl Tray {
        pub fn new(ctx: &egui::Context) -> Option<Tray> {
            let (tx, rx) = channel::<TrayEvent>();
            let (rgba, w, h) = icon_rgba(64)?;
            let icon = Icon::from_rgba(rgba, w, h).ok()?;
            let show = MenuItem::new(tr!("Mostra Grabbit", "Show Grabbit"), true, None);
            let quit = MenuItem::new(tr!("Esci", "Quit"), true, None);
            let menu = Menu::new();
            menu.append_items(&[&show, &quit]).ok()?;
            let (show_id, quit_id) = (show.id().clone(), quit.id().clone());

            let (t1, c1) = (tx.clone(), ctx.clone());
            MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
                if e.id == show_id {
                    let _ = t1.send(TrayEvent::Show);
                } else if e.id == quit_id {
                    let _ = t1.send(TrayEvent::Quit);
                }
                c1.request_repaint();
            }));
            let c2 = ctx.clone();
            TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
                | TrayIconEvent::DoubleClick { .. } = e
                {
                    let _ = tx.send(TrayEvent::Show);
                    c2.request_repaint();
                }
            }));
            let icon = TrayIconBuilder::new()
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .with_tooltip("Grabbit")
                .with_icon(icon)
                .build()
                .ok()?;
            Some(Tray { icon, rx })
        }
        pub fn set_status(&mut self, busy: bool) {
            let t = if busy {
                tr!("Grabbit — download in corso", "Grabbit — downloading")
            } else {
                "Grabbit"
            };
            let _ = self.icon.set_tooltip(Some(t));
        }
        pub fn events(&mut self) -> Vec<TrayEvent> {
            self.rx.try_iter().collect()
        }
    }
}

pub use imp::Tray;
