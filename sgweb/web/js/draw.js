// Draws the construction on the canvas: grid paper with rulers, the entities, dimensions in drafting style,
// constraint badges, and the previews and inputs of the current tool. It records where labels, badges and
// handles went in the scene, for hit testing.

import { fmt, fmtDim, splitSub } from "./util.js";

const TAU = Math.PI * 2;

/** Constraint badges: the symbol each kind shows. */
const BADGE = {
  coincident: "•",
  on_curve: "∈",
  midpoint: "½",
  midpoint_of: "½",
  horizontal: "H",
  vertical: "V",
  parallel: "∥",
  perpendicular: "⟂",
  collinear: "⋯",
  equal: "=",
  tangent: "T",
  tangent_at: "T",
  concentric: "◎",
  symmetric: "⇔",
  bisector: "∡",
  focus: "F",
  directrix: "D",
  asymptote: "∼",
};

/** Reads the canvas colors from the CSS custom properties of the page. */
export function palette() {
  const cs = getComputedStyle(document.documentElement);
  const v = (n) => cs.getPropertyValue(n).trim();
  return {
    paper: v("--paper"),
    gridMinor: v("--grid-minor"),
    gridMajor: v("--grid-major"),
    ruler: v("--ruler"),
    rulerText: v("--ruler-text"),
    axis: v("--axis"),
    ink: v("--ink"),
    free: v("--free"),
    sel: v("--sel"),
    err: v("--err"),
    dim: v("--dim"),
    ref: v("--ref"),
    ghost: v("--ghost"),
    badge: v("--badge"),
    badgeText: v("--badge-text"),
    label: v("--label"),
    curves: [1, 2, 3, 4, 5, 6].map((k) => v("--curve-" + k)),
    font: getComputedStyle(document.body).fontFamily,
  };
}

function withAlpha(col, a) {
  // colors are #rrggbb in the style sheet
  if (/^#[0-9a-f]{6}$/i.test(col)) {
    const n = parseInt(col.slice(1), 16);
    return `rgba(${n >> 16},${(n >> 8) & 255},${n & 255},${a})`;
  }
  return col;
}

export class Renderer {
  constructor(canvas) {
    this.canvas = canvas;
    this.g = canvas.getContext("2d");
    this.dpr = 1;
    this.pal = palette();
  }

  resize(w, h, dpr) {
    this.dpr = dpr;
    this.canvas.width = Math.round(w * dpr);
    this.canvas.height = Math.round(h * dpr);
  }

  /** Draws the whole canvas for app state `app`. */
  draw(app) {
    const g = this.g, v = app.view, pal = this.pal, scene = app.scene, doc = app.doc;
    g.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);
    this.grid(v, app.insets);
    const now = performance.now();
    const flash = now < app.flash.until ? new Set(app.flash.ents) : null;
    const selEnts = new Set(app.sel.filter((s) => s.t === "ent").map((s) => s.i));
    const picked = new Set(app.inputEnts());
    const hover = app.hover;
    // entities a hovered constraint or dimension refers to light up with it
    const related = new Set();
    if (hover?.t === "con") for (const e of doc.conEnts(hover.i)) related.add(e);
    if (hover?.t === "dim") for (const e of doc.dimEnts(hover.i)) related.add(e);
    const conflicts = this.conflicts(doc);

    const color = (r) => {
      if (flash?.has(r.e)) return pal.err;
      if (selEnts.has(r.e) || picked.has(r.e)) return pal.sel;
      if (conflicts.has(r.e)) return pal.err;
      if (r.axis) return pal.axis;
      if (r.kind === "curve") return pal.curves[r.fnIndex % pal.curves.length];
      return r.determined ? pal.ink : pal.free;
    };
    const isHover = (e) => (hover?.t === "ent" && hover.i === e) || related.has(e);

    // curves
    for (const [e, polys] of scene.shapes) {
      const r = scene.ent(e);
      if (r.kind === "point") continue;
      const col = color(r), hv = isHover(e), sel = selEnts.has(e);
      if ((hv || sel) && !r.axis) this.polys(polys, withAlpha(col, 0.18), 9, false);
      this.polys(polys, col, r.axis ? (hv ? 1.8 : 1) : hv ? 2.6 : 2, r.construction && !r.axis);
    }
    scene.handles = [];
    for (const [e] of scene.shapes) {
      const r = scene.ent(e);
      if (r.kind === "conic" && (selEnts.has(e) || isHover(e) || (hover?.t === "handle" && hover.e === e) || app.drag?.e === e)) this.conicHandles(app, e, color(r));
    }
    this.extensions(doc, v, scene, color);
    for (let c = 0; c < doc.conCount; c++) {
      const contact = doc.conKind(c) === "tangent" ? doc.conContact(c) : null;
      if (contact && contact[2]) {
        g.strokeStyle = pal.ref;
        g.lineWidth = 1;
        g.beginPath();
        g.arc(v.sx(contact[0]), v.sy(contact[1]), 2.8, 0, TAU);
        g.stroke();
      }
    }

