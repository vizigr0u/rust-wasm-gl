use std::{cell::RefCell, rc::Rc};

use cfg_if::cfg_if;
use wasm_bindgen::prelude::*;

use crate::core::Game;

mod core;
mod graphics;
mod gui;
pub mod math;
mod objects;
mod utils;
mod world;

cfg_if! {
    if #[cfg(feature = "console_log")] {
        fn init_log() {
            use log::Level;
            console_log::init_with_level(Level::Trace).expect("error initializing log");
        }
    } else {
        fn init_log() {}
    }
}

fn request_animation_frame(f: &Closure<dyn FnMut(f64) -> Result<(), JsValue>>) {
    web_sys::window()
        .unwrap()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .unwrap();
}

#[wasm_bindgen(start)]
fn start() -> Result<(), JsValue> {
    use utils::get_webgl2_context;

    utils::set_panic_hook();
    tracing_wasm::set_as_global_default();

    init_log();

    let gl = get_webgl2_context()?;

    let mut game = Game::new()?;

    unsafe {
        game.load(&gl)?;
    }

    main_loop(game, gl)?;

    Ok(())
}

fn main_loop(game: Game, gl: glow::Context) -> Result<(), JsValue> {
    let context = Rc::new(RefCell::new(gl));
    let update: Rc<RefCell<Option<Closure<dyn FnMut(f64) -> Result<(), JsValue>>>>> =
        Rc::new(RefCell::new(None));
    /* Reference to closure requests for first frame and then it's dropped */
    let request_update = update.clone();
    let game = Rc::new(RefCell::new(game));

    *request_update.borrow_mut() = Some(Closure::new(move |time| {
        let gl = &context.borrow();
        game.borrow_mut().tick(gl, time)?;

        // Request the next animation frame.
        request_animation_frame(update.borrow().as_ref().unwrap());
        Ok(())
    }));

    request_animation_frame(request_update.borrow().as_ref().unwrap());

    Ok(())
}
