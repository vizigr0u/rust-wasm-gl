use std::rc::Rc;

use glam::{vec3, Mat4};
use glow::HasContext;
use wasm_bindgen::JsValue;

use crate::material::{Material, TextureType};
use crate::mesh::VertexAttrType;
use crate::meshrenderer::MeshRenderer;
use crate::quad::Quad;
use crate::shader_def;
use crate::shaders::{ShaderDef, UniformTypes};
use crate::textureloader::TextureLoader;
use crate::trianglescene::TriangleScene;

const GRASS_TEXTURE_PATH: &str = "data/textures/blocks/grass_block_side.png";
const SAND_TEXTURE_PATH: &str = "data/textures/blocks/sand.png";

pub struct Game {
    scene: TriangleScene,
    texture_loader: TextureLoader,
    quads: Vec<Quad>,
    quad_renderer: Option<MeshRenderer>,
}

impl Game {
    pub fn new() -> Result<Self, JsValue> {
        Ok(Game {
            scene: TriangleScene::new(),
            texture_loader: TextureLoader::new(10)?,
            quads: Vec::new(),
            quad_renderer: None,
        })
    }

    pub fn update(&mut self, time: f64) -> Result<(), String> {
        for (index, quad) in self.quads.iter_mut().enumerate() {
            quad.position = vec3(
                f64::sin(2.0 * index as f64 + time / 1000.0) as f32,
                f64::cos(2.0 * index as f64 + time / 200.0) as f32,
                0.0,
            );
        }

        self.scene.update(time)?;

        Ok(())
    }

    pub unsafe fn init(&mut self, gl: &glow::Context) -> Result<(), String> {
        self.scene.init(gl)?;
        self.init_quads(gl)?;

        Ok(())
    }

    pub unsafe fn init_quads(&mut self, gl: &glow::Context) -> Result<(), String> {
        let grass_key = self.texture_loader.load(gl, GRASS_TEXTURE_PATH)?;
        let dirt_key = self.texture_loader.load(gl, SAND_TEXTURE_PATH)?;

        let quad_shader = shader_def!(
            "textureTransform.vert",
            "textureTransform.frag",
            vec!(
                (VertexAttrType::Position, "position"),
                (VertexAttrType::UVs, "uv")
            ),
            vec!(
                (UniformTypes::ModelMatrix, "model"),
                (UniformTypes::ViewMatrix, "view"),
                (UniformTypes::ProjMatrix, "projection"),
            )
        )
        .compile(gl)?;
        let quad_shader_ref = Rc::new(quad_shader);
        let mut mat = Material::from_shader(&quad_shader_ref);
        mat.texture = Some((TextureType::Texture2D, grass_key));
        let mut mat2 = mat.clone();
        mat2.texture = Some((TextureType::Texture2D, dirt_key));
        let mat1 = Rc::new(mat);
        let mat2 = Rc::new(mat2);

        let quad_mesh = Rc::new(Quad::make_mesh());
        self.quad_renderer = Some(MeshRenderer::load(gl, mat1, quad_mesh)?);
        for i in 0..50 {
            let quad: Quad = Quad::new();
            self.quads.push(quad);
        }

        Ok(())
    }

    pub unsafe fn render(&mut self, gl: &glow::Context) -> Result<(), String> {
        self.texture_loader.tick(&gl)?;
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);

        if let Some(quad_renderer) = &self.quad_renderer {
            for quad in &self.quads {
                let (position, rotation, scale) = (quad.position, quad.rotation, quad.scale);
                quad_renderer.draw(
                    gl,
                    &Mat4::from_scale_rotation_translation(scale, rotation, position),
                );
            }
        }

        self.scene.render(gl);

        Ok(())
    }
}
