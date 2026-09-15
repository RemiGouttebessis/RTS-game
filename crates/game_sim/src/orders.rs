use bevy::prelude::*;

/// Turns player/AI commands (move orders, attack orders, ...) into sim state
/// lands here (`SimSet::Orders`, `FixedUpdate`).
pub struct OrdersPlugin;

impl Plugin for OrdersPlugin {
    fn build(&self, _app: &mut App) {}
}
