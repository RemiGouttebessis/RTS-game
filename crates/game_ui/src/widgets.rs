use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

pub(crate) const BACKGROUND: Color = Color::srgb(0.1, 0.1, 0.12);
pub(crate) const NORMAL_BUTTON: Color = Color::srgb(0.2, 0.2, 0.25);
const HOVERED_BUTTON: Color = Color::srgb(0.3, 0.3, 0.4);
const PRESSED_BUTTON: Color = Color::srgb(0.15, 0.5, 0.25);
/// Text color for a not-yet-implemented menu entry (e.g. "Multiplayer
/// (WIP)") — shown so it's visible on the roadmap, but rendered as plain
/// text with no `Button`/`Interaction`, so it's honestly non-clickable
/// rather than a button that silently does nothing.
pub(crate) const DIMMED_TEXT: Color = Color::srgb(0.5, 0.5, 0.5);

/// Warm bordered-panel palette (dark leather + gold trim) for screens going
/// for a Civ-style look — deliberately scoped to specific screens
/// (currently `new_game_menu`/`players_menu`) rather than replacing
/// `BACKGROUND` app-wide, so the rest of the still-placeholder menus aren't
/// dragged into a reskin nobody asked for yet.
pub(crate) const PANEL_BACKGROUND: Color = Color::srgb(0.16, 0.13, 0.1);
pub(crate) const PANEL_BORDER: Color = Color::srgb(0.55, 0.44, 0.24);
pub(crate) const TITLE_TEXT: Color = Color::srgb(0.92, 0.8, 0.55);

/// A bordered panel; pair with `(BackgroundColor(PANEL_BACKGROUND), BorderColor::all(PANEL_BORDER))`.
pub(crate) fn panel_node() -> Node {
    Node {
        border: UiRect::all(Val::Px(2.0)),
        padding: UiRect::all(Val::Px(12.0)),
        flex_direction: FlexDirection::Column,
        ..default()
    }
}

/// Registered once for the whole crate: hover/press color feedback for any
/// `Button` in any menu screen, and mouse-wheel scrolling for any node with
/// `ScrollPosition` (e.g. a scrollable list), so no screen reimplements
/// either.
pub(crate) fn plugin(app: &mut App) {
    app.add_systems(Update, (button_visuals, scroll_on_hover));
}

type ChangedButton = (Changed<Interaction>, With<Button>);

fn button_visuals(mut buttons: Query<(&Interaction, &mut BackgroundColor), ChangedButton>) {
    for (interaction, mut color) in &mut buttons {
        *color = BackgroundColor(match interaction {
            Interaction::Pressed => PRESSED_BUTTON,
            Interaction::Hovered => HOVERED_BUTTON,
            Interaction::None => NORMAL_BUTTON,
        });
    }
}

const SCROLL_SPEED: f32 = 28.0;

/// Any node with both `ScrollPosition` and `Interaction` (the latter just to
/// get hover tracking — it doesn't need to be a `Button`) scrolls under the
/// cursor on mouse wheel. This reads `MouseWheel` directly rather than going
/// through `game_input` — unlike a semantic, rebindable game action (camera
/// pan/zoom), list scrolling is a UI-internal behavior with no keybind to
/// speak of, the same reasoning that lets `keybinds_menu` read raw
/// `ButtonInput<KeyCode>` directly for its rebind-capture flow. Bevy's own
/// layout pass clamps the resulting `ScrollPosition` to the content's actual
/// overflow each frame, so no manual bounds-checking is needed here.
fn scroll_on_hover(
    mut wheel: MessageReader<MouseWheel>,
    mut scrollables: Query<(&Interaction, &mut ScrollPosition)>,
) {
    let delta: f32 = wheel.read().map(|event| event.y).sum();
    if delta == 0.0 {
        return;
    }

    for (interaction, mut scroll) in &mut scrollables {
        if matches!(interaction, Interaction::Hovered | Interaction::Pressed) {
            scroll.y -= delta * SCROLL_SPEED;
        }
    }
}

/// A full-screen, centered, vertically-stacked menu root.
pub(crate) fn fullscreen_menu_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        row_gap: Val::Px(16.0),
        ..default()
    }
}

/// Standard menu button size/centering; pair with `(Button, BackgroundColor(NORMAL_BUTTON))`.
pub(crate) fn button_node() -> Node {
    Node {
        width: Val::Px(220.0),
        height: Val::Px(56.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    }
}

/// A small square button for `-`/`+` steppers and similar compact controls.
pub(crate) fn stepper_button_node() -> Node {
    Node {
        width: Val::Px(32.0),
        height: Val::Px(32.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    }
}
