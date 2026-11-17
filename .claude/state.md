# rust-wasm-gl: project state

_Snapshot: 2026-11-17. Based on a skim of layout, TODO, README, git log, world renderer and generator. Not built or run._

## State
- **Last commit:** 2024-04-02, about 19 months idle. Working tree has only package.json edits and new pnpm files (pnpm switch started).
- **Stack:** Rust compiled to WASM, with `glow` for GL, `glam` for math, `egui` for debug GUI and `fastrand`. Vite and TypeScript are only a thin shell.
- **Engine bits that exist:** Game loop, Camera, Inputs, Time, Mesh, Shader, GameObject, Gizmo, Player and an FPS display.
- **Voxel world:**
  - Chunks stream in and out around the player.
  - Rendering is packed vertex data per face, in 6 sides per chunk.
  - Back-face chunk culling and an origin reset around the player chunk ("infinite" world).
  - World generation is a `sin(x) + sin(z)` heightmap, called `GeneratorV0`.
- **Tests:** `backend-tests` has a duplicated copy of `position.rs`, a known wart.
- **Open TODOs (from TODO file):**
  - Skip empty chunks.
  - Cull faces between neighbouring chunks.
  - More chunk optimization (linked video).
  - Load meshes.
  - Inputs and a better camera.
  - Check correctness against the webgl2fundamentals lessons.
  - Watch GLSL tutorials.

## Paths to delve into
1. **Revive the build.** Check that `wasm-pack`, `pnpm` and `vite` work, and drop the leftover npm and yarn lockfiles. Several crates are old (`glam` 0.27, `egui` 0.27, `glow` 0.13), so decide whether to bump them.
2. **Meshing performance:**
   - Skip empty chunks.
   - Cull faces across chunk borders.
   - Try greedy meshing.
   - Move meshing to a Web Worker. Streaming and generation run on the main thread, the likely cause of hitches.
3. **Real terrain.** Replace the sine heightmap with simplex or Perlin noise: fBm, caves, biomes, trees.
4. **Rendering quality:**
   - Ambient occlusion per vertex.
   - Fog.
   - A texture atlas or array to replace the borrowed textures.
   - Frustum culling.
   - A sky.
5. **Gameplay loop.** Block raycast, break and place, and player collision with gravity. This forces chunk remeshing on edit.
6. **Engine hygiene.** Remove the duplicated `position.rs` (make tests a proper `cargo test` module or a shared crate). Tidy `eguibackend.rs` and input handling.
7. **Stretch goal.** WebGPU via `wgpu` instead of WebGL2, or a persistent world with save/load in IndexedDB.

## Suggested order
1, then 2, then 5: working build, performance base, then something playable. Option 3 is a good break for something prettier fast.
