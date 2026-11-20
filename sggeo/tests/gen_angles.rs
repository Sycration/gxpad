//! Angle-centred constructions: driving angles of all sizes, polar walks,
//! sine/cosine rule setups, circle theorems, polygons and mechanisms.

use sggeo::model::DimKind;
mod testkit;
use testkit::Kit;
use std::f64::consts::PI;


/// Label position inside the sector between the direction vectors of two
/// lines (the unsigned angle between their directions), judged from the
/// current layout. The label position selects the sector the dimension shows.
fn label(k: &Kit, l1: usize, l2: usize) -> [f64; 2] {
    let d = k.doc();
    let dir = |l: usize| {
        let (a, b) = d.line_pts(l);
        let (a, b) = (d.pval(a), d.pval(b));
        (a, [b[0] - a[0], b[1] - a[1]])
    };
    let (p1, u) = dir(l1);
    let (p2, w) = dir(l2);
    let cr = u[0] * w[1] - u[1] * w[0];
    assert!(cr.abs() > 1e-9, "lines are parallel in the initial layout");
    let t = ((p2[0] - p1[0]) * w[1] - (p2[1] - p1[1]) * w[0]) / cr;
    let v = [p1[0] + t * u[0], p1[1] + t * u[1]];
    let (nu, nw) = (u[0].hypot(u[1]), w[0].hypot(w[1]));
    let b = [u[0] / nu + w[0] / nw, u[1] / nu + w[1] / nw];
    let nb = b[0].hypot(b[1]);
    [v[0] + b[0] / nb, v[1] + b[1] / nb]
}

fn adrive(k: &mut Kit, l1: usize, l2: usize, val: &str) -> usize {
    let lw = label(k, l1, l2);
    let i = k.geo.sketch_mut().add_dimension(DimKind::Angle(l1, l2), lw).expect("dimension rejected");
    assert!(k.doc().dims[i].as_ref().unwrap().driving, "angle already determined");
    k.set(i, val);
    i
}

fn ameas(k: &mut Kit, l1: usize, l2: usize) -> usize {
    let lw = label(k, l1, l2);
    let i = k.geo.sketch_mut().add_dimension(DimKind::Angle(l1, l2), lw).expect("dimension rejected");
    if k.doc().dims[i].as_ref().unwrap().driving {
        k.geo.sketch_mut().toggle_driving(i);
    }
    i
}

fn rad(d: f64) -> f64 {
    d.to_radians()
}

fn polar(c: [f64; 2], r: f64, deg: f64) -> (f64, f64) {
    (c[0] + r * rad(deg).cos(), c[1] + r * rad(deg).sin())
}

/// Initial angle ~20 degrees, driven to 150: the measured value must be 150.
#[test]
fn drive_far_from_initial_value() {
    let mut k = Kit::new();
    let o = k.origin();
    let p = k.point(3.0, 1.1);
    let seg = k.line(o, p);
    k.drive(DimKind::Length(seg), "3.2");
    let xa = k.xaxis();
    let a = adrive(&mut k, xa, seg, "150");
    assert_eq!(k.dof(), 0);
    k.check_value(a, rad(150.0));
    let x = k.measure(DimKind::CoordX(p));
    let y = k.measure(DimKind::CoordY(p));
    k.check_value(x, 3.2 * rad(150.0).cos());
    k.check_value(y, 3.2 * rad(150.0).sin());
}

/// Right triangle legs from hypotenuse and an angle, for many angle sizes
/// including near 0 and near 180 degrees.
#[test]
fn angle_size_sweep_legs() {
    for deg in [1.0, 5.0, 30.0, 45.0, 89.0, 90.0, 91.0, 120.0, 150.0, 175.0, 179.0] {
        let mut k = Kit::new();
        let o = k.origin();
        let p = k.point(2.0, 1.0);
        let seg = k.line(o, p);
        k.drive(DimKind::Length(seg), "10");
        let xa = k.xaxis();
        adrive(&mut k, xa, seg, &format!("{deg}"));
        assert_eq!(k.dof(), 0);
        let x = k.measure(DimKind::CoordX(p));
        let y = k.measure(DimKind::CoordY(p));
        k.check_value(x, 10.0 * rad(deg).cos());
        k.check_value(y, 10.0 * rad(deg).sin());
    }
}

/// Angle between the x axis and segments pointing into every quadrant,
/// determined by coordinates.
#[test]
fn axis_angle_all_quadrants() {
    for (x, y) in [(3.0, 2.0), (-3.0, 2.0), (-3.0, -2.0), (3.0, -2.0), (0.5, 4.0), (-4.0, -0.5)] {
        let mut k = Kit::new();
        let o = k.origin();
        let p = k.point(x * 1.1, y * 0.9);
        let seg = k.line(o, p);
        k.drive(DimKind::CoordX(p), &format!("{x}"));
        k.drive(DimKind::CoordY(p), &format!("{y}"));
        assert_eq!(k.dof(), 0);
        let xa = k.xaxis();
        let a = ameas(&mut k, xa, seg);
        let r = (x * x + y * y).sqrt();
        k.check_value(a, (x / r).acos());
    }
}

/// Two segments from a shared vertex, floating freely: the third side
/// follows from the cosine rule.
#[test]
fn floating_cosine_rule() {
    for deg in [25.0, 110.0, 170.0] {
        let mut k = Kit::new();
        let b = k.point(0.0, 0.0);
        let (ax, ay) = polar([0.0, 0.0], 3.0, 10.0);
        let (cx, cy) = polar([0.0, 0.0], 4.0, 10.0 + deg);
        let a = k.point(ax, ay);
        let c = k.point(cx, cy);
        let ba = k.line(b, a);
        let bc = k.line(b, c);
        k.drive(DimKind::Length(ba), "3");
        k.drive(DimKind::Length(bc), "4");
        adrive(&mut k, ba, bc, &format!("{deg}"));
        assert_eq!(k.dof(), 3);
        let d = k.measure(DimKind::Distance(a, c));
        k.check_value(d, (25.0 - 24.0 * rad(deg).cos()).sqrt());
    }
}

