//! Edge-case constructions: extreme scales, all quadrants, signs, mirrored
//! layouts, axes, arcs, exact irrational values, far value changes.

use gxpad::model::DimKind;
use gxpad::testkit::Kit;
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

