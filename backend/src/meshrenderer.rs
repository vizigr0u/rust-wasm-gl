use std::rc::Rc;

use glow::{HasContext, WebBufferKey, WebVertexArrayKey};
use log::{info, warn};
use web_sys::WebGlVertexArrayObject;

use crate::mesh::{Mesh, MeshDisplayType, PrimitiveType};
use crate::shaders::CompiledShader;

enum DisplayData {
    None,
    Primitives(Option<WebVertexArrayKey>, PrimitiveType, usize),
    Elements(Option<WebVertexArrayKey>, Option<WebBufferKey>, usize),
}

pub struct MeshRenderer {
    program: Rc<CompiledShader>,
    display_data: DisplayData,
    vertex_count: i32,
}

impl MeshRenderer {
    pub fn get_program(&self) -> &CompiledShader {
        &self.program
    }

    pub fn new(gl: &glow::Context, program: &Rc<CompiledShader>) -> Self {
        MeshRenderer {
            display_data: DisplayData::None,
            vertex_count: 0,
            program: program.clone(),
        }
    }

    pub fn set_mesh(&mut self, gl: &glow::Context, mesh: Rc<Mesh>) -> Result<(), String> {
        match self.display_data {
            DisplayData::None => {}
            DisplayData::Primitives(vao, t, size) => {}
            DisplayData::Elements(vao, vbo, size) => {}
        }
        match mesh.display_type {
            MeshDisplayType::None => {}
            MeshDisplayType::Primitive(t) => unsafe {
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

                self.display_data =
                    DisplayData::Primitives(vao, t, mesh.get_data().len() / layout_size);
            },
            MeshDisplayType::Elements => unsafe {
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

                self.display_data =
                    DisplayData::Elements(vao, vbo, mesh.get_data().len() / layout_size);
            },
        }

        Ok(())
    }

    pub fn render(&self, gl: &glow::Context) {
        match self.display_data {
            DisplayData::None => {}
            DisplayData::Primitives(vao, t, vertex_count) => unsafe {
                self.get_program().gl_use(gl);
                gl.bind_vertex_array(vao);
                gl.draw_arrays(t as _, 0, vertex_count as _);
            },
            DisplayData::Elements(vbo, count) => unsafe {
                self.get_program().gl_use(gl);
                gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, vbo);
                gl.draw_elements(glow::ELEMENT_ARRAY_BUFFER, 0, self.vertex_count);
            },
        }
    }
}
