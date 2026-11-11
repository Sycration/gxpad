//! Conic constructions: foci, directrices and asymptotes (constructed as
//! points and lines bound to their conic), tangents, intersections, rotation.

use gxpad::model::DimKind::*;
use gxpad::testkit::Kit;
use std::f64::consts::PI;

/// Ellipse: |PF1| + |PF2| = 2a.
#[test]
fn ellipse_focal_sum() {
    let mut k = Kit::new();
    let o = k.origin();
    k.var("a", 5.0);
    k.var("b", 3.0);
    let e = k.ellipse(o, 5.0, 3.0, 0.0);
    k.lock_conic_angle(e);
    k.drive(ConicA(e), "a");
    k.drive(ConicB(e), "b");
    let foci = k.foci(e);
    let (f2, f1) = (foci[0], foci[1]);
    let p = k.point(2.5, 2.6);
    k.on(p, e);
    k.drive(Distance(f2, p), "3");
    assert_eq!(k.dof(), 0);
    let d1 = k.measure(Distance(f1, p));
    k.check_value(d1, 7.0);
}

/// Hyperbola: |PF1| - |PF2| = 2a on the right branch.
#[test]
fn hyperbola_focal_difference() {
    let mut k = Kit::new();
    let o = k.origin();
    k.var("a", 3.0);
    k.var("b", 4.0);
    let h = k.hyperbola(o, 3.0, 4.0, 0.0);
    k.lock_conic_angle(h);
    k.drive(ConicA(h), "a");
    k.drive(ConicB(h), "b");
    let foci = k.foci(h);
    let (f2, f1) = (foci[0], foci[1]);
    let p = k.point(4.2, 3.9);
    k.on(p, h);
    k.drive(Distance(f2, p), "4");
    assert_eq!(k.dof(), 0);
    let d1 = k.measure(Distance(f1, p));
    k.check_value(d1, 10.0);
}

/// Parabola: a point is as far from the focus as from the directrix.
#[test]
fn parabola_focus_directrix() {
    let mut k = Kit::new();
    let o = k.origin();
    k.var("a", 1.5);
    let par = k.parabola(o, 1.5, 0.0);
    k.lock_conic_angle(par);
    k.drive(ConicA(par), "a");
    let f = k.foci(par)[0];
    let dir = k.directrix(par, [0.0, 0.0]);
    let p = k.point(1.5, 3.0);
    k.on(p, par);
    k.drive(CoordY(p), "3");
    assert_eq!(k.dof(), 0);
    let pf = k.measure(Distance(p, f));
    let pd = k.measure(PointLine(p, dir));
    k.check_value(pf, 3.0);
    k.check_value(pd, 3.0);
}

/// Latus rectum of a parabola with a free vertex: length 4a.
#[test]
fn parabola_latus_rectum() {
    let mut k = Kit::new();
    k.var("a", 2.0);
    let v = k.point(1.0, 1.0);
    let par = k.parabola(v, 2.0, 0.0);
    k.lock_conic_angle(par);
    k.drive(ConicA(par), "a");
    let f = k.foci(par)[0];
    let g = k.point(3.0, 3.0);
    let l = k.inf_line(f, g);
    k.vertical(l);
    let p1 = k.intersect(l, par, [3.0, 5.0]);
    let p2 = k.intersect(l, par, [3.0, -3.0]);
    assert!(k.dof() > 0);
    let len = k.measure(Distance(p1, p2));
    k.check_value(len, 8.0);
}

/// Latus rectum of an ellipse with a free center: 2b²/a.
#[test]
fn ellipse_latus_rectum() {
    let mut k = Kit::new();
    k.var("a", 5.0);
    k.var("b", 3.0);
    let c = k.point(1.0, 1.0);
    let e = k.ellipse(c, 5.0, 3.0, 0.0);
    k.lock_conic_angle(e);
    k.drive(ConicA(e), "a");
    k.drive(ConicB(e), "b");
    let f = k.foci(e)[0];
    let g = k.point(5.0, 3.0);
    let l = k.inf_line(f, g);
    k.vertical(l);
    let p1 = k.intersect(l, e, [5.0, 2.8]);
    let p2 = k.intersect(l, e, [5.0, -0.8]);
    assert!(k.dof() > 0);
    let len = k.measure(Distance(p1, p2));
    k.check_value(len, 2.0 * 9.0 / 5.0);
}

/// Tangent segment from an external point to an ellipse.
#[test]
fn ellipse_tangent_from_point() {
    let mut k = Kit::new();
    let o = k.origin();
    let e = k.ellipse(o, 5.0, 3.0, 0.0);
    k.lock_conic_angle(e);
    k.drive(ConicA(e), "5");
    k.drive(ConicB(e), "3");
    let p = k.fixed_point(7.0, 0.0);
    let (seg, t) = k.tangent_line(p, e, [3.6, 1.9]);
    assert_eq!(k.dof(), 0);
    let len = k.measure(Length(seg));
    let ty = k.measure(CoordY(t));
    k.check_value(len, (792.0f64).sqrt() / 7.0);
    k.check_value(ty, 3.0 * 24.0f64.sqrt() / 7.0);
}

