use std::{
    collections::{hash_map, HashMap},
    rc::Rc,
};

use egui::TextureId;
use glow::WebTextureKey;
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
                vec!()
            )
            .compile(gl)
        }
        .expect("Cant compile eguis shaders");
        let program = Rc::new(program);
        Self {
            egui_ctx: egui::Context::default(),
            egui_once: true,
            mesh_renderer: MeshRenderer::new(gl, &program).expect("Cant init eguis mesh renderer"),
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

        self.paint(gl, full_output.textures_delta, clipped_primitives);
    }

    fn paint(
        &mut self,
        gl: &glow::Context,
        textures_delta: egui::TexturesDelta,
        clipped_primitives: Vec<egui::ClippedPrimitive>,
    ) {
        self.debug_paint(textures_delta, &clipped_primitives);
        let mesh = self.make_mesh(&clipped_primitives);
        self.mesh_renderer.set_mesh(gl, mesh);
        self.mesh_renderer.render(gl);
    }

    fn make_mesh(&self, clipped_primitives: &Vec<egui::ClippedPrimitive>) -> Rc<Mesh> {
        let data: Vec<f32> = if clipped_primitives.len() < 1 {
            vec![]
        } else {
            vec![
                -1.0, -1.0, 0.0, 1.0, 0.1, 0.1, 0.1, 1.0, //
                1.0, -1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, //
                -1.0, 1.0, 0.0, 0.0, 0.1, 0.1, 0.1, 1.0, //
                1.0, 1.0, 1.0, 0.0, 0.1, 0.1, 0.1, 1.0, //
            ]
        };
        Rc::new(Mesh {
            data,
            display_type: crate::mesh::MeshDisplayType::TriangleStrip,
            layout: vec![
                (VertexAttrType::Position, 2),
                (VertexAttrType::UVs, 2),
                (VertexAttrType::Color, 4),
            ],
        })
    }

    fn debug_paint(
        &mut self,
        textures_delta: egui::TexturesDelta,
        clipped_primitives: &Vec<egui::ClippedPrimitive>,
    ) {
        if self.egui_once {
            for (id, img_delta) in textures_delta.set {
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
                max: egui::pos2(800.0, 600.0),
            }),
            ..Default::default()
        }
    }
}
