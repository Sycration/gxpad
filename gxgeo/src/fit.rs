//! Formula discovery from samples.
//!
//! The target of a derivation is an algebraic function of the driving
//! parameters (lengths, cos and sin of angles). Elimination finds its
//! minimal polynomial symbolically and can drown in intermediate terms or in
//! degenerate components. This module finds the same polynomial from numbers:
//! the construction is solved numerically at many parameter values near the
//! current configuration, and a polynomial relation
//!
//!   Σ c · T^k · (monomial in the parameters) · (cos / sin of the angles) = 0
//!
//! with exact coefficients c is fitted to the samples (the one-dimensional
//! null space of the matrix of basis functions). Candidate bases are tried
//! from small to large: degree of T first (1, 2, 4), then total degree.
//!
//! A smooth function is approximated very well by a modest basis, so in f64
//! an exact relation and a good approximation look alike. The samples are
//! therefore refined to double-double precision (about 31 digits) and the
//! null space is computed in that precision: an exact relation then leaves a
//! residual near 1e-28, an approximation one many orders larger, and the
//! coefficients come out accurate enough to recognize fractions and
//! quadratic irrationals with certainty.
//!
//! The relation is checked against samples that took no part in the fit,
//! and the caller solves it for T and checks the formula again.

use crate::dd::{Dd, DdCode};
use crate::expr::*;
use crate::model::Owner;
use crate::solver::Sys;
use std::collections::{HashMap, HashSet};

pub struct Problem<'a> {
    /// The constraint equations (each = 0) and their unknowns.
    pub rows: &'a [Expr],
    pub unknowns: &'a [u32],
    pub target: &'a Expr,
    /// Current values of every symbol.
    pub vals: &'a HashMap<u32, f64>,
    /// Symbols that scale with the construction (lengths, radii).
    pub lengths: &'a HashSet<u32>,
    /// Largest basis to try.
    pub max_cols: usize,
    /// Symbol standing for the target in the relation.
    pub t: u32,
}

/// One solved configuration: parameter values and the target's value there.
#[derive(Clone)]
pub struct Sample {
    pub params: Vec<(u32, f64)>,
    pub value: f64,
    exact: Dd,
}

pub enum Found {
    /// The target itself.
    Formula(Expr),
    /// A polynomial in the symbol `Problem::t` that vanishes at the target
    /// (or at its cosine with `cos`).
    Relation { p: Expr, cos: bool },
}

pub struct Fit {
    pub found: Found,
    /// Samples that took no part in the fit.
    pub checks: Vec<Sample>,
}

/// Whether `formula` reproduces the target at every check sample.
pub fn agrees(formula: &Expr, checks: &[Sample], base: &HashMap<u32, f64>) -> bool {
    !checks.is_empty()
        && checks.iter().all(|s| {
            let v = eval(formula, &|x| s.params.iter().find(|p| p.0 == x).map(|p| p.1).or_else(|| base.get(&x).copied()).unwrap_or(f64::NAN));
            v.is_finite() && (v - s.value).abs() <= 1e-8 * (1.0 + s.value.abs())
        })
}

struct Sampler {
    sys: Sys,
    /// The equations and the target for double-double evaluation.
    eqs: Vec<DdCode>,
    target: DdCode,
    base: Vec<f64>,
    code: Code,
    rng: u64,
    /// Solved configurations so far (parameter values, all values): a new
    /// sample continues from the nearest one.
    states: Vec<(Vec<(u32, f64)>, Vec<f64>)>,
}

