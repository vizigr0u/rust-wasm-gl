use egui::mutex::Mutex;
use graphics::{GameRenderer, Gl2Renderer, Renderer};
use instant::{Duration, Instant};
use log::info;
use render::GraphicsContext;
use std::{cell::RefCell, rc::Rc, sync::Arc};
use utils::get_canvas;
use wasm_thread as thread;
use web_sys::{DedicatedWorkerGlobalScope, WorkerGlobalScope};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    platform::web::WindowExtWebSys,
    window::{Window, WindowBuilder},
};

async fn run_wgpu(event_loop: EventLoop<()>, window: Window) {
    let window = &window;
    if let Some(canvas) = window.canvas() {
        let offscreen_canvas = canvas
            .transfer_control_to_offscreen()
            .expect("canvas transfer failed");
        let game_state = Arc::new(Mutex::new(gamestate::GameState {
            mouse_pos: (0.0, 0.0),
            gui_state: None,
        }));

        let mut graphics_context = GraphicsContext::new(offscreen_canvas, game_state.clone()).await;

        {
            let state_clone = game_state.clone();
            thread::spawn(move || loop {
                thread::sleep(Duration::from_secs(1));
                state_clone.lock().mouse_pos = (0.0, 0.0);
            });
        }
        let egui_ctx = egui::Context::default();
        // let e = egui_wgpu::winit::Painter::new(
        //     egui_wgpu::WgpuConfiguration {
        //         supported_backends: Backend::BrowserWebGpu.into(),
        //         device_descriptor: (),
        //         present_mode: (),
        //         desired_maximum_frame_latency: (),
        //         power_preference: (),
        //         on_surface_error: (),
        //     },
        //     1,
        //     None,
        //     false,
        // );

        event_loop
            .run(|event, event_loop_window_target| match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == window.id() => {
                    match &event {
                        WindowEvent::Resized(physical_size) => {
                            graphics_context.resize(*physical_size)
                        }
                        WindowEvent::ScaleFactorChanged { .. } => {
                            graphics_context.resize(window.inner_size());
                        }
                        WindowEvent::CloseRequested => event_loop_window_target.exit(),
                        WindowEvent::CursorMoved { position, .. } => {
                            game_state.lock().mouse_pos = (position.x, position.y);
                        }
                        WindowEvent::KeyboardInput {
                            event: key_event, ..
                        } => {
                            if key_event.state.is_pressed() {
                                match key_event.physical_key {
                                    PhysicalKey::Code(KeyCode::Escape) => {
                                        event_loop_window_target.exit();
                                    }
                                    _ => {}
                                }
                            }
                        }
                        WindowEvent::RedrawRequested => {
                            let raw_input = egui::RawInput::default();
                            game_state.lock().gui_state = Some(egui_ctx.run(raw_input, |ctx| {
                                egui::CentralPanel::default().show(&ctx, |ui| {
                                    ui.label("Hello world!");
                                    if ui.button("Click me").clicked() {
                                        // take some action here
                                    }
                                });
                            }));
                            match graphics_context.tick() {
                                Ok(_) => {}
                                // Recreate the swap_chain if lost
                                Err(wgpu::SurfaceError::Lost) => {
                                    graphics_context.reset_swap_chain()
                                }
                                Err(wgpu::SurfaceError::OutOfMemory) => {
                                    event_loop_window_target.exit()
                                }
                                Err(e) => eprintln!("Some unhandled error {:?}", e),
                            }
                        }
                        _ => {}
                    }
                }
                Event::AboutToWait => {
                    // RedrawRequested will only trigger once, unless we manually
                    // request it.
                    window.request_redraw();
                }
                _ => {}
            })
            .unwrap()
    }
}

use std::sync::mpsc;
use wasm_bindgen::prelude::*;

use crate::core::Game;

mod gamestate;
mod render;

mod core;
mod graphics;
mod gui;
pub mod math;
mod objects;
mod utils;
// mod world;

use winit::platform::web::WindowBuilderExtWebSys;

struct Tick {
    time: f64,
}

