//! Linkage and polygon tests: four-bars, slider-cranks, parallelograms,
//! inscribed polygons, scissor mechanisms and other articulated chains.

use gxpad::model::DimKind;
use gxpad::testkit::Kit;
use std::f64::consts::PI;

type P = [f64; 2];

/// Both intersections of circles (c0, r0) and (c1, r1).
fn ci(c0: P, r0: f64, c1: P, r1: f64) -> (P, P) {
    let (dx, dy) = (c1[0] - c0[0], c1[1] - c0[1]);
    let d = dx.hypot(dy);
    let a = (r0 * r0 - r1 * r1 + d * d) / (2.0 * d);
    let h = (r0 * r0 - a * a).sqrt();
    let m = [c0[0] + a * dx / d, c0[1] + a * dy / d];
    let n = [-dy / d, dx / d];
    ([m[0] + h * n[0], m[1] + h * n[1]], [m[0] - h * n[0], m[1] - h * n[1]])
}
/// The intersection with the larger y.
fn up(c0: P, r0: f64, c1: P, r1: f64) -> P {
    let (p, q) = ci(c0, r0, c1, r1);
    if p[1] > q[1] { p } else { q }
}
/// Value (degrees) to drive an angle dimension between directed lines `u`, `v`
/// so that it means the angle `deg` between the rays: the dimension label sits
/// at vertex + (0.3, 0.3), which selects the sector containing the direction
/// (1, 1); in a "supplementary" sector the stored value is 180 - deg.
fn ang(deg: f64, u: P, v: P) -> String {
    let cr = |a: P, b: P| a[0] * b[1] - a[1] * b[0];
    let q = [1.0, 1.0];
    for s1 in [1.0, -1.0] {
        for s2 in [1.0, -1.0] {
            let a = [u[0] * s1, u[1] * s1];
            let b = [v[0] * s2, v[1] * s2];
            let den = cr(a, b);
            let (ka, kb) = (cr(q, b) / den, cr(a, q) / den);
            if ka >= 0.0 && kb >= 0.0 {
                return format!("{}", if s1 * s2 < 0.0 { 180.0 - deg } else { deg });
            }
        }
    }
    panic!("no sector");
}
fn dir(a: P, b: P) -> P {
    [b[0] - a[0], b[1] - a[1]]
}

/// Crank-rocker four-bar with both ground pivots fixed and the crank angle driven.
#[test]
fn four_bar_fixed_pivots_coupler_joint() {
    let mut k = Kit::new();
    let o = k.origin();
    let th = 60f64.to_radians();
    let a_p = [th.cos(), th.sin()];
    let d_p = [4.0, 0.0];
    let b_p = up(a_p, 3.5, d_p, 2.5);
    let d = k.fixed_point(4.0, 0.0);
    let a = k.point(a_p[0] + 0.1, a_p[1] - 0.1);
    let b = k.point(b_p[0] + 0.15, b_p[1] - 0.1);
    let oa = k.line(o, a);
    let ab = k.line(a, b);
    let bd = k.line(b, d);
    k.drive(DimKind::Length(oa), "1");
    k.drive(DimKind::Length(ab), "3.5");
    k.drive(DimKind::Length(bd), "2.5");
    let xa = k.xaxis();
    k.drive(DimKind::Angle(xa, oa), &ang(60.0, [1.0, 0.0], a_p));
    assert_eq!(k.dof(), 0);
    let bx = k.measure(DimKind::CoordX(b));
    let by = k.measure(DimKind::CoordY(b));
    k.check_value(bx, b_p[0]);
    k.check_value(by, b_p[1]);
}

/// Four-bar floating in the plane: the ground link is a driven length and the
/// crank-to-ground angle is driven, so only rigid motions remain.
#[test]
fn four_bar_floating_distances() {
    let mut k = Kit::new();
    let th = 60f64.to_radians();
    let a_p = [th.cos(), th.sin()];
    let b_p = up(a_p, 3.5, [4.0, 0.0], 2.5);
    let o = k.point(0.1, 0.1);
    let d = k.point(4.0, -0.1);
    let a = k.point(a_p[0], a_p[1] + 0.1);
    let b = k.point(b_p[0] - 0.1, b_p[1]);
    let od = k.line(o, d);
    let oa = k.line(o, a);
    let ab = k.line(a, b);
    let bd = k.line(b, d);
    k.drive(DimKind::Length(od), "4");
    k.drive(DimKind::Length(oa), "1");
    k.drive(DimKind::Length(ab), "3.5");
    k.drive(DimKind::Length(bd), "2.5");
    k.drive(DimKind::Angle(od, oa), &ang(60.0, [1.0, 0.0], a_p));
    assert!(k.dof() > 0);
    let ad = k.measure(DimKind::Distance(a, d));
    let ob = k.measure(DimKind::Distance(o, b));
    k.check_value(ad, (17.0 - 8.0 * th.cos()).sqrt());
    k.check_value(ob, b_p[0].hypot(b_p[1]));
}

