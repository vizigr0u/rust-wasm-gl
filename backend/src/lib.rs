use egui::mutex::Mutex;
use graphics::{GameRenderer, Gl2Renderer, WgpuRenderer};
use instant::Duration;
use log::{info, warn};
use std::sync::Arc;
use utils::get_canvas_by_id;
use wasm_thread as thread;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    platform::web::WindowExtWebSys,
    window::{Window, WindowBuilder},
};

async fn run_wgpu(event_loop: EventLoop<()>, winit_window: Window) {
    let window = GlobalProxy::new()
        .expect("Unable to create global proxy")
        .get_window()
        .expect("Unable to get window");
    let webgl2_canvas = get_canvas_by_id(&window, "canvas").expect("Unable to get canvas");

    let (graphics_sender, graphics_receiver) = mpsc::channel::<SerializedCommand>();
    let (webgl2_sender, webgl2_receiver) = mpsc::channel::<SerializedCommand>();
    let (wgpu_sender, wgpu_receiver) = mpsc::channel::<SerializedCommand>();

    let (game_sender, game_tick_receiver) = mpsc::channel::<Tick>();

    let game_graphics_sender = graphics_sender.clone();

    // Spawn game thread
    thread::spawn(move || {
        info!("Starting game script loop");
        let mut game = Game::new(game_graphics_sender).expect("Unable to create game");
        for _ in game_tick_receiver {
            game.tick().unwrap()
        }
    });

    // Dispatcher
    thread::spawn(move || {
        info!("Starting dispatcher");
        while let Ok(c) = graphics_receiver.recv() {
            wgpu_sender.send(c.clone()).unwrap();
            webgl2_sender.send(c).unwrap();
        }
    });

    let mut game_gl2_renderer =
        GameRenderer::<Gl2Renderer>::from_canvas(webgl2_canvas, webgl2_receiver)
            .await
            .expect("Unable to create GL2 Game renderer");

    let window = &winit_window;
    if let Some(wgpu_canvas) = window.canvas() {
        // let game_state = Arc::new(Mutex::new(gamestate::GameState {
        //     mouse_pos: (0.0, 0.0),
        //     gui_state: None,
        // }));

        let mut wgpu_renderer =
            GameRenderer::<WgpuRenderer>::from_canvas(wgpu_canvas, wgpu_receiver)
                .await
                .expect("Unable to create GL2 Game renderer");

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
                        WindowEvent::Resized(_physical_size) => {
                            // graphics_context.resize(*physical_size)
                        }
                        WindowEvent::ScaleFactorChanged { .. } => {
                            // graphics_context.resize(window.inner_size());
                        }
                        WindowEvent::CloseRequested => event_loop_window_target.exit(),
                        WindowEvent::CursorMoved { .. } => {
                            // game_state.lock().mouse_pos = (position.x, position.y);
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
                            let _ = game_sender.send(Tick {});
                            match game_gl2_renderer.tick() {
                                Ok(_) => {}
                                Err(s) => warn!("game_gl2_renderer tick error: {}", s),
                            };
                            match wgpu_renderer.tick() {
                                Ok(_) => {}
                                Err(s) => warn!("wgpu_renderer tick error: {}", s),
                            };
                            // match graphics_context.tick() {
                            //     Ok(_) => {}
                            //     // Recreate the swap_chain if lost
                            //     Err(wgpu::SurfaceError::Lost) => {
                            //         graphics_context.reset_swap_chain()
                            //     }
                            //     Err(wgpu::SurfaceError::OutOfMemory) => {
                            //         event_loop_window_target.exit()
                            //     }
                            //     Err(e) => eprintln!("Some unhandled error {:?}", e),
                            // }
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

use crate::{core::Game, graphics::commands::SerializedCommand, utils::GlobalProxy};

mod gamestate;

mod core;
mod graphics;
mod gui;
pub mod math;
mod objects;
mod utils;
// mod world;

use winit::platform::web::WindowBuilderExtWebSys;

struct Tick {}

#[wasm_bindgen(start)]
fn start() -> Result<(), JsValue> {
    utils::set_panic_hook();
    tracing_wasm::set_as_global_default();

    utils::init_log();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_append(true)
        .with_inner_size(LogicalSize::new(800, 600))
        .build(&event_loop)
        .unwrap();
    wasm_bindgen_futures::spawn_local(run_wgpu(event_loop, window));

    Ok(())
}
