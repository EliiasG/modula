use bevy_ecs::prelude::*;
use modula_asset::{AssetId, AssetWorldExt};

use crate::{Operation, OperationBuilder};

pub struct ClearNext {
    pub render_target: AssetId<crate::RenderTarget>,
}

impl Operation for ClearNext {
    fn run(&mut self, world: &mut World, _command_encoder: &mut wgpu::CommandEncoder) {
        world.with_asset(self.render_target, |render_target| {
            render_target.schedule_clear_color();
        });
    }
}

impl OperationBuilder for ClearNext {
    // not reading or writing, as the render target only written to when creating a pass
    fn reading(&self) -> Vec<AssetId<crate::RenderTarget>> {
        Vec::new()
    }

    fn writing(&self) -> Vec<AssetId<crate::RenderTarget>> {
        Vec::new()
    }

    fn finish(self, _device: &wgpu::Device) -> impl Operation + 'static {
        self
    }
}

pub struct EmptyPass {
    pub render_target: AssetId<crate::RenderTarget>,
}

impl Operation for EmptyPass {
    fn run(&mut self, world: &mut World, command_encoder: &mut wgpu::CommandEncoder) {
        world.with_asset(self.render_target, |render_target| {
            render_target.begin_pass(command_encoder);
        });
    }
}

impl OperationBuilder for EmptyPass {
    fn reading(&self) -> Vec<AssetId<crate::RenderTarget>> {
        Vec::new()
    }

    fn writing(&self) -> Vec<AssetId<crate::RenderTarget>> {
        vec![self.render_target]
    }

    fn finish(self, _device: &wgpu::Device) -> impl Operation + 'static {
        self
    }
}
