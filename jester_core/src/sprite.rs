#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureId(u32);

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sprite {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub uv: [f32; 4],
    pub texture: TextureId,
}

#[derive(Default)]
pub struct SpriteBatch {
    sprites: Vec<Sprite>,
}

impl SpriteBatch {
    /// Create an empty batch with a pre-allocated capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            sprites: Vec::with_capacity(cap),
        }
    }

    /// Add a sprite to the batch.
    pub fn push(&mut self, sprite: Sprite) {
        self.sprites.push(sprite);
    }

    /// Remove all sprites but keep the allocation.
    pub fn clear(&mut self) {
        self.sprites.clear();
    }

    /// How many sprites are queued?
    pub fn len(&self) -> usize {
        self.sprites.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sprites.is_empty()
    }

    /// Raw access for back-ends that stream-map the whole slice.
    pub fn as_slice(&self) -> &[Sprite] {
        &self.sprites
    }

    /// Mutable access if a back-end wants to write in-place.
    pub fn as_mut_slice(&mut self) -> &mut [Sprite] {
        &mut self.sprites
    }

    /// Iterator helpers for ergonomic for-loops.
    pub fn iter(&self) -> impl Iterator<Item = &Sprite> {
        self.sprites.iter()
    }
}
