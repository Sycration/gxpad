//! C interface. The header `include/gxgeo.h` is generated from this file
//! with `cbindgen --config cbindgen.toml --output include/gxgeo.h` (run in
//! the `gxgeo` directory).
//!
//! Conventions:
//! - Functions that return an index return -1 on failure; functions that
//!   return a status return 1 for success, 0 when the change was rejected
//!   (the constraints would not solve, or it is redundant) and -1 on error;
//!   functions that return a pointer return NULL on failure. After a
//!   failure, `gx_last_error` gives the message on the same thread.
//! - Entities (points, lines, circles, ...), dimensions and variables are
//!   numbered by index. Deleting leaves empty slots, so indices stay valid.
//! - Strings passed in are NUL-terminated UTF-8.
//! - Strings and formulas returned are owned by the caller and released with
//!   `gx_string_free` and `gx_formula_free`; documents with `gx_doc_free`.

use crate::formula::{self, Target};
use crate::gxp;
use crate::model::Doc;
use crate::ops::{self, ConKind, ConicFeature, DimType, EntKind};
use crate::sketch::Sketch;
use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char, c_int};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

/// A geometry document with its solver state and undo history. Opaque.
pub struct GxDoc {
    sk: Sketch,
}

/// Kind of quantity passed to `gx_derive`.
pub type GxTarget = u32;
/// A dimension: the index refers to `gx_doc_dim_*`.
pub const GX_TARGET_DIM: GxTarget = 0;
/// A variable: the index refers to `gx_doc_var_*`.
pub const GX_TARGET_VAR: GxTarget = 1;

/// Constraint kind passed to `gx_doc_constrain`, with its arguments a and b.
pub type GxCon = u32;
/// Points a and b coincide.
pub const GX_CON_COINCIDENT: GxCon = 0;
/// Point a lies on curve b.
pub const GX_CON_ON_CURVE: GxCon = 1;
/// Point a is the midpoint of line b.
pub const GX_CON_MIDPOINT: GxCon = 2;
/// Line a horizontal (b unused), or points a and b level.
pub const GX_CON_HORIZONTAL: GxCon = 3;
/// Line a vertical (b unused), or points a and b above each other.
pub const GX_CON_VERTICAL: GxCon = 4;
/// Lines a and b parallel.
pub const GX_CON_PARALLEL: GxCon = 5;
/// Lines a and b perpendicular.
pub const GX_CON_PERPENDICULAR: GxCon = 6;
/// Lines a and b on one line.
pub const GX_CON_COLLINEAR: GxCon = 7;
/// Lines a and b of equal length, or circles a and b of equal radius.
pub const GX_CON_EQUAL: GxCon = 8;
/// Circles, arcs or conics a and b share a center.
pub const GX_CON_CONCENTRIC: GxCon = 9;
/// Curves a and b tangent, on the side they are on now.
pub const GX_CON_TANGENT: GxCon = 10;
/// Points a and b symmetric about line c (read-only).
pub const GX_CON_SYMMETRIC: GxCon = 11;
/// Point a is the midpoint of points b and c (read-only).
pub const GX_CON_MIDPOINT_OF: GxCon = 12;
/// Point a offset from point b by the direction c to d (read-only).
pub const GX_CON_OFFSET: GxCon = 13;
/// Point a on the bisector of lines b and c through point d (read-only).
pub const GX_CON_BISECTOR: GxCon = 14;
/// The line through a and b touches curve c at b (read-only).
pub const GX_CON_TANGENT_AT: GxCon = 15;
/// Point a is a focus of conic b (read-only).
pub const GX_CON_FOCUS: GxCon = 16;
/// Line a is the directrix of conic c (read-only).
pub const GX_CON_DIRECTRIX: GxCon = 17;
/// Line a is an asymptote of conic b (read-only).
pub const GX_CON_ASYMPTOTE: GxCon = 18;

/// Dimension kind passed to `gx_doc_add_dim`, with its arguments a and b
/// (b unused for kinds with one argument).
pub type GxDim = u32;
/// Distance between points a and b.
pub const GX_DIM_DISTANCE: GxDim = 0;
/// Horizontal distance between points a and b.
pub const GX_DIM_HDIST: GxDim = 1;
/// Vertical distance between points a and b.
pub const GX_DIM_VDIST: GxDim = 2;
/// Distance from point a to line b.
pub const GX_DIM_POINT_LINE: GxDim = 3;
/// Length of line a.
pub const GX_DIM_LENGTH: GxDim = 4;
/// Radius of circle or arc a.
pub const GX_DIM_RADIUS: GxDim = 5;
/// Diameter of circle or arc a.
pub const GX_DIM_DIAMETER: GxDim = 6;
/// Angle between lines a and b.
pub const GX_DIM_ANGLE: GxDim = 7;
/// First half-axis of conic a.
pub const GX_DIM_CONIC_A: GxDim = 8;
/// Second half-axis of conic a.
pub const GX_DIM_CONIC_B: GxDim = 9;
/// x coordinate of point a.
pub const GX_DIM_X: GxDim = 10;
/// y coordinate of point a.
pub const GX_DIM_Y: GxDim = 11;

/// Conic kind passed to `gx_doc_add_conic` and the conic constructions.
pub type GxConic = u32;
/// An ellipse.
pub const GX_CONIC_ELLIPSE: GxConic = 0;
/// A hyperbola.
pub const GX_CONIC_HYPERBOLA: GxConic = 1;
/// A parabola.
pub const GX_CONIC_PARABOLA: GxConic = 2;

/// Entity kind returned by `gx_doc_ent_kind`.
pub type GxEnt = u32;
/// A point.
pub const GX_ENT_POINT: GxEnt = 0;
/// A segment or infinite line.
pub const GX_ENT_LINE: GxEnt = 1;
/// A circle.
pub const GX_ENT_CIRCLE: GxEnt = 2;
/// An arc.
pub const GX_ENT_ARC: GxEnt = 3;
/// A conic (ellipse, hyperbola or parabola).
pub const GX_ENT_CONIC: GxEnt = 4;
/// A curve given as `y = f(x)` or `F(x, y) = G(x, y)`.
pub const GX_ENT_CURVE: GxEnt = 5;

