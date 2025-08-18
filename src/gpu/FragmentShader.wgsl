struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, 1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0)
    );
    var uv = (positions[vertex_index] + 1.0) * 0.5;
    return VertexOutput(vec4<f32>(positions[vertex_index], 0.0, 1.0), uv);
}

@group(0) @binding(0)
var output_texture: texture_storage_2d<rgba8unorm, read>;

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let coords = vec2<i32>(i32(pos.x), i32(pos.y));
    let sample = textureLoad(output_texture, coords);
    let color = vec3<f32>(sample.x, sample.y, sample.z);

    return vec4<f32>(linear_to_srgb(color), 1.0);
}

fn linear_to_srgb(c: vec3<f32>) -> vec3<f32> {
    return select(
        12.92 * c,
        1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055,
        c > vec3<f32>(0.0031308)
    );
}