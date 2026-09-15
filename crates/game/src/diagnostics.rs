use bevy::prelude::*;

#[cfg(debug_assertions)]
mod overlay;

/// In-game performance overlay: FPS, frame time, sim tick rate, entity count.
/// Debug builds only — release builds pay zero cost for this.
pub struct DevDiagnosticsPlugin;

impl Plugin for DevDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(debug_assertions)]
        overlay::install(app);
    }
}
