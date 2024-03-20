use std::collections::HashSet;

use device_query::{DeviceQuery, DeviceState, Keycode};

pub trait KeyDetector {
    fn new() -> Self;
    fn get_pressed_keys(&self) -> Vec<Keycode>;
    fn get_released_keys(&self) -> Vec<Keycode>;
    fn update_keys(&mut self);
}

pub struct DeviceStateKeyDetector {
    device_state: DeviceState,
    keys: HashSet<Keycode>,
    prev_keys: HashSet<Keycode>,
}

impl KeyDetector for DeviceStateKeyDetector {
    fn new() -> Self {
        let device_state = DeviceState::new();
        let keys = device_state.get_keys().into_iter().collect();
        Self {
            device_state,
            keys,
            prev_keys: HashSet::new(),
        }
    }

    fn get_pressed_keys(&self) -> Vec<Keycode> {
        self.keys.difference(&self.prev_keys).cloned().collect()
    }

    fn get_released_keys(&self) -> Vec<Keycode> {
        self.prev_keys.difference(&self.keys).cloned().collect()
    }

    fn update_keys(&mut self) {
        self.prev_keys = self.keys.clone();
        self.keys = self.device_state.get_keys().into_iter().collect();
    }
}