impl Sampler {
    fn rand(&mut self) -> f64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 11) as f64 / (1u64 << 53) as f64
    }

    /// The configuration with the parameters moved to `pv`, by continuation
    /// from the nearest solved configuration (so the solution stays on the
    /// branch of the current one), and the target's value there.
    fn solve(&mut self, pv: &[(u32, f64)]) -> Option<(Vec<f64>, f64)> {
        let dist = |from: &[f64]| pv.iter().map(|(p, x)| (x - from[*p as usize]).abs() / (1.0 + self.base[*p as usize].abs())).fold(0.0, f64::max);
        let mut start = &self.base;
        let mut far = dist(start);
        for (spv, v) in &self.states {
            // only states that moved the same parameters
            if spv.len() == pv.len() && spv.iter().zip(pv).all(|(a, b)| a.0 == b.0) {
                let d = dist(v);
                if d < far {
                    far = d;
                    start = v;
                }
            }
        }
        let from = start.clone();
        let mut v = from.clone();
        let steps = ((far / 0.08).ceil() as usize).clamp(1, 8);
        for k in 1..=steps {
            for (p, x) in pv {
                let x0 = from[*p as usize];
                // (the last step lands on the value exactly)
                v[*p as usize] = if k == steps { *x } else { x0 + (x - x0) * k as f64 / steps as f64 };
            }
            if !self.sys.solve(&mut v, &[], [0.0; 2]) {
                return None;
            }
        }
        // the solver accepts loose solutions near a singular configuration
        if !(self.sys.max_residual(&v) < crate::solver::TOL) {
            return None;
        }
        let mut st = Vec::new();
        let t = self.code.eval(&|s: u32| v.get(s as usize).copied().unwrap_or(f64::NAN), &mut st);
        if !t.is_finite() {
            return None;
        }
        if self.states.len() < 400 {
            self.states.push((pv.to_vec(), v.clone()));
        }
        Some((v, t))
    }

    fn at(&mut self, pv: &[(u32, f64)]) -> Option<f64> {
        self.solve(pv).map(|r| r.1)
    }

    /// Like `at`, with the target to double-double precision: Newton steps
    /// with residuals in double-double arithmetic and the f64 Jacobian.
    fn exact_at(&mut self, pv: &[(u32, f64)]) -> Option<(f64, Dd)> {
        let (v, t) = self.solve(pv)?;
        let (m, n) = (self.sys.eqs.len(), self.sys.unknowns.len());
        charge((m * n + m * m * m / 50 + 40) as u64);
        let mut x: Vec<Dd> = v.iter().map(|a| Dd::from(*a)).collect();
        if m > 0 && n > 0 {
            let mut st = Vec::new();
            let ld = |s: u32| v.get(s as usize).copied().unwrap_or(f64::NAN);
            let mut jm = vec![0.0; m * n];
            for (i, e) in self.sys.eqs.iter().enumerate() {
                for (col, code) in &e.jac {
                    jm[i * n + col] = code.eval(&ld, &mut st);
                }
            }
            // rows scaled to unit length, so the factorization shows the conditioning
            let rs: Vec<f64> = (0..m).map(|i| (0..n).map(|j| jm[i * n + j] * jm[i * n + j]).sum::<f64>().sqrt().max(1e-300)).collect();
            for i in 0..m {
                for j in 0..n {
                    jm[i * n + j] /= rs[i];
                }
            }
            let mut mm = vec![0.0; m * m];
            let mut trace = 0.0;
            for i in 0..m {
                for k in i..m {
                    let s: f64 = (0..n).map(|j| jm[i * n + j] * jm[k * n + j]).sum();
                    mm[i * m + k] = s;
                    mm[k * m + i] = s;
                }
                trace += mm[i * m + i];
            }
            let damp = 1e-14 * (trace / m as f64) + 1e-300;
            for i in 0..m {
                mm[i * m + i] += damp;
            }
            let mut dst = Vec::new();
            let mut last = f64::INFINITY;
            for _ in 0..8 {
                let r: Vec<Dd> = self.eqs.iter().map(|e| e.eval(&|s: u32| x.get(s as usize).copied().unwrap_or(Dd::from(f64::NAN)), &mut dst)).collect();
                let mx = r.iter().map(|a| a.hi.abs()).fold(0.0, f64::max);
                if !mx.is_finite() {
                    return None;
                }
                if mx >= last {
                    break;
                }
                last = mx;
                if mx < 1e-29 {
                    break;
                }
                let mut y: Vec<f64> = r.iter().zip(&rs).map(|(a, s)| a.to_f64() / s).collect();
                let mut a = mm.clone();
                if !crate::solver::spd_solve(&mut a, m, &mut y) {
                    return None;
                }
                // near a singular configuration the solution is not determined
                // well enough, however small the residual
                let lmin = (0..m).map(|i| a[i * m + i]).fold(f64::INFINITY, f64::min);
                if !(lmin > 1e-5) {
                    return None;
                }
                for j in 0..n {
                    let d: f64 = (0..m).map(|i| jm[i * n + j] * y[i]).sum();
                    let u = self.sys.unknowns[j];
                    x[u] = x[u] - Dd::from(d);
                }
            }
            // far from converged: the Jacobian is too ill-conditioned here
            if !(last < 1e-22) {
                if std::env::var("GXGEO_DEBUG").is_ok() {
                    eprintln!("fit: refinement stopped at residual {last:.1e}");
                }
                return None;
            }
        }
        let mut dst = Vec::new();
        let td = self.target.eval(&|s: u32| x.get(s as usize).copied().unwrap_or(Dd::from(f64::NAN)), &mut dst);
        if !td.is_finite() || (td.hi - t).abs() > 1e-9 * (1.0 + t.abs()) {
            return None;
        }
        Some((t, td))
    }
}

/// The argument of a trigonometric function without its constant part and
/// with a positive leading sign.
fn canon_arg(a: &Expr) -> Expr {
    let ts: Vec<Expr> = match a.node() {
        Node::Add(ts) => ts.iter().filter(|t| !sym_set(t).is_empty()).cloned().collect(),
        _ => vec![a.clone()],
    };
    let r = add(ts);
    if r.is_negative_form() { neg(r) } else { r }
}

/// Collects the arguments of sin, cos and tan that are free of unknowns
/// (`args`), and the symbols that occur anywhere else (`outside`).
fn scan(e: &Expr, unknown: &HashSet<u32>, args: &mut Vec<Expr>, outside: &mut HashSet<u32>) {
    match e.node() {
        Node::Fun(Fun::Sin | Fun::Cos | Fun::Tan, a) if !sym_set(a).is_empty() && sym_set(a).is_disjoint(unknown) => {
            let c = canon_arg(a);
            if !args.contains(&c) {
                args.push(c);
            }
        }
        Node::Sym(s) => {
            outside.insert(*s);
        }
        Node::Num(_) | Node::Pi => {}
        Node::Add(ts) | Node::Mul(ts) => ts.iter().for_each(|t| scan(t, unknown, args, outside)),
        Node::Pow(a, b) | Node::Atan2(a, b) => {
            scan(a, unknown, args, outside);
            scan(b, unknown, args, outside);
        }
        Node::Fun(_, a) => scan(a, unknown, args, outside),
    }
}

