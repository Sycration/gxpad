// Test helpers: the web app's modules on the sggeo module, without a page. tests/web.rs writes the embedded
// app to a directory and these tests to its tests/, so `../js` and `../pkg` are what the server serves.

import { readFileSync } from "node:fs";
import { initSync } from "../pkg/sggeo.js";
import { Doc } from "../js/sg.js";
import { Scene } from "../js/scene.js";
import { View } from "../js/view.js";
import { TOOLS, click, contextAt, preview } from "../js/tools.js";

initSync({ module: readFileSync(new URL("../pkg/sggeo_bg.wasm", import.meta.url)) });

export { Doc };

/** A document with a view, a scene and the inputs of a tool, clicked at world positions. */
export class Pad {
  constructor(doc = new Doc()) {
    this.doc = doc;
    this.view = new View();
    this.view.w = 800;
    this.view.h = 600;
    this.view.scale = 50;
    this.scene = new Scene();
    this.inputs = [];
  }

  ctx(p) {
    this.scene.update(this.doc, this.view);
    const [x, y] = this.view.toScreen(p);
    return contextAt(this.doc, this.scene, this.view, x, y, 9);
  }

  /** Clicks tool `tool` at world position `p`; returns the engine's answer. */
  click(tool, p) {
    const r = click(TOOLS[tool], this.doc, this.ctx(p), this.inputs);
    this.inputs = r.inputs;
    return r;
  }

  /** Clicks each of `ps` in turn with `tool`; returns the last answer. */
  clicks(tool, ...ps) {
    let r;
    for (const p of ps) r = this.click(tool, p);
    return r;
  }

  /** Whether a click of `tool` at `p` would complete a change, and the entity count it would leave. */
  preview(tool, p) {
    let made = null;
    const shown = preview(TOOLS[tool], this.doc, this.ctx(p), this.inputs, (copy, before) => (made = { before, after: copy.entCount }));
    return shown ? made : null;
  }

  of(kind) {
    const out = [];
    for (let e = 0; e < this.doc.entCount; e++) if (this.doc.entKind(e) === kind && !this.doc.entHidden(e)) out.push(e);
    return out;
  }

  /** Entities of `kind` that are not built in (the origin and the axes). */
  made(kind) {
    const d = this.doc;
    return this.of(kind).filter((e) => e !== d.origin && e !== d.xAxis && e !== d.yAxis);
  }
}

export const near = (a, b, eps = 1e-6) => Math.abs(a - b) <= eps * (1 + Math.abs(b));
