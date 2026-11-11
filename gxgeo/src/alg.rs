//! Exact polynomial algebra over Q: sparse multivariate polynomials,
//! resultants, subresultant sequences, and factorization (univariate over Q,
//! multivariate in a main variable by Hensel lifting).

use num_bigint::BigInt;
use num_rational::BigRational as Q;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::collections::BTreeMap;

pub type Mono = Vec<u16>;

/// Sparse polynomial in `n` variables with rational coefficients. Keys are
/// exponent vectors in lexicographic order (variable 0 most significant).
#[derive(Clone, Debug, PartialEq)]
pub struct MPoly {
    pub n: usize,
    pub t: BTreeMap<Mono, Q>,
}

pub fn q(i: i64) -> Q {
    Q::from_integer(BigInt::from(i))
}

fn qf(x: &Q) -> f64 {
    x.to_f64().unwrap_or(f64::NAN)
}

impl MPoly {
    pub fn zero(n: usize) -> MPoly {
        MPoly { n, t: BTreeMap::new() }
    }
    pub fn constant(n: usize, c: Q) -> MPoly {
        let mut p = MPoly::zero(n);
        if !c.is_zero() {
            p.t.insert(vec![0; n], c);
        }
        p
    }
    pub fn one(n: usize) -> MPoly {
        MPoly::constant(n, Q::one())
    }
    pub fn var(n: usize, i: usize) -> MPoly {
        let mut m = vec![0; n];
        m[i] = 1;
        let mut p = MPoly::zero(n);
        p.t.insert(m, Q::one());
        p
    }
    pub fn is_zero(&self) -> bool {
        self.t.is_empty()
    }
    pub fn terms(&self) -> usize {
        self.t.len()
    }
    pub fn add_term(&mut self, m: Mono, c: Q) {
        // (a term costs several times an expression node: exact rational
        // arithmetic and a map insertion)
        crate::expr::charge(8);
        if c.is_zero() {
            return;
        }
        let e = self.t.entry(m.clone()).or_insert_with(Q::zero);
        *e += c;
        if e.is_zero() {
            self.t.remove(&m);
        }
    }
    pub fn add(&self, o: &MPoly) -> MPoly {
        let mut r = self.clone();
        for (m, c) in &o.t {
            r.add_term(m.clone(), c.clone());
        }
        r
    }
    pub fn sub(&self, o: &MPoly) -> MPoly {
        let mut r = self.clone();
        for (m, c) in &o.t {
            r.add_term(m.clone(), -c.clone());
        }
        r
    }
    pub fn scale(&self, c: &Q) -> MPoly {
        if c.is_zero() {
            return MPoly::zero(self.n);
        }
        MPoly { n: self.n, t: self.t.iter().map(|(m, x)| (m.clone(), x * c)).collect() }
    }
    /// Product. Past the derivation deadline it stops early and returns a
    /// partial result; callers discard everything once the deadline passed.
    pub fn mul(&self, o: &MPoly) -> MPoly {
        let mut r = MPoly::zero(self.n);
        let mut work = 0usize;
        for (ma, ca) in &self.t {
            work += o.t.len();
            if work > 4096 {
                work = 0;
                if crate::expr::past_deadline() {
                    return r;
                }
            }
            for (mb, cb) in &o.t {
                let m: Mono = ma.iter().zip(mb).map(|(a, b)| a + b).collect();
                r.add_term(m, ca * cb);
            }
        }
        r
    }
    pub fn pow(&self, k: u32) -> MPoly {
        let mut r = MPoly::one(self.n);
        for _ in 0..k {
            r = r.mul(self);
        }
        r
    }
    pub fn deg(&self, i: usize) -> u32 {
        self.t.keys().map(|m| m[i] as u32).max().unwrap_or(0)
    }
    pub fn total_deg_in(&self, vars: &[usize]) -> u32 {
        self.t.keys().map(|m| vars.iter().map(|&v| m[v] as u32).sum()).max().unwrap_or(0)
    }
    pub fn uses(&self, i: usize) -> bool {
        self.t.keys().any(|m| m[i] > 0)
    }
    pub fn is_const(&self) -> bool {
        self.t.keys().all(|m| m.iter().all(|e| *e == 0))
    }
    /// Coefficients in variable `i`, lowest first (variable `i` removed).
    pub fn coeffs(&self, i: usize) -> Vec<MPoly> {
        let d = self.deg(i) as usize;
        let mut out = vec![MPoly::zero(self.n); d + 1];
        for (m, c) in &self.t {
            let k = m[i] as usize;
            let mut m2 = m.clone();
            m2[i] = 0;
            out[k].add_term(m2, c.clone());
        }
        out
    }
    pub fn from_coeffs(n: usize, i: usize, cs: &[MPoly]) -> MPoly {
        let mut r = MPoly::zero(n);
        for (k, c) in cs.iter().enumerate() {
            for (m, x) in &c.t {
                let mut m2 = m.clone();
                m2[i] += k as u16;
                r.add_term(m2, x.clone());
            }
        }
        r
    }
    pub fn lc(&self, i: usize) -> MPoly {
        self.coeffs(i).pop().unwrap_or_else(|| MPoly::zero(self.n))
    }
    pub fn derivative(&self, i: usize) -> MPoly {
        let mut r = MPoly::zero(self.n);
        for (m, c) in &self.t {
            if m[i] > 0 {
                let mut m2 = m.clone();
                m2[i] -= 1;
                r.add_term(m2, c * q(m[i] as i64));
            }
        }
        r
    }
    /// Substitutes variable `i` by the polynomial `v`.
    pub fn subst(&self, i: usize, v: &MPoly) -> MPoly {
        let cs = self.coeffs(i);
        let mut r = MPoly::zero(self.n);
        for c in cs.iter().rev() {
            r = r.mul(v).add(c);
        }
        r
    }
    pub fn subst_const(&self, i: usize, a: &Q) -> MPoly {
        self.subst(i, &MPoly::constant(self.n, a.clone()))
    }
    /// Value and magnitude scale (sum of absolute term values).
    pub fn eval_f64(&self, vals: &[f64]) -> (f64, f64) {
        let (mut s, mut a) = (0.0, 0.0);
        for (m, c) in &self.t {
            let mut t = qf(c);
            for (k, e) in m.iter().enumerate() {
                if *e > 0 {
                    t *= vals[k].powi(*e as i32);
                }
            }
            s += t;
            a += t.abs();
        }
        (s, a)
    }
    /// Value and a magnitude scale in which every value counts as at least 1
    /// in size, so a lone small factor (a coordinate that is 0) still
    /// registers as zero.
    pub fn eval_scaled(&self, vals: &[f64]) -> (f64, f64) {
        let (mut s, mut a) = (0.0, 0.0);
        for (m, c) in &self.t {
            let (mut t, mut u) = (qf(c), qf(c).abs());
            for (k, e) in m.iter().enumerate() {
                if *e > 0 {
                    t *= vals[k].powi(*e as i32);
                    u *= vals[k].abs().max(1.0).powi(*e as i32);
                }
            }
            s += t;
            a += u;
        }
        (s, a)
    }
    /// True when the polynomial is (numerically) zero at `vals`.
    pub fn vanishes(&self, vals: &[f64]) -> bool {
        if self.is_zero() {
            return false;
        }
        let (v, a) = self.eval_scaled(vals);
        v.is_finite() && v.abs() <= 1e-7 * a.max(1e-300)
    }
    fn lead(&self) -> Option<(&Mono, &Q)> {
        self.t.iter().next_back()
    }
    /// Exact quotient self / b, or None when b does not divide.
    pub fn div_exact(&self, b: &MPoly) -> Option<MPoly> {
        let (mb, cb) = {
            let (m, c) = b.lead()?;
            (m.clone(), c.clone())
        };
        if self.is_zero() {
            return Some(MPoly::zero(self.n));
        }
        // a quotient term beyond these degrees proves b does not divide
        let maxq: Vec<i64> = (0..self.n).map(|i| self.deg(i) as i64 - b.deg(i) as i64).collect();
        if maxq.iter().any(|d| *d < 0) {
            return None;
        }
        let mut r = self.clone();
        let mut out = MPoly::zero(self.n);
        let mut guard = 0usize;
        while let Some((m, c)) = r.lead() {
            guard += 1;
            if guard > 200_000 || (guard % 64 == 0 && crate::expr::past_deadline()) {
                return None;
            }
            if m.iter().zip(&mb).any(|(a, b)| a < b) {
                return None;
            }
            let mq: Mono = m.iter().zip(&mb).map(|(a, b)| a - b).collect();
            if mq.iter().zip(&maxq).any(|(e, d)| *e as i64 > *d) {
                return None;
            }
            let cq = c / &cb;
            for (bm, bc) in &b.t {
                let m2: Mono = mq.iter().zip(bm).map(|(a, b)| a + b).collect();
                r.add_term(m2, -(&cq * bc));
            }
            out.add_term(mq, cq);
        }
        Some(out)
    }
    /// Pseudo-remainder of self by b in variable i.
    pub fn prem(&self, b: &MPoly, i: usize) -> MPoly {
        let db = b.deg(i);
        let lb = b.lc(i);
        let mut r = self.clone();
        let da = self.deg(i);
        if da < db {
            return r;
        }
        for _ in 0..=(da - db) {
            if crate::expr::past_deadline() {
                return MPoly::zero(self.n);
            }
            let dr = r.deg(i);
            if r.is_zero() || dr < db {
                r = r.mul(&lb);
                continue;
            }
            let lr = r.lc(i);
            let mut xm = MPoly::one(self.n);
            if dr > db {
                xm = MPoly::var(self.n, i).pow(dr - db);
            }
            r = r.mul(&lb).sub(&lr.mul(&xm).mul(b));
        }
        r
    }
    /// Shifts variable i: p(x_i + a).
    pub fn shift(&self, i: usize, a: &Q) -> MPoly {
        if a.is_zero() {
            return self.clone();
        }
        let v = MPoly::var(self.n, i).add(&MPoly::constant(self.n, a.clone()));
        self.subst(i, &v)
    }
}

