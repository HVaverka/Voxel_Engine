# Voxel Engine

A voxel rendering engine written in Rust using [wgpu](https://wgpu.rs).  
It uses compute shaders and implements a recursive voxel traversal algorithm on the GPU.

## Issues
- Blank video output

## Getting Started

### Prerequisites
- Rust (latest stable recommended)
- Cargo
- Vulkan drivers installed
- (Optional) RenderDoc for debugging GPU resources

### Build
```sh
cargo build --release
