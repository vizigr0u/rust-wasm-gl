use std::{iter::Sum, ops::Div};

use egui::{Button, Color32, RichText};
use egui_plot::{Legend, Line, Plot};

const CHART_SIZE: usize = 40;
const AVG_SIZE: usize = 3;
const BUFFER_SIZE: usize = CHART_SIZE * AVG_SIZE;
const REFRESH_EVERY: usize = 10;

/// Milliseconds spent in each part of one frame (all measured with `performance.now()`).
#[derive(Debug, Default, Clone, Copy)]
pub struct FrameTimes {
    /// Inputs, world update, camera (without chunk uploads).
    pub script: f64,
    /// `upload_chunk` / `remove_chunk` calls.
    pub upload: f64,
    /// egui run + tessellation.
    pub ui: f64,
    /// Renderer CPU prep (draw list, per-frame uploads).
    pub prepare: f64,
    /// Surface acquire + view (webgpu).
    pub acquire: f64,
    /// Main 3D pass encode.
    pub pass: f64,
    /// egui buffers + pass.
    pub egui: f64,
    /// Submit + present (webgpu).
    pub submit: f64,
    /// Start of the tick to the end of it.
    pub frame: f64,
    /// Time since the previous frame started (real frame period).
    pub interval: f64,
}

type PerformanceTime = FrameTimes;

impl Sum<PerformanceTime> for PerformanceTime {
    fn sum<I: Iterator<Item = PerformanceTime>>(iter: I) -> Self {
        iter.fold(PerformanceTime::default(), |a, b| PerformanceTime {
            script: a.script + b.script,
            upload: a.upload + b.upload,
            ui: a.ui + b.ui,
            prepare: a.prepare + b.prepare,
            acquire: a.acquire + b.acquire,
            pass: a.pass + b.pass,
            egui: a.egui + b.egui,
            submit: a.submit + b.submit,
            frame: a.frame + b.frame,
            interval: a.interval + b.interval,
        })
    }
}

impl<'a> Sum<&'a PerformanceTime> for PerformanceTime {
    fn sum<I: Iterator<Item = &'a PerformanceTime>>(iter: I) -> Self {
        iter.fold(PerformanceTime::default(), |a, b| PerformanceTime {
            script: a.script + b.script,
            upload: a.upload + b.upload,
            ui: a.ui + b.ui,
            prepare: a.prepare + b.prepare,
            acquire: a.acquire + b.acquire,
            pass: a.pass + b.pass,
            egui: a.egui + b.egui,
            submit: a.submit + b.submit,
            frame: a.frame + b.frame,
            interval: a.interval + b.interval,
        })
    }
}

