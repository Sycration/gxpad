//! Editing operations: drawing samples, the numeric flag, deleting and flags.

use gxgeo::formula::{Target, derive_formula};
use gxgeo::model::ConicKind;
use gxgeo::ops::{self, ConKind, DimType};
use gxgeo::sketch::Sketch;

const WIN: ([f64; 2], [f64; 2], f64) = ([-10.0, -10.0], [10.0, 10.0], 0.02);

fn samples(sk: &Sketch, e: usize) -> Vec<Vec<[f64; 2]>> {
    ops::ent_samples(sk, e, WIN.0, WIN.1, WIN.2).unwrap()
}

#[test]
fn samples_draw_each_kind() {
    let mut sk = Sketch::new();
    let a = ops::add_point(&mut sk, 1.0, 2.0).unwrap();
    let b = ops::add_point(&mut sk, 3.0, 2.0).unwrap();
    assert_eq!(samples(&sk, a), vec![vec![[1.0, 2.0]]]);

    // a segment is its endpoints, an infinite line crosses the window
    let l = ops::add_line(&mut sk, a, b, false).unwrap();
    assert_eq!(samples(&sk, l), vec![vec![[1.0, 2.0], [3.0, 2.0]]]);
    ops::set_infinite(&mut sk, l, true).unwrap();
    let inf = samples(&sk, l);
    assert!(inf[0][0][0] < -10.0 && inf[0][1][0] > 10.0);

    // an arc stays on its own quarter
    let c = ops::add_point(&mut sk, 5.0, 5.0).unwrap();
    let p = ops::add_point(&mut sk, 6.0, 5.0).unwrap();
    let arc = ops::arc_toward(&mut sk, c, p, [5.0, 6.0]).unwrap();
    let s = samples(&sk, arc);
    assert_eq!(s.len(), 1);
    assert!(s[0].iter().all(|q| q[0] >= 5.0 - 1e-9 && q[1] >= 5.0 - 1e-9), "{s:?}");

    // a hyperbola has two branches
    let h = ops::add_conic(&mut sk, ConicKind::Hyperbola, c, 1.0, 0.5, 0.0).unwrap();
    assert_eq!(samples(&sk, h).len(), 2);

    // a function curve stays inside the window, an implicit curve is traced
    let f = ops::add_curve(&mut sk, "y = x^2").unwrap();
    let s = samples(&sk, f);
    assert!(!s.is_empty() && s.iter().all(|pl| pl.len() > 1));
    assert!(s.iter().flatten().all(|q| q[0] >= -10.1 && q[0] <= 10.1));
    let g = ops::add_curve(&mut sk, "x^2 + y^2 = 4").unwrap();
    let s = samples(&sk, g);
    assert!(!s.is_empty());
    assert!(s.iter().flatten().all(|q| (q[0].hypot(q[1]) - 2.0).abs() < 0.05), "{s:?}");

    assert!(ops::ent_samples(&sk, a, [0.0, 0.0], [0.0, 1.0], 0.1).is_err());
    assert!(ops::ent_samples(&sk, a, [0.0, 0.0], [1.0, 1.0], 1e-9).is_err());
}

/// a = 1.5 (free); line OP horizontal with length L = 2a; R a reference
/// dimension of the same line.
fn numeric_setup() -> (Sketch, usize, usize, usize) {
    let mut sk = Sketch::new();
    let a = ops::add_var(&mut sk, Some("a"), 1.5, true).unwrap();
    let o = sk.doc.origin;
    let p = ops::add_point(&mut sk, 3.0, 0.5).unwrap();
    let l = ops::add_line(&mut sk, o, p, false).unwrap();
    assert!(ops::constrain(&mut sk, ConKind::Horizontal, l, 0).unwrap());
    let len = ops::add_dim(&mut sk, DimType::Length, l, 0, [0.0, 1.0]).unwrap();
    assert!(ops::set_dim(&mut sk, len, "2*a").unwrap());
    let r = ops::add_dim(&mut sk, DimType::Length, l, 0, [0.0, -1.0]).unwrap();
    assert!(!ops::dim_driving(&sk, r).unwrap());
    (sk, a, len, r)
}

fn plain(sk: &Sketch, t: Target) -> String {
    derive_formula(&sk.doc, t, 0).unwrap().plain
}

fn rhs(sk: &Sketch, t: Target) -> String {
    plain(sk, t).split(" = ").nth(1).unwrap().to_string()
}