/// Pivot sizes of a Householder QR with column pivoting of the m×n matrix
/// `a` (row-major, columns scaled to unit length), in f64: a cheap first
/// look that tells a clearly full-rank matrix.
fn smallest_pivot(a: &[f64], m: usize, n: usize) -> f64 {
    let mut a = a.to_vec();
    let mut smallest = f64::INFINITY;
    for k in 0..n.min(m) {
        let (mut best, mut bn) = (k, -1.0);
        for j in k..n {
            let s: f64 = (k..m).map(|i| a[i * n + j] * a[i * n + j]).sum();
            if s > bn {
                bn = s;
                best = j;
            }
        }
        let norm = bn.max(0.0).sqrt();
        smallest = smallest.min(norm);
        if norm < 1e-7 {
            return norm;
        }
        if best != k {
            for i in 0..m {
                a.swap(i * n + k, i * n + best);
            }
        }
        let alpha = if a[k * n + k] >= 0.0 { -norm } else { norm };
        let mut v: Vec<f64> = (k..m).map(|i| a[i * n + k]).collect();
        v[0] -= alpha;
        let vn: f64 = v.iter().map(|x| x * x).sum();
        if vn > 0.0 {
            for j in k..n {
                let d: f64 = (k..m).map(|i| v[i - k] * a[i * n + j]).sum::<f64>() * 2.0 / vn;
                for i in k..m {
                    a[i * n + j] -= d * v[i - k];
                }
            }
        }
    }
    smallest
}

/// Householder QR with column pivoting of the m×n matrix `a` (row-major) in
/// double-double arithmetic. Returns the null vector when the numerical
/// null space has dimension one, Err(dimension) otherwise (0: none, or not
/// clearly separated).
fn null_vector(a: &mut [Dd], m: usize, n: usize) -> Result<Vec<Dd>, usize> {
    let dbg = std::env::var("GXGEO_DEBUG").is_ok();
    let mut scale = vec![Dd::ONE; n];
    let mut plain = vec![0.0; m * n];
    for j in 0..n {
        let s = (0..m).fold(Dd::ZERO, |s, i| s + a[i * n + j] * a[i * n + j]).sqrt();
        if s.hi > 0.0 {
            scale[j] = s;
        }
        for i in 0..m {
            a[i * n + j] = a[i * n + j] / scale[j];
            plain[i * n + j] = a[i * n + j].hi;
        }
    }
    if m < n {
        return Err(0);
    }
    let rough = smallest_pivot(&plain, m, n);
    if rough >= 1e-7 {
        if dbg && n > 2 {
            eprintln!("fit: {n} columns: full rank (smallest pivot {rough:.1e})");
        }
        return Err(0);
    }
    // residues of an exact relation are near 1e-28; anything above NOISE is
    // a real (if tiny) component
    const NOISE: f64 = 1e-24;
    let mut perm: Vec<usize> = (0..n).collect();
    let mut rank = n;
    let mut last_good = 1.0;
    // squared norms of the remaining parts of the columns: downdated after
    // each step, and computed afresh whenever they have shrunk so far that
    // the downdated values have lost their digits
    let fresh = |a: &[Dd], k: usize, j: usize| (k..m).fold(Dd::ZERO, |s, i| s + a[i * n + j] * a[i * n + j]).hi;
    let mut cn: Vec<f64> = (0..n).map(|j| fresh(a, 0, j)).collect();
    let mut at_fresh = cn.clone();
    for k in 0..n {
        let pick = |cn: &[f64]| (k..n).fold((k, -1.0), |b, j| if cn[j] > b.1 { (j, cn[j]) } else { b });
        let (mut best, mut bn) = pick(&cn);
        if bn < 1e-6 * at_fresh[best] {
            for j in k..n {
                cn[j] = fresh(a, k, j);
                at_fresh[j] = cn[j];
            }
            (best, bn) = pick(&cn);
        }
        let norm = bn.max(0.0).sqrt();
        if norm < NOISE {
            rank = k;
            break;
        }
        last_good = norm;
        if best != k {
            perm.swap(k, best);
            cn.swap(k, best);
            at_fresh.swap(k, best);
            for i in 0..m {
                a.swap(i * n + k, i * n + best);
            }
        }
        let x0 = a[k * n + k];
        let nd = (k..m).fold(Dd::ZERO, |s, i| s + a[i * n + k] * a[i * n + k]).sqrt();
        let alpha = if x0.hi >= 0.0 { -nd } else { nd };
        let mut v: Vec<Dd> = (k..m).map(|i| a[i * n + k]).collect();
        v[0] = v[0] - alpha;
        let vn = v.iter().fold(Dd::ZERO, |s, x| s + *x * *x);
        if vn.hi > 0.0 {
            let f = Dd::from(2.0) / vn;
            for j in k..n {
                let d = (k..m).fold(Dd::ZERO, |s, i| s + v[i - k] * a[i * n + j]) * f;
                for i in k..m {
                    a[i * n + j] = a[i * n + j] - d * v[i - k];
                }
                if j > k {
                    let h = a[k * n + j].hi;
                    cn[j] -= h * h;
                }
            }
        }
        charge(((m - k) * (n - k) / 3 + 1) as u64);
        if past_deadline() {
            return Err(0);
        }
    }
    if rank == n {
        if dbg {
            eprintln!("fit: {n} columns: full rank (smallest pivot {last_good:.1e})");
        }
        return Err(0);
    }
    if rank + 1 < n {
        return Err(n - rank);
    }
    // the null vector is as accurate as the noise over the smallest pivot
    if last_good < 1e-14 {
        if dbg {
            eprintln!("fit: {n} columns: null vector too inaccurate (smallest pivot {last_good:.1e})");
        }
        return Err(0);
    }
    // R y = -R[.., last], back-substitution
    let mut y = vec![Dd::ZERO; n];
    y[n - 1] = Dd::ONE;
    for i in (0..rank).rev() {
        let mut s = a[i * n + (n - 1)];
        for j in i + 1..rank {
            s = s + a[i * n + j] * y[j];
        }
        y[i] = -s / a[i * n + i];
    }
    let mut x = vec![Dd::ZERO; n];
    for (k, &j) in perm.iter().enumerate() {
        x[j] = y[k] / scale[j];
    }
    Ok(x)
}

