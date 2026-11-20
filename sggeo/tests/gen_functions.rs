//! Function graphs: explicit and implicit curves, tangents, extrema,
//! inflections, intersections, curve coefficients as variables.

use sggeo::model::DimKind;
mod testkit;
use testkit::Kit;
use std::f64::consts::{E, PI};

/// Horizontal tangents of y = x^3 - 3x at x = 1 and x = -1; both tangent
/// segments have a free end, so the construction is under-constrained.
#[test]
fn cubic_horizontal_tangents() {
    let mut k = Kit::new();
    let f = k.curve("y = x^3 - 3*x");
    let p = k.point(1.1, -1.9);
    let q = k.point(2.5, -2.0);
    let r = k.point(-1.1, 1.9);
    let s = k.point(-2.5, 2.0);
    k.on(p, f);
    k.on(r, f);
    let pq = k.line(p, q);
    let rs = k.line(r, s);
    k.horizontal(pq);
    k.horizontal(rs);
    k.tangent(pq, f);
    k.tangent(rs, f);
    assert!(k.dof() > 0);
    let d = k.measure(DimKind::Distance(p, r));
    k.check_value(d, 20.0f64.sqrt());
    let py = k.measure(DimKind::CoordY(p));
    k.check_value(py, -2.0);
    let rx = k.measure(DimKind::CoordX(r));
    k.check_value(rx, -1.0);
}

/// Tangent at the inflection point (1, 0) of y = x^3 - 3x^2 + 2: slope -3.
#[test]
fn cubic_inflection_tangent() {
    let mut k = Kit::new();
    let f = k.curve("y = x^3 - 3*x^2 + 2");
    let p = k.point(1.05, 0.05);
    let q = k.point(1.6, -1.7);
    k.on(p, f);
    k.drive(DimKind::CoordX(p), "1");
    let pq = k.line(p, q);
    k.tangent(pq, f);
    k.drive(DimKind::Length(pq), "2");
    let dx = k.measure(DimKind::HDist(p, q));
    let dy = k.measure(DimKind::VDist(p, q));
    let py = k.measure(DimKind::CoordY(p));
    k.check_value(py, 0.0);
    k.check_value(dx, 2.0 / 10.0f64.sqrt());
    k.check_value(dy, 6.0 / 10.0f64.sqrt()); // VDist is unsigned
}

/// The inflection point is the root x = 1 of the cubic (x-1)(x^2-2x-2),
/// which has a closed form, but the root is found numerically.
#[test]
fn cubic_root_on_axis_exact() {
    let mut k = Kit::new();
    let f = k.curve("y = x^3 - 3*x^2 + 2");
    let xa = k.xaxis();
    let p = k.point(1.05, 0.05);
    k.on(p, f);
    k.on(p, xa);
    let px = k.measure(DimKind::CoordX(p));
    k.check_value(px, 1.0);
}

/// Tangent from (0, -a) to y = x^2 touches at x = sqrt(a).
#[test]
fn parabola_tangent_from_point_var() {
    let mut k = Kit::new();
    let a = k.var("a", 1.0);
    let f = k.curve("y = x^2");
    let pt = k.point(0.1, -1.1);
    k.drive(DimKind::CoordX(pt), "0");
    k.drive(DimKind::CoordY(pt), "-a");
    let (seg, t) = k.tangent_line(pt, f, [1.1, 1.0]);
    let free = k.point(0.7, -0.1);
    k.on(free, seg);
    assert!(k.dof() > 0);
    let tx = k.measure(DimKind::CoordX(t));
    let ty = k.measure(DimKind::CoordY(t));
    k.check_value(tx, 1.0);
    k.check_value(ty, 1.0);
    k.set_var(a, 4.0);
    k.check_value(tx, 2.0);
    k.check_value(ty, 4.0);
}

/// Zeros of sin at -pi and 2*pi (not the principal root).
#[test]
fn sine_zeros_distance() {
    let mut k = Kit::new();
    let f = k.curve("y = sin(x)");
    let xa = k.xaxis();
    let p = k.point(-3.2, 0.05);
    let q = k.point(6.2, -0.05);
    k.on(p, f);
    k.on(p, xa);
    k.on(q, f);
    k.on(q, xa);
    assert_eq!(k.dof(), 0);
    let d = k.measure(DimKind::Distance(p, q));
    k.check_value(d, 3.0 * PI);
    let h = k.measure(DimKind::HDist(p, q));
    k.check_value(h, 3.0 * PI);
}

/// Maximum of sin at x = -3*pi/2 (value 1), horizontal tangent segment.
#[test]
fn sine_extremum_minus_three_halves_pi() {
    let mut k = Kit::new();
    let f = k.curve("y = sin(x)");
    let p = k.point(-4.6, 0.95);
    let q = k.point(-3.0, 1.0);
    k.on(p, f);
    let pq = k.line(p, q);
    k.horizontal(pq);
    k.tangent(pq, f);
    assert!(k.dof() > 0);
    let x = k.measure(DimKind::CoordX(p));
    let y = k.measure(DimKind::CoordY(p));
    k.check_value(x, -1.5 * PI);
    k.check_value(y, 1.0);
}

/// Minimum of cos at pi and the next maximum at 2*pi: horizontal tangents.
#[test]
fn cosine_min_and_max_separation() {
    let mut k = Kit::new();
    let f = k.curve("y = cos(x)");
    let p = k.point(3.0, -0.98);
    let q = k.point(4.0, -1.0);
    let r = k.point(6.1, 0.98);
    let s = k.point(7.0, 1.0);
    k.on(p, f);
    k.on(r, f);
    let pq = k.line(p, q);
    let rs = k.line(r, s);
    k.horizontal(pq);
    k.horizontal(rs);
    k.tangent(pq, f);
    k.tangent(rs, f);
    assert!(k.dof() > 0);
    let h = k.measure(DimKind::HDist(p, r));
    let v = k.measure(DimKind::VDist(p, r));
    k.check_value(h, PI);
    k.check_value(v, 2.0);
}

