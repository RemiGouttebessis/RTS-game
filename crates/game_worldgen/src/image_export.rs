use image::{ImageBuffer, Rgb, RgbImage};

use crate::biome::{BiomeMaps, ElevationBand, Feature, Terrain};
use crate::elevation::ElevationMaps;
use crate::hydrology::HydrologyMaps;

const RIVER_COLOR: [u8; 3] = [50, 110, 200];

/// Renders the classified world as a single color-coded PNG: Civ-style flat
/// terrain colors, darkened by elevation band for hills/mountains, ocean
/// depth-shaded by elevation, with rivers drawn on top of land. This is the
/// map to look at; see [`elevation_grayscale`] for the raw heightmap
/// underneath it.
pub fn biome_map(
    width: usize,
    height: usize,
    elevation: &ElevationMaps,
    hydrology: &HydrologyMaps,
    biome: &BiomeMaps,
) -> RgbImage {
    let mut image: RgbImage = ImageBuffer::new(width as u32, height as u32);

    for y in 0..height {
        for x in 0..width {
            let pos = (x as i64, y as i64);
            let terrain = *biome.terrain.get(pos.0, pos.1);
            let band = *biome.elevation_band.get(pos.0, pos.1);

            let mut color = terrain_color(terrain);

            match terrain {
                Terrain::Ocean => {
                    let depth = 1.0 - *elevation.elevation.get(pos.0, pos.1);
                    color = blend(color, [4, 10, 40], depth * 0.6);
                }
                Terrain::Coast | Terrain::Lake => {}
                _ => {
                    color = shade_for_band(color, band);
                    if let Some(feature) = biome.feature.get(pos.0, pos.1).0 {
                        color = blend(color, feature_color(feature), 0.55);
                    }
                    if *hydrology.is_river.get(pos.0, pos.1) {
                        color = blend(color, RIVER_COLOR, 0.85);
                    }
                }
            }

            image.put_pixel(x as u32, y as u32, Rgb(color));
        }
    }

    image
}

/// Draws a small white-disk-with-black-ring marker at each nation position
/// (see [`crate::nations::place`]), on top of an already-rendered map —
/// visible against any terrain color underneath it.
pub fn draw_nations(image: &mut RgbImage, positions: &[(usize, usize)]) {
    let (width, height) = image.dimensions();
    for &(x, y) in positions {
        for dy in -3i32..=3 {
            for dx in -3i32..=3 {
                let dist_sq = dx * dx + dy * dy;
                if dist_sq > 9 {
                    continue;
                }
                let px = (x as i32 + dx).rem_euclid(width as i32) as u32;
                let py = (y as i32 + dy).clamp(0, height as i32 - 1) as u32;
                let color = if dist_sq > 4 {
                    [10, 10, 10]
                } else {
                    [255, 255, 255]
                };
                image.put_pixel(px, py, Rgb(color));
            }
        }
    }
}

/// Name + swatch color for everything [`biome_map`] can draw, in display
/// order — a UI can render this as a color key without duplicating (and
/// risking drifting from) the actual terrain/feature colors above.
pub fn legend() -> Vec<(&'static str, [u8; 3])> {
    vec![
        ("Ocean", terrain_color(Terrain::Ocean)),
        ("Coast", terrain_color(Terrain::Coast)),
        ("Lake", terrain_color(Terrain::Lake)),
        ("Grassland", terrain_color(Terrain::Grassland)),
        ("Plains", terrain_color(Terrain::Plains)),
        ("Desert", terrain_color(Terrain::Desert)),
        ("Tundra", terrain_color(Terrain::Tundra)),
        ("Snow", terrain_color(Terrain::Snow)),
        ("Forest", feature_color(Feature::Forest)),
        ("Jungle", feature_color(Feature::Jungle)),
        ("Marsh", feature_color(Feature::Marsh)),
        ("River", RIVER_COLOR),
        ("Nation", [255, 255, 255]),
    ]
}

/// The raw heightmap as grayscale — useful for sanity-checking generation
/// parameters (continent shapes, mountain belts) independent of biome
/// classification.
pub fn elevation_grayscale(width: usize, height: usize, elevation: &ElevationMaps) -> RgbImage {
    let mut image: RgbImage = ImageBuffer::new(width as u32, height as u32);
    for y in 0..height {
        for x in 0..width {
            let h = *elevation.elevation.get(x as i64, y as i64);
            let v = (h.clamp(0.0, 1.0) * 255.0) as u8;
            image.put_pixel(x as u32, y as u32, Rgb([v, v, v]));
        }
    }
    image
}

fn terrain_color(terrain: Terrain) -> [u8; 3] {
    match terrain {
        Terrain::Ocean => [20, 60, 130],
        Terrain::Coast => [70, 130, 190],
        Terrain::Lake => [60, 120, 190],
        Terrain::Grassland => [90, 150, 60],
        Terrain::Plains => [190, 175, 90],
        Terrain::Desert => [225, 200, 130],
        Terrain::Tundra => [150, 160, 140],
        Terrain::Snow => [240, 240, 245],
    }
}

fn feature_color(feature: Feature) -> [u8; 3] {
    match feature {
        Feature::Forest => [30, 90, 35],
        Feature::Jungle => [15, 100, 40],
        Feature::Marsh => [70, 100, 60],
    }
}

fn shade_for_band(color: [u8; 3], band: ElevationBand) -> [u8; 3] {
    match band {
        ElevationBand::Flat => color,
        ElevationBand::Hills => blend(color, [70, 60, 40], 0.25),
        ElevationBand::Mountains => blend(color, [90, 85, 80], 0.6),
    }
}

fn blend(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        (a[0] as f32 * (1.0 - t) + b[0] as f32 * t) as u8,
        (a[1] as f32 * (1.0 - t) + b[1] as f32 * t) as u8,
        (a[2] as f32 * (1.0 - t) + b[2] as f32 * t) as u8,
    ]
}
