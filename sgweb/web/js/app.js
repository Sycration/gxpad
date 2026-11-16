// The app state and every change the user can make: the document and its view, the selection, the active
// tool and its inputs, drags, and the render loop. The UI modules listen to its events:
//   "doc"   the document changed (any edit, drag, undo)
//   "sel"   the selection changed
//   "tool"  the tool or its inputs changed
//   "frame" a frame was drawn (overlays follow the view)
//   "formula" the formula state changed
//   "toast" {text, kind} a message for the user

import { Doc, describe, extentPoints } from "./sg.js";
import { Scene } from "./scene.js";
import { View } from "./view.js";
import { Renderer, palette } from "./draw.js";
import { TOOLS, click as toolClick, contextAt, preview, addDimAt, inputEnts } from "./tools.js";
import { Formulas } from "./formula.js";
import { load, store } from "./util.js";

const FLASH_MS = 700;

export class App {
  constructor(canvas, module) {
    this.doc = new Doc();
    this.view = new View();
    this.scene = new Scene();
    this.renderer = new Renderer(canvas);
    this.canvas = canvas;
    this.listeners = {};
    /** [{t: "ent" | "dim" | "con" | "var", i}] in the order selected */
    this.sel = [];
    this.hover = null;
    this.tool = "select";
    /** inputs collected by the active tool */
    this.inputs = [];
    /** the pointer over the canvas: {x, y, type}, or null */
    this.pointer = null;
    this.drag = null;
    this.marquee = null;
    this.flash = { ents: [], until: 0 };
    /** dimension whose value editor is open */
    this.editing = null;
    this.showConstraints = load("showConstraints", true);
    this.showLabels = load("showLabels", true);
    this.coarse = matchMedia("(pointer: coarse)").matches;
    /** parts of the canvas covered by panels: {left, top, right, bottom} in CSS pixels */
    this.insets = { left: 0, top: 0, right: 0, bottom: 0 };
    this.name = "Untitled";
    this.lastGen = -1;
    this.raf = 0;
    this.formulas = new Formulas(module, () => this.emit("formula"));
  }

  // ---------------------------------------------------------------- events and frames

  on(ev, fn) {
    (this.listeners[ev] ??= []).push(fn);
  }

  emit(ev, arg) {
    for (const fn of this.listeners[ev] ?? []) fn(arg);
  }

  /** A message for the user; `action` ({label, run}) adds a button, and the message stays until used. */
  toast(text, kind = "info", action) {
    this.emit("toast", { text, kind, action });
  }

  request() {
    if (!this.raf) this.raf = requestAnimationFrame(() => this.frame());
  }

  frame() {
    this.raf = 0;
    this.scene.update(this.doc, this.view);
    const flashing = this.renderer.draw(this);
    const gen = this.doc.generation;
    if (gen !== this.lastGen) {
      this.lastGen = gen;
      // whatever changed the document, items it no longer has leave the selection first
      this.prune();
      this.emit("doc");
    }
    this.formulas.update(this.formulaTarget(), gen, () => this.doc.toSgp(), !this.drag);
    this.emit("frame");
    if (flashing) this.request();
  }

  /** Re-reads the theme colors, after a theme change. */
  restyle() {
    this.renderer.pal = palette();
    this.request();
  }

  resize(w, h) {
    const dpr = window.devicePixelRatio || 1;
    this.view.w = w;
    this.view.h = h;
    this.renderer.resize(w, h, dpr);
    this.request();
  }

  setOption(name, value) {
    this[name] = value;
    store(name, value);
    this.request();
    this.emit("options");
  }

  // ---------------------------------------------------------------- documents

  /** Replaces the document with the construction in .sgp text `text` (a new document, its own history). */
  open(text, name) {
    const d = Doc.fromSgp(text);
    d.solve();
    this.replaceDoc(d, name);
  }

  replaceDoc(d, name = "Untitled") {
    this.doc.free();
    this.doc = d;
    this.name = name;
    this.reset();
    this.fit();
    this.emit("name");
  }

  /** A new, empty document, as an undoable change of this one. */
  clear() {
    this.doc.clear();
    this.name = "Untitled";
    this.reset();
    this.emit("name");
  }

