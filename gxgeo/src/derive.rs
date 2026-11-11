//! Symbolic derivation of closed-form formulas from the constraint system.
//!
//! The equations are decomposed into a block-triangular sequence of small
//! square subsystems (maximum matching + strongly connected components).
//! Each block is solved symbolically by elimination: linear solving,
//! pairwise elimination of shared quadratic forms, the quadratic formula,
//! radical rationalization, harmonic (a·cos + b·sin) forms and inversion of
//! elementary functions. Branches are chosen by comparing each candidate
//! against the current numeric solution.

use crate::alg::{MPoly, cheapest_main, factor_main, interreduce, resultant, strip_factor, subresultant_chain, subst_linear};
use crate::expr::poly::{coeffs, from_poly, poly, split_frac, total_degree};
use num_bigint::BigInt;
use num_rational::BigRational as BigQ;
use num_traits::ToPrimitive;
use crate::expr::*;
use crate::model::*;
use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;

pub struct Derived {
    pub expr: Expr,
    /// False when some block had to be solved numerically.
    pub exact: bool,
    /// Formula evaluated at the current configuration.
    pub value: f64,
    /// Measured value of the target.
    pub actual: f64,
}

struct Env {
    vals: HashMap<u32, f64>,
}

impl Env {
    fn get(&self, s: u32) -> f64 {
        self.vals.get(&s).copied().unwrap_or(f64::NAN)
    }
    fn eval(&self, e: &Expr) -> f64 {
        eval(e, &|s| self.get(s))
    }
    fn eval_perturbed(&self, e: &Expr, k: u32) -> f64 {
        eval(e, &|s| {
            let v = self.get(s);
            let h = ((s.wrapping_mul(2654435761).wrapping_add(k.wrapping_mul(40503))) % 1000) as f64 / 1000.0 - 0.5;
            v + h * 0.05 * (1.0 + v.abs())
        })
    }
    /// Probabilistic test that an expression is identically zero.
    fn is_zero(&self, e: &Expr) -> bool {
        if e.is_zero() {
            return true;
        }
        if let Some(n) = e.as_num() {
            return n.is_zero();
        }
        let v0 = self.eval(e);
        if !(v0.abs() < 1e-9) {
            return false;
        }
        for k in 1..3 {
            let v = self.eval_perturbed(e, k);
            if v.is_finite() && v.abs() > 1e-8 {
                return false;
            }
        }
        true
    }
}

/// Polynomial view of expressions: block variables first, then atoms (symbols
/// and variable-free non-polynomial subexpressions such as √r or cos θ).
struct PolyCtx {
    vars: Vec<u32>,
    atoms: Vec<Expr>,
}

impl PolyCtx {
    fn new(vars: &[u32]) -> PolyCtx {
        PolyCtx { vars: vars.to_vec(), atoms: Vec::new() }
    }
    fn n(&self) -> usize {
        self.vars.len() + self.atoms.len()
    }
    fn atom(&mut self, e: &Expr) -> usize {
        if let Some(i) = self.atoms.iter().position(|a| a == e) {
            return self.vars.len() + i;
        }
        self.atoms.push(e.clone());
        self.vars.len() + self.atoms.len() - 1
    }
    /// Registers atoms; false when `e` is not polynomial in the block variables.
    fn collect(&mut self, e: &Expr) -> bool {
        match e.node() {
            Node::Num(Num::R(_)) => true,
            Node::Sym(s) => {
                if !self.vars.contains(s) {
                    self.atom(e);
                }
                true
            }
            Node::Add(ts) | Node::Mul(ts) => ts.iter().all(|t| self.collect(t)),
            Node::Pow(b, ex) if ex.as_int().is_some_and(|k| k >= 0) => self.collect(b),
            _ => {
                if contains_any(e, &self.vars) {
                    return false;
                }
                self.atom(e);
                true
            }
        }
    }
    fn index(&self, e: &Expr) -> Option<usize> {
        if let Some(s) = e.as_sym() {
            if let Some(i) = self.vars.iter().position(|v| *v == s) {
                return Some(i);
            }
        }
        self.atoms.iter().position(|a| a == e).map(|i| self.vars.len() + i)
    }
    fn to_mpoly(&self, e: &Expr) -> Option<MPoly> {
        let n = self.n();
        Some(match e.node() {
            Node::Num(Num::R(r)) => MPoly::constant(n, BigQ::new(BigInt::from(r.n), BigInt::from(r.d))),
            Node::Add(ts) => {
                let mut acc = MPoly::zero(n);
                for t in ts {
                    acc = acc.add(&self.to_mpoly(t)?);
                }
                acc
            }
            Node::Mul(ts) => {
                let mut acc = MPoly::one(n);
                for t in ts {
                    acc = acc.mul(&self.to_mpoly(t)?);
                }
                acc
            }
            Node::Pow(b, ex) if ex.as_int().is_some_and(|k| k >= 0) && self.index(e).is_none() => {
                self.to_mpoly(b)?.pow(ex.as_int().unwrap() as u32)
            }
            _ => MPoly::var(n, self.index(e)?),
        })
    }
    fn to_expr(&self, p: &MPoly) -> Expr {
        let mut terms = Vec::new();
        for (m, c) in &p.t {
            let cn = c.numer().to_i128();
            let cd = c.denom().to_i128();
            let coef = match (cn, cd) {
                (Some(a), Some(b)) => Rat::new(a, b).map(|r| num(Num::R(r))).unwrap_or_else(|| float_raw(c.to_f64().unwrap_or(f64::NAN))),
                _ => float_raw(c.to_f64().unwrap_or(f64::NAN)),
            };
            let mut fs = vec![coef];
            for (k, e) in m.iter().enumerate() {
                if *e > 0 {
                    let base = if k < self.vars.len() { sym(self.vars[k]) } else { self.atoms[k - self.vars.len()].clone() };
                    fs.push(pow(base, int(*e as i64)));
                }
            }
            terms.push(mul(fs));
        }
        add(terms)
    }
    fn values(&self, env: &Env) -> Vec<f64> {
        self.vars.iter().map(|v| env.get(*v)).chain(self.atoms.iter().map(|a| env.eval(a))).collect()
    }
}

fn nv_of(v: &[u32]) -> usize {
    v.len()
}

/// The factors of r (in one of `rest`) that vanish at `vals`, multiplied.
fn vanishing_part(r: &MPoly, rest: &[usize], vals: &[f64]) -> MPoly {
    // factorization is unique, so factor in whichever variable is cheapest
    let all: Vec<usize> = (0..r.n).filter(|&v| r.uses(v)).collect();
    if !rest.iter().any(|&v| r.uses(v)) {
        return r.clone();
    }
    let Some(main) = cheapest_main(r, &all) else { return r.clone() };
    let fs = factor_main(r, main);
    let van: Vec<MPoly> = fs.into_iter().filter(|f| rest.iter().any(|&v| f.uses(v)) && f.vanishes(vals)).collect();
    if van.is_empty() {
        return r.clone();
    }
    let prod = van.iter().fold(MPoly::one(r.n), |acc, f| acc.mul(f));
    // factors may carry content in the parameters; never make things bigger
    if prod.terms() <= r.terms() { prod } else { r.clone() }
}

/// Rewrites radicals (A ± A·cos x)^(k/2) with sin(x/2) or cos(x/2), the sign
/// taken from the current configuration.
fn half_angles(e: &Expr, env: &Env) -> Expr {
    let r = match e.node() {
        Node::Num(_) | Node::Pi | Node::Sym(_) => return e.clone(),
        Node::Add(ts) => add(ts.iter().map(|t| half_angles(t, env)).collect()),
        Node::Mul(ts) => mul(ts.iter().map(|t| half_angles(t, env)).collect()),
        Node::Pow(b, x) => pow(half_angles(b, env), x.clone()),
        Node::Fun(f, a) => fun(*f, half_angles(a, env)),
        Node::Atan2(a, b) => atan2(half_angles(a, env), half_angles(b, env)),
    };
    if let Node::Pow(b, x) = r.node() {
        if let Some(h) = half_angle_root(b, x) {
            return resolve_abs(&h, env);
        }
    }
    r
}

/// The bases of the non-integer powers in `e`.
fn radicands(e: &Expr, out: &mut std::collections::BTreeSet<Expr>) {
    match e.node() {
        Node::Pow(b, x) => {
            if x.as_int().is_none() {
                out.insert(b.clone());
            }
            radicands(b, out);
            radicands(x, out);
        }
        Node::Add(ts) | Node::Mul(ts) => ts.iter().for_each(|t| radicands(t, out)),
        Node::Fun(_, a) => radicands(a, out),
        Node::Atan2(a, b) => {
            radicands(a, out);
            radicands(b, out);
        }
        _ => {}
    }
}

/// Whether `e` contains a non-integer power of a sum.
fn has_sqrt_of_sum(e: &Expr) -> bool {
    match e.node() {
        Node::Pow(b, x) => (x.as_int().is_none() && matches!(b.node(), Node::Add(_))) || has_sqrt_of_sum(b),
        Node::Add(ts) | Node::Mul(ts) => ts.iter().any(has_sqrt_of_sum),
        Node::Fun(_, a) => has_sqrt_of_sum(a),
        Node::Atan2(a, b) => has_sqrt_of_sum(a) || has_sqrt_of_sum(b),
        _ => false,
    }
}

fn has_sin_cos(e: &Expr) -> bool {
    match e.node() {
        Node::Fun(Fun::Sin | Fun::Cos, _) => true,
        Node::Add(ts) | Node::Mul(ts) => ts.iter().any(has_sin_cos),
        Node::Pow(a, b) | Node::Atan2(a, b) => has_sin_cos(a) || has_sin_cos(b),
        Node::Fun(_, a) => has_sin_cos(a),
        _ => false,
    }
}

/// Simplifies the trigonometric radicals in `e` that do not involve `vars`
/// (they become atoms of the block), e.g. √(100 tan²θ + 100) = 10 / cos θ,
/// the signs taken from the current configuration.
fn simplify_radical_atoms(e: &Expr, vars: &[u32], env: &Env) -> Expr {
    match e.node() {
        Node::Num(_) | Node::Pi | Node::Sym(_) => e.clone(),
        Node::Pow(b, x) if !contains_any(e, vars) && x.as_int().is_none() && e.size() < 200 && has_sin_cos(b) => {
            let r = resolve_abs(&deep_best(e), env);
            if complexity(&r) < complexity(e) { r } else { pow(b.clone(), x.clone()) }
        }
        Node::Add(ts) => add(ts.iter().map(|t| simplify_radical_atoms(t, vars, env)).collect()),
        Node::Mul(ts) => mul(ts.iter().map(|t| simplify_radical_atoms(t, vars, env)).collect()),
        Node::Pow(b, x) => pow(simplify_radical_atoms(b, vars, env), simplify_radical_atoms(x, vars, env)),
        Node::Fun(f, a) => fun(*f, simplify_radical_atoms(a, vars, env)),
        Node::Atan2(a, b) => atan2(simplify_radical_atoms(a, vars, env), simplify_radical_atoms(b, vars, env)),
    }
}

/// The atom tan(θ/2) (or cot(θ/2) with `cot`) that replaces sin θ and cos θ
/// in a polynomial block.
#[derive(Clone, Debug)]
struct HalfTan {
    th: Expr,
    cot: bool,
}

impl HalfTan {
    fn expr(&self) -> Expr {
        let c = cos(self.th.clone());
        let d = if self.cot { sub(one(), c) } else { add(vec![one(), c]) };
        div(sin(self.th.clone()), d)
    }
}

/// p(t) / (1 + t²)^k for a polynomial p of degree ≤ 2k in t = tan(θ/2)
/// (or t = cot(θ/2) with `cot`), as a polynomial in cos θ and sin θ:
/// t^j / (1 + t²)^k = sin^j(θ/2) cos^(2k-j)(θ/2), and sin²(θ/2) = (1 - cos θ)/2,
/// cos²(θ/2) = (1 + cos θ)/2, sin(θ/2) cos(θ/2) = sin(θ)/2.
fn half_tan_form(cs: &[Expr], k: usize, th: &Expr, cot: bool) -> Expr {
    let (c, s) = (cos(th.clone()), sin(th.clone()));
    let half = |e: Expr| mul(vec![rat(1, 2), e]);
    let (lo, hi) = (half(sub(one(), c.clone())), half(add(vec![one(), c.clone()])));
    let (num_sq, other_sq) = if cot { (hi, lo) } else { (lo, hi) };
    let mut terms = Vec::new();
    for (j, a) in cs.iter().enumerate() {
        if a.is_zero() {
            continue;
        }
        let (p, q) = (j / 2, k - (j + 1) / 2);
        let mut fs = vec![a.clone(), pow(num_sq.clone(), int(p as i64)), pow(other_sq.clone(), int(q as i64))];
        if j % 2 == 1 {
            fs.push(half(s.clone()));
        }
        terms.push(mul(fs));
    }
    add(terms)
}

/// Rewrites rational functions of the symbol `t` (standing for the atom
/// `kind`) in sin θ and cos θ, wherever that makes the expression smaller.
/// Occurrences that are not rational in t are replaced by the atom itself.
fn untrig_atom(e: &Expr, t: u32, kind: &HalfTan) -> Expr {
    if !contains(e, t) || past_deadline() {
        return subst_map(e, &HashMap::from([(t, kind.expr())]));
    }
    let mut cands = Vec::new();
    // as a whole, when rational in t
    let (n, d) = split_frac(e, &[t]);
    let poly_in_t = |x: &Expr| coeffs(&expand(x), t).filter(|cs| cs.iter().all(|c| !contains(c, t)));
    if let (Some(cn), Some(cd)) = (poly_in_t(&n), poly_in_t(&d)) {
        let k = (cn.len() - 1).max(cd.len() - 1).div_ceil(2);
        let q = div(half_tan_form(&cn, k, &kind.th, kind.cot), half_tan_form(&cd, k, &kind.th, kind.cot));
        cands.push(best_form(&q));
    }
    // part by part
    let parts = match e.node() {
        Node::Add(ts) => add(ts.iter().map(|x| untrig_atom(x, t, kind)).collect()),
        Node::Mul(ts) => mul(ts.iter().map(|x| untrig_atom(x, t, kind)).collect()),
        Node::Pow(b, x) => pow(untrig_atom(b, t, kind), untrig_atom(x, t, kind)),
        Node::Fun(f, a) => fun(*f, untrig_atom(a, t, kind)),
        Node::Atan2(a, b) => atan2(untrig_atom(a, t, kind), untrig_atom(b, t, kind)),
        _ => subst_map(e, &HashMap::from([(t, kind.expr())])),
    };
    cands.push(parts);
    cands.into_iter().min_by_key(complexity).unwrap()
}

/// sin θ and cos θ as separate atoms lose sin² + cos² = 1, and with it any
/// factorization. Rewrites them in one atom: tan θ or cot θ when every
/// polynomial is homogeneous in the pair, tan(θ/2) or cot(θ/2) otherwise
/// (whichever stays finite at the current solution).
/// Returns the atoms that became tan(θ/2) or cot(θ/2).
fn merge_trig(ctx: &mut PolyCtx, polys: &mut [MPoly], env: &Env) -> Vec<(usize, HalfTan)> {
    let nv = ctx.vars.len();
    let mut merged = Vec::new();
    for i in 0..ctx.atoms.len() {
        let Node::Fun(Fun::Sin, th) = ctx.atoms[i].node() else { continue };
        let th = th.clone();
        let Some(j) = ctx.atoms.iter().position(|a| matches!(a.node(), Node::Fun(Fun::Cos, t) if *t == th)) else { continue };
        let (si, ci) = (nv + i, nv + j);
        if !polys.iter().any(|p| p.uses(si)) || !polys.iter().any(|p| p.uses(ci)) {
            continue;
        }
        let (sv, cv) = (env.eval(&sin(th.clone())), env.eval(&cos(th.clone())));
        let homog = polys.iter().all(|p| {
            let mut ks = p.t.keys().map(|m| m[si] + m[ci]);
            let k0 = ks.next();
            ks.all(|k| Some(k) == k0)
        });
        let n = polys[0].n;
        if homog {
            // divide by cos^k (keep the sin exponent) or by sin^k (keep the cos exponent)
            let (keep, drop, atom) = if cv.abs() >= sv.abs() { (si, ci, div(sin(th.clone()), cos(th.clone()))) } else { (ci, si, div(cos(th.clone()), sin(th.clone()))) };
            for p in polys.iter_mut() {
                let mut t = std::collections::BTreeMap::new();
                for (m, c) in &p.t {
                    let mut m2 = m.clone();
                    m2[drop] = 0;
                    t.insert(m2, c.clone());
                }
                p.t = t;
            }
            ctx.atoms[keep - nv] = atom;
            ctx.atoms[drop - nv] = num(Num::R(Rat::new(1, 1).unwrap()));
        } else {
            // S = 2t/(1+t²), C = ±(1-t²)/(1+t²), times (1+t²)^d
            let flip = cv < 0.0;
            let atom = if flip { div(sin(th.clone()), sub(one(), cos(th.clone()))) } else { div(sin(th.clone()), add(vec![one(), cos(th.clone())])) };
            let t = MPoly::var(n, si);
            let tt = t.mul(&t);
            let one_p = MPoly::one(n);
            let sp = t.scale(&BigQ::from_integer(BigInt::from(2)));
            let cp = if flip { tt.sub(&one_p) } else { one_p.sub(&tt) };
            let hp = one_p.add(&tt);
            for p in polys.iter_mut() {
                let d = p.t.keys().map(|m| m[si] + m[ci]).max().unwrap_or(0) as u32;
                let mut cache: HashMap<(u32, u32), MPoly> = HashMap::new();
                let mut r = MPoly::zero(n);
                for (m, c) in &p.t {
                    if past_deadline() {
                        return merged;
                    }
                    let (a, b) = (m[si] as u32, m[ci] as u32);
                    let f = cache.entry((a, b)).or_insert_with(|| sp.pow(a).mul(&cp.pow(b)).mul(&hp.pow(d - a - b)));
                    for (fm, fc) in &f.t {
                        let mut m2: Vec<u16> = m.iter().zip(fm).map(|(x, y)| x + y).collect();
                        m2[si] = fm[si];
                        m2[ci] = 0;
                        r.add_term(m2, c * fc);
                    }
                }
                *p = r;
            }
            ctx.atoms[i] = atom;
            ctx.atoms[j] = num(Num::R(Rat::new(1, 1).unwrap()));
            merged.push((i, HalfTan { th, cot: flip }));
        }
    }
    merged
}

