use std::iter;

use bevy_ecs::prelude::*;
use modula_asset::{init_assets, AssetId, Assets};
use modula_core::{DeviceRes, PreInit, QueueRes, ScheduleBuilder};
use modula_utils::HashSet;
use wgpu::{CommandEncoder, CommandEncoderDescriptor, Device};

use crate::RenderTarget;
mod basic;
pub use basic::*;

pub trait OperationBuilder: Send + Sync + 'static {
    /// used by the sequence to determine when to resolve
    fn reading(&self) -> Vec<AssetId<RenderTarget>>;
    /// used by the sequence to determine when to resolve
    fn writing(&self) -> Vec<AssetId<RenderTarget>>;
    /// should only be called once, does not consume self because it needs to be stored as dyn
    fn finish(self, device: &Device) -> impl Operation + 'static;
}

pub trait Operation: Send + Sync {
    fn run(&mut self, world: &mut World, command_encoder: &mut CommandEncoder);
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
                        op.run(world, command_encoder);
                    }
                }
            }
        }
    }
}

pub struct SequenceBuilder {
    operation_builders: Vec<Box<dyn DynOperationBuilder>>,
}

impl SequenceBuilder {
    pub fn new() -> SequenceBuilder {
        return SequenceBuilder {
            operation_builders: vec![],
        };
    }

    pub fn add(mut self, operation_builder: impl OperationBuilder) -> Self {
        self.operation_builders
            .push(Box::new(DynOperationBuilderImpl(Some(Box::new(
                operation_builder,
            )))));
        self
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

impl SequenceQueue {
    pub fn schedule(&mut self, sequence: AssetId<Sequence>) {
        self.0.push(sequence);
    }
}

// to get around dyn not being able to consume self
// maybe there is a better way to do this
trait DynOperationBuilder: Send + Sync + 'static {
    fn reading(&self) -> Vec<AssetId<RenderTarget>>;
    fn writing(&self) -> Vec<AssetId<RenderTarget>>;
    fn finish(&mut self, device: &Device) -> Box<dyn Operation>;
}

struct DynOperationBuilderImpl<T: OperationBuilder>(Option<Box<T>>);

impl<T: OperationBuilder> DynOperationBuilder for DynOperationBuilderImpl<T> {
    fn reading(&self) -> Vec<AssetId<RenderTarget>> {
        self.0.as_ref().unwrap().reading()
    }

    fn writing(&self) -> Vec<AssetId<RenderTarget>> {
        self.0.as_ref().unwrap().writing()
    }

    fn finish(&mut self, device: &Device) -> Box<dyn Operation> {
        Box::new(self.0.take().unwrap().finish(device))
    }
}
enum InnerSequence {
    Ready(Vec<SequenceOperation>),
    UnInitialized(Vec<Box<dyn DynOperationBuilder>>),
}

pub(crate) fn run_sequences(world: &mut World) {
    world.resource_scope(|world, mut sequence_assets: Mut<Assets<Sequence>>| {
        world.resource_scope(|world, mut sequence_queue: Mut<SequenceQueue>| {
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
            sequence_queue.0.clear();
            world
                .resource::<QueueRes>()
                .0
                .submit(iter::once(command_encoder.finish()));
        });
    });
}

pub(crate) fn init_sequences(schedule_builder: &mut ScheduleBuilder) {
    schedule_builder.add_systems(PreInit, |mut commands: Commands| {
        commands.insert_resource(SequenceQueue(Vec::new()));
    });
    init_assets::<Sequence>(schedule_builder);
}
