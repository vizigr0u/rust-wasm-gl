use web_sys::{WebGl2RenderingContext, WebGlProgram, WebGlShader};

#[macro_export]
macro_rules! include_shader {
    ($name:expr) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/", $name))
    };
}

#[macro_export]
macro_rules! shader_def {
    ($vert_name: expr, $frag_name:expr) => {
        ShaderDef {
            vertex: include_shader!($vert_name),
            fragment: include_shader!($frag_name),
        }
    };
}

pub struct ShaderDef {
    pub vertex: &'static str,
    pub fragment: &'static str,
}

impl ShaderDef {
    pub fn compile(&self, context: &WebGl2RenderingContext) -> Result<WebGlProgram, String> {
        let vert = compile_shader(context, WebGl2RenderingContext::VERTEX_SHADER, self.vertex)?;
        let frag = compile_shader(
            context,
            WebGl2RenderingContext::FRAGMENT_SHADER,
            self.fragment,
        )?;

        link_program(context, &vert, &frag)
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
