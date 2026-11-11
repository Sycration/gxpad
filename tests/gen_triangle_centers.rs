//! Triangle centers and related quantities. The reference triangle is the
//! 13-14-15 triangle: B=(0,0), C=(14,0), A=(5,12), area 84, inradius 4,
//! circumradius 65/8, circumcenter (7, 33/8), centroid (19/3, 4), orthocenter (5, 15/4).

use gxpad::model::DimKind;
use gxpad::testkit::Kit;
use std::f64::consts::PI;

const R: f64 = 65.0 / 8.0;

struct Tri {
    a: usize,
    b: usize,
    c: usize,
    /// side opposite A (BC), opposite B (CA), opposite C (AB)
    la: usize,
    lb: usize,
    lc: usize,
}

/// Floating triangle with the three side lengths given as driving text.
fn floating(k: &mut Kit, sa: &str, sb: &str, sc: &str) -> Tri {
    let a = k.point(5.2, 11.8);
    let b = k.point(-0.2, 0.1);
    let c = k.point(14.1, -0.2);
    let la = k.line(b, c);
    let lb = k.line(c, a);
    let lc = k.line(a, b);
    k.drive(DimKind::Length(la), sa);
    k.drive(DimKind::Length(lb), sb);
    k.drive(DimKind::Length(lc), sc);
    Tri { a, b, c, la, lb, lc }
}

fn floating_1314_15(k: &mut Kit) -> Tri {
    floating(k, "14", "15", "13")
}

#[test]
fn circumradius_circle_through_vertices() {
    let mut k = Kit::new();
    let t = floating_1314_15(&mut k);
    let (circ, _o) = k.circle_through(t.a, t.b, t.c);
    assert!(k.dof() > 0);
    let r = k.measure(DimKind::Radius(circ));
    k.check_value(r, R);
}

#[test]
fn inradius_circle_tangent_to_sides() {
    let mut k = Kit::new();
    let o = k.origin();
    let c = k.point(14.2, 0.0);
    let a = k.point(5.1, 11.9);
    let bc = k.inf_line(o, c);
    let ca = k.inf_line(c, a);
    let ab = k.inf_line(a, o);
    k.horizontal(bc);
    k.drive(DimKind::Length(bc), "14");
    k.drive(DimKind::Length(ca), "15");
    k.drive(DimKind::Length(ab), "13");
    let i = k.point(6.0, 3.9);
    let circ = k.circle(i, 4.0);
    k.tangent(circ, bc);
    k.tangent(circ, ca);
    k.tangent(circ, ab);
    assert_eq!(k.dof(), 0);
    let r = k.measure(DimKind::Radius(circ));
    k.check_value(r, 4.0);
    let ix = k.measure(DimKind::CoordX(i));
    k.check_value(ix, 6.0);
}

#[test]
fn centroid_on_medians() {
    let mut k = Kit::new();
    let t = floating_1314_15(&mut k);
    let ma = k.midpoint_of(t.b, t.c);
    let mb = k.midpoint_of(t.c, t.a);
    let la = k.line(t.a, ma);
    let lb = k.line(t.b, mb);
    let g = k.intersect(la, lb, [6.3, 4.1]);
    assert!(k.dof() > 0);
    let med = k.measure(DimKind::Length(la));
    k.check_value(med, 148.0f64.sqrt());
    let ag = k.measure(DimKind::Distance(t.a, g));
    k.check_value(ag, 2.0 / 3.0 * 148.0f64.sqrt());
    let gm = k.measure(DimKind::Distance(g, ma));
    k.check_value(gm, 148.0f64.sqrt() / 3.0);
}

#[test]
fn orthocenter_distance_from_vertex() {
    let mut k = Kit::new();
    let t = floating_1314_15(&mut k);
    let h1 = k.perp_line(t.la, t.a);
    let h2 = k.perp_line(t.lb, t.b);
    let h = k.intersect(h1, h2, [5.0, 3.8]);
    assert!(k.dof() > 0);
    // AH = 2 R cos A
    let ah = k.measure(DimKind::Distance(t.a, h));
    k.check_value(ah, 8.25);
    let bh = k.measure(DimKind::Distance(t.b, h));
    // BH = 2 R cos B, cos B = 5/13... (a^2+c^2-b^2)/(2ac) = 140/364
    k.check_value(bh, 2.0 * R * (196.0 + 169.0 - 225.0) / (2.0 * 14.0 * 13.0));
}