/// A rectangle of the plane, from (x0, y0) to (x1, y1) with x0 < x1 and
/// y0 < y1, and the size of one pixel in it in world units. At most 20 000
/// pixels wide and tall.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GxWindow {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub px: f64,
}

/// A derived formula. Released with `gx_formula_free`.
#[repr(C)]
pub struct GxFormula {
    /// Name of the target.
    pub name: *mut c_char,
    /// `name = formula` as plain text.
    pub plain: *mut c_char,
    /// `name = formula` as LaTeX.
    pub latex: *mut c_char,
    /// False when some part had to be solved numerically.
    pub exact: bool,
    /// True when the target is an angle (in radians).
    pub is_angle: bool,
    /// Formula evaluated at the current configuration.
    pub value: f64,
    /// Measured value of the target.
    pub actual: f64,
}

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_error(msg: impl Into<String>) {
    LAST_ERROR.with(|e| *e.borrow_mut() = Some(cstring(msg.into())));
}

fn cstring(s: String) -> CString {
    CString::new(s.replace('\0', "")).unwrap_or_default()
}

fn out_str(s: String) -> *mut c_char {
    cstring(s).into_raw()
}

/// Runs `f`, turning an error or a panic into a message and `fallback`.
fn guard<T>(fallback: T, f: impl FnOnce() -> Result<T, String>) -> T {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(v)) => v,
        Ok(Err(m)) => {
            set_error(m);
            fallback
        }
        Err(p) => {
            let m = p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned());
            set_error(format!("internal error: {}", m.unwrap_or_default()));
            fallback
        }
    }
}

/// An index result: -1 on failure.
fn index(f: impl FnOnce() -> Result<usize, String>) -> isize {
    guard(-1, || f().map(|i| i as isize))
}

/// A status result: 1 done, 0 rejected (with a message), -1 on failure.
fn status(what: &str, f: impl FnOnce() -> Result<bool, String>) -> c_int {
    guard(-1, || match f()? {
        true => Ok(1),
        false => {
            set_error(format!("{what} rejected"));
            Ok(0)
        }
    })
}

/// An index result that also stores a second index in `aux` (when not NULL).
fn index2(aux: *mut usize, f: impl FnOnce() -> Result<(usize, usize), String>) -> isize {
    guard(-1, || {
        let (a, b) = f()?;
        if let Some(o) = unsafe { aux.as_mut() } {
            *o = b;
        }
        Ok(a as isize)
    })
}

/// A list result: the entities are stored in `out` (up to `cap`), the
/// number of entities is returned, -1 on failure.
fn list(out: *mut usize, cap: usize, f: impl FnOnce() -> Result<Vec<usize>, String>) -> isize {
    guard(-1, || {
        let v = f()?;
        if !out.is_null() {
            for (i, e) in v.iter().take(cap).enumerate() {
                unsafe { *out.add(i) = *e };
            }
        }
        Ok(v.len() as isize)
    })
}

unsafe fn in_str<'a>(s: *const c_char) -> Result<&'a str, String> {
    if s.is_null() {
        return Err("null string".into());
    }
    unsafe { CStr::from_ptr(s) }.to_str().map_err(|e| e.to_string())
}

/// NULL is None.
unsafe fn opt_str<'a>(s: *const c_char) -> Result<Option<&'a str>, String> {
    if s.is_null() { Ok(None) } else { unsafe { in_str(s) }.map(Some) }
}

unsafe fn sk_ref<'a>(d: *const GxDoc) -> Result<&'a Sketch, String> {
    unsafe { d.as_ref() }.map(|d| &d.sk).ok_or_else(|| "null document".into())
}

unsafe fn sk_mut<'a>(d: *mut GxDoc) -> Result<&'a mut Sketch, String> {
    unsafe { d.as_mut() }.map(|d| &mut d.sk).ok_or_else(|| "null document".into())
}

unsafe fn doc_ref<'a>(d: *const GxDoc) -> Result<&'a Doc, String> {
    unsafe { sk_ref(d) }.map(|s| &s.doc)
}

fn boxed(sk: Sketch) -> *mut GxDoc {
    Box::into_raw(Box::new(GxDoc { sk }))
}

/// Version of the library, as a static string.
#[unsafe(no_mangle)]
pub extern "C" fn gx_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}

/// Message of the last failed call on this thread, or NULL. Valid until the
/// next failed call on this thread.
#[unsafe(no_mangle)]
pub extern "C" fn gx_last_error() -> *const c_char {
    LAST_ERROR.with(|e| e.borrow().as_ref().map_or(ptr::null(), |s| s.as_ptr()))
}

// ------------------------------------------------------------ documents

/// New document holding only the origin point and the x and y axes.
#[unsafe(no_mangle)]
pub extern "C" fn gx_doc_new() -> *mut GxDoc {
    guard(ptr::null_mut(), || Ok(boxed(Sketch::new())))
}

/// Document decoded from .gxp text, or NULL. Call `gx_doc_solve` before
/// reading values from it.
///
/// # Safety
/// `text` is NULL or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_from_gxp(text: *const c_char) -> *mut GxDoc {
    guard(ptr::null_mut(), || {
        let doc = gxp::from_str(unsafe { in_str(text) }?).map_err(|e| e.to_string())?;
        Ok(boxed(Sketch::from_doc(doc)))
    })
}

/// Document encoded as indented .gxp text, or NULL.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_to_gxp(doc: *const GxDoc) -> *mut c_char {
    guard(ptr::null_mut(), || Ok(out_str(gxp::to_string_pretty(unsafe { doc_ref(doc) }?).map_err(|e| e.to_string())?)))
}

/// Releases a document. NULL is ignored.
///
/// # Safety
/// `doc` is NULL or a document from this library, not yet released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_free(doc: *mut GxDoc) {
    if !doc.is_null() {
        drop(unsafe { Box::from_raw(doc) });
    }
}

