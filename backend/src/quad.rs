use std::rc::Rc;

use glam::vec4;
use glam::Mat4;
use glam::Quat;
use glam::Vec3;
use glam::Vec4;
use web_sys::WebGl2RenderingContext;
use web_sys::WebGlUniformLocation;
use web_sys::WebGlVertexArrayObject;

use crate::material::Material;

pub struct Quad {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub color: Vec4,
    pub buffer: [f32; 12],

    vao: Option<WebGlVertexArrayObject>,
    material: Rc<Material>,
    transform_location: Option<WebGlUniformLocation>,
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
            color: vec4(1.0, 1.0, 1.0, 1.0),
            vao: None,
            material: material.clone(),
            transform_location: None,
        }
    }
    pub fn init(&mut self, context: &WebGl2RenderingContext) -> Result<(), String> {
        let vao = context
            .create_vertex_array()
            .ok_or("Could not create vertex array object")?;
        self.vao = Some(vao);
        context.bind_vertex_array(self.vao.as_ref());
        let buffer = context.create_buffer().ok_or("Failed to create buffer")?;
        context.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));

        unsafe {
            // need to make sure we don't allow between view and buffer_data
            let positions = js_sys::Float32Array::view(&self.buffer);

            context.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &positions,
                WebGl2RenderingContext::STATIC_DRAW,
            );
        };

        let shader = self.material.get_shader();
        let position_location = *shader
            .get_attr_location("position")
            .ok_or("can't get position")?;
        context.vertex_attrib_pointer_with_i32(
            position_location,
            3,
            WebGl2RenderingContext::FLOAT,
            false,
            0,
            0,
        );
        context.enable_vertex_attrib_array(position_location);

        self.transform_location = context.get_uniform_location(shader.get_program(), "transform");

        if self.transform_location.is_none() {
            return Err("Can't get transform uniform".to_string());
        }

        Ok(())
    }

    pub fn render(&self, context: &WebGl2RenderingContext) {
        let gl = context;
        gl.use_program(Some(self.material.get_shader().get_program()));
        gl.bind_vertex_array(self.vao.as_ref());

        let mat = Mat4::from_scale(self.scale)
            * Mat4::from_quat(self.rotation)
            * Mat4::from_translation(self.position);

        gl.uniform_matrix4fv_with_f32_array(
            self.transform_location.as_ref(),
            false,
            &mat.to_cols_array().as_slice(),
        );
        gl.draw_arrays(WebGl2RenderingContext::TRIANGLE_STRIP, 0, 4);
    }
}
