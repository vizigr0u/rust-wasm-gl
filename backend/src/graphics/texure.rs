use super::Renderer;

#[derive(Clone, Copy, Debug, Default)]
pub enum TextureType {
    #[default]
    Texture2D,
    Texture2DArray(u32),
}

// pub type TextureDef<R>
// where
//     R: Renderer,
// = (TextureType, R::TextureKey);