/// cos(x) = x has no closed form (Dottie number): numeric block allowed.
#[test]
fn dottie_number() {
    let mut k = Kit::new();
    k.allow_inexact = true; // transcendental equation cos(x) = x
    let f = k.curve("y = cos(x)");
    let g = k.curve("y = x");
    let p = k.intersect(f, g, [0.7, 0.7]);
    let mut x = 0.7f64;
    for _ in 0..200 {
        x = x.cos();
    }
    let px = k.measure(DimKind::CoordX(p));
    k.check_value(px, x);
}

/// y = a x^2 with free coefficient a; tangent at x = 2 meets the x-axis at 1.
#[test]
fn coefficient_var_tangent_meets_axis() {
    let mut k = Kit::new();
    let a = k.var("a", 0.5);
    let f = k.curve("y = a*x^2");
    let p = k.point(2.1, 2.0);
    let q = k.point(1.1, 0.0);
    k.on(p, f);
    k.drive(DimKind::CoordX(p), "2");
    let xa = k.xaxis();
    k.on(q, xa);
    let pq = k.line(p, q);
    k.tangent(pq, f);
    let free = k.point(1.6, 0.9);
    k.on(free, pq);
    assert!(k.dof() > 0);
    let qx = k.measure(DimKind::CoordX(q));
    let py = k.measure(DimKind::CoordY(p));
    k.check_value(qx, 1.0);
    k.check_value(py, 2.0);
    k.set_var(a, 0.75);
    k.check_value(py, 3.0);
    k.check_value(qx, 1.0);
}

/// Parabola y = a x^2 + b x + c with solved coefficients fitted through three
/// dimensioned points; then vertex and roots are derived.
#[test]
fn quadratic_fit_solved_coefficients() {
    let mut k = Kit::new();
    k.solved_var("a", 1.5);
    k.solved_var("b", -2.0);
    k.solved_var("c", 0.5);
    let f = k.curve("y = a*x^2 + b*x + c");
    let pts = [(0.0, 1.0), (2.0, 3.0), (-1.0, 6.0)];
    for (x, y) in pts {
        let p = k.point(x + 0.05, y - 0.05);
        k.on(p, f);
        k.drive(DimKind::CoordX(p), &format!("{x}"));
        k.drive(DimKind::CoordY(p), &format!("{y}"));
    }
    assert_eq!(k.dof(), 0);
    // y = 2x^2 - 3x + 1: roots 1/2 and 1
    let xa = k.xaxis();
    let r1 = k.point(0.45, 0.05);
    let r2 = k.point(1.05, -0.05);
    k.on(r1, f);
    k.on(r1, xa);
    k.on(r2, f);
    k.on(r2, xa);
    let gap = k.measure(DimKind::Distance(r1, r2));
    k.check_value(gap, 0.5);
    // vertex via horizontal tangent
    let v = k.point(0.8, -0.1);
    let w = k.point(1.5, -0.125);
    k.on(v, f);
    let vw = k.line(v, w);
    k.horizontal(vw);
    k.tangent(vw, f);
    let vx = k.measure(DimKind::CoordX(v));
    let vy = k.measure(DimKind::CoordY(v));
    k.check_value(vx, 0.75);
    k.check_value(vy, -0.125);
}

/// y = k e^x fitted through (1, 3): k = 3/e, so the y-intercept is 3/e.
#[test]
fn exp_fit_solved_scale() {
    let mut k = Kit::new();
    k.solved_var("k", 1.0);
    let f = k.curve("y = k*exp(x)");
    let p = k.point(1.05, 2.9);
    k.on(p, f);
    k.drive(DimKind::CoordX(p), "1");
    k.drive(DimKind::CoordY(p), "3");
    let q = k.point(0.05, 1.0);
    let ya = k.yaxis();
    k.on(q, f);
    k.on(q, ya);
    assert_eq!(k.dof(), 0);
    let qy = k.measure(DimKind::CoordY(q));
    k.check_value(qy, 3.0 / E);
}

/// y = sin(w x) with w solved from a zero at x = 3 (3w = 2 pi, not pi);
/// the first maximum is then at x = pi / (2 w) = 3/4.
#[test]
fn sine_frequency_solved_nonprincipal_root() {
    let mut k = Kit::new();
    k.solved_var("w", 2.1);
    let f = k.curve("y = sin(w*x)");
    let xa = k.xaxis();
    let z = k.point(3.02, 0.02);
    k.on(z, f);
    k.on(z, xa);
    k.drive(DimKind::CoordX(z), "3");
    let m = k.point(0.7, 0.98);
    let n = k.point(1.5, 1.0);
    k.on(m, f);
    let mn = k.line(m, n);
    k.horizontal(mn);
    k.tangent(mn, f);
    assert!(k.dof() > 0);
    let mx = k.measure(DimKind::CoordX(m));
    let my = k.measure(DimKind::CoordY(m));
    k.check_value(mx, 0.75);
    k.check_value(my, 1.0);
}

/// Tangent from the origin to y = e^x touches at (1, e).
#[test]
fn exp_tangent_from_origin() {
    let mut k = Kit::new();
    let o = k.origin();
    let f = k.curve("y = exp(x)");
    let (seg, t) = k.tangent_line(o, f, [1.1, 3.0]);
    let free = k.point(0.5, 1.4);
    k.on(free, seg);
    assert!(k.dof() > 0);
    let tx = k.measure(DimKind::CoordX(t));
    let ty = k.measure(DimKind::CoordY(t));
    let len = k.measure(DimKind::Length(seg));
    k.check_value(tx, 1.0);
    k.check_value(ty, E);
    k.check_value(len, (1.0 + E * E).sqrt());
}

/// Tangent from the origin to y = ln x touches at (e, 1).
#[test]
fn ln_tangent_from_origin() {
    let mut k = Kit::new();
    let o = k.origin();
    let f = k.curve("y = ln(x)");
    let (seg, t) = k.tangent_line(o, f, [2.6, 0.95]);
    assert_eq!(k.dof(), 0);
    let tx = k.measure(DimKind::CoordX(t));
    let ty = k.measure(DimKind::CoordY(t));
    k.check_value(tx, E);
    k.check_value(ty, 1.0);
    let len = k.measure(DimKind::Length(seg));
    k.check_value(len, (E * E + 1.0).sqrt());
}