/// Row reduction of the polynomials as vectors over monomials with rational
/// coefficients, monomials of highest total degree in `vars` first. Shared
/// nonlinear parts cancel (two circle equations give a line). Zero rows are
/// dropped.
pub fn interreduce(polys: &[MPoly], vars: &[usize]) -> Vec<MPoly> {
    let key = |m: &Mono| (vars.iter().map(|&v| m[v] as u32).sum::<u32>(), m.clone());
    let pivot = |p: &MPoly| p.t.keys().max_by_key(|m| key(m)).cloned();
    let mut rows: Vec<(Mono, MPoly)> = Vec::new();
    for p in polys {
        let mut p = p.clone();
        for (pm, r) in &rows {
            if let Some(c) = p.t.get(pm).cloned() {
                p = p.sub(&r.scale(&c));
            }
        }
        let Some(pm) = pivot(&p) else { continue };
        let inv = Q::one() / &p.t[&pm];
        p = p.scale(&inv);
        for (_, r) in rows.iter_mut() {
            if let Some(c) = r.t.get(&pm).cloned() {
                *r = r.sub(&p.scale(&c));
            }
        }
        rows.push((pm, p));
    }
    rows.into_iter().map(|(_, r)| r).collect()
}

/// p with variable y replaced by -c0/c1, times c1^deg_y(p) (a polynomial).
pub fn subst_linear(p: &MPoly, y: usize, c0: &MPoly, c1: &MPoly) -> MPoly {
    let d = p.deg(y);
    let cs = p.coeffs(y);
    let num = c0.scale(&q(-1));
    let mut r = MPoly::zero(p.n);
    // Horner in the homogenized form: sum cs[k] num^k c1^(d-k)
    let mut c1p = vec![MPoly::one(p.n)];
    for k in 1..=d as usize {
        c1p.push(c1p[k - 1].mul(c1));
    }
    let mut nump = MPoly::one(p.n);
    for (k, c) in cs.iter().enumerate() {
        if !c.is_zero() {
            r = r.add(&c.mul(&nump).mul(&c1p[d as usize - k]));
        }
        if k < d as usize {
            nump = nump.mul(&num);
        }
    }
    r
}

/// p divided by f as often as f divides exactly.
pub fn strip_factor(p: &MPoly, f: &MPoly) -> MPoly {
    if f.is_const() {
        return p.clone();
    }
    let mut p = p.clone();
    for _ in 0..64 {
        match p.div_exact(f) {
            Some(h) if !h.is_zero() => p = h,
            _ => break,
        }
    }
    p
}

/// Resultant of a and b in variable i (fraction-free Bareiss on the Sylvester matrix).
pub fn resultant(a: &MPoly, b: &MPoly, i: usize) -> Option<MPoly> {
    let n = a.n;
    let (da, db) = (a.deg(i) as usize, b.deg(i) as usize);
    if da == 0 {
        return Some(a.pow(db as u32));
    }
    if db == 0 {
        return Some(b.pow(da as u32));
    }
    let (ca, cb) = (a.coeffs(i), b.coeffs(i));
    let size = da + db;
    let mut m: Vec<Vec<MPoly>> = vec![vec![MPoly::zero(n); size]; size];
    for r in 0..db {
        for k in 0..=da {
            m[r][r + k] = ca[da - k].clone();
        }
    }
    for r in 0..da {
        for k in 0..=db {
            m[db + r][r + k] = cb[db - k].clone();
        }
    }
    let mut sign = false;
    let mut prev = MPoly::one(n);
    for k in 0..size {
        if crate::expr::past_deadline() {
            return None;
        }
        // pivot
        if m[k][k].is_zero() {
            let p = (k + 1..size).find(|&r| !m[r][k].is_zero());
            match p {
                Some(p) => {
                    m.swap(k, p);
                    sign = !sign;
                }
                None => return Some(MPoly::zero(n)),
            }
        }
        for r in k + 1..size {
            if crate::expr::past_deadline() {
                return None;
            }
            for c in k + 1..size {
                let v = m[k][k].mul(&m[r][c]).sub(&m[r][k].mul(&m[k][c]));
                m[r][c] = v.div_exact(&prev)?;
            }
            m[r][k] = MPoly::zero(n);
        }
        prev = m[k][k].clone();
    }
    let d = m[size - 1][size - 1].clone();
    Some(if sign { d.scale(&q(-1)) } else { d })
}

/// Subresultant polynomial remainder sequence of a and b in variable i.
pub fn subresultant_chain(a: &MPoly, b: &MPoly, i: usize) -> Vec<MPoly> {
    let n = a.n;
    let (mut a, mut b) = if a.deg(i) >= b.deg(i) { (a.clone(), b.clone()) } else { (b.clone(), a.clone()) };
    let mut out = vec![a.clone(), b.clone()];
    if b.is_zero() {
        return out;
    }
    let mut g = MPoly::one(n);
    let mut h = MPoly::one(n);
    for _ in 0..64 {
        if crate::expr::past_deadline() {
            break;
        }
        let d = a.deg(i) - b.deg(i);
        let r = a.prem(&b, i);
        if r.is_zero() {
            break;
        }
        let den = g.mul(&h.pow(d));
        let Some(nb) = r.div_exact(&den) else { break };
        out.push(nb.clone());
        if nb.deg(i) == 0 {
            break;
        }
        a = b;
        b = nb;
        g = a.lc(i);
        h = if d == 0 {
            h
        } else {
            let num = g.pow(d);
            match num.div_exact(&h.pow(d - 1)) {
                Some(x) => x,
                None => break,
            }
        };
    }
    out
}

// ---------------------------------------------------------------- univariate over Q

pub type UPoly = Vec<Q>;

fn utrim(p: &mut UPoly) {
    while p.last().is_some_and(|c| c.is_zero()) {
        p.pop();
    }
}

fn udeg(p: &UPoly) -> usize {
    p.len().saturating_sub(1)
}



/// Division with remainder in Q[x].
fn udivrem(a: &UPoly, b: &UPoly) -> (UPoly, UPoly) {
    let mut r = a.clone();
    utrim(&mut r);
    let db = udeg(b);
    let lb = b[db].clone();
    if r.len() < b.len() {
        return (vec![], r);
    }
    let mut qv = vec![Q::zero(); r.len() - b.len() + 1];
    while r.len() >= b.len() && !r.is_empty() {
        let k = r.len() - b.len();
        let c = r.last().unwrap() / &lb;
        for (j, y) in b.iter().enumerate() {
            r[k + j] -= &c * y;
        }
        qv[k] = c;
        r.pop();
        utrim(&mut r);
    }
    utrim(&mut qv);
    (qv, r)
}

fn umonic(p: &UPoly) -> UPoly {
    let l = p.last().unwrap().clone();
    p.iter().map(|c| c / &l).collect()
}

fn ugcd(a: &UPoly, b: &UPoly) -> UPoly {
    let (mut a, mut b) = (a.clone(), b.clone());
    utrim(&mut a);
    utrim(&mut b);
    while !b.is_empty() {
        let (_, r) = udivrem(&a, &b);
        a = b;
        b = r;
    }
    if a.is_empty() { a } else { umonic(&a) }
}


fn uderiv(p: &UPoly) -> UPoly {
    let mut r: UPoly = p.iter().enumerate().skip(1).map(|(i, c)| c * q(i as i64)).collect();
    utrim(&mut r);
    r
}

/// Complex roots by the Aberth–Ehrlich iteration.
fn uroots(p: &UPoly) -> Option<Vec<(f64, f64)>> {
    let n = udeg(p);
    if n == 0 {
        return Some(vec![]);
    }
    let c: Vec<f64> = p.iter().map(qf).collect();
    if c.iter().any(|x| !x.is_finite()) {
        return None;
    }
    let lead = c[n];
    let a: Vec<f64> = c.iter().map(|x| x / lead).collect();
    let bound = 1.0 + a[..n].iter().map(|x| x.abs()).fold(0.0, f64::max);
    let mut z: Vec<(f64, f64)> = (0..n)
        .map(|k| {
            let t = std::f64::consts::TAU * k as f64 / n as f64 + 0.4;
            (bound * 0.7 * t.cos(), bound * 0.7 * t.sin())
        })
        .collect();
    let cmul = |a: (f64, f64), b: (f64, f64)| (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0);
    let cdiv = |a: (f64, f64), b: (f64, f64)| {
        let d = b.0 * b.0 + b.1 * b.1;
        ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d)
    };
    for _ in 0..2000 {
        let mut maxstep: f64 = 0.0;
        for k in 0..n {
            // p(z), p'(z) by Horner
            let (mut pv, mut dv) = ((a[n], 0.0), (0.0, 0.0));
            for j in (0..n).rev() {
                dv = (cmul(dv, z[k]).0 + pv.0, cmul(dv, z[k]).1 + pv.1);
                pv = (cmul(pv, z[k]).0 + a[j], cmul(pv, z[k]).1);
            }
            if pv.0 == 0.0 && pv.1 == 0.0 {
                continue;
            }
            let ratio = cdiv(pv, dv);
            let mut s = (0.0, 0.0);
            for j in 0..n {
                if j != k {
                    let d = (z[k].0 - z[j].0, z[k].1 - z[j].1);
                    let inv = cdiv((1.0, 0.0), d);
                    s = (s.0 + inv.0, s.1 + inv.1);
                }
            }
            let den = (1.0 - cmul(ratio, s).0, -cmul(ratio, s).1);
            let w = cdiv(ratio, den);
            z[k] = (z[k].0 - w.0, z[k].1 - w.1);
            maxstep = maxstep.max(w.0.hypot(w.1) / (1.0 + z[k].0.hypot(z[k].1)));
        }
        if maxstep < 1e-15 {
            break;
        }
    }
    Some(z)
}

