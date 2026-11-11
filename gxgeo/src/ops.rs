//! Checked editing operations on a [`Sketch`] for the C and JS interfaces:
//! arguments are validated before the document changes, so a bad index
//! gives an error instead of a panic.

use crate::build::Pt;
use crate::draw::{Window, draw_polys};
use crate::model::*;
use crate::expr::parse::plain_number;
use crate::sketch::{crossing_near, tangent_contact_near, valid_ident, Sketch};

pub use crate::build::ConicFeature;

/// Constraint kinds available through the interfaces.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConKind {
    /// Points a and b coincide.
    Coincident,
    /// Point a lies on curve b.
    OnCurve,
    /// Point a is the midpoint of line b.
    Midpoint,
    /// Line a (or points a, b) horizontal.
    Horizontal,
    /// Line a (or points a, b) vertical.
    Vertical,
    /// Lines a and b parallel.
    Parallel,
    /// Lines a and b perpendicular.
    Perpendicular,
    /// Lines a and b on one line.
    Collinear,
    /// Lines a and b of equal length, or circles a and b of equal radius.
    Equal,
    /// Circles, arcs or conics a and b share a center.
    Concentric,
    /// Curves a and b tangent, on the side they are on now.
    Tangent,
    /// Points a and b are symmetric about line c.
    Symmetric,
    /// Point a is the midpoint of points b and c.
    MidpointOf,
    /// Point a is point b offset by the direction of points c to d.
    Offset,
    /// Point a lies on the bisector of lines b and c through point d.
    Bisector,
    /// The line through a and b touches curve c at b.
    TangentAt,
    /// Point a is a focus of conic b.
    Focus,
    /// Line a (through point b) is the directrix of conic c.
    Directrix,
    /// Line a is an asymptote of conic b.
    Asymptote,
}

impl ConKind {
    pub const ALL: [ConKind; 19] = [
        ConKind::Coincident,
        ConKind::OnCurve,
        ConKind::Midpoint,
        ConKind::Horizontal,
        ConKind::Vertical,
        ConKind::Parallel,
        ConKind::Perpendicular,
        ConKind::Collinear,
        ConKind::Equal,
        ConKind::Concentric,
        ConKind::Tangent,
        ConKind::Symmetric,
        ConKind::MidpointOf,
        ConKind::Offset,
        ConKind::Bisector,
        ConKind::TangentAt,
        ConKind::Focus,
        ConKind::Directrix,
        ConKind::Asymptote,
    ];

    /// The kinds a client can add with [`constrain`]; the rest are bindings a
    /// construction tool creates and that a client can only read back.
    pub const ADDABLE: [ConKind; 11] = [
        ConKind::Coincident,
        ConKind::OnCurve,
        ConKind::Midpoint,
        ConKind::Horizontal,
        ConKind::Vertical,
        ConKind::Parallel,
        ConKind::Perpendicular,
        ConKind::Collinear,
        ConKind::Equal,
        ConKind::Concentric,
        ConKind::Tangent,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ConKind::Coincident => "coincident",
            ConKind::OnCurve => "on_curve",
            ConKind::Midpoint => "midpoint",
            ConKind::Horizontal => "horizontal",
            ConKind::Vertical => "vertical",
            ConKind::Parallel => "parallel",
            ConKind::Perpendicular => "perpendicular",
            ConKind::Collinear => "collinear",
            ConKind::Equal => "equal",
            ConKind::Concentric => "concentric",
            ConKind::Tangent => "tangent",
            ConKind::Symmetric => "symmetric",
            ConKind::MidpointOf => "midpoint_of",
            ConKind::Offset => "offset",
            ConKind::Bisector => "bisector",
            ConKind::TangentAt => "tangent_at",
            ConKind::Focus => "focus",
            ConKind::Directrix => "directrix",
            ConKind::Asymptote => "asymptote",
        }
    }

    pub fn from_name(s: &str) -> Result<ConKind, String> {
        ConKind::ALL.into_iter().find(|k| k.name() == s).ok_or_else(|| format!("unknown constraint kind {s:?}"))
    }

    pub fn from_index(i: u32) -> Result<ConKind, String> {
        ConKind::ALL.get(i as usize).copied().ok_or_else(|| format!("unknown constraint kind {i}"))
    }
}

/// Dimension kinds available through the interfaces.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DimType {
    /// Distance between points a and b.
    Distance,
    /// Horizontal distance between points a and b.
    HDist,
    /// Vertical distance between points a and b.
    VDist,
    /// Distance from point a to line b.
    PointLine,
    /// Length of line a.
    Length,
    /// Radius of circle or arc a.
    Radius,
    /// Diameter of circle or arc a.
    Diameter,
    /// Angle between lines a and b.
    Angle,
    /// First half-axis of conic a.
    ConicA,
    /// Second half-axis of conic a.
    ConicB,
    /// x coordinate of point a.
    CoordX,
    /// y coordinate of point a.
    CoordY,
}

impl DimType {
    pub const ALL: [DimType; 12] = [
        DimType::Distance,
        DimType::HDist,
        DimType::VDist,
        DimType::PointLine,
        DimType::Length,
        DimType::Radius,
        DimType::Diameter,
        DimType::Angle,
        DimType::ConicA,
        DimType::ConicB,
        DimType::CoordX,
        DimType::CoordY,
    ];

    pub fn name(self) -> &'static str {
        match self {
            DimType::Distance => "distance",
            DimType::HDist => "hdist",
            DimType::VDist => "vdist",
            DimType::PointLine => "point_line",
            DimType::Length => "length",
            DimType::Radius => "radius",
            DimType::Diameter => "diameter",
            DimType::Angle => "angle",
            DimType::ConicA => "conic_a",
            DimType::ConicB => "conic_b",
            DimType::CoordX => "x",
            DimType::CoordY => "y",
        }
    }

    pub fn from_name(s: &str) -> Result<DimType, String> {
        DimType::ALL.into_iter().find(|k| k.name() == s).ok_or_else(|| format!("unknown dimension kind {s:?}"))
    }

    pub fn from_index(i: u32) -> Result<DimType, String> {
        DimType::ALL.get(i as usize).copied().ok_or_else(|| format!("unknown dimension kind {i}"))
    }
}

#[derive(Clone, Copy)]
enum Need {
    Point,
    Line,
    /// Circle or arc.
    Round,
    /// Circle, arc or conic.
    Centered,
    Conic,
    /// Any entity other than a point.
    Curve,
    /// Line, circle or arc.
    LineOrRound,
}

fn check(d: &Doc, e: usize, need: Need) -> Result<usize, String> {
    if d.ent(e).is_none() {
        return Err(format!("no entity {e}"));
    }
    let (ok, what) = match need {
        Need::Point => (d.is_point(e), "a point"),
        Need::Line => (d.is_line(e), "a line"),
        Need::Round => (d.is_circlelike(e), "a circle or arc"),
        Need::Centered => (d.is_circlelike(e) || d.is_conic(e), "a circle, arc or conic"),
        Need::Conic => (d.is_conic(e), "a conic"),
        Need::Curve => (!d.is_point(e), "a curve"),
        Need::LineOrRound => (d.is_line(e) || d.is_circlelike(e), "a line, circle or arc"),
    };
    if ok { Ok(e) } else { Err(format!("entity {e} is not {what}")) }
}

