//! Numeric constraint solver: compiled residuals and Jacobians, damped
//! least-squares for dragging, and minimum-norm Newton projection onto the
//! constraint manifold.

use crate::expr::*;
use crate::model::Owner;
use std::collections::HashMap;

pub struct CEq {
    pub f: Code,
    pub jac: Vec<(usize, Code)>,
    pub owner: Owner,
}

pub struct DragEq {
    pub f: Code,
    pub jac: Vec<(usize, Code)>,
}

pub struct Sys {
    pub eqs: Vec<CEq>,
    pub unknowns: Vec<usize>,
    pub col: HashMap<usize, usize>,
}

pub struct Analysis {
    /// Per unknown column: true when it is not determined by the constraints.
    pub free: Vec<bool>,
    pub dof: usize,
}

fn compile_eq(e: &Expr, col: &HashMap<usize, usize>) -> (Code, Vec<(usize, Code)>) {
    let mut s: Vec<u32> = sym_set(e).into_iter().collect();
    s.sort();
    let mut jac = Vec::new();
    for v in s {
        if let Some(&c) = col.get(&(v as usize)) {
            let d = diff(e, v);
            if !d.is_zero() {
                jac.push((c, Code::new(&d)));
            }
        }
    }
    (Code::new(e), jac)
}

#[inline]
fn loader<'a>(vals: &'a [f64], t: [f64; 2]) -> impl Fn(u32) -> f64 + 'a {
    move |s: u32| {
        if (s as usize) < vals.len() {
            vals[s as usize]
        } else if s == TX_SYM {
            t[0]
        } else if s == TY_SYM {
            t[1]
        } else {
            f64::NAN
        }
    }
}

/// Solves the symmetric positive definite system `a x = b` in place.
pub(crate) fn spd_solve(a: &mut [f64], n: usize, b: &mut [f64]) -> bool {
    for j in 0..n {
        let mut d = a[j * n + j];
        for k in 0..j {
            d -= a[j * n + k] * a[j * n + k];
        }
        if d <= 0.0 || !d.is_finite() {
            return false;
        }
        let d = d.sqrt();
        a[j * n + j] = d;
        for i in j + 1..n {
            let mut s = a[i * n + j];
            for k in 0..j {
                s -= a[i * n + k] * a[j * n + k];
            }
            a[i * n + j] = s / d;
        }
    }
    for i in 0..n {
        let mut s = b[i];
        for k in 0..i {
            s -= a[i * n + k] * b[k];
        }
        b[i] = s / a[i * n + i];
    }
    for i in (0..n).rev() {
        let mut s = b[i];
        for k in i + 1..n {
            s -= a[k * n + i] * b[k];
        }
        b[i] = s / a[i * n + i];
    }
    true
}

struct Comp {
    eqs: Vec<usize>,
    drags: Vec<usize>,
    cols: Vec<usize>,
}

/// Residual tolerance, relative to the equation's scale (see `Sys::eq_scale`).
pub const TOL: f64 = 1e-9;

impl Sys {
    pub fn build(eqns: &[(Expr, Owner)], unknowns: Vec<usize>) -> Sys {
        let col: HashMap<usize, usize> = unknowns.iter().enumerate().map(|(i, p)| (*p, i)).collect();
        let mut eqs = Vec::new();
        for (e, o) in eqns {
            let (f, jac) = compile_eq(e, &col);
            eqs.push(CEq { f, jac, owner: *o });
        }
        Sys { eqs, unknowns, col }
    }

    pub fn drag_eq(&self, e: &Expr) -> DragEq {
        let (f, jac) = compile_eq(e, &self.col);
        DragEq { f, jac }
    }

