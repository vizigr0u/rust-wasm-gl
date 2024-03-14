use std::{collections::HashMap, rc::Rc};

use egui::{epaint::Primitive, TextureFilter, TextureId, TextureWrapMode};
use glow::{HasContext, WebTextureKey};
use log::{info, warn};

use crate::{
    mesh::{Mesh, VertexAttrType},
    meshrenderer::MeshRenderer,
    shader_def,
    shaders::{ShaderDef, UniformTypes},
};

pub struct EguiBackend {
    egui_ctx: egui::Context,
    egui_once: bool,
    textures: HashMap<TextureId, WebTextureKey>,
    mesh_renderer: MeshRenderer,
}

impl EguiBackend {
    pub fn new(gl: &glow::Context) -> Self {
        let program = unsafe {
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
        .expect("Cant compile eguis shaders");
        let program = Rc::new(program);
        Self {
            egui_ctx: egui::Context::default(),
            egui_once: true,
            mesh_renderer: MeshRenderer::new(&program),
            textures: HashMap::new(),
        }
    }

    pub fn render(&mut self, gl: &glow::Context) {
        let raw_input: egui::RawInput = self.gather_input();

        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            egui::CentralPanel::default().show(&ctx, |ui| {
                ui.label("Hello world!");
                if ui.button("Click me").clicked() {
                    warn!("CLICK!");
                }
            });
        });
        self.handle_platform_output(full_output.platform_output);
        let clipped_primitives = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        unsafe {
            gl.disable(glow::CULL_FACE);
            gl.enable(glow::BLEND);
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
            self.paint(gl, full_output.textures_delta, clipped_primitives);
            gl.disable(glow::BLEND);
            gl.enable(glow::CULL_FACE);
        }
    }

    fn paint(
        &mut self,
        gl: &glow::Context,
        textures_delta: egui::TexturesDelta,
        clipped_primitives: Vec<egui::ClippedPrimitive>,
    ) {
        self.debug_paint(&textures_delta, &clipped_primitives);
        self.update_textures(gl, &textures_delta);
        for primitive in &clipped_primitives {
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
                    let mesh = Rc::new(Mesh::from(p));
                    self.mesh_renderer
                        .set_mesh(gl, mesh)
                        .expect("Can't set egui mesh");
                    let program = self.mesh_renderer.get_program();
                    program.gl_use(gl);
                    program.set_matrix(
                        gl,
                        UniformTypes::ProjMatrix,
                        &self.make_projection(800.0, 600.0),
                    );
                    self.mesh_renderer.render(gl);
                }
                _ => {
                    warn!("Cant render egui callback");
                }
            }
        }
    }

    fn make_projection(&self, width: f32, height: f32) -> glam::Mat4 {
        glam::Mat4::orthographic_rh_gl(0.0, width, height, 0.0, -1.0, 1.0)
    }

    // fn make_mesh(&self, mesh: &egui::Mesh) -> Rc<Mesh> {
    //     let mut data = Vec::<f32>::with_capacity(mesh.vertices.len() * 8);
    //     for v in &mesh.vertices {
    //         data.push(v.pos.x);
    //         data.push(v.pos.y);
    //         data.push(v.uv.x);
    //         data.push(v.uv.y);
    //         data.push(v.color.r() as f32 / 255.0);
    //         data.push(v.color.g() as f32 / 255.0);
    //         data.push(v.color.b() as f32 / 255.0);
    //         data.push(v.color.a() as f32 / 255.0);
    //     }
    //     Rc::new(Mesh {
    //         data,
    //         primitive_type: glow::TRIANGLES,
    //         layout: vec![
    //             (VertexAttrType::Position, 2),
    //             (VertexAttrType::UVs, 2),
    //             (VertexAttrType::Color, 4),
    //         ],
    //         indices: Some(mesh.indices.clone()),
    //     })
    // }

    fn update_textures(&mut self, gl: &glow::Context, textures_delta: &egui::TexturesDelta) {
        unsafe {
            for (id, img_delta) in &textures_delta.set {
                if !img_delta.is_whole() {
                    warn!("Partial update of texture {:?} not supported", id);
                    continue;
                }
                info!("loading texture: {:?}", id);
                let options = &img_delta.options;
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
                            Some(data),
                        );
                        self.textures.insert(*id, key);
                    }
                    egui::ImageData::Font(image) => {
                        gl.tex_image_2d(
                            glow::TEXTURE_2D,
                            0,
                            internal_format as _,
                            image.width() as _,
                            image.height() as _,
                            0,
                            src_format,
                            glow::UNSIGNED_BYTE,
                            Some(
                                &image
                                    .srgba_pixels(None)
                                    .flat_map(|p| p.to_array())
                                    .collect::<Vec<_>>(),
                            ),
                        );
                    }
                };
                self.textures.insert(*id, key);
            }
        }
    }

    fn debug_paint(
        &mut self,
        textures_delta: &egui::TexturesDelta,
        clipped_primitives: &Vec<egui::ClippedPrimitive>,
    ) {
        if self.egui_once {
            for (id, img_delta) in &textures_delta.set {
                info!("{id:?}, {:?}", img_delta.options);
            }
            for p in clipped_primitives {
                info!("prim: {:?}, {:?}", p.clip_rect, p.primitive);
            }
            self.egui_once = false;
        }
    }

    fn handle_platform_output(&self, _platform_output: egui::PlatformOutput) {}

    fn gather_input(&self) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect {
                min: egui::pos2(0.0, 0.0),
                max: egui::pos2(200.0, 200.0),
            }),
            ..Default::default()
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

// impl Into<Mesh> for egui::Mesh {
//     fn into(self) -> Mesh {
//         let mesh = self;
//         let mut data = Vec::<f32>::with_capacity(mesh.vertices.len() * 8);
//         for v in &mesh.vertices {
//             data.push(v.pos.x);
//             data.push(v.pos.y);
//             data.push(v.uv.x);
//             data.push(v.uv.y);
//             data.push(v.color.r() as f32 / 255.0);
//             data.push(v.color.g() as f32 / 255.0);
//             data.push(v.color.b() as f32 / 255.0);
//             data.push(v.color.a() as f32 / 255.0);
//         }
//         Mesh {
//             data,
//             primitive_type: glow::TRIANGLES,
//             layout: vec![
//                 (VertexAttrType::Position, 2),
//                 (VertexAttrType::UVs, 2),
//                 (VertexAttrType::Color, 4),
//             ],
//             indices: Some(mesh.indices.clone()),
//         }
//     }
// }

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
