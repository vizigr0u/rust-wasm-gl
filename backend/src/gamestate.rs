use egui::FullOutput;

pub struct GameState {
    pub mouse_pos: (f64, f64),
    pub gui_state: Option<FullOutput>,
}
