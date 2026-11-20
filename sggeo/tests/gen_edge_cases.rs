//! Edge-case constructions: extreme scales, all quadrants, signs, mirrored
//! layouts, axes, arcs, exact irrational values, far value changes.

use sggeo::model::DimKind;
mod testkit;
use testkit::Kit;
use std::f64::consts::PI;

#[test]
fn tiny_scale_right_triangle() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(0.003, 0.0001);
    let c = k.point(0.003, 0.004);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let ca = k.line(c, a);
    k.perpendicular(ab, bc);
    k.drive(DimKind::Length(ab), "0.003");
    k.drive(DimKind::Length(bc), "0.004");
    assert!(k.dof() > 0);
    let h = k.measure(DimKind::Length(ca));
    k.check_value(h, 0.005);
}

#[test]
fn huge_scale_right_triangle() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(1000.0, 3.0);
    let c = k.point(1000.0, 2000.0);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let ca = k.line(c, a);
    k.horizontal(ab);
    k.vertical(bc);
    k.drive(DimKind::Length(ab), "1000");
    k.drive(DimKind::Length(bc), "2000");
    assert!(k.dof() > 0);
    let h = k.measure(DimKind::Length(ca));
    k.check_value(h, 1000.0 * 5.0f64.sqrt());
}

#[test]
fn negative_quadrant_coordinates() {
    let mut k = Kit::new();
    let a = k.fixed_point(-3.7, -2.1);
    let p = k.point(-5.0, -6.0);
    k.drive(DimKind::CoordX(p), "-5");
    k.drive(DimKind::CoordY(p), "-6");
    assert_eq!(k.dof(), 0);
    let d = k.measure(DimKind::Distance(a, p));
    k.check_value(d, (1.3f64).hypot(3.9));
    let hd = k.measure(DimKind::HDist(a, p));
    let vd = k.measure(DimKind::VDist(a, p));
    println!("hdist {} vdist {}", k.value(hd), k.value(vd));
    k.check(hd);
    k.check(vd);
}

#[test]
fn hdist_vdist_left_below() {
    let mut k = Kit::new();
    let a = k.fixed_point(2.0, 3.0);
    let b = k.point(-1.5, -0.5);
    k.drive(DimKind::HDist(a, b), "3.5");
    k.drive(DimKind::VDist(a, b), "3.5");
    assert_eq!(k.dof(), 0);
    let cx = k.measure(DimKind::CoordX(b));
    let cy = k.measure(DimKind::CoordY(b));
    k.check_value(cx, -1.5);
    k.check_value(cy, -0.5);
}

#[test]
fn negative_signed_coordinates_tiny() {
    let mut k = Kit::new();
    let p = k.point(-2.4, -0.002);
    let q = k.point(0.001, 0.5);
    k.drive(DimKind::CoordX(p), "-2.5");
    k.drive(DimKind::CoordY(p), "-0.001");
    let l = k.line(p, q);
    k.horizontal(l);
    assert!(k.dof() > 0);
    let cy = k.measure(DimKind::CoordY(q));
    k.check_value(cy, -0.001);
}

#[test]
fn clockwise_triangle_isosceles() {
    // clockwise layout: apex at top, base below
    let mut k = Kit::new();
    let a = k.point(0.0, 3.0);
    let b = k.point(2.0, 0.1);
    let c = k.point(-2.0, 0.0);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let ca = k.line(c, a);
    k.equal(ab, ca);
    k.drive(DimKind::Length(bc), "4");
    k.drive(DimKind::Length(ab), "2.5");
    assert!(k.dof() > 0);
    let h = k.measure(DimKind::PointLine(a, bc));
    k.check_value(h, (6.25f64 - 4.0).sqrt());
}

#[test]
fn arc_chord_from_central_angle() {
    let mut k = Kit::new();
    let o = k.origin();
    let a = k.point(2.0, 0.3);
    let b = k.point(1.0, 1.7);
    let arc = k.arc(o, a, b);
    let oa = k.line(o, a);
    let ob = k.line(o, b);
    k.drive(DimKind::Radius(arc), "2");
    k.drive(DimKind::Angle(oa, ob), "60");
    assert!(k.dof() > 0);
    let chord = k.measure(DimKind::Distance(a, b));
    k.check_value(chord, 2.0);
}

#[test]
fn point_on_arc_diameter_dim() {
    let mut k = Kit::new();
    let c = k.fixed_point(-1.0, -1.0);
    let a = k.point(1.0, -1.0);
    let b = k.point(-1.0, 1.0);
    let arc = k.arc(c, a, b);
    let p = k.point(0.5, 0.5);
    k.on(p, arc);
    k.drive(DimKind::Diameter(arc), "4");
    k.drive(DimKind::CoordX(p), "0.5");
    assert!(k.dof() > 0);
    let y = k.measure(DimKind::CoordY(p));
    // |p - c| = 2 with p.x - c.x = 1.5 -> dy = sqrt(4 - 2.25)
    k.check_value(y, -1.0 + (1.75f64).sqrt());
}

#[test]
fn diameter_sqrt2_radius() {
    let mut k = Kit::new();
    let o = k.origin();
    let c = k.circle(o, 1.4);
    k.drive(DimKind::Diameter(c), "2*sqrt(2)");
    assert_eq!(k.dof(), 0);
    let r = k.measure(DimKind::Radius(c));
    k.check_value(r, 2.0f64.sqrt());
}

