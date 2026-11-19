use glow::WebTextureKey;

#[derive(Clone, Copy, Debug, Default)]
pub enum TextureType {
    #[default]
    Texture2D,
    Texture2DArray(u32),
}

pub type TextureDef = (TextureType, WebTextureKey);

impl From<TextureType> for u32 {
    fn from(val: TextureType) -> Self {
        match val {
            TextureType::Texture2D => glow::TEXTURE_2D,
            TextureType::Texture2DArray(_) => glow::TEXTURE_2D_ARRAY,
        }
    }
}
