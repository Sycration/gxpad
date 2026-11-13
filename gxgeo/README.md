# gxgeo

Geometry model, constraint solver and closed-form formula derivation for 2D geometric
constructions.

A construction is a document of entities (points, lines, circles, arcs, conics, curves),
constraints between them, dimensions and variables. gxgeo solves the constraints numerically and
derives a formula for a measured dimension or a variable in terms of the driving dimensions and
the other variables, for example `θ1 = acos((d1^2 + d3^2 - d2^2)/(2d1*d3))` for an angle of a
triangle with three driving side lengths.

gxgeo is a Rust crate, a C shared library and a WebAssembly module with JavaScript bindings.

## Contents

| Platform | Files |
| --- | --- |
| Linux | `libgxgeo.so`, `gxgeo.h` |
| macOS | `libgxgeo.dylib` (install name `@rpath/libgxgeo.dylib`), `gxgeo.h` |
| Windows | `gxgeo.dll`, `libgxgeo.dll.a` (import library), `gxgeo.h` |
| Windows XP | `gxgeo.dll`, `gxgeo.lib` (import library), `gxgeo.h` |
| Web | `gxgeo_bg.wasm`, `gxgeo.js` (ES module), `gxgeo.d.ts` |

The Windows XP builds are for 32-bit Windows XP (x86, which requires a processor with SSE2) and for
Windows XP x64 and Server 2003 (x64). They use only the C runtime that ships with Windows
(`msvcrt.dll`), so they need no Visual C++ redistributable. `gxgeo.h` and `gxgeo.lib` also work with
Visual C++ 2008, whose C compiler is C89 only:

```bat
cl /Tc program.c gxgeo.lib
```

## C

`gxgeo.h` documents every function. Entities, constraints, dimensions and variables are numbered
by index. Functions that add something return its index, or -1 on error. Functions that change
the document return 1 on success, 0 when the change is rejected (the constraints would not solve,
or the change is redundant) and -1 on error. `gx_last_error()` holds the message of the last
failed call on the calling thread. Strings from the library are released with `gx_string_free`,
formulas with `gx_formula_free` and documents with `gx_doc_free`.

```c
#include "gxgeo.h"
#include <stdio.h>

int main(void) {
    GxDoc *doc = gx_doc_new();

    /* a triangle with one corner on the origin and one side horizontal */
    size_t o = gx_doc_origin(doc);
    size_t b = gx_doc_add_point(doc, 4.0, 0.2);
    size_t c = gx_doc_add_point(doc, 1.0, 3.0);
    size_t ob = gx_doc_add_line(doc, o, b, false);
    size_t bc = gx_doc_add_line(doc, b, c, false);
    size_t co = gx_doc_add_line(doc, c, o, false);
    gx_doc_constrain(doc, GX_CON_HORIZONTAL, ob, 0);

    /* driving side lengths 5, 4, 3 */
    size_t sides[3] = {ob, bc, co};
    const char *lengths[3] = {"5", "4", "3"};
    for (int k = 0; k < 3; k++) {
        size_t d = gx_doc_add_dim(doc, GX_DIM_LENGTH, sides[k], 0, 0.3, 0.3);
        gx_doc_set_dim(doc, d, lengths[k]);
    }

    /* measured angle at the origin */
    size_t angle = gx_doc_add_dim(doc, GX_DIM_ANGLE, ob, co, 0.3, 0.3);
    gx_doc_set_dim_driving(doc, angle, false);

    gx_doc_solve(doc, NULL);
    GxFormula *f = gx_derive(doc, GX_TARGET_DIM, angle, 0, NULL);
    if (f) {
        printf("%s\n", f->plain); /* θ1 = acos((d1^2 + d3^2 - d2^2)/(2d1*d3)) */
        gx_formula_free(f);
    } else {
        fprintf(stderr, "%s\n", gx_last_error());
    }
    gx_doc_free(doc);
    return 0;
}
```

```sh
cc -I. triangle.c -L. -lgxgeo -o triangle
LD_LIBRARY_PATH=. ./triangle
```

The header is plain C99 and also compiles as C++.

## JavaScript

`gxgeo.d.ts` documents the `Doc` class. Kinds of constraints, dimensions and conics are strings
(`"horizontal"`, `"length"`, `"ellipse"`, ...). Methods that change the document throw on wrong
arguments and return `false` when the change is rejected; `deriveDim` and `deriveVar` throw when
no formula is found.

