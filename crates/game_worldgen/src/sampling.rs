//! Point scattering shared by continent-seed placement (`elevation`) and
//! nation-starting-position placement (`nations`) — both are "N points,
//! spaced apart, satisfying some per-cell condition" on the same cylinder.

use crate::noise::splitmix64;

/// Scatters up to `count` grid cells across a `width`x`height` cylinder
/// (wrapping in X), each accepted by `accept` and at least `min_spacing`
/// (Euclidean, wrap-aware) from every other point already placed. Rejection
/// sampling: may return fewer than `count` points if there isn't room to
/// satisfy the spacing/acceptance constraints.
pub fn scatter_points(
    width: usize,
    height: usize,
    count: usize,
    seed: u64,
    min_spacing: f64,
    accept: impl Fn(usize, usize) -> bool,
) -> Vec<(usize, usize)> {
    if count == 0 || width == 0 || height == 0 {
        return Vec::new();
    }

    let min_dist_sq = min_spacing * min_spacing;
    let mut positions: Vec<(usize, usize)> = Vec::with_capacity(count);
    let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
    let max_attempts = count * 500;

    for _ in 0..max_attempts {
        if positions.len() >= count {
            break;
        }

        state = splitmix64(state);
        let x = (state % width as u64) as usize;
        state = splitmix64(state);
        let y = (state % height as u64) as usize;

        if !accept(x, y) {
            continue;
        }

        let far_enough = positions
            .iter()
            .all(|&(px, py)| wrapped_dist_sq(x, y, px, py, width) >= min_dist_sq);

        if far_enough {
            positions.push((x, y));
        }
    }

    positions
}

/// Squared distance accounting for the cylinder's X wrap (the shorter of
/// going left or right around).
pub fn wrapped_dist_sq(x1: usize, y1: usize, x2: usize, y2: usize, width: usize) -> f64 {
    let raw_dx = (x1 as f64 - x2 as f64).abs();
    let dx = raw_dx.min(width as f64 - raw_dx);
    let dy = y1 as f64 - y2 as f64;
    dx * dx + dy * dy
}
