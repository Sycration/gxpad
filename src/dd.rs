//! Double-double arithmetic: numbers as unevaluated sums hi + lo of two
//! f64, about 31 significant digits. Used where a fit to samples must tell
//! an exact relation from a merely good approximation (see `fit`).

use crate::expr::{Expr, Fun, Node, Num};
use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dd {
    pub hi: f64,
    pub lo: f64,
}

#[inline]
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    (s, (a - (s - bb)) + (b - bb))
}

#[inline]
fn quick_two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    (s, b - (s - a))
}

#[inline]
fn two_prod(a: f64, b: f64) -> (f64, f64) {
    let p = a * b;
    (p, a.mul_add(b, -p))
}

pub const PI: Dd = Dd { hi: 3.141592653589793, lo: 1.2246467991473532e-16 };
const HALF_PI: Dd = Dd { hi: 1.5707963267948966, lo: 6.123233995736766e-17 };
const LN2: Dd = Dd { hi: 0.6931471805599453, lo: 2.3190468138462996e-17 };

impl Dd {
    pub const ZERO: Dd = Dd { hi: 0.0, lo: 0.0 };
    pub const ONE: Dd = Dd { hi: 1.0, lo: 0.0 };

    #[inline]
    pub fn from(x: f64) -> Dd {
        Dd { hi: x, lo: 0.0 }
    }
    #[inline]
    pub fn to_f64(self) -> f64 {
        self.hi + self.lo
    }
    pub fn is_finite(self) -> bool {
        self.hi.is_finite() && self.lo.is_finite()
    }
    pub fn abs(self) -> Dd {
        if self.hi < 0.0 || (self.hi == 0.0 && self.lo < 0.0) { -self } else { self }
    }
    pub fn sqrt(self) -> Dd {
        if self.hi <= 0.0 {
            return Dd::from(if self.hi == 0.0 { 0.0 } else { f64::NAN });
        }
        let x = 1.0 / self.hi.sqrt();
        let ax = self.hi * x;
        let (p, e) = two_prod(ax, ax);
        let d = (self - Dd { hi: p, lo: e }).hi * (x * 0.5);
        let (s, e) = quick_two_sum(ax, d);
        Dd { hi: s, lo: e }
    }
    pub fn powi(self, k: i32) -> Dd {
        let mut r = Dd::ONE;
        let mut b = self;
        let mut n = k.unsigned_abs();
        while n > 0 {
            if n & 1 == 1 {
                r = r * b;
            }
            b = b * b;
            n >>= 1;
        }
        if k < 0 { Dd::ONE / r } else { r }
    }
    /// (sin, cos).
    pub fn sin_cos(self) -> (Dd, Dd) {
        if !self.is_finite() {
            return (Dd::from(f64::NAN), Dd::from(f64::NAN));
        }
        // x = k·π/2 + r, |r| ≤ π/4
        let k = (self.hi / HALF_PI.hi).round();
        let r = self - HALF_PI * Dd::from(k);
        let r2 = r * r;
        // Taylor series
        let (mut s, mut c) = (r, Dd::ONE);
        let (mut ts, mut tc) = (r, Dd::ONE);
        let mut n = 1.0;
        loop {
            tc = -(tc * r2) / Dd::from((2.0 * n - 1.0) * (2.0 * n));
            ts = -(ts * r2) / Dd::from((2.0 * n) * (2.0 * n + 1.0));
            c = c + tc;
            s = s + ts;
            n += 1.0;
            if (tc.hi.abs() < 1e-34 && ts.hi.abs() < 1e-34) || n > 40.0 {
                break;
            }
        }
        match (k as i64).rem_euclid(4) {
            0 => (s, c),
            1 => (c, -s),
            2 => (-s, -c),
            _ => (-c, s),
        }
    }
    pub fn atan2(self, x: Dd) -> Dd {
        let y = self;
        let a0 = y.hi.atan2(x.hi);
        if !a0.is_finite() || (y.hi == 0.0 && x.hi == 0.0) {
            return Dd::from(a0);
        }
        // rotate (x, y) back by a0; what is left is a tiny angle z ≈ tan z
        let (s, c) = Dd::from(a0).sin_cos();
        let z = (y * c - x * s) / (x * c + y * s);
        Dd::from(a0) + z
    }
    pub fn exp(self) -> Dd {
        if !self.is_finite() || self.hi.abs() > 700.0 {
            return Dd::from(self.hi.exp());
        }
        let k = (self.hi / LN2.hi).round();
        let r = (self - LN2 * Dd::from(k)) * Dd::from(1.0 / 256.0);
        // s = exp(r) - 1, then (1 + s)² - 1 = 2s + s² eight times
        let mut s = Dd::ZERO;
        let mut t = Dd::ONE;
        for n in 1..30 {
            t = t * r / Dd::from(n as f64);
            s = s + t;
            if t.hi.abs() < 1e-36 {
                break;
            }
        }
        for _ in 0..8 {
            s = s + s + s * s;
        }
        (Dd::ONE + s) * Dd::from(2f64.powi(k as i32))
    }
    pub fn ln(self) -> Dd {
        if !(self.hi > 0.0) || !self.is_finite() {
            return Dd::from(self.hi.ln());
        }
        // Newton step on exp from the f64 value
        let y0 = Dd::from(self.hi.ln());
        y0 + self * (-y0).exp() - Dd::ONE
    }
    pub fn apply(self, f: Fun) -> Dd {
        let clamp = |x: Dd| {
            if x.hi > 1.0 && x.hi < 1.0 + 1e-9 {
                Dd::ONE
            } else if x.hi < -1.0 && x.hi > -1.0 - 1e-9 {
                -Dd::ONE
            } else {
                x
            }
        };
        match f {
            Fun::Sin => self.sin_cos().0,
            Fun::Cos => self.sin_cos().1,
            Fun::Tan => {
                let (s, c) = self.sin_cos();
                s / c
            }
            Fun::Asin => {
                let x = clamp(self);
                x.atan2((Dd::ONE - x * x).sqrt())
            }
            Fun::Acos => {
                let x = clamp(self);
                (Dd::ONE - x * x).sqrt().atan2(x)
            }
            Fun::Atan => self.atan2(Dd::ONE),
            Fun::Exp => self.exp(),
            Fun::Ln => self.ln(),
            Fun::Sinh => {
                let e = self.exp();
                (e - Dd::ONE / e) * Dd::from(0.5)
            }
            Fun::Cosh => {
                let e = self.exp();
                (e + Dd::ONE / e) * Dd::from(0.5)
            }
            Fun::Tanh => {
                let e = (self + self).exp();
                (e - Dd::ONE) / (e + Dd::ONE)
            }
            Fun::Abs => self.abs(),
        }
    }
    pub fn pow(self, e: Dd) -> Dd {
        if e.lo == 0.0 && e.hi.fract() == 0.0 && e.hi.abs() < 1000.0 {
            return self.powi(e.hi as i32);
        }
        if self.hi == 0.0 {
            return Dd::from(0f64.powf(e.hi));
        }
        if self.hi < 0.0 {
            return Dd::from(self.hi.powf(e.hi));
        }
        (e * self.ln()).exp()
    }
}

