use image::{ImageBuffer, Rgb, RgbImage};

use crate::World;
use crate::biome::{BiomeMaps, ElevationBand, Feature, Terrain};
use crate::elevation::ElevationMaps;
use crate::grid::Grid;
use crate::hydrology::HydrologyMaps;
use crate::preset;

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
                Terrain::Coast => {
                    if let Some(feature) = biome.feature.get(pos.0, pos.1).0 {
                        color = blend(color, feature_color(feature), 0.6);
                    }
                }
                Terrain::Lake | Terrain::Ice => {}
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

/// Maps a world-space `(x, z)` position (meters, same convention
/// `game_render::map::build_chunk_mesh` uses to place vertices: `x = (col -
/// width/2) * METERS_PER_QUAD`, `z` likewise from `row`) back to the nearest
/// grid cell — wrapping `col` (the cylinder wraps east-west) and clamping
/// `row` (it doesn't wrap north-south), matching `Grid::index`'s own
/// convention exactly so this always lands on a cell `Grid::get` would too.
/// Used wherever something needs "what terrain is under this world
/// position" for a single nearest cell (e.g. `game_render::camera`'s
/// ground-following); doesn't fit `game_render::map`'s teraform brush, which
/// needs continuous (non-rounded) col/row for its falloff math instead.
pub fn world_to_grid(
    world_width: usize,
    world_height: usize,
    world_x: f32,
    world_z: f32,
) -> (usize, usize) {
    let col = (world_x / preset::METERS_PER_QUAD + world_width as f32 / 2.0).round() as i64;
    let row = (world_z / preset::METERS_PER_QUAD + world_height as f32 / 2.0).round() as i64;
    let col = col.rem_euclid(world_width as i64) as usize;
    let row = row.clamp(0, world_height as i64 - 1) as usize;
    (col, row)
}

/// Visual (mesh) height for one world cell: a flat-clipped depth for ocean
/// (see `preset::COAST_FLOOR_DEPTH`/`OCEAN_FLOOR_DEPTH`), the basin's real
/// pour-point elevation for a lake (`HydrologyMaps::filled`, not a fixed
/// clip — see that field's doc comment), or scaled-and-mountain-boosted raw
/// elevation for land (`preset::HEIGHT_SCALE`/`MOUNTAIN_HEIGHT_BOOST`).
/// Shared by `game_render::map`'s actual mesh and this module's own slope
/// math (`slope_at`) so the two can never disagree about how tall a cell
/// reads.
pub fn visual_height(world: &World, sea_level: f32, col: usize, row: usize) -> f32 {
    let pos = (col as i64, row as i64);

    if *world.hydrology.is_lake.get(pos.0, pos.1) {
        let filled = *world.hydrology.filled.get(pos.0, pos.1);
        return (filled - sea_level) * preset::HEIGHT_SCALE - preset::LAKE_SURFACE_OFFSET;
    }
    if *world.hydrology.is_ocean.get(pos.0, pos.1) {
        let terrain = *world.biome.terrain.get(pos.0, pos.1);
        return if terrain == Terrain::Coast {
            -preset::COAST_FLOOR_DEPTH
        } else {
            -preset::OCEAN_FLOOR_DEPTH
        };
    }

    let elevation = *world.elevation.elevation.get(pos.0, pos.1);
    let mountain = *world.elevation.mountain_mask.get(pos.0, pos.1);
    (elevation - sea_level)
        * preset::HEIGHT_SCALE
        * (1.0 + mountain * preset::MOUNTAIN_HEIGHT_BOOST)
}

/// Steepness at a cell, as an angle in radians from horizontal — the
/// gradient of `visual_height` over its 4 immediate neighbors (wrap-aware in
/// x, clamped at the poles in y, matching `Grid`'s own convention).
fn slope_at(world: &World, sea_level: f32, col: usize, row: usize) -> f32 {
    let width = world.width as i64;
    let height = world.height as i64;
    let c = col as i64;
    let r = row as i64;

    let west = (c - 1).rem_euclid(width) as usize;
    let east = (c + 1).rem_euclid(width) as usize;
    let north = (r - 1).clamp(0, height - 1) as usize;
    let south = (r + 1).clamp(0, height - 1) as usize;

    let h_west = visual_height(world, sea_level, west, row);
    let h_east = visual_height(world, sea_level, east, row);
    let h_north = visual_height(world, sea_level, col, north);
    let h_south = visual_height(world, sea_level, col, south);

    let dx = (h_east - h_west) / (2.0 * preset::METERS_PER_QUAD);
    let dz = (h_south - h_north) / (2.0 * preset::METERS_PER_QUAD);
    (dx * dx + dz * dz).sqrt().atan()
}

