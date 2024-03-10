use std::rc::Rc;

// use glam::Mat4;
// use glam::Vec4;
// use web_sys::WebGl2RenderingContext;
// use web_sys::WebGlVertexArrayObject;

use crate::shaders::CompiledShader;

pub struct Material {
    shader: Rc<CompiledShader>,
}

impl Material {
    pub fn from_shader(shader: &Rc<CompiledShader>) -> Self {
        Material {
            shader: shader.clone(),
        }
    }
    pub fn get_shader(&self) -> &CompiledShader {
        &self.shader
    }
}
