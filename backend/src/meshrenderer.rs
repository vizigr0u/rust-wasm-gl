use std::rc::Rc;

use glow::{HasContext, WebVertexArrayKey};
use log::{info, warn};

use crate::mesh::{Mesh, MeshDisplayType};
use crate::shaders::CompiledShader;

pub struct MeshRenderer {
    program: Rc<CompiledShader>,
    vao: Option<WebVertexArrayKey>,
    display_type: MeshDisplayType,
    vertex_count: i32,
}

impl MeshRenderer {
    pub fn get_program(&self) -> &CompiledShader {
        &self.program
    }

    pub fn new(gl: &glow::Context, program: &Rc<CompiledShader>) -> Result<Self, String> {
        unsafe {
            let vao = Some(gl.create_vertex_array()?);
            gl.bind_vertex_array(vao);
            let buffer = gl.create_buffer()?;
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer));

            Ok(MeshRenderer {
                vao,
                display_type: MeshDisplayType::None,
                vertex_count: 0,
                program: program.clone(),
            })
        }
    }

    pub fn set_mesh(&mut self, gl: &glow::Context, mesh: Rc<Mesh>) {
        unsafe {
            gl.bind_vertex_array(self.vao);
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

            self.display_type = mesh.display_type;
            self.vertex_count = mesh.get_data().len() as i32 / layout_size as i32;
        }
    }

    pub fn render(&self, gl: &glow::Context) {
        match self.display_type {
            MeshDisplayType::None => {}
            _ => unsafe {
                self.get_program().gl_use(gl);
                gl.bind_vertex_array(self.vao);
                gl.draw_arrays(self.display_type as _, 0, self.vertex_count);
            },
        }
    }
}
