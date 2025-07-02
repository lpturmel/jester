use glam::Vec2;
use winit::{event::MouseButton, keyboard::KeyCode};

#[derive(Default, Clone, Debug)]
pub struct InputState {
    pressed: smallvec::SmallVec<[KeyCode; 32]>,
    just_pressed: smallvec::SmallVec<[KeyCode; 32]>,
    just_released: smallvec::SmallVec<[KeyCode; 32]>,

    mouse_pos: Vec2,
    mouse_pressed: smallvec::SmallVec<[MouseButton; 8]>,
    mouse_just_pressed: smallvec::SmallVec<[MouseButton; 8]>,
    mouse_just_released: smallvec::SmallVec<[MouseButton; 8]>,
}

impl InputState {
    pub fn key_pressed(&self, k: KeyCode) -> bool {
        self.pressed.contains(&k)
    }
    pub fn just_pressed(&self, k: KeyCode) -> bool {
        self.just_pressed.contains(&k)
    }
    pub fn just_released(&self, k: KeyCode) -> bool {
        self.just_released.contains(&k)
    }

    pub fn mouse_pressed(&self, b: MouseButton) -> bool {
        self.mouse_pressed.contains(&b)
    }
    pub fn mouse_pos(&self) -> Vec2 {
        self.mouse_pos
    }

    pub fn begin_frame(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
        self.mouse_just_pressed.clear();
        self.mouse_just_released.clear();
    }
    pub fn set_mouse_pos(&mut self, pos: Vec2) {
        self.mouse_pos = pos;
    }
    pub fn set_key_down(&mut self, k: KeyCode, down: bool) {
        match down {
            true if !self.pressed.contains(&k) => {
                self.pressed.push(k);
                self.just_pressed.push(k);
            }
            false if self.pressed.contains(&k) => {
                self.pressed.retain(|x| *x != k);
                self.just_released.push(k);
            }
            _ => {}
        }
    }
    pub fn set_mouse_btn(&mut self, b: MouseButton, down: bool) {
        match down {
            true if !self.mouse_pressed.contains(&b) => {
                self.mouse_pressed.push(b);
                self.mouse_just_pressed.push(b);
            }
            false if self.mouse_pressed.contains(&b) => {
                self.mouse_pressed.retain(|x| *x != b);
                self.mouse_just_released.push(b);
            }
            _ => {}
        }
    }
}
