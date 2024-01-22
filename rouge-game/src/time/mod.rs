use rouge_ecs::{macros::Resource, world::resource::Resource};
use std::time::Instant;

#[derive(Resource)]
pub struct Time {
    current: Instant,
    current_fixed: Instant,
    delta_time: f32,
    time_scale: f32,
    fixed_time_scale: f32,
    fixed_delta_time: f32,
    frame_count: u64,
}

impl Time {
    pub fn new() -> Self {
        Self {
            current: Instant::now(),
            current_fixed: Instant::now(),
            delta_time: 0.0,
            time_scale: 1.0,
            fixed_time_scale: 1.0,
            fixed_delta_time: 1.0 / 60.0,
            frame_count: 0,
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        self.delta_time = now.duration_since(self.current).as_secs_f32();
        self.current = now;
        self.frame_count += 1;
    }

    pub fn fixed_update(&mut self) -> f32 {
        let now = Instant::now();
        let elapsed = now.duration_since(self.current_fixed).as_secs_f32();
        self.current_fixed = now;

        elapsed * self.fixed_time_scale
    }

    pub fn delta_time(&self) -> f32 {
        self.delta_time * self.time_scale
    }

    pub fn time_scale(&self) -> f32 {
        self.time_scale
    }

    pub fn set_time_scale(&mut self, time_scale: f32) {
        self.time_scale = time_scale;
    }

    pub fn fixed_time_scale(&self) -> f32 {
        self.fixed_time_scale
    }

    pub fn set_fixed_time_scale(&mut self, fixed_time_scale: f32) {
        self.fixed_time_scale = fixed_time_scale;
    }

    pub fn fixed_delta_time(&self) -> f32 {
        self.fixed_delta_time
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}