struct BlockSolver<'a> {
    env: &'a Env,
    tmp: u32,
    budget: u32,
    /// Blocks still unsolved at this point fall back to numeric values.
    deadline: web_time::Instant,
    /// Polynomial elimination down to this variable only: it is never
    /// eliminated, and the others are not solved for.
    keep: Option<u32>,
}

fn has_inverse_trig(e: &Expr) -> bool {
    match e.node() {
        Node::Fun(Fun::Asin | Fun::Acos | Fun::Atan, _) | Node::Atan2(..) => true,
        Node::Num(_) | Node::Pi | Node::Sym(_) => false,
        Node::Add(ts) | Node::Mul(ts) => ts.iter().any(has_inverse_trig),
        Node::Pow(a, b) => has_inverse_trig(a) || has_inverse_trig(b),
        Node::Fun(_, a) => has_inverse_trig(a),
    }
}

/// True when `v` occurs inside sin, cos or tan somewhere in `e`.
fn periodic_in(e: &Expr, v: u32) -> bool {
    match e.node() {
        Node::Fun(Fun::Sin | Fun::Cos | Fun::Tan, a) => contains(a, v) || periodic_in(a, v),
        Node::Num(_) | Node::Pi | Node::Sym(_) => false,
        Node::Add(ts) | Node::Mul(ts) => ts.iter().any(|t| periodic_in(t, v)),
        Node::Pow(a, b) | Node::Atan2(a, b) => periodic_in(a, v) || periodic_in(b, v),
        Node::Fun(_, a) => periodic_in(a, v),
    }
}

/// Square roots containing `v`, outermost first.
fn sqrts_with(e: &Expr, v: u32, out: &mut Vec<Expr>) {
    if !contains(e, v) {
        return;
    }
    match e.node() {
        Node::Pow(b, ex) => {
            if ex.as_rat().is_some_and(|r| r.d == 2) {
                let s = pow(b.clone(), rat(1, 2));
                if !out.contains(&s) {
                    out.push(s);
                }
            }
            sqrts_with(b, v, out);
        }
        Node::Add(ts) | Node::Mul(ts) => ts.iter().for_each(|t| sqrts_with(t, v, out)),
        Node::Fun(_, a) => sqrts_with(a, v, out),
        Node::Atan2(a, b) => {
            sqrts_with(a, v, out);
            sqrts_with(b, v, out);
        }
        _ => {}
    }
}

#[allow(dead_code)]
fn find_sqrt_with(e: &Expr, v: u32) -> Option<Expr> {
    match e.node() {
        Node::Pow(b, ex) => {
            if let Some(r) = ex.as_rat() {
                if r.d == 2 && contains(b, v) {
                    if let Some(inner) = find_sqrt_with(b, v) {
                        return Some(inner);
                    }
                    return Some(pow(b.clone(), rat(1, 2)));
                }
            }
            find_sqrt_with(b, v).or_else(|| find_sqrt_with(ex, v))
        }
        Node::Add(ts) | Node::Mul(ts) => ts.iter().find_map(|t| find_sqrt_with(t, v)),
        Node::Fun(_, a) => find_sqrt_with(a, v),
        Node::Atan2(a, b) => find_sqrt_with(a, v).or_else(|| find_sqrt_with(b, v)),
        _ => None,
    }
}

/// Rewrites powers b^(k/2) with odd k as b^((k-1)/2) * sqrt(b) so a single
/// radical can be isolated.
fn split_half_powers(e: &Expr) -> Expr {
    match e.node() {
        Node::Pow(b, ex) => {
            let b2 = split_half_powers(b);
            if let Some(r) = ex.as_rat() {
                if r.d == 2 && r.n != 1 && r.n != -1 {
                    let k = (r.n - 1) / 2;
                    return mul(vec![pow(b2.clone(), int(k as i64)), Expr(std::rc::Rc::new(Node::Pow(b2, rat(1, 2))))]);
                }
            }
            pow(b2, ex.clone())
        }
        Node::Add(ts) => add(ts.iter().map(split_half_powers).collect()),
        Node::Mul(ts) => mul(ts.iter().map(split_half_powers).collect()),
        Node::Fun(f, a) => fun(*f, split_half_powers(a)),
        _ => e.clone(),
    }
}

impl<'a> BlockSolver<'a> {
    fn new_tmp(&mut self) -> u32 {
        self.tmp += 1;
        self.tmp
    }

    fn close(&self, cand: &Expr, v: u32) -> f64 {
        let x = self.env.eval(cand);
        let t = self.env.get(v);
        if !x.is_finite() {
            return f64::INFINITY;
        }
        (x - t).abs() / (1.0 + t.abs())
    }

    /// Picks the candidate closest to the numeric solution, adjusting angles by 2πk.
    /// Picks the candidate closest to the numeric solution. With `periodic`
    /// (v occurs inside sin/cos/tan of the equation) candidates may be
    /// shifted by multiples of 2π.
    fn pick(&self, cands: Vec<Expr>, v: u32, periodic: bool) -> Option<Expr> {
        let t = self.env.get(v);
        let mut best: Option<(f64, Expr)> = None;
        for c in cands {
            let mut c = c;
            if periodic || has_inverse_trig(&c) {
                let x = self.env.eval(&c);
                if x.is_finite() {
                    let k = ((t - x) / (2.0 * PI)).round();
                    if k != 0.0 {
                        c = add(vec![c, mul(vec![int(2 * k as i64), pi()])]);
                    }
                }
            }
            let d = self.close(&c, v);
            if best.as_ref().is_none_or(|(bd, be)| d < *bd - 1e-12 || (d <= *bd + 1e-12 && c.size() < be.size())) {
                best = Some((d, c));
            }
        }
        let (d, e) = best?;
        if d < 1e-6 { Some(e) } else { None }
    }

    /// Normalizes an equation: numerator only, constant factors dropped,
    /// and among several factors the one that vanishes at the solution.
    fn prep(&self, e: &Expr, vars: &[u32]) -> Expr {
        // atan2(y, x) = 0 (with x > 0 at the solution) and atan(z) = 0 reduce to y = 0, z = 0
        let e = match e.node() {
            Node::Atan2(y, _) => y.clone(),
            Node::Fun(Fun::Atan, z) => z.clone(),
            _ => e.clone(),
        };
        let (n, _) = split_frac(&e, vars);
        // pull common factors (e.g. exp(t)) out so the vanishing one can be chosen
        let n = match n.node() {
            Node::Add(_) => factor_common(&n)
                .or_else(|| if n.size() < 400 { factor_common(&expand(&n)) } else { None })
                .filter(|f| match f.node() {
                    Node::Mul(fs) => fs[..fs.len() - 1].iter().any(|x| contains_any(x, vars)),
                    _ => false,
                })
                .unwrap_or(n),
            _ => n,
        };
        self.pick_factor(&n, vars)
    }

    fn pick_factor(&self, e: &Expr, vars: &[u32]) -> Expr {
        match e.node() {
            Node::Mul(fs) => {
                let with: Vec<&Expr> = fs.iter().filter(|f| contains_any(f, vars)).collect();
                if with.is_empty() {
                    return e.clone();
                }
                let mut best: Option<(f64, Expr)> = None;
                for f in with {
                    let base = self.pick_factor(f, vars);
                    let v = self.env.eval(&base).abs();
                    let v = if v.is_finite() { v } else { f64::INFINITY };
                    if best.as_ref().is_none_or(|(bv, _)| v < *bv) {
                        best = Some((v, base));
                    }
                }
                best.unwrap().1
            }
            Node::Pow(b, ex) if ex.as_int().is_some_and(|k| k > 0) => self.pick_factor(b, vars),
            _ => e.clone(),
        }
    }

    fn quad_roots(&self, c: &[Expr]) -> Vec<Expr> {
        let (c0, c1, c2) = (&c[0], &c[1], &c[2]);
        if c1.is_zero() || self.env.is_zero(c1) {
            let r = best_form(&neg(div(c0.clone(), c2.clone())));
            let s = sqrt(r);
            return vec![s.clone(), neg(s)];
        }
        let disc = best_form(&(sq(c1.clone()) - int(4) * c0 * c2));
        let sd = sqrt(disc);
        let den = int(2) * c2;
        vec![div(neg(c1.clone()) + &sd, den.clone()), div(neg(c1.clone()) - sd, den)]
    }

    /// Candidate solutions of eq = 0 for the single unknown v.
    fn candidates(&mut self, eq: &Expr, v: u32, depth: u32) -> Vec<Expr> {
        let mut out = self.candidates_inner(eq, v, depth);
        if depth == 0 && !out.iter().any(|c| self.close(c, v) < 1e-6) {
            if let Some(c) = self.nice_root(eq, v) {
                out.push(c);
            }
        }
        out
    }

    /// A simple closed-form constant (fraction, π multiple, small root) that
    /// matches the numeric solution and satisfies the equation identically.
    fn nice_root(&self, eq: &Expr, v: u32) -> Option<Expr> {
        let c = nice_const(self.env.get(v));
        if has_float(&c) {
            return None;
        }
        let r = subst(eq, &|x| if x == v { Some(c.clone()) } else { None });
        let r = best_form(&r);
        if r.is_zero() || self.env.is_zero(&r) { Some(c) } else { None }
    }

    fn candidates_inner(&mut self, eq: &Expr, v: u32, depth: u32) -> Vec<Expr> {
        if depth > 4 || past_deadline() {
            return vec![];
        }
        if let Some(mut cs) = coeffs(eq, v) {
            // factor out v^k
            let k = cs.iter().position(|c| !self.env.is_zero(c)).unwrap_or(0);
            let mut out = Vec::new();
            if k > 0 {
                out.push(zero());
                cs.drain(..k);
            }
            while cs.len() > 1 && self.env.is_zero(cs.last().unwrap()) {
                cs.pop();
            }
            match cs.len() {
                2 => out.push(neg(div(cs[0].clone(), cs[1].clone()))),
                3 => out.extend(self.quad_roots(&cs)),
                n if n >= 4 && cs[1..n - 1].iter().all(|c| self.env.is_zero(c)) => {
                    // binomial c0 + c_d v^d = 0
                    let d = (n - 1) as i64;
                    let w = neg(div(cs[0].clone(), cs[n - 1].clone()));
                    let wv = self.env.eval(&w);
                    if d % 2 == 1 {
                        out.push(if wv < 0.0 { neg(pow(neg(w), rat(1, d))) } else { pow(w, rat(1, d)) });
                    } else {
                        let r = pow(w, rat(1, d));
                        out.push(r.clone());
                        out.push(neg(r));
                    }
                }
                5 if self.env.is_zero(&cs[1]) && self.env.is_zero(&cs[3]) => {
                    for w in self.quad_roots(&[cs[0].clone(), cs[2].clone(), cs[4].clone()]) {
                        let s = sqrt(w);
                        out.push(s.clone());
                        out.push(neg(s));
                    }
                }
                n if n >= 4 => {
                    // higher degree: solve the factor that holds the root
                    let poly = add(cs.iter().enumerate().map(|(i, c)| mul(vec![c.clone(), pow(sym(v), int(i as i64))])).collect());
                    out.extend(self.factor_roots(&poly, v, depth));
                }
                _ => {}
            }
            return out;
        }
        // radicals: isolate one square root (outermost first) and square
        let eq_s = split_half_powers(eq);
        let mut rads = Vec::new();
        sqrts_with(&eq_s, v, &mut rads);
        for s in rads.into_iter().take(4) {
            let radicand = match s.node() {
                Node::Pow(b, _) => b.clone(),
                _ => continue,
            };
            let t = self.new_tmp();
            let e2 = replace(&eq_s, &s, &sym(t));
            let Some(cs) = coeffs(&e2, t) else { continue };
            if cs.len() > 3 || cs.iter().any(|c| contains(c, t)) {
                continue;
            }
            let c2 = cs.get(2).cloned().unwrap_or_else(zero);
            let a = &cs[0] + c2 * &radicand;
            let b = cs.get(1).cloned().unwrap_or_else(zero);
            let squared = sq(a) - sq(b) * radicand.clone();
            let squared = self.prep(&expand(&squared), &[v]);
            let mut out = self.candidates(&squared, v, depth + 1);
            // keep only roots of the original equation
            out.retain(|c| {
                let r = self.env.eval(&subst(eq, &|x| if x == v { Some(c.clone()) } else { None }));
                r.is_finite() && r.abs() < 1e-6 * (1.0 + self.env.eval(&radicand).abs())
            });
            if !out.is_empty() {
                return out;
            }
        }
        // harmonic form a cos v + b sin v + c
        let (cs, ss) = (self.new_tmp(), self.new_tmp());
        let vv = sym(v);
        let e3 = replace(&replace(eq, &cos(vv.clone()), &sym(cs)), &sin(vv.clone()), &sym(ss));
        if !contains(&e3, v) && (contains(&e3, cs) || contains(&e3, ss)) {
            if let Some(p) = poly(&e3, &[cs, ss]) {
                if total_degree(&p) == 1 {
                    let a = p.get(&vec![1, 0]).cloned().unwrap_or_else(zero);
                    let b = p.get(&vec![0, 1]).cloned().unwrap_or_else(zero);
                    let c = p.get(&vec![0, 0]).cloned().unwrap_or_else(zero);
                    let mut out = Vec::new();
                    if self.env.is_zero(&b) {
                        let x = fun(Fun::Acos, neg(div(c, a)));
                        out.push(x.clone());
                        out.push(neg(x));
                    } else if self.env.is_zero(&a) {
                        let x = fun(Fun::Asin, neg(div(c, b)));
                        out.push(x.clone());
                        out.push(pi() - x);
                    } else if self.env.is_zero(&c) {
                        let x = atan2(neg(a), b);
                        out.push(x.clone());
                        out.push(x + pi());
                    } else {
                        let r = sqrt(best_form(&(sq(a.clone()) + sq(b.clone()))));
                        let phi = atan2(b, a);
                        let w = fun(Fun::Acos, neg(div(c, r)));
                        out.push(&phi + &w);
                        out.push(phi - w);
                    }
                    return out;
                }
                // even quadratic in (cos v, sin v): linear in (cos 2v, sin 2v)
                if total_degree(&p) == 2 && p.keys().all(|m| (m[0] + m[1]) % 2 == 0) {
                    let get = |m: [u32; 2]| p.get(&vec![m[0], m[1]]).cloned().unwrap_or_else(zero);
                    let (cc, ss, cs, k0) = (get([2, 0]), get([0, 2]), get([1, 1]), get([0, 0]));
                    // C² = (1 + cos 2v)/2, S² = (1 − cos 2v)/2, CS = sin 2v / 2
                    let a = best_form(&mul(vec![rat(1, 2), sub(cc.clone(), ss.clone())]));
                    let b = best_form(&mul(vec![rat(1, 2), cs]));
                    let c = best_form(&add(vec![k0, mul(vec![rat(1, 2), add(vec![cc, ss])])]));
                    let mut us = Vec::new();
                    if self.env.is_zero(&b) {
                        let x = fun(Fun::Acos, neg(div(c, a)));
                        us.push(x.clone());
                        us.push(neg(x));
                    } else if self.env.is_zero(&a) {
                        let x = fun(Fun::Asin, neg(div(c, b)));
                        us.push(x.clone());
                        us.push(pi() - x);
                    } else {
                        let r = sqrt(best_form(&(sq(a.clone()) + sq(b.clone()))));
                        let phi = atan2(b, a);
                        let w = fun(Fun::Acos, neg(div(c, r)));
                        us.push(&phi + &w);
                        us.push(phi - w);
                    }
                    let mut out = Vec::new();
                    for u in us {
                        let half = mul(vec![rat(1, 2), u]);
                        out.push(half.clone());
                        out.push(half + pi());
                    }
                    return out;
                }
            }
        }
        self.isolate(eq, &zero(), v)
    }

    fn isolate(&self, lhs: &Expr, rhs: &Expr, v: u32) -> Vec<Expr> {
        match lhs.node() {
            Node::Sym(s) if *s == v => vec![rhs.clone()],
            Node::Add(ts) => {
                let (w, wo): (Vec<Expr>, Vec<Expr>) = ts.iter().cloned().partition(|t| contains(t, v));
                if w.len() != 1 {
                    return vec![];
                }
                self.isolate(&w[0], &(rhs - add(wo)), v)
            }
            Node::Mul(fs) => {
                let (w, wo): (Vec<Expr>, Vec<Expr>) = fs.iter().cloned().partition(|t| contains(t, v));
                if w.len() != 1 {
                    return vec![];
                }
                self.isolate(&w[0], &div(rhs.clone(), mul(wo)), v)
            }
            Node::Pow(b, ex) => {
                if contains(b, v) && !contains(ex, v) {
                    let r = pow(rhs.clone(), pow(ex.clone(), int(-1)));
                    let mut c = vec![r.clone()];
                    if ex.as_int().is_some_and(|k| k % 2 == 0) {
                        c.push(neg(r));
                    }
                    c.into_iter().flat_map(|x| self.isolate(b, &x, v)).collect()
                } else if contains(ex, v) && !contains(b, v) {
                    self.isolate(ex, &div(fun(Fun::Ln, rhs.clone()), fun(Fun::Ln, b.clone())), v)
                } else {
                    vec![]
                }
            }
            Node::Fun(f, a) => {
                let r = rhs.clone();
                let c: Vec<Expr> = match f {
                    Fun::Sin => {
                        let x = fun(Fun::Asin, r);
                        vec![x.clone(), pi() - x]
                    }
                    Fun::Cos => {
                        let x = fun(Fun::Acos, r);
                        vec![x.clone(), neg(x)]
                    }
                    Fun::Tan => {
                        let x = fun(Fun::Atan, r);
                        vec![x.clone(), x + pi()]
                    }
                    Fun::Asin => vec![sin(r)],
                    Fun::Acos => vec![cos(r)],
                    Fun::Atan => vec![fun(Fun::Tan, r)],
                    Fun::Exp => vec![fun(Fun::Ln, r)],
                    Fun::Ln => vec![fun(Fun::Exp, r)],
                    Fun::Sinh => vec![fun(Fun::Ln, &r + sqrt(sq(r.clone()) + one()))],
                    Fun::Cosh => {
                        let x = fun(Fun::Ln, &r + sqrt(sq(r.clone()) - one()));
                        vec![x.clone(), neg(x)]
                    }
                    Fun::Tanh => vec![mul(vec![rat(1, 2), fun(Fun::Ln, div(one() + &r, one() - &r))])],
                    Fun::Abs => vec![r.clone(), neg(r)],
                };
                // periodic functions: move each candidate to the period holding
                // the argument's current value
                let c: Vec<Expr> = if matches!(f, Fun::Sin | Fun::Cos | Fun::Tan) {
                    let target = self.env.eval(a);
                    c.into_iter()
                        .map(|x| {
                            let xv = self.env.eval(&x);
                            let k = if xv.is_finite() && target.is_finite() { ((target - xv) / (2.0 * PI)).round() } else { 0.0 };
                            if k != 0.0 { add(vec![x, mul(vec![int(2 * k as i64), pi()])]) } else { x }
                        })
                        .collect()
                } else {
                    c
                };
                c.into_iter().flat_map(|x| self.isolate(a, &x, v)).collect()
            }
            _ => vec![],
        }
    }