/// Tangent to a parabola from a point on its axis; the vertex floats.
#[test]
fn parabola_tangent_from_axis_point() {
    let mut k = Kit::new();
    let v = k.point(1.0, 1.0);
    let par = k.parabola(v, 1.0, 0.0);
    k.lock_conic_angle(par);
    k.drive(ConicA(par), "1");
    let p = k.point(0.1, 1.1);
    k.drive(HDist(v, p), "1");
    k.drive(VDist(v, p), "0");
    let (seg, t) = k.tangent_line(p, par, [2.0, 3.0]);
    assert!(k.dof() > 0);
    let len = k.measure(Length(seg));
    let vt = k.measure(VDist(v, t));
    k.check_value(len, 8.0f64.sqrt());
    k.check_value(vt, 2.0);
}

/// Line of given direction tangent to an ellipse: its distance from the center.
#[test]
fn ellipse_tangent_with_direction() {
    let mut k = Kit::new();
    let o = k.origin();
    let e = k.ellipse(o, 5.0, 3.0, 0.0);
    k.lock_conic_angle(e);
    k.drive(ConicA(e), "5");
    k.drive(ConicB(e), "3");
    let a = k.point(-4.0, 1.8);
    let b = k.point(4.0, 6.5);
    let l = k.inf_line(a, b);
    let xa = k.xaxis();
    k.drive(Angle(xa, l), "30");
    k.tangent(l, e);
    assert!(k.dof() > 0);
    let d = k.measure(PointLine(o, l));
    let phi = 30.0f64.to_radians();
    k.check_value(d, (25.0 * phi.sin().powi(2) + 9.0 * phi.cos().powi(2)).sqrt());
}

/// Line of given direction tangent to a hyperbola.
#[test]
fn hyperbola_tangent_with_direction() {
    let mut k = Kit::new();
    let o = k.origin();
    k.var("a", 3.0);
    let h = k.hyperbola(o, 3.0, 2.0, 0.0);
    k.lock_conic_angle(h);
    k.drive(ConicA(h), "a");
    k.drive(ConicB(h), "2");
    let a = k.point(0.0, 4.8);
    let b = k.point(2.0, 8.3);
    let l = k.inf_line(a, b);
    let xa = k.xaxis();
    k.drive(Angle(xa, l), "60");
    k.tangent(l, h);
    assert!(k.dof() > 0);
    let d = k.measure(PointLine(o, l));
    let phi = 60.0f64.to_radians();
    k.check_value(d, (9.0 * phi.sin().powi(2) - 4.0 * phi.cos().powi(2)).sqrt());
}

/// Horizontal chord of a hyperbola at height h.
#[test]
fn hyperbola_horizontal_chord() {
    let mut k = Kit::new();
    let o = k.origin();
    k.var("h", 1.5);
    let hy = k.hyperbola(o, 3.0, 2.0, 0.0);
    k.lock_conic_angle(hy);
    k.drive(ConicA(hy), "3");
    k.drive(ConicB(hy), "2");
    let a = k.point(-5.0, 1.5);
    let b = k.point(5.0, 1.5);
    let l = k.inf_line(a, b);
    k.horizontal(l);
    k.drive(PointLine(o, l), "h");
    let p1 = k.intersect(l, hy, [-3.8, 1.5]);
    let p2 = k.intersect(l, hy, [3.8, 1.5]);
    assert!(k.dof() > 0);
    let len = k.measure(Distance(p1, p2));
    k.check_value(len, 6.0 * (1.0f64 + 1.5 * 1.5 / 4.0).sqrt());
}

/// Ellipse meets a concentric circle; the center floats.
#[test]
fn ellipse_concentric_circle() {
    let mut k = Kit::new();
    let c = k.point(2.0, 1.0);
    let e = k.ellipse(c, 5.0, 3.0, 0.0);
    k.lock_conic_angle(e);
    k.drive(ConicA(e), "5");
    k.drive(ConicB(e), "3");
    let circ = k.circle(c, 4.0);
    k.drive(Radius(circ), "4");
    let p = k.intersect(e, circ, [5.3, 3.25]);
    assert!(k.dof() > 0);
    let dx = k.measure(HDist(c, p));
    let dy = k.measure(VDist(c, p));
    k.check_value(dx, (25.0f64 * 7.0 / 16.0).sqrt());
    k.check_value(dy, 2.25);
}

/// Circle around a parabola's focus meets the parabola at focal distance r.
#[test]
fn parabola_focus_circle() {
    let mut k = Kit::new();
    let v = k.point(0.5, 0.5);
    let par = k.parabola(v, 1.0, 0.0);
    k.lock_conic_angle(par);
    k.drive(ConicA(par), "1");
    let f = k.foci(par)[0];
    let circ = k.circle(f, 3.0);
    k.drive(Radius(circ), "3");
    let p = k.intersect(circ, par, [2.5, 3.3]);
    assert!(k.dof() > 0);
    let u = k.measure(HDist(v, p));
    let w = k.measure(VDist(v, p));
    k.check_value(u, 2.0);
    k.check_value(w, 8.0f64.sqrt());
}

