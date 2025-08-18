use wgpu::wgt::SamplerDescriptor;
use wgpu::{
    Buffer, BufferDescriptor, BufferUsages, Extent3d, TextureDescriptor, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor,
};

use crate::core::cpu_side_svo::Stager;
use crate::gpu::types::{self, GpuNode, ViewPort};
pub struct Resources {
    shared_texture: wgpu::Texture,
    shared_texture_view: wgpu::TextureView,

    scene: types::GpuScene,
    uniform: types::Uniforms,
}

impl Resources {
    pub fn new(device: &wgpu::Device, surface_config: &wgpu::SurfaceConfiguration) -> Resources {
        let (width, height) = (surface_config.width, surface_config.height);
        let shared_texture = device.create_texture(&TextureDescriptor {
            label: Some("Output texture"),
            size: Extent3d {
                width: width,
                height: height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let shared_texture_view = shared_texture.create_view(&TextureViewDescriptor::default());

        // Sparse voxel octree
        let scene = types::GpuScene::new(device, [8, 8, 8]);

        // Uniform buffers
        let uniform = types::Uniforms::new(device);

        Self {
            shared_texture,
            shared_texture_view,

            scene,
            uniform,
        }
    }
    pub fn get_shared_texture_view(&self) -> &TextureView {
        &self.shared_texture_view
    }
    pub fn get_world_buffer(&self) -> (&Buffer, &Buffer) {
        self.scene.get_buffers()
    }

    pub fn replace_world_buffer(&self, queue: &wgpu::Queue, data: &Stager) {
        let (header, nodes) = self.get_world_buffer();

        let length = bytemuck::cast_slice::<GpuNode, u8>(&data.gpu_nodes).len();
        queue.write_buffer(nodes, size_of::<GpuNode>() as u64, bytemuck::cast_slice(&data.gpu_nodes));
        queue.write_buffer(header, 0, bytemuck::bytes_of(&data.header));

        queue.submit([]);
    }

    pub fn view_port(&self) -> &Buffer {
        self.uniform.view_port()
    }

    pub fn update_view_port(&self, queue: &wgpu::Queue, data: &ViewPort) {
        queue.write_buffer(self.view_port(), 0, bytemuck::bytes_of(data));
    }
}
