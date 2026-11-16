// Files and persistence: the construction is kept in the browser between visits, opens from .sgp files,
// examples and shared links, saves as .sgp, and exports as a picture. Opening replaces the drawing as an
// undoable change, so what was there before is one undo away.

import { h, load, store } from "./util.js";

const AUTOSAVE_MS = 400;

function base64url(bytes) {
  let s = "";
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(s).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function unbase64url(s) {
  const b = atob(s.replace(/-/g, "+").replace(/_/g, "/"));
  return Uint8Array.from(b, (c) => c.charCodeAt(0));
}

async function pipe(bytes, stream) {
  return new Uint8Array(await new Response(new Blob([bytes]).stream().pipeThrough(stream)).arrayBuffer());
}

/** The construction text in the form a link carries: deflated and base64url-encoded. */
export async function packLink(text) {
  return base64url(await pipe(new TextEncoder().encode(text), new CompressionStream("deflate-raw")));
}

export async function unpackLink(s) {
  return new TextDecoder().decode(await pipe(unbase64url(s), new DecompressionStream("deflate-raw")));
}

function download(blob, name) {
  const a = h("a", { href: URL.createObjectURL(blob), download: name });
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(a.href), 10000);
}

function fileName(name, ext) {
  const base = name.replace(/[\\/:*?"<>|]+/g, "-").trim() || "construction";
  return base.endsWith(ext) ? base : base + ext;
}

export class Files {
  constructor(app) {
    this.app = app;
    this.handle = null;
    this.timer = 0;
    this.input = h("input", { type: "file", accept: ".sgp,text/plain", hidden: true });
    this.input.addEventListener("change", () => {
      const f = this.input.files?.[0];
      if (f) this.openFile(f);
      this.input.value = "";
    });
    document.body.append(this.input);
    app.on("doc", () => this.autosave());
    app.on("name", () => this.autosave());
    // a .sgp file dropped anywhere on the page opens
    window.addEventListener("dragover", (ev) => {
      if ([...(ev.dataTransfer?.items ?? [])].some((i) => i.kind === "file")) ev.preventDefault();
    });
    window.addEventListener("drop", (ev) => {
      const f = ev.dataTransfer?.files?.[0];
      if (!f) return;
      ev.preventDefault();
      this.openFile(f);
    });
  }

  /** Restores the last construction of this browser, then opens a shared one from the address, if any. */
  async boot() {
    const saved = load("doc", null);
    if (saved) {
      try {
        this.app.open(saved, load("name", "Untitled"));
      } catch {
        // a construction an older or broken session left behind is dropped
      }
    }
    const m = /^#sgp=([\w-]+)$/.exec(location.hash);
    if (m) {
      history.replaceState(null, "", location.pathname + location.search);
      try {
        const name = new URLSearchParams(location.search).get("name") ?? "Shared construction";
        this.load(await unpackLink(m[1]), name);
        if (saved) this.app.toast("Opened the shared construction. Undo brings back your previous drawing.");
      } catch (e) {
        this.app.toast("The link does not hold a construction sgweb can read: " + (e?.message ?? e), "error");
      }
    }
  }

  autosave() {
    clearTimeout(this.timer);
    this.timer = setTimeout(() => {
      try {
        store("doc", this.app.doc.toSgp());
        store("name", this.app.name);
      } catch {}
    }, AUTOSAVE_MS);
  }

  /** Replaces the construction with the one in .sgp text `text`, as an undoable change. */
  load(text, name) {
    const app = this.app;
    app.doc.loadSgp(text);
    app.reset();
    app.setName(name);
    app.fit();
    this.handle = null;
  }

  pick() {
    this.input.click();
  }

  async openFile(file) {
    try {
      this.load(await file.text(), file.name.replace(/\.sgp$/i, ""));
    } catch (e) {
      this.app.toast(`${file.name} is not a construction sgweb can read: ${e?.message ?? e}`, "error");
    }
  }

  async openExample(path, title) {
    try {
      const r = await fetch(path);
      if (!r.ok) throw new Error(r.statusText);
      this.load(await r.text(), title);
    } catch (e) {
      this.app.toast(`The example did not open: ${e?.message ?? e}`, "error");
    }
  }

  async examples() {
    if (!this.list) {
      try {
        this.list = await (await fetch("examples/index.json")).json();
      } catch {
        this.list = [];
      }
    }
    return this.list;
  }

  /** Saves as .sgp: to the file it came from or was saved to where the browser allows that, else as a
   *  download. `ask` asks for a new file. */
  async save(ask = false) {
    const app = this.app;
    const text = app.doc.toSgp();
    if (window.showSaveFilePicker) {
      try {
        if (ask || !this.handle) {
          this.handle = await window.showSaveFilePicker({
            suggestedName: fileName(app.name, ".sgp"),
            types: [{ description: "Construction", accept: { "text/plain": [".sgp"] } }],
          });
          app.setName(this.handle.name.replace(/\.sgp$/i, ""));
        }
        const w = await this.handle.createWritable();
        await w.write(text);
        await w.close();
        app.toast(`Saved ${this.handle.name}`);
        return;
      } catch (e) {
        if (e?.name === "AbortError") return;
        this.handle = null;
      }
    }
    download(new Blob([text], { type: "text/plain" }), fileName(app.name, ".sgp"));
  }

  exportImage() {
    const app = this.app;
    app.canvas.toBlob((b) => b && download(b, fileName(app.name, ".png")), "image/png");
  }

  /** A link that opens this construction, copied to the clipboard. */
  async link() {
    const app = this.app;
    const q = app.name !== "Untitled" ? "?name=" + encodeURIComponent(app.name) : "";
    return location.origin + location.pathname + q + "#sgp=" + (await packLink(app.doc.toSgp()));
  }
}
