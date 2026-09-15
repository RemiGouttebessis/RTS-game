use bevy::prelude::*;

mod camera;
mod map;
mod units;

pub use camera::CameraPlugin;
pub use map::MapPlugin;
pub use units::UnitsPlugin;

/// Presentation: camera, world visuals, unit meshes, VFX. Reads `game_core`
/// state to know what to draw; never mutates sim state itself.
pub struct GameRenderPlugin;

impl Plugin for GameRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((CameraPlugin, MapPlugin, UnitsPlugin));
    }
}
