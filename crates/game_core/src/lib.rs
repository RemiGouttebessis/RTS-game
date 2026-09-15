use bevy::prelude::*;

mod state;
mod units;

pub use state::{GameState, PauseState};
pub use units::Unit;

/// Sim-side ECS data: components, resources, events. Zero rendering dependencies —
/// this is what lets the simulation eventually run headless (dedicated server,
/// replays, automated balance testing) without dragging in a renderer.
pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>().add_sub_state::<PauseState>();
    }
}
