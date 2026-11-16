// The drawing and construction tools. A tool collects inputs click by click: places (where a point goes),
// picked entities and positions. `accept` turns a click into the next list of inputs (null when the click
// does not fit the step), `done` says when the list is complete, and `build` makes the shape in a document.
// One engine runs every tool: it shows the hint of the step, a preview of what the next click would make
// (built on a copy of the document) and commits complete inputs as one undoable change.

import { Rejected, at, change, must, onCopy, placePos, realize, samePoint } from "./sg.js";

// ------------------------------------------------------------------ input helpers

const place = (ctx) => ({ place: ctx.snap });
const pos = (ctx) => ({ pos: ctx.snap.kind === "point" ? placePos(ctx.doc, ctx.snap) : ctx.snap.p });

/** The entity under the pointer that satisfies `pred(kind, e)`: the hit entity, or else the nearest curve. */
function pick(ctx, pred) {
  const k = (e) => ctx.scene.ent(e)?.kind;
  if (ctx.hit?.t === "ent" && pred(k(ctx.hit.i), ctx.hit.i)) return ctx.hit.i;
  const c = ctx.near.find((n) => pred(k(n.e), n.e));
  return c ? c.e : null;
}

const isLine = (k) => k === "line";
const isSegment = (ctx) => (k, e) => k === "line" && !ctx.scene.ent(e).infinite;
const isRound = (k) => k === "circle" || k === "arc";
const isConic = (k) => k === "conic";
const touchable = (k) => k === "circle" || k === "arc" || k === "conic" || k === "curve";

function conicKind(doc, e) {
  return doc.conicVals(e)?.[0];
}

function distinctPlaces(ins) {
  const ps = ins.filter((i) => i.place).map((i) => i.place);
  return ps.every((a, i) => ps.every((b, j) => i === j || !samePoint(a, b)));
}

/** Accepts a place as the next input, refusing an existing point that is already an input. */
function nextPlace(ctx, ins) {
  const n = [...ins, place(ctx)];
  return distinctPlaces(n) ? n : null;
}

function steps(...hints) {
  return (ins) => hints[Math.min(ins.length, hints.length - 1)];
}

// ------------------------------------------------------------------ the tools

const pointTool = {
  id: "point",
  label: "Point",
  key: "p",
  icon: "point",
  hint: steps("Tap anywhere to place a point — on a curve, it stays on it"),
  accept: (ctx, ins) => (ctx.snap.kind === "point" ? null : [...ins, place(ctx)]),
  done: (ins) => ins.length === 1,
  build: (d, ins) => realize(d, ins[0].place),
};

/** The axis a line from `from` to `p` snaps to (within 3°), with the snapped position. */
function axisSnap(ctx, from, p) {
  const dx = p[0] - from[0], dy = p[1] - from[1];
  if (Math.hypot(dx, dy) * ctx.view.scale < 16) return null;
  const ang = ((Math.atan2(dy, dx) * 180) / Math.PI + 180) % 180;
  if (ang < 3 || ang > 177) return { axis: "horizontal", p: [p[0], from[1]] };
  if (Math.abs(ang - 90) < 3) return { axis: "vertical", p: [from[0], p[1]] };
  return null;
}

const lineTool = {
  id: "line",
  label: "Line",
  key: "l",
  icon: "line",
  hint: steps("Tap the start of the segment", "Tap its end — keep tapping to draw a chain, Esc to stop"),
  accept(ctx, ins) {
    if (ins.length === 1 && ctx.snap.kind === "free") {
      const s = axisSnap(ctx, placePos(ctx.doc, ins[0].place), ctx.snap.p);
      if (s) return [...ins, { place: at.free(s.p), axis: s.axis }];
    }
    return nextPlace(ctx, ins);
  },
  done: (ins) => ins.length === 2,
  build(d, ins) {
    const a = realize(d, ins[0].place), b = realize(d, ins[1].place);
    return { line: d.addLine(a, b, false, ins[1].axis), end: b };
  },
  // a chain goes on from the end of the last segment unless it ended on an existing point
  after: (ins, r) => (ins[1].place.kind === "point" ? [] : [{ place: at.point(r.end) }]),
};

