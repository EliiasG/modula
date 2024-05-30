use bevy_ecs::prelude::*;
pub use hashbrown;
use modula_core::{EventOccured, EventRes, ScheduleBuilder, ShuoldExit};
use winit::event::{Event, WindowEvent};

pub type HashMap<K, V> = hashbrown::HashMap<K, V>;

pub fn init_window_closing(schedule_builder: &mut ScheduleBuilder) {
    schedule_builder.add_systems(EventOccured, handle_window_close)
}

fn handle_window_close(mut commands: Commands, event: Res<EventRes>) {
    match event.0 {
        Event::WindowEvent {
            window_id: _,
            event: WindowEvent::CloseRequested,
        } => commands.insert_resource(ShuoldExit),
        _ => {}
    }
}
