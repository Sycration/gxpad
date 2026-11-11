//! Construction tools: midpoint, intersection, perpendicular / parallel lines,
//! bisectors, tangent lines and reflection. Every construction stays bound to
//! its sources through constraints.

use crate::app::*;
use crate::canvas::{nearest_on, realize};
use crate::model::*;
use eframe::egui::{Color32, Painter, Pos2, Response, Shape, Stroke, pos2};

const HIT: f32 = 9.0;

/// Builders shared by the interactive tools and the test kit. Each returns the
/// entity it creates; geometry stays bound to its sources by constraints.
impl Doc {
    pub fn build_midpoint_of_line(&mut self, l: usize) -> usize {
        let (a, b) = self.line_pts(l);
        let (pa, pb) = (self.pval(a), self.pval(b));
        let p = self.add_point((pa[0] + pb[0]) * 0.5, (pa[1] + pb[1]) * 0.5);
        self.cons.push(Some(Con::Midpoint(p, l)));
        p
    }

    pub fn build_midpoint(&mut self, a: usize, b: usize) -> usize {
        let (pa, pb) = (self.pval(a), self.pval(b));
        let p = self.add_point((pa[0] + pb[0]) * 0.5, (pa[1] + pb[1]) * 0.5);
        self.cons.push(Some(Con::MidpointOf(p, a, b)));
        p
    }

    /// Point at the crossing of curves a and b, starting from `guess`.
    pub fn build_intersection(&mut self, a: usize, b: usize, guess: [f64; 2]) -> usize {
        let p = self.add_point(guess[0], guess[1]);
        self.cons.push(Some(Con::OnCurve(p, a)));
        self.cons.push(Some(Con::OnCurve(p, b)));
        p
    }

    /// Infinite line through p, perpendicular (`rot`) or parallel to line l.
    pub fn build_dir_line(&mut self, l: usize, p: usize, rot: bool) -> usize {
        let (a, b) = self.line_pts(l);
        let (pa, pb, pp) = (self.pval(a), self.pval(b), self.pval(p));
        let dir = [pb[0] - pa[0], pb[1] - pa[1]];
        let dir = if rot { [-dir[1], dir[0]] } else { dir };
        let q = self.add_helper(pp[0] + dir[0], pp[1] + dir[1]);
        self.cons.push(Some(Con::Offset { q, p, a, b, rot }));
        self.add_ent(Ent::Line { a: p, b: q, infinite: true })
    }

    /// Perpendicular bisector of points a, b; returns (line, midpoint).
    pub fn build_perp_bisector(&mut self, a: usize, b: usize) -> (usize, usize) {
        let (pa, pb) = (self.pval(a), self.pval(b));
        let mid = [(pa[0] + pb[0]) * 0.5, (pa[1] + pb[1]) * 0.5];
        let mp = self.add_point(mid[0], mid[1]);
        self.cons.push(Some(Con::MidpointOf(mp, a, b)));
        let q = self.add_helper(mid[0] - (pb[1] - pa[1]), mid[1] + (pb[0] - pa[0]));
        self.cons.push(Some(Con::Offset { q, p: mp, a, b, rot: true }));
        (self.add_ent(Ent::Line { a: mp, b: q, infinite: true }), mp)
    }

    /// Bisector of lines l1, l2 in the sector containing `toward`; returns
    /// (line, vertex point). Uses a shared endpoint as vertex when present.
    pub fn build_angle_bisector(&mut self, l1: usize, l2: usize, toward: [f64; 2]) -> Option<(usize, usize)> {
        let shared = {
            let (a, b) = self.line_pts(l1);
            let (c, d) = self.line_pts(l2);
            [a, b].into_iter().find(|p| *p == c || *p == d)
        };
        let x = shared.map(|p| self.pval(p)).unwrap_or_else(|| angle_vertex(self, l1, l2));
        let s = sector_signs(self, l1, l2, x, toward);
        let dir = bisector_dir(self, l1, l2, s)?;
        let xp = match shared {
            Some(p) => p,
            None => self.build_intersection(l1, l2, x),
        };
        let q = self.add_helper(x[0] + dir[0], x[1] + dir[1]);
        self.cons.push(Some(Con::Bisect { q, x: xp, l1, l2, s1: s.0, s2: s.1 }));
        Some((self.add_ent(Ent::Line { a: xp, b: q, infinite: true }), xp))
    }

