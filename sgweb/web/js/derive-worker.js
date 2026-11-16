// Derives formulas off the main thread. It gets the compiled sggeo module once, then for each job the
// construction as .sgp text and the name of the target; the app stops a job by ending the worker.

import { initSync, Doc } from "../pkg/sggeo.js";

const LANGS = ["c", "rust", "js", "python", "mathematica", "maxima"];

function find(doc, kind, name) {
  const n = kind === "dim" ? doc.dimCount : doc.varCount;
  for (let i = 0; i < n; i++) if ((kind === "dim" ? doc.dimName(i) : doc.varName(i)) === name) return i;
  throw new Error(`${name} is not in the construction`);
}

self.onmessage = (ev) => {
  const m = ev.data;
  if (m.type === "init") {
    initSync({ module: m.module });
    return;
  }
  if (m.type !== "derive") return;
  let doc = null;
  const t0 = performance.now();
  try {
    doc = Doc.fromSgp(m.sgp);
    doc.solve();
    const i = find(doc, m.kind, m.name);
    const f = m.kind === "dim" ? doc.deriveDim(i, m.extraMs) : doc.deriveVar(i, m.extraMs);
    const out = {
      name: f.name,
      plain: f.plain,
      latex: f.latex,
      latexShort: f.latexShort,
      defs: f.defs.map((d) => ({ name: d.name, latex: d.latex })),
      exact: f.exact,
      isAngle: f.isAngle,
      value: f.value,
      actual: f.actual,
      size: f.size,
      code: Object.fromEntries(LANGS.map((l) => [l, f.code(l)])),
    };
    f.free();
    self.postMessage({ id: m.id, ok: true, f: out, ms: performance.now() - t0 });
  } catch (e) {
    self.postMessage({ id: m.id, ok: false, error: String(e?.message ?? e), ms: performance.now() - t0 });
  } finally {
    doc?.free();
  }
};