/// Polar walk with a pinned start: lengths and turning angles, then the
/// distance back to the start. The first segment may rotate (dof 1).
#[test]
fn polar_walk_free_rotation() {
    let mut k = Kit::new();
    let o = k.origin();
    let lens = [2.0, 1.5, 3.0];
    let turns = [30.0, 50.0];
    let mut pts = vec![o];
    let mut h = 5.0;
    let mut pos = [0.0, 0.0];
    for i in 0..3 {
        if i > 0 {
            h += turns[i - 1] * 1.1;
        }
        let (x, y) = polar(pos, lens[i], h);
        pos = [x, y];
        pts.push(k.point(x, y));
    }
    let l: Vec<usize> = (0..3).map(|i| k.line(pts[i], pts[i + 1])).collect();
    for i in 0..3 {
        k.drive(DimKind::Length(l[i]), &format!("{}", lens[i]));
    }
    adrive(&mut k, l[0], l[1], "30");
    adrive(&mut k, l[1], l[2], "50");
    assert!(k.dof() > 0);
    let d = k.measure(DimKind::Distance(o, pts[3]));
    let (mut x, mut y, mut hh) = (0.0, 0.0, 0.0f64);
    for i in 0..3 {
        if i > 0 {
            hh += turns[i - 1];
        }
        x += lens[i] * rad(hh).cos();
        y += lens[i] * rad(hh).sin();
    }
    k.check_value(d, x.hypot(y));
}

/// Longer polar walk with a pinned first direction, large turning angles
/// (more than 180 degrees of total turning).
#[test]
fn polar_walk_large_turning() {
    let mut k = Kit::new();
    let o = k.origin();
    let lens = [2.0, 1.0, 1.5, 2.5, 1.2];
    let turns = [80.0, 100.0, 70.0, 90.0];
    let mut pts = vec![o];
    let mut pos = [0.0, 0.0];
    let mut h = 0.0;
    for i in 0..5 {
        if i > 0 {
            h += turns[i - 1] + 6.0;
        }
        let (x, y) = polar(pos, lens[i], h);
        pos = [x, y];
        pts.push(k.point(x, y));
    }
    let l: Vec<usize> = (0..5).map(|i| k.line(pts[i], pts[i + 1])).collect();
    k.horizontal(l[0]);
    for i in 0..5 {
        k.drive(DimKind::Length(l[i]), &format!("{}", lens[i]));
    }
    for i in 0..4 {
        adrive(&mut k, l[i], l[i + 1], &format!("{}", turns[i]));
    }
    assert_eq!(k.dof(), 0);
    let d = k.measure(DimKind::Distance(o, pts[5]));
    let (mut x, mut y, mut hh) = (0.0, 0.0, 0.0f64);
    for i in 0..5 {
        if i > 0 {
            hh += turns[i - 1];
        }
        x += lens[i] * rad(hh).cos();
        y += lens[i] * rad(hh).sin();
    }
    k.check_value(d, x.hypot(y));
}

/// ASA: two angles and the included side give the other sides (sine rule).
#[test]
fn asa_sine_rule_floating() {
    let (da, db, c) = (40.0, 65.0, 5.0);
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(5.0, 0.1);
    let dc = 180.0 - da - db;
    let cs = c * rad(db).sin() / rad(dc).sin();
    let (px, py) = polar([0.0, 0.0], cs * 1.05, da);
    let p = k.point(px, py);
    let ab = k.line(a, b);
    let ac = k.line(a, p);
    let ba = k.line(b, a);
    let bc = k.line(b, p);
    k.drive(DimKind::Length(ab), "5");
    adrive(&mut k, ab, ac, "40");
    adrive(&mut k, ba, bc, "65");
    assert_eq!(k.dof(), 3);
    let bcl = k.measure(DimKind::Length(bc));
    let acl = k.measure(DimKind::Length(ac));
    k.check_value(bcl, c * rad(da).sin() / rad(dc).sin());
    k.check_value(acl, cs);
}

/// SSA with both branches: the intended one follows the initial layout.
#[test]
fn ssa_ambiguous_case_both_branches() {
    let (ang_a, a_len, b_len) = (35.0f64, 4.0f64, 6.0f64);
    let sin_b = b_len * rad(ang_a).sin() / a_len;
    for obtuse in [false, true] {
        let mut deg_b = sin_b.asin().to_degrees();
        if obtuse {
            deg_b = 180.0 - deg_b;
        }
        let deg_c = 180.0 - ang_a - deg_b;
        let c_len = a_len * rad(deg_c).sin() / rad(ang_a).sin();
        let mut k = Kit::new();
        let a = k.point(0.0, 0.0);
        let bx = k.point(c_len + 0.05, 0.05);
        // C from A at angle A, distance b
        let (cx, cy) = polar([0.0, 0.0], b_len, ang_a);
        let c = k.point(cx - 0.05, cy + 0.05);
        let ab = k.line(a, bx);
        let ac = k.line(a, c);
        let bc = k.line(bx, c);
        adrive(&mut k, ab, ac, "35");
        k.drive(DimKind::Length(bc), "4");
        k.drive(DimKind::Length(ac), "6");
        assert_eq!(k.dof(), 3);
        let l = k.measure(DimKind::Length(ab));
        if obtuse {
            // the second branch has B obtuse, C acute: c = a sin C / sin A
            assert!(c_len > 0.0);
        }
        k.check_value(l, c_len);
    }
}

