//! DevTools Performance-panel tracing. Opt-in with the `?trace` URL param, read once.
//! Entries use the extended `console.timeStamp` (custom tracks); off = no clock reads, no JS calls.
use std::cell::OnceCell;

use wasm_bindgen::{intern, prelude::*};

use super::performance_now;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = timeStamp)]
    fn time_stamp(label: &str, start: f64, end: f64, track: &str, group: &str, color: &str);
}

const GROUP: &str = "Engine";

#[derive(Clone, Copy)]
pub enum Track {
    Frame,
    World,
    Render,
}

impl Track {
    fn name(self) -> &'static str {
        match self {
            Track::Frame => "Frame",
            Track::World => "World",
            Track::Render => "Render",
        }
    }

    fn color(self) -> &'static str {
        match self {
            Track::Frame => "primary",
            Track::World => "secondary",
            Track::Render => "tertiary",
        }
    }
}

thread_local! {
    static ENABLED: OnceCell<bool> = const { OnceCell::new() };
}

fn url_has_trace() -> bool {
    web_sys::window()
        .and_then(|w| w.location().search().ok())
        .is_some_and(|s| {
            s.trim_start_matches('?')
                .split('&')
                .any(|p| p == "trace" || p.starts_with("trace="))
        })
}

pub fn enabled() -> bool {
    ENABLED.with(|c| *c.get_or_init(url_has_trace))
}

/// Record `label` from `start` to `end` (`performance.now()` ms) on `track`.
pub fn entry(label: &'static str, start: f64, end: f64, track: Track) {
    if !enabled() {
        return;
    }
    time_stamp(
        intern(label),
        start,
        end,
        intern(track.name()),
        intern(GROUP),
        intern(track.color()),
    );
}

/// RAII timing guard: records on drop. Does not read the clock when tracing is off.
pub struct Span {
    label: &'static str,
    track: Track,
    start: f64,
    on: bool,
}

impl Span {
    pub fn new(label: &'static str, track: Track) -> Self {
        let on = enabled();
        Self {
            label,
            track,
            start: if on { performance_now() } else { 0.0 },
            on,
        }
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        if self.on {
            entry(self.label, self.start, performance_now(), self.track);
        }
    }
}
