use bevy::prelude::*;

mod camera;
mod debug;

pub use camera::{CameraPanAction, CameraZoomAction};
pub use debug::ToggleDebugOverlay;

/// Ordering label for input-reading systems. Consumers add their systems
/// `.after(InputSet)` so they see this frame's input rather than last frame's.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputSet;

/// Translates raw device input (keyboard, mouse) into semantic game actions.
/// Key/mouse bindings live only here — consumers (camera, UI, …) react to
/// actions and never read `ButtonInput`/`MouseWheel` directly, so rebinding a
/// key touches one place.
pub struct GameInputPlugin;

impl Plugin for GameInputPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(Update, InputSet)
            .add_plugins((camera::plugin, debug::plugin));
    }
}