/// Solves the constraints and stores the solution in the document. Returns 1
/// when every constraint is satisfied, 0 when not, -1 on error. When `dof` is
/// not NULL, it receives the remaining degrees of freedom.
///
/// # Safety
/// `doc` is NULL or a document from this library; `dof` is NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_solve(doc: *mut GxDoc, dof: *mut usize) -> c_int {
    guard(-1, || {
        let sk = unsafe { sk_mut(doc) }?;
        let ok = sk.solve_full();
        if let Some(out) = unsafe { dof.as_mut() } {
            *out = sk.dof;
        }
        Ok(ok as c_int)
    })
}

/// Undoes the last change. Returns 1, or 0 when there is nothing to undo.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_undo(doc: *mut GxDoc) -> c_int {
    status("undo", || Ok(unsafe { sk_mut(doc) }?.undo()))
}

/// Redoes the last undone change. Returns 1, or 0 when there is nothing to redo.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_redo(doc: *mut GxDoc) -> c_int {
    status("redo", || Ok(unsafe { sk_mut(doc) }?.redo()))
}

// ------------------------------------------------------------ construction

/// The origin point.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_origin(doc: *const GxDoc) -> isize {
    index(|| Ok(unsafe { doc_ref(doc) }?.origin))
}

/// The x axis (an infinite line through the origin).
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_x_axis(doc: *const GxDoc) -> isize {
    index(|| Ok(unsafe { doc_ref(doc) }?.xaxis))
}

/// The y axis (an infinite line through the origin).
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_y_axis(doc: *const GxDoc) -> isize {
    index(|| Ok(unsafe { doc_ref(doc) }?.yaxis))
}

/// Adds a free point at (x, y), its starting position. Returns the entity.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_point(doc: *mut GxDoc, x: f64, y: f64) -> isize {
    index(|| ops::add_point(unsafe { sk_mut(doc) }?, x, y))
}

/// Adds a line from point a to point b: a segment, or an infinite line when
/// `infinite`. Returns the entity.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_line(doc: *mut GxDoc, a: usize, b: usize, infinite: bool) -> isize {
    index(|| ops::add_line(unsafe { sk_mut(doc) }?, a, b, infinite))
}

/// Adds a circle around point c with starting radius r (the radius stays
/// free until constrained). Returns the entity.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_circle(doc: *mut GxDoc, c: usize, r: f64) -> isize {
    index(|| ops::add_circle(unsafe { sk_mut(doc) }?, c, r))
}

/// Adds an arc around point c from point a counterclockwise to point b
/// (|ca| = |cb| is enforced). Returns the entity.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_arc(doc: *mut GxDoc, c: usize, a: usize, b: usize) -> isize {
    index(|| ops::add_arc(unsafe { sk_mut(doc) }?, c, a, b))
}

/// Adds a curve given as text: `y = f(x)` or `F(x, y) = G(x, y)` in x, y,
/// variables and dimensions. Returns the entity.
///
/// # Safety
/// `doc` is NULL or a document from this library; `src` is NULL or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_curve(doc: *mut GxDoc, src: *const c_char) -> isize {
    index(|| ops::add_curve(unsafe { sk_mut(doc) }?, unsafe { in_str(src) }?))
}

/// Sets the text of curve `i`. Returns 1, 0 when rejected, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library; `src` is NULL or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_set_curve(doc: *mut GxDoc, i: usize, src: *const c_char) -> c_int {
    status("curve", || ops::set_curve(unsafe { sk_mut(doc) }?, i, unsafe { in_str(src) }?))
}

/// Adds a conic of kind `kind` (`GX_CONIC_*`) centered on point c (the vertex
/// of a parabola), with half-axes a and b (b is ignored for a parabola,
/// where a is the focal distance) and axis angle t in radians. Returns the entity.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_conic(doc: *mut GxDoc, kind: GxConic, c: usize, a: f64, b: f64, t: f64) -> isize {
    index(|| ops::add_conic(unsafe { sk_mut(doc) }?, ops::conic_kind(kind)?, c, a, b, t))
}

/// Adds a point at (x, y) on curve c, bound to it by a constraint. Returns the entity.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_point_on(doc: *mut GxDoc, c: usize, x: f64, y: f64) -> isize {
    index(|| ops::add_point_on(unsafe { sk_mut(doc) }?, c, x, y))
}

/// Adds the circle around point c through point p, bound to it. Returns the entity.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_circle_to(doc: *mut GxDoc, c: usize, p: usize) -> isize {
    index(|| ops::circle_to(unsafe { sk_mut(doc) }?, c, p))
}

/// Adds the arc around point c from point a, ending at a new point in the
/// direction of (x, y). Returns the entity.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_arc_toward(doc: *mut GxDoc, c: usize, a: usize, x: f64, y: f64) -> isize {
    index(|| ops::arc_toward(unsafe { sk_mut(doc) }?, c, a, [x, y]))
}

/// Adds the midpoint of points a and b, bound to them. Returns the point.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_midpoint(doc: *mut GxDoc, a: usize, b: usize) -> isize {
    index(|| ops::midpoint(unsafe { sk_mut(doc) }?, a, b))
}

/// Adds the midpoint of line l, bound to it. Returns the point.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_line_midpoint(doc: *mut GxDoc, l: usize) -> isize {
    index(|| ops::line_midpoint(unsafe { sk_mut(doc) }?, l))
}

/// Adds a point at a crossing of curves a and b, starting from (x, y).
/// Returns the point.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_intersection(doc: *mut GxDoc, a: usize, b: usize, x: f64, y: f64) -> isize {
    index(|| ops::intersection(unsafe { sk_mut(doc) }?, a, b, [x, y]))
}

/// Adds a point at the crossing of curves a and b nearest to (x, y).
/// Returns the point.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_intersection_near(doc: *mut GxDoc, a: usize, b: usize, x: f64, y: f64) -> isize {
    index(|| ops::intersect_near(unsafe { sk_mut(doc) }?, a, b, [x, y]))
}

