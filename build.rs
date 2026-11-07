// SPDX-License-Identifier: GPL-3.0-or-later
// Incorpora l'icona (e i metadati di versione) nell'eseguibile Windows.
// Su Linux/macOS non fa nulla.
fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-env-changed=GRABBIT_REPO");
    #[cfg(feature = "torrent")]
    torrent::build();
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "Grabbit");
        res.set("FileDescription", "Grabbit - scarica video e musica");
        if let Err(e) = res.compile() {
            eprintln!("avviso: impossibile incorporare l'icona: {e}");
        }
    }
}

/// Compila libtorrent (BSD-3-Clause, in `third_party/libtorrent`) insieme al
/// piccolo strato C `native/gt.cpp` e lo collega staticamente.
/// OpenSSL viene compilato da `openssl-src`; Boost (solo header) si prende da
/// `GRABBIT_BOOST_ROOT` oppure dal sistema.
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

    pub fn build() {
        for p in [
            "native",
            "third_party/libtorrent/src",
            "third_party/libtorrent/include",
        ] {
            println!("cargo:rerun-if-changed={p}");
        }
        println!("cargo:rerun-if-env-changed=GRABBIT_BOOST_ROOT");

        let target = std::env::var("TARGET").unwrap_or_default();

        // OpenSSL statico
        let artifacts = openssl_src::Build::new().build();
        let ssl_root = artifacts.lib_dir().parent().unwrap().to_path_buf();

        let mut cfg = cmake::Config::new("native");
        cfg.profile("Release")
            .define("OPENSSL_ROOT_DIR", &ssl_root)
            .define("OPENSSL_USE_STATIC_LIBS", "ON")
            .define("OPENSSL_INCLUDE_DIR", artifacts.include_dir())
            .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
            .build_target("grabbit_gt");
        let out = cfg.build();

        let build_dir = out.join("build");
        let gt = find_lib(&build_dir, &["libgrabbit_gt.a", "grabbit_gt.lib"])
            .expect("grabbit_gt non trovata");
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
        println!("cargo:rustc-link-lib=static=grabbit_gt");
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
