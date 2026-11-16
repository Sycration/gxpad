// The inspector: the formula of the selected measurement (Result), the variables, functions, dimensions and
// constraints of the construction (Quantities), and the construction as .sgp text (Text).

import { latexPng, describe } from "./sg.js";
import { settled } from "./formula.js";
import { iconEl } from "./icons.js";
import { copyText, fmt, fmtDim, h, splitSub } from "./util.js";

const LANGS = [
  ["c", "C"],
  ["rust", "Rust"],
  ["js", "JavaScript"],
  ["python", "Python"],
  ["mathematica", "Mathematica"],
  ["maxima", "Maxima"],
];

const CON_LABEL = {
  coincident: ["•", "coincident"],
  on_curve: ["∈", "on"],
  midpoint: ["½", "midpoint of"],
  midpoint_of: ["½", "midpoint of"],
  horizontal: ["H", "horizontal"],
  vertical: ["V", "vertical"],
  parallel: ["∥", "parallel"],
  perpendicular: ["⟂", "perpendicular"],
  collinear: ["⋯", "collinear"],
  equal: ["=", "equal"],
  tangent: ["T", "tangent"],
  tangent_at: ["T", "tangent at"],
  concentric: ["◎", "concentric"],
  symmetric: ["⇔", "symmetric"],
  offset: ["∥", "through"],
  bisector: ["∡", "bisects"],
  focus: ["F", "focus of"],
  directrix: ["D", "directrix of"],
  asymptote: ["∼", "asymptote of"],
};

/** A name with its subscript as elements: d12 → d<sub>12</sub>. */
export function nameEl(name) {
  const [b, s] = splitSub(name);
  return h("span.qname", null, b, s ? h("sub", null, s) : null);
}

function iconButton(icon, title, onclick, cls = "") {
  const b = h("button.ibtn" + cls, { type: "button", title, "aria-label": title, onclick });
  b.append(iconEl(icon));
  return b;
}

function toggleButton(icon, title, on, onclick) {
  const b = iconButton(icon, title, onclick, on ? ".on" : "");
  b.setAttribute("aria-pressed", String(on));
  return b;
}

/**
 * A text field that commits on Enter or when it loses focus, and shows `current` again while it is not
 * being edited. `check(text)` returns an error message or null.
 */
function field(cls, current, commit, { check, select, label, inputmode } = {}) {
  const el = h("input" + cls, { type: "text", spellcheck: false, autocomplete: "off", "aria-label": label, inputmode });
  el.value = current;
  el.dataset.current = current;
  const validate = () => el.classList.toggle("bad", !!check?.(el.value));
  el.addEventListener("input", validate);
  el.addEventListener("focus", () => select?.());
  el.addEventListener("keydown", (ev) => {
    if (ev.key === "Enter") el.blur();
    if (ev.key === "Escape") {
      el.value = el.dataset.current;
      el.blur();
    }
  });
  el.addEventListener("change", () => {
    if (el.value !== el.dataset.current) commit(el.value);
  });
  el.set = (v) => {
    el.dataset.current = v;
    if (document.activeElement !== el) {
      el.value = v;
      validate();
    }
  };
  validate();
  return el;
}

export class Inspector {
  constructor(app, root) {
    this.app = app;
    this.root = root;
    this.tab = "result";
    this.images = new Map();
    this.tabs = root.querySelector(".tabs");
    this.panes = {
      result: root.querySelector("#pane-result"),
      quantities: root.querySelector("#pane-quantities"),
      text: root.querySelector("#pane-text"),
    };
    for (const b of this.tabs.querySelectorAll("[data-tab]")) b.addEventListener("click", () => this.show(b.dataset.tab, true));
    this.buildText();
    app.on("doc", () => this.refresh());
    app.on("sel", () => {
      this.refresh();
      // choosing a measurement on the canvas brings its formula forward
      if (!this.picking && app.formulaTarget() && this.tab !== "result") this.show("result");
    });
    app.on("formula", () => this.renderResult());
    app.on("options", () => this.images.clear());
    this.show("result");
  }

