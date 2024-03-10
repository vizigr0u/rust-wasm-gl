use glam::Mat4;
use glam::Vec3;

use glow::HasContext;
use glow::WebVertexArrayKey;

use web_sys::WebGlUniformLocation;

use crate::mesh::VertexAttrType;
use crate::shader_def;
use crate::shaders::CompiledShader;
use crate::shaders::ShaderDef;

pub struct TriangleScene {
    tris: Vec<Tri>,
    vao: Option<WebVertexArrayKey>,
    transform_location: Option<WebGlUniformLocation>,
    shader: Option<CompiledShader>,
}

impl TriangleScene {
    pub fn new() -> Self {
        let tris = Vec::<Tri>::new();
        TriangleScene {
            tris,
            vao: None,
            transform_location: None,
            shader: None,
        }
    }

    pub fn update(&mut self, _time: f64) -> Result<(), String> {
        Ok(())
    }

    pub unsafe fn init(&mut self, gl: &glow::Context) -> Result<(), String> {
        let vert_color_def: ShaderDef = shader_def!(
            "vertColor.vert",
            "vertColor.frag",
            vec!(
                (VertexAttrType::Position, "position"),
                (VertexAttrType::Color, "vertexColor")
            ),
            vec!()
        );

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

        let vao = gl.create_vertex_array()?;
        self.vao = Some(vao);
        gl.bind_vertex_array(self.vao);
        let buffer = gl.create_buffer()?;
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer));

        let mut vertices = Vec::with_capacity(TRI_BUFFER_SIZE * self.tris.len());

        for tri in &self.tris {
            vertices.extend_from_slice(&tri.buffer);
        }

        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            &vertices.align_to::<u8>().1,
            glow::STATIC_DRAW,
        );

        let shader = vert_color_def.compile(gl)?;

        let position_location = *shader
            .get_attr_location(VertexAttrType::Position)
            .ok_or("can't get position")?;
        gl.vertex_attrib_pointer_f32(position_location, 3, glow::FLOAT, false, 6 * 4, 0);
        gl.enable_vertex_attrib_array(position_location);

        let color_location = *shader
            .get_attr_location(VertexAttrType::Color)
            .ok_or("can't get color")?;
        gl.vertex_attrib_pointer_f32(color_location, 3, glow::FLOAT, false, 6 * 4, 3 * 4);
        gl.enable_vertex_attrib_array(color_location);

        self.shader = Some(shader);

        Ok(())
    }

    pub unsafe fn render(&self, gl: &glow::Context) {
        let shader = match &self.shader {
            Some(s) => s,
            None => return,
        };
        gl.use_program(Some(shader.get_program()));
        gl.bind_vertex_array(self.vao);

        let mat = Mat4::IDENTITY;

        gl.uniform_matrix_4_f32_slice(
            self.transform_location.as_ref(),
            false,
            &mat.to_cols_array().as_slice(),
        );

        let vert_count = self.tris.len() as i32 * 3;
        gl.draw_arrays(glow::TRIANGLES, 0, vert_count);
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
