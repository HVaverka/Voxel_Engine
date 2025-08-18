use std::any::Any;
use std::sync::Arc;

use egui::FullOutput;
use egui_wgpu::ScreenDescriptor;
use wgpu::Device;
use winit::window::Window;

use wgpu::{
    CommandEncoder, CommandEncoderDescriptor, ComputePassDescriptor, MemoryHints::Performance,
    Operations, RenderPassColorAttachment, RenderPassDescriptor, SurfaceTexture,
    TextureViewDescriptor,
};

use crate::app::egui::Egui;
use crate::core::cpu_side_svo::Stager;
use crate::gpu::types::ViewPort;
use crate::gpu::{pipelines::Pipelines, resources::Resources, types::GpuNode};
pub struct WgpuCtx<'window> {
    surface: wgpu::Surface<'window>,
    surface_config: wgpu::SurfaceConfiguration,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,

    pipelines: Pipelines,
    resources: Resources,
}

impl<'window> WgpuCtx<'window> {
    pub fn new(window: Arc<Window>) -> WgpuCtx<'window> {
        pollster::block_on(WgpuCtx::new_async(window))
    }

    async fn new_async(window: Arc<Window>) -> WgpuCtx<'window> {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(Arc::clone(&window)).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                required_limits: adapter.limits(),
                memory_hints: Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("Failed to create device");

        let mut size = window.inner_size();
        // w and h > 1 or it panics
        let width = size.width.max(1);
        let height = size.height.max(1);
        // default config for surface to use
        let mut surface_config = surface.get_default_config(&adapter, width, height).unwrap();
        surface_config.format = wgpu::TextureFormat::Rgba8UnormSrgb;
        surface.configure(&device, &surface_config);

        let resources = Resources::new(&device, &surface_config);
        let pipelines = Pipelines::new(&device, &resources, &surface_config);

        Self {
            surface,
            surface_config,
            adapter,
            device,
            queue,

            resources,
            pipelines,
        }
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn replace_world_buffer(&self, data: &Stager) {
        self.resources.replace_world_buffer(&self.queue, data);
    }

    pub fn update_view_port(&self, data: &ViewPort) {
        self.resources.update_view_port(&self.queue, data);
    }

    pub fn draw(&mut self, egui: &mut Egui, output: FullOutput) {
        match self.surface.get_current_texture() {
            Ok(frame) => {
                self.dispatch_pipelines(frame, egui, output);
            }
            Err(wgpu::SurfaceError::Lost) => {
                println!("Surface lost! Reconfiguring...");
            }
            Err(wgpu::SurfaceError::Timeout) => {
                println!("Surface timeout! Skipping this frame...");
            }
            Err(e) => {
                eprintln!("Failed to acquire swap chain frame: {:?}", e);
            }
        }
    }

    fn dispatch_pipelines(&mut self, frame: SurfaceTexture, egui: &mut Egui, output: FullOutput) {
        let mut encoder = self.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Compute encoder"),
        });

        self.encode_compute_pass(&mut encoder);
        self.encode_render_pass(&mut encoder, &frame);
        self.encode_egui_pass(&mut encoder, egui, output, &frame);

        self.queue.submit(Some(encoder.finish()));
        frame.present(); // ✅ Present frame after submission
    }

    fn encode_compute_pass(&self, encoder: &mut CommandEncoder) {
        let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("Compute pass"),
            timestamp_writes: None,
        });

        let window_size = (self.surface_config.width, self.surface_config.height);
        let shared_set = self.pipelines.get_shared_bind_group();

        compute_pass.set_pipeline(self.pipelines.get_compute_pipeline());
        compute_pass.set_bind_group(0, self.pipelines.get_compute_bind_group(), &[]);
        compute_pass.set_bind_group(1, &shared_set.group_compute, &[]);
        compute_pass.set_bind_group(2, self.pipelines.get_uniform_bind_group(), &[]);
        compute_pass.dispatch_workgroups((window_size.0 + 7) / 8, (window_size.1 + 7) / 8, 1);
    }

    fn encode_render_pass(&self, encoder: &mut CommandEncoder, frame: &SurfaceTexture) {
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let shared_set = self.pipelines.get_shared_bind_group();

        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(self.pipelines.get_render_pipeline());

        // do not have only render group, if yes shared index should be 1,
        //          render_pass.set_bind_group(0, self.pipelines.get_render_bind_group(), &[]);
        render_pass.set_bind_group(0, &shared_set.group_render, &[]);

        render_pass.draw(0..6, 0..1);
    }

    fn encode_egui_pass(
        &self, 
        encoder: &mut CommandEncoder,
        egui: &mut Egui,
        output: FullOutput,
        frame: &SurfaceTexture,
    ) {
        let paint_jobs = egui
            .ctx()
            .tessellate(output.shapes, output.pixels_per_point);
        let renderer = egui.renderer();
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.surface_config.width, self.surface_config.height],
            pixels_per_point: output.pixels_per_point,
        };
        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        let mut render_pass = encoder
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("Gui render pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            })
            .forget_lifetime();

        for (id, image_delta) in output.textures_delta.set {
            renderer.update_texture(&self.device, &self.queue, id, &image_delta);
        }

        renderer.update_buffers(
            &self.device,
            &self.queue,
            encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        renderer.render(&mut render_pass, &paint_jobs, &screen_descriptor);

        for id in output.textures_delta.free {
            renderer.free_texture(&id);
        }
    }

    fn create_command_encoder(&self, desc: &CommandEncoderDescriptor) -> CommandEncoder {
        self.device.create_command_encoder(desc)
    }
}
