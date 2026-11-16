//! Runs the JavaScript tests in tests/js with Node.js against the app as the binary embeds it: the embedded
//! files are written to a directory, the tests next to them. Without Node.js the test passes with a notice,
//! unless SGWEB_REQUIRE_NODE is set.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn javascript() {
    let node = std::env::var("NODE").unwrap_or_else(|_| "node".into());
    if Command::new(&node).arg("--version").output().is_err() {
        assert!(std::env::var_os("SGWEB_REQUIRE_NODE").is_none(), "Node.js ({node}) is not installed");
        eprintln!("skipping the JavaScript tests: Node.js ({node}) is not installed");
        return;
    }
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sgweb-js");
    let _ = fs::remove_dir_all(&dir);
    for a in sgweb::ASSETS {
        let p = dir.join(a.path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, a.raw).unwrap();
    }
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/js");
    let tests = dir.join("tests");
    fs::create_dir_all(&tests).unwrap();
    let mut files = Vec::new();
    for e in fs::read_dir(&src).unwrap().flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        fs::copy(e.path(), tests.join(&name)).unwrap();
        if name.ends_with(".test.mjs") {
            files.push(tests.join(&name));
        }
    }
    files.sort();
    let out = Command::new(&node).arg("--test").arg("--test-reporter=spec").args(&files).current_dir(&dir).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "the JavaScript tests failed:\n{text}");
    eprintln!("{text}");
}
