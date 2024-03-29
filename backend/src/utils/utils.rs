use glow::HasContext;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    Document, Performance, Request, RequestInit, RequestMode, Response, WebGl2RenderingContext,
    Window,
};

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
        .ok_or("Can't get document".into())
}

pub fn get_canvas() -> Result<web_sys::HtmlCanvasElement, JsValue> {
    let document = get_document()?;
    document
        .get_element_by_id("canvas")
        .ok_or("cannot find canvas")?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "canvas isnt a Canvas Element".into())
}

pub fn get_web_sys_context() -> Result<WebGl2RenderingContext, JsValue> {
    let context = get_canvas()?
        .get_context("webgl2")?
        .expect("Unable to get WebGl2 context from canvas.")
        .dyn_into::<WebGl2RenderingContext>()?;

    Ok(context)
}

pub fn get_webgl2_context() -> Result<glow::Context, JsValue> {
    Ok(glow::Context::from_webgl2_context(get_web_sys_context()?))
}

#[wasm_bindgen]
pub async fn run(repo: String) -> Result<JsValue, JsValue> {
    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    let url = format!("https://api.github.com/repos/{}/branches/master", repo);

    let request = Request::new_with_str_and_init(&url, &opts)?;

    request
        .headers()
        .set("Accept", "application/vnd.github.v3+json")?;

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

    // `resp_value` is a `Response` object.
    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into().unwrap();

    // Convert this other `Promise` into a rust `Future`.
    let json = JsFuture::from(resp.json()?).await?;

    // Send the JSON response back to JS.
    Ok(json)
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