const circleTool = {
  id: "circle",
  label: "Circle",
  key: "c",
  icon: "circle",
  hint: steps("Tap the center", "Tap a point the circle passes through"),
  accept: nextPlace,
  done: (ins) => ins.length === 2,
  build(d, ins) {
    const t = ins[1].place;
    const c = realize(d, ins[0].place);
    if (t.kind === "free") {
      const pc = d.pointPos(c);
      return d.addCircle(c, Math.max(Math.hypot(t.p[0] - pc[0], t.p[1] - pc[1]), 1e-3));
    }
    return d.addCircleTo(c, realize(d, t));
  },
};

const circle3Tool = {
  id: "circle3",
  label: "Circle through 3 points",
  icon: "circle3",
  hint: steps("Tap the first point on the circle", "Tap the second point", "Tap the third point"),
  accept: nextPlace,
  done: (ins) => ins.length === 3,
  build: (d, ins) => d.addCircleThrough(...ins.map((i) => realize(d, i.place)))[0],
};

const arcTool = {
  id: "arc",
  label: "Arc",
  key: "a",
  icon: "arc",
  hint: steps("Tap the center of the arc", "Tap where the arc starts", "Tap where it ends, counterclockwise"),
  accept: nextPlace,
  done: (ins) => ins.length === 3,
  build(d, ins) {
    const c = realize(d, ins[0].place), a = realize(d, ins[1].place);
    const end = ins[2].place;
    if (end.kind === "point") return d.addArc(c, a, end.e);
    const p = end.p;
    return d.addArcToward(c, a, p[0], p[1]);
  },
};

const arc3Tool = {
  id: "arc3",
  label: "Arc through 3 points",
  icon: "arc3",
  hint: steps("Tap where the arc starts", "Tap a point it passes through", "Tap where it ends"),
  accept: nextPlace,
  done: (ins) => ins.length === 3,
  build: (d, ins) => d.addArcThrough(...ins.map((i) => realize(d, i.place)))[0],
};

function conicTool(kind, key, label) {
  return {
    id: kind,
    label,
    key,
    icon: kind,
    hint: steps("Tap the center", "Tap the end of the first axis", "Tap how wide the " + kind + " is"),
    accept: (ctx, ins) => (ins.length === 0 ? [place(ctx)] : [...ins, pos(ctx)]),
    done: (ins) => ins.length === 3,
    build: (d, ins) => d.addConicFrom(kind, realize(d, ins[0].place), ...ins[1].pos, ...ins[2].pos),
  };
}

const parabolaTool = {
  id: "parabola",
  label: "Parabola",
  key: "b",
  icon: "parabola",
  hint: steps("Tap the vertex", "Tap the focus: the parabola opens toward it"),
  accept: (ctx, ins) => (ins.length === 0 ? [place(ctx)] : [...ins, pos(ctx)]),
  done: (ins) => ins.length === 2,
  build: (d, ins) => d.addParabolaToward(realize(d, ins[0].place), ...ins[1].pos),
};

function fociTool(kind) {
  return {
    id: kind + "Foci",
    label: (kind === "ellipse" ? "Ellipse" : "Hyperbola") + " by foci",
    icon: kind + "Foci",
    hint: steps("Tap the first focus", "Tap the second focus", "Tap a point on the " + kind),
    accept: nextPlace,
    done: (ins) => ins.length === 3,
    build: (d, ins) => d.addConicByFoci(kind, ...ins.map((i) => realize(d, i.place))),
  };
}

const parabolaFocusTool = {
  id: "parabolaFocus",
  label: "Parabola by focus and directrix",
  icon: "parabolaFocus",
  hint: steps("Tap the focus", "Tap the directrix, a line"),
  accept(ctx, ins) {
    if (ins.length === 0) return [place(ctx)];
    const l = pick(ctx, isLine);
    return l === null ? null : [...ins, { ent: l }];
  },
  done: (ins) => ins.length === 2,
  build: (d, ins) => d.addParabolaByFocus(realize(d, ins[0].place), ins[1].ent),
};

const midpointTool = {
  id: "midpoint",
  label: "Midpoint",
  key: "m",
  icon: "midpoint",
  hint: steps("Tap a segment, or the first of two points", "Tap the second point"),
  accept(ctx, ins) {
    if (ins.length === 0 && ctx.snap.kind !== "point") {
      const s = pick(ctx, isSegment(ctx));
      if (s !== null) return [{ ent: s }];
    }
    return nextPlace(ctx, ins);
  },
  done: (ins) => (ins.length === 1 && ins[0].ent !== undefined) || ins.length === 2,
  build: (d, ins) =>
    ins[0].ent !== undefined ? d.addLineMidpoint(ins[0].ent) : d.addMidpoint(realize(d, ins[0].place), realize(d, ins[1].place)),
};