```js
import init, { Doc } from "./gxgeo.js";

await init(); // loads gxgeo_bg.wasm from next to gxgeo.js

const doc = new Doc();
const o = doc.origin;
const b = doc.addPoint(4.0, 0.2);
const c = doc.addPoint(1.0, 3.0);
const ob = doc.addLine(o, b);
const bc = doc.addLine(b, c);
const co = doc.addLine(c, o);
doc.constrain("horizontal", ob);

for (const [side, length] of [[ob, "5"], [bc, "4"], [co, "3"]]) {
  doc.setDim(doc.addDim("length", side, undefined, 0.3, 0.3), length);
}

const angle = doc.addDim("angle", ob, co, 0.3, 0.3);
doc.setDimDriving(angle, false);

doc.solve();
console.log(doc.deriveDim(angle).plain); // θ1 = acos((d1^2 + d3^2 - d2^2)/(2d1*d3))
```

In Node.js, pass the module bytes to `init`:

```js
import { readFile } from "node:fs/promises";
await init({ module_or_path: await readFile(new URL("./gxgeo_bg.wasm", import.meta.url)) });
```

## Rust

Add the crate as a path dependency:

```toml
[dependencies]
gxgeo = { path = "path/to/gxgeo" }
```

```rust
use gxgeo::ops::{self, Document, Target};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string("triangle.gxp")?;
    let mut doc = Document::from_gxp(&text)?;
    ops::solve(&mut doc);
    for i in 0..ops::dim_count(&doc) {
        match ops::derive(&doc, Target::Dim(i), 0, None) {
            Ok(f) => println!("{}", f.plain),
            Err(e) => println!("dimension {i}: {e}"),
        }
    }
    Ok(())
}
```

`gxgeo::ops` is the interface the C and JS bindings wrap: each of its functions has a C function
(`gx_doc_*`) and a JS method, and the `parity` test keeps it so. The other modules are the library
internals; `Document::sketch` hands them to Rust code that needs them.

`cargo doc -p gxgeo --open` builds the API documentation.

## Documents

Documents are saved and loaded as `.gxp` text, a JSON encoding of the document:
`gx_doc_to_gxp` / `gx_doc_from_gxp` in C, `toGxp` / `Doc.fromGxp` in JavaScript and
`gxgeo::gxp` in Rust. A loaded document is solved before its values are read.

Every change is one undo step. A change made with several calls, such as adding two points and
then a line between them, becomes one step by reading the length of the undo history before the
calls and joining the steps recorded since: `gx_doc_undo_len` / `gx_doc_join_undo` in C,
`undoLen` / `joinUndo` in JavaScript, `ops::undo_len` / `ops::join_undo` in Rust.

## Formulas

A formula is exact where one is found, otherwise some part of it is a number found numerically
(`exact` is false). The derivation has a default time budget; the `extra_ms` argument of
`gx_derive`, `deriveDim`, `deriveVar` and `ops::derive` extends it (steps of 10 000 ms suit
retrying a hard target). A derivation can take seconds: run it on a copy of the document
(`gx_doc_copy`, `copy`, `ops::copy`) on another thread, with a cancel flag (`gx_cancel_new`,
`Cancel`, `ops::new_cancel`) to stop it early. Besides plain text and LaTeX, a formula has a LaTeX
form with repeated subexpressions written `u_1`, `u_2`, … and their definitions. A dimension or
variable marked numeric appears in the formulas of other targets as its current value.

The LaTeX of a formula can be typeset as a PNG image at a given size, resolution and color, on a
transparent background: `gx_latex_png` in C (the bytes are released with `gx_bytes_free`),
`latexPng` in JavaScript and `ops::latex_png` in Rust. The typesetting is done by
[latex-rust](https://crates.io/crates/latex-rust) with the STIX Two Math font.

## Building from source

Run inside the `gxgeo` directory; the outputs land in the workspace's `target/` directory.

```sh
cargo build --release                                   # C library and Rust crate
cbindgen --config cbindgen.toml --output include/gxgeo.h  # regenerates the C header
```

The Wasm module requires `rustup target add wasm32-unknown-unknown` and
`cargo install wasm-bindgen-cli` at the `wasm-bindgen` version in `Cargo.lock`:

```sh
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir pkg ../target/wasm32-unknown-unknown/release/gxgeo.wasm
```

`wasm-bindgen --target nodejs` produces a CommonJS module for Node.js instead. The `examples/`
directory holds C and JavaScript programs that build a construction or load a `.gxp` file and
print its formulas.

## License

GPL-3.0-or-later. See [LICENSE](../LICENSE).
