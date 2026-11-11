//! Polylines for drawing entities inside a window of the plane: the same
//! tessellation gxpad draws, in world coordinates.

use crate::expr::{Code, X_SYM, Y_SYM};
use crate::model::*;
use std::collections::{HashMap, VecDeque};
use std::f64::consts::TAU;

/// Largest window side, in pixels, that [`Window::new`] accepts.
pub const MAX_PIXELS: f64 = 20_000.0;

/// A rectangle of the plane and the size of one pixel in it. Infinite
/// lines, hyperbolas and parabolas are drawn far enough to cross it, and
/// function curves are sampled inside it at a resolution set by `px`.
#[derive(Clone, Copy, Debug)]
pub struct Window {
    /// Corner with the smallest coordinates.
    pub min: [f64; 2],
    /// Corner with the largest coordinates.
    pub max: [f64; 2],
    /// World units per pixel.
    pub px: f64,
}

impl Window {
    /// Window from (x0, y0) to (x1, y1) with pixels of size `px`; an error
    /// when the corners are not ordered, a value is not finite, or the
    /// window is wider or taller than [`MAX_PIXELS`] pixels.
    pub fn new(min: [f64; 2], max: [f64; 2], px: f64) -> Result<Window, String> {
        if !min.iter().chain(max.iter()).chain([px].iter()).all(|v| v.is_finite()) {
            return Err("the window must be finite".into());
        }
        if !(max[0] > min[0] && max[1] > min[1]) {
            return Err("the window corners must be ordered (x0 < x1, y0 < y1)".into());
        }
        if px <= 0.0 {
            return Err("the pixel size must be positive".into());
        }
        let w = Window { min, max, px };
        if w.width_px() > MAX_PIXELS || w.height_px() > MAX_PIXELS {
            return Err(format!("the window is more than {MAX_PIXELS} pixels wide or tall"));
        }
        Ok(w)
    }

    fn center(&self) -> [f64; 2] {
        [(self.min[0] + self.max[0]) * 0.5, (self.min[1] + self.max[1]) * 0.5]
    }

    /// Half the diagonal, in world units.
    fn radius(&self) -> f64 {
        (self.max[0] - self.min[0]).hypot(self.max[1] - self.min[1]) * 0.5
    }

    fn width_px(&self) -> f64 {
        (self.max[0] - self.min[0]) / self.px
    }

    fn height_px(&self) -> f64 {
        (self.max[1] - self.min[1]) / self.px
    }
}