fn changed(ok: bool, what: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(format!("{what} rejected: the constraints would not solve")) }
}

/// Runs the builder `f` as one undoable change and returns what it makes.
/// `f` returns None when its arguments determine nothing; then the document
/// stays as it is.
fn make<T>(sk: &mut Sketch, strict: bool, what: &str, f: impl Fn(&mut Doc) -> Option<T>) -> Result<T, String> {
    if f(&mut sk.doc.clone()).is_none() {
        return Err(format!("{what}: the arguments do not determine one"));
    }
    let mut out = None;
    changed(sk.try_change(strict, |d| out = f(d)), what)?;
    out.ok_or_else(|| format!("{what}: the arguments do not determine one"))
}

fn finite2(p: [f64; 2]) -> Result<[f64; 2], String> {
    if p[0].is_finite() && p[1].is_finite() { Ok(p) } else { Err("coordinates must be finite".into()) }
}

/// Validates a point argument.
fn pt(d: &Doc, p: Pt) -> Result<Pt, String> {
    match p {
        Pt::Ent(e) => {
            check(d, e, Need::Point)?;
        }
        Pt::Free(q) => {
            finite2(q)?;
        }
        Pt::On(c, q) => {
            check(d, c, Need::Curve)?;
            finite2(q)?;
        }
    }
    Ok(p)
}

/// Errors when two arguments are the same existing point.
fn distinct(ps: &[Pt]) -> Result<(), String> {
    for (i, p) in ps.iter().enumerate() {
        for q in &ps[i + 1..] {
            if let (Pt::Ent(a), Pt::Ent(b)) = (p, q) {
                if a == b {
                    return Err("the points must be different".into());
                }
            }
        }
    }
    Ok(())
}

pub fn add_point(sk: &mut Sketch, x: f64, y: f64) -> Result<usize, String> {
    if !(x.is_finite() && y.is_finite()) {
        return Err("coordinates must be finite".into());
    }
    let mut out = 0;
    changed(sk.try_change(false, |d| out = d.add_point(x, y)), "point")?;
    Ok(out)
}

pub fn add_line(sk: &mut Sketch, a: impl Into<Pt>, b: impl Into<Pt>, infinite: bool) -> Result<usize, String> {
    line_between(sk, a, b, infinite, None)
}

/// Like [`add_line`], also constraining the line horizontal (`Some(true)`)
/// or vertical (`Some(false)`).
pub fn line_between(sk: &mut Sketch, a: impl Into<Pt>, b: impl Into<Pt>, infinite: bool, hv: Option<bool>) -> Result<usize, String> {
    let (a, b) = (pt(&sk.doc, a.into())?, pt(&sk.doc, b.into())?);
    if distinct(&[a, b]).is_err() {
        return Err("a line needs two different points".into());
    }
    make(sk, false, "line", |d| {
        let (a, b) = (d.realize(a), d.realize(b));
        let l = d.add_ent(Ent::Line { a, b, infinite });
        match hv {
            Some(true) => d.cons.push(Some(Con::Horizontal(a, b))),
            Some(false) => d.cons.push(Some(Con::Vertical(a, b))),
            None => {}
        }
        Some(l)
    })
}

pub fn add_circle(sk: &mut Sketch, c: impl Into<Pt>, r: f64) -> Result<usize, String> {
    let c = pt(&sk.doc, c.into())?;
    if !(r.is_finite() && r > 0.0) {
        return Err("the radius must be positive".into());
    }
    make(sk, false, "circle", |d| {
        let c = d.realize(c);
        Some(d.add_circle(c, r))
    })
}

/// Arc around point c from point a counterclockwise to point b (|ca| = |cb| is enforced).
pub fn add_arc(sk: &mut Sketch, c: impl Into<Pt>, a: impl Into<Pt>, b: impl Into<Pt>) -> Result<usize, String> {
    let (c, a, b) = (pt(&sk.doc, c.into())?, pt(&sk.doc, a.into())?, pt(&sk.doc, b.into())?);
    if distinct(&[c, a, b]).is_err() {
        return Err("an arc needs three different points".into());
    }
    make(sk, false, "arc", |d| {
        let (c, a, b) = (d.realize(c), d.realize(a), d.realize(b));
        Some(d.add_ent(Ent::Arc { c, a, b }))
    })
}

/// Adds a constraint. Ok(false) when it is rejected (unsatisfiable or redundant).
pub fn constrain(sk: &mut Sketch, kind: ConKind, a: usize, b: usize) -> Result<bool, String> {
    let d = &sk.doc;
    let c = match kind {
        ConKind::Coincident => Con::Coincident(check(d, a, Need::Point)?, check(d, b, Need::Point)?),
        ConKind::OnCurve => Con::OnCurve(check(d, a, Need::Point)?, check(d, b, Need::Curve)?),
        ConKind::Midpoint => Con::Midpoint(check(d, a, Need::Point)?, check(d, b, Need::Line)?),
        ConKind::Horizontal | ConKind::Vertical => {
            let (p, q) = if d.ent(a).is_some() && d.is_line(a) { d.line_pts(a) } else { (check(d, a, Need::Point)?, check(d, b, Need::Point)?) };
            if kind == ConKind::Horizontal { Con::Horizontal(p, q) } else { Con::Vertical(p, q) }
        }
        ConKind::Parallel => Con::Parallel(check(d, a, Need::Line)?, check(d, b, Need::Line)?),
        ConKind::Perpendicular => Con::Perpendicular(check(d, a, Need::Line)?, check(d, b, Need::Line)?),
        ConKind::Collinear => Con::Collinear(check(d, a, Need::Line)?, check(d, b, Need::Line)?),
        ConKind::Equal => Con::Equal(check(d, a, Need::LineOrRound)?, check(d, b, Need::LineOrRound)?),
        ConKind::Concentric => Con::Concentric(check(d, a, Need::Centered)?, check(d, b, Need::Centered)?),
        ConKind::Tangent => {
            check(d, a, Need::Curve)?;
            check(d, b, Need::Curve)?;
            d.tangent_con(a, b).ok_or("these curves cannot be tangent")?
        }
        ConKind::Symmetric
        | ConKind::MidpointOf
        | ConKind::Offset
        | ConKind::Bisector
        | ConKind::TangentAt
        | ConKind::Focus
        | ConKind::Directrix
        | ConKind::Asymptote => {
            return Err(format!("{} is a construction binding, not a constraint to add", kind.name()));
        }
    };
    if let Con::Coincident(p, q) | Con::Horizontal(p, q) | Con::Vertical(p, q) = c {
        if p == q {
            return Err("the constraint needs two different points".into());
        }
    }
    Ok(sk.add_constraint(c))
}

/// Adds several constraints as one undoable change, rejecting them together
/// when any would not solve or is redundant. Ok(false) when rejected.
pub fn add_constraints(sk: &mut Sketch, cons: Vec<Con>) -> Result<bool, String> {
    if cons.is_empty() {
        return Err("no constraints given".into());
    }
    for c in &cons {
        for e in Doc::con_ents(c) {
            if sk.doc.ent(e).is_none() {
                return Err(format!("no entity {e}"));
            }
        }
    }
    Ok(sk.try_change(true, |d| d.cons.extend(cons.into_iter().map(Some))))
}