/// Circle centered on the major axis, tangent to the ellipse from inside.
#[test]
fn circle_inside_ellipse() {
    let mut k = Kit::new();
    let o = k.origin();
    let e = k.ellipse(o, 5.0, 3.0, 0.0);
    k.lock_conic_angle(e);
    k.drive(ConicA(e), "5");
    k.drive(ConicB(e), "3");
    let c = k.point(1.5, 0.2);
    let xa = k.xaxis();
    k.on(c, xa);
    k.drive(CoordX(c), "1.5");
    let circ = k.circle(c, 2.78);
    k.app.mouse_w = [2.34, 2.65];
    k.tangent(circ, e);
    assert_eq!(k.dof(), 0);
    let r = k.measure(Radius(circ));
    k.check_value(r, 3.0 * (1.0f64 - 1.5 * 1.5 / 16.0).sqrt());
}

/// Circle at the parabola's focus tangent to it: radius is the focal length.
#[test]
fn parabola_focus_circle_tangent() {
    let mut k = Kit::new();
    k.var("a", 1.5);
    let v = k.point(0.0, 0.0);
    let par = k.parabola(v, 1.5, 0.0);
    k.lock_conic_angle(par);
    k.drive(ConicA(par), "a");
    let f = k.foci(par)[0];
    let circ = k.circle(f, 1.4);
    k.app.mouse_w = [0.0, 0.0];
    k.tangent(circ, par);
    assert!(k.dof() > 0);
    let r = k.measure(Radius(circ));
    k.check_value(r, 1.5);
}

/// A point sliding on an ellipse and its partner on the auxiliary circle.
#[test]
fn ellipse_auxiliary_circle() {
    let mut k = Kit::new();
    let c = k.point(0.5, 0.5);
    k.var("a", 5.0);
    let e = k.ellipse(c, 5.0, 3.0, 0.0);
    k.lock_conic_angle(e);
    k.drive(ConicA(e), "a");
    k.drive(ConicB(e), "3");
    let circ = k.circle(c, 5.0);
    k.drive(Radius(circ), "a");
    let p = k.point(3.5, 3.0);
    let q = k.point(3.5, 4.5);
    k.on(p, e);
    k.on(q, circ);
    k.drive(HDist(c, p), "3");
    let pq = k.line(p, q);
    k.vertical(pq);
    assert!(k.dof() > 0);
    let yp = k.measure(VDist(c, p));
    let yq = k.measure(VDist(c, q));
    k.check_value(yp, 2.4);
    k.check_value(yq, 4.0);
}

/// Rotated ellipse: the rotation is determined by a point's distance and direction.
#[test]
fn rotated_ellipse_by_radial_point() {
    let mut k = Kit::new();
    let c = k.point(1.0, -1.0);
    let e = k.ellipse(c, 5.0, 3.0, 0.5);
    k.drive(ConicA(e), "5");
    k.drive(ConicB(e), "3");
    let h = k.point(4.0, -1.0);
    let hl = k.line(c, h);
    k.horizontal(hl);
    let p = k.point(3.0, 2.4);
    let q = k.point(-2.2, 1.0);
    k.on(p, e);
    k.on(q, e);
    let cp = k.line(c, p);
    let cq = k.line(c, q);
    k.drive(Distance(c, p), "4");
    k.drive(Angle(hl, cp), "60");
    k.drive(Angle(hl, cq), "150");
    assert!(k.dof() > 0);
    let r2 = k.measure(Distance(c, q));
    let s2: f64 = (1.0 / 16.0 - 1.0 / 25.0) / (1.0 / 9.0 - 1.0 / 25.0);
    let expected: f64 = 1.0 / (s2 / 25.0 + (1.0 - s2) / 9.0).sqrt();
    k.check_value(r2, expected);
}

/// Radial distance to a hyperbola along a ray from the center.
#[test]
fn hyperbola_ray_from_center() {
    let mut k = Kit::new();
    let c = k.point(2.0, 1.0);
    let h = k.hyperbola(c, 2.0, 4.0, 0.0);
    k.lock_conic_angle(h);
    k.drive(ConicA(h), "2");
    k.drive(ConicB(h), "4");
    let p = k.point(4.86, 5.08);
    k.on(p, h);
    let cp = k.line(c, p);
    let xa = k.xaxis();
    k.drive(Angle(xa, cp), "55");
    assert!(k.dof() > 0);
    let t = 55.0f64.to_radians();
    let d = k.measure(Distance(c, p));
    k.check_value(d, (64.0 / (16.0 * t.cos().powi(2) - 4.0 * t.sin().powi(2))).sqrt());
}

