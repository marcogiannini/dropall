use super::*;
use std::time::{Duration, Instant};

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("grabbit-torrent-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn classifies_inputs() {
    assert!(matches!(
        classify("magnet:?xt=urn:btih:abc"),
        Source::Magnet(_)
    ));
    assert!(matches!(
        classify("  MAGNET:?xt=urn:btih:abc "),
        Source::Magnet(_)
    ));
    assert!(matches!(
        classify("https://x.org/a/b.torrent"),
        Source::TorrentUrl(_)
    ));
    assert!(matches!(
        classify("https://x.org/a/b.TORRENT?token=1"),
        Source::TorrentUrl(_)
    ));
    assert!(matches!(
        classify("https://x.org/video.mp4"),
        Source::Other(_)
    ));
    assert_eq!(
        classify("file:///home/u/a.torrent"),
        Source::TorrentFile("/home/u/a.torrent".into())
    );
    assert!(matches!(
        classify("\"C:\\dl\\a.torrent\""),
        Source::TorrentFile(_)
    ));
    assert!(is_torrent_source("magnet:?xt=urn:btih:x"));
    assert!(!is_torrent_source("https://x.org/f.zip"));
}

#[test]
fn invalid_inputs_are_errors() {
    let dir = tmp("invalid");
    let s = Session::new(&dir.join("state"), 46101, true).unwrap();
    assert!(s.add_magnet("magnet:?foo=bar", &dir).is_err());
    std::fs::write(dir.join("bad.torrent"), b"not a torrent").unwrap();
    assert!(s.add_file(&dir.join("bad.torrent"), &dir).is_err());
    assert!(s.add_file(&dir.join("missing.torrent"), &dir).is_err());
    assert!(!version().is_empty());
}

#[test]
fn magnet_is_listed_and_survives_restart() {
    let dir = tmp("magnet");
    let state = dir.join("state");
    let hash = "0123456789abcdef0123456789abcdef01234567";
    let uri = format!("magnet:?xt=urn:btih:{hash}&dn=Example");
    {
        let s = Session::new(&state, 46102, true).unwrap();
        let h = s.add_magnet(&uri, &dir).unwrap();
        assert_eq!(h, hash);
        let snap = s.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].hash, hash);
        assert_eq!(snap[0].name, "Example");
        s.pause(hash);
        s.save_all();
    }
    let s = Session::new(&state, 46102, true).unwrap();
    let snap = s.snapshot();
    assert_eq!(snap.len(), 1, "torrent restored");
    assert_eq!(snap[0].hash, hash);
    s.remove(hash, false);
    assert!(s.snapshot().is_empty());
    assert!(!state.join(format!("{hash}.resume")).exists());
}

#[test]
fn two_sessions_exchange_a_file() {
    let dir = tmp("p2p");
    let seed_dir = dir.join("seed");
    let leech_dir = dir.join("leech");
    std::fs::create_dir_all(&seed_dir).unwrap();
    std::fs::create_dir_all(&leech_dir).unwrap();
    let content: Vec<u8> = (0..3_000_000u32)
        .map(|i| (i.wrapping_mul(2654435761) >> 13) as u8)
        .collect();
    let file = seed_dir.join("payload.bin");
    std::fs::write(&file, &content).unwrap();
    let tfile = dir.join("payload.torrent");
    make_torrent(&file, &tfile).unwrap();

    let seed = Session::new(&dir.join("s1"), 46103, true).unwrap();
    let leech = Session::new(&dir.join("s2"), 46104, true).unwrap();
    seed.set_seed_after(true);
    let h = seed.add_file(&tfile, &seed_dir).unwrap();
    let h2 = leech.add_file(&tfile, &leech_dir).unwrap();
    assert_eq!(h, h2);

    // wait for the seed to verify the existing data
    let t0 = Instant::now();
    loop {
        let s = seed.snapshot();
        if s.first()
            .map(|i| i.state == TState::Seeding)
            .unwrap_or(false)
        {
            break;
        }
        assert!(
            t0.elapsed() < Duration::from_secs(20),
            "seed never ready: {:?}",
            s.first().map(|i| i.state)
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    leech.add_peer(&h, "127.0.0.1", 46103);
    let t0 = Instant::now();
    let mut max_fraction = 0.0f32;
    loop {
        let l = leech.snapshot();
        let it = &l[0];
        max_fraction = max_fraction.max(it.fraction());
        if it.state == TState::Finished || it.state == TState::Seeding {
            break;
        }
        assert!(
            t0.elapsed() < Duration::from_secs(60),
            "download too slow: {:?} {:.2}",
            it.state,
            it.fraction()
        );
        std::thread::sleep(Duration::from_millis(200));
    }
    let got = std::fs::read(leech_dir.join("payload.bin")).unwrap();
    assert_eq!(got, content, "downloaded bytes differ");
    // seed_after is off for the leech: the finished torrent gets paused, not removed
    let it = &leech.snapshot()[0];
    assert!((it.fraction() - 1.0).abs() < 1e-6);
    assert_eq!(it.total, content.len() as u64);
}

#[test]
fn hub_adds_torrent_from_url_file_and_magnet() {
    use std::io::{Read, Write};
    let dir = tmp("hub");
    let content: Vec<u8> = (0..400_000u32).map(|i| (i % 251) as u8).collect();
    let file = dir.join("data.bin");
    std::fs::write(&file, &content).unwrap();
    let tfile = dir.join("data.torrent");
    make_torrent(&file, &tfile).unwrap();
    let body = std::fs::read(&tfile).unwrap();

    // tiny HTTP server that serves the .torrent
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for s in listener.incoming().flatten() {
            let mut s = s;
            let mut buf = [0u8; 2048];
            let _ = s.read(&mut buf);
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/x-bittorrent\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = s.write_all(head.as_bytes());
            let _ = s.write_all(&body);
        }
    });

    let hub = Hub::new(dir.join("state"), || {});
    let save = dir.join("save");
    hub.add_lines(
        vec![
            format!("http://127.0.0.1:{port}/files/data.torrent?x=1"),
            "magnet:?xt=urn:btih:cccccccccccccccccccccccccccccccccccccccc&dn=M".to_string(),
            "https://example.org/not-a-torrent.zip".to_string(),
        ],
        save,
    );
    let t0 = Instant::now();
    loop {
        let snap = hub.snapshot();
        let notes_ok = !hub.take_notes().is_empty() || snap.len() >= 2;
        if snap.len() == 2 && notes_ok {
            break;
        }
        assert!(
            t0.elapsed() < Duration::from_secs(20),
            "got {} torrents",
            snap.len()
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    let names: Vec<String> = hub.snapshot().into_iter().map(|t| t.name).collect();
    assert!(names.iter().any(|n| n == "data.bin"), "{names:?}");
    assert!(names.iter().any(|n| n == "M"), "{names:?}");
    hub.shutdown();
}
