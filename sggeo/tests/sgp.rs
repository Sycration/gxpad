//! The .sgp text format: documents read back as they were written, and
//! documents written by hand read as they say.

mod testkit;

use sggeo::sgp;
use sggeo::model::*;
use sggeo::sketch::Sketch;
use testkit::EXAMPLES_DIR;

fn open(text: &str) -> Sketch {
    let doc = sgp::from_str(text).unwrap_or_else(|e| panic!("{e}\n{text}"));
    let mut sk = Sketch::from_doc(doc);
    sk.solve_full();
    sk
}

fn dim(sk: &Sketch, name: &str) -> f64 {
    let i = sk.doc.dims.iter().position(|x| x.as_ref().is_some_and(|x| x.name == name)).unwrap();
    sk.doc.measure_value(i)
}

/// An example reads as the construction it was saved from, and from then
/// on opening and saving it changes nothing. (The first save can differ:
/// the test that wrote it made things in its own order, which the file
/// puts in an order they can be built in.)
#[test]
fn examples_save_as_they_read() {
    let Ok(groups) = std::fs::read_dir(EXAMPLES_DIR) else { return };
    for group in groups.flatten() {
        for f in std::fs::read_dir(group.path()).into_iter().flatten().flatten() {
            if f.path().extension().is_none_or(|e| e != "sgp") {
                continue;
            }
            let text = std::fs::read_to_string(f.path()).unwrap();
            let a = open(&text);
            let saved = sgp::to_string(&a.doc);
            let b = open(&saved);
            assert_eq!(sgp::to_string(&b.doc), saved, "{}", f.path().display());
            for x in a.doc.dims.iter().flatten() {
                assert!((dim(&a, &x.name) - dim(&b, &x.name)).abs() < 1e-9, "{}: {}", f.path().display(), x.name);
            }
        }
    }
}

/// A construction without positions: the points start where their
/// constructions put them, or spread out.
#[test]
fn handwritten_without_positions() {
    let sk = open(
        "sgp 2
        # the inradius of a 3-4-5 triangle
        given
          triangle ABC
            d1: |AB| = 4
            d2: |BC| = 3
            d3: |CA| = 5
        construct
          l1 = bisect(AB, AC)
          l2 = bisect(BA, BC)
          I = meet(l1, l2)
        find
          r: dist(I, AB)",
    );
    assert!(sk.errors.is_empty() && sk.conflicts.is_empty());
    assert!((dim(&sk, "r") - 1.0).abs() < 1e-9, "{}", dim(&sk, "r"));
}

/// Statements the examples do not have read and write back as they were.
#[test]
fn every_statement_round_trips() {
    let text = "sgp 2
origin Z
xaxis [aux, hidden]

given
  s = 2
  t ~ 1.5
  point A B
  segment AB [aux]
    d1: |AB| = 3
  point C
  segment CD [hidden]
    AB coll CD
    q: len(CD) = t
  k1 = circle(A)
    r1: r(k1) = s [numeric]
  k2 = circle(A)
    k1 conc k2
    r(k2) == |AB|
  point H [hidden]
    H == offset(C, A, B, 90°)
  line CH
  e1 = hyperbola(Z)
    a1: a(e1) = 1
    b1: b(e1) = 1

construct
  M = mid(AB)
  point G
    G == bisect(AB, CD, M)
  point P
    P == asymptote(e1, -1)
  F = focus(e1)
  point T on k2
    T == tangent(C, k2)
  point R
    R == reflect(T, AB)
  point K
    K == mid(R, T)

find
  u = s*t

where
  A (0, 0) [fixed]
  B (3, 0)
  C (1, 2)
  D (2.5, 2) [fixed y]
  k1 r 2
  k2 r 3
  H (1, 5)
  e1 a 1 b 1 angle 0° [fixed angle]
  M (1.5, 0)
  G (2.5, 1)
  P (1, -1)
  F (1.4142135623730951, 0)
  T (2, 2)
  R (2, -2)
  K (2, 0)
  d1 label (0, 1)
";
    let sk = open(text);
    let out = sgp::to_string(&sk.doc);
    for want in [
        "origin Z",
        "xaxis [aux, hidden]",
        "segment AB [aux]",
        "segment CD [hidden]",
        "AB coll CD",
        "q: |CD| = t",
        "r1: r(k1) = s [numeric]",
        "k1 conc k2",
        "r(k2) == |AB|",
        "H == offset(C, A, B, 90°)",
        "G == bisect(AB, CD, M)",
        "P == asymptote(e1, -1)",
        "F = focus(e1)",
        "T = tangent(C, k2)",
        "R = reflect(T, AB)",
        "K = mid(R, T)",
        "t ~ ",
        "u = s*t",
        "A (0, 0) [fixed]",
        "[fixed y]",
        "[fixed angle]",
        "d1 label (0, 1)",
    ] {
        assert!(out.contains(want), "no {want:?} in\n{out}");
    }
    let again = sgp::to_string(&open(&out).doc);
    assert_eq!(again, out);
    let d = &sk.doc;
    let named = |n: &str| (0..d.ents.len()).find(|&e| d.ent(e).is_some_and(|x| x.name == n)).unwrap();
    let h = d.ent(named("H")).unwrap();
    assert!(h.hidden && !h.construction);
    assert!(d.ent(d.xaxis).unwrap().hidden);
    assert_eq!(d.ent(d.origin).unwrap().name, "Z");
}

