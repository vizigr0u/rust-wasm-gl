#[derive(Debug, Clone, Default)]
pub struct Time {
    previous_time: Option<f64>,
    delta_time: f64,
}

impl Time {
    pub fn update(&mut self, time: f64) {
        if let Some(previous_time) = self.previous_time {
            self.delta_time = previous_time - time;
        }
        self.previous_time = Some(time);
    }

    pub fn delta_time(&self) -> f64 {
        self.delta_time
    }

    pub fn time(&self) -> f64 {
        self.previous_time.unwrap_or(0.0)
    }
}
