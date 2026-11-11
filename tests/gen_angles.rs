//! Angle-centred constructions: driving angles of all sizes, polar walks,
//! sine/cosine rule setups, circle theorems, polygons and mechanisms.

use gxpad::model::DimKind;
use gxpad::testkit::Kit;
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
    let i = k.app.geo.add_dimension(DimKind::Angle(l1, l2), lw).expect("dimension rejected");
    assert!(k.doc().dims[i].as_ref().unwrap().driving, "angle already determined");
    k.set(i, val);
    i
}

fn ameas(k: &mut Kit, l1: usize, l2: usize) -> usize {
    let lw = label(k, l1, l2);
    let i = k.app.geo.add_dimension(DimKind::Angle(l1, l2), lw).expect("dimension rejected");
    if k.doc().dims[i].as_ref().unwrap().driving {
        k.app.toggle_driving(i);
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
