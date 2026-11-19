use std::convert::TryInto;

use crate::core::Time;
use crate::core::{HandleInputs, InputEventType, InputState};
use crate::render::UiFrame;
use egui::{Event, Key};
use log::warn;
use web_sys::{KeyboardEvent, MouseEvent};

/// App-side egui: context, input translation, frame building. Painting lives in the renderer.
#[derive(Debug)]
pub struct EguiBackend {
    egui_ctx: egui::Context,
    current_events: Vec<Event>,
    size: (usize, usize),
    time: Time,
}

impl EguiBackend {
    pub fn new() -> Self {
        Self {
            egui_ctx: egui::Context::default(),
            current_events: Vec::new(),
            size: (800, 600),
            time: Time::default(),
        }
    }

    /// Run one egui pass and tessellate it for the renderer.
    pub fn run_ui<F>(&mut self, mut build_gui: F) -> UiFrame
    where
        F: FnMut(&egui::Context),
    {
        self.update_size();
        let raw_input: egui::RawInput = self.gather_input();

        let full_output = self.egui_ctx.run_pass(raw_input, |ctx| {
            build_gui(ctx);
        });
        self.handle_platform_output(full_output.platform_output);
        let primitives = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        UiFrame {
            textures_delta: full_output.textures_delta,
            primitives,
        }
    }

    /// Read the real drawing-buffer size from the canvas (egui points == pixels here).
    fn update_size(&mut self) {
        if let Ok(canvas) = crate::utils::get_canvas() {
            self.size = (canvas.width() as usize, canvas.height() as usize);
        }
    }

    // fn debug_paint(
    //     &mut self,
    //     textures_delta: &egui::TexturesDelta,
    //     clipped_primitives: &Vec<egui::ClippedPrimitive>,
    // ) {
    //     if self.egui_once {
    //         for (id, img_delta) in &textures_delta.set {
    //             info!("{id:?}, {:?}", img_delta.options);
    //         }
    //         for p in clipped_primitives {
    //             info!("prim: {:?}, {:?}", p.clip_rect, p.primitive);
    //         }
    //         self.egui_once = false;
    //     }
    // }

    fn handle_platform_output(&self, _platform_output: egui::PlatformOutput) {}

    fn gather_input(&self) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect {
                min: egui::pos2(0.0, 0.0),
                max: egui::pos2(self.size.0 as f32, self.size.1 as f32),
            }),
            events: self.current_events.clone(),
            // time: Some(self.time.time()),
            // predicted_dt: self.time.delta_time() as _,
            ..Default::default()
        }
    }

    fn set_events(&mut self, events: &Vec<InputEventType>) {
        self.current_events.clear();
        for ev in events.iter() {
            let e: Result<Event, ConversionError> = ev.try_into();
            match e {
                Ok(e) => {
                    self.current_events.push(e);
                    // Printable keys also produce text for text edits.
                    if let Some(text) = text_for_event(ev) {
                        self.current_events.push(Event::Text(text));
                    }
                }
                Err(e) => match e {
                    ConversionError::IgnoredKey(_) => {}
                    _ => warn!("Could not convert event: {e:?}"),
                },
            }
        }
    }

    pub fn clear_events(&mut self) {
        self.current_events.clear();
    }

    pub fn set_time(&mut self, time: &Time) {
        self.time = time.clone();
    }
}

impl HandleInputs for EguiBackend {
    fn handle_inputs(&mut self, inputs: &InputState) {
        self.set_events(inputs.get_events());
    }
}

#[derive(Debug)]
pub enum ConversionError {
    KeyNotFound(String),
    UnknownButton(i16),
    IgnoredKey(String),
    Unhandled,
}

impl TryInto<Event> for &InputEventType {
    type Error = ConversionError;

