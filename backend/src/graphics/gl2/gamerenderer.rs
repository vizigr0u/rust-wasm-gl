use std::{cell::RefCell, rc::Rc, sync::Arc};

use egui::mutex::Mutex;
use glam::Vec4;
use wasm_bindgen::JsValue;

use crate::{
    core::Game,
    graphics::{Renderer, TextureDef, TextureLoader, TextureType},
    utils::performance_now,
};

use super::Gl2Renderer;

// const BLOCKS_ATLAS_PATH: &str = "data/textures/blocks/blocks_atlas.png";
// const UV_CHECKER_PATH: &str = "data/textures/UV_checker_Map_byValle.jpg";

// // impl GameRenderer<Gl2Renderer> {
// pub struct Gl2GameRenderer {
//     pub(super) game: Arc<Mutex<Game>>,
//     pub(super) renderer: Rc<RefCell<Gl2Renderer>>,
//     pub(super) texture_loader: TextureLoader<Gl2Renderer>,
//     pub(super) loaded_textures: Vec<Rc<TextureDef<Gl2Renderer>>>,
// }