/// Ray from a parabola's vertex: r = 4a cos(t) / sin(t)^2.
#[test]
fn parabola_ray_from_vertex() {
    let mut k = Kit::new();
    k.var("a", 1.2);
    let v = k.point(-1.0, 2.0);
    let par = k.parabola(v, 1.2, 0.0);
    k.lock_conic_angle(par);
    k.drive(ConicA(par), "a");
    let p = k.point(0.0, 4.0);
    k.on(p, par);
    let vp = k.line(v, p);
    let xa = k.xaxis();
    k.drive(Angle(xa, vp), "50");
    assert!(k.dof() > 0);
    let t = 50.0f64.to_radians();
    let d = k.measure(Distance(v, p));
    k.check_value(d, 4.0 * 1.2 * t.cos() / t.sin().powi(2));
}

/// Confocal ellipse and hyperbola cross at a right angle.
#[test]
fn confocal_orthogonal() {
    let mut k = Kit::new();
    let o = k.origin();
    let e = k.ellipse(o, 5.0, 3.0, 0.0);
    k.lock_conic_angle(e);
    k.drive(ConicA(e), "5");
    k.drive(ConicB(e), "3");
    let h = k.hyperbola(o, 2.0, 12.0f64.sqrt(), 0.0);
    k.lock_conic_angle(h);
    k.drive(ConicA(h), "2");
    k.drive(ConicB(h), "sqrt(12)");
    let p = k.intersect(e, h, [2.5, 2.6]);
    let b1 = k.point(1.5, 2.95);
    let b2 = k.point(2.85, 3.6);
    let l1 = k.inf_line(p, b1);
    let l2 = k.inf_line(p, b2);
    let pp = k.doc().pval(p);
    k.app.mouse_w = pp;
    k.tangent(l1, e);
    k.tangent(l2, h);
    assert!(k.dof() > 0);
    let ang = k.measure(Angle(l1, l2));
    k.check_value(ang, PI / 2.0);
}

/// Focal distance and distance to directrix: ratio is the eccentricity.
#[test]
fn ellipse_directrix_ratio() {
    let mut k = Kit::new();
    let o = k.origin();
    k.var("a", 5.0);
    k.var("b", 3.0);
    let e = k.ellipse(o, 5.0, 3.0, 0.0);
    k.lock_conic_angle(e);
    k.drive(ConicA(e), "a");
    k.drive(ConicB(e), "b");
    let f = k.foci(e)[0];
    let dir = k.directrix(e, [6.25, 0.0]);
    let p = k.point(3.7, 2.0);
    k.on(p, e);
    k.drive(CoordY(p), "2");
    assert_eq!(k.dof(), 0);
    let x = 5.0 * (1.0f64 - 4.0 / 9.0).sqrt();
    let pf = k.measure(Distance(p, f));
    let pd = k.measure(PointLine(p, dir));
    k.check_value(pf, 5.0 - 0.8 * x);
    k.check_value(pd, 6.25 - x);
}

/// Focal chord of an ellipse at angle theta: 2ab²/(a² - c²cos²).
#[test]
fn ellipse_focal_chord() {
    let mut k = Kit::new();
    let o = k.origin();
    k.var("a", 5.0);
    k.var("b", 3.0);
    let e = k.ellipse(o, 5.0, 3.0, 0.0);
    k.lock_conic_angle(e);
    k.drive(ConicA(e), "a");
    k.drive(ConicB(e), "b");
    let f = k.foci(e)[0];
    let th = 50.0f64.to_radians();
    let g = k.point(4.0 + 2.0 * th.cos(), 2.0 * th.sin());
    let l = k.inf_line(f, g);
    let xa = k.xaxis();
    k.drive(Angle(xa, l), "50");
    let r = |t: f64| 1.8 / (1.0 + 0.8 * t.cos());
    let (r1, r2) = (r(th), r(th + PI));
    let p1 = k.intersect(l, e, [4.0 + r1 * th.cos(), r1 * th.sin()]);
    let p2 = k.intersect(l, e, [4.0 - r2 * th.cos(), -r2 * th.sin()]);
    assert!(k.dof() > 0);
    let len = k.measure(Distance(p1, p2));
    k.check_value(len, 2.0 * 5.0 * 9.0 / (25.0 - 16.0 * th.cos().powi(2)));
}

/// Focal chord of a parabola at angle theta: 4a / sin².
#[test]
fn parabola_focal_chord() {
    let mut k = Kit::new();
    let o = k.origin();
    let par = k.parabola(o, 1.5, 0.0);
    k.lock_conic_angle(par);
    k.drive(ConicA(par), "1.5");
    let f = k.foci(par)[0];
    let th = 70.0f64.to_radians();
    let g = k.point(1.5 + th.cos(), th.sin());
    let l = k.inf_line(f, g);
    let xa = k.xaxis();
    k.drive(Angle(xa, l), "70");
    let r = |t: f64| 3.0 / (1.0 - t.cos());
    let p1 = k.intersect(l, par, [1.5 + r(th) * th.cos(), r(th) * th.sin()]);
    let t2 = th + PI;
    let p2 = k.intersect(l, par, [1.5 + r(t2) * t2.cos(), r(t2) * t2.sin()]);
    assert!(k.dof() > 0);
    let len = k.measure(Distance(p1, p2));
    k.check_value(len, 6.0 / th.sin().powi(2));
}

