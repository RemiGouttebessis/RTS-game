use bevy::prelude::*;

mod keybinds_menu;
mod main_menu;
mod pause_menu;
mod play_mode_menu;
mod solo_mode_menu;
mod widgets;
mod worldgen_menu;

/// Main menu (and its Play → Solo → New flow, ending at the world generator
/// preview), ESC/pause menu, and its keybind-rebind screen live here today.
/// HUD, selection box, minimap land here too, eventually.
pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            widgets::plugin,
            main_menu::plugin,
            play_mode_menu::plugin,
            solo_mode_menu::plugin,
            worldgen_menu::plugin,
            pause_menu::plugin,
            keybinds_menu::plugin,
        ));
    }
}
