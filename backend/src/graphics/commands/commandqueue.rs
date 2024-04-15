use super::SerializedCommand;

pub const DEFAULT_QUEUE_SIZE: usize = 256;

#[derive(Debug, Clone)]
pub struct CommandQueue(Vec<SerializedCommand>);

impl Default for CommandQueue {
    fn default() -> Self {
        Self(Vec::with_capacity(DEFAULT_QUEUE_SIZE))
    }
}

impl CommandQueue {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push(&mut self, command: SerializedCommand) {
        self.0.push(command)
    }
    pub fn commands(&self) -> &Vec<SerializedCommand> {
        &self.0
    }
    pub fn commands_mut(&mut self) -> &mut Vec<SerializedCommand> {
        &mut self.0
    }
    pub fn clear(&mut self) {
        self.0.clear()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
}