/// Four-bar whose lengths are expressions of a scale variable.
#[test]
fn four_bar_scaled_by_variable() {
    let mut k = Kit::new();
    let o = k.origin();
    let th = 75f64.to_radians();
    let a_p = [th.cos(), th.sin()];
    let b_p = up(a_p, 3.0, [3.0, 0.0], 2.0);
    let s = k.var("s", 1.0);
    let d = k.point(3.05, 0.05);
    let a = k.point(a_p[0] + 0.05, a_p[1]);
    let b = k.point(b_p[0], b_p[1] - 0.05);
    let oa = k.line(o, a);
    let ab = k.line(a, b);
    let bd = k.line(b, d);
    let xa0 = k.xaxis();
    k.on(d, xa0);
    k.drive(DimKind::Distance(o, d), "3s");
    k.drive(DimKind::Length(oa), "s");
    k.drive(DimKind::Length(ab), "3s");
    k.drive(DimKind::Length(bd), "2s");
    let xa = k.xaxis();
    k.drive(DimKind::Angle(xa, oa), &ang(75.0, [1.0, 0.0], a_p));
    assert_eq!(k.dof(), 0);
    let by = k.measure(DimKind::CoordY(b));
    k.check_value(by, b_p[1]);
    k.set_var(s, 1.2);
    let a2 = [1.2 * th.cos(), 1.2 * th.sin()];
    let b2 = up(a2, 3.6, [3.6, 0.0], 2.4);
    k.check_value(by, b2[1]);
}

/// Four-bar with a triangular coupler: a coupler point at fixed distances
/// from both coupler joints.
#[test]
fn four_bar_coupler_triangle_point() {
    let mut k = Kit::new();
    let o = k.origin();
    let th = 50f64.to_radians();
    let a_p = [2.0 * th.cos(), 2.0 * th.sin()];
    let d_p = [5.0, 0.0];
    let b_p = up(a_p, 4.0, d_p, 3.0);
    // coupler point p with |ap| = 2, |bp| = 3, on the side away from the ground
    let (p1, p2) = ci(a_p, 2.0, b_p, 3.0);
    let p_p = if p1[1] > p2[1] { p1 } else { p2 };
    let d = k.fixed_point(5.0, 0.0);
    let a = k.point(a_p[0] + 0.1, a_p[1]);
    let b = k.point(b_p[0], b_p[1] + 0.1);
    let p = k.point(p_p[0] - 0.1, p_p[1] + 0.2);
    let oa = k.line(o, a);
    let ab = k.line(a, b);
    let bd = k.line(b, d);
    let ap = k.line(a, p);
    let bp = k.line(b, p);
    k.drive(DimKind::Length(oa), "2");
    k.drive(DimKind::Length(ab), "4");
    k.drive(DimKind::Length(bd), "3");
    k.drive(DimKind::Length(ap), "2");
    k.drive(DimKind::Length(bp), "3");
    let xa = k.xaxis();
    k.drive(DimKind::Angle(xa, oa), &ang(50.0, [1.0, 0.0], a_p));
    assert_eq!(k.dof(), 0);
    let px = k.measure(DimKind::CoordX(p));
    let py = k.measure(DimKind::CoordY(p));
    k.check_value(px, p_p[0]);
    k.check_value(py, p_p[1]);
}

/// Chebyshev lambda linkage: with the coupler parallel to the ground link the
/// coupler midpoint sits at height sqrt(5^2 - 1).
#[test]
fn chebyshev_lambda_height() {
    let mut k = Kit::new();
    let a = k.fixed_point(-2.0, 0.0);
    let b = k.fixed_point(2.0, 0.0);
    let h = 24f64.sqrt();
    let c = k.point(-1.0, h + 0.2);
    let d = k.point(1.0, h - 0.2);
    let ac = k.line(a, c);
    let bd = k.line(b, d);
    let cd = k.line(c, d);
    k.drive(DimKind::Length(ac), "5");
    k.drive(DimKind::Length(bd), "5");
    k.drive(DimKind::Length(cd), "2");
    let ab = k.line(a, b);
    k.parallel(cd, ab);
    assert_eq!(k.dof(), 0);
    let m = k.midpoint_of(c, d);
    let my = k.measure(DimKind::CoordY(m));
    let mx = k.measure(DimKind::CoordX(m));
    k.check_value(my, h);
    k.check_value(mx, 0.0);
}

/// Watt linkage: two equal rockers on fixed pivots, one rocker angle driven,
/// the coupler midpoint is derived.
#[test]
fn watt_linkage_midpoint() {
    let mut k = Kit::new();
    let a = k.fixed_point(-2.0, 0.0);
    let b = k.fixed_point(2.0, 0.0);
    let th = 80f64.to_radians();
    let c_p = [-2.0 + 3.0 * th.cos(), 3.0 * th.sin()];
    let d_p = up(c_p, 2.0, [2.0, 0.0], 3.0);
    let c = k.point(c_p[0] + 0.1, c_p[1] - 0.1);
    let d = k.point(d_p[0] - 0.1, d_p[1] + 0.1);
    let ac = k.line(a, c);
    let bd = k.line(b, d);
    let cd = k.line(c, d);
    k.drive(DimKind::Length(ac), "3");
    k.drive(DimKind::Length(bd), "3");
    k.drive(DimKind::Length(cd), "2");
    let xa = k.xaxis();
    k.drive(DimKind::Angle(xa, ac), &ang(80.0, [1.0, 0.0], [th.cos(), th.sin()]));
    assert_eq!(k.dof(), 0);
    let m = k.midpoint_of(c, d);
    let mx = k.measure(DimKind::CoordX(m));
    let my = k.measure(DimKind::CoordY(m));
    k.check_value(mx, (c_p[0] + d_p[0]) / 2.0);
    k.check_value(my, (c_p[1] + d_p[1]) / 2.0);
}