  /** After the document was replaced: everything that named its items goes, all at once, before anything
   *  reacts to the new document. */
  reset() {
    this.sel = [];
    this.tool = "select";
    this.inputs = [];
    this.hover = null;
    this.editing = null;
    this.drag = null;
    this.scene = new Scene();
    this.lastGen = -1;
    this.emit("edit");
    this.emit("sel");
    this.emit("tool");
    this.request();
  }

  setName(name) {
    this.name = name.trim() || "Untitled";
    this.emit("name");
  }

  /** True when the document holds nothing but the origin and the axes. */
  isEmpty() {
    const d = this.doc;
    for (let e = 0; e < d.entCount; e++) {
      if (d.entKind(e) && !d.entHidden(e) && e !== d.origin && e !== d.xAxis && e !== d.yAxis) return false;
    }
    for (let i = 0; i < d.varCount; i++) if (d.varName(i) !== undefined) return false;
    return true;
  }

  /** Shows the whole construction. */
  fit() {
    const d = this.doc;
    let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
    for (let e = 0; e < d.entCount; e++) {
      const k = d.entKind(e);
      if (!k || d.entHidden(e)) continue;
      for (const p of extentPoints(d, e, k)) {
        if (!Number.isFinite(p[0]) || !Number.isFinite(p[1])) continue;
        x0 = Math.min(x0, p[0]);
        y0 = Math.min(y0, p[1]);
        x1 = Math.max(x1, p[0]);
        y1 = Math.max(y1, p[1]);
      }
    }
    if (!(x1 - x0 > 1e-9 || y1 - y0 > 1e-9)) {
      this.view.cx = 0;
      this.view.cy = 0;
      this.view.scale = this.coarse ? 40 : 60;
    } else {
      const i = this.insets;
      this.view.fit([x0, y0, x1, y1], [i.left + 70, i.top + 70, i.right + 70, i.bottom + 70]);
      this.view.scale = Math.min(this.view.scale, 200);
    }
    this.request();
  }

  // ---------------------------------------------------------------- changing the document

  /** Shows a refused change: a message, and the entities involved flash. */
  refuse(err, ents = []) {
    this.flash = { ents, until: performance.now() + FLASH_MS };
    this.toast(err?.message || String(err), "error");
    this.request();
  }

  /**
   * Runs `fn`, which changes the document through its interface: a false result or an error is a refusal,
   * shown with `ents` flashing. Returns the result, or null when refused.
   */
  edit(fn, ents = this.sel.filter((s) => s.t === "ent").map((s) => s.i)) {
    try {
      const r = fn();
      if (r === false) {
        this.refuse(new Error("That does not fit: it conflicts with the construction or is already implied."), ents);
        return null;
      }
      return r;
    } catch (e) {
      this.refuse(e, ents);
      return null;
    } finally {
      this.request();
    }
  }

  constrain(kind, a, b) {
    const p = this.pointer ? this.view.toWorld(this.pointer.x, this.pointer.y) : [0, 0];
    return this.edit(() => this.doc.constrainNear(kind, a, b, p[0], p[1]), [a, b].filter((e) => e !== undefined));
  }

  constrainChain(kind, ents) {
    const pairs = ents.slice(1).map((e, k) => [ents[k], e]);
    return this.edit(() => this.doc.addConstraints(pairs.map(() => kind), pairs.map((p) => p[0]), pairs.map((p) => p[1])), ents);
  }

  /** Adds dimension `spec` ({kind, a, b}) with its label at world position `p`, selects it, and opens its
   *  editor when it is driving. */
  addDimension(spec, p) {
    const i = this.edit(() => addDimAt(this.doc, spec, p), [spec.a, spec.b].filter((e) => e !== undefined));
    if (i !== null) this.selectDim(i, true);
    return i;
  }

  selectDim(i, edit) {
    this.select([{ t: "dim", i }]);
    if (edit && this.doc.dimDriving(i)) this.editDim(i);
  }

  editDim(i) {
    this.editing = i;
    this.emit("edit");
    this.request();
  }

