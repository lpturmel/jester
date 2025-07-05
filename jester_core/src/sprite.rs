use glam::Vec2;

use crate::Transform;
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    ops::Deref,
    path::Path,
};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureId(pub u64);

impl TextureId {
    pub fn from_path<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        let p = path.as_ref();
        let mut h = DefaultHasher::new();
        p.hash(&mut h);
        Self(h.finish())
    }
}

impl Deref for TextureId {
    type Target = u64;

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

#[derive(Debug, Clone, Copy)]
pub struct Sprite {
    pub transform: Transform,
    pub size: Option<Vec2>,
    pub uv: [f32; 4],
    pub tex: TextureId,
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            size: None,
            transform: Transform::default(),
            uv: [0.0, 0.0, 1.0, 1.0],
            tex: TextureId(0),
        }
    }
}