/// A land cell's color before slope/beach/neighbor blending — terrain +
/// elevation-band darkening + feature blend, the same recipe `biome_map`
/// uses for land, minus river blending (rivers are still a 2D-preview-only
/// concept for now, not drawn into the 3D mesh).
fn land_base_color(world: &World, col: usize, row: usize) -> [u8; 3] {
    let pos = (col as i64, row as i64);
    let terrain = *world.biome.terrain.get(pos.0, pos.1);
    let band = *world.biome.elevation_band.get(pos.0, pos.1);
    let mut color = shade_for_band(terrain_color(terrain), band);
    if let Some(feature) = world.biome.feature.get(pos.0, pos.1).0 {
        color = blend(color, feature_color(feature), 0.55);
    }
    color
}

const ROCK_COLOR: [u8; 3] = [112, 106, 100];
/// Slope angle (radians) where rock starts blending in / is fully rock,
/// ~30°/~55° — steep enough that "still Plains" stops looking plausible.
const ROCK_SLOPE_START: f32 = 0.524;
const ROCK_SLOPE_FULL: f32 = 0.960;

const SAND_COLOR: [u8; 3] = [214, 199, 152];
/// How close to `sea_level` (in the same normalized `0..1.2` elevation
/// units) land still counts as "beach" — a cliff dropping straight into the
/// ocean is well above this and correctly gets no beach. Wide enough to
/// read as an actual beach band rather than a 1-pixel fringe at typical map
/// resolutions.
const BEACH_ELEVATION_BAND: f32 = 0.09;

/// Which broad surface a cell reads as, for texturing purposes
/// (`game_render::map`'s atlas UV selection) — a coarser, discrete cousin of
/// `terrain_paint_color`'s continuous blend factors. Uses the exact same
/// beach/rock threshold checks as that function (see `terrain_paint_color`'s
/// body), just picking a hard winner instead of a blend weight, since a mesh
/// vertex can only sample one atlas quadrant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceClass {
    Water,
    Sand,
    Rock,
    Land,
}

/// A 3×3-neighbor-blended, slope/beach-aware land color — what
/// `game_render::map` actually paints onto the terrain mesh, unlike
/// `biome_map`'s flat per-cell rendering (still used for the 2D preview and
/// `WorldMapPlane`, which really is just a preview and doesn't need any of
/// this). Water cells (ocean/coast/lake/ice) skip all three effects and
/// just return their flat `terrain_color` — slope/beach/blend are land
/// concepts, and blending a lake edge toward its neighbors would just tint
/// the water, not soften a biome boundary. Also returns the `SurfaceClass`
/// that produced this color, for `game_render::map`'s atlas texture — the
/// same beach/rock checks decide both, so the two can't disagree about which
/// cells count as beach/cliff/plain land.
pub fn terrain_paint_color(
    world: &World,
    sea_level: f32,
    col: usize,
    row: usize,
) -> (SurfaceClass, [u8; 3]) {
    let pos = (col as i64, row as i64);
    if *world.hydrology.is_lake.get(pos.0, pos.1) || *world.hydrology.is_ocean.get(pos.0, pos.1) {
        return (
            SurfaceClass::Water,
            terrain_color(*world.biome.terrain.get(pos.0, pos.1)),
        );
    }

    let base = blended_land_color(world, col, row);

    let elevation = *world.elevation.elevation.get(pos.0, pos.1);
    let above_sea = elevation - sea_level;
    if above_sea < BEACH_ELEVATION_BAND && near_water(world, col, row) {
        let t = (1.0 - above_sea / BEACH_ELEVATION_BAND).clamp(0.0, 1.0);
        return (SurfaceClass::Sand, blend(base, SAND_COLOR, t * 0.95));
    }

    let slope = slope_at(world, sea_level, col, row);
    if slope > ROCK_SLOPE_START {
        let t = ((slope - ROCK_SLOPE_START) / (ROCK_SLOPE_FULL - ROCK_SLOPE_START)).clamp(0.0, 1.0);
        return (SurfaceClass::Rock, blend(base, ROCK_COLOR, t));
    }

    (SurfaceClass::Land, base)
}

