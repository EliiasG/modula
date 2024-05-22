#![windows_subsystem = "windows"]

use modula::core::{self, App, ScheduleBuilder};
use modula::render;
use winit::window::WindowAttributes;

fn main() {
    let mut schedule_builder = ScheduleBuilder::new();
    render::init_render(&mut schedule_builder);
    core::init_window_closing(&mut schedule_builder);
    App { schedule_builder }.run(wgpu::PowerPreference::LowPower, WindowAttributes::default());
}
