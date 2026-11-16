// The page around the canvas: app bar and menus, the tool rail, the step hint of the active tool, the
// selection chips, the dimension value editor, the function entry, status, toasts, the welcome card, the
// help dialog, and on narrow screens the bottom sheet that holds the inspector.

import { RAIL, TOOLS } from "./tools.js";
import { actionsFor } from "./actions.js";
import { icon, iconEl } from "./icons.js";
import { copyText, fmt, h, isMac, load, store } from "./util.js";

const narrow = matchMedia("(max-width: 760px)");
const mod = isMac ? "⌘" : "Ctrl+";

// ------------------------------------------------------------------ menus

let openMenu = null;
let openSub = null;

function closeSub() {
  openSub?.remove();
  openSub = null;
}

function closeMenu() {
  closeSub();
  openMenu?.remove();
  openMenu = null;
}

/** The elements of menu items (see `menu`) in `el`. */
function fill(el, items) {
  for (const it of items) {
    if (it === "-") {
      el.append(h("hr"));
      continue;
    }
    if (it.heading) {
      el.append(h("div.menu-heading", null, it.heading));
      continue;
    }
    const b = h(
      "button.menu-item",
      {
        type: "button",
        role: it.checked === undefined ? "menuitem" : "menuitemcheckbox",
        "aria-checked": it.checked === undefined ? undefined : String(it.checked),
        "aria-haspopup": it.submenu ? "menu" : undefined,
        disabled: it.disabled,
        onclick: () => {
          if (it.submenu) return openSubmenu(el, b, it);
          closeMenu();
          it.run();
        },
      },
      it.icon ? iconEl(it.icon) : it.checked !== undefined ? iconEl("check", it.checked ? "" : "invisible") : h("span.ic"),
      h("span.grow", null, it.label),
      it.keys ? h("kbd", null, it.keys) : null,
      it.submenu ? iconEl("chevronRight", "sub") : null,
    );
    // with a mouse, resting on a submenu item opens it; resting on another item closes the open one
    b.addEventListener("pointerenter", (ev) => {
      if (ev.pointerType !== "mouse" || el !== openMenu) return;
      if (it.submenu) openSubmenu(el, b, it);
      else closeSub();
    });
    el.append(b);
  }
}

/**
 * Opens a menu under `anchor`: items are {label, icon, keys, run, checked, disabled}, {label, icon,
 * submenu} where `submenu()` gives the items (or a promise of them) of a menu that opens beside it, "-" for a
 * separator, or {heading}.
 */
export function menu(anchor, items) {
  closeMenu();
  const el = h("div.menu", { role: "menu" });
  fill(el, items);
  document.body.append(el);
  const r = anchor.getBoundingClientRect();
  const w = el.offsetWidth, hh = el.offsetHeight;
  let x = Math.min(r.left, innerWidth - w - 8), y = r.bottom + 4;
  if (y + hh > innerHeight - 8) y = Math.max(8, r.top - hh - 4);
  el.style.left = Math.max(8, x) + "px";
  el.style.top = y + "px";
  openMenu = el;
  el.querySelector("button:not([disabled])")?.focus({ preventScroll: true });
  return el;
}

/** Opens the submenu of item `it` (button `b` of menu `parent`): beside the menu where there is room, and
 *  else in its place, with a way back. */