/// The simplest fraction p/q, q ≤ maxd, that equals x to working precision.
fn frac(x: Dd, maxd: i128) -> Option<Rat> {
    let xf = x.to_f64();
    let (mut h0, mut h1, mut k0, mut k1) = (0i128, 1i128, 1i128, 0i128);
    let mut y = xf;
    for _ in 0..30 {
        let a = y.floor();
        if a.abs() > 1e12 {
            return None;
        }
        let a = a as i128;
        let (h2, k2) = (a * h1 + h0, a * k1 + k0);
        if k2 > maxd {
            return None;
        }
        if ((h2 as f64) / (k2 as f64) - xf).abs() <= 1e-11 * (1e-3 + xf.abs()) {
            // the f64 match must hold far beyond f64 precision
            let d = (x * Dd::from(k2 as f64) - Dd::from(h2 as f64)).abs().hi;
            return (d <= COEF_TOL * (1e-3 + xf.abs()) * k2 as f64).then(|| Rat::new(h2, k2)).flatten();
        }
        (h0, h1, k0, k1) = (h1, h2, k1, k2);
        let f = y - y.floor();
        if f.abs() < 1e-13 {
            return None;
        }
        y = 1.0 / f;
    }
    None
}

/// Relative accuracy a recognized coefficient must have.
const COEF_TOL: f64 = 1e-19;

/// A root of a·y² + b·y − c = 0 with small integers that equals y.
fn quadratic_irrational(y: Dd) -> Option<Expr> {
    let yf = y.to_f64();
    if !(yf.abs() > 1e-6 && yf.abs() < 1e4) {
        return None;
    }
    for a in 1..=40i64 {
        for b in -400..=400i64 {
            let v = a as f64 * yf * yf + b as f64 * yf;
            let c = v.round();
            let mag = a as f64 * yf * yf + (b as f64 * yf).abs() + 1.0;
            if (v - c).abs() > 1e-10 * mag || c.abs() > 1e6 {
                continue;
            }
            // as an error of the root: the residual over the derivative
            // (a double root would hide any error)
            let d = (Dd::from(a as f64) * y * y + Dd::from(b as f64) * y - Dd::from(c)).abs().hi;
            let slope = (2.0 * a as f64 * yf + b as f64).abs();
            if slope < 1e-6 * mag || !(d <= COEF_TOL * yf.abs() * slope) {
                continue;
            }
            let c = c as i64;
            let disc = b * b + 4 * a * c;
            if disc < 0 {
                continue;
            }
            let sd = sqrt(int(disc));
            let cands = [div(add(vec![int(-b), sd.clone()]), int(2 * a)), div(sub(int(-b), sd), int(2 * a))];
            return cands.into_iter().find(|e| (eval(e, &|_| 0.0) - yf).abs() <= 1e-12 * (1.0 + yf.abs()));
        }
    }
    None
}

/// A coefficient as an exact number: a fraction, a square root of one, a
/// quadratic irrational, or a square root of one.
fn nice_coef(x: Dd) -> Option<Expr> {
    if let Some(r) = frac(x, 20000) {
        return Some(num(Num::R(r)));
    }
    let signed = |e: Expr| if x.hi < 0.0 { neg(e) } else { e };
    if let Some(r) = frac(x * x, 2000) {
        return Some(signed(sqrt(num(Num::R(r)))));
    }
    if let Some(e) = quadratic_irrational(x) {
        return Some(e);
    }
    quadratic_irrational(x * x).map(|e| signed(sqrt(e)))
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Shape {
    Lin,
    Sq,
    Quad,
    Biq,
}

impl Shape {
    fn powers(self) -> &'static [u32] {
        match self {
            Shape::Lin => &[0, 1],
            Shape::Sq => &[0, 2],
            Shape::Quad => &[0, 1, 2],
            Shape::Biq => &[0, 2, 4],
        }
    }
}

#[derive(Clone, Debug)]
struct Term {
    k: u32,
    pe: Vec<u8>,
    /// Per angle: 0 for 1, 2h-1 for cos(h·A), 2h for sin(h·A).
    tr: Vec<u8>,
}

/// All exponent vectors over `n` variables of total degree exactly `d`
/// (`exact`) or at most `d`.
fn monomials(n: usize, d: u32, exact: bool) -> Vec<Vec<u8>> {
    fn rec(i: usize, n: usize, left: u32, exact: bool, cur: &mut Vec<u8>, out: &mut Vec<Vec<u8>>) {
        if i == n {
            if !exact || left == 0 {
                out.push(cur.clone());
            }
            return;
        }
        for e in 0..=left {
            cur.push(e as u8);
            rec(i + 1, n, left - e, exact, cur, out);
            cur.pop();
        }
    }
    let mut out = Vec::new();
    rec(0, n, d, exact, &mut Vec::new(), &mut out);
    out
}

