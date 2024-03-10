use std::rc::Rc;

use glam::vec3;
use glam::Mat4;
use glam::Vec3;
use js_sys::Math::sin;
use rand::Rng;
use web_sys::WebGl2RenderingContext;
use web_sys::WebGlUniformLocation;
use web_sys::WebGlVertexArrayObject;

use crate::include_shader;
use crate::material::Material;
use crate::quad::Quad;
use crate::shader_def;
use crate::shaders::CompiledShader;
use crate::shaders::ShaderDef;

pub struct Game {
    scene: TriangleScene,
}

impl Game {
    pub fn new() -> Self {
        Game {
            scene: TriangleScene::new(),
        }
    }

    pub fn update(&mut self, time: f64) -> Result<(), String> {
        self.scene.update(time)?;
        Ok(())
    }

    pub fn init(&mut self, context: &WebGl2RenderingContext) -> Result<(), String> {
        self.scene.init(context)?;

        Ok(())
    }

    pub fn render(&self, context: &WebGl2RenderingContext) {
        context.clear_color(0.0, 0.0, 0.0, 1.0);
        context.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);

        self.scene.render(context);
    }
}

struct TriangleScene {
    tris: Vec<Tri>,
    quads: Vec<Quad>,
    vao: Option<WebGlVertexArrayObject>,
    transform_location: Option<WebGlUniformLocation>,
    shader: Option<CompiledShader>,
}

impl TriangleScene {
    pub fn new() -> Self {
        let tris = Vec::<Tri>::new();
        TriangleScene {
            tris,
            quads: Vec::new(),
            vao: None,
            transform_location: None,
            shader: None,
        }
    }

    pub fn update(&mut self, _time: f64) -> Result<(), String> {
        let mut rng = rand::thread_rng();
        for quad in self.quads.iter_mut() {
            quad.position = vec3(sin(_time / 1000.0) as f32, 0.0, 0.0);
            quad.color = vec3(rng.gen(), rng.gen(), rng.gen());
        }

        Ok(())
    }

    pub fn init(&mut self, context: &WebGl2RenderingContext) -> Result<(), String> {
        let vert_color_def: ShaderDef = shader_def!(
            "vertColor.vert",
            "vertColor.frag",
            vec!("position", "vertexColor")
        );
        let shader = vert_color_def.compile(context)?;

        let quad_shader =
            shader_def!("colorTrans.vert", "colorTrans.frag", vec!("position")).compile(context)?;
        let quad_shader_ref = Rc::new(quad_shader);
        let quad_mat = Rc::new(Material::from_shader(&quad_shader_ref));

        // self.transform_location = context.get_uniform_location(&program, "transform");

        self.tris.push(Tri::new(
            Vec3 {
                x: -0.7,
                y: -0.7,
                z: 0.0,
            },
            1.4,
        )?);
        self.tris.push(Tri::new(
            Vec3 {
                x: -0.7,
                y: 0.0,
                z: 0.0,
            },
            0.3,
        )?);
        self.tris.push(Tri::new(
            Vec3 {
                x: 0.7,
                y: 0.0,
                z: 0.0,
            },
            0.3,
        )?);

        self.quads.push(Quad::new(&quad_mat));

        for quad in self.quads.iter_mut() {
            quad.init(context)?;
        }

        let vao = context
            .create_vertex_array()
            .ok_or("Could not create vertex array object")?;
        self.vao = Some(vao);
        context.bind_vertex_array(self.vao.as_ref());
        let buffer = context.create_buffer().ok_or("Failed to create buffer")?;
        context.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));

        unsafe {
            let mut data = Vec::with_capacity(TRI_BUFFER_SIZE * self.tris.len());

            for tri in &self.tris {
                data.extend_from_slice(&tri.buffer);
            }

            // need to make sure we don't allow between view and buffer_data
            let positions = js_sys::Float32Array::view(data.as_slice());

            context.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &positions,
                WebGl2RenderingContext::STATIC_DRAW,
            );
        };

        let position_location = *shader
            .get_attr_location("position")
            .ok_or("can't get position")?;
        context.vertex_attrib_pointer_with_i32(
            position_location,
            3,
            WebGl2RenderingContext::FLOAT,
            false,
            6 * 4,
            0,
        );
        context.enable_vertex_attrib_array(position_location);

        let color_location = *shader
            .get_attr_location("vertexColor")
            .ok_or("can't get color")?;
        context.vertex_attrib_pointer_with_i32(
            color_location,
            3,
            WebGl2RenderingContext::FLOAT,
            false,
            6 * 4,
            3 * 4,
        );
        context.enable_vertex_attrib_array(color_location);

        self.shader = Some(shader);

        Ok(())
    }

    pub fn render(&self, context: &WebGl2RenderingContext) {
        let shader = match &self.shader {
            Some(s) => s,
            None => return,
        };
        context.use_program(Some(shader.get_program()));
        context.bind_vertex_array(self.vao.as_ref());

        let mat = Mat4::IDENTITY;

        context.uniform_matrix4fv_with_f32_array(
            self.transform_location.as_ref(),
            false,
            &mat.to_cols_array().as_slice(),
        );

        let vert_count = self.tris.len() as i32 * 3;
        context.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, vert_count);

        for quad in &self.quads {
            quad.render(context);
        }
    }
}

const TRI_BUFFER_SIZE: usize = 18;

pub struct Tri {
    pub pos: Vec3,
    pub size: f32,
    pub buffer: [f32; TRI_BUFFER_SIZE],
}

impl Tri {
    pub fn new(pos: Vec3, size: f32) -> Result<Self, String> {
        let buffer = [
            // Vertex 1
            pos.x,
            pos.y,
            0.0, // Position
            1.0,
            0.0,
            0.0, // Color (Red)
            // Vertex 2
            pos.x + size,
            pos.y,
            0.0, // Position
            0.0,
            1.0,
            0.0, // Color (Green)
            // Vertex 3
            pos.x + size * 0.5,
            pos.y + size,
            0.0, // Position
            0.0,
            0.0,
            1.0, // Color (Blue)
        ];
        Ok(Tri { pos, size, buffer })
    }
}
