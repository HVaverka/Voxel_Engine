use std::collections::HashMap;

use winit::keyboard::KeyCode;

use crate::core::settings;

pub struct Settings {
    key_bindings: HashMap<Action, KeyCode>,
    field_of_view: f64,
}

impl Default for Settings {
    fn default() -> Self {
        use Action::*;
        use KeyCode::*;

        let mut key_bindings = HashMap::new();

        key_bindings.insert(Forward, KeyW);
        key_bindings.insert(Backwards, KeyS);
        key_bindings.insert(Left, KeyA);
        key_bindings.insert(Right, KeyD);
        key_bindings.insert(Up, Space);
        key_bindings.insert(Down, ShiftLeft);

        key_bindings.insert(GrabCursor, Escape);

        Settings {
            key_bindings,
            field_of_view: 70.0,
        }
    }
}

impl Settings {
    pub fn binding(&self, action: Action) -> Option<&KeyCode> {
        self.key_bindings.get(&action)
    }

    pub fn field_of_view(&self) -> f64 {
        self.field_of_view
    }
}

#[derive(PartialEq, Eq, Hash)]
pub enum Action {
    Forward,
    Backwards,
    Left,
    Right,

    Up,
    Down,

    GrabCursor,
}
