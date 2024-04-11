use std::collections::HashMap;

use crate::graphics::{ShaderDef, ShaderFile};
use glam::Mat4;
use glow::{HasContext, WebProgramKey, WebShaderKey};
use web_sys::WebGlUniformLocation;

// use crate::world::ChunkPos;

use crate::{
    core::position::ChunkPos,
    graphics::{UniformTypes, VertexAttrType},
};

use super::Gl2Renderer;

pub struct Glsl;

#[macro_export]
macro_rules! glsl_def {
    ($vert_name: expr, $frag_name: expr, $attributes: expr, $uniforms: expr) => {
        ShaderDef::<Glsl>::new(
            vec![
                ShaderFile {
                    name: $vert_name,
                    source: include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/shaders/",
                        $vert_name
                    )),
                },
                ShaderFile {
                    name: $frag_name,
                    source: include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/shaders/",
                        $frag_name
                    )),
                },
            ],
            $attributes,
            $uniforms,
        )
    };
}

impl ShaderDef<Glsl> {
    fn vertex_shader(&self) -> &ShaderFile {
        debug_assert!(self.files.len() >= 1);
        &self.files[0]
    }

    fn fragment_shader(&self) -> &ShaderFile {
        debug_assert!(self.files.len() >= 2);
        &self.files[1]
    }

    pub fn compile(&self, renderer: &Gl2Renderer) -> Result<GlShaderProgram, String> {
        let gl: &glow::Context = renderer.gl();
        let vertex_shader = self.vertex_shader();
        let fragment_shader = self.fragment_shader();
        let vert = compile_shader(gl, glow::VERTEX_SHADER, vertex_shader.source)?;
        let frag = compile_shader(gl, glow::FRAGMENT_SHADER, fragment_shader.source)?;

        let program = link_program(gl, vert, frag)?;

        let mut attribute_locations = HashMap::new();
        for (attr_type, attr_name) in self.attributes.iter() {
            let location =
                unsafe { gl.get_attrib_location(program, *attr_name) }.ok_or(format!(
                    "Error getting attribute '{}' in ({}, {})",
                    attr_name, vertex_shader.name, fragment_shader.name
                ))?;
            attribute_locations.insert(*attr_type, location);
        }

        let mut uniform_locations = HashMap::new();
        for (u_type, u_name) in self.uniforms.iter() {
            let location = unsafe { gl.get_uniform_location(program, *u_name) }.ok_or(format!(
                "Error getting uniform '{}' in ({}, {})",
                u_name, vertex_shader.name, fragment_shader.name
            ))?;
            uniform_locations.insert(*u_type, location);
        }
        Ok(GlShaderProgram {
            attribute_locations,
            uniform_locations,
            program_key: program,
        })
    }
}

#[derive(Debug)]
pub struct GlShaderProgram {
    program_key: WebProgramKey,
    attribute_locations: HashMap<VertexAttrType, u32>,
    uniform_locations: HashMap<UniformTypes, WebGlUniformLocation>,
}

impl GlShaderProgram {
    pub fn get_attr_location(&self, attr: VertexAttrType) -> Option<&u32> {
        self.attribute_locations.get(&attr)
    }

    pub fn get_uniform_location(&self, uniform: UniformTypes) -> Option<&WebGlUniformLocation> {
        self.uniform_locations.get(&uniform)
    }

    // pub fn get_program(&self) -> WebProgramKey {
    //     self.program_key
    // }
    pub fn set_current_player_chunk(&self, gl: &glow::Context, chunk: ChunkPos) {
        let location = self.get_uniform_location(UniformTypes::PlayerChunkPos);
        let v = chunk.as_vec();
        unsafe { gl.uniform_3_i32(location, v.x, v.y, v.z) }
    }

    pub fn set_matrix(&self, gl: &glow::Context, matrix_type: UniformTypes, value: &Mat4) {
        let location = self.get_uniform_location(matrix_type);
        unsafe {
            gl.uniform_matrix_4_f32_slice(location, false, &value.to_cols_array().as_slice())
        };
    }

    pub fn gl_use(&self, gl: &glow::Context) {
        unsafe { gl.use_program(Some(self.program_key)) };
    }
}

fn compile_shader(
    gl: &glow::Context,
    shader_type: u32,
    source: &str,
) -> Result<WebShaderKey, String> {
    unsafe {
        let shader = gl.create_shader(shader_type)?;
        gl.shader_source(shader, source);
        gl.compile_shader(shader);

        match gl.get_shader_compile_status(shader) {
            true => Ok(shader),
            false => Err(gl.get_shader_info_log(shader)),
        }
    }
}

fn link_program(
    gl: &glow::Context,
    vert_shader: WebShaderKey,
    frag_shader: WebShaderKey,
) -> Result<WebProgramKey, String> {
    unsafe {
        let program = gl.create_program()?;

        gl.attach_shader(program, vert_shader);
        gl.attach_shader(program, frag_shader);
        gl.link_program(program);

        match gl.get_program_link_status(program) {
            true => Ok(program),
            false => Err(gl.get_program_info_log(program)),
        }
    }
}
