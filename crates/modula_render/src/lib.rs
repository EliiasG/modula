use std::iter;

use bevy_ecs::{prelude::*, schedule::ScheduleLabel};
use modula_core::{
    self, DeviceRes, EventOccured, EventRes, PreInit, QueueRes, ScheduleBuilder, ShuoldExit,
    SurfaceConfigRes, SurfaceRes, WindowRes, WorldExt,
};
use wgpu::{
    CommandEncoder, CommandEncoderDescriptor, SurfaceError, SurfaceTexture, TextureView,
    TextureViewDescriptor,
};
use winit::event::{Event, WindowEvent};
mod sequence;

pub use sequence::*;

#[derive(ScheduleLabel, Clone, Hash, PartialEq, Eq, Debug)]
pub struct PreDraw;

#[derive(ScheduleLabel, Clone, Hash, PartialEq, Eq, Debug)]
pub struct Draw;

/// This is intended to be private, it exists because commands used to insert resources related to rendering must be applied before rendering systems are run.  
/// Not sure if there is a more elegant solution (maybe just apply deferred for )
#[derive(ScheduleLabel, Clone, Hash, PartialEq, Eq, Debug)]
struct DrawSetup;

pub fn init_render(schedule_builder: &mut ScheduleBuilder) {
    schedule_builder.add_system(PreInit, |world: &mut World| {
        world.try_add_schedule(Draw);
        world.try_add_schedule(PreDraw);
    });
    schedule_builder.add_system(EventOccured, (handle_redraw_command, handle_resized));
    schedule_builder.add_system(DrawSetup, draw_setup);
}

fn handle_resized(
    event_res: Res<EventRes>,
    mut surface_config: ResMut<SurfaceConfigRes>,
    surface: Res<SurfaceRes>,
    device: Res<DeviceRes>,
) {
    let surface = &surface.0;
    let surface_config = &mut surface_config.0;
    let device = &device.0;
    //FIXME maybe handle scale factor change?
    let size = match &event_res.0 {
        Event::WindowEvent {
            window_id: _,
            event: WindowEvent::Resized(size),
        } => size,
        _ => return,
    };
    if size.height == 0 || size.width == 0 {
        return;
    }
    surface_config.width = size.width;
    surface_config.height = size.height;
    surface.configure(device, &surface_config);
}

#[derive(Resource)]
struct ShouldDraw;

#[derive(Resource)]
pub struct SurfaceTextureRes(pub SurfaceTexture);

#[derive(Resource)]
pub struct SurfaceTextureViewRes(pub TextureView);

#[derive(Resource)]
pub struct CommandEncoderRes(pub CommandEncoder);

fn handle_redraw_command(world: &mut World) {
    match world.resource::<EventRes>().0 {
        Event::WindowEvent {
            window_id: _,
            event: WindowEvent::RedrawRequested,
        } => {}
        _ => return,
    }
    world.run_and_apply_deferred(DrawSetup);
    // if ShouldDraw exists it is removed, if not return
    if world.remove_resource::<ShouldDraw>().is_none() {
        return;
    }
    world.run_and_apply_deferred(PreDraw);
    world.run_and_apply_deferred(Draw);
    // would be overkill to make a schedule, since it just removes resources and draws CommandBuffer
    draw_finish(world);
}

fn draw_finish(world: &mut World) {
    world.remove_resource::<SurfaceTextureViewRes>();
    let command_encoder = world
        .remove_resource::<CommandEncoderRes>()
        .expect("No CommandBuffer, did you remove it, you little rascal? huh?")
        .0;
    let surface_texture = world
        .remove_resource::<SurfaceTextureRes>()
        .expect("No SurfaceTexture, did you remove it, you little rascal? huh?")
        .0;
    world
        .resource_mut::<QueueRes>()
        .0
        .submit(iter::once(command_encoder.finish()));
    surface_texture.present();
    world.resource::<WindowRes>().0.request_redraw();
}

fn draw_setup(
    mut commands: Commands,
    device: Res<DeviceRes>,
    surface: Res<SurfaceRes>,
    surface_config: Res<SurfaceConfigRes>,
    window: Res<WindowRes>,
) {
    let device = &device.0;
    let surface = &surface.0;
    let surface_config = &surface_config.0;
    let window = window.0;
    let texture = match surface.get_current_texture() {
        Ok(t) => t,
        Err(SurfaceError::OutOfMemory) => {
            commands.insert_resource(ShuoldExit);
            return;
        }
        Err(SurfaceError::Lost | SurfaceError::Outdated) => {
            surface.configure(device, surface_config);
            window.request_redraw();
            return;
        }
        Err(_) => {
            window.request_redraw();
            return;
        }
    };
    commands.insert_resource(ShouldDraw);
    commands.insert_resource(SurfaceTextureViewRes(
        texture
            .texture
            .create_view(&TextureViewDescriptor::default()),
    ));
    commands.insert_resource(SurfaceTextureRes(texture));
    commands.insert_resource(CommandEncoderRes(
        // should be fine not to have a label as it should be the only command encoder
        device.create_command_encoder(&CommandEncoderDescriptor { label: None }),
    ));
}
