//! Classical ruler-and-compass construction chains built with the Kit tools.

use gxpad::model::DimKind;
use gxpad::testkit::Kit;

type P = [f64; 2];

fn dist(p: P, q: P) -> f64 {
    (p[0] - q[0]).hypot(p[1] - q[1])
}
fn mid(p: P, q: P) -> P {
    [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0]
}
fn proj(p: P, a: P, b: P) -> P {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy);
    [a[0] + t * dx, a[1] + t * dy]
}
fn refl(p: P, a: P, b: P) -> P {
    let f = proj(p, a, b);
    [2.0 * f[0] - p[0], 2.0 * f[1] - p[1]]
}
/// Intersection of line (p1, p1 + d1) with line (p2, p2 + d2).
fn xline(p1: P, d1: P, p2: P, d2: P) -> P {
    let den = d1[0] * d2[1] - d1[1] * d2[0];
    let t = ((p2[0] - p1[0]) * d2[1] - (p2[1] - p1[1]) * d2[0]) / den;
    [p1[0] + t * d1[0], p1[1] + t * d1[1]]
}
fn sub(p: P, q: P) -> P {
    [p[0] - q[0], p[1] - q[1]]
}
fn area(a: P, b: P, c: P) -> f64 {
    ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() / 2.0
}
fn s(v: f64) -> String {
    format!("{v}")
}

struct Tri {
    a: usize,
    b: usize,
    c: usize,
    ab: usize,
    bc: usize,
    ca: usize,
}

/// A floating triangle with its three side lengths driven (3 DOF left).
fn tri(k: &mut Kit, pa: P, pb: P, pc: P) -> Tri {
    let a = k.point(pa[0], pa[1]);
    let b = k.point(pb[0], pb[1]);
    let c = k.point(pc[0], pc[1]);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let ca = k.line(c, a);
    k.drive(DimKind::Length(ab), &s(dist(pa, pb)));
    k.drive(DimKind::Length(bc), &s(dist(pb, pc)));
    k.drive(DimKind::Length(ca), &s(dist(pc, pa)));
    Tri { a, b, c, ab, bc, ca }
}

fn foot(k: &mut Kit, l: usize, p: usize, at: P) -> usize {
    let pl = k.perp_line(l, p);
    k.intersect(pl, l, at)
}

const TA: P = [0.0, 0.0];
const TB: P = [6.0, 0.5];
const TC: P = [2.0, 4.5];

/// Varignon parallelogram of a floating quadrilateral: sides are half the diagonals.
#[test]
fn varignon_parallelogram() {
    let mut k = Kit::new();
    let (pa, pb, pc, pd) = ([0.0, 0.0], [5.0, 0.5], [6.0, 4.0], [1.0, 3.5]);
    let a = k.point(pa[0], pa[1]);
    let b = k.point(pb[0], pb[1]);
    let c = k.point(pc[0], pc[1]);
    let d = k.point(pd[0], pd[1]);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let cd = k.line(c, d);
    let da = k.line(d, a);
    let ac = k.line(a, c);
    for (l, x, y) in [(ab, pa, pb), (bc, pb, pc), (cd, pc, pd), (da, pd, pa), (ac, pa, pc)] {
        k.drive(DimKind::Length(l), &s(dist(x, y)));
    }
    assert!(k.dof() > 0);
    let p = k.midpoint_of_line(ab);
    let q = k.midpoint_of_line(bc);
    let r = k.midpoint_of_line(cd);
    let t = k.midpoint_of_line(da);
    let pq = k.measure(DimKind::Distance(p, q));
    let rt = k.measure(DimKind::Distance(r, t));
    k.check_value(pq, dist(pa, pc) / 2.0);
    k.check_value(rt, dist(pa, pc) / 2.0);
    let _ = (q, t);
}

/// Varignon: the sides along the undriven diagonal BD are also half of it.
#[test]
fn varignon_other_diagonal() {
    let mut k = Kit::new();
    let (pa, pb, pc, pd) = ([0.0, 0.0], [5.0, 0.5], [6.0, 4.0], [1.0, 3.5]);
    let a = k.point(pa[0], pa[1]);
    let b = k.point(pb[0], pb[1]);
    let c = k.point(pc[0], pc[1]);
    let d = k.point(pd[0], pd[1]);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let cd = k.line(c, d);
    let da = k.line(d, a);
    let ac = k.line(a, c);
    for (l, x, y) in [(ab, pa, pb), (bc, pb, pc), (cd, pc, pd), (da, pd, pa), (ac, pa, pc)] {
        k.drive(DimKind::Length(l), &s(dist(x, y)));
    }
    assert!(k.dof() > 0);
    let q = k.midpoint_of_line(bc);
    let r = k.midpoint_of_line(cd);
    let qr = k.measure(DimKind::Distance(q, r));
    k.check_value(qr, dist(pb, pd) / 2.0);
}

