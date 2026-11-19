//! Pure game logic: no `glow`, `web-sys` or `wasm-bindgen`. Compiles and tests natively.
pub mod camera;
pub mod math;
pub mod render;
#[cfg(test)]
mod testutil;
pub mod world;
