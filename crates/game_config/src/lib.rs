use bevy::prelude::*;

mod camera;
mod graphics;
mod keybinds;
mod persistence;
mod settings;

pub use camera::CameraSettings;
pub use graphics::{Backend, GraphicsSettings};
pub use keybinds::KeyBindings;
pub use persistence::{load, save};
pub use settings::Settings;

/// Installs settings already loaded via [`load`] as resources, so gameplay
/// code reads `Res<CameraSettings>` / `Res<KeyBindings>` instead of hardcoded
/// constants. The graphics backend is consumed directly in `main()` before
/// the `App` exists (`RenderPlugin` needs it up front) — this plugin doesn't
/// handle that part.
pub struct GameConfigPlugin {
    pub settings: Settings,
}

impl Plugin for GameConfigPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.settings.graphics)
            .insert_resource(self.settings.camera)
            .insert_resource(self.settings.keybinds);
    }
}
