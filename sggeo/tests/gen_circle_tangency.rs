//! Circle tangency configurations: chains, inscribed circles, common tangents,
//! tangent segments, Soddy circles, fillets, packings.

use sggeo::model::DimKind;
mod testkit;
use testkit::Kit;
use std::f64::consts::PI;

/// A circle with a fresh center point.
fn disc(k: &mut Kit, x: f64, y: f64, r: f64) -> (usize, usize) {
    let p = k.point(x, y);
    let c = k.circle(p, r);
    (p, c)
}

/// Two circles tangent externally: center distance is r1 + r2.
#[test]
fn external_pair_distance() {
    let mut k = Kit::new();
    let (a, c1) = disc(&mut k, 0.0, 0.0, 3.0);
    let (b, c2) = disc(&mut k, 5.2, 0.3, 2.0);
    k.tangent(c1, c2);
    k.drive(DimKind::Radius(c1), "3");
    k.drive(DimKind::Radius(c2), "2");
    assert!(k.dof() > 0);
    let d = k.measure(DimKind::Distance(a, b));
    k.check_value(d, 5.0);
}

/// Internal tangency: center distance is r1 - r2.
#[test]
fn internal_pair_distance() {
    let mut k = Kit::new();
    let (a, c1) = disc(&mut k, 0.0, 0.0, 4.0);
    let (b, c2) = disc(&mut k, 1.4, 0.2, 2.5);
    k.tangent(c1, c2);
    k.drive(DimKind::Radius(c1), "4");
    k.drive(DimKind::Radius(c2), "2.5");
    assert!(k.dof() > 0);
    let d = k.measure(DimKind::Distance(a, b));
    k.check_value(d, 1.5);
}

/// Common external tangent: length sqrt(d^2 - (r1-r2)^2).
#[test]
fn common_external_tangent_length() {
    let mut k = Kit::new();
    let (a, c1) = disc(&mut k, 0.0, 0.0, 3.0);
    let (b, c2) = disc(&mut k, 7.0, 0.0, 2.0);
    let p = k.point(1.0, 2.9);
    let q = k.point(7.4, 1.9);
    k.on(p, c1);
    k.on(q, c2);
    let l = k.line(p, q);
    k.tangent(l, c1);
    k.tangent(l, c2);
    k.drive(DimKind::Radius(c1), "3");
    k.drive(DimKind::Radius(c2), "2");
    k.drive(DimKind::Distance(a, b), "7");
    assert!(k.dof() > 0);
    let len = k.measure(DimKind::Length(l));
    k.check_value(len, 48.0f64.sqrt());
}

/// Common internal (crossing) tangent: length sqrt(d^2 - (r1+r2)^2).
#[test]
fn common_internal_tangent_length() {
    let mut k = Kit::new();
    let (a, c1) = disc(&mut k, 0.0, 0.0, 3.0);
    let (b, c2) = disc(&mut k, 8.0, 0.0, 2.0);
    let p = k.point(1.0, 2.8);
    let q = k.point(7.3, -1.9);
    k.on(p, c1);
    k.on(q, c2);
    let l = k.line(p, q);
    k.tangent(l, c1);
    k.tangent(l, c2);
    k.drive(DimKind::Radius(c1), "3");
    k.drive(DimKind::Radius(c2), "2");
    k.drive(DimKind::Distance(a, b), "8");
    assert!(k.dof() > 0);
    let len = k.measure(DimKind::Length(l));
    k.check_value(len, 39.0f64.sqrt());
}

/// Tangent segment from a point sliding around a circle: sqrt(d^2 - r^2).
#[test]
fn tangent_segment_from_point() {
    let mut k = Kit::new();
    let o = k.origin();
    let c = k.circle(o, 2.0);
    let p = k.point(5.0, 0.3);
    k.drive(DimKind::Radius(c), "2");
    k.drive(DimKind::Distance(o, p), "5");
    let (seg, _t) = k.tangent_line(p, c, [1.0, 1.8]);
    assert!(k.dof() > 0);
    let len = k.measure(DimKind::Length(seg));
    k.check_value(len, 21.0f64.sqrt());
}

/// Two tangents from an external point: contact chord length 2 r t / d.
#[test]
fn two_tangents_chord_of_contact() {
    let mut k = Kit::new();
    let o = k.origin();
    let c = k.circle(o, 2.0);
    let p = k.point(5.0, 0.2);
    k.drive(DimKind::Radius(c), "2");
    k.drive(DimKind::Distance(o, p), "5");
    let (_s1, t1) = k.tangent_line(p, c, [0.8, 1.8]);
    let (_s2, t2) = k.tangent_line(p, c, [0.8, -1.8]);
    assert!(k.dof() > 0);
    let chord = k.measure(DimKind::Distance(t1, t2));
    k.check_value(chord, 2.0 * 2.0 * 21.0f64.sqrt() / 5.0);
}

/// Angle between the two tangent lines from a point: 2 asin(r/d).
#[test]
fn two_tangents_angle() {
    let mut k = Kit::new();
    let o = k.origin();
    let c = k.circle(o, 2.0);
    let p = k.point(5.0, 0.2);
    k.drive(DimKind::Radius(c), "2");
    k.drive(DimKind::Distance(o, p), "5");
    let (s1, _) = k.tangent_line(p, c, [0.8, 1.8]);
    let (s2, _) = k.tangent_line(p, c, [0.8, -1.8]);
    let ang = k.measure(DimKind::Angle(s1, s2));
    let a = 2.0 * (2.0f64 / 5.0).asin();
    let v = k.value(ang);
    let e = if (v - a).abs() < 1e-6 { a } else { PI - a };
    k.check_value(ang, e);
}