/// Tangent to y = 1/x at x = t cuts the axes at (2t, 0) and (0, 2/t).
#[test]
fn reciprocal_tangent_intercepts() {
    let mut k = Kit::new();
    let t = k.var("t", 2.0);
    let f = k.curve("y = 1/x");
    let p = k.point(2.1, 0.45);
    k.on(p, f);
    k.drive(DimKind::CoordX(p), "t");
    let a = k.point(3.9, 0.1);
    let b = k.point(0.1, 0.9);
    let xa = k.xaxis();
    let ya = k.yaxis();
    k.on(a, xa);
    k.on(b, ya);
    let ab = k.inf_line(a, b);
    k.on(p, ab);
    k.tangent(ab, f);
    assert_eq!(k.dof(), 0);
    let d = k.measure(DimKind::Distance(a, b));
    k.check_value(d, 17.0f64.sqrt());
    let ax = k.measure(DimKind::CoordX(a));
    k.check_value(ax, 4.0);
    k.set_var(t, 1.0);
    k.check_value(d, 2.0 * 2.0f64.sqrt());
    k.check_value(ax, 2.0);
}

/// Circle of radius 5 about the origin meets y = x^2: x^2 = (sqrt(101) - 1)/2.
#[test]
fn parabola_circle_intersection() {
    let mut k = Kit::new();
    let o = k.origin();
    let f = k.curve("y = x^2");
    let c = k.circle(o, 5.0);
    k.drive(DimKind::Radius(c), "5");
    let p = k.intersect(f, c, [2.0, 4.5]);
    let q = k.intersect(f, c, [-2.0, 4.5]);
    let on_circle = k.point(0.0, 5.0);
    k.on(on_circle, c);
    assert!(k.dof() > 0);
    let x2 = ((101.0f64).sqrt() - 1.0) / 2.0;
    let px = k.measure(DimKind::CoordX(p));
    k.check_value(px, x2.sqrt());
    let w = k.measure(DimKind::Distance(p, q));
    k.check_value(w, 2.0 * x2.sqrt());
    let py = k.measure(DimKind::CoordY(p));
    k.check_value(py, x2);
}

/// Implicit ellipse x^2 + 4 y^2 = 4: horizontal chord at y = 1/2.
#[test]
fn implicit_ellipse_chord() {
    let mut k = Kit::new();
    let e = k.curve("x^2 + 4*y^2 = 4");
    let p = k.point(1.6, 0.5);
    let q = k.point(-1.6, 0.5);
    k.on(p, e);
    k.on(q, e);
    let pq = k.line(p, q);
    k.horizontal(pq);
    k.drive(DimKind::CoordY(p), "0.5");
    assert_eq!(k.dof(), 0);
    let len = k.measure(DimKind::Length(pq));
    k.check_value(len, 2.0 * 3.0f64.sqrt());
}

/// Tilted ellipse x^2 + x y + y^2 = 3: horizontal tangent at (1, -2).
#[test]
fn rotated_ellipse_horizontal_tangent() {
    let mut k = Kit::new();
    let e = k.curve("x^2 + x*y + y^2 = 3");
    let p = k.point(0.9, -1.9);
    let q = k.point(2.0, -2.0);
    k.on(p, e);
    let pq = k.line(p, q);
    k.horizontal(pq);
    k.tangent(pq, e);
    assert!(k.dof() > 0);
    let x = k.measure(DimKind::CoordX(p));
    let y = k.measure(DimKind::CoordY(p));
    k.check_value(x, 1.0);
    k.check_value(y, -2.0);
}

/// Folium of Descartes x^3 + y^3 = 3xy: horizontal tangent at (2^(1/3), 2^(2/3)).
#[test]
fn folium_horizontal_tangent() {
    let mut k = Kit::new();
    let f = k.curve("x^3 + y^3 = 3*x*y");
    let p = k.point(1.2, 1.6);
    let q = k.point(2.0, 1.6);
    k.on(p, f);
    let pq = k.line(p, q);
    k.horizontal(pq);
    k.tangent(pq, f);
    assert!(k.dof() > 0);
    let x = k.measure(DimKind::CoordX(p));
    let y = k.measure(DimKind::CoordY(p));
    k.check_value(x, 2.0f64.powf(1.0 / 3.0));
    k.check_value(y, 2.0f64.powf(2.0 / 3.0));
}

/// Elliptic curve y^2 = x^3 - x: horizontal tangent at x = -1/sqrt(3).
#[test]
fn elliptic_curve_horizontal_tangent() {
    let mut k = Kit::new();
    let f = k.curve("y^2 = x^3 - x");
    let p = k.point(-0.55, 0.6);
    let q = k.point(0.5, 0.6);
    k.on(p, f);
    let pq = k.line(p, q);
    k.horizontal(pq);
    k.tangent(pq, f);
    assert!(k.dof() > 0);
    let x = k.measure(DimKind::CoordX(p));
    let y = k.measure(DimKind::CoordY(p));
    k.check_value(x, -1.0 / 3.0f64.sqrt());
    k.check_value(y, (2.0 / (3.0 * 3.0f64.sqrt())).sqrt());
}

/// The line y = x (free along its length) meets y = x^3 - 3x at +-2.
#[test]
fn diagonal_line_cubic_intersections() {
    let mut k = Kit::new();
    let o = k.origin();
    let f = k.curve("y = x^3 - 3*x");
    let b = k.point(1.0, 1.1);
    let l = k.inf_line(o, b);
    k.drive(DimKind::Angle(k.xaxis(), l), "45");
    assert!(k.dof() > 0);
    let p = k.intersect(l, f, [2.1, 2.0]);
    let q = k.intersect(l, f, [-2.1, -2.0]);
    let d = k.measure(DimKind::Distance(p, q));
    k.check_value(d, 4.0 * 2.0f64.sqrt());
}

