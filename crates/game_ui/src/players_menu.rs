use bevy::prelude::*;
use game_assets::Civilizations;

use crate::new_game_menu::{NewGameContentArea, NewGameTab};
use crate::widgets::{
    BACKGROUND, NORMAL_BUTTON, PANEL_BACKGROUND, PANEL_BORDER, TITLE_TEXT, button_node, panel_node,
    stepper_button_node,
};

/// Slot 0 is always the local human player, so this is the AI-slot cap —
/// total roster size tops out at `MAX_AI_SLOTS + 1`. Matches the
/// `worldgen_menu` Nations stepper's rough scale, since nation starting
/// positions are what these slots will eventually claim. Comfortably more
/// than fits the list's fixed-height viewport at once, so the roster is
/// always a good test of `LIST_HEIGHT`'s scrolling.
const MAX_AI_SLOTS: usize = 7;

/// Fixed viewport height for the roster — without a cap here nothing ever
/// overflows, so `Overflow::scroll_y()` never has anything to actually
/// scroll. `widgets::scroll_on_hover` (mouse wheel) drives it.
const LIST_HEIGHT: f32 = 260.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Nation {
    Random,
    /// Index into the loaded `Civilizations` roster.
    Fixed(usize),
}

impl Nation {
    fn label(self, civs: &Civilizations) -> String {
        match self {
            Nation::Random => "Random".to_string(),
            Nation::Fixed(i) => civs
                .0
                .get(i)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| "Random".to_string()),
        }
    }

    /// The swatch color for this pick, or `None` for Random (no fixed color
    /// to show yet).
    fn color(self, civs: &Civilizations) -> Option<[u8; 3]> {
        match self {
            Nation::Random => None,
            Nation::Fixed(i) => civs.0.get(i).map(|c| c.color),
        }
    }

    /// Cycles Random -> first civ -> ... -> last civ -> Random.
    fn next(self, civs: &Civilizations) -> Self {
        match self {
            Nation::Random if !civs.0.is_empty() => Nation::Fixed(0),
            Nation::Random => Nation::Random,
            Nation::Fixed(i) if i + 1 < civs.0.len() => Nation::Fixed(i + 1),
            Nation::Fixed(_) => Nation::Random,
        }
    }

    fn prev(self, civs: &Civilizations) -> Self {
        match self {
            Nation::Random if !civs.0.is_empty() => Nation::Fixed(civs.0.len() - 1),
            Nation::Random => Nation::Random,
            Nation::Fixed(0) => Nation::Random,
            Nation::Fixed(i) => Nation::Fixed(i - 1),
        }
    }
}

#[derive(Clone, Copy)]
struct PlayerSlot {
    nation: Nation,
}

/// The player/AI roster for the next game. Slot 0 is always the local human
/// player — fixed, never removable, never re-typed as AI, since this is the
/// Solo flow and there's always exactly one seat at the keyboard. Every
/// slot after it is an AI opponent added via "+ Add AI". Survives leaving/
/// re-entering the screen (a normal resource, not reset by `OnEnter`), same
/// as `worldgen_menu::WorldGenSettings`.
#[derive(Resource)]
pub(crate) struct PlayersSettings {
    slots: Vec<PlayerSlot>,
}

impl Default for PlayersSettings {
    fn default() -> Self {
        Self {
            slots: vec![PlayerSlot {
                nation: Nation::Random,
            }],
        }
    }
}

#[derive(Component)]
struct PlayersRoot;

/// The list viewport — cleared and rebuilt from scratch on every add/
/// remove/nation change, rather than diffed. The roster is capped at 8 rows,
/// so a full rebuild is cheap and avoids tracking per-row entities through
/// index shifts when a slot in the middle gets removed.
#[derive(Component)]
struct PlayerListArea;

#[derive(Component, Clone, Copy)]
enum PlayersButton {
    AddAi,
    RemoveSlot(usize),
    NationPrev(usize),
    NationNext(usize),
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<PlayersSettings>()
        .add_systems(OnEnter(NewGameTab::Players), spawn_screen)
        .add_systems(OnExit(NewGameTab::Players), despawn_screen)
        .add_systems(Update, button_actions.run_if(in_state(NewGameTab::Players)));
}