#[test]
fn radius_pi() {
    let mut k = Kit::new();
    let o = k.origin();
    let c = k.circle(o, 3.0);
    let p = k.point(3.0, 0.1);
    k.on(p, c);
    k.drive(DimKind::Radius(c), "pi");
    k.drive(DimKind::CoordY(p), "1");
    assert_eq!(k.dof(), 0);
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, (PI * PI - 1.0).sqrt());
}

#[test]
fn fixed_nonround_points() {
    let mut k = Kit::new();
    let p = k.fixed_point(0.37, 3.14159);
    let q = k.fixed_point(1.41421, -2.71828);
    let l = k.line(p, q);
    let d = k.measure(DimKind::Length(l));
    k.check_value(d, (1.41421f64 - 0.37).hypot(-2.71828 - 3.14159));
}

#[test]
fn point_line_axis_pythagoras() {
    let mut k = Kit::new();
    let o = k.origin();
    let p = k.point(5.5, 2.4);
    let xa = k.xaxis();
    k.drive(DimKind::PointLine(p, xa), "2.5");
    k.drive(DimKind::Distance(o, p), "6.5");
    assert_eq!(k.dof(), 0);
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, 6.0);
}

#[test]
fn point_line_to_yaxis_negative_side() {
    let mut k = Kit::new();
    let o = k.origin();
    let p = k.point(-3.1, -4.2);
    let ya = k.yaxis();
    k.drive(DimKind::Distance(o, p), "5");
    k.drive(DimKind::PointLine(p, ya), "3");
    assert_eq!(k.dof(), 0);
    let y = k.measure(DimKind::CoordY(p));
    k.check_value(y, -4.0);
}

#[test]
fn far_value_change() {
    let mut k = Kit::new();
    let o = k.origin();
    let p = k.point(2.0, 0.1);
    let q = k.point(2.0, 1.9);
    let l = k.line(o, p);
    let m = k.line(p, q);
    k.horizontal(l);
    k.perpendicular(l, m);
    let dl = k.drive(DimKind::Length(l), "2");
    let dm = k.drive(DimKind::Length(m), "2");
    assert_eq!(k.dof(), 0);
    let d = k.measure(DimKind::Distance(o, q));
    k.check_value(d, 8.0f64.sqrt());
    k.set(dl, "50");
    k.check_value(d, (2500.0f64 + 4.0).sqrt());
    k.set(dm, "0.01");
    k.check_value(d, (2500.0f64 + 0.0001).sqrt());
}

#[test]
fn equal_chain_collinear() {
    let mut k = Kit::new();
    let o = k.origin();
    let ps: Vec<usize> = (1..=4).map(|i| k.point(i as f64 * 1.4, 0.05)).collect();
    let mut ls = vec![k.line(o, ps[0])];
    for i in 1..4 {
        ls.push(k.line(ps[i - 1], ps[i]));
    }
    for l in &ls {
        k.horizontal(*l);
    }
    for i in 1..4 {
        k.equal(ls[0], ls[i]);
    }
    k.drive(DimKind::Length(ls[0]), "1.5");
    assert_eq!(k.dof(), 0);
    let d = k.measure(DimKind::Distance(o, ps[3]));
    k.check_value(d, 6.0);
}

#[test]
fn coincident_chain() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(0.1, 0.0);
    let c = k.point(0.0, 0.1);
    let d = k.point(3.0, 0.2);
    k.coincident(a, b);
    k.coincident(b, c);
    let l = k.line(c, d);
    k.drive(DimKind::Length(l), "3");
    let ld = k.line(a, d);
    assert!(k.dof() > 0);
    let m = k.measure(DimKind::Length(ld));
    k.check_value(m, 3.0);
}

#[test]
fn ladder_between_axes() {
    let mut k = Kit::new();
    let o = k.origin();
    let p = k.point(3.0, 0.1);
    let q = k.point(0.1, 4.0);
    let xa = k.xaxis();
    let ya = k.yaxis();
    k.on(p, xa);
    k.on(q, ya);
    k.drive(DimKind::Distance(p, q), "5");
    let m = k.midpoint_of(p, q);
    assert!(k.dof() > 0);
    let d = k.measure(DimKind::Distance(o, m));
    k.check_value(d, 2.5);
}

#[test]
fn ellipse_point_on_locked_axes() {
    let mut k = Kit::new();
    let o = k.origin();
    let e = k.ellipse(o, 3.0, 2.0, 0.0);
    k.lock_conic_angle(e);
    k.drive(DimKind::ConicA(e), "3");
    k.drive(DimKind::ConicB(e), "2");
    let p = k.point(1.5, 1.6);
    k.on(p, e);
    k.drive(DimKind::CoordX(p), "1.5");
    assert_eq!(k.dof(), 0);
    let y = k.measure(DimKind::CoordY(p));
    k.check_value(y, 3.0f64.sqrt());
}

