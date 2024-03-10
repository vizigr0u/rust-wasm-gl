use std::rc::Rc;

use glam::vec3;
use glam::Mat4;
use glam::Quat;
use glam::Vec3;
use glow::HasContext;
use glow::WebVertexArrayKey;
use web_sys::WebGlUniformLocation;

use crate::material::Material;

pub struct Quad {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub color: Vec3,
    pub buffer: [f32; 12],

    vao: Option<WebVertexArrayKey>,
    material: Rc<Material>,
    transform_location: Option<WebGlUniformLocation>,
    color_location: Option<WebGlUniformLocation>,
}

impl Quad {
    pub fn new(material: &Rc<Material>) -> Self {
        Quad {
            buffer: [
                -1.0, -1.0, 0.0, 1.0, -1.0, 0.0, -1.0, 1.0, 0.0, 1.0, 1.0, 0.0,
            ],
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE * 0.5,
            color: vec3(1.0, 1.0, 1.0),
            vao: None,
            material: material.clone(),
            transform_location: None,
            color_location: None,
        }
    }
    pub unsafe fn init(&mut self, gl: &glow::Context) -> Result<(), String> {
        let vao = gl.create_vertex_array()?;
        self.vao = Some(vao);
        gl.bind_vertex_array(self.vao);
        let buffer = gl.create_buffer()?;
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer));

        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            &self.buffer.align_to::<u8>().1,
            glow::STATIC_DRAW,
        );

        let shader = self.material.get_shader();
        let position_location = *shader
            .get_attr_location("position")
            .ok_or("can't get position")?;
        gl.vertex_attrib_pointer_f32(position_location, 3, glow::FLOAT, false, 0, 0);
        gl.enable_vertex_attrib_array(position_location);

        self.transform_location = gl.get_uniform_location(shader.get_program(), "transform");

        if self.transform_location.is_none() {
            return Err("Can't get transform uniform".to_string());
        }

        self.color_location = gl.get_uniform_location(shader.get_program(), "color");

        if self.color_location.is_none() {
            return Err("Can't get color uniform".to_string());
        }

        Ok(())
    }

    pub unsafe fn render(&self, gl: &glow::Context) {
        let gl = gl;
        gl.use_program(Some(self.material.get_shader().get_program()));
        gl.bind_vertex_array(self.vao);

        let mat = Mat4::from_scale(self.scale)
            * Mat4::from_quat(self.rotation)
            * Mat4::from_translation(self.position);

        gl.uniform_matrix_4_f32_slice(
            self.transform_location.as_ref(),
            false,
            &mat.to_cols_array().as_slice(),
        );

        let [r, g, b] = self.color.to_array();
        gl.uniform_3_f32(self.color_location.as_ref(), r, g, b);

        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
    }
}
