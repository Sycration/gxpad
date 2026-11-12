//! Constructions built directly on the library internals: tangencies,
//! construction bindings and formulas they lead to. Set GXGEO_DEMO_DIR to
//! save the demo constructions as .gxp files.

use gxgeo::build::angle_vertex;
use gxgeo::formula::{Formula, Target, derive_formula};
use gxgeo::model::*;
use gxgeo::sketch::Sketch;

/// The formula of target `t`, with the time its derivation took.
fn formula(sk: &Sketch, t: Target) -> Timed {
    let t0 = std::time::Instant::now();
    let f = derive_formula(&sk.doc, t, 0).expect("derivation failed");
    Timed { ms: t0.elapsed().as_secs_f64() * 1e3, f }
}

struct Timed {
    f: Formula,
    ms: f64,
}

impl std::ops::Deref for Timed {
    type Target = Formula;
    fn deref(&self) -> &Formula {
        &self.f
    }
}

mod demo {
    use super::*;

    fn save(sk: &Sketch, name: &str) {
        if let Ok(dir) = std::env::var("GXGEO_DEMO_DIR") {
            std::fs::write(format!("{dir}/{name}.gxp"), gxgeo::gxp::to_string(&sk.doc).unwrap()).unwrap();
        }
    }

    fn line(sk: &mut Sketch, a: usize, b: usize) -> usize {
        let mut l = 0;
        assert!(sk.try_change(false, |d| l = d.add_ent(Ent::Line { a, b, infinite: false })));
        l
    }

    #[test]
    fn demo_tangent_circle() {
        let mut sk = Sketch::new();
        let c = sk.doc.add_point(3.5, 2.0);
        let mut circ = 0;
        sk.try_change(false, |d| circ = d.add_circle(c, 1.2));
        let q = sk.doc.add_point(6.0, 4.5);
        let o = sk.doc.origin;
        let l = line(&mut sk, o, q);
        let xa = sk.doc.xaxis;
        let t1 = sk.doc.tangent_con(xa, circ).unwrap();
        assert!(sk.add_constraint(t1));
        let t2 = sk.doc.tangent_con(l, circ).unwrap();
        assert!(sk.add_constraint(t2));
        sk.add_dimension(DimKind::Radius(circ), [5.0, 0.5]).unwrap();
        sk.add_dimension(DimKind::Angle(xa, l), [2.0, 0.6]).unwrap();
        sk.add_dimension(DimKind::Length(l), [2.0, 4.0]).unwrap();
        let dd = sk.add_dimension(DimKind::Distance(o, c), [1.5, -0.8]).unwrap();
        assert!(!sk.doc.dims[dd].as_ref().unwrap().driving);
        assert_eq!(sk.dof, 0);
        let f = formula(&sk, Target::Dim(dd));
        let f = &f;
        println!("{}  exact={} ({} ms)", f.plain, f.exact, f.ms);
        assert!(f.exact);
        save(&sk, "tangent");
    }

    #[test]
    fn demo_function_fit() {
        let mut sk = Sketch::new();
        sk.try_change(false, |d| {
            d.add_var("k".into(), 1.0);
            d.add_var("c".into(), 0.0);
        });
        for v in sk.doc.vars.iter().flatten() {
            let p = v.param;
            sk.doc.params[p].locked = false;
        }
        let mut f = 0;
        sk.try_change(false, |d| f = d.add_ent(Ent::Curve { src: "y = k x^2 + c".into() }));
        let p1 = sk.doc.add_point(1.0, 1.0);
        let p2 = sk.doc.add_point(2.0, 4.0);
        sk.try_change(false, |d| {
            d.cons.push(Some(Con::OnCurve(p1, f)));
            d.cons.push(Some(Con::OnCurve(p2, f)));
        });
        let o = sk.doc.origin;
        sk.add_dimension(DimKind::HDist(o, p1), [0.5, -1.0]).unwrap();
        sk.add_dimension(DimKind::VDist(o, p1), [-1.0, 0.5]).unwrap();
        sk.add_dimension(DimKind::HDist(o, p2), [1.0, -1.6]).unwrap();
        sk.add_dimension(DimKind::VDist(o, p2), [-1.6, 2.0]).unwrap();
        println!("dof={}", sk.dof);
        println!("{}", formula(&sk, Target::Var(0)).plain);
        println!("{}", formula(&sk, Target::Var(1)).plain);
        save(&sk, "function");
    }