/// Adds the infinite line through point p perpendicular to line l. Returns the line.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_perp_line(doc: *mut GxDoc, l: usize, p: usize) -> isize {
    index(|| ops::dir_line(unsafe { sk_mut(doc) }?, l, p, true))
}

/// Adds the infinite line through point p parallel to line l. Returns the line.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_parallel_line(doc: *mut GxDoc, l: usize, p: usize) -> isize {
    index(|| ops::dir_line(unsafe { sk_mut(doc) }?, l, p, false))
}

/// Adds the perpendicular bisector of points a and b. Returns the line;
/// the midpoint is stored in `mid` when it is not NULL.
///
/// # Safety
/// `doc` is NULL or a document from this library; `mid` is NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_perp_bisector(doc: *mut GxDoc, a: usize, b: usize, mid: *mut usize) -> isize {
    index2(mid, || ops::perp_bisector(unsafe { sk_mut(doc) }?, a, b))
}

/// Adds the bisector of lines l1 and l2 in the sector containing (x, y).
/// Returns the line; the vertex (a shared endpoint, or a new point at the
/// crossing) is stored in `vertex` when it is not NULL.
///
/// # Safety
/// `doc` is NULL or a document from this library; `vertex` is NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_angle_bisector(doc: *mut GxDoc, l1: usize, l2: usize, x: f64, y: f64, vertex: *mut usize) -> isize {
    index2(vertex, || ops::angle_bisector(unsafe { sk_mut(doc) }?, l1, l2, [x, y]))
}

/// Adds the segment from point p touching curve c at a contact point
/// starting from (x, y). Returns the segment; the contact point is stored
/// in `contact` when it is not NULL.
///
/// # Safety
/// `doc` is NULL or a document from this library; `contact` is NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_tangent_line(doc: *mut GxDoc, p: usize, c: usize, x: f64, y: f64, contact: *mut usize) -> isize {
    index2(contact, || ops::tangent_line(unsafe { sk_mut(doc) }?, p, c, [x, y]))
}

/// Like `gx_doc_add_tangent_line`, with the contact point nearest to (x, y).
///
/// # Safety
/// `doc` is NULL or a document from this library; `contact` is NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_tangent_line_near(doc: *mut GxDoc, p: usize, c: usize, x: f64, y: f64, contact: *mut usize) -> isize {
    index2(contact, || ops::tangent_line_near(unsafe { sk_mut(doc) }?, p, c, [x, y]))
}

/// Adds the mirror image of point p in line l. Returns the point.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_reflection(doc: *mut GxDoc, p: usize, l: usize) -> isize {
    index(|| ops::reflection(unsafe { sk_mut(doc) }?, p, l))
}

/// Adds the circle through points a, b and c. Returns the circle; its new
/// center point is stored in `center` when it is not NULL.
///
/// # Safety
/// `doc` is NULL or a document from this library; `center` is NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_circle_through(doc: *mut GxDoc, a: usize, b: usize, c: usize, center: *mut usize) -> isize {
    index2(center, || ops::circle_through(unsafe { sk_mut(doc) }?, a, b, c))
}

/// Adds the arc from point a through point m to point b. Returns the arc;
/// its new center point is stored in `center` when it is not NULL.
///
/// # Safety
/// `doc` is NULL or a document from this library; `center` is NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_arc_through(doc: *mut GxDoc, a: usize, m: usize, b: usize, center: *mut usize) -> isize {
    index2(center, || ops::arc_through(unsafe { sk_mut(doc) }?, a, m, b))
}

/// Adds the ellipse or hyperbola (`kind`: `GX_CONIC_ELLIPSE` or
/// `GX_CONIC_HYPERBOLA`) with foci f1 and f2 through point p. Returns the entity.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_conic_by_foci(doc: *mut GxDoc, kind: GxConic, f1: usize, f2: usize, p: usize) -> isize {
    index(|| ops::conic_by_foci(unsafe { sk_mut(doc) }?, ops::conic_kind(kind)?, f1, f2, p))
}

/// Adds the parabola with focus f and directrix line l. Returns the entity.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_parabola_by_focus(doc: *mut GxDoc, f: usize, l: usize) -> isize {
    index(|| ops::parabola_by_focus(unsafe { sk_mut(doc) }?, f, l))
}

/// Adds the ellipse or hyperbola centered on point c, its first axis ending
/// at (ax, ay) and its second half-axis reaching the line through (sx, sy)
/// parallel to the first axis. Returns the entity.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_conic_from(doc: *mut GxDoc, kind: GxConic, c: usize, ax: f64, ay: f64, sx: f64, sy: f64) -> isize {
    index(|| ops::conic_from(unsafe { sk_mut(doc) }?, ops::conic_kind(kind)?, c, [ax, ay], [sx, sy]))
}

/// Adds the parabola with vertex at point c opening toward (x, y), its focal
/// distance the distance to (x, y). Returns the entity.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_parabola_toward(doc: *mut GxDoc, c: usize, x: f64, y: f64) -> isize {
    index(|| ops::parabola_toward(unsafe { sk_mut(doc) }?, c, [x, y]))
}

/// Adds the foci of conic c (two points, one for a parabola), storing up to
/// `cap` of them in `out`. Returns their number, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library; `out` is NULL or writable for `cap` entries.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_foci(doc: *mut GxDoc, c: usize, out: *mut usize, cap: usize) -> isize {
    list(out, cap, || ops::conic_feature(unsafe { sk_mut(doc) }?, ConicFeature::Foci, c, [0.0; 2]))
}

/// Adds the directrix of conic c belonging to the focus nearest (x, y).
/// Returns the line.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_directrix(doc: *mut GxDoc, c: usize, x: f64, y: f64) -> isize {
    index(|| ops::conic_feature(unsafe { sk_mut(doc) }?, ConicFeature::Directrix, c, [x, y])?.first().copied().ok_or_else(|| "no directrix".into()))
}

/// Adds both asymptotes of hyperbola c, storing up to `cap` of them in
/// `out`. Returns their number, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library; `out` is NULL or writable for `cap` entries.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_asymptotes(doc: *mut GxDoc, c: usize, out: *mut usize, cap: usize) -> isize {
    list(out, cap, || ops::conic_feature(unsafe { sk_mut(doc) }?, ConicFeature::Asymptotes, c, [0.0; 2]))
}

