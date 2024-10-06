use std::marker::PhantomData;

use wgpu::{BindGroup, BindGroupLayout, ShaderSource};

/// Provides bind group layouts to [AtlasShaders](AtlasShader), this exists to make bind groups more abstract
pub trait BindGroupLayoutProvider {
    fn layouts(&self) -> &[&BindGroupLayout];
}

/// Provides bind groups to [AtlasRenderers](AtlasRenderer), this exists to make bind groups more abstract
pub trait BindGroupProvider {
    fn bind_groups(&self) -> &[&BindGroup];
}

pub struct AtlasShader<Layout: BindGroupLayoutProvider> {
    _layout: PhantomData<Layout>,
    layouts: Vec<BindGroupLayout>,
}

impl<Layout: BindGroupLayoutProvider> AtlasShader<Layout> {
    pub fn new(source: ShaderSource) {}
}
