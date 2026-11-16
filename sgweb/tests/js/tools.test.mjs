// The drawing and construction tools, clicked through the same engine the canvas uses.

import { test } from "node:test";
import assert from "node:assert/strict";
import { Pad, near } from "./pad.mjs";
import { change, at, Rejected } from "../js/sg.js";
import { dimSpec } from "../js/tools.js";

test("a point tool click adds a point, and a click on a curve keeps it there", () => {
  const p = new Pad();
  p.click("point", [1, 2]);
  assert.equal(p.made("point").length, 1);
  // on the x axis: the point is held on it
  p.click("point", [3, 0.01]);
  const q = p.made("point").at(-1);
  assert.ok(near(p.doc.pointPos(q)[1], 0, 1e-9));
  assert.ok([...Array(p.doc.conCount).keys()].some((c) => p.doc.conKind(c) === "on_curve"));
});

test("each click of a tool is one undo step, however many points it makes", () => {
  const p = new Pad();
  const n = p.doc.undoLen();
  p.clicks("circle3", [1, 0], [0, 1], [-1, 0]);
  assert.equal(p.doc.undoLen(), n + 1);
  assert.equal(p.made("circle").length, 1);
  p.doc.undo();
  assert.equal(p.made("point").length, 0);
  assert.equal(p.made("circle").length, 0);
});

test("lines chain from the end of the last segment, and snap to the axes", () => {
  const p = new Pad();
  p.clicks("line", [0, 1], [3, 1.05]);
  assert.equal(p.made("line").length, 1);
  // the second click was within 3° of horizontal: the line is held horizontal
  assert.ok([...Array(p.doc.conCount).keys()].some((c) => p.doc.conKind(c) === "horizontal"));
  assert.equal(p.inputs.length, 1, "the chain goes on");
  p.click("line", [3, 4]);
  assert.equal(p.made("line").length, 2);
  // a click on an existing point ends the chain
  const first = p.made("point")[0];
  p.click("line", p.doc.pointPos(first));
  assert.equal(p.made("line").length, 3);
  assert.equal(p.inputs.length, 0);
});

test("a circle through a free position keeps its radius free; through a point it is bound", () => {
  const p = new Pad();
  p.clicks("circle", [0, 0], [1.2, 1.6]);
  const c = p.made("circle")[0];
  assert.ok(near(p.doc.circleVals(c)[1], 2));
  assert.equal(p.made("point").length, 0, "the center is the origin, and no point marks the radius");
  const q = new Pad();
  q.click("point", [5, 5]);
  q.clicks("circle", [3, 5], [5, 5]);
  assert.ok(near(q.doc.circleVals(q.made("circle")[0])[1], 2));
  const [pt] = q.made("point");
  q.doc.dragPoint(pt, 6, 5);
  q.doc.solve();
  const [c2, r2] = q.doc.circleVals(q.made("circle")[0]);
  const a = q.doc.pointPos(c2), b = q.doc.pointPos(pt);
  assert.ok(near(Math.hypot(b[0] - a[0], b[1] - a[1]), r2), "the circle follows the point it passes through");
});

test("arcs end on an existing point, or point toward the third click", () => {
  const p = new Pad();
  p.clicks("arc", [1, 1], [3, 1], [1, 4]);
  const a = p.made("arc")[0];
  const [c, s, e] = p.doc.arcPts(a);
  const pc = p.doc.pointPos(c), pe = p.doc.pointPos(e);
  assert.ok(near(Math.hypot(pe[0] - pc[0], pe[1] - pc[1]), 2));
  assert.ok(near(pe[0], 1, 1e-6) && pe[1] > 1);
  void s;
});

test("conics from center and axes, and by foci", () => {
  const p = new Pad();
  p.clicks("ellipse", [0, 0], [3, 0], [0, 1.5]);
  const [kind, , a, b] = p.doc.conicVals(p.made("conic")[0]);
  assert.equal(kind, "ellipse");
  assert.ok(near(a, 3) && near(b, 1.5));
  const q = new Pad();
  q.clicks("hyperbolaFoci", [-4, 1], [4, 1], [3, 2]);
  assert.equal(q.doc.conicVals(q.made("conic")[0])[0], "hyperbola");
  const r = new Pad();
  r.clicks("parabolaFocus", [0, 2], [1, 0]);
  assert.equal(r.doc.conicVals(r.made("conic")[0])[0], "parabola");
  assert.ok(near(r.doc.conicVals(r.made("conic")[0])[2], 1));
});

test("a refused click changes nothing and can be tried again", () => {
  const p = new Pad();
  p.clicks("circle3", [0, 0], [1, 1]);
  const gen = p.doc.generation, undo = p.doc.undoLen();
  const r = p.click("circle3", [2, 2]);
  assert.ok(r.error, "three points on a line make no circle");
  assert.equal(p.doc.generation, gen);
  assert.equal(p.doc.undoLen(), undo);
  assert.equal(p.made("point").length, 0, "no points were left behind");
  assert.equal(p.inputs.length, 2);
  p.click("circle3", [2, 0]);
  assert.equal(p.made("circle").length, 1);
});

