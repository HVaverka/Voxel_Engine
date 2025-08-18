use crate::gpu::resources::Resources;
use std::num::NonZeroU64;
use wgpu::{
    hal::Surface, include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BufferBinding,
    BufferBindingType, ColorTargetState, ColorWrites, ComputePipeline, ComputePipelineDescriptor,
    FragmentState, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource,
    ShaderStages, StorageTextureAccess, SurfaceConfiguration, TextureFormat, VertexState,
};

pub struct Pipelines {
    shared_set: SharedSet,
    uniform_set: UniformSet,
    compute_set: ComputeSet,
    render_set: RenderSet,
}

pub struct SharedSet {
    layout_compute: BindGroupLayout,
    pub group_compute: BindGroup,

    layout_render: BindGroupLayout,
    pub group_render: BindGroup,
}
struct UniformSet {
    layout: BindGroupLayout,
    group: BindGroup,
}
struct ComputeSet {
    pipeline: wgpu::ComputePipeline,
    p_layout: wgpu::PipelineLayout,

    bind_group: wgpu::BindGroup,
    bg_layout: wgpu::BindGroupLayout,
}

struct RenderSet {
    pipeline: wgpu::RenderPipeline,
    p_layout: wgpu::PipelineLayout,
    // at this point only using shared resources in here
    // bind_group: wgpu::BindGroup,
    // bg_layout: wgpu::BindGroupLayout,
}

impl Pipelines {
    pub fn new(
        device: &wgpu::Device,
        resources: &Resources,
        surface_conf: &SurfaceConfiguration,
    ) -> Pipelines {
        let shared_set = create_shared_set(device, resources);
        let uniform_set = create_uniform_set(device, resources);
        let compute_set = create_compute_pipeline(device, resources, &shared_set, &uniform_set);
        let render_set = create_render_pipeline(device, resources, surface_conf, &shared_set);

        Self {
            shared_set,
            uniform_set,
            compute_set,
            render_set,
        }
    }
    pub fn get_compute_pipeline(&self) -> &ComputePipeline {
        &self.compute_set.pipeline
    }
    pub fn get_compute_bind_group(&self) -> &BindGroup {
        &self.compute_set.bind_group
    }

    pub fn get_render_pipeline(&self) -> &RenderPipeline {
        &self.render_set.pipeline
    }

    // we do not have render only group
    /*
    pub fn get_render_bind_group(&self) -> &BindGroup {
        &self.render_set.bind_group
    }
     */

    pub fn get_shared_bind_group(&self) -> &SharedSet {
        &self.shared_set
    }

    pub fn get_uniform_bind_group(&self) -> &BindGroup {
        &self.uniform_set.group
    }
}

fn create_shared_set(device: &wgpu::Device, resources: &Resources) -> SharedSet {
    let layout_compute = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Shared transfer texture for compute layout"),
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::WriteOnly,
                format: TextureFormat::Rgba8Unorm,
                view_dimension: wgpu::TextureViewDimension::D2,
            },
            count: None,
        }],
    });

    let group_compute = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Shared transfer texture for compute group"),
        layout: &layout_compute,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: BindingResource::TextureView(resources.get_shared_texture_view()),
        }],
    });

    let layout_render = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Shared transfer texture for render layout"),
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::ReadOnly,
                format: TextureFormat::Rgba8Unorm,
                view_dimension: wgpu::TextureViewDimension::D2,
            },
            count: None,
        }],
    });

    let group_render = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Shared transer texture for render group"),
        layout: &layout_render,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: BindingResource::TextureView(resources.get_shared_texture_view()),
        }],
    });
    SharedSet {
        layout_compute,
        group_compute,

        layout_render,
        group_render,
    }
}

fn create_uniform_set(device: &wgpu::Device, resources: &Resources) -> UniformSet {
    let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Uniform layout"),
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let group = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Uniform group"),
        layout: &layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: resources.view_port().as_entire_binding(),
        }],
    });

    UniformSet { layout, group }
}
fn create_compute_pipeline(
    device: &wgpu::Device,
    resources: &Resources,
    shared_set: &SharedSet,
    uniform_set: &UniformSet,
) -> ComputeSet {
    let bg_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Compute bind group layout"),
        entries: &[
            // header for buffre roots
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // world buffer nodes (reads)
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let (header, nodes) = resources.get_world_buffer();

    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Compute bind group"),
        layout: &bg_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: header.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: nodes.as_entire_binding(),
            },
        ],
    });

    let p_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Compute pipeline layout"),
        bind_group_layouts: &[&bg_layout, &shared_set.layout_compute, &uniform_set.layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
        label: Some("Compute pipeline"),
        layout: Some(&p_layout),
        module: &device.create_shader_module(include_wgsl!("ComputeShader.wgsl")),
        entry_point: Some("main"),
        compilation_options: PipelineCompilationOptions::default(),
        cache: None,
    });

    ComputeSet {
        pipeline,
        p_layout,
        bind_group,
        bg_layout,
    }
}

fn create_render_pipeline(
    device: &wgpu::Device,
    resources: &Resources,
    surface_config: &SurfaceConfiguration,
    shared_set: &SharedSet,
) -> RenderSet {
    // not needed as we use only the shared texture here
    /*
    let bg_laout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Render bind group layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::StorageTexture {
                    access: StorageTextureAccess::ReadOnly,
                    format: TextureFormat::Rgba8Unorm,
                    view_dimension: wgpu::TextureViewDimension::D2
                },
                count: None,
            },
        ],
    });

    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Render bind group"),
        layout: &bg_laout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: BindingResource::TextureView(resources.get_output_view()),
        }],
    });
     */

    let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Render pipeline layout"),
        bind_group_layouts: &[&shared_set.layout_render],
        push_constant_ranges: &[],
    });

    let format = surface_config.format;
    let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Render pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: VertexState {
            module: &device.create_shader_module(include_wgsl!("FragmentShader.wgsl")),
            entry_point: Some("vs_main"),
            compilation_options: PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(FragmentState {
            module: &device.create_shader_module(include_wgsl!("FragmentShader.wgsl")),
            entry_point: Some("fs_main"),
            compilation_options: PipelineCompilationOptions::default(),
            targets: &[Some(ColorTargetState {
                format: format,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    });

    RenderSet {
        pipeline: render_pipeline,
        p_layout: render_pipeline_layout,
        // bind_group: bind_group,
        // bg_layout: bg_laout,
    }
}