/// Averages `land_base_color` over a cell's 3×3 neighborhood (falling back
/// to the center cell's own color for any water neighbor, so a coastline
/// doesn't get tinted by the ocean) — softens the hard edges between
/// adjacent biomes into a gradient instead of a flat color boundary.
fn blended_land_color(world: &World, col: usize, row: usize) -> [u8; 3] {
    let width = world.width as i64;
    let height = world.height as i64;
    let c = col as i64;
    let r = row as i64;
    let center = land_base_color(world, col, row);

    let mut sum = [0u32; 3];
    let mut count = 0u32;
    for dy in -1..=1 {
        for dx in -1..=1 {
            let nx = (c + dx).rem_euclid(width) as usize;
            let ny = (r + dy).clamp(0, height - 1) as usize;
            let npos = (nx as i64, ny as i64);
            let is_water = *world.hydrology.is_lake.get(npos.0, npos.1)
                || *world.hydrology.is_ocean.get(npos.0, npos.1);
            let color = if is_water {
                center
            } else {
                land_base_color(world, nx, ny)
            };
            sum[0] += color[0] as u32;
            sum[1] += color[1] as u32;
            sum[2] += color[2] as u32;
            count += 1;
        }
    }

    [
        (sum[0] / count) as u8,
        (sum[1] / count) as u8,
        (sum[2] / count) as u8,
    ]
}

/// Whether any of a land cell's 8 neighbors is ocean or lake — used only for
/// the beach blend, which needs actual adjacency, not just "close to sea
/// level" (an inland low point near `sea_level` isn't a beach).
fn near_water(world: &World, col: usize, row: usize) -> bool {
    let width = world.width as i64;
    let height = world.height as i64;
    let c = col as i64;
    let r = row as i64;
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = (c + dx).rem_euclid(width);
            let ny = (r + dy).clamp(0, height - 1);
            if *world.hydrology.is_ocean.get(nx, ny) || *world.hydrology.is_lake.get(nx, ny) {
                return true;
            }
        }
    }
    false
}

