//! Draws formulas from their LaTeX text with latex-rust. The library hands
//! formulas out as LaTeX (the same text C and JS get), so the app renders
//! that text rather than any internal form.

use eframe::egui::{Color32, Painter, Pos2, Rect, Shape, Vec2};
use latex_rust::{Color, Dim, EguiOptions, MathFont, latex_to_shapes};
use std::cell::RefCell;
use std::collections::HashMap;

/// Layout key: text, size in quarter points, color.
type Key = (String, u32, [u8; 4]);

thread_local! {
    static FONT: Option<MathFont> = MathFont::stix_two_math().ok();
    static CACHE: RefCell<HashMap<Key, Option<(Vec<Shape>, Rect)>>> = RefCell::new(HashMap::new());
}

/// Shapes of `latex` at `size` points, laid out with the top left of its
/// rect at the origin; None when the text does not parse. The glyphs are
/// triangle meshes, which egui does not anti-alias; the window's
/// multisampling smooths their edges.
fn layout(latex: &str, size: f32, color: Color32) -> Option<(Vec<Shape>, Rect)> {
    let q = (size * 4.0).round().max(1.0) as u32;
    let key = (latex.to_string(), q, color.to_array());
    if let Some(hit) = CACHE.with(|c| c.borrow().get(&key).cloned()) {
        return hit;
    }
    let made = FONT.with(|f| {
        let font = f.as_ref()?;
        let mut opt = EguiOptions::new();
        opt.font_size_pt = Dim::ratio(q as i64, 4);
        opt.color = Color::rgb(color.r(), color.g(), color.b());
        opt.display = true;
        // latex-rust scales its output by the pixels per point it is given,
        // while egui shapes are in points
        let (shapes, rect) = latex_to_shapes(latex, font, &opt, Pos2::ZERO, 1.0).ok()?;
        Some((shapes, rect))
    });
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() > 256 {
            c.clear();
        }
        c.insert(key, made.clone());
    });
    made
}

/// Size of `latex` drawn at `size` points; None when it does not parse.
pub fn measure(latex: &str, size: f32, color: Color32) -> Option<Vec2> {
    layout(latex, size, color).map(|(_, r)| r.size())
}

/// Draws `latex` at `size` points with its top left at `at`; returns the
/// rect it covers, None when the text does not parse.
pub fn draw(p: &Painter, latex: &str, size: f32, at: Pos2, color: Color32) -> Option<Rect> {
    let (shapes, rect) = layout(latex, size, color)?;
    let d = at - rect.min;
    p.extend(shapes.into_iter().map(|mut s| {
        s.translate(d);
        s
    }));
    Some(rect.translate(d))
}
