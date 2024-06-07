use bevy_ecs::prelude::*;
use modula_asset::{AssetFetcher, AssetId, Assets};
use modula_core::DeviceRes;
use modula_utils::HashSet;
use wgpu::{CommandEncoder, CommandEncoderDescriptor, Device};

use crate::RenderTarget;

pub trait OperationBuilder: Send + Sync {
    /// used by the sequence to determine when to resolve
    fn reading(&self) -> Vec<AssetId<RenderTarget>>;
    /// used by the sequence to determine when to resolve
    fn writing(&self) -> Vec<AssetId<RenderTarget>>;
    /// should only be called once, does not consume self because it needs to be stored as dyn
    fn finish(&mut self, device: &Device) -> Box<dyn Operation>;
}

pub trait Operation: Send + Sync {
    fn run(&mut self, asset_fetcher: &AssetFetcher, command_encoder: &mut CommandEncoder);
}

pub struct Sequence {
    // to not have Sequence publicly be a enum
    inner: InnerSequence,
}

impl Sequence {
    fn run(&mut self, command_encoder: &mut CommandEncoder, world: &mut World) {
        if let InnerSequence::UnInitialized(builders) = &mut self.inner {
            let device = &world.resource::<DeviceRes>().0;
            let mut operations = Vec::new();
            let mut needs_resolving = HashSet::<AssetId<RenderTarget>>::new();
            for builder in builders {
                for reading in builder.reading() {
                    if needs_resolving.contains(&reading) {
                        needs_resolving.remove(&reading);
                        operations.push(SequenceOperation::ResolveNext(reading));
                    }
                }
                for writing in builder.writing() {
                    needs_resolving.insert(writing);
                }
                operations.push(SequenceOperation::Run(builder.finish(device)));
            }
            for resolve in needs_resolving {
                operations.push(SequenceOperation::ResolveNext(resolve));
            }
            self.inner = InnerSequence::Ready(operations);
        }
        // should always be true, not using match as this will run after the other if let
        if let InnerSequence::Ready(ops) = &mut self.inner {
            for op in ops.iter_mut() {
                match op {
                    SequenceOperation::ResolveNext(target) => {
                        let mut resource_mut = world.resource_mut::<Assets<RenderTarget>>();
                        resource_mut
                            .get_mut(*target)
                            .expect("target to resolve was not found")
                            .schedule_resolve();
                    }
                    SequenceOperation::Run(op) => {
                        op.run(&AssetFetcher::new(world), command_encoder);
                    }
                }
            }
        }
    }
}

pub struct SequenceBuilder {
    operation_builders: Vec<Box<dyn OperationBuilder>>,
}

impl SequenceBuilder {
    pub fn new() -> SequenceBuilder {
        return SequenceBuilder {
            operation_builders: vec![],
        };
    }

    pub fn add(&mut self, operation_builder: impl OperationBuilder + 'static) {
        self.operation_builders.push(Box::new(operation_builder));
    }

    pub fn finish(self, assets: &mut Assets<Sequence>) -> AssetId<Sequence> {
        return assets.add(Sequence {
            inner: InnerSequence::UnInitialized(self.operation_builders),
        });
    }
}

pub enum SequenceOperation {
    Run(Box<dyn Operation>),
    ResolveNext(AssetId<RenderTarget>),
}

#[derive(Resource)]
pub struct SequenceQueue(Vec<AssetId<Sequence>>);

enum InnerSequence {
    Ready(Vec<SequenceOperation>),
    UnInitialized(Vec<Box<dyn OperationBuilder>>),
}

pub(crate) fn run_sequences(world: &mut World) {
    world.resource_scope(|world, mut sequence_assets: Mut<Assets<Sequence>>| {
        world.resource_scope(|world, sequence_queue: Mut<SequenceQueue>| {
            // FIXME maybe use multiple command encoders and run in parallel??
            let mut command_encoder =
                world
                    .resource::<DeviceRes>()
                    .0
                    .create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("Sequence runner encoder"),
                    });
            for asset_id in &sequence_queue.0 {
                sequence_assets
                    .get_mut(*asset_id)
                    .expect("sequence was added to queue, but does not exist")
                    .run(&mut command_encoder, world)
            }
            world.resource_mut::<SequenceQueue>().0.clear();
        });
    });
}
