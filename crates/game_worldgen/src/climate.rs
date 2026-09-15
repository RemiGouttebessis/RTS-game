use crate::grid::Grid;
use crate::noise::{Fbm3, cylinder_point};
use crate::preset::Preset;

pub struct ClimateMaps {
    /// `0..1`, 1 = hottest.
    pub temperature: Grid<f32>,
    /// `0..1`, 1 = wettest.
    pub moisture: Grid<f32>,
}

pub fn generate(
    width: usize,
    height: usize,
    seed: u64,
    preset: &Preset,
    elevation: &Grid<f32>,
) -> ClimateMaps {
    let temp_noise = Fbm3::new(seed.wrapping_add(10), 4, 2.0, 0.5);
    let moisture_noise = Fbm3::new(seed.wrapping_add(11), 5, 2.0, 0.5);

    let mut temperature = Grid::<f32>::new(width, height);
    let mut moisture = Grid::<f32>::new(width, height);

    for y in 0..height {
        for x in 0..width {
            // Absolute latitude: 0 at the equator, 1 at the poles.
            let lat = ((y as f64 / (height - 1).max(1) as f64) * 2.0 - 1.0).abs();
            let base_temp = 1.0 - lat;

            let (tx, ty, tz) = cylinder_point(x as f64, y as f64, width as f64, 3.0);
            let temp_variation = temp_noise.sample(tx, ty, tz) * 0.1;

            let elev = *elevation.get(x as i64, y as i64) as f64;
            // Only altitude meaningfully above typical land cools things
            // down — a lapse-rate stand-in, not a real atmospheric model.
            let altitude_cooling = (elev - 0.5).max(0.0) * 0.6;

            let temp = (base_temp + temp_variation - altitude_cooling
                + preset.temperature_bias as f64)
                .clamp(0.0, 1.0);
            temperature.set(x as i64, y as i64, temp as f32);

            let (mx, my, mz) = cylinder_point(x as f64, y as f64, width as f64, 3.5);
            let moisture_v = moisture_noise.sample(mx, my, mz) * 0.5 + 0.5;
            let m = (moisture_v + preset.moisture_bias as f64).clamp(0.0, 1.0);
            moisture.set(x as i64, y as i64, m as f32);
        }
    }

    ClimateMaps {
        temperature,
        moisture,
    }
}

/// Raises moisture near ocean/lake/river — real coastal and riparian areas
/// are wetter than what a pure noise layer captures. Called after hydrology
/// so it knows where the water actually ended up.
pub fn boost_near_water(
    moisture: &mut Grid<f32>,
    is_ocean: &Grid<bool>,
    is_lake: &Grid<bool>,
    is_river: &Grid<bool>,
) {
    for y in 0..moisture.height {
        for x in 0..moisture.width {
            let pos = (x as i64, y as i64);
            if *is_ocean.get(pos.0, pos.1) {
                continue;
            }

            let on_water_edge = is_ocean
                .neighbors(pos.0, pos.1)
                .any(|(nx, ny)| *is_ocean.get(nx, ny))
                || *is_lake.get(pos.0, pos.1)
                || is_lake
                    .neighbors(pos.0, pos.1)
                    .any(|(nx, ny)| *is_lake.get(nx, ny))
                || *is_river.get(pos.0, pos.1);

            if on_water_edge {
                let current = *moisture.get(pos.0, pos.1);
                moisture.set(pos.0, pos.1, (current + 0.2).min(1.0));
            }
        }
    }
}
