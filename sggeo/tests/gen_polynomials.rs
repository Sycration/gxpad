//! Roots of irreducible polynomials of degree 3 and 4 (and of polynomials
//! in a power of the unknown that reduce to them): a point on a curve
//! y = p(x) at a given height has x as such a root, which has a closed
//! form by Cardano's and Ferrari's formulas.

use sggeo::model::DimKind;
mod testkit;
use testkit::Kit;

/// Real root of p(x) = y nearest `guess`, by Newton's method.
fn root(p: impl Fn(f64) -> f64, y: f64, guess: f64) -> f64 {
    let mut x = guess;
    for _ in 0..100 {
        let h = 1e-6 * (1.0 + x.abs());
        let d = (p(x + h) - p(x - h)) / (2.0 * h);
        x -= (p(x) - y) / d;
    }
    x
}

/// Point on y = p(x) at height `y`, near (guess, y); returns its x dimension.
fn point_at_height(k: &mut Kit, src: &str, y: &str, guess: f64, yv: f64) -> usize {
    let f = k.curve(src);
    let p = k.point(guess, yv);
    k.on(p, f);
    k.drive(DimKind::CoordY(p), y);
    assert_eq!(k.dof(), 0);
    k.measure(DimKind::CoordX(p))
}

/// x³ + x = 3: one real root (Cardano with real cube roots).
#[test]
fn cubic_one_real_root() {
    let mut k = Kit::new();
    let x = point_at_height(&mut k, "y = x^3 + x", "3", 1.2, 3.0);
    k.check_value(x, root(|x| x * x * x + x, 3.0, 1.2));
}

/// x³ − 3x = 1 has three real roots (casus irreducibilis): the largest,
/// 2·cos(π/9).
#[test]
fn cubic_three_real_roots_largest() {
    let mut k = Kit::new();
    let x = point_at_height(&mut k, "y = x^3 - 3*x", "1", 1.9, 1.0);
    k.check_value(x, 2.0 * (std::f64::consts::PI / 9.0).cos());
}

/// The middle root of x³ − 3x = 1, 2·cos(5π/9).
#[test]
fn cubic_three_real_roots_middle() {
    let mut k = Kit::new();
    let x = point_at_height(&mut k, "y = x^3 - 3*x", "1", -0.35, 1.0);
    k.check_value(x, 2.0 * (5.0 * std::f64::consts::PI / 9.0).cos());
}

/// The smallest root of x³ − 3x = 1, 2·cos(7π/9).
#[test]
fn cubic_three_real_roots_smallest() {
    let mut k = Kit::new();
    let x = point_at_height(&mut k, "y = x^3 - 3*x", "1", -1.5, 1.0);
    k.check_value(x, 2.0 * (7.0 * std::f64::consts::PI / 9.0).cos());
}

/// A cubic with all terms, x³ − 2x² − x = 1.
#[test]
fn cubic_all_terms() {
    let mut k = Kit::new();
    let x = point_at_height(&mut k, "y = x^3 - 2*x^2 - x", "1", 2.5, 1.0);
    k.check_value(x, root(|x| x * x * x - 2.0 * x * x - x, 1.0, 2.5));
}

/// A cubic with a variable coefficient, x³ + a·x = 3.
#[test]
fn cubic_variable_coefficient() {
    let mut k = Kit::new();
    let a = k.var("a", 2.0);
    let x = point_at_height(&mut k, "y = x^3 + a*x", "3", 1.0, 3.0);
    k.check_value(x, 1.0);
    k.set_var(a, 5.0);
    k.check_value(x, root(|x| x * x * x + 5.0 * x, 3.0, 0.5));
}

/// x⁴ − 4x = 2, irreducible by Eisenstein at 2 (Ferrari).
#[test]
fn quartic_depressed() {
    let mut k = Kit::new();
    let x = point_at_height(&mut k, "y = x^4 - 4*x", "2", 1.7, 2.0);
    k.check_value(x, root(|x| x.powi(4) - 4.0 * x, 2.0, 1.7));
}

/// The negative root of x⁴ − 4x = 2.
#[test]
fn quartic_depressed_negative_root() {
    let mut k = Kit::new();
    let x = point_at_height(&mut k, "y = x^4 - 4*x", "2", -0.5, 2.0);
    k.check_value(x, root(|x| x.powi(4) - 4.0 * x, 2.0, -0.5));
}

/// A quartic with all terms, x⁴ + x³ − x² + x = 2.
#[test]
fn quartic_all_terms() {
    let mut k = Kit::new();
    let x = point_at_height(&mut k, "y = x^4 + x^3 - x^2 + x", "2", 1.0, 2.0);
    k.check_value(x, root(|x| x.powi(4) + x.powi(3) - x * x + x, 2.0, 1.0));
}

