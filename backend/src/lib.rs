use instant::Duration;
use render::GraphicsContext;
use std::sync::{Arc, Mutex};
use wasm_thread as thread;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    platform::web::WindowExtWebSys,
    window::{Window, WindowBuilder},
};

mod gamestate;
mod render;

async fn run(event_loop: EventLoop<()>, window: Window) {
    let window = &window;
    if let Some(canvas) = window.canvas() {
        let offscreen_canvas = canvas
            .transfer_control_to_offscreen()
            .expect("canvas transfer failed");
        let game_state = Arc::new(Mutex::new(gamestate::GameState {
            mouse_pos: (0.0, 0.0),
        }));

        let mut graphics_context = GraphicsContext::new(offscreen_canvas, game_state.clone()).await;

        let state_clone = game_state.clone();
        thread::spawn(move || {
            while true {
                thread::sleep(Duration::from_secs(1));
                state_clone.lock().unwrap().mouse_pos = (0.0, 0.0);
            }
        });

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
                            game_state.lock().unwrap().mouse_pos = (position.x, position.y);
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

use wasm_bindgen::prelude::*;
use winit::platform::web::WindowBuilderExtWebSys;

#[wasm_bindgen(start)]
pub fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_append(true)
        .with_inner_size(PhysicalSize::new(800, 600))
        .build(&event_loop)
        .unwrap();

    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("could not initialize logger");

        wasm_bindgen_futures::spawn_local(run(event_loop, window));
    }
}
