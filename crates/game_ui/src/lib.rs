use bevy::prelude::*;

mod keybinds_menu;
mod loading_menu;
mod main_menu;
mod new_game_menu;
mod pause_menu;
mod play_mode_menu;
mod players_menu;
mod solo_mode_menu;
mod widgets;
mod worldgen_menu;

/// Main menu (and its Play → Solo → New flow, landing on the tabbed New
/// Game screen — World generator, Players), ESC/pause menu, and its
/// keybind-rebind screen live here today. HUD, selection box, minimap land
/// here too, eventually.
pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            widgets::plugin,
            main_menu::plugin,
            play_mode_menu::plugin,
            solo_mode_menu::plugin,
            new_game_menu::plugin,
            worldgen_menu::plugin,
            players_menu::plugin,
            loading_menu::plugin,
            pause_menu::plugin,
            keybinds_menu::plugin,
        ));
    }
}
