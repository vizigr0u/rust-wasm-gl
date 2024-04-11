use std::{cell::RefCell, rc::Rc, sync::Arc};

use egui::mutex::Mutex;
use glam::Vec4;
use wasm_bindgen::JsValue;

use crate::{
    core::Game,
    graphics::{EguiRenderer, Renderer, TextureDef, TextureLoader, TextureType},
    utils::performance_now,
};

const BLOCKS_ATLAS_PATH: &str = "data/textures/blocks/blocks_atlas.png";
const UV_CHECKER_PATH: &str = "data/textures/UV_checker_Map_byValle.jpg";

pub struct GameRenderer<R>
where
    R: Renderer,
    // G: EguiRenderer<R>,
{
    pub(super) game: Arc<Mutex<Game>>,
    pub(super) renderer: Rc<RefCell<R>>,
    pub(super) texture_loader: TextureLoader<R>,
    pub(super) loaded_textures: Vec<Rc<TextureDef<R>>>,
    // egui_renderer: Option<G>,
    // egui: Option<EguiBackend>,
    // world_renderer: WorldRenderer<R>,
}

impl<R> GameRenderer<R>
where
    R: Renderer,
{
    pub fn new(game: Arc<Mutex<Game>>, renderer: R) -> Result<Self, JsValue> {
        let renderer = Rc::new(RefCell::new(renderer));
        Ok(Self {
            game: game.clone(),
            renderer: renderer.clone(),
            texture_loader: TextureLoader::new(10, renderer)?,
            loaded_textures: Vec::new(),
            // egui_renderer: None,
            // egui: EguiBackend::new(gl),
        })
    }

    fn tick_loading(&mut self) -> Result<(), String> {
        for (path, t) in [
            (BLOCKS_ATLAS_PATH, TextureType::Texture2DArray(16)),
            (UV_CHECKER_PATH, TextureType::Texture2D),
        ] {
            let key = self.texture_loader.load(path, t)?;
            self.loaded_textures.push(Rc::new((t, key)));
        }

        // self.world
        //     .setup_graphics(gl, self.loaded_textures[0].clone())?;

        Ok(())
    }

    pub fn tick(&mut self) -> Result<(), String> {
        let _start = performance_now();
        self.texture_loader.tick()?;
        let update_time = performance_now();

        self.render()?;
        let render_time = performance_now();

        self.draw_ui();

        let frame_time = performance_now();
        {
            let mut game = self.game.lock();
            game.last_render_time = render_time - update_time;
            game.last_render_ui_time = frame_time - render_time;
        }

        Ok(())
    }

    fn render(&mut self) -> Result<(), String> {
        {
            let mut renderer = self.renderer.borrow_mut();
            let mut color: Vec4 = Vec4::new(0.0, 0.0, 0.0, 1.0);
            {
                let game = self.game.lock();
                color = game.clear_color;
            }
            renderer.clear(color.x, color.y, color.z, color.w);
        }

        // self.world.render(gl, &self.camera);

        // self.player.render(gl, &self.camera);
        // self.gizmo.render_lazy(gl, &self.camera);

        Ok(())
    }

    fn draw_ui(&mut self) {
        // if let Some(egui) = &mut self.egui {
        //     egui.render_ui(gl, |ctx| {});
        // }
    }
}
