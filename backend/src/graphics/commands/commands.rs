use super::{
    resourcereference::{create_reference, ResourceReference, Texture},
    CommandQueue,
};
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
pub struct CommandBuilder<S> {
    data: S,
}

impl CommandBuilder<Empty> {
    pub fn set_clear_color(color: &[f32; 4]) -> CommandBuilder<Command<BasicCommand>> {
        Self::command(BasicCommand::SetClearColor(color.clone()))
    }
    pub fn create_texture() -> CommandBuilder<ResourceCommand<Texture, TextureCommand>> {
        let a = Self::create::<Texture>();
        a.command(TextureCommand::CreateTexture)
    }
    pub fn with_texture(texture: ResourceReference<Texture>) -> CommandBuilder<Resource<Texture>> {
        Self::with_resource(texture)
    }

    fn with_resource<R>(r: ResourceReference<R>) -> CommandBuilder<Resource<R>>
    where
        R: Copy,
    {
        CommandBuilder::<Resource<R>> { data: Resource(r) }
    }
    fn create<R>() -> CommandBuilder<Resource<R>>
    where
        R: Default + Copy,
    {
        CommandBuilder::<Resource<R>> {
            data: Resource(create_reference::<R>()),
        }
    }
    fn command<C>(command: C) -> CommandBuilder<Command<C>>
    where
        C: Encode,
    {
        CommandBuilder::<Command<C>> {
            data: Command(command),
        }
    }
}

impl<R> CommandBuilder<Resource<R>>
where
    R: Copy,
{
    pub fn get_resource(&self) -> &ResourceReference<R> {
        &self.data.0
    }
    fn command<C>(self, command: C) -> CommandBuilder<ResourceCommand<R, C>>
    where
        C: Encode,
    {
        CommandBuilder::<ResourceCommand<R, C>> {
            data: ResourceCommand(*self.get_resource(), command),
        }
    }
}

impl<C> CommandBuilder<Command<C>>
where
    C: Encode + Clone,
    CommandBuilder<Command<C>>: Into<SerializedCommand>,
{
    pub fn enqueue(&self, queue: &mut CommandQueue) {
        queue.push(<CommandBuilder<Command<C>> as Into<SerializedCommand>>::into(self.clone()));
    }
}

pub enum CommandType {
    Basic = 1,
    Texture = 2,
}

#[derive(Debug, Clone)]
pub struct SerializedCommand {
    pub command_type: u32,
    pub data: Vec<u8>,
}

impl Into<SerializedCommand> for CommandBuilder<Command<BasicCommand>> {
    fn into(self) -> SerializedCommand {
        SerializedCommand {
            command_type: CommandType::Basic as u32,
            data: bitcode::encode(&self.data.0),
        }
    }
}

// impl SerializedCommand {
//     pub fn run

// }
