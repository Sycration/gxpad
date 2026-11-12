//! Programmatic construction building and formula verification, for tests.
//!
//! A test builds a construction with [`Kit`], adds driving dimensions and
//! reference dimensions, and calls [`Kit::check`] on a reference dimension.
//! `check` derives the formula and verifies that it
//!
//! 1. is exact (no numeric fallback), unless `allow_inexact` is set;
//! 2. depends only on named quantities (dimensions and variables), unless
//!    `allow_free_symbols` is set, so the checked dimension really is
//!    determined by the construction;
//! 3. evaluates to the measured value;
//! 4. keeps evaluating to the measured value while the construction is
//!    articulated: free points are dragged and driving dimensions and free
//!    variables are perturbed, then the same formula is re-evaluated.
//!
//! When a `Kit` is dropped at the end of a test, its construction is saved
//! as an example file `test_examples/<test file>/<test name>.gxp` (the app's
//! own file format), overwriting the previous version, so every test run
//! regenerates the examples.

#![allow(dead_code)]

use gxgeo::build::angle_vertex;
use gxgeo::derive;
use gxgeo::expr::bx::{plain, to_bx};
use gxgeo::expr::{Expr, eval, sym_set};
use gxgeo::formula::{Target, numeric_doc, sym_name, target_expr};
use gxgeo::model::*;
use gxgeo::ops::{self, ConicFeature, Document};
use gxgeo::sketch::{Sketch, contact_guess, sample_polys};
use std::collections::HashSet;

pub struct Kit {
    /// The construction.
    pub geo: Document,
    /// Where the contact search of a tangency between two curves starts
    /// (see `ops::constrain_near`).
    pub hint: [f64; 2],
    rng: u64,
    /// Accept formulas where some block was solved numerically.
    pub allow_inexact: bool,
    /// Accept formulas containing undetermined coordinates (x_A, …).
    pub allow_free_symbols: bool,
    /// Random articulation trials per check.
    pub trials: usize,
    /// Relative size of point drags during articulation.
    pub drag_scale: f64,
    /// Relative size of driving value perturbations.
    pub value_scale: f64,
    /// Upper bound for one derivation, in milliseconds of real time. The
    /// derivation's own budgets are counted in work done, so this is only a
    /// guard against a runaway computation; it is generous because real
    /// time under parallel test load is several times the work time.
    pub max_ms: f64,
}

/// Folder for the example files written by finished tests.
pub const EXAMPLES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/test_examples");

thread_local! {
    /// Kits saved so far by the test on this thread (a test can build several).
    static SAVED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The example file for the current test: the test binary's name without
/// its hash as the folder, the test's name as the file.
fn example_path() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let stem = exe.file_stem()?.to_str()?;
    let group = match stem.rsplit_once('-') {
        Some((name, hash)) if hash.chars().all(|c| c.is_ascii_hexdigit()) => name,
        _ => stem,
    };
    let thread = std::thread::current();
    let test = match thread.name() {
        Some(n) if n != "main" => n.replace("::", "__"),
        _ => "construction".to_string(),
    };
    let k = SAVED.with(|c| {
        c.set(c.get() + 1);
        c.get()
    });
    let file = if k == 1 { format!("{test}.gxp") } else { format!("{test}_{k}.gxp") };
    Some(std::path::Path::new(EXAMPLES_DIR).join(group).join(file))
}

impl Drop for Kit {
    fn drop(&mut self) {
        let Some(path) = example_path() else { return };
        let Ok(text) = gxgeo::gxp::to_string_pretty(&self.sk().doc) else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(&path, text) {
            eprintln!("could not write example {}: {e}", path.display());
        }
    }
}

impl Default for Kit {
    fn default() -> Self {
        Self::new()
    }
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * (1.0 + a.abs().max(b.abs()))
}