/// Adds a dimension with its label at `off` from the dimension's anchor
/// (the midpoint of a length, the vertex of an angle). It is driving when
/// that keeps the constraints solvable, otherwise a reference dimension.
/// For an angle, the sector holding the label selects the angle or its
/// supplement.
pub fn add_dim(sk: &mut Sketch, kind: DimType, a: usize, b: usize, off: [f64; 2]) -> Result<usize, String> {
    let d = &sk.doc;
    let k = match kind {
        DimType::Distance => DimKind::Distance(check(d, a, Need::Point)?, check(d, b, Need::Point)?),
        DimType::HDist => DimKind::HDist(check(d, a, Need::Point)?, check(d, b, Need::Point)?),
        DimType::VDist => DimKind::VDist(check(d, a, Need::Point)?, check(d, b, Need::Point)?),
        DimType::PointLine => DimKind::PointLine(check(d, a, Need::Point)?, check(d, b, Need::Line)?),
        DimType::Length => DimKind::Length(check(d, a, Need::Line)?),
        DimType::Radius => DimKind::Radius(check(d, a, Need::Round)?),
        DimType::Diameter => DimKind::Diameter(check(d, a, Need::Round)?),
        DimType::Angle => DimKind::Angle(check(d, a, Need::Line)?, check(d, b, Need::Line)?),
        DimType::ConicA => DimKind::ConicA(check(d, a, Need::Conic)?),
        DimType::ConicB => DimKind::ConicB(check(d, a, Need::Conic)?),
        DimType::CoordX => DimKind::CoordX(check(d, a, Need::Point)?),
        DimType::CoordY => DimKind::CoordY(check(d, a, Need::Point)?),
    };
    if !(off[0].is_finite() && off[1].is_finite()) {
        return Err("the label offset must be finite".into());
    }
    let anchor = d.dim_anchor(&k);
    sk.add_dimension(k, [anchor[0] + off[0], anchor[1] + off[1]]).ok_or_else(|| "dimension rejected".into())
}

fn dim(sk: &Sketch, i: usize) -> Result<&Dim, String> {
    sk.doc.dims.get(i).and_then(|d| d.as_ref()).ok_or_else(|| format!("no dimension {i}"))
}

/// Sets dimension `i` from text as typed in gxpad: a number (degrees for
/// angles) or an expression drives it, empty text makes it a reference
/// dimension. Ok(false) when the value cannot be reached.
pub fn set_dim(sk: &mut Sketch, i: usize, text: &str) -> Result<bool, String> {
    dim(sk, i)?;
    Ok(sk.set_dim_text(i, text))
}

/// Makes dimension `i` driving or reference. Ok(false) when making it
/// driving is rejected (it is determined by the constraints).
pub fn set_driving(sk: &mut Sketch, i: usize, driving: bool) -> Result<bool, String> {
    if dim(sk, i)?.driving == driving {
        return Ok(true);
    }
    Ok(sk.toggle_driving(i))
}

pub fn dim_driving(sk: &Sketch, i: usize) -> Result<bool, String> {
    Ok(dim(sk, i)?.driving)
}

/// Marks dimension `i` numeric or not. Formulas derived for other targets
/// use a numeric dimension's current value as a plain number, so neither its
/// name nor what it is computed from appears in them. The dimension itself
/// is unchanged, so turning the flag off restores the formulas.
pub fn set_dim_numeric(sk: &mut Sketch, i: usize, on: bool) -> Result<bool, String> {
    if dim(sk, i)?.numeric == on {
        return Ok(true);
    }
    Ok(sk.try_change(false, |d| d.dims[i].as_mut().unwrap().numeric = on))
}

/// True when dimension `i` is numeric (substituted as a number in formulas).
pub fn dim_numeric(sk: &Sketch, i: usize) -> Result<bool, String> {
    Ok(dim(sk, i)?.numeric)
}

/// Toggles the supplementary-angle flag of angle dimension `i` (show the
/// angle or its supplement); a driving dimension with a plain value is
/// flipped to the supplement's value.
pub fn toggle_supp(sk: &mut Sketch, i: usize) -> Result<bool, String> {
    let d = dim(sk, i)?;
    if !d.kind.is_angle() {
        return Err(format!("dimension {i} is not an angle"));
    }
    let cur = sk.doc.params[d.param].v;
    let driving = d.driving;
    let p = d.param;
    Ok(sk.try_change(false, |d| {
        let x = d.dims[i].as_mut().unwrap();
        x.supp = !x.supp;
        if driving && x.expr.is_empty() {
            d.params[p].v = std::f64::consts::PI - cur;
        }
    }))
}

/// Gives point, dimension or variable `old` the name `new`. Ok(false) when
/// `new` is not an identifier or is taken.
pub fn rename(sk: &mut Sketch, old: &str, new: &str) -> Result<bool, String> {
    if old.is_empty() {
        return Err("empty name".into());
    }
    let d = &sk.doc;
    let new_owned = new.trim().to_string();
    if let Some(e) = d.ents.iter().position(|e| e.as_ref().is_some_and(|e| e.name == old)) {
        return Ok(sk.rename(old, new, |d| d.ents[e].as_mut().unwrap().name = new_owned));
    }
    if let Some(i) = d.dims.iter().position(|x| x.as_ref().is_some_and(|x| x.name == old)) {
        return Ok(sk.rename(old, new, |d| d.dims[i].as_mut().unwrap().name = new_owned));
    }
    if let Some(i) = d.vars.iter().position(|x| x.as_ref().is_some_and(|x| x.name == old)) {
        return Ok(sk.rename(old, new, |d| d.vars[i].as_mut().unwrap().name = new_owned));
    }
    Err(format!("nothing is named {old:?}"))
}

fn var(sk: &Sketch, i: usize) -> Result<&Var, String> {
    sk.doc.vars.get(i).and_then(|v| v.as_ref()).ok_or_else(|| format!("no variable {i}"))
}

/// A variable that holds a value (no expression) and the parameter storing it.
fn value_var(sk: &Sketch, i: usize) -> Result<usize, String> {
    let v = var(sk, i)?;
    if v.expr.is_empty() { Ok(v.param) } else { Err(format!("variable {i} is defined by an expression")) }
}

fn new_var_name(sk: &Sketch, name: Option<&str>, auto: impl FnOnce(&Doc) -> String) -> Result<String, String> {
    match name.map(str::trim) {
        None | Some("") => Ok(auto(&sk.doc)),
        Some(n) if valid_ident(n) && !sk.doc.name_taken(n) => Ok(n.to_string()),
        Some(n) => Err(format!("bad or taken name {n:?}")),
    }
}

