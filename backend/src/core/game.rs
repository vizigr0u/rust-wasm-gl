use fastrand::Rng;
use glam::{vec3, Vec3};
use voxel_core::{gpu::UploadQueue, math::Frustum};
use wasm_bindgen::JsValue;

use crate::{
    graphics::{Camera, TextureLoader},
    gui::{DebugOverlay, FrameStats, FrameTimes},
    objects::{gizmo_model, player_model, Player},
    render::{FrameInput, RenderStats, Renderer, TextureId},
    utils::{
        performance_now,
        trace::{self, Track},
    },
    world::{
        Chunk, ChunkPos, GeneratorV0, InlineMesher, MeshExecutor, SingleBlockGenerator, World,
        WorldGenerator,
    },
};

use std::cell::RefCell;

use wasm_bindgen::prelude::wasm_bindgen;

use super::{meshworker::WorkerMesher, HandleInputs, InputEventType, InputSystem, Time};

/// Mesh bytes written to the GPU arena per frame (at least one chunk always goes through).
const UPLOAD_BUDGET_BYTES: usize = 64 * 1024;

const BLOCKS_ATLAS_PATH: &str = "data/textures/blocks/blocks_atlas.png";
const UV_CHECKER_PATH: &str = "data/textures/UV_checker_Map_byValle.jpg";

#[derive(Debug)]
enum Generator {
    V0(GeneratorV0),
    SingleBlock(SingleBlockGenerator),
}

impl WorldGenerator for Generator {
    fn generate(&self, pos: ChunkPos) -> Chunk {
        match self {
            Generator::V0(g) => g.generate(pos),
            Generator::SingleBlock(g) => g.generate(pos),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(feature = "test-hooks"), allow(dead_code))]
pub enum SceneGenerator {
    V0,
    SingleBlock,
}

/// Everything that varies between the normal game and a deterministic test scene.
#[derive(Debug, Clone)]
pub struct SceneConfig {
    pub generator: SceneGenerator,
    pub seed: u64,
    pub camera_offset: Vec3,
    pub show_player: bool,
    pub show_gizmo: bool,
    pub show_overlay: bool,
    /// Initial state of the culling toggle and the frustum/culled-box overlays.
    pub frustum_culling: bool,
    pub debug_overlays: bool,
}

impl Default for SceneConfig {
    /// The `default_spawn` scene, which is also the real game.
    fn default() -> Self {
        Self {
            generator: SceneGenerator::V0,
            seed: 0,
            camera_offset: vec3(-10.0, 5.0, 0.0),
            show_player: true,
            show_gizmo: true,
            show_overlay: true,
            frustum_culling: true,
            debug_overlays: true,
        }
    }
}

/// Pause-menu and F4 debug switches.
#[derive(Debug, Clone, Copy)]
struct DebugToggles {
    /// Render from the player's view instead of the orbit camera.
    player_cam_view: bool,
    frustum_culling: bool,
    show_frustum: bool,
    show_culled_boxes: bool,
    /// Frame-time graph in the debug overlay.
    show_graph: bool,
}

thread_local! {
    /// Toggles requested from the DOM overlay, drained at the start of each tick.
    static PENDING_TOGGLES: RefCell<Vec<(String, bool)>> = const { RefCell::new(Vec::new()) };
}

/// Called by `src/debug-overlay.ts` when the user flips a checkbox or the graph button.
#[wasm_bindgen]
pub fn debug_set_toggle(name: String, on: bool) {
    PENDING_TOGGLES.with(|q| q.borrow_mut().push((name, on)));
}

fn make_generator(config: &SceneConfig, _rng: Rng) -> Generator {
    match config.generator {
        SceneGenerator::V0 => Generator::V0(GeneratorV0::new()),
        SceneGenerator::SingleBlock => Generator::SingleBlock(SingleBlockGenerator),
    }
}

#[derive(Debug)]
enum Stage {
    Loading,
    WaitingForTextures,
    Running,
}

#[derive(Debug)]
pub struct Game {
    seed: u64,
    stage: Stage,
    texture_loader: TextureLoader,
    world: World<Generator>,
    /// Meshes built but not yet written to the GPU.
    upload_queue: UploadQueue,
    camera: Camera,
    config: SceneConfig,
    /// The last `World::update` produced no mesh changes (and they were applied).
    last_changes_empty: bool,
    /// Test hooks place the camera by hand; skip the follow-the-player retarget.
    camera_pinned: bool,
    /// Test hooks: run update only, skip draw calls (set while settling).
    skip_render: bool,