fn spawn_screen(
    mut commands: Commands,
    settings: Res<PlayersSettings>,
    civs: Res<Civilizations>,
    content_area: Query<Entity, With<NewGameContentArea>>,
) {
    let Ok(area) = content_area.single() else {
        return;
    };

    commands.entity(area).with_children(|parent| {
        parent
            .spawn((
                PlayersRoot,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Start,
                    justify_content: JustifyContent::Center,
                    row_gap: Val::Px(12.0),
                    padding: UiRect::all(Val::Px(24.0)),
                    ..default()
                },
                BackgroundColor(BACKGROUND),
            ))
            .with_children(|column| {
                column
                    .spawn((
                        BackgroundColor(PANEL_BACKGROUND),
                        BorderColor::all(PANEL_BORDER),
                        Node {
                            row_gap: Val::Px(10.0),
                            ..panel_node()
                        },
                    ))
                    .with_children(|panel| {
                        panel.spawn((
                            Text::new("Players"),
                            TextFont::from_font_size(26.0),
                            TextColor(TITLE_TEXT),
                        ));

                        panel
                            .spawn((
                                PlayerListArea,
                                Interaction::default(),
                                ScrollPosition::default(),
                                Node {
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(8.0),
                                    height: Val::Px(LIST_HEIGHT),
                                    overflow: Overflow::scroll_y(),
                                    ..default()
                                },
                            ))
                            .with_children(|list| spawn_rows(list, &settings, &civs));

                        panel
                            .spawn((
                                Button,
                                PlayersButton::AddAi,
                                button_node(),
                                BackgroundColor(NORMAL_BUTTON),
                            ))
                            .with_child((Text::new("+ Add AI"), TextFont::from_font_size(18.0)));
                    });
            });
    });
}

fn spawn_rows(list: &mut ChildSpawnerCommands, settings: &PlayersSettings, civs: &Civilizations) {
    for (i, slot) in settings.slots.iter().enumerate() {
        list.spawn((
            BackgroundColor(BACKGROUND),
            BorderColor::all(PANEL_BORDER.with_alpha(0.4)),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                ..panel_node()
            },
        ))
        .with_children(|row| {
            let swatch = slot
                .nation
                .color(civs)
                .map(|c| Color::srgb_u8(c[0], c[1], c[2]))
                .unwrap_or(BACKGROUND);
            row.spawn((
                Node {
                    width: Val::Px(18.0),
                    height: Val::Px(18.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(swatch),
                BorderColor::all(PANEL_BORDER),
            ));

            let kind_label = if i == 0 { "You" } else { "AI" };
            row.spawn((
                Text::new(format!("Player {}  ({kind_label})", i + 1)),
                TextFont::from_font_size(16.0),
                Node {
                    width: Val::Px(130.0),
                    ..default()
                },
            ));

            row.spawn((
                Button,
                PlayersButton::NationPrev(i),
                stepper_button_node(),
                BackgroundColor(NORMAL_BUTTON),
            ))
            .with_child((Text::new("-"), TextFont::from_font_size(18.0)));
            row.spawn((
                Text::new(slot.nation.label(civs)),
                TextFont::from_font_size(16.0),
                Node {
                    width: Val::Px(150.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
            ));
            row.spawn((
                Button,
                PlayersButton::NationNext(i),
                stepper_button_node(),
                BackgroundColor(NORMAL_BUTTON),
            ))
            .with_child((Text::new("+"), TextFont::from_font_size(18.0)));

            if i != 0 {
                row.spawn((
                    Button,
                    PlayersButton::RemoveSlot(i),
                    Node {
                        width: Val::Px(84.0),
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_child((Text::new("Remove"), TextFont::from_font_size(13.0)));
            }
        });
    }
}

fn button_actions(
    buttons: Query<(&Interaction, &PlayersButton), Changed<Interaction>>,
    mut settings: ResMut<PlayersSettings>,
    civs: Res<Civilizations>,
    mut commands: Commands,
    list_area: Query<(Entity, Option<&Children>), With<PlayerListArea>>,
) {
    let mut changed = false;

    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *action {
            PlayersButton::AddAi => {
                if settings.slots.len() < MAX_AI_SLOTS + 1 {
                    settings.slots.push(PlayerSlot {
                        nation: Nation::Random,
                    });
                    changed = true;
                }
            }
            PlayersButton::RemoveSlot(i) => {
                if i != 0 && i < settings.slots.len() {
                    settings.slots.remove(i);
                    changed = true;
                }
            }
            PlayersButton::NationPrev(i) => {
                if let Some(slot) = settings.slots.get_mut(i) {
                    slot.nation = slot.nation.prev(&civs);
                    changed = true;
                }
            }
            PlayersButton::NationNext(i) => {
                if let Some(slot) = settings.slots.get_mut(i) {
                    slot.nation = slot.nation.next(&civs);
                    changed = true;
                }
            }
        }
    }

    if !changed {
        return;
    }

    let Ok((area, children)) = list_area.single() else {
        return;
    };
    if let Some(children) = children {
        for &child in children {
            commands.entity(child).despawn();
        }
    }
    commands
        .entity(area)
        .with_children(|list| spawn_rows(list, &settings, &civs));
}

fn despawn_screen(mut commands: Commands, roots: Query<Entity, With<PlayersRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}
