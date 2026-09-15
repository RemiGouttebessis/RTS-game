use bevy::prelude::*;

mod units;

pub use units::Unit;

/// Sim-side ECS data: components, resources, events. Zero rendering dependencies —
/// this is what lets the simulation eventually run headless (dedicated server,
/// replays, automated balance testing) without dragging in a renderer.
pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, _app: &mut App) {}
}