const intersectTool = {
  id: "intersect",
  label: "Intersection",
  key: "i",
  icon: "intersect",
  hint: steps("Tap where two curves cross, or tap the first curve", "Tap the second curve near the crossing"),
  accept(ctx, ins) {
    const near = ctx.near.map((n) => n.e);
    if (ins.length === 0) {
      if (near.length >= 2) return [{ ent: near[0] }, { ent: near[1] }, pos(ctx)];
      return near.length ? [{ ent: near[0] }] : null;
    }
    const b = near.find((e) => e !== ins[0].ent);
    return b === undefined ? null : [...ins, { ent: b }, { pos: ctx.w }];
  },
  done: (ins) => ins.length === 3,
  build: (d, ins) => d.addIntersectionNear(ins[0].ent, ins[1].ent, ...ins[2].pos),
};

function dirLineTool(perp) {
  return {
    id: perp ? "perpLine" : "parLine",
    label: perp ? "Perpendicular line" : "Parallel line",
    key: perp ? "n" : "r",
    icon: perp ? "perpLine" : "parLine",
    hint: steps("Tap the line to be " + (perp ? "perpendicular" : "parallel") + " to", "Tap the point it goes through"),
    accept(ctx, ins) {
      if (ins.length === 0) {
        const l = pick(ctx, isLine);
        return l === null ? null : [{ ent: l }];
      }
      return [...ins, place(ctx)];
    },
    done: (ins) => ins.length === 2,
    build: (d, ins) => (perp ? d.addPerpLine : d.addParallelLine).call(d, ins[0].ent, realize(d, ins[1].place)),
  };
}

const perpBisectorTool = {
  id: "perpBisector",
  label: "Perpendicular bisector",
  key: "w",
  icon: "perpBisector",
  hint: steps("Tap a segment, or the first of two points", "Tap the second point"),
  accept: midpointTool.accept,
  done: midpointTool.done,
  build(d, ins) {
    if (ins[0].ent !== undefined) {
      const [a, b] = d.linePts(ins[0].ent);
      return d.addPerpBisector(a, b)[0];
    }
    return d.addPerpBisector(realize(d, ins[0].place), realize(d, ins[1].place))[0];
  },
};

const angleBisectorTool = {
  id: "angleBisector",
  label: "Angle bisector",
  key: "g",
  icon: "angleBisector",
  hint: steps("Tap the first line", "Tap the second line", "Tap inside the angle to bisect"),
  accept(ctx, ins) {
    if (ins.length < 2) {
      const l = pick(ctx, (k, e) => isLine(k) && !ins.some((i) => i.ent === e));
      return l === null ? null : [...ins, { ent: l }];
    }
    return [...ins, { pos: ctx.w }];
  },
  done: (ins) => ins.length === 3,
  build: (d, ins) => d.addAngleBisector(ins[0].ent, ins[1].ent, ...ins[2].pos)[0],
};

const tangentLineTool = {
  id: "tangentLine",
  label: "Tangent line",
  key: "j",
  icon: "tangentLine",
  hint: steps("Tap the point the tangent starts from", "Tap the curve near where it should touch"),
  accept(ctx, ins) {
    if (ins.length === 0) return [place(ctx)];
    const c = pick(ctx, touchable);
    return c === null ? null : [...ins, { ent: c }, { pos: ctx.w }];
  },
  done: (ins) => ins.length === 3,
  build: (d, ins) => d.addTangentLineNear(realize(d, ins[0].place), ins[1].ent, ...ins[2].pos)[0],
};

const reflectTool = {
  id: "reflect",
  label: "Reflection",
  key: "u",
  icon: "reflect",
  hint: steps("Tap the point to reflect", "Tap the mirror line"),
  accept(ctx, ins) {
    if (ins.length === 0) return [place(ctx)];
    const l = pick(ctx, isLine);
    return l === null ? null : [...ins, { ent: l }];
  },
  done: (ins) => ins.length === 2,
  build: (d, ins) => d.addReflection(realize(d, ins[0].place), ins[1].ent),
};