/// Reflection property: the tangent at P meets the axis at T with |TF| = |PF|.
#[test]
fn parabola_tangent_isosceles() {
    let mut k = Kit::new();
    let o = k.origin();
    let par = k.parabola(o, 1.5, 0.0);
    k.lock_conic_angle(par);
    k.drive(ConicA(par), "1.5");
    let f = k.foci(par)[0];
    let p = k.point(1.5, 3.0);
    k.on(p, par);
    k.drive(CoordY(p), "3");
    let b = k.point(0.0, 1.5);
    let l = k.inf_line(p, b);
    k.tangent(l, par);
    let xa = k.xaxis();
    let t = k.intersect(l, xa, [-1.5, 0.0]);
    assert!(k.dof() > 0);
    let tf = k.measure(Distance(t, f));
    let tx = k.measure(CoordX(t));
    k.check_value(tf, 3.0);
    k.check_value(tx, -1.5);
}

/// Director circle: perpendicular tangents meet at distance sqrt(a²+b²).
#[test]
fn ellipse_director_circle() {
    let mut k = Kit::new();
    let o = k.origin();
    let e = k.ellipse(o, 5.0, 3.0, 0.0);
    k.lock_conic_angle(e);
    k.drive(ConicA(e), "5");
    k.drive(ConicB(e), "3");
    let p = k.point(5.8, 0.0);
    let xa = k.xaxis();
    k.on(p, xa);
    let (s1, _) = k.tangent_line(p, e, [4.5, 1.4]);
    let (s2, _) = k.tangent_line(p, e, [4.5, -1.4]);
    k.drive(Angle(s2, s1), "90");
    assert_eq!(k.dof(), 0);
    let x = k.measure(CoordX(p));
    k.check_value(x, 34.0f64.sqrt());
}

/// Tangent segment from a point between the hyperbola's branches.
#[test]
fn hyperbola_tangent_from_point() {
    let mut k = Kit::new();
    let o = k.origin();
    let h = k.hyperbola(o, 2.0, 3.0, 0.0);
    k.lock_conic_angle(h);
    k.drive(ConicA(h), "2");
    k.drive(ConicB(h), "3");
    let p = k.fixed_point(1.0, 0.0);
    let (seg, t) = k.tangent_line(p, h, [4.0, 5.2]);
    assert_eq!(k.dof(), 0);
    let len = k.measure(Length(seg));
    let tx = k.measure(CoordX(t));
    k.check_value(len, 6.0);
    k.check_value(tx, 4.0);
}

/// Circle at the hyperbola's focus touching the near vertex: r = c - a.
#[test]
fn hyperbola_focus_circle() {
    let mut k = Kit::new();
    k.var("a", 3.0);
    k.var("b", 4.0);
    let c = k.point(1.0, 1.0);
    let h = k.hyperbola(c, 3.0, 4.0, 0.0);
    k.lock_conic_angle(h);
    k.drive(ConicA(h), "a");
    k.drive(ConicB(h), "b");
    let f = k.foci(h)[0];
    let circ = k.circle(f, 1.9);
    k.app.mouse_w = [4.0, 1.0];
    k.tangent(circ, h);
    assert!(k.dof() > 0);
    let r = k.measure(Radius(circ));
    k.check_value(r, 2.0);
}

/// Two parabolas with a common focus and opposite axes meet at focal distance a+b.
#[test]
fn confocal_parabolas() {
    let mut k = Kit::new();
    let o = k.origin();
    k.var("a", 1.0);
    k.var("b", 2.0);
    let v1 = k.point(-1.0, 0.1);
    let v2 = k.point(2.0, 0.1);
    k.drive(CoordX(v1), "-a");
    k.drive(CoordY(v1), "0");
    k.drive(CoordX(v2), "b");
    k.drive(CoordY(v2), "0");
    let p1 = k.parabola(v1, 1.0, 0.0);
    let p2 = k.parabola(v2, 2.0, PI);
    k.lock_conic_angle(p1);
    k.lock_conic_angle(p2);
    k.drive(ConicA(p1), "a");
    k.drive(ConicA(p2), "b");
    let p = k.intersect(p1, p2, [1.0, 2.8]);
    assert_eq!(k.dof(), 0);
    let d = k.measure(Distance(o, p));
    let x = k.measure(CoordX(p));
    let y = k.measure(CoordY(p));
    k.check_value(d, 3.0);
    k.check_value(x, 1.0);
    k.check_value(y, 8.0f64.sqrt());
}

/// Ellipse semi-minor axis found from a point on it (free center).
#[test]
fn ellipse_minor_axis_from_point() {
    let mut k = Kit::new();
    let c = k.point(1.0, 1.0);
    let e = k.ellipse(c, 5.0, 2.5, 0.0);
    k.lock_conic_angle(e);
    k.drive(ConicA(e), "5");
    let p = k.point(4.0, 3.3);
    k.on(p, e);
    k.drive(HDist(c, p), "3");
    k.drive(VDist(c, p), "2.4");
    assert!(k.dof() > 0);
    let b = k.measure(ConicB(e));
    k.check_value(b, 3.0);
}