/// x⁶ + x² = 3: a cubic in x².
#[test]
fn sextic_cubic_in_square() {
    let mut k = Kit::new();
    let x = point_at_height(&mut k, "y = x^6 + x^2", "3", 1.1, 3.0);
    k.check_value(x, root(|x| x.powi(6) + x * x, 3.0, 1.1));
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

/// Point on two curves near `guess`; returns (x dim, y dim).
fn on_two(k: &mut Kit, a: &str, b: &str, guess: [f64; 2]) -> (usize, usize) {
    let f = k.curve(a);
    let g = k.curve(b);
    let p = k.point(guess[0], guess[1]);
    k.on(p, f);
    k.on(p, g);
    assert_eq!(k.dof(), 0);
    (k.measure(DimKind::CoordX(p)), k.measure(DimKind::CoordY(p)))
}

/// y = x² − 2 and x = y² − 3: x⁴ − 4x² − x + 1 = 0, irreducible, four real
/// roots (Ferrari with a resolvent of three real roots).
#[test]
fn two_parabolas_quartic() {
    let mut k = Kit::new();
    let (x, _) = on_two(&mut k, "y = x^2 - 2", "x = y^2 - 3", [2.06, 2.25]);
    k.check_value(x, zero_near(|x| x.powi(4) - 4.0 * x * x - x + 1.0, 2.06));
}

/// The same with symbolic shifts a, b.
#[test]
fn two_parabolas_symbolic() {
    let mut k = Kit::new();
    k.var("a", -2.0);
    k.var("b", -3.0);
    let (x, _) = on_two(&mut k, "y = x^2 + a", "x = y^2 + b", [2.06, 2.25]);
    k.check_value(x, zero_near(|x| x.powi(4) - 4.0 * x * x - x + 1.0, 2.06));
}

/// Circle with centre (1, 2) and radius 2 (dimensions) and y = x²:
/// x⁴ − 3x² − 2x + 1 = 0, irreducible.
#[test]
fn circle_parabola_quartic() {
    let mut k = Kit::new();
    let c = k.point(1.0, 2.0);
    k.drive(DimKind::CoordX(c), "1");
    k.drive(DimKind::CoordY(c), "2");
    let ci = k.circle(c, 2.0);
    k.drive(DimKind::Radius(ci), "2");
    let f = k.curve("y = x^2");
    let p = k.point(1.94, 3.76);
    k.on(p, ci);
    k.on(p, f);
    assert_eq!(k.dof(), 0);
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, zero_near(|x| x.powi(4) - 3.0 * x * x - 2.0 * x + 1.0, 1.94));
}

/// Hyperbola xy = 1 and y = x² − x: x³ − x² − 1 = 0 (one real root).
#[test]
fn hyperbola_parabola_cubic() {
    let mut k = Kit::new();
    let (x, y) = on_two(&mut k, "x*y = 1", "y = x^2 - x", [1.47, 0.68]);
    let r = zero_near(|x| x.powi(3) - x * x - 1.0, 1.47);
    k.check_value(x, r);
    k.check_value(y, 1.0 / r);
}

/// y = x³ and the line through A(0, −1), B(1, 2) (dimensioned): x³ = 3x − 1,
/// three real roots (trigonometric Cardano with symbolic coefficients).
#[test]
fn line_cubic_three_real_symbolic() {
    let mut k = Kit::new();
    let a = k.point(0.0, -1.0);
    let b = k.point(1.0, 2.0);
    k.drive(DimKind::CoordX(a), "0");
    k.drive(DimKind::CoordY(a), "-1");
    k.drive(DimKind::CoordX(b), "1");
    k.drive(DimKind::CoordY(b), "2");
    let l = k.inf_line(a, b);
    let f = k.curve("y = x^3");
    let p = k.point(1.53, 3.6);
    k.on(p, l);
    k.on(p, f);
    assert_eq!(k.dof(), 0);
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, zero_near(|x| x.powi(3) - 3.0 * x + 1.0, 1.53));
}

/// Point on curve `src` at height `y` near `guess`; returns its x dimension.
fn at_height(k: &mut Kit, src: &str, y: &str, guess: f64, yv: f64) -> usize {
    let f = k.curve(src);
    let p = k.point(guess, yv);
    k.on(p, f);
    k.drive(DimKind::CoordY(p), y);
    assert_eq!(k.dof(), 0);
    k.measure(DimKind::CoordX(p))
}

/// Folium x³ + y³ = 3xy at height 1: x³ − 3x + 1 = 0, three real roots.
#[test]
fn folium_at_height() {
    let mut k = Kit::new();
    let x = at_height(&mut k, "x^3 + y^3 = 3*x*y", "1", 1.53, 1.0);
    k.check_value(x, zero_near(|x| x.powi(3) - 3.0 * x + 1.0, 1.53));
}

/// Ellipse (x−1)² + 4y² = 4 and xy = 1: (x − 1)(x³ − x² − 4x − 4) = 0, the
/// root of the cubic factor.
#[test]
fn ellipse_hyperbola_reducible() {
    let mut k = Kit::new();
    let (x, _) = on_two(&mut k, "(x-1)^2 + 4*y^2 = 4", "x*y = 1", [2.87, 0.35]);
    k.check_value(x, zero_near(|x| x.powi(3) - x * x - 4.0 * x - 4.0, 2.87));
}

