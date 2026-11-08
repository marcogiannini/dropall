// SPDX-License-Identifier: GPL-3.0-or-later
// Incorpora l'icona (e i metadati di versione) nell'eseguibile Windows.
// Su Linux/macOS non fa nulla.
fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-env-changed=DROPALL_REPO");
    #[cfg(feature = "torrent")]
    torrent::build();
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "Dropall");
        res.set("FileDescription", "Dropall - scarica video e musica");
        if let Err(e) = res.compile() {
            eprintln!("avviso: impossibile incorporare l'icona: {e}");
        }
    }
}

/// Compila libtorrent (BSD-3-Clause, in `third_party/libtorrent`) insieme al
/// piccolo strato C `native/gt.cpp` e lo collega staticamente.
/// OpenSSL viene compilato da `openssl-src`; Boost (solo header) si prende da
/// `DROPALL_BOOST_ROOT` oppure dal sistema.
#[cfg(feature = "torrent")]
mod torrent {
    use std::path::{Path, PathBuf};

    fn find_lib(dir: &Path, names: &[&str]) -> Option<PathBuf> {
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = find_lib(&path, names) {
                    return Some(found);
                }
            } else if let Some(n) = path.file_name().and_then(|n| n.to_str()) {
                if names.contains(&n) {
                    return Some(path);
                }
            }
        }
        None
    }

    const BOOST_URL: &str = "https://archives.boost.io/release/1.86.0/source/boost_1_86_0.tar.gz";

    fn has_boost(dir: &Path) -> bool {
        dir.join("boost/version.hpp").exists()
    }

    /// Trova gli header di Boost. `Some(cartella)` = da passare a CMake come
    /// DROPALL_BOOST_ROOT; `None` = si usa il Boost di sistema.
    fn ensure_boost(out_dir: &Path) -> Option<PathBuf> {
        if let Ok(v) = std::env::var("DROPALL_BOOST_ROOT") {
            if !v.is_empty() {
                let p = PathBuf::from(&v);
                if has_boost(&p) {
                    return Some(p);
                }
                println!(
                    "cargo:warning=DROPALL_BOOST_ROOT={v} has no boost/version.hpp, ignoring it"
                );
            }
        }
        if [
            "/usr/include",
            "/usr/local/include",
            "/opt/homebrew/include",
        ]
        .iter()
        .any(|d| has_boost(Path::new(d)))
        {
            return None;
        }
        let dest = out_dir.join("boost-headers");
        if has_boost(&dest) {
            return Some(dest);
        }
        let url = std::env::var("DROPALL_BOOST_URL").unwrap_or_else(|_| BOOST_URL.to_string());
        println!("cargo:warning=Boost headers not found: downloading {url}");
        if let Err(e) = download_boost(&url, &dest) {
            panic!(
                "Boost headers not found and the download failed ({e}). Set DROPALL_BOOST_ROOT to a \
                 folder containing `boost/`, install libboost-dev / `brew install boost`, or build \
                 without torrent support: cargo build --no-default-features"
            );
        }
        Some(dest)
    }

    /// Scarica l'archivio di Boost ed estrae solo la cartella `boost/` (header).
    fn download_boost(url: &str, dest: &Path) -> Result<(), String> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(900)))
            .build()
            .into();
        let resp = agent.get(url).call().map_err(|e| e.to_string())?;
        let gz = flate2::read::GzDecoder::new(resp.into_body().into_reader());
        let mut ar = tar::Archive::new(gz);
        let _ = std::fs::remove_dir_all(dest);
        std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
        for entry in ar.entries().map_err(|e| e.to_string())? {
            let mut entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path().map_err(|e| e.to_string())?.into_owned();
            let mut comps = path.components();
            comps.next(); // boost_1_86_0/
            let rel: PathBuf = comps.collect();
            if !rel.starts_with("boost") || !entry.header().entry_type().is_file() {
                continue;
            }
            if rel
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                continue;
            }
            let target = dest.join(&rel);
            if let Some(d) = target.parent() {
                std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
            }
            entry.unpack(&target).map_err(|e| e.to_string())?;
        }
        if has_boost(dest) {
            Ok(())
        } else {
            Err("archive does not contain boost/version.hpp".into())
        }
    }

    pub fn build() {
        for p in [
            "native",
            "third_party/libtorrent/src",
            "third_party/libtorrent/include",
        ] {
            println!("cargo:rerun-if-changed={p}");
        }
        println!("cargo:rerun-if-env-changed=DROPALL_BOOST_ROOT");

        let target = std::env::var("TARGET").unwrap_or_default();

        // Boost: variabile d'ambiente, sistema, oppure scaricato qui (solo header)
        let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let boost_root = ensure_boost(&out_dir);

        // OpenSSL statico
        let artifacts = openssl_src::Build::new().build();
        let ssl_root = artifacts.lib_dir().parent().unwrap().to_path_buf();

        let mut cfg = cmake::Config::new("native");
        cfg.profile("Release")
            .define("OPENSSL_ROOT_DIR", &ssl_root)
            .define("OPENSSL_USE_STATIC_LIBS", "ON")
            .define("OPENSSL_INCLUDE_DIR", artifacts.include_dir())
            .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
            .build_target("dropall_gt");
        if let Some(root) = &boost_root {
            cfg.env(
                "DROPALL_BOOST_ROOT",
                root.to_string_lossy().replace('\\', "/"),
            );
        }
        let out = cfg.build();

        let build_dir = out.join("build");
        let gt = find_lib(&build_dir, &["libdropall_gt.a", "dropall_gt.lib"])
            .expect("dropall_gt non trovata");
        let lt = find_lib(
            &build_dir,
            &["libtorrent-rasterbar.a", "torrent-rasterbar.lib"],
        )
        .expect("libtorrent non trovata");
        for lib in [&gt, &lt] {
            println!(
                "cargo:rustc-link-search=native={}",
                lib.parent().unwrap().display()
            );
        }
        println!("cargo:rustc-link-lib=static=dropall_gt");
        println!("cargo:rustc-link-lib=static=torrent-rasterbar");
        println!(
            "cargo:rustc-link-search=native={}",
            artifacts.lib_dir().display()
        );
        // MSVC names the static OpenSSL libraries libssl.lib / libcrypto.lib
        let (ssl, crypto) = if target.contains("msvc") {
            ("libssl", "libcrypto")
        } else {
            ("ssl", "crypto")
        };
        println!("cargo:rustc-link-lib=static={ssl}");
        println!("cargo:rustc-link-lib=static={crypto}");

        if target.contains("windows") {
            for l in [
                "ws2_32", "mswsock", "iphlpapi", "crypt32", "advapi32", "user32", "bcrypt",
            ] {
                println!("cargo:rustc-link-lib={l}");
            }
        } else if target.contains("apple") {
            println!("cargo:rustc-link-lib=c++");
            println!("cargo:rustc-link-lib=framework=CoreFoundation");
            println!("cargo:rustc-link-lib=framework=SystemConfiguration");
        } else {
            println!("cargo:rustc-link-lib=stdc++");
            println!("cargo:rustc-link-lib=pthread");
            println!("cargo:rustc-link-lib=dl");
        }
    }
}
