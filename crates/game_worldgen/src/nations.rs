use crate::World;
use crate::biome::Terrain;
use crate::sampling::scatter_points;

/// Scatters `count` nation starting positions across habitable land,
/// spaced apart so they don't cluster, on the cylinder (wrapping in X).
///
/// This is placement only, not a nation *simulation* — no borders, no
/// growth, nothing political. It exists so "number of nations" is a
/// setting you can actually see change the generated map, ahead of any
/// real nation/settlement system.
pub fn place(world: &World, count: usize, seed: u64) -> Vec<(usize, usize)> {
    if count == 0 {
        return Vec::new();
    }

    let area = (world.width * world.height) as f64;
    // Target spacing if `count` points were evenly packed over the map,
    // scaled down so rejection sampling can actually satisfy it in
    // practice (a perfect packing would never succeed by random rejection).
    let min_spacing = (area / count as f64).sqrt() * 0.5;

    scatter_points(
        world.width,
        world.height,
        count,
        seed,
        min_spacing,
        |x, y| is_habitable(*world.biome.terrain.get(x as i64, y as i64)),
    )
}

fn is_habitable(terrain: Terrain) -> bool {
    !matches!(
        terrain,
        Terrain::Ocean | Terrain::Coast | Terrain::Lake | Terrain::Ice | Terrain::Snow
    )
}