/// Circle x² + y² = r² (r a variable) and y = x³: x⁶ + x² = r², a cubic in x².
#[test]
fn circle_cubic_graph_symbolic() {
    let mut k = Kit::new();
    k.var("r", 2.0);
    let (x, _) = on_two(&mut k, "x^2 + y^2 = r^2", "y = x^3", [1.17, 1.62]);
    k.check_value(x, zero_near(|x| x.powi(6) + x * x - 4.0, 1.17));
}

/// Circle with dimensioned centre and radius and y = x³ through a circle
/// centre on the y axis: x² + (x³ − c)² = r², degree 6, no closed form in
/// general, but with c = 0 by construction it is a cubic in x².
#[test]
fn circle_cubic_graph_dims() {
    let mut k = Kit::new();
    let o = k.origin();
    let ci = k.circle(o, 2.0);
    k.drive(DimKind::Radius(ci), "2");
    let f = k.curve("y = x^3");
    let p = k.point(1.17, 1.62);
    k.on(p, ci);
    k.on(p, f);
    assert_eq!(k.dof(), 0);
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, zero_near(|x| x.powi(6) + x * x - 4.0, 1.17));
}

/// (x+1)⁶ + (x+1)² = 3 expanded: a cubic in (x + 1)², not in x².
#[test]
fn shifted_cubic_in_square() {
    let mut k = Kit::new();
    let x = at_height(&mut k, "y = x^6 + 6*x^5 + 15*x^4 + 20*x^3 + 16*x^2 + 8*x + 2", "3", 0.1, 3.0);
    k.check_value(x, zero_near(|x| (x + 1.0).powi(6) + (x + 1.0).powi(2) - 3.0, 0.1));
}

/// (x³ + x)² + (x³ + x) = 3: a quadratic in x³ + x, then Cardano.
#[test]
fn decomposable_sextic() {
    let mut k = Kit::new();
    let x = at_height(&mut k, "y = x^6 + 2*x^4 + x^3 + x^2 + x", "3", 0.8, 3.0);
    k.check_value(x, zero_near(|x| (x.powi(3) + x).powi(2) + x.powi(3) + x - 3.0, 0.8));
}

/// Root of curve y = p(x) on the x axis near `guess`.
fn on_axis(k: &mut Kit, src: &str, guess: f64) -> usize {
    let f = k.curve(src);
    let xa = k.xaxis();
    let p = k.point(guess, 0.02);
    k.on(p, f);
    k.on(p, xa);
    assert_eq!(k.dof(), 0);
    k.measure(DimKind::CoordX(p))
}

/// Reciprocal sextic x⁶ + x⁵ − 3x⁴ + x³ − 3x² + x + 1 = 0: a cubic in
/// z = x + 1/x.
#[test]
fn palindromic_sextic() {
    let mut k = Kit::new();
    let x = on_axis(&mut k, "y = x^6 + x^5 - 3*x^4 + x^3 - 3*x^2 + x + 1", 0.74);
    k.check_value(x, zero_near(|x| x.powi(6) + x.powi(5) - 3.0 * x.powi(4) + x.powi(3) - 3.0 * x * x + x + 1.0, 0.74));
}

/// Reciprocal quartic x⁴ + x³ − 5x² + x + 1 = 0 (a quadratic in x + 1/x).
#[test]
fn palindromic_quartic() {
    let mut k = Kit::new();
    let x = on_axis(&mut k, "y = x^4 + x^3 - 5*x^2 + x + 1", 1.5);
    k.check_value(x, zero_near(|x| x.powi(4) + x.powi(3) - 5.0 * x * x + x + 1.0, 1.5));
}

/// de Moivre quintic x⁵ − 5x³ + 5x = h: x = 2 cos(acos(h/2)/5 + 2πk/5).
#[test]
fn de_moivre_quintic() {
    let mut k = Kit::new();
    let x = at_height(&mut k, "y = x^5 - 5*x^3 + 5*x", "0.5", 1.09, 0.5);
    k.check_value(x, zero_near(|x| x.powi(5) - 5.0 * x.powi(3) + 5.0 * x - 0.5, 1.09));
}

/// x⁵ = a: binomial with a variable.
#[test]
fn quintic_binomial_var() {
    let mut k = Kit::new();
    k.var("a", 3.0);
    let x = at_height(&mut k, "y = x^5", "a", 1.25, 3.0);
    k.check_value(x, 3.0f64.powf(0.2));
}

/// x⁹ + x³ = 3: a cubic in x³ (odd power g = 3).
#[test]
fn nonic_cubic_in_cube() {
    let mut k = Kit::new();
    let x = at_height(&mut k, "y = x^9 + x^3", "3", 1.07, 3.0);
    k.check_value(x, zero_near(|x| x.powi(9) + x.powi(3) - 3.0, 1.07));
}

