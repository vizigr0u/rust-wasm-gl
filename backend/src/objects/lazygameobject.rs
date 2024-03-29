use std::rc::Rc;

use glow::HasContext;
use log::info;

use crate::{
    core::Time,
    graphics::{Camera, MeshRenderer},
    utils::GlState,
};

use super::GameObject;

pub trait MakeRenderer {
    fn make_renderer(&self, gl: &glow::Context) -> Result<Rc<MeshRenderer>, String>;
    fn init_gameobject(&self, _gameobject: &mut GameObject) {}
}

#[derive(Debug)]
pub struct LazyRenderGameObject<R>
where
    R: MakeRenderer,
{
    pub gameobject: Option<GameObject>,
    pub renderer_creator: R,
}

impl<R> LazyRenderGameObject<R>
where
    R: MakeRenderer,
{
    pub fn get_gameobject(&self) -> Option<&GameObject> {
        self.gameobject.as_ref()
    }

    pub fn render_lazy(&mut self, gl: &glow::Context, camera: &Camera) {
        if self.gameobject.is_none() {
            self.load(gl);
        }
        self.gameobject.as_ref().unwrap().render(gl, camera);
    }

    pub fn load(&mut self, gl: &glow::Context) {
        if self.gameobject.is_some() {
            return;
        }
        let renderer = self.renderer_creator.make_renderer(gl).unwrap();
        let mut gameobject = GameObject::new(&Rc::new(Default::default()), &renderer);
        self.renderer_creator.init_gameobject(&mut gameobject);
        self.gameobject = Some(gameobject);
    }

    pub fn update(&mut self, time: &Time) {
        if let Some(go) = self.gameobject.as_mut() {
            go.update(time);
        }
    }
}
