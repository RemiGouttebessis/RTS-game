use crate::elevation::ElevationMaps;
use crate::grid::Grid;
use crate::hydrology::HydrologyMaps;
use crate::noise::splitmix64;

/// Base terrain, Civ-style: a small closed set of types, with ruggedness and
/// vegetation layered on top as [`ElevationBand`]/[`Feature`] rather than
/// folded into more terrain variants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Terrain {
    #[default]
    Ocean,
    Coast,
    /// Frozen ocean/coast near the poles — same underlying water, just cold
    /// enough to render (and eventually play) differently.
    Ice,
    Lake,
    Grassland,
    Plains,
    /// Hot, moderately dry grassland — fills the gap between Desert (too
    /// dry) and Plains/Grassland (temperate) for warm climates.
    Savanna,
    /// Cold, dry grassland — the cold-climate counterpart to Desert, the way
    /// Savanna is the warm-climate counterpart to Plains. Without this, both
    /// a Saharan-style hot desert and a Central-Asian-style cold dry plain
    /// classified as plain "Desert" regardless of temperature.
    Steppe,
    Desert,
    Tundra,
    Snow,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ElevationBand {
    #[default]
    Flat,
    Hills,
    Mountains,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    Forest,
    Jungle,
    Marsh,
    /// Rare, on Mountains only.
    Volcano,
    /// Rare, on Desert only.
    Oasis,
    /// A fertile strip directly along a river running through otherwise-arid
    /// land (Desert/Steppe) — distinct from Marsh, which is a wetland from
    /// generally high ambient moisture rather than one specific river.
    Floodplains,
    /// Warm shallow water next to Coast. A feature on `Terrain::Coast`, not
    /// its own terrain — same underlying water, just visually/mechanically
    /// distinct, the way Volcano is a feature on Mountains rather than its
    /// own elevation band.
    Reef,
}

#[derive(Clone, Copy, Default)]
pub struct FeatureCell(pub Option<Feature>);

pub struct BiomeMaps {
    pub terrain: Grid<Terrain>,
    pub elevation_band: Grid<ElevationBand>,
    pub feature: Grid<FeatureCell>,
}

pub fn generate(
    width: usize,
    height: usize,
    seed: u64,
    elevation: &ElevationMaps,
    hydrology: &HydrologyMaps,
    temperature: &Grid<f32>,
    moisture: &Grid<f32>,
) -> BiomeMaps {
    let mut terrain = Grid::<Terrain>::new(width, height);
    let mut elevation_band = Grid::<ElevationBand>::new(width, height);
    let mut feature = Grid::<FeatureCell>::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let pos = (x as i64, y as i64);

            if *hydrology.is_ocean.get(pos.0, pos.1) {
                let temp = *temperature.get(pos.0, pos.1);
                if temp < 0.12 {
                    terrain.set(pos.0, pos.1, Terrain::Ice);
                    continue;
                }

                let near_land = hydrology
                    .is_ocean
                    .neighbors(pos.0, pos.1)
                    .any(|(nx, ny)| !*hydrology.is_ocean.get(nx, ny));
                if !near_land {
                    terrain.set(pos.0, pos.1, Terrain::Ocean);
                    continue;
                }

                terrain.set(pos.0, pos.1, Terrain::Coast);
                if temp > 0.6 && cell_random(x, y, seed ^ 0x2EEF) < 0.22 {
                    feature.set(pos.0, pos.1, FeatureCell(Some(Feature::Reef)));
                }
                continue;
            }

            if *hydrology.is_lake.get(pos.0, pos.1) {
                terrain.set(pos.0, pos.1, Terrain::Lake);
                continue;
            }

            let mountain = *elevation.mountain_mask.get(pos.0, pos.1);
            let band = if mountain > 0.5 {
                ElevationBand::Mountains
            } else if mountain > 0.15 {
                ElevationBand::Hills
            } else {
                ElevationBand::Flat
            };
            elevation_band.set(pos.0, pos.1, band);

            let temp = *temperature.get(pos.0, pos.1);
            let moist = *moisture.get(pos.0, pos.1);

            let base = if temp < 0.2 {
                Terrain::Snow
            } else if temp < 0.4 {
                Terrain::Tundra
            } else if moist < 0.3 {
                if temp > 0.55 {
                    Terrain::Desert
                } else {
                    Terrain::Steppe
                }
            } else if moist < 0.45 {
                if temp > 0.55 {
                    Terrain::Savanna
                } else {
                    Terrain::Plains
                }
            } else {
                Terrain::Grassland
            };
            terrain.set(pos.0, pos.1, base);

            let on_river = *hydrology.is_river.get(pos.0, pos.1)
                || hydrology
                    .is_river
                    .neighbors(pos.0, pos.1)
                    .any(|(nx, ny)| *hydrology.is_river.get(nx, ny));
            let near_water = on_river
                || hydrology
                    .is_lake
                    .neighbors(pos.0, pos.1)
                    .any(|(nx, ny)| *hydrology.is_lake.get(nx, ny))
                || hydrology
                    .is_ocean
                    .neighbors(pos.0, pos.1)
                    .any(|(nx, ny)| *hydrology.is_ocean.get(nx, ny));

            let is_flat_fertile = matches!(base, Terrain::Grassland | Terrain::Plains);
            let is_arid = matches!(base, Terrain::Desert | Terrain::Steppe);

            let feat =
                if band == ElevationBand::Mountains && cell_random(x, y, seed ^ 0xC0FFEE) < 0.05 {
                    Some(Feature::Volcano)
                } else if base == Terrain::Desert && cell_random(x, y, seed ^ 0xFACADE) < 0.09 {
                    Some(Feature::Oasis)
                } else if band == ElevationBand::Flat && on_river && is_arid {
                    Some(Feature::Floodplains)
                } else if band == ElevationBand::Flat
                    && near_water
                    && moist > 0.6
                    && temp > 0.3
                    && temp < 0.85
                {
                    Some(Feature::Marsh)
                } else if temp > 0.75 && moist > 0.6 && is_flat_fertile {
                    Some(Feature::Jungle)
                } else if band != ElevationBand::Mountains
                    && moist > 0.45
                    && temp > 0.25
                    && matches!(base, Terrain::Grassland | Terrain::Plains | Terrain::Tundra)
                {
                    Some(Feature::Forest)
                } else {
                    None
                };
            feature.set(pos.0, pos.1, FeatureCell(feat));
        }
    }

    BiomeMaps {
        terrain,
        elevation_band,
        feature,
    }
}

/// Deterministic pseudo-random `0..1` for a single cell — used for rare,
/// scattered features (volcanoes, oases, reefs) that just need "an
/// unpredictable but reproducible yes/no here," not a continuous noise
/// field.
fn cell_random(x: usize, y: usize, seed: u64) -> f64 {
    let mixed = splitmix64(
        seed ^ (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F),
    );
    (mixed >> 11) as f64 / (1u64 << 53) as f64
}
