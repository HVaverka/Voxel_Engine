use std::fs::File;
use std::io::Write;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use egui::FullOutput;
use egui_wgpu::ScreenDescriptor;
use wgpu::wgt::{BufferDescriptor, QuerySetDescriptor};
use wgpu::{BufferUsages, Device, QueryType};
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
                required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES | wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS,
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
        surface_config.present_mode = wgpu::PresentMode::Fifo;
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
                println!("Ok");
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

        let query_set = self.device.create_query_set(&QuerySetDescriptor {
            label: Some("Time query set"),
            ty: QueryType::Timestamp,
            count: 4,
        });

        encoder.write_timestamp(&query_set, 0);
        self.encode_compute_pass(&mut encoder);

        encoder.write_timestamp(&query_set, 1);
        self.encode_render_pass(&mut encoder, &frame);

        encoder.write_timestamp(&query_set, 2);
        self.encode_egui_pass(&mut encoder, egui, output, &frame);

        encoder.write_timestamp(&query_set, 3);

        let query_resolve_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Query Resolve Buffer"),
            size: size_of::<u64>() as u64 * 4,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let readback_buffer = self.device.create_buffer(&BufferDescriptor
            { label: Some("Readback buffer"),
            size: size_of::<u64>() as u64 * 4,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.resolve_query_set(&query_set, 0..4, &query_resolve_buffer, 0);
        encoder.copy_buffer_to_buffer(&query_resolve_buffer, 0, &readback_buffer, 0, size_of::<u64>() as u64 * 4);
/*
        let (_, nodes) = self.resources.get_world_buffer();
        let read_back_voxel_buffer = self.device.create_buffer(&BufferDescriptor {
            label: Some("Voxel Buffer"),
            size: nodes.size(),
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(nodes, 0, &read_back_voxel_buffer, 0, nodes.size());
*/

        self.queue.submit(Some(encoder.finish()));

        let _ = self.device.poll(wgpu::MaintainBase::Wait);

        let buffer_slice = readback_buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::MaintainBase::Wait);

        let data = buffer_slice.get_mapped_range();
        let stamps: &[u64] = bytemuck::cast_slice(&data);

        let period = self.queue.get_timestamp_period();
        println!("Compute pass duration: {}", (stamps[1] - stamps[0]) as f64 * period as f64);
        println!("Render pass duration: {}", (stamps[2] - stamps[1]) as f64 * period as f64);
        println!("Gui pass duration: {}", (stamps[3] - stamps[2]) as f64 * period as f64);

        /*
        let buffer_slice = read_back_voxel_buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::MaintainBase::Wait);
        let data = buffer_slice.get_mapped_range();

        let nodes: Vec<GpuNode2> = bytemuck::cast_slice(&data).to_vec();

        let mut file = File::create("OutputGPU.txt").unwrap();
        for item in &nodes {
            let _ = writeln!(file, "{:?}", item);
        }
        */
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

#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
struct GpuNode2 {
    mask0: u32,
    mask1: u32,
    base: u32,
    color: u32,
}