    input_system: InputSystem,
    time: Time,
    is_paused: bool,
    gui_state: GuiState,
    toggles: DebugToggles,
    player: Player,
    frame_stats: FrameStats,
    overlay: DebugOverlay,
    chart_buf: Vec<f64>,
    /// ms spent in chunk upload/remove during this tick's `update`.
    upload_ms: f64,
    /// ms spent updating the DOM overlay; reported with the next tick (it runs after the stats).
    ui_ms: f64,
    /// Timings of the last running tick (profiling hooks read these).
    last_times: FrameTimes,
}

#[derive(Debug)]
struct GuiState {
    show_info: bool,
}

impl Default for GuiState {
    fn default() -> Self {
        Self { show_info: true }
    }
}

impl Game {
    pub fn new() -> Result<Self, JsValue> {
        let mesher: Box<dyn MeshExecutor> = match WorkerMesher::from_window() {
            Some(w) => Box::new(w),
            None => Box::<InlineMesher>::default(),
        };
        Self::with_mesher(SceneConfig::default(), mesher)
    }

    /// Meshes inline: deterministic for tests.
    #[cfg_attr(not(feature = "test-hooks"), allow(dead_code))]
    pub fn with_config(config: SceneConfig) -> Result<Self, JsValue> {
        Self::with_mesher(config, Box::<InlineMesher>::default())
    }

    fn with_mesher(config: SceneConfig, mesher: Box<dyn MeshExecutor>) -> Result<Self, JsValue> {
        let seed = config.seed;
        let rng = Rng::with_seed(seed);
        let toggles = DebugToggles {
            player_cam_view: false,
            frustum_culling: config.frustum_culling,
            show_frustum: config.debug_overlays,
            show_culled_boxes: false,
            show_graph: false,
        };
        let game = Game {
            seed,
            stage: Stage::Loading,
            texture_loader: TextureLoader::new(10)?,
            upload_queue: UploadQueue::default(),
            world: World::with_mesher(make_generator(&config, rng), mesher),
            camera: Camera::new(config.camera_offset),
            config,
            last_changes_empty: false,
            camera_pinned: false,
            skip_render: false,
            input_system: InputSystem::new()?,
            is_paused: false,
            time: Time::default(),
            gui_state: GuiState::default(),
            toggles,
            player: Player::new(vec3(0.0, 16.0, 0.0)),
            frame_stats: FrameStats::new(),
            overlay: DebugOverlay::new(),
            chart_buf: Vec::new(),
            upload_ms: 0.0,
            ui_ms: 0.0,
            last_times: FrameTimes::default(),
        };

        Ok(game)
    }

    #[cfg_attr(not(feature = "test-hooks"), allow(dead_code))]
    pub fn is_running(&self) -> bool {
        matches!(self.stage, Stage::Running)
    }

    /// Running, and nothing is left to stream, mesh or upload.
    #[cfg_attr(not(feature = "test-hooks"), allow(dead_code))]
    pub fn is_settled(&self) -> bool {
        self.is_running() && self.world.is_settled() && self.last_changes_empty
    }

    #[cfg_attr(not(feature = "test-hooks"), allow(dead_code))]
    pub fn set_skip_render(&mut self, skip: bool) {
        self.skip_render = skip;
    }

