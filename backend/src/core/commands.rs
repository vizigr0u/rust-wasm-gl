use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct SerializedCommand<T> {
    // pub command_type: u32,
    pub data: Vec<u8>,
    phantom: PhantomData<T>,
}

impl<T> SerializedCommand<T> {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            phantom: PhantomData,
        }
    }
}