/// Angle between a line and each axis: the complementary angle follows.
#[test]
fn line_angle_to_both_axes() {
    let mut k = Kit::new();
    let p0 = k.fixed_point(1.0, 1.0);
    let p = k.point(3.0, 4.0);
    let l = k.line(p0, p);
    let ya = k.yaxis();
    let xa = k.xaxis();
    adrive(&mut k, ya, l, "25");
    assert!(k.dof() > 0);
    let a = ameas(&mut k, xa, l);
    k.check_value(a, rad(65.0));
}

/// Inscribed angle theorem with a point sliding on the circle.
#[test]
fn inscribed_angle_sliding_point() {
    let mut k = Kit::new();
    let o = k.origin();
    let circ = k.circle(o, 3.0);
    let a = k.point(3.0, 0.1);
    let b = k.point(3.0 * rad(100.0).cos(), 3.0 * rad(100.0).sin());
    let p = k.point(3.0 * rad(220.0).cos(), 3.0 * rad(220.0).sin());
    k.on(a, circ);
    k.on(b, circ);
    k.on(p, circ);
    k.drive(DimKind::Radius(circ), "3");
    k.drive(DimKind::CoordY(a), "0");
    let oa = k.line(o, a);
    let ob = k.line(o, b);
    adrive(&mut k, oa, ob, "100");
    let pa = k.line(p, a);
    let pb = k.line(p, b);
    assert_eq!(k.dof(), 1);
    let ang = ameas(&mut k, pa, pb);
    k.check_value(ang, rad(50.0));
}

/// Tangent-chord angle equals half the central angle.
#[test]
fn tangent_chord_angle() {
    let mut k = Kit::new();
    let o = k.origin();
    let circ = k.circle(o, 2.0);
    let a = k.point(2.0, 0.0);
    let b = k.point(0.4, 1.9);
    let t = k.point(2.0, 1.5);
    k.on(a, circ);
    k.on(b, circ);
    k.drive(DimKind::Radius(circ), "2");
    k.drive(DimKind::CoordY(a), "0");
    let tl = k.line(a, t);
    k.tangent(tl, circ);
    let oa = k.line(o, a);
    let ob = k.line(o, b);
    adrive(&mut k, oa, ob, "80");
    let ab = k.line(a, b);
    assert!(k.dof() > 0);
    let ang = ameas(&mut k, tl, ab);
    k.check_value(ang, rad(40.0));
}

/// Regular pentagon from equal sides and equal turning angles of 72 degrees;
/// the closing side equals the others and the diagonal is 4cos(36).
#[test]
fn regular_pentagon_turning_angles() {
    let mut k = Kit::new();
    let o = k.origin();
    let mut pts = vec![o];
    let mut pos = [0.0, 0.0];
    for i in 0..4 {
        let (x, y) = polar(pos, 2.0, 72.0 * i as f64 + 3.0);
        pos = [x, y];
        pts.push(k.point(x, y));
    }
    let l: Vec<usize> = (0..4).map(|i| k.line(pts[i], pts[i + 1])).collect();
    k.drive(DimKind::Length(l[0]), "2");
    for i in 1..4 {
        k.equal(l[0], l[i]);
    }
    let t = k.var("t", 72.0);
    for i in 0..3 {
        adrive(&mut k, l[i], l[i + 1], "t * deg");
    }
    assert!(k.dof() > 0);
    let close = k.measure(DimKind::Distance(pts[4], o));
    let diag = k.measure(DimKind::Distance(pts[0], pts[2]));
    k.check_value(close, 2.0);
    k.check_value(diag, 4.0 * rad(36.0).cos());
    // a square instead: turning angle 90 gives a closing side of 2
    k.set_var(t, 90.0);
    k.check_value(diag, 2.0 * 2.0f64.sqrt());
}

/// Angle driven by an expression of a variable; the angle is in degrees.
#[test]
fn angle_expression_of_variable() {
    let mut k = Kit::new();
    let o = k.origin();
    let a = k.var("a", 40.0);
    let p = k.point(2.0, 2.5);
    let seg = k.line(o, p);
    k.drive(DimKind::Length(seg), "4");
    let xa = k.xaxis();
    let ang = adrive(&mut k, xa, seg, "(2a + 10) * deg");
    let y = k.measure(DimKind::CoordY(p));
    k.check_value(ang, rad(90.0));
    k.check_value(y, 4.0);
    k.set_var(a, 70.0);
    k.check_value(y, 4.0 * rad(150.0).sin());
    k.set_var(a, 10.0);
    k.check_value(y, 4.0 * rad(30.0).sin());
}

/// Free-floating rectangle: the diagonal angle follows from the sides.
#[test]
fn rectangle_diagonal_angle() {
    let (w, h) = (4.0f64, 2.5f64);
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(4.1, 0.2);
    let c = k.point(4.0, 2.7);
    let d = k.point(-0.1, 2.4);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let cd = k.line(c, d);
    let da = k.line(d, a);
    k.perpendicular(ab, bc);
    k.perpendicular(bc, cd);
    k.perpendicular(cd, da);
    k.drive(DimKind::Length(ab), "4");
    k.drive(DimKind::Length(bc), "2.5");
    assert_eq!(k.dof(), 3);
    let ac = k.line(a, c);
    let ang = ameas(&mut k, ab, ac);
    k.check_value(ang, (h / w).atan());
    let bd = k.line(b, d);
    let ang2 = ameas(&mut k, ac, bd);
    // diagonals meet at angle 2*atan(h/w) or its supplement
    let t = 2.0 * (h / w).atan();
    let v = k.value(ang2);
    assert!((v - t).abs() < 1e-9 || (v - (PI - t)).abs() < 1e-9, "diagonal angle {v}");
    k.check(ang2);
}

