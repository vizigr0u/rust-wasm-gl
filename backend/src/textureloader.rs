use log::info;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use wasm_bindgen::JsCast;
use wasm_bindgen::{closure::Closure, JsValue};
use web_sys::HtmlImageElement;

use crate::utils::{get_document, get_performance};

enum TextureStatus {
    Idle,
    Busy(f64, Rc<RefCell<Closure<dyn FnMut()>>>),
}

struct LoadingTexture {
    img: HtmlImageElement,
    status: TextureStatus,
}

pub struct TextureLoader {
    pool: Rc<RefCell<Vec<LoadingTexture>>>,
}

impl TextureLoader {
    pub fn new(pool_size: usize) -> Result<Self, JsValue> {
        let document = get_document()?;
        let body = document.body().ok_or("document should have a body")?;
        let pool = (0..pool_size)
            .map(|_| {
                let img = document
                    .create_element("img")
                    .unwrap()
                    .dyn_into::<HtmlImageElement>()
                    .unwrap();
                img.set_src("data/textures/blocks/dirt.png");
                let _ = body.append_child(&img);
                LoadingTexture {
                    img,
                    status: TextureStatus::Idle,
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
            .filter(|tex| matches!(tex.status, TextureStatus::Busy(_, _)))
            .count()
    }

    pub fn load(&mut self, src: &str) -> Result<(), String> {
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

                let rc_closure = Rc::new(RefCell::new(on_load));

                loading_tex
                    .img
                    .set_onload(Some(rc_closure.borrow().as_ref().unchecked_ref()));

                loading_tex.img.set_src(src);
                let start_time: f64 = get_performance()?.now(); // TODO
                loading_tex.status = TextureStatus::Busy(start_time, rc_closure);

                Ok(())
            } else {
                Err("Texture is already loading".to_string())
            }
        } else {
            Err("Invalid texture index".to_string())
        }
    }

    pub fn tick(&mut self /* gl context if needed */) -> Result<(), String> {
        let completed_loads = MessageSystem::take_completed_loads();

        if completed_loads.len() > 0 {
            info!("{} new messages", completed_loads.len());
        }

        let mut errors = "".to_string();

        for index in completed_loads {
            if let Some(tex) = self.pool.borrow_mut().get_mut(index) {
                match &mut tex.status {
                    TextureStatus::Idle => {
                        errors = format!(
                            "{}, received message from idle LoadingTexture index {}",
                            errors, index
                        );
                    }
                    TextureStatus::Busy(start_time, _) => {
                        let img = &tex.img;
                        let now: f64 = get_performance()?.now();
                        let total_time = now - *start_time;
                        info!(
                            "Texture loaded: {} ({}x{}) in {}ms",
                            img.src(),
                            img.client_width(),
                            img.client_height(),
                            total_time.round()
                        );
                        tex.status = TextureStatus::Idle;
                    }
                };
            }
        }

        // Handle any additional logic, e.g., timeouts
        Ok(())
    }
}

type MessageType = usize;
static PENDING_MESSAGES: Mutex<Vec<MessageType>> = Mutex::new(Vec::new());

struct MessageSystem {}

impl MessageSystem {
    fn add_completed_load(msg: MessageType) {
        PENDING_MESSAGES.lock().unwrap().push(msg);
    }
    fn take_completed_loads() -> Vec<MessageType> {
        PENDING_MESSAGES.lock().unwrap().drain(..).collect()
    }
}