    #[test]
    fn demo_conics() {
        let mut sk = Sketch::new();
        let c = sk.doc.add_point(0.0, 0.0);
        let o = sk.doc.origin;
        let mut e = 0;
        sk.try_change(false, |d| {
            e = d.add_conic(ConicKind::Ellipse, c, 3.0, 1.6, 0.0);
            d.cons.push(Some(Con::Coincident(c, o)));
        });
        let ka = sk.add_dimension(DimKind::ConicA(e), [1.5, 0.4]).unwrap();
        let kb = sk.add_dimension(DimKind::ConicB(e), [-0.5, 1.0]).unwrap();
        let _ = (ka, kb);
        // ellipse axis aligned
        let t = match sk.doc.kind(e) {
            Some(Ent::Conic { t, .. }) => *t,
            _ => unreachable!(),
        };
        sk.doc.params[t].locked = true;
        // a line tangent to the ellipse through a fixed point
        let p = sk.doc.add_point(5.0, 3.0);
        let q = sk.doc.add_point(0.0, 3.0);
        let l = line(&mut sk, p, q);
        sk.try_change(false, |d| d.lock_point(p, true));
        let tc = sk.doc.tangent_con(l, e).unwrap();
        assert!(sk.add_constraint(tc));
        let xa = sk.doc.xaxis;
        let ang = sk.add_dimension(DimKind::Angle(xa, l), [3.4, 2.4]).unwrap();
        sk.doc.dims[ang].as_mut().unwrap().driving = false;
        sk.dirty = true;
        sk.solve_full();
        println!("dof={}", sk.dof);
        let f = formula(&sk, Target::Dim(ang));
        let f = &f;
        println!("{}  exact={} value={} actual={} ({:.2} ms)", f.plain, f.exact, f.value, f.actual, f.ms);
        let hp = sk.doc.add_point(-6.0, -3.0);
        let pp = sk.doc.add_point(5.0, -4.0);
        sk.try_change(false, |d| {
            d.add_conic(ConicKind::Hyperbola, hp, 1.0, 0.8, 0.3);
            d.add_conic(ConicKind::Parabola, pp, 0.5, 1.0, 1.9);
            let ac = d.add_point(-5.0, 3.0);
            let a1 = d.add_point(-3.0, 3.0);
            let a2 = d.add_point(-5.0, 5.0);
            d.add_ent(Ent::Arc { c: ac, a: a1, b: a2 });
            d.add_ent(Ent::Curve { src: "x^2 y + y^3 = 2".into() });
        });
        save(&sk, "conics");
    }
}

mod construct_tests {
    use super::*;

    /// Perpendicular bisector through hidden helper, as the tool builds it.
    fn bisector(d: &mut Doc, a: usize, b: usize) -> usize {
        let (pa, pb) = (d.pval(a), d.pval(b));
        let mid = [(pa[0] + pb[0]) * 0.5, (pa[1] + pb[1]) * 0.5];
        let m = d.add_point(mid[0], mid[1]);
        d.cons.push(Some(Con::MidpointOf(m, a, b)));
        let q = d.add_helper(mid[0] - (pb[1] - pa[1]), mid[1] + (pb[0] - pa[0]));
        d.cons.push(Some(Con::Offset { q, p: m, a, b, rot: true }));
        d.add_ent(Ent::Line { a: m, b: q, infinite: true })
    }

