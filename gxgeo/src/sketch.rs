//! Editing a document: structural changes that are kept only when the
//! constraints still solve, dimension values, undo history, and the
//! constraint-status analysis of the last solve.

use crate::build::{angle_supp, angle_vertex, foot};
use crate::expr::parse::plain_number;
use crate::expr::{Expr, X_SYM, eval};
use crate::model::*;
use crate::solver::{DragEq, Sys};
use std::collections::{HashMap, HashSet};

/// Undo entries kept.
const UNDO_LIMIT: usize = 300;

/// A document with its solver state and undo history.
pub struct Sketch {
    pub doc: Doc,
    pub undo: Vec<Doc>,
    pub redo: Vec<Doc>,
    /// Equation system of `doc`; stale while `dirty`.
    pub sys: Option<Sys>,
    /// True when `doc` changed structurally since `sys` was built.
    pub dirty: bool,
    /// Parameters the constraints leave free.
    pub free_params: HashSet<usize>,
    /// Remaining degrees of freedom.
    pub dof: usize,
    /// Rank of the equation system.
    pub rank: usize,
    /// Constraints and dimensions that the last solve left unsatisfied.
    pub conflicts: HashSet<Owner>,
    /// Constraints and dimensions that could not be turned into equations.
    pub errors: HashMap<Owner, String>,
    /// Counter raised by every change of the document.
    pub generation: u64,
}

impl Default for Sketch {
    fn default() -> Self {
        Self::new()
    }
}

impl Sketch {
    /// Empty document (origin and axes only), solved.
    pub fn new() -> Sketch {
        let mut s = Sketch::from_doc(Doc::new());
        s.solve_full();
        s
    }

    /// Wraps `doc` without solving it.
    pub fn from_doc(doc: Doc) -> Sketch {
        Sketch {
            doc,
            undo: vec![],
            redo: vec![],
            sys: None,
            dirty: true,
            free_params: HashSet::new(),
            dof: 0,
            rank: 0,
            conflicts: HashSet::new(),
            errors: HashMap::new(),
            generation: 0,
        }
    }

    // ------------------------------------------------------------ solving

    pub fn rebuild(&mut self) {
        let (eqns, errs) = self.doc.equations();
        self.errors = errs.into_iter().collect();
        self.sys = Some(Sys::build(&eqns, self.doc.unknowns()));
        self.dirty = false;
    }

    pub fn vals(&self) -> Vec<f64> {
        self.doc.params.iter().map(|p| p.v).collect()
    }

    pub fn store(&mut self, vals: &[f64]) {
        for (p, v) in self.doc.params.iter_mut().zip(vals) {
            p.v = *v;
        }
    }

    /// Solves without drags and refreshes the constraint-status analysis.
    pub fn solve_full(&mut self) -> bool {
        if self.dirty || self.sys.is_none() {
            self.rebuild();
        }
        let sys = self.sys.as_ref().unwrap();
        let mut vals = self.vals();
        let ok = sys.solve(&mut vals, &[], [0.0; 2]);
        let an = sys.analyze(&vals);
        let free: HashSet<usize> = an.free.iter().enumerate().filter(|(_, f)| **f).map(|(i, _)| sys.unknowns[i]).collect();
        let rank = sys.unknowns.len() - an.dof;
        let conflicts: HashSet<Owner> = sys.violated(&vals).into_iter().collect();
        self.store(&vals);
        self.free_params = free;
        self.dof = an.dof;
        self.rank = rank;
        self.conflicts = conflicts;
        ok
    }

    fn eq_count(&self) -> usize {
        self.sys.as_ref().map(|s| s.eqs.len()).unwrap_or(0)
    }

    /// Applies a structural change; rolls back when it cannot be satisfied
    /// (or, with `strict`, when it adds redundant equations).
    pub fn try_change(&mut self, strict: bool, f: impl FnOnce(&mut Doc)) -> bool {
        let before = self.doc.clone();
        if self.dirty {
            self.solve_full();
        }
        let (rank0, eqs0) = (self.rank, self.eq_count());
        let had_conflicts = !self.conflicts.is_empty();
        f(&mut self.doc);
        self.dirty = true;
        let ok = self.solve_full();
        let added = self.eq_count().saturating_sub(eqs0);
        let redundant = strict && added > 0 && self.rank < rank0 + added;
        if (!ok && !had_conflicts) || redundant || (!self.conflicts.is_empty() && !had_conflicts) {
            self.doc = before;
            self.dirty = true;
            self.solve_full();
            return false;
        }
        self.push_undo(before);
        true
    }