/// x⁶ + x³ = 3: a quadratic in x³, the negative root.
#[test]
fn sextic_quadratic_in_cube_negative() {
    let mut k = Kit::new();
    let x = at_height(&mut k, "y = x^6 + x^3", "3", -1.32, 3.0);
    k.check_value(x, zero_near(|x| x.powi(6) + x.powi(3) - 3.0, -1.32));
}

/// x⁸ + x² = 3: a quartic in x².
#[test]
fn octic_quartic_in_square() {
    let mut k = Kit::new();
    let x = at_height(&mut k, "y = x^8 + x^2", "3", 1.08, 3.0);
    k.check_value(x, zero_near(|x| x.powi(8) + x * x - 3.0, 1.08));
}

/// (x³ + a x − 1)(x³ − 2x + 3) = 0 expanded: reducible over Q(a) into cubics.
#[test]
fn reducible_sextic_two_cubics() {
    let mut k = Kit::new();
    k.var("a", 1.0);
    let x = on_axis(&mut k, "y = x^6 + (a - 2)*x^4 + 2*x^3 - 2*a*x^2 + (3*a + 2)*x - 3", 0.68);
    k.check_value(x, zero_near(|x| x.powi(3) + x - 1.0, 0.68));
}

/// (x⁴ + a x − 1)(x⁴ − 2x² + x + 3) = 0 expanded: reducible into quartics.
#[test]
fn reducible_octic_two_quartics() {
    let mut k = Kit::new();
    k.var("a", 1.0);
    let x = on_axis(&mut k, "y = x^8 - 2*x^6 + (a + 1)*x^5 + 2*x^4 - 2*a*x^3 + (a + 2)*x^2 + (3*a - 1)*x - 3", 0.72);
    k.check_value(x, zero_near(|x| x.powi(4) + x - 1.0, 0.72));
}

/// x⁴ + a x² + b x = c with variables (a = −4, b = −1, c = −1): four real
/// roots, symbolic Ferrari.
#[test]
fn quartic_symbolic_four_real() {
    let mut k = Kit::new();
    k.var("a", -4.0);
    k.var("b", -1.0);
    k.var("c", -1.0);
    let x = at_height(&mut k, "y = x^4 + a*x^2 + b*x", "c", 0.4, -1.0);
    k.check_value(x, zero_near(|x| x.powi(4) - 4.0 * x * x - x + 1.0, 0.4));
}

/// x³ + p x + q = 0 with variables, one real root.
#[test]
fn cubic_symbolic_one_real() {
    let mut k = Kit::new();
    k.var("p", 2.0);
    k.var("q", -3.0);
    let x = on_axis(&mut k, "y = x^3 + p*x + q", 1.0);
    k.check_value(x, 1.0);
}

/// a x³ + x² = 2 with a = 0 now: the cubic degenerates; the root moves with
/// a (≈ √2 − a). Closed form valid near a = 0 is not standard Cardano.
#[test]
fn cubic_leading_coefficient_zero() {
    let mut k = Kit::new();
    k.var("a", 0.0);
    let x = at_height(&mut k, "y = a*x^3 + x^2", "2", 1.41, 2.0);
    k.check_value(x, 2.0f64.sqrt());
}

/// a x² + x = 2 with a = 0 now: x = 4/(1 + √(1 + 8a)).
#[test]
fn quadratic_leading_coefficient_zero() {
    let mut k = Kit::new();
    k.var("a", 0.0);
    let x = at_height(&mut k, "y = a*x^2 + x", "2", 2.0, 2.0);
    k.check_value(x, 2.0);
}

/// a x³ + x = 2 with a small but nonzero.
#[test]
fn cubic_small_leading_coefficient() {
    let mut k = Kit::new();
    k.var("a", 0.01);
    let x = at_height(&mut k, "y = a*x^3 + x", "2", 1.92, 2.0);
    k.check_value(x, zero_near(|x| 0.01 * x.powi(3) + x - 2.0, 1.92));
}

/// Tangent from P(1, 0.3) to y = x³ touching at t: 2t³ − 3t² + 0.3 = 0.
#[test]
fn tangent_from_point_to_cubic() {
    let mut k = Kit::new();
    let f = k.curve("y = x^3");
    let p = k.point(1.0, 0.3);
    k.drive(DimKind::CoordX(p), "1");
    k.drive(DimKind::CoordY(p), "0.3");
    let t0 = zero_near(|t| 2.0 * t.powi(3) - 3.0 * t * t + 0.3, 1.4);
    let (seg, t) = k.tangent_line(p, f, [t0, t0.powi(3)]);
    let free = k.point(0.5 * (1.0 + t0), 0.5 * (0.3 + t0.powi(3)) + 0.1);
    k.on(free, seg);
    let tx = k.measure(DimKind::CoordX(t));
    k.check_value(tx, t0);
}

