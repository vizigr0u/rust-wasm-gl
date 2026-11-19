//! Pure game logic: no `web-sys` or `wasm-bindgen`. Compiles and tests natively.
pub mod camera;
pub mod gpu;
pub mod math;
pub mod playerview;
pub mod render;
#[cfg(test)]
mod testutil;
pub mod world;