async function openSubmenu(parent, b, it) {
  if (openSub?.owner === b) return;
  closeSub();
  const items = await it.submenu();
  if (openMenu !== parent) return;
  const el = h("div.menu.submenu", { role: "menu" });
  el.owner = b;
  fill(el, items);
  document.body.append(el);
  const pr = parent.getBoundingClientRect(), br = b.getBoundingClientRect();
  const w = el.offsetWidth, hh = el.offsetHeight;
  const y = Math.max(8, Math.min(br.top - 5, innerHeight - hh - 8));
  if (pr.right + w - 4 <= innerWidth - 8) el.style.left = pr.right - 4 + "px";
  else if (pr.left - w + 4 >= 8) el.style.left = pr.left - w + 4 + "px";
  else {
    // no room beside the menu, as on a phone: the submenu takes its place
    el.prepend(
      h("button.menu-item.back", { type: "button", onclick: () => (closeSub(), (parent.hidden = false), b.focus()) }, iconEl("chevronLeft"), h("span.grow", null, it.label)),
      h("hr"),
    );
    parent.hidden = true;
    el.style.left = Math.max(8, Math.min(pr.left, innerWidth - w - 8)) + "px";
    el.style.top = pr.top + "px";
    openSub = el;
    el.querySelector(".menu-item:not(.back)")?.focus({ preventScroll: true });
    return;
  }
  el.style.top = y + "px";
  openSub = el;
  if (document.activeElement === b) el.querySelector("button:not([disabled])")?.focus({ preventScroll: true });
}

document.addEventListener("pointerdown", (ev) => {
  if (!openMenu) return;
  if (openSub?.contains(ev.target) || openMenu.contains(ev.target) || ev.target.closest?.("[data-menu-anchor]")) return;
  closeMenu();
});
document.addEventListener(
  "keydown",
  (ev) => {
    if (!openMenu) return;
    const active = openSub?.contains(document.activeElement) ? openSub : openMenu;
    if (ev.key === "Escape") {
      if (active === openSub) {
        const owner = openSub.owner;
        closeSub();
        openMenu.hidden = false;
        owner.focus();
      } else closeMenu();
      ev.stopPropagation();
    }
    if (ev.key === "ArrowDown" || ev.key === "ArrowUp") {
      const items = [...active.querySelectorAll("button:not([disabled])")];
      const k = items.indexOf(document.activeElement);
      items[(k + (ev.key === "ArrowDown" ? 1 : items.length - 1)) % items.length]?.focus();
      ev.preventDefault();
    }
    if (ev.key === "ArrowRight" && active === openMenu && document.activeElement?.getAttribute("aria-haspopup")) document.activeElement.click();
    if (ev.key === "ArrowLeft" && active === openSub && !openMenu.hidden) {
      const owner = openSub.owner;
      closeSub();
      owner.focus();
    }
  },
  true,
);

// ------------------------------------------------------------------ the shell

export class Shell {
  constructor(app, files) {
    this.app = app;
    this.files = files;
    this.$ = (s) => document.querySelector(s);
    this.stage = this.$("#stage");
    this.groupLast = {};
    this.buildBar();
    this.buildRail();
    this.buildHint();
    this.buildFunctionBar();
    this.buildDimEditor();
    this.buildStatus();
    this.buildWelcome();
    this.buildSheet();
    this.applyTheme(load("theme", "system"));

    this.actions = this.$("#actions");
    app.on("sel", () => this.updateActions());
    app.on("doc", () => {
      this.updateActions();
      this.updateStatus();
      this.updateWelcome();
    });
    app.on("tool", () => {
      this.updateRail();
      this.updateHint();
      this.updateActions();
      this.updateWelcome();
    });
    app.on("frame", () => {
      this.placeActions();
      this.placeDimEditor();
    });
    app.on("toast", (t) => this.toast(t.text, t.kind, t.action));
    app.on("name", () => {
      const n = this.$("#doc-name");
      if (document.activeElement !== n) n.value = app.name;
      document.title = app.name === "Untitled" ? "sgweb" : `${app.name} — sgweb`;
    });
    app.on("edit", () => this.openDimEditor());
    app.on("function", () => this.startFunction());
    narrow.addEventListener("change", () => this.layout());
    new ResizeObserver(() => this.layout()).observe(this.stage);
    this.layout();
  }

  /** Commands that keys and menus share. */
  get commands() {
    return {
      save: (ask) => this.files.save(ask),
      open: () => this.files.pick(),
      help: () => this.help(),
      function: () => this.startFunction(),
    };
  }

  // ---------------------------------------------------------------- bar and menus

