use crate::biome::Terrain;
use crate::grid::Grid;

/// Counts distinct connected landmasses, ignoring specks smaller than
/// `min_size` cells — the *actual* generated continent count, which can
/// legitimately differ from the `continent_radius`/`continent_count` input:
/// that input only controls the *expected* number of noise blobs, and sea
/// level, coastline fragmentation, and randomness can split one blob into
/// several islands or merge several into one landmass.
pub fn count_landmasses(terrain: &Grid<Terrain>, min_size: usize) -> usize {
    let mut visited = Grid::<bool>::new(terrain.width, terrain.height);
    let mut stack = Vec::new();
    let mut count = 0;

    for y in 0..terrain.height {
        for x in 0..terrain.width {
            let pos = (x as i64, y as i64);
            if *visited.get(pos.0, pos.1) || !is_land(*terrain.get(pos.0, pos.1)) {
                continue;
            }

            let mut size = 0;
            stack.push(pos);
            visited.set(pos.0, pos.1, true);
            while let Some((cx, cy)) = stack.pop() {
                size += 1;
                for (nx, ny) in terrain.neighbors(cx, cy) {
                    if !*visited.get(nx, ny) && is_land(*terrain.get(nx, ny)) {
                        visited.set(nx, ny, true);
                        stack.push((nx, ny));
                    }
                }
            }

            if size >= min_size {
                count += 1;
            }
        }
    }

    count
}

fn is_land(terrain: Terrain) -> bool {
    !matches!(terrain, Terrain::Ocean | Terrain::Coast | Terrain::Lake)
}
