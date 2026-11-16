// Pointer, touch and keyboard input on the canvas.
//
// Mouse: click selects (Shift/Ctrl adds), drag moves what is under it or draws a selection box, the wheel
// zooms, the middle or right button (or Space) pans, right click steps back.
// Touch: tap selects and adds to the selection (tap empty space to clear), drag moves what is under the
// finger or pans, two fingers pinch and pan. With a drawing tool the preview follows the finger and the
// point goes where it is lifted.

import { TOOLS } from "./tools.js";

const DOUBLE_MS = 350;

export function attachInput(app, el, commands) {
  const pointers = new Map();
  let press = null;
  let pinch = null;
  let lastTap = { t: 0, x: 0, y: 0 };
  let space = false;

  const local = (ev) => {
    const r = el.getBoundingClientRect();
    return [ev.clientX - r.left, ev.clientY - r.top];
  };
  const world = (x, y) => app.view.toWorld(x, y);

  function hoverAt(x, y) {
    const r = app.hitRadius();
    const h = app.tool === "select" ? app.scene.hit(x, y, r) : app.scene.hit(x, y, r, { markers: false });
    // only what the tool can use lights up under a drawing tool
    app.hover = h && (app.tool === "select" || h.t === "ent" || (app.tool === "measure" && h.t === "dim")) ? h : null;
    el.style.cursor = cursorFor(h);
  }

  function cursorFor(h) {
    if (space || press?.mode === "pan") return "grabbing";
    if (press?.mode === "drag") return "grabbing";
    if (app.tool !== "select") return "crosshair";
    if (h?.t === "handle") return "grab";
    return h ? "pointer" : "default";
  }

  function pinchState() {
    const [a, b] = [...pointers.values()];
    return { d: Math.hypot(a.x - b.x, a.y - b.y), mx: (a.x + b.x) / 2, my: (a.y + b.y) / 2 };
  }

  el.addEventListener("pointerdown", (ev) => {
    const [x, y] = local(ev);
    el.setPointerCapture(ev.pointerId);
    pointers.set(ev.pointerId, { x, y });
    if (pointers.size === 2) {
      // a second finger turns whatever the first one did into a pinch
      if (press?.mode === "drag") app.endDrag();
      app.marquee = null;
      press = null;
      pinch = pinchState();
      app.pointer = null;
      app.hover = null;
      app.request();
      return;
    }
    if (pointers.size > 2) return;
    app.pointer = { x, y, type: ev.pointerType, live: true };
    press = { id: ev.pointerId, x0: x, y0: y, lx: x, ly: y, type: ev.pointerType, button: ev.button, moved: false, mode: "pending" };
    if (ev.button === 1 || ev.button === 2 || space) press.mode = "pan";
    else if (app.tool === "select") press.hit = app.scene.hit(x, y, app.hitRadius());
    hoverAt(x, y);
    app.request();
  });

  el.addEventListener("pointermove", (ev) => {
    const [x, y] = local(ev);
    if (pointers.has(ev.pointerId)) pointers.set(ev.pointerId, { x, y });
    if (pinch) {
      if (pointers.size >= 2) {
        const p = pinchState();
        if (pinch.d > 0 && p.d > 0) app.view.zoomAt(p.mx, p.my, p.d / pinch.d);
        app.view.panBy(p.mx - pinch.mx, p.my - pinch.my);
        pinch = p;
        app.request();
      }
      return;
    }
    if (!press) {
      if (ev.pointerType === "mouse") {
        app.pointer = { x, y, type: "mouse", live: true };
        hoverAt(x, y);
        app.request();
      }
      return;
    }
    if (ev.pointerId !== press.id) return;
    app.pointer = { x, y, type: press.type, live: true };
    const slop = press.type === "mouse" ? 4 : 9;
    if (!press.moved && Math.hypot(x - press.x0, y - press.y0) > slop) {
      press.moved = true;
      begin(press);
    }
    if (press.moved) {
      switch (press.mode) {
        case "drag":
          app.dragTo(world(x, y));
          break;
        case "marquee":
          app.marquee = [press.x0, press.y0, x, y];
          break;
        case "pan":
          app.view.panBy(x - press.lx, y - press.ly);
          break;
      }
    }
    if (press.mode !== "drag" && press.mode !== "pan") hoverAt(x, y);
    press.lx = x;
    press.ly = y;
    app.request();
  });

  function begin(p) {
    if (p.mode !== "pending") return;
    if (app.tool === "select") {
      if (p.hit && app.startDrag(p.hit, world(p.x0, p.y0))) p.mode = "drag";
      else p.mode = p.type === "mouse" ? "marquee" : "pan";
    } else {
      // a finger aims the tool and places on release; a mouse pans
      p.mode = p.type === "mouse" ? "pan" : "aim";
    }
    el.style.cursor = cursorFor(null);
  }

  function finish(ev, cancelled) {
    pointers.delete(ev.pointerId);
    if (pinch) {
      if (pointers.size < 2) pinch = null;
      return;
    }
    if (!press || ev.pointerId !== press.id) return;
    const p = press;
    press = null;
    const [x, y] = local(ev);
    switch (p.mode) {
      case "drag":
        app.endDrag();
        break;
      case "marquee": {
        app.marquee = null;
        const ents = app.scene.inside(Math.min(p.x0, x), Math.min(p.y0, y), Math.max(p.x0, x), Math.max(p.y0, y));
        const dims = app.scene.labels
          .filter((l) => l.x >= Math.min(p.x0, x) && l.x + l.w <= Math.max(p.x0, x) && l.y >= Math.min(p.y0, y) && l.y + l.h <= Math.max(p.y0, y))
          .map((l) => ({ t: "dim", i: l.d }));
        const found = [...ents.map((e) => ({ t: "ent", i: e })), ...dims];
        const base = ev.shiftKey || ev.ctrlKey || ev.metaKey ? app.sel : [];
        app.select([...base, ...found.filter((f) => !base.some((s) => s.t === f.t && s.i === f.i))]);
        break;
      }
      case "aim":
        if (!cancelled) app.toolClickAt(x, y);
        break;
      case "pending":
        if (cancelled) break;
        if (p.button === 2) app.back();
        else if (app.tool === "select") selectClick(p, ev, x, y);
        else app.toolClickAt(x, y);
        break;
    }
    if (p.type !== "mouse") {
      app.pointer = null;
      app.hover = null;
    } else hoverAt(x, y);
    el.style.cursor = cursorFor(app.hover);
    app.request();
  }

  function selectClick(p, ev, x, y) {
    const h = p.hit;
    const now = performance.now();
    const double = now - lastTap.t < DOUBLE_MS && Math.hypot(x - lastTap.x, y - lastTap.y) < 14;
    lastTap = { t: now, x, y };
    if (double && h?.t === "dim") {
      app.select([{ t: "dim", i: h.i }]);
      app.editDim(h.i);
      return;
    }
    const additive = ev.shiftKey || ev.ctrlKey || ev.metaKey || p.type !== "mouse";
    if (!h) {
      if (!additive || p.type !== "mouse") app.select([]);
      return;
    }
    const item = h.t === "handle" ? { t: "ent", i: h.e } : { t: h.t, i: h.i };
    if (additive) app.toggle(item);
    else app.select([item]);
  }

  el.addEventListener("pointerup", (ev) => finish(ev, false));
  el.addEventListener("pointercancel", (ev) => finish(ev, true));
  el.addEventListener("pointerleave", (ev) => {
    if (ev.pointerType === "mouse" && !press) {
      app.pointer = null;
      app.hover = null;
      app.request();
    }
  });
  el.addEventListener("contextmenu", (ev) => ev.preventDefault());
  el.addEventListener(
    "wheel",
    (ev) => {
      ev.preventDefault();
      const [x, y] = local(ev);
      const unit = ev.deltaMode === 1 ? 16 : ev.deltaMode === 2 ? 400 : 1;
      if (ev.ctrlKey || !ev.shiftKey) {
        // trackpad pinches arrive as wheel events with Ctrl
        const f = Math.exp(-ev.deltaY * unit * (ev.ctrlKey ? 0.01 : 0.0018));
        app.view.zoomAt(x, y, f);
        if (!ev.ctrlKey && ev.deltaX) app.view.panBy(-ev.deltaX * unit, 0);
      } else app.view.panBy(-ev.deltaY * unit, -ev.deltaX * unit);
      app.request();
    },
    { passive: false },
  );

  // ---------------------------------------------------------------- keys

  const typing = (t) => t instanceof HTMLElement && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName));

  window.addEventListener("keyup", (ev) => {
    if (ev.code === "Space") {
      space = false;
      el.style.cursor = cursorFor(app.hover);
    }
  });

  window.addEventListener("keydown", (ev) => {
    if (typing(ev.target)) return;
    if (document.querySelector("dialog[open]")) return;
    const mod = ev.ctrlKey || ev.metaKey;
    const k = ev.key.toLowerCase();
    if (mod) {
      const run = {
        z: () => (ev.shiftKey ? app.redo() : app.undo()),
        y: () => app.redo(),
        a: () => app.selectAll(),
        s: () => commands.save(ev.shiftKey),
        o: () => commands.open(),
      }[k];
      if (run && !ev.altKey) {
        ev.preventDefault();
        run();
      }
      return;
    }
    if (ev.altKey) return;
    if (ev.code === "Space") {
      space = true;
      el.style.cursor = "grab";
      ev.preventDefault();
      return;
    }
    switch (ev.key) {
      case "Escape":
        app.back();
        return;
      case "Delete":
      case "Backspace":
        app.deleteSelection();
        ev.preventDefault();
        return;
      case "Enter": {
        const d = app.sel.length === 1 && app.sel[0].t === "dim" ? app.sel[0].i : null;
        if (d !== null) {
          app.editDim(d);
          ev.preventDefault();
        }
        return;
      }
      case "Home":
        app.fit();
        return;
      case "+":
      case "=":
        app.view.zoomAt(app.view.w / 2, app.view.h / 2, 1.25);
        app.request();
        return;
      case "-":
        app.view.zoomAt(app.view.w / 2, app.view.h / 2, 0.8);
        app.request();
        return;
      case "?":
        commands.help();
        return;
    }
    const tool = Object.values(TOOLS).find((t) => t.key === k);
    if (tool) {
      if (tool.id === "function") commands.function();
      else app.setTool(tool.id);
      return;
    }
    shortcutsOnSelection(app, k);
  });
}

/** H, V, T and X act on the selected entities. */
function shortcutsOnSelection(app, k) {
  const ents = app.sel.filter((s) => s.t === "ent").map((s) => s.i);
  const kind = (e) => app.scene.ent(e)?.kind;
  if (k === "h" || k === "v") {
    const c = k === "h" ? "horizontal" : "vertical";
    if (ents.length >= 2 && ents.every((e) => kind(e) === "point")) app.constrainChain(c, ents);
    else if (ents.length && ents.every((e) => kind(e) === "line")) {
      app.edit(() => app.doc.addConstraints(ents.map(() => c), ents, ents));
    }
  } else if (k === "t" && ents.length === 2 && app.doc.canConstrain("tangent", ents[0], ents[1])) {
    app.constrain("tangent", ents[0], ents[1]);
  } else if (k === "x") {
    const shaped = ents.filter((e) => kind(e) && kind(e) !== "point" && kind(e) !== "curve" && !app.scene.ent(e).builtin);
    if (shaped.length) app.edit(() => app.doc.setConstructionAll(shaped, !shaped.every((e) => app.scene.ent(e).construction)));
  }
}
