use nalgebra::Vector3;
use std::sync::Arc;
use winit::window::{Window};

use crate::{
    app::input::{CursorState},
    core::{
        cpu_side_svo::{Loader, Stager},
        settings::{Action, Settings},
        types::{self},
    },
    gpu::{types::ViewPort, wgpu_ctx::WgpuCtx},
    UPDATE_PER_SECOND,
};

use crate::app::{input::InputState};
use crate::core::types::Camera;

const CAMERA_SPEED: f32 = 1.0 / UPDATE_PER_SECOND as f32;

pub struct Core {
    scene: types::Scene,
    camera: Camera,

    settings: Settings,
}

impl Core {
    pub fn new() -> Self {
        let mut scene = types::Scene::new();

        let mut loader = Loader::new();
        loader.load_data("dragon.vox");

        if let Ok(data) = loader.make_chunk() {
            scene.add_chunk(data, (3, 3, 3));
        }

        let camera = Camera::new();
        let settings = Settings::default();

        Core {
            scene,
            camera,
            settings,
        }
    }

    pub fn update(
        &mut self,
        delta_time: f64,
        input: &mut InputState,
        wgpu: &Option<WgpuCtx>,
        window: &Option<Arc<Window>>,
    ) -> bool {
        let changed = self.rotate_camera(delta_time, input);
        let changed = changed || self.move_camera(delta_time, input);
        if changed {
            self.update_view_port(wgpu, window);
        }

        self.grab(window, input);

        if !self.scene.world_changed() {
        } else if let Some(wgpu) = wgpu {
            self.scene.reset_changed();
            let data = self.stage_svo();

            for (i, r) in data.gpu_nodes.iter().enumerate() {
                if r.mask != 0 {
                    println!("{}", i)
                }
            }
            wgpu.replace_world_buffer(&data);
        }

        true
    }

    pub fn render(&mut self) {}

    pub fn draw_gui(&self) {}
    pub fn draw_debug_info(&self, ctx: &egui::Context) {
        let raw = self.camera.get_raw();
        let pos = raw.0;
        let dir = raw.1;

        egui::Window::new("Debug Info").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Position [x; y; z]: ");
                ui.label(format!(
                    "[{:.2}, {:.2}, {:.2}, {:.2}]",
                    pos[0], pos[1], pos[2], pos[3]
                ));
            });
            ui.horizontal(|ui| {
                ui.label("Direction: [x; y; z]");
                ui.label(format!(
                    "[{:.2}, {:.2}, {:.2}, {:.2}]",
                    dir[0], dir[1], dir[2], dir[3]
                ));
            })
        });
    }

    fn stage_svo(&self) -> Stager {
        let mut stager = Stager::new();
        stager.stage(&self.scene, (0, 0 , 0), (8, 8, 8));
        stager
    }

    fn move_camera(&mut self, delta_time: f64, input: &InputState) -> bool {
        let forward = input.is_pressed(self.settings.binding(Action::Forward));
        let backward = input.is_pressed(self.settings.binding(Action::Backwards));
        let left = input.is_pressed(self.settings.binding(Action::Left));
        let right = input.is_pressed(self.settings.binding(Action::Right));
        let up = input.is_pressed(self.settings.binding(Action::Up));
        let down = input.is_pressed(self.settings.binding(Action::Down));

        // Create a direction vector based on input
        let mut dir: nalgebra::Vector3<f32> = Vector3::zeros();

        if forward {
            dir.z -= 1.0;
        }
        if backward {
            dir.z += 1.0;
        }
        if left {
            dir.x -= 1.0;
        }
        if right {
            dir.x += 1.0;
        }
        if up {
            dir.y += 1.0;
        }
        if down {
            dir.y -= 1.0;
        }

        // Optional: normalize the direction
        if dir != Vector3::zeros() {
            dir = dir.normalize();
        }

        self.camera.move_cam(delta_time, &dir);

        dir != Vector3::zeros()
    }

    fn rotate_camera(&mut self, delta_time: f64, input: &mut InputState) -> bool {
        let (dx, dy) = input.get_mouse_delta();

        let sensitivity = 0.25; // add to settings eventually

        self.camera
            .rotate_cam(delta_time, dx * sensitivity, -dy * sensitivity); // minus for inversion

        dx != 0.0 || dy != 0.0
    }

    fn update_view_port(&self, wgpu: &Option<WgpuCtx>, window: &Option<Arc<Window>>) {
        match (wgpu, window) {
            (Some(wgpu), Some(window)) => {
                wgpu.update_view_port(&ViewPort::new(
                    &self.camera,
                    window.inner_size(),
                    self.settings.field_of_view(),
                ));
            }
            _ => {}
        }
    }

    fn grab(&mut self, window: &Option<Arc<Window>>, input: &mut InputState) {
        match (input.cursor_state(), window) {
            (CursorState::Entered, Some(window)) => {
                if input.consume_key(self.settings.binding(Action::GrabCursor)) {
                    input.cursor_locked();
                    let _ = window.set_cursor_grab(winit::window::CursorGrabMode::Locked);
                    window.set_cursor_visible(false);
                }
            }
            (CursorState::Locked, Some(window)) => {
                if input.consume_key(self.settings.binding(Action::GrabCursor)) {
                    input.cursor_entered();
                    let _ = window.set_cursor_grab(winit::window::CursorGrabMode::None);
                    window.set_cursor_visible(true);
                }
            }
            _ => {}
        }
    }
}
