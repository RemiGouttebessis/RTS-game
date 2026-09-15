use bevy::prelude::*;
use game_core::{GameState, PauseState};
use game_input::{InputSet, TogglePauseMenu};

use crate::keybinds_menu::RebindingAction;
use crate::widgets::{BACKGROUND, NORMAL_BUTTON, button_node, fullscreen_menu_node};

/// Which pause-menu screen is showing. Only exists while `PauseState::Paused`
/// (a sub-state of a sub-state) — resets to `Root` every time the menu opens.
#[derive(SubStates, Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
#[source(PauseState = PauseState::Paused)]
pub(crate) enum PauseMenuScreen {
    #[default]
    Root,
    Keybinds,
}

#[derive(Component)]
struct PauseRoot;

#[derive(Component, Clone, Copy)]
enum PauseButton {
    Resume,
    Settings,
    QuitToMainMenu,
}

pub(crate) fn plugin(app: &mut App) {
    app.add_sub_state::<PauseMenuScreen>()
        .add_systems(
            Update,
            handle_escape
                .after(InputSet)
                .run_if(in_state(GameState::InGame)),
        )
        .add_systems(OnEnter(PauseMenuScreen::Root), spawn_root)
        .add_systems(OnExit(PauseMenuScreen::Root), despawn_root)
        .add_systems(
            Update,
            button_actions.run_if(in_state(PauseMenuScreen::Root)),
        );
}

/// Escape is context-sensitive: cancel an in-progress keybind capture first,
/// then back out of the keybinds screen, then close/open the pause menu.
fn handle_escape(
    mut toggled: MessageReader<TogglePauseMenu>,
    mut rebinding: ResMut<RebindingAction>,
    screen: Option<Res<State<PauseMenuScreen>>>,
    mut next_screen: ResMut<NextState<PauseMenuScreen>>,
    pause_state: Res<State<PauseState>>,
    mut next_pause: ResMut<NextState<PauseState>>,
) {
    if toggled.read().count() == 0 {
        return;
    }

    if rebinding.0.take().is_some() {
        return;
    }

    match screen.map(|screen| *screen.get()) {
        Some(PauseMenuScreen::Keybinds) => next_screen.set(PauseMenuScreen::Root),
        Some(PauseMenuScreen::Root) | None => {
            next_pause.set(match pause_state.get() {
                PauseState::Running => PauseState::Paused,
                PauseState::Paused => PauseState::Running,
            });
        }
    }
}

fn spawn_root(mut commands: Commands) {
    commands
        .spawn((
            PauseRoot,
            fullscreen_menu_node(),
            BackgroundColor(BACKGROUND),
        ))
        .with_children(|parent| {
            parent.spawn((Text::new("Paused"), TextFont::from_font_size(40.0)));

            for (action, label) in [
                (PauseButton::Resume, "Resume"),
                (PauseButton::Settings, "Settings"),
                (PauseButton::QuitToMainMenu, "Quit to Main Menu"),
            ] {
                parent
                    .spawn((
                        Button,
                        action,
                        button_node(),
                        BackgroundColor(NORMAL_BUTTON),
                    ))
                    .with_child((Text::new(label), TextFont::from_font_size(22.0)));
            }
        });
}

fn despawn_root(mut commands: Commands, roots: Query<Entity, With<PauseRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}

fn button_actions(
    buttons: Query<(&Interaction, &PauseButton), Changed<Interaction>>,
    mut next_screen: ResMut<NextState<PauseMenuScreen>>,
    mut next_pause: ResMut<NextState<PauseState>>,
    mut next_game_state: ResMut<NextState<GameState>>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            PauseButton::Resume => next_pause.set(PauseState::Running),
            PauseButton::Settings => next_screen.set(PauseMenuScreen::Keybinds),
            PauseButton::QuitToMainMenu => next_game_state.set(GameState::MainMenu),
        }
    }
}
