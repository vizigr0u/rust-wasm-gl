use std::{iter::Sum, ops::Div};

use egui::{Button, Color32, RichText};
use egui_plot::{Legend, Line, Plot};

const CHART_SIZE: usize = 40;
const AVG_SIZE: usize = 3;
const BUFFER_SIZE: usize = CHART_SIZE * AVG_SIZE;

#[derive(Debug, Default, Clone, Copy)]
struct PerformanceTime {
    frame_time: f64,
    update_time: f64,
    render_time: f64,
}

impl PerformanceTime {
    pub fn new(start_time: f64, frame_time: f64, update_time: f64, render_time: f64) -> Self {
        Self {
            frame_time: frame_time - start_time,
            update_time: update_time - start_time,
            render_time: render_time - update_time,
        }
    }
    pub fn get_ui_time(&self) -> f64 {
        self.frame_time - self.update_time - self.render_time
    }
}

impl Sum<PerformanceTime> for PerformanceTime {
    fn sum<I: Iterator<Item = PerformanceTime>>(iter: I) -> Self {
        iter.fold(PerformanceTime::default(), |a, b| PerformanceTime {
            frame_time: a.frame_time + b.frame_time,
            update_time: a.update_time + b.update_time,
            render_time: a.render_time + b.render_time,
        })
    }
}

impl<'a> Sum<&'a PerformanceTime> for PerformanceTime {
    fn sum<I: Iterator<Item = &'a PerformanceTime>>(iter: I) -> Self {
        iter.fold(PerformanceTime::default(), |a, b| PerformanceTime {
            frame_time: a.frame_time + b.frame_time,
            update_time: a.update_time + b.update_time,
            render_time: a.render_time + b.render_time,
        })
    }
}

impl Div<f64> for PerformanceTime {
    type Output = PerformanceTime;
    fn div(self, rhs: f64) -> Self::Output {
        PerformanceTime {
            frame_time: self.frame_time / rhs,
            update_time: self.update_time / rhs,
            render_time: self.render_time / rhs,
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
}

impl FpsDisplay {
    pub fn new() -> Self {
        Self {
            tick_times: [PerformanceTime::default(); BUFFER_SIZE],
            avg_times: PerformanceTime::default(),
            tick_index: 0,
            show_details: false,
            buffer_was_filled: false,
        }
    }

    pub fn update_fps_report(
        &mut self,
        start_time: f64,
        update_time: f64,
        render_time: f64,
        render_ui_time: f64,
        frame_time: f64,
    ) {
        self.tick_times[self.tick_index] =
            PerformanceTime::new(start_time, frame_time, update_time, render_time);
        if !self.buffer_was_filled && self.tick_index == BUFFER_SIZE - 1 {
            self.buffer_was_filled = true;
        }
        self.tick_index = (self.tick_index + 1) % self.tick_times.len();
        if !self.buffer_was_filled {
            self.avg_times =
                self.tick_times.iter().sum::<PerformanceTime>() / self.tick_index as f64;
        } else if self.tick_index == 0 {
            self.avg_times =
                self.tick_times.iter().sum::<PerformanceTime>() / self.tick_times.len() as f64;
        }
    }
    pub fn get_debug_info(&self) -> String {
        if self.avg_times.frame_time == 0.0 {
            return "???".to_string();
        }
        format!("{:.1}", 1000.0 / self.avg_times.frame_time)
    }

    pub fn display(&mut self, ui: &mut egui::Ui) {
        let button = Button::new(
            RichText::new(format!("FPS: {}", self.get_debug_info())).color(Color32::WHITE),
        );
        if ui.add(button).clicked() {
            self.show_details = !self.show_details;
        }
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
                                .map(|j| self.tick_times[(i + j + self.tick_index) % BUFFER_SIZE])
                                .sum::<PerformanceTime>()
                                / AVG_SIZE as f64;
                            avg
                        })
                        .collect::<Vec<_>>();
                    plot_ui.line(
                        Line::new(
                            charts
                                .iter()
                                .enumerate()
                                .map(|(i, &v)| [i as f64, v.update_time])
                                .collect::<Vec<_>>(),
                        )
                        .name(format!("script: {:.2}ms", self.avg_times.update_time)),
                    );
                    plot_ui.line(
                        Line::new(
                            charts
                                .iter()
                                .enumerate()
                                .map(|(i, &v)| [i as f64, v.render_time])
                                .collect::<Vec<_>>(),
                        )
                        .name(format!("render: {:.2}ms", self.avg_times.render_time)),
                    );
                    plot_ui.line(
                        Line::new(
                            charts
                                .iter()
                                .enumerate()
                                .map(|(i, &v)| [i as f64, v.get_ui_time()])
                                .collect::<Vec<_>>(),
                        )
                        .name(format!("UI: {:.2}ms", self.avg_times.get_ui_time())),
                    );
                });
        }
    }
}
