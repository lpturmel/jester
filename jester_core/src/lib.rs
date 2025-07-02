pub use error::Error;
pub use input::InputState;
pub use render::{constants::*, Backend, Renderer};
pub use scene::{Commands, Ctx, EntityId, EntityPool, Resources, Scene, SceneKey};
pub use sprite::{Sprite, SpriteBatch, SpriteInstance, TextureId};

mod error;
mod input;
mod render;
mod scene;
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