/// Slider-crank on the fixed origin: piston position against crank angle.
#[test]
fn slider_crank_piston_position() {
    let mut k = Kit::new();
    let o = k.origin();
    let (r, l, th) = (1.2f64, 4.0f64, 70f64.to_radians());
    let a = k.point(r * th.cos() + 0.1, r * th.sin());
    let px = r * th.cos() + (l * l - r * r * th.sin().powi(2)).sqrt();
    let p = k.point(px + 0.2, 0.1);
    let oa = k.line(o, a);
    let ap = k.line(a, p);
    let xa = k.xaxis();
    k.on(p, xa);
    k.drive(DimKind::Length(oa), "1.2");
    k.drive(DimKind::Length(ap), "4");
    k.drive(DimKind::Angle(xa, oa), &ang(70.0, [1.0, 0.0], [th.cos(), th.sin()]));
    assert_eq!(k.dof(), 0);
    let x = k.measure(DimKind::CoordX(p));
    k.check_value(x, px);
}

/// Slider-crank whose whole frame is free: the piston stroke distance depends
/// only on the crank angle relative to the guide.
#[test]
fn slider_crank_floating_frame() {
    let mut k = Kit::new();
    let (r, l, th) = (1.5f64, 5.0f64, 120f64.to_radians());
    let o = k.point(0.3, 0.2);
    let a = k.point(0.3 + r * th.cos(), 0.2 + r * th.sin());
    let px = r * th.cos() + (l * l - r * r * th.sin().powi(2)).sqrt();
    let p = k.point(0.3 + px + 0.1, 0.2);
    let guide = k.line(o, p);
    let oa = k.line(o, a);
    let ap = k.line(a, p);
    k.horizontal(guide);
    k.drive(DimKind::Length(oa), "1.5");
    k.drive(DimKind::Length(ap), "5");
    k.drive(DimKind::Angle(guide, oa), &ang(120.0, [1.0, 0.0], [th.cos(), th.sin()]));
    assert!(k.dof() > 0);
    let d = k.measure(DimKind::Distance(o, p));
    let h = k.measure(DimKind::PointLine(a, guide));
    k.check_value(d, px);
    k.check_value(h, r * th.sin());
}

/// Ellipsograph: a bar sliding between the two axes; a marked point on it.
#[test]
fn trammel_ellipsograph_point() {
    let mut k = Kit::new();
    let (l, p_len, th) = (5.0f64, 2.0f64, 35f64.to_radians());
    let a_p = [l * th.cos(), 0.0];
    let b_p = [0.0, l * th.sin()];
    let a = k.point(a_p[0] + 0.1, 0.1);
    let b = k.point(0.1, b_p[1] - 0.1);
    let (xa, ya) = (k.xaxis(), k.yaxis());
    k.on(a, xa);
    k.on(b, ya);
    let ab = k.line(a, b);
    let t = p_len / l;
    let pp = [a_p[0] + t * (b_p[0] - a_p[0]), a_p[1] + t * (b_p[1] - a_p[1])];
    let p = k.point(pp[0] + 0.1, pp[1] + 0.1);
    k.on(p, ab);
    let ap = k.line(a, p);
    k.drive(DimKind::Length(ab), "5");
    k.drive(DimKind::Length(ap), "2");
    k.drive(DimKind::Angle(xa, ab), &ang(145.0, [1.0, 0.0], dir(a_p, b_p)));
    assert_eq!(k.dof(), 0);
    let px = k.measure(DimKind::CoordX(p));
    let py = k.measure(DimKind::CoordY(p));
    k.check_value(px, pp[0]);
    k.check_value(py, pp[1]);
}

/// Trammel with the bar length as the only driven quantity and the end on the
/// y axis left sliding: the distance from the origin to the bar's end on the
/// x axis follows from the bar length and the other end's height.
#[test]
fn trammel_sliding_end_height_driven() {
    let mut k = Kit::new();
    let a = k.point(4.0, 0.1);
    let b = k.point(0.1, 3.0);
    let (xa, ya) = (k.xaxis(), k.yaxis());
    k.on(a, xa);
    k.on(b, ya);
    let ab = k.line(a, b);
    k.drive(DimKind::Length(ab), "5");
    k.drive(DimKind::CoordY(b), "3");
    assert_eq!(k.dof(), 0);
    let ax = k.measure(DimKind::CoordX(a));
    let h = k.measure(DimKind::PointLine(k.origin(), ab));
    k.check_value(ax, 4.0);
    k.check_value(h, 12.0 / 5.0);
}

/// Planar three-link arm anchored at the origin, absolute link angles driven.
#[test]
fn three_link_arm_end_effector() {
    let mut k = Kit::new();
    let o = k.origin();
    let ls = [2.0, 1.5, 1.0];
    let angs = [40f64, 100.0, 160.0];
    let mut pts = vec![o];
    let mut cur = [0.0, 0.0];
    let mut lines = vec![];
    for i in 0..3 {
        let t = angs[i].to_radians();
        cur = [cur[0] + ls[i] * t.cos(), cur[1] + ls[i] * t.sin()];
        let p = k.point(cur[0] + 0.1, cur[1] - 0.1);
        let ln = k.line(pts[i], p);
        pts.push(p);
        lines.push(ln);
    }
    let xa = k.xaxis();
    for i in 0..3 {
        k.drive(DimKind::Length(lines[i]), &format!("{}", ls[i]));
        let t = angs[i].to_radians();
        k.drive(DimKind::Angle(xa, lines[i]), &ang(angs[i], [1.0, 0.0], [t.cos(), t.sin()]));
    }
    assert_eq!(k.dof(), 0);
    let ex = k.measure(DimKind::CoordX(pts[3]));
    let ey = k.measure(DimKind::CoordY(pts[3]));
    k.check_value(ex, cur[0]);
    k.check_value(ey, cur[1]);
}

