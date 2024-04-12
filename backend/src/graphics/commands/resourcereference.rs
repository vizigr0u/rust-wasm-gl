use std::{marker::PhantomData, sync::Mutex};

#[derive(Default, Clone, Copy)]
pub struct Texture;

#[derive(Default, Clone, Copy)]
pub struct ResourceReference<T> {
    reference: usize,
    phantom: PhantomData<T>,
}
impl<T> ResourceReference<T> {
    pub fn reference(&self) -> usize {
        self.reference
    }
}

static NEXT_REFERENCE: Mutex<usize> = Mutex::new(0);

pub(super) fn create_reference<T>() -> ResourceReference<T> {
    let reference = {
        let mut data = NEXT_REFERENCE.lock().unwrap();
        *data += 1;
        *data
    };

    ResourceReference {
        reference,
        phantom: PhantomData,
    }
}