function featureTool(id, label, key, hint, pred, build) {
  return {
    id,
    label,
    key,
    icon: id,
    hint: steps(hint),
    accept(ctx) {
      const c = pick(ctx, (k, e) => isConic(k) && pred(ctx.doc, e));
      return c === null ? null : [{ ent: c }, { pos: ctx.w }];
    },
    done: (ins) => ins.length === 2,
    build: (d, ins) => build(d, ins[0].ent, ins[1].pos),
  };
}

const anyConic = () => true;
const hyperbola = (doc, e) => conicKind(doc, e) === "hyperbola";

// ------------------------------------------------------------------ measuring

/** The dimension that entities `ents` and a label at world position `p` describe: {kind, a, b}, or null. */
export function dimSpec(doc, ents, p) {
  const k = (e) => doc.entKind(e);
  const [a, b] = ents;
  if (ents.length === 1) {
    switch (k(a)) {
      case "point": {
        const q = doc.pointPos(a);
        // a label above or below the point measures its x, one beside it its y
        return { kind: Math.abs(p[1] - q[1]) > Math.abs(p[0] - q[0]) ? "x" : "y", a };
      }
      case "line":
        return { kind: "length", a };
      case "circle":
      case "arc":
        return { kind: "radius", a };
      case "conic":
        return { kind: "conic_a", a };
    }
    return null;
  }
  if (ents.length !== 2) return null;
  const [ka, kb] = [k(a), k(b)];
  if (ka === "point" && kb === "point") return { kind: "distance", a, b };
  if (ka === "point" && kb === "line") return { kind: "point_line", a, b };
  if (ka === "line" && kb === "point") return { kind: "point_line", a: b, b: a };
  if (ka === "line" && kb === "line") {
    if (parallel(doc, a, b)) return { kind: "point_line", a: doc.linePts(b)[0], b: a };
    return { kind: "angle", a, b };
  }
  return null;
}

export function lineDir(doc, l) {
  const [a, b] = doc.linePts(l);
  const pa = doc.pointPos(a), pb = doc.pointPos(b);
  return [pb[0] - pa[0], pb[1] - pa[1]];
}

export function parallel(doc, l1, l2) {
  const u = lineDir(doc, l1), v = lineDir(doc, l2);
  return Math.abs(u[0] * v[1] - u[1] * v[0]) < 1e-6 * Math.hypot(...u) * Math.hypot(...v);
}

/** Adds the dimension `spec` with its label at world position `p`; returns it. */
export function addDimAt(d, spec, p) {
  const an = d.dimAnchorOf(spec.kind, spec.a, spec.b);
  return d.addDim(spec.kind, spec.a, spec.b, p[0] - an[0], p[1] - an[1]);
}

const measureTool = {
  id: "measure",
  label: "Measure",
  key: "d",
  icon: "measure",
  hint(ins) {
    if (ins.length === 0) return "Tap what to measure: a point, segment, circle, conic, or two of them";
    if (ins.length === 1) return "Tap a second item, or tap empty space to place the label";
    return "Tap empty space to place the label";
  },
  accept(ctx, ins) {
    if (ctx.hit?.t === "dim") return { edit: ctx.hit.i };
    if (ctx.hit?.t === "ent" && !ctx.scene.ent(ctx.hit.i)?.hidden) {
      const e = ctx.hit.i;
      if (ins.some((i) => i.ent === e)) return ins.filter((i) => i.ent !== e);
      return ins.length >= 2 ? [{ ent: e }] : [...ins, { ent: e }];
    }
    const spec = dimSpec(ctx.doc, ins.map((i) => i.ent), ctx.w);
    return spec ? [...ins, { pos: ctx.w, spec }] : null;
  },
  done: (ins) => ins.length > 0 && ins[ins.length - 1].pos !== undefined,
  build: (d, ins) => addDimAt(d, ins[ins.length - 1].spec, ins[ins.length - 1].pos),
  onDone: (app, dim) => app.selectDim(dim, true),
};

// ------------------------------------------------------------------ registry

