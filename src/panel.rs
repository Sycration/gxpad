//! Side panel: variables, functions, dimensions and the derived formula.

use crate::app::*;
use crate::canvas::small_icon_btn;
use crate::expr::bx::Bx;
use crate::icons::{self, Icon};
use crate::mathview;
use crate::ops;
use crate::model::*;
use eframe::egui::{self, Color32, FontId, Key, Response, Sense, Stroke, TextEdit, Ui, vec2};
use std::collections::HashMap;

fn text_cell(ui: &mut Ui, bufs: &mut HashMap<String, String>, key: &str, current: &str, width: f32, col: Option<Color32>) -> (Response, Option<String>) {
    let live = ui.ctx().current_pass_index() == 0;
    let buf = bufs.entry(key.to_string()).or_insert_with(|| current.to_string());
    let mut te = TextEdit::singleline(buf).id_salt(key).desired_width(width).font(FontId::proportional(14.0));
    if let Some(c) = col {
        te = te.text_color(c);
    }
    let resp = ui.add(te);
    let esc = ui.input(|i| i.key_pressed(Key::Escape));
    let mut out = None;
    if live && resp.lost_focus() && !esc && *buf != current {
        out = Some(buf.clone());
    } else if !resp.has_focus() {
        *buf = current.to_string();
    }
    (resp, out)
}

fn section(ui: &mut Ui, icon: Icon) -> egui::Rect {
    let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
    icons::paint(ui.painter(), r, icon, MUTED);
    r
}

