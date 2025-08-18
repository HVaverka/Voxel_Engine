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
    mask1: u32,
    mask0: u32,

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

    if (dda_iter(ray)) {
        textureStore(output_texture, vec2<i32>(global_id.xy), vec4<f32>(1.0, 1.0, 1.0, 1.0));
    } else {
        textureStore(output_texture, vec2<i32>(global_id.xy), vec4<f32>(0.0, 0.0, 0.0, 1.0));
    }
}

// fn dda(ray_dir: vec3<f32>, ray_orig: vec3<f32>) -> bool {
//     var cell_size = array<f32, 5>(256.0, 64.0, 16.0, 4.0, 1.0);
//     var stack = stack_init();
//     var map_check: array<vec3<i32>, 5>;
//     for (var i = 0; i < 5; i++) {
//         map_check[i] = vec3<i32>(floor(ray_orig) / cell_size[i]);
//     }

//     // find largest empty cell
//     let offset = vec3<u32>(map_check[0] - header.base);
//     let r_ptr = offset.x + u32(4) * offset.y + u32(16) * offset.z;
//     let region = roots[r_ptr];
//     push(&stack, r_ptr);
//     if ((region.mask0 | region.mask1) != u32(0)) {
//         for (var i = 1; i < 5 && ((region.mask0 | region.mask1) != u32(0)); i++) {
//             let offset = vec3<u32>(map_check[i] - header.base);
//             let r_ptr = offset.x + u32(4) * offset.y + u32(16) * offset.z;
//             let region = roots[r_ptr];
//             push(&stack, r_ptr);
//         }
//     }

//     let step = vec3<i32>(sign(ray_dir));
    

//     //let rd = { x: cos(rdAngle), y: sin(rdAngle) }

//     let rayUnitStepSize = vec3<f32>(
//         sqrt(1.0 + (ray_dir.y * ray_dir.y + ray_dir.z * ray_dir.z) / (ray_dir.x * ray_dir.x)),
//         sqrt(1.0 + (ray_dir.x * ray_dir.x + ray_dir.z * ray_dir.z) / (ray_dir.y * ray_dir.y)),
//         sqrt(1.0 + (ray_dir.x * ray_dir.x + ray_dir.y * ray_dir.y) / (ray_dir.z * ray_dir.z)),
//     );

//     var rayLength: array<vec3<f32>, 5>;
//     var rayLength = vec3<f32>(0.0, 0.0, 0.0);

//     let roFract = ray_orig - vec3<f32>(map_check);

//     if (ray_dir.x < 0.0) {
//         rayLength.x = roFract.x * rayUnitStepSize.x;
//     } else {
//         rayLength.x = (1.0 - roFract.x) * rayUnitStepSize.x;
//     }

//     if (ray_dir.y < 0.0) {
//         rayLength.y = roFract.y * rayUnitStepSize.y;
//     } else {
//         rayLength.y = (1.0 - roFract.y) * rayUnitStepSize.y;
//     }

//     if (ray_dir.z < 0.0) {
//         rayLength.z = roFract.z * rayUnitStepSize.z;
//     } else {
//         rayLength.z = (1.0 - roFract.z) * rayUnitStepSize.z;
//     }

//     var distance = 0.0;
//     for (var i = 0; i < 100; i++) {
//         if (rayLength.x < rayLength.y) {
//             if (rayLength.x < rayLength.z) {
//                 map_check.x += step.x;
//                 distance = rayLength.x;
//                 rayLength.x += rayUnitStepSize.x;
//             } else {
//                 map_check.z += step.z;
//                 distance = rayLength.z;
//                 rayLength.z += rayUnitStepSize.z;
//             }
//         } else {
//             if (rayLength.y < rayLength.z) {
//                 map_check.y += step.y;
//                 distance = rayLength.y;
//                 rayLength.y += rayUnitStepSize.y;
//             } else {
//                 map_check.z += step.z;
//                 distance = rayLength.z;
//                 rayLength.z += rayUnitStepSize.z;
//             }
//         }

//         let offset = map_check;

//         let corrected = offset - header.base;
//         let ptr_ = corrected.x + 8 * corrected.y + 64 * corrected.z;
//         if (ptr_ < 0 || u32(ptr_) >= header.size) {
//             continue;
//         }
//         let root = roots[ptr_];

