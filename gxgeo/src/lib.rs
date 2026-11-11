//! gxgeo: geometry model, constraint solver, and symbolic formula
//! derivation for 2D geometric constructions.

pub mod alg;
pub mod build;
pub mod dd;
pub mod derive;
pub mod expr;
pub mod ffi;
pub mod fit;
pub mod formula;
pub mod gxp;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub mod js;
pub mod model;
pub mod ops;
pub mod sketch;
pub mod solver;