    // dimensions
    scene.labels = [];
    const selDims = new Set(app.sel.filter((s) => s.t === "dim").map((s) => s.i));
    for (let i = 0; i < doc.dimCount; i++) {
      if (!doc.dimKind(i)) continue;
      const ents = doc.dimEnts(i);
      const err = doc.dimConflict(i) || !!doc.dimError(i);
      const driving = doc.dimDriving(i);
      let col = driving ? pal.dim : pal.ref;
      if (err) col = pal.err;
      if (selDims.has(i)) col = pal.sel;
      if (flash && ents.every((e) => flash.has(e))) col = pal.err;
      const box = this.dimension(doc, v, i, col, {
        hover: hover?.t === "dim" && hover.i === i,
        hideLabel: app.editing === i,
      });
      if (box) scene.labels.push({ ...box, d: i });
    }

    // constraint badges
    scene.markers = [];
    if (app.showConstraints) this.badges(app, doc, v, scene);

    // points
    const r0 = app.coarse ? 4.6 : 3.8;
    for (const [e, polys] of scene.shapes) {
      const r = scene.ent(e);
      if (r.kind !== "point" || !polys.length) continue;
      const x = polys[0][0], y = polys[0][1];
      const col = color(r), hv = isHover(e), sel = selEnts.has(e);
      if (e === doc.origin) {
        g.strokeStyle = sel ? pal.sel : hv ? pal.ink : pal.axis;
        g.lineWidth = 1.3;
        g.beginPath();
        g.arc(x, y, 4.5, 0, TAU);
        g.moveTo(x - 7, y);
        g.lineTo(x + 7, y);
        g.moveTo(x, y - 7);
        g.lineTo(x, y + 7);
        g.stroke();
        continue;
      }
      const rad = hv || sel ? r0 + 1 : r0;
      if (hv || sel) {
        g.fillStyle = withAlpha(col, 0.2);
        g.beginPath();
        g.arc(x, y, rad + 6, 0, TAU);
        g.fill();
      }
      g.fillStyle = col;
      g.strokeStyle = pal.paper;
      g.lineWidth = 2;
      g.beginPath();
      if (r.locked) g.rect(x - rad, y - rad, rad * 2, rad * 2);
      else g.arc(x, y, rad, 0, TAU);
      g.stroke();
      g.fill();
      if (!r.determined && !r.locked && !conflicts.has(e) && !sel && !flash?.has(e)) {
        // a point that can still move has a light center
        g.fillStyle = pal.paper;
        g.beginPath();
        g.arc(x, y, rad * 0.42, 0, TAU);
        g.fill();
      }
      if (r.name && (app.showLabels || hv || sel)) {
        g.font = `600 12px ${pal.font}`;
        g.fillStyle = sel ? pal.sel : pal.label;
        g.textAlign = "left";
        g.textBaseline = "bottom";
        this.halo(r.name, x + 7, y - 5);
        g.fillText(r.name, x + 7, y - 5);
      }
    }

    this.toolOverlay(app);