/// Rhombus with a driven angle: both diagonals from side and angle.
#[test]
fn rhombus_diagonals() {
    let mut k = Kit::new();
    let s = 3.0;
    let a = k.point(0.0, 0.0);
    let (bx, by) = polar([0.0, 0.0], s, 5.0);
    let (dx, dy) = polar([0.0, 0.0], s, 80.0);
    let b = k.point(bx, by);
    let d = k.point(dx, dy);
    let c = k.point(bx + dx, by + dy + 0.2);
    let ab = k.line(a, b);
    let ad = k.line(a, d);
    let bc = k.line(b, c);
    let cd = k.line(c, d);
    k.equal(ab, bc);
    k.equal(bc, cd);
    k.equal(cd, ad);
    k.drive(DimKind::Length(ab), "3");
    adrive(&mut k, ab, ad, "70");
    assert!(k.dof() > 0);
    let ac = k.measure(DimKind::Distance(a, c));
    let bd = k.measure(DimKind::Distance(b, d));
    k.check_value(ac, 2.0 * s * rad(35.0).cos());
    k.check_value(bd, 2.0 * s * rad(35.0).sin());
}

/// Angle bisector tool: the bisector makes half the angle with each line.
#[test]
fn bisector_half_angle() {
    let mut k = Kit::new();
    let o = k.origin();
    let p = k.point(4.0, 0.2);
    let q = k.point(1.0, 3.5);
    let l1 = k.line(o, p);
    let l2 = k.line(o, q);
    k.horizontal(l1);
    k.drive(DimKind::Length(l1), "4");
    adrive(&mut k, l1, l2, "75");
    let (bl, _) = k.angle_bisector(l1, l2, [2.0, 1.0]);
    let half = ameas(&mut k, l1, bl);
    let v = k.value(half);
    assert!((v - rad(37.5)).abs() < 1e-9 || (v - rad(142.5)).abs() < 1e-9, "bisector angle {v}");
    k.check(half);
}

/// Parallel lines cut by a transversal: corresponding angles agree.
#[test]
fn parallel_lines_transversal() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(4.0, 0.3);
    let c = k.point(1.0, 2.0);
    let d = k.point(5.0, 2.3);
    let t0 = k.point(2.0, -1.0);
    let t1 = k.point(3.0, 4.0);
    let l1 = k.line(a, b);
    let l2 = k.line(c, d);
    let t = k.line(t0, t1);
    k.parallel(l1, l2);
    adrive(&mut k, t, l1, "65");
    assert!(k.dof() > 0);
    let m = ameas(&mut k, t, l2);
    let v = k.value(m);
    assert!((v - rad(65.0)).abs() < 1e-9 || (v - rad(115.0)).abs() < 1e-9, "{v}");
    k.check(m);
}

/// Triangle angle sum: the third angle is 180 minus the other two.
#[test]
fn triangle_angle_sum() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(6.0, 0.4);
    let c = k.point(2.0, 3.0);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let ba = k.line(b, a);
    let bc = k.line(b, c);
    let ca = k.line(c, a);
    let cb = k.line(c, b);
    adrive(&mut k, ab, ac, "50");
    adrive(&mut k, ba, bc, "60");
    assert!(k.dof() > 0);
    let g = ameas(&mut k, ca, cb);
    k.check_value(g, rad(70.0));
}

/// Isosceles triangles with extreme apex angles: base angles follow.
#[test]
fn isosceles_extreme_apex() {
    for apex in [2.0f64, 20.0, 160.0, 178.0] {
        let mut k = Kit::new();
        let a = k.point(0.0, 0.0);
        let (bx, by) = polar([0.0, 0.0], 3.0, 0.0);
        let (cx, cy) = polar([0.0, 0.0], 3.0, apex);
        let b = k.point(bx, by + 0.05);
        let c = k.point(cx, cy);
        let ab = k.line(a, b);
        let ac = k.line(a, c);
        let ba = k.line(b, a);
        let bc = k.line(b, c);
        k.equal(ab, ac);
        k.drive(DimKind::Length(ab), "3");
        adrive(&mut k, ab, ac, &format!("{apex}"));
        assert_eq!(k.dof(), 3);
        let base = ameas(&mut k, ba, bc);
        k.check_value(base, rad((180.0 - apex) / 2.0));
    }
}

/// Two floating segments driven from 15 degrees to 170: the distance
/// between the free ends follows.
#[test]
fn floating_lines_far_drive() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let p = k.point(3.0, 0.8);
    let q = k.point(2.6, 0.4);
    let l1 = k.line(a, p);
    let l2 = k.line(a, q);
    k.drive(DimKind::Length(l1), "3");
    k.drive(DimKind::Length(l2), "2");
    let ang = adrive(&mut k, l1, l2, "170");
    assert_eq!(k.dof(), 3);
    let d = k.measure(DimKind::Distance(p, q));
    k.check_value(ang, rad(170.0));
    k.check_value(d, (13.0 - 12.0 * rad(170.0).cos()).sqrt());
}

