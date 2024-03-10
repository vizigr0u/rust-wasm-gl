use std::{cell::RefCell, rc::Rc};

use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{EventTarget, HtmlCanvasElement};

pub trait InputReactive {
    fn handle_input(&mut self, ev: &InputEventType);
}

pub enum InputEventType {
    MouseDown(Rc<web_sys::MouseEvent>),
    MouseUp(Rc<web_sys::MouseEvent>),
    MouseMove(Rc<web_sys::MouseEvent>),
    KeyDown(Rc<web_sys::KeyboardEvent>),
    KeyUp(Rc<web_sys::KeyboardEvent>),
}

pub struct InputSystem {
    subscribers: Rc<RefCell<Vec<Rc<RefCell<dyn InputReactive>>>>>,
    closures: Vec<Rc<RefCell<Closure<dyn FnMut(JsValue)>>>>,
}

impl InputSystem {
    pub fn new(canvas: &HtmlCanvasElement) -> Result<Self, JsValue> {
        let subscribers = Rc::new(RefCell::new(Vec::<Rc<RefCell<dyn InputReactive>>>::new()));
        let mut system = Self {
            subscribers,
            closures: Vec::new(),
        };

        system.add_event_listener(
            canvas,
            "mousedown",
            Box::new(|event: JsValue| {
                let event: web_sys::MouseEvent = event.dyn_into().unwrap();
                InputEventType::MouseDown(Rc::new(event))
            }),
        )?;
        system.add_event_listener(
            canvas,
            "mouseup",
            Box::new(|event: JsValue| {
                let event: web_sys::MouseEvent = event.dyn_into().unwrap();
                InputEventType::MouseUp(Rc::new(event))
            }),
        )?;
        system.add_event_listener(
            canvas,
            "mousemove",
            Box::new(|event: JsValue| {
                let event: web_sys::MouseEvent = event.dyn_into().unwrap();
                InputEventType::MouseMove(Rc::new(event))
            }),
        )?;

        system.add_event_listener(
            canvas,
            "keydown",
            Box::new(|event: JsValue| {
                let event: web_sys::KeyboardEvent = event.dyn_into().unwrap();
                InputEventType::KeyDown(Rc::new(event))
            }),
        )?;

        system.add_event_listener(
            canvas,
            "keyup",
            Box::new(|event: JsValue| {
                let event: web_sys::KeyboardEvent = event.dyn_into().unwrap();
                InputEventType::KeyUp(Rc::new(event))
            }),
        )?;
        Ok(system)
    }

    pub fn subscribe(&mut self, subscriber: Rc<RefCell<dyn InputReactive>>) {
        self.subscribers.borrow_mut().push(subscriber);
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
        let subscribers = self.subscribers.clone();
        let closure = Closure::wrap(Box::new(move |event: JsValue| {
            let event_type = convert(event);
            for subscriber in subscribers.borrow_mut().iter_mut() {
                subscriber.borrow_mut().handle_input(&event_type);
            }
        }) as Box<dyn FnMut(JsValue)>);
        let rc_closure = Rc::new(RefCell::new(closure));

        target.add_event_listener_with_callback(
            event_type,
            rc_closure.borrow().as_ref().unchecked_ref(),
        )?;

        // closure.forget();

        // Keep the closure from being dropped
        self.closures.push(rc_closure);

        Ok(())
    }
}