/// Adds a value variable. Without a name, the first unused of a, b, c, k,
/// ... is taken. A free variable (`free`) is set by `value`, otherwise it
/// is solved from the constraints, starting at `value`. Returns the variable.
pub fn add_var(sk: &mut Sketch, name: Option<&str>, value: f64, free: bool) -> Result<usize, String> {
    if !value.is_finite() {
        return Err("value is not finite".into());
    }
    let name = new_var_name(sk, name, |d| {
        ["a", "b", "c", "k", "m", "n", "s", "t", "u", "w"].iter().find(|n| !d.name_taken(n)).map(|s| s.to_string()).unwrap_or_else(|| d.unique_name("k"))
    })?;
    let i = sk.doc.vars.len();
    match sk.try_change(false, |d| {
        let i = d.add_var(name, value);
        let p = d.vars[i].as_ref().unwrap().param;
        d.params[p].locked = free;
    }) {
        true => Ok(i),
        false => Err("variable rejected".into()),
    }
}

/// Adds a variable defined by the expression `expr` of variables and
/// dimensions. Without a name, f1, f2, ... is taken. Returns the variable.
pub fn add_defvar(sk: &mut Sketch, name: Option<&str>, expr: &str) -> Result<usize, String> {
    sk.doc.parse_expr(expr, 0)?;
    let name = new_var_name(sk, name, |d| d.unique_name("f"))?;
    let (i, e) = (sk.doc.vars.len(), expr.to_string());
    match sk.try_change(false, |d| {
        let i = d.add_var(name, 0.0);
        d.vars[i].as_mut().unwrap().expr = e;
    }) {
        true => Ok(i),
        false => Err("variable rejected".into()),
    }
}

/// Sets the value of value variable `i`. Ok(false) when the value cannot be
/// reached.
pub fn set_var(sk: &mut Sketch, i: usize, value: f64) -> Result<bool, String> {
    let p = value_var(sk, i)?;
    if !value.is_finite() {
        return Err("value is not finite".into());
    }
    Ok(sk.set_values(&[(p, value)], true))
}

/// Sets variable `i` from text as typed in gxpad: a plain number makes it a
/// free value variable, any other text an expression. Ok(false) when the
/// change is rejected.
pub fn set_var_expr(sk: &mut Sketch, i: usize, text: &str) -> Result<bool, String> {
    let p = var(sk, i)?.param;
    if let Some(x) = plain_number(text) {
        return Ok(sk.try_change(false, |d| {
            d.vars[i].as_mut().unwrap().expr.clear();
            d.params[p].v = x;
            d.params[p].locked = true;
        }));
    }
    sk.doc.parse_expr(text, 0)?;
    let e = text.to_string();
    Ok(sk.try_change(false, |d| d.vars[i].as_mut().unwrap().expr = e))
}

/// Makes value variable `i` free (set by its value) or solved from the
/// constraints.
pub fn set_var_free(sk: &mut Sketch, i: usize, free: bool) -> Result<bool, String> {
    let p = value_var(sk, i)?;
    Ok(sk.try_change(false, |d| d.params[p].locked = free))
}

/// Removes variable `i`, leaving an empty slot. Expressions naming it are
/// left as they are and fail to parse until edited. Ok(false) when the
/// constraints would no longer solve; the document is then unchanged.
pub fn delete_var(sk: &mut Sketch, i: usize) -> Result<bool, String> {
    var(sk, i)?;
    Ok(sk.delete(&[], &[], &[], &[i]))
}

pub fn var_value(sk: &Sketch, i: usize) -> Result<f64, String> {
    Ok(sk.doc.params[var(sk, i)?.param].v)
}

/// The expression of variable `i`; empty for a value variable.
pub fn var_expr(sk: &Sketch, i: usize) -> Result<String, String> {
    Ok(var(sk, i)?.expr.clone())
}

/// True when variable `i` is free, false when it is solved or defined.
pub fn var_free(sk: &Sketch, i: usize) -> Result<bool, String> {
    let v = var(sk, i)?;
    Ok(v.expr.is_empty() && sk.doc.params[v.param].locked)
}

/// Marks variable `i` numeric or not. Formulas derived for other targets
/// use a numeric variable's current value as a plain number, so neither its
/// name nor its defining expression appears in them. The variable itself is
/// unchanged (a free variable stays free, a solved one solved, a defined one
/// keeps its expression), so turning the flag off restores the formulas.
pub fn set_var_numeric(sk: &mut Sketch, i: usize, on: bool) -> Result<bool, String> {
    if var(sk, i)?.numeric == on {
        return Ok(true);
    }
    Ok(sk.try_change(false, |d| d.vars[i].as_mut().unwrap().numeric = on))
}

/// True when variable `i` is numeric (substituted as a number in formulas).
pub fn var_numeric(sk: &Sketch, i: usize) -> Result<bool, String> {
    Ok(var(sk, i)?.numeric)
}

// ------------------------------------------------------------ constructions

/// Adds a point at (x, y) on curve c, bound to it by a constraint.
pub fn add_point_on(sk: &mut Sketch, c: usize, x: f64, y: f64) -> Result<usize, String> {
    let p = pt(&sk.doc, Pt::On(c, [x, y]))?;
    make(sk, false, "point", |d| Some(d.realize(p)))
}

/// Adds a conic with center point c, half-axes a and b (b is ignored for a
/// parabola, where a is the focal distance) and axis angle t in radians.
pub fn add_conic(sk: &mut Sketch, kind: ConicKind, c: impl Into<Pt>, a: f64, b: f64, t: f64) -> Result<usize, String> {
    let c = pt(&sk.doc, c.into())?;
    if !(a.is_finite() && a > 0.0 && b.is_finite() && b > 0.0 && t.is_finite()) {
        return Err("the half-axes must be positive".into());
    }
    let b = if kind == ConicKind::Parabola { 1.0 } else { b };
    make(sk, false, "conic", |d| {
        let c = d.realize(c);
        Some(d.add_conic(kind, c, a, b, t))
    })
}

/// Adds the curve `y = f(x)` or `F(x, y) = G(x, y)` given as text.
pub fn add_curve(sk: &mut Sketch, src: &str) -> Result<usize, String> {
    sk.doc.curve_def(src)?;
    let src = src.trim().to_string();
    make(sk, false, "curve", |d| Some(d.add_ent(Ent::Curve { src: src.clone() })))
}

/// Sets the text of curve `i`. Ok(false) when the change is rejected.
pub fn set_curve(sk: &mut Sketch, i: usize, src: &str) -> Result<bool, String> {
    if !matches!(sk.doc.kind(i), Some(Ent::Curve { .. })) {
        return Err(format!("entity {i} is not a function curve"));
    }
    sk.doc.curve_def(src)?;
    let src = src.trim().to_string();
    Ok(sk.try_change(false, |d| d.ents[i].as_mut().unwrap().kind = Ent::Curve { src }))
}

/// Adds the midpoint of points a and b, bound to them by a constraint.
pub fn midpoint(sk: &mut Sketch, a: impl Into<Pt>, b: impl Into<Pt>) -> Result<usize, String> {
    let (a, b) = (pt(&sk.doc, a.into())?, pt(&sk.doc, b.into())?);
    distinct(&[a, b])?;
    make(sk, false, "midpoint", |d| {
        let (a, b) = (d.realize(a), d.realize(b));
        Some(d.build_midpoint(a, b))
    })
}

/// Adds the midpoint of line l.
pub fn line_midpoint(sk: &mut Sketch, l: usize) -> Result<usize, String> {
    check(&sk.doc, l, Need::Line)?;
    make(sk, false, "midpoint", |d| Some(d.build_midpoint_of_line(l)))
}

