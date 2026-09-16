use bevy::prelude::*;
use serde::Deserialize;

/// Where the civilization roster is loaded from, relative to the working
/// directory `cargo run`/the built exe is launched from — same convention
/// Bevy's own `AssetPlugin` uses for its `assets/` folder. Plain
/// `std::fs`+`serde_json` rather than Bevy's `AssetServer`: this only needs
/// to exist as a `Resource` before any UI reads it, and a full custom
/// `AssetLoader` (async, hot-reload-capable) is more machinery than a
/// handful of flavor definitions need right now.
const CIVILIZATIONS_JSON: &str = "assets/civilizations.json";

#[derive(Deserialize, Clone)]
pub struct CivilizationDef {
    pub name: String,
    /// `[r, g, b]`, 0-255 — used for the roster's per-slot color swatch and,
    /// eventually, team/unit coloring once `game_sim` has civs to color.
    pub color: [u8; 3],
}

/// The loaded civilization roster — see `players_menu` for where it's used
/// (the Nation stepper on each player/AI slot).
#[derive(Resource, Clone, Default)]
pub struct Civilizations(pub Vec<CivilizationDef>);

/// Used when `CIVILIZATIONS_JSON` is missing or fails to parse, so a broken
/// or absent config degrades to "a handful of generic civs" rather than an
/// empty roster or a startup panic — this is player-facing flavor data, not
/// something that should be able to brick the New Game screen.
fn fallback_civilizations() -> Vec<CivilizationDef> {
    [
        ("Ardenmoor", [90, 150, 60]),
        ("Kaelthar", [190, 90, 70]),
        ("Suncrest", [210, 170, 60]),
        ("Frosthold", [140, 180, 210]),
    ]
    .into_iter()
    .map(|(name, color)| CivilizationDef {
        name: name.to_string(),
        color,
    })
    .collect()
}

fn load_civilizations(mut commands: Commands) {
    let civs = std::fs::read_to_string(CIVILIZATIONS_JSON)
        .ok()
        .and_then(|raw| serde_json::from_str::<Vec<CivilizationDef>>(&raw).ok())
        .filter(|civs| !civs.is_empty())
        .unwrap_or_else(|| {
            warn!("couldn't load {CIVILIZATIONS_JSON}, falling back to built-in civilizations");
            fallback_civilizations()
        });
    commands.insert_resource(Civilizations(civs));
}

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(Startup, load_civilizations);
}
