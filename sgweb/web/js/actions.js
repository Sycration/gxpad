// What can be done with the current selection: constraints between the selected entities, measurements of
// them, and their properties. The app shows these as labelled chips next to the selection.

import { addDimAt, parallel } from "./tools.js";

/** The selected entities, dimensions, constraints and variables of a selection list. */
export function split(sel) {
  const of = (t) => sel.filter((s) => s.t === t).map((s) => s.i);
  return { ents: of("ent"), dims: of("dim"), cons: of("con"), vars: of("var") };
}

/** Where to put the label of a new dimension of `spec`: `off` pixels out from its anchor, in world units. */
function labelFor(doc, spec, scale) {
  const an = doc.dimAnchorOf(spec.kind, spec.a, spec.b);
  const o = 40 / scale;
  switch (spec.kind) {
    case "hdist":
      return [an[0], an[1] + o * 1.2];
    case "vdist":
      return [an[0] + o * 1.2, an[1]];
    case "x": {
      const q = doc.pointPos(spec.a);
      return [an[0], q[1] >= 0 ? -o * 0.6 : o * 0.6];
    }
    case "y": {
      const q = doc.pointPos(spec.a);
      return [q[0] >= 0 ? -o * 0.9 : o * 0.9, an[1]];
    }
    case "radius":
    case "diameter": {
      const [c, r] = doc.circleVals(spec.a);
      const p = doc.pointPos(c);
      return [p[0] + (r + o) * 0.7071, p[1] + (r + o) * 0.7071];
    }
    case "angle":
      return [an[0] + o * 1.5, an[1] + o * 0.6];
  }
  return [an[0] + o * 0.5, an[1] + o];
}

/**
 * The actions for selection `sel` of app `app`: [{label, icon, run, danger?}]. `run` performs the action
 * through the app, which reports refusals.
 */