/// Adds a point at the crossing of curves a and b, starting from `guess`.
pub fn intersection(sk: &mut Sketch, a: usize, b: usize, guess: [f64; 2]) -> Result<usize, String> {
    check(&sk.doc, a, Need::Curve)?;
    check(&sk.doc, b, Need::Curve)?;
    if a == b {
        return Err("an intersection needs two different curves".into());
    }
    finite2(guess)?;
    make(sk, true, "intersection", |d| Some(d.build_intersection(a, b, guess)))
}

/// Like [`intersection`], starting from the crossing of the curves nearest
/// to `hint`.
pub fn intersect_near(sk: &mut Sketch, a: usize, b: usize, hint: [f64; 2]) -> Result<usize, String> {
    check(&sk.doc, a, Need::Curve)?;
    check(&sk.doc, b, Need::Curve)?;
    finite2(hint)?;
    let x = crossing_near(&sk.doc, a, b, hint).ok_or("the curves do not cross")?;
    intersection(sk, a, b, x)
}

/// Adds the infinite line through point p perpendicular (`perp`) or
/// parallel to line l.
pub fn dir_line(sk: &mut Sketch, l: usize, p: impl Into<Pt>, perp: bool) -> Result<usize, String> {
    check(&sk.doc, l, Need::Line)?;
    let p = pt(&sk.doc, p.into())?;
    make(sk, false, if perp { "perpendicular line" } else { "parallel line" }, |d| {
        let p = d.realize(p);
        Some(d.build_dir_line(l, p, perp))
    })
}

/// Adds the perpendicular bisector of points a and b; returns (line, midpoint).
pub fn perp_bisector(sk: &mut Sketch, a: impl Into<Pt>, b: impl Into<Pt>) -> Result<(usize, usize), String> {
    let (a, b) = (pt(&sk.doc, a.into())?, pt(&sk.doc, b.into())?);
    distinct(&[a, b])?;
    make(sk, false, "perpendicular bisector", |d| {
        let (a, b) = (d.realize(a), d.realize(b));
        Some(d.build_perp_bisector(a, b))
    })
}

/// Adds the bisector of lines l1 and l2 in the sector containing `toward`;
/// returns (line, vertex). A shared endpoint is the vertex when there is
/// one, otherwise the vertex is a new point at the crossing.
pub fn angle_bisector(sk: &mut Sketch, l1: usize, l2: usize, toward: [f64; 2]) -> Result<(usize, usize), String> {
    check(&sk.doc, l1, Need::Line)?;
    check(&sk.doc, l2, Need::Line)?;
    finite2(toward)?;
    if l1 == l2 {
        return Err("a bisector needs two different lines".into());
    }
    make(sk, false, "angle bisector", |d| d.build_angle_bisector(l1, l2, toward))
}

fn tangent_target(d: &Doc, c: usize) -> Result<usize, String> {
    check(d, c, Need::Curve)?;
    if d.is_line(c) { Err(format!("entity {c} is a line")) } else { Ok(c) }
}

/// Adds the segment from point p touching curve c at a new contact point
/// starting at `guess`; returns (segment, contact point).
pub fn tangent_line(sk: &mut Sketch, p: impl Into<Pt>, c: usize, guess: [f64; 2]) -> Result<(usize, usize), String> {
    let p = pt(&sk.doc, p.into())?;
    tangent_target(&sk.doc, c)?;
    finite2(guess)?;
    make(sk, true, "tangent line", |d| {
        let p = d.realize(p);
        Some(d.build_tangent_line(p, c, guess))
    })
}

/// Like [`tangent_line`], touching the curve at the contact nearest to `hint`.
pub fn tangent_line_near(sk: &mut Sketch, p: impl Into<Pt>, c: usize, hint: [f64; 2]) -> Result<(usize, usize), String> {
    let p = pt(&sk.doc, p.into())?;
    tangent_target(&sk.doc, c)?;
    finite2(hint)?;
    let t = tangent_contact_near(&sk.doc, sk.doc.pt_pos(p), c, hint).ok_or("no tangent from this point")?;
    tangent_line(sk, p, c, t)
}

/// Adds the mirror image of point p in line l.
pub fn reflection(sk: &mut Sketch, p: impl Into<Pt>, l: usize) -> Result<usize, String> {
    let p = pt(&sk.doc, p.into())?;
    check(&sk.doc, l, Need::Line)?;
    make(sk, true, "reflection", |d| {
        let p = d.realize(p);
        Some(d.build_reflection(p, l))
    })
}

/// Adds the circle through points a, b and c with a new center point;
/// returns (circle, center).
pub fn circle_through(sk: &mut Sketch, a: impl Into<Pt>, b: impl Into<Pt>, c: impl Into<Pt>) -> Result<(usize, usize), String> {
    let ps = [pt(&sk.doc, a.into())?, pt(&sk.doc, b.into())?, pt(&sk.doc, c.into())?];
    distinct(&ps)?;
    make(sk, false, "circle through three points", |d| {
        let v = ps.map(|p| d.realize(p));
        d.build_circle_through(v[0], v[1], v[2])
    })
}

/// Adds the arc from point a through point m to point b with a new center
/// point; returns (arc, center).
pub fn arc_through(sk: &mut Sketch, a: impl Into<Pt>, m: impl Into<Pt>, b: impl Into<Pt>) -> Result<(usize, usize), String> {
    let ps = [pt(&sk.doc, a.into())?, pt(&sk.doc, m.into())?, pt(&sk.doc, b.into())?];
    distinct(&ps)?;
    make(sk, false, "arc through three points", |d| {
        let v = ps.map(|p| d.realize(p));
        d.build_arc_through(v[0], v[1], v[2])
    })
}

/// Adds the circle around point c through point `through`, bound to it
/// when it is an existing point or a point on a curve.
pub fn circle_to(sk: &mut Sketch, c: impl Into<Pt>, through: impl Into<Pt>) -> Result<usize, String> {
    let (c, t) = (pt(&sk.doc, c.into())?, pt(&sk.doc, through.into())?);
    distinct(&[c, t])?;
    make(sk, false, "circle", |d| {
        let pos = d.pt_pos(t);
        let c = d.realize(c);
        let ci = d.build_circle_to(c, pos);
        if !matches!(t, Pt::Free(_)) {
            let e = d.realize(t);
            d.cons.push(Some(Con::OnCurve(e, ci)));
        }
        Some(ci)
    })
}

/// Adds the arc around point c from point a, ending at a new point in the
/// direction of `toward`.
pub fn arc_toward(sk: &mut Sketch, c: impl Into<Pt>, a: impl Into<Pt>, toward: [f64; 2]) -> Result<usize, String> {
    let (c, a) = (pt(&sk.doc, c.into())?, pt(&sk.doc, a.into())?);
    finite2(toward)?;
    distinct(&[c, a])?;
    make(sk, false, "arc", |d| {
        let (c, a) = (d.realize(c), d.realize(a));
        let b = d.build_arc_end(c, a, toward);
        Some(d.add_ent(Ent::Arc { c, a, b }))
    })
}

