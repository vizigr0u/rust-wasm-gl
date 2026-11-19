use std::{cell::RefCell, rc::Rc};

use cfg_if::cfg_if;
use wasm_bindgen::prelude::*;

use crate::core::Game;
use crate::render::Renderer;

mod core;
mod graphics;
mod gui;
pub mod math;
mod objects;
mod render;
#[cfg(feature = "test-hooks")]
mod testhooks;
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
    // The mesh worker loads this module too: it only needs the `mesh_job` export.
    if web_sys::window().is_none() {
        utils::set_panic_hook();
        return Ok(());
    }
    utils::set_panic_hook();
    tracing_wasm::set_as_global_default();

    init_log();

    #[cfg(feature = "test-hooks")]
    if testhooks::is_test_mode() {
        // The spec drives the game through test_start/test_step instead.
        return Ok(());
    }

    let canvas = utils::get_canvas()?;
    let game = Game::new()?;

    // Renderer creation is async (wgpu adapter/device), so the loop starts from a task.
    wasm_bindgen_futures::spawn_local(async move {
        match Renderer::new(canvas).await {
            Ok(renderer) => {
                if let Err(err) = main_loop(game, renderer) {
                    web_sys::console::error_2(&"Could not start main loop:".into(), &err);
                }
            }
            Err(err) => {
                log::error!("Could not create renderer: {err}");
                show_overlay("WebGPU not available in this browser");
            }
        }
    });

    Ok(())
}

/// Full-page message instead of a blank canvas.
fn show_overlay(msg: &str) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let (Ok(div), Some(body)) = (doc.create_element("div"), doc.body()) else {
        return;
    };
    div.set_text_content(Some(msg));
    let _ = div.set_attribute(
        "style",
        "position:fixed;inset:0;display:flex;align-items:center;justify-content:center;\
         background:#111;color:#eee;font:20px sans-serif;z-index:100",
    );
    let _ = body.append_child(&div);
}

fn main_loop(game: Game, renderer: Renderer) -> Result<(), JsValue> {
    let renderer = Rc::new(RefCell::new(renderer));
    let update: Rc<RefCell<Option<Closure<dyn FnMut(f64) -> Result<(), JsValue>>>>> =
        Rc::new(RefCell::new(None));
    /* Reference to closure requests for first frame and then it's dropped */
    let request_update = update.clone();
    let game = Rc::new(RefCell::new(game));

    *request_update.borrow_mut() = Some(Closure::new(move |time| {
        if let Err(err) = game.borrow_mut().tick(&mut renderer.borrow_mut(), time) {
            // Log and keep the loop alive instead of dying silently.
            web_sys::console::error_2(&"Game tick failed:".into(), &JsValue::from(err));
        }

        // Always request the next animation frame.
        request_animation_frame(update.borrow().as_ref().unwrap());
        Ok(())
    }));

    request_animation_frame(request_update.borrow().as_ref().unwrap());

    Ok(())
}
