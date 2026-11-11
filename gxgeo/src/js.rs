//! JavaScript interface for `wasm32-unknown-unknown`, through wasm-bindgen.
//! Build the JS bindings with
//! `wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/gxgeo.wasm`
//! (or `--target nodejs`, `--target bundler`).
//!
//! Entities (points, lines, circles, ...), dimensions and variables are
//! numbered by index; deleting leaves empty slots, so indices stay valid.
//! Methods that change the document throw when the arguments are wrong, and
//! return false when the change is rejected (the constraints would not
//! solve, or it is redundant).

use crate::formula::{self, Target};
use crate::gxp;
use crate::ops::{self, ConKind, ConicFeature, DimType};
use crate::sketch::Sketch;
use wasm_bindgen::prelude::*;

/// A geometry document with its solver state and undo history.
#[wasm_bindgen(js_name = Doc)]
pub struct JsDoc {
    sk: Sketch,
}

/// A derived formula.
#[wasm_bindgen(getter_with_clone)]
pub struct Formula {
    /// Name of the target.
    pub name: String,
    /// `name = formula` as plain text.
    pub plain: String,
    /// `name = formula` as LaTeX.
    pub latex: String,
    /// False when some part had to be solved numerically.
    pub exact: bool,
    /// True when the target is an angle (in radians).
    #[wasm_bindgen(js_name = isAngle)]
    pub is_angle: bool,
    /// Formula evaluated at the current configuration.
    pub value: f64,
    /// Measured value of the target.
    pub actual: f64,
}

/// Result of `Doc.solve`.
#[wasm_bindgen(getter_with_clone)]
pub struct Solved {
    /// True when every constraint is satisfied.
    pub ok: bool,
    /// Remaining degrees of freedom.
    pub dof: usize,
    /// Messages for constraints or dimensions that could not be turned into equations.
    pub errors: Vec<String>,
}

fn js(e: String) -> JsError {
    JsError::new(&e)
}

