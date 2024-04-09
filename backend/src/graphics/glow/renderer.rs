use glow::{HasContext, WebTextureKey};

use crate::graphics::{Renderer, TextureType};

#[derive(Debug)]
pub struct GlowRenderer {
    gl: glow::Context,
}

impl Renderer for GlowRenderer {
    type TextureKey = WebTextureKey;

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