/// Line through the origin and (1, m) meets y = x^3 - 3x at x = sqrt(3 + m).
#[test]
fn slope_variable_cubic_intersection() {
    let mut k = Kit::new();
    let o = k.origin();
    let m = k.var("m", 1.0);
    let f = k.curve("y = x^3 - 3*x");
    let a = k.point(1.0, 1.0);
    k.drive(DimKind::CoordX(a), "1");
    k.drive(DimKind::CoordY(a), "m");
    let l = k.inf_line(o, a);
    let p = k.intersect(l, f, [2.0, 2.0]);
    assert_eq!(k.dof(), 0);
    let px = k.measure(DimKind::CoordX(p));
    let py = k.measure(DimKind::CoordY(p));
    k.check_value(px, 2.0);
    k.check_value(py, 2.0);
    k.set_var(m, 6.0);
    k.check_value(px, 3.0);
    k.check_value(py, 18.0);
}

/// Witch of Agnesi y = 1/(1+x^2): chord at height h has half-width sqrt(1/h - 1).
#[test]
fn witch_of_agnesi_chord() {
    let mut k = Kit::new();
    let h = k.var("h", 0.5);
    let f = k.curve("y = 1/(1 + x^2)");
    let a = k.point(0.0, 0.5);
    k.drive(DimKind::CoordX(a), "0");
    let hl = k.point(2.0, 0.5);
    let l = k.inf_line(a, hl);
    k.horizontal(l);
    k.drive(DimKind::CoordY(a), "h");
    let p = k.intersect(l, f, [1.1, 0.5]);
    let q = k.intersect(l, f, [-1.1, 0.5]);
    assert!(k.dof() > 0);
    let w = k.measure(DimKind::Distance(p, q));
    k.check_value(w, 2.0);
    k.set_var(h, 0.2);
    k.check_value(w, 4.0);
}

/// y = 1/x meets the circle x^2 + y^2 = r^2 at two points, chord sqrt(2(r^2 - 2)).
#[test]
fn reciprocal_circle_chord() {
    let mut k = Kit::new();
    let o = k.origin();
    let r = k.var("r", 2.0);
    let f = k.curve("y = 1/x");
    let c = k.circle(o, 2.0);
    k.drive(DimKind::Radius(c), "r");
    let p = k.intersect(f, c, [1.9, 0.52]);
    let q = k.intersect(f, c, [0.52, 1.9]);
    let free = k.point(-2.0, 0.0);
    k.on(free, c);
    assert!(k.dof() > 0);
    let d = k.measure(DimKind::Distance(p, q));
    k.check_value(d, 2.0);
    k.set_var(r, 3.0);
    k.check_value(d, (2.0 * 7.0f64).sqrt());
}

/// Circle centered (0, 3) tangent to y = x^2 has radius sqrt(11)/2.
#[test]
fn circle_tangent_to_parabola() {
    let mut k = Kit::new();
    let f = k.curve("y = x^2");
    let c = k.fixed_point(0.0, 3.0);
    let circ = k.circle(c, 1.66);
    k.hint = [1.58, 2.5]; // where the contact point is picked
    k.tangent(circ, f);
    let free = k.point(1.6, 3.0);
    k.on(free, circ);
    assert!(k.dof() > 0);
    let r = k.measure(DimKind::Radius(circ));
    k.check_value(r, 11.0f64.sqrt() / 2.0);
}

/// Two tangents from (0, -1) to y = x^2 enclose an angle 2 atan(1/2).
#[test]
fn two_tangents_angle() {
    let mut k = Kit::new();
    let f = k.curve("y = x^2");
    let a = k.fixed_point(0.0, -1.0);
    let (s1, t1) = k.tangent_line(a, f, [1.1, 1.0]);
    let (s2, t2) = k.tangent_line(a, f, [-1.1, 1.0]);
    let free = k.point(0.5, -0.5);
    k.on(free, s1);
    assert!(k.dof() > 0);
    let ang = k.measure(DimKind::Angle(s1, s2));
    println!("angle {}", k.value(ang));
    let v = k.value(ang);
    let want = 2.0 * 0.5f64.atan();
    let want = if (v - want).abs() < 1e-6 { want } else { PI - want };
    k.check_value(ang, want);
    let chord = k.measure(DimKind::Distance(t1, t2));
    k.check_value(chord, 2.0);
}

/// Normal to y = x^2 at x = t meets the y-axis at t^2 + 1/2.
#[test]
fn parabola_normal_meets_axis() {
    let mut k = Kit::new();
    let t = k.var("t", 1.0);
    let f = k.curve("y = x^2");
    let p = k.point(1.05, 0.95);
    let q = k.point(0.5, 0.0);
    k.on(p, f);
    k.drive(DimKind::CoordX(p), "t");
    let pq = k.line(p, q);
    k.tangent(pq, f);
    let nl = k.perp_line(pq, p);
    let ya = k.yaxis();
    let n = k.intersect(nl, ya, [0.0, 1.4]);
    assert!(k.dof() > 0);
    let ny = k.measure(DimKind::CoordY(n));
    k.check_value(ny, 1.5);
    k.set_var(t, 2.0);
    k.check_value(ny, 4.5);
}

/// y = sin x has an inflection at the origin with slope 1.
#[test]
fn sine_inflection_tangent_angle() {
    let mut k = Kit::new();
    let f = k.curve("y = sin(x)");
    let p = k.point(0.05, 0.04);
    k.on(p, f);
    k.drive(DimKind::CoordX(p), "0");
    let q = k.point(1.0, 0.9);
    let pq = k.line(p, q);
    k.tangent(pq, f);
    k.drive(DimKind::Length(pq), "3");
    let dx = k.measure(DimKind::HDist(p, q));
    k.check_value(dx, 3.0 / 2.0f64.sqrt());
    let dy = k.measure(DimKind::VDist(p, q));
    k.check_value(dy, 3.0 / 2.0f64.sqrt());
}

/// Line through the origin tangent to sin x: only the inflection tangent y = x
/// has contact there; the constraint is valid but degenerate (order-3 contact:
/// the contact point is a triple root of sin t = t·cos t, known numerically
/// only to about 1e-5, and recognized as t = 0).
#[test]
fn sine_inflection_tangent_unpinned() {
    let mut k = Kit::new();
    let o = k.origin();
    let f = k.curve("y = sin(x)");
    let q = k.point(1.0, 0.9);
    let oq = k.line(o, q);
    k.tangent(oq, f);
    k.drive(DimKind::Length(oq), "3");
    let dx = k.measure(DimKind::HDist(o, q));
    k.check_value(dx, 3.0 / 2.0f64.sqrt());
}