  show(tab, user = false) {
    this.tab = tab;
    for (const b of this.tabs.querySelectorAll("[data-tab]")) {
      const on = b.dataset.tab === tab;
      b.classList.toggle("on", on);
      b.setAttribute("aria-selected", String(on));
    }
    for (const [k, p] of Object.entries(this.panes)) p.hidden = k !== tab;
    this.refresh();
    if (user) this.app.emit("tab", tab);
  }

  refresh() {
    if (this.tab === "result") this.renderResult();
    else if (this.tab === "quantities") this.renderQuantities();
    else this.syncText();
  }

  // ---------------------------------------------------------------- result

  /** The formula row as an image in the text color, at the screen's pixel density. */
  image(latex, size) {
    const color = getComputedStyle(document.documentElement).getPropertyValue("--text").trim();
    const rgb = /^#[0-9a-f]{6}$/i.test(color) ? parseInt(color.slice(1), 16) : 0x222222;
    const dpr = window.devicePixelRatio || 1;
    const key = `${latex}|${size}|${rgb}|${dpr}`;
    // {url, width}: the width in CSS pixels is known once the image has loaded once
    let entry = this.images.get(key);
    if (entry === undefined) {
      let url = null;
      try {
        url = URL.createObjectURL(new Blob([latexPng(latex, size, 96 * dpr, rgb)], { type: "image/png" }));
      } catch {}
      if (this.images.size > 200) {
        for (const e of this.images.values()) if (e.url) URL.revokeObjectURL(e.url);
        this.images.clear();
      }
      entry = { url, width: 0 };
      this.images.set(key, entry);
    }
    if (!entry.url) return h("code.latex-fallback", null, latex);
    const img = h("img.formula-row", { src: entry.url, alt: latex, draggable: false });
    if (entry.width) img.style.width = entry.width + "px";
    else {
      // until it has loaded, its size is that of its pixels, several times too large on a dense screen
      img.style.visibility = "hidden";
      img.addEventListener("load", () => {
        entry.width = img.naturalWidth / dpr;
        img.style.width = entry.width + "px";
        img.style.visibility = "";
      });
    }
    return img;
  }