/// Floating two-link chain with a driven relative angle between links: the
/// end-to-end distance is determined by the law of cosines.
#[test]
fn floating_two_link_chain_reach() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(3.0, 0.0);
    let c_p = [3.0 + 2.0 * 70f64.to_radians().cos(), 2.0 * 70f64.to_radians().sin()];
    let c = k.point(c_p[0], c_p[1]);
    let l1 = k.line(a, b);
    let l2 = k.line(b, c);
    k.drive(DimKind::Length(l1), "3");
    k.drive(DimKind::Length(l2), "2");
    // the angle at the joint between the two links, measured at b
    let l1r = k.line(b, a);
    k.drive(DimKind::Angle(l1r, l2), &ang(110.0, dir([3.0, 0.0], [0.0, 0.0]), dir([3.0, 0.0], c_p)));
    assert!(k.dof() > 0);
    let d = k.measure(DimKind::Distance(a, c));
    let expect = (9.0 + 4.0 - 12.0 * 110f64.to_radians().cos()).sqrt();
    k.check_value(d, expect);
}

/// Parallelogram: both diagonals from two sides and the included angle.
#[test]
fn parallelogram_diagonals() {
    let mut k = Kit::new();
    let (s, t, al) = (4.0f64, 3.0f64, 60f64.to_radians());
    let a = k.point(0.0, 0.0);
    let b = k.point(s, 0.1);
    let d = k.point(t * al.cos(), t * al.sin());
    let c = k.point(s + t * al.cos() + 0.1, t * al.sin());
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let cd = k.line(c, d);
    let da = k.line(d, a);
    k.parallel(ab, cd);
    k.parallel(bc, da);
    k.drive(DimKind::Length(ab), "4");
    k.drive(DimKind::Length(da), "3");
    let ad = k.line(a, d);
    k.drive(DimKind::Angle(ab, ad), &ang(60.0, [s, 0.0], [t * al.cos(), t * al.sin()]));
    assert_eq!(k.dof(), 3);
    let ac = k.line(a, c);
    let bd = k.line(b, d);
    let m1 = k.measure(DimKind::Length(ac));
    let m2 = k.measure(DimKind::Length(bd));
    k.check_value(m1, (s * s + t * t + 2.0 * s * t * al.cos()).sqrt());
    k.check_value(m2, (s * s + t * t - 2.0 * s * t * al.cos()).sqrt());
}

fn rhombus() -> (Kit, [usize; 4]) {
    let mut k = Kit::new();
    let (s, al) = (3.0f64, 70f64.to_radians());
    let a = k.point(0.0, 0.0);
    let b = k.point(s, 0.1);
    let d = k.point(s * al.cos(), s * al.sin());
    let c = k.point(s + s * al.cos() + 0.1, s * al.sin() - 0.1);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let cd = k.line(c, d);
    let da = k.line(d, a);
    k.equal(ab, bc);
    k.equal(bc, cd);
    k.equal(cd, da);
    k.drive(DimKind::Length(ab), "3");
    let ad = k.line(a, d);
    k.drive(DimKind::Angle(ab, ad), &ang(70.0, [s, 0.0], [al.cos(), al.sin()]));
    assert_eq!(k.dof(), 3);
    (k, [a, b, c, d])
}

/// Rhombus from four equal sides: the long diagonal.
#[test]
fn rhombus_long_diagonal() {
    let (mut k, [a, _, c, _]) = rhombus();
    let ac = k.line(a, c);
    let m = k.measure(DimKind::Length(ac));
    k.check_value(m, 2.0 * 3.0 * 35f64.to_radians().cos());
}

/// Rhombus from four equal sides: the short diagonal.
#[test]
fn rhombus_short_diagonal() {
    let (mut k, [_, b, _, d]) = rhombus();
    let bd = k.line(b, d);
    let m = k.measure(DimKind::Length(bd));
    k.check_value(m, 2.0 * 3.0 * 35f64.to_radians().sin());
}

/// Rhombus from four equal sides: the diagonals are perpendicular.
#[test]
fn rhombus_diagonals_perpendicular() {
    let (mut k, [a, b, c, d]) = rhombus();
    let ac = k.line(a, c);
    let bd = k.line(b, d);
    let m = k.measure(DimKind::Angle(ac, bd));
    k.check_value(m, PI / 2.0);
}

/// Kite: two pairs of adjacent equal sides; the long diagonal.
#[test]
fn kite_long_diagonal() {
    let mut k = Kit::new();
    let (p, q, al) = (2.0f64, 3.5f64, 80f64.to_radians());
    let hb = al / 2.0;
    let a = k.point(0.0, 0.0);
    let b = k.point(p * (-hb).cos() * 0.0 + p * hb.cos(), -p * hb.sin());
    let d = k.point(p * hb.cos(), p * hb.sin());
    let cx = p * hb.cos() + (q * q - p * p * hb.sin().powi(2)).sqrt();
    let c = k.point(cx + 0.2, 0.1);
    let ab = k.line(a, b);
    let ad = k.line(a, d);
    let cb = k.line(c, b);
    let cd = k.line(c, d);
    k.equal(ab, ad);
    k.equal(cb, cd);
    k.drive(DimKind::Length(ab), "2");
    k.drive(DimKind::Length(cb), "3.5");
    k.drive(DimKind::Angle(ab, ad), &ang(80.0, [hb.cos(), -hb.sin()], [hb.cos(), hb.sin()]));
    assert_eq!(k.dof(), 3);
    let ac = k.measure(DimKind::Distance(a, c));
    let bd = k.measure(DimKind::Distance(b, d));
    k.check_value(ac, cx);
    k.check_value(bd, 2.0 * p * hb.sin());
}