/// Foot of the normal from P(1, 3) to y = x²: 2x³ − 5x − 1 = 0 (three real).
#[test]
fn normal_to_parabola() {
    let mut k = Kit::new();
    let f = k.curve("y = x^2");
    let p = k.point(1.0, 3.0);
    k.drive(DimKind::CoordX(p), "1");
    k.drive(DimKind::CoordY(p), "3");
    let x0 = zero_near(|x| 2.0 * x.powi(3) - 5.0 * x - 1.0, 1.67);
    let q = k.point(x0 + 0.02, x0 * x0 - 0.02);
    k.on(q, f);
    let r = k.point(x0 + 1.0, x0 * x0 + 2.0 * x0);
    let qr = k.line(q, r);
    k.tangent(qr, f);
    let pq = k.line(p, q);
    k.perpendicular(pq, qr);
    let qx = k.measure(DimKind::CoordX(q));
    k.check_value(qx, x0);
}

/// Line through the origin at 40° to the x axis and y = x³ − 1:
/// x³ − tan(40°)·x − 1 = 0, coefficients with a trigonometric atom.
#[test]
fn line_at_angle_cubic() {
    let mut k = Kit::new();
    let o = k.origin();
    let xa = k.xaxis();
    let q = k.point(1.27, 1.07);
    let l = k.inf_line(o, q);
    k.drive_angle(xa, l, "40");
    let f = k.curve("y = x^3 - 1");
    k.on(q, f);
    assert_eq!(k.dof(), 0);
    let x = k.measure(DimKind::CoordX(q));
    let t = 40f64.to_radians().tan();
    k.check_value(x, zero_near(|x| x.powi(3) - t * x - 1.0, 1.27));
}

/// x⁴ + y⁴ = 1 and y = x + 1/2: a quartic symmetric about x = −1/4.
#[test]
fn superellipse_line() {
    let mut k = Kit::new();
    let (x, _) = on_two(&mut k, "x^4 + y^4 = 1", "y = x + 0.5", [0.49, 0.99]);
    k.check_value(x, zero_near(|x| x.powi(4) + (x + 0.5).powi(4) - 1.0, 0.49));
}

/// Circle with centre (a, b) and radius r (all variables) and y = x²:
/// a fully symbolic quartic.
#[test]
fn circle_parabola_all_symbolic() {
    let mut k = Kit::new();
    k.var("a", 1.0);
    k.var("b", 2.0);
    k.var("r", 2.0);
    let (x, _) = on_two(&mut k, "(x - a)^2 + (y - b)^2 = r^2", "y = x^2", [1.94, 3.76]);
    k.check_value(x, zero_near(|x| x.powi(4) - 3.0 * x * x - 2.0 * x + 1.0, 1.94));
}

/// a x⁴ + x = 2 with symbolic leading coefficient a = 1.
#[test]
fn quartic_symbolic_leading() {
    let mut k = Kit::new();
    k.var("a", 1.0);
    let x = at_height(&mut k, "y = a*x^4 + x", "2", 1.0, 2.0);
    k.check_value(x, 1.0);
}

/// Two cubic graphs y = x³ + x and y = 2x³ − 1: x³ − x − 1 = 0 (plastic number).
#[test]
fn two_cubic_graphs() {
    let mut k = Kit::new();
    let (x, _) = on_two(&mut k, "y = x^3 + x", "y = 2*x^3 - 1", [1.32, 3.6]);
    k.check_value(x, zero_near(|x| x.powi(3) - x - 1.0, 1.32));
}

/// (x − a)(x² + x − b) = 0 expanded, a = 1/2, b = 3: the quadratic factor's root.
#[test]
fn cubic_reducible_symbolic() {
    let mut k = Kit::new();
    k.var("a", 0.5);
    k.var("b", 3.0);
    let x = on_axis(&mut k, "y = x^3 + (1 - a)*x^2 - (a + b)*x + a*b", 1.30);
    k.check_value(x, (-1.0 + 13f64.sqrt()) / 2.0);
}

/// Cubic with three real roots close to a double root: x³ − 3x + c,
/// c = 1.98, root near −2. The root exists for c > 2 as well, but there it is
/// Cardano's one-real-root form, and no formula in real radicals and
/// trigonometric functions covers both sides of c = 2 (SHORTCOMINGS 2.3), so
/// the articulation stays on this side.
#[test]
fn cubic_near_double_root() {
    let mut k = Kit::new();
    k.value_scale = 0.005;
    let x = at_height(&mut k, "y = x^3 - 3*x", "-1.98", -2.0, -1.98);
    k.check_value(x, zero_near(|x| x.powi(3) - 3.0 * x + 1.98, -2.0));
}