    /// Segment from p touching curve c at a new contact point placed at `guess`;
    /// returns (line, contact point).
    pub fn build_tangent_line(&mut self, p: usize, c: usize, guess: [f64; 2]) -> (usize, usize) {
        let t = self.add_point(guess[0], guess[1]);
        let l = self.add_ent(Ent::Line { a: p, b: t, infinite: false });
        self.cons.push(Some(Con::TangentAt { p, t, c }));
        (l, t)
    }

    /// Current frame of conic c: center (vertex of a parabola), unit axis
    /// directions e₁ (along a) and e₂, a, b and the kind.
    fn conic_val(&self, c: usize) -> Option<([f64; 2], [f64; 2], [f64; 2], f64, f64, ConicKind)> {
        let Some(Ent::Conic { kind, c: ctr, a, b, t }) = self.kind(c) else { return None };
        let t = self.params[*t].v;
        let (ct, st) = (t.cos(), t.sin());
        Some((self.pval(*ctr), [ct, st], [-st, ct], self.params[*a].v, self.params[*b].v, *kind))
    }

    /// Foci of conic c already in the document (the first one carries the
    /// focus constraint, the second is its mirror image in the center).
    pub fn foci_of(&self, c: usize) -> Vec<usize> {
        let Some(Ent::Conic { c: ctr, .. }) = self.kind(c) else { return vec![] };
        let ctr = *ctr;
        let mut out = Vec::new();
        for k in self.cons.iter().flatten() {
            if let Con::Focus { p, c: cc } = k {
                if *cc == c && self.ent(*p).is_some() {
                    out.push(*p);
                }
            }
        }
        let firsts = out.clone();
        for f in firsts {
            for k in self.cons.iter().flatten() {
                if let Con::MidpointOf(m, a, b) = *k {
                    if m == ctr && (a == f || b == f) {
                        let other = if a == f { b } else { a };
                        if self.ent(other).is_some() && !out.contains(&other) {
                            out.push(other);
                        }
                    }
                }
            }
        }
        out
    }

    /// The foci of conic c as points (two for an ellipse or hyperbola, one
    /// for a parabola), creating them when missing.
    pub fn build_foci(&mut self, c: usize) -> Vec<usize> {
        let have = self.foci_of(c);
        let want = match self.kind(c) {
            Some(Ent::Conic { kind: ConicKind::Parabola, .. }) => 1,
            Some(Ent::Conic { .. }) => 2,
            _ => return vec![],
        };
        if have.len() >= want {
            return have;
        }
        let Some((o, e1, e2, a, b, kind)) = self.conic_val(c) else { return vec![] };
        let Some(Ent::Conic { c: ctr, .. }) = self.kind(c) else { return vec![] };
        let ctr = *ctr;
        // focus offset from the center
        let off = match kind {
            ConicKind::Ellipse if a >= b => {
                let f = (a * a - b * b).sqrt();
                [f * e1[0], f * e1[1]]
            }
            ConicKind::Ellipse => {
                let f = (b * b - a * a).sqrt();
                [f * e2[0], f * e2[1]]
            }
            ConicKind::Hyperbola => {
                let f = (a * a + b * b).sqrt();
                [f * e1[0], f * e1[1]]
            }
            ConicKind::Parabola => [a * e1[0], a * e1[1]],
        };
        let f1 = match have.first() {
            Some(&f) => f,
            None => {
                let f = self.add_point(o[0] + off[0], o[1] + off[1]);
                self.cons.push(Some(Con::Focus { p: f, c }));
                f
            }
        };
        if want == 1 {
            return vec![f1];
        }
        let p1 = self.pval(f1);
        let f2 = self.add_point(2.0 * o[0] - p1[0], 2.0 * o[1] - p1[1]);
        self.cons.push(Some(Con::MidpointOf(ctr, f1, f2)));
        vec![f1, f2]
    }

