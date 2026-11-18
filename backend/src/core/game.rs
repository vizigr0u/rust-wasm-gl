use std::rc::Rc;

use egui::RichText;
use fastrand::Rng;
use glam::{vec3, Vec3};
use glow::HasContext;
use wasm_bindgen::JsValue;

use crate::{
    graphics::{Camera, TextureDef, TextureLoader, TextureType},
    gui::{EguiBackend, FpsDisplay},
    objects::{Gizmo, Player, Transform},
    utils::performance_now,
    world::{GeneratorV0, TestGenerator, World},
};

use super::{HandleInputs, InputEventType, InputSystem, Time};

const BLOCKS_ATLAS_PATH: &str = "data/textures/blocks/blocks_atlas.png";
const UV_CHECKER_PATH: &str = "data/textures/UV_checker_Map_byValle.jpg";

type WorldGenerator = GeneratorV0;

fn make_generator(rng: Rng) -> WorldGenerator {
    GeneratorV0::new(rng)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum State {
    Loading,
    Running,
}

#[derive(Debug)]
pub struct Game {
    seed: u64,
    current_state: State,
    texture_loader: TextureLoader,
    world: World<WorldGenerator>,
    camera: Camera,

    input_system: InputSystem,
    loaded_textures: Vec<Rc<TextureDef>>,
    time: Time,
    is_paused: bool,
    gui_state: GuiState,
    gizmo: Gizmo,
    player: Player,

    egui: Option<EguiBackend>,
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
            current_state: State::Loading,
            texture_loader: TextureLoader::new(10)?,
            world: World::new(make_generator(rng)),
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
            gizmo: Gizmo::new(Vec3::ZERO, 10.0),
            player: Player::new(vec3(0.0, 16.0, 0.0)),
            fps_display: FpsDisplay::new(),
        };

        Ok(game)
    }

    fn tick_loading(&mut self, gl: &glow::Context) -> Result<State, String> {
        for (path, t) in [
            (BLOCKS_ATLAS_PATH, TextureType::Texture2DArray(16)),
            (UV_CHECKER_PATH, TextureType::Texture2D),
        ] {
            let key = self.texture_loader.load(gl, path, t)?;
            self.loaded_textures.push(Rc::new((t, key)));
        }

        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LESS);
            gl.enable(glow::CULL_FACE);
        }

        self.world
            .setup_graphics(gl, self.loaded_textures[0].clone())?;
        self.egui = Some(EguiBackend::new(gl));

        Ok(State::Running)
    }

    pub fn tick(&mut self, gl: &glow::Context, time: f64) -> Result<(), String> {
        match self.current_state {
            State::Loading => {
                let load_state = self.tick_loading(gl)?;
                self.current_state = load_state;
            }
            State::Running => {
                self.tick_running(gl, time)?;
            }
        }
        Ok(())
    }

    fn tick_running(&mut self, gl: &glow::Context, time: f64) -> Result<(), String> {
        let start = performance_now();

        self.time.update(time);

        self.handle_inputs();

        self.update(gl)?;

        self.update_gui();
        let update_time = performance_now();

        self.render(gl)?;
        let render_time = performance_now();

        self.draw_ui(gl);

        let frame_time = performance_now();
        self.fps_display
            .update_fps_report(start, update_time, render_time, frame_time);

        Ok(())
    }

    fn update_gui(&mut self) {
        self.gui_state.show_pause_menu = self.is_paused;
        if let Some(egui) = &mut self.egui {
            egui.set_time(&self.time);
        }
    }

    fn update(&mut self, gl: &glow::Context) -> Result<(), String> {
        if !self.is_paused {
            self.player.update(&self.time);
            let player_chunk_pos = self.player.get_position().chunk_pos();
            self.world.update(gl, &self.time, player_chunk_pos)?;
            self.gizmo.update(&self.time, player_chunk_pos);
        }

        self.camera.target = self.player.get_screen_position();

        self.camera.update(&self.time);

        Ok(())
    }

    fn handle_inputs(&mut self) {
        let inputs = self.input_system.get_inputs();

        for event in inputs.get_events() {
            match event {
                InputEventType::KeyDown(event) => {
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

        self.player.render(gl, &self.camera);
        self.gizmo.render_lazy(gl, &self.camera);

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
                                            self.world.get_info()
                                        )).monospace());
                                });
                        });
                }
            });
        }
    }
}