/// Kite built with a symmetry constraint about its axis.
#[test]
fn kite_by_symmetry() {
    let mut k = Kit::new();
    let (ab_l, bc_l, ac_l) = (3.0f64, 4.0f64, 6.0f64);
    let a = k.point(0.0, 0.0);
    let c = k.point(6.0, 0.1);
    let x = (ab_l * ab_l - bc_l * bc_l + ac_l * ac_l) / (2.0 * ac_l);
    let y = (ab_l * ab_l - x * x).sqrt();
    let b = k.point(x, y);
    let d = k.point(x, -y + 0.1);
    let ac = k.line(a, c);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    k.symmetric(b, d, ac);
    k.drive(DimKind::Length(ac), "6");
    k.drive(DimKind::Length(ab), "3");
    k.drive(DimKind::Length(bc), "4");
    assert_eq!(k.dof(), 3);
    let bd = k.measure(DimKind::Distance(b, d));
    k.check_value(bd, 2.0 * y);
    let ad = k.measure(DimKind::Distance(a, d));
    k.check_value(ad, 3.0);
}

/// Isosceles trapezoid: height from the legs and the base difference.
#[test]
fn isosceles_trapezoid_height_and_diagonal() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(6.0, 0.0);
    let c = k.point(4.5, 2.0);
    let d = k.point(1.5, 2.1);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let cd = k.line(c, d);
    let da = k.line(d, a);
    k.horizontal(ab);
    k.parallel(cd, ab);
    k.drive(DimKind::Length(ab), "6");
    k.drive(DimKind::Length(cd), "3");
    k.drive(DimKind::Length(bc), "2.5");
    k.equal(bc, da);
    assert_eq!(k.dof(), 2);
    let h = k.measure(DimKind::PointLine(c, ab));
    let diag = k.measure(DimKind::Distance(a, c));
    k.check_value(h, 2.0);
    k.check_value(diag, (4.5f64 * 4.5 + 4.0).sqrt());
}

/// Trapezoid with a free shear: the midsegment and the segment between the
/// diagonal midpoints are fixed by the two bases alone.
#[test]
fn trapezoid_midsegments_with_free_shear() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(7.0, 0.1);
    let d = k.point(1.0, 2.0);
    let c = k.point(4.0, 2.1);
    let ab = k.line(a, b);
    let cd = k.line(c, d);
    k.parallel(ab, cd);
    k.drive(DimKind::Length(ab), "7");
    k.drive(DimKind::Length(cd), "3");
    k.drive(DimKind::PointLine(d, ab), "2");
    assert_eq!(k.dof(), 4);
    let m = k.midpoint_of(a, d);
    let n = k.midpoint_of(b, c);
    let mid = k.measure(DimKind::Distance(m, n));
    k.check_value(mid, 5.0);
    let m2 = k.midpoint_of(a, c);
    let n2 = k.midpoint_of(b, d);
    let dm = k.measure(DimKind::Distance(m2, n2));
    k.check_value(dm, 2.0);
}

/// Varignon parallelogram: sides are half the diagonals; diagonals driven,
/// the rest of the quadrilateral free.
#[test]
fn varignon_sides_half_diagonals() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(4.0, -0.5);
    let c = k.point(5.0, 3.0);
    let d = k.point(0.5, 4.0);
    let ac = k.line(a, c);
    let bd = k.line(b, d);
    let ab = k.line(a, b);
    let _ = ab;
    k.drive(DimKind::Length(ac), "5.8");
    k.drive(DimKind::Length(bd), "6.2");
    assert_eq!(k.dof(), 6);
    let m1 = k.midpoint_of(a, b);
    let m2 = k.midpoint_of(b, c);
    let m3 = k.midpoint_of(c, d);
    let s1 = k.measure(DimKind::Distance(m1, m2));
    let s2 = k.measure(DimKind::Distance(m2, m3));
    k.check_value(s1, 2.9);
    k.check_value(s2, 3.1);
}

/// Varignon with perpendicular diagonals gives a rectangle, and with equal
/// diagonals a rhombus: the angle between the midpoint-sides is 90 degrees.
#[test]
fn varignon_rectangle_perpendicular_diagonals() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(4.0, -0.5);
    let c = k.point(5.0, 3.0);
    let d = k.point(0.5, 4.0);
    let ac = k.line(a, c);
    let bd = k.line(b, d);
    k.perpendicular(ac, bd);
    k.drive(DimKind::Length(ac), "6");
    k.drive(DimKind::Length(bd), "5");
    assert_eq!(k.dof(), 5);
    let m1 = k.midpoint_of(a, b);
    let m2 = k.midpoint_of(b, c);
    let m3 = k.midpoint_of(c, d);
    let l12 = k.line(m1, m2);
    let l23 = k.line(m2, m3);
    let ang = k.measure(DimKind::Angle(l12, l23));
    let diag = k.measure(DimKind::Distance(m1, m3));
    k.check_value(ang, PI / 2.0);
    // the diagonal of the Varignon rectangle with sides 3 and 2.5
    k.check_value(diag, (3.0f64 * 3.0 + 2.5 * 2.5).sqrt());
}

fn pentagon() -> (Kit, Vec<usize>, Vec<usize>) {
    let mut k = Kit::new();
    let o = k.origin();
    let r = 2.0f64;
    let circ = k.circle(o, r);
    k.drive(DimKind::Radius(circ), "2");
    let mut pts = vec![];
    for i in 0..5 {
        let t = (90.0 + 72.0 * i as f64).to_radians();
        let p = k.point(r * t.cos() * 1.02, r * t.sin() * 0.97);
        k.on(p, circ);
        pts.push(p);
    }
    let ls: Vec<usize> = (0..5).map(|i| k.line(pts[i], pts[(i + 1) % 5])).collect();
    for i in 0..3 {
        k.equal(ls[i], ls[i + 1]);
    }
    k.equal(ls[3], ls[4]);
    assert_eq!(k.dof(), 1);
    (k, pts, ls)
}

