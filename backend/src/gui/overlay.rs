//! Bridge to the TypeScript DOM debug overlay (`window.__rwgDebug`, see `src/debug-overlay.ts`).
//! Every call is a no-op when the object is missing, and state setters only call JS on change.

use js_sys::{Array, Float64Array, Function, Object, Reflect};
use wasm_bindgen::{JsCast, JsValue};

#[derive(Debug)]
pub struct DebugOverlay {
    api: Option<Api>,
    visible: Option<bool>,
    paused: Option<bool>,
    toggles: Vec<(String, bool)>,
}

#[derive(Debug)]
struct Api {
    this: JsValue,
    init: Function,
    set_visible: Function,
    set_paused: Function,
    set_toggles: Function,
    set_info: Function,
    set_graph: Function,
}

impl Api {
    fn lookup() -> Option<Self> {
        let this = Reflect::get(&web_sys::window()?.into(), &"__rwgDebug".into()).ok()?;
        if !this.is_object() {
            return None;
        }
        let f = |name: &str| -> Option<Function> {
            Reflect::get(&this, &name.into()).ok()?.dyn_into().ok()
        };
        Some(Self {
            init: f("init")?,
            set_visible: f("setVisible")?,
            set_paused: f("setPaused")?,
            set_toggles: f("setToggles")?,
            set_info: f("setInfo")?,
            set_graph: f("setGraph")?,
            this,
        })
    }
}

impl DebugOverlay {
    pub fn new() -> Self {
        Self {
            api: Api::lookup(),
            visible: None,
            paused: None,
            toggles: Vec::new(),
        }
    }

    pub fn init(&self, series_names: &[&str]) {
        if let Some(a) = &self.api {
            let names: Array = series_names.iter().map(|s| JsValue::from_str(s)).collect();
            let _ = a.init.call1(&a.this, &names);
        }
    }

    pub fn set_visible(&mut self, on: bool) {
        if let (Some(a), true) = (&self.api, self.visible != Some(on)) {
            let _ = a.set_visible.call1(&a.this, &on.into());
            self.visible = Some(on);
        }
    }

    pub fn set_paused(&mut self, on: bool) {
        if let (Some(a), true) = (&self.api, self.paused != Some(on)) {
            let _ = a.set_paused.call1(&a.this, &on.into());
            self.paused = Some(on);
        }
    }

    /// Sends only the entries that differ from the last values sent.
    pub fn set_toggles(&mut self, state: &[(&str, bool)]) {
        let Some(a) = &self.api else { return };
        let changed = Object::new();
        let mut any = false;
        for &(name, on) in state {
            match self.toggles.iter_mut().find(|(n, _)| n == name) {
                Some((_, last)) if *last == on => continue,
                Some((_, last)) => *last = on,
                None => self.toggles.push((name.to_owned(), on)),
            }
            let _ = Reflect::set(&changed, &name.into(), &on.into());
            any = true;
        }
        if any {
            let _ = a.set_toggles.call1(&a.this, &changed);
        }
    }

    pub fn set_info(&self, text: &str) {
        if let Some(a) = &self.api {
            let _ = a.set_info.call1(&a.this, &JsValue::from_str(text));
        }
    }

    /// `data` is series-major (7 x CHART_SIZE), `avgs` one average per series.
    pub fn set_graph(&self, data: &[f64], avgs: &[f64]) {
        if let Some(a) = &self.api {
            let _ = a.set_graph.call2(
                &a.this,
                &Float64Array::from(data),
                &Float64Array::from(avgs),
            );
        }
    }
}

impl Default for DebugOverlay {
    fn default() -> Self {
        Self::new()
    }
}
