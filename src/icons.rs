//! Vector icons painted directly with the egui painter.

use eframe::egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Select,
    Point,
    Line,
    Circle,
    Arc,
    Ellipse,
    Hyperbola,
    Parabola,
    Curve,
    Dimension,
    Coincident,
    OnCurve,
    Midpoint,
    Horizontal,
    Vertical,
    Parallel,
    Perpendicular,
    Collinear,
    Equal,
    Tangent,
    Concentric,
    Symmetric,
    Lock,
    Unlock,
    Distance,
    HDist,
    VDist,
    Angle,
    Radius,
    Diameter,
    AxisA,
    AxisB,
    Undo,
    Redo,
    Fit,
    Eye,
    Plus,
    Trash,
    Copy,
    Latex,
    Construction,
    Save,
    Open,
    Variable,
    Driving,
    Reference,
    Supplement,
    Close,
    Intersect,
    PerpLine,
    ParLine,
    PerpBisector,
    AngleBisector,
    TangentLine,
    Reflect,
    CoordX,
    CoordY,
    Foci,
    Directrix,
    Asymptotes,
    Circle3,
    Arc3,
    EllipseFoci,
    HyperbolaFoci,
    ParabolaFocus,
}

fn poly(p: &Painter, pts: Vec<Pos2>, s: Stroke) {
    p.add(Shape::line(pts, s));
}

fn arc_pts(c: Pos2, r: f32, a0: f32, a1: f32, n: usize) -> Vec<Pos2> {
    (0..=n)
        .map(|i| {
            let t = a0 + (a1 - a0) * i as f32 / n as f32;
            c + vec2(t.cos(), -t.sin()) * r
        })
        .collect()
}

fn arrow_head(p: &Painter, tip: Pos2, dir: eframe::egui::Vec2, s: Stroke) {
    let d = dir.normalized();
    let n = vec2(-d.y, d.x);
    let l = 3.5;
    p.line_segment([tip, tip - d * l + n * l * 0.7], s);
    p.line_segment([tip, tip - d * l - n * l * 0.7], s);
}

