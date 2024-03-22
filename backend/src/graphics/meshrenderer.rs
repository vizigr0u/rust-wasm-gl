use std::rc::Rc;

use glow::{HasContext, WebBufferKey, WebVertexArrayKey};
use log::warn;

use crate::utils;
use crate::utils::GlRenderFlags;
use crate::utils::GlState;

use super::Mesh;
use super::ShaderProgram;

#[derive(Debug)]
enum DisplayData {
    None,
    Array(Option<WebVertexArrayKey>, usize),
    Elements(Option<WebVertexArrayKey>, Option<WebBufferKey>, usize),
}

const DEFAULT_FLAGS: [GlRenderFlags; 3] = [
    GlRenderFlags::CullFace,
    GlRenderFlags::Blend,
    GlRenderFlags::DepthTest,
];

#[derive(Debug)]
pub struct MeshRenderer {
    program: Rc<ShaderProgram>,
    primitive_type: u32,
    display_data: DisplayData,
    render_flags: &'static [GlRenderFlags], // vertex_count: i32,
}

impl MeshRenderer {
    pub fn get_program(&self) -> &ShaderProgram {
        &self.program
    }

    pub fn new(program: &Rc<ShaderProgram>) -> Self {
        MeshRenderer {
            display_data: DisplayData::None,
            primitive_type: glow::TRIANGLES,
            // vertex_count: 0,
            program: program.clone(),
            render_flags: &DEFAULT_FLAGS,
        }
    }

    pub fn with_render_flags(
        render_flags: &'static [GlRenderFlags],
        program: &Rc<ShaderProgram>,
    ) -> Self {
        let mut result = Self::new(program);
        result.render_flags = render_flags;
        result
    }

    pub fn set_mesh(&mut self, gl: &glow::Context, mesh: Rc<Mesh>) -> Result<(), String> {
        unsafe {
            let vao = Some(gl.create_vertex_array()?);
            gl.bind_vertex_array(vao);
            let buffer = gl.create_buffer()?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer));
            gl.bind_vertex_array(vao);
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                mesh.get_data().align_to::<u8>().1,
                glow::STATIC_DRAW,
            );

            let layout_size: usize = mesh.layout.iter().map(|(_type, size)| size).sum();

            if mesh.get_data().len() % layout_size != 0 {
                warn!(
                    "Mesh data of size {} doesn't match layout size of {}.",
                    mesh.get_data().len(),
                    layout_size
                );
            }

            let stride: i32 = 4 * layout_size as i32;

            let mut offset: i32 = 0;
            for &(data_type, size) in mesh.layout.iter() {
                match self.program.get_attr_location(data_type) {
                    Some(&location) => {
                        let location = location as u32;
                        gl.vertex_attrib_pointer_f32(
                            location,
                            size as _,
                            glow::FLOAT,
                            false,
                            stride,
                            offset,
                        );
                        gl.enable_vertex_attrib_array(location);
                    }
                    None => warn!("Unable to get location for type {:?}", data_type),
                }
                offset += 4 * size as i32;
            }

            self.display_data = match &mesh.indices {
                None => {
                    // info!(
                    //     "created Mesh of type {:?} with {} vertices.",
                    //     mesh.primitive_type,
                    //     mesh.get_data().len() / layout_size
                    // );
                    DisplayData::Array(vao, mesh.get_data().len() / layout_size)
                }
                Some(indices) => {
                    /* create vbo out of mesh indices */
                    let vbo = Some(gl.create_buffer()?);
                    gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, vbo);
                    gl.buffer_data_u8_slice(
                        glow::ELEMENT_ARRAY_BUFFER,
                        indices.align_to::<u8>().1,
                        glow::STATIC_DRAW,
                    );

                    // info!("created Mesh with {} indices.", indices.len());

                    DisplayData::Elements(vao, vbo, indices.len() as _)
                }
            };
            self.primitive_type = mesh.primitive_type;

            Ok(())
        }
    }

    pub fn render(&self, gl: &glow::Context) {
        self.set_gl_flags(gl);
        match self.display_data {
            DisplayData::None => {}
            DisplayData::Array(vao, vertex_count) => unsafe {
                self.get_program().gl_use(gl);
                gl.bind_vertex_array(vao);
                gl.draw_arrays(self.primitive_type as _, 0, vertex_count as _);
            },
            DisplayData::Elements(vao, vbo, count) => unsafe {
                self.get_program().gl_use(gl);
                gl.bind_vertex_array(vao);
                gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, vbo);
                gl.draw_elements(self.primitive_type as _, count as _, glow::UNSIGNED_INT, 0);
            },
        }
    }

    fn set_gl_flags(&self, gl: &glow::Context) {
        GlState::set_flag(
            gl,
            GlRenderFlags::CullFace,
            self.render_flags.contains(&utils::GlRenderFlags::CullFace),
        );
        GlState::set_flag(
            gl,
            GlRenderFlags::Blend,
            self.render_flags.contains(&utils::GlRenderFlags::Blend),
        );
        GlState::set_flag(
            gl,
            GlRenderFlags::DepthTest,
            self.render_flags.contains(&utils::GlRenderFlags::DepthTest),
        )
    }
}
