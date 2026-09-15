//! Procedural terrain generation for the cylindrical world (see
//! `rts-game-design-doc.md` § World & Geography). No `bevy` dependency —
//! see the crate-level note in `Cargo.toml`.
//!
//! For now, output is a grid classified into Civ-style terrain/features and
//! rendered to a PNG (`image_export`) rather than turned into actual mesh
//! terrain — see the crate's module list for the generation pipeline
//! (elevation → climate → hydrology → biome).

pub mod biome;
pub mod climate;
pub mod elevation;
pub mod grid;
pub mod hydrology;
pub mod image_export;
pub mod nations;
pub mod noise;
pub mod preset;
pub mod stats;

use biome::BiomeMaps;
use climate::ClimateMaps;
use elevation::ElevationMaps;
use hydrology::HydrologyMaps;
use preset::Preset;

pub struct World {
    pub width: usize,
    pub height: usize,
    pub elevation: ElevationMaps,
    pub climate: ClimateMaps,
    pub hydrology: HydrologyMaps,
    pub biome: BiomeMaps,
}

/// Runs the full generation pipeline for a `width` (circumference, wraps) by
/// `height` (pole to pole, doesn't wrap) map.
pub fn generate(width: usize, height: usize, seed: u64, preset: &Preset) -> World {
    let elevation = elevation::generate(width, height, seed, preset);
    let mut climate = climate::generate(width, height, seed, preset, &elevation.elevation);
    let hydrology = hydrology::generate(
        width,
        height,
        &elevation.elevation,
        preset.sea_level,
        preset.river_threshold,
    );
    climate::boost_near_water(
        &mut climate.moisture,
        &hydrology.is_ocean,
        &hydrology.is_lake,
        &hydrology.is_river,
    );
    let biome = biome::generate(
        width,
        height,
        &elevation,
        &hydrology,
        &climate.temperature,
        &climate.moisture,
    );

    World {
        width,
        height,
        elevation,
        climate,
        hydrology,
        biome,
    }
}