  renderResult() {
    if (this.tab !== "result") return;
    const app = this.app, pane = this.panes.result, F = app.formulas, doc = app.doc;
    const t = app.formulaTarget();
    pane.replaceChildren();
    if (!t) {
      pane.append(
        h(
          "div.empty",
          null,
          iconEl("result", "big"),
          h("h3", null, "No measurement selected"),
          h("p", null, "Select a dimension label or a variable to see a closed-form formula for it, in terms of the values that drive the construction."),
          h("p.muted", null, "Measure things with the Measure tool, or pick two items and use the chips that appear."),
        ),
      );
      return;
    }
    const r = F.result;
    const angle = t.kind === "dim" ? doc.dimKind(t.i) === "angle" : false;
    const value = t.kind === "dim" ? doc.dimValue(t.i) : doc.varValue(t.i);
    const head = h("div.result-head", null, nameEl(t.name), h("span.eq", null, "="), h("span.val", null, fmtDim(value, angle)));
    if (angle) head.append(h("span.muted", null, ` (${fmt(value)} rad)`));
    const status = h("div.result-status");
    if (F.busy) status.append(h("span.spinner", { "aria-hidden": true }), h("span", null, r ? "Looking for a simpler form…" : "Deriving…"));
    if (r && !r.error) {
      status.prepend(h("span.chip." + (r.f.exact ? "good" : "warn"), { title: r.f.exact ? "A closed form" : "Some part was solved numerically" }, r.f.exact ? "Exact" : "Numeric"));
      if (!F.busy) status.append(h("span.muted", null, `${r.ms < 1000 ? Math.round(r.ms) + " ms" : (r.ms / 1000).toFixed(1) + " s"}`));
    }
    if (r && r.gen !== doc.generation) status.append(h("span.muted", null, "for the construction before the last change"));
    pane.append(head, status);
    if (!r) return;
    if (r.error) {
      pane.append(h("p.error", null, r.error));
      return;
    }
    const f = r.f;
    const rows = [f.exact ? f.latexShort : f.latexShort.replace(" = ", " \\approx "), ...f.defs.map((d) => `${d.name} = ${d.latex}`)];
    // the same formula keeps its box, so updates while dragging do not make its images anew
    const boxKey = rows.join("\n") + "|" + getComputedStyle(document.documentElement).getPropertyValue("--text");
    let box = this.formulaBox;
    if (!box || box.dataset.key !== boxKey) {
      box = h("div.formula", { role: "img", "aria-label": f.plain });
      box.dataset.key = boxKey;
      rows.forEach((row, k) => box.append(this.image(row, k === 0 ? 17 : 14)));
      this.formulaBox = box;
    }
    pane.append(box);
    if (!(Math.abs(f.value - f.actual) <= 1e-6 * (1 + Math.abs(f.actual))) && Number.isFinite(f.value)) {
      pane.append(h("p.error", null, iconEl("warning"), ` The formula gives ${fmt(f.value)} here, the construction measures ${fmt(f.actual)}.`));
    }
    const bar = h("div.result-actions");
    if (!settled(r) || !f.exact) {
      const b = h(
        "button.btn",
        { type: "button", disabled: F.busy, title: `Search with ${F.nextSeconds()} s more time for ${f.exact ? "a simpler" : "an exact"} formula`, onclick: () => F.searchLonger() },
        "Search longer",
      );
      bar.append(b);
    }
    const copied = (what) => (ok) => app.toast(ok ? `Copied ${what}` : "Copying is not allowed here", ok ? "info" : "error");
    bar.append(
      h("button.btn", { type: "button", onclick: () => copyText(f.plain).then(copied("the formula")) }, iconEl("copy"), "Text"),
      h("button.btn", { type: "button", onclick: () => copyText(f.latex).then(copied("the LaTeX")) }, iconEl("latex"), "LaTeX"),
    );
    const sel = h("select.btn", { "aria-label": "Copy as code" }, h("option", { value: "" }, "Code…"), ...LANGS.map(([k, n]) => h("option", { value: k }, n)));
    sel.addEventListener("change", () => {
      const k = sel.value;
      sel.value = "";
      if (k) copyText(f.code[k]).then(copied(`the ${LANGS.find((l) => l[0] === k)[1]} code`));
    });
    bar.append(sel);
    pane.append(bar);
  }

  // ---------------------------------------------------------------- quantities

  renderQuantities() {
    const app = this.app, doc = app.doc, pane = this.panes.quantities;
    // the rows are rebuilt only when what they list changes; values update in place, which also keeps
    // what is being typed in a field
    const sig = this.signature();
    if (sig === this.qsig) return this.updateQuantities();
    this.qsig = sig;
    const keep = pane.scrollTop;
    pane.replaceChildren(this.varsSection(doc), this.functionsSection(doc), this.dimsSection(doc), this.consSection(doc));
    pane.scrollTop = keep;
  }

  /** What the list holds, apart from values: when it is unchanged, fields update in place. */
  signature() {
    const d = this.app.doc;
    let s = "";
    for (let i = 0; i < d.varCount; i++) s += `v${d.varName(i)}${d.varExpr(i) === "" ? d.varFree(i) : "e"}${d.varName(i) ? d.varNumeric(i) : ""};`;
    for (let e = 0; e < d.entCount; e++) if (d.entKind(e) === "curve") s += `f${e};`;
    for (let i = 0; i < d.dimCount; i++) if (d.dimKind(i)) s += `d${d.dimName(i)}${d.dimDriving(i)}${d.dimNumeric(i)};`;
    for (let c = 0; c < d.conCount; c++) if (d.conKind(c)) s += `c${c};`;
    return s;
  }

  updateQuantities() {
    for (const el of this.panes.quantities.querySelectorAll("[data-live]")) el.refresh?.();
    for (const r of this.panes.quantities.querySelectorAll(".qrow[data-item]")) {
      const [t, i] = r.dataset.item.split(":");
      r.classList.toggle("sel", this.app.isSelected(t, Number(i)));
    }
  }

