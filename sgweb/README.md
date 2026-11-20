# sgweb

sgweb is a web app for 2D geometry with constraints: draw points, lines, circles, arcs, conics and
function curves, tie them together with constraints and dimensions, and select a measurement to get a
closed-form formula for it. It works with a mouse and keyboard on a desktop and with touch on a phone or
tablet.

The whole app is one binary. It serves the page, its scripts and styles, the example constructions, and
the sggeo geometry library compiled to WebAssembly. Nothing is loaded from anywhere else. The geometry
runs in the browser; the server only hands out files.

## Running it

```sh
sgweb                      # serves on http://127.0.0.1:8080/
sgweb --open               # ... and opens it in the default browser
sgweb --port 9000          # another port (0 picks a free one)
sgweb --host 0.0.0.0       # also reachable from other devices on the network, such as a phone
```

From the source tree, `cargo run --release -p sgweb -- --open`.

## Docker

`sgweb/Dockerfile` builds an image that holds nothing but a static sgweb binary (about 14 MB). It runs as an
unprivileged user and serves on port 8080. Build and run it from the root of the repository:

```sh
docker compose -f sgweb/compose.yaml up -d            # build and start, on port 8080
SGWEB_PORT=9000 docker compose -f sgweb/compose.yaml up -d    # on another port of the host
docker compose -f sgweb/compose.yaml down             # stop
```

Or without Compose:

```sh
docker build -f sgweb/Dockerfile -t sgweb .
docker run -d --init -p 8080:8080 --read-only --cap-drop ALL sgweb
```

`--init` (`init: true` in `compose.yaml`) lets the server stop at once on `docker stop`. Without it, a
process that is the container's first process ignores the signal, and Docker kills it after 10 seconds.
The image builds wherever Rust builds: the build stage compiles for the machine's own architecture (x86-64
or arm64). `compose.yaml` also restarts the container unless it was stopped, keeps its filesystem
read-only, and drops all privileges.

To install sgweb as an app on other devices, serve it over HTTPS, for example behind a reverse proxy; see
below.

## Installing it and working offline

sgweb is a progressive web app. On the first visit, a service worker (`web/sw.js`) keeps every file of
the app in the browser. From then on it starts and runs without the server or any network, and browsers
offer to install it like an app ("Install", or "Add to Home screen" on a phone). When a newer sgweb binary
serves it, the new version installs in the background, and the page offers to reload into it. The
construction is saved in the browser, so reloading loses nothing.

Browsers only run service workers on pages served over HTTPS or from `localhost`. On this machine,
`http://localhost:8080/` installs and works offline. A page opened as `http://192.168.x.y:8080/` from
another device still works, but online only and without installing. For that, serve sgweb behind HTTPS.

## Using it

- **Tools** are on the left (at the bottom on a phone): Select, Point, Line, the Circles and Conics
  groups, Function, the Construct group (midpoints, intersections, perpendicular and parallel lines,
  bisectors, tangents, reflections, foci, directrices, asymptotes) and Measure. The bar at the top of the
  canvas says what the next tap does. Every tool shows what the next tap would make before you tap.
- **Taps snap** to points and curves: a point placed on a curve stays on it.
- **Selecting** items brings up chips with what fits them: constraints (coincident, horizontal,
  parallel, perpendicular, tangent, equal, symmetric, ...), measurements, locking, construction lines.
- **Dimensions** are driving (they set a value: a number, or an expression of variables and other
  dimensions) or measured. Double-click or double-tap a dimension label to edit it.
- **Result** shows the formula of the selected dimension or variable, derived in the background. A
  numeric or large result is searched further on its own; *Search longer* gives the search more time.
  The formula copies as text, LaTeX, or code in C, Rust, JavaScript, Python, Mathematica or Maxima.
- **Quantities** lists the variables (free ones have a slider), functions, dimensions and
  constraints.
- **Text** shows the construction in the [.sgp format](../docs/sgp-format.md). It can be edited there
  and applied.
- **Files**: the construction is kept in the browser between visits. *File* opens and saves `.sgp`
  files (dropping a file on the page opens it too), exports a PNG picture, and copies a link that holds
  the whole construction. Opening anything is a change that Undo takes back.

On a touch screen, tapping adds to the selection and tapping empty space clears it. One finger drags
items or pans; two fingers pinch and pan. With a drawing tool, the preview follows the finger and the
point goes where it is lifted. *Help* (or `?`) lists the keyboard shortcuts.

## How it is built

```
sgweb/
  src/main.rs, src/lib.rs   the server (tiny_http): embedded files, gzip, ETags, a strict CSP
  build.rs                  embeds web/, examples/, the constructions of sggeo's tests and the sggeo Wasm
                            module into the binary, and gives the service worker the list of files to keep
                            and a version that changes with them
  web/                      the app: index.html, css/app.css and ES modules in js/
  examples/                 the featured example constructions of the Examples menu; All examples… adds
                            those of sggeo/test_examples/, a folder for each gen_ test file (served as
                            examples/<name without gen_>/), which the service worker keeps once opened
  tests/                    server tests (server.rs) and the app's tests (js/, run by web.rs)
  Dockerfile, compose.yaml  the container image and how to run it (with Dockerfile.dockerignore)
```

The app is plain JavaScript modules with no framework and no bundler, drawing on a canvas. It uses sggeo
only through its JS interface (`sggeo.js` from `wasm-bindgen --target web`). `js/sg.js` loads it and turns
the clicks of the tools into the library's constructions as single undoable changes. Formulas are derived
in a Web Worker (`js/derive-worker.js`), so the page stays responsive during long searches. They are drawn
by sggeo's `latexPng`.

`build.rs` gets the sggeo module from the directory that `SGWEB_SGGEO_PKG` names (as `../build.sh`
passes it). Without that variable, it builds sggeo for `wasm32-unknown-unknown` in `target/sgweb-wasm`
and runs `wasm-bindgen` on it. That needs the `wasm32-unknown-unknown` rustup target and
`wasm-bindgen-cli` at the wasm-bindgen version in `Cargo.lock`. Every file is stored with its gzip form
when that is smaller; the server sends that form to browsers that accept it.

The release build of the web (`../build.sh web`) puts `sgweb` for Linux x64 and `sgweb.exe` for Windows
x64 in `build/web/wasm32/bin/`, next to the sggeo Wasm module in `lib/`. They are in the
`sg-<version>-web-wasm32` release archive.

## Tests

```sh
cargo test -p sgweb
```

`tests/server.rs` checks the server over HTTP and that every file the page refers to is in the binary.
`tests/web.rs` writes the embedded app to a directory and runs the JavaScript tests in `tests/js` with
Node.js. They drive the tools through the same code the canvas uses, open every example, and check the
selection actions and links. Without Node.js these tests are skipped with a notice; set
`SGWEB_REQUIRE_NODE=1` to make that a failure.

## License

GPL-3.0-or-later, like the rest of the project.
