//! Tests driven by variables: expressions of free variables, dimension names in
//! expressions, defined variables and solved variables.

use sggeo::model::DimKind;
use sggeo::ops;
mod testkit;
use testkit::Kit;
use std::f64::consts::PI;

fn nm(k: &Kit, d: usize) -> String {
    k.doc().dims[d].as_ref().unwrap().name.clone()
}

/// Polar to Cartesian with sqrt/atan2 expressions of two variables.
#[test]
fn polar_from_cartesian_vars() {
    let mut k = Kit::new();
    let o = k.origin();
    k.var("a", 3.0);
    k.var("b", 4.0);
    let p = k.point(2.5, 3.5);
    let s = k.line(o, p);
    k.drive(DimKind::Length(s), "sqrt(a^2 + b^2)");
    k.drive(DimKind::Angle(k.xaxis(), s), "atan2(b, a)");
    assert_eq!(k.dof(), 0);
    let x = k.measure(DimKind::CoordX(p));
    let y = k.measure(DimKind::CoordY(p));
    k.check_value(x, 3.0);
    k.check_value(y, 4.0);
}

/// Angle given with the degree postfix on a variable.
#[test]
fn degree_postfix_variable() {
    let mut k = Kit::new();
    let o = k.origin();
    let a = k.var("a", 40.0);
    let p = k.point(1.5, 2.0);
    let s = k.line(o, p);
    k.drive(DimKind::Length(s), "5");
    k.drive(DimKind::Angle(k.xaxis(), s), "a°");
    let x = k.measure(DimKind::CoordX(p));
    let y = k.measure(DimKind::CoordY(p));
    k.check_value(x, 5.0 * 40f64.to_radians().cos());
    k.check_value(y, 5.0 * 40f64.to_radians().sin());
    k.set_var(a, 55.0);
    k.check_value(x, 5.0 * 55f64.to_radians().cos());
    k.check_value(y, 5.0 * 55f64.to_radians().sin());
}

/// Angle given as `a*deg` and a coordinate driven by a trig expression.
#[test]
fn deg_constant_and_trig_coordinate() {
    let mut k = Kit::new();
    let o = k.origin();
    let a = k.var("a", 30.0);
    let p = k.point(1.0, 2.0);
    let s = k.line(o, p);
    k.drive(DimKind::Angle(k.xaxis(), s), "2*a*deg");
    k.drive(DimKind::CoordY(p), "3*sin(a*deg)");
    assert_eq!(k.dof(), 0);
    // angle 60°, y = 1.5 -> x = y / tan 60°
    let x = k.measure(DimKind::CoordX(p));
    let l = k.measure(DimKind::Length(s));
    k.check_value(x, 1.5 / 60f64.to_radians().tan());
    k.check_value(l, 1.5 / 60f64.to_radians().sin());
    k.set_var(a, 20.0);
    k.check_value(l, 3.0 * 20f64.to_radians().sin() / 40f64.to_radians().sin());
}

/// Floating rectangle: the second side is driven by the name of the first.
#[test]
fn rectangle_dimension_names() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(3.0, 0.1);
    let c = k.point(3.1, 2.0);
    let d = k.point(0.1, 1.9);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let cd = k.line(c, d);
    let da = k.line(d, a);
    k.perpendicular(ab, bc);
    k.perpendicular(bc, cd);
    k.perpendicular(cd, da);
    let w = k.drive(DimKind::Length(ab), "3");
    let wn = nm(&k, w);
    k.drive(DimKind::Length(bc), &format!("{wn}*3/2"));
    assert_eq!(k.dof(), 3);
    let diag = k.measure(DimKind::Distance(a, c));
    k.check_value(diag, (9.0f64 + 20.25).sqrt());
}

/// Right triangle floating; second leg `d1 + 1`, hypotenuse by Pythagoras.
#[test]
fn right_triangle_names() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(3.0, 0.2);
    let c = k.point(0.1, 4.2);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.perpendicular(ab, ac);
    let d1 = k.drive(DimKind::Length(ab), "3");
    let n1 = nm(&k, d1);
    k.drive(DimKind::Length(ac), &format!("{n1} + 1"));
    assert!(k.dof() > 0);
    let h = k.measure(DimKind::Length(bc));
    k.check_value(h, 5.0);
    k.set(d1, "6");
    k.check_value(h, (36.0f64 + 49.0).sqrt());
}

/// Defined variable: area of a rectangle feeds a circle radius.
#[test]
fn defvar_area_to_radius() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(4.0, 0.0);
    let c = k.point(4.0, 3.0);
    let d = k.point(0.0, 3.0);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let cd = k.line(c, d);
    let da = k.line(d, a);
    k.horizontal(ab);
    k.horizontal(cd);
    k.vertical(bc);
    k.vertical(da);
    let w = k.drive(DimKind::Length(ab), "4");
    let h = k.drive(DimKind::Length(bc), "3");
    let (wn, hn) = (nm(&k, w), nm(&k, h));
    k.defvar("A", &format!("{wn}*{hn}"));
    let q = k.point(10.0, 10.0);
    let circ = k.circle(q, 2.0);
    k.drive(DimKind::Radius(circ), "sqrt(A/pi)");
    assert!(k.dof() > 0);
    let dia = k.measure(DimKind::Diameter(circ));
    k.check_value(dia, 2.0 * (12.0 / PI).sqrt());
    k.set(w, "5");
    k.check_value(dia, 2.0 * (15.0 / PI).sqrt());
}

