use glam::Mat4;
use glam::Vec4;
use web_sys::WebGl2RenderingContext;
use web_sys::WebGlVertexArrayObject;

use crate::shaders::CompiledShader;

trait Material {
    fn get_shader(&self) -> &CompiledShader;
    fn make_vertex_array(
        &self,
        context: &WebGl2RenderingContext,
    ) -> Result<WebGlVertexArrayObject, String>;

    fn set_color(color: &Vec4);
    fn set_transform(transform: &Mat4);
}
