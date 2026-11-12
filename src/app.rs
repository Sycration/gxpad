//! Application state, editing actions, solving and formula derivation.

use crate::geo;
use crate::ops::{self, ConKind, DimType, Document, Place, Target};
use eframe::egui::{self, Color32, Pos2, Rect, pos2};
use std::collections::HashMap;
use std::path::PathBuf;

pub const BG: Color32 = Color32::from_rgb(0x11, 0x13, 0x17);
pub const PANEL: Color32 = Color32::from_rgb(0x17, 0x1a, 0x20);
pub const SURFACE: Color32 = Color32::from_rgb(0x20, 0x24, 0x2c);
pub const SURFACE_HI: Color32 = Color32::from_rgb(0x2a, 0x2f, 0x3a);
pub const LINE: Color32 = Color32::from_rgb(0x2c, 0x31, 0x3b);
pub const ACCENT: Color32 = Color32::from_rgb(0x6c, 0xb6, 0xff);
pub const FULL: Color32 = Color32::from_rgb(0xe8, 0xeb, 0xf0);
pub const SEL: Color32 = Color32::from_rgb(0xff, 0xb3, 0x47);
pub const ERR: Color32 = Color32::from_rgb(0xff, 0x5f, 0x5f);
pub const DIMC: Color32 = Color32::from_rgb(0xb7, 0x94, 0xff);
pub const MUTED: Color32 = Color32::from_rgb(0x84, 0x8b, 0x98);
pub const ICON: Color32 = Color32::from_rgb(0xb4, 0xbb, 0xc7);
pub const GOOD: Color32 = Color32::from_rgb(0x5f, 0xd3, 0x8d);
pub const CURVES: [Color32; 6] = [
    Color32::from_rgb(0x4f, 0xd1, 0xc5),
    Color32::from_rgb(0xf6, 0x87, 0xb3),
    Color32::from_rgb(0x9a, 0xe6, 0x6e),
    Color32::from_rgb(0xff, 0x9f, 0x5a),
    Color32::from_rgb(0xf6, 0xe0, 0x5e),
    Color32::from_rgb(0x8f, 0xa8, 0xff),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Select,
    Point,
    Line,
    Circle,
    /// Circle through three points.
    Circle3,
    Arc,
    /// Arc from a start point through a point to an end point.
    Arc3,
    Ellipse,
    Hyperbola,
    Parabola,
    /// Ellipse from its foci and a point on it.
    EllipseFoci,
    /// Hyperbola from its foci and a point on it.
    HyperbolaFoci,
    /// Parabola from its focus and directrix line.
    ParabolaFocus,
    Curve,
    Dimension,
    Midpoint,
    Intersect,
    PerpLine,
    ParLine,
    PerpBisector,
    AngleBisector,
    TangentLine,
    Reflect,
    Foci,
    Directrix,
    Asymptotes,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Sel {
    Ent(usize),
    Con(usize),
    Dim(usize),
    Var(usize),
}

#[derive(Clone, Copy, Debug)]
pub enum Snap {
    Point(usize),
    Curve(usize, [f64; 2]),
    Free([f64; 2]),
}

impl From<Snap> for Place {
    fn from(s: Snap) -> Self {
        match s {
            Snap::Point(e) => Place::Ent(e),
            Snap::Free(p) => Place::Free(p),
            Snap::Curve(c, p) => Place::On(c, p),
        }
    }
}

/// A dimension to add: its kind and the entities it measures (`b` unused
/// for one-entity kinds).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DimSpec {
    pub kind: DimType,
    pub a: usize,
    pub b: usize,
}

