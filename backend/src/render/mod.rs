//! Renderer seam: `Game` talks to the wgpu (WebGPU) `Renderer`.

use glam::Mat4;
use voxel_core::math::Frustum;

use crate::world::ChunkPos;

pub(crate) mod webgpu;
pub use webgpu::Renderer;

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
    /// Cull chunks outside this frustum (player-chunk-relative space). `None` = no culling.
    pub cull_frustum: Option<Frustum>,
    /// Model matrix (inverse view-projection) of the debug frustum lines.
    pub frustum_lines: Option<Mat4>,
    /// Draw wireframe boxes around culled chunks.
    pub culled_boxes: bool,
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
    /// Draw entries before adjacent ones were merged.
    pub draw_entries_raw: u32,
    /// Arena occupancy, in quads (webgpu only).
    pub arena_used: u32,
    pub arena_capacity: u32,
    pub arena_largest_free: u32,
    /// Chunks with at least one drawn side, chunks culled by the frustum, quads drawn.
    pub chunks_visible: u32,
    pub chunks_culled: u32,
    pub quads_drawn: u32,
    /// Bytes uploaded to the arena before the last `render`.
    pub upload_bytes: u64,
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
    /// Absolute `performance.now()` marks t0..t5: start, acquired, prepared, pass, egui, submitted.
    pub marks: [f64; 6],
}

impl From<&str> for RenderError {
    fn from(s: &str) -> Self {
        RenderError(s.to_string())
    }
}