impl Kit {
    pub fn new() -> Kit {
        // the app's own budgets: they are counted in work done, not in
        // real time, so parallel test load does not change the results
        derive::set_time_scale(1.0);
        Kit {
            geo: Document::new(),
            hint: [0.0; 2],
            rng: 0x9E37_79B9_7F4A_7C15,
            allow_inexact: false,
            allow_free_symbols: false,
            trials: 12,
            drag_scale: 0.04,
            value_scale: 0.03,
            max_ms: 30000.0,
        }
    }

    fn sk(&self) -> &Sketch {
        self.geo.sketch()
    }
    fn skm(&mut self) -> &mut Sketch {
        self.geo.sketch_mut()
    }
    /// Adds `c`; a tangency whose contact is solved for starts its search
    /// near `hint`.
    fn add_con(&mut self, c: Con) -> bool {
        let hint = self.hint;
        self.skm().add_constraint_with(c, |d, a, b| contact_guess(&sample_polys(d, a), &sample_polys(d, b), hint))
    }
    /// Deletes entities with everything that depends on them; false when the
    /// constraints would no longer solve.
    pub fn delete(&mut self, ents: &[usize]) -> bool {
        ops::delete(&mut self.geo, ents, &[], &[], &[]).unwrap_or(false)
    }

    pub fn doc(&self) -> &Doc {
        &self.sk().doc
    }
    pub fn origin(&self) -> usize {
        self.sk().doc.origin
    }
    pub fn xaxis(&self) -> usize {
        self.sk().doc.xaxis
    }
    pub fn yaxis(&self) -> usize {
        self.sk().doc.yaxis
    }

