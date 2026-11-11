//! Theorem-level invariants: under-constrained constructions whose checked
//! dimension is fixed by a classical theorem.

use gxpad::model::DimKind;
use gxpad::testkit::Kit;
use std::f64::consts::PI;

fn dname(k: &Kit, d: usize) -> String {
    k.doc().dims[d].as_ref().unwrap().name.clone()
}

/// Points on a circle (center `c`, radius `r`) at the given angles in degrees.
fn on_circle(k: &mut Kit, circ: usize, c: [f64; 2], r: f64, deg: f64) -> usize {
    let p = k.point(c[0] + r * deg.to_radians().cos(), c[1] + r * deg.to_radians().sin());
    k.on(p, circ);
    p
}

/// Thales: a floating circle, diameter AB, P anywhere on the circle.
#[test]
fn thales_floating_circle() {
    let mut k = Kit::new();
    let c = k.point(1.0, 1.0);
    let circ = k.circle(c, 2.0);
    k.drive(DimKind::Radius(circ), "2");
    let a = k.point(-1.0, 1.0);
    let b = k.point(3.0, 1.0);
    k.on(a, circ);
    k.on(b, circ);
    let ab = k.line(a, b);
    k.on(c, ab);
    let p = on_circle(&mut k, circ, [1.0, 1.0], 2.0, 70.0);
    let pa = k.line(p, a);
    let pb = k.line(p, b);
    assert!(k.dof() > 0);
    let ang = k.measure(DimKind::Angle(pa, pb));
    k.check_value(ang, PI / 2.0);
}

/// Thales with a radius variable: the legs satisfy PA^2 + PB^2 = (2r)^2.
#[test]
fn thales_leg_from_leg() {
    let mut k = Kit::new();
    let r = k.var("r", 2.5);
    let c = k.point(0.2, 0.1);
    let circ = k.circle(c, 2.5);
    k.drive(DimKind::Radius(circ), "r");
    let a = k.point(-2.3, 0.2);
    let b = k.point(2.7, 0.0);
    k.on(a, circ);
    k.on(b, circ);
    let ab = k.line(a, b);
    k.on(c, ab);
    let p = on_circle(&mut k, circ, [0.2, 0.1], 2.5, 60.0);
    let pa = k.line(p, a);
    let pb = k.line(p, b);
    k.drive(DimKind::Length(pa), "3");
    assert!(k.dof() > 0);
    let leg = k.measure(DimKind::Length(pb));
    k.check_value(leg, (100.0f64 - 9.0).sqrt() * 0.0 + (4.0 * 2.5 * 2.5 - 9.0f64).sqrt());
    k.set_var(r, 3.0);
    k.check_value(leg, (36.0f64 - 9.0).sqrt());
}

/// Inscribed angle theorem: two points on the same arc see a fixed chord under
/// the same angle, and the central angle is twice that.
#[test]
fn inscribed_angle_same_arc() {
    let mut k = Kit::new();
    let o = k.origin();
    let circ = k.circle(o, 2.0);
    k.drive(DimKind::Radius(circ), "2");
    let a = on_circle(&mut k, circ, [0.0, 0.0], 2.0, 200.0);
    let b = on_circle(&mut k, circ, [0.0, 0.0], 2.0, 340.0);
    let p = on_circle(&mut k, circ, [0.0, 0.0], 2.0, 80.0);
    let q = on_circle(&mut k, circ, [0.0, 0.0], 2.0, 120.0);
    let pa = k.line(p, a);
    let pb = k.line(p, b);
    let qa = k.line(q, a);
    let qb = k.line(q, b);
    let oa = k.line(o, a);
    let ob = k.line(o, b);
    k.drive(DimKind::Angle(pa, pb), "70");
    assert!(k.dof() > 0);
    let iq = k.measure(DimKind::Angle(qa, qb));
    let cen = k.measure(DimKind::Angle(oa, ob));
    println!("inscribed {} central {}", k.value(iq).to_degrees(), k.value(cen).to_degrees());
    k.check_value(iq, 70f64.to_radians());
    k.check_value(cen, 140f64.to_radians());
}

