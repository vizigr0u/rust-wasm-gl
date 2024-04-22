use std::{fmt::Debug, marker::PhantomData, sync::mpsc::Sender};

use bitcode::Encode;
use log::info;

use super::commands::SerializedCommand;

pub const DEFAULT_QUEUE_SIZE: usize = 256;

#[derive(Debug, Clone)]
pub struct CommandQueue<T>
where
    T: Debug,
{
    commands: Vec<SerializedCommand<T>>,
    sender: Sender<SerializedCommand<T>>,
    _marker: PhantomData<T>,
}

impl<T> CommandQueue<T>
where
    T: Debug,
{
    pub fn new(sender: Sender<SerializedCommand<T>>) -> Self {
        Self {
            commands: Vec::with_capacity(DEFAULT_QUEUE_SIZE),
            sender,
            _marker: PhantomData,
        }
    }
    pub fn push<C>(&mut self, command: C)
    where
        C: Encode + Clone + Into<SerializedCommand<T>>,
    {
        self.commands
            .push(Into::<SerializedCommand<T>>::into(command.clone()));
    }
    // fn push_serialized(&mut self, command: SerializedCommand) {
    //     self.commands.push(command)
    // }
    pub fn commands(&self) -> &Vec<SerializedCommand<T>> {
        &self.commands
    }
    pub fn commands_mut(&mut self) -> &mut Vec<SerializedCommand<T>> {
        &mut self.commands
    }
    pub fn clear(&mut self) {
        self.commands.clear()
    }
    pub fn len(&self) -> usize {
        self.commands.len()
    }
    pub fn send_commands(&mut self) {
        for command in self.commands.drain(0..) {
            info!("sending command: {:?}", command);
            self.sender.send(command).unwrap();
        }
    }
}
