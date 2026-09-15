use bevy::prelude::*;

/// Pathfinding (flow fields / A*), decoupled from movement execution, lands
/// here (`SimSet::Movement` or earlier, `FixedUpdate`).
pub struct PathfindingPlugin;

impl Plugin for PathfindingPlugin {
    fn build(&self, _app: &mut App) {}
}
