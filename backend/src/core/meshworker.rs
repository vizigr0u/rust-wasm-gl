//! Meshing in a Web Worker. `src/mesh-worker.ts` runs `mesh_job` from this same wasm module.
//! Jobs and results move as transferred `ArrayBuffer`s (no copies, no shared memory).
use std::{cell::RefCell, fmt, rc::Rc};

use js_sys::{Array, Object, Reflect, Uint32Array, Uint8Array};
use voxel_core::world::{mesh_input, MeshExecutor, MeshJob, MeshResult};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{MessageEvent, Worker};

use crate::utils::trace::{Span, Track};

/// Worker entry point (called from `mesh-worker.ts`): `encode_mesh_input` bytes in, quad words out.
#[wasm_bindgen]
pub fn mesh_job(input: &[u8]) -> Vec<u32> {
    mesh_input(input)
}

pub struct WorkerMesher {
    worker: Worker,
    results: Rc<RefCell<Vec<MeshResult>>>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_error: Closure<dyn FnMut(web_sys::ErrorEvent)>,
}

impl fmt::Debug for WorkerMesher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WorkerMesher")
    }
}

impl WorkerMesher {
    /// The page shell (`main.ts`) stores its mesh worker in `window.__meshWorker`.
    pub fn from_window() -> Option<Self> {
        let window = web_sys::window()?;
        let worker: Worker = Reflect::get(&window, &"__meshWorker".into())
            .ok()?
            .dyn_into()
            .ok()?;
        let results: Rc<RefCell<Vec<MeshResult>>> = Rc::default();
        let sink = results.clone();
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
            let _s = Span::new("mesh result", Track::World);
            let data = e.data();
            let (Ok(id), Ok(out)) = (
                Reflect::get(&data, &"id".into()),
                Reflect::get(&data, &"output".into()),
            ) else {
                return;
            };
            let Some(id) = id.as_f64() else { return };
            sink.borrow_mut().push(MeshResult {
                id: id as u64,
                output: Uint32Array::new(&out).to_vec(),
            });
        });
        worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        let on_error = Closure::<dyn FnMut(web_sys::ErrorEvent)>::new(|e: web_sys::ErrorEvent| {
            log::error!("mesh worker error: {}", e.message());
        });
        worker.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        Some(Self {
            worker,
            results,
            _on_message: on_message,
            _on_error: on_error,
        })
    }
}

impl MeshExecutor for WorkerMesher {
    fn submit(&mut self, job: MeshJob) {
        let input = Uint8Array::from(job.input.as_slice());
        let msg = Object::new();
        let _ = Reflect::set(&msg, &"id".into(), &(job.id as f64).into());
        let _ = Reflect::set(&msg, &"input".into(), &input);
        let transfer = Array::of1(&input.buffer());
        if let Err(e) = self.worker.post_message_with_transfer(&msg, &transfer) {
            log::error!("mesh worker post failed: {e:?}");
        }
    }

    fn poll(&mut self) -> Vec<MeshResult> {
        std::mem::take(&mut self.results.borrow_mut())
    }
}