/// Inscribed angle on the opposite arc is supplementary (cyclic quadrilateral).
#[test]
fn inscribed_angle_opposite_arc() {
    let mut k = Kit::new();
    let c = k.point(0.0, 0.0);
    let circ = k.circle(c, 2.0);
    let a = on_circle(&mut k, circ, [0.0, 0.0], 2.0, 190.0);
    let b = on_circle(&mut k, circ, [0.0, 0.0], 2.0, 350.0);
    let p = on_circle(&mut k, circ, [0.0, 0.0], 2.0, 90.0);
    let q = on_circle(&mut k, circ, [0.0, 0.0], 2.0, 270.0);
    let pa = k.line(p, a);
    let pb = k.line(p, b);
    let qa = k.line(q, a);
    let qb = k.line(q, b);
    k.drive(DimKind::Radius(circ), "2");
    k.drive(DimKind::Angle(pa, pb), "75");
    assert!(k.dof() > 0);
    let other = k.measure(DimKind::Angle(qa, qb));
    let v = k.value(other);
    println!("opposite value {}", v.to_degrees());
    k.check_value(other, 105f64.to_radians());
}

/// Chord length from a fixed central angle while the chord rotates freely.
#[test]
fn chord_from_central_angle() {
    let mut k = Kit::new();
    let o = k.origin();
    let r = k.var("r", 3.0);
    let circ = k.circle(o, 3.0);
    k.drive(DimKind::Radius(circ), "r");
    let a = on_circle(&mut k, circ, [0.0, 0.0], 3.0, 20.0);
    let b = on_circle(&mut k, circ, [0.0, 0.0], 3.0, 90.0);
    let oa = k.line(o, a);
    let ob = k.line(o, b);
    let ab = k.line(a, b);
    k.drive(DimKind::Angle(oa, ob), "70");
    assert!(k.dof() > 0);
    let chord = k.measure(DimKind::Length(ab));
    k.check_value(chord, 2.0 * 3.0 * 35f64.to_radians().sin());
    k.set_var(r, 4.5);
    k.check_value(chord, 2.0 * 4.5 * 35f64.to_radians().sin());
}

/// Distance from the center to a chord of fixed length is constant.
#[test]
fn chord_distance_from_center() {
    let mut k = Kit::new();
    let c = k.point(0.5, -0.5);
    let circ = k.circle(c, 3.0);
    k.drive(DimKind::Radius(circ), "3");
    let a = on_circle(&mut k, circ, [0.5, -0.5], 3.0, 10.0);
    let b = on_circle(&mut k, circ, [0.5, -0.5], 3.0, 100.0);
    let ab = k.line(a, b);
    k.drive(DimKind::Length(ab), "4");
    assert!(k.dof() > 0);
    let h = k.measure(DimKind::PointLine(c, ab));
    k.check_value(h, 5.0f64.sqrt());
}

/// Equal chords are equidistant from the center.
#[test]
fn equal_chords_equidistant() {
    let mut k = Kit::new();
    let o = k.origin();
    let circ = k.circle(o, 3.0);
    k.drive(DimKind::Radius(circ), "3");
    let a = on_circle(&mut k, circ, [0.0, 0.0], 3.0, 20.0);
    let b = on_circle(&mut k, circ, [0.0, 0.0], 3.0, 110.0);
    let c = on_circle(&mut k, circ, [0.0, 0.0], 3.0, 200.0);
    let d = on_circle(&mut k, circ, [0.0, 0.0], 3.0, 300.0);
    let ab = k.line(a, b);
    let cd = k.line(c, d);
    k.drive(DimKind::Length(ab), "4");
    k.equal(ab, cd);
    assert!(k.dof() > 0);
    let h = k.measure(DimKind::PointLine(o, cd));
    k.check_value(h, 5.0f64.sqrt());
}

