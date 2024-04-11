use super::Renderer;

pub trait EguiRenderer<R>
where
    R: Renderer,
{
    fn render(&mut self, renderer: &mut R, egui_output: egui::FullOutput);
}
