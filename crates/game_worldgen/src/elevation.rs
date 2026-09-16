use std::collections::{HashMap, HashSet, VecDeque};
use std::f64::consts::TAU;

use crate::grid::Grid;
use crate::noise::{Fbm3, RidgedFbm3, cylinder_point, splitmix64};
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
    // Continents come from layered noise, the same family of noise used for
    // everything else in this generator — that's what makes the coastlines
    // read as organic (branching peninsulas, irregular bays) rather than
    // "circle with a noisy edge." A seed-point/distance-field approach was
    // tried instead (guarantees an exact landmass count by construction);
    // it looked conspicuously artificial no matter how much edge noise was
    // layered on, because the underlying shape is still fundamentally a
    // blob around each seed. Noise doesn't guarantee an exact count on its
    // own, so `correct_continent_count` below nudges the result afterward
    // instead — organic shape from generation, correct count from
    // post-processing, rather than trying to get both from one mechanism.
    let continent_radius = preset.continent_count.max(1) as f64 / TAU;

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
            let (cx, cy, cz) = cylinder_point(x as f64, y as f64, width as f64, continent_radius);
            let continent_v = continent.sample(cx, cy, cz);

            // Mild roughening only — this is texture, not a second
            // continent-scale signal, so it stays at low amplitude and low
            // relative frequency.
            let (dx, dy, dz) =
                cylinder_point(x as f64, y as f64, width as f64, continent_radius * 2.0);
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
            // that mask. The mask threshold (0.62) trades some of "most land
            // is ordinary terrain" for chunkier, more contiguous massifs —
            // a tighter threshold (this used to be 0.72) reads as scattered
            // hill speckle rather than the kind of big, cohesive mountain
            // range a "massif" should be.
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

            const BELT_THRESHOLD: f64 = 0.62;
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

    if preset.correct_continents {
        correct_continent_count(
            &mut elevation,
            preset.sea_level,
            preset.continent_count,
            seed,
            width,
            height,
        );
    }

    ElevationMaps {
        elevation,
        mountain_mask,
    }
}

/// Nudges the noise-generated landmass count toward `target_count`: merges
/// the closest pair of landmasses (a land bridge between them) when there
/// are too many, or splits the largest landmass (a strait cut across its
/// shorter axis) when there are too few. Runs directly on the elevation
/// grid, after normal generation, so every downstream stage (hydrology,
/// biome) sees a consistent result rather than needing its own awareness of
/// the correction.
fn correct_continent_count(
    elevation: &mut Grid<f32>,
    sea_level: f32,
    target_count: u32,
    seed: u64,
    width: usize,
    height: usize,
) {
    let target = target_count.max(1) as usize;
    let min_size = ((width * height) as f64 * 0.0015).max(1.0) as usize;
    let max_iterations = target * 2 + 20;

    for i in 0..max_iterations {
        let mut components = label_land_components(elevation, sea_level);
        components.retain(|c| c.len() >= min_size);

        if components.len() == target {
            break;
        }

        if components.len() > target {
            let Some((a, b)) = closest_pair(&components, width) else {
                break;
            };
            let from = centroid(&components[a]);
            let to = centroid(&components[b]);
            bridge_land(
                elevation,
                sea_level,
                from,
                to,
                width,
                height,
                seed.wrapping_add(i as u64),
            );
        } else {
            let Some(largest) = components.iter().max_by_key(|c| c.len()) else {
                break;
            };
            split_land(elevation, sea_level, largest, seed.wrapping_add(i as u64));
        }
    }
}

/// Flood-fills connected land (elevation >= sea_level), 8-connected and
/// cylinder-wrap-aware (via `Grid::neighbors`).
fn label_land_components(elevation: &Grid<f32>, sea_level: f32) -> Vec<Vec<(usize, usize)>> {
    let width = elevation.width;
    let height = elevation.height;
    let mut visited = Grid::<bool>::new(width, height);
    let mut components = Vec::new();

    for y in 0..height {
        for x in 0..width {
            if *visited.get(x as i64, y as i64) || *elevation.get(x as i64, y as i64) < sea_level {
                continue;
            }

            // Kept as `(i64, i64)` throughout the walk (matching
            // `stats::count_landmasses`'s pattern) and only converted to
            // `(usize, usize)` when stored below: `Grid::neighbors` returns
            // *raw* offsets (e.g. `-1` just past the west edge), relying on
            // `Grid::get`/`set` to wrap them via `rem_euclid` — casting a
            // raw negative offset straight to `usize` (as this used to)
            // wraps to `usize::MAX` instead, corrupting every component
            // that touched the map's X seam.
            let mut stack = vec![(x as i64, y as i64)];
            visited.set(x as i64, y as i64, true);
            let mut member = Vec::new();

            while let Some((cx, cy)) = stack.pop() {
                member.push((cx.rem_euclid(width as i64) as usize, cy as usize));
                for (nx, ny) in elevation.neighbors(cx, cy) {
                    if !*visited.get(nx, ny) && *elevation.get(nx, ny) >= sea_level {
                        visited.set(nx, ny, true);
                        stack.push((nx, ny));
                    }
                }
            }

            components.push(member);
        }
    }

    components
}

