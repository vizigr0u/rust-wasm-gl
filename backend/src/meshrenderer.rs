use std::rc::Rc;

use glam::Mat4;
use glow::{HasContext, WebVertexArrayKey};
use log::warn;

use crate::material::{Material, TextureType};
use crate::mesh::{Mesh, MeshDisplayType};
use crate::shaders::UniformTypes;

pub struct MeshRenderer {
    material: Rc<Material>,
    vao: Option<WebVertexArrayKey>,
    display_type: MeshDisplayType,
    vertex_count: i32,
}

impl MeshRenderer {
    pub unsafe fn load(
        gl: &glow::Context,
        material: Rc<Material>,
        mesh: Rc<Mesh>,
    ) -> Result<MeshRenderer, String> {
        let vao = Some(gl.create_vertex_array()?);
        gl.bind_vertex_array(vao);
        let buffer = gl.create_buffer()?;
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer));

        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            mesh.get_data().align_to::<u8>().1,
            glow::STATIC_DRAW,
        );

        let shader = material.get_shader();

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
            match shader.get_attr_location(data_type) {
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

        gl.use_program(Some(shader.get_program()));

        shader.set_matrix(gl, UniformTypes::ModelMatrix, &Mat4::IDENTITY);
        shader.set_matrix(gl, UniformTypes::ViewMatrix, &Mat4::IDENTITY);
        shader.set_matrix(gl, UniformTypes::ProjMatrix, &Mat4::IDENTITY);

        if let Some((tex_type, key)) = &material.texture {
            let texture = Some(*key);
            match tex_type {
                TextureType::Texture2D => gl.bind_texture(glow::TEXTURE_2D, texture),
                TextureType::Texture2DArray => todo!(),
            };
        }

        Ok(MeshRenderer {
            material: material.clone(),
            vao,
            display_type: mesh.display_type,
            vertex_count: mesh.get_data().len() as i32 / layout_size as i32,
        })
    }

    pub unsafe fn draw(&self, gl: &glow::Context, transform: &Mat4) {
        let material = &self.material;
        gl.use_program(Some(material.get_shader().get_program()));

        let shader = material.get_shader();

        shader.set_matrix(gl, UniformTypes::ModelMatrix, transform);

        gl.bind_vertex_array(self.vao);

        gl.draw_arrays(self.display_type as _, 0, self.vertex_count);
    }
}
