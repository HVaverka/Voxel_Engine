use nalgebra::{distance_squared, Vector3};
use std::collections::HashMap;
use wgpu::rwh::RawDisplayHandle;

const CHUNK_WIDTH_IN_BLOCKS: i32 = 16;
const BLOCK_WIDTH_IN_VOXELS: i32 = 16;
const CHUNK_WIDTH: i32 = CHUNK_WIDTH_IN_BLOCKS * BLOCK_WIDTH_IN_VOXELS;
const MAX_DEPTH: u8 = 7; // log2(256) = 8 (but -1 -> last level is voxel not tree)

pub struct World {
    seed: u64,
    chunks: HashMap<(Vector3<i32>), Box<OctreeNode>>,
}

impl World {
    pub fn new(world_seed: Option<u64>) -> World {
        let seed = match world_seed {
            Some(ws) => ws,
            None => 0,
        };
        let chunks: HashMap<Vector3<i32>, Box<OctreeNode>> = HashMap::new();

        Self { seed, chunks }
    }
    pub fn load_unload_chunk(&mut self, origin: Vector3<i32>, radius: i32) {
        for x in origin.x - radius..=origin.x + radius {
            for y in origin.y - radius..=origin.y + radius {
                for z in origin.z - radius..=origin.z + radius {
                    if self.chunks.contains_key(&Vector3::new(x, y, z)) {}
                }
            }
        }
    }
    pub fn load_chunks(&mut self, origin: Vector3<i32>, radius: i32) {
        let radius_squared = radius * radius;
        let mut new_chunks_coords = Vec::new();
        (-radius..=radius)
            .flat_map(|dx| (-radius..=radius).flat_map(move |dy| (-radius..=radius)))
            .filter(|pos| {
                let dx = pos.x - origin.x;
                let dy = pos.y - origin.y;
                let dz = pos.z - origin.z;
                let distance_squared = dx * dx + dy * dy + dz * dz;

                distance_squared <= radius_squared
            })
            .for_each(|pos| {
                if !self.chunks.contains_key(&pos) {
                    new_chunks_coords.push(pos);
                }
            });

        new_chunks_coords.into_iter().for_each(|pos| {
            self.chunks.insert(pos, Box::new(Self::generate_chunk(pos)));
        });
    }
    pub fn unload_chunks(&mut self, origin: Vector3<i32>, radius: i32) {
        // rewrite into extract_if when possible

        let radius_squared = radius * radius;
        let mut removed_chunks = Vec::new();

        // Collect positions of chunks to be removed
        let to_remove: Vec<Vector3<i32>> = self
            .chunks
            .iter()
            .filter(|(pos, _)| {
                let dx = pos.x - origin.x;
                let dy = pos.y - origin.y;
                let dz = pos.z - origin.z;
                let distance_squared = dx * dx + dy * dy + dz * dz;

                distance_squared > radius_squared // Mark for removal
            })
            .map(|(pos, _)| *pos) // Extract positions
            .collect();

        // Remove from HashMap and store removed elements
        for pos in to_remove {
            if let Some(chunk) = self.chunks.remove(&pos) {
                removed_chunks.push((pos, chunk));
            }
        }

        // add different structure to record changes
        // Self::save_changes(removed_chunks);
    }
    fn save_changes(chunks: Vec<(Vector3<i32>, OctreeNode)>) {}
    fn generate_chunk(pos: Vector3<i32>) -> OctreeNode {
        let sphere_center = pos.cast::<f32>() + Vector3::new(7.5, 7.5, 7.5);
        let radius = 4f32;

        let mut chunk = OctreeNode::new(pos, 0);
        let mut voxel_coordinates = vec![];
        for x in 0..32 {
            for y in 0..32 {
                for z in 0..32 {
                    let voxel_pos = Vector3::new(x, y, z);
                    if ((pos + voxel_pos).cast::<f32>() - sphere_center).norm_squared() > radius {
                        continue;
                    }
                    // is inside the sphere:
                    voxel_coordinates.push(voxel_pos);
                }
            }
        }
        chunk.fill_tree(&voxel_coordinates);
        return chunk;
    }
}

struct OctreeNode {
    pos: Vector3<i32>,
    depth: u8,
    sub_tree: [Node; 8],
}

impl OctreeNode {
    pub fn new(pos: Vector3<i32>, depth: u8) -> OctreeNode {
        Self {
            pos: pos,
            depth: 0,
            sub_tree: [
                Node::Empty,
                Node::Empty,
                Node::Empty,
                Node::Empty,
                Node::Empty,
                Node::Empty,
                Node::Empty,
                Node::Empty,
            ],
        }
    }
    pub fn fill_tree(&mut self, v_coord: &Vec<Vector3<i32>>) {
        for v in v_coord {
            self.rec_fill(&v);
        }
    }
    fn rec_fill(&mut self, v: &Vector3<i32>) {
        let mut index = 0;
        if v.x >= CHUNK_WIDTH / 2i32.pow(self.depth as u32 + 1) {
            index |= 1
        }
        if v.y >= CHUNK_WIDTH / 2i32.pow(self.depth as u32 + 1) {
            index |= 2
        }
        if v.z >= CHUNK_WIDTH / 2i32.pow(self.depth as u32 + 1) {
            index |= 4
        }

        self.sub_tree[index] = match &mut self.sub_tree[index] {
            Node::Empty if self.depth < MAX_DEPTH => {
                let mut octant = Self::create_octant(self.pos, self.depth, index);
                octant.rec_fill(v);
                Node::SubTree(Box::new(octant))
            }
            Node::Empty => Node::Full(Voxel {
                color: Vector3::new(0u8, 0u8, 0u8),
            }),
            Node::SubTree(octant) => {
                octant.rec_fill(v);
                return;
            }
            Node::Full(_) => return,
        };
    }
    fn create_octant(parent_pos: Vector3<i32>, parent_depth: u8, octant_i: usize) -> OctreeNode {
        let offset = CHUNK_WIDTH / 2i32.pow(parent_depth as u32 + 1);
        let x = parent_pos.x + if octant_i & 1 != 0 { offset } else { 0 };
        let y = parent_pos.z + if octant_i & 2 != 0 { offset } else { 0 };
        let z = parent_pos.y + if octant_i & 4 != 0 { offset } else { 0 };
        let new_pos = Vector3::new(x, y, z);

        return OctreeNode::new(new_pos, parent_depth - 1);
    }
}

enum Node {
    Empty,
    SubTree(Box<OctreeNode>),
    Full(Voxel),
}

struct Voxel {
    color: Vector3<u8>,
}
