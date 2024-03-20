use std::collections::HashMap;

use glam::Mat4;
use glow::{HasContext, WebProgramKey, WebShaderKey};
use web_sys::WebGlUniformLocation;

use crate::mesh::VertexAttrType;

#[macro_export]
macro_rules! shader_def {
    ($vert_name: expr, $frag_name: expr, $attributes: expr, $uniforms: expr) => {
        ShaderDef::new(
            $vert_name,
            $frag_name,
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/", $vert_name)),
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/", $frag_name)),
            $attributes,
            $uniforms,
        )
    };
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum UniformTypes {
    // Texture,
    ModelMatrix,
    ViewMatrix,
    ProjMatrix,
    WorldPosition,
}

pub struct ShaderDef {
    vertex_filename: &'static str,
    fragment_filename: &'static str,
    vertex: &'static str,
    fragment: &'static str,
    attributes: Vec<(VertexAttrType, &'static str)>,
    uniforms: Vec<(UniformTypes, &'static str)>,
}

#[derive(Debug)]
pub struct CompiledShader {
    program_key: WebProgramKey,
    attribute_locations: HashMap<VertexAttrType, u32>,
    uniform_locations: HashMap<UniformTypes, WebGlUniformLocation>,
}

impl CompiledShader {
    pub fn get_attr_location(&self, attr: VertexAttrType) -> Option<&u32> {
        self.attribute_locations.get(&attr)
    }

    pub fn get_uniform_location(&self, uniform: UniformTypes) -> Option<&WebGlUniformLocation> {
        self.uniform_locations.get(&uniform)
    }

    // pub fn get_program(&self) -> WebProgramKey {
    //     self.program_key
    // }

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

impl ShaderDef {
    pub fn new(
        vertex_filename: &'static str,
        fragment_filename: &'static str,
        vertex: &'static str,
        fragment: &'static str,
        attributes: Vec<(VertexAttrType, &'static str)>,
        uniforms: Vec<(UniformTypes, &'static str)>,
    ) -> Self {
        ShaderDef {
            vertex_filename,
            fragment_filename,
            vertex,
            fragment,
            attributes,
            uniforms,
        }
    }

    pub unsafe fn compile(&self, gl: &glow::Context) -> Result<CompiledShader, String> {
        let vert = compile_shader(gl, glow::VERTEX_SHADER, self.vertex)?;
        let frag = compile_shader(gl, glow::FRAGMENT_SHADER, self.fragment)?;

        let program = link_program(gl, vert, frag)?;

        let mut attribute_locations = HashMap::new();
        for (attr_type, attr_name) in self.attributes.iter() {
            let location = gl.get_attrib_location(program, *attr_name).ok_or(format!(
                "Error getting attribute '{}' in ({}, {})",
                attr_name, self.vertex_filename, self.fragment_filename
            ))?;
            attribute_locations.insert(*attr_type, location);
        }

        let mut uniform_locations = HashMap::new();
        for (u_type, u_name) in self.uniforms.iter() {
            let location = gl.get_uniform_location(program, *u_name).ok_or(format!(
                "Error getting uniform '{}' in ({}, {})",
                u_name, self.vertex_filename, self.fragment_filename
            ))?;
            uniform_locations.insert(*u_type, location);
        }
        Ok(CompiledShader {
            attribute_locations,
            uniform_locations,
            program_key: program,
        })
    }
}

unsafe fn compile_shader(
    gl: &glow::Context,
    shader_type: u32,
    source: &str,
) -> Result<WebShaderKey, String> {
    let shader = gl.create_shader(shader_type)?;
    gl.shader_source(shader, source);
    gl.compile_shader(shader);

    match gl.get_shader_compile_status(shader) {
        true => Ok(shader),
        false => Err(gl.get_shader_info_log(shader)),
    }
}

unsafe fn link_program(
    gl: &glow::Context,
    vert_shader: WebShaderKey,
    frag_shader: WebShaderKey,
) -> Result<WebProgramKey, String> {
    let program = gl.create_program()?;

    gl.attach_shader(program, vert_shader);
    gl.attach_shader(program, frag_shader);
    gl.link_program(program);

    match gl.get_program_link_status(program) {
        true => Ok(program),
        false => Err(gl.get_program_info_log(program)),
    }
}