impl Div<f64> for PerformanceTime {
    type Output = PerformanceTime;
    fn div(self, rhs: f64) -> Self::Output {
        PerformanceTime {
            script: self.script / rhs,
            upload: self.upload / rhs,
            ui: self.ui / rhs,
            prepare: self.prepare / rhs,
            acquire: self.acquire / rhs,
            pass: self.pass / rhs,
            egui: self.egui / rhs,
            submit: self.submit / rhs,
            frame: self.frame / rhs,
            interval: self.interval / rhs,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FpsDisplay {
    tick_times: [PerformanceTime; BUFFER_SIZE],
    avg_times: PerformanceTime,
    tick_index: usize,
    show_details: bool,
    buffer_was_filled: bool,
    last_start: Option<f64>,
}

impl FpsDisplay {
    pub fn new() -> Self {
        Self {
            tick_times: [PerformanceTime::default(); BUFFER_SIZE],
            avg_times: PerformanceTime::default(),
            tick_index: 0,
            show_details: false,
            buffer_was_filled: false,
            last_start: None,
        }
    }

    /// `start_time`: `performance.now()` at the start of the tick; `times.interval` is filled here.
    pub fn update_fps_report(&mut self, start_time: f64, mut times: FrameTimes) {
        times.interval = self.last_start.map_or(0.0, |last| start_time - last);
        self.last_start = Some(start_time);
        self.tick_times[self.tick_index] = times;
        self.tick_index = (self.tick_index + 1) % self.tick_times.len();
        if self.tick_index == BUFFER_SIZE - 1 {
            self.buffer_was_filled = true;
        }
        // Refresh often (every REFRESH_EVERY frames) instead of once per full buffer.
        if self.tick_index.is_multiple_of(REFRESH_EVERY) {
            let count = if self.buffer_was_filled {
                self.tick_times.len()
            } else {
                self.tick_index.max(1)
            };
            self.avg_times =
                self.tick_times[..count].iter().sum::<PerformanceTime>() / count as f64;
        }
    }
    pub fn get_debug_info(&self) -> String {
        if self.avg_times.interval <= 0.0 {
            return "???".to_string();
        }
        let t = self.avg_times;
        // fps = real frame rate; max = theoretical rate if the CPU work were the only limit
        format!(
            "{:.0} (actual {:.1}) | ui {:.2}ms, render {:.2}ms",
            1000.0 / self.avg_times.frame.max(0.001),
            1000.0 / self.avg_times.interval,
            self.avg_times.ui,
            t.prepare + t.acquire + t.pass + t.egui + t.submit
        )
    }

    /// Button toggling the frame-time graph.
    pub fn graph_button(&mut self, ui: &mut egui::Ui) {
        let label = if self.show_details {
            "Hide frame graph"
        } else {
            "Show frame graph"
        };
        if ui
            .add(Button::new(RichText::new(label).color(Color32::WHITE)))
            .clicked()
        {
            self.show_details = !self.show_details;
        }
    }

    pub fn display(&mut self, ui: &mut egui::Ui) {
        if self.show_details {
            Plot::new("fps")
                .height(80.0)
                .width(600.0)
                .show_axes(false)
                .legend(Legend::default().position(egui_plot::Corner::LeftBottom))
                .show(ui, |plot_ui| {
                    // plot_ui.line(
                    //     Line::new(
                    //         (0..BUFFER_SIZE)
                    //             .map(|i| {
                    //                 let index = (i + self.tick_index) % BUFFER_SIZE;
                    //                 [i as f64, self.tick_times[index].frame_time]
                    //             })
                    //             .collect::<Vec<_>>(),
                    //     )
                    //     .name("frame"),
                    // );
                    let charts = (0..CHART_SIZE)
                        .map(|i| {
                            let avg: PerformanceTime = (0..AVG_SIZE)
                                .map(|j| {
                                    self.tick_times
                                        [(i * AVG_SIZE + j + self.tick_index) % BUFFER_SIZE]
                                })
                                .sum::<PerformanceTime>()
                                / AVG_SIZE as f64;
                            avg
                        })
                        .collect::<Vec<_>>();
                    plot_ui.line(Line::new(
                        format!("script: {:.2}ms", self.avg_times.script),
                        charts
                            .iter()
                            .enumerate()
                            .map(|(i, &v)| [i as f64, v.script])
                            .collect::<Vec<_>>(),
                    ));
                    plot_ui.line(Line::new(
                        format!("upload: {:.2}ms", self.avg_times.upload),
                        charts
                            .iter()
                            .enumerate()
                            .map(|(i, &v)| [i as f64, v.upload])
                            .collect::<Vec<_>>(),
                    ));
                    plot_ui.line(Line::new(
                        format!("ui: {:.2}ms", self.avg_times.ui),
                        charts
                            .iter()
                            .enumerate()
                            .map(|(i, &v)| [i as f64, v.ui])
                            .collect::<Vec<_>>(),
                    ));
                    plot_ui.line(Line::new(
                        format!("prepare: {:.2}ms", self.avg_times.prepare),
                        charts
                            .iter()
                            .enumerate()
                            .map(|(i, &v)| [i as f64, v.prepare])
                            .collect::<Vec<_>>(),
                    ));
                    plot_ui.line(Line::new(
                        format!("acquire: {:.2}ms", self.avg_times.acquire),
                        charts
                            .iter()
                            .enumerate()
                            .map(|(i, &v)| [i as f64, v.acquire])
                            .collect::<Vec<_>>(),
                    ));
                    plot_ui.line(Line::new(
                        format!("pass: {:.2}ms", self.avg_times.pass),
                        charts
                            .iter()
                            .enumerate()
                            .map(|(i, &v)| [i as f64, v.pass])
                            .collect::<Vec<_>>(),
                    ));
                    plot_ui.line(Line::new(
                        format!("egui: {:.2}ms", self.avg_times.egui),
                        charts
                            .iter()
                            .enumerate()
                            .map(|(i, &v)| [i as f64, v.egui])
                            .collect::<Vec<_>>(),
                    ));
                    plot_ui.line(Line::new(
                        format!("submit: {:.2}ms", self.avg_times.submit),
                        charts
                            .iter()
                            .enumerate()
                            .map(|(i, &v)| [i as f64, v.submit])
                            .collect::<Vec<_>>(),
                    ));
                });
        }
    }
}
