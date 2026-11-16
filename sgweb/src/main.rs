//! sgweb: serves the sgweb web app on a local address.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::ExitCode;

const USAGE: &str = "\
Usage: sgweb [--host ADDR] [--port N] [--open]

Serves sgweb, a web app for geometry with constraints and closed-form formulas.
Everything it serves is inside this binary; the geometry runs in the browser.

Options:
  --host ADDR   address to listen on (default 127.0.0.1; 0.0.0.0 serves other devices on the network)
  --port N      port to listen on (default 8080; 0 picks a free one)
  --open        open the app in the default browser
  -h, --help    show this help
  -V, --version show the version
";

fn main() -> ExitCode {
    let mut host = IpAddr::V4(Ipv4Addr::LOCALHOST);
    let mut port: u16 = 8080;
    let mut open = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let (flag, inline) = match a.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f.to_string(), Some(v.to_string())),
            _ => (a.clone(), None),
        };
        let mut value = |name: &str| inline.clone().or_else(|| args.next()).ok_or_else(|| format!("{name} needs a value"));
        let r: Result<(), String> = match flag.as_str() {
            "--host" => value("--host").and_then(|v| v.parse().map(|h| host = h).map_err(|_| format!("not an IP address: {v}"))),
            "--port" => value("--port").and_then(|v| v.parse().map(|p| port = p).map_err(|_| format!("not a port: {v}"))),
            "--open" => {
                open = true;
                Ok(())
            }
            "-h" | "--help" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "-V" | "--version" => {
                println!("sgweb {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            _ => Err(format!("unknown argument {a}")),
        };
        if let Err(e) = r {
            eprintln!("sgweb: {e}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    }

    let running = match sgweb::start(SocketAddr::new(host, port)) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("sgweb: cannot listen on {}: {e}", SocketAddr::new(host, port));
            return ExitCode::FAILURE;
        }
    };
    let addr = running.addr();
    // an unspecified address is reached through the loopback address on this machine
    let shown = if addr.ip().is_unspecified() {
        SocketAddr::new(if addr.is_ipv4() { IpAddr::V4(Ipv4Addr::LOCALHOST) } else { "::1".parse().unwrap() }, addr.port())
    } else {
        addr
    };
    let url = format!("http://{shown}/");
    println!("sgweb {} is serving on {url}", env!("CARGO_PKG_VERSION"));
    if addr.ip().is_unspecified() {
        println!("(listening on all interfaces, port {}; other devices use this machine's address)", addr.port());
    }
    println!("Press Ctrl+C to stop.");
    if open {
        open_browser(&url);
    }
    running.wait();
    ExitCode::SUCCESS
}

fn open_browser(url: &str) {
    #[cfg(windows)]
    let r = std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(url).spawn();
    #[cfg(not(any(windows, target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(url).spawn();
    if let Err(e) = r {
        eprintln!("sgweb: could not open a browser ({e}); open {url} yourself");
    }
}
