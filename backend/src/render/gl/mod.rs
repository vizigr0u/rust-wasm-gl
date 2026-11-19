//! WebGL2 renderer (glow). The only place the game touches GL.

use std::collections::HashMap;

use glow::HasContext;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, HtmlImageElement, WebGl2RenderingContext};

use super::{FrameInput, RenderError, RenderStats, TextureId, UiFrame};
use crate::world::{ChunkDrawData, ChunkPos};

mod egui_painter;
mod mesh;
mod meshrenderer;
mod objects;
mod shaders;
mod texture;
mod world;

use egui_painter::EguiPainter;
pub use mesh::*;
pub use meshrenderer::*;
pub use objects::*;
pub use shaders::*;
pub use texture::*;
pub use world::*;

const PLACEHOLDER_TEXEL: [u8; 4] = [255, 0, 255, 255];

#[derive(Debug)]
pub struct Renderer {
    gl: glow::Context,
    textures: HashMap<TextureId, TextureDef>,
    world: WorldRenderData,
    objects: ObjectRenderers,
    egui: EguiPainter,
    size: (u32, u32),
    pass_ms: f64,
    egui_ms: f64,
}

fn texture_type(id: TextureId, layers: u32) -> TextureType {
    match id {
        TextureId::Blocks => TextureType::Texture2DArray(layers),
        TextureId::UvChecker => TextureType::Texture2D,
    }
}

impl Renderer {
    /// Async so the wgpu backend can share the signature (adapter/device are async).
    pub async fn new(canvas: HtmlCanvasElement) -> Result<Self, RenderError> {
        let ctx = canvas
            .get_context("webgl2")?
            .ok_or("Unable to get WebGl2 context from canvas")?
            .dyn_into::<WebGl2RenderingContext>()
            .map_err(|_| "canvas context is not WebGL2")?;
        let gl = glow::Context::from_webgl2_context(ctx);

        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LESS);
            gl.enable(glow::CULL_FACE);
        }

        let mut textures = HashMap::new();
        for (id, layers) in [(TextureId::Blocks, 1), (TextureId::UvChecker, 1)] {
            let ty = texture_type(id, layers);
            textures.insert(id, (ty, create_placeholder(&gl, ty)?));
        }

        let world = WorldRenderData::new(&gl)?;
        let objects = ObjectRenderers::new(&gl)?;
        let egui = EguiPainter::new(&gl);
        let size = (canvas.width(), canvas.height());

        Ok(Self {
            gl,
            textures,
            world,
            objects,
            egui,
            size,
            pass_ms: 0.0,
            egui_ms: 0.0,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.size = (width, height);
        unsafe { self.gl.viewport(0, 0, width as i32, height as i32) };
    }

    /// Upload a decoded image into the texture `id`. `layers` is the array depth (blocks atlas).
    pub fn load_texture(&mut self, id: TextureId, img: &HtmlImageElement, layers: u32) {
        let gl = &self.gl;
        let ty = texture_type(id, layers);
        let Some(entry) = self.textures.get_mut(&id) else {
            return;
        };
        entry.0 = ty;
        let target: u32 = ty.into();
        unsafe {
            gl.bind_texture(target, Some(entry.1));
            gl.tex_parameter_i32(target, glow::TEXTURE_MIN_FILTER, glow::NEAREST as _);
            gl.tex_parameter_i32(target, glow::TEXTURE_MAG_FILTER, glow::NEAREST as _);
            match ty {
                TextureType::Texture2D => gl.tex_image_2d_with_html_image(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA as _,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    img,
                ),
                TextureType::Texture2DArray(depth) => gl.tex_image_3d_with_html_image_element(
                    target,
                    0,
                    glow::RGBA as _,
                    img.natural_width() as _,
                    (img.natural_height() / depth) as _,
                    depth as _,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    img,
                ),
            }
            gl.generate_mipmap(target);
        }
    }

    pub fn upload_chunk(&mut self, pos: ChunkPos, data: &ChunkDrawData) {
        self.world.upload_chunk(&self.gl, pos, data);
    }

    pub fn remove_chunk(&mut self, pos: ChunkPos) {
        self.world.remove_chunk(&self.gl, pos);
    }

    pub fn render(&mut self, frame: &FrameInput, ui: Option<UiFrame>) {
        let t0 = crate::utils::performance_now();
        let gl = &self.gl;
        let [r, g, b, a] = frame.clear_color;
        unsafe {
            gl.clear_color(r, g, b, a);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        }
        // Objects sample whatever the world draw left bound (the block atlas).
        self.world.render(
            gl,
            &self.textures[&TextureId::Blocks],
            &frame.view,
            &frame.projection,
            frame.player_chunk,
        );
        if let Some(m) = &frame.player_model {
            self.objects
                .draw_cube(gl, &frame.view, &frame.projection, m);
        }
        if let Some(m) = &frame.gizmo_model {
            self.objects
                .draw_gizmo(gl, &frame.view, &frame.projection, m);
        }
        let t1 = crate::utils::performance_now();
        if let Some(ui) = ui {
            self.egui
                .paint(gl, self.size, ui.textures_delta, &ui.primitives);
        }
        let t2 = crate::utils::performance_now();
        self.pass_ms = t1 - t0;
        self.egui_ms = t2 - t1;
    }

    pub fn stats(&self) -> RenderStats {
        RenderStats {
            draw_calls: self.world.draw_calls(),
            chunk_buffers: self.world.chunk_buffers(),
            // GL has no separate prepare/acquire/submit phases: all in the pass.
            prepare_ms: 0.0,
            acquire_ms: 0.0,
            pass_ms: self.pass_ms,
            egui_ms: self.egui_ms,
            submit_ms: 0.0,
            ..Default::default()
        }
    }

    /// Compile and link every shader program. One message per failure; empty = all good.
    #[cfg_attr(not(feature = "test-hooks"), allow(dead_code))]
    pub fn compile_check(&self) -> impl std::future::Future<Output = Vec<String>> + 'static {
        let gl = &self.gl;
        let mut errors = Vec::new();
        let mut check = |name: &str, r: Result<(), String>| {
            if let Err(e) = r {
                errors.push(format!("{name}: {e}"));
            }
        };
        check("chunk", compile_shader(gl).map(|_| ()));
        check("cube", compile_cube_shader(gl).map(|_| ()));
        check("gizmo", compile_gizmo_shader(gl).map(|_| ()));
        check("egui", EguiPainter::compile_shader(gl).map(|_| ()));
        std::future::ready(errors)
    }
}

/// 1x1 magenta texture, so a failed load shows up instead of sampling nothing.
fn create_placeholder(gl: &glow::Context, ty: TextureType) -> Result<glow::WebTextureKey, String> {
    let target: u32 = ty.into();
    unsafe {
        let tex = gl.create_texture()?;
        gl.bind_texture(target, Some(tex));
        gl.tex_parameter_i32(target, glow::TEXTURE_MIN_FILTER, glow::NEAREST as _);
        gl.tex_parameter_i32(target, glow::TEXTURE_MAG_FILTER, glow::NEAREST as _);
        let data = glow::PixelUnpackData::Slice(Some(&PLACEHOLDER_TEXEL));
        match ty {
            TextureType::Texture2D => gl.tex_image_2d(
                target,
                0,
                glow::RGBA as _,
                1,
                1,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                data,
            ),
            TextureType::Texture2DArray(_) => gl.tex_image_3d(
                target,
                0,
                glow::RGBA as _,
                1,
                1,
                1,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                data,
            ),
        }
        Ok(tex)
    }
}
