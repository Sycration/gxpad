//! gxpad uses the geometry library only through `gxgeo::ops`, the interface
//! the C and JS bindings wrap one to one (gxgeo's `parity` test checks
//! that), so anything the app does can also be done from C and JS.

use std::path::Path;

/// The library internals that `ops::Document` hands to Rust code; C and JS
/// have no equivalent.
const INTERNALS: [&str; 3] = ["sketch", "sketch_mut", "from_sketch"];

fn sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            sources(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn app_reaches_gxgeo_only_through_ops() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = vec![];
    sources(&root.join("src"), &mut files);
    sources(&root.join("tests"), &mut files);
    assert!(files.len() > 5, "found only {} source files", files.len());
    let mut bad = vec![];
    for f in &files {
        if f.ends_with("public_api.rs") {
            continue;
        }
        let text = std::fs::read_to_string(f).unwrap();
        for (n, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            let past_ops = code.match_indices("gxgeo::").any(|(i, _)| !code[i..].starts_with("gxgeo::ops"));
            let internals = INTERNALS.iter().any(|m| code.contains(&format!(".{m}(")) || code.contains(&format!("::{m}(")));
            if past_ops || internals {
                bad.push(format!("{}:{}: {}", f.strip_prefix(root).unwrap().display(), n + 1, line.trim()));
            }
        }
    }
    assert!(bad.is_empty(), "gxpad reaches past gxgeo::ops:\n{}", bad.join("\n"));
}
