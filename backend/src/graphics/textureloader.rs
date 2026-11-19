use log::info;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Mutex;
use wasm_bindgen::JsCast;
use wasm_bindgen::{closure::Closure, JsValue};
use web_sys::HtmlImageElement;

use crate::render::TextureId;
use crate::utils::{get_document, get_performance};

#[derive(Debug)]
enum TextureStatus {
    Idle,
    Busy {
        start_time: f64,
        // Held only to keep the JS callbacks alive while the image loads.
        _on_load: Rc<RefCell<Closure<dyn FnMut()>>>,
        _on_error: Rc<RefCell<Closure<dyn FnMut()>>>,
    },
}

#[derive(Debug)]
struct LoadingTexture {
    img: HtmlImageElement,
    id: TextureId,
    layers: u32,
    status: TextureStatus,
}

#[derive(Debug)]
pub struct TextureLoader {
    pool: Rc<RefCell<Vec<LoadingTexture>>>,
}

const TEXTURE_CONTAINER_ID: &str = "textures_container";

impl TextureLoader {
    pub fn new(pool_size: usize) -> Result<Self, JsValue> {
        let document = get_document()?;
        // let body = document.body().ok_or("document should have a body")?;
        let container = document
            .get_element_by_id(TEXTURE_CONTAINER_ID)
            .ok_or(format!(
                "document should have an element with id {}",
                TEXTURE_CONTAINER_ID
            ))?;
        let pool = (0..pool_size)
            .map(|_| {
                let img = document
                    .create_element("img")
                    .unwrap()
                    .dyn_into::<HtmlImageElement>()
                    .unwrap();
                let _ = container.append_child(&img);
                LoadingTexture {
                    img,
                    status: TextureStatus::Idle,
                    id: TextureId::Blocks,
                    layers: 1,
                }
            })
            .collect();
        let pool = Rc::new(RefCell::new(pool));

        Ok(TextureLoader { pool })
    }

    pub fn get_num_loading(&self) -> usize {
        self.pool
            .borrow()
            .iter()
            .filter(|tex| matches!(tex.status, TextureStatus::Busy { .. }))
            .count()
    }

    /// Start fetching `src`; `tick` hands the decoded image back with `id` and `layers`.
    pub fn load(&mut self, src: &str, id: TextureId, layers: u32) -> Result<(), String> {
        let mut pool = self.pool.borrow_mut();
        if let Some((index, loading_tex)) = pool
            .iter_mut()
            .enumerate()
            .find(|(_, tex)| matches!(tex.status, TextureStatus::Idle))
        {
            if matches!(loading_tex.status, TextureStatus::Idle) {
                let on_load = {
                    Closure::wrap(Box::new(move || {
                        MessageSystem::add_completed_load(index);
                    }) as Box<dyn FnMut()>)
                };

                let on_error = Closure::wrap(Box::new(move || {
                    MessageSystem::add_failed_load(index);
                }) as Box<dyn FnMut()>);
                let rc_error = Rc::new(RefCell::new(on_error));
                loading_tex
                    .img
                    .set_onerror(Some(rc_error.borrow().as_ref().unchecked_ref()));

                let rc_closure = Rc::new(RefCell::new(on_load));

                loading_tex
                    .img
                    .set_onload(Some(rc_closure.borrow().as_ref().unchecked_ref()));

                loading_tex.img.set_src(src);
                let start_time: f64 = get_performance()?.now();
                loading_tex.status = TextureStatus::Busy {
                    start_time,
                    _on_load: rc_closure,
                    _on_error: rc_error,
                };
                loading_tex.id = id;
                loading_tex.layers = layers;
                Ok(())
            } else {
                Err("Texture is already loading".to_string())
            }
        } else {
            Err("Invalid texture index".to_string())
        }
    }

    /// Collect finished loads; `on_loaded` gets each decoded image to upload.
    pub fn tick(
        &mut self,
        mut on_loaded: impl FnMut(TextureId, &HtmlImageElement, u32),
    ) -> Result<(), String> {
        let completed_loads = MessageSystem::take_completed_loads();
        let mut errors = "".to_string();

        for index in MessageSystem::take_failed_loads() {
            if let Some(tex) = self.pool.borrow_mut().get_mut(index) {
                if matches!(tex.status, TextureStatus::Busy { .. }) {
                    errors = format!("{}, failed to load texture '{}'", errors, tex.img.src());
                    // Free the slot; the texture stays the magenta placeholder.
                    tex.status = TextureStatus::Idle;
                }
            }
        }

        if !completed_loads.is_empty() {
            info!("{} new messages", completed_loads.len());
        }

        for index in completed_loads {
            if let Some(tex) = self.pool.borrow_mut().get_mut(index) {
                match &mut tex.status {
                    TextureStatus::Idle => {
                        errors = format!(
                            "{}, received message from idle LoadingTexture index {}",
                            errors, index
                        );
                    }
                    TextureStatus::Busy { start_time, .. } => {
                        let img = &tex.img;
                        let now: f64 = get_performance().map(|p| p.now()).unwrap_or(*start_time);
                        let total_time = now - *start_time;
                        info!(
                            "Texture loaded: {} ({}x{}) in {}ms",
                            img.src(),
                            img.natural_width(),
                            img.natural_height(),
                            total_time.round()
                        );

                        on_loaded(tex.id, img, tex.layers);

                        tex.status = TextureStatus::Idle;
                    }
                };
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

type MessageType = usize;
static PENDING_MESSAGES: Mutex<Vec<MessageType>> = Mutex::new(Vec::new());
static FAILED_MESSAGES: Mutex<Vec<MessageType>> = Mutex::new(Vec::new());

struct MessageSystem {}

impl MessageSystem {
    fn add_completed_load(msg: MessageType) {
        PENDING_MESSAGES.lock().unwrap().push(msg);
    }
    fn add_failed_load(msg: MessageType) {
        FAILED_MESSAGES.lock().unwrap().push(msg);
    }
    fn take_failed_loads() -> Vec<MessageType> {
        FAILED_MESSAGES.lock().unwrap().drain(..).collect()
    }
    fn take_completed_loads() -> Vec<MessageType> {
        PENDING_MESSAGES.lock().unwrap().drain(..).collect()
    }
}
