//! Normal form of expressions with nested square roots.
//!
//! A formula built by substitution (a coordinate from one intersection put
//! into the next) is a tower of square roots over the field of rational
//! functions of the parameters. This module computes in that tower exactly:
//!
//! - each distinct square root is a generator g with g² = ρ, where ρ is a
//!   polynomial in the parameters and the earlier generators;
//! - an element is Σ p_S · Π_{i ∈ S} g_i over a common polynomial denominator
//!   kept in factored form, with p_S polynomials in the parameters: products
//!   are reduced with g² = ρ and denominators are rationalized by conjugates;
//! - a square root first loses its square factors (√(q²·h) = |q|·√h), is
//!   then looked up among the generators and their products (√(f·g) with
//!   √f and √g known), and is denested when it is a square in the field
//!   below (√(a + b·g) = √((a + d)/2) ± √((a − d)/2) with d² = a² − b²·ρ);
//!   only what remains becomes a new generator;
//! - sin and cos of sums and multiples of angles are expanded to cos β and
//!   sin β of base angles, with sin β a generator (sin² β = 1 − cos² β), and
//!   cos and sin of acos, asin, atan and atan2 are taken algebraically.
//!
//! With independent generators the form is canonical: two expressions for
//! the same function come out equal, and identities reduce to zero without
//! numeric tests. Signs (|q|, the branch of a denested root) are taken at
//! the current configuration, like every branch choice of the derivation.

use crate::alg::{MPoly, cheapest_main, factor_main, q};
use crate::expr::*;
use num_bigint::BigInt;
use num_integer::Integer;
use num_rational::BigRational as Q;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Ring element: generator set (bit mask) → polynomial coefficient.
type Rn = BTreeMap<u32, MPoly>;
/// Factored denominator: normalized polynomials with multiplicities.
type Den = Vec<(MPoly, u32)>;

#[derive(Clone, Debug)]
struct El {
    n: Rn,
    d: Den,
}

/// A prime of a square-free radicand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Tok {
    Neg,
    Prime(u64),
    /// Index into `Ctx::irreds`.
    Poly(usize),
}
type Sig = BTreeSet<Tok>;

struct Gen {
    rho: Rn,
    /// Value at the current configuration.
    val: f64,
    /// How the generator is written when it is not √ρ (sin β).
    show: Option<Expr>,
    /// The primes of a radicand without generators.
    sig: Option<Sig>,
}

/// Most generators (a bit of the mask each).
const MAX_GENS: usize = 24;
thread_local! {
    /// Most polynomial terms an element may have.
    static MAX_TERMS_NOW: std::cell::Cell<usize> = const { std::cell::Cell::new(8000) };
}

struct Ctx<'a> {
    nvar: usize,
    atoms: Vec<Expr>,
    vals: Vec<f64>,
    gens: Vec<Gen>,
    env: &'a dyn Fn(u32) -> f64,
    /// Angle terms with the common denominator of their coefficients.
    angle_lcm: Vec<(Expr, i64)>,
    /// (cos β, sin β) of the base angles.
    angles: Vec<(Expr, (El, El))>,
    irreds: Vec<MPoly>,
    fcache: Vec<(MPoly, Den)>,
    /// Above 0, square roots that need a new generator fail instead.
    no_new: u32,
    gexprs: Vec<Option<Expr>>,
    /// Converted subexpressions by node address.
    memo: HashMap<*const Node, El>,
}

