struct Camera {
    origin: vec3<f32>,
    dir: vec3<f32>,
    up: vec3<f32>,
    right: vec3<f32>,
    far: f32,
    fov: f32,
    screen: vec2<f32>,
}

struct Ray {
    dir: vec3<f32>,
    origin: vec3<f32>,
    invdir: vec3<f32>,
    
}

struct Header {
    base: vec3<i32>,
    end: vec3<i32>,
    size: u32,
}
struct GpuRoot {
    mask1: u32,
    mask0: u32,

    offset: u32,
    size: u32,
}
struct GpuNode {
    mask_h: u32,
    mask_l: u32,

    base: u32,
    color: u32,
}

@group(0) @binding(0)
var<uniform> header: Header;
@group(0) @binding(1)
var<storage, read> nodes: array<GpuNode>;

@group(1) @binding(0)
var output_texture: texture_storage_2d<rgba8unorm, write>;

@group(2) @binding(0)
var<uniform> cam: Camera;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let size = textureDimensions(output_texture);
    if (global_id.x >= size.x || global_id.y >= size.y) {
        return;
    }

//    let color = vec4<f32>(f32(global_id.x) / f32(size.x), f32(global_id.y) / f32(size.y), 0.5, 1.0);
//    textureStore(output_texture, vec2<i32>(global_id.xy), color);

    let PI: f32 = 3.14159265359;

    // compute primary ray through pixel
    let aspectRatio = cam.screen.x / cam.screen.y;
    let pX = (2.0 * ((f32(global_id.x) + 0.5) / cam.screen.x - 1.0)) * tan((cam.fov / 2.0 * PI / 180.0)) * aspectRatio;
    let pY = (1.0 - 2.0 * ((f32(global_id.y) + 0.5) / cam.screen.y)) * tan(cam.fov / 2.0 * PI / 180.0);
    let primary_ray = normalize(vec3<f32> (pX, pY, -1.0));

    // world ray
    let ray = normalize(primary_ray.x * cam.right + primary_ray.y * cam.up + primary_ray.z * cam.dir);

//    let dist: f32 = dda_iter(ray); // distance returned by DDA / dda_iter(ray);
//    let b: f32 = 1.0 / (1.0 + dist * dist); // closer = brighter, farther = darker

    let hit_info = dda_init(cam.origin, ray);

    if (hit_info.hit) {
        let target = hit_info.location;
        let dist = distance(target, cam.origin);
        let b: f32 = 1.0 / (1.0 + dist * dist); // closer = brighter, farther = darker
        textureStore(output_texture, vec2<i32>(global_id.xy), vec4<f32>(b, b, b, 1.0));
    } else {
        textureStore(output_texture, vec2<i32>(global_id.xy), vec4<f32>(0.0, 0.0, 0.0, 1.0));
    }
}

struct StackFrame {
    origin: vec3<f32>,
    map_check: vec3<i32>,
    ray_length: vec3<f32>,
    
    low: vec3<i32>,
    high: vec3<i32>,

    node: GpuNode,

    cell_size: f32,
    depth: u32,
    resumed: bool,
}

struct Stack {
    frame: array<StackFrame, 8>,
    sp: u32,
}

struct HitInfo {
    hit: bool,
    location: vec3<f32>,
}

// Initialize stack
fn stack_init(stack: ptr<function, Stack>) {
    (*stack).sp = 0u;
    // optional: clear frames
    for(var i: u32 = 0u; i < 8u; i = i + 1u) {
        (*stack).frame[i].resumed = false;
    }
}

// Push a frame
fn stack_push(stack: ptr<function, Stack>, frame: StackFrame) -> bool {
    if ((*stack).sp >= 8u) {
        return false; // stack overflow
    }
    (*stack).frame[(*stack).sp] = frame;
    (*stack).sp = (*stack).sp + 1u;
    return true;
}

// Peek top frame
fn stack_pop(stack: ptr<function, Stack>) {
    (*stack).sp = (*stack).sp - 1u;
}

fn stack_peek(stack: ptr<function, Stack>) -> StackFrame {
    return (*stack).frame[(*stack).sp - 1u];
}