/// Adds a constraint of kind `kind` (`GX_CON_*`) on entities a and b.
/// Returns 1, 0 when it is rejected (unsatisfiable or redundant), -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_constrain(doc: *mut GxDoc, kind: GxCon, a: usize, b: usize) -> c_int {
    status("constraint", || ops::constrain(unsafe { sk_mut(doc) }?, ConKind::from_index(kind)?, a, b))
}

/// Adds a dimension of kind `kind` (`GX_DIM_*`) on entities a and b, with
/// its label at (off_x, off_y) from the dimension's anchor (the midpoint of
/// a length, the vertex of an angle). The dimension is driving when that
/// keeps the constraints solvable, otherwise a reference (measured)
/// dimension. For an angle, the sector holding the label selects the angle
/// or its supplement. Returns the dimension.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_dim(doc: *mut GxDoc, kind: GxDim, a: usize, b: usize, off_x: f64, off_y: f64) -> isize {
    index(|| ops::add_dim(unsafe { sk_mut(doc) }?, DimType::from_index(kind)?, a, b, [off_x, off_y]))
}

/// Sets dimension `i` from text as typed in gxpad: a number (degrees for
/// angles) or an expression of variables and dimensions drives it; empty
/// text makes it a reference dimension. Returns 1, 0 when the value cannot
/// be reached, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library; `text` is NULL or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_set_dim(doc: *mut GxDoc, i: usize, text: *const c_char) -> c_int {
    status("dimension value", || ops::set_dim(unsafe { sk_mut(doc) }?, i, unsafe { in_str(text) }?))
}

/// Makes dimension `i` driving or reference. Returns 1, 0 when making it
/// driving is rejected (the constraints already determine it), -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_set_dim_driving(doc: *mut GxDoc, i: usize, driving: bool) -> c_int {
    status("driving dimension", || ops::set_driving(unsafe { sk_mut(doc) }?, i, driving))
}

/// Marks dimension `i` numeric or not. Formulas derived for other targets
/// use a numeric dimension's current value as a plain number. Solving and
/// the dimension itself are unaffected. Returns 1, 0 when rejected, -1 on
/// error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_set_dim_numeric(doc: *mut GxDoc, i: usize, numeric: bool) -> c_int {
    status("numeric dimension", || ops::set_dim_numeric(unsafe { sk_mut(doc) }?, i, numeric))
}

/// Adds a value variable named `name` (NULL picks the first unused of a, b,
/// c, k, ...). A free variable is set by `value`, otherwise it is solved
/// from the constraints, starting at `value`. Returns the variable.
///
/// # Safety
/// `doc` is NULL or a document from this library; `name` is NULL or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_var(doc: *mut GxDoc, name: *const c_char, value: f64, free: bool) -> isize {
    index(|| ops::add_var(unsafe { sk_mut(doc) }?, unsafe { opt_str(name) }?, value, free))
}

/// Adds a variable named `name` (NULL picks f1, f2, ...) defined by the
/// expression `expr` of variables and dimensions. Returns the variable.
///
/// # Safety
/// `doc` is NULL or a document from this library; `name` is NULL or a NUL-terminated string; `expr` is NULL or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_add_defvar(doc: *mut GxDoc, name: *const c_char, expr: *const c_char) -> isize {
    index(|| ops::add_defvar(unsafe { sk_mut(doc) }?, unsafe { opt_str(name) }?, unsafe { in_str(expr) }?))
}

/// Sets the value of value variable `i`. Returns 1, 0 when the value cannot
/// be reached, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_set_var(doc: *mut GxDoc, i: usize, value: f64) -> c_int {
    status("variable value", || ops::set_var(unsafe { sk_mut(doc) }?, i, value))
}

/// Sets variable `i` from text as typed in gxpad: a plain number makes it a
/// free value variable, any other text an expression of variables and
/// dimensions. Returns 1, 0 when rejected, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library; `text` is NULL or a NUL-terminated string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_set_var_expr(doc: *mut GxDoc, i: usize, text: *const c_char) -> c_int {
    status("variable expression", || ops::set_var_expr(unsafe { sk_mut(doc) }?, i, unsafe { in_str(text) }?))
}

/// Makes value variable `i` free (set by its value) or solved from the
/// constraints. Returns 1, 0 when rejected, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_set_var_free(doc: *mut GxDoc, i: usize, free: bool) -> c_int {
    status("free variable", || ops::set_var_free(unsafe { sk_mut(doc) }?, i, free))
}

/// Marks variable `i` numeric or not. Formulas derived for other targets
/// use a numeric variable's current value as a plain number. Solving and
/// the variable itself (value, expression, free or solved) are unaffected.
/// Returns 1, 0 when rejected, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_set_var_numeric(doc: *mut GxDoc, i: usize, numeric: bool) -> c_int {
    status("numeric variable", || ops::set_var_numeric(unsafe { sk_mut(doc) }?, i, numeric))
}

/// Removes variable `i`, leaving an empty slot; expressions naming it fail
/// to parse until edited. Returns 1, 0 when rejected, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_delete_var(doc: *mut GxDoc, i: usize) -> c_int {
    status("variable deletion", || ops::delete_var(unsafe { sk_mut(doc) }?, i))
}

/// Deletes entity `e` and what depends on it. Returns 1, 0 when the
/// constraints would no longer solve (the document is then unchanged), -1
/// on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_delete_ent(doc: *mut GxDoc, e: usize) -> c_int {
    status("entity deletion", || ops::delete_ent(unsafe { sk_mut(doc) }?, e))
}

/// Deletes constraint `c`. Returns 1, 0 when the constraints would no
/// longer solve (the document is then unchanged), -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_delete_con(doc: *mut GxDoc, c: usize) -> c_int {
    status("constraint deletion", || ops::delete_con(unsafe { sk_mut(doc) }?, c))
}