fn not_parabola(kind: ConicKind) -> Result<(), String> {
    if kind == ConicKind::Parabola { Err("an ellipse or hyperbola is needed".into()) } else { Ok(()) }
}

/// Adds the ellipse or hyperbola with foci f1 and f2 through point p, with
/// a new center point.
pub fn conic_by_foci(sk: &mut Sketch, kind: ConicKind, f1: impl Into<Pt>, f2: impl Into<Pt>, p: impl Into<Pt>) -> Result<usize, String> {
    not_parabola(kind)?;
    let ps = [pt(&sk.doc, f1.into())?, pt(&sk.doc, f2.into())?, pt(&sk.doc, p.into())?];
    distinct(&ps)?;
    make(sk, false, "conic by foci", |d| {
        let v = ps.map(|p| d.realize(p));
        d.build_conic_by_foci(kind, v[0], v[1], v[2])
    })
}

/// Adds the parabola with focus f and directrix line l, with a new vertex.
pub fn parabola_by_focus(sk: &mut Sketch, f: impl Into<Pt>, l: usize) -> Result<usize, String> {
    let f = pt(&sk.doc, f.into())?;
    check(&sk.doc, l, Need::Line)?;
    make(sk, false, "parabola", |d| {
        let f = d.realize(f);
        d.build_parabola_by_focus(f, l)
    })
}

/// Adds the ellipse or hyperbola centered on point c, its first axis ending
/// at `axis` and its second half-axis reaching the line through `side`
/// parallel to the first axis.
pub fn conic_from(sk: &mut Sketch, kind: ConicKind, c: impl Into<Pt>, axis: [f64; 2], side: [f64; 2]) -> Result<usize, String> {
    not_parabola(kind)?;
    let c = pt(&sk.doc, c.into())?;
    finite2(axis)?;
    finite2(side)?;
    make(sk, false, "conic", |d| {
        let c = d.realize(c);
        Some(d.build_conic_from(kind, c, axis, side))
    })
}

/// Adds the parabola with vertex at point c opening toward `axis`, its
/// focal distance |c axis|.
pub fn parabola_toward(sk: &mut Sketch, c: impl Into<Pt>, axis: [f64; 2]) -> Result<usize, String> {
    let c = pt(&sk.doc, c.into())?;
    finite2(axis)?;
    make(sk, false, "parabola", |d| {
        let c = d.realize(c);
        Some(d.build_parabola_toward(c, axis))
    })
}

/// Adds the conic feature `f` of conic c (for a directrix, that of the focus
/// nearest `near`; asymptotes need a hyperbola); returns the features.
/// Features already in the document are returned without being added again.
pub fn conic_feature(sk: &mut Sketch, f: ConicFeature, c: usize, near: [f64; 2]) -> Result<Vec<usize>, String> {
    check(&sk.doc, c, Need::Conic)?;
    if f == ConicFeature::Asymptotes && !matches!(sk.doc.kind(c), Some(Ent::Conic { kind: ConicKind::Hyperbola, .. })) {
        return Err(format!("entity {c} is not a hyperbola"));
    }
    finite2(near)?;
    make(sk, true, "conic feature", |d| Some(d.build_conic_feature(f, c, near)).filter(|v| !v.is_empty()))
}

/// Index of a conic kind: 0 ellipse, 1 hyperbola, 2 parabola.
pub fn conic_kind(i: u32) -> Result<ConicKind, String> {
    [ConicKind::Ellipse, ConicKind::Hyperbola, ConicKind::Parabola].get(i as usize).copied().ok_or_else(|| format!("unknown conic kind {i}"))
}

/// Conic kind from its name.
pub fn conic_kind_named(s: &str) -> Result<ConicKind, String> {
    match s {
        "ellipse" => Ok(ConicKind::Ellipse),
        "hyperbola" => Ok(ConicKind::Hyperbola),
        "parabola" => Ok(ConicKind::Parabola),
        _ => Err(format!("unknown conic kind {s:?}")),
    }
}

/// Index of a conic feature: 0 foci, 1 directrix, 2 asymptotes.
pub fn conic_feature_kind(i: u32) -> Result<ConicFeature, String> {
    [ConicFeature::Foci, ConicFeature::Directrix, ConicFeature::Asymptotes].get(i as usize).copied().ok_or_else(|| format!("unknown conic feature {i}"))
}

// ------------------------------------------------------------ deleting

/// Deletes entity `e` and what depends on it (constraints and dimensions that
/// reference it, owned points, and features built from it). Ok(false) when
/// the constraints would no longer solve; the document is then unchanged.
pub fn delete_ent(sk: &mut Sketch, e: usize) -> Result<bool, String> {
    if sk.doc.ent(e).is_none() {
        return Err(format!("no entity {e}"));
    }
    Ok(sk.delete(&[e], &[], &[], &[]))
}

/// Deletes constraint `c`. Ok(false) when the constraints would no longer
/// solve; the document is then unchanged.
pub fn delete_con(sk: &mut Sketch, c: usize) -> Result<bool, String> {
    if sk.doc.cons.get(c).is_none_or(|x| x.is_none()) {
        return Err(format!("no constraint {c}"));
    }
    Ok(sk.delete(&[], &[c], &[], &[]))
}

/// Deletes dimension `i`. Expressions naming it are left as they are and
/// fail to parse until edited. Ok(false) when the constraints would no
/// longer solve; the document is then unchanged.
pub fn delete_dim(sk: &mut Sketch, i: usize) -> Result<bool, String> {
    dim(sk, i)?;
    Ok(sk.delete(&[], &[], &[i], &[]))
}

// ------------------------------------------------------------ flags and dragging

/// Locks point `e` (its coordinates become fixed) or unlocks it (they are
/// solved from the constraints again).
pub fn lock_point(sk: &mut Sketch, e: usize, on: bool) -> Result<bool, String> {
    lock_points(sk, &[e], on)
}

/// Locks or unlocks the given points as one undoable change.
pub fn lock_points(sk: &mut Sketch, pts: &[usize], on: bool) -> Result<bool, String> {
    for &e in pts {
        if !sk.doc.is_point(e) {
            return Err(format!("entity {e} is not a point"));
        }
    }
    Ok(sk.try_change(false, |d| {
        for &e in pts {
            d.lock_point(e, on);
        }
    }))
}

/// True when point `e` is locked (both coordinates fixed).
pub fn point_locked(sk: &Sketch, e: usize) -> Result<bool, String> {
    if !sk.doc.is_point(e) {
        return Err(format!("entity {e} is not a point"));
    }
    Ok(sk.doc.point_locked(e))
}

/// Sets the construction flag of entity `e` (a line, circle, arc, conic or
/// curve drawn as a dashed helper). Points have no construction flag.
pub fn set_construction(sk: &mut Sketch, e: usize, on: bool) -> Result<bool, String> {
    set_construction_all(sk, &[e], on)
}

/// Sets the construction flag of the given non-point entities as one
/// undoable change.
pub fn set_construction_all(sk: &mut Sketch, ents: &[usize], on: bool) -> Result<bool, String> {
    for &e in ents {
        if sk.doc.ent(e).is_none() {
            return Err(format!("no entity {e}"));
        }
        if sk.doc.is_point(e) {
            return Err(format!("entity {e} is a point"));
        }
    }
    Ok(sk.try_change(false, |d| {
        for &e in ents {
            d.ents[e].as_mut().unwrap().construction = on;
        }
    }))
}