    fn try_into(self) -> Result<Event, Self::Error> {
        let event = match self {
            InputEventType::KeyDown(event) => Event::Key {
                key: try_parse_key(event.key())?,
                pressed: true,
                modifiers: get_key_modifiers(event),
                physical_key: None,
                repeat: event.repeat(),
            },
            InputEventType::KeyUp(event) => Event::Key {
                key: try_parse_key(event.key())?,
                pressed: false,
                modifiers: get_key_modifiers(event),
                physical_key: None,
                repeat: event.repeat(),
            },
            InputEventType::MouseMove(event) => Event::PointerMoved(mouse_event_to_pos2(event)),
            InputEventType::MouseDown(event) => Event::PointerButton {
                pos: mouse_event_to_pos2(event),
                button: try_parse_mouse_button(event.button())?,
                pressed: true,
                modifiers: get_mouse_modifiers(event),
            },
            InputEventType::MouseUp(event) => Event::PointerButton {
                pos: mouse_event_to_pos2(event),
                button: try_parse_mouse_button(event.button())?,
                pressed: false,
                modifiers: get_mouse_modifiers(event),
            },
            InputEventType::MouseWheel(event) => {
                let unit = match event.delta_mode() {
                    1 => egui::MouseWheelUnit::Line,
                    2 => egui::MouseWheelUnit::Page,
                    _ => egui::MouseWheelUnit::Point,
                };
                Event::MouseWheel {
                    unit,
                    // Web deltaY is positive when scrolling down; egui wants the opposite.
                    delta: egui::vec2(-event.delta_x() as f32, -event.delta_y() as f32),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Unknown,
                    modifiers: get_mouse_modifiers(event),
                }
            }
            _ => Err(ConversionError::Unhandled)?,
        };
        Ok(event)
    }
}

fn get_key_modifiers(event: &KeyboardEvent) -> egui::Modifiers {
    let mut modifiers = egui::Modifiers::default();
    modifiers.shift = event.get_modifier_state("Shift");
    modifiers.ctrl = event.get_modifier_state("Control");
    modifiers.alt = event.get_modifier_state("Alt");
    modifiers.command = event.get_modifier_state("Meta");

    modifiers
}

fn get_mouse_modifiers(event: &MouseEvent) -> egui::Modifiers {
    let mut modifiers = egui::Modifiers::default();
    modifiers.shift = event.get_modifier_state("Shift");
    modifiers.ctrl = event.get_modifier_state("Control");
    modifiers.alt = event.get_modifier_state("Alt");
    modifiers.command = event.get_modifier_state("Meta");

    modifiers
}

fn try_parse_mouse_button(button: i16) -> Result<egui::PointerButton, ConversionError> {
    match button {
        0 => Ok(egui::PointerButton::Primary),
        1 => Ok(egui::PointerButton::Middle),
        2 => Ok(egui::PointerButton::Secondary),
        n => Err(ConversionError::UnknownButton(n))?,
    }
}

fn try_parse_key(key: String) -> Result<egui::Key, ConversionError> {
    if key == "Shift" || key == "Control" || key == "Alt" || key == "Meta" {
        Err(ConversionError::IgnoredKey(key))
    } else {
        Key::from_name(key.as_str()).ok_or(ConversionError::KeyNotFound(key))
    }
}

/// Text typed by a KeyDown event (printable single characters, no ctrl/meta chords).
fn text_for_event(event: &InputEventType) -> Option<String> {
    let InputEventType::KeyDown(e) = event else {
        return None;
    };
    if e.ctrl_key() || e.meta_key() {
        return None;
    }
    let key = e.key();
    (key.chars().count() == 1).then_some(key)
}

/// Canvas-relative position in drawing-buffer pixels (accounts for CSS scaling).
fn mouse_event_to_pos2(event: &MouseEvent) -> egui::Pos2 {
    let (cx, cy) = (event.client_x() as f32, event.client_y() as f32);
    let Ok(canvas) = crate::utils::get_canvas() else {
        return egui::pos2(cx, cy);
    };
    let rect = canvas.get_bounding_client_rect();
    let sx = if rect.width() > 0.0 {
        canvas.width() as f32 / rect.width() as f32
    } else {
        1.0
    };
    let sy = if rect.height() > 0.0 {
        canvas.height() as f32 / rect.height() as f32
    } else {
        1.0
    };
    egui::pos2(
        (cx - rect.left() as f32) * sx,
        (cy - rect.top() as f32) * sy,
    )
}