/// Midpoint coordinates of a fully constrained quadrilateral.
#[test]
fn varignon_coordinates() {
    let mut k = Kit::new();
    let o = k.origin();
    let b = k.point(4.0, 0.3);
    let c = k.point(5.0, 3.0);
    let d = k.point(0.5, 2.5);
    k.drive(DimKind::CoordX(b), "4");
    k.drive(DimKind::CoordY(b), "0");
    k.drive(DimKind::CoordX(c), "5");
    k.drive(DimKind::CoordY(c), "3");
    k.drive(DimKind::CoordX(d), "0");
    k.drive(DimKind::CoordY(d), "2");
    let l1 = k.line(o, b);
    let l2 = k.line(b, c);
    let l3 = k.line(c, d);
    let l4 = k.line(d, o);
    let p = k.midpoint_of_line(l1);
    let q = k.midpoint_of_line(l2);
    let r = k.midpoint_of_line(l3);
    let t = k.midpoint_of_line(l4);
    assert_eq!(k.dof(), 0);
    // Varignon parallelogram: its centre is the mean of the four vertices
    let m1 = k.midpoint_of(p, r);
    let m2 = k.midpoint_of(q, t);
    let x1 = k.measure(DimKind::CoordX(m1));
    let y1 = k.measure(DimKind::CoordY(m1));
    let x2 = k.measure(DimKind::CoordX(m2));
    let y2 = k.measure(DimKind::CoordY(m2));
    k.check_value(x1, 9.0 / 4.0);
    k.check_value(y1, 5.0 / 4.0);
    k.check_value(x2, 9.0 / 4.0);
    k.check_value(y2, 5.0 / 4.0);
    let px = k.measure(DimKind::CoordX(q));
    k.check_value(px, 4.5);
}

/// Two reflections in lines through a common point rotate by twice the angle.
#[test]
fn two_reflections_rotation_distance() {
    let mut k = Kit::new();
    let o = k.origin();
    let u = k.point(1.0, 1.7);
    let v = k.point(0.2, 1.1);
    let pp = k.point(3.0, 0.2);
    let l1 = k.inf_line(o, u);
    let l2 = k.inf_line(o, v);
    let op = k.line(o, pp);
    let xa = k.xaxis();
    k.drive(DimKind::Angle(xa, l1), "60");
    k.drive(DimKind::Angle(xa, l2), "80");
    k.drive(DimKind::Length(op), "3");
    assert!(k.dof() > 0);
    let p1 = k.reflect(pp, l1);
    let p2 = k.reflect(p1, l2);
    // rotation by 2 * 20 degrees: chord 2 r sin(20 deg)
    let d = k.measure(DimKind::Distance(pp, p2));
    k.check_value(d, 2.0 * 3.0 * (20f64.to_radians()).sin());
    let d1 = k.measure(DimKind::Distance(o, p2));
    k.check_value(d1, 3.0);
}

/// Same with fixed point: coordinates of the rotated image.
#[test]
fn two_reflections_rotation_coordinates() {
    let mut k = Kit::new();
    let o = k.origin();
    let u = k.point(1.0, 1.7);
    let v = k.point(0.2, 1.1);
    let pp = k.point(3.0, 0.0);
    k.drive(DimKind::CoordX(pp), "3");
    k.drive(DimKind::CoordY(pp), "0");
    let l1 = k.inf_line(o, u);
    let l2 = k.inf_line(o, v);
    let xa = k.xaxis();
    k.drive(DimKind::Angle(xa, l1), "60");
    k.drive(DimKind::Angle(xa, l2), "80");
    assert!(k.dof() > 0);
    let p1 = k.reflect(pp, l1);
    let p2 = k.reflect(p1, l2);
    let x = k.measure(DimKind::CoordX(p2));
    let y = k.measure(DimKind::CoordY(p2));
    let th = 40f64.to_radians();
    k.check_value(x, 3.0 * th.cos());
    k.check_value(y, 3.0 * th.sin());
    let x1 = k.measure(DimKind::CoordX(p1));
    k.check_value(x1, 3.0 * 120f64.to_radians().cos());
}

/// Two reflections in perpendicular lines is a point reflection.
#[test]
fn reflections_perpendicular_point_reflection() {
    let mut k = Kit::new();
    let c = k.point(0.5, 0.5);
    let u = k.point(3.0, 0.7);
    let v = k.point(0.2, 3.0);
    let a = k.point(2.0, 2.5);
    let l1 = k.inf_line(c, u);
    let l2 = k.inf_line(c, v);
    k.perpendicular(l1, l2);
    let ca = k.line(c, a);
    k.drive(DimKind::Length(ca), "3");
    assert!(k.dof() > 0);
    let a1 = k.reflect(a, l1);
    let a2 = k.reflect(a1, l2);
    let d = k.measure(DimKind::Distance(a, a2));
    k.check_value(d, 6.0);
    let d2 = k.measure(DimKind::Distance(a, a1));
    // |AA1| is twice the distance of A from l1, not determined: only the square sum is
    let _ = d2;
    let m = k.midpoint_of(a, a2);
    let dm = k.measure(DimKind::Distance(m, c));
    k.check_value(dm, 0.0);
}

/// Two reflections in parallel lines translate by twice their distance.
#[test]
fn parallel_reflections_translation() {
    let mut k = Kit::new();
    let u = k.point(0.0, 0.0);
    let v = k.point(4.0, 1.0);
    let w = k.point(1.0, 2.0);
    let a = k.point(2.0, 4.0);
    let l1 = k.inf_line(u, v);
    let l2 = k.par_line(l1, w);
    k.drive(DimKind::PointLine(w, l1), "1.2");
    let a1 = k.reflect(a, l1);
    let a2 = k.reflect(a1, l2);
    assert!(k.dof() > 0);
    let d = k.measure(DimKind::Distance(a, a2));
    k.check_value(d, 2.4);
}

