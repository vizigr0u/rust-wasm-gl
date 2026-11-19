# Rendering profile harness

Measure the renderer under fixed scenarios, save a **baseline**, change something, measure again,
and get a noise-aware verdict per metric.

```bash
pnpm profile -- --baseline --label before          # -> profile/baselines/before.json (commit it)
# ...hack on the renderer...
pnpm profile -- --label after                      # -> profile/results/after.json (gitignored)
pnpm profile:compare profile/baselines/before.json profile/results/after.json
pnpm profile:compare profile/results/after.json    # summary of one run
```

Machine-comparable A/B for a past commit: `git stash` / checkout the baseline commit, run with
`--baseline`, go back, run again. Same machine, same session, idle box.

## How it works

| Piece | Role |
|---|---|
| `backend/src/testhooks.rs` (`test-hooks` feature) | `test_profile_step` (one frame -> CPU timings + counters), `test_gpu_idle` / `test_gpu_collect`, `test_set_toggle`, `test_move_player`, `test_profile_info` |
| `backend/src/render/webgpu/gpuprobe.rs` | GPU timestamp query around the main pass + submit->done wall clock, async readback |
| `profile/scenarios.mjs` | Scenario list (data only). Add one and it runs. |
| `profile/run.mjs` | Builds wasm, serves `dist/` with COOP/COEP, drives headless Chromium, writes JSON |
| `profile/compare.mjs` | Stats and verdicts |

Each scenario = fresh page, `default_spawn` scene, settle the world, apply toggles, 30 warmup
frames (dropped), then N measured frames at a fixed 16 ms virtual dt. Runs are interleaved
(run 1 of every scenario, then run 2, ...) so drift hits everything alike. Meshing is inline, so
`stream_walk` includes mesh cost in `script`.

## Metrics

Times in ms, per frame.

- CPU, from `performance.now()` inside wasm: `frame` (whole tick), `script` (input + world update
  + meshing, without uploads), `upload`, `ui`, `prepare` (draw list + per-frame buffers), `acquire`,
  `pass` (main pass encode), `egui`, `submit`.
- GPU: `gpu_pass` (timestamp queries around the main pass), `gpu_done` (submit -> work done,
  includes queue latency). One frame is in flight at a time (`test_gpu_idle` after each frame), so
  they are not polluted by a backlog. Not part of the CPU `frame` time.
- Counters (deterministic, must match exactly between two runs of the same code): `draw_calls`,
  `draw_entries_raw`, `chunks_visible`, `chunks_culled`, `quads_drawn`, `upload_bytes`,
  `chunk_buffers`, `arena_*`, `wasm_mem_mb`. A counter change in compare means behaviour changed,
  not just speed: check it is intended (e.g. an optim that culls more).

## Reading the verdict

Pooled frames of all runs are compared with a Mann-Whitney U test. A metric is `SLOWER` / `faster`
only if **p < 0.01** and the median shifted more than **max(3%, run-to-run noise)**, where noise is
the spread of per-run medians in either file. Metrics under 0.02 ms are ignored. `--fail-on-regress`
exits 1 when a headline metric (`frame script prepare pass submit gpu_pass gpu_done`) regressed.
Tune with `--min-effect PCT`, `--alpha P`, `--verbose` (show every row).

## Caveats

- **Default adapter is SwiftShader (software).** Reproducible, but `gpu_*` is a CPU rasteriser
  proxy: good for "did I issue less/cheaper GPU work" (fewer draws, fewer quads, less overdraw),
  useless for real-GPU questions like memory bandwidth or occlusion. `--gpu` tries the host GPU
  instead; those numbers are only comparable on that host. compare warns on adapter / CPU / viewport
  / wasm-profile mismatch.
- CPU timings are wasm in headless Chromium: relative changes are meaningful, absolutes are not
  what a user sees. 5 us timer resolution needs cross-origin isolation (the runner does that).
- Wasm is built with the `wasm:test` profile (dev + opt-level 3, debug assertions on).
- Run on an idle machine. Host `load1` is recorded in the JSON. Use `--runs 5` for small effects.
- Viewport defaults to 640x360 (`--size WxH`): SwiftShader fill cost scales with pixels.
- A scenario change invalidates baselines: names are matched, contents are not.