/// Quartic at a height given by a square-root dimension: x⁴ + x = √2·d.
#[test]
fn quartic_sqrt_coefficient() {
    let mut k = Kit::new();
    k.var("d", 2.0);
    let x = at_height(&mut k, "y = x^4 + x", "sqrt(2)*d", 1.3, 2.0 * 2f64.sqrt());
    k.check_value(x, zero_near(|x| x.powi(4) + x - 2.0 * 2f64.sqrt(), 1.3));
}

/// Rotated parabola entity (axis angle dimensioned) and y = x³ ... replaced:
/// circle of radius r about (0, c) and y = x³: degree 6, generically no
/// closed form, but with the y axis symmetric x ↦ −x it is not; so here the
/// curve y = x⁴ instead: x² + (x⁴ − c)² = r², a quartic in x².
#[test]
fn circle_quartic_graph() {
    let mut k = Kit::new();
    let c = k.point(0.0, 1.0);
    k.drive(DimKind::CoordX(c), "0");
    k.drive(DimKind::CoordY(c), "1");
    let ci = k.circle(c, 2.0);
    k.drive(DimKind::Radius(ci), "2");
    let f = k.curve("y = x^4");
    let x0 = zero_near(|x| x * x + (x.powi(4) - 1.0).powi(2) - 4.0, 1.4);
    let p = k.point(x0, x0.powi(4));
    k.on(p, ci);
    k.on(p, f);
    assert_eq!(k.dof(), 0);
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, x0);
}

/// x¹² + x³ = 3: a quartic in x³ (odd g = 3 with Ferrari).
#[test]
fn dodecic_quartic_in_cube() {
    let mut k = Kit::new();
    let x = at_height(&mut k, "y = x^12 + x^3", "3", 1.07, 3.0);
    k.check_value(x, zero_near(|x| x.powi(12) + x.powi(3) - 3.0, 1.07));
}

/// x³ + 3x² + 3x = 3, i.e. (x + 1)³ = 4: p = 0 after the shift.
#[test]
fn cubic_perfect_cube_shift() {
    let mut k = Kit::new();
    let x = at_height(&mut k, "y = x^3 + 3*x^2 + 3*x", "3", 0.59, 3.0);
    k.check_value(x, 4f64.cbrt() - 1.0);
}

/// x⁴ + 4x³ + 6x² + 4x = 3, i.e. (x + 1)⁴ = 4.
#[test]
fn quartic_perfect_fourth_shift() {
    let mut k = Kit::new();
    let x = at_height(&mut k, "y = x^4 + 4*x^3 + 6*x^2 + 4*x", "3", 0.41, 3.0);
    k.check_value(x, 2f64.sqrt() - 1.0);
}

/// x⁴ + x² + e·x = 2 with a variable e that is 0 now: root x = 1 now and a
/// Ferrari root nearby for e ≠ 0. (The resolvent's only real root is m = 0
/// here.)
#[test]
fn quartic_odd_coefficient_zero_now() {
    let mut k = Kit::new();
    k.var("g", 0.0);
    let x = at_height(&mut k, "y = x^4 + x^2 + g*x", "2", 1.0, 2.0);
    k.check_value(x, 1.0);
}

/// x⁴ + x² + e·x = 2 with e = 0.1: generic, one positive and one negative root.
#[test]
fn quartic_odd_coefficient_small() {
    let mut k = Kit::new();
    k.var("g", 0.1);
    let x = at_height(&mut k, "y = x^4 + x^2 + g*x", "2", 0.98, 2.0);
    k.check_value(x, zero_near(|x| x.powi(4) + x * x + 0.1 * x - 2.0, 0.98));
}

/// x³ + p·x = 2 with a variable p that is 0 now.
#[test]
fn cubic_linear_coefficient_zero_now() {
    let mut k = Kit::new();
    k.var("p", 0.0);
    let x = at_height(&mut k, "y = x^3 + p*x", "2", 1.26, 2.0);
    k.check_value(x, 2f64.cbrt());
}

/// x³ − 3x + c = 0 with c a variable that is 0 now (three real roots, one
/// of them 0 now): the root √3.
#[test]
fn cubic_constant_zero_now() {
    let mut k = Kit::new();
    k.var("c", 0.0);
    let x = on_axis(&mut k, "y = x^3 - 3*x + c", 1.73);
    k.check_value(x, 3f64.sqrt());
}

/// Product of three quartics with a parameter: degree 12 (factoring time).
#[test]
fn reducible_degree12() {
    let mut k = Kit::new();
    k.var("a", 1.0);
    // (x⁴ + a x − 1)(x⁴ + x² + 2)(x⁴ − a x² + 3)
    let x = on_axis(
        &mut k,
        "y = (x^4 + a*x - 1)*(x^4 + x^2 + 2)*(x^4 - a*x^2 + 3)",
        0.72,
    );
    k.check_value(x, zero_near(|x| x.powi(4) + x - 1.0, 0.72));
}