/// Defined variables chained: b = a + 1, c = b^2, side driven by c.
#[test]
fn defvar_chain() {
    let mut k = Kit::new();
    let o = k.origin();
    let a = k.var("a", 2.0);
    k.defvar("b", "a + 1");
    k.defvar("c", "b^2 - a");
    let p = k.point(5.0, 0.5);
    let s = k.line(o, p);
    k.horizontal(s);
    k.drive(DimKind::Length(s), "c");
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, 7.0);
    k.set_var(a, 3.0);
    k.check_value(x, 13.0);
}

/// Defined variable combining perimeter ratio of a floating triangle
/// drives an angle.
#[test]
fn defvar_perimeter_ratio() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(4.0, 0.0);
    let c = k.point(4.0, 3.0);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let ca = k.line(c, a);
    let d1 = k.drive(DimKind::Length(ab), "4");
    let d2 = k.drive(DimKind::Length(bc), "3");
    let d3 = k.drive(DimKind::Length(ca), "5");
    let (n1, n2, n3) = (nm(&k, d1), nm(&k, d2), nm(&k, d3));
    k.defvar("P", &format!("{n1}+{n2}+{n3}"));
    k.defvar("ratio", &format!("{n2}/P"));
    assert_eq!(k.dof(), 3);
    // a second triangle, angle at its corner is asin(ratio)
    let p = k.point(10.0, 0.0);
    let q = k.point(14.0, 0.0);
    let r = k.point(14.0, 5.0);
    let pq = k.line(p, q);
    let qr = k.line(q, r);
    let rp = k.line(r, p);
    k.perpendicular(pq, qr);
    k.drive(DimKind::Length(pq), "4");
    let ang = k.drive(DimKind::Angle(pq, rp), "asin(ratio)");
    let _ = ang;
    let rr = k.measure(DimKind::Length(qr));
    k.check_value(rr, 4.0 * (0.25f64).asin().tan());
}

/// Solved variable determined by Pythagoras: 9 + s^2 = (s+1)^2, s = 4.
#[test]
fn solved_var_pythagoras() {
    let mut k = Kit::new();
    k.solved_var("s", 3.0);
    let a = k.point(0.0, 0.0);
    let b = k.point(3.0, 0.1);
    let c = k.point(0.1, 3.8);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.perpendicular(ab, ac);
    k.drive(DimKind::Length(ab), "3");
    k.drive(DimKind::Length(ac), "s");
    k.drive(DimKind::Length(bc), "s + 1");
    assert!(k.dof() > 0);
    let ang = k.measure(DimKind::Angle(ab, bc));
    // the angle sector chosen by the label is the supplement of the interior angle at B
    k.check_value(ang, PI - (4.0f64 / 3.0).atan());
}

/// Solved variable in a curve source: y = s x^2 through a given point.
#[test]
fn solved_var_in_curve() {
    let mut k = Kit::new();
    k.solved_var("s", 0.5);
    let f = k.curve("y = s*x^2");
    let p = k.point(2.0, 3.0);
    let q = k.point(-4.0, 10.0);
    k.on(p, f);
    k.on(q, f);
    k.drive(DimKind::CoordX(p), "2");
    k.drive(DimKind::CoordY(p), "3");
    k.drive(DimKind::CoordX(q), "-4");
    assert_eq!(k.dof(), 0);
    let y = k.measure(DimKind::CoordY(q));
    k.check_value(y, 0.75 * 16.0);
}

/// Solved variable linking two lengths on a line: AB = u, BC = u/2, AC = 7.
#[test]
fn solved_var_split_segment() {
    let mut k = Kit::new();
    k.solved_var("u", 4.0);
    let a = k.point(0.0, 0.0);
    let b = k.point(4.5, 0.0);
    let c = k.point(7.0, 0.0);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    k.parallel(ab, bc);
    let ac = k.line(a, c);
    k.drive(DimKind::Length(ab), "u");
    k.drive(DimKind::Length(bc), "u/2");
    k.drive(DimKind::Length(ac), "7");
    assert!(k.dof() > 0);
    let m = k.measure(DimKind::Length(ab));
    k.check_value(m, 14.0 / 3.0);
}

