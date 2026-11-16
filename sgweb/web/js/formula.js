// Formulas of the selected measurement: derives them in a worker whenever the target or the construction
// changes, and keeps searching longer on its own while the result is numeric or large. The newest result
// for the current construction that is no worse than the one shown replaces it.

/** Extra search time each longer search adds. */
export const STEP_MS = 10000;
/** Longer searches started without being asked, while each one changes the result. */
const AUTO_LEVELS = 2;
/** Size up to which an exact formula is simple enough to stop searching. */
const SIMPLE = 60;

/** Lower is better: an error, then numeric, then larger formulas rank lower. */
function rank(r) {
  return r.error ? [2, 0] : [r.f.exact ? 0 : 1, r.f.size];
}

function better(a, b) {
  const [x, y] = [rank(a), rank(b)];
  return x[0] < y[0] || (x[0] === y[0] && x[1] <= y[1]);
}

/** True when no further search is useful by default. */
export function settled(r) {
  return !!r.error || (r.f.exact && r.f.size <= SIMPLE);
}

export class Formulas {
  /** `module` is the compiled sggeo module; `onchange` is called when the state below changes. */
  constructor(module, onchange) {
    this.module = module;
    this.onchange = onchange;
    this.worker = null;
    this.job = null;
    this.seq = 0;
    /** {kind, name}: the measurement whose formula is wanted */
    this.target = null;
    /** the result shown: {key, gen, level, f | error, ms} */
    this.result = null;
    this.level = 0;
    this.timer = 0;
  }

  get busy() {
    return this.job !== null;
  }

  spawn() {
    this.worker = new Worker(new URL("./derive-worker.js", import.meta.url), { type: "module" });
    this.worker.postMessage({ type: "init", module: this.module });
    this.worker.onmessage = (ev) => this.finished(ev.data);
    this.worker.onerror = (ev) => {
      ev.preventDefault();
      this.finished({ id: this.job?.id, ok: false, error: "The formula search stopped unexpectedly." });
    };
  }

  /** Stops the running search; the worker cannot be interrupted, so it is replaced. */
  cancel() {
    clearTimeout(this.timer);
    if (this.job) {
      this.worker.terminate();
      this.worker = null;
      this.job = null;
    }
  }

  /**
   * Updates the wanted formula: `target` ({kind: "dim" | "var", name}) or null, in document generation
   * `gen`, whose .sgp text `sgp()` gives. `still` is false during a drag, when no new search starts.
   */
  update(target, gen, sgp, still) {
    const key = target ? target.kind + ":" + target.name : null;
    const same = this.target && key === this.target.kind + ":" + this.target.name;
    if (!target) {
      this.cancel();
      const had = this.target || this.result;
      this.target = null;
      this.result = null;
      if (had) this.onchange();
      return;
    }
    if (!same) {
      this.cancel();
      this.target = target;
      this.result = null;
      this.level = 0;
      this.onchange();
    }
    if (!still) {
      // a search for a construction the drag has left behind is of no use
      if (this.job && this.job.gen !== gen) this.cancel();
      return;
    }
    if (this.result?.gen === gen && this.result.key === key && (this.job === null || this.job.gen === gen)) return;
    if (this.job?.gen === gen && this.job.key === key) return;
    if (this.result && this.result.gen !== gen) this.level = 0;
    this.cancel();
    // a short pause lets a burst of edits settle first
    this.timer = setTimeout(() => this.run(key, gen, sgp(), this.level), same ? 150 : 0);
    this.onchange();
  }

  run(key, gen, text, level) {
    if (!this.worker) this.spawn();
    const id = ++this.seq;
    this.job = { id, key, gen, level };
    this.worker.postMessage({ type: "derive", id, sgp: text, kind: this.target.kind, name: this.target.name, extraMs: level * STEP_MS });
    this.text = text;
    this.onchange();
  }

  finished(m) {
    const job = this.job;
    if (!job || m.id !== job.id) return;
    this.job = null;
    const r = { key: job.key, gen: job.gen, level: job.level, ms: m.ms, f: m.f, error: m.ok ? null : m.error };
    const prev = this.result && this.result.key === r.key && this.result.gen === r.gen ? this.result : null;
    const changed = !prev || prev.error !== r.error || prev.f?.plain !== r.f?.plain || prev.f?.exact !== r.f?.exact;
    if (!prev || better(r, prev)) this.result = r;
    else this.result = { ...prev, level: r.level };
    this.level = job.level;
    if (changed && !settled(this.result) && job.level < AUTO_LEVELS) {
      this.level = job.level + 1;
      this.run(job.key, job.gen, this.text, this.level);
    }
    this.onchange();
  }

  /** Searches again with more time than the last search. */
  searchLonger() {
    if (!this.result || this.busy) return;
    this.level += 1;
    this.run(this.result.key, this.result.gen, this.text, this.level);
  }

  /** Seconds of extra search time the next "search longer" gives. */
  nextSeconds() {
    return ((this.level + 1) * STEP_MS) / 1000;
  }
}
