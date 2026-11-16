//! The server over HTTP: what it serves, with which headers, and what it refuses.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};

struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Reply {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }
}

fn request(addr: SocketAddr, method: &str, path: &str, headers: &[(&str, &str)]) -> Reply {
    let mut s = TcpStream::connect(addr).unwrap();
    let mut rq = format!("{method} {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n");
    for (k, v) in headers {
        rq.push_str(&format!("{k}: {v}\r\n"));
    }
    rq.push_str("\r\n");
    s.write_all(rq.as_bytes()).unwrap();
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).unwrap();
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").expect("a complete reply");
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let mut lines = head.lines();
    let status = lines.next().unwrap().split(' ').nth(1).unwrap().parse().unwrap();
    let headers = lines.filter_map(|l| l.split_once(':')).map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).collect();
    Reply { status, headers, body: raw[split + 4..].to_vec() }
}

fn server() -> sgweb::Running {
    sgweb::start("127.0.0.1:0".parse().unwrap()).unwrap()
}

#[test]
fn serves_the_app_from_the_binary() {
    let s = server();
    let addr = s.addr();
    let r = request(addr, "GET", "/", &[]);
    assert_eq!(r.status, 200);
    assert_eq!(r.header("Content-Type"), Some("text/html; charset=utf-8"));
    let html = String::from_utf8(r.body.clone()).unwrap();
    assert!(html.contains(r#"src="js/main.js""#));
    // the page may load nothing from elsewhere
    let csp = r.header("Content-Security-Policy").unwrap();
    assert!(csp.contains("default-src 'self'") && csp.contains("'wasm-unsafe-eval'"), "{csp}");
    assert!(!html.contains("http://") && !html.contains("https://"), "no outside resources in the page");

    let r = request(addr, "GET", "/pkg/sggeo_bg.wasm", &[("Accept-Encoding", "gzip, br")]);
    assert_eq!(r.status, 200);
    assert_eq!(r.header("Content-Type"), Some("application/wasm"));
    assert_eq!(r.header("Content-Encoding"), Some("gzip"));
    assert_eq!(r.header("Vary"), Some("Accept-Encoding"));
    let wasm = sgweb::asset("/pkg/sggeo_bg.wasm").unwrap();
    assert!(r.body.len() < wasm.raw.len() / 2, "compressed: {} of {}", r.body.len(), wasm.raw.len());

    let r = request(addr, "GET", "/pkg/sggeo_bg.wasm", &[]);
    assert_eq!(r.header("Content-Encoding"), None);
    assert_eq!(&r.body[..4], b"\0asm");
    s.stop();
}

#[test]
fn revalidation_head_and_errors() {
    let s = server();
    let addr = s.addr();
    let r = request(addr, "GET", "/js/main.js", &[]);
    assert_eq!(r.status, 200);
    assert_eq!(r.header("Cache-Control"), Some("no-cache"));
    let etag = r.header("ETag").unwrap().to_string();
    let r = request(addr, "GET", "/js/main.js", &[("If-None-Match", &etag)]);
    assert_eq!(r.status, 304);
    assert!(r.body.is_empty());

    let r = request(addr, "HEAD", "/css/app.css", &[]);
    assert_eq!(r.status, 200);
    assert!(r.body.is_empty());
    assert!(r.header("Content-Length").unwrap().parse::<usize>().unwrap() > 1000);

    assert_eq!(request(addr, "GET", "/nothing-here", &[]).status, 404);
    assert_eq!(request(addr, "GET", "/../Cargo.toml", &[]).status, 404);
    assert_eq!(request(addr, "POST", "/", &[]).status, 405);
    s.stop();
}

#[test]
fn every_page_reference_is_served() {
    // the files the page, its modules and the worker load are all in the binary
    for a in sgweb::ASSETS.iter().filter(|a| a.path.ends_with(".js") || a.path.ends_with(".html")) {
        let text = std::str::from_utf8(a.raw).unwrap();
        let base = a.path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        for (i, _) in text.match_indices(['"', '\'']) {
            let rest = &text[i + 1..];
            let Some(end) = rest.find(['"', '\'']) else { continue };
            let r = &rest[..end];
            // a quoted object key, such as the import module names of wasm-bindgen, loads nothing
            if rest[end + 1..].trim_start().starts_with(':') {
                continue;
            }
            let local = r.ends_with(".js") || r.ends_with(".css") || r.ends_with(".svg") || r.ends_with(".webmanifest") || r.ends_with(".wasm");
            if !local || r.contains(' ') || r.contains("://") || r.contains('$') {
                continue;
            }
            // relative to the file, as module imports and URLs made from import.meta.url are
            let mut parts: Vec<&str> = if r.starts_with('/') { vec![] } else { base.split('/').filter(|s| !s.is_empty()).collect() };
            for p in r.split('/') {
                match p {
                    "." | "" => {}
                    ".." => {
                        parts.pop();
                    }
                    p => parts.push(p),
                }
            }
            let path = parts.join("/");
            assert!(sgweb::asset(&path).is_some(), "{} refers to {r} ({path}), which is not served", a.path);
        }
    }
}

#[test]
fn examples_are_listed() {
    let index = std::str::from_utf8(sgweb::asset("examples/index.json").unwrap().raw).unwrap();
    let n = sgweb::ASSETS.iter().filter(|a| a.path.starts_with("examples/") && a.path.ends_with(".sgp")).count();
    assert!(n >= 5);
    assert_eq!(index.matches("\"path\"").count(), n);
    assert!(sgweb::asset("info.json").is_some());
}

#[test]
fn the_service_worker_keeps_every_file() {
    let sw = std::str::from_utf8(sgweb::asset("sw.js").unwrap().raw).unwrap();
    assert!(!sw.contains("__SGWEB_"), "placeholders left in sw.js");
    let version = sw.split("const VERSION = \"").nth(1).unwrap().split('"').next().unwrap();
    assert!(version.starts_with(env!("CARGO_PKG_VERSION")), "{version}");
    let files = sw.split("const FILES = [").nth(1).unwrap().split(']').next().unwrap();
    let files: Vec<&str> = files.split(',').map(|f| f.trim().trim_matches('"')).collect();
    assert!(files.contains(&"./"));
    for a in sgweb::ASSETS {
        let kept = files.contains(&a.path);
        match a.path {
            "sw.js" | "index.html" => assert!(!kept, "{} is in the list", a.path),
            p => assert!(kept, "{p} is not kept for offline use"),
        }
    }
    for f in &files {
        // ./ is the address of the site, which a browser asks for as /
        let path = if *f == "./" { "/" } else { f };
        assert!(sgweb::asset(path).is_some(), "the service worker lists {f}, which is not served");
    }
}
