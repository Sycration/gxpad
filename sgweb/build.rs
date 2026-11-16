//! Collects the web app into the binary: the files of `web/`, the example constructions of `examples/`
//! and the sggeo Wasm module with its wasm-bindgen glue under `pkg/`.
//!
//! The sggeo module comes from the directory that `SGWEB_SGGEO_PKG` names (the output of
//! `wasm-bindgen --target web`, as `./build.sh web` writes it to build/web/wasm32/lib). Without it, this
//! script builds sggeo for wasm32-unknown-unknown in a target directory of its own and runs wasm-bindgen
//! (`WASM_BINDGEN`, or wasm-bindgen on PATH) on it.
//!
//! Every file is stored as is and, when that is smaller, gzip-compressed; `$OUT_DIR/assets.rs` lists them.
//! The service worker `web/sw.js` gets the list of the other files and a version, a hash of all of them, in
//! place of its placeholders `__SGWEB_FILES__` and `__SGWEB_VERSION__`.

use flate2::Compression;
use flate2::write::GzEncoder;
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let ws = manifest.parent().unwrap().to_path_buf();
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed=web");
    println!("cargo:rerun-if-changed=examples");
    println!("cargo:rerun-if-env-changed=SGWEB_SGGEO_PKG");
    println!("cargo:rerun-if-env-changed=WASM_BINDGEN");

    let pkg = match env::var_os("SGWEB_SGGEO_PKG") {
        Some(dir) => PathBuf::from(dir),
        None => build_sggeo(&ws),
    };

    let mut files: Vec<(String, PathBuf)> = Vec::new();
    walk(&manifest.join("web"), "", &mut files);
    for name in ["sggeo.js", "sggeo_bg.wasm"] {
        let p = pkg.join(name);
        if !p.is_file() {
            panic!("{} is missing: SGWEB_SGGEO_PKG must name the output of `wasm-bindgen --target web`", p.display());
        }
        println!("cargo:rerun-if-changed={}", p.display());
        files.push((format!("pkg/{name}"), p));
    }
    let mut examples: Vec<(String, PathBuf)> = Vec::new();
    walk(&manifest.join("examples"), "examples/", &mut examples);
    examples.retain(|(n, _)| n.ends_with(".sgp"));
    examples.sort();
    files.extend(examples.iter().cloned());

    let dir = out.join("assets");
    fs::create_dir_all(&dir).unwrap();
    let mut table = String::from("pub static ASSETS: &[Asset] = &[\n");
    let mut add = |path: &str, bytes: &[u8], k: usize| {
        let raw = dir.join(format!("{k}.raw"));
        fs::write(&raw, bytes).unwrap();
        let gz = gzip(bytes);
        let gz_ref = if gz.len() * 10 < bytes.len() * 9 {
            let f = dir.join(format!("{k}.gz"));
            fs::write(&f, &gz).unwrap();
            format!("Some(include_bytes!({:?}))", f.display().to_string())
        } else {
            "None".into()
        };
        let mut h = DefaultHasher::new();
        bytes.hash(&mut h);
        writeln!(
            table,
            "    Asset {{ path: {path:?}, mime: {:?}, raw: include_bytes!({:?}), gz: {gz_ref}, etag: \"\\\"{:016x}\\\"\" }},",
            mime(path),
            raw.display().to_string(),
            h.finish()
        )
        .unwrap();
    };
    let mut k = 0;
    let mut sw = None;
    // what the service worker keeps: every file but itself, the page under the address of the site
    let mut kept = vec!["./".to_string()];
    let mut version = DefaultHasher::new();
    for (path, src) in &files {
        if path == "sw.js" {
            sw = Some(fs::read_to_string(src).unwrap());
            continue;
        }
        let bytes = fs::read(src).unwrap();
        (path, &bytes).hash(&mut version);
        if path != "index.html" {
            kept.push(path.clone());
        }
        add(path, &bytes, k);
        k += 1;
    }
    // the list of examples the app offers, with a title from the first comment line of each
    let mut index = String::from("[");
    for (i, (path, src)) in examples.iter().enumerate() {
        let text = fs::read_to_string(src).unwrap();
        let title = text.lines().find_map(|l| l.trim().strip_prefix('#')).map(str::trim).unwrap_or(path);
        let sep = if i == 0 { "" } else { "," };
        write!(index, "{sep}\n  {{\"path\": {}, \"title\": {}}}", json(path), json(title)).unwrap();
    }
    index.push_str("\n]\n");
    add("examples/index.json", index.as_bytes(), k);
    k += 1;
    let info = format!("{{\"sgweb\": {}}}\n", json(env!("CARGO_PKG_VERSION")));
    add("info.json", info.as_bytes(), k);
    k += 1;
    for (path, bytes) in [("examples/index.json", index.as_bytes()), ("info.json", info.as_bytes())] {
        (path, bytes).hash(&mut version);
        kept.push(path.to_string());
    }
    let sw = sw.expect("web/sw.js is missing");
    let list = format!("[{}]", kept.iter().map(|p| json(p)).collect::<Vec<_>>().join(", "));
    let sw = sw.replace("__SGWEB_FILES__", &list).replace("__SGWEB_VERSION__", &format!("{}-{:016x}", env!("CARGO_PKG_VERSION"), version.finish()));
    add("sw.js", sw.as_bytes(), k);
    table.push_str("];\n");
    fs::write(out.join("assets.rs"), table).unwrap();
}

