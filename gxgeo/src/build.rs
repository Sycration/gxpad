//! Construction builders: midpoint, intersection, perpendicular / parallel
//! lines, bisectors, tangent lines, conic features and reflection. Every
//! construction stays bound to its sources through constraints.

use crate::model::*;

/// A point argument of a construction: an existing point, or one to create.
#[derive(Clone, Copy, Debug)]
pub enum Pt {
    /// The existing point.
    Ent(usize),
    /// A new free point at the position.
    Free([f64; 2]),
    /// A new point at the position, on the curve.
    On(usize, [f64; 2]),
}

impl From<usize> for Pt {
    fn from(e: usize) -> Pt {
        Pt::Ent(e)
    }
}

/// The conic features the interfaces build.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConicFeature {
    /// The foci.
    Foci,
    /// The directrix belonging to the focus nearest a position.
    Directrix,
    /// Both asymptotes of a hyperbola.
    Asymptotes,
}

impl Doc {
    /// Creates (or reuses) the point `p` refers to.
    pub fn realize(&mut self, p: Pt) -> usize {
        match p {
            Pt::Ent(e) => e,
            Pt::Free(q) => self.add_point(q[0], q[1]),
            Pt::On(c, q) => {
                let e = self.add_point(q[0], q[1]);
                self.cons.push(Some(Con::OnCurve(e, c)));
                e
            }
        }
    }

    /// Current position of the point `p` refers to.
    pub fn pt_pos(&self, p: Pt) -> [f64; 2] {
        match p {
            Pt::Ent(e) => self.pval(e),
            Pt::Free(q) | Pt::On(_, q) => q,
        }
    }

    /// The conic feature `f` of conic c (for a directrix, the one of the
    /// focus nearest `near`); returns the features.
    pub fn build_conic_feature(&mut self, f: ConicFeature, c: usize, near: [f64; 2]) -> Vec<usize> {
        match f {
            ConicFeature::Foci => self.build_foci(c),
            ConicFeature::Directrix => self.build_directrix(c, near).into_iter().collect(),
            ConicFeature::Asymptotes => self.build_asymptotes(c),
        }
    }

    /// Direction of line l from its first to its second point.
    pub fn line_dir_val(&self, l: usize) -> [f64; 2] {
        line_dir_of(self, l)
    }

    /// The endpoint lines l1 and l2 have in common, when there is one.
    pub fn shared_point(&self, l1: usize, l2: usize) -> Option<usize> {
        let (a, b) = self.line_pts(l1);
        let (c, d) = self.line_pts(l2);
        [a, b].into_iter().find(|p| *p == c || *p == d)
    }
}

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
        let shared = self.shared_point(l1, l2);
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

    /// Circle around point c through position p (radius at least 1e-3).
    pub fn build_circle_to(&mut self, c: usize, p: [f64; 2]) -> usize {
        let pc = self.pval(c);
        let r = (p[0] - pc[0]).hypot(p[1] - pc[1]).max(1e-3);
        self.add_circle(c, r)
    }

    /// New point at the radius |ca| around c, in the direction of `toward`:
    /// the end of an arc around c starting at a.
    pub fn build_arc_end(&mut self, c: usize, a: usize, toward: [f64; 2]) -> usize {
        let (ca, pa) = (self.pval(c), self.pval(a));
        let r = (pa[0] - ca[0]).hypot(pa[1] - ca[1]);
        let ang = (toward[1] - ca[1]).atan2(toward[0] - ca[0]);
        self.add_point(ca[0] + r * ang.cos(), ca[1] + r * ang.sin())
    }

    /// Ellipse or hyperbola centered on point c, with its first axis ending
    /// at `axis` and its second half-axis reaching the line through `side`
    /// parallel to the first axis.
    pub fn build_conic_from(&mut self, kind: ConicKind, c: usize, axis: [f64; 2], side: [f64; 2]) -> usize {
        let cc = self.pval(c);
        let (dx, dy) = (axis[0] - cc[0], axis[1] - cc[1]);
        let a = dx.hypot(dy).max(1e-3);
        let t = dy.atan2(dx);
        let b = ((-(side[0] - cc[0]) * t.sin() + (side[1] - cc[1]) * t.cos()).abs()).max(a * 0.05);
        self.add_conic(kind, c, a, b, t)
    }

    /// Parabola with vertex at point c opening toward `axis`, its focal
    /// distance |c axis|.
    pub fn build_parabola_toward(&mut self, c: usize, axis: [f64; 2]) -> usize {
        let cc = self.pval(c);
        let (dx, dy) = (axis[0] - cc[0], axis[1] - cc[1]);
        self.add_conic(ConicKind::Parabola, c, dx.hypot(dy).max(1e-3), 1.0, dy.atan2(dx))
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

impl Doc {
    pub fn center_of_val(&self, e: usize) -> [f64; 2] {
        match self.kind(e) {
            Some(Ent::Point { .. }) => self.pval(e),
            Some(Ent::Circle { c, .. }) | Some(Ent::Arc { c, .. }) | Some(Ent::Conic { c, .. }) => self.pval(*c),
            Some(Ent::Line { a, b, .. }) => {
                let (pa, pb) = (self.pval(*a), self.pval(*b));
                [(pa[0] + pb[0]) * 0.5, (pa[1] + pb[1]) * 0.5]
            }
            _ => [0.0, 0.0],
        }
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

pub fn unit(v: [f64; 2]) -> Option<[f64; 2]> {
    let l = v[0].hypot(v[1]);
    if l < 1e-12 { None } else { Some([v[0] / l, v[1] / l]) }
}

pub fn foot(d: &Doc, p: [f64; 2], l: usize) -> [f64; 2] {
    let (a, b) = d.line_pts(l);
    let (a, b) = (d.pval(a), d.pval(b));
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    if l2 < 1e-18 {
        return a;
    }
    let t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2;
    [a[0] + t * dx, a[1] + t * dy]
}

/// Whether a label at `w` sits in a sector measuring the supplementary angle
/// of the directed lines.
pub fn angle_supp(d: &Doc, l1: usize, l2: usize, w: [f64; 2]) -> bool {
    let x = angle_vertex(d, l1, l2);
    let dir = |l: usize| {
        let (a, b) = d.line_pts(l);
        let (a, b) = (d.pval(a), d.pval(b));
        [b[0] - a[0], b[1] - a[1]]
    };
    let (u, v) = (dir(l1), dir(l2));
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
            let ka = cr(q, b) / den;
            let kb = cr(a, q) / den;
            if ka >= 0.0 && kb >= 0.0 {
                return s1 * s2 < 0.0;
            }
        }
    }
    false
}

/// Intersection of two lines, or the midpoint of their midpoints when parallel.
pub fn angle_vertex(d: &Doc, l1: usize, l2: usize) -> [f64; 2] {
    let (a, b) = d.line_pts(l1);
    let (c, e) = d.line_pts(l2);
    let (a, b, c, e) = (d.pval(a), d.pval(b), d.pval(c), d.pval(e));
    let r = [b[0] - a[0], b[1] - a[1]];
    let s = [e[0] - c[0], e[1] - c[1]];
    let den = r[0] * s[1] - r[1] * s[0];
    if den.abs() < 1e-12 {
        return [(a[0] + b[0] + c[0] + e[0]) * 0.25, (a[1] + b[1] + c[1] + e[1]) * 0.25];
    }
    let t = ((c[0] - a[0]) * s[1] - (c[1] - a[1]) * s[0]) / den;
    [a[0] + t * r[0], a[1] + t * r[1]]
}
