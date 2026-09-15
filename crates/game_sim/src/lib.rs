use bevy::prelude::*;

mod combat;
mod movement;
mod orders;
mod pathfinding;

pub use combat::CombatPlugin;
pub use movement::MovementPlugin;
pub use orders::OrdersPlugin;
pub use pathfinding::PathfindingPlugin;

/// Ordering for sim systems in `FixedUpdate`. Orders must land before movement,
/// movement before combat resolution, combat before cleanup — explicit sets
/// instead of relying on insertion order, since that gets unreadable fast once
/// hundreds of units are interacting.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimSet {
    Input,
    Orders,
    Movement,
    Combat,
    Cleanup,
}

/// Deterministic simulation: movement, combat, pathfinding, orders. Runs on
/// `FixedUpdate`, independent of rendering — presentation (`game_render`) reads
/// this state, never the other way around.
pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MovementPlugin,
            CombatPlugin,
            PathfindingPlugin,
            OrdersPlugin,
        ))
        .configure_sets(
            FixedUpdate,
            (
                SimSet::Input,
                SimSet::Orders,
                SimSet::Movement,
                SimSet::Combat,
                SimSet::Cleanup,
            )
                .chain(),
        );
    }
}
