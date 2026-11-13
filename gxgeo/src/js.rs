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

use crate::ops::{self, ConKind, ConicFeature, DimType, Document, Target};
use wasm_bindgen::prelude::*;

/// A geometry document with its solver state and undo history.
#[wasm_bindgen(js_name = Doc)]
pub struct JsDoc {
    doc: Document,
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
    /// `name = formula` as LaTeX with repeated subexpressions replaced by
    /// `u_{1}`, `u_{2}`, ..., which `defs` defines.
    #[wasm_bindgen(js_name = latexShort)]
    pub latex_short: String,
    /// The subexpressions of `latexShort`, in order.
    pub defs: Vec<Def>,
    /// Complexity of the formula: its number of operations and terms.
    pub size: usize,
}

/// A subexpression of a formula.
#[wasm_bindgen(getter_with_clone)]
#[derive(Clone)]
pub struct Def {
    /// LaTeX name, such as `u_{1}`.
    pub name: String,
    /// LaTeX expression; can name earlier subexpressions.
    pub latex: String,
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

fn ids(v: &[u32]) -> Vec<usize> {
    v.iter().map(|e| *e as usize).collect()
}

fn ids_out(v: Vec<usize>) -> Vec<u32> {
    v.into_iter().map(|e| e as u32).collect()
}

fn pair(p: [f64; 2]) -> Vec<JsValue> {
    p.iter().map(|x| JsValue::from_f64(*x)).collect()
}

#[wasm_bindgen(js_class = Doc)]
impl JsDoc {
    /// New document holding only the origin point and the x and y axes.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> JsDoc {
        JsDoc { doc: Document::new() }
    }

    /// Document decoded from .gxp text. Call `solve` before reading values from it.
    #[wasm_bindgen(js_name = fromGxp)]
    pub fn from_gxp(text: &str) -> Result<JsDoc, JsError> {
        Ok(JsDoc { doc: Document::from_gxp(text).map_err(js)? })
    }

    /// Document encoded as indented .gxp text.
    #[wasm_bindgen(js_name = toGxp)]
    pub fn to_gxp(&self) -> Result<String, JsError> {
        ops::to_gxp(&self.doc).map_err(js)
    }

    /// Solves the constraints and stores the solution in the document.
    pub fn solve(&mut self) -> Solved {
        let ok = ops::solve(&mut self.doc);
        Solved { ok, dof: ops::dof(&self.doc), errors: ops::errors(&self.doc) }
    }

