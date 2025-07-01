pub use error::Error;
pub use render::{Backend, Renderer, MAX_SPRITES, MAX_TEXTURES};
pub use sprite::{SpriteBatch, SpriteInstance, TextureId};

mod error;
mod render;
mod sprite;