    fn solve_single(&mut self, eq: &Expr, v: u32) -> Option<Expr> {
        let c = self.candidates(eq, v, 0);
        self.pick(c, v, periodic_in(eq, v))
    }

    /// A point (x, y) on a circle and on a line through the circle's centre
    /// (a point at a given distance in a given direction, the usual result
    /// of a length and an angle dimension): centre ± radius · unit direction.
    /// Solving the pair by substitution instead leaves the square root of a
    /// perfect square that the simplifier rarely recognizes.
    fn solve_ray_circle(&mut self, eqs: &[Expr], vars: &[u32]) -> Option<Vec<(u32, Expr)>> {
        if eqs.len() != 2 || vars.len() != 2 {
            return None;
        }
        let ps: Vec<crate::expr::poly::Poly> = eqs.iter().map(|e| poly(&expand(e), vars)).collect::<Option<_>>()?;
        let (line, circ) = match (total_degree(&ps[0]), total_degree(&ps[1])) {
            (1, 2) => (&ps[0], &ps[1]),
            (2, 1) => (&ps[1], &ps[0]),
            _ => return None,
        };
        let get = |p: &crate::expr::poly::Poly, m: [u32; 2]| p.get(&m.to_vec()).cloned().unwrap_or_else(zero);
        let (a, b, c) = (get(line, [1, 0]), get(line, [0, 1]), get(line, [0, 0]));
        let (q, q2, qxy) = (get(circ, [2, 0]), get(circ, [0, 2]), get(circ, [1, 1]));
        if !self.env.is_zero(&qxy) || !self.env.is_zero(&sub(q.clone(), q2)) || self.env.is_zero(&q) {
            return None;
        }
        let tidy = |e: &Expr| {
            let l = trig_linearize(e).unwrap_or_else(|| e.clone());
            let f = best_form(e);
            if complexity(&l) <= complexity(&f) { l } else { f }
        };
        let px = tidy(&div(get(circ, [1, 0]), mul(vec![int(-2), q.clone()])));
        let py = tidy(&div(get(circ, [0, 1]), mul(vec![int(-2), q.clone()])));
        if !self.env.is_zero(&add(vec![mul(vec![a.clone(), px.clone()]), mul(vec![b.clone(), py.clone()]), c])) {
            return None;
        }
        let r2 = tidy(&sub(add(vec![sq(px.clone()), sq(py.clone())]), div(get(circ, [0, 0]), q)));
        let n2 = tidy(&add(vec![sq(a.clone()), sq(b.clone())]));
        let root = |this: &Self, e: Expr| resolve_abs(&deep_best(&sqrt(e)), this.env);
        let (r, n) = (root(self, r2), root(self, n2));
        if has_sqrt_of_sum(&n) || has_sqrt_of_sum(&r) {
            // the direction's length did not come out as a perfect square: no gain
            return None;
        }
        let (a, b) = (tidy(&a), tidy(&b));
        let mut best: Option<(f64, Expr, Expr)> = None;
        for s in [1i64, -1] {
            let x = add(vec![px.clone(), tidy(&div(mul(vec![int(s), r.clone(), b.clone()]), n.clone()))]);
            let y = add(vec![py.clone(), tidy(&div(mul(vec![int(-s), r.clone(), a.clone()]), n.clone()))]);
            let d = self.close(&x, vars[0]).max(self.close(&y, vars[1]));
            if best.as_ref().is_none_or(|bst| d < bst.0) {
                best = Some((d, x, y));
            }
        }
        let (d, x, y) = best?;
        if !(d < 1e-6) {
            return None;
        }
        if std::env::var("GXPAD_DEBUG").is_ok() {
            eprintln!("  point on a ray at a distance");
        }
        Some(vec![(vars[0], x), (vars[1], y)])
    }

    /// Solves a square block of equations for `vars`.
    fn solve_block(&mut self, eqs: Vec<Expr>, vars: Vec<u32>) -> Option<Vec<(u32, Expr)>> {
        let eqs: Vec<Expr> = eqs.iter().map(|e| self.prep(&simplify_radical_atoms(e, &vars, self.env), &vars)).collect();
        if let Some(r) = self.solve_ray_circle(&eqs, &vars) {
            return Some(r);
        }
        self.budget = 80;
        // Blocks of three or more unknowns go to polynomial elimination first
        // (with part of the remaining time): substitution there tends to nest
        // radicals that make every later block expensive. Smaller blocks go
        // to substitution first. Either way the other method gets a turn.
        let full = self.deadline;
        let with_share = |this: &mut Self, frac: f64, poly: bool| {
            let now = work_now();
            this.deadline = now + full.saturating_duration_since(now).mul_f64(frac);
            set_deadline(Some(this.deadline));
            let r = if poly { this.solve_block_poly_trig(&eqs, &vars) } else { this.search(eqs.clone(), vars.clone(), 0) };
            this.deadline = full;
            set_deadline(Some(full));
            r
        };
        let poly_first = vars.len() >= 3;
        let mut found = if poly_first { with_share(self, 0.45, true) } else { None };
        if found.is_none() {
            found = with_share(self, if poly_first { 0.6 } else { 0.4 }, false);
        }
        if found.is_none() && !poly_first {
            found = with_share(self, 1.0, true);
        }
        let steps = found?;
        let mut result: HashMap<u32, Expr> = HashMap::new();

        for (v, e) in steps.into_iter().rev() {
            let e2 = subst_map(&e, &result);
            // large intermediate results get a short time slice of their own:
            // the target is simplified again at the end, and a long pass here
            // rarely shrinks them much
            let e2 = if e2.size() > 200 {
                let slice = work_now() + scaled(60);
                set_deadline(Some(slice.min(full)));
                let r = simplify_result(&e2);
                set_deadline(Some(full));
                r
            } else {
                simplify_result(&e2)
            };
            result.insert(v, e2);
        }
        Some(result.into_iter().collect())
    }

    /// e = 0 without square roots of the unknowns: one root at a time is
    /// isolated and the equation squared (adding the mirror-image branch,
    /// which the polynomial path tells apart numerically). None when a root
    /// cannot be isolated.
    fn rationalize(&mut self, e: &Expr, vars: &[u32]) -> Option<Expr> {
        let mut e = e.clone();
        for _ in 0..4 {
            let eq_s = split_half_powers(&e);
            let mut rads = Vec::new();
            for &v in vars {
                sqrts_with(&eq_s, v, &mut rads);
            }
            let Some(s) = rads.first().cloned() else { return Some(e) };
            let Node::Pow(radicand, _) = s.node() else { return None };
            let radicand = radicand.clone();
            let t = self.new_tmp();
            let e2 = replace(&eq_s, &s, &sym(t));
            let cs = coeffs(&e2, t)?;
            if cs.len() > 3 || cs.iter().any(|c| contains(c, t)) {
                return None;
            }
            let c2 = cs.get(2).cloned().unwrap_or_else(zero);
            let a = &cs[0] + c2 * &radicand;
            let b = cs.get(1).cloned().unwrap_or_else(zero);
            e = self.prep(&expand(&(sq(a) - sq(b) * radicand)), vars);
        }
        None
    }

    /// Polynomial elimination for blocks where an unknown angle θ occurs
    /// only as cos θ and sin θ (a conic's rotation): c = cos θ and s = sin θ
    /// become polynomial unknowns tied by c² + s² = 1, and θ = atan2(s, c).
    fn solve_block_poly_trig(&mut self, eqs: &[Expr], vars: &[u32]) -> Option<Vec<(u32, Expr)>> {
        let mut eqs: Vec<Expr> = eqs.iter().map(|e| self.rationalize(e, vars).unwrap_or_else(|| e.clone())).collect();
        let mut vals = self.env.vals.clone();
        let mut new_vars: Vec<u32> = Vec::new();
        let mut angles: Vec<(u32, u32, u32)> = Vec::new();
        for &v in vars {
            let vv = sym(v);
            if !eqs.iter().any(|e| periodic_in(e, v)) {
                new_vars.push(v);
                continue;
            }
            let (cs, sn) = (self.new_tmp(), self.new_tmp());
            let swapped: Vec<Expr> = eqs.iter().map(|e| replace(&replace(e, &cos(vv.clone()), &sym(cs)), &sin(vv.clone()), &sym(sn))).collect();
            if swapped.iter().any(|e| contains(e, v)) {
                // θ also occurs some other way: leave it to the plain path
                return self.solve_block_poly(&eqs, vars);
            }
            eqs = swapped;
            eqs.push(sq(sym(cs)) + sq(sym(sn)) - one());
            let t = self.env.get(v);
            vals.insert(cs, t.cos());
            vals.insert(sn, t.sin());
            new_vars.push(cs);
            new_vars.push(sn);
            angles.push((v, cs, sn));
        }
        if angles.is_empty() {
            return self.solve_block_poly(&eqs, vars);
        }
        if std::env::var("GXPAD_DEBUG").is_ok() {
            eprintln!("  angles as cos/sin unknowns: {:?}", angles.iter().map(|a| a.0).collect::<Vec<_>>());
        }
        let env2 = Env { vals };
        let mut sub = BlockSolver { env: &env2, tmp: self.tmp, budget: self.budget, deadline: self.deadline, keep: self.keep };
        let r = sub.solve_block_poly(&eqs, &new_vars);
        self.tmp = sub.tmp;
        let r: HashMap<u32, Expr> = r?.into_iter().collect();
        let mut out: Vec<(u32, Expr)> = Vec::new();
        for &v in vars {
            if let Some(e) = r.get(&v) {
                out.push((v, e.clone()));
            }
        }
        for (v, cs, sn) in angles {
            let mut th = atan2(r.get(&sn)?.clone(), r.get(&cs)?.clone());
            // the branch of the current angle
            let (t, x) = (self.env.get(v), self.env.eval(&th));
            let k = ((t - x) / (2.0 * PI)).round();
            if k != 0.0 && k.is_finite() {
                th = add(vec![th, mul(vec![int(2 * k as i64), pi()])]);
            }
            out.push((v, deep_best(&th)));
        }
        Some(out)
    }

    /// Polynomial elimination down to one variable. Each round normalizes
    /// the system (row reduction over monomials, then only the factors that
    /// vanish at the current solution), and removes one variable: one that
    /// occurs in a single equation is set aside, one that occurs linearly is
    /// substituted, otherwise resultants eliminate the cheapest variable.
    /// The last variable comes from its lowest-degree vanishing factor and
    /// the others by back-substitution.
    fn solve_block_poly(&mut self, eqs: &[Expr], vars: &[u32]) -> Option<Vec<(u32, Expr)>> {
        let dbg = std::env::var("GXPAD_DEBUG").is_ok();
        macro_rules! fail {
            ($($t:tt)*) => {{
                if dbg {
                    eprintln!("  poly fail: {} (deadline {})", format!($($t)*), past_deadline());
                }
                return None;
            }};
        }
        // solving for a target alone has its own time slice instead
        let limit = if EXTRA_MS.with(|c| c.get()) > 0 || self.keep.is_some() { usize::MAX } else { POLY_VARS_NORMAL };
        if vars.is_empty() || vars.len() > limit {
            fail!("{} vars", vars.len());
        }
        let mut ctx = PolyCtx::new(vars);
        for e in eqs {
            if !ctx.collect(e) {
                fail!("not polynomial");
            }
        }
        let Some(mut polys) = eqs.iter().map(|e| ctx.to_mpoly(e)).collect::<Option<Vec<MPoly>>>() else { fail!("conversion") };
        polys.retain(|p| !p.is_zero());
        let merged = merge_trig(&mut ctx, &mut polys, self.env);
        if dbg {
            eprintln!("  poly block: {} eqs, {} vars, {} atoms, terms {:?}", polys.len(), nv_of(vars), ctx.atoms.len(), polys.iter().map(|p| p.terms()).collect::<Vec<_>>());
        }
        if merged.is_empty() {
            return self.solve_polys(&ctx, polys, vars, false);
        }
        // tan(θ/2) atoms stay plain symbols until the results are simplified,
        // so factors such as 1 + tan²(θ/2) can cancel
        let mut vals = self.env.vals.clone();
        let mut back = Vec::new();
        for (i, kind) in merged {
            let t = self.new_tmp();
            vals.insert(t, self.env.eval(&ctx.atoms[i]));
            ctx.atoms[i] = sym(t);
            back.push((t, kind));
        }
        let env2 = Env { vals };
        let mut sub = BlockSolver { env: &env2, tmp: self.tmp, budget: self.budget, deadline: self.deadline, keep: self.keep };
        let r = sub.solve_polys(&ctx, polys, vars, true);
        self.tmp = sub.tmp;
        let r = r?;
        let t0 = web_time::Instant::now();
        let mut out: HashMap<u32, Expr> = r
            .into_iter()
            .map(|(v, e)| {
                // plain substitution of the atoms, and the conversion of
                // rational functions of them, kept when at most half the size
                let atoms: HashMap<u32, Expr> = back.iter().map(|(t, kind)| (*t, kind.expr())).collect();
                let plain = deep_best(&resolve_abs(&subst_map(&e, &atoms), self.env));
                let mut x = e;
                for (t, kind) in &back {
                    x = untrig_atom(&x, *t, kind);
                }
                let x = resolve_abs(&deep_best(&resolve_abs(&x, self.env)), self.env);
                let x = combine_shared_denominators(&x);
                let x = deep_best(&half_angles(&x, self.env));
                // a modest gain mixes forms that combine worse in the target
                (v, if 2 * complexity(&x) <= complexity(&plain) { x } else { plain })
            })
            .collect();
        if self.keep.is_none() {
            self.resolve_smaller(eqs, vars, &mut out);
        }
        if dbg {
            eprintln!("  trig atoms restored in {:?}", t0.elapsed());
        }
        Some(out.into_iter().collect())
    }