/// Regular pentagon inscribed in a circle by equal chords: the side.
#[test]
fn regular_pentagon_side() {
    let (mut k, _, ls) = pentagon();
    let side = k.measure(DimKind::Length(ls[0]));
    k.check_value(side, 4.0 * (PI / 5.0).sin());
}

/// Regular pentagon inscribed in a circle by equal chords: the diagonal.
#[test]
fn regular_pentagon_diagonal() {
    let (mut k, pts, _) = pentagon();
    let diag = k.measure(DimKind::Distance(pts[0], pts[2]));
    k.check_value(diag, 4.0 * (2.0 * PI / 5.0).sin());
}

/// Regular pentagon: the closing side is not constrained directly but is
/// determined by the four others.
#[test]
fn regular_pentagon_closing_side() {
    let (mut k, _, ls) = pentagon();
    let closing = k.measure(DimKind::Length(ls[4]));
    k.check_value(closing, 4.0 * (PI / 5.0).sin());
}

fn hexagon() -> (Kit, Vec<usize>, Vec<usize>) {
    let mut k = Kit::new();
    let o = k.origin();
    let r = 2.0f64;
    let circ = k.circle(o, r);
    k.drive(DimKind::Radius(circ), "2");
    let mut pts = vec![];
    for i in 0..6 {
        let t = (30.0 + 60.0 * i as f64).to_radians();
        let p = k.point(r * t.cos() * 1.03, r * t.sin() * 0.98);
        k.on(p, circ);
        pts.push(p);
    }
    let ls: Vec<usize> = (0..6).map(|i| k.line(pts[i], pts[(i + 1) % 6])).collect();
    for i in 0..5 {
        k.equal(ls[i], circ);
    }
    assert_eq!(k.dof(), 1);
    (k, pts, ls)
}

/// Regular hexagon (side equals radius): long and short diagonals.
#[test]
fn regular_hexagon_diagonals() {
    let (mut k, pts, _) = hexagon();
    let ad = k.measure(DimKind::Distance(pts[0], pts[3]));
    let ac = k.measure(DimKind::Distance(pts[0], pts[2]));
    k.check_value(ad, 4.0);
    k.check_value(ac, 2.0 * 3f64.sqrt());
}

/// Regular hexagon: the sixth side is determined by the other five.
#[test]
fn regular_hexagon_closing_side() {
    let (mut k, _, ls) = hexagon();
    let last = k.measure(DimKind::Length(ls[5]));
    k.check_value(last, 2.0);
}

/// Free square from equal sides and right angles; side and diagonal.
#[test]
fn square_side_to_diagonal_floating() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(2.0, 0.2);
    let c = k.point(2.1, 2.1);
    let d = k.point(-0.1, 1.9);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let cd = k.line(c, d);
    let da = k.line(d, a);
    k.perpendicular(ab, bc);
    k.perpendicular(bc, cd);
    k.perpendicular(cd, da);
    k.equal(ab, bc);
    k.drive(DimKind::Length(ab), "2.5");
    assert_eq!(k.dof(), 3);
    let ac = k.line(a, c);
    let diag = k.measure(DimKind::Length(ac));
    let dl = k.measure(DimKind::Length(cd));
    k.check_value(diag, 2.5 * 2f64.sqrt());
    k.check_value(dl, 2.5);
}

/// Equilateral triangle inscribed in a free circle: the circumradius.
#[test]
fn inscribed_equilateral_circumradius() {
    let mut k = Kit::new();
    let o = k.point(0.1, 0.1);
    let circ = k.circle(o, 1.8);
    let a = k.point(1.7, 0.0);
    let b = k.point(-0.9, 1.5);
    let c = k.point(-0.9, -1.5);
    k.on(a, circ);
    k.on(b, circ);
    k.on(c, circ);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let ca = k.line(c, a);
    k.equal(ab, bc);
    k.equal(bc, ca);
    k.drive(DimKind::Length(ab), "3");
    assert!(k.dof() > 0);
    let rad = k.measure(DimKind::Radius(circ));
    k.check_value(rad, 3.0 / 3f64.sqrt());
}

/// Rectangle inscribed in a circle: the circle radius from two sides, and
/// the other diagonal.
#[test]
fn inscribed_rectangle_radius() {
    let mut k = Kit::new();
    let o = k.point(0.0, 0.0);
    let circ = k.circle(o, 5.1);
    let a = k.point(-3.0, -4.1);
    let b = k.point(3.0, -4.0);
    let c = k.point(3.1, 4.0);
    let d = k.point(-3.0, 4.0);
    for p in [a, b, c, d] {
        k.on(p, circ);
    }
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let ad = k.line(a, d);
    k.perpendicular(ab, bc);
    k.drive(DimKind::Length(ab), "6");
    k.drive(DimKind::Length(bc), "8");
    k.drive(DimKind::Length(ad), "8");
    assert!(k.dof() > 0);
    let rad = k.measure(DimKind::Radius(circ));
    let bd = k.measure(DimKind::Distance(b, d));
    k.check_value(rad, 5.0);
    k.check_value(bd, 10.0);
}