impl Add for Dd {
    type Output = Dd;
    #[inline]
    fn add(self, o: Dd) -> Dd {
        let (s, e) = two_sum(self.hi, o.hi);
        let (t, f) = two_sum(self.lo, o.lo);
        let (s, e) = quick_two_sum(s, e + t);
        let (s, e) = quick_two_sum(s, e + f);
        Dd { hi: s, lo: e }
    }
}
impl Neg for Dd {
    type Output = Dd;
    #[inline]
    fn neg(self) -> Dd {
        Dd { hi: -self.hi, lo: -self.lo }
    }
}
impl Sub for Dd {
    type Output = Dd;
    #[inline]
    fn sub(self, o: Dd) -> Dd {
        self + (-o)
    }
}
impl Mul for Dd {
    type Output = Dd;
    #[inline]
    fn mul(self, o: Dd) -> Dd {
        let (p, e) = two_prod(self.hi, o.hi);
        let e = e + (self.hi * o.lo + self.lo * o.hi);
        let (s, e) = quick_two_sum(p, e);
        Dd { hi: s, lo: e }
    }
}
impl Div for Dd {
    type Output = Dd;
    #[inline]
    fn div(self, o: Dd) -> Dd {
        let q1 = self.hi / o.hi;
        if !q1.is_finite() {
            return Dd::from(q1);
        }
        let r = self - o * Dd::from(q1);
        let q2 = r.hi / o.hi;
        let r = r - o * Dd::from(q2);
        let q3 = r.hi / o.hi;
        let (s, e) = quick_two_sum(q1, q2);
        Dd { hi: s, lo: e } + Dd::from(q3)
    }
}

fn from_i128(n: i128) -> Dd {
    let hi = n as f64;
    Dd { hi, lo: (n - hi as i128) as f64 }
}

enum DOp {
    C(Dd),
    L(u32),
    Add(u32),
    Mul(u32),
    Pow,
    Powi(i32),
    Sqrt,
    F(Fun),
    Atan2,
}

/// An expression compiled for evaluation in double-double arithmetic.
pub struct DdCode {
    ops: Vec<DOp>,
}

