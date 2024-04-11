use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::WebGl2RenderingContext;

fn get_web_sys_webgl2_context(
    canvas: web_sys::HtmlCanvasElement,
) -> Result<WebGl2RenderingContext, JsValue> {
    let context = canvas
        .get_context("webgl2")?
        .expect("Unable to get WebGl2 context from canvas.")
        .dyn_into::<WebGl2RenderingContext>()?;

    Ok(context)
}

pub fn get_webgl2_context(canvas: web_sys::HtmlCanvasElement) -> Result<glow::Context, JsValue> {
    Ok(glow::Context::from_webgl2_context(
        get_web_sys_webgl2_context(canvas)?,
    ))
}