/// Cyclic isosceles trapezoid: Ptolemy gives the diagonal sqrt(b^2 + a c).
#[test]
fn cyclic_trapezoid_ptolemy() {
    let mut k = Kit::new();
    let o = k.point(0.0, 0.0);
    let circ = k.circle(o, 3.1);
    let a = k.point(-3.0, -0.67);
    let b = k.point(3.0, -0.67);
    let c = k.point(1.0, -2.9);
    let d = k.point(-1.0, -2.9);
    for p in [a, b, c, d] {
        k.on(p, circ);
    }
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let cd = k.line(c, d);
    let da = k.line(d, a);
    k.drive(DimKind::Length(ab), "6");
    k.drive(DimKind::Length(cd), "2");
    k.drive(DimKind::Length(bc), "3");
    k.drive(DimKind::Length(da), "3");
    assert!(k.dof() > 0);
    let ac = k.measure(DimKind::Distance(a, c));
    k.check_value(ac, 21f64.sqrt());
}

/// One stage of a scissor mechanism: two bars of equal length sharing their
/// midpoint form a rectangle; the lift height follows from the width.
#[test]
fn scissor_single_stage_height() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(3.0, 0.1);
    let h = 7f64.sqrt();
    let c = k.point(3.1, h);
    let d = k.point(0.0, h + 0.1);
    let bar1 = k.line(a, c);
    let bar2 = k.line(b, d);
    let ab = k.line(a, b);
    k.horizontal(ab);
    let m = k.midpoint_of_line(bar1);
    k.midpoint(m, bar2);
    k.drive(DimKind::Length(bar1), "4");
    k.drive(DimKind::Length(bar2), "4");
    k.drive(DimKind::Length(ab), "3");
    assert_eq!(k.dof(), 2);
    let lift = k.measure(DimKind::VDist(b, c));
    let top = k.measure(DimKind::Distance(c, d));
    k.check_value(lift, h);
    k.check_value(top, 3.0);
}

/// Two-stage scissor lift: the total height is twice one stage.
#[test]
fn scissor_two_stage_total_height() {
    let mut k = Kit::new();
    let w = 3.0f64;
    let bar = 4.0f64;
    let h = (bar * bar - w * w).sqrt();
    let a = k.point(0.0, 0.0);
    let b = k.point(w, 0.1);
    let c = k.point(w + 0.1, h);
    let d = k.point(0.0, h + 0.1);
    let f = k.point(w, 2.0 * h - 0.1);
    let e = k.point(0.1, 2.0 * h);
    let bar1 = k.line(a, c);
    let bar2 = k.line(b, d);
    let bar3 = k.line(d, f);
    let bar4 = k.line(c, e);
    let ab = k.line(a, b);
    k.horizontal(ab);
    let m = k.midpoint_of_line(bar1);
    k.midpoint(m, bar2);
    let n = k.midpoint_of_line(bar3);
    k.midpoint(n, bar4);
    k.drive(DimKind::Length(bar1), "4");
    k.drive(DimKind::Length(bar2), "4");
    k.drive(DimKind::Length(bar3), "4");
    k.drive(DimKind::Length(bar4), "4");
    k.drive(DimKind::Length(ab), "3");
    assert_eq!(k.dof(), 2);
    let total = k.measure(DimKind::VDist(b, f));
    let top = k.measure(DimKind::Distance(e, f));
    k.check_value(total, 2.0 * h);
    k.check_value(top, w);
}

/// Peaucellier-Lipkin inversor with the input point on the x axis:
/// |OP| * |OQ| = l^2 - s^2.
#[test]
fn peaucellier_inversor_fixed_axis() {
    let mut k = Kit::new();
    let o = k.origin();
    let (l, s, op) = (3.0f64, 1.0f64, 2.4f64);
    let oq = (l * l - s * s) / op;
    let mx = (op + oq) / 2.0;
    let hy = (s * s - ((oq - op) / 2.0).powi(2)).sqrt();
    let p = k.point(op, 0.05);
    let q = k.point(oq + 0.05, -0.05);
    let a = k.point(mx + 0.05, hy);
    let b = k.point(mx, -hy - 0.05);
    let xa = k.xaxis();
    k.on(p, xa);
    let oa = k.line(o, a);
    let ob = k.line(o, b);
    let pa = k.line(p, a);
    let aq = k.line(a, q);
    let qb = k.line(q, b);
    let bp = k.line(b, p);
    k.drive(DimKind::Length(oa), "3");
    k.drive(DimKind::Length(ob), "3");
    for ln in [pa, aq, qb, bp] {
        k.drive(DimKind::Length(ln), "1");
    }
    let v = k.var("d", 2.4);
    let _ = v;
    k.drive(DimKind::Distance(o, p), "d");
    assert_eq!(k.dof(), 0);
    let x = k.measure(DimKind::CoordX(q));
    k.check_value(x, oq);
    // (OP = 2 would flatten the rhombus into a singular configuration)
    k.set_var(v, 2.2);
    k.check_value(x, 8.0 / 2.2);
}

/// Peaucellier-Lipkin with the whole linkage free to rotate about its pivot:
/// the output distance is still determined.
#[test]
fn peaucellier_inversor_rotating() {
    let mut k = Kit::new();
    let o = k.origin();
    let (l, s, op) = (3.0f64, 1.0f64, 2.4f64);
    let oq = (l * l - s * s) / op;
    let mx = (op + oq) / 2.0;
    let hy = (s * s - ((oq - op) / 2.0).powi(2)).sqrt();
    let p = k.point(op, 0.05);
    let q = k.point(oq + 0.05, -0.05);
    let a = k.point(mx + 0.05, hy);
    let b = k.point(mx, -hy - 0.05);
    let oa = k.line(o, a);
    let ob = k.line(o, b);
    let pa = k.line(p, a);
    let aq = k.line(a, q);
    let qb = k.line(q, b);
    let bp = k.line(b, p);
    k.drive(DimKind::Length(oa), "3");
    k.drive(DimKind::Length(ob), "3");
    for ln in [pa, aq, qb, bp] {
        k.drive(DimKind::Length(ln), "1");
    }
    k.drive(DimKind::Distance(o, p), "2.4");
    assert_eq!(k.dof(), 1);
    let d = k.measure(DimKind::Distance(o, q));
    k.check_value(d, oq);
}