/// How the target scales when the scaling atoms are multiplied by λ.
#[derive(Clone, Copy, Debug)]
enum Homog {
    /// T scales as λ^w; `scaling[i]` tells which atoms scale.
    Yes(u32),
    No,
}

struct Atoms {
    /// Polynomial atoms (parameters) and whether each scales with the construction.
    poly: Vec<u32>,
    scaling: Vec<bool>,
    /// Angle atoms (arguments of cos and sin).
    args: Vec<Expr>,
    homog: Homog,
}

/// The terms of a candidate relation. With `monic` the highest power of T
/// stands alone (T^k = a polynomial in the atoms and lower powers of T);
/// otherwise every power of T has a polynomial coefficient of matching
/// degree.
fn basis(at: &Atoms, shape: Shape, monic: bool, harmonics: u8, w: u32, cap: usize) -> Option<Vec<Term>> {
    let top = *shape.powers().last().unwrap();
    let s_idx: Vec<usize> = (0..at.poly.len()).filter(|&i| at.scaling[i]).collect();
    let f_idx: Vec<usize> = (0..at.poly.len()).filter(|&i| !at.scaling[i]).collect();
    let mut trig: Vec<Vec<u8>> = vec![vec![]];
    for _ in 0..at.args.len() {
        let mut next = Vec::new();
        for t in &trig {
            for h in 0..=2 * harmonics {
                let mut t2 = t.clone();
                t2.push(h);
                next.push(t2);
            }
        }
        trig = next;
        if trig.len() > cap {
            return None;
        }
    }
    let mut out = Vec::new();
    for &k in shape.powers() {
        let (sdeg, fdeg) = match (at.homog, monic) {
            (Homog::Yes(wt), true) => ((top - k) * wt, w),
            (Homog::No, true) => (0, w),
            (Homog::Yes(wt), false) => {
                if w < k * wt {
                    continue;
                }
                (w - k * wt, w)
            }
            (Homog::No, false) => {
                if w < k {
                    continue;
                }
                (0, w - k)
            }
        };
        if monic && k == top {
            out.push(Term { k, pe: vec![0; at.poly.len()], tr: vec![0; at.args.len()] });
            continue;
        }
        let sm = if matches!(at.homog, Homog::Yes(_)) { monomials(s_idx.len(), sdeg, true) } else { vec![vec![0; s_idx.len()]] };
        let fm = monomials(f_idx.len(), fdeg, false);
        for a in &sm {
            for b in &fm {
                let mut pe = vec![0u8; at.poly.len()];
                for (j, &i) in s_idx.iter().enumerate() {
                    pe[i] = a[j];
                }
                for (j, &i) in f_idx.iter().enumerate() {
                    pe[i] = b[j];
                }
                for tr in &trig {
                    out.push(Term { k, pe: pe.clone(), tr: tr.clone() });
                    if out.len() > cap {
                        return None;
                    }
                }
            }
        }
    }
    // a relation needs T in it and at least two terms
    (out.len() >= 2 && out.iter().any(|t| t.k > 0)).then_some(out)
}

/// Values of the atoms at a sample: T, the polynomial atoms, and for each
/// angle cos(hA), sin(hA) for h = 1..=2.
struct Row {
    t: Dd,
    p: Vec<Dd>,
    trig: Vec<[Dd; 5]>,
}

fn term_value(t: &Term, r: &Row) -> Dd {
    let mut v = r.t.powi(t.k as i32);
    for (i, &e) in t.pe.iter().enumerate() {
        if e > 0 {
            v = v * r.p[i].powi(e as i32);
        }
    }
    for (i, &h) in t.tr.iter().enumerate() {
        if h > 0 {
            v = v * r.trig[i][h as usize];
        }
    }
    v
}

fn term_expr(t: &Term, at: &Atoms, tsym: u32, half: bool) -> Expr {
    let mut fs = vec![pow(sym(tsym), int(t.k as i64))];
    for (i, &e) in t.pe.iter().enumerate() {
        if e > 0 {
            fs.push(pow(sym(at.poly[i]), int(e as i64)));
        }
    }
    for (i, &h) in t.tr.iter().enumerate() {
        if h == 0 {
            continue;
        }
        let mult = h.div_ceil(2) as i64;
        let a = if half { mul(vec![rat(mult, 2), at.args[i].clone()]) } else { mul(vec![int(mult), at.args[i].clone()]) };
        fs.push(if h % 2 == 1 { cos(a) } else { sin(a) });
    }
    mul(fs)
}

