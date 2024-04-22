use std::{marker::PhantomData, sync::Mutex};

use bitcode::{Decode, Encode};

#[derive(Default, Clone, Copy, Encode, Decode)]
pub struct ResourceReference<T> {
    reference: usize,
    phantom: PhantomData<T>,
}
impl<T> ResourceReference<T> {
    pub fn new() -> Self {
        Self {
            reference: create_reference(),
            phantom: PhantomData,
        }
    }
    pub fn reference(&self) -> usize {
        self.reference
    }
}

static NEXT_REFERENCE: Mutex<usize> = Mutex::new(0);

fn create_reference() -> usize {
    let mut data = NEXT_REFERENCE.lock().unwrap();
    *data += 1;
    *data
}