    fn components(&self, drags: &[DragEq]) -> Vec<Comp> {
        let n = self.unknowns.len();
        let mut parent: Vec<usize> = (0..n).collect();
        fn find(p: &mut [usize], mut i: usize) -> usize {
            while p[i] != i {
                p[i] = p[p[i]];
                i = p[i];
            }
            i
        }
        let join = |jac: &[(usize, Code)], parent: &mut Vec<usize>| {
            if let Some((c0, _)) = jac.first() {
                let r0 = find(parent, *c0);
                for (c, _) in &jac[1..] {
                    let r = find(parent, *c);
                    parent[r] = r0;
                }
            }
        };
        for e in &self.eqs {
            join(&e.jac, &mut parent);
        }
        for d in drags {
            join(&d.jac, &mut parent);
        }
        let mut map: HashMap<usize, Comp> = HashMap::new();
        for c in 0..n {
            let r = find(&mut parent, c);
            map.entry(r).or_insert_with(|| Comp { eqs: vec![], drags: vec![], cols: vec![] }).cols.push(c);
        }
        for (i, e) in self.eqs.iter().enumerate() {
            if let Some((c, _)) = e.jac.first() {
                let r = find(&mut parent, *c);
                map.get_mut(&r).unwrap().eqs.push(i);
            }
        }
        for (i, d) in drags.iter().enumerate() {
            if let Some((c, _)) = d.jac.first() {
                let r = find(&mut parent, *c);
                map.get_mut(&r).unwrap().drags.push(i);
            }
        }
        let mut v: Vec<Comp> = map.into_values().collect();
        v.sort_by_key(|c| c.cols[0]);
        v
    }

    /// The size of the unknowns in `cols` (their largest magnitude): the
    /// length scale the residual tolerances are relative to.
    fn length_scale(&self, vals: &[f64], cols: impl Iterator<Item = usize>) -> f64 {
        cols.map(|k| vals[self.unknowns[k]].abs()).filter(|x| x.is_finite()).fold(1e-9, f64::max)
    }

    /// Scale of equation `k`'s residual: what a change of the unknowns by
    /// `l` (the length scale) changes it by, |∇f|·l. Residuals measured
    /// against it are the same at every scale of the construction: a length
    /// equation |AB|² − d² grows with the square of the scale, an angle
    /// equation not at all.
    fn eq_scale(&self, k: usize, ld: &impl Fn(u32) -> f64, st: &mut Vec<f64>, l: f64) -> f64 {
        let g: f64 = self.eqs[k].jac.iter().map(|(_, code)| code.eval(ld, st).powi(2)).sum::<f64>().sqrt();
        if g.is_finite() { g * l } else { 0.0 }
    }

    /// Scales of the equations `eqs` (see `eq_scale`). An equation whose
    /// gradient vanishes here (a double root) gets a small share of the
    /// largest one instead; with no gradient at all, 1.
    fn eq_scales(&self, eqs: &[usize], vals: &[f64], target: [f64; 2], cols: impl Iterator<Item = usize>) -> Vec<f64> {
        let mut st = Vec::new();
        let ld = loader(vals, target);
        let l = self.length_scale(vals, cols);
        let s: Vec<f64> = eqs.iter().map(|&k| self.eq_scale(k, &ld, &mut st, l)).collect();
        let top = s.iter().copied().fold(0.0, f64::max);
        let floor = if top > 0.0 { 1e-6 * top } else { 1.0 };
        s.into_iter().map(|x| x.max(floor)).collect()
    }

    /// Residuals of all equations relative to their scales.
    fn scaled_residuals(&self, vals: &[f64]) -> Vec<f64> {
        let all: Vec<usize> = (0..self.eqs.len()).collect();
        let scales = self.eq_scales(&all, vals, [0.0; 2], 0..self.unknowns.len());
        let mut st = Vec::new();
        let ld = loader(vals, [0.0; 2]);
        all.iter()
            .map(|&k| {
                let r = self.eqs[k].f.eval(&ld, &mut st);
                if r.is_nan() { f64::INFINITY } else { r.abs() / scales[k] }
            })
            .collect()
    }

    /// Maximum residual over all equations, relative to their scales.
    pub fn max_residual(&self, vals: &[f64]) -> f64 {
        self.scaled_residuals(vals).into_iter().fold(0.0, f64::max)
    }

    /// Owners of equations that are violated.
    pub fn violated(&self, vals: &[f64]) -> Vec<Owner> {
        self.scaled_residuals(vals).into_iter().enumerate().filter(|(_, r)| !(*r < 1e-6)).map(|(k, _)| self.eqs[k].owner).collect()
    }

