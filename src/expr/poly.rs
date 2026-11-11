//! Polynomial views of expressions with respect to chosen symbols.

use super::*;

pub type Mono = Vec<u32>;
pub type Poly = BTreeMap<Mono, Expr>;

const MAX_DEG: u32 = 12;

/// Views `e` as a polynomial in `vars` whose coefficients are free of `vars`.
pub fn poly(e: &Expr, vars: &[u32]) -> Option<Poly> {
    let raw = poly_raw(e, vars)?;
    let mut out = Poly::new();
    for (m, cs) in raw {
        let c = add(cs);
        if !c.is_zero() {
            out.insert(m, c);
        }
    }
    Some(out)
}

type RawPoly = BTreeMap<Mono, Vec<Expr>>;

fn constant(e: Expr, n: usize) -> RawPoly {
    let mut p = RawPoly::new();
    p.insert(vec![0; n], vec![e]);
    p
}

fn finish(p: RawPoly) -> RawPoly {
    p.into_iter()
        .map(|(m, cs)| (m, vec![add(cs)]))
        .filter(|(_, cs)| !cs[0].is_zero())
        .collect()
}

fn poly_mul(a: &RawPoly, b: &RawPoly) -> Option<RawPoly> {
    let mut out = RawPoly::new();
    for (ma, ca) in a {
        let ca = add(ca.clone());
        for (mb, cb) in b {
            let cb = add(cb.clone());
            let m: Mono = ma.iter().zip(mb).map(|(x, y)| x + y).collect();
            if m.iter().sum::<u32>() > MAX_DEG {
                return None;
            }
            out.entry(m).or_default().push(mul(vec![ca.clone(), cb]));
        }
    }
    Some(finish(out))
}

fn poly_raw(e: &Expr, vars: &[u32]) -> Option<RawPoly> {
    if past_deadline() {
        return None;
    }
    let n = vars.len();
    if !contains_any(e, vars) {
        return Some(constant(e.clone(), n));
    }
    match e.node() {
        Node::Sym(s) => {
            let i = vars.iter().position(|v| v == s)?;
            let mut m = vec![0; n];
            m[i] = 1;
            let mut p = RawPoly::new();
            p.insert(m, vec![one()]);
            Some(p)
        }
        Node::Add(ts) => {
            let mut out = RawPoly::new();
            for t in ts {
                for (m, cs) in poly_raw(t, vars)? {
                    out.entry(m).or_default().extend(cs);
                }
            }
            Some(out)
        }
        Node::Mul(fs) => {
            let mut acc = constant(one(), n);
            let mut cst = Vec::new();
            for f in fs {
                if !contains_any(f, vars) {
                    cst.push(f.clone());
                    continue;
                }
                let p = finish(poly_raw(f, vars)?);
                acc = poly_mul(&acc, &p)?;
            }
            if !cst.is_empty() {
                let c = mul(cst);
                for cs in acc.values_mut() {
                    let v = add(std::mem::take(cs));
                    *cs = vec![mul(vec![c.clone(), v])];
                }
            }
            Some(acc)
        }
        Node::Pow(b, ex) => {
            let k = ex.as_int()?;
            if !(0..=MAX_DEG as i64).contains(&k) {
                return None;
            }
            let base = finish(poly_raw(b, vars)?);
            let mut acc = constant(one(), n);
            for _ in 0..k {
                acc = poly_mul(&acc, &base)?;
            }
            Some(acc)
        }
        _ => None,
    }
}

pub fn total_degree(p: &Poly) -> u32 {
    p.keys().map(|m| m.iter().sum::<u32>()).max().unwrap_or(0)
}

/// Coefficients of `e` as a univariate polynomial in `v`, lowest degree first.
pub fn coeffs(e: &Expr, v: u32) -> Option<Vec<Expr>> {
    let p = poly(e, &[v])?;
    let deg = p.keys().map(|m| m[0]).max().unwrap_or(0) as usize;
    let mut out = vec![zero(); deg + 1];
    for (m, c) in p {
        out[m[0] as usize] = c;
    }
    Some(out)
}

pub fn from_poly(p: &Poly, vars: &[u32]) -> Expr {
    add(p
        .iter()
        .map(|(m, c)| {
            let mut fs = vec![c.clone()];
            for (i, k) in m.iter().enumerate() {
                if *k > 0 {
                    fs.push(pow(sym(vars[i]), int(*k as i64)));
                }
            }
            mul(fs)
        })
        .collect())
}

/// Splits `e` into numerator / denominator, moving into the denominator only
/// factors that involve `vars`.
pub fn split_frac(e: &Expr, vars: &[u32]) -> (Expr, Expr) {
    if !contains_any(e, vars) {
        return (e.clone(), one());
    }
    match e.node() {
        Node::Add(ts) => {
            let mut parts: Vec<(Expr, Expr)> = ts.iter().map(|t| split_frac(t, vars)).collect();
            // group by denominator
            let mut den = one();
            let mut dens: Vec<Expr> = Vec::new();
            for (_, d) in &parts {
                if !d.is_one() && !dens.contains(d) {
                    dens.push(d.clone());
                    den = mul(vec![den, d.clone()]);
                }
            }
            if dens.is_empty() {
                return (e.clone(), one());
            }
            let mut nums = Vec::new();
            for (n, d) in parts.drain(..) {
                let mut f = vec![n];
                for o in &dens {
                    if *o != d {
                        f.push(o.clone());
                    }
                }
                nums.push(mul(f));
            }
            (add(nums), den)
        }
        Node::Mul(fs) => {
            let mut n = Vec::new();
            let mut d = Vec::new();
            for f in fs {
                let (a, b) = split_frac(f, vars);
                n.push(a);
                d.push(b);
            }
            (mul(n), mul(d))
        }
        Node::Pow(b, ex) => {
            if let Some(k) = ex.as_int() {
                let (nb, db) = split_frac(b, vars);
                if k < 0 {
                    return (pow(db, int(-k)), pow(nb, int(-k)));
                }
                return (pow(nb, int(k)), pow(db, int(k)));
            }
            if let Some(r) = ex.as_rat() {
                if r.n < 0 {
                    return (one(), pow(b.clone(), num(Num::R(r.neg()))));
                }
            }
            (e.clone(), one())
        }
        _ => (e.clone(), one()),
    }
}
