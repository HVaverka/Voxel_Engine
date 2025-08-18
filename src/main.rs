mod app;
mod core;
mod gpu;
mod util;

use app::App;
use util::timer::Time;
use winit::event_loop::{ControlFlow, EventLoop};

pub const UPDATE_PER_SECOND: u32 = 120;

fn main() -> Result<(), winit::error::EventLoopError> {
    let mut app: App<'_, Time> = App::new();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    event_loop.run_app(&mut app)
}
