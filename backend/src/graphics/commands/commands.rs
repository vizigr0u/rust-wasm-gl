use super::resourcereference::{create_reference, ResourceReference, Texture};
use bitcode::{Decode, Encode};

// region: command states
#[derive(Default, Clone)]
pub struct Empty;
#[derive(Default, Clone)]
pub struct Resource<R>(ResourceReference<R>)
where
    R: Copy;
#[derive(Default, Clone)]
pub struct ResourceCommand<R, C>(ResourceReference<R>, C)
where
    C: Encode;
#[derive(Default, Clone)]
pub struct Command<C>(C)
where
    C: Encode;
// endregion

// region: command types
// pub struct Shader;
#[derive(Encode, Decode)]
pub enum BasicCommand {
    SetClearColor([f32; 3]),
}

#[derive(Encode, Decode)]
pub enum TextureCommand {
    CreateTexture,
}
// endregion

#[derive(Default, Clone)]
pub struct GraphicsCommands<S> {
    data: S,
}

impl GraphicsCommands<Empty> {
    pub fn set_clear_color(self, color: [f32; 3]) -> GraphicsCommands<Command<BasicCommand>> {
        Self::command(BasicCommand::SetClearColor(color))
    }
    pub fn create_texture() -> GraphicsCommands<ResourceCommand<Texture, TextureCommand>> {
        let a = Self::create::<Texture>();
        a.command(TextureCommand::CreateTexture)
    }
    pub fn with_texture(
        texture: ResourceReference<Texture>,
    ) -> GraphicsCommands<Resource<Texture>> {
        Self::with_resource(texture)
    }

    fn with_resource<R>(r: ResourceReference<R>) -> GraphicsCommands<Resource<R>>
    where
        R: Copy,
    {
        GraphicsCommands::<Resource<R>> { data: Resource(r) }
    }
    fn create<R>() -> GraphicsCommands<Resource<R>>
    where
        R: Default + Copy,
    {
        GraphicsCommands::<Resource<R>> {
            data: Resource(create_reference::<R>()),
        }
    }
    fn command<C>(command: C) -> GraphicsCommands<Command<C>>
    where
        C: Encode,
    {
        GraphicsCommands::<Command<C>> {
            data: Command(command),
        }
    }
}

impl<R> GraphicsCommands<Resource<R>>
where
    R: Copy,
{
    pub fn get_resource(&self) -> &ResourceReference<R> {
        &self.data.0
    }
    fn command<C>(self, command: C) -> GraphicsCommands<ResourceCommand<R, C>>
    where
        C: Encode,
    {
        GraphicsCommands::<ResourceCommand<R, C>> {
            data: ResourceCommand(*self.get_resource(), command),
        }
    }
}

pub struct SerializedCommand {
    pub command_type: u32,
    pub data: Vec<u8>,
}

impl Into<SerializedCommand> for GraphicsCommands<Command<BasicCommand>> {
    fn into(self) -> SerializedCommand {
        SerializedCommand {
            command_type: 0,
            data: bitcode::encode(&self.data.0),
        }
    }
}
