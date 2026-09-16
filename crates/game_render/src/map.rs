use std::collections::VecDeque;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use game_config::CameraSettings;
use game_core::{GameState, GeneratedWorld, TerrainBuildProgress};
use game_worldgen::biome::Terrain;
use game_worldgen::{World, image_export, preset};

/// Vertical exaggeration — elevation is normalized `0..1.2`, which reads as
/// nearly flat without a strong multiplier: at `preset::METERS_PER_QUAD`'s
/// 2m/quad horizontal scale, even a full elevation swing is a barely-there
/// slope unless height is stretched out disproportionately, the same
/// stylized exaggeration every Civ-style/RTS terrain view uses for
/// readability from a top-down camera. Horizontal scale is
/// `preset::METERS_PER_QUAD` instead — a real-world unit, unlike this.
const HEIGHT_SCALE: f32 = 45.0;

/// Ocean/lake floors are flat-clipped to one of these fixed depths rather
/// than following the raw (often noisy, sometimes barely-below-sea-level)
/// elevation value underneath them — a jagged, wildly-varying seabed read
/// as broken, not like water. Coast (shallow water right at the shoreline)
/// gets a shallower clip than open ocean, so there's at least a two-step
/// "continental shelf" rather than every water cell at one uniform depth.
/// Scaled to sit comfortably below `HEIGHT_SCALE`'s land relief.
const COAST_FLOOR_DEPTH: f32 = 4.0;
const OCEAN_FLOOR_DEPTH: f32 = 16.0;
const LAKE_FLOOR_DEPTH: f32 = 5.0;

/// Quads per chunk edge — the world grid is split into `CHUNK_QUADS` ×
/// `CHUNK_QUADS` mesh entities instead of one giant mesh, so Bevy's normal
/// per-entity frustum culling can skip whatever's off-screen. This is the
/// *only* optimization here — there is no distance-based LOD (decimated
/// meshes swapped in when a chunk is far away) yet, so pushing `Size` (see
/// `worldgen_menu`) very high will still get slow to fly around, just not
/// *as* slow as one unculled mesh would. Real LOD is the natural next step.
const CHUNK_QUADS: usize = 64;

/// How many chunk meshes `build_terrain_incrementally` builds per frame
/// while `GameState::Loading` — spread out instead of all at once so
/// `loading_menu` can actually show "X / Y chunks" advancing across
/// several frames rather than the whole build happening in one blocking
/// step between two frames.
const CHUNKS_PER_FRAME: usize = 8;

/// Raise/lower amount per teraform click, in the same world-height units as
/// `HEIGHT_SCALE` produces — a single early, fixed-radius/fixed-strength
/// brush, not a tunable tool yet.
const TERAFORM_STRENGTH: f32 = 1.5;
const TERAFORM_RADIUS: f32 = 2.5;

/// Above this camera height, the detailed terrain mesh hides and the flat
/// `WorldMapPlane` shows instead — a fraction of the way from `min_height`
/// to `max_height` rather than a fixed world-unit height, so it stays a
/// sensible "zoomed most of the way out" threshold regardless of what those
/// are configured to.
const WORLD_MAP_ZOOM_FRACTION: f32 = 0.75;

/// Marks entities spawned by this plugin, so they can be cleaned up on
/// `OnExit(GameState::InGame)` (e.g. "Quit to Main Menu") without leaving
/// stale terrain/light entities around for the next game to duplicate.
#[derive(Component)]
struct MapEntity;

/// One `CHUNK_QUADS`×`CHUNK_QUADS` (edge chunks may be smaller) slice of the
/// heightmap terrain. Keeps its own CPU-side height grid, in this chunk's
/// *local* vertex space, so `teraform` can look up/mutate a cell's height
/// without reading it back out of the GPU-bound `Mesh` asset.
/// `world_width`/`world_height` and `col_start`/`row_start` are what let it
/// translate a world-space click position back into this chunk's local grid.
#[derive(Component)]
struct TerrainChunk {
    mesh: Handle<Mesh>,
    world_width: usize,
    world_height: usize,
    col_start: usize,
    row_start: usize,
    /// Vertex counts (not quad counts) for this chunk.
    cols: usize,
    rows: usize,
    /// Row-major, local to this chunk — `heights[local_row * cols + local_col]`.
    heights: Vec<f32>,
}

/// The flat, textured overview plane shown instead of the detailed terrain
/// once the camera zooms out past `WORLD_MAP_ZOOM_FRACTION` — see
/// `toggle_world_map`. Its texture is `image_export::biome_map`, the same
/// rendering `worldgen_menu`'s preview uses — that image is *only* ever
/// used here, as a flat top-down map, never sampled for the 3D terrain's
/// own coloring (see `build_chunk_mesh`, which reads biome data directly).
/// Early version of "zoom out far enough to see the world map": one image,
/// swapped by visibility, not an actual continuous LOD transition.
#[derive(Component)]
struct WorldMapPlane;

