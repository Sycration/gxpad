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
use crate::ops::{self, ConKind, DimType};
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

    /// Sets dimension `i` from text as typed in gxpad: a number (degrees
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
