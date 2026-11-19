use std::{collections::HashMap, rc::Rc};

use egui::{epaint::Primitive, TextureFilter, TextureId, TextureWrapMode};
use glow::{HasContext, WebTextureKey};
use log::warn;

use crate::{
    graphics::{Mesh, MeshRenderer, ShaderDef, ShaderProgram, UniformTypes, VertexAttrType},
    shader_def,
    utils::GlRenderFlags,
};

const RENDER_FLAGS: [GlRenderFlags; 1] = [GlRenderFlags::Blend];

/// The GL painting half of egui.
#[derive(Debug)]
pub struct EguiPainter {
    textures: HashMap<TextureId, WebTextureKey>,
    mesh_renderer: MeshRenderer,
}

impl EguiPainter {
    pub fn compile_shader(gl: &glow::Context) -> Result<ShaderProgram, String> {
        unsafe {
            shader_def!(
                "egui.vert",
                "egui.frag",
                vec!(
                    (VertexAttrType::Position, "position"),
                    (VertexAttrType::UVs, "uv"),
                    (VertexAttrType::Color, "color"),
                ),
                vec!((UniformTypes::ProjMatrix, "u_ortho"),)
            )
            .compile(gl)
        }
    }

    pub fn new(gl: &glow::Context) -> Self {
        let program = Rc::new(Self::compile_shader(gl).expect("Cant compile eguis shaders"));
        Self {
            mesh_renderer: MeshRenderer::with_render_flags(&RENDER_FLAGS, &program),
            textures: HashMap::new(),
        }
    }

    pub fn paint(
        &mut self,
        gl: &glow::Context,
        size: (u32, u32),
        mut textures_delta: egui::TexturesDelta,
        clipped_primitives: &[egui::ClippedPrimitive],
    ) {
        self.update_textures(gl, &textures_delta);
        for id in &textures_delta.free {
            if let Some(key) = self.textures.remove(id) {
                unsafe { gl.delete_texture(key) };
            }
        }
        textures_delta.clear();
        let (width, height) = (size.0 as f32, size.1 as f32);
        unsafe { gl.enable(glow::SCISSOR_TEST) };
        for primitive in clipped_primitives {
            match &primitive.primitive {
                Primitive::Mesh(p) => {
                    unsafe {
                        let tex = self.textures.get(&p.texture_id);
                        match tex {
                            Some(t) => gl.bind_texture(glow::TEXTURE_2D, Some(*t)),
                            None => {
                                warn!("Texture not found: {:?}", p.texture_id);
                                continue;
                            }
                        }
                    }
                    // GL scissor origin is bottom-left; egui's is top-left.
                    let clip = primitive.clip_rect;
                    let x0 = clip.min.x.clamp(0.0, width).round();
                    let x1 = clip.max.x.clamp(0.0, width).round();
                    let y0 = clip.min.y.clamp(0.0, height).round();
                    let y1 = clip.max.y.clamp(0.0, height).round();
                    unsafe {
                        gl.scissor(
                            x0 as i32,
                            (height - y1) as i32,
                            (x1 - x0).max(0.0) as i32,
                            (y1 - y0).max(0.0) as i32,
                        );
                    }
                    let mesh = Rc::new(Mesh::from(p));
                    self.mesh_renderer
                        .set_mesh(gl, mesh)
                        .expect("Can't set egui mesh");
                    let program = self.mesh_renderer.get_program();
                    program.gl_use(gl);
                    program.set_matrix(
                        gl,
                        UniformTypes::ProjMatrix,
                        &self.make_projection(width, height),
                    );
                    unsafe {
                        gl.blend_equation_separate(glow::FUNC_ADD, glow::FUNC_ADD);
                        gl.blend_func_separate(
                            // egui outputs colors with premultiplied alpha:
                            glow::ONE,
                            glow::ONE_MINUS_SRC_ALPHA,
                            // Less important, but this is technically the correct alpha blend function
                            // when you want to make use of the framebuffer alpha (for screenshots, compositing, etc).
                            glow::ONE_MINUS_DST_ALPHA,
                            glow::ONE,
                        );
                    }
                    self.mesh_renderer.render(gl);
                }
                _ => {
                    warn!("Cant render egui callback");
                }
            }
        }
        unsafe { gl.disable(glow::SCISSOR_TEST) };
    }

