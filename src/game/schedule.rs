use super::GamePhase;
use crate::ecs::System;
use std::collections::HashMap;

pub trait ScheduleLabel: 'static {
    fn label(&self) -> &str;
}

impl ScheduleLabel for &'static str {
    fn label(&self) -> &str {
        self
    }
}

impl ScheduleLabel for String {
    fn label(&self) -> &str {
        self
    }
}

pub struct Schedule {
    label: Box<dyn ScheduleLabel>,
    systems: Vec<Box<dyn System>>,
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new("default")
    }
}

impl Schedule {
    pub fn new(label: impl ScheduleLabel) -> Schedule {
        Schedule {
            label: Box::new(label),
            systems: Vec::new(),
        }
    }

    pub fn add_system<T: System>(&mut self, system: T) -> &mut Self {
        self.systems.push(Box::new(system));

        self
    }

    pub fn dispatch(&mut self) -> &mut Self {
        self.systems.push(Box::new(|world: &crate::ecs::World| {
            world.dispatch();
        }));

        self
    }

    pub fn dispatch_type<T: crate::ecs::world::Event>(&mut self) -> &mut Self {
        self.systems.push(Box::new(|world: &crate::ecs::World| {
            world.dispatch_type::<T>();
        }));

        self
    }

    pub fn build(&mut self) -> Self {
        std::mem::take(self)
    }

    pub fn label(&self) -> &Box<dyn ScheduleLabel> {
        &self.label
    }

    pub fn run(&self, world: &mut crate::ecs::World) {
        for system in &self.systems {
            system.run(world);
        }
    }
}

pub struct SchedulePlan {
    schedules: HashMap<GamePhase, Vec<Schedule>>,
}

impl SchedulePlan {
    pub fn new() -> SchedulePlan {
        SchedulePlan {
            schedules: HashMap::new(),
        }
    }

    pub fn add_schedule(&mut self, phase: GamePhase, schedule: Schedule) -> &mut Self {
        self.schedules
            .entry(phase)
            .or_insert_with(Vec::new)
            .push(schedule);

        self
    }

    pub fn add_system(&mut self, phase: GamePhase, system: impl System) -> &mut Self {
        if let Some(schedule) = self
            .schedules
            .get_mut(&phase)
            .and_then(|schedules| schedules.last_mut())
        {
            schedule.add_system(system);
        } else if let Some(schedules) = self.schedules.get_mut(&phase) {
            if let Some(schedule) = schedules
                .iter_mut()
                .find(|s| s.label().label() == "default")
            {
                schedule.add_system(system);
            } else {
                let mut schedule = Schedule::default();
                schedule.add_system(system);
                schedules.push(schedule);
            }
        } else {
            let mut schedule = Schedule::default();
            schedule.add_system(system);
            self.schedules.insert(phase, vec![schedule]);
        }

        self
    }

    pub(super) fn run(&self, phase: GamePhase, world: &mut crate::ecs::World) {
        if let Some(schedules) = self.schedules.get(&phase) {
            for schedule in schedules {
                schedule.run(world);
            }
        }
    }
}