test("change runs on a copy first, so a rejection leaves no points behind", () => {
  const p = new Pad();
  assert.throws(
    () =>
      change(p.doc, (d) => {
        d.addPoint(1, 1);
        throw new Rejected();
      }),
    Rejected,
  );
  assert.equal(p.made("point").length, 0);
});

test("midpoint of a segment, and of two places", () => {
  const p = new Pad();
  p.clicks("line", [0, 0.5], [4, 2.5]);
  p.inputs = [];
  p.click("midpoint", [1, 1]);
  const m = p.made("point").at(-1);
  assert.deepEqual(p.doc.pointPos(m).map((x) => +x.toFixed(9)), [2, 1.5]);
  p.clicks("midpoint", [6, 6], [8, 8]);
  assert.deepEqual(p.doc.pointPos(p.made("point").at(-1)).map((x) => +x.toFixed(9)), [7, 7]);
});

test("intersection where two curves cross", () => {
  const p = new Pad();
  p.clicks("line", [-3, -2], [3, 4]);
  p.inputs = [];
  p.clicks("circle", [5, 5], [5, 7]);
  // the line y = x + 1 meets the y axis at (0, 1)
  const n = p.made("point").length;
  p.click("intersect", [0, 1]);
  assert.equal(p.made("point").length, n + 1);
  const x = p.doc.pointPos(p.made("point").at(-1));
  assert.ok(near(x[0], 0, 1e-9) && near(x[1], 1, 1e-9));
});

test("perpendicular line, tangent line and reflection", () => {
  const p = new Pad();
  p.clicks("line", [0, 3], [4, 3]);
  p.inputs = [];
  const l = p.made("line")[0];
  p.clicks("perpLine", [2, 3], [1, 6]);
  const perp = p.made("line").find((e) => e !== l);
  const [a, b] = p.doc.linePts(perp);
  assert.ok(near(p.doc.pointPos(a)[0], p.doc.pointPos(b)[0], 1e-9), "vertical, like a perpendicular to a horizontal line");

  const q = new Pad();
  q.clicks("circle", [0, 0], [1.2, 1.6]);
  q.clicks("tangentLine", [5, 0.5], [1.2, 1.6]);
  const t = q.made("line")[0];
  assert.ok(t !== undefined, "a tangent line was made");
  // the solver may move free values to make it touch, so the radius is read afterwards
  const contact = q.doc.pointPos(q.doc.linePts(t)[1]);
  const radius = q.doc.circleVals(q.made("circle")[0])[1];
  assert.ok(near(Math.hypot(...contact), radius, 1e-6), "it touches the circle");
  assert.ok(contact[1] > 0, "on the side clicked");

  const r = new Pad();
  r.click("point", [1, 2]);
  r.clicks("reflect", [1, 2], [3, 0]);
  const pts = r.made("point");
  assert.deepEqual(r.doc.pointPos(pts.at(-1)).map((x) => +x.toFixed(9)), [1, -2]);
});

test("the preview of the last click builds what the click would", () => {
  const p = new Pad();
  p.clicks("circle3", [1, 0], [0, 1]);
  const made = p.preview("circle3", [-1, 0]);
  assert.ok(made && made.after > made.before);
  assert.equal(p.made("circle").length, 0, "a preview leaves the document alone");
  assert.equal(p.preview("circle3", [2, 1]) !== null, true);
  assert.equal(p.preview("circle3", [2, -1]), null, "on a line with the first two clicks: no circle");
  p.inputs = [];
  assert.equal(p.preview("circle3", [5, 5]), null, "the first click completes nothing");
});

test("measuring: what the picked items and the label position give", () => {
  const p = new Pad();
  p.clicks("line", [0, 0.5], [3, 0.5]);
  p.inputs = [];
  p.clicks("line", [0, 0.5], [0, 4]);
  p.inputs = [];
  const [l1, l2] = p.made("line");
  const [a, b] = p.doc.linePts(l1);
  assert.equal(dimSpec(p.doc, [l1], [1, 1]).kind, "length");
  assert.equal(dimSpec(p.doc, [a, b], [1, 1]).kind, "distance");
  assert.equal(dimSpec(p.doc, [l1, l2], [1, 1]).kind, "angle");
  assert.equal(dimSpec(p.doc, [a], [0.1, 3]).kind, "x");
  assert.equal(dimSpec(p.doc, [a], [3, 0.6]).kind, "y");

  // with the tool: pick the segment, place the label
  p.click("measure", [1.5, 0.5]);
  assert.equal(p.inputs.length, 1);
  const r = p.click("measure", [1.5, -1]);
  assert.equal(typeof r.result, "number");
  assert.equal(p.doc.dimKind(r.result), "length");
  assert.ok(near(p.doc.dimValue(r.result), 3));
});

test("places resolve to points, positions or curves", () => {
  const p = new Pad();
  p.click("point", [2, 2]);
  const e = p.made("point")[0];
  assert.deepEqual(p.ctx([2.05, 2.05]).snap, at.point(e));
  assert.equal(p.ctx([4, 0.02]).snap.kind, "on");
  assert.equal(p.ctx([4, 3]).snap.kind, "free");
});