    #[test]
    fn circumcenter_from_coordinates() {
        let mut sk = Sketch::new();
        let o = sk.doc.origin;
        let (b, c) = (sk.doc.add_point(4.0, 0.3), sk.doc.add_point(1.2, 3.1));
        let mut ls = (0, 0);
        assert!(sk.try_change(false, |d| {
            d.add_ent(Ent::Line { a: o, b, infinite: false });
            d.add_ent(Ent::Line { a: b, b: c, infinite: false });
            d.add_ent(Ent::Line { a: c, b: o, infinite: false });
            ls = (bisector(d, o, b), bisector(d, b, c));
        }));
        // intersection of the bisectors
        let x = {
            let v = angle_vertex(&sk.doc, ls.0, ls.1);
            let mut x = 0;
            assert!(sk.try_change(true, |d| {
                x = d.add_point(v[0], v[1]);
                d.cons.push(Some(Con::OnCurve(x, ls.0)));
                d.cons.push(Some(Con::OnCurve(x, ls.1)));
            }));
            x
        };
        for k in [DimKind::CoordX(b), DimKind::CoordY(b), DimKind::CoordX(c), DimKind::CoordY(c)] {
            let i = sk.add_dimension(k, [0.0, 0.0]).unwrap();
            assert!(sk.doc.dims[i].as_ref().unwrap().driving);
        }
        assert_eq!(sk.dof, 0, "fully constrained");
        // drive a coordinate to a negative value
        let yb = sk.doc.dims.iter().position(|d| d.as_ref().is_some_and(|d| d.name == "y_A")).unwrap();
        sk.set_dim_text(yb, "-0.5");
        assert!((sk.doc.pval(b)[1] + 0.5).abs() < 1e-9);
        let rx = sk.add_dimension(DimKind::CoordX(x), [0.0, 0.0]).unwrap();
        let ry = sk.add_dimension(DimKind::CoordY(x), [0.0, 0.0]).unwrap();
        assert!(!sk.doc.dims[rx].as_ref().unwrap().driving);
        for i in [rx, ry] {
            let f = formula(&sk, Target::Dim(i));
            println!("{}  exact={} value={} actual={}", f.plain, f.exact, f.value, f.actual);
            assert!(f.exact);
            assert!((f.value - f.actual).abs() < 1e-9);
        }
        // circumcenter is equidistant from all three vertices
        let pc = sk.doc.pval(x);
        let dist = |p: [f64; 2]| (p[0] - pc[0]).hypot(p[1] - pc[1]);
        let (d0, d1, d2) = (dist([0.0, 0.0]), dist(sk.doc.pval(b)), dist(sk.doc.pval(c)));
        assert!((d0 - d1).abs() < 1e-9 && (d0 - d2).abs() < 1e-9);
    }

    #[test]
    fn tangent_from_point_and_angle_bisector() {
        let mut sk = Sketch::new();
        let o = sk.doc.origin;
        let c = sk.doc.add_point(4.0, 1.0);
        let p = sk.doc.add_point(0.5, 3.0);
        let mut circ = 0;
        assert!(sk.try_change(false, |d| circ = d.add_circle(c, 1.5)));
        let t = sk.doc.add_point(3.0, 2.3);
        assert!(sk.try_change(true, |d| {
            d.add_ent(Ent::Line { a: p, b: t, infinite: false });
            d.cons.push(Some(Con::TangentAt { p, t, c: circ }));
        }));
        let (pc, pt) = (sk.doc.pval(c), sk.doc.pval(t));
        let pp = sk.doc.pval(p);
        let rad = [pt[0] - pc[0], pt[1] - pc[1]];
        let tan = [pt[0] - pp[0], pt[1] - pp[1]];
        assert!((rad[0] * tan[0] + rad[1] * tan[1]).abs() < 1e-8);
        assert!((rad[0].hypot(rad[1]) - sk.doc.circ_val(circ).1).abs() < 1e-9);
        // angle bisector of the two axes through the origin: the line y = x
        let (xa, ya) = (sk.doc.xaxis, sk.doc.yaxis);
        assert!(sk.try_change(false, |d| {
            let q = d.add_helper(1.0, 1.0);
            d.cons.push(Some(Con::Bisect { q, x: o, l1: xa, l2: ya, s1: 1.0, s2: 1.0 }));
            d.add_ent(Ent::Line { a: o, b: q, infinite: true });
        }));
        let q = sk.doc.ents.iter().rposition(|e| e.as_ref().is_some_and(|e| e.hidden)).unwrap();
        let qv = sk.doc.pval(q);
        assert!((qv[0] - qv[1]).abs() < 1e-9);
    }
}

mod tangent_contact_tests {
    use super::*;

