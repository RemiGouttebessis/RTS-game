use bevy::prelude::*;

mod civilizations;

pub use civilizations::{CivilizationDef, Civilizations};

/// Asset loading and data-driven definitions. Civilizations are JSON
/// (`assets/civilizations.json`, see `civilizations.rs`); unit/building
/// definitions will likely be RON under `data/` once they exist — no
/// particular reason both need the same format, JSON was what was asked for
/// here.
pub struct GameAssetsPlugin;

impl Plugin for GameAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(civilizations::plugin);
    }
}