/// Power of a point (secants): PA * PB = |OP|^2 - r^2 = PC * PD.
#[test]
fn power_of_point_secants() {
    let mut k = Kit::new();
    let o = k.origin();
    let circ = k.circle(o, 2.0);
    k.drive(DimKind::Radius(circ), "2");
    let p = k.point(5.0, 0.1);
    k.drive(DimKind::Distance(o, p), "5");
    // first secant along OP
    let a = k.point(2.0, 0.0);
    let b = k.point(-2.0, 0.0);
    k.on(a, circ);
    k.on(b, circ);
    let ab = k.line(p, b);
    k.on(a, ab);
    k.on(o, ab);
    // second secant, laid out near the intended configuration: from P along
    // direction α with PC = 3.4 and PD = 21/3.4 (otherwise the nearest
    // solution of the constraints is the degenerate C = D)
    let alpha = (-(3.4f64 * 3.4 + 21.0) / 34.0).acos();
    let at = |t: f64| (5.0 + t * alpha.cos(), t * alpha.sin());
    let (cx, cy) = at(3.4);
    let (dx, dy) = at(21.0 / 3.4);
    let c = k.point(cx, cy);
    k.on(c, circ);
    let d = k.point(dx, dy);
    k.on(d, circ);
    let pd = k.line(p, d);
    k.on(c, pd);
    let pc = k.line(p, c);
    k.drive(DimKind::Length(pc), "3.4");
    assert!(k.dof() > 0);
    let pa = k.measure(DimKind::Distance(p, a));
    let pbm = k.measure(DimKind::Distance(p, b));
    let pdm = k.measure(DimKind::Distance(p, d));
    k.check_value(pa, 3.0);
    k.check_value(pbm, 7.0);
    k.check_value(pdm, 21.0 / 3.4);
}

/// Power of a point driven by a product expression of two dimensions.
#[test]
fn power_of_point_product_expression() {
    let mut k = Kit::new();
    let o = k.origin();
    let circ = k.circle(o, 3.0);
    k.drive(DimKind::Radius(circ), "3");
    let x = k.point(1.0, 0.0);
    k.drive(DimKind::Distance(o, x), "1");
    let c = on_circle(&mut k, circ, [0.0, 0.0], 3.0, 80.0);
    let d = on_circle(&mut k, circ, [0.0, 0.0], 3.0, 260.0);
    let cd = k.line(c, d);
    k.on(x, cd);
    let e = on_circle(&mut k, circ, [0.0, 0.0], 3.0, 160.0);
    let f = on_circle(&mut k, circ, [0.0, 0.0], 3.0, 330.0);
    let ef = k.line(e, f);
    k.on(x, ef);
    let xc = k.line(x, c);
    k.drive(DimKind::Length(xc), "2.5");
    assert!(k.dof() > 0);
    let xd = k.measure(DimKind::Distance(x, d));
    let xe = k.measure(DimKind::Distance(x, e));
    let xf = k.measure(DimKind::Distance(x, f));
    let (n1, n2) = (dname(&k, xe), dname(&k, xf));
    // the product XE * XF equals XC * XD: compare through a defined variable
    k.check_value(xd, 8.0 / 2.5);
    // XE alone depends on how chord EF is turned about X; only XE·XF is fixed
    let prod = k.value(xe) * k.value(xf);
    assert!((prod - 8.0).abs() < 1e-7, "{n1}*{n2} = {prod}");
    // a segment whose length is the product divided by XC
    let g0 = k.point(6.0, 6.0);
    let g1 = k.point(8.0, 6.0);
    let g = k.line(g0, g1);
    k.drive(DimKind::Length(g), &format!("{n1}*{n2}/2.5"));
    let gl = k.measure(DimKind::Length(g));
    k.check_value(gl, 8.0 / 2.5);
}

/// Intersecting chords: XA * XB = XC * XD with X inside, one chord free to turn.
#[test]
fn intersecting_chords() {
    let mut k = Kit::new();
    let o = k.origin();
    let circ = k.circle(o, 3.0);
    k.drive(DimKind::Radius(circ), "3");
    let x = k.point(1.2, 0.3);
    k.drive(DimKind::Distance(o, x), "1");
    let a = on_circle(&mut k, circ, [0.0, 0.0], 3.0, 10.0);
    let b = on_circle(&mut k, circ, [0.0, 0.0], 3.0, 190.0);
    let ab = k.line(a, b);
    k.on(x, ab);
    let c = on_circle(&mut k, circ, [0.0, 0.0], 3.0, 100.0);
    let d = on_circle(&mut k, circ, [0.0, 0.0], 3.0, 280.0);
    let cd = k.line(c, d);
    k.on(x, cd);
    k.drive(DimKind::Distance(x, c), "2");
    assert!(k.dof() > 0);
    let xd = k.measure(DimKind::Distance(x, d));
    k.check_value(xd, 4.0);
    let xa = k.measure(DimKind::Distance(x, a));
    let xb = k.measure(DimKind::Distance(x, b));
    // XA and XB individually depend on how the chord AB is turned about X;
    // only their product (the power of X) is determined
    let prod = k.value(xa) * k.value(xb);
    assert!((prod - 8.0).abs() < 1e-7);
}