#[test]
fn line_through_origin_meets_circle() {
    let mut k = Kit::new();
    let o = k.origin();
    let c = k.circle(o, 1000.0);
    k.drive(DimKind::Radius(c), "1000");
    let p = k.point(-700.0, -700.0);
    k.on(p, c);
    let l = k.line(o, p);
    k.drive(DimKind::Angle(k.xaxis(), l), "45");
    assert_eq!(k.dof(), 0);
    let x = k.measure(DimKind::CoordX(p));
    let y = k.measure(DimKind::CoordY(p));
    k.check_value(x, -1000.0 / 2.0f64.sqrt());
    k.check_value(y, -1000.0 / 2.0f64.sqrt());
}

#[test]
fn equilateral_third() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(0.35, 0.0);
    let c = k.point(0.17, 0.3);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let ca = k.line(c, a);
    k.drive(DimKind::Length(ab), "1/3");
    k.equal(ab, bc);
    k.equal(bc, ca);
    assert!(k.dof() > 0);
    let h = k.measure(DimKind::PointLine(c, ab));
    k.check_value(h, 3.0f64.sqrt() / 6.0);
}

#[test]
fn square_diagonal_mirrored() {
    // clockwise square, free rotation
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(0.1, -1.0);
    let c = k.point(1.0, -1.1);
    let d = k.point(1.1, 0.0);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let cd = k.line(c, d);
    let da = k.line(d, a);
    k.perpendicular(ab, bc);
    k.parallel(ab, cd);
    k.parallel(bc, da);
    k.equal(ab, bc);
    k.drive(DimKind::Length(ab), "1");
    assert!(k.dof() > 0);
    let dg = k.measure(DimKind::Distance(a, c));
    k.check_value(dg, 2.0f64.sqrt());
}

#[test]
fn supplementary_angle() {
    let mut k = Kit::new();
    let o = k.origin();
    let p = k.point(2.0, 0.0);
    let q = k.point(-1.0, 1.8);
    let l1 = k.line(o, p);
    let l2 = k.line(o, q);
    k.horizontal(l1);
    k.drive(DimKind::Length(l1), "2");
    k.drive(DimKind::Length(l2), "2");
    let d = k.drive(DimKind::Angle(l1, l2), "120");
    assert_eq!(k.dof(), 0);
    let _ = d;
    let cy = k.measure(DimKind::CoordY(q));
    k.check_value(cy, 3.0f64.sqrt());
    let cx = k.measure(DimKind::CoordX(q));
    k.check_value(cx, -1.0);
}

#[test]
fn infinite_lines_intersection_far() {
    let mut k = Kit::new();
    let a = k.fixed_point(0.0, 0.0);
    let b = k.point(1.0, 0.5);
    let c = k.fixed_point(10.0, 0.0);
    let d = k.point(9.0, 1.0);
    let l1 = k.inf_line(a, b);
    let l2 = k.inf_line(c, d);
    k.drive(DimKind::Angle(k.xaxis(), l1), "150");
    k.drive(DimKind::Angle(k.xaxis(), l2), "135");
    let x = k.intersect(l1, l2, [6.3, 3.7]);
    // y = x tan30 ; y = 10 - x  -> x = 10/(1+tan30)
    let t = (30.0f64).to_radians().tan();
    let cx = k.measure(DimKind::CoordX(x));
    k.check_value(cx, 10.0 / (1.0 + t));
}

#[test]
fn reflection_distance() {
    let mut k = Kit::new();
    let a = k.fixed_point(-2.0, 1.0);
    let b = k.fixed_point(3.0, 2.0);
    let l = k.inf_line(a, b);
    let p = k.point(0.0, 3.5);
    k.drive(DimKind::PointLine(p, l), "1.25");
    let r = k.reflect(p, l);
    assert!(k.dof() > 0);
    let d = k.measure(DimKind::Distance(p, r));
    k.check_value(d, 2.5);
}

#[test]
fn tiny_circle_tangent_to_axis() {
    let mut k = Kit::new();
    let o = k.origin();
    let c = k.point(0.0, 0.002);
    let ya = k.yaxis();
    k.on(c, ya);
    let ci = k.circle(c, 0.001);
    k.drive(DimKind::Diameter(ci), "0.002");
    let xa = k.xaxis();
    k.tangent(ci, xa);
    assert_eq!(k.dof(), 0);
    let y = k.measure(DimKind::CoordY(c));
    k.check_value(y, 0.001);
    let _ = o;
}

#[test]
fn huge_circle_tangent_chord() {
    let mut k = Kit::new();
    let o = k.fixed_point(0.0, 0.0);
    let c = k.circle(o, 1000.0);
    k.drive(DimKind::Radius(c), "1000");
    let p = k.point(600.0, 790.0);
    let q = k.point(-600.0, 790.0);
    k.on(p, c);
    k.on(q, c);
    let l = k.line(p, q);
    k.horizontal(l);
    k.drive(DimKind::VDist(o, p), "800");
    assert_eq!(k.dof(), 0);
    let len = k.measure(DimKind::Length(l));
    k.check_value(len, 1200.0);
}

#[test]
fn concentric_ring_width() {
    let mut k = Kit::new();
    let c0 = k.point(1.0, -2.0);
    let c1 = k.circle(c0, 1.0);
    let c2 = k.circle(c0, 2.0);
    k.drive(DimKind::Diameter(c1), "2");
    k.drive(DimKind::Diameter(c2), "2*pi");
    assert!(k.dof() > 0);
    let w = k.measure(DimKind::Radius(c2));
    k.check_value(w, PI);
}

