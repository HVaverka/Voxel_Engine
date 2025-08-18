use std::{array, collections::HashMap};

use bytemuck::{Pod, Zeroable};
use nalgebra::Vector3;

pub struct Camera {
    pos: nalgebra::Vector3<f32>,
    dir: nalgebra::Vector3<f32>,
    up_dir: nalgebra::Vector3<f32>,
    r_dir: nalgebra::Vector3<f32>,

    yaw: f32,
    pitch: f32,
}

impl Camera {
    pub fn new() -> Self {
        let pos = Vector3::from([150.0, 150.0, 8.0 * 200.0] as [f32; 3]);
        let dir = Vector3::from([0.0, 0.0, -1.0] as [f32; 3]);
        let up_dir = Vector3::from([0.0, 1.0, 0.0] as [f32; 3]);
        let r_dir = Vector3::from([1.0, 0.0, 0.0] as [f32; 3]);

        let yaw = 0.0;
        let pitch = 0.0;

        Self {
            pos,
            dir,
            up_dir,
            r_dir,

            yaw,
            pitch,
        }
    }

    pub fn move_cam(&mut self, time: f64, input: &Vector3<f32>) {
        let movement = self.dir * -input.z + self.up_dir * input.y + self.r_dir * input.x;
        let speed: f32 = 50.0;
        self.pos += movement * speed * time as f32;
    }

    pub fn rotate_cam(&mut self, time: f64, dx: f64, dy: f64) {
        let sensitivity = 0.01;
        self.yaw += dx as f32 * sensitivity;
        self.pitch += dy as f32 * sensitivity;

        // Clamp pitch to avoid flipping
        let max_pitch = std::f32::consts::FRAC_PI_2 - 0.01;
        self.pitch = self.pitch.clamp(-max_pitch, max_pitch);

        // Convert yaw/pitch to direction
        self.dir.x = self.yaw.cos() * self.pitch.cos();
        self.dir.y = self.pitch.sin();
        self.dir.z = self.yaw.sin() * self.pitch.cos();
        self.dir = self.dir.normalize();

        // Update right and up vectors
        self.r_dir = self.dir.cross(&Vector3::y_axis()).normalize();
        self.up_dir = self.r_dir.cross(&self.dir).normalize();
    }
    pub fn get_raw(&self) -> ([f32; 4], [f32; 4], [f32; 4], [f32; 4]) {
        let dir = to_arr4(&self.dir);
        let up_dir = to_arr4(&self.up_dir);
        let r_dir = to_arr4(&self.r_dir);

        let pos = to_arr4(&self.pos);

        (pos, dir, up_dir, r_dir)
    }
}

fn to_arr4(v3: &Vector3<f32>) -> [f32; 4] {
    [v3.x, v3.y, v3.z, 0.0]
}

// Cpu side chunk representation
pub struct Node64 {
    pub children: [Box<Node>; 64],
}

impl Node64 {
    pub fn new() -> Self {
        Self {
            children: array::from_fn(|_| Box::new(Node::Empty)),
        }
    }
}
pub enum Node {
    Empty,
    Branch(Node64),
    Leaf(u64),
}
pub struct Scene {
    world: HashMap<(i32, i32, i32), Node>,
    world_changed: bool,
}

impl Scene {
    pub fn new() -> Self {
        let world = HashMap::new();

        Self {
            world,
            world_changed: true,
        }
    }

    pub fn add_chunk(&mut self, root: Node, coords: (i32, i32, i32)) {
        self.world.insert(coords, root);
        self.world_changed = true;
    }

    pub fn world_changed(&self) -> bool {
        self.world_changed
    }

    pub fn get_chunk(&self, coord: (i32, i32, i32)) -> Option<&Node> {
        self.world.get(&coord)
    }

    pub fn reset_changed(&mut self) {
        self.world_changed = false;
    }
}

fn fill_hashmap(world: &mut HashMap<(i32, i32, i32), Node>) {
    let mut start = (-4, -4, -4);
    let mut end = (4, 4, 4);

    for z in start.2..end.2 {
        for y in start.1..end.1 {
            for x in start.0..end.0 {
                world.insert((x, y, z), Node::Empty);
            }
        }
    }
}
