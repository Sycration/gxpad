//! Editing operations: drawing samples, the numeric flag, deleting and flags.

use gxgeo::ops::{self, ConKind, ConicKind, DimType, Document, Target};

const WIN: ([f64; 2], [f64; 2], f64) = ([-10.0, -10.0], [10.0, 10.0], 0.02);

fn samples(sk: &Document, e: usize) -> Vec<Vec<[f64; 2]>> {
    ops::ent_samples(sk, e, WIN.0, WIN.1, WIN.2).unwrap()
}

#[test]
fn samples_draw_each_kind() {
    let mut sk = Document::new();
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
fn numeric_setup() -> (Document, usize, usize, usize) {
    let mut sk = Document::new();
    let a = ops::add_var(&mut sk, Some("a"), 1.5, true).unwrap();
    let o = ops::origin(&sk);
    let p = ops::add_point(&mut sk, 3.0, 0.5).unwrap();
    let l = ops::add_line(&mut sk, o, p, false).unwrap();
    assert!(ops::constrain(&mut sk, ConKind::Horizontal, l, 0).unwrap());
    let len = ops::add_dim(&mut sk, DimType::Length, l, 0, [0.0, 1.0]).unwrap();
    assert!(ops::set_dim(&mut sk, len, "2*a").unwrap());
    let r = ops::add_dim(&mut sk, DimType::Length, l, 0, [0.0, -1.0]).unwrap();
    assert!(!ops::dim_driving(&sk, r).unwrap());
    (sk, a, len, r)
}

fn plain(sk: &Document, t: Target) -> String {
    ops::derive(sk, t, 0, None).unwrap().plain
}

fn rhs(sk: &Document, t: Target) -> String {
    plain(sk, t).split(" = ").nth(1).unwrap().to_string()
}

#[test]
fn numeric_dimension_given_by_expression() {
    let (mut sk, _, len, _) = numeric_setup();
    let name = ops::dim_name(&sk, len).unwrap();
    let h = ops::add_defvar(&mut sk, Some("h"), &name).unwrap();
    assert!(rhs(&sk, Target::Var(h)).contains('a'));
    assert!(ops::set_dim_numeric(&mut sk, len, true).unwrap());
    assert_eq!(rhs(&sk, Target::Var(h)), "3");
    // the dimension keeps its expression and still follows a
    assert_eq!(ops::dim_expr(&sk, len).unwrap(), "2*a");
    assert!(ops::set_dim_numeric(&mut sk, len, false).unwrap());
    assert!(rhs(&sk, Target::Var(h)).contains('a'));
}

#[test]
fn numeric_reference_dimension() {
    let (mut sk, _, _, r) = numeric_setup();
    let name = ops::dim_name(&sk, r).unwrap();
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
    let mut sk = Document::new();
    let p = ops::add_point(&mut sk, 1.0, 1.0).unwrap();
    assert!(ops::delete_ent(&mut sk, p).unwrap());
    assert!(ops::delete_ent(&mut sk, p).is_err());
    // nothing given
    assert!(!sk.sketch_mut().delete(&[], &[], &[], &[]));
}

#[test]
fn toggle_infinite_is_one_change() {
    let mut sk = Document::new();
    let o = ops::origin(&sk);
    let a = ops::add_point(&mut sk, 1.0, 1.0).unwrap();
    let b = ops::add_point(&mut sk, 2.0, -1.0).unwrap();
    let l1 = ops::add_line(&mut sk, o, a, false).unwrap();
    let l2 = ops::add_line(&mut sk, o, b, true).unwrap();
    assert!(ops::toggle_infinite(&mut sk, &[l1, l2]).unwrap());
    assert!(ops::ent_infinite(&sk, l1).unwrap() && !ops::ent_infinite(&sk, l2).unwrap());
    assert!(ops::undo(&mut sk));
    assert!(!ops::ent_infinite(&sk, l1).unwrap() && ops::ent_infinite(&sk, l2).unwrap());
    assert!(ops::toggle_infinite(&mut sk, &[a]).is_err());
}

#[test]
fn argument_checks() {
    let mut sk = Document::new();
    let c = ops::add_point(&mut sk, 0.0, 0.0).unwrap();
    assert!(ops::circle_to(&mut sk, c, c).is_err());
    let pb = ops::add_conic(&mut sk, ConicKind::Parabola, c, 1.0, 1.0, 0.0).unwrap();
    assert!(ops::drag_conic_handle(&mut sk, pb, 1, 0.0, 3.0).is_err());
    assert!(ops::drag_conic_handle(&mut sk, pb, 0, 2.0, 0.0).is_ok());
}

#[test]
fn dimension_label_offset() {
    use std::f64::consts::FRAC_PI_4;
    let near = |p: [f64; 2], q: [f64; 2]| (p[0] - q[0]).hypot(p[1] - q[1]) < 1e-9;
    let mut sk = Document::new();
    let o = ops::origin(&sk);
    let a = ops::add_point(&mut sk, 1.0, 0.0).unwrap();
    let b = ops::add_point(&mut sk, 1.0, 1.0).unwrap();
    let l1 = ops::add_line(&mut sk, o, a, false).unwrap();
    let l2 = ops::add_line(&mut sk, o, b, false).unwrap();

    // length: offset and anchor, undoable
    let d = ops::add_dim(&mut sk, DimType::Length, l2, 0, [0.2, -0.1]).unwrap();
    assert!(near(ops::dim_offset(&sk, d).unwrap(), [0.2, -0.1]));
    assert!(near(ops::dim_anchor(&sk, d).unwrap(), [0.5, 0.5]));
    ops::set_dim_offset(&mut sk, d, [-0.4, 0.3]).unwrap();
    assert!(near(ops::dim_offset(&sk, d).unwrap(), [-0.4, 0.3]));
    assert!(ops::undo(&mut sk));
    assert!(near(ops::dim_offset(&sk, d).unwrap(), [0.2, -0.1]));
    assert!(ops::set_dim_offset(&mut sk, d, [f64::NAN, 0.0]).is_err());
    assert!(ops::set_dim_offset(&mut sk, 99, [0.0, 0.0]).is_err());
    assert!(ops::dim_offset(&sk, 99).is_err() && ops::dim_anchor(&sk, 99).is_err());

    // driving angle: the label's sector does not change the angle
    let t = ops::add_dim(&mut sk, DimType::Angle, l1, l2, [0.3, 0.1]).unwrap();
    assert!(ops::dim_driving(&sk, t).unwrap());
    ops::set_dim_offset(&mut sk, t, [-0.3, 0.1]).unwrap();
    assert!((ops::dim_value(&sk, t).unwrap().abs() - FRAC_PI_4).abs() < 1e-9);

    // reference angle: the label's sector selects the angle or its supplement
    assert!(ops::set_driving(&mut sk, t, false).unwrap());
    ops::set_dim_offset(&mut sk, t, [0.3, 0.1]).unwrap();
    assert!((ops::dim_value(&sk, t).unwrap().abs() - FRAC_PI_4).abs() < 1e-9);
    ops::set_dim_offset(&mut sk, t, [-0.3, 0.1]).unwrap();
    assert!((ops::dim_value(&sk, t).unwrap().abs() - 3.0 * FRAC_PI_4).abs() < 1e-9);
    assert!(ops::undo(&mut sk));
    assert!((ops::dim_value(&sk, t).unwrap().abs() - FRAC_PI_4).abs() < 1e-9);
}

#[test]
fn joined_undo_steps_undo_at_once() {
    let mut sk = Document::new();
    let first = ops::add_point(&mut sk, 0.0, 0.0).unwrap();
    let len = ops::undo_len(&sk);
    // a line between two new points, made with three calls
    let a = ops::add_point(&mut sk, 1.0, 0.0).unwrap();
    let b = ops::add_point(&mut sk, 2.0, 1.0).unwrap();
    let l = ops::add_line(&mut sk, a, b, false).unwrap();
    assert_eq!(ops::undo_len(&sk), len + 3);
    assert!(ops::join_undo(&mut sk, len));
    assert_eq!(ops::undo_len(&sk), len + 1);
    assert!(ops::undo(&mut sk));
    for e in [a, b, l] {
        assert!(ops::ent_kind(&sk, e).is_err(), "entity {e} is still there");
    }
    assert!(ops::ent_kind(&sk, first).is_ok());
    // redo brings back all three
    assert!(ops::redo(&mut sk));
    assert!(ops::ent_kind(&sk, l).is_ok());
    // nothing to join with fewer than two steps
    let len = ops::undo_len(&sk);
    assert!(!ops::join_undo(&mut sk, len));
    ops::add_point(&mut sk, 3.0, 3.0).unwrap();
    assert!(!ops::join_undo(&mut sk, len));
    assert!(!ops::join_undo(&mut sk, usize::MAX));
}

/// Width and height of a PNG image, from its header.
fn png_size(png: &[u8]) -> (u32, u32) {
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(&png[12..16], b"IHDR");
    let be = |i: usize| u32::from_be_bytes(png[i..i + 4].try_into().unwrap());
    (be(16), be(20))
}

#[test]
fn formula_typeset_as_png() {
    let mut sk = Document::new();
    let a = ops::add_point(&mut sk, 0.0, 0.0).unwrap();
    let b = ops::add_point(&mut sk, 3.0, 4.0).unwrap();
    let d = ops::add_dim(&mut sk, DimType::Distance, a, b, [0.5, 0.5]).unwrap();
    ops::set_driving(&mut sk, d, false).unwrap();
    let f = ops::derive(&sk, Target::Dim(d), 0, None).unwrap();
    let small = png_size(&ops::latex_png(&f.latex, 12.0, 96.0, [0, 0, 0]).unwrap());
    let large = png_size(&ops::latex_png(&f.latex, 24.0, 96.0, [0x20, 0x40, 0x80]).unwrap());
    assert!(small.0 > 20 && small.1 > 8, "{small:?}");
    // twice the size, about twice the pixels across
    let ratio = large.0 as f64 / small.0 as f64;
    assert!((1.8..2.2).contains(&ratio), "{small:?} {large:?}");
    // the same size at twice the resolution
    let sharp = png_size(&ops::latex_png(&f.latex, 12.0, 192.0, [0, 0, 0]).unwrap());
    assert!((sharp.0 as f64 / small.0 as f64 - 2.0).abs() < 0.2, "{small:?} {sharp:?}");

    assert!(ops::latex_png(r"\frac{1}{", 12.0, 96.0, [0, 0, 0]).is_err());
    assert!(ops::latex_png("x", 0.0, 96.0, [0, 0, 0]).is_err());
    assert!(ops::latex_png("x", 12.0, f64::NAN, [0, 0, 0]).is_err());
    assert!(ops::latex_png("x", 12.0, 3000.0, [0, 0, 0]).is_err());
}

/// A measured quantity that is 0 (the Simson feet are collinear, so the
/// third foot is 0 from the line through the other two) reads exactly 0,
/// not the rounding of the solved coordinates.
#[test]
fn zero_measurement_is_exactly_zero() {
    let text = include_str!("../test_examples/gen_constructions/simson_line.gxp");
    let mut sk = Document::from_gxp(text).unwrap();
    ops::solve(&mut sk);
    let offset = (0..ops::dim_count(&sk)).find(|&i| ops::dim_kind(&sk, i) == Ok(DimType::PointLine)).unwrap();
    assert!(!ops::dim_driving(&sk, offset).unwrap());
    assert_eq!(ops::dim_value(&sk, offset).unwrap(), 0.0);
    assert_eq!(ops::dim_set_value(&sk, offset).unwrap(), 0.0);
    let f = ops::derive(&sk, Target::Dim(offset), 0, None).unwrap();
    assert!(f.plain.ends_with(" = 0"), "{}", f.plain);
    assert_eq!((f.value, f.actual), (0.0, 0.0));
    // a value that is not 0 stays as it is
    let driving = (0..ops::dim_count(&sk)).find(|&i| ops::dim_driving(&sk, i).unwrap_or(false)).unwrap();
    assert!(ops::dim_value(&sk, driving).unwrap() > 1.0);
}
