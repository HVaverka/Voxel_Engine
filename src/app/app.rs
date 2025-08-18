use std::{sync::Arc};

use wgpu::Device;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, WindowEvent, KeyEvent},
    event_loop::{ActiveEventLoop},
    window::{Window, WindowId},
};

use crate::app::{
    egui::Egui,
    input::{InputState},
};
use crate::gpu::wgpu_ctx::WgpuCtx;
use crate::{
    app::frame_timer::{FrameTimer, State},
    util::timer::TimeTrait,
};
use crate::{core::Core, UPDATE_PER_SECOND};
pub struct App<'window, T: TimeTrait> {
    window: Option<Arc<Window>>,
    wgpu: Option<WgpuCtx<'window>>,

    egui: Option<Egui>,

    core: Core,
    timer: FrameTimer<T>,

    input: InputState,

    exit_requested: bool,
}

impl<'window, T: TimeTrait> App<'window, T> {
    pub fn new() -> Self {
        let core = Core::new();
        let input = InputState::default();
        Self {
            window: None,
            wgpu: None,
            egui: None,

            core: core,
            timer: FrameTimer::new(UPDATE_PER_SECOND, 0.1),

            input: input,
            exit_requested: false,
        }
    }

    fn app_cycle(&mut self) {
        match self.timer.state() {
            State::Tick => {
                self.render();
                self.timer.tick();
            }
            State::Update => {
                self.update();
                self.timer.drain_update();
            }
        }
    }

    fn update(&mut self) {
        self.core.update(
            self.timer.fixed_time_step(),
            &mut self.input,
            &self.wgpu,
            &self.window,
        );
    }

    fn render(&mut self) {
        if let None = self.window {
            return;
        };

        let _window = self.window.as_ref().unwrap();
        let wgpu = self.wgpu.as_mut().unwrap();
        let egui = self.egui.as_mut().unwrap();

        // start egui pass
        egui.begin_pass(self.window.as_ref().unwrap());
        let ctx = egui.ctx();

        self.core.draw_gui();
        self.core.draw_debug_info(ctx);

        let output = egui.end_pass();

        // draw
        wgpu.draw(egui, output);
    }
}

impl<'window, T: TimeTrait> ApplicationHandler for App<'window, T> {
    // for mobile and wasm - not used, only for window creation
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = create_window(event_loop);
            let wgpu = create_wgpuctx(window.clone());
            let egui = create_egui(wgpu.device(), window.clone());

            self.window = Some(window);
            self.wgpu = Some(wgpu);
            self.egui = Some(egui);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let egui = self.egui.as_mut().unwrap();
        let state = egui.state();

        let _response = state.on_window_event(self.window.as_ref().unwrap(), &event);

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => {
                self.input.update_mouse_button(state, button);
            }
            WindowEvent::MouseWheel {
                device_id,
                delta,
                phase,
            } => {
                self.input.update_mouse_scroll(delta);
            }
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => {
                self.input.update_mouse_position(position);
            }
            WindowEvent::CursorEntered { device_id } => {
                self.input.cursor_entered();
            }
            WindowEvent::CursorLeft { device_id } => {
                self.input.cursor_left();
            }

            WindowEvent::KeyboardInput {
                device_id,
                event: key_event @ KeyEvent { repeat: false, .. },
                is_synthetic,
            } => {
                self.input.update_key(key_event, is_synthetic);
            }
            WindowEvent::RedrawRequested => {
                if self.exit_requested {
                    event_loop.exit();
                }
            }
            _ => (),
        }
        /*
        match self.input.cursor_state() {
            CursorState::Entered => {
                self.gui_event(event);
            }
            _ => {
                self.game_event(event, event_loop);
            }
        }
         */
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                self.input.update_mouse_delta(delta);
            }
            _ => {}
        }
    }

    // emitted after one update
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.exit_requested {
            event_loop.exit();
        }
        if let Some(w) = &self.window {
            let s =
                ((1.0 / self.timer.last_frame_time()) as u32).to_string() + " updates per second";
            w.set_title(&s);
        }
        self.app_cycle();
    }
}

fn create_window(event_loop: &ActiveEventLoop) -> Arc<Window> {
    let win_attr = Window::default_attributes()
        .with_title("VoxelGame")
        .with_resizable(false)
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600));
    // use Arc.
    Arc::new(
        event_loop
            .create_window(win_attr)
            .expect("create window err."),
    )
}

fn create_wgpuctx<'window>(window: Arc<Window>) -> WgpuCtx<'window> {
    WgpuCtx::new(window)
}

fn create_egui(device: &Device, window: Arc<Window>) -> Egui {
    Egui::new(device, &window)
}
