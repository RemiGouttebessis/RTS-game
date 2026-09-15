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

/// Registered once for the whole crate: hover/press color feedback for any
/// `Button` in any menu screen, so each screen doesn't reimplement it.
pub(crate) fn plugin(app: &mut App) {
    app.add_systems(Update, button_visuals);
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