#[test]
fn variable_scaled_rectangle() {
    let mut k = Kit::new();
    let s = k.var("s", 1.0);
    let o = k.origin();
    let p = k.point(1.0, 0.0);
    let q = k.point(1.0, 2.0);
    let l = k.line(o, p);
    let m = k.line(p, q);
    k.horizontal(l);
    k.perpendicular(l, m);
    k.drive(DimKind::Length(l), "1000*s");
    k.drive(DimKind::Length(m), "s/1000");
    let d = k.measure(DimKind::Distance(o, q));
    k.check_value(d, (1.0e6f64 + 1.0e-6).sqrt());
    k.set_var(s, 3.0);
    k.check_value(d, (9.0e6f64 + 9.0e-6).sqrt());
}

/// Triangle by three sides, angle at A acute; later made obtuse. The angle
/// formula must stay valid (no branch boundary is crossed: C stays above AB).
#[test]
fn sss_angle_acute_to_obtuse() {
    let mut k = Kit::new();
    let a = k.origin();
    let b = k.point(5.0, 0.1);
    let c = k.point(1.0, 3.0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.horizontal(ab);
    k.drive(DimKind::Length(ab), "5");
    k.drive(DimKind::Length(ac), "3");
    let dbc = k.drive(DimKind::Length(bc), "4");
    let ang = k.measure_angle(ab, ac);
    k.check_value(ang, (0.6f64).acos());
    let f = k.formula(ang);
    k.set(dbc, "7");
    k.still(ang, &f, "obtuse");
    k.check_value(ang, ((25.0 + 9.0 - 49.0) / 30.0f64).acos());
}

/// Right angle exactly at the configuration: SSS with 3,4,5, angle at C.
#[test]
fn sss_exact_right_angle() {
    let mut k = Kit::new();
    let a = k.origin();
    let b = k.point(5.0, 0.1);
    let c = k.point(1.6, 2.6);
    let ab = k.line(a, b);
    let ca = k.line(c, a);
    let cb = k.line(c, b);
    k.horizontal(ab);
    k.drive(DimKind::Length(ab), "5");
    k.drive(DimKind::Length(ca), "3");
    k.drive(DimKind::Length(cb), "4");
    let ang = k.measure_angle(ca, cb);
    let t = k.check_value(ang, PI / 2.0);
    assert!(t.contains('d'), "angle folded to a constant: {t}");
}

/// Point C exactly above A (x_C = 0) then moved to the left of A.
#[test]
fn third_point_crosses_y_axis() {
    let mut k = Kit::new();
    let a = k.origin();
    let b = k.point(4.0, 0.1);
    let c = k.point(0.1, 3.0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.horizontal(ab);
    k.drive(DimKind::Length(ab), "4");
    k.drive(DimKind::Length(ac), "3");
    let dbc = k.drive(DimKind::Length(bc), "5");
    let x = k.measure(DimKind::CoordX(c));
    let ang = k.measure_angle(ab, ac);
    k.check_value(x, 0.0);
    k.check_value(ang, PI / 2.0);
    let fx = k.formula(x);
    let fa = k.formula(ang);
    k.set(dbc, "6");
    k.still(x, &fx, "x left of A");
    k.still(ang, &fa, "angle obtuse");
}

/// Isosceles triangle with apex angle driven 60° (equilateral): the base
/// angles, measured, are π/3; then apex changed.
#[test]
fn isosceles_equilateral_special() {
    let mut k = Kit::new();
    let a = k.origin();
    let b = k.point(3.0, 2.0);
    let c = k.point(3.0, -2.0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.equal(ab, ac);
    k.drive(DimKind::Length(ab), "4");
    let th = k.drive_angle(ac, ab, "60");
    let d = k.measure(DimKind::Length(bc));
    k.check_value(d, 4.0);
    let f = k.formula(d);
    k.set(th, "150");
    k.still(d, &f, "apex 150");
}

/// Law of cosines with angle driven at 90°, large value perturbations.
#[test]
fn sas_large_value_scale() {
    let mut k = Kit::new();
    k.value_scale = 0.3;
    k.trials = 40;
    let a = k.origin();
    let b = k.point(3.0, 0.1);
    let c = k.point(0.1, 4.0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    k.horizontal(ab);
    k.drive(DimKind::Length(ab), "3");
    k.drive(DimKind::Length(ac), "4");
    k.drive_angle(ab, ac, "90");
    let d = k.measure(DimKind::Distance(b, c));
    k.check_value(d, 5.0);
}

/// SAS with driving angle 0 between the two sides: the third side is |a−b|.
#[test]
fn sas_zero_angle() {
    let mut k = Kit::new();
    let a = k.origin();
    let b = k.point(5.0, 0.1);
    let c = k.point(3.0, 0.3);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    k.horizontal(ab);
    k.drive(DimKind::Length(ab), "5");
    k.drive(DimKind::Length(ac), "3");
    k.drive_angle(ab, ac, "0");
    let d = k.measure(DimKind::Distance(b, c));
    k.check_value(d, 2.0);
}

/// SAS with driving angle 180°.
#[test]
fn sas_straight_angle() {
    let mut k = Kit::new();
    let a = k.origin();
    let b = k.point(5.0, 0.1);
    let c = k.point(-3.0, 0.3);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    k.horizontal(ab);
    k.drive(DimKind::Length(ab), "5");
    k.drive(DimKind::Length(ac), "3");
    k.drive_angle(ab, ac, "180");
    let d = k.measure(DimKind::Distance(b, c));
    k.check_value(d, 8.0);
}

/// Point on a circle with CoordX driven to the radius: y = 0 exactly.
#[test]
fn point_on_circle_at_extreme() {
    let mut k = Kit::new();
    let o = k.origin();
    let c = k.circle(o, 5.0);
    k.drive(DimKind::Radius(c), "5");
    let p = k.point(4.9, 0.5);
    k.on(p, c);
    k.drive(DimKind::CoordX(p), "4");
    let f = k.fixed_point(0.0, 5.0);
    let d = k.measure(DimKind::Distance(p, f));
    k.check_value(d, (16.0f64 + 4.0).sqrt());
}

/// Two circles intersecting at tiny scale (1e-4).
#[test]
fn tiny_circle_circle() {
    let mut k = Kit::new();
    let s = 1e-4;
    let a = k.origin();
    let b = k.point(5.0 * s, 0.1 * s);
    let ab = k.line(a, b);
    k.horizontal(ab);
    k.drive(DimKind::Length(ab), &format!("{}", 5.0 * s));
    let c = k.point(1.0 * s, 3.0 * s);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.drive(DimKind::Length(ac), &format!("{}", 3.0 * s));
    k.drive(DimKind::Length(bc), &format!("{}", 4.0 * s));
    let ang = k.measure_angle(ab, ac);
    k.check_value(ang, (0.6f64).acos());
}

/// Same at huge scale (1e5).
#[test]
fn huge_circle_circle() {
    let mut k = Kit::new();
    let s: f64 = std::env::var("HUGE").ok().and_then(|v| v.parse().ok()).unwrap_or(1e5);
    let a = k.origin();
    let b = k.point(5.0 * s, 0.1 * s);
    let ab = k.line(a, b);
    k.horizontal(ab);
    k.drive(DimKind::Length(ab), &format!("{}", 5.0 * s));
    let c = k.point(1.0 * s, 3.0 * s);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.drive(DimKind::Length(ac), &format!("{}", 3.0 * s));
    k.drive(DimKind::Length(bc), &format!("{}", 4.0 * s));
    let y = k.measure(DimKind::CoordY(c));
    k.check_value(y, 2.4 * s);
}

/// Zigzag strip of triangles: node k at given distances from nodes k−1 and
/// k−2 (circle-circle intersections chained). `lens(k)` gives the two lengths.
fn zigzag(k: &mut Kit, n: usize, lens: &dyn Fn(usize) -> (String, String)) -> Vec<usize> {
    let h = 3f64.sqrt();
    let mut ps = vec![k.origin()];
    let p1 = k.point(1.0, h);
    let l0 = k.line(ps[0], p1);
    if std::env::var("FLOAT").is_err() {
        let xa = k.xaxis();
        k.drive_angle(xa, l0, "60");
    } else {
        k.lock(ps[0]);
    }
    k.drive(DimKind::Length(l0), &lens(1).0);
    ps.push(p1);
    for i in 2..n {
        let g = if i % 2 == 0 { [i as f64, 0.0] } else { [i as f64, h] };
        let p = k.point(g[0], g[1]);
        let (a, b) = lens(i);
        let la = k.line(ps[i - 1], p);
        let lb = k.line(ps[i - 2], p);
        k.drive(DimKind::Length(la), &a);
        k.drive(DimKind::Length(lb), &b);
        ps.push(p);
    }
    ps
}

/// Zigzag strip with unequal lengths: distance from the first to the last node.
#[test]
fn long_truss_chain() {
    let ns: Vec<usize> = std::env::var("TN").map(|v| vec![v.parse().unwrap()]).unwrap_or(vec![4, 5, 6, 7]);
    for n in ns {
        let mut k = Kit::new();
        k.allow_inexact = true;
        let ps = zigzag(&mut k, n, &|i| (format!("{}", 2.0 + 0.05 * i as f64), format!("{}", 2.0 - 0.03 * i as f64)));
        for &p in &ps {
            println!("node {:?}", k.doc().pval(p));
        }
        let d = k.measure(DimKind::Distance(ps[0], *ps.last().unwrap()));
        let t0 = std::time::Instant::now();
        let f = k.check(d);
        println!("n={n}: {} chars, {:.0} ms", f.len(), t0.elapsed().as_secs_f64() * 1e3);
    }
}

/// Zigzag strip of equilateral triangles, all sides one variable s.
#[test]
fn equilateral_strip_variable() {
    for n in [4usize, 6, 8] {
        let mut k = Kit::new();
        k.var("s", 2.0);
        let ps = zigzag(&mut k, n, &|_| ("s".to_string(), "s".to_string()));
        let d = k.measure(DimKind::Distance(ps[0], *ps.last().unwrap()));
        let last = k.doc().pval(*ps.last().unwrap());
        k.check_value(d, last[0].hypot(last[1]));
    }
}

/// Mirrored construction: C below AB (y < 0), measure the signed y.
#[test]
fn mirrored_branch_signed_y() {
    let mut k = Kit::new();
    let a = k.origin();
    let b = k.point(5.0, 0.1);
    let c = k.point(1.8, -2.4);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.horizontal(ab);
    k.drive(DimKind::Length(ab), "5");
    k.drive(DimKind::Length(ac), "3");
    k.drive(DimKind::Length(bc), "4");
    let d = k.point(4.0, 3.0);
    let cd = k.line(c, d);
    let ad = k.line(a, d);
    k.drive(DimKind::Length(cd), "6");
    k.drive(DimKind::Length(ad), "5");
    let y = k.measure(DimKind::CoordY(d));
    println!("y_d {}", k.value(y));
    k.check(y);
}

/// Angle sum reaching exactly 180°: the measured angle sits at a fold.
#[test]
fn angle_sum_at_fold() {
    let mut k = Kit::new();
    let o = k.origin();
    let p = k.point(-1.0, 4.0);
    let q = k.point(-4.0, 0.3);
    let l1 = k.line(o, p);
    let l2 = k.line(o, q);
    let xa = k.xaxis();
    k.drive_angle(xa, l1, "100");
    k.drive_angle(l1, l2, "80");
    k.drive(DimKind::Length(l1), "4");
    k.drive(DimKind::Length(l2), "4");
    let y = k.measure(DimKind::CoordY(q));
    k.check_value(y, 0.0);
}

/// Angle sum near 180° measured as an angle (acos fold).
#[test]
fn angle_sum_measured_near_fold() {
    let mut k = Kit::new();
    let o = k.origin();
    let p = k.point(-1.0, 4.0);
    let q = k.point(-4.0, 0.3);
    let r = k.point(4.0, 0.0);
    let l1 = k.line(o, p);
    let l2 = k.line(o, q);
    let l0 = k.line(o, r);
    k.horizontal(l0);
    k.drive_angle(l0, l1, "100");
    k.drive_angle(l1, l2, "75");
    let ang = k.measure_angle(l0, l2);
    k.check_value(ang, 175f64.to_radians());
}

/// A point on a circle at angle 0 (on the x axis) via a driven angle.
#[test]
fn point_on_circle_angle_zero() {
    let mut k = Kit::new();
    let o = k.origin();
    let p = k.point(5.0, 0.3);
    let l = k.line(o, p);
    let xa = k.xaxis();
    k.drive_angle(xa, l, "0");
    k.drive(DimKind::Length(l), "5");
    let q = k.fixed_point(2.0, 3.0);
    let d = k.measure(DimKind::Distance(p, q));
    k.check_value(d, (9.0f64 + 9.0).sqrt());
}

/// Tiny driving angle (0.001°) with long sides.
#[test]
fn tiny_angle_long_sides() {
    let mut k = Kit::new();
    let a = k.origin();
    let b = k.point(1000.0, 0.1);
    let c = k.point(1000.0, 1.0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    k.horizontal(ab);
    k.drive(DimKind::Length(ab), "1000");
    k.drive(DimKind::Length(ac), "1000");
    k.drive_angle(ab, ac, "0.001");
    let d = k.measure(DimKind::Distance(b, c));
    k.check_value(d, 2000.0 * (0.0005f64.to_radians()).sin());
}

/// Circle-circle intersection with AB inclined at 30°: C (left of A→B) is
/// first above the x axis, then moved far so that it is below it.
#[test]
fn inclined_base_c_crosses_axis() {
    let mut k = Kit::new();
    let a = k.origin();
    let b = k.point(4.3, 2.5);
    let c = k.point(-1.0, 3.0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    let xa = k.xaxis();
    let th = k.drive_angle(xa, ab, "30");
    let _ = th;
    k.drive(DimKind::Length(ab), "5");
    let dac = k.drive(DimKind::Length(ac), "3");
    let dbc = k.drive(DimKind::Length(bc), "5");
    let y = k.measure(DimKind::CoordY(c));
    let x = k.measure(DimKind::CoordX(c));
    println!("C = ({}, {})", k.value(x), k.value(y));
    k.check(y);
    let fy = k.formula(y);
    let fx = k.formula(x);
    // obtuse at A, C swings round below the x axis
    k.set(dbc, "7.5");
    k.set(dac, "3.2");
    println!("C = ({}, {})", k.value(x), k.value(y));
    k.still(y, &fy, "y after");
    k.still(x, &fx, "x after");
}

fn nm(k: &Kit, d: usize) -> String {
    k.doc().dims[d].as_ref().unwrap().name.clone()
}

/// Driving dimension given by an expression that is 0 at the configuration.
#[test]
fn zero_valued_expression_dim() {
    let mut k = Kit::new();
    let a = k.origin();
    let b = k.point(3.0, 0.1);
    let c = k.point(3.1, 2.0);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    k.horizontal(ab);
    let d1 = k.drive(DimKind::Length(ab), "3");
    let n1 = nm(&k, d1);
    k.drive(DimKind::Length(bc), "2");
    k.drive(DimKind::HDist(b, c), &format!("{n1} - 3"));
    let d = k.measure(DimKind::Distance(a, c));
    k.check_value(d, 13f64.sqrt());
    let f = k.formula(d);
    println!("{:?}", f);
    k.set(d1, "4");
    // now DimKind::HDist(b,c) = 1 → C = (4 ± 1, ...)
    println!("C after: {:?}", k.doc().pval(c));
    k.still(d, &f, "d1 = 4");
}

/// A numeric zero driving dimension, later changed to a nonzero value.
#[test]
fn zero_driving_dim_changed() {
    let mut k = Kit::new();
    let a = k.origin();
    let b = k.point(3.0, 0.1);
    let c = k.point(3.1, 2.0);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    k.horizontal(ab);
    k.drive(DimKind::Length(ab), "3");
    k.drive(DimKind::Length(bc), "2");
    let h = k.drive(DimKind::HDist(b, c), "0");
    let d = k.measure(DimKind::Distance(a, c));
    let t = k.check_value(d, 13f64.sqrt());
    println!("{t}");
    let f = k.formula(d);
    k.set(h, "1");
    let m = k.value(d);
    println!("HDist 1: measured {m}, old formula {}", k.eval_now(&f));
    let t2 = k.check(d);
    println!("{t2}");
}

/// Floating triangle by three sides, dragged far (rotations of the whole).
#[test]
fn floating_triangle_big_drags() {
    let mut k = Kit::new();
    k.drag_scale = 1.0;
    k.trials = 40;
    let a = k.point(1.0, 1.0);
    let b = k.point(6.0, 1.3);
    let c = k.point(2.0, 4.0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.drive(DimKind::Length(ab), "5");
    k.drive(DimKind::Length(ac), "3");
    k.drive(DimKind::Length(bc), "4.5");
    let d = k.point(4.0, 5.0);
    let ad = k.line(a, d);
    let cd = k.line(c, d);
    k.drive(DimKind::Length(ad), "5");
    k.drive(DimKind::Length(cd), "2.5");
    let m = k.measure(DimKind::Distance(b, d));
    let ang = k.measure_angle(ab, cd);
    k.check(m);
    k.check(ang);
}

/// Reflection across a line through the origin at a driven angle; the angle
/// is then moved past 90°.
#[test]
fn reflection_across_rotating_line() {
    let mut k = Kit::new();
    let o = k.origin();
    let r = k.point(3.0, 2.0);
    let l = k.inf_line(o, r);
    let xa = k.xaxis();
    let th = k.drive_angle(xa, l, "40");
    let p = k.point(4.0, 1.0);
    k.drive(DimKind::CoordX(p), "4");
    k.drive(DimKind::CoordY(p), "1");
    let q = k.reflect(p, l);
    let qx = k.measure(DimKind::CoordX(q));
    k.check(qx);
    let f = k.formula(qx);
    for v in ["80", "100", "150", "175"] {
        k.set(th, v);
        k.still(qx, &f, v);
    }
}

/// Ladder of 12 squares (linear chain): far corner distance, timing.
#[test]
fn square_ladder_chain() {
    let mut k = Kit::new();
    let n = 12;
    let mut a = k.origin();
    let b0 = k.point(0.0, 1.0);
    let l0 = k.line(a, b0);
    k.vertical(l0);
    k.drive(DimKind::Length(l0), "1");
    let mut b = b0;
    let mut prev = l0;
    for i in 0..n {
        let a2 = k.point(i as f64 + 1.0, 0.1);
        let b2 = k.point(i as f64 + 1.0, 1.1);
        let la = k.line(a, a2);
        let lb = k.line(b, b2);
        let lr = k.line(a2, b2);
        k.perpendicular(prev, la);
        k.drive(DimKind::Length(la), &format!("{}", 1.0 + 0.1 * i as f64));
        k.drive(DimKind::Length(lr), "1");
        k.perpendicular(la, lr);
        let _ = lb;
        a = a2;
        b = b2;
        prev = lr;
    }
    let d = k.measure(DimKind::Distance(k.origin(), b));
    k.check(d);
}

/// Many independent triangles; the target lives in the last one.
#[test]
fn many_independent_blocks() {
    let mut k = Kit::new();
    let mut last = 0;
    for i in 0..20 {
        let x0 = 10.0 * i as f64;
        let a = k.point(x0, 0.0);
        let b = k.point(x0 + 4.0, 0.1);
        let c = k.point(x0 + 1.0, 3.0);
        let ab = k.line(a, b);
        let ac = k.line(a, c);
        let bc = k.line(b, c);
        k.drive(DimKind::Length(ab), &format!("{}", 4.0 + 0.1 * i as f64));
        k.drive(DimKind::Length(ac), "3");
        k.drive(DimKind::Length(bc), "3.5");
        last = k.measure_angle(ab, ac);
    }
    k.check(last);
}

/// Large articulation on a chained circle-circle construction.
#[test]
fn chained_intersections_large_values() {
    let mut k = Kit::new();
    k.value_scale = 0.3;
    k.trials = 60;
    let a = k.origin();
    let b = k.point(4.3, 2.5);
    let c = k.point(-1.0, 3.0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    let xa = k.xaxis();
    k.drive_angle(xa, ab, "30");
    k.drive(DimKind::Length(ab), "5");
    k.drive(DimKind::Length(ac), "3");
    k.drive(DimKind::Length(bc), "5");
    let d = k.point(2.0, 6.0);
    let cd = k.line(c, d);
    let bd = k.line(b, d);
    k.drive(DimKind::Length(cd), "4");
    k.drive(DimKind::Length(bd), "4");
    let m = k.measure(DimKind::Distance(a, d));
    let y = k.measure(DimKind::CoordY(d));
    k.check(m);
    k.check(y);
}

/// Arc with a reflex central angle (counter-clockwise from A to B > 180°).
#[test]
fn reflex_arc_chord() {
    let mut k = Kit::new();
    let o = k.origin();
    let a = k.point(3.0, 0.1);
    let b = k.point(0.1, -3.0);
    let arc = k.arc(o, a, b);
    let oa = k.line(o, a);
    let ob = k.line(o, b);
    k.horizontal(oa);
    k.drive(DimKind::Radius(arc), "3");
    k.drive_angle(oa, ob, "100");
    let d = k.measure(DimKind::Distance(a, b));
    k.check_value(d, 6.0 * (50f64.to_radians()).sin());
}

/// Point at a driven distance from a line on a circle: the point crosses the
/// circle's horizontal diameter when the distance is changed.
#[test]
fn point_on_circle_from_line_distance() {
    let mut k = Kit::new();
    let o = k.origin();
    let c = k.circle(o, 5.0);
    k.drive(DimKind::Radius(c), "5");
    let p = k.point(4.0, 3.0);
    k.on(p, c);
    let a = k.point(-6.0, 6.0);
    let b = k.point(6.0, 6.1);
    let l = k.inf_line(a, b);
    k.horizontal(l);
    k.drive(DimKind::CoordY(a), "6");
    k.drive(DimKind::CoordX(a), "-6");
    let dd = k.drive(DimKind::PointLine(p, l), "3");
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, 4.0);
    let f = k.formula(x);
    for v in ["5.5", "6", "6.5", "8", "10.5"] {
        k.set(dd, v);
        println!("dist {v}: p = {:?}", k.doc().pval(p));
        k.still(x, &f, v);
    }
}

/// Chained circle-circle intersections and a tangent at a small scale; only
/// the value at the configuration is compared (the numeric solver's own
/// tolerance is absolute, so articulation at this scale is not meaningful).
fn small_scale_chain(s: f64) {
    let mut k = Kit::new();
    k.trials = 0;
    let f = |v: f64| format!("{}", v * s);
    let a = k.origin();
    let b = k.point(4.3 * s, 2.5 * s);
    let c = k.point(-1.0 * s, 3.0 * s);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    let xa = k.xaxis();
    k.drive_angle(xa, ab, "30");
    k.drive(DimKind::Length(ab), &f(5.0));
    k.drive(DimKind::Length(ac), &f(3.0));
    k.drive(DimKind::Length(bc), &f(5.0));
    let d = k.point(2.0 * s, 6.0 * s);
    let cd = k.line(c, d);
    let bd = k.line(b, d);
    k.drive(DimKind::Length(cd), &f(4.0));
    k.drive(DimKind::Length(bd), &f(4.0));
    let m = k.measure(DimKind::Distance(a, d));
    let y = k.measure(DimKind::CoordY(d));
    let circ = k.circle(a, 1.5 * s);
    k.drive(DimKind::Radius(circ), &f(1.5));
    let (_seg, t) = k.tangent_line(d, circ, [-1.0 * s, 1.0 * s]);
    let ty = k.measure(DimKind::CoordY(t));
    println!("scale {s}: |AD| = {}, y_D = {}, y_T = {}", k.value(m), k.value(y), k.value(ty));
    k.check(m);
    k.check(y);
    k.check(ty);
}

#[test]
fn small_scale_chain_1() {
    small_scale_chain(1.0);
}

#[test]
fn small_scale_chain_1e_3() {
    small_scale_chain(1e-3);
}

#[test]
fn small_scale_chain_1e_5() {
    small_scale_chain(1e-5);
}

#[test]
fn large_scale_chain_1e4() {
    small_scale_chain(1e4);
}

/// Tangent from a point to a circle at scale s: contact coordinate.
fn tangent_at_scale(s: f64) {
    let mut k = Kit::new();
    k.trials = 0;
    let o = k.origin();
    let c = k.circle(o, 3.0 * s);
    k.drive(DimKind::Radius(c), &format!("{}", 3.0 * s));
    let p = k.point(5.0 * s, 4.0 * s);
    k.drive(DimKind::CoordX(p), &format!("{}", 5.0 * s));
    k.drive(DimKind::CoordY(p), &format!("{}", 4.0 * s));
    let (_seg, t) = k.tangent_line(p, c, [3.0 * s, -0.5 * s]);
    let ty = k.measure(DimKind::CoordY(t));
    // contact: x·5 + y·4 = 9, x² + y² = 9 (in units of s)
    let yy = (36.0 - (36.0f64 * 36.0 - 41.0 * (81.0 - 225.0)).sqrt()) / 41.0;
    println!("y_T = {}, expected {}", k.value(ty), yy * s);
    k.check(ty);
}

#[test]
fn tangent_scale_1() {
    tangent_at_scale(1.0);
}

#[test]
fn tangent_scale_1e_2() {
    tangent_at_scale(1e-2);
}

#[test]
fn tangent_scale_1e_3() {
    tangent_at_scale(1e-3);
}