/// Exterior angle theorem on a chain a-b-c: the turning angle at b equals
/// the sum of the two remote interior angles.
#[test]
fn exterior_angle_theorem() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(5.0, 0.3);
    let c = k.point(3.0, 3.0);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let ac = k.line(a, c);
    let ca = k.line(c, a);
    let cb = k.line(c, b);
    adrive(&mut k, ab, ac, "35"); // interior angle at a
    adrive(&mut k, ca, cb, "60"); // interior angle at c
    assert!(k.dof() > 0);
    let ext = ameas(&mut k, ab, bc); // turning angle at b
    k.check_value(ext, rad(95.0));
}

/// Law of sines: circumradius = BC / (2 sin A) for a triangle inscribed in a
/// circle.
#[test]
fn circumradius_law_of_sines() {
    let mut k = Kit::new();
    let o = k.point(2.0, 1.0);
    let circ = k.circle(o, 3.0);
    let (ax, ay) = polar([2.0, 1.0], 3.0, 90.0);
    let (bx, by) = polar([2.0, 1.0], 3.0, 210.0);
    let (cx, cy) = polar([2.0, 1.0], 3.0, 330.0);
    let a = k.point(ax, ay);
    let b = k.point(bx, by);
    let c = k.point(cx, cy);
    k.on(a, circ);
    k.on(b, circ);
    k.on(c, circ);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    adrive(&mut k, ab, ac, "30");
    k.drive(DimKind::Length(bc), "5");
    assert!(k.dof() > 0);
    let r = k.measure(DimKind::Radius(circ));
    k.check_value(r, 5.0 / (2.0 * rad(30.0).sin()));
}

/// Slider-crank: crank angle and rod length give the slider position.
#[test]
fn slider_crank_position() {
    let (cr, rod) = (2.0f64, 3.0f64);
    for th in [15.0f64, 75.0, 120.0] {
        let mut k = Kit::new();
        let o = k.origin();
        let (cx, cy) = polar([0.0, 0.0], cr, th * 0.9);
        let c = k.point(cx, cy);
        let x = cr * rad(th).cos() + (rod * rod - cr * cr * rad(th).sin().powi(2)).sqrt();
        let p = k.point(x + 0.1, 0.1);
        let xa = k.xaxis();
        let crank = k.line(o, c);
        let rodl = k.line(c, p);
        k.on(p, xa);
        k.drive(DimKind::Length(crank), "2");
        k.drive(DimKind::Length(rodl), "3");
        adrive(&mut k, xa, crank, &format!("{th}"));
        assert_eq!(k.dof(), 0);
        let px = k.measure(DimKind::CoordX(p));
        k.check_value(px, x);
    }
}

/// Slider-crank driven from the slider: the crank angle is a pure function
/// of the slider position (cosine rule). The mechanism is not locked in a
/// pose by the angle, so the crank angle is only measured.
#[test]
fn slider_crank_inverse() {
    let mut k = Kit::new();
    let o = k.origin();
    let (cx, cy) = polar([0.0, 0.0], 2.0, 40.0);
    let c = k.point(cx, cy);
    let p = k.point(4.0, 0.0);
    let xa = k.xaxis();
    let crank = k.line(o, c);
    let rodl = k.line(c, p);
    k.on(p, xa);
    k.drive(DimKind::Length(crank), "2");
    k.drive(DimKind::Length(rodl), "3");
    k.drive(DimKind::CoordX(p), "4");
    assert_eq!(k.dof(), 0);
    let ang = ameas(&mut k, xa, crank);
    k.check_value(ang, (11.0f64 / 16.0).acos());
}

/// Intersecting circles: the angle subtended at the intersection point.
#[test]
fn circle_intersection_angle() {
    let mut k = Kit::new();
    let o = k.origin();
    let c2 = k.point(3.9, 0.1);
    let ca = k.circle(o, 3.0);
    let cb = k.circle(c2, 2.5);
    k.drive(DimKind::Radius(ca), "3");
    k.drive(DimKind::Radius(cb), "2.5");
    k.drive(DimKind::CoordX(c2), "4");
    k.drive(DimKind::CoordY(c2), "0");
    let x = k.intersect(ca, cb, [1.8, 2.3]);
    let xo = k.line(x, o);
    let xc = k.line(x, c2);
    assert_eq!(k.dof(), 0);
    let ang = ameas(&mut k, xo, xc);
    k.check_value(ang, ((9.0f64 + 6.25 - 16.0) / (2.0 * 3.0 * 2.5)).acos());
}

/// Angles about one vertex add up; the sum exceeds 180 so the angle between
/// the outer rays wraps around.
#[test]
fn adjacent_angles_sum_wraps() {
    let mut k = Kit::new();
    let o = k.origin();
    let p1 = k.point(3.0, 0.2);
    let (x2, y2) = polar([0.0, 0.0], 3.0, 95.0);
    let (x3, y3) = polar([0.0, 0.0], 3.0, 205.0);
    let p2 = k.point(x2, y2);
    let p3 = k.point(x3, y3);
    let l1 = k.line(o, p1);
    let l2 = k.line(o, p2);
    let l3 = k.line(o, p3);
    k.horizontal(l1);
    adrive(&mut k, l1, l2, "100");
    adrive(&mut k, l2, l3, "120");
    assert!(k.dof() > 0);
    let outer = ameas(&mut k, l1, l3);
    // the rays are at 0, 100, 220 degrees, so the angle l1-l3 is 140
    k.check_value(outer, rad(140.0));
}

