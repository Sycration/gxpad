// The bridge to sggeo, the geometry library, in its Wasm build: loading it, and the ways the app reads and
// changes a document through its JS interface.
//
// The tools work with places: where a click lands is an existing point, a free position, or a position on a
// curve. The library's constructions take points, so a change first creates the points its places need.
// `change` makes all of that one undoable step, and leaves no trace when the library rejects it.

import init, { Doc, latexPng, version } from "../pkg/sggeo.js";

export { Doc, latexPng, version };

/** The compiled module, which workers instantiate again. */
export let wasmModule = null;

export async function loadSggeo(url = new URL("../pkg/sggeo_bg.wasm", import.meta.url)) {
  const resp = fetch(url);
  wasmModule = WebAssembly.compileStreaming
    ? await WebAssembly.compileStreaming(resp)
    : await WebAssembly.compile(await (await resp).arrayBuffer());
  await init({ module_or_path: wasmModule });
  return wasmModule;
}

// ------------------------------------------------------------------ places

export const at = {
  point: (e) => ({ kind: "point", e }),
  free: (p) => ({ kind: "free", p }),
  on: (c, p) => ({ kind: "on", c, p }),
};

/** World position of a place. */
export function placePos(doc, pl) {
  return pl.kind === "point" ? doc.pointPos(pl.e) : pl.p;
}

/** The point a place stands for, created when it is new. */
export function realize(doc, pl) {
  switch (pl.kind) {
    case "point":
      return pl.e;
    case "free":
      return doc.addPoint(pl.p[0], pl.p[1]);
    case "on":
      return doc.addPointOn(pl.c, pl.p[0], pl.p[1]);
  }
  throw new Error("not a place");
}

export function samePoint(a, b) {
  return a.kind === "point" && b.kind === "point" && a.e === b.e;
}

/** Thrown by a change the library turns down without an error message of its own. */
export class Rejected extends Error {
  constructor(msg = "That does not fit the construction: it conflicts with it or is already implied.") {
    super(msg);
  }
}

/** Passes a result through, turning the `false` of a rejected change into a Rejected error. */
export function must(r, msg) {
  if (r === false) throw new Rejected(msg);
  return r;
}

/**
 * Runs `fn(doc)` as one undoable change and returns its result. It runs on a copy first: when it throws
 * there, the document is left as it was (no points made for the places, no undo steps) and the error
 * propagates.
 */
export function change(doc, fn) {
  const trial = doc.copy();
  try {
    fn(trial);
  } finally {
    trial.free();
  }
  const n = doc.undoLen();
  try {
    return fn(doc);
  } finally {
    doc.joinUndo(n);
  }
}

/**
 * Runs `fn` on a copy of the document and hands the copy and the result to `use`, for previews: what
 * `fn` would make. Returns what `use` returns, or null when `fn` fails.
 */
export function onCopy(doc, fn, use) {
  const copy = doc.copy();
  try {
    const before = copy.entCount;
    let r;
    try {
      r = fn(copy);
    } catch {
      return null;
    }
    return use(copy, r, before);
  } finally {
    copy.free();
  }
}

// ------------------------------------------------------------------ reading

/**
 * Snapshot of the entities for drawing and hit testing: one record per live entity slot, indexed by
 * entity.
 */
export function readEntities(doc) {
  const n = doc.entCount;
  const ents = new Array(n).fill(null);
  const fixed = new Set([doc.origin, doc.xAxis, doc.yAxis]);
  let fn = 0;
  for (let e = 0; e < n; e++) {
    const kind = doc.entKind(e);
    if (!kind) continue;
    const r = {
      e,
      kind,
      name: doc.entName(e) ?? "",
      hidden: doc.entHidden(e),
      construction: doc.entConstruction(e),
      determined: doc.entFixed(e),
      conflict: doc.entConflict(e),
      builtin: fixed.has(e),
      axis: e === doc.xAxis || e === doc.yAxis,
      locked: false,
      infinite: false,
      fnIndex: -1,
    };
    if (kind === "point") r.locked = doc.pointLocked(e);
    if (kind === "line") r.infinite = doc.entInfinite(e);
    if (kind === "curve") r.fnIndex = fn++;
    ents[e] = r;
  }
  return ents;
}

/** A short readable name of an entity: its name, the points of a line (AB), or its kind and center. */
export function describe(doc, e) {
  const kind = doc.entKind(e);
  if (!kind) return "?";
  if (e === doc.xAxis) return "x axis";
  if (e === doc.yAxis) return "y axis";
  const name = doc.entName(e);
  if (kind === "point") return name || "point";
  if (kind === "line") {
    const [a, b] = doc.linePts(e);
    const na = doc.entName(a), nb = doc.entName(b);
    if (na && nb) return na + nb;
  }
  if (name) return name;
  if (kind === "curve") return doc.curveSrc(e);
  if (kind === "circle" || kind === "arc") {
    const c = doc.circleVals(e)[0];
    return `${kind} at ${doc.entName(c) || "?"}`;
  }
  if (kind === "conic") return doc.conicVals(e)[0];
  return kind;
}

/** Positions of the points that drawing or fitting an entity takes into account. */
export function extentPoints(doc, e, kind) {
  switch (kind) {
    case "point":
      return [doc.pointPos(e)];
    case "circle":
    case "arc": {
      const [c, r] = doc.circleVals(e);
      const p = doc.pointPos(c);
      return [
        [p[0] - r, p[1] - r],
        [p[0] + r, p[1] + r],
      ];
    }
    case "conic": {
      const [, c, a, b] = doc.conicVals(e);
      const p = doc.pointPos(c);
      const r = Math.max(Math.abs(a), Math.abs(b), 1e-9) * 1.2;
      return [
        [p[0] - r, p[1] - r],
        [p[0] + r, p[1] + r],
      ];
    }
  }
  return [];
}
