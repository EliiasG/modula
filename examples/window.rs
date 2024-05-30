#![windows_subsystem = "windows"]

use modula::render;
use modula::{
    core::{self, App, ScheduleBuilder},
    utils,
};
use winit::window::WindowAttributes;

fn main() {
    let mut schedule_builder = ScheduleBuilder::new();
    render::init_render(&mut schedule_builder);
    utils::init_window_closing(&mut schedule_builder);
    App { schedule_builder }.run(wgpu::PowerPreference::LowPower, WindowAttributes::default());
}