/// The same polynomial written out expanded.
#[test]
fn reducible_degree12_expanded() {
    let mut k = Kit::new();
    k.var("a", 1.0);
    let x = on_axis(
        &mut k,
        "y = x^12 + (1 - a)*x^10 + a*x^9 + (4 - a)*x^8 + (a - a^2)*x^7 + (2 - a)*x^6 + (5*a - a^2)*x^5 + (a + 1)*x^4 + (3*a - 2*a^2)*x^3 + (2*a - 3)*x^2 + 6*a*x - 6",
        0.72,
    );
    k.check_value(x, zero_near(|x| x.powi(4) + x - 1.0, 0.72));
}

/// x = y³ + y and x² + y⁴ + y² − xy = 2 (the second is x² − 2 plus y times
/// the first): x = √2 and y the real root of y³ + y = √2, found in
/// back-substitution.
#[test]
fn back_substitution_cubic() {
    let mut k = Kit::new();
    let y0 = zero_near(|y| y.powi(3) + y - 2f64.sqrt(), 0.8);
    let (_, y) = on_two(&mut k, "x = y^3 + y", "x^2 + y^4 + y^2 - x*y = 2", [2f64.sqrt(), y0]);
    k.check_value(y, y0);
}

/// A tower of cubics: x³ − x − 1 = 0 and y³ + y = x (the second curve is
/// x³ − x − 1 plus y times the first).
#[test]
fn tower_of_cubics() {
    let mut k = Kit::new();
    let x0 = zero_near(|x| x.powi(3) - x - 1.0, 1.32);
    let y0 = zero_near(|y| y.powi(3) + y - x0, 0.8);
    let (_, y) = on_two(&mut k, "x = y^3 + y", "x^3 - x - 1 + y^4 + y^2 - x*y = 0", [x0, y0]);
    k.check_value(y, y0);
}

/// The same tower, with a variable in the inner cubic: x³ − x − a = 0.
#[test]
fn tower_of_cubics_var() {
    let mut k = Kit::new();
    k.var("a", 1.0);
    let x0 = zero_near(|x| x.powi(3) - x - 1.0, 1.32);
    let y0 = zero_near(|y| y.powi(3) + y - x0, 0.8);
    let (_, y) = on_two(&mut k, "x = y^3 + y", "x^3 - x - a + y^4 + y^2 - x*y = 0", [x0, y0]);
    k.check_value(y, y0);
}

/// Distance between a point at height 3 on y = x³ + x and a point at
/// height 2 on y = x³ + 2x: target combining two Cardano roots.
#[test]
fn distance_between_two_cubic_roots() {
    let mut k = Kit::new();
    let f = k.curve("y = x^3 + x");
    let g = k.curve("y = x^3 + 2*x");
    let x1 = zero_near(|x| x.powi(3) + x - 3.0, 1.2);
    let x2 = zero_near(|x| x.powi(3) + 2.0 * x - 2.0, 0.77);
    let p = k.point(x1, 3.0);
    let q = k.point(x2, 2.0);
    k.on(p, f);
    k.on(q, g);
    k.drive(DimKind::CoordY(p), "3");
    k.drive(DimKind::CoordY(q), "2");
    let d = k.measure(DimKind::Distance(p, q));
    k.check_value(d, ((x1 - x2).powi(2) + 1.0).sqrt());
}

/// Point on y = x³ at distance d = 2 from the point (1, 0) ... degree 6,
/// no closed form in general; with the centre at the origin and the curve
/// y = x³ − x (odd), it is a cubic in x².
#[test]
fn odd_cubic_circle_about_origin() {
    let mut k = Kit::new();
    let o = k.origin();
    let f = k.curve("y = x^3 - x");
    let x0 = zero_near(|x| x * x + (x.powi(3) - x).powi(2) - 4.0, 1.6);
    let p = k.point(x0, x0.powi(3) - x0);
    k.on(p, f);
    k.drive(DimKind::Distance(o, p), "2");
    assert_eq!(k.dof(), 0);
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, x0);
}

/// Chord–tangent: P, Q on y² = x³ − 2x + 2 at x = 1 and x = 2; the third
/// intersection R of line PQ with the curve has x_R = m² − x_P − x_Q
/// (Vieta), m the slope: x_R = 4 − 2√6.
#[test]
fn elliptic_chord_third_point() {
    let mut k = Kit::new();
    let f = k.curve("y^2 = x^3 - 2*x + 2");
    let p = k.point(1.0, 1.0);
    let q = k.point(2.0, 6f64.sqrt());
    k.on(p, f);
    k.on(q, f);
    k.drive(DimKind::CoordX(p), "1");
    k.drive(DimKind::CoordX(q), "2");
    let l = k.inf_line(p, q);
    let m = 6f64.sqrt() - 1.0;
    let xr = m * m - 3.0;
    let yr = 1.0 + m * (xr - 1.0);
    let r = k.point(xr + 0.02, yr - 0.02);
    k.on(r, f);
    k.on(r, l);
    assert_eq!(k.dof(), 0);
    let rx = k.measure(DimKind::CoordX(r));
    k.check_value(rx, 4.0 - 2.0 * 6f64.sqrt());
}