/// Deletes dimension `i`; expressions naming it fail to parse until edited.
/// Returns 1, 0 when the constraints would no longer solve (the document is
/// then unchanged), -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_delete_dim(doc: *mut GxDoc, i: usize) -> c_int {
    status("dimension deletion", || ops::delete_dim(unsafe { sk_mut(doc) }?, i))
}

/// Locks or unlocks point `e` (fixing or freeing its coordinates). Returns 1,
/// 0 when rejected, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_lock_point(doc: *mut GxDoc, e: usize, on: bool) -> c_int {
    status("point lock", || ops::lock_point(unsafe { sk_mut(doc) }?, e, on))
}

/// Sets the construction flag of entity `e` (a dashed helper). Returns 1, 0
/// when rejected, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_set_construction(doc: *mut GxDoc, e: usize, on: bool) -> c_int {
    status("construction flag", || ops::set_construction(unsafe { sk_mut(doc) }?, e, on))
}

/// Sets the hidden flag of entity `e`. Returns 1, 0 when rejected, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_set_hidden(doc: *mut GxDoc, e: usize, on: bool) -> c_int {
    status("hidden flag", || ops::set_hidden(unsafe { sk_mut(doc) }?, e, on))
}

/// Makes line `e` infinite or a segment. Returns 1, 0 when rejected, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_set_infinite(doc: *mut GxDoc, e: usize, on: bool) -> c_int {
    status("infinite line", || ops::set_infinite(unsafe { sk_mut(doc) }?, e, on))
}

/// Pulls point `e` toward (x, y) by solving the constraints with it pinned
/// there (a drag, not an undoable change). Returns 1 when reached, 0 when
/// not, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_drag_point(doc: *mut GxDoc, e: usize, x: f64, y: f64) -> c_int {
    status("drag", || ops::drag_point(unsafe { sk_mut(doc) }?, e, x, y))
}

/// Pulls the radius of circle `e` so its edge passes through (x, y). Returns
/// 1 when reached, 0 when not, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_drag_circle(doc: *mut GxDoc, e: usize, x: f64, y: f64) -> c_int {
    status("drag circle", || ops::drag_circle_edge(unsafe { sk_mut(doc) }?, e, x, y))
}

/// Pulls a handle of conic `e` (`which` 0 the first-axis end, 1 the
/// second-axis end) toward (x, y). Returns 1 when reached, 0 when not, -1 on
/// error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_drag_conic_handle(doc: *mut GxDoc, e: usize, which: u8, x: f64, y: f64) -> c_int {
    status("drag conic", || ops::drag_conic_handle(unsafe { sk_mut(doc) }?, e, which, x, y))
}

/// Gives the point, dimension or variable named `old` the name `new`,
/// rewriting expressions that use it. Returns 1, 0 when `new` is not an
/// identifier or is taken, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library; `old` and `new` are NULL or NUL-terminated strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_rename(doc: *mut GxDoc, old: *const c_char, new: *const c_char) -> c_int {
    status("rename", || ops::rename(unsafe { sk_mut(doc) }?, unsafe { in_str(old) }?, unsafe { in_str(new) }?))
}

// ------------------------------------------------------------ reading

/// Number of entity slots. Deleted entities leave empty slots.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_ent_count(doc: *const GxDoc) -> usize {
    guard(0, || Ok(ops::ent_count(unsafe { sk_ref(doc) }?)))
}

/// Kind of entity `e` (`GX_ENT_*`), or -1 for an empty slot or a bad index.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_ent_kind(doc: *const GxDoc, e: usize) -> c_int {
    guard(-1, || Ok(match ops::ent_kind(unsafe { sk_ref(doc) }?, e)? {
        EntKind::Point => GX_ENT_POINT,
        EntKind::Line => GX_ENT_LINE,
        EntKind::Circle => GX_ENT_CIRCLE,
        EntKind::Arc => GX_ENT_ARC,
        EntKind::Conic => GX_ENT_CONIC,
        EntKind::Curve => GX_ENT_CURVE,
    } as c_int))
}

/// Name of entity `e`, or NULL for an empty name, an empty slot or a bad
/// index. Free the result with `gx_string_free`.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_ent_name(doc: *const GxDoc, e: usize) -> *mut c_char {
    guard(ptr::null_mut(), || Ok(out_str(ops::ent_name(unsafe { sk_ref(doc) }?, e)?)))
}

/// World coordinates of point `e`, stored in `x` and `y` when they are not
/// NULL. Returns 1, or -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library; `x` and `y` are NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_point_pos(doc: *const GxDoc, e: usize, x: *mut f64, y: *mut f64) -> c_int {
    guard(-1, || {
        let p = ops::point_pos(unsafe { sk_ref(doc) }?, e)?;
        if let Some(o) = unsafe { x.as_mut() } {
            *o = p[0];
        }
        if let Some(o) = unsafe { y.as_mut() } {
            *o = p[1];
        }
        Ok(1)
    })
}

/// 1 when point `e` is locked (both coordinates fixed), 0 when it is free, -1
/// on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_point_locked(doc: *const GxDoc, e: usize) -> c_int {
    guard(-1, || Ok(ops::point_locked(unsafe { sk_ref(doc) }?, e)? as c_int))
}

/// Endpoints a and b of line `e`, stored in `a`, `b` and `infinite` when they
/// are not NULL. Returns 1, or -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library; `a`, `b` and `infinite` are NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_line_pts(doc: *const GxDoc, e: usize, a: *mut usize, b: *mut usize, infinite: *mut bool) -> c_int {
    guard(-1, || {
        let (pa, pb, inf) = ops::line_pts(unsafe { sk_ref(doc) }?, e)?;
        if let Some(o) = unsafe { a.as_mut() } {
            *o = pa;
        }
        if let Some(o) = unsafe { b.as_mut() } {
            *o = pb;
        }
        if let Some(o) = unsafe { infinite.as_mut() } {
            *o = inf;
        }
        Ok(1)
    })
}

/// 1 when line `e` is infinite, 0 when it is a segment, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_ent_infinite(doc: *const GxDoc, e: usize) -> c_int {
    guard(-1, || Ok(ops::ent_infinite(unsafe { sk_ref(doc) }?, e)? as c_int))
}