struct ChunkJob {
    col_start: usize,
    row_start: usize,
    cols: usize,
    rows: usize,
}

/// Drives `build_terrain_incrementally`: the queue of not-yet-built chunks
/// for the world currently in `GeneratedWorld`, plus the one
/// `StandardMaterial` every chunk shares (color comes from per-vertex
/// `Mesh::ATTRIBUTE_COLOR`, not the material, so there's nothing
/// chunk-specific to configure — one shared `Handle` is just cheaper than
/// allocating hundreds of identical materials).
#[derive(Resource, Default)]
struct ChunkBuildQueue {
    material: Option<Handle<StandardMaterial>>,
    jobs: VecDeque<ChunkJob>,
    /// `false` until the job list has been populated from `GeneratedWorld`
    /// (which might not be ready the first several frames of `Loading` —
    /// world generation is also asynchronous, see `new_game_menu`).
    started: bool,
}

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkBuildQueue>()
            .add_systems(
                OnEnter(GameState::Loading),
                (reset_terrain_build, spawn_light),
            )
            .add_systems(
                Update,
                build_terrain_incrementally.run_if(in_state(GameState::Loading)),
            )
            .add_systems(Update, toggle_world_map.run_if(in_state(GameState::InGame)))
            .add_systems(OnExit(GameState::InGame), despawn_map);
    }
}

fn reset_terrain_build(
    mut queue: ResMut<ChunkBuildQueue>,
    mut progress: ResMut<TerrainBuildProgress>,
) {
    *queue = ChunkBuildQueue::default();
    *progress = TerrainBuildProgress::default();
}

/// Builds up to `CHUNKS_PER_FRAME` terrain chunks per frame while
/// `GameState::Loading`, so `loading_menu` can show real progress instead
/// of the whole (potentially multi-second, at a large `Size`) mesh build
/// happening as one frame-blocking step. Once every chunk is built, spawns
/// the `WorldMapPlane` overview and hands off to `GameState::InGame` —
/// this system, not `new_game_menu`'s Start button, is what actually makes
/// that transition, since it's the one that knows when there's something
/// ready to show.
#[allow(clippy::too_many_arguments)]
fn build_terrain_incrementally(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    generated: Res<GeneratedWorld>,
    mut queue: ResMut<ChunkBuildQueue>,
    mut progress: ResMut<TerrainBuildProgress>,
    mut next_game_state: ResMut<NextState<GameState>>,
) {
    let Some(generated) = generated.0.as_ref() else {
        return; // world generation (new_game_menu/loading_menu) isn't done yet
    };
    let world = &generated.world;

    if !queue.started {
        queue.started = true;
        queue.material = Some(materials.add(StandardMaterial {
            perceptual_roughness: 0.95,
            ..default()
        }));

        let chunk_cols = world.width.div_ceil(CHUNK_QUADS);
        let chunk_rows = world.height.div_ceil(CHUNK_QUADS);
        for chunk_y in 0..chunk_rows {
            for chunk_x in 0..chunk_cols {
                let col_start = chunk_x * CHUNK_QUADS;
                let row_start = chunk_y * CHUNK_QUADS;
                let cols = (col_start + CHUNK_QUADS + 1).min(world.width) - col_start;
                let rows = (row_start + CHUNK_QUADS + 1).min(world.height) - row_start;
                if cols < 2 || rows < 2 {
                    continue;
                }
                queue.jobs.push_back(ChunkJob {
                    col_start,
                    row_start,
                    cols,
                    rows,
                });
            }
        }
        progress.total_chunks = queue.jobs.len();
        progress.built_chunks = 0;
    }

    let material = queue.material.clone().expect("set when queue.started");
    for _ in 0..CHUNKS_PER_FRAME {
        let Some(job) = queue.jobs.pop_front() else {
            break;
        };
        let (mesh, heights) = build_chunk_mesh(
            world,
            generated.sea_level,
            job.col_start,
            job.row_start,
            job.cols,
            job.rows,
        );
        let mesh_handle = meshes.add(mesh);
        commands
            .spawn((
                MapEntity,
                TerrainChunk {
                    mesh: mesh_handle.clone(),
                    world_width: world.width,
                    world_height: world.height,
                    col_start: job.col_start,
                    row_start: job.row_start,
                    cols: job.cols,
                    rows: job.rows,
                    heights,
                },
                Mesh3d(mesh_handle),
                MeshMaterial3d(material.clone()),
                Transform::default(),
            ))
            .observe(teraform);
        progress.built_chunks += 1;
    }

    if queue.started && queue.jobs.is_empty() {
        let map_image = build_world_map_image(world);
        let map_handle = images.add(map_image);
        commands.spawn((
            MapEntity,
            WorldMapPlane,
            Visibility::Hidden,
            Mesh3d(meshes.add(Plane3d::default().mesh().size(
                world.width as f32 * preset::METERS_PER_QUAD,
                world.height as f32 * preset::METERS_PER_QUAD,
            ))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color_texture: Some(map_handle),
                unlit: true,
                ..default()
            })),
            Transform::from_xyz(0.0, -0.1, 0.0),
        ));
        next_game_state.set(GameState::InGame);
    }
}

