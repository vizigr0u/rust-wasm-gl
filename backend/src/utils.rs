use std::rc::Rc;

use gloo::events::EventListener;
use glow::{HasContext, WebTextureKey};

use log::{debug, info};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    Document, HtmlImageElement, Performance, Request, RequestInit, RequestMode, Response,
    WebGl2RenderingContext, Window,
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

pub fn get_web_sys_context() -> Result<WebGl2RenderingContext, JsValue> {
    let document = get_document()?;
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;

    let context = canvas
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

const IMAGE_SIZE: usize = 16;

pub unsafe fn load_texture(gl: &glow::Context, img_src: &str) -> Result<WebTextureKey, String> {
    info!("Trying to load texture {img_src}");
    let texture = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_2D, Some(texture));
    info!("Binding texture {:?}", texture);
    let texture2 = gl.create_texture()?;
    info!("texture2 {:?}", texture2);
    let level = 0;
    let internal_format = glow::RGBA;
    let width = IMAGE_SIZE;
    let height = IMAGE_SIZE;
    let border = 0;
    let src_format: u32 = glow::RGBA;
    let src_type = glow::UNSIGNED_BYTE;

    // Now upload single pixel.
    let pixels: [u8; 4 * IMAGE_SIZE * IMAGE_SIZE] = [255; 4 * IMAGE_SIZE * IMAGE_SIZE];
    debug!("{}", pixels.len());

    gl.tex_image_2d(
        glow::TEXTURE_2D,
        level,
        internal_format as i32,
        width as _,
        height as _,
        border,
        src_format,
        src_type,
        Some(&pixels),
    );

    let img =
        HtmlImageElement::new_with_width_and_height(IMAGE_SIZE as _, IMAGE_SIZE as _).unwrap();
    let document = web_sys::window()
        .unwrap()
        .document()
        .expect("no document??");
    let body = document.body().expect("document should have a body");
    let _ = body.append_child(&img).expect("error appending");
    info!("Image size {}*{}", img.client_width(), img.client_height());

    let img2 = document.create_element("img").unwrap();
    info!("img2 size {}*{}", img2.client_width(), img2.client_height());
    let _ = body.append_child(&img2).expect("error appending");

    let imgrc = Rc::new(img);
    let texture_rc = Rc::new(texture);
    {
        let img = imgrc.clone();
        let texture = texture_rc.clone();

        let a = Closure::wrap(Box::new(move || {
            // web_sys::console::log_1(&"LOAD EVENT".into());
            // let gl2 = gl; // get_webgl2_context().unwrap();
            // info!("Binding texture {:?}", texture);
            // gl2.bind_texture(glow::TEXTURE_2D, Some(*texture));

            // gl2.tex_image_2d_with_html_image(
            //     glow::TEXTURE_2D,
            //     level,
            //     internal_format as i32,
            //     src_format,
            //     src_type,
            //     &img,
            // );
            // info!(
            //     "Image loaded into texture {:?}: {}*{}",
            //     texture,
            //     img.client_width(),
            //     img.client_height()
            // );

            // // different from webgl1 where we need the pic to be power of 2
            // gl2.generate_mipmap(glow::TEXTURE_2D);
        }) as Box<dyn FnMut()>);
        imgrc.set_onload(Some(a.as_ref().unchecked_ref()));

        // Normally we'd store the handle to later get dropped at an appropriate
        // time but for now we want it to be a global handler so we use the
        // forget method to drop it without invalidating the closure. Note that
        // this is leaking memory in Rust, so this should be done judiciously!
        a.forget();
    }

    imgrc.set_src(img_src);

    Ok(texture)
}

pub unsafe fn load_texture2(
    gl: &glow::Context,
    img_src: &str,
) -> Result<Rc<WebTextureKey>, String> {
    web_sys::console::log_1(&(format!("Trying to load texture {}", img_src).as_str()).into());
    let texture = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_2D, Some(texture));
    let level = 0;
    let internal_format = glow::RGBA;
    let width = 1;
    let height = 1;
    let border = 0;
    let src_format: u32 = glow::RGBA;
    let src_type = glow::UNSIGNED_BYTE;

    // Now upload single pixel.
    let pixel: [u8; 4] = [0, 0, 255, 255];

    gl.tex_image_2d(
        glow::TEXTURE_2D,
        level,
        internal_format as i32,
        width,
        height,
        border,
        src_format,
        src_type,
        Some(&pixel),
    );

    let img = HtmlImageElement::new().unwrap();
    img.set_cross_origin(Some(""));

    let imgrc = Rc::new(img);

    let texture = Rc::new(texture);
    {
        let img = imgrc.clone();
        let texture = texture.clone();

        let event = EventListener::new(&imgrc, "load", move |_| {
            web_sys::console::log_1(&"LOAD EVENT".into());
            let texture = *texture;
            let gl2 = get_webgl2_context().unwrap();
            gl2.bind_texture(glow::TEXTURE_2D, Some(texture));

            gl2.tex_image_2d_with_html_image(
                glow::TEXTURE_2D,
                level,
                internal_format as i32,
                src_format,
                src_type,
                &img,
            );
            web_sys::console::log_1(&"IMAGE LOADED".into());

            // different from webgl1 where we need the pic to be power of 2
            gl2.generate_mipmap(glow::TEXTURE_2D);
        });
        web_sys::console::log_1(&event.event_type().into());
    }

    imgrc.set_src(img_src);

    Ok(texture)
}