    /// Directrix of conic c belonging to the focus nearest `near` (creating
    /// the foci when missing); returns the line.
    pub fn build_directrix(&mut self, c: usize, near: [f64; 2]) -> Option<usize> {
        let foci = self.build_foci(c);
        let f = *foci.iter().min_by(|a, b| {
            let d = |p: usize| {
                let q = self.pval(p);
                (q[0] - near[0]).hypot(q[1] - near[1])
            };
            d(**a).total_cmp(&d(**b))
        })?;
        if let Some(l) = self.directrix_of(c, f) {
            return Some(l);
        }
        let (o, e1, e2, a, b, kind) = self.conic_val(c)?;
        let pf = self.pval(f);
        let df = [pf[0] - o[0], pf[1] - o[1]];
        let (uf, vf) = (df[0] * e1[0] + df[1] * e1[1], df[0] * e2[0] + df[1] * e2[1]);
        // the foot of the directrix: center + s·(f − center), on the polar line of f
        let s = match kind {
            ConicKind::Ellipse => a * a * b * b / (b * b * uf * uf + a * a * vf * vf),
            ConicKind::Hyperbola => a * a * b * b / (b * b * uf * uf - a * a * vf * vf),
            ConicKind::Parabola => 2.0 * a * uf / (vf * vf - 2.0 * a * uf),
        };
        if !s.is_finite() {
            return None;
        }
        let foot = [o[0] + s * df[0], o[1] + s * df[1]];
        let p = self.add_helper(foot[0], foot[1]);
        let q = self.add_helper(foot[0] - df[1], foot[1] + df[0]);
        self.cons.push(Some(Con::Directrix { a: p, b: q, f, c }));
        Some(self.add_ent(Ent::Line { a: p, b: q, infinite: true }))
    }

