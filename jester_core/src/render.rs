use crate::sprite::SpriteBatch;
use winit::window::Window;

pub struct Renderer<B: Backend> {
    backend: B,
}

impl<B: Backend> Renderer<B> {
    pub fn new(app_name: &str, window: &Window) -> Result<Self, B::Error> {
        assert!(!app_name.is_empty());
        let backend = B::init(app_name, window)?;
        Ok(Self { backend })
    }

    pub fn begin_frame(&mut self) {
        self.backend.begin_frame()
    }
    pub fn end_frame(&mut self) {
        self.backend.end_frame()
    }
    pub fn handle_resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.backend.handle_resize(size)
    }
    pub fn draw_sprites(&mut self, batch: &SpriteBatch) {
        self.backend.draw_sprites(batch)
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }
}

pub trait Backend: Sized {
    type Error;

    fn init(app_name: &str, window: &Window) -> std::result::Result<Self, Self::Error>;

    fn begin_frame(&mut self);
    fn draw_sprites(&mut self, batch: &SpriteBatch);
    fn end_frame(&mut self);
    fn handle_resize(&mut self, _size: winit::dpi::PhysicalSize<u32>) {}
}