fn rationalize(x: f64, maxd: i64) -> Option<Q> {
    if !x.is_finite() || x.abs() > 1e15 {
        return None;
    }
    // continued fraction
    let (mut h0, mut h1, mut k0, mut k1) = (0i128, 1i128, 1i128, 0i128);
    let mut y = x;
    for _ in 0..40 {
        let a = y.floor();
        if a.abs() > 1e15 {
            break;
        }
        let a = a as i128;
        let (h2, k2) = (a * h1 + h0, a * k1 + k0);
        if k2 > maxd as i128 {
            break;
        }
        if ((h2 as f64) / (k2 as f64) - x).abs() <= 1e-9 * (1.0 + x.abs()) {
            return Some(Q::new(BigInt::from(h2), BigInt::from(k2)));
        }
        h0 = h1;
        h1 = h2;
        k0 = k1;
        k1 = k2;
        let f = y - y.floor();
        if f.abs() < 1e-15 {
            break;
        }
        y = 1.0 / f;
    }
    None
}

/// Monic irreducible factors over Q of a square-free polynomial, found from
/// conjugate-closed subsets of its numeric roots and verified exactly.
pub fn ufactor(p: &UPoly) -> Vec<UPoly> {
    let mut rest = p.clone();
    utrim(&mut rest);
    if udeg(&rest) <= 1 {
        return vec![umonic(&rest)];
    }
    let mut out = Vec::new();
    for size in 1..=udeg(&rest) / 2 {
        loop {
            let n = udeg(&rest);
            if size > n / 2 {
                break;
            }
            let Some(roots) = uroots(&rest) else { return finish(out, rest) };
            let mut found: Option<UPoly> = None;
            let mut idx: Vec<usize> = (0..size).collect();
            let mut tries = 0usize;
            'subsets: loop {
                tries += 1;
                if tries > 200_000 || crate::expr::past_deadline() {
                    break;
                }
                // monic product of the chosen roots
                let mut coef: Vec<(f64, f64)> = vec![(1.0, 0.0)];
                for &k in &idx {
                    let r = roots[k];
                    let mut nc = vec![(0.0, 0.0); coef.len() + 1];
                    for (j, c) in coef.iter().enumerate() {
                        nc[j + 1].0 += c.0;
                        nc[j + 1].1 += c.1;
                        nc[j].0 -= c.0 * r.0 - c.1 * r.1;
                        nc[j].1 -= c.0 * r.1 + c.1 * r.0;
                    }
                    coef = nc;
                }
                let scale = 1.0 + coef.iter().map(|c| c.0.abs()).fold(0.0, f64::max);
                if coef.iter().all(|c| c.1.abs() <= 1e-6 * scale) {
                    let cand: Option<UPoly> = coef.iter().map(|c| rationalize(c.0, 1_000_000)).collect();
                    if let Some(f) = cand {
                        let (_, r) = udivrem(&rest, &f);
                        if r.is_empty() {
                            found = Some(f);
                            break 'subsets;
                        }
                    }
                }
                // next combination
                let mut i = size;
                loop {
                    if i == 0 {
                        break 'subsets;
                    }
                    i -= 1;
                    if idx[i] < n - size + i {
                        idx[i] += 1;
                        for j in i + 1..size {
                            idx[j] = idx[j - 1] + 1;
                        }
                        break;
                    }
                    if i == 0 {
                        break 'subsets;
                    }
                }
            }
            match found {
                Some(f) => {
                    rest = udivrem(&rest, &f).0;
                    out.push(f);
                }
                None => break,
            }
        }
    }
    finish(out, rest)
}

fn finish(mut out: Vec<UPoly>, rest: UPoly) -> Vec<UPoly> {
    if udeg(&rest) >= 1 {
        out.push(umonic(&rest));
    }
    out
}

// ---------------------------------------------------------------- multivariate factorization

fn to_upoly(p: &MPoly, x: usize) -> Option<UPoly> {
    let mut u: UPoly = Vec::new();
    for c in p.coeffs(x) {
        if !c.is_const() {
            return None;
        }
        u.push(c.t.values().next().cloned().unwrap_or_else(Q::zero));
    }
    utrim(&mut u);
    Some(u)
}

fn from_upoly(n: usize, x: usize, u: &UPoly) -> MPoly {
    let cs: Vec<MPoly> = u.iter().map(|c| MPoly::constant(n, c.clone())).collect();
    MPoly::from_coeffs(n, x, &cs)
}


/// The variable among `vars` in which `p` is cheapest to factor: lowest
/// degree (fewer image factors, a shorter lift), then the simplest leading
/// coefficient.
pub fn cheapest_main(p: &MPoly, vars: &[usize]) -> Option<usize> {
    let all: Vec<usize> = (0..p.n).collect();
    vars.iter()
        .copied()
        .filter(|&v| p.deg(v) >= 1)
        .min_by_key(|&v| {
            let d = p.deg(v) as u64;
            let lc = p.lc(v);
            (d, lc.total_deg_in(&all) as u64, lc.terms())
        })
}

/// Factors of p as a polynomial in the main variable x (coefficients in the
/// other variables). Factors are not necessarily primitive; their product
/// equals the square-free part of p up to a factor free of x.
pub fn factor_main(p: &MPoly, x: usize) -> Vec<MPoly> {
    // factor in a ring of only the variables p uses: the modular lifting
    // packs at most KMAX variables into a monomial key
    let used: Vec<usize> = (0..p.n).filter(|&v| p.uses(v) || v == x).collect();
    if used.len() == p.n {
        return factor_in(p, x, true);
    }
    let small = remap(p, used.len(), |v| used.iter().position(|u| *u == v));
    let xs = used.iter().position(|u| *u == x).unwrap();
    factor_in(&small, xs, true).into_iter().map(|f| remap(&f, p.n, |v| Some(used[v]))).collect()
}

/// p in a ring of n variables, variable v becoming `to(v)` (None: unused).
fn remap(p: &MPoly, n: usize, to: impl Fn(usize) -> Option<usize>) -> MPoly {
    let mut out = MPoly::zero(n);
    for (m, c) in &p.t {
        let mut m2 = vec![0; n];
        for (v, e) in m.iter().enumerate() {
            if *e > 0 {
                m2[to(v).expect("variable in use")] = *e;
            }
        }
        out.add_term(m2.into_iter().collect(), c.clone());
    }
    out
}