#[wasm_bindgen(start)]
fn start() -> Result<(), JsValue> {
    utils::set_panic_hook();
    tracing_wasm::set_as_global_default();

    utils::init_log();

    let canvas = get_canvas()?;
    let renderer = Gl2Renderer::new(canvas)?;
    info!("renderer created");

    use crate::graphics::commands::SerializedCommand;
    let (graphics_sender, graphics_receiver) = mpsc::channel::<SerializedCommand>();
    let (game_sender, game_tick_receiver) = mpsc::channel::<Tick>();

    let renderer = GameRenderer::new(
        /*game.lock().state().clone(), */ renderer,
        graphics_receiver,
    )?;

    let game_graphics_sender = graphics_sender.clone();
    thread::spawn(move || {
        let mut game = Game::new(game_graphics_sender).unwrap();
        for tick in game_tick_receiver {
            game.tick(tick.time).unwrap()
        }
    });

    thread::spawn(move || {
        let _ = infinite_loop_render_test();
    });

    info!("game and game renderer created");

    // main_loop(game_sender, renderer)?;

    // info!("game loop started");

    {
        let event_loop = EventLoop::new().unwrap();
        let window = WindowBuilder::new()
            .with_append(true)
            .with_inner_size(PhysicalSize::new(800, 600))
            .build(&event_loop)
            .unwrap();
        wasm_bindgen_futures::spawn_local(run_wgpu(event_loop, window));
    }

    Ok(())
}

fn request_animation_frame(
    f: &Closure<dyn FnMut(f64) -> Result<(), JsValue>>,
) -> Result<i32, JsValue> {
    info!("request_animation_frame");
    match &web_sys::window() {
        Some(window) => {
            info!("got window");
            window.request_animation_frame(f.as_ref().unchecked_ref())
        }
        None => {
            info!("no window!");
            Err(JsValue::from_str("no window!"))
        }
    }
}

fn main_loop<R>(game_sender: mpsc::Sender<Tick>, renderer: GameRenderer<R>) -> Result<(), JsValue>
where
    R: Renderer + 'static,
{
    let renderer = Rc::new(RefCell::new(renderer));
    let update: Rc<RefCell<Option<Closure<dyn FnMut(f64) -> Result<(), JsValue>>>>> =
        Rc::new(RefCell::new(None));
    /* Reference to closure requests for first frame and then it's dropped */
    let request_update = update.clone();

    let game_sender = Rc::new(RefCell::new(game_sender.clone()));
    *request_update.borrow_mut() = Some(Closure::new(move |time| {
        let _ = game_sender.borrow().send(Tick { time });
        // thread::spawn(move || {
        // let _ = game.lock().tick(time);
        // });
        renderer.borrow_mut().tick()?;

        // Request the next animation frame.
        request_animation_frame(update.borrow().as_ref().unwrap());
        Ok(())
    }));

    request_animation_frame(request_update.borrow().as_ref().unwrap());

    Ok(())
}

enum GlobalProxy {
    Window(web_sys::Window),
    WorkerGlobalScope(DedicatedWorkerGlobalScope),
    //... more scopes
}

impl GlobalProxy {
    fn request_animation_frame(
        &self,
        f: &Closure<dyn FnMut(f64) -> Result<(), JsValue>>,
    ) -> Result<i32, JsValue> {
        match self {
            GlobalProxy::Window(window) => {
                info!("request_animation_frame for window");
                window.request_animation_frame(f.as_ref().unchecked_ref())
            }
            GlobalProxy::WorkerGlobalScope(scope) => {
                info!("request_animation_frame for worker");
                scope.request_animation_frame(f.as_ref().unchecked_ref())
            } //... more of that
        }
    }
}

fn self_() -> Result<GlobalProxy, JsValue> {
    let global = js_sys::global();
    // how to properly detect this in wasm_bindgen?
    if js_sys::eval("typeof DedicatedWorkerGlobalScope !== 'undefined'")?
        .as_bool()
        .unwrap()
    {
        Ok(global
            .dyn_into::<DedicatedWorkerGlobalScope>()
            .map(GlobalProxy::WorkerGlobalScope)?)
    } else {
        Ok(global
            .dyn_into::<web_sys::Window>()
            .map(GlobalProxy::Window)?)
    }
}

fn infinite_loop_render_test() {
    info!("infinite_loop_render_test");

    let update: Rc<RefCell<Option<Closure<dyn FnMut(f64) -> Result<(), JsValue>>>>> =
        Rc::new(RefCell::new(None));
    /* Reference to closure requests for first frame and then it's dropped */
    let request_update = update.clone();
    *request_update.borrow_mut() = Some(Closure::new(move |time| {
        info!("Ticking Test - time: {time} - instant {:?}", Instant::now());

        // Request the next animation frame.
        let scope = self_().unwrap();
        scope.request_animation_frame(update.borrow().as_ref().expect("no inner update"))?;
        Ok(())
    }));

    info!("requesting first frame");
    let scope = self_().unwrap();
    scope
        .request_animation_frame(request_update.borrow().as_ref().expect("no first update"))
        .expect("no first request");
    loop {
        thread::sleep(Duration::from_millis(1));
    }
}
