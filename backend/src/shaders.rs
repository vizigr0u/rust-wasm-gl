use std::collections::HashMap;
use web_sys::{WebGl2RenderingContext, WebGlProgram, WebGlShader};

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
    program: WebGlProgram,
    attribute_locations: HashMap<&'static str, u32>,
}

impl CompiledShader {
    pub fn get_attr_location(&self, attr: &'static str) -> Option<&u32> {
        self.attribute_locations.get(attr)
    }

    pub fn get_program(&self) -> &WebGlProgram {
        &self.program
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

    // pub fn get_compiled(
    //     &mut self,
    //     context: &WebGl2RenderingContext,
    // ) -> Result<&CompiledShader, &String> {
    //     if self.compiled.is_none() {
    //         let res = self.compile(context);
    //         self.compiled = Some(res);
    //     }
    //     match &self.compiled {
    //         Some(Ok(compiled_shader)) => Ok(compiled_shader),
    //         Some(Err(e)) => Err(e),
    //         None => unreachable!("Just set above;"),
    //     }
    // }

    // pub fn is_compiled(&self) -> bool {
    //     self.compiled.is_some()
    // }

    pub fn compile(&self, context: &WebGl2RenderingContext) -> Result<CompiledShader, String> {
        let vert = compile_shader(context, WebGl2RenderingContext::VERTEX_SHADER, self.vertex)?;
        let frag = compile_shader(
            context,
            WebGl2RenderingContext::FRAGMENT_SHADER,
            self.fragment,
        )?;

        let program = link_program(context, &vert, &frag)?;

        let mut attribute_locations = HashMap::new();
        for attr in &self.attributes {
            let location = get_attrib_location(&program, context, attr).map_err(|s| {
                format!(
                    "{} ({}, {})",
                    s, self.vertex_filename, self.fragment_filename
                )
            })?;
            attribute_locations.insert(*attr, location);
        }
        Ok(CompiledShader {
            attribute_locations,
            program,
        })
    }
}

fn get_attrib_location(
    program: &WebGlProgram,
    context: &WebGl2RenderingContext,
    attr: &str,
) -> Result<u32, String> {
    match context.get_attrib_location(&program, attr) {
        -1 => Err(String::from(format!(
            "Unable to get attribute {attr} in shader"
        ))),
        n => Ok(n as u32),
    }
}

fn compile_shader(
    context: &WebGl2RenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = context
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    context.shader_source(&shader, source);
    context.compile_shader(&shader);

    if context
        .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(context
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

fn link_program(
    context: &WebGl2RenderingContext,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = context
        .create_program()
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    context.attach_shader(&program, vert_shader);
    context.attach_shader(&program, frag_shader);
    context.link_program(&program);

    if context
        .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(context
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creating program object")))
    }
}
