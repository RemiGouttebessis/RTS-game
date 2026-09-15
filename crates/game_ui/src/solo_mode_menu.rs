use bevy::prelude::*;

use crate::main_menu::MainMenuScreen;
use crate::widgets::{BACKGROUND, DIMMED_TEXT, NORMAL_BUTTON, button_node, fullscreen_menu_node};

#[derive(Component)]
struct SoloModeRoot;

#[derive(Component, Clone, Copy)]
enum SoloModeButton {
    New,
    Back,
}

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(MainMenuScreen::SoloMode), spawn_screen)
        .add_systems(OnExit(MainMenuScreen::SoloMode), despawn_screen)
        .add_systems(
            Update,
            button_actions.run_if(in_state(MainMenuScreen::SoloMode)),
        );
}

fn spawn_screen(mut commands: Commands) {
    commands
        .spawn((
            SoloModeRoot,
            fullscreen_menu_node(),
            BackgroundColor(BACKGROUND),
        ))
        .with_children(|parent| {
            parent.spawn((Text::new("Solo"), TextFont::from_font_size(40.0)));

            // Save/load isn't implemented yet — `game_save` is still an
            // empty stub.
            parent.spawn((
                Text::new("Saves (WIP)"),
                TextFont::from_font_size(22.0),
                TextColor(DIMMED_TEXT),
            ));

            parent
                .spawn((
                    Button,
                    SoloModeButton::New,
                    button_node(),
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_child((Text::new("New"), TextFont::from_font_size(22.0)));

            parent
                .spawn((
                    Button,
                    SoloModeButton::Back,
                    button_node(),
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_child((Text::new("Back"), TextFont::from_font_size(22.0)));
        });
}

fn button_actions(
    buttons: Query<(&Interaction, &SoloModeButton), Changed<Interaction>>,
    mut next_screen: ResMut<NextState<MainMenuScreen>>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            SoloModeButton::New => next_screen.set(MainMenuScreen::WorldGen),
            SoloModeButton::Back => next_screen.set(MainMenuScreen::PlayMode),
        }
    }
}

fn despawn_screen(mut commands: Commands, roots: Query<Entity, With<SoloModeRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}