#[test]
fn numeric_dimension_given_by_expression() {
    let (mut sk, _, len, _) = numeric_setup();
    let name = sk.doc.dims[len].as_ref().unwrap().name.clone();
    let h = ops::add_defvar(&mut sk, Some("h"), &name).unwrap();
    assert!(rhs(&sk, Target::Var(h)).contains('a'));
    assert!(ops::set_dim_numeric(&mut sk, len, true).unwrap());
    assert_eq!(rhs(&sk, Target::Var(h)), "3");
    // the dimension keeps its expression and still follows a
    assert_eq!(sk.doc.dims[len].as_ref().unwrap().expr, "2*a");
    assert!(ops::set_dim_numeric(&mut sk, len, false).unwrap());
    assert!(rhs(&sk, Target::Var(h)).contains('a'));
}

#[test]
fn numeric_reference_dimension() {
    let (mut sk, _, _, r) = numeric_setup();
    let name = sk.doc.dims[r].as_ref().unwrap().name.clone();
    let k = ops::add_defvar(&mut sk, Some("k"), &name).unwrap();
    assert!(rhs(&sk, Target::Var(k)).contains('a'));
    assert!(ops::set_dim_numeric(&mut sk, r, true).unwrap());
    assert_eq!(rhs(&sk, Target::Var(k)), "3");
    // its own formula is still derived, under its own name
    let own = plain(&sk, Target::Dim(r));
    assert!(own.starts_with(&format!("{name} = ")) && own.contains('a'), "{own}");
}

#[test]
fn numeric_variables_are_unchanged() {
    let (mut sk, a, _, _) = numeric_setup();
    let g = ops::add_defvar(&mut sk, Some("g"), "a + 1").unwrap();
    let h = ops::add_defvar(&mut sk, Some("h"), "2*g").unwrap();
    assert!(ops::set_var_numeric(&mut sk, g, true).unwrap());
    assert_eq!(rhs(&sk, Target::Var(h)), "5");
    assert!(ops::set_var_numeric(&mut sk, g, false).unwrap());
    assert_eq!(ops::var_expr(&sk, g).unwrap(), "a + 1");
    assert!(rhs(&sk, Target::Var(h)).contains('a'));

    // a free value variable
    assert!(ops::set_var_numeric(&mut sk, a, true).unwrap());
    assert!(!rhs(&sk, Target::Var(h)).contains('a'));
    assert!(ops::var_free(&sk, a).unwrap());
    assert!(ops::set_var(&mut sk, a, 2.0).unwrap());
    assert_eq!(rhs(&sk, Target::Var(h)), "6");

    // a solved variable stays solved
    let s = ops::add_var(&mut sk, Some("s"), 2.0, false).unwrap();
    assert!(ops::set_var_numeric(&mut sk, s, true).unwrap());
    assert!(ops::set_var_numeric(&mut sk, s, false).unwrap());
    assert!(!ops::var_free(&sk, s).unwrap());
}

#[test]
fn delete_reports_rejection() {
    let mut sk = Sketch::new();
    let p = ops::add_point(&mut sk, 1.0, 1.0).unwrap();
    assert!(ops::delete_ent(&mut sk, p).unwrap());
    assert!(ops::delete_ent(&mut sk, p).is_err());
    // nothing given
    assert!(!sk.delete(&[], &[], &[], &[]));
}

#[test]
fn toggle_infinite_is_one_change() {
    let mut sk = Sketch::new();
    let o = sk.doc.origin;
    let a = ops::add_point(&mut sk, 1.0, 1.0).unwrap();
    let b = ops::add_point(&mut sk, 2.0, -1.0).unwrap();
    let l1 = ops::add_line(&mut sk, o, a, false).unwrap();
    let l2 = ops::add_line(&mut sk, o, b, true).unwrap();
    assert!(ops::toggle_infinite(&mut sk, &[l1, l2]).unwrap());
    assert!(ops::ent_infinite(&sk, l1).unwrap() && !ops::ent_infinite(&sk, l2).unwrap());
    assert!(sk.undo());
    assert!(!ops::ent_infinite(&sk, l1).unwrap() && ops::ent_infinite(&sk, l2).unwrap());
    assert!(ops::toggle_infinite(&mut sk, &[a]).is_err());
}

#[test]
fn argument_checks() {
    let mut sk = Sketch::new();
    let c = ops::add_point(&mut sk, 0.0, 0.0).unwrap();
    assert!(ops::circle_to(&mut sk, c, c).is_err());
    let pb = ops::add_conic(&mut sk, ConicKind::Parabola, c, 1.0, 1.0, 0.0).unwrap();
    assert!(ops::drag_conic_handle(&mut sk, pb, 1, 0.0, 3.0).is_err());
    assert!(ops::drag_conic_handle(&mut sk, pb, 0, 2.0, 0.0).is_ok());
}
