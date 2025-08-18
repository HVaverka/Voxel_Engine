use std::{collections::hash_set::HashSet, time::Instant};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta},
    keyboard::{KeyCode, PhysicalKey::Code},
};

#[derive(Default, Debug)]
pub struct InputState {
    key_released: HashSet<KeyCode>,
    key_pressed: HashSet<KeyCode>,

    mouse_released: HashSet<MouseButton>,
    mouse_pressed: HashSet<MouseButton>,

    mouse_position: (f64, f64),
    mouse_delta: (f64, f64),

    scroll_delta_y: f32,
    scroll_delta_x: f32,

    cursor_state: CursorState,
}

impl InputState {
    pub fn update_key(&mut self, event: KeyEvent, is_synthetic: bool) {
        let key = match event.physical_key {
            Code(key_code) => key_code,
            _ => return,
        };

        match event.state {
            ElementState::Pressed => {
                if !is_synthetic {
                    self.key_released.remove(&key);
                    self.key_pressed.insert(key);
                }
            }
            ElementState::Released => {
                self.key_pressed.remove(&key);
                self.key_released.insert(key);
            }
        }
    }

    pub fn update_mouse_button(&mut self, state: ElementState, button: MouseButton) {
        match state {
            ElementState::Pressed => {
                self.mouse_released.remove(&button);
                self.mouse_pressed.insert(button);
            }
            ElementState::Released => {
                self.mouse_pressed.remove(&button);
                self.mouse_released.insert(button);
            }
        }
    }

    pub fn update_mouse_position(&mut self, pos: PhysicalPosition<f64>) {
        self.mouse_position = (pos.x, pos.y);
    }

    pub fn update_mouse_delta(&mut self, delta: (f64, f64)) {
        self.mouse_delta.0 += delta.0;
        self.mouse_delta.1 += delta.1;
    }

    pub fn update_mouse_scroll(&mut self, delta: MouseScrollDelta) {
        match delta {
            MouseScrollDelta::LineDelta(x, y) => {
                self.scroll_delta_x = x;
                self.scroll_delta_y = y;
            }
            MouseScrollDelta::PixelDelta(pos) => {
                self.scroll_delta_x = pos.x as f32;
                self.scroll_delta_y = pos.y as f32;
            }
        }
    }

    pub fn is_pressed(&self, key: Option<&KeyCode>) -> bool {
        if let Some(key_code) = key {
            self.key_pressed.contains(key_code)
        } else {
            false
        }
    }

    pub fn consume_key(&mut self, key: Option<&KeyCode>) -> bool {
        match key {
            Some(key_code) => {
                if self.key_pressed.contains(key_code) {
                    self.key_pressed.remove(key_code);
                    self.key_released.insert(*key_code);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn get_mouse_delta(&mut self) -> (f64, f64) {
        let current_delta = self.mouse_delta;
        self.mouse_delta = (0.0, 0.0);
        current_delta
    }

    pub fn cursor_state(&self) -> &CursorState {
        &self.cursor_state
    }

    pub fn cursor_entered(&mut self) {
        self.cursor_state = CursorState::Entered;
    }

    pub fn cursor_left(&mut self) {
        self.cursor_state = CursorState::Left;
    }

    pub fn cursor_locked(&mut self) {
        self.cursor_state = CursorState::Locked;
    }
}

#[derive(Debug)]
pub enum CursorState {
    Undefined,
    Entered,
    Left,
    Locked,
}

impl Default for CursorState {
    fn default() -> Self {
        CursorState::Undefined
    }
}

enum KeyState {
    None,
    Pressed(Instant),
    Released(Instant),
}