/// Three mutually tangent circles with radii 1, 2, 3: right angle at the
/// center of the smallest.
#[test]
fn three_mutually_tangent_right_angle() {
    let mut k = Kit::new();
    let (a, c1) = disc(&mut k, 0.0, 0.0, 1.0);
    let (b, c2) = disc(&mut k, 3.0, 0.1, 2.0);
    let (c, c3) = disc(&mut k, 0.1, 4.0, 3.0);
    k.tangent(c1, c2);
    k.tangent(c1, c3);
    k.tangent(c2, c3);
    k.drive(DimKind::Radius(c1), "1");
    k.drive(DimKind::Radius(c2), "2");
    k.drive(DimKind::Radius(c3), "3");
    assert_eq!(k.dof(), 3);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let ang = k.measure(DimKind::Angle(ab, ac));
    k.check_value(ang, PI / 2.0);
}

/// Radii 2s, 3s, 6s: the angles of the center triangle do not depend on s.
#[test]
fn scaled_chain_angles_invariant() {
    let mut k = Kit::new();
    let s = k.var("s", 1.0);
    let (a, c1) = disc(&mut k, 0.0, 0.0, 2.0);
    let (b, c2) = disc(&mut k, 5.0, 0.0, 3.0);
    let (c, c3) = disc(&mut k, 0.8, 8.5, 6.0);
    k.tangent(c1, c2);
    k.tangent(c1, c3);
    k.tangent(c2, c3);
    k.drive(DimKind::Radius(c1), "2s");
    k.drive(DimKind::Radius(c2), "3s");
    k.drive(DimKind::Radius(c3), "6s");
    assert!(k.dof() > 0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let ca = k.line(c, a);
    let cb = k.line(c, b);
    let at_a = k.measure(DimKind::Angle(ab, ac));
    let at_c = k.measure(DimKind::Angle(ca, cb));
    let ea = 0.1f64.acos();
    let ec = (5.0f64 / 6.0).acos();
    k.check_value(at_a, ea);
    k.check_value(at_c, PI - ec);
    k.set_var(s, 2.0);
    k.check_value(at_a, ea);
    k.check_value(at_c, PI - ec);
}

/// Soddy (Descartes) inner circle for radii 1, 2, 3 has radius 6/23.
#[test]
fn descartes_inner_circle() {
    let mut k = Kit::new();
    let (_, c1) = disc(&mut k, 0.0, 0.0, 1.0);
    let (_, c2) = disc(&mut k, 3.0, 0.0, 2.0);
    let (_, c3) = disc(&mut k, 0.0, 4.0, 3.0);
    let (_, c4) = disc(&mut k, 0.9, 0.87, 0.26);
    k.tangent(c1, c2);
    k.tangent(c1, c3);
    k.tangent(c2, c3);
    k.tangent(c4, c1);
    k.tangent(c4, c2);
    k.tangent(c4, c3);
    k.drive(DimKind::Radius(c1), "1");
    k.drive(DimKind::Radius(c2), "2");
    k.drive(DimKind::Radius(c3), "3");
    assert_eq!(k.dof(), 3);
    let r4 = k.measure(DimKind::Radius(c4));
    k.check_value(r4, 6.0 / 23.0);
}

/// Three equal unit-scale circles with a small circle in the gap, scaled by r.
#[test]
fn descartes_equal_circles_scaled() {
    let mut k = Kit::new();
    let r = k.var("r", 1.0);
    let h = 3.0f64.sqrt();
    let (_, c1) = disc(&mut k, 0.0, 0.0, 1.0);
    let (_, c2) = disc(&mut k, 2.0, 0.05, 1.0);
    let (_, c3) = disc(&mut k, 1.0, h, 1.0);
    let (_, c4) = disc(&mut k, 1.0, 0.6, 0.2);
    k.tangent(c1, c2);
    k.tangent(c1, c3);
    k.tangent(c2, c3);
    k.tangent(c4, c1);
    k.tangent(c4, c2);
    k.tangent(c4, c3);
    k.drive(DimKind::Radius(c1), "r");
    k.equal(c1, c2);
    k.equal(c2, c3);
    assert_eq!(k.dof(), 3);
    let r4 = k.measure(DimKind::Radius(c4));
    let f = (2.0 * h - 3.0) / 3.0;
    k.check_value(r4, f);
    k.set_var(r, 2.5);
    k.check_value(r4, 2.5 * f);
}

/// Outer Soddy circle: encloses three unit circles, radius 1 + 2/sqrt(3).
#[test]
fn descartes_enclosing_circle() {
    let mut k = Kit::new();
    let h = 3.0f64.sqrt();
    let (_, c1) = disc(&mut k, 0.0, 0.0, 1.0);
    let (_, c2) = disc(&mut k, 2.0, 0.05, 1.0);
    let (_, c3) = disc(&mut k, 1.0, h, 1.0);
    let (_, c4) = disc(&mut k, 1.0, h / 3.0, 2.2);
    k.tangent(c1, c2);
    k.tangent(c1, c3);
    k.tangent(c2, c3);
    k.tangent(c4, c1);
    k.tangent(c4, c2);
    k.tangent(c4, c3);
    k.drive(DimKind::Radius(c1), "1");
    k.drive(DimKind::Radius(c2), "1");
    k.drive(DimKind::Radius(c3), "1");
    assert_eq!(k.dof(), 3);
    let r4 = k.measure(DimKind::Radius(c4));
    k.check_value(r4, 1.0 + 2.0 / h);
}

/// Circle inscribed in an angle: |VC| = r / sin(theta/2); the wedge can still
/// lengthen.
#[test]
fn circle_in_angle() {
    let mut k = Kit::new();
    let o = k.origin();
    let xa = k.xaxis();
    let q = k.point(2.0, 3.5);
    let l2 = k.inf_line(o, q);
    k.drive(DimKind::Angle(xa, l2), "60");
    let (cc, c) = disc(&mut k, 2.5, 1.5, 1.5);
    k.tangent(c, xa);
    k.tangent(c, l2);
    k.drive(DimKind::Radius(c), "1.5");
    assert!(k.dof() > 0);
    let d = k.measure(DimKind::Distance(o, cc));
    k.check_value(d, 3.0);
}

/// Circle in the corner formed by both axes.
#[test]
fn circle_in_axes_corner() {
    let mut k = Kit::new();
    let (cc, c) = disc(&mut k, 2.1, 1.9, 2.0);
    let (xa, ya) = (k.xaxis(), k.yaxis());
    k.tangent(c, xa);
    k.tangent(c, ya);
    k.drive(DimKind::Radius(c), "2");
    assert_eq!(k.dof(), 0);
    let x = k.measure(DimKind::CoordX(cc));
    let y = k.measure(DimKind::CoordY(cc));
    k.check_value(x, 2.0);
    k.check_value(y, 2.0);
}

/// Two equal circles on a line, tangent to each other, and a small circle in
/// the gap: radius r/4. Both circles can slide along the line.
#[test]
fn gap_circle_between_two_on_line() {
    let mut k = Kit::new();
    let r = k.var("r", 4.0);
    let xa = k.xaxis();
    let (_, c1) = disc(&mut k, -4.0, 4.0, 4.0);
    let (_, c2) = disc(&mut k, 4.1, 4.0, 4.0);
    let (_, s) = disc(&mut k, 0.0, 1.0, 1.0);
    k.tangent(c1, xa);
    k.tangent(c2, xa);
    k.tangent(c1, c2);
    k.tangent(s, c1);
    k.tangent(s, c2);
    k.tangent(s, xa);
    k.drive(DimKind::Radius(c1), "r");
    k.equal(c1, c2);
    assert!(k.dof() > 0);
    let rs = k.measure(DimKind::Radius(s));
    k.check_value(rs, 1.0);
    k.set_var(r, 6.0);
    k.check_value(rs, 1.5);
}

/// Circle tangent to a line at the origin and through a point: r = 5.
#[test]
fn circle_tangent_at_point_through_point() {
    let mut k = Kit::new();
    let o = k.origin();
    let xa = k.xaxis();
    let (_, c) = disc(&mut k, 0.1, 5.1, 5.0);
    let p = k.point(4.1, 1.9);
    k.tangent(c, xa);
    k.on(o, c);
    k.on(p, c);
    k.drive(DimKind::CoordX(p), "4");
    k.drive(DimKind::CoordY(p), "2");
    assert_eq!(k.dof(), 0);
    let r = k.measure(DimKind::Radius(c));
    k.check_value(r, 5.0);
}

/// Circle of fixed radius through a point at height h, tangent to the axis:
/// horizontal offset sqrt(2rh - h^2). The point slides horizontally.
#[test]
fn circle_through_point_tangent_to_line() {
    let mut k = Kit::new();
    let xa = k.xaxis();
    let (cc, c) = disc(&mut k, 2.0, 2.5, 2.5);
    let p = k.point(0.1, 1.0);
    k.tangent(c, xa);
    k.on(p, c);
    k.drive(DimKind::Radius(c), "2.5");
    k.drive(DimKind::CoordY(p), "1");
    assert!(k.dof() > 0);
    let dx = k.measure(DimKind::HDist(p, cc));
    k.check_value(dx, 2.0);
}

/// Fillet of radius r in a right-angle corner: chord between contact points.
#[test]
fn fillet_arc_in_right_corner() {
    let mut k = Kit::new();
    let (xa, ya) = (k.xaxis(), k.yaxis());
    let c = k.point(2.0, 2.0);
    let b = k.point(0.0, 2.0);
    let a = k.point(2.0, 0.0);
    let arc = k.arc(c, b, a);
    k.tangent(arc, xa);
    k.tangent(arc, ya);
    k.on(a, xa);
    k.on(b, ya);
    k.drive(DimKind::Radius(arc), "2");
    assert_eq!(k.dof(), 0);
    let chord = k.measure(DimKind::Distance(a, b));
    k.check_value(chord, 2.0 * 2.0f64.sqrt());
}

/// Fillet in a 120 degree corner: tangent length r / tan(60 deg).
#[test]
fn fillet_in_obtuse_corner() {
    let mut k = Kit::new();
    let o = k.origin();
    let xa = k.xaxis();
    let q = k.point(-1.0, 1.7);
    let l2 = k.inf_line(o, q);
    k.drive(DimKind::Angle(xa, l2), "120");
    let (_, c) = disc(&mut k, 0.87, 1.5, 1.5);
    k.tangent(c, xa);
    k.tangent(c, l2);
    k.drive(DimKind::Radius(c), "1.5");
    let t = k.point(0.87, 0.05);
    k.on(t, xa);
    k.on(t, c);
    assert!(k.dof() > 0);
    let len = k.measure(DimKind::Distance(o, t));
    k.check_value(len, 1.5 / 3.0f64.sqrt());
}

/// Two equal circles filling a box of width W: r = W / 4. The box can still
/// change height at the right wall.
#[test]
fn two_circles_packed_in_box() {
    let mut k = Kit::new();
    let w = k.var("W", 10.0);
    let (xa, ya) = (k.xaxis(), k.yaxis());
    let r1 = k.point(10.0, 4.0);
    let r2 = k.point(10.0, -2.0);
    let wall = k.inf_line(r1, r2);
    k.vertical(wall);
    k.drive(DimKind::CoordX(r1), "W");
    let (_, a) = disc(&mut k, 2.4, 2.6, 2.5);
    let (_, b) = disc(&mut k, 7.6, 2.6, 2.5);
    k.tangent(a, xa);
    k.tangent(a, ya);
    k.tangent(b, xa);
    k.tangent(b, wall);
    k.tangent(a, b);
    k.equal(a, b);
    assert!(k.dof() > 0);
    let ra = k.measure(DimKind::Radius(a));
    k.check_value(ra, 2.5);
    k.set_var(w, 12.0);
    k.check_value(ra, 3.0);
}

/// Belt around two pulleys: both straight runs have length
/// sqrt(d^2 - (R1-R2)^2) and the wrap angle on the big pulley is 2 acos((R1-R2)/d).
#[test]
fn belt_straight_runs_and_wrap() {
    let mut k = Kit::new();
    let (a, c1) = disc(&mut k, 0.0, 0.0, 3.0);
    let (b, c2) = disc(&mut k, 7.0, 0.0, 2.0);
    k.drive(DimKind::Radius(c1), "3");
    k.drive(DimKind::Radius(c2), "2");
    k.drive(DimKind::Distance(a, b), "7");
    let p = k.point(0.4, 2.9);
    let q = k.point(7.3, 1.9);
    let p2 = k.point(0.4, -2.9);
    let q2 = k.point(7.3, -1.9);
    k.on(p, c1);
    k.on(q, c2);
    k.on(p2, c1);
    k.on(q2, c2);
    let top = k.line(p, q);
    let bot = k.line(p2, q2);
    k.tangent(top, c1);
    k.tangent(top, c2);
    k.tangent(bot, c1);
    k.tangent(bot, c2);
    assert!(k.dof() > 0);
    let lt = k.measure(DimKind::Length(top));
    let lb = k.measure(DimKind::Length(bot));
    let e = 48.0f64.sqrt();
    k.check_value(lt, e);
    k.check_value(lb, e);
    let ap = k.line(a, p);
    let ap2 = k.line(a, p2);
    let wrap = k.measure(DimKind::Angle(ap, ap2));
    let beta = (1.0f64 / 7.0).acos();
    let v = k.value(wrap);
    let ex = if (v - 2.0 * beta).abs() < 1e-6 { 2.0 * beta } else { 2.0 * PI - 2.0 * beta };
    k.check_value(wrap, ex);
}

/// A rolling circle between the axis and a big circle: horizontal distance
/// between centers is 2 sqrt(R r) wherever the pair sits on the line.
#[test]
fn rolling_circle_horizontal_offset() {
    let mut k = Kit::new();
    let xa = k.xaxis();
    let (_, big) = disc(&mut k, 0.0, 4.0, 4.0);
    let (_, small) = disc(&mut k, 4.1, 1.0, 1.0);
    k.tangent(big, xa);
    k.tangent(small, xa);
    k.tangent(big, small);
    k.drive(DimKind::Radius(big), "4");
    k.drive(DimKind::Radius(small), "1");
    assert_eq!(k.dof(), 1);
    let (cb, cs) = (big_center(&k, big), big_center(&k, small));
    let dx = k.measure(DimKind::HDist(cb, cs));
    k.check_value(dx, 4.0);
}

fn big_center(k: &Kit, circ: usize) -> usize {
    match k.doc().kind(circ) {
        Some(sggeo::model::Ent::Circle { c, .. }) => *c,
        _ => panic!("not a circle"),
    }
}

/// The common tangent at the contact point of two tangent circles is
/// perpendicular to the line of centers.
#[test]
fn contact_tangent_perpendicular_to_centers() {
    let mut k = Kit::new();
    let (a, c1) = disc(&mut k, 0.0, 0.0, 2.0);
    let (b, c2) = disc(&mut k, 5.0, 0.2, 3.0);
    k.tangent(c1, c2);
    k.drive(DimKind::Radius(c1), "2");
    k.drive(DimKind::Radius(c2), "3");
    let t = k.point(2.0, 0.1);
    k.on(t, c1);
    k.on(t, c2);
    let u = k.point(2.1, 3.0);
    let l = k.inf_line(t, u);
    k.tangent(l, c1);
    let ab = k.line(a, b);
    assert!(k.dof() > 0);
    let ang = k.measure(DimKind::Angle(l, ab));
    k.check_value(ang, PI / 2.0);
}

/// Chain of circles in a 60 degree wedge, each tangent to both lines and the
/// previous circle: radii shrink by 1/3.
#[test]
fn wedge_chain_of_three() {
    let mut k = Kit::new();
    let o = k.origin();
    let xa = k.xaxis();
    let q = k.point(10.0, 17.0);
    let l2 = k.inf_line(o, q);
    k.drive(DimKind::Angle(xa, l2), "60");
    let (_, c1) = disc(&mut k, 15.6, 9.0, 9.0);
    let (_, c2) = disc(&mut k, 5.2, 3.0, 3.0);
    let (_, c3) = disc(&mut k, 1.7, 1.0, 1.0);
    for c in [c1, c2, c3] {
        k.tangent(c, xa);
        k.tangent(c, l2);
    }
    k.tangent(c1, c2);
    k.tangent(c2, c3);
    k.drive(DimKind::Radius(c1), "9");
    assert!(k.dof() > 0);
    let r2 = k.measure(DimKind::Radius(c2));
    let r3 = k.measure(DimKind::Radius(c3));
    k.check_value(r2, 3.0);
    k.check_value(r3, 1.0);
}

/// Radii a and 2a, externally tangent: distance 3a, for several a.
#[test]
fn variable_radii_pair() {
    let mut k = Kit::new();
    let a = k.var("a", 2.0);
    let (p, c1) = disc(&mut k, 0.0, 0.0, 2.0);
    let (q, c2) = disc(&mut k, 6.2, 0.3, 4.0);
    k.tangent(c1, c2);
    k.drive(DimKind::Radius(c1), "a");
    k.drive(DimKind::Radius(c2), "2a");
    assert!(k.dof() > 0);
    let d = k.measure(DimKind::Distance(p, q));
    k.check_value(d, 6.0);
    k.set_var(a, 3.0);
    k.check_value(d, 9.0);
}

/// Incircle (r = 1) and the excircle opposite the right angle (r = 6) of a
/// floating 3-4-5 triangle.
#[test]
fn incircle_and_excircle() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(4.0, 0.1);
    let c = k.point(0.1, 3.0);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let ca = k.line(c, a);
    k.drive(DimKind::Length(ab), "4");
    k.drive(DimKind::Length(ca), "3");
    k.drive(DimKind::Length(bc), "5");
    let (_, inc) = disc(&mut k, 1.0, 1.0, 1.0);
    let (_, exc) = disc(&mut k, 5.8, 6.1, 6.0);
    for s in [ab, bc, ca] {
        k.tangent(inc, s);
        k.tangent(exc, s);
    }
    assert_eq!(k.dof(), 3);
    let ri = k.measure(DimKind::Radius(inc));
    let re = k.measure(DimKind::Radius(exc));
    k.check_value(ri, 1.0);
    k.check_value(re, 6.0);
}

