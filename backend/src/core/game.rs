use std::rc::Rc;

use fastrand::Rng;
use glam::{vec3, UVec3, Vec3};
use glow::HasContext;
use wasm_bindgen::JsValue;

use crate::{
    graphics::{Camera, TextureDef, TextureLoader, TextureType},
    gui::EguiBackend,
    objects::{Gizmo, Player, Transform},
    utils::performance_now,
    world::{TestGenerator, World},
};

use super::{HandleInputs, InputEventType, InputSystem, Time};

const WORLD_SIZE: UVec3 = UVec3::new(10, 10, 10);

const GRASS_TEXTURE_PATH: &str = "data/textures/blocks/grass_block_side.png";
const SAND_TEXTURE_PATH: &str = "data/textures/blocks/sand.png";
const DIRT_TEXTURE_PATH: &str = "data/textures/blocks/dirt.png";
const BLOCKS_ATLAS_PATH: &str = "data/textures/blocks/blocks_atlas.png";

type WorldGenerator = TestGenerator;

fn make_generator(rng: Rng) -> WorldGenerator {
    TestGenerator { rng }
}

#[derive(Debug)]
pub struct Game {
    texture_loader: TextureLoader,
    world: World<WorldGenerator>,
    camera: Camera,

    input_system: InputSystem,
    loaded_textures: Vec<Rc<TextureDef>>,
    time: Time,
    is_paused: bool,
    gui_state: GuiState,
    tick_times: [f64; 30],
    tick_time: f64,
    tick_index: usize,
    gizmo: Gizmo,
    player: Player,

    egui: Option<EguiBackend>,
}

#[derive(Default, Debug)]
struct GuiState {
    show_pause_menu: bool,
    show_info: bool,
}

impl GuiState {
    pub fn eats_input(&self) -> bool {
        self.show_pause_menu
    }
}

impl Game {
    pub fn new() -> Result<Self, JsValue> {
        let rng = Rng::with_seed(0);
        let game = Game {
            texture_loader: TextureLoader::new(10)?,
            world: World::random(WORLD_SIZE, make_generator(rng)),
            loaded_textures: Vec::new(),
            camera: Camera::new(Vec3 {
                x: -10.0,
                y: 5.0,
                z: 0.0,
            }),
            input_system: InputSystem::new()?,
            is_paused: false,
            time: Time::default(),
            egui: None,
            gui_state: GuiState::default(),
            tick_time: 0.0,
            tick_times: [0.0; 30],
            tick_index: 0,
            gizmo: Gizmo::new(Vec3::ZERO, 10.0),
            player: Player::new(vec3(0.0, 0.5, 0.0)),
        };

        Ok(game)
    }

    pub unsafe fn load(&mut self, gl: &glow::Context) -> Result<(), String> {
        for (path, t) in [
            (GRASS_TEXTURE_PATH, TextureType::Texture2D),
            (SAND_TEXTURE_PATH, TextureType::Texture2D),
            (DIRT_TEXTURE_PATH, TextureType::Texture2D),
            (BLOCKS_ATLAS_PATH, TextureType::Texture2DArray(16)),
        ] {
            let key = self.texture_loader.load(gl, path, t)?;
            self.loaded_textures.push(Rc::new((t, key)));
        }

        gl.enable(glow::DEPTH_TEST);
        gl.depth_func(glow::LESS);
        gl.enable(glow::CULL_FACE);

        self.world
            .setup_graphics(gl, self.loaded_textures[3].clone())?;
        self.egui = Some(EguiBackend::new(gl));

        Ok(())
    }

    pub fn tick(&mut self, gl: &glow::Context, time: f64) -> Result<(), String> {
        let start = performance_now();

        self.time.update(time);

        self.handle_inputs();

        self.update(gl)?;

        self.update_gui();

        self.render(gl)?;

        let frame_time = performance_now() - start;
        self.update_fps_report(frame_time);

        Ok(())
    }

    fn update_gui(&mut self) {
        self.gui_state.show_pause_menu = self.is_paused;
        if let Some(egui) = &mut self.egui {
            egui.set_time(&self.time);
        }
    }

    fn update_fps_report(&mut self, frame_time: f64) {
        self.tick_times[self.tick_index] = frame_time;
        self.tick_index = (self.tick_index + 1) % self.tick_times.len();
        if self.tick_index == 0 {
            self.tick_time = self.tick_times.iter().sum::<f64>() / self.tick_times.len() as f64;
        }
    }

    fn update(&mut self, gl: &glow::Context) -> Result<(), String> {
        if !self.is_paused {
            self.player.update(&self.time);
            if let Some(player) = self.player.get_gameobject() {
                self.world.update(gl, &self.time, player.get_position())?;
            }
        }

        if let Some(player) = self.player.get_gameobject() {
            self.camera.target = player.get_position();
        }
        self.camera.update(&self.time);

        self.gizmo.update(&self.time);

        Ok(())
    }

    fn handle_inputs(&mut self) {
        let inputs = self.input_system.get_inputs();

        for event in inputs.get_events() {
            match event {
                InputEventType::KeyDown(event) => {
                    if inputs.is_key_down("Escape") {
                        self.is_paused = !self.is_paused;
                    }
                    if inputs.is_key_down("F2") {
                        self.gui_state.show_info = !self.gui_state.show_info;
                        event.prevent_default();
                    }
                }
                _ => {}
            }
        }

        match self.gui_state.eats_input() {
            true => {
                if let Some(egui) = &mut self.egui {
                    egui.handle_inputs(&inputs);
                }
            }
            false => {
                self.camera.handle_inputs(&inputs);
                self.player.handle_inputs(&inputs);
            }
        }

        self.input_system.clear_events();
    }

    fn render(&mut self, gl: &glow::Context) -> Result<(), String> {
        self.texture_loader.tick(&gl)?;
        unsafe {
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        }
        self.world.render(gl, &self.camera);

        self.player.render_lazy(gl, &self.camera);
        self.gizmo.render_lazy(gl, &self.camera);

        self.draw_ui(gl);

        Ok(())
    }

    fn draw_ui(&mut self, gl: &glow::Context) {
        if let Some(egui) = &mut self.egui {
            egui.render_ui(gl, |ctx| {
                if self.is_paused {
                    egui::Window::new("PAUSE").show(ctx, |ui| {
                        ui.label("Game is paused");
                    });
                }
                if self.gui_state.show_info {
                    egui::Window::new("Game Info").show(ctx, |ui| {
                        ui.label(format!(
                            "FPS: {:.1}\nWorld: {}",
                            1000.0 / self.tick_time,
                            self.world.get_info()
                        ));
                    });
                }
            });
        }
    }
}