/// True when entity `e` is a construction (dashed helper).
pub fn ent_construction(sk: &Sketch, e: usize) -> Result<bool, String> {
    Ok(ent_of(&sk.doc, e)?.construction)
}

/// Sets the hidden flag of entity `e`. Hidden entities are not drawn.
pub fn set_hidden(sk: &mut Sketch, e: usize, on: bool) -> Result<bool, String> {
    if sk.doc.ent(e).is_none() {
        return Err(format!("no entity {e}"));
    }
    Ok(sk.try_change(false, |d| d.ents[e].as_mut().unwrap().hidden = on))
}

/// True when entity `e` is hidden.
pub fn ent_hidden(sk: &Sketch, e: usize) -> Result<bool, String> {
    Ok(ent_of(&sk.doc, e)?.hidden)
}

/// Makes line `e` infinite or a segment.
pub fn set_infinite(sk: &mut Sketch, e: usize, on: bool) -> Result<bool, String> {
    if !sk.doc.is_line(e) {
        return Err(format!("entity {e} is not a line"));
    }
    Ok(sk.try_change(false, |d| {
        if let Some(Entity { kind: Ent::Line { infinite, .. }, .. }) = d.ents[e].as_mut() {
            *infinite = on;
        }
    }))
}

/// Switches each of the given lines between infinite and segment, as one
/// undoable change.
pub fn toggle_infinite(sk: &mut Sketch, lines: &[usize]) -> Result<bool, String> {
    for &e in lines {
        if !sk.doc.is_line(e) {
            return Err(format!("entity {e} is not a line"));
        }
    }
    Ok(sk.try_change(false, |d| {
        for &e in lines {
            if let Some(Entity { kind: Ent::Line { infinite, .. }, .. }) = d.ents[e].as_mut() {
                *infinite = !*infinite;
            }
        }
    }))
}

/// True when line `e` is infinite.
pub fn ent_infinite(sk: &Sketch, e: usize) -> Result<bool, String> {
    match sk.doc.kind(e) {
        Some(Ent::Line { infinite, .. }) => Ok(*infinite),
        _ => Err(format!("entity {e} is not a line")),
    }
}

/// Pulls point `e` toward (x, y) by solving the constraints with the point
/// pinned there; a drag in the sense of `Sketch::drag_solve`, not an undoable
/// change. Returns Ok(false) when the configuration cannot reach the target.
pub fn drag_point(sk: &mut Sketch, e: usize, x: f64, y: f64) -> Result<bool, String> {
    let d = &sk.doc;
    if !d.is_point(e) {
        return Err(format!("entity {e} is not a point"));
    }
    if !(x.is_finite() && y.is_finite()) {
        return Err("coordinates must be finite".into());
    }
    let (px, py) = d.pexpr(e);
    sk.drag_solve(&[px - crate::expr::float_raw(x), py - crate::expr::float_raw(y)], [x, y]);
    let p = sk.doc.pval(e);
    Ok((p[0] - x).hypot(p[1] - y) < 1e-6)
}

/// Pulls the radius of circle `e` so its edge passes through (x, y), solving
/// the constraints (a drag, not an undoable change). Returns Ok(false) when
/// the radius cannot reach that value.
pub fn drag_circle_edge(sk: &mut Sketch, e: usize, x: f64, y: f64) -> Result<bool, String> {
    let r_param = match sk.doc.kind(e) {
        Some(Ent::Circle { r, .. }) => *r,
        _ => return Err(format!("entity {e} is not a circle")),
    };
    let c = sk.doc.circ_val(e).0;
    let target = (x - c[0]).hypot(y - c[1]);
    sk.drag_solve(&[crate::expr::sym(r_param as u32) - crate::expr::float_raw(target)], [x, y]);
    let rv = sk.doc.circ_val(e).1;
    Ok((rv - target).abs() < 1e-6)
}

/// Pulls a handle of conic `e`: `which` 0 drags the first-axis end (its
/// length and angle), 1 drags the second-axis end (its length). A parabola
/// has only handle 0. A drag, not an undoable change; Ok(false) when the
/// handle cannot reach (x, y).
pub fn drag_conic_handle(sk: &mut Sketch, e: usize, which: u8, x: f64, y: f64) -> Result<bool, String> {
    let (c, a, b, t) = match sk.doc.kind(e) {
        Some(Ent::Conic { c, a, b, t, .. }) => (*c, *a, *b, *t),
        _ => return Err(format!("entity {e} is not a conic")),
    };
    if which > 1 || (which == 1 && matches!(sk.doc.kind(e), Some(Ent::Conic { kind: ConicKind::Parabola, .. }))) {
        return Err(format!("conic {e} has no handle {which}"));
    }
    let cc = sk.doc.pval(c);
    let (dx, dy) = (x - cc[0], y - cc[1]);
    let tv = sk.doc.params[t].v;
    let (exprs, want) = if which == 0 {
        let mut ang = dy.atan2(dx);
        while ang - tv > std::f64::consts::PI {
            ang -= std::f64::consts::TAU;
        }
        while ang - tv < -std::f64::consts::PI {
            ang += std::f64::consts::TAU;
        }
        let dist = dx.hypot(dy).max(1e-6);
        (vec![crate::expr::sym(a as u32) - crate::expr::float_raw(dist), crate::expr::sym(t as u32) - crate::expr::float_raw(ang)], dist)
    } else {
        let bv = (-dx * tv.sin() + dy * tv.cos()).abs().max(1e-6);
        (vec![crate::expr::sym(b as u32) - crate::expr::float_raw(bv)], bv)
    };
    sk.drag_solve(&exprs, [x, y]);
    let got = if which == 0 { sk.doc.params[a].v } else { sk.doc.params[b].v };
    Ok((got - want).abs() < 1e-6)
}

// ------------------------------------------------------------ reading

/// Kind of an entity.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EntKind {
    /// A point.
    Point,
    /// A segment or infinite line.
    Line,
    /// A circle.
    Circle,
    /// An arc.
    Arc,
    /// A conic (ellipse, hyperbola or parabola).
    Conic,
    /// A curve given as `y = f(x)` or `F(x, y) = G(x, y)`.
    Curve,
}

impl EntKind {
    pub const ALL: [EntKind; 6] = [
        EntKind::Point,
        EntKind::Line,
        EntKind::Circle,
        EntKind::Arc,
        EntKind::Conic,
        EntKind::Curve,
    ];

    pub fn name(self) -> &'static str {
        match self {
            EntKind::Point => "point",
            EntKind::Line => "line",
            EntKind::Circle => "circle",
            EntKind::Arc => "arc",
            EntKind::Conic => "conic",
            EntKind::Curve => "curve",
        }
    }

    pub fn from_index(i: u32) -> Result<EntKind, String> {
        EntKind::ALL.get(i as usize).copied().ok_or_else(|| format!("unknown entity kind {i}"))
    }

    pub fn from_name(s: &str) -> Result<EntKind, String> {
        EntKind::ALL.into_iter().find(|k| k.name() == s).ok_or_else(|| format!("unknown entity kind {s:?}"))
    }
}