/// Parabola focal length found from a point on it (free vertex).
#[test]
fn parabola_focal_length_from_point() {
    let mut k = Kit::new();
    let v = k.point(1.0, 1.0);
    let par = k.parabola(v, 1.0, 0.0);
    k.lock_conic_angle(par);
    let p = k.point(3.0, 4.0);
    k.on(p, par);
    k.drive(HDist(v, p), "2");
    k.drive(VDist(v, p), "3");
    assert!(k.dof() > 0);
    let a = k.measure(ConicA(par));
    k.check_value(a, 9.0 / 8.0);
}

/// Hyperbola semi-axes from two points on it.
#[test]
fn hyperbola_axes_from_two_points() {
    let mut k = Kit::new();
    let o = k.origin();
    let h = k.hyperbola(o, 3.0, 2.0, 0.0);
    k.lock_conic_angle(h);
    let p = k.point(5.0, 3.0);
    let q = k.point(6.0, 3.3);
    k.on(p, h);
    k.on(q, h);
    // a = 4, b = 3
    let (a, b) = (4.0f64, 3.0f64);
    let py = b * (25.0 / 16.0 - 1.0f64).sqrt();
    let qy = b * (36.0f64 / 16.0 - 1.0).sqrt();
    k.drive(CoordX(p), "5");
    k.drive(CoordY(p), &format!("{py}"));
    k.drive(CoordX(q), "6");
    k.drive(CoordY(q), &format!("{qy}"));
    assert_eq!(k.dof(), 0);
    let ma = k.measure(ConicA(h));
    let mb = k.measure(ConicB(h));
    k.check_value(ma, a);
    k.check_value(mb, b);
}

/// Latus rectum of a parabola whose vertex slides along the axis line.
#[test]
fn parabola_latus_rectum_sliding_vertex() {
    let mut k = Kit::new();
    k.var("a", 2.0);
    let xa = k.xaxis();
    let v = k.point(1.0, 0.1);
    k.on(v, xa);
    let par = k.parabola(v, 2.0, 0.0);
    k.lock_conic_angle(par);
    k.drive(ConicA(par), "a");
    let f = k.foci(par)[0];
    let l = k.perp_line(xa, f);
    let p1 = k.intersect(l, par, [3.0, 4.0]);
    let p2 = k.intersect(l, par, [3.0, -4.0]);
    assert_eq!(k.dof(), 1);
    let len = k.measure(Distance(p1, p2));
    k.check_value(len, 8.0);
}

/// Latus rectum of an ellipse whose center slides along the major axis.
#[test]
fn ellipse_latus_rectum_sliding_center() {
    let mut k = Kit::new();
    k.var("a", 5.0);
    k.var("b", 3.0);
    let xa = k.xaxis();
    let c = k.point(1.0, 0.1);
    k.on(c, xa);
    let e = k.ellipse(c, 5.0, 3.0, 0.0);
    k.lock_conic_angle(e);
    k.drive(ConicA(e), "a");
    k.drive(ConicB(e), "b");
    let f = k.foci(e)[0];
    let l = k.perp_line(xa, f);
    let p1 = k.intersect(l, e, [5.0, 1.8]);
    let p2 = k.intersect(l, e, [5.0, -1.8]);
    assert_eq!(k.dof(), 1);
    let len = k.measure(Distance(p1, p2));
    k.check_value(len, 3.6);
}

// ---------------------------------------------------------------- foci, directrices and
// asymptotes as features: the points and lines are bound to the conic, so
// they can be measured, dragged and constrained like any other

/// Distance between the foci of an ellipse: 2 sqrt(a^2 - b^2).
#[test]
fn ellipse_foci_distance() {
    let mut k = Kit::new();
    let o = k.origin();
    let e = k.ellipse(o, 5.0, 3.0, 0.3);
    k.drive(ConicA(e), "5");
    k.drive(ConicB(e), "3");
    let f = k.foci(e);
    assert_eq!(f.len(), 2);
    let d = k.measure(Distance(f[0], f[1]));
    k.check_value(d, 8.0);
}

/// With b > a the foci lie on the other axis: 2 sqrt(b^2 - a^2).
#[test]
fn ellipse_foci_on_second_axis() {
    let mut k = Kit::new();
    let o = k.origin();
    let e = k.ellipse(o, 3.0, 5.0, 0.0);
    k.lock_conic_angle(e);
    k.drive(ConicA(e), "3");
    k.drive(ConicB(e), "5");
    let f = k.foci(e);
    let d = k.measure(Distance(f[0], f[1]));
    k.check_value(d, 8.0);
    let x = k.measure(CoordX(f[0]));
    k.check_value(x, 0.0);
}

