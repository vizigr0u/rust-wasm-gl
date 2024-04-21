use glow::HasContext;

use log::warn;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    DedicatedWorkerGlobalScope, Document, Performance, Request, RequestInit, RequestMode, Response,
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

pub fn get_document(window: &Window) -> Result<Document, String> {
    window.document().ok_or("Can't get document".into())
}

pub fn get_canvas_by_id(window: &Window, id: &str) -> Result<web_sys::HtmlCanvasElement, JsValue> {
    let document = get_document(window)?;
    document
        .get_element_by_id(id)
        .ok_or("cannot find canvas")?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "canvas isnt a Canvas Element".into())
}

#[derive(Debug, Clone)]
pub enum GlobalProxy {
    Window(web_sys::Window),
    WorkerGlobalScope(DedicatedWorkerGlobalScope),
}

impl std::fmt::Display for GlobalProxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GlobalProxy::Window(_) => write!(f, "Window"),
            GlobalProxy::WorkerGlobalScope(_) => write!(f, "Worker"),
        }
    }
}

impl GlobalProxy {
    pub fn new() -> Result<Self, JsValue> {
        let global = js_sys::global();
        if js_sys::eval("typeof DedicatedWorkerGlobalScope !== 'undefined'")?
            .as_bool()
            .unwrap()
        {
            Ok(global
                .dyn_into::<DedicatedWorkerGlobalScope>()
                .map(Self::WorkerGlobalScope)?)
        } else {
            Ok(global.dyn_into::<web_sys::Window>().map(Self::Window)?)
        }
    }

    pub fn get_window(&self) -> Result<Window, String> {
        match self {
            GlobalProxy::Window(_) => web_sys::window().ok_or("no global window".into()),
            GlobalProxy::WorkerGlobalScope(scope) => {
                Err(format!("current scope:{self} is not a window"))
            }
        }
    }

    pub fn get_performance(&self) -> Option<Performance> {
        match self {
            GlobalProxy::Window(window) => window.performance(),
            GlobalProxy::WorkerGlobalScope(scope) => scope.performance(),
        }
    }

    pub fn performance_now(&self) -> f64 {
        self.get_performance().unwrap().now()
    }

    pub fn request_animation_frame(
        &self,
        f: &Closure<dyn FnMut(f64) -> Result<i32, JsValue>>,
    ) -> Result<i32, JsValue> {
        // info!("{self}.request_animation_frame");
        match self {
            GlobalProxy::Window(window) => {
                window.request_animation_frame(f.as_ref().unchecked_ref())
            }
            GlobalProxy::WorkerGlobalScope(scope) => {
                match scope.request_animation_frame(f.as_ref().unchecked_ref()) {
                    Ok(id) => {
                        // info!("worker RAF id: {id}");
                        Ok(id)
                    }
                    Err(err) => {
                        warn!(
                            "worker RAF err: {}",
                            err.as_string().unwrap_or("unknow".into())
                        );
                        Err(err)
                    }
                }
            }
        }
    }

    pub fn set_interval(
        &self,
        f: Closure<dyn FnMut() -> ()>,
        timeout: i32,
    ) -> Result<i32, JsValue> {
        // info!("{self}.set_interval");
        let res: Result<i32, JsValue> = match self {
            GlobalProxy::Window(window) => window
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    f.into_js_value().unchecked_ref(),
                    timeout,
                ),
            GlobalProxy::WorkerGlobalScope(scope) => scope
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    f.into_js_value().unchecked_ref(),
                    timeout,
                ),
        };
        match res {
            Ok(id) => {
                // info!("{self} interval id: {id}");
                Ok(id)
            }
            Err(err) => {
                warn!(
                    "{self} interval err: {}",
                    err.as_string().unwrap_or("unknow".into())
                );
                Err(err)
            }
        }
    }
}

// #[wasm_bindgen]
// pub async fn run(repo: String) -> Result<JsValue, JsValue> {
//     let mut opts = RequestInit::new();
//     opts.method("GET");
//     opts.mode(RequestMode::Cors);

//     let url = format!("https://api.github.com/repos/{}/branches/master", repo);

//     let request = Request::new_with_str_and_init(&url, &opts)?;

//     request
//         .headers()
//         .set("Accept", "application/vnd.github.v3+json")?;

//     let window = web_sys::window().unwrap();
//     let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

//     // `resp_value` is a `Response` object.
//     assert!(resp_value.is_instance_of::<Response>());
//     let resp: Response = resp_value.dyn_into().unwrap();

//     // Convert this other `Promise` into a rust `Future`.
//     let json = JsFuture::from(resp.json()?).await?;

//     // Send the JSON response back to JS.
//     Ok(json)
// }

pub fn init_log() {
    use log::Level;
    console_log::init_with_level(Level::Trace).expect("error initializing log");
}