    /// The elimination part of `solve_block_poly`, on polynomials in `ctx`.
    fn solve_polys(&mut self, ctx: &PolyCtx, mut polys: Vec<MPoly>, vars: &[u32], trig_atoms: bool) -> Option<Vec<(u32, Expr)>> {
        let dbg = std::env::var("GXPAD_DEBUG").is_ok();
        macro_rules! fail {
            ($($t:tt)*) => {{
                if dbg {
                    eprintln!("  poly fail: {} (deadline {})", format!($($t)*), past_deadline());
                }
                return None;
            }};
        }
        let vals = ctx.values(self.env);
        let orig = polys.clone();
        let nv = vars.len();
        let keep = self.keep.and_then(|k| vars.iter().position(|v| *v == k));
        let kept = |y: usize| Some(y) == keep;
        let mut active: Vec<usize> = (0..nv).collect();
        // (variable, its equation, resultant partners)
        let mut steps: Vec<(usize, MPoly, Vec<MPoly>)> = Vec::new();
        let mut fresh = true;
        while active.len() > 1 {
            if past_deadline() {
                fail!("deadline at {} vars", active.len());
            }
            // normalize
            if fresh {
                let t0 = web_time::Instant::now();
                let red = interreduce(&polys, &active);
                let before: usize = polys.iter().map(|p| p.terms()).sum();
                let after: usize = red.iter().map(|p| p.terms()).sum();
                if after <= 2 * before {
                    polys = red;
                }
                polys = polys.iter().map(|p| vanishing_part(p, &active, &vals)).collect();
                if past_deadline() {
                    fail!("deadline while normalizing");
                }
                if dbg {
                    eprintln!("  normalized {} eqs, {} terms in {:?}", polys.len(), polys.iter().map(|p| p.terms()).sum::<usize>(), t0.elapsed());
                }
                fresh = false;
            }
            let uses = |p: &MPoly, y: usize| p.uses(y);
            if keep.is_some() {
                // variables no equation uses any more do not matter for the kept one
                active.retain(|&y| kept(y) || polys.iter().any(|p| uses(p, y)));
                if active.len() <= 1 {
                    break;
                }
            }
            // a variable in a single equation: that equation determines it last
            // (never the kept variable's equation: that one is needed)
            if let Some(&y) = active
                .iter()
                .find(|&&y| !kept(y) && polys.iter().filter(|p| uses(p, y)).count() == 1 && !keep.is_some_and(|k| polys.iter().any(|p| uses(p, y) && uses(p, k))))
            {
                let i = polys.iter().position(|p| uses(p, y)).unwrap();
                let e = polys.remove(i);
                if dbg {
                    eprintln!("  set aside v{y} ({} terms)", e.terms());
                }
                steps.push((y, e, vec![]));
                active.retain(|&v| v != y);
                continue;
            }
            if let Some(&y) = active.iter().find(|&&y| !polys.iter().any(|p| uses(p, y))) {
                fail!("v{y} undetermined");
            }
            // a linear occurrence: substitute
            let mut lin: Option<((usize, usize, usize), usize, usize)> = None;
            for (i, p) in polys.iter().enumerate() {
                for &y in &active {
                    if kept(y) || p.deg(y) != 1 {
                        continue;
                    }
                    let cs = p.coeffs(y);
                    let (v1, a1) = cs[1].eval_scaled(&vals);
                    if !(v1.abs() > 1e-7 * a1.max(1e-300)) {
                        continue;
                    }
                    let cvars = active.iter().filter(|&&v| cs[1].uses(v)).count();
                    let key = (cvars, cs[1].terms(), p.terms());
                    if lin.as_ref().is_none_or(|b| key < b.0) {
                        lin = Some((key, i, y));
                    }
                }
            }
            if let Some((_, i, y)) = lin {
                let e = polys.remove(i);
                let cs = e.coeffs(y);
                let t0 = web_time::Instant::now();
                let mut next = Vec::new();
                for p in polys.drain(..) {
                    if !p.uses(y) {
                        next.push(p);
                        continue;
                    }
                    let r = strip_factor(&subst_linear(&p, y, &cs[0], &cs[1]), &cs[1]);
                    if r.is_zero() {
                        continue;
                    }
                    next.push(r);
                }
                if dbg {
                    eprintln!("  substitute v{y} (coefficient {} terms): {} terms in {:?}", cs[1].terms(), next.iter().map(|p| p.terms()).sum::<usize>(), t0.elapsed());
                }
                steps.push((y, e, vec![]));
                polys = next;
                active.retain(|&v| v != y);
                fresh = true;
                continue;
            }
            // resultants in the variable with the smallest expected degree growth
            let mut best: Option<((u64, usize), usize, usize)> = None;
            for &y in active.iter().filter(|&&y| !kept(y)) {
                let with: Vec<usize> = (0..polys.len()).filter(|&i| polys[i].uses(y)).collect();
                for &piv in &with {
                    let e = &polys[piv];
                    let mut cost = 0u64;
                    for &j in &with {
                        if j == piv {
                            continue;
                        }
                        let p = &polys[j];
                        // degree of res_y(e, p) in the others: deg_y(e) deg(p) + deg_y(p) deg(e)
                        let g: u64 = active
                            .iter()
                            .filter(|&&v| v != y)
                            .map(|&v| (e.deg(y) * p.deg(v) + p.deg(y) * e.deg(v)) as u64)
                            .sum();
                        cost = cost.max(g);
                    }
                    let key = (cost, e.terms() * with.len());
                    if best.as_ref().is_none_or(|b| key < b.0) {
                        best = Some((key, y, piv));
                    }
                }
            }
            let Some((_, y, piv)) = best else { fail!("no pivot") };
            let e = polys.remove(piv);
            let rest: Vec<usize> = active.iter().copied().filter(|&v| v != y).collect();
            let mut partners = Vec::new();
            let mut next = Vec::new();
            for p in polys.drain(..) {
                if !p.uses(y) {
                    next.push(p);
                    continue;
                }
                let t0 = web_time::Instant::now();
                let Some(r) = resultant(&e, &p, y) else { fail!("resultant in v{y}") };
                partners.push(p);
                let vp = vanishing_part(&r, &rest, &vals);
                if past_deadline() {
                    fail!("deadline while factoring a resultant of {} terms in v{y}", r.terms());
                }
                if dbg {
                    eprintln!("  eliminate v{y}: resultant {} terms (deg {:?}) -> factor {} terms, {:?}",
                        r.terms(), rest.iter().map(|&v| r.deg(v)).collect::<Vec<_>>(), vp.terms(), t0.elapsed());
                }
                next.push(vp);
            }
            steps.push((y, e, partners));
            polys = next;
            active = rest;
            fresh = true;
        }
        let x = match keep {
            Some(k) if active == [k] => k,
            Some(_) => fail!("kept variable eliminated"),
            None => active[0],
        };
        if dbg {
            for p in &polys {
                eprintln!("  remaining poly: {} terms, deg in v{x} {}", p.terms(), p.deg(x));
            }
        }
        if let Ok(path) = std::env::var("GXPAD_DUMP") {
            if let Some(p) = polys.iter().find(|p| p.uses(x)) {
                let mut s = format!("{} {}\n", p.n, x);
                for (m, c) in &p.t {
                    s += &format!("{} {}\n", c, m.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(" "));
                }
                s += &format!("vals {}\n", vals.iter().map(|v| format!("{v:e}")).collect::<Vec<_>>().join(" "));
                let _ = std::fs::write(path, s);
            }
        }
        let mut best: Option<MPoly> = None;
        for p in polys.iter().filter(|p| p.uses(x)) {
            let all: Vec<usize> = (0..p.n).filter(|&v| p.uses(v)).collect();
            let main = cheapest_main(p, &all).unwrap_or(x);
            for f in factor_main(p, main) {
                if f.uses(x) && f.vanishes(&vals) && best.as_ref().is_none_or(|b| f.deg(x) < b.deg(x)) {
                    best = Some(f);
                }
            }
        }
        if past_deadline() {
            fail!("deadline while factoring the last polynomial");
        }
        if dbg {
            eprintln!("  last var v{x}: factor degree {:?}", best.as_ref().map(|b| b.deg(x)));
        }
        let Some(f) = best else { fail!("no vanishing factor in v{x}") };
        let xs = vars[x];
        let fx = self.prep(&ctx.to_expr(&f), &[xs]);
        let c = self.candidates(&fx, xs, 0);
        let Some(xe) = self.pick(c, xs, periodic_in(&fx, xs)) else { fail!("no root of the degree {} factor in v{x}", f.deg(x)) };
        let xe = deep_best(&resolve_abs(&xe, self.env));
        if keep.is_some() {
            return Some(vec![(xs, xe)]);
        }
        let mut solved: HashMap<u32, Expr> = HashMap::new();
        solved.insert(xs, xe);
        for (y, e, partners) in steps.into_iter().rev() {
            if past_deadline() {
                fail!("deadline in back-substitution");
            }
            let ys = vars[y];
            let mut found: Option<Expr> = None;
            'outer: for p in &partners {
                for sr in subresultant_chain(&e, p, y) {
                    if sr.deg(y) != 1 {
                        continue;
                    }
                    let cs = sr.coeffs(y);
                    let c1 = subst_map(&ctx.to_expr(&cs[1]), &solved);
                    if !(self.env.eval(&c1).abs() > 1e-12) {
                        continue;
                    }
                    let sol = neg(div(subst_map(&ctx.to_expr(&cs[0]), &solved), c1));
                    let sol = deep_best(&resolve_abs(&sol, self.env));
                    if self.close(&sol, ys) < 1e-6 {
                        found = Some(sol);
                        break 'outer;
                    }
                }
            }
            if found.is_none() {
                let ee = self.prep(&subst_map(&ctx.to_expr(&e), &solved), &[ys]);
                let c = self.candidates(&ee, ys, 0);
                found = self.pick(c, ys, periodic_in(&ee, ys));
            }
            let Some(sol) = found else { fail!("back-substitution of v{y}") };
            solved.insert(ys, sol);
        }
        // with trig atoms the caller does this after converting them back
        if !trig_atoms {
            let orig: Vec<Expr> = orig.iter().map(|p| ctx.to_expr(p)).collect();
            self.resolve_smaller(&orig, vars, &mut solved);
        }
        Some(vars.iter().map(|v| (*v, solved[v].clone())).collect())
    }

    /// Solves for `target` (an expression in `vars`) directly: a new unknown
    /// t with t = target joins `eqs`, and polynomial elimination removes all
    /// of `vars`, leaving a polynomial in t whose vanishing factor gives it.
    fn solve_target(&mut self, eqs: &[Expr], vars: &[u32], target: &Expr, actual: f64) -> Option<Expr> {
        let t = self.new_tmp();
        let mut vals = self.env.vals.clone();
        vals.insert(t, actual);
        let env2 = Env { vals };
        let mut all = vars.to_vec();
        all.push(t);
        let mut bs = BlockSolver { env: &env2, tmp: self.tmp, budget: 80, deadline: self.deadline, keep: Some(t) };
        let es: Vec<Expr> = eqs
            .iter()
            .chain(std::iter::once(&sub(sym(t), target.clone())))
            .map(|e| bs.prep(&simplify_radical_atoms(e, &all, &env2), &all))
            .collect();
        let r = bs.solve_block_poly_trig(&es, &all);
        self.tmp = bs.tmp;
        r?.into_iter().find(|(v, _)| *v == t).map(|(_, e)| e)
    }

    /// Solves for the value of `e`, a formula with square roots, directly:
    /// each root becomes an unknown r with r² = its radicand (innermost
    /// first), and polynomial elimination of the roots leaves a polynomial
    /// in a new unknown t = e. The roots already sit on the right branches,
    /// so this avoids the degenerate solutions of the original equations.
    fn solve_target_radicals(&mut self, e: &Expr, actual: f64) -> Option<Expr> {
        let mut vals = self.env.vals.clone();
        let mut roots: Vec<(u32, Expr)> = Vec::new();
        let body = self.abstract_roots(e, &mut roots, &mut vals)?;
        if roots.is_empty() || roots.len() > 8 {
            return None;
        }
        let t = self.new_tmp();
        vals.insert(t, actual);
        let env2 = Env { vals };
        let mut all: Vec<u32> = roots.iter().map(|r| r.0).collect();
        all.push(t);
        let mut eqs: Vec<Expr> = roots.iter().map(|(r, b)| sub(sq(sym(*r)), b.clone())).collect();
        eqs.push(sub(sym(t), body));
        let mut bs = BlockSolver { env: &env2, tmp: self.tmp, budget: 80, deadline: self.deadline, keep: Some(t) };
        let es: Vec<Expr> = eqs.iter().map(|e| bs.prep(e, &all)).collect();
        let r = bs.solve_block_poly(&es, &all);
        self.tmp = bs.tmp;
        r?.into_iter().find(|(v, _)| *v == t).map(|(_, e)| e)
    }

    /// `e` with each square root (and half-integer power) replaced by a power
    /// of a new symbol, recorded with its radicand. None for other non-integer powers.
    fn abstract_roots(&mut self, e: &Expr, roots: &mut Vec<(u32, Expr)>, vals: &mut HashMap<u32, f64>) -> Option<Expr> {
        Some(match e.node() {
            Node::Num(_) | Node::Pi | Node::Sym(_) => e.clone(),
            Node::Add(ts) => add(ts.iter().map(|t| self.abstract_roots(t, roots, vals)).collect::<Option<Vec<_>>>()?),
            Node::Mul(ts) => mul(ts.iter().map(|t| self.abstract_roots(t, roots, vals)).collect::<Option<Vec<_>>>()?),
            Node::Pow(b, x) => {
                let b2 = self.abstract_roots(b, roots, vals)?;
                if let Some(k) = x.as_int() {
                    return Some(pow(b2, int(k)));
                }
                let r = x.as_rat().filter(|r| r.d == 2)?;
                let s = match roots.iter().find(|(_, rb)| *rb == b2) {
                    Some((s, _)) => *s,
                    None => {
                        let s = self.new_tmp();
                        vals.insert(s, eval(&b2, &|v| vals.get(&v).copied().unwrap_or(f64::NAN)).sqrt());
                        roots.push((s, b2));
                        s
                    }
                };
                pow(sym(s), int(r.n as i64))
            }
            Node::Fun(_, a) | Node::Atan2(a, _) if !sym_set(a).is_empty() && self.has_root(e) => return None,
            _ => e.clone(),
        })
    }

    fn has_root(&self, e: &Expr) -> bool {
        match e.node() {
            Node::Pow(b, x) => x.as_int().is_none() || self.has_root(b),
            Node::Add(ts) | Node::Mul(ts) => ts.iter().any(|t| self.has_root(t)),
            Node::Fun(_, a) => self.has_root(a),
            Node::Atan2(a, b) => self.has_root(a) || self.has_root(b),
            _ => false,
        }
    }

    /// Back-substitution follows the elimination order, so a variable can
    /// come out of a large subresultant although one of the block's own
    /// equations of degree ≤ 2 in it gives it far more simply. Each variable,
    /// smallest result first, is solved again from such equations (with the
    /// other variables substituted), and a matching root at most half the
    /// size, with no radical the block's results do not already have,
    /// replaces the result.
    fn resolve_smaller(&mut self, eqs: &[Expr], vars: &[u32], solved: &mut HashMap<u32, Expr>) {
        let slice = (work_now() + scaled(100)).min(self.deadline);
        set_deadline(Some(slice));
        self.resolve_smaller_inner(eqs, vars, solved);
        set_deadline(Some(self.deadline));
    }

    fn resolve_smaller_inner(&mut self, eqs: &[Expr], vars: &[u32], solved: &mut HashMap<u32, Expr>) {
        // results this small are not worth the time
        const SMALL: usize = 40;
        let mut order: Vec<usize> = (0..vars.len()).filter(|&i| complexity(&solved[&vars[i]]) > SMALL).collect();
        order.sort_by_key(|&i| complexity(&solved[&vars[i]]));
        for i in order {
            let ys = vars[i];
            for eq in eqs {
                if past_deadline() {
                    return;
                }
                if !contains(eq, ys) {
                    continue;
                }
                let others: HashMap<u32, Expr> = solved.iter().filter(|(v, _)| **v != ys).map(|(v, e)| (*v, e.clone())).collect();
                // a root is about as large as the equation's coefficients, so
                // an equation larger than the current result cannot do better
                let ee = subst_map(eq, &others);
                if ee.size() > 1500 || complexity(&ee) > complexity(&solved[&ys]) {
                    continue;
                }
                let ee = self.prep(&ee, &[ys]);
                if !coeffs(&expand(&ee), ys).is_some_and(|cs| (2..=3).contains(&cs.len())) {
                    continue;
                }
                let c = self.candidates(&ee, ys, 0);
                if let Some(sol) = self.pick(c, ys, false) {
                    let raw = resolve_abs(&sol, self.env);
                    // a slightly smaller form can combine worse with the rest of
                    // the formula, so only a clear improvement replaces the result
                    let cur = complexity(&solved[&ys]);
                    if has_float(&raw) || complexity(&raw) >= cur {
                        continue;
                    }
                    let sol = deep_best(&raw);
                    // nor may it bring in a radical that none of the block's results have
                    let mut known = std::collections::BTreeSet::new();
                    solved.values().for_each(|e| radicands(e, &mut known));
                    let mut new = std::collections::BTreeSet::new();
                    radicands(&sol, &mut new);
                    if 2 * complexity(&sol) <= cur && new.is_subset(&known) && self.close(&sol, ys) < 1e-6 {
                        if std::env::var("GXPAD_DEBUG").is_ok() {
                            let f = |e: &Expr| crate::expr::bx::plain(&crate::expr::bx::to_bx(e, &|s| format!("s{s}")));
                            eprintln!("  re-solved s{ys}: {} -> {}", f(&solved[&ys]), f(&sol));
                        }
                        solved.insert(ys, sol);
                    }
                }
            }
        }
    }

    /// Roots of a high-degree polynomial equation in v from its factors.
    fn factor_roots(&mut self, eq: &Expr, v: u32, depth: u32) -> Vec<Expr> {
        let mut ctx = PolyCtx::new(&[v]);
        if !ctx.collect(eq) {
            return vec![];
        }
        let Some(p) = ctx.to_mpoly(eq) else { return vec![] };
        let vals = ctx.values(self.env);
        let total = p.deg(0);
        let mut out = Vec::new();
        let fs = factor_main(&p, 0);
        if past_deadline() {
            return out;
        }
        for f in fs {
            if f.uses(0) && f.deg(0) < total && f.vanishes(&vals) {
                let fe = self.prep(&ctx.to_expr(&f), &[v]);
                out.extend(self.candidates_inner(&fe, v, depth + 1));
            }
        }
        out
    }

    /// Depth-first elimination: tries the preferred step first and, on a dead
    /// end, a few alternatives at each level, within a step budget.
    fn search(&mut self, mut eqs: Vec<Expr>, vars: Vec<u32>, combos: u32) -> Option<Vec<(u32, Expr)>> {
        if vars.is_empty() {
            return Some(vec![]);
        }
        if self.budget == 0 || work_now() > self.deadline || cancelled() {
            return None;
        }
        self.budget -= 1;
        eqs.retain(|e| contains_any(e, &vars));
        if eqs.is_empty() {
            return None;
        }
        let dbg = std::env::var("GXPAD_DEBUG").is_ok();
        let mut moves: Vec<(usize, u32, Expr)> = Vec::new();
        moves.extend(self.find_linear(&eqs, &vars).into_iter().take(3));
        if eqs.len() >= 2 && combos < 4 {
            if let Some((i, ne)) = self.combine_pair(&eqs, &vars) {
                // a combination comes before other moves once no linear step exists
                let at = if moves.is_empty() { 0 } else { 1.min(moves.len()) };
                moves.insert(at, (i, u32::MAX, ne));
            }
        }
        if moves.len() < 3 {
            moves.extend(self.find_single(&eqs, &vars));
        }
        if moves.len() < 3 {
            moves.extend(self.find_quadratic_multi(&eqs, &vars));
        }
        if dbg && moves.is_empty() {
            eprintln!("     stuck with vars {vars:?}");
            for e in &eqs {
                eprintln!("       {}", crate::expr::bx::plain(&crate::expr::bx::to_bx(e, &|s| format!("s{s}"))));
            }
        }
        for (i, v, sol) in moves.into_iter().take(4) {
            if dbg {
                eprintln!("     step eq{i} var s{v} = {}", crate::expr::bx::plain(&crate::expr::bx::to_bx(&sol, &|s| format!("s{s}"))));
            }
            if v == u32::MAX {
                let mut e2 = eqs.clone();
                e2[i] = self.prep(&sol, &vars);
                if let Some(r) = self.search(e2, vars.clone(), combos + 1) {
                    return Some(r);
                }
                continue;
            }
            let sol = if sol.size() < 600 {
                let s1 = deep_best(&resolve_abs(&sol, self.env));
                let s2 = resolve_abs(&s1, self.env);
                if s2 == s1 { s1 } else { deep_best(&s2) }
            } else {
                sol
            };
            // a step must reproduce the current solution (rejects degenerate branches)
            if !(self.close(&sol, v) < 1e-6) {
                continue;
            }
            let rest_vars: Vec<u32> = vars.iter().copied().filter(|x| *x != v).collect();
            let rest: Vec<Expr> = eqs
                .iter()
                .enumerate()
                .filter(|(k, _)| *k != i)
                .map(|(_, e)| self.prep(&subst(e, &|x| if x == v { Some(sol.clone()) } else { None }), &rest_vars))
                .collect();
            if let Some(mut r) = self.search(rest, rest_vars, combos) {
                r.insert(0, (v, sol));
                return Some(r);
            }
            if self.budget == 0 {
                return None;
            }
        }
        None
    }

    fn find_linear(&self, eqs: &[Expr], vars: &[u32]) -> Vec<(usize, u32, Expr)> {
        // score: (equations where v enters nonlinearly, equations containing v, vars in eq, size)
        let mut nonlin: HashMap<u32, (usize, usize)> = HashMap::new();
        for &v in vars {
            let mut bad = 0;
            let mut cnt = 0;
            for e in eqs {
                if !contains(e, v) {
                    continue;
                }
                cnt += 1;
                if coeffs(e, v).is_none_or(|c| c.len() > 2) {
                    bad += 1;
                }
            }
            nonlin.insert(v, (bad, cnt));
        }
        let mut all: Vec<((usize, usize, usize, usize), usize, u32, Expr)> = Vec::new();
        for (i, e) in eqs.iter().enumerate() {
            let nv = vars.iter().filter(|v| contains(e, **v)).count();
            for &v in vars {
                if !contains(e, v) {
                    continue;
                }
                let Some(cs) = coeffs(e, v) else { continue };
                if cs.len() != 2 {
                    continue;
                }
                let c1v = self.env.eval(&cs[1]);
                if !(c1v.abs() > 1e-10) {
                    continue;
                }
                let sol = neg(div(cs[0].clone(), cs[1].clone()));
                let (bad, cnt) = nonlin[&v];
                let key = (bad, cnt, nv, sol.size());
                all.push((key, i, v, sol));
            }
        }
        all.sort_by(|a, b| a.0.cmp(&b.0));
        // one option per variable keeps the branching meaningful
        let mut seen = HashSet::new();
        all.into_iter().filter(|o| seen.insert(o.2)).map(|b| (b.1, b.2, b.3)).collect()
    }

    fn combine_pair(&self, eqs: &[Expr], vars: &[u32]) -> Option<(usize, Expr)> {
        let polys: Vec<Option<crate::expr::poly::Poly>> = eqs.iter().map(|e| poly(e, vars)).collect();
        for i in 0..eqs.len() {
            let Some(pi_) = &polys[i] else { continue };
            if total_degree(pi_) != 2 {
                continue;
            }
            for j in 0..eqs.len() {
                if i == j {
                    continue;
                }
                let Some(pj) = &polys[j] else { continue };
                if total_degree(pj) != 2 {
                    continue;
                }
                let qi: Vec<(&Vec<u32>, &Expr)> = pi_.iter().filter(|(m, _)| m.iter().sum::<u32>() == 2).collect();
                let qj: Vec<(&Vec<u32>, &Expr)> = pj.iter().filter(|(m, _)| m.iter().sum::<u32>() == 2).collect();
                if qi.len() != qj.len() || qi.iter().zip(&qj).any(|(a, b)| a.0 != b.0) {
                    continue;
                }
                let k = div(qi[0].1.clone(), qj[0].1.clone());
                if !qi.iter().zip(&qj).all(|(a, b)| self.env.is_zero(&(a.1 - &k * b.1))) {
                    continue;
                }
                let mut np = crate::expr::poly::Poly::new();
                for (m, c) in pi_ {
                    if m.iter().sum::<u32>() < 2 {
                        np.insert(m.clone(), c.clone());
                    }
                }
                for (m, c) in pj {
                    if m.iter().sum::<u32>() < 2 {
                        let e = np.entry(m.clone()).or_insert_with(zero);
                        *e = &*e - &k * c;
                    }
                }
                let ne = from_poly(&np, vars);
                if !contains_any(&ne, vars) {
                    continue;
                }
                return Some((i, ne));
            }
        }
        None
    }

    fn find_single(&mut self, eqs: &[Expr], vars: &[u32]) -> Vec<(usize, u32, Expr)> {
        let mut order: Vec<usize> = (0..eqs.len()).collect();
        order.sort_by_key(|&i| eqs[i].size());
        let mut out = Vec::new();
        for i in order {
            let inv: Vec<u32> = vars.iter().copied().filter(|v| contains(&eqs[i], *v)).collect();
            if inv.len() != 1 {
                continue;
            }
            if let Some(s) = self.solve_single(&eqs[i], inv[0]) {
                out.push((i, inv[0], s));
                if out.len() >= 2 {
                    break;
                }
            }
        }
        out
    }

    fn find_quadratic_multi(&mut self, eqs: &[Expr], vars: &[u32]) -> Vec<(usize, u32, Expr)> {
        let mut all: Vec<(usize, usize, u32, Expr)> = Vec::new();
        for (i, e) in eqs.iter().enumerate() {
            for &v in vars {
                if !contains(e, v) {
                    continue;
                }
                let Some(cs) = coeffs(e, v) else { continue };
                if cs.len() != 3 {
                    continue;
                }
                if let Some(s) = self.pick(self.quad_roots(&cs), v, false) {
                    all.push((s.size(), i, v, s));
                }
            }
        }
        all.sort_by_key(|a| a.0);
        all.into_iter().take(3).map(|b| (b.1, b.2, b.3)).collect()
    }
}