    #[cfg_attr(not(feature = "test-hooks"), allow(dead_code))]
    pub fn last_times(&self) -> FrameTimes {
        self.last_times
    }

    /// Flip a debug/scene switch by name. Returns false for an unknown name.
    pub fn set_toggle(&mut self, name: &str, on: bool) -> bool {
        match name {
            "frustum_culling" => self.toggles.frustum_culling = on,
            "player_cam_view" => self.toggles.player_cam_view = on,
            "show_frustum" => self.toggles.show_frustum = on,
            "show_culled_boxes" => self.toggles.show_culled_boxes = on,
            "graph" => self.toggles.show_graph = on,
            "overlay" => self.config.show_overlay = on,
            _ => return false,
        }
        true
    }

    /// Move the player by `delta` blocks (streams new chunks when it crosses a border).
    #[cfg_attr(not(feature = "test-hooks"), allow(dead_code))]
    pub fn translate_player(&mut self, delta: Vec3) {
        self.player.translate(delta);
    }

    /// Place the camera by hand. `target` is relative to the player's chunk.
    #[cfg_attr(not(feature = "test-hooks"), allow(dead_code))]
    pub fn set_camera(&mut self, target: Vec3, offset: Vec3) {
        self.camera_pinned = true;
        self.camera.orbit.target = target;
        self.camera.orbit.offset = offset;
    }

    fn tick_loading(&mut self) -> Result<Stage, String> {
        self.texture_loader
            .load(BLOCKS_ATLAS_PATH, TextureId::Blocks, 16)?;
        self.texture_loader
            .load(UV_CHECKER_PATH, TextureId::UvChecker, 1)?;
        Ok(Stage::WaitingForTextures)
    }

    fn tick_waiting_for_textures(&mut self, renderer: &mut Renderer) -> Result<Stage, String> {
        if let Err(e) = self
            .texture_loader
            .tick(|id, img, layers| renderer.load_texture(id, img, layers))
        {
            // Failed textures stay as placeholders; keep going.
            log::error!("Texture load error: {}", e);
        }
        if self.texture_loader.get_num_loading() > 0 {
            return Ok(Stage::WaitingForTextures);
        }

        self.overlay.init(&FrameStats::SERIES);
        Ok(Stage::Running)
    }

    pub fn tick(&mut self, renderer: &mut Renderer, time: f64) -> Result<(), String> {
        // On error the game is stuck in Loading, but the error is fatal upstream anyway.
        match std::mem::replace(&mut self.stage, Stage::Loading) {
            Stage::Loading => self.stage = self.tick_loading()?,
            Stage::WaitingForTextures => self.stage = self.tick_waiting_for_textures(renderer)?,
            Stage::Running => {
                let r = self.tick_running(renderer, time);
                self.stage = Stage::Running;
                r?;
            }
        }
        Ok(())
    }

    fn tick_running(&mut self, renderer: &mut Renderer, time: f64) -> Result<(), String> {
        let start = performance_now();
        self.upload_ms = 0.0;
        let ui_ms = std::mem::take(&mut self.ui_ms);

        self.time.update(time);

        self.apply_pending_toggles();
        self.handle_inputs();

        {
            let _s = trace::Span::new("textures", Track::World);
            self.texture_loader
                .tick(|id, img, layers| renderer.load_texture(id, img, layers))?;
        }
        self.update(renderer);

        let update_time = performance_now();

        if !self.skip_render {
            self.render(renderer);
        }
        let end = performance_now();

        let stats = renderer.stats();
        let times = FrameTimes {
            script: update_time - start - self.upload_ms,
            upload: self.upload_ms,
            ui: ui_ms,
            prepare: stats.prepare_ms,
            acquire: stats.acquire_ms,
            pass: stats.pass_ms,
            submit: stats.submit_ms,
            frame: end - start,
            interval: 0.0,
        };
        self.last_times = times;
        let refreshed = self.frame_stats.update_fps_report(start, times);

        if trace::enabled() {
            let m = stats.marks;
            let script_end = start + times.script;
            trace::entry("frame", start, end, Track::Frame);
            trace::entry("script", start, script_end, Track::World);
            trace::entry(
                "upload",
                script_end,
                script_end + self.upload_ms,
                Track::World,
            );
            if !self.skip_render {
                trace::entry("acquire", m[0], m[1], Track::Render);
                trace::entry("prepare", m[1], m[2], Track::Render);
                trace::entry("pass", m[2], m[3], Track::Render);
                trace::entry("submit", m[3], m[4], Track::Render);
            }
        }

        self.update_overlay(renderer, refreshed);

        Ok(())
    }