#[test]
fn errors_name_their_line() {
    let err = |t: &str| sgp::from_str(t).unwrap_err().to_string();
    assert!(err("{\"params\": []}").contains("old JSON format"));
    assert!(err("sgp 9\n").contains("version 9"));
    assert!(err("point A\n").contains("must start with `sgp 2`"));
    assert_eq!(err("sgp 2\npoint A\n\nsegment AQ\nQ on k\n"), "line 5: nothing is named k");
    assert_eq!(err("sgp 2\npoint A\npoint A\n"), "line 3: A is named twice");
    assert_eq!(err("sgp 2\npoint A\nA (1, 2) [aux]\n"), "line 3: unknown flags [\"aux\"] (here: [fixed] or [fixed x y])");
    assert!(err("sgp 2\npoint A\nB (1, 2)\n").contains("places B"));
    assert_eq!(err("sgp 2\npoint line\n"), "line 2: line is a keyword, which cannot name a point or curve");
    assert_eq!(err("sgp 2\nsegment AB\nl = bisect(AB, CD)\n"), "line 3: nothing is named C");
}

/// Names of variables and of points are apart: a point A and a variable A.
#[test]
fn points_and_variables_share_names() {
    let sk = open("sgp 2\nA = 2\nsegment AB\n  d1: |AB| = A\n");
    assert!((dim(&sk, "d1") - 2.0).abs() < 1e-12);
    let out = sgp::to_string(&sk.doc);
    assert!(out.contains("A = 2") && out.contains("segment AB"), "{out}");
}

/// The angle between two segments from a shared point reads as ∠ when it
/// is the angle between the rays, and as `angle` with a direction otherwise.
#[test]
fn angles_keep_their_side() {
    let sk = open("sgp 2\ntriangle ABC\nu: ∠BAC\nv: angle(AB, CA)\nwhere\nA (0, 0)\nB (1, 0)\nC (0, 1)\n");
    assert!((dim(&sk, "u") - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    let out = sgp::to_string(&sk.doc);
    assert!(out.contains("u: ∠BAC") && out.contains("v: angle(AB, CA)"), "{out}");
    // `angle(AB, CA)` with the lines as drawn is the angle between their
    // directions: 180° - ∠BAC
    let sk = open("sgp 2\ntriangle ABC\nv: angle(AB, CA)\nwhere\nA (0, 0)\nB (1, 0)\nC (1, 1)\n");
    assert!((dim(&sk, "v") - 135f64.to_radians()).abs() < 1e-12, "{}", dim(&sk, "v"));
    let sk2 = open(&sgp::to_string(&sk.doc));
    assert!((dim(&sk2, "v") - 135f64.to_radians()).abs() < 1e-12);
    assert!(matches!(sk2.doc.dims[0].as_ref().unwrap().kind, DimKind::Angle(..)));
}

/// The examples of docs/sgp-format.md read and solve.
#[test]
fn format_doc_examples() {
    let doc = include_str!("../../docs/sgp-format.md");
    let blocks: Vec<&str> = doc.split("```").skip(1).step_by(2).filter(|b| b.trim_start().starts_with("sgp 2")).collect();
    assert_eq!(blocks.len(), 2);
    for b in blocks {
        let sk = open(b);
        assert!(sk.errors.is_empty() && sk.conflicts.is_empty(), "{b}");
        assert_eq!(sgp::to_string(&open(&sgp::to_string(&sk.doc)).doc), sgp::to_string(&sk.doc));
    }
    let first = open(blocks_first(doc));
    assert_eq!(sgp::to_string(&first.doc), blocks_first(doc).trim_start());
}

fn blocks_first(doc: &str) -> &str {
    doc.split("```").nth(1).unwrap()
}

/// Statements applied one at a time, as sgcli does, build what a file does.
#[test]
fn statements_apply_to_a_document() {
    use sggeo::ops::{self, Document};
    let mut doc = Document::new();
    for s in ["triangle ABC", "AB ⟂ CA", "b = 3", "d1: |AB| = b", "d2: |CA| = 4", "l1 = bisect(AB, AC)", "l2 = bisect(BA, BC)", "I = meet(l1, l2)", "r: dist(I, AB)"] {
        assert!(ops::apply_sgp(&mut doc, s).unwrap(), "{s}");
    }
    let r = (0..ops::dim_count(&doc)).find(|&i| ops::dim_name(&doc, i).unwrap() == "r").unwrap();
    assert!((ops::dim_value(&doc, r).unwrap() - 1.0).abs() < 1e-9);
    // what the others already say is rejected, and changes nothing
    let n = ops::con_count(&doc);
    assert!(!ops::apply_sgp(&mut doc, "BA ⟂ AC").unwrap());
    assert_eq!(ops::con_count(&doc), n);
    // nothing new can name what is not there
    assert!(ops::apply_sgp(&mut doc, "d3: |BC| = z").unwrap_err().contains("unknown"));
    // a position of a point that is there drags it
    let i = ops::ent_by_name(&doc, "I").unwrap();
    let before = ops::point_pos(&doc, i).unwrap();
    assert!(ops::apply_sgp(&mut doc, "A (10, 10)").unwrap());
    assert_ne!(ops::point_pos(&doc, i).unwrap(), before);
    assert_eq!(ops::ent_by_name(&doc, "BA"), ops::ent_by_name(&doc, "AB"));
    assert!(ops::ent_by_name(&doc, "AI").is_err());
    // one undo takes a statement back
    let n = ops::ent_count(&doc);
    assert!(ops::apply_sgp(&mut doc, "point P Q").unwrap());
    assert_eq!(ops::ent_count(&doc), n + 2);
    ops::undo(&mut doc);
    assert_eq!(ops::ent_count(&doc), n);
}
