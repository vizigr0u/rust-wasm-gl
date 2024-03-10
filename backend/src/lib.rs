use std::{cell::RefCell, rc::Rc};

use cfg_if::cfg_if;
use game::Game;
use wasm_bindgen::prelude::*;

mod game;
mod material;
mod quad;
mod shaders;
mod textureloader;
mod utils;

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

#[cfg(not(any(target_arch = "wasm32")))]
compile_error!("This project is for wasm only");

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
fn start() -> Result<(), JsValue> {
    use utils::get_webgl2_context;

    utils::set_panic_hook();
    init_log();

    let gl = get_webgl2_context()?;

    let mut game = Game::new()?;
    unsafe {
        game.init(&gl)?;
    }

    main_loop(game, gl)?;

    Ok(())
}

fn main_loop(game: Game, gl: glow::Context) -> Result<(), JsValue> {
    let game = Rc::new(RefCell::new(game));
    let context = Rc::new(RefCell::new(gl));
    let update: Rc<RefCell<Option<Closure<dyn FnMut(f64) -> Result<(), JsValue>>>>> =
        Rc::new(RefCell::new(None));
    /* Reference to closure requests for first frame and then it's dropped */
    let request_update = update.clone();

    *request_update.borrow_mut() = Some(Closure::new(move |time| {
        game.borrow_mut().update(time)?;
        unsafe {
            game.borrow_mut().render(&context.borrow())?; // borrow the game for drawing.
        }

        // Request the next animation frame.
        request_animation_frame(update.borrow().as_ref().unwrap());
        Ok(())
    }));

    request_animation_frame(request_update.borrow().as_ref().unwrap());

    Ok(())
}
