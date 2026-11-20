use egui::RichText;
use fastrand::Rng;
use glam::{vec3, Vec3};
use voxel_core::{gpu::UploadQueue, math::Frustum};
use wasm_bindgen::{JsCast, JsValue};

use crate::{
    graphics::{Camera, TextureLoader},
    gui::{EguiBackend, FpsDisplay, FrameTimes},
    objects::{gizmo_model, player_model, Player},
    render::{FrameInput, RenderStats, Renderer, TextureId, UiFrame},
    utils::performance_now,
    world::{
        Chunk, ChunkPos, GeneratorV0, InlineMesher, MeshExecutor, SingleBlockGenerator, World,
        WorldGenerator,
    },
};

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
    pub show_egui: bool,
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
            show_egui: true,
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
}

fn make_generator(config: &SceneConfig, _rng: Rng) -> Generator {
    match config.generator {
        SceneGenerator::V0 => Generator::V0(GeneratorV0::new()),
        SceneGenerator::SingleBlock => Generator::SingleBlock(SingleBlockGenerator),
    }
}

/// Game-side state that exists once textures are ready (the GPU side lives in `Renderer`).
#[derive(Debug)]
struct Gfx {
    egui: EguiBackend,
}

#[derive(Debug)]
enum Stage {
    Loading,
    WaitingForTextures,
    Running(Box<Gfx>),
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
    fps_display: FpsDisplay,
    /// ms spent in chunk upload/remove during this tick's `update`.
    upload_ms: f64,
    /// ms spent building egui this tick.
    ui_ms: f64,
    /// Timings of the last running tick (profiling hooks read these).
    last_times: FrameTimes,
}

#[derive(Debug)]
struct GuiState {
    show_pause_menu: bool,
    show_info: bool,
}

