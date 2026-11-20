//! Deterministic scene driver for e2e tests (feature `test-hooks`).
//!
//! If `window.__RWG_TEST__` is set, `start()` skips the rAF loop and the spec
//! drives the game through the exported `test_*` functions. The JS API must stay
//! identical across backends so one spec runs against GL and wgpu.

use std::cell::RefCell;

use glam::Vec3;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::JsFuture;

use crate::{
    core::{Game, SceneConfig, SceneGenerator},
    render::Renderer,
    utils::get_canvas,
};

/// Virtual frame time, so runs do not depend on wall-clock speed.
const DEFAULT_DT_MS: f64 = 16.0;
/// Texture loading is async in the browser; give it ~30 s.
const MAX_LOAD_POLLS: u32 = 3000;

struct Harness {
    game: Game,
    renderer: Renderer,
    /// Virtual clock (ms).
    time: f64,
}

thread_local! {
    static HARNESS: RefCell<Option<Harness>> = const { RefCell::new(None) };
}

pub fn is_test_mode() -> bool {
    web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &"__RWG_TEST__".into()).ok())
        .is_some_and(|v| v.is_truthy())
}

fn scene_config(name: &str) -> Result<SceneConfig, JsValue> {
    match name {
        "default_spawn" => Ok(SceneConfig {
            show_overlay: false,
            frustum_culling: false,
            debug_overlays: false,
            ..SceneConfig::default()
        }),
        "single_block" => Ok(SceneConfig {
            generator: SceneGenerator::SingleBlock,
            show_player: false,
            show_gizmo: false,
            show_overlay: false,
            frustum_culling: false,
            debug_overlays: false,
            ..SceneConfig::default()
        }),
        other => Err(format!("unknown scene '{other}'").into()),
    }
}

fn with_harness<R>(f: impl FnOnce(&mut Harness) -> Result<R, JsValue>) -> Result<R, JsValue> {
    HARNESS.with(|h| match h.borrow_mut().as_mut() {
        Some(h) => f(h),
        None => Err("test_start() was not called".into()),
    })
}

fn tick(h: &mut Harness, dt_ms: f64) -> Result<(), JsValue> {
    h.time += dt_ms;
    h.game.tick(&mut h.renderer, h.time).map_err(JsValue::from)
}

async fn next_macrotask() -> Result<(), JsValue> {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(w) = web_sys::window() {
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 1);
        }
    });
    JsFuture::from(promise).await.map(|_| ())
}

/// Build the scene and run until the game is in `Stage::Running`
/// (textures in). Returns a Promise, because wgpu init is async.
#[wasm_bindgen]
pub async fn test_start(scene: String) -> Result<(), JsValue> {
    let config = scene_config(&scene)?;
    let renderer = Renderer::new(get_canvas()?)
        .await
        .map_err(|e| JsValue::from(e.to_string()))?;
    let game = Game::with_config(config)?;
    HARNESS.with(|h| {
        *h.borrow_mut() = Some(Harness {
            game,
            renderer,
            time: 0.0,
        })
    });

    for _ in 0..MAX_LOAD_POLLS {
        let running = with_harness(|h| {
            tick(h, DEFAULT_DT_MS)?;
            Ok(h.game.is_running())
        })?;
        if running {
            return Ok(());
        }
        next_macrotask().await?;
    }
    Err("timed out waiting for textures".into())
}

/// Run `frames` full frames (update + render) of `dt_ms` virtual time each.
#[wasm_bindgen]
pub fn test_step(frames: u32, dt_ms: f64) -> Result<(), JsValue> {
    with_harness(|h| {
        for _ in 0..frames {
            tick(h, dt_ms)?;
        }
        Ok(())
    })
}

/// Update-only frames (no draw calls) until settled or `max_frames`, then one
/// full frame so the canvas holds the final picture. Returns frames used.
#[wasm_bindgen]
pub fn test_settle(max_frames: u32, dt_ms: f64) -> Result<u32, JsValue> {
    with_harness(|h| {
        // is_settled can be true before the first streaming pass queued anything.
        const WARMUP: u32 = 30;
        let mut used = 0;
        h.game.set_skip_render(true);
        while used < max_frames && !(used >= WARMUP && h.game.is_settled()) {
            tick(h, dt_ms)?;
            used += 1;
        }
        h.game.set_skip_render(false);
        tick(h, dt_ms)?;
        Ok(used + 1)
    })
}

/// Running, streamer queue and chunks_to_load empty, last MeshChanges empty.
#[wasm_bindgen]
pub fn test_is_settled() -> Result<bool, JsValue> {
    with_harness(|h| Ok(h.game.is_settled()))
}

/// `target` is relative to the player's chunk; both are `[x, y, z]`.
#[wasm_bindgen]
pub fn test_set_camera(target: Vec<f32>, offset: Vec<f32>) -> Result<(), JsValue> {
    let v3 = |v: &[f32]| match v {
        [x, y, z] => Ok(Vec3::new(*x, *y, *z)),
        _ => Err(JsValue::from("expected [x, y, z]")),
    };
    let (target, offset) = (v3(&target)?, v3(&offset)?);
    with_harness(|h| {
        h.game.set_camera(target, offset);
        Ok(())
    })
}

