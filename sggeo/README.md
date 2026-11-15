# sggeo

Geometry model, constraint solver and closed-form formula derivation for 2D geometric
constructions.

A construction is a document of entities (points, lines, circles, arcs, conics, curves),
constraints between them, dimensions and variables. sggeo solves the constraints numerically and
derives a formula for a measured dimension or a variable in terms of the driving dimensions and
the other variables, for example `θ1 = acos((d1^2 + d3^2 - d2^2)/(2d1*d3))` for an angle of a
triangle with three driving side lengths.

sggeo is a Rust crate, a C shared library and a WebAssembly module with JavaScript bindings.

## Contents

| Platform | Files |
| --- | --- |
| Linux | `libsggeo.so`, `sggeo.h` |
| macOS | `libsggeo.dylib` (install name `@rpath/libsggeo.dylib`), `sggeo.h` |
| Windows | `sggeo.dll`, `libsggeo.dll.a` (import library), `sggeo.h` |
| Windows XP | `sggeo.dll`, `sggeo.lib` (import library), `sggeo.h` |
| Web | `sggeo_bg.wasm`, `sggeo.js` (ES module), `sggeo.d.ts` |

The Windows XP builds are for 32-bit Windows XP (x86, which requires a processor with SSE2) and for
Windows XP x64 and Server 2003 (x64). They use only the C runtime that ships with Windows
(`msvcrt.dll`), so they need no Visual C++ redistributable. `sggeo.h` and `sggeo.lib` also work with
Visual C++ 2008, whose C compiler is C89 only:

```bat
cl /Tc program.c sggeo.lib
```

## C

`sggeo.h` documents every function. Entities, constraints, dimensions and variables are numbered
by index. Functions that add something return its index, or -1 on error. Functions that change
the document return 1 on success, 0 when the change is rejected (the constraints would not solve,
or the change is redundant) and -1 on error. `sg_last_error()` holds the message of the last
failed call on the calling thread. Strings from the library are released with `sg_string_free`,
formulas with `sg_formula_free` and documents with `sg_doc_free`.

```c
#include "sggeo.h"
#include <stdio.h>

int main(void) {
    SgDoc *doc = sg_doc_new();

    /* a triangle with one corner on the origin and one side horizontal */
    size_t o = sg_doc_origin(doc);
    size_t b = sg_doc_add_point(doc, 4.0, 0.2);
    size_t c = sg_doc_add_point(doc, 1.0, 3.0);
    size_t ob = sg_doc_add_line(doc, o, b, false);
    size_t bc = sg_doc_add_line(doc, b, c, false);
    size_t co = sg_doc_add_line(doc, c, o, false);
    sg_doc_constrain(doc, SG_CON_HORIZONTAL, ob, 0);

    /* driving side lengths 5, 4, 3 */
    size_t sides[3] = {ob, bc, co};
    const char *lengths[3] = {"5", "4", "3"};
    for (int k = 0; k < 3; k++) {
        size_t d = sg_doc_add_dim(doc, SG_DIM_LENGTH, sides[k], 0, 0.3, 0.3);
        sg_doc_set_dim(doc, d, lengths[k]);
    }

    /* measured angle at the origin */
    size_t angle = sg_doc_add_dim(doc, SG_DIM_ANGLE, ob, co, 0.3, 0.3);
    sg_doc_set_dim_driving(doc, angle, false);

    sg_doc_solve(doc, NULL);
    SgFormula *f = sg_derive(doc, SG_TARGET_DIM, angle, 0, NULL);
    if (f) {
        printf("%s\n", f->plain); /* θ1 = acos((d1^2 + d3^2 - d2^2)/(2d1*d3)) */
        sg_formula_free(f);
    } else {
        fprintf(stderr, "%s\n", sg_last_error());
    }
    sg_doc_free(doc);
    return 0;
}
```

```sh
cc -I. triangle.c -L. -lsggeo -o triangle
LD_LIBRARY_PATH=. ./triangle
```

The header is plain C99 and also compiles as C++.

## JavaScript

`sggeo.d.ts` documents the `Doc` class. Kinds of constraints, dimensions and conics are strings
(`"horizontal"`, `"length"`, `"ellipse"`, ...). Methods that change the document throw on wrong
arguments and return `false` when the change is rejected; `deriveDim` and `deriveVar` throw when
no formula is found.

```js
import init, { Doc } from "./sggeo.js";

await init(); // loads sggeo_bg.wasm from next to sggeo.js

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
await init({ module_or_path: await readFile(new URL("./sggeo_bg.wasm", import.meta.url)) });
```

