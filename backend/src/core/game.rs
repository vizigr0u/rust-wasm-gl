use std::{cell::RefCell, rc::Rc};

use egui::RichText;
use fastrand::Rng;
use glam::{vec3, Vec3};
use glow::HasContext;
use wasm_bindgen::JsValue;

use crate::{
    graphics::{Camera, Renderer, TextureDef, TextureLoader, TextureType},
    gui::{EguiBackend, FpsDisplay},
    objects::{Gizmo, Player, Transform},
    utils::performance_now,
    world::{GeneratorV0, TestGenerator, World},
};

use super::{HandleInputs, InputEventType, InputSystem, Time};

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

    world: World<WorldGenerator>,
    camera: Camera,

    input_system: InputSystem,

    time: Time,
    is_paused: bool,
    gui_state: GuiState,
    gizmo: Gizmo,
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
            current_state: State::Loading,

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
            gizmo: Gizmo::new(Vec3::ZERO, 10.0),
            player: Player::new(vec3(0.0, 16.0, 0.0)),
            fps_display: FpsDisplay::new(),
        };

        Ok(game)
    }

    fn tick_loading(&mut self) -> Result<State, String> {
        Ok(State::Running)
    }

    pub fn tick(&mut self, time: f64) -> Result<(), String> {
        match self.current_state {
            State::Loading => {
                let load_state = self.tick_loading()?;
                self.current_state = load_state;
            }
            State::Running => {
                self.tick_running(time)?;
            }
        }
        Ok(())
    }

    fn tick_running(&mut self, time: f64) -> Result<(), String> {
        let start = performance_now();

        self.time.update(time);

        self.handle_inputs();

        self.update()?;

        self.update_gui();
        let update_time = performance_now();

        let frame_time = performance_now();
        self.fps_display
            .update_fps_report(start, update_time, frame_time);

        Ok(())
    }

    fn update_gui(&mut self, ctx: &egui::Context) {
        self.gui_state.show_pause_menu = self.is_paused;

        if self.is_paused {
            egui::Window::new("PAUSE").show(ctx, |ui| {
                ui.label("Game is paused");
            });
        }
        if self.gui_state.show_info {
            // egui::Window::new("Game Info").show(ctx, |ui| {
            egui::Area::new("my_area".into()).show(ctx, |ui| {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(20, 20, 20, 100))
                    .show(ui, |ui| {
                        self.fps_display.display(ui);

                        ui.colored_label(
                            egui::Color32::WHITE,
                            RichText::new(format!(
                                "RNG seed: {}\ngit rev: {}\nPlayer position: {:?}\nWorld: {}",
                                self.seed,
                                env!("GIT_HASH"),
                                self.player.get_position(),
                                self.world.get_info()
                            ))
                            .monospace(),
                        );
                    });
            });
        }
    }

    fn update(&mut self) -> Result<(), String> {
        if !self.is_paused {
            self.player.update(&self.time);
            let player_chunk_pos = self.player.get_position().chunk_pos();
            self.world.update(&self.time, player_chunk_pos)?;
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
}
