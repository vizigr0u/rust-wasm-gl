use std::fmt::Debug;

use web_sys::HtmlImageElement;

use super::TextureType;

pub trait Renderer: Debug {
    type TextureKey: Debug + Copy;
    fn create_texture(&mut self) -> Self::TextureKey;
    fn load_texture(
        &mut self,
        key: Option<Self::TextureKey>,
        texture_type: TextureType,
        data: &[u8],
        width: i32,
        height: i32,
    );
    fn load_texture_from_image(
        &mut self,
        key: Option<Self::TextureKey>,
        texture_type: TextureType,
        img: &HtmlImageElement,
    );
    fn bind_texture(&mut self, texture_type: TextureType, key: Option<Self::TextureKey>);
}
