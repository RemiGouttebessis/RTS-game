use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Rebindable keys. Arrow keys always work as a fixed fallback for camera pan
/// (see `game_input`) and aren't part of this — these are the user-remappable
/// primary binds, the ones a future keybind UI would edit.
#[derive(Resource, Serialize, Deserialize, Clone, Copy, Debug)]
pub struct KeyBindings {
    pub pan_north: KeyCode,
    pub pan_south: KeyCode,
    pub pan_west: KeyCode,
    pub pan_east: KeyCode,
    pub toggle_debug_overlay: KeyCode,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            pan_north: KeyCode::KeyW,
            pan_south: KeyCode::KeyS,
            pan_west: KeyCode::KeyA,
            pan_east: KeyCode::KeyD,
            toggle_debug_overlay: KeyCode::F3,
        }
    }
}