  /** Selects an item from the list. That keeps the tab, where a selection on the canvas shows the formula. */
  pick(item) {
    if (this.app.sel.length === 1 && this.app.isSelected(item.t, item.i)) return;
    this.picking = true;
    try {
      this.app.select([item]);
    } finally {
      this.picking = false;
    }
  }

  section(title, icon, ...buttons) {
    return h("section.qsec", null, h("header", null, iconEl(icon), h("h3", null, title), h("span.grow"), ...buttons));
  }

  /** A row of the list for item `item` ({t, i}), highlighted while the item is selected. */
  row(item, onselect, ...cells) {
    const r = h("div.qrow" + (this.app.isSelected(item.t, item.i) ? ".sel" : ""), null, ...cells);
    r.dataset.item = item.t + ":" + item.i;
    r.addEventListener("pointerdown", (ev) => {
      if (!(ev.target instanceof HTMLInputElement) && !(ev.target instanceof HTMLButtonElement) && !ev.target.closest("button")) onselect();
    });
    return r;
  }

  live(el, fn) {
    el.dataset.live = "";
    el.refresh = fn;
    return el;
  }

  varsSection(doc) {
    const app = this.app;
    const sec = this.section(
      "Variables",
      "variable",
      h("button.btn.small", { type: "button", title: "A variable with a value", onclick: () => app.edit(() => doc.addVar(undefined, 1, true)) }, iconEl("plus"), "Value"),
      h("button.btn.small", { type: "button", title: "A variable defined by an expression", onclick: () => app.edit(() => doc.addDefVar(undefined, "0")) }, iconEl("plus"), "Defined"),
    );
    let any = false;
    for (let i = 0; i < doc.varCount; i++) {
      const name = doc.varName(i);
      if (name === undefined) continue;
      any = true;
      const expr = doc.varExpr(i), free = doc.varFree(i), numeric = doc.varNumeric(i);
      const select = () => this.pick({ t: "var", i });
      const nameField = field(".name", name, (v) => this.rename(name, v), { select, label: "Variable name" });
      let valueCell;
      if (expr !== "") {
        valueCell = this.live(
          field(".expr", expr, (v) => app.edit(() => doc.setVarExpr(i, v)), { check: (t) => doc.checkExpr(t), select, label: `Expression of ${name}` }),
          function () {
            this.set(doc.varExpr(i));
          },
        );
      } else if (free) {
        valueCell = this.slider(i, select);
      } else {
        valueCell = this.live(h("output.value", { title: "Solved from the constraints" }, fmt(doc.varValue(i))), function () {
          this.textContent = fmt(doc.varValue(i));
        });
      }
      const buttons = [];
      if (expr === "")
        buttons.push(toggleButton(free ? "lock" : "unlock", free ? "Set freely — click to solve it from the constraints" : "Solved from the constraints — click to set it freely", free, () => app.edit(() => doc.setVarFree(i, !free))));
      buttons.push(toggleButton("numeric", "Numeric: other formulas use its value as a plain number", numeric, () => app.edit(() => doc.setVarNumeric(i, !numeric))));
      buttons.push(iconButton("close", "Delete", () => app.edit(() => doc.deleteVar(i)), ".del"));
      sec.append(this.row({ t: "var", i: i }, select, nameField, valueCell, h("div.btns", null, ...buttons)));
    }
    if (!any) sec.append(h("p.muted.small", null, "Variables name values that dimensions and functions can use, like a or 2·a + 1."));
    return sec;
  }