#[test]
fn euler_line_og_gh() {
    let mut k = Kit::new();
    let t = floating_1314_15(&mut k);
    let (p1, _) = k.perp_bisector(t.b, t.c);
    let (p2, _) = k.perp_bisector(t.c, t.a);
    let o = k.intersect(p1, p2, [7.0, 4.0]);
    let ma = k.midpoint_of(t.b, t.c);
    let mb = k.midpoint_of(t.c, t.a);
    let m1 = k.line(t.a, ma);
    let m2 = k.line(t.b, mb);
    let g = k.intersect(m1, m2, [6.3, 4.0]);
    let h1 = k.perp_line(t.la, t.a);
    let h2 = k.perp_line(t.lb, t.b);
    let h = k.intersect(h1, h2, [5.0, 3.8]);
    assert!(k.dof() > 0);
    let og = k.measure(DimKind::Distance(o, g));
    let gh = k.measure(DimKind::Distance(g, h));
    let oh = k.measure(DimKind::Distance(o, h));
    let oh_v = (9.0 * R * R - 590.0f64).sqrt();
    k.check_value(oh, oh_v);
    k.check_value(og, oh_v / 3.0);
    k.check_value(gh, 2.0 * oh_v / 3.0);
}

#[test]
fn nine_point_circle_radius() {
    let mut k = Kit::new();
    let t = floating_1314_15(&mut k);
    let ma = k.midpoint_of(t.b, t.c);
    let mb = k.midpoint_of(t.c, t.a);
    let mc = k.midpoint_of(t.a, t.b);
    let (circ, _n) = k.circle_through(ma, mb, mc);
    assert!(k.dof() > 0);
    let r = k.measure(DimKind::Radius(circ));
    k.check_value(r, R / 2.0);
}

#[test]
fn nine_point_circle_through_foot_of_altitude() {
    let mut k = Kit::new();
    let t = floating_1314_15(&mut k);
    let ma = k.midpoint_of(t.b, t.c);
    let mb = k.midpoint_of(t.c, t.a);
    let mc = k.midpoint_of(t.a, t.b);
    let (_circ, n) = k.circle_through(ma, mb, mc);
    // foot of the altitude from A lies on BC and on the nine-point circle
    let alt = k.perp_line(t.la, t.a);
    let bc_inf = k.inf_line(t.b, t.c);
    let foot = k.intersect(alt, bc_inf, [5.0, 0.0]);
    let on_circle = k.measure(DimKind::Distance(n, foot));
    k.check_value(on_circle, R / 2.0);
    let bf = k.measure(DimKind::Distance(t.b, foot));
    k.check_value(bf, 5.0);
}

#[test]
fn incenter_by_angle_bisectors() {
    let mut k = Kit::new();
    let o = k.origin();
    let c = k.point(14.2, 0.0);
    let a = k.point(5.1, 11.9);
    let bc = k.line(o, c);
    let ca = k.line(c, a);
    let ab = k.line(a, o);
    k.horizontal(bc);
    k.drive(DimKind::Length(bc), "14");
    k.drive(DimKind::Length(ca), "15");
    k.drive(DimKind::Length(ab), "13");
    let (b1, _) = k.angle_bisector(ab, bc, [4.0, 1.5]);
    let (b2, _) = k.angle_bisector(bc, ca, [10.0, 1.5]);
    let i = k.intersect(b1, b2, [6.0, 4.0]);
    assert_eq!(k.dof(), 0);
    let ix = k.measure(DimKind::CoordX(i));
    let iy = k.measure(DimKind::CoordY(i));
    k.check_value(ix, 6.0);
    k.check_value(iy, 4.0);
    let d = k.measure(DimKind::PointLine(i, ca));
    k.check_value(d, 4.0);
}