/// Distance between the foci of a hyperbola: 2 sqrt(a^2 + b^2).
#[test]
fn hyperbola_foci_distance() {
    let mut k = Kit::new();
    let o = k.origin();
    let h = k.hyperbola(o, 3.0, 4.0, 0.0);
    k.drive(ConicA(h), "3");
    k.drive(ConicB(h), "4");
    let f = k.foci(h);
    let d = k.measure(Distance(f[0], f[1]));
    k.check_value(d, 10.0);
}

/// Parabola: the focus is a from the vertex and 2a from the directrix.
#[test]
fn parabola_focus_and_directrix() {
    let mut k = Kit::new();
    let o = k.origin();
    let par = k.parabola(o, 1.5, 0.4);
    k.drive(ConicA(par), "1.5");
    let f = k.foci(par);
    assert_eq!(f.len(), 1);
    let dl = k.directrix(par, [0.0, 0.0]);
    let fv = k.measure(Distance(o, f[0]));
    k.check_value(fv, 1.5);
    let fd = k.measure(PointLine(f[0], dl));
    k.check_value(fd, 3.0);
}

/// The directrix of an ellipse is a^2 / c from the center (c the focal distance).
#[test]
fn ellipse_directrix_distance() {
    let mut k = Kit::new();
    let o = k.origin();
    let e = k.ellipse(o, 5.0, 3.0, 0.0);
    k.drive(ConicA(e), "5");
    k.drive(ConicB(e), "3");
    let dl = k.directrix(e, [6.0, 0.0]);
    let d = k.measure(PointLine(o, dl));
    k.check_value(d, 25.0 / 4.0);
}

/// The directrix of a hyperbola is a^2 / c from the center.
#[test]
fn hyperbola_directrix_distance() {
    let mut k = Kit::new();
    let o = k.origin();
    let h = k.hyperbola(o, 3.0, 4.0, 0.0);
    k.drive(ConicA(h), "3");
    k.drive(ConicB(h), "4");
    let dl = k.directrix(h, [-2.0, 0.0]);
    let d = k.measure(PointLine(o, dl));
    k.check_value(d, 9.0 / 5.0);
}

/// The asymptotes of a hyperbola meet at 2 atan(b/a).
#[test]
fn hyperbola_asymptote_angle() {
    let mut k = Kit::new();
    let o = k.origin();
    let h = k.hyperbola(o, 3.0, 4.0, 0.2);
    k.drive(ConicA(h), "3");
    k.drive(ConicB(h), "4");
    let (l1, l2) = k.asymptotes(h);
    let ang = k.measure_angle(l1, l2);
    let v = k.value(ang);
    let full = 2.0 * (4.0f64 / 3.0).atan();
    assert!((v - full).abs() < 1e-9 || (v - (std::f64::consts::PI - full)).abs() < 1e-9, "angle {v}");
    k.check(ang);
}

/// The foci are handles: fixing their distance determines b.
#[test]
fn ellipse_b_from_foci_distance() {
    let mut k = Kit::new();
    let o = k.origin();
    let e = k.ellipse(o, 5.0, 3.2, 0.0);
    k.drive(ConicA(e), "5");
    let f = k.foci(e);
    k.drive(Distance(f[0], f[1]), "8");
    let b = k.measure(ConicB(e));
    k.check_value(b, 3.0);
}

/// An asymptote is a line like any other: passing it through a point fixes b.
#[test]
fn hyperbola_b_from_asymptote_point() {
    let mut k = Kit::new();
    let o = k.origin();
    let h = k.hyperbola(o, 3.0, 3.8, 0.0);
    k.lock_conic_angle(h);
    k.drive(ConicA(h), "3");
    let (l1, _) = k.asymptotes(h);
    let p = k.point(3.0, 4.1);
    k.drive(CoordX(p), "3");
    k.drive(CoordY(p), "4");
    k.on(p, l1);
    let b = k.measure(ConicB(h));
    k.check_value(b, 4.0);
}

/// Deleting a conic deletes its foci, directrices and asymptotes with it.
#[test]
fn features_deleted_with_conic() {
    use gxpad::app::Sel;
    use gxpad::model::Con;
    let mut k = Kit::new();
    let o = k.origin();
    let h = k.hyperbola(o, 3.0, 4.0, 0.0);
    let f = k.foci(h);
    let dl = k.directrix(h, [2.0, 0.0]);
    let (a1, a2) = k.asymptotes(h);
    k.app.sel = vec![Sel::Ent(h)];
    k.app.delete_selection();
    let d = k.doc();
    for e in f.iter().copied().chain([dl, a1, a2]) {
        assert!(d.ent(e).is_none(), "entity {e} survived its conic");
    }
    assert!(!d.cons.iter().flatten().any(|c| matches!(c, Con::Focus { .. } | Con::Directrix { .. } | Con::Asymptote { .. })));
    assert!(d.ent(o).is_some(), "the origin is not part of the conic");
}

// ---------------------------------------------------------------- conics built
// from their foci; their features are constructed from them in turn

