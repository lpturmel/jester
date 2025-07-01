#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SpriteInstance {
    pub pos_size: [f32; 4],
    pub uv: [f32; 4],
    pub tex_id: u32,
    pub _pad: u32,
}

unsafe impl bytemuck::Pod for SpriteInstance {}
unsafe impl bytemuck::Zeroable for SpriteInstance {}

#[derive(Debug, Default)]
pub struct SpriteBatch {
    pub instances: Vec<SpriteInstance>,
}