/// Circle between two parallel lines, tangent to a circle resting on one of
/// them: horizontal offset 2 sqrt(2) (strip height 4).
#[test]
fn strip_circle_tangent_to_resting_circle() {
    let mut k = Kit::new();
    let xa = k.xaxis();
    let h1 = k.point(0.0, 4.0);
    let h2 = k.point(3.0, 4.0);
    let top = k.inf_line(h1, h2);
    k.horizontal(top);
    k.drive(DimKind::CoordY(h1), "4");
    let (f, cf) = disc(&mut k, 0.0, 1.0, 1.0);
    let (g, cg) = disc(&mut k, 3.0, 2.0, 2.0);
    k.tangent(cf, xa);
    k.drive(DimKind::Radius(cf), "1");
    k.tangent(cg, xa);
    k.tangent(cg, top);
    k.tangent(cf, cg);
    assert!(k.dof() > 0);
    let dx = k.measure(DimKind::HDist(f, g));
    k.check_value(dx, 8.0f64.sqrt());
}

/// Archimedes' circle in the arbelos: r = r1 r2 (r1+r2) / (r1^2 + r1 r2 + r2^2).
#[test]
fn arbelos_incircle() {
    let mut k = Kit::new();
    let xa = k.xaxis();
    let o = k.point(0.0, 0.0);
    k.on(o, xa);
    let big = k.circle(o, 5.0);
    let p1 = k.point(-2.0, 0.0);
    let p2 = k.point(3.0, 0.0);
    k.on(p1, xa);
    k.on(p2, xa);
    let c1 = k.circle(p1, 3.0);
    let c2 = k.circle(p2, 2.0);
    k.tangent(big, c1);
    k.tangent(big, c2);
    k.tangent(c1, c2);
    k.drive(DimKind::Radius(c1), "3");
    k.drive(DimKind::Radius(c2), "2");
    let (_, s) = disc(&mut k, 1.3, 3.1, 1.5);
    k.tangent(s, big);
    k.tangent(s, c1);
    k.tangent(s, c2);
    assert!(k.dof() > 0);
    let rs = k.measure(DimKind::Radius(s));
    k.check_value(rs, 30.0 / 19.0);
}

