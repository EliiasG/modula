use modula_asset::AssetId;
use wgpu::{BindGroup, Sampler, TextureView};

pub struct Atlas {
    bind_group: BindGroup,
    sprite_count: u32,
    size: (u32, u32, u32),
}

impl Atlas {
    pub fn bind_group(&self) -> &BindGroup {
        todo!()
    }

    pub fn sprite_count(&self) -> u32 {
        todo!()
    }

    pub fn size(&self) -> (u32, u32, u32) {
        todo!()
    }
}

/// A subsection of a texture atlas.
pub struct SubTexture {
    pub layer: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
