use std::rc::Rc;

use glam::{vec3, Quat, UVec3, Vec3};
use glow::HasContext;
use log::{info, warn};
use wasm_bindgen::JsValue;

use crate::camera::Camera;
use crate::chunk::Chunk;
use crate::eguibackend::EguiBackend;
use crate::gameobject::GameObject;
use crate::inputsystem::{self, HandleInputs, InputEventType, InputSystem};
use crate::material::{TextureDef, TextureType};
use crate::mesh::{Mesh, ToMesh, VertexAttrType};
use crate::meshrenderer::MeshRenderer;
use crate::shaders::{CompiledShader, ShaderDef, UniformTypes};
use crate::textureloader::TextureLoader;
use crate::time::Time;
use crate::world::World;
use crate::{basicmeshes, shader_def};

const GRASS_TEXTURE_PATH: &str = "data/textures/blocks/grass_block_side.png";
const SAND_TEXTURE_PATH: &str = "data/textures/blocks/sand.png";
const DIRT_TEXTURE_PATH: &str = "data/textures/blocks/dirt.png";
const BLOCKS_ATLAS_PATH: &str = "data/textures/blocks/blocks_atlas.png";

pub struct Game {
    texture_loader: TextureLoader,
    objects: Vec<GameObject>,
    cube: Option<GameObject>,
    world: World,
    camera: Camera,

    input_system: InputSystem,
    loaded_textures: Vec<Rc<TextureDef>>,
    time: Time,
    show_menu: bool,

    egui: Option<EguiBackend>,
}

impl Game {
    pub fn new() -> Result<Self, JsValue> {
        let game = Game {
            texture_loader: TextureLoader::new(10)?,
            objects: Vec::new(),
            cube: None,
            world: World::random(UVec3::new(25, 1, 25)),
            loaded_textures: Vec::new(),
            camera: Camera::new(
                Vec3 {
                    x: -10.0,
                    y: 60.0,
                    z: -30.0,
                },
                Vec3 {
                    x: 0.5,
                    y: -0.3,
                    z: 0.8,
                },
                Vec3 {
                    x: 0.2,
                    y: 1.0,
                    z: 0.2,
                },
            ),
            input_system: inputsystem::InputSystem::new()?,
            time: Time::new(),
            egui: None,
            show_menu: false,
        };

        Ok(game)
    }

    pub unsafe fn load(&mut self, gl: &glow::Context) -> Result<(), String> {
        for (path, t) in [
            (GRASS_TEXTURE_PATH, TextureType::Texture2D),
            (SAND_TEXTURE_PATH, TextureType::Texture2D),
            (DIRT_TEXTURE_PATH, TextureType::Texture2D),
            (BLOCKS_ATLAS_PATH, TextureType::Texture2DArray(16)),
        ] {
            let key = self.texture_loader.load(gl, path, t)?;
            self.loaded_textures.push(Rc::new((t, key)));
        }

        gl.enable(glow::DEPTH_TEST);
        gl.depth_func(glow::LESS);
        gl.enable(glow::CULL_FACE);

        self.init_objects(gl)?;
        let program = make_cube_program(gl)?;
        // self.init_cube(gl, &program)?;
        self.world
            .set_graphics(program.clone(), self.loaded_textures[3].clone());
        self.egui = Some(EguiBackend::new(gl));

        Ok(())
    }