//         if ((root.mask0 | root.mask1) != 0u) {
//             return true;
//         }
//     }
//     return false;
// }

// struct Stack {
//     data: array<u32, 16>,
//     sp: u32,
// }

// fn stack_init() -> Stack {
//     var s: Stack;
//     s.sp = u32(0);
//     return s;
// }

// fn push(s: ptr<function, Stack>, value: u32) {
//     (*s).data[(*s).sp] = value;
//     (*s).sp = (*s).sp + u32(1);
// }

// fn pop(s: ptr<function, Stack>) -> u32 {
//     (*s).sp = (*s).sp - u32(1);
//     return (*s).data[(*s).sp];
// }

// fn stack_isEmpty(s: ptr<function, Stack>) -> bool {
//     return (*s).sp == u32(0);
// }


struct StackFrame {
    origin: vec3<f32>,
    map_check: vec3<i32>,
    ray_length: vec3<f32>,
    
    low: vec3<i32>,
    high: vec3<i32>,

    node_offset: u32,
    depth: i32,
    
    step_to_boundary: bool,
    resumed: bool,
}

struct Stack {
    frame: array<StackFrame, 8>,
    sp: u32,
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
    (*stack).frame[(*stack).sp - 1u] = frame;
}

// Pop top frame
fn stack_full(stack: ptr<function, Stack>) -> bool {
    if ((*stack).sp == 0u) {
        return false; // stack underflow
    }
    return true;
}

const SUBDIVISION: u32 = 4u;
const REGION_SIZE: u32 = 256u;
const SIZE_XYZ: vec3<i32> = vec3<i32>(i32(SUBDIVISION), i32(SUBDIVISION), i32(SUBDIVISION));
const MAX_STEP_COUNT = 3u * SUBDIVISION - 2u;

fn dda_iter(ray_dir: vec3<f32>) -> bool {
    let origin = cam.origin;
    let step: vec3<i32> = vec3<i32>(sign(ray_dir));

    let ray_dir_offset = ray_dir * 0.0000001;

    let ray_unit_step: vec3<f32> = vec3<f32>(
        sqrt(1.0 + (ray_dir.y * ray_dir.y + ray_dir.z * ray_dir.z) / (ray_dir.x * ray_dir.x)),
        sqrt(1.0 + (ray_dir.x * ray_dir.x + ray_dir.z * ray_dir.z) / (ray_dir.y * ray_dir.y)),
        sqrt(1.0 + (ray_dir.x * ray_dir.x + ray_dir.y * ray_dir.y) / (ray_dir.z * ray_dir.z)),
    );

    var curr_node = GpuNode(
        0u,
        0u,
        0u,
        0u,
    );

    var low: vec3<i32> = header.base;
    var high: vec3<i32> = header.end;

    let base_frame = StackFrame(
        origin,                     // origin
        vec3<i32>(0, 0, 0),         // map_check
        vec3<f32>(0.0, 0.0, 0.0),   // ray_length

        header.base,    // low
        header.end,     // high

        0u,     // node
        1,      // depth
        false,  // step to boundary
        false,  // resumed
    );

    var stack = Stack();
    stack_init(&stack);

    stack_push(&stack, base_frame);

    while stack_full(&stack) {
        var frame = stack_peek(&stack);
        var distance = 0f;

        if (frame.depth == 1) {
            curr_node = init_region(frame.origin);
        } else {
            curr_node = get_region(frame.node_offset);
        }

        let cell_jump = pow_u32(SUBDIVISION, 1u);
        let cell_size = f32(REGION_SIZE) / f32(cell_jump);

        if (!frame.resumed) {
            frame.resumed = true;
            let origin_grid: vec3<f32> = (frame.origin + ray_dir_offset) / cell_size;
            frame.map_check = vec3<i32>(floor(frame.origin));

            let delta = origin_grid - vec3<f32>(frame.map_check);
            
            frame.ray_length = init_ray_length(ray_dir, ray_unit_step, delta);

            let cell_offset = get_sub_region_offset(frame.map_check, curr_node);

            // approaching new bounding box
            if (frame.step_to_boundary) {
                distance = step(&frame, step, ray_unit_step);
            }
            
        } else {
            distance = step(&frame, step, ray_unit_step);
            if (outside_bounds(frame.map_check, frame.low, frame.high)) { continue; }
        }

        for (var i: u32 = 0u; i < MAX_STEP_COUNT; i++) {
            let cell_offset = get_sub_region_offset(frame.map_check, curr_node);

            if (frame.depth != 4 && cell_offset != 0) {
                let low = frame.map_check * i32(SUBDIVISION);
                let high = low + SIZE_XYZ;

                let new_frame = StackFrame(
                    frame.origin + (ray_dir - ray_dir_offset) * distance * cell_size, // origin
                    vec3<i32>(0, 0, 0),       // map_check
                    vec3<f32>(0.0, 0.0, 0.0), // ray_length

                    low,    // low
                    high,   // high

                    cell_offset,        // node
                    frame.depth + 1,    // depth
                    distance != 0f,     // step to boundary
                    false,              // resumed
                );
                // upload changes to current frame
                stack_update(&stack, frame);
                stack_push(&stack, new_frame);
                break;
            } 
            
            else if (frame.depth == 4 && cell_offset != 0) {
                return true; // hit
            }

            step(&frame, step, ray_unit_step);
            if(outside_bounds(frame.map_check, frame.low, frame.high)) {
                stack_pop(&stack);
                break;
            }
        }
    }
    return false; // miss / return sky box
}

