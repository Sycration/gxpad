// Loads a .sgp file, solves it and prints the formula of every dimension.
//
//   cargo build -p sggeo --release --target wasm32-unknown-unknown
//   wasm-bindgen --target nodejs --out-dir pkg ../target/wasm32-unknown-unknown/release/sggeo.wasm
//   node examples/js/derive.mjs tests/law_of_cosines.sgp
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const pkg = process.env.SGGEO_PKG ?? new URL("../../pkg/sggeo.js", import.meta.url).pathname;
const { Doc, version } = createRequire(import.meta.url)(pkg);

const doc = Doc.fromSgp(readFileSync(process.argv[2], "utf8"));
const s = doc.solve();
console.log(`sggeo ${version()}: solve ${s.ok}, dof ${s.dof}`);
for (let i = 0; i < doc.dimCount; i++) {
  try {
    const f = doc.deriveDim(i);
    console.log(`${f.plain}\n  latex: ${f.latex}\n  exact ${f.exact}, value ${f.value}, measured ${f.actual}`);
    console.log(`  JavaScript: ${f.code("js")}`);
  } catch (e) {
    console.log(`dim ${i}: ${e.message}`);
  }
}
try {
  Doc.fromSgp("not sgp");
} catch (e) {
  console.log(`bad input: ${e.message}`);
}
