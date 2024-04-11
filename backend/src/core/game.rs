use std::{cell::RefCell, fmt::Debug, rc::Rc};

use egui::{FullOutput, RichText};
use fastrand::Rng;
use glam::{vec3, Vec3, Vec4};
use wasm_bindgen::JsValue;

use crate::{
    graphics::commands::{self, GraphicsCommands},
    gui::FpsDisplay,
    objects::Camera,
    utils::performance_now, // world::{GeneratorV0, TestGenerator, World},
};

use super::{HandleInputs, InputEventType, InputSystem, Time};

// type WorldGenerator = GeneratorV0;

// fn make_generator(rng: Rng) -> WorldGenerator {
//     GeneratorV0::new(rng)
// }

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum State {
    Loading,
    Running,
}

#[derive(Debug)]
pub struct Game {
    seed: u64,
    current_state: State,

    // world: World<WorldGenerator>,
    camera: Camera,

    input_system: InputSystem,

    time: Time,
    is_paused: bool,
    egui_context: Option<egui::Context>,
    gui_state: GuiState,
    // gizmo: Gizmo,
    // player: Player,
    screen_size: (u32, u32),
    fps_display: FpsDisplay,
    pub last_render_time: f64,
    pub last_render_ui_time: f64,
    pub clear_color: Vec4,
    pub commands: Vec<commands::SerializedCommand>,
}

struct GuiState {
    show_pause_menu: bool,
    show_info: bool,
    gui: Option<FullOutput>,
}

impl Debug for GuiState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GuiState")
            .field("show_pause_menu", &self.show_pause_menu)
            .field("show_info", &self.show_info)
            .finish()
    }
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
            gui: None,
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

            // world: World::new(make_generator(rng)),
            camera: Camera::new(Vec3 {
                x: -10.0,
                y: 5.0,
                z: 0.0,
            }),
            input_system: InputSystem::new()?,
            is_paused: false,
            time: Time::default(),
            gui_state: GuiState::default(),
            // gizmo: Gizmo::new(Vec3::ZERO, 10.0),
            // player: Player::new(vec3(0.0, 16.0, 0.0)),
            fps_display: FpsDisplay::new(),
            screen_size: (800, 600),
            egui_context: None,
            last_render_time: 0.0,
            last_render_ui_time: 0.0,
            clear_color: Vec4::new(0.4, 0.6, 0.7, 1.0),
            commands: Vec::new(),
        };

        Ok(game)
    }

    fn tick_loading(&mut self) -> Result<State, String> {
        Ok(State::Running)
    }

    pub fn tick(&mut self, time: f64) -> Result<(), String> {
        self.commands.clear();
        match self.current_state {
            State::Loading => {
                let load_state = self.tick_loading()?;
                self.current_state = load_state;
            }
            State::Running => {
                self.tick_running(time)?;
            }
        }
        let command = GraphicsCommands::set_clear_color(&self.clear_color.to_array());
        command.enqueue(&mut self.commands);
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
        self.fps_display.update_fps_report(
            start,
            update_time,
            self.last_render_time,
            self.last_render_ui_time,
            frame_time,
        );

        Ok(())
    }

    fn update_gui(&mut self) {
        self.gui_state.show_pause_menu = self.is_paused;

        if self.egui_context.is_none() {
            self.gui_state.gui = None;
            return;
        }

        let new_input: egui::RawInput = self.gather_egui_input();
        let ctx = self.egui_context.as_mut().unwrap();
        self.gui_state.gui = Some(ctx.run(new_input, |ctx| {
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
                                    "(no player)", // self.player.get_position(),
                                    "(none)",      // self.world.get_info()
                                ))
                                .monospace(),
                            );
                        });
                });
            }
        }));
    }

    fn gather_egui_input(&self) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect {
                min: egui::pos2(0.0, 0.0),
                max: egui::pos2(self.screen_size.0 as f32, self.screen_size.1 as f32),
            }),
            // events: self.current_events.clone(),
            // time: Some(self.time.time()),
            // predicted_dt: self.time.delta_time() as _,
            ..Default::default()
        }
    }

    fn update(&mut self) -> Result<(), String> {
        if !self.is_paused {
            // self.player.update(&self.time);
            // let player_chunk_pos = self.player.get_position().chunk_pos();
            // self.world.update(&self.time, player_chunk_pos)?;
            // self.gizmo.update(&self.time, player_chunk_pos);
        }

        // self.camera.target = self.player.get_screen_position();

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
            true => {}
            false => {
                self.camera.handle_inputs(&inputs);
                // self.player.handle_inputs(&inputs);
            }
        }

        self.input_system.clear_events();
    }
}