  buildBar() {
    const app = this.app, files = this.files;
    const name = this.$("#doc-name");
    name.value = app.name;
    name.addEventListener("change", () => app.setName(name.value));
    name.addEventListener("keydown", (ev) => ev.key === "Enter" && name.blur());
    this.$("#undo").addEventListener("click", () => app.undo());
    this.$("#redo").addEventListener("click", () => app.redo());
    this.$("#help-btn").addEventListener("click", () => this.help());
    const file = () => [
      { label: "New", icon: "file", run: () => (app.clear(), files.handle = null) },
      { label: "Open…", icon: "folder", keys: mod + "O", run: () => files.pick() },
      { label: "Save", icon: "save", keys: mod + "S", run: () => files.save() },
      { label: "Save as…", keys: (isMac ? "⇧⌘" : "Ctrl+Shift+") + "S", run: () => files.save(true) },
      "-",
      { label: "Copy link to this construction", icon: "link", run: () => this.shareLink() },
      { label: "Export picture (PNG)", icon: "image", run: () => files.exportImage() },
      "-",
      { label: "Examples", icon: "folder", submenu: () => examples() },
    ];
    const examples = async () => {
      const list = await files.examples();
      return list.length ? list.map((x) => ({ label: x.title, run: () => files.openExample(x.path, x.title) })) : [{ label: "No examples", disabled: true, run() {} }];
    };
    const view = () => [
      { label: "Fit the construction", icon: "fit", keys: "Home", run: () => app.fit() },
      { label: "Zoom in", icon: "zoomIn", keys: "+", run: () => this.zoom(1.25) },
      { label: "Zoom out", icon: "zoomOut", keys: "−", run: () => this.zoom(0.8) },
      "-",
      { label: "Constraint badges", checked: app.showConstraints, run: () => app.setOption("showConstraints", !app.showConstraints) },
      { label: "Point names", checked: app.showLabels, run: () => app.setOption("showLabels", !app.showLabels) },
      "-",
      { heading: "Theme" },
      ...["system", "light", "dark"].map((t) => ({ label: t[0].toUpperCase() + t.slice(1), checked: this.theme === t, run: () => this.applyTheme(t) })),
    ];
    const bind = (sel, items) => {
      const b = this.$(sel);
      b.dataset.menuAnchor = "";
      b.addEventListener("click", async () => (openMenu && openMenu.anchor === b ? closeMenu() : (menu(b, await items()).anchor = b)));
    };
    bind("#menu-file", file);
    bind("#menu-view", view);
    bind("#menu-more", async () => [
      ...file(),
      "-",
      ...view(),
      "-",
      { label: "Help", icon: "help", run: () => this.help() },
    ]);
    app.on("doc", () => {
      this.$("#undo").disabled = false;
    });
  }

  zoom(f) {
    const v = this.app.view;
    v.zoomAt(v.w / 2, v.h / 2, f);
    this.app.request();
  }

  async shareLink() {
    const url = await this.files.link();
    const ok = await copyText(url);
    this.toast(ok ? "Copied a link that opens this construction" : url, ok ? "info" : "info");
  }

