#[derive(Debug, Clone)]
pub struct Time {
    initial_time: f64,
    elapsed_time: f64,
    delta_time: f64,
}

impl Time {
    pub fn new(initial_time: f64) -> Self {
        Self {
            initial_time,
            elapsed_time: 0.0,
            delta_time: 0.0,
        }
    }
    pub fn update(&mut self, current_time: f64) {
        self.delta_time = current_time - self.elapsed_time;
        self.elapsed_time = current_time - self.initial_time;
    }

    pub fn delta(&self) -> f64 {
        self.delta_time
    }

    pub fn elapsed(&self) -> f64 {
        self.elapsed_time
    }
}
