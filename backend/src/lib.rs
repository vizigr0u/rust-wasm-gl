use std::{cell::RefCell, rc::Rc};

use game::Game;
use wasm_bindgen::prelude::*;
use web_sys::WebGl2RenderingContext;

mod game;
mod material;
mod quad;
mod shaders;
mod utils;

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
    utils::set_panic_hook();
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;

    let context = canvas
        .get_context("webgl2")?
        .expect("Unable to get WebGl2 context from canvas.")
        .dyn_into::<WebGl2RenderingContext>()?;

    let gl = glow::Context::from_webgl2_context(context);

    let mut game = Game::new();
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
            game.borrow().render(&context.borrow()); // borrow the game for drawing.
        }

        // Request the next animation frame.
        request_animation_frame(update.borrow().as_ref().unwrap());
        Ok(())
    }));

    request_animation_frame(request_update.borrow().as_ref().unwrap());

    Ok(())
}