pub fn simplify_result(e: &Expr) -> Expr {
    deep_best(e)
}

fn zero_abs(e: &Expr, env: &Env, out: &mut Vec<Expr>) {
    match e.node() {
        Node::Fun(Fun::Abs, a) => {
            if env.eval(a).abs() <= 1e-12 && !out.contains(e) {
                out.push(e.clone());
            }
            zero_abs(a, env, out);
        }
        Node::Num(_) | Node::Pi | Node::Sym(_) => {}
        Node::Add(ts) | Node::Mul(ts) => ts.iter().for_each(|t| zero_abs(t, env, out)),
        Node::Pow(a, b) | Node::Atan2(a, b) => {
            zero_abs(a, env, out);
            zero_abs(b, env, out);
        }
        Node::Fun(_, a) => zero_abs(a, env, out),
    }
}

/// |a| with a = 0 in the current configuration cannot be resolved by sign,
/// and leaves 0/0 forms such as a/|a|. Picks the ± replacement for each that
/// reproduces the measured value.
fn settle_zero_abs(e: Expr, env: &Env, actual: f64) -> Expr {
    let ok = |x: &Expr| {
        let v = env.eval(x);
        v.is_finite() && (v - actual).abs() <= 1e-8 * (1.0 + actual.abs())
    };
    if ok(&e) {
        return e;
    }
    let mut abs_nodes = Vec::new();
    zero_abs(&e, env, &mut abs_nodes);
    abs_nodes.truncate(4);
    let mut best: Option<Expr> = None;
    for mask in 0..(1u32 << abs_nodes.len()) {
        let mut x = e.clone();
        for (i, node) in abs_nodes.iter().enumerate() {
            let Node::Fun(_, a) = node.node() else { continue };
            let r = if mask & (1 << i) == 0 { a.clone() } else { neg(a.clone()) };
            x = replace(&x, node, &r);
        }
        let x = simplify_result(&x);
        if ok(&x) && best.as_ref().is_none_or(|b| complexity(&x) < complexity(b)) {
            best = Some(x);
        }
    }
    best.unwrap_or(e)
}

/// |a| with a = 0 at the current configuration often marks a branch point
/// where two solution branches meet: |a| fits the measurement there but not
/// nearby. Perturbs the named parameters of the formula both ways, re-solves
/// the constraints numerically, and keeps the sign choice for each such |a|
/// that matches the measurement on both sides.
fn settle_branch(e: Expr, env: &Env, rows: &[Expr], unknowns: &[u32], target: &Expr, named: &HashSet<u32>) -> Expr {
    let mut abs_nodes = Vec::new();
    zero_abs(&e, env, &mut abs_nodes);
    abs_nodes.truncate(3);
    if abs_nodes.is_empty() || rows.is_empty() {
        return e;
    }
    // (in symbol order: a set's own order differs from run to run)
    let mut params: Vec<u32> = sym_set(&e).into_iter().filter(|s| named.contains(s) && !unknowns.contains(s)).collect();
    params.sort();
    params.truncate(4);
    if params.is_empty() {
        return e;
    }
    let mut cands: Vec<Expr> = vec![e.clone()];
    for mask in 0..(1u32 << abs_nodes.len()) {
        let mut x = e.clone();
        for (i, node) in abs_nodes.iter().enumerate() {
            let Node::Fun(_, a) = node.node() else { continue };
            let r = if mask & (1 << i) == 0 { a.clone() } else { neg(a.clone()) };
            x = replace(&x, node, &r);
        }
        cands.push(simplify_result(&x));
    }
    let n = env.vals.keys().filter(|k| **k < TMP_BASE).max().copied().unwrap_or(0) as usize + 1;
    let mut base = vec![0.0; n];
    for (k, v) in &env.vals {
        if (*k as usize) < n {
            base[*k as usize] = *v;
        }
    }
    let owned: Vec<(Expr, Owner)> = rows.iter().map(|e| (e.clone(), Owner::Ent(0))).collect();
    let sys = crate::solver::Sys::build(&owned, unknowns.iter().map(|u| *u as usize).collect());
    let mut ok = vec![true; cands.len()];
    let mut trials = 0;
    for &p in &params {
        let x0 = base[p as usize];
        let d = 1e-3 * (1.0 + x0.abs());
        for sign in [1.0, -1.0] {
            if past_deadline() {
                return e;
            }
            let mut v = base.clone();
            let mut solved = true;
            for k in 1..=4 {
                v[p as usize] = x0 + sign * d * k as f64 / 4.0;
                if !sys.solve(&mut v, &[], [0.0; 2]) {
                    solved = false;
                    break;
                }
            }
            if !solved {
                continue;
            }
            let get = |s: u32| v.get(s as usize).copied().unwrap_or(f64::NAN);
            let t = eval(target, &get);
            if !t.is_finite() {
                continue;
            }
            trials += 1;
            for (i, c) in cands.iter().enumerate() {
                let f = eval(c, &get);
                if !(f.is_finite() && (f - t).abs() <= 1e-7 * (1.0 + t.abs())) {
                    ok[i] = false;
                }
            }
        }
    }
    if trials == 0 || ok[0] {
        return e;
    }
    cands.into_iter().zip(ok).filter(|(_, k)| *k).map(|(c, _)| c).min_by_key(complexity).unwrap_or(e)
}

/// Whether formula `f` keeps giving the target when the parameters it uses
/// (up to 4) move slightly and the construction is solved again numerically.
/// A formula can match at the current configuration only by coincidence
/// (two branches meeting at 60°, say). None when no perturbed configuration
/// could be solved.
fn agrees_when_perturbed(f: &Expr, env: &Env, rows: &[Expr], unknowns: &[u32], target: &Expr, named: &HashSet<u32>) -> Option<bool> {
    // (in symbol order: a set's own order differs from run to run)
    let mut params: Vec<u32> = sym_set(f).into_iter().filter(|s| named.contains(s) && !unknowns.contains(s)).collect();
    params.sort();
    params.truncate(4);
    if params.is_empty() || rows.is_empty() {
        return None;
    }
    let n = env.vals.keys().filter(|k| **k < TMP_BASE).max().copied().unwrap_or(0) as usize + 1;
    let mut base = vec![0.0; n];
    for (k, v) in &env.vals {
        if (*k as usize) < n {
            base[*k as usize] = *v;
        }
    }
    let owned: Vec<(Expr, Owner)> = rows.iter().map(|e| (e.clone(), Owner::Ent(0))).collect();
    let sys = crate::solver::Sys::build(&owned, unknowns.iter().map(|u| *u as usize).collect());
    let mut trials = 0;
    for &p in &params {
        let x0 = base[p as usize];
        let d = 1e-3 * (1.0 + x0.abs());
        for sign in [1.0, -1.0] {
            let mut v = base.clone();
            let mut solved = true;
            for k in 1..=4 {
                v[p as usize] = x0 + sign * d * k as f64 / 4.0;
                if !sys.solve(&mut v, &[], [0.0; 2]) {
                    solved = false;
                    break;
                }
            }
            if !solved {
                continue;
            }
            let get = |s: u32| v.get(s as usize).copied().unwrap_or(f64::NAN);
            let t = eval(target, &get);
            if !t.is_finite() {
                continue;
            }
            trials += 1;
            let fv = eval(f, &get);
            if !(fv.is_finite() && (fv - t).abs() <= 1e-7 * (1.0 + t.abs())) {
                return Some(false);
            }
        }
    }
    (trials > 0).then_some(true)
}

/// Whether two formulas agree at perturbations of all their parameters.
fn same_function(a: &Expr, b: &Expr, env: &Env) -> bool {
    let mut n = 0;
    for k in 1..=6 {
        let (x, y) = (env.eval_perturbed(a, k), env.eval_perturbed(b, k));
        if !x.is_finite() || !y.is_finite() {
            continue;
        }
        if (x - y).abs() > 1e-7 * (1.0 + x.abs().max(y.abs())) {
            return false;
        }
        n += 1;
    }
    n >= 3
}

/// Factors the polynomial sums in `e` (e.g. a radicand d1·d2·d3² + d1·d2·d4²
/// + d1²·d3·d4 + d2²·d3·d4 = (d1·d3 + d2·d4)(d1·d4 + d2·d3)), keeping a
/// product where it is smaller.
fn factor_sums(e: &Expr) -> Expr {
    if past_deadline() {
        return e.clone();
    }
    let r = match e.node() {
        Node::Num(_) | Node::Pi | Node::Sym(_) => return e.clone(),
        Node::Add(ts) => add(ts.iter().map(factor_sums).collect()),
        Node::Mul(ts) => mul(ts.iter().map(factor_sums).collect()),
        Node::Pow(b, x) => pow(factor_sums(b), x.clone()),
        Node::Fun(f, a) => fun(*f, factor_sums(a)),
        Node::Atan2(a, b) => atan2(factor_sums(a), factor_sums(b)),
    };
    let Node::Add(ts) = r.node() else { return r };
    if ts.len() < 3 || r.size() > 2000 {
        return r;
    }
    let mut vars: Vec<u32> = sym_set(&r).into_iter().collect();
    vars.sort();
    let mut ctx = PolyCtx::new(&vars);
    if vars.is_empty() || !ctx.collect(&r) || !ctx.atoms.is_empty() {
        return r;
    }
    let Some(p) = ctx.to_mpoly(&r) else { return r };
    let all: Vec<usize> = (0..p.n).filter(|&v| p.uses(v)).collect();
    let Some(main) = cheapest_main(&p, &all) else { return r };
    let fs = factor_main(&p, main);
    if fs.iter().filter(|f| !f.is_const()).count() < 2 || past_deadline() {
        return r;
    }
    // the factors carry p's content only up to a constant; each factor gets
    // the sign with more positive terms (d2 + d3 - d1 rather than d1 - d2 - d3)
    let prod = fs.iter().fold(MPoly::one(p.n), |acc, f| acc.mul(f));
    let Some(c) = p.div_exact(&prod).filter(|c| c.is_const()) else { return r };
    let mut c = c;
    let mut parts: Vec<Expr> = Vec::new();
    for f in &fs {
        // numeric content out of the factor: 2a + 2b = 2(a + b)
        use num_integer::Integer;
        let g = f.t.values().fold(BigInt::from(0), |g, v| g.gcd(v.numer()));
        let l = f.t.values().fold(BigInt::from(1), |l, v| l.lcm(v.denom()));
        let content = BigQ::new(g, l);
        let f = &if num_traits::Zero::is_zero(&content) { f.clone() } else {
            c = c.scale(&content);
            f.scale(&num_traits::Inv::inv(content))
        };
        let neg_terms = f.t.values().filter(|v| num_traits::Signed::is_negative(*v)).count();
        if 2 * neg_terms > f.terms() {
            c = c.scale(&BigQ::from_integer(BigInt::from(-1)));
            parts.push(ctx.to_expr(&f.scale(&BigQ::from_integer(BigInt::from(-1)))));
        } else {
            parts.push(ctx.to_expr(f));
        }
    }
    parts.push(ctx.to_expr(&c));
    // a product only a little smaller reads worse than the sum
    let f = mul(parts);
    if 10 * complexity(&f) <= 7 * complexity(&r) { f } else { r }
}

/// Pulls the factor shared by all terms out of each sum where that does not
/// make the sum larger (r − r·sin x → r·(1 − sin x)).
fn common_factors_out(e: &Expr) -> Expr {
    let r = match e.node() {
        Node::Num(_) | Node::Pi | Node::Sym(_) => return e.clone(),
        Node::Add(ts) => add(ts.iter().map(common_factors_out).collect()),
        Node::Mul(ts) => mul(ts.iter().map(common_factors_out).collect()),
        Node::Pow(b, x) => pow(common_factors_out(b), x.clone()),
        Node::Fun(f, a) => fun(*f, common_factors_out(a)),
        Node::Atan2(a, b) => atan2(common_factors_out(a), common_factors_out(b)),
    };
    match factor_common(&r) {
        // (the explicit 1 left in the sum counts as a little more to read)
        Some(f) if complexity(&f) <= complexity(&r) + 2 => f,
        _ => r,
    }
}

/// The value of `e` when it does not change as its parameters vary: `e` is
/// evaluated at the current values and at 8 perturbations of all of them
/// (up to ±2.5%); points outside its domain are skipped, and at least 4 must
/// remain.
fn constant_value(e: &Expr, env: &Env) -> Option<f64> {
    if sym_set(e).is_empty() {
        return None;
    }
    let v0 = env.eval(e);
    if !v0.is_finite() {
        return None;
    }
    let mut n = 0;
    for k in 1..=8 {
        let v = env.eval_perturbed(e, k);
        if !v.is_finite() {
            continue;
        }
        if (v - v0).abs() > 1e-9 * (1.0 + v0.abs().max(v.abs())) {
            return None;
        }
        n += 1;
    }
    (n >= 4).then_some(v0)
}

/// Replaces the parts of an exact formula that are constant (identities such
/// as a Simson line's offset, which is 0 for every triangle) by their exact
/// values, largest parts first. A constant whose exact value is not
/// recognized stays as it is.
fn fold_constants(e: &Expr, env: &Env) -> Expr {
    if past_deadline() || e.size() < 6 {
        return e.clone();
    }
    if let Some(v) = constant_value(e, env) {
        let c = nice_const(v);
        if !has_float(&c) {
            return c;
        }
    }
    match e.node() {
        Node::Add(ts) => add(ts.iter().map(|t| fold_constants(t, env)).collect()),
        Node::Mul(ts) => mul(ts.iter().map(|t| fold_constants(t, env)).collect()),
        Node::Pow(b, x) => pow(fold_constants(b, env), fold_constants(x, env)),
        Node::Fun(f, a) => fun(*f, fold_constants(a, env)),
        Node::Atan2(a, b) => atan2(fold_constants(a, env), fold_constants(b, env)),
        _ => e.clone(),
    }
}

