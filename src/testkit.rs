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

use crate::app::*;
use crate::derive;
use crate::expr::bx::{plain, to_bx};
use crate::expr::{Expr, eval, sym_set};
use crate::model::*;
use std::collections::HashSet;

pub struct Kit {
    pub app: App,
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
        let Ok(text) = crate::gxp::to_string_pretty(&self.app.geo.doc) else { return };
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
            app: App::new(),
            rng: 0x9E37_79B9_7F4A_7C15,
            allow_inexact: false,
            allow_free_symbols: false,
            trials: 12,
            drag_scale: 0.04,
            value_scale: 0.03,
            max_ms: 30000.0,
        }
    }

    pub fn doc(&self) -> &Doc {
        &self.app.geo.doc
    }
    pub fn origin(&self) -> usize {
        self.app.geo.doc.origin
    }
    pub fn xaxis(&self) -> usize {
        self.app.geo.doc.xaxis
    }
    pub fn yaxis(&self) -> usize {
        self.app.geo.doc.yaxis
    }

    fn rand(&mut self) -> f64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 11) as f64 / (1u64 << 53) as f64
    }

    fn change(&mut self, what: &str, strict: bool, f: impl FnOnce(&mut Doc) -> usize) -> usize {
        let mut out = usize::MAX;
        let ok = self.app.geo.try_change(strict, |d| out = f(d));
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
        self.change("line", false, |d| d.add_ent(Ent::Line { a, b, infinite: false }))
    }
    pub fn inf_line(&mut self, a: usize, b: usize) -> usize {
        self.change("infinite line", false, |d| d.add_ent(Ent::Line { a, b, infinite: true }))
    }
    /// Circle with center point `c` and initial radius `r` (radius is free).
    pub fn circle(&mut self, c: usize, r: f64) -> usize {
        self.change("circle", false, |d| d.add_circle(c, r))
    }
    /// Arc around `c` from point `a` counter-clockwise to point `b`
    /// (|ca| = |cb| is enforced).
    pub fn arc(&mut self, c: usize, a: usize, b: usize) -> usize {
        self.change("arc", false, |d| d.add_ent(Ent::Arc { c, a, b }))
    }
    /// Ellipse with center point, semi-axes and rotation angle (all free).
    pub fn ellipse(&mut self, c: usize, a: f64, b: f64, angle: f64) -> usize {
        self.change("ellipse", false, |d| d.add_conic(ConicKind::Ellipse, c, a, b, angle))
    }
    pub fn hyperbola(&mut self, c: usize, a: f64, b: f64, angle: f64) -> usize {
        self.change("hyperbola", false, |d| d.add_conic(ConicKind::Hyperbola, c, a, b, angle))
    }
    /// Parabola with vertex point, focal length and axis angle.
    pub fn parabola(&mut self, vertex: usize, focal: f64, angle: f64) -> usize {
        self.change("parabola", false, |d| d.add_conic(ConicKind::Parabola, vertex, focal, 1.0, angle))
    }
    /// The rotation-angle parameter of a conic, for locking.
    pub fn lock_conic_angle(&mut self, c: usize) {
        let t = match self.app.geo.doc.kind(c) {
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
        assert!(self.app.geo.doc.curve_def(src).is_ok(), "bad curve {src}");
        let s = src.to_string();
        self.change("curve", false, |d| d.add_ent(Ent::Curve { src: s }))
    }

    // ------------------------------------------------------------ variables

    /// Free (driving) variable.
    pub fn var(&mut self, name: &str, value: f64) -> usize {
        let n = name.to_string();
        self.change("variable", false, |d| d.add_var(n, value))
    }
    /// Variable solved from the constraints, starting at `guess`.
    pub fn solved_var(&mut self, name: &str, guess: f64) -> usize {
        let n = name.to_string();
        self.change("solved variable", false, |d| {
            let i = d.add_var(n, guess);
            let p = d.vars[i].as_ref().unwrap().param;
            d.params[p].locked = false;
            i
        })
    }
    /// Variable defined by an expression (may use dimension and variable names).
    pub fn defvar(&mut self, name: &str, expr: &str) -> usize {
        let (n, e) = (name.to_string(), expr.to_string());
        self.change("defined variable", false, |d| {
            let i = d.add_var(n, 0.0);
            d.vars[i].as_mut().unwrap().expr = e;
            i
        })
    }
    pub fn set_var(&mut self, v: usize, value: f64) {
        let p = self.app.geo.doc.vars[v].as_ref().unwrap().param;
        assert!(self.app.geo.set_values(&[(p, value)], true), "could not set variable");
    }

    // ------------------------------------------------------------ constraints

    /// Adds a constraint; panics when it is unsatisfiable or redundant.
    pub fn constrain(&mut self, c: Con) {
        assert!(self.app.add_constraint(c.clone()), "rejected constraint {c:?}");
    }
    /// Adds a constraint, returning whether it was accepted.
    pub fn try_constrain(&mut self, c: Con) -> bool {
        self.app.add_constraint(c)
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
        let (a, b) = self.app.geo.doc.line_pts(line);
        self.constrain(Con::Horizontal(a, b))
    }
    pub fn vertical(&mut self, line: usize) {
        let (a, b) = self.app.geo.doc.line_pts(line);
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
        let c = self.app.geo.doc.tangent_con(a, b).expect("cannot be tangent");
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
        self.change("midpoint", false, |d| d.build_midpoint(a, b))
    }
    pub fn midpoint_of_line(&mut self, l: usize) -> usize {
        self.change("midpoint of line", false, |d| d.build_midpoint_of_line(l))
    }
    /// Intersection point of two curves near `guess`.
    pub fn intersect(&mut self, a: usize, b: usize, guess: [f64; 2]) -> usize {
        self.change("intersection", true, |d| d.build_intersection(a, b, guess))
    }
    /// Infinite line through `p` perpendicular to line `l`.
    pub fn perp_line(&mut self, l: usize, p: usize) -> usize {
        self.change("perpendicular line", false, |d| d.build_dir_line(l, p, true))
    }
    /// Infinite line through `p` parallel to line `l`.
    pub fn par_line(&mut self, l: usize, p: usize) -> usize {
        self.change("parallel line", false, |d| d.build_dir_line(l, p, false))
    }
    /// Perpendicular bisector of points a, b: (infinite line, midpoint).
    pub fn perp_bisector(&mut self, a: usize, b: usize) -> (usize, usize) {
        let mut mp = 0;
        let l = self.change("perpendicular bisector", false, |d| {
            let (l, m) = d.build_perp_bisector(a, b);
            mp = m;
            l
        });
        (l, mp)
    }
    /// Angle bisector of lines l1, l2 in the sector containing `toward`:
    /// (infinite line, vertex point).
    pub fn angle_bisector(&mut self, l1: usize, l2: usize, toward: [f64; 2]) -> (usize, usize) {
        let mut x = 0;
        let l = self.change("angle bisector", false, |d| {
            let (l, v) = d.build_angle_bisector(l1, l2, toward).expect("parallel lines");
            x = v;
            l
        });
        (l, x)
    }
    /// Segment from `p` tangent to curve `c`, touching near `guess`:
    /// (segment, contact point).
    pub fn tangent_line(&mut self, p: usize, c: usize, guess: [f64; 2]) -> (usize, usize) {
        let mut t = 0;
        let l = self.change("tangent line", true, |d| {
            let (l, tp) = d.build_tangent_line(p, c, guess);
            t = tp;
            l
        });
        (l, t)
    }
    /// Circle through points a, b, c: (circle, center point).
    pub fn circle_through(&mut self, a: usize, b: usize, c: usize) -> (usize, usize) {
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
        let mut out = vec![];
        self.change("foci", true, |d| {
            out = d.build_foci(c);
            0
        });
        assert!(!out.is_empty(), "no foci for entity {c}");
        out
    }
    /// The directrix of conic `c` belonging to the focus nearest `near`.
    pub fn directrix(&mut self, c: usize, near: [f64; 2]) -> usize {
        self.change("directrix", true, |d| d.build_directrix(c, near).expect("no directrix"))
    }
    /// Both asymptotes of hyperbola `c`.
    pub fn asymptotes(&mut self, c: usize) -> (usize, usize) {
        let mut out = vec![];
        self.change("asymptotes", true, |d| {
            out = d.build_asymptotes(c);
            0
        });
        assert_eq!(out.len(), 2, "entity {c} has no asymptotes");
        (out[0], out[1])
    }
    /// Mirror image of `p` in line `l`.
    pub fn reflect(&mut self, p: usize, l: usize) -> usize {
        self.change("reflection", true, |d| d.build_reflection(p, l))
    }

    // ------------------------------------------------------------ dimensions

    fn label_for(&self, kind: &DimKind) -> [f64; 2] {
        let a = self.app.geo.doc.dim_anchor(kind);
        [a[0] + 0.3, a[1] + 0.3]
    }

    /// Adds a driving dimension and sets its value (a number — degrees for
    /// angles — or an expression of variables/dimensions). Panics when the
    /// dimension is redundant or the value cannot be reached.
    pub fn drive(&mut self, kind: DimKind, value: &str) -> usize {
        let lw = self.label_for(&kind);
        let i = self.app.geo.add_dimension(kind, lw).expect("dimension rejected");
        let dim = self.app.geo.doc.dims[i].as_ref().unwrap();
        assert!(dim.driving, "dimension {kind:?} is already determined (redundant); use measure()");
        self.set(i, value);
        i
    }

    /// Adds a reference (measured) dimension.
    pub fn measure(&mut self, kind: DimKind) -> usize {
        let lw = self.label_for(&kind);
        let i = self.app.geo.add_dimension(kind, lw).expect("dimension rejected");
        if self.app.geo.doc.dims[i].as_ref().unwrap().driving {
            self.app.toggle_driving(i);
        }
        assert!(!self.app.geo.doc.dims[i].as_ref().unwrap().driving);
        i
    }

    /// Label position inside the angle between the directions of l1 and l2
    /// (from their first to second points), so the dimension measures the
    /// angle between those directions rather than its supplement.
    fn angle_label(&self, l1: usize, l2: usize) -> [f64; 2] {
        let d = &self.app.geo.doc;
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
        let i = self.app.geo.add_dimension(DimKind::Angle(l1, l2), lw).expect("dimension rejected");
        assert!(self.app.geo.doc.dims[i].as_ref().unwrap().driving, "angle is already determined; use measure_angle()");
        self.set(i, value);
        i
    }

    /// Measures the angle between the directions of lines l1 and l2.
    pub fn measure_angle(&mut self, l1: usize, l2: usize) -> usize {
        let lw = self.angle_label(l1, l2);
        let i = self.app.geo.add_dimension(DimKind::Angle(l1, l2), lw).expect("dimension rejected");
        if self.app.geo.doc.dims[i].as_ref().unwrap().driving {
            self.app.toggle_driving(i);
        }
        i
    }

    /// Sets a driving dimension's value text.
    pub fn set(&mut self, d: usize, value: &str) {
        self.app.commit_dim_text(d, value);
        let dim = self.app.geo.doc.dims[d].as_ref().unwrap();
        assert!(dim.driving, "dimension {} is not driving", dim.name);
        if let Some(v) = crate::expr::parse::plain_number(value) {
            let v = if dim.kind.is_angle() { v.to_radians() } else { v };
            let m = self.app.geo.doc.measure_value(d);
            assert!(close(m, v, 1e-7), "value {value} could not be reached for {} (measures {m})", dim.name);
        } else {
            assert_eq!(dim.expr, value.trim(), "expression {value} rejected for {}", dim.name);
        }
    }

    /// Measured value of a dimension (radians for angles).
    pub fn value(&self, d: usize) -> f64 {
        self.app.geo.doc.measure_value(d)
    }

    /// Remaining degrees of freedom of the whole construction.
    pub fn dof(&self) -> usize {
        self.app.geo.dof
    }

    // ------------------------------------------------------------ checking

    fn formula_text(&self, e: &Expr) -> String {
        let names = self.app.geo.doc.param_names();
        plain(&to_bx(e, &|s| sym_name(&names, s)))
    }

    fn named_symbols(&self) -> HashSet<u32> {
        let d = &self.app.geo.doc;
        d.vars.iter().flatten().map(|v| v.param as u32).chain(d.dims.iter().flatten().map(|x| x.param as u32)).collect()
    }

    /// Derives the formula of dimension `d` and verifies it (see module docs).
    /// Returns the formula as text.
    pub fn check(&mut self, d: usize) -> String {
        assert!(self.app.geo.conflicts.is_empty(), "construction has violated constraints: {:?}", self.app.geo.conflicts);
        assert!(self.app.geo.errors.is_empty(), "construction has errors: {:?}", self.app.geo.errors);
        let (name, target, _) = target_expr(&self.app.geo.doc, Sel::Dim(d)).expect("bad target");
        let t0 = std::time::Instant::now();
        let der = derive::derive(&self.app.geo.doc, &target).unwrap_or_else(|e| panic!("derivation of {name} failed: {e}"));
        let ms = t0.elapsed().as_secs_f64() * 1e3;
        let text = format!("{name} = {}", self.formula_text(&der.expr));
        println!("[{}] {text}   [{ms:.1} ms, dof {}]", std::thread::current().name().unwrap_or("?"), self.app.geo.dof);
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
            let names = self.app.geo.doc.param_names();
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
        let p = &self.app.geo.doc.params;
        eval(e, &|s| p.get(s as usize).map(|x| x.v).unwrap_or(f64::NAN))
    }

    fn scale(&self) -> f64 {
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        for (i, e) in self.app.geo.doc.ents.iter().enumerate() {
            if let Some(Entity { kind: Ent::Point { .. }, hidden: false, .. }) = e {
                let p = self.app.geo.doc.pval(i);
                for k in 0..2 {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
        }
        ((hi[0] - lo[0]).hypot(hi[1] - lo[1])).max(1.0)
    }

    fn articulate(&mut self, d: usize, formula: &Expr, text: &str) {
        let free_pts: Vec<usize> = (0..self.app.geo.doc.ents.len())
            .filter(|&i| match self.app.geo.doc.ent(i) {
                Some(Entity { kind: Ent::Point { x, y }, hidden: false, .. }) => {
                    self.app.geo.free_params.contains(x) || self.app.geo.free_params.contains(y)
                }
                _ => false,
            })
            .collect();
        let mut knobs: Vec<(usize, String)> = Vec::new();
        for x in self.app.geo.doc.dims.iter().flatten() {
            // zero-valued driving dimensions are structural, not parameters
            if x.driving && x.expr.is_empty() && self.app.geo.doc.params[x.param].v != 0.0 {
                knobs.push((x.param, x.name.clone()));
            }
        }
        for v in self.app.geo.doc.vars.iter().flatten() {
            if v.expr.is_empty() && self.app.geo.doc.params[v.param].locked {
                knobs.push((v.param, v.name.clone()));
            }
        }
        let scale = self.scale();
        for _ in 0..self.trials {
            let before = self.app.geo.doc.clone();
            let use_drag = !free_pts.is_empty() && (knobs.is_empty() || self.rand() < 0.55);
            let action = if use_drag {
                let e = free_pts[(self.rand() * free_pts.len() as f64) as usize % free_pts.len()];
                let p0 = self.app.geo.doc.pval(e);
                let ang = self.rand() * std::f64::consts::TAU;
                let r = scale * self.drag_scale * (0.3 + self.rand());
                let kind = DragKind::Point { e, grab: [0.0, 0.0] };
                for k in 1..=5 {
                    let f = k as f64 / 5.0;
                    self.app.drag_solve(&kind, [p0[0] + r * ang.cos() * f, p0[1] + r * ang.sin() * f]);
                }
                let ok = self.app.geo.solve_full();
                (ok, format!("dragged {} by {:.3} toward {:.0}°", self.app.geo.doc.ent(e).map(|x| x.name.clone()).unwrap_or_default(), r, ang.to_degrees()))
            } else if !knobs.is_empty() {
                let (p, name) = knobs[(self.rand() * knobs.len() as f64) as usize % knobs.len()].clone();
                let v0 = self.app.geo.doc.params[p].v;
                let f = 1.0 + self.value_scale * (2.0 * self.rand() - 1.0);
                let nv = if v0.abs() < 1e-9 { self.value_scale * (2.0 * self.rand() - 1.0) } else { v0 * f };
                let ok = self.app.geo.set_values(&[(p, nv)], false);
                (ok, format!("set {name} from {v0} to {nv}"))
            } else {
                break;
            };
            if !action.0 || !self.app.geo.conflicts.is_empty() {
                self.app.geo.doc = before;
                self.app.geo.dirty = true;
                self.app.geo.solve_full();
                continue;
            }
            let m = self.app.geo.doc.measure_value(d);
            let f = self.eval_now(formula);
            assert!(
                close(m, f, 1e-7),
                "after articulation ({}) the formula gives {f} but the measurement is {m}\n  {text}",
                action.1
            );
            self.app.geo.doc = before;
            self.app.geo.dirty = true;
            self.app.geo.solve_full();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every example file written by earlier test runs opens the way the app
    /// opens a file and solves without errors or conflicts.
    #[test]
    fn example_files_load() {
        let Ok(groups) = std::fs::read_dir(EXAMPLES_DIR) else { return };
        for group in groups.flatten() {
            let Ok(files) = std::fs::read_dir(group.path()) else { continue };
            for f in files.flatten() {
                let path = f.path();
                if path.extension().is_none_or(|e| e != "gxp") {
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap();
                let doc: Doc = crate::gxp::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                let mut app = App::new();
                app.geo.doc = doc;
                app.geo.dirty = true;
                app.geo.solve_full();
                assert!(app.geo.errors.is_empty(), "{}: errors {:?}", path.display(), app.geo.errors);
            }
        }
    }
}

#[cfg(test)]
mod formula_sizes {
    use super::*;

    /// Formula lengths. Each entry: an example file written by the generated
    /// tests, and the most characters any of its reference dimensions'
    /// formulas may have.
    #[test]
    fn known_short_formulas_stay_short() {
        let cases: &[(&str, usize)] = &[
            ("gen_angles/polar_walk_large_turning", 260),
            ("gen_angles/quadrilateral_angle_sum", 30),
            ("gen_angles/regular_pentagon_turning_angles", 130),
            ("gen_circle_tangency/wedge_chain_of_three", 90),
            ("gen_circle_tangency/descartes_inner_circle", 180),
            ("gen_constructions/napoleon_triangle", 150),
            ("gen_constructions/two_reflections_rotation_distance", 30),
            ("gen_constructions/bisector_reflection_on_side", 20),
            ("gen_invariants/ptolemy_diagonals", 80),
            ("gen_linkages/regular_pentagon_closing_side", 40),
            ("gen_linkages/hinged_triangles_median", 40),
            ("gen_triangle_centers/incenter_distance_to_sides_floating", 90),
            ("gen_variables/pentagon_golden_ratio", 120),
        ];
        for (file, limit) in cases {
            let path = std::path::Path::new(EXAMPLES_DIR).join(format!("{file}.gxp"));
            let Ok(text) = std::fs::read_to_string(&path) else {
                eprintln!("skipped (no example file yet): {file}");
                continue;
            };
            let doc: Doc = crate::gxp::from_str(&text).unwrap();
            let mut app = App::new();
            app.geo.doc = doc;
            app.geo.dirty = true;
            app.geo.solve_full();
            let names = app.geo.doc.param_names();
            for i in 0..app.geo.doc.dims.len() {
                let Some(d) = app.geo.doc.dims[i].as_ref() else { continue };
                if d.driving {
                    continue;
                }
                let (name, target, _) = target_expr(&app.geo.doc, Sel::Dim(i)).expect("bad target");
                let der = derive::derive(&app.geo.doc, &target).unwrap();
                let text = plain(&to_bx(&der.expr, &|s| sym_name(&names, s)));
                assert!(der.exact, "{file}: {name} is not exact");
                assert!(text.len() <= *limit, "{file}: {name} has {} characters (limit {limit}): {text}", text.len());
                // the same derivation again gives the same formula
                let again = derive::derive(&app.geo.doc, &target).unwrap();
                assert_eq!(text, plain(&to_bx(&again.expr, &|s| sym_name(&names, s))), "{file}: {name} differs between two derivations");
            }
        }
    }
}

#[cfg(test)]
mod derive_file {
    use super::*;

    /// Derives every reference dimension of the construction file named by
    /// GXPAD_DERIVE_FILE, for debugging a formula the app could not find
    /// (GXPAD_DERIVE_LEVEL: "search harder" presses, default 0).
    #[test]
    #[ignore]
    fn derive_file() {
        let Ok(path) = std::env::var("GXPAD_DERIVE_FILE") else { return };
        let doc: Doc = crate::gxp::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let mut app = App::new();
        app.geo.doc = doc;
        app.geo.dirty = true;
        app.geo.solve_full();
        println!("dof {}, conflicts {:?}, errors {:?}", app.geo.dof, app.geo.conflicts, app.geo.errors);
        for i in 0..app.geo.doc.dims.len() {
            let Some(d) = app.geo.doc.dims[i].as_ref() else { continue };
            if d.driving {
                continue;
            }
            let (name, target, _) = target_expr(&app.geo.doc, Sel::Dim(i)).expect("bad target");
            let t0 = std::time::Instant::now();
            let level: u64 = std::env::var("GXPAD_DERIVE_LEVEL").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
            match derive::derive_harder(&app.geo.doc, &target, level * SEARCH_HARDER_MS) {
                Ok(der) => println!(
                    "{name} = {}   [exact {}, value {}, measured {}, {:?}]",
                    plain(&to_bx(&der.expr, &|s| format!("s{s}"))),
                    der.exact,
                    der.value,
                    der.actual,
                    t0.elapsed()
                ),
                Err(e) => println!("{name}: error {e}"),
            }
        }
    }
}
