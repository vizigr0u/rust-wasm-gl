use std::cell::RefCell;
use std::rc::Rc;

use glam::{vec3, Quat, Vec3};
use glow::HasContext;
use log::{info, warn};
use wasm_bindgen::JsValue;

use crate::camera::Camera;
use crate::eguibackend::EguiBackend;
use crate::gameobject::GameObject;
use crate::inputsystem::{self, InputEventType, InputSubscription, InputSystem};
use crate::material::{TextureDef, TextureType};
use crate::mesh::{Mesh, VertexAttrType};
use crate::meshrenderer::MeshRenderer;
use crate::shaders::{ShaderDef, UniformTypes};
use crate::textureloader::TextureLoader;
use crate::time::Time;
use crate::trianglescene::TriangleScene;
use crate::{shader_def, utils};

const GRASS_TEXTURE_PATH: &str = "data/textures/blocks/grass_block_side.png";
const SAND_TEXTURE_PATH: &str = "data/textures/blocks/sand.png";
const DIRT_TEXTURE_PATH: &str = "data/textures/blocks/dirt.png";
const BLOCK_ATLAS_PATH: &str = "data/textures/blocks/atlas_blocks.png";

pub struct Game {
    scene: TriangleScene,
    texture_loader: TextureLoader,
    quads: Vec<GameObject>,
    cube: Option<GameObject>,
    camera: Rc<RefCell<Camera>>,

    input_system: InputSystem,
    loaded_textures: Vec<Rc<TextureDef>>,
    time: Time,

    camera_input_sub: Option<InputSubscription>,
    egui: Option<EguiBackend>,
}

impl Game {
    pub fn new() -> Result<Self, JsValue> {
        let input_system = inputsystem::InputSystem::new(&utils::get_canvas()?)?;

        let game = Game {
            scene: TriangleScene::new(),
            texture_loader: TextureLoader::new(10)?,
            quads: Vec::new(),
            cube: None,
            loaded_textures: Vec::new(),
            camera: Rc::new(RefCell::new(Camera::new(
                Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: -10.0,
                },
                Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                Vec3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
            ))),
            input_system,
            time: Time::new(),
            camera_input_sub: None,
            egui: None,
        };

        Ok(game)
    }

    pub unsafe fn load(&mut self, gl: &glow::Context) -> Result<(), String> {
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

        self.scene.init(gl)?;
        let camera: Rc<RefCell<Camera>> = self.camera.clone();
        let callback = Box::new(move |e: &InputEventType| {
            camera.borrow_mut().handle_input(e);
        });
        self.camera_input_sub = Some(self.input_system.subscribe(callback));
        // self.camera.borrow_mut().register_inputs(self.input_system);

        self.init_quads(gl)?;
        self.init_cube(gl)?;
        self.egui = Some(EguiBackend::new(gl));

        Ok(())
    }

    pub fn update(&mut self, time: f64) -> Result<(), String> {
        self.time.update(time);
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

        self.scene.update(&self.time)?;

        self.camera.borrow_mut().update(&self.time);

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

        // if let Some(cube) = &self.cube {
        //     cube.render(gl, &self.camera.borrow());
        // }

        if let Some(egui) = &mut self.egui {
            egui.render(gl);
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
        let mut cube_renderer = MeshRenderer::new(gl, &program)?;
        cube_renderer.set_mesh(gl, cube_mesh);
        let cube_renderer = Rc::new(cube_renderer);
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
        let mut quad_renderer = MeshRenderer::new(gl, &quad_program)?;
        quad_renderer.set_mesh(gl, quad_mesh);
        let quad_renderer = Rc::new(quad_renderer);
        for i in 0..50 {
            let tex = if i % 2 == 0 { &grass_tex } else { &dirt_tex };
            let quad: GameObject = GameObject::new(tex, &quad_renderer);
            self.quads.push(quad);
        }

        Ok(())
    }
}
