//! Shorthands for reading a document through `ops`, and the plane geometry
//! the canvas computes from what they return.

use crate::ops::{self, ConicKind, Document, EntKind};

const NAN2: [f64; 2] = [f64::NAN; 2];

pub fn kind(d: &Document, e: usize) -> Option<EntKind> {
    ops::ent_kind(d, e).ok()
}

pub fn is_point(d: &Document, e: usize) -> bool {
    kind(d, e) == Some(EntKind::Point)
}

pub fn is_line(d: &Document, e: usize) -> bool {
    kind(d, e) == Some(EntKind::Line)
}

/// A circle or an arc.
pub fn is_circlelike(d: &Document, e: usize) -> bool {
    matches!(kind(d, e), Some(EntKind::Circle | EntKind::Arc))
}

pub fn is_conic(d: &Document, e: usize) -> bool {
    kind(d, e) == Some(EntKind::Conic)
}

/// A function curve (`y = f(x)` or an implicit equation).
pub fn is_function(d: &Document, e: usize) -> bool {
    kind(d, e) == Some(EntKind::Curve)
}

/// Any entity other than a point.
pub fn is_curve(d: &Document, e: usize) -> bool {
    kind(d, e).is_some_and(|k| k != EntKind::Point)
}

/// The origin or an axis.
pub fn is_fixed(d: &Document, e: usize) -> bool {
    e == ops::origin(d) || e == ops::x_axis(d) || e == ops::y_axis(d)
}

pub fn is_axis(d: &Document, e: usize) -> bool {
    e == ops::x_axis(d) || e == ops::y_axis(d)
}

pub fn hidden(d: &Document, e: usize) -> bool {
    ops::ent_hidden(d, e).unwrap_or(true)
}

/// Position of point `p` (NaN for anything else).
pub fn pos(d: &Document, p: usize) -> [f64; 2] {
    ops::point_pos(d, p).unwrap_or(NAN2)
}

/// End points of line `l`.
pub fn line_pts(d: &Document, l: usize) -> (usize, usize) {
    ops::line_pts(d, l).map(|(a, b, _)| (a, b)).unwrap_or((usize::MAX, usize::MAX))
}

/// Positions of the end points of line `l`.
pub fn line_ends(d: &Document, l: usize) -> ([f64; 2], [f64; 2]) {
    let (a, b) = line_pts(d, l);
    (pos(d, a), pos(d, b))
}

pub fn is_segment(d: &Document, l: usize) -> bool {
    ops::line_pts(d, l).is_ok_and(|(_, _, inf)| !inf)
}

/// Direction of line `l` (from its first to its second point).
pub fn line_dir(d: &Document, l: usize) -> [f64; 2] {
    let (a, b) = line_ends(d, l);
    [b[0] - a[0], b[1] - a[1]]
}

/// Center position and radius of a circle or arc.
pub fn circle(d: &Document, e: usize) -> ([f64; 2], f64) {
    ops::circle_vals(d, e).map(|(c, r)| (pos(d, c), r)).unwrap_or((NAN2, f64::NAN))
}

/// Kind, center position, semi-axes and axis angle of a conic.
pub fn conic(d: &Document, e: usize) -> Option<(ConicKind, [f64; 2], f64, f64, f64)> {
    ops::conic_vals(d, e).ok().map(|(k, c, a, b, t)| (k, pos(d, c), a, b, t))
}

/// Center of a circle, arc or conic, or the position of a point.
pub fn center(d: &Document, e: usize) -> [f64; 2] {
    match kind(d, e) {
        Some(EntKind::Point) => pos(d, e),
        Some(EntKind::Circle | EntKind::Arc) => circle(d, e).0,
        Some(EntKind::Conic) => conic(d, e).map(|c| c.1).unwrap_or(NAN2),
        _ => NAN2,
    }
}

/// Foot of the perpendicular from `p` onto the line through `a` and `b`.
pub fn foot(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    if l2 < 1e-18 {
        return a;
    }
    let t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2;
    [a[0] + t * dx, a[1] + t * dy]
}

/// Foot of the perpendicular from `p` onto line `l`.
pub fn line_foot(d: &Document, p: [f64; 2], l: usize) -> [f64; 2] {
    let (a, b) = line_ends(d, l);
    foot(p, a, b)
}

/// Unit vector along `v`, None for a zero vector.
pub fn unit(v: [f64; 2]) -> Option<[f64; 2]> {
    let l = v[0].hypot(v[1]);
    (l > 1e-12 && l.is_finite()).then(|| [v[0] / l, v[1] / l])
}

/// Number of function curves among the entities before `e`, for picking
/// its color.
pub fn function_index(d: &Document, e: usize) -> usize {
    (0..e).filter(|&i| is_function(d, i)).count()
}

/// Splits a name into base and subscript for display: `x_A` → (`x`, `A`),
/// `d12` → (`d`, `12`).
pub fn split_sub(s: &str) -> (&str, Option<&str>) {
    if let Some((a, b)) = s.split_once('_') {
        return (a, Some(b));
    }
    let i = s.char_indices().find(|(_, c)| c.is_ascii_digit()).map(|(i, _)| i);
    match i {
        Some(i) if i > 0 && s[i..].chars().all(|c| c.is_ascii_digit()) => (&s[..i], Some(&s[i..])),
        _ => (s, None),
    }
}