/// Orthocenter via altitudes; its distance to a vertex is 2 R cos A.
#[test]
fn orthocenter_distance() {
    let mut k = Kit::new();
    let t = tri(&mut k, TA, TB, TC);
    assert_eq!(k.dof(), 3);
    let fa = foot(&mut k, t.bc, t.a, proj(TA, TB, TC));
    let fb = foot(&mut k, t.ca, t.b, proj(TB, TC, TA));
    let alt_a = k.perp_line(t.bc, t.a);
    let alt_b = k.perp_line(t.ca, t.b);
    let h_pt = {
        let x = [TA[0], TA[1]];
        let _ = x;
        // orthocenter from coordinates
        let da = [-(TC[1] - TB[1]), TC[0] - TB[0]];
        let db = [-(TA[1] - TC[1]), TA[0] - TC[0]];
        xline(TA, da, TB, db)
    };
    let h = k.intersect(alt_a, alt_b, h_pt);
    let r = {
        let (a, b, c) = (dist(TB, TC), dist(TC, TA), dist(TA, TB));
        a * b * c / (4.0 * area(TA, TB, TC))
    };
    let cos_a = {
        let (b, c, a) = (dist(TC, TA), dist(TA, TB), dist(TB, TC));
        (b * b + c * c - a * a) / (2.0 * b * c)
    };
    let ha = k.measure(DimKind::Distance(h, t.a));
    k.check_value(ha, 2.0 * r * cos_a.abs());
    let hf = k.measure(DimKind::Distance(fa, fb));
    let _ = hf;
    k.check(hf);
    let hy = k.measure(DimKind::Distance(h, fa));
    k.check_value(hy, dist(h_pt, proj(TA, TB, TC)).min(dist(h_pt, fa_val())));
}
fn fa_val() -> P {
    proj(TA, TB, TC)
}

/// Circumradius from perpendicular bisectors.
#[test]
fn circumradius_from_bisectors() {
    let mut k = Kit::new();
    let t = tri(&mut k, TA, TB, TC);
    let (l1, m1) = k.perp_bisector(t.a, t.b);
    let (l2, _m2) = k.perp_bisector(t.b, t.c);
    let (l3, _m3) = k.perp_bisector(t.c, t.a);
    let cc = {
        let d1 = [-(TB[1] - TA[1]), TB[0] - TA[0]];
        let d2 = [-(TC[1] - TB[1]), TC[0] - TB[0]];
        xline(mid(TA, TB), d1, mid(TB, TC), d2)
    };
    let o = k.intersect(l1, l2, cc);
    assert_eq!(k.dof(), 3);
    let r = dist(TB, TC) * dist(TC, TA) * dist(TA, TB) / (4.0 * area(TA, TB, TC));
    let ra = k.measure(DimKind::Distance(o, t.a));
    let rb = k.measure(DimKind::Distance(o, t.b));
    let rc = k.measure(DimKind::Distance(o, t.c));
    k.check_value(ra, r);
    k.check_value(rb, r);
    k.check_value(rc, r);
    let hm = k.measure(DimKind::Distance(m1, t.a));
    k.check_value(hm, dist(TA, TB) / 2.0);
    let _ = l3;
}

/// The third perpendicular bisector passes through the circumcenter.
#[test]
fn concurrent_bisectors() {
    let mut k = Kit::new();
    let t = tri(&mut k, TA, TB, TC);
    let (l1, _) = k.perp_bisector(t.a, t.b);
    let (l2, _) = k.perp_bisector(t.b, t.c);
    let (l3, _) = k.perp_bisector(t.c, t.a);
    let cc = xline(
        mid(TA, TB),
        [-(TB[1] - TA[1]), TB[0] - TA[0]],
        mid(TB, TC),
        [-(TC[1] - TB[1]), TC[0] - TB[0]],
    );
    let o = k.intersect(l1, l2, cc);
    let o3 = k.intersect(l3, l1, cc);
    assert_eq!(k.dof(), 3);
    let d = k.measure(DimKind::Distance(o, o3));
    k.check_value(d, 0.0);
}

/// Euler's nine-point circle: N = midpoint(O, H) is R/2 from midpoints of the sides.
#[test]
fn nine_point_center() {
    let mut k = Kit::new();
    let t = tri(&mut k, TA, TB, TC);
    let (l1, _) = k.perp_bisector(t.a, t.b);
    let (l2, _) = k.perp_bisector(t.b, t.c);
    let cc = xline(
        mid(TA, TB),
        [-(TB[1] - TA[1]), TB[0] - TA[0]],
        mid(TB, TC),
        [-(TC[1] - TB[1]), TC[0] - TB[0]],
    );
    let o = k.intersect(l1, l2, cc);
    let alt_a = k.perp_line(t.bc, t.a);
    let alt_b = k.perp_line(t.ca, t.b);
    let hp = [TA[0] + TB[0] + TC[0] - 2.0 * cc[0], TA[1] + TB[1] + TC[1] - 2.0 * cc[1]];
    let h = k.intersect(alt_a, alt_b, hp);
    let n = k.midpoint_of(o, h);
    let m = k.midpoint_of(t.b, t.c);
    let r = dist(TB, TC) * dist(TC, TA) * dist(TA, TB) / (4.0 * area(TA, TB, TC));
    assert_eq!(k.dof(), 3);
    let d = k.measure(DimKind::Distance(n, m));
    k.check_value(d, r / 2.0);
    // Euler: |OH|^2 = 9R^2 - (a^2+b^2+c^2)
    let oh = k.measure(DimKind::Distance(o, h));
    let sq = dist(TB, TC).powi(2) + dist(TC, TA).powi(2) + dist(TA, TB).powi(2);
    k.check_value(oh, (9.0 * r * r - sq).sqrt());
}

