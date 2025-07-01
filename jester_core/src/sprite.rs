use std::ops::Deref;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureId(pub u32);

impl Deref for TextureId {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

unsafe impl bytemuck::Pod for TextureId {}
unsafe impl bytemuck::Zeroable for TextureId {}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SpriteInstance {
    pub pos_size: [f32; 4],
    pub uv: [f32; 4],
}

unsafe impl bytemuck::Pod for SpriteInstance {}
unsafe impl bytemuck::Zeroable for SpriteInstance {}

#[derive(Debug)]
pub struct SpriteBatch {
    pub tex: TextureId,
    pub instances: Vec<SpriteInstance>,
}