  /** Text of the value editor of dimension `i`: its expression, or its value (degrees for angles). */
  dimEditText(i) {
    const expr = this.doc.dimExpr(i);
    if (expr) return expr;
    let v = this.doc.dimSetValue(i);
    if (this.doc.dimKind(i) === "angle") v = (v * 180) / Math.PI;
    return String(+v.toPrecision(10));
  }

  setDimText(i, text) {
    this.edit(() => this.doc.setDim(i, text), this.doc.dimEnts(i));
  }

  setDriving(i, on) {
    this.edit(() => this.doc.setDimDriving(i, on), this.doc.dimEnts(i));
  }

  deleteSelection() {
    const of = (t) => this.sel.filter((s) => s.t === t).map((s) => s.i);
    const ents = of("ent").filter((e) => !this.scene.ent(e)?.builtin);
    const r = this.edit(() => this.doc.delete(ents, of("con"), of("dim"), of("var")), ents);
    if (r !== null) this.select([]);
  }

  undo() {
    if (this.doc.undo()) this.afterHistory();
  }

  redo() {
    if (this.doc.redo()) this.afterHistory();
  }

  afterHistory() {
    this.inputs = [];
    this.editing = null;
    this.emit("edit");
    this.prune();
    this.emit("tool");
    this.request();
  }

  /** Drops selected items that no longer exist. */
  prune() {
    const d = this.doc;
    const live = (s) =>
      s.t === "ent" ? d.entKind(s.i) : s.t === "con" ? d.conKind(s.i) : s.t === "dim" ? d.dimKind(s.i) : d.varName(s.i);
    const kept = this.sel.filter((s) => live(s) !== undefined);
    if (kept.length !== this.sel.length) this.select(kept);
  }

  // ---------------------------------------------------------------- selection

  isSelected(t, i) {
    return this.sel.some((s) => s.t === t && s.i === i);
  }

  select(list) {
    this.sel = list;
    this.emit("sel");
    this.request();
  }

  /** Adds an item to the selection, or removes it when it is there. */
  toggle(item) {
    if (this.isSelected(item.t, item.i)) this.select(this.sel.filter((s) => !(s.t === item.t && s.i === item.i)));
    else this.select([...this.sel, item]);
  }

  selectAll() {
    const list = [];
    for (const r of this.scene.ents) if (r && !r.hidden && !r.builtin) list.push({ t: "ent", i: r.e });
    this.select(list);
  }

  /** The measurement whose formula is wanted: the last selected dimension or variable. */
  formulaTarget() {
    for (let k = this.sel.length - 1; k >= 0; k--) {
      const s = this.sel[k];
      if (s.t === "dim") {
        const name = this.doc.dimName(s.i);
        if (name) return { kind: "dim", name, i: s.i };
      }
      if (s.t === "var") {
        const name = this.doc.varName(s.i);
        if (name) return { kind: "var", name, i: s.i };
      }
    }
    return null;
  }

  describe(e) {
    return describe(this.doc, e);
  }

  // ---------------------------------------------------------------- tools

  setTool(id) {
    if (!TOOLS[id]) return;
    this.tool = id;
    this.inputs = [];
    if (id === "measure") this.select(this.sel.filter((s) => s.t === "ent").slice(-2));
    this.emit("tool");
    this.request();
  }

  /** Steps back: drops the tool's inputs, or else returns to the select tool, or else clears the selection. */
  back() {
    if (this.editing !== null) {
      this.editing = null;
      this.emit("edit");
    } else if (this.inputs.length) {
      this.inputs = [];
      this.emit("tool");
    } else if (this.tool !== "select") this.setTool("select");
    else this.select([]);
    this.request();
  }

  inputEnts() {
    return inputEnts(this.inputs);
  }

  hitRadius() {
    return this.pointer?.type === "touch" || this.pointer?.type === "pen" ? 18 : 9;
  }

  /** What the active tool sees at the pointer (or at screen position x, y). */
  toolContext(x = this.pointer?.x, y = this.pointer?.y) {
    if (x === undefined) return null;
    return contextAt(this.doc, this.scene, this.view, x, y, this.hitRadius());
  }

  previewTool(ctx, draw) {
    const tool = TOOLS[this.tool];
    // a preview needs a pointer that hovers (a mouse), or a finger that is down
    if (!this.pointer?.live) return false;
    return preview(tool, this.doc, ctx, this.inputs, draw);
  }

