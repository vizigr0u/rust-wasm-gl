# Architecture

Rust → WASM voxel engine, wgpu (WebGPU only). Browser shell in TS + Vite.

## Workspace (Cargo: `voxel-core`, `backend`)

| Path | Role |
|---|---|
| `voxel-core/` | Pure game logic. No glow/web-sys/wasm-bindgen. Builds and tests natively (`pnpm test`). |
| `voxel-core/src/world/` | Chunks, chunk streaming, draw data (what to render). |
| `voxel-core/src/camera.rs`, `math/` | Camera, AABB, math helpers. |
| `voxel-core/src/render*` | Renderer-agnostic render types (e.g. shader refs). |
| `backend/` | WASM crate. Glue to browser + wgpu rendering. Built via wasm-pack. |
| `backend/src/lib.rs` | wasm-bindgen entry, wires game loop to canvas. |
| `backend/src/core/` | Game loop, input, time. |
| `backend/src/graphics/` | Camera, texture loading. |
| `backend/src/world/` | World rendering (chunks → arena draws). |
| `backend/src/objects/` | Player, gizmo, renderers for scene objects. |
| `backend/src/gui/` | egui backend + FPS display. |
| `backend/src/utils/`, `math/` | Basic meshes, helpers. |
| `backend/src/testhooks.rs` | Hooks for e2e tests (feature `test-hooks`). |
| `backend/shaders/` | WGSL shaders. |

## Web shell

| Path | Role |
|---|---|
| `index.html`, `src/main.ts` | Loads WASM pkg, starts the app. |
| `src/shaders/`, `src/style.css` | Misc web-side assets. |
| `public/data/textures/` | Texture atlas and block textures. |
| `vite.config.ts`, `package.json` | Dev server and scripts (`dev`, `build`, `wasm`, `test`). |

## Tests

- `voxel-core` unit tests: `pnpm test` (native cargo).
- `e2e/`: Playwright pixel and shader tests, screenshots in `e2e/__screenshots__`. Needs `wasm:test` build.

## Docs / meta

- `.claude/state.md` — current state, TODOs, roadmap.
- `.claude/plan-*.md` — plans (wgpu migration, frustum culling, arena multidraw, ...).
- `codebase-review.md`, `bugs.md`, `TODO` — review notes and backlog.
- `.github/workflows/` — CI and GitHub Pages deploy.

Build outputs (ignore): `target/`, `dist/`, `backend/pkg/`, `node_modules/`, `test-results/`.
