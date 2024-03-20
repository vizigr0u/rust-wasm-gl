use std::rc::Rc;

use glow::HasContext;
use log::info;

use crate::{gameobject::GameObject, meshrenderer::MeshRenderer, utils::GlState};

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

impl<R> AsRef<GameObject> for LazyRenderGameObject<R>
where
    R: MakeRenderer,
{
    fn as_ref(&self) -> &GameObject {
        &self
            .gameobject
            .as_ref()
            .expect("Cant get gameobject - call render_lazy or load first")
    }
}

impl<R> LazyRenderGameObject<R>
where
    R: MakeRenderer,
{
    pub fn render_lazy(&mut self, gl: &glow::Context, camera: &crate::camera::Camera) {
        if self.gameobject.is_none() {
            self.load(gl);
            info!("Loaded gizmo");
        }
        let state = GlState::save(gl);
        unsafe {
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::CULL_FACE);
        }
        self.gameobject.as_ref().unwrap().render(gl, camera);
        state.restore(gl);
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

    pub fn update(&mut self) {
        if let Some(go) = self.gameobject.as_mut() {
            go.update();
        }
    }
}