/// Builds one chunk's mesh — `cols`×`rows` vertices starting at
/// (`col_start`, `row_start`) in the world grid, positioned in *world*
/// space (not chunk-local), so the chunk entity's own `Transform` can stay
/// identity and every chunk's vertices line up seamlessly. Vertex color is
/// `image_export::terrain_color` read directly from `world.biome.terrain`
/// — flat, no elevation-band darkening or river/feature blending, unlike
/// `biome_map`'s preview rendering; that extra shading is tuned for a
/// static top-down image, not real mesh lighting, and its baked-in ocean
/// depth tint in particular fights this function's own flat-clipped ocean
/// floor (see `cell_height`).
fn build_chunk_mesh(
    world: &World,
    sea_level: f32,
    col_start: usize,
    row_start: usize,
    cols: usize,
    rows: usize,
) -> (Mesh, Vec<f32>) {
    let world_width = world.width;
    let world_height = world.height;

    let mut positions = Vec::with_capacity(cols * rows);
    let mut colors = Vec::with_capacity(cols * rows);
    let mut heights = Vec::with_capacity(cols * rows);

    for local_row in 0..rows {
        let row = row_start + local_row;
        for local_col in 0..cols {
            let col = col_start + local_col;

            let y = cell_height(world, sea_level, col, row);
            let x = (col as f32 - world_width as f32 / 2.0) * preset::METERS_PER_QUAD;
            let z = (row as f32 - world_height as f32 / 2.0) * preset::METERS_PER_QUAD;
            positions.push([x, y, z]);
            heights.push(y);

            let terrain = *world.biome.terrain.get(col as i64, row as i64);
            let [r, g, b] = image_export::terrain_color(terrain);
            colors.push([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]);
        }
    }

    let indices = grid_indices(cols, rows);

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh.compute_smooth_normals();

    (mesh, heights)
}

/// The mesh height for one world cell: a flat-clipped depth for water (see
/// `COAST_FLOOR_DEPTH`/`OCEAN_FLOOR_DEPTH`/`LAKE_FLOOR_DEPTH`), or actual
/// elevation, scaled, for land.
fn cell_height(world: &World, sea_level: f32, col: usize, row: usize) -> f32 {
    let pos = (col as i64, row as i64);

    if *world.hydrology.is_lake.get(pos.0, pos.1) {
        return -LAKE_FLOOR_DEPTH;
    }
    if *world.hydrology.is_ocean.get(pos.0, pos.1) {
        let terrain = *world.biome.terrain.get(pos.0, pos.1);
        return if terrain == Terrain::Coast {
            -COAST_FLOOR_DEPTH
        } else {
            -OCEAN_FLOOR_DEPTH
        };
    }

    let elevation = *world.elevation.elevation.get(pos.0, pos.1);
    (elevation - sea_level) * HEIGHT_SCALE
}

