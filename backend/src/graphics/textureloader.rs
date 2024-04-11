// use glow::{HasContext, WebTextureKey};
use log::info;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;
use std::sync::Mutex;
use wasm_bindgen::JsCast;
use wasm_bindgen::{closure::Closure, JsValue};
use web_sys::HtmlImageElement;

use super::{Renderer, TextureType};
use crate::utils::{get_document, get_performance};

#[derive(Debug)]
enum TextureStatus {
    Idle,
    Busy(f64, Rc<RefCell<Closure<dyn FnMut()>>>),
}

#[derive(Debug)]
struct LoadingTexture<R>
where
    R: Renderer,
    R: Debug,
{
    img: HtmlImageElement,
    key: Option<R::TextureKey>,
    texture_type: TextureType,
    status: TextureStatus,
}

#[derive(Debug)]
pub struct TextureLoader<R>
where
    R: Renderer,
{
    pool: Rc<RefCell<Vec<LoadingTexture<R>>>>,
    renderer: Rc<RefCell<R>>,
}

const TEXTURE_CONTAINER_ID: &str = "textures_container";
const EMPTY_TEXTURE: [u8; 4] = [255, 0, 255, 255];

impl<R> TextureLoader<R>
where
    R: Renderer,
{
    pub fn new(pool_size: usize, renderer: Rc<RefCell<R>>) -> Result<Self, JsValue> {
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
                    key: None,
                    texture_type: TextureType::Texture2D,
                }
            })
            .collect();
        let pool = Rc::new(RefCell::new(pool));

        Ok(TextureLoader { pool, renderer })
    }

    // pub fn get_num_loading(&self) -> usize {
    //     self.pool
    //         .borrow()
    //         .iter()
    //         .filter(|tex| matches!(tex.status, TextureStatus::Busy(_, _)))
    //         .count()
    // }

    pub fn load(&mut self, src: &str, texture_type: TextureType) -> Result<R::TextureKey, String> {
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

                {
                    let mut renderer = self.renderer.borrow_mut();
                    let key = Some(renderer.create_texture());
                    renderer.bind_texture(texture_type, key);

                    loading_tex.key = key;
                    loading_tex.texture_type = texture_type;
                }

                Ok(loading_tex.key.expect("Shouldn't be none"))
            } else {
                Err("Texture is already loading".to_string())
            }
        } else {
            Err("Invalid texture index".to_string())
        }
    }

    pub fn tick(&mut self) -> Result<(), String> {
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
                        let key = tex.key;
                        let t = tex.texture_type;
                        let now: f64 = get_performance()?.now();
                        let total_time = now - *start_time;
                        info!(
                            "Texture loaded: {} ({}x{}) in {}ms",
                            img.src(),
                            img.natural_width(),
                            img.natural_height(),
                            total_time.round()
                        );

                        {
                            let mut renderer = self.renderer.borrow_mut();
                            renderer.load_texture_from_image(key, t, img)
                        }

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