/// Polylines drawing entity `i` in window `w`: one point for a point, the
/// two endpoints for a segment, a segment crossing the window for an
/// infinite line, the sampled outline for a circle, arc or conic (two
/// polylines for a hyperbola), and the parts inside the window for a
/// function curve. Empty for a missing entity.
pub fn draw_polys(doc: &Doc, i: usize, w: &Window) -> Vec<Vec<[f64; 2]>> {
    let Some(e) = doc.ent(i) else { return vec![] };
    let s = 1.0 / w.px;
    let vc = w.center();
    let vr = w.radius();
    match &e.kind {
        Ent::Point { .. } => vec![vec![doc.pval(i)]],
        Ent::Line { a, b, infinite } => {
            let (pa, pb) = (doc.pval(*a), doc.pval(*b));
            if *infinite {
                let d = [pb[0] - pa[0], pb[1] - pa[1]];
                let l = d[0].hypot(d[1]).max(1e-12);
                let u = [d[0] / l, d[1] / l];
                let t = (vc[0] - pa[0]) * u[0] + (vc[1] - pa[1]) * u[1];
                let f = [pa[0] + u[0] * t, pa[1] + u[1] * t];
                let ext = vr * 1.5;
                vec![vec![[f[0] - u[0] * ext, f[1] - u[1] * ext], [f[0] + u[0] * ext, f[1] + u[1] * ext]]]
            } else {
                vec![vec![pa, pb]]
            }
        }
        Ent::Circle { .. } => {
            let (c, r) = doc.circ_val(i);
            let n = ((r * s * 0.6) as usize).clamp(48, 720);
            vec![(0..=n).map(|k| {
                let t = TAU * k as f64 / n as f64;
                [c[0] + r * t.cos(), c[1] + r * t.sin()]
            }).collect()]
        }
        Ent::Arc { c, a, b } => {
            let (cc, pa, pb) = (doc.pval(*c), doc.pval(*a), doc.pval(*b));
            let r = (pa[0] - cc[0]).hypot(pa[1] - cc[1]);
            let a0 = (pa[1] - cc[1]).atan2(pa[0] - cc[0]);
            let mut a1 = (pb[1] - cc[1]).atan2(pb[0] - cc[0]);
            while a1 <= a0 {
                a1 += TAU;
            }
            let n = (((a1 - a0) * r * s * 0.6) as usize).clamp(16, 720);
            vec![(0..=n).map(|k| {
                let t = a0 + (a1 - a0) * k as f64 / n as f64;
                [cc[0] + r * t.cos(), cc[1] + r * t.sin()]
            }).collect()]
        }
        Ent::Conic { kind, c, a, b, t } => {
            let cc = doc.pval(*c);
            let (a, b, t) = (doc.params[*a].v, doc.params[*b].v, doc.params[*t].v);
            let (ct, st) = (t.cos(), t.sin());
            let tr = |u: f64, v: f64| [cc[0] + u * ct - v * st, cc[1] + u * st + v * ct];
            let far = vr + (cc[0] - vc[0]).hypot(cc[1] - vc[1]);
            match kind {
                ConicKind::Ellipse => {
                    let n = (((a.abs() + b.abs()) * s) as usize).clamp(64, 720);
                    vec![(0..=n).map(|k| {
                        let th = TAU * k as f64 / n as f64;
                        tr(a * th.cos(), b * th.sin())
                    }).collect()]
                }
                ConicKind::Hyperbola => {
                    let umax = (far / b.abs().max(1e-9)).asinh().clamp(0.5, 12.0) + 0.2;
                    let n = 240;
                    [1.0, -1.0]
                        .iter()
                        .map(|sg| {
                            (0..=n).map(|k| {
                                let q = -1.0 + 2.0 * k as f64 / n as f64;
                                let u = umax * q * q.abs();
                                tr(sg * a * u.cosh(), b * u.sinh())
                            }).collect()
                        })
                        .collect()
                }
                ConicKind::Parabola => {
                    let vmax = 2.0 * far + 4.0 * a.abs().sqrt() * far.sqrt() + 1.0;
                    let n = 300;
                    vec![(0..=n).map(|k| {
                        let q = -1.0 + 2.0 * k as f64 / n as f64;
                        let v = vmax * q * q * q.signum();
                        tr(v * v / (4.0 * a), v)
                    }).collect()]
                }
            }
        }
        Ent::Curve { src } => curve_polys(doc, src, w),
    }
}

