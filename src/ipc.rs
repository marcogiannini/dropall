// SPDX-License-Identifier: GPL-3.0-or-later
//! Istanza unica: se Grabbit è già aperto, un secondo avvio (per esempio da un
//! link magnet nel browser o da un file .torrent) passa gli argomenti
//! all'istanza esistente tramite una connessione locale (127.0.0.1) protetta
//! da un token casuale, poi termina.
use crate::torrent::is_torrent_source;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn info_path() -> Option<PathBuf> {
    crate::tools::project_dirs().map(|d| d.config_dir().join("instance.json"))
}

/// Elementi da aprire presenti negli argomenti: magnet, .torrent (percorsi resi assoluti).
pub fn items_from_args(args: &[String]) -> Vec<String> {
    args.iter()
        .filter(|a| is_torrent_source(a))
        .map(|a| {
            let lower = a.to_ascii_lowercase();
            if lower.starts_with("magnet:")
                || lower.starts_with("http")
                || lower.starts_with("file://")
            {
                a.clone()
            } else {
                std::path::absolute(a)
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|_| a.clone())
            }
        })
        .collect()
}

fn random_token() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut s = String::new();
    for _ in 0..4 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        s.push_str(&format!("{:016x}", h.finish()));
    }
    s
}

/// Prova a passare `items` all'istanza già aperta. true = consegnato.
pub fn forward(items: &[String]) -> bool {
    let Some(path) = info_path() else {
        return false;
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return false;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    let (Some(port), Some(token)) = (v["port"].as_u64(), v["token"].as_str()) else {
        return false;
    };
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port as u16));
    let Ok(mut s) = TcpStream::connect_timeout(&addr, Duration::from_millis(800)) else {
        return false;
    };
    let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
    let mut msg = format!("{token}\n");
    for i in items {
        msg.push_str(&i.replace(['\n', '\r'], ""));
        msg.push('\n');
    }
    msg.push_str("\u{1}END\n");
    if s.write_all(msg.as_bytes()).is_err() {
        return false;
    }
    // aspetta la conferma: niente conferma = istanza non valida
    let mut reply = String::new();
    BufReader::new(s).read_line(&mut reply).is_ok() && reply.trim() == "OK"
}

/// Resta in ascolto per conto dell'istanza principale. Ogni messaggio ricevuto
/// (anche vuoto = "mostra la finestra") arriva sul canale; `repaint` sveglia l'interfaccia.
pub fn listen(repaint: Arc<Mutex<Option<eframe::egui::Context>>>) -> Option<Receiver<Vec<String>>> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).ok()?;
    let port = listener.local_addr().ok()?.port();
    let token = random_token();
    let path = info_path()?;
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    std::fs::write(
        &path,
        serde_json::json!({ "port": port, "token": token }).to_string(),
    )
    .ok()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        for conn in listener.incoming().flatten() {
            let _ = conn.set_read_timeout(Some(Duration::from_secs(3)));
            let mut reader = BufReader::new(conn.try_clone().unwrap());
            let mut first = String::new();
            if reader.read_line(&mut first).is_err() || first.trim() != token {
                continue;
            }
            let mut items = Vec::new();
            let mut ok = false;
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if line == "\u{1}END" {
                    ok = true;
                    break;
                }
                if items.len() < 200 && is_torrent_source(&line) {
                    items.push(line);
                }
            }
            if ok {
                let mut c = conn;
                let _ = c.write_all(b"OK\n");
                let _ = tx.send(items);
                if let Some(ctx) = repaint.lock().unwrap().as_ref() {
                    ctx.request_repaint();
                }
            }
        }
    });
    Some(rx)
}
