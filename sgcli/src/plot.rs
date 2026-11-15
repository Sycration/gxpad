//! `plot`: the construction drawn in characters. Curves are `#`, helper
//! (aux) curves `:`, points `*` with their names.

use sggeo::ops::{self, Document, EntKind};

pub fn plot(doc: &Document, w: usize, h: usize) -> Vec<String> {
    let (w, h) = (w.clamp(10, 400), h.clamp(5, 200));
    let shown = |e: usize| ops::ent_kind(doc, e).is_ok() && !ops::ent_hidden(doc, e).unwrap_or(true);
    let ents: Vec<usize> = (0..ops::ent_count(doc)).filter(|&e| shown(e)).collect();
    // the window: around the points and circles, a character twice as tall as wide
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    let mut grow = |p: [f64; 2], r: f64| {
        for k in 0..2 {
            lo[k] = lo[k].min(p[k] - r);
            hi[k] = hi[k].max(p[k] + r);
        }
    };
    for &e in &ents {
        match ops::ent_kind(doc, e) {
            Ok(EntKind::Point) => grow(ops::point_pos(doc, e).unwrap(), 0.0),
            Ok(EntKind::Circle) => {
                let (c, r) = ops::circle_vals(doc, e).unwrap();
                grow(ops::point_pos(doc, c).unwrap(), r);
            }
            _ => {}
        }
    }
    let size = [(hi[0] - lo[0]).max(1.0) * 1.1, (hi[1] - lo[1]).max(1.0) * 1.1];
    let cell = (size[0] / w as f64).max(size[1] / (2.0 * h as f64));
    let (cx, cy) = (cell, cell * 2.0);
    let mid = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
    let min = [mid[0] - cx * w as f64 / 2.0, mid[1] - cy * h as f64 / 2.0];
    let max = [min[0] + cx * w as f64, min[1] + cy * h as f64];
    let mut grid = vec![vec![' '; w]; h];
    let at = |p: [f64; 2]| -> Option<(usize, usize)> {
        let (c, r) = (((p[0] - min[0]) / cx).floor(), ((max[1] - p[1]) / cy).floor());
        (c >= 0.0 && r >= 0.0 && (c as usize) < w && (r as usize) < h).then_some((r as usize, c as usize))
    };
    for &e in ents.iter().filter(|&&e| ops::ent_kind(doc, e) != Ok(EntKind::Point)) {
        let ch = if ops::ent_construction(doc, e).unwrap_or(false) { ':' } else { '#' };
        for pl in ops::ent_samples(doc, e, min, max, cx.min(cy) / 4.0).unwrap_or_default() {
            for s in pl.windows(2) {
                let n = (((s[1][0] - s[0][0]) / cx).abs().max(((s[1][1] - s[0][1]) / cy).abs()).ceil() as usize * 2).max(1);
                for k in 0..=n {
                    let t = k as f64 / n as f64;
                    if let Some((r, c)) = at([s[0][0] + t * (s[1][0] - s[0][0]), s[0][1] + t * (s[1][1] - s[0][1])]) {
                        if grid[r][c] != '#' {
                            grid[r][c] = ch;
                        }
                    }
                }
            }
        }
    }
    for &e in ents.iter().filter(|&&e| ops::ent_kind(doc, e) == Ok(EntKind::Point)) {
        if let Some((r, c)) = at(ops::point_pos(doc, e).unwrap()) {
            grid[r][c] = '*';
            for (k, ch) in ops::ent_name(doc, e).unwrap_or_default().chars().enumerate() {
                if c + 1 + k < w {
                    grid[r][c + 1 + k] = ch;
                }
            }
        }
    }
    grid.into_iter().map(|row| row.into_iter().collect::<String>().trim_end().to_string()).collect()
}
