use wgpu::{CommandEncoder, Device};

pub trait OperationBuilder {
    type ReturnOperation: Operation;
    fn finish(self, device: &mut Device) -> Self::ReturnOperation;
}

pub trait Operation {
    fn run(&mut self, command_encoder: &mut CommandEncoder);
}

pub struct Sequence {
    operations: Vec<Box<dyn Operation>>,
}

impl Sequence {
    pub fn run(&mut self, command_encoder: &mut CommandEncoder) {
        for op in self.operations.iter_mut() {
            op.run(command_encoder);
        }
    }
}

pub struct SequenceBuilder<'a> {
    operations: Vec<Box<dyn Operation>>,
    device: &'a mut Device,
}

impl SequenceBuilder<'_> {
    pub fn new<'a>(device: &'a mut Device) -> SequenceBuilder<'a> {
        return SequenceBuilder {
            operations: vec![],
            device,
        };
    }

    pub fn add(&mut self, operation_builder: impl OperationBuilder + 'static) {
        self.operations
            .push(Box::new(operation_builder.finish(self.device)));
    }

    pub fn finish(self) -> Sequence {
        return Sequence {
            operations: self.operations,
        };
    }
}
