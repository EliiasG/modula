use bevy_ecs::prelude::*;
use modula_asset::{AssetFetcher, AssetId, Assets};
use modula_core::DeviceRes;
use wgpu::{CommandEncoder, CommandEncoderDescriptor, Device};

pub trait OperationBuilder: Send + Sync {
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
    fn run(
        &mut self,
        asset_fetcher: &AssetFetcher,
        command_encoder: &mut CommandEncoder,
        device: &Device,
    ) {
        if let InnerSequence::UnInitialized(builders) = &mut self.inner {
            self.inner = InnerSequence::Ready(
                builders
                    .iter_mut()
                    .map(|builder| builder.finish(device))
                    .collect(),
            );
        }
        // should always be true, not using match as this will run after the other if let
        if let InnerSequence::Ready(ops) = &mut self.inner {
            for op in ops.iter_mut() {
                op.run(asset_fetcher, command_encoder);
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

#[derive(Resource)]
pub struct SequenceQueue(Vec<AssetId<Sequence>>);

enum InnerSequence {
    Ready(Vec<Box<dyn Operation>>),
    UnInitialized(Vec<Box<dyn OperationBuilder>>),
}

pub(crate) fn run_sequences(world: &mut World) {
    world.resource_scope(|world, mut sequence_assets: Mut<Assets<Sequence>>| {
        let device = &world.resource::<DeviceRes>().0;
        // FIXME maybe use multiple command encoders and run in parallel??
        let mut command_encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Sequence runner encoder"),
        });
        for asset_id in &world.resource::<SequenceQueue>().0 {
            sequence_assets
                .get_mut(*asset_id)
                .expect("sequence was added to queue, but does not exist")
                .run(&AssetFetcher::new(&world), &mut command_encoder, device)
        }
        world.resource_mut::<SequenceQueue>().0.clear();
    });
}