/// Polylines of the function curve `src` inside window `w`: the graph of
/// `y = f(x)` sampled every 1.5 pixels, or the zero set of `F(x, y) - G(x, y)`
/// by marching squares on 7-pixel cells.
fn curve_polys(doc: &Doc, src: &str, w: &Window) -> Vec<Vec<[f64; 2]>> {
    let Ok(def) = doc.curve_def(src) else { return vec![] };
    let vals: Vec<f64> = doc.params.iter().map(|p| p.v).collect();
    let (wpx, hpx) = (w.width_px(), w.height_px());
    let mut st = Vec::new();
    if let Some(f) = &def.explicit {
        let code = Code::new(f);
        let mut out: Vec<Vec<[f64; 2]>> = vec![];
        let mut cur: Vec<[f64; 2]> = vec![];
        let n = (wpx / 1.5) as usize + 2;
        let (ylo, yhi) = (w.min[1] - 4000.0 * w.px, w.max[1] + 4000.0 * w.px);
        let inside = |y: f64| y >= w.min[1] && y <= w.max[1];
        let jump = (w.max[1] - w.min[1]) * 1.5;
        let mut prev: Option<f64> = None;
        for k in 0..=n {
            let x = w.min[0] + (-2.0 + (wpx + 4.0) * k as f64 / n as f64) * w.px;
            let y = code.eval(&|s: u32| if s == X_SYM { x } else { vals.get(s as usize).copied().unwrap_or(f64::NAN) }, &mut st);
            if !y.is_finite() {
                if cur.len() > 1 {
                    out.push(std::mem::take(&mut cur));
                }
                cur.clear();
                prev = None;
                continue;
            }
            let y = y.clamp(ylo, yhi);
            if let Some(q) = prev
                && (y - q).abs() > jump
                && !(inside(y) && inside(q))
            {
                if cur.len() > 1 {
                    out.push(std::mem::take(&mut cur));
                }
                cur.clear();
            }
            cur.push([x, y]);
            prev = Some(y);
        }
        if cur.len() > 1 {
            out.push(cur);
        }
        return out;
    }
    // marching squares on the implicit form, rows running down from the top
    let code = Code::new(&def.implicit);
    let cell = 7.0 * w.px;
    let nx = (wpx / 7.0) as usize + 2;
    let ny = (hpx / 7.0) as usize + 2;
    let at = |i: usize, j: usize| [w.min[0] + i as f64 * cell, w.max[1] - j as f64 * cell];
    let mut g = vec![0.0f64; (nx + 1) * (ny + 1)];
    for j in 0..=ny {
        for i in 0..=nx {
            let p = at(i, j);
            g[j * (nx + 1) + i] = code.eval(
                &|s: u32| {
                    if s == X_SYM {
                        p[0]
                    } else if s == Y_SYM {
                        p[1]
                    } else {
                        vals.get(s as usize).copied().unwrap_or(f64::NAN)
                    }
                },
                &mut st,
            );
        }
    }
    let mut segs = Vec::new();
    for j in 0..ny {
        for i in 0..nx {
            let c = [g[j * (nx + 1) + i], g[j * (nx + 1) + i + 1], g[(j + 1) * (nx + 1) + i + 1], g[(j + 1) * (nx + 1) + i]];
            if c.iter().any(|x| !x.is_finite()) {
                continue;
            }
            let corners = [at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)];
            let mut pts = Vec::new();
            for k in 0..4 {
                let (a, b) = (c[k], c[(k + 1) % 4]);
                if (a < 0.0) != (b < 0.0) {
                    // reject sign changes caused by poles
                    if (a - b).abs() > 1e6 * (a.abs().min(b.abs()) + 1.0) && a.abs().max(b.abs()) > 1e3 {
                        continue;
                    }
                    let t = a / (a - b);
                    let (p, q) = (corners[k], corners[(k + 1) % 4]);
                    pts.push([p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t]);
                }
            }
            if pts.len() == 2 {
                segs.push([pts[0], pts[1]]);
            } else if pts.len() == 4 {
                segs.push([pts[0], pts[1]]);
                segs.push([pts[2], pts[3]]);
            }
        }
    }
    join_segments(&segs, w.px)
}

/// Joins segments sharing endpoints (to an eighth of a pixel of size `px`)
/// into longer polylines.
fn join_segments(segs: &[[[f64; 2]; 2]], px: f64) -> Vec<Vec<[f64; 2]>> {
    let key = |p: [f64; 2]| ((p[0] / px * 8.0).round() as i64, (p[1] / px * 8.0).round() as i64);
    let mut ends: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (i, s) in segs.iter().enumerate() {
        ends.entry(key(s[0])).or_default().push(i);
        ends.entry(key(s[1])).or_default().push(i);
    }
    let mut used = vec![false; segs.len()];
    let mut out = Vec::new();
    for i in 0..segs.len() {
        if used[i] {
            continue;
        }
        used[i] = true;
        let mut line: VecDeque<[f64; 2]> = segs[i].iter().copied().collect();
        for forward in [true, false] {
            loop {
                let end = if forward { *line.back().unwrap() } else { *line.front().unwrap() };
                let next = ends.get(&key(end)).and_then(|v| v.iter().copied().find(|j| !used[*j]));
                let Some(j) = next else { break };
                used[j] = true;
                let s = &segs[j];
                let other = if key(s[0]) == key(end) { s[1] } else { s[0] };
                if forward {
                    line.push_back(other);
                } else {
                    line.push_front(other);
                }
            }
        }
        out.push(line.into_iter().collect());
    }
    out
}