/// Compile and link every shader program. Returns one message per failure
/// (empty array = all good). Needs `test_start` first (one context per canvas).
#[wasm_bindgen]
pub async fn test_compile_all_shaders() -> Result<Vec<String>, JsValue> {
    let check = with_harness(|h| Ok(h.renderer.compile_check()))?;
    Ok(check.await)
}

// ---------------------------------------------------------------------------
// Profiling API (driven by profile/profile.spec.ts). Same harness as the pixel tests.
// ---------------------------------------------------------------------------

/// Column order of `test_profile_step`'s result. Times are ms, the rest are counters.
/// GPU timings come separately from `test_gpu_collect`.
const PROFILE_FIELDS: [&str; 19] = [
    // CPU timings
    "frame",
    "script",
    "upload",
    "ui",
    "prepare",
    "acquire",
    "pass",
    "submit",
    // deterministic counters
    "draw_calls",
    "draw_entries_raw",
    "chunks_visible",
    "chunks_culled",
    "quads_drawn",
    "upload_bytes",
    "chunk_buffers",
    "arena_used",
    "arena_capacity",
    "arena_largest_free",
    "wasm_mem_mb",
];

#[wasm_bindgen]
pub fn test_profile_fields() -> Vec<String> {
    PROFILE_FIELDS.iter().map(|s| s.to_string()).collect()
}

/// One full frame of `dt_ms` virtual time. Returns the metrics in `PROFILE_FIELDS` order.
#[wasm_bindgen]
pub fn test_profile_step(dt_ms: f64) -> Result<Vec<f64>, JsValue> {
    with_harness(|h| {
        tick(h, dt_ms)?;
        let t = h.game.last_times();
        let s = h.renderer.stats();
        let mem_mb = js_sys::Reflect::get(&wasm_bindgen::memory(), &"buffer".into())
            .ok()
            .and_then(|b| b.dyn_into::<js_sys::ArrayBuffer>().ok())
            .map_or(f64::NAN, |b| b.byte_length() as f64 / 1048576.0);
        Ok(vec![
            t.frame,
            t.script,
            t.upload,
            t.ui,
            t.prepare,
            t.acquire,
            t.pass,
            t.submit,
            s.draw_calls as f64,
            s.draw_entries_raw as f64,
            s.chunks_visible as f64,
            s.chunks_culled as f64,
            s.quads_drawn as f64,
            s.upload_bytes as f64,
            s.chunk_buffers as f64,
            s.arena_used as f64,
            s.arena_capacity as f64,
            s.arena_largest_free as f64,
            mem_mb,
        ])
    })
}

/// Flip a switch: `frustum_culling`, `overlay`, `show_frustum`, `player_cam_view`.
#[wasm_bindgen]
pub fn test_set_toggle(name: String, on: bool) -> Result<(), JsValue> {
    with_harness(|h| {
        if h.game.set_toggle(&name, on) {
            Ok(())
        } else {
            Err(format!("unknown toggle '{name}'").into())
        }
    })
}

/// Move the player by `[x, y, z]` blocks (deterministic streaming load).
#[wasm_bindgen]
pub fn test_move_player(delta: Vec<f32>) -> Result<(), JsValue> {
    let [x, y, z] = delta[..] else {
        return Err("expected [x, y, z]".into());
    };
    with_harness(|h| {
        h.game.translate_player(Vec3::new(x, y, z));
        Ok(())
    })
}

/// Wait until the GPU finished the last frame and its timing readbacks landed.
/// Without this a slow adapter (SwiftShader) lets the CPU run ahead and queue frames up,
/// so GPU timings would include the backlog.
#[wasm_bindgen]
pub async fn test_gpu_idle() -> Result<(), JsValue> {
    for _ in 0..20_000 {
        if !with_harness(|h| Ok(h.renderer.gpu_busy()))? {
            return Ok(());
        }
        next_macrotask().await?;
    }
    Err("GPU did not go idle".into())
}

/// `[gpu_pass_ms, gpu_done_ms]` of the last frame (NaN = unavailable). Call after `test_gpu_idle`.
#[wasm_bindgen]
pub fn test_gpu_collect() -> Result<Vec<f64>, JsValue> {
    with_harness(|h| {
        let (pass, done) = h.renderer.collect_gpu();
        Ok(vec![pass, done])
    })
}

/// JSON object describing the environment of this run.
#[wasm_bindgen]
pub fn test_profile_info() -> Result<String, JsValue> {
    with_harness(|h| {
        let (adapter, timestamps) = h.renderer.gpu_report();
        let canvas = get_canvas()?;
        Ok(format!(
            "{{\"adapter\":{},\"gpu_timestamps\":{},\"canvas\":[{},{}],\"git\":\"{}\",\"opt_level\":\"{}\",\"debug_assertions\":{}}}",
            js_sys::JSON::stringify(&adapter.into())?.as_string().unwrap_or_default(),
            timestamps,
            canvas.width(),
            canvas.height(),
            env!("GIT_HASH"),
            if cfg!(debug_assertions) { "dev" } else { "release" },
            cfg!(debug_assertions),
        ))
    })
}