fn init_region(origin: vec3<f32>) -> GpuNode {
    let coord: vec3<i32> = vec3<i32>(floor(origin)) >> vec3<u32>(8u, 8u, 8u);
    let offset = coord - header.base;
    let per_axis = header.end - header.base;

    let final_offset = u32(offset.x + offset.y * per_axis.x + offset.z * per_axis.x * per_axis.z);
    return nodes[final_offset + 1]; // +1 -> 0x0 is NULL
}

fn get_region(offset: u32) -> GpuNode {
    return nodes[offset];
}

fn get_sub_region_offset(map_check: vec3<i32>, node: GpuNode) -> u32 {
    let mask = SUBDIVISION - 1u;
    let coord  = map_check & vec3<i32>(mask);
    // let offset = coord.x + coord.y * SUBDIVISION + coord.z * SUBDIVISION * SUBDIVISION;
    let shift = u32(coord.x) + u32(coord.y << 4u) + u32(coord.z << 16u);
    
    if (shift < 32u) {
        if ((node.mask0 & (1u << shift)) != 0u) {
            let o1 = countOneBits(node.mask0 >> shift);
            let offset = o1 - 1u;

            return node.base + offset;
        }
    } else {
        let shift = shift - 32u;
        if ((node.mask1 & (1u << shift)) != 0u) {
            let o1 = countOneBits(node.mask0);
            let o2 = countOneBits(node.mask1 >> shift);
            let offset = o1 + o2 - 1u;

            return node.base + offset;
        }
    }

    return 0u;
}

fn step(frame: ptr<function, StackFrame>, step: vec3<i32>, ray_unit_step: vec3<f32>) -> f32 {
    var distance: f32 = 0.0;

    if ((*frame).ray_length.x < (*frame).ray_length.y && (*frame).ray_length.x < (*frame).ray_length.z) {
        (*frame).map_check.x += step.x;
        distance = (*frame).ray_length.x;
        (*frame).ray_length.x += ray_unit_step.x;
    } else if ((*frame).ray_length.y < (*frame).ray_length.z) {
        (*frame).map_check.y += step.y;
        distance = (*frame).ray_length.y;
        (*frame).ray_length.y += ray_unit_step.y;
    } else {
        (*frame).map_check.z += step.z;
        distance = (*frame).ray_length.z;
        (*frame).ray_length.z += ray_unit_step.z;
    }

    return distance;
}

fn outside_bounds(map_check: vec3<i32>, low: vec3<i32>, high: vec3<i32>) -> bool {
    return (map_check.x < low.x) || // add - epsilon
           (map_check.y < low.y) || // ----||-----
           (map_check.z < low.z) || // ----||-----
           (map_check.x >= high.x) || // add + epsilon
           (map_check.y >= high.y) || // add + epsilon
           (map_check.z >= high.z);
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