  /** The value of free variable `i`: a number field and a slider that drags it. */
  slider(i, select) {
    const app = this.app, doc = app.doc;
    const num = field(".num", fmt(doc.varValue(i)), (v) => {
      const x = Number(v);
      if (Number.isFinite(x)) app.edit(() => doc.setVar(i, x));
      else app.edit(() => doc.setVarExpr(i, v));
    }, { select, label: `Value of ${doc.varName(i)}`, inputmode: "decimal" });
    const range = h("input.range", { type: "range", min: "0", max: "1000", step: "1", "aria-label": `Drag ${doc.varName(i)}` });
    let lo = 0, hi = 1, dragging = false;
    const span = () => {
      const v = doc.varValue(i);
      const s = Math.max(1, Math.abs(v));
      lo = v - s;
      hi = v + s;
    };
    const place = () => {
      range.value = String(Math.round(((doc.varValue(i) - lo) / (hi - lo)) * 1000));
    };
    span();
    place();
    range.addEventListener("pointerdown", () => {
      select();
      span();
      place();
      dragging = true;
      // the whole drag is one undo step
      doc.checkpoint();
    });
    range.addEventListener("input", () => {
      if (!dragging) {
        doc.checkpoint();
        dragging = true;
      }
      const v = lo + (Number(range.value) / 1000) * (hi - lo);
      try {
        doc.dragVar(i, Number(v.toPrecision(6)));
      } catch {}
      app.request();
    });
    const end = () => {
      if (!dragging) return;
      dragging = false;
      doc.solve();
      span();
      place();
      app.request();
    };
    range.addEventListener("change", end);
    range.addEventListener("pointerup", end);
    const cell = h("div.slider", null, num, range);
    return this.live(cell, () => {
      num.set(fmt(doc.varValue(i)));
      if (!dragging) place();
    });
  }

  functionsSection(doc) {
    const app = this.app;
    const sec = this.section("Functions", "function", h("button.btn.small", { type: "button", onclick: () => app.emit("function") }, iconEl("plus"), "Function"));
    let k = 0;
    for (let e = 0; e < doc.entCount; e++) {
      if (doc.entKind(e) !== "curve") continue;
      const src = doc.curveSrc(e);
      const select = () => this.pick({ t: "ent", i: e });
      const dot = h("span.swatch");
      dot.style.background = `var(--curve-${(k++ % 6) + 1})`;
      const f = this.live(
        field(".expr", src, (v) => app.edit(() => doc.setCurve(e, v), [e]), { check: (t) => doc.checkCurve(t), select, label: "Function" }),
        function () {
          this.set(doc.curveSrc(e));
        },
      );
      sec.append(this.row({ t: "ent", i: e }, select, dot, f, h("div.btns", null, iconButton("close", "Delete", () => app.edit(() => doc.deleteEnt(e)), ".del"))));
    }
    if (!k) sec.append(h("p.muted.small", null, "Curves y = f(x) or equations in x and y, such as y = a·sin(x) or x^2 + y^2/4 = 1."));
    return sec;
  }

  dimsSection(doc) {
    const app = this.app;
    const sec = this.section("Dimensions", "measure");
    let any = false;
    for (let i = 0; i < doc.dimCount; i++) {
      const kind = doc.dimKind(i);
      if (!kind) continue;
      any = true;
      const name = doc.dimName(i), driving = doc.dimDriving(i), numeric = doc.dimNumeric(i), angle = kind === "angle";
      const select = () => this.pick({ t: "dim", i });
      const shown = () => doc.dimExpr(i) || fmtDim(driving ? doc.dimSetValue(i) : doc.dimValue(i), angle);
      const nameField = field(".name", name, (v) => this.rename(name, v), { select, label: "Dimension name" });
      const bad = () => doc.dimConflict(i) || !!doc.dimError(i);
      const val = this.live(
        field(".expr" + (driving ? "" : ".measured"), shown(), (v) => app.setDimText(i, v.replace(/°\s*$/, "")), {
          check: (t) => (t.trim() === "" || /^-?[\d.]+(e-?\d+)?°?$/i.test(t.trim()) ? null : doc.checkExpr(t)),
          select,
          label: `Value of ${name}`,
        }),
        function () {
          this.set(shown());
          this.classList.toggle("conflict", bad());
        },
      );
      val.classList.toggle("conflict", bad());
      if (!driving) val.title = "Measured — type a value to drive it";
      const buttons = [
        toggleButton(driving ? "driving" : "measured", driving ? "Driving — click to only measure it" : "Measured — click to drive the construction with it", driving, () => app.setDriving(i, !driving)),
        toggleButton("numeric", "Numeric: other formulas use its value as a plain number", numeric, () => app.edit(() => doc.setDimNumeric(i, !numeric))),
        iconButton("close", "Delete", () => app.edit(() => doc.deleteDim(i)), ".del"),
      ];
      sec.append(this.row({ t: "dim", i: i }, select, nameField, val, h("div.btns", null, ...buttons)));
    }
    if (!any) sec.append(h("p.muted.small", null, "Dimensions you add with the Measure tool appear here. A driving dimension sets a value; a measured one reads it."));
    return sec;
  }