/// Builds sggeo for the web and runs wasm-bindgen on it; returns the directory of the module and its glue.
fn build_sggeo(ws: &Path) -> PathBuf {
    for p in ["sggeo/src", "sggeo/Cargo.toml", "vendor", "Cargo.lock"] {
        println!("cargo:rerun-if-changed={}", ws.join(p).display());
    }
    // one directory for every profile and target of sgweb, so the module is built once
    let target_dir = env::var_os("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|| ws.join("target")).join("sgweb-wasm");
    let mut cmd = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    // the variables cargo sets for build scripts (flags, wrappers, the target) would leak into the build of
    // the module, which has a target of its own
    for (k, _) in env::vars_os() {
        let k = k.to_string_lossy();
        if (k.starts_with("CARGO_") && k != "CARGO_HOME") || k.starts_with("RUSTC_") || k == "RUSTFLAGS" || k == "RUSTDOCFLAGS" {
            cmd.env_remove(k.as_ref());
        }
    }
    let status = cmd
        .args(["build", "--release", "--locked", "-p", "sggeo", "--lib", "--target", "wasm32-unknown-unknown", "--target-dir"])
        .arg(&target_dir)
        .arg("--manifest-path")
        .arg(ws.join("Cargo.toml"))
        .status()
        .expect("cargo did not run");
    assert!(status.success(), "the wasm32 build of sggeo failed");

    let pkg = target_dir.join("pkg");
    let bindgen = env::var_os("WASM_BINDGEN").unwrap_or_else(|| "wasm-bindgen".into());
    let status = Command::new(&bindgen)
        .args(["--target", "web", "--out-dir"])
        .arg(&pkg)
        .arg(target_dir.join("wasm32-unknown-unknown/release/sggeo.wasm"))
        .status()
        .unwrap_or_else(|e| {
            panic!(
                "{} did not run ({e}): install wasm-bindgen-cli at the wasm-bindgen version in Cargo.lock, or set SGWEB_SGGEO_PKG",
                Path::new(&bindgen).display()
            )
        });
    assert!(status.success(), "wasm-bindgen failed");
    pkg
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<(String, PathBuf)>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if p.is_dir() {
            walk(&p, &format!("{prefix}{name}/"), out);
        } else {
            println!("cargo:rerun-if-changed={}", p.display());
            out.push((format!("{prefix}{name}"), p));
        }
    }
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut e = GzEncoder::new(Vec::new(), Compression::best());
    e.write_all(bytes).unwrap();
    e.finish().unwrap()
}

fn mime(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "wasm" => "application/wasm",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "webmanifest" => "application/manifest+json",
        "sgp" | "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn json(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => write!(o, "\\u{:04x}", c as u32).unwrap(),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}