export function actionsFor(app) {
  const doc = app.doc, scene = app.scene;
  const { ents, dims, cons, vars } = split(app.sel);
  const out = [];
  const kind = (e) => scene.ent(e)?.kind;
  const con = (label, icon, k, a, b) => out.push({ label, icon, run: () => app.constrain(k, a, b) });
  const dim = (label, icon, spec) =>
    out.push({ label, icon, run: () => app.addDimension(spec, labelFor(doc, spec, app.view.scale)) });
  const del = () => out.push({ label: "Delete", icon: "trash", danger: true, run: () => app.deleteSelection() });

  if (dims.length === 1 && ents.length === 0 && cons.length === 0 && vars.length === 0) {
    const d = dims[0];
    const driving = doc.dimDriving(d);
    out.push({ label: "Edit value", icon: "edit", run: () => app.editDim(d) });
    out.push({
      label: driving ? "Make measured" : "Make driving",
      icon: driving ? "measured" : "driving",
      run: () => app.setDriving(d, !driving),
    });
    if (doc.dimKind(d) === "angle") out.push({ label: "Other angle", icon: "supplement", run: () => app.edit(() => doc.toggleSupp(d)) });
    del();
    return out;
  }
  if (ents.length === 0) {
    if (app.sel.length) del();
    return out;
  }

  const is = (e, ...k) => k.includes(kind(e));
  const pts = ents.filter((e) => is(e, "point"));
  const lines = ents.filter((e) => is(e, "line"));
  const rounds = ents.filter((e) => is(e, "circle", "arc"));
  const n = ents.length;
  const [a, b] = ents;
  const curve = (e) => kind(e) && kind(e) !== "point";

  if (n === 1 && is(a, "point")) {
    dim("x coordinate", "coordX", { kind: "x", a });
    dim("y coordinate", "coordY", { kind: "y", a });
    if (!scene.ent(a).builtin) {
      const locked = scene.ent(a).locked;
      out.push({ label: locked ? "Unlock" : "Lock", icon: locked ? "unlock" : "lock", run: () => app.edit(() => doc.lockPoints([a], !locked)) });
    }
  } else if (n === 1 && is(a, "line")) {
    con("Horizontal", "horizontal", "horizontal", a);
    con("Vertical", "vertical", "vertical", a);
    dim("Length", "distance", { kind: "length", a });
  } else if (n === 1 && is(a, "circle", "arc")) {
    dim("Radius", "radius", { kind: "radius", a });
    dim("Diameter", "diameter", { kind: "diameter", a });
  } else if (n === 1 && is(a, "conic")) {
    dim("Semi-axis a", "axisA", { kind: "conic_a", a });
    if (doc.conicVals(a)[0] !== "parabola") dim("Semi-axis b", "axisB", { kind: "conic_b", a });
  } else if (n === 2 && pts.length === 2) {
    con("Coincident", "coincident", "coincident", a, b);
    con("Horizontal", "horizontal", "horizontal", a, b);
    con("Vertical", "vertical", "vertical", a, b);
    dim("Distance", "distance", { kind: "distance", a, b });
    dim("Horizontal distance", "hdist", { kind: "hdist", a, b });
    dim("Vertical distance", "vdist", { kind: "vdist", a, b });
  } else if (n === 2 && pts.length === 1) {
    const p = pts[0], c = a === p ? b : a;
    con("On curve", "onCurve", "on_curve", p, c);
    if (is(c, "line")) {
      con("Midpoint", "midpoint", "midpoint", p, c);
      dim("Distance", "distance", { kind: "point_line", a: p, b: c });
    }
    if (is(c, "circle", "arc", "conic")) con("At center", "concentric", "concentric", p, c);
  } else if (n === 2 && lines.length === 2) {
    con("Parallel", "parallel", "parallel", a, b);
    con("Perpendicular", "perpendicular", "perpendicular", a, b);
    con("Collinear", "collinear", "collinear", a, b);
    con("Equal length", "equal", "equal", a, b);
    if (parallel(doc, a, b)) dim("Distance", "distance", { kind: "point_line", a: doc.linePts(b)[0], b: a });
    else dim("Angle", "angle", { kind: "angle", a, b });
  } else if (n === 2 && curve(a) && curve(b)) {
    if (doc.canConstrain("tangent", a, b)) con("Tangent", "tangent", "tangent", a, b);
    const lc = (e) => is(e, "line", "circle", "arc");
    if (lc(a) && lc(b)) con("Equal", "equal", "equal", a, b);
    const rc = (e) => is(e, "circle", "arc", "conic");
    if (rc(a) && rc(b)) con("Concentric", "concentric", "concentric", a, b);
  }
  if (n === 3 && pts.length === 2 && lines.length === 1) {
    out.push({ label: "Symmetric", icon: "symmetric", run: () => app.edit(() => doc.constrainSymmetric(pts[0], pts[1], lines[0])) });
  }
  const chain = (label, icon, k) => out.push({ label, icon, run: () => app.constrainChain(k, ents) });
  if (n > 2 && lines.length === n) {
    chain("All parallel", "parallel", "parallel");
    chain("All equal", "equal", "equal");
  }
  if (n > 2 && rounds.length === n) chain("Equal radii", "equal", "equal");
  if (n > 2 && pts.length === n) {
    chain("In a row", "horizontal", "horizontal");
    chain("In a column", "vertical", "vertical");
  }
  if (n === 1 && is(a, "line") && !scene.ent(a).builtin) {
    const inf = scene.ent(a).infinite;
    out.push({ label: inf ? "Make segment" : "Make infinite", icon: "infinite", run: () => app.edit(() => doc.toggleInfinite([a])) });
  }
  const shaped = ents.filter((e) => curve(e) && !is(e, "curve") && !scene.ent(e).builtin);
  if (shaped.length) {
    const on = !shaped.every((e) => scene.ent(e).construction);
    out.push({ label: on ? "Construction" : "Regular", icon: "construction", run: () => app.edit(() => doc.setConstructionAll(shaped, on)) });
  }
  if (ents.some((e) => !scene.ent(e)?.builtin) || dims.length || cons.length || vars.length) del();
  return out;
}

export { addDimAt };