/// The reflection of the orthocenter in a side lies on the circumcircle.
#[test]
fn orthocenter_reflection_on_circumcircle() {
    let mut k = Kit::new();
    let t = tri(&mut k, TA, TB, TC);
    let (l1, _) = k.perp_bisector(t.a, t.b);
    let (l2, _) = k.perp_bisector(t.b, t.c);
    let cc = xline(
        mid(TA, TB),
        [-(TB[1] - TA[1]), TB[0] - TA[0]],
        mid(TB, TC),
        [-(TC[1] - TB[1]), TC[0] - TB[0]],
    );
    let o = k.intersect(l1, l2, cc);
    let alt_a = k.perp_line(t.bc, t.a);
    let alt_b = k.perp_line(t.ca, t.b);
    let hp = [TA[0] + TB[0] + TC[0] - 2.0 * cc[0], TA[1] + TB[1] + TC[1] - 2.0 * cc[1]];
    let h = k.intersect(alt_a, alt_b, hp);
    let h2 = k.reflect(h, t.bc);
    let r = dist(TB, TC) * dist(TC, TA) * dist(TA, TB) / (4.0 * area(TA, TB, TC));
    let d = k.measure(DimKind::Distance(o, h2));
    k.check_value(d, r);
    let h3 = k.reflect(h, t.ca);
    let d3 = k.measure(DimKind::Distance(o, h3));
    k.check_value(d3, r);
    assert!(k.dof() > 0);
}

/// Incenter from two angle bisectors: inradius = area / semiperimeter.
#[test]
fn incenter_inradius() {
    let mut k = Kit::new();
    let t = tri(&mut k, TA, TB, TC);
    let g = [(TA[0] + TB[0] + TC[0]) / 3.0, (TA[1] + TB[1] + TC[1]) / 3.0];
    let (ba, _) = k.angle_bisector(t.ab, t.ca, g);
    let (bb, _) = k.angle_bisector(t.ab, t.bc, g);
    let (a, b, c) = (dist(TB, TC), dist(TC, TA), dist(TA, TB));
    let ic = [(a * TA[0] + b * TB[0] + c * TC[0]) / (a + b + c), (a * TA[1] + b * TB[1] + c * TC[1]) / (a + b + c)];
    let i = k.intersect(ba, bb, ic);
    assert!(k.dof() > 0);
    let sp = (a + b + c) / 2.0;
    let rin = area(TA, TB, TC) / sp;
    let r1 = k.measure(DimKind::PointLine(i, t.ab));
    let r2 = k.measure(DimKind::PointLine(i, t.bc));
    let r3 = k.measure(DimKind::PointLine(i, t.ca));
    k.check_value(r1, rin);
    k.check_value(r2, rin);
    k.check_value(r3, rin);
    // |AI| = (s - a) / cos(A/2)
    let ai = k.measure(DimKind::Distance(i, t.a));
    k.check_value(ai, dist(TA, ic));
}

/// Reflecting B in the bisector at A lands on AC: |B'C| = |b - c|.
#[test]
fn bisector_reflection_on_side() {
    let mut k = Kit::new();
    let t = tri(&mut k, TA, TB, TC);
    let g = [(TA[0] + TB[0] + TC[0]) / 3.0, (TA[1] + TB[1] + TC[1]) / 3.0];
    let (bis, _) = k.angle_bisector(t.ab, t.ca, g);
    let b2 = k.reflect(t.b, bis);
    assert!(k.dof() > 0);
    let d = k.measure(DimKind::Distance(b2, t.c));
    k.check_value(d, (dist(TC, TA) - dist(TA, TB)).abs());
    let ab2 = k.measure(DimKind::Distance(t.a, b2));
    k.check_value(ab2, dist(TA, TB));
    let on = k.measure(DimKind::PointLine(b2, t.ca));
    k.check_value(on, 0.0);
}

/// Medians meet at the centroid; |AG| is 2/3 of the median.
#[test]
fn centroid_by_medians() {
    let mut k = Kit::new();
    let t = tri(&mut k, TA, TB, TC);
    let ma = k.midpoint_of_line(t.bc);
    let mb = k.midpoint_of_line(t.ca);
    let la = k.inf_line(t.a, ma);
    let lb = k.inf_line(t.b, mb);
    let g0 = [(TA[0] + TB[0] + TC[0]) / 3.0, (TA[1] + TB[1] + TC[1]) / 3.0];
    let g = k.intersect(la, lb, g0);
    assert_eq!(k.dof(), 3);
    let ag = k.measure(DimKind::Distance(t.a, g));
    k.check_value(ag, dist(TA, g0));
    let gm = k.measure(DimKind::Distance(g, ma));
    k.check_value(gm, dist(g0, mid(TB, TC)));
    // apollonius: median length
    let (a, b, c) = (dist(TB, TC), dist(TC, TA), dist(TA, TB));
    let med = (2.0 * b * b + 2.0 * c * c - a * a).sqrt() / 2.0;
    let am = k.measure(DimKind::Distance(t.a, ma));
    k.check_value(am, med);
}

/// Line through the midpoint of AB parallel to BC meets AC at its midpoint.
#[test]
fn midline_theorem() {
    let mut k = Kit::new();
    let t = tri(&mut k, TA, TB, TC);
    let m = k.midpoint_of_line(t.ab);
    let pl = k.par_line(t.bc, m);
    let n = k.intersect(pl, t.ca, mid(TA, TC));
    assert_eq!(k.dof(), 3);
    let mn = k.measure(DimKind::Distance(m, n));
    k.check_value(mn, dist(TB, TC) / 2.0);
    let an = k.measure(DimKind::Distance(t.a, n));
    k.check_value(an, dist(TA, TC) / 2.0);
}

