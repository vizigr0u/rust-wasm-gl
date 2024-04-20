mod gamestate;
mod render;

mod core;
mod graphics;
mod gui;
pub mod math;
mod objects;
mod utils;

// use log::info;
use std::{cell::RefCell, rc::Rc};
// use wasm_thread as thread;
use web_sys::DedicatedWorkerGlobalScope;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
fn start() {
    console_error_panic_hook::set_once();

    wasm_thread::spawn(|| {
        let update: Rc<RefCell<Option<Closure<dyn FnMut() -> Result<(), JsValue>>>>> =
            Rc::new(RefCell::new(None));

        let request_update = update.clone();

        *request_update.borrow_mut() = Some(Closure::new(move || {
            web_sys::console::log_1(&"tick".into());

            js_sys::global()
                .dyn_into::<DedicatedWorkerGlobalScope>()?
                .request_animation_frame(
                    update.borrow().as_ref().unwrap().as_ref().unchecked_ref(),
                )?;
            Ok(())
        }));

        web_sys::console::log_1(&"starting loop".into());
        js_sys::global()
            .dyn_into::<DedicatedWorkerGlobalScope>()
            .unwrap()
            .request_animation_frame(
                request_update
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .unchecked_ref(),
            )
            .unwrap();
    });
}

// fn infinite_worker_raf() {
//     let update: Rc<RefCell<Option<Closure<dyn FnMut(f64) -> Result<(), JsValue>>>>> =
//         Rc::new(RefCell::new(None));

//     let request_update = update.clone();

//     *request_update.borrow_mut() = Some(Closure::new(move |_time| {
//         web_sys::console::log_1(&"tick".into());

//         js_sys::global()
//             .dyn_into::<DedicatedWorkerGlobalScope>()?
//             .request_animation_frame(update.borrow().as_ref().unwrap().as_ref().unchecked_ref())?;
//         Ok(())
//     }));

//     web_sys::console::log_1(&"starting loop".into());
//     js_sys::global()
//         .dyn_into::<DedicatedWorkerGlobalScope>()
//         .unwrap()
//         .request_animation_frame(
//             request_update
//                 .borrow()
//                 .as_ref()
//                 .unwrap()
//                 .as_ref()
//                 .unchecked_ref(),
//         )
//         .unwrap();
// }

// #[derive(Clone)]
// enum GlobalProxy {
//     Window(web_sys::Window),
//     WorkerGlobalScope(DedicatedWorkerGlobalScope),
// }

// impl std::fmt::Display for GlobalProxy {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             GlobalProxy::Window(_) => write!(f, "Window"),
//             GlobalProxy::WorkerGlobalScope(_) => write!(f, "Worker"),
//         }
//     }
// }

// impl GlobalProxy {
//     fn request_animation_frame(
//         &self,
//         f: &Closure<dyn FnMut(f64) -> Result<i32, JsValue>>,
//     ) -> Result<i32, JsValue> {
//         info!("{self}.request_animation_frame");
//         match self {
//             GlobalProxy::Window(window) => {
//                 window.request_animation_frame(f.as_ref().unchecked_ref())
//             }
//             GlobalProxy::WorkerGlobalScope(scope) => {
//                 match scope.request_animation_frame(f.as_ref().unchecked_ref()) {
//                     Ok(id) => {
//                         info!("worker RAF id: {id}");
//                         Ok(id)
//                     }
//                     Err(err) => {
//                         warn!(
//                             "worker RAF err: {}",
//                             err.as_string().unwrap_or("unknow".into())
//                         );
//                         Err(err)
//                     }
//                 }
//             }
//         }
//     }
// }

// fn self_() -> Result<GlobalProxy, JsValue> {
//     let global = js_sys::global();
//     if js_sys::eval("typeof DedicatedWorkerGlobalScope !== 'undefined'")?
//         .as_bool()
//         .unwrap()
//     {
//         Ok(global
//             .dyn_into::<DedicatedWorkerGlobalScope>()
//             .map(GlobalProxy::WorkerGlobalScope)?)
//     } else {
//         Ok(global
//             .dyn_into::<web_sys::Window>()
//             .map(GlobalProxy::Window)?)
//     }
// }
