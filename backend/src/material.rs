use std::rc::Rc;

// use web_sys::WebGl2RenderingContext;
// use web_sys::WebGlVertexArrayObject;

use glam::{Mat4, Vec3};
use glow::WebTextureKey;

use crate::shaders::CompiledShader;

#[derive(Clone)]
pub struct Material {
    pub transform: Mat4,
    pub color: Vec3,
    pub texture: Option<WebTextureKey>,
    shader: Rc<CompiledShader>,
}

impl Material {
    pub fn from_shader(shader: &Rc<CompiledShader>) -> Self {
        Material {
            shader: shader.clone(),
            transform: Mat4::IDENTITY,
            color: Vec3::ONE,
            texture: None,
        }
    }
    pub fn get_shader(&self) -> &CompiledShader {
        &self.shader
    }
}

// pub struct MaterialRenderer {
//     pub
// }