/// A segment at 45 degrees tangent to y = x^2 touches at (1/2, 1/4).
#[test]
fn parabola_tangent_with_given_slope() {
    let mut k = Kit::new();
    let f = k.curve("y = x^2");
    let p = k.point(0.55, 0.3);
    let q = k.point(1.5, 1.2);
    k.on(p, f);
    let pq = k.line(p, q);
    k.tangent(pq, f);
    k.drive(DimKind::Angle(k.xaxis(), pq), "45");
    assert!(k.dof() > 0);
    let x = k.measure(DimKind::CoordX(p));
    let y = k.measure(DimKind::CoordY(p));
    k.check_value(x, 0.5);
    k.check_value(y, 0.25);
}

/// Gaussian y = exp(-x^2) cut at height 1/2: width 2 sqrt(ln 2).
#[test]
fn gaussian_width_at_half_height() {
    let mut k = Kit::new();
    let f = k.curve("y = exp(-x^2)");
    let a = k.fixed_point(0.0, 0.5);
    let b = k.point(1.0, 0.5);
    let l = k.inf_line(a, b);
    k.horizontal(l);
    let p = k.intersect(l, f, [0.8, 0.5]);
    let q = k.intersect(l, f, [-0.8, 0.5]);
    assert!(k.dof() > 0);
    let w = k.measure(DimKind::Distance(p, q));
    k.check_value(w, 2.0 * 2.0f64.ln().sqrt());
}

/// y = A sin(w x): first maximum at x = pi/(2w), height A (both variables).
#[test]
fn amplitude_frequency_variables_peak() {
    let mut k = Kit::new();
    let am = k.var("A", 2.0);
    let w = k.var("w", 1.0);
    let f = k.curve("y = A*sin(w*x)");
    let p = k.point(1.5, 1.9);
    let q = k.point(2.5, 2.0);
    k.on(p, f);
    let pq = k.line(p, q);
    k.horizontal(pq);
    k.tangent(pq, f);
    assert!(k.dof() > 0);
    let x = k.measure(DimKind::CoordX(p));
    let y = k.measure(DimKind::CoordY(p));
    k.check_value(x, PI / 2.0);
    k.check_value(y, 2.0);
    k.set_var(w, 1.5);
    k.check_value(x, PI / 3.0);
    k.set_var(am, 3.0);
    k.check_value(y, 3.0);
}

/// Same construction with a larger frequency step: after w goes 1 -> 2 the
/// solver reports success but the horizontal tangent at the peak is violated.
#[test]
fn amplitude_frequency_variables_peak_big_step() {
    let mut k = Kit::new();
    let _am = k.var("A", 2.0);
    let w = k.var("w", 1.0);
    let f = k.curve("y = A*sin(w*x)");
    let p = k.point(1.5, 1.9);
    let q = k.point(2.5, 2.0);
    k.on(p, f);
    let pq = k.line(p, q);
    k.horizontal(pq);
    k.tangent(pq, f);
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, PI / 2.0);
    k.set_var(w, 2.0);
    k.check_value(x, PI / 4.0);
}

/// Point on y = cos(3x) at height c: x = acos(c)/3.
#[test]
fn cos3x_curve_height() {
    let mut k = Kit::new();
    k.var("c", 0.3);
    let f = k.curve("y = 4*cos(x)^3 - 3*cos(x)");
    let p = k.point(0.42, 0.3);
    k.on(p, f);
    k.drive(DimKind::CoordY(p), "c");
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, 0.3f64.acos() / 3.0);
}

/// Point on y = sin(x)^3 - sin(x) at height c (a genuine cubic in sin x,
/// three real roots): x = asin(2/√3 cos(acos(...)/3 ...)).
#[test]
fn cubic_in_sin_curve_height() {
    let mut k = Kit::new();
    k.var("c", -0.2);
    let f = k.curve("y = sin(x)^3 - sin(x)");
    // s^3 - s + 0.2 = 0 root near 0.2
    let mut s = 0.2f64;
    for _ in 0..50 {
        s -= (s * s * s - s + 0.2) / (3.0 * s * s - 1.0);
    }
    let x0 = s.asin();
    let p = k.point(x0 + 0.01, -0.2);
    k.on(p, f);
    k.drive(DimKind::CoordY(p), "c");
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, x0);
}

fn height_probe(src: &str, c: f64, x0: f64, vars: &[(&str, f64)]) {
    let mut k = Kit::new();
    for (n, v) in vars {
        k.var(n, *v);
    }
    k.var("c", c);
    let f = k.curve(src);
    let p = k.point(x0 + 0.01, c);
    k.on(p, f);
    k.drive(DimKind::CoordY(p), "c");
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, x0);
}

/// y = a cos x + b sin x at height c, symbolic a, b, c, a root in each quadrant range.
#[test]
fn harmonic_symbolic_all_quadrants() {
    let (a, b, c) = (1.0f64, 2.0f64, 0.7f64);
    let f = |x: f64| a * x.cos() + b * x.sin() - c;
    let r = (a * a + b * b).sqrt();
    let phi = b.atan2(a);
    let w = (c / r).acos();
    for x0 in [phi + w, phi - w, phi + w - 2.0 * PI, phi - w + 2.0 * PI] {
        assert!(f(x0).abs() < 1e-12);
        height_probe("y = a*cos(x) + b*sin(x)", c, x0, &[("a", a), ("b", b)]);
    }
}

/// y = cos x + sin 2x at height c: a quartic in sin x (mixed harmonics).
/// Horizontal tangent: -sin x + 2 cos 2x = 0 → sin x = (√33 − 1)/8.
#[test]
fn mixed_harmonic_horizontal_tangent() {
    let mut k = Kit::new();
    let f = k.curve("y = cos(x) + sin(2*x)");
    let x0 = ((33.0f64.sqrt() - 1.0) / 8.0).asin();
    let p = k.point(x0 + 0.02, 1.7);
    let q = k.point(x0 + 1.0, 1.7);
    k.on(p, f);
    let pq = k.line(p, q);
    k.horizontal(pq);
    k.tangent(pq, f);
    let x = k.measure(DimKind::CoordX(p));
    let y = k.measure(DimKind::CoordY(p));
    k.check_value(x, x0);
    k.check_value(y, x0.cos() + (2.0 * x0).sin());
}