#[test]
fn incenter_distance_to_sides_floating() {
    let mut k = Kit::new();
    let t = floating_1314_15(&mut k);
    let (b1, _) = k.angle_bisector(t.lc, t.la, [4.0, 1.5]);
    let (b2, _) = k.angle_bisector(t.la, t.lb, [10.0, 1.5]);
    let i = k.intersect(b1, b2, [6.0, 4.0]);
    assert!(k.dof() > 0);
    let d1 = k.measure(DimKind::PointLine(i, t.lb));
    k.check_value(d1, 4.0);
    // AI = r / sin(A/2); sin^2(A/2) = (1-cosA)/2, cosA = 198/390
    let ai = k.measure(DimKind::Distance(t.a, i));
    let sa2 = ((1.0 - 198.0 / 390.0) / 2.0f64).sqrt();
    k.check_value(ai, 4.0 / sa2);
}

#[test]
fn angle_bisector_foot_ratio() {
    let mut k = Kit::new();
    let t = floating_1314_15(&mut k);
    let (bis, _) = k.angle_bisector(t.lc, t.lb, [5.0, 6.0]);
    let bc = k.inf_line(t.b, t.c);
    let d = k.intersect(bis, bc, [6.5, 0.0]);
    assert!(k.dof() > 0);
    // BD = a c / (b + c)
    let bd = k.measure(DimKind::Distance(t.b, d));
    k.check_value(bd, 14.0 * 13.0 / 28.0);
    let dc = k.measure(DimKind::Distance(d, t.c));
    k.check_value(dc, 14.0 * 15.0 / 28.0);
}

#[test]
fn altitude_lengths() {
    let mut k = Kit::new();
    let t = floating_1314_15(&mut k);
    assert!(k.dof() > 0);
    let ha = k.measure(DimKind::PointLine(t.a, t.la));
    let hb = k.measure(DimKind::PointLine(t.b, t.lb));
    let hc = k.measure(DimKind::PointLine(t.c, t.lc));
    k.check_value(ha, 12.0);
    k.check_value(hb, 168.0 / 15.0);
    k.check_value(hc, 168.0 / 13.0);
}

#[test]
fn heron_with_defined_variables() {
    let mut k = Kit::new();
    k.var("p", 14.0);
    k.var("q", 15.0);
    k.var("r", 13.0);
    let t = floating(&mut k, "p", "q", "r");
    k.defvar("s", "(p+q+r)/2");
    let (circ, _o) = k.circle_through(t.a, t.b, t.c);
    assert!(k.dof() > 0);
    let rad = k.measure(DimKind::Radius(circ));
    // R = pqr / (4 Area), Area = sqrt(s(s-p)(s-q)(s-r))
    k.check_value(rad, 14.0 * 15.0 * 13.0 / (4.0 * 84.0));
    let h = k.measure(DimKind::PointLine(t.a, t.la));
    k.check_value(h, 12.0);
}

#[test]
fn heron_variable_change_inradius() {
    let mut k = Kit::new();
    let p = k.var("p", 14.0);
    let t = floating(&mut k, "p", "15", "13");
    let o = k.point(6.0, 4.0);
    let circ = k.circle(o, 4.0);
    k.tangent(circ, t.la);
    k.tangent(circ, t.lb);
    k.tangent(circ, t.lc);
    assert!(k.dof() > 0);
    let r = k.measure(DimKind::Radius(circ));
    k.check_value(r, 4.0);
    k.set_var(p, 13.0);
    let s: f64 = (13.0 + 15.0 + 13.0) / 2.0;
    let area = (s * (s - 13.0) * (s - 15.0) * (s - 13.0)).sqrt();
    k.check_value(r, area / s);
}