    /// Undoes the last change; false when there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        ops::undo(&mut self.doc)
    }

    /// Redoes the last undone change; false when there is nothing to redo.
    pub fn redo(&mut self) -> bool {
        ops::redo(&mut self.doc)
    }

    /// Records the current document as an undo step. A drag (`dragPoint`,
    /// `dragVar`, ...) is not undoable by itself, so a checkpoint before it
    /// makes the whole drag one undo step.
    pub fn checkpoint(&mut self) {
        ops::checkpoint(&mut self.doc);
    }

    /// Remaining degrees of freedom after the last solve.
    pub fn dof(&self) -> usize {
        ops::dof(&self.doc)
    }

    /// Number of steps in the undo history; see `joinUndo`.
    #[wasm_bindgen(js_name = undoLen)]
    pub fn undo_len(&self) -> usize {
        ops::undo_len(&self.doc)
    }

    /// Joins the undo steps recorded since the undo history held `len` steps
    /// (a value of `undoLen` read before them) into one step, so that a
    /// change made with several calls undoes at once. False when fewer than
    /// two steps were recorded since. The history keeps at most 300 steps;
    /// past that, steps recorded since `len` can be left unjoined.
    #[wasm_bindgen(js_name = joinUndo)]
    pub fn join_undo(&mut self, len: usize) -> bool {
        ops::join_undo(&mut self.doc, len)
    }

    /// A copy of the document in its current state, with the same solve
    /// status and without the undo history.
    pub fn copy(&self) -> JsDoc {
        JsDoc { doc: ops::copy(&self.doc) }
    }

    /// Replaces the content of the document with the document decoded from
    /// .gxp text, as one undoable change, and solves it.
    #[wasm_bindgen(js_name = loadGxp)]
    pub fn load_gxp(&mut self, text: &str) -> Result<(), JsError> {
        ops::load_gxp(&mut self.doc, text).map_err(js)
    }

    /// Replaces the content of the document with an empty document (origin
    /// and axes only), as one undoable change.
    pub fn clear(&mut self) {
        ops::clear(&mut self.doc);
    }

    /// Counter raised by every change of the document, including drags;
    /// equal values mean an unchanged document.
    #[wasm_bindgen(getter)]
    pub fn generation(&self) -> f64 {
        ops::generation(&self.doc) as f64
    }

    /// Number of constraints, dimensions and entities the last solve left
    /// unsatisfied.
    #[wasm_bindgen(getter, js_name = conflictCount)]
    pub fn conflict_count(&self) -> usize {
        ops::conflict_count(&self.doc)
    }

    // ------------------------------------------------------------ construction

    /// The origin point.
    #[wasm_bindgen(getter)]
    pub fn origin(&self) -> usize {
        ops::origin(&self.doc)
    }

    /// The x axis (an infinite line through the origin).
    #[wasm_bindgen(getter, js_name = xAxis)]
    pub fn x_axis(&self) -> usize {
        ops::x_axis(&self.doc)
    }

    /// The y axis (an infinite line through the origin).
    #[wasm_bindgen(getter, js_name = yAxis)]
    pub fn y_axis(&self) -> usize {
        ops::y_axis(&self.doc)
    }

    /// Adds a free point at (x, y), its starting position. Returns the entity.
    #[wasm_bindgen(js_name = addPoint)]
    pub fn add_point(&mut self, x: f64, y: f64) -> Result<usize, JsError> {
        ops::add_point(&mut self.doc, x, y).map_err(js)
    }

    /// Adds a line from point a to point b: a segment, or an infinite line
    /// when `infinite`. With `axis` "horizontal" or "vertical", the line is
    /// also constrained along that axis. Returns the entity.
    #[wasm_bindgen(js_name = addLine)]
    pub fn add_line(&mut self, a: usize, b: usize, infinite: Option<bool>, axis: Option<String>) -> Result<usize, JsError> {
        let inf = infinite.unwrap_or(false);
        match axis.as_deref() {
            None => ops::add_line(&mut self.doc, a, b, inf),
            Some("horizontal") => ops::line_between(&mut self.doc, a, b, inf, Some(true)),
            Some("vertical") => ops::line_between(&mut self.doc, a, b, inf, Some(false)),
            Some(o) => Err(format!("unknown axis {o}")),
        }
        .map_err(js)
    }

    /// Adds a circle around point c with starting radius r (the radius stays
    /// free until constrained). Returns the entity.
    #[wasm_bindgen(js_name = addCircle)]
    pub fn add_circle(&mut self, c: usize, r: f64) -> Result<usize, JsError> {
        ops::add_circle(&mut self.doc, c, r).map_err(js)
    }

    /// Adds an arc around point c from point a counterclockwise to point b
    /// (|ca| = |cb| is enforced). Returns the entity.
    #[wasm_bindgen(js_name = addArc)]
    pub fn add_arc(&mut self, c: usize, a: usize, b: usize) -> Result<usize, JsError> {
        ops::add_arc(&mut self.doc, c, a, b).map_err(js)
    }

    /// Adds a curve given as text: `y = f(x)` or `F(x, y) = G(x, y)` in x, y,
    /// variables and dimensions. Returns the entity.
    #[wasm_bindgen(js_name = addCurve)]
    pub fn add_curve(&mut self, src: &str) -> Result<usize, JsError> {
        ops::add_curve(&mut self.doc, src).map_err(js)
    }

    /// Sets the text of curve `i`. False when rejected.
    #[wasm_bindgen(js_name = setCurve)]
    pub fn set_curve(&mut self, i: usize, src: &str) -> Result<bool, JsError> {
        ops::set_curve(&mut self.doc, i, src).map_err(js)
    }

    /// Adds a conic: `kind` is "ellipse", "hyperbola" or "parabola",
    /// centered on point c (the vertex of a parabola), with half-axes a and
    /// b (b is ignored for a parabola, where a is the focal distance) and
    /// axis angle t in radians. Returns the entity.
    #[wasm_bindgen(js_name = addConic)]
    pub fn add_conic(&mut self, kind: &str, c: usize, a: f64, b: f64, t: f64) -> Result<usize, JsError> {
        ops::add_conic(&mut self.doc, ops::conic_kind_named(kind).map_err(js)?, c, a, b, t).map_err(js)
    }

    /// Adds a point at (x, y) on curve c, bound to it by a constraint.
    #[wasm_bindgen(js_name = addPointOn)]
    pub fn add_point_on(&mut self, c: usize, x: f64, y: f64) -> Result<usize, JsError> {
        ops::add_point_on(&mut self.doc, c, x, y).map_err(js)
    }

    /// Adds the circle around point c through point p, bound to it.
    #[wasm_bindgen(js_name = addCircleTo)]
    pub fn add_circle_to(&mut self, c: usize, p: usize) -> Result<usize, JsError> {
        ops::circle_to(&mut self.doc, c, p).map_err(js)
    }

    /// Adds the arc around point c from point a, ending at a new point in
    /// the direction of (x, y).
    #[wasm_bindgen(js_name = addArcToward)]
    pub fn add_arc_toward(&mut self, c: usize, a: usize, x: f64, y: f64) -> Result<usize, JsError> {
        ops::arc_toward(&mut self.doc, c, a, [x, y]).map_err(js)
    }

    /// Adds the midpoint of points a and b, bound to them. Returns the point.
    #[wasm_bindgen(js_name = addMidpoint)]
    pub fn add_midpoint(&mut self, a: usize, b: usize) -> Result<usize, JsError> {
        ops::midpoint(&mut self.doc, a, b).map_err(js)
    }

    /// Adds the midpoint of line l, bound to it. Returns the point.
    #[wasm_bindgen(js_name = addLineMidpoint)]
    pub fn add_line_midpoint(&mut self, l: usize) -> Result<usize, JsError> {
        ops::line_midpoint(&mut self.doc, l).map_err(js)
    }

    /// Adds a point at a crossing of curves a and b, starting from (x, y).
    #[wasm_bindgen(js_name = addIntersection)]
    pub fn add_intersection(&mut self, a: usize, b: usize, x: f64, y: f64) -> Result<usize, JsError> {
        ops::intersection(&mut self.doc, a, b, [x, y]).map_err(js)
    }

    /// Adds a point at the crossing of curves a and b nearest to (x, y).
    #[wasm_bindgen(js_name = addIntersectionNear)]
    pub fn add_intersection_near(&mut self, a: usize, b: usize, x: f64, y: f64) -> Result<usize, JsError> {
        ops::intersect_near(&mut self.doc, a, b, [x, y]).map_err(js)
    }

    /// Adds the infinite line through point p perpendicular to line l.
    #[wasm_bindgen(js_name = addPerpLine)]
    pub fn add_perp_line(&mut self, l: usize, p: usize) -> Result<usize, JsError> {
        ops::dir_line(&mut self.doc, l, p, true).map_err(js)
    }

    /// Adds the infinite line through point p parallel to line l.
    #[wasm_bindgen(js_name = addParallelLine)]
    pub fn add_parallel_line(&mut self, l: usize, p: usize) -> Result<usize, JsError> {
        ops::dir_line(&mut self.doc, l, p, false).map_err(js)
    }

    /// Adds the perpendicular bisector of points a and b. Returns
    /// [line, midpoint].
    #[wasm_bindgen(js_name = addPerpBisector)]
    pub fn add_perp_bisector(&mut self, a: usize, b: usize) -> Result<Vec<u32>, JsError> {
        ops::perp_bisector(&mut self.doc, a, b).map(|(l, m)| vec![l as u32, m as u32]).map_err(js)
    }

    /// Adds the bisector of lines l1 and l2 in the sector containing (x, y).
    /// Returns [line, vertex]; the vertex is a shared endpoint, or a new
    /// point at the crossing.
    #[wasm_bindgen(js_name = addAngleBisector)]
    pub fn add_angle_bisector(&mut self, l1: usize, l2: usize, x: f64, y: f64) -> Result<Vec<u32>, JsError> {
        ops::angle_bisector(&mut self.doc, l1, l2, [x, y]).map(|(l, v)| vec![l as u32, v as u32]).map_err(js)
    }

    /// Adds the segment from point p touching curve c at a contact point
    /// starting from (x, y). Returns [segment, contact point].
    #[wasm_bindgen(js_name = addTangentLine)]
    pub fn add_tangent_line(&mut self, p: usize, c: usize, x: f64, y: f64) -> Result<Vec<u32>, JsError> {
        ops::tangent_line(&mut self.doc, p, c, [x, y]).map(|(l, t)| vec![l as u32, t as u32]).map_err(js)
    }

    /// Like `addTangentLine`, with the contact point nearest to (x, y).
    #[wasm_bindgen(js_name = addTangentLineNear)]
    pub fn add_tangent_line_near(&mut self, p: usize, c: usize, x: f64, y: f64) -> Result<Vec<u32>, JsError> {
        ops::tangent_line_near(&mut self.doc, p, c, [x, y]).map(|(l, t)| vec![l as u32, t as u32]).map_err(js)
    }

    /// Adds the mirror image of point p in line l.
    #[wasm_bindgen(js_name = addReflection)]
    pub fn add_reflection(&mut self, p: usize, l: usize) -> Result<usize, JsError> {
        ops::reflection(&mut self.doc, p, l).map_err(js)
    }

    /// Adds the circle through points a, b and c. Returns [circle, center],
    /// the center being a new point.
    #[wasm_bindgen(js_name = addCircleThrough)]
    pub fn add_circle_through(&mut self, a: usize, b: usize, c: usize) -> Result<Vec<u32>, JsError> {
        ops::circle_through(&mut self.doc, a, b, c).map(|(x, o)| vec![x as u32, o as u32]).map_err(js)
    }

    /// Adds the arc from point a through point m to point b. Returns [arc,
    /// center], the center being a new point.
    #[wasm_bindgen(js_name = addArcThrough)]
    pub fn add_arc_through(&mut self, a: usize, m: usize, b: usize) -> Result<Vec<u32>, JsError> {
        ops::arc_through(&mut self.doc, a, m, b).map(|(x, o)| vec![x as u32, o as u32]).map_err(js)
    }

    /// Adds the ellipse or hyperbola (`kind` "ellipse" or "hyperbola") with
    /// foci f1 and f2 through point p.
    #[wasm_bindgen(js_name = addConicByFoci)]
    pub fn add_conic_by_foci(&mut self, kind: &str, f1: usize, f2: usize, p: usize) -> Result<usize, JsError> {
        ops::conic_by_foci(&mut self.doc, ops::conic_kind_named(kind).map_err(js)?, f1, f2, p).map_err(js)
    }

    /// Adds the parabola with focus f and directrix line l.
    #[wasm_bindgen(js_name = addParabolaByFocus)]
    pub fn add_parabola_by_focus(&mut self, f: usize, l: usize) -> Result<usize, JsError> {
        ops::parabola_by_focus(&mut self.doc, f, l).map_err(js)
    }

    /// Adds the ellipse or hyperbola centered on point c, its first axis
    /// ending at (ax, ay) and its second half-axis reaching the line through
    /// (sx, sy) parallel to the first axis.
    #[wasm_bindgen(js_name = addConicFrom)]
    pub fn add_conic_from(&mut self, kind: &str, c: usize, ax: f64, ay: f64, sx: f64, sy: f64) -> Result<usize, JsError> {
        ops::conic_from(&mut self.doc, ops::conic_kind_named(kind).map_err(js)?, c, [ax, ay], [sx, sy]).map_err(js)
    }

    /// Adds the parabola with vertex at point c opening toward (x, y), its
    /// focal distance the distance to (x, y).
    #[wasm_bindgen(js_name = addParabolaToward)]
    pub fn add_parabola_toward(&mut self, c: usize, x: f64, y: f64) -> Result<usize, JsError> {
        ops::parabola_toward(&mut self.doc, c, [x, y]).map_err(js)
    }

    /// Adds the foci of conic c: two points, one for a parabola.
    #[wasm_bindgen(js_name = addFoci)]
    pub fn add_foci(&mut self, c: usize) -> Result<Vec<u32>, JsError> {
        ops::conic_feature(&mut self.doc, ConicFeature::Foci, c, [0.0; 2]).map(|v| v.into_iter().map(|e| e as u32).collect()).map_err(js)
    }

    /// Adds the directrix of conic c belonging to the focus nearest (x, y).
    #[wasm_bindgen(js_name = addDirectrix)]
    pub fn add_directrix(&mut self, c: usize, x: f64, y: f64) -> Result<usize, JsError> {
        ops::conic_feature(&mut self.doc, ConicFeature::Directrix, c, [x, y]).map_err(js)?.first().copied().ok_or_else(|| js("no directrix".into()))
    }

    /// Adds both asymptotes of hyperbola c.
    #[wasm_bindgen(js_name = addAsymptotes)]
    pub fn add_asymptotes(&mut self, c: usize) -> Result<Vec<u32>, JsError> {
        ops::conic_feature(&mut self.doc, ConicFeature::Asymptotes, c, [0.0; 2]).map(|v| v.into_iter().map(|e| e as u32).collect()).map_err(js)
    }

    /// Adds a constraint: `kind` is "coincident", "on_curve", "midpoint",
    /// "horizontal", "vertical", "parallel", "perpendicular", "collinear",
    /// "equal", "concentric" or "tangent", on entities a and b ("horizontal"
    /// and "vertical" take a line, or two points). False when it is
    /// rejected.
    pub fn constrain(&mut self, kind: &str, a: usize, b: Option<usize>) -> Result<bool, JsError> {
        ops::constrain(&mut self.doc, ConKind::from_name(kind).map_err(js)?, a, b.unwrap_or(usize::MAX)).map_err(js)
    }

    /// Like `constrain`; a tangency between two curves whose contact point
    /// is solved for starts its contact search at the point of the curves
    /// nearest to (x, y), such as where the user clicked.
    #[wasm_bindgen(js_name = constrainNear)]
    pub fn constrain_near(&mut self, kind: &str, a: usize, b: Option<usize>, x: f64, y: f64) -> Result<bool, JsError> {
        ops::constrain_near(&mut self.doc, ConKind::from_name(kind).map_err(js)?, a, b.unwrap_or(usize::MAX), [x, y]).map_err(js)
    }

    /// True when `constrain` would accept these arguments; the constraint
    /// might still be rejected for not solving. The document is unchanged.
    #[wasm_bindgen(js_name = canConstrain)]
    pub fn can_constrain(&self, kind: &str, a: usize, b: Option<usize>) -> Result<bool, JsError> {
        Ok(ops::can_constrain(&self.doc, ConKind::from_name(kind).map_err(js)?, a, b.unwrap_or(usize::MAX)))
    }

    /// Adds the constraints (kinds[k], a[k], b[k]), each as `constrain`
    /// takes it (b[k] is ignored where a kind takes one entity), as one
    /// undoable change, rejecting them together when any would not solve or
    /// is redundant. False when rejected.
    #[wasm_bindgen(js_name = addConstraints)]
    pub fn add_constraints(&mut self, kinds: Vec<String>, a: Vec<u32>, b: Vec<u32>) -> Result<bool, JsError> {
        if kinds.len() != a.len() || kinds.len() != b.len() {
            return Err(js("kinds, a and b differ in length".into()));
        }
        let cons = kinds.iter().zip(a.iter().zip(&b)).map(|(k, (a, b))| Ok((ConKind::from_name(k)?, *a as usize, *b as usize))).collect::<Result<Vec<_>, String>>().map_err(js)?;
        ops::add_constraints(&mut self.doc, &cons).map_err(js)
    }

    /// Constrains points a and b to be mirror images of each other in line
    /// `l`. False when rejected.
    #[wasm_bindgen(js_name = constrainSymmetric)]
    pub fn constrain_symmetric(&mut self, a: usize, b: usize, l: usize) -> Result<bool, JsError> {
        ops::symmetric(&mut self.doc, a, b, l).map_err(js)
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
        ops::add_dim(&mut self.doc, DimType::from_name(kind).map_err(js)?, a, b.unwrap_or(usize::MAX), off).map_err(js)
    }

    /// Sets dimension `i` from text as typed: a number (degrees
    /// for angles) or an expression of variables and dimensions drives it;
    /// empty text makes it a reference dimension. False when the value
    /// cannot be reached.
    #[wasm_bindgen(js_name = setDim)]
    pub fn set_dim(&mut self, i: usize, text: &str) -> Result<bool, JsError> {
        ops::set_dim(&mut self.doc, i, text).map_err(js)
    }

    /// Makes dimension `i` driving or reference. False when making it
    /// driving is rejected (the constraints already determine it).
    #[wasm_bindgen(js_name = setDimDriving)]
    pub fn set_dim_driving(&mut self, i: usize, driving: bool) -> Result<bool, JsError> {
        ops::set_driving(&mut self.doc, i, driving).map_err(js)
    }

    /// Marks dimension `i` numeric or not. Formulas derived for other targets
    /// use a numeric dimension's current value as a plain number. Solving and
    /// the dimension itself are unaffected.
    #[wasm_bindgen(js_name = setDimNumeric)]
    pub fn set_dim_numeric(&mut self, i: usize, numeric: bool) -> Result<bool, JsError> {
        ops::set_dim_numeric(&mut self.doc, i, numeric).map_err(js)
    }

    /// Moves the label of dimension `i` to (offX, offY) from the dimension's
    /// anchor (see `dimAnchor`), as one undoable change. For a reference
    /// angle, the sector holding the label selects the angle or its
    /// supplement; a driving angle keeps its choice.
    #[wasm_bindgen(js_name = setDimOffset)]
    pub fn set_dim_offset(&mut self, i: usize, off_x: f64, off_y: f64) -> Result<(), JsError> {
        ops::set_dim_offset(&mut self.doc, i, [off_x, off_y]).map_err(js)
    }

    /// Moves the label of dimension `i` like `setDimOffset`, as a drag: not
    /// an undoable change by itself (see `checkpoint`).
    #[wasm_bindgen(js_name = dragDimOffset)]
    pub fn drag_dim_offset(&mut self, i: usize, off_x: f64, off_y: f64) -> Result<(), JsError> {
        ops::drag_dim_offset(&mut self.doc, i, [off_x, off_y]).map_err(js)
    }

    /// Toggles angle dimension `i` between the angle of its lines and its
    /// supplement; a driving dimension with a plain value is set to the
    /// supplement's value. False when rejected.
    #[wasm_bindgen(js_name = toggleSupp)]
    pub fn toggle_supp(&mut self, i: usize) -> Result<bool, JsError> {
        ops::toggle_supp(&mut self.doc, i).map_err(js)
    }

    /// Adds a value variable. Without a name, the first unused of a, b, c, k,
    /// ... is taken. A free variable (default) is set by `value` (default
    /// 1), otherwise it is solved from the constraints, starting at `value`.
    /// Returns the variable.
    #[wasm_bindgen(js_name = addVar)]
    pub fn add_var(&mut self, name: Option<String>, value: Option<f64>, free: Option<bool>) -> Result<usize, JsError> {
        ops::add_var(&mut self.doc, name.as_deref(), value.unwrap_or(1.0), free.unwrap_or(true)).map_err(js)
    }

    /// Adds a variable defined by the expression `expr` of variables and
    /// dimensions. Without a name, f1, f2, ... is taken. Returns the variable.
    #[wasm_bindgen(js_name = addDefVar)]
    pub fn add_def_var(&mut self, name: Option<String>, expr: &str) -> Result<usize, JsError> {
        ops::add_defvar(&mut self.doc, name.as_deref(), expr).map_err(js)
    }

    /// Sets the value of value variable `i`. False when the value cannot be reached.
    #[wasm_bindgen(js_name = setVar)]
    pub fn set_var(&mut self, i: usize, value: f64) -> Result<bool, JsError> {
        ops::set_var(&mut self.doc, i, value).map_err(js)
    }

    /// Sets variable `i` from text as typed: a plain number makes it
    /// a free value variable, any other text an expression of variables and
    /// dimensions. False when rejected.
    #[wasm_bindgen(js_name = setVarExpr)]
    pub fn set_var_expr(&mut self, i: usize, text: &str) -> Result<bool, JsError> {
        ops::set_var_expr(&mut self.doc, i, text).map_err(js)
    }

    /// Checks expression text as `setVarExpr` and `setDim` read it: the
    /// reason when it does not parse or names an unknown quantity, undefined
    /// when it is valid.
    #[wasm_bindgen(js_name = checkExpr)]
    pub fn check_expr(&self, text: &str) -> Option<String> {
        ops::check_expr(&self.doc, text).err()
    }

    /// Checks a function curve definition as `addCurve` reads it: the reason
    /// when it does not parse, undefined when it is valid.
    #[wasm_bindgen(js_name = checkCurve)]
    pub fn check_curve(&self, src: &str) -> Option<String> {
        ops::check_curve(&self.doc, src).err()
    }

    /// Makes value variable `i` free (set by its value) or solved from the
    /// constraints. False when rejected.
    #[wasm_bindgen(js_name = setVarFree)]
    pub fn set_var_free(&mut self, i: usize, free: bool) -> Result<bool, JsError> {
        ops::set_var_free(&mut self.doc, i, free).map_err(js)
    }

    /// Marks variable `i` numeric or not. Formulas derived for other targets
    /// use a numeric variable's current value as a plain number. Solving and
    /// the variable itself (value, expression, free or solved) are
    /// unaffected.
    #[wasm_bindgen(js_name = setVarNumeric)]
    pub fn set_var_numeric(&mut self, i: usize, numeric: bool) -> Result<bool, JsError> {
        ops::set_var_numeric(&mut self.doc, i, numeric).map_err(js)
    }

    /// Removes variable `i`, leaving an empty slot; expressions naming it
    /// fail to parse until edited. False when rejected.
    #[wasm_bindgen(js_name = deleteVar)]
    pub fn delete_var(&mut self, i: usize) -> Result<bool, JsError> {
        ops::delete_var(&mut self.doc, i).map_err(js)
    }

    /// Deletes entity `e` and what depends on it (constraints and dimensions
    /// that reference it, and features built from it). False when the
    /// constraints would no longer solve.
    #[wasm_bindgen(js_name = deleteEnt)]
    pub fn delete_ent(&mut self, e: usize) -> Result<bool, JsError> {
        ops::delete_ent(&mut self.doc, e).map_err(js)
    }

    /// Deletes constraint `c`. False when the constraints would no longer
    /// solve.
    #[wasm_bindgen(js_name = deleteCon)]
    pub fn delete_con(&mut self, c: usize) -> Result<bool, JsError> {
        ops::delete_con(&mut self.doc, c).map_err(js)
    }

    /// Deletes dimension `i`; expressions naming it fail to parse until
    /// edited. False when the constraints would no longer solve.
    #[wasm_bindgen(js_name = deleteDim)]
    pub fn delete_dim(&mut self, i: usize) -> Result<bool, JsError> {
        ops::delete_dim(&mut self.doc, i).map_err(js)
    }

    /// Deletes the given entities, constraints, dimensions and variables as
    /// one undoable change, with everything that depends on the entities.
    /// False when the constraints would no longer solve; nothing is deleted.
    pub fn delete(&mut self, ents: Vec<u32>, cons: Vec<u32>, dims: Vec<u32>, vars: Vec<u32>) -> Result<bool, JsError> {
        ops::delete(&mut self.doc, &ids(&ents), &ids(&cons), &ids(&dims), &ids(&vars)).map_err(js)
    }

    /// Locks point `e` (its coordinates become fixed) or unlocks it.
    #[wasm_bindgen(js_name = lockPoint)]
    pub fn lock_point(&mut self, e: usize, on: bool) -> Result<bool, JsError> {
        ops::lock_point(&mut self.doc, e, on).map_err(js)
    }

    /// Locks or unlocks the points `pts` as one undoable change.
    #[wasm_bindgen(js_name = lockPoints)]
    pub fn lock_points(&mut self, pts: Vec<u32>, on: bool) -> Result<bool, JsError> {
        ops::lock_points(&mut self.doc, &ids(&pts), on).map_err(js)
    }

    /// True when point `e` is locked (both coordinates fixed).
    #[wasm_bindgen(js_name = pointLocked)]
    pub fn point_locked(&self, e: usize) -> Result<bool, JsError> {
        ops::point_locked(&self.doc, e).map_err(js)
    }

    /// Sets the construction flag of entity `e` (a dashed helper).
    #[wasm_bindgen(js_name = setConstruction)]
    pub fn set_construction(&mut self, e: usize, on: bool) -> Result<bool, JsError> {
        ops::set_construction(&mut self.doc, e, on).map_err(js)
    }

    /// Sets the construction flag of the non-point entities `ents` as one
    /// undoable change.
    #[wasm_bindgen(js_name = setConstructionAll)]
    pub fn set_construction_all(&mut self, ents: Vec<u32>, on: bool) -> Result<bool, JsError> {
        ops::set_construction_all(&mut self.doc, &ids(&ents), on).map_err(js)
    }

    /// Sets the hidden flag of entity `e`. Hidden entities are not drawn.
    #[wasm_bindgen(js_name = setHidden)]
    pub fn set_hidden(&mut self, e: usize, on: bool) -> Result<bool, JsError> {
        ops::set_hidden(&mut self.doc, e, on).map_err(js)
    }

    /// Makes line `e` infinite or a segment.
    #[wasm_bindgen(js_name = setInfinite)]
    pub fn set_infinite(&mut self, e: usize, on: bool) -> Result<bool, JsError> {
        ops::set_infinite(&mut self.doc, e, on).map_err(js)
    }

    /// Switches each of `lines` between infinite and segment, as one undoable
    /// change.
    #[wasm_bindgen(js_name = toggleInfinite)]
    pub fn toggle_infinite(&mut self, lines: Vec<u32>) -> Result<bool, JsError> {
        ops::toggle_infinite(&mut self.doc, &ids(&lines)).map_err(js)
    }

    /// Pulls point `e` toward (x, y) by solving the constraints with it
    /// pinned there (a drag, not an undoable change). False when the
    /// configuration cannot reach the target.
    #[wasm_bindgen(js_name = dragPoint)]
    pub fn drag_point(&mut self, e: usize, x: f64, y: f64) -> Result<bool, JsError> {
        ops::drag_point(&mut self.doc, e, x, y).map_err(js)
    }

    /// Pulls the radius of circle `e` so its edge passes through (x, y).
    #[wasm_bindgen(js_name = dragCircle)]
    pub fn drag_circle(&mut self, e: usize, x: f64, y: f64) -> Result<bool, JsError> {
        ops::drag_circle_edge(&mut self.doc, e, x, y).map_err(js)
    }

    /// Pulls a handle of conic `e` (`which` 0 the first-axis end, 1 the
    /// second-axis end) toward (x, y).
    #[wasm_bindgen(js_name = dragConicHandle)]
    pub fn drag_conic_handle(&mut self, e: usize, which: u8, x: f64, y: f64) -> Result<bool, JsError> {
        ops::drag_conic_handle(&mut self.doc, e, which, x, y).map_err(js)
    }

    /// Pulls each point of `pts` toward its target (xy[2k], xy[2k+1])
    /// together, solving the constraints (such as the ends of a dragged
    /// line). A drag, not an undoable change. False when the points cannot
    /// reach their targets.
    #[wasm_bindgen(js_name = dragPoints)]
    pub fn drag_points(&mut self, pts: Vec<u32>, xy: Vec<f64>) -> Result<bool, JsError> {
        if xy.len() != 2 * pts.len() {
            return Err(js("xy holds two numbers per point".into()));
        }
        let targets: Vec<[f64; 2]> = xy.chunks(2).map(|c| [c[0], c[1]]).collect();
        ops::drag_points(&mut self.doc, &ids(&pts), &targets).map_err(js)
    }

    /// Pulls curve `e` so it passes through (x, y), solving the constraints.
    /// A drag, not an undoable change. False when the curve cannot reach the
    /// point.
    #[wasm_bindgen(js_name = dragCurve)]
    pub fn drag_curve(&mut self, e: usize, x: f64, y: f64) -> Result<bool, JsError> {
        ops::drag_curve(&mut self.doc, e, x, y).map_err(js)
    }

    /// Sets value variable `i` to `value` and solves, as a drag: not an
    /// undoable change by itself (see `checkpoint`). False when the
    /// constraints do not solve at that value; the variable keeps its value.
    #[wasm_bindgen(js_name = dragVar)]
    pub fn drag_var(&mut self, i: usize, value: f64) -> Result<bool, JsError> {
        ops::drag_var(&mut self.doc, i, value).map_err(js)
    }

    /// Gives the point, dimension or variable named `old` the name `new`,
    /// rewriting expressions that use it. False when `new` is not an
    /// identifier or is taken.
    pub fn rename(&mut self, old: &str, new: &str) -> Result<bool, JsError> {
        ops::rename(&mut self.doc, old, new).map_err(js)
    }

    // ------------------------------------------------------------ reading

    /// Number of dimension slots. Removed dimensions leave empty slots.
    #[wasm_bindgen(getter, js_name = dimCount)]
    pub fn dim_count(&self) -> usize {
        ops::dim_count(&self.doc)
    }

    /// Name of dimension `i`; undefined for an empty slot or a bad index.
    #[wasm_bindgen(js_name = dimName)]
    pub fn dim_name(&self, i: usize) -> Option<String> {
        ops::dim_name(&self.doc, i).ok()
    }

    /// Current measured value of dimension `i` (radians for angles); NaN
    /// for an empty slot or a bad index. A value that is 0 up to the
    /// rounding of the solved coordinates is exactly 0.
    #[wasm_bindgen(js_name = dimValue)]
    pub fn dim_value(&self, i: usize) -> f64 {
        ops::dim_value(&self.doc, i).unwrap_or(f64::NAN)
    }

    /// True when dimension `i` is driving, false when it is a reference dimension.
    #[wasm_bindgen(js_name = dimDriving)]
    pub fn dim_driving(&self, i: usize) -> Result<bool, JsError> {
        ops::dim_driving(&self.doc, i).map_err(js)
    }

    /// True when dimension `i` is numeric (substituted as a number in formulas).
    #[wasm_bindgen(js_name = dimNumeric)]
    pub fn dim_numeric(&self, i: usize) -> Result<bool, JsError> {
        ops::dim_numeric(&self.doc, i).map_err(js)
    }

    /// Offset of the label of dimension `i` from the dimension's anchor as
    /// [x, y]; undefined for an empty slot or a bad index.
    #[wasm_bindgen(js_name = dimOffset)]
    pub fn dim_offset(&self, i: usize) -> Option<Vec<JsValue>> {
        ops::dim_offset(&self.doc, i).ok().map(|p| p.iter().map(|x| JsValue::from_f64(*x)).collect())
    }

    /// World coordinates of the anchor of dimension `i` (the midpoint of a
    /// length, the vertex of an angle, the centre of a circle or conic) as
    /// [x, y]; undefined for an empty slot or a bad index. The label sits at
    /// `dimOffset` from here.
    #[wasm_bindgen(js_name = dimAnchor)]
    pub fn dim_anchor(&self, i: usize) -> Option<Vec<JsValue>> {
        ops::dim_anchor(&self.doc, i).ok().map(|p| p.iter().map(|x| JsValue::from_f64(*x)).collect())
    }

    /// World coordinates of the anchor a dimension of kind `kind` on entities
    /// a and b would have, as [x, y], for placing its label before adding it.
    #[wasm_bindgen(js_name = dimAnchorOf)]
    pub fn dim_anchor_of(&self, kind: &str, a: usize, b: Option<usize>) -> Result<Vec<JsValue>, JsError> {
        ops::dim_anchor_of(&self.doc, DimType::from_name(kind).map_err(js)?, a, b.unwrap_or(usize::MAX)).map(pair).map_err(js)
    }

    /// Expression driving dimension `i`: empty when a plain value drives it
    /// or it is a reference dimension; undefined for a bad index.
    #[wasm_bindgen(js_name = dimExpr)]
    pub fn dim_expr(&self, i: usize) -> Option<String> {
        ops::dim_expr(&self.doc, i).ok()
    }

    /// The value dimension `i` is set to (radians for angles): for a driving
    /// dimension the value it drives toward, which differs from `dimValue`
    /// while the constraints conflict; for a reference dimension its measured
    /// value. NaN for a bad index.
    #[wasm_bindgen(js_name = dimSetValue)]
    pub fn dim_set_value(&self, i: usize) -> f64 {
        ops::dim_set_value(&self.doc, i).unwrap_or(f64::NAN)
    }

    /// The entities dimension `i` measures, in the order `addDim` takes them.
    #[wasm_bindgen(js_name = dimEnts)]
    pub fn dim_ents(&self, i: usize) -> Result<Vec<u32>, JsError> {
        ops::dim_ents(&self.doc, i).map(ids_out).map_err(js)
    }

    /// True when angle dimension `i` measures the supplementary angle of its
    /// lines.
    #[wasm_bindgen(js_name = dimSupp)]
    pub fn dim_supp(&self, i: usize) -> Result<bool, JsError> {
        ops::dim_supp(&self.doc, i).map_err(js)
    }

    /// True when the last solve left dimension `i` unsatisfied.
    #[wasm_bindgen(js_name = dimConflict)]
    pub fn dim_conflict(&self, i: usize) -> Result<bool, JsError> {
        ops::dim_conflict(&self.doc, i).map_err(js)
    }

    /// Why dimension `i` could not be turned into an equation; undefined
    /// when it could.
    #[wasm_bindgen(js_name = dimError)]
    pub fn dim_error(&self, i: usize) -> Result<Option<String>, JsError> {
        ops::dim_error(&self.doc, i).map_err(js)
    }

    /// Number of variable slots. Removed variables leave empty slots.
    #[wasm_bindgen(getter, js_name = varCount)]
    pub fn var_count(&self) -> usize {
        ops::var_count(&self.doc)
    }

    /// Name of variable `i`; undefined for an empty slot or a bad index.
    #[wasm_bindgen(js_name = varName)]
    pub fn var_name(&self, i: usize) -> Option<String> {
        ops::var_name(&self.doc, i).ok()
    }

    /// Value of variable `i`; NaN for an empty slot or a bad index.
    #[wasm_bindgen(js_name = varValue)]
    pub fn var_value(&self, i: usize) -> f64 {
        ops::var_value(&self.doc, i).unwrap_or(f64::NAN)
    }

    /// Expression of variable `i` (empty for a value variable); undefined
    /// for an empty slot or a bad index.
    #[wasm_bindgen(js_name = varExpr)]
    pub fn var_expr(&self, i: usize) -> Option<String> {
        ops::var_expr(&self.doc, i).ok()
    }

    /// True when variable `i` is free, false when it is solved or defined by an expression.
    #[wasm_bindgen(js_name = varFree)]
    pub fn var_free(&self, i: usize) -> Result<bool, JsError> {
        ops::var_free(&self.doc, i).map_err(js)
    }

    /// True when variable `i` is numeric (substituted as a number in formulas).
    #[wasm_bindgen(js_name = varNumeric)]
    pub fn var_numeric(&self, i: usize) -> Result<bool, JsError> {
        ops::var_numeric(&self.doc, i).map_err(js)
    }

    // ------------------------------------------------------------ entities

    /// Number of entity slots. Deleted entities leave empty slots.
    #[wasm_bindgen(getter, js_name = entCount)]
    pub fn ent_count(&self) -> usize {
        ops::ent_count(&self.doc)
    }

    /// Kind of entity `e`: "point", "line", "circle", "arc", "conic" or
    /// "curve"; undefined for an empty slot or a bad index.
    #[wasm_bindgen(js_name = entKind)]
    pub fn ent_kind(&self, e: usize) -> Option<String> {
        ops::ent_kind(&self.doc, e).ok().map(|k| k.name().to_string())
    }

    /// Name of entity `e` (points are named); undefined for an empty slot, a
    /// bad index or an unnamed entity.
    #[wasm_bindgen(js_name = entName)]
    pub fn ent_name(&self, e: usize) -> Option<String> {
        ops::ent_name(&self.doc, e).ok()
    }

    /// World coordinates of point `e` as [x, y]; undefined for an empty slot
    /// or a non-point.
    #[wasm_bindgen(js_name = pointPos)]
    pub fn point_pos(&self, e: usize) -> Option<Vec<JsValue>> {
        ops::point_pos(&self.doc, e).ok().map(|p| p.iter().map(|x| JsValue::from_f64(*x)).collect())
    }

    /// Endpoints of line `e` as [a, b, infinite]; undefined for a non-line.
    #[wasm_bindgen(js_name = linePts)]
    pub fn line_pts(&self, e: usize) -> Option<Vec<JsValue>> {
        let (a, b, inf) = ops::line_pts(&self.doc, e).ok()?;
        Some(vec![JsValue::from_f64(a as f64), JsValue::from_f64(b as f64), JsValue::from_bool(inf)])
    }

    /// Center point and radius of circle or arc `e` as [center, radius];
    /// undefined for a non-circle.
    #[wasm_bindgen(js_name = circleVals)]
    pub fn circle_vals(&self, e: usize) -> Option<Vec<JsValue>> {
        let (c, r) = ops::circle_vals(&self.doc, e).ok()?;
        Some(vec![JsValue::from_f64(c as f64), JsValue::from_f64(r)])
    }

    /// Center and endpoints of arc `e` as [c, a, b]; undefined for a non-arc.
    #[wasm_bindgen(js_name = arcPts)]
    pub fn arc_pts(&self, e: usize) -> Option<Vec<JsValue>> {
        let (c, a, b) = ops::arc_pts(&self.doc, e).ok()?;
        Some(vec![JsValue::from_f64(c as f64), JsValue::from_f64(a as f64), JsValue::from_f64(b as f64)])
    }

    /// Kind, center and axis values of conic `e` as
    /// [kind, center, a, b, angle]; undefined for a non-conic.
    #[wasm_bindgen(js_name = conicVals)]
    pub fn conic_vals(&self, e: usize) -> Option<Vec<JsValue>> {
        let (k, c, a, b, t) = ops::conic_vals(&self.doc, e).ok()?;
        let name = match k {
            ops::ConicKind::Ellipse => "ellipse",
            ops::ConicKind::Hyperbola => "hyperbola",
            ops::ConicKind::Parabola => "parabola",
        };
        Some(vec![JsValue::from_str(name), JsValue::from_f64(c as f64), JsValue::from_f64(a), JsValue::from_f64(b), JsValue::from_f64(t)])
    }

    /// Source text of function curve `e`; undefined for a non-curve.
    #[wasm_bindgen(js_name = curveSrc)]
    pub fn curve_src(&self, e: usize) -> Option<String> {
        ops::curve_src(&self.doc, e).ok()
    }

    /// True when entity `e` is a construction (dashed helper).
    #[wasm_bindgen(js_name = entConstruction)]
    pub fn ent_construction(&self, e: usize) -> Result<bool, JsError> {
        ops::ent_construction(&self.doc, e).map_err(js)
    }

    /// True when entity `e` is hidden.
    #[wasm_bindgen(js_name = entHidden)]
    pub fn ent_hidden(&self, e: usize) -> Result<bool, JsError> {
        ops::ent_hidden(&self.doc, e).map_err(js)
    }

    /// True when the constraints determine entity `e` completely: nothing
    /// about its position or shape is left free.
    #[wasm_bindgen(js_name = entFixed)]
    pub fn ent_fixed(&self, e: usize) -> Result<bool, JsError> {
        ops::ent_fixed(&self.doc, e).map_err(js)
    }

    /// True when the last solve left entity `e` in conflict.
    #[wasm_bindgen(js_name = entConflict)]
    pub fn ent_conflict(&self, e: usize) -> Result<bool, JsError> {
        ops::ent_conflict(&self.doc, e).map_err(js)
    }

    /// True when line `e` is infinite.
    #[wasm_bindgen(js_name = entInfinite)]
    pub fn ent_infinite(&self, e: usize) -> Result<bool, JsError> {
        ops::ent_infinite(&self.doc, e).map_err(js)
    }

    /// Number of constraint slots. Deleted constraints leave empty slots.
    #[wasm_bindgen(getter, js_name = conCount)]
    pub fn con_count(&self) -> usize {
        ops::con_count(&self.doc)
    }

    /// Kind name of constraint `c` (e.g. "coincident", "parallel",
    /// "symmetric", ...); undefined for an empty slot or a bad index.
    #[wasm_bindgen(js_name = conKind)]
    pub fn con_kind(&self, c: usize) -> Option<String> {
        ops::con_kind(&self.doc, c).ok().map(|k| k.name().to_string())
    }

    /// The entities of constraint `c`, in the order they appear in it.
    #[wasm_bindgen(js_name = conEnts)]
    pub fn con_ents(&self, c: usize) -> Result<Vec<u32>, JsError> {
        ops::con_ents(&self.doc, c).map(|v| v.into_iter().map(|e| e as u32).collect()).map_err(js)
    }

    /// True when the last solve left constraint `c` unsatisfied.
    #[wasm_bindgen(js_name = conConflict)]
    pub fn con_conflict(&self, c: usize) -> Result<bool, JsError> {
        ops::con_conflict(&self.doc, c).map_err(js)
    }

    /// Why constraint `c` could not be turned into equations; undefined when
    /// it could.
    #[wasm_bindgen(js_name = conError)]
    pub fn con_error(&self, c: usize) -> Result<Option<String>, JsError> {
        ops::con_error(&self.doc, c).map_err(js)
    }

    /// True for an "offset" constraint whose offset is turned by 90° (the
    /// point of a perpendicular line).
    #[wasm_bindgen(js_name = conTurned)]
    pub fn con_turned(&self, c: usize) -> Result<bool, JsError> {
        ops::con_turned(&self.doc, c).map_err(js)
    }

    /// Contact point of tangency constraint `c` as [x, y, solved], where
    /// `solved` is true for a point the solver finds that no entity marks;
    /// undefined for other constraints.
    #[wasm_bindgen(js_name = conContact)]
    pub fn con_contact(&self, c: usize) -> Result<Option<Vec<JsValue>>, JsError> {
        Ok(ops::con_contact(&self.doc, c).map_err(js)?.map(|t| vec![JsValue::from_f64(t.at[0]), JsValue::from_f64(t.at[1]), JsValue::from_bool(t.solved)]))
    }

    /// Kind name of dimension `i` ("distance", "radius", "angle", ...).
    #[wasm_bindgen(js_name = dimKind)]
    pub fn dim_kind(&self, i: usize) -> Option<String> {
        ops::dim_kind(&self.doc, i).ok().map(|k| k.name().to_string())
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
        let polys = ops::ent_samples(&self.doc, e, [x0, y0], [x1, y1], px).map_err(js)?;
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
    /// beyond the default budget; `cancel` stops it early.
    #[wasm_bindgen(js_name = deriveDim)]
    pub fn derive_dim(&self, i: usize, extra_ms: Option<u32>, cancel: Option<Cancel>) -> Result<Formula, JsError> {
        self.derive(Target::Dim(i), extra_ms, cancel)
    }

    /// Closed-form formula of variable `i`. `extraMs` extends the search
    /// beyond the default budget; `cancel` stops it early.
    #[wasm_bindgen(js_name = deriveVar)]
    pub fn derive_var(&self, i: usize, extra_ms: Option<u32>, cancel: Option<Cancel>) -> Result<Formula, JsError> {
        self.derive(Target::Var(i), extra_ms, cancel)
    }
}

