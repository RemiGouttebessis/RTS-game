use bevy::prelude::*;
use game_config::KeyBindings;

use crate::InputSet;

/// Fired when the player toggles the in-game performance overlay.
#[derive(Message, Default, Clone, Copy)]
pub struct ToggleDebugOverlay;

pub(crate) fn plugin(app: &mut App) {
    app.add_message::<ToggleDebugOverlay>()
        .add_systems(Update, read_toggle.in_set(InputSet));
}

fn read_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    binds: Res<KeyBindings>,
    mut toggle: MessageWriter<ToggleDebugOverlay>,
) {
    if keys.just_pressed(binds.toggle_debug_overlay) {
        toggle.write(ToggleDebugOverlay);
    }
}
