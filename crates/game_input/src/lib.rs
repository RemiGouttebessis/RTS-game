use bevy::prelude::*;

/// Translates raw player input into game commands/orders for `game_sim` to
/// consume (e.g. via `leafwing-input-manager`). No systems yet.
pub struct GameInputPlugin;

impl Plugin for GameInputPlugin {
    fn build(&self, _app: &mut App) {}
}