fn stack_update(stack: ptr<function, Stack>, frame: StackFrame) {
    (*stack).frame[(*stack).sp - 1u].map_check = frame.map_check;
    (*stack).frame[(*stack).sp - 1u].ray_length = frame.ray_length;
    (*stack).frame[(*stack).sp - 1u].resumed = frame.resumed;
}

fn stack_not_empty(stack: ptr<function, Stack>) -> bool {
    if ((*stack).sp == 0u) {
        return false; // stack underflow
    }
    return true;
}

const SUBDIVISION: u32 = 4u;
const REGION_SIZE: u32 = 256u;
const SIZE_XYZ: vec3<i32> = vec3<i32>(i32(SUBDIVISION), i32(SUBDIVISION), i32(SUBDIVISION));
const MAX_STEP_COUNT = (3u * SUBDIVISION - 2u) * 8u;
const MAX_DEPTH = 3u;

fn dda_init(origin: vec3<f32>, ray_dir: vec3<f32>) -> HitInfo {
    let depth = 0;
    let cell_size: f32 = 256.0;
    var distance: f32 = 0.0;

    let step: vec3<i32> = vec3<i32>(sign(ray_dir));
    let ray_dir_offset = ray_dir * 0.0000001;
    let ray_unit_step: vec3<f32> = vec3<f32>(
        sqrt(1.0 + (ray_dir.y * ray_dir.y + ray_dir.z * ray_dir.z) / (ray_dir.x * ray_dir.x)),
        sqrt(1.0 + (ray_dir.x * ray_dir.x + ray_dir.z * ray_dir.z) / (ray_dir.y * ray_dir.y)),
        sqrt(1.0 + (ray_dir.x * ray_dir.x + ray_dir.y * ray_dir.y) / (ray_dir.z * ray_dir.z)),
    );

    let origin_grid: vec3<f32> = (origin + ray_dir_offset) / cell_size;
    var map_check: vec3<i32> = vec3<i32>(origin_grid);
    
    let delta: vec3<f32> = origin_grid - vec3<f32>(map_check);
    var ray_length: vec3<f32> = init_ray_length(ray_dir, ray_unit_step, delta);

    var stack = Stack();
    stack_init(&stack);

    while (!outside_bounds(map_check, header.base, header.end)) {
        let node = init_region(map_check);

        // skip empty chunk (region 0)
        if ((node.mask_l == 0u) && (node.mask_h == 0u)) {
            distance = step(&map_check, &ray_length, step, ray_unit_step);
            continue;
        }

        let new_origin = origin + ray_dir * distance * cell_size;
        let new_low = map_check * i32(SUBDIVISION);
        let new_high = new_low + SIZE_XYZ;

        let frame = StackFrame(
            new_origin,
            vec3<i32>(0), // map_check
            vec3<f32>(0.0), // ray_length

            new_low,
            new_high,

            node,

            cell_size / 4.0,
            1u, // depth
            false,
        )
        stack_push(&stack, frame);
        let hit_info = dda_iter(&stack, ray_dir, ray_unit_step, step);

        if (hit_info.hit) {
            return hit_info;
        }
    }
    return HitInfo(
        false,
        vec3<f32>(0.0),
    )
}

