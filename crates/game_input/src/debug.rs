use bevy::prelude::*;

use crate::InputSet;

/// Fired when the player toggles the in-game performance overlay (F3).
#[derive(Message, Default, Clone, Copy)]
pub struct ToggleDebugOverlay;

pub(crate) fn plugin(app: &mut App) {
    app.add_message::<ToggleDebugOverlay>()
        .add_systems(Update, read_toggle.in_set(InputSet));
}

fn read_toggle(keys: Res<ButtonInput<KeyCode>>, mut toggle: MessageWriter<ToggleDebugOverlay>) {
    if keys.just_pressed(KeyCode::F3) {
        toggle.write(ToggleDebugOverlay);
    }
}
