use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

use crate::grid::Grid;

pub struct HydrologyMaps {
    pub is_ocean: Grid<bool>,
    /// A filled depression (a real topographic basin, not a single sunken
    /// pixel) — see `generate`'s priority-flood pass.
    pub is_lake: Grid<bool>,
    pub is_river: Grid<bool>,
    /// The priority-flood pass's "filled" elevation for every cell — for a
    /// lake cell specifically, this is the basin's actual pour-point
    /// elevation, i.e. the real, correct water surface height for that
    /// *entire* lake (flat, like a real lake, and different from one lake to
    /// the next depending on where its basin sits). `image_export` and
    /// `game_render::map` use this instead of an arbitrary fixed depth the
    /// way ocean cells get clipped to — a lake's correct level was already
    /// computed here, no need to override it with a guess.
    pub filled: Grid<f32>,
}

/// One entry in the priority-flood frontier: a cell paired with its `filled`
/// elevation (see `generate`), ordered so a `BinaryHeap` pops the lowest
/// first. Elevation is always finite here (never NaN), so `total_cmp` gives
/// a total order without needing a NaN-checked float wrapper.
struct Frontier {
    filled: f32,
    x: i64,
    y: i64,
}

impl PartialEq for Frontier {
    fn eq(&self, other: &Self) -> bool {
        self.filled == other.filled
    }
}
impl Eq for Frontier {}
impl PartialOrd for Frontier {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Frontier {
    fn cmp(&self, other: &Self) -> Ordering {
        self.filled.total_cmp(&other.filled)
    }
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

    // Priority-flood depression filling (Barnes et al.): starting from the
    // ocean (the one boundary every drop of rain eventually has to reach),
    // repeatedly pop the lowest-`filled` unvisited cell and relax its
    // neighbors to `max(their own elevation, this cell's filled elevation)`.
    // The result is a "filled" surface with no interior local minima except
    // the ocean itself — every land cell has a monotonically non-increasing
    // 8-connected path down to the ocean along `filled` values, so flow
    // direction (recorded as "which cell did I get relaxed from") is by
    // construction always defined and always actually drains, rather than
    // stopping dead at every small pit the way a plain steepest-descent scan
    // does. A cell where `filled > elevation` sat inside a depression that
    // got submerged to reach that path — i.e. a lake, and its exact extent,
    // not an approximation of one. This replaces what used to be two
    // separate heuristics (a steepest-descent-with-`None`-sinks pass for
    // rivers, and a "no lower neighbor + near-equal neighbors" pass for
    // lakes) with one pass that both derive from correctly, so rivers and
    // lakes actually agree with the terrain's real verticality instead of
    // each approximating it their own way.
    let mut filled = Grid::<f32>::new(width, height);
    let mut visited = Grid::<bool>::new(width, height);
    let mut flow_target: Vec<Option<(i64, i64)>> = vec![None; width * height];
    let mut heap: BinaryHeap<Reverse<Frontier>> = BinaryHeap::new();

    for y in 0..height {
        for x in 0..width {
            let pos = (x as i64, y as i64);
            if *is_ocean.get(pos.0, pos.1) {
                let h = *elevation.get(pos.0, pos.1);
                visited.set(pos.0, pos.1, true);
                filled.set(pos.0, pos.1, h);
                heap.push(Reverse(Frontier {
                    filled: h,
                    x: pos.0,
                    y: pos.1,
                }));
            }
        }
    }

    while let Some(Reverse(here)) = heap.pop() {
        for (nx, ny) in elevation.neighbors(here.x, here.y) {
            let wx = nx.rem_euclid(width as i64);
            if *visited.get(wx, ny) {
                continue;
            }
            visited.set(wx, ny, true);

            let raw = *elevation.get(wx, ny);
            let f = raw.max(here.filled);
            filled.set(wx, ny, f);
            flow_target[ny as usize * width + wx as usize] = Some((here.x, here.y));
            heap.push(Reverse(Frontier {
                filled: f,
                x: wx,
                y: ny,
            }));
        }
    }

    // Flow accumulation (D8): process cells from highest to lowest `filled`
    // so every upstream cell has already deposited its flow before it's
    // passed on to whatever it drains into — matches the order `filled` was
    // actually assigned in above, so every target really has been visited
    // first regardless of how raw elevation compares within a lake's flat
    // surface.
    let mut cells: Vec<(i64, i64)> = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x as i64, y as i64)))
        .collect();
    cells.sort_by(|a, b| {
        let fa = *filled.get(a.0, a.1);
        let fb = *filled.get(b.0, b.1);
        fb.total_cmp(&fa)
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

    // A lake is exactly the cells priority-flood had to submerge to find a
    // drainage path — a real filled basin, shaped however the surrounding
    // terrain actually shapes it, rather than a sink pixel plus whatever
    // happened to be within 0.01 of its elevation.
    const LAKE_EPSILON: f32 = 0.001;
    let mut is_lake = Grid::<bool>::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let pos = (x as i64, y as i64);
            if *is_ocean.get(pos.0, pos.1) {
                continue;
            }
            let raw = *elevation.get(pos.0, pos.1);
            let f = *filled.get(pos.0, pos.1);
            if f > raw + LAKE_EPSILON {
                is_lake.set(pos.0, pos.1, true);
            }
        }
    }

    HydrologyMaps {
        is_ocean,
        is_lake,
        is_river,
        filled,
    }
}