/// Searches for a formula or polynomial relation for the target of `pb`.
pub fn fit(pb: &Problem) -> Option<Fit> {
    let dbg = std::env::var("GXGEO_DEBUG").is_ok();
    let n = pb.vals.keys().filter(|k| **k < TMP_BASE).max().copied().unwrap_or(0) as usize + 1;
    let mut base = vec![0.0; n];
    for (k, v) in pb.vals {
        if (*k as usize) < n {
            base[*k as usize] = *v;
        }
    }
    let unknown: HashSet<u32> = pb.unknowns.iter().copied().collect();
    let owned: Vec<(Expr, Owner)> = pb.rows.iter().map(|e| (e.clone(), Owner::Ent(0))).collect();
    let sys = Sys::build(&owned, pb.unknowns.iter().map(|u| *u as usize).collect());
    let mut sm = Sampler {
        sys,
        eqs: pb.rows.iter().map(DdCode::new).collect(),
        target: DdCode::new(pb.target),
        base,
        code: Code::new(pb.target),
        rng: 0x2545_F491_4F6C_DD1D,
        states: Vec::new(),
    };
    let t0 = sm.at(&[])?;

    // angle targets (acos of something) are fitted through their cosine
    let is_angle = matches!(pb.target.node(), Node::Fun(Fun::Acos, _))
        || matches!(pb.target.node(), Node::Add(ts) if ts.iter().any(|t| matches!(t.node(), Node::Pi)) && ts.iter().any(|t| matches!(t.split_coeff().1.node(), Node::Fun(Fun::Acos, _))));

    // atoms: parameters outside trigonometric functions, and angle arguments
    let mut args = Vec::new();
    let mut outside = HashSet::new();
    for e in pb.rows.iter().chain(std::iter::once(pb.target)) {
        scan(e, &unknown, &mut args, &mut outside);
    }
    let mut params: Vec<u32> = pb
        .rows
        .iter()
        .chain(std::iter::once(pb.target))
        .flat_map(sym_set)
        .filter(|s| !unknown.contains(s) && *s < TMP_BASE)
        .collect::<HashSet<u32>>()
        .into_iter()
        .collect();
    params.sort();
    let val = |s: u32| pb.vals.get(&s).copied().unwrap_or(f64::NAN);
    // step sizes: a fraction of the value, or (inside angles only) what moves the angle by the same amount
    let step_of = |p: u32, r: f64| -> f64 {
        if outside.contains(&p) {
            let v = val(p).abs();
            r * if v > 1e-6 { v } else { 1.0 }
        } else {
            let g = args.iter().map(|a| eval(&diff(a, p), &val).abs()).fold(0.0, f64::max);
            if g > 1e-9 { r / g } else { r }
        }
    };
    // parameters the target depends on
    let mut relevant = Vec::new();
    let mut stuck = false;
    for &p in &params {
        let d = step_of(p, 0.03);
        let moved = [1.0, -1.0].iter().find_map(|s| sm.at(&[(p, val(p) + s * d)]));
        sm.states.clear();
        match moved {
            Some(t) if (t - t0).abs() > 1e-9 * (1.0 + t0.abs()) => relevant.push(p),
            Some(_) => {}
            // cannot move at all: treat as part of the formula
            None => {
                stuck = true;
                relevant.push(p)
            }
        }
        if past_deadline() {
            return None;
        }
    }
    if relevant.is_empty() {
        // the target is the same whatever the parameters
        let c = nice_const(t0);
        return (!stuck && !has_float(&c)).then(|| Fit { found: Found::Formula(c), checks: vec![Sample { params: vec![], value: t0, exact: Dd::from(t0) }] });
    }
    let all_args = &args;
    let args: Vec<Expr> = all_args.iter().filter(|a| sym_set(a).iter().any(|s| relevant.contains(s))).cloned().collect();
    let poly: Vec<u32> = relevant.iter().copied().filter(|p| outside.contains(p)).collect();
    if dbg {
        eprintln!("fit: {} polynomial atoms, {} angles, angle target {is_angle}", poly.len(), args.len());
    }
    if poly.len() + args.len() > 12 {
        return None;
    }

    // samples: random parameter values around the current ones
    let mut range = 0.4;
    let mut fails = 0;
    let mut tries = 0;
    let draw = |sm: &mut Sampler, range: f64| -> Option<Sample> {
        let pv: Vec<(u32, f64)> = relevant.iter().map(|&p| (p, val(p) + (2.0 * sm.rand() - 1.0) * step_of(p, range))).collect();
        let (t, td) = sm.exact_at(&pv)?;
        Some(Sample { params: pv, value: t, exact: td })
    };
    let mut samples: Vec<Sample> = Vec::new();
    // probe how far the construction can move
    while tries < 12 {
        tries += 1;
        match draw(&mut sm, range) {
            Some(s) => samples.push(s),
            None => fails += 1,
        }
        if fails >= 3 && range > 0.04 {
            range *= 0.5;
            fails = 0;
            tries = 0;
            samples.clear();
        }
        if past_deadline() {
            return None;
        }
    }
    if samples.len() < 6 {
        if dbg {
            eprintln!("fit: only {} samples", samples.len());
        }
        return None;
    }
    let mut checks: Vec<Sample> = Vec::new();
    for _ in 0..30 {
        if checks.len() >= 8 {
            break;
        }
        if let Some(s) = draw(&mut sm, range) {
            checks.push(s);
        }
    }
    if checks.len() < 4 {
        return None;
    }
    let more = |sm: &mut Sampler, samples: &mut Vec<Sample>, want: usize| -> bool {
        let mut misses = 0;
        while samples.len() < want {
            if past_deadline() || misses > 20 + want {
                return false;
            }
            match draw(sm, range) {
                Some(s) => samples.push(s),
                None => misses += 1,
            }
        }
        true
    };
    let sval = |s: &Sample, x: u32| s.params.iter().find(|p| p.0 == x).map(|p| p.1).unwrap_or_else(|| val(x));
    let arg_codes: Vec<DdCode> = args.iter().map(DdCode::new).collect();
    let arg_values = |s: &Sample| -> Vec<Dd> {
        let mut st = Vec::new();
        arg_codes.iter().map(|c| c.eval(&|x: u32| Dd::from(sval(s, x)), &mut st)).collect()
    };

    // an angle that is a rational combination of the given angles and π
    if is_angle && !args.is_empty() {
        let nc = args.len() + 2;
        if more(&mut sm, &mut samples, nc + 6) {
            let m = samples.len();
            let mut a = vec![Dd::ZERO; m * nc];
            for (i, s) in samples.iter().enumerate() {
                a[i * nc] = s.exact;
                a[i * nc + 1] = crate::dd::PI;
                for (j, g) in arg_values(s).into_iter().enumerate() {
                    a[i * nc + 2 + j] = g;
                }
            }
            if let Ok(x) = null_vector(&mut a, m, nc) {
                if x[0].hi.abs() > 1e-9 {
                    let zero_or = |v: Dd, maxd: i128| if v.hi.abs() < 1e-20 { Some(Rat::int(0)) } else { frac(v, maxd) };
                    let ks: Option<Vec<Rat>> = x[1..].iter().enumerate().map(|(i, v)| zero_or(-*v / x[0], if i == 0 { 360 } else { 24 })).collect();
                    if let Some(ks) = ks {
                        let mut ts = vec![mul(vec![num(Num::R(ks[0])), pi()])];
                        for (k, g) in ks[1..].iter().zip(&args) {
                            ts.push(mul(vec![num(Num::R(*k)), g.clone()]));
                        }
                        let f = add(ts);
                        if agrees(&f, &checks, pb.vals) {
                            return Some(Fit { found: Found::Formula(f), checks });
                        }
                    }
                }
            }
        }
    }

    // homogeneity: scaling the lengths (or every polynomial atom) scales T
    let tv = |t: f64| if is_angle { t.cos() } else { t };
    let scale_test = |sm: &mut Sampler, which: &dyn Fn(u32) -> bool| -> Option<u32> {
        if !poly.iter().any(|p| which(*p)) {
            return None;
        }
        let mut w: Option<u32> = None;
        for lam in [1.09f64, 0.93] {
            let pv: Vec<(u32, f64)> = poly.iter().filter(|p| which(**p)).map(|&p| (p, val(p) * lam)).collect();
            let t = tv(sm.at(&pv)?);
            let t00 = tv(t0);
            if t00.abs() < 1e-9 || t / t00 <= 0.0 {
                return None;
            }
            let e = (t / t00).ln() / lam.ln();
            let k = e.round();
            if (e - k).abs() > 1e-7 || !(0.0..=1.0).contains(&k) || w.is_some_and(|w| w != k as u32) {
                return None;
            }
            w = Some(k as u32);
        }
        w
    };
    let (homog, scaling): (Homog, Vec<bool>) = if let Some(w) = scale_test(&mut sm, &|_| true) {
        (Homog::Yes(w), vec![true; poly.len()])
    } else if let Some(w) = scale_test(&mut sm, &|p| pb.lengths.contains(&p)) {
        (Homog::Yes(w), poly.iter().map(|p| pb.lengths.contains(p)).collect())
    } else {
        (Homog::No, vec![false; poly.len()])
    };
    let at = Atoms { poly, scaling, args, homog };
    if dbg {
        eprintln!("fit: homogeneity {:?}, sample range {range}", at.homog);
    }

    let row_of = |s: &Sample, half: bool| -> Row {
        let trig = arg_values(s)
            .into_iter()
            .map(|a| {
                let a = if half { a * Dd::from(0.5) } else { a };
                let (s1, c1) = a.sin_cos();
                let (s2, c2) = (a + a).sin_cos();
                [Dd::ONE, c1, s1, c2, s2]
            })
            .collect();
        let t = if is_angle { s.exact.sin_cos().1 } else { s.exact };
        Row { t, p: at.poly.iter().map(|&p| Dd::from(sval(s, p))).collect(), trig }
    };

    // T or T² as a polynomial first, then as a rational function, then the
    // quadratic and biquadratic relations
    let shapes = [
        (Shape::Lin, true),
        (Shape::Sq, true),
        (Shape::Lin, false),
        (Shape::Sq, false),
        (Shape::Quad, true),
        (Shape::Biq, true),
        (Shape::Quad, false),
        (Shape::Biq, false),
    ];
    // (half angles, harmonics)
    let levels: &[(bool, u8)] = if at.args.is_empty() { &[(false, 1)] } else { &[(false, 1), (true, 1), (false, 2), (true, 2)] };
    for (shape, monic) in shapes {
        for &(half, harm) in levels {
            let mut prev_len = 0;
            for w in 0..=8u32 {
                if past_deadline() {
                    return None;
                }
                let Some(terms) = basis(&at, shape, monic, harm, w, pb.max_cols) else {
                    if w > 0 && basis(&at, shape, monic, harm, w, usize::MAX).is_some_and(|b| b.len() > pb.max_cols) {
                        break;
                    }
                    continue;
                };
                let nc = terms.len();
                // the same terms as one degree lower
                if nc == prev_len {
                    continue;
                }
                prev_len = nc;
                if !more(&mut sm, &mut samples, nc + 8) {
                    if dbg {
                        eprintln!("fit: only {} of {} samples for {shape:?} w {w}", samples.len(), nc + 8);
                    }
                    return None;
                }
                let m = nc + 8;
                let rows: Vec<Row> = samples[..m].iter().map(|s| row_of(s, half)).collect();
                let mut a = vec![Dd::ZERO; m * nc];
                for (i, r) in rows.iter().enumerate() {
                    for (j, t) in terms.iter().enumerate() {
                        a[i * nc + j] = term_value(t, r);
                    }
                }
                let x = match null_vector(&mut a, m, nc) {
                    Ok(x) => x,
                    Err(d) => {
                        if dbg && d > 1 {
                            eprintln!("fit: {shape:?} monic {monic} half {half} harmonics {harm} w {w}: {nc} columns, null space {d}");
                        }
                        continue;
                    }
                };
                // support, by each term's share of the relation at the samples
                let mut share = vec![0.0f64; nc];
                for r in &rows {
                    for (j, t) in terms.iter().enumerate() {
                        share[j] = share[j].max((x[j] * term_value(t, r)).hi.abs());
                    }
                }
                let top = share.iter().cloned().fold(0.0, f64::max);
                let keep: Vec<usize> = (0..nc).filter(|&j| share[j] > 1e-9 * top).collect();
                if keep.len() < 2 || !keep.iter().any(|&j| terms[j].k > 0) {
                    continue;
                }
                // refit on the support alone for accurate coefficients
                let kc = keep.len();
                let mut a2 = vec![Dd::ZERO; m * kc];
                for (i, r) in rows.iter().enumerate() {
                    for (jj, &j) in keep.iter().enumerate() {
                        a2[i * kc + jj] = term_value(&terms[j], r);
                    }
                }
                let Ok(x2) = null_vector(&mut a2, m, kc) else { continue };
                // exact coefficients, normalized by a term with T in it
                let mut coefs: Option<Vec<Expr>> = None;
                let mut order: Vec<usize> = (0..kc).collect();
                order.sort_by(|&p, &q| terms[keep[q]].k.cmp(&terms[keep[p]].k).then(x2[q].hi.abs().partial_cmp(&x2[p].hi.abs()).unwrap_or(std::cmp::Ordering::Equal)));
                for &piv in order.iter().take(4) {
                    if x2[piv].hi.abs() < 1e-200 {
                        continue;
                    }
                    let cs: Option<Vec<Expr>> = x2.iter().map(|v| nice_coef(*v / x2[piv])).collect();
                    if cs.is_some() {
                        coefs = cs;
                        break;
                    }
                }
                let Some(coefs) = coefs else {
                    if dbg {
                        eprintln!("fit: {shape:?} half {half} w {w}: null vector of {kc} terms has no exact coefficients");
                    }
                    continue;
                };
                let p = add(coefs.iter().zip(&keep).map(|(c, &j)| mul(vec![c.clone(), term_expr(&terms[j], &at, pb.t, half)])).collect());
                // the relation must hold at the check samples
                let cvals: Vec<f64> = coefs.iter().map(|c| eval(c, &|_| 0.0)).collect();
                let ok = checks.iter().all(|s| {
                    let r = row_of(s, half);
                    let (mut sum, mut mag) = (0.0, 0.0);
                    for (c, &j) in cvals.iter().zip(&keep) {
                        let v = c * term_value(&terms[j], &r).to_f64();
                        sum += v;
                        mag += v.abs();
                    }
                    sum.abs() <= 1e-10 * mag
                });
                if dbg {
                    eprintln!("fit: {shape:?} monic {monic} half {half} harmonics {harm} w {w}: {nc} columns, {kc} terms, checks {ok}");
                }
                if ok {
                    return Some(Fit { found: Found::Relation { p, cos: is_angle }, checks });
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_vector_of_a_relation() {
        // columns x, y, x + 2y
        let (m, n) = (8, 3);
        let mut a = vec![Dd::ZERO; m * n];
        for i in 0..m {
            let (x, y) = (Dd::from(1.0 + i as f64 * 0.37), Dd::from(2.0 - (i * i) as f64 * 0.11));
            a[i * n] = x;
            a[i * n + 1] = y;
            a[i * n + 2] = x + Dd::from(2.0) * y;
        }
        let v = null_vector(&mut a, m, n).unwrap();
        assert!(((v[0] / v[2]).hi + 1.0).abs() < 1e-14 && ((v[1] / v[2]).hi + 2.0).abs() < 1e-14, "{v:?}");
        assert_eq!(frac(-v[1] / v[2], 100), Rat::new(2, 1));
    }

    #[test]
    fn full_rank_has_no_null_vector() {
        let (m, n) = (6, 2);
        let mut a = vec![Dd::ZERO; m * n];
        for i in 0..m {
            a[i * n] = Dd::from(i as f64);
            a[i * n + 1] = Dd::from((i * i) as f64 + 1.0);
        }
        assert!(null_vector(&mut a, m, n).is_err());
    }

    #[test]
    fn coefficients_are_recognized() {
        let show = |x: Dd| nice_coef(x).map(|e| crate::expr::bx::to_plain(&e, &|_| String::new()));
        assert_eq!(show(Dd::from(3.0) / Dd::from(7.0)).as_deref(), Some("3/7"));
        let golden = (Dd::ONE + Dd::from(5.0).sqrt()) / Dd::from(2.0);
        assert!(show(golden).is_some_and(|s| s.contains("sqrt(5)")), "{:?}", show(golden));
        // a number that is only close to a fraction
        assert_eq!(show(Dd::from(1.0 / 3.0)), None);
        assert_eq!(show(crate::dd::PI), None);
    }
}
