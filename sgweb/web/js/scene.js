// What the canvas shows, in screen coordinates: the entities as polylines, plus the label and marker boxes
// the renderer records. It answers what lies under a screen position (hit testing) and where a click would
// put a point (snapping).

import { at, readEntities } from "./sg.js";
import { nearestOn } from "./util.js";

/** Screen polylines (flat [x0, y0, x1, y1, ...] arrays) of entity `e` of `doc` in `view`. */
export function shapesOf(doc, e, view) {
  const w = view.window();
  // a margin beyond the window keeps line caps and curve ends out of sight
  const m = 20 * w.px;
  let polys;
  try {
    polys = doc.entSamples(e, w.x0 - m, w.y0 - m, w.x1 + m, w.y1 + m, w.px);
  } catch {
    return [];
  }
  const out = [];
  for (const pl of polys) {
    const f = new Float64Array(pl.length * 2);
    let k = 0;
    for (const p of pl) {
      const x = view.sx(p[0]), y = view.sy(p[1]);
      if (!Number.isFinite(x) || !Number.isFinite(y)) continue;
      f[k++] = x;
      f[k++] = y;
    }
    if (k) out.push(k === f.length ? f : f.slice(0, k));
  }
  return out;
}

export class Scene {
  constructor() {
    this.gen = -1;
    this.viewKey = "";
    this.ents = [];
    /** entity -> polylines */
    this.shapes = new Map();
    /** function curves -> [cache key, polylines]; sampling them is costly */
    this.curveCache = new Map();
    /** label boxes of dimensions: {x, y, w, h, d}, from the renderer */
    this.labels = [];
    /** constraint markers: {x, y, r, c}, from the renderer */
    this.markers = [];
    /** conic handles: {x, y, e, which}, from the renderer */
    this.handles = [];
  }

  /** Brings the shapes up to date with the document and view; true when anything changed. */
  update(doc, view) {
    const gen = doc.generation, vk = view.key();
    if (gen === this.gen && vk === this.viewKey) return false;
    let values = this.values;
    if (gen !== this.gen) {
      this.ents = readEntities(doc);
      // function curves change only with the values of the quantities they can name
      values = "";
      for (let i = 0; i < doc.varCount; i++) values += doc.varValue(i) + ",";
      for (let i = 0; i < doc.dimCount; i++) values += doc.dimValue(i) + ",";
      this.values = values;
    }
    this.gen = gen;
    this.viewKey = vk;
    this.shapes.clear();
    for (const r of this.ents) {
      if (!r || r.hidden) continue;
      if (r.kind === "curve") {
        const key = doc.curveSrc(r.e) + "|" + values + "|" + vk;
        const hit = this.curveCache.get(r.e);
        if (hit && hit[0] === key) {
          this.shapes.set(r.e, hit[1]);
          continue;
        }
        const p = shapesOf(doc, r.e, view);
        this.curveCache.set(r.e, [key, p]);
        this.shapes.set(r.e, p);
      } else {
        this.shapes.set(r.e, shapesOf(doc, r.e, view));
      }
    }
    return true;
  }

  ent(e) {
    return this.ents[e] ?? null;
  }

  isKind(e, ...kinds) {
    const r = this.ents[e];
    return !!r && kinds.includes(r.kind);
  }

  /** The nearest visible point within `r` pixels of (x, y). */
  pointAt(x, y, r, exclude = []) {
    let best = null, bd = r;
    for (const [e, polys] of this.shapes) {
      if (this.ents[e].kind !== "point" || exclude.includes(e) || !polys.length) continue;
      const d = Math.hypot(polys[0][0] - x, polys[0][1] - y);
      if (d < bd) [best, bd] = [e, d];
    }
    return best;
  }

  /** Visible curves (anything but points) passing within `r` pixels of (x, y), nearest first, with the
   *  nearest screen point of each: [{e, d, x, y}]. */
  curvesNear(x, y, r) {
    const out = [];
    for (const [e, polys] of this.shapes) {
      const ent = this.ents[e];
      if (ent.kind === "point") continue;
      const [d, qx, qy] = nearestOn(polys, x, y);
      // the axes give way to anything drawn on them
      const lim = ent.axis ? r * 0.6 : r;
      if (d < lim) out.push({ e, d: ent.axis ? d + r * 0.3 : d, x: qx, y: qy });
    }
    // among curves at the same distance, the later one, drawn on top, comes first
    out.sort((a, b) => (Math.abs(a.d - b.d) < 0.5 ? b.e - a.e : a.d - b.d));
    return out;
  }

  /** What lies under (x, y): {t: "ent" | "dim" | "con" | "handle", ...}, or null. */
  hit(x, y, r, { markers = true } = {}) {
    for (const hd of this.handles) {
      if (Math.hypot(hd.x - x, hd.y - y) < r) return { t: "handle", e: hd.e, which: hd.which };
    }
    const p = this.pointAt(x, y, r);
    if (p !== null) return { t: "ent", i: p };
    for (let k = this.labels.length - 1; k >= 0; k--) {
      const l = this.labels[k];
      if (x >= l.x - 3 && x <= l.x + l.w + 3 && y >= l.y - 3 && y <= l.y + l.h + 3) return { t: "dim", i: l.d };
    }
    if (markers) {
      for (let k = this.markers.length - 1; k >= 0; k--) {
        const m = this.markers[k];
        if (Math.hypot(m.x - x, m.y - y) <= m.r + 2) return { t: "con", i: m.c };
      }
    }
    const c = this.curvesNear(x, y, r * 0.85)[0];
    return c ? { t: "ent", i: c.e } : null;
  }

  /** The place a click at (x, y) gives: an existing point, a point on a curve, or a free position. */
  snap(doc, view, x, y, r, exclude = []) {
    const p = this.pointAt(x, y, r + 2, exclude);
    if (p !== null) return at.point(p);
    const c = this.curvesNear(x, y, r * 0.85)[0];
    if (c) {
      let w = view.toWorld(c.x, c.y);
      if (this.ents[c.e].kind === "line") {
        // project onto the line itself, not its drawn polyline, for an exact position
        const [a, b] = doc.linePts(c.e);
        const pa = doc.pointPos(a), pb = doc.pointPos(b), m = view.toWorld(x, y);
        const dx = pb[0] - pa[0], dy = pb[1] - pa[1];
        const t = ((m[0] - pa[0]) * dx + (m[1] - pa[1]) * dy) / Math.max(dx * dx + dy * dy, 1e-18);
        w = [pa[0] + t * dx, pa[1] + t * dy];
      }
      return at.on(c.e, w);
    }
    return at.free(view.toWorld(x, y));
  }

  /** Entities whose shapes lie inside the screen rectangle. */
  inside(x0, y0, x1, y1) {
    const out = [];
    for (const [e, polys] of this.shapes) {
      if (this.ents[e].builtin || !polys.length) continue;
      let all = true;
      for (const pl of polys) {
        for (let i = 0; i < pl.length && all; i += 2) all = pl[i] >= x0 && pl[i] <= x1 && pl[i + 1] >= y0 && pl[i + 1] <= y1;
      }
      if (all) out.push(e);
    }
    return out;
  }
}
