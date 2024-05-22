use bevy_ecs::{
    schedule::{Schedule, ScheduleLabel, Schedules},
    world::World,
};

pub trait WorldExt {
    fn run_and_apply_deferred(&mut self, label: impl ScheduleLabel);
    fn try_add_schedule(&mut self, label: impl ScheduleLabel + Clone);
}

impl WorldExt for World {
    fn run_and_apply_deferred(&mut self, label: impl ScheduleLabel) {
        self.schedule_scope(label, |world, schedule| {
            // should be fine not to world.run_schedule, as world.run_schedule is implemented like this
            schedule.run(world);
            schedule.apply_deferred(world);
        });
    }

    fn try_add_schedule(&mut self, label: impl ScheduleLabel + Clone) {
        let mut schedules = self.resource_mut::<Schedules>();
        if schedules.contains(label.clone()) {
            return;
        }
        schedules.insert(Schedule::new(label));
    }
}
