use std::fmt::Debug;

use wasm_bindgen::JsValue;
use web_sys::HtmlImageElement;

use super::TextureType;

#[derive(Debug, Copy, Clone)]
pub enum RenderFlag {
    DepthTest,
    CullFace,
    Blend,
}

pub trait Renderer {
    async fn from_canvas(canvas: web_sys::HtmlCanvasElement) -> Result<Self, JsValue>
    where
        Self: Sized;
    fn begin_render(&mut self);
    fn end_render(&mut self);
    fn set_flag(&mut self, flag: RenderFlag, enabled: bool);
    fn set_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32);
}

pub trait TodoRenderer {
    type ShaderProgramKey: Copy;
    type TextureRef: Copy;
    type BufferRef: Copy;
    type VertexArrayKey: Copy;

    fn create_texture(&mut self) -> Self::TextureRef;
    fn load_texture(
        &mut self,
        key: Option<Self::TextureRef>,
        texture_type: TextureType,
        data: &[u8],
        width: i32,
        height: i32,
    );
    fn load_texture_from_image(
        &mut self,
        key: Option<Self::TextureRef>,
        texture_type: TextureType,
        img: &HtmlImageElement,
    );
    fn bind_texture(&mut self, texture_type: TextureType, key: Option<Self::TextureRef>);
}
