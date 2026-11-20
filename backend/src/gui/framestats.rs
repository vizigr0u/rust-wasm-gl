use std::{iter::Sum, ops::Div};

const CHART_SIZE: usize = 120;
const AVG_SIZE: usize = 3;
const BUFFER_SIZE: usize = CHART_SIZE * AVG_SIZE;
const REFRESH_EVERY: usize = 10;
const SERIES_COUNT: usize = 7;

/// Milliseconds spent in each part of one frame (all measured with `performance.now()`).
#[derive(Debug, Default, Clone, Copy)]
pub struct FrameTimes {
    /// Inputs, world update, camera (without chunk uploads).
    pub script: f64,
    /// `upload_chunk` / `remove_chunk` calls.
    pub upload: f64,
    /// Rust cost of updating the DOM overlay.
    pub ui: f64,
    /// Renderer CPU prep (draw list, per-frame uploads).
    pub prepare: f64,
    /// Surface acquire + view (webgpu).
    pub acquire: f64,
    /// Main 3D pass encode.
    pub pass: f64,
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
            submit: self.submit / rhs,
            frame: self.frame / rhs,
            interval: self.interval / rhs,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FrameStats {
    tick_times: [PerformanceTime; BUFFER_SIZE],
    avg_times: PerformanceTime,
    tick_index: usize,
    buffer_was_filled: bool,
    last_start: Option<f64>,
}

impl Default for FrameStats {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameStats {
    /// Graph series names, in the order `fill_chart` / `averages` write them.
    pub const SERIES: [&'static str; SERIES_COUNT] = [
        "script", "upload", "ui", "prepare", "acquire", "pass", "submit",
    ];

    pub fn new() -> Self {
        Self {
            tick_times: [PerformanceTime::default(); BUFFER_SIZE],
            avg_times: PerformanceTime::default(),
            tick_index: 0,
            buffer_was_filled: false,
            last_start: None,
        }
    }

    /// `start_time`: `performance.now()` at the start of the tick; `times.interval` is filled here.
    /// Returns true when the averages were refreshed (every `REFRESH_EVERY` frames).
    pub fn update_fps_report(&mut self, start_time: f64, mut times: FrameTimes) -> bool {
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
            return true;
        }
        false
    }

    pub fn get_debug_info(&self) -> String {
        if self.avg_times.interval <= 0.0 {
            return "???".to_string();
        }
        let t = self.avg_times;
        // fps = real frame rate; max = theoretical rate if the CPU work were the only limit
        format!(
            "{:.0} (actual {:.1}) | ui {:.2}ms, render {:.2}ms",
            1000.0 / t.frame.max(0.001),
            1000.0 / t.interval,
            t.ui,
            t.prepare + t.acquire + t.pass + t.submit
        )
    }

    /// Per-series averages, same order as `SERIES`.
    pub fn averages(&self) -> [f64; SERIES_COUNT] {
        Self::series_values(&self.avg_times)
    }

    /// Chart data, series-major (`SERIES_COUNT` x `CHART_POINTS`), oldest point first.
    /// Each point is the mean of `AVG_SIZE` consecutive frames.
    pub fn fill_chart(&self, out: &mut Vec<f64>) {
        out.clear();
        out.resize(SERIES_COUNT * CHART_SIZE, 0.0);
        for i in 0..CHART_SIZE {
            let avg: PerformanceTime = (0..AVG_SIZE)
                .map(|j| self.tick_times[(i * AVG_SIZE + j + self.tick_index) % BUFFER_SIZE])
                .sum::<PerformanceTime>()
                / AVG_SIZE as f64;
            for (s, v) in Self::series_values(&avg).into_iter().enumerate() {
                out[s * CHART_SIZE + i] = v;
            }
        }
    }

    fn series_values(t: &PerformanceTime) -> [f64; SERIES_COUNT] {
        [
            t.script, t.upload, t.ui, t.prepare, t.acquire, t.pass, t.submit,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_chart_is_series_major_oldest_first() {
        let mut fs = FrameStats::new();
        for n in 0..BUFFER_SIZE {
            let t = FrameTimes {
                script: n as f64,
                submit: 100.0 + n as f64,
                ..Default::default()
            };
            fs.update_fps_report(n as f64, t);
        }
        let mut out = Vec::new();
        fs.fill_chart(&mut out);
        assert_eq!(out.len(), SERIES_COUNT * CHART_SIZE);
        // script series (0): oldest point = mean of frames 0..3 = 1, newest = mean of the last AVG_SIZE frames
        assert_eq!(out[0], 1.0);
        assert_eq!(out[CHART_SIZE - 1], (BUFFER_SIZE - 2) as f64);
        // submit series (6)
        assert_eq!(out[6 * CHART_SIZE], 101.0);
        // ui series untouched
        assert!(out[2 * CHART_SIZE..3 * CHART_SIZE]
            .iter()
            .all(|&v| v == 0.0));
    }

    #[test]
    fn refresh_flag_every_ten_frames() {
        let mut fs = FrameStats::new();
        let flags: Vec<bool> = (0..20)
            .map(|n| fs.update_fps_report(n as f64, FrameTimes::default()))
            .collect();
        assert_eq!(flags.iter().filter(|&&f| f).count(), 2);
        assert!(flags[9] && flags[19]);
    }
}
