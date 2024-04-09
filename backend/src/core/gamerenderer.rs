use std::{cell::RefCell, rc::Rc};

use wasm_bindgen::JsValue;

use crate::{
    graphics::{Renderer, TextureDef, TextureLoader, TextureType},
    utils::performance_now,
};

const BLOCKS_ATLAS_PATH: &str = "data/textures/blocks/blocks_atlas.png";
const UV_CHECKER_PATH: &str = "data/textures/UV_checker_Map_byValle.jpg";

pub struct GameRenderer<R>
where
    R: Renderer,
{
    renderer: Rc<RefCell<R>>,
    texture_loader: TextureLoader<R>,
    loaded_textures: Vec<Rc<TextureDef<R>>>,
    egui: Option<EguiBackend>,
    world_renderer: WorldRenderer<R>,
}

impl<R> GameRenderer<R>
where
    R: Renderer,
{
    pub fn new(renderer: R) -> Result<Self, JsValue> {
        let renderer = Rc::new(RefCell::new(renderer));
        Ok(Self {
            renderer: renderer.clone(),
            texture_loader: TextureLoader::new(10, renderer)?,
            loaded_textures: Vec::new(),
            egui: EguiBackend::new(gl),
        })
    }

    fn tick_loading(&mut self, gl: &glow::Context) -> Result<(), String> {
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

        Ok()
    }

    pub fn tick(&mut self, time: f64) -> Result<(), String> {
        let start = performance_now();
        let update_time = performance_now();

        self.render()?;
        let render_time = performance_now();

        self.draw_ui();

        let frame_time = performance_now();
        self.update_fps_report(start, update_time, render_time, frame_time);

        Ok(())
    }

    fn render(&mut self) -> Result<(), String> {
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

    fn draw_ui(&mut self) {
        if let Some(egui) = &mut self.egui {
            egui.render_ui(gl, |ctx| {
                if self.is_paused {
                    egui::Window::new("PAUSE").show(ctx, |ui| {
                        ui.label("Game is paused");
                    });
                }
                if self.gui_state.show_info {
                    // egui::Window::new("Game Info").show(ctx, |ui| {
                        egui::Area::new("my_area".into())
                            .show(ctx, |ui| {
                                egui::Frame::none()
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

    fn update_fps_report(&self, start: f64, update_time: f64, render_time: f64, frame_time: f64) {
        todo!()
    }
}