export const TOOLS = {};
for (const t of [
  pointTool,
  lineTool,
  circleTool,
  circle3Tool,
  arcTool,
  arc3Tool,
  conicTool("ellipse", "e", "Ellipse"),
  conicTool("hyperbola", "y", "Hyperbola"),
  parabolaTool,
  fociTool("ellipse"),
  fociTool("hyperbola"),
  parabolaFocusTool,
  midpointTool,
  intersectTool,
  dirLineTool(true),
  dirLineTool(false),
  perpBisectorTool,
  angleBisectorTool,
  tangentLineTool,
  reflectTool,
  featureTool("foci", "Foci of a conic", "k", "Tap a conic to add its foci", anyConic, (d, c) => d.addFoci(c)[0]),
  featureTool("directrix", "Directrix of a conic", "q", "Tap a conic on the side of the directrix", anyConic, (d, c, p) =>
    d.addDirectrix(c, ...p),
  ),
  featureTool("asymptotes", "Asymptotes", "o", "Tap a hyperbola to add its asymptotes", hyperbola, (d, c) => d.addAsymptotes(c)[0]),
  measureTool,
]) {
  TOOLS[t.id] = t;
}
TOOLS.select = { id: "select", label: "Select", key: "s", icon: "select", hint: () => "" };
TOOLS.function = { id: "function", label: "Function", key: "f", icon: "function", hint: () => "Type a curve: y = f(x), or an equation in x and y" };

/** The tool rail: single tools and groups of tools. */
export const RAIL = [
  { tool: "select" },
  { tool: "point" },
  { tool: "line" },
  { group: "Circles", tools: ["circle", "circle3", "arc", "arc3"] },
  { group: "Conics", tools: ["ellipse", "hyperbola", "parabola", "ellipseFoci", "hyperbolaFoci", "parabolaFocus"] },
  { tool: "function" },
  {
    group: "Construct",
    tools: ["midpoint", "intersect", "perpLine", "parLine", "perpBisector", "angleBisector", "tangentLine", "reflect", "foci", "directrix", "asymptotes"],
  },
  { tool: "measure" },
];

// ------------------------------------------------------------------ the engine

/** What a tool sees at screen position (x, y) with hit radius r: the snap place, the item hit, the curves
 *  near, and the world position. */
export function contextAt(doc, scene, view, x, y, r) {
  return {
    doc,
    scene,
    view,
    x,
    y,
    w: view.toWorld(x, y),
    r,
    snap: scene.snap(doc, view, x, y, r),
    hit: scene.hit(x, y, r, { markers: false }),
    near: scene.curvesNear(x, y, r),
  };
}

/** Entities an input list refers to: what was picked and the existing points used, and with `curves` also
 *  the curves new points go on (for flashing a refused change). */
export function inputEnts(ins, curves = false) {
  const out = [];
  for (const i of ins) {
    if (i.ent !== undefined) out.push(i.ent);
    if (i.place?.kind === "point") out.push(i.place.e);
    if (curves && i.place?.kind === "on") out.push(i.place.c);
  }
  return out;
}

/**
 * Handles a click of the current tool. Returns {inputs} with the inputs to keep, plus `result` after a
 * change, `error` when the change was refused, `edit` for a dimension to edit, or `refused` when the click
 * fits no step.
 */
export function click(tool, doc, ctx, ins) {
  const next = tool.accept(ctx, ins);
  if (next === null) return { inputs: ins, refused: true };
  if (next.edit !== undefined) return { inputs: [], edit: next.edit };
  if (!tool.done(next)) return { inputs: next };
  try {
    const result = change(doc, (d) => tool.build(d, next));
    return { inputs: tool.after ? tool.after(next, result, doc) : [], result };
  } catch (e) {
    // the refused click is dropped, so another try at the last step can follow
    return { inputs: ins, error: e, ents: inputEnts(next, true) };
  }
}

/**
 * What a click at the pointer would make: calls `draw(copy, before, inputs)` with a copy of the document
 * that has it (`before` is the entity count before the change), or returns false when the click would not
 * complete the tool.
 */
export function preview(tool, doc, ctx, ins, draw) {
  if (!tool.accept) return false;
  const next = tool.accept(ctx, ins);
  if (!next || next.edit !== undefined || !tool.done(next)) return false;
  return onCopy(doc, (d) => tool.build(d, next), (copy, r, before) => (draw(copy, before, next, r), true)) ?? false;
}

export { Rejected, must };
