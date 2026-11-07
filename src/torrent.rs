// SPDX-License-Identifier: GPL-3.0-or-later
//! BitTorrent support built on libtorrent (BSD-3-Clause, vendored in
//! `third_party/libtorrent`) through the small C shim in `native/gt.cpp`.
#![allow(dead_code)]

use std::ffi::{c_char, c_int, CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, bail, Result};

/// What a pasted line or an opened path refers to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// `magnet:?xt=urn:btih:...`
    Magnet(String),
    /// http(s) link to a `.torrent` file
    TorrentUrl(String),
    /// local `.torrent` file
    TorrentFile(String),
    /// anything else (regular file / page link)
    Other(String),
}

/// Classifies one input line: magnet link, `.torrent` URL, `.torrent` path
/// (also `file://`), or something else.
pub fn classify(line: &str) -> Source {
    let t = line.trim().trim_matches(|c| c == '"' || c == '\'');
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("magnet:") {
        return Source::Magnet(t.to_string());
    }
    if lower.starts_with("http://") || lower.starts_with("https://") {
        let path = lower.split(['?', '#']).next().unwrap_or("");
        if path.ends_with(".torrent") {
            return Source::TorrentUrl(t.to_string());
        }
        return Source::Other(t.to_string());
    }
    let local = t.strip_prefix("file://").unwrap_or(t);
    if local.to_ascii_lowercase().ends_with(".torrent") {
        return Source::TorrentFile(local.to_string());
    }
    Source::Other(t.to_string())
}

/// True when the line must be handled by the torrent engine.
pub fn is_torrent_source(line: &str) -> bool {
    !matches!(classify(line), Source::Other(_))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TState {
    Checking,
    Metadata,
    Downloading,
    Finished,
    Seeding,
    Paused,
    Error,
}

#[derive(Debug, Clone)]
pub struct TorrentItem {
    pub hash: String,
    pub name: String,
    pub save_path: String,
    pub error: String,
    pub state: TState,
    pub peers: i32,
    pub seeds: i32,
    pub has_metadata: bool,
    pub total: u64,
    pub done: u64,
    pub down_rate: u64,
    pub up_rate: u64,
    pub uploaded: u64,
}

impl TorrentItem {
    pub fn fraction(&self) -> f32 {
        match self.state {
            TState::Finished | TState::Seeding if self.total == 0 => 1.0,
            _ if self.total == 0 => 0.0,
            _ => (self.done as f64 / self.total as f64).clamp(0.0, 1.0) as f32,
        }
    }
    pub fn eta_secs(&self) -> Option<u64> {
        if self.state != TState::Downloading || self.down_rate == 0 {
            return None;
        }
        Some(self.total.saturating_sub(self.done) / self.down_rate)
    }
    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            TState::Checking | TState::Metadata | TState::Downloading | TState::Seeding
        )
    }
}

#[repr(C)]
pub struct GtInfo {
    hash: [c_char; 41],
    name: [c_char; 256],
    save_path: [c_char; 1024],
    error: [c_char; 256],
    state: i32,
    peers: i32,
    seeds: i32,
    has_metadata: i32,
    total_wanted: i64,
    total_done: i64,
    down_rate: i64,
    up_rate: i64,
    uploaded: i64,
}

#[allow(non_camel_case_types)]
mod sys {
    use super::*;
    #[cfg(feature = "torrent")]
    mod imp {
        use super::*;
        #[allow(non_camel_case_types)]
        pub enum gt_session {}