/// Ellipse from its foci and a point: a and b follow from the focal distances.
#[test]
fn ellipse_by_foci_and_point() {
    let mut k = Kit::new();
    let f1 = k.origin();
    let f2 = k.point(8.1, 0.3);
    k.drive(Distance(f1, f2), "8");
    let p = k.point(3.5, 3.2);
    k.drive(Distance(p, f1), "6");
    k.drive(Distance(p, f2), "4");
    let e = k.ellipse_by_foci(f1, f2, p);
    // the given points are the foci, and the directrix is built on them
    let mut foci = k.foci(e);
    foci.sort();
    assert_eq!(foci, {
        let mut v = vec![f1, f2];
        v.sort();
        v
    });
    let a = k.measure(ConicA(e));
    let b = k.measure(ConicB(e));
    k.check_value(a, 5.0);
    k.check_value(b, 3.0);
    let near = k.doc().pval(f1);
    let dl = k.directrix(e, near);
    let fd = k.measure(PointLine(f1, dl));
    k.check_value(fd, 25.0 / 4.0 - 4.0);
}

/// Hyperbola from its foci and a point, with asymptotes built on it.
#[test]
fn hyperbola_by_foci_and_point() {
    let mut k = Kit::new();
    let f1 = k.origin();
    let f2 = k.point(10.2, 0.1);
    k.drive(Distance(f1, f2), "10");
    let p = k.point(7.0, 1.5);
    k.drive(Distance(p, f1), "9");
    k.drive(Distance(p, f2), "3");
    let h = k.hyperbola_by_foci(f1, f2, p);
    let a = k.measure(ConicA(h));
    let b = k.measure(ConicB(h));
    k.check_value(a, 3.0);
    k.check_value(b, 4.0);
    let (l1, l2) = k.asymptotes(h);
    let ang = k.measure_angle(l1, l2);
    let v = k.value(ang);
    let full = 2.0 * (4.0f64 / 3.0).atan();
    assert!((v - full).abs() < 1e-9 || (v - (PI - full)).abs() < 1e-9, "angle {v}");
    k.check(ang);
}

/// Parabola from its focus and directrix: the focal length is half their
/// distance, and every point is as far from one as from the other.
#[test]
fn parabola_by_focus_and_directrix() {
    let mut k = Kit::new();
    let xa = k.xaxis();
    let f = k.point(0.3, 3.1);
    k.drive(CoordX(f), "0");
    k.drive(CoordY(f), "3");
    let par = k.parabola_by_focus(f, xa);
    assert_eq!(k.foci(par), vec![f]);
    // the given line is the directrix: the tool does not add a second one
    let near = k.doc().pval(f);
    assert_eq!(k.directrix(par, near), xa);
    let a = k.measure(ConicA(par));
    k.check_value(a, 1.5);
    let p = k.point(4.1, 4.2);
    k.on(p, par);
    k.drive(Distance(f, p), "5");
    let pd = k.measure(PointLine(p, xa));
    k.check_value(pd, 5.0);
}

/// Deleting a conic built from given foci keeps those points.
#[test]
fn given_foci_survive_their_conic() {
    use gxpad::app::Sel;
    let mut k = Kit::new();
    let f1 = k.point(-4.0, 0.0);
    let f2 = k.point(4.0, 0.0);
    let p = k.point(0.0, 3.0);
    let e = k.ellipse_by_foci(f1, f2, p);
    let near = k.doc().pval(f2);
    let dl = k.directrix(e, near);
    k.app.sel = vec![Sel::Ent(e)];
    k.app.delete_selection();
    let d = k.doc();
    for q in [f1, f2, p] {
        assert!(d.ent(q).is_some(), "point {q} was deleted with the conic");
    }
    assert!(d.ent(dl).is_none(), "the directrix outlived its conic");
}

/// A parabola whose focus is 2r from the center of a circle of radius r, with
/// the directrix tangent to the circle, rotated until it touches the x-axis:
/// the contact is 2 sqrt(3) r from the center. The parabola's angle is an
/// unknown of the system.
#[test]
fn parabola_from_circle_touching_axis() {
    let mut k = Kit::new();
    k.var("r", 1.0);
    let o = k.origin();
    let circ = k.circle(o, 1.0);
    k.drive(Radius(circ), "r");
    let a = k.point(2.55, 2.42);
    let b = k.point(0.23, -1.6);
    let ab = k.inf_line(a, b);
    k.app.mouse_w = [0.866, -0.5];
    k.tangent(ab, circ);
    let c = k.point(1.732, -1.0);
    let oc = k.line(o, c);
    k.perpendicular(oc, ab);
    let par = k.parabola_by_focus(c, ab);
    let e = k.point(0.866, -0.5);
    k.on(e, oc);
    k.on(e, circ);
    let ce = k.line(c, e);
    k.drive(Length(ce), "r");
    let g = k.point(3.46, 0.0);
    let xa = k.xaxis();
    k.on(g, par);
    k.on(g, xa);
    k.app.mouse_w = [3.46, 0.0];
    k.tangent(par, xa);
    let q = k.measure(Distance(g, o));
    k.check_value(q, 2.0 * 3f64.sqrt());
}
