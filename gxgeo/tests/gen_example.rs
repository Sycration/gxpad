//! Example tests showing how to use the test kit. Each `tests/gen_*.rs` file
//! is its own test crate: run one with `cargo test --test gen_example`.

use gxgeo::model::{Con, DimKind};
mod testkit;
use testkit::Kit;
use std::f64::consts::PI;

/// Fully constrained triangle on the origin: an angle from three sides.
#[test]
fn law_of_cosines() {
    let mut k = Kit::new();
    let o = k.origin();
    let b = k.point(4.0, 0.2);
    let c = k.point(1.0, 3.0);
    let ab = k.line(o, b);
    let bc = k.line(b, c);
    let ca = k.line(c, o);
    k.horizontal(ab);
    k.drive(DimKind::Length(ab), "5");
    k.drive(DimKind::Length(bc), "4");
    k.drive(DimKind::Length(ca), "3");
    let ang = k.measure(DimKind::Angle(ab, ca));
    // the angle at the origin of a 3-4-5 triangle
    k.check_value(ang, (3.0f64 / 5.0).acos().min(PI - (3.0f64 / 5.0).acos()).max((3.0f64 / 5.0).acos()));
}

/// Under-constrained: a free-floating triangle (3 rigid DOF) still has a
/// fully determined angle. Articulation drags the free points.
#[test]
fn floating_triangle_angle() {
    let mut k = Kit::new();
    let a = k.point(-1.0, -1.0);
    let b = k.point(3.0, -0.5);
    let c = k.point(0.5, 2.0);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let ca = k.line(c, a);
    k.drive(DimKind::Length(ab), "4");
    k.drive(DimKind::Length(bc), "3.5");
    k.drive(DimKind::Length(ca), "3");
    assert_eq!(k.dof(), 3);
    let ang = k.measure(DimKind::Angle(bc, ab));
    k.check(ang);
}

/// Thales: the angle at a point sliding on a circle over a diameter is 90°.
#[test]
fn thales_right_angle() {
    let mut k = Kit::new();
    let o = k.origin();
    let circ = k.circle(o, 2.0);
    let a = k.point(-2.0, 0.0);
    let b = k.point(2.0, 0.0);
    let p = k.point(0.7, 1.8);
    k.on(a, circ);
    k.on(b, circ);
    k.on(p, circ);
    let diam = k.line(a, b);
    k.on(o, diam);
    let pa = k.line(p, a);
    let pb = k.line(p, b);
    k.drive(DimKind::Radius(circ), "2");
    let ang = k.measure(DimKind::Angle(pa, pb));
    k.check_value(ang, PI / 2.0);
}

/// Curve, circle and tangent: |BC| = π/2 + √(π² − 1).
#[test]
fn sine_peak_tangent() {
    let mut k = Kit::new();
    let o = k.origin();
    let f = k.curve("y = sin(x)");
    let circ = k.circle(o, 3.1);
    let d = k.point(-3.1, 0.05);
    let b = k.point(-2.9, 1.0);
    let c = k.point(1.5, 1.0);
    let xa = k.xaxis();
    k.on(d, f);
    k.on(d, xa);
    k.on(d, circ);
    k.on(c, f);
    k.on(b, circ);
    let seg = k.line(b, c);
    k.horizontal(seg);
    k.tangent(seg, f);
    let len = k.measure(DimKind::Length(seg));
    k.check_value(len, PI / 2.0 + (PI * PI - 1.0).sqrt());
}

/// Construction tools and coordinate dimensions: circumcenter coordinates.
#[test]
fn circumcenter_coordinates() {
    let mut k = Kit::new();
    let o = k.origin();
    let b = k.point(4.0, 0.3);
    let c = k.point(1.2, 3.1);
    k.drive(DimKind::CoordX(b), "4");
    k.drive(DimKind::CoordY(b), "0");
    k.drive(DimKind::CoordX(c), "1");
    k.drive(DimKind::CoordY(c), "3");
    let (l1, _) = k.perp_bisector(o, b);
    let (l2, _) = k.perp_bisector(b, c);
    let x = k.intersect(l1, l2, [2.0, 0.8]);
    let cx = k.measure(DimKind::CoordX(x));
    let cy = k.measure(DimKind::CoordY(x));
    k.check_value(cx, 2.0);
    k.check_value(cy, 1.0);
}

/// Variables: a dimension driven by an expression, and a solved variable.
#[test]
fn variables() {
    let mut k = Kit::new();
    let o = k.origin();
    let s = k.var("s", 1.5);
    let p = k.point(2.0, 1.0);
    let seg = k.line(o, p);
    k.drive(DimKind::Length(seg), "2s + 1");
    k.drive(DimKind::Angle(k.xaxis(), seg), "30");
    let y = k.measure(DimKind::CoordY(p));
    k.check_value(y, 2.0);
    k.set_var(s, 2.0);
    k.check_value(y, 2.5);
    let _ = Con::Coincident(0, 0);
}