#[test]
fn excircle_radius() {
    let mut k = Kit::new();
    let o = k.origin();
    let c = k.point(14.2, 0.0);
    let a = k.point(5.1, 11.9);
    let bc = k.inf_line(o, c);
    let ca = k.inf_line(c, a);
    let ab = k.inf_line(a, o);
    k.horizontal(bc);
    k.drive(DimKind::Length(bc), "14");
    k.drive(DimKind::Length(ca), "15");
    k.drive(DimKind::Length(ab), "13");
    let e = k.point(8.0, -11.0);
    let circ = k.circle(e, 12.0);
    k.tangent(circ, bc);
    k.tangent(circ, ca);
    k.tangent(circ, ab);
    assert_eq!(k.dof(), 0);
    let r = k.measure(DimKind::Radius(circ));
    k.check_value(r, 12.0);
    let ex = k.measure(DimKind::CoordX(e));
    k.check_value(ex, 8.0);
}

#[test]
fn sas_third_side() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(5.0, 0.2);
    let c = k.point(4.0, 6.5);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.drive(DimKind::Length(ab), "5");
    k.drive(DimKind::Length(ac), "8");
    let ang = k.drive(DimKind::Angle(ab, ac), "60");
    assert!(k.dof() > 0);
    let _ = ang;
    let third = k.measure(DimKind::Length(bc));
    k.check_value(third, 7.0);
    let h = k.measure(DimKind::PointLine(c, ab));
    k.check_value(h, 8.0 * (PI / 3.0).sin());
}

#[test]
fn asa_side_by_law_of_sines() {
    let mut k = Kit::new();
    let b = k.point(0.0, 0.0);
    let c = k.point(10.0, 0.3);
    let a = k.point(4.0, 7.0);
    let bc = k.line(b, c);
    let ba = k.line(b, a);
    let ca = k.line(c, a);
    k.drive(DimKind::Length(bc), "10");
    k.drive(DimKind::Angle(bc, ba), "50");
    // the exterior angle at C is 120 degrees, so the interior angle is 60
    k.drive(DimKind::Angle(ca, bc), "120");
    assert!(k.dof() > 0);
    // angle A = 70; AB = a sin C / sin A
    let s70 = 70f64.to_radians().sin();
    let ab = k.measure(DimKind::Length(ba));
    k.check_value(ab, 10.0 * 60f64.to_radians().sin() / s70);
    let ac = k.measure(DimKind::Length(ca));
    k.check_value(ac, 10.0 * 50f64.to_radians().sin() / s70);
}

#[test]
fn isosceles_height_variable() {
    let mut k = Kit::new();
    let h = k.var("h", 4.0);
    let b = k.point(-3.0, 0.1);
    let c = k.point(3.1, 0.0);
    let a = k.point(0.2, 4.0);
    let bc = k.line(b, c);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    k.equal(ab, ac);
    k.drive(DimKind::Length(bc), "6");
    let m = k.midpoint_of(b, c);
    k.drive(DimKind::Distance(a, m), "h");
    assert!(k.dof() > 0);
    let leg = k.measure(DimKind::Length(ab));
    k.check_value(leg, 5.0);
    k.set_var(h, 2.0);
    k.check_value(leg, 13.0f64.sqrt());
    // apex angle
    let apex = k.measure(DimKind::Angle(ab, ac));
    let v = k.value(apex);
    let want = 2.0 * (3.0f64 / 2.0).atan();
    assert!((v - want).abs() < 1e-9 || (v - (PI - want)).abs() < 1e-9 || (v - (2.0 * PI - want)).abs() < 1e-9, "apex {v}");
}

#[test]
fn right_triangle_altitude_and_circumradius() {
    let mut k = Kit::new();
    let c = k.point(0.0, 0.0);
    let a = k.point(4.0, 0.2);
    let b = k.point(0.1, 3.0);
    let ca = k.line(c, a);
    let cb = k.line(c, b);
    let ab = k.line(a, b);
    k.perpendicular(ca, cb);
    k.drive(DimKind::Length(ca), "4");
    k.drive(DimKind::Length(cb), "3");
    assert!(k.dof() > 0);
    let hyp = k.measure(DimKind::Length(ab));
    k.check_value(hyp, 5.0);
    let alt = k.measure(DimKind::PointLine(c, ab));
    k.check_value(alt, 12.0 / 5.0);
    let m = k.midpoint_of_line(ab);
    let cm = k.measure(DimKind::Distance(c, m));
    k.check_value(cm, 2.5);
}