/// Changing a free variable and re-checking a derived dimension.
#[test]
fn set_var_sequence() {
    let mut k = Kit::new();
    let o = k.origin();
    let t = k.var("t", 2.0);
    let b = k.point(3.0, 0.5);
    let c = k.point(1.0, 3.0);
    let ob = k.line(o, b);
    let oc = k.line(o, c);
    let bc = k.line(b, c);
    k.horizontal(ob);
    k.drive(DimKind::Length(ob), "3");
    k.drive(DimKind::Length(oc), "t + 1");
    k.drive(DimKind::Length(bc), "t + 2");
    let ang = k.measure(DimKind::Angle(ob, oc));
    let law = |t: f64| ((3.0f64).powi(2) + (t + 1.0).powi(2) - (t + 2.0).powi(2)) / (2.0 * 3.0 * (t + 1.0));
    k.check_value(ang, law(2.0).acos());
    for tv in [3.0, 4.5, 1.5] {
        k.set_var(t, tv);
        k.check_value(ang, law(tv).acos());
    }
}

/// Coordinates driven by expressions: a point on a circle given by a parameter.
#[test]
fn coords_parametric_circle() {
    let mut k = Kit::new();
    let o = k.origin();
    let th = k.var("th", 50.0);
    k.var("R", 4.0);
    let p = k.point(2.0, 3.0);
    let q = k.point(-3.0, 1.0);
    k.drive(DimKind::CoordX(p), "R*cos(th°)");
    k.drive(DimKind::CoordY(p), "R*sin(th°)");
    let s = k.line(o, p);
    let t = k.line(o, q);
    k.perpendicular(s, t);
    k.drive(DimKind::Length(t), "R/2");
    assert_eq!(k.dof(), 0);
    let qx = k.measure(DimKind::CoordX(q));
    let qy = k.measure(DimKind::CoordY(q));
    let r = 4.0;
    k.check_value(qx, -r / 2.0 * 50f64.to_radians().sin());
    k.check_value(qy, r / 2.0 * 50f64.to_radians().cos());
    k.set_var(th, 80.0);
    k.check_value(qx, -r / 2.0 * 80f64.to_radians().sin());
}

/// Two circles tangent, radii from variables; centre distance is r1 + r2.
#[test]
fn tangent_circles_radii_vars() {
    let mut k = Kit::new();
    let o = k.origin();
    let u = k.var("u", 2.0);
    k.var("v", 1.0);
    let c1 = k.circle(o, 2.0);
    let q = k.point(3.2, 0.0);
    let c2 = k.circle(q, 1.0);
    k.drive(DimKind::Radius(c1), "u");
    k.drive(DimKind::Radius(c2), "v");
    k.tangent(c1, c2);
    k.on(q, k.xaxis());
    assert_eq!(k.dof(), 0);
    let d = k.measure(DimKind::Distance(o, q));
    k.check_value(d, 3.0);
    k.set_var(u, 2.5);
    k.check_value(d, 3.5);
}

/// Internally tangent circles: distance of centres is |R - r|, sliding free.
#[test]
fn internal_tangent_circles_floating() {
    let mut k = Kit::new();
    k.var("R", 5.0);
    let p = k.point(0.0, 0.0);
    let q = k.point(1.8, 0.5);
    let c1 = k.circle(p, 5.0);
    let c2 = k.circle(q, 2.0);
    k.drive(DimKind::Radius(c1), "R");
    k.drive(DimKind::Radius(c2), "R*2/5");
    k.tangent(c1, c2);
    assert!(k.dof() > 0);
    let d = k.measure(DimKind::Distance(p, q));
    k.check_value(d, 3.0);
}

/// Ellipse semi-axes from variables; point at a parametric angle.
#[test]
fn ellipse_param_point() {
    let mut k = Kit::new();
    let o = k.origin();
    k.var("a", 5.0);
    k.var("b", 3.0);
    k.var("t", 35.0);
    let e = k.ellipse(o, 5.0, 3.0, 0.0);
    k.lock_conic_angle(e);
    k.drive(DimKind::ConicA(e), "a");
    k.drive(DimKind::ConicB(e), "b");
    let p = k.point(4.0, 1.5);
    k.on(p, e);
    k.drive(DimKind::CoordX(p), "a*cos(t°)");
    assert_eq!(k.dof(), 0);
    let y = k.measure(DimKind::CoordY(p));
    k.check_value(y, 3.0 * 35f64.to_radians().sin());
}

/// Parabola with focal length variable: point at x = 3f has y = 2f sqrt(3).
#[test]
fn parabola_focal_var() {
    let mut k = Kit::new();
    let o = k.origin();
    let f = k.var("f", 1.5);
    let par = k.parabola(o, 1.5, 0.0);
    k.lock_conic_angle(par);
    k.drive(DimKind::ConicA(par), "f");
    let p = k.point(5.0, 3.5);
    k.on(p, par);
    k.drive(DimKind::CoordX(p), "3f");
    let y = k.measure(DimKind::CoordY(p));
    k.check_value(y, 2.0 * 1.5 * 3f64.sqrt());
    k.set_var(f, 2.0);
    k.check_value(y, 2.0 * 2.0 * 3f64.sqrt());
}