/// Tangent-secant: tangent length^2 = PA * PB.
#[test]
fn tangent_secant_theorem() {
    let mut k = Kit::new();
    let o = k.origin();
    let circ = k.circle(o, 3.0);
    k.drive(DimKind::Radius(circ), "3");
    let p = k.point(5.0, 0.2);
    k.drive(DimKind::Distance(o, p), "5");
    let (tl, _t) = k.tangent_line(p, circ, [1.8, 2.4]);
    let a = on_circle(&mut k, circ, [0.0, 0.0], 3.0, 15.0);
    let b = on_circle(&mut k, circ, [0.0, 0.0], 3.0, 130.0);
    let pb = k.line(p, b);
    k.on(a, pb);
    k.drive(DimKind::Distance(p, a), "2.5");
    assert!(k.dof() > 0);
    let tlen = k.measure(DimKind::Length(tl));
    k.check_value(tlen, 4.0);
    let pbl = k.measure(DimKind::Length(pb));
    k.check_value(pbl, 16.0 / 2.5);
}

/// Tangent length from a point on a concentric circle is constant.
#[test]
fn tangent_length_concentric() {
    let mut k = Kit::new();
    let c = k.point(1.0, 1.0);
    let big = k.circle(c, 5.0);
    let small = k.circle(c, 3.0);
    k.drive(DimKind::Radius(big), "5");
    k.drive(DimKind::Radius(small), "3");
    let p = on_circle(&mut k, big, [1.0, 1.0], 5.0, 35.0);
    let (tl, _) = k.tangent_line(p, small, [1.0 + 3.0 * 0.9f64.cos(), 1.0 + 3.0 * 0.9f64.sin()]);
    assert!(k.dof() > 0);
    let len = k.measure(DimKind::Length(tl));
    k.check_value(len, 4.0);
}

/// The chord of the big circle tangent to the small one has length 2 sqrt(R^2 - r^2).
#[test]
fn annulus_tangent_chord() {
    let mut k = Kit::new();
    let o = k.origin();
    let big = k.circle(o, 5.0);
    let small = k.circle(o, 3.0);
    k.drive(DimKind::Radius(big), "5");
    k.drive(DimKind::Radius(small), "3");
    let p = on_circle(&mut k, big, [0.0, 0.0], 5.0, 127.0);
    let q = on_circle(&mut k, big, [0.0, 0.0], 5.0, 53.0);
    let pq = k.line(p, q);
    k.tangent(pq, small);
    assert!(k.dof() > 0);
    let len = k.measure(DimKind::Length(pq));
    k.check_value(len, 8.0);
}

/// Pythagoras with a floating right triangle.
#[test]
fn pythagoras_floating() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(3.1, 0.3);
    let c = k.point(0.1, 4.2);
    let ca = k.line(c, a);
    let cb = k.line(c, b);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let ab2 = k.line(a, b);
    let _ = (ac, ab2);
    k.perpendicular(ab, ca);
    k.drive(DimKind::Length(ab), "3");
    k.drive(DimKind::Length(ca), "4");
    assert!(k.dof() > 0);
    let hyp = k.measure(DimKind::Length(cb));
    k.check_value(hyp, 5.0);
}

/// Parallelogram law: d1^2 + d2^2 = 2 (a^2 + b^2).
#[test]
fn parallelogram_law() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(3.0, 0.2);
    let c = k.point(4.0, 2.0);
    let d = k.point(1.0, 1.9);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let cd = k.line(c, d);
    let da = k.line(d, a);
    k.parallel(ab, cd);
    k.parallel(bc, da);
    k.drive(DimKind::Length(ab), "3");
    k.drive(DimKind::Length(bc), "2");
    let ac = k.line(a, c);
    k.drive(DimKind::Length(ac), "4");
    assert!(k.dof() > 0);
    let bd = k.line(b, d);
    let l = k.measure(DimKind::Length(bd));
    k.check_value(l, (2.0 * 9.0 + 2.0 * 4.0 - 16.0f64).sqrt());
}

