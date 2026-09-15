/// Tunable knobs for [`crate::generate`]. Named presets below are just
/// different values for these — add a preset by adding a `const`, not by
/// branching generation logic on a preset enum.
#[derive(Clone, Copy, Debug)]
pub struct Preset {
    pub name: &'static str,

    /// Radius (in noise-space) of the circle the base continent octave is
    /// sampled on — see `noise::cylinder_point`. A single Perlin octave has
    /// a wavelength of ~1 noise-space unit, and the circle's circumference
    /// is `2π * radius`, so this produces roughly `2π * radius` landmasses
    /// around the loop: ~0.2 for one supercontinent, ~0.6-0.7 for a
    /// handful of continents, ~2.5+ for many small islands. Easy to
    /// mis-tune by an order of magnitude if you forget the `2π` — that's
    /// exactly what happened the first time these were picked.
    pub continent_radius: f64,
    pub continent_octaves: u32,

    /// Fraction of the map below this elevation is ocean.
    pub sea_level: f32,

    /// How much ridged mountain noise gets added to elevation, and how
    /// tightly it's masked to specific "mountain belt" regions (higher mask
    /// frequency = more, narrower belts).
    pub mountain_strength: f32,
    pub mountain_belt_radius: f64,

    /// Flow accumulation threshold (as a fraction of the max observed on
    /// this map) above which a land cell becomes a river.
    pub river_threshold: f32,

    /// Overall wetness bias applied after the moisture noise layer;
    /// positive pushes toward jungle/forest, negative toward desert/steppe.
    pub moisture_bias: f32,

    /// Overall warmth bias applied after the latitude/altitude temperature
    /// model; positive pushes toward tropical, negative toward tundra/snow.
    pub temperature_bias: f32,
}

/// Converts a user-facing "roughly this many landmasses" count into the
/// radius [`crate::elevation::generate`] actually samples on — see
/// `continent_radius`'s doc comment above for the `2π` relationship.
pub fn radius_for_count(count: f64) -> f64 {
    count / core::f64::consts::TAU
}

/// Inverse of [`radius_for_count`] — recovers the "roughly this many
/// landmasses" figure a preset's own `continent_radius` implies, so a UI can
/// show/seed a count control from whichever preset is selected instead of
/// leaving it at a stale, unrelated value.
pub fn count_for_radius(radius: f64) -> f64 {
    radius * core::f64::consts::TAU
}

pub const CONTINENTS: Preset = Preset {
    name: "continents",
    continent_radius: 0.65,
    continent_octaves: 4,
    sea_level: 0.5,
    mountain_strength: 0.3,
    mountain_belt_radius: 0.4,
    river_threshold: 0.02,
    moisture_bias: 0.0,
    temperature_bias: 0.0,
};

pub const PANGAEA: Preset = Preset {
    name: "pangaea",
    continent_radius: 0.2,
    continent_octaves: 4,
    sea_level: 0.45,
    mountain_strength: 0.32,
    mountain_belt_radius: 0.32,
    river_threshold: 0.02,
    moisture_bias: 0.0,
    temperature_bias: 0.0,
};

pub const ARCHIPELAGO: Preset = Preset {
    name: "archipelago",
    continent_radius: 2.5,
    continent_octaves: 5,
    sea_level: 0.56,
    mountain_strength: 0.22,
    mountain_belt_radius: 0.48,
    river_threshold: 0.03,
    moisture_bias: 0.15,
    temperature_bias: 0.0,
};

pub const HIGHLANDS: Preset = Preset {
    name: "highlands",
    continent_radius: 0.65,
    continent_octaves: 4,
    sea_level: 0.48,
    mountain_strength: 0.5,
    mountain_belt_radius: 0.64,
    river_threshold: 0.015,
    moisture_bias: -0.05,
    temperature_bias: 0.0,
};

pub const ALL: &[Preset] = &[CONTINENTS, PANGAEA, ARCHIPELAGO, HIGHLANDS];

pub fn by_name(name: &str) -> Option<Preset> {
    ALL.iter().copied().find(|preset| preset.name == name)
}
