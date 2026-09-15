use crate::elevation::ElevationMaps;
use crate::grid::Grid;
use crate::hydrology::HydrologyMaps;

/// Base terrain, Civ-style: a small closed set of types, with ruggedness and
/// vegetation layered on top as [`ElevationBand`]/[`Feature`] rather than
/// folded into more terrain variants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Terrain {
    #[default]
    Ocean,
    Coast,
    Lake,
    Grassland,
    Plains,
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
                let near_land = hydrology
                    .is_ocean
                    .neighbors(pos.0, pos.1)
                    .any(|(nx, ny)| !*hydrology.is_ocean.get(nx, ny));
                terrain.set(
                    pos.0,
                    pos.1,
                    if near_land {
                        Terrain::Coast
                    } else {
                        Terrain::Ocean
                    },
                );
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
                Terrain::Desert
            } else if moist < 0.55 {
                Terrain::Plains
            } else {
                Terrain::Grassland
            };
            terrain.set(pos.0, pos.1, base);

            let near_water = *hydrology.is_river.get(pos.0, pos.1)
                || hydrology
                    .is_lake
                    .neighbors(pos.0, pos.1)
                    .any(|(nx, ny)| *hydrology.is_lake.get(nx, ny))
                || hydrology
                    .is_ocean
                    .neighbors(pos.0, pos.1)
                    .any(|(nx, ny)| *hydrology.is_ocean.get(nx, ny));

            let is_flat_fertile = matches!(base, Terrain::Grassland | Terrain::Plains);

            let feat = if band == ElevationBand::Flat
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
