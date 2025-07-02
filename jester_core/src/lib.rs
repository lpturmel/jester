pub use error::Error;
pub use render::{constants::*, Backend, Renderer};
pub use sprite::{SpriteBatch, SpriteInstance, TextureId};

mod error;
mod render;
mod sprite;

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub center: glam::Vec2,
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            center: glam::Vec2::ZERO,
            zoom: 1.0,
        }
    }
}
