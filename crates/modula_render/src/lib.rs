use bevy_ecs::{prelude::*, schedule::ScheduleLabel};
use modula_asset::{init_assets, AssetId, AssetWorldExt, Assets, InitAssetsSet};
use modula_core::{
    self, DeviceRes, EventOccurred, EventRes, PreInit, ScheduleBuilder, ShuoldExit,
    SurfaceConfigRes, SurfaceRes, WindowRes, WorldExt,
};
use wgpu::SurfaceError;
use winit::event::{Event, WindowEvent};
mod render_target;
mod sequence;

pub use render_target::*;
pub use sequence::*;

/// Used to extract / sync data for drawing
#[derive(ScheduleLabel, Clone, Hash, PartialEq, Eq, Debug)]
pub struct PreDraw;

/// Used for drawing and updating
#[derive(ScheduleLabel, Clone, Hash, PartialEq, Eq, Debug)]
pub struct Draw;

/// This is intended to be private, it exists because commands used to insert resources related to rendering must be applied before rendering systems are run.  
/// Not sure if there is a more elegant solution (maybe just apply deferred for )
#[derive(ScheduleLabel, Clone, Hash, PartialEq, Eq, Debug)]
struct DrawSetup;

/// Runs in EventOccurred to do rendering and run Draw and PreDraw
#[derive(SystemSet, Clone, Hash, PartialEq, Eq, Debug)]
pub struct RenderSystemSet;

pub fn init_render(schedule_builder: &mut ScheduleBuilder) {
    schedule_builder.add_systems(PreInit, |world: &mut World| {
        world.try_add_schedule(Draw);
        world.try_add_schedule(PreDraw);
    });
    // maybe should be in a set, but SurfaceTargetRes should probably not be used before init anyway
    schedule_builder.add_systems(
        PreInit,
        (|world: &mut World| {
            let asset = world.add_asset(RenderTarget::new(RenderTargetConfig::default()));
            world.insert_resource(SurfaceTargetRes(asset));
        })
        .after(InitAssetsSet),
    );
    schedule_builder.add_systems(
        EventOccurred,
        (handle_redraw_event, handle_resized).in_set(RenderSystemSet),
    );
    schedule_builder.add_systems(DrawSetup, draw_setup);
    init_sequences(schedule_builder);
    init_assets::<RenderTarget>(schedule_builder);
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
    // TODO maybe handle scale factor change?
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
pub struct SurfaceTargetRes(pub AssetId<RenderTarget>);

fn handle_redraw_event(world: &mut World) {
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
    // FIXME maybe submit queue here because of texture loading, currently textures will only load at end of frame
    world.run_and_apply_deferred(Draw);
    // would be overkill to make a schedule, since it just removes resources presents surface
    sequence::run_sequences(world);
    draw_finish(world);
}

fn draw_finish(world: &mut World) {
    let surface_target = world.resource::<SurfaceTargetRes>().0;
    world.with_asset(surface_target, |target| target.present());
    world.resource::<WindowRes>().0.request_redraw();
}

fn draw_setup(
    mut commands: Commands,
    device: Res<DeviceRes>,
    surface: Res<SurfaceRes>,
    surface_config: Res<SurfaceConfigRes>,
    surface_target: Res<SurfaceTargetRes>,
    mut render_target_assets: ResMut<Assets<RenderTarget>>,
    window: Res<WindowRes>,
) {
    let device = &device.0;
    let surface = &surface.0;
    let surface_config = &surface_config.0;
    let window = window.0;
    let texture = match surface.get_current_texture() {
        Ok(t) => t,
        Err(SurfaceError::OutOfMemory) => {
            eprintln!("Out of memory while getting surface texture");
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
    render_target_assets
        .get_mut(surface_target.0)
        .expect("no render target")
        .apply_surface(device, texture);
    commands.insert_resource(ShouldDraw);
}