    /// Push state to the DOM overlay. Setters skip unchanged values; the info text and graph
    /// only go out when the stats refreshed (every 10 frames) and the overlay is visible.
    fn update_overlay(&mut self, renderer: &Renderer, refreshed: bool) {
        let visible = self.gui_state.show_info && self.config.show_overlay;
        self.overlay.set_visible(visible);
        self.overlay.set_paused(self.is_paused);
        if !(refreshed && visible) {
            return;
        }
        let t = performance_now();
        let info = self.build_info(renderer.stats());
        self.overlay.set_info(&info);
        if self.toggles.show_graph {
            self.frame_stats.fill_chart(&mut self.chart_buf);
            self.overlay
                .set_graph(&self.chart_buf, &self.frame_stats.averages());
        }
        let end = performance_now();
        self.ui_ms = end - t;
        trace::entry("ui", t, end, Track::World);
    }

    /// Apply toggles queued by the DOM overlay, then echo the real state back to it.
    fn apply_pending_toggles(&mut self) {
        let pending = PENDING_TOGGLES.with(|q| std::mem::take(&mut *q.borrow_mut()));
        for (name, on) in pending {
            if !self.set_toggle(&name, on) {
                log::warn!("unknown debug toggle: {name}");
            }
        }
        let t = self.toggles;
        self.overlay.set_toggles(&[
            ("frustum_culling", t.frustum_culling),
            ("show_frustum", t.show_frustum),
            ("show_culled_boxes", t.show_culled_boxes),
            ("player_cam_view", t.player_cam_view),
            ("graph", t.show_graph),
        ]);
    }

    fn update(&mut self, renderer: &mut Renderer) {
        if !self.is_paused {
            self.player.update(&self.time);
            let player_chunk_pos = self.player.get_position().chunk_pos();
            let changes = {
                let _s = trace::Span::new("world.update", Track::World);
                self.world.update(player_chunk_pos)
            };
            let t = performance_now();
            for pos in changes.removed {
                self.upload_queue.remove(pos);
                renderer.remove_chunk(pos);
            }
            for pos in changes.added {
                self.upload_queue.push(pos);
            }
            let world = &self.world;
            let batch = self.upload_queue.drain_budget(UPLOAD_BUDGET_BYTES, |p| {
                world.mesh(p).map_or(0, |m| m.quad_count() * 4)
            });
            self.last_changes_empty = batch.is_empty();
            for pos in batch {
                if let Some(mesh) = self.world.mesh(pos) {
                    renderer.upload_chunk(pos, mesh);
                }
            }
            self.upload_ms = performance_now() - t;
        }

        if !self.camera_pinned {
            self.camera.orbit.target = self.player.get_screen_position();
        }

        self.camera.update(&self.time);
    }

    fn handle_inputs(&mut self) {
        let inputs = self.input_system.get_inputs();

        for event in inputs.get_events() {
            if let InputEventType::KeyDown(event) = event {
                if event.repeat() {
                    continue;
                }
                match event.code().as_str() {
                    "Escape" => self.is_paused = !self.is_paused,
                    "F4" => {
                        self.toggles.player_cam_view = !self.toggles.player_cam_view;
                        event.prevent_default();
                    }
                    "F2" => {
                        self.gui_state.show_info = !self.gui_state.show_info;
                        event.prevent_default();
                    }
                    _ => {}
                }
            }
        }

        // The pause panel is DOM; while paused the game gets no input.
        if !self.is_paused {
            if !self.toggles.player_cam_view {
                self.camera.handle_inputs(&inputs);
            }
            self.player.handle_inputs(&inputs);
        }

        self.input_system.clear_events();
    }

