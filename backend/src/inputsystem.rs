use std::{
    cell::RefCell,
    collections::HashSet,
    rc::Rc,
    sync::{Arc, Mutex},
};

use glam::Vec2;
use log::{info, warn};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::EventTarget;

use crate::utils;

#[derive(Debug)]
pub enum InputEventType {
    MouseDown(Rc<web_sys::MouseEvent>),
    MouseUp(Rc<web_sys::MouseEvent>),
    MouseMove(Rc<web_sys::MouseEvent>),
    MouseWheel(Rc<web_sys::WheelEvent>),
    KeyDown(Rc<web_sys::KeyboardEvent>),
    KeyUp(Rc<web_sys::KeyboardEvent>),
}

impl Clone for InputEventType {
    fn clone(&self) -> Self {
        match self {
            InputEventType::MouseDown(e) => InputEventType::MouseDown(e.clone()),
            InputEventType::MouseUp(e) => InputEventType::MouseUp(e.clone()),
            InputEventType::MouseMove(e) => InputEventType::MouseMove(e.clone()),
            InputEventType::MouseWheel(e) => InputEventType::MouseWheel(e.clone()),
            InputEventType::KeyDown(e) => InputEventType::KeyDown(e.clone()),
            InputEventType::KeyUp(e) => InputEventType::KeyUp(e.clone()),
        }
    }
}

#[derive(Debug)]
pub struct InputState {
    mouse_pos: Vec2,
    mouse_delta: Vec2,
    mouse_down: bool,
    keys_down: HashSet<String>,
    current_events: Vec<InputEventType>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            mouse_pos: Vec2::new(0.0, 0.0),
            mouse_delta: Vec2::new(0.0, 0.0),
            mouse_down: false,
            keys_down: HashSet::new(),
            current_events: Vec::new(),
        }
    }
    pub fn get_mouse_pos(&self) -> Vec2 {
        self.mouse_pos
    }
    pub fn get_mouse_delta(&self) -> Vec2 {
        self.mouse_delta
    }
    pub fn is_mouse_down(&self) -> bool {
        self.mouse_down
    }
    pub fn is_key_down(&self, key: &str) -> bool {
        self.keys_down.contains(key)
    }

    pub fn get_events(&self) -> &Vec<InputEventType> {
        &self.current_events
    }

    fn add_event(&mut self, event: InputEventType) {
        match &event {
            InputEventType::MouseDown(_) => self.mouse_down = true,
            InputEventType::MouseUp(_) => self.mouse_down = false,
            InputEventType::MouseMove(e) => {
                let new_pos = Vec2::new(e.client_x() as _, e.client_y() as _);
                self.mouse_delta = new_pos - self.mouse_pos;
                self.mouse_pos = new_pos;
            }
            InputEventType::KeyDown(e) => {
                if !self.keys_down.insert(e.code()) {
                    warn!("Key already down: {}", e.code());
                }
            }
            InputEventType::KeyUp(e) => {
                if !self.keys_down.remove(&e.code()) {
                    warn!("Key not down: {}", e.code());
                }
            }
            InputEventType::MouseWheel(_) => (),
        }
        self.current_events.push(event);
    }
}

impl Clone for InputState {
    fn clone(&self) -> Self {
        Self {
            mouse_pos: self.mouse_pos.clone(),
            mouse_delta: self.mouse_delta.clone(),
            mouse_down: self.mouse_down,
            keys_down: self.keys_down.clone(),
            current_events: self.current_events.clone(),
        }
    }
}

pub trait HandleInputs {
    fn handle_inputs(&mut self, inputs: &InputState);
}

#[derive(Debug)]
pub struct InputSystem {
    current_inputs: Rc<RefCell<InputState>>,
    closures: Vec<Arc<Mutex<Closure<dyn FnMut(JsValue)>>>>,
}

impl InputSystem {
    pub fn new() -> Result<Self, JsValue> {
        let mut system = Self {
            current_inputs: Rc::new(RefCell::new(InputState::new())),
            closures: Vec::new(),
        };
        system.sub_to_all_events()?;

        Ok(system)
    }

    pub fn get_inputs(&self) -> InputState {
        self.current_inputs.borrow().clone()
    }

    pub fn clear_events(&mut self) {
        self.current_inputs.borrow_mut().current_events.clear();
    }

    fn sub_to_all_events(&mut self) -> Result<(), JsValue> {
        let canvas = &utils::get_canvas()?;
        self.add_event_listener(
            canvas,
            "mousedown",
            Box::new(|event: JsValue| {
                let event: web_sys::MouseEvent = event.dyn_into().unwrap();
                InputEventType::MouseDown(Rc::new(event))
            }),
        )?;
        self.add_event_listener(
            canvas,
            "mouseup",
            Box::new(|event: JsValue| {
                let event: web_sys::MouseEvent = event.dyn_into().unwrap();
                InputEventType::MouseUp(Rc::new(event))
            }),
        )?;
        self.add_event_listener(
            canvas,
            "mousemove",
            Box::new(|event: JsValue| {
                let event: web_sys::MouseEvent = event.dyn_into().unwrap();
                InputEventType::MouseMove(Rc::new(event))
            }),
        )?;
        self.add_event_listener(
            canvas,
            "wheel",
            Box::new(|event: JsValue| {
                let event: web_sys::WheelEvent = event.dyn_into().unwrap();
                InputEventType::MouseWheel(Rc::new(event))
            }),
        )?;

        self.add_event_listener(
            canvas,
            "keydown",
            Box::new(|event: JsValue| {
                let event: web_sys::KeyboardEvent = event.dyn_into().unwrap();
                InputEventType::KeyDown(Rc::new(event))
            }),
        )?;

        self.add_event_listener(
            canvas,
            "keyup",
            Box::new(|event: JsValue| {
                let event: web_sys::KeyboardEvent = event.dyn_into().unwrap();
                InputEventType::KeyUp(Rc::new(event))
            }),
        )?;

        Ok(())
    }

    fn add_event_listener<F>(
        &mut self,
        target: &EventTarget,
        event_type: &str,
        mut convert: Box<F>,
    ) -> Result<(), JsValue>
    where
        F: 'static + FnMut(JsValue) -> InputEventType,
    {
        let inputs = self.current_inputs.clone();
        let closure = Closure::wrap(Box::new(move |event: JsValue| {
            let event_type = convert(event);
            inputs.borrow_mut().add_event(event_type);
        }) as Box<dyn FnMut(JsValue)>);
        let rc_closure = Arc::new(Mutex::new(closure));
        target.add_event_listener_with_callback(
            event_type,
            rc_closure.lock().unwrap().as_ref().unchecked_ref(),
        )?;

        // Keep the closure from being dropped
        self.closures.push(rc_closure);

        Ok(())
    }
}
