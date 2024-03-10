use std::collections::HashMap;

use glow::{HasContext, WebProgramKey, WebShaderKey};

#[macro_export]
macro_rules! include_shader {
    ($name:expr) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/", $name))
    };
}

#[macro_export]
macro_rules! shader_def {
    ($vert_name: expr, $frag_name: expr, $attributes: expr) => {
        ShaderDef::new(
            $vert_name,
            $frag_name,
            include_shader!($vert_name),
            include_shader!($frag_name),
            $attributes,
        )
    };
}

pub struct ShaderDef {
    vertex_filename: &'static str,
    fragment_filename: &'static str,
    vertex: &'static str,
    fragment: &'static str,
    attributes: Vec<&'static str>,
}

pub struct CompiledShader {
    program: WebProgramKey,
    attribute_locations: HashMap<&'static str, u32>,
}

impl CompiledShader {
    pub fn get_attr_location(&self, attr: &'static str) -> Option<&u32> {
        self.attribute_locations.get(attr)
    }

    pub fn get_program(&self) -> WebProgramKey {
        self.program
    }
}

impl ShaderDef {
    pub fn new(
        vertex_filename: &'static str,
        fragment_filename: &'static str,
        vertex: &'static str,
        fragment: &'static str,
        attributes: Vec<&'static str>,
    ) -> Self {
        ShaderDef {
            vertex_filename,
            fragment_filename,
            vertex,
            fragment,
            attributes,
        }
    }

    pub unsafe fn compile(&self, gl: &glow::Context) -> Result<CompiledShader, String> {
        let vert = compile_shader(gl, glow::VERTEX_SHADER, self.vertex)?;
        let frag = compile_shader(gl, glow::FRAGMENT_SHADER, self.fragment)?;

        let program = link_program(gl, vert, frag)?;

        let mut attribute_locations = HashMap::new();
        for attr in &self.attributes {
            let location = gl.get_attrib_location(program, attr).ok_or(format!(
                "Error getting location attribute in ({}, {})",
                self.vertex_filename, self.fragment_filename
            ))?;
            attribute_locations.insert(*attr, location);
        }
        Ok(CompiledShader {
            attribute_locations,
            program,
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