fn factor_in(p: &MPoly, x: usize, switch: bool) -> Vec<MPoly> {
    let n = p.n;
    // monomial content: each variable dividing every term is a factor
    if let Some(lo) = p.t.keys().cloned().reduce(|a, b| a.iter().zip(&b).map(|(u, v)| *u.min(v)).collect()) {
        if lo.iter().any(|e| *e > 0) {
            let rest = MPoly { n, t: p.t.iter().map(|(m, c)| (m.iter().zip(&lo).map(|(a, b)| a - b).collect(), c.clone())).collect() };
            let mut out: Vec<MPoly> = (0..n).filter(|&v| lo[v] > 0).map(|v| MPoly::var(n, v)).collect();
            if !rest.is_const() {
                let main = if rest.deg(x) >= 1 { x } else { (0..n).find(|&v| rest.deg(v) >= 1).unwrap() };
                out.extend(factor_main(&rest, main));
            }
            return out;
        }
    }
    let deg = p.deg(x);
    if p.terms() > 4000 {
        return gave_up(p);
    }
    // content in x: factors of the smallest coefficient that divide them all
    if deg >= 1 {
        let cs: Vec<MPoly> = p.coeffs(x).into_iter().filter(|c| !c.is_zero()).collect();
        if let Some(small) = cs.iter().min_by_key(|c| c.terms()).filter(|c| !c.is_const() && cs.len() >= 2 && may_share_factor(&cs, c)) {
            let vs: Vec<usize> = (0..n).filter(|&v| small.uses(v)).collect();
            let mut cont: Vec<MPoly> = Vec::new();
            let mut rest = p.clone();
            if let Some(v) = cheapest_main(small, &vs) {
                for f in factor_main(small, v) {
                    if f.is_const() || crate::expr::past_deadline() {
                        continue;
                    }
                    let mut hit = false;
                    for _ in 0..32 {
                        match rest.div_exact(&f) {
                            Some(h) if !h.is_zero() => {
                                rest = h;
                                hit = true;
                            }
                            _ => break,
                        }
                    }
                    if hit {
                        cont.push(f);
                    }
                }
            }
            if !cont.is_empty() {
                if !rest.is_const() {
                    cont.extend(factor_in(&rest, x, true));
                }
                return cont;
            }
        }
    }
    if deg <= 1 {
        return vec![p.clone()];
    }
    let params: Vec<usize> = (0..n).filter(|&i| i != x && p.uses(i)).collect();
    if deg == 2 && !params.is_empty() {
        // a x² + b x + c splits exactly when b² − 4ac is a square s²:
        // 4a·p = (2a x + b − s)(2a x + b + s)
        let cs = p.coeffs(x);
        let (c, b, a) = (&cs[0], &cs[1], &cs[2]);
        let disc = b.mul(b).sub(&a.mul(c).scale(&q(4)));
        match poly_sqrt(&disc) {
            Some(Some(sq)) => {
                let lin = |sg: i64| {
                    let mut xa = a.scale(&q(2)).mul(&MPoly::var(n, x));
                    xa = xa.add(b).add(&sq.scale(&q(sg)));
                    xa
                };
                let parts: Vec<MPoly> = if a.is_const() {
                    vec![]
                } else {
                    match (0..n).find(|&v| a.deg(v) >= 1) {
                        Some(v) => factor_main(a, v).into_iter().filter(|f| !f.is_const()).collect(),
                        None => vec![],
                    }
                };
                let mut out = Vec::new();
                for sg in [-1, 1] {
                    let mut f = lin(sg);
                    for part in &parts {
                        f = strip_factor(&f, part);
                    }
                    out.push(f);
                }
                return out;
            }
            Some(None) => return vec![p.clone()],
            None => {}
        }
    }
    if params.is_empty() {
        let Some(u) = to_upoly(p, x) else { return gave_up(p) };
        let g = ugcd(&u, &uderiv(&u));
        let u = if udeg(&g) > 0 { udivrem(&u, &g).0 } else { u };
        return ufactor(&u).into_iter().map(|f| from_upoly(n, x, &f)).collect();
    }
    // a square-free image at the evaluation point proves p square-free;
    // only otherwise is the (costly) subresultant square-free part needed
    let pick_point = |sf: &MPoly, want: usize| -> Vec<(Vec<Q>, UPoly)> {
        let mut found: Vec<(Vec<Q>, UPoly)> = Vec::new();
        let deg = sf.deg(x) as usize;
        let mut seed: u64 = 0x2545_F491_4F6C_DD1D;
        let (mut seen_gd, mut same_gd): (Option<usize>, u32) = (None, 0);
        for attempt in 0..48usize {
            if crate::expr::past_deadline() {
                return found;
            }
            // zeros keep the shifted polynomial sparse: the first points are
            // mostly zero, later ones spread out
            let a: Vec<Q> = params
                .iter()
                .map(|_| {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                    let h = seed >> 33;
                    let zero_odds = if attempt == 0 { 1.0 } else { 0.6 - attempt as f64 / 60.0 };
                    if ((h % 1000) as f64) < zero_odds * 1000.0 {
                        q(0)
                    } else {
                        let span = 1 + attempt as i64 / 6;
                        let v = 1 + ((h / 1000) as i64 % span);
                        q(if (h / 7) % 2 == 0 { v } else { -v })
                    }
                })
                .collect();
            // square-free modulo a prime (with the same degree) implies square-free over Q
            let pr = PRIMES[0];
            let Some(um) = image_mod(sf, x, &params, &a, pr) else { continue };
            // the leading coefficient must not vanish there
            if um.len() != deg + 1 {
                continue;
            }
            let dm: Vec<u64> = {
                let mut d: Vec<u64> = um.iter().enumerate().skip(1).map(|(i, c)| mulm(*c, i as u64 % pr, pr)).collect();
                um_trim(&mut d);
                d
            };
            let gd = um_gcd(&um, &dm, pr).len().saturating_sub(1);
            if gd == 0 {
                let mut u = sf.clone();
                for (j, &v) in params.iter().enumerate() {
                    u = u.subst_const(v, &a[j]);
                }
                let Some(uu) = to_upoly(&u, x) else { continue };
                if udeg(&uu) == deg {
                    found.push((a, uu));
                    if found.len() >= want {
                        return found;
                    }
                }
                continue;
            }
            // the same repeated part at every point: a repeated factor, not bad luck
            if seen_gd.is_some_and(|g| g != gd) {
                same_gd = 0;
            }
            seen_gd = Some(gd);
            same_gd += 1;
            if same_gd >= 6 && found.is_empty() {
                return found;
            }
        }
        found
    };
    let mut sf = p.clone();
    let mut points = pick_point(&sf, 3);
    if points.is_empty() && switch {
        // another main variable may have square-free images
        let others: Vec<usize> = params.iter().copied().filter(|&v| p.deg(v) >= 2).collect();
        let mut order = others.clone();
        let all: Vec<usize> = (0..n).collect();
        order.sort_by_key(|&v| {
            let d = p.deg(v) as u64;
            (p.lc(v).total_deg_in(&all) as u64 * d.saturating_sub(1), d)
        });
        for v in order {
            if crate::expr::past_deadline() {
                return gave_up(p);
            }
            GAVE_UP.with(|g| g.set(false));
            let fs = factor_in(p, v, false);
            // a factorization, or a proof of irreducibility, in any variable is final
            if fs.len() > 1 || fs[0] != *p || !GAVE_UP.with(|g| g.get()) {
                return fs;
            }
        }
    }
    if points.is_empty() && !switch {
        return gave_up(p);
    }
    if points.is_empty() {
        // repeated factors: the square-free part by lifting gcd(p, ∂p/∂x) out of ∂p/∂x
        match repeated_part(p, x, &params) {
            // the earlier points were only unlucky
            Repeated::None(pts) => points = pts,
            Repeated::Part(s) => {
                if s.deg(x) <= 1 {
                    return vec![s];
                }
                points = pick_point(&s, 3);
                if !points.is_empty() {
                    sf = s;
                }
            }
            Repeated::Unknown => {}
        }
    }
    if points.is_empty() {
        // otherwise the subresultant square-free part, only affordable when small
        if p.deg(x) > 8 || p.terms() > 400 {
            return gave_up(p);
        }
        let dp = p.derivative(x);
        let chain = subresultant_chain(p, &dp, x);
        let g = chain.last().cloned().unwrap_or_else(|| MPoly::one(n));
        if g.deg(x) >= 1 {
            let d = p.deg(x) - g.deg(x) + 1;
            let lg = g.lc(x).pow(d);
            if let Some(s2) = p.mul(&lg).div_exact(&g) {
                // undo the content the pseudo-division brought in
                let mut s2 = s2;
                for _ in 0..d {
                    match s2.div_exact(&g.lc(x)) {
                        Some(h) if !h.is_zero() && !g.lc(x).is_const() => s2 = h,
                        _ => break,
                    }
                }
                sf = strip_monomial(&s2);
            }
        }
        if sf.deg(x) <= 1 {
            return vec![sf];
        }
        points = pick_point(&sf, 3);
    }
    // the image with the fewest factors limits extraneous splitting; an
    // irreducible image proves irreducibility
    let mut best: Option<(Vec<Q>, Vec<UPoly>)> = None;
    for (pt, image) in points {
        let fs = ufactor(&image);
        if fs.len() <= 1 {
            return vec![sf];
        }
        if best.as_ref().is_none_or(|b| fs.len() < b.1.len()) {
            best = Some((pt, fs));
        }
    }
    let Some((a, ufs)) = best else { return gave_up(&sf) };
    if ufs.len() <= 1 {
        return vec![sf];
    }
    // irreducible pieces of the leading coefficient, to strip content
    let lc = sf.lc(x);
    let lc_parts: Vec<MPoly> = if lc.is_const() {
        vec![]
    } else {
        match (0..n).find(|&v| lc.deg(v) >= 1) {
            Some(v) => {
                let mut parts = factor_main(&lc, v);
                parts.retain(|f| !f.is_const());
                if parts.is_empty() { vec![lc.clone()] } else { parts }
            }
            None => vec![lc.clone()],
        }
    };
    if crate::expr::past_deadline() {
        return gave_up(&sf);
    }
    match lift_and_recombine(&sf, &lc, &lc_parts, x, &params, &a, &ufs) {
        Some(fs) => fs,
        None => gave_up(&sf),
    }
}

/// Whether the polynomials `cs` may have a common non-constant factor (a
/// factor of `small`, one of them). A common factor depending on w survives
/// in the images where all other variables are set to random numbers, so a
/// constant gcd of those images for every variable w of `small` rules it out.
fn may_share_factor(cs: &[MPoly], small: &MPoly) -> bool {
    let n = small.n;
    let vars: Vec<usize> = (0..n).filter(|&v| small.uses(v)).collect();
    let mut seed: u64 = 0xD1B5_4A32_D192_ED03;
    for &w in &vars {
        let others: Vec<usize> = (0..n).filter(|&v| v != w && cs.iter().any(|c| c.uses(v))).collect();
        let vals: Vec<Q> = others
            .iter()
            .map(|_| {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                q(2 + ((seed >> 33) % 97) as i64)
            })
            .collect();
        let mut g: Option<UPoly> = None;
        for c in cs {
            let mut u = c.clone();
            for (j, &v) in others.iter().enumerate() {
                u = u.subst_const(v, &vals[j]);
            }
            let Some(uu) = to_upoly(&u, w) else { return true };
            g = Some(match g {
                None => uu,
                Some(prev) => ugcd(&prev, &uu),
            });
            if g.as_ref().is_some_and(|g| udeg(g) == 0) {
                break;
            }
        }
        if g.is_some_and(|g| udeg(&g) > 0) {
            return true;
        }
    }
    false
}

enum Repeated {
    /// Points (with their images) where p is square-free: it has no repeated factor.
    None(Vec<(Vec<Q>, UPoly)>),
    /// The square-free part of p.
    Part(MPoly),
    Unknown,
}

/// Repeated factors of p in x, at points with random non-zero coordinates
/// (sparse points can all be degenerate). With p = s²t, ∂p/∂x = s·(2s′t + st′),
/// and at a generic point the images of s = gcd(p, ∂p/∂x) and of the cofactor
/// are coprime, so ∂p/∂x lifts as their product (EEZ-GCD), giving the
/// square-free part p / s.
fn repeated_part(p: &MPoly, x: usize, params: &[usize]) -> Repeated {
    match sqfree_by_lifting(p, x, params) {
        Ok(s) => Repeated::Part(s),
        Err(pts) if !pts.is_empty() => Repeated::None(pts),
        Err(_) => Repeated::Unknown,
    }
}