fn dda_iter(stack: ptr<function, Stack>, ray_dir: vec3<f32>, ray_unit_step: vec3<f32>, step: vec3<i32>) -> HitInfo {
    var map_check: vec3<i32>;
    var ray_length: vec3<f32>;

    while (stack_not_empty(stack)) {
        var frame: StackFrame = stack_peek(stack);

        let origin = frame.origin;

        var distance: f32 = 0.0;
        var inside_bounds = false;

        if (frame.resumed) {
            map_check = frame.map_check;
            ray_length = frame.ray_length;

            let result = step_and_check(&map_check, &ray_length, step, ray_unit_step, frame.low, frame.high);
            distance = result.distance;
            inside_bounds = result.inside_bounds;

        } else {
            let origin_grid: vec3<f32> = origin / frame.cell_size;
            map_check = vec3<i32>(origin_grid);
            
            let delta: vec3<f32> = origin_grid - vec3<f32>(map_check);
            ray_length = init_ray_length(ray_dir, ray_unit_step, delta);

            if (outside_bounds(map_check, frame.low, frame.high)) {
                let result = step_and_check(&map_check, &ray_length, step, ray_unit_step, frame.low, frame.high);
                distance = result.distance;
                inside_bounds = result.inside_bounds;
            }
        }

        while (inside_bounds) {
            let sub_region_bit = compute_shift(map_check);
            let not_empty = hit(sub_region_bit, frame.node);

            // sub node, later can be dependent on LOD
            if (not_empty && frame.depth < 4u) {
                let sub_node = init_subregion(sub_region_bit, frame.node);

                let new_origin = origin + ray_dir * distance * frame.cell_size;
                let low = map_check * 4;
                let high = low + SIZE_XYZ;

                let new_frame = StackFrame(
                    new_origin,
                    vec3<i32>(0), // map_check
                    vec3<f32>(0.0), // ray_length

                    low,
                    high,

                    sub_node,

                    frame.cell_size / 4.0,
                    frame.depth + 1u,

                    false, // resumed
                );

                frame.map_check = map_check;
                frame.ray_length = ray_length;
                frame.resumed = true;
                stack_update(stack, frame);
                stack_push(stack, new_frame);
                break;
            }
            // leaf voxel for now
            else if (not_empty) {
                return HitInfo(
                    true,
                    origin + ray_dir * distance * frame.cell_size,
                );
            }

            let result = step_and_check(&map_check, &ray_length, step, ray_unit_step, frame.low, frame.high);
            distance = result.distance;
            inside_bounds = result.inside_bounds;
        }
        stack_pop(stack);
    }

    // miss
    return HitInfo(
        false,
        vec3<f32>(0.0),
    );
}
fn init_region(map_check: vec3<i32>) -> GpuNode {
    if (outside_bounds(map_check, header.base, header.end)) {
        return nodes[0];
    }
    let coord = map_check - header.base;
    let per_axis = header.end - header.base;
    let offset = coord.x + coord.y * per_axis.x + coord.z * per_axis.x * per_axis.y;

    return nodes[offset];
}
fn init_subregion(shift: u32, node: GpuNode) -> GpuNode {
    if (shift < 32u) {
        let offset = countOneBits(node.mask_l & ((1u << shift) - 1u));
        return nodes[node.base + offset];
    }

    let o1 = countOneBits(node.mask_l);
    let o2 = countOneBits(node.mask_h & ((1u << (shift - 32u)) - 1u));

    return nodes[node.base + o1 + o2];
}
fn hit(shift: u32, node: GpuNode) -> bool {
    if (shift < 32u) {
        return (node.mask_l & (1u << shift)) != 0u;
    }

    return (node.mask_h & (1u << (shift - 32u))) != 0u;
}
fn compute_shift(map_check: vec3<i32>) -> u32 {
    let MASK: u32 = 0x3u; // 0b0000_0011;
    let mod = vec3<u32>(map_check) & vec3<u32>(MASK);
    let shift = mod.x + (mod.y << 2u) + (mod.z << 4u); // x + 4 * y + 16 * z
    return shift;
}

// fn dda_iter(ray_dir: vec3<f32>) -> vec3<f32> {
//     let cam_origin = cam.origin;
//     let step: vec3<i32> = vec3<i32>(sign(ray_dir));

//     let ray_dir_offset = ray_dir * 0.0000001;

//     let ray_unit_step: vec3<f32> = vec3<f32>(
//         sqrt(1.0 + (ray_dir.y * ray_dir.y + ray_dir.z * ray_dir.z) / (ray_dir.x * ray_dir.x)),
//         sqrt(1.0 + (ray_dir.x * ray_dir.x + ray_dir.z * ray_dir.z) / (ray_dir.y * ray_dir.y)),
//         sqrt(1.0 + (ray_dir.x * ray_dir.x + ray_dir.y * ray_dir.y) / (ray_dir.z * ray_dir.z)),
//     );

//     var curr: GpuNode;
//     // -------------------------------- //

