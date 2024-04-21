use glow::{HasContext, WebBufferKey, WebProgramKey, WebTextureKey, WebVertexArrayKey};
use wasm_bindgen::JsValue;

use crate::graphics::{RenderFlag, Renderer, TextureType, TodoRenderer, VertexDataType};

use super::utils;

#[derive(Debug)]
pub struct Gl2Renderer {
    gl: glow::Context,
}

impl Gl2Renderer {
    pub fn gl(&self) -> &glow::Context {
        &self.gl
    }
}

impl Into<u32> for TextureType {
    fn into(self) -> u32 {
        match self {
            TextureType::Texture2D => glow::TEXTURE_2D,
            TextureType::Texture2DArray(_) => glow::TEXTURE_2D_ARRAY,
        }
    }
}

impl TodoRenderer for Gl2Renderer {
    type ShaderProgramKey = WebProgramKey;
    type TextureKey = WebTextureKey;
    type BufferKey = WebBufferKey;
    type VertexArrayKey = WebVertexArrayKey;

    fn create_texture(&mut self) -> Self::TextureKey {
        unsafe { self.gl.create_texture().expect("Can't create texture") }
    }

    fn load_texture(
        &mut self,
        key: Option<Self::TextureKey>,
        texture_type: TextureType,
        data: &[u8],
        width: i32,
        height: i32,
    ) {
        self.bind_texture(texture_type, key);
        unsafe {
            match texture_type {
                TextureType::Texture2D => self.gl.tex_image_2d(
                    texture_type.into(),
                    0,
                    glow::RGBA as _,
                    width,
                    height,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    Some(data),
                ),
                TextureType::Texture2DArray(_) => self.gl.tex_image_3d(
                    texture_type.into(),
                    0,
                    glow::RGBA as _,
                    width,
                    height,
                    1,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    Some(data),
                ),
            }
        }
    }

    fn bind_texture(&mut self, texture_type: TextureType, key: Option<WebTextureKey>) {
        unsafe { self.gl.bind_texture(texture_type.into(), key) }
    }

    fn load_texture_from_image(
        &mut self,
        key: Option<Self::TextureKey>,
        texture_type: TextureType,
        img: &web_sys::HtmlImageElement,
    ) {
        self.bind_texture(texture_type, key);
        let texture_u32 = texture_type.into();

        unsafe {
            self.gl
                .tex_parameter_i32(texture_u32, glow::TEXTURE_MIN_FILTER, glow::NEAREST as _);
            self.gl
                .tex_parameter_i32(texture_u32, glow::TEXTURE_MAG_FILTER, glow::NEAREST as _);

            let lod = 0;
            let internal_format = glow::RGBA;
            let src_format: u32 = glow::RGBA;
            let src_type = glow::UNSIGNED_BYTE;

            match texture_type {
                TextureType::Texture2D => {
                    self.gl.tex_image_2d_with_html_image(
                        texture_u32,
                        lod,
                        internal_format as _,
                        src_format,
                        src_type,
                        &img,
                    );
                }
                TextureType::Texture2DArray(depth) => self.gl.tex_image_3d_with_html_image_element(
                    texture_u32,
                    lod,
                    internal_format as _,
                    img.natural_width() as _,
                    (img.natural_height() / depth) as _,
                    depth as _,
                    0,
                    src_format,
                    glow::UNSIGNED_BYTE,
                    &img,
                ),
            }

            self.gl.generate_mipmap(texture_u32);
        }
    }
}

impl Renderer for Gl2Renderer {
    async fn from_canvas(canvas: web_sys::HtmlCanvasElement) -> Result<Self, JsValue> {
        let gl = utils::get_webgl2_context(canvas)?;
        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LESS);
            gl.enable(glow::CULL_FACE);
        }
        Ok(Gl2Renderer { gl })
    }

    fn set_flag(&mut self, flag: RenderFlag, enabled: bool) {
        let value = match flag {
            RenderFlag::DepthTest => glow::DEPTH_TEST,
            RenderFlag::CullFace => glow::CULL_FACE,
            RenderFlag::Blend => glow::BLEND,
        };
        if enabled {
            unsafe { self.gl.enable(value) };
        } else {
            unsafe { self.gl.disable(value) };
        }
    }

    fn clear(&mut self, r: f32, g: f32, b: f32, a: f32) {
        unsafe {
            self.gl.clear_color(r, g, b, a);
            self.gl
                .clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        }
    }

    fn begin_render(&mut self) {}

    fn end_render(&mut self) {}
}

fn get_glow_vertex_data_type(data_type: VertexDataType) -> u32 {
    match data_type {
        VertexDataType::U8 => glow::UNSIGNED_BYTE,
        VertexDataType::I8 => glow::BYTE,
        VertexDataType::U16 => glow::UNSIGNED_SHORT,
        VertexDataType::I16 => glow::SHORT,
        VertexDataType::U32 => glow::UNSIGNED_INT,
        VertexDataType::I32 => glow::INT,
        VertexDataType::F16 => glow::HALF_FLOAT,
        VertexDataType::F32 => glow::FLOAT,
    }
}
