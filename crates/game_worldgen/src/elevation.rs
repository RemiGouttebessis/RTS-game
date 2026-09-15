use crate::grid::Grid;
use crate::noise::{Fbm3, RidgedFbm3, cylinder_point};
use crate::preset::Preset;

pub struct ElevationMaps {
    /// Normalized `0..1` (mountains can push a cell's raw value slightly
    /// above 1 before it's clamped — that's fine, they're supposed to be the
    /// highest points).
    pub elevation: Grid<f32>,
    /// How "mountainous" this region's tectonics are, `0..1`. Separate from
    /// elevation itself so biome/hills classification can ask "is this
    /// terrain rugged" independent of "is this terrain high" (a plateau is
    /// high but not rugged; a coastal mountain range is rugged near sea
    /// level).
    pub mountain_mask: Grid<f32>,
}

pub fn generate(width: usize, height: usize, seed: u64, preset: &Preset) -> ElevationMaps {
    let continent = Fbm3::new(seed, preset.continent_octaves, 2.0, 0.5);
    let detail = Fbm3::new(seed.wrapping_add(1), 3, 2.0, 0.5);
    let belt = Fbm3::new(seed.wrapping_add(2), 3, 2.0, 0.5);
    let ridged = RidgedFbm3::new(seed.wrapping_add(3), 4, 2.0, 0.5);

    let mut base = Grid::<f32>::new(width, height);
    let mut mountain_mask = Grid::<f32>::new(width, height);

    let mut min = f64::MAX;
    let mut max = f64::MIN;

    for y in 0..height {
        for x in 0..width {
            let (cx, cy, cz) =
                cylinder_point(x as f64, y as f64, width as f64, preset.continent_radius);
            let continent_v = continent.sample(cx, cy, cz);

            // Mild roughening only — this is texture, not a second
            // continent-scale signal, so it stays at low amplitude and low
            // relative frequency.
            let (dx, dy, dz) = cylinder_point(
                x as f64,
                y as f64,
                width as f64,
                preset.continent_radius * 2.0,
            );
            let detail_v = detail.sample(dx, dy, dz) * 0.08;

            let mut h = continent_v + detail_v;

            // Latitude falloff: pulls elevation down near the poles so
            // there's meaningfully more ocean/ice-adjacent area at the map
            // edges, Civ-style, rather than land running to the pole.
            let lat = (y as f64 / (height - 1).max(1) as f64) * 2.0 - 1.0; // -1..1
            let pole_falloff = 1.0 - (lat.abs() - 0.75).max(0.0) / 0.25 * 0.6;
            h *= pole_falloff;

            base.set(x as i64, y as i64, h as f32);
            min = min.min(h);
            max = max.max(h);

            // Mountain belts: a low-frequency mask picks *where* belts run,
            // a high-frequency ridged layer gives them jagged shape within
            // that mask. The mask threshold (0.72) is intentionally tight —
            // most land should be ordinary terrain, with mountains as
            // distinct ranges rather than uniform speckle everywhere.
            let (bx, by, bz) = cylinder_point(
                x as f64,
                y as f64,
                width as f64,
                preset.mountain_belt_radius,
            );
            let belt_v = (belt.sample(bx, by, bz) * 0.5 + 0.5).clamp(0.0, 1.0);

            let (rx, ry, rz) = cylinder_point(
                x as f64,
                y as f64,
                width as f64,
                preset.mountain_belt_radius * 6.0,
            );
            let ridge_v = ridged.sample(rx, ry, rz);

            const BELT_THRESHOLD: f64 = 0.72;
            let mountain = (((belt_v - BELT_THRESHOLD).max(0.0) / (1.0 - BELT_THRESHOLD))
                * ridge_v)
                .clamp(0.0, 1.0);
            mountain_mask.set(x as i64, y as i64, mountain as f32);
        }
    }

    // Normalize the base continent shape *before* adding mountains, so a
    // few tall peaks can't compress the sea-level threshold's meaning for
    // the rest of the map (a global min/max taken after adding mountains
    // would do exactly that).
    let mut elevation = Grid::<f32>::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let h = *base.get(x as i64, y as i64) as f64;
            let normalized = if max > min {
                (h - min) / (max - min)
            } else {
                0.5
            };

            let mountain = *mountain_mask.get(x as i64, y as i64) as f64;
            let with_mountains = (normalized + mountain * preset.mountain_strength as f64) as f32;
            elevation.set(x as i64, y as i64, with_mountains.clamp(0.0, 1.2));
        }
    }

    ElevationMaps {
        elevation,
        mountain_mask,
    }
}