/// Rhombus: diagonals are perpendicular and the side is half the hypotenuse of the diagonals.
#[test]
fn rhombus_diagonals() {
    let mut k = Kit::new();
    let a = k.point(-3.0, 0.0);
    let b = k.point(0.1, -4.0);
    let c = k.point(3.0, 0.1);
    let d = k.point(0.0, 4.0);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let cd = k.line(c, d);
    let da = k.line(d, a);
    k.equal(ab, bc);
    k.equal(bc, cd);
    k.equal(cd, da);
    let ac = k.line(a, c);
    let bd = k.line(b, d);
    k.drive(DimKind::Length(ac), "6");
    k.drive(DimKind::Length(bd), "8");
    assert!(k.dof() > 0);
    let side = k.measure(DimKind::Length(ab));
    k.check_value(side, 5.0);
    let ang = k.measure(DimKind::Angle(ac, bd));
    println!("diag angle {}", k.value(ang).to_degrees());
    k.check_value(ang, PI / 2.0);
}

/// Viviani: the distances from an interior point to the sides of an equilateral
/// triangle add up to its height.
#[test]
fn viviani() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(4.0, 0.1);
    let c = k.point(2.0, 3.4);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let ca = k.line(c, a);
    k.drive(DimKind::Length(ab), "4");
    k.equal(ab, bc);
    k.equal(bc, ca);
    let p = k.point(2.0, 1.2);
    k.drive(DimKind::PointLine(p, ab), "1");
    k.drive(DimKind::PointLine(p, bc), "1.3");
    assert!(k.dof() > 0);
    let h3 = k.measure(DimKind::PointLine(p, ca));
    k.check_value(h3, 2.0 * 3f64.sqrt() - 2.3);
}

/// Viviani with a side variable and the point sliding on a parallel to a side.
#[test]
fn viviani_variable_side() {
    let mut k = Kit::new();
    let s = k.var("s", 6.0);
    let a = k.point(0.0, 0.0);
    let b = k.point(6.0, 0.0);
    let c = k.point(3.0, 5.2);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let ca = k.line(c, a);
    k.horizontal(ab);
    k.drive(DimKind::Length(ab), "s");
    k.equal(ab, bc);
    k.equal(bc, ca);
    let p = k.point(3.0, 1.5);
    let d1 = k.drive(DimKind::PointLine(p, ab), "1.5");
    let _ = d1;
    let d2 = k.measure(DimKind::PointLine(p, bc));
    let d3 = k.measure(DimKind::PointLine(p, ca));
    let n2 = dname(&k, d2);
    // sum of the three distances is the height: tie via the sliding point
    let sum = k.value(d2) + k.value(d3) + 1.5;
    assert!((sum - 3.0 * 3f64.sqrt()).abs() < 1e-7);
    // the free coordinate of P along AB moves the two other distances, not their sum
    k.drive(DimKind::PointLine(p, bc), "2");
    assert!(k.dof() == 1 + 0 || k.dof() > 0, "{n2}");
    k.check_value(d3, 3.0 * 3f64.sqrt() - 3.5);
    k.set_var(s, 8.0);
    k.check_value(d3, 4.0 * 3f64.sqrt() - 3.5);
}

/// Ladder: the midpoint of a ladder sliding between the axes stays at distance L/2 from the corner.
#[test]
fn ladder_midpoint() {
    let mut k = Kit::new();
    let o = k.origin();
    let a = k.point(4.0, 0.0);
    let b = k.point(0.0, 3.0);
    let xa = k.xaxis();
    let ya = k.yaxis();
    k.on(a, xa);
    k.on(b, ya);
    let ab = k.line(a, b);
    k.drive(DimKind::Length(ab), "6");
    let m = k.midpoint_of_line(ab);
    assert_eq!(k.dof(), 1);
    let om = k.measure(DimKind::Distance(o, m));
    k.check_value(om, 3.0);
}

