use std::thread::sleep;
use std::time::{Duration, Instant};

pub trait TimeTrait: Copy {
    fn now() -> Self;
    fn sub(&self, other: &Self) -> f64;
    fn supports_sleep() -> bool;
    fn sleep(seconds: f64);
}

#[derive(Copy, Clone)]
pub struct Time(Instant);

impl TimeTrait for Time {
    fn now() -> Self {
        Self(Instant::now())
    }

    fn sub(&self, other: &Self) -> f64 {
        self.0.duration_since(other.0).as_secs_f64()
    }

    fn supports_sleep() -> bool {
        true
    }

    fn sleep(seconds: f64) {
        sleep(Duration::from_secs_f64(seconds));
    }
}
