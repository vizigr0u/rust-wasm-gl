use std::rc::Rc;

use fastrand::Rng;
use glam::{UVec3, Vec3};
use glow::HasContext;
use log::{info, warn};
use wasm_bindgen::JsValue;

use crate::camera::Camera;
use crate::eguibackend::EguiBackend;
use crate::inputsystem::{self, HandleInputs, InputEventType, InputSystem};
use crate::material::{TextureDef, TextureType};
use crate::testworldgenerator::TestGenerator;
use crate::textureloader::TextureLoader;
use crate::time::Time;
use crate::utils::performance_now;
use crate::world::World;

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
    show_menu: bool,
    show_info: bool,
    tick_times: [f64; 30],
    tick_time: f64,
    tick_index: usize,

    egui: Option<EguiBackend>,
}

impl Game {
    pub fn new() -> Result<Self, JsValue> {
        let rng = Rng::with_seed(0);
        let game = Game {
            texture_loader: TextureLoader::new(10)?,
            world: World::random(WORLD_SIZE, make_generator(rng)),
            loaded_textures: Vec::new(),
            camera: Camera::new(
                Vec3 {
                    x: -10.0,
                    y: 60.0,
                    z: -30.0,
                },
                Vec3 {
                    x: 0.5,
                    y: -0.3,
                    z: 0.8,
                },
                Vec3 {
                    x: 0.2,
                    y: 1.0,
                    z: 0.2,
                },
            ),
            input_system: inputsystem::InputSystem::new()?,
            time: Time::default(),
            egui: None,
            show_menu: false,
            show_info: true,
            tick_time: 0.0,
            tick_times: [0.0; 30],
            tick_index: 0,
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
        self.update(gl, time)?;
        self.render(gl)?;
        let dt = performance_now() - start;
        self.tick_times[self.tick_index] = dt;
        self.tick_index = (self.tick_index + 1) % self.tick_times.len();
        if self.tick_index == 0 {
            self.tick_time = self.tick_times.iter().sum::<f64>() / self.tick_times.len() as f64;
        }
        Ok(())
    }

    fn update(&mut self, gl: &glow::Context, time: f64) -> Result<(), String> {
        self.time.update(time);
        self.handle_inputs();

        if !self.show_menu {
            self.world.update(gl, &self.time);
        }

        self.camera.update(&self.time);

        Ok(())
    }

    pub fn handle_inputs(&mut self) {
        let inputs = self.input_system.get_inputs();

        for event in inputs.get_events() {
            match event {
                InputEventType::KeyDown(event) => {
                    if inputs.is_key_down("Escape") {
                        self.show_menu = !self.show_menu;
                    }
                    if inputs.is_key_down("F2") {
                        self.show_info = !self.show_info;
                        event.prevent_default();
                    }
                }
                _ => {}
            }
        }

        if !self.show_menu {
            self.camera.handle_inputs(&inputs);
        } else {
            if let Some(egui) = &mut self.egui {
                egui.set_time(&self.time);
                egui.handle_inputs(&inputs);
            }
        }

        self.input_system.clear_events();
    }

    fn render(&mut self, gl: &glow::Context) -> Result<(), String> {
        self.texture_loader.tick(&gl)?;
        unsafe {
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
        }
        self.world.render(gl, &self.camera);

        self.draw_ui(gl);

        Ok(())
    }

    fn draw_ui(&mut self, gl: &glow::Context) {
        if let Some(egui) = &mut self.egui {
            egui.render_ui(gl, |ctx| {
                if self.show_menu {
                    egui::Window::new("PAUSE").show(ctx, |ui| {
                        ui.label("Game is paused");
                    });
                }
                if self.show_info {
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
