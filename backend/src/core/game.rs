use std::rc::Rc;

use egui::RichText;
use fastrand::Rng;
use glam::{vec3, Vec3};
use glow::HasContext;
use wasm_bindgen::JsValue;

use crate::{
    graphics::{Camera, TextureDef, TextureLoader, TextureType},
    gui::{EguiBackend, FpsDisplay},
    objects::{ObjectRenderers, Player},
    utils::performance_now,
    world::{GeneratorV0, World, WorldRenderData},
};

use super::{HandleInputs, InputEventType, InputSystem, Time};

const BLOCKS_ATLAS_PATH: &str = "data/textures/blocks/blocks_atlas.png";
const UV_CHECKER_PATH: &str = "data/textures/UV_checker_Map_byValle.jpg";

type WorldGenerator = GeneratorV0;

fn make_generator(_rng: Rng) -> WorldGenerator {
    GeneratorV0::new()
}

#[derive(Debug)]
struct Textures {
    blocks: Rc<TextureDef>,
    #[allow(dead_code)]
    uv_checker: Rc<TextureDef>,
}

/// GL-dependent state; only exists once textures are ready.
#[derive(Debug)]
struct Gfx {
    #[allow(dead_code)]
    textures: Textures,
    egui: EguiBackend,
    world: WorldRenderData,
    objects: ObjectRenderers,
}

#[derive(Debug)]
enum Stage {
    Loading,
    WaitingForTextures(Textures),
    Running(Box<Gfx>),
}

#[derive(Debug)]
pub struct Game {
    seed: u64,
    stage: Stage,
    texture_loader: TextureLoader,
    world: World<WorldGenerator>,
    camera: Camera,

    input_system: InputSystem,
    time: Time,
    is_paused: bool,
    gui_state: GuiState,
    player: Player,
    fps_display: FpsDisplay,
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
        let seed = 0;
        let rng = Rng::with_seed(seed);
        let game = Game {
            seed,
            stage: Stage::Loading,
            texture_loader: TextureLoader::new(10)?,
            world: World::new(make_generator(rng)),
            camera: Camera::new(Vec3 {
                x: -10.0,
                y: 5.0,
                z: 0.0,
            }),
            input_system: InputSystem::new()?,
            is_paused: false,
            time: Time::default(),
            gui_state: GuiState::default(),
            player: Player::new(vec3(0.0, 16.0, 0.0)),
            fps_display: FpsDisplay::new(),
        };