  consSection(doc) {
    const app = this.app;
    const sec = this.section("Constraints", "parallel");
    let any = false;
    for (let c = 0; c < doc.conCount; c++) {
      const kind = doc.conKind(c);
      if (!kind) continue;
      any = true;
      let [sym, word] = CON_LABEL[kind] ?? ["?", kind];
      if (kind === "offset" && doc.conTurned(c)) sym = "⟂";
      // conEnts is a typed array, which cannot hold the names
      const ents = Array.from(doc.conEnts(c), (e) => describe(doc, e));
      const err = doc.conConflict(c) || !!doc.conError(c);
      const select = () => this.pick({ t: "con", i: c });
      const r = this.row(
        { t: "con", i: c },
        select,
        h("span.badge" + (err ? ".bad" : ""), null, sym),
        h("span.con-text", { title: doc.conError(c) ?? "" }, `${ents[0] ?? ""} ${word} ${ents.slice(1).join(", ")}`.trim()),
        h("div.btns", null, iconButton("close", "Delete", () => app.edit(() => doc.deleteCon(c)), ".del")),
      );
      r.addEventListener("pointerenter", () => {
        app.hover = { t: "con", i: c };
        app.request();
      });
      r.addEventListener("pointerleave", () => {
        app.hover = null;
        app.request();
      });
      sec.append(r);
    }
    if (!any) sec.append(h("p.muted.small", null, "Constraints tie the construction together: select items and choose one from the chips."));
    return sec;
  }

  rename(old, name) {
    const app = this.app;
    if (!app.edit(() => app.doc.rename(old, name.trim()))) this.qsig = "";
    this.renderQuantities();
  }

  // ---------------------------------------------------------------- text

  buildText() {
    const pane = this.panes.text, app = this.app;
    this.area = h("textarea.sgp", { spellcheck: false, autocapitalize: "off", autocomplete: "off", "aria-label": "The construction as .sgp text", wrap: "off" });
    this.textMsg = h("p.small.muted", null, "The construction as text: each line states a point, curve, constraint or dimension. Edit it and apply.");
    const apply = h("button.btn.primary", { type: "button", disabled: true }, "Apply");
    const revert = h("button.btn", { type: "button", disabled: true }, "Revert");
    const copy = h("button.btn", { type: "button" }, iconEl("copy"), "Copy");
    const dirty = (on) => {
      this.dirty = on;
      apply.disabled = revert.disabled = !on;
    };
    this.area.addEventListener("input", () => dirty(true));
    apply.addEventListener("click", () => {
      try {
        app.doc.loadSgp(this.area.value);
        app.reset();
        dirty(false);
        this.textMsg.className = "small muted";
        this.textMsg.textContent = "Applied. Undo brings back the construction before.";
        this.syncText();
      } catch (e) {
        this.textMsg.className = "small error";
        this.textMsg.textContent = String(e?.message ?? e);
      }
    });
    revert.addEventListener("click", () => {
      dirty(false);
      this.syncText();
    });
    copy.addEventListener("click", () => copyText(this.area.value).then((ok) => app.toast(ok ? "Copied the text" : "Copying is not allowed here")));
    pane.append(this.textMsg, this.area, h("div.result-actions", null, apply, revert, copy));
    dirty(false);
  }

  syncText() {
    if (this.tab !== "text" || this.dirty) return;
    const t = this.app.doc.toSgp();
    if (this.area.value !== t) this.area.value = t;
  }
}
