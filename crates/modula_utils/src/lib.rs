use std::ops::Range;

use bevy_ecs::prelude::*;
pub use hashbrown;
use modula_core::{EventOccurred, EventRes, ScheduleBuilder, ShuoldExit};
use winit::event::{Event, WindowEvent};

pub type HashMap<K, V> = hashbrown::HashMap<K, V>;
pub type HashSet<T> = hashbrown::HashSet<T>;

pub fn init_window_closing(schedule_builder: &mut ScheduleBuilder) {
    schedule_builder.add_systems(EventOccurred, handle_window_close)
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

/// Binary searches between lower and upper, returning the lowest value giving ok, if all values give error, the error returned by the end of the range is returned
pub fn binsearch<T, E>(
    mut f: impl FnMut(i32) -> Result<T, E>,
    mut range: Range<i32>,
) -> Result<T, E> {
    if range.is_empty() {
        panic!("binsearch on empty range");
    }
    let mut res = None;
    while range.start < range.end {
        let mid = (range.start + range.end) / 2;
        res = Some(f(mid));
        if res.as_ref().unwrap().is_ok() {
            range.end = mid;
        } else {
            range.start = mid + 1;
        }
    }
    res.unwrap()
}
