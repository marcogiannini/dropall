// SPDX-License-Identifier: GPL-3.0-or-later
//! Avvisi di sistema quando un download finisce (toast di Windows, Centro notifiche di
//! macOS, notifiche del desktop su Linux).
use crate::tr;
use std::collections::HashMap;

/// A che punto è un download, dal punto di vista degli avvisi.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    /// ancora da fare o in corso
    Active,
    Done,
    Failed,
    /// concluso o in verifica senza nulla da dire (annullato dall'utente, controllo dei file)
    Quiet,
}

/// Cosa è appena finito.
#[derive(Default, Debug, PartialEq, Eq)]
pub struct Finished {
    pub done: Vec<String>,
    pub failed: Vec<String>,
}

/// Ricorda lo stato di ogni download e individua quelli appena conclusi.
#[derive(Default)]
pub struct Watcher {
    seen: HashMap<String, Phase>,
}

impl Watcher {
    /// `now`: (chiave stabile, stato, nome). Avvisa solo per chi era *visto in corso* e ora ha
    /// finito: i download già conclusi all'avvio, o quelli che si riaprono dopo un riavvio
    /// e vengono solo ricontrollati, non fanno rumore.
    pub fn update(&mut self, now: Vec<(String, Phase, String)>) -> Finished {
        let mut out = Finished::default();
        let mut next = HashMap::with_capacity(now.len());
        for (key, phase, name) in now {
            if self.seen.get(&key) == Some(&Phase::Active) {
                match phase {
                    Phase::Done => out.done.push(name),
                    Phase::Failed => out.failed.push(name),
                    _ => {}
                }
            }
            next.insert(key, phase);
        }
        self.seen = next;
        out
    }
}

/// Titolo e testo dell'avviso per un gruppo di download conclusi.
pub fn message(names: &[String], failed: bool) -> Option<(String, String)> {
    match (names.len(), failed) {
        (0, _) => None,
        (1, false) => Some((tr!("Download completato", "Download complete").into(), names[0].clone())),
        (1, true) => Some((tr!("Download non riuscito", "Download failed").into(), names[0].clone())),
        (n, false) => Some((
            format!("{n} {}", tr!("download completati", "downloads complete")),
            names.join(", "),
        )),
        (n, true) => Some((
            format!("{n} {}", tr!("download non riusciti", "downloads failed")),
            names.join(", "),
        )),
    }
}

/// Mostra l'avviso senza bloccare l'interfaccia. Se il sistema non lo permette si ignora.
pub fn show(title: String, body: String) {
    std::thread::spawn(move || {
        let _ = notify_rust::Notification::new()
            .appname("Dropall")
            .summary(&title)
            .body(&body)
            .show();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(k: &str, p: Phase) -> (String, Phase, String) {
        (k.into(), p, format!("file-{k}"))
    }

    #[test]
    fn only_downloads_seen_running_trigger_a_notice() {
        let mut w = Watcher::default();
        // all'avvio c'è già un download concluso e uno in corso
        assert_eq!(w.update(vec![item("a", Phase::Done), item("b", Phase::Active)]), Finished::default());
        // b finisce: un avviso; a resta zitto
        let f = w.update(vec![item("a", Phase::Done), item("b", Phase::Done)]);
        assert_eq!(f.done, vec!["file-b".to_string()]);
        assert!(f.failed.is_empty());
        // nessun avviso ripetuto
        assert_eq!(w.update(vec![item("a", Phase::Done), item("b", Phase::Done)]), Finished::default());
    }

    #[test]
    fn failures_are_reported_but_cancel_and_recheck_are_quiet() {
        let mut w = Watcher::default();
        w.update(vec![item("a", Phase::Active), item("b", Phase::Active), item("c", Phase::Quiet)]);
        let f = w.update(vec![item("a", Phase::Failed), item("b", Phase::Quiet), item("c", Phase::Done)]);
        assert_eq!(f.failed, vec!["file-a".to_string()]);
        assert!(f.done.is_empty());
    }

    #[test]
    fn several_at_once_make_one_notice() {
        let names = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let (t, b) = message(&names, false).unwrap();
        assert!(t.starts_with("3 "));
        assert_eq!(b, "a, b, c");
        assert!(message(&[], false).is_none());
        let (t1, b1) = message(&names[..1], true).unwrap();
        assert!(t1.contains("non riuscito") || t1.contains("failed"));
        assert_eq!(b1, "a");
    }
}
