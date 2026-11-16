//! The .sgp document format: a construction written as text, one statement
//! per line, in an order it can be built in. `docs/sgp-format.md` describes
//! it in full; in short:
//!
//! ```text
//! sgp 2
//!
//! given
//!   b = 4.925938
//!   triangle ABC
//!     AB ⟂ CA
//!     d1: |AB| = b
//!
//! construct
//!   l1 = bisect(AB, AC)
//!   l2 = bisect(BA, BC)
//!   I = meet(l1, l2)
//!
//! find
//!   d3: dist(I, AB)
//!
//! where
//!   A (1.371874, -0.2742224)
//!   ...
//! ```
//!
//! Every statement stands for the entities and constraints it makes, so a
//! document reads back as the same entities and constraints (renumbered in
//! the order of the file). The `where` section holds the solved positions:
//! they only pick which of the solutions the constraints allow is meant.

mod read;
mod write;

use crate::model::Doc;

/// File extension of the format, without the dot.
pub const EXTENSION: &str = "sgp";

/// Version of the format written in the header line `sgp 2`.
pub const VERSION: u32 = 2;

/// A document that could not be read: the line (1-based, 0 for the whole
/// text) and what is wrong with it.
#[derive(Clone, Debug, PartialEq)]
pub struct Error {
    pub line: usize,
    pub msg: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line == 0 { write!(f, "{}", self.msg) } else { write!(f, "line {}: {}", self.line, self.msg) }
    }
}

impl std::error::Error for Error {}

/// Encodes a document as .sgp text.
pub fn to_string(doc: &Doc) -> String {
    write::write(doc)
}

/// Decodes a document from .sgp text. The document is not solved.
pub fn from_str(text: &str) -> Result<Doc, Error> {
    read::read(text)
}

pub use read::{Applied, apply, lookup};

/// Words that start a statement or join two things in one, which no point
/// or curve can be named.
const RESERVED: &[&str] = &[
    "sgp", "given", "construct", "find", "where", "origin", "point", "segment", "line", "triangle", "polygon", "horiz", "vert", "on", "tan", "conc", "coll",
    "perp", "par", "xaxis", "yaxis",
];

/// Names of the axes, which every document has.
const XAXIS: &str = "xaxis";
const YAXIS: &str = "yaxis";

/// True for a point name that can be run together with others, as in `AB`
/// for the line through A and B: an uppercase letter, then lowercase
/// letters, digits or `_`.
fn joinable(name: &str) -> bool {
    let mut cs = name.chars();
    matches!(cs.next(), Some(c) if c.is_uppercase()) && cs.all(|c| c.is_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Splits run-together point names (`AB`, `P1Q2`) at their uppercase letters.
fn split_points(s: &str) -> Option<Vec<&str>> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in s.char_indices() {
        if c.is_uppercase() {
            if let Some(st) = start {
                out.push(&s[st..i]);
            }
            start = Some(i);
        } else if !(start.is_some() && (c.is_lowercase() || c.is_ascii_digit() || c == '_')) {
            return None;
        }
    }
    out.push(&s[start?..]);
    Some(out)
}

/// `v` with `n` significant digits, without trailing zeros.
fn sig(v: f64, n: i32) -> String {
    if v == 0.0 || !v.is_finite() {
        return if v.is_finite() { "0".into() } else { format!("{v}") };
    }
    let dec = (n - 1 - v.abs().log10().floor() as i32).max(0) as usize;
    let s = format!("{v:.dec$}");
    let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.') } else { &s };
    if s == "-0" { "0".into() } else { s.into() }
}

/// `v` exactly: the shortest decimal that reads back as `v`, with an
/// exponent when it is very small or large.
fn exact(v: f64) -> String {
    if v == 0.0 {
        "0".into()
    } else if v.abs() < 1e-6 || v.abs() >= 1e16 {
        format!("{v:e}")
    } else {
        format!("{v}")
    }
}

/// Angle `rad` in degrees, as the shortest decimal that converts back to
/// exactly `rad`.
fn degrees(rad: f64) -> String {
    let deg = rad.to_degrees();
    for n in 1..=17 {
        let s = sig(deg, n);
        if s.parse::<f64>().is_ok_and(|v| v.to_radians() == rad) {
            return s;
        }
    }
    sig(deg, 17)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers() {
        assert_eq!(sig(1.3718735075214266, 7), "1.371874");
        assert_eq!(sig(-0.000001, 7), "-0.000001");
        assert_eq!(sig(-0.00000000001, 3), "-0.00000000001");
        assert_eq!(sig(1234567.89, 3), "1234568");
        assert_eq!(degrees(30f64.to_radians()), "30");
        assert_eq!(degrees(0.9272952180016124).parse::<f64>().unwrap().to_radians(), 0.9272952180016124);
        assert_eq!(exact(4.925937527486427), "4.925937527486427");
        assert_eq!(exact(9.755345e-26), "9.755345e-26");
        assert_eq!(exact(-2.5), "-2.5");
    }

    #[test]
    fn point_names() {
        assert_eq!(split_points("AB"), Some(vec!["A", "B"]));
        assert_eq!(split_points("P1Fa_2Q"), Some(vec!["P1", "Fa_2", "Q"]));
        assert_eq!(split_points("aB"), None);
        assert_eq!(split_points("A-B"), None);
        assert!(joinable("Fa") && !joinable("p") && !joinable("AB"));
    }
}