/// Parallels through the vertices: the fourth vertex of a parallelogram.
#[test]
fn parallelogram_completion() {
    let mut k = Kit::new();
    let t = tri(&mut k, TA, TB, TC);
    let l1 = k.par_line(t.bc, t.a);
    let l2 = k.par_line(t.ca, t.b);
    let dp = [TA[0] + TB[0] - TC[0], TA[1] + TB[1] - TC[1]];
    let d = k.intersect(l1, l2, dp);
    let cd = k.measure(DimKind::Distance(t.c, d));
    let (a, b, c) = (dist(TB, TC), dist(TC, TA), dist(TA, TB));
    let med = (2.0 * a * a + 2.0 * b * b - c * c).sqrt() / 2.0;
    k.check_value(cd, 2.0 * med);
    // anticomplementary triangle: third parallel too
    let l3 = k.par_line(t.ab, t.c);
    let e = k.intersect(l3, l1, [TA[0] + TC[0] - TB[0], TA[1] + TC[1] - TB[1]]);
    let f = k.intersect(l3, l2, [TB[0] + TC[0] - TA[0], TB[1] + TC[1] - TA[1]]);
    assert!(k.dof() > 0);
    let ef = k.measure(DimKind::Distance(e, f));
    k.check_value(ef, 2.0 * dist(TA, TB));
}

/// Iterated medial triangles shrink by a factor 2 each time.
#[test]
fn nested_medial_triangles() {
    let mut k = Kit::new();
    let t = tri(&mut k, TA, TB, TC);
    let mut pa = t.a;
    let mut pb = t.b;
    let mut pc = t.c;
    let (mut qa, mut qb, mut qc) = (TA, TB, TC);
    for _ in 0..3 {
        let l1 = k.line(pa, pb);
        let l2 = k.line(pb, pc);
        let l3 = k.line(pc, pa);
        let m1 = k.midpoint_of_line(l1);
        let m2 = k.midpoint_of_line(l2);
        let m3 = k.midpoint_of_line(l3);
        (pa, pb, pc) = (m1, m2, m3);
        (qa, qb, qc) = (mid(qa, qb), mid(qb, qc), mid(qc, qa));
    }
    assert_eq!(k.dof(), 3);
    let d = k.measure(DimKind::Distance(pa, pb));
    k.check_value(d, dist(TA, TB) / 8.0);
    let d2 = k.measure(DimKind::Distance(pb, pc));
    k.check_value(d2, dist(TB, TC) / 8.0);
    let _ = qc;
    let _ = qb;
}

/// Reflecting C in the perpendicular bisector of AB gives an isosceles trapezoid.
#[test]
fn isosceles_trapezoid_by_reflection() {
    let mut k = Kit::new();
    let t = tri(&mut k, TA, TB, TC);
    let (pb, _) = k.perp_bisector(t.a, t.b);
    let c2 = k.reflect(t.c, pb);
    assert!(k.dof() > 0);
    let (a, b, c) = (dist(TB, TC), dist(TC, TA), dist(TA, TB));
    let d1 = k.measure(DimKind::Distance(c2, t.a));
    k.check_value(d1, a);
    let d2 = k.measure(DimKind::Distance(c2, t.b));
    k.check_value(d2, b);
    let cc = k.measure(DimKind::Distance(c2, t.c));
    k.check_value(cc, ((b * b - a * a) / c).abs());
    let cl = k.line(t.c, c2);
    let l = k.measure(DimKind::Length(cl));
    k.check(l);
}

/// Feet of perpendiculars from a point P on the triangle sides (pedal triangle).
#[test]
fn pedal_triangle_sides() {
    let mut k = Kit::new();
    let t = tri(&mut k, TA, TB, TC);
    let pp = [2.5, 2.0];
    let p = k.point(pp[0], pp[1]);
    k.drive(DimKind::Distance(p, t.a), &s(dist(pp, TA)));
    k.drive(DimKind::Distance(p, t.b), &s(dist(pp, TB)));
    assert_eq!(k.dof(), 3);
    let d = foot(&mut k, t.bc, p, proj(pp, TB, TC));
    let e = foot(&mut k, t.ca, p, proj(pp, TC, TA));
    let f = foot(&mut k, t.ab, p, proj(pp, TA, TB));
    let (fd, fe, ff) = (proj(pp, TB, TC), proj(pp, TC, TA), proj(pp, TA, TB));
    let de = k.measure(DimKind::Distance(d, e));
    let ef = k.measure(DimKind::Distance(e, f));
    let fd_ = k.measure(DimKind::Distance(f, d));
    k.check_value(de, dist(fd, fe));
    k.check_value(ef, dist(fe, ff));
    k.check_value(fd_, dist(ff, fd));
    // |DE| = |PC| sin C
    let sinc = 2.0 * area(TA, TB, TC) / (dist(TB, TC) * dist(TC, TA));
    k.check_value(de, dist(pp, TC) * sinc);
}

/// Circumcircle with a point P on it, and the feet of perpendiculars from P.
fn simson_setup(k: &mut Kit) -> (usize, usize, usize, [P; 3]) {
    let (pa, pb, pc) = ([0.0, 0.0], [6.0, 0.0], [2.0, 4.0]);
    let t = tri(k, pa, pb, pc);
    let (circ, _oc) = k.circle_through(t.a, t.b, t.c);
    let th = 100f64.to_radians();
    let r = 10f64.sqrt();
    let pp = [3.0 + r * th.cos(), 1.0 + r * th.sin()];
    let p = k.point(pp[0] + 0.05, pp[1] + 0.05);
    k.on(p, circ);
    k.drive(DimKind::Distance(p, t.a), &s(dist(pp, pa)));
    assert_eq!(k.dof(), 3);
    let d = foot(k, t.bc, p, proj(pp, pb, pc));
    let e = foot(k, t.ca, p, proj(pp, pc, pa));
    let f = foot(k, t.ab, p, proj(pp, pa, pb));
    (d, e, f, [proj(pp, pb, pc), proj(pp, pc, pa), proj(pp, pa, pb)])
}

/// Simson line: feet from a circumcircle point are collinear.
#[test]
fn simson_line() {
    let mut k = Kit::new();
    let (d, e, f, _) = simson_setup(&mut k);
    let de = k.inf_line(d, e);
    let off = k.measure(DimKind::PointLine(f, de));
    // the offset is 0 for every triangle: the formula must say so
    let text = k.check_value(off, 0.0);
    assert!(text.ends_with(" = 0"), "{text}");
}

