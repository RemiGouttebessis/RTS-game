/// Real-world size of one grid cell — the fact that turns a `width`×`height`
/// grid into an actual map size (`width * METERS_PER_QUAD` meters east-west).
/// Lives here rather than in `game_render` because it's a property of the
/// *world*, not of how it's rendered: `game_ui::worldgen_menu`'s Size
/// stepper uses it to show the chosen grid width in km, and
/// `game_render::map` uses it as the terrain mesh's horizontal quad
/// spacing — both need the same number.
pub const METERS_PER_QUAD: f32 = 2.0;

/// Vertical exaggeration for the visual (mesh) height of land — elevation is
/// normalized `0..1.2`, which reads as nearly flat at `METERS_PER_QUAD`'s 2m
/// horizontal scale without one. Lives here (not just in `game_render`) so
/// `image_export::terrain_paint_color`'s slope calculation and
/// `game_render::map`'s actual mesh use the exact same number — computing a
/// steepness that doesn't match what gets drawn would defeat the point of
/// slope-based rock blending.
pub const HEIGHT_SCALE: f32 = 60.0;

/// Extra height multiplier for cells `elevation::ElevationMaps::mountain_mask`
/// already flags as rugged (`0..1`, independent of raw elevation) — mountains
/// end up dramatically taller than a flat, uniform `HEIGHT_SCALE` would give
/// them, while ordinary rolling land stays modest. `visual_height` is the one
/// function that applies this; nothing else should reimplement the formula.
pub const MOUNTAIN_HEIGHT_BOOST: f32 = 1.8;

/// Flat-clipped visual depths for ocean cells — a real ocean floor's exact
/// depth is unbounded/arbitrary in this model, so instead of following raw
/// (often barely-below-`sea_level`, noisy) elevation, these give a clean
/// two-step "continental shelf." Lakes are *not* in this list on purpose —
/// see `HydrologyMaps::filled`, which gives each lake its own correct
/// natural water level instead of a fixed clip.
pub const COAST_FLOOR_DEPTH: f32 = 4.0;
pub const OCEAN_FLOOR_DEPTH: f32 = 16.0;

/// How far below a lake basin's true (`HydrologyMaps::filled`) pour-point
/// elevation the water surface sits — just enough that the water plane
/// doesn't z-fight with the lakebed itself, not a real depth.
pub const LAKE_SURFACE_OFFSET: f32 = 0.05;

/// Tunable knobs for [`crate::generate`]. Named presets below are just
/// different values for these — add a preset by adding a `const`, not by
/// branching generation logic on a preset enum.
#[derive(Clone, Copy, Debug)]
pub struct Preset {
    pub name: &'static str,

    /// Target number of landmasses. Continents come from layered noise (see
    /// `elevation::generate`), which only produces *roughly* this many
    /// blobs on its own; `elevation::correct_continent_count` merges or
    /// splits landmasses after the fact to close the gap. That two-step
    /// approach (organic noise shape, then a count correction pass) reads
    /// far more natural than trying to force the exact count out of the
    /// noise/distance-field math directly — that was tried and looked
    /// conspicuously artificial (see this field's git history).
    pub continent_count: u32,
    /// Octave count for the main continent-shape noise layer. Higher = more
    /// detail folded into the coastline at generation time (before the
    /// count-correction pass runs).
    pub continent_octaves: u32,
    /// Whether `elevation::correct_continent_count` runs at all — off skips
    /// straight to whatever `continent_count`'s noise landed on, no
    /// merge/split post-processing. `worldgen_menu`'s "Split/Merge" toggle
    /// controls this directly (see `WorldGenSettings::correct_continents`);
    /// every `const` preset below defaults it on.
    pub correct_continents: bool,

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

pub const CONTINENTS: Preset = Preset {
    name: "continents",
    continent_count: 4,
    continent_octaves: 4,
    correct_continents: true,
    sea_level: 0.5,
    mountain_strength: 0.42,
    mountain_belt_radius: 0.28,
    river_threshold: 0.02,
    moisture_bias: 0.0,
    temperature_bias: 0.0,
};

pub const PANGAEA: Preset = Preset {
    name: "pangaea",
    continent_count: 1,
    continent_octaves: 4,
    correct_continents: true,
    sea_level: 0.45,
    mountain_strength: 0.45,
    mountain_belt_radius: 0.22,
    river_threshold: 0.02,
    moisture_bias: 0.0,
    temperature_bias: 0.0,
};

pub const ARCHIPELAGO: Preset = Preset {
    name: "archipelago",
    continent_count: 16,
    continent_octaves: 5,
    correct_continents: true,
    sea_level: 0.56,
    mountain_strength: 0.32,
    mountain_belt_radius: 0.34,
    river_threshold: 0.03,
    moisture_bias: 0.15,
    temperature_bias: 0.0,
};

pub const HIGHLANDS: Preset = Preset {
    name: "highlands",
    continent_count: 4,
    continent_octaves: 4,
    correct_continents: true,
    sea_level: 0.48,
    mountain_strength: 0.65,
    mountain_belt_radius: 0.45,
    river_threshold: 0.015,
    moisture_bias: -0.05,
    temperature_bias: 0.0,
};

pub const ALL: &[Preset] = &[CONTINENTS, PANGAEA, ARCHIPELAGO, HIGHLANDS];

pub fn by_name(name: &str) -> Option<Preset> {
    ALL.iter().copied().find(|preset| preset.name == name)
}