/// Ladder: point at ratio 2:3, its coordinates satisfy the ellipse equation; distance
/// to the corner at the ends of the slide is checked through the invariant |AP|.
#[test]
fn ladder_ratio_point() {
    let mut k = Kit::new();
    let o = k.origin();
    let a = k.point(4.0, 0.0);
    let b = k.point(0.0, 3.0);
    let xa = k.xaxis();
    let ya = k.yaxis();
    k.on(a, xa);
    k.on(b, ya);
    let ab = k.line(a, b);
    k.drive(DimKind::Length(ab), "5");
    let p = k.point(2.4, 1.2);
    k.on(p, ab);
    k.drive(DimKind::Distance(a, p), "2");
    assert!(k.dof() > 0);
    // P = A + 2/5 (B - A): P_x = 0.6 A_x, P_y = 0.4 B_y. With the ladder at angle t:
    // |OP|^2 = 9 cos^2 t + 4 sin^2 t, but the horizontal distance from P to B's axis
    // and the vertical one to A's axis tie to the ellipse (x/3)^2 + (y/2)^2 = 1.
    let px = k.measure(DimKind::HDist(o, p));
    let py = k.measure(DimKind::VDist(o, p));
    let (x, y) = (k.value(px), k.value(py));
    assert!((x * x / 9.0 + y * y / 4.0 - 1.0).abs() < 1e-7);
    let pb = k.measure(DimKind::Distance(p, b));
    k.check_value(pb, 3.0);
    // driving the angle of the ladder fixes x and y
    let ang = k.measure(DimKind::Angle(xa, ab));
    println!("ladder angle {}", k.value(ang).to_degrees());
    let t = (k.value(py) / 2.0).asin();
    let _ = t;
}

/// Midline theorem: the segment joining midpoints of two sides is parallel to the
/// third and half as long.
#[test]
fn midline_theorem() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(6.0, 0.5);
    let c = k.point(2.0, 4.0);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let ca = k.line(c, a);
    k.drive(DimKind::Length(bc), "7");
    let m = k.midpoint_of_line(ab);
    let n = k.midpoint_of_line(ca);
    let mn = k.line(m, n);
    assert!(k.dof() > 0);
    let l = k.measure(DimKind::Length(mn));
    k.check_value(l, 3.5);
    let ang = k.measure(DimKind::Angle(mn, bc));
    println!("midline angle {}", k.value(ang).to_degrees());
    let v = k.value(ang);
    assert!(v.abs() < 1e-9 || (v - PI).abs() < 1e-9);
    k.check(ang);
}

/// Varignon: the midpoints of the sides of any quadrilateral form a parallelogram
/// whose sides are half the diagonals.
#[test]
fn varignon_parallelogram() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(5.0, 0.4);
    let c = k.point(6.0, 4.0);
    let d = k.point(0.5, 3.0);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let cd = k.line(c, d);
    let da = k.line(d, a);
    let ac = k.line(a, c);
    let bd = k.line(b, d);
    k.drive(DimKind::Length(ac), "7");
    k.drive(DimKind::Length(bd), "5");
    let m1 = k.midpoint_of_line(ab);
    let m2 = k.midpoint_of_line(bc);
    let m3 = k.midpoint_of_line(cd);
    let m4 = k.midpoint_of_line(da);
    let s12 = k.line(m1, m2);
    let s23 = k.line(m2, m3);
    assert!(k.dof() > 0);
    let l1 = k.measure(DimKind::Length(s12));
    let l2 = k.measure(DimKind::Length(s23));
    k.check_value(l1, 3.5);
    k.check_value(l2, 2.5);
    let d14 = k.measure(DimKind::Distance(m1, m4));
    k.check_value(d14, 2.5);
}

/// The median to the hypotenuse of a right triangle equals half the hypotenuse.
#[test]
fn right_triangle_median() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(4.0, 0.2);
    let c = k.point(0.1, 3.0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.perpendicular(ab, ac);
    k.drive(DimKind::Length(ab), "4");
    k.drive(DimKind::Length(ac), "3");
    let m = k.midpoint_of_line(bc);
    assert!(k.dof() > 0);
    let am = k.measure(DimKind::Distance(a, m));
    k.check_value(am, 2.5);
}

