//! The interactive drawing canvas.

use crate::app::*;
use crate::expr::*;
use crate::icons::{self, Icon};
use crate::model::*;
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Painter, PointerButton, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};
use std::collections::{HashMap, HashSet};
use std::f64::consts::{PI, TAU};

const HIT_PT: f32 = 9.0;
const HIT_CURVE: f32 = 7.0;
const AXIS: Color32 = Color32::from_rgb(0x3a, 0x41, 0x4e);

/// Creates (or reuses) the point a snap refers to.
pub fn realize(d: &mut Doc, s: Snap) -> usize {
    match s {
        Snap::Point(e) => e,
        Snap::Free(p) => d.add_point(p[0], p[1]),
        Snap::Curve(c, p) => {
            let e = d.add_point(p[0], p[1]);
            d.cons.push(Some(Con::OnCurve(e, c)));
            e
        }
    }
}

fn seg_dist(p: Pos2, a: Pos2, b: Pos2) -> (f32, Pos2) {
    let ab = b - a;
    let l2 = ab.length_sq();
    let t = if l2 > 0.0 { ((p - a).dot(ab) / l2).clamp(0.0, 1.0) } else { 0.0 };
    let q = a + ab * t;
    (p.distance(q), q)
}

pub fn nearest_on(polys: &[Vec<Pos2>], p: Pos2) -> (f32, Pos2) {
    let mut best = (f32::INFINITY, p);
    for pl in polys {
        if pl.len() == 1 {
            let d = p.distance(pl[0]);
            if d < best.0 {
                best = (d, pl[0]);
            }
        }
        for w in pl.windows(2) {
            let r = seg_dist(p, w[0], w[1]);
            if r.0 < best.0 {
                best = r;
            }
        }
    }
    best
}

fn finite(p: Pos2) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

/// Buttons only report clicks on the first pass of a frame.
fn sense_click(ui: &Ui) -> Sense {
    if ui.ctx().current_pass_index() == 0 { Sense::click() } else { Sense::hover() }
}

pub fn icon_btn(ui: &mut Ui, icon: Icon, active: bool, tip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(34.0, 34.0), sense_click(ui));
    let hov = resp.hovered();
    let bg = if active {
        Color32::from_rgba_unmultiplied(0x6c, 0xb6, 0xff, 46)
    } else if hov {
        SURFACE_HI
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect.shrink(2.0), 8.0, bg);
    let col = if active { ACCENT } else if hov { FULL } else { ICON };
    icons::paint(ui.painter(), rect.shrink(8.0), icon, col);
    resp.on_hover_text(tip)
}

pub fn small_icon_btn(ui: &mut Ui, icon: Icon, active: bool, tip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(24.0, 24.0), sense_click(ui));
    let hov = resp.hovered();
    if hov {
        ui.painter().rect_filled(rect, 6.0, SURFACE_HI);
    }
    let col = if active { ACCENT } else if hov { FULL } else { MUTED };
    icons::paint(ui.painter(), rect.shrink(6.0), icon, col);
    resp.on_hover_text(tip)
}

#[derive(Clone)]
enum Act {
    Con(Con),
    ConChain(u8),
    Dim(DimKind),
    Lock(bool),
    Construction,
    Delete,
    Driving(usize),
    Supp(usize),
    Infinite,
}

impl App {
    // ------------------------------------------------------------ geometry tessellation

    fn view_radius_w(&self) -> f64 {
        let r = self.view.rect;
        (r.width().hypot(r.height()) as f64 * 0.5) / self.view.s
    }

    fn build_shapes(&mut self, cache: &mut HashMap<usize, (u64, Vec<Vec<Pos2>>)>) {
        self.shapes.clear();
        let v = self.view;
        let vr = self.view_radius_w();
        for i in 0..self.geo.doc.ents.len() {
            let Some(e) = self.geo.doc.ents[i].clone() else { continue };
            if e.hidden {
                continue;
            }
            let polys: Vec<Vec<Pos2>> = match &e.kind {
                Ent::Point { .. } | Ent::Line { .. } | Ent::Circle { .. } | Ent::Arc { .. } | Ent::Conic { .. } => geom_polys(&self.geo.doc, v, vr, i),
                Ent::Curve { src } => {
                    let used: Vec<f64> = self.geo.doc.params.iter().map(|p| p.v).collect();
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    use std::hash::{Hash, Hasher};
                    src.hash(&mut h);
                    for x in [v.c[0], v.c[1], v.s, v.rect.width() as f64, v.rect.height() as f64, v.rect.min.x as f64, v.rect.min.y as f64] {
                        x.to_bits().hash(&mut h);
                    }
                    if let Ok(def) = self.geo.doc.curve_def(src) {
                        for s in sym_set(&def.implicit) {
                            if let Some(x) = used.get(s as usize) {
                                x.to_bits().hash(&mut h);
                            }
                        }
                    }
                    let key = h.finish();
                    if let Some((k, p)) = cache.get(&i) {
                        if *k == key {
                            self.shapes.insert(i, p.clone());
                            continue;
                        }
                    }
                    let p = self.curve_polys(src);
                    cache.insert(i, (key, p.clone()));
                    p
                }
            };
            self.shapes.insert(i, polys);
        }
    }

