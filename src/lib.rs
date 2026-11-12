//! gxpad: interactive 2D geometric constructions with constraints and
//! symbolic formula derivation.
//!
//! The app reaches the geometry library only through `gxgeo::ops`, the
//! interface the C and JS bindings wrap, so everything the app does can also
//! be done from C and JS. A test enforces this.

pub use gxgeo::ops;

pub mod app;
pub mod canvas;
pub mod construct;
pub mod geo;
pub mod icons;
pub mod mathview;
pub mod panel;