/// Length of the common external tangent segment between contact points of
/// two tangent circles: 2 sqrt(r1 r2); the pair floats freely.
#[test]
fn tangent_circles_outer_tangent_segment() {
    let mut k = Kit::new();
    let (_, c1) = disc(&mut k, 0.0, 0.0, 3.0);
    let (_, c2) = disc(&mut k, 5.1, 0.2, 2.0);
    k.tangent(c1, c2);
    k.drive(DimKind::Radius(c1), "3");
    k.drive(DimKind::Radius(c2), "2");
    let p = k.point(1.5, 2.6);
    let q = k.point(5.5, 2.0);
    k.on(p, c1);
    k.on(q, c2);
    let l = k.line(p, q);
    k.tangent(l, c1);
    k.tangent(l, c2);
    assert_eq!(k.dof(), 3);
    let len = k.measure(DimKind::Length(l));
    k.check_value(len, 2.0 * 6.0f64.sqrt());
}

/// A ring of four equal circles around a unit circle, each touching its
/// neighbours: r = 1 + sqrt 2.
#[test]
fn ring_of_four_around_unit_circle() {
    let mut k = Kit::new();
    let (_, mid) = disc(&mut k, 0.1, 0.0, 1.0);
    let (_, a) = disc(&mut k, 3.4, 0.0, 2.4);
    let (_, b) = disc(&mut k, 0.0, 3.4, 2.4);
    let (_, c) = disc(&mut k, -3.4, 0.0, 2.4);
    let (_, d) = disc(&mut k, 0.0, -3.4, 2.4);
    for x in [a, b, c, d] {
        k.tangent(mid, x);
    }
    k.tangent(a, b);
    k.tangent(b, c);
    k.tangent(c, d);
    k.tangent(d, a);
    k.equal(a, b);
    k.equal(b, c);
    k.equal(c, d);
    k.drive(DimKind::Radius(mid), "1");
    assert_eq!(k.dof(), 3);
    let r = k.measure(DimKind::Radius(a));
    k.check_value(r, 1.0 + 2.0f64.sqrt());
}

