// sgweb's service worker: keeps every file of the app in the browser, so that it starts and runs without a
// network (the test constructions among the examples as they are opened). build.rs fills in the files and a version that changes with any of them; a changed version
// installs beside the running one, and the page offers to reload into it.

const VERSION = "__SGWEB_VERSION__";
const FILES = __SGWEB_FILES__;
const CACHE = "sgweb-" + VERSION;

self.addEventListener("install", (ev) => {
  // fetched past the HTTP cache, so the new version gets the new files
  ev.waitUntil(caches.open(CACHE).then((c) => c.addAll(FILES.map((f) => new Request(f, { cache: "reload" })))));
});

self.addEventListener("activate", (ev) => {
  ev.waitUntil(
    (async () => {
      for (const k of await caches.keys()) if (k.startsWith("sgweb-") && k !== CACHE) await caches.delete(k);
      await self.clients.claim();
    })(),
  );
});

// the page asks for the waiting version to take over when the user chooses to reload
self.addEventListener("message", (ev) => {
  if (ev.data === "update") self.skipWaiting();
});

self.addEventListener("fetch", (ev) => {
  const req = ev.request;
  if (req.method !== "GET" || new URL(req.url).origin !== location.origin) return;
  ev.respondWith(
    (async () => {
      const cache = await caches.open(CACHE);
      // every page is the app, whatever its query or fragment (a shared construction)
      const hit = req.mode === "navigate" ? await cache.match("./") : await cache.match(req);
      if (hit) return hit;
      const res = await fetch(req);
      // the test constructions are not in FILES (there are hundreds): each is kept once it is opened
      if (res.ok && /\/examples\/[^/]+\/[^/]+\.sgp$/.test(new URL(req.url).pathname)) await cache.put(req, res.clone());
      return res;
    })(),
  );
});