/// Circumradius of a right triangle is half the hypotenuse (circumcenter by construction).
#[test]
fn right_triangle_circumradius() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(6.0, 0.2);
    let c = k.point(0.1, 8.0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let _bc = k.line(b, c);
    k.perpendicular(ab, ac);
    k.drive(DimKind::Length(ab), "6");
    k.drive(DimKind::Length(ac), "8");
    let (l1, _) = k.perp_bisector(a, b);
    let (l2, _) = k.perp_bisector(a, c);
    let x = k.intersect(l1, l2, [3.0, 4.0]);
    assert!(k.dof() > 0);
    let r = k.measure(DimKind::Distance(x, a));
    k.check_value(r, 5.0);
    let r2 = k.measure(DimKind::Distance(x, c));
    k.check_value(r2, 5.0);
}

/// Altitude to the hypotenuse: h = ab / c and the foot splits it into a^2/c and b^2/c.
#[test]
fn right_triangle_altitude() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(4.0, 0.1);
    let c = k.point(0.1, 3.0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.perpendicular(ab, ac);
    k.drive(DimKind::Length(ab), "4");
    k.drive(DimKind::Length(ac), "3");
    let alt = k.perp_line(bc, a);
    let h = k.intersect(alt, bc, [1.9, 1.4]);
    assert!(k.dof() > 0);
    let ah = k.measure(DimKind::Distance(a, h));
    k.check_value(ah, 12.0 / 5.0);
    let bh = k.measure(DimKind::Distance(b, h));
    k.check_value(bh, 16.0 / 5.0);
    let ch = k.measure(DimKind::Distance(c, h));
    k.check_value(ch, 9.0 / 5.0);
}

/// Angle bisector theorem: BD / DC = AB / AC.
#[test]
fn angle_bisector_theorem() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(3.0, 0.1);
    let c = k.point(4.0, 3.5);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let ca = k.line(c, a);
    k.drive(DimKind::Length(ab), "3");
    k.drive(DimKind::Length(bc), "6");
    k.drive(DimKind::Length(ca), "5");
    let (bis, _) = k.angle_bisector(ab, ca, [2.5, 1.8]);
    let ibc = k.inf_line(b, c);
    let d = k.intersect(bis, ibc, [3.4, 1.8]);
    assert!(k.dof() > 0);
    let bd = k.measure(DimKind::Distance(b, d));
    let dc = k.measure(DimKind::Distance(d, c));
    k.check_value(bd, 6.0 * 3.0 / 8.0);
    k.check_value(dc, 6.0 * 5.0 / 8.0);
}

/// The centroid divides each median 2:1; median length from Apollonius.
#[test]
fn centroid_median_ratio() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(6.0, 0.3);
    let c = k.point(2.0, 4.5);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let ca = k.line(c, a);
    k.drive(DimKind::Length(ab), "6");
    k.drive(DimKind::Length(bc), "5");
    k.drive(DimKind::Length(ca), "4");
    let ma = k.midpoint_of_line(bc);
    let mb = k.midpoint_of_line(ca);
    let med_a = k.inf_line(a, ma);
    let med_b = k.inf_line(b, mb);
    let g = k.intersect(med_a, med_b, [2.6, 1.6]);
    assert!(k.dof() > 0);
    let ag = k.measure(DimKind::Distance(a, g));
    let m2 = (2.0 * 36.0 + 2.0 * 16.0 - 25.0) / 4.0;
    k.check_value(ag, 2.0 / 3.0 * f64::sqrt(m2));
    let gm = k.measure(DimKind::Distance(g, ma));
    k.check_value(gm, 1.0 / 3.0 * f64::sqrt(m2));
}

/// Incircle radius = area / semiperimeter, for a floating 3-4-5 triangle.
#[test]
fn inradius_345() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(4.0, 0.2);
    let c = k.point(0.1, 3.1);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.drive(DimKind::Length(ab), "4");
    k.drive(DimKind::Length(ac), "3");
    k.drive(DimKind::Length(bc), "5");
    let ic = k.point(1.0, 1.0);
    let circ = k.circle(ic, 1.0);
    k.tangent(circ, ab);
    k.tangent(circ, ac);
    k.tangent(circ, bc);
    assert!(k.dof() > 0);
    let r = k.measure(DimKind::Radius(circ));
    k.check_value(r, 1.0);
}