/// Apollonius' problem: circle touching three given circles externally.
#[test]
fn apollonius_three_circles() {
    let mut k = Kit::new();
    let (p1, c1) = disc(&mut k, 0.0, 0.0, 1.0);
    let (p2, c2) = disc(&mut k, 10.0, 0.0, 2.0);
    let (p3, c3) = disc(&mut k, 3.0, 8.0, 3.0);
    k.drive(DimKind::Radius(c1), "1");
    k.drive(DimKind::Radius(c2), "2");
    k.drive(DimKind::Radius(c3), "3");
    k.drive(DimKind::Distance(p1, p2), "10");
    k.drive(DimKind::Distance(p1, p3), "sqrt(73)");
    k.drive(DimKind::Distance(p2, p3), "sqrt(113)");
    let (_, s) = disc(&mut k, 4.48, 1.46, 3.7);
    k.tangent(s, c1);
    k.tangent(s, c2);
    k.tangent(s, c3);
    assert_eq!(k.dof(), 3);
    let r = k.measure(DimKind::Radius(s));
    k.check_value(r, 3.70958454);
}

/// Point with both coordinates driven.
fn placed(k: &mut Kit, x: f64, y: f64) -> usize {
    let p = k.point(x + 0.01, y - 0.01);
    k.drive(DimKind::CoordX(p), &format!("{x}"));
    k.drive(DimKind::CoordY(p), &format!("{y}"));
    p
}