/// Regular pentagon built from sides and angles: diagonal = s * phi.
#[test]
fn pentagon_golden_ratio() {
    let mut k = Kit::new();
    let o = k.origin();
    k.var("s", 2.0);
    let side = 2.0;
    let mut v = vec![[0.0, 0.0]];
    for i in 0..4 {
        let a = (72.0 * i as f64).to_radians();
        let l = v[i];
        v.push([l[0] + side * a.cos() * 1.05, l[1] + side * a.sin() * 0.95]);
    }
    let a = o;
    let b = k.point(v[1][0], v[1][1]);
    let c = k.point(v[2][0], v[2][1]);
    let d = k.point(v[3][0], v[3][1]);
    let e = k.point(v[4][0], v[4][1]);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let cd = k.line(c, d);
    let de = k.line(d, e);
    let ea = k.line(e, a);
    k.horizontal(ab);
    for l in [ab, bc, cd, de] {
        k.drive(DimKind::Length(l), "s");
    }
    // the label sector at B is the exterior one (72°); at C and D the interior one
    k.drive(DimKind::Angle(ab, bc), "72");
    k.drive(DimKind::Angle(bc, cd), "108");
    k.drive(DimKind::Angle(cd, de), "108");
    assert_eq!(k.dof(), 0);
    let diag = k.measure(DimKind::Distance(a, c));
    let last = k.measure(DimKind::Length(ea));
    let phi = (1.0 + 5f64.sqrt()) / 2.0;
    k.check_value(diag, 2.0 * phi);
    k.check_value(last, 2.0);
}

/// Chord of a circle by central angle: chord = 2 r sin(theta/2), floating.
#[test]
fn chord_from_angle_var() {
    let mut k = Kit::new();
    let c = k.point(0.0, 0.0);
    let a = k.point(3.0, 0.2);
    let b = k.point(1.0, 2.8);
    let circ = k.circle(c, 3.0);
    k.on(a, circ);
    k.on(b, circ);
    let ca = k.line(c, a);
    let cb = k.line(c, b);
    k.var("r", 3.0);
    k.var("th", 70.0);
    k.drive(DimKind::Radius(circ), "sqrt(2)*r/sqrt(2)");
    k.drive(DimKind::Angle(ca, cb), "th°");
    assert!(k.dof() > 0);
    let chord = k.measure(DimKind::Distance(a, b));
    k.check_value(chord, 2.0 * 3.0 * 35f64.to_radians().sin());
}

/// Crank-slider: slider position x = L cos(th) + sqrt(R^2 - L^2 sin^2(th)).
#[test]
fn crank_slider_position() {
    let mut k = Kit::new();
    let o = k.origin();
    let th = k.var("th", 40.0);
    k.var("L", 2.0);
    k.var("R", 5.0);
    let crank_end = k.point(1.2, 1.6);
    let slider = k.point(6.5, 0.0);
    let crank = k.line(o, crank_end);
    let rod = k.line(crank_end, slider);
    k.on(slider, k.xaxis());
    k.drive(DimKind::Length(crank), "L");
    k.drive(DimKind::Length(rod), "R");
    k.drive(DimKind::Angle(k.xaxis(), crank), "th°");
    assert_eq!(k.dof(), 0);
    let x = k.measure(DimKind::CoordX(slider));
    let f = |t: f64| 2.0 * t.to_radians().cos() + (25.0 - 4.0 * t.to_radians().sin().powi(2)).sqrt();
    k.check_value(x, f(40.0));
    k.set_var(th, 75.0);
    k.check_value(x, f(75.0));
}

/// Floating two-link arm: a right angle at the joint fixes the reach.
#[test]
fn floating_two_link_arm() {
    let mut k = Kit::new();
    let l = k.var("L", 2.0);
    let a = k.point(0.0, 0.0);
    let j = k.point(2.0, 0.1);
    let e = k.point(2.1, 3.1);
    let upper = k.line(a, j);
    let lower = k.line(j, e);
    k.drive(DimKind::Length(upper), "L");
    k.drive(DimKind::Length(lower), "1.5*L");
    k.drive(DimKind::Angle(upper, lower), "90");
    assert!(k.dof() > 0);
    let reach = k.measure(DimKind::Distance(a, e));
    k.check_value(reach, 2.0 * 3.25f64.sqrt());
    k.set_var(l, 3.0);
    k.check_value(reach, 3.0 * 3.25f64.sqrt());
}

/// Slope variable: angle = atan(k), point on the ray at given x has y = k x.
#[test]
fn slope_variable_atan() {
    let mut k = Kit::new();
    let o = k.origin();
    let m = k.var("m", 0.75);
    let p = k.point(2.0, 3.0);
    let s = k.line(o, p);
    k.drive(DimKind::Angle(k.xaxis(), s), "atan(m)");
    k.drive(DimKind::CoordX(p), "4");
    let y = k.measure(DimKind::CoordY(p));
    let l = k.measure(DimKind::Length(s));
    k.check_value(y, 3.0);
    k.check_value(l, 5.0);
    k.set_var(m, 2.0);
    k.check_value(y, 8.0);
    k.check_value(l, 80f64.sqrt());
}