#[test]
fn right_triangle_incircle_radius() {
    let mut k = Kit::new();
    let c = k.point(0.0, 0.0);
    let a = k.point(4.0, 0.2);
    let b = k.point(0.1, 3.0);
    let ca = k.inf_line(c, a);
    let cb = k.inf_line(c, b);
    let ab = k.inf_line(a, b);
    k.perpendicular(ca, cb);
    k.drive(DimKind::Length(ca), "4");
    k.drive(DimKind::Length(cb), "3");
    let i = k.point(1.1, 0.9);
    let circ = k.circle(i, 1.0);
    k.tangent(circ, ca);
    k.tangent(circ, cb);
    k.tangent(circ, ab);
    assert!(k.dof() > 0);
    // r = (a + b - c) / 2
    let r = k.measure(DimKind::Radius(circ));
    k.check_value(r, 1.0);
}

#[test]
fn sliding_apex_centroid_height() {
    let mut k = Kit::new();
    let b = k.fixed_point(0.0, 0.0);
    let c = k.point(6.0, 0.0);
    let a = k.point(2.0, 4.0);
    let bc = k.line(b, c);
    k.horizontal(bc);
    k.drive(DimKind::Length(bc), "6");
    let xa = k.xaxis();
    k.drive(DimKind::PointLine(a, xa), "4");
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let ma = k.midpoint_of(b, c);
    let mb = k.midpoint_of(c, a);
    let m1 = k.line(a, ma);
    let m2 = k.line(b, mb);
    let g = k.intersect(m1, m2, [2.7, 1.3]);
    assert!(k.dof() > 0);
    let _ = (ab, ac);
    let gh = k.measure(DimKind::PointLine(g, xa));
    k.check_value(gh, 4.0 / 3.0);
}

#[test]
fn sliding_apex_circumcenter_fixed_base() {
    // Apex slides on the perpendicular bisector of the base: isosceles, the
    // circumradius is determined by the apex height.
    let mut k = Kit::new();
    let b = k.fixed_point(-3.0, 0.0);
    let c = k.fixed_point(3.0, 0.0);
    let a = k.point(0.1, 4.0);
    let axis = k.inf_line(k.origin(), a);
    k.vertical(axis);
    let (circ, _o) = k.circle_through(a, b, c);
    let hgt = k.var("hh", 4.0);
    let yaxis_dim = k.drive(DimKind::CoordY(a), "hh");
    let _ = yaxis_dim;
    assert_eq!(k.dof(), 0);
    let r = k.measure(DimKind::Radius(circ));
    k.check_value(r, (9.0 + 16.0) / 8.0);
    k.set_var(hgt, 6.0);
    k.check_value(r, (9.0 + 36.0) / 12.0);
}

#[test]
fn ladder_midpoint_circle() {
    // B slides on the x-axis and C on the y-axis with |BC| fixed: the
    // midpoint of BC stays at distance L/2 from the origin.
    let mut k = Kit::new();
    let o = k.origin();
    let b = k.point(4.0, 0.0);
    let c = k.point(0.0, 3.0);
    let xa = k.xaxis();
    let ya = k.yaxis();
    k.on(b, xa);
    k.on(c, ya);
    let bc = k.line(b, c);
    k.drive(DimKind::Length(bc), "5");
    let m = k.midpoint_of_line(bc);
    assert_eq!(k.dof(), 1);
    let om = k.measure(DimKind::Distance(o, m));
    k.check_value(om, 2.5);
    // distance from the right angle to the hypotenuse is not constant, but
    // the circumradius of OBC is
    let circ = k.circle(m, 2.0);
    k.on(o, circ);
    let r = k.measure(DimKind::Radius(circ));
    k.check_value(r, 2.5);
}

