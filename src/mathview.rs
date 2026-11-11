//! Renders a math layout tree (fractions, radicals, exponents) with the egui painter.

use crate::expr::bx::{Bx, split_sub};
use eframe::egui::{Color32, FontId, Painter, Pos2, Shape, Stroke, pos2};

#[derive(Clone, Copy)]
pub struct Style {
    pub sym: Color32,
    pub num: Color32,
    pub op: Color32,
    pub func: Color32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Lay {
    pub w: f32,
    pub a: f32,
    pub d: f32,
}

const MIN: f32 = 9.0;

fn axis(size: f32) -> f32 {
    size * 0.3
}

fn text_lay(p: &Painter, t: &str, size: f32) -> Lay {
    let g = p.layout_no_wrap(t.to_string(), FontId::proportional(size), Color32::WHITE);
    let h = g.size().y;
    Lay { w: g.size().x, a: h * 0.5 + axis(size), d: h * 0.5 - axis(size) }
}

fn text_draw(p: &Painter, t: &str, size: f32, x: f32, base: f32, col: Color32) -> f32 {
    let g = p.layout_no_wrap(t.to_string(), FontId::proportional(size), col);
    let h = g.size().y;
    let w = g.size().x;
    p.galley(pos2(x, base - axis(size) - h * 0.5), g, col);
    w
}

fn sym_lay(p: &Painter, s: &str, size: f32) -> Lay {
    let (b, sub) = split_sub(s);
    let mut l = text_lay(p, b, size);
    if let Some(sub) = sub {
        let ls = text_lay(p, sub, (size * 0.68).max(MIN));
        l.w += ls.w + 1.0;
        l.d = l.d.max(ls.d + size * 0.22);
    }
    l
}

fn op_pad(op: &str, size: f32) -> (f32, f32) {
    match op {
        "," => (0.0, size * 0.3),
        "/" => (size * 0.05, size * 0.05),
        "·" => (size * 0.12, size * 0.12),
        _ => (size * 0.24, size * 0.24),
    }
}

/// Horizontal offset of an exponent: stacked over a subscript when present.
fn sup_x(p: &Painter, base: &Bx, size: f32, full: f32) -> f32 {
    if let Bx::Sym(s) = base {
        let (b, sub) = split_sub(s);
        if sub.is_some() {
            return text_lay(p, b, size).w + 1.0;
        }
    }
    full
}

pub fn measure(p: &Painter, b: &Bx, size: f32) -> Lay {
    match b {
        Bx::Sym(s) => sym_lay(p, s, size),
        Bx::Num(s) | Bx::Fn(s) => text_lay(p, s, size),
        Bx::Op(o) => {
            let mut l = text_lay(p, o, size);
            let (a, c) = op_pad(o, size);
            l.w += a + c;
            l
        }
        Bx::Row(v) => {
            let mut l = Lay::default();
            for x in v {
                let m = measure(p, x, size);
                l.w += m.w;
                l.a = l.a.max(m.a);
                l.d = l.d.max(m.d);
            }
            l
        }
        Bx::Frac(n, d) => {
            let s2 = (size * 0.86).max(MIN);
            let ln = measure(p, n, s2);
            let ld = measure(p, d, s2);
            let gap = size * 0.14;
            Lay { w: ln.w.max(ld.w) + size * 0.3, a: axis(size) + gap + ln.a + ln.d, d: gap + ld.a + ld.d - axis(size) }
        }
        Bx::Root(x, idx) => {
            let l = measure(p, x, size);
            let extra = if idx.is_some() { size * 0.25 } else { 0.0 };
            Lay { w: l.w + size * 0.7 + extra, a: l.a + size * 0.18, d: l.d + 1.0 }
        }
        Bx::Sup(base, e) => {
            let lb = measure(p, base, size);
            let le = measure(p, e, (size * 0.68).max(MIN));
            let shift = lb.a * 0.62;
            let x0 = sup_x(p, base, size, lb.w);
            Lay { w: lb.w.max(x0 + le.w + 1.5), a: lb.a.max(shift + le.a), d: lb.d }
        }
        Bx::Paren(x) | Bx::Abs(x) => {
            let l = measure(p, x, size);
            let pw = size * 0.32 + (l.a + l.d) * 0.05;
            Lay { w: l.w + 2.0 * pw, a: l.a + 1.0, d: l.d + 1.0 }
        }
    }
}

/// Draws `b` with its left edge at `x` and baseline at `base`; returns its width.
pub fn draw(p: &Painter, b: &Bx, size: f32, x: f32, base: f32, st: &Style) -> f32 {
    match b {
        Bx::Sym(s) => {
            let (bs, sub) = split_sub(s);
            let mut w = text_draw(p, bs, size, x, base, st.sym);
            if let Some(sub) = sub {
                let ss = (size * 0.68).max(MIN);
                w += 1.0 + text_draw(p, sub, ss, x + w + 1.0, base + size * 0.28, st.sym);
            }
            w
        }
        Bx::Num(s) => text_draw(p, s, size, x, base, st.num),
        Bx::Fn(s) => text_draw(p, s, size, x, base, st.func),
        Bx::Op(o) => {
            let (a, c) = op_pad(o, size);
            a + text_draw(p, o, size, x + a, base, st.op) + c
        }
        Bx::Row(v) => {
            let mut cx = x;
            for e in v {
                cx += draw(p, e, size, cx, base, st);
            }
            cx - x
        }
        Bx::Frac(n, d) => {
            let s2 = (size * 0.86).max(MIN);
            let ln = measure(p, n, s2);
            let ld = measure(p, d, s2);
            let w = ln.w.max(ld.w) + size * 0.3;
            let gap = size * 0.14;
            let bar = base - axis(size);
            p.line_segment([pos2(x + 1.0, bar), pos2(x + w - 1.0, bar)], Stroke::new(1.0, st.op));
            draw(p, n, s2, x + (w - ln.w) * 0.5, bar - gap - ln.d, st);
            draw(p, d, s2, x + (w - ld.w) * 0.5, bar + gap + ld.a, st);
            w
        }
        Bx::Root(inner, idx) => {
            let l = measure(p, inner, size);
            let extra = if idx.is_some() { size * 0.25 } else { 0.0 };
            let top = base - l.a - size * 0.1;
            let bot = base + l.d;
            let x0 = x + extra;
            let sw = size * 0.55;
            let pts = vec![
                pos2(x0, base - axis(size) + size * 0.05),
                pos2(x0 + sw * 0.25, base - axis(size) - size * 0.05),
                pos2(x0 + sw * 0.55, bot),
                pos2(x0 + sw, top),
                pos2(x0 + sw + l.w + size * 0.12, top),
            ];
            p.add(Shape::line(pts, Stroke::new(1.1, st.op)));
            if let Some(n) = idx {
                text_draw(p, &n.to_string(), (size * 0.55).max(MIN), x, base - axis(size) - size * 0.35, st.num);
            }
            draw(p, inner, size, x0 + sw + size * 0.08, base, st);
            l.w + size * 0.7 + extra
        }
        Bx::Sup(b, e) => {
            let lb = measure(p, b, size);
            let es = (size * 0.68).max(MIN);
            let le = measure(p, e, es);
            let wb = draw(p, b, size, x, base, st);
            let shift = lb.a * 0.62;
            let x0 = sup_x(p, b, size, wb);
            draw(p, e, es, x + x0 + 1.5, base - shift - le.d.max(0.0) * 0.3, st);
            wb.max(x0 + le.w + 1.5)
        }
        Bx::Paren(inner) | Bx::Abs(inner) => {
            let l = measure(p, inner, size);
            let pw = size * 0.32 + (l.a + l.d) * 0.05;
            let top = base - l.a;
            let bot = base + l.d;
            let hgt = bot - top;
            let stroke = Stroke::new(1.1, st.op);
            let is_abs = matches!(b, Bx::Abs(_));
            for (side, cx) in [(-1.0f32, x + pw * 0.62), (1.0, x + pw + l.w + pw * 0.38)] {
                if is_abs {
                    p.line_segment([pos2(cx, top), pos2(cx, bot)], stroke);
                } else {
                    let bulge = pw * 0.45;
                    let pts: Vec<Pos2> = (0..=16)
                        .map(|i| {
                            let t = i as f32 / 16.0;
                            let y = top + hgt * t;
                            let k = 4.0 * t * (1.0 - t);
                            pos2(cx + side * (bulge * k - bulge * 0.5), y)
                        })
                        .collect();
                    p.add(Shape::line(pts, stroke));
                }
            }
            draw(p, inner, size, x + pw, base, st);
            l.w + 2.0 * pw
        }
    }
}
