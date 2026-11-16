// Starts sgweb: loads the geometry library, restores the last construction and wires up the page.

import { loadSggeo, version } from "./sg.js";
import { App } from "./app.js";
import { Files } from "./files.js";
import { Inspector } from "./inspector.js";
import { Shell } from "./ui.js";
import { attachInput } from "./input.js";

async function start() {
  const root = document.getElementById("app");
  try {
    const module = await loadSggeo();
    const canvas = document.getElementById("canvas");
    const app = new App(canvas, module);
    const files = new Files(app);
    const shell = new Shell(app, files);
    new Inspector(app, document.getElementById("inspector"));
    attachInput(app, canvas, shell.commands);
    await files.boot();
    app.emit("name");
    shell.layout();
    if (app.isEmpty()) app.fit();
    app.request();
    root.classList.remove("loading");
    const info = await fetch("info.json").then((r) => r.json()).catch(() => ({}));
    document.getElementById("about").textContent = `sgweb ${info.sgweb ?? ""} · sggeo ${version()}`;
    // for tests and the browser console
    window.sgweb = { app, files };
    offline(app);
  } catch (e) {
    root.classList.remove("loading");
    root.classList.add("failed");
    document.getElementById("boot-error").textContent = `sgweb could not start: ${e?.message ?? e}`;
    throw e;
  }
}

start();

/**
 * Registers the service worker that keeps the app for use without a network. A new version of sgweb
 * installs in the background; the page then offers to reload into it (the construction is saved in the
 * browser, so nothing is lost). Browsers allow service workers only on https and on localhost.
 */
async function offline(app) {
  if (!("serviceWorker" in navigator) || !window.isSecureContext) return;
  // a page that had no worker yet gets one without reloading; only a change of version reloads
  const had = !!navigator.serviceWorker.controller;
  let reloading = false;
  navigator.serviceWorker.addEventListener("controllerchange", () => {
    if (had && !reloading) {
      reloading = true;
      location.reload();
    }
  });
  let reg;
  try {
    reg = await navigator.serviceWorker.register(new URL("../sw.js", import.meta.url));
  } catch {
    return;
  }
  const offer = (w) => app.toast("A new version of sgweb is ready.", "info", { label: "Reload", run: () => w.postMessage("update") });
  if (reg.waiting && had) offer(reg.waiting);
  reg.addEventListener("updatefound", () => {
    const w = reg.installing;
    w?.addEventListener("statechange", () => {
      if (w.state === "installed" && navigator.serviceWorker.controller) offer(w);
    });
  });
}
