use bevy::prelude::*;

/// Unit movement execution lands here (`SimSet::Movement`, `FixedUpdate`).
pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, _app: &mut App) {}
}
