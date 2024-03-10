use std::rc::Rc;

// use web_sys::WebGl2RenderingContext;
// use web_sys::WebGlVertexArrayObject;

use glam::{Mat4, Vec3};

use crate::shaders::CompiledShader;

pub struct Material {
    pub transform: Mat4,
    pub color: Vec3,

    shader: Rc<CompiledShader>,
}

impl Material {
    pub fn from_shader(shader: &Rc<CompiledShader>) -> Self {
        Material {
            shader: shader.clone(),
            transform: Mat4::IDENTITY,
            color: Vec3::ONE,
        }
    }
    pub fn get_shader(&self) -> &CompiledShader {
        &self.shader
    }
}

// pub struct MaterialRenderer {
//     pub
// }
