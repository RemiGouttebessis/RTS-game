use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use game_config::KeyBindings;

use crate::InputSet;

/// Normalized camera pan direction for this frame, in ground-plane space:
/// `x` is west(-)/east(+), `y` is north(-)/south(+). Updated every `Update`;
/// held-key state, so a resource rather than a message.
#[derive(Resource, Default, Clone, Copy, PartialEq)]
pub struct CameraPanAction(pub Vec2);

/// Camera zoom input for this frame (positive = zoom in), summed from scroll
/// events. A message: zoom is a discrete per-frame delta, not held state.
#[derive(Message, Default, Clone, Copy)]
pub struct CameraZoomAction(pub f32);

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<CameraPanAction>()
        .add_message::<CameraZoomAction>()
        .add_systems(Update, (read_pan, read_zoom).in_set(InputSet));
}

fn read_pan(
    keys: Res<ButtonInput<KeyCode>>,
    binds: Res<KeyBindings>,
    mut pan: ResMut<CameraPanAction>,
) {
    let mut direction = Vec2::ZERO;
    if keys.pressed(binds.pan_north) || keys.pressed(KeyCode::ArrowUp) {
        direction.y -= 1.0; // north
    }
    if keys.pressed(binds.pan_south) || keys.pressed(KeyCode::ArrowDown) {
        direction.y += 1.0; // south
    }
    if keys.pressed(binds.pan_west) || keys.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0; // west
    }
    if keys.pressed(binds.pan_east) || keys.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0; // east
    }
    pan.0 = direction.normalize_or_zero();
}

fn read_zoom(
    mut scroll_events: MessageReader<MouseWheel>,
    mut zoom: MessageWriter<CameraZoomAction>,
) {
    let scroll: f32 = scroll_events.read().map(|event| event.y).sum();
    if scroll != 0.0 {
        zoom.write(CameraZoomAction(scroll));
    }
}