/// Replaces |a| by ±a using the sign of `a` in the current configuration,
/// and acos(cos u) by the ±u + 2πk it equals there.
fn resolve_abs(e: &Expr, env: &Env) -> Expr {
    match e.node() {
        Node::Fun(Fun::Acos, a) if matches!(a.node(), Node::Fun(Fun::Cos | Fun::Sin, _)) => {
            let Node::Fun(g, u) = a.node() else { unreachable!() };
            let u2 = resolve_abs(u, env);
            // sin u = cos(π/2 - u)
            let u2 = if *g == Fun::Sin { sub(mul(vec![rat(1, 2), pi()]), u2) } else { u2 };
            let v = env.eval(&u2);
            let k = (v / (2.0 * PI)).round();
            let r = v - 2.0 * PI * k;
            // near a fold the ±u form would hold on one side only
            if !v.is_finite() || r.abs() < 0.1 || (r.abs() - PI).abs() < 0.1 {
                return fun(Fun::Acos, cos(u2));
            }
            let shifted = if k == 0.0 { u2 } else { sub(u2, mul(vec![int(2 * k as i64), pi()])) };
            if r > 0.0 { shifted } else { neg(shifted) }
        }
        Node::Fun(Fun::Abs, a) => {
            let a2 = resolve_abs(a, env);
            let v = env.eval(&a2);
            if v > 1e-9 {
                a2
            } else if v < -1e-9 {
                neg(a2)
            } else {
                fun(Fun::Abs, a2)
            }
        }
        Node::Num(_) | Node::Pi | Node::Sym(_) => e.clone(),
        Node::Add(ts) => add(ts.iter().map(|t| resolve_abs(t, env)).collect()),
        Node::Mul(ts) => mul(ts.iter().map(|t| resolve_abs(t, env)).collect()),
        Node::Pow(b, x) => pow(resolve_abs(b, env), resolve_abs(x, env)),
        Node::Fun(f, a) => fun(*f, resolve_abs(a, env)),
        Node::Atan2(a, b) => atan2(resolve_abs(a, env), resolve_abs(b, env)),
    }
}

// ---------------------------------------------------------------- structure

/// Maximum bipartite matching of equations to unknowns (Kuhn's algorithm).
fn matching(adj: &[Vec<usize>], nu: usize) -> Vec<Option<usize>> {
    let mut mu: Vec<Option<usize>> = vec![None; nu];
    fn try_k(e: usize, adj: &[Vec<usize>], seen: &mut [bool], mu: &mut [Option<usize>]) -> bool {
        for &u in &adj[e] {
            if seen[u] {
                continue;
            }
            seen[u] = true;
            if mu[u].is_none() || try_k(mu[u].unwrap(), adj, seen, mu) {
                mu[u] = Some(e);
                return true;
            }
        }
        false
    }
    for e in 0..adj.len() {
        let mut seen = vec![false; nu];
        try_k(e, adj, &mut seen, &mut mu);
    }
    mu
}

/// Tarjan SCC; returns components in dependency-first order.
fn tarjan(n: usize, edges: &[Vec<usize>]) -> Vec<Vec<usize>> {
    struct T<'a> {
        edges: &'a [Vec<usize>],
        index: Vec<Option<usize>>,
        low: Vec<usize>,
        on: Vec<bool>,
        stack: Vec<usize>,
        next: usize,
        out: Vec<Vec<usize>>,
    }
    fn strong(t: &mut T, v: usize) {
        t.index[v] = Some(t.next);
        t.low[v] = t.next;
        t.next += 1;
        t.stack.push(v);
        t.on[v] = true;
        for i in 0..t.edges[v].len() {
            let w = t.edges[v][i];
            if t.index[w].is_none() {
                strong(t, w);
                t.low[v] = t.low[v].min(t.low[w]);
            } else if t.on[w] {
                t.low[v] = t.low[v].min(t.index[w].unwrap());
            }
        }
        if t.low[v] == t.index[v].unwrap() {
            let mut c = Vec::new();
            loop {
                let w = t.stack.pop().unwrap();
                t.on[w] = false;
                c.push(w);
                if w == v {
                    break;
                }
            }
            t.out.push(c);
        }
    }
    let mut t = T { edges, index: vec![None; n], low: vec![0; n], on: vec![false; n], stack: vec![], next: 0, out: vec![] };
    for v in 0..n {
        if t.index[v].is_none() {
            strong(&mut t, v);
        }
    }
    t.out
}

/// Point-like parameter pairs and angle parameters (for rigid transforms).
fn rigid_parts(doc: &Doc) -> (Vec<(usize, usize)>, Vec<usize>) {
    let mut pts = Vec::new();
    let mut angs = Vec::new();
    for e in doc.ents.iter().flatten() {
        match &e.kind {
            Ent::Point { x, y } => pts.push((*x, *y)),
            Ent::Conic { t, .. } => angs.push(*t),
            _ => {}
        }
    }
    for c in doc.cons.iter().flatten() {
        if let Con::Tangent { aux: Some((x, y)), .. } = c {
            pts.push((*x, *y));
        }
    }
    (pts, angs)
}

fn transform(vals: &mut HashMap<u32, f64>, pts: &[(usize, usize)], angs: &[usize], o: [f64; 2], rot: f64, only: &HashSet<u32>) {
    transform_about(vals, pts, angs, o, rot, only, [0.0, 0.0]);
}

/// Rotates points by `rot` about `o`, then places `o` at `shift`.
fn transform_about(vals: &mut HashMap<u32, f64>, pts: &[(usize, usize)], angs: &[usize], o: [f64; 2], rot: f64, only: &HashSet<u32>, shift: [f64; 2]) {
    let (c, s) = (rot.cos(), rot.sin());
    for &(x, y) in pts {
        if !only.contains(&(x as u32)) {
            continue;
        }
        let (Some(&px), Some(&py)) = (vals.get(&(x as u32)), vals.get(&(y as u32))) else { continue };
        let (dx, dy) = (px - o[0], py - o[1]);
        vals.insert(x as u32, shift[0] + c * dx - s * dy);
        vals.insert(y as u32, shift[1] + s * dx + c * dy);
    }
    for &a in angs {
        if only.contains(&(a as u32)) {
            if let Some(v) = vals.get_mut(&(a as u32)) {
                *v += rot;
            }
        }
    }
}

/// Elimination order for the rank analysis: auxiliary parameters are
/// determined first and user-facing coordinates are left free last.
fn priorities(doc: &Doc) -> HashMap<u32, u8> {
    let mut m = HashMap::new();
    for c in doc.cons.iter().flatten() {
        if let Con::Tangent { aux: Some((x, y)), .. } = c {
            m.insert(*x as u32, 0);
            m.insert(*y as u32, 0);
        }
    }
    for e in doc.ents.iter().flatten() {
        match &e.kind {
            Ent::Circle { r, .. } => {
                m.insert(*r as u32, 1);
            }
            Ent::Conic { a, b, t, .. } => {
                for p in [a, b, t] {
                    m.insert(*p as u32, 1);
                }
            }
            Ent::Point { x, y } => {
                m.insert(*y as u32, 3);
                m.insert(*x as u32, 4);
            }
            _ => {}
        }
    }
    m
}

/// Gaussian elimination with partial pivoting over columns in order.
/// Returns pivot column and pivot row indices (local).
fn rank_select(eqs: &[Expr], rows: &[usize], cols: &[u32], env: &Env) -> (Vec<usize>, Vec<usize>) {
    let (m, n) = (rows.len(), cols.len());
    let mut a = vec![0.0f64; m * n];
    for (i, &r) in rows.iter().enumerate() {
        let syms = sym_set(&eqs[r]);
        for (j, &c) in cols.iter().enumerate() {
            if syms.contains(&c) {
                let v = env.eval(&diff(&eqs[r], c));
                a[i * n + j] = if v.is_finite() { v } else { 0.0 };
            }
        }
        let s: f64 = a[i * n..(i + 1) * n].iter().map(|x| x * x).sum::<f64>().sqrt();
        if s > 0.0 {
            a[i * n..(i + 1) * n].iter_mut().for_each(|x| *x /= s);
        }
    }
    let mut order: Vec<usize> = (0..m).collect();
    let mut piv_cols = Vec::new();
    let mut piv_rows = Vec::new();
    let mut row = 0;
    for col in 0..n {
        if row >= m {
            break;
        }
        let (mut best, mut bv) = (row, 0.0);
        for i in row..m {
            let v = a[i * n + col].abs();
            if v > bv {
                bv = v;
                best = i;
            }
        }
        if bv < 1e-9 {
            continue;
        }
        for j in 0..n {
            a.swap(row * n + j, best * n + j);
        }
        order.swap(row, best);
        let p = a[row * n + col];
        for i in row + 1..m {
            let f = a[i * n + col] / p;
            if f != 0.0 {
                for j in col..n {
                    a[i * n + j] -= f * a[row * n + j];
                }
            }
        }
        piv_cols.push(col);
        piv_rows.push(order[row]);
        row += 1;
    }
    piv_rows.sort();
    (piv_cols, piv_rows)
}

/// When `target` is locally independent of the free unknown `f`, finds a
/// simple nearby value for `f` and the re-solved configuration there.
#[allow(clippy::too_many_arguments)]
fn fix_free(
    rows: &[Expr],
    unknowns: &[u32],
    f: u32,
    target: &Expr,
    env: &Env,
    actual: f64,
    rank: usize,
    pts: &[(usize, usize)],
    lines: &[((usize, usize), (usize, usize))],
) -> Option<(Rat, HashMap<u32, f64>)> {
    if unknowns.is_empty() && rows.is_empty() {
        return None;
    }
    let n = env.vals.keys().filter(|k| **k < TMP_BASE).max().copied().unwrap_or(0) as usize + 1;
    let mut base = vec![0.0; n];
    for (k, v) in &env.vals {
        if (*k as usize) < n {
            base[*k as usize] = *v;
        }
    }
    let owned: Vec<(Expr, Owner)> = rows.iter().map(|e| (e.clone(), Owner::Ent(0))).collect();
    let sys = crate::solver::Sys::build(&owned, unknowns.iter().map(|u| *u as usize).collect());
    // continuation from the current value keeps the configuration on its branch
    let solve_at = |x: f64| -> Option<Vec<f64>> {
        let mut v = base.clone();
        let x0 = base[f as usize];
        let steps = 8;
        for k in 1..=steps {
            v[f as usize] = x0 + (x - x0) * k as f64 / steps as f64;
            if !sys.solve(&mut v, &[], [0.0; 2]) {
                return None;
            }
        }
        Some(v)
    };
    let tval = |v: &[f64]| eval(target, &|s| v.get(s as usize).copied().unwrap_or(f64::NAN));
    let same = |t: f64| (t - actual).abs() <= 1e-8 * (1.0 + actual.abs());
    let v0 = base[f as usize];
    let base_pivot = min_pivot(rows, unknowns, env, env);
    let d = 1e-3 * (1.0 + v0.abs());
    let p = solve_at(v0 + d)?;
    if !same(tval(&p)) {
        return None;
    }
    // 0 gives the simplest formulas; degenerate choices are rejected below
    let mut cands: Vec<f64> = vec![0.0, v0.round(), (v0 * 2.0).round() / 2.0, (v0 * 10.0).round() / 10.0, (v0 * 100.0).round() / 100.0];
    let mut seen: Vec<f64> = Vec::new();
    cands.retain(|c| {
        if seen.contains(c) {
            false
        } else {
            seen.push(*c);
            true
        }
    });
    for c in cands {
        let Some(v) = solve_at(c) else { continue };
        if !same(tval(&v)) || creates_coincidence(&base, &v, pts) || reverses_line(&base, &v, lines) {
            continue;
        }
        let mut vals = env.vals.clone();
        for (i, x) in v.iter().enumerate() {
            if vals.contains_key(&(i as u32)) {
                vals.insert(i as u32, *x);
            }
        }
        let e2 = Env { vals };
        let idx: Vec<usize> = (0..rows.len()).collect();
        let fake: Vec<Expr> = rows.to_vec();
        let (pc, _) = rank_select(&fake, &idx, unknowns, &e2);
        if pc.len() < rank {
            continue;
        }
        // reject values where the system becomes nearly singular (double roots)
        let mp = min_pivot(rows, unknowns, &e2, env) / base_pivot.max(1e-300);
        if std::env::var("GXPAD_DEBUG").is_ok() {
            eprintln!("  fix_free s{f} = {c}: relative min pivot {mp:.3e}");
        }
        if mp < 1e-2 {
            continue;
        }
        let r = Rat::from_f64_nice(c)?;
        return Some((r, e2.vals));
    }
    None
}

/// Smallest elimination pivot of the Jacobian at `at`, with rows scaled by
/// their norms at `base` (so conditioning loss relative to `base` shows).
fn min_pivot(rows: &[Expr], cols: &[u32], at: &Env, base: &Env) -> f64 {
    let (m, n) = (rows.len(), cols.len());
    if m == 0 || n == 0 {
        return 1.0;
    }
    let jac = |env: &Env| -> Vec<f64> {
        let mut a = vec![0.0; m * n];
        for (i, r) in rows.iter().enumerate() {
            let syms = sym_set(r);
            for (j, &c) in cols.iter().enumerate() {
                if syms.contains(&c) {
                    let v = env.eval(&diff(r, c));
                    a[i * n + j] = if v.is_finite() { v } else { 0.0 };
                }
            }
        }
        a
    };
    let a0 = jac(base);
    let mut a = jac(at);
    for i in 0..m {
        let s: f64 = a0[i * n..(i + 1) * n].iter().map(|x| x * x).sum::<f64>().sqrt();
        if s > 0.0 {
            a[i * n..(i + 1) * n].iter_mut().for_each(|x| *x /= s);
        }
    }
    let mut minp = f64::INFINITY;
    let mut row = 0;
    for col in 0..n {
        if row >= m {
            break;
        }
        let (mut best, mut bv) = (row, 0.0);
        for i in row..m {
            if a[i * n + col].abs() > bv {
                bv = a[i * n + col].abs();
                best = i;
            }
        }
        if bv < 1e-12 {
            continue;
        }
        for j in 0..n {
            a.swap(row * n + j, best * n + j);
        }
        minp = minp.min(bv);
        let p = a[row * n + col];
        for i in row + 1..m {
            let f = a[i * n + col] / p;
            for j in col..n {
                a[i * n + j] -= f * a[row * n + j];
            }
        }
        row += 1;
    }
    minp
}

/// True when a line's direction reverses between `before` and `after`
/// (directed angles would change into their supplements).
fn reverses_line(before: &[f64], after: &[f64], lines: &[((usize, usize), (usize, usize))]) -> bool {
    let dir = |v: &[f64], (a, b): ((usize, usize), (usize, usize))| -> Option<[f64; 2]> {
        Some([*v.get(b.0)? - *v.get(a.0)?, *v.get(b.1)? - *v.get(a.1)?])
    };
    lines.iter().any(|&l| match (dir(before, l), dir(after, l)) {
        (Some(d0), Some(d1)) => d0[0] * d1[0] + d0[1] * d1[1] <= 0.0,
        _ => false,
    })
}

/// True when two points that are apart in `before` coincide in `after`
/// (a parked value must not create a degenerate configuration).
fn creates_coincidence(before: &[f64], after: &[f64], pts: &[(usize, usize)]) -> bool {
    let get = |v: &[f64], p: (usize, usize)| -> Option<[f64; 2]> { Some([*v.get(p.0)?, *v.get(p.1)?]) };
    let mut scale: f64 = 1e-9;
    for &p in pts {
        if let Some(q) = get(before, p) {
            scale = scale.max(q[0].abs()).max(q[1].abs());
        }
    }
    let tol = 1e-7 * scale;
    for i in 0..pts.len() {
        for j in i + 1..pts.len() {
            let (Some(a0), Some(b0), Some(a1), Some(b1)) = (get(before, pts[i]), get(before, pts[j]), get(after, pts[i]), get(after, pts[j])) else {
                continue;
            };
            let d0 = (a0[0] - b0[0]).hypot(a0[1] - b0[1]);
            let d1 = (a1[0] - b1[0]).hypot(a1[1] - b1[1]);
            if d0 > tol && d1 <= tol {
                if std::env::var("GXPAD_DEBUG").is_ok() {
                    eprintln!("  coincidence {:?} {:?}: {a0:?}/{b0:?} -> {a1:?}/{b1:?}", pts[i], pts[j]);
                }
                return true;
            }
        }
    }
    false
}

/// Derives a closed-form expression of `target` in terms of the driving
/// symbols (free variables and driving dimensions).
pub fn derive(doc: &Doc, target: &Expr) -> Result<Derived, String> {
    derive_harder(doc, target, 0)
}

/// Like [`derive`] with `extra_ms` more search time. With extra time the
/// polynomial elimination also takes blocks of any size.
pub fn derive_harder(doc: &Doc, target: &Expr, extra_ms: u64) -> Result<Derived, String> {
    EXTRA_MS.with(|c| c.set(extra_ms));
    let r = derive_strategies(doc, target, extra_ms);
    EXTRA_MS.with(|c| c.set(0));
    r
}

fn derive_strategies(doc: &Doc, target: &Expr, extra_ms: u64) -> Result<Derived, String> {
    // Grounding choices lead to very different block structures; try the
    // base-geometry anchor first and the target anchor second, keeping the
    // simplest exact result. Extra time goes mostly to the first.
    let extra0 = extra_ms * 7 / 10;
    let (first, grounded) = derive_impl(doc, target, 0, 1500 + extra0)?;
    if !grounded || valid(&first) && first.exact && complexity(&first.expr) < 300 {
        return Ok(checked(first));
    }
    let second = derive_impl(doc, target, 1, 600 + (extra_ms - extra0)).ok().map(|r| r.0);
    let score = |d: &Derived| (!d.exact, complexity(&d.expr));
    let best = match second {
        Some(b) if valid(&b) && (!valid(&first) || score(&b) < score(&first)) => b,
        _ => first,
    };
    Ok(checked(best))
}

fn valid(d: &Derived) -> bool {
    d.value.is_finite() && (d.value - d.actual).abs() <= 1e-7 * (1.0 + d.actual.abs())
}