thread_local! {
    /// Set when a normal form failed because a square factor or a radicand
    /// is zero at the current configuration.
    static DEGENERATE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Whether a normal form on this thread has failed at a zero factor or
/// radicand since `reset_degenerate`: the configuration is a special one
/// (equal lengths, coinciding points) where signs cannot be read off, and
/// other formulas of the same construction will fail the same way.
pub(crate) fn degenerate_seen() -> bool {
    DEGENERATE.with(|c| c.get())
}

pub(crate) fn reset_degenerate() {
    DEGENERATE.with(|c| c.set(false));
}

fn fail_zero<T>(why: &str) -> Option<T> {
    DEGENERATE.with(|c| c.set(true));
    fail(why)
}

fn max_terms() -> usize {
    MAX_TERMS_NOW.with(|c| c.get())
}

/// A failure with its reason (shown with GXGEO_DEBUG).
fn fail<T>(why: &str) -> Option<T> {
    if std::env::var("GXGEO_DEBUG").is_ok() {
        eprintln!("  normal form fails: {why}");
    }
    None
}

/// p = c · pp with pp primitive with integer coefficients and a positive
/// leading coefficient.
fn norm_poly(p: &MPoly) -> (Q, MPoly) {
    if p.is_zero() {
        return (Q::zero(), p.clone());
    }
    let mut g = BigInt::zero();
    let mut l = BigInt::one();
    for c in p.t.values() {
        g = g.gcd(c.numer());
        l = l.lcm(c.denom());
    }
    let mut c = Q::new(g, l);
    if p.t.values().next_back().unwrap().is_negative() {
        c = -c;
    }
    let pp = p.scale(&c.recip());
    (c, pp)
}

/// |n| = t² · u with u square-free as far as trial division by small primes
/// tells: (t, the primes of u).
fn square_part(n: &BigInt) -> (BigInt, Vec<u64>) {
    let mut n = n.abs();
    let mut t = BigInt::one();
    let mut primes = Vec::new();
    let mut p = 2u64;
    while p < 2000 && BigInt::from(p * p) <= n {
        let bp = BigInt::from(p);
        let mut e = 0;
        while (&n % &bp).is_zero() {
            n /= &bp;
            e += 1;
        }
        for _ in 0..e / 2 {
            t *= &bp;
        }
        if e % 2 == 1 {
            primes.push(p);
        }
        p += if p == 2 { 1 } else { 2 };
    }
    if !n.is_one() {
        // what is left is a prime, or too large to split: a perfect square
        // is still recognized
        let r = n.sqrt();
        if &r * &r == n {
            t *= r;
        } else {
            match n.to_u64() {
                Some(v) => primes.push(v),
                None => primes.push(u64::MAX),
            }
        }
    }
    (t, primes)
}

fn den_merge(a: &Den, b: &Den) -> Den {
    let mut out = a.clone();
    for (f, e) in b {
        match out.iter_mut().find(|x| x.0 == *f) {
            Some(x) => x.1 += e,
            None => out.push((f.clone(), *e)),
        }
    }
    out
}

fn rn_terms(a: &Rn) -> usize {
    a.values().map(|p| p.terms()).sum()
}

fn rn_top(a: &Rn) -> Option<u32> {
    let all = a.keys().fold(0u32, |m, k| m | k);
    (all != 0).then(|| 31 - all.leading_zeros())
}

fn rn_add(a: &Rn, b: &Rn, minus: bool) -> Rn {
    let mut r = a.clone();
    for (m, p) in b {
        let s = match r.get(m) {
            Some(x) => if minus { x.sub(p) } else { x.add(p) },
            None => if minus { p.scale(&q(-1)) } else { p.clone() },
        };
        if s.is_zero() {
            r.remove(m);
        } else {
            r.insert(*m, s);
        }
    }
    r
}

fn rn_scale(a: &Rn, p: &MPoly) -> Rn {
    a.iter().map(|(m, x)| (*m, x.mul(p))).filter(|(_, x)| !x.is_zero()).collect()
}

fn rn_scale_q(a: &Rn, c: &Q) -> Rn {
    a.iter().map(|(m, x)| (*m, x.scale(c))).filter(|(_, x)| !x.is_zero()).collect()
}

/// a = u + v·g_k: (u, v) with the bit of k removed from v's masks.
fn rn_split(a: &Rn, k: u32) -> (Rn, Rn) {
    let bit = 1u32 << k;
    let mut u = Rn::new();
    let mut v = Rn::new();
    for (m, p) in a {
        if m & bit != 0 {
            v.insert(m & !bit, p.clone());
        } else {
            u.insert(*m, p.clone());
        }
    }
    (u, v)
}

fn rn_join(u: Rn, v: Rn, k: u32) -> Rn {
    let mut r = u;
    for (m, p) in v {
        r.insert(m | (1 << k), p);
    }
    r
}

impl<'a> Ctx<'a> {
    fn poly_const(&self, c: Q) -> MPoly {
        MPoly::constant(self.nvar, c)
    }
    fn el_poly(&self, p: MPoly) -> El {
        let mut n = Rn::new();
        if !p.is_zero() {
            n.insert(0, p);
        }
        El { n, d: vec![] }
    }
    fn el_q(&self, c: Q) -> El {
        self.el_poly(self.poly_const(c))
    }
    fn el_int(&self, i: i64) -> El {
        self.el_q(q(i))
    }

    fn atom(&mut self, e: &Expr) -> Option<usize> {
        if let Some(i) = self.atoms.iter().position(|a| a == e) {
            return Some(i);
        }
        if self.atoms.len() >= self.nvar {
            return fail("too many atoms");
        }
        let v = eval(e, self.env);
        if !v.is_finite() {
            return fail("an atom without a value");
        }
        self.vals[self.atoms.len()] = v;
        self.atoms.push(e.clone());
        Some(self.atoms.len() - 1)
    }
    fn atom_el(&mut self, e: &Expr) -> Option<El> {
        let i = self.atom(e)?;
        Some(self.el_poly(MPoly::var(self.nvar, i)))
    }

    // ------------------------------------------------------------ values

    /// Value and magnitude (sum of absolute term values).
    fn rn_val(&self, a: &Rn) -> (f64, f64) {
        let (mut s, mut mag) = (0.0, 0.0);
        for (m, p) in a {
            let (v, pm) = p.eval_f64(&self.vals);
            let g: f64 = (0..self.gens.len()).filter(|i| m & (1 << i) != 0).map(|i| self.gens[i].val).product();
            s += v * g;
            mag += pm * g.abs();
        }
        (s, mag)
    }
    fn val(&self, a: &El) -> f64 {
        let d: f64 = a.d.iter().map(|(f, e)| f.eval_f64(&self.vals).0.powi(*e as i32)).product();
        self.rn_val(&a.n).0 / d
    }
    /// The sign of a polynomial at the current configuration; None when it
    /// is (numerically) zero there.
    fn poly_sign(&self, p: &MPoly) -> Option<f64> {
        let (v, mag) = p.eval_f64(&self.vals);
        if v.is_finite() && v.abs() > 1e-9 * mag.max(1e-300) { Some(v.signum()) } else { fail_zero("a factor is zero here") }
    }

    // ------------------------------------------------------------ factoring

    fn merge_factor(out: &mut Den, f: MPoly, e: u32) {
        match out.iter_mut().find(|x| x.0 == f) {
            Some(x) => x.1 += e,
            None => out.push((f, e)),
        }
    }

    fn factor_prim(&self, pp: &MPoly, depth: u32) -> Den {
        let used: Vec<usize> = (0..pp.n).filter(|&v| pp.uses(v)).collect();
        let mut out = Den::new();
        if pp.terms() == 1 {
            let m = pp.t.keys().next().unwrap();
            for &v in &used {
                out.push((MPoly::var(pp.n, v), m[v] as u32));
            }
            return out;
        }
        if used.len() > 14 || pp.terms() > 3000 || depth > 3 || past_deadline() {
            return vec![(pp.clone(), 1)];
        }
        let Some(main) = cheapest_main(pp, &used) else { return vec![(pp.clone(), 1)] };
        let fs = factor_main(pp, main);
        let mut rest = pp.clone();
        for f in fs {
            if f.is_const() {
                continue;
            }
            let (_, f) = norm_poly(&f);
            let mut e = 0;
            while e < 64 {
                match rest.div_exact(&f) {
                    Some(h) if !h.is_zero() => {
                        rest = h;
                        e += 1;
                    }
                    _ => break,
                }
            }
            if e > 0 {
                Self::merge_factor(&mut out, f, e);
            }
        }
        if !rest.is_const() {
            let (_, r) = norm_poly(&rest);
            if r.terms() < pp.terms() {
                for (f, e) in self.factor_prim(&r, depth + 1) {
                    Self::merge_factor(&mut out, f, e);
                }
            } else {
                Self::merge_factor(&mut out, r, 1);
            }
        }
        out
    }

    /// p = c · Π f^e with normalized factors (irreducible as far as the
    /// factorization got).
    fn factor_full(&mut self, p: &MPoly) -> (Q, Den) {
        let (c, pp) = norm_poly(p);
        if pp.is_const() {
            return (c, vec![]);
        }
        if let Some(hit) = self.fcache.iter().find(|x| x.0 == pp) {
            return (c, hit.1.clone());
        }
        let fs = self.factor_prim(&pp, 0);
        self.fcache.push((pp, fs.clone()));
        (c, fs)
    }

    fn tok_poly(&mut self, f: &MPoly) -> Tok {
        let i = match self.irreds.iter().position(|x| x == f) {
            Some(i) => i,
            None => {
                self.irreds.push(f.clone());
                self.irreds.len() - 1
            }
        };
        Tok::Poly(i)
    }

    /// c = cs² · Π primes (· −1): (cs, the primes and the sign as tokens).
    fn split_q(c: &Q) -> (Q, Sig) {
        let mut sig = Sig::new();
        if c.is_negative() {
            sig.insert(Tok::Neg);
        }
        // a/b = a·b / b²
        let (t, primes) = square_part(&(c.numer() * c.denom()));
        for p in primes {
            // (a prime twice cannot happen; a large unsplit rest could only repeat itself)
            if !sig.insert(Tok::Prime(p)) {
                sig.remove(&Tok::Prime(p));
            }
        }
        (Q::new(t, c.denom().clone()), sig)
    }

    /// p = cs² · s² · Π sig for a polynomial without generators.
    fn sqfree_split(&mut self, p: &MPoly) -> (Q, MPoly, Sig) {
        let (c, fs) = self.factor_full(p);
        let (cs, mut sig) = Self::split_q(&c);
        let mut s = MPoly::one(self.nvar);
        for (f, e) in fs {
            for _ in 0..e / 2 {
                s = s.mul(&f);
            }
            if e % 2 == 1 {
                let t = self.tok_poly(&f);
                sig.insert(t);
            }
        }
        (cs, s, sig)
    }

    fn sig_poly(&self, sig: &Sig) -> MPoly {
        let mut p = MPoly::one(self.nvar);
        for t in sig {
            p = match t {
                Tok::Neg => p.scale(&q(-1)),
                Tok::Prime(v) => p.scale(&Q::from_integer(BigInt::from(*v))),
                Tok::Poly(i) => p.mul(&self.irreds[*i]),
            };
        }
        p
    }

    // ------------------------------------------------------------ ring

    fn rn_mul(&self, a: &Rn, b: &Rn) -> Rn {
        if a.is_empty() || b.is_empty() {
            return Rn::new();
        }
        let all = a.keys().chain(b.keys()).fold(0u32, |m, k| m | k);
        if all == 0 {
            let p = a[&0].mul(&b[&0]);
            let mut r = Rn::new();
            if !p.is_zero() {
                r.insert(0, p);
            }
            return r;
        }
        // generators of one side only just multiply the masks
        let (ma, mb) = (a.keys().fold(0u32, |m, k| m | k), b.keys().fold(0u32, |m, k| m | k));
        if ma & mb == 0 {
            let mut r = Rn::new();
            for (sa, pa) in a {
                for (sb, pb) in b {
                    let p = pa.mul(pb);
                    if !p.is_zero() {
                        r.insert(sa | sb, p);
                    }
                }
            }
            return r;
        }
        let k = 31 - (ma & mb).leading_zeros();
        let (au, av) = rn_split(a, k);
        let (bu, bv) = rn_split(b, k);
        let uu = self.rn_mul(&au, &bu);
        let vv = self.rn_mul(&self.rn_mul(&av, &bv), &self.gens[k as usize].rho);
        let uv = rn_add(&self.rn_mul(&au, &bv), &self.rn_mul(&av, &bu), false);
        rn_join(rn_add(&uu, &vv, false), uv, k)
    }

    /// (m, p) with a · m = p, p a polynomial: the conjugates' product.
    fn rn_inv(&self, a: &Rn) -> Option<(Rn, MPoly)> {
        if past_deadline() || rn_terms(a) > max_terms() {
            return fail("inverse too large");
        }
        let Some(k) = rn_top(a) else {
            let mut one = Rn::new();
            one.insert(0, MPoly::one(self.nvar));
            // (|p| − p and the like: zero on one side)
            return match a.get(&0) {
                Some(p) => Some((one, p.clone())),
                None => fail("division by a zero divisor"),
            };
        };
        let (u, v) = rn_split(a, k);
        let conj = rn_join(u.clone(), rn_scale_q(&v, &q(-1)), k);
        let norm = rn_add(&self.rn_mul(&u, &u), &self.rn_mul(&self.rn_mul(&v, &v), &self.gens[k as usize].rho), true);
        let (m, p) = self.rn_inv(&norm)?;
        Some((self.rn_mul(&conj, &m), p))
    }

    // ------------------------------------------------------------ field

    /// Cancels the denominator's factors that divide the whole numerator.
    fn cancel(&self, mut x: El) -> El {
        if x.n.is_empty() {
            x.d.clear();
            return x;
        }
        for i in 0..x.d.len() {
            while x.d[i].1 > 0 {
                let f = &x.d[i].0;
                let qs: Option<Vec<(u32, MPoly)>> = x.n.iter().map(|(m, p)| p.div_exact(f).map(|h| (*m, h))).collect();
                match qs {
                    Some(qs) => {
                        x.n = qs.into_iter().collect();
                        x.d[i].1 -= 1;
                    }
                    None => break,
                }
            }
        }
        x.d.retain(|f| f.1 > 0);
        x
    }

    fn add(&self, a: &El, b: &El, minus: bool) -> El {
        if a.d.is_empty() && b.d.is_empty() {
            return El { n: rn_add(&a.n, &b.n, minus), d: vec![] };
        }
        // least common denominator
        let mut l = a.d.clone();
        for (f, e) in &b.d {
            match l.iter_mut().find(|x| x.0 == *f) {
                Some(x) => x.1 = x.1.max(*e),
                None => l.push((f.clone(), *e)),
            }
        }
        let lift = |x: &El| {
            let mut m = MPoly::one(self.nvar);
            for (f, e) in &l {
                let have = x.d.iter().find(|y| y.0 == *f).map(|y| y.1).unwrap_or(0);
                for _ in have..*e {
                    m = m.mul(f);
                }
            }
            if m.is_const() { x.n.clone() } else { rn_scale(&x.n, &m) }
        };
        self.cancel(El { n: rn_add(&lift(a), &lift(b), minus), d: l })
    }

    fn mul(&self, a: &El, b: &El) -> El {
        let x = El { n: self.rn_mul(&a.n, &b.n), d: den_merge(&a.d, &b.d) };
        if x.d.is_empty() { x } else { self.cancel(x) }
    }

    fn inv(&mut self, a: &El) -> Option<El> {
        if a.n.is_empty() {
            return fail("division by zero");
        }
        let (m, p) = self.rn_inv(&a.n)?;
        let (c, fs) = self.factor_full(&p);
        let mut n = rn_scale_q(&m, &c.recip());
        for (f, e) in &a.d {
            for _ in 0..*e {
                n = rn_scale(&n, f);
            }
        }
        Some(self.cancel(El { n, d: fs }))
    }

    fn powi(&mut self, a: &El, k: i64) -> Option<El> {
        let base = if k < 0 { self.inv(a)? } else { a.clone() };
        let mut r = self.el_int(1);
        for _ in 0..k.unsigned_abs() {
            r = self.mul(&r, &base);
            if rn_terms(&r.n) > max_terms() || past_deadline() {
                return fail("power too large");
            }
        }
        Some(r)
    }

    // ------------------------------------------------------------ roots

    /// The content of a ring element with generators: (c, factors) dividing
    /// every coefficient, c positive.
    fn content(&mut self, m: &[MPoly]) -> (Q, Den) {
        let mut g = BigInt::zero();
        let mut l = BigInt::one();
        for p in m {
            for c in p.t.values() {
                g = g.gcd(c.numer());
                l = l.lcm(c.denom());
            }
        }
        let c = Q::new(g, l);
        let small = m.iter().min_by_key(|p| p.terms()).unwrap().clone();
        let (_, fs) = self.factor_full(&small);
        let mut out = Den::new();
        for (f, e) in fs {
            let mut k = e;
            for p in m {
                let mut r = p.clone();
                let mut have = 0;
                while have < k {
                    match r.div_exact(&f) {
                        Some(h) if !h.is_zero() => {
                            r = h;
                            have += 1;
                        }
                        _ => break,
                    }
                }
                k = have;
                if k == 0 {
                    break;
                }
            }
            if k > 0 {
                out.push((f, k));
            }
        }
        (c, out)
    }

    fn new_gen(&mut self, rho: Rn, val: f64, show: Option<Expr>, sig: Option<Sig>) -> Option<El> {
        if self.gens.len() >= MAX_GENS {
            return fail("too many roots");
        }
        self.gens.push(Gen { rho, val, show, sig });
        self.gexprs.push(None);
        let mut n = Rn::new();
        n.insert(1 << (self.gens.len() - 1), MPoly::one(self.nvar));
        Some(El { n, d: vec![] })
    }

    /// |p|: ±p by the sign of p at the current configuration. Where p is
    /// zero there the sign cannot be read off, and |p| becomes a generator
    /// with |p|² = p²: products still reduce, and only a division by
    /// |p| ± p, which is zero on one side, fails.
    fn abs_poly(&mut self, p: &MPoly) -> Option<El> {
        if p.is_const() {
            let (c, _) = norm_poly(p);
            return Some(self.el_poly(p.scale(&q(if c.is_negative() { -1 } else { 1 }))));
        }
        let (v, mag) = p.eval_f64(&self.vals);
        if !v.is_finite() {
            return fail("a factor without a value");
        }
        if v.abs() > 1e-9 * mag.max(1e-300) {
            return Some(self.el_poly(p.scale(&q(if v < 0.0 { -1 } else { 1 }))));
        }
        // (one sign of p for both of ±p)
        let (c, pp) = norm_poly(p);
        let show = fun(Fun::Abs, self.poly_plain(&pp)?);
        let g = match self.gens.iter().position(|g| g.show.as_ref() == Some(&show)) {
            Some(i) => {
                let mut n = Rn::new();
                n.insert(1 << i, MPoly::one(self.nvar));
                El { n, d: vec![] }
            }
            None => {
                if self.no_new > 0 {
                    return None;
                }
                let mut rho = Rn::new();
                rho.insert(0, pp.mul(&pp));
                self.new_gen(rho, 0.0, Some(show), None)?
            }
        };
        let c = if c.is_negative() { -c } else { c };
        Some(El { n: rn_scale_q(&g.n, &c), d: vec![] })
    }

    /// g_i with the sign that makes it positive.
    fn gen_abs(&self, i: usize) -> Option<El> {
        let v = self.gens[i].val;
        // (a square root is never negative; a sine needs its sign)
        if !(v.abs() > 1e-9) && self.gens[i].show.is_some() {
            return fail_zero("a sine is zero here");
        }
        let v = if self.gens[i].show.is_none() { 1.0 } else { v };
        let mut n = Rn::new();
        n.insert(1 << i, self.poly_const(q(if v > 0.0 { 1 } else { -1 })));
        Some(El { n, d: vec![] })
    }

    /// √(Π sig) for a square-free radicand without generators.
    fn root_base(&mut self, sig: Sig) -> Option<El> {
        if sig.is_empty() {
            return Some(self.el_int(1));
        }
        let rho_p = self.sig_poly(&sig);
        // (zero is fine: the root is zero there)
        let (v, mag) = rho_p.eval_f64(&self.vals);
        if !(v > -1e-9 * mag.max(1e-300)) {
            return fail("a radicand is negative here");
        }
        let v = v.max(0.0);
        // a product of known roots: √ρ · Π g_J = |P| with P² = ρ · Π ρ_J
        let base: Vec<usize> = (0..self.gens.len()).filter(|&i| self.gens[i].sig.is_some()).collect();
        if base.len() <= 12 {
            let mut best: Option<u32> = None;
            for mask in 1u32..(1 << base.len()) {
                if best.is_some_and(|b| b.count_ones() <= mask.count_ones()) {
                    continue;
                }
                let mut s = sig.clone();
                for (j, &i) in base.iter().enumerate() {
                    if mask & (1 << j) != 0 {
                        for t in self.gens[i].sig.as_ref().unwrap() {
                            if !s.insert(*t) {
                                s.remove(t);
                            }
                        }
                    }
                }
                if s.is_empty() {
                    best = Some(mask);
                }
            }
            if let Some(mask) = best {
                let js: Vec<usize> = base.iter().enumerate().filter(|(j, _)| mask & (1 << j) != 0).map(|(_, &i)| i).collect();
                // P: every prime to half its total multiplicity
                let mut count: BTreeMap<Tok, u32> = BTreeMap::new();
                for t in sig.iter().chain(js.iter().flat_map(|&i| self.gens[i].sig.as_ref().unwrap().iter())) {
                    *count.entry(*t).or_insert(0) += 1;
                }
                let mut p = MPoly::one(self.nvar);
                for (t, c) in &count {
                    for _ in 0..c / 2 {
                        p = match t {
                            Tok::Neg => p,
                            Tok::Prime(v) => p.scale(&Q::from_integer(BigInt::from(*v))),
                            Tok::Poly(i) => p.mul(&self.irreds[*i]),
                        };
                    }
                }
                // √ρ = |P| · Π|g_J| / Π ρ_J
                let mut x = self.abs_poly(&p)?;
                for &i in &js {
                    let rho = self.gens[i].rho.get(&0)?.clone();
                    let g = self.gen_abs(i)?;
                    let (c, fs) = self.factor_full(&rho);
                    let r = El { n: rn_scale_q(&g.n, &c.recip()), d: fs };
                    x = self.mul(&x, &r);
                }
                return Some(x);
            }
        }
        if self.no_new > 0 {
            return None;
        }
        let mut rho = Rn::new();
        rho.insert(0, rho_p);
        self.new_gen(rho, v.sqrt(), None, Some(sig))
    }

    /// √ρ for a radicand with generators, its content already removed.
    fn root_nested(&mut self, rho: Rn, depth: u32) -> Option<El> {
        let (v, mag) = self.rn_val(&rho);
        if !(v > -1e-9 * mag.max(1e-300)) {
            return fail("a nested radicand is negative here");
        }
        let zero_here = !(v > 1e-9 * mag.max(1e-300));
        let v = v.max(0.0);
        if let Some(i) = self.gens.iter().position(|g| g.rho == rho) {
            return self.gen_abs(i);
        }
        // a square in the field below: √(a + b·g) = √((a + d)/2) ± √((a − d)/2), d² = a² − b²·ρ_k
        // (at a zero the branch of the two parts cannot be read off)
        if depth < 3 && !zero_here {
            let k = rn_top(&rho)?;
            let (a, b) = rn_split(&rho, k);
            let norm = rn_add(&self.rn_mul(&a, &a), &self.rn_mul(&self.rn_mul(&b, &b), &self.gens[k as usize].rho), true);
            let saved = self.gens.len();
            self.no_new += 1;
            let d = if norm.is_empty() { None } else { self.sqrt_depth(&El { n: norm, d: vec![] }, depth + 1) };
            self.no_new -= 1;
            if let Some(d) = d {
                let ae = El { n: a.clone(), d: vec![] };
                let half = self.el_q(Q::new(BigInt::one(), BigInt::from(2)));
                let u = self.mul(&self.add(&ae, &d, false), &half);
                let w = self.mul(&self.add(&ae, &d, true), &half);
                let bs = self.rn_val(&b).0 * self.gens[k as usize].val;
                let r = (|| {
                    let su = self.sqrt_depth(&u, depth + 1)?;
                    let sw = self.sqrt_depth(&w, depth + 1)?;
                    let r = self.add(&su, &sw, bs < 0.0);
                    ((self.val(&r) - v.sqrt()).abs() <= 1e-8 * (1.0 + v.sqrt())).then_some(r)
                })();
                if r.is_some() {
                    return r;
                }
                self.gens.truncate(saved);
                self.gexprs.truncate(saved);
            }
        }
        if self.no_new > 0 {
            return None;
        }
        self.new_gen(rho, v.sqrt(), None, None)
    }

    fn sqrt(&mut self, x: &El) -> Option<El> {
        self.sqrt_depth(x, 0)
    }

    fn sqrt_depth(&mut self, x: &El, depth: u32) -> Option<El> {
        if x.n.is_empty() {
            return Some(self.el_int(0));
        }
        if past_deadline() {
            return None;
        }
        // √(n/d) = √(n · Π f_odd) / Π |f|^⌈e/2⌉
        let mut m = x.n.clone();
        let mut den = Den::new();
        let mut sign = 1.0;
        for (f, e) in &x.d {
            if e % 2 == 1 {
                m = rn_scale(&m, f);
            }
            let h = (e + 1) / 2;
            if h % 2 == 1 {
                sign *= self.poly_sign(f)?;
            }
            den.push((f.clone(), h));
        }
        // square factors out
        let (outer, root) = if m.len() == 1 && m.contains_key(&0) {
            let (cs, s, sig) = self.sqfree_split(&m[&0]);
            (s.scale(&cs), self.root_base(sig)?)
        } else {
            let (c, fs) = self.content(&m.values().cloned().collect::<Vec<_>>());
            // (the square-free part of c stays in the radicand)
            let (cs, _) = Self::split_q(&c);
            let mut s = MPoly::one(self.nvar).scale(&cs);
            let mut rho = rn_scale_q(&m, &(&cs * &cs).recip());
            for (f, e) in fs {
                for _ in 0..e / 2 {
                    s = s.mul(&f);
                    rho = rho.iter().map(|(k, p)| Some((*k, p.div_exact(&f)?.div_exact(&f)?))).collect::<Option<Rn>>()?;
                }
            }
            (s, self.root_nested(rho, depth)?)
        };
        // |outer| over the |f|: the signs of both at the current configuration
        let outer = self.abs_poly(&outer)?;
        let outer = if sign < 0.0 { self.mul(&outer, &self.el_int(-1)) } else { outer };
        let r = self.mul(&self.mul(&root, &outer), &El { n: self.el_int(1).n, d: den });
        let (want, got) = (self.val(x).max(0.0).sqrt(), self.val(&r));
        // (near a zero the root of a rounding error is not small)
        if (got - want).abs() <= 1e-7 * (1.0 + want.abs()) || (got * got - want * want).abs() <= 1e-12 * (1.0 + want * want) { Some(r) } else { fail("a root lost its value") }
    }

    // ------------------------------------------------------------ angles

    /// arg = Σ coefficient · term + c·π.
    fn angle_terms(arg: &Expr) -> Option<(Vec<(Rat, Expr)>, Rat)> {
        let ts: Vec<Expr> = match arg.node() {
            Node::Add(ts) => ts.clone(),
            _ => vec![arg.clone()],
        };
        let mut out = Vec::new();
        let mut cpi = Rat::int(0);
        for t in ts {
            let (c, rest) = t.split_coeff();
            let Num::R(c) = c else { return None };
            match rest.node() {
                Node::Pi => cpi = cpi.add(c)?,
                Node::Num(_) => return None,
                _ => out.push((c, rest)),
            }
        }
        Some((out, cpi))
    }

    fn scan_angles(&mut self, e: &Expr) {
        match e.node() {
            Node::Num(_) | Node::Pi | Node::Sym(_) => {}
            Node::Add(ts) | Node::Mul(ts) => ts.iter().for_each(|t| self.scan_angles(t)),
            Node::Pow(a, b) | Node::Atan2(a, b) => {
                self.scan_angles(a);
                self.scan_angles(b);
            }
            Node::Fun(f, a) => {
                if matches!(f, Fun::Sin | Fun::Cos | Fun::Tan) {
                    if let Some((ts, _)) = Self::angle_terms(a) {
                        for (c, t) in ts {
                            match self.angle_lcm.iter_mut().find(|x| x.0 == t) {
                                Some(x) => x.1 = (x.1 as i128).lcm(&c.d) as i64,
                                None => self.angle_lcm.push((t, c.d as i64)),
                            }
                        }
                    }
                }
                self.scan_angles(a);
            }
        }
    }

    /// (cos β, sin β, L) for the base angle β = term / L.
    fn base_angle(&mut self, term: &Expr) -> Option<(El, El, i64)> {
        let l = self.angle_lcm.iter().find(|x| x.0 == *term).map(|x| x.1).unwrap_or(1);
        if let Some(hit) = self.angles.iter().find(|x| x.0 == *term) {
            return Some((hit.1.0.clone(), hit.1.1.clone(), l));
        }
        let one = self.el_int(1);
        let pair = match term.node() {
            Node::Fun(Fun::Acos, u) if l == 1 => {
                let c = self.conv(u)?;
                let s = self.sqrt(&self.add(&one, &self.mul(&c, &c), true))?;
                (c, s)
            }
            Node::Fun(Fun::Asin, u) if l == 1 => {
                let s = self.conv(u)?;
                let c = self.sqrt(&self.add(&one, &self.mul(&s, &s), true))?;
                (c, s)
            }
            Node::Fun(Fun::Atan, u) if l == 1 => {
                let u = self.conv(u)?;
                let h = self.sqrt(&self.add(&one, &self.mul(&u, &u), false))?;
                let c = self.inv(&h)?;
                let s = self.mul(&u, &c);
                (c, s)
            }
            Node::Atan2(y, x) if l == 1 => {
                let (y, x) = (self.conv(y)?, self.conv(x)?);
                let h = self.sqrt(&self.add(&self.mul(&x, &x), &self.mul(&y, &y), false))?;
                let hi = self.inv(&h)?;
                (self.mul(&x, &hi), self.mul(&y, &hi))
            }
            _ => {
                let beta = mul(vec![rat(1, l), term.clone()]);
                let bv = eval(&beta, self.env);
                if !bv.is_finite() {
                    return None;
                }
                let ci = self.atom(&cos(beta.clone()))?;
                let c = MPoly::var(self.nvar, ci);
                let rho_p = MPoly::one(self.nvar).sub(&c.mul(&c));
                let (_, _, sig) = self.sqfree_split(&rho_p);
                let mut rho = Rn::new();
                rho.insert(0, rho_p);
                let s = self.new_gen(rho, bv.sin(), Some(sin(beta)), Some(sig))?;
                (self.el_poly(c), s)
            }
        };
        self.angles.push((term.clone(), pair.clone()));
        Some((pair.0, pair.1, l))
    }

    /// (cos, sin) of an argument, expanded over the base angles.
    fn trig_pair(&mut self, arg: &Expr) -> Option<(El, El)> {
        let (ts, cpi) = Self::angle_terms(arg)?;
        let mut c = self.el_int(1);
        let mut s = self.el_int(0);
        let turn = |this: &mut Self, c: &mut El, s: &mut El, c2: &El, s2: &El| {
            let nc = this.add(&this.mul(c, c2), &this.mul(s, s2), true);
            let ns = this.add(&this.mul(s, c2), &this.mul(c, s2), false);
            *c = nc;
            *s = ns;
        };
        if cpi.n != 0 {
            let a = mul(vec![num(Num::R(cpi)), pi()]);
            let (ce, se) = (cos(a.clone()), sin(a));
            let plain = |e: &Expr| !matches!(e.node(), Node::Fun(..));
            if !plain(&ce) || !plain(&se) {
                return None;
            }
            let (c2, s2) = (self.conv(&ce)?, self.conv(&se)?);
            turn(self, &mut c, &mut s, &c2, &s2);
        }
        for (coef, term) in ts {
            let (cb, sb, l) = self.base_angle(&term)?;
            let n = coef.mul(Rat::int(l as i128))?;
            if n.d != 1 || n.n.abs() > 8 {
                return None;
            }
            let sb = if n.n < 0 { self.mul(&sb, &self.el_int(-1)) } else { sb };
            for _ in 0..n.n.abs() {
                turn(self, &mut c, &mut s, &cb, &sb);
                if rn_terms(&c.n) + rn_terms(&s.n) > max_terms() || past_deadline() {
                    return None;
                }
            }
        }
        Some((c, s))
    }

    // ------------------------------------------------------------ conversion

    /// A subexpression that is not algebraic over its arguments: the
    /// function of the arguments' normal forms, as an atom.
    fn opaque(&mut self, e: &Expr) -> Option<El> {
        let inner = |this: &mut Self, a: &Expr| -> Expr {
            if a.size() <= 2 {
                return a.clone();
            }
            let (saved, saved_angles) = (this.gens.len(), this.angles.len());
            match this.conv(a).and_then(|x| this.to_expr(&x)) {
                Some(x) if !past_deadline() => x,
                _ => {
                    this.gens.truncate(saved);
                    this.gexprs.truncate(saved);
                    this.angles.truncate(saved_angles);
                    this.memo.clear();
                    a.clone()
                }
            }
        };
        let r = match e.node() {
            Node::Fun(f, a) => fun(*f, inner(self, a)),
            Node::Atan2(a, b) => atan2(inner(self, a), inner(self, b)),
            Node::Pow(a, b) => pow(inner(self, a), inner(self, b)),
            _ => e.clone(),
        };
        // (the constructors may have reduced it to something algebraic)
        if matches!(r.node(), Node::Fun(..) | Node::Atan2(..) | Node::Pow(..) | Node::Sym(_) | Node::Pi) {
            self.atom_el(&r)
        } else {
            self.conv(&r)
        }
    }

    fn conv(&mut self, e: &Expr) -> Option<El> {
        if past_deadline() {
            return None;
        }
        // substituted formulas share their subexpressions many times over
        let key = std::rc::Rc::as_ptr(&e.0);
        if let Some(hit) = self.memo.get(&key) {
            return Some(hit.clone());
        }
        let r = self.conv_node(e)?;
        self.memo.insert(key, r.clone());
        Some(r)
    }

    fn conv_node(&mut self, e: &Expr) -> Option<El> {
        let r = match e.node() {
            Node::Num(Num::R(r)) => self.el_q(Q::new(BigInt::from(r.n), BigInt::from(r.d))),
            Node::Num(_) => return fail("a float"),
            Node::Pi | Node::Sym(_) => self.atom_el(e)?,
            Node::Add(ts) => {
                let mut acc = self.el_int(0);
                for t in ts {
                    let x = self.conv(t)?;
                    acc = self.add(&acc, &x, false);
                }
                acc
            }
            Node::Mul(ts) => {
                let mut acc = self.el_int(1);
                for t in ts {
                    let x = self.conv(t)?;
                    acc = self.mul(&acc, &x);
                }
                acc
            }
            Node::Pow(b, x) => match x.as_rat() {
                Some(r) if r.d == 1 && r.n.abs() <= 64 => {
                    let b = self.conv(b)?;
                    self.powi(&b, r.n as i64)?
                }
                Some(r) if r.d == 2 && r.n.abs() <= 64 => {
                    let b = self.conv(b)?;
                    let s = self.sqrt(&b)?;
                    self.powi(&s, r.n as i64)?
                }
                _ => self.opaque(e)?,
            },
            Node::Fun(Fun::Abs, a) => {
                let x = self.conv(a)?;
                let (v, mag) = self.rn_val(&x.n);
                if !(v.abs() > 1e-9 * mag.max(1e-300)) {
                    // zero here: |p| for a polynomial, the function itself otherwise
                    return if x.d.is_empty() && x.n.len() == 1 && x.n.contains_key(&0) {
                        let p = x.n[&0].clone();
                        self.abs_poly(&p)
                    } else {
                        let inner = self.to_expr(&x)?;
                        self.atom_el(&fun(Fun::Abs, inner))
                    };
                }
                let d: f64 = x.d.iter().map(|(f, e)| f.eval_f64(&self.vals).0.powi(*e as i32)).product();
                if v * d < 0.0 { self.mul(&x, &self.el_int(-1)) } else { x }
            }
            Node::Fun(f @ (Fun::Sin | Fun::Cos | Fun::Tan), a) => {
                let saved = (self.gens.len(), self.angles.len());
                match self.trig_pair(a) {
                    Some((c, s)) => match f {
                        Fun::Cos => c,
                        Fun::Sin => s,
                        _ => {
                            let ci = self.inv(&c)?;
                            self.mul(&s, &ci)
                        }
                    },
                    None => {
                        if past_deadline() {
                            return None;
                        }
                        self.gens.truncate(saved.0);
                        self.gexprs.truncate(saved.0);
                        self.angles.truncate(saved.1);
                        self.memo.clear();
                        self.opaque(e)?
                    }
                }
            }
            Node::Fun(..) | Node::Atan2(..) => self.opaque(e)?,
        };
        if rn_terms(&r.n) <= max_terms() { Some(r) } else { fail("too many terms") }
    }

    // ------------------------------------------------------------ output

    fn q_expr(c: &Q) -> Option<Expr> {
        match (c.numer().to_i128(), c.denom().to_i128()) {
            (Some(a), Some(b)) => Rat::new(a, b).map(|r| num(Num::R(r))).or_else(|| fail("a coefficient too large")),
            _ => fail("a coefficient too large"),
        }
    }

    fn poly_plain(&self, p: &MPoly) -> Option<Expr> {
        let mut terms = Vec::new();
        for (m, c) in &p.t {
            let mut fs = vec![Self::q_expr(c)?];
            for (k, e) in m.iter().enumerate() {
                if *e > 0 {
                    fs.push(pow(self.atoms[k].clone(), int(*e as i64)));
                }
            }
            terms.push(mul(fs));
        }
        Some(add(terms))
    }

    /// A polynomial as a product of its factors where that is shorter, each
    /// factor with the sign that has more positive terms.
    fn poly_expr(&mut self, p: &MPoly) -> Option<Expr> {
        let plain = self.poly_plain(p)?;
        if p.terms() < 2 {
            return Some(plain);
        }
        let (mut c, fs) = self.factor_full(p);
        if fs.len() == 1 && fs[0].1 == 1 && (c.is_one() || p.terms() < 3) {
            return Some(plain);
        }
        let mut parts = Vec::new();
        for (f, e) in &fs {
            let negs = f.t.values().filter(|v| v.is_negative()).count();
            let f = if 2 * negs > f.terms() {
                if e % 2 == 1 {
                    c = -c;
                }
                f.scale(&q(-1))
            } else {
                f.clone()
            };
            parts.push(pow(self.poly_plain(&f)?, int(*e as i64)));
        }
        parts.push(Self::q_expr(&c)?);
        let f = mul(parts);
        // (a product only a little smaller reads worse than the sum)
        Some(if 10 * complexity(&f) <= 7 * complexity(&plain) { f } else { plain })
    }

    fn gen_expr(&mut self, i: usize) -> Option<Expr> {
        if let Some(e) = &self.gexprs[i] {
            return Some(e.clone());
        }
        let e = match &self.gens[i].show {
            Some(e) => e.clone(),
            None => {
                let rho = self.gens[i].rho.clone();
                sqrt(self.rn_expr(&rho)?)
            }
        };
        self.gexprs[i] = Some(e.clone());
        Some(e)
    }

    fn rn_expr(&mut self, a: &Rn) -> Option<Expr> {
        if a.is_empty() {
            return Some(zero());
        }
        // each term: its polynomial and its roots. Plain square roots of one
        // term are written as one root, √f·√(f·g) = |f|·√g.
        let mut terms: Vec<(MPoly, Vec<Expr>)> = Vec::new();
        for (m, p) in a {
            let ids: Vec<usize> = (0..self.gens.len()).filter(|i| m & (1 << i) != 0).collect();
            let plain: Vec<usize> = ids.iter().copied().filter(|&i| self.gens[i].show.is_none() && self.gens[i].sig.is_some()).collect();
            let mut p = p.clone();
            let mut roots = Vec::new();
            let mut joined = false;
            if plain.len() >= 2 {
                let mut count: BTreeMap<Tok, u32> = BTreeMap::new();
                for t in plain.iter().flat_map(|&i| self.gens[i].sig.as_ref().unwrap().iter()) {
                    *count.entry(*t).or_insert(0) += 1;
                }
                let mut sq = MPoly::one(self.nvar);
                let mut rest = Sig::new();
                for (t, c) in &count {
                    for _ in 0..c / 2 {
                        sq = match t {
                            Tok::Neg => sq,
                            Tok::Prime(v) => sq.scale(&Q::from_integer(BigInt::from(*v))),
                            Tok::Poly(i) => sq.mul(&self.irreds[*i]),
                        };
                    }
                    if c % 2 == 1 {
                        rest.insert(*t);
                    }
                }
                let (v, mag) = sq.eval_f64(&self.vals);
                if v.abs() > 1e-9 * mag.max(1e-300) {
                    p = p.mul(&sq).scale(&q(if v < 0.0 { -1 } else { 1 }));
                    if !rest.is_empty() {
                        let rho = self.sig_poly(&rest);
                        roots.push(sqrt(self.poly_expr(&rho)?));
                    }
                    joined = true;
                }
            }
            for i in ids {
                if !(joined && plain.contains(&i)) {
                    roots.push(self.gen_expr(i)?);
                }
            }
            terms.push((p, roots));
        }
        // what all coefficients share goes in front
        let mut front = None;
        if terms.len() > 1 {
            let ps: Vec<MPoly> = terms.iter().map(|t| t.0.clone()).collect();
            let (c, fs) = self.content(&ps);
            let mut g = MPoly::one(self.nvar).scale(&c);
            for (f, e) in &fs {
                for _ in 0..*e {
                    g = g.mul(f);
                }
            }
            if !(g.is_const() && c.is_one()) {
                for t in terms.iter_mut() {
                    t.0 = t.0.div_exact(&g)?;
                }
                front = Some(g);
            }
        }
        let mut sum = Vec::new();
        for (p, roots) in terms {
            let mut fs = vec![self.poly_expr(&p)?];
            fs.extend(roots);
            sum.push(mul(fs));
        }
        let sum = add(sum);
        Some(match front {
            Some(g) => mul(vec![self.poly_expr(&g)?, sum]),
            None => sum,
        })
    }

    fn to_expr(&mut self, x: &El) -> Option<Expr> {
        let n = self.rn_expr(&x.n)?;
        let mut ds = Vec::new();
        for (f, e) in &x.d {
            ds.push(pow(self.poly_expr(f)?, int(*e as i64)));
        }
        Some(if ds.is_empty() { n } else { div(n, mul(ds)) })
    }
}

/// The subexpressions of `e` that become atoms: symbols, π, and functions
/// and powers that are not algebraic.
fn count_atoms(e: &Expr, seen: &mut BTreeSet<Expr>) {
    match e.node() {
        Node::Num(_) => {}
        Node::Pi | Node::Sym(_) => {
            seen.insert(e.clone());
        }
        Node::Add(ts) | Node::Mul(ts) => ts.iter().for_each(|t| count_atoms(t, seen)),
        Node::Pow(a, b) => {
            if b.as_rat().is_none_or(|r| r.d > 2) {
                seen.insert(e.clone());
            }
            count_atoms(a, seen);
            count_atoms(b, seen);
        }
        Node::Atan2(a, b) => {
            seen.insert(e.clone());
            count_atoms(a, seen);
            count_atoms(b, seen);
        }
        Node::Fun(_, a) => {
            seen.insert(e.clone());
            count_atoms(a, seen);
        }
    }
}

/// The normal form of `e` (see the module documentation), with signs and
/// branches taken where `env` gives the values of the symbols. None when
/// `e` is outside what the form covers, grows too large, or the deadline
/// passes. The caller compares the result with `e` numerically.
pub(crate) fn normal_form(e: &Expr, env: &dyn Fn(u32) -> f64) -> Option<Expr> {
    normal_form_sized(e, env, 8000)
}

/// Like [`normal_form`], with elements of up to `terms` polynomial terms
/// (a longer search takes more).
pub(crate) fn normal_form_sized(e: &Expr, env: &dyn Fn(u32) -> f64, terms: usize) -> Option<Expr> {
    MAX_TERMS_NOW.with(|c| c.set(terms));
    let mut seen = BTreeSet::new();
    count_atoms(e, &mut seen);
    // (an |f| that cannot be resolved becomes one more)
    let nvar = seen.len() + 4;
    if nvar > 60 {
        return fail("too many atoms to start with");
    }
    let mut cx = Ctx {
        nvar,
        atoms: Vec::new(),
        vals: vec![0.0; nvar],
        gens: Vec::new(),
        env,
        angle_lcm: Vec::new(),
        angles: Vec::new(),
        irreds: Vec::new(),
        fcache: Vec::new(),
        no_new: 0,
        gexprs: Vec::new(),
        memo: HashMap::new(),
    };
    cx.scan_angles(e);
    if cx.angle_lcm.iter().any(|x| x.1 > 4) {
        return fail("angle fractions");
    }
    // products of cos and sin of many angles grow much faster than their
    // Fourier form (see `trig_linearize`), which is the better tool there
    if cx.angle_lcm.len() > 2 {
        return fail("more than two angles");
    }
    let x = cx.conv(e)?;
    if past_deadline() {
        return None;
    }
    let out = cx.to_expr(&x)?;
    (!past_deadline()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMES: [&str; 6] = ["a", "b", "c", "d", "x", "y"];
    const VALS: [f64; 6] = [5.0, 4.3, 3.9, 1.7, 0.6, 1.1];

    fn p(src: &str) -> Expr {
        crate::expr::parse::parse(src, &|n: &str| NAMES.iter().position(|x| *x == n).map(|i| sym(i as u32))).unwrap()
    }
    fn show(e: &Expr) -> String {
        crate::expr::bx::to_plain(e, &|s| NAMES[s as usize].to_string())
    }
    fn nf(src: &str) -> String {
        let e = p(src);
        let env = |s: u32| VALS[s as usize];
        let r = normal_form(&e, &env).unwrap_or_else(|| panic!("no normal form of {src}"));
        let (v0, v1) = (eval(&e, &env), eval(&r, &env));
        assert!((v0 - v1).abs() <= 1e-9 * (1.0 + v0.abs()), "{src}: {v0} became {v1}: {}", show(&r));
        show(&r)
    }

    #[test]
    fn rational_functions_cancel() {
        assert_eq!(nf("(a^2 - b^2)/(a - b) - b"), "a");
        assert_eq!(nf("1/(1/a + 1/b)"), "(a*b)/(a + b)");
    }

    #[test]
    fn square_factors_leave_the_root() {
        assert_eq!(nf("sqrt(a^2*b - 2*a*b*c + b*c^2)"), "sqrt(b)*(a - c)");
        assert_eq!(nf("sqrt((a - b)^2)"), "a - b");
        assert_eq!(nf("sqrt((b - a)^2)"), "a - b");
    }

    #[test]
    fn roots_are_identified() {
        // the same root up to a square factor, and a product of known roots
        assert_eq!(nf("sqrt(4a^2 - b^2) - 2sqrt(a^2 - b^2/4)"), "0");
        assert_eq!(nf("sqrt(a)*sqrt(b) - sqrt(a*b)"), "0");
        assert_eq!(nf("1/(sqrt(a) - sqrt(b)) - (sqrt(a) + sqrt(b))/(a - b)"), "0");
    }

    #[test]
    fn denesting() {
        assert_eq!(nf("sqrt(5 + 2sqrt(6)) - sqrt(2) - sqrt(3)"), "0");
        assert_eq!(nf("sqrt(a + b + 2sqrt(a*b)) - sqrt(a) - sqrt(b)"), "0");
        // a perfect square in the field with √d
        assert_eq!(nf("sqrt((a + b*sqrt(d))^2) - a - b*sqrt(d)"), "0");
    }

    #[test]
    fn absolute_value_where_the_sign_is_unknown() {
        // a = b here: the sign of a − b cannot be read off
        let env = |s: u32| [2.0, 2.0, 3.0, 1.7, 0.6, 1.1][s as usize];
        let nf = |src: &str| show(&normal_form(&p(src), &env).unwrap_or_else(|| panic!("no normal form of {src}")));
        assert_eq!(nf("sqrt((a - b)^2*c)"), "sqrt(c)*abs(a - b)");
        assert_eq!(nf("sqrt((b - a)^2*c) - sqrt((a - b)^2*c)"), "0");
        assert_eq!(nf("sqrt((a - b)^2)^2"), "(a - b)^2");
        // a root that is zero here is still a root
        assert_eq!(nf("sqrt(a - b)*sqrt(a - b)"), "a - b");
    }

    #[test]
    fn trigonometry() {
        assert_eq!(nf("sin(x)^2 + cos(x)^2"), "1");
        assert_eq!(nf("cos(x + y) - cos(x)*cos(y) + sin(x)*sin(y)"), "0");
        assert_eq!(nf("sqrt(1 - cos(x)^2)"), "sin(x)");
        assert_eq!(nf("cos(acos(a/(a + b)))"), "a/(a + b)");
        assert_eq!(nf("sin(atan2(a, b))*sqrt(a^2 + b^2)"), "a");
    }
}