    /// Match the canvas drawing buffer to its CSS size * devicePixelRatio,
    /// then keep the renderer viewport and camera projection in sync.
    fn sync_canvas_size(&mut self, renderer: &mut Renderer) {
        let Ok(canvas) = crate::utils::get_canvas() else {
            return;
        };
        let dpr = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio());
        let w = (canvas.client_width() as f64 * dpr).round() as u32;
        let h = (canvas.client_height() as f64 * dpr).round() as u32;
        if w > 0 && h > 0 && (canvas.width() != w || canvas.height() != h) {
            canvas.set_width(w);
            canvas.set_height(h);
        }
        let (w, h) = (canvas.width(), canvas.height());
        self.camera.set_viewport_size(w, h);
        self.player.view_mut().set_viewport_size(w, h);
        renderer.resize(w, h);
    }

    fn render(&mut self, renderer: &mut Renderer) {
        self.sync_canvas_size(renderer);

        let player_chunk = self.player.get_position().chunk_pos();
        // Eye in player-chunk-relative space (floating origin), same space as the chunk AABBs.
        let eye = self.player.get_screen_position();
        let pv = self.player.view();
        let t = self.toggles;
        let (view, projection) = if t.player_cam_view {
            (pv.view(eye), pv.projection())
        } else {
            (self.camera.look_at, self.camera.projection)
        };
        let cull_frustum: Option<Frustum> = t.frustum_culling.then(|| pv.frustum(eye));
        // Debug-only drawing: hidden when looking through the player's own eyes.
        let debug_view = !t.player_cam_view;
        let frame = FrameInput {
            view,
            projection,
            player_chunk,
            player_model: (self.config.show_player && debug_view)
                .then(|| player_model(&self.player, player_chunk)),
            gizmo_model: self.config.show_gizmo.then(|| gizmo_model(player_chunk)),
            clear_color: [0.0, 0.0, 0.0, 1.0],
            cull_frustum,
            frustum_lines: (t.show_frustum && debug_view).then(|| pv.view_proj(eye).inverse()),
            culled_boxes: t.show_culled_boxes && debug_view && t.frustum_culling,
        };
        renderer.render(&frame);
    }

    fn build_info(&self, stats: RenderStats) -> String {
        let gpu_info = format!(
            "GPU: {} chunks, {} draw calls ({} before merge)\nView: {} (F4)  Frustum: {}/{} chunks, {} quads drawn [culling {}]\narena: {}/{} quads ({:.1}/{:.1} MB), largest free {}\nupload: {:.1} KB/frame, {} queued",
            stats.chunk_buffers,
            stats.draw_calls,
            stats.draw_entries_raw,
            if self.toggles.player_cam_view { "player" } else { "debug" },
            stats.chunks_visible,
            stats.chunks_visible + stats.chunks_culled,
            stats.quads_drawn,
            if self.toggles.frustum_culling { "on" } else { "off" },
            stats.arena_used,
            stats.arena_capacity,
            stats.arena_used as f64 * 4e-6,
            stats.arena_capacity as f64 * 4e-6,
            stats.arena_largest_free,
            stats.upload_bytes as f64 / 1024.0,
            self.upload_queue.len()
        );
        format!(
            "FPS: {}\nRNG seed: {}\ngit rev: {}\nPlayer position: {:?}\nWorld: {}",
            self.frame_stats.get_debug_info(),
            self.seed,
            env!("GIT_HASH"),
            self.player.get_position(),
            self.world.get_info(&gpu_info)
        )
    }
}