/// A formula that does not reproduce the measurement is a derivation failure:
/// report the measured number instead of a wrong expression.
fn checked(d: Derived) -> Derived {
    if valid(&d) {
        d
    } else {
        Derived { expr: float_raw(d.actual), exact: false, value: d.actual, actual: d.actual }
    }
}

thread_local! {
    static TIME_SCALE: std::cell::Cell<f64> = const { std::cell::Cell::new(1.0) };
    /// Extra search time of the derivation running on this thread.
    static EXTRA_MS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Most unknowns a block may have for polynomial elimination in a normal
/// search: larger blocks rarely succeed in the default time and would take
/// it from the other methods. A search given extra time has no limit.
const POLY_VARS_NORMAL: usize = 14;

/// Formulas above this complexity (or not exact) are also searched for by
/// fitting a relation to samples.
const FIT_ABOVE: usize = 40;

/// A fitted formula at most this complex ends the search.
const FIT_SETTLES: usize = 300;

/// Scales the derivation time budgets on this thread (tests running under
/// heavy parallel load use more than 1).
pub fn set_time_scale(f: f64) {
    TIME_SCALE.with(|c| c.set(f));
}

fn scaled(ms: u64) -> std::time::Duration {
    std::time::Duration::from_millis((ms as f64 * TIME_SCALE.with(|c| c.get())) as u64)
}

fn derive_impl(doc: &Doc, target: &Expr, strategy: u8, budget_ms: u64) -> Result<(Derived, bool), String> {
    let deadline = work_now() + scaled(budget_ms);
    set_deadline(Some(deadline));
    // real-time safeguard for work that the work clock undercounts
    let real0 = web_time::Instant::now();
    set_hard_deadline(Some(real0 + scaled(budget_ms) * 4 + std::time::Duration::from_secs(4)));
    let ticks0 = work_ticks();
    let r = derive_inner(doc, target, strategy, deadline);
    set_deadline(None);
    set_hard_deadline(None);
    if std::env::var("GXPAD_DEBUG").is_ok() {
        eprintln!("work: {} ticks in {:?}", work_ticks() - ticks0, real0.elapsed());
    }
    r
}

fn derive_inner(doc: &Doc, target: &Expr, strategy: u8, deadline: web_time::Instant) -> Result<(Derived, bool), String> {

    let (pos, ang) = doc.assumptions();
    set_assumptions(pos, ang);
    let (eqns, errs) = doc.equations();
    if let Some((_, m)) = errs.first() {
        return Err(m.clone());
    }
    let unknown_set: HashSet<u32> = doc.unknowns().into_iter().map(|p| p as u32).collect();
    let named: HashSet<u32> = doc
        .vars
        .iter()
        .flatten()
        .map(|v| v.param as u32)
        .chain(doc.dims.iter().flatten().map(|d| d.param as u32))
        .collect();

    let mut env = Env { vals: HashMap::new() };
    for (i, p) in doc.params.iter().enumerate() {
        env.vals.insert(i as u32, p.v);
    }
    let actual = env.eval(target);

    // When moving the whole construction, pinned points included, changes
    // neither the constraints nor the target, measure coordinates from a
    // pinned point involved with the target: its coordinates become exact
    // zeros instead of arbitrary decimals.
    {
        let (all_pts, _) = rigid_parts(doc);
        let unknown_any = |e: &Expr| sym_set(e).iter().any(|s| unknown_set.contains(s));
        // equations connected to the target through unknowns
        let mut reach: HashSet<u32> = sym_set(target).into_iter().filter(|s| unknown_set.contains(s)).collect();
        let mut used: HashSet<usize> = HashSet::new();
        loop {
            let mut grew = false;
            for (i, (e, _)) in eqns.iter().enumerate() {
                if used.contains(&i) || !unknown_any(e) {
                    continue;
                }
                let ss = sym_set(e);
                if ss.iter().any(|x| reach.contains(x)) {
                    used.insert(i);
                    for x in ss {
                        if unknown_set.contains(&x) {
                            reach.insert(x);
                        }
                    }
                    grew = true;
                }
            }
            if !grew {
                break;
            }
        }
        let mut involved: HashSet<u32> = sym_set(target);
        for &i in &used {
            involved.extend(sym_set(&eqns[i].0));
        }
        let pivot = doc.ents.iter().flatten().find_map(|e| match e.kind {
            Ent::Point { x, y } if !e.hidden && doc.params[x].locked && doc.params[y].locked && (involved.contains(&(x as u32)) || involved.contains(&(y as u32))) => {
                Some([doc.params[x].v, doc.params[y].v])
            }
            _ => None,
        });
        if let Some(pv) = pivot.filter(|pv| pv[0] != 0.0 || pv[1] != 0.0) {
            let shifted = |d: [f64; 2]| {
                let mut v = env.vals.clone();
                for &(x, y) in &all_pts {
                    if let (Some(&px), Some(&py)) = (env.vals.get(&(x as u32)), env.vals.get(&(y as u32))) {
                        v.insert(x as u32, px + d[0]);
                        v.insert(y as u32, py + d[1]);
                    }
                }
                Env { vals: v }
            };
            let e2 = shifted([0.731, -0.417]);
            let same_target = (e2.eval(target) - actual).abs() <= 1e-8 * (1.0 + actual.abs());
            let same_eqs = eqns.iter().all(|(e, _)| {
                let (r0, r1) = (env.eval(e), e2.eval(e));
                !r0.is_finite() || (r1 - r0).abs() <= 1e-7 * (1.0 + r0.abs())
            });
            if same_target && same_eqs {
                env = shifted([-pv[0], -pv[1]]);
            }
        }
    }

    // constants: locked geometry parameters become numbers
    let mut consts: HashMap<u32, Expr> = HashMap::new();
    for (i, p) in doc.params.iter().enumerate() {
        let s = i as u32;
        if p.locked && !named.contains(&s) {
            consts.insert(s, nice_const(env.get(s)));
        }
    }
    // a driving dimension set to exactly 0 is structural (e.g. a point pinned to
    // an axis), not a parameter of the formula
    for d in doc.dims.iter().flatten() {
        if d.driving && d.expr.trim().is_empty() && doc.params[d.param].v == 0.0 {
            consts.insert(d.param as u32, zero());
        }
    }
    // numeric variables and dimensions are frozen: their value enters the
    // formula as a plain number, so their name never appears
    for v in doc.vars.iter().flatten().filter(|v| v.numeric) {
        consts.insert(v.param as u32, nice_const(env.get(v.param as u32)));
    }
    for d in doc.dims.iter().flatten().filter(|d| d.numeric) {
        consts.insert(d.param as u32, nice_const(env.get(d.param as u32)));
    }
    let eqs: Vec<Expr> = eqns.iter().map(|(e, _)| subst_map(e, &consts)).collect();
    let target_c = subst_map(target, &consts);

    // connected component of the target
    let eq_unk: Vec<Vec<u32>> = eqs
        .iter()
        .map(|e| sym_set(e).into_iter().filter(|s| unknown_set.contains(s)).collect())
        .collect();
    let mut comp_u: HashSet<u32> = sym_set(&target_c).into_iter().filter(|s| unknown_set.contains(s)).collect();
    let mut comp_e: HashSet<usize> = HashSet::new();
    loop {
        let mut changed = false;
        for (i, us) in eq_unk.iter().enumerate() {
            if comp_e.contains(&i) {
                continue;
            }
            if us.iter().any(|u| comp_u.contains(u)) {
                comp_e.insert(i);
                for u in us {
                    comp_u.insert(*u);
                }
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // rigid-motion grounding when the component and target are invariant
    let mut ground: HashMap<u32, Expr> = HashMap::new();
    let (pts, angs) = rigid_parts(doc);
    let comp_pts: Vec<(usize, usize)> = pts
        .iter()
        .copied()
        .filter(|(x, y)| comp_u.contains(&(*x as u32)) && comp_u.contains(&(*y as u32)))
        .collect();
    if !comp_pts.is_empty() {
        let invariant = |o: [f64; 2], rot: f64| -> bool {
            let mut v2 = env.vals.clone();
            transform(&mut v2, &pts, &angs, o, rot, &comp_u);
            let e2 = Env { vals: v2 };
            let scale = 1.0 + actual.abs();
            if !((e2.eval(&target_c) - actual).abs() < 1e-7 * scale) {
                return false;
            }
            comp_e.iter().all(|&i| {
                let r = e2.eval(&eqs[i]);
                r.abs() < 1e-6 * (1.0 + env.eval(&eqs[i]).abs()) + 1e-7
            })
        };
        // ground at base geometry: a point joined to another by a dimensioned
        // length, created early (constructed points come later). Grounding a
        // constructed point instead couples everything into one large block.
        let ent_of: std::collections::HashMap<usize, usize> = doc
            .ents
            .iter()
            .enumerate()
            .filter_map(|(i, e)| match e {
                Some(Entity { kind: Ent::Point { x, .. }, hidden: false, .. }) => Some((*x, i)),
                _ => None,
            })
            .collect();
        let mut dim_pairs: Vec<(usize, usize)> = Vec::new();
        for dm in doc.dims.iter().flatten() {
            if !dm.driving {
                continue;
            }
            match dm.kind {
                DimKind::Length(l) => dim_pairs.push(doc.line_pts(l)),
                DimKind::Distance(a, b) => dim_pairs.push((a, b)),
                _ => {}
            }
        }
        let tsyms = sym_set(&target_c);
        let p0 = if strategy == 1 {
            comp_pts.iter().copied().find(|(x, _)| tsyms.contains(&(*x as u32))).unwrap_or(comp_pts[0])
        } else {
            comp_pts
                .iter()
                .copied()
                .filter(|(x, _)| ent_of.contains_key(x))
                .min_by_key(|(x, _)| {
                    let e = ent_of[x];
                    (!dim_pairs.iter().any(|(a, b)| *a == e || *b == e), e)
                })
                .unwrap_or(comp_pts[0])
        };
        let o = [env.get(p0.0 as u32), env.get(p0.1 as u32)];
        if invariant([o[0] + 0.731, o[1] - 0.417], 0.0) {
            // find the partner point for rotation: prefer a line from p0
            let p0e = doc.ents.iter().position(|e| matches!(e, Some(Entity { kind: Ent::Point { x, .. }, .. }) if *x == p0.0));
            let mut p1: Option<(usize, usize)> = None;
            if let Some(p0e) = p0e {
                for (a, b) in &dim_pairs {
                    let other = if *a == p0e { Some(*b) } else if *b == p0e { Some(*a) } else { None };
                    if let Some(q) = other.filter(|q| doc.is_point(*q)) {
                        let qp = doc.pparams(q);
                        if comp_u.contains(&(qp.0 as u32)) && comp_u.contains(&(qp.1 as u32)) {
                            p1 = Some(qp);
                            break;
                        }
                    }
                }
            }
            if let (Some(p0e), None) = (p0e, p1) {
                for (i, e) in doc.ents.iter().enumerate() {
                    if let Some(Entity { kind: Ent::Line { a, b, .. }, .. }) = e {
                        let other = if *a == p0e { Some(*b) } else if *b == p0e { Some(*a) } else { None };
                        if let Some(q) = other {
                            let qp = doc.pparams(q);
                            if comp_u.contains(&(qp.0 as u32)) && comp_u.contains(&(qp.1 as u32)) {
                                let _ = i;
                                p1 = Some(qp);
                                break;
                            }
                        }
                    }
                }
            }
            if p1.is_none() {
                p1 = comp_pts.iter().copied().find(|q| {
                    *q != p0 && ((env.get(q.0 as u32) - o[0]).hypot(env.get(q.1 as u32) - o[1])) > 1e-6
                });
            }
            let rot_ok = p1.is_some() && invariant(o, 0.6);
            let mut rot = 0.0;
            if rot_ok {
                let q = p1.unwrap();
                rot = -(env.get(q.1 as u32) - o[1]).atan2(env.get(q.0 as u32) - o[0]);
            }
            if std::env::var("GXPAD_DEBUG").is_ok() {
                eprintln!("ground p0={p0:?} p1={p1:?} rot={rot_ok}");
            }
            transform(&mut env.vals, &pts, &angs, o, rot, &comp_u);
            ground.insert(p0.0 as u32, zero());
            ground.insert(p0.1 as u32, zero());
            if rot_ok {
                ground.insert(p1.unwrap().1 as u32, zero());
            }
        } else {
            // rotation about a fixed pivot (e.g. a mechanism hinged at the origin)
            let rot_invariant = |o: [f64; 2]| -> bool {
                let mut v2 = env.vals.clone();
                transform_about(&mut v2, &pts, &angs, o, 0.6, &comp_u, o);
                let e2 = Env { vals: v2 };
                if !((e2.eval(&target_c) - actual).abs() < 1e-7 * (1.0 + actual.abs())) {
                    return false;
                }
                comp_e.iter().all(|&i| {
                    let r = e2.eval(&eqs[i]);
                    r.abs() < 1e-6 * (1.0 + env.eval(&eqs[i]).abs()) + 1e-7
                })
            };
            let pivots: Vec<(usize, [f64; 2])> = doc
                .ents
                .iter()
                .enumerate()
                .filter_map(|(i, e)| match e {
                    Some(Entity { kind: Ent::Point { x, y }, hidden: false, .. }) if doc.params[*x].locked && doc.params[*y].locked => {
                        Some((i, [env.get(*x as u32), env.get(*y as u32)]))
                    }
                    _ => None,
                })
                .collect();
            if let Some(&(pe, po)) = pivots.iter().find(|(_, po)| rot_invariant(*po)) {
                // the partner: a point joined to the pivot by a dimensioned length, else the first one away from it
                let mut p1: Option<(usize, usize)> = None;
                for (a, b) in &dim_pairs {
                    let other = if *a == pe { Some(*b) } else if *b == pe { Some(*a) } else { None };
                    if let Some(q) = other.filter(|q| doc.is_point(*q)) {
                        let qp = doc.pparams(q);
                        if comp_u.contains(&(qp.0 as u32)) && comp_u.contains(&(qp.1 as u32)) {
                            p1 = Some(qp);
                            break;
                        }
                    }
                }
                if p1.is_none() {
                    p1 = comp_pts.iter().copied().find(|q| ((env.get(q.0 as u32) - po[0]).hypot(env.get(q.1 as u32) - po[1])) > 1e-6);
                }
                if let Some(q) = p1 {
                    let rot = -(env.get(q.1 as u32) - po[1]).atan2(env.get(q.0 as u32) - po[0]);
                    transform_about(&mut env.vals, &pts, &angs, po, rot, &comp_u, po);
                    ground.insert(q.1 as u32, nice_const(po[1]));
                }
            }
        }
    }
    let eqs: Vec<Expr> = eqs.iter().map(|e| subst_map(e, &ground)).collect();
    let target_c = subst_map(&target_c, &ground);
    for k in ground.keys() {
        comp_u.remove(k);
    }

    // structural decomposition of the component
    let all_unk: Vec<u32> = {
        let mut v: Vec<u32> = comp_u.iter().copied().collect();
        let pr = priorities(doc);
        v.sort_by_key(|u| (pr.get(u).copied().unwrap_or(2), *u));
        v
    };
    let all_ce: Vec<usize> = {
        let mut v: Vec<usize> = comp_e.iter().copied().filter(|&i| contains_any(&eqs[i], &all_unk)).collect();
        v.sort();
        v
    };
    // numeric rank analysis picks which unknowns stay free and which equations are independent
    let (piv_cols, piv_rows) = rank_select(&eqs, &all_ce, &all_unk, &env);
    let mut free: Vec<u32> = all_unk.iter().enumerate().filter(|(j, _)| !piv_cols.contains(j)).map(|(_, u)| *u).collect();
    let unk: Vec<u32> = piv_cols.iter().map(|&j| all_unk[j]).collect();
    // free unknowns the target does not depend on are moved to simple values
    let rows_e: Vec<Expr> = piv_rows.iter().map(|&r| eqs[all_ce[r]].clone()).collect();
    let line_pts: Vec<((usize, usize), (usize, usize))> = doc
        .ents
        .iter()
        .flatten()
        .filter_map(|e| match e.kind {
            Ent::Line { a, b, .. } => Some((doc.pparams(a), doc.pparams(b))),
            _ => None,
        })
        .collect();
    // points that can take part in a degenerate coincidence (not the fixed axis helpers)
    let check_pts: Vec<(usize, usize)> = pts
        .iter()
        .copied()
        .filter(|(x, y)| {
            !doc.ents.iter().flatten().any(|e| matches!(e, Entity { kind: Ent::Point { x: ex, .. }, hidden: true, .. } if ex == x))
                || !(doc.params[*x].locked && doc.params[*y].locked)
        })
        .collect();
    let mut fixed: HashMap<u32, Expr> = HashMap::new();
    for &f in &free {
        if let Some((nv, vals)) = fix_free(&rows_e, &unk, f, &target_c, &env, actual, piv_cols.len(), &check_pts, &line_pts) {
            env.vals = vals;
            if std::env::var("GXPAD_DEBUG").is_ok() {
                eprintln!("fix free s{f} = {}", nv.to_f64());
            }
            fixed.insert(f, num(Num::R(nv)));
        }
    }
    free.retain(|f| !fixed.contains_key(f));
    let eqs: Vec<Expr> = if fixed.is_empty() { eqs } else { eqs.iter().map(|e| subst_map(e, &fixed)).collect() };
    let target_c = subst_map(&target_c, &fixed);
    let uidx: HashMap<u32, usize> = unk.iter().enumerate().map(|(i, u)| (*u, i)).collect();
    let ce: Vec<usize> = piv_rows.iter().map(|&r| all_ce[r]).collect();
    let adj: Vec<Vec<usize>> = ce
        .iter()
        .map(|&i| {
            let mut a: Vec<usize> = sym_set(&eqs[i]).into_iter().filter_map(|s| uidx.get(&s).copied()).collect();
            a.sort();
            a
        })
        .collect();
    let mu = matching(&adj, unk.len());
    // eq (local) -> unknown (local)
    let mut em: Vec<Option<usize>> = vec![None; ce.len()];
    for (u, e) in mu.iter().enumerate() {
        if let Some(e) = e {
            em[*e] = Some(u);
        }
    }
    let matched: Vec<usize> = (0..ce.len()).filter(|&e| em[e].is_some()).collect();
    let node_of_u: HashMap<usize, usize> = matched.iter().enumerate().map(|(n, &e)| (em[e].unwrap(), n)).collect();
    let edges: Vec<Vec<usize>> = matched
        .iter()
        .map(|&e| adj[e].iter().filter(|u| Some(**u) != em[e]).filter_map(|u| node_of_u.get(u).copied()).collect())
        .collect();
    let blocks = tarjan(matched.len(), &edges);
    free.extend((0..unk.len()).filter(|u| mu[*u].is_none()).map(|u| unk[u]));

    // needed blocks
    let mut need: HashSet<usize> = HashSet::new();
    let mut stack: Vec<usize> = sym_set(&target_c).into_iter().filter_map(|s| uidx.get(&s)).filter_map(|u| node_of_u.get(u).copied()).collect();
    while let Some(n) = stack.pop() {
        if need.insert(n) {
            stack.extend(edges[n].iter().copied());
        }
    }

    if std::env::var("GXPAD_DEBUG").is_ok() {
        let names = doc.param_names();
        let mut ks: Vec<_> = names.iter().collect();
        ks.sort();
        eprintln!("names: {:?}", ks);
        eprintln!("target: {}", crate::expr::bx::plain(&crate::expr::bx::to_bx(&target_c, &|s| format!("s{s}"))));
    }
    let blocks_start = work_now();
    let mut solver = BlockSolver { env: &env, tmp: TMP_BASE, budget: 0, deadline, keep: None };
    // the equations of the needed blocks, for solving for the target directly
    let mut target_vars: Vec<u32> = Vec::new();
    let mut target_eqs: Vec<Expr> = Vec::new();
    for b in &blocks {
        if !b.iter().any(|n| need.contains(n)) {
            continue;
        }
        target_vars.extend(b.iter().map(|&n| unk[em[matched[n]].unwrap()]));
        target_eqs.extend(b.iter().map(|&n| eqs[ce[matched[n]]].clone()));
    }
    // Solves the needed blocks in order until `until`; blocks not solved by
    // then get numeric values.
    let run_blocks = |solver: &mut BlockSolver, until: web_time::Instant| -> (HashMap<u32, Expr>, bool) {
        solver.deadline = until;
        set_deadline(Some(until));
        let mut sol: HashMap<u32, Expr> = HashMap::new();
        let mut exact = true;
        for b in &blocks {
            if !b.iter().any(|n| need.contains(n)) {
                continue;
            }
            let vars: Vec<u32> = b.iter().map(|&n| unk[em[matched[n]].unwrap()]).collect();
            let beqs: Vec<Expr> = b.iter().map(|&n| subst_map(&eqs[ce[matched[n]]], &sol)).collect();
            let dbg = std::env::var("GXPAD_DEBUG").is_ok();
            if dbg {
                eprintln!("block vars={:?}", vars);
                for e in &beqs {
                    eprintln!("   eq: {}", crate::expr::bx::plain(&crate::expr::bx::to_bx(e, &|s| format!("s{s}"))));
                }
            }
            let tb = web_time::Instant::now();
            let solved = if past_deadline() { None } else { solver.solve_block(beqs, vars.clone()) };
            if dbg {
                eprintln!("   block took {:?}", tb.elapsed());
                for (v, e) in solved.iter().flatten() {
                    eprintln!("   -> s{v} = {}", crate::expr::bx::plain(&crate::expr::bx::to_bx(e, &|s| format!("s{s}"))));
                }
            }
            match solved {
                Some(r) => {
                    for (v, e) in r {
                        sol.insert(v, e);
                    }
                }
                None => {
                    if dbg {
                        eprintln!("   FAILED -> numeric");
                    }
                    exact = false;
                    for v in vars {
                        sol.insert(v, float_raw(env.get(v)));
                    }
                }
            }
        }
        (sol, exact)
    };
    let rows: Vec<Expr> = ce.iter().map(|&i| eqs[i].clone()).collect();
    // A formula fitted to samples of the construction (see `fit`).
    let run_fit = |solver: &mut BlockSolver| -> Option<Expr> {
        if unk.is_empty() {
            return None;
        }
        let tf = web_time::Instant::now();
        let s = work_now() + scaled(700 + EXTRA_MS.with(|c| c.get()) / 4);
        set_deadline(Some(s));
        let lengths: HashSet<u32> = doc.dims.iter().flatten().filter(|d| !d.kind.is_angle()).map(|d| d.param as u32).collect();
        let t = solver.new_tmp();
        let max_cols = if EXTRA_MS.with(|c| c.get()) > 0 { 400 } else { 170 };
        if std::env::var("GXPAD_DEBUG").is_ok() {
            for e in &rows {
                eprintln!("fit row: {}", crate::expr::bx::plain(&crate::expr::bx::to_bx(e, &|s| format!("s{s}"))));
            }
        }
        let pb = crate::fit::Problem { rows: &rows, unknowns: &unk, target: &target_c, vals: &env.vals, lengths: &lengths, max_cols, t };
        let f = crate::fit::fit(&pb).and_then(|fit| {
            let f = match fit.found {
                crate::fit::Found::Formula(f) => f,
                crate::fit::Found::Relation { p, cos } => {
                    let mut vals = env.vals.clone();
                    vals.insert(t, if cos { actual.cos() } else { actual });
                    let env2 = Env { vals };
                    let mut bs = BlockSolver { env: &env2, tmp: solver.tmp, budget: 80, deadline: s, keep: None };
                    let c = bs.candidates(&p, t, 0);
                    let x = bs.pick(c, t, false)?;
                    if cos { fun(Fun::Acos, x) } else { x }
                }
            };
            set_deadline(Some(work_now() + scaled(300)));
            // |a| keeps its sign near the current configuration only; the
            // samples reach further, so where the resolved form fails them
            // the form with |a| is tried
            let unresolved = simplify_result(&f);
            let f = simplify_result(&resolve_abs(&f, &env));
            let f = {
                let h = simplify_result(&resolve_abs(&half_angles(&f, &env), &env));
                if complexity(&h) < complexity(&f) { h } else { f }
            };
            let f = resolve_abs(&f, &env);
            let f = if crate::fit::agrees(&f, &fit.checks, &env.vals) { f } else { unresolved };
            let f = {
                let g = factor_sums(&f);
                if complexity(&g) < complexity(&f) { g } else { f }
            };
            let f = common_factors_out(&f);
            let checks = [
                !has_float(&f),
                (env.eval(&f) - actual).abs() <= 1e-8 * (1.0 + actual.abs()),
                crate::fit::agrees(&f, &fit.checks, &env.vals),
                agrees_when_perturbed(&f, &env, &rows, &unk, &target_c, &named) != Some(false),
            ];
            if std::env::var("GXPAD_DEBUG").is_ok() {
                eprintln!("fit: formula {} (exact, value, samples, perturbed: {checks:?})", crate::expr::bx::plain(&crate::expr::bx::to_bx(&f, &|s| format!("s{s}"))));
            }
            checks.iter().all(|c| *c).then_some(f)
        });
        if std::env::var("GXPAD_DEBUG").is_ok() {
            eprintln!("fit took {:?}: {:?}", tf.elapsed(), f.as_ref().map(complexity));
        }
        f
    };
    // Blocks first, briefly: most constructions are solved in milliseconds.
    // When that leaves a large or numeric formula, a fit to samples comes
    // next, and only without a small fitted formula do the blocks get the
    // whole time.
    // (the second strategy only runs after the first has tried the fit)
    let brief = if strategy == 0 { (work_now() + scaled(150)).min(deadline) } else { deadline };
    let (mut sol, mut exact) = run_blocks(&mut solver, brief);
    let small = |e: &Expr| complexity(e) <= FIT_ABOVE;
    let mut fitted: Option<Expr> = None;
    if strategy == 0 && !(exact && small(&subst_map(&target_c, &sol))) {
        fitted = run_fit(&mut solver);
    }
    if strategy == 0 && !exact && !fitted.as_ref().is_some_and(|f| complexity(f) <= FIT_SETTLES) {
        // the whole block time again: the brief pass and the fit used some up
        (sol, exact) = run_blocks(&mut solver, work_now() + deadline.saturating_duration_since(blocks_start));
    }
    let deadline = deadline.max(solver.deadline);
    solver.deadline = deadline;
    set_deadline(Some(deadline));
    // simplifying the result gets a tenth of any extra search time
    let extra = if strategy == 0 { 500 } else { 300 } + EXTRA_MS.with(|c| c.get()) / 10;
    set_deadline(Some(work_now().max(deadline) + scaled(extra)));
    let stage_t = std::cell::Cell::new((web_time::Instant::now(), work_ticks()));
    let stage = |name: &str| {
        if std::env::var("GXPAD_DEBUG").is_ok() {
            let (t, k) = stage_t.get();
            eprintln!("stage {name}: {:?}, {} ticks", t.elapsed(), work_ticks() - k);
            stage_t.set((web_time::Instant::now(), work_ticks()));
        }
    };
    stage("blocks");
    let result = resolve_abs(&subst_map(&target_c, &sol), &env);
    stage("substitution");
    // numbers from blocks solved numerically do not follow the parameters,
    // so constancy only says something about an exact formula
    let result = if exact { fold_constants(&result, &env) } else { result };
    stage("constant folding");
    // A large or numeric result: the target can have a much simpler formula
    // than the coordinates it is measured from (a distance between points with
    // nested radicals), so it is solved for directly, from the needed blocks'
    // equations with all other unknowns eliminated.
    // (the second strategy only regrounds the same construction)
    // When the blocks alone were slow, a quick direct attempt rarely
    // succeeds; a longer search (extra time) still makes one.
    let quick = EXTRA_MS.with(|c| c.get()) > 0 || work_now().saturating_duration_since(blocks_start) < scaled(500);
    let direct = if fitted.as_ref().is_some_and(|f| complexity(f) <= FIT_SETTLES) {
        None
    // (with an exact formula in hand and no fitted one, elimination rarely
    // finds what the fit could not: that is left to a longer search)
    } else if strategy == 0 && quick && (!exact || (EXTRA_MS.with(|c| c.get()) > 0 && complexity(&result) > 80)) && !target_vars.is_empty() {
        let td = web_time::Instant::now();
        let slice = |solver: &mut BlockSolver| {
            let s = work_now() + scaled(250 + EXTRA_MS.with(|c| c.get()) / 5);
            set_deadline(Some(s));
            solver.deadline = s;
        };
        // from the exact formula's square roots, then from the equations
        let mut t = None;
        // (a large formula expands into far too many terms)
        if exact && result.size() < 1500 {
            slice(&mut solver);
            t = solver.solve_target_radicals(&result, actual);
        }
        if t.is_none() {
            slice(&mut solver);
            t = solver.solve_target(&target_eqs, &target_vars, &target_c, actual);
        }
        if std::env::var("GXPAD_DEBUG").is_ok() {
            eprintln!("direct solve took {:?}", td.elapsed());
        }
        set_deadline(Some(work_now() + scaled(extra)));
        let t = t.map(|t| simplify_result(&resolve_abs(&t, &env))).filter(|t| !has_float(t) && (env.eval(t) - actual).abs() <= 1e-7 * (1.0 + actual.abs()));
        // the factor of the eliminated polynomial is chosen at the current
        // configuration only: check that it is the right one nearby
        let rows: Vec<Expr> = ce.iter().map(|&i| eqs[i].clone()).collect();
        t.filter(|t| {
            if exact {
                same_function(t, &result, &env)
            } else {
                agrees_when_perturbed(t, &env, &rows, &unk, &target_c, &named) == Some(true)
            }
        })
    } else {
        None
    };
    if std::env::var("GXPAD_DEBUG").is_ok() {
        eprintln!("direct: {:?}", direct.as_ref().map(complexity));
    }
    stage("direct");
    let direct = match (fitted, direct) {
        (Some(a), Some(b)) => Some(if complexity(&b) < complexity(&a) { b } else { a }),
        (a, b) => a.or(b),
    };
    let (result, exact) = match direct {
        Some(t) if !exact => (t, true),
        // much smaller than the other one even before simplifying it
        Some(t) if 4 * complexity(&t) < complexity(&result) => (t, true),
        Some(t) => {
            let s = simplify_result(&result);
            if complexity(&t) < complexity(&s) { (t, true) } else { (s, exact) }
        }
        None => (simplify_result(&result), exact),
    };
    // simplification can expose new |·| (perfect squares under roots)
    let result = {
        let r2 = resolve_abs(&result, &env);
        if r2 == result { result } else { simplify_result(&r2) }
    };
    // products of sines and cosines as sines and cosines of angle sums
    let result = {
        set_deadline(Some(work_now() + scaled(200)));
        let f = fourier_best(&result);
        if complexity(&f) < complexity(&result) && (env.eval(&f) - actual).abs() <= 1e-9 * (1.0 + actual.abs()) { f } else { result }
    };
    // factored sums only help a formula small enough to read
    let result = if result.size() < 1500 {
        // a short slice of its own: the simplification may have used up its time
        set_deadline(Some(work_now() + scaled(200)));
        let f = factor_sums(&result);
        if complexity(&f) < complexity(&result) { f } else { result }
    } else {
        result
    };
    stage("simplification");
    let result = settle_zero_abs(result, &env, actual);
    let rows: Vec<Expr> = ce.iter().map(|&i| eqs[i].clone()).collect();
    let result = settle_branch(result, &env, &rows, &unk, &target_c, &named);
    stage("branch settling");
    let value = env.eval(&result);
    let _ = free;
    let exact = exact && !has_float(&result);
    let grounded = !ground.is_empty();
    Ok((Derived { expr: result, exact, value, actual }, grounded))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tri(doc: &mut Doc, p: [[f64; 2]; 3]) -> [usize; 3] {
        let a = doc.add_point(p[0][0], p[0][1]);
        let b = doc.add_point(p[1][0], p[1][1]);
        let c = doc.add_point(p[2][0], p[2][1]);
        [a, b, c]
    }

    fn line(doc: &mut Doc, a: usize, b: usize) -> usize {
        doc.add_ent(Ent::Line { a, b, infinite: false })
    }

    fn solve(doc: &mut Doc) {
        let (eqns, _) = doc.equations();
        let sys = crate::solver::Sys::build(&eqns, doc.unknowns());
        let mut vals: Vec<f64> = doc.params.iter().map(|p| p.v).collect();
        assert!(sys.solve(&mut vals, &[], [0.0; 2]));
        for (p, v) in doc.params.iter_mut().zip(vals) {
            p.v = v;
        }
    }

    #[test]
    fn law_of_cosines() {
        let mut doc = Doc::new();
        let [a, b, c] = tri(&mut doc, [[1.0, 1.0], [5.0, 1.5], [2.0, 4.0]]);
        let ab = line(&mut doc, a, b);
        let ac = line(&mut doc, a, c);
        let bc = line(&mut doc, b, c);
        for l in [ab, ac, bc] {
            doc.add_dim(DimKind::Length(l), [0.0, 0.0]);
        }
        let ang = doc.add_dim(DimKind::Angle(ab, ac), [0.0, 0.0]);
        doc.dims[ang].as_mut().unwrap().driving = false;
        solve(&mut doc);
        let t = doc.measure(ang);
        let d = derive(&doc, &t).unwrap();
        let names = doc.param_names();
        let s = crate::expr::bx::to_plain(&d.expr, &|s| names.get(&(s as usize)).cloned().unwrap_or(format!("?{s}")));
        println!("{s}  exact={} value={} actual={}", d.exact, d.value, d.actual);
        assert!(d.exact);
        assert!((d.value - d.actual).abs() < 1e-9);
        assert!(d.expr.size() < 40, "{s}");
    }

    #[test]
    fn circle_tangent_to_lines() {
        // circle of radius r tangent to the x axis and to a line through the origin at angle θ
        let mut doc = Doc::new();
        let p = doc.add_point(3.0, 1.5);
        let q = doc.add_point(4.0, 3.0);
        let o = doc.origin;
        let l = line(&mut doc, o, q);
        let circ = doc.add_circle(p, 1.0);
        let r = doc.add_dim(DimKind::Radius(circ), [0.0, 0.0]);
        let _ = r;
        let xa = doc.xaxis;
        let th = doc.add_dim(DimKind::Angle(xa, l), [0.0, 0.0]);
        let _ = th;
        doc.add_dim(DimKind::Length(l), [0.0, 0.0]);
        let cc = doc.circ_val(circ);
        let s1 = {
            let (pp, qq) = doc.line_pts(doc.xaxis);
            let a = doc.pval(pp);
            let b = doc.pval(qq);
            ((b[0] - a[0]) * (cc.0[1] - a[1]) - (b[1] - a[1]) * (cc.0[0] - a[0])).signum()
        };
        let s2 = {
            let a = doc.pval(doc.origin);
            let b = doc.pval(q);
            ((b[0] - a[0]) * (cc.0[1] - a[1]) - (b[1] - a[1]) * (cc.0[0] - a[0])).signum()
        };
        doc.cons.push(Some(Con::Tangent { a: doc.xaxis, b: circ, s: s1, aux: None }));
        doc.cons.push(Some(Con::Tangent { a: l, b: circ, s: s2, aux: None }));
        solve(&mut doc);
        let dist = doc.add_dim(DimKind::Distance(o, p), [0.0, 0.0]);
        doc.dims[dist].as_mut().unwrap().driving = false;
        let t = doc.measure(dist);
        let d = derive(&doc, &t).unwrap();
        let names = doc.param_names();
        let s = crate::expr::bx::to_plain(&d.expr, &|s| names.get(&(s as usize)).cloned().unwrap_or(format!("?{s}")));
        println!("{s}  exact={} value={} actual={}", d.exact, d.value, d.actual);
        assert!(d.exact);
        assert!((d.value - d.actual).abs() < 1e-8);
    }
}

#[cfg(test)]
mod factor_sums_tests {
    use super::*;

    #[test]
    fn ptolemy_radicand_factors() {
        let names = ["a", "b", "c", "d"];
        let e = crate::expr::parse::parse("sqrt(a*b*c^2 + a*b*d^2 + a^2*c*d + b^2*c*d)/sqrt(a*b + c*d)", &|n: &str| names.iter().position(|x| *x == n).map(|i| sym(i as u32))).unwrap();
        let f = factor_sums(&e);
        let show = |e: &Expr| crate::expr::bx::to_plain(e, &|s| names[s as usize].to_string());
        assert!(complexity(&f) < complexity(&e), "{}", show(&f));
        assert_eq!(show(&f), "sqrt((a*c + b*d)*(a*d + b*c))/sqrt(a*b + c*d)");
    }
}
