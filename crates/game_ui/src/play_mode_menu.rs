use bevy::prelude::*;

use crate::main_menu::MainMenuScreen;
use crate::widgets::{BACKGROUND, DIMMED_TEXT, NORMAL_BUTTON, button_node, fullscreen_menu_node};

#[derive(Component)]
struct PlayModeRoot;

#[derive(Component, Clone, Copy)]
enum PlayModeButton {
    Solo,
    Back,
}

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(MainMenuScreen::PlayMode), spawn_screen)
        .add_systems(OnExit(MainMenuScreen::PlayMode), despawn_screen)
        .add_systems(
            Update,
            button_actions.run_if(in_state(MainMenuScreen::PlayMode)),
        );
}

fn spawn_screen(mut commands: Commands) {
    commands
        .spawn((
            PlayModeRoot,
            fullscreen_menu_node(),
            BackgroundColor(BACKGROUND),
        ))
        .with_children(|parent| {
            parent.spawn((Text::new("Play"), TextFont::from_font_size(40.0)));

            parent
                .spawn((
                    Button,
                    PlayModeButton::Solo,
                    button_node(),
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_child((Text::new("Solo"), TextFont::from_font_size(22.0)));

            // Multiplayer isn't implemented yet.
            parent.spawn((
                Text::new("Multiplayer (WIP)"),
                TextFont::from_font_size(22.0),
                TextColor(DIMMED_TEXT),
            ));

            parent
                .spawn((
                    Button,
                    PlayModeButton::Back,
                    button_node(),
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_child((Text::new("Back"), TextFont::from_font_size(22.0)));
        });
}

fn button_actions(
    buttons: Query<(&Interaction, &PlayModeButton), Changed<Interaction>>,
    mut next_screen: ResMut<NextState<MainMenuScreen>>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            PlayModeButton::Solo => next_screen.set(MainMenuScreen::SoloMode),
            PlayModeButton::Back => next_screen.set(MainMenuScreen::Root),
        }
    }
}

fn despawn_screen(mut commands: Commands, roots: Query<Entity, With<PlayModeRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}