/// Center point and radius of circle or arc `e`, stored in `c` and `r` when
/// they are not NULL. Returns 1, or -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library; `c` and `r` are NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_circle_vals(doc: *const GxDoc, e: usize, c: *mut usize, r: *mut f64) -> c_int {
    guard(-1, || {
        let (pc, pr) = ops::circle_vals(unsafe { sk_ref(doc) }?, e)?;
        if let Some(o) = unsafe { c.as_mut() } {
            *o = pc;
        }
        if let Some(o) = unsafe { r.as_mut() } {
            *o = pr;
        }
        Ok(1)
    })
}

/// Center and endpoints of arc `e`, stored in `c`, `a` and `b` when they are
/// not NULL. Returns 1, or -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library; `c`, `a` and `b` are NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_arc_pts(doc: *const GxDoc, e: usize, c: *mut usize, a: *mut usize, b: *mut usize) -> c_int {
    guard(-1, || {
        let (pc, pa, pb) = ops::arc_pts(unsafe { sk_ref(doc) }?, e)?;
        if let Some(o) = unsafe { c.as_mut() } {
            *o = pc;
        }
        if let Some(o) = unsafe { a.as_mut() } {
            *o = pa;
        }
        if let Some(o) = unsafe { b.as_mut() } {
            *o = pb;
        }
        Ok(1)
    })
}

/// Kind (`GX_CONIC_*`), center point and axis values (a, b, angle in
/// radians) of conic `e`, stored in the non-NULL outputs. Returns 1, or -1
/// on error.
///
/// # Safety
/// `doc` is NULL or a document from this library; the outputs are NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_conic_vals(doc: *const GxDoc, e: usize, kind: *mut GxConic, c: *mut usize, a: *mut f64, b: *mut f64, t: *mut f64) -> c_int {
    guard(-1, || {
        let (k, pc, pa, pb, pt) = ops::conic_vals(unsafe { sk_ref(doc) }?, e)?;
        if let Some(o) = unsafe { kind.as_mut() } {
            *o = match k {
                crate::model::ConicKind::Ellipse => GX_CONIC_ELLIPSE,
                crate::model::ConicKind::Hyperbola => GX_CONIC_HYPERBOLA,
                crate::model::ConicKind::Parabola => GX_CONIC_PARABOLA,
            };
        }
        if let Some(o) = unsafe { c.as_mut() } {
            *o = pc;
        }
        if let Some(o) = unsafe { a.as_mut() } {
            *o = pa;
        }
        if let Some(o) = unsafe { b.as_mut() } {
            *o = pb;
        }
        if let Some(o) = unsafe { t.as_mut() } {
            *o = pt;
        }
        Ok(1)
    })
}

/// Source text of function curve `e`, or NULL for an empty slot or a bad
/// index. Free the result with `gx_string_free`.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_curve_src(doc: *const GxDoc, e: usize) -> *mut c_char {
    guard(ptr::null_mut(), || Ok(out_str(ops::curve_src(unsafe { sk_ref(doc) }?, e)?)))
}

/// 1 when entity `e` is a construction (dashed helper), 0 when it is a
/// normal entity, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_ent_construction(doc: *const GxDoc, e: usize) -> c_int {
    guard(-1, || Ok(ops::ent_construction(unsafe { sk_ref(doc) }?, e)? as c_int))
}

/// 1 when entity `e` is hidden, 0 when it is drawn, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_ent_hidden(doc: *const GxDoc, e: usize) -> c_int {
    guard(-1, || Ok(ops::ent_hidden(unsafe { sk_ref(doc) }?, e)? as c_int))
}

/// Number of constraint slots. Deleted constraints leave empty slots.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_con_count(doc: *const GxDoc) -> usize {
    guard(0, || Ok(ops::con_count(unsafe { sk_ref(doc) }?)))
}

/// Kind of constraint `c` (`GX_CON_*`), or -1 for an empty slot or a bad
/// index.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_con_kind(doc: *const GxDoc, c: usize) -> c_int {
    guard(-1, || Ok(ops::con_kind(unsafe { sk_ref(doc) }?, c)? as c_int))
}

/// The entities of constraint `c`, stored in `out` (up to `cap`); returns
/// their number, -1 for an empty slot or a bad index.
///
/// # Safety
/// `doc` is NULL or a document from this library; `out` is NULL or writable for `cap` entries.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_con_ents(doc: *const GxDoc, c: usize, out: *mut usize, cap: usize) -> isize {
    list(out, cap, || ops::con_ents(unsafe { sk_ref(doc) }?, c))
}

/// Kind of dimension `i` (`GX_DIM_*`), or -1 for an empty slot or a bad index.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_dim_kind(doc: *const GxDoc, i: usize) -> c_int {
    guard(-1, || Ok(ops::dim_kind(unsafe { sk_ref(doc) }?, i)? as c_int))
}

/// World-coordinate polylines drawing entity `e` inside `window`: one
/// point for a point, the endpoints of a segment, a segment crossing the
/// window for an infinite line, the outline of a circle, arc or conic (two
/// polylines for a hyperbola), and the parts of a function curve inside the
/// window, sampled at the window's pixel size.
///
/// The points go to `out` as flat pairs (x0, y0, x1, y1, ...), up to `cap`
/// points; the index of the first point of each polyline goes to `starts`,
/// up to `cap_starts` polylines; the number of polylines goes to `n_polys`.
/// Any of the three may be NULL. Returns the total number of points, which
/// is more than `cap` when `out` was too small, or -1 on error. Calling
/// first with NULL buffers gives the sizes to allocate.
///
/// # Safety
/// `doc` is NULL or a document from this library; `out` is NULL or writable
/// for `2 * cap` values; `starts` is NULL or writable for `cap_starts`
/// entries; `n_polys` is NULL or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_ent_samples(
    doc: *const GxDoc,
    e: usize,
    window: GxWindow,
    out: *mut f64,
    cap: usize,
    starts: *mut usize,
    cap_starts: usize,
    n_polys: *mut usize,
) -> isize {
    guard(-1, || {
        let polys = ops::ent_samples(unsafe { sk_ref(doc) }?, e, [window.x0, window.y0], [window.x1, window.y1], window.px)?;
        let total: usize = polys.iter().map(|pl| pl.len()).sum();
        if !out.is_null() {
            for (n, p) in polys.iter().flatten().take(cap).enumerate() {
                unsafe {
                    *out.add(n * 2) = p[0];
                    *out.add(n * 2 + 1) = p[1];
                }
            }
        }
        if !starts.is_null() {
            let mut acc = 0;
            for (k, pl) in polys.iter().enumerate().take(cap_starts) {
                unsafe { *starts.add(k) = acc };
                acc += pl.len();
            }
        }
        if let Some(o) = unsafe { n_polys.as_mut() } {
            *o = polys.len();
        }
        Ok(total as isize)
    })
}

