use bevy::prelude::*;

/// Combat resolution lands here (`SimSet::Combat`, `FixedUpdate`).
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, _app: &mut App) {}
}