/// Paints `icon` centered in `rect`.
pub fn paint(p: &Painter, rect: Rect, icon: Icon, col: Color32) {
    let c = rect.center();
    let h = rect.width().min(rect.height()) * 0.5 * 0.78;
    let s = Stroke::new(1.6, col);
    let thin = Stroke::new(1.1, col);
    let at = |x: f32, y: f32| pos2(c.x + x * h, c.y + y * h);
    let dot = |q: Pos2, r: f32| p.circle_filled(q, r, col);
    match icon {
        Icon::Select => {
            let pts = vec![at(-0.55, -0.85), at(-0.55, 0.6), at(-0.18, 0.25), at(0.12, 0.85), at(0.35, 0.74), at(0.06, 0.15), at(0.55, 0.12), at(-0.55, -0.85)];
            p.add(Shape::convex_polygon(pts[..7].to_vec(), Color32::TRANSPARENT, Stroke::NONE));
            poly(p, pts, s);
        }
        Icon::Point => {
            dot(c, h * 0.32);
            p.circle_stroke(c, h * 0.62, thin);
        }
        Icon::Line => {
            p.line_segment([at(-0.7, 0.7), at(0.7, -0.7)], s);
            dot(at(-0.7, 0.7), 2.4);
            dot(at(0.7, -0.7), 2.4);
        }
        Icon::Circle => {
            p.circle_stroke(c, h * 0.8, s);
            dot(c, 1.8);
        }
        Icon::Arc => {
            poly(p, arc_pts(at(-0.4, 0.5), h * 1.1, 0.0, PI * 0.5, 20), s);
            dot(at(-0.4, 0.5), 1.8);
        }
        Icon::Ellipse => {
            let pts: Vec<Pos2> = (0..=40)
                .map(|i| {
                    let t = TAU * i as f32 / 40.0;
                    let (x, y) = (0.9 * t.cos(), 0.5 * t.sin());
                    let (cs, sn) = ((-0.5f32).cos(), (-0.5f32).sin());
                    at(x * cs - y * sn, x * sn + y * cs)
                })
                .collect();
            poly(p, pts, s);
        }
        Icon::Hyperbola => {
            for sgn in [-1.0f32, 1.0] {
                let pts: Vec<Pos2> = (0..=20)
                    .map(|i| {
                        let u = -1.3 + 2.6 * i as f32 / 20.0;
                        at(sgn * (0.25 * u.cosh()), 0.6 * u.sinh())
                    })
                    .collect();
                poly(p, pts, s);
            }
        }
        Icon::Parabola => {
            let pts: Vec<Pos2> = (0..=24).map(|i| {
                let x = -0.85 + 1.7 * i as f32 / 24.0;
                at(x, 0.75 - 1.6 * x * x)
            }).collect();
            poly(p, pts, s);
            dot(at(0.0, 0.25), 1.6);
        }
        Icon::Curve => {
            let pts: Vec<Pos2> = (0..=30).map(|i| {
                let x = -0.9 + 1.8 * i as f32 / 30.0;
                at(x, -0.55 * (x * 3.3).sin())
            }).collect();
            poly(p, pts, s);
            p.line_segment([at(-0.95, 0.85), at(0.95, 0.85)], Stroke::new(0.8, col.gamma_multiply(0.5)));
        }
        Icon::Dimension | Icon::Distance => {
            p.line_segment([at(-0.85, -0.6), at(-0.85, 0.6)], thin);
            p.line_segment([at(0.85, -0.6), at(0.85, 0.6)], thin);
            p.line_segment([at(-0.8, 0.0), at(0.8, 0.0)], s);
            arrow_head(p, at(-0.8, 0.0), vec2(-1.0, 0.0), s);
            arrow_head(p, at(0.8, 0.0), vec2(1.0, 0.0), s);
        }
        Icon::HDist => {
            p.line_segment([at(-0.85, -0.8), at(-0.85, 0.8)], thin);
            p.line_segment([at(0.85, -0.8), at(0.85, 0.8)], thin);
            p.line_segment([at(-0.8, 0.3), at(0.8, 0.3)], s);
            arrow_head(p, at(-0.8, 0.3), vec2(-1.0, 0.0), s);
            arrow_head(p, at(0.8, 0.3), vec2(1.0, 0.0), s);
            p.line_segment([at(-0.4, -0.55), at(0.4, -0.55)], thin);
        }
        Icon::VDist => {
            p.line_segment([at(-0.8, -0.85), at(0.8, -0.85)], thin);
            p.line_segment([at(-0.8, 0.85), at(0.8, 0.85)], thin);
            p.line_segment([at(-0.3, -0.8), at(-0.3, 0.8)], s);
            arrow_head(p, at(-0.3, -0.8), vec2(0.0, -1.0), s);
            arrow_head(p, at(-0.3, 0.8), vec2(0.0, 1.0), s);
            p.line_segment([at(0.55, -0.4), at(0.55, 0.4)], thin);
        }
        Icon::Coincident => {
            p.circle_stroke(at(-0.25, 0.0), h * 0.45, s);
            p.circle_stroke(at(0.25, 0.0), h * 0.45, s);
            dot(c, 2.0);
        }
        Icon::OnCurve => {
            poly(p, arc_pts(at(0.0, 1.4), h * 1.6, 0.2 * PI, 0.8 * PI, 20), s);
            dot(at(0.0, -0.2), 3.0);
        }
        Icon::Midpoint => {
            p.line_segment([at(-0.9, 0.5), at(0.9, -0.5)], s);
            p.circle_filled(c, 3.0, col);
            p.line_segment([at(-0.3, -0.6), at(0.3, 0.6)], thin);
        }
        Icon::Horizontal => {
            p.line_segment([at(-0.85, 0.0), at(0.85, 0.0)], Stroke::new(2.0, col));
            dot(at(-0.85, 0.0), 2.2);
            dot(at(0.85, 0.0), 2.2);
        }
        Icon::Vertical => {
            p.line_segment([at(0.0, -0.85), at(0.0, 0.85)], Stroke::new(2.0, col));
            dot(at(0.0, -0.85), 2.2);
            dot(at(0.0, 0.85), 2.2);
        }
        Icon::Parallel => {
            p.line_segment([at(-0.6, 0.8), at(0.0, -0.8)], s);
            p.line_segment([at(0.0, 0.8), at(0.6, -0.8)], s);
        }
        Icon::Perpendicular => {
            p.line_segment([at(-0.8, 0.7), at(0.8, 0.7)], s);
            p.line_segment([at(0.0, 0.7), at(0.0, -0.8)], s);
            poly(p, vec![at(0.0, 0.35), at(0.35, 0.35), at(0.35, 0.7)], thin);
        }
        Icon::Collinear => {
            p.line_segment([at(-0.9, 0.6), at(-0.1, 0.07)], s);
            p.line_segment([at(0.15, -0.1), at(0.9, -0.6)], s);
            p.add(Shape::dashed_line(&[at(-0.9, 0.6), at(0.9, -0.6)], thin, 2.0, 3.0));
        }
        Icon::Equal => {
            p.line_segment([at(-0.7, -0.3), at(0.7, -0.3)], Stroke::new(2.0, col));
            p.line_segment([at(-0.7, 0.3), at(0.7, 0.3)], Stroke::new(2.0, col));
        }
        Icon::Tangent => {
            p.circle_stroke(at(0.0, 0.2), h * 0.6, s);
            p.line_segment([at(-0.95, -0.4), at(0.95, -0.4)], s);
        }
        Icon::Concentric => {
            p.circle_stroke(c, h * 0.85, s);
            p.circle_stroke(c, h * 0.4, s);
            dot(c, 1.5);
        }
        Icon::Symmetric => {
            p.add(Shape::dashed_line(&[at(0.0, -0.95), at(0.0, 0.95)], thin, 2.0, 2.0));
            dot(at(-0.6, 0.0), 2.8);
            dot(at(0.6, 0.0), 2.8);
        }
        Icon::Lock | Icon::Unlock => {
            let body = Rect::from_min_max(at(-0.6, -0.05), at(0.6, 0.85));
            p.rect_filled(body, 2.0, col);
            let pts = if icon == Icon::Lock {
                arc_pts(at(0.0, -0.3), h * 0.4, 0.0, PI, 16)
            } else {
                arc_pts(at(0.55, -0.4), h * 0.4, 0.0, PI, 16)
            };
            let mut v = pts;
            if icon == Icon::Lock {
                v.insert(0, at(0.4, -0.05));
                v.push(at(-0.4, -0.05));
            } else {
                v.insert(0, at(0.95, -0.4));
                v.push(at(0.15, -0.05));
            }
            poly(p, v, s);
        }
        Icon::Angle => {
            p.line_segment([at(-0.8, 0.7), at(0.9, 0.7)], s);
            p.line_segment([at(-0.8, 0.7), at(0.5, -0.8)], s);
            poly(p, arc_pts(at(-0.8, 0.7), h * 0.9, 0.0, 0.85, 12), thin);
        }
        Icon::Radius => {
            p.circle_stroke(c, h * 0.85, thin);
            p.line_segment([c, at(0.6, -0.6)], s);
            arrow_head(p, at(0.6, -0.6), vec2(1.0, -1.0), s);
            dot(c, 1.8);
        }
        Icon::Diameter => {
            p.circle_stroke(c, h * 0.85, thin);
            p.line_segment([at(-0.6, 0.6), at(0.6, -0.6)], s);
            arrow_head(p, at(0.6, -0.6), vec2(1.0, -1.0), s);
            arrow_head(p, at(-0.6, 0.6), vec2(-1.0, 1.0), s);
        }
        Icon::AxisA | Icon::AxisB => {
            let pts: Vec<Pos2> = (0..=40).map(|i| {
                let t = TAU * i as f32 / 40.0;
                at(0.9 * t.cos(), 0.55 * t.sin())
            }).collect();
            poly(p, pts, thin);
            if icon == Icon::AxisA {
                p.line_segment([c, at(0.9, 0.0)], s);
            } else {
                p.line_segment([c, at(0.0, -0.55)], s);
            }
            dot(c, 1.6);
        }
        Icon::Undo | Icon::Redo => {
            let flip = if icon == Icon::Undo { -1.0 } else { 1.0 };
            let pts: Vec<Pos2> = arc_pts(at(0.0, 0.15), h * 0.6, -0.5 * PI, 0.75 * PI, 18)
                .into_iter()
                .map(|q| pos2(c.x + (q.x - c.x) * -flip, q.y))
                .collect();
            let tip = pts[pts.len() - 1];
            poly(p, pts.clone(), s);
            let d = tip - pts[pts.len() - 3];
            arrow_head(p, tip, d, s);
        }
        Icon::Fit => {
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let k = at(0.8 * sx, 0.8 * sy);
                p.line_segment([k, k - vec2(0.45 * h * sx, 0.0)], s);
                p.line_segment([k, k - vec2(0.0, 0.45 * h * sy)], s);
            }
            dot(c, 2.0);
        }
        Icon::Eye => {
            let top = arc_pts(at(0.0, 0.7), h * 1.1, 0.25 * PI, 0.75 * PI, 14);
            let bot = arc_pts(at(0.0, -0.7), h * 1.1, -0.75 * PI, -0.25 * PI, 14);
            poly(p, top, s);
            poly(p, bot, s);
            p.circle_filled(c, h * 0.28, col);
        }
        Icon::Plus => {
            p.line_segment([at(-0.6, 0.0), at(0.6, 0.0)], s);
            p.line_segment([at(0.0, -0.6), at(0.0, 0.6)], s);
        }
        Icon::Close => {
            p.line_segment([at(-0.45, -0.45), at(0.45, 0.45)], s);
            p.line_segment([at(-0.45, 0.45), at(0.45, -0.45)], s);
        }
        Icon::Trash => {
            p.line_segment([at(-0.75, -0.55), at(0.75, -0.55)], s);
            poly(p, vec![at(-0.55, -0.55), at(-0.42, 0.85), at(0.42, 0.85), at(0.55, -0.55)], s);
            poly(p, vec![at(-0.25, -0.55), at(-0.2, -0.85), at(0.2, -0.85), at(0.25, -0.55)], thin);
        }
        Icon::Copy => {
            p.rect_stroke(Rect::from_min_max(at(-0.7, -0.4), at(0.3, 0.8)), 2.0, s, eframe::egui::StrokeKind::Middle);
            poly(p, vec![at(-0.3, -0.4), at(-0.3, -0.8), at(0.7, -0.8), at(0.7, 0.4), at(0.3, 0.4)], thin);
        }
        Icon::Latex => {
            // stylized radical
            poly(p, vec![at(-0.9, 0.1), at(-0.65, -0.05), at(-0.35, 0.75), at(0.0, -0.75), at(0.9, -0.75)], s);
            p.line_segment([at(0.2, -0.3), at(0.7, 0.3)], thin);
            p.line_segment([at(0.2, 0.3), at(0.7, -0.3)], thin);
        }
        Icon::Construction => {
            p.add(Shape::dashed_line(&[at(-0.8, 0.8), at(0.8, -0.8)], s, 3.0, 2.5));
        }
        Icon::Save => {
            p.line_segment([at(0.0, -0.85), at(0.0, 0.35)], s);
            arrow_head(p, at(0.0, 0.35), vec2(0.0, 1.0), s);
            poly(p, vec![at(-0.8, 0.3), at(-0.8, 0.85), at(0.8, 0.85), at(0.8, 0.3)], s);
        }
        Icon::Open => {
            poly(p, vec![at(-0.85, 0.75), at(-0.85, -0.6), at(-0.3, -0.6), at(-0.15, -0.4), at(0.75, -0.4), at(0.75, 0.75), at(-0.85, 0.75)], s);
        }
        Icon::Variable => {
            p.line_segment([at(-0.7, -0.5), at(-0.05, 0.5)], s);
            p.line_segment([at(-0.05, -0.5), at(-0.7, 0.5)], s);
            p.line_segment([at(0.25, -0.15), at(0.85, -0.15)], thin);
            p.line_segment([at(0.25, 0.2), at(0.85, 0.2)], thin);
        }
        Icon::Driving => {
            p.line_segment([at(-0.85, 0.0), at(0.85, 0.0)], s);
            arrow_head(p, at(-0.85, 0.0), vec2(-1.0, 0.0), s);
            arrow_head(p, at(0.85, 0.0), vec2(1.0, 0.0), s);
            dot(c, 2.6);
        }
        Icon::Reference => {
            poly(p, arc_pts(at(0.5, 0.0), h * 1.1, 0.75 * PI, 1.25 * PI, 10), s);
            poly(p, arc_pts(at(-0.5, 0.0), h * 1.1, -0.25 * PI, 0.25 * PI, 10), s);
            p.line_segment([at(-0.3, 0.0), at(0.3, 0.0)], thin);
        }
        Icon::Intersect => {
            poly(p, arc_pts(at(0.0, 1.6), h * 1.9, 0.18 * PI, 0.82 * PI, 20), s);
            p.line_segment([at(-0.9, 0.8), at(0.9, -0.8)], s);
            p.circle_filled(at(0.0, -0.3), 2.8, col);
        }
        Icon::PerpLine => {
            p.line_segment([at(-0.9, 0.6), at(0.9, 0.6)], thin);
            p.line_segment([at(0.1, 0.95), at(0.1, -0.95)], s);
            poly(p, vec![at(0.1, 0.3), at(0.4, 0.3), at(0.4, 0.6)], thin);
            dot(at(0.1, -0.35), 2.6);
        }
        Icon::ParLine => {
            p.line_segment([at(-0.9, 0.75), at(0.4, -0.75)], thin);
            p.line_segment([at(-0.4, 0.75), at(0.9, -0.75)], s);
            dot(at(0.25, 0.0), 2.6);
        }
        Icon::PerpBisector => {
            p.line_segment([at(-0.85, 0.45), at(0.85, 0.45)], s);
            dot(at(-0.85, 0.45), 2.2);
            dot(at(0.85, 0.45), 2.2);
            p.add(Shape::dashed_line(&[at(0.0, 0.95), at(0.0, -0.95)], s, 2.5, 2.0));
        }
        Icon::AngleBisector => {
            p.line_segment([at(-0.85, 0.75), at(0.9, 0.75)], s);
            p.line_segment([at(-0.85, 0.75), at(0.45, -0.85)], s);
            p.add(Shape::dashed_line(&[at(-0.85, 0.75), at(0.95, -0.25)], thin, 2.5, 2.0));
        }
        Icon::TangentLine => {
            p.circle_stroke(at(0.25, 0.25), h * 0.55, s);
            p.line_segment([at(-0.9, -0.85), at(0.75, -0.25)], s);
            dot(at(-0.9, -0.85), 2.4);
        }
        Icon::Reflect => {
            p.add(Shape::dashed_line(&[at(0.0, -0.95), at(0.0, 0.95)], thin, 2.0, 2.0));
            p.circle_filled(at(-0.6, -0.3), 2.8, col);
            p.circle_stroke(at(0.6, -0.3), 2.8, s);
            poly(p, arc_pts(at(0.0, 0.1), h * 0.6, 0.2 * PI, 0.8 * PI, 10), thin);
        }
        Icon::CoordX | Icon::CoordY => {
            p.line_segment([at(-0.9, 0.85), at(0.95, 0.85)], thin);
            p.line_segment([at(-0.85, 0.9), at(-0.85, -0.95)], thin);
            let q = at(0.45, -0.35);
            dot(q, 2.6);
            if icon == Icon::CoordX {
                p.add(Shape::dashed_line(&[q, at(0.45, 0.85)], thin, 2.0, 2.0));
                p.line_segment([at(-0.85, 0.45), at(0.45, 0.45)], s);
            } else {
                p.add(Shape::dashed_line(&[q, at(-0.85, -0.35)], thin, 2.0, 2.0));
                p.line_segment([at(0.05, -0.35), at(0.05, 0.85)], s);
            }
        }
        Icon::Foci => {
            let pts: Vec<Pos2> = (0..=40)
                .map(|i| {
                    let t = TAU * i as f32 / 40.0;
                    at(0.9 * t.cos(), 0.55 * t.sin())
                })
                .collect();
            poly(p, pts, thin);
            dot(at(-0.55, 0.0), 2.4);
            dot(at(0.55, 0.0), 2.4);
        }
        Icon::Directrix => {
            let pts: Vec<Pos2> = (0..=24)
                .map(|i| {
                    let y = -0.9 + 1.8 * i as f32 / 24.0;
                    at(-0.35 + 0.9 * y * y, y)
                })
                .collect();
            poly(p, pts, thin);
            dot(at(0.0, 0.0), 2.2);
            p.line_segment([at(-0.75, -0.95), at(-0.75, 0.95)], s);
        }
        Icon::Asymptotes => {
            p.line_segment([at(-0.9, -0.75), at(0.9, 0.75)], s);
            p.line_segment([at(-0.9, 0.75), at(0.9, -0.75)], s);
            for sgn in [-1.0f32, 1.0] {
                let pts: Vec<Pos2> = (0..=16)
                    .map(|i| {
                        let u = -1.2 + 2.4 * i as f32 / 16.0;
                        at(sgn * 0.35 * u.cosh(), 0.3 * u.sinh())
                    })
                    .collect();
                poly(p, pts, thin);
            }
        }
        Icon::Circle3 => {
            p.circle_stroke(c, h * 0.8, s);
            for t in [0.3f32, 2.3, 4.2] {
                dot(c + vec2(t.cos(), -t.sin()) * h * 0.8, 2.3);
            }
        }
        Icon::Arc3 => {
            poly(p, arc_pts(at(0.0, 0.6), h * 1.15, 0.12 * PI, 0.88 * PI, 24), s);
            for t in [0.12f32, 0.5, 0.88] {
                let a = t * PI;
                dot(at(0.0, 0.6) + vec2(a.cos(), -a.sin()) * h * 1.15, 2.3);
            }
        }
        Icon::EllipseFoci => {
            let pts: Vec<Pos2> = (0..=40)
                .map(|i| {
                    let t = TAU * i as f32 / 40.0;
                    at(0.9 * t.cos(), 0.55 * t.sin())
                })
                .collect();
            poly(p, pts, s);
            dot(at(-0.55, 0.0), 2.2);
            dot(at(0.55, 0.0), 2.2);
            p.circle_stroke(at(0.0, -0.55), 2.4, thin);
        }
        Icon::HyperbolaFoci => {
            for sgn in [-1.0f32, 1.0] {
                let pts: Vec<Pos2> = (0..=16)
                    .map(|i| {
                        let u = -1.1 + 2.2 * i as f32 / 16.0;
                        at(sgn * 0.4 * u.cosh(), 0.65 * u.sinh())
                    })
                    .collect();
                poly(p, pts, s);
                dot(at(sgn * 0.85, 0.0), 2.2);
            }
        }
        Icon::ParabolaFocus => {
            let pts: Vec<Pos2> = (0..=24)
                .map(|i| {
                    let y = -0.9 + 1.8 * i as f32 / 24.0;
                    at(-0.2 + 0.85 * y * y, y)
                })
                .collect();
            poly(p, pts, s);
            dot(at(0.1, 0.0), 2.4);
            p.add(Shape::dashed_line(&[at(-0.6, -0.95), at(-0.6, 0.95)], thin, 2.0, 2.0));
        }
        Icon::Supplement => {
            p.line_segment([at(-0.9, 0.5), at(0.9, 0.5)], s);
            p.line_segment([at(0.0, 0.5), at(0.6, -0.7)], s);
            poly(p, arc_pts(at(0.0, 0.5), h * 0.55, 1.1, PI, 12), thin);
        }
    }
}