        extern "C" {
            pub fn gt_create(
                state_dir: *const c_char,
                port: c_int,
                test_mode: c_int,
            ) -> *mut gt_session;
            pub fn gt_destroy(s: *mut gt_session);
            pub fn gt_add_magnet(
                s: *mut gt_session,
                uri: *const c_char,
                dir: *const c_char,
                hash: *mut c_char,
                err: *mut c_char,
                err_len: c_int,
            ) -> c_int;
            pub fn gt_add_file(
                s: *mut gt_session,
                path: *const c_char,
                dir: *const c_char,
                hash: *mut c_char,
                err: *mut c_char,
                err_len: c_int,
            ) -> c_int;
            pub fn gt_remove(s: *mut gt_session, hash: *const c_char, delete_files: c_int);
            pub fn gt_pause(s: *mut gt_session, hash: *const c_char);
            pub fn gt_resume(s: *mut gt_session, hash: *const c_char);
            pub fn gt_set_limits(s: *mut gt_session, down_kbps: c_int, up_kbps: c_int);
            pub fn gt_set_seed_after(s: *mut gt_session, seed_after: c_int);
            pub fn gt_snapshot(s: *mut gt_session, out: *mut GtInfo, max: c_int) -> c_int;
            pub fn gt_save_all(s: *mut gt_session);
            pub fn gt_make_torrent(
                content: *const c_char,
                out: *const c_char,
                err: *mut c_char,
                err_len: c_int,
            ) -> c_int;
            pub fn gt_add_peer(
                s: *mut gt_session,
                hash: *const c_char,
                ip: *const c_char,
                port: c_int,
            );
            pub fn gt_version() -> *const c_char;
        }
    }

    // Build without the `torrent` feature: every session creation fails.
    #[cfg(not(feature = "torrent"))]
    mod imp {
        use super::*;
        pub enum gt_session {}
        pub unsafe fn gt_create(_: *const c_char, _: c_int, _: c_int) -> *mut gt_session {
            std::ptr::null_mut()
        }
        pub unsafe fn gt_destroy(_: *mut gt_session) {}
        pub unsafe fn gt_add_magnet(
            _: *mut gt_session,
            _: *const c_char,
            _: *const c_char,
            _: *mut c_char,
            _: *mut c_char,
            _: c_int,
        ) -> c_int {
            -1
        }
        pub unsafe fn gt_add_file(
            _: *mut gt_session,
            _: *const c_char,
            _: *const c_char,
            _: *mut c_char,
            _: *mut c_char,
            _: c_int,
        ) -> c_int {
            -1
        }
        pub unsafe fn gt_remove(_: *mut gt_session, _: *const c_char, _: c_int) {}
        pub unsafe fn gt_pause(_: *mut gt_session, _: *const c_char) {}
        pub unsafe fn gt_resume(_: *mut gt_session, _: *const c_char) {}
        pub unsafe fn gt_set_limits(_: *mut gt_session, _: c_int, _: c_int) {}
        pub unsafe fn gt_set_seed_after(_: *mut gt_session, _: c_int) {}
        pub unsafe fn gt_snapshot(_: *mut gt_session, _: *mut GtInfo, _: c_int) -> c_int {
            0
        }
        pub unsafe fn gt_save_all(_: *mut gt_session) {}
        pub unsafe fn gt_make_torrent(
            _: *const c_char,
            _: *const c_char,
            _: *mut c_char,
            _: c_int,
        ) -> c_int {
            -1
        }
        pub unsafe fn gt_add_peer(
            _: *mut gt_session,
            _: *const c_char,
            _: *const c_char,
            _: c_int,
        ) {
        }
        pub unsafe fn gt_version() -> *const c_char {
            c"not built".as_ptr()
        }
    }
    pub use imp::*;
}
use sys::*;

fn cstr(s: &str) -> Result<CString> {
    CString::new(s).map_err(|_| anyhow!("invalid text"))
}

