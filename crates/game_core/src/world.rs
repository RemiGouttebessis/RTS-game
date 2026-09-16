use bevy::prelude::*;
use game_worldgen::World;

/// A generated `World` plus the `sea_level` it was generated with — `World`
/// itself doesn't retain that number (it only affects generation, e.g.
/// `hydrology.is_ocean`), but `game_render` needs it too, to know where to
/// sit the visual waterline/zero height on the terrain mesh.
pub struct GeneratedWorldData {
    pub world: World,
    pub sea_level: f32,
}

/// The generated world for the current game — `None` until the background
/// generation task `game_ui::new_game_menu`'s Start button kicks off
/// finishes (see `GameState::Loading`). `game_render::map` starts building
/// terrain chunks from it as soon as it's set.
#[derive(Resource, Default)]
pub struct GeneratedWorld(pub Option<GeneratedWorldData>);

/// How far along `game_render::map`'s chunk-by-chunk terrain mesh building
/// is — `game_ui::loading_menu` reads this to show real "X / Y chunks"
/// progress instead of an indeterminate spinner for that phase (world
/// generation itself, before this has anything to report, just shows a
/// plain "Generating world..." message — its single `game_worldgen::generate`
/// call has no sub-progress to report without restructuring that pipeline).
#[derive(Resource, Default, Clone, Copy)]
pub struct TerrainBuildProgress {
    pub total_chunks: usize,
    pub built_chunks: usize,
}
