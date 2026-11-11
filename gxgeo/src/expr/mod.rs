//! Symbolic expression engine: exact rationals, canonical simplification,
//! differentiation, substitution, evaluation and compilation to bytecode.

pub mod bx;
pub mod parse;
pub mod poly;

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

/// Symbol reserved for the free variable `x` of user curves.
pub const X_SYM: u32 = u32::MAX;
/// Symbol reserved for the free variable `y` of user curves.
pub const Y_SYM: u32 = u32::MAX - 1;
/// Drag target symbols.
pub const TX_SYM: u32 = u32::MAX - 2;
pub const TY_SYM: u32 = u32::MAX - 3;
/// Temporary symbols used by algorithms start here.
pub const TMP_BASE: u32 = 0x8000_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Fun {
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sinh,
    Cosh,
    Tanh,
    Exp,
    Ln,
    Abs,
}

impl Fun {
    pub fn name(self) -> &'static str {
        match self {
            Fun::Sin => "sin",
            Fun::Cos => "cos",
            Fun::Tan => "tan",
            Fun::Asin => "asin",
            Fun::Acos => "acos",
            Fun::Atan => "atan",
            Fun::Sinh => "sinh",
            Fun::Cosh => "cosh",
            Fun::Tanh => "tanh",
            Fun::Exp => "exp",
            Fun::Ln => "ln",
            Fun::Abs => "abs",
        }
    }
    pub fn from_name(s: &str) -> Option<Fun> {
        Some(match s {
            "sin" => Fun::Sin,
            "cos" => Fun::Cos,
            "tan" => Fun::Tan,
            "asin" | "arcsin" => Fun::Asin,
            "acos" | "arccos" => Fun::Acos,
            "atan" | "arctan" => Fun::Atan,
            "sinh" => Fun::Sinh,
            "cosh" => Fun::Cosh,
            "tanh" => Fun::Tanh,
            "exp" => Fun::Exp,
            "ln" | "log" => Fun::Ln,
            "abs" => Fun::Abs,
            _ => return None,
        })
    }
    pub fn apply(self, x: f64) -> f64 {
        match self {
            Fun::Sin => x.sin(),
            Fun::Cos => x.cos(),
            Fun::Tan => x.tan(),
            Fun::Asin => clamp_unit(x).asin(),
            Fun::Acos => clamp_unit(x).acos(),
            Fun::Atan => x.atan(),
            Fun::Sinh => x.sinh(),
            Fun::Cosh => x.cosh(),
            Fun::Tanh => x.tanh(),
            Fun::Exp => x.exp(),
            Fun::Ln => x.ln(),
            Fun::Abs => x.abs(),
        }
    }
}

/// Clamps values within rounding noise of [−1, 1] for asin/acos.
fn clamp_unit(x: f64) -> f64 {
    if x.abs() > 1.0 && x.abs() < 1.0 + 1e-9 { x.signum() } else { x }
}

// ---------------------------------------------------------------- rationals