impl DdCode {
    pub fn new(e: &Expr) -> DdCode {
        let mut c = DdCode { ops: Vec::new() };
        c.emit(e);
        c
    }
    fn emit(&mut self, e: &Expr) {
        match e.node() {
            Node::Num(Num::R(r)) => self.ops.push(DOp::C(if r.d == 1 { from_i128(r.n) } else { from_i128(r.n) / from_i128(r.d) })),
            Node::Num(Num::F(f)) => self.ops.push(DOp::C(Dd::from(*f))),
            Node::Pi => self.ops.push(DOp::C(PI)),
            Node::Sym(s) => self.ops.push(DOp::L(*s)),
            Node::Add(ts) => {
                ts.iter().for_each(|t| self.emit(t));
                self.ops.push(DOp::Add(ts.len() as u32));
            }
            Node::Mul(ts) => {
                ts.iter().for_each(|t| self.emit(t));
                self.ops.push(DOp::Mul(ts.len() as u32));
            }
            Node::Pow(b, ex) => {
                self.emit(b);
                match ex.as_num() {
                    Some(Num::R(r)) if r.d == 1 && r.n.abs() < 1000 => self.ops.push(DOp::Powi(r.n as i32)),
                    Some(Num::R(r)) if r.n == 1 && r.d == 2 => self.ops.push(DOp::Sqrt),
                    Some(Num::R(r)) if r.n == -1 && r.d == 2 => {
                        self.ops.push(DOp::Sqrt);
                        self.ops.push(DOp::Powi(-1));
                    }
                    _ => {
                        self.emit(ex);
                        self.ops.push(DOp::Pow);
                    }
                }
            }
            Node::Fun(f, a) => {
                self.emit(a);
                self.ops.push(DOp::F(*f));
            }
            Node::Atan2(y, x) => {
                self.emit(y);
                self.emit(x);
                self.ops.push(DOp::Atan2);
            }
        }
    }
    pub fn eval(&self, load: &impl Fn(u32) -> Dd, st: &mut Vec<Dd>) -> Dd {
        st.clear();
        for op in &self.ops {
            match op {
                DOp::C(c) => st.push(*c),
                DOp::L(s) => st.push(load(*s)),
                DOp::Add(n) => {
                    let k = st.len() - *n as usize;
                    let s = st[k..].iter().fold(Dd::ZERO, |a, b| a + *b);
                    st.truncate(k);
                    st.push(s);
                }
                DOp::Mul(n) => {
                    let k = st.len() - *n as usize;
                    let s = st[k..].iter().fold(Dd::ONE, |a, b| a * *b);
                    st.truncate(k);
                    st.push(s);
                }
                DOp::Pow => {
                    let e = st.pop().unwrap();
                    let b = st.last_mut().unwrap();
                    *b = b.pow(e);
                }
                DOp::Powi(k) => {
                    let b = st.last_mut().unwrap();
                    *b = b.powi(*k);
                }
                DOp::Sqrt => {
                    let b = st.last_mut().unwrap();
                    *b = b.sqrt();
                }
                DOp::F(f) => {
                    let b = st.last_mut().unwrap();
                    *b = b.apply(*f);
                }
                DOp::Atan2 => {
                    let x = st.pop().unwrap();
                    let y = st.last_mut().unwrap();
                    *y = y.atan2(x);
                }
            }
        }
        st.pop().unwrap_or(Dd::from(f64::NAN))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Dd, b: Dd, tol: f64) -> bool {
        (a - b).abs().hi <= tol * (1.0 + b.hi.abs())
    }

    #[test]
    fn arithmetic_keeps_thirty_digits() {
        let third = Dd::ONE / Dd::from(3.0);
        assert!(close(third * Dd::from(3.0), Dd::ONE, 1e-31));
        let r = Dd::from(2.0).sqrt();
        assert!(close(r * r, Dd::from(2.0), 1e-31));
        assert!(close(Dd::from(1.7).powi(-3) * Dd::from(1.7).powi(3), Dd::ONE, 1e-30));
    }

    #[test]
    fn functions_keep_thirty_digits() {
        for x in [0.3, 1.1, -2.7, 5.9, 40.0] {
            let (s, c) = Dd::from(x).sin_cos();
            assert!(close(s * s + c * c, Dd::ONE, 1e-30), "{x}");
            assert!((s.hi - x.sin()).abs() < 1e-15 && (c.hi - x.cos()).abs() < 1e-15, "{x}");
            let a = s.atan2(c);
            let (s2, c2) = a.sin_cos();
            assert!(close(s2, s, 1e-30) && close(c2, c, 1e-30), "{x}");
        }
        for x in [0.2, 1.0, 7.5] {
            let d = Dd::from(x);
            assert!(close(d.exp().ln(), d, 1e-30), "{x}");
            assert!((d.exp().hi - x.exp()).abs() < 1e-14 * x.exp(), "{x}");
            let u = Dd::from(x / 8.0);
            assert!(close(u.apply(Fun::Asin).apply(Fun::Sin), u, 1e-30) && close(u.apply(Fun::Acos).apply(Fun::Cos), u, 1e-30), "{x}");
            assert!(close(u.apply(Fun::Atan).apply(Fun::Tan), u, 1e-30), "{x}");
        }
        let h = Dd::from(0.5);
        assert!(close(h.apply(Fun::Acos) * Dd::from(3.0), PI, 1e-30));
        assert!(close(h.apply(Fun::Asin) * Dd::from(6.0), PI, 1e-30));
        assert!(close(Dd::from(2.0).pow(h) * Dd::from(2.0).pow(h), Dd::from(2.0), 1e-30));
    }
}