/// Circle touching a line and two circles (all externally).
#[test]
fn circle_tangent_line_and_two_circles() {
    let mut k = Kit::new();
    let xa = k.xaxis();
    let p1 = placed(&mut k, 0.0, 3.0);
    let c1 = k.circle(p1, 1.0);
    let p2 = placed(&mut k, 6.0, 2.0);
    let c2 = k.circle(p2, 1.5);
    k.drive(DimKind::Radius(c1), "1");
    k.drive(DimKind::Radius(c2), "1.5");
    let (_, s) = disc(&mut k, 2.63, 1.87, 1.87);
    k.tangent(s, xa);
    k.tangent(s, c1);
    k.tangent(s, c2);
    assert_eq!(k.dof(), 0);
    let r = k.measure(DimKind::Radius(s));
    k.check_value(r, 1.86778133);
}

/// Malfatti circles in the 3-4-5 right triangle (closed form by Malfatti's
/// formulas with half-angle tangents): the radius at the right angle.
#[test]
#[ignore = "an octic that is a tower of quadratics is not solved (SHORTCOMINGS 2.6)"]
fn malfatti_circles() {
    let mut k = Kit::new();
    let o = k.origin();
    let (xa, ya) = (k.xaxis(), k.yaxis());
    let b = k.point(4.0, 0.0);
    let c = k.point(0.0, 3.0);
    k.on(b, xa);
    k.on(c, ya);
    k.drive(DimKind::Distance(o, b), "4");
    k.drive(DimKind::Distance(o, c), "3");
    let hyp = k.inf_line(b, c);
    let (_, m0) = disc(&mut k, 0.508, 0.508, 0.508);
    let (_, m1) = disc(&mut k, 1.744, 0.752, 0.752);
    let (_, m2) = disc(&mut k, 0.665, 1.670, 0.665);
    k.tangent(m0, xa);
    k.tangent(m0, ya);
    k.tangent(m1, xa);
    k.tangent(m1, hyp);
    k.tangent(m2, ya);
    k.tangent(m2, hyp);
    k.tangent(m0, m1);
    k.tangent(m1, m2);
    k.tangent(m2, m0);
    assert_eq!(k.dof(), 0);
    let r = k.measure(DimKind::Radius(m0));
    k.check_value(r, 0.5079339623521082);
}

/// Second circle of the Pappus chain in the arbelos with radii 5 = 3 + 2.
#[test]
fn pappus_chain_second_circle() {
    let mut k = Kit::new();
    let xa = k.xaxis();
    let o = k.point(0.0, 0.0);
    k.on(o, xa);
    let big = k.circle(o, 5.0);
    let p1 = k.point(-2.0, 0.0);
    let p2 = k.point(3.0, 0.0);
    k.on(p1, xa);
    k.on(p2, xa);
    let c1 = k.circle(p1, 3.0);
    let c2 = k.circle(p2, 2.0);
    k.tangent(big, c1);
    k.tangent(big, c2);
    k.tangent(c1, c2);
    k.drive(DimKind::Radius(c1), "3");
    k.drive(DimKind::Radius(c2), "2");
    let (_, s1) = disc(&mut k, 1.316, 3.158, 1.579);
    k.tangent(s1, big);
    k.tangent(s1, c1);
    k.tangent(s1, c2);
    let (_, s2) = disc(&mut k, -1.129, 3.871, 0.968);
    k.tangent(s2, big);
    k.tangent(s2, c1);
    k.tangent(s2, s1);
    assert!(k.dof() > 0);
    let r = k.measure(DimKind::Radius(s2));
    k.check_value(r, 30.0 / 31.0);
}

/// Outer Soddy circle of three touching circles with radii 2, 3, 4:
/// curvature 1/2 + 1/3 + 1/4 - 2 sqrt(1/6 + 1/12 + 1/8).
#[test]
fn soddy_outer_general() {
    let mut k = Kit::new();
    // 2-3-4 radii: centre distances 5, 6, 7
    let (_, c1) = disc(&mut k, 0.0, 0.0, 2.0);
    let (_, c2) = disc(&mut k, 5.0, 0.05, 3.0);
    let (_, c3) = disc(&mut k, 1.0, 5.9, 4.0);
    let kk = 1.0 / 2.0 + 1.0 / 3.0 + 1.0 / 4.0 - 2.0 * (1.0f64 / 6.0 + 1.0 / 12.0 + 1.0 / 8.0).sqrt();
    let r4 = -1.0 / kk;
    let (_, c4) = disc(&mut k, 2.0, 2.0, r4);
    k.tangent(c1, c2);
    k.tangent(c1, c3);
    k.tangent(c2, c3);
    k.tangent(c4, c1);
    k.tangent(c4, c2);
    k.tangent(c4, c3);
    k.drive(DimKind::Radius(c1), "2");
    k.drive(DimKind::Radius(c2), "3");
    k.drive(DimKind::Radius(c3), "4");
    assert_eq!(k.dof(), 3);
    let r = k.measure(DimKind::Radius(c4));
    k.check_value(r, r4);
}

/// Circle through two points touching a circle (Apollonius PPC).
#[test]
fn apollonius_ppc() {
    let mut k = Kit::new();
    let p = placed(&mut k, 1.0, 0.0);
    let q = placed(&mut k, 3.0, 1.0);
    let ctr = placed(&mut k, 5.0, 4.0);
    let circ = k.circle(ctr, 1.5);
    k.drive(DimKind::Radius(circ), "1.5");
    let (_, s) = disc(&mut k, 0.84, 2.82, 2.82);
    k.on(p, s);
    k.on(q, s);
    k.tangent(s, circ);
    assert_eq!(k.dof(), 0);
    let r = k.measure(DimKind::Radius(s));
    k.check_value(r, 2.82401243);
}

/// Circle in a 70 degree wedge touching a circle (Apollonius LLC).
#[test]
fn apollonius_llc() {
    let mut k = Kit::new();
    let o = k.origin();
    let xa = k.xaxis();
    let q = k.point(1.0, 2.75);
    let l2 = k.inf_line(o, q);
    k.drive_angle(xa, l2, "70");
    let ctr = placed(&mut k, 4.0, 1.5);
    let circ = k.circle(ctr, 1.0);
    k.drive(DimKind::Radius(circ), "1");
    let (_, s) = disc(&mut k, 1.77, 1.24, 1.24);
    k.tangent(s, xa);
    k.tangent(s, l2);
    k.tangent(s, circ);
    assert_eq!(k.dof(), 1);
    let r = k.measure(DimKind::Radius(s));
    k.check_value(r, 1.24166066);
}