  applyTheme(t) {
    this.theme = t;
    store("theme", t);
    const root = document.documentElement;
    if (t === "system") delete root.dataset.theme;
    else root.dataset.theme = t;
    this.app.restyle();
    this.app.emit("options");
    if (!this.watchingScheme) {
      this.watchingScheme = true;
      matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
        if (this.theme === "system") {
          this.app.restyle();
          this.app.emit("options");
        }
      });
    }
  }

  // ---------------------------------------------------------------- tool rail

  buildRail() {
    const rail = this.$("#rail");
    this.railButtons = [];
    for (const entry of RAIL) {
      if (entry.tool) {
        const t = TOOLS[entry.tool];
        const b = h(
          "button.tool",
          { type: "button", title: `${t.label}${t.key ? "  (" + t.key.toUpperCase() + ")" : ""}`, onclick: () => this.pickTool(t.id) },
          iconEl(t.icon),
          h("span", null, t.id === "function" ? "Function" : t.label),
        );
        b.dataset.tools = t.id;
        rail.append(b);
        this.railButtons.push({ b, tools: [t.id] });
      } else {
        const b = h("button.tool.group", { type: "button", title: entry.group, "aria-haspopup": "menu" }, iconEl(TOOLS[entry.tools[0]].icon), h("span", null, entry.group));
        b.dataset.menuAnchor = "";
        b.addEventListener("click", () => this.groupMenu(b, entry));
        rail.append(b);
        this.railButtons.push({ b, tools: entry.tools, group: entry.group });
      }
    }
    this.updateRail();
  }

  groupMenu(b, entry) {
    if (openMenu?.anchor === b) return closeMenu();
    const m = menu(
      b,
      entry.tools.map((id) => {
        const t = TOOLS[id];
        return { label: t.label, icon: t.icon, keys: t.key?.toUpperCase(), run: () => this.pickTool(id) };
      }),
    );
    m.anchor = b;
    m.classList.add("tool-menu");
    // beside the rail on wide screens, above the dock on narrow ones
    const r = b.getBoundingClientRect();
    if (narrow.matches) {
      m.style.top = Math.max(8, r.top - m.offsetHeight - 6) + "px";
      m.style.left = Math.min(Math.max(8, r.left), innerWidth - m.offsetWidth - 8) + "px";
    } else {
      m.style.left = r.right + 6 + "px";
      m.style.top = Math.min(r.top, innerHeight - m.offsetHeight - 8) + "px";
    }
  }

  pickTool(id) {
    if (id === "function") return this.startFunction();
    this.app.setTool(id);
  }

  updateRail() {
    const tool = this.app.tool;
    for (const { b, tools, group } of this.railButtons) {
      const on = tools.includes(tool);
      b.classList.toggle("on", on);
      b.setAttribute("aria-pressed", String(on));
      if (group) {
        if (on) this.groupLast[group] = tool;
        const show = TOOLS[this.groupLast[group] ?? tools[0]];
        b.querySelector("svg").outerHTML = icon(show.icon);
        b.title = `${group}: ${show.label}…`;
      }
    }
  }

  // ---------------------------------------------------------------- tool hint

  buildHint() {
    this.hintEl = this.$("#hint");
    this.hintText = this.hintEl.querySelector(".text");
    this.hintEl.querySelector(".done").addEventListener("click", () => this.app.setTool("select"));
  }

  updateHint() {
    const app = this.app;
    const on = app.tool !== "select" && app.tool !== "function";
    this.hintEl.hidden = !on;
    if (!on) return;
    const t = TOOLS[app.tool];
    this.hintEl.querySelector(".tool-name").replaceChildren(iconEl(t.icon), t.label);
    this.hintText.textContent = app.hint();
  }

  // ---------------------------------------------------------------- functions

  buildFunctionBar() {
    const bar = this.$("#fnbar"), input = bar.querySelector("input"), msg = bar.querySelector(".msg");
    const check = () => {
      const err = input.value.trim() ? this.app.doc.checkCurve(input.value) : null;
      input.classList.toggle("bad", !!err);
      msg.textContent = err ?? "";
    };
    input.addEventListener("input", check);
    input.addEventListener("keydown", (ev) => {
      if (ev.key === "Enter") add();
      if (ev.key === "Escape") this.app.setTool("select");
    });
    const add = () => {
      if (this.app.addFunction(input.value)) bar.hidden = true;
    };
    bar.querySelector(".add").addEventListener("click", add);
    bar.querySelector(".cancel").addEventListener("click", () => this.app.setTool("select"));
    this.app.on("tool", () => {
      if (this.app.tool !== "function") bar.hidden = true;
    });
    this.fnInput = input;
    this.fnCheck = check;
  }

  startFunction() {
    this.app.setTool("function");
    const bar = this.$("#fnbar");
    bar.hidden = false;
    this.fnInput.value = "y = ";
    this.fnCheck();
    this.fnInput.focus();
    this.fnInput.setSelectionRange(4, 4);
    this.updateRail();
  }

  // ---------------------------------------------------------------- selection chips

  updateActions() {
    const app = this.app;
    let acts = [];
    try {
      if (app.tool === "select" && !app.drag) acts = actionsFor(app);
    } catch {
      // a selection the document no longer has: no actions until it is pruned
    }
    this.actions.replaceChildren(
      ...acts.map((a) => h("button.chip-btn" + (a.danger ? ".danger" : ""), { type: "button", title: a.label, onclick: () => a.run() }, iconEl(a.icon), h("span", null, a.label))),
    );
    this.actions.hidden = acts.length === 0;
    this.placeActions();
  }

  /** On wide screens the chips float above the selection; on narrow ones they sit in a strip. */
  placeActions() {
    const el = this.actions;
    if (el.hidden || narrow.matches) {
      el.style.left = el.style.top = "";
      return;
    }
    const app = this.app, sc = app.scene;
    let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
    const add = (x, y) => {
      x0 = Math.min(x0, x);
      y0 = Math.min(y0, y);
      x1 = Math.max(x1, x);
      y1 = Math.max(y1, y);
    };
    for (const s of app.sel) {
      if (s.t === "ent") for (const pl of sc.shapes.get(s.i) ?? []) for (let k = 0; k < pl.length; k += 2) add(Math.max(0, Math.min(app.view.w, pl[k])), Math.max(0, Math.min(app.view.h, pl[k + 1])));
      if (s.t === "dim") {
        const l = sc.labels.find((l) => l.d === s.i);
        if (l) (add(l.x, l.y), add(l.x + l.w, l.y + l.h));
      }
      if (s.t === "con") {
        const m = sc.markers.find((m) => m.c === s.i);
        if (m) add(m.x, m.y);
      }
    }
    const w = el.offsetWidth, hh = el.offsetHeight, W = app.view.w, H = app.view.h;
    if (!Number.isFinite(x0)) {
      x0 = W / 2;
      x1 = W / 2;
      y0 = 80;
      y1 = 80;
    }
    let x = (x0 + x1) / 2 - w / 2, y = y0 - hh - 16;
    if (y < 56) y = y1 + 18;
    // a selection that fills most of the canvas leaves no good spot beside it: the top edge then
    if (y + hh > H - 40 || y1 - y0 > H * 0.6) {
      x = W / 2 - w / 2;
      y = 12;
    }
    el.style.left = Math.round(Math.max(8, Math.min(W - w - 8, x))) + "px";
    el.style.top = Math.round(Math.max(8, Math.min(H - hh - 8, y))) + "px";
  }

  // ---------------------------------------------------------------- dimension editor

  buildDimEditor() {
    const ed = this.$("#dim-editor");
    this.dimEd = ed;
    const input = ed.querySelector("input");
    const commit = () => {
      const i = this.app.editing;
      if (i === null) return;
      this.app.editing = null;
      ed.hidden = true;
      if (input.value.trim() !== input.dataset.start) this.app.setDimText(i, input.value.trim().replace(/°$/, ""));
      this.app.request();
    };
    input.addEventListener("keydown", (ev) => {
      if (ev.key === "Enter") commit();
      if (ev.key === "Escape") {
        this.app.editing = null;
        ed.hidden = true;
        this.app.request();
        ev.stopPropagation();
      }
    });
    input.addEventListener("input", () => {
      const t = input.value.trim();
      input.classList.toggle("bad", !!t && !/^-?[\d.]+(e-?\d+)?°?$/i.test(t) && !!this.app.doc.checkExpr(t));
    });
    input.addEventListener("blur", commit);
    ed.querySelector(".ok").addEventListener("pointerdown", (ev) => {
      ev.preventDefault();
      commit();
    });
    this.dimInput = input;
  }

  openDimEditor() {
    const app = this.app, i = app.editing;
    if (i === null) {
      this.dimEd.hidden = true;
      return;
    }
    const kind = app.doc.dimKind(i);
    this.dimEd.querySelector(".unit").textContent = kind === "angle" ? "°" : "";
    this.dimEd.querySelector(".label").replaceChildren(app.doc.dimName(i) ?? "", " =");
    this.dimInput.value = app.dimEditText(i);
    this.dimInput.dataset.start = this.dimInput.value;
    this.dimInput.classList.remove("bad");
    this.dimEd.hidden = false;
    this.placeDimEditor();
    this.dimInput.focus({ preventScroll: true });
    this.dimInput.select();
  }

  placeDimEditor() {
    const app = this.app, i = app.editing;
    if (i === null || this.dimEd.hidden) return;
    const a = app.doc.dimAnchor(i), off = app.doc.dimOffset(i);
    if (!a || !off) return;
    const [x, y] = app.view.toScreen([a[0] + off[0], a[1] + off[1]]);
    const w = this.dimEd.offsetWidth, hh = this.dimEd.offsetHeight;
    this.dimEd.style.left = Math.round(Math.max(4, Math.min(app.view.w - w - 4, x - w / 2))) + "px";
    this.dimEd.style.top = Math.round(Math.max(4, Math.min(app.view.h - hh - 4, y - hh / 2))) + "px";
  }

  // ---------------------------------------------------------------- status

  buildStatus() {
    this.dof = this.$("#dof");
    this.coords = this.$("#coords");
    this.stage.addEventListener("pointermove", () => {
      const p = this.app.pointer;
      if (!p) return;
      const w = this.app.view.toWorld(p.x, p.y);
      this.coords.textContent = `${fmt(w[0])}, ${fmt(w[1])}`;
    });
    this.$("#zoom-in").addEventListener("click", () => this.zoom(1.25));
    this.$("#zoom-out").addEventListener("click", () => this.zoom(0.8));
    this.$("#zoom-fit").addEventListener("click", () => this.app.fit());
  }

  updateStatus() {
    const d = this.app.doc;
    const dof = d.dof(), conflict = d.conflictCount > 0;
    this.dof.className = "dof " + (conflict ? "bad" : dof === 0 ? "good" : "free");
    this.dof.textContent = conflict ? "Conflict" : dof === 0 ? "Fully determined" : `${dof} degree${dof === 1 ? "" : "s"} of freedom`;
    this.dof.title = conflict
      ? "Some constraints or dimensions cannot all hold; they are shown in red"
      : dof === 0
        ? "Constraints and dimensions fix every point and curve"
        : "How many more values could still move: add dimensions or constraints to fix them";
  }

  // ---------------------------------------------------------------- welcome

  buildWelcome() {
    this.welcome = this.$("#welcome");
    this.welcome.querySelector(".open").addEventListener("click", () => this.files.pick());
    this.welcome.querySelector(".draw").addEventListener("click", () => this.app.setTool("line"));
    this.welcome.querySelector(".close").addEventListener("click", () => {
      this.welcomeClosed = true;
      this.updateWelcome();
    });
    this.files.examples().then((list) => {
      const ul = this.welcome.querySelector(".examples");
      ul.replaceChildren(...list.slice(0, 6).map((x) => h("li", null, h("button.link", { type: "button", onclick: () => this.files.openExample(x.path, x.title) }, x.title))));
    });
  }

  updateWelcome() {
    const app = this.app;
    this.welcome.hidden = this.welcomeClosed || app.tool !== "select" || !app.isEmpty();
  }

  // ---------------------------------------------------------------- toasts and help

  toast(text, kind = "info", action) {
    const box = this.$("#toasts");
    if (kind === "hint" && box.querySelector(".toast.hint")) return;
    const t = h("div.toast." + kind, { role: kind === "error" ? "alert" : "status" }, kind === "error" ? iconEl("warning") : null, h("span", null, text));
    const out = () => {
      t.classList.add("out");
      setTimeout(() => t.remove(), 300);
    };
    if (action) {
      t.classList.add("has-action");
      t.append(
        h("button.toast-action", { type: "button", onclick: () => (out(), action.run()) }, action.label),
        h("button.toast-close", { type: "button", "aria-label": "Dismiss", onclick: out }, iconEl("close")),
      );
    }
    box.append(t);
    while (box.children.length > 3) box.firstElementChild.remove();
    if (!action) setTimeout(out, kind === "error" ? 4500 : 2600);
  }

  help() {
    const d = this.$("#help");
    if (!d.dataset.built) {
      d.dataset.built = "1";
      const keys = [
        ["Tools", Object.values(TOOLS).filter((t) => t.key).map((t) => [t.key.toUpperCase(), t.label])],
        [
          "Editing",
          [
            [`${mod}Z`, "Undo"],
            [isMac ? "⇧⌘Z" : "Ctrl+Y", "Redo"],
            ["Del", "Delete the selection"],
            ["Enter", "Edit the selected dimension"],
            ["H / V", "Make the selection horizontal / vertical"],
            ["T", "Make two curves tangent"],
            ["X", "Toggle construction lines"],
            ["Esc", "Step back: cancel the tool, then deselect"],
            [`${mod}A`, "Select everything"],
          ],
        ],
        [
          "View",
          [
            ["Home", "Fit the construction"],
            ["Wheel / pinch", "Zoom"],
            ["Space + drag, right drag", "Pan"],
          ],
        ],
      ];
      const body = d.querySelector(".help-keys");
      for (const [title, rows] of keys) {
        body.append(h("h3", null, title), h("dl", null, ...rows.flatMap(([k, v]) => [h("dt", null, h("kbd", null, k)), h("dd", null, v)])));
      }
      d.querySelector(".close").addEventListener("click", () => d.close());
      d.addEventListener("click", (ev) => ev.target === d && d.close());
    }
    d.showModal();
  }

  // ---------------------------------------------------------------- bottom sheet

  /**
   * The inspector as a sheet on narrow screens: lowered (only its tabs show), half or fully raised. Its top
   * (handle and tabs) drags it; a drag moves it with a transform, without layout, and on release it settles
   * on the state nearest to where it was let go, or the next one in the direction of a flick.
   */
  buildSheet() {
    const insp = this.$("#inspector"), handle = insp.querySelector(".sheet-handle"), tabs = insp.querySelector(".tabs");
    const work = this.$(".work"), rail = this.$("#rail");
    const order = ["peek", "half", "full"];
    this.sheetState = "peek";
    insp.dataset.sheet = "peek";
    const heights = () => {
      const w = work.clientHeight, d = rail.offsetHeight;
      const peek = parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--peek")) || 60;
      return { peek, half: Math.round(w * 0.5), full: w - d - 12 };
    };
    let settling = 0;
    const rest = (state) => {
      clearTimeout(settling);
      insp.style.transition = insp.style.transform = insp.style.height = "";
      insp.dataset.sheet = state;
      this.sheetState = state;
    };
    // shows the sheet at its full height, moved down so that `h` pixels of it show
    const hold = (hs, h) => {
      insp.style.transition = "none";
      insp.style.height = hs.full + "px";
      insp.dataset.sheet = "full";
      insp.style.transform = `translateY(${hs.full - h}px)`;
    };
    const settle = (state, hs) => {
      insp.style.transition = "transform 0.2s cubic-bezier(0.2, 0.8, 0.2, 1)";
      insp.style.transform = `translateY(${hs.full - hs[state]}px)`;
      clearTimeout(settling);
      settling = setTimeout(() => rest(state), 210);
    };
    const set = (state) => {
      if (!narrow.matches || state === this.sheetState) return rest(state);
      const hs = heights();
      hold(hs, insp.getBoundingClientRect().height);
      insp.getBoundingClientRect();
      this.sheetState = state;
      settle(state, hs);
    };

    let drag = null, frame = 0, swallowClick = false;
    const down = (ev) => {
      if (!narrow.matches || drag) return;
      drag = { id: ev.pointerId, el: ev.currentTarget, y0: ev.clientY, y: ev.clientY, h0: insp.getBoundingClientRect().height, moved: false, samples: [[ev.clientY, ev.timeStamp]] };
    };
    const place = () => {
      frame = 0;
      if (!drag?.moved) return;
      const hs = drag.hs;
      drag.h = Math.max(hs.peek, Math.min(hs.full, drag.h0 + drag.y0 - drag.y));
      insp.style.transform = `translateY(${hs.full - drag.h}px)`;
    };
    const move = (ev) => {
      if (!drag || ev.pointerId !== drag.id) return;
      drag.y = ev.clientY;
      drag.samples.push([ev.clientY, ev.timeStamp]);
      if (drag.samples.length > 6) drag.samples.shift();
      if (!drag.moved && Math.abs(drag.y - drag.y0) > 6) {
        drag.moved = true;
        drag.el.setPointerCapture(drag.id);
        clearTimeout(settling);
        drag.hs = heights();
        hold(drag.hs, drag.h0);
      }
      if (drag.moved && !frame) frame = requestAnimationFrame(place);
    };
    const up = (ev) => {
      if (!drag || ev.pointerId !== drag.id) return;
      const d = drag;
      drag = null;
      if (!d.moved) {
        // a tap on the handle raises or lowers the sheet; a tap on a tab is the tab's
        if (d.el === handle) set(this.sheetState === "peek" ? "half" : "peek");
        return;
      }
      if (frame) cancelAnimationFrame(frame);
      frame = 0;
      swallowClick = d.el === tabs;
      const [first, last] = [d.samples[0], d.samples.at(-1)];
      const v = (last[0] - first[0]) / Math.max(1, last[1] - first[1]);
      const hs = d.hs, h = Math.max(hs.peek, Math.min(hs.full, d.h0 + d.y0 - d.y));
      let state = order.reduce((a, b) => (Math.abs(hs[b] - h) < Math.abs(hs[a] - h) ? b : a));
      if (Math.abs(v) > 0.4) {
        // a flick goes on to the next state that way
        const ups = order.filter((s) => hs[s] > h + 4), downs = order.filter((s) => hs[s] < h - 4);
        state = v < 0 ? (ups[0] ?? "full") : (downs.at(-1) ?? "peek");
      }
      this.sheetState = state;
      settle(state, hs);
    };
    for (const el of [handle, tabs]) {
      el.addEventListener("pointerdown", down);
      el.addEventListener("pointermove", move);
      el.addEventListener("pointerup", up);
      el.addEventListener("pointercancel", up);
    }
    tabs.addEventListener(
      "click",
      (ev) => {
        if (swallowClick) {
          swallowClick = false;
          ev.stopPropagation();
          ev.preventDefault();
        }
      },
      true,
    );
    this.app.on("tab", () => {
      if (narrow.matches && this.sheetState === "peek") set("half");
    });
    // selecting a measurement on a phone brings its formula up
    let target = null;
    this.app.on("sel", () => {
      const t = this.app.formulaTarget();
      const key = t ? t.kind + t.name : null;
      if (narrow.matches && key && key !== target && this.sheetState === "peek") set("half");
      target = key;
    });
    this.app.on("tool", () => {
      if (narrow.matches && this.app.tool !== "select" && this.sheetState !== "peek") set("peek");
    });
  }

  /** Recomputes which parts of the canvas the panels cover and resizes it. */
  layout() {
    const app = this.app;
    const r = this.stage.getBoundingClientRect();
    const ins = { left: 0, top: 0, right: 0, bottom: 0 };
    app.insets = ins;
    app.resize(r.width, r.height);
    this.placeActions();
  }
}