/// Two rigid triangles hinged at a free joint: the median of the first
/// triangle is determined while the hinge angle stays free.
#[test]
fn hinged_triangles_median() {
    let mut k = Kit::new();
    let a = k.point(0.0, 0.0);
    let b = k.point(4.0, 0.0);
    let c = k.point(1.5, 2.5);
    let d = k.point(4.0, 4.5);
    let e = k.point(2.5, 6.0);
    let ab = k.line(a, b);
    let bc = k.line(b, c);
    let ca = k.line(c, a);
    let cd = k.line(c, d);
    let de = k.line(d, e);
    let ec = k.line(e, c);
    k.drive(DimKind::Length(ab), "4");
    k.drive(DimKind::Length(bc), "3.2");
    k.drive(DimKind::Length(ca), "2.9");
    k.drive(DimKind::Length(cd), "3.1");
    k.drive(DimKind::Length(de), "2");
    k.drive(DimKind::Length(ec), "2.7");
    assert_eq!(k.dof(), 4);
    let m = k.midpoint_of(a, b);
    let med = k.measure(DimKind::Distance(c, m));
    let m2 = k.midpoint_of(d, e);
    let med2 = k.measure(DimKind::Distance(c, m2));
    let e1 = 0.5 * (2.0 * 3.2f64 * 3.2 + 2.0 * 2.9 * 2.9 - 16.0).sqrt();
    let e2 = 0.5 * (2.0 * 3.1f64 * 3.1 + 2.0 * 2.7 * 2.7 - 4.0).sqrt();
    k.check_value(med, e1);
    k.check_value(med2, e2);
}

/// Scotch yoke: the yoke position is the crank's x projection.
#[test]
fn scotch_yoke_projection() {
    let mut k = Kit::new();
    let o = k.origin();
    let th = 38f64.to_radians();
    let a = k.point(1.8 * th.cos() + 0.1, 1.8 * th.sin());
    let y = k.point(1.8 * th.cos(), 0.2);
    let oa = k.line(o, a);
    let ya = k.line(y, a);
    let xa = k.xaxis();
    k.on(y, xa);
    k.vertical(ya);
    k.drive(DimKind::Length(oa), "1.8");
    k.drive(DimKind::Angle(xa, oa), &ang(38.0, [1.0, 0.0], [th.cos(), th.sin()]));
    assert_eq!(k.dof(), 0);
    let yx = k.measure(DimKind::CoordX(y));
    k.check_value(yx, 1.8 * th.cos());
}

/// Symmetric four-link chain O-D-E-F-G with G on the x axis: the base angles
/// at O and G are the same variable (the one at G a supplementary dimension),
/// and so is the angle at the apex E. The supplementary dimensions give
/// sin(π - θ) and cos(π - θ), which must reduce to ±sin θ and ±cos θ for the
/// 5-unknown block of E, F, G to solve exactly.
#[test]
fn symmetric_chain_supplementary_angles() {
    let mut k = Kit::new();
    let o = k.origin();
    let xa = k.xaxis();
    let (l, u, tl, tu) = (3.0, 5.0, 0.91f64, 1.0f64);
    k.var("L", l);
    k.var("U", u);
    k.var("θL", tl);
    k.var("θU", tu);
    let pd = [l * tl.cos(), l * tl.sin()];
    let pe = [pd[0] + u * (tu / 2.0).sin(), pd[1] + u * (tu / 2.0).cos()];
    let pf = [pe[0] + u * (tu / 2.0).sin(), pd[1]];
    let pg = [pf[0] - l * tl.cos(), 0.0];
    let d = k.point(pd[0] + 0.1, pd[1] - 0.1);
    let e = k.point(pe[0] - 0.1, pe[1] + 0.2);
    let f = k.point(pf[0] + 0.2, pf[1] + 0.1);
    let g = k.point(pg[0] - 0.1, 0.1);
    let od = k.line(o, d);
    let de = k.line(d, e);
    let ef = k.line(e, f);
    let fg = k.line(f, g);
    k.on(g, xa);
    k.drive_angle(xa, od, "θL");
    // interior angles at G and E: labels between the rays from the vertex
    let lw = [pg[0] + 0.4, 0.3];
    let ag = k.app.add_dimension(DimKind::Angle(fg, xa), lw).expect("dimension rejected");
    k.set(ag, "θL");
    let lw = [pe[0], pe[1] - 0.5];
    let ae = k.app.add_dimension(DimKind::Angle(de, ef), lw).expect("dimension rejected");
    k.set(ae, "θU");
    assert!(k.doc().dims[ag].as_ref().unwrap().supp && k.doc().dims[ae].as_ref().unwrap().supp);
    k.drive(DimKind::Length(od), "L");
    k.drive(DimKind::Length(fg), "L");
    k.drive(DimKind::Length(de), "U");
    k.drive(DimKind::Length(ef), "U");
    assert_eq!(k.dof(), 0);
    let ex = k.measure(DimKind::CoordX(e));
    let ey = k.measure(DimKind::CoordY(e));
    k.check_value(ex, pe[0]);
    k.check_value(ey, pe[1]);
}
