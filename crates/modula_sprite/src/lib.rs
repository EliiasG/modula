/* old code todo fix
use bevy_ecs::world::World;
use modula_asset::{AssetId, AssetWorldExt};
use modula_render::{Operation, RenderTarget};
use modula_texture::atlas::Atlas;
use wgpu::{BindGroup, Buffer, CommandEncoder, RenderPipeline};

pub struct SpriteOperation {
    target: AssetId<RenderTarget>,
    queue: AssetId<SpriteQueue>,
}

impl Operation for SpriteOperation {
    fn run(&mut self, world: &mut World, command_encoder: &mut CommandEncoder) {
        world.asset_scope(self.target, |world, target| {
            let mut pass = target.begin_pass(command_encoder);
            let queue = world
                .get_asset(self.queue)
                .expect("no queue for sprite operation");
            for batch in queue.batches.iter() {
                pass.set_pipeline(
                    &world
                        .get_asset(batch.pipeline)
                        .expect("no pipeline for sprite batch"),
                );
                pass.set_vertex_buffer(
                    0,
                    world
                        .get_asset(batch.buffer)
                        .expect("buffer was not avalibe")
                        .slice(batch.start..(batch.start + batch.count as u64 * batch.size)),
                );
                let atlas = world
                    .get_asset(batch.atlas)
                    .expect("no atlas for sprite batch");
                pass.set_bind_group(0, atlas.bind_group(), &[]);
                for (i, group) in queue.bind_groups.iter().enumerate() {
                    pass.set_bind_group(
                        i as u32 + 1,
                        world.get_asset(*group).expect("bind group was missing"),
                        &[],
                    );
                }
                pass.draw(0..6, 0..batch.count);
            }
        })
    }
}

/// a type that holds information about what sprites to draw, and their order
pub struct SpriteQueue {
    // Starting at 1, as group 0 is from atlas
    bind_groups: Vec<AssetId<BindGroup>>,
    batches: Vec<SpriteBatch>,
}

pub struct SpriteBatch {
    atlas: AssetId<Atlas>,
    pipeline: AssetId<RenderPipeline>,
    buffer: AssetId<Buffer>,
    start: u64,
    size: u64,
    count: u32,
}
 */