fn from_buf(b: &[c_char]) -> String {
    // SAFETY: the shim always NUL-terminates its buffers.
    unsafe { CStr::from_ptr(b.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

/// libtorrent version string, e.g. "2.0.15.0".
pub fn version() -> String {
    // SAFETY: returns a static NUL-terminated string.
    unsafe { CStr::from_ptr(gt_version()) }
        .to_string_lossy()
        .into_owned()
}

/// Creates a `.torrent` from a file (used by tests).
pub fn make_torrent(content: &Path, out: &Path) -> Result<()> {
    let c = cstr(&content.to_string_lossy())?;
    let o = cstr(&out.to_string_lossy())?;
    let mut err = [0 as c_char; 256];
    // SAFETY: valid C strings and buffer.
    let r = unsafe { gt_make_torrent(c.as_ptr(), o.as_ptr(), err.as_mut_ptr(), 256) };
    if r != 0 {
        bail!("{}", from_buf(&err));
    }
    Ok(())
}

/// A libtorrent session. Cheap to share: every call is serialized inside the shim.
pub struct Session {
    raw: Mutex<*mut gt_session>,
}

// SAFETY: the shim guards every entry point with its own mutex.
unsafe impl Send for Session {}
unsafe impl Sync for Session {}

impl Session {
    /// `state_dir` stores resume data. `port` 0 = default 6881.
    pub fn new(state_dir: &Path, port: u16, test_mode: bool) -> Result<Session> {
        let d = cstr(&state_dir.to_string_lossy())?;
        // SAFETY: valid C string.
        let raw = unsafe { gt_create(d.as_ptr(), port as c_int, test_mode as c_int) };
        if raw.is_null() {
            bail!("libtorrent session could not be created");
        }
        Ok(Session {
            raw: Mutex::new(raw),
        })
    }

    fn ptr(&self) -> *mut gt_session {
        *self.raw.lock().unwrap()
    }

    pub fn add_magnet(&self, uri: &str, dir: &Path) -> Result<String> {
        self.add(uri, dir, true)
    }

    pub fn add_file(&self, path: &Path, dir: &Path) -> Result<String> {
        self.add(&path.to_string_lossy(), dir, false)
    }

    fn add(&self, what: &str, dir: &Path, magnet: bool) -> Result<String> {
        let w = cstr(what)?;
        let d = cstr(&dir.to_string_lossy())?;
        let mut hash = [0 as c_char; 41];
        let mut err = [0 as c_char; 256];
        // SAFETY: valid C strings and buffers of the documented sizes.
        let r = unsafe {
            if magnet {
                gt_add_magnet(
                    self.ptr(),
                    w.as_ptr(),
                    d.as_ptr(),
                    hash.as_mut_ptr(),
                    err.as_mut_ptr(),
                    256,
                )
            } else {
                gt_add_file(
                    self.ptr(),
                    w.as_ptr(),
                    d.as_ptr(),
                    hash.as_mut_ptr(),
                    err.as_mut_ptr(),
                    256,
                )
            }
        };
        if r != 0 {
            bail!("{}", from_buf(&err));
        }
        Ok(from_buf(&hash))
    }

    pub fn remove(&self, hash: &str, delete_files: bool) {
        if let Ok(h) = cstr(hash) {
            // SAFETY: valid C string.
            unsafe { gt_remove(self.ptr(), h.as_ptr(), delete_files as c_int) }
        }
    }
    pub fn pause(&self, hash: &str) {
        if let Ok(h) = cstr(hash) {
            // SAFETY: valid C string.
            unsafe { gt_pause(self.ptr(), h.as_ptr()) }
        }
    }
    pub fn resume(&self, hash: &str) {
        if let Ok(h) = cstr(hash) {
            // SAFETY: valid C string.
            unsafe { gt_resume(self.ptr(), h.as_ptr()) }
        }
    }
    pub fn add_peer(&self, hash: &str, ip: &str, port: u16) {
        if let (Ok(h), Ok(i)) = (cstr(hash), cstr(ip)) {
            // SAFETY: valid C strings.
            unsafe { gt_add_peer(self.ptr(), h.as_ptr(), i.as_ptr(), port as c_int) }
        }
    }
    pub fn set_limits(&self, down_kbps: u32, up_kbps: u32) {
        // SAFETY: plain values.
        unsafe { gt_set_limits(self.ptr(), down_kbps as c_int, up_kbps as c_int) }
    }
    pub fn set_seed_after(&self, on: bool) {
        // SAFETY: plain value.
        unsafe { gt_set_seed_after(self.ptr(), on as c_int) }
    }
    pub fn save_all(&self) {
        // SAFETY: valid session.
        unsafe { gt_save_all(self.ptr()) }
    }

    pub fn snapshot(&self) -> Vec<TorrentItem> {
        const MAX: usize = 512;
        let mut buf: Vec<GtInfo> = Vec::with_capacity(MAX);
        // SAFETY: the shim initialises the first `n` entries, n <= MAX.
        let n = unsafe { gt_snapshot(self.ptr(), buf.as_mut_ptr(), MAX as c_int) };
        let n = (n.max(0) as usize).min(MAX);
        unsafe { buf.set_len(n) };
        buf.iter()
            .map(|i| TorrentItem {
                hash: from_buf(&i.hash),
                name: from_buf(&i.name),
                save_path: from_buf(&i.save_path),
                error: from_buf(&i.error),
                state: match i.state {
                    0 => TState::Checking,
                    1 => TState::Metadata,
                    2 => TState::Downloading,
                    3 => TState::Finished,
                    4 => TState::Seeding,
                    5 => TState::Paused,
                    _ => TState::Error,
                },
                peers: i.peers,
                seeds: i.seeds,
                has_metadata: i.has_metadata != 0,
                total: i.total_wanted.max(0) as u64,
                done: i.total_done.max(0) as u64,
                down_rate: i.down_rate.max(0) as u64,
                up_rate: i.up_rate.max(0) as u64,
                uploaded: i.uploaded.max(0) as u64,
            })
            .collect()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let p = *self.raw.get_mut().unwrap();
        // SAFETY: created by gt_create, destroyed exactly once.
        unsafe { gt_destroy(p) }
    }
}

/// Shared handle used by the UI: creates the libtorrent session lazily (or at
/// start-up when torrents are waiting to be restored), turns pasted links,
/// `.torrent` URLs and files into torrents, and collects error notes.
pub struct Hub {
    state_dir: PathBuf,
    session: Mutex<Option<Arc<Session>>>,
    notes: Arc<Mutex<Vec<String>>>,
    repaint: Arc<dyn Fn() + Send + Sync>,
    seed_after: Mutex<bool>,
    limits: Mutex<(u32, u32)>,
}

impl Hub {
    pub fn new(state_dir: PathBuf, repaint: impl Fn() + Send + Sync + 'static) -> Arc<Hub> {
        let hub = Arc::new(Hub {
            state_dir,
            session: Mutex::new(None),
            notes: Arc::new(Mutex::new(Vec::new())),
            repaint: Arc::new(repaint),
            seed_after: Mutex::new(false),
            limits: Mutex::new((0, 0)),
        });
        let has_saved = std::fs::read_dir(&hub.state_dir)
            .map(|d| {
                d.flatten()
                    .any(|e| e.path().extension().is_some_and(|x| x == "resume"))
            })
            .unwrap_or(false);
        if has_saved {
            let _ = hub.session();
        }
        hub
    }

    /// Whether this build includes libtorrent.
    pub fn available() -> bool {
        cfg!(feature = "torrent")
    }

    fn session(&self) -> Result<Arc<Session>> {
        let mut g = self.session.lock().unwrap();
        if let Some(s) = g.as_ref() {
            return Ok(s.clone());
        }
        if !Self::available() {
            bail!(crate::tr!(
                "questa versione di Grabbit è stata compilata senza supporto torrent",
                "this build of Grabbit was compiled without torrent support"
            ));
        }
        let s = Arc::new(Session::new(&self.state_dir, 0, false)?);
        s.set_seed_after(*self.seed_after.lock().unwrap());
        let (d, u) = *self.limits.lock().unwrap();
        s.set_limits(d, u);
        *g = Some(s.clone());
        Ok(s)
    }

    pub fn is_running(&self) -> bool {
        self.session.lock().unwrap().is_some()
    }

    pub fn set_seed_after(&self, on: bool) {
        *self.seed_after.lock().unwrap() = on;
        if let Some(s) = self.session.lock().unwrap().as_ref() {
            s.set_seed_after(on);
        }
    }

    pub fn set_limits(&self, down_kbps: u32, up_kbps: u32) {
        *self.limits.lock().unwrap() = (down_kbps, up_kbps);
        if let Some(s) = self.session.lock().unwrap().as_ref() {
            s.set_limits(down_kbps, up_kbps);
        }
    }

    pub fn snapshot(&self) -> Vec<TorrentItem> {
        match self.session.lock().unwrap().clone() {
            Some(s) => s.snapshot(),
            None => Vec::new(),
        }
    }

    /// Any torrent still transferring data?
    pub fn any_active(&self) -> bool {
        self.snapshot().iter().any(|t| t.is_active())
    }

    pub fn pause(&self, hash: &str) {
        if let Some(s) = self.session.lock().unwrap().as_ref() {
            s.pause(hash);
        }
    }
    pub fn resume(&self, hash: &str) {
        if let Some(s) = self.session.lock().unwrap().as_ref() {
            s.resume(hash);
        }
    }
    pub fn remove(&self, hash: &str, delete_files: bool) {
        if let Some(s) = self.session.lock().unwrap().as_ref() {
            s.remove(hash, delete_files);
        }
    }

    /// Errors and notices produced by background work, oldest first.
    pub fn take_notes(&self) -> Vec<String> {
        std::mem::take(&mut *self.notes.lock().unwrap())
    }

    fn note(&self, msg: String) {
        self.notes.lock().unwrap().push(msg);
        (self.repaint)();
    }

    /// Adds magnet links, `.torrent` URLs and local `.torrent` files (one per
    /// line). Network work happens on a background thread.
    pub fn add_lines(self: &Arc<Self>, lines: Vec<String>, dir: PathBuf) {
        let me = self.clone();
        std::thread::spawn(move || {
            for line in lines {
                if let Err(e) = me.add_one(&line, &dir) {
                    me.note(format!("{}: {e}", short(&line)));
                }
            }
            (me.repaint)();
        });
    }

    fn add_one(&self, line: &str, dir: &Path) -> Result<()> {
        let src = classify(line);
        if let Source::Other(_) = src {
            bail!(crate::tr!(
                "non è un link magnet né un file .torrent",
                "not a magnet link or .torrent file"
            ));
        }
        let ses = self.session()?;
        std::fs::create_dir_all(dir)?;
        match src {
            Source::Magnet(uri) => {
                ses.add_magnet(&uri, dir)?;
            }
            Source::TorrentFile(path) => {
                ses.add_file(Path::new(&path), dir)?;
            }
            Source::TorrentUrl(url) => {
                let tmp = self.state_dir.join("incoming");
                std::fs::create_dir_all(&tmp)?;
                let file = tmp.join(format!("{:x}.torrent", fnv(&url)));
                crate::net::download(&url, &file, &mut |_, _| {})?;
                let r = ses.add_file(&file, dir);
                let _ = std::fs::remove_file(&file);
                r?;
            }
            Source::Other(_) => {}
        }
        Ok(())
    }

    /// Stores resume data and stops the session (call when quitting).
    pub fn shutdown(&self) {
        if let Some(s) = self.session.lock().unwrap().take() {
            s.save_all();
        }
    }
}

fn fnv(s: &str) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn short(s: &str) -> String {
    let t = s.trim();
    if t.chars().count() > 60 {
        format!("{}…", t.chars().take(60).collect::<String>())
    } else {
        t.to_string()
    }
}

#[cfg(test)]
mod tests;