impl App {
    pub fn frame_ui(&mut self, ui: &mut Ui, cache: &mut HashMap<usize, (u64, Vec<Vec<egui::Pos2>>)>) {
        self.live = ui.ctx().current_pass_index() == 0;
        if self.live {
            self.keys(ui);
        }
        egui::Panel::right("side")
            .resizable(true)
            .default_size(360.0)
            .size_range(260.0..=900.0)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::same(12)).stroke(Stroke::new(1.0, LINE)))
            .show(ui, |ui| self.panel_ui(ui));
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(BG)).show(ui, |ui| self.canvas_ui(ui, cache));
        let title = match &self.path {
            Some(p) => format!("gxpad — {}", p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default()),
            None => "gxpad".into(),
        };
        if self.title != title {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        if self.now < self.flash_until {
            ui.ctx().request_repaint();
        }
    }

    fn keys(&mut self, ui: &Ui) {
        let ctx = ui.ctx();
        if ctx.text_edit_focused() {
            return;
        }
        let ev = ui.input(|i| {
            let m = i.modifiers;
            let p = |k: Key| i.key_pressed(k);
            (m, p(Key::Escape), p(Key::Delete) || p(Key::Backspace), p(Key::Z), p(Key::Y), p(Key::S), p(Key::O), p(Key::N), p(Key::A), p(Key::Home), p(Key::Enter))
        });
        let (m, esc, del, z, y, s, o, n, a, home, enter) = ev;
        let cmd = m.command;
        if cmd {
            if z && m.shift || y {
                self.redo();
            } else if z {
                self.undo();
            } else if s {
                self.save(m.shift);
            } else if o {
                self.open();
            } else if n {
                let before = std::mem::replace(&mut self.geo.doc, Doc::new());
                self.geo.push_undo(before);
                self.path = None;
                self.sel.clear();
                self.geo.dirty = true;
                self.geo.solve_full();
            } else if a {
                self.sel = self.geo.doc.ents.iter().enumerate().filter(|(i, e)| e.as_ref().is_some_and(|e| !e.hidden) && !self.geo.doc.is_fixed_ent(*i)).map(|(i, _)| Sel::Ent(i)).collect();
            }
            return;
        }
        if esc {
            if self.edit.is_some() {
                self.edit = None;
            } else if self.flyout.is_some() {
                self.flyout = None;
            } else if !self.pending.is_empty() || !self.picks.is_empty() {
                self.pending.clear();
                self.picks.clear();
            } else if self.tool != Tool::Select {
                self.set_tool(Tool::Select);
            } else {
                self.sel.clear();
            }
        }
        if del {
            self.delete_selection();
        }
        if home {
            self.fit();
        }
        if enter {
            if let [Sel::Dim(d)] = self.sel.as_slice() {
                let d = *d;
                self.open_dim_editor(d);
            }
        }
        let keys: Vec<Key> = ui.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Key { key, pressed: true, repeat: false, modifiers, .. } if !modifiers.command && !modifiers.alt => Some(*key),
                    _ => None,
                })
                .collect()
        });
        for k in keys {
            let tool = match k {
                Key::S => Some(Tool::Select),
                Key::P => Some(Tool::Point),
                Key::L => Some(Tool::Line),
                Key::C => Some(Tool::Circle),
                Key::A => Some(Tool::Arc),
                Key::E => Some(Tool::Ellipse),
                Key::Y => Some(Tool::Hyperbola),
                Key::B => Some(Tool::Parabola),
                Key::F => Some(Tool::Curve),
                Key::D => Some(Tool::Dimension),
                Key::M => Some(Tool::Midpoint),
                Key::I => Some(Tool::Intersect),
                Key::N => Some(Tool::PerpLine),
                Key::R => Some(Tool::ParLine),
                Key::W => Some(Tool::PerpBisector),
                Key::G => Some(Tool::AngleBisector),
                Key::J => Some(Tool::TangentLine),
                Key::U => Some(Tool::Reflect),
                Key::K => Some(Tool::Foci),
                Key::Q => Some(Tool::Directrix),
                Key::O => Some(Tool::Asymptotes),
                _ => None,
            };
            if let Some(t) = tool {
                self.set_tool(t);
                continue;
            }
            let ents: Vec<usize> = self.sel.iter().filter_map(|s| if let Sel::Ent(e) = s { Some(*e) } else { None }).collect();
            match k {
                Key::H | Key::V => {
                    let h = k == Key::H;
                    let mk = |a: usize, b: usize| if h { Con::Horizontal(a, b) } else { Con::Vertical(a, b) };
                    let cons: Vec<Con> = if ents.len() == 1 && self.geo.doc.is_line(ents[0]) {
                        let (a, b) = self.geo.doc.line_pts(ents[0]);
                        vec![mk(a, b)]
                    } else if ents.len() >= 2 && ents.iter().all(|e| self.geo.doc.is_point(*e)) {
                        ents.windows(2).map(|w| mk(w[0], w[1])).collect()
                    } else if !ents.is_empty() && ents.iter().all(|e| self.geo.doc.is_line(*e)) {
                        ents.iter().map(|l| {
                            let (a, b) = self.geo.doc.line_pts(*l);
                            mk(a, b)
                        }).collect()
                    } else {
                        vec![]
                    };
                    if !cons.is_empty() && !ops::add_constraints(&mut self.geo, cons).unwrap_or(false) {
                        self.flash_ents(ents);
                    }
                }
                Key::T => {
                    if let [a, b] = ents.as_slice() {
                        if let Some(c) = self.geo.doc.tangent_con(*a, *b) {
                            self.add_constraint(c);
                        }
                    }
                }
                Key::X => {
                    let ents: Vec<usize> = ents.into_iter().filter(|e| !self.geo.doc.is_point(*e)).collect();
                    let on = !ents.iter().all(|e| self.geo.doc.ent(*e).is_some_and(|x| x.construction));
                    ops::set_construction_all(&mut self.geo, &ents, on).ok();
                }
                _ => {}
            }
        }
    }

    fn row_bg(&self, ui: &Ui, r: egui::Rect, selected: bool) {
        if selected {
            ui.painter().rect_filled(r.expand2(vec2(4.0, 2.0)), 6.0, Color32::from_rgba_unmultiplied(0xff, 0xb3, 0x47, 22));
        }
    }

    fn panel_ui(&mut self, ui: &mut Ui) {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        egui::Panel::bottom("formula")
            .resizable(true)
            .default_size(240.0)
            .size_range(80.0..=700.0)
            .frame(egui::Frame::new().inner_margin(egui::Margin { left: 0, right: 0, top: 10, bottom: 0 }))
            .show(ui, |ui| self.formula_ui(ui));
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            self.vars_ui(ui);
            ui.add_space(6.0);
            ui.separator();
            self.curves_ui(ui);
            ui.add_space(6.0);
            ui.separator();
            self.dims_ui(ui);
        });
    }

    fn vars_ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            section(ui, Icon::Variable);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if small_icon_btn(ui, Icon::Latex, false, "Add defined variable").clicked() {
                    ops::add_defvar(&mut self.geo, None, "0").ok();
                }
                if small_icon_btn(ui, Icon::Plus, false, "Add variable").clicked() {
                    ops::add_var(&mut self.geo, None, 1.0, true).ok();
                }
            });
        });
        let w = ui.available_width();
        for i in 0..self.geo.doc.vars.len() {
            let Some(v) = self.geo.doc.vars[i].clone() else { continue };
            let selected = self.sel.contains(&Sel::Var(i));
            let r = ui.horizontal(|ui| {
                let (resp, new_name) = text_cell(ui, &mut self.bufs, &format!("vn{i}"), &v.name, 54.0, Some(FULL));
                if resp.gained_focus() {
                    self.sel = vec![Sel::Var(i)];
                }
                if let Some(nn) = new_name {
                    let old = v.name.clone();
                    let nn2 = nn.trim().to_string();
                    if !self.geo.rename(&old, &nn, move |d| d.vars[i].as_mut().unwrap().name = nn2) {
                        self.bufs.remove(&format!("vn{i}"));
                    }
                }
                let right = 94.0;
                let fw = (w - 54.0 - right - 24.0).max(60.0);
                if v.expr.is_empty() {
                    let locked = self.geo.doc.params[v.param].locked;
                    let mut val = self.geo.doc.params[v.param].v;
                    if locked {
                        let speed = (val.abs() * 0.005).max(0.005);
                        let dv = ui.add_sized(vec2(fw, 22.0), egui::DragValue::new(&mut val).speed(speed).max_decimals(6));
                        if dv.gained_focus() || dv.drag_started() {
                            self.sel = vec![Sel::Var(i)];
                        }
                        if dv.drag_started() {
                            self.geo.checkpoint();
                        }
                        if dv.changed() && self.live {
                            if dv.dragged() {
                                let p = v.param;
                                let old = self.geo.doc.params[p].v;
                                self.geo.doc.params[p].v = val;
                                if !self.geo.solve_full() {
                                    self.geo.doc.params[p].v = old;
                                    self.geo.solve_full();
                                }
                            } else {
                                ops::set_var(&mut self.geo, i, val).ok();
                            }
                        }
                        if dv.drag_stopped() {
                            self.geo.generation += 1;
                        }
                    } else {
                        let (r, _) = ui.allocate_exact_size(vec2(fw, 22.0), Sense::click());
                        ui.painter().rect_filled(r, 6.0, Color32::from_rgba_unmultiplied(255, 255, 255, 6));
                        ui.painter().text(r.left_center() + vec2(8.0, 0.0), egui::Align2::LEFT_CENTER, fmt_val(val), FontId::proportional(14.0), ACCENT);
                        if r.is_finite() && ui.interact(r, ui.make_persistent_id(("vsel", i)), Sense::click()).clicked() {
                            self.sel = vec![Sel::Var(i)];
                        }
                    }
                    let tip = if locked { "Free — click to solve from constraints" } else { "Solved — click to set freely" };
                    if small_icon_btn(ui, if locked { Icon::Lock } else { Icon::Unlock }, !locked, tip).clicked() {
                        ops::set_var_free(&mut self.geo, i, !locked).ok();
                    }
                    if small_icon_btn(ui, Icon::Numeric, v.numeric, "Numeric — substitute the value as a number in formulas").clicked() {
                        ops::set_var_numeric(&mut self.geo, i, !v.numeric).ok();
                    }
                } else {
                    let err = self.geo.doc.parse_expr(&v.expr, 0).is_err();
                    let (resp, ne) = text_cell(ui, &mut self.bufs, &format!("ve{i}"), &v.expr, fw, Some(if err { ERR } else { DIMC }));
                    if resp.gained_focus() {
                        self.sel = vec![Sel::Var(i)];
                    }
                    if let Some(ne) = ne {
                        ops::set_var_expr(&mut self.geo, i, &ne).ok();
                    }
                    if small_icon_btn(ui, Icon::Numeric, v.numeric, "Numeric — substitute the value as a number in formulas").clicked() {
                        ops::set_var_numeric(&mut self.geo, i, !v.numeric).ok();
                    }
                }
                if small_icon_btn(ui, Icon::Close, false, "Delete").clicked() {
                    ops::delete_var(&mut self.geo, i).ok();
                }
            });
            self.row_bg(ui, r.response.rect, selected);
        }
    }

    fn curves_ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            section(ui, Icon::Curve);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if small_icon_btn(ui, Icon::Plus, false, "Add function  F").clicked() {
                    self.set_tool(Tool::Curve);
                }
            });
        });
        let w = ui.available_width();
        let mut k = 0;
        for i in 0..self.geo.doc.ents.len() {
            let Some(Entity { kind: Ent::Curve { src }, .. }) = self.geo.doc.ents[i].clone() else { continue };
            let col = CURVES[k % CURVES.len()];
            k += 1;
            let selected = self.sel.contains(&Sel::Ent(i));
            let r = ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(14.0, 22.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 5.0, col);
                let err = self.geo.doc.curve_def(&src).is_err();
                let (resp, ns) = text_cell(ui, &mut self.bufs, &format!("cv{i}"), &src, (w - 14.0 - 40.0).max(80.0), Some(if err { ERR } else { FULL }));
                if resp.gained_focus() {
                    self.sel = vec![Sel::Ent(i)];
                }
                if let Some(ns) = ns {
                    if self.geo.doc.curve_def(&ns).is_ok() {
                        if !ops::set_curve(&mut self.geo, i, &ns).unwrap_or(false) {
                            self.flash_ents(vec![i]);
                        }
                    }
                }
                if small_icon_btn(ui, Icon::Close, false, "Delete").clicked() {
                    ops::delete_ent(&mut self.geo, i).ok();
                }
            });
            self.row_bg(ui, r.response.rect, selected);
        }
    }

    fn dims_ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            section(ui, Icon::Dimension);
        });
        let w = ui.available_width();
        for i in 0..self.geo.doc.dims.len() {
            let Some(d) = self.geo.doc.dims[i].clone() else { continue };
            let selected = self.sel.contains(&Sel::Dim(i));
            let angle = d.kind.is_angle();
            let r = ui.horizontal(|ui| {
                let (resp, nn) = text_cell(ui, &mut self.bufs, &format!("dn{i}"), &d.name, 54.0, Some(MUTED));
                if resp.gained_focus() {
                    self.sel = vec![Sel::Dim(i)];
                }
                if let Some(nn) = nn {
                    let old = d.name.clone();
                    let nn2 = nn.trim().to_string();
                    if !self.geo.rename(&old, &nn, move |doc| doc.dims[i].as_mut().unwrap().name = nn2) {
                        self.bufs.remove(&format!("dn{i}"));
                    }
                }
                let fw = (w - 54.0 - 94.0 - 24.0).max(60.0);
                let shown = if !d.expr.is_empty() {
                    d.expr.clone()
                } else {
                    let v = if d.driving { self.geo.doc.params[d.param].v } else { self.geo.doc.measure_value(i) };
                    fmt_dim(v, angle)
                };
                let err = self.geo.errors.contains_key(&Owner::Dim(i)) || self.geo.conflicts.contains(&Owner::Dim(i));
                let col = if err { ERR } else if d.driving { DIMC } else { MUTED };
                let (resp, nv) = text_cell(ui, &mut self.bufs, &format!("dv{i}"), &shown, fw, Some(col));
                if resp.gained_focus() {
                    self.sel = vec![Sel::Dim(i)];
                }
                if let Some(nv) = nv {
                    self.commit_dim_text(i, &nv);
                }
                let (icon, tip) = if d.driving { (Icon::Driving, "Driving — click for reference") } else { (Icon::Reference, "Reference — click to drive") };
                if small_icon_btn(ui, icon, d.driving, tip).clicked() {
                    self.toggle_driving(i);
                }
                if small_icon_btn(ui, Icon::Numeric, d.numeric, "Numeric — substitute the value as a number in formulas").clicked() {
                    ops::set_dim_numeric(&mut self.geo, i, !d.numeric).ok();
                }
                if small_icon_btn(ui, Icon::Close, false, "Delete").clicked() {
                    ops::delete_dim(&mut self.geo, i).ok();
                    self.sel.retain(|s| *s != Sel::Dim(i));
                }
            });
            self.row_bg(ui, r.response.rect, selected);
        }
    }

    fn formula_ui(&mut self, ui: &mut Ui) {
        ui.set_min_size(ui.available_size());
        self.update_formula();
        if self.formula_busy() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(15));
        }
        let Some(f) = &self.formula else {
            let r = ui.available_rect_before_wrap();
            if self.formula_busy() {
                ui.put(egui::Rect::from_center_size(r.center(), vec2(24.0, 24.0)), egui::Spinner::new().color(MUTED));
                return;
            }
            icons::paint(ui.painter(), egui::Rect::from_center_size(r.center(), vec2(40.0, 40.0)), Icon::Latex, Color32::from_rgba_unmultiplied(255, 255, 255, 18));
            return;
        };
        let style = mathview::Style { sym: FULL, num: Color32::from_rgb(0xff, 0xc6, 0x8a), op: MUTED, func: Color32::from_rgb(0x7f, 0xd1, 0xff) };
        let mut copy: Option<String> = None;
        ui.horizontal(|ui| {
            let val = if f.is_angle { format!("{}  ({})", fmt_val(f.actual), fmt_dim(f.actual, true)) } else { fmt_val(f.actual) };
            ui.label(egui::RichText::new(format!("{} = {}", f.name, val)).color(MUTED).size(13.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // a longer search for a simpler formula is running
                if self.formula_busy() && f.exact {
                    ui.add(egui::Spinner::new().color(MUTED));
                }
                if f.bx.is_some() {
                    if small_icon_btn(ui, Icon::Latex, false, "Copy LaTeX").clicked() {
                        copy = Some(f.latex.clone());
                    }
                    if small_icon_btn(ui, Icon::Copy, false, "Copy").clicked() {
                        copy = Some(f.plain.clone());
                    }
                }
            });
        });
        if let Some(c) = copy {
            ui.ctx().copy_text(c);
        }
        // a numeric fallback, or an exact formula large enough that a simpler
        // form may exist: offer a longer search (10 s more per press), also
        // after the automatic longer searches have stopped
        let mut harder = false;
        if !f.settled() {
            let busy = self.formula_busy();
            let next = (self.search_level_of(f.target) + 1) as u64 * SEARCH_HARDER_MS / 1000;
            ui.horizontal(|ui| {
                let goal = if f.exact { "a simpler formula" } else { "an exact formula" };
                let tip = format!("Look for {goal} with {next} s more search time");
                if ui.add_enabled(!busy, egui::Button::new("Search harder")).on_hover_text(tip).clicked() {
                    harder = true;
                }
                if busy {
                    ui.add(egui::Spinner::new().color(MUTED));
                }
            });
        }
        if harder {
            self.search_harder();
            return;
        }
        let Some(f) = &self.formula else { return };
        if let Some(e) = &f.err {
            if !e.is_empty() {
                ui.label(egui::RichText::new(e).color(ERR));
            }
            return;
        }
        let Some(b) = &f.bx else { return };
        let eq = if f.exact { "=" } else { "≈" };
        let row = Bx::Row(vec![Bx::Sym(f.name.clone()), Bx::Op(eq.into()), b.clone()]);
        let mut rows = vec![row];
        for (n, d) in &f.defs {
            rows.push(Bx::Row(vec![Bx::Sym(n.clone()), Bx::Op("=".into()), d.clone()]));
        }
        let mismatch = !((f.value - f.actual).abs() <= 1e-6 * (1.0 + f.actual.abs()));
        egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
            let avail = ui.available_width() - 12.0;
            for (k, row) in rows.iter().enumerate() {
                let p = ui.painter().clone();
                let mut size: f32 = if k == 0 { 20.0 } else { 16.0 };
                let mut l = mathview::measure(&p, row, size);
                while l.w > avail && size > 12.0 {
                    size -= 1.0;
                    l = mathview::measure(&p, row, size);
                }
                let (r, _) = ui.allocate_exact_size(vec2(l.w + 8.0, l.a + l.d + 10.0), Sense::hover());
                mathview::draw(&p, row, size, r.min.x + 4.0, r.min.y + 5.0 + l.a, &style);
            }
            if mismatch && f.value.is_finite() {
                ui.label(egui::RichText::new(format!("≠ {}", fmt_val(f.value))).color(ERR).size(12.0));
            }
        });
    }
}
