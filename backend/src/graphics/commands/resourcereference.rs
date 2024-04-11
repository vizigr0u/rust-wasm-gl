use std::{marker::PhantomData, sync::Mutex};

#[derive(Default, Clone, Copy)]
pub struct Texture;

#[derive(Default, Clone, Copy)]
pub struct ResourceReference<T> {
    pub reference: usize,
    phantom: PhantomData<T>,
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