/// Renders the world with the exact per-cell coloring `game_render::map`
/// uses for the 3D terrain mesh (`terrain_paint_color`, cell by cell) — a
/// verification view: what to actually look at to check slope/beach/blend
/// math (via `cargo run -p game_worldgen --example generate -- --paint`)
/// before any of it reaches the live 3D game, which isn't otherwise
/// possible to eyeball non-interactively.
pub fn terrain_paint_map(width: usize, height: usize, world: &World, sea_level: f32) -> RgbImage {
    let mut image: RgbImage = ImageBuffer::new(width as u32, height as u32);
    for y in 0..height {
        for x in 0..width {
            let (_, color) = terrain_paint_color(world, sea_level, x, y);
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
        ("Ice", terrain_color(Terrain::Ice)),
        ("Lake", terrain_color(Terrain::Lake)),
        ("Grassland", terrain_color(Terrain::Grassland)),
        ("Plains", terrain_color(Terrain::Plains)),
        ("Savanna", terrain_color(Terrain::Savanna)),
        ("Steppe", terrain_color(Terrain::Steppe)),
        ("Desert", terrain_color(Terrain::Desert)),
        ("Tundra", terrain_color(Terrain::Tundra)),
        ("Snow", terrain_color(Terrain::Snow)),
        ("Forest", feature_color(Feature::Forest)),
        ("Jungle", feature_color(Feature::Jungle)),
        ("Marsh", feature_color(Feature::Marsh)),
        ("Floodplains", feature_color(Feature::Floodplains)),
        ("Reef", feature_color(Feature::Reef)),
        ("Volcano", feature_color(Feature::Volcano)),
        ("Oasis", feature_color(Feature::Oasis)),
        ("River", RIVER_COLOR),
        ("Nation", [255, 255, 255]),
    ]
}

/// Renders each distinct landmass (from `stats::label_landmasses`) in its
/// own flat, distinct color, with ocean/lake/ice/coast and any speck too
/// small to count as a landmass all rendered as one dark "sea" color — a
/// verification view, independent of biome coloring, for checking the
/// actual continent count and how cleanly separated they are at a glance.
pub fn highlight_landmasses(width: usize, height: usize, labels: &Grid<i32>) -> RgbImage {
    let mut image: RgbImage = ImageBuffer::new(width as u32, height as u32);
    for y in 0..height {
        for x in 0..width {
            let id = *labels.get(x as i64, y as i64);
            let color = if id < 0 {
                [8, 16, 40]
            } else {
                landmass_color(id as u32)
            };
            image.put_pixel(x as u32, y as u32, Rgb(color));
        }
    }
    image
}

/// A distinct, readable color per landmass id via golden-angle hue rotation
/// — successive ids land far apart on the color wheel, so even a map with
/// dozens of landmasses never puts two similar hues next to each other.
fn landmass_color(id: u32) -> [u8; 3] {
    let hue = (id as f32 * 137.50777) % 360.0;
    hsv_to_rgb(hue, 0.55, 0.85)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [
        ((r1 + m) * 255.0) as u8,
        ((g1 + m) * 255.0) as u8,
        ((b1 + m) * 255.0) as u8,
    ]
}

/// The raw heightmap as grayscale — useful for sanity-checking generation
/// parameters (continent shapes, mountain belts) independent of biome
/// classification. See [`elevation_hypsometric`] for a more readable,
/// color-graded version.
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

/// A topographic-map-style ("hypsometric tint") rendering of the raw
/// heightmap: deep blue → shallow blue below `sea_level`, green → yellow →
/// brown → white above it. Much easier to read verticality from than flat
/// grayscale — the whole point of this view.
pub fn elevation_hypsometric(
    width: usize,
    height: usize,
    elevation: &ElevationMaps,
    sea_level: f32,
) -> RgbImage {
    let mut image: RgbImage = ImageBuffer::new(width as u32, height as u32);
    for y in 0..height {
        for x in 0..width {
            let h = *elevation.elevation.get(x as i64, y as i64);
            image.put_pixel(x as u32, y as u32, Rgb(hypsometric_color(h, sea_level)));
        }
    }
    image
}

fn hypsometric_color(elevation: f32, sea_level: f32) -> [u8; 3] {
    if elevation < sea_level {
        let t = (elevation / sea_level.max(0.001)).clamp(0.0, 1.0);
        blend([6, 14, 60], [130, 180, 220], t)
    } else {
        let land_span = (1.2 - sea_level).max(0.05);
        let t = ((elevation - sea_level) / land_span).clamp(0.0, 1.0);
        if t < 0.35 {
            blend([60, 120, 55], [190, 180, 90], t / 0.35)
        } else if t < 0.7 {
            blend([190, 180, 90], [130, 90, 60], (t - 0.35) / 0.35)
        } else {
            blend([130, 90, 60], [250, 250, 250], (t - 0.7) / 0.3)
        }
    }
}

/// Flat per-`Terrain` color, with none of `biome_map`'s baked-in shading
/// (elevation-band darkening, feature/river blending, ocean depth tint).
/// `biome_map` uses this as its base color before layering those on; a 3D
/// terrain mesh wants the flat version instead — that extra shading was
/// tuned to read well in a static top-down image, not to sit under real
/// mesh lighting/normals, and baked-in depth shading in particular fights
/// `game_render::map`'s own flat-clipped ocean floor.
pub fn terrain_color(terrain: Terrain) -> [u8; 3] {
    match terrain {
        Terrain::Ocean => [20, 60, 130],
        Terrain::Coast => [70, 130, 190],
        Terrain::Ice => [210, 230, 240],
        Terrain::Lake => [60, 120, 190],
        Terrain::Grassland => [90, 150, 60],
        Terrain::Plains => [190, 175, 90],
        Terrain::Savanna => [175, 165, 75],
        Terrain::Steppe => [195, 185, 140],
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
        Feature::Volcano => [180, 40, 20],
        Feature::Oasis => [50, 180, 160],
        Feature::Floodplains => [140, 160, 80],
        Feature::Reef => [230, 170, 140],
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
