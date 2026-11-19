//! Renderer seam: `Game` talks to one `Renderer` type, chosen at compile time by cargo feature.
//! A canvas can only hold one context type, so there is no `dyn`.

use glam::Mat4;

use crate::world::ChunkPos;

#[cfg(feature = "gl")]
mod gl;
#[cfg(feature = "gl")]
pub use gl::Renderer;

#[cfg(not(feature = "gl"))]
compile_error!("enable a renderer backend feature (`gl`)");

#[derive(Debug)]
pub struct RenderError(pub String);

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<wasm_bindgen::JsValue> for RenderError {
    fn from(v: wasm_bindgen::JsValue) -> Self {
        RenderError(format!("{v:?}"))
    }
}

impl From<String> for RenderError {
    fn from(s: String) -> Self {
        RenderError(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureId {
    /// 2D array atlas of block textures.
    Blocks,
    UvChecker,
}

/// Everything the renderer needs to draw one 3D frame. Plain data, no GPU types.
#[derive(Debug, Clone)]
pub struct FrameInput {
    pub view: Mat4,
    pub projection: Mat4,
    pub player_chunk: ChunkPos,
    pub player_model: Option<Mat4>,
    pub gizmo_model: Option<Mat4>,
    pub clear_color: [f32; 4],
}

/// Tessellated egui output for one frame.
pub struct UiFrame {
    pub textures_delta: egui::TexturesDelta,
    pub primitives: Vec<egui::ClippedPrimitive>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RenderStats {
    /// Draw calls issued by the last `render`.
    pub draw_calls: u32,
    pub chunk_buffers: usize,
}

impl From<&str> for RenderError {
    fn from(s: &str) -> Self {
        RenderError(s.to_string())
    }
}
