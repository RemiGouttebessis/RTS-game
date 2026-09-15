use serde::{Deserialize, Serialize};

use crate::{CameraSettings, GraphicsSettings, KeyBindings};

/// Every user-configurable, cross-run setting, in one file on disk. Gameplay
/// constants that aren't user-facing (unit spawn counts, ground size, ...)
/// don't belong here — this is what `main` needs before the `App` exists
/// (graphics backend) and what a future settings/keybind UI would edit.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Settings {
    pub graphics: GraphicsSettings,
    pub camera: CameraSettings,
    pub keybinds: KeyBindings,
}