/// Ptolemy: for a cyclic quadrilateral AC * BD = AB * CD + BC * DA; the diagonal
/// follows from the four sides.
#[test]
fn ptolemy_diagonals() {
    let mut k = Kit::new();
    let c0 = k.point(0.0, 0.0);
    let circ = k.circle(c0, 2.0);
    let angs = [15.0f64, 95.0, 205.0, 285.0];
    let pts: Vec<usize> = angs.iter().map(|&t| on_circle(&mut k, circ, [0.0, 0.0], 2.0, t)).collect();
    let (a, b, c, d) = (pts[0], pts[1], pts[2], pts[3]);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let cd = k.line(c, d);
    let da = k.line(d, a);
    let (sab, sbc, scd, sda) = (2.6f64, 3.1, 2.4, 3.3);
    k.drive(DimKind::Length(ab), &sab.to_string());
    k.drive(DimKind::Length(bc), &sbc.to_string());
    k.drive(DimKind::Length(cd), &scd.to_string());
    k.drive(DimKind::Length(da), &sda.to_string());
    assert!(k.dof() > 0);
    let ac = k.line(a, c);
    let bd = k.line(b, d);
    let p = sab * scd + sbc * sda;
    let ac2 = p * (sab * sda + sbc * scd) / (sab * sbc + scd * sda);
    let bd2 = p * (sab * sbc + scd * sda) / (sab * sda + sbc * scd);
    let acm = k.measure(DimKind::Length(ac));
    let bdm = k.measure(DimKind::Length(bd));
    k.check_value(acm, ac2.sqrt());
    k.check_value(bdm, bd2.sqrt());
    assert!((ac2.sqrt() * bd2.sqrt() - p).abs() < 1e-9);
}

/// Tangent-chord angle equals the inscribed angle on the chord.
#[test]
fn tangent_chord_angle() {
    let mut k = Kit::new();
    let o = k.origin();
    let circ = k.circle(o, 2.0);
    k.drive(DimKind::Radius(circ), "2");
    let a = on_circle(&mut k, circ, [0.0, 0.0], 2.0, 10.0);
    let b = on_circle(&mut k, circ, [0.0, 0.0], 2.0, 100.0);
    let oa = k.line(o, a);
    let ob = k.line(o, b);
    k.drive(DimKind::Angle(oa, ob), "84");
    let ab = k.line(a, b);
    let tan = k.perp_line(oa, a);
    assert!(k.dof() > 0);
    let ang = k.measure(DimKind::Angle(ab, tan));
    println!("tangent-chord {}", k.value(ang).to_degrees());
    // the tangent-chord angle is 42 deg or its supplement, depending on the sector
    k.check_value(ang, 138f64.to_radians());
}

/// Triangle angle sum via line directions: the third angle follows from two driven ones.
#[test]
fn triangle_angle_sum() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(4.0, 0.1);
    let c = k.point(1.0, 2.5);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.drive(DimKind::Length(ab), "4");
    k.drive(DimKind::Angle(ab, ac), "50");
    k.drive(DimKind::Angle(ab, bc), "110");
    assert!(k.dof() > 0);
    let ang = k.measure(DimKind::Angle(ac, bc));
    println!("third {}", k.value(ang).to_degrees());
    let cl = k.measure(DimKind::Length(ac));
    // law of sines: AC = AB sin(B) / sin(C), B = 70, C = 60
    k.check_value(cl, 4.0 * 70f64.to_radians().sin() / 60f64.to_radians().sin());
    k.check(ang);
}

/// Fully constrained: the foot of the altitude on the hypotenuse of a 6-8-10 triangle.
#[test]
fn altitude_foot_fully_constrained() {
    let mut k = Kit::new();
    let o = k.origin();
    let b = k.point(6.0, 0.1);
    let c = k.point(0.1, 8.0);
    let ob = k.line(o, b);
    let oc = k.line(o, c);
    let bc = k.line(b, c);
    k.horizontal(ob);
    k.perpendicular(ob, oc);
    k.drive(DimKind::Length(ob), "6");
    k.drive(DimKind::Length(oc), "8");
    let alt = k.perp_line(bc, o);
    let h = k.intersect(alt, bc, [2.9, 2.3]);
    assert_eq!(k.dof(), 0);
    let oh = k.measure(DimKind::Distance(o, h));
    k.check_value(oh, 4.8);
    let hx = k.measure(DimKind::CoordX(h));
    k.check_value(hx, 36.0 * 6.0 / 100.0 * 0.0 + 4.8 * 4.8 / 6.0);
}
