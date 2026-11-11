//! Checked editing operations on a [`Sketch`] for the C and JS interfaces:
//! arguments are validated before the document changes, so a bad index
//! gives an error instead of a panic.

use crate::model::*;
use crate::sketch::Sketch;

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
}

impl ConKind {
    pub const ALL: [ConKind; 11] = [
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

pub fn add_point(sk: &mut Sketch, x: f64, y: f64) -> Result<usize, String> {
    if !(x.is_finite() && y.is_finite()) {
        return Err("coordinates must be finite".into());
    }
    let mut out = 0;
    changed(sk.try_change(false, |d| out = d.add_point(x, y)), "point")?;
    Ok(out)
}

pub fn add_line(sk: &mut Sketch, a: usize, b: usize, infinite: bool) -> Result<usize, String> {
    check(&sk.doc, a, Need::Point)?;
    check(&sk.doc, b, Need::Point)?;
    if a == b {
        return Err("a line needs two different points".into());
    }
    let mut out = 0;
    changed(sk.try_change(false, |d| out = d.add_ent(Ent::Line { a, b, infinite })), "line")?;
    Ok(out)
}

pub fn add_circle(sk: &mut Sketch, c: usize, r: f64) -> Result<usize, String> {
    check(&sk.doc, c, Need::Point)?;
    if !(r.is_finite() && r > 0.0) {
        return Err("the radius must be positive".into());
    }
    let mut out = 0;
    changed(sk.try_change(false, |d| out = d.add_circle(c, r)), "circle")?;
    Ok(out)
}

/// Arc around point c from point a counterclockwise to point b (|ca| = |cb| is enforced).
pub fn add_arc(sk: &mut Sketch, c: usize, a: usize, b: usize) -> Result<usize, String> {
    for p in [c, a, b] {
        check(&sk.doc, p, Need::Point)?;
    }
    if a == b || a == c || b == c {
        return Err("an arc needs three different points".into());
    }
    let mut out = 0;
    changed(sk.try_change(false, |d| out = d.add_ent(Ent::Arc { c, a, b })), "arc")?;
    Ok(out)
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
    };
    if let Con::Coincident(p, q) | Con::Horizontal(p, q) | Con::Vertical(p, q) = c {
        if p == q {
            return Err("the constraint needs two different points".into());
        }
    }
    Ok(sk.add_constraint(c))
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
