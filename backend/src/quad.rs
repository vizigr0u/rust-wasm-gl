use glam::vec4;
use glam::Mat4;
use glam::Quat;
use glam::Vec3;
use glam::Vec4;

struct Quad {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub color: Vec4,
    pub buffer: [f32; 12],
}

impl Quad {
    fn new() -> Self {
        Quad {
            buffer: [
                -1.0, -1.0, 0.0, 1.0, -1.0, 0.0, -1.0, 1.0, 0.0, 1.0, 1.0, 0.0,
            ],
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            color: vec4(1.0, 1.0, 1.0, 1.0),
        }
    }
    fn render(&self, material: &Material, context: &QuadRenderContext) {
        if let Some(gl) = &context.gl {
            gl.use_program(context.program.as_ref());
            gl.bind_vertex_array(context.vao.as_ref());

            let mat = Mat4::from_scale(self.scale)
                * Mat4::from_quat(self.rotation)
                * Mat4::from_translation(self.position);

            gl.uniform_matrix4fv_with_f32_array(
                context.transform_location.as_ref(),
                false,
                &mat.to_cols_array().as_slice(),
            );
            gl.draw_arrays(WebGl2RenderingContext::TRIANGLE_STRIP, 0, 4);
        }
    }
}
