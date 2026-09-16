//! Hand-rolled 3D gradient (Perlin) noise, plus fBm/ridged layering.
//!
//! Why 3D noise for a 2D map: the world is a cylinder, so the map must wrap
//! seamlessly along X (longitude) but not along Y (latitude/poles). Sampling
//! a 2D noise field directly along X produces a visible seam at the wrap.
//! Instead, each map column's X coordinate is mapped to an angle and sampled
//! on a circle embedded in the noise field's X/Z plane (see
//! [`cylinder_point`]) — walking all the way around that circle returns
//! exactly to the start, so the noise is seamless by construction, with no
//! special-casing needed at the edge.

use std::f64::consts::TAU;

const PERM_SIZE: usize = 256;

/// Ken Perlin's "Improved Noise" (2002), seeded via a splitmix64-shuffled
/// permutation table.
pub struct Perlin3 {
    perm: [u8; PERM_SIZE * 2],
}

impl Perlin3 {
    pub fn new(seed: u64) -> Self {
        let mut table: [u8; PERM_SIZE] = [0; PERM_SIZE];
        for (i, slot) in table.iter_mut().enumerate() {
            *slot = i as u8;
        }

        let mut state = seed;
        for i in (1..PERM_SIZE).rev() {
            state = splitmix64(state);
            let j = (state as usize) % (i + 1);
            table.swap(i, j);
        }

        let mut perm = [0u8; PERM_SIZE * 2];
        for (i, slot) in perm.iter_mut().enumerate() {
            *slot = table[i % PERM_SIZE];
        }
        Self { perm }
    }

    /// Returns a value in roughly `[-1, 1]`.
    pub fn sample(&self, x: f64, y: f64, z: f64) -> f64 {
        let xi = x.floor() as i32 as u8;
        let yi = y.floor() as i32 as u8;
        let zi = z.floor() as i32 as u8;

        let xf = x - x.floor();
        let yf = y - y.floor();
        let zf = z - z.floor();

        let u = fade(xf);
        let v = fade(yf);
        let w = fade(zf);

        let p = &self.perm;
        let a = p[xi as usize] as usize + yi as usize;
        let aa = p[a] as usize + zi as usize;
        let ab = p[a + 1] as usize + zi as usize;
        let b = p[xi as usize + 1] as usize + yi as usize;
        let ba = p[b] as usize + zi as usize;
        let bb = p[b + 1] as usize + zi as usize;

        lerp(
            w,
            lerp(
                v,
                lerp(u, grad(p[aa], xf, yf, zf), grad(p[ba], xf - 1.0, yf, zf)),
                lerp(
                    u,
                    grad(p[ab], xf, yf - 1.0, zf),
                    grad(p[bb], xf - 1.0, yf - 1.0, zf),
                ),
            ),
            lerp(
                v,
                lerp(
                    u,
                    grad(p[aa + 1], xf, yf, zf - 1.0),
                    grad(p[ba + 1], xf - 1.0, yf, zf - 1.0),
                ),
                lerp(
                    u,
                    grad(p[ab + 1], xf, yf - 1.0, zf - 1.0),
                    grad(p[bb + 1], xf - 1.0, yf - 1.0, zf - 1.0),
                ),
            ),
        )
    }
}

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

/// Ken Perlin's reference gradient function: picks one of 12 edge directions
/// of a cube (with 4 repeats to fill a 4-bit hash) via the low bits of `hash`.
fn grad(hash: u8, x: f64, y: f64, z: f64) -> f64 {
    let h = hash & 0xF;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 {
        y
    } else if h == 12 || h == 14 {
        x
    } else {
        z
    };
    let u = if h & 1 == 0 { u } else { -u };
    let v = if h & 2 == 0 { v } else { -v };
    u + v
}

/// A small, fast, well-mixed hash — used here to seed the permutation table,
/// and reused (via [`crate::sampling`] and elsewhere) for deterministic
/// pseudo-randomness anywhere a cell just needs "an unpredictable-looking
/// but reproducible number," not a full noise field.
pub(crate) fn splitmix64(x: u64) -> u64 {
    let x = x.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// Fractal Brownian motion: layered octaves of [`Perlin3`], normalized to
/// roughly `[-1, 1]`. Lower frequency / fewer octaves = broad, smooth
/// features (continent shapes); higher frequency / more octaves = fine
/// detail (coastline roughness, terrain texture).
pub struct Fbm3 {
    perlin: Perlin3,
    octaves: u32,
    lacunarity: f64,
    gain: f64,
}

impl Fbm3 {
    pub fn new(seed: u64, octaves: u32, lacunarity: f64, gain: f64) -> Self {
        Self {
            perlin: Perlin3::new(seed),
            octaves,
            lacunarity,
            gain,
        }
    }

    pub fn sample(&self, x: f64, y: f64, z: f64) -> f64 {
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut sum = 0.0;
        let mut norm = 0.0;
        for _ in 0..self.octaves {
            sum += self
                .perlin
                .sample(x * frequency, y * frequency, z * frequency)
                * amplitude;
            norm += amplitude;
            amplitude *= self.gain;
            frequency *= self.lacunarity;
        }
        if norm == 0.0 { 0.0 } else { sum / norm }
    }
}

/// Ridged multifractal: `1 - |noise|` per octave, squared and weighted by
/// the previous octave's value. Produces sharp ridgelines rather than smooth
/// bumps — used for mountain ranges.
pub struct RidgedFbm3 {
    perlin: Perlin3,
    octaves: u32,
    lacunarity: f64,
    gain: f64,
}

impl RidgedFbm3 {
    pub fn new(seed: u64, octaves: u32, lacunarity: f64, gain: f64) -> Self {
        Self {
            perlin: Perlin3::new(seed),
            octaves,
            lacunarity,
            gain,
        }
    }

    /// Returns a value in roughly `[0, 1]`, with sharp ridges near 1.
    pub fn sample(&self, x: f64, y: f64, z: f64) -> f64 {
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut sum = 0.0;
        let mut norm = 0.0;
        let mut weight = 1.0;
        for _ in 0..self.octaves {
            let mut signal = self
                .perlin
                .sample(x * frequency, y * frequency, z * frequency);
            signal = 1.0 - signal.abs();
            signal *= signal;
            signal *= weight;
            weight = (signal * 2.0).clamp(0.0, 1.0);

            sum += signal * amplitude;
            norm += amplitude;
            amplitude *= self.gain;
            frequency *= self.lacunarity;
        }
        if norm == 0.0 { 0.0 } else { sum / norm }
    }
}

/// Maps a map-space `(x, y)` cell to a point on a 3D circle so that sampling
/// a 3D noise field there wraps seamlessly as `x` goes `0..width`. `radius`
/// controls feature density around the loop — a bigger radius packs more
/// noise-space distance (so more/smaller features) into one trip around the
/// cylinder. `y` is scaled to match the circle's noise-space-per-cell
/// spacing, so features are isotropic (a "continent" is as wide as it is
/// tall) rather than stretched along one axis.
pub fn cylinder_point(x: f64, y: f64, width: f64, radius: f64) -> (f64, f64, f64) {
    let theta = (x / width) * TAU;
    let y_scale = TAU * radius / width;
    (theta.cos() * radius, theta.sin() * radius, y * y_scale)
}
