use std::{
    cell::RefCell,
    rc::Rc,
    sync::{mpsc::Receiver, Arc},
};

use egui::mutex::Mutex;
use glam::Vec4;
use log::{info, warn};
use wasm_bindgen::JsValue;

use crate::{
    core::{Game, GameSharedState},
    graphics::{EguiRenderer, Renderer, TextureDef, TextureLoader, TextureType},
    utils::performance_now,
};

use super::commands::{self, CommandQueue, SerializedCommand};

const BLOCKS_ATLAS_PATH: &str = "data/textures/blocks/blocks_atlas.png";
const UV_CHECKER_PATH: &str = "data/textures/UV_checker_Map_byValle.jpg";

#[derive(Default)]
struct RenderState {
    clear_color: Vec4,
}

pub struct GameRenderer<R>
where
    R: Renderer,
    // G: EguiRenderer<R>,
{
    // pub(super) game_state: Arc<Mutex<GameSharedState>>,
    pub(super) renderer: Rc<RefCell<R>>,
    pub(super) texture_loader: TextureLoader<R>,
    pub(super) loaded_textures: Vec<Rc<TextureDef<R>>>,
    // egui_renderer: Option<G>,
    // egui: Option<EguiBackend>,
    // world_renderer: WorldRenderer<R>,
    local_state: GameSharedState,
    render_state: RenderState,
    command_receiver: Receiver<SerializedCommand>,
}

impl<R> GameRenderer<R>
where
    R: Renderer,
{
    pub fn new(
        // game_state: Arc<Mutex<GameSharedState>>,
        renderer: R,
        command_receiver: Receiver<SerializedCommand>,
    ) -> Result<Self, JsValue> {
        let renderer = Rc::new(RefCell::new(renderer));
        Ok(Self {
            // game_state: game_state.clone(),
            renderer: renderer.clone(),
            texture_loader: TextureLoader::new(16, renderer)?,
            loaded_textures: Vec::with_capacity(16),
            local_state: Default::default(),
            render_state: Default::default(),
            command_receiver, // egui_renderer: None,
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
        self.execute_commands();

        self.texture_loader.tick()?;
        let update_time = performance_now();

        self.render()?;
        let render_time = performance_now();

        self.draw_ui();

        let frame_time = performance_now();
        {
            // let mut game = self.game_state.lock();
            // game.last_render_time = render_time - update_time;
            // game.last_render_ui_time = frame_time - render_time;
        }

        Ok(())
    }

    fn render(&mut self) -> Result<(), String> {
        {
            let mut renderer = self.renderer.borrow_mut();
            let color = self.render_state.clear_color;
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

    fn execute_commands(&mut self) {
        loop {
            match self.command_receiver.try_recv() {
                Ok(c) => self.execute_command(c),
                Err(_) => {
                    break;
                }
            }
        }
        // info!("executing {} commands", self.commands.len());
        // self.commands.clear();
    }

    fn execute_command(&mut self, c: SerializedCommand) {
        match c.command_type {
            1 => match c.parse_basic() {
                Ok(basic) => match basic {
                    commands::BasicCommand::SetClearColor(color) => {
                        self.render_state.clear_color =
                            Vec4::new(color[0], color[1], color[2], color[3]);
                    }
                },
                Err(_) => todo!(),
            },
            _ => {
                info!("Unknown Command: {:?}", c);
            }
        }
    }
}