fn newton(f: impl Fn(f64) -> f64, mut x: f64) -> f64 {
    for _ in 0..100 {
        let h = 1e-7;
        let d = (f(x + h) - f(x - h)) / (2.0 * h);
        x -= f(x) / d;
    }
    x
}

/// y = cos x + sin 2x at height 1: cos x(1 + 2 sin x) = 1 → quartic in sin x
/// that factors? x = 0 is a root; take a nontrivial root.
#[test]
fn mixed_harmonic_height() {
    let c = 1.2;
    let x0 = newton(|x| x.cos() + (2.0 * x).sin() - c, 0.3);
    height_probe("y = cos(x) + sin(2*x)", c, x0, &[]);
}

/// Even-quadratic form: y = cos(x)^2 + 2 sin(x) cos(x) at height c:
/// (1 + cos 2x)/2 + sin 2x = c.
#[test]
fn even_quadratic_trig_height() {
    let c = 1.1;
    let x0 = newton(|x| x.cos().powi(2) + 2.0 * x.sin() * x.cos() - c, 0.3);
    height_probe("y = cos(x)^2 + 2*sin(x)*cos(x)", c, x0, &[]);
}

/// y = tan(x) at height c on the branch x ∈ (π/2, 3π/2): x = atan(c) + π.
#[test]
fn tan_other_branch_height() {
    let c = 0.8f64;
    height_probe("y = tan(x)", c, c.atan() + PI, &[]);
}

/// y = 1/tan(x) (cot) at height c: x = atan(1/c).
#[test]
fn cot_height() {
    let c = 0.8f64;
    height_probe("y = 1/tan(x)", c, (1.0 / c).atan(), &[]);
}

/// y = sin(x) at height c on a far branch x ≈ 2π + π − asin c.
#[test]
fn sin_far_branch_height() {
    let c = 0.4f64;
    height_probe("y = sin(x)", c, 3.0 * PI - c.asin(), &[]);
}

/// y = sin(x)/cos(x)^2? no: y = sin(x) + cos(x)^2 at height c (quadratic in sin x).
#[test]
fn sin_plus_cos2_height() {
    let c = 1.1f64;
    // s + 1 - s^2 = c → s^2 - s + (c-1) = 0, s = (1 - √(1 - 4(c-1)))/2... take s = (1+√(5-4c))/2
    let s = (1.0 - (5.0 - 4.0 * c).sqrt()) / 2.0 * -1.0 + 0.0;
    let s = if s.abs() <= 1.0 { s } else { (1.0 - (5.0 - 4.0 * c).sqrt()) / 2.0 };
    let s2 = (1.0 + (5.0f64 - 4.0 * c).sqrt()) / 2.0;
    let _ = s;
    height_probe("y = sin(x) + cos(x)^2", c, s2.asin(), &[]);
}

/// y = e^x + e^(-x) at height c: x = ln((c + √(c² − 4))/2).
#[test]
fn exp_sum_height() {
    let c = 3.0f64;
    height_probe("y = exp(x) + exp(-x)", c, ((c + (c * c - 4.0).sqrt()) / 2.0).ln(), &[]);
}

/// Negative branch of the same.
#[test]
fn exp_sum_height_negative_branch() {
    let c = 3.0f64;
    height_probe("y = exp(x) + exp(-x)", c, -((c + (c * c - 4.0).sqrt()) / 2.0).ln(), &[]);
}

/// y = sinh(x) at height c: asinh(c).
#[test]
fn sinh_height() {
    let c = 1.5f64;
    height_probe("y = sinh(x)", c, c.asinh(), &[]);
}

/// y = cosh(x) at height c on the negative branch.
#[test]
fn cosh_height_negative() {
    let c = 2.0f64;
    height_probe("y = cosh(x)", c, -c.acosh(), &[]);
}

/// y = exp(2x) − exp(x) at height c: e^x = (1 + √(1 + 4c))/2.
#[test]
fn exp_quadratic_height() {
    let c = 2.5f64;
    height_probe("y = exp(2*x) - exp(x)", c, ((1.0 + (1.0 + 4.0 * c).sqrt()) / 2.0).ln(), &[]);
}

/// y = 2^x at height c: x = ln c / ln 2.
#[test]
fn power_base_height() {
    let c = 5.0f64;
    height_probe("y = 2^x", c, c.ln() / 2.0f64.ln(), &[]);
}

/// y = ln(x) + ln(x + 1) at height c: x(x+1) = e^c.
#[test]
fn log_sum_height() {
    let c = 1.0f64;
    height_probe("y = ln(x) + ln(x + 1)", c, (-1.0 + (1.0 + 4.0 * c.exp()).sqrt()) / 2.0, &[]);
}

fn min_probe(src: &str, x0: f64, y0: f64) {
    let mut k = Kit::new();
    let f = k.curve(src);
    let p = k.point(x0 + 0.03, y0);
    let q = k.point(x0 + 1.0, y0);
    k.on(p, f);
    let pq = k.line(p, q);
    k.horizontal(pq);
    k.tangent(pq, f);
    let x = k.measure(DimKind::CoordX(p));
    let y = k.measure(DimKind::CoordY(p));
    k.check_value(x, x0);
    k.check_value(y, y0);
}

/// Minimum of x e^x at x = −1, y = −1/e.
#[test]
fn x_exp_x_minimum() {
    min_probe("y = x*exp(x)", -1.0, -1.0 / E);
}

/// Minimum of x ln x at x = 1/e.
#[test]
fn x_ln_x_minimum() {
    min_probe("y = x*ln(x)", 1.0 / E, -1.0 / E);
}

/// Minimum of e^x / x at x = 1.
#[test]
fn exp_over_x_minimum() {
    min_probe("y = exp(x)/x", 1.0, E);
}

/// Minimum of x^x at x = 1/e.
#[test]
fn x_pow_x_minimum() {
    min_probe("y = x^x", 1.0 / E, (-1.0 / E).exp());
}

