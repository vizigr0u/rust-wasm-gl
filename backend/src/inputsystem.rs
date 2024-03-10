use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use log::info;
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{EventTarget, HtmlCanvasElement};

type InputCallback = Box<dyn FnMut(&InputEventType) + 'static>;

pub enum InputEventType {
    MouseDown(Rc<web_sys::MouseEvent>),
    MouseUp(Rc<web_sys::MouseEvent>),
    MouseMove(Rc<web_sys::MouseEvent>),
    KeyDown(Rc<web_sys::KeyboardEvent>),
    KeyUp(Rc<web_sys::KeyboardEvent>),
}

pub type SubscriberId = usize;
type SubscriberType = (SubscriberId, InputCallback);

pub struct InputSubscription {
    id: SubscriberId,
    subscribers: Arc<Mutex<Vec<SubscriberType>>>,
}

impl Drop for InputSubscription {
    fn drop(&mut self) {
        info!("InputSystem auto-unsubscribe: #{}", self.id);
        self.unsubscribe();
    }
}

impl InputSubscription {
    pub fn new(id: SubscriberId, subscribers: Arc<Mutex<Vec<SubscriberType>>>) -> Self {
        Self { id, subscribers }
    }

    fn unsubscribe(&mut self) {
        info!("InputSystem unsubscribe: #{}", self.id);
        let mut subs = self.subscribers.lock().unwrap();
        if subs.is_empty() {
            return;
        }
        subs.retain(|(id, _)| *id != self.id);
    }
}

pub struct InputSystem {
    subscribers: Arc<Mutex<Vec<SubscriberType>>>,
    closures: Vec<Arc<Mutex<Closure<dyn FnMut(JsValue)>>>>,
    next_id: SubscriberId,
}

impl InputSystem {
    pub fn new(canvas: &HtmlCanvasElement) -> Result<Self, JsValue> {
        let subscribers = Arc::new(Mutex::new(Vec::<SubscriberType>::new()));
        let mut system = Self {
            subscribers,
            closures: Vec::new(),
            next_id: 1,
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

    pub fn subscribe(&mut self, subscriber: InputCallback) -> InputSubscription {
        let mut subs = self.subscribers.lock().unwrap();
        let id = self.next_id;
        self.next_id += 1;
        subs.push((id, subscriber));
        info!("InputSystem subscription #{id}");
        InputSubscription::new(id, self.subscribers.clone())
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
            for (_, subscriber) in subscribers.lock().unwrap().iter_mut() {
                subscriber(&event_type);
            }
        }) as Box<dyn FnMut(JsValue)>);
        let rc_closure = Arc::new(Mutex::new(closure));
        target.add_event_listener_with_callback(
            event_type,
            rc_closure.lock().unwrap().as_ref().unchecked_ref(),
        )?;

        // closure.forget();

        // Keep the closure from being dropped
        self.closures.push(rc_closure);

        Ok(())
    }
}