// ------------------------------------------------------------ formulas

/// Number of dimension slots. Removed dimensions leave empty slots.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_dim_count(doc: *const GxDoc) -> usize {
    guard(0, || Ok(unsafe { doc_ref(doc) }?.dims.len()))
}

/// Name of dimension `i`, or NULL for an empty slot or a bad index.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_dim_name(doc: *const GxDoc, i: usize) -> *mut c_char {
    guard(ptr::null_mut(), || {
        let d = unsafe { doc_ref(doc) }?;
        let dim = d.dims.get(i).and_then(|d| d.as_ref()).ok_or("no such dimension")?;
        Ok(out_str(dim.name.clone()))
    })
}

/// Current measured value of dimension `i` (radians for angles), or NaN for
/// an empty slot or a bad index.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_dim_value(doc: *const GxDoc, i: usize) -> f64 {
    guard(f64::NAN, || {
        let d = unsafe { doc_ref(doc) }?;
        d.dims.get(i).and_then(|d| d.as_ref()).ok_or("no such dimension")?;
        Ok(d.measure_value(i))
    })
}

/// 1 when dimension `i` is driving, 0 when it is a reference dimension, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_dim_driving(doc: *const GxDoc, i: usize) -> c_int {
    guard(-1, || Ok(ops::dim_driving(unsafe { sk_ref(doc) }?, i)? as c_int))
}

/// 1 when dimension `i` is numeric (substituted as a number in formulas), 0 when
/// it is symbolic, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_dim_numeric(doc: *const GxDoc, i: usize) -> c_int {
    guard(-1, || Ok(ops::dim_numeric(unsafe { sk_ref(doc) }?, i)? as c_int))
}

/// Number of variable slots. Removed variables leave empty slots.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_var_count(doc: *const GxDoc) -> usize {
    guard(0, || Ok(unsafe { doc_ref(doc) }?.vars.len()))
}

/// Name of variable `i`, or NULL for an empty slot or a bad index.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_var_name(doc: *const GxDoc, i: usize) -> *mut c_char {
    guard(ptr::null_mut(), || {
        let d = unsafe { doc_ref(doc) }?;
        let v = d.vars.get(i).and_then(|v| v.as_ref()).ok_or("no such variable")?;
        Ok(out_str(v.name.clone()))
    })
}

/// Value of variable `i`, or NaN for an empty slot or a bad index.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_var_value(doc: *const GxDoc, i: usize) -> f64 {
    guard(f64::NAN, || ops::var_value(unsafe { sk_ref(doc) }?, i))
}

/// Expression of variable `i` (empty for a value variable), or NULL for an
/// empty slot or a bad index. Free the result with `gx_string_free`.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_var_expr(doc: *const GxDoc, i: usize) -> *mut c_char {
    guard(ptr::null_mut(), || Ok(out_str(ops::var_expr(unsafe { sk_ref(doc) }?, i)?)))
}

/// 1 when variable `i` is free, 0 when it is solved or defined by an
/// expression, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_var_free(doc: *const GxDoc, i: usize) -> c_int {
    guard(-1, || Ok(ops::var_free(unsafe { sk_ref(doc) }?, i)? as c_int))
}

/// 1 when variable `i` is numeric (substituted as a number in formulas), 0 when
/// it is symbolic, -1 on error.
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_doc_var_numeric(doc: *const GxDoc, i: usize) -> c_int {
    guard(-1, || Ok(ops::var_numeric(unsafe { sk_ref(doc) }?, i)? as c_int))
}

// ------------------------------------------------------------ formulas

/// Derives the closed-form formula of a dimension or variable, or NULL.
/// `extra_ms` extends the search beyond the default budget (0 for the
/// default; gxpad's "search harder" adds 10000 per press).
///
/// # Safety
/// `doc` is NULL or a document from this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_derive(doc: *const GxDoc, kind: GxTarget, index: usize, extra_ms: u64) -> *mut GxFormula {
    guard(ptr::null_mut(), || {
        let d = unsafe { doc_ref(doc) }?;
        let t = match kind {
            GX_TARGET_DIM => Target::Dim(index),
            GX_TARGET_VAR => Target::Var(index),
            _ => return Err(format!("bad target kind {kind}")),
        };
        let f = formula::derive_formula(d, t, extra_ms)?;
        Ok(Box::into_raw(Box::new(GxFormula {
            name: out_str(f.name),
            plain: out_str(f.plain),
            latex: out_str(f.latex),
            exact: f.exact,
            is_angle: f.is_angle,
            value: f.value,
            actual: f.actual,
        })))
    })
}

/// Releases a formula. NULL is ignored.
///
/// # Safety
/// `f` is NULL or a formula from `gx_derive`, not yet released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_formula_free(f: *mut GxFormula) {
    if f.is_null() {
        return;
    }
    let f = unsafe { Box::from_raw(f) };
    for s in [f.name, f.plain, f.latex] {
        unsafe { gx_string_free(s) };
    }
}

/// Releases a string returned by this library. NULL is ignored.
///
/// # Safety
/// `s` is NULL or a string from this library, not yet released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gx_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(unsafe { CString::from_raw(s) });
    }
}