fn sqfree_by_lifting(p: &MPoly, x: usize, params: &[usize]) -> Result<MPoly, Vec<(Vec<Q>, UPoly)>> {
    let n = p.n;
    let d = p.derivative(x);
    let (dp, dd) = (p.deg(x) as usize, d.deg(x) as usize);
    let image = |f: &MPoly, a: &[Q]| {
        let mut u = f.clone();
        for (j, &v) in params.iter().enumerate() {
            u = u.subst_const(v, &a[j]);
        }
        to_upoly(&u, x)
    };
    // the point with the smallest gcd: larger ones are accidents of the point
    let mut best: Option<(Vec<Q>, UPoly, UPoly)> = None;
    let mut square_free: Vec<(Vec<Q>, UPoly)> = Vec::new();
    let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
    for attempt in 0..12usize {
        if crate::expr::past_deadline() {
            return Err(square_free);
        }
        let a: Vec<Q> = params
            .iter()
            .map(|_| {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let h = seed >> 33;
                let v = 1 + (h % (3 + attempt as u64)) as i64;
                q(if (h >> 20) % 2 == 0 { v } else { -v })
            })
            .collect();
        let (Some(u), Some(ud)) = (image(p, &a), image(&d, &a)) else { continue };
        if udeg(&u) != dp || udeg(&ud) != dd {
            continue;
        }
        let g = ugcd(&u, &uderiv(&u));
        if udeg(&g) == 0 {
            // square-free here: p has no repeated factor in x
            square_free.push((a, u));
            if square_free.len() >= 3 {
                return Err(square_free);
            }
            continue;
        }
        if !square_free.is_empty() {
            continue;
        }
        let h = umonic(&udivrem(&ud, &g).0);
        if udeg(&ugcd(&g, &h)) > 0 {
            continue;
        }
        if best.as_ref().is_none_or(|b| udeg(&g) < udeg(&b.1)) {
            best = Some((a, g, h));
        }
    }
    if !square_free.is_empty() {
        return Err(square_free);
    }
    let Some((a, g, h)) = best else { return Err(vec![]) };
    let lc = d.lc(x);
    let lc_parts: Vec<MPoly> = if lc.is_const() {
        vec![]
    } else {
        match (0..n).find(|&v| lc.deg(v) >= 1) {
            Some(v) => factor_main(&lc, v).into_iter().filter(|f| !f.is_const()).collect(),
            None => vec![lc.clone()],
        }
    };
    let fs = lift_and_recombine(&d, &lc, &lc_parts, x, params, &a, &[g.clone(), h]).ok_or_else(Vec::new)?;
    // the lifted factor with the gcd's degree, without content p does not have
    let mut gl = fs.into_iter().find(|f| f.deg(x) as usize == udeg(&g)).ok_or_else(Vec::new)?;
    for part in &lc_parts {
        if p.div_exact(&gl).is_some() {
            break;
        }
        gl = strip_factor(&gl, part);
    }
    let s = p.div_exact(&gl).ok_or_else(Vec::new)?;
    Ok(strip_monomial(&s))
}

thread_local! {
    /// Set when a factorization attempt stopped without a result (as opposed
    /// to proving the polynomial irreducible).
    static GAVE_UP: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn gave_up(p: &MPoly) -> Vec<MPoly> {
    GAVE_UP.with(|g| g.set(true));
    vec![p.clone()]
}

// ---------------------------------------------------------------- modular Hensel lifting

const PRIMES: [u64; 4] = [4611686018427387847, 4611686018427387817, 4611686018427387787, 4611686018427387761];

fn mulm(a: u64, b: u64, p: u64) -> u64 {
    ((a as u128 * b as u128) % p as u128) as u64
}
fn addm(a: u64, b: u64, p: u64) -> u64 {
    ((a as u128 + b as u128) % p as u128) as u64
}
fn negm(a: u64, p: u64) -> u64 {
    if a == 0 { 0 } else { p - a }
}
fn powm(mut a: u64, mut e: u64, p: u64) -> u64 {
    let mut r = 1u64;
    while e > 0 {
        if e & 1 == 1 {
            r = mulm(r, a, p);
        }
        a = mulm(a, a, p);
        e >>= 1;
    }
    r
}
fn invm(a: u64, p: u64) -> u64 {
    powm(a, p - 2, p)
}
fn qmod(c: &Q, p: u64) -> Option<u64> {
    use num_integer::Integer;
    let pb = BigInt::from(p);
    let nm = c.numer().mod_floor(&pb).to_u64()?;
    let dm = c.denom().mod_floor(&pb).to_u64()?;
    if dm == 0 { None } else { Some(mulm(nm, invm(dm, p), p)) }
}

/// Exponent vectors packed 8 bits per variable, variable 0 in the highest
/// byte, so the numeric order of keys is the lexicographic monomial order and
/// a monomial product is a key sum.
type Key = u128;
const KMAX: usize = 16;
fn kshift(i: usize) -> u32 {
    8 * (KMAX - 1 - i) as u32
}
fn kexp(k: Key, i: usize) -> u32 {
    ((k >> kshift(i)) & 255) as u32
}
fn pack(m: &Mono) -> Key {
    m.iter().enumerate().fold(0, |k, (i, e)| k | ((*e as Key) << kshift(i)))
}
fn unpack(k: Key, n: usize) -> Mono {
    (0..n).map(|i| kexp(k, i) as u16).collect()
}
fn kdivides(a: Key, b: Key, n: usize) -> bool {
    (0..n).all(|i| kexp(a, i) <= kexp(b, i))
}

/// Sparse polynomial with coefficients modulo a prime.
#[derive(Clone, PartialEq, Debug)]
struct PP {
    t: BTreeMap<Key, u64>,
}

impl PP {
    fn zero() -> PP {
        PP { t: BTreeMap::new() }
    }
    fn constant(c: u64) -> PP {
        let mut r = PP::zero();
        if c != 0 {
            r.t.insert(0, c);
        }
        r
    }
    fn from_q(p: &MPoly, pr: u64) -> Option<PP> {
        let mut r = PP::zero();
        for (m, c) in &p.t {
            if m.iter().any(|e| *e > 120) {
                return None;
            }
            let v = qmod(c, pr)?;
            if v != 0 {
                r.t.insert(pack(m), v);
            }
        }
        Some(r)
    }
    fn is_zero(&self) -> bool {
        self.t.is_empty()
    }
    fn add_term(&mut self, k: Key, c: u64, pr: u64) {
        crate::expr::charge(1);
        if c == 0 {
            return;
        }
        let e = self.t.entry(k).or_insert(0);
        *e = addm(*e, c, pr);
        if *e == 0 {
            self.t.remove(&k);
        }
    }
    fn add(&self, o: &PP, pr: u64) -> PP {
        let mut r = self.clone();
        for (k, c) in &o.t {
            r.add_term(*k, *c, pr);
        }
        r
    }
    fn sub(&self, o: &PP, pr: u64) -> PP {
        let mut r = self.clone();
        for (k, c) in &o.t {
            r.add_term(*k, negm(*c, pr), pr);
        }
        r
    }
    fn scale(&self, c: u64, pr: u64) -> PP {
        PP { t: self.t.iter().map(|(k, v)| (*k, mulm(*v, c, pr))).filter(|(_, v)| *v != 0).collect() }
    }
    /// Product, keeping only terms of degree at most `d` in `vars` when given.
    fn mul_t(&self, o: &PP, pr: u64, trunc: Option<(&[usize], u32)>) -> PP {
        let deg = |k: Key| trunc.map_or(0, |(vs, _)| vs.iter().map(|&v| kexp(k, v)).sum::<u32>());
        let lim = trunc.map_or(u32::MAX, |(_, d)| d);
        let bs: Vec<(Key, u64, u32)> = o.t.iter().map(|(k, c)| (*k, *c, deg(*k))).collect();
        let mut acc: std::collections::HashMap<Key, u128> = std::collections::HashMap::new();
        let mut work = 0usize;
        let p2 = (pr as u128) * (pr as u128);
        for (ka, ca) in &self.t {
            let da = deg(*ka);
            if da > lim {
                continue;
            }
            work += bs.len();
            if work > 1 << 16 {
                work = 0;
                if crate::expr::past_deadline() {
                    break;
                }
            }
            for (kb, cb, db) in &bs {
                if da + db > lim {
                    continue;
                }
                let e = acc.entry(ka + kb).or_insert(0);
                *e += *ca as u128 * *cb as u128;
                if *e >= p2 {
                    *e -= p2;
                }
            }
        }
        PP { t: acc.into_iter().map(|(k, v)| (k, (v % pr as u128) as u64)).filter(|(_, v)| *v != 0).collect() }
    }
    fn mul(&self, o: &PP, pr: u64) -> PP {
        self.mul_t(o, pr, None)
    }
    /// x_v replaced by x_v + a.
    fn shift(&self, v: usize, a: u64, pr: u64) -> PP {
        if a == 0 {
            return self.clone();
        }
        let mut r = PP::zero();
        for (k, c) in &self.t {
            let e = kexp(*k, v);
            let base = k & !((255 as Key) << kshift(v));
            // sum_j C(e, j) a^(e-j) x^j
            let mut binom = 1u64;
            let mut apow = vec![1u64; e as usize + 1];
            for i in 1..=e as usize {
                apow[i] = mulm(apow[i - 1], a, pr);
            }
            for j in 0..=e {
                let coef = mulm(mulm(binom, apow[(e - j) as usize], pr), *c, pr);
                r.add_term(base | ((j as Key) << kshift(v)), coef, pr);
                binom = mulm(mulm(binom, (e - j) as u64, pr), invm(j as u64 + 1, pr), pr);
            }
        }
        r
    }
    fn div_exact(&self, b: &PP, pr: u64, n: usize) -> Option<PP> {
        let (&bk, &bc) = b.t.iter().next_back()?;
        let binv = invm(bc, pr);
        if self.is_zero() {
            return Some(PP::zero());
        }
        let degs = |p: &PP| (0..n).map(|i| p.t.keys().map(|k| kexp(*k, i)).max().unwrap_or(0) as i64).collect::<Vec<i64>>();
        let maxq: Vec<i64> = degs(self).iter().zip(degs(b)).map(|(a, b)| a - b).collect();
        if maxq.iter().any(|d| *d < 0) {
            return None;
        }
        let mut r = self.clone();
        let mut out = PP::zero();
        let mut guard = 0usize;
        while let Some((&k, &c)) = r.t.iter().next_back() {
            guard += 1;
            if guard > 200_000 || (guard % 256 == 0 && crate::expr::past_deadline()) {
                return None;
            }
            if !kdivides(bk, k, n) {
                return None;
            }
            let qk = k - bk;
            if (0..n).any(|i| kexp(qk, i) as i64 > maxq[i]) {
                return None;
            }
            let qc = mulm(c, binv, pr);
            for (k2, c2) in &b.t {
                r.add_term(qk + k2, negm(mulm(qc, *c2, pr), pr), pr);
            }
            out.add_term(qk, qc, pr);
        }
        Some(out)
    }
}

// univariate polynomials modulo a prime, lowest coefficient first
fn um_trim(p: &mut Vec<u64>) {
    while p.last() == Some(&0) {
        p.pop();
    }
}
fn um_mul(a: &[u64], b: &[u64], pr: u64) -> Vec<u64> {
    if a.is_empty() || b.is_empty() {
        return vec![];
    }
    let mut r = vec![0u64; a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            r[i + j] = addm(r[i + j], mulm(*x, *y, pr), pr);
        }
    }
    um_trim(&mut r);
    r
}
fn um_sub(a: &[u64], b: &[u64], pr: u64) -> Vec<u64> {
    let mut r = vec![0u64; a.len().max(b.len())];
    for (i, x) in a.iter().enumerate() {
        r[i] = *x;
    }
    for (i, y) in b.iter().enumerate() {
        r[i] = addm(r[i], negm(*y, pr), pr);
    }
    um_trim(&mut r);
    r
}
fn um_divrem(a: &[u64], b: &[u64], pr: u64) -> (Vec<u64>, Vec<u64>) {
    let mut r = a.to_vec();
    um_trim(&mut r);
    let db = b.len() - 1;
    let linv = invm(b[db], pr);
    if r.len() < b.len() {
        return (vec![], r);
    }
    let mut qv = vec![0u64; r.len() - b.len() + 1];
    while r.len() >= b.len() {
        let k = r.len() - b.len();
        let c = mulm(*r.last().unwrap(), linv, pr);
        for (j, y) in b.iter().enumerate() {
            r[k + j] = addm(r[k + j], negm(mulm(c, *y, pr), pr), pr);
        }
        qv[k] = c;
        r.pop();
        um_trim(&mut r);
    }
    um_trim(&mut qv);
    (qv, r)
}
fn um_invmod(a: &[u64], m: &[u64], pr: u64) -> Option<Vec<u64>> {
    let (mut r0, mut r1) = (m.to_vec(), um_divrem(a, m, pr).1);
    let (mut t0, mut t1): (Vec<u64>, Vec<u64>) = (vec![], vec![1]);
    while !r1.is_empty() {
        let (qq, r2) = um_divrem(&r0, &r1, pr);
        let t2 = um_sub(&t0, &um_mul(&qq, &t1, pr), pr);
        r0 = r1;
        r1 = r2;
        t0 = t1;
        t1 = t2;
    }
    if r0.len() != 1 {
        return None;
    }
    let inv = invm(r0[0], pr);
    let t: Vec<u64> = t0.iter().map(|c| mulm(*c, inv, pr)).collect();
    Some(um_divrem(&t, m, pr).1)
}