  lineAxisHint(ctx) {
    const next = TOOLS.line.accept(ctx, this.inputs);
    return next?.[1]?.axis ?? null;
  }

  /** A click of the active tool at screen position (x, y). */
  toolClickAt(x, y) {
    const tool = TOOLS[this.tool];
    if (!tool.accept) return;
    const ctx = this.toolContext(x, y);
    const r = toolClick(tool, this.doc, ctx, this.inputs);
    this.inputs = r.inputs;
    if (r.edit !== undefined) this.editDim(r.edit);
    else if (r.error) this.refuse(r.error, r.ents);
    else if (r.refused) this.toast(tool.hint(this.inputs), "hint");
    else if (r.result !== undefined) tool.onDone?.(this, r.result);
    this.emit("tool");
    this.request();
  }

  hint() {
    return TOOLS[this.tool].hint(this.inputs);
  }

  addFunction(src) {
    const e = this.edit(() => this.doc.addCurve(src.trim()), []);
    if (e !== null) {
      this.setTool("select");
      this.select([{ t: "ent", i: e }]);
    }
    return e !== null;
  }

  // ---------------------------------------------------------------- dragging

  /** Starts dragging what `hit` names, grabbed at world position `w`; false when it does not move. */
  startDrag(hit, w) {
    const d = this.doc;
    const unlocked = (pts) => pts.filter((p) => !d.pointLocked(p)).map((p) => [p, d.pointPos(p)]);
    let drag = null;
    if (hit.t === "handle") drag = { kind: "handle", e: hit.e, which: hit.which };
    else if (hit.t === "dim") {
      const a = d.dimAnchor(hit.i), off = d.dimOffset(hit.i);
      drag = { kind: "label", d: hit.i, grab: [w[0] - a[0] - off[0], w[1] - a[1] - off[1]] };
    } else if (hit.t === "ent") {
      const e = hit.i, r = this.scene.ent(e);
      if (!r || r.builtin) return false;
      switch (r.kind) {
        case "point":
          if (r.locked) return false;
          {
            const p = d.pointPos(e);
            drag = { kind: "point", e, grab: [w[0] - p[0], w[1] - p[1]] };
          }
          break;
        case "line":
          drag = { kind: "move", pts: unlocked(d.linePts(e).slice(0, 2)), start: w };
          break;
        case "arc":
          drag = { kind: "move", pts: unlocked(d.arcPts(e)), start: w };
          break;
        case "conic":
          drag = { kind: "move", pts: unlocked([d.conicVals(e)[1]]), start: w };
          break;
        case "circle":
          drag = { kind: "circle", e };
          break;
        case "curve":
          drag = { kind: "curve", e };
          break;
      }
      if (drag?.kind === "move" && drag.pts.length === 0) return false;
    }
    if (!drag) return false;
    // the whole drag is one undo step
    d.checkpoint();
    this.drag = drag;
    return true;
  }

  dragTo(w) {
    const g = this.drag, d = this.doc;
    if (!g) return;
    try {
      switch (g.kind) {
        case "point":
          d.dragPoint(g.e, w[0] - g.grab[0], w[1] - g.grab[1]);
          break;
        case "move": {
          const dx = w[0] - g.start[0], dy = w[1] - g.start[1];
          d.dragPoints(
            g.pts.map((p) => p[0]),
            g.pts.flatMap((p) => [p[1][0] + dx, p[1][1] + dy]),
          );
          break;
        }
        case "circle":
          d.dragCircle(g.e, w[0], w[1]);
          break;
        case "curve":
          d.dragCurve(g.e, w[0], w[1]);
          break;
        case "handle":
          d.dragConicHandle(g.e, g.which, w[0], w[1]);
          break;
        case "label": {
          const a = d.dimAnchor(g.d);
          d.dragDimOffset(g.d, w[0] - g.grab[0] - a[0], w[1] - g.grab[1] - a[1]);
          break;
        }
      }
    } catch {
      // a drag that cannot follow the pointer leaves the construction where it was
    }
    this.request();
  }

  endDrag() {
    if (!this.drag) return;
    this.drag = null;
    this.doc.solve();
    this.request();
  }
}