fn centroid(component: &[(usize, usize)]) -> (f64, f64) {
    let n = component.len() as f64;
    let sum_x: f64 = component.iter().map(|&(x, _)| x as f64).sum();
    let sum_y: f64 = component.iter().map(|&(_, y)| y as f64).sum();
    (sum_x / n, sum_y / n)
}

/// The closest pair of components by centroid distance (wrap-aware) — a
/// cheap approximation of "closest landmasses" that's plenty good enough for
/// picking a merge target.
fn closest_pair(components: &[Vec<(usize, usize)>], width: usize) -> Option<(usize, usize)> {
    if components.len() < 2 {
        return None;
    }

    let centroids: Vec<(f64, f64)> = components.iter().map(|c| centroid(c)).collect();
    let mut best: Option<(usize, usize, f64)> = None;

    for i in 0..centroids.len() {
        for j in (i + 1)..centroids.len() {
            let raw_dx = (centroids[i].0 - centroids[j].0).abs();
            let dx = raw_dx.min(width as f64 - raw_dx);
            let dy = centroids[i].1 - centroids[j].1;
            let dist_sq = dx * dx + dy * dy;

            let is_better = best
                .map(|(_, _, best_dist)| dist_sq < best_dist)
                .unwrap_or(true);
            if is_better {
                best = Some((i, j, dist_sq));
            }
        }
    }

    best.map(|(i, j, _)| (i, j))
}

/// Raises a strip of ocean to just above sea level along a meandering path
/// between two points, connecting whatever land is at each end into one
/// landmass — a proper isthmus, not a thread.
///
/// The strip used to be a fixed radius-1 (3px) band around a dead-straight
/// centerline, regardless of map resolution or how big the landmasses being
/// joined were. At any resolution big enough to look good otherwise, that's
/// a handful of pixels swallowed by antialiasing and the ocean's depth
/// shading — the merge succeeds by the numbers (the two landmasses really
/// are one connected component afterward) but reads as nothing having
/// happened, which is the "merge doesn't work" symptom. The width now scales
/// with the map, and both the centerline and the width itself wander via two
/// independent sine waves (different frequency/phase per `jitter`, so
/// repeated bridges in one generation don't all wobble in lockstep) — a
/// dead-straight strait reads as artificial the same way a dead-straight
/// split does.
#[allow(clippy::too_many_arguments)]
fn bridge_land(
    elevation: &mut Grid<f32>,
    sea_level: f32,
    from: (f64, f64),
    to: (f64, f64),
    width: usize,
    height: usize,
    jitter: u64,
) {
    let raw_dx = to.0 - from.0;
    let to_x = if raw_dx.abs() > width as f64 / 2.0 {
        if raw_dx > 0.0 {
            to.0 - width as f64
        } else {
            to.0 + width as f64
        }
    } else {
        to.0
    };

    let dx = to_x - from.0;
    let dy = to.1 - from.1;
    let distance = (dx * dx + dy * dy).sqrt();
    if distance < 1.0 {
        return;
    }
    let steps = (distance.ceil() as usize).max(1);

    // Perpendicular unit vector — the meander and width both displace along
    // this, so they stay transverse to the bridge's direction of travel
    // regardless of its orientation.
    let perp = (-dy / distance, dx / distance);

    let base_half_width = strait_half_width(width, height);
    let phase_a = (jitter % 997) as f64 * 0.0177;
    let phase_b = ((jitter / 997) % 997) as f64 * 0.0231;
    let cycles = 1.5 + (jitter % 3) as f64;

    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let cx = from.0 + dx * t;
        let cy = from.1 + dy * t;

        let meander = (t * TAU * cycles + phase_a).sin() * base_half_width * 1.8;
        let half_width = (base_half_width
            + (t * TAU * cycles * 1.7 + phase_b).sin() * base_half_width * 0.5)
            .max(1.5);

        let center_x = cx + perp.0 * meander;
        let center_y = cy + perp.1 * meander;

        let w = half_width.ceil() as i64;
        for oy in -w..=w {
            for ox in -w..=w {
                let dist = ((ox * ox + oy * oy) as f64).sqrt();
                if dist > half_width {
                    continue;
                }
                let px = (center_x.round() as i64 + ox).rem_euclid(width as i64);
                let py = (center_y.round() as i64 + oy).clamp(0, height as i64 - 1);
                let current = *elevation.get(px, py);
                if current < sea_level {
                    // A radial dome (highest at the centerline, tapering to
                    // just above sea level at the edge) plus a tiny per-cell
                    // jitter, rather than one flat constant everywhere, so
                    // the strip has a believable cross-section in the
                    // elevation view instead of reading as a mesa.
                    // `hydrology::generate`'s priority-flood drainage handles
                    // a wide, mostly-flat raised area correctly regardless
                    // (it fills to the nearest lower pour point rather than
                    // flagging "no strictly-lower neighbor" as a lake), so
                    // this is purely cosmetic, not load-bearing for the
                    // merge actually reading as land.
                    let profile = 1.0 - (dist / half_width).clamp(0.0, 1.0);
                    let jitter_n = cell_jitter(px, py, jitter);
                    let target = sea_level as f64 + 0.02 + profile * 0.05 + jitter_n * 0.01;
                    elevation.set(px, py, target as f32);
                }
            }
        }
    }
}

