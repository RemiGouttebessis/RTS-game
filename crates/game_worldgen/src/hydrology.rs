use std::cmp::Ordering;

use crate::grid::Grid;

pub struct HydrologyMaps {
    pub is_ocean: Grid<bool>,
    /// Simplified stand-in for proper depression-filling hydrology: land
    /// cells with no lower neighbor (and a small ring around them), rather
    /// than exactly-computed drainage basins. Good enough for a preview
    /// map; worth revisiting if lake placement/shape ever needs to be exact.
    pub is_lake: Grid<bool>,
    pub is_river: Grid<bool>,
}

pub fn generate(
    width: usize,
    height: usize,
    elevation: &Grid<f32>,
    sea_level: f32,
    river_threshold: f32,
) -> HydrologyMaps {
    let mut is_ocean = Grid::<bool>::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let ocean = *elevation.get(x as i64, y as i64) < sea_level;
            is_ocean.set(x as i64, y as i64, ocean);
        }
    }

    // Flow direction: each land cell points at its steepest-descent
    // neighbor, or `None` if it's a local minimum (a sink — no ocean
    // neighbor is lower, so water pools there instead of draining out).
    let mut flow_target: Vec<Option<(i64, i64)>> = vec![None; width * height];
    let mut cells: Vec<(i64, i64)> = Vec::with_capacity(width * height);

    for y in 0..height {
        for x in 0..width {
            let pos = (x as i64, y as i64);
            cells.push(pos);
            if *is_ocean.get(pos.0, pos.1) {
                continue;
            }

            let here = *elevation.get(pos.0, pos.1);
            let mut best: Option<((i64, i64), f32)> = None;
            for (nx, ny) in elevation.neighbors(pos.0, pos.1) {
                let drop = here - *elevation.get(nx, ny);
                let is_better =
                    drop > 0.0 && best.map(|(_, best_drop)| drop > best_drop).unwrap_or(true);
                if is_better {
                    best = Some(((nx, ny), drop));
                }
            }
            flow_target[y * width + x] = best.map(|(target, _)| target);
        }
    }

    // Flow accumulation (D8): process cells from highest to lowest
    // elevation so every upstream cell has already deposited its flow
    // before it's passed on to whatever it drains into.
    cells.sort_by(|a, b| {
        let ea = *elevation.get(a.0, a.1);
        let eb = *elevation.get(b.0, b.1);
        eb.partial_cmp(&ea).unwrap_or(Ordering::Equal)
    });

    let mut flow_accumulation = Grid::<f32>::new(width, height);
    for y in 0..height {
        for x in 0..width {
            flow_accumulation.set(x as i64, y as i64, 1.0); // each cell's own rainfall unit
        }
    }
    for &(x, y) in &cells {
        if let Some((tx, ty)) = flow_target[(y as usize) * width + (x as usize)] {
            let amount = *flow_accumulation.get(x, y);
            let target_amount = *flow_accumulation.get(tx, ty);
            flow_accumulation.set(tx, ty, target_amount + amount);
        }
    }

    let mut max_land_accum = 0f32;
    for y in 0..height {
        for x in 0..width {
            let pos = (x as i64, y as i64);
            if !*is_ocean.get(pos.0, pos.1) {
                max_land_accum = max_land_accum.max(*flow_accumulation.get(pos.0, pos.1));
            }
        }
    }

    let mut is_river = Grid::<bool>::new(width, height);
    if max_land_accum > 0.0 {
        for y in 0..height {
            for x in 0..width {
                let pos = (x as i64, y as i64);
                if *is_ocean.get(pos.0, pos.1) {
                    continue;
                }
                let accum = *flow_accumulation.get(pos.0, pos.1);
                if accum / max_land_accum >= river_threshold {
                    is_river.set(pos.0, pos.1, true);
                }
            }
        }
    }

    // Lakes: sinks (no lower neighbor, not ocean), plus their immediate
    // near-equal-elevation neighbors, so a lake reads as a small pool
    // rather than a single pixel.
    let mut is_lake = Grid::<bool>::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let pos = (x as i64, y as i64);
            if *is_ocean.get(pos.0, pos.1) {
                continue;
            }
            if flow_target[y * width + x].is_some() {
                continue;
            }

            let here = *elevation.get(pos.0, pos.1);
            is_lake.set(pos.0, pos.1, true);
            for (nx, ny) in elevation.neighbors(pos.0, pos.1) {
                if *is_ocean.get(nx, ny) {
                    continue;
                }
                if (*elevation.get(nx, ny) - here).abs() < 0.01 {
                    is_lake.set(nx, ny, true);
                }
            }
        }
    }

    HydrologyMaps {
        is_ocean,
        is_lake,
        is_river,
    }
}