/// The same with symbolic curve coefficients y² = x³ + a x + b.
#[test]
fn elliptic_chord_third_point_symbolic() {
    let mut k = Kit::new();
    k.var("a", -2.0);
    k.var("b", 2.0);
    let f = k.curve("y^2 = x^3 + a*x + b");
    let p = k.point(1.0, 1.0);
    let q = k.point(2.0, 6f64.sqrt());
    k.on(p, f);
    k.on(q, f);
    k.drive(DimKind::CoordX(p), "1");
    k.drive(DimKind::CoordX(q), "2");
    let l = k.inf_line(p, q);
    let m = 6f64.sqrt() - 1.0;
    let xr = m * m - 3.0;
    let yr = 1.0 + m * (xr - 1.0);
    let r = k.point(xr + 0.02, yr - 0.02);
    k.on(r, f);
    k.on(r, l);
    assert_eq!(k.dof(), 0);
    let rx = k.measure(DimKind::CoordX(r));
    k.check_value(rx, 4.0 - 2.0 * 6f64.sqrt());
}

/// Tangent at P (x = 1) to y² = x³ − 2x + 2 meets the curve again at R:
/// x_R = m² − 2x_P with m = (3x_P² − 2)/(2y_P) = 1/2: x_R = −7/4.
#[test]
fn elliptic_tangent_second_point() {
    let mut k = Kit::new();
    let f = k.curve("y^2 = x^3 - 2*x + 2");
    let p = k.point(1.0, 1.0);
    k.on(p, f);
    k.drive(DimKind::CoordX(p), "1");
    let t = k.point(2.0, 1.5);
    let l = k.inf_line(p, t);
    k.tangent(l, f);
    let r = k.point(-1.73, 1.0 + 0.5 * (-2.75) - 0.02);
    k.on(r, f);
    k.on(r, l);
    let rx = k.measure(DimKind::CoordX(r));
    k.check_value(rx, -1.75);
}

/// √x + x² = 3: squaring gives a quartic.
#[test]
fn sqrt_plus_square() {
    let mut k = Kit::new();
    let x = at_height(&mut k, "y = sqrt(x) + x^2", "3", 1.4, 3.0);
    k.check_value(x, zero_near(|x| x.sqrt() + x * x - 3.0, 1.4));
}

/// x^(1/3) + x = 3: a cubic in t = x^(1/3).
#[test]
fn cube_root_plus_x() {
    let mut k = Kit::new();
    let x = at_height(&mut k, "y = x^(1/3) + x", "3", 1.7, 3.0);
    k.check_value(x, zero_near(|x| x.cbrt() + x - 3.0, 1.7));
}

/// x³ + 1/x = 3: x⁴ − 3x + 1 = 0.
#[test]
fn cubic_plus_reciprocal() {
    let mut k = Kit::new();
    let x = at_height(&mut k, "y = x^3 + 1/x", "3", 1.3, 3.0);
    k.check_value(x, zero_near(|x| x.powi(4) - 3.0 * x + 1.0, 1.3));
}

/// Elliptic curve y² = x³ − x at height 1: x³ − x − 1 = 0.
#[test]
fn elliptic_curve_at_height() {
    let mut k = Kit::new();
    let x = at_height(&mut k, "y^2 = x^3 - x", "1", 1.32, 1.0);
    k.check_value(x, zero_near(|x| x.powi(3) - x - 1.0, 1.32));
}

/// x⁹ − 3x³ + 1 = 0: a cubic in x³ with three real roots; the negative one
/// (x³ = 2cos(7π/9)·… < 0) needs the real cube root of a trig form.
#[test]
fn nonic_trig_cube_negative() {
    let mut k = Kit::new();
    let x = on_axis(&mut k, "y = x^9 - 3*x^3 + 1", -1.23);
    k.check_value(x, zero_near(|x| x.powi(9) - 3.0 * x.powi(3) + 1.0, -1.23));
}

/// x⁶ − 3x² + 1 = 0: a cubic in x² with three real roots.
#[test]
fn sextic_trig_square() {
    let mut k = Kit::new();
    let x = on_axis(&mut k, "y = x^6 - 3*x^2 + 1", 0.59);
    k.check_value(x, zero_near(|x| x.powi(6) - 3.0 * x * x + 1.0, 0.59));
}

/// (x² + x)³ − 3(x² + x) + 1 = 0: a trig cubic in u = x² + x, then a quadratic
/// (decomposition with a non-monomial inner quadratic).
#[test]
fn decomposable_sextic_inner_quadratic() {
    let mut k = Kit::new();
    let x = on_axis(&mut k, "y = (x^2 + x)^3 - 3*(x^2 + x) + 1", 0.97);
    k.check_value(x, zero_near(|x| (x * x + x).powi(3) - 3.0 * (x * x + x) + 1.0, 0.97));
}
