pub use error::Error;
use glam::Vec2;
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

impl Camera {
    pub fn pixel_perfect(w: f32, h: f32) -> Self {
        Self {
            center: Vec2::new(-w * 0.5, -h * 0.5),
            zoom: 1.0,
        }
    }

    pub fn update_pixel_perfect(&mut self, new_w: f32, new_h: f32) {
        self.center = Vec2::new(-new_w * 0.5, -new_h * 0.5);
    }
    pub fn world_to_screen(&self, world: Vec2, screen: Vec2) -> Vec2 {
        (world - self.center) * self.zoom + screen * 0.5
    }
    pub fn screen_to_world(&self, screen_pt: Vec2, screen: Vec2) -> Vec2 {
        (screen_pt - screen * 0.5) / self.zoom + self.center
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            center: glam::Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub translation: Vec2,
    pub scale: Vec2,
    pub rotation: f32, // currently unused
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec2::ZERO,
            scale: Vec2::ONE,
            rotation: 0.0,
        }
    }
}

impl Transform {
    pub fn from_xy(x: f32, y: f32) -> Self {
        Self {
            translation: Vec2::new(x, y),
            scale: Vec2::ONE,
            rotation: 0.0,
        }
    }
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self {
            translation: Vec2::new(x, y),
            scale: Vec2::ONE,
            rotation: z,
        }
    }
    pub fn with_size(mut self, w: f32, h: f32) -> Self {
        self.scale = Vec2::new(w, h);
        self
    }
    pub fn with_rotation(mut self, angle: f32) -> Self {
        self.rotation = angle;
        self
    }
}

impl From<Transform> for [f32; 4] {
    fn from(v: Transform) -> Self {
        [v.translation.x, v.translation.y, v.scale.x, v.scale.y]
    }
}