/// Distances between the Simson feet.
#[test]
fn simson_feet_distance() {
    let mut k = Kit::new();
    let (d, e, f, ft) = simson_setup(&mut k);
    let dd = k.measure(DimKind::Distance(d, e));
    k.check_value(dd, dist(ft[0], ft[1]));
    let ee = k.measure(DimKind::Distance(e, f));
    k.check_value(ee, dist(ft[1], ft[2]));
    // collinear: |DF| = |DE| + |EF| (E is between the others for this P)
    let df = k.measure(DimKind::Distance(d, f));
    k.check_value(df, dist(ft[0], ft[2]));
}

/// Napoleon's theorem: centers of outward equilateral triangles form an equilateral one.
#[test]
fn napoleon_triangle() {
    let mut k = Kit::new();
    let (pa, pb, pc) = ([0.0, 0.0], [5.0, 0.0], [1.5, 3.5]);
    let t = tri(&mut k, pa, pb, pc);
    let outward = |p: P, q: P| -> (P, P) {
        let m = mid(p, q);
        let n = [q[1] - p[1], -(q[0] - p[0])];
        let ap = [m[0] + 3f64.sqrt() / 2.0 * n[0], m[1] + 3f64.sqrt() / 2.0 * n[1]];
        let g = [(p[0] + q[0] + ap[0]) / 3.0, (p[1] + q[1] + ap[1]) / 3.0];
        (ap, g)
    };
    let mut centers = vec![];
    let mut cs = vec![];
    for (u, v, pu, pv, l) in [(t.a, t.b, pa, pb, t.ab), (t.b, t.c, pb, pc, t.bc), (t.c, t.a, pc, pa, t.ca)] {
        let (ap, g) = outward(pu, pv);
        let c1 = k.circle(u, 4.0);
        let c2 = k.circle(v, 4.0);
        k.equal(c1, l);
        k.equal(c2, l);
        let apex = k.intersect(c1, c2, ap);
        let (b1, _) = k.perp_bisector(u, v);
        let (b2, _) = k.perp_bisector(u, apex);
        let cen = k.intersect(b1, b2, g);
        centers.push(cen);
        cs.push(g);
    }
    assert_eq!(k.dof(), 3);
    for i in 0..3 {
        let j = (i + 1) % 3;
        let d = k.measure(DimKind::Distance(centers[i], centers[j]));
        k.check_value(d, dist(cs[i], cs[j]));
    }
    let (a, b, c) = (dist(pb, pc), dist(pc, pa), dist(pa, pb));
    let ar = area(pa, pb, pc);
    let side2 = (a * a + b * b + c * c) / 6.0 + 2.0 * ar / 3f64.sqrt();
    let d = k.measure(DimKind::Distance(centers[0], centers[2]));
    k.check_value(d, side2.sqrt());
}

/// Fermat point: lines from vertices to the far apexes of outward equilateral triangles.
#[test]
fn fermat_point() {
    let mut k = Kit::new();
    let (pa, pb, pc) = ([0.0, 0.0], [5.0, 0.0], [1.5, 3.5]);
    let t = tri(&mut k, pa, pb, pc);
    let apex = |p: P, q: P| -> P {
        let m = mid(p, q);
        let n = [q[1] - p[1], -(q[0] - p[0])];
        [m[0] + 3f64.sqrt() / 2.0 * n[0], m[1] + 3f64.sqrt() / 2.0 * n[1]]
    };
    let mk = |k: &mut Kit, u: usize, v: usize, l: usize, g: P| {
        let c1 = k.circle(u, 4.0);
        let c2 = k.circle(v, 4.0);
        k.equal(c1, l);
        k.equal(c2, l);
        k.intersect(c1, c2, g)
    };
    let (ac, aa) = (apex(pa, pb), apex(pb, pc));
    let cp = mk(&mut k, t.a, t.b, t.ab, ac);
    let ap = mk(&mut k, t.b, t.c, t.bc, aa);
    let l1 = k.inf_line(t.c, cp);
    let l2 = k.inf_line(t.a, ap);
    let f0 = xline(pc, sub(ac, pc), pa, sub(aa, pa));
    let f = k.intersect(l1, l2, f0);
    assert_eq!(k.dof(), 3);
    let (a, b, c) = (dist(pb, pc), dist(pc, pa), dist(pa, pb));
    let cc = k.measure(DimKind::Distance(t.c, cp));
    k.check_value(cc, ((a * a + b * b + c * c) / 2.0 + 2.0 * 3f64.sqrt() * area(pa, pb, pc)).sqrt());
    let fa = k.measure(DimKind::Distance(f, t.a));
    let fb = k.measure(DimKind::Distance(f, t.b));
    let fc = k.measure(DimKind::Distance(f, t.c));
    k.check_value(fa, dist(f0, pa));
    k.check_value(fb, dist(f0, pb));
    k.check_value(fc, dist(f0, pc));
    // sum of distances = |CC'|
    assert!((dist(f0, pa) + dist(f0, pb) + dist(f0, pc) - dist(pc, ac)).abs() < 1e-9);
}

