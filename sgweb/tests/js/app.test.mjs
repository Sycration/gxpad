// Everything else the app does without a page: the examples it ships, links, the selection actions and
// formula ranking.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { Doc, Pad } from "./pad.mjs";
import { packLink, unpackLink } from "../js/files.js";
import { actionsFor } from "../js/actions.js";
import { settled } from "../js/formula.js";
import { fmt, splitSub } from "../js/util.js";
import { iconNames } from "../js/icons.js";
import { TOOLS } from "../js/tools.js";

const dir = new URL("../examples/", import.meta.url);

test("every example opens and solves, and the first measurement of each featured one derives", () => {
  const index = JSON.parse(readFileSync(new URL("index.json", dir), "utf8"));
  const files = readdirSync(dir).filter((f) => f.endsWith(".sgp"));
  assert.equal(index.filter((x) => !x.folder).length, files.length);
  // the constructions of sggeo's tests, in folders named after their test files without gen_
  const folders = new Set(index.filter((x) => x.folder).map((x) => x.folder));
  assert.ok(folders.has("conics") && folders.has("linkages"), [...folders].join(" "));
  for (const { path, title, folder } of index) {
    assert.ok(title && !title.startsWith("examples/"), `${path} has a title`);
    if (folder) assert.ok(!folder.startsWith("gen_") && path.startsWith(`examples/${folder}/`), path);
    const d = Doc.fromSgp(readFileSync(new URL("../" + path, dir), "utf8"));
    const s = d.solve();
    assert.ok(s.ok, `${path} solves: ${s.errors}`);
    const measured = [...Array(d.dimCount).keys()].find((i) => d.dimKind(i) && !d.dimDriving(i));
    if (measured !== undefined && !folder) {
      const f = d.deriveDim(measured);
      assert.ok(Math.abs(f.value - f.actual) <= 1e-6 * (1 + Math.abs(f.actual)), `${path}: ${f.plain}`);
    }
    // the text the app saves reads back
    Doc.fromSgp(d.toSgp()).free();
    d.free();
  }
});

test("a link carries the construction", async () => {
  const d = Doc.fromSgp(readFileSync(new URL("01-ptolemy.sgp", dir), "utf8"));
  const text = d.toSgp();
  const packed = await packLink(text);
  assert.match(packed, /^[\w-]+$/);
  assert.ok(packed.length < text.length);
  assert.equal(await unpackLink(packed), text);
});

/** A stand-in for the app with what actionsFor reads. */
function fakeApp(pad, sel) {
  pad.scene.update(pad.doc, pad.view);
  const calls = [];
  return {
    doc: pad.doc,
    scene: pad.scene,
    view: pad.view,
    sel,
    calls,
    constrain: (...a) => calls.push(["constrain", ...a]),
    constrainChain: (...a) => calls.push(["chain", ...a]),
    addDimension: (spec) => calls.push(["dim", spec.kind]),
    edit: (fn) => fn(),
    editDim: () => {},
    setDriving: () => {},
    deleteSelection: () => calls.push(["delete"]),
  };
}

test("selection chips fit what is selected", () => {
  const p = new Pad();
  p.clicks("line", [0, 0.5], [3, 0.5]);
  p.inputs = [];
  p.clicks("line", [0, 1], [2, 3]);
  p.inputs = [];
  const [l1, l2] = p.made("line");
  const labels = (sel) => actionsFor(fakeApp(p, sel)).map((a) => a.label);
  const two = labels([
    { t: "ent", i: l1 },
    { t: "ent", i: l2 },
  ]);
  for (const l of ["Parallel", "Perpendicular", "Equal length", "Angle", "Delete"]) assert.ok(two.includes(l), l);
  assert.ok(labels([{ t: "ent", i: l1 }]).includes("Length"));
  const pts = p.made("point");
  assert.ok(labels(pts.slice(0, 2).map((i) => ({ t: "ent", i }))).includes("Distance"));
  assert.ok(labels(pts.slice(0, 3).map((i) => ({ t: "ent", i }))).includes("In a row"));
  // the axes cannot be deleted
  assert.ok(!labels([{ t: "ent", i: p.doc.xAxis }]).includes("Delete"));

  const app = fakeApp(p, [
    { t: "ent", i: l1 },
    { t: "ent", i: l2 },
  ]);
  actionsFor(app).find((a) => a.label === "Parallel").run();
  assert.deepEqual(app.calls[0], ["constrain", "parallel", l1, l2]);
});

test("formula results settle when exact and small", () => {
  assert.ok(settled({ error: "x" }));
  assert.ok(settled({ f: { exact: true, size: 10 } }));
  assert.ok(!settled({ f: { exact: false, size: 10 } }));
  assert.ok(!settled({ f: { exact: true, size: 500 } }));
});

test("numbers and names as the app shows them", () => {
  assert.equal(fmt(3), "3");
  assert.equal(fmt(-0.00001), "-1e-5");
  assert.equal(fmt(2.50000001), "2.5");
  assert.equal(fmt(-0), "0");
  assert.deepEqual(splitSub("d12"), ["d", "12"]);
  assert.deepEqual(splitSub("x_A"), ["x", "A"]);
  assert.deepEqual(splitSub("a"), ["a", ""]);
});

test("every tool has an icon and a hint", () => {
  for (const t of Object.values(TOOLS)) {
    assert.ok(iconNames.includes(t.icon), `${t.id} icon ${t.icon}`);
    assert.ok(t.hint([]).length > 0 || t.id === "select", t.id);
  }
});