    /// Solves the drag equations `exprs` (each equal to 0) together with the
    /// constraints, pulling toward `target`; keeps the result when it solves.
    pub fn drag_solve(&mut self, exprs: &[Expr], target: [f64; 2]) {
        if self.dirty {
            self.rebuild();
        }
        let sys = self.sys.as_ref().unwrap();
        let drags: Vec<DragEq> = exprs.iter().map(|e| sys.drag_eq(e)).collect();
        let mut vals = self.vals();
        if sys.solve(&mut vals, &drags, target) {
            self.store(&vals);
        }
    }

    // ------------------------------------------------------------ history

    pub fn push_undo(&mut self, before: Doc) {
        self.undo.push(before);
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.generation += 1;
    }

    /// Records the current document as an undo step.
    pub fn checkpoint(&mut self) {
        let d = self.doc.clone();
        self.push_undo(d);
    }

    /// Returns false when there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        let Some(d) = self.undo.pop() else { return false };
        let cur = std::mem::replace(&mut self.doc, d);
        self.redo.push(cur);
        self.after_history();
        true
    }

    /// Returns false when there is nothing to redo.
    pub fn redo(&mut self) -> bool {
        let Some(d) = self.redo.pop() else { return false };
        let cur = std::mem::replace(&mut self.doc, d);
        self.undo.push(cur);
        self.after_history();
        true
    }

    fn after_history(&mut self) {
        self.dirty = true;
        self.generation += 1;
        self.solve_full();
    }

    /// Replaces the document (an opened file) as an undoable step, and solves it.
    pub fn load(&mut self, doc: Doc) {
        let before = std::mem::replace(&mut self.doc, doc);
        self.push_undo(before);
        self.dirty = true;
        self.solve_full();
    }

    /// Changes parameter values (dimensions, variables), solving with
    /// continuation when a direct solve fails.
    pub fn set_values(&mut self, changes: &[(usize, f64)], undoable: bool) -> bool {
        let before = self.doc.clone();
        let old: Vec<f64> = changes.iter().map(|(p, _)| self.doc.params[*p].v).collect();
        for (p, v) in changes {
            self.doc.params[*p].v = *v;
        }
        let mut ok = self.solve_full();
        if !ok {
            self.doc = before.clone();
            let steps = 24;
            ok = true;
            for k in 1..=steps {
                let t = k as f64 / steps as f64;
                for (j, (p, v)) in changes.iter().enumerate() {
                    self.doc.params[*p].v = old[j] + (v - old[j]) * t;
                }
                if !self.solve_full() {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            self.doc = before;
            self.solve_full();
            return false;
        }
        if undoable {
            self.push_undo(before);
        } else {
            self.generation += 1;
        }
        true
    }

    // ------------------------------------------------------------ creation

    /// Adds a constraint; false when it is rejected (unsatisfiable or
    /// redundant). The contact point of a generic tangency starts at a guess
    /// from sampling both curves.
    pub fn add_constraint(&mut self, c: Con) -> bool {
        self.add_constraint_with(c, |d, a, b| contact_guess(&sample_polys(d, a), &sample_polys(d, b), [0.0; 2]))
    }

    /// Like [`Sketch::add_constraint`], with `guess(doc, a, b)` giving the
    /// starting contact point of a generic tangency between curves a and b.
    pub fn add_constraint_with(&mut self, c: Con, guess: impl FnOnce(&Doc, usize, usize) -> [f64; 2]) -> bool {
        let c = if let Con::Tangent { a, b, s, aux: None } = c {
            let d = &self.doc;
            let special = (d.is_line(a) && d.is_circlelike(b)) || (d.is_circlelike(a) && d.is_circlelike(b));
            if special || d.tangent_contact(a, b).is_some() {
                Con::Tangent { a, b, s, aux: None }
            } else {
                let g = guess(&self.doc, a, b);
                let tx = self.doc.add_param(g[0], false);
                let ty = self.doc.add_param(g[1], false);
                Con::Tangent { a, b, s, aux: Some((tx, ty)) }
            }
        } else {
            c
        };
        self.try_change(true, |d| d.cons.push(Some(c)))
    }

    /// Adds a dimension with its label at `label_w`: driving when that keeps
    /// the constraints solvable, otherwise a reference (measured) dimension.
    /// For an angle, the label's sector selects the angle or its supplement.
    pub fn add_dimension(&mut self, kind: DimKind, label_w: [f64; 2]) -> Option<usize> {
        let anchor = self.doc.dim_anchor(&kind);
        let off = [label_w[0] - anchor[0], label_w[1] - anchor[1]];
        let supp = match kind {
            DimKind::Angle(l1, l2) => angle_supp(&self.doc, l1, l2, label_w),
            _ => false,
        };
        let make = |d: &mut Doc, driving: bool| -> usize {
            let i = d.add_dim(kind, off);
            if supp {
                d.dims[i].as_mut().unwrap().supp = true;
                let v = d.measure_value(i);
                let p = d.dims[i].as_ref().unwrap().param;
                d.params[p].v = v;
            }
            d.dims[i].as_mut().unwrap().driving = driving;
            i
        };
        let mut idx = None;
        let ok = self.try_change(true, |d| {
            idx = Some(make(d, true));
        });
        if !ok {
            let mut idx2 = None;
            self.try_change(false, |d| {
                idx2 = Some(make(d, false));
            });
            return idx2;
        }
        idx
    }

    /// Sets dimension `i` from typed text: a number (degrees for angles)
    /// drives it to that value, an expression of variables and dimensions
    /// drives it to that expression, empty text makes it a reference
    /// dimension. False when the change is rejected.
    pub fn set_dim_text(&mut self, i: usize, text: &str) -> bool {
        let Some(dim) = self.doc.dims[i].clone() else { return false };
        let t = text.trim();
        if t.is_empty() {
            if dim.driving {
                self.try_change(false, |d| {
                    let x = d.dims[i].as_mut().unwrap();
                    x.driving = false;
                    x.expr.clear();
                });
            }
            return true;
        }
        let angle = dim.kind.is_angle();
        if let Some(v) = plain_number(t) {
            let v = if angle { v.to_radians() } else { v };
            if !dim.driving || !dim.expr.is_empty() {
                let mv = self.doc.measure_value(i);
                let ok = self.try_change(true, |d| {
                    let s = d.dim_sign(d.dims[i].as_ref().unwrap());
                    let x = d.dims[i].as_mut().unwrap();
                    x.driving = true;
                    x.expr.clear();
                    x.s = s;
                    d.params[x.param].v = mv;
                });
                if !ok {
                    return false;
                }
            }
            let p = dim.param;
            self.set_values(&[(p, v)], true)
        } else {
            if self.doc.parse_expr(t, 0).is_err() {
                return false;
            }
            let was_driving = dim.driving;
            let tt = t.to_string();
            let ok = self.try_change(!was_driving, |d| {
                let s = d.dim_sign(d.dims[i].as_ref().unwrap());
                let x = d.dims[i].as_mut().unwrap();
                if !x.driving {
                    x.s = s;
                }
                x.driving = true;
                x.expr = tt;
            });
            if ok {
                return true;
            }
            // try reaching the new value gradually
            let before = self.doc.clone();
            let cur = self.doc.measure_value(i);
            self.doc.dims[i].as_mut().unwrap().driving = true;
            self.doc.dims[i].as_mut().unwrap().expr = t.to_string();
            let target = self.doc.dim_value_expr(i, 0).map(|e| self.doc.eval_now(&e)).unwrap_or(f64::NAN);
            self.doc = before;
            if target.is_finite() {
                let p = dim.param;
                let ok2 = self.try_change(!was_driving, |d| {
                    let s = d.dim_sign(d.dims[i].as_ref().unwrap());
                    let x = d.dims[i].as_mut().unwrap();
                    x.driving = true;
                    x.expr.clear();
                    x.s = s;
                    d.params[p].v = cur;
                });
                if ok2 && self.set_values(&[(p, target)], false) {
                    let tt = t.to_string();
                    if self.try_change(false, |d| d.dims[i].as_mut().unwrap().expr = tt) {
                        return true;
                    }
                }
            }
            false
        }
    }

    /// Switches dimension `i` between driving and reference. False when
    /// making it driving is rejected.
    pub fn toggle_driving(&mut self, i: usize) -> bool {
        let Some(dim) = self.doc.dims[i].clone() else { return false };
        if dim.driving {
            self.try_change(false, |d| d.dims[i].as_mut().unwrap().driving = false);
            true
        } else {
            let v = self.doc.measure_value(i);
            self.try_change(true, |d| {
                let s = d.dim_sign(d.dims[i].as_ref().unwrap());
                let x = d.dims[i].as_mut().unwrap();
                x.driving = true;
                x.s = s;
                if x.expr.is_empty() {
                    d.params[x.param].v = v;
                }
            })
        }
    }

    /// Renames a point, dimension or variable from `old` to `new`: `set`
    /// stores the new name, and expressions using the old one are rewritten.
    /// False when `new` is not an identifier or is taken.
    pub fn rename(&mut self, old: &str, new: &str, set: impl FnOnce(&mut Doc)) -> bool {
        let new = new.trim();
        if new == old {
            return true;
        }
        if !valid_ident(new) || self.doc.name_taken(new) {
            return false;
        }
        let (o, n) = (old.to_string(), new.to_string());
        self.try_change(false, |d| {
            set(d);
            for v in d.vars.iter_mut().flatten() {
                v.expr = rename_ident(&v.expr, &o, &n);
            }
            for x in d.dims.iter_mut().flatten() {
                x.expr = rename_ident(&x.expr, &o, &n);
            }
            for e in d.ents.iter_mut().flatten() {
                if let Ent::Curve { src } = &mut e.kind {
                    *src = rename_ident(src, &o, &n);
                }
            }
        })
    }

    /// Deletes entities, constraints, dimensions and variables (and what
    /// depends on them). Returns false when nothing was given.
    pub fn delete(&mut self, ents: &[usize], cons: &[usize], dims: &[usize], vars: &[usize]) -> bool {
        if ents.is_empty() && cons.is_empty() && dims.is_empty() && vars.is_empty() {
            return false;
        }
        self.try_change(false, |d| {
            d.delete(ents, cons, dims);
            for v in vars {
                d.vars[*v] = None;
            }
        });
        true
    }
}

impl Doc {
    /// Tangency constraint between a and b with the side or kind matching
    /// the current configuration; None when they cannot be tangent.
    pub fn tangent_con(&self, a: usize, b: usize) -> Option<Con> {
        let d = self;
        let side = |l: usize, c: usize| {
            let (p, q) = d.line_pts(l);
            let (p, q) = (d.pval(p), d.pval(q));
            let (cc, _) = d.circ_val(c);
            let cr = (q[0] - p[0]) * (cc[1] - p[1]) - (q[1] - p[1]) * (cc[0] - p[0]);
            if cr < 0.0 { -1.0 } else { 1.0 }
        };
        if d.is_line(a) && d.is_circlelike(b) {
            return Some(Con::Tangent { a, b, s: side(a, b), aux: None });
        }
        if d.is_line(b) && d.is_circlelike(a) {
            return Some(Con::Tangent { a: b, b: a, s: side(b, a), aux: None });
        }
        if d.is_circlelike(a) && d.is_circlelike(b) {
            let (c1, r1) = d.circ_val(a);
            let (c2, r2) = d.circ_val(b);
            let dist = (c1[0] - c2[0]).hypot(c1[1] - c2[1]);
            let s = if (dist - (r1 + r2)).abs() <= (dist - (r1 - r2).abs()).abs() { 1.0 } else { -1.0 };
            return Some(Con::Tangent { a, b, s, aux: None });
        }
        if d.is_point(a) || d.is_point(b) || a == b {
            return None;
        }
        Some(Con::Tangent { a, b, s: 1.0, aux: None })
    }

    /// Point a dimension of kind `k` is attached to; its label sits at an
    /// offset from here.
    pub fn dim_anchor(&self, k: &DimKind) -> [f64; 2] {
        let d = self;
        let mid = |a: [f64; 2], b: [f64; 2]| [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
        match *k {
            DimKind::Distance(a, b) | DimKind::HDist(a, b) | DimKind::VDist(a, b) => mid(d.pval(a), d.pval(b)),
            DimKind::Length(l) => {
                let (a, b) = d.line_pts(l);
                mid(d.pval(a), d.pval(b))
            }
            DimKind::PointLine(p, l) => {
                let pp = d.pval(p);
                let f = foot(d, pp, l);
                mid(pp, f)
            }
            DimKind::Radius(c) | DimKind::Diameter(c) => d.circ_val(c).0,
            DimKind::Angle(l1, l2) => angle_vertex(d, l1, l2),
            DimKind::ConicA(c) | DimKind::ConicB(c) => match d.kind(c) {
                Some(Ent::Conic { c, .. }) => d.pval(*c),
                _ => [0.0; 2],
            },
            DimKind::CoordX(p) | DimKind::CoordY(p) => mid(d.pval(d.origin), d.pval(p)),
        }
    }
}

/// Polylines sampling entity e in world coordinates.
pub fn sample_polys(d: &Doc, e: usize) -> Vec<Vec<[f64; 2]>> {
    let n = 400;
    let lin = |lo: f64, hi: f64, k: usize| lo + (hi - lo) * k as f64 / n as f64;
    match d.kind(e) {
        Some(Ent::Line { a, b, .. }) => {
            let (pa, pb) = (d.pval(*a), d.pval(*b));
            vec![(0..=n).map(|k| {
                let t = lin(-3.0, 4.0, k);
                [pa[0] + (pb[0] - pa[0]) * t, pa[1] + (pb[1] - pa[1]) * t]
            }).collect()]
        }
        Some(Ent::Circle { .. }) | Some(Ent::Arc { .. }) => {
            let (c, r) = d.circ_val(e);
            vec![(0..=n).map(|k| {
                let t = lin(0.0, std::f64::consts::TAU, k);
                [c[0] + r * t.cos(), c[1] + r * t.sin()]
            }).collect()]
        }
        Some(Ent::Conic { kind, c, a, b, t }) => {
            let cc = d.pval(*c);
            let (a, b, t) = (d.params[*a].v, d.params[*b].v, d.params[*t].v);
            let (ct, st) = (t.cos(), t.sin());
            let tr = |u: f64, w: f64| [cc[0] + u * ct - w * st, cc[1] + u * st + w * ct];
            match kind {
                ConicKind::Ellipse => vec![(0..=n).map(|k| {
                    let th = lin(0.0, std::f64::consts::TAU, k);
                    tr(a * th.cos(), b * th.sin())
                }).collect()],
                ConicKind::Hyperbola => [1.0, -1.0]
                    .iter()
                    .map(|sg| (0..=n).map(|k| {
                        let u = lin(-3.0, 3.0, k);
                        tr(sg * a * u.cosh(), b * u.sinh())
                    }).collect())
                    .collect(),
                ConicKind::Parabola => vec![(0..=n).map(|k| {
                    let w = lin(-12.0 * a.abs().max(0.1), 12.0 * a.abs().max(0.1), k);
                    tr(w * w / (4.0 * a), w)
                }).collect()],
            }
        }
        Some(Ent::Curve { src }) => {
            let Ok(def) = d.curve_def(src) else { return vec![] };
            let Some(f) = def.explicit else { return vec![] };
            let vals: Vec<f64> = d.params.iter().map(|p| p.v).collect();
            let mut out = vec![vec![]];
            for k in 0..=2000 {
                let x = -25.0 + 50.0 * k as f64 / 2000.0;
                let y = eval(&f, &|s| if s == X_SYM { x } else { vals.get(s as usize).copied().unwrap_or(f64::NAN) });
                if y.is_finite() && y.abs() < 1e6 {
                    out.last_mut().unwrap().push([x, y]);
                } else if !out.last().unwrap().is_empty() {
                    out.push(vec![]);
                }
            }
            out
        }
        _ => vec![],
    }
}

/// Initial contact point for a generic tangency between two curves given as
/// polylines: the pair of samples that are close together and have parallel
/// tangents; `fallback` when either has no samples.
pub fn contact_guess(pa: &[Vec<[f64; 2]>], pb: &[Vec<[f64; 2]>], fallback: [f64; 2]) -> [f64; 2] {
    let with_dirs = |polys: &[Vec<[f64; 2]>]| -> Vec<([f64; 2], [f64; 2])> {
        let mut v = Vec::new();
        for pl in polys {
            for i in 1..pl.len().saturating_sub(1) {
                let d = [pl[i + 1][0] - pl[i - 1][0], pl[i + 1][1] - pl[i - 1][1]];
                let l = d[0].hypot(d[1]);
                if l > 0.0 {
                    v.push((pl[i], [d[0] / l, d[1] / l]));
                }
            }
        }
        v
    };
    let (sa, sb) = (with_dirs(pa), with_dirs(pb));
    if sa.is_empty() || sb.is_empty() {
        return fallback;
    }
    let step_a = (sa.len() / 500).max(1);
    let step_b = (sb.len() / 500).max(1);
    let scale = {
        let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        for (p, _) in sa.iter().chain(&sb) {
            for k in 0..2 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        (hi[0] - lo[0]).hypot(hi[1] - lo[1]).max(1e-9)
    };
    let mut best = (f64::INFINITY, fallback);
    for (p, dp) in sa.iter().step_by(step_a) {
        for (q, dq) in sb.iter().step_by(step_b) {
            let dist = (p[0] - q[0]).hypot(p[1] - q[1]);
            let mis = (dp[0] * dq[1] - dp[1] * dq[0]).abs();
            let score = dist / scale + 0.5 * mis;
            if score < best.0 {
                best = (score, [(p[0] + q[0]) * 0.5, (p[1] + q[1]) * 0.5]);
            }
        }
    }
    best.1
}

/// Crossing of curves a and b nearest to `hint`, found on their sampled
/// polylines.
pub fn crossing_near(d: &Doc, a: usize, b: usize, hint: [f64; 2]) -> Option<[f64; 2]> {
    let (pa, pb) = (sample_polys(d, a), sample_polys(d, b));
    let mut best: Option<(f64, [f64; 2])> = None;
    for la in &pa {
        for wa in la.windows(2) {
            for lb in &pb {
                for wb in lb.windows(2) {
                    if let Some(q) = seg_cross(wa[0], wa[1], wb[0], wb[1]) {
                        let dist = (q[0] - hint[0]).hypot(q[1] - hint[1]);
                        if best.is_none_or(|b| dist < b.0) {
                            best = Some((dist, q));
                        }
                    }
                }
            }
        }
    }
    best.map(|b| b.1)
}

/// Crossing of segments ab and cd.
fn seg_cross(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> Option<[f64; 2]> {
    let (r, s) = ([b[0] - a[0], b[1] - a[1]], [d[0] - c[0], d[1] - c[1]]);
    let den = r[0] * s[1] - r[1] * s[0];
    if den.abs() < 1e-12 {
        return None;
    }
    let t = ((c[0] - a[0]) * s[1] - (c[1] - a[1]) * s[0]) / den;
    let u = ((c[0] - a[0]) * r[1] - (c[1] - a[1]) * r[0]) / den;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) { Some([a[0] + r[0] * t, a[1] + r[1] * t]) } else { None }
}

/// Point of curve c, found on its sampled polylines, where a line from `p`
/// touches it, nearest to `hint`.
pub fn tangent_contact_near(d: &Doc, p: [f64; 2], c: usize, hint: [f64; 2]) -> Option<[f64; 2]> {
    let mut best: Option<(f64, [f64; 2])> = None;
    for pl in sample_polys(d, c) {
        if pl.len() < 3 {
            continue;
        }
        // side of the tangent at sample i relative to the line from p
        let f = |i: usize| {
            let (n, m) = (pl[(i + 1).min(pl.len() - 1)], pl[i.saturating_sub(1)]);
            let (t, q) = ([n[0] - m[0], n[1] - m[1]], [pl[i][0] - p[0], pl[i][1] - p[1]]);
            t[0] * q[1] - t[1] * q[0]
        };
        let mut prev = f(0);
        for i in 1..pl.len() {
            let cur = f(i);
            if (prev < 0.0) != (cur < 0.0) {
                let dist = (pl[i][0] - hint[0]).hypot(pl[i][1] - hint[1]);
                if best.is_none_or(|b| dist < b.0) {
                    best = Some((dist, pl[i]));
                }
            }
            prev = cur;
        }
    }
    best.map(|b| b.1)
}

/// True for a name usable in expressions: a letter, then letters, digits or `_`.
pub fn valid_ident(s: &str) -> bool {
    let mut cs = s.chars();
    matches!(cs.next(), Some(c) if c.is_alphabetic()) && cs.all(|c| c.is_alphanumeric() || c == '_')
}

/// Renames identifier tokens equal to `old` in an expression string.
pub fn rename_ident(s: &str, old: &str, new: &str) -> String {
    let cs: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < cs.len() {
        if cs[i].is_alphabetic() || cs[i] == '_' {
            let st = i;
            while i < cs.len() && (cs[i].is_alphanumeric() || cs[i] == '_') {
                i += 1;
            }
            let w: String = cs[st..i].iter().collect();
            out.push_str(if w == old { new } else { &w });
        } else {
            out.push(cs[i]);
            i += 1;
        }
    }
    out
}