fn gcd(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rat {
    pub n: i128,
    pub d: i128,
}

const RAT_LIMIT: i128 = 1_000_000_000_000_000_000_000_000_000; // 1e30

impl Rat {
    pub fn new(n: i128, d: i128) -> Option<Rat> {
        if d == 0 {
            return None;
        }
        let g = gcd(n, d).max(1);
        let (mut n, mut d) = (n / g, d / g);
        if d < 0 {
            n = -n;
            d = -d;
        }
        if n.abs() > RAT_LIMIT || d > RAT_LIMIT {
            return None;
        }
        Some(Rat { n, d })
    }
    pub fn int(i: i128) -> Rat {
        Rat { n: i, d: 1 }
    }
    pub fn add(self, o: Rat) -> Option<Rat> {
        let n = self.n.checked_mul(o.d)?.checked_add(o.n.checked_mul(self.d)?)?;
        Rat::new(n, self.d.checked_mul(o.d)?)
    }
    pub fn mul(self, o: Rat) -> Option<Rat> {
        let g1 = gcd(self.n, o.d).max(1);
        let g2 = gcd(o.n, self.d).max(1);
        let n = (self.n / g1).checked_mul(o.n / g2)?;
        let d = (self.d / g2).checked_mul(o.d / g1)?;
        Rat::new(n, d)
    }
    pub fn neg(self) -> Rat {
        Rat { n: -self.n, d: self.d }
    }
    pub fn recip(self) -> Option<Rat> {
        Rat::new(self.d, self.n)
    }
    pub fn is_int(self) -> bool {
        self.d == 1
    }
    pub fn to_f64(self) -> f64 {
        self.n as f64 / self.d as f64
    }
    pub fn powi(self, k: i64) -> Option<Rat> {
        if k < 0 {
            return self.recip()?.powi(-k);
        }
        let mut r = Rat::int(1);
        let mut b = self;
        let mut k = k as u64;
        while k > 0 {
            if k & 1 == 1 {
                r = r.mul(b)?;
            }
            k >>= 1;
            if k > 0 {
                b = b.mul(b)?;
            }
        }
        Some(r)
    }
    /// Converts a float to an exact rational when it has a short decimal form.
    pub fn from_f64_nice(v: f64) -> Option<Rat> {
        if !v.is_finite() {
            return None;
        }
        let mut p: i128 = 1;
        for _ in 0..=7 {
            let s = v * p as f64;
            let r = s.round();
            if (s - r).abs() < 1e-9 * p as f64 && r.abs() < 1e18 {
                return Rat::new(r as i128, p);
            }
            p *= 10;
        }
        None
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Num {
    R(Rat),
    F(f64),
}

impl Num {
    pub fn int(i: i64) -> Num {
        Num::R(Rat::int(i as i128))
    }
    pub fn to_f64(self) -> f64 {
        match self {
            Num::R(r) => r.to_f64(),
            Num::F(f) => f,
        }
    }
    pub fn is_zero(self) -> bool {
        match self {
            Num::R(r) => r.n == 0,
            Num::F(f) => f == 0.0,
        }
    }
    pub fn is_one(self) -> bool {
        match self {
            Num::R(r) => r.n == 1 && r.d == 1,
            Num::F(f) => f == 1.0,
        }
    }
    pub fn is_neg(self) -> bool {
        self.to_f64() < 0.0
    }
    pub fn add(self, o: Num) -> Num {
        match (self, o) {
            (Num::R(a), Num::R(b)) => match a.add(b) {
                Some(r) => Num::R(r),
                None => Num::F(a.to_f64() + b.to_f64()),
            },
            _ => Num::F(self.to_f64() + o.to_f64()),
        }
    }
    pub fn mul(self, o: Num) -> Num {
        match (self, o) {
            (Num::R(a), Num::R(b)) => match a.mul(b) {
                Some(r) => Num::R(r),
                None => Num::F(a.to_f64() * b.to_f64()),
            },
            _ => Num::F(self.to_f64() * o.to_f64()),
        }
    }
    pub fn neg(self) -> Num {
        match self {
            Num::R(r) => Num::R(r.neg()),
            Num::F(f) => Num::F(-f),
        }
    }
    pub fn abs(self) -> Num {
        if self.is_neg() { self.neg() } else { self }
    }
    pub fn as_int(self) -> Option<i64> {
        match self {
            Num::R(r) if r.d == 1 && r.n.abs() < (1i128 << 62) => Some(r.n as i64),
            _ => None,
        }
    }
    fn total_cmp(&self, o: &Num) -> Ordering {
        match (self, o) {
            (Num::R(a), Num::R(b)) => match (a.n.checked_mul(b.d), b.n.checked_mul(a.d)) {
                (Some(x), Some(y)) => x.cmp(&y),
                _ => a.to_f64().total_cmp(&b.to_f64()),
            },
            (Num::R(_), Num::F(_)) => Ordering::Less,
            (Num::F(_), Num::R(_)) => Ordering::Greater,
            (Num::F(a), Num::F(b)) => a.total_cmp(b),
        }
    }
}

// ---------------------------------------------------------------- nodes

#[derive(Debug)]
pub enum Node {
    Num(Num),
    Pi,
    Sym(u32),
    Add(Vec<Expr>),
    Mul(Vec<Expr>),
    Pow(Expr, Expr),
    Fun(Fun, Expr),
    Atan2(Expr, Expr),
}

#[derive(Clone, Debug)]
pub struct Expr(pub Rc<Node>);

impl Expr {
    fn raw(n: Node) -> Expr {
        // building expressions is most of the symbolic work
        charge(1);
        Expr(Rc::new(n))
    }
    pub fn node(&self) -> &Node {
        &self.0
    }
    pub fn as_num(&self) -> Option<Num> {
        match self.node() {
            Node::Num(n) => Some(*n),
            _ => None,
        }
    }
    pub fn as_rat(&self) -> Option<Rat> {
        match self.node() {
            Node::Num(Num::R(r)) => Some(*r),
            _ => None,
        }
    }
    pub fn as_int(&self) -> Option<i64> {
        self.as_num().and_then(|n| n.as_int())
    }
    pub fn is_zero(&self) -> bool {
        self.as_num().is_some_and(|n| n.is_zero())
    }
    pub fn is_one(&self) -> bool {
        self.as_num().is_some_and(|n| n.is_one())
    }
    pub fn as_sym(&self) -> Option<u32> {
        match self.node() {
            Node::Sym(s) => Some(*s),
            _ => None,
        }
    }
    /// Number of nodes in the tree.
    pub fn size(&self) -> usize {
        match self.node() {
            Node::Num(_) | Node::Pi | Node::Sym(_) => 1,
            Node::Add(v) | Node::Mul(v) => 1 + v.iter().map(|e| e.size()).sum::<usize>(),
            Node::Pow(a, b) | Node::Atan2(a, b) => 1 + a.size() + b.size(),
            Node::Fun(_, a) => 1 + a.size(),
        }
    }
    /// Splits off the numeric coefficient: `c * rest`.
    pub fn split_coeff(&self) -> (Num, Expr) {
        match self.node() {
            Node::Num(n) => (*n, one()),
            Node::Mul(fs) => {
                if let Node::Num(n) = fs[0].node() {
                    let rest = if fs.len() == 2 {
                        fs[1].clone()
                    } else {
                        Expr::raw(Node::Mul(fs[1..].to_vec()))
                    };
                    (*n, rest)
                } else {
                    (Num::int(1), self.clone())
                }
            }
            _ => (Num::int(1), self.clone()),
        }
    }
    /// True when the expression has a negative leading coefficient.
    pub fn is_negative_form(&self) -> bool {
        match self.node() {
            Node::Num(n) => n.is_neg(),
            Node::Mul(fs) => fs[0].as_num().is_some_and(|n| n.is_neg()),
            _ => false,
        }
    }
}

fn rank(n: &Node) -> u8 {
    match n {
        Node::Num(_) => 0,
        Node::Pi => 1,
        Node::Sym(_) => 2,
        Node::Pow(..) => 3,
        Node::Mul(_) => 4,
        Node::Add(_) => 5,
        Node::Fun(..) => 6,
        Node::Atan2(..) => 7,
    }
}

impl Ord for Expr {
    fn cmp(&self, o: &Expr) -> Ordering {
        if Rc::ptr_eq(&self.0, &o.0) {
            return Ordering::Equal;
        }
        // (sorting terms into canonical order compares whole trees)
        charge_small(2);
        match (self.node(), o.node()) {
            (Node::Num(a), Node::Num(b)) => a.total_cmp(b),
            (Node::Pi, Node::Pi) => Ordering::Equal,
            (Node::Sym(a), Node::Sym(b)) => a.cmp(b),
            (Node::Add(a), Node::Add(b)) | (Node::Mul(a), Node::Mul(b)) => a.cmp(b),
            (Node::Pow(a1, b1), Node::Pow(a2, b2)) | (Node::Atan2(a1, b1), Node::Atan2(a2, b2)) => {
                a1.cmp(a2).then_with(|| b1.cmp(b2))
            }
            (Node::Fun(f1, a1), Node::Fun(f2, a2)) => f1.cmp(f2).then_with(|| a1.cmp(a2)),
            (a, b) => rank(a).cmp(&rank(b)),
        }
    }
}
impl PartialOrd for Expr {
    fn partial_cmp(&self, o: &Expr) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl PartialEq for Expr {
    fn eq(&self, o: &Expr) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for Expr {}

// ---------------------------------------------------------------- assumptions

thread_local! {
    static POSITIVE: RefCell<HashSet<u32>> = RefCell::new(HashSet::new());
    static ANGLES: RefCell<HashSet<u32>> = RefCell::new(HashSet::new());
}

thread_local! {
    static DEADLINE: std::cell::Cell<Option<web_time::Instant>> = const { std::cell::Cell::new(None) };
    /// Set from another thread to stop the work on this one as if its deadline had passed.
    static CANCEL: RefCell<Option<std::sync::Arc<std::sync::atomic::AtomicBool>>> = const { RefCell::new(None) };
}

/// Installs (or removes) a flag that, once set, makes `past_deadline` true on this thread.
pub fn set_cancel(flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>) {
    CANCEL.with(|c| *c.borrow_mut() = flag);
}

thread_local! {
    /// Work done on this thread, in polls of `past_deadline` and explicit charges.
    static TICKS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static ORIGIN: web_time::Instant = web_time::Instant::now();
    /// Real time after which work stops whatever the work clock says.
    static HARD: std::cell::Cell<Option<web_time::Instant>> = const { std::cell::Cell::new(None) };
}

/// Duration of one tick of the work clock: about the real time between two
/// polls of `past_deadline` in the derivation's loops on an idle machine.
const TICK_NS: u64 = 200;

/// The work clock. Deadlines are points on this clock, which advances with
/// the work done (each poll of `past_deadline`, each `charge`) and not with
/// real time: the same derivation stops at the same point however busy the
/// machine is, so its result does not depend on the load.
pub fn work_now() -> web_time::Instant {
    ORIGIN.with(|o| *o) + std::time::Duration::from_nanos(TICKS.with(|t| t.get()).saturating_mul(TICK_NS))
}

/// Advances the work clock by `ticks`, for work done between polls.
pub fn charge(ticks: u64) {
    TICKS.with(|t| t.set(t.get().saturating_add(ticks)));
}

thread_local! {
    static SUB_TICKS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Advances the work clock by n/32 of a tick, for very small steps.
#[inline]
fn charge_small(n: u32) {
    SUB_TICKS.with(|c| {
        let s = c.get() + n;
        if s >= 32 {
            charge((s / 32) as u64);
            c.set(s % 32);
        } else {
            c.set(s);
        }
    });
}

/// Ticks of the work clock so far on this thread.
pub fn work_ticks() -> u64 {
    TICKS.with(|t| t.get())
}

/// Sets (or clears) the point on the work clock after which work stops early.
pub fn set_deadline(d: Option<web_time::Instant>) {
    DEADLINE.with(|c| c.set(d));
}

/// Sets (or clears) a real-time limit that stops work even when the work
/// clock runs far slower than real time (a safeguard, not a budget).
pub fn set_hard_deadline(d: Option<web_time::Instant>) {
    HARD.with(|c| c.set(d));
}

/// Whether the flag installed with `set_cancel` is set.
pub fn cancelled() -> bool {
    CANCEL.with(|c| c.borrow().as_ref().is_some_and(|f| f.load(std::sync::atomic::Ordering::Relaxed)))
}

pub fn past_deadline() -> bool {
    let t = TICKS.with(|t| {
        t.set(t.get() + 1);
        t.get()
    });
    cancelled()
        || DEADLINE.with(|c| c.get()).is_some_and(|d| work_now() > d)
        || (t % 64 == 0 && HARD.with(|c| c.get()).is_some_and(|h| web_time::Instant::now() > h))
}

/// Declares symbols known to be non-negative, and symbols known to lie in [0, π].
pub fn set_assumptions(pos: HashSet<u32>, ang: HashSet<u32>) {
    POSITIVE.with(|p| *p.borrow_mut() = pos);
    ANGLES.with(|p| *p.borrow_mut() = ang);
}

pub fn is_nonneg(e: &Expr) -> bool {
    match e.node() {
        Node::Num(n) => !n.is_neg(),
        Node::Pi => true,
        Node::Sym(s) => POSITIVE.with(|p| p.borrow().contains(s)),
        Node::Pow(b, ex) => {
            if let Some(Num::R(r)) = ex.as_num() {
                if r.d != 1 || r.n % 2 == 0 {
                    return true;
                }
            }
            is_nonneg(b)
        }
        Node::Mul(fs) => fs.iter().all(is_nonneg),
        Node::Add(ts) => ts.iter().all(is_nonneg),
        Node::Fun(Fun::Abs | Fun::Exp | Fun::Cosh | Fun::Acos, _) => true,
        Node::Fun(Fun::Sin, a) => a
            .as_sym()
            .is_some_and(|s| ANGLES.with(|p| p.borrow().contains(&s))),
        _ => false,
    }
}

// ---------------------------------------------------------------- constructors

pub fn num(n: Num) -> Expr {
    Expr::raw(Node::Num(n))
}
pub fn int(i: i64) -> Expr {
    num(Num::int(i))
}
pub fn rat(n: i64, d: i64) -> Expr {
    num(Num::R(Rat::new(n as i128, d as i128).unwrap()))
}
pub fn float(f: f64) -> Expr {
    if let Some(r) = Rat::from_f64_nice(f) {
        return num(Num::R(r));
    }
    num(Num::F(f))
}
/// Best simple closed form of a float: short decimal, small fraction,
/// rational multiple of π, or square/cube root of a small fraction.
pub fn nice_const(v: f64) -> Expr {
    if let Some(r) = Rat::from_f64_nice(v) {
        return num(Num::R(r));
    }
    let tol = 1e-12 * (1.0 + v.abs());
    let frac = |x: f64, maxd: i128| -> Option<Rat> {
        // continued fraction approximation
        let (mut h0, mut h1, mut k0, mut k1) = (0i128, 1i128, 1i128, 0i128);
        let mut y = x;
        for _ in 0..40 {
            let a = y.floor();
            if a.abs() > 1e12 {
                break;
            }
            let a = a as i128;
            let (h2, k2) = (a * h1 + h0, a * k1 + k0);
            if k2 > maxd {
                break;
            }
            if ((h2 as f64) / (k2 as f64) - x).abs() <= 1e-12 * (1.0 + x.abs()) {
                return Rat::new(h2, k2);
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
    };
    if let Some(r) = frac(v, 1000) {
        return num(Num::R(r));
    }
    if let Some(r) = frac(v / std::f64::consts::PI, 360) {
        return mul(vec![num(Num::R(r)), pi()]);
    }
    for k in [2i64, 3] {
        let p = v.abs().powi(k as i32);
        if let Some(r) = Rat::from_f64_nice(p).or_else(|| frac(p, 1000)) {
            let e = pow(num(Num::R(r)), rat(1, k));
            if (eval(&e, &|_| 0.0) - v.abs()).abs() <= tol {
                return if v < 0.0 { neg(e) } else { e };
            }
        }
    }
    num(Num::F(v))
}

pub fn float_raw(f: f64) -> Expr {
    num(Num::F(f))
}
pub fn zero() -> Expr {
    int(0)
}
pub fn one() -> Expr {
    int(1)
}
pub fn sym(s: u32) -> Expr {
    Expr::raw(Node::Sym(s))
}
pub fn pi() -> Expr {
    Expr::raw(Node::Pi)
}

pub fn add(terms: Vec<Expr>) -> Expr {
    let mut flat: Vec<Expr> = Vec::with_capacity(terms.len());
    for t in terms {
        match t.node() {
            Node::Add(ts) => flat.extend(ts.iter().cloned()),
            _ => flat.push(t),
        }
    }
    if flat.len() == 1 {
        return flat.pop().unwrap();
    }
    let mut cst = Num::int(0);
    let mut map: BTreeMap<Expr, Num> = BTreeMap::new();
    for t in flat {
        if let Node::Num(n) = t.node() {
            cst = cst.add(*n);
            continue;
        }
        let (c, rest) = t.split_coeff();
        let e = map.entry(rest).or_insert(Num::int(0));
        *e = e.add(c);
    }
    let mut out = Vec::with_capacity(map.len() + 1);
    if !cst.is_zero() {
        out.push(num(cst));
    }
    for (rest, c) in map {
        if c.is_zero() {
            continue;
        }
        if c.is_one() {
            out.push(rest);
        } else {
            match rest.node() {
                Node::Mul(fs) => {
                    let mut v = Vec::with_capacity(fs.len() + 1);
                    v.push(num(c));
                    v.extend(fs.iter().cloned());
                    out.push(Expr::raw(Node::Mul(v)));
                }
                _ => out.push(Expr::raw(Node::Mul(vec![num(c), rest]))),
            }
        }
    }
    // π − acos(x) = acos(−x)
    if let Some(ip) = out.iter().position(|t| matches!(t.node(), Node::Pi)) {
        let ia = out.iter().position(|t| match t.node() {
            Node::Mul(fs) => fs.len() == 2 && fs[0].as_num().is_some_and(|n| n.neg().is_one()) && matches!(fs[1].node(), Node::Fun(Fun::Acos, _)),
            _ => false,
        });
        if let Some(ia) = ia {
            let Node::Mul(fs) = out[ia].node() else { unreachable!() };
            let Node::Fun(_, x) = fs[1].node() else { unreachable!() };
            let r = fun(Fun::Acos, mul(vec![int(-1), x.clone()]));
            let (a, b) = (ip.max(ia), ip.min(ia));
            out.remove(a);
            out.remove(b);
            out.push(r);
            return add(out);
        }
    }
    match out.len() {
        0 => zero(),
        1 => out.pop().unwrap(),
        _ => Expr::raw(Node::Add(out)),
    }
}

fn base_exp(f: &Expr) -> (Expr, Expr) {
    match f.node() {
        Node::Pow(b, e) => (b.clone(), e.clone()),
        _ => (f.clone(), one()),
    }
}

/// Negates each term of a sum.
fn neg_sum(e: &Expr) -> Expr {
    match e.node() {
        Node::Add(ts) => add(ts.iter().map(|t| mul(vec![int(-1), t.clone()])).collect()),
        _ => mul(vec![int(-1), e.clone()]),
    }
}

/// True for a sum whose first canonical term has a negative coefficient.
fn leading_negative(e: &Expr) -> bool {
    match e.node() {
        Node::Add(ts) => ts[0].is_negative_form(),
        _ => false,
    }
}

pub fn mul(factors: Vec<Expr>) -> Expr {
    let mut flat: Vec<Expr> = Vec::with_capacity(factors.len());
    for f in factors {
        match f.node() {
            Node::Mul(fs) => flat.extend(fs.iter().cloned()),
            _ => flat.push(f),
        }
    }
    if flat.len() == 1 {
        return flat.pop().unwrap();
    }
    let mut coeff = Num::int(1);
    let mut map: BTreeMap<Expr, Vec<Expr>> = BTreeMap::new();
    for f in flat {
        if let Node::Num(n) = f.node() {
            coeff = coeff.mul(*n);
            continue;
        }
        let (mut b, e) = base_exp(&f);
        if let Some(k) = e.as_int() {
            if leading_negative(&b) {
                b = neg_sum(&b);
                if k % 2 != 0 {
                    coeff = coeff.neg();
                }
            }
        }
        map.entry(b).or_default().push(e);
    }
    if coeff.is_zero() {
        return zero();
    }
    let mut out: Vec<Expr> = Vec::with_capacity(map.len());
    let mut redo: Vec<Expr> = Vec::new();
    for (b, es) in map {
        let p = if es.len() == 1 && !es[0].is_one() {
            // unchanged single factor
            let e = es.into_iter().next().unwrap();
            if let (Node::Num(_), Node::Num(_)) = (b.node(), e.node()) {
                pow(b, e)
            } else {
                Expr::raw(Node::Pow(b, e))
            }
        } else if es.len() == 1 {
            b
        } else {
            pow(b, add(es))
        };
        match p.node() {
            Node::Num(n) => coeff = coeff.mul(*n),
            Node::Mul(_) => redo.push(p),
            _ => out.push(p),
        }
    }
    if !redo.is_empty() {
        out.extend(redo);
        out.push(num(coeff));
        return mul(out);
    }
    if coeff.is_zero() {
        return zero();
    }
    if out.is_empty() {
        return num(coeff);
    }
    if out.len() == 1 && !coeff.is_one() {
        if let Node::Add(ts) = out[0].node() {
            return add(ts.iter().map(|t| mul(vec![num(coeff), t.clone()])).collect());
        }
    }
    out.sort_by(|x, y| {
        let (bx, ex) = base_exp(x);
        let (by, ey) = base_exp(y);
        bx.cmp(&by).then_with(|| ex.cmp(&ey))
    });
    if !coeff.is_one() {
        out.insert(0, num(coeff));
    }
    if out.len() == 1 {
        out.pop().unwrap()
    } else {
        Expr::raw(Node::Mul(out))
    }
}

/// Writes |n| = a^q * b with a maximal, for small factors.
fn root_extract(n: i128, q: u32) -> (i128, i128) {
    let mut a: i128 = 1;
    let mut b = n;
    let mut f: i128 = 2;
    let mut iters = 0;
    while iters < 200_000 {
        let fq = match f.checked_pow(q) {
            Some(v) => v,
            None => break,
        };
        if fq > b {
            break;
        }
        if b % fq == 0 {
            a *= f;
            b /= fq;
        } else {
            f += 1;
        }
        iters += 1;
    }
    (a, b)
}

fn pow_num(b: Num, e: Num) -> Option<Expr> {
    match (b, e) {
        (Num::R(br), Num::R(er)) => {
            if er.d == 1 {
                if er.n.abs() > 512 {
                    return Some(num(Num::F(br.to_f64().powf(er.to_f64()))));
                }
                if br.n == 0 && er.n < 0 {
                    return None;
                }
                return Some(match br.powi(er.n as i64) {
                    Some(r) => num(Num::R(r)),
                    None => num(Num::F(br.to_f64().powf(er.to_f64()))),
                });
            }
            let q = er.d;
            if br.n == 0 {
                return if er.n > 0 { Some(zero()) } else { None };
            }
            if q > 12 {
                return None;
            }
            let mut sign = 1i128;
            if br.n < 0 {
                if q % 2 == 0 {
                    return None;
                }
                sign = -1;
            }
            let p = er.n;
            let (a, bb) = root_extract(br.n.abs(), q as u32);
            let (c, dd) = root_extract(br.d, q as u32);
            let k = p.div_euclid(q);
            let r = p.rem_euclid(q);
            let ac = Rat::new(a, c)?.powi(p as i64)?;
            let bd = Rat::new(bb, dd)?.powi(k as i64)?;
            let mut coeff = ac.mul(bd)?;
            if sign < 0 && p % 2 != 0 {
                coeff = coeff.neg();
            }
            let mut fs = vec![num(Num::R(coeff))];
            if r != 0 {
                let ex = Rat::new(r, q)?;
                if bb != 1 {
                    fs.push(Expr::raw(Node::Pow(num(Num::R(Rat::int(bb))), num(Num::R(ex)))));
                }
                if dd != 1 {
                    fs.push(Expr::raw(Node::Pow(
                        num(Num::R(Rat::int(dd))),
                        num(Num::R(ex.neg())),
                    )));
                }
            }
            if fs.len() == 1 {
                return Some(fs.pop().unwrap());
            }
            if fs[0].is_one() {
                fs.remove(0);
            }
            if fs.len() == 1 {
                return Some(fs.pop().unwrap());
            }
            Some(Expr::raw(Node::Mul(fs)))
        }
        _ => {
            let (x, y) = (b.to_f64(), e.to_f64());
            if x < 0.0 && y.fract() != 0.0 {
                return None;
            }
            Some(num(Num::F(x.powf(y))))
        }
    }
}

pub fn pow(b: Expr, e: Expr) -> Expr {
    if let Some(en) = e.as_num() {
        if en.is_zero() {
            return one();
        }
        if en.is_one() {
            return b;
        }
        if let Some(bn) = b.as_num() {
            if let Some(r) = pow_num(bn, en) {
                return r;
            }
        }
    }
    if let Some(bn) = b.as_num() {
        if bn.is_one() {
            return one();
        }
        if bn.is_zero() && e.as_num().is_some_and(|n| !n.is_neg()) {
            return zero();
        }
    }
    let e_int = e.as_int();
    if let Some(k) = e_int {
        if leading_negative(&b) {
            let nb = neg_sum(&b);
            return if k % 2 == 0 { pow(nb, e) } else { mul(vec![int(-1), pow(nb, e)]) };
        }
    }
    match b.node() {
        Node::Pow(bb, e1) => {
            if e_int.is_some() || is_nonneg(bb) {
                return pow(bb.clone(), mul(vec![e1.clone(), e.clone()]));
            }
            if let Some(k) = e1.as_int() {
                if k % 2 == 0 {
                    return pow(fun(Fun::Abs, bb.clone()), mul(vec![e1.clone(), e.clone()]));
                }
                // odd powers keep the sign, so the real domain is unchanged
                return pow(bb.clone(), mul(vec![e1.clone(), e.clone()]));
            }
        }
        Node::Mul(fs) => {
            if e_int.is_some() {
                return mul(fs.iter().map(|f| pow(f.clone(), e.clone())).collect());
            }
            if e.as_num().is_some() {
                let mut nn = Vec::new();
                let mut rest = Vec::new();
                let mut neg = false;
                for f in fs {
                    if let Some(n) = f.as_num() {
                        if n.is_neg() {
                            neg = true;
                        }
                        let a = n.abs();
                        if !a.is_one() {
                            nn.push(num(a));
                        }
                    } else if is_nonneg(f) {
                        nn.push(f.clone());
                    } else {
                        rest.push(f.clone());
                    }
                }
                if !nn.is_empty() {
                    let mut out: Vec<Expr> = nn.into_iter().map(|f| pow(f, e.clone())).collect();
                    if neg {
                        rest.insert(0, int(-1));
                    }
                    if !rest.is_empty() {
                        out.push(pow(mul(rest), e.clone()));
                    }
                    return mul(out);
                }
            }
        }
        Node::Fun(Fun::Abs, x) => {
            if e_int.is_some_and(|k| k % 2 == 0) {
                return pow(x.clone(), e);
            }
        }
        Node::Fun(Fun::Exp, x) => {
            return fun(Fun::Exp, mul(vec![x.clone(), e]));
        }
        _ => {}
    }
    Expr::raw(Node::Pow(b, e))
}

fn sqrt_e(e: Expr) -> Expr {
    pow(e, rat(1, 2))
}

/// cos(kπ) for k with denominator dividing 12.
fn cos_pi_multiple(k: Rat) -> Option<Expr> {
    let twelfths = k.mul(Rat::int(12))?;
    if !twelfths.is_int() {
        return None;
    }
    let m = twelfths.n.rem_euclid(24);
    let s3 = || mul(vec![rat(1, 2), sqrt_e(int(3))]);
    let s2 = || mul(vec![rat(1, 2), sqrt_e(int(2))]);
    let neg = |e: Expr| mul(vec![int(-1), e]);
    Some(match m {
        0 => int(1),
        2 | 22 => s3(),
        3 | 21 => s2(),
        4 | 20 => rat(1, 2),
        6 | 18 => int(0),
        8 | 16 => rat(-1, 2),
        9 | 15 => neg(s2()),
        10 | 14 => neg(s3()),
        12 => int(-1),
        _ => return None,
    })
}

fn pi_multiple(a: &Expr) -> Option<Rat> {
    match a.node() {
        Node::Pi => Some(Rat::int(1)),
        Node::Mul(fs) if fs.len() == 2 => {
            if let (Some(r), Node::Pi) = (fs[0].as_rat(), fs[1].node()) {
                Some(r)
            } else {
                None
            }
        }
        Node::Num(Num::R(r)) if r.n == 0 => Some(*r),
        _ => None,
    }
}

pub fn fun(f: Fun, a: Expr) -> Expr {
    if let Some(Num::F(x)) = a.as_num() {
        return num(Num::F(f.apply(x)));
    }
    if let Some(r) = a.as_rat() {
        let z = r.n == 0;
        let o = r.n == 1 && r.d == 1;
        let mo = r.n == -1 && r.d == 1;
        match f {
            Fun::Sin | Fun::Tan | Fun::Asin | Fun::Atan | Fun::Sinh | Fun::Tanh if z => return zero(),
            Fun::Cos | Fun::Cosh | Fun::Exp if z => return one(),
            Fun::Acos if z => return mul(vec![rat(1, 2), pi()]),
            Fun::Ln if o => return zero(),
            Fun::Acos if o => return zero(),
            Fun::Acos if mo => return pi(),
            Fun::Asin if o => return mul(vec![rat(1, 2), pi()]),
            Fun::Asin if mo => return mul(vec![rat(-1, 2), pi()]),
            Fun::Atan if o => return mul(vec![rat(1, 4), pi()]),
            Fun::Atan if mo => return mul(vec![rat(-1, 4), pi()]),
            Fun::Abs => return num(Num::R(if r.n < 0 { r.neg() } else { r })),
            _ => {}
        }
    }
    if let Some(k) = pi_multiple(&a) {
        match f {
            Fun::Cos => {
                if let Some(v) = cos_pi_multiple(k) {
                    return v;
                }
            }
            Fun::Sin => {
                if let Some(v) = k.add(Rat::new(-1, 2).unwrap()).and_then(cos_pi_multiple) {
                    return v;
                }
            }
            Fun::Tan => {
                if let (Some(s), Some(c)) = (
                    k.add(Rat::new(-1, 2).unwrap()).and_then(cos_pi_multiple),
                    cos_pi_multiple(k),
                ) {
                    if !c.is_zero() {
                        return mul(vec![s, pow(c, int(-1))]);
                    }
                }
            }
            _ => {}
        }
    }
    // shifts by multiples of π/2: sin(x + kπ/2) and cos(x + kπ/2) become ±sin x
    // or ±cos x, tan(x + kπ) becomes tan x, so equal angles give equal atoms
    if let (Fun::Sin | Fun::Cos | Fun::Tan, Node::Add(ts)) = (f, a.node()) {
        if let Some(i) = ts.iter().position(|t| pi_multiple(t).is_some_and(|k| k.n != 0)) {
            let k = pi_multiple(&ts[i]).unwrap();
            let halves = k.mul(Rat::int(2)).filter(|h| h.is_int()).map(|h| h.n.rem_euclid(4));
            if let Some(h) = halves {
                let mut rest = ts.clone();
                rest.remove(i);
                let x = add(rest);
                let neg = |e: Expr| mul(vec![int(-1), e]);
                match (f, h) {
                    (Fun::Sin, 0) | (Fun::Cos, 3) => return fun(Fun::Sin, x),
                    (Fun::Sin, 2) | (Fun::Cos, 1) => return neg(fun(Fun::Sin, x)),
                    (Fun::Cos, 0) | (Fun::Sin, 1) => return fun(Fun::Cos, x),
                    (Fun::Cos, 2) | (Fun::Sin, 3) => return neg(fun(Fun::Cos, x)),
                    (Fun::Tan, 0 | 2) => return fun(Fun::Tan, x),
                    _ => {}
                }
            }
        }
    }
    // inverse pairs
    if let Node::Fun(g, x) = a.node() {
        let x = x.clone();
        let one_minus_sq = || sqrt_e(add(vec![one(), mul(vec![int(-1), pow(x.clone(), int(2))])]));
        match (f, g) {
            (Fun::Sin, Fun::Asin) | (Fun::Cos, Fun::Acos) | (Fun::Tan, Fun::Atan) => return x,
            (Fun::Exp, Fun::Ln) | (Fun::Ln, Fun::Exp) => return x,
            (Fun::Sinh, Fun::Asin) => {}
            (Fun::Cos, Fun::Asin) | (Fun::Sin, Fun::Acos) => return one_minus_sq(),
            (Fun::Cos, Fun::Atan) => {
                return pow(add(vec![one(), pow(x, int(2))]), rat(-1, 2));
            }
            (Fun::Sin, Fun::Atan) => {
                return mul(vec![x.clone(), pow(add(vec![one(), pow(x, int(2))]), rat(-1, 2))]);
            }
            (Fun::Abs, Fun::Abs) => return a,
            _ => {}
        }
    }
    match f {
        Fun::Abs => {
            if is_nonneg(&a) {
                return a;
            }
            if a.is_negative_form() {
                return fun(Fun::Abs, mul(vec![int(-1), a]));
            }
            if let Node::Mul(fs) = a.node() {
                let (pos, rest): (Vec<Expr>, Vec<Expr>) = fs.iter().cloned().partition(is_nonneg);
                if !pos.is_empty() && !rest.is_empty() {
                    let mut v = pos;
                    v.push(fun(Fun::Abs, mul(rest)));
                    return mul(v);
                }
            }
        }
        Fun::Sin | Fun::Tan | Fun::Asin | Fun::Atan | Fun::Sinh | Fun::Tanh => {
            if a.is_negative_form() {
                return mul(vec![int(-1), fun(f, mul(vec![int(-1), a]))]);
            }
        }
        Fun::Cos | Fun::Cosh => {
            if a.is_negative_form() {
                return fun(f, mul(vec![int(-1), a]));
            }
        }
        Fun::Ln => {
            if let Node::Pow(b, e) = a.node() {
                if is_nonneg(b) {
                    return mul(vec![e.clone(), fun(Fun::Ln, b.clone())]);
                }
            }
        }
        _ => {}
    }
    Expr::raw(Node::Fun(f, a))
}

pub fn atan2(y: Expr, x: Expr) -> Expr {
    if let (Some(a), Some(b)) = (y.as_num(), x.as_num()) {
        if matches!(a, Num::F(_)) || matches!(b, Num::F(_)) {
            return num(Num::F(a.to_f64().atan2(b.to_f64())));
        }
        if a.is_zero() && !b.is_neg() && !b.is_zero() {
            return zero();
        }
        if a.is_zero() && b.is_neg() {
            return pi();
        }
        if b.is_zero() && !a.is_zero() {
            return mul(vec![rat(if a.is_neg() { -1 } else { 1 }, 2), pi()]);
        }
    }
    if y.is_zero() && is_nonneg(&x) {
        return zero();
    }
    if is_nonneg(&x) && !x.is_zero() {
        return fun(Fun::Atan, mul(vec![y, pow(x, int(-1))]));
    }
    Expr::raw(Node::Atan2(y, x))
}

pub fn neg(a: Expr) -> Expr {
    mul(vec![int(-1), a])
}
pub fn sub(a: Expr, b: Expr) -> Expr {
    add(vec![a, neg(b)])
}
pub fn div(a: Expr, b: Expr) -> Expr {
    mul(vec![a, pow(b, int(-1))])
}
pub fn sqrt(a: Expr) -> Expr {
    sqrt_e(a)
}
pub fn sq(a: Expr) -> Expr {
    pow(a, int(2))
}
pub fn cos(a: Expr) -> Expr {
    fun(Fun::Cos, a)
}
pub fn sin(a: Expr) -> Expr {
    fun(Fun::Sin, a)
}

macro_rules! binop {
    ($tr:ident, $m:ident, $f:expr) => {
        impl std::ops::$tr<Expr> for Expr {
            type Output = Expr;
            fn $m(self, o: Expr) -> Expr {
                ($f)(self, o)
            }
        }
        impl std::ops::$tr<&Expr> for Expr {
            type Output = Expr;
            fn $m(self, o: &Expr) -> Expr {
                ($f)(self, o.clone())
            }
        }
        impl std::ops::$tr<Expr> for &Expr {
            type Output = Expr;
            fn $m(self, o: Expr) -> Expr {
                ($f)(self.clone(), o)
            }
        }
        impl std::ops::$tr<&Expr> for &Expr {
            type Output = Expr;
            fn $m(self, o: &Expr) -> Expr {
                ($f)(self.clone(), o.clone())
            }
        }
    };
}
binop!(Add, add, |a, b| add(vec![a, b]));
binop!(Sub, sub, sub);
binop!(Mul, mul, |a, b| mul(vec![a, b]));
binop!(Div, div, div);

impl std::ops::Neg for Expr {
    type Output = Expr;
    fn neg(self) -> Expr {
        neg(self)
    }
}
impl std::ops::Neg for &Expr {
    type Output = Expr;
    fn neg(self) -> Expr {
        neg(self.clone())
    }
}

// ---------------------------------------------------------------- queries

pub fn contains(e: &Expr, v: u32) -> bool {
    match e.node() {
        Node::Num(_) | Node::Pi => false,
        Node::Sym(s) => *s == v,
        Node::Add(ts) | Node::Mul(ts) => ts.iter().any(|t| contains(t, v)),
        Node::Pow(a, b) | Node::Atan2(a, b) => contains(a, v) || contains(b, v),
        Node::Fun(_, a) => contains(a, v),
    }
}

pub fn contains_any(e: &Expr, vs: &[u32]) -> bool {
    vs.iter().any(|v| contains(e, *v))
}

pub fn syms(e: &Expr, out: &mut HashSet<u32>) {
    match e.node() {
        Node::Num(_) | Node::Pi => {}
        Node::Sym(s) => {
            out.insert(*s);
        }
        Node::Add(ts) | Node::Mul(ts) => {
            charge_small(ts.len() as u32);
            ts.iter().for_each(|t| syms(t, out))
        }
        Node::Pow(a, b) | Node::Atan2(a, b) => {
            syms(a, out);
            syms(b, out);
        }
        Node::Fun(_, a) => syms(a, out),
    }
}

pub fn sym_set(e: &Expr) -> HashSet<u32> {
    charge_small(8);
    let mut s = HashSet::new();
    syms(e, &mut s);
    s
}

pub fn has_float(e: &Expr) -> bool {
    match e.node() {
        Node::Num(Num::F(_)) => true,
        Node::Num(_) | Node::Pi | Node::Sym(_) => false,
        Node::Add(ts) | Node::Mul(ts) => ts.iter().any(has_float),
        Node::Pow(a, b) | Node::Atan2(a, b) => has_float(a) || has_float(b),
        Node::Fun(_, a) => has_float(a),
    }
}

// ---------------------------------------------------------------- transforms

/// Rebuilds the expression with symbols replaced by `f`, re-simplifying.
pub fn subst(e: &Expr, f: &dyn Fn(u32) -> Option<Expr>) -> Expr {
    let mut memo: HashMap<*const Node, Expr> = HashMap::new();
    subst_rec(e, f, &mut memo)
}

fn subst_rec(e: &Expr, f: &dyn Fn(u32) -> Option<Expr>, memo: &mut HashMap<*const Node, Expr>) -> Expr {
    let key = Rc::as_ptr(&e.0);
    if let Some(r) = memo.get(&key) {
        return r.clone();
    }
    let r = match e.node() {
        Node::Num(_) | Node::Pi => e.clone(),
        Node::Sym(s) => f(*s).unwrap_or_else(|| e.clone()),
        Node::Add(ts) | Node::Mul(ts) => {
            let new: Vec<Expr> = ts.iter().map(|t| subst_rec(t, f, memo)).collect();
            if new.iter().zip(ts).all(|(a, b)| Rc::ptr_eq(&a.0, &b.0)) {
                e.clone()
            } else if matches!(e.node(), Node::Add(_)) {
                add(new)
            } else {
                mul(new)
            }
        }
        Node::Pow(a, b) => {
            let (na, nb) = (subst_rec(a, f, memo), subst_rec(b, f, memo));
            if Rc::ptr_eq(&na.0, &a.0) && Rc::ptr_eq(&nb.0, &b.0) {
                e.clone()
            } else {
                pow(na, nb)
            }
        }
        Node::Atan2(a, b) => {
            let (na, nb) = (subst_rec(a, f, memo), subst_rec(b, f, memo));
            if Rc::ptr_eq(&na.0, &a.0) && Rc::ptr_eq(&nb.0, &b.0) {
                e.clone()
            } else {
                atan2(na, nb)
            }
        }
        Node::Fun(g, a) => {
            let na = subst_rec(a, f, memo);
            if Rc::ptr_eq(&na.0, &a.0) { e.clone() } else { fun(*g, na) }
        }
    };
    memo.insert(key, r.clone());
    r
}

pub fn subst_map(e: &Expr, m: &HashMap<u32, Expr>) -> Expr {
    if m.is_empty() {
        return e.clone();
    }
    subst(e, &|s| m.get(&s).cloned())
}

/// Replaces every occurrence of the subexpression `pat` with `rep`.
pub fn replace(e: &Expr, pat: &Expr, rep: &Expr) -> Expr {
    if e == pat {
        return rep.clone();
    }
    match e.node() {
        Node::Num(_) | Node::Pi | Node::Sym(_) => e.clone(),
        Node::Add(ts) => add(ts.iter().map(|t| replace(t, pat, rep)).collect()),
        Node::Mul(ts) => mul(ts.iter().map(|t| replace(t, pat, rep)).collect()),
        Node::Pow(a, b) => pow(replace(a, pat, rep), replace(b, pat, rep)),
        Node::Atan2(a, b) => atan2(replace(a, pat, rep), replace(b, pat, rep)),
        Node::Fun(g, a) => fun(*g, replace(a, pat, rep)),
    }
}

/// Symbolic derivative with respect to symbol `v`.
pub fn diff(e: &Expr, v: u32) -> Expr {
    if !contains(e, v) {
        return zero();
    }
    match e.node() {
        Node::Num(_) | Node::Pi => zero(),
        Node::Sym(s) => {
            if *s == v {
                one()
            } else {
                zero()
            }
        }
        Node::Add(ts) => add(ts.iter().map(|t| diff(t, v)).collect()),
        Node::Mul(fs) => {
            let mut terms = Vec::new();
            for i in 0..fs.len() {
                let d = diff(&fs[i], v);
                if d.is_zero() {
                    continue;
                }
                let mut p: Vec<Expr> = fs.clone();
                p[i] = d;
                terms.push(mul(p));
            }
            add(terms)
        }
        Node::Pow(b, ex) => {
            if !contains(ex, v) {
                mul(vec![ex.clone(), pow(b.clone(), add(vec![ex.clone(), int(-1)])), diff(b, v)])
            } else {
                mul(vec![
                    e.clone(),
                    add(vec![
                        mul(vec![diff(ex, v), fun(Fun::Ln, b.clone())]),
                        mul(vec![ex.clone(), diff(b, v), pow(b.clone(), int(-1))]),
                    ]),
                ])
            }
        }
        Node::Fun(f, a) => {
            let da = diff(a, v);
            let a = a.clone();
            let d = match f {
                Fun::Sin => cos(a),
                Fun::Cos => neg(sin(a)),
                Fun::Tan => add(vec![one(), sq(fun(Fun::Tan, a))]),
                Fun::Asin => pow(sub(one(), sq(a)), rat(-1, 2)),
                Fun::Acos => neg(pow(sub(one(), sq(a)), rat(-1, 2))),
                Fun::Atan => pow(add(vec![one(), sq(a)]), int(-1)),
                Fun::Sinh => fun(Fun::Cosh, a),
                Fun::Cosh => fun(Fun::Sinh, a),
                Fun::Tanh => sub(one(), sq(fun(Fun::Tanh, a))),
                Fun::Exp => fun(Fun::Exp, a),
                Fun::Ln => pow(a, int(-1)),
                Fun::Abs => div(a.clone(), fun(Fun::Abs, a)),
            };
            mul(vec![d, da])
        }
        Node::Atan2(y, x) => {
            let num_ = sub(mul(vec![x.clone(), diff(y, v)]), mul(vec![y.clone(), diff(x, v)]));
            div(num_, add(vec![sq(x.clone()), sq(y.clone())]))
        }
    }
}

/// Distributes products over sums and expands integer powers of sums.
pub fn expand(e: &Expr) -> Expr {
    expand_lim(e, 400)
}

fn expand_lim(e: &Expr, lim: usize) -> Expr {
    if past_deadline() {
        return e.clone();
    }
    match e.node() {
        Node::Num(_) | Node::Pi | Node::Sym(_) => e.clone(),
        Node::Add(ts) => add(ts.iter().map(|t| expand_lim(t, lim)).collect()),
        Node::Mul(fs) => {
            let parts: Vec<Expr> = fs.iter().map(|f| expand_lim(f, lim)).collect();
            let mut acc: Vec<Expr> = vec![one()];
            for p in parts {
                let terms: Vec<Expr> = match p.node() {
                    Node::Add(ts) => ts.clone(),
                    _ => vec![p.clone()],
                };
                if acc.len() * terms.len() > lim {
                    return mul(fs.iter().map(|f| expand_lim(f, lim)).collect());
                }
                let mut next = Vec::with_capacity(acc.len() * terms.len());
                for a in &acc {
                    for t in &terms {
                        next.push(mul(vec![a.clone(), t.clone()]));
                    }
                }
                acc = next;
            }
            add(acc)
        }
        Node::Pow(b, ex) => {
            let b2 = expand_lim(b, lim);
            if let Some(k) = ex.as_int() {
                if (2..=8).contains(&k) {
                    if let Node::Add(_) = b2.node() {
                        let mut acc = one();
                        for _ in 0..k {
                            match mul_expand(&acc, &b2, lim) {
                                Some(r) => acc = r,
                                None => return pow(b2, ex.clone()),
                            }
                        }
                        return acc;
                    }
                }
            }
            pow(b2, expand_lim(ex, lim))
        }
        Node::Fun(f, a) => fun(*f, expand_lim(a, lim)),
        Node::Atan2(a, b) => atan2(expand_lim(a, lim), expand_lim(b, lim)),
    }
}

fn terms_of(e: &Expr) -> Vec<Expr> {
    match e.node() {
        Node::Add(ts) => ts.clone(),
        _ => vec![e.clone()],
    }
}

/// Multiplies two already-expanded expressions term by term.
fn mul_expand(a: &Expr, b: &Expr, lim: usize) -> Option<Expr> {
    let (ta, tb) = (terms_of(a), terms_of(b));
    if ta.len() * tb.len() > lim {
        return None;
    }
    let mut out = Vec::with_capacity(ta.len() * tb.len());
    for x in &ta {
        for y in &tb {
            out.push(mul(vec![x.clone(), y.clone()]));
        }
    }
    Some(add(out))
}

fn trig_args(e: &Expr, out: &mut Vec<Expr>) {
    match e.node() {
        Node::Pow(b, ex) => {
            if let Node::Fun(Fun::Sin | Fun::Cos, a) = b.node() {
                if ex.as_int().is_some_and(|k| k >= 2) && !out.contains(a) {
                    out.push(a.clone());
                }
            }
            trig_args(b, out);
            trig_args(ex, out);
        }
        Node::Add(ts) | Node::Mul(ts) => ts.iter().for_each(|t| trig_args(t, out)),
        Node::Fun(_, a) => trig_args(a, out),
        Node::Atan2(a, b) => {
            trig_args(a, out);
            trig_args(b, out);
        }
        _ => {}
    }
}

/// Rewrites even powers of sin (or cos) of `arg` through sin² + cos² = 1.
fn pythagoras(e: &Expr, arg: &Expr, from_sin: bool) -> Expr {
    match e.node() {
        Node::Pow(b, ex) => {
            if let (Node::Fun(f, a), Some(k)) = (b.node(), ex.as_int()) {
                let hit = if from_sin { *f == Fun::Sin } else { *f == Fun::Cos };
                if hit && a == arg && k >= 2 {
                    let other = if from_sin { cos(a.clone()) } else { sin(a.clone()) };
                    let base = sub(one(), sq(other));
                    let rest = pow(b.clone(), int(k % 2));
                    return mul(vec![pow(base, int(k / 2)), rest]);
                }
            }
            pow(pythagoras(b, arg, from_sin), pythagoras(ex, arg, from_sin))
        }
        Node::Add(ts) => add(ts.iter().map(|t| pythagoras(t, arg, from_sin)).collect()),
        Node::Mul(ts) => mul(ts.iter().map(|t| pythagoras(t, arg, from_sin)).collect()),
        Node::Fun(f, a) => fun(*f, pythagoras(a, arg, from_sin)),
        Node::Atan2(a, b) => atan2(pythagoras(a, arg, from_sin), pythagoras(b, arg, from_sin)),
        _ => e.clone(),
    }
}

fn digits(n: i128) -> usize {
    let mut n = n.unsigned_abs();
    let mut d = 1;
    while n >= 10 {
        n /= 10;
        d += 1;
    }
    d
}

/// Visual complexity: node count with long numerals costing more.
pub fn complexity(e: &Expr) -> usize {
    let c = complexity_rec(e);
    charge(c as u64 / 64);
    c
}

fn complexity_rec(e: &Expr) -> usize {
    match e.node() {
        Node::Num(Num::R(r)) => 2 + digits(r.n) + if r.d != 1 { digits(r.d) + 1 } else { 0 },
        Node::Num(Num::F(_)) => 12,
        Node::Pi | Node::Sym(_) => 2,
        Node::Add(v) | Node::Mul(v) => 1 + v.iter().map(complexity_rec).sum::<usize>(),
        Node::Pow(a, b) => 1 + complexity_rec(a) + complexity_rec(b),
        Node::Atan2(a, b) => 3 + complexity_rec(a) + complexity_rec(b),
        Node::Fun(_, a) => 3 + complexity_rec(a),
    }
}

fn smallest(cands: Vec<Expr>) -> Expr {
    cands.into_iter().min_by_key(complexity).unwrap()
}

/// Expanded form with Pythagorean reductions applied where they shrink it.
fn trig_best(e: &Expr) -> Expr {
    let mut best = expand(e);
    if best.size() > 3000 {
        return best;
    }
    let mut args = Vec::new();
    trig_args(&best, &mut args);
    for a in args {
        let s = expand(&pythagoras(&best, &a, true));
        let c = expand(&pythagoras(&best, &a, false));
        best = smallest(vec![best, s, c]);
    }
    best
}

/// A coefficient times 1 (None), cos(arg) or sin(arg) (Some(is_sin, arg)),
/// with the argument in canonical form: no zero argument, leading term
/// positive, multiples of π/2 folded away.
fn fourier_term(coef: Expr, is_sin: bool, arg: Expr) -> Option<(Expr, Option<(bool, Expr)>)> {
    let arg = expand(&arg);
    let lead_neg = match arg.node() {
        Node::Add(ts) => ts[0].is_negative_form(),
        _ => arg.is_negative_form(),
    };
    let (arg, coef) = if lead_neg { (expand(&neg(arg)), if is_sin { neg(coef) } else { coef }) } else { (arg, coef) };
    let f = fun(if is_sin { Fun::Sin } else { Fun::Cos }, arg);
    if f.is_zero() {
        return None;
    }
    let (c, rest) = f.split_coeff();
    let coef = mul(vec![num(c), coef]);
    Some(match rest.node() {
        Node::Fun(Fun::Sin, a) => (coef, Some((true, a.clone()))),
        Node::Fun(Fun::Cos, a) => (coef, Some((false, a.clone()))),
        _ => (mul(vec![coef, rest]), None),
    })
}

/// Fourier form of a polynomial in sines and cosines: products and powers
/// of sin and cos become sums of cos and sin of sums of their arguments
/// (cos a·cos b = cos(a − b)/2 + cos(a + b)/2, …). The form is canonical, so
/// identities such as sin² + cos² = 1 or the angle addition formulas reduce
/// by themselves. None when `e` has no such product, or the form gets too
/// large.
pub fn trig_linearize(e: &Expr) -> Option<Expr> {
    type Key = (u8, Expr);
    let ex = expand(e);
    let mut total: BTreeMap<Key, Vec<Expr>> = BTreeMap::new();
    let mut any = false;
    let key = |t: &Option<(bool, Expr)>| -> Key {
        match t {
            None => (0, one()),
            Some((false, a)) => (1, a.clone()),
            Some((true, a)) => (2, a.clone()),
        }
    };
    for term in terms_of(&ex) {
        let factors: Vec<Expr> = match term.node() {
            Node::Mul(fs) => fs.clone(),
            _ => vec![term.clone()],
        };
        let mut coef: Vec<Expr> = Vec::new();
        let mut cur: BTreeMap<Key, Expr> = BTreeMap::new();
        cur.insert((0, one()), one());
        let mut count = 0;
        for f in &factors {
            let (b, k) = match f.node() {
                Node::Pow(b, x) if x.as_int().is_some_and(|k| (1..=12).contains(&k)) => (b.clone(), x.as_int().unwrap()),
                _ => (f.clone(), 1),
            };
            let Node::Fun(g @ (Fun::Sin | Fun::Cos), arg) = b.node() else {
                coef.push(f.clone());
                continue;
            };
            let b_sin = *g == Fun::Sin;
            for _ in 0..k {
                count += 1;
                let mut next: BTreeMap<Key, Vec<Expr>> = BTreeMap::new();
                let mut put = |t: Option<(Expr, Option<(bool, Expr)>)>| {
                    if let Some((c, t)) = t {
                        next.entry(key(&t)).or_default().push(c);
                    }
                };
                for ((kind, a), c) in &cur {
                    let half = mul(vec![rat(1, 2), c.clone()]);
                    let (sum, dif) = (add(vec![a.clone(), arg.clone()]), sub(a.clone(), arg.clone()));
                    match (*kind, b_sin) {
                        (0, _) => put(fourier_term(c.clone(), b_sin, arg.clone())),
                        // cos a cos b
                        (1, false) => {
                            put(fourier_term(half.clone(), false, dif));
                            put(fourier_term(half, false, sum));
                        }
                        // cos a sin b
                        (1, true) => {
                            put(fourier_term(half.clone(), true, sum));
                            put(fourier_term(neg(half), true, dif));
                        }
                        // sin a cos b
                        (_, false) => {
                            put(fourier_term(half.clone(), true, sum));
                            put(fourier_term(half, true, dif));
                        }
                        // sin a sin b
                        (_, true) => {
                            put(fourier_term(half.clone(), false, dif));
                            put(fourier_term(neg(half), false, sum));
                        }
                    }
                }
                cur = next.into_iter().map(|(k, v)| (k, add(v))).filter(|(_, c)| !c.is_zero()).collect();
                if cur.len() > 400 || past_deadline() {
                    return None;
                }
            }
        }
        // a lone sin or cos is already in Fourier form
        if count >= 2 {
            any = true;
        }
        let c0 = mul(coef);
        for (k, c) in cur {
            total.entry(k).or_default().push(mul(vec![c0.clone(), c]));
        }
        if total.len() > 2000 {
            return None;
        }
    }
    if !any {
        return None;
    }
    let mut out = Vec::new();
    for ((kind, a), cs) in total {
        let c = expand(&add(cs));
        if c.is_zero() {
            continue;
        }
        let c = factor_common(&c).unwrap_or(c);
        out.push(match kind {
            0 => c,
            1 => mul(vec![c, cos(a)]),
            _ => mul(vec![c, sin(a)]),
        });
    }
    Some(add(out))
}

/// Replaces sums (and whole radicands) by their Fourier forms where that
/// makes them smaller, bottom-up.
pub fn fourier_best(e: &Expr) -> Expr {
    if past_deadline() || e.size() > 20000 {
        return e.clone();
    }
    let r = match e.node() {
        Node::Num(_) | Node::Pi | Node::Sym(_) => return e.clone(),
        Node::Add(ts) => add(ts.iter().map(fourier_best).collect()),
        Node::Mul(ts) => mul(ts.iter().map(fourier_best).collect()),
        Node::Pow(b, x) => pow(fourier_best(b), x.clone()),
        Node::Fun(f, a) => fun(*f, fourier_best(a)),
        Node::Atan2(a, b) => atan2(fourier_best(a), fourier_best(b)),
    };
    let whole = match r.node() {
        Node::Add(_) => true,
        Node::Pow(b, x) => x.as_int().is_none() && matches!(b.node(), Node::Add(_) | Node::Mul(_)),
        _ => false,
    };
    if !whole {
        return r;
    }
    let cand = match r.node() {
        Node::Pow(b, x) => trig_linearize(b).map(|l| pow(l, x.clone())),
        _ => trig_linearize(&r),
    };
    match cand {
        Some(c) if complexity(&c) < complexity(&r) => c,
        _ => r,
    }
}

/// Pulls the common numeric content and shared powers out of a sum,
/// e.g. 6a²b + 9ab² → 3ab·(2a + 3b). Returns None when nothing is common.
pub fn factor_common(e: &Expr) -> Option<Expr> {
    let Node::Add(ts) = e.node() else { return None };
    let mut g: i128 = 0;
    let mut l: i128 = 1;
    let mut common: Option<BTreeMap<Expr, Rat>> = None;
    for t in ts {
        let (c, rest) = t.split_coeff();
        match c {
            Num::R(r) => {
                g = gcd(g, r.n);
                let gg = gcd(l, r.d).max(1);
                l = (l / gg).checked_mul(r.d)?;
            }
            Num::F(_) => return None,
        }
        let mut m: BTreeMap<Expr, Rat> = BTreeMap::new();
        let fs: Vec<Expr> = match rest.node() {
            Node::Mul(fs) => fs.clone(),
            _ if rest.is_one() => vec![],
            _ => vec![rest.clone()],
        };
        for f in fs {
            let (b, ex) = base_exp(&f);
            if let Some(k) = ex.as_rat() {
                if k.n > 0 && (k.d == 1 || is_nonneg(&f)) {
                    m.insert(b, k);
                }
            }
        }
        common = Some(match common {
            None => m,
            Some(c) => c
                .into_iter()
                .filter_map(|(b, k)| m.get(&b).map(|k2| (b, if k2.to_f64() < k.to_f64() { *k2 } else { k })))
                .collect(),
        });
    }
    let common = common.unwrap_or_default();
    let content = Rat::new(g.max(1), l)?;
    if content.n == 1 && content.d == 1 && common.is_empty() {
        return None;
    }
    let mut fs = vec![num(Num::R(content))];
    for (b, k) in &common {
        fs.push(pow(b.clone(), num(Num::R(*k))));
    }
    let factor = mul(fs);
    let inv = pow(factor.clone(), int(-1));
    let inner = add(ts.iter().map(|t| mul(vec![t.clone(), inv.clone()])).collect());
    let mut parts = match factor.node() {
        Node::Mul(f) => f.clone(),
        _ => vec![factor],
    };
    parts.push(inner);
    Some(Expr::raw(Node::Mul(parts)))
}

/// Applies `best_form` bottom-up through radicands, function arguments and sums.
pub fn deep_best(e: &Expr) -> Expr {
    if e.size() > 3000 || past_deadline() {
        return e.clone();
    }
    let r = match e.node() {
        Node::Num(_) | Node::Pi | Node::Sym(_) => return e.clone(),
        Node::Add(ts) => add(ts.iter().map(deep_best).collect()),
        Node::Mul(ts) => {
            let kids: Vec<Expr> = ts.iter().map(deep_best).collect();
            let mut best = mul(kids.clone());
            // factoring a radicand can cancel against the rest of the product
            for i in 0..kids.len() {
                if let Node::Pow(b, ex) = kids[i].node() {
                    if ex.as_int().is_none() {
                        for c in [b.clone(), expand(b)] {
                            if past_deadline() {
                                break;
                            }
                            if let Some(f) = factor_common(&c) {
                                let mut k2 = kids.clone();
                                k2[i] = pow(f, ex.clone());
                                let cand = mul(k2);
                                if complexity(&cand) < complexity(&best) {
                                    best = cand;
                                }
                            }
                        }
                    }
                }
            }
            best
        }
        Node::Pow(b, ex) => {
            let b1 = deep_best(b);
            let mut cands = vec![pow(b1.clone(), ex.clone())];
            if ex.as_int().is_none() {
                let mut all: Vec<u32> = sym_set(&b1).into_iter().collect();
                all.sort();
                let (n, d) = poly::split_frac(&b1, &all);
                let n = trig_best(&n);
                let d = trig_best(&d);
                // splitting the root is only valid for a denominator that cannot be negative
                if !d.is_one() && is_nonneg(&d) {
                    cands.push(mul(vec![pow(n.clone(), ex.clone()), pow(d.clone(), neg(ex.clone()))]));
                }
                // (n/d)^x = n^x·d^(-x) needs one side known to be non-negative:
                // with both negative the two roots would not exist. Otherwise the
                // factored quotient stays under one root.
                let split_ok = is_nonneg(&d) || is_nonneg(&n);
                for (nn, dd) in [(factor_common(&n), Some(d.clone())), (factor_common(&n), factor_common(&d))] {
                    if let Some(nn) = nn {
                        let dd = dd.unwrap_or_else(|| d.clone());
                        if split_ok {
                            cands.push(mul(vec![pow(nn, ex.clone()), pow(dd, neg(ex.clone()))]));
                        } else {
                            cands.push(pow(div(nn, dd), ex.clone()));
                        }
                    }
                }
                for c in [b1.clone(), expand(&b1)] {
                    if past_deadline() {
                        break;
                    }
                    if let Some(f) = factor_common(&c) {
                        cands.push(pow(f, ex.clone()));
                    }
                }
                // perfect squares under a square root
                if ex.as_rat().is_some_and(|r| r.d == 2) {
                    if let (Some(qn), Some(qd)) = (poly_sqrt(&n), poly_sqrt(&d)) {
                        let r = div(fun(Fun::Abs, qn), fun(Fun::Abs, qd));
                        let k = ex.as_rat().unwrap().n;
                        cands.push(deep_best_shallow(pow(r, int(k as i64))));
                    }
                }
            }
            smallest(cands)
        }
        Node::Fun(f, a) => fun(*f, deep_best(a)),
        Node::Atan2(a, b) => atan2(deep_best(a), deep_best(b)),
    };
    // past the deadline the parts are already simplified; skip the whole-node pass
    if past_deadline() { r } else { best_form(&r) }
}

/// (A ± A·cos x)^(k/2) as (2A)^(k/2)·|cos(x/2)|^k or (2A)^(k/2)·|sin(x/2)|^k,
/// from 1 + cos x = 2cos²(x/2) and 1 - cos x = 2sin²(x/2).
pub fn half_angle_root(b: &Expr, ex: &Expr) -> Option<Expr> {
    let r = ex.as_rat().filter(|r| r.d == 2)?;
    let b = expand(b);
    let Node::Add(ts) = b.node() else { return None };
    let mut arg: Option<Expr> = None;
    let (mut a, mut c) = (Vec::new(), Vec::new());
    for t in ts {
        let fs = match t.node() {
            Node::Mul(fs) => fs.clone(),
            _ => vec![t.clone()],
        };
        let Some(i) = fs.iter().position(|f| matches!(f.node(), Node::Fun(Fun::Cos, _))) else {
            a.push(t.clone());
            continue;
        };
        let Node::Fun(_, x) = fs[i].node() else { unreachable!() };
        if arg.as_ref().is_some_and(|y| y != x) {
            return None;
        }
        arg = Some(x.clone());
        let mut rest = fs.clone();
        rest.remove(i);
        c.push(mul(rest));
    }
    let x = arg?;
    let (a, c) = (add(a), add(c));
    if a.is_zero() || contains_fun_of(&a, &x) || contains_fun_of(&c, &x) {
        return None;
    }
    let half = mul(vec![rat(1, 2), x]);
    let f = if expand(&add(vec![a.clone(), c.clone()])).is_zero() {
        sin(half)
    } else if expand(&sub(a.clone(), c)).is_zero() {
        cos(half)
    } else {
        return None;
    };
    Some(mul(vec![pow(mul(vec![int(2), a]), ex.clone()), pow(fun(Fun::Abs, f), int(r.n as i64))]))
}

/// Whether e contains cos x or sin x.
fn contains_fun_of(e: &Expr, x: &Expr) -> bool {
    match e.node() {
        Node::Fun(Fun::Sin | Fun::Cos, a) if a == x => true,
        Node::Add(ts) | Node::Mul(ts) => ts.iter().any(|t| contains_fun_of(t, x)),
        Node::Pow(a, b) | Node::Atan2(a, b) => contains_fun_of(a, x) || contains_fun_of(b, x),
        Node::Fun(_, a) => contains_fun_of(a, x),
        _ => false,
    }
}

/// Exact square root of a polynomial (with rational-function coefficients),
/// verified by expansion: returns q with q² = e.
pub fn poly_sqrt(e: &Expr) -> Option<Expr> {
    poly_sqrt_depth(&expand(e), 0)
}

fn poly_sqrt_depth(e: &Expr, depth: u32) -> Option<Expr> {
    if depth > 6 || e.size() > 600 {
        return None;
    }
    if let Some(n) = e.as_num() {
        if n.is_neg() {
            return None;
        }
        let r = pow(e.clone(), rat(1, 2));
        return if r.as_num().is_some() { Some(r) } else { None };
    }
    let mut vs: Vec<u32> = sym_set(e).into_iter().collect();
    vs.sort();
    let mut best: Option<(usize, u32, Vec<Expr>)> = None;
    for v in vs {
        if let Some(cs) = poly::coeffs(e, v) {
            let d = cs.len() - 1;
            if d >= 1 && best.as_ref().is_none_or(|b| d > b.0) {
                best = Some((d, v, cs));
            }
        }
    }
    let (deg, x, cs) = best?;
    if deg % 2 != 0 {
        // a monomial square such as x^2·y^2 is handled through its coefficient
        return None;
    }
    let k = deg / 2;
    let mut q: Vec<Expr> = vec![zero(); k + 1];
    q[k] = match cs[deg].node() {
        Node::Num(_) => poly_sqrt_depth(&cs[deg], depth + 1)?,
        _ => poly_sqrt_depth(&expand(&cs[deg]), depth + 1)?,
    };
    let two_lead = mul(vec![int(2), q[k].clone()]);
    for j in 1..=k {
        let idx = deg - j;
        let mut sum = Vec::new();
        for i in (k - j + 1)..=k {
            let l = idx as i64 - i as i64;
            if l >= (k - j + 1) as i64 && (l as usize) <= k {
                sum.push(mul(vec![q[i].clone(), q[l as usize].clone()]));
            }
        }
        let rest = sub(cs[idx].clone(), add(sum));
        q[k - j] = expand(&div(rest, two_lead.clone()));
    }
    let qe = add(q.iter().enumerate().map(|(i, c)| mul(vec![c.clone(), pow(sym(x), int(i as i64))])).collect());
    let check = expand(&sub(mul(vec![qe.clone(), qe.clone()]), e.clone()));
    let mut all: Vec<u32> = sym_set(&check).into_iter().collect();
    all.sort();
    let (cn, _) = poly::split_frac(&check, &all);
    if check.is_zero() || expand(&cn).is_zero() { Some(qe) } else { None }
}

/// Exact polynomial quotient n / f (None unless the remainder vanishes).
pub fn poly_div(n: &Expr, f: &Expr) -> Option<Expr> {
    if n.size() > 800 || f.size() > 300 || past_deadline() {
        return None;
    }
    let (n, f) = (expand(n), expand(f));
    let mut vs: Vec<u32> = sym_set(&f).into_iter().collect();
    vs.sort();
    let (x, cf) = vs
        .iter()
        .filter_map(|v| poly::coeffs(&f, *v).map(|c| (*v, c)))
        .filter(|(_, c)| c.len() >= 2)
        .max_by_key(|(_, c)| c.len())?;
    let cn = poly::coeffs(&n, x)?;
    let (dn, df) = (cn.len() - 1, cf.len() - 1);
    if dn < df {
        return None;
    }
    let mut r = cn;
    let mut q = vec![zero(); dn - df + 1];
    for k in (0..=dn - df).rev() {
        let c = best_form(&div(r[k + df].clone(), cf[df].clone()));
        for j in 0..=df {
            r[k + j] = expand(&sub(r[k + j].clone(), mul(vec![c.clone(), cf[j].clone()])));
        }
        q[k] = c;
    }
    let zero_rem = r[..df].iter().all(|c| {
        if c.is_zero() {
            return true;
        }
        let mut all: Vec<u32> = sym_set(c).into_iter().collect();
        all.sort();
        expand(&poly::split_frac(c, &all).0).is_zero()
    });
    if !zero_rem {
        return None;
    }
    Some(add(q.into_iter().enumerate().map(|(i, c)| mul(vec![c, pow(sym(x), int(i as i64))])).collect()))
}

/// Cancels polynomial factors shared by numerator and denominator.
fn cancel_fraction(n: &Expr, d: &Expr) -> Option<Expr> {
    let factors = |e: &Expr| -> Vec<Expr> {
        match e.node() {
            Node::Mul(fs) => fs.clone(),
            _ => vec![e.clone()],
        }
    };
    let mut nn = n.clone();
    let mut dd = d.clone();
    let mut changed = false;
    for f in factors(d) {
        let (b, ex) = base_exp(&f);
        if !matches!(b.node(), Node::Add(_)) || !ex.as_int().is_some_and(|k| k >= 1) {
            continue;
        }
        if let Some(qn) = poly_div(&nn, &b) {
            nn = qn;
            dd = div(dd, b);
            changed = true;
        }
    }
    for f in factors(n) {
        let (b, ex) = base_exp(&f);
        if !matches!(b.node(), Node::Add(_)) || !ex.as_int().is_some_and(|k| k >= 1) {
            continue;
        }
        if let Some(qd) = poly_div(&dd, &b) {
            dd = qd;
            nn = div(nn, b);
            changed = true;
        }
    }
    if changed { Some(div(nn, dd)) } else { None }
}

/// Factors each factor of a product (or a single sum) as far as `factor_common` allows.
fn factor_any(e: &Expr) -> Expr {
    let one_factor = |f: &Expr| -> Expr {
        let (b, ex) = base_exp(f);
        if matches!(b.node(), Node::Add(_)) && ex.as_int().is_some() {
            let t = trig_best(&b);
            let fb = factor_common(&t).or_else(|| factor_common(&b)).unwrap_or(b);
            return pow(fb, ex);
        }
        f.clone()
    };
    match e.node() {
        Node::Mul(fs) => mul(fs.iter().map(one_factor).collect()),
        _ => one_factor(e),
    }
}

fn deep_best_shallow(e: Expr) -> Expr {
    best_form(&e)
}

/// Returns the simplest of several equivalent forms: as given, expanded,
/// trigonometrically reduced, or over a common denominator.
pub fn best_form(e: &Expr) -> Expr {
    if e.size() > 4000 || past_deadline() {
        return e.clone();
    }
    let x = trig_best(e);
    if past_deadline() {
        return smallest(vec![e.clone(), x]);
    }
    let y = rationalize_pass(&x);
    let mut all: Vec<u32> = sym_set(e).into_iter().collect();
    all.sort();
    let (n, d) = poly::split_frac(e, &all);
    let mut cands = vec![e.clone(), x.clone(), y, factor_common(&x).unwrap_or_else(|| e.clone())];
    if !d.is_one() && !past_deadline() {
        // factor both sides so shared factors cancel, then also try expanding each side
        let q = div(factor_any(&n), factor_any(&d));
        let (n2, d2) = poly::split_frac(&q, &all);
        let (tn, td) = (trig_best(&n2), trig_best(&d2));
        cands.push(q);
        cands.push(div(tn.clone(), td.clone()));
        cands.push(div(n2.clone(), td.clone()));
        cands.push(div(tn.clone(), d2.clone()));
        let (fnn, fdd) = (factor_common(&tn).unwrap_or(tn.clone()), factor_common(&td).unwrap_or(td.clone()));
        cands.push(div(fnn.clone(), fdd.clone()));
        let cancelled = if past_deadline() { None } else { cancel_fraction(&fnn, &fdd).or_else(|| cancel_fraction(&tn, &td)) };
        if let Some(c) = cancelled {
            let (cn, cd) = poly::split_frac(&c, &all);
            cands.push(c);
            cands.push(div(factor_common(&trig_best(&cn)).unwrap_or(cn), factor_common(&trig_best(&cd)).unwrap_or(cd)));
        }
    }
    smallest(cands)
}

/// Combines terms of a sum over a common denominator when it shrinks the expression.
fn rationalize_pass(e: &Expr) -> Expr {
    match shared_denominator(e) {
        Some((ts, den)) => {
            let inv = pow(den.clone(), int(-1));
            let inner = add(ts.iter().map(|t| expand(&mul(vec![t.clone(), inv.clone()]))).collect());
            mul(vec![inner, den])
        }
        None => e.clone(),
    }
}

/// The terms of a sum and the negative power that most of them share (at least two).
fn shared_denominator(e: &Expr) -> Option<(Vec<Expr>, Expr)> {
    let Node::Add(ts) = e.node() else { return None };
    let mut den_count: BTreeMap<Expr, usize> = BTreeMap::new();
    for t in ts {
        let fs = match t.node() {
            Node::Mul(fs) => fs.clone(),
            _ => vec![t.clone()],
        };
        for f in fs {
            if let Node::Pow(_, ex) = f.node() {
                if ex.as_num().is_some_and(|n| n.is_neg()) {
                    *den_count.entry(f.clone()).or_default() += 1;
                }
            }
        }
    }
    let (den, cnt) = den_count.into_iter().max_by_key(|(_, c)| *c)?;
    if cnt < 2 {
        return None;
    }
    Some((ts.clone(), den))
}

/// In every sum, the terms that share a denominator are combined over it and
/// factored, when that makes the sum smaller, e.g.
/// a/√(1 - c) - a·c/√(1 - c) = a·√(1 - c). The other terms stay as they are.
pub fn combine_shared_denominators(e: &Expr) -> Expr {
    if past_deadline() {
        return e.clone();
    }
    let r = match e.node() {
        Node::Num(_) | Node::Pi | Node::Sym(_) => return e.clone(),
        Node::Add(ts) => add(ts.iter().map(combine_shared_denominators).collect()),
        Node::Mul(ts) => mul(ts.iter().map(combine_shared_denominators).collect()),
        Node::Pow(b, x) => pow(combine_shared_denominators(b), x.clone()),
        Node::Fun(f, a) => fun(*f, combine_shared_denominators(a)),
        Node::Atan2(a, b) => atan2(combine_shared_denominators(a), combine_shared_denominators(b)),
    };
    let Some((ts, den)) = shared_denominator(&r) else { return r };
    let inv = pow(den.clone(), int(-1));
    let has = |t: &Expr| match t.node() {
        Node::Mul(fs) => fs.contains(&den),
        _ => *t == den,
    };
    let (with, mut others): (Vec<Expr>, Vec<Expr>) = ts.into_iter().partition(has);
    let inner = add(with.iter().map(|t| expand(&mul(vec![t.clone(), inv.clone()]))).collect());
    let Some(f) = factor_common(&inner) else { return r };
    others.push(mul(vec![f, den]));
    let c = add(others);
    if complexity(&c) < complexity(&r) { c } else { r }
}

// ---------------------------------------------------------------- evaluation

pub fn eval(e: &Expr, env: &dyn Fn(u32) -> f64) -> f64 {
    charge(e.size() as u64 / 32);
    eval_rec(e, env)
}

fn eval_rec(e: &Expr, env: &dyn Fn(u32) -> f64) -> f64 {
    match e.node() {
        Node::Num(n) => n.to_f64(),
        Node::Pi => std::f64::consts::PI,
        Node::Sym(s) => env(*s),
        Node::Add(ts) => ts.iter().map(|t| eval_rec(t, env)).sum(),
        Node::Mul(ts) => ts.iter().map(|t| eval_rec(t, env)).product(),
        Node::Pow(b, ex) => {
            let x = eval_rec(b, env);
            match ex.as_num() {
                Some(Num::R(r)) if r.d == 1 => x.powi(r.n as i32),
                Some(Num::R(r)) if r.n == 1 && r.d == 2 => x.sqrt(),
                Some(Num::R(r)) if r.n == -1 && r.d == 2 => 1.0 / x.sqrt(),
                Some(Num::R(r)) if r.d % 2 == 1 && x < 0.0 => -((-x).powf(r.to_f64())) * if r.n % 2 == 0 { -1.0 } else { 1.0 },
                _ => x.powf(eval_rec(ex, env)),
            }
        }
        Node::Fun(f, a) => f.apply(eval_rec(a, env)),
        Node::Atan2(y, x) => eval_rec(y, env).atan2(eval_rec(x, env)),
    }
}

#[derive(Clone, Debug)]
pub enum Op {
    C(f64),
    L(u32),
    Add(u32),
    Mul(u32),
    Pow,
    Powi(i32),
    Sqrt,
    Inv,
    F(Fun),
    Atan2,
}

/// Flat stack bytecode for fast repeated evaluation.
#[derive(Clone, Debug, Default)]
pub struct Code {
    pub ops: Vec<Op>,
}

impl Code {
    pub fn new(e: &Expr) -> Code {
        let mut c = Code { ops: Vec::new() };
        c.emit(e);
        c
    }
    fn emit(&mut self, e: &Expr) {
        match e.node() {
            Node::Num(n) => self.ops.push(Op::C(n.to_f64())),
            Node::Pi => self.ops.push(Op::C(std::f64::consts::PI)),
            Node::Sym(s) => self.ops.push(Op::L(*s)),
            Node::Add(ts) => {
                ts.iter().for_each(|t| self.emit(t));
                self.ops.push(Op::Add(ts.len() as u32));
            }
            Node::Mul(ts) => {
                ts.iter().for_each(|t| self.emit(t));
                self.ops.push(Op::Mul(ts.len() as u32));
            }
            Node::Pow(b, ex) => {
                self.emit(b);
                match ex.as_num() {
                    Some(Num::R(r)) if r.d == 1 && r.n == -1 => self.ops.push(Op::Inv),
                    Some(Num::R(r)) if r.d == 1 && r.n.abs() < 1000 => self.ops.push(Op::Powi(r.n as i32)),
                    Some(Num::R(r)) if r.n == 1 && r.d == 2 => self.ops.push(Op::Sqrt),
                    Some(Num::R(r)) if r.n == -1 && r.d == 2 => {
                        self.ops.push(Op::Sqrt);
                        self.ops.push(Op::Inv);
                    }
                    _ => {
                        self.emit(ex);
                        self.ops.push(Op::Pow);
                    }
                }
            }
            Node::Fun(f, a) => {
                self.emit(a);
                self.ops.push(Op::F(*f));
            }
            Node::Atan2(y, x) => {
                self.emit(y);
                self.emit(x);
                self.ops.push(Op::Atan2);
            }
        }
    }
    #[inline]
    pub fn eval(&self, load: &impl Fn(u32) -> f64, st: &mut Vec<f64>) -> f64 {
        st.clear();
        for op in &self.ops {
            match op {
                Op::C(c) => st.push(*c),
                Op::L(s) => st.push(load(*s)),
                Op::Add(n) => {
                    let k = st.len() - *n as usize;
                    let s: f64 = st[k..].iter().sum();
                    st.truncate(k);
                    st.push(s);
                }
                Op::Mul(n) => {
                    let k = st.len() - *n as usize;
                    let s: f64 = st[k..].iter().product();
                    st.truncate(k);
                    st.push(s);
                }
                Op::Pow => {
                    let e = st.pop().unwrap();
                    let b = st.last_mut().unwrap();
                    *b = b.powf(e);
                }
                Op::Powi(k) => {
                    let b = st.last_mut().unwrap();
                    *b = b.powi(*k);
                }
                Op::Sqrt => {
                    let b = st.last_mut().unwrap();
                    *b = b.sqrt();
                }
                Op::Inv => {
                    let b = st.last_mut().unwrap();
                    *b = 1.0 / *b;
                }
                Op::F(f) => {
                    let b = st.last_mut().unwrap();
                    *b = f.apply(*b);
                }
                Op::Atan2 => {
                    let x = st.pop().unwrap();
                    let y = st.last_mut().unwrap();
                    *y = y.atan2(x);
                }
            }
        }
        st.pop().unwrap_or(f64::NAN)
    }
}

// ---------------------------------------------------------------- CSE for display

/// Extracts repeated large subexpressions into named temporaries.
/// Returns the reduced expression and (symbol, definition) pairs in dependency order.
pub fn cse(e: &Expr, first_sym: u32, max_defs: usize) -> (Expr, Vec<(u32, Expr)>) {
    let mut defs: Vec<(u32, Expr)> = Vec::new();
    let mut cur = e.clone();
    let mut next = first_sym;
    for _ in 0..max_defs {
        if cur.size() < 40 && defs.iter().map(|d| d.1.size()).sum::<usize>() < 40 {
            break;
        }
        let mut counts: BTreeMap<Expr, usize> = BTreeMap::new();
        count_subs(&cur, &mut counts);
        for (_, d) in &defs {
            count_subs(d, &mut counts);
        }
        let best = counts
            .into_iter()
            .filter(|(x, c)| *c >= 2 && x.size() >= 6)
            .max_by_key(|(x, c)| (x.size() * (*c - 1), x.size()));
        let Some((pat, _)) = best else { break };
        let s = sym(next);
        cur = replace(&cur, &pat, &s);
        for d in defs.iter_mut() {
            d.1 = replace(&d.1, &pat, &s);
        }
        defs.push((next, pat));
        next += 1;
    }
    // definitions reference only earlier ones after reversing (later extractions are smaller/outer)
    defs.reverse();
    // order definitions so each only refers to previous ones
    let mut ordered: Vec<(u32, Expr)> = Vec::new();
    let mut remaining = defs;
    while !remaining.is_empty() {
        let idx = remaining
            .iter()
            .position(|(_, d)| remaining.iter().all(|(s2, _)| !contains(d, *s2)))
            .unwrap_or(0);
        ordered.push(remaining.remove(idx));
    }
    (cur, ordered)
}

fn count_subs(e: &Expr, counts: &mut BTreeMap<Expr, usize>) {
    match e.node() {
        Node::Num(_) | Node::Pi | Node::Sym(_) => return,
        // name the base of a reciprocal or square rather than the power itself
        Node::Pow(b, ex) if ex.as_int().is_some() => {
            count_subs(b, counts);
            return;
        }
        _ => {}
    }
    *counts.entry(e.clone()).or_default() += 1;
    match e.node() {
        Node::Add(ts) | Node::Mul(ts) => ts.iter().for_each(|t| count_subs(t, counts)),
        Node::Pow(a, b) | Node::Atan2(a, b) => {
            count_subs(a, counts);
            count_subs(b, counts);
        }
        Node::Fun(_, a) => count_subs(a, counts),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_simplify() {
        let x = sym(0);
        let y = sym(1);
        assert_eq!(&x + &x, mul(vec![int(2), x.clone()]));
        assert_eq!(&x - &x, zero());
        assert_eq!(&x * &x, sq(x.clone()));
        assert_eq!(sqrt(int(12)), mul(vec![int(2), sqrt(int(3))]));
        assert_eq!(sqrt(int(2)) * sqrt(int(2)), int(2));
        assert_eq!(sq(sqrt(&x + &y)), &x + &y);
        assert_eq!(&x / &x, one());
        assert_eq!(cos(mul(vec![rat(1, 3), pi()])), rat(1, 2));
        assert_eq!(sin(pi()), zero());
        assert_eq!(sqrt(zero()), zero());
        assert_eq!(pow(zero(), rat(3, 2)), zero());
    }

    #[test]
    fn positive_sqrt() {
        set_assumptions([5u32].into_iter().collect(), HashSet::new());
        let d = sym(5);
        assert_eq!(sqrt(sq(d.clone())), d);
        assert_eq!(sqrt(mul(vec![int(4), sq(d.clone())])), mul(vec![int(2), d.clone()]));
        set_assumptions(HashSet::new(), HashSet::new());
    }

    #[test]
    fn derivative() {
        let x = sym(0);
        let e = sq(x.clone()) * sin(x.clone());
        let d = diff(&e, 0);
        let v = eval(&d, &|_| 1.3);
        let expect = 2.0 * 1.3 * 1.3f64.sin() + 1.3 * 1.3 * 1.3f64.cos();
        assert!((v - expect).abs() < 1e-12);
        let c = Code::new(&d);
        let mut st = Vec::new();
        assert!((c.eval(&|_| 1.3, &mut st) - expect).abs() < 1e-12);
    }
}

#[cfg(test)]
mod tests2 {
    use super::*;
    #[test]
    fn factor_radicand() {
        set_assumptions([1u32].into_iter().collect(), HashSet::new());
        let (a, b) = (sym(0), sym(1));
        let e = div(add(vec![mul(vec![int(6400), pow(b.clone(), int(6))]), mul(vec![int(2304), sq(a.clone()), pow(b.clone(), int(4))]), mul(vec![int(-256), sq(a.clone()), pow(b.clone(), int(6))])]), int(9));
        println!("e = {:?}", e);
        println!("fc = {:?}", factor_common(&e).map(|x| format!("{:?}", x)));
        let r = deep_best(&sqrt(e.clone()));
        println!("r = {}", crate::expr::bx::to_plain(&r, &|s| format!("s{s}")));
        assert!(complexity(&r) < complexity(&sqrt(e)));
    }
}

#[cfg(test)]
mod tests3 {
    use super::*;
    #[test]
    fn nested_fraction() {
        let names = ["a", "b"];
        let r = |n: &str| names.iter().position(|x| *x == n).map(|i| sym(i as u32));
        let e = parse::parse("(9a^2*sqrt(9a^2+25b^2-a^2 b^2) + 25 b^2 sqrt(9a^2+25b^2-a^2 b^2)) / ((9a^2+25b^2)*(a^2*(3sqrt(9a^2+25b^2-a^2 b^2) - 5b^2)/(9a^2+25b^2) + 5))", &r).unwrap();
        let nm = |s: u32| names[s as usize].to_string();
        println!("in : {} c={}", bx::to_plain(&e, &nm), complexity(&e));
        let d = deep_best(&e);
        println!("out: {} c={}", bx::to_plain(&d, &nm), complexity(&d));
    }
}

#[cfg(test)]
mod trig_tests {
    use super::*;

    fn p(s: &str) -> Expr {
        let names = ["a", "t", "u"];
        parse::parse(s, &|n: &str| names.iter().position(|x| *x == n).map(|i| sym(i as u32))).unwrap()
    }
    fn show(e: &Expr) -> String {
        let names = ["a", "t", "u"];
        bx::to_plain(e, &|s| names[s as usize].to_string())
    }

    #[test]
    fn shifts_by_half_pi() {
        assert_eq!(p("sin(pi - t)"), p("sin(t)"));
        assert_eq!(p("cos(pi - t)"), p("-cos(t)"));
        assert_eq!(p("sin(t + pi/2)"), p("cos(t)"));
        assert_eq!(p("cos(t - pi/2)"), p("sin(t)"));
        assert_eq!(p("sin(t + 3pi)"), p("-sin(t)"));
        assert_eq!(p("tan(t - pi)"), p("tan(t)"));
    }

    #[test]
    fn pythagoras_under_root() {
        let r = deep_best(&p("sqrt(100sin(t)^2/cos(t)^2 + 100)"));
        assert_eq!(show(&r), "10/abs(cos(t))");
    }

    #[test]
    fn half_angle_roots() {
        let r = half_angle_root(&p("2 - 2cos(t)"), &rat(1, 2)).unwrap();
        assert_eq!(show(&r), "2abs(sin(t/2))");
        let r = half_angle_root(&p("a + a*cos(t)"), &rat(1, 2)).unwrap();
        assert_eq!(show(&r), "sqrt(2)*sqrt(a)*abs(cos(t/2))");
        assert!(half_angle_root(&p("2 - cos(t)"), &rat(1, 2)).is_none());
    }

    #[test]
    fn combine_shared_denominator() {
        let e = p("a*cos(u) + a/sqrt(1 - cos(t)) - a*cos(t)/sqrt(1 - cos(t))");
        let r = combine_shared_denominators(&e);
        assert_eq!(show(&r), "a*cos(u) + a*sqrt(1 - cos(t))");
    }
}

#[cfg(test)]
mod fourier_tests {
    use super::*;

    fn p(src: &str) -> Expr {
        let names = ["a", "b", "c", "u", "v"];
        parse::parse(src, &|n: &str| names.iter().position(|x| *x == n).map(|i| sym(i as u32))).unwrap()
    }

    fn show(e: &Expr) -> String {
        let names = ["a", "b", "c", "u", "v"];
        bx::to_plain(e, &|s| names[s as usize].to_string())
    }

    #[test]
    fn pythagoras_reduces() {
        assert_eq!(show(&trig_linearize(&p("u*sin(a)^2 + u*cos(a)^2")).unwrap()), "u");
    }

    #[test]
    fn angle_addition_folds() {
        assert_eq!(show(&trig_linearize(&p("cos(a)*cos(b) - sin(a)*sin(b)")).unwrap()), "cos(a + b)");
        assert_eq!(show(&trig_linearize(&p("sin(a)*cos(b) - cos(a)*sin(b)")).unwrap()), "sin(a - b)");
    }

    #[test]
    fn rotated_vector_keeps_its_length() {
        // |R(b)·(u cos a, u sin a)|² = u²
        let e = p("(u*cos(a)*cos(b) - u*sin(a)*sin(b))^2 + (u*sin(a)*cos(b) + u*cos(a)*sin(b))^2");
        assert_eq!(show(&trig_linearize(&e).unwrap()), "u^2");
    }

    #[test]
    fn nothing_to_linearize() {
        assert!(trig_linearize(&p("u*cos(a) + v*sin(b) + 1")).is_none());
    }

    #[test]
    fn walk_distance_radicand_shrinks() {
        let e = p("sqrt((u*cos(a) + v*cos(a)*cos(b) - v*sin(a)*sin(b))^2 + (u*sin(a) + v*sin(a)*cos(b) + v*cos(a)*sin(b))^2)");
        let f = fourier_best(&e);
        assert_eq!(show(&f), "sqrt(2u*v*cos(b) + u^2 + v^2)");
    }
}