    /// A segment from the center of one circle, ending on a second circle and
    /// tangent to it at that end point: its length is √3·r exactly.
    #[test]
    fn tangent_segment_ending_on_circle() {
        let mut sk = Sketch::new();
        let o = sk.doc.origin;
        let e = sk.doc.add_point(0.0, 0.0);
        let f = sk.doc.add_point(-1.3, 1.5);
        let h = sk.doc.add_point(0.34, 0.94);
        let a = sk.doc.add_point(0.98, 0.19);
        let (mut ce, mut cf, mut seg, mut rl) = (0, 0, 0, 0);
        assert!(sk.try_change(false, |d| {
            ce = d.add_circle(e, 1.0);
            cf = d.add_circle(f, 1.0);
            seg = d.add_ent(Ent::Line { a: f, b: h, infinite: false });
            rl = d.add_ent(Ent::Line { a: o, b: a, infinite: false });
            d.cons.push(Some(Con::Coincident(o, e)));
            d.cons.push(Some(Con::OnCurve(a, ce)));
            d.cons.push(Some(Con::Equal(cf, ce)));
            d.cons.push(Some(Con::OnCurve(h, ce)));
        }));
        let t1 = sk.doc.tangent_con(cf, ce).unwrap();
        assert!(sk.add_constraint(t1));
        let t2 = sk.doc.tangent_con(seg, ce).unwrap();
        assert!(sk.add_constraint(t2));
        let r = sk.add_dimension(DimKind::Length(rl), [0.0, 0.0]).unwrap();
        sk.set_dim_text(r, "1");
        let dd = sk.add_dimension(DimKind::Length(seg), [0.0, 0.0]).unwrap();
        assert!(!sk.doc.dims[dd].as_ref().unwrap().driving);
        let v = sk.doc.measure_value(dd);
        assert!((v - 3f64.sqrt()).abs() < 1e-10, "measured {v}");
        let fm = formula(&sk, Target::Dim(dd));
        println!("{} ({:.2} ms)", fm.plain, fm.ms);
        assert!(fm.exact);
        assert!((fm.value - 3f64.sqrt()).abs() < 1e-10);
        assert_eq!(sk.dof, 2);
    }
}

mod periodic_tests {
    use super::*;

    /// Circle through a zero of sin(x) at x = −π; horizontal segment tangent to
    /// sin(x) at its peak, ending on the circle: length π/2 + √(π² − 1).
    #[test]
    fn sine_peak_tangent_to_circle() {
        let mut sk = Sketch::new();
        let o = sk.doc.origin;
        let xa = sk.doc.xaxis;
        let (mut f, mut circ, mut seg) = (0, 0, 0);
        let dpt = sk.doc.add_point(-3.1, 0.05);
        let b = sk.doc.add_point(-2.9, 1.0);
        let c = sk.doc.add_point(1.5, 1.0);
        assert!(sk.try_change(false, |d| {
            f = d.add_ent(Ent::Curve { src: "y = sin(x)".into() });
            circ = d.add_circle(o, 3.1);
            seg = d.add_ent(Ent::Line { a: b, b: c, infinite: false });
            d.cons.push(Some(Con::OnCurve(dpt, f)));
            d.cons.push(Some(Con::OnCurve(dpt, xa)));
            d.cons.push(Some(Con::OnCurve(dpt, circ)));
            d.cons.push(Some(Con::OnCurve(c, f)));
            d.cons.push(Some(Con::OnCurve(b, circ)));
            d.cons.push(Some(Con::Horizontal(b, c)));
        }));
        let t = sk.doc.tangent_con(seg, f).unwrap();
        assert!(sk.add_constraint(t));
        assert_eq!(sk.dof, 0);
        let dd = sk.add_dimension(DimKind::Length(seg), [0.0, 0.0]).unwrap();
        let fm = formula(&sk, Target::Dim(dd));
        println!("{}", fm.plain);
        let want = std::f64::consts::FRAC_PI_2 + (std::f64::consts::PI.powi(2) - 1.0).sqrt();
        assert!(fm.exact, "{}", fm.plain);
        assert!((fm.value - want).abs() < 1e-12);
        assert!((sk.doc.measure_value(dd) - want).abs() < 1e-9);
    }
}