/// Maximum of e^x sin x at x = 3π/4.
#[test]
fn exp_sin_extremum() {
    let x0 = 0.75 * PI;
    min_probe("y = exp(x)*sin(x)", x0, x0.exp() * x0.sin());
}

/// Maximum of ln(x)/x at x = e.
#[test]
fn ln_over_x_maximum() {
    min_probe("y = ln(x)/x", E, 1.0 / E);
}

/// Tangent from (0, b) to y = ln x touches at x = e^(b+1).
#[test]
fn ln_tangent_from_axis_point_var() {
    let mut k = Kit::new();
    k.var("b", 0.5);
    let f = k.curve("y = ln(x)");
    let a = k.point(0.0, 0.5);
    k.drive(DimKind::CoordX(a), "0");
    k.drive(DimKind::CoordY(a), "b");
    let x0 = 1.5f64.exp();
    k.hint = [x0, 1.5];
    let (_seg, t) = k.tangent_line(a, f, [x0 * 1.05, 1.5]);
    let tx = k.measure(DimKind::CoordX(t));
    k.check_value(tx, x0);
}

/// Tangent from (a, 0) to y = e^x touches at x = a + 1.
#[test]
fn exp_tangent_from_axis_point_var() {
    let mut k = Kit::new();
    k.var("a", 0.5);
    let f = k.curve("y = exp(x)");
    let p = k.point(0.5, 0.0);
    k.drive(DimKind::CoordX(p), "a");
    k.drive(DimKind::CoordY(p), "0");
    let (_seg, t) = k.tangent_line(p, f, [1.55, 4.4]);
    let ty = k.measure(DimKind::CoordY(t));
    k.check_value(ty, 1.5f64.exp());
}

/// Intersection of y = e^x and y = e^(−x) + 1: e^x = φ (golden ratio).
#[test]
fn exp_curves_intersection() {
    let mut k = Kit::new();
    let f = k.curve("y = exp(x)");
    let g = k.curve("y = exp(-x) + 1");
    let phi = (1.0 + 5.0f64.sqrt()) / 2.0;
    let p = k.intersect(f, g, [phi.ln() + 0.02, phi]);
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, phi.ln());
}

/// Intersection of y = sin x and y = cos x on the far branch x = 5π/4.
#[test]
fn sin_cos_intersection_far() {
    let mut k = Kit::new();
    let f = k.curve("y = sin(x)");
    let g = k.curve("y = cos(x)");
    let p = k.intersect(f, g, [3.95, -0.7]);
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, 1.25 * PI);
}

/// Intersection of y = sin(2x) and y = sin(x): 2 sin x cos x = sin x,
/// cos x = 1/2 → x = π/3.
#[test]
fn sin2x_sinx_intersection() {
    let mut k = Kit::new();
    let f = k.curve("y = sin(2*x)");
    let g = k.curve("y = sin(x)");
    let p = k.intersect(f, g, [1.0, 0.86]);
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, PI / 3.0);
}

/// Intersection of y = a sin(x) and y = cos(2x) with var a:
/// 1 − 2s² = a s → s = (−a + √(a² + 8))/4.
#[test]
fn sin_cos2x_intersection_var() {
    let mut k = Kit::new();
    k.var("a", 1.0);
    let f = k.curve("y = a*sin(x)");
    let g = k.curve("y = cos(2*x)");
    let s = (-1.0 + 9.0f64.sqrt()) / 4.0;
    let p = k.intersect(f, g, [s.asin() + 0.02, s]);
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, s.asin());
}

fn rad(d: f64) -> f64 {
    d.to_radians()
}

/// Tangent to y = e^x making angle θ with the x axis: x = ln(tan θ).
#[test]
fn exp_tangent_given_angle() {
    let mut k = Kit::new();
    let f = k.curve("y = exp(x)");
    let xa = k.xaxis();
    let x0 = rad(60.0).tan().ln();
    let p = k.point(x0 + 0.02, x0.exp());
    let q = k.point(x0 + 1.0, x0.exp() + 1.8);
    k.on(p, f);
    let pq = k.line(p, q);
    k.tangent(pq, f);
    k.var("w", 60.0);
    k.drive_angle(xa, pq, "w*deg");
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, x0);
}

/// Tangent to y = sin x making angle θ with the x axis: x = acos(tan θ).
#[test]
fn sin_tangent_given_angle() {
    let mut k = Kit::new();
    let f = k.curve("y = sin(x)");
    let xa = k.xaxis();
    let x0 = rad(30.0).tan().acos();
    let p = k.point(x0 + 0.02, x0.sin());
    let q = k.point(x0 + 1.0, x0.sin() + 0.577);
    k.on(p, f);
    let pq = k.line(p, q);
    k.tangent(pq, f);
    k.var("w", 30.0);
    k.drive_angle(xa, pq, "w*deg");
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, x0);
}

/// Intersection of y = sin(x + a) and y = cos(x): x = (π/2 − a)/2.
#[test]
fn shifted_sin_cos_intersection() {
    let mut k = Kit::new();
    k.var("a", 0.4);
    let f = k.curve("y = sin(x + a)");
    let g = k.curve("y = cos(x)");
    let x0 = (PI / 2.0 - 0.4) / 2.0;
    let p = k.intersect(f, g, [x0 + 0.02, x0.cos()]);
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, x0);
}

/// y = sin(x/2) + cos(x) at height c: with s = sin(x/2), s + 1 − 2s² = c.
#[test]
fn half_angle_curve_height() {
    let c = 1.05f64;
    let s = (1.0 + (1.0f64 + 8.0 * (1.0 - c)).sqrt()) / 4.0;
    height_probe("y = sin(x/2) + cos(x)", c, 2.0 * s.asin(), &[]);
}

/// ln(c − x) = ln(x): x = c/2.
#[test]
fn ln_equal_intersection() {
    let mut k = Kit::new();
    k.var("c", 3.0);
    let f = k.curve("y = ln(c - x)");
    let g = k.curve("y = ln(x)");
    let p = k.intersect(f, g, [1.55, 0.4]);
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, 1.5);
}