#[test]
/// A is pinned at arbitrary decimal coordinates; the distances do not depend
/// on where A is, so those decimals must not appear in the formula.
fn rotating_triangle_centroid_distance() {
    let mut k = Kit::new();
    let t = floating_1314_15(&mut k);
    k.lock(t.a);
    assert_eq!(k.dof(), 1);
    let ma = k.midpoint_of(t.b, t.c);
    let mb = k.midpoint_of(t.c, t.a);
    let m1 = k.line(t.a, ma);
    let m2 = k.line(t.b, mb);
    let g = k.intersect(m1, m2, [6.3, 4.0]);
    let ag = k.measure(DimKind::Distance(t.a, g));
    k.check_value(ag, 2.0 / 3.0 * 148.0f64.sqrt());
    let bg = k.measure(DimKind::Distance(t.b, g));
    // m_b = 1/2 sqrt(2a^2 + 2c^2 - b^2)
    let mbl = 0.5 * (2.0 * 196.0 + 2.0 * 169.0 - 225.0f64).sqrt();
    k.check_value(bg, 2.0 / 3.0 * mbl);
}

#[test]
fn midline_length() {
    let mut k = Kit::new();
    let t = floating_1314_15(&mut k);
    let mc = k.midpoint_of(t.a, t.b);
    let mb = k.midpoint_of(t.c, t.a);
    let mid = k.line(mc, mb);
    assert!(k.dof() > 0);
    let l = k.measure(DimKind::Length(mid));
    k.check_value(l, 7.0);
    // the midline is parallel to BC: the angle between them is 0 or pi
    assert!(!k.try_constrain(gxpad::model::Con::Parallel(mid, t.la)), "parallelism is already implied");
}

#[test]
fn equilateral_circumradius_inradius() {
    let mut k = Kit::new();
    let s = k.var("s", 6.0);
    let a = k.point(0.0, 3.4);
    let b = k.point(-3.0, -0.1);
    let c = k.point(3.1, 0.0);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let ca = k.line(c, a);
    k.equal(ab, bc);
    k.equal(bc, ca);
    k.drive(DimKind::Length(bc), "s");
    assert!(k.dof() > 0);
    let (circ, o) = k.circle_through(a, b, c);
    let inc = k.circle(o, 1.5);
    let ab_inf = k.inf_line(a, b);
    k.tangent(inc, ab_inf);
    let r_out = k.measure(DimKind::Radius(circ));
    let r_in = k.measure(DimKind::Radius(inc));
    k.check_value(r_out, 6.0 / 3.0f64.sqrt());
    k.check_value(r_in, 6.0 / (2.0 * 3.0f64.sqrt()));
    k.set_var(s, 9.0);
    k.check_value(r_out, 9.0 / 3.0f64.sqrt());
    k.check_value(r_in, 9.0 / (2.0 * 3.0f64.sqrt()));
}

#[test]
fn equilateral_by_angles_height() {
    let mut k = Kit::new();
    let b = k.point(0.0, 0.0);
    let c = k.point(4.0, 0.1);
    let a = k.point(2.0, 3.3);
    let bc = k.line(b, c);
    let ba = k.line(b, a);
    let ca = k.line(c, a);
    k.drive(DimKind::Length(bc), "4");
    k.drive(DimKind::Angle(bc, ba), "60");
    // exterior angle at C
    k.drive(DimKind::Angle(ca, bc), "120");
    assert!(k.dof() > 0);
    let h = k.measure(DimKind::PointLine(a, bc));
    k.check_value(h, 12.0f64.sqrt());
}

#[test]
fn isosceles_apex_angle_base() {
    let mut k = Kit::new();
    let l = k.var("L", 5.0);
    // apex at the origin side, wedge contains the 45 degree label direction
    let a = k.point(0.0, 0.0);
    let b = k.point(4.3, 2.5);
    let c = k.point(1.7, 4.7);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.drive(DimKind::Length(ab), "L");
    k.equal(ab, ac);
    k.drive(DimKind::Angle(ab, ac), "40");
    assert!(k.dof() > 0);
    let base = k.measure(DimKind::Length(bc));
    let want = 2.0 * 5.0 * 20f64.to_radians().sin();
    k.check_value(base, want);
    k.set_var(l, 7.0);
    k.check_value(base, 2.0 * 7.0 * 20f64.to_radians().sin());
    let h = k.measure(DimKind::PointLine(a, bc));
    k.check_value(h, 7.0 * 20f64.to_radians().cos());
}

