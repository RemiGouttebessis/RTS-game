use bevy::prelude::*;

use crate::InputSet;

/// Fired when the player presses Escape. `game_ui` decides what that means
/// (open/close the pause menu, back out of a submenu) based on what's
/// currently showing — this action is just "the player pressed Escape".
#[derive(Message, Default, Clone, Copy)]
pub struct TogglePauseMenu;

pub(crate) fn plugin(app: &mut App) {
    app.add_message::<TogglePauseMenu>()
        .add_systems(Update, read_toggle.in_set(InputSet));
}

fn read_toggle(keys: Res<ButtonInput<KeyCode>>, mut toggle: MessageWriter<TogglePauseMenu>) {
    if keys.just_pressed(KeyCode::Escape) {
        toggle.write(TogglePauseMenu);
    }
}