## Rust

Add the crate as a path dependency:

```toml
[dependencies]
sggeo = { path = "path/to/sggeo" }
```

```rust
use sggeo::ops::{self, Document, Target};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string("triangle.sgp")?;
    let mut doc = Document::from_sgp(&text)?;
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

`sggeo::ops` is the interface the C and JS bindings wrap: each of its functions has a C function
(`sg_doc_*`) and a JS method, and the `parity` test keeps it so. The other modules are the library
internals; `Document::sketch` hands them to Rust code that needs them.

`cargo doc -p sggeo --open` builds the API documentation.

## Documents

Documents are saved and loaded as `.sgp` text: `sg_doc_to_sgp` / `sg_doc_from_sgp` in C,
`toSgp` / `Doc.fromSgp` in JavaScript and `sggeo::sgp` in Rust. A loaded document is solved
before its values are read. The text states the construction one step per line, in an order it can
be built in, and then the solved positions ([the format](../docs/sgp-format.md)):

```
sgp 2

given
  triangle ABC
    d1: |AB| = 4
    d2: |BC| = 3
    d3: |CA| = 5

construct
  l1 = bisect(AB, AC)
  l2 = bisect(BA, BC)
  I = meet(l1, l2)

find
  r: dist(I, AB)

where
  A (0, 0)
  ...
```

A document reads back as the same construction, with its entities, constraints and dimensions
numbered in the order of the file.

Every change is one undo step. A change made with several calls, such as adding two points and
then a line between them, becomes one step by reading the length of the undo history before the
calls and joining the steps recorded since: `sg_doc_undo_len` / `sg_doc_join_undo` in C,
`undoLen` / `joinUndo` in JavaScript, `ops::undo_len` / `ops::join_undo` in Rust.

## Formulas

A formula is exact where one is found, otherwise some part of it is a number found numerically
(`exact` is false). The derivation has a default time budget; the `extra_ms` argument of
`sg_derive`, `deriveDim`, `deriveVar` and `ops::derive` extends it (steps of 10 000 ms suit
retrying a hard target). A derivation can take seconds: run it on a copy of the document
(`sg_doc_copy`, `copy`, `ops::copy`) on another thread, with a cancel flag (`sg_cancel_new`,
`Cancel`, `ops::new_cancel`) to stop it early. Besides plain text and LaTeX, a formula has a LaTeX
form with repeated subexpressions written `u_1`, `u_2`, … and their definitions. A dimension or
variable marked numeric appears in the formulas of other targets as its current value.

A formula can also be written as source code in C, Rust, JavaScript, Python, Mathematica or
Maxima: `sg_formula_code` with a `SG_LANG_*` constant in C (the string is released with
`sg_string_free`), `Formula.code` with `"c"`, `"rust"`, `"js"`, `"python"`, `"mathematica"` or
`"maxima"` in JavaScript, and `ops::formula_code` with an `ops::Lang` in Rust. The code is one
statement per line: the repeated subexpressions assigned to temporaries `u_1`, `u_2`, …, then the
formula assigned to a variable named after the target. The other quantities are read from
variables of their names, made into identifiers of the language (Greek letters spelled out except
in Mathematica). Angles are in radians. The law of cosines example in Python reads

```python
import math
theta1 = math.acos((d1**2 + d3**2 - d2**2) / (2 * d1 * d3))
```

The LaTeX of a formula can be typeset as a PNG image at a given size, resolution and color, on a
transparent background: `sg_latex_png` in C (the bytes are released with `sg_bytes_free`),
`latexPng` in JavaScript and `ops::latex_png` in Rust. The typesetting is done by
[latex-rust](https://crates.io/crates/latex-rust) with the STIX Two Math font.

## Building from source

Run inside the `sggeo` directory; the outputs land in the workspace's `target/` directory.

```sh
cargo build --release                                   # C library and Rust crate
cbindgen --config cbindgen.toml --output include/sggeo.h  # regenerates the C header
```

The Wasm module requires `rustup target add wasm32-unknown-unknown` and
`cargo install wasm-bindgen-cli` at the `wasm-bindgen` version in `Cargo.lock`:

```sh
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir pkg ../target/wasm32-unknown-unknown/release/sggeo.wasm
```

`wasm-bindgen --target nodejs` produces a CommonJS module for Node.js instead. The `examples/`
directory holds C and JavaScript programs that build a construction or load a `.sgp` file and
print its formulas.

## License

GPL-3.0-or-later. See [LICENSE](../LICENSE).