fn um_gcd(a: &[u64], b: &[u64], pr: u64) -> Vec<u64> {
    let (mut a, mut b) = (a.to_vec(), b.to_vec());
    um_trim(&mut a);
    um_trim(&mut b);
    while !b.is_empty() {
        let r = um_divrem(&a, &b, pr).1;
        a = b;
        b = r;
    }
    a
}

/// The univariate image of p in x at params = a, modulo a prime.
fn image_mod(p: &MPoly, x: usize, params: &[usize], a: &[Q], pr: u64) -> Option<Vec<u64>> {
    let am: Vec<u64> = a.iter().map(|v| qmod(v, pr)).collect::<Option<_>>()?;
    let mut out = vec![0u64; p.deg(x) as usize + 1];
    for (m, c) in &p.t {
        let mut v = qmod(c, pr)?;
        for (j, &w) in params.iter().enumerate() {
            if m[w] > 0 {
                v = mulm(v, powm(am[j], m[w] as u64, pr), pr);
            }
        }
        let k = m[x] as usize;
        out[k] = addm(out[k], v, pr);
    }
    um_trim(&mut out);
    Some(out)
}

/// One prime's lift: factors F_j (parameters shifted to the evaluation point,
/// leading coefficient lc imposed) of lc^(r-1) sf, and ways to form the
/// candidate factor of sf for a subset of them.
struct ModLift {
    pr: u64,
    lifted: Vec<PP>,
    ls: PP,
    inv_ls: PP,
    bound: u32,
    parts: Vec<PP>,
}

fn lift_mod(sf: &MPoly, lc: &MPoly, lc_parts: &[MPoly], x: usize, params: &[usize], a: &[Q], ufs: &[UPoly], pr: u64) -> Option<ModLift> {
    let n = sf.n;
    if n > KMAX {
        return None;
    }
    let r = ufs.len();
    let am: Vec<u64> = a.iter().map(|v| qmod(v, pr)).collect::<Option<_>>()?;
    let shift_all = |p: &PP| params.iter().enumerate().fold(p.clone(), |acc, (j, &v)| acc.shift(v, am[j], pr));
    let ss = shift_all(&PP::from_q(sf, pr)?);
    let ls = shift_all(&PP::from_q(lc, pr)?);
    let l0 = *ls.t.get(&0)?;
    let mut target = ss.clone();
    for _ in 1..r {
        target = target.mul(&ls, pr);
    }
    let pdeg = |k: Key| params.iter().map(|&v| kexp(k, v)).sum::<u32>();
    let bound = target.t.keys().map(|k| pdeg(*k)).max().unwrap_or(0);
    // base factors l0 f_j and the Diophantine inverses
    let base: Vec<Vec<u64>> = ufs
        .iter()
        .map(|f| f.iter().map(|c| qmod(c, pr).map(|v| mulm(v, l0, pr))).collect::<Option<Vec<u64>>>())
        .collect::<Option<_>>()?;
    if base.iter().any(|b| b.last() == Some(&0) || b.is_empty()) {
        return None;
    }
    let full = base.iter().fold(vec![1u64], |acc, f| um_mul(&acc, f, pr));
    let mut s: Vec<Vec<u64>> = Vec::new();
    for b in &base {
        let others = um_divrem(&full, b, pr).0;
        s.push(um_invmod(&others, b, pr)?);
    }
    let xkey = |d: usize| (d as Key) << kshift(x);
    let from_u = |u: &[u64]| PP { t: u.iter().enumerate().filter(|(_, c)| **c != 0).map(|(i, c)| (xkey(i), *c)).collect() };
    // homogeneous components in the parameters
    let nb = bound as usize + 1;
    let split = |p: &PP| {
        let mut out = vec![PP::zero(); nb];
        for (k, c) in &p.t {
            let d = pdeg(*k) as usize;
            if d < nb {
                out[d].t.insert(*k, *c);
            }
        }
        out
    };
    let tcomp = split(&target);
    let mut fc: Vec<Vec<PP>> = base
        .iter()
        .map(|b| {
            let d = b.len() - 1;
            let lower = from_u(&b[..d]);
            let lead = PP { t: [(xkey(d), 1u64)].into_iter().collect() };
            split(&lower.add(&ls.mul(&lead, pr), pr))
        })
        .collect();
    let mut chain: Vec<Vec<PP>> = vec![vec![PP::zero(); nb]; r];
    let chain_at = |chain: &mut Vec<Vec<PP>>, fc: &Vec<Vec<PP>>, m: usize| {
        chain[0][m] = fc[0][m].clone();
        for j in 1..r {
            let mut acc = PP::zero();
            for k in 0..=m {
                if chain[j - 1][k].is_zero() || fc[j][m - k].is_zero() {
                    continue;
                }
                acc = acc.add(&chain[j - 1][k].mul(&fc[j][m - k], pr), pr);
            }
            chain[j][m] = acc;
        }
    };
    chain_at(&mut chain, &fc, 0);
    if chain[r - 1][0] != tcomp[0] {
        return None;
    }
    for m in 1..nb {
        if crate::expr::past_deadline() {
            return None;
        }
        chain_at(&mut chain, &fc, m);
        let e = tcomp[m].sub(&chain[r - 1][m], pr);
        if e.is_zero() {
            continue;
        }
        let xmask: Key = !((255 as Key) << kshift(x));
        let mut groups: BTreeMap<Key, Vec<u64>> = BTreeMap::new();
        for (k, c) in &e.t {
            let up = groups.entry(k & xmask).or_default();
            let yd = kexp(*k, x) as usize;
            if up.len() <= yd {
                up.resize(yd + 1, 0);
            }
            up[yd] = addm(up[yd], *c, pr);
        }
        for (key, mut cy) in groups {
            um_trim(&mut cy);
            if cy.is_empty() {
                continue;
            }
            for j in 0..r {
                let sigma = um_divrem(&um_mul(&s[j], &cy, pr), &base[j], pr).1;
                if sigma.is_empty() {
                    continue;
                }
                let corr = PP { t: sigma.iter().enumerate().filter(|(_, c)| **c != 0).map(|(i, c)| (xkey(i) + key, *c)).collect() };
                fc[j][m] = fc[j][m].add(&corr, pr);
            }
        }
        chain_at(&mut chain, &fc, m);
    }
    let lifted: Vec<PP> = fc
        .iter()
        .map(|cs| {
            let mut g = PP::zero();
            for c in cs {
                g.t.extend(c.t.iter().map(|(k, v)| (*k, *v)));
            }
            g
        })
        .collect();
    // 1/ls as a series in the shifted parameters
    let dl = ls.sub(&PP::constant(l0), pr).scale(negm(invm(l0, pr), pr), pr);
    let mut inv_ls = PP::constant(invm(l0, pr));
    let mut term = inv_ls.clone();
    for _ in 0..bound {
        term = term.mul_t(&dl, pr, Some((params, bound)));
        if term.is_zero() {
            break;
        }
        inv_ls = inv_ls.add(&term, pr);
    }
    let parts: Vec<PP> = lc_parts.iter().map(|p| PP::from_q(p, pr)).collect::<Option<_>>()?;
    Some(ModLift { pr, lifted, ls, inv_ls, bound, parts })
}

