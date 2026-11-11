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
use crate::ops::{self, ConKind, DimType};
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

unsafe fn in_str<'a>(s: *const c_char) -> Result<&'a str, String> {
    if s.is_null() {
        return Err("null string".into());
    }
    unsafe { CStr::from_ptr(s) }.to_str().map_err(|e| e.to_string())
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
