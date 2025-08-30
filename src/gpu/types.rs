use std::default;

use bytemuck::{Pod, Zeroable};
use wgpu::{Buffer, BufferUsages};

#[repr(C)]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
pub struct GpuRoot {
    pub mask: u64,
    pub offset: u32,
    pub size: u32, // number of GpuNodes in whole subtree
}

#[repr(C)]
#[derive(Default, Clone, Copy, Pod, Zeroable, Debug)]
pub struct GpuNode {
    // bit i = 1: children base + i exists
    // if mask == 0 { this node is leaf_node }
    pub mask_h: u32,
    pub mask_l: u32,

    // base pointer to next GpuNode in Buffer
    pub base: u32,

    // index into color buffer
    pub color_index: u32,
}

impl GpuNode {
    pub fn set_leaf(mask: u64, color_index: u32) -> Self {
        Self {
            mask_h: (mask >> 32) as u32,
            mask_l: mask as u32,
            base: 0,
            color_index: color_index,
        }
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy, Pod, Zeroable)]
pub struct GpuSceneHeader {
    pub start: [i32; 4],
    pub end: [i32; 4],
    pub size: u32,
}

pub struct GpuScene {
    header: wgpu::Buffer,
    nodes: wgpu::Buffer, // <GpuNode>
}

impl GpuScene {
    pub fn new(device: &wgpu::Device, size_r: [u64; 3]) -> Self {
        let header = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Header"),
            size: size_of::<GpuSceneHeader>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let nodes = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Nodes"),
            size: 131_072,
            usage: BufferUsages::COPY_DST | BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        Self { header, nodes }
    }

    pub fn get_buffers(&self) -> (&Buffer, &Buffer) {
        (&self.header, &self.nodes)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct ViewPort {
    // 4 bytes are padding on each
    origin: [f32; 4],
    dir: [f32; 4],
    up: [f32; 4],
    right: [f32; 4],
    // plain data without padding
    far: f32,
    fov: f32,
    screen_x: f32,
    screen_y: f32,
}

use crate::core::types::Camera;
use winit::dpi::PhysicalSize;
impl ViewPort {
    pub fn new(cam: &Camera, size: PhysicalSize<u32>, fov: f64) -> Self {
        let (pos, dir, up, right) = cam.get_raw();

        Self {
            origin: pos,
            dir,
            up,
            right,
            far: 64.0,
            fov: fov as f32,
            screen_x: size.width as f32,
            screen_y: size.height as f32,
        }
    }
}

pub struct Uniforms {
    view_port: wgpu::Buffer,
}

impl Uniforms {
    pub fn new(device: &wgpu::Device) -> Self {
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Uniform camera"),
            size: size_of::<ViewPort>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self { view_port: camera }
    }

    pub fn view_port(&self) -> &wgpu::Buffer {
        &self.view_port
    }
}