/// Powers: a floating segment of length a^3 - 2a^2 + 1 with a given height.
#[test]
fn power_expression_length() {
    let mut k = Kit::new();
    let a = k.var("a", 2.5);
    let q = k.point(0.0, 0.0);
    let p = k.point(2.5, 0.3);
    let s = k.line(q, p);
    k.drive(DimKind::Length(s), "a^3 - 2*a^2 + 1");
    k.drive(DimKind::VDist(q, p), "a/2");
    assert!(k.dof() > 0);
    let hx = k.measure(DimKind::HDist(q, p));
    k.check_value(hx, hx_val(2.5));
    k.set_var(a, 3.0);
    k.check_value(hx, hx_val(3.0));
}

fn hx_val(a: f64) -> f64 {
    let l = a.powi(3) - 2.0 * a * a + 1.0;
    (l * l - a * a / 4.0).sqrt()
}

/// Dimension referencing coordinate dimension names: y_B = x_B^2 / 4.
#[test]
fn coordinate_dimension_names() {
    let mut k = Kit::new();
    let b = k.point(3.0, 2.0);
    let dx = k.drive(DimKind::CoordX(b), "3");
    let xn = nm(&k, dx);
    k.drive(DimKind::CoordY(b), &format!("{xn}^2/4"));
    assert_eq!(k.dof(), 0);
    let o = k.origin();
    let d = k.measure(DimKind::Distance(o, b));
    k.check_value(d, (9.0f64 + 81.0 / 16.0).sqrt());
    k.set(dx, "4");
    k.check_value(d, (16.0f64 + 16.0).sqrt());
}