    /// The directrix line of conic c belonging to focus f, when there is one.
    pub fn directrix_of(&self, c: usize, f: usize) -> Option<usize> {
        for k in self.cons.iter().flatten() {
            match *k {
                Con::DirectrixOf { l, f: ff, c: cc } if cc == c && ff == f && self.ent(l).is_some() => return Some(l),
                Con::Directrix { a, b, f: ff, c: cc } if cc == c && ff == f => {
                    let line = (0..self.ents.len()).find(|&i| matches!(self.kind(i), Some(Ent::Line { a: x, b: y, .. }) if *x == a && *y == b));
                    if line.is_some() {
                        return line;
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Circle through points a, b, c; its center is a new point (the
    /// circumcenter). Returns (circle, center); None for collinear points.
    pub fn build_circle_through(&mut self, a: usize, b: usize, c: usize) -> Option<(usize, usize)> {
        let (o, r) = circumcircle(self.pval(a), self.pval(b), self.pval(c))?;
        let ctr = self.add_point(o[0], o[1]);
        let circ = self.add_circle(ctr, r);
        for p in [a, b, c] {
            self.cons.push(Some(Con::OnCurve(p, circ)));
        }
        Some((circ, ctr))
    }

    /// Arc from a through m to b; its center is a new point. Returns
    /// (arc, center); None for collinear points.
    pub fn build_arc_through(&mut self, a: usize, m: usize, b: usize) -> Option<(usize, usize)> {
        let (pa, pm, pb) = (self.pval(a), self.pval(m), self.pval(b));
        let (o, _) = circumcircle(pa, pm, pb)?;
        // arcs run counterclockwise from their first point: order the ends so m is on the way
        let ang = |p: [f64; 2]| (p[1] - o[1]).atan2(p[0] - o[0]);
        let sweep = |from: f64, to: f64| (to - from).rem_euclid(std::f64::consts::TAU);
        let (s, e) = if sweep(ang(pa), ang(pm)) < sweep(ang(pa), ang(pb)) { (a, b) } else { (b, a) };
        let ctr = self.add_point(o[0], o[1]);
        let arc = self.add_ent(Ent::Arc { c: ctr, a: s, b: e });
        self.cons.push(Some(Con::OnCurve(m, arc)));
        Some((arc, ctr))
    }

    /// Ellipse or hyperbola with foci f1, f2 through point p; its center is
    /// a new point midway between the foci. Returns the conic; None when no
    /// such conic exists (p between the foci of a hyperbola, coincident foci).
    pub fn build_conic_by_foci(&mut self, kind: ConicKind, f1: usize, f2: usize, p: usize) -> Option<usize> {
        let (q1, q2, pp) = (self.pval(f1), self.pval(f2), self.pval(p));
        let dist = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
        let c = dist(q1, q2) / 2.0;
        let (d1, d2) = (dist(pp, q1), dist(pp, q2));
        let (a, b) = match kind {
            ConicKind::Ellipse => {
                let a = (d1 + d2) / 2.0;
                (a, (a * a - c * c).sqrt())
            }
            ConicKind::Hyperbola => {
                let a = (d1 - d2).abs() / 2.0;
                (a, (c * c - a * a).sqrt())
            }
            ConicKind::Parabola => return None,
        };
        if !(c > 1e-9 && a > 1e-9 && b > 1e-9) {
            return None;
        }
        let o = [(q1[0] + q2[0]) / 2.0, (q1[1] + q2[1]) / 2.0];
        let t = (q1[1] - q2[1]).atan2(q1[0] - q2[0]);
        let ctr = self.add_point(o[0], o[1]);
        let conic = self.add_conic(kind, ctr, a, b, t);
        self.cons.push(Some(Con::Focus { p: f1, c: conic }));
        self.cons.push(Some(Con::MidpointOf(ctr, f1, f2)));
        self.cons.push(Some(Con::OnCurve(p, conic)));
        Some(conic)
    }

    /// Parabola with focus f and directrix line l; its vertex is a new point.
    /// Returns the parabola; None when f lies on l.
    pub fn build_parabola_by_focus(&mut self, f: usize, l: usize) -> Option<usize> {
        let pf = self.pval(f);
        let foot = foot(self, pf, l);
        let (dx, dy) = (pf[0] - foot[0], pf[1] - foot[1]);
        let dist = dx.hypot(dy);
        if dist < 1e-9 {
            return None;
        }
        let v = self.add_point((pf[0] + foot[0]) / 2.0, (pf[1] + foot[1]) / 2.0);
        let par = self.add_conic(ConicKind::Parabola, v, dist / 2.0, 1.0, dy.atan2(dx));
        self.cons.push(Some(Con::Focus { p: f, c: par }));
        self.cons.push(Some(Con::DirectrixOf { l, f, c: par }));
        Some(par)
    }

    /// Both asymptotes of hyperbola c, as lines through its center.
    pub fn build_asymptotes(&mut self, c: usize) -> Vec<usize> {
        let Some((o, e1, e2, a, b, ConicKind::Hyperbola)) = self.conic_val(c) else { return vec![] };
        let Some(Ent::Conic { c: ctr, .. }) = self.kind(c) else { return vec![] };
        let ctr = *ctr;
        let mut out = Vec::new();
        for s in [1.0, -1.0] {
            let q = self.add_helper(o[0] + a * e1[0] + s * b * e2[0], o[1] + a * e1[1] + s * b * e2[1]);
            self.cons.push(Some(Con::Asymptote { q, c, s }));
            out.push(self.add_ent(Ent::Line { a: ctr, b: q, infinite: true }));
        }
        out
    }

    /// Mirror image of point p in line l.
    pub fn build_reflection(&mut self, p: usize, l: usize) -> usize {
        let pp = self.pval(p);
        let f = foot(self, pp, l);
        let q = self.add_point(2.0 * f[0] - pp[0], 2.0 * f[1] - pp[1]);
        self.cons.push(Some(Con::Symmetric(p, q, l)));
        q
    }
}

/// Center and radius of the circle through three points; None when collinear.
pub fn circumcircle(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> Option<([f64; 2], f64)> {
    let d = 2.0 * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]));
    let scale = (b[0] - a[0]).hypot(b[1] - a[1]) * (c[0] - a[0]).hypot(c[1] - a[1]);
    if !(d.abs() > 1e-9 * scale.max(1e-300)) {
        return None;
    }
    let n = |p: [f64; 2]| p[0] * p[0] + p[1] * p[1];
    let x = (n(a) * (b[1] - c[1]) + n(b) * (c[1] - a[1]) + n(c) * (a[1] - b[1])) / d;
    let y = (n(a) * (c[0] - b[0]) + n(b) * (a[0] - c[0]) + n(c) * (b[0] - a[0])) / d;
    Some(([x, y], (a[0] - x).hypot(a[1] - y)))
}

fn line_dir_of(d: &Doc, l: usize) -> [f64; 2] {
    let (a, b) = d.line_pts(l);
    let (a, b) = (d.pval(a), d.pval(b));
    [b[0] - a[0], b[1] - a[1]]
}

/// Signs selecting the angle sector of lines l1, l2 around `x` containing `w`.
pub fn sector_signs(d: &Doc, l1: usize, l2: usize, x: [f64; 2], w: [f64; 2]) -> (f64, f64) {
    let (u, v) = (line_dir_of(d, l1), line_dir_of(d, l2));
    let q = [w[0] - x[0], w[1] - x[1]];
    let cr = |a: [f64; 2], b: [f64; 2]| a[0] * b[1] - a[1] * b[0];
    for s1 in [1.0, -1.0] {
        for s2 in [1.0, -1.0] {
            let a = [u[0] * s1, u[1] * s1];
            let b = [v[0] * s2, v[1] * s2];
            let den = cr(a, b);
            if den.abs() < 1e-15 {
                continue;
            }
            if cr(q, b) / den >= 0.0 && cr(a, q) / den >= 0.0 {
                return (s1, s2);
            }
        }
    }
    (1.0, 1.0)
}

pub fn bisector_dir(d: &Doc, l1: usize, l2: usize, s: (f64, f64)) -> Option<[f64; 2]> {
    let u = unit(line_dir_of(d, l1))?;
    let v = unit(line_dir_of(d, l2))?;
    let r = [s.0 * u[0] + s.1 * v[0], s.0 * u[1] + s.1 * v[1]];
    unit(r).map(|_| r)
}

pub fn is_construct(t: Tool) -> bool {
    matches!(
        t,
        Tool::Midpoint
            | Tool::Intersect
            | Tool::PerpLine
            | Tool::ParLine
            | Tool::PerpBisector
            | Tool::AngleBisector
            | Tool::TangentLine
            | Tool::Reflect
            | Tool::Foci
            | Tool::Directrix
            | Tool::Asymptotes
    )
}

/// Builds the conic feature of `tool` (foci, the directrix on the side of
/// `near`, or asymptotes) for conic c; returns the features.
fn build_conic_feature(d: &mut Doc, tool: Tool, c: usize, near: [f64; 2]) -> Vec<usize> {
    match tool {
        Tool::Foci => d.build_foci(c),
        Tool::Directrix => d.build_directrix(c, near).into_iter().collect(),
        Tool::Asymptotes => d.build_asymptotes(c),
        _ => vec![],
    }
}

fn unit(v: [f64; 2]) -> Option<[f64; 2]> {
    let l = v[0].hypot(v[1]);
    if l < 1e-12 { None } else { Some([v[0] / l, v[1] / l]) }
}

fn seg_x(a: Pos2, b: Pos2, c: Pos2, d: Pos2) -> Option<Pos2> {
    let r = b - a;
    let s = d - c;
    let den = r.x * s.y - r.y * s.x;
    if den.abs() < 1e-9 {
        return None;
    }
    let t = ((c.x - a.x) * s.y - (c.y - a.y) * s.x) / den;
    let u = ((c.x - a.x) * r.y - (c.y - a.y) * r.x) / den;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) { Some(a + r * t) } else { None }
}

impl App {
    fn line_dir_val(&self, l: usize) -> [f64; 2] {
        let (a, b) = self.doc.line_pts(l);
        let (a, b) = (self.doc.pval(a), self.doc.pval(b));
        [b[0] - a[0], b[1] - a[1]]
    }

    fn hovered_ent(&self) -> Option<usize> {
        match self.hover {
            Some(Sel::Ent(e)) if self.doc.ent(e).is_some_and(|x| !x.hidden) => Some(e),
            _ => None,
        }
    }

    fn is_segment(&self, e: usize) -> bool {
        matches!(self.doc.kind(e), Some(Ent::Line { infinite: false, .. }))
    }

    /// Curves (non-point entities) passing near a screen position, nearest first.
    fn curves_near(&self, m: Pos2) -> Vec<usize> {
        let mut v: Vec<(f32, usize)> = self
            .shapes
            .iter()
            .filter(|(e, _)| !self.doc.is_point(**e))
            .map(|(e, p)| (nearest_on(p, m).0, *e))
            .filter(|(d, _)| *d < HIT)
            .collect();
        v.sort_by(|a, b| a.0.total_cmp(&b.0));
        v.into_iter().map(|(_, e)| e).collect()
    }

    /// Crossing of two curves closest to `m`, in world coordinates.
    fn crossing_near(&self, a: usize, b: usize, m: Pos2) -> Option<[f64; 2]> {
        let (pa, pb) = (self.shapes.get(&a)?, self.shapes.get(&b)?);
        let mut best: Option<(f32, Pos2)> = None;
        for la in pa {
            for wa in la.windows(2) {
                for lb in pb {
                    for wb in lb.windows(2) {
                        if let Some(q) = seg_x(wa[0], wa[1], wb[0], wb[1]) {
                            let d = q.distance(m);
                            if best.is_none_or(|b| d < b.0) {
                                best = Some((d, q));
                            }
                        }
                    }
                }
            }
        }
        best.filter(|b| b.0 < 60.0).map(|b| self.view.to_w(b.1))
    }

    /// Point of curve `c` where a line from `p` touches it, nearest to `m`.
    fn tangent_contact(&self, p: [f64; 2], c: usize, m: Pos2) -> Option<[f64; 2]> {
        let ps = self.view.to_s(p);
        let polys = self.shapes.get(&c)?;
        let mut best: Option<(f32, Pos2)> = None;
        for pl in polys {
            if pl.len() < 3 {
                continue;
            }
            let f = |i: usize| {
                let t = pl[(i + 1).min(pl.len() - 1)] - pl[i.saturating_sub(1)];
                let q = pl[i] - ps;
                t.x * q.y - t.y * q.x
            };
            let mut prev = f(0);
            for i in 1..pl.len() {
                let cur = f(i);
                if (prev < 0.0) != (cur < 0.0) {
                    let q = pl[i];
                    let d = q.distance(m);
                    if best.is_none_or(|b| d < b.0) {
                        best = Some((d, q));
                    }
                }
                prev = cur;
            }
        }
        best.map(|b| self.view.to_w(b.1))
    }

    fn sector_signs(&self, l1: usize, l2: usize, x: [f64; 2], w: [f64; 2]) -> (f64, f64) {
        sector_signs(&self.doc, l1, l2, x, w)
    }

    fn shared_point(&self, l1: usize, l2: usize) -> Option<usize> {
        let (a, b) = self.doc.line_pts(l1);
        let (c, d) = self.doc.line_pts(l2);
        [a, b].into_iter().find(|p| *p == c || *p == d)
    }

    fn bisector_dir(&self, l1: usize, l2: usize, s: (f64, f64)) -> Option<[f64; 2]> {
        bisector_dir(&self.doc, l1, l2, s)
    }

    /// True when entity c can take the conic feature of the current tool.
    fn conic_target(&self, c: usize) -> bool {
        match self.doc.kind(c) {
            Some(Ent::Conic { kind, .. }) => self.tool != Tool::Asymptotes || *kind == ConicKind::Hyperbola,
            _ => false,
        }
    }

    fn construct_done(&mut self, ok: bool, ents: Vec<usize>) {
        if !ok {
            self.flash_ents(ents);
        }
        self.pending.clear();
        self.picks.clear();
    }

    pub fn construct_input(&mut self, resp: &Response, mouse: Option<Pos2>) {
        let Some(m) = mouse else { return };
        if !resp.clicked() {
            return;
        }
        let hov = self.hovered_ent();
        let snap = self.snap_at_pub(m);
        let mw = self.view.to_w(m);
        match self.tool {
            Tool::Midpoint => {
                if self.pending.is_empty() {
                    if let Some(l) = hov.filter(|l| self.is_segment(*l)) {
                        let ok = self.try_change(false, |d| {
                            d.build_midpoint_of_line(l);
                        });
                        return self.construct_done(ok, vec![l]);
                    }
                }
                self.pending.push(snap);
                if self.pending.len() == 2 {
                    let (s0, s1) = (self.pending[0], self.pending[1]);
                    let ok = self.try_change(false, |d| {
                        let a = realize(d, s0);
                        let b = realize(d, s1);
                        if a != b {
                            d.build_midpoint(a, b);
                        }
                    });
                    self.construct_done(ok, vec![]);
                }
            }
            Tool::Intersect => {
                let near = self.curves_near(m);
                let pair = match self.picks.as_slice() {
                    [] if near.len() >= 2 => Some((near[0], near[1])),
                    [a] => near.iter().copied().find(|e| e != a).map(|b| (*a, b)),
                    _ => None,
                };
                let Some((a, b)) = pair else {
                    if let Some(e) = near.first() {
                        self.picks = vec![*e];
                    }
                    return;
                };
                let Some(x) = self.crossing_near(a, b, m) else {
                    return self.construct_done(false, vec![a, b]);
                };
                let ok = self.try_change(true, |d| {
                    d.build_intersection(a, b, x);
                });
                self.construct_done(ok, vec![a, b]);
            }
            Tool::PerpLine | Tool::ParLine => {
                if self.picks.is_empty() {
                    if let Some(l) = hov.filter(|l| self.doc.is_line(*l)) {
                        self.picks = vec![l];
                    }
                    return;
                }
                let l = self.picks[0];
                let rot = self.tool == Tool::PerpLine;
                let ok = self.try_change(false, |d| {
                    let p = realize(d, snap);
                    d.build_dir_line(l, p, rot);
                });
                self.construct_done(ok, vec![l]);
            }
            Tool::PerpBisector => {
                let mut ends: Option<(Snap, Snap)> = None;
                if self.pending.is_empty() {
                    if let Some(l) = hov.filter(|l| self.is_segment(*l)) {
                        let (a, b) = self.doc.line_pts(l);
                        ends = Some((Snap::Point(a), Snap::Point(b)));
                    }
                }
                if ends.is_none() {
                    self.pending.push(snap);
                    if self.pending.len() < 2 {
                        return;
                    }
                    ends = Some((self.pending[0], self.pending[1]));
                }
                let (s0, s1) = ends.unwrap();
                let ok = self.try_change(false, |d| {
                    let a = realize(d, s0);
                    let b = realize(d, s1);
                    if a != b {
                        d.build_perp_bisector(a, b);
                    }
                });
                self.construct_done(ok, vec![]);
            }
            Tool::AngleBisector => {
                if self.picks.len() < 2 {
                    if let Some(l) = hov.filter(|l| self.doc.is_line(*l) && !self.picks.contains(l)) {
                        self.picks.push(l);
                    }
                    return;
                }
                let (l1, l2) = (self.picks[0], self.picks[1]);
                let mut built = false;
                let ok = self.try_change(false, |d| {
                    built = d.build_angle_bisector(l1, l2, mw).is_some();
                }) && built;
                self.construct_done(ok, vec![l1, l2]);
            }
            Tool::TangentLine => {
                if self.pending.is_empty() {
                    self.pending.push(snap);
                    return;
                }
                let Some(c) = hov.filter(|c| !self.doc.is_point(*c) && !self.doc.is_line(*c)) else { return };
                let s0 = self.pending[0];
                let p0 = self.snap_pos(&s0);
                let Some(t) = self.tangent_contact(p0, c, m) else {
                    return self.construct_done(false, vec![c]);
                };
                let ok = self.try_change(true, |d| {
                    let p = realize(d, s0);
                    d.build_tangent_line(p, c, t);
                });
                self.construct_done(ok, vec![c]);
            }
            Tool::Reflect => {
                if self.pending.is_empty() {
                    self.pending.push(snap);
                    return;
                }
                let Some(l) = hov.filter(|l| self.doc.is_line(*l)) else { return };
                let s0 = self.pending[0];
                let ok = self.try_change(true, |d| {
                    let p = realize(d, s0);
                    d.build_reflection(p, l);
                });
                self.construct_done(ok, vec![l]);
            }
            Tool::Foci | Tool::Directrix | Tool::Asymptotes => {
                let Some(c) = hov.filter(|c| self.conic_target(*c)) else {
                    if let Some(e) = hov {
                        self.construct_done(false, vec![e]);
                    }
                    return;
                };
                let tool = self.tool;
                let ok = self.try_change(true, |d| {
                    build_conic_feature(d, tool, c, mw);
                });
                self.construct_done(ok, vec![c]);
            }
            _ => {}
        }
    }

    pub fn paint_construct_preview(&self, p: &Painter, m: Pos2) {
        let ghost = Stroke::new(1.5, ACCENT.gamma_multiply(0.7));
        let v = self.view;
        let mw = v.to_w(m);
        let sp = self.snap_pos(&self.snap_at_pub(m));
        let pend: Vec<[f64; 2]> = self.pending.iter().map(|s| self.snap_pos(s)).collect();
        let far = (v.rect.width().hypot(v.rect.height()) as f64) / v.s;
        let inf_line = |o: [f64; 2], d: [f64; 2]| {
            if let Some(u) = unit(d) {
                let a = v.to_s([o[0] - u[0] * far, o[1] - u[1] * far]);
                let b = v.to_s([o[0] + u[0] * far, o[1] + u[1] * far]);
                p.extend(Shape::dashed_line(&[a, b], ghost, 6.0, 4.0));
            }
        };
        let ghost_pt = |w: [f64; 2]| {
            let q = v.to_s(w);
            p.circle_stroke(q, 5.0, Stroke::new(1.5, ACCENT));
            p.circle_filled(q, 2.0, ACCENT);
        };
        let hov = self.hovered_ent();
        match self.tool {
            Tool::Midpoint => {
                if let [a] = pend.as_slice() {
                    p.extend(Shape::dashed_line(&[v.to_s(*a), v.to_s(sp)], Stroke::new(1.0, ACCENT.gamma_multiply(0.4)), 4.0, 4.0));
                    ghost_pt([(a[0] + sp[0]) * 0.5, (a[1] + sp[1]) * 0.5]);
                } else if let Some(l) = hov.filter(|l| self.is_segment(*l)) {
                    let (a, b) = self.doc.line_pts(l);
                    let (a, b) = (self.doc.pval(a), self.doc.pval(b));
                    ghost_pt([(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5]);
                }
            }
            Tool::Intersect => {
                let near = self.curves_near(m);
                let pair = match self.picks.as_slice() {
                    [] if near.len() >= 2 => Some((near[0], near[1])),
                    [a] => near.iter().copied().find(|e| e != a).map(|b| (*a, b)),
                    _ => None,
                };
                if let Some((a, b)) = pair {
                    if let Some(x) = self.crossing_near(a, b, m) {
                        ghost_pt(x);
                    }
                }
            }
            Tool::PerpLine | Tool::ParLine => {
                if let Some(&l) = self.picks.first() {
                    let d = self.line_dir_val(l);
                    let d = if self.tool == Tool::PerpLine { [-d[1], d[0]] } else { d };
                    inf_line(sp, d);
                }
            }
            Tool::PerpBisector => {
                let ends = match pend.as_slice() {
                    [a] => Some((*a, sp)),
                    _ => hov.filter(|l| self.is_segment(*l)).map(|l| {
                        let (a, b) = self.doc.line_pts(l);
                        (self.doc.pval(a), self.doc.pval(b))
                    }),
                };
                if let Some((a, b)) = ends {
                    inf_line([(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5], [-(b[1] - a[1]), b[0] - a[0]]);
                }
            }
            Tool::AngleBisector => {
                if let [l1, l2] = self.picks.as_slice() {
                    let x = self.shared_point(*l1, *l2).map(|p| self.doc.pval(p)).unwrap_or_else(|| angle_vertex(&self.doc, *l1, *l2));
                    let s = self.sector_signs(*l1, *l2, x, mw);
                    if let Some(d) = self.bisector_dir(*l1, *l2, s) {
                        inf_line(x, d);
                        ghost_pt(x);
                    }
                }
            }
            Tool::TangentLine => {
                if let [a] = pend.as_slice() {
                    let end = hov
                        .filter(|c| !self.doc.is_point(*c) && !self.doc.is_line(*c))
                        .and_then(|c| self.tangent_contact(*a, c, m))
                        .unwrap_or(mw);
                    p.line_segment([v.to_s(*a), v.to_s(end)], ghost);
                    ghost_pt(end);
                }
            }
            Tool::Foci | Tool::Directrix | Tool::Asymptotes => {
                if let Some(c) = hov.filter(|c| self.conic_target(*c)) {
                    // build on a copy: the ghost is exactly what a click creates
                    let mut d = self.doc.clone();
                    let n0 = d.ents.len();
                    let made = build_conic_feature(&mut d, self.tool, c, mw);
                    for e in made.into_iter().chain(n0..d.ents.len()) {
                        match d.ent(e) {
                            Some(Entity { kind: Ent::Point { .. }, hidden: false, .. }) => ghost_pt(d.pval(e)),
                            Some(Entity { kind: Ent::Line { a, b, .. }, .. }) => {
                                let (pa, pb) = (d.pval(*a), d.pval(*b));
                                inf_line(pa, [pb[0] - pa[0], pb[1] - pa[1]]);
                            }
                            _ => {}
                        }
                    }
                }
            }
            Tool::Reflect => {
                if let [a] = pend.as_slice() {
                    if let Some(l) = hov.filter(|l| self.doc.is_line(*l)) {
                        let f = foot(&self.doc, *a, l);
                        let q = [2.0 * f[0] - a[0], 2.0 * f[1] - a[1]];
                        p.extend(Shape::dashed_line(&[v.to_s(*a), v.to_s(q)], Stroke::new(1.0, ACCENT.gamma_multiply(0.4)), 4.0, 4.0));
                        ghost_pt(q);
                    }
                }
            }
            _ => {}
        }
        for q in &pend {
            p.circle_filled(v.to_s(*q), 3.5, ACCENT);
        }
        let _ = (pos2(0.0, 0.0), Color32::TRANSPARENT);
    }
}
