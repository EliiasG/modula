use std::time::{Duration, Instant};

use bevy_ecs::system::{Commands, Res, ResMut, Resource};
use modula_core::{EventOccurred, EventRes, Init, ScheduleBuilder};
use winit::event::{Event, WindowEvent};

pub fn init_time(schedule_builder: &mut ScheduleBuilder) {
    schedule_builder.add_systems(EventOccurred, update_time);
    schedule_builder.add_systems(Init, |mut c: Commands| {
        c.insert_resource(Time {
            delta: Duration::from_secs_f64(1.0 / 30.0),
            elapsed: Duration::from_secs(0),
            frame_start: None,
        })
    });
}

#[derive(Resource)]
pub struct Time {
    delta: Duration,
    elapsed: Duration,
    frame_start: Option<Instant>,
}

impl Time {
    /// Time since last frame
    pub fn delta(&self) -> Duration {
        self.delta
    }
    /// seconds since last frame as f32
    pub fn delta_f32(&self) -> f32 {
        self.delta.as_secs_f32()
    }
    /// seconds since last frame as f64
    pub fn delta_f64(&self) -> f64 {
        self.delta.as_secs_f64()
    }

    /// Total running duration
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Total running duration as seconds
    pub fn elapsed_f32(&self) -> f32 {
        self.elapsed.as_secs_f32()
    }

    /// Total running duration as seconds
    pub fn elapsed_f64(&self) -> f64 {
        self.elapsed.as_secs_f64()
    }

    /// Reset every time the window is to be redrawn
    pub fn frame_start(&self) -> Instant {
        self.frame_start
            .expect("frame_start called before fisrt frame")
    }
}

fn update_time(event: Res<EventRes>, mut time: ResMut<Time>) {
    match event.0 {
        Event::WindowEvent {
            window_id: _,
            event: WindowEvent::RedrawRequested,
        } => {}
        _ => return,
    }
    let now = Instant::now();
    let delta = if let Some(prev) = time.frame_start {
        now - prev
    } else {
        // kinda arbitrary but initial delta should not really be important
        Duration::from_secs_f64(1.0 / 30.0)
    };
    time.elapsed += delta;
    time.frame_start = Some(now);
    time.delta = delta
}
