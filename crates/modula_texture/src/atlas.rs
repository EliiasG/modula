use modula_asset::AssetId;
use wgpu::Texture;

pub struct Atlas {
    pub texture: AssetId<Texture>,
    pub sub_textures: Vec<SubTexture>,
}

/// A subsection of a texture atlas.
pub struct SubTexture {
    pub layer: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
