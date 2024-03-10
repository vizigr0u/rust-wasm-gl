use glow::{HasContext, WebTextureKey};
use log::info;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Mutex;
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
    key: Option<WebTextureKey>,
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
                let _ = body.append_child(&img);
                LoadingTexture {
                    img,
                    status: TextureStatus::Idle,
                    key: None,
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

    pub fn load(&mut self, gl: &glow::Context, src: &str) -> Result<WebTextureKey, String> {
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
                unsafe {
                    loading_tex.key = Some(gl.create_texture().expect("Can't create texture"));
                }

                Ok(loading_tex.key.expect("Shouldn't be none"))
            } else {
                Err("Texture is already loading".to_string())
            }
        } else {
            Err("Invalid texture index".to_string())
        }
    }

    pub fn tick(&mut self, gl: &glow::Context) -> Result<(), String> {
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

                        unsafe {
                            gl.bind_texture(glow::TEXTURE_2D, tex.key);

                            gl.tex_parameter_i32(
                                glow::TEXTURE_2D,
                                glow::TEXTURE_MIN_FILTER,
                                glow::NEAREST as _,
                            );
                            gl.tex_parameter_i32(
                                glow::TEXTURE_2D,
                                glow::TEXTURE_MAG_FILTER,
                                glow::NEAREST as _,
                            );

                            let level = 0;
                            let internal_format = glow::RGBA;
                            let src_format: u32 = glow::RGBA;
                            let src_type = glow::UNSIGNED_BYTE;

                            gl.tex_image_2d_with_html_image(
                                glow::TEXTURE_2D,
                                level,
                                internal_format as i32,
                                src_format,
                                src_type,
                                &img,
                            );
                            gl.generate_mipmap(glow::TEXTURE_2D);
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