fn ent_of(d: &Doc, e: usize) -> Result<&Entity, String> {
    d.ent(e).ok_or_else(|| format!("no entity {e}"))
}

/// Number of entity slots. Deleted entities leave empty slots.
pub fn ent_count(sk: &Sketch) -> usize {
    sk.doc.ents.len()
}

/// Kind of entity `e`.
pub fn ent_kind(sk: &Sketch, e: usize) -> Result<EntKind, String> {
    Ok(match ent_of(&sk.doc, e)?.kind {
        Ent::Point { .. } => EntKind::Point,
        Ent::Line { .. } => EntKind::Line,
        Ent::Circle { .. } => EntKind::Circle,
        Ent::Arc { .. } => EntKind::Arc,
        Ent::Conic { .. } => EntKind::Conic,
        Ent::Curve { .. } => EntKind::Curve,
    })
}

/// Name of entity `e` (points are named, other entities have an empty name).
pub fn ent_name(sk: &Sketch, e: usize) -> Result<String, String> {
    Ok(ent_of(&sk.doc, e)?.name.clone())
}

/// World coordinates of point `e`.
pub fn point_pos(sk: &Sketch, e: usize) -> Result<[f64; 2], String> {
    if !sk.doc.is_point(e) {
        return Err(format!("entity {e} is not a point"));
    }
    Ok(sk.doc.pval(e))
}

/// Endpoints a and b of line `e` and whether it is infinite.
pub fn line_pts(sk: &Sketch, e: usize) -> Result<(usize, usize, bool), String> {
    match sk.doc.kind(e) {
        Some(Ent::Line { a, b, infinite }) => Ok((*a, *b, *infinite)),
        _ => Err(format!("entity {e} is not a line")),
    }
}

/// Center point and radius of circle or arc `e`.
pub fn circle_vals(sk: &Sketch, e: usize) -> Result<(usize, f64), String> {
    let d = &sk.doc;
    if !d.is_circlelike(e) {
        return Err(format!("entity {e} is not a circle or arc"));
    }
    let c = match d.kind(e) {
        Some(Ent::Circle { c, .. }) | Some(Ent::Arc { c, .. }) => *c,
        _ => unreachable!(),
    };
    Ok((c, d.circ_val(e).1))
}

/// Center point and the two arc endpoints of arc `e`.
pub fn arc_pts(sk: &Sketch, e: usize) -> Result<(usize, usize, usize), String> {
    match sk.doc.kind(e) {
        Some(Ent::Arc { c, a, b }) => Ok((*c, *a, *b)),
        _ => Err(format!("entity {e} is not an arc")),
    }
}

/// Kind, center point and axis values (a, b, angle) of conic `e`.
pub fn conic_vals(sk: &Sketch, e: usize) -> Result<(ConicKind, usize, f64, f64, f64), String> {
    let d = &sk.doc;
    match d.kind(e) {
        Some(Ent::Conic { kind, c, a, b, t }) => Ok((*kind, *c, d.params[*a].v, d.params[*b].v, d.params[*t].v)),
        _ => Err(format!("entity {e} is not a conic")),
    }
}

/// Source text of function curve `e`.
pub fn curve_src(sk: &Sketch, e: usize) -> Result<String, String> {
    match sk.doc.kind(e) {
        Some(Ent::Curve { src }) => Ok(src.clone()),
        _ => Err(format!("entity {e} is not a function curve")),
    }
}

/// Number of constraint slots. Deleted constraints leave empty slots.
pub fn con_count(sk: &Sketch) -> usize {
    sk.doc.cons.len()
}

/// The kind of constraint `c`, including the construction bindings that
/// [`constrain`] cannot add.
pub fn con_kind(sk: &Sketch, c: usize) -> Result<ConKind, String> {
    let con = sk.doc.cons.get(c).and_then(|x| x.as_ref()).ok_or_else(|| format!("no constraint {c}"))?;
    Ok(con_kind_of(con))
}

/// The entities of constraint `c`, in the order they appear in it.
pub fn con_ents(sk: &Sketch, c: usize) -> Result<Vec<usize>, String> {
    let con = sk.doc.cons.get(c).and_then(|x| x.as_ref()).ok_or_else(|| format!("no constraint {c}"))?;
    Ok(Doc::con_ents(con))
}

fn con_kind_of(c: &Con) -> ConKind {
    match *c {
        Con::Coincident(..) => ConKind::Coincident,
        Con::OnCurve(..) => ConKind::OnCurve,
        Con::Midpoint(..) => ConKind::Midpoint,
        Con::Horizontal(..) => ConKind::Horizontal,
        Con::Vertical(..) => ConKind::Vertical,
        Con::Parallel(..) => ConKind::Parallel,
        Con::Perpendicular(..) => ConKind::Perpendicular,
        Con::Collinear(..) => ConKind::Collinear,
        Con::Equal(..) => ConKind::Equal,
        Con::Concentric(..) => ConKind::Concentric,
        Con::Tangent { .. } => ConKind::Tangent,
        Con::Symmetric(..) => ConKind::Symmetric,
        Con::MidpointOf(..) => ConKind::MidpointOf,
        Con::Offset { .. } => ConKind::Offset,
        Con::Bisect { .. } => ConKind::Bisector,
        Con::TangentAt { .. } => ConKind::TangentAt,
        Con::Focus { .. } => ConKind::Focus,
        Con::Directrix { .. } | Con::DirectrixOf { .. } => ConKind::Directrix,
        Con::Asymptote { .. } => ConKind::Asymptote,
    }
}

/// Kind of dimension `i` (`DimType`).
pub fn dim_kind(sk: &Sketch, i: usize) -> Result<DimType, String> {
    Ok(match dim(sk, i)?.kind {
        DimKind::Distance(..) => DimType::Distance,
        DimKind::HDist(..) => DimType::HDist,
        DimKind::VDist(..) => DimType::VDist,
        DimKind::PointLine(..) => DimType::PointLine,
        DimKind::Length(..) => DimType::Length,
        DimKind::Radius(..) => DimType::Radius,
        DimKind::Diameter(..) => DimType::Diameter,
        DimKind::Angle(..) => DimType::Angle,
        DimKind::ConicA(..) => DimType::ConicA,
        DimKind::ConicB(..) => DimType::ConicB,
        DimKind::CoordX(..) => DimType::CoordX,
        DimKind::CoordY(..) => DimType::CoordY,
    })
}

/// World-coordinate polylines drawing entity `e` in the window from `min`
/// to `max` with pixels of size `px` (see [`draw_polys`]): one point for a
/// point, the endpoints of a segment, a segment crossing the window for an
/// infinite line, the outline of a circle, arc or conic (two polylines for a
/// hyperbola), and the parts of a function curve inside the window.
pub fn ent_samples(sk: &Sketch, e: usize, min: [f64; 2], max: [f64; 2], px: f64) -> Result<Vec<Vec<[f64; 2]>>, String> {
    ent_of(&sk.doc, e)?;
    let w = Window::new(min, max, px)?;
    Ok(draw_polys(&sk.doc, e, &w))
}