    /// Solves all constraints, pulling drag equations toward zero.
    /// Returns false when the constraints could not be satisfied.
    pub fn solve(&self, vals: &mut [f64], drags: &[DragEq], target: [f64; 2]) -> bool {
        let comps = self.components(drags);
        let mut ok = true;
        let mut st = Vec::new();
        for c in comps {
            if c.eqs.is_empty() && c.drags.is_empty() {
                continue;
            }
            let need = !c.drags.is_empty() || {
                let scales = self.eq_scales(&c.eqs, vals, target, c.cols.iter().copied());
                let ld = loader(vals, target);
                c.eqs.iter().zip(&scales).any(|(&i, s)| !(self.eqs[i].f.eval(&ld, &mut st).abs() / s < TOL))
            };
            if !need {
                continue;
            }
            let backup: Vec<f64> = c.cols.iter().map(|&k| vals[self.unknowns[k]]).collect();
            if !c.drags.is_empty() {
                self.lm(vals, &c, drags, target, 1e3, 40);
            }
            let mut good = self.project(vals, &c, target);
            if !good {
                // least-squares recovery, then project again
                self.lm(vals, &c, drags, target, 1.0, 60);
                good = self.project(vals, &c, target);
            }
            if !good {
                // from a double root (the foci of an ellipse with a = b) the
                // gradients give no direction to leave it by: start a little
                // way off
                let l = backup.iter().map(|x| x.abs()).filter(|x| x.is_finite()).fold(1e-9, f64::max);
                for (j, &k) in c.cols.iter().enumerate() {
                    vals[self.unknowns[k]] = backup[j] + 1e-4 * l * ((k as f64 + 1.0) * 78.233).sin();
                }
                good = self.project(vals, &c, target);
            }
            if !good {
                for (j, &k) in c.cols.iter().enumerate() {
                    vals[self.unknowns[k]] = backup[j];
                }
                ok = false;
            }
        }
        ok
    }

    fn rows(&self, c: &Comp, drags: &[DragEq], with_drags: bool) -> Vec<(bool, usize)> {
        let mut r: Vec<(bool, usize)> = c.eqs.iter().map(|&i| (false, i)).collect();
        if with_drags {
            r.extend(c.drags.iter().map(|&i| (true, i)));
        }
        let _ = drags;
        r
    }

    /// Levenberg–Marquardt on hard equations (weighted by `w`) plus drag equations.
    fn lm(&self, vals: &mut [f64], c: &Comp, drags: &[DragEq], target: [f64; 2], w: f64, iters: usize) {
        let n = c.cols.len();
        let local: HashMap<usize, usize> = c.cols.iter().enumerate().map(|(j, k)| (*k, j)).collect();
        let rows = self.rows(c, drags, true);
        let m = rows.len();
        let mut st = Vec::new();
        let mut r = vec![0.0; m];
        let mut jm = vec![0.0; m * n];
        let mut lambda = 1e-3;
        let eval_r = |vals: &[f64], r: &mut [f64], st: &mut Vec<f64>| -> f64 {
            let ld = loader(vals, target);
            let mut s = 0.0;
            for (i, (is_drag, k)) in rows.iter().enumerate() {
                let v = if *is_drag { drags[*k].f.eval(&ld, st) } else { w * self.eqs[*k].f.eval(&ld, st) };
                r[i] = v;
                s += v * v;
            }
            s
        };
        let mut cost = eval_r(vals, &mut r, &mut st);
        if !cost.is_finite() {
            return;
        }
        let cost0 = cost;
        let l = self.length_scale(vals, c.cols.iter().copied());
        for _ in 0..iters {
            jm.iter_mut().for_each(|x| *x = 0.0);
            {
                let ld = loader(vals, target);
                for (i, (is_drag, k)) in rows.iter().enumerate() {
                    let (jac, scale) = if *is_drag { (&drags[*k].jac, 1.0) } else { (&self.eqs[*k].jac, w) };
                    for (col, code) in jac {
                        if let Some(&j) = local.get(col) {
                            jm[i * n + j] = scale * code.eval(&ld, &mut st);
                        }
                    }
                }
            }
            let mut a = vec![0.0; n * n];
            let mut g = vec![0.0; n];
            for i in 0..m {
                let row = &jm[i * n..(i + 1) * n];
                for p in 0..n {
                    if row[p] == 0.0 {
                        continue;
                    }
                    g[p] += row[p] * r[i];
                    for q in 0..n {
                        a[p * n + q] += row[p] * row[q];
                    }
                }
            }
            let mut improved = false;
            // a uniform floor keeps weakly constrained parameters (e.g. a point
            // sliding along an infinite line) from taking large cheap steps
            let mean_diag = (0..n).map(|p| a[p * n + p]).sum::<f64>() / n.max(1) as f64;
            for _ in 0..12 {
                let mut aa = a.clone();
                for p in 0..n {
                    aa[p * n + p] += lambda * (a[p * n + p] + 0.1 * mean_diag + (1e-6 * mean_diag).max(1e-300));
                }
                let mut d: Vec<f64> = g.iter().map(|x| -x).collect();
                if !spd_solve(&mut aa, n, &mut d) {
                    lambda *= 10.0;
                    continue;
                }
                let old: Vec<f64> = c.cols.iter().map(|&k| vals[self.unknowns[k]]).collect();
                for (j, &k) in c.cols.iter().enumerate() {
                    vals[self.unknowns[k]] += d[j];
                }
                let mut r2 = vec![0.0; m];
                let c2 = eval_r(vals, &mut r2, &mut st);
                if c2.is_finite() && c2 < cost {
                    cost = c2;
                    r = r2;
                    lambda = (lambda * 0.3).max(1e-12);
                    improved = true;
                    let step: f64 = d.iter().map(|x| x * x).sum::<f64>().sqrt();
                    if step < 1e-13 * l {
                        return;
                    }
                    break;
                }
                for (j, &k) in c.cols.iter().enumerate() {
                    vals[self.unknowns[k]] = old[j];
                }
                lambda *= 8.0;
            }
            if !improved || cost < 1e-28 * cost0 {
                return;
            }
        }
    }

