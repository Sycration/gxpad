//! The sgweb server: serves the web app that build.rs embedded, and nothing else. Every file is static;
//! the geometry runs in the browser, in the sggeo Wasm module.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

/// A file of the web app.
pub struct Asset {
    /// Path under the site root, without a leading `/`.
    pub path: &'static str,
    pub mime: &'static str,
    pub raw: &'static [u8],
    /// The gzip-compressed file, when that is smaller.
    pub gz: Option<&'static [u8]>,
    /// Quoted entity tag, a hash of `raw`.
    pub etag: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/assets.rs"));

/// The asset served at `path` (the path of a request, with or without its query).
pub fn asset(path: &str) -> Option<&'static Asset> {
    let path = path.split(['?', '#']).next().unwrap_or("").trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    ASSETS.iter().find(|a| a.path == path)
}

/// Pages may load only what this server serves; the module needs to be compiled, and formulas are shown as
/// images made in the page.
const CSP: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; worker-src 'self'; \
                   img-src 'self' blob: data:; style-src 'self'; connect-src 'self'; object-src 'none'; \
                   base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

/// Number of threads answering requests.
const THREADS: usize = 4;

/// A running server.
pub struct Running {
    server: Arc<Server>,
    threads: Vec<thread::JoinHandle<()>>,
}

impl Running {
    /// The address the server listens on.
    pub fn addr(&self) -> SocketAddr {
        self.server.server_addr().to_ip().expect("sgweb listens on TCP")
    }

    /// Blocks until the server stops (it runs until the process ends).
    pub fn wait(self) {
        for t in self.threads {
            let _ = t.join();
        }
    }

    /// Stops answering requests.
    pub fn stop(self) {
        // each unblock wakes one thread waiting in recv
        for _ in 0..THREADS {
            self.server.unblock();
        }
        for t in self.threads {
            let _ = t.join();
        }
    }
}

/// Starts serving on `addr` (port 0 picks a free port).
pub fn start(addr: SocketAddr) -> io::Result<Running> {
    let server = Arc::new(Server::http(addr).map_err(io::Error::other)?);
    let threads = (0..THREADS)
        .map(|_| {
            let s = server.clone();
            thread::spawn(move || {
                while let Ok(rq) = s.recv() {
                    let _ = respond(rq);
                }
            })
        })
        .collect();
    Ok(Running { server, threads })
}

fn header(k: &str, v: &str) -> Header {
    Header::from_bytes(k.as_bytes(), v.as_bytes()).expect("valid header")
}

fn request_header<'a>(rq: &'a Request, name: &'static str) -> Option<&'a str> {
    rq.headers().iter().find(|h| h.field.equiv(name)).map(|h| h.value.as_str())
}

fn respond(rq: Request) -> io::Result<()> {
    let head = *rq.method() == Method::Head;
    if !(head || *rq.method() == Method::Get) {
        let r = Response::from_string("method not allowed\n").with_status_code(405).with_header(header("Allow", "GET, HEAD"));
        return rq.respond(r);
    }
    let common = [
        header("Content-Security-Policy", CSP),
        header("X-Content-Type-Options", "nosniff"),
        header("Referrer-Policy", "no-referrer"),
        header("Cross-Origin-Opener-Policy", "same-origin"),
    ];
    let Some(a) = asset(rq.url()) else {
        let mut r = Response::from_string("not found\n").with_status_code(404).with_header(header("Content-Type", "text/plain; charset=utf-8"));
        for h in common {
            r.add_header(h);
        }
        return rq.respond(r);
    };
    // revalidated on every load, so a new binary takes effect at once; unchanged files cost a 304
    let cache = [header("ETag", a.etag), header("Cache-Control", "no-cache"), header("Vary", "Accept-Encoding")];
    if request_header(&rq, "If-None-Match").is_some_and(|v| v.split(',').any(|t| t.trim() == a.etag)) {
        let mut r = Response::empty(StatusCode(304));
        for h in cache.into_iter().chain(common) {
            r.add_header(h);
        }
        return rq.respond(r);
    }
    let gzip = a.gz.filter(|_| request_header(&rq, "Accept-Encoding").is_some_and(accepts_gzip));
    let body = gzip.unwrap_or(a.raw);
    // a length rather than chunks, so browsers can show the progress of the module download
    let mut r = Response::new(StatusCode(200), vec![header("Content-Type", a.mime)], if head { &[][..] } else { body }, Some(body.len()), None)
        .with_chunked_threshold(usize::MAX);
    if gzip.is_some() {
        r.add_header(header("Content-Encoding", "gzip"));
    }
    for h in cache.into_iter().chain(common) {
        r.add_header(h);
    }
    rq.respond(r)
}

/// True when an Accept-Encoding value allows gzip.
fn accepts_gzip(v: &str) -> bool {
    v.split(',').any(|part| {
        let mut it = part.split(';');
        let name = it.next().unwrap_or("").trim();
        let q0 = it.any(|p| p.trim().replace(' ', "").trim_start_matches("q=").parse::<f32>().is_ok_and(|q| q == 0.0) && p.contains("q="));
        (name.eq_ignore_ascii_case("gzip") || name == "*") && !q0
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gzip_negotiation() {
        assert!(accepts_gzip("gzip, deflate, br"));
        assert!(accepts_gzip("br;q=1.0, gzip;q=0.8"));
        assert!(accepts_gzip("*"));
        assert!(!accepts_gzip("gzip;q=0"));
        assert!(!accepts_gzip("deflate, br"));
        assert!(!accepts_gzip("identity"));
    }

    #[test]
    fn paths() {
        assert_eq!(asset("/").unwrap().path, "index.html");
        assert_eq!(asset("/?x=1").unwrap().path, "index.html");
        assert!(asset("/pkg/sggeo_bg.wasm").is_some());
        assert!(asset("/../Cargo.toml").is_none());
        assert!(asset("/nope").is_none());
    }
}
