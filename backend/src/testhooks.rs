//! Deterministic scene driver for e2e tests (feature `test-hooks`).
//!
//! If `window.__RWG_TEST__` is set, `start()` skips the rAF loop and the spec
//! drives the game through the exported `test_*` functions. The JS API must stay
//! identical across backends so one spec runs against GL and wgpu.

use std::cell::RefCell;

use glam::Vec3;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::{
    core::{Game, SceneConfig, SceneGenerator},
    gui::EguiBackend,
    objects::{compile_cube_shader, compile_gizmo_shader},
    utils::get_webgl2_context,
    world,
};

/// Virtual frame time, so runs do not depend on wall-clock speed.
const DEFAULT_DT_MS: f64 = 16.0;
/// Texture loading is async in the browser; give it ~30 s.
const MAX_LOAD_POLLS: u32 = 3000;

struct Harness {
    game: Game,
    gl: glow::Context,
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
            show_egui: false,
            ..SceneConfig::default()
        }),
        "single_block" => Ok(SceneConfig {
            generator: SceneGenerator::SingleBlock,
            show_player: false,
            show_gizmo: false,
            show_egui: false,
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
    h.game.tick(&h.gl, h.time).map_err(JsValue::from)
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
    let gl = get_webgl2_context()?;
    let game = Game::with_config(config)?;
    HARNESS.with(|h| {
        *h.borrow_mut() = Some(Harness {
            game,
            gl,
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
/// (empty array = all good).
#[wasm_bindgen]
pub fn test_compile_all_shaders() -> Result<Vec<String>, JsValue> {
    let gl = match with_harness(|_| Ok(())) {
        // Reuse the harness context so we do not create a second one on the canvas.
        Ok(()) => None,
        Err(_) => Some(get_webgl2_context()?),
    };
    let run = |gl: &glow::Context| {
        let mut errors = Vec::new();
        let mut check = |name: &str, r: Result<(), String>| {
            if let Err(e) = r {
                errors.push(format!("{name}: {e}"));
            }
        };
        check("chunk", world::compile_shader(gl).map(|_| ()));
        check("cube", compile_cube_shader(gl).map(|_| ()));
        check("gizmo", compile_gizmo_shader(gl).map(|_| ()));
        check("egui", EguiBackend::compile_shader(gl).map(|_| ()));
        errors
    };
    match gl {
        Some(gl) => Ok(run(&gl)),
        None => with_harness(|h| Ok(run(&h.gl))),
    }
}
