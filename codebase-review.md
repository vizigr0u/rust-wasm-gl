# Codebase review: architecture, anti-patterns, smells

_2026-11-18. I read every file in `backend/src`, the shaders, the frontend shell, the build files and CI. I also ran `cargo test` (native) and `cargo clippy --target wasm32-unknown-unknown` on `backend`, and `cargo test` on `backend-tests`. Items already listed in [bugs.md](bugs.md) are not repeated. Paths are relative to the repo root._

Severity: **H** = shapes the whole codebase or blocks the roadmap (wgpu port, workers, editing). **M** = real cost, local fix. **L** = hygiene.

## TL;DR: top 10

| # | Sev | Finding | Section |
|---|---|---|---|
| 1 | H | No pure-logic core. `glow` and `web_sys` types reach everything, so the crate cannot compile natively. All in-crate `#[test]`s are dead code that never compiles or runs. | [A1](#a1-no-pure-logic-core-in-crate-tests-are-dead) |
| 2 | H | Module dependency cycles: `graphics` ↔ `world`, `graphics` ↔ `core`. There is no layering. | [A2](#a2-module-cycles-no-layering) |
| 3 | H | Game state lives inside render resources. The player's position only exists after its mesh was first rendered. | [A3](#a3-game-state-is-owned-by-render-resources) |
| 4 | H | Chunk pipeline: everything runs on the main thread, the mesher can't see neighbours, every change re-uploads one global buffer, and each quad takes 16 B when it needs 4 B. | [A6](#a6-chunk-pipeline-does-not-scale) |
| 5 | H | Memory: dense 4 KB chunks everywhere. About 47 MB of page pool is preallocated at startup, about 75 % of it is known-empty air. Meshes outlive their block data. | [A7](#a7-memory-model) |
| 6 | M | Streaming parameters disagree. Load distance 15 vs. about 6-9 chunks actually streamed. A cube-shaped load and a sphere-shaped unload probably cause remesh churn. | [A8](#a8-streaming-parameters-contradict-each-other) |
| 7 | M | Face geometry and the bit-packing layout each have 3-4 independent sources of truth (Rust ×3, GLSL). | [A9](#a9-multiple-sources-of-truth-for-face-geometry-and-packing) |
| 8 | M | GL state is ad hoc. The player cube renders only because the world bound its atlas just before it (hidden coupling). | [A10](#a10-gl-state-management-is-ad-hoc) |
| 9 | M | CI deploy workflow is broken: npm vs pnpm, deprecated actions, Node too old, double wasm build. No tests or clippy in CI. | [A14](#a14-build-ci-tooling) |
| 10 | M | Errors are `String` + `JsValue` + panics in the render path. | [A13](#a13-error-handling) |

---

## Architecture

### A1. No pure-logic core; in-crate tests are dead
**H**

- `cargo test` in `backend` fails with 18 errors. `glow::Web*Key`, `from_webgl2_context` and `tex_image_*_with_html_*` only exist on wasm32.
- So these tests have **never run** since they were written:
  - [camera.rs:77](backend/src/graphics/camera.rs#L77)
  - [aabb.rs:44](backend/src/math/aabb.rs#L44)
  - [player.rs:101](backend/src/objects/player.rs#L101)
  - [newworldgenerator.rs:52](backend/src/world/newworldgenerator.rs#L52)
- The generator test would not compile even natively: `IVec3` is not imported ([newworldgenerator.rs:60](backend/src/world/newworldgenerator.rs#L60)).
- [backend-tests](backend-tests/src/lib.rs) works around this with `#[path = "../../backend/src/world/position.rs"]`. It only reaches `position.rs`, because any file that imports `crate::…` breaks the trick. 20 tests pass there. That is the entire real test coverage.
- Root cause: domain code (positions, chunks, meshing, streaming, generation, camera math) sits in the same crate as, and is entangled with, web and GL code.
  - `World::update` takes a `&glow::Context`.
  - `ShaderProgram` knows `ChunkPos`.
  - `Game::new()` touches the DOM (it creates `<img>` elements and subscribes listeners), so `Game` cannot be built in a test.

**Fix:** make it a cargo workspace.
- `voxel-core`: no `glow`, `web-sys` or `wasm-bindgen`. Holds positions, chunks, mesher, streamer, generator traits, camera math and AABB. Plain `cargo test` works, `backend-tests` and its `#[path]` hack go away, and this is also the crate a Web Worker or the wgpu port would reuse.
- `render-gl` (later `render-wgpu`): owns all `glow` calls.
- `app-web`: owns `wasm_bindgen(start)`, DOM, input and the loop.

[.claude/plan-wgpu-reference-tests.md](.claude/plan-wgpu-reference-tests.md) step 1 plans to cfg-gate web calls inside one crate. A crate split does the same job and also enforces the boundary afterwards.

### A2. Module cycles, no layering
**H**

| Edge | Where |
|---|---|
| `graphics` → `world` | `shaders.rs` uses `ChunkPos` ([shaders.rs:7](backend/src/graphics/shaders.rs#L7)), `mesh.rs` uses `BlockSideTexture` ([mesh.rs:6](backend/src/graphics/mesh.rs#L6)) |
| `world` → `graphics` | `chunk.rs` uses `Side` ([chunk.rs:5](backend/src/world/chunk.rs#L5)), `world.rs` uses `Camera`, `TextureType` |
| `graphics` → `core` | `Camera` implements `HandleInputs` over `InputState` and `InputEventType` (web events) ([camera.rs:4](backend/src/graphics/camera.rs#L4)) |
| `core` → `graphics`, `world`, `gui`, `objects` | `Game` |
| `utils` → `graphics`, `world` | `basicmeshes.rs` |

- Every `mod.rs` does `pub use x::*`. The namespace is flat, so these cycles are invisible.
- `Side` and `BlockSideTexture` are domain concepts, not graphics ones. Camera math is not input handling.
- Target layering: `math` → `world` (domain) → `render` → `app`. Input mapping (events to camera or player intents) belongs in `app`.

### A3. Game state is owned by render resources
**H**

[lazygameobject.rs](backend/src/objects/lazygameobject.rs), [player.rs](backend/src/objects/player.rs)

- `Player(LazyRenderGameObject { gameobject: Option<GameObject>, renderer_creator })`. The `GameObject` (position, velocity) is only created inside `render_lazy` on the first draw.
- Until then:
  - `get_position()` returns `WorldPos::ZERO`.
  - `handle_inputs` is a silent no-op.
  - The world streams around (0,0,0), not around the spawn point.
- The spawn position is stored in the *renderer factory* (`CreatePlayerRenderer { origin }`). `Gizmo` has an `origin` field that is only logged, never used ([gizmo.rs:39](backend/src/objects/gizmo.rs#L39)).
- `make_renderer(gl).unwrap()` means a shader compile error panics inside the render path ([lazygameobject.rs:45](backend/src/objects/lazygameobject.rs#L45)).
- `GameObject` bundles transform + velocity + `Rc<MeshRenderer>` + `Rc<TextureDef>`. Simulation and drawing can't be separated, for example to simulate without GL or to draw one mesh many times.

**Fix:**
- Make `Player { pos: WorldPos, vel: Vec3 }` plain data, created in `Game::new`.
- Create renderers eagerly when GL is ready, in a `Renderer` or `RenderAssets` struct, and draw by reference: `renderer.draw_cube(gl, &camera, player.model_matrix(origin))`.
- Drop `LazyRenderGameObject`, `MakeRenderer`, and the `Transform` getter/setter trait (Java-style, one implementor, mostly dead).

### A4. Late-init `Option`s instead of typestate
**M**

- `Game.egui: Option<EguiBackend>`, `WorldRenderData.graphics / vertex_array / *_buffer: Option<…>`, and `self.loaded_textures[0]` (a magic index, [game.rs:132](backend/src/core/game.rs#L132)). These exist because the state enum `Loading | WaitingForTextures | Running` carries no data.
- Every render call then re-checks `if let Some(...)`, and nothing stops calling `world.render` before `setup_graphics`.

**Fix:** let the enum own its state: `enum App { Loading(Loader), Running(RunningGame) }`, where `RunningGame` holds non-optional `egui`, `WorldRenderer` and `Textures { blocks, uv_checker }`.

### A5. World mixes simulation and rendering
**M**

[world.rs](backend/src/world/world.rs)

- `World` owns `render_data: WorldRenderData`.
- `update()` takes `gl` and uploads buffers.
- `on_chunk_changed(_gl)` and `update(_time)` carry unused parameters.
- `Game::render` ticks the texture loader ([game.rs:259](backend/src/core/game.rs#L259)), which is loading work, not rendering.

**Fix:** keep `World` as data plus streaming and meshing, producing "mesh dirty" events or a list of changed chunks. A separate `WorldRenderer` consumes those changes and owns the GPU buffers. That also makes the wgpu port a renderer swap.

### A6. Chunk pipeline does not scale
**H**

[chunkstreaming.rs](backend/src/world/chunkstreaming.rs), [world.rs](backend/src/world/world.rs), [worldrender.rs](backend/src/world/worldrender.rs), [chunk.vert](backend/shaders/chunk.vert)

1. **All on the main thread, in big lumps.**
   - `PAGE_LOAD_PER_FRAME = 1`, but one page is 720 chunks. 180 of them are in the generated band y ∈ [-2, 2], so about 737k `sin` evaluations land in a single frame. That is the likely hitch source.
   - Meshing runs on the same thread, up to 8 chunks per frame.
   - `generate(&mut self)` ([worldgenerator.rs:6](backend/src/world/worldgenerator.rs#L6)) takes `&mut`, which blocks parallel or worker generation. The trait also invites stateful RNG, which would make the world depend on load order. Use `&self` and hash or noise of `(seed, pos)`.
2. **The mesher can't see neighbours.** `ChunkDrawData::load(chunk: &Chunk)` ([worldrender.rs:44](backend/src/world/worldrender.rs#L44)) only sees one chunk, so border faces are always emitted (the open TODO). The API needs a neighbourhood view, for example a `Chunk` plus 6 neighbour slices, or a padded 18³ copy. Once editing exists, a block change must also dirty its neighbours.
3. **Global rebuild.**
   - `gather_draw_data` walks *every* loaded chunk, re-concatenates all quads into one `Vec`, and re-uploads the whole thing with `buffer_data(STATIC_DRAW)`. The cost is O(all quads) on every change and every chunk crossing.
   - `FRAME_RENDER_DELAY = 8` is a band-aid over that cost.
   - Fix: give each chunk its own buffer region (per-chunk VBO, or one arena with `bufferSubData`), and use per-chunk draws or `WEBGL_multi_draw`.
4. **16 B per quad when 4 B would do.** Every quad gets a repeated `Vec3` block origin (12 B) next to its 4 B packed word ([worldrender.rs:97](backend/src/world/worldrender.rs#L97)). Alternatives:
   - A per-chunk uniform with per-chunk draws.
   - A second packed int holding the chunk offset relative to the player.
   - The 11 spare bits in the existing word.
5. **The floating origin is done twice and cancels out.**
   - The CPU sends the *absolute* block origin as f32.
   - The shader computes `local - player_chunk*16 + world_pos` ([chunk.vert:54,91](backend/shaders/chunk.vert)).
   - This only works because every term is an integer, exact up to 2²⁴ blocks. The rebase is pointless as written: either send `(chunk - player_chunk)` from the CPU, which already regathers on chunk change, and drop the uniform, or drop the rebase.
6. **No frustum culling.** Draw order is `HashMap` iteration order, so it is random with no front-to-back ordering. The `HashMap` uses default SipHash where FxHash or ahash would be cheaper.

### A7. Memory model
**H**

- `Chunk` is always `[BlockType; 4096]` + `bool`, even when empty ([chunk.rs:40](backend/src/world/chunk.rs#L40)).
- A page is 6×20×6 = 720 chunks ≈ 2.9 MB. `ChunkStreamer::new` preallocates 16 of them (`vec![ChunkPage::default(); MAX_NUM_PAGES]`, [chunkstreaming.rs:96](backend/src/world/chunkstreaming.rs#L96)): **about 47 MB at startup**.
- The generator only fills y ∈ [-2, 2], so 15 of the 20 layers per page are known air. WASM linear memory never shrinks.
- **Fix:** `enum ChunkData { Empty, Uniform(BlockType), Dense(Box<[BlockType; N]>) }`, or a palette. Let page height follow the generated range, or stream column by column.
- **Ownership mismatch:** freeing a page returns its block data to the pool, but the meshes built from it stay in `World.chunks` ([chunkstreaming.rs:193](backend/src/world/chunkstreaming.rs#L193)). Block edits on a mesh with no data behind it will desync. Pick one owner per chunk: one `ChunkEntry { data, mesh_state }` map.
- `World.chunks: HashMap<ChunkPos, Option<ChunkDrawData>>`: since bug #5 it is never `None`, so the `Option` and the `.expect("added empty chunk")` ([world.rs:205](backend/src/world/world.rs#L205)) are vestigial. Empty chunks still get a map entry.

### A8. Streaming parameters contradict each other
**M**

- **Load distance vs. streamed area.** `MAX_LOAD_CHUNK_DISTANCE = 15`, but streaming only loads the 3×3 pages around the player's page, and a page is 6 chunks wide. Real horizontal reach is 6-9 chunks depending on where the player stands in its page.
- **Wasted recompute.** `make_offset_priority(15)` builds 29³ = 24,389 offsets, and every recompute filters all of them, most of which can never be streamed. Its `with_capacity(size³)` is also wrong: the count is `(2·size−1)³` ([world.rs:27](backend/src/world/world.rs#L27)).
- **Probable remesh churn** (from reading, not measured):
  - Load uses a cube (Chebyshev ≤ 14), unload uses a sphere (`dist² > 225`). About 17 Y layers × 18×18 streamed gives roughly 5.5k chunks, many beyond radius 15.
  - `on_chunk_changed` unloads all but 1024 of those, then **in the same call** rebuilds `chunks_to_load` from the cube. That re-queues the chunks it just unloaded, and they get re-meshed at 8 per frame.
  - This happens on every chunk crossing and every page load.
  - Fix: use one metric for both sides, plus hysteresis (unload radius > load radius).
- `MAX_MESH_TO_KEEP` means "keep up to 1024 *out-of-range* meshes", which is a surprising meaning for that name.

### A9. Multiple sources of truth for face geometry and packing
**M**

| Concept | Copies |
|---|---|
| Cube corners and per-side vertex order | `chunk.rs` `SIDE_VERTICES` (0..1 cube), `basicmeshes.rs` `SIDE_VERTICES` (−1..1 cube), two separate `SideVertices` structs (`chunk.rs:376`, `mesh.rs:121`), and the `switch(face)` in `chunk.vert` |
| Side enum order | `Side` in `mesh.rs`, `SideUp…SideLeft` constants and `NORMALS[]` in `chunk.vert` |
| Bit layout `ttttttsss zzzzyyyyxxxx` | `make_side_quad_data` ([chunk.rs:277](backend/src/world/chunk.rs#L277)) and the shader unpack ([chunk.vert:48](backend/shaders/chunk.vert#L48)), plus the dead `_generate_mesh_old` with a *different* layout |
| Attribute locations | Hardcoded `0/1/2` in `worldrender.rs` and `layout(location=…)` in GLSL, while every other shader goes through `ShaderDef` name lookup. Two parallel binding mechanisms. |
| Chunk size 16 | `CHUNK_SIZE`, the shader literal `16.0f`, and `& 15` masks, guarded only by a `debug_assert!` on a constant (clippy flags it) |

**Fix:**
- Keep one `quad_packing` module with `pack` and `unpack` plus a round-trip test.
- Generate the GLSL constants from Rust (`build.rs` or `format!` prepended to the shader source, e.g. `#define CHUNK_SIZE`).
- Keep one canonical cube definition.

### A10. GL state management is ad hoc
**M**

- **Hidden coupling.**
  - The player's `GameObject` gets `TextureDef::default()`, a null key used as a sentinel ([lazygameobject.rs:46](backend/src/objects/lazygameobject.rs#L46), [gameobject.rs:92](backend/src/objects/gameobject.rs#L92)), so it binds no texture.
  - `cube.frag` samples a `sampler2DArray`. It gets the block atlas only because `World::render` bound it just before.
  - Reordering the draws, or adding any draw in between, breaks the player.
- **Three state mechanisms.**
  - `MeshRenderer` flag slices, which issue 3 enable/disable calls per draw.
  - Raw `gl.enable` in `WorldRenderData::render` and `tick_waiting_for_textures`.
  - A `GlState` struct with three fields that are never read (it is just a namespace for `set_flag`).
  - Fix: one small state cache, `struct GlCache { depth, cull, blend, program, vao, tex }`, that skips redundant calls.
- **egui churn.**
  - `paint()` calls `set_mesh` per primitive per frame. That creates and deletes a VAO and 2 buffers each time (the leak is fixed, the churn is not).
  - It also converts `egui::Vertex` (20 B, u8 colours) into 8 f32s (32 B) and clones the index `Vec`.
  - Projection and blend func are set again for each primitive.
  - Fix: one persistent VAO and buffers, `buffer_data` into them, and upload egui vertices raw with a normalized `UNSIGNED_BYTE` colour attribute.
- `MeshRenderer` only supports f32 attributes (`vertex_attrib_pointer_f32(…, glow::FLOAT, …)` with a hardcoded `4 *` stride). The `VertexDataType` enum ([mesh.rs:9](backend/src/graphics/mesh.rs#L9)) exists but is never used.
- Every object compiles its own program, with no program cache. `Rc<MeshRenderer>` and `Rc<ShaderProgram>` are never actually shared.

### A11. Texture loading design
**M**

[textureloader.rs](backend/src/graphics/textureloader.rs)

- It uses a pool of hidden DOM `<img>` elements, plus **process-global `static Mutex<Vec<usize>>` queues** keyed by pool index, plus per-frame polling. Two `TextureLoader`s would share those globals, and nothing ties a message to a load generation.
- `wasm-bindgen-futures` is already a dependency, and unused. With `fetch` + `createImageBitmap` + `texImage*D(ImageBitmap)` in an `async fn`, the pool, the messaging, the `Busy/Idle` state machine and the DOM container all go away.
- **Ownership:**
  - `load()` creates a GL texture and hands out the key, but the loader keeps it in `slot.key`. Reusing a slot overwrites it, and nothing ever deletes textures.
  - `onload`/`onerror` handlers are never cleared (`set_onload(None)`), while the closures they point at get dropped when the slot goes `Idle`.
- `generate_mipmap` runs, but `MIN_FILTER = NEAREST`, so the mipmaps are never sampled. Use `NEAREST_MIPMAP_LINEAR` for distant voxels.
- `Rc<RefCell<Closure>>`: the `RefCell` is never mutably borrowed.
- The error message `"Invalid texture index"` actually means the pool is full.
- `if let Some(idle) = find(Idle) { if matches!(idle.status, Idle) {…} else {…} }`: the inner check is always true.

### A12. Input layer
**M**

[inputs.rs](backend/src/core/inputs.rs)

- **DOM events stored past dispatch.**
  - `Rc<web_sys::KeyboardEvent>` values are queued and processed in the next rAF.
  - So `event.prevent_default()` for F2 ([game.rs:212](backend/src/core/game.rs#L212)) runs after dispatch and has no effect.
  - Decide `preventDefault` inside the listener, and convert to a plain Rust enum (`Key(Code, pressed)`, `MouseMove{dx,dy}`) at the boundary. Then input logic can be tested natively too (see A1).
- **Per-frame cloning.** `get_inputs()` clones the whole `InputState` every frame: the `HashSet<String>` and the event `Vec`. Use `std::mem::take` on the events, and an enum or bitset for key codes instead of a `String` per event.
- **Closure storage.** `Vec<Arc<Mutex<Closure>>>` in single-threaded wasm (clippy: "Arc not Send/Sync"). `gloo` (with `gloo-events`) is a dependency, and unused. `gloo::events::EventListener` does this with RAII and also unsubscribes on drop.
- **DOM lookups in hot paths.**
  - `get_canvas()` (`getElementById`) runs every frame in `sync_canvas_size` and in `EguiBackend::update_size`.
  - `mouse_event_to_pos2` calls `getElementById` + `getBoundingClientRect` **per mouse event**, which can force layout.
  - Fix: cache the canvas handle, and refresh the rect on resize.
- The manual `Clone` impls for `InputEventType` and `InputState` are exactly what `#[derive(Clone)]` produces.

### A13. Error handling
**M**

- `Result<_, String>` and `Result<_, JsValue>` are mixed, with `?` converting between them by accident of `From<String> for JsValue`.
- `TextureLoader::tick` builds errors by concatenating strings with `format!("{}, …", errors)`.
- There are 29 `unwrap`/`expect` calls, several in per-frame paths:
  - `performance_now()` ([utils.rs:92](backend/src/utils/utils.rs#L92))
  - egui `set_mesh(...).expect` ([eguibackend.rs:155](backend/src/gui/eguibackend.rs#L155))
  - `create_texture().expect` in two places
  - `request_animation_frame` unwraps
  - `ChunkDrawData::load(...).expect`, on a function that can never fail
- The rAF closure returns `Result<(), JsValue>` but always returns `Ok`, and rAF ignores the value anyway.
- **Fix:** one `enum EngineError` (`thiserror`) or `anyhow` in the app layer, no panics after init, and infallible functions return plain values.

### A14. Build, CI, tooling
**M**

- **CI is broken.** [github-page.yaml](.github/workflows/github-page.yaml):
  - It uses `npm install` and `cache: 'npm'`, but the repo is pnpm-only with no `package-lock.json`, so setup-node fails.
  - `upload-pages-artifact@v1`, `deploy-pages@v1` and `configure-pages@v3` are deprecated. v1 depends on `upload-artifact@v3`, which GitHub has disabled.
  - Node 20.5 is below the minimum for Vite 7 and 8.
  - `npm run wasm-release` is immediately followed by `npm run build`, which runs `wasm-release` again.
  - `wasm-pack` is installed via npm.
- **No quality gate.** CI runs no `cargo test` (on either crate), no `clippy` and no `fmt`.
- **Misleading perf numbers.** `pnpm dev` builds the wasm with `opt-level = 0`, which makes meshing and generation 10-50× slower than release. Add `[profile.dev] opt-level = 1` (or `[profile.dev.package."*"] opt-level = 3`) so dev frame times mean something. Release uses `opt-level = "s"`, but a game probably wants `3`.
- **Unused dependencies:** `gloo`, `tracing` (only `tracing-wasm` is initialised, and nothing emits spans), `getrandom` (fastrand's `js` feature covers it), `js-sys` and `wasm-bindgen-futures`. Two logging systems are initialised (`tracing_wasm` and `console_log`).
- **Logging volume:** `console_log` runs at `Level::Trace` in every build, and `info!` fires on hot-ish paths: every page fill, chunk recompute, bounds update and texture message.
- `vite-plugin-wasm-pack@0.1.12` is old. Since `main.ts` imports `../backend/pkg` by relative path and the scripts already run `wasm-pack`, the plugin is probably redundant. Check, and remove it if so.

---

## Rust anti-patterns

| Pattern | Where | Better |
|---|---|---|
| `impl TryInto<Event> for &InputEventType` | [eguibackend.rs:333](backend/src/gui/eguibackend.rs#L333) | Implement `TryFrom`, and `TryInto` comes for free. Implementing `TryInto` directly is explicitly discouraged by std. |
| `_ => Err(ConversionError::Unhandled)?` is unreachable (compiler warns), and `Err(x)?` is used in place of `return Err(x)` | [eguibackend.rs:380](backend/src/gui/eguibackend.rs#L380), [:411](backend/src/gui/eguibackend.rs#L411) | Drop the arm, and use `return Err(..)` |
| Validating or lossy conversions as `From`, checked only by `debug_assert!` (release silently builds invalid values) | `From<IVec3> for ChunkPos/BlockPos/PageChunkOffset`, `From<usize> for PageIndex`, `From<ChunkPos> for PagePos` (a projection) ([position.rs:261-329](backend/src/world/position.rs#L261-L329)) | Use `TryFrom` or `new() -> Option`, and named methods such as `chunk.page()` and `block.chunk()` |
| `From<u8> for BlockType` maps unknown values to `Empty`, and there is no `#[repr(u8)]` although the value is bit-packed | [chunk.rs:22](backend/src/world/chunk.rs#L22) | Add `#[repr(u8)]` plus `TryFrom<u8>` |
| Manual `Hash` and `Clone` that match the derived ones | `Hash for ChunkPos` ([position.rs:79](backend/src/world/position.rs#L79)), `Clone for InputEventType/InputState` | `#[derive]` |
| Trait bounds on struct definitions | `World<G: WorldGenerator>`, `ChunkStreamer<G>`, `LazyRenderGameObject<R: MakeRenderer>` | Put bounds on the `impl`s only (API guideline C-STRUCT-BOUNDS). Or use `Box<dyn WorldGenerator>`, since the generic only buys a type alias in `game.rs`. |
| Tuple type aliases as domain types | `TextureDef = (TextureType, WebTextureKey)`, `BlockSideTextures = (T, T, T)` with `From<BlockType>` on a tuple, `QuadSideData`, `ChunkSideData` (unused) | Named structs, or `fn face_texture(self, Side) -> BlockSideTexture` |
| `Rc` around `Copy` data | `Rc<TextureDef>`, `Rc<(TextureType, WebTextureKey)>` | Pass the key by value |
| Sentinel default instead of `Option` | `key != WebTextureKey::default()` ([gameobject.rs:92](backend/src/objects/gameobject.rs#L92)) | `Option<Texture>` |
| `Arc<Mutex<…>>` and `static Mutex` in single-threaded wasm | `inputs.rs:125`, `textureloader.rs:261` | Own the value, `Rc<RefCell>` if it must be shared, or `thread_local!` |
| `pub unsafe fn` with no safety contract, and unsafe blocks around whole function bodies (34 `unsafe` total) | `ShaderDef::compile`, `compile_shader`, `link_program`, `MeshRenderer::set_mesh`, the world and egui render paths | A thin safe GL wrapper layer with narrow `unsafe` and `// SAFETY:` comments. `glow` WebGL calls are nominally unsafe, so the wrapper is the place to contain that. |
| Unnecessary `unsafe` block (compiler warns) | [eguibackend.rs:97](backend/src/gui/eguibackend.rs#L97) | Remove it |
| Public fields that break invariants | `Chunk.blocks` is `pub` and bypasses `set()`, which maintains `is_empty_flag`. `Camera.target/projection/look_at` are `pub`. | Private fields plus accessors |
| `get_` prefix on getters | `get_position`, `get_info`, `get_events`, `get_data`, `get_program`… | `position()`, `info()` (API guideline C-GETTER) |
| `&Vec<T>` in signatures | `get_events() -> &Vec<_>`, `Mesh::get_data`, `set_events(&Vec<_>)` | `&[T]` |
| `pub use x::*` in every `mod.rs`, with `pub` everywhere in a crate that exports nothing | all modules | Explicit re-exports and `pub(crate)`. Dead code then shows up under lints. |
| Module inception, typos | `utils/utils.rs`, `world/world.rs`, `graphics/texure.rs` | Rename |
| `#![allow(dead_code, unused_variables)]` at file level | [mesh.rs:1](backend/src/graphics/mesh.rs#L1), [basicmeshes.rs:1](backend/src/utils/basicmeshes.rs#L1) | Delete the dead code |
| `#[macro_export]` in a binary-ish cdylib | `shader_def!` | `macro_rules!` plus `pub(crate) use shader_def;` |
| `Into::<WorldPos>::into(x) + v`, `Into::<BlockType>::into(..)` | position.rs, chunk.rs | `WorldPos::from(x)` |
| `bool as i32 as f32` | [player.rs:93](backend/src/objects/player.rs#L93) | `f32::from(u8::from(b))`, or a match |
| `Rc<RefCell<glow::Context>>` that is only ever borrowed immutably | [lib.rs:53](backend/src/lib.rs#L53) | Move `gl` into the closure |
| `HashMap<K, Option<V>>` where `None` never occurs | `World.chunks` | `HashMap<K, V>` |
| `#[derive(Debug)]` on structs holding 4 KB to 3 MB arrays, required transitively by `#[derive(Debug)] Game` | `Chunk`, `ChunkPage`, `ChunkStreamer` | Manual `Debug` that summarises, or drop `Debug` from `Game` |
| `Time` as raw `f64` milliseconds, with `* 0.001` and `MAX_DELTA_MS` scattered | `time.rs`, `gameobject.rs:68` | Seconds as `f32` (or `Duration`), clamped once in `Time::update` |

## Code smells

- **Commented-out code.** About 170 comment lines in `world.rs`, `utils.rs`, `camera.rs`, `chunk.rs` and `eguibackend.rs`, including a 25-line egui demo at the top of the backend. Git has history; delete it.
- **Debug leftovers.**
  - `if true { … }` ×3 in `WorldRenderData::setup_graphics`.
  - The `const TEST_ONE_CUBE: bool = false` branch in `to_vertex_data`.
  - `chunk.frag` and `cube.frag` assign `outColor` three times. Only the last assignment counts.
  - `cube.vert` writes `o_normal`, which nothing reads.
- **Dead code that clippy lists:**
  - `TestGenerator`, `EmptyWorldGenerator`, `RandomChunkGenerator`, `Chunk::new/random/plain`.
  - `FiveBitsPerInt`, `FourBitsPerInt` (empty structs), `pack_ivec_5bits`, `OPTIMIZATION_LEVEL`, `_generate_mesh_old`.
  - `UniformTypes::WorldPosition`, `GlState` fields, the `GeneratorV0.rng` field.
  - Unused `basicmeshes` (`make_quad*`), the `textureTransform` and `vertColor` shaders, and `src/shaders/phong.frag` on the TS side.
  - Either move the fixtures into `#[cfg(test)]` (once A1 makes tests run) or delete them.
- **Misleading names.**
  - `newworldgenerator.rs` holds `GeneratorV0`.
  - `testworldgenerator.rs` is not a test.
  - `Camera.look_at` is the view matrix.
  - `Player::get_screen_position` returns the chunk-local offset.
  - The vertex attribute called `Depth` / `depth` is the texture array layer.
  - `MessageSystem` is two global vectors.
  - `is_chunked_streamed`.
- **Magic values.**
  - `loaded_textures[0]`.
  - Attribute locations `0/1/2`.
  - `TextureType::Texture2DArray(16)`, which must match the `BlockSideTexture` count and the atlas height.
  - `size: (800, 600)` initial egui size.
  - `e.buttons() % 2 == 1` for "left button held".
  - `egui::Id::new("my_area")`, which is deprecated (clippy).
- **Tautological test.** `diagonal_is_normalized` ([player.rs:105](backend/src/objects/player.rs#L105)) tests `glam::normalize_or_zero`, not player code.
- **`Mesh::from_offset_sides`** reserves `size * 4 * 9` floats but pushes 6 vertices per side ([mesh.rs:59](backend/src/graphics/mesh.rs#L59)).

## Small bugs noticed along the way (not in bugs.md)

| Sev | Bug | Where |
|---|---|---|
| L | The gizmo and vertColor fragment shaders write `alpha = 0.0`. With the default `premultipliedAlpha: true` canvas, those pixels have colour > alpha, which is undefined compositing (they may show page background or wrong colours). | [gizmo.frag](backend/shaders/gizmo.frag), [vertColor.frag](backend/shaders/vertColor.frag) |
| L | egui gets no `pixels_per_point`, although the canvas buffer is CSS size × DPR. The UI renders at half size on HiDPI screens. egui gets no `time` either (the field is commented out), so animations and double-click timing are off. `EguiBackend.time` is stored but never read. | [eguibackend.rs:277](backend/src/gui/eguibackend.rs#L277) |
| L | `preventDefault` for F2 is called a frame late and does nothing (A12). | [game.rs:212](backend/src/core/game.rs#L212) |
| L | If the 2nd texture `load()` in `tick_loading` errors, the state stays `Loading`. The next frame loads the 1st texture again, pushes a duplicate into `loaded_textures`, and burns a pool slot each time. | [game.rs:104](backend/src/core/game.rs#L104) |
| L | `WorldPos + Vec3` has an f32 edge guard for x and z but not for y. A tiny negative `offset.y` can wrap to exactly `16.0`. | [position.rs:155](backend/src/world/position.rs#L155) |

---

## Suggested order

1. **Workspace split (A1, A2).** Move `position`, `chunk`, `aabb`, mesher, streamer, generator and camera math into `voxel-core`. Delete `backend-tests`. Fix the dead tests. This unblocks everything else, including the wgpu reference-test plan.
2. **CI (A14).** pnpm, current actions, `cargo test -p voxel-core`, `clippy -D warnings`, `fmt --check`. Add a dev `opt-level`.
3. **Decouple state from render (A3, A4, A5).** Plain-data `Player`, typestate `App`, and a `WorldRenderer` that consumes dirty chunks.
4. **Chunk data and pipeline (A7, A6, A8).** Sparse `ChunkData`, single ownership, neighbour-aware mesher, per-chunk GPU regions, one metric with hysteresis for load and unload. Then move generation and meshing to a worker. The `&self` generator and the pure core crate make that straightforward.
5. **Single source of truth (A9).** A shared packing module with a round-trip test, and generated shader constants.
6. **GL hygiene (A10, A11, A13).** State cache, persistent egui buffers, async `ImageBitmap` texture loading, a typed error enum.
7. **Sweep (Rust anti-patterns, code smells).** Mostly mechanical, `clippy --fix` plus deletion. Easiest once step 1 makes dead code visible.
