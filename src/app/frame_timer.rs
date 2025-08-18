use crate::util::timer::TimeTrait;

pub struct FrameTimer<T: TimeTrait> {
    updates_per_second: u32,
    max_frame_time: f64,
    pub exit_next_iteration: bool,
    pub window_occluded: bool,

    fixed_time_step: f64,
    number_of_updates: u64,
    number_of_renders: u64,
    last_frame_time: f64,
    running_time: f64,
    accumulated_time: f64,
    blending_factor: f64,
    previous_instant: T,
    current_instant: T,

    state: State,
}

impl<T: TimeTrait> FrameTimer<T> {
    pub fn new(updates_per_second: u32, max_frame_time: f64) -> Self {
        Self {
            updates_per_second,
            max_frame_time,

            exit_next_iteration: false,
            window_occluded: false,

            fixed_time_step: 1.0 / updates_per_second as f64,
            number_of_updates: 0,
            number_of_renders: 0,
            last_frame_time: 0.0,
            running_time: 0.0,
            accumulated_time: 0.0,
            blending_factor: 0.0,
            previous_instant: T::now(),
            current_instant: T::now(),

            state: State::Tick,
        }
    }

    pub fn tick(&mut self) {
        self.current_instant = T::now();

        let mut elapsed = self.current_instant.sub(&self.previous_instant);

        self.previous_instant = self.current_instant;

        if elapsed > self.max_frame_time {
            elapsed = self.max_frame_time;
        }

        self.last_frame_time = elapsed;
        self.accumulated_time += elapsed;
        self.running_time += elapsed;

        self.blending_factor = self.accumulated_time / self.fixed_time_step;
        self.number_of_renders += 1;
        self.state = State::Update;
    }

    pub fn drain_update(&mut self) {
        if self.accumulated_time <= self.fixed_time_step {
            self.state = State::Tick;
            return;
        }

        self.number_of_updates += 1;
        self.accumulated_time -= self.fixed_time_step;
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn re_accumulate(&mut self) {
        self.current_instant = T::now();

        let prev_elapsed = self.last_frame_time;
        let new_elapsed = self.current_instant.sub(&self.previous_instant);

        let delta = new_elapsed - prev_elapsed;

        // We don't update g.last_frame_time since this additional time in the
        // render function is considered part of the current frame.dt

        self.running_time += delta;
        self.accumulated_time += delta;

        self.blending_factor = self.accumulated_time / self.fixed_time_step;
    }

    pub fn exit(&mut self) {
        self.exit_next_iteration = true;
    }

    pub fn set_updates_per_second(&mut self, new_updates_per_second: u32) {
        self.updates_per_second = new_updates_per_second;
        self.fixed_time_step = 1.0 / new_updates_per_second as f64;
    }

    pub fn fixed_time_step(&self) -> f64 {
        self.fixed_time_step
    }

    pub fn number_of_updates(&self) -> u64 {
        self.number_of_updates
    }

    pub fn number_of_renders(&self) -> u64 {
        self.number_of_renders
    }

    pub fn last_frame_time(&self) -> f64 {
        self.last_frame_time
    }

    pub fn running_time(&self) -> f64 {
        self.running_time
    }

    pub fn accumulated_time(&self) -> f64 {
        self.accumulated_time
    }

    pub fn blending_factor(&self) -> f64 {
        self.blending_factor
    }

    pub fn previous_instant(&self) -> T {
        self.previous_instant
    }

    pub fn current_instant(&self) -> T {
        self.current_instant
    }
}

pub enum State {
    Update,
    Tick,
}
