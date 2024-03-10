use std::path;
use std::rc::Rc;

use glam::{vec3, Mat4, Quat, Vec3};
use glow::HasContext;
use wasm_bindgen::JsValue;

use crate::camera::Camera;
use crate::gameobject::GameObject;
use crate::material::{TextureDef, TextureType};
use crate::mesh::{Mesh, VertexAttrType};
use crate::meshrenderer::MeshRenderer;
use crate::shader_def;
use crate::shaders::{CompiledShader, ShaderDef, UniformTypes};
use crate::textureloader::TextureLoader;
use crate::trianglescene::TriangleScene;

const GRASS_TEXTURE_PATH: &str = "data/textures/blocks/grass_block_side.png";
const SAND_TEXTURE_PATH: &str = "data/textures/blocks/sand.png";
const DIRT_TEXTURE_PATH: &str = "data/textures/blocks/dirt.png";
const BLOCK_ATLAS_PATH: &str = "data/textures/blocks/atlas_blocks.png";

pub struct Game {
    scene: TriangleScene,
    texture_loader: TextureLoader,
    quads: Vec<GameObject>,
    cube: Option<GameObject>,
    camera: Camera,

    loaded_textures: Vec<Rc<TextureDef>>,
}

impl Game {
    pub fn new() -> Result<Self, JsValue> {
        Ok(Game {
            scene: TriangleScene::new(),
            texture_loader: TextureLoader::new(10)?,
            quads: Vec::new(),
            cube: None,
            loaded_textures: Vec::new(),
            camera: Camera {
                position: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: -10.0,
                },
                direction: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                up: Vec3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                projection: Mat4::IDENTITY,
                look_at: Mat4::IDENTITY,
            },
        })
    }

    pub unsafe fn init(&mut self, gl: &glow::Context) -> Result<(), String> {
        for path in [
            GRASS_TEXTURE_PATH,
            SAND_TEXTURE_PATH,
            DIRT_TEXTURE_PATH,
            BLOCK_ATLAS_PATH,
        ] {
            let key = self.texture_loader.load(gl, path)?;
            self.loaded_textures
                .push(Rc::new((TextureType::Texture2D, key)));
        }

        gl.enable(glow::CULL_FACE);
        self.camera.projection =
            Mat4::perspective_rh_gl(f32::to_degrees(45.0), 800.0 / 600.0, 0.1, 100.0);

        self.scene.init(gl)?;

        self.init_quads(gl)?;
        self.init_cube(gl)?;

        Ok(())
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

        if let Some(cube) = &mut self.cube {
            cube.set_rotation(Quat::from_euler(
                glam::EulerRot::XYZ,
                0.0,
                (time / 1000.0) as f32,
                (time / 2000.0) as f32,
            ));
            cube.update();
        }

        self.scene.update(time)?;

        self.camera.position = vec3(0.0, 0.0, -5.0 - (f64::cos(time / 1000.0) as f32 * 4.0));
        self.camera.look_at =
            Mat4::look_at_rh(self.camera.position, self.camera.direction, self.camera.up);

        Ok(())
    }

    pub unsafe fn render(&mut self, gl: &glow::Context) -> Result<(), String> {
        self.texture_loader.tick(&gl)?;
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);

        // for quad in &self.quads {
        //     quad.render(gl);
        // }

        // self.scene.render(gl);

        if let Some(cube) = &self.cube {
            cube.render(gl, &self.camera);
        }

        Ok(())
    }

    pub unsafe fn init_cube(&mut self, gl: &glow::Context) -> Result<(), String> {
        let program = shader_def!(
            "cube.vert",
            "cube.frag",
            vec!(
                (VertexAttrType::Position, "position"),
                (VertexAttrType::UVs, "uv"),
                (VertexAttrType::Normal, "normal"),
            ),
            vec!(
                (UniformTypes::ModelMatrix, "model"),
                (UniformTypes::ViewMatrix, "view"),
                (UniformTypes::ProjMatrix, "projection"),
            )
        )
        .compile(gl)?;
        let program = Rc::new(program);

        let grass_tex = &self.loaded_textures[0];

        let cube_mesh = Rc::new(Mesh::make_cube());
        let cube_renderer = Rc::new(MeshRenderer::new(gl, &program, cube_mesh)?);
        let cube: GameObject = GameObject::new(&grass_tex, &cube_renderer);
        self.cube = Some(cube);

        Ok(())
    }

    pub unsafe fn init_quads(&mut self, gl: &glow::Context) -> Result<(), String> {
        let program = shader_def!(
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
        let quad_program = Rc::new(program);

        let grass_tex = &self.loaded_textures[0];
        let dirt_tex = &self.loaded_textures[1];

        let quad_mesh = Rc::new(Mesh::make_quad());
        let quad_renderer = Rc::new(MeshRenderer::new(gl, &quad_program, quad_mesh)?);
        for i in 0..50 {
            let tex = if i % 2 == 0 { &grass_tex } else { &dirt_tex };
            let quad: GameObject = GameObject::new(tex, &quad_renderer);
            self.quads.push(quad);
        }

        Ok(())
    }
}
