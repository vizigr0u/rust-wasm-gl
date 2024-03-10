use std::rc::Rc;

use glam::{vec3, Mat4, Quat};
use glow::HasContext;
use wasm_bindgen::JsValue;

use crate::material::TextureType;
use crate::mesh::{Mesh, VertexAttrType};
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
}

impl Game {
    pub fn new() -> Result<Self, JsValue> {
        Ok(Game {
            scene: TriangleScene::new(),
            texture_loader: TextureLoader::new(10)?,
            quads: Vec::new(),
        })
    }

    pub fn update(&mut self, time: f64) -> Result<(), String> {
        for (index, quad) in self.quads.iter_mut().enumerate() {
            quad.set_position(vec3(
                f64::sin(2.0 * index as f64 + time / 1000.0) as f32,
                f64::cos(2.0 * index as f64 + time / 200.0) as f32,
                0.0,
            ));
            quad.set_scale(vec3(0.2, f64::sin(time / 1000.0) as f32 * 0.2, 0.5));
            quad.update();
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

        let quad_program = shader_def!(
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
        let quad_program = Rc::new(quad_program);
        let quad_tex1 = Rc::new((TextureType::Texture2D, grass_key));
        let quad_tex2 = Rc::new((TextureType::Texture2D, dirt_key));

        let quad_mesh = Rc::new(Mesh::make_quad());
        let quad_renderer = Rc::new(MeshRenderer::load(gl, &quad_program, quad_mesh)?);
        for i in 0..50 {
            let tex = if i % 2 == 0 { &quad_tex1 } else { &quad_tex2 };
            let quad: Quad = Quad::new(tex, &quad_renderer);
            self.quads.push(quad);
        }

        Ok(())
    }

    pub unsafe fn render(&mut self, gl: &glow::Context) -> Result<(), String> {
        self.texture_loader.tick(&gl)?;
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);

        for quad in &self.quads {
            quad.render(gl);
        }

        self.scene.render(gl);

        Ok(())
    }
}
