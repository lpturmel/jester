use crate::{
    sprite::{SpriteBatch, TextureId},
    Camera,
};
use hashbrown::HashMap;
use image::ImageResult;
use winit::window::Window;

pub mod constants {
    pub const MAX_SPRITES: usize = 10000;
    pub const MAX_TEXTURES: usize = 256;
    pub const VERTEX_COUNT: usize = 4;
}

#[derive(Debug, Clone, Copy)]
pub struct TextureMeta {
    pub w: u32,
    pub h: u32,
}

pub struct Renderer<B: Backend> {
    backend: B,
    metadata: Vec<Option<TextureMeta>>,
    lut: HashMap<TextureId, usize>,
}

impl<B: Backend> Renderer<B> {
    pub fn new(app_name: &str, window: &Window) -> Result<Self, B::Error> {
        assert!(!app_name.is_empty());
        let backend = B::init(app_name, window)?;
        Ok(Self {
            backend,
            metadata: Vec::new(),
            lut: HashMap::new(),
        })
    }

    pub fn begin_frame(&mut self) {
        self.backend.begin_frame()
    }
    pub fn end_frame(&mut self) {
        self.backend.end_frame()
    }
    pub fn bind_camera(&mut self, camera: &Camera) {
        self.backend.bind_camera(camera)
    }
    pub fn handle_resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.backend.handle_resize(size)
    }
    pub fn draw_sprites(&mut self, batch: &SpriteBatch) {
        let Some(idx) = self.lut.get(&batch.tex).copied() else {
            return;
        };
        self.backend.draw_sprites(idx, batch)
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }
    pub fn texture_meta(&self, tex: TextureId) -> Option<TextureMeta> {
        self.metadata.get(tex.0 as usize).and_then(|m| *m)
    }

    pub fn load_texture_sync<P>(&mut self, tex_id: TextureId, path: P) -> ImageResult<()>
    where
        P: AsRef<std::path::Path>,
    {
        let img = image::open(path)?.to_rgba8();
        let (w, h) = img.dimensions();
        let slot = self
            .backend
            .create_texture(w, h, &img)
            .expect("Failed to create texture");

        self.lut.insert(tex_id, slot);

        if slot >= self.metadata.len() {
            self.metadata.resize(slot + 1, None);
        }
        self.metadata[slot] = Some(TextureMeta { w, h });
        Ok(())
    }
}

pub trait Backend: Sized {
    type Error: std::error::Error;

    fn init(app_name: &str, window: &Window) -> std::result::Result<Self, Self::Error>;

    fn begin_frame(&mut self);
    fn draw_sprites(&mut self, tex_idx: usize, batch: &SpriteBatch);
    fn end_frame(&mut self);
    fn handle_resize(&mut self, _size: winit::dpi::PhysicalSize<u32>) {}
    fn bind_camera(&mut self, camera: &Camera);

    fn create_texture(
        &mut self,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Result<usize, Self::Error>;
}
