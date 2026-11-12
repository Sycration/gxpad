//! The C and JS interfaces offer exactly what `gxgeo::ops` offers: every
//! public function of `ops` has a C wrapper and a JS wrapper, and the
//! wrappers reach the library only through `ops`.

use std::collections::BTreeSet;

const OPS: &str = include_str!("../src/ops.rs");
const FFI: &str = include_str!("../src/ffi.rs");
const JS: &str = include_str!("../src/js.rs");

/// Functions of `Document` for Rust code that also uses the library
/// internals; C and JS have no internals to hand over.
const RUST_ONLY: [&str; 3] = ["from_sketch", "sketch", "sketch_mut"];

/// Public functions of `ops`: the free functions (`ops::name`) and the
/// associated functions of `Document` (`Document::name`).
fn ops_api() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut in_document = false;
    for line in OPS.lines() {
        if line.starts_with("impl Document") {
            in_document = true;
        } else if line.starts_with('}') {
            in_document = false;
        }
        let name = |rest: &str| rest.split(|c: char| !(c.is_alphanumeric() || c == '_')).next().unwrap_or("").to_string();
        if let Some(rest) = line.strip_prefix("pub fn ") {
            out.insert(format!("ops::{}", name(rest)));
        } else if let Some(rest) = line.strip_prefix("    pub fn ").filter(|_| in_document) {
            let n = name(rest);
            if !RUST_ONLY.contains(&n.as_str()) {
                out.insert(format!("Document::{n}"));
            }
        }
    }
    out
}

/// True when `src` calls `path`: `path(` or `path::<`.
fn calls(src: &str, path: &str) -> bool {
    src.match_indices(path).any(|(i, _)| {
        let before = src[..i].chars().next_back();
        let after = &src[i + path.len()..];
        !before.is_some_and(|c| c.is_alphanumeric() || c == '_') && (after.starts_with('(') || after.starts_with("::<"))
    })
}

#[test]
fn every_ops_function_has_c_and_js_wrappers() {
    let api = ops_api();
    assert!(api.len() > 50, "found only {} ops functions; is the scan broken?", api.len());
    let missing = |src: &str| api.iter().filter(|p| !calls(src, p)).cloned().collect::<Vec<_>>();
    let (c, js) = (missing(FFI), missing(JS));
    assert!(c.is_empty() && js.is_empty(), "no C wrapper: {c:?}\nno JS wrapper: {js:?}");
}

#[test]
fn wrappers_use_only_ops() {
    for (file, src) in [("ffi.rs", FFI), ("js.rs", JS)] {
        for (n, line) in src.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            let bad = code.match_indices("crate::").any(|(i, _)| !code[i..].starts_with("crate::ops")) || RUST_ONLY.iter().any(|f| code.contains(&format!(".{f}(")) || code.contains(&format!("::{f}(")));
            assert!(!bad, "{file}:{}: reaches past ops: {}", n + 1, line.trim());
        }
    }
}