/// Malfatti circles in an equilateral triangle of side a: r = (sqrt 3 - 1) a / 4.
#[test]
fn malfatti_equilateral() {
    let mut k = Kit::new();
    let o = k.origin();
    let xa = k.xaxis();
    let b = k.point(4.0, 0.0);
    k.on(b, xa);
    let c = k.point(2.0, 3.46);
    let ab = k.line(o, b);
    let bc = k.inf_line(b, c);
    let ca = k.inf_line(c, o);
    k.drive(DimKind::Length(ab), "4");
    k.equal(ab, bc);
    k.equal(bc, ca);
    let r = 3f64.sqrt() - 1.0;
    let rr = r * 3f64.sqrt();
    let (_, m0) = disc(&mut k, rr, r, r);
    let (_, m1) = disc(&mut k, 4.0 - rr, r, r);
    let (_, m2) = disc(&mut k, 2.0, 3.46 - 2.0 * r, r);
    k.tangent(m0, xa);
    k.tangent(m0, ca);
    k.tangent(m1, xa);
    k.tangent(m1, bc);
    k.tangent(m2, bc);
    k.tangent(m2, ca);
    k.tangent(m0, m1);
    k.tangent(m1, m2);
    k.tangent(m2, m0);
    assert_eq!(k.dof(), 0);
    let rad = k.measure(DimKind::Radius(m0));
    k.check_value(rad, r);
}

/// Circle tangent to the x-axis, through a point and touching a circle (LCP).
#[test]
fn apollonius_lcp() {
    let mut k = Kit::new();
    let xa = k.xaxis();
    let p = placed(&mut k, 1.0, 3.0);
    let ctr = placed(&mut k, 5.0, 2.0);
    let circ = k.circle(ctr, 1.0);
    k.drive(DimKind::Radius(circ), "1");
    let (_, s) = disc(&mut k, 2.25, 1.76, 1.76);
    k.tangent(s, xa);
    k.on(p, s);
    k.tangent(s, circ);
    assert_eq!(k.dof(), 0);
    let r = k.measure(DimKind::Radius(s));
    k.check_value(r, 1.76041667);
}

/// Circle through a point touching two circles (CCP).
#[test]
fn apollonius_ccp() {
    let mut k = Kit::new();
    let p = placed(&mut k, 0.0, 4.0);
    let c1 = placed(&mut k, 0.0, 0.0);
    let c2 = placed(&mut k, 6.0, 0.0);
    let k1 = k.circle(c1, 1.0);
    let k2 = k.circle(c2, 2.0);
    k.drive(DimKind::Radius(k1), "1");
    k.drive(DimKind::Radius(k2), "2");
    let (_, s) = disc(&mut k, 2.314, 2.779, 2.616);
    k.on(p, s);
    k.tangent(s, k1);
    k.tangent(s, k2);
    assert_eq!(k.dof(), 0);
    let r = k.measure(DimKind::Radius(s));
    k.check_value(r, 2.61629951);
}

/// LCP as above with P moved off the special height y_P = y_C + r (where
/// the quadratic's leading coefficient vanishes).
#[test]
fn apollonius_lcp_generic() {
    let mut k = Kit::new();
    let xa = k.xaxis();
    let p = placed(&mut k, 1.0, 3.3);
    let ctr = placed(&mut k, 5.0, 2.0);
    let circ = k.circle(ctr, 1.0);
    k.drive(DimKind::Radius(circ), "1");
    let (_, s) = disc(&mut k, 2.3, 1.8, 1.8);
    k.tangent(s, xa);
    k.on(p, s);
    k.tangent(s, circ);
    assert_eq!(k.dof(), 0);
    let r = k.measure(DimKind::Radius(s));
    k.check(r);
}

/// Steiner chain of three circles between a circle of radius R = 10 and an
/// inner circle of radius r = 0.5: the offset of the centres is
/// d = sqrt((R - r)^2 - 4 R r tan^2(pi/3)) = 5.5.
#[test]
fn steiner_chain_three() {
    let mut k = Kit::new();
    let o = k.origin();
    let outer = k.circle(o, 10.0);
    k.drive(DimKind::Radius(outer), "10");
    let (ic, inner) = disc(&mut k, 5.5, 0.05, 0.5);
    k.drive(DimKind::Radius(inner), "0.5");
    let (_, s0) = disc(&mut k, -2.5, 0.0, 7.5);
    let (_, s1) = disc(&mut k, 7.14, 2.45, 2.45);
    let (_, s2) = disc(&mut k, 7.14, -2.45, 2.45);
    for &s in &[s0, s1, s2] {
        k.tangent(s, outer);
        k.tangent(s, inner);
    }
    k.tangent(s0, s1);
    k.tangent(s1, s2);
    k.tangent(s2, s0);
    assert_eq!(k.dof(), 2);
    let d = k.measure(DimKind::Distance(o, ic));
    k.check_value(d, 5.5);
}

/// Sangaku: square of side a with quarter circles of radius a about two
/// adjacent corners; the circle touching both arcs and the base: r = 3a/8.
#[test]
fn sangaku_quarter_circles() {
    let mut k = Kit::new();
    let o = k.origin();
    let xa = k.xaxis();
    let b = k.point(4.0, 0.0);
    k.on(b, xa);
    let base = k.line(o, b);
    k.drive(DimKind::Length(base), "4");
    let q1 = k.circle(o, 4.0);
    let q2 = k.circle(b, 4.0);
    k.equal(q1, base);
    k.equal(q2, base);
    let (_, s) = disc(&mut k, 2.0, 1.52, 1.48);
    k.tangent(s, xa);
    k.tangent(s, q1);
    k.tangent(s, q2);
    assert_eq!(k.dof(), 0);
    let r = k.measure(DimKind::Radius(s));
    k.check_value(r, 1.5);
}

