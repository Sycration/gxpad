// Small helpers shared by the modules: formatting, DOM building and geometry on screen points.

/** A number as the app shows it: up to 4 decimals, scientific notation outside 1e-4..1e7. */
export function fmt(v) {
  if (!Number.isFinite(v)) return "—";
  const a = Math.abs(v);
  if (a !== 0 && (a < 1e-4 || a >= 1e7)) return v.toExponential(4).replace(/\.?0+e/, "e");
  let s = v.toFixed(4).replace(/\.?0+$/, "");
  return s === "-0" ? "0" : s;
}

/** A dimension value: angles (radians) in degrees. */
export function fmtDim(v, angle) {
  return angle ? `${fmt((v * 180) / Math.PI)}°` : fmt(v);
}

/** Splits a name into base and subscript: `x_A` → [x, A], `d12` → [d, 12]. */
export function splitSub(s) {
  const u = s.indexOf("_");
  if (u > 0) return [s.slice(0, u), s.slice(u + 1)];
  const m = /^(.*?[^\d])(\d+)$/.exec(s);
  return m ? [m[1], m[2]] : [s, ""];
}

/** Creates an element: h("div.card#main", {title: "x", onclick}, child, "text", ...). */
export function h(tag, props, ...children) {
  const [, name, rest] = /^([a-z0-9-]*)(.*)$/i.exec(tag);
  const el = document.createElement(name || "div");
  for (const m of rest.matchAll(/([.#])([\w-]+)/g)) {
    if (m[1] === ".") el.classList.add(m[2]);
    else el.id = m[2];
  }
  if (props) {
    for (const [k, v] of Object.entries(props)) {
      if (v === undefined || v === null || v === false) continue;
      if (k.startsWith("on")) el.addEventListener(k.slice(2), v);
      else if (k === "html") el.innerHTML = v;
      else if (k === "style" && typeof v === "object") Object.assign(el.style, v);
      else if (k in el && typeof v !== "string") el[k] = v;
      else el.setAttribute(k, v === true ? "" : v);
    }
  }
  for (const c of children.flat()) {
    if (c === null || c === undefined || c === false) continue;
    el.append(c instanceof Node ? c : document.createTextNode(String(c)));
  }
  return el;
}

/** Distance from (px, py) to the segment (ax, ay)-(bx, by), and the nearest point on it. */
export function segDist(px, py, ax, ay, bx, by) {
  const dx = bx - ax, dy = by - ay;
  const l2 = dx * dx + dy * dy;
  let t = l2 > 0 ? ((px - ax) * dx + (py - ay) * dy) / l2 : 0;
  t = Math.max(0, Math.min(1, t));
  const qx = ax + t * dx, qy = ay + t * dy;
  return [Math.hypot(px - qx, py - qy), qx, qy];
}

/** Nearest point of polylines (arrays of flat [x0, y0, x1, y1, ...] arrays) to (x, y): [distance, qx, qy]. */
export function nearestOn(polys, x, y) {
  let best = [Infinity, x, y];
  for (const pl of polys) {
    if (pl.length === 2) {
      const d = Math.hypot(x - pl[0], y - pl[1]);
      if (d < best[0]) best = [d, pl[0], pl[1]];
      continue;
    }
    for (let i = 0; i + 3 < pl.length; i += 2) {
      const r = segDist(x, y, pl[i], pl[i + 1], pl[i + 2], pl[i + 3]);
      if (r[0] < best[0]) best = r;
    }
  }
  return best;
}

/** Copies text to the clipboard; also works where the async clipboard needs a secure page. */
export async function copyText(text) {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    const ta = h("textarea", { style: { position: "fixed", opacity: "0" } });
    ta.value = text;
    document.body.append(ta);
    ta.select();
    let ok = false;
    try {
      ok = document.execCommand("copy");
    } catch {}
    ta.remove();
    return ok;
  }
}

/** Reads a setting saved by `store`, or `def`. Storage can be unavailable (private windows). */
export function load(key, def) {
  try {
    const v = localStorage.getItem("sgweb." + key);
    return v === null ? def : JSON.parse(v);
  } catch {
    return def;
  }
}

export function store(key, value) {
  try {
    localStorage.setItem("sgweb." + key, JSON.stringify(value));
  } catch {}
}

export const isMac = /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
