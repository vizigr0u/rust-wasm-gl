## What is this?

trying to make a little game in Rust WASM + WebGL2.

uses vite + Typescript (for some reason all rust WASM doc uses webpack but I am stubborn I wanted Vite)

Game (in the backend directory): Rust compiled to WASM

uses glam for maths and web-sys for JS bindings. I still have gloo in my dependencies but not sure I'll end up using it.

Github page: <https://vizigr0u.github.io/rust-wasm-gl/>

As of 2024-03-29:

- basic game engine structure (not final): Game, GameObject, Mesh, Shader, Renderer, Time, Camera, InputSystem.
- voxel world with "streaming" and "generation".

![alt text](image.png)

For now as a proof of concept, I'm using textures extracted from a very famous game. I will change these very soon, please don't sue me.

I don't want to bother with JS/WASM interaction too much so everything is handled on the WASM (Rust) side.
I might experiment at some point to check whether request-animation-frame can be better off left on the JS side and call the loop on the WASM side.

Web page : just a canvas, the game engine for now just shows img containing textures as they get loaded, for debug purposes.

## Getting Started

```bash
git clone https://github.com/vizigr0u/rust-wasm-gl
pnpm install

# compile Rust into WASM
pnpm wasm

# start local web server
pnpm dev

# or build
pnpm build
```


## Testing

```bash
# run the logic tests (voxel-core)
pnpm test

# snapshot tests use insta; review and accept changed snapshots
cargo install cargo-insta   # once
cargo insta test -p voxel-core --review
```

### Pixel tests (Playwright)

Deterministic scenes are rendered in headless Chromium (SwiftShader) and compared to golden PNGs in `e2e/__screenshots__/<backend>/`. The WASM is built with the `test-hooks` cargo feature, which exposes `test_start`, `test_step`, `test_is_settled`, `test_set_camera` and `test_compile_all_shaders` instead of running the rAF loop.

```bash
pnpm exec playwright install --with-deps chromium   # once

# build wasm with test-hooks, bundle, serve, run specs
pnpm test:pixels

# regenerate goldens after an intended visual change (review the PNG diff in git!)
pnpm test:pixels:update

# only rebuild the test-hooks wasm
pnpm wasm:test
```

Env vars:

- `BACKEND=gl|webgpu`: picks Chromium flags and the golden dir (default `gl`). Only `gl` goldens exist so far.
- `PIXEL_TIER=strict|loose`: strict is `threshold 0.1` and `0.1%` differing pixels; loose allows 2%.
- `GOLDEN_DIR=gl`: compare a `webgpu` run against the GL goldens.

On failure, diffs and traces land in `test-results/` (CI uploads it as an artifact).

### Backend feature flags

In `backend/Cargo.toml`:

- `test-hooks`: deterministic scene driver for the e2e tests. Never enable it in release builds.
- `webgpu`: not there yet (planned wgpu renderer, see `.claude/plan-wgpu-migration.md`). GL is the only backend today.

### Git hook

Pre-commit runs `cargo fmt --all --check`. Enable once per clone:

```bash
git config core.hooksPath .githooks
```

### CI

`.github/workflows/ci.yaml`: `rust` job (fmt, test, clippy) then `e2e` job (Playwright pixel tests, GL only).