/// Apollonius with mixed tangency: enclosing two circles, touching the
/// third from outside.
#[test]
fn apollonius_mixed_tangency() {
    let mut k = Kit::new();
    let (p1, c1) = disc(&mut k, 0.0, 0.0, 1.0);
    let (p2, c2) = disc(&mut k, 10.0, 0.0, 2.0);
    let (p3, c3) = disc(&mut k, 3.0, 8.0, 3.0);
    k.drive(DimKind::Radius(c1), "1");
    k.drive(DimKind::Radius(c2), "2");
    k.drive(DimKind::Radius(c3), "3");
    k.drive(DimKind::Distance(p1, p2), "10");
    k.drive(DimKind::Distance(p1, p3), "8.5");
    k.drive(DimKind::Distance(p2, p3), "10.6");
    let (_, s) = disc(&mut k, 5.52, -1.35, 6.68);
    k.tangent(s, c1);
    k.tangent(s, c2);
    k.tangent(s, c3);
    assert_eq!(k.dof(), 3);
    let r = k.measure(DimKind::Radius(s));
    k.check(r);
}

/// Circle inscribed in a circular sector of radius R and angle 70 degrees:
/// r = R sin(t/2) / (1 + sin(t/2)).
#[test]
fn circle_in_sector() {
    let mut k = Kit::new();
    let o = k.origin();
    let xa = k.xaxis();
    let a = k.point(3.0, 0.0);
    k.on(a, xa);
    let t = 70f64.to_radians();
    let b = k.point(3.0 * t.cos(), 3.0 * t.sin());
    let arc = k.arc(o, a, b);
    let oa = k.line(o, a);
    let ob = k.line(o, b);
    k.drive(DimKind::Length(oa), "3");
    k.drive_angle(oa, ob, "70");
    let rr = 1.0935149188197835;
    let dc = 3.0 - rr;
    let (_, s) = disc(&mut k, dc * (t / 2.0).cos(), dc * (t / 2.0).sin(), rr);
    k.tangent(s, oa);
    k.tangent(s, ob);
    k.tangent(s, arc);
    assert_eq!(k.dof(), 0);
    let r = k.measure(DimKind::Radius(s));
    k.check_value(r, rr);
}

/// Incircle via tangency to three lines of a 3-4-5 triangle with a
/// variable scale: r = (a+b−c)/2.
#[test]
fn incircle_tangent_three_lines() {
    let mut k = Kit::new();
    let a = k.origin();
    let b = k.point(4.0, 0.1);
    let c = k.point(0.1, 3.0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.horizontal(ab);
    k.perpendicular(ab, ac);
    k.drive(DimKind::Length(ab), "4");
    k.drive(DimKind::Length(ac), "3");
    let i = k.point(1.0, 1.0);
    let circ = k.circle(i, 1.0);
    k.hint = [1.0, 0.0];
    k.tangent(circ, ab);
    k.hint = [0.0, 1.0];
    k.tangent(circ, ac);
    k.hint = [1.6, 1.8];
    k.tangent(circ, bc);
    let r = k.measure(DimKind::Radius(circ));
    k.check_value(r, 1.0);
}

/// Tangent from P = (r, y_P) to a circle touches at (r, 0) for every y_P.
/// The formula for the contact's y must still hold once x_P ≠ r and y_P moves.
#[test]
fn circle_tangent_contact_on_vertical_tangent() {
    let mut k = Kit::new();
    let o = k.origin();
    let c = k.circle(o, 3.0);
    let dr = k.drive(DimKind::Radius(c), "3");
    let p = k.point(3.0, 4.0);
    let dx = k.drive(DimKind::CoordX(p), "3");
    let dy = k.drive(DimKind::CoordY(p), "4");
    let (_seg, t) = k.tangent_line(p, c, [3.0, 0.0]);
    let ty = k.measure(DimKind::CoordY(t));
    let txt = k.check(ty);
    println!("{txt}");
    let f = k.formula(ty);
    k.set(dx, "3.5");
    println!("t = {:?}", k.doc().pval(t));
    k.still(ty, &f, "x_P = 3.5");
    k.set(dy, "6");
    println!("t = {:?}", k.doc().pval(t));
    k.still(ty, &f, "x_P = 3.5, y_P = 6");
    k.set(dr, "2");
    k.still(ty, &f, "x_P = 3.5, y_P = 6, r = 2");
}

/// Circle tangent to a line and to two circles (one Apollonius case).
#[test]
fn circle_tangent_two_circles_and_line() {
    let mut k = Kit::new();
    k.allow_inexact = true;
    let a = k.origin();
    let b = k.point(6.0, 0.1);
    let ab = k.line(a, b);
    k.horizontal(ab);
    k.drive(DimKind::Length(ab), "6");
    let c1 = k.circle(a, 2.0);
    let c2 = k.circle(b, 1.0);
    k.drive(DimKind::Radius(c1), "2");
    k.drive(DimKind::Radius(c2), "1");
    let p = k.point(-3.0, -3.0);
    let q = k.point(9.0, -3.1);
    let l = k.inf_line(p, q);
    k.horizontal(l);
    k.drive(DimKind::CoordY(p), "-3");
    k.drive(DimKind::CoordX(p), "-3");
    let x = k.point(3.5, -1.0);
    let c3 = k.circle(x, 2.0);
    k.hint = [3.5, -3.0];
    k.tangent(c3, l);
    k.hint = [1.8, -0.9];
    k.tangent(c3, c1);
    k.hint = [5.2, -0.5];
    k.tangent(c3, c2);
    println!("x = {:?}, r = {}", k.doc().pval(x), {
        let r = k.measure(DimKind::Radius(c3));
        k.value(r)
    });
    let r = k.doc().dims.len() - 1;
    let t0 = std::time::Instant::now();
    let t = k.check(r);
    println!("{} chars {:.0} ms", t.len(), t0.elapsed().as_secs_f64() * 1e3);
}