    fn rand(&mut self) -> f64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 11) as f64 / (1u64 << 53) as f64
    }

    fn change(&mut self, what: &str, strict: bool, f: impl FnOnce(&mut Doc) -> usize) -> usize {
        let mut out = usize::MAX;
        let ok = self.skm().try_change(strict, |d| out = f(d));
        assert!(ok, "rejected (unsatisfiable or redundant): {what}");
        out
    }

    // ------------------------------------------------------------ entities

    /// A free point at an initial position.
    pub fn point(&mut self, x: f64, y: f64) -> usize {
        self.change("point", false, |d| d.add_point(x, y))
    }
    /// A point locked in place.
    pub fn fixed_point(&mut self, x: f64, y: f64) -> usize {
        self.change("fixed point", false, |d| {
            let p = d.add_point(x, y);
            d.lock_point(p, true);
            p
        })
    }
    pub fn line(&mut self, a: usize, b: usize) -> usize {
        ops::add_line(&mut self.geo, a, b, false).expect("line")
    }
    pub fn inf_line(&mut self, a: usize, b: usize) -> usize {
        ops::add_line(&mut self.geo, a, b, true).expect("infinite line")
    }
    /// Circle with center point `c` and initial radius `r` (radius is free).
    pub fn circle(&mut self, c: usize, r: f64) -> usize {
        ops::add_circle(&mut self.geo, c, r).expect("circle")
    }
    /// Arc around `c` from point `a` counter-clockwise to point `b`
    /// (|ca| = |cb| is enforced).
    pub fn arc(&mut self, c: usize, a: usize, b: usize) -> usize {
        ops::add_arc(&mut self.geo, c, a, b).expect("arc")
    }
    /// Ellipse with center point, semi-axes and rotation angle (all free).
    pub fn ellipse(&mut self, c: usize, a: f64, b: f64, angle: f64) -> usize {
        ops::add_conic(&mut self.geo, ConicKind::Ellipse, c, a, b, angle).expect("ellipse")
    }
    pub fn hyperbola(&mut self, c: usize, a: f64, b: f64, angle: f64) -> usize {
        ops::add_conic(&mut self.geo, ConicKind::Hyperbola, c, a, b, angle).expect("hyperbola")
    }
    /// Parabola with vertex point, focal length and axis angle.
    pub fn parabola(&mut self, vertex: usize, focal: f64, angle: f64) -> usize {
        ops::add_conic(&mut self.geo, ConicKind::Parabola, vertex, focal, 1.0, angle).expect("parabola")
    }
    /// The rotation-angle parameter of a conic, for locking.
    pub fn lock_conic_angle(&mut self, c: usize) {
        let t = match self.sk().doc.kind(c) {
            Some(Ent::Conic { t, .. }) => *t,
            _ => panic!("not a conic"),
        };
        self.change("lock conic angle", false, |d| {
            d.params[t].locked = true;
            0
        });
    }
    /// Explicit `y = f(x)` or implicit `F(x, y) = G(x, y)` curve.
    pub fn curve(&mut self, src: &str) -> usize {
        assert!(self.sk().doc.curve_def(src).is_ok(), "bad curve {src}");
        ops::add_curve(&mut self.geo, src).expect("curve")
    }

    // ------------------------------------------------------------ variables

    /// Free (driving) variable.
    pub fn var(&mut self, name: &str, value: f64) -> usize {
        ops::add_var(&mut self.geo, Some(name), value, true).expect("variable")
    }
    /// Variable solved from the constraints, starting at `guess`.
    pub fn solved_var(&mut self, name: &str, guess: f64) -> usize {
        ops::add_var(&mut self.geo, Some(name), guess, false).expect("solved variable")
    }
    /// Variable defined by an expression (may use dimension and variable names).
    pub fn defvar(&mut self, name: &str, expr: &str) -> usize {
        ops::add_defvar(&mut self.geo, Some(name), expr).expect("defined variable")
    }
    pub fn set_var(&mut self, v: usize, value: f64) {
        assert!(ops::set_var(&mut self.geo, v, value).unwrap(), "could not set variable");
    }

    // ------------------------------------------------------------ constraints

    /// Adds a constraint; panics when it is unsatisfiable or redundant.
    pub fn constrain(&mut self, c: Con) {
        assert!(self.add_con(c.clone()), "rejected constraint {c:?}");
    }
    /// Adds a constraint, returning whether it was accepted.
    pub fn try_constrain(&mut self, c: Con) -> bool {
        self.add_con(c)
    }
    pub fn coincident(&mut self, a: usize, b: usize) {
        self.constrain(Con::Coincident(a, b))
    }
    /// Point `p` lies on curve `c` (line, circle, arc, conic or curve).
    pub fn on(&mut self, p: usize, c: usize) {
        self.constrain(Con::OnCurve(p, c))
    }
    pub fn midpoint(&mut self, p: usize, line: usize) {
        self.constrain(Con::Midpoint(p, line))
    }
    pub fn horizontal(&mut self, line: usize) {
        let (a, b) = self.sk().doc.line_pts(line);
        self.constrain(Con::Horizontal(a, b))
    }
    pub fn vertical(&mut self, line: usize) {
        let (a, b) = self.sk().doc.line_pts(line);
        self.constrain(Con::Vertical(a, b))
    }
    pub fn parallel(&mut self, a: usize, b: usize) {
        self.constrain(Con::Parallel(a, b))
    }
    pub fn perpendicular(&mut self, a: usize, b: usize) {
        self.constrain(Con::Perpendicular(a, b))
    }
    pub fn collinear(&mut self, a: usize, b: usize) {
        self.constrain(Con::Collinear(a, b))
    }
    /// Equal lengths (lines), radii (circles/arcs), or length = radius.
    pub fn equal(&mut self, a: usize, b: usize) {
        self.constrain(Con::Equal(a, b))
    }
    pub fn concentric(&mut self, a: usize, b: usize) {
        self.constrain(Con::Concentric(a, b))
    }
    pub fn symmetric(&mut self, p: usize, q: usize, line: usize) {
        self.constrain(Con::Symmetric(p, q, line))
    }
    /// Tangency between any two curves (side chosen from the current layout).
    pub fn tangent(&mut self, a: usize, b: usize) {
        let c = self.sk().doc.tangent_con(a, b).expect("cannot be tangent");
        self.constrain(c)
    }
    pub fn lock(&mut self, p: usize) {
        self.change("lock", false, |d| {
            d.lock_point(p, true);
            0
        });
    }

    // ------------------------------------------------------------ construction tools

    pub fn midpoint_of(&mut self, a: usize, b: usize) -> usize {
        ops::midpoint(&mut self.geo, a, b).expect("midpoint")
    }
    pub fn midpoint_of_line(&mut self, l: usize) -> usize {
        ops::line_midpoint(&mut self.geo, l).expect("midpoint of line")
    }
    /// Intersection point of two curves near `guess`.
    pub fn intersect(&mut self, a: usize, b: usize, guess: [f64; 2]) -> usize {
        ops::intersection(&mut self.geo, a, b, guess).expect("intersection")
    }
    /// Infinite line through `p` perpendicular to line `l`.
    pub fn perp_line(&mut self, l: usize, p: usize) -> usize {
        ops::dir_line(&mut self.geo, l, p, true).expect("perpendicular line")
    }
    /// Infinite line through `p` parallel to line `l`.
    pub fn par_line(&mut self, l: usize, p: usize) -> usize {
        ops::dir_line(&mut self.geo, l, p, false).expect("parallel line")
    }
    /// Perpendicular bisector of points a, b: (infinite line, midpoint).
    pub fn perp_bisector(&mut self, a: usize, b: usize) -> (usize, usize) {
        ops::perp_bisector(&mut self.geo, a, b).expect("perpendicular bisector")
    }
    /// Angle bisector of lines l1, l2 in the sector containing `toward`:
    /// (infinite line, vertex point).
    pub fn angle_bisector(&mut self, l1: usize, l2: usize, toward: [f64; 2]) -> (usize, usize) {
        ops::angle_bisector(&mut self.geo, l1, l2, toward).expect("angle bisector")
    }
    /// Segment from `p` tangent to curve `c`, touching near `guess`:
    /// (segment, contact point).
    pub fn tangent_line(&mut self, p: usize, c: usize, guess: [f64; 2]) -> (usize, usize) {
        ops::tangent_line(&mut self.geo, p, c, guess).expect("tangent line")
    }
    /// Circle through points a, b, c: (circle, center point).
    pub fn circle_through(&mut self, a: usize, b: usize, c: usize) -> (usize, usize) {
        // strict, unlike the tool: a redundant construction is a test error
        let mut ctr = 0;
        let circ = self.change("circle through three points", true, |d| {
            let (circ, o) = d.build_circle_through(a, b, c).expect("collinear points");
            ctr = o;
            circ
        });
        (circ, ctr)
    }
    /// Arc from a through m to b: (arc, center point).
    pub fn arc_through(&mut self, a: usize, m: usize, b: usize) -> (usize, usize) {
        let mut ctr = 0;
        let arc = self.change("arc through three points", true, |d| {
            let (arc, o) = d.build_arc_through(a, m, b).expect("collinear points");
            ctr = o;
            arc
        });
        (arc, ctr)
    }
    /// Ellipse with foci f1, f2 through point p.
    pub fn ellipse_by_foci(&mut self, f1: usize, f2: usize, p: usize) -> usize {
        self.change("ellipse by foci", true, |d| d.build_conic_by_foci(ConicKind::Ellipse, f1, f2, p).expect("no such ellipse"))
    }
    /// Hyperbola with foci f1, f2 through point p.
    pub fn hyperbola_by_foci(&mut self, f1: usize, f2: usize, p: usize) -> usize {
        self.change("hyperbola by foci", true, |d| d.build_conic_by_foci(ConicKind::Hyperbola, f1, f2, p).expect("no such hyperbola"))
    }
    /// Parabola with focus f and directrix line l.
    pub fn parabola_by_focus(&mut self, f: usize, l: usize) -> usize {
        self.change("parabola by focus and directrix", true, |d| d.build_parabola_by_focus(f, l).expect("focus on the directrix"))
    }
    /// The foci of conic `c` (two points, or one for a parabola).
    pub fn foci(&mut self, c: usize) -> Vec<usize> {
        ops::conic_feature(&mut self.geo, ConicFeature::Foci, c, [0.0; 2]).expect("foci")
    }
    /// The directrix of conic `c` belonging to the focus nearest `near`.
    pub fn directrix(&mut self, c: usize, near: [f64; 2]) -> usize {
        ops::conic_feature(&mut self.geo, ConicFeature::Directrix, c, near).expect("directrix")[0]
    }
    /// Both asymptotes of hyperbola `c`.
    pub fn asymptotes(&mut self, c: usize) -> (usize, usize) {
        let v = ops::conic_feature(&mut self.geo, ConicFeature::Asymptotes, c, [0.0; 2]).expect("asymptotes");
        assert_eq!(v.len(), 2, "entity {c} has no asymptotes");
        (v[0], v[1])
    }
    /// Mirror image of `p` in line `l`.
    pub fn reflect(&mut self, p: usize, l: usize) -> usize {
        ops::reflection(&mut self.geo, p, l).expect("reflection")
    }

    // ------------------------------------------------------------ dimensions

    fn label_for(&self, kind: &DimKind) -> [f64; 2] {
        let a = self.sk().doc.dim_anchor(kind);
        [a[0] + 0.3, a[1] + 0.3]
    }

    /// Adds a driving dimension and sets its value (a number — degrees for
    /// angles — or an expression of variables/dimensions). Panics when the
    /// dimension is redundant or the value cannot be reached.
    pub fn drive(&mut self, kind: DimKind, value: &str) -> usize {
        let lw = self.label_for(&kind);
        let i = self.skm().add_dimension(kind, lw).expect("dimension rejected");
        let dim = self.sk().doc.dims[i].as_ref().unwrap();
        assert!(dim.driving, "dimension {kind:?} is already determined (redundant); use measure()");
        self.set(i, value);
        i
    }

    /// Adds a reference (measured) dimension.
    pub fn measure(&mut self, kind: DimKind) -> usize {
        let lw = self.label_for(&kind);
        let i = self.skm().add_dimension(kind, lw).expect("dimension rejected");
        if self.sk().doc.dims[i].as_ref().unwrap().driving {
            self.skm().toggle_driving(i);
        }
        assert!(!self.sk().doc.dims[i].as_ref().unwrap().driving);
        i
    }

    /// Label position inside the angle between the directions of l1 and l2
    /// (from their first to second points), so the dimension measures the
    /// angle between those directions rather than its supplement.
    fn angle_label(&self, l1: usize, l2: usize) -> [f64; 2] {
        let d = &self.sk().doc;
        let x = angle_vertex(d, l1, l2);
        let dir = |l: usize| {
            let (a, b) = d.line_pts(l);
            let (a, b) = (d.pval(a), d.pval(b));
            let v = [b[0] - a[0], b[1] - a[1]];
            let n = v[0].hypot(v[1]).max(1e-12);
            [v[0] / n, v[1] / n]
        };
        let (u, w) = (dir(l1), dir(l2));
        let mut m = [u[0] + w[0], u[1] + w[1]];
        if m[0].hypot(m[1]) < 1e-9 {
            m = [-u[1], u[0]];
        }
        let n = m[0].hypot(m[1]);
        [x[0] + 0.5 * m[0] / n, x[1] + 0.5 * m[1] / n]
    }

    /// Drives the angle between the directions of lines l1 and l2 (degrees,
    /// or an expression in radians).
    pub fn drive_angle(&mut self, l1: usize, l2: usize, value: &str) -> usize {
        let lw = self.angle_label(l1, l2);
        let i = self.skm().add_dimension(DimKind::Angle(l1, l2), lw).expect("dimension rejected");
        assert!(self.sk().doc.dims[i].as_ref().unwrap().driving, "angle is already determined; use measure_angle()");
        self.set(i, value);
        i
    }

    /// Measures the angle between the directions of lines l1 and l2.
    pub fn measure_angle(&mut self, l1: usize, l2: usize) -> usize {
        let lw = self.angle_label(l1, l2);
        let i = self.skm().add_dimension(DimKind::Angle(l1, l2), lw).expect("dimension rejected");
        if self.sk().doc.dims[i].as_ref().unwrap().driving {
            self.skm().toggle_driving(i);
        }
        i
    }

    /// Sets a driving dimension's value text.
    pub fn set(&mut self, d: usize, value: &str) {
        self.skm().set_dim_text(d, value);
        let dim = self.sk().doc.dims[d].as_ref().unwrap();
        assert!(dim.driving, "dimension {} is not driving", dim.name);
        if let Some(v) = gxgeo::expr::parse::plain_number(value) {
            let v = if dim.kind.is_angle() { v.to_radians() } else { v };
            let m = self.sk().doc.measure_value(d);
            assert!(close(m, v, 1e-7), "value {value} could not be reached for {} (measures {m})", dim.name);
        } else {
            assert_eq!(dim.expr, value.trim(), "expression {value} rejected for {}", dim.name);
        }
    }

    /// Measured value of a dimension (radians for angles).
    pub fn value(&self, d: usize) -> f64 {
        self.sk().doc.measure_value(d)
    }

    /// Remaining degrees of freedom of the whole construction.
    pub fn dof(&self) -> usize {
        self.sk().dof
    }

    // ------------------------------------------------------------ checking

    fn formula_text(&self, e: &Expr) -> String {
        let names = self.sk().doc.param_names();
        plain(&to_bx(e, &|s| sym_name(&names, s)))
    }

    fn named_symbols(&self) -> HashSet<u32> {
        let d = &self.sk().doc;
        d.vars.iter().flatten().map(|v| v.param as u32).chain(d.dims.iter().flatten().map(|x| x.param as u32)).collect()
    }

    /// Derives the formula of dimension `d` and verifies it (see module docs).
    /// Returns the formula as text.
    pub fn check(&mut self, d: usize) -> String {
        assert!(self.sk().conflicts.is_empty(), "construction has violated constraints: {:?}", self.sk().conflicts);
        assert!(self.sk().errors.is_empty(), "construction has errors: {:?}", self.sk().errors);
        let doc = numeric_doc(&self.sk().doc, Target::Dim(d));
        let (name, target, _) = target_expr(&doc, Target::Dim(d)).expect("bad target");
        let t0 = std::time::Instant::now();
        let der = derive::derive(&doc, &target).unwrap_or_else(|e| panic!("derivation of {name} failed: {e}"));
        let ms = t0.elapsed().as_secs_f64() * 1e3;
        let text = format!("{name} = {}", self.formula_text(&der.expr));
        println!("[{}] {text}   [{ms:.1} ms, dof {}]", std::thread::current().name().unwrap_or("?"), self.sk().dof);
        assert!(ms <= self.max_ms, "derivation took {ms:.0} ms: {text}");
        assert!(der.exact || self.allow_inexact, "formula is not exact: {text}");
        assert!(
            close(der.value, der.actual, 1e-8),
            "formula value {} differs from measured {}: {text}",
            der.value,
            der.actual
        );
        if !self.allow_free_symbols {
            let named = self.named_symbols();
            let names = self.sk().doc.param_names();
            let free: Vec<String> = sym_set(&der.expr).into_iter().filter(|s| !named.contains(s)).map(|s| sym_name(&names, s)).collect();
            assert!(free.is_empty(), "formula depends on undetermined quantities {free:?}: {text}");
        }
        self.articulate(d, &der.expr, &text);
        text
    }

    /// Like [`check`], additionally comparing the measured value with `expected`.
    pub fn check_value(&mut self, d: usize, expected: f64) -> String {
        let v = self.value(d);
        assert!(close(v, expected, 1e-8), "measured {v}, expected {expected}");
        self.check(d)
    }

    fn eval_now(&self, e: &Expr) -> f64 {
        let p = &self.sk().doc.params;
        eval(e, &|s| p.get(s as usize).map(|x| x.v).unwrap_or(f64::NAN))
    }

    fn scale(&self) -> f64 {
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        for (i, e) in self.sk().doc.ents.iter().enumerate() {
            if let Some(Entity { kind: Ent::Point { .. }, hidden: false, .. }) = e {
                let p = self.sk().doc.pval(i);
                for k in 0..2 {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
        }
        ((hi[0] - lo[0]).hypot(hi[1] - lo[1])).max(1.0)
    }

    fn articulate(&mut self, d: usize, formula: &Expr, text: &str) {
        let free_pts: Vec<usize> = (0..self.sk().doc.ents.len())
            .filter(|&i| match self.sk().doc.ent(i) {
                Some(Entity { kind: Ent::Point { x, y }, hidden: false, .. }) => {
                    self.sk().free_params.contains(x) || self.sk().free_params.contains(y)
                }
                _ => false,
            })
            .collect();
        let mut knobs: Vec<(usize, String)> = Vec::new();
        for x in self.sk().doc.dims.iter().flatten() {
            // zero-valued driving dimensions are structural, not parameters;
            // numeric dimensions enter formulas as constants
            if x.driving && !x.numeric && x.expr.is_empty() && self.sk().doc.params[x.param].v != 0.0 {
                knobs.push((x.param, x.name.clone()));
            }
        }
        for v in self.sk().doc.vars.iter().flatten() {
            if v.expr.is_empty() && !v.numeric && self.sk().doc.params[v.param].locked {
                knobs.push((v.param, v.name.clone()));
            }
        }
        let scale = self.scale();
        for _ in 0..self.trials {
            let before = self.sk().doc.clone();
            let use_drag = !free_pts.is_empty() && (knobs.is_empty() || self.rand() < 0.55);
            let action = if use_drag {
                let e = free_pts[(self.rand() * free_pts.len() as f64) as usize % free_pts.len()];
                let p0 = self.sk().doc.pval(e);
                let ang = self.rand() * std::f64::consts::TAU;
                let r = scale * self.drag_scale * (0.3 + self.rand());
                for k in 1..=5 {
                    let f = k as f64 / 5.0;
                    ops::drag_point(&mut self.geo, e, p0[0] + r * ang.cos() * f, p0[1] + r * ang.sin() * f).ok();
                }
                let ok = self.skm().solve_full();
                (ok, format!("dragged {} by {:.3} toward {:.0}°", self.sk().doc.ent(e).map(|x| x.name.clone()).unwrap_or_default(), r, ang.to_degrees()))
            } else if !knobs.is_empty() {
                let (p, name) = knobs[(self.rand() * knobs.len() as f64) as usize % knobs.len()].clone();
                let v0 = self.sk().doc.params[p].v;
                let f = 1.0 + self.value_scale * (2.0 * self.rand() - 1.0);
                let nv = if v0.abs() < 1e-9 { self.value_scale * (2.0 * self.rand() - 1.0) } else { v0 * f };
                let ok = self.skm().set_values(&[(p, nv)], false);
                (ok, format!("set {name} from {v0} to {nv}"))
            } else {
                break;
            };
            if !action.0 || !self.sk().conflicts.is_empty() {
                self.skm().doc = before;
                self.skm().dirty = true;
                self.skm().solve_full();
                continue;
            }
            let m = self.sk().doc.measure_value(d);
            let f = self.eval_now(formula);
            assert!(
                close(m, f, 1e-7),
                "after articulation ({}) the formula gives {f} but the measurement is {m}\n  {text}",
                action.1
            );
            self.skm().doc = before;
            self.skm().dirty = true;
            self.skm().solve_full();
        }
    }
}
