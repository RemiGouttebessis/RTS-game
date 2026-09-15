use bevy::prelude::*;
use game_config::{CameraSettings, GraphicsSettings, KeyBindings, Settings};

use crate::pause_menu::PauseMenuScreen;
use crate::widgets::{BACKGROUND, NORMAL_BUTTON, button_node, fullscreen_menu_node};

/// Action currently waiting for a key press to rebind, if any. Cleared by a
/// successful rebind or by Escape (see `pause_menu::handle_escape`).
#[derive(Resource, Default)]
pub(crate) struct RebindingAction(pub(crate) Option<BindAction>);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum BindAction {
    PanNorth,
    PanSouth,
    PanWest,
    PanEast,
    ToggleDebugOverlay,
}

impl BindAction {
    const ALL: [BindAction; 5] = [
        BindAction::PanNorth,
        BindAction::PanSouth,
        BindAction::PanWest,
        BindAction::PanEast,
        BindAction::ToggleDebugOverlay,
    ];

    fn label(self) -> &'static str {
        match self {
            BindAction::PanNorth => "Pan North",
            BindAction::PanSouth => "Pan South",
            BindAction::PanWest => "Pan West",
            BindAction::PanEast => "Pan East",
            BindAction::ToggleDebugOverlay => "Toggle Performance Overlay",
        }
    }

    fn get(self, binds: &KeyBindings) -> KeyCode {
        match self {
            BindAction::PanNorth => binds.pan_north,
            BindAction::PanSouth => binds.pan_south,
            BindAction::PanWest => binds.pan_west,
            BindAction::PanEast => binds.pan_east,
            BindAction::ToggleDebugOverlay => binds.toggle_debug_overlay,
        }
    }

    fn set(self, binds: &mut KeyBindings, key: KeyCode) {
        match self {
            BindAction::PanNorth => binds.pan_north = key,
            BindAction::PanSouth => binds.pan_south = key,
            BindAction::PanWest => binds.pan_west = key,
            BindAction::PanEast => binds.pan_east = key,
            BindAction::ToggleDebugOverlay => binds.toggle_debug_overlay = key,
        }
    }
}

#[derive(Component)]
struct KeybindsRoot;

#[derive(Component)]
struct KeybindsRows;

#[derive(Component)]
struct RebindHint;

#[derive(Component, Clone, Copy)]
enum KeybindsButton {
    Rebind(BindAction),
    Back,
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<RebindingAction>()
        .add_systems(OnEnter(PauseMenuScreen::Keybinds), spawn_screen)
        .add_systems(OnExit(PauseMenuScreen::Keybinds), despawn_screen)
        .add_systems(
            Update,
            (button_actions, capture_rebind, update_rebind_hint)
                .chain()
                .run_if(in_state(PauseMenuScreen::Keybinds)),
        );
}

fn spawn_screen(mut commands: Commands, binds: Res<KeyBindings>) {
    commands
        .spawn((
            KeybindsRoot,
            fullscreen_menu_node(),
            BackgroundColor(BACKGROUND),
        ))
        .with_children(|parent| {
            parent.spawn((Text::new("Keybinds"), TextFont::from_font_size(36.0)));
            parent.spawn((RebindHint, Text::new(""), TextFont::from_font_size(16.0)));

            parent
                .spawn((
                    KeybindsRows,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        align_items: AlignItems::Center,
                        margin: UiRect::vertical(Val::Px(8.0)),
                        ..default()
                    },
                ))
                .with_children(|rows| spawn_rows(rows, &binds));

            parent
                .spawn((
                    Button,
                    KeybindsButton::Back,
                    button_node(),
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_child((Text::new("Back"), TextFont::from_font_size(22.0)));
        });
}

fn spawn_rows(rows: &mut ChildSpawnerCommands, binds: &KeyBindings) {
    for action in BindAction::ALL {
        rows.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(12.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(action.label()),
                TextFont::from_font_size(18.0),
                Node {
                    width: Val::Px(220.0),
                    ..default()
                },
            ));
            row.spawn((
                Button,
                KeybindsButton::Rebind(action),
                Node {
                    width: Val::Px(140.0),
                    height: Val::Px(40.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(NORMAL_BUTTON),
            ))
            .with_child((
                Text::new(format!("{:?}", action.get(binds))),
                TextFont::from_font_size(16.0),
            ));
        });
    }
}

fn despawn_screen(mut commands: Commands, roots: Query<Entity, With<KeybindsRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}

fn button_actions(
    buttons: Query<(&Interaction, &KeybindsButton), Changed<Interaction>>,
    mut rebinding: ResMut<RebindingAction>,
    mut next_screen: ResMut<NextState<PauseMenuScreen>>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            KeybindsButton::Rebind(bind_action) => rebinding.0 = Some(*bind_action),
            KeybindsButton::Back => next_screen.set(PauseMenuScreen::Root),
        }
    }
}

fn capture_rebind(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut rebinding: ResMut<RebindingAction>,
    mut binds: ResMut<KeyBindings>,
    graphics: Res<GraphicsSettings>,
    camera: Res<CameraSettings>,
    rows: Query<Entity, With<KeybindsRows>>,
) {
    let Some(action) = rebinding.0 else {
        return;
    };

    // Escape is handled by `pause_menu::handle_escape` (cancels the rebind);
    // ignore it here so the two systems don't fight over the same keypress.
    let Some(&key) = keys.get_just_pressed().find(|key| **key != KeyCode::Escape) else {
        return;
    };

    action.set(&mut binds, key);
    rebinding.0 = None;

    game_config::save(&Settings {
        graphics: *graphics,
        camera: *camera,
        keybinds: *binds,
    });

    for entity in &rows {
        commands.entity(entity).despawn_children();
        commands
            .entity(entity)
            .with_children(|rows| spawn_rows(rows, &binds));
    }
}

fn update_rebind_hint(
    rebinding: Res<RebindingAction>,
    mut hints: Query<&mut Text, With<RebindHint>>,
) {
    if !rebinding.is_changed() {
        return;
    }
    let Ok(mut text) = hints.single_mut() else {
        return;
    };
    text.0 = match rebinding.0 {
        Some(action) => format!("Press a key to bind \"{}\" — Esc to cancel", action.label()),
        None => String::new(),
    };
}
