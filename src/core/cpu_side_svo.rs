use dot_vox::{load, DotVoxData};

use crate::{
    core::types::Scene,
    gpu::types::{GpuNode, GpuRoot, GpuSceneHeader},
};

use crate::core::types::{Node, Node64};
const BIT_MASK: u8 = 0b0000_0011;
pub struct Loader {
    data: Option<DotVoxData>,
}

impl Loader {
    pub fn new() -> Self {
        Loader { data: None }
    }

    pub fn load_data(&mut self, path: &str) {
        self.data = Some(load(path).unwrap());
    }

    pub fn make_chunk(&mut self) -> Result<Node, ()> {
        if let None = &self.data {
            return Err(());
        }

        let model = &self.data.as_ref().unwrap().models[0];
        let mut chunk = Node::Empty;

        for v in &model.voxels {
            let mut base = 6;
            let mut node = &mut chunk;

            while base > 1 {
                let x = (v.x >> base) & BIT_MASK;
                let y = (v.y >> base) & BIT_MASK;
                let z = (v.z >> base) & BIT_MASK;
                base -= 2;

                let offset = (x + 4 * y + 16 * z) as usize;

                if let Node::Empty = node {
                    *node = Node::Branch(Node64::new());
                }

                if let Node::Branch(ref mut branch) = node {
                    node = &mut *branch.children[offset];
                }
            }

            let bit: u64 = 1 << (v.x & BIT_MASK + 4 * (v.y & BIT_MASK) + 16 * (v.z & BIT_MASK));

            if let Node::Empty = node {
                *node = Node::Leaf(0);
            }

            if let Node::Leaf(vox) = node {
                *vox |= bit;
            }
        }

        Ok(chunk)
    }
}

pub struct Stager {
    pub header: GpuSceneHeader,
    pub gpu_nodes: Vec<GpuNode>,
    colors: Vec<u32>, // change
}

impl Stager {
    pub fn new() -> Self {
        Self {
            header: GpuSceneHeader::default(),
            gpu_nodes: Vec::new(),
            colors: Vec::new(),
        }
    }

    pub fn stage(&mut self, chunks: &Scene, start: (i32, i32, i32), end: (i32, i32, i32)) {
        let mut nodes = Vec::new();

        reserve_roots(&mut nodes, start, end);

        for z in start.2..end.2 {
            for y in start.1..end.1 {
                for x in start.0..end.0 {
                    let offset = root_offset((x, y, z), start, end);
                    self.flatten(chunks.get_chunk((x, y, z)), &mut nodes, offset);
                }
            }
        }

        let header = GpuSceneHeader {
            start: [start.0, start.1, start.2, 0],
            end: [end.0, end.1, end.2, 0],
            size: nodes.len() as u32,
        };
        self.header = header;
        self.gpu_nodes = nodes;
    }

    fn flatten(&self, chunk: Option<&Node>, nodes: &mut Vec<GpuNode>, root_offset: usize) {
        use std::collections::VecDeque;

        if chunk.is_none() {
            return;
        }

        let mut queue = VecDeque::new();

        match chunk.unwrap() {
            Node::Empty => {}
            Node::Branch(branch) => {
                let mut mask: u64 = 0;
                let mut children: Vec<&Node> = Vec::new();
    
                for (i, child) in branch.children.iter().enumerate() {
                    mask |= 1 << i;
                    children.push(child.as_ref());
                }
    
                nodes[root_offset].mask = mask;
                nodes[root_offset].base = nodes.len() as u32;
    
                // reserve space for children
                let base = nodes.len();
                for _ in 0..children.len() {
                    nodes.push(GpuNode::default());
                }
    
                for (i, child) in children.into_iter().enumerate() {
                    queue.push_back((child, i + base));
                }
            }
            Node::Leaf(mask) => {
                nodes[root_offset].mask = *mask;
            }
        }
    
        // BFS traversal
        while let Some((node, index)) = queue.pop_front() {
            match node {
                Node::Empty => {}
                Node::Branch(branch) => {
                    let mut mask: u64 = 0;
                    let mut children: Vec<&Node> = Vec::new();
    
                    for (i, child) in branch.children.iter().enumerate() {
                        mask |= 1 << i;
                        children.push(child.as_ref());
                    }
    
                    let base = nodes.len();
    
                    nodes[index].mask = mask;
                    nodes[index].base = base as u32;
    
                    // reserve space for children
                    for _ in 0..children.len() {
                        nodes.push(GpuNode::default());
                    }
    
                    for (i, child) in children.into_iter().enumerate() {
                        queue.push_back((child, i + base));
                    }
                }
                Node::Leaf(mask) => {
                    nodes[index] = GpuNode::set_leaf(*mask, 10);
                }
            }
        }
    }
}

fn reserve_roots(nodes: &mut Vec<GpuNode>, start: (i32, i32, i32), end: (i32, i32, i32)) {
    for z in start.2..end.2 {
        for y in start.1..end.1 {
            for x in start.0..end.0 {
                nodes.push(GpuNode::default());
            }
        }
    }
}

fn root_offset(coord: (i32, i32, i32), start: (i32, i32, i32), end: (i32, i32, i32)) -> usize {
    let x_size = end.0 - start.0;
    let y_size = end.1 - start.1;

    let offset = coord.0 + coord.1 * x_size + coord.2 * x_size * y_size;
    return offset as usize
}