/// Tangent segments from an external point to a circle and the chord of contact.
#[test]
fn tangent_chord_of_contact() {
    let mut k = Kit::new();
    let o = k.origin();
    let circ = k.circle(o, 2.0);
    let p = k.point(5.0, 0.5);
    let op = k.line(o, p);
    k.drive(DimKind::Radius(circ), "2");
    k.drive(DimKind::Length(op), "5");
    let d = 5.0f64;
    let r = 2.0f64;
    let ph = (0.5f64).atan2(5.0);
    let _ = ph;
    let base = (0.5f64.atan2(5.0)) + (r / d).acos();
    let g1 = [r * base.cos(), r * base.sin()];
    let base2 = (0.5f64.atan2(5.0)) - (r / d).acos();
    let g2 = [r * base2.cos(), r * base2.sin()];
    let (s1, t1) = k.tangent_line(p, circ, g1);
    let (s2, t2) = k.tangent_line(p, circ, g2);
    assert!(k.dof() > 0);
    let l1 = k.measure(DimKind::Length(s1));
    let l2 = k.measure(DimKind::Length(s2));
    let tl = (d * d - r * r).sqrt();
    k.check_value(l1, tl);
    k.check_value(l2, tl);
    let ch = k.measure(DimKind::Distance(t1, t2));
    k.check_value(ch, 2.0 * r * tl / d);
    let ang = k.measure(DimKind::Angle(s1, s2));
    let _ = ang;
}

/// Mirror image of a circle through its reflected center: the common chord.
#[test]
fn mirrored_circle_common_chord() {
    let mut k = Kit::new();
    let xa = k.xaxis();
    let c = k.point(1.0, 1.5);
    k.drive(DimKind::PointLine(c, xa), "1.5");
    let circ = k.circle(c, 2.5);
    k.drive(DimKind::Radius(circ), "2.5");
    let c2 = k.reflect(c, xa);
    let circ2 = k.circle(c2, 2.5);
    k.equal(circ, circ2);
    assert!(k.dof() > 0);
    let w = (2.5f64 * 2.5 - 1.5 * 1.5).sqrt();
    let x = k.intersect(circ, circ2, [1.0 + w, 0.0]);
    let y = k.intersect(circ, circ2, [1.0 - w, 0.0]);
    let chord = k.measure(DimKind::Distance(x, y));
    k.check_value(chord, 2.0 * w);
    let cc = k.measure(DimKind::Distance(c, c2));
    k.check_value(cc, 3.0);
    let vy = k.measure(DimKind::CoordY(c2));
    k.check_value(vy, -1.5);
}

/// Foot of a perpendicular, fully constrained: coordinates.
#[test]
fn foot_coordinates_fixed() {
    let mut k = Kit::new();
    let o = k.origin();
    let p1 = k.point(0.2, 1.1);
    let p2 = k.point(4.1, 3.9);
    let q = k.point(5.2, 0.3);
    k.drive(DimKind::CoordX(p1), "0");
    k.drive(DimKind::CoordY(p1), "1");
    k.drive(DimKind::CoordX(p2), "4");
    k.drive(DimKind::CoordY(p2), "4");
    k.drive(DimKind::CoordX(q), "5");
    k.drive(DimKind::CoordY(q), "0");
    let l = k.inf_line(p1, p2);
    let f = foot(&mut k, l, q, proj([5.0, 0.0], [0.0, 1.0], [4.0, 4.0]));
    let r = k.reflect(q, l);
    let _ = o;
    assert_eq!(k.dof(), 0);
    let f_ = proj([5.0, 0.0], [0.0, 1.0], [4.0, 4.0]);
    let r_ = refl([5.0, 0.0], [0.0, 1.0], [4.0, 4.0]);
    let fx = k.measure(DimKind::CoordX(f));
    let fy = k.measure(DimKind::CoordY(f));
    let rx = k.measure(DimKind::CoordX(r));
    let ry = k.measure(DimKind::CoordY(r));
    k.check_value(fx, f_[0]);
    k.check_value(fy, f_[1]);
    k.check_value(rx, r_[0]);
    k.check_value(ry, r_[1]);
}

/// Rhombus: the perpendicular bisector of a diagonal contains the other diagonal.
#[test]
fn rhombus_diagonals() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let c = k.point(4.0, 0.2);
    let b = k.point(2.0, -2.2);
    let d = k.point(2.0, 2.3);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let cd = k.line(c, d);
    let da = k.line(d, a);
    let ac = k.line(a, c);
    let bd = k.inf_line(b, d);
    k.drive(DimKind::Length(ab), "3");
    k.equal(ab, bc);
    k.equal(ab, cd);
    k.equal(ab, da);
    k.drive(DimKind::Length(ac), "4");
    assert!(k.dof() > 0);
    let (pb, _) = k.perp_bisector(a, c);
    // in a rhombus the other diagonal is that perpendicular bisector
    let x = k.intersect(pb, ac, [2.0, 0.1]);
    let xb = k.measure(DimKind::Distance(x, b));
    k.check_value(xb, 5f64.sqrt());
    let xd = k.measure(DimKind::Distance(x, d));
    k.check_value(xd, 5f64.sqrt());
    let bdl = k.measure(DimKind::Distance(b, d));
    k.check_value(bdl, 2.0 * 5f64.sqrt());
    let _ = bd;
}

/// Variables: a family of constructions driven by an expression, checked as it changes.
#[test]
fn variable_driven_bisector_distance() {
    let mut k = Kit::new();
    let o = k.origin();
    let w = k.var("w", 2.0);
    let b = k.point(4.0, 0.2);
    let c = k.point(0.5, 3.0);
    let ob = k.line(o, b);
    let oc = k.line(o, c);
    k.drive(DimKind::Length(ob), "3w");
    k.drive(DimKind::Length(oc), "2w");
    let xa = k.xaxis();
    k.horizontal(ob);
    let _ = xa;
    k.drive(DimKind::Angle(ob, oc), "90");
    assert_eq!(k.dof(), 0);
    let m = k.midpoint_of(b, c);
    let (pb, _) = k.perp_bisector(b, c);
    let x = k.intersect(pb, ob, [2.0, 0.0]);
    let mx = k.measure(DimKind::CoordX(x));
    // X = (x, 0) with |XB| = |XC|: (3w - x)^2 = x^2 + 4w^2, so x = 5w/6
    k.check_value(mx, 5.0 * 2.0 / 6.0);
    let d = k.measure(DimKind::Distance(m, x));
    k.check_value(d, 2.0 * 13f64.sqrt() / 3.0);
    k.set_var(w, 3.0);
    k.check_value(mx, 2.5);
    k.check_value(d, 13f64.sqrt());
}