    /// Minimum-norm Newton iteration onto the hard constraints.
    fn project(&self, vals: &mut [f64], c: &Comp, target: [f64; 2]) -> bool {
        let n = c.cols.len();
        let m = c.eqs.len();
        if m == 0 {
            return true;
        }
        let local: HashMap<usize, usize> = c.cols.iter().enumerate().map(|(j, k)| (*k, j)).collect();
        let mut st = Vec::new();
        let mut r = vec![0.0; m];
        // the equations' scales at the start (mx is relative to them)
        let scales = self.eq_scales(&c.eqs, vals, target, c.cols.iter().copied());
        // far from unit size, length equations (|AB|² − d², scale ~ size²)
        // and angle equations (scale ~ 1) differ by orders of magnitude, and
        // the damping drowns the small rows: then the Newton rows are scaled
        // to their equations' scales. At ordinary sizes they are not, which
        // finds the nearer solution after a large move.
        let size = self.length_scale(vals, c.cols.iter().copied());
        let rows: Vec<f64> = if (1e-2..=1e2).contains(&size) { vec![1.0; m] } else { scales.clone() };
        let eval_r = |vals: &[f64], r: &mut [f64], st: &mut Vec<f64>| -> (f64, f64) {
            let ld = loader(vals, target);
            let mut s = 0.0;
            let mut mx: f64 = 0.0;
            for (i, &k) in c.eqs.iter().enumerate() {
                let v = self.eqs[k].f.eval(&ld, st);
                r[i] = v;
                s += v * v;
                mx = if v.is_nan() { f64::INFINITY } else { mx.max(v.abs() / scales[i]) };
            }
            (s, mx)
        };
        let (mut cost, mut mx) = eval_r(vals, &mut r, &mut st);
        let mut jm = vec![0.0; m * n];
        // after reaching TOL, a few more Newton steps reach machine precision
        // whatever the scale of the construction; at a double root (a
        // tangency, a discriminant 0) Newton only halves the error per step,
        // so the polishing goes on while the steps still halve the residual
        let mut polish = 0;
        let mut polished_cost = f64::INFINITY;
        for _ in 0..120 {
            // (numeric work advances the work clock too)
            charge((m * n / 4 + m * m * m / 200 + 20) as u64);
            if mx < TOL {
                polish += 1;
                if mx == 0.0 || (polish > 3 && !(cost < 0.5 * polished_cost)) || polish > 60 {
                    return true;
                }
                polished_cost = cost;
            }
            if !cost.is_finite() {
                return false;
            }
            jm.iter_mut().for_each(|x| *x = 0.0);
            {
                let ld = loader(vals, target);
                for (i, &k) in c.eqs.iter().enumerate() {
                    for (col, code) in &self.eqs[k].jac {
                        if let Some(&j) = local.get(col) {
                            jm[i * n + j] = code.eval(&ld, &mut st) / rows[i];
                        }
                    }
                }
            }
            let mut mm = vec![0.0; m * m];
            let mut trace = 0.0;
            for i in 0..m {
                for k in i..m {
                    let mut s = 0.0;
                    for j in 0..n {
                        s += jm[i * n + j] * jm[k * n + j];
                    }
                    mm[i * m + k] = s;
                    mm[k * m + i] = s;
                }
                trace += mm[i * m + i];
            }
            let mu = 1e-16 * (trace / m as f64) + 1e-300;
            let ys: Vec<f64> = r.iter().zip(&rows).map(|(x, s)| x / s).collect();
            let mut y = ys.clone();
            let mut ok = false;
            let mut damp = mu;
            for _ in 0..8 {
                let mut a = mm.clone();
                for i in 0..m {
                    a[i * m + i] += damp;
                }
                y.copy_from_slice(&ys);
                if spd_solve(&mut a, m, &mut y) {
                    ok = true;
                    break;
                }
                damp = damp * 1e3 + 1e-12 * (trace / m as f64);
            }
            if !ok {
                return false;
            }
            let mut d = vec![0.0; n];
            for i in 0..m {
                for j in 0..n {
                    d[j] -= jm[i * n + j] * y[i];
                }
            }
            let old: Vec<f64> = c.cols.iter().map(|&k| vals[self.unknowns[k]]).collect();
            let mut alpha = 1.0;
            let mut accepted = false;
            for _ in 0..30 {
                for (j, &k) in c.cols.iter().enumerate() {
                    vals[self.unknowns[k]] = old[j] + alpha * d[j];
                }
                let mut r2 = vec![0.0; m];
                let (c2, mx2) = eval_r(vals, &mut r2, &mut st);
                let enough = if polish > 0 { c2 < cost } else { c2 < cost * (1.0 - 1e-4 * alpha) || mx2 < TOL };
                if c2.is_finite() && enough {
                    cost = c2;
                    mx = mx2;
                    r = r2;
                    accepted = true;
                    break;
                }
                alpha *= 0.5;
            }
            if !accepted {
                for (j, &k) in c.cols.iter().enumerate() {
                    vals[self.unknowns[k]] = old[j];
                }
                return mx < 1e-7;
            }
        }
        mx < 1e-7
    }

