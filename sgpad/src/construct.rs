//! Construction tools: midpoint, intersection, perpendicular / parallel lines,
//! bisectors, tangent lines and reflection. Every construction stays bound to
//! its sources through constraints.

use crate::app::*;
use crate::canvas::nearest_on;
use crate::geo::{self, unit};
use crate::ops::{self, ConicFeature, ConicKind, EntKind};
use eframe::egui::{Painter, Pos2, Response, Shape, Stroke};

const HIT: f32 = 9.0;

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

/// The conic feature the tool builds.
fn feature_of(tool: Tool) -> Option<ConicFeature> {
    match tool {
        Tool::Foci => Some(ConicFeature::Foci),
        Tool::Directrix => Some(ConicFeature::Directrix),
        Tool::Asymptotes => Some(ConicFeature::Asymptotes),
        _ => None,
    }
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
    fn hovered_ent(&self) -> Option<usize> {
        match self.hover {
            Some(Sel::Ent(e)) if !geo::hidden(&self.geo, e) => Some(e),
            _ => None,
        }
    }

    fn is_segment(&self, e: usize) -> bool {
        geo::is_segment(&self.geo, e)
    }

    /// Curves (non-point entities) passing near a screen position, nearest first.
    fn curves_near(&self, m: Pos2) -> Vec<usize> {
        let mut v: Vec<(f32, usize)> = self
            .shapes
            .iter()
            .filter(|(e, _)| !geo::is_point(&self.geo, **e))
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

    /// True when entity c can take the conic feature of the current tool.
    fn conic_target(&self, c: usize) -> bool {
        match geo::conic(&self.geo, c) {
            Some((kind, ..)) => self.tool != Tool::Asymptotes || kind == ConicKind::Hyperbola,
            None => false,
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
                        let ok = ops::line_midpoint(&mut self.geo, l).is_ok();
                        return self.construct_done(ok, vec![l]);
                    }
                }
                self.pending.push(snap);
                if self.pending.len() == 2 {
                    let (s0, s1) = (self.pending[0], self.pending[1]);
                    let ok = ops::midpoint(&mut self.geo, s0, s1).is_ok();
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
                let ok = ops::intersection(&mut self.geo, a, b, x).is_ok();
                self.construct_done(ok, vec![a, b]);
            }
            Tool::PerpLine | Tool::ParLine => {
                if self.picks.is_empty() {
                    if let Some(l) = hov.filter(|l| geo::is_line(&self.geo, *l)) {
                        self.picks = vec![l];
                    }
                    return;
                }
                let l = self.picks[0];
                let rot = self.tool == Tool::PerpLine;
                let ok = ops::dir_line(&mut self.geo, l, snap, rot).is_ok();
                self.construct_done(ok, vec![l]);
            }
            Tool::PerpBisector => {
                let mut ends: Option<(Snap, Snap)> = None;
                if self.pending.is_empty() {
                    if let Some(l) = hov.filter(|l| self.is_segment(*l)) {
                        let (a, b) = geo::line_pts(&self.geo, l);
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
                let ok = ops::perp_bisector(&mut self.geo, s0, s1).is_ok();
                self.construct_done(ok, vec![]);
            }
            Tool::AngleBisector => {
                if self.picks.len() < 2 {
                    if let Some(l) = hov.filter(|l| geo::is_line(&self.geo, *l) && !self.picks.contains(l)) {
                        self.picks.push(l);
                    }
                    return;
                }
                let (l1, l2) = (self.picks[0], self.picks[1]);
                let ok = ops::angle_bisector(&mut self.geo, l1, l2, mw).is_ok();
                self.construct_done(ok, vec![l1, l2]);
            }
            Tool::TangentLine => {
                if self.pending.is_empty() {
                    self.pending.push(snap);
                    return;
                }
                let Some(c) = hov.filter(|c| geo::is_curve(&self.geo, *c) && !geo::is_line(&self.geo, *c)) else { return };
                let s0 = self.pending[0];
                let p0 = self.snap_pos(&s0);
                let Some(t) = self.tangent_contact(p0, c, m) else {
                    return self.construct_done(false, vec![c]);
                };
                let ok = ops::tangent_line(&mut self.geo, s0, c, t).is_ok();
                self.construct_done(ok, vec![c]);
            }
            Tool::Reflect => {
                if self.pending.is_empty() {
                    self.pending.push(snap);
                    return;
                }
                let Some(l) = hov.filter(|l| geo::is_line(&self.geo, *l)) else { return };
                let s0 = self.pending[0];
                let ok = ops::reflection(&mut self.geo, s0, l).is_ok();
                self.construct_done(ok, vec![l]);
            }
            Tool::Foci | Tool::Directrix | Tool::Asymptotes => {
                let Some(c) = hov.filter(|c| self.conic_target(*c)) else {
                    if let Some(e) = hov {
                        self.construct_done(false, vec![e]);
                    }
                    return;
                };
                let ok = feature_of(self.tool).is_some_and(|f| ops::conic_feature(&mut self.geo, f, c, mw).is_ok());
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
                    let (a, b) = geo::line_ends(&self.geo, l);
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
                    let d = geo::line_dir(&self.geo, l);
                    let d = if self.tool == Tool::PerpLine { [-d[1], d[0]] } else { d };
                    inf_line(sp, d);
                }
            }
            Tool::PerpBisector => {
                let ends = match pend.as_slice() {
                    [a] => Some((*a, sp)),
                    _ => hov.filter(|l| self.is_segment(*l)).map(|l| geo::line_ends(&self.geo, l)),
                };
                if let Some((a, b)) = ends {
                    inf_line([(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5], [-(b[1] - a[1]), b[0] - a[0]]);
                }
            }
            Tool::AngleBisector => {
                if let [l1, l2] = self.picks.as_slice() {
                    // build on a copy: the ghost is exactly what a click creates
                    let mut d = ops::copy(&self.geo);
                    if let Ok((l, x)) = ops::angle_bisector(&mut d, *l1, *l2, mw) {
                        let x = geo::pos(&d, x);
                        inf_line(x, geo::line_dir(&d, l));
                        ghost_pt(x);
                    }
                }
            }
            Tool::TangentLine => {
                if let [a] = pend.as_slice() {
                    let end = hov
                        .filter(|c| geo::is_curve(&self.geo, *c) && !geo::is_line(&self.geo, *c))
                        .and_then(|c| self.tangent_contact(*a, c, m))
                        .unwrap_or(mw);
                    p.line_segment([v.to_s(*a), v.to_s(end)], ghost);
                    ghost_pt(end);
                }
            }
            Tool::Foci | Tool::Directrix | Tool::Asymptotes => {
                if let Some(c) = hov.filter(|c| self.conic_target(*c)) {
                    // build on a copy: the ghost is exactly what a click creates
                    let mut d = ops::copy(&self.geo);
                    let n0 = ops::ent_count(&d);
                    let made = feature_of(self.tool).and_then(|f| ops::conic_feature(&mut d, f, c, mw).ok()).unwrap_or_default();
                    for e in made.into_iter().chain(n0..ops::ent_count(&d)) {
                        match geo::kind(&d, e) {
                            Some(EntKind::Point) if !geo::hidden(&d, e) => ghost_pt(geo::pos(&d, e)),
                            Some(EntKind::Line) => {
                                let (pa, pb) = geo::line_ends(&d, e);
                                inf_line(pa, [pb[0] - pa[0], pb[1] - pa[1]]);
                            }
                            _ => {}
                        }
                    }
                }
            }
            Tool::Reflect => {
                if let [a] = pend.as_slice() {
                    if let Some(l) = hov.filter(|l| geo::is_line(&self.geo, *l)) {
                        let f = geo::line_foot(&self.geo, *a, l);
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
    }
}
