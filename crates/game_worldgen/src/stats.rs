use crate::biome::Terrain;
use crate::grid::Grid;

/// Counts distinct connected landmasses, ignoring specks smaller than
/// `min_size` cells — the *actual* generated continent count, which can
/// legitimately differ from the `continent_radius`/`continent_count` input:
/// that input only controls the *expected* number of noise blobs, and sea
/// level, coastline fragmentation, and randomness can split one blob into
/// several islands or merge several into one landmass.
pub fn count_landmasses(terrain: &Grid<Terrain>, min_size: usize) -> usize {
    label_landmasses(terrain, min_size).1
}

/// Flood-fills connected landmasses (same rule as `count_landmasses`: 8-
/// connected, cylinder-wrap-aware, specks under `min_size` don't count) and
/// labels every cell with its landmass's 0-based index in flood-fill
/// encounter order, or `-1` for ocean/lake/ice/coast and discarded specks.
/// Shared by `count_landmasses` and the world-gen preview's continent-
/// highlight overlay, so both always agree on what counts as "a continent."
pub fn label_landmasses(terrain: &Grid<Terrain>, min_size: usize) -> (Grid<i32>, usize) {
    let mut labels = Grid::<i32>::new(terrain.width, terrain.height);
    for y in 0..terrain.height {
        for x in 0..terrain.width {
            labels.set(x as i64, y as i64, -1);
        }
    }

    let mut visited = Grid::<bool>::new(terrain.width, terrain.height);
    let mut stack = Vec::new();
    let mut next_id = 0i32;

    for y in 0..terrain.height {
        for x in 0..terrain.width {
            let pos = (x as i64, y as i64);
            if *visited.get(pos.0, pos.1) || !is_land(*terrain.get(pos.0, pos.1)) {
                continue;
            }

            stack.push(pos);
            visited.set(pos.0, pos.1, true);
            let mut member = Vec::new();
            while let Some((cx, cy)) = stack.pop() {
                member.push((cx, cy));
                for (nx, ny) in terrain.neighbors(cx, cy) {
                    if !*visited.get(nx, ny) && is_land(*terrain.get(nx, ny)) {
                        visited.set(nx, ny, true);
                        stack.push((nx, ny));
                    }
                }
            }

            if member.len() >= min_size {
                for (cx, cy) in member {
                    labels.set(cx, cy, next_id);
                }
                next_id += 1;
            }
        }
    }

    (labels, next_id as usize)
}

fn is_land(terrain: Terrain) -> bool {
    !matches!(
        terrain,
        Terrain::Ocean | Terrain::Coast | Terrain::Lake | Terrain::Ice
    )
}