/// Floating triangle by law of sines with variables for the angles.
#[test]
fn law_of_sines_variables() {
    let mut k = Kit::new();
    k.var("A", 40.0);
    k.var("B", 65.0);
    let a = k.point(0.0, 0.0);
    let b = k.point(5.0, 0.2);
    let c = k.point(1.5, 3.0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.drive(DimKind::Length(ab), "5");
    k.drive(DimKind::Angle(ab, ac), "A°");
    k.drive(DimKind::Angle(ab, bc), "180° - B°");
    assert!(k.dof() > 0);
    let side = k.measure(DimKind::Length(ac));
    let gamma = (180.0 - 40.0 - 65.0f64).to_radians();
    k.check_value(side, 5.0 * 65f64.to_radians().sin() / gamma.sin());
}

/// A numeric variable is frozen and substituted as a plain number in the
/// derived formula, so its name no longer appears there.
#[test]
fn numeric_variable_drops_from_formula() {
    let mut k = Kit::new();
    let o = k.origin();
    let aa = k.var("aa", 3.0);
    k.var("bb", 4.0);
    let p = k.point(3.0, 0.0);
    let q = k.point(3.0, 4.0);
    let op = k.line(o, p);
    let pq = k.line(p, q);
    k.horizontal(op);
    k.perpendicular(op, pq);
    k.drive(DimKind::Length(op), "aa");
    k.drive(DimKind::Length(pq), "bb");
    assert_eq!(k.dof(), 0);
    let d = k.measure(DimKind::Distance(o, q));
    let text = k.check(d);
    assert!(text.contains("aa"), "{text}");
    assert!(text.contains("bb"), "{text}");

    ops::set_var_numeric(&mut k.geo, aa, true).unwrap();
    let text = k.check(d);
    assert!(!text.contains("aa"), "{text}");
    assert!(text.contains("bb"), "{text}");
    k.check_value(d, 5.0);
}

/// A numeric dimension is substituted as its frozen value in the derived
/// formula, so its name no longer appears there.
#[test]
fn numeric_dimension_drops_from_formula() {
    let mut k = Kit::new();
    let o = k.origin();
    let p = k.point(3.0, 0.0);
    let q = k.point(3.0, 4.0);
    let op = k.line(o, p);
    let pq = k.line(p, q);
    k.horizontal(op);
    k.perpendicular(op, pq);
    let d1 = k.drive(DimKind::Length(op), "3");
    let d2 = k.drive(DimKind::Length(pq), "4");
    assert_eq!(k.dof(), 0);
    let dd = k.measure(DimKind::Distance(o, q));
    let n1 = nm(&k, d1);
    let n2 = nm(&k, d2);
    let text = k.check(dd);
    assert!(text.contains(&n1), "{text}");
    assert!(text.contains(&n2), "{text}");

    ops::set_dim_numeric(&mut k.geo, d1, true).unwrap();
    let text = k.check(dd);
    assert!(!text.contains(&n1), "{text}");
    assert!(text.contains(&n2), "{text}");
    k.check_value(dd, 5.0);
}

/// Real root of p(x) = 0 nearest `guess`, by Newton's method.
fn zero_near(p: impl Fn(f64) -> f64, guess: f64) -> f64 {
    let mut x = guess;
    for _ in 0..200 {
        let h = 1e-6 * (1.0 + x.abs());
        let d = (p(x + h) - p(x - h)) / (2.0 * h);
        x -= p(x) / d;
    }
    x
}

/// Solved variable s with a right triangle of legs s and s² + s and
/// hypotenuse 3: s⁴ + 2s³ + 2s² − 9 = 0 (irreducible quartic), unknowns
/// coupled with the triangle's coordinates.
#[test]
fn solved_var_quartic_triangle() {
    let mut k = Kit::new();
    k.solved_var("s", 1.2);
    let a = k.point(0.0, 0.0);
    let b = k.point(1.2, 0.1);
    let c = k.point(0.1, 2.7);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.perpendicular(ab, ac);
    k.drive(DimKind::Length(ab), "s");
    k.drive(DimKind::Length(ac), "s^2 + s");
    k.drive(DimKind::Length(bc), "3");
    let ang = k.measure_angle(ab, bc);
    let s0 = zero_near(|s| s.powi(4) + 2.0 * s.powi(3) + 2.0 * s * s - 9.0, 1.2);
    k.check_value(ang, std::f64::consts::PI - (s0 / 3.0).acos());
}

/// Solved variable s with s³ + s = 3 through two dimensions of one point.
#[test]
fn solved_var_cubic() {
    let mut k = Kit::new();
    k.solved_var("s", 1.2);
    let p = k.point(1.2, 3.0);
    let f = k.curve("y = x^3 + x");
    k.on(p, f);
    k.drive(DimKind::CoordX(p), "s");
    k.drive(DimKind::CoordY(p), "3");
    let q = k.point(1.2, 1.2);
    let g = k.curve("y = x");
    k.on(q, g);
    k.drive(DimKind::CoordX(q), "s");
    let qy = k.measure(DimKind::CoordY(q));
    k.check_value(qy, zero_near(|x| x.powi(3) + x - 3.0, 1.2));
}

/// Three solved variables u, v, w with u + v + w = 2, uv + vw + wu = −1,
/// uvw = −1 (elementary symmetric functions): u is a root of
/// t³ − 2t² − t + 1 = 0 (three real roots), a polynomial system whose
/// eliminant is an irreducible cubic.
#[test]
fn symmetric_system_cubic() {
    let mut k = Kit::new();
    let t0 = zero_near(|t| t.powi(3) - 2.0 * t * t - t + 1.0, 2.25);
    let t1 = zero_near(|t| t.powi(3) - 2.0 * t * t - t + 1.0, 0.55);
    let t2 = zero_near(|t| t.powi(3) - 2.0 * t * t - t + 1.0, -0.8);
    k.solved_var("u", t0 + 0.01);
    k.solved_var("v", t1 - 0.01);
    k.solved_var("w", t2 + 0.01);
    // points on vertical lines x = 2, x = −1, x = −1 whose x coordinates
    // are driven by the symmetric functions
    for (xv, e, y) in [(2.0, "u + v + w", 0.0), (-1.0, "u*v + v*w + w*u", 1.0), (-1.0, "u*v*w", 2.0)] {
        let c = k.curve(&format!("x = {e}"));
        let p = k.point(xv, y);
        k.on(p, c);
        k.drive(DimKind::CoordX(p), &format!("{xv}"));
        k.drive(DimKind::CoordY(p), &format!("{y}"));
    }
    let s = k.point(t0, t0);
    let g = k.curve("y = x");
    k.on(s, g);
    k.drive(DimKind::CoordX(s), "u");
    let sy = k.measure(DimKind::CoordY(s));
    k.check_value(sy, t0);
}

/// Solved variable with two positive roots (s² − 5s + c = 0), upper root.
#[test]
fn solved_var_quadratic_upper_root() {
    let mut k = Kit::new();
    k.var("c", 6.0);
    k.solved_var("s", 3.2);
    let a = k.origin();
    let b = k.point(15.0, 0.1);
    let cpt = k.point(0.1, 15.0);
    let ab = k.line(a, b);
    let ac = k.line(a, cpt);
    k.horizontal(ab);
    k.vertical(ac);
    k.drive(DimKind::Length(ab), "s*s + c");
    k.drive(DimKind::Length(ac), "5*s");
    k.equal(ab, ac);
    let m = k.measure(DimKind::Distance(b, cpt));
    k.check_value(m, 15.0 * 2f64.sqrt());
}

/// Same, lower root.
#[test]
fn solved_var_quadratic_lower_root() {
    let mut k = Kit::new();
    k.var("c", 6.0);
    k.solved_var("s", 1.8);
    let a = k.origin();
    let b = k.point(10.0, 0.1);
    let cpt = k.point(0.1, 10.0);
    let ab = k.line(a, b);
    let ac = k.line(a, cpt);
    k.horizontal(ab);
    k.vertical(ac);
    k.drive(DimKind::Length(ab), "s*s + c");
    k.drive(DimKind::Length(ac), "5*s");
    k.equal(ab, ac);
    let m = k.measure(DimKind::Distance(b, cpt));
    k.check_value(m, 10.0 * 2f64.sqrt());
}

/// Solved variable at a double root (discriminant exactly zero). The
/// numeric solution of a double root is only accurate to about √ε, so the
/// measurement is too.
#[test]
fn solved_var_double_root() {
    let mut k = Kit::new();
    k.value_tol = 1e-7;
    k.var("c", 6.25);
    k.solved_var("s", 2.4);
    let a = k.origin();
    let b = k.point(12.5, 0.1);
    let cpt = k.point(0.1, 12.5);
    let ab = k.line(a, b);
    let ac = k.line(a, cpt);
    k.horizontal(ab);
    k.vertical(ac);
    k.drive(DimKind::Length(ab), "s*s + c");
    k.drive(DimKind::Length(ac), "5*s");
    k.equal(ab, ac);
    let m = k.measure(DimKind::Distance(b, cpt));
    println!("measured {} vs exact {}", k.value(m), 12.5 * 2f64.sqrt());
    k.check(m);
}

/// A chain of defined variables feeding a driving dimension.
#[test]
fn defvar_long_chain() {
    let mut k = Kit::new();
    k.var("s", 2.0);
    k.defvar("t", "2*s + 1");
    k.defvar("u", "t^2 - s");
    k.defvar("w", "sqrt(u) / t");
    k.defvar("z", "w*t*t");
    let a = k.origin();
    let b = k.point(3.0, 0.1);
    let c = k.point(0.1, 15.0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    k.horizontal(ab);
    k.perpendicular(ab, ac);
    k.drive(DimKind::Length(ab), "t");
    k.drive(DimKind::Length(ac), "z");
    let d = k.measure(DimKind::Distance(b, c));
    let (t, u) = (5.0f64, 23.0f64);
    let z = u.sqrt() * t;
    k.check_value(d, (t * t + z * z).sqrt());
}

/// Dimensions driven by expressions of other dimensions.
#[test]
fn dims_referencing_dims() {
    let mut k = Kit::new();
    let a = k.origin();
    let b = k.point(3.0, 0.1);
    let c = k.point(1.0, 3.0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.horizontal(ab);
    let d1 = k.drive(DimKind::Length(ab), "3");
    let n1 = nm(&k, d1);
    let d2 = k.drive(DimKind::Length(ac), &format!("{n1} + 1"));
    let n2 = nm(&k, d2);
    k.drive(DimKind::Length(bc), &format!("({n1} + {n2})/2 + 1"));
    let ang = k.measure_angle(ab, ac);
    // a=3, b=4, c=4.5
    k.check_value(ang, ((9.0 + 16.0 - 20.25) / 24.0f64).acos());
}

/// Angle driven by an expression of another angle (radians).
#[test]
fn angle_expression_of_angle() {
    let mut k = Kit::new();
    let o = k.origin();
    let p = k.point(4.0, 1.0);
    let q = k.point(1.0, 4.0);
    let l1 = k.line(o, p);
    let l2 = k.line(o, q);
    let xa = k.xaxis();
    let t1 = k.drive_angle(xa, l1, "20");
    let n1 = nm(&k, t1);
    k.drive_angle(l1, l2, &format!("2*{n1}"));
    k.drive(DimKind::Length(l1), "4");
    k.drive(DimKind::Length(l2), "4");
    let y = k.measure(DimKind::CoordY(q));
    k.check_value(y, 4.0 * (60.0f64).to_radians().sin());
}

/// Variable taken far (×20) and then to a small value, re-deriving each time.
#[test]
fn var_far_moves_reder() {
    let mut k = Kit::new();
    let s = k.var("s", 2.0);
    let a = k.origin();
    let b = k.point(2.0, 0.1);
    let c = k.point(1.0, 1.5);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.horizontal(ab);
    k.drive(DimKind::Length(ab), "s");
    k.drive(DimKind::Length(ac), "s");
    k.drive(DimKind::Length(bc), "s");
    let y = k.measure(DimKind::CoordY(c));
    k.check_value(y, 3f64.sqrt());
    let f = k.formula(y);
    k.set_var(s, 40.0);
    k.still(y, &f, "s=40");
    k.set_var(s, 0.001);
    k.still(y, &f, "s=0.001");
}

/// A driving dimension with functions of other dimensions.
#[test]
fn dim_expression_with_functions() {
    let mut k = Kit::new();
    let a = k.origin();
    let b = k.point(3.0, 0.1);
    let c = k.point(0.1, 3.0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    k.horizontal(ab);
    k.perpendicular(ab, ac);
    let d1 = k.drive(DimKind::Length(ab), "3");
    let n1 = nm(&k, d1);
    k.drive(DimKind::Length(ac), &format!("sqrt({n1}^2 + 7)"));
    let d = k.measure(DimKind::Distance(b, c));
    k.check_value(d, (9.0f64 + 16.0).sqrt());
}

/// A defined variable of a dimension feeds another dimension.
#[test]
fn defvar_of_dimension() {
    let mut k = Kit::new();
    let a = k.origin();
    let b = k.point(3.0, 0.1);
    let c = k.point(1.0, 3.0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.horizontal(ab);
    let d1 = k.drive(DimKind::Length(ab), "4");
    let n1 = nm(&k, d1);
    k.defvar("w", &format!("{n1}*3/4"));
    k.drive(DimKind::Length(ac), "w");
    k.drive(DimKind::Length(bc), "w + 2");
    let ang = k.measure_angle(ab, ac);
    // a=4, b=3, c=5 → right angle at A
    k.check_value(ang, PI / 2.0);
}

/// Solved variable fixed by a geometric condition: right triangle with legs
/// s and s+1 and hypotenuse 5 → s = 3 (other root −4 is not geometric).
#[test]
fn solved_var_pythagorean_root() {
    let mut k = Kit::new();
    k.solved_var("s", 2.8);
    let a = k.origin();
    let b = k.point(3.0, 0.1);
    let c = k.point(0.1, 4.0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.horizontal(ab);
    k.perpendicular(ab, ac);
    k.drive(DimKind::Length(ab), "s");
    k.drive(DimKind::Length(ac), "s + 1");
    let h = k.drive(DimKind::Length(bc), "5");
    let m = k.measure(DimKind::Distance(a, b));
    k.check_value(m, 3.0);
    let f = k.formula(m);
    k.set(h, "13");
    k.still(m, &f, "hyp 13");
}

/// Defined variable of a solved variable.
#[test]
fn defvar_of_solved_var() {
    let mut k = Kit::new();
    k.solved_var("s", 2.8);
    k.defvar("w", "2*s + 1");
    let a = k.origin();
    let b = k.point(3.0, 0.1);
    let c = k.point(0.1, 7.0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    k.horizontal(ab);
    k.perpendicular(ab, ac);
    k.drive(DimKind::Length(ab), "s");
    k.drive(DimKind::Length(ac), "w");
    k.drive(DimKind::Length(bc), "w + 1/3");
    // s² + (2s+1)² = (2s + 4/3)² → s² + 4s + 1 = 4s²... solve numerically below
    let m = k.measure(DimKind::Distance(a, b));
    // s² + 4s² + 4s + 1 = 4s² + 16s/3 + 16/9 → s² − 4s/3 − 7/9 = 0 → s = (4/3 + sqrt(16/9 + 28/9))/2
    k.check_value(m, (4.0 / 3.0 + (44.0f64 / 9.0).sqrt()) / 2.0);
}

/// Solved variable that is a root of a cubic with three real roots
/// s³ − 6s² + 11s − c = 0 (c = 6: roots 1, 2, 3).
fn cubic_root_case(guess: f64, expect_s: f64) {
    let mut k = Kit::new();
    let cv = k.var("c", 6.0);
    k.solved_var("s", guess);
    let a = k.origin();
    let l = expect_s.powi(3) + 11.0 * expect_s + 10.0;
    let b = k.point(l, 0.1);
    let cpt = k.point(0.1, l);
    let ab = k.line(a, b);
    let ac = k.line(a, cpt);
    k.horizontal(ab);
    k.vertical(ac);
    k.drive(DimKind::Length(ab), "s^3 + 11*s + 10");
    k.drive(DimKind::Length(ac), "6*s^2 + c + 10");
    k.equal(ab, ac);
    let m = k.measure(DimKind::Length(ab));
    let s = expect_s;
    k.check_value(m, s * s * s + 11.0 * s + 10.0);
    let f = k.formula(m);
    k.set_var(cv, 6.1);
    k.still(m, &f, "c = 6.1");
}

#[test]
fn solved_var_cubic_root_low() {
    cubic_root_case(0.9, 1.0);
}

#[test]
fn solved_var_cubic_root_mid() {
    cubic_root_case(2.1, 2.0);
}

#[test]
fn solved_var_cubic_root_high() {
    cubic_root_case(3.1, 3.0);
}

/// Length driven by an expression that is 0 (coincident end points).
#[test]
fn zero_length_by_expression() {
    let mut k = Kit::new();
    let a = k.origin();
    let b = k.point(3.0, 0.1);
    let c = k.point(3.2, 0.3);
    let d = k.point(1.0, 4.0);
    let ab = k.line(a, b);
    k.horizontal(ab);
    let d1 = k.drive(DimKind::Length(ab), "3");
    let n1 = nm(&k, d1);
    let ok = k.try_constrain(sggeo::model::Con::Coincident(c, c));
    let _ = ok;
    k.drive(DimKind::Distance(b, c), &format!("{n1} - 3"));
    let ad = k.line(a, d);
    let cd = k.line(c, d);
    k.drive(DimKind::Length(ad), "4");
    k.drive(DimKind::Length(cd), "5");
    let y = k.measure(DimKind::CoordY(d));
    k.check_value(y, 4.0);
}