    pub fn update(&mut self, gl: &glow::Context, time: f64) -> Result<(), String> {
        self.time.update(time);
        self.handle_inputs();

        if !self.show_menu {
            for (index, quad) in self.objects.iter_mut().enumerate() {
                quad.set_position(vec3(
                    f64::sin(2.0 * index as f64 + time / 1000.0) as f32,
                    f64::cos(2.0 * index as f64 + time / 200.0) as f32,
                    0.0,
                ));
                quad.set_scale(vec3(0.2, f64::sin(time / 1000.0) as f32 * 0.2, 0.5));
                quad.set_scale(vec3(0.2, 0.5, 0.5));
                quad.update();
            }

            if let Some(cube) = &mut self.cube {
                let rotation_angle_radians = 3.0 * 0.001 * self.time.delta_time() as f32;
                let rotation_y = Quat::from_rotation_y(-rotation_angle_radians);
                let rotation_z = Quat::from_rotation_z(0.8 * rotation_angle_radians);
                cube.set_rotation(cube.get_rotation() * rotation_y * rotation_z);
                cube.update();
            }
        }

        self.world.update(gl, &self.time);

        self.camera.update(&self.time);

        Ok(())
    }

    pub fn handle_inputs(&mut self) {
        let inputs = self.input_system.get_inputs();

        for event in inputs.get_events() {
            match event {
                InputEventType::KeyDown(_event) => {
                    if inputs.is_key_down("Escape") {
                        self.show_menu = !self.show_menu;
                    }
                }
                _ => {}
            }
        }

        if !self.show_menu {
            self.camera.handle_inputs(&inputs);
        } else {
            if let Some(egui) = &mut self.egui {
                egui.handle_inputs(&inputs);
            }
        }

        self.input_system.clear_events();
    }

    pub unsafe fn render(&mut self, gl: &glow::Context) -> Result<(), String> {
        self.texture_loader.tick(&gl)?;
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT);

        // self.scene.render(gl);

        // for quad in &self.objects {
        //     quad.render(gl, &self.camera.borrow());
        // }

        if let Some(cube) = &self.cube {
            cube.render(gl, &self.camera);
        }

        self.world.render(gl, &self.camera);

        if self.show_menu {
            if let Some(egui) = &mut self.egui {
                egui.render(gl);
            }
        }

        Ok(())
    }

    unsafe fn init_cube(
        &mut self,
        gl: &glow::Context,
        program: &Rc<CompiledShader>,
    ) -> Result<(), String> {
        let grass_tex = &self.loaded_textures[0];

        let cube_mesh = Rc::new(basicmeshes::make_cube());
        let mut cube_renderer = MeshRenderer::new(program);
        cube_renderer.set_mesh(gl, cube_mesh)?;
        let cube_renderer = Rc::new(cube_renderer);
        let mut cube: GameObject = GameObject::new(&grass_tex, &cube_renderer);
        cube.set_position(Vec3::ONE * -0.5);
        self.cube = Some(cube);

        Ok(())
    }

    pub unsafe fn init_objects(&mut self, gl: &glow::Context) -> Result<(), String> {
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
        let program = Rc::new(program);

        let grass_tex = &self.loaded_textures[0];
        let dirt_tex = &self.loaded_textures[1];

        let mesh = Rc::new(basicmeshes::make_quad_elements());
        let mut renderer = MeshRenderer::new(&program);
        renderer.set_mesh(gl, mesh)?;
        let renderer = Rc::new(renderer);
        for i in 0..50 {
            let tex = if i % 2 == 0 { &grass_tex } else { &dirt_tex };
            let quad: GameObject = GameObject::new(tex, &renderer);
            self.objects.push(quad);
        }

        Ok(())
    }
}

fn make_cube_program(gl: &glow::Context) -> Result<Rc<CompiledShader>, String> {
    unsafe {
        let program = shader_def!(
            "cube.vert",
            "cube.frag",
            vec!(
                (VertexAttrType::Position, "position"),
                (VertexAttrType::UVs, "uv"),
                (VertexAttrType::Normal, "normal"),
                (VertexAttrType::Depth, "depth"),
            ),
            vec!(
                (UniformTypes::ModelMatrix, "model"),
                (UniformTypes::ViewMatrix, "view"),
                (UniformTypes::ProjMatrix, "projection"),
            )
        )
        .compile(gl)?;
        Ok(Rc::new(program))
    }
}