#[test]
fn coordinate_triangle_all_centers() {
    // right triangle A=(0,0), B=(6,0), C=(0,8): O=(3,4), G=(2,8/3), H=A, I=(2,2)
    let mut k = Kit::new();
    let o = k.origin();
    let b = k.point(6.0, 0.2);
    let c = k.point(0.2, 8.0);
    k.drive(DimKind::CoordX(b), "6");
    k.drive(DimKind::CoordY(b), "0");
    k.drive(DimKind::CoordX(c), "0");
    k.drive(DimKind::CoordY(c), "8");
    assert_eq!(k.dof(), 0);
    let ab = k.line(o, b);
    let bc = k.line(b, c);
    let ca = k.line(c, o);
    let (p1, _) = k.perp_bisector(o, b);
    let (p2, _) = k.perp_bisector(o, c);
    let circum = k.intersect(p1, p2, [3.0, 4.0]);
    let ox = k.measure(DimKind::CoordX(circum));
    let oy = k.measure(DimKind::CoordY(circum));
    k.check_value(ox, 3.0);
    k.check_value(oy, 4.0);
    let mbc = k.midpoint_of(b, c);
    let mca = k.midpoint_of(c, o);
    let m1 = k.line(o, mbc);
    let m2 = k.line(b, mca);
    let g = k.intersect(m1, m2, [2.0, 2.6]);
    let gx = k.measure(DimKind::CoordX(g));
    let gy = k.measure(DimKind::CoordY(g));
    k.check_value(gx, 2.0);
    k.check_value(gy, 8.0 / 3.0);
    let (b1, _) = k.angle_bisector(ab, bc, [4.0, 1.0]);
    let (b2, _) = k.angle_bisector(ab, ca, [1.0, 1.0]);
    let i = k.intersect(b1, b2, [2.0, 2.0]);
    let ix = k.measure(DimKind::CoordX(i));
    let iy = k.measure(DimKind::CoordY(i));
    k.check_value(ix, 2.0);
    k.check_value(iy, 2.0);
}

#[test]
fn ceva_medians_concurrent_with_symmetric_point() {
    // the reflection of the centroid in the midpoint of BC is the fourth
    // vertex of the parallelogram ABDC; AD = 2 * AM
    let mut k = Kit::new();
    let t = floating_1314_15(&mut k);
    let ma = k.midpoint_of(t.b, t.c);
    let line_a = k.inf_line(t.a, ma);
    let d = k.point(9.0, -12.0);
    k.on(d, line_a);
    let md = k.line(ma, d);
    let am = k.line(t.a, ma);
    k.equal(md, am);
    assert!(k.dof() > 0);
    let ad = k.measure(DimKind::Distance(t.a, d));
    k.check_value(ad, 2.0 * 148.0f64.sqrt());
    // BD = AC for the parallelogram ABDC
    let bd = k.measure(DimKind::Distance(t.b, d));
    k.check_value(bd, 15.0);
}

#[test]
fn euler_line_og_only() {
    let mut k = Kit::new();
    let t = floating_1314_15(&mut k);
    let (p1, _) = k.perp_bisector(t.b, t.c);
    let (p2, _) = k.perp_bisector(t.c, t.a);
    let o = k.intersect(p1, p2, [7.0, 4.0]);
    let ma = k.midpoint_of(t.b, t.c);
    let mb = k.midpoint_of(t.c, t.a);
    let m1 = k.line(t.a, ma);
    let m2 = k.line(t.b, mb);
    let g = k.intersect(m1, m2, [6.3, 4.0]);
    assert!(k.dof() > 0);
    let og = k.measure(DimKind::Distance(o, g));
    let oh_v = (9.0 * R * R - 590.0f64).sqrt();
    k.check_value(og, oh_v / 3.0);
}
