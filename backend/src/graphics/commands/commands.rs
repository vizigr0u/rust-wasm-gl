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
#[derive(Encode, Decode, Clone)]
pub enum BasicCommand {
    SetClearColor([f32; 4]),
}

#[derive(Encode, Decode, Clone)]
pub enum TextureCommand {
    CreateTexture,
}
// endregion

#[derive(Default, Clone)]
pub struct GraphicsCommands<S> {
    data: S,
}

impl GraphicsCommands<Empty> {
    pub fn set_clear_color(color: &[f32; 4]) -> GraphicsCommands<Command<BasicCommand>> {
        Self::command(BasicCommand::SetClearColor(color.clone()))
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

impl<C> GraphicsCommands<Command<C>>
where
    C: Encode + Clone,
    GraphicsCommands<Command<C>>: Into<SerializedCommand>,
{
    pub fn enqueue(&self, queue: &mut Vec<SerializedCommand>) {
        queue.push(<GraphicsCommands<Command<C>> as Into<SerializedCommand>>::into(self.clone()));
    }
}

pub enum CommandType {
    Basic = 1,
    Texture = 2,
}

#[derive(Debug)]
pub struct SerializedCommand {
    pub command_type: u32,
    pub data: Vec<u8>,
}

impl Into<SerializedCommand> for GraphicsCommands<Command<BasicCommand>> {
    fn into(self) -> SerializedCommand {
        SerializedCommand {
            command_type: CommandType::Basic as u32,
            data: bitcode::encode(&self.data.0),
        }
    }
}