//     // Stack set up
//     var stack: Stack = Stack();
//     stack_init(&stack);

//     // push base frame
//     let base_frame = StackFrame(
//         cam_origin,                     // origin
//         vec3<i32>(0, 0, 0),         // map_check
//         vec3<f32>(0.0, 0.0, 0.0),   // ray_length

//         header.base,    // low
//         header.end,     // high

//         init_region_offset(cam_origin), // offset to node
//         0u,      // depth
//         0u,     // steps done

//         false,  // step to boundary
//         false,  // resumed
//     );
//     stack_push(&stack, base_frame);

//     var map_check: vec3<i32> = vec3<i32>(0);
//     var ray_length: vec3<f32> = vec3<f32>(0.0);

//     while(stack_full(&stack)) {
//         var frame = stack_peek(&stack);

//         // local variables for layer in voxel grid:
//         let origin: vec3<f32> = frame.origin;

//         let low: vec3<i32> = frame.low;
//         let high: vec3<i32> = frame.high;
    
//         let node_offset: u32 = frame.node_offset;
//         let depth: u32 = frame.depth;
//         var steps: u32 = frame.steps;

//         let step_to_boundary: bool = frame.step_to_boundary;
//         var resumed: bool = frame.resumed;

//         let cell_size: f32 = f32(REGION_SIZE) / f32(pow_u32(SUBDIVISION, depth));
//         var distance = 0.0;
//         let node: GpuNode = get_region(node_offset);

//         // initialize ray at layer
//         if (!resumed) {
//             resumed = true;

//             let origin_grid = (origin + ray_dir_offset) / cell_size;
//             map_check = vec3<i32>(floor(origin_grid));

//             let delta = origin_grid - vec3<f32>(map_check);
//             ray_length = init_ray_length(ray_dir, ray_unit_step, delta);

//             if (step_to_boundary) {
//                 distance = step(&map_check, &ray_length, step, ray_unit_step);
//             }
//         } else {
//             map_check = frame.map_check;
//             ray_length = frame.ray_length;

//             distance = step(&map_check, &ray_length, step, ray_unit_step);
//             if (outside_bounds(map_check, low, high)) {
//                 stack_pop(&stack);
//                 continue;
//             }
//         }

//         for (var i = steps; i < MAX_STEP_COUNT; i++) {
//             let sub_node_offset = get_sub_region_offset(map_check, node);
//             if (depth == MAX_DEPTH) { return cam.origin; }

//             if (sub_node_offset != 0u && (depth < MAX_DEPTH - 1u)) {
//                 steps = i + 1u;
//                 frame.steps = steps;
//                 frame.map_check = map_check;
//                 frame.ray_length = ray_length;
//                 frame.resumed = resumed;
//                 stack_update(&stack, frame);

//                 let low = map_check * i32(SUBDIVISION);
//                 let high = low + SIZE_XYZ;

//                 let new_frame = StackFrame(
//                     origin + (ray_dir - ray_dir_offset) * distance * cell_size,

//                     vec3<i32>(0), // mapcheck
//                     vec3<f32>(0.0), // ray_length

//                     low,
//                     high,

//                     sub_node_offset,

//                     depth + 1u,
//                     0u, // steps

//                     distance != 0.0, // step to boundary
//                     false,
//                 );

//                 stack_push(&stack, new_frame);
//                 break;
//             }

//             if ((depth == MAX_DEPTH - 1u) && sub_node_offset != 0u) {
//                 // return point of collision
//                 return origin + ray_dir * distance * cell_size;
//             }

//             distance = step(&map_check, &ray_length, step, ray_unit_step);
//             if (outside_bounds(map_check, low, high)) {
//                 stack_pop(&stack);
//                 break;
//             }
//         }
//     }
//     return vec3<f32>(1000000.0);
// }

fn init_region_offset(origin: vec3<f32>) -> u32 {
    let coord = vec3<i32>(floor(origin / 256f));
    let offset = coord - header.base;
    let per_axis = header.end - header.base;

    let final_offset = u32(offset.x + offset.y * per_axis.x + offset.z * per_axis.x * per_axis.y);
    return final_offset + 1u; // + 1 because [0] is NULL
}

