use bevy::prelude::*;

mod keybinds_menu;
mod main_menu;
mod pause_menu;
mod widgets;

/// Main menu, ESC/pause menu, and its keybind-rebind screen live here today.
/// HUD, selection box, minimap land here too, eventually.
pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            widgets::plugin,
            main_menu::plugin,
            pause_menu::plugin,
            keybinds_menu::plugin,
        ));
    }
}