impl ModLift {
    /// The candidate factor of sf from the lifted factors in `subset`:
    /// leading coefficient lc, parameters unshifted, content from lc removed,
    /// leading term scaled to 1.
    fn candidate(&self, subset: &[usize], params: &[usize], a: &[Q], n: usize) -> Option<PP> {
        let pr = self.pr;
        let tr = Some((params, self.bound));
        let mut g = self.ls.clone();
        for &j in subset {
            g = g.mul_t(&self.lifted[j].mul_t(&self.inv_ls, pr, tr), pr, tr);
        }
        for (j, &v) in params.iter().enumerate() {
            g = g.shift(v, negm(qmod(&a[j], pr)?, pr), pr);
        }
        for part in &self.parts {
            for _ in 0..64 {
                match g.div_exact(part, pr, n) {
                    Some(h) if !h.is_zero() => g = h,
                    _ => break,
                }
            }
        }
        let (_, &lead) = g.t.iter().next_back()?;
        Some(g.scale(invm(lead, pr), pr))
    }
}

/// Rational reconstruction of residues modulo several primes.
fn reconstruct(cands: &[(u64, &PP)], n: usize) -> Option<MPoly> {
    use num_integer::Integer;
    let mut keys: Vec<Key> = cands.iter().flat_map(|(_, p)| p.t.keys().copied()).collect();
    keys.sort();
    keys.dedup();
    let mut m = BigInt::one();
    for (p, _) in cands {
        m *= BigInt::from(*p);
    }
    let bound = (&m / BigInt::from(2)).sqrt();
    let mut out = MPoly::zero(n);
    for k in keys {
        // CRT
        let mut x = BigInt::zero();
        let mut mm = BigInt::one();
        for (p, poly) in cands {
            let pb = BigInt::from(*p);
            let r = BigInt::from(*poly.t.get(&k).unwrap_or(&0));
            let diff = (&r - &x).mod_floor(&pb);
            let inv = BigInt::from(invm((&mm).mod_floor(&pb).to_u64()?, *p));
            x += &mm * ((diff * inv).mod_floor(&pb));
            mm *= pb;
        }
        // half extended Euclid
        let (mut r0, mut r1) = (m.clone(), x);
        let (mut t0, mut t1) = (BigInt::zero(), BigInt::one());
        while r1 > bound {
            let qq = &r0 / &r1;
            let r2 = &r0 - &qq * &r1;
            r0 = std::mem::replace(&mut r1, r2);
            let t2 = &t0 - &qq * &t1;
            t0 = std::mem::replace(&mut t1, t2);
        }
        if t1.abs() > bound || t1.is_zero() || !r1.gcd(&t1).is_one() {
            return None;
        }
        out.add_term(unpack(k, n), Q::new(r1, t1));
    }
    Some(out)
}

/// Lifts the image factorization modulo primes and recombines: subsets of
/// lifted factors whose reconstructed product divides sf exactly.
fn lift_and_recombine(sf: &MPoly, lc: &MPoly, lc_parts: &[MPoly], x: usize, params: &[usize], a: &[Q], ufs: &[UPoly]) -> Option<Vec<MPoly>> {
    let n = sf.n;
    let r = ufs.len();
    let mut lifts: Vec<ModLift> = Vec::new();
    for &pr in &PRIMES {
        if crate::expr::past_deadline() {
            return None;
        }
        let Some(l) = lift_mod(sf, lc, lc_parts, x, params, a, ufs, pr) else { continue };
        lifts.push(l);
        // recombination with the primes so far
        let mut left: Vec<usize> = (0..r).collect();
        let mut rem = sf.clone();
        let mut found: Vec<MPoly> = Vec::new();
        let mut size = 1;
        let mut failed = false;
        // a candidate that does not reconstruct may need more primes; one that
        // reconstructs but does not divide is simply not a factor
        let mut unrecovered = false;
        while size <= left.len() / 2 || (size == 1 && left.len() == 1) {
            if left.len() == 1 {
                break;
            }
            let k = left.len();
            let mut idx: Vec<usize> = (0..size).collect();
            let mut hit: Option<(Vec<usize>, MPoly)> = None;
            loop {
                if crate::expr::past_deadline() {
                    return None;
                }
                let subset: Vec<usize> = idx.iter().map(|&i| left[i]).collect();
                let cands: Option<Vec<PP>> = lifts.iter().map(|l| l.candidate(&subset, params, a, n)).collect();
                if let Some(cands) = cands {
                    let pairs: Vec<(u64, &PP)> = lifts.iter().map(|l| l.pr).zip(cands.iter()).collect();
                    match reconstruct(&pairs, n) {
                        Some(g) => {
                            if g.deg(x) >= 1 {
                                if let Some(qt) = rem.div_exact(&g) {
                                    hit = Some((subset, g));
                                    rem = qt;
                                }
                            }
                        }
                        None => unrecovered = true,
                    }
                }
                if hit.is_some() {
                    break;
                }
                // next combination
                let mut i = size;
                let mut done = true;
                while i > 0 {
                    i -= 1;
                    if idx[i] < k - size + i {
                        idx[i] += 1;
                        for j in i + 1..size {
                            idx[j] = idx[j - 1] + 1;
                        }
                        done = false;
                        break;
                    }
                }
                if done {
                    break;
                }
            }
            match hit {
                Some((subset, g)) => {
                    found.push(g);
                    left.retain(|i| !subset.contains(i));
                }
                None => {
                    if size == 1 && found.is_empty() && unrecovered {
                        // nothing reconstructs yet: more primes may help
                        failed = true;
                        break;
                    }
                    size += 1;
                }
            }
        }
        if failed {
            continue;
        }
        if found.is_empty() {
            // the image factors do not combine into any proper factor: irreducible
            return Some(vec![sf.clone()]);
        }
        if left.len() == 1 {
            let cands: Option<Vec<PP>> = lifts.iter().map(|l| l.candidate(&left, params, a, n)).collect();
            if let Some(cands) = cands {
                let pairs: Vec<(u64, &PP)> = lifts.iter().map(|l| l.pr).zip(cands.iter()).collect();
                if let Some(g) = reconstruct(&pairs, n).filter(|g| g.deg(x) >= 1) {
                    if let Some(qt) = rem.div_exact(&g) {
                        found.push(g);
                        rem = qt;
                    }
                }
            }
        }
        // what is left: the last factor, or content free of x
        if !rem.is_const() {
            match (0..n).filter(|&v| rem.deg(v) >= 1).max_by_key(|&v| (v == x, std::cmp::Reverse(rem.deg(v)))) {
                Some(v) if rem.deg(x) == 0 => found.extend(factor_main(&rem, v)),
                _ => found.push(rem),
            }
        }
        return Some(found);
    }
    None
}

