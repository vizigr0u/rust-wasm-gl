use bitcode::{Decode, Encode};

use crate::core::{commands::SerializedCommand, CommandQueue, ResourceReference};

#[derive(Debug, Clone, Copy)]
pub struct Graphics;

pub type GraphicsCommand = SerializedCommand<Graphics>;

#[derive(Default, Clone, Copy)]
pub struct Texture;

// region: command types
// pub struct Shader;
#[derive(Encode, Decode, Clone)]
pub enum BasicCommand {
    SetClearColor([f32; 4]),
    CreateTexture(ResourceReference<Texture>),
}

#[derive(Encode, Decode, Clone)]
pub enum TextureCommand {
    CreateTexture,
}
// endregion

// pub enum CommandType {
//     Basic = 1,
// }

impl CommandQueue<Graphics> {
    pub fn set_clear_color(&mut self, color: &[f32; 4]) {
        let c: BasicCommand = BasicCommand::SetClearColor(color.clone());
        self.push(c);
    }
    pub fn create_texture(&mut self) -> ResourceReference<Texture> {
        let t = ResourceReference::<Texture>::new();
        let c: BasicCommand = BasicCommand::CreateTexture(t);
        self.push(c);
        t
    }
}

impl Into<SerializedCommand<Graphics>> for BasicCommand {
    fn into(self) -> SerializedCommand<Graphics> {
        SerializedCommand::<Graphics>::new(bitcode::encode(&self))
    }
}

impl SerializedCommand<Graphics> {
    pub fn parse(&self) -> Result<BasicCommand, String> {
        bitcode::decode::<BasicCommand>(&self.data).map_err(|e| e.to_string())
    }
}
