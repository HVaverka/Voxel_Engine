use egui::{FullOutput};
use egui_wgpu::Renderer;
use wgpu::TextureFormat;
use winit::window::Window;

pub struct Egui {
    ctx: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
}

impl Egui {
    pub fn new(device: &wgpu::Device, window: &Window) -> Self {
        let ctx = egui::Context::default();
        let state = egui_winit::State::new(
            ctx.clone(),
            ctx.viewport_id(),
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let renderer =
            egui_wgpu::Renderer::new(device, TextureFormat::Rgba8UnormSrgb, None, 1, false);

        Self {
            ctx,
            state,
            renderer,
        }
    }

    pub fn begin_pass(&mut self, window: &Window) {
        let raw_input: egui::RawInput = self.state.take_egui_input(window);
        self.ctx.begin_pass(raw_input);
    }

    pub fn end_pass(&self) -> FullOutput {
        self.ctx.end_pass()
    }

    pub fn ctx(&self) -> &egui::Context {
        &self.ctx
    }

    pub fn state(&mut self) -> &mut egui_winit::State {
        &mut self.state
    }

    pub fn renderer(&mut self) -> &mut Renderer {
        &mut self.renderer
    }
}