    /// Jacobian rank analysis: which unknowns remain free.
    pub fn analyze(&self, vals: &[f64]) -> Analysis {
        let n = self.unknowns.len();
        let m = self.eqs.len();
        let mut free = vec![true; n];
        let comps = self.components(&[]);
        let mut st = Vec::new();
        let ld = loader(vals, [0.0; 2]);
        let mut dof = 0;
        for c in comps {
            let cn = c.cols.len();
            let cm = c.eqs.len();
            if cm == 0 {
                dof += cn;
                continue;
            }
            let local: HashMap<usize, usize> = c.cols.iter().enumerate().map(|(j, k)| (*k, j)).collect();
            let mut a = vec![0.0; cm * cn];
            for (i, &k) in c.eqs.iter().enumerate() {
                for (col, code) in &self.eqs[k].jac {
                    if let Some(&j) = local.get(col) {
                        a[i * cn + j] = code.eval(&ld, &mut st);
                    }
                }
            }
            // a gradient that vanishes here (a double root, which the solver
            // reaches only to about √ε) is tiny next to the same gradient a
            // little way off, where an ordinary one barely changes: it is
            // taken as 0, not normalized into a full row
            let l = self.length_scale(vals, c.cols.iter().copied());
            let mut off = vals.to_vec();
            let w: Vec<f64> = (0..cn).map(|j| ((j as f64 + 1.0) * 12.9898).sin()).collect();
            let wn = w.iter().map(|x| x * x).sum::<f64>().sqrt();
            for (j, &k) in c.cols.iter().enumerate() {
                off[self.unknowns[k]] += 1e-3 * l * w[j] / wn;
            }
            let ld_off = loader(&off, [0.0; 2]);
            for (i, &k) in c.eqs.iter().enumerate() {
                let s: f64 = a[i * cn..(i + 1) * cn].iter().map(|x| x * x).sum::<f64>().sqrt();
                let s_off: f64 = self.eqs[k].jac.iter().filter(|(col, _)| local.contains_key(col)).map(|(_, code)| code.eval(&ld_off, &mut st).powi(2)).sum::<f64>().sqrt();
                if s < 1e-2 * s_off {
                    a[i * cn..(i + 1) * cn].iter_mut().for_each(|x| *x = 0.0);
                }
            }
            // normalize rows
            for i in 0..cm {
                let s: f64 = a[i * cn..(i + 1) * cn].iter().map(|x| x * x).sum::<f64>().sqrt();
                if s > 0.0 {
                    a[i * cn..(i + 1) * cn].iter_mut().for_each(|x| *x /= s);
                }
            }
            // reduced row echelon form
            let mut pivots: Vec<usize> = Vec::new();
            let mut row = 0;
            for col in 0..cn {
                if row >= cm {
                    break;
                }
                let (mut best, mut bv) = (row, 0.0);
                for i in row..cm {
                    let v = a[i * cn + col].abs();
                    if v > bv {
                        bv = v;
                        best = i;
                    }
                }
                if bv < 1e-8 {
                    continue;
                }
                for j in 0..cn {
                    a.swap(row * cn + j, best * cn + j);
                }
                let p = a[row * cn + col];
                for j in 0..cn {
                    a[row * cn + j] /= p;
                }
                for i in 0..cm {
                    if i != row {
                        let f = a[i * cn + col];
                        if f != 0.0 {
                            for j in 0..cn {
                                a[i * cn + j] -= f * a[row * cn + j];
                            }
                        }
                    }
                }
                pivots.push(col);
                row += 1;
            }
            let is_pivot: Vec<bool> = (0..cn).map(|j| pivots.contains(&j)).collect();
            // with dependent rows, a column without a pivot can still be fixed
            // to higher order: at a double root (|BC|² = 0, the foci of an
            // ellipse with a = b) the gradients vanish, at a triple one (a
            // tangent at an inflection) the Jacobian loses rank. Held at a
            // value 2% of the size away, a free column leaves the others
            // solvable to rounding; a fixed one leaves a residual of the
            // order of the move squared or cubed.
            let mut pinned = vec![false; cn];
            if cm > pivots.len() {
                for j in (0..cn).filter(|&j| !is_pivot[j]) {
                    let held = Comp { eqs: c.eqs.clone(), drags: vec![], cols: c.cols.iter().copied().filter(|&k| k != c.cols[j]).collect() };
                    let mut v = vals.to_vec();
                    v[self.unknowns[c.cols[j]]] += 2e-2 * l;
                    self.project(&mut v, &held, [0.0; 2]);
                    let scales = self.eq_scales(&c.eqs, &v, [0.0; 2], c.cols.iter().copied());
                    let ld_v = loader(&v, [0.0; 2]);
                    let worst = c.eqs.iter().zip(&scales).map(|(&k, sc)| self.eqs[k].f.eval(&ld_v, &mut st).abs() / sc).fold(0.0, |a: f64, b| if b.is_nan() { f64::INFINITY } else { a.max(b) });
                    pinned[j] = worst > 1e2 * TOL;
                }
            }
            let rank = pivots.len() + pinned.iter().filter(|p| **p).count();
            dof += cn - rank;
            let mut f = vec![false; cn];
            for j in 0..cn {
                if !is_pivot[j] && !pinned[j] {
                    f[j] = true;
                    // pivot variables depending on this free column are free too
                    for (r, &pc) in pivots.iter().enumerate() {
                        if a[r * cn + j].abs() > 1e-8 {
                            f[pc] = true;
                        }
                    }
                }
            }
            for (j, &k) in c.cols.iter().enumerate() {
                free[k] = f[j];
            }
        }
        let _ = m;
        Analysis { free, dof }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::*;

    #[test]
    fn triangle_resize() {
        let mut d = Doc::new();
        let a = d.add_point(-2.67, -2.83);
        let b = d.add_point(4.67, -2.83);
        let c = d.add_point(-0.33, 2.83);
        let l1 = d.add_ent(Ent::Line { a, b, infinite: false });
        let l2 = d.add_ent(Ent::Line { a: b, b: c, infinite: false });
        let l3 = d.add_ent(Ent::Line { a: c, b: a, infinite: false });
        d.cons.push(Some(Con::Horizontal(a, b)));
        let d1 = d.add_dim(DimKind::Length(l1), [0.0, 0.0]);
        d.add_dim(DimKind::Length(l2), [0.0, 0.0]);
        d.add_dim(DimKind::Length(l3), [0.0, 0.0]);
        let p = d.dims[d1].as_ref().unwrap().param;
        d.params[p].v = 5.0;
        let (eqns, _) = d.equations();
        let sys = super::Sys::build(&eqns, d.unknowns());
        let mut vals: Vec<f64> = d.params.iter().map(|p| p.v).collect();
        let ok = sys.solve(&mut vals, &[], [0.0; 2]);
        println!("ok={ok} resid={}", sys.max_residual(&vals));
        assert!(ok);
    }
}
