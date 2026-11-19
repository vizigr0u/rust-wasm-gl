use glow::HasContext;

use wasm_bindgen::prelude::*;
use web_sys::{Document, Performance, Window};

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

pub fn get_window() -> Result<Window, String> {
    web_sys::window().ok_or("Can't get window".into())
}

pub fn get_document() -> Result<Document, String> {
    get_window()?.document().ok_or("Can't get document".into())
}

pub fn get_performance() -> Result<Performance, String> {
    get_window()?
        .performance()
        .ok_or("Can't get performance".into())
}

pub fn get_canvas() -> Result<web_sys::HtmlCanvasElement, JsValue> {
    let document = get_document()?;
    document
        .get_element_by_id("canvas")
        .ok_or("cannot find canvas")?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "canvas isnt a Canvas Element".into())
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum GlRenderFlags {
    DepthTest = glow::DEPTH_TEST as isize,
    CullFace = glow::CULL_FACE as isize,
    Blend = glow::BLEND as isize,
}

pub struct GlState {
    pub depth_test: bool,
    pub cull_face: bool,
    pub blend: bool,
}

impl GlState {
    // pub fn save(gl: &glow::Context) -> Self {
    //     unsafe {
    //         Self {
    //             depth_test: gl.is_enabled(GlRenderFlags::DepthTest as _),
    //             cull_face: gl.is_enabled(GlRenderFlags::CullFace as _),
    //             blend: gl.is_enabled(GlRenderFlags::Blend as _),
    //         }
    //     }
    // }
    // pub fn restore(&self, gl: &glow::Context) {
    //     GlState::set_flag(gl, GlRenderFlags::DepthTest, self.depth_test);
    //     GlState::set_flag(gl, GlRenderFlags::CullFace, self.cull_face);
    //     GlState::set_flag(gl, GlRenderFlags::Blend, self.blend);
    // }

    pub fn set_flag(gl: &glow::Context, param: GlRenderFlags, value: bool) {
        if value {
            unsafe { gl.enable(param as _) };
        } else {
            unsafe { gl.disable(param as _) };
        }
    }
}

pub fn performance_now() -> f64 {
    get_performance().unwrap().now()
}