        Ok(game)
    }

    fn tick_loading(&mut self, gl: &glow::Context) -> Result<Stage, String> {
        let mut load = |path, t| -> Result<Rc<TextureDef>, String> {
            let key = self.texture_loader.load(gl, path, t)?;
            Ok(Rc::new((t, key)))
        };
        let blocks = load(BLOCKS_ATLAS_PATH, TextureType::Texture2DArray(16))?;
        let uv_checker = load(UV_CHECKER_PATH, TextureType::Texture2D)?;

        Ok(Stage::WaitingForTextures(Textures { blocks, uv_checker }))
    }

    fn tick_waiting_for_textures(
        &mut self,
        gl: &glow::Context,
        textures: Textures,
    ) -> Result<Stage, String> {
        if let Err(e) = self.texture_loader.tick(gl) {
            // Failed textures stay as placeholders; keep going.
            log::error!("Texture load error: {}", e);
        }
        if self.texture_loader.get_num_loading() > 0 {
            return Ok(Stage::WaitingForTextures(textures));
        }

        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LESS);
            gl.enable(glow::CULL_FACE);
        }

        let world = WorldRenderData::new(gl, textures.blocks.clone())?;
        let egui = EguiBackend::new(gl);
        let objects = ObjectRenderers::new(gl)?;

        Ok(Stage::Running(Box::new(Gfx {
            textures,
            egui,
            world,
            objects,
        })))
    }

    pub fn tick(&mut self, gl: &glow::Context, time: f64) -> Result<(), String> {
        // Take the stage out so Running's Gfx can be borrowed alongside &mut self.
        // On error the game is stuck in Loading, but the error is fatal upstream anyway.
        match std::mem::replace(&mut self.stage, Stage::Loading) {
            Stage::Loading => self.stage = self.tick_loading(gl)?,
            Stage::WaitingForTextures(t) => self.stage = self.tick_waiting_for_textures(gl, t)?,
            Stage::Running(mut gfx) => {
                let r = self.tick_running(gl, &mut gfx, time);
                self.stage = Stage::Running(gfx);
                r?;
            }
        }
        Ok(())
    }

    fn tick_running(&mut self, gl: &glow::Context, gfx: &mut Gfx, time: f64) -> Result<(), String> {
        let start = performance_now();

        self.time.update(time);

        self.handle_inputs(&mut gfx.egui);

        self.texture_loader.tick(gl)?;
        self.update(gl, gfx);

        self.update_gui(&mut gfx.egui);
        let update_time = performance_now();

        self.render(gl, gfx)?;
        let render_time = performance_now();

        self.draw_ui(gl, gfx);

        let frame_time = performance_now();
        self.fps_display
            .update_fps_report(start, update_time, render_time, frame_time);

        Ok(())
    }

    fn update_gui(&mut self, egui: &mut EguiBackend) {
        self.gui_state.show_pause_menu = self.is_paused;
        egui.set_time(&self.time);
    }

    fn update(&mut self, gl: &glow::Context, gfx: &mut Gfx) {
        if !self.is_paused {
            self.player.update(&self.time);
            let player_chunk_pos = self.player.get_position().chunk_pos();
            let changes = self.world.update(player_chunk_pos);
            for pos in changes.removed {
                gfx.world.remove_chunk(gl, pos);
            }
            for pos in changes.added {
                if let Some(mesh) = self.world.mesh(pos) {
                    gfx.world.upload_chunk(gl, pos, mesh);
                }
            }
        }

        self.camera.target = self.player.get_screen_position();

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
                self.camera.handle_inputs(&inputs);
                self.player.handle_inputs(&inputs);
            }
        }

        self.input_system.clear_events();
    }

    /// Match the canvas drawing buffer to its CSS size * devicePixelRatio,
    /// then keep the GL viewport and camera projection in sync.
    fn sync_canvas_size(&mut self, gl: &glow::Context) {
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
        unsafe {
            gl.viewport(0, 0, w as i32, h as i32);
        }
    }

    fn render(&mut self, gl: &glow::Context, gfx: &Gfx) -> Result<(), String> {
        self.sync_canvas_size(gl);
        unsafe {
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        }
        gfx.world
            .render(gl, &self.camera, self.world.player_chunk_pos());

        let player_chunk = self.player.get_position().chunk_pos();
        gfx.objects
            .draw_player(gl, &self.camera, &self.player, player_chunk);
        gfx.objects.draw_gizmo(gl, &self.camera, player_chunk);

        Ok(())
    }

    fn draw_ui(&mut self, gl: &glow::Context, gfx: &mut Gfx) {
        let gpu_info = gfx.world.get_info();
        {
            gfx.egui.render_ui(gl, |ctx| {
                if self.is_paused {
                    egui::Window::new("PAUSE").show(ctx, |ui| {
                        ui.label("Game is paused");
                    });
                }
                if self.gui_state.show_info {
                    // egui::Window::new("Game Info").show(ctx, |ui| {
                        egui::Area::new(egui::Id::new("my_area"))
                            .show(ctx, |ui| {
                                egui::Frame::new()
                                .fill(egui::Color32::from_rgba_unmultiplied(20, 20, 20, 100))
                                .show(ui, |ui| {
                                    self.fps_display.display(ui);

                                        ui.colored_label(egui::Color32::WHITE,RichText::new(format!(
                                            "RNG seed: {}\ngit rev: {}\nPlayer position: {:?}\nWorld: {}",
                                            self.seed,
                                            env!("GIT_HASH"),
                                            self.player.get_position(),
                                            self.world.get_info(&gpu_info)
                                        )).monospace());
                                });
                        });
                }
            });
        }
    }
}
