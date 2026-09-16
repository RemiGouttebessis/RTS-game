use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use game_core::{GameState, GeneratedWorldData};

use crate::main_menu::MainMenuScreen;
use crate::widgets::{BACKGROUND, NORMAL_BUTTON, PANEL_BACKGROUND, PANEL_BORDER, button_node};
use crate::worldgen_menu::WorldGenSettings;

const ACTIVE_TAB_TEXT: Color = Color::srgb(1.0, 0.82, 0.3);
const INACTIVE_TAB_TEXT: Color = Color::srgb(0.85, 0.85, 0.85);

/// The world-generation task Start kicks off — `game_worldgen::generate` at
/// a real (possibly large) size can take a while, so it runs on
/// `AsyncComputeTaskPool` rather than blocking the whole app the instant
/// Start is pressed; `loading_menu::poll_world_generation` picks up the
/// result once it's ready and populates `GeneratedWorld`.
#[derive(Resource, Default)]
pub(crate) struct PendingStart(pub(crate) Option<Task<GeneratedWorldData>>);

/// Which tab of the New Game screen is showing. Unlike `PauseMenuScreen`'s
/// Root/Keybinds (navigating away to a sub-screen and back), these are
/// meant to feel like flipping between panels of one dialog: the tab bar
/// and Back button spawned here stay on screen the whole time, and only the
/// content in `NewGameContentArea` is swapped. Resets to `World` every time
/// `MainMenuScreen::NewGame` is (re)entered.
#[derive(SubStates, Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
#[source(MainMenuScreen = MainMenuScreen::NewGame)]
pub(crate) enum NewGameTab {
    #[default]
    World,
    Players,
}

/// Where each tab's content mounts itself — `worldgen_menu`/`players_menu`
/// spawn as children of this entity (via `OnEnter(NewGameTab::_)`) instead
/// of spawning their own top-level root, so the tab bar/Back button above
/// them never has to be re-spawned when the tab switches.
#[derive(Component)]
pub(crate) struct NewGameContentArea;

#[derive(Component)]
struct NewGameShellRoot;

/// Marks a tab button's label `Text` with which tab it selects, so
/// `style_tabs` can highlight whichever one is currently active — done via
/// `TextColor` rather than `BackgroundColor` because `widgets::button_visuals`
/// already drives every button's background from hover/press state and would
/// stomp on an "active" tint the instant the mouse moved off the button.
#[derive(Component)]
struct TabLabel(NewGameTab);

#[derive(Component, Clone, Copy)]
enum ShellButton {
    Tab(NewGameTab),
    Back,
    Start,
}

pub(crate) fn plugin(app: &mut App) {
    app.add_sub_state::<NewGameTab>()
        .init_resource::<PendingStart>()
        .add_systems(OnEnter(MainMenuScreen::NewGame), spawn_shell)
        .add_systems(OnExit(MainMenuScreen::NewGame), despawn_shell)
        .add_systems(
            Update,
            (button_actions, style_tabs).run_if(in_state(MainMenuScreen::NewGame)),
        );
}

fn spawn_shell(mut commands: Commands) {
    commands
        .spawn((
            NewGameShellRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(BACKGROUND),
        ))
        .with_children(|root| {
            root.spawn((
                BackgroundColor(PANEL_BACKGROUND),
                BorderColor::all(PANEL_BORDER),
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::all(Val::Px(12.0)),
                    column_gap: Val::Px(12.0),
                    border: UiRect::bottom(Val::Px(2.0)),
                    ..default()
                },
            ))
            .with_children(|header| {
                header
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|tabs| {
                        for (tab, label) in [
                            (NewGameTab::World, "World"),
                            (NewGameTab::Players, "Players"),
                        ] {
                            tabs.spawn((
                                Button,
                                ShellButton::Tab(tab),
                                button_node(),
                                BackgroundColor(NORMAL_BUTTON),
                            ))
                            .with_child((
                                Text::new(label),
                                TabLabel(tab),
                                TextFont::from_font_size(18.0),
                                TextColor(INACTIVE_TAB_TEXT),
                            ));
                        }
                    });

                header
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|buttons| {
                        buttons
                            .spawn((
                                Button,
                                ShellButton::Back,
                                button_node(),
                                BackgroundColor(NORMAL_BUTTON),
                            ))
                            .with_child((Text::new("Back"), TextFont::from_font_size(18.0)));

                        buttons
                            .spawn((
                                Button,
                                ShellButton::Start,
                                button_node(),
                                BackgroundColor(NORMAL_BUTTON),
                            ))
                            .with_child((Text::new("Start"), TextFont::from_font_size(18.0)));
                    });
            });

            root.spawn((
                NewGameContentArea,
                Node {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    ..default()
                },
            ));
        });
}

#[allow(clippy::too_many_arguments)]
fn button_actions(
    buttons: Query<(&Interaction, &ShellButton), Changed<Interaction>>,
    mut next_tab: ResMut<NextState<NewGameTab>>,
    mut next_screen: ResMut<NextState<MainMenuScreen>>,
    mut next_game_state: ResMut<NextState<GameState>>,
    world_settings: Res<WorldGenSettings>,
    mut pending_start: ResMut<PendingStart>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            ShellButton::Tab(tab) => next_tab.set(*tab),
            ShellButton::Back => next_screen.set(MainMenuScreen::SoloMode),
            ShellButton::Start => {
                // Same grid size the World tab's preview last showed — Size
                // isn't decoupled from the real terrain anymore, so
                // regenerating at `dimensions()` (rather than some smaller,
                // separately-tuned constant) is what makes "what you
                // preview is what you get" actually true. Runs in the
                // background (see `PendingStart`) and only *starts* the
                // transition to `GameState::Loading` — `game_render::map`
                // is what actually flips to `InGame` once terrain chunks
                // are built from the result.
                let preset = world_settings.effective_preset();
                let (width, height) = world_settings.dimensions();
                let seed = world_settings.seed();
                let pool = AsyncComputeTaskPool::get();
                pending_start.0 = Some(pool.spawn(async move {
                    let world = game_worldgen::generate(width, height, seed, &preset);
                    GeneratedWorldData {
                        world,
                        sea_level: preset.sea_level,
                    }
                }));
                next_game_state.set(GameState::Loading);
            }
        }
    }
}

/// Runs every frame the New Game screen is open (cheap — two labels) rather
/// than only on tab-change, so the initial tab is styled correctly on spawn
/// without needing a separate first-frame case.
fn style_tabs(current: Res<State<NewGameTab>>, mut labels: Query<(&TabLabel, &mut TextColor)>) {
    for (tab, mut color) in &mut labels {
        color.0 = if tab.0 == *current.get() {
            ACTIVE_TAB_TEXT
        } else {
            INACTIVE_TAB_TEXT
        };
    }
}

fn despawn_shell(mut commands: Commands, roots: Query<Entity, With<NewGameShellRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}
