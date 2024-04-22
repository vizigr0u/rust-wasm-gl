use std::sync::mpsc::Receiver;

use log::{info, warn};
use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;

use crate::{
    core::{commands::SerializedCommand, Game, GameSharedState},
    graphics::{EguiRenderer, Renderer, TextureType},
    utils::GlobalProxy,
};

use super::{BasicCommand, Graphics};

const BLOCKS_ATLAS_PATH: &str = "data/textures/blocks/blocks_atlas.png";
const UV_CHECKER_PATH: &str = "data/textures/UV_checker_Map_byValle.jpg";

// #[derive(Default)]
// struct RenderState {
//     clear_color: Vec4,
// }

pub struct GameRenderer<R>
where
    R: Renderer,
    // G: EguiRenderer<R>,
{
    // pub(super) game_state: Arc<Mutex<GameSharedState>>,
    pub(super) renderer: R,
    // pub(super) texture_loader: TextureLoader<R>,
    // pub(super) loaded_textures: Vec<Rc<TextureDef<R>>>,
    // egui_renderer: Option<G>,
    // egui: Option<EguiBackend>,
    // world_renderer: WorldRenderer<R>,
    // local_state: GameSharedState,
    // render_state: RenderState,
    command_receiver: Receiver<SerializedCommand<Graphics>>,
    context: GlobalProxy,
}

impl<R> GameRenderer<R>
where
    R: Renderer,
{
    pub async fn from_canvas(
        // game_state: Arc<Mutex<GameSharedState>>,
        canvas: HtmlCanvasElement,
        command_receiver: Receiver<SerializedCommand<Graphics>>,
    ) -> Result<Self, JsValue> {
        let renderer = R::from_canvas(canvas).await?;
        Ok(Self {
            context: GlobalProxy::new()?,
            // game_state: game_state.clone(),
            renderer,
            // texture_loader: TextureLoader::new(16, renderer)?,
            // loaded_textures: Vec::with_capacity(16),
            // local_state: Default::default(),
            // render_state: Default::default(),
            command_receiver, // egui_renderer: None,
                              // egui: EguiBackend::new(gl),
        })
    }

    fn tick_loading(&mut self) -> Result<(), String> {
        for (path, t) in [
            (BLOCKS_ATLAS_PATH, TextureType::Texture2DArray(16)),
            (UV_CHECKER_PATH, TextureType::Texture2D),
        ] {
            // let key = self.texture_loader.load(path, t)?;
            // self.loaded_textures.push(Rc::new((t, key)));
        }

        // self.world
        //     .setup_graphics(gl, self.loaded_textures[0].clone())?;

        Ok(())
    }

    pub fn tick(&mut self) -> Result<(), String> {
        let _start = self.context.performance_now();
        self.execute_commands();

        // self.texture_loader.tick()?;
        let update_time = self.context.performance_now();

        self.render()?;
        let render_time = self.context.performance_now();

        self.draw_ui();

        let frame_time = self.context.performance_now();
        {
            // let mut game = self.game_state.lock();
            // game.last_render_time = render_time - update_time;
            // game.last_render_ui_time = frame_time - render_time;
        }

        Ok(())
    }

    fn render(&mut self) -> Result<(), String> {
        {
            self.renderer.begin_render();
            self.renderer.end_render();
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

    fn execute_command(&mut self, c: SerializedCommand<Graphics>) {
        match c.parse() {
            Ok(basic) => match basic {
                BasicCommand::SetClearColor(color) => {
                    self.renderer
                        .set_clear_color(color[0], color[1], color[2], color[3]);
                }
                BasicCommand::CreateTexture(_t) => todo!(),
            },
            Err(e) => warn!("Failed to parse graphics command {:?}: {e}", c),
        }
    }
}
