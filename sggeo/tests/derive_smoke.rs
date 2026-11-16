//! Loads a .sgp document and derives a measured dimension. Runs natively
//! and on wasm32 (`cargo test -p sggeo --target wasm32-unknown-unknown`),
//! where it checks that the derivation's clock works without std time.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[test]
fn law_of_cosines_angle() {
    let doc = sggeo::sgp::from_str(include_str!("law_of_cosines.sgp")).unwrap();
    let i = doc.dims.iter().position(|d| d.as_ref().is_some_and(|d| !d.driving)).unwrap();
    let d = sggeo::derive::derive(&doc, &doc.measure(i)).unwrap();
    assert!(d.exact);
    assert!((d.value - d.actual).abs() < 1e-9, "{} vs {}", d.value, d.actual);
}
