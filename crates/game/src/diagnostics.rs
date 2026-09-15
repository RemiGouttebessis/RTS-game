use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;

/// Frame-time/FPS logging. Not part of `DefaultPlugins` — opt in explicitly.
/// Debug-only: an RTS with hundreds of units is exactly where perf regressions
/// creep in, so it's worth having this wired up from the start.
pub struct DevDiagnosticsPlugin;

impl Plugin for DevDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(debug_assertions)]
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ));
    }
}
