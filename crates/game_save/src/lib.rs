use bevy::prelude::*;

/// Save/load game state (serde). No systems yet.
pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, _app: &mut App) {}
}
