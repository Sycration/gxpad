// Loads a .gxp file, solves it and prints the formula of every dimension.
//
//   cargo build -p gxgeo --release --target wasm32-unknown-unknown
//   wasm-bindgen --target nodejs --out-dir pkg ../target/wasm32-unknown-unknown/release/gxgeo.wasm
//   node examples/js/derive.mjs tests/law_of_cosines.gxp
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const pkg = process.env.GXGEO_PKG ?? new URL("../../pkg/gxgeo.js", import.meta.url).pathname;
const { Doc, version } = createRequire(import.meta.url)(pkg);

const doc = Doc.fromGxp(readFileSync(process.argv[2], "utf8"));
const s = doc.solve();
console.log(`gxgeo ${version()}: solve ${s.ok}, dof ${s.dof}`);
for (let i = 0; i < doc.dimCount; i++) {
  try {
    const f = doc.deriveDim(i);
    console.log(`${f.plain}\n  latex: ${f.latex}\n  exact ${f.exact}, value ${f.value}, measured ${f.actual}`);
  } catch (e) {
    console.log(`dim ${i}: ${e.message}`);
  }
}
try {
  Doc.fromGxp("not gxp");
} catch (e) {
  console.log(`bad input: ${e.message}`);
}