/// Exterior polygon: interior angles of a floating quadrilateral sum to 360
/// (the fourth angle is determined by the other three).
#[test]
fn quadrilateral_angle_sum() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(4.0, 0.2);
    let c = k.point(4.5, 3.5);
    let d = k.point(0.5, 3.0);
    let ab = k.line(a, b);
    let ad = k.line(a, d);
    let ba = k.line(b, a);
    let bc = k.line(b, c);
    let cb = k.line(c, b);
    let cd = k.line(c, d);
    let dc = k.line(d, c);
    let da = k.line(d, a);
    k.drive(DimKind::Length(ab), "4");
    k.drive(DimKind::Length(ad), "3");
    adrive(&mut k, ab, ad, "85");
    adrive(&mut k, ba, bc, "95");
    adrive(&mut k, da, dc, "100");
    assert_eq!(k.dof(), 3);
    let g = ameas(&mut k, cb, cd);
    k.check_value(g, rad(80.0));
}

/// Triangle with sides a = BC, b = CA and angle A = 3·angle B (solved
/// variable t = B): cos t = √((a + b)/(4b)), c = 4b·cos t·cos 2t.
#[test]
fn triangle_angle_triple() {
    let mut k = Kit::new();
    // b = 2, a = 4: cos t = √(6/8), t = 30°, A = 90°, C = 60°, c = 2√3
    k.solved_var("t", 0.5);
    let a = k.point(0.0, 0.0);
    let b = k.point(3.4, 0.05);
    let c = k.point(0.05, 2.1);
    let ab = k.line(a, b);
    let ac = k.line(a, c);
    let bc = k.line(b, c);
    let ba = k.line(b, a);
    k.drive(DimKind::Length(bc), "4");
    k.drive(DimKind::Length(ac), "2");
    k.drive_angle(ab, ac, "3*t");
    k.drive_angle(ba, bc, "t");
    let d = k.measure(DimKind::Length(ab));
    k.check_value(d, 2.0 * 3f64.sqrt());
}

/// Triangle with angle B = 2·angle C (solved variable t = C) and sides
/// b = CA = 3, c = AB = 2: a = (b² − c²)/c = 5/2.
#[test]
fn triangle_angle_double() {
    let mut k = Kit::new();
    k.solved_var("t", 0.72);
    // C = acos(3/4) ≈ 0.7227, B = 2C, A = π − 3C
    let cc = (0.75f64).acos();
    let bpt = k.point(0.0, 0.0);
    let cpt = k.point(2.5, 0.03);
    let apt = k.point(2.0 * (2.0 * cc).cos() + 0.02, 2.0 * (2.0 * cc).sin());
    let bc = k.line(bpt, cpt);
    let ba = k.line(bpt, apt);
    let cb = k.line(cpt, bpt);
    let ca = k.line(cpt, apt);
    k.drive(DimKind::Length(ca), "3");
    k.drive(DimKind::Length(ba), "2");
    k.drive_angle(bc, ba, "2*t");
    k.drive_angle(ca, cb, "t");
    let a = k.measure(DimKind::Length(bc));
    k.check_value(a, 2.5);
}

/// Angle of OP to the x axis is fixed by P's coordinates and driven as 3t
/// (t solved); Q at unit distance at angle t: y_Q = sin(atan2(2, 3)/3).

#[test]
fn triple_angle_solved_var() {
    let mut k = Kit::new();
    let o = k.origin();
    let xa = k.xaxis();
    
    k.solved_var("t", 0.25);
    let a: f64 = (2.0f64).atan2(3.0) / 3.0;
    let q = k.point(a.cos() * 1.1, a.sin());
    let oq = k.line(o, q);
    k.drive(DimKind::Length(oq), "1");
    k.drive_angle(xa, oq, "t");
    let p = k.point(3.0 * 1.05, 2.0);
    let op = k.line(o, p);
    k.drive_angle(xa, op, "3*t");
    k.drive(DimKind::CoordX(p), "3");
    k.drive(DimKind::CoordY(p), "2");
    assert_eq!(k.dof(), 0);
    let y = k.measure(DimKind::CoordY(q));
    k.check_value(y, a.sin());
}

/// Same with factor 2 (half angle): y_Q = sin(atan2(2, a)/2).

#[test]
fn double_angle_solved_var() {
    let mut k = Kit::new();
    let o = k.origin();
    let xa = k.xaxis();
    k.var("a", 3.0);
    k.solved_var("t", 0.25);
    let a: f64 = (2.0f64).atan2(3.0) / 2.0;
    let q = k.point(a.cos() * 1.1, a.sin());
    let oq = k.line(o, q);
    k.drive(DimKind::Length(oq), "1");
    k.drive_angle(xa, oq, "t");
    let p = k.point(3.0 * 1.05, 2.0);
    let op = k.line(o, p);
    k.drive_angle(xa, op, "2*t");
    k.drive(DimKind::CoordX(p), "a");
    k.drive(DimKind::CoordY(p), "2");
    assert_eq!(k.dof(), 0);
    let y = k.measure(DimKind::CoordY(q));
    k.check_value(y, a.sin());
}

/// Angle t + 30° driven, t solved: y_Q = sin(atan2(2, a) − 30°).

#[test]
fn offset_angle_solved_var() {
    let mut k = Kit::new();
    let o = k.origin();
    let xa = k.xaxis();
    k.var("a", 1.0);
    k.solved_var("t", 0.25);
    let a: f64 = (2.0f64).atan2(1.0) - rad(30.0);
    let q = k.point(a.cos() * 1.1, a.sin());
    let oq = k.line(o, q);
    k.drive(DimKind::Length(oq), "1");
    k.drive_angle(xa, oq, "t");
    let p = k.point(1.0 * 1.05, 2.0);
    let op = k.line(o, p);
    k.drive_angle(xa, op, "t + 30°");
    k.drive(DimKind::CoordX(p), "a");
    k.drive(DimKind::CoordY(p), "2");
    assert_eq!(k.dof(), 0);
    let y = k.measure(DimKind::CoordY(q));
    k.check_value(y, a.sin());
}