    fn make_projection(&self, width: f32, height: f32) -> glam::Mat4 {
        glam::camera::rh::proj::opengl::orthographic(0.0, width, height, 0.0, -1.0, 1.0)
    }

    fn update_textures(&mut self, gl: &glow::Context, textures_delta: &egui::TexturesDelta) {
        unsafe {
            for (id, deltas) in &textures_delta.set {
                for img_delta in deltas {
                    if let Some(pos) = img_delta.pos {
                        let Some(key) = self.textures.get(id).copied() else {
                            warn!("Partial update of unknown texture {:?}", id);
                            continue;
                        };
                        gl.bind_texture(glow::TEXTURE_2D, Some(key));
                        match &img_delta.image {
                            egui::ImageData::Color(image) => {
                                let data: &[u8] = bytemuck::cast_slice(image.pixels.as_ref());
                                gl.tex_sub_image_2d(
                                    glow::TEXTURE_2D,
                                    0,
                                    pos[0] as _,
                                    pos[1] as _,
                                    image.width() as _,
                                    image.height() as _,
                                    glow::RGBA,
                                    glow::UNSIGNED_BYTE,
                                    glow::PixelUnpackData::Slice(Some(data)),
                                );
                            }
                        }
                        continue;
                    }
                    let options = &img_delta.options;
                    if let Some(old) = self.textures.remove(id) {
                        gl.delete_texture(old);
                    }
                    let key: WebTextureKey = gl.create_texture().expect("Can't create texture");
                    gl.bind_texture(glow::TEXTURE_2D, Some(key));
                    let wrap_mode = wrap_to_glow(options.wrap_mode);
                    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, wrap_mode as i32);
                    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, wrap_mode as i32);
                    gl.tex_parameter_i32(
                        glow::TEXTURE_2D,
                        glow::TEXTURE_MIN_FILTER,
                        filter_to_glow(options.minification) as i32,
                    );
                    gl.tex_parameter_i32(
                        glow::TEXTURE_2D,
                        glow::TEXTURE_MAG_FILTER,
                        filter_to_glow(options.magnification) as i32,
                    );
                    let (internal_format, src_format) = (glow::SRGB8_ALPHA8, glow::RGBA); // if not SRGB : (glow::RGBA8, glow::RGBA);
                    match &img_delta.image {
                        egui::ImageData::Color(image) => {
                            let data: &[u8] = bytemuck::cast_slice(image.pixels.as_ref());
                            gl.tex_image_2d(
                                glow::TEXTURE_2D,
                                0,
                                internal_format as _,
                                image.width() as _,
                                image.height() as _,
                                0,
                                src_format,
                                glow::UNSIGNED_BYTE,
                                glow::PixelUnpackData::Slice(Some(data)),
                            );
                        }
                    };
                    self.textures.insert(*id, key);
                }
            }
        }
    }
}

/* convert a TextureFilter to a glow filter */
fn filter_to_glow(filter: TextureFilter) -> u32 {
    match filter {
        TextureFilter::Linear => glow::LINEAR,
        TextureFilter::Nearest => glow::NEAREST,
    }
}

/* convert a TextureWrapMode to a glow filter */
fn wrap_to_glow(wrap: TextureWrapMode) -> u32 {
    match wrap {
        TextureWrapMode::ClampToEdge => glow::CLAMP_TO_EDGE,
        TextureWrapMode::Repeat => glow::REPEAT,
        TextureWrapMode::MirroredRepeat => glow::MIRRORED_REPEAT,
    }
}

impl From<&egui::Mesh> for Mesh {
    fn from(mesh: &egui::Mesh) -> Self {
        let mut data = Vec::<f32>::with_capacity(mesh.vertices.len() * 8);
        for v in &mesh.vertices {
            data.push(v.pos.x);
            data.push(v.pos.y);
            data.push(v.uv.x);
            data.push(v.uv.y);
            data.push(v.color.r() as f32 / 255.0);
            data.push(v.color.g() as f32 / 255.0);
            data.push(v.color.b() as f32 / 255.0);
            data.push(v.color.a() as f32 / 255.0);
        }
        Mesh {
            data,
            primitive_type: glow::TRIANGLES,
            layout: vec![
                (VertexAttrType::Position, 2),
                (VertexAttrType::UVs, 2),
                (VertexAttrType::Color, 4),
            ],
            indices: Some(mesh.indices.clone()),
        }
    }
}