impl GuiState {
    pub fn eats_input(&self) -> bool {
        self.show_pause_menu
    }
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            show_pause_menu: false,
            show_info: true,
        }
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
            fps_display: FpsDisplay::new(),
            upload_ms: 0.0,
            ui_ms: 0.0,
            last_times: FrameTimes::default(),
        };

        Ok(game)
    }

    #[cfg_attr(not(feature = "test-hooks"), allow(dead_code))]
    pub fn is_running(&self) -> bool {
        matches!(self.stage, Stage::Running(_))
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
    #[cfg_attr(not(feature = "test-hooks"), allow(dead_code))]
    pub fn set_toggle(&mut self, name: &str, on: bool) -> bool {
        match name {
            "frustum_culling" => self.toggles.frustum_culling = on,
            "player_cam_view" => self.toggles.player_cam_view = on,
            "show_frustum" => self.toggles.show_frustum = on,
            "egui" => self.config.show_egui = on,
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

        Ok(Stage::Running(Box::new(Gfx {
            egui: EguiBackend::new(),
        })))
    }

    pub fn tick(&mut self, renderer: &mut Renderer, time: f64) -> Result<(), String> {
        // Take the stage out so Running's Gfx can be borrowed alongside &mut self.
        // On error the game is stuck in Loading, but the error is fatal upstream anyway.
        match std::mem::replace(&mut self.stage, Stage::Loading) {
            Stage::Loading => self.stage = self.tick_loading()?,
            Stage::WaitingForTextures => self.stage = self.tick_waiting_for_textures(renderer)?,
            Stage::Running(mut gfx) => {
                let r = self.tick_running(renderer, &mut gfx, time);
                self.stage = Stage::Running(gfx);
                r?;
            }
        }
        Ok(())
    }

    fn tick_running(
        &mut self,
        renderer: &mut Renderer,
        gfx: &mut Gfx,
        time: f64,
    ) -> Result<(), String> {
        let start = performance_now();
        self.upload_ms = 0.0;
        self.ui_ms = 0.0;

        self.time.update(time);

        self.handle_inputs(&mut gfx.egui);

        self.texture_loader
            .tick(|id, img, layers| renderer.load_texture(id, img, layers))?;
        self.update(renderer);

        self.update_gui(&mut gfx.egui);
        let update_time = performance_now();

        if !self.skip_render {
            self.render(renderer, gfx);
        }
        let end = performance_now();

        let stats = renderer.stats();
        let times = FrameTimes {
            script: update_time - start - self.upload_ms,
            upload: self.upload_ms,
            ui: self.ui_ms,
            prepare: stats.prepare_ms,
            acquire: stats.acquire_ms,
            pass: stats.pass_ms,
            egui: stats.egui_ms,
            submit: stats.submit_ms,
            frame: end - start,
            interval: 0.0,
        };
        self.last_times = times;
        self.fps_display.update_fps_report(start, times);

        Ok(())
    }

    fn update_gui(&mut self, egui: &mut EguiBackend) {
        self.gui_state.show_pause_menu = self.is_paused;
        egui.set_time(&self.time);
    }

    fn update(&mut self, renderer: &mut Renderer) {
        if !self.is_paused {
            self.player.update(&self.time);
            let player_chunk_pos = self.player.get_position().chunk_pos();
            let changes = self.world.update(player_chunk_pos);
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

    fn handle_inputs(&mut self, egui: &mut EguiBackend) {
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

        match self.gui_state.eats_input() {
            true => egui.handle_inputs(&inputs),
            false => {
                // egui gets no input now; drop stale events so they don't replay every frame
                egui.clear_events();
                if !self.toggles.player_cam_view {
                    self.camera.handle_inputs(&inputs);
                }
                self.player.handle_inputs(&inputs);
            }
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

    fn render(&mut self, renderer: &mut Renderer, gfx: &mut Gfx) {
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
        // The info label shows the previous frame's draw stats.
        let t = performance_now();
        let ui = self
            .config
            .show_egui
            .then(|| self.build_ui(gfx, renderer.stats()));
        self.ui_ms = performance_now() - t;
        renderer.render(&frame, ui);
    }

    fn build_ui(&mut self, gfx: &mut Gfx, stats: RenderStats) -> UiFrame {
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
        {
            gfx.egui.run_ui(|ctx| {
                if self.is_paused {
                    egui::Window::new("PAUSE").show(ctx, |ui| {
                        ui.label("Game is paused");
                        ui.checkbox(&mut self.toggles.frustum_culling, "Frustum culling");
                        ui.checkbox(&mut self.toggles.show_frustum, "Show frustum lines");
                        ui.checkbox(&mut self.toggles.show_culled_boxes, "Show culled boxes");
                        ui.checkbox(&mut self.toggles.player_cam_view, "Player camera view (F4)");
                    });
                }
                if self.gui_state.show_info {
                    // egui::Window::new("Game Info").show(ctx, |ui| {
                        egui::Area::new(egui::Id::unique("my_area"))
                            .show(ctx, |ui| {
                                egui::Frame::new()
                                .fill(egui::Color32::from_rgba_unmultiplied(20, 20, 20, 100))
                                .show(ui, |ui| {
                                    let info = format!(
                                        "FPS: {}\nRNG seed: {}\ngit rev: {}\nPlayer position: {:?}\nWorld: {}",
                                        self.fps_display.get_debug_info(),
                                        self.seed,
                                        env!("GIT_HASH"),
                                        self.player.get_position(),
                                        self.world.get_info(&gpu_info)
                                    );
                                    ui.colored_label(
                                        egui::Color32::WHITE,
                                        RichText::new(&info).monospace(),
                                    );
                                    if self.is_paused {
                                        ui.horizontal(|ui| {
                                            if ui.button("Copy").clicked() {
                                                copy_to_clipboard(&info);
                                            }
                                            self.fps_display.graph_button(ui);
                                        });
                                    }
                                    self.fps_display.display(ui);
                                });
                        });
                }
            })
        }
    }
}

/// Write text to the system clipboard via `navigator.clipboard.writeText`.
fn copy_to_clipboard(text: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let clipboard = js_sys::Reflect::get(&window, &"navigator".into())
        .and_then(|n| js_sys::Reflect::get(&n, &"clipboard".into()))
        .ok();
    let write = clipboard.as_ref().and_then(|c| {
        let f = js_sys::Reflect::get(c, &"writeText".into()).ok()?;
        f.dyn_into::<js_sys::Function>().ok()
    });
    match (clipboard, write) {
        (Some(c), Some(f)) => {
            if f.call1(&c, &text.into()).is_err() {
                log::error!("clipboard write failed");
            }
        }
        _ => log::error!("clipboard not available"),
    }
}
