//! Document model: parameters, entities, constraints, dimensions, variables,
//! and the translation of all of them into symbolic equations.

use crate::expr::parse::{parse, parse_equation};
use crate::expr::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// True for `false`: skips serializing a field that is at its default.
fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Param {
    pub v: f64,
    pub locked: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConicKind {
    Ellipse,
    Hyperbola,
    Parabola,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Ent {
    Point { x: usize, y: usize },
    Line { a: usize, b: usize, infinite: bool },
    Circle { c: usize, r: usize },
    Arc { c: usize, a: usize, b: usize },
    Conic { kind: ConicKind, c: usize, a: usize, b: usize, t: usize },
    Curve { src: String },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entity {
    pub kind: Ent,
    pub name: String,
    #[serde(default)]
    pub construction: bool,
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Con {
    Coincident(usize, usize),
    OnCurve(usize, usize),
    Midpoint(usize, usize),
    Horizontal(usize, usize),
    Vertical(usize, usize),
    Parallel(usize, usize),
    Perpendicular(usize, usize),
    Collinear(usize, usize),
    Equal(usize, usize),
    Tangent { a: usize, b: usize, s: f64, aux: Option<(usize, usize)> },
    Concentric(usize, usize),
    Symmetric(usize, usize, usize),
    /// m is the midpoint of points a and b.
    MidpointOf(usize, usize, usize),
    /// q = p + (b − a), rotated by 90° when `rot`.
    Offset { q: usize, p: usize, a: usize, b: usize, rot: bool },
    /// q = x + s1·û1 + s2·û2 for the unit directions of lines l1, l2.
    Bisect { q: usize, x: usize, l1: usize, l2: usize, s1: f64, s2: f64 },
    /// t lies on curve c and the line from p to t touches c at t.
    TangentAt { p: usize, t: usize, c: usize },
    /// Point p is a focus of conic c.
    Focus { p: usize, c: usize },
    /// The line through a and b is the directrix of conic c belonging to its
    /// focus f (the polar line of f): a lies on it and on the focal axis,
    /// and b = a + (f − center) turned by 90°.
    Directrix { a: usize, b: usize, f: usize, c: usize },
    /// Line l is the directrix of conic c belonging to its focus f: both
    /// points of l lie on the polar line of f.
    DirectrixOf { l: usize, f: usize, c: usize },
    /// q = center + a·e₁ + s·b·e₂ in the frame of hyperbola c: the line from
    /// the center through q is an asymptote.
    Asymptote { q: usize, c: usize, s: f64 },
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum DimKind {
    Distance(usize, usize),
    HDist(usize, usize),
    VDist(usize, usize),
    PointLine(usize, usize),
    Length(usize),
    Radius(usize),
    Diameter(usize),
    Angle(usize, usize),
    ConicA(usize),
    ConicB(usize),
    CoordX(usize),
    CoordY(usize),
}

impl DimKind {
    pub fn is_angle(&self) -> bool {
        matches!(self, DimKind::Angle(..))
    }
    /// Signed quantities (coordinates) can be negative.
    pub fn is_signed(&self) -> bool {
        matches!(self, DimKind::CoordX(_) | DimKind::CoordY(_))
    }
    pub fn ents(&self) -> Vec<usize> {
        match *self {
            DimKind::Distance(a, b)
            | DimKind::HDist(a, b)
            | DimKind::VDist(a, b)
            | DimKind::PointLine(a, b)
            | DimKind::Angle(a, b) => vec![a, b],
            DimKind::Length(a)
            | DimKind::Radius(a)
            | DimKind::Diameter(a)
            | DimKind::ConicA(a)
            | DimKind::ConicB(a)
            | DimKind::CoordX(a)
            | DimKind::CoordY(a) => {
                vec![a]
            }
        }
    }
    pub fn prefix(&self) -> &'static str {
        match self {
            DimKind::Angle(..) => "θ",
            DimKind::Radius(_) => "r",
            DimKind::Diameter(_) => "D",
            DimKind::ConicA(_) => "a",
            DimKind::ConicB(_) => "b",
            DimKind::CoordX(_) => "x",
            DimKind::CoordY(_) => "y",
            _ => "d",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Dim {
    pub kind: DimKind,
    pub name: String,
    pub driving: bool,
    /// Value expression; empty when the value is the plain number in `param`.
    pub expr: String,
    pub param: usize,
    /// Orientation sign recorded when the dimension became driving.
    pub s: f64,
    /// Angle dimensions: show the supplementary angle.
    pub supp: bool,
    /// Label offset from the anchor, in world units.
    pub off: [f64; 2],
    /// When true, the dimension's value is a frozen constant: it is
    /// substituted as a plain number in formula derivation, so its name never
    /// appears.
    #[serde(default, skip_serializing_if = "is_false")]
    pub numeric: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Var {
    pub name: String,
    /// Empty for value variables (stored in `param`), otherwise a defining expression.
    pub expr: String,
    pub param: usize,
    /// When true, the variable's value is a frozen constant: it is substituted
    /// as a plain number in formula derivation, so its name never appears.
    #[serde(default, skip_serializing_if = "is_false")]
    pub numeric: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Owner {
    Con(usize),
    Dim(usize),
    Ent(usize),
}

pub type Pt = (Expr, Expr);

pub fn cross(a: &Pt, b: &Pt) -> Expr {
    &a.0 * &b.1 - &a.1 * &b.0
}
pub fn dot(a: &Pt, b: &Pt) -> Expr {
    &a.0 * &b.0 + &a.1 * &b.1
}
pub fn vsub(a: &Pt, b: &Pt) -> Pt {
    (&a.0 - &b.0, &a.1 - &b.1)
}
pub fn norm2(a: &Pt) -> Expr {
    sq(a.0.clone()) + sq(a.1.clone())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Doc {
    pub params: Vec<Param>,
    pub ents: Vec<Option<Entity>>,
    pub cons: Vec<Option<Con>>,
    pub dims: Vec<Option<Dim>>,
    pub vars: Vec<Option<Var>>,
    pub origin: usize,
    pub xaxis: usize,
    pub yaxis: usize,
}

pub struct CurveDef {
    /// F(x, y) = 0 in terms of X_SYM / Y_SYM.
    pub implicit: Expr,
    /// f(x) for explicit curves y = f(x).
    pub explicit: Option<Expr>,
}

impl Default for Doc {
    fn default() -> Self {
        Self::new()
    }
}

impl Doc {
    pub fn new() -> Doc {
        let mut d = Doc {
            params: vec![],
            ents: vec![],
            cons: vec![],
            dims: vec![],
            vars: vec![],
            origin: 0,
            xaxis: 0,
            yaxis: 0,
        };
        let o = d.add_point(0.0, 0.0);
        d.ents[o].as_mut().unwrap().name = "O".into();
        d.lock_point(o, true);
        let hx = d.add_point(1.0, 0.0);
        let hy = d.add_point(0.0, 1.0);
        for h in [hx, hy] {
            d.lock_point(h, true);
            let e = d.ents[h].as_mut().unwrap();
            e.hidden = true;
            e.name = String::new();
        }
        d.xaxis = d.add_ent(Ent::Line { a: o, b: hx, infinite: true });
        d.yaxis = d.add_ent(Ent::Line { a: o, b: hy, infinite: true });
        for a in [d.xaxis, d.yaxis] {
            d.ents[a].as_mut().unwrap().construction = true;
        }
        d.origin = o;
        d
    }

    pub fn is_fixed_ent(&self, e: usize) -> bool {
        e == self.origin || e == self.xaxis || e == self.yaxis || (e < 5 && self.ent(e).is_some_and(|x| x.hidden))
    }

    /// Adds a hidden helper point that construction lines pass through.
    pub fn add_helper(&mut self, x: f64, y: f64) -> usize {
        let px = self.add_param(x, false);
        let py = self.add_param(y, false);
        self.ents.push(Some(Entity { kind: Ent::Point { x: px, y: py }, name: String::new(), construction: false, hidden: true }));
        self.ents.len() - 1
    }

    // ------------------------------------------------------------ creation

    pub fn add_param(&mut self, v: f64, locked: bool) -> usize {
        self.params.push(Param { v, locked });
        self.params.len() - 1
    }

    fn next_point_name(&self) -> String {
        let used: HashSet<&str> = self.ents.iter().flatten().map(|e| e.name.as_str()).collect();
        for round in 0.. {
            for c in 'A'..='Z' {
                if c == 'O' {
                    continue;
                }
                let n = if round == 0 { c.to_string() } else { format!("{c}{round}") };
                if !used.contains(n.as_str()) {
                    return n;
                }
            }
        }
        unreachable!()
    }

    pub fn add_ent(&mut self, kind: Ent) -> usize {
        let name = if matches!(kind, Ent::Point { .. }) { self.next_point_name() } else { String::new() };
        self.ents.push(Some(Entity { kind, name, construction: false, hidden: false }));
        self.ents.len() - 1
    }

    pub fn add_point(&mut self, x: f64, y: f64) -> usize {
        let px = self.add_param(x, false);
        let py = self.add_param(y, false);
        self.add_ent(Ent::Point { x: px, y: py })
    }

    pub fn add_circle(&mut self, c: usize, r: f64) -> usize {
        let pr = self.add_param(r, false);
        self.add_ent(Ent::Circle { c, r: pr })
    }

    pub fn add_conic(&mut self, kind: ConicKind, c: usize, a: f64, b: f64, t: f64) -> usize {
        let pa = self.add_param(a, false);
        let pb = self.add_param(b, kind == ConicKind::Parabola);
        let pt = self.add_param(t, false);
        self.add_ent(Ent::Conic { kind, c, a: pa, b: pb, t: pt })
    }

    pub fn unique_name(&self, prefix: &str) -> String {
        for i in 1.. {
            let n = format!("{prefix}{i}");
            if !self.name_taken(&n) {
                return n;
            }
        }
        unreachable!()
    }

    pub fn name_taken(&self, n: &str) -> bool {
        self.vars.iter().flatten().any(|v| v.name == n)
            || self.dims.iter().flatten().any(|d| d.name == n)
            || matches!(n, "x" | "y" | "e" | "pi" | "π")
            || Fun::from_name(n).is_some()
    }

    pub fn add_var(&mut self, name: String, v: f64) -> usize {
        let p = self.add_param(v, true);
        self.vars.push(Some(Var { name, expr: String::new(), param: p, numeric: false }));
        self.vars.len() - 1
    }

    pub fn add_dim(&mut self, kind: DimKind, off: [f64; 2]) -> usize {
        let name = match kind {
            DimKind::CoordX(p) | DimKind::CoordY(p) => {
                let n = format!("{}_{}", kind.prefix(), self.ent(p).map(|e| e.name.as_str()).unwrap_or(""));
                if self.name_taken(&n) || n.ends_with('_') { self.unique_name(kind.prefix()) } else { n }
            }
            _ => self.unique_name(kind.prefix()),
        };
        let p = self.add_param(0.0, true);
        let mut d = Dim { kind, name, driving: true, expr: String::new(), param: p, s: 1.0, supp: false, off, numeric: false };
        self.dims.push(None);
        let i = self.dims.len() - 1;
        self.dims[i] = Some(d.clone());
        let v = self.measure_value(i);
        d.s = self.dim_sign(&d);
        self.params[p].v = v;
        self.dims[i] = Some(d);
        i
    }

    pub fn lock_point(&mut self, e: usize, on: bool) {
        if let Some(Ent::Point { x, y }) = self.ent(e).map(|e| e.kind.clone()) {
            self.params[x].locked = on;
            self.params[y].locked = on;
        }
    }

    // ------------------------------------------------------------ queries

    pub fn ent(&self, e: usize) -> Option<&Entity> {
        self.ents.get(e).and_then(|x| x.as_ref())
    }
    pub fn kind(&self, e: usize) -> Option<&Ent> {
        self.ent(e).map(|x| &x.kind)
    }
    pub fn is_point(&self, e: usize) -> bool {
        matches!(self.kind(e), Some(Ent::Point { .. }))
    }
    pub fn is_line(&self, e: usize) -> bool {
        matches!(self.kind(e), Some(Ent::Line { .. }))
    }
    pub fn is_circlelike(&self, e: usize) -> bool {
        matches!(self.kind(e), Some(Ent::Circle { .. } | Ent::Arc { .. }))
    }
    pub fn is_conic(&self, e: usize) -> bool {
        matches!(self.kind(e), Some(Ent::Conic { .. }))
    }
    pub fn is_curve_ent(&self, e: usize) -> bool {
        matches!(self.kind(e), Some(Ent::Curve { .. }))
    }

    pub fn pparams(&self, e: usize) -> (usize, usize) {
        match self.kind(e) {
            Some(Ent::Point { x, y }) => (*x, *y),
            _ => panic!("not a point"),
        }
    }
    pub fn pexpr(&self, e: usize) -> Pt {
        let (x, y) = self.pparams(e);
        (sym(x as u32), sym(y as u32))
    }
    pub fn pval(&self, e: usize) -> [f64; 2] {
        let (x, y) = self.pparams(e);
        [self.params[x].v, self.params[y].v]
    }
    pub fn point_locked(&self, e: usize) -> bool {
        let (x, y) = self.pparams(e);
        self.params[x].locked && self.params[y].locked
    }
    pub fn line_pts(&self, e: usize) -> (usize, usize) {
        match self.kind(e) {
            Some(Ent::Line { a, b, .. }) => (*a, *b),
            _ => panic!("not a line"),
        }
    }
    pub fn line_expr(&self, e: usize) -> (Pt, Pt) {
        let (a, b) = self.line_pts(e);
        (self.pexpr(a), self.pexpr(b))
    }
    /// Center, radius and squared radius of a circle or arc.
    pub fn circ_expr(&self, e: usize) -> (Pt, Expr, Expr) {
        match self.kind(e) {
            Some(Ent::Circle { c, r }) => {
                let r = sym(*r as u32);
                (self.pexpr(*c), r.clone(), sq(r))
            }
            Some(Ent::Arc { c, a, .. }) => {
                let cc = self.pexpr(*c);
                let r2 = norm2(&vsub(&self.pexpr(*a), &cc));
                (cc, sqrt(r2.clone()), r2)
            }
            _ => panic!("not circular"),
        }
    }
    pub fn circ_val(&self, e: usize) -> ([f64; 2], f64) {
        match self.kind(e) {
            Some(Ent::Circle { c, r }) => (self.pval(*c), self.params[*r].v),
            Some(Ent::Arc { c, a, .. }) => {
                let cc = self.pval(*c);
                let aa = self.pval(*a);
                (cc, ((aa[0] - cc[0]).powi(2) + (aa[1] - cc[1]).powi(2)).sqrt())
            }
            _ => ([0.0; 2], 0.0),
        }
    }

    /// Points that an entity is built from.
    pub fn ent_points(&self, e: usize) -> Vec<usize> {
        match self.kind(e) {
            Some(Ent::Line { a, b, .. }) => vec![*a, *b],
            Some(Ent::Circle { c, .. }) | Some(Ent::Conic { c, .. }) => vec![*c],
            Some(Ent::Arc { c, a, b }) => vec![*c, *a, *b],
            _ => vec![],
        }
    }

    /// Parameters owned directly by an entity.
    pub fn ent_params(&self, e: usize) -> Vec<usize> {
        match self.kind(e) {
            Some(Ent::Point { x, y }) => vec![*x, *y],
            Some(Ent::Circle { r, .. }) => vec![*r],
            Some(Ent::Conic { a, b, t, kind, .. }) => {
                if *kind == ConicKind::Parabola {
                    vec![*a, *t]
                } else {
                    vec![*a, *b, *t]
                }
            }
            _ => vec![],
        }
    }

    /// All parameters that an entity's shape depends on.
    pub fn ent_all_params(&self, e: usize) -> Vec<usize> {
        let mut v = self.ent_params(e);
        for p in self.ent_points(e) {
            v.extend(self.ent_params(p));
        }
        if let Some(Ent::Curve { src }) = self.kind(e) {
            if let Ok(d) = self.curve_def(src) {
                let s = sym_set(&d.implicit);
                v.extend(s.into_iter().filter(|s| (*s as usize) < self.params.len()).map(|s| s as usize));
            }
        }
        v
    }

    pub fn con_ents(c: &Con) -> Vec<usize> {
        match *c {
            Con::Coincident(a, b)
            | Con::OnCurve(a, b)
            | Con::Midpoint(a, b)
            | Con::Horizontal(a, b)
            | Con::Vertical(a, b)
            | Con::Parallel(a, b)
            | Con::Perpendicular(a, b)
            | Con::Collinear(a, b)
            | Con::Equal(a, b)
            | Con::Concentric(a, b)
            | Con::Tangent { a, b, .. } => vec![a, b],
            Con::Symmetric(a, b, l) | Con::MidpointOf(a, b, l) => vec![a, b, l],
            Con::Offset { q, p, a, b, .. } => vec![q, p, a, b],
            Con::Bisect { q, x, l1, l2, .. } => vec![q, x, l1, l2],
            Con::TangentAt { p, t, c } => vec![p, t, c],
            Con::Focus { p, c } => vec![p, c],
            Con::Directrix { a, b, f, c } => vec![a, b, f, c],
            Con::DirectrixOf { l, f, c } => vec![l, f, c],
            Con::Asymptote { q, c, .. } => vec![q, c],
        }
    }

    /// Points identified with `p` through coincidence constraints.
    fn coincident_class(&self, p: usize) -> HashSet<usize> {
        let mut class: HashSet<usize> = [p].into_iter().collect();
        loop {
            let mut grew = false;
            for c in self.cons.iter().flatten() {
                if let Con::Coincident(a, b) = c {
                    if class.contains(a) && class.insert(*b) || class.contains(b) && class.insert(*a) {
                        grew = true;
                    }
                }
            }
            if !grew {
                return class;
            }
        }
    }

    /// Points that lie on curve `c` by construction or by constraint.
    fn points_on(&self, c: usize) -> HashSet<usize> {
        let mut s: HashSet<usize> = HashSet::new();
        match self.kind(c) {
            Some(Ent::Line { a, b, .. }) => {
                s.insert(*a);
                s.insert(*b);
            }
            Some(Ent::Arc { a, b, .. }) => {
                s.insert(*a);
                s.insert(*b);
            }
            _ => {}
        }
        for k in self.cons.iter().flatten() {
            match k {
                Con::OnCurve(p, cv) if *cv == c => {
                    s.insert(*p);
                }
                Con::TangentAt { t, c: cv, .. } if *cv == c => {
                    s.insert(*t);
                }
                _ => {}
            }
        }
        let mut out = HashSet::new();
        for p in s {
            out.extend(self.coincident_class(p));
        }
        out
    }

    /// A point lying on both curves of a tangency, where the contact can be
    /// stated directly. Stating tangency through such a point avoids the
    /// singular (double-root) system formed by "on both curves" plus a
    /// separate tangency condition.
    pub fn tangent_contact(&self, a: usize, b: usize) -> Option<usize> {
        let on_a = self.points_on(a);
        let on_b = self.points_on(b);
        let mut common: Vec<usize> = on_a.intersection(&on_b).copied().collect();
        common.sort();
        common.into_iter().find(|p| self.is_point(*p))
    }

    /// Parameters that the solver may change.
    pub fn unknowns(&self) -> Vec<usize> {
        let mut s: HashSet<usize> = HashSet::new();
        for (i, e) in self.ents.iter().enumerate() {
            if e.is_some() {
                s.extend(self.ent_params(i));
            }
        }
        for c in self.cons.iter().flatten() {
            if let Con::Tangent { a, b, aux: Some((x, y)), .. } = c {
                if self.tangent_contact(*a, *b).is_none() {
                    s.insert(*x);
                    s.insert(*y);
                }
            }
        }
        for v in self.vars.iter().flatten() {
            if v.expr.is_empty() {
                s.insert(v.param);
            }
        }
        let mut v: Vec<usize> = s.into_iter().filter(|p| !self.params[*p].locked && !self.is_numeric_param(*p)).collect();
        v.sort();
        v
    }

    /// True when parameter `p` is the frozen value of a numeric variable or
    /// dimension.
    pub fn is_numeric_param(&self, p: usize) -> bool {
        self.vars.iter().flatten().any(|v| v.numeric && v.param == p)
            || self.dims.iter().flatten().any(|d| d.numeric && d.param == p)
    }

    // ------------------------------------------------------------ names & parsing

    pub fn param_names(&self) -> HashMap<usize, String> {
        let mut m = HashMap::new();
        for e in self.ents.iter().flatten() {
            match &e.kind {
                Ent::Point { x, y } => {
                    m.insert(*x, format!("x_{}", e.name));
                    m.insert(*y, format!("y_{}", e.name));
                }
                Ent::Circle { c, r } => {
                    m.insert(*r, format!("r_{}", self.ent(*c).map(|e| e.name.as_str()).unwrap_or("")));
                }
                Ent::Conic { c, a, b, t, .. } => {
                    let n = self.ent(*c).map(|e| e.name.clone()).unwrap_or_default();
                    m.insert(*a, format!("a_{n}"));
                    m.insert(*b, format!("b_{n}"));
                    m.insert(*t, format!("φ_{n}"));
                }
                _ => {}
            }
        }
        for c in self.cons.iter().flatten() {
            if let Con::Tangent { aux: Some((x, y)), .. } = c {
                m.insert(*x, "x_T".into());
                m.insert(*y, "y_T".into());
            }
        }
        for v in self.vars.iter().flatten() {
            if !v.numeric {
                m.insert(v.param, v.name.clone());
            }
        }
        for d in self.dims.iter().flatten() {
            if !d.numeric {
                m.insert(d.param, d.name.clone());
            }
        }
        m
    }

    fn resolve_name(&self, name: &str, depth: u32) -> Option<Expr> {
        if depth > 24 {
            return None;
        }
        for v in self.vars.iter().flatten() {
            if v.name == name {
                if v.expr.trim().is_empty() {
                    return Some(sym(v.param as u32));
                }
                return self.parse_expr(&v.expr, depth + 1).ok();
            }
        }
        for (i, d) in self.dims.iter().enumerate() {
            let Some(d) = d else { continue };
            if d.name == name {
                if d.driving {
                    return self.dim_value_expr(i, depth + 1).ok();
                }
                return Some(self.measure(i));
            }
        }
        if let Some((p, c)) = name.split_once('.') {
            for (i, e) in self.ents.iter().enumerate() {
                if let Some(e) = e {
                    if e.name == p && matches!(e.kind, Ent::Point { .. }) {
                        let (x, y) = self.pexpr(i);
                        return match c {
                            "x" => Some(x),
                            "y" => Some(y),
                            _ => None,
                        };
                    }
                }
            }
        }
        None
    }

    pub fn parse_expr(&self, s: &str, depth: u32) -> Result<Expr, String> {
        if depth > 24 {
            return Err("circular definition".into());
        }
        parse(s, &|n| self.resolve_name(n, depth))
    }

    pub fn curve_def(&self, src: &str) -> Result<CurveDef, String> {
        let res = |n: &str| match n {
            "x" => Some(sym(X_SYM)),
            "y" => Some(sym(Y_SYM)),
            _ => self.resolve_name(n, 1),
        };
        let (l, r) = parse_equation(src, &res)?;
        match l {
            Some(l) => {
                if l.as_sym() == Some(Y_SYM) && !contains(&r, Y_SYM) {
                    Ok(CurveDef { implicit: sym(Y_SYM) - &r, explicit: Some(r) })
                } else {
                    Ok(CurveDef { implicit: l - r, explicit: None })
                }
            }
            None => {
                if contains(&r, Y_SYM) {
                    Ok(CurveDef { implicit: r, explicit: None })
                } else {
                    Ok(CurveDef { implicit: sym(Y_SYM) - &r, explicit: Some(r) })
                }
            }
        }
    }

    pub fn dim_value_expr(&self, i: usize, depth: u32) -> Result<Expr, String> {
        let d = self.dims[i].as_ref().ok_or("missing")?;
        if d.expr.trim().is_empty() {
            Ok(sym(d.param as u32))
        } else {
            self.parse_expr(&d.expr, depth)
        }
    }

    // ------------------------------------------------------------ geometry as equations

    /// Frame of conic e: center (vertex for a parabola), unit axis directions
    /// e₁ (along a) and e₂, and the a, b, kind.
    pub fn conic_frame(&self, e: usize) -> Option<(Pt, Pt, Pt, Expr, Expr, ConicKind)> {
        let Some(Ent::Conic { kind, c, a, b, t }) = self.kind(e) else { return None };
        let t = sym(*t as u32);
        let (ct, st) = (cos(t.clone()), sin(t));
        let e1 = (ct.clone(), st.clone());
        let e2 = (neg(st), ct);
        Some((self.pexpr(*c), e1, e2, sym(*a as u32), sym(*b as u32), *kind))
    }

    /// Coordinates of point p in the frame of conic e.
    fn conic_coords(&self, e: usize, p: &Pt) -> Option<(Expr, Expr)> {
        let (c, e1, e2, ..) = self.conic_frame(e)?;
        let d = vsub(p, &c);
        Some((dot(&d, &e1), dot(&d, &e2)))
    }

    /// The polar line of point f with respect to conic c, evaluated at x
    /// (zero when x lies on it). For a focus it is the directrix.
    fn polar_at(&self, c: usize, f: &Pt, x: &Pt) -> Option<Expr> {
        let (uf, vf) = self.conic_coords(c, f)?;
        let (ux, vx) = self.conic_coords(c, x)?;
        let (_, _, _, a, b, kind) = self.conic_frame(c)?;
        Some(match kind {
            ConicKind::Ellipse => sq(b.clone()) * &uf * &ux + sq(a.clone()) * &vf * &vx - sq(a) * sq(b),
            ConicKind::Hyperbola => sq(b.clone()) * &uf * &ux - sq(a.clone()) * &vf * &vx - sq(a) * sq(b),
            ConicKind::Parabola => &vf * &vx - int(2) * a * (&ux + &uf),
        })
    }

    /// Implicit equation F(X, Y) = 0 of a curve entity.
    pub fn implicit(&self, e: usize, x: &Expr, y: &Expr) -> Option<Expr> {
        let p = (x.clone(), y.clone());
        Some(match self.kind(e)? {
            Ent::Line { .. } => {
                let (a, b) = self.line_expr(e);
                cross(&vsub(&b, &a), &vsub(&p, &a))
            }
            Ent::Circle { .. } | Ent::Arc { .. } => {
                let (c, _, r2) = self.circ_expr(e);
                norm2(&vsub(&p, &c)) - r2
            }
            Ent::Conic { kind, c, a, b, t } => {
                let cc = self.pexpr(*c);
                let d = vsub(&p, &cc);
                let t = sym(*t as u32);
                let (ct, st) = (cos(t.clone()), sin(t));
                let u = &d.0 * &ct + &d.1 * &st;
                let v = &d.1 * &ct - &d.0 * &st;
                let a = sym(*a as u32);
                let b = sym(*b as u32);
                match kind {
                    ConicKind::Ellipse => sq(b.clone()) * sq(u) + sq(a.clone()) * sq(v) - sq(a) * sq(b),
                    ConicKind::Hyperbola => sq(b.clone()) * sq(u) - sq(a.clone()) * sq(v) - sq(a) * sq(b),
                    ConicKind::Parabola => sq(v) - int(4) * a * u,
                }
            }
            Ent::Curve { src } => {
                let d = self.curve_def(src).ok()?;
                subst(&d.implicit, &|s| {
                    if s == X_SYM {
                        Some(x.clone())
                    } else if s == Y_SYM {
                        Some(y.clone())
                    } else {
                        None
                    }
                })
            }
            Ent::Point { .. } => return None,
        })
    }

    pub fn line_len(&self, l: usize) -> Expr {
        sqrt(norm2(&self.line_dir(l)))
    }

    pub fn line_dir(&self, l: usize) -> Pt {
        let (a, b) = self.line_expr(l);
        vsub(&b, &a)
    }

    /// Equations of one constraint.
    pub fn con_eqs(&self, c: &Con) -> Vec<Expr> {
        match c {
            Con::Coincident(a, b) => {
                let (pa, pb) = (self.pexpr(*a), self.pexpr(*b));
                vec![&pa.0 - &pb.0, &pa.1 - &pb.1]
            }
            Con::OnCurve(p, cv) => {
                let pp = self.pexpr(*p);
                let f = self.implicit(*cv, &pp.0, &pp.1);
                // signed distance to a line: unnormalized, least squares could
                // satisfy it by shrinking the line to a point
                if self.is_line(*cv) {
                    return f.map(|f| f / self.line_len(*cv)).into_iter().collect();
                }
                f.into_iter().collect()
            }
            Con::Midpoint(p, l) => {
                let pp = self.pexpr(*p);
                let (a, b) = self.line_expr(*l);
                vec![int(2) * &pp.0 - &a.0 - &b.0, int(2) * &pp.1 - &a.1 - &b.1]
            }
            Con::Horizontal(a, b) => vec![&self.pexpr(*a).1 - &self.pexpr(*b).1],
            Con::Vertical(a, b) => vec![&self.pexpr(*a).0 - &self.pexpr(*b).0],
            Con::Parallel(l1, l2) => vec![cross(&self.line_dir(*l1), &self.line_dir(*l2)) / (self.line_len(*l1) * self.line_len(*l2))],
            Con::Perpendicular(l1, l2) => vec![dot(&self.line_dir(*l1), &self.line_dir(*l2)) / (self.line_len(*l1) * self.line_len(*l2))],
            Con::Collinear(l1, l2) => {
                let (a, b) = self.line_expr(*l1);
                let (c, d) = self.line_expr(*l2);
                let dir = vsub(&b, &a);
                let n = self.line_len(*l1);
                vec![cross(&dir, &vsub(&c, &a)) / &n, cross(&dir, &vsub(&d, &a)) / n]
            }
            Con::Equal(a, b) => {
                let la = self.is_line(*a);
                let lb = self.is_line(*b);
                let len2 = |e: usize| -> Expr {
                    if self.is_line(e) { norm2(&self.line_dir(e)) } else { self.circ_expr(e).2 }
                };
                if !la && !lb {
                    if let (Some(Ent::Circle { r: r1, .. }), Some(Ent::Circle { r: r2, .. })) = (self.kind(*a), self.kind(*b)) {
                        return vec![sym(*r1 as u32) - sym(*r2 as u32)];
                    }
                }
                vec![len2(*a) - len2(*b)]
            }
            Con::Concentric(a, b) => {
                let ca = self.center_of(*a);
                let cb = self.center_of(*b);
                vec![&ca.0 - &cb.0, &ca.1 - &cb.1]
            }
            Con::Symmetric(p, q, l) => {
                let (pp, qq) = (self.pexpr(*p), self.pexpr(*q));
                let (a, b) = self.line_expr(*l);
                let dir = vsub(&b, &a);
                let mid2 = (&pp.0 + &qq.0, &pp.1 + &qq.1);
                let a2 = (int(2) * &a.0, int(2) * &a.1);
                let n = self.line_len(*l);
                vec![cross(&dir, &vsub(&mid2, &a2)) / &n, dot(&dir, &vsub(&qq, &pp)) / n]
            }
            Con::MidpointOf(m, a, b) => {
                let (pm, pa, pb) = (self.pexpr(*m), self.pexpr(*a), self.pexpr(*b));
                vec![int(2) * &pm.0 - &pa.0 - &pb.0, int(2) * &pm.1 - &pa.1 - &pb.1]
            }
            Con::Offset { q, p, a, b, rot } => {
                let (pq, pp) = (self.pexpr(*q), self.pexpr(*p));
                let d = vsub(&self.pexpr(*b), &self.pexpr(*a));
                let d = if *rot { (neg(d.1), d.0) } else { d };
                vec![&pq.0 - &pp.0 - &d.0, &pq.1 - &pp.1 - &d.1]
            }
            Con::Bisect { q, x, l1, l2, s1, s2 } => {
                let (pq, px) = (self.pexpr(*q), self.pexpr(*x));
                let (u, w) = (self.line_dir(*l1), self.line_dir(*l2));
                let (nu, nw) = (sqrt(norm2(&u)), sqrt(norm2(&w)));
                let (s1, s2) = (float(*s1), float(*s2));
                vec![
                    &pq.0 - &px.0 - &s1 * &u.0 / &nu - &s2 * &w.0 / &nw,
                    &pq.1 - &px.1 - &s1 * &u.1 / &nu - &s2 * &w.1 / &nw,
                ]
            }
            Con::Focus { p, c } => {
                let Some((u, v)) = self.conic_coords(*c, &self.pexpr(*p)) else { return vec![] };
                let Some((_, _, _, a, b, kind)) = self.conic_frame(*c) else { return vec![] };
                match kind {
                    // on either axis: the major one carries the foci
                    ConicKind::Ellipse => vec![&u * &v, sq(u) - sq(v) - sq(a) + sq(b)],
                    ConicKind::Hyperbola => vec![v, sq(u) - sq(a) - sq(b)],
                    ConicKind::Parabola => vec![v, u - a],
                }
            }
            Con::Directrix { a: pa, b: pb, f, c } => {
                let (fa, ffa, fb) = (self.pexpr(*pa), self.pexpr(*f), self.pexpr(*pb));
                let (Some((cc, ..)), Some(polar)) = (self.conic_frame(*c), self.polar_at(*c, &ffa, &fa)) else { return vec![] };
                let axis = vsub(&ffa, &cc);
                vec![polar, cross(&axis, &vsub(&fa, &cc)), &fb.0 - &fa.0 + &axis.1, &fb.1 - &fa.1 - &axis.0]
            }
            Con::DirectrixOf { l, f, c } => {
                let ff = self.pexpr(*f);
                let (la, lb) = self.line_expr(*l);
                [la, lb].iter().filter_map(|x| self.polar_at(*c, &ff, x)).collect()
            }
            Con::Asymptote { q, c, s } => {
                let Some((cc, e1, e2, a, b, _)) = self.conic_frame(*c) else { return vec![] };
                let pq = self.pexpr(*q);
                let sb = float(*s) * b;
                vec![&pq.0 - &cc.0 - &a * &e1.0 - &sb * &e2.0, &pq.1 - &cc.1 - &a * &e1.1 - &sb * &e2.1]
            }
            Con::TangentAt { p, t, c } => {
                let (pp, pt) = (self.pexpr(*p), self.pexpr(*t));
                let (xs, ys) = (sym(X_SYM), sym(Y_SYM));
                let Some(f) = self.implicit(*c, &xs, &ys) else { return vec![] };
                let at = |e: &Expr| subst(e, &|v| if v == X_SYM { Some(pt.0.clone()) } else if v == Y_SYM { Some(pt.1.clone()) } else { None });
                let g = (at(&diff(&f, X_SYM)), at(&diff(&f, Y_SYM)));
                let d = vsub(&pt, &pp);
                let n = sqrt(norm2(&g) * norm2(&d));
                vec![at(&f), dot(&g, &d) / n]
            }
            Con::Tangent { a, b, s, aux } => {
                let s = float(*s);
                let (a, b) = (*a, *b);
                if let Some(p) = self.tangent_contact(a, b) {
                    // contact point known: the curve normals there must be parallel
                    let pp = self.pexpr(p);
                    let (xs, ys) = (sym(X_SYM), sym(Y_SYM));
                    let (Some(f1), Some(f2)) = (self.implicit(a, &xs, &ys), self.implicit(b, &xs, &ys)) else {
                        return vec![];
                    };
                    let at = |e: &Expr| subst(e, &|v| if v == X_SYM { Some(pp.0.clone()) } else if v == Y_SYM { Some(pp.1.clone()) } else { None });
                    let g1 = (at(&diff(&f1, X_SYM)), at(&diff(&f1, Y_SYM)));
                    let g2 = (at(&diff(&f2, X_SYM)), at(&diff(&f2, Y_SYM)));
                    // sine of the angle between the normals: unnormalized, a
                    // collapsing line would satisfy it trivially
                    let n = sqrt(norm2(&g1) * norm2(&g2) + float(1e-300));
                    return vec![cross(&g1, &g2) / n];
                }
                if self.is_line(a) && self.is_circlelike(b) || self.is_line(b) && self.is_circlelike(a) {
                    let (l, c) = if self.is_line(a) { (a, b) } else { (b, a) };
                    let (p, q) = self.line_expr(l);
                    let (cc, r, _) = self.circ_expr(c);
                    let dir = vsub(&q, &p);
                    return vec![cross(&dir, &vsub(&cc, &p)) - s * r * sqrt(norm2(&dir))];
                }
                if self.is_circlelike(a) && self.is_circlelike(b) {
                    let (c1, r1, _) = self.circ_expr(a);
                    let (c2, r2, _) = self.circ_expr(b);
                    return vec![norm2(&vsub(&c1, &c2)) - sq(r1 + s * r2)];
                }
                let Some((tx, ty)) = aux else { return vec![] };
                let (tx, ty) = (sym(*tx as u32), sym(*ty as u32));
                let (xs, ys) = (sym(X_SYM), sym(Y_SYM));
                let (Some(f1), Some(f2)) = (self.implicit(a, &xs, &ys), self.implicit(b, &xs, &ys)) else {
                    return vec![];
                };
                let (ga, gb) = ((diff(&f1, X_SYM), diff(&f1, Y_SYM)), (diff(&f2, X_SYM), diff(&f2, Y_SYM)));
                let g = cross(&ga, &gb) / sqrt(norm2(&ga) * norm2(&gb) + float(1e-300));
                let at = |e: &Expr| {
                    subst(e, &|v| {
                        if v == X_SYM {
                            Some(tx.clone())
                        } else if v == Y_SYM {
                            Some(ty.clone())
                        } else {
                            None
                        }
                    })
                };
                vec![at(&f1), at(&f2), at(&g)]
            }
        }
    }

    pub fn center_of(&self, e: usize) -> Pt {
        match self.kind(e) {
            Some(Ent::Point { .. }) => self.pexpr(e),
            Some(Ent::Circle { c, .. }) | Some(Ent::Arc { c, .. }) | Some(Ent::Conic { c, .. }) => self.pexpr(*c),
            _ => (zero(), zero()),
        }
    }

    fn dim_points(&self, k: &DimKind) -> Option<(Pt, Pt)> {
        match *k {
            DimKind::Distance(a, b) | DimKind::HDist(a, b) | DimKind::VDist(a, b) => Some((self.pexpr(a), self.pexpr(b))),
            DimKind::Length(l) => Some(self.line_expr(l)),
            _ => None,
        }
    }

    /// Signed raw quantity of a dimension, used for orientation signs.
    fn dim_signed(&self, d: &Dim) -> Option<Expr> {
        Some(match d.kind {
            DimKind::HDist(a, b) => &self.pexpr(b).0 - &self.pexpr(a).0,
            DimKind::VDist(a, b) => &self.pexpr(b).1 - &self.pexpr(a).1,
            DimKind::PointLine(p, l) => {
                let (a, b) = self.line_expr(l);
                cross(&vsub(&b, &a), &vsub(&self.pexpr(p), &a))
            }
            DimKind::Angle(l1, l2) => cross(&self.line_dir(l1), &self.line_dir(l2)),
            _ => return None,
        })
    }

    pub fn eval_now(&self, e: &Expr) -> f64 {
        eval(e, &|s| self.params.get(s as usize).map(|p| p.v).unwrap_or(f64::NAN))
    }

    pub fn dim_sign(&self, d: &Dim) -> f64 {
        match self.dim_signed(d) {
            Some(e) => {
                if self.eval_now(&e) < 0.0 {
                    -1.0
                } else {
                    1.0
                }
            }
            None => 1.0,
        }
    }

    /// Expression of the quantity a dimension measures, oriented so that it is
    /// non-negative in the current configuration.
    pub fn measure(&self, i: usize) -> Expr {
        let Some(d) = self.dims[i].as_ref() else { return zero() };
        let s = float(self.dim_sign(d));
        match d.kind {
            DimKind::Distance(..) | DimKind::Length(_) => {
                let (a, b) = self.dim_points(&d.kind).unwrap();
                sqrt(norm2(&vsub(&b, &a)))
            }
            DimKind::HDist(..) | DimKind::VDist(..) => s * self.dim_signed(d).unwrap(),
            DimKind::PointLine(_, l) => s * self.dim_signed(d).unwrap() / sqrt(norm2(&self.line_dir(l))),
            DimKind::Radius(c) => self.circ_expr(c).1,
            DimKind::Diameter(c) => int(2) * self.circ_expr(c).1,
            DimKind::Angle(l1, l2) => {
                let (u, v) = (self.line_dir(l1), self.line_dir(l2));
                let th = fun(Fun::Acos, dot(&u, &v) / sqrt(norm2(&u) * norm2(&v)));
                if d.supp { pi() - th } else { th }
            }
            DimKind::ConicA(c) => match self.kind(c) {
                Some(Ent::Conic { a, .. }) => sym(*a as u32),
                _ => zero(),
            },
            DimKind::ConicB(c) => match self.kind(c) {
                Some(Ent::Conic { b, .. }) => sym(*b as u32),
                _ => zero(),
            },
            DimKind::CoordX(p) => &self.pexpr(p).0 - &self.pexpr(self.origin).0,
            DimKind::CoordY(p) => &self.pexpr(p).1 - &self.pexpr(self.origin).1,
        }
    }

    pub fn measure_value(&self, i: usize) -> f64 {
        self.eval_now(&self.measure(i))
    }

    /// Equation forcing a driving dimension to its value.
    pub fn dim_eq(&self, i: usize) -> Result<Option<Expr>, String> {
        let d = self.dims[i].as_ref().ok_or("missing")?;
        if !d.driving {
            return Ok(None);
        }
        let v = self.dim_value_expr(i, 0)?;
        let s = float(d.s);
        Ok(Some(match d.kind {
            DimKind::Distance(..) | DimKind::Length(_) => {
                let (a, b) = self.dim_points(&d.kind).unwrap();
                norm2(&vsub(&b, &a)) - sq(v)
            }
            DimKind::HDist(..) | DimKind::VDist(..) => self.dim_signed(d).unwrap() - s * v,
            DimKind::PointLine(_, l) => self.dim_signed(d).unwrap() - s * v * sqrt(norm2(&self.line_dir(l))),
            DimKind::Radius(c) => match self.kind(c) {
                Some(Ent::Circle { r, .. }) => sym(*r as u32) - v,
                _ => self.circ_expr(c).2 - sq(v),
            },
            DimKind::Diameter(c) => match self.kind(c) {
                Some(Ent::Circle { r, .. }) => int(2) * sym(*r as u32) - v,
                _ => int(4) * self.circ_expr(c).2 - sq(v),
            },
            DimKind::Angle(l1, l2) => {
                let (u, w) = (self.line_dir(l1), self.line_dir(l2));
                let th = if d.supp { pi() - v } else { v };
                // atan2(sin(φ − sθ), cos(φ − sθ)) = 0 has the single root φ = sθ,
                // unlike sin(φ − sθ) = 0, which also admits φ = sθ + π
                let (cr, dt) = (cross(&u, &w), dot(&u, &w));
                let y = &cr * cos(th.clone()) - &s * &dt * sin(th.clone());
                let x = dt * cos(th.clone()) + s * cr * sin(th);
                atan2(y, x)
            }
            DimKind::ConicA(c) => match self.kind(c) {
                Some(Ent::Conic { a, .. }) => sym(*a as u32) - v,
                _ => return Ok(None),
            },
            DimKind::ConicB(c) => match self.kind(c) {
                Some(Ent::Conic { b, .. }) => sym(*b as u32) - v,
                _ => return Ok(None),
            },
            DimKind::CoordX(p) => &self.pexpr(p).0 - &self.pexpr(self.origin).0 - v,
            DimKind::CoordY(p) => &self.pexpr(p).1 - &self.pexpr(self.origin).1 - v,
        }))
    }

    /// Every equation of the document with its owner, plus errors per owner.
    pub fn equations(&self) -> (Vec<(Expr, Owner)>, Vec<(Owner, String)>) {
        let mut out = Vec::new();
        let mut errs = Vec::new();
        for (i, e) in self.ents.iter().enumerate() {
            if let Some(Entity { kind: Ent::Arc { c, a, b }, .. }) = e {
                let cc = self.pexpr(*c);
                out.push((
                    norm2(&vsub(&self.pexpr(*a), &cc)) - norm2(&vsub(&self.pexpr(*b), &cc)),
                    Owner::Ent(i),
                ));
            }
        }
        for (i, c) in self.cons.iter().enumerate() {
            if let Some(c) = c {
                for e in self.con_eqs(c) {
                    out.push((e, Owner::Con(i)));
                }
            }
        }
        for i in 0..self.dims.len() {
            if self.dims[i].is_none() {
                continue;
            }
            match self.dim_eq(i) {
                Ok(Some(e)) => out.push((e, Owner::Dim(i))),
                Ok(None) => {}
                Err(m) => errs.push((Owner::Dim(i), m)),
            }
        }
        (out, errs)
    }

    /// Symbols known to be non-negative and symbols known to be angles in [0, π].
    pub fn assumptions(&self) -> (HashSet<u32>, HashSet<u32>) {
        let mut pos = HashSet::new();
        let mut ang = HashSet::new();
        for d in self.dims.iter().flatten() {
            if d.kind.is_signed() {
                continue;
            }
            if d.kind.is_angle() {
                ang.insert(d.param as u32);
                pos.insert(d.param as u32);
            } else {
                pos.insert(d.param as u32);
            }
        }
        for e in self.ents.iter().flatten() {
            match &e.kind {
                Ent::Circle { r, .. } => {
                    pos.insert(*r as u32);
                }
                Ent::Conic { a, b, .. } => {
                    pos.insert(*a as u32);
                    pos.insert(*b as u32);
                }
                _ => {}
            }
        }
        (pos, ang)
    }

    // ------------------------------------------------------------ deletion

    pub fn delete(&mut self, ents: &[usize], cons: &[usize], dims: &[usize]) {
        let mut dead: HashSet<usize> = ents.iter().copied().filter(|e| !self.is_fixed_ent(*e)).collect();
        // features constructed from a conic (foci, directrices, asymptotes:
        // created after it) go with it, and a directrix goes with its focus;
        // points the conic was built from (given foci) stay
        loop {
            let before = dead.len();
            let conics: Vec<usize> = dead.iter().copied().filter(|e| self.is_conic(*e)).collect();
            for c in conics {
                dead.extend(self.foci_of(c).into_iter().filter(|f| *f > c));
            }
            for k in self.cons.iter().flatten() {
                match *k {
                    Con::Focus { p, c } if dead.contains(&c) && p > c => {
                        dead.insert(p);
                    }
                    Con::Directrix { a, b, f, c } if dead.contains(&c) || dead.contains(&f) => {
                        dead.insert(a);
                        dead.insert(b);
                    }
                    Con::Asymptote { q, c, .. } if dead.contains(&c) => {
                        dead.insert(q);
                    }
                    _ => {}
                }
            }
            if dead.len() == before {
                break;
            }
        }
        // owned points of deleted non-point entities, when not shared
        let mut owned: HashSet<usize> = HashSet::new();
        for &e in &dead {
            if !self.is_point(e) {
                owned.extend(self.ent_points(e));
            }
        }
        loop {
            let mut changed = false;
            for i in 0..self.ents.len() {
                if self.ents[i].is_none() || dead.contains(&i) {
                    continue;
                }
                if self.ent_points(i).iter().any(|p| dead.contains(p)) {
                    dead.insert(i);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        for p in owned {
            if self.is_fixed_ent(p) || dead.contains(&p) {
                continue;
            }
            let used = (0..self.ents.len()).any(|i| self.ents[i].is_some() && !dead.contains(&i) && self.ent_points(i).contains(&p));
            if !used {
                dead.insert(p);
            }
        }
        for &e in &dead {
            self.ents[e] = None;
        }
        for (i, c) in self.cons.iter_mut().enumerate() {
            if let Some(cc) = c {
                if cons.contains(&i) || Doc::con_ents(cc).iter().any(|e| dead.contains(e)) {
                    *c = None;
                }
            }
        }
        for (i, d) in self.dims.iter_mut().enumerate() {
            if let Some(dd) = d {
                if dims.contains(&i) || dd.kind.ents().iter().any(|e| dead.contains(e)) {
                    *d = None;
                }
            }
        }
    }
}