impl DimSpec {
    pub fn new(kind: DimType, a: usize, b: usize) -> DimSpec {
        DimSpec { kind, a, b }
    }
    pub fn one(kind: DimType, a: usize) -> DimSpec {
        DimSpec { kind, a, b: usize::MAX }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct View {
    pub c: [f64; 2],
    pub s: f64,
    pub rect: Rect,
}

impl View {
    pub fn to_s(&self, p: [f64; 2]) -> Pos2 {
        let m = self.rect.center();
        pos2(m.x + ((p[0] - self.c[0]) * self.s) as f32, m.y - ((p[1] - self.c[1]) * self.s) as f32)
    }
    pub fn to_w(&self, p: Pos2) -> [f64; 2] {
        let m = self.rect.center();
        [self.c[0] + (p.x - m.x) as f64 / self.s, self.c[1] - (p.y - m.y) as f64 / self.s]
    }
    /// The part of the plane the view shows: lower-left and upper-right
    /// corners and the size of a pixel, in world units.
    pub fn window(&self) -> ([f64; 2], [f64; 2], f64) {
        let (hw, hh) = (self.rect.width() as f64 * 0.5 / self.s, self.rect.height() as f64 * 0.5 / self.s);
        ([self.c[0] - hw, self.c[1] - hh], [self.c[0] + hw, self.c[1] + hh], 1.0 / self.s)
    }
}

#[derive(Clone, Debug)]
pub enum DragKind {
    Point { e: usize, grab: [f64; 2] },
    /// Unlocked points with their positions when the drag started.
    Translate { pts: Vec<(usize, [f64; 2])>, start: [f64; 2] },
    CircleEdge { e: usize },
    Handle { e: usize, which: u8 },
    Curve { e: usize },
    DimLabel { d: usize, grab: [f64; 2] },
    Pan,
    Box { start: Pos2 },
}

pub struct FormulaView {
    pub target: Sel,
    pub name: String,
    /// The formula as LaTeX rows: `name = formula` with repeated
    /// subexpressions written `u_k`, then one row per `u_k`.
    pub rows: Vec<String>,
    pub exact: bool,
    pub value: f64,
    pub actual: f64,
    pub is_angle: bool,
    pub err: Option<String>,
    pub plain: String,
    pub latex: String,
    pub ms: f64,
    /// "Search harder" level the formula was derived at (10 s more per level).
    pub level: u32,
    /// Complexity of the formula (0 without one).
    pub size: usize,
    /// Document generation the formula was derived for.
    pub generation: u64,
}

impl FormulaView {
    /// Lower is better: exact before numeric, then smaller.
    fn score(&self) -> (bool, bool, usize) {
        (self.err.is_some(), !self.exact, self.size)
    }
    /// Exact and small: no further search is started on its own.
    pub fn settled(&self) -> bool {
        self.err.is_some() || (self.exact && self.size <= SETTLED_SIZE)
    }
}

/// Extra derivation time per "search harder" press.
pub const SEARCH_HARDER_MS: u64 = 10_000;
/// Levels of longer search started automatically for a formula that is
/// numeric or large, as long as each one changes the result.
pub const AUTO_SEARCH_LEVELS: u32 = 3;
/// Formula complexity up to which an exact formula counts as simple enough.
pub const SETTLED_SIZE: usize = 60;

pub struct DimEdit {
    pub dim: usize,
    pub text: String,
    pub focus: bool,
}

pub struct App {
    /// The document with its solver state and undo history.
    pub geo: Document,
    pub view: View,
    pub tool: Tool,
    pub sel: Vec<Sel>,
    pub hover: Option<Sel>,
    pub hover_handle: Option<(usize, u8)>,
    pub drag: Option<DragKind>,
    pub pending: Vec<Snap>,
    pub shapes: HashMap<usize, Vec<Vec<Pos2>>>,
    pub glyphs: Vec<(Rect, usize)>,
    pub dim_rects: Vec<(Rect, usize)>,
    pub handles: Vec<(Pos2, usize, u8)>,
    pub formula: Option<FormulaView>,
    pub formula_key: Option<(Sel, u64, u32)>,
    pub formula_job: Option<((Sel, u64, u32), std::sync::mpsc::Receiver<FormulaView>, ops::Cancel)>,
    /// "Search harder" level for a target in a document generation.
    pub search_level: Option<(Sel, u64, u32)>,
    pub edit: Option<DimEdit>,
    pub flash: Vec<usize>,
    pub flash_until: f64,
    pub show_cons: bool,
    pub curve_input: Option<String>,
    pub path: Option<PathBuf>,
    pub bufs: HashMap<String, String>,
    pub fitted: bool,
    pub hv_hint: Option<bool>,
    pub now: f64,
    pub box_sel: Option<(Pos2, Pos2)>,
    pub mouse_w: [f64; 2],
    pub title: String,
    /// False during a repeated egui pass, when input must not be applied twice.
    pub live: bool,
    /// Entities picked so far by a construction tool.
    pub picks: Vec<usize>,
    /// Open tool group fly-out.
    pub flyout: Option<usize>,
    /// Last tool used from each group.
    pub group_last: [Tool; 4],
}

pub fn fmt_val(v: f64) -> String {
    if !v.is_finite() {
        return "—".into();
    }
    let a = v.abs();
    if a != 0.0 && !(1e-4..1e7).contains(&a) {
        return format!("{v:.4e}");
    }
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

pub fn fmt_dim(v: f64, angle: bool) -> String {
    if angle { format!("{}°", fmt_val(v.to_degrees())) } else { fmt_val(v) }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> App {
        App {
            geo: Document::new(),
            view: View { c: [0.0, 0.0], s: 60.0, rect: Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0)) },
            tool: Tool::Select,
            sel: vec![],
            hover: None,
            hover_handle: None,
            drag: None,
            pending: vec![],
            shapes: HashMap::new(),
            glyphs: vec![],
            dim_rects: vec![],
            handles: vec![],
            formula: None,
            formula_key: None,
            formula_job: None,
            search_level: None,
            edit: None,
            flash: vec![],
            flash_until: 0.0,
            show_cons: true,
            curve_input: None,
            path: None,
            bufs: HashMap::new(),
            fitted: false,
            hv_hint: None,
            now: 0.0,
            box_sel: None,
            mouse_w: [0.0; 2],
            title: String::new(),
            live: true,
            picks: vec![],
            flyout: None,
            group_last: [Tool::Ellipse, Tool::Midpoint, Tool::Circle, Tool::Arc],
        }
    }

    // ------------------------------------------------------------ history

    pub fn undo(&mut self) {
        if ops::undo(&mut self.geo) {
            self.after_history();
        }
    }

    pub fn redo(&mut self) {
        if ops::redo(&mut self.geo) {
            self.after_history();
        }
    }

    fn after_history(&mut self) {
        self.edit = None;
        self.pending.clear();
        let d = &self.geo;
        self.sel.retain(|s| match *s {
            Sel::Ent(e) => ops::ent_kind(d, e).is_ok(),
            Sel::Con(c) => ops::con_kind(d, c).is_ok(),
            Sel::Dim(i) => ops::dim_kind(d, i).is_ok(),
            Sel::Var(v) => ops::var_name(d, v).is_ok(),
        });
    }

    pub fn flash_ents(&mut self, ents: Vec<usize>) {
        self.flash = ents;
        self.flash_until = self.now + 0.6;
    }

    // ------------------------------------------------------------ dragging

    /// Moves what drag `kind` holds to the mouse at world position `m`.
    pub fn drag_solve(&mut self, kind: &DragKind, m: [f64; 2]) {
        let d = &mut self.geo;
        let _ = match kind {
            DragKind::Point { e, grab } => ops::drag_point(d, *e, m[0] - grab[0], m[1] - grab[1]),
            DragKind::Translate { pts, start } => {
                let ids: Vec<usize> = pts.iter().map(|(p, _)| *p).collect();
                let to: Vec<[f64; 2]> = pts.iter().map(|(_, p0)| [p0[0] + m[0] - start[0], p0[1] + m[1] - start[1]]).collect();
                ops::drag_points(d, &ids, &to)
            }
            DragKind::CircleEdge { e } => ops::drag_circle_edge(d, *e, m[0], m[1]),
            DragKind::Handle { e, which } => ops::drag_conic_handle(d, *e, *which, m[0], m[1]),
            DragKind::Curve { e } => ops::drag_curve(d, *e, m[0], m[1]),
            DragKind::DimLabel { .. } | DragKind::Pan | DragKind::Box { .. } => Ok(true),
        };
    }

    // ------------------------------------------------------------ creation

    pub fn snap_pos(&self, s: &Snap) -> [f64; 2] {
        match s {
            Snap::Point(e) => geo::pos(&self.geo, *e),
            Snap::Curve(_, p) | Snap::Free(p) => *p,
        }
    }

    /// Adds constraint (kind, a, b); flashes its entities when it is rejected.
    pub fn add_constraint(&mut self, kind: ConKind, a: usize, b: usize) -> bool {
        let ok = ops::constrain_near(&mut self.geo, kind, a, b, self.mouse_w).unwrap_or(false);
        if !ok {
            self.flash_ents(vec![a, b]);
        }
        ok
    }

    /// Adds a dimension with its label at world position `label`; selects it
    /// and opens its editor when it is driving.
    pub fn add_dimension(&mut self, k: DimSpec, label: [f64; 2]) -> Option<usize> {
        let a = ops::dim_anchor_of(&self.geo, k.kind, k.a, k.b).ok()?;
        let i = ops::add_dim(&mut self.geo, k.kind, k.a, k.b, [label[0] - a[0], label[1] - a[1]]).ok()?;
        self.sel = vec![Sel::Dim(i)];
        if ops::dim_driving(&self.geo, i).unwrap_or(false) {
            self.open_dim_editor(i);
        }
        Some(i)
    }

    fn flash_dim(&mut self, i: usize) {
        if let Ok(ents) = ops::dim_ents(&self.geo, i) {
            self.flash_ents(ents);
        }
    }

    /// Commits text typed for a dimension value.
    pub fn commit_dim_text(&mut self, i: usize, text: &str) {
        if !ops::set_dim(&mut self.geo, i, text).unwrap_or(false) {
            self.flash_dim(i);
        }
    }

    pub fn toggle_driving(&mut self, i: usize) {
        let Ok(driving) = ops::dim_driving(&self.geo, i) else { return };
        if !ops::set_driving(&mut self.geo, i, !driving).unwrap_or(false) {
            self.flash_dim(i);
        }
    }

    pub fn delete_selection(&mut self) {
        let pick = |f: fn(&Sel) -> Option<usize>| -> Vec<usize> { self.sel.iter().filter_map(f).collect() };
        let ents = pick(|s| if let Sel::Ent(e) = s { Some(*e) } else { None });
        let cons = pick(|s| if let Sel::Con(e) = s { Some(*e) } else { None });
        let dims = pick(|s| if let Sel::Dim(e) = s { Some(*e) } else { None });
        let vars = pick(|s| if let Sel::Var(e) = s { Some(*e) } else { None });
        if !ops::delete(&mut self.geo, &ents, &cons, &dims, &vars).unwrap_or(false) {
            return;
        }
        self.sel.clear();
        self.hover = None;
    }

    // ------------------------------------------------------------ formulas

    /// Starts (or reuses) the derivation of the formula for the selected
    /// dimension or variable. Derivation runs on a worker thread.
    pub fn update_formula(&mut self) {
        if let Some((key, rx, _)) = &self.formula_job {
            let key = *key;
            match rx.try_recv() {
                Ok(mut fv) => {
                    self.formula_job = None;
                    if Some(key) == self.formula_key {
                        fv.generation = key.1;
                        self.take_formula(fv, key);
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.formula_job = None,
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
        let target = self.sel.iter().rev().find(|s| matches!(s, Sel::Dim(_) | Sel::Var(_))).copied();
        let Some(target) = target else {
            self.cancel_formula_job();
            self.formula = None;
            self.formula_key = None;
            return;
        };
        let generation = ops::generation(&self.geo);
        if self.drag.is_some() && self.formula.as_ref().is_some_and(|f| f.target == target) {
            // a long search for the construction before the drag is of no use
            if self.formula_job.as_ref().is_some_and(|j| j.0 .1 != generation) {
                self.cancel_formula_job();
            }
            return;
        }
        let level = self.search_level_of(target);
        let key = (target, generation, level);
        if self.formula_key == Some(key) {
            return;
        }
        self.cancel_formula_job();
        self.formula_key = Some(key);
        if self.formula.as_ref().is_some_and(|f| f.target != target) {
            self.formula = None;
        }
        let doc = ops::copy(&self.geo);
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = ops::new_cancel();
        let flag = cancel.clone();
        std::thread::spawn(move || {
            let _ = tx.send(compute_formula(&doc, target, level, Some(&flag)));
        });
        self.formula_job = Some((key, rx, cancel));
    }

    /// Shows a finished derivation unless the one shown for the same target
    /// and construction is better, and starts the next longer search while
    /// the formula is numeric or large and the last search changed it.
    fn take_formula(&mut self, fv: FormulaView, key: (Sel, u64, u32)) {
        let (target, generation, level) = key;
        let prev = self.formula.as_ref().filter(|f| f.target == target && f.generation == generation);
        let changed = prev.is_none_or(|p| p.plain != fv.plain || p.exact != fv.exact);
        // a tie goes to the longer search
        if prev.is_none_or(|p| fv.score() <= p.score()) {
            self.formula = Some(fv);
        }
        let best = self.formula.as_ref().unwrap();
        if changed && !best.settled() && level < AUTO_SEARCH_LEVELS {
            self.search_level = Some((target, generation, level + 1));
        }
    }

    fn cancel_formula_job(&mut self) {
        if let Some((_, _, cancel)) = self.formula_job.take() {
            ops::cancel(&cancel);
        }
    }

    /// The search level for `target` in the current construction.
    pub fn search_level_of(&self, target: Sel) -> u32 {
        match self.search_level {
            Some((t, g, n)) if t == target && g == ops::generation(&self.geo) => n,
            _ => 0,
        }
    }

    pub fn formula_busy(&self) -> bool {
        self.formula_job.is_some()
    }

    /// Derives the shown formula again with 10 s more search time than the
    /// last attempt.
    pub fn search_harder(&mut self) {
        let Some(f) = &self.formula else { return };
        let target = f.target;
        let level = self.search_level_of(target);
        self.search_level = Some((target, ops::generation(&self.geo), level + 1));
    }

    // ------------------------------------------------------------ files

    pub fn save(&mut self, ask: bool) {
        let path = if ask || self.path.is_none() {
            rfd::FileDialog::new().add_filter("gxpad", &["gxp"]).set_file_name("construction.gxp").save_file()
        } else {
            self.path.clone()
        };
        if let Some(p) = path {
            if let Ok(s) = ops::to_gxp(&self.geo) {
                if std::fs::write(&p, s).is_ok() {
                    self.path = Some(p);
                }
            }
        }
    }

    pub fn open(&mut self) {
        if let Some(p) = rfd::FileDialog::new().add_filter("gxpad", &["gxp"]).pick_file() {
            if let Ok(s) = std::fs::read_to_string(&p) {
                if ops::load_gxp(&mut self.geo, &s).is_ok() {
                    self.path = Some(p);
                    self.sel.clear();
                    self.fitted = false;
                }
            }
        }
    }

    /// Replaces the document with an empty one, as an undoable change.
    pub fn new_document(&mut self) {
        ops::clear(&mut self.geo);
        self.path = None;
        self.sel.clear();
    }
}

/// Derives the formula of a selected dimension or variable.
pub fn compute_formula(doc: &Document, target: Sel, level: u32, cancel: Option<&ops::Cancel>) -> FormulaView {
    let t0 = std::time::Instant::now();
    let mut fv = FormulaView {
        target,
        name: String::new(),
        rows: vec![],
        exact: false,
        value: f64::NAN,
        actual: f64::NAN,
        is_angle: false,
        err: None,
        plain: String::new(),
        latex: String::new(),
        ms: 0.0,
        level,
        size: 0,
        generation: 0,
    };
    let t = match target {
        Sel::Dim(i) => Target::Dim(i),
        Sel::Var(i) => Target::Var(i),
        _ => {
            fv.err = Some(String::new());
            return fv;
        }
    };
    fv.name = match t {
        Target::Dim(i) => ops::dim_name(doc, i),
        Target::Var(i) => ops::var_name(doc, i),
    }
    .unwrap_or_default();
    match ops::derive(doc, t, level as u64 * SEARCH_HARDER_MS, cancel) {
        Ok(f) => {
            fv.rows = std::iter::once(f.latex_short.clone()).chain(f.defs.iter().map(|(n, d)| format!("{n} = {d}"))).collect();
            fv.name = f.name;
            fv.is_angle = f.is_angle;
            fv.plain = f.plain;
            fv.latex = f.latex;
            fv.exact = f.exact;
            fv.size = f.size;
            fv.value = f.value;
            fv.actual = f.actual;
        }
        Err(m) => fv.err = Some(m),
    }
    fv.ms = t0.elapsed().as_secs_f64() * 1e3;
    fv
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Triangle with three length dimensions, driven to a 5-4-3 shape by
    /// typing into a dimension.
    fn triangle(app: &mut App) -> Vec<usize> {
        let g = &mut app.geo;
        let (a, b, c) = (ops::add_point(g, -2.67, -2.83).unwrap(), ops::add_point(g, 4.67, -2.83).unwrap(), ops::add_point(g, -0.33, 2.83).unwrap());
        let ls: Vec<usize> = [(a, b), (b, c), (c, a)].iter().map(|&(p, q)| ops::add_line(g, p, q, false).unwrap()).collect();
        assert!(ops::constrain(g, ConKind::Horizontal, a, b).unwrap());
        ls.iter().map(|l| app.add_dimension(DimSpec::one(DimType::Length, *l), [0.0, -4.0]).unwrap()).collect()
    }

    #[test]
    fn commit_value() {
        let mut app = App::new();
        let dims = triangle(&mut app);
        assert!(dims.iter().all(|d| ops::dim_driving(&app.geo, *d).unwrap()));
        // a new driving dimension opens its editor
        assert_eq!(app.edit.as_ref().map(|e| e.dim), Some(dims[2]));
        app.commit_dim_text(dims[0], "5");
        assert!((ops::dim_value(&app.geo, dims[0]).unwrap() - 5.0).abs() < 1e-6);
    }

    #[test]
    fn undo_after_a_drag_is_one_step() {
        let mut app = App::new();
        let p = ops::add_point(&mut app.geo, 1.0, 1.0).unwrap();
        ops::checkpoint(&mut app.geo);
        let kind = DragKind::Point { e: p, grab: [0.0, 0.0] };
        for k in 1..=5 {
            app.drag_solve(&kind, [1.0 + k as f64 * 0.2, 1.0]);
        }
        assert!((geo::pos(&app.geo, p)[0] - 2.0).abs() < 1e-9);
        app.undo();
        assert!((geo::pos(&app.geo, p)[0] - 1.0).abs() < 1e-9);
    }

    fn wait_formula(app: &mut App) {
        for _ in 0..2000 {
            app.update_formula();
            if !app.formula_busy() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("formula did not finish");
    }

    /// Two points fixed by coordinate dimensions, and the distances between
    /// them and to the origin as reference dimensions.
    fn two_distances(app: &mut App) -> (usize, usize) {
        let g = &mut app.geo;
        let a = ops::add_point(g, 0.0, 0.0).unwrap();
        let b = ops::add_point(g, 3.0, 4.0).unwrap();
        for (k, p, v) in [(DimType::CoordX, a, "0"), (DimType::CoordY, a, "0"), (DimType::CoordX, b, "3"), (DimType::CoordY, b, "4")] {
            let i = ops::add_dim(g, k, p, usize::MAX, [0.3, 0.3]).unwrap();
            assert!(ops::set_dim(g, i, v).unwrap());
        }
        let o = ops::origin(g);
        let d = ops::add_dim(g, DimType::Distance, a, b, [0.3, 0.3]).unwrap();
        let e = ops::add_dim(g, DimType::Distance, b, o, [0.3, 0.3]).unwrap();
        assert!(!ops::dim_driving(g, d).unwrap() && !ops::dim_driving(g, e).unwrap());
        (d, e)
    }

    #[test]
    fn each_press_raises_the_level_until_the_target_changes() {
        let mut app = App::new();
        let (d, e) = two_distances(&mut app);
        app.sel = vec![Sel::Dim(d)];
        wait_formula(&mut app);
        assert_eq!(app.formula.as_ref().unwrap().level, 0);
        for level in 1..=2 {
            app.search_harder();
            wait_formula(&mut app);
            let f = app.formula.as_ref().unwrap();
            assert_eq!(f.level, level);
            assert!(f.exact);
        }
        // another target starts over at the normal search
        app.sel = vec![Sel::Dim(e)];
        wait_formula(&mut app);
        assert_eq!(app.formula.as_ref().unwrap().level, 0);
    }

    /// The formula rows the panel shows are LaTeX that latex-rust lays out.
    #[test]
    fn formula_rows_render() {
        let mut app = App::new();
        let (d, _) = two_distances(&mut app);
        let fv = compute_formula(&app.geo, Sel::Dim(d), 0, None);
        assert!(fv.err.is_none(), "{:?}", fv.err);
        assert!(!fv.rows.is_empty());
        let font = latex_rust::MathFont::stix_two_math().unwrap();
        for row in fv.rows.iter().chain([&fv.latex]) {
            latex_rust::latex_to_shapes(row, &font, &latex_rust::EguiOptions::new(), egui::Pos2::ZERO, 1.0).unwrap_or_else(|e| panic!("{row}: {e}"));
        }
    }
}