fn dim_name(k: &Kit, d: usize) -> String {
    k.doc().dims[d].as_ref().unwrap().name.clone()
}

/// θ2 = θ1 / 3 driven by a dimension expression; leg y = L sin(θ1/3).
#[test]
fn angle_expression_of_dimension() {
    let mut k = Kit::new();
    let o = k.origin();
    let xa = k.xaxis();
    k.var("L", 2.0);
    let p = k.point(1.0, 2.0);
    let op = k.line(o, p);
    k.drive(DimKind::Length(op), "L");
    let t1 = k.drive_angle(xa, op, "100");
    let n1 = dim_name(&k, t1);
    let q = k.point(2.0, 0.5);
    let oq = k.line(o, q);
    k.drive(DimKind::Length(oq), "L");
    k.drive_angle(xa, oq, &format!("{n1}/3"));
    assert_eq!(k.dof(), 0);
    let d = k.measure(DimKind::Distance(p, q));
    // chord between angles 100 and 100/3 deg
    let ch = 2.0 * 2.0 * (rad(100.0 - 100.0 / 3.0) / 2.0).sin();
    k.check_value(d, ch);
}

/// Inverse trig in a driven expression: angle acos(a/b), y = L sin(acos(a/b)).
#[test]
fn inverse_trig_driven_angle() {
    let mut k = Kit::new();
    let o = k.origin();
    let xa = k.xaxis();
    k.var("a", 1.0);
    k.var("b", 3.0);
    k.var("L", 2.0);
    let p = k.point(1.0, 2.0);
    let op = k.line(o, p);
    k.drive(DimKind::Length(op), "L");
    k.drive_angle(xa, op, "acos(a/b)");
    let y = k.measure(DimKind::CoordY(p));
    let s = k.check_value(y, 2.0 * (1.0f64 - 1.0 / 9.0).sqrt());
    assert!(!s.contains("acos"), "not simplified: {s}");
}

/// Angle driven by atan2 of vars, at a supplementary-quadrant value.
#[test]
fn atan2_driven_angle_obtuse() {
    let mut k = Kit::new();
    let o = k.origin();
    let xa = k.xaxis();
    k.var("a", -2.0);
    k.var("b", 1.0);
    let p = k.point(-2.0, 1.2);
    let op = k.line(o, p);
    k.drive(DimKind::Length(op), "5");
    k.drive_angle(xa, op, "atan2(b, a)");
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, 5.0 * (-2.0 / 5.0f64.sqrt()));
}

/// Three rays at turning angles 150°, 150°: the outer rays are 60° apart
/// (total turning 300°, wraps past 180 and 360 is not reached).
#[test]
fn turning_walk_wraps_past_pi() {
    let mut k = Kit::new();
    let o = k.origin();
    let pts: Vec<usize> = [0.0f64, 150.0, 300.0].iter().map(|d| k.point(3.0 * rad(d + 5.0).cos(), 3.0 * rad(d + 5.0).sin())).collect();
    let l: Vec<usize> = pts.iter().map(|&p| k.line(o, p)).collect();
    k.horizontal(l[0]);
    k.var("a", 150.0);
    k.drive_angle(l[0], l[1], "a*deg");
    k.drive_angle(l[1], l[2], "a*deg");
    let m = k.measure_angle(l[0], l[2]);
    k.check_value(m, rad(60.0));
}

/// Pentagon walk with turning angles a, b, c, d (vars) and unit sides; the
/// distance from start to end.
#[test]
fn walk_symbolic_turns_distance() {
    let mut k = Kit::new();
    let o = k.origin();
    let turns = [50.0f64, 70.0, 40.0];
    let names = ["a", "b", "c"];
    for (n, t) in names.iter().zip(turns) {
        k.var(n, t);
    }
    let mut pts = vec![o];
    let mut pos = [0.0, 0.0];
    let mut h = 0.0;
    for i in 0..4 {
        if i > 0 {
            h += turns[i - 1] + 4.0;
        }
        pos = [pos[0] + rad(h).cos(), pos[1] + rad(h).sin()];
        pts.push(k.point(pos[0], pos[1]));
    }
    let l: Vec<usize> = (0..4).map(|i| k.line(pts[i], pts[i + 1])).collect();
    k.horizontal(l[0]);
    k.var("s", 1.0);
    for i in 0..4 {
        k.drive(DimKind::Length(l[i]), "s");
    }
    for i in 0..3 {
        k.drive_angle(l[i], l[i + 1], &format!("{}*deg", names[i]));
    }
    let (mut x, mut y, mut hh) = (0.0, 0.0, 0.0f64);
    for i in 0..4 {
        if i > 0 {
            hh += turns[i - 1];
        }
        x += rad(hh).cos();
        y += rad(hh).sin();
    }
    let d = k.measure(DimKind::Distance(o, pts[4]));
    k.check_value(d, x.hypot(y));
}

/// Arc of radius r with chord c: sector angle 2 asin(c/(2r)) between the radii.
#[test]
fn arc_sector_angle_from_chord() {
    let mut k = Kit::new();
    let o = k.origin();
    k.var("r", 2.0);
    k.var("c", 3.0);
    let ang = 2.0 * (0.75f64).asin();
    let a = k.point(2.0, 0.1);
    let b = k.point(2.0 * (ang * 1.05).cos(), 2.0 * (ang * 1.05).sin());
    let arc = k.arc(o, a, b);
    k.drive(DimKind::Radius(arc), "r");
    let oa = k.line(o, a);
    let ob = k.line(o, b);
    k.horizontal(oa);
    k.drive(DimKind::Distance(a, b), "c");
    let m = k.measure_angle(oa, ob);
    k.check_value(m, ang);
}