/// Two triangles per grid quad, wound so the face normal points toward `+Y`
/// (up, toward a camera looking down at the terrain from above).
fn grid_indices(cols: usize, rows: usize) -> Vec<u32> {
    let mut indices = Vec::with_capacity((cols - 1) * (rows - 1) * 6);
    let at = |row: usize, col: usize| (row * cols + col) as u32;

    for row in 0..rows - 1 {
        for col in 0..cols - 1 {
            let a = at(row, col);
            let b = at(row, col + 1);
            let c = at(row + 1, col);
            let d = at(row + 1, col + 1);
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }

    indices
}

fn build_world_map_image(world: &World) -> Image {
    let rgb = image_export::biome_map(
        world.width,
        world.height,
        &world.elevation,
        &world.hydrology,
        &world.biome,
    );
    let mut data = Vec::with_capacity(rgb.width() as usize * rgb.height() as usize * 4);
    for pixel in rgb.pixels() {
        data.extend_from_slice(&pixel.0);
        data.push(255);
    }
    Image::new(
        Extent3d {
            width: rgb.width(),
            height: rgb.height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// A single early, fixed-radius/fixed-strength raise (left click) or lower
/// (right click) brush — proves the terrain is actually mutable at runtime
/// (mesh + CPU height cache both updated, normals recomputed), not a tuned
/// terraforming tool. `MeshPickingPlugin` (added once in `main.rs`) is what
/// makes chunk entities clickable at all and gives `hit.position` in world
/// space; since every chunk's own `Transform` is identity, that's usable
/// directly as world/mesh space either way. A brush that reaches past the
/// clicked chunk's own edge is clamped to that chunk only — it doesn't
/// paint into the neighbor, a known seam limitation of chunking without
/// cross-chunk brush support.
fn teraform(
    click: On<Pointer<Click>>,
    mut chunks: Query<&mut TerrainChunk>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Ok(mut chunk) = chunks.get_mut(click.entity) else {
        return;
    };
    let Some(hit_position) = click.event.hit.position else {
        return;
    };
    let delta = match click.event.button {
        PointerButton::Primary => TERAFORM_STRENGTH,
        PointerButton::Secondary => -TERAFORM_STRENGTH,
        _ => return,
    };

    let TerrainChunk {
        mesh,
        world_width,
        world_height,
        col_start,
        row_start,
        cols,
        rows,
        heights,
    } = &mut *chunk;
    let Some(mut mesh) = meshes.get_mut(&*mesh) else {
        return;
    };

    apply_teraform_brush(
        *world_width,
        *world_height,
        *col_start,
        *row_start,
        *cols,
        *rows,
        heights,
        hit_position,
        delta,
    );
    write_heights_to_mesh(&mut mesh, heights);
    mesh.compute_smooth_normals();
}

/// Raises/lowers every height-grid cell within `TERAFORM_RADIUS` of the
/// click, falling off linearly to the edge of the brush so it reads as a
/// mound/pit rather than a flat-topped cylinder.
#[allow(clippy::too_many_arguments)]
fn apply_teraform_brush(
    world_width: usize,
    world_height: usize,
    col_start: usize,
    row_start: usize,
    cols: usize,
    rows: usize,
    heights: &mut [f32],
    hit_position: Vec3,
    delta: f32,
) {
    let global_col = hit_position.x / preset::METERS_PER_QUAD + world_width as f32 / 2.0;
    let global_row = hit_position.z / preset::METERS_PER_QUAD + world_height as f32 / 2.0;
    let center_col = global_col - col_start as f32;
    let center_row = global_row - row_start as f32;

    let min_col = (center_col - TERAFORM_RADIUS).floor().max(0.0) as usize;
    let max_col = (center_col + TERAFORM_RADIUS)
        .ceil()
        .max(0.0)
        .min(cols as f32 - 1.0) as usize;
    let min_row = (center_row - TERAFORM_RADIUS).floor().max(0.0) as usize;
    let max_row = (center_row + TERAFORM_RADIUS)
        .ceil()
        .max(0.0)
        .min(rows as f32 - 1.0) as usize;

    for row in min_row..=max_row {
        for col in min_col..=max_col {
            let dist =
                ((col as f32 - center_col).powi(2) + (row as f32 - center_row).powi(2)).sqrt();
            if dist > TERAFORM_RADIUS {
                continue;
            }
            let falloff = 1.0 - dist / TERAFORM_RADIUS;
            heights[row * cols + col] += delta * falloff;
        }
    }
}

fn write_heights_to_mesh(mesh: &mut Mesh, heights: &[f32]) {
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    else {
        return;
    };
    for (position, &y) in positions.iter_mut().zip(heights) {
        position[1] = y;
    }
}

fn spawn_light(mut commands: Commands) {
    commands.spawn((
        MapEntity,
        DirectionalLight {
            illuminance: 10_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(20.0, 40.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Hides the detailed terrain chunks and shows `WorldMapPlane` (or vice
/// versa) based on how far the camera has zoomed out — see
/// `WORLD_MAP_ZOOM_FRACTION`.
fn toggle_world_map(
    cameras: Query<&Transform, With<Camera3d>>,
    settings: Res<CameraSettings>,
    mut terrain: Query<&mut Visibility, (With<TerrainChunk>, Without<WorldMapPlane>)>,
    mut world_map: Query<&mut Visibility, (With<WorldMapPlane>, Without<TerrainChunk>)>,
) {
    let Ok(camera) = cameras.single() else {
        return;
    };
    let threshold =
        settings.min_height + (settings.max_height - settings.min_height) * WORLD_MAP_ZOOM_FRACTION;
    let show_world_map = camera.translation.y >= threshold;

    for mut visibility in &mut terrain {
        *visibility = if show_world_map {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    for mut visibility in &mut world_map {
        *visibility = if show_world_map {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn despawn_map(mut commands: Commands, entities: Query<Entity, With<MapEntity>>) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
}
