use glow::WebTextureKey;

#[derive(Clone, Copy, Debug, Default)]
pub enum TextureType {
    #[default]
    Texture2D,
    Texture2DArray(u32),
}

pub type TextureDef = (TextureType, WebTextureKey);

impl Into<u32> for TextureType {
    fn into(self) -> u32 {
        match self {
            TextureType::Texture2D => glow::TEXTURE_2D,
            TextureType::Texture2DArray(_) => glow::TEXTURE_2D_ARRAY,
        }
    }
}
