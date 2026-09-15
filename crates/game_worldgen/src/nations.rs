use crate::World;
use crate::biome::Terrain;

/// Scatters `count` nation starting positions across habitable land,
/// spaced apart so they don't cluster, on the cylinder (wrapping in X).
///
/// This is placement only, not a nation *simulation* — no borders, no
/// growth, nothing political. It exists so "number of nations" is a
/// setting you can actually see change the generated map, ahead of any
/// real nation/settlement system.
pub fn place(world: &World, count: usize, seed: u64) -> Vec<(usize, usize)> {
    if count == 0 || world.width == 0 || world.height == 0 {
        return Vec::new();
    }

    let area = (world.width * world.height) as f64;
    // Target spacing if `count` points were evenly packed over the map,
    // scaled down so rejection sampling can actually satisfy it in
    // practice (a perfect packing would never succeed by random rejection).
    let min_spacing = (area / count as f64).sqrt() * 0.5;
    let min_dist_sq = min_spacing * min_spacing;

    let mut positions: Vec<(usize, usize)> = Vec::with_capacity(count);
    let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
    let max_attempts = count * 500;

    for _ in 0..max_attempts {
        if positions.len() >= count {
            break;
        }

        state = splitmix64(state);
        let x = (state % world.width as u64) as usize;
        state = splitmix64(state);
        let y = (state % world.height as u64) as usize;

        if !is_habitable(*world.biome.terrain.get(x as i64, y as i64)) {
            continue;
        }

        let far_enough = positions
            .iter()
            .all(|&(px, py)| wrapped_dist_sq(x, y, px, py, world.width) >= min_dist_sq);

        if far_enough {
            positions.push((x, y));
        }
    }

    positions
}

fn is_habitable(terrain: Terrain) -> bool {
    !matches!(
        terrain,
        Terrain::Ocean | Terrain::Coast | Terrain::Lake | Terrain::Snow
    )
}

/// Squared distance accounting for the cylinder's X wrap (the shorter of
/// going left or right around).
fn wrapped_dist_sq(x1: usize, y1: usize, x2: usize, y2: usize, width: usize) -> f64 {
    let raw_dx = (x1 as f64 - x2 as f64).abs();
    let dx = raw_dx.min(width as f64 - raw_dx);
    let dy = y1 as f64 - y2 as f64;
    dx * dx + dy * dy
}

fn splitmix64(x: u64) -> u64 {
    let x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