    if (app.marquee) {
      const [x0, y0, x1, y1] = app.marquee;
      g.fillStyle = withAlpha(pal.free, 0.08);
      g.strokeStyle = withAlpha(pal.free, 0.7);
      g.setLineDash([4, 3]);
      g.lineWidth = 1;
      g.fillRect(Math.min(x0, x1), Math.min(y0, y1), Math.abs(x1 - x0), Math.abs(y1 - y0));
      g.strokeRect(Math.min(x0, x1) + 0.5, Math.min(y0, y1) + 0.5, Math.abs(x1 - x0), Math.abs(y1 - y0));
      g.setLineDash([]);
    }
    return flash !== null;
  }

  /** Text outline in the paper color, so labels stay readable over lines. */
  halo(text, x, y) {
    const g = this.g;
    g.strokeStyle = this.pal.paper;
    g.lineWidth = 3;
    g.lineJoin = "round";
    g.strokeText(text, x, y);
  }

  conflicts(doc) {
    const s = new Set();
    if (doc.conflictCount === 0) return s;
    for (let c = 0; c < doc.conCount; c++) if (doc.conKind(c) && doc.conConflict(c)) for (const e of doc.conEnts(c)) s.add(e);
    for (let i = 0; i < doc.dimCount; i++) if (doc.dimKind(i) && doc.dimConflict(i)) for (const e of doc.dimEnts(i)) s.add(e);
    for (let e = 0; e < doc.entCount; e++) if (doc.entKind(e) && doc.entConflict(e)) s.add(e);
    return s;
  }

  grid(v, ins) {
    const g = this.g, pal = this.pal;
    g.fillStyle = pal.paper;
    g.fillRect(0, 0, v.w, v.h);
    const raw = 22 / v.scale;
    const base = Math.pow(10, Math.floor(Math.log10(raw)));
    const step = [1, 2, 5, 10].map((m) => m * base).find((s) => s >= raw);
    const major = step * 5;
    const w = v.window();
    const line = (major) => {
      g.strokeStyle = major ? pal.gridMajor : pal.gridMinor;
      g.lineWidth = 1;
      g.beginPath();
    };
    for (const isMajor of [false, true]) {
      line(isMajor);
      const s = isMajor ? major : step;
      for (let x = Math.floor(w.x0 / s) * s; x <= w.x1; x += s) {
        if (!isMajor && Math.abs(Math.round(x / major) * major - x) < step * 0.01) continue;
        const sx = Math.round(v.sx(x)) + 0.5;
        g.moveTo(sx, 0);
        g.lineTo(sx, v.h);
      }
      for (let y = Math.floor(w.y0 / s) * s; y <= w.y1; y += s) {
        if (!isMajor && Math.abs(Math.round(y / major) * major - y) < step * 0.01) continue;
        const sy = Math.round(v.sy(y)) + 0.5;
        g.moveTo(0, sy);
        g.lineTo(v.w, sy);
      }
      g.stroke();
    }
    // rulers along the left and bottom edges of the free part of the canvas
    const left = ins.left, bottom = v.h - ins.bottom, rw = 20;
    g.fillStyle = pal.ruler;
    g.fillRect(left, ins.top, rw, bottom - ins.top);
    g.fillRect(left, bottom - rw, v.w - left - ins.right, rw);
    g.font = `10px ${pal.font}`;
    g.fillStyle = pal.rulerText;
    g.textAlign = "center";
    g.textBaseline = "middle";
    for (let x = Math.floor(w.x0 / major) * major; x <= w.x1; x += major) {
      const sx = v.sx(x);
      if (sx < left + rw + 14 || sx > v.w - ins.right - 14) continue;
      g.fillText(fmt(Math.abs(x) < major * 1e-6 ? 0 : x), sx, bottom - rw / 2);
    }
    g.save();
    g.translate(left + rw / 2, 0);
    g.rotate(-Math.PI / 2);
    for (let y = Math.floor(w.y0 / major) * major; y <= w.y1; y += major) {
      const sy = v.sy(y);
      if (sy < ins.top + 14 || sy > bottom - rw - 14) continue;
      g.fillText(fmt(Math.abs(y) < major * 1e-6 ? 0 : y), -sy, 0);
    }
    g.restore();
  }

  polys(polys, col, width, dashed) {
    const g = this.g;
    g.strokeStyle = col;
    g.lineWidth = width;
    g.lineJoin = "round";
    g.lineCap = "round";
    g.setLineDash(dashed ? [7, 5] : []);
    for (const pl of polys) {
      if (pl.length < 4) continue;
      g.beginPath();
      g.moveTo(pl[0], pl[1]);
      for (let i = 2; i < pl.length; i += 2) g.lineTo(pl[i], pl[i + 1]);
      g.stroke();
    }
    g.setLineDash([]);
  }

  /** The axes and drag handles of conic `e`, which also go into the scene for hit testing. */
  conicHandles(app, e, col) {
    const g = this.g, v = app.view, doc = app.doc;
    const [kind, c, a, b, t] = doc.conicVals(e);
    const p = doc.pointPos(c);
    const ct = Math.cos(t), st = Math.sin(t);
    const ends = [[p[0] + a * ct, p[1] + a * st]];
    if (kind !== "parabola") ends.push([p[0] - b * st, p[1] + b * ct]);
    g.strokeStyle = withAlpha(col, 0.55);
    g.lineWidth = 1;
    g.setLineDash([3, 3]);
    g.beginPath();
    for (const q of ends) {
      g.moveTo(v.sx(p[0]), v.sy(p[1]));
      g.lineTo(v.sx(q[0]), v.sy(q[1]));
    }
    g.stroke();
    g.setLineDash([]);
    ends.forEach((q, which) => {
      const x = v.sx(q[0]), y = v.sy(q[1]);
      const hv = app.hover?.t === "handle" && app.hover.e === e && app.hover.which === which;
      const r = hv ? 6 : 4.5;
      g.fillStyle = this.pal.paper;
      g.strokeStyle = this.pal.sel;
      g.lineWidth = 1.6;
      g.beginPath();
      g.rect(x - r, y - r, 2 * r, 2 * r);
      g.fill();
      g.stroke();
      app.scene.handles.push({ x, y, e, which });
    });
  }

  /** Dashed continuations of segments up to points held on their line and to tangent contacts. */
  extensions(doc, v, scene, color) {
    const g = this.g;
    const ext = (l, q) => {
      const r = scene.ent(l);
      if (!r || r.kind !== "line" || r.infinite || r.hidden) return;
      const [a, b] = doc.linePts(l);
      const pa = doc.pointPos(a), pb = doc.pointPos(b);
      const dx = pb[0] - pa[0], dy = pb[1] - pa[1];
      const l2 = dx * dx + dy * dy;
      if (l2 < 1e-18) return;
      const t = ((q[0] - pa[0]) * dx + (q[1] - pa[1]) * dy) / l2;
      if (t >= 0 && t <= 1) return;
      const from = t < 0 ? pa : pb;
      g.strokeStyle = withAlpha(color(r), 0.45);
      g.lineWidth = 1;
      g.setLineDash([4, 4]);
      g.beginPath();
      g.moveTo(v.sx(from[0]), v.sy(from[1]));
      g.lineTo(v.sx(pa[0] + dx * t), v.sy(pa[1] + dy * t));
      g.stroke();
      g.setLineDash([]);
    };
    for (let c = 0; c < doc.conCount; c++) {
      const k = doc.conKind(c);
      if (k === "on_curve") {
        const [p, l] = doc.conEnts(c);
        if (scene.ent(p)?.kind === "point") ext(l, doc.pointPos(p));
      } else if (k === "tangent") {
        const t = doc.conContact(c);
        if (t) for (const e of doc.conEnts(c)) ext(e, t);
      }
    }
  }

  // ---------------------------------------------------------------- dimensions

  arrow(x, y, dx, dy, col) {
    const l = Math.hypot(dx, dy);
    if (!(l > 0)) return;
    const ux = dx / l, uy = dy / l;
    const g = this.g;
    g.fillStyle = col;
    g.beginPath();
    g.moveTo(x, y);
    g.lineTo(x - ux * 8 - uy * 3, y - uy * 8 + ux * 3);
    g.lineTo(x - ux * 8 + uy * 3, y - uy * 8 - ux * 3);
    g.closePath();
    g.fill();
  }

  seg(x0, y0, x1, y1) {
    const g = this.g;
    g.beginPath();
    g.moveTo(x0, y0);
    g.lineTo(x1, y1);
    g.stroke();
  }

  /**
   * Draws dimension `i` of `doc` (any document: previews draw from a copy) in color `col`; returns the box
   * of its label in screen coordinates, or null when the label is not drawn.
   */
  dimension(doc, v, i, col, { hover = false, hideLabel = false, ghost = false } = {}) {
    const g = this.g;
    const kind = doc.dimKind(i);
    const ents = doc.dimEnts(i);
    const anchor = doc.dimAnchor(i), off = doc.dimOffset(i);
    if (!anchor || !off) return null;
    const lw = [anchor[0] + off[0], anchor[1] + off[1]];
    const L = v.toScreen(lw);
    const S = (p) => v.toScreen(p);
    const pos = (e) => doc.pointPos(e);
    const lineEnds = (l) => doc.linePts(l).slice(0, 2).map((p) => doc.pointPos(p));
    g.strokeStyle = col;
    g.lineWidth = hover ? 1.6 : 1.1;
    const thinCol = withAlpha(col, 0.55);

    const linear = (a, b, dir) => {
      const l = Math.hypot(dir[0], dir[1]) || 1;
      const u = [dir[0] / l, dir[1] / l], n = [-u[1], u[0]];
      const ha = (lw[0] - a[0]) * n[0] + (lw[1] - a[1]) * n[1];
      const hb = (lw[0] - b[0]) * n[0] + (lw[1] - b[1]) * n[1];
      const pa = [a[0] + n[0] * ha, a[1] + n[1] * ha], pb = [b[0] + n[0] * hb, b[1] + n[1] * hb];
      const [sa, sb] = [S(pa), S(pb)];
      g.strokeStyle = thinCol;
      for (const [from, to] of [
        [S(a), sa],
        [S(b), sb],
      ]) {
        const dx = to[0] - from[0], dy = to[1] - from[1], d = Math.hypot(dx, dy);
        if (d > 5) this.seg(from[0] + (dx / d) * 4, from[1] + (dy / d) * 4, to[0] + (dx / d) * 5, to[1] + (dy / d) * 5);
      }
      g.strokeStyle = col;
      this.seg(sa[0], sa[1], sb[0], sb[1]);
      this.arrow(sa[0], sa[1], sa[0] - sb[0], sa[1] - sb[1], col);
      this.arrow(sb[0], sb[1], sb[0] - sa[0], sb[1] - sa[1], col);
      const t = (lw[0] - pa[0]) * u[0] + (lw[1] - pa[1]) * u[1];
      const len = (pb[0] - pa[0]) * u[0] + (pb[1] - pa[1]) * u[1];
      if (t < 0 || t > len) {
        const q = t < 0 ? sa : sb;
        g.strokeStyle = thinCol;
        this.seg(q[0], q[1], L[0], L[1]);
      }
    };
    const origin = doc.pointPos(doc.origin);
    switch (kind) {
      case "distance":
        linear(pos(ents[0]), pos(ents[1]), [pos(ents[1])[0] - pos(ents[0])[0], pos(ents[1])[1] - pos(ents[0])[1]]);
        break;
      case "length": {
        const [a, b] = lineEnds(ents[0]);
        linear(a, b, [b[0] - a[0], b[1] - a[1]]);
        break;
      }
      case "hdist":
        linear(pos(ents[0]), pos(ents[1]), [1, 0]);
        break;
      case "vdist":
        linear(pos(ents[0]), pos(ents[1]), [0, 1]);
        break;
      case "x":
        linear(origin, pos(ents[0]), [1, 0]);
        break;
      case "y":
        linear(origin, pos(ents[0]), [0, 1]);
        break;
      case "point_line": {
        const p = pos(ents[0]);
        const [a, b] = lineEnds(ents[1]);
        const dx = b[0] - a[0], dy = b[1] - a[1];
        const t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / Math.max(dx * dx + dy * dy, 1e-18);
        const f = [a[0] + t * dx, a[1] + t * dy];
        const [sp, sf] = [S(p), S(f)];
        this.seg(sp[0], sp[1], sf[0], sf[1]);
        this.arrow(sp[0], sp[1], sp[0] - sf[0], sp[1] - sf[1], col);
        this.arrow(sf[0], sf[1], sf[0] - sp[0], sf[1] - sp[1], col);
        g.strokeStyle = thinCol;
        if (t < 0 || t > 1) {
          const near = S(t < 0 ? a : b);
          g.setLineDash([3, 3]);
          this.seg(near[0], near[1], sf[0], sf[1]);
          g.setLineDash([]);
        }
        this.seg((sp[0] + sf[0]) / 2, (sp[1] + sf[1]) / 2, L[0], L[1]);
        break;
      }
      case "radius":
      case "diameter": {
        const [c, r] = doc.circleVals(ents[0]);
        const cc = pos(c);
        const d = Math.hypot(lw[0] - cc[0], lw[1] - cc[1]) || 1e-12;
        const u = [(lw[0] - cc[0]) / d, (lw[1] - cc[1]) / d];
        const edge = S([cc[0] + u[0] * r, cc[1] + u[1] * r]);
        const start = kind === "diameter" ? S([cc[0] - u[0] * r, cc[1] - u[1] * r]) : S(cc);
        this.seg(start[0], start[1], edge[0], edge[1]);
        this.arrow(edge[0], edge[1], edge[0] - start[0], edge[1] - start[1], col);
        if (kind === "diameter") this.arrow(start[0], start[1], start[0] - edge[0], start[1] - edge[1], col);
        if (d > r) {
          g.strokeStyle = thinCol;
          this.seg(edge[0], edge[1], L[0], L[1]);
        }
        break;
      }
      case "angle": {
        const x = anchor;
        const dir = (l) => {
          const [a, b] = lineEnds(l);
          return Math.atan2(b[1] - a[1], b[0] - a[0]);
        };
        const wrap = (s) => {
          s = ((s % TAU) + TAU) % TAU;
          return s > Math.PI ? s - TAU : s;
        };
        let a0 = dir(ents[0]);
        const a1 = dir(ents[1]);
        if (doc.dimSupp(i)) a0 += Math.PI;
        const sweep = wrap(a1 - a0);
        const rad = Math.max(Math.hypot(lw[0] - x[0], lw[1] - x[1]), 4 / v.scale);
        // of the angle and its vertical angle, the one the label sits in
        const la = Math.atan2(lw[1] - x[1], lw[0] - x[0]);
        const rel = (((la - a0) % TAU) + TAU) % TAU;
        const inside = sweep >= 0 ? rel <= sweep : TAU - rel <= -sweep;
        const s0 = inside ? a0 : a0 + Math.PI;
        const pts = [];
        for (let k = 0; k <= 48; k++) {
          const t = s0 + (sweep * k) / 48;
          pts.push(S([x[0] + rad * Math.cos(t), x[1] + rad * Math.sin(t)]));
        }
        g.beginPath();
        pts.forEach((p, k) => (k ? g.lineTo(p[0], p[1]) : g.moveTo(p[0], p[1])));
        g.stroke();
        const n = pts.length;
        this.arrow(pts[0][0], pts[0][1], pts[0][0] - pts[2][0], pts[0][1] - pts[2][1], col);
        this.arrow(pts[n - 1][0], pts[n - 1][1], pts[n - 1][0] - pts[n - 3][0], pts[n - 1][1] - pts[n - 3][1], col);
        // rays out to the arc where it lies beyond the lines
        g.strokeStyle = thinCol;
        for (const [l, ang] of [
          [ents[0], s0],
          [ents[1], s0 + sweep],
        ]) {
          const reach = Math.max(...lineEnds(l).map((p) => Math.hypot(p[0] - x[0], p[1] - x[1])));
          if (rad > reach) {
            const e0 = S([x[0] + reach * Math.cos(ang), x[1] + reach * Math.sin(ang)]);
            const e1 = S([x[0] + (rad + 6 / v.scale) * Math.cos(ang), x[1] + (rad + 6 / v.scale) * Math.sin(ang)]);
            this.seg(e0[0], e0[1], e1[0], e1[1]);
          }
        }
        break;
      }
      case "conic_a":
      case "conic_b": {
        const [, c, a, b, t] = doc.conicVals(ents[0]);
        const cc = pos(c);
        const tip = kind === "conic_a" ? [cc[0] + a * Math.cos(t), cc[1] + a * Math.sin(t)] : [cc[0] - b * Math.sin(t), cc[1] + b * Math.cos(t)];
        const [sc, stp] = [S(cc), S(tip)];
        this.seg(sc[0], sc[1], stp[0], stp[1]);
        this.arrow(stp[0], stp[1], stp[0] - sc[0], stp[1] - sc[1], col);
        g.strokeStyle = thinCol;
        this.seg((sc[0] + stp[0]) / 2, (sc[1] + stp[1]) / 2, L[0], L[1]);
        break;
      }
    }
    if (hideLabel) return null;
    return this.dimLabel(doc, i, L, col, hover, ghost);
  }

  /** The label of dimension `i` centered at screen position L: name, then value or expression. */
  dimLabel(doc, i, L, col, hover, ghost) {
    const g = this.g, pal = this.pal;
    const angle = doc.dimKind(i) === "angle";
    const driving = doc.dimDriving(i);
    let value = fmtDim(doc.dimValue(i), angle);
    if (doc.dimKind(i) === "diameter") value = "⌀" + value;
    const expr = driving ? doc.dimExpr(i) : "";
    const text = expr ? `${expr} = ${value}` : value;
    const [base, sub] = splitSub(doc.dimName(i) ?? "");
    const fBase = `500 12px ${pal.font}`, fSub = `500 9px ${pal.font}`, fVal = `600 13px ${pal.font}`;
    g.font = fBase;
    const wb = g.measureText(base).width;
    g.font = fSub;
    const ws = sub ? g.measureText(sub).width + 1 : 0;
    g.font = fVal;
    const wv = g.measureText(text).width;
    const w = 8 + wb + ws + 6 + wv + 8, h = 21;
    const x = L[0] - w / 2, y = L[1] - h / 2;
    g.beginPath();
    g.roundRect(x, y, w, h, 10.5);
    g.fillStyle = ghost ? withAlpha(pal.paper, 0.7) : pal.paper;
    g.fill();
    g.fillStyle = withAlpha(col, driving ? 0.14 : 0.06);
    g.fill();
    g.lineWidth = hover ? 1.8 : 1;
    g.strokeStyle = col;
    g.setLineDash(driving ? [] : [3, 2]);
    g.stroke();
    g.setLineDash([]);
    g.textBaseline = "middle";
    g.textAlign = "left";
    g.fillStyle = withAlpha(col, 0.75);
    g.font = fBase;
    g.fillText(base, x + 8, L[1]);
    if (sub) {
      g.font = fSub;
      g.fillText(sub, x + 8 + wb + 1, L[1] + 3);
    }
    g.fillStyle = col;
    g.font = fVal;
    g.fillText(text, x + 8 + wb + ws + 6, L[1] + 0.5);
    return { x, y, w, h };
  }

  // ---------------------------------------------------------------- constraints

  /** Screen anchors (with offsets) for the badges of constraint `c`. */
  badgeSpots(doc, v, scene, c) {
    const kind = doc.conKind(c), e = doc.conEnts(c);
    const pt = (p) => v.toScreen(doc.pointPos(p));
    const lineSpot = (l) => {
      const [a, b] = doc.linePts(l).slice(0, 2).map((p) => v.toScreen(doc.pointPos(p)));
      let nx = -(b[1] - a[1]), ny = b[0] - a[0];
      const n = Math.hypot(nx, ny) || 1;
      nx /= n;
      ny /= n;
      if (ny > 0) [nx, ny] = [-nx, -ny];
      return [(a[0] + b[0]) / 2 + nx * 13, (a[1] + b[1]) / 2 + ny * 13];
    };
    const curveSpot = (x) => {
      const k = doc.entKind(x);
      if (k === "line") return lineSpot(x);
      if (k === "point") return pt(x);
      const polys = scene.shapes.get(x);
      if (polys?.length) {
        const pl = polys.reduce((a, b) => (b.length > a.length ? b : a));
        const m = 2 * Math.floor(pl.length / 6);
        return [pl[m] + 9, pl[m + 1] - 9];
      }
      return null;
    };
    const mid = (a, b) => {
      const [p, q] = [pt(a), pt(b)];
      return [(p[0] + q[0]) / 2, (p[1] + q[1]) / 2];
    };
    const off = (p, dx, dy) => (p ? [p[0] + dx, p[1] + dy] : null);
    switch (kind) {
      case "coincident":
        return [off(pt(e[0]), 11, 11)];
      case "on_curve":
        return [off(pt(e[0]), -11, -11)];
      case "midpoint":
      case "midpoint_of":
        return [off(pt(e[0]), -11, 11)];
      case "horizontal":
        return [e.length === 1 || doc.entKind(e[0]) === "line" ? lineSpot(e[0]) : off(mid(e[0], e[1]), 0, -12)];
      case "vertical":
        return [e.length === 1 || doc.entKind(e[0]) === "line" ? lineSpot(e[0]) : off(mid(e[0], e[1]), 12, 0)];
      case "parallel":
      case "perpendicular":
      case "collinear":
      case "equal":
        return [curveSpot(e[0]), curveSpot(e[1])];
      case "symmetric":
        return [off(mid(e[0], e[1]), 0, 12)];
      case "concentric": {
        const k = doc.entKind(e[0]);
        const c0 = k === "point" ? doc.pointPos(e[0]) : k === "conic" ? doc.pointPos(doc.conicVals(e[0])[1]) : doc.pointPos(doc.circleVals(e[0])[0]);
        return [off(v.toScreen(c0), 12, 12)];
      }
      case "tangent": {
        const t = doc.conContact(c);
        return t ? [off(v.toScreen(t), 11, -11)] : [];
      }
      case "tangent_at":
        return [off(pt(e[1]), 11, -11)];
      case "focus":
        return [off(pt(e[0]), 11, -11)];
      case "directrix":
        return e.length === 4 ? [pt(e[0])] : [curveSpot(e[0])];
      default:
        return [pt(e[0])];
    }
  }

  badges(app, doc, v, scene) {
    const g = this.g, pal = this.pal;
    const used = new Map();
    const selCons = new Set(app.sel.filter((s) => s.t === "con").map((s) => s.i));
    const hovEnt = app.hover?.t === "ent" ? app.hover.i : -1;
    for (let c = 0; c < doc.conCount; c++) {
      const kind = doc.conKind(c);
      if (!kind) continue;
      const ents = doc.conEnts(c);
      if (ents.some((e) => scene.ent(e)?.hidden && scene.ent(e)?.kind !== "point")) continue;
      let sym = BADGE[kind] ?? "?";
      if (kind === "offset") sym = doc.conTurned(c) ? "⟂" : "∥";
      const sel = selCons.has(c);
      const hov = (app.hover?.t === "con" && app.hover.i === c) || ents.includes(hovEnt);
      const err = doc.conConflict(c);
      for (const spot of this.badgeSpots(doc, v, scene, c)) {
        if (!spot || !Number.isFinite(spot[0]) || !Number.isFinite(spot[1])) continue;
        let [x, y] = spot;
        const key = `${Math.round(x / 16)},${Math.round(y / 16)}`;
        const n = used.get(key) ?? 0;
        used.set(key, n + 1);
        x += n * 17;
        if (x < -10 || y < -10 || x > v.w + 10 || y > v.h + 10) continue;
        const r = 8;
        g.beginPath();
        g.arc(x, y, r, 0, TAU);
        g.fillStyle = sel ? pal.sel : err ? pal.err : hov ? pal.badgeText : pal.badge;
        g.fill();
        g.fillStyle = sel || err || hov ? pal.paper : pal.badgeText;
        g.font = `700 ${sym.length > 1 ? 9 : 11}px ${pal.font}`;
        g.textAlign = "center";
        g.textBaseline = "middle";
        g.fillText(sym, x, y + 0.5);
        scene.markers.push({ x, y, r, c });
      }
    }
  }

  // ---------------------------------------------------------------- tools

  /** The inputs, snapping and preview of the active tool. */
  toolOverlay(app) {
    const g = this.g, pal = this.pal, v = app.view, doc = app.doc;
    if (!app.pointer || app.tool === "select" || app.tool === "function") return;
    const ctx = app.toolContext();
    if (!ctx) return;
    // what the next click would make, drawn from a copy of the document that has it
    const shown = app.previewTool(ctx, (copy, before) => {
      for (let e = before; e < copy.entCount; e++) {
        const k = copy.entKind(e);
        if (!k || copy.entHidden(e)) continue;
        const polys = this.copyShapes(copy, e, v);
        if (k === "point") {
          for (const pl of polys) {
            g.fillStyle = pal.ghost;
            g.beginPath();
            g.arc(pl[0], pl[1], 3.5, 0, TAU);
            g.fill();
          }
        } else this.polys(polys, pal.ghost, 2, true);
      }
      const nd = copy.dimCount;
      if (nd > doc.dimCount) this.dimension(copy, v, nd - 1, pal.ghost, { ghost: true });
    });
    // rubber band from the last place to the pointer
    const ins = app.inputs;
    const last = [...ins].reverse().find((i) => i.place || i.pos);
    const snapPos = ctx.snap.kind === "point" ? doc.pointPos(ctx.snap.e) : ctx.snap.p;
    if (!shown && last && app.tool !== "measure") {
      const p = last.place ? (last.place.kind === "point" ? doc.pointPos(last.place.e) : last.place.p) : last.pos;
      g.strokeStyle = pal.ghost;
      g.lineWidth = 1.2;
      g.setLineDash([5, 4]);
      this.seg(v.sx(p[0]), v.sy(p[1]), v.sx(snapPos[0]), v.sy(snapPos[1]));
      g.setLineDash([]);
    }
    for (const i of ins) {
      const p = i.place ? (i.place.kind === "point" ? doc.pointPos(i.place.e) : i.place.p) : i.pos;
      if (!p) continue;
      g.fillStyle = pal.sel;
      g.beginPath();
      g.arc(v.sx(p[0]), v.sy(p[1]), 4, 0, TAU);
      g.fill();
    }
    // where the pointer snaps
    const [sx, sy] = v.toScreen(snapPos);
    g.strokeStyle = pal.sel;
    g.lineWidth = 1.6;
    if (ctx.snap.kind === "point") {
      g.beginPath();
      g.arc(sx, sy, 9, 0, TAU);
      g.stroke();
    } else if (ctx.snap.kind === "on") {
      g.beginPath();
      g.moveTo(sx, sy - 6);
      g.lineTo(sx + 6, sy);
      g.lineTo(sx, sy + 6);
      g.lineTo(sx - 6, sy);
      g.closePath();
      g.stroke();
    }
    const axis = ins.length && app.tool === "line" ? app.lineAxisHint(ctx) : null;
    if (axis) {
      g.font = `700 11px ${pal.font}`;
      g.fillStyle = pal.sel;
      g.textAlign = "left";
      g.textBaseline = "bottom";
      g.fillText(axis === "horizontal" ? "H" : "V", sx + 10, sy - 8);
    }
  }

  copyShapes(copy, e, v) {
    const w = v.window();
    try {
      return copy.entSamples(e, w.x0, w.y0, w.x1, w.y1, w.px).map((pl) => {
        const f = [];
        for (const p of pl) f.push(v.sx(p[0]), v.sy(p[1]));
        return f;
      });
    } catch {
      return [];
    }
  }
}
