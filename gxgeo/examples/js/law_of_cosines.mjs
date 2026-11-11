// Builds the law of cosines construction from scratch (the law_of_cosines
// test in tests/gen_example.rs), solves it, prints the derived formula of
// the angle and writes the document as a .gxp file.
//
//   cargo build -p gxgeo --release --target wasm32-unknown-unknown
//   wasm-bindgen --target nodejs --out-dir pkg ../target/wasm32-unknown-unknown/release/gxgeo.wasm
//   node examples/js/law_of_cosines.mjs law_of_cosines.gxp
import { writeFileSync } from "node:fs";
import { createRequire } from "node:module";

const pkg = process.env.GXGEO_PKG ?? new URL("../../pkg/gxgeo.js", import.meta.url).pathname;
const { Doc } = createRequire(import.meta.url)(pkg);

const out = process.argv[2];
if (!out) {
  console.error("usage: node law_of_cosines.mjs OUT.gxp");
  process.exit(2);
}

function must(ok, what) {
  if (!ok) throw new Error(`${what} rejected`);
}

const doc = new Doc();

// a triangle with one corner on the origin
const o = doc.origin;
const b = doc.addPoint(4.0, 0.2);
const c = doc.addPoint(1.0, 3.0);
const ab = doc.addLine(o, b);
const bc = doc.addLine(b, c);
const ca = doc.addLine(c, o);
must(doc.constrain("horizontal", ab), "horizontal OB");

// side lengths 5, 4, 3, labels at (0.3, 0.3) from each side's midpoint
for (const [side, length] of [[ab, "5"], [bc, "4"], [ca, "3"]]) {
  const d = doc.addDim("length", side, undefined, 0.3, 0.3);
  must(doc.dimDriving(d), "length is determined already");
  must(doc.setDim(d, length), "length value");
}

// the angle at the origin, measured
const ang = doc.addDim("angle", ab, ca, 0.3, 0.3);
must(doc.setDimDriving(ang, false), "measured angle");

const s = doc.solve();
must(s.ok, "solve");
console.log(`solved, ${s.dof} degrees of freedom left`);

const f = doc.deriveDim(ang);
console.log(`${f.plain}\n  latex: ${f.latex}\n  exact ${f.exact}, value ${f.value}, measured ${f.actual}`);

writeFileSync(out, doc.toGxp());
console.log(`wrote ${out}`);
