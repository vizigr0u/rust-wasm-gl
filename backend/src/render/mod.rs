//! Renderer seam: `Game` talks to one `Renderer` type, chosen at compile time by cargo feature.
//! A canvas can only hold one context type, so there is no `dyn`.

use glam::Mat4;

use crate::world::ChunkPos;

#[cfg(feature = "gl")]
pub(crate) mod gl;
#[cfg(feature = "gl")]
pub use gl::Renderer;

#[cfg(feature = "webgpu")]
pub(crate) mod webgpu;
#[cfg(all(feature = "webgpu", not(feature = "gl")))]
pub use webgpu::Renderer;

#[cfg(all(feature = "gl", feature = "webgpu"))]
compile_error!("enable exactly one renderer backend feature (`gl` or `webgpu`)");
#[cfg(not(any(feature = "gl", feature = "webgpu")))]
compile_error!("enable a renderer backend feature (`gl` or `webgpu`)");

/// Clip-space z range of the active backend's projection.
#[cfg(feature = "gl")]
pub const CLIP_Z: voxel_core::camera::ClipZ = voxel_core::camera::ClipZ::NegOneToOne;
#[cfg(not(feature = "gl"))]
pub const CLIP_Z: voxel_core::camera::ClipZ = voxel_core::camera::ClipZ::ZeroToOne;

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
    /// CPU ms of the last `render` spent preparing (draw list, per-frame uploads).
    pub prepare_ms: f64,
    /// CPU ms of the last `render`: surface acquire + view (webgpu only).
    pub acquire_ms: f64,
    /// CPU ms of the last `render`: encoding the main 3D pass.
    pub pass_ms: f64,
    /// CPU ms of the last `render`: egui buffers, pass and paint.
    pub egui_ms: f64,
    /// CPU ms of the last `render`: submit + present (webgpu only).
    pub submit_ms: f64,
}

impl From<&str> for RenderError {
    fn from(s: &str) -> Self {
        RenderError(s.to_string())
    }
}