/// y = acos(x) meets y = asin(x) at x = √2/2.
#[test]
fn acos_asin_intersection() {
    let mut k = Kit::new();
    let f = k.curve("y = acos(x)");
    let g = k.curve("y = asin(x)");
    let p = k.intersect(f, g, [0.72, 0.78]);
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, 0.5f64.sqrt());
}

/// y = asin(x) at height c: x = sin c.
#[test]
fn asin_curve_height() {
    height_probe("y = asin(x)", 0.6, 0.6f64.sin(), &[]);
}

/// y = atan(x) with a tangent at angle w to the x axis: 1/(1+x²) = tan w.
#[test]
fn atan_tangent_given_angle() {
    let mut k = Kit::new();
    let f = k.curve("y = atan(x)");
    let xa = k.xaxis();
    let w = rad(30.0);
    let x0 = (1.0 / w.tan() - 1.0).sqrt();
    let p = k.point(x0 + 0.02, x0.atan());
    let q = k.point(x0 + 1.0, x0.atan() + w.tan());
    k.on(p, f);
    let pq = k.line(p, q);
    k.tangent(pq, f);
    k.var("w", 30.0);
    k.drive_angle(xa, pq, "w*deg");
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, x0);
}

/// Tangent to y = tan x at angle w: cos² x = 1/tan w (here w = 70°).
#[test]
fn tan_tangent_given_angle() {
    let mut k = Kit::new();
    let f = k.curve("y = tan(x)");
    let xa = k.xaxis();
    let w = rad(70.0);
    let x0 = (1.0 / w.tan().sqrt()).acos();
    let p = k.point(x0 + 0.02, x0.tan());
    let q = k.point(x0 + 0.5, x0.tan() + 0.5 * w.tan());
    k.on(p, f);
    let pq = k.line(p, q);
    k.tangent(pq, f);
    k.var("w", 70.0);
    k.drive_angle(xa, pq, "w*deg");
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, x0);
}

fn meet_probe(f_src: &str, g_src: &str, x0: f64, y0: f64, vars: &[(&str, f64)]) {
    let mut k = Kit::new();
    for (n, v) in vars {
        k.var(n, *v);
    }
    let f = k.curve(f_src);
    let g = k.curve(g_src);
    let p = k.intersect(f, g, [x0 + 0.02, y0]);
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, x0);
}

/// sin(2x) = a sin(x) with var a: cos x = a/2 (the a = 1 constant case passes).
#[test]
fn sin2x_a_sinx_intersection_var() {
    let x0 = (0.6f64).acos();
    meet_probe("y = sin(2*x)", "y = a*sin(x)", x0, 1.2 * x0.sin(), &[("a", 1.2)]);
}

/// a sin x = cos x: x = atan(1/a) (plain harmonic, var).
#[test]
fn a_sin_cos_intersection_var() {
    let x0 = (1.0f64 / 1.3).atan();
    meet_probe("y = a*sin(x)", "y = cos(x)", x0, x0.cos(), &[("a", 1.3)]);
}

/// tan x = a sin x: cos x = 1/a.
#[test]
fn tan_a_sin_intersection_var() {
    let x0 = (1.0f64 / 2.5).acos();
    meet_probe("y = tan(x)", "y = a*sin(x)", x0, x0.tan(), &[("a", 2.5)]);
}

/// a sin x = cos² x: s² + a s − 1 = 0.
#[test]
fn sin_cos_squared_intersection_var() {
    let a = 0.7f64;
    let s = (-a + (a * a + 4.0).sqrt()) / 2.0;
    meet_probe("y = a*sin(x)", "y = cos(x)^2", s.asin(), a * s, &[("a", a)]);
}

/// Implicit sin x + sin y = 1, point with x = c: y = asin(1 − sin c).
#[test]
fn implicit_sin_sum_curve() {
    let mut k = Kit::new();
    k.var("c", 0.4);
    let f = k.curve("sin(x) + sin(y) = 1");
    let y0 = (1.0 - 0.4f64.sin()).asin();
    let p = k.point(0.42, y0 + 0.01);
    k.on(p, f);
    k.drive(DimKind::CoordX(p), "c");
    let y = k.measure(DimKind::CoordY(p));
    k.check_value(y, y0);
}

/// Minimum of x − a sin x: cos x = 1/a → x = acos(1/a).
#[test]
fn x_minus_a_sin_minimum() {
    let mut k = Kit::new();
    k.var("a", 2.0);
    let f = k.curve("y = x - a*sin(x)");
    let x0 = PI / 3.0;
    let y0 = x0 - 2.0 * x0.sin();
    let p = k.point(x0 + 0.03, y0);
    let q = k.point(x0 + 1.0, y0);
    k.on(p, f);
    let pq = k.line(p, q);
    k.horizontal(pq);
    k.tangent(pq, f);
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, x0);
}

/// Maximum of x e^(−x²) at x = 1/√2.
#[test]
fn gaussian_x_maximum() {
    let x0 = 0.5f64.sqrt();
    min_probe("y = x*exp(-x^2)", x0, x0 * (-0.5f64).exp());
}

/// Tangent to y = ln x at angle w: x = cot w.
#[test]
fn ln_tangent_given_angle() {
    let mut k = Kit::new();
    let f = k.curve("y = ln(x)");
    let xa = k.xaxis();
    let w = rad(35.0);
    let x0 = 1.0 / w.tan();
    let p = k.point(x0 + 0.02, x0.ln());
    let q = k.point(x0 + 1.0, x0.ln() + w.tan());
    k.on(p, f);
    let pq = k.line(p, q);
    k.tangent(pq, f);
    k.var("w", 35.0);
    k.drive_angle(xa, pq, "w*deg");
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, x0);
}

/// Expression layer: exp(x)·exp(−x) should be 1.
#[test]
fn expr_exp_product_cancels() {
    use sggeo::expr::*;
    let x = sym(7);
    let e = mul(vec![fun(Fun::Exp, x.clone()), fun(Fun::Exp, neg(x.clone()))]);
    let b = deep_best(&e);
    println!("exp(x)*exp(-x) -> {:?} / deep_best {:?}", e, b);
    assert!(b.is_one(), "not cancelled: {b:?}");
}