    fn curve_polys(&self, src: &str) -> Vec<Vec<Pos2>> {
        let Ok(def) = self.geo.doc.curve_def(src) else { return vec![] };
        let v = self.view;
        let r = v.rect;
        let vals: Vec<f64> = self.geo.doc.params.iter().map(|p| p.v).collect();
        let mut st = Vec::new();
        if let Some(f) = &def.explicit {
            let code = Code::new(f);
            let mut out: Vec<Vec<Pos2>> = vec![];
            let mut cur: Vec<Pos2> = vec![];
            let n = (r.width() / 1.5) as usize + 2;
            let mut prev: Option<Pos2> = None;
            for k in 0..=n {
                let sx = r.min.x - 2.0 + (r.width() + 4.0) * k as f32 / n as f32;
                let x = v.to_w(pos2(sx, 0.0))[0];
                let y = code.eval(&|s: u32| if s == X_SYM { x } else { vals.get(s as usize).copied().unwrap_or(f64::NAN) }, &mut st);
                if !y.is_finite() {
                    if cur.len() > 1 {
                        out.push(std::mem::take(&mut cur));
                    }
                    cur.clear();
                    prev = None;
                    continue;
                }
                let p = v.to_s([x, y]);
                let p = pos2(p.x, p.y.clamp(r.min.y - 4000.0, r.max.y + 4000.0));
                if let Some(q) = prev {
                    if (p.y - q.y).abs() > r.height() * 1.5 && (p.y < r.min.y || p.y > r.max.y || q.y < r.min.y || q.y > r.max.y) {
                        if cur.len() > 1 {
                            out.push(std::mem::take(&mut cur));
                        }
                        cur.clear();
                    }
                }
                cur.push(p);
                prev = Some(p);
            }
            if cur.len() > 1 {
                out.push(cur);
            }
            return out;
        }
        // marching squares on the implicit form
        let code = Code::new(&def.implicit);
        let cell = 7.0f32;
        let nx = (r.width() / cell) as usize + 2;
        let ny = (r.height() / cell) as usize + 2;
        let mut g = vec![0.0f64; (nx + 1) * (ny + 1)];
        for j in 0..=ny {
            for i in 0..=nx {
                let w = v.to_w(pos2(r.min.x + i as f32 * cell, r.min.y + j as f32 * cell));
                g[j * (nx + 1) + i] = code.eval(
                    &|s: u32| {
                        if s == X_SYM {
                            w[0]
                        } else if s == Y_SYM {
                            w[1]
                        } else {
                            vals.get(s as usize).copied().unwrap_or(f64::NAN)
                        }
                    },
                    &mut st,
                );
            }
        }
        let mut segs = Vec::new();
        let at = |i: usize, j: usize| pos2(r.min.x + i as f32 * cell, r.min.y + j as f32 * cell);
        for j in 0..ny {
            for i in 0..nx {
                let c = [g[j * (nx + 1) + i], g[j * (nx + 1) + i + 1], g[(j + 1) * (nx + 1) + i + 1], g[(j + 1) * (nx + 1) + i]];
                if c.iter().any(|x| !x.is_finite()) {
                    continue;
                }
                let corners = [at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)];
                let mut pts = Vec::new();
                for k in 0..4 {
                    let (a, b) = (c[k], c[(k + 1) % 4]);
                    if (a < 0.0) != (b < 0.0) {
                        let t = (a / (a - b)) as f32;
                        // reject sign changes caused by poles
                        if (a - b).abs() > 1e6 * (a.abs().min(b.abs()) + 1.0) && a.abs().max(b.abs()) > 1e3 {
                            continue;
                        }
                        pts.push(corners[k] + (corners[(k + 1) % 4] - corners[k]) * t);
                    }
                }
                if pts.len() == 2 {
                    segs.push(vec![pts[0], pts[1]]);
                } else if pts.len() == 4 {
                    segs.push(vec![pts[0], pts[1]]);
                    segs.push(vec![pts[2], pts[3]]);
                }
            }
        }
        join_segments(segs)
    }

    // ------------------------------------------------------------ hit testing

    pub fn hit(&self, m: Pos2) -> Option<Sel> {
        for (r, d) in self.dim_rects.iter().rev() {
            if r.expand(2.0).contains(m) {
                return Some(Sel::Dim(*d));
            }
        }
        if self.show_cons {
            for (r, c) in self.glyphs.iter().rev() {
                if r.contains(m) {
                    return Some(Sel::Con(*c));
                }
            }
        }
        let mut best: Option<(f32, usize)> = None;
        for (e, polys) in &self.shapes {
            if !self.geo.doc.is_point(*e) {
                continue;
            }
            let d = polys[0][0].distance(m);
            if d < HIT_PT && best.is_none_or(|b| d < b.0) {
                best = Some((d, *e));
            }
        }
        if let Some((_, e)) = best {
            return Some(Sel::Ent(e));
        }
        let mut best: Option<(f32, usize)> = None;
        for (e, polys) in &self.shapes {
            if self.geo.doc.is_point(*e) {
                continue;
            }
            let (d, _) = nearest_on(polys, m);
            let thr = if *e == self.geo.doc.xaxis || *e == self.geo.doc.yaxis { HIT_CURVE * 0.7 } else { HIT_CURVE };
            if d < thr && best.is_none_or(|b| d < b.0 || (d - b.0).abs() < 0.5 && *e > b.1) {
                best = Some((d, *e));
            }
        }
        best.map(|(_, e)| Sel::Ent(e))
    }

    fn hit_handle(&self, m: Pos2) -> Option<(usize, u8)> {
        self.handles.iter().find(|(p, _, _)| p.distance(m) < HIT_PT).map(|(_, e, w)| (*e, *w))
    }

    pub fn snap_at_pub(&self, m: Pos2) -> Snap {
        self.snap_at(m, &[])
    }

    fn snap_at(&self, m: Pos2, exclude: &[usize]) -> Snap {
        let mut best: Option<(f32, usize)> = None;
        for (e, polys) in &self.shapes {
            if !self.geo.doc.is_point(*e) || exclude.contains(e) {
                continue;
            }
            let d = polys[0][0].distance(m);
            if d < HIT_PT + 2.0 && best.is_none_or(|b| d < b.0) {
                best = Some((d, *e));
            }
        }
        if let Some((_, e)) = best {
            return Snap::Point(e);
        }
        let mut best: Option<(f32, usize, Pos2)> = None;
        for (e, polys) in &self.shapes {
            if self.geo.doc.is_point(*e) {
                continue;
            }
            let (d, q) = nearest_on(polys, m);
            if d < HIT_CURVE && best.is_none_or(|b| d < b.0) {
                best = Some((d, *e, q));
            }
        }
        if let Some((_, e, q)) = best {
            let mut w = self.view.to_w(q);
            if let Some(Ent::Line { a, b, .. }) = self.geo.doc.kind(e) {
                let (pa, pb) = (self.geo.doc.pval(*a), self.geo.doc.pval(*b));
                let mw = self.view.to_w(m);
                let d = [pb[0] - pa[0], pb[1] - pa[1]];
                let l2 = (d[0] * d[0] + d[1] * d[1]).max(1e-18);
                let t = ((mw[0] - pa[0]) * d[0] + (mw[1] - pa[1]) * d[1]) / l2;
                w = [pa[0] + t * d[0], pa[1] + t * d[1]];
            }
            return Snap::Curve(e, w);
        }
        Snap::Free(self.view.to_w(m))
    }

    // ------------------------------------------------------------ view

    pub fn fit(&mut self) {
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        for (i, e) in self.geo.doc.ents.iter().enumerate() {
            if let Some(e) = e {
                if e.hidden {
                    continue;
                }
                let pts: Vec<[f64; 2]> = match &e.kind {
                    Ent::Point { .. } => vec![self.geo.doc.pval(i)],
                    Ent::Circle { .. } | Ent::Arc { .. } => {
                        let (c, r) = self.geo.doc.circ_val(i);
                        vec![[c[0] - r, c[1] - r], [c[0] + r, c[1] + r]]
                    }
                    Ent::Conic { c, a, b, .. } => {
                        let cc = self.geo.doc.pval(*c);
                        let r = self.geo.doc.params[*a].v.abs().max(self.geo.doc.params[*b].v.abs());
                        vec![[cc[0] - r, cc[1] - r], [cc[0] + r, cc[1] + r]]
                    }
                    _ => vec![],
                };
                for p in pts {
                    for k in 0..2 {
                        lo[k] = lo[k].min(p[k]);
                        hi[k] = hi[k].max(p[k]);
                    }
                }
            }
        }
        if !lo[0].is_finite() {
            return;
        }
        let w = (hi[0] - lo[0]).max(1.0);
        let h = (hi[1] - lo[1]).max(1.0);
        let r = self.view.rect;
        self.view.c = [(lo[0] + hi[0]) * 0.5, (lo[1] + hi[1]) * 0.5];
        self.view.s = ((r.width() as f64 - 220.0) / w).min((r.height() as f64 - 160.0) / h).clamp(2.0, 2000.0);
    }

    fn view_input(&mut self, ui: &Ui, resp: &Response) {
        let (scroll, zoom, hover) = ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta(), i.pointer.hover_pos()));
        if resp.hovered() {
            if let Some(m) = hover {
                let before = self.view.to_w(m);
                let mut f = zoom as f64;
                if scroll.y != 0.0 && zoom == 1.0 {
                    f = (scroll.y as f64 * 0.0025).exp();
                }
                if f != 1.0 {
                    self.view.s = (self.view.s * f).clamp(0.5, 50000.0);
                    let after = self.view.to_w(m);
                    self.view.c[0] += before[0] - after[0];
                    self.view.c[1] += before[1] - after[1];
                }
                if scroll.x != 0.0 && zoom == 1.0 {
                    self.view.c[0] -= scroll.x as f64 / self.view.s;
                }
            }
        }
        if resp.dragged_by(PointerButton::Middle) || resp.dragged_by(PointerButton::Secondary) {
            let d = resp.drag_delta();
            self.view.c[0] -= d.x as f64 / self.view.s;
            self.view.c[1] += d.y as f64 / self.view.s;
        }
    }

    // ------------------------------------------------------------ main canvas

    pub fn canvas_ui(&mut self, ui: &mut Ui, cache: &mut HashMap<usize, (u64, Vec<Vec<Pos2>>)>) {
        let rect = ui.max_rect();
        let resp = ui.allocate_rect(rect, Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        self.view.rect = rect;
        if !self.fitted && rect.width() > 100.0 {
            self.fitted = true;
            if self.geo.doc.ents.iter().flatten().filter(|e| !e.hidden).count() > 3 {
                self.fit();
            }
        }
        self.now = ui.input(|i| i.time);
        if self.live {
            self.view_input(ui, &resp);
        }
        self.build_shapes(cache);
        let mouse = ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p));
        if let Some(m) = mouse {
            self.mouse_w = self.view.to_w(m);
        }
        self.hover = None;
        self.hover_handle = None;
        if resp.hovered() && self.drag.is_none() {
            if let Some(m) = mouse {
                self.hover_handle = self.hit_handle(m);
                if self.hover_handle.is_none() {
                    self.hover = self.hit(m);
                }
            }
        }
        match self.tool {
            _ if !self.live => {}
            t if crate::construct::is_construct(t) => self.construct_input(&resp, mouse),
            Tool::Select => self.select_input(ui, &resp, mouse),
            Tool::Dimension => self.dim_input(ui, &resp, mouse),
            Tool::Curve => {
                if resp.clicked() {
                    self.sel.clear();
                }
            }
            _ => self.draw_input(ui, &resp, mouse),
        }
        if self.live && resp.clicked() {
            self.flyout = None;
        }
        if self.live && resp.secondary_clicked() {
            if !self.pending.is_empty() || !self.picks.is_empty() {
                self.pending.clear();
                self.picks.clear();
            } else if self.tool != Tool::Select {
                self.tool = Tool::Select;
            }
        }
        let cursor = match (self.tool, self.hover, self.hover_handle) {
            (Tool::Select, _, Some(_)) => egui::CursorIcon::Grab,
            (Tool::Select, Some(Sel::Ent(_) | Sel::Dim(_)), _) => egui::CursorIcon::PointingHand,
            (Tool::Select, _, _) => egui::CursorIcon::Default,
            _ => egui::CursorIcon::Crosshair,
        };
        if resp.hovered() {
            ui.ctx().set_cursor_icon(if self.drag.is_some() { egui::CursorIcon::Grabbing } else { cursor });
        }
        self.paint(&painter, mouse);
        self.overlays(ui, rect);
    }

    fn select_input(&mut self, ui: &Ui, resp: &Response, mouse: Option<Pos2>) {
        let shift = ui.input(|i| i.modifiers.shift || i.modifiers.command);
        if resp.drag_started_by(PointerButton::Primary) {
            let p = ui.input(|i| i.pointer.press_origin()).or(mouse).unwrap_or_default();
            let pw = self.view.to_w(p);
            let mut kind = None;
            if let Some((e, w)) = self.hit_handle(p) {
                kind = Some(DragKind::Handle { e, which: w });
            } else {
                match self.hit(p) {
                    Some(Sel::Dim(d)) => {
                        let dim = self.geo.doc.dims[d].as_ref().unwrap();
                        let a = self.geo.doc.dim_anchor(&dim.kind);
                        let l = [a[0] + dim.off[0], a[1] + dim.off[1]];
                        kind = Some(DragKind::DimLabel { d, grab: [pw[0] - l[0], pw[1] - l[1]] });
                    }
                    Some(Sel::Ent(e)) => kind = self.drag_kind_for(e, pw),
                    _ => {}
                }
            }
            match kind {
                Some(k) => {
                    self.drag_backup = Some(self.geo.doc.clone());
                    self.drag = Some(k);
                }
                None => {
                    self.drag = Some(DragKind::Box { start: p });
                }
            }
        }
        if resp.drag_started_by(PointerButton::Middle) || resp.drag_started_by(PointerButton::Secondary) {
            self.drag = Some(DragKind::Pan);
        }
        if let Some(k) = self.drag.clone() {
            if resp.dragged() {
                match &k {
                    DragKind::Pan => {}
                    DragKind::Box { start } => {
                        if let Some(m) = mouse.or(ui.input(|i| i.pointer.hover_pos())) {
                            self.box_sel = Some((*start, m));
                        }
                    }
                    DragKind::DimLabel { d, grab } => {
                        if let Some(dim) = self.geo.doc.dims[*d].clone() {
                            let a = self.geo.doc.dim_anchor(&dim.kind);
                            let m = self.mouse_w;
                            let lw = [m[0] - grab[0], m[1] - grab[1]];
                            let x = self.geo.doc.dims[*d].as_mut().unwrap();
                            x.off = [lw[0] - a[0], lw[1] - a[1]];
                            if let (DimKind::Angle(l1, l2), false) = (dim.kind, dim.driving) {
                                let supp = angle_supp(&self.geo.doc, l1, l2, lw);
                                if supp != dim.supp {
                                    self.geo.doc.dims[*d].as_mut().unwrap().supp = supp;
                                    self.geo.generation += 1;
                                }
                            }
                        }
                    }
                    _ => {
                        let m = self.mouse_w;
                        self.drag_solve(&k, m);
                    }
                }
            }
            if resp.drag_stopped() {
                match k {
                    DragKind::Box { .. } => {
                        if let Some((a, b)) = self.box_sel.take() {
                            let r = Rect::from_two_pos(a, b);
                            if !shift {
                                self.sel.clear();
                            }
                            for (e, polys) in &self.shapes {
                                if self.geo.doc.is_fixed_ent(*e) {
                                    continue;
                                }
                                let all = polys.iter().flatten().all(|p| r.contains(*p));
                                if all && !polys.is_empty() && !self.sel.contains(&Sel::Ent(*e)) {
                                    self.sel.push(Sel::Ent(*e));
                                }
                            }
                            for (rr, d) in &self.dim_rects {
                                if r.contains_rect(*rr) && !self.sel.contains(&Sel::Dim(*d)) {
                                    self.sel.push(Sel::Dim(*d));
                                }
                            }
                        }
                    }
                    DragKind::Pan => {}
                    _ => {
                        if let Some(b) = self.drag_backup.take() {
                            self.geo.push_undo(b);
                        }
                        self.geo.solve_full();
                    }
                }
                self.drag = None;
                self.box_sel = None;
            }
        }
        if resp.clicked() {
            match self.hover {
                Some(s) => {
                    if shift {
                        if let Some(i) = self.sel.iter().position(|x| *x == s) {
                            self.sel.remove(i);
                        } else {
                            self.sel.push(s);
                        }
                    } else {
                        self.sel = vec![s];
                    }
                }
                None => {
                    if !shift {
                        self.sel.clear();
                    }
                }
            }
        }
        if resp.double_clicked() {
            if let Some(Sel::Dim(d)) = self.hover {
                self.open_dim_editor(d);
            }
        }
    }

    pub fn open_dim_editor(&mut self, d: usize) {
        if let Some(dim) = &self.geo.doc.dims[d] {
            let text = if dim.expr.is_empty() {
                let v = if dim.driving { self.geo.doc.params[dim.param].v } else { self.geo.doc.measure_value(d) };
                fmt_dim(v, dim.kind.is_angle()).trim_end_matches('°').to_string()
            } else {
                dim.expr.clone()
            };
            self.edit = Some(DimEdit { dim: d, text, focus: true });
        }
    }

    fn drag_kind_for(&self, e: usize, pw: [f64; 2]) -> Option<DragKind> {
        let d = &self.geo.doc;
        let unlocked_pts = |pts: Vec<usize>| -> Vec<(usize, [f64; 2])> {
            pts.into_iter().filter(|p| !d.point_locked(*p)).map(|p| (p, d.pval(p))).collect()
        };
        match d.kind(e)? {
            Ent::Point { .. } => {
                if d.point_locked(e) {
                    return None;
                }
                let p = d.pval(e);
                Some(DragKind::Point { e, grab: [pw[0] - p[0], pw[1] - p[1]] })
            }
            Ent::Line { a, b, .. } => {
                let pts = unlocked_pts(vec![*a, *b]);
                if pts.is_empty() {
                    return None;
                }
                Some(DragKind::Translate { pts, start: pw })
            }
            Ent::Circle { r, c } => {
                if !d.params[*r].locked {
                    Some(DragKind::CircleEdge { e })
                } else {
                    Some(DragKind::Translate { pts: unlocked_pts(vec![*c]), start: pw })
                }
            }
            Ent::Arc { c, a, b } => Some(DragKind::Translate { pts: unlocked_pts(vec![*c, *a, *b]), start: pw }),
            Ent::Conic { c, .. } => Some(DragKind::Translate { pts: unlocked_pts(vec![*c]), start: pw }),
            Ent::Curve { .. } => Some(DragKind::Curve { e }),
        }
    }

    fn draw_input(&mut self, ui: &Ui, resp: &Response, mouse: Option<Pos2>) {
        self.hv_hint = None;
        let Some(m) = mouse else { return };
        let mut snap = self.snap_at(m, &[]);
        if self.tool == Tool::Line && self.pending.len() == 1 {
            if let Snap::Free(p) = snap {
                let s = self.snap_pos(&self.pending[0]);
                let (dx, dy) = (p[0] - s[0], p[1] - s[1]);
                let ang = dy.atan2(dx).to_degrees().rem_euclid(180.0);
                let tol = 3.0;
                if (dx.hypot(dy) * self.view.s) > 12.0 {
                    if ang < tol || ang > 180.0 - tol {
                        snap = Snap::Free([p[0], s[1]]);
                        self.hv_hint = Some(true);
                    } else if (ang - 90.0).abs() < tol {
                        snap = Snap::Free([s[0], p[1]]);
                        self.hv_hint = Some(false);
                    }
                }
            }
        }
        let _ = ui;
        if resp.clicked() {
            self.tool_click(snap);
        }
    }

    /// The line under the cursor, for the directrix click of the
    /// parabola-by-focus tool.
    fn directrix_snap(&self, s: Snap) -> Option<Snap> {
        match s {
            Snap::Curve(l, _) if self.geo.doc.is_line(l) => Some(s),
            _ => match self.hover {
                Some(Sel::Ent(l)) if self.geo.doc.is_line(l) => Some(Snap::Curve(l, self.mouse_w)),
                _ => None,
            },
        }
    }

    /// The shape the next click would complete, built on a copy of the
    /// document, as screen polylines.
    fn shape_ghost(&self, last: Snap) -> Vec<Vec<Pos2>> {
        let mut snaps = self.pending.clone();
        snaps.push(last);
        let mut d = self.geo.doc.clone();
        match build_shape(&mut d, self.tool, &snaps) {
            Some(e) => geom_polys(&d, self.view, self.view_radius_w(), e),
            None => vec![],
        }
    }

    fn tool_click(&mut self, s: Snap) {
        let need = match self.tool {
            Tool::Point => 1,
            Tool::Line | Tool::Circle | Tool::Parabola | Tool::ParabolaFocus => 2,
            Tool::Arc | Tool::Ellipse | Tool::Hyperbola | Tool::Circle3 | Tool::Arc3 | Tool::EllipseFoci | Tool::HyperbolaFoci => 3,
            _ => return,
        };
        let s = if self.tool == Tool::ParabolaFocus && self.pending.len() == 1 {
            match self.directrix_snap(s) {
                Some(l) => l,
                None => return,
            }
        } else {
            s
        };
        if self.pending.len() + 1 == need && matches!(self.tool, Tool::Circle3 | Tool::Arc3 | Tool::EllipseFoci | Tool::HyperbolaFoci | Tool::ParabolaFocus) {
            // only commit clicks that determine a shape
            let mut snaps = self.pending.clone();
            snaps.push(s);
            let mut trial = self.geo.doc.clone();
            if build_shape(&mut trial, self.tool, &snaps).is_none() {
                let ents: Vec<usize> = snaps.iter().filter_map(|x| if let Snap::Point(e) | Snap::Curve(e, _) = x { Some(*e) } else { None }).collect();
                self.flash_ents(ents);
                return;
            }
        }
        self.pending.push(s);
        if self.pending.len() < need {
            return;
        }
        let p: Vec<Snap> = std::mem::take(&mut self.pending);
        let pos: Vec<[f64; 2]> = p.iter().map(|s| self.snap_pos(s)).collect();
        let hv = self.hv_hint;
        let tool = self.tool;
        let mut chain: Option<usize> = None;
        let mut created: Option<usize> = None;
        let ok = self.geo.try_change(false, |d| match tool {
            Tool::Point => {
                if !matches!(p[0], Snap::Point(_)) {
                    created = Some(realize(d, p[0]));
                }
            }
            Tool::Line => {
                let a = realize(d, p[0]);
                let b = realize(d, p[1]);
                if a == b {
                    return;
                }
                created = Some(d.add_ent(Ent::Line { a, b, infinite: false }));
                match hv {
                    Some(true) => d.cons.push(Some(Con::Horizontal(a, b))),
                    Some(false) => d.cons.push(Some(Con::Vertical(a, b))),
                    None => {}
                }
                if !matches!(p[1], Snap::Point(_)) {
                    chain = Some(b);
                }
            }
            Tool::Circle => {
                let c = realize(d, p[0]);
                let ci = d.build_circle_to(c, pos[1]);
                created = Some(ci);
                match p[1] {
                    Snap::Point(e) => d.cons.push(Some(Con::OnCurve(e, ci))),
                    Snap::Curve(..) => {
                        let e = realize(d, p[1]);
                        d.cons.push(Some(Con::OnCurve(e, ci)));
                    }
                    _ => {}
                }
            }
            Tool::Arc => {
                let c = realize(d, p[0]);
                let a = realize(d, p[1]);
                let b = match p[2] {
                    Snap::Point(e) => e,
                    _ => d.build_arc_end(c, a, pos[2]),
                };
                if a != b {
                    created = Some(d.add_ent(Ent::Arc { c, a, b }));
                }
            }
            Tool::Ellipse | Tool::Hyperbola => {
                let c = realize(d, p[0]);
                let kind = if tool == Tool::Ellipse { ConicKind::Ellipse } else { ConicKind::Hyperbola };
                created = Some(d.build_conic_from(kind, c, pos[1], pos[2]));
            }
            Tool::Parabola => {
                let c = realize(d, p[0]);
                created = Some(d.build_parabola_toward(c, pos[1]));
            }
            Tool::Circle3 | Tool::Arc3 | Tool::EllipseFoci | Tool::HyperbolaFoci | Tool::ParabolaFocus => {
                created = build_shape(d, tool, &p);
            }
            _ => {}
        });
        if ok {
            if let Some(b) = chain {
                self.pending.push(Snap::Point(b));
            }
            if let Some(e) = created {
                if tool != Tool::Point {
                    let _ = e;
                }
            }
        }
    }

    fn dim_kind_for(&self, ents: &[usize]) -> Option<DimKind> {
        let d = &self.geo.doc;
        match ents {
            [p] if d.is_point(*p) => {
                let q = d.pval(*p);
                let m = self.mouse_w;
                if (m[1] - q[1]).abs() > (m[0] - q[0]).abs() { Some(DimKind::CoordX(*p)) } else { Some(DimKind::CoordY(*p)) }
            }
            [l] if d.is_line(*l) => Some(DimKind::Length(*l)),
            [c] if d.is_circlelike(*c) => Some(DimKind::Radius(*c)),
            [c] if d.is_conic(*c) => Some(DimKind::ConicA(*c)),
            [a, b] if d.is_point(*a) && d.is_point(*b) => Some(DimKind::Distance(*a, *b)),
            [p, l] | [l, p] if d.is_point(*p) && d.is_line(*l) => Some(DimKind::PointLine(*p, *l)),
            [l1, l2] if d.is_line(*l1) && d.is_line(*l2) => {
                if self.lines_parallel(*l1, *l2) {
                    Some(DimKind::PointLine(d.line_pts(*l2).0, *l1))
                } else {
                    Some(DimKind::Angle(*l1, *l2))
                }
            }
            _ => None,
        }
    }

    fn lines_parallel(&self, l1: usize, l2: usize) -> bool {
        let d = &self.geo.doc;
        let dir = |l: usize| {
            let (a, b) = d.line_pts(l);
            let (a, b) = (d.pval(a), d.pval(b));
            [b[0] - a[0], b[1] - a[1]]
        };
        let (u, v) = (dir(l1), dir(l2));
        let cr = u[0] * v[1] - u[1] * v[0];
        cr.abs() < 1e-6 * u[0].hypot(u[1]) * v[0].hypot(v[1])
    }

    fn sel_ents(&self) -> Vec<usize> {
        self.sel.iter().filter_map(|s| if let Sel::Ent(e) = s { Some(*e) } else { None }).collect()
    }

    fn dim_input(&mut self, ui: &Ui, resp: &Response, _mouse: Option<Pos2>) {
        let _ = ui;
        if !resp.clicked() {
            return;
        }
        match self.hover {
            Some(Sel::Ent(e)) if !self.geo.doc.ent(e).is_some_and(|x| x.hidden) => {
                if let Some(i) = self.sel.iter().position(|s| *s == Sel::Ent(e)) {
                    self.sel.remove(i);
                } else {
                    self.sel.retain(|s| matches!(s, Sel::Ent(_)));
                    if self.sel.len() >= 2 {
                        self.sel.clear();
                    }
                    self.sel.push(Sel::Ent(e));
                }
            }
            Some(Sel::Dim(d)) => {
                self.open_dim_editor(d);
            }
            _ => {
                let ents = self.sel_ents();
                if let Some(k) = self.dim_kind_for(&ents) {
                    if let Some(i) = self.geo.add_dimension(k, self.mouse_w) {
                        self.sel = vec![Sel::Dim(i)];
                        if self.geo.doc.dims[i].as_ref().is_some_and(|d| d.driving) {
                            self.open_dim_editor(i);
                        }
                    }
                } else {
                    self.sel.clear();
                }
            }
        }
    }

    // ------------------------------------------------------------ painting

    fn conflict_ents(&self) -> HashSet<usize> {
        let mut s = HashSet::new();
        for o in &self.geo.conflicts {
            match o {
                Owner::Con(c) => {
                    if let Some(c) = &self.geo.doc.cons[*c] {
                        s.extend(Doc::con_ents(c));
                    }
                }
                Owner::Dim(d) => {
                    if let Some(d) = &self.geo.doc.dims[*d] {
                        s.extend(d.kind.ents());
                    }
                }
                Owner::Ent(e) => {
                    s.insert(*e);
                }
            }
        }
        s
    }

    fn fully_constrained(&self, e: usize) -> bool {
        self.geo.doc.ent_all_params(e).iter().all(|p| !self.geo.free_params.contains(p))
    }

    fn curve_index(&self, e: usize) -> usize {
        self.geo.doc.ents[..e].iter().flatten().filter(|x| matches!(x.kind, Ent::Curve { .. })).count()
    }

    fn ent_color(&self, e: usize, conflicts: &HashSet<usize>) -> Color32 {
        if self.now < self.flash_until && self.flash.contains(&e) {
            return ERR;
        }
        if self.sel.contains(&Sel::Ent(e)) || self.picks.contains(&e) {
            return SEL;
        }
        if conflicts.contains(&e) {
            return ERR;
        }
        if e == self.geo.doc.xaxis || e == self.geo.doc.yaxis {
            return AXIS;
        }
        if self.geo.doc.is_curve_ent(e) {
            return CURVES[self.curve_index(e) % CURVES.len()];
        }
        if self.fully_constrained(e) { FULL } else { ACCENT }
    }

    fn paint_grid(&self, p: &Painter) {
        let r = self.view.rect;
        p.rect_filled(r, 0.0, BG);
        let s = self.view.s;
        let raw = 28.0 / s;
        let k = raw.log10().floor();
        let base = 10f64.powf(k);
        let step = [1.0, 2.0, 5.0, 10.0].iter().map(|m| m * base).find(|x| *x >= raw).unwrap_or(10.0 * base);
        let major = step * 5.0;
        let lo = self.view.to_w(r.left_bottom());
        let hi = self.view.to_w(r.right_top());
        let minor_c = Color32::from_rgba_unmultiplied(255, 255, 255, 6);
        let major_c = Color32::from_rgba_unmultiplied(255, 255, 255, 13);
        let mut x = (lo[0] / step).floor() * step;
        while x <= hi[0] {
            let is_major = ((x / major).round() * major - x).abs() < step * 0.01;
            let sx = self.view.to_s([x, 0.0]).x;
            p.line_segment([pos2(sx, r.min.y), pos2(sx, r.max.y)], Stroke::new(1.0, if is_major { major_c } else { minor_c }));
            x += step;
        }
        let mut y = (lo[1] / step).floor() * step;
        while y <= hi[1] {
            let is_major = ((y / major).round() * major - y).abs() < step * 0.01;
            let sy = self.view.to_s([0.0, y]).y;
            p.line_segment([pos2(r.min.x, sy), pos2(r.max.x, sy)], Stroke::new(1.0, if is_major { major_c } else { minor_c }));
            y += step;
        }
        // tick labels on the axes
        let font = FontId::proportional(10.0);
        let lc = Color32::from_rgba_unmultiplied(160, 168, 180, 70);
        let o = self.view.to_s([0.0, 0.0]);
        let ly = o.y.clamp(r.min.y + 4.0, r.max.y - 16.0);
        let mut x = (lo[0] / major).floor() * major;
        while x <= hi[0] {
            if x.abs() > major * 1e-6 {
                let sx = self.view.to_s([x, 0.0]).x;
                p.text(pos2(sx + 3.0, ly + 2.0), Align2::LEFT_TOP, fmt_val(x), font.clone(), lc);
            }
            x += major;
        }
        let lx = o.x.clamp(r.min.x + 4.0, r.max.x - 40.0);
        let mut y = (lo[1] / major).floor() * major;
        while y <= hi[1] {
            if y.abs() > major * 1e-6 {
                let sy = self.view.to_s([0.0, y]).y;
                p.text(pos2(lx + 3.0, sy - 2.0), Align2::LEFT_BOTTOM, fmt_val(y), font.clone(), lc);
            }
            y += major;
        }
    }

    fn stroke_polys(&self, p: &Painter, polys: &[Vec<Pos2>], col: Color32, w: f32, dashed: bool, glow: bool) {
        for pl in polys {
            let pl: Vec<Pos2> = pl.iter().copied().filter(|q| finite(*q)).collect();
            if pl.len() < 2 {
                continue;
            }
            if glow {
                p.add(Shape::line(pl.clone(), Stroke::new(w + 6.0, col.gamma_multiply(0.18))));
            }
            if dashed {
                p.extend(Shape::dashed_line(&pl, Stroke::new(w * 0.8, col.gamma_multiply(0.75)), 6.0, 4.0));
            } else {
                p.add(Shape::line(pl, Stroke::new(w, col)));
            }
        }
    }

    fn paint(&mut self, p: &Painter, mouse: Option<Pos2>) {
        self.paint_grid(p);
        let conflicts = self.conflict_ents();
        let hovered_con_ents: Vec<usize> = match self.hover {
            Some(Sel::Con(c)) => self.geo.doc.cons[c].as_ref().map(Doc::con_ents).unwrap_or_default(),
            Some(Sel::Dim(d)) => self.geo.doc.dims[d].as_ref().map(|d| d.kind.ents()).unwrap_or_default(),
            _ => vec![],
        };
        let mut ids: Vec<usize> = self.shapes.keys().copied().collect();
        ids.sort();
        // curves first, then points on top
        for &e in &ids {
            if self.geo.doc.is_point(e) {
                continue;
            }
            let ent = self.geo.doc.ent(e).unwrap();
            let col = self.ent_color(e, &conflicts);
            let hov = self.hover == Some(Sel::Ent(e)) || hovered_con_ents.contains(&e);
            let is_axis = e == self.geo.doc.xaxis || e == self.geo.doc.yaxis;
            let w = if is_axis { 1.2 } else if hov { 2.6 } else { 2.0 };
            let col = if hov && !self.sel.contains(&Sel::Ent(e)) && !is_axis { brighten(col) } else { col };
            let dashed = ent.construction && !is_axis;
            self.stroke_polys(p, &self.shapes[&e], col, w, dashed, hov && !is_axis);
            if is_axis && hov {
                self.stroke_polys(p, &self.shapes[&e], ACCENT.gamma_multiply(0.6), 1.6, false, false);
            }
        }
        // conic handles and asymptotes
        self.handles.clear();
        for &e in &ids {
            if let Some(Ent::Conic { kind, c, a, b, t }) = self.geo.doc.kind(e).cloned() {
                let show = self.sel.contains(&Sel::Ent(e)) || self.hover == Some(Sel::Ent(e)) || matches!(self.drag, Some(DragKind::Handle { e: de, .. }) if de == e);
                let cc = self.geo.doc.pval(c);
                let (av, bv, tv) = (self.geo.doc.params[a].v, self.geo.doc.params[b].v, self.geo.doc.params[t].v);
                let (ct, st) = (tv.cos(), tv.sin());
                let h0 = self.view.to_s([cc[0] + av * ct, cc[1] + av * st]);
                let h1 = self.view.to_s([cc[0] - bv * st, cc[1] + bv * ct]);
                self.handles.push((h0, e, 0));
                if kind != ConicKind::Parabola {
                    self.handles.push((h1, e, 1));
                }
                if show {
                    let col = self.ent_color(e, &conflicts).gamma_multiply(0.5);
                    let cs = self.view.to_s(cc);
                    if kind == ConicKind::Hyperbola {
                        let ext = self.view_radius_w() * 2.0;
                        for sg in [1.0, -1.0] {
                            let d = [av * ct - sg * bv * st, av * st + sg * bv * ct];
                            let l = d[0].hypot(d[1]).max(1e-12);
                            let u = [d[0] / l * ext, d[1] / l * ext];
                            p.extend(Shape::dashed_line(
                                &[self.view.to_s([cc[0] - u[0], cc[1] - u[1]]), self.view.to_s([cc[0] + u[0], cc[1] + u[1]])],
                                Stroke::new(1.0, col.gamma_multiply(0.6)),
                                5.0,
                                5.0,
                            ));
                        }
                    }
                    p.line_segment([cs, h0], Stroke::new(1.0, col));
                    if kind != ConicKind::Parabola {
                        p.line_segment([cs, h1], Stroke::new(1.0, col));
                    }
                    if kind == ConicKind::Parabola {
                        // directrix
                        let dd = self.view_radius_w() * 2.0;
                        let q = [cc[0] - av * ct, cc[1] - av * st];
                        p.extend(Shape::dashed_line(
                            &[self.view.to_s([q[0] - st * dd, q[1] + ct * dd]), self.view.to_s([q[0] + st * dd, q[1] - ct * dd])],
                            Stroke::new(1.0, col.gamma_multiply(0.6)),
                            5.0,
                            5.0,
                        ));
                    }
                    for (i, h) in [h0, h1].iter().enumerate() {
                        if i == 1 && kind == ConicKind::Parabola {
                            continue;
                        }
                        let hov = self.hover_handle == Some((e, i as u8));
                        let r = if hov { 5.5 } else { 4.0 };
                        let pts = vec![*h + vec2(0.0, -r), *h + vec2(r, 0.0), *h + vec2(0.0, r), *h + vec2(-r, 0.0)];
                        p.add(Shape::convex_polygon(pts, BG, Stroke::new(1.5, SEL)));
                    }
                }
            }
        }
        self.paint_extensions(p, &conflicts);
        // tangent contact points
        for c in self.geo.doc.cons.iter().flatten() {
            if let Con::Tangent { a, b, aux: Some((x, y)), .. } = c {
                if self.geo.doc.tangent_contact(*a, *b).is_some() {
                    continue;
                }
                let q = self.view.to_s([self.geo.doc.params[*x].v, self.geo.doc.params[*y].v]);
                p.circle_stroke(q, 2.5, Stroke::new(1.0, MUTED));
            }
        }
        self.paint_dims(p);
        if self.show_cons {
            self.paint_glyphs(p);
        } else {
            self.glyphs.clear();
        }
        for &e in &ids {
            if !self.geo.doc.is_point(e) {
                continue;
            }
            let ent = self.geo.doc.ent(e).unwrap();
            let q = self.shapes[&e][0][0];
            let col = self.ent_color(e, &conflicts);
            let hov = self.hover == Some(Sel::Ent(e)) || hovered_con_ents.contains(&e);
            let sel = self.sel.contains(&Sel::Ent(e));
            if e == self.geo.doc.origin {
                p.circle_stroke(q, 4.5, Stroke::new(1.4, if sel { SEL } else if hov { FULL } else { AXIS.gamma_multiply(1.8) }));
                p.circle_filled(q, 1.6, if sel { SEL } else { MUTED });
                continue;
            }
            let r = if hov || sel { 4.6 } else { 3.6 };
            if hov {
                p.circle_filled(q, r + 5.0, col.gamma_multiply(0.18));
            }
            if self.geo.doc.point_locked(e) {
                let rr = Rect::from_center_size(q, vec2(r * 2.0, r * 2.0));
                p.rect_filled(rr.expand(1.5), 1.5, BG);
                p.rect_filled(rr, 1.0, col);
            } else {
                p.circle_filled(q, r + 1.5, BG);
                p.circle_filled(q, r, col);
            }
            if hov || sel {
                p.text(q + vec2(8.0, -8.0), Align2::LEFT_BOTTOM, &ent.name, FontId::proportional(12.0), col.gamma_multiply(0.85));
            }
        }
        self.paint_preview(p, mouse);
        if let Some((a, b)) = self.box_sel {
            let r = Rect::from_two_pos(a, b);
            p.rect_filled(r, 2.0, Color32::from_rgba_unmultiplied(0x6c, 0xb6, 0xff, 18));
            p.rect_stroke(r, 2.0, Stroke::new(1.0, ACCENT.gamma_multiply(0.7)), StrokeKind::Inside);
        }
    }

    /// Dashed continuation of a segment up to points constrained onto its line.
    fn paint_extensions(&self, p: &Painter, conflicts: &HashSet<usize>) {
        let d = &self.geo.doc;
        let ext = |l: usize, q: [f64; 2]| {
            let Some(Ent::Line { a, b, infinite: false }) = d.kind(l) else { return };
            let (pa, pb) = (d.pval(*a), d.pval(*b));
            let dir = [pb[0] - pa[0], pb[1] - pa[1]];
            let l2 = dir[0] * dir[0] + dir[1] * dir[1];
            if l2 < 1e-18 {
                return;
            }
            let t = ((q[0] - pa[0]) * dir[0] + (q[1] - pa[1]) * dir[1]) / l2;
            if (0.0..=1.0).contains(&t) {
                return;
            }
            let from = if t < 0.0 { pa } else { pb };
            let to = [pa[0] + dir[0] * t, pa[1] + dir[1] * t];
            let col = self.ent_color(l, conflicts).gamma_multiply(0.45);
            p.extend(Shape::dashed_line(&[self.view.to_s(from), self.view.to_s(to)], Stroke::new(1.0, col), 4.0, 4.0));
        };
        for c in d.cons.iter().flatten() {
            match c {
                Con::OnCurve(pt, l) if d.is_line(*l) => ext(*l, d.pval(*pt)),
                Con::Tangent { a, b, aux: Some((x, y)), .. } => {
                    let q = [d.params[*x].v, d.params[*y].v];
                    for e in [*a, *b] {
                        if d.is_line(e) {
                            ext(e, q);
                        }
                    }
                }
                Con::Tangent { a, b, aux: None, .. } if d.is_line(*a) && d.is_circlelike(*b) => {
                    let (cc, _) = d.circ_val(*b);
                    ext(*a, foot(d, cc, *a));
                }
                Con::Symmetric(_, _, l) | Con::Midpoint(_, l) => {
                    let _ = l;
                }
                _ => {}
            }
        }
    }

    fn paint_preview(&self, p: &Painter, mouse: Option<Pos2>) {
        let Some(m) = mouse else { return };
        let ghost = Stroke::new(1.5, ACCENT.gamma_multiply(0.7));
        let v = self.view;
        let mw = self.mouse_w;
        if matches!(self.tool, Tool::Select | Tool::Curve) {
            return;
        }
        if self.tool == Tool::Dimension {
            let ents = self.sel_ents();
            if let Some(k) = self.dim_kind_for(&ents) {
                let a = self.geo.doc.dim_anchor(&k);
                let tmp = Dim { kind: k, name: String::new(), driving: true, expr: String::new(), param: 0, s: 1.0, supp: false, off: [mw[0] - a[0], mw[1] - a[1]] };
                self.paint_dim_shape(p, &tmp, None, DIMC.gamma_multiply(0.6), "");
            }
            return;
        }
        // snap indicator
        let snap = self.snap_at(m, &[]);
        if crate::construct::is_construct(self.tool) {
            let needs_point = match self.tool {
                Tool::Midpoint | Tool::PerpBisector | Tool::TangentLine | Tool::Reflect => self.pending.is_empty() || matches!(self.tool, Tool::Midpoint | Tool::PerpBisector),
                Tool::PerpLine | Tool::ParLine => !self.picks.is_empty(),
                _ => false,
            };
            if needs_point {
                match snap {
                    Snap::Point(e) => {
                        p.circle_stroke(self.view.to_s(self.geo.doc.pval(e)), 8.0, Stroke::new(1.5, SEL));
                    }
                    Snap::Curve(_, w) => {
                        p.circle_stroke(self.view.to_s(w), 5.0, Stroke::new(1.5, SEL));
                    }
                    Snap::Free(_) => {}
                }
            }
            self.paint_construct_preview(p, m);
            return;
        }
        let sp = match snap {
            Snap::Point(e) => {
                let q = v.to_s(self.geo.doc.pval(e));
                p.circle_stroke(q, 8.0, Stroke::new(1.5, SEL));
                q
            }
            Snap::Curve(_, w) => {
                let q = v.to_s(w);
                p.circle_stroke(q, 5.0, Stroke::new(1.5, SEL));
                p.circle_filled(q, 2.0, SEL);
                q
            }
            Snap::Free(_) => m,
        };
        let pend: Vec<[f64; 2]> = self.pending.iter().map(|s| self.snap_pos(s)).collect();
        let cur_w = match (self.tool, self.hv_hint, pend.first()) {
            (Tool::Line, Some(true), Some(s)) => [mw[0], s[1]],
            (Tool::Line, Some(false), Some(s)) => [s[0], mw[1]],
            _ => v.to_w(sp),
        };
        let cur = v.to_s(cur_w);
        match (self.tool, pend.len()) {
            (Tool::Line, 1) => {
                p.line_segment([v.to_s(pend[0]), cur], ghost);
                if let Some(h) = self.hv_hint {
                    let mid = v.to_s([(pend[0][0] + cur_w[0]) * 0.5, (pend[0][1] + cur_w[1]) * 0.5]);
                    let r = Rect::from_center_size(mid + if h { vec2(0.0, -14.0) } else { vec2(14.0, 0.0) }, vec2(16.0, 16.0));
                    p.rect_filled(r, 4.0, SURFACE_HI);
                    icons::paint(p, r.shrink(3.0), if h { Icon::Horizontal } else { Icon::Vertical }, ACCENT);
                }
            }
            (Tool::Circle, 1) | (Tool::Arc, 1) => {
                let c = v.to_s(pend[0]);
                p.circle_stroke(c, c.distance(cur), ghost);
                p.line_segment([c, cur], Stroke::new(1.0, ACCENT.gamma_multiply(0.4)));
            }
            (Tool::Arc, 2) => {
                let c = pend[0];
                let a = pend[1];
                let r = (a[0] - c[0]).hypot(a[1] - c[1]);
                let a0 = (a[1] - c[1]).atan2(a[0] - c[0]);
                let mut a1 = (cur_w[1] - c[1]).atan2(cur_w[0] - c[0]);
                while a1 <= a0 {
                    a1 += TAU;
                }
                let pts: Vec<Pos2> = (0..=64).map(|k| {
                    let t = a0 + (a1 - a0) * k as f64 / 64.0;
                    v.to_s([c[0] + r * t.cos(), c[1] + r * t.sin()])
                }).collect();
                p.add(Shape::line(pts, ghost));
            }
            (Tool::Ellipse | Tool::Hyperbola, 1) | (Tool::Parabola, 1) => {
                p.line_segment([v.to_s(pend[0]), cur], ghost);
            }
            (Tool::Ellipse | Tool::Hyperbola, 2) => {
                let c = pend[0];
                let (dx, dy) = (pend[1][0] - c[0], pend[1][1] - c[1]);
                let a = dx.hypot(dy);
                let t = dy.atan2(dx);
                let b = (-(cur_w[0] - c[0]) * t.sin() + (cur_w[1] - c[1]) * t.cos()).abs().max(a * 0.05);
                let (ct, st) = (t.cos(), t.sin());
                let tr = |u: f64, w: f64| v.to_s([c[0] + u * ct - w * st, c[1] + u * st + w * ct]);
                if self.tool == Tool::Ellipse {
                    let pts: Vec<Pos2> = (0..=128).map(|k| {
                        let th = TAU * k as f64 / 128.0;
                        tr(a * th.cos(), b * th.sin())
                    }).collect();
                    p.add(Shape::line(pts, ghost));
                } else {
                    for sg in [1.0, -1.0] {
                        let pts: Vec<Pos2> = (0..=64).map(|k| {
                            let u = -2.5 + 5.0 * k as f64 / 64.0;
                            tr(sg * a * u.cosh(), b * u.sinh())
                        }).collect();
                        p.add(Shape::line(pts, ghost));
                    }
                }
            }
            _ => {}
        }
        match (self.tool, pend.len()) {
            (Tool::Circle3 | Tool::Arc3, 1) => {
                p.line_segment([v.to_s(pend[0]), cur], Stroke::new(1.0, ACCENT.gamma_multiply(0.4)));
            }
            (Tool::EllipseFoci | Tool::HyperbolaFoci, 1) => {
                p.extend(Shape::dashed_line(&[v.to_s(pend[0]), cur], Stroke::new(1.0, ACCENT.gamma_multiply(0.4)), 4.0, 4.0));
            }
            (Tool::Circle3 | Tool::Arc3 | Tool::EllipseFoci | Tool::HyperbolaFoci, 2) | (Tool::ParabolaFocus, 1) => {
                let snap = self.snap_at(m, &[]);
                let last = if self.tool == Tool::ParabolaFocus { self.directrix_snap(snap) } else { Some(snap) };
                if let Some(last) = last {
                    for pl in self.shape_ghost(last) {
                        p.add(Shape::line(pl, ghost));
                    }
                }
            }
            _ => {}
        }
        if self.tool == Tool::Parabola && pend.len() == 1 {
            let c = pend[0];
            let (dx, dy) = (cur_w[0] - c[0], cur_w[1] - c[1]);
            let a = dx.hypot(dy).max(1e-6);
            let t = dy.atan2(dx);
            let (ct, st) = (t.cos(), t.sin());
            let pts: Vec<Pos2> = (0..=128).map(|k| {
                let w = (-1.0 + 2.0 * k as f64 / 128.0) * a * 6.0;
                let u = w * w / (4.0 * a);
                v.to_s([c[0] + u * ct - w * st, c[1] + u * st + w * ct])
            }).collect();
            p.add(Shape::line(pts, ghost));
        }
        for q in &pend {
            p.circle_filled(v.to_s(*q), 3.5, ACCENT);
        }
    }

    // ------------------------------------------------------------ dimensions

    fn paint_dims(&mut self, p: &Painter) {
        self.dim_rects.clear();
        for i in 0..self.geo.doc.dims.len() {
            let Some(d) = self.geo.doc.dims[i].clone() else { continue };
            if self.edit.as_ref().is_some_and(|e| e.dim == i) {
                // label is replaced by the editor, but still draw geometry
            }
            let sel = self.sel.contains(&Sel::Dim(i));
            let hov = self.hover == Some(Sel::Dim(i));
            let err = self.geo.conflicts.contains(&Owner::Dim(i)) || self.geo.errors.contains_key(&Owner::Dim(i));
            let col = if self.now < self.flash_until && d.kind.ents().iter().all(|e| self.flash.contains(e)) {
                ERR
            } else if sel {
                SEL
            } else if err {
                ERR
            } else if d.driving {
                DIMC
            } else {
                MUTED
            };
            let col = if hov && !sel { brighten(col) } else { col };
            let v = self.geo.doc.measure_value(i);
            let mut txt = fmt_dim(v, d.kind.is_angle());
            if d.kind == DimKind::Diameter(match d.kind { DimKind::Diameter(c) => c, _ => 0 }) {
                txt = format!("⌀{txt}");
            }
            if !d.driving {
                txt = format!("({txt})");
            } else if !d.expr.is_empty() {
                txt = format!("{} = {}", d.expr, txt);
            }
            if let Some(r) = self.paint_dim_shape(p, &d, Some(&d.name), col, &txt) {
                self.dim_rects.push((r, i));
            }
        }
    }

    /// Draws the dimension geometry and label; returns the label rect.
    fn paint_dim_shape(&self, p: &Painter, d: &Dim, name: Option<&String>, col: Color32, txt: &str) -> Option<Rect> {
        let v = self.view;
        let doc = &self.geo.doc;
        let anchor = self.geo.doc.dim_anchor(&d.kind);
        let lw = [anchor[0] + d.off[0], anchor[1] + d.off[1]];
        let lp = v.to_s(lw);
        let st = Stroke::new(1.1, col);
        let thin = Stroke::new(0.8, col.gamma_multiply(0.6));
        let arrow = |tip: Pos2, dir: Vec2| {
            let dn = dir.normalized();
            if !dn.x.is_finite() {
                return;
            }
            let n = vec2(-dn.y, dn.x);
            let l = 8.0;
            p.add(Shape::convex_polygon(vec![tip, tip - dn * l + n * 3.0, tip - dn * l - n * 3.0], col, Stroke::NONE));
        };
        let linear = |a: [f64; 2], b: [f64; 2], dir: [f64; 2]| {
            // dimension line parallel to `dir` passing through the label
            let l = dir[0].hypot(dir[1]).max(1e-12);
            let u = [dir[0] / l, dir[1] / l];
            let n = [-u[1], u[0]];
            let h = (lw[0] - a[0]) * n[0] + (lw[1] - a[1]) * n[1];
            let hb = (lw[0] - b[0]) * n[0] + (lw[1] - b[1]) * n[1];
            let pa = [a[0] + n[0] * h, a[1] + n[1] * h];
            let pb = [b[0] + n[0] * hb, b[1] + n[1] * hb];
            let (sa, sb) = (v.to_s(pa), v.to_s(pb));
            let gap = 4.0;
            for (from, to) in [(v.to_s(a), sa), (v.to_s(b), sb)] {
                let dd = to - from;
                if dd.length() > gap + 1.0 {
                    p.line_segment([from + dd.normalized() * gap, to + dd.normalized() * 5.0], thin);
                }
            }
            p.line_segment([sa, sb], st);
            arrow(sa, sa - sb);
            arrow(sb, sb - sa);
            // leader from the dimension line to the label if it sits outside
            let t = (lw[0] - pa[0]) * u[0] + (lw[1] - pa[1]) * u[1];
            let len = (pb[0] - pa[0]) * u[0] + (pb[1] - pa[1]) * u[1];
            if t < 0.0 || t > len {
                let q = if t < 0.0 { sa } else { sb };
                p.line_segment([q, lp], thin);
            }
        };
        match d.kind {
            DimKind::Distance(a, b) => {
                let (pa, pb) = (doc.pval(a), doc.pval(b));
                linear(pa, pb, [pb[0] - pa[0], pb[1] - pa[1]]);
            }
            DimKind::Length(l) => {
                let (a, b) = doc.line_pts(l);
                let (pa, pb) = (doc.pval(a), doc.pval(b));
                linear(pa, pb, [pb[0] - pa[0], pb[1] - pa[1]]);
            }
            DimKind::HDist(a, b) => linear(doc.pval(a), doc.pval(b), [1.0, 0.0]),
            DimKind::CoordX(p) => linear(doc.pval(doc.origin), doc.pval(p), [1.0, 0.0]),
            DimKind::CoordY(p) => linear(doc.pval(doc.origin), doc.pval(p), [0.0, 1.0]),
            DimKind::VDist(a, b) => linear(doc.pval(a), doc.pval(b), [0.0, 1.0]),
            DimKind::PointLine(pt, l) => {
                let pp = doc.pval(pt);
                let f = foot(doc, pp, l);
                let (sp, sf) = (v.to_s(pp), v.to_s(f));
                p.line_segment([sp, sf], st);
                arrow(sp, sp - sf);
                arrow(sf, sf - sp);
                // extend the line to the foot if beyond the segment
                let (a, b) = doc.line_pts(l);
                let (sa, sb) = (v.to_s(doc.pval(a)), v.to_s(doc.pval(b)));
                let near = if sa.distance(sf) < sb.distance(sf) { sa } else { sb };
                p.extend(Shape::dashed_line(&[near, sf], thin, 3.0, 3.0));
                p.line_segment([pos2((sp.x + sf.x) * 0.5, (sp.y + sf.y) * 0.5), lp], thin);
            }
            DimKind::Radius(c) | DimKind::Diameter(c) => {
                let (cc, r) = doc.circ_val(c);
                let dir = [lw[0] - cc[0], lw[1] - cc[1]];
                let l = dir[0].hypot(dir[1]).max(1e-12);
                let u = [dir[0] / l, dir[1] / l];
                let edge = v.to_s([cc[0] + u[0] * r, cc[1] + u[1] * r]);
                let start = if matches!(d.kind, DimKind::Diameter(_)) { v.to_s([cc[0] - u[0] * r, cc[1] - u[1] * r]) } else { v.to_s(cc) };
                p.line_segment([start, edge], st);
                arrow(edge, edge - start);
                if matches!(d.kind, DimKind::Diameter(_)) {
                    arrow(start, start - edge);
                }
                if l > r {
                    p.line_segment([edge, lp], thin);
                }
            }
            DimKind::Angle(l1, l2) => {
                let x = angle_vertex(doc, l1, l2);
                let dir = |l: usize| {
                    let (a, b) = doc.line_pts(l);
                    let (a, b) = (doc.pval(a), doc.pval(b));
                    (b[1] - a[1]).atan2(b[0] - a[0])
                };
                let (mut a0, a1) = (dir(l1), dir(l2));
                let mut sweep = (a1 - a0).rem_euclid(TAU);
                if sweep > PI {
                    sweep -= TAU;
                }
                if d.supp {
                    a0 += PI;
                    sweep = (a1 - a0).rem_euclid(TAU);
                    if sweep > PI {
                        sweep -= TAU;
                    }
                }
                let rad = (lw[0] - x[0]).hypot(lw[1] - x[1]).max(4.0 / v.s);
                // choose the vertical angle containing the label
                let la = (lw[1] - x[1]).atan2(lw[0] - x[0]);
                let inside = |s: f64, sw: f64| {
                    let rel = (la - s).rem_euclid(TAU);
                    if sw >= 0.0 { rel <= sw } else { (TAU - rel) <= -sw }
                };
                let (s0, sw) = if inside(a0, sweep) { (a0, sweep) } else { (a0 + PI, sweep) };
                let n = 48;
                let pts: Vec<Pos2> = (0..=n).map(|k| {
                    let t = s0 + sw * k as f64 / n as f64;
                    v.to_s([x[0] + rad * t.cos(), x[1] + rad * t.sin()])
                }).collect();
                if pts.len() > 2 {
                    let first = pts[0];
                    let last = pts[pts.len() - 1];
                    let d0 = pts[0] - pts[2];
                    let d1 = pts[pts.len() - 1] - pts[pts.len() - 3];
                    p.add(Shape::line(pts, st));
                    arrow(first, d0);
                    arrow(last, d1);
                }
                // extension along the rays if the arc lies beyond the segments
                for (l, ang) in [(l1, s0), (l2, s0 + sw)] {
                    let (a, b) = doc.line_pts(l);
                    let (pa, pb) = (doc.pval(a), doc.pval(b));
                    let reach = ((pa[0] - x[0]).hypot(pa[1] - x[1])).max((pb[0] - x[0]).hypot(pb[1] - x[1]));
                    if rad > reach {
                        let e0 = [x[0] + reach * ang.cos(), x[1] + reach * ang.sin()];
                        let e1 = [x[0] + (rad + 6.0 / v.s) * ang.cos(), x[1] + (rad + 6.0 / v.s) * ang.sin()];
                        p.line_segment([v.to_s(e0), v.to_s(e1)], thin);
                    }
                }
            }
            DimKind::ConicA(c) | DimKind::ConicB(c) => {
                if let Some(Ent::Conic { c: cp, a, b, t, .. }) = doc.kind(c) {
                    let cc = doc.pval(*cp);
                    let tv = doc.params[*t].v;
                    let tip = if matches!(d.kind, DimKind::ConicA(_)) {
                        let av = doc.params[*a].v;
                        [cc[0] + av * tv.cos(), cc[1] + av * tv.sin()]
                    } else {
                        let bv = doc.params[*b].v;
                        [cc[0] - bv * tv.sin(), cc[1] + bv * tv.cos()]
                    };
                    let (sc, stp) = (v.to_s(cc), v.to_s(tip));
                    p.line_segment([sc, stp], st);
                    arrow(stp, stp - sc);
                    p.line_segment([pos2((sc.x + stp.x) * 0.5, (sc.y + stp.y) * 0.5), lp], thin);
                }
            }
        }
        if txt.is_empty() && name.is_none() {
            return None;
        }
        // label pill
        let font = FontId::proportional(13.0);
        let g = p.layout_no_wrap(txt.to_string(), font.clone(), col);
        // name with its subscript (x_A, d12) set smaller and lowered
        let ng = name.map(|n| {
            let (base, sub) = crate::expr::bx::split_sub(n);
            let gb = p.layout_no_wrap(base.to_string(), FontId::proportional(11.0), MUTED);
            let gs = sub.map(|s| p.layout_no_wrap(s.to_string(), FontId::proportional(8.5), MUTED));
            (gb, gs)
        });
        let nw = ng.as_ref().map(|(gb, gs)| gb.size().x + gs.as_ref().map(|g| g.size().x + 1.0).unwrap_or(0.0) + 6.0).unwrap_or(0.0);
        let size = vec2(g.size().x + nw + 14.0, g.size().y + 6.0);
        let r = Rect::from_center_size(lp, size);
        if self.edit.as_ref().is_some_and(|e| self.geo.doc.dims.get(e.dim).and_then(|x| x.as_ref()).is_some_and(|x| x.name == d.name) && name.is_some()) {
            return Some(r);
        }
        p.rect_filled(r, CornerRadius::same(7), SURFACE);
        p.rect_stroke(r, CornerRadius::same(7), Stroke::new(1.0, col.gamma_multiply(0.55)), StrokeKind::Inside);
        let mut x = r.min.x + 7.0;
        if let Some((gb, gs)) = ng {
            let h = gb.size().y;
            let w = gb.size().x;
            p.galley(pos2(x, r.center().y - h * 0.5), gb, MUTED);
            if let Some(gs) = gs {
                p.galley(pos2(x + w + 1.0, r.center().y - gs.size().y * 0.5 + 3.0), gs, MUTED);
            }
            x += nw;
        }
        p.galley(pos2(x, r.center().y - g.size().y * 0.5), g, col);
        Some(r)
    }

    // ------------------------------------------------------------ constraint glyphs

    fn glyph_anchor(&self, c: &Con) -> Vec<(Pos2, Vec2)> {
        let v = self.view;
        let d = &self.geo.doc;
        let mid_line = |l: usize| {
            let (a, b) = d.line_pts(l);
            let (a, b) = (d.pval(a), d.pval(b));
            let m = v.to_s([(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5]);
            let dir = v.to_s(b) - v.to_s(a);
            let n = vec2(-dir.y, dir.x).normalized();
            let n = if n.y > 0.0 { -n } else { n };
            (m, if n.x.is_finite() { n * 14.0 } else { vec2(0.0, -14.0) })
        };
        let top_of = |e: usize| -> (Pos2, Vec2) {
            if d.is_line(e) {
                return mid_line(e);
            }
            if d.is_circlelike(e) {
                let (c, r) = d.circ_val(e);
                return (v.to_s([c[0] + r * 0.7071, c[1] + r * 0.7071]), vec2(10.0, -10.0));
            }
            if let Some(polys) = self.shapes.get(&e) {
                if let Some(pl) = polys.iter().max_by_key(|p| p.len()) {
                    if !pl.is_empty() {
                        return (pl[pl.len() / 3], vec2(10.0, -12.0));
                    }
                }
            }
            (v.to_s(d.center_of_val(e)), vec2(10.0, -10.0))
        };
        let pt = |e: usize| v.to_s(d.pval(e));
        match *c {
            Con::Coincident(a, _) => vec![(pt(a), vec2(11.0, 11.0))],
            Con::OnCurve(a, _) => vec![(pt(a), vec2(11.0, -11.0))],
            Con::Midpoint(a, _) => vec![(pt(a), vec2(-11.0, 11.0))],
            Con::Horizontal(a, b) => {
                let m = pos2((pt(a).x + pt(b).x) * 0.5, (pt(a).y + pt(b).y) * 0.5);
                vec![(m, vec2(0.0, -14.0))]
            }
            Con::Vertical(a, b) => {
                let m = pos2((pt(a).x + pt(b).x) * 0.5, (pt(a).y + pt(b).y) * 0.5);
                vec![(m, vec2(14.0, 0.0))]
            }
            Con::Parallel(a, b) | Con::Perpendicular(a, b) | Con::Collinear(a, b) | Con::Equal(a, b) => vec![top_of(a), top_of(b)],
            Con::Concentric(a, _) => vec![(v.to_s(d.center_of_val(a)), vec2(12.0, 12.0))],
            Con::Symmetric(a, b, _) => {
                let m = pos2((pt(a).x + pt(b).x) * 0.5, (pt(a).y + pt(b).y) * 0.5);
                vec![(m, vec2(0.0, 14.0))]
            }
            Con::MidpointOf(m, _, _) => vec![(pt(m), vec2(-11.0, 11.0))],
            Con::TangentAt { t, .. } => vec![(pt(t), vec2(12.0, -12.0))],
            Con::Offset { q, .. } | Con::Bisect { q, .. } | Con::Asymptote { q, .. } => vec![(pt(q), vec2(0.0, 0.0))],
            Con::Focus { p, .. } => vec![(pt(p), vec2(11.0, -11.0))],
            Con::Directrix { a, .. } => vec![(pt(a), vec2(0.0, 0.0))],
            Con::DirectrixOf { l, .. } => vec![top_of(l)],
            Con::Tangent { a, b, aux, .. } => {
                if let Some(p) = d.tangent_contact(a, b) {
                    return vec![(pt(p), vec2(12.0, -12.0))];
                }
                if let Some((x, y)) = aux {
                    return vec![(v.to_s([d.params[x].v, d.params[y].v]), vec2(12.0, -12.0))];
                }
                if d.is_line(a) && d.is_circlelike(b) {
                    let (cc, _) = d.circ_val(b);
                    return vec![(v.to_s(foot(d, cc, a)), vec2(12.0, -12.0))];
                }
                let (c1, r1) = d.circ_val(a);
                let (c2, _) = d.circ_val(b);
                let dd = [c2[0] - c1[0], c2[1] - c1[1]];
                let l = dd[0].hypot(dd[1]).max(1e-12);
                vec![(v.to_s([c1[0] + dd[0] / l * r1, c1[1] + dd[1] / l * r1]), vec2(12.0, -12.0))]
            }
        }
    }

    fn paint_glyphs(&mut self, p: &Painter) {
        self.glyphs.clear();
        let mut used: HashMap<(i32, i32), u32> = HashMap::new();
        for i in 0..self.geo.doc.cons.len() {
            let Some(c) = self.geo.doc.cons[i].clone() else { continue };
            let ents = Doc::con_ents(&c);
            if ents.iter().any(|e| self.geo.doc.ent(*e).is_none_or(|x| x.hidden)) {
                continue;
            }
            let icon = match c {
                Con::Coincident(..) => Icon::Coincident,
                Con::OnCurve(..) => Icon::OnCurve,
                Con::Midpoint(..) => Icon::Midpoint,
                Con::Horizontal(..) => Icon::Horizontal,
                Con::Vertical(..) => Icon::Vertical,
                Con::Parallel(..) => Icon::Parallel,
                Con::Perpendicular(..) => Icon::Perpendicular,
                Con::Collinear(..) => Icon::Collinear,
                Con::Equal(..) => Icon::Equal,
                Con::Tangent { .. } => Icon::Tangent,
                Con::Concentric(..) => Icon::Concentric,
                Con::Symmetric(..) => Icon::Symmetric,
                Con::MidpointOf(..) => Icon::Midpoint,
                Con::TangentAt { .. } => Icon::Tangent,
                Con::Offset { rot: true, .. } => Icon::Perpendicular,
                Con::Offset { .. } => Icon::Parallel,
                Con::Bisect { .. } => Icon::AngleBisector,
                Con::Focus { .. } => Icon::Foci,
                Con::Directrix { .. } | Con::DirectrixOf { .. } => Icon::Directrix,
                Con::Asymptote { .. } => Icon::Asymptotes,
            };
            let sel = self.sel.contains(&Sel::Con(i));
            let hov = self.hover == Some(Sel::Con(i)) || self.hover.is_some_and(|h| matches!(h, Sel::Ent(e) if ents.contains(&e)));
            let err = self.geo.conflicts.contains(&Owner::Con(i));
            for (anchor, off) in self.glyph_anchor(&c) {
                if !finite(anchor) {
                    continue;
                }
                let mut q = anchor + off;
                let key = ((q.x / 16.0) as i32, (q.y / 16.0) as i32);
                let n = used.entry(key).or_insert(0);
                q.x += *n as f32 * 18.0;
                *n += 1;
                let r = Rect::from_center_size(q, vec2(17.0, 17.0));
                if !self.view.rect.intersects(r) {
                    continue;
                }
                let bg = if sel { Color32::from_rgba_unmultiplied(0xff, 0xb3, 0x47, 60) } else if hov { SURFACE_HI } else { Color32::from_rgba_unmultiplied(0x20, 0x24, 0x2c, 220) };
                p.rect_filled(r, 5.0, bg);
                let col = if err { ERR } else if sel { SEL } else if hov { FULL } else { MUTED };
                icons::paint(p, r.shrink(3.0), icon, col);
                self.glyphs.push((r, i));
            }
        }
    }

    // ------------------------------------------------------------ overlays

    fn actions(&self) -> Vec<(Icon, &'static str, Act)> {
        let d = &self.geo.doc;
        let mut out: Vec<(Icon, &'static str, Act)> = Vec::new();
        if let [Sel::Dim(i)] = self.sel.as_slice() {
            if let Some(dim) = &d.dims[*i] {
                out.push(if dim.driving { (Icon::Reference, "Make reference", Act::Driving(*i)) } else { (Icon::Driving, "Make driving", Act::Driving(*i)) });
                if dim.kind.is_angle() {
                    out.push((Icon::Supplement, "Supplementary angle", Act::Supp(*i)));
                }
                out.push((Icon::Trash, "Delete  Del", Act::Delete));
            }
            return out;
        }
        let ents: Vec<usize> = self.sel_ents();
        if ents.is_empty() {
            if !self.sel.is_empty() {
                out.push((Icon::Trash, "Delete  Del", Act::Delete));
            }
            return out;
        }
        let pts: Vec<usize> = ents.iter().copied().filter(|e| d.is_point(*e)).collect();
        let lines: Vec<usize> = ents.iter().copied().filter(|e| d.is_line(*e)).collect();
        let circs: Vec<usize> = ents.iter().copied().filter(|e| d.is_circlelike(*e)).collect();
        let n = ents.len();
        let is_curve = |e: usize| !d.is_point(e);
        match ents.as_slice() {
            [a, b] if d.is_point(*a) && d.is_point(*b) => {
                out.push((Icon::Coincident, "Coincident", Act::Con(Con::Coincident(*a, *b))));
                out.push((Icon::Horizontal, "Horizontal  H", Act::Con(Con::Horizontal(*a, *b))));
                out.push((Icon::Vertical, "Vertical  V", Act::Con(Con::Vertical(*a, *b))));
                out.push((Icon::Distance, "Distance", Act::Dim(DimKind::Distance(*a, *b))));
                out.push((Icon::HDist, "Horizontal distance", Act::Dim(DimKind::HDist(*a, *b))));
                out.push((Icon::VDist, "Vertical distance", Act::Dim(DimKind::VDist(*a, *b))));
            }
            [a, b] if (d.is_point(*a) && is_curve(*b)) || (d.is_point(*b) && is_curve(*a)) => {
                let (pt, cv) = if d.is_point(*a) { (*a, *b) } else { (*b, *a) };
                out.push((Icon::OnCurve, "On curve", Act::Con(Con::OnCurve(pt, cv))));
                if d.is_line(cv) {
                    out.push((Icon::Midpoint, "Midpoint", Act::Con(Con::Midpoint(pt, cv))));
                    out.push((Icon::Distance, "Distance", Act::Dim(DimKind::PointLine(pt, cv))));
                }
                if d.is_circlelike(cv) || d.is_conic(cv) {
                    out.push((Icon::Concentric, "At center", Act::Con(Con::Concentric(pt, cv))));
                }
            }
            [l] if d.is_line(*l) => {
                let (a, b) = d.line_pts(*l);
                out.push((Icon::Horizontal, "Horizontal  H", Act::Con(Con::Horizontal(a, b))));
                out.push((Icon::Vertical, "Vertical  V", Act::Con(Con::Vertical(a, b))));
                out.push((Icon::Distance, "Length", Act::Dim(DimKind::Length(*l))));
            }
            [c] if d.is_circlelike(*c) => {
                out.push((Icon::Radius, "Radius", Act::Dim(DimKind::Radius(*c))));
                out.push((Icon::Diameter, "Diameter", Act::Dim(DimKind::Diameter(*c))));
            }
            [c] if d.is_conic(*c) => {
                out.push((Icon::AxisA, "Semi-axis a", Act::Dim(DimKind::ConicA(*c))));
                if !matches!(d.kind(*c), Some(Ent::Conic { kind: ConicKind::Parabola, .. })) {
                    out.push((Icon::AxisB, "Semi-axis b", Act::Dim(DimKind::ConicB(*c))));
                }
            }
            [p] if d.is_point(*p) => {
                out.push((Icon::CoordX, "x coordinate", Act::Dim(DimKind::CoordX(*p))));
                out.push((Icon::CoordY, "y coordinate", Act::Dim(DimKind::CoordY(*p))));
                if !d.is_fixed_ent(*p) {
                    let locked = d.point_locked(*p);
                    out.push(if locked { (Icon::Unlock, "Unlock", Act::Lock(false)) } else { (Icon::Lock, "Lock", Act::Lock(true)) });
                }
            }
            [a, b] if d.is_line(*a) && d.is_line(*b) => {
                out.push((Icon::Parallel, "Parallel", Act::Con(Con::Parallel(*a, *b))));
                out.push((Icon::Perpendicular, "Perpendicular", Act::Con(Con::Perpendicular(*a, *b))));
                out.push((Icon::Collinear, "Collinear", Act::Con(Con::Collinear(*a, *b))));
                out.push((Icon::Equal, "Equal length", Act::Con(Con::Equal(*a, *b))));
                if self.lines_parallel(*a, *b) {
                    out.push((Icon::Distance, "Distance", Act::Dim(DimKind::PointLine(d.line_pts(*b).0, *a))));
                } else {
                    out.push((Icon::Angle, "Angle", Act::Dim(DimKind::Angle(*a, *b))));
                }
            }
            [a, b] if is_curve(*a) && is_curve(*b) => {
                if let Some(t) = self.geo.doc.tangent_con(*a, *b) {
                    out.push((Icon::Tangent, "Tangent  T", Act::Con(t)));
                }
                let lc = |e: usize| d.is_line(e) || d.is_circlelike(e);
                if lc(*a) && lc(*b) {
                    out.push((Icon::Equal, "Equal", Act::Con(Con::Equal(*a, *b))));
                }
                if (d.is_circlelike(*a) || d.is_conic(*a)) && (d.is_circlelike(*b) || d.is_conic(*b)) {
                    out.push((Icon::Concentric, "Concentric", Act::Con(Con::Concentric(*a, *b))));
                }
            }
            _ => {}
        }
        if n == 3 && pts.len() == 2 && lines.len() == 1 {
            out.push((Icon::Symmetric, "Symmetric", Act::Con(Con::Symmetric(pts[0], pts[1], lines[0]))));
        }
        if n > 2 && lines.len() == n {
            out.push((Icon::Parallel, "Parallel", Act::ConChain(0)));
            out.push((Icon::Equal, "Equal length", Act::ConChain(1)));
        }
        if n > 2 && circs.len() == n {
            out.push((Icon::Equal, "Equal radius", Act::ConChain(1)));
        }
        if n > 2 && pts.len() == n {
            out.push((Icon::Horizontal, "Horizontal", Act::ConChain(2)));
            out.push((Icon::Vertical, "Vertical", Act::ConChain(3)));
        }
        if ents.iter().any(|e| !d.is_point(*e) && !d.is_curve_ent(*e) && !d.is_fixed_ent(*e)) {
            if lines.len() == 1 && n == 1 {
                out.push((Icon::Line, "Infinite", Act::Infinite));
            }
            out.push((Icon::Construction, "Construction  X", Act::Construction));
        }
        if ents.iter().any(|e| !d.is_fixed_ent(*e)) || self.sel.iter().any(|s| !matches!(s, Sel::Ent(_))) {
            out.push((Icon::Trash, "Delete  Del", Act::Delete));
        }
        out
    }

    fn run_act(&mut self, a: Act) {
        match a {
            Act::Con(c) => {
                self.add_constraint(c);
            }
            Act::ConChain(k) => {
                let ents = self.sel_ents();
                let cons: Vec<Con> = ents
                    .windows(2)
                    .map(|w| match k {
                        0 => Con::Parallel(w[0], w[1]),
                        1 => Con::Equal(w[0], w[1]),
                        2 => Con::Horizontal(w[0], w[1]),
                        _ => Con::Vertical(w[0], w[1]),
                    })
                    .collect();
                if !self.geo.try_change(true, |d| {
                    for c in cons {
                        d.cons.push(Some(c));
                    }
                }) {
                    self.flash_ents(ents);
                }
            }
            Act::Dim(k) => {
                let anchor = self.geo.doc.dim_anchor(&k);
                let off = 40.0 / self.view.s;
                let label = match k {
                    DimKind::HDist(..) => [anchor[0], anchor[1] + off * 1.2],
                    DimKind::CoordX(p) => {
                        let q = self.geo.doc.pval(p);
                        [anchor[0], if q[1] >= 0.0 { -off * 0.6 } else { off * 0.6 }]
                    }
                    DimKind::CoordY(p) => {
                        let q = self.geo.doc.pval(p);
                        [if q[0] >= 0.0 { -off * 0.9 } else { off * 0.9 }, anchor[1]]
                    }
                    DimKind::VDist(..) => [anchor[0] + off * 1.2, anchor[1]],
                    DimKind::Radius(c) | DimKind::Diameter(c) => {
                        let (cc, r) = self.geo.doc.circ_val(c);
                        [cc[0] + (r + off) * 0.7071, cc[1] + (r + off) * 0.7071]
                    }
                    DimKind::Angle(..) => [anchor[0] + off * 1.5, anchor[1] + off * 0.6],
                    _ => [anchor[0] + off * 0.5, anchor[1] + off],
                };
                if let Some(i) = self.geo.add_dimension(k, label) {
                    self.sel = vec![Sel::Dim(i)];
                    if self.geo.doc.dims[i].as_ref().is_some_and(|d| d.driving) {
                        self.open_dim_editor(i);
                    }
                }
            }
            Act::Lock(on) => {
                let ents = self.sel_ents();
                self.geo.try_change(false, |d| {
                    for e in ents {
                        d.lock_point(e, on);
                    }
                });
            }
            Act::Construction => {
                let ents = self.sel_ents();
                let on = !ents.iter().all(|e| self.geo.doc.ent(*e).is_some_and(|x| x.construction));
                self.geo.try_change(false, |d| {
                    for e in ents {
                        if let Some(x) = d.ents[e].as_mut() {
                            if !matches!(x.kind, Ent::Point { .. }) {
                                x.construction = on;
                            }
                        }
                    }
                });
            }
            Act::Infinite => {
                let ents = self.sel_ents();
                self.geo.try_change(false, |d| {
                    for e in ents {
                        if let Some(Entity { kind: Ent::Line { infinite, .. }, .. }) = d.ents[e].as_mut() {
                            *infinite = !*infinite;
                        }
                    }
                });
            }
            Act::Delete => self.delete_selection(),
            Act::Driving(i) => self.toggle_driving(i),
            Act::Supp(i) => {
                let p = self.geo.doc.dims[i].as_ref().map(|d| d.param).unwrap();
                let cur = self.geo.doc.params[p].v;
                let driving = self.geo.doc.dims[i].as_ref().unwrap().driving;
                self.geo.try_change(false, |d| {
                    let x = d.dims[i].as_mut().unwrap();
                    x.supp = !x.supp;
                    if driving && x.expr.is_empty() {
                        d.params[p].v = PI - cur;
                    }
                });
            }
        }
    }

    fn pill(&self, ui: &mut Ui, r: Rect) {
        ui.painter().rect_filled(r.translate(vec2(0.0, 2.0)), 12.0, Color32::from_black_alpha(70));
        ui.painter().rect_filled(r, 12.0, PANEL);
        ui.painter().rect_stroke(r, 12.0, Stroke::new(1.0, LINE), StrokeKind::Inside);
    }

    fn overlays(&mut self, ui: &mut Ui, rect: Rect) {
        // tool palette
        let entries: [Result<(Tool, Icon, &str), usize>; 9] = [
            Ok((Tool::Select, Icon::Select, "Select  Esc")),
            Ok((Tool::Point, Icon::Point, "Point  P")),
            Ok((Tool::Line, Icon::Line, "Line  L")),
            Err(2),
            Err(3),
            Err(0),
            Ok((Tool::Curve, Icon::Curve, "Function  F")),
            Err(1),
            Ok((Tool::Dimension, Icon::Dimension, "Dimension  D")),
        ];
        let h = entries.len() as f32 * 34.0 + 12.0;
        let r = Rect::from_min_size(pos2(rect.min.x + 12.0, rect.center().y - h * 0.5), vec2(46.0, h));
        self.pill(ui, r);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(vec2(6.0, 6.0))).layout(egui::Layout::top_down(egui::Align::Center)));
        child.spacing_mut().item_spacing = vec2(0.0, 0.0);
        let mut open_at: Option<(usize, Rect)> = None;
        for e in entries {
            match e {
                Ok((t, icon, tip)) => {
                    if icon_btn(&mut child, icon, self.tool == t, tip).clicked() {
                        self.set_tool(t);
                    }
                }
                Err(g) => {
                    let items = TOOL_GROUPS[g];
                    let active = items.iter().find(|x| x.0 == self.tool).copied();
                    let shown = active.unwrap_or_else(|| *items.iter().find(|x| x.0 == self.group_last[g]).unwrap_or(&items[0]));
                    let resp = icon_btn(&mut child, shown.1, active.is_some(), shown.2);
                    let rr = resp.rect;
                    let corner = rr.right_bottom() + vec2(-5.0, -5.0);
                    child.painter().add(Shape::convex_polygon(vec![corner, corner + vec2(-5.0, 0.0), corner + vec2(0.0, -5.0)], MUTED, Stroke::NONE));
                    if resp.clicked() {
                        self.flyout = if self.flyout == Some(g) { None } else { Some(g) };
                    }
                    if self.flyout == Some(g) {
                        open_at = Some((g, rr));
                    }
                }
            }
        }
        if let Some((g, rr)) = open_at {
            let items = TOOL_GROUPS[g];
            let w = items.len() as f32 * 34.0 + 12.0;
            let fr = Rect::from_min_size(pos2(r.max.x + 8.0, rr.center().y - 23.0), vec2(w, 46.0));
            self.pill(ui, fr);
            let mut fc = ui.new_child(egui::UiBuilder::new().max_rect(fr.shrink2(vec2(6.0, 6.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
            fc.spacing_mut().item_spacing = vec2(0.0, 0.0);
            for (t, icon, tip) in items {
                if icon_btn(&mut fc, *icon, self.tool == *t, tip).clicked() {
                    self.set_tool(*t);
                }
            }
        }

        // top-right controls
        let ctrls: [(Icon, &str, u8); 6] = [
            (Icon::Undo, "Undo  Ctrl+Z", 0),
            (Icon::Redo, "Redo  Ctrl+Shift+Z", 1),
            (Icon::Fit, "Fit  Home", 2),
            (Icon::Eye, "Constraints", 3),
            (Icon::Open, "Open  Ctrl+O", 4),
            (Icon::Save, "Save  Ctrl+S", 5),
        ];
        let w = ctrls.len() as f32 * 34.0 + 12.0;
        let r = Rect::from_min_size(pos2(rect.max.x - w - 12.0, rect.min.y + 12.0), vec2(w, 46.0));
        self.pill(ui, r);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(vec2(6.0, 6.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
        child.spacing_mut().item_spacing = vec2(0.0, 0.0);
        for (icon, tip, k) in ctrls {
            let active = k == 3 && self.show_cons;
            if icon_btn(&mut child, icon, active, tip).clicked() {
                match k {
                    0 => self.undo(),
                    1 => self.redo(),
                    2 => self.fit(),
                    3 => self.show_cons = !self.show_cons,
                    4 => self.open(),
                    _ => self.save(false),
                }
            }
        }

        // context actions
        let acts = self.actions();
        if !acts.is_empty() {
            let w = acts.len() as f32 * 34.0 + 12.0;
            let r = Rect::from_min_size(pos2(rect.center().x - w * 0.5, rect.min.y + 12.0), vec2(w, 46.0));
            self.pill(ui, r);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(vec2(6.0, 6.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
            child.spacing_mut().item_spacing = vec2(0.0, 0.0);
            let mut run = None;
            for (icon, tip, a) in acts {
                if icon_btn(&mut child, icon, false, tip).clicked() {
                    run = Some(a.clone());
                }
            }
            if let Some(a) = run {
                self.run_act(a);
            }
        }

        // constraint status
        let (col, txt) = if !self.geo.conflicts.is_empty() {
            (ERR, format!("{}", self.geo.dof))
        } else if self.geo.dof == 0 {
            (GOOD, "0".to_string())
        } else {
            (ACCENT, format!("{}", self.geo.dof))
        };
        let r = Rect::from_min_size(pos2(rect.min.x + 12.0, rect.max.y - 40.0), vec2(56.0, 28.0));
        self.pill(ui, r);
        ui.painter().circle_filled(pos2(r.min.x + 15.0, r.center().y), 4.5, col);
        ui.painter().text(pos2(r.min.x + 26.0, r.center().y), Align2::LEFT_CENTER, txt, FontId::proportional(13.0), FULL);
        let resp = ui.interact(r, ui.make_persistent_id("dof"), Sense::hover());
        resp.on_hover_text("Degrees of freedom");

        // function input
        if self.tool == Tool::Curve {
            let mut owned = self.curve_input.take().unwrap_or_else(|| "y = ".to_string());
            let input = &mut owned;
            let w = 420.0f32.min(rect.width() - 160.0);
            let r = Rect::from_min_size(pos2(rect.center().x - w * 0.5, rect.min.y + 66.0), vec2(w, 40.0));
            self.pill(ui, r);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(vec2(10.0, 6.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
            let te = egui::TextEdit::singleline(input)
                .font(FontId::proportional(16.0))
                .frame(egui::Frame::NONE)
                .desired_width(w - 24.0)
                .id_salt("curve_input");
            let resp = child.add(te);
            if !resp.has_focus() && !resp.lost_focus() {
                resp.request_focus();
            }
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter)) && self.live;
            let esc = ui.input(|i| i.key_pressed(egui::Key::Escape)) && self.live;
            self.curve_input = Some(owned.clone());
            if resp.lost_focus() && enter {
                let src = owned.trim().to_string();
                if self.geo.doc.curve_def(&src).is_ok() {
                    let mut idx = 0;
                    self.geo.try_change(false, |d| idx = d.add_ent(Ent::Curve { src }));
                    self.curve_input = None;
                    self.tool = Tool::Select;
                    self.sel = vec![Sel::Ent(idx)];
                } else {
                    resp.request_focus();
                }
            } else if esc {
                self.curve_input = None;
                self.tool = Tool::Select;
            }
            if let Some(src) = &self.curve_input {
                if let Err(_) = self.geo.doc.curve_def(src) {
                    ui.painter().circle_filled(pos2(r.max.x - 14.0, r.center().y), 3.0, ERR);
                }
            }
        }

        // inline dimension editor
        if let Some(mut ed_owned) = self.edit.take() {
            let ed = &mut ed_owned;
            let Some(dim) = self.geo.doc.dims.get(ed.dim).and_then(|d| d.clone()) else {
                return;
            };
            let a = self.geo.doc.dim_anchor(&dim.kind);
            let lp = self.view.to_s([a[0] + dim.off[0], a[1] + dim.off[1]]);
            let w = 130.0;
            let r = Rect::from_center_size(lp, vec2(w, 30.0));
            ui.painter().rect_filled(r.expand(2.0), 9.0, Color32::from_black_alpha(80));
            ui.painter().rect_filled(r, 8.0, SURFACE_HI);
            ui.painter().rect_stroke(r, 8.0, Stroke::new(1.2, DIMC), StrokeKind::Inside);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(r.shrink2(vec2(8.0, 4.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
            let te = egui::TextEdit::singleline(&mut ed.text)
                .font(FontId::proportional(14.0))
                .frame(egui::Frame::NONE)
                .desired_width(w - 16.0)
                .id_salt("dim_edit");
            let resp = child.add(te);
            if ed.focus {
                resp.request_focus();
                if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), resp.id) {
                    let ccur = egui::text::CCursor::new(ed.text.chars().count());
                    st.cursor.set_char_range(Some(egui::text::CCursorRange::two(egui::text::CCursor::new(0), ccur)));
                    st.store(ui.ctx(), resp.id);
                }
                ed.focus = false;
            }
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            let esc = ui.input(|i| i.key_pressed(egui::Key::Escape));
            if !self.live {
                self.edit = Some(ed_owned);
            } else if esc {
                self.edit = None;
            } else if resp.lost_focus() {
                let (d, t) = (ed.dim, ed.text.clone());
                self.edit = None;
                if enter {
                    self.commit_dim_text(d, &t);
                }
            } else {
                self.edit = Some(ed_owned);
            }
        }
    }

    pub fn set_tool(&mut self, t: Tool) {
        self.tool = t;
        self.flyout = None;
        self.picks.clear();
        for (g, items) in TOOL_GROUPS.iter().enumerate() {
            if items.iter().any(|x| x.0 == t) {
                self.group_last[g] = t;
            }
        }
        self.pending.clear();
        self.hv_hint = None;
        if t == Tool::Curve {
            self.curve_input = Some("y = ".into());
        } else {
            self.curve_input = None;
        }
        if t == Tool::Dimension {
            self.sel.retain(|s| matches!(s, Sel::Ent(_)));
            if self.sel.len() > 2 {
                self.sel.clear();
            }
        }
    }
}

/// Screen polylines of a point, line, circle, arc or conic of `doc` (curves
/// given by functions need the tessellation cache and are not covered).
pub fn geom_polys(doc: &Doc, v: View, vr: f64, i: usize) -> Vec<Vec<Pos2>> {
    let Some(e) = doc.ent(i) else { return vec![] };
    match &e.kind {
        Ent::Point { .. } => vec![vec![v.to_s(doc.pval(i))]],
        Ent::Line { a, b, infinite } => {
            let (pa, pb) = (doc.pval(*a), doc.pval(*b));
            if *infinite {
                let d = [pb[0] - pa[0], pb[1] - pa[1]];
                let l = d[0].hypot(d[1]).max(1e-12);
                let u = [d[0] / l, d[1] / l];
                let t = (v.c[0] - pa[0]) * u[0] + (v.c[1] - pa[1]) * u[1];
                let f = [pa[0] + u[0] * t, pa[1] + u[1] * t];
                let ext = vr * 1.5;
                vec![vec![v.to_s([f[0] - u[0] * ext, f[1] - u[1] * ext]), v.to_s([f[0] + u[0] * ext, f[1] + u[1] * ext])]]
            } else {
                vec![vec![v.to_s(pa), v.to_s(pb)]]
            }
        }
        Ent::Circle { .. } => {
            let (c, r) = doc.circ_val(i);
            let n = ((r * v.s * 0.6) as usize).clamp(48, 720);
            vec![(0..=n).map(|k| {
                let t = TAU * k as f64 / n as f64;
                v.to_s([c[0] + r * t.cos(), c[1] + r * t.sin()])
            }).collect()]
        }
        Ent::Arc { c, a, b } => {
            let (cc, pa, pb) = (doc.pval(*c), doc.pval(*a), doc.pval(*b));
            let r = (pa[0] - cc[0]).hypot(pa[1] - cc[1]);
            let a0 = (pa[1] - cc[1]).atan2(pa[0] - cc[0]);
            let mut a1 = (pb[1] - cc[1]).atan2(pb[0] - cc[0]);
            while a1 <= a0 {
                a1 += TAU;
            }
            let n = (((a1 - a0) * r * v.s * 0.6) as usize).clamp(16, 720);
            vec![(0..=n).map(|k| {
                let t = a0 + (a1 - a0) * k as f64 / n as f64;
                v.to_s([cc[0] + r * t.cos(), cc[1] + r * t.sin()])
            }).collect()]
        }
        Ent::Conic { kind, c, a, b, t } => {
            let cc = doc.pval(*c);
            let (a, b, t) = (doc.params[*a].v, doc.params[*b].v, doc.params[*t].v);
            let (ct, st) = (t.cos(), t.sin());
            let tr = |u: f64, w: f64| v.to_s([cc[0] + u * ct - w * st, cc[1] + u * st + w * ct]);
            let far = vr + (cc[0] - v.c[0]).hypot(cc[1] - v.c[1]);
            match kind {
                ConicKind::Ellipse => {
                    let n = (((a.abs() + b.abs()) * v.s) as usize).clamp(64, 720);
                    vec![(0..=n).map(|k| {
                        let th = TAU * k as f64 / n as f64;
                        tr(a * th.cos(), b * th.sin())
                    }).collect()]
                }
                ConicKind::Hyperbola => {
                    let umax = (far / b.abs().max(1e-9)).asinh().clamp(0.5, 12.0) + 0.2;
                    let n = 240;
                    [1.0, -1.0]
                        .iter()
                        .map(|sg| {
                            (0..=n).map(|k| {
                                let s = -1.0 + 2.0 * k as f64 / n as f64;
                                let u = umax * s * s.abs();
                                tr(sg * a * u.cosh(), b * u.sinh())
                            }).collect()
                        })
                        .collect()
                }
                ConicKind::Parabola => {
                    let vmax = 2.0 * far + 4.0 * a.abs().sqrt() * far.sqrt() + 1.0;
                    let n = 300;
                    vec![(0..=n).map(|k| {
                        let s = -1.0 + 2.0 * k as f64 / n as f64;
                        let w = vmax * s * s * s.signum();
                        tr(w * w / (4.0 * a), w)
                    }).collect()]
                }
            }
        }
        Ent::Curve { .. } => vec![],
    }
}

pub const TOOL_GROUPS: [&[(Tool, Icon, &str)]; 4] = [
    &[
        (Tool::Ellipse, Icon::Ellipse, "Ellipse  E"),
        (Tool::Hyperbola, Icon::Hyperbola, "Hyperbola  Y"),
        (Tool::Parabola, Icon::Parabola, "Parabola  B"),
        (Tool::EllipseFoci, Icon::EllipseFoci, "Ellipse by foci and a point"),
        (Tool::HyperbolaFoci, Icon::HyperbolaFoci, "Hyperbola by foci and a point"),
        (Tool::ParabolaFocus, Icon::ParabolaFocus, "Parabola by focus and directrix line"),
    ],
    &[
        (Tool::Midpoint, Icon::Midpoint, "Midpoint  M"),
        (Tool::Intersect, Icon::Intersect, "Intersection  I"),
        (Tool::PerpLine, Icon::PerpLine, "Perpendicular line  N"),
        (Tool::ParLine, Icon::ParLine, "Parallel line  R"),
        (Tool::PerpBisector, Icon::PerpBisector, "Perpendicular bisector  W"),
        (Tool::AngleBisector, Icon::AngleBisector, "Angle bisector  G"),
        (Tool::TangentLine, Icon::TangentLine, "Tangent line  J"),
        (Tool::Reflect, Icon::Reflect, "Reflect  U"),
        (Tool::Foci, Icon::Foci, "Foci of a conic  K"),
        (Tool::Directrix, Icon::Directrix, "Directrix of a conic, on the clicked side  Q"),
        (Tool::Asymptotes, Icon::Asymptotes, "Asymptotes of a hyperbola  O"),
    ],
    &[(Tool::Circle, Icon::Circle, "Circle  C"), (Tool::Circle3, Icon::Circle3, "Circle through three points")],
    &[(Tool::Arc, Icon::Arc, "Arc  A"), (Tool::Arc3, Icon::Arc3, "Arc through three points")],
];

/// Builds the shape of one of the point-based shape tools (circle or arc
/// through three points, conic by foci) from its clicks; None when the
/// clicks do not determine one.
pub fn build_shape(d: &mut Doc, tool: Tool, snaps: &[Snap]) -> Option<usize> {
    let pts = |d: &mut Doc, n: usize| -> Option<Vec<usize>> {
        let v: Vec<usize> = snaps.iter().take(n).map(|s| realize(d, *s)).collect();
        let distinct = (0..v.len()).all(|i| (i + 1..v.len()).all(|j| v[i] != v[j]));
        (v.len() == n && distinct).then_some(v)
    };
    match tool {
        Tool::Circle3 => {
            let p = pts(d, 3)?;
            d.build_circle_through(p[0], p[1], p[2]).map(|x| x.0)
        }
        Tool::Arc3 => {
            let p = pts(d, 3)?;
            d.build_arc_through(p[0], p[1], p[2]).map(|x| x.0)
        }
        Tool::EllipseFoci | Tool::HyperbolaFoci => {
            let kind = if tool == Tool::EllipseFoci { ConicKind::Ellipse } else { ConicKind::Hyperbola };
            let p = pts(d, 3)?;
            d.build_conic_by_foci(kind, p[0], p[1], p[2])
        }
        Tool::ParabolaFocus => {
            let Some(&Snap::Curve(l, _)) = snaps.get(1) else { return None };
            if !d.is_line(l) {
                return None;
            }
            let f = realize(d, *snaps.first()?);
            d.build_parabola_by_focus(f, l)
        }
        _ => None,
    }
}

fn brighten(c: Color32) -> Color32 {
    let f = |x: u8| (x as f32 + (255.0 - x as f32) * 0.35) as u8;
    Color32::from_rgb(f(c.r()), f(c.g()), f(c.b()))
}

/// Joins short segments sharing endpoints into longer polylines.
fn join_segments(segs: Vec<Vec<Pos2>>) -> Vec<Vec<Pos2>> {
    let key = |p: Pos2| ((p.x * 8.0).round() as i64, (p.y * 8.0).round() as i64);
    let mut ends: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (i, s) in segs.iter().enumerate() {
        ends.entry(key(s[0])).or_default().push(i);
        ends.entry(key(s[1])).or_default().push(i);
    }
    let mut used = vec![false; segs.len()];
    let mut out = Vec::new();
    for i in 0..segs.len() {
        if used[i] {
            continue;
        }
        used[i] = true;
        let mut line: std::collections::VecDeque<Pos2> = segs[i].iter().copied().collect();
        for forward in [true, false] {
            loop {
                let end = if forward { *line.back().unwrap() } else { *line.front().unwrap() };
                let next = ends.get(&key(end)).and_then(|v| v.iter().copied().find(|j| !used[*j]));
                let Some(j) = next else { break };
                used[j] = true;
                let s = &segs[j];
                let other = if key(s[0]) == key(end) { s[1] } else { s[0] };
                if forward {
                    line.push_back(other);
                } else {
                    line.push_front(other);
                }
            }
        }
        out.push(line.into_iter().collect());
    }
    out
}

#[cfg(test)]
mod shape_tool_tests {
    use super::*;

    fn count(app: &App, f: impl Fn(&Ent) -> bool) -> usize {
        app.geo.doc.ents.iter().flatten().filter(|e| f(&e.kind)).count()
    }

    fn run(tool: Tool, clicks: &[Snap]) -> App {
        let mut app = App::new();
        app.set_tool(tool);
        for s in clicks {
            app.tool_click(*s);
        }
        app
    }

    #[test]
    fn circle_through_three_clicks() {
        let app = run(Tool::Circle3, &[Snap::Free([1.0, 0.0]), Snap::Free([0.0, 1.0]), Snap::Free([-1.0, 0.0])]);
        let circ = (0..app.geo.doc.ents.len()).find(|&i| matches!(app.geo.doc.kind(i), Some(Ent::Circle { .. }))).expect("no circle");
        let (c, r) = app.geo.doc.circ_val(circ);
        assert!(c[0].abs() < 1e-9 && c[1].abs() < 1e-9 && (r - 1.0).abs() < 1e-9, "{c:?} {r}");
        assert!(app.pending.is_empty());
    }

    #[test]
    fn collinear_clicks_make_no_circle() {
        let app = run(Tool::Circle3, &[Snap::Free([0.0, 0.0]), Snap::Free([1.0, 1.0]), Snap::Free([2.0, 2.0])]);
        assert_eq!(count(&app, |k| matches!(k, Ent::Circle { .. })), 0);
        // the refused click is not kept: another third click can follow
        assert_eq!(app.pending.len(), 2);
    }

    #[test]
    fn arc_through_three_clicks() {
        let app = run(Tool::Arc3, &[Snap::Free([2.0, 0.0]), Snap::Free([0.0, 2.0]), Snap::Free([-2.0, 0.0])]);
        assert_eq!(count(&app, |k| matches!(k, Ent::Arc { .. })), 1);
    }

    #[test]
    fn conics_by_foci_clicks() {
        for (tool, kind) in [(Tool::EllipseFoci, ConicKind::Ellipse), (Tool::HyperbolaFoci, ConicKind::Hyperbola)] {
            let third = if kind == ConicKind::Ellipse { [0.0, 3.0] } else { [3.5, 1.0] };
            let app = run(tool, &[Snap::Free([-4.0, 0.0]), Snap::Free([4.0, 0.0]), Snap::Free(third)]);
            let c = (0..app.geo.doc.ents.len()).find(|&i| matches!(app.geo.doc.kind(i), Some(Ent::Conic { kind: k, .. }) if *k == kind)).expect("no conic");
            assert_eq!(app.geo.doc.foci_of(c).len(), 2, "{kind:?}");
        }
    }

    #[test]
    fn parabola_by_focus_then_line() {
        let app = App::new();
        let xa = app.geo.doc.xaxis;
        let app = run(Tool::ParabolaFocus, &[Snap::Free([0.0, 2.0]), Snap::Curve(xa, [0.5, 0.0])]);
        let par = (0..app.geo.doc.ents.len()).find(|&i| matches!(app.geo.doc.kind(i), Some(Ent::Conic { kind: ConicKind::Parabola, .. }))).expect("no parabola");
        let Some(Ent::Conic { a, .. }) = app.geo.doc.kind(par) else { unreachable!() };
        assert!((app.geo.doc.params[*a].v - 1.0).abs() < 1e-9);
        // a second click that is not on a line is ignored
        let app = run(Tool::ParabolaFocus, &[Snap::Free([0.0, 2.0]), Snap::Free([1.0, 1.0])]);
        assert_eq!(app.pending.len(), 1);
    }

    #[test]
    fn ghost_of_next_click() {
        let mut app = run(Tool::EllipseFoci, &[Snap::Free([-4.0, 0.0]), Snap::Free([4.0, 0.0])]);
        assert!(!app.shape_ghost(Snap::Free([0.0, 3.0])).is_empty());
        app.set_tool(Tool::Circle3);
        app.pending = vec![Snap::Free([0.0, 0.0]), Snap::Free([1.0, 1.0])];
        assert!(app.shape_ghost(Snap::Free([2.0, 2.0])).is_empty());
    }
}
