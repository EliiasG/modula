#![windows_subsystem = "windows"]

use bevy_ecs::prelude::*;
use modula::render;
use modula::render::Draw;
use modula::{
    core::{App, ScheduleBuilder},
    utils,
};
use modula_asset::{AssetId, Assets};
use modula_core::Init;
use modula_render::{
    ClearNext, EmptyPass, RenderTarget, Sequence, SequenceBuilder, SequenceQueue, SurfaceTargetRes,
};
use wgpu::Color;
use winit::window::WindowAttributes;

fn main() {
    let mut schedule_builder = ScheduleBuilder::new();
    render::init_render(&mut schedule_builder);
    utils::init_window_closing(&mut schedule_builder);
    schedule_builder.add_systems(Draw, set_color);
    schedule_builder.add_systems(Init, init_sequence);
    schedule_builder.add_systems(Draw, color_system);
    App { schedule_builder }.run(wgpu::PowerPreference::LowPower, WindowAttributes::default());
}

#[derive(Resource)]
struct SequenceRes(AssetId<Sequence>);

#[derive(Resource)]
struct FrameCount(u64);

fn set_color(
    mut render_target_assets: ResMut<Assets<RenderTarget>>,
    mut frame_count: ResMut<FrameCount>,
    surface_target: Res<SurfaceTargetRes>,
) {
    frame_count.0 += 1;
    render_target_assets
        .get_mut(surface_target.0)
        .unwrap()
        .set_clear_color(Color {
            r: (frame_count.0 % 200) as f64 / 200.0,
            g: (frame_count.0 % 600) as f64 / 600.0,
            b: (frame_count.0 % 1800) as f64 / 1800.0,
            a: 1.0,
        });
}

fn init_sequence(
    mut sequence_assets: ResMut<Assets<Sequence>>,
    surface_target: Res<SurfaceTargetRes>,
    mut commands: Commands,
) {
    let asset = SequenceBuilder::new()
        .add(ClearNext {
            render_target: surface_target.0,
        })
        .add(EmptyPass {
            render_target: surface_target.0,
        })
        .finish(&mut sequence_assets);
    commands.insert_resource(SequenceRes(asset));
    commands.insert_resource(FrameCount(0));
}

fn color_system(sequence_res: Res<SequenceRes>, mut sequence_queue: ResMut<SequenceQueue>) {
    sequence_queue.schedule(sequence_res.0);
}