fn get_region(offset: u32) -> GpuNode {
    return nodes[offset];
}

fn get_sub_region_offset(map_check: vec3<i32>, node: GpuNode) -> u32 {
    let mask = SUBDIVISION - 1u;
    let coord  = vec3<u32>(map_check) & vec3<u32>(mask);
    // let offset = coord.x + coord.y * SUBDIVISION + coord.z * SUBDIVISION * SUBDIVISION;
    let shift = coord.x + coord.y * SUBDIVISION + coord.z * SUBDIVISION * SUBDIVISION;

    var pointer = 0u;
    if (shift < 32u) {
        if ((node.mask_l & (1u << shift)) != 0u) {
            let o1 = countOneBits(node.mask_l >> shift);
            let offset = o1 - 1u;

            pointer = node.base + offset;
        }
    } else {
        let shift = shift - 32u;
        if ((node.mask_h & (1u << shift)) != 0u) {
            let o1 = countOneBits(node.mask_l);
            let o2 = countOneBits(node.mask_h >> shift);
            let offset = o1 + o2 - 1u;

            pointer = node.base + offset;
        }
    }

    if (pointer >= header.size) { return 0u; }
    return pointer;
}

fn step(
    map_check: ptr<function, vec3<i32>>,
    ray_length: ptr<function, vec3<f32>>,
    step: vec3<i32>,
    ray_unit_step: vec3<f32>,
) -> f32 {
    // Find index of smallest component (0 = x, 1 = y, 2 = z)
    let min_xy = select(1u, 0u, (*ray_length).x < (*ray_length).y);
    let min_idx = select(2u, min_xy, (*ray_length)[min_xy] < (*ray_length).z);

    // Distance is that component
    let distance = (*ray_length)[min_idx];

    // Advance along that axis
    (*map_check)[min_idx] += step[min_idx];
    (*ray_length)[min_idx] += ray_unit_step[min_idx];

    return distance;
}

fn outside_bounds(map_check: vec3<i32>, low: vec3<i32>, high: vec3<i32>) -> bool {
    return any(map_check < low) || any(map_check >= high);
}

struct StepResult {
    distance: f32,
    inside_bounds: bool,
}
fn step_and_check(
    map_check: ptr<function, vec3<i32>>,
    ray_length: ptr<function, vec3<f32>>,
    step: vec3<i32>,
    ray_unit_step: vec3<f32>,
    low: vec3<i32>,
    high: vec3<i32>,
) -> StepResult {
    var distance: f32 = 0.0;

    // Find index of smallest component (0 = x, 1 = y, 2 = z)
    let min_xy = select(1u, 0u, (*ray_length).x < (*ray_length).y);
    let min_idx = select(2u, min_xy, (*ray_length)[min_xy] < (*ray_length).z);

    // Distance is that component
    let distance = (*ray_length)[min_idx];

    // Advance along that axis
    (*map_check)[min_idx] += step[min_idx];
    (*ray_length)[min_idx] += ray_unit_step[min_idx];

    // Check for outside_bounds for component
    let inside_bounds = (*map_check)[min_idx] >= low[min_idx] && (*map_check)[min_idx] < high[min_idx];

    return StepResult(
        distance,
        inside_bounds,
    )
}

fn init_ray_length(ray_dir: vec3<f32>, ray_unit_step: vec3<f32>, delta: vec3<f32>) -> vec3<f32> {
    let ray_length = select(
        (1.0 - delta) * ray_unit_step, // when ray_dir >= 0
        delta * ray_unit_step,         // when ray_dir < 0
        ray_dir < vec3<f32>(0.0),       // condition
    );

    return ray_length;
}

fn pow_u32(base: u32, exp: u32) -> u32 {
    var result: u32 = 1u;
    var e: u32 = exp;
    var b: u32 = base;

    // Fast exponentiation (O(log n))
    while (e > 0u) {
        if ((e & 1u) != 0u) {
            result = result * b;
        }
        e = e >> 1u;   // divide exponent by 2
        b = b * b;     // square base
    }
    return result;
}