/// Square root of p: Some(Some(r)) with r² = p, Some(None) when p is not a
/// square, None when the search stopped (deadline). Terms of r come from the
/// lexicographic leading terms of the remainder, as in long division.
fn poly_sqrt(p: &MPoly) -> Option<Option<MPoly>> {
    if p.is_zero() {
        return Some(Some(MPoly::zero(p.n)));
    }
    let ratio_sqrt = |c: &Q| -> Option<Q> {
        if c.is_negative() {
            return None;
        }
        let (nn, dd) = (c.numer().sqrt(), c.denom().sqrt());
        (&nn * &nn == *c.numer() && &dd * &dd == *c.denom()).then(|| Q::new(nn, dd))
    };
    let maxdeg: Vec<u32> = (0..p.n).map(|i| p.deg(i)).collect();
    let (lm, lc) = p.lead().map(|(m, c)| (m.clone(), c.clone()))?;
    if lm.iter().any(|e| e % 2 == 1) {
        return Some(None);
    }
    let Some(r0) = ratio_sqrt(&lc) else { return Some(None) };
    let mut r = MPoly::zero(p.n);
    let m0: Mono = lm.iter().map(|e| e / 2).collect();
    r.t.insert(m0.clone(), r0.clone());
    let two_lead = (m0, r0 * q(2));
    let mut rem = p.sub(&r.mul(&r));
    let mut guard = 0usize;
    while let Some((m, c)) = rem.lead().map(|(m, c)| (m.clone(), c.clone())) {
        guard += 1;
        if guard > 20_000 || (guard % 64 == 0 && crate::expr::past_deadline()) {
            return None;
        }
        // next term t = lead(rem) / (2 lead(r))
        if m.iter().zip(&two_lead.0).any(|(a, b)| a < b) {
            return Some(None);
        }
        let tm: Mono = m.iter().zip(&two_lead.0).map(|(a, b)| a - b).collect();
        if tm.iter().zip(&maxdeg).any(|(e, d)| 2 * (*e as u32) > *d) {
            return Some(None);
        }
        let tc = &c / &two_lead.1;
        let mut t = MPoly::zero(p.n);
        t.t.insert(tm, tc);
        // rem −= 2 r t + t²
        rem = rem.sub(&r.mul(&t).scale(&q(2))).sub(&t.mul(&t));
        r = r.add(&t);
    }
    Some(Some(r))
}

/// p without the largest monomial dividing all its terms.
fn strip_monomial(p: &MPoly) -> MPoly {
    let Some(lo) = p.t.keys().cloned().reduce(|a, b| a.iter().zip(&b).map(|(u, v)| *u.min(v)).collect()) else { return p.clone() };
    MPoly { n: p.n, t: p.t.iter().map(|(m, c)| (m.iter().zip(&lo).map(|(a, b)| a - b).collect(), c.clone())).collect() }
}


pub fn abs_q(x: &Q) -> Q {
    x.abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn x(n: usize, i: usize) -> MPoly {
        MPoly::var(n, i)
    }
    fn c(n: usize, v: i64) -> MPoly {
        MPoly::constant(n, q(v))
    }

    #[test]
    fn univariate_factor() {
        // (x^2 - 2)(x - 3)(x^2 + x + 1)
        let n = 1;
        let p = x(n, 0).mul(&x(n, 0)).sub(&c(n, 2)).mul(&x(n, 0).sub(&c(n, 3))).mul(&x(n, 0).mul(&x(n, 0)).add(&x(n, 0)).add(&c(n, 1)));
        let f = factor_main(&p, 0);
        let degs: Vec<u32> = f.iter().map(|g| g.deg(0)).collect();
        assert_eq!(f.len(), 3, "{degs:?}");
    }

    #[test]
    fn bivariate_factor() {
        // (x^2 + r x - r^2)(x - 2r)(x + r + 1): factors with a parameter r
        let n = 2;
        let (xx, r) = (x(n, 0), x(n, 1));
        let f1 = xx.mul(&xx).add(&r.mul(&xx)).sub(&r.mul(&r));
        let f2 = xx.sub(&r.scale(&q(2)));
        let f3 = xx.add(&r).add(&c(n, 1));
        let p = f1.mul(&f2).mul(&f3);
        let f = factor_main(&p, 0);
        let mut degs: Vec<u32> = f.iter().map(|g| g.deg(0)).collect();
        degs.sort();
        assert_eq!(degs, vec![1, 1, 2]);
        // each factor divides p
        for g in &f {
            assert!(p.div_exact(g).is_some() || p.scale(&q(1)).div_exact(&g.scale(&q(1))).is_some());
        }
    }

    #[test]
    fn non_monic_factor() {
        // (r x - 1)(x^2 - r) with leading coefficient depending on r
        let n = 2;
        let (xx, r) = (x(n, 0), x(n, 1));
        let p = r.mul(&xx).sub(&c(n, 1)).mul(&xx.mul(&xx).sub(&r));
        let f = factor_main(&p, 0);
        let mut degs: Vec<u32> = f.iter().map(|g| g.deg(0)).collect();
        degs.sort();
        assert_eq!(degs, vec![1, 2], "{f:?}");
        // the roots of each factor are roots of p
        let vals = [1.0 / 1.7, 1.7];
        let lin = f.iter().find(|g| g.deg(0) == 1).unwrap();
        assert!(lin.vanishes(&vals));
    }

    #[test]
    fn two_parameter_factor() {
        // (x^2 + r s x - s^2)(x^3 - r x + s)(x - r - s)
        let n = 3;
        let (xx, r, s) = (x(n, 0), x(n, 1), x(n, 2));
        let f1 = xx.mul(&xx).add(&r.mul(&s).mul(&xx)).sub(&s.mul(&s));
        let f2 = xx.pow(3).sub(&r.mul(&xx)).add(&s);
        let f3 = xx.sub(&r).sub(&s);
        let p = f1.mul(&f2).mul(&f3);
        let t0 = web_time::Instant::now();
        let f = factor_main(&p, 0);
        let mut degs: Vec<u32> = f.iter().map(|g| g.deg(0)).collect();
        degs.sort();
        println!("{degs:?} in {:?}", t0.elapsed());
        assert_eq!(degs, vec![1, 2, 3]);
    }

    /// Reads a polynomial dumped by the derivation (GXPAD_DUMP) from the file
    /// named by GXPAD_FACTOR_FILE and factors it in each variable (or in
    /// GXPAD_MAIN only), for profiling.
    #[test]
    #[ignore]
    fn factor_dump() {
        let Ok(path) = std::env::var("GXPAD_FACTOR_FILE") else { return };
        let text = std::fs::read_to_string(path).unwrap();
        let mut lines = text.lines();
        let head: Vec<usize> = lines.next().unwrap().split(' ').map(|t| t.parse().unwrap()).collect();
        let (n, xv) = (head[0], head[1]);
        let mut p = MPoly::zero(n);
        let mut vals = vec![];
        for l in lines {
            let mut it = l.split(' ');
            let c = it.next().unwrap();
            if c == "vals" {
                vals = it.map(|t| t.parse::<f64>().unwrap()).collect();
                continue;
            }
            let c: Q = c.parse().unwrap();
            let m: Mono = it.map(|t| t.parse().unwrap()).collect();
            p.add_term(m, c);
        }
        let all: Vec<usize> = (0..n).filter(|&v| p.uses(v)).collect();
        let degs: Vec<u32> = all.iter().map(|&v| p.deg(v)).collect();
        println!("{} terms, vars {all:?} degrees {degs:?}, solved variable v{xv}", p.terms());
        let mains: Vec<usize> = match std::env::var("GXPAD_MAIN").as_deref() {
            Ok("cheapest") => vec![cheapest_main(&p, &all).unwrap()],
            Ok(v) => vec![v.parse().unwrap()],
            Err(_) => all.clone(),
        };
        for &main in &mains {
            let t0 = web_time::Instant::now();
            crate::expr::set_deadline(Some(t0 + std::time::Duration::from_secs(10)));
            let f = factor_main(&p, main);
            println!("factored in v{main} in {:?}", t0.elapsed());
            for g in &f {
                println!("  factor: {} terms, deg in v{xv} {}, vanishes {}, divides {}", g.terms(), g.deg(xv), g.vanishes(&vals), p.div_exact(g).is_some());
            }
        }
        crate::expr::set_deadline(None);
    }

    #[test]
    fn resultant_circle_line() {
        // circle x^2 + y^2 - 1 and line y - x: eliminating y gives 2x^2 - 1
        let n = 2;
        let (xx, yy) = (x(n, 0), x(n, 1));
        let a = xx.mul(&xx).add(&yy.mul(&yy)).sub(&c(n, 1));
        let b = yy.sub(&xx);
        let r = resultant(&a, &b, 1).unwrap();
        let expect = xx.mul(&xx).scale(&q(2)).sub(&c(n, 1));
        assert!(r == expect || r == expect.scale(&q(-1)), "{r:?}");
    }
}

#[cfg(test)]
mod sqfree_tests {
    use super::*;

    fn v(n: usize, i: usize) -> MPoly {
        MPoly::var(n, i)
    }

    #[test]
    fn square_free_part_by_lifting() {
        // p = (x² + y·x + z + 1)² · (x + 2y)
        let n = 3;
        let (x, y, z) = (v(n, 0), v(n, 1), v(n, 2));
        let s = x.mul(&x).add(&y.mul(&x)).add(&z).add(&MPoly::one(n));
        let t = x.add(&y.scale(&q(2)));
        let p = s.mul(&s).mul(&t);
        let r = sqfree_by_lifting(&p, 0, &[1, 2]);
        let want = s.mul(&t);
        match r {
            Ok(sf) => assert!(sf.div_exact(&want).is_some_and(|c| c.is_const()), "got {} terms", sf.terms()),
            Err(pts) => panic!("no square-free part ({} square-free points)", pts.len()),
        }
    }

    #[test]
    fn square_free_part_by_lifting_non_monic() {
        // p = ((y + 1)·x² + z·x + 1)² · (y·x + 2)
        let n = 3;
        let (x, y, z) = (v(n, 0), v(n, 1), v(n, 2));
        let s = y.add(&MPoly::one(n)).mul(&x).mul(&x).add(&z.mul(&x)).add(&MPoly::one(n));
        let t = y.mul(&x).add(&MPoly::constant(n, q(2)));
        let p = s.mul(&s).mul(&t);
        let want = s.mul(&t);
        match sqfree_by_lifting(&p, 0, &[1, 2]) {
            Ok(sf) => assert!(sf.div_exact(&want).is_some_and(|c| c.is_const()), "got {} terms", sf.terms()),
            Err(pts) => panic!("no square-free part ({} square-free points)", pts.len()),
        }
    }
}