/// Two solved angles t and 2t: Q at angle t (unit), R at angle 2t (unit),
/// with |QR| = d driven: chord QR = 2 sin(t/2) = d → y_Q = sin(2 asin(d/2)).
#[test]
fn solved_angles_t_and_2t_chord() {
    let mut k = Kit::new();
    let o = k.origin();
    let xa = k.xaxis();
    k.var("d", 0.8);
    k.solved_var("t", 0.9);
    let t = 2.0 * (0.4f64).asin();
    let q = k.point(t.cos() * 1.05, t.sin());
    let r = k.point((2.0 * t).cos(), (2.0 * t).sin() * 1.05);
    let oq = k.line(o, q);
    let or = k.line(o, r);
    k.drive(DimKind::Length(oq), "1");
    k.drive(DimKind::Length(or), "1");
    k.drive_angle(xa, oq, "t");
    k.drive_angle(xa, or, "2*t");
    k.drive(DimKind::Distance(q, r), "d");
    assert_eq!(k.dof(), 0);
    let y = k.measure(DimKind::CoordY(q));
    k.check_value(y, t.sin());
}

/// Angle driven as θ1 + θ2 of other dimensions where the sum exceeds π:
/// angles 120 and 100 driven, a third driven "2π − θ1 − θ2" closes the fan.
#[test]
fn angle_sum_expression_over_pi() {
    let mut k = Kit::new();
    let o = k.origin();
    let p1 = k.point(3.0, 0.2);
    let p2 = k.point(3.0 * rad(122.0).cos(), 3.0 * rad(122.0).sin());
    let p3 = k.point(3.0 * rad(225.0).cos(), 3.0 * rad(225.0).sin());
    let l1 = k.line(o, p1);
    let l2 = k.line(o, p2);
    let l3 = k.line(o, p3);
    k.horizontal(l1);
    k.var("u", 120.0);
    k.var("v", 100.0);
    k.drive_angle(l1, l2, "u*deg");
    k.drive_angle(l2, l3, "v*deg");
    k.drive(DimKind::Length(l3), "2");
    let y = k.measure(DimKind::CoordY(p3));
    k.check_value(y, 2.0 * rad(220.0).sin());
}

/// Angle driven by a degree-postfix expression of a var: "(a + 10)°".
#[test]
fn degree_postfix_expression() {
    let mut k = Kit::new();
    let o = k.origin();
    let xa = k.xaxis();
    k.var("a", 50.0);
    k.var("L", 2.0);
    let p = k.point(1.0, 2.0);
    let op = k.line(o, p);
    k.drive(DimKind::Length(op), "L");
    k.drive_angle(xa, op, "(a + 10)°");
    let y = k.measure(DimKind::CoordY(p));
    k.check_value(y, 2.0 * rad(60.0).sin());
}

/// Half of an obtuse angle near π: P = (a, 0.3) with a = −3, angle 2t.
#[test]
fn double_angle_solved_var_near_pi() {
    let mut k = Kit::new();
    let o = k.origin();
    let xa = k.xaxis();
    k.var("a", -3.0);
    k.solved_var("t", 1.5);
    let a: f64 = (0.3f64).atan2(-3.0) / 2.0;
    let q = k.point(a.cos() * 1.1, a.sin());
    let oq = k.line(o, q);
    k.drive(DimKind::Length(oq), "1");
    k.drive_angle(xa, oq, "t");
    let p = k.point(-3.1, 0.3);
    let op = k.line(o, p);
    k.drive_angle(xa, op, "2*t");
    k.drive(DimKind::CoordX(p), "a");
    k.drive(DimKind::CoordY(p), "0.3");
    assert_eq!(k.dof(), 0);
    let x = k.measure(DimKind::CoordX(q));
    k.check_value(x, a.cos());
}

/// Walk O→A (p, horizontal) →B (q, turn α) →C (r) with C back on the x axis:
/// the second turning angle β satisfies q sin α + r sin(α + β) = 0.
#[test]
fn walk_closing_turn_angle() {
    let (p, q, r, al) = (3.0f64, 2.0f64, 2.5f64, rad(70.0));
    let b = [p + q * al.cos(), q * al.sin()];
    // C = B + r (cos h, sin h) with sin h = −q sin α / r, heading h in (180, 270)?
    let h = PI + (q * al.sin() / r).asin();
    let c = [b[0] + r * h.cos(), b[1] + r * h.sin()];
    let mut k = Kit::new();
    let o = k.origin();
    let xa = k.xaxis();
    k.var("p", p);
    k.var("q", q);
    k.var("r", r);
    k.var("al", 70.0);
    let pa = k.point(p, 0.1);
    let pb = k.point(b[0] + 0.05, b[1]);
    let pc = k.point(c[0] + 0.05, 0.05);
    let oa = k.line(o, pa);
    let ab = k.line(pa, pb);
    let bc = k.line(pb, pc);
    k.horizontal(oa);
    k.on(pc, xa);
    k.drive(DimKind::Length(oa), "p");
    k.drive(DimKind::Length(ab), "q");
    k.drive(DimKind::Length(bc), "r");
    k.drive_angle(oa, ab, "al*deg");
    assert_eq!(k.dof(), 0);
    let m = k.measure_angle(ab, bc);
    let ex = h - al; // turning angle between directions
    let ex = if ex > PI { 2.0 * PI - ex } else { ex };
    k.check_value(m, ex);
}