/// A small, deterministic per-cell elevation perturbation (roughly
/// `-0.005..0.005`) — just enough that two adjacent cells raised by the same
/// stamp are never exactly equal, so the strip reads as textured ground
/// rather than a dead-flat mesa in the elevation view.
fn cell_jitter(x: i64, y: i64, seed: u64) -> f64 {
    let h = splitmix64(
        seed ^ (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F),
    );
    (h >> 11) as f64 / (1u64 << 53) as f64 - 0.5
}

/// The half-width (in cells) for both a merge's isthmus and a split's
/// strait, scaled to the map so it reads as real geography at any
/// resolution rather than a fixed pixel count that's generous at 128-wide
/// and invisible at 2048-wide.
fn strait_half_width(width: usize, height: usize) -> f64 {
    (width.min(height) as f64 * 0.012).clamp(2.5, 22.0)
}

/// Cuts a strait across a landmass, splitting it into two connected pieces.
///
/// A straight (or gently wiggled) coordinate-line cut — the previous
/// approach — only reliably separates a convex-ish blob. Noise-generated
/// continents are routinely concave (a horseshoe bay, a peninsula that
/// wraps back around), and on those a coordinate-line cut carves a scratch
/// across the landmass *without* actually disconnecting it: the two "halves"
/// are still joined around the open side. `label_land_components` then still
/// reports one landmass, so `correct_continent_count` tries again next
/// iteration with a different jitter — and each failed attempt leaves
/// another thin, pointless channel of ocean gouged into the continent that
/// never finishes separating anything. That's the "river of ocean" artifact.
///
/// Instead this does a true geodesic split: seed two points at the
/// landmass's approximate diameter (farthest-from-farthest, via double BFS,
/// so a horseshoe gets seeds on its two far ends rather than its bounding
/// box's corners), then a simultaneous multi-source BFS over *only this
/// component's cells* assigns every cell to whichever seed's flood front
/// reaches it first. That partition follows the landmass's actual
/// connectivity, so it can't fail to separate it, however concave.
///
/// The zero-width seam between the two BFS halves is then widened into an
/// actual strait: a second multi-source BFS, seeded from every cell on that
/// seam, gives every component cell its ring-distance from the seam, and
/// each cell within a (noise-varied, map-scaled) half-width of it gets
/// cleared to ocean — same idea, and same `strait_half_width` scale, as
/// `bridge_land`'s isthmus. Clearing is symmetric around the seam and always
/// includes the seam cells themselves (half-width is never allowed below
/// 1.0), so separation is still guaranteed regardless of how the per-cell
/// noise happens to fall. Only component cells are ever touched, so a nearby
/// unrelated landmass sharing a row/column isn't affected. `jitter` seeds
/// both which component cell starts the first BFS and the width noise, for
/// variety across repeated calls.
fn split_land(
    elevation: &mut Grid<f32>,
    sea_level: f32,
    component: &[(usize, usize)],
    jitter: u64,
) {
    if component.len() < 2 {
        return;
    }

    let width = elevation.width;
    let height = elevation.height;
    let members: HashSet<(i64, i64)> = component
        .iter()
        .map(|&(x, y)| (x as i64, y as i64))
        .collect();

    let start = component[jitter as usize % component.len()];
    let a = farthest_from(elevation, &members, (start.0 as i64, start.1 as i64));
    let b = farthest_from(elevation, &members, a);
    if a == b {
        return;
    }

    // Multi-source BFS: both seeds start in the same FIFO queue at distance
    // 0, so cells are dequeued (and thus labeled) in nondecreasing distance
    // from whichever seed is closer — a standard geodesic Voronoi split.
    let mut label: HashMap<(i64, i64), bool> = HashMap::new();
    label.insert(a, false);
    label.insert(b, true);
    let mut queue: VecDeque<(i64, i64, bool)> =
        VecDeque::from([(a.0, a.1, false), (b.0, b.1, true)]);

    while let Some((cx, cy, side)) = queue.pop_front() {
        for (nx, ny) in elevation.neighbors(cx, cy) {
            let wrapped = (wrap_x(nx, width), ny);
            if members.contains(&wrapped) && !label.contains_key(&wrapped) {
                label.insert(wrapped, side);
                queue.push_back((wrapped.0, wrapped.1, side));
            }
        }
    }

    let seam: Vec<(i64, i64)> = members
        .iter()
        .filter(|&&cell| {
            let side = label[&cell];
            elevation
                .neighbors(cell.0, cell.1)
                .any(|(nx, ny)| label.get(&(wrap_x(nx, width), ny)) == Some(&!side))
        })
        .copied()
        .collect();

    let mut dist_from_seam: HashMap<(i64, i64), u32> = HashMap::new();
    let mut ring_queue: VecDeque<(i64, i64)> = VecDeque::new();
    for &cell in &seam {
        dist_from_seam.insert(cell, 0);
        ring_queue.push_back(cell);
    }
    while let Some(cell) = ring_queue.pop_front() {
        let d = dist_from_seam[&cell];
        for (nx, ny) in elevation.neighbors(cell.0, cell.1) {
            let wrapped = (wrap_x(nx, width), ny);
            if members.contains(&wrapped) && !dist_from_seam.contains_key(&wrapped) {
                dist_from_seam.insert(wrapped, d + 1);
                ring_queue.push_back(wrapped);
            }
        }
    }

    let base_half_width = strait_half_width(width, height);
    let width_noise = Fbm3::new(jitter ^ 0x5EED_5EED, 3, 2.0, 0.5);
    let noise_radius = base_half_width.max(4.0) * 0.6;

    let cut: Vec<(i64, i64)> = members
        .iter()
        .filter(|&&cell| {
            let d = *dist_from_seam.get(&cell).unwrap_or(&0) as f64;
            let (nx, ny, nz) =
                cylinder_point(cell.0 as f64, cell.1 as f64, width as f64, noise_radius);
            let n = width_noise.sample(nx, ny, nz);
            let half_width = (base_half_width + n * base_half_width * 0.6).max(1.0);
            d <= half_width
        })
        .copied()
        .collect();

    for (cx, cy) in cut {
        elevation.set(cx, cy, sea_level - 0.05);
    }
}

fn wrap_x(x: i64, width: usize) -> i64 {
    x.rem_euclid(width as i64)
}

/// BFS from `start` over `members` only (wrap-aware), returning the cell
/// last dequeued — the one at maximum BFS distance from `start`. Called
/// twice in a row (second call seeded with the first's result) is the
/// standard double-BFS approximation of a graph's diameter endpoints.
fn farthest_from(
    elevation: &Grid<f32>,
    members: &HashSet<(i64, i64)>,
    start: (i64, i64),
) -> (i64, i64) {
    let mut visited: HashSet<(i64, i64)> = HashSet::from([start]);
    let mut queue: VecDeque<(i64, i64)> = VecDeque::from([start]);
    let mut farthest = start;

    while let Some(cell) = queue.pop_front() {
        farthest = cell;
        for (nx, ny) in elevation.neighbors(cell.0, cell.1) {
            let wrapped = (wrap_x(nx, elevation.width), ny);
            if members.contains(&wrapped) && !visited.contains(&wrapped) {
                visited.insert(wrapped);
                queue.push_back(wrapped);
            }
        }
    }

    farthest
}