#[wasm_bindgen(js_class = Doc)]
impl JsDoc {
    /// New document holding only the origin point and the x and y axes.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> JsDoc {
        JsDoc { sk: Sketch::new() }
    }

    /// Document decoded from .gxp text. Call `solve` before reading values from it.
    #[wasm_bindgen(js_name = fromGxp)]
    pub fn from_gxp(text: &str) -> Result<JsDoc, JsError> {
        Ok(JsDoc { sk: Sketch::from_doc(gxp::from_str(text)?) })
    }

    /// Document encoded as indented .gxp text.
    #[wasm_bindgen(js_name = toGxp)]
    pub fn to_gxp(&self) -> Result<String, JsError> {
        Ok(gxp::to_string_pretty(&self.sk.doc)?)
    }

    /// Solves the constraints and stores the solution in the document.
    pub fn solve(&mut self) -> Solved {
        let ok = self.sk.solve_full();
        Solved { ok, dof: self.sk.dof, errors: self.sk.errors.values().cloned().collect() }
    }

    /// Undoes the last change; false when there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        self.sk.undo()
    }

    /// Redoes the last undone change; false when there is nothing to redo.
    pub fn redo(&mut self) -> bool {
        self.sk.redo()
    }

    // ------------------------------------------------------------ construction

    /// The origin point.
    #[wasm_bindgen(getter)]
    pub fn origin(&self) -> usize {
        self.sk.doc.origin
    }

    /// The x axis (an infinite line through the origin).
    #[wasm_bindgen(getter, js_name = xAxis)]
    pub fn x_axis(&self) -> usize {
        self.sk.doc.xaxis
    }

    /// The y axis (an infinite line through the origin).
    #[wasm_bindgen(getter, js_name = yAxis)]
    pub fn y_axis(&self) -> usize {
        self.sk.doc.yaxis
    }

    /// Adds a free point at (x, y), its starting position. Returns the entity.
    #[wasm_bindgen(js_name = addPoint)]
    pub fn add_point(&mut self, x: f64, y: f64) -> Result<usize, JsError> {
        ops::add_point(&mut self.sk, x, y).map_err(js)
    }

    /// Adds a line from point a to point b: a segment, or an infinite line
    /// when `infinite`. Returns the entity.
    #[wasm_bindgen(js_name = addLine)]
    pub fn add_line(&mut self, a: usize, b: usize, infinite: Option<bool>) -> Result<usize, JsError> {
        ops::add_line(&mut self.sk, a, b, infinite.unwrap_or(false)).map_err(js)
    }

    /// Adds a circle around point c with starting radius r (the radius stays
    /// free until constrained). Returns the entity.
    #[wasm_bindgen(js_name = addCircle)]
    pub fn add_circle(&mut self, c: usize, r: f64) -> Result<usize, JsError> {
        ops::add_circle(&mut self.sk, c, r).map_err(js)
    }

    /// Adds an arc around point c from point a counterclockwise to point b
    /// (|ca| = |cb| is enforced). Returns the entity.
    #[wasm_bindgen(js_name = addArc)]
    pub fn add_arc(&mut self, c: usize, a: usize, b: usize) -> Result<usize, JsError> {
        ops::add_arc(&mut self.sk, c, a, b).map_err(js)
    }

    /// Adds a curve given as text: `y = f(x)` or `F(x, y) = G(x, y)` in x, y,
    /// variables and dimensions. Returns the entity.
    #[wasm_bindgen(js_name = addCurve)]
    pub fn add_curve(&mut self, src: &str) -> Result<usize, JsError> {
        ops::add_curve(&mut self.sk, src).map_err(js)
    }

    /// Sets the text of curve `i`. False when rejected.
    #[wasm_bindgen(js_name = setCurve)]
    pub fn set_curve(&mut self, i: usize, src: &str) -> Result<bool, JsError> {
        ops::set_curve(&mut self.sk, i, src).map_err(js)
    }

    /// Adds a conic: `kind` is "ellipse", "hyperbola" or "parabola",
    /// centered on point c (the vertex of a parabola), with half-axes a and
    /// b (b is ignored for a parabola, where a is the focal distance) and
    /// axis angle t in radians. Returns the entity.
    #[wasm_bindgen(js_name = addConic)]
    pub fn add_conic(&mut self, kind: &str, c: usize, a: f64, b: f64, t: f64) -> Result<usize, JsError> {
        ops::add_conic(&mut self.sk, ops::conic_kind_named(kind).map_err(js)?, c, a, b, t).map_err(js)
    }

    /// Adds a point at (x, y) on curve c, bound to it by a constraint.
    #[wasm_bindgen(js_name = addPointOn)]
    pub fn add_point_on(&mut self, c: usize, x: f64, y: f64) -> Result<usize, JsError> {
        ops::add_point_on(&mut self.sk, c, x, y).map_err(js)
    }

    /// Adds the circle around point c through point p, bound to it.
    #[wasm_bindgen(js_name = addCircleTo)]
    pub fn add_circle_to(&mut self, c: usize, p: usize) -> Result<usize, JsError> {
        ops::circle_to(&mut self.sk, c, p).map_err(js)
    }

    /// Adds the arc around point c from point a, ending at a new point in
    /// the direction of (x, y).
    #[wasm_bindgen(js_name = addArcToward)]
    pub fn add_arc_toward(&mut self, c: usize, a: usize, x: f64, y: f64) -> Result<usize, JsError> {
        ops::arc_toward(&mut self.sk, c, a, [x, y]).map_err(js)
    }

    /// Adds the midpoint of points a and b, bound to them. Returns the point.
    #[wasm_bindgen(js_name = addMidpoint)]
    pub fn add_midpoint(&mut self, a: usize, b: usize) -> Result<usize, JsError> {
        ops::midpoint(&mut self.sk, a, b).map_err(js)
    }

    /// Adds the midpoint of line l, bound to it. Returns the point.
    #[wasm_bindgen(js_name = addLineMidpoint)]
    pub fn add_line_midpoint(&mut self, l: usize) -> Result<usize, JsError> {
        ops::line_midpoint(&mut self.sk, l).map_err(js)
    }

    /// Adds a point at a crossing of curves a and b, starting from (x, y).
    #[wasm_bindgen(js_name = addIntersection)]
    pub fn add_intersection(&mut self, a: usize, b: usize, x: f64, y: f64) -> Result<usize, JsError> {
        ops::intersection(&mut self.sk, a, b, [x, y]).map_err(js)
    }

    /// Adds a point at the crossing of curves a and b nearest to (x, y).
    #[wasm_bindgen(js_name = addIntersectionNear)]
    pub fn add_intersection_near(&mut self, a: usize, b: usize, x: f64, y: f64) -> Result<usize, JsError> {
        ops::intersect_near(&mut self.sk, a, b, [x, y]).map_err(js)
    }

    /// Adds the infinite line through point p perpendicular to line l.
    #[wasm_bindgen(js_name = addPerpLine)]
    pub fn add_perp_line(&mut self, l: usize, p: usize) -> Result<usize, JsError> {
        ops::dir_line(&mut self.sk, l, p, true).map_err(js)
    }

    /// Adds the infinite line through point p parallel to line l.
    #[wasm_bindgen(js_name = addParallelLine)]
    pub fn add_parallel_line(&mut self, l: usize, p: usize) -> Result<usize, JsError> {
        ops::dir_line(&mut self.sk, l, p, false).map_err(js)
    }

    /// Adds the perpendicular bisector of points a and b. Returns
    /// [line, midpoint].
    #[wasm_bindgen(js_name = addPerpBisector)]
    pub fn add_perp_bisector(&mut self, a: usize, b: usize) -> Result<Vec<u32>, JsError> {
        ops::perp_bisector(&mut self.sk, a, b).map(|(l, m)| vec![l as u32, m as u32]).map_err(js)
    }

    /// Adds the bisector of lines l1 and l2 in the sector containing (x, y).
    /// Returns [line, vertex]; the vertex is a shared endpoint, or a new
    /// point at the crossing.
    #[wasm_bindgen(js_name = addAngleBisector)]
    pub fn add_angle_bisector(&mut self, l1: usize, l2: usize, x: f64, y: f64) -> Result<Vec<u32>, JsError> {
        ops::angle_bisector(&mut self.sk, l1, l2, [x, y]).map(|(l, v)| vec![l as u32, v as u32]).map_err(js)
    }

    /// Adds the segment from point p touching curve c at a contact point
    /// starting from (x, y). Returns [segment, contact point].
    #[wasm_bindgen(js_name = addTangentLine)]
    pub fn add_tangent_line(&mut self, p: usize, c: usize, x: f64, y: f64) -> Result<Vec<u32>, JsError> {
        ops::tangent_line(&mut self.sk, p, c, [x, y]).map(|(l, t)| vec![l as u32, t as u32]).map_err(js)
    }

    /// Like `addTangentLine`, with the contact point nearest to (x, y).
    #[wasm_bindgen(js_name = addTangentLineNear)]
    pub fn add_tangent_line_near(&mut self, p: usize, c: usize, x: f64, y: f64) -> Result<Vec<u32>, JsError> {
        ops::tangent_line_near(&mut self.sk, p, c, [x, y]).map(|(l, t)| vec![l as u32, t as u32]).map_err(js)
    }

    /// Adds the mirror image of point p in line l.
    #[wasm_bindgen(js_name = addReflection)]
    pub fn add_reflection(&mut self, p: usize, l: usize) -> Result<usize, JsError> {
        ops::reflection(&mut self.sk, p, l).map_err(js)
    }

    /// Adds the circle through points a, b and c. Returns [circle, center],
    /// the center being a new point.
    #[wasm_bindgen(js_name = addCircleThrough)]
    pub fn add_circle_through(&mut self, a: usize, b: usize, c: usize) -> Result<Vec<u32>, JsError> {
        ops::circle_through(&mut self.sk, a, b, c).map(|(x, o)| vec![x as u32, o as u32]).map_err(js)
    }

    /// Adds the arc from point a through point m to point b. Returns [arc,
    /// center], the center being a new point.
    #[wasm_bindgen(js_name = addArcThrough)]
    pub fn add_arc_through(&mut self, a: usize, m: usize, b: usize) -> Result<Vec<u32>, JsError> {
        ops::arc_through(&mut self.sk, a, m, b).map(|(x, o)| vec![x as u32, o as u32]).map_err(js)
    }

    /// Adds the ellipse or hyperbola (`kind` "ellipse" or "hyperbola") with
    /// foci f1 and f2 through point p.
    #[wasm_bindgen(js_name = addConicByFoci)]
    pub fn add_conic_by_foci(&mut self, kind: &str, f1: usize, f2: usize, p: usize) -> Result<usize, JsError> {
        ops::conic_by_foci(&mut self.sk, ops::conic_kind_named(kind).map_err(js)?, f1, f2, p).map_err(js)
    }

    /// Adds the parabola with focus f and directrix line l.
    #[wasm_bindgen(js_name = addParabolaByFocus)]
    pub fn add_parabola_by_focus(&mut self, f: usize, l: usize) -> Result<usize, JsError> {
        ops::parabola_by_focus(&mut self.sk, f, l).map_err(js)
    }

    /// Adds the ellipse or hyperbola centered on point c, its first axis
    /// ending at (ax, ay) and its second half-axis reaching the line through
    /// (sx, sy) parallel to the first axis.
    #[wasm_bindgen(js_name = addConicFrom)]
    pub fn add_conic_from(&mut self, kind: &str, c: usize, ax: f64, ay: f64, sx: f64, sy: f64) -> Result<usize, JsError> {
        ops::conic_from(&mut self.sk, ops::conic_kind_named(kind).map_err(js)?, c, [ax, ay], [sx, sy]).map_err(js)
    }

    /// Adds the parabola with vertex at point c opening toward (x, y), its
    /// focal distance the distance to (x, y).
    #[wasm_bindgen(js_name = addParabolaToward)]
    pub fn add_parabola_toward(&mut self, c: usize, x: f64, y: f64) -> Result<usize, JsError> {
        ops::parabola_toward(&mut self.sk, c, [x, y]).map_err(js)
    }

    /// Adds the foci of conic c: two points, one for a parabola.
    #[wasm_bindgen(js_name = addFoci)]
    pub fn add_foci(&mut self, c: usize) -> Result<Vec<u32>, JsError> {
        ops::conic_feature(&mut self.sk, ConicFeature::Foci, c, [0.0; 2]).map(|v| v.into_iter().map(|e| e as u32).collect()).map_err(js)
    }

    /// Adds the directrix of conic c belonging to the focus nearest (x, y).
    #[wasm_bindgen(js_name = addDirectrix)]
    pub fn add_directrix(&mut self, c: usize, x: f64, y: f64) -> Result<usize, JsError> {
        ops::conic_feature(&mut self.sk, ConicFeature::Directrix, c, [x, y]).map_err(js)?.first().copied().ok_or_else(|| js("no directrix".into()))
    }

    /// Adds both asymptotes of hyperbola c.
    #[wasm_bindgen(js_name = addAsymptotes)]
    pub fn add_asymptotes(&mut self, c: usize) -> Result<Vec<u32>, JsError> {
        ops::conic_feature(&mut self.sk, ConicFeature::Asymptotes, c, [0.0; 2]).map(|v| v.into_iter().map(|e| e as u32).collect()).map_err(js)
    }

    /// Adds a constraint: `kind` is "coincident", "on_curve", "midpoint",
    /// "horizontal", "vertical", "parallel", "perpendicular", "collinear",
    /// "equal", "concentric" or "tangent", on entities a and b ("horizontal"
    /// and "vertical" take a line, or two points). False when it is
    /// rejected.
    pub fn constrain(&mut self, kind: &str, a: usize, b: Option<usize>) -> Result<bool, JsError> {
        ops::constrain(&mut self.sk, ConKind::from_name(kind).map_err(js)?, a, b.unwrap_or(usize::MAX)).map_err(js)
    }

    /// Adds a dimension: `kind` is "distance", "hdist", "vdist",
    /// "point_line", "length", "radius", "diameter", "angle", "conic_a",
    /// "conic_b", "x" or "y", on entities a and b, with its label at
    /// (offX, offY) (default 0.3, 0.3) from the dimension's anchor (the midpoint of a length,
    /// the vertex of an angle). It is driving when that keeps the
    /// constraints solvable, otherwise a reference (measured) dimension. For
    /// an angle, the sector holding the label selects the angle or its
    /// supplement. Returns the dimension.
    #[wasm_bindgen(js_name = addDim)]
    pub fn add_dim(&mut self, kind: &str, a: usize, b: Option<usize>, off_x: Option<f64>, off_y: Option<f64>) -> Result<usize, JsError> {
        let off = [off_x.unwrap_or(0.3), off_y.unwrap_or(0.3)];
        ops::add_dim(&mut self.sk, DimType::from_name(kind).map_err(js)?, a, b.unwrap_or(usize::MAX), off).map_err(js)
    }

    /// Sets dimension `i` from text as typed: a number (degrees
    /// for angles) or an expression of variables and dimensions drives it;
    /// empty text makes it a reference dimension. False when the value
    /// cannot be reached.
    #[wasm_bindgen(js_name = setDim)]
    pub fn set_dim(&mut self, i: usize, text: &str) -> Result<bool, JsError> {
        ops::set_dim(&mut self.sk, i, text).map_err(js)
    }

    /// Makes dimension `i` driving or reference. False when making it
    /// driving is rejected (the constraints already determine it).
    #[wasm_bindgen(js_name = setDimDriving)]
    pub fn set_dim_driving(&mut self, i: usize, driving: bool) -> Result<bool, JsError> {
        ops::set_driving(&mut self.sk, i, driving).map_err(js)
    }

    /// Marks dimension `i` numeric or not. Formulas derived for other targets
    /// use a numeric dimension's current value as a plain number. Solving and
    /// the dimension itself are unaffected.
    #[wasm_bindgen(js_name = setDimNumeric)]
    pub fn set_dim_numeric(&mut self, i: usize, numeric: bool) -> Result<bool, JsError> {
        ops::set_dim_numeric(&mut self.sk, i, numeric).map_err(js)
    }

    /// Adds a value variable. Without a name, the first unused of a, b, c, k,
    /// ... is taken. A free variable (default) is set by `value` (default
    /// 1), otherwise it is solved from the constraints, starting at `value`.
    /// Returns the variable.
    #[wasm_bindgen(js_name = addVar)]
    pub fn add_var(&mut self, name: Option<String>, value: Option<f64>, free: Option<bool>) -> Result<usize, JsError> {
        ops::add_var(&mut self.sk, name.as_deref(), value.unwrap_or(1.0), free.unwrap_or(true)).map_err(js)
    }

    /// Adds a variable defined by the expression `expr` of variables and
    /// dimensions. Without a name, f1, f2, ... is taken. Returns the variable.
    #[wasm_bindgen(js_name = addDefVar)]
    pub fn add_def_var(&mut self, name: Option<String>, expr: &str) -> Result<usize, JsError> {
        ops::add_defvar(&mut self.sk, name.as_deref(), expr).map_err(js)
    }

    /// Sets the value of value variable `i`. False when the value cannot be reached.
    #[wasm_bindgen(js_name = setVar)]
    pub fn set_var(&mut self, i: usize, value: f64) -> Result<bool, JsError> {
        ops::set_var(&mut self.sk, i, value).map_err(js)
    }

    /// Sets variable `i` from text as typed: a plain number makes it
    /// a free value variable, any other text an expression of variables and
    /// dimensions. False when rejected.
    #[wasm_bindgen(js_name = setVarExpr)]
    pub fn set_var_expr(&mut self, i: usize, text: &str) -> Result<bool, JsError> {
        ops::set_var_expr(&mut self.sk, i, text).map_err(js)
    }

    /// Makes value variable `i` free (set by its value) or solved from the
    /// constraints. False when rejected.
    #[wasm_bindgen(js_name = setVarFree)]
    pub fn set_var_free(&mut self, i: usize, free: bool) -> Result<bool, JsError> {
        ops::set_var_free(&mut self.sk, i, free).map_err(js)
    }

    /// Marks variable `i` numeric or not. Formulas derived for other targets
    /// use a numeric variable's current value as a plain number. Solving and
    /// the variable itself (value, expression, free or solved) are
    /// unaffected.
    #[wasm_bindgen(js_name = setVarNumeric)]
    pub fn set_var_numeric(&mut self, i: usize, numeric: bool) -> Result<bool, JsError> {
        ops::set_var_numeric(&mut self.sk, i, numeric).map_err(js)
    }

    /// Removes variable `i`, leaving an empty slot; expressions naming it
    /// fail to parse until edited. False when rejected.
    #[wasm_bindgen(js_name = deleteVar)]
    pub fn delete_var(&mut self, i: usize) -> Result<bool, JsError> {
        ops::delete_var(&mut self.sk, i).map_err(js)
    }

    /// Deletes entity `e` and what depends on it (constraints and dimensions
    /// that reference it, and features built from it). False when the
    /// constraints would no longer solve.
    #[wasm_bindgen(js_name = deleteEnt)]
    pub fn delete_ent(&mut self, e: usize) -> Result<bool, JsError> {
        ops::delete_ent(&mut self.sk, e).map_err(js)
    }

    /// Deletes constraint `c`. False when the constraints would no longer
    /// solve.
    #[wasm_bindgen(js_name = deleteCon)]
    pub fn delete_con(&mut self, c: usize) -> Result<bool, JsError> {
        ops::delete_con(&mut self.sk, c).map_err(js)
    }

    /// Deletes dimension `i`; expressions naming it fail to parse until
    /// edited. False when the constraints would no longer solve.
    #[wasm_bindgen(js_name = deleteDim)]
    pub fn delete_dim(&mut self, i: usize) -> Result<bool, JsError> {
        ops::delete_dim(&mut self.sk, i).map_err(js)
    }

    /// Locks point `e` (its coordinates become fixed) or unlocks it.
    #[wasm_bindgen(js_name = lockPoint)]
    pub fn lock_point(&mut self, e: usize, on: bool) -> Result<bool, JsError> {
        ops::lock_point(&mut self.sk, e, on).map_err(js)
    }

    /// True when point `e` is locked (both coordinates fixed).
    #[wasm_bindgen(js_name = pointLocked)]
    pub fn point_locked(&self, e: usize) -> Result<bool, JsError> {
        ops::point_locked(&self.sk, e).map_err(js)
    }

    /// Sets the construction flag of entity `e` (a dashed helper).
    #[wasm_bindgen(js_name = setConstruction)]
    pub fn set_construction(&mut self, e: usize, on: bool) -> Result<bool, JsError> {
        ops::set_construction(&mut self.sk, e, on).map_err(js)
    }

    /// Sets the hidden flag of entity `e`. Hidden entities are not drawn.
    #[wasm_bindgen(js_name = setHidden)]
    pub fn set_hidden(&mut self, e: usize, on: bool) -> Result<bool, JsError> {
        ops::set_hidden(&mut self.sk, e, on).map_err(js)
    }

    /// Makes line `e` infinite or a segment.
    #[wasm_bindgen(js_name = setInfinite)]
    pub fn set_infinite(&mut self, e: usize, on: bool) -> Result<bool, JsError> {
        ops::set_infinite(&mut self.sk, e, on).map_err(js)
    }

    /// Pulls point `e` toward (x, y) by solving the constraints with it
    /// pinned there (a drag, not an undoable change). False when the
    /// configuration cannot reach the target.
    #[wasm_bindgen(js_name = dragPoint)]
    pub fn drag_point(&mut self, e: usize, x: f64, y: f64) -> Result<bool, JsError> {
        ops::drag_point(&mut self.sk, e, x, y).map_err(js)
    }

    /// Pulls the radius of circle `e` so its edge passes through (x, y).
    #[wasm_bindgen(js_name = dragCircle)]
    pub fn drag_circle(&mut self, e: usize, x: f64, y: f64) -> Result<bool, JsError> {
        ops::drag_circle_edge(&mut self.sk, e, x, y).map_err(js)
    }

    /// Pulls a handle of conic `e` (`which` 0 the first-axis end, 1 the
    /// second-axis end) toward (x, y).
    #[wasm_bindgen(js_name = dragConicHandle)]
    pub fn drag_conic_handle(&mut self, e: usize, which: u8, x: f64, y: f64) -> Result<bool, JsError> {
        ops::drag_conic_handle(&mut self.sk, e, which, x, y).map_err(js)
    }

    /// Gives the point, dimension or variable named `old` the name `new`,
    /// rewriting expressions that use it. False when `new` is not an
    /// identifier or is taken.
    pub fn rename(&mut self, old: &str, new: &str) -> Result<bool, JsError> {
        ops::rename(&mut self.sk, old, new).map_err(js)
    }

    // ------------------------------------------------------------ reading

    /// Number of dimension slots. Removed dimensions leave empty slots.
    #[wasm_bindgen(getter, js_name = dimCount)]
    pub fn dim_count(&self) -> usize {
        self.sk.doc.dims.len()
    }

    /// Name of dimension `i`; undefined for an empty slot or a bad index.
    #[wasm_bindgen(js_name = dimName)]
    pub fn dim_name(&self, i: usize) -> Option<String> {
        self.sk.doc.dims.get(i)?.as_ref().map(|d| d.name.clone())
    }

    /// Current measured value of dimension `i` (radians for angles); NaN
    /// for an empty slot or a bad index.
    #[wasm_bindgen(js_name = dimValue)]
    pub fn dim_value(&self, i: usize) -> f64 {
        match self.sk.doc.dims.get(i) {
            Some(Some(_)) => self.sk.doc.measure_value(i),
            _ => f64::NAN,
        }
    }

    /// True when dimension `i` is driving, false when it is a reference dimension.
    #[wasm_bindgen(js_name = dimDriving)]
    pub fn dim_driving(&self, i: usize) -> Result<bool, JsError> {
        ops::dim_driving(&self.sk, i).map_err(js)
    }

    /// True when dimension `i` is numeric (substituted as a number in formulas).
    #[wasm_bindgen(js_name = dimNumeric)]
    pub fn dim_numeric(&self, i: usize) -> Result<bool, JsError> {
        ops::dim_numeric(&self.sk, i).map_err(js)
    }

    /// Number of variable slots. Removed variables leave empty slots.
    #[wasm_bindgen(getter, js_name = varCount)]
    pub fn var_count(&self) -> usize {
        self.sk.doc.vars.len()
    }

    /// Name of variable `i`; undefined for an empty slot or a bad index.
    #[wasm_bindgen(js_name = varName)]
    pub fn var_name(&self, i: usize) -> Option<String> {
        self.sk.doc.vars.get(i)?.as_ref().map(|v| v.name.clone())
    }

    /// Value of variable `i`; NaN for an empty slot or a bad index.
    #[wasm_bindgen(js_name = varValue)]
    pub fn var_value(&self, i: usize) -> f64 {
        ops::var_value(&self.sk, i).unwrap_or(f64::NAN)
    }

    /// Expression of variable `i` (empty for a value variable); undefined
    /// for an empty slot or a bad index.
    #[wasm_bindgen(js_name = varExpr)]
    pub fn var_expr(&self, i: usize) -> Option<String> {
        ops::var_expr(&self.sk, i).ok()
    }

    /// True when variable `i` is free, false when it is solved or defined by an expression.
    #[wasm_bindgen(js_name = varFree)]
    pub fn var_free(&self, i: usize) -> Result<bool, JsError> {
        ops::var_free(&self.sk, i).map_err(js)
    }

    /// True when variable `i` is numeric (substituted as a number in formulas).
    #[wasm_bindgen(js_name = varNumeric)]
    pub fn var_numeric(&self, i: usize) -> Result<bool, JsError> {
        ops::var_numeric(&self.sk, i).map_err(js)
    }

    // ------------------------------------------------------------ entities

    /// Number of entity slots. Deleted entities leave empty slots.
    #[wasm_bindgen(getter, js_name = entCount)]
    pub fn ent_count(&self) -> usize {
        ops::ent_count(&self.sk)
    }

    /// Kind of entity `e`: "point", "line", "circle", "arc", "conic" or
    /// "curve"; undefined for an empty slot or a bad index.
    #[wasm_bindgen(js_name = entKind)]
    pub fn ent_kind(&self, e: usize) -> Option<String> {
        ops::ent_kind(&self.sk, e).ok().map(|k| k.name().to_string())
    }

    /// Name of entity `e` (points are named); undefined for an empty slot, a
    /// bad index or an unnamed entity.
    #[wasm_bindgen(js_name = entName)]
    pub fn ent_name(&self, e: usize) -> Option<String> {
        ops::ent_name(&self.sk, e).ok()
    }

    /// World coordinates of point `e` as [x, y]; undefined for an empty slot
    /// or a non-point.
    #[wasm_bindgen(js_name = pointPos)]
    pub fn point_pos(&self, e: usize) -> Option<Vec<JsValue>> {
        ops::point_pos(&self.sk, e).ok().map(|p| p.iter().map(|x| JsValue::from_f64(*x)).collect())
    }

    /// Endpoints of line `e` as [a, b, infinite]; undefined for a non-line.
    #[wasm_bindgen(js_name = linePts)]
    pub fn line_pts(&self, e: usize) -> Option<Vec<JsValue>> {
        let (a, b, inf) = ops::line_pts(&self.sk, e).ok()?;
        Some(vec![JsValue::from_f64(a as f64), JsValue::from_f64(b as f64), JsValue::from_bool(inf)])
    }

    /// Center point and radius of circle or arc `e` as [center, radius];
    /// undefined for a non-circle.
    #[wasm_bindgen(js_name = circleVals)]
    pub fn circle_vals(&self, e: usize) -> Option<Vec<JsValue>> {
        let (c, r) = ops::circle_vals(&self.sk, e).ok()?;
        Some(vec![JsValue::from_f64(c as f64), JsValue::from_f64(r)])
    }

    /// Center and endpoints of arc `e` as [c, a, b]; undefined for a non-arc.
    #[wasm_bindgen(js_name = arcPts)]
    pub fn arc_pts(&self, e: usize) -> Option<Vec<JsValue>> {
        let (c, a, b) = ops::arc_pts(&self.sk, e).ok()?;
        Some(vec![JsValue::from_f64(c as f64), JsValue::from_f64(a as f64), JsValue::from_f64(b as f64)])
    }

    /// Kind, center and axis values of conic `e` as
    /// [kind, center, a, b, angle]; undefined for a non-conic.
    #[wasm_bindgen(js_name = conicVals)]
    pub fn conic_vals(&self, e: usize) -> Option<Vec<JsValue>> {
        let (k, c, a, b, t) = ops::conic_vals(&self.sk, e).ok()?;
        let name = match k {
            crate::model::ConicKind::Ellipse => "ellipse",
            crate::model::ConicKind::Hyperbola => "hyperbola",
            crate::model::ConicKind::Parabola => "parabola",
        };
        Some(vec![JsValue::from_str(name), JsValue::from_f64(c as f64), JsValue::from_f64(a), JsValue::from_f64(b), JsValue::from_f64(t)])
    }

    /// Source text of function curve `e`; undefined for a non-curve.
    #[wasm_bindgen(js_name = curveSrc)]
    pub fn curve_src(&self, e: usize) -> Option<String> {
        ops::curve_src(&self.sk, e).ok()
    }

    /// True when entity `e` is a construction (dashed helper).
    #[wasm_bindgen(js_name = entConstruction)]
    pub fn ent_construction(&self, e: usize) -> Result<bool, JsError> {
        ops::ent_construction(&self.sk, e).map_err(js)
    }

    /// True when entity `e` is hidden.
    #[wasm_bindgen(js_name = entHidden)]
    pub fn ent_hidden(&self, e: usize) -> Result<bool, JsError> {
        ops::ent_hidden(&self.sk, e).map_err(js)
    }

    /// True when line `e` is infinite.
    #[wasm_bindgen(js_name = entInfinite)]
    pub fn ent_infinite(&self, e: usize) -> Result<bool, JsError> {
        ops::ent_infinite(&self.sk, e).map_err(js)
    }

    /// Number of constraint slots. Deleted constraints leave empty slots.
    #[wasm_bindgen(getter, js_name = conCount)]
    pub fn con_count(&self) -> usize {
        ops::con_count(&self.sk)
    }

    /// Kind name of constraint `c` (e.g. "coincident", "parallel",
    /// "symmetric", ...); undefined for an empty slot or a bad index.
    #[wasm_bindgen(js_name = conKind)]
    pub fn con_kind(&self, c: usize) -> Option<String> {
        ops::con_kind(&self.sk, c).ok().map(|k| k.name().to_string())
    }

    /// The entities of constraint `c`, in the order they appear in it.
    #[wasm_bindgen(js_name = conEnts)]
    pub fn con_ents(&self, c: usize) -> Result<Vec<u32>, JsError> {
        ops::con_ents(&self.sk, c).map(|v| v.into_iter().map(|e| e as u32).collect()).map_err(js)
    }

    /// Kind name of dimension `i` ("distance", "radius", "angle", ...).
    #[wasm_bindgen(js_name = dimKind)]
    pub fn dim_kind(&self, i: usize) -> Option<String> {
        ops::dim_kind(&self.sk, i).ok().map(|k| k.name().to_string())
    }

    /// World-coordinate polylines drawing entity `e` inside the window from
    /// (x0, y0) to (x1, y1) with pixels of size `px`, as an array of
    /// polylines, each an array of [x, y] pairs: one point for a point, the
    /// endpoints of a segment, a segment crossing the window for an infinite
    /// line, the outline of a circle, arc or conic (two polylines for a
    /// hyperbola), and the parts of a function curve inside the window.
    #[wasm_bindgen(js_name = entSamples)]
    #[allow(clippy::too_many_arguments)]
    pub fn ent_samples(&self, e: usize, x0: f64, y0: f64, x1: f64, y1: f64, px: f64) -> Result<js_sys::Array, JsError> {
        let polys = ops::ent_samples(&self.sk, e, [x0, y0], [x1, y1], px).map_err(js)?;
        let outer = js_sys::Array::new();
        for pl in polys {
            let line = js_sys::Array::new();
            for p in pl {
                line.push(&js_sys::Array::of2(&JsValue::from_f64(p[0]), &JsValue::from_f64(p[1])));
            }
            outer.push(&line);
        }
        Ok(outer)
    }

    // ------------------------------------------------------------ formulas

    /// Closed-form formula of dimension `i`. `extraMs` extends the search
    /// beyond the default budget.
    #[wasm_bindgen(js_name = deriveDim)]
    pub fn derive_dim(&self, i: usize, extra_ms: Option<u32>) -> Result<Formula, JsError> {
        self.derive(Target::Dim(i), extra_ms)
    }

    /// Closed-form formula of variable `i`. `extraMs` extends the search
    /// beyond the default budget.
    #[wasm_bindgen(js_name = deriveVar)]
    pub fn derive_var(&self, i: usize, extra_ms: Option<u32>) -> Result<Formula, JsError> {
        self.derive(Target::Var(i), extra_ms)
    }
}

impl JsDoc {
    fn derive(&self, t: Target, extra_ms: Option<u32>) -> Result<Formula, JsError> {
        let f = formula::derive_formula(&self.sk.doc, t, extra_ms.unwrap_or(0) as u64).map_err(js)?;
        Ok(Formula { name: f.name, plain: f.plain, latex: f.latex, exact: f.exact, is_angle: f.is_angle, value: f.value, actual: f.actual })
    }
}

/// Version of the library.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").into()
}
