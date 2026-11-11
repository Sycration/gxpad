//! Application state, editing actions, solving and formula derivation.

use crate::derive;
use crate::expr::bx::{Bx, latex, plain, to_bx};
use crate::expr::parse::plain_number;
use crate::expr::*;
use crate::model::*;
use crate::solver::{DragEq, Sys};
use eframe::egui::{self, Color32, Pos2, Rect, pos2};
use std::collections::{HashMap, HashSet};
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

/// Symbols at or above this value name common subexpressions in formulas.
pub const CSE_BASE: u32 = 0xF000_0000;

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
}

#[derive(Clone, Debug)]
pub enum DragKind {
    Point { e: usize, grab: [f64; 2] },
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
    pub bx: Option<Bx>,
    pub defs: Vec<(String, Bx)>,
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
    pub doc: Doc,
    pub undo: Vec<Doc>,
    pub redo: Vec<Doc>,
    pub sys: Option<Sys>,
    pub dirty: bool,
    pub free_params: HashSet<usize>,
    pub dof: usize,
    pub rank: usize,
    pub conflicts: HashSet<Owner>,
    pub errors: HashMap<Owner, String>,
    pub view: View,
    pub tool: Tool,
    pub sel: Vec<Sel>,
    pub hover: Option<Sel>,
    pub hover_handle: Option<(usize, u8)>,
    pub drag: Option<DragKind>,
    pub drag_backup: Option<Doc>,
    pub pending: Vec<Snap>,
    pub shapes: HashMap<usize, Vec<Vec<Pos2>>>,
    pub glyphs: Vec<(Rect, usize)>,
    pub dim_rects: Vec<(Rect, usize)>,
    pub handles: Vec<(Pos2, usize, u8)>,
    pub formula: Option<FormulaView>,
    pub formula_key: Option<(Sel, u64, u32)>,
    pub formula_job: Option<((Sel, u64, u32), std::sync::mpsc::Receiver<FormulaView>, std::sync::Arc<std::sync::atomic::AtomicBool>)>,
    /// "Search harder" level for a target in a document generation.
    pub search_level: Option<(Sel, u64, u32)>,
    pub generation: u64,
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

/// Renames identifier tokens equal to `old` in an expression string.
pub fn rename_ident(s: &str, old: &str, new: &str) -> String {
    let cs: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < cs.len() {
        if cs[i].is_alphabetic() || cs[i] == '_' {
            let st = i;
            while i < cs.len() && (cs[i].is_alphanumeric() || cs[i] == '_') {
                i += 1;
            }
            let w: String = cs[st..i].iter().collect();
            out.push_str(if w == old { new } else { &w });
        } else {
            out.push(cs[i]);
            i += 1;
        }
    }
    out
}

impl App {
    pub fn new() -> App {
        let mut a = App {
            doc: Doc::new(),
            undo: vec![],
            redo: vec![],
            sys: None,
            dirty: true,
            free_params: HashSet::new(),
            dof: 0,
            rank: 0,
            conflicts: HashSet::new(),
            errors: HashMap::new(),
            view: View { c: [0.0, 0.0], s: 60.0, rect: Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0)) },
            tool: Tool::Select,
            sel: vec![],
            hover: None,
            hover_handle: None,
            drag: None,
            drag_backup: None,
            pending: vec![],
            shapes: HashMap::new(),
            glyphs: vec![],
            dim_rects: vec![],
            handles: vec![],
            formula: None,
            formula_key: None,
            formula_job: None,
            search_level: None,
            generation: 0,
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
        };
        a.solve_full();
        a
    }

    // ------------------------------------------------------------ solving

    pub fn rebuild(&mut self) {
        let (eqns, errs) = self.doc.equations();
        self.errors = errs.into_iter().collect();
        self.sys = Some(Sys::build(&eqns, self.doc.unknowns()));
        self.dirty = false;
    }

    fn vals(&self) -> Vec<f64> {
        self.doc.params.iter().map(|p| p.v).collect()
    }

    fn store(&mut self, vals: &[f64]) {
        for (p, v) in self.doc.params.iter_mut().zip(vals) {
            p.v = *v;
        }
    }

    /// Solves without drags and refreshes the constraint-status analysis.
    pub fn solve_full(&mut self) -> bool {
        if self.dirty || self.sys.is_none() {
            self.rebuild();
        }
        let sys = self.sys.as_ref().unwrap();
        let mut vals = self.vals();
        let ok = sys.solve(&mut vals, &[], [0.0; 2]);
        let an = sys.analyze(&vals);
        let free: HashSet<usize> = an.free.iter().enumerate().filter(|(_, f)| **f).map(|(i, _)| sys.unknowns[i]).collect();
        let rank = sys.unknowns.len() - an.dof;
        let conflicts: HashSet<Owner> = sys.violated(&vals).into_iter().collect();
        self.store(&vals);
        self.free_params = free;
        self.dof = an.dof;
        self.rank = rank;
        self.conflicts = conflicts;
        ok
    }

    fn eq_count(&self) -> usize {
        self.sys.as_ref().map(|s| s.eqs.len()).unwrap_or(0)
    }

    /// Applies a structural change; rolls back when it cannot be satisfied
    /// (or, with `strict`, when it adds redundant equations).
    pub fn try_change(&mut self, strict: bool, f: impl FnOnce(&mut Doc)) -> bool {
        let before = self.doc.clone();
        if self.dirty {
            self.solve_full();
        }
        let (rank0, eqs0) = (self.rank, self.eq_count());
        let had_conflicts = !self.conflicts.is_empty();
        let unk0 = self.sys.as_ref().map(|s| s.unknowns.len()).unwrap_or(0);
        f(&mut self.doc);
        self.dirty = true;
        let ok = self.solve_full();
        let new_unk = self.sys.as_ref().map(|s| s.unknowns.len()).unwrap_or(0).saturating_sub(unk0);
        let added = self.eq_count().saturating_sub(eqs0);
        let _ = new_unk;
        let redundant = strict && added > 0 && self.rank < rank0 + added;
        if (!ok && !had_conflicts) || redundant || (!self.conflicts.is_empty() && !had_conflicts) {
            self.doc = before;
            self.dirty = true;
            self.solve_full();
            return false;
        }
        self.push_undo(before);
        true
    }

    pub fn push_undo(&mut self, before: Doc) {
        self.undo.push(before);
        if self.undo.len() > 300 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.generation += 1;
    }

    pub fn checkpoint(&mut self) {
        let d = self.doc.clone();
        self.push_undo(d);
    }

    pub fn undo(&mut self) {
        if let Some(d) = self.undo.pop() {
            let cur = std::mem::replace(&mut self.doc, d);
            self.redo.push(cur);
            self.after_history();
        }
    }

    pub fn redo(&mut self) {
        if let Some(d) = self.redo.pop() {
            let cur = std::mem::replace(&mut self.doc, d);
            self.undo.push(cur);
            self.after_history();
        }
    }

    fn after_history(&mut self) {
        self.dirty = true;
        self.generation += 1;
        self.edit = None;
        self.pending.clear();
        self.sel.retain(|s| match *s {
            Sel::Ent(e) => self.doc.ent(e).is_some(),
            Sel::Con(c) => self.doc.cons.get(c).is_some_and(|x| x.is_some()),
            Sel::Dim(d) => self.doc.dims.get(d).is_some_and(|x| x.is_some()),
            Sel::Var(v) => self.doc.vars.get(v).is_some_and(|x| x.is_some()),
        });
        self.solve_full();
    }

    /// Changes parameter values (dimensions, variables), solving with
    /// continuation when a direct solve fails.
    pub fn set_values(&mut self, changes: &[(usize, f64)], undoable: bool) -> bool {
        let before = self.doc.clone();
        let old: Vec<f64> = changes.iter().map(|(p, _)| self.doc.params[*p].v).collect();
        for (p, v) in changes {
            self.doc.params[*p].v = *v;
        }
        let mut ok = self.solve_full();
        if !ok {
            self.doc = before.clone();
            let steps = 24;
            ok = true;
            for k in 1..=steps {
                let t = k as f64 / steps as f64;
                for (j, (p, v)) in changes.iter().enumerate() {
                    self.doc.params[*p].v = old[j] + (v - old[j]) * t;
                }
                if !self.solve_full() {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            self.doc = before;
            self.solve_full();
            return false;
        }
        if undoable {
            self.push_undo(before);
        } else {
            self.generation += 1;
        }
        true
    }

    pub fn flash_ents(&mut self, ents: Vec<usize>) {
        self.flash = ents;
        self.flash_until = self.now + 0.6;
    }

    // ------------------------------------------------------------ dragging

    pub fn drag_exprs(&self, kind: &DragKind, m: [f64; 2]) -> Vec<Expr> {
        let d = &self.doc;
        let k = |v: f64| float_raw(v);
        match kind {
            DragKind::Point { e, grab } => {
                let (x, y) = d.pexpr(*e);
                vec![x - k(m[0] - grab[0]), y - k(m[1] - grab[1])]
            }
            DragKind::Translate { pts, start } => {
                let mut v = Vec::new();
                for (p, p0) in pts {
                    let (x, y) = d.pexpr(*p);
                    v.push(x - k(p0[0] + m[0] - start[0]));
                    v.push(y - k(p0[1] + m[1] - start[1]));
                }
                v
            }
            DragKind::CircleEdge { e } => {
                let (c, _) = d.circ_val(*e);
                match d.kind(*e) {
                    Some(Ent::Circle { r, .. }) => vec![sym(*r as u32) - k((m[0] - c[0]).hypot(m[1] - c[1]))],
                    _ => vec![],
                }
            }
            DragKind::Handle { e, which } => {
                let Some(Ent::Conic { c, a, b, t, .. }) = d.kind(*e) else { return vec![] };
                let cc = d.pval(*c);
                let (dx, dy) = (m[0] - cc[0], m[1] - cc[1]);
                let tv = d.params[*t].v;
                if *which == 0 {
                    let mut ang = dy.atan2(dx);
                    while ang - tv > std::f64::consts::PI {
                        ang -= std::f64::consts::TAU;
                    }
                    while ang - tv < -std::f64::consts::PI {
                        ang += std::f64::consts::TAU;
                    }
                    vec![sym(*a as u32) - k(dx.hypot(dy).max(1e-6)), sym(*t as u32) - k(ang)]
                } else {
                    let bv = (-dx * tv.sin() + dy * tv.cos()).abs().max(1e-6);
                    vec![sym(*b as u32) - k(bv)]
                }
            }
            DragKind::Curve { e } => d.implicit(*e, &k(m[0]), &k(m[1])).into_iter().collect(),
            _ => vec![],
        }
    }

    pub fn drag_solve(&mut self, kind: &DragKind, m: [f64; 2]) {
        if self.dirty {
            self.rebuild();
        }
        let exprs = self.drag_exprs(kind, m);
        let sys = self.sys.as_ref().unwrap();
        let drags: Vec<DragEq> = exprs.iter().map(|e| sys.drag_eq(e)).collect();
        let mut vals = self.vals();
        if sys.solve(&mut vals, &drags, m) {
            self.store(&vals);
        }
    }

    // ------------------------------------------------------------ creation

    pub fn snap_pos(&self, s: &Snap) -> [f64; 2] {
        match s {
            Snap::Point(e) => self.doc.pval(*e),
            Snap::Curve(_, p) | Snap::Free(p) => *p,
        }
    }

    pub fn tangent_con(&self, a: usize, b: usize) -> Option<Con> {
        let d = &self.doc;
        let side = |l: usize, c: usize| {
            let (p, q) = d.line_pts(l);
            let (p, q) = (d.pval(p), d.pval(q));
            let (cc, _) = d.circ_val(c);
            let cr = (q[0] - p[0]) * (cc[1] - p[1]) - (q[1] - p[1]) * (cc[0] - p[0]);
            if cr < 0.0 { -1.0 } else { 1.0 }
        };
        if d.is_line(a) && d.is_circlelike(b) {
            return Some(Con::Tangent { a, b, s: side(a, b), aux: None });
        }
        if d.is_line(b) && d.is_circlelike(a) {
            return Some(Con::Tangent { a: b, b: a, s: side(b, a), aux: None });
        }
        if d.is_circlelike(a) && d.is_circlelike(b) {
            let (c1, r1) = d.circ_val(a);
            let (c2, r2) = d.circ_val(b);
            let dist = (c1[0] - c2[0]).hypot(c1[1] - c2[1]);
            let s = if (dist - (r1 + r2)).abs() <= (dist - (r1 - r2).abs()).abs() { 1.0 } else { -1.0 };
            return Some(Con::Tangent { a, b, s, aux: None });
        }
        if d.is_point(a) || d.is_point(b) || a == b {
            return None;
        }
        Some(Con::Tangent { a, b, s: 1.0, aux: None })
    }

    /// Initial contact point for a generic tangency: the closest pair of samples.
    /// Polylines of an entity in world coordinates: the drawn shape when on
    /// screen, otherwise a sampling of the geometry.
    fn world_polys(&self, e: usize) -> Vec<Vec<[f64; 2]>> {
        if let Some(p) = self.shapes.get(&e) {
            return p.iter().map(|pl| pl.iter().map(|q| self.view.to_w(*q)).collect()).collect();
        }
        let d = &self.doc;
        let n = 400;
        let lin = |lo: f64, hi: f64, k: usize| lo + (hi - lo) * k as f64 / n as f64;
        match d.kind(e) {
            Some(Ent::Line { a, b, .. }) => {
                let (pa, pb) = (d.pval(*a), d.pval(*b));
                vec![(0..=n).map(|k| {
                    let t = lin(-3.0, 4.0, k);
                    [pa[0] + (pb[0] - pa[0]) * t, pa[1] + (pb[1] - pa[1]) * t]
                }).collect()]
            }
            Some(Ent::Circle { .. }) | Some(Ent::Arc { .. }) => {
                let (c, r) = d.circ_val(e);
                vec![(0..=n).map(|k| {
                    let t = lin(0.0, std::f64::consts::TAU, k);
                    [c[0] + r * t.cos(), c[1] + r * t.sin()]
                }).collect()]
            }
            Some(Ent::Conic { kind, c, a, b, t }) => {
                let cc = d.pval(*c);
                let (a, b, t) = (d.params[*a].v, d.params[*b].v, d.params[*t].v);
                let (ct, st) = (t.cos(), t.sin());
                let tr = |u: f64, w: f64| [cc[0] + u * ct - w * st, cc[1] + u * st + w * ct];
                match kind {
                    ConicKind::Ellipse => vec![(0..=n).map(|k| {
                        let th = lin(0.0, std::f64::consts::TAU, k);
                        tr(a * th.cos(), b * th.sin())
                    }).collect()],
                    ConicKind::Hyperbola => [1.0, -1.0]
                        .iter()
                        .map(|sg| (0..=n).map(|k| {
                            let u = lin(-3.0, 3.0, k);
                            tr(sg * a * u.cosh(), b * u.sinh())
                        }).collect())
                        .collect(),
                    ConicKind::Parabola => vec![(0..=n).map(|k| {
                        let w = lin(-12.0 * a.abs().max(0.1), 12.0 * a.abs().max(0.1), k);
                        tr(w * w / (4.0 * a), w)
                    }).collect()],
                }
            }
            Some(Ent::Curve { src }) => {
                let Ok(def) = d.curve_def(src) else { return vec![] };
                let Some(f) = def.explicit else { return vec![] };
                let vals: Vec<f64> = d.params.iter().map(|p| p.v).collect();
                let mut out = vec![vec![]];
                for k in 0..=2000 {
                    let x = -25.0 + 50.0 * k as f64 / 2000.0;
                    let y = eval(&f, &|s| if s == X_SYM { x } else { vals.get(s as usize).copied().unwrap_or(f64::NAN) });
                    if y.is_finite() && y.abs() < 1e6 {
                        out.last_mut().unwrap().push([x, y]);
                    } else if !out.last().unwrap().is_empty() {
                        out.push(vec![]);
                    }
                }
                out
            }
            _ => vec![],
        }
    }

    /// Initial contact point for a generic tangency: the pair of samples that
    /// are close together and have parallel tangents.
    fn contact_guess(&self, a: usize, b: usize) -> [f64; 2] {
        let (pa, pb) = (self.world_polys(a), self.world_polys(b));
        let with_dirs = |polys: &[Vec<[f64; 2]>]| -> Vec<([f64; 2], [f64; 2])> {
            let mut v = Vec::new();
            for pl in polys {
                for i in 1..pl.len().saturating_sub(1) {
                    let d = [pl[i + 1][0] - pl[i - 1][0], pl[i + 1][1] - pl[i - 1][1]];
                    let l = d[0].hypot(d[1]);
                    if l > 0.0 {
                        v.push((pl[i], [d[0] / l, d[1] / l]));
                    }
                }
            }
            v
        };
        let (sa, sb) = (with_dirs(&pa), with_dirs(&pb));
        if sa.is_empty() || sb.is_empty() {
            return self.mouse_w;
        }
        let step_a = (sa.len() / 500).max(1);
        let step_b = (sb.len() / 500).max(1);
        let scale = {
            let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
            for (p, _) in sa.iter().chain(&sb) {
                for k in 0..2 {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
            (hi[0] - lo[0]).hypot(hi[1] - lo[1]).max(1e-9)
        };
        let mut best = (f64::INFINITY, self.mouse_w);
        for (p, dp) in sa.iter().step_by(step_a) {
            for (q, dq) in sb.iter().step_by(step_b) {
                let dist = (p[0] - q[0]).hypot(p[1] - q[1]);
                let mis = (dp[0] * dq[1] - dp[1] * dq[0]).abs();
                let score = dist / scale + 0.5 * mis;
                if score < best.0 {
                    best = (score, [(p[0] + q[0]) * 0.5, (p[1] + q[1]) * 0.5]);
                }
            }
        }
        best.1
    }

    pub fn add_constraint(&mut self, c: Con) -> bool {
        let c = if let Con::Tangent { a, b, s, aux: None } = c {
            let d = &self.doc;
            let special = (d.is_line(a) && d.is_circlelike(b)) || (d.is_circlelike(a) && d.is_circlelike(b));
            if special || d.tangent_contact(a, b).is_some() {
                Con::Tangent { a, b, s, aux: None }
            } else {
                let g = self.contact_guess(a, b);
                let tx = self.doc.add_param(g[0], false);
                let ty = self.doc.add_param(g[1], false);
                Con::Tangent { a, b, s, aux: Some((tx, ty)) }
            }
        } else {
            c
        };
        let ents = Doc::con_ents(&c);
        let ok = self.try_change(true, |d| d.cons.push(Some(c)));
        if !ok {
            self.flash_ents(ents);
        }
        ok
    }

    pub fn add_dimension(&mut self, kind: DimKind, label_w: [f64; 2]) -> Option<usize> {
        let anchor = self.dim_anchor_kind(&kind);
        let off = [label_w[0] - anchor[0], label_w[1] - anchor[1]];
        let supp = match kind {
            DimKind::Angle(l1, l2) => angle_supp(&self.doc, l1, l2, label_w),
            _ => false,
        };
        let make = |d: &mut Doc, driving: bool| -> usize {
            let i = d.add_dim(kind, off);
            if supp {
                d.dims[i].as_mut().unwrap().supp = true;
                let v = d.measure_value(i);
                let p = d.dims[i].as_ref().unwrap().param;
                d.params[p].v = v;
            }
            d.dims[i].as_mut().unwrap().driving = driving;
            i
        };
        let mut idx = None;
        let ok = self.try_change(true, |d| {
            idx = Some(make(d, true));
        });
        if !ok {
            let mut idx2 = None;
            self.try_change(false, |d| {
                idx2 = Some(make(d, false));
            });
            return idx2;
        }
        idx
    }

    pub fn dim_anchor_kind(&self, k: &DimKind) -> [f64; 2] {
        let d = &self.doc;
        let mid = |a: [f64; 2], b: [f64; 2]| [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
        match *k {
            DimKind::Distance(a, b) | DimKind::HDist(a, b) | DimKind::VDist(a, b) => mid(d.pval(a), d.pval(b)),
            DimKind::Length(l) => {
                let (a, b) = d.line_pts(l);
                mid(d.pval(a), d.pval(b))
            }
            DimKind::PointLine(p, l) => {
                let pp = d.pval(p);
                let f = foot(d, pp, l);
                mid(pp, f)
            }
            DimKind::Radius(c) | DimKind::Diameter(c) => d.circ_val(c).0,
            DimKind::Angle(l1, l2) => angle_vertex(d, l1, l2),
            DimKind::ConicA(c) | DimKind::ConicB(c) => match d.kind(c) {
                Some(Ent::Conic { c, .. }) => d.pval(*c),
                _ => [0.0; 2],
            },
            DimKind::CoordX(p) | DimKind::CoordY(p) => mid(d.pval(d.origin), d.pval(p)),
        }
    }

    /// Commits text typed for a dimension value.
    pub fn commit_dim_text(&mut self, i: usize, text: &str) {
        let Some(dim) = self.doc.dims[i].clone() else { return };
        let t = text.trim();
        if t.is_empty() {
            if dim.driving {
                self.try_change(false, |d| {
                    let x = d.dims[i].as_mut().unwrap();
                    x.driving = false;
                    x.expr.clear();
                });
            }
            return;
        }
        let angle = dim.kind.is_angle();
        if let Some(v) = plain_number(t) {
            let v = if angle { v.to_radians() } else { v };
            if !dim.driving || !dim.expr.is_empty() {
                let mv = self.doc.measure_value(i);
                let ok = self.try_change(true, |d| {
                    let s = d.dim_sign(d.dims[i].as_ref().unwrap());
                    let x = d.dims[i].as_mut().unwrap();
                    x.driving = true;
                    x.expr.clear();
                    x.s = s;
                    d.params[x.param].v = mv;
                });
                if !ok {
                    self.flash_ents(dim.kind.ents());
                    return;
                }
            }
            let p = dim.param;
            if !self.set_values(&[(p, v)], true) {
                self.flash_ents(dim.kind.ents());
            }
        } else {
            if let Err(_) = self.doc.parse_expr(t, 0) {
                self.flash_ents(dim.kind.ents());
                return;
            }
            let was_driving = dim.driving;
            let tt = t.to_string();
            let ok = self.try_change(!was_driving, |d| {
                let s = d.dim_sign(d.dims[i].as_ref().unwrap());
                let x = d.dims[i].as_mut().unwrap();
                if !x.driving {
                    x.s = s;
                }
                x.driving = true;
                x.expr = tt;
            });
            if !ok {
                // try reaching the new value gradually
                let before = self.doc.clone();
                let cur = self.doc.measure_value(i);
                self.doc.dims[i].as_mut().unwrap().driving = true;
                self.doc.dims[i].as_mut().unwrap().expr = t.to_string();
                let target = self.doc.dim_value_expr(i, 0).map(|e| self.doc.eval_now(&e)).unwrap_or(f64::NAN);
                self.doc = before;
                if target.is_finite() {
                    let p = dim.param;
                    let ok2 = self.try_change(!was_driving, |d| {
                        let s = d.dim_sign(d.dims[i].as_ref().unwrap());
                        let x = d.dims[i].as_mut().unwrap();
                        x.driving = true;
                        x.expr.clear();
                        x.s = s;
                        d.params[p].v = cur;
                    });
                    if ok2 && self.set_values(&[(p, target)], false) {
                        let tt = t.to_string();
                        if self.try_change(false, |d| d.dims[i].as_mut().unwrap().expr = tt) {
                            return;
                        }
                    }
                }
                self.flash_ents(dim.kind.ents());
            }
        }
    }

    pub fn toggle_driving(&mut self, i: usize) {
        let Some(dim) = self.doc.dims[i].clone() else { return };
        if dim.driving {
            self.try_change(false, |d| d.dims[i].as_mut().unwrap().driving = false);
        } else {
            let v = self.doc.measure_value(i);
            let ok = self.try_change(true, |d| {
                let s = d.dim_sign(d.dims[i].as_ref().unwrap());
                let x = d.dims[i].as_mut().unwrap();
                x.driving = true;
                x.s = s;
                if x.expr.is_empty() {
                    d.params[x.param].v = v;
                }
            });
            if !ok {
                self.flash_ents(dim.kind.ents());
            }
        }
    }

    pub fn delete_selection(&mut self) {
        let ents: Vec<usize> = self.sel.iter().filter_map(|s| if let Sel::Ent(e) = s { Some(*e) } else { None }).collect();
        let cons: Vec<usize> = self.sel.iter().filter_map(|s| if let Sel::Con(e) = s { Some(*e) } else { None }).collect();
        let dims: Vec<usize> = self.sel.iter().filter_map(|s| if let Sel::Dim(e) = s { Some(*e) } else { None }).collect();
        let vars: Vec<usize> = self.sel.iter().filter_map(|s| if let Sel::Var(e) = s { Some(*e) } else { None }).collect();
        if ents.is_empty() && cons.is_empty() && dims.is_empty() && vars.is_empty() {
            return;
        }
        self.try_change(false, |d| {
            d.delete(&ents, &cons, &dims);
            for v in vars {
                d.vars[v] = None;
            }
        });
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
        if self.drag.is_some() && self.formula.as_ref().is_some_and(|f| f.target == target) {
            // a long search for the construction before the drag is of no use
            if self.formula_job.as_ref().is_some_and(|j| j.0 .1 != self.generation) {
                self.cancel_formula_job();
            }
            return;
        }
        let level = self.search_level_of(target);
        let key = (target, self.generation, level);
        if self.formula_key == Some(key) {
            return;
        }
        self.cancel_formula_job();
        self.formula_key = Some(key);
        if self.formula.as_ref().is_some_and(|f| f.target != target) {
            self.formula = None;
        }
        let doc = self.doc.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = cancel.clone();
        std::thread::spawn(move || {
            crate::expr::set_cancel(Some(flag));
            let _ = tx.send(compute_formula(&doc, target, level));
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
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// The search level for `target` in the current construction.
    pub fn search_level_of(&self, target: Sel) -> u32 {
        match self.search_level {
            Some((t, g, n)) if t == target && g == self.generation => n,
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
        self.search_level = Some((target, self.generation, level + 1));
    }

    // ------------------------------------------------------------ files

    pub fn save(&mut self, ask: bool) {
        let path = if ask || self.path.is_none() {
            rfd::FileDialog::new().add_filter("gxpad", &["gxp"]).set_file_name("construction.gxp").save_file()
        } else {
            self.path.clone()
        };
        if let Some(p) = path {
            if let Ok(s) = serde_json::to_string_pretty(&self.doc) {
                if std::fs::write(&p, s).is_ok() {
                    self.path = Some(p);
                }
            }
        }
    }

    pub fn open(&mut self) {
        if let Some(p) = rfd::FileDialog::new().add_filter("gxpad", &["gxp"]).pick_file() {
            if let Ok(s) = std::fs::read_to_string(&p) {
                if let Ok(d) = serde_json::from_str::<Doc>(&s) {
                    let before = std::mem::replace(&mut self.doc, d);
                    self.push_undo(before);
                    self.path = Some(p);
                    self.sel.clear();
                    self.dirty = true;
                    self.fitted = false;
                    self.solve_full();
                }
            }
        }
    }
}

pub fn sym_name(names: &HashMap<usize, String>, s: u32) -> String {
    if s >= CSE_BASE {
        return format!("u_{}", s - CSE_BASE + 1);
    }
    names.get(&(s as usize)).cloned().unwrap_or_else(|| format!("p{s}"))
}

/// Expression whose formula is derived for a selected dimension or variable.
pub fn target_expr(doc: &Doc, t: Sel) -> Result<(String, Expr, bool), String> {
    match t {
        Sel::Dim(i) => {
            let d = doc.dims[i].as_ref().ok_or("gone")?;
            let e = if d.driving { doc.dim_value_expr(i, 0)? } else { doc.measure(i) };
            Ok((d.name.clone(), e, d.kind.is_angle()))
        }
        Sel::Var(i) => {
            let v = doc.vars[i].as_ref().ok_or("gone")?;
            let e = if v.expr.trim().is_empty() { sym(v.param as u32) } else { doc.parse_expr(&v.expr, 0)? };
            Ok((v.name.clone(), e, false))
        }
        _ => Err(String::new()),
    }
}

pub fn compute_formula(doc: &Doc, target: Sel, level: u32) -> FormulaView {
    let t0 = std::time::Instant::now();
    let names = doc.param_names();
    let nm = |s: u32| sym_name(&names, s);
    let mut fv = FormulaView {
        target,
        name: String::new(),
        bx: None,
        defs: vec![],
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
    match target_expr(doc, target) {
        Ok((name, e, ang)) => {
            fv.name = name.clone();
            fv.is_angle = ang;
            match derive::derive_harder(doc, &e, level as u64 * SEARCH_HARDER_MS) {
                Ok(d) => {
                    let (main, defs) = cse(&d.expr, CSE_BASE, 6);
                    fv.bx = Some(to_bx(&main, &nm));
                    fv.defs = defs.iter().map(|(s, e)| (nm(*s), to_bx(e, &nm))).collect();
                    let full = to_bx(&d.expr, &nm);
                    fv.plain = format!("{} = {}", name, plain(&full));
                    fv.latex = format!("{} = {}", latex(&Bx::Sym(name.clone())), latex(&full));
                    fv.exact = d.exact;
                    fv.size = complexity(&d.expr);
                    fv.value = d.value;
                    fv.actual = d.actual;
                }
                Err(m) => fv.err = Some(m),
            }
        }
        Err(m) => fv.err = Some(m),
    }
    fv.ms = t0.elapsed().as_secs_f64() * 1e3;
    fv
}

pub fn foot(d: &Doc, p: [f64; 2], l: usize) -> [f64; 2] {
    let (a, b) = d.line_pts(l);
    let (a, b) = (d.pval(a), d.pval(b));
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    if l2 < 1e-18 {
        return a;
    }
    let t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2;
    [a[0] + t * dx, a[1] + t * dy]
}

/// Whether a label at `w` sits in a sector measuring the supplementary angle
/// of the directed lines.
pub fn angle_supp(d: &Doc, l1: usize, l2: usize, w: [f64; 2]) -> bool {
    let x = angle_vertex(d, l1, l2);
    let dir = |l: usize| {
        let (a, b) = d.line_pts(l);
        let (a, b) = (d.pval(a), d.pval(b));
        [b[0] - a[0], b[1] - a[1]]
    };
    let (u, v) = (dir(l1), dir(l2));
    let q = [w[0] - x[0], w[1] - x[1]];
    let cr = |a: [f64; 2], b: [f64; 2]| a[0] * b[1] - a[1] * b[0];
    for s1 in [1.0, -1.0] {
        for s2 in [1.0, -1.0] {
            let a = [u[0] * s1, u[1] * s1];
            let b = [v[0] * s2, v[1] * s2];
            let den = cr(a, b);
            if den.abs() < 1e-15 {
                continue;
            }
            let ka = cr(q, b) / den;
            let kb = cr(a, q) / den;
            if ka >= 0.0 && kb >= 0.0 {
                return s1 * s2 < 0.0;
            }
        }
    }
    false
}

/// Intersection of two lines, or the midpoint of their midpoints when parallel.
pub fn angle_vertex(d: &Doc, l1: usize, l2: usize) -> [f64; 2] {
    let (a, b) = d.line_pts(l1);
    let (c, e) = d.line_pts(l2);
    let (a, b, c, e) = (d.pval(a), d.pval(b), d.pval(c), d.pval(e));
    let r = [b[0] - a[0], b[1] - a[1]];
    let s = [e[0] - c[0], e[1] - c[1]];
    let den = r[0] * s[1] - r[1] * s[0];
    if den.abs() < 1e-12 {
        return [(a[0] + b[0] + c[0] + e[0]) * 0.25, (a[1] + b[1] + c[1] + e[1]) * 0.25];
    }
    let t = ((c[0] - a[0]) * s[1] - (c[1] - a[1]) * s[0]) / den;
    [a[0] + t * r[0], a[1] + t * r[1]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_value() {
        let mut app = App::new();
        let (a, b, c);
        {
            let d = &mut app.doc;
            a = d.add_point(-2.67, -2.83);
            b = d.add_point(4.67, -2.83);
            c = d.add_point(-0.33, 2.83);
        }
        let mut ls = vec![];
        app.try_change(false, |d| {
            ls.push(d.add_ent(Ent::Line { a, b, infinite: false }));
            ls.push(d.add_ent(Ent::Line { a: b, b: c, infinite: false }));
            ls.push(d.add_ent(Ent::Line { a: c, b: a, infinite: false }));
            d.cons.push(Some(Con::Horizontal(a, b)));
        });
        let mut dims = vec![];
        for l in &ls {
            dims.push(app.add_dimension(DimKind::Length(*l), [0.0, -4.0]).unwrap());
        }
        println!("driving: {:?}", dims.iter().map(|i| app.doc.dims[*i].as_ref().unwrap().driving).collect::<Vec<_>>());
        app.commit_dim_text(dims[0], "5");
        let v = app.doc.measure_value(dims[0]);
        println!("after: {v} conflicts={:?} rank={} dof={}", app.conflicts, app.rank, app.dof);
        assert!((v - 5.0).abs() < 1e-6);
    }
}

#[cfg(test)]
mod demo {
    use super::*;

    fn save(app: &App, name: &str) {
        if let Ok(dir) = std::env::var("GXPAD_DEMO_DIR") {
            std::fs::write(format!("{dir}/{name}.gxp"), serde_json::to_string(&app.doc).unwrap()).unwrap();
        }
    }

    fn line(app: &mut App, a: usize, b: usize) -> usize {
        let mut l = 0;
        assert!(app.try_change(false, |d| l = d.add_ent(Ent::Line { a, b, infinite: false })));
        l
    }

    #[test]
    fn demo_tangent_circle() {
        let mut app = App::new();
        let c = app.doc.add_point(3.5, 2.0);
        let mut circ = 0;
        app.try_change(false, |d| circ = d.add_circle(c, 1.2));
        let q = app.doc.add_point(6.0, 4.5);
        let o = app.doc.origin;
        let l = line(&mut app, o, q);
        let xa = app.doc.xaxis;
        let t1 = app.tangent_con(xa, circ).unwrap();
        assert!(app.add_constraint(t1));
        let t2 = app.tangent_con(l, circ).unwrap();
        assert!(app.add_constraint(t2));
        app.add_dimension(DimKind::Radius(circ), [5.0, 0.5]).unwrap();
        app.add_dimension(DimKind::Angle(xa, l), [2.0, 0.6]).unwrap();
        app.add_dimension(DimKind::Length(l), [2.0, 4.0]).unwrap();
        let dd = app.add_dimension(DimKind::Distance(o, c), [1.5, -0.8]).unwrap();
        assert!(!app.doc.dims[dd].as_ref().unwrap().driving);
        assert_eq!(app.dof, 0);
        let f = compute_formula(&app.doc, Sel::Dim(dd), 0);
        let f = &f;
        println!("{}  exact={} ({} ms)", f.plain, f.exact, f.ms);
        assert!(f.exact);
        save(&app, "tangent");
    }

    #[test]
    fn demo_function_fit() {
        let mut app = App::new();
        app.try_change(false, |d| {
            d.add_var("k".into(), 1.0);
            d.add_var("c".into(), 0.0);
        });
        for v in app.doc.vars.iter().flatten() {
            let p = v.param;
            app.doc.params[p].locked = false;
        }
        let mut f = 0;
        app.try_change(false, |d| f = d.add_ent(Ent::Curve { src: "y = k x^2 + c".into() }));
        let p1 = app.doc.add_point(1.0, 1.0);
        let p2 = app.doc.add_point(2.0, 4.0);
        app.try_change(false, |d| {
            d.cons.push(Some(Con::OnCurve(p1, f)));
            d.cons.push(Some(Con::OnCurve(p2, f)));
        });
        let o = app.doc.origin;
        app.add_dimension(DimKind::HDist(o, p1), [0.5, -1.0]).unwrap();
        app.add_dimension(DimKind::VDist(o, p1), [-1.0, 0.5]).unwrap();
        app.add_dimension(DimKind::HDist(o, p2), [1.0, -1.6]).unwrap();
        app.add_dimension(DimKind::VDist(o, p2), [-1.6, 2.0]).unwrap();
        println!("dof={}", app.dof);
        println!("{}", compute_formula(&app.doc, Sel::Var(0), 0).plain);
        println!("{}", compute_formula(&app.doc, Sel::Var(1), 0).plain);
        save(&app, "function");
    }

    #[test]
    fn demo_conics() {
        let mut app = App::new();
        let c = app.doc.add_point(0.0, 0.0);
        let o = app.doc.origin;
        let mut e = 0;
        app.try_change(false, |d| {
            e = d.add_conic(ConicKind::Ellipse, c, 3.0, 1.6, 0.0);
            d.cons.push(Some(Con::Coincident(c, o)));
        });
        let ka = app.add_dimension(DimKind::ConicA(e), [1.5, 0.4]).unwrap();
        let kb = app.add_dimension(DimKind::ConicB(e), [-0.5, 1.0]).unwrap();
        let _ = (ka, kb);
        // ellipse axis aligned
        let t = match app.doc.kind(e) {
            Some(Ent::Conic { t, .. }) => *t,
            _ => unreachable!(),
        };
        app.doc.params[t].locked = true;
        // a line tangent to the ellipse through a fixed point
        let p = app.doc.add_point(5.0, 3.0);
        let q = app.doc.add_point(0.0, 3.0);
        let l = line(&mut app, p, q);
        app.try_change(false, |d| d.lock_point(p, true));
        let tc = app.tangent_con(l, e).unwrap();
        assert!(app.add_constraint(tc));
        let xa = app.doc.xaxis;
        let ang = app.add_dimension(DimKind::Angle(xa, l), [3.4, 2.4]).unwrap();
        app.doc.dims[ang].as_mut().unwrap().driving = false;
        app.dirty = true;
        app.solve_full();
        println!("dof={}", app.dof);
        let f = compute_formula(&app.doc, Sel::Dim(ang), 0);
        let f = &f;
        println!("{}  exact={} value={} actual={} ({:.2} ms)", f.plain, f.exact, f.value, f.actual, f.ms);
        let hp = app.doc.add_point(-6.0, -3.0);
        let pp = app.doc.add_point(5.0, -4.0);
        app.try_change(false, |d| {
            d.add_conic(ConicKind::Hyperbola, hp, 1.0, 0.8, 0.3);
            d.add_conic(ConicKind::Parabola, pp, 0.5, 1.0, 1.9);
            let ac = d.add_point(-5.0, 3.0);
            let a1 = d.add_point(-3.0, 3.0);
            let a2 = d.add_point(-5.0, 5.0);
            d.add_ent(Ent::Arc { c: ac, a: a1, b: a2 });
            d.add_ent(Ent::Curve { src: "x^2 y + y^3 = 2".into() });
        });
        save(&app, "conics");
    }
}

#[cfg(test)]
mod construct_tests {
    use super::*;

    /// Perpendicular bisector through hidden helper, as the tool builds it.
    fn bisector(d: &mut Doc, a: usize, b: usize) -> usize {
        let (pa, pb) = (d.pval(a), d.pval(b));
        let mid = [(pa[0] + pb[0]) * 0.5, (pa[1] + pb[1]) * 0.5];
        let m = d.add_point(mid[0], mid[1]);
        d.cons.push(Some(Con::MidpointOf(m, a, b)));
        let q = d.add_helper(mid[0] - (pb[1] - pa[1]), mid[1] + (pb[0] - pa[0]));
        d.cons.push(Some(Con::Offset { q, p: m, a, b, rot: true }));
        d.add_ent(Ent::Line { a: m, b: q, infinite: true })
    }

    #[test]
    fn circumcenter_from_coordinates() {
        let mut app = App::new();
        let o = app.doc.origin;
        let (b, c) = (app.doc.add_point(4.0, 0.3), app.doc.add_point(1.2, 3.1));
        let mut ls = (0, 0);
        assert!(app.try_change(false, |d| {
            d.add_ent(Ent::Line { a: o, b, infinite: false });
            d.add_ent(Ent::Line { a: b, b: c, infinite: false });
            d.add_ent(Ent::Line { a: c, b: o, infinite: false });
            ls = (bisector(d, o, b), bisector(d, b, c));
        }));
        // intersection of the bisectors
        let x = {
            let v = angle_vertex(&app.doc, ls.0, ls.1);
            let mut x = 0;
            assert!(app.try_change(true, |d| {
                x = d.add_point(v[0], v[1]);
                d.cons.push(Some(Con::OnCurve(x, ls.0)));
                d.cons.push(Some(Con::OnCurve(x, ls.1)));
            }));
            x
        };
        for k in [DimKind::CoordX(b), DimKind::CoordY(b), DimKind::CoordX(c), DimKind::CoordY(c)] {
            let i = app.add_dimension(k, [0.0, 0.0]).unwrap();
            assert!(app.doc.dims[i].as_ref().unwrap().driving);
        }
        assert_eq!(app.dof, 0, "fully constrained");
        // drive a coordinate to a negative value
        let yb = app.doc.dims.iter().position(|d| d.as_ref().is_some_and(|d| d.name == "y_A")).unwrap();
        app.commit_dim_text(yb, "-0.5");
        assert!((app.doc.pval(b)[1] + 0.5).abs() < 1e-9);
        let rx = app.add_dimension(DimKind::CoordX(x), [0.0, 0.0]).unwrap();
        let ry = app.add_dimension(DimKind::CoordY(x), [0.0, 0.0]).unwrap();
        assert!(!app.doc.dims[rx].as_ref().unwrap().driving);
        for i in [rx, ry] {
            let f = compute_formula(&app.doc, Sel::Dim(i), 0);
            println!("{}  exact={} value={} actual={}", f.plain, f.exact, f.value, f.actual);
            assert!(f.exact);
            assert!((f.value - f.actual).abs() < 1e-9);
        }
        // circumcenter is equidistant from all three vertices
        let pc = app.doc.pval(x);
        let dist = |p: [f64; 2]| (p[0] - pc[0]).hypot(p[1] - pc[1]);
        let (d0, d1, d2) = (dist([0.0, 0.0]), dist(app.doc.pval(b)), dist(app.doc.pval(c)));
        assert!((d0 - d1).abs() < 1e-9 && (d0 - d2).abs() < 1e-9);
    }

    #[test]
    fn tangent_from_point_and_angle_bisector() {
        let mut app = App::new();
        let o = app.doc.origin;
        let c = app.doc.add_point(4.0, 1.0);
        let p = app.doc.add_point(0.5, 3.0);
        let mut circ = 0;
        assert!(app.try_change(false, |d| circ = d.add_circle(c, 1.5)));
        let t = app.doc.add_point(3.0, 2.3);
        assert!(app.try_change(true, |d| {
            d.add_ent(Ent::Line { a: p, b: t, infinite: false });
            d.cons.push(Some(Con::TangentAt { p, t, c: circ }));
        }));
        let (pc, pt) = (app.doc.pval(c), app.doc.pval(t));
        let pp = app.doc.pval(p);
        let rad = [pt[0] - pc[0], pt[1] - pc[1]];
        let tan = [pt[0] - pp[0], pt[1] - pp[1]];
        assert!((rad[0] * tan[0] + rad[1] * tan[1]).abs() < 1e-8);
        assert!((rad[0].hypot(rad[1]) - app.doc.circ_val(circ).1).abs() < 1e-9);
        // angle bisector of the two axes through the origin: the line y = x
        let (xa, ya) = (app.doc.xaxis, app.doc.yaxis);
        assert!(app.try_change(false, |d| {
            let q = d.add_helper(1.0, 1.0);
            d.cons.push(Some(Con::Bisect { q, x: o, l1: xa, l2: ya, s1: 1.0, s2: 1.0 }));
            d.add_ent(Ent::Line { a: o, b: q, infinite: true });
        }));
        let q = app.doc.ents.iter().rposition(|e| e.as_ref().is_some_and(|e| e.hidden)).unwrap();
        let qv = app.doc.pval(q);
        assert!((qv[0] - qv[1]).abs() < 1e-9);
    }
}

#[cfg(test)]
mod tangent_contact_tests {
    use super::*;

    /// A segment from the center of one circle, ending on a second circle and
    /// tangent to it at that end point: its length is √3·r exactly.
    #[test]
    fn tangent_segment_ending_on_circle() {
        let mut app = App::new();
        let o = app.doc.origin;
        let e = app.doc.add_point(0.0, 0.0);
        let f = app.doc.add_point(-1.3, 1.5);
        let h = app.doc.add_point(0.34, 0.94);
        let a = app.doc.add_point(0.98, 0.19);
        let (mut ce, mut cf, mut seg, mut rl) = (0, 0, 0, 0);
        assert!(app.try_change(false, |d| {
            ce = d.add_circle(e, 1.0);
            cf = d.add_circle(f, 1.0);
            seg = d.add_ent(Ent::Line { a: f, b: h, infinite: false });
            rl = d.add_ent(Ent::Line { a: o, b: a, infinite: false });
            d.cons.push(Some(Con::Coincident(o, e)));
            d.cons.push(Some(Con::OnCurve(a, ce)));
            d.cons.push(Some(Con::Equal(cf, ce)));
            d.cons.push(Some(Con::OnCurve(h, ce)));
        }));
        let t1 = app.tangent_con(cf, ce).unwrap();
        assert!(app.add_constraint(t1));
        let t2 = app.tangent_con(seg, ce).unwrap();
        assert!(app.add_constraint(t2));
        let r = app.add_dimension(DimKind::Length(rl), [0.0, 0.0]).unwrap();
        app.commit_dim_text(r, "1");
        let dd = app.add_dimension(DimKind::Length(seg), [0.0, 0.0]).unwrap();
        assert!(!app.doc.dims[dd].as_ref().unwrap().driving);
        let v = app.doc.measure_value(dd);
        assert!((v - 3f64.sqrt()).abs() < 1e-10, "measured {v}");
        let fm = compute_formula(&app.doc, Sel::Dim(dd), 0);
        println!("{} ({:.2} ms)", fm.plain, fm.ms);
        assert!(fm.exact);
        assert!((fm.value - 3f64.sqrt()).abs() < 1e-10);
        assert_eq!(app.dof, 2);
    }
}

#[cfg(test)]
mod periodic_tests {
    use super::*;

    /// Circle through a zero of sin(x) at x = −π; horizontal segment tangent to
    /// sin(x) at its peak, ending on the circle: length π/2 + √(π² − 1).
    #[test]
    fn sine_peak_tangent_to_circle() {
        let mut app = App::new();
        let o = app.doc.origin;
        let xa = app.doc.xaxis;
        let (mut f, mut circ, mut seg) = (0, 0, 0);
        let dpt = app.doc.add_point(-3.1, 0.05);
        let b = app.doc.add_point(-2.9, 1.0);
        let c = app.doc.add_point(1.5, 1.0);
        assert!(app.try_change(false, |d| {
            f = d.add_ent(Ent::Curve { src: "y = sin(x)".into() });
            circ = d.add_circle(o, 3.1);
            seg = d.add_ent(Ent::Line { a: b, b: c, infinite: false });
            d.cons.push(Some(Con::OnCurve(dpt, f)));
            d.cons.push(Some(Con::OnCurve(dpt, xa)));
            d.cons.push(Some(Con::OnCurve(dpt, circ)));
            d.cons.push(Some(Con::OnCurve(c, f)));
            d.cons.push(Some(Con::OnCurve(b, circ)));
            d.cons.push(Some(Con::Horizontal(b, c)));
        }));
        let t = app.tangent_con(seg, f).unwrap();
        assert!(app.add_constraint(t));
        assert_eq!(app.dof, 0);
        let dd = app.add_dimension(DimKind::Length(seg), [0.0, 0.0]).unwrap();
        let fm = compute_formula(&app.doc, Sel::Dim(dd), 0);
        println!("{}", fm.plain);
        let want = std::f64::consts::FRAC_PI_2 + (std::f64::consts::PI.powi(2) - 1.0).sqrt();
        assert!(fm.exact, "{}", fm.plain);
        assert!((fm.value - want).abs() < 1e-12);
        assert!((app.doc.measure_value(dd) - want).abs() < 1e-9);
    }
}

#[cfg(test)]
mod search_harder_tests {
    use super::*;
    use crate::testkit::Kit;

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

    #[test]
    fn each_press_raises_the_level_until_the_target_changes() {
        let mut k = Kit::new();
        let a = k.point(0.0, 0.0);
        let b = k.point(3.0, 4.0);
        k.drive(DimKind::CoordX(a), "0");
        k.drive(DimKind::CoordY(a), "0");
        k.drive(DimKind::CoordX(b), "3");
        k.drive(DimKind::CoordY(b), "4");
        let d = k.measure(DimKind::Distance(a, b));
        let e = k.measure(DimKind::Distance(b, k.origin()));
        let app = &mut k.app;
        app.sel = vec![Sel::Dim(d)];
        wait_formula(app);
        assert_eq!(app.formula.as_ref().unwrap().level, 0);
        for level in 1..=2 {
            app.search_harder();
            wait_formula(app);
            let f = app.formula.as_ref().unwrap();
            assert_eq!(f.level, level);
            assert!(f.exact);
        }
        // another target starts over at the normal search
        app.sel = vec![Sel::Dim(e)];
        wait_formula(app);
        assert_eq!(app.formula.as_ref().unwrap().level, 0);
    }
}
