// release builds on Windows use the GUI subsystem, so launching gxpad.exe opens no console window
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use gxpad::app::*;
use gxpad::ops;
use eframe::egui::{self, Color32, CornerRadius, Stroke, vec2};
use std::collections::HashMap;

struct GxApp {
    app: App,
    cache: HashMap<usize, (u64, Vec<Vec<egui::Pos2>>)>,
}

impl eframe::App for GxApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.app.frame_ui(ui, &mut self.cache);
    }
}

fn style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    let mut v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.extreme_bg_color = Color32::from_rgb(0x12, 0x14, 0x19);
    v.faint_bg_color = SURFACE;
    v.window_stroke = Stroke::new(1.0, LINE);
    v.selection.bg_fill = Color32::from_rgba_unmultiplied(0x6c, 0xb6, 0xff, 70);
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.text_cursor.stroke = Stroke::new(1.5, ACCENT);
    let r = CornerRadius::same(6);
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.corner_radius = r;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    v.widgets.inactive.bg_fill = SURFACE;
    v.widgets.inactive.weak_bg_fill = SURFACE;
    v.widgets.inactive.bg_stroke = Stroke::NONE;
    v.widgets.hovered.bg_fill = SURFACE_HI;
    v.widgets.hovered.weak_bg_fill = SURFACE_HI;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x3a, 0x40, 0x4c));
    v.widgets.active.bg_fill = SURFACE_HI;
    v.widgets.active.weak_bg_fill = SURFACE_HI;
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(0xc8, 0xcd, 0xd6));
    ctx.set_visuals_of(egui::Theme::Dark, v);
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = vec2(6.0, 6.0);
        s.spacing.button_padding = vec2(8.0, 4.0);
        s.spacing.interact_size.y = 24.0;
        if let Some(t) = s.text_styles.get_mut(&egui::TextStyle::Body) {
            t.size = 14.0;
        }
    });
}

fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([820.0, 520.0])
            .with_title("gxpad"),
        // smooths the edges of the formula glyphs, which are drawn as
        // triangle meshes that egui's own anti-aliasing does not cover
        multisampling: 4,
        ..Default::default()
    };
    eframe::run_native(
        "gxpad",
        opts,
        Box::new(|cc| {
            style(&cc.egui_ctx);
            let mut app = App::new();
            if let Some(path) = std::env::args().nth(1) {
                if let Ok(s) = std::fs::read_to_string(&path) {
                    if let Ok(mut d) = ops::Document::from_gxp(&s) {
                        ops::solve(&mut d);
                        app.geo = d;
                        app.path = Some(path.into());
                        // an optional second argument: the dimension to select
                        if let Some(name) = std::env::args().nth(2) {
                            if let Some(i) = (0..ops::dim_count(&app.geo)).find(|&i| ops::dim_name(&app.geo, i).is_ok_and(|n| n == name)) {
                                app.sel = vec![Sel::Dim(i)];
                            }
                        }
                    }
                }
            }
            Ok(Box::new(GxApp { app, cache: HashMap::new() }))
        }),
    )
}
