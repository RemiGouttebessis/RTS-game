use bevy::prelude::*;
use game_core::GameState;

use crate::widgets::{BACKGROUND, NORMAL_BUTTON, button_node, fullscreen_menu_node};

/// Which main-menu screen is showing. Resets to `Root` every time the menu
/// (re)opens (i.e. every time `GameState` becomes `MainMenu`) — same pattern
/// as `pause_menu::PauseMenuScreen`.
#[derive(SubStates, Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
#[source(GameState = GameState::MainMenu)]
pub(crate) enum MainMenuScreen {
    #[default]
    Root,
    PlayMode,
    SoloMode,
    /// The New Game setup screen — a tab bar (`new_game_menu::NewGameTab`:
    /// World / Players) over a shared content area, not a single screen
    /// despite the name surviving from when it was just the world-gen
    /// preview.
    NewGame,
}

/// Root of the main menu UI tree; despawned (with all children) on exit.
#[derive(Component)]
struct MainMenuRoot;

#[derive(Component, Clone, Copy)]
enum MenuButton {
    Play,
    Quit,
}

pub(crate) fn plugin(app: &mut App) {
    app.add_sub_state::<MainMenuScreen>()
        .add_systems(OnEnter(MainMenuScreen::Root), spawn_menu)
        .add_systems(OnExit(MainMenuScreen::Root), despawn_menu)
        .add_systems(
            Update,
            button_actions.run_if(in_state(MainMenuScreen::Root)),
        );
}

fn spawn_menu(mut commands: Commands) {
    commands
        .spawn((
            MainMenuRoot,
            fullscreen_menu_node(),
            BackgroundColor(BACKGROUND),
        ))
        .with_children(|parent| {
            parent.spawn((Text::new("RTS Game"), TextFont::from_font_size(48.0)));

            for (action, label) in [(MenuButton::Play, "Play"), (MenuButton::Quit, "Quit")] {
                parent
                    .spawn((
                        Button,
                        action,
                        button_node(),
                        BackgroundColor(NORMAL_BUTTON),
                    ))
                    .with_child((Text::new(label), TextFont::from_font_size(24.0)));
            }
        });
}

fn button_actions(
    buttons: Query<(&Interaction, &MenuButton), Changed<Interaction>>,
    mut next_screen: ResMut<NextState<MainMenuScreen>>,
    mut exit: MessageWriter<AppExit>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            MenuButton::Play => next_screen.set(MainMenuScreen::PlayMode),
            MenuButton::Quit => {
                exit.write(AppExit::Success);
            }
        }
    }
}

fn despawn_menu(mut commands: Commands, roots: Query<Entity, With<MainMenuRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}
