use nalgebra::Vector3;
pub struct Player {
    pos: Vector3<f32>,
    dir: Vector3<f32>,

    inventory: Inventory,
}

impl Player {
    pub fn new() -> Player {
        Self {
            pos: Vector3::new(0.0, 0.0, 0.0),
            dir: Vector3::new(0.0, 0.0, 0.0),
            inventory: Inventory {},
        }
    }
}

struct Inventory {}