impl JsDoc {
    fn derive(&self, t: Target, extra_ms: Option<u32>, cancel: Option<Cancel>) -> Result<Formula, JsError> {
        let f = ops::derive(&self.doc, t, extra_ms.unwrap_or(0) as u64, cancel.as_ref().map(|c| &c.c)).map_err(js)?;
        Ok(Formula {
            name: f.name,
            plain: f.plain,
            latex: f.latex,
            latex_short: f.latex_short,
            defs: f.defs.into_iter().map(|(n, l)| Def { name: n, latex: l }).collect(),
            exact: f.exact,
            is_angle: f.is_angle,
            value: f.value,
            actual: f.actual,
            size: f.size,
        })
    }
}

/// A flag that stops a derivation early.
#[wasm_bindgen]
#[derive(Clone)]
pub struct Cancel {
    c: ops::Cancel,
}

#[wasm_bindgen]
impl Cancel {
    /// New, unset flag.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Cancel {
        Cancel { c: ops::new_cancel() }
    }

    /// Sets the flag: a derivation given it stops soon with an error.
    pub fn cancel(&self) {
        ops::cancel(&self.c);
    }
}

/// LaTeX text, such as the `latex` or `latexShort` of a `Formula`, typeset
/// in display style as the bytes of a PNG image: glyphs of color `rgb`
/// (0xRRGGBB) on a transparent background, at `sizePt` points and `dpi` dots
/// per inch (a point is 1/72 inch, so 12 pt at 96 dpi puts 16 pixels to the
/// em).
#[wasm_bindgen(js_name = latexPng)]
pub fn latex_png(latex: &str, size_pt: f64, dpi: f64, rgb: u32) -> Result<Vec<u8>, JsError> {
    let [_, r, g, b] = rgb.to_be_bytes();
    ops::latex_png(latex, size_pt, dpi, [r, g, b]).map_err(js)
}

/// Version of the library.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").into()
}