/// Perpendicular bisector chain: the point equidistant from three fixed-angle rays.
#[test]
fn reflection_chain_zigzag() {
    let mut k = Kit::new();
    let o = k.origin();
    let u = k.point(4.0, 0.3);
    let v = k.point(2.0, 3.0);
    let a = k.point(3.0, 1.0);
    let l1 = k.inf_line(o, u);
    let l2 = k.inf_line(o, v);
    let xa = k.xaxis();
    k.drive(DimKind::Angle(xa, l1), "0");
    k.drive(DimKind::Angle(xa, l2), "60");
    k.drive(DimKind::CoordX(a), "3");
    k.drive(DimKind::CoordY(a), "1");
    assert!(k.dof() > 0);
    // reflect three times: l1, l2, l1 -> rotation by 120 then mirror: net reflection in line at -60 deg... 
    let a1 = k.reflect(a, l1);
    let a2 = k.reflect(a1, l2);
    let a3 = k.reflect(a2, l1);
    // angles (polar): a at atan2(1,3)=t; a1 at -t; a2 at 120+t; a3 at -120-t
    let rad = 10f64.sqrt();
    let t = (1.0f64).atan2(3.0);
    let x = k.measure(DimKind::CoordX(a3));
    let y = k.measure(DimKind::CoordY(a3));
    k.check_value(x, rad * (-(120f64.to_radians()) - t).cos());
    k.check_value(y, rad * (-(120f64.to_radians()) - t).sin());
}

/// Equilateral triangle on a side from two equal circles; height and apex coordinates.
#[test]
fn equilateral_apex_coordinates() {
    let mut k = Kit::new();
    let o = k.origin();
    let b = k.point(4.0, 0.2);
    k.drive(DimKind::CoordX(b), "4");
    k.drive(DimKind::CoordY(b), "0");
    let ab = k.line(o, b);
    let c1 = k.circle(o, 4.0);
    let c2 = k.circle(b, 4.0);
    k.equal(c1, ab);
    k.equal(c2, ab);
    let apex = k.intersect(c1, c2, [2.0, 3.0]);
    assert_eq!(k.dof(), 0);
    let x = k.measure(DimKind::CoordX(apex));
    let y = k.measure(DimKind::CoordY(apex));
    k.check_value(x, 2.0);
    k.check_value(y, 12f64.sqrt());
    // centroid of the equilateral triangle via two perpendicular bisectors
    let (l1, _) = k.perp_bisector(o, b);
    let (l2, _) = k.perp_bisector(o, apex);
    let g = k.intersect(l1, l2, [2.0, 1.1]);
    let gy = k.measure(DimKind::CoordY(g));
    k.check_value(gy, 12f64.sqrt() / 3.0);
}

/// Floating equilateral apex: a free segment, the apex stays at sqrt(3)/2 from its midpoint.
#[test]
fn floating_equilateral_apex_height() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(3.0, 0.3);
    let ab = k.line(a, b);
    k.drive(DimKind::Length(ab), "3");
    let c1 = k.circle(a, 3.0);
    let c2 = k.circle(b, 3.0);
    k.equal(c1, ab);
    k.equal(c2, ab);
    let apex = k.intersect(c1, c2, [1.5, 2.5]);
    assert!(k.dof() > 0);
    let m = k.midpoint_of_line(ab);
    let h = k.measure(DimKind::Distance(m, apex));
    k.check_value(h, 3.0 * 3f64.sqrt() / 2.0);
    let side = k.measure(DimKind::Distance(a, apex));
    k.check_value(side, 3.0);
}

/// Arc through three points: the radius from chord and sagitta,
/// r = (c²/4 + s²) / (2s).
#[test]
fn arc_through_three_points() {
    let mut k = Kit::new();
    let a = k.point(-3.1, 0.1);
    let m = k.point(0.1, 1.1);
    let b = k.point(3.0, -0.1);
    k.drive(DimKind::CoordX(a), "-3");
    k.drive(DimKind::CoordY(a), "0");
    k.drive(DimKind::CoordX(b), "3");
    k.drive(DimKind::CoordY(b), "0");
    k.drive(DimKind::CoordX(m), "0");
    k.drive(DimKind::CoordY(m), "1");
    let (arc, c) = k.arc_through(a, m, b);
    assert_eq!(k.dof(), 0);
    let r = k.measure(DimKind::Radius(arc));
    k.check_value(r, 5.0);
    let cy = k.measure(DimKind::CoordY(c));
    k.check_value(cy, -4.0);
}

/// Circle through three points: the center is the circumcenter, a real point.
#[test]
fn circle_through_right_triangle() {
    let mut k = Kit::new();
    let (pa, pb, pc) = ([0.0, 0.0], [6.0, 0.0], [0.0, 8.0]);
    let t = tri(&mut k, pa, pb, pc);
    let (circ, o) = k.circle_through(t.a, t.b, t.c);
    // the hypotenuse is a diameter: the center is its midpoint
    let ob = k.measure(DimKind::Distance(o, t.b));
    let oc = k.measure(DimKind::Distance(o, t.c));
    k.check_value(ob, 5.0);
    k.check_value(oc, 5.0);
    let r = k.measure(DimKind::Radius(circ));
    k.check_value(r, 5.0);
}
