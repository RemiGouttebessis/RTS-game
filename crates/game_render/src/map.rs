use std::collections::VecDeque;

use bevy::asset::RenderAssetUsages;
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::VisibilityRange;
use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use game_config::CameraSettings;
use game_core::{GameState, GeneratedWorld, TerrainBuildProgress};
use game_worldgen::image_export::SurfaceClass;
use game_worldgen::{World, image_export, preset};

use crate::camera::{CameraDistance, zoom_fraction};

/// Quads per chunk edge — the world grid is split into `CHUNK_QUADS` ×
/// `CHUNK_QUADS` mesh entities instead of one giant mesh, so Bevy's normal
/// per-entity frustum culling can skip whatever's off-screen. Must be a
/// multiple of `LOD_STRIDE` — see that constant.
const CHUNK_QUADS: usize = 64;

/// How many chunk meshes `build_terrain_incrementally` builds per frame
/// while `GameState::Loading` — spread out instead of all at once so
/// `loading_menu` can actually show "X / Y chunks" advancing across
/// several frames rather than the whole build happening in one blocking
/// step between two frames.
const CHUNKS_PER_FRAME: usize = 8;

/// Every chunk builds a decimated LOD mesh alongside its full-res one —
/// `build_chunk_mesh`'s `stride` samples every `LOD_STRIDE`-th grid cell
/// instead of every one, so the decimated mesh has `1 / LOD_STRIDE²` as many
/// triangles. Must evenly divide `CHUNK_QUADS` (64 / 4 = 16 quads/edge at
/// LOD) so two adjacent chunks' decimated meshes still share exactly the
/// same border vertices — `CHUNK_QUADS` is itself always a multiple of
/// `LOD_STRIDE` for every interior chunk (`col_start`/`row_start` are always
/// multiples of `CHUNK_QUADS`), so this only risks a cosmetic, very minor
/// misalignment at the world's own outer edge where a chunk can be smaller
/// than `CHUNK_QUADS`.
const LOD_STRIDE: usize = 4;

/// Camera-distance band (world units — meters, via `preset::METERS_PER_QUAD`,
/// same space `CameraDistance` lives in) where a chunk crossfades from
/// full-res to its decimated LOD mesh — dithered by
/// `VisibilityRange`/`VisibilityRangePlugin` (already part of Bevy's default
/// camera plugin, no extra wiring needed) rather than an abrupt pop.
const LOD_DISTANCE: f32 = 250.0;
const LOD_MARGIN: f32 = 60.0;

/// Path (relative to the working directory's `assets/`, Bevy's normal
/// convention — see `game_assets::civilizations`' own doc comment for the
/// same pattern) to the terrain texture atlas: a 2×2 grid of CC0 tiles from
/// Kenney's "Retro Textures Fantasy" pack (`assets/textures/terrain/
/// LICENSE.txt` has the exact source/license), one quadrant per
/// `SurfaceClass` — see `atlas_uv`. Pixel-art styled (nearest filtering, no
/// smoothing) rather than the realistic-PBR look `StandardMaterial` usually
/// implies.
const TERRAIN_ATLAS_PATH: &str = "textures/terrain/atlas.png";

/// How many meters one texture tile covers before repeating — independent of
/// `CHUNK_QUADS`/mesh vertex spacing entirely (`Mesh::ATTRIBUTE_UV_0` is
/// computed straight from world-space `(x, z)`, not grid indices), so it
/// stays the same apparent texture density regardless of how coarse the
/// current LOD tier's mesh is.
const TILE_METERS: f32 = 4.0;

/// Keeps `atlas_uv`'s per-tile fractional coordinate away from a cell's
/// shared edge with its neighbor in the atlas — cheap insurance against
/// sampling bleeding across that boundary (e.g. from GPU mip/anisotropic
/// filtering) without needing to bake padding into the atlas image itself.
const ATLAS_UV_INSET: f32 = 0.015;

/// Maps a world-space `(x, z)` position plus its `SurfaceClass` to a UV
/// coordinate inside `TERRAIN_ATLAS_PATH`'s matching quadrant — a *hard*
/// per-vertex class pick (whichever `SurfaceClass` a cell has, its whole
/// triangle samples that one quadrant), not a soft cross-quadrant blend; a
/// blend needs a custom shader sampling multiple layers, real extra
/// machinery for a future pass, not attempted here. The per-vertex
/// `Mesh::ATTRIBUTE_COLOR` from `terrain_paint_color` still tints whatever
/// this samples (Bevy's PBR shader multiplies base color texture × vertex
/// color), so the existing biome/slope/beach coloring keeps mattering.
fn atlas_uv(class: SurfaceClass, x: f32, z: f32) -> [f32; 2] {
    let local_u = (x / TILE_METERS)
        .rem_euclid(1.0)
        .clamp(ATLAS_UV_INSET, 1.0 - ATLAS_UV_INSET);
    let local_v = (z / TILE_METERS)
        .rem_euclid(1.0)
        .clamp(ATLAS_UV_INSET, 1.0 - ATLAS_UV_INSET);
    // Matches atlas.png's actual layout: top-left/top-right/bottom-left/
    // bottom-right, in image (and Bevy UV) coordinates where (0,0) is the
    // top-left texel — see `assets/textures/terrain/LICENSE.txt`.
    let (cell_u, cell_v) = match class {
        SurfaceClass::Land => (0.0, 0.0),
        SurfaceClass::Sand => (0.5, 0.0),
        SurfaceClass::Rock => (0.0, 0.5),
        SurfaceClass::Water => (0.5, 0.5),
    };
    [cell_u + local_u * 0.5, cell_v + local_v * 0.5]
}

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
/// `StandardMaterial` every chunk shares — per-vertex `Mesh::ATTRIBUTE_COLOR`
/// still does the biome/slope/beach tinting, but the material now also
/// carries `TERRAIN_ATLAS_PATH` as its `base_color_texture` (see `atlas_uv`),
/// so there's still nothing chunk-specific to configure on it — one shared
/// `Handle` is just cheaper than allocating hundreds of identical materials.
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
    asset_server: Res<AssetServer>,
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
        // Nearest filtering (crisp, blocky — pixel-art styled, not smoothed)
        // and repeat addressing (so `atlas_uv`'s fractional tiling actually
        // tiles instead of clamping to the atlas's outer edge).
        let atlas: Handle<Image> = asset_server
            .load_builder()
            .with_settings(|settings: &mut ImageLoaderSettings| {
                let mut sampler = ImageSamplerDescriptor::nearest();
                sampler.address_mode_u = ImageAddressMode::Repeat;
                sampler.address_mode_v = ImageAddressMode::Repeat;
                settings.sampler = ImageSampler::Descriptor(sampler);
            })
            .load(TERRAIN_ATLAS_PATH);
        queue.material = Some(materials.add(StandardMaterial {
            base_color_texture: Some(atlas),
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
        let (mesh, heights, has_land, aabb) = build_chunk_mesh(
            world,
            generated.sea_level,
            job.col_start,
            job.row_start,
            job.cols,
            job.rows,
            1,
        );
        let mesh_handle = meshes.add(mesh);
        let mut chunk = commands.spawn((
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
            aabb,
            VisibilityRange {
                start_margin: 0.0..0.0,
                end_margin: LOD_DISTANCE..LOD_DISTANCE + LOD_MARGIN,
                use_aabb: true,
            },
        ));
        chunk.observe(teraform);
        if !has_land {
            // Open ocean: no reason to pay for shadow-map rendering on a
            // chunk that's just a flat, unlit-from-below seabed either way.
            chunk.insert((NotShadowCaster, NotShadowReceiver));
        }

        // The decimated LOD sibling — same world footprint, coarser mesh,
        // no `TerrainChunk`/`teraform` observer (it's a distant stand-in,
        // never the thing actually clicked on), given the *same* `aabb` as
        // its full-res sibling so `VisibilityRange`'s crossfade anchors both
        // tiers at an identical point (see `build_chunk_mesh`'s doc comment).
        let (lod_mesh, _lod_heights, _lod_has_land, _) = build_chunk_mesh(
            world,
            generated.sea_level,
            job.col_start,
            job.row_start,
            job.cols,
            job.rows,
            LOD_STRIDE,
        );
        let lod_mesh_handle = meshes.add(lod_mesh);
        let mut lod_chunk = commands.spawn((
            MapEntity,
            Mesh3d(lod_mesh_handle),
            MeshMaterial3d(material.clone()),
            Transform::default(),
            aabb,
            VisibilityRange {
                start_margin: LOD_DISTANCE..LOD_DISTANCE + LOD_MARGIN,
                end_margin: f32::MAX..f32::MAX,
                use_aabb: true,
            },
        ));
        if !has_land {
            lod_chunk.insert((NotShadowCaster, NotShadowReceiver));
        }

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

/// Which local indices (within `0..len`) to sample for one axis of a
/// decimated mesh: every `stride`-th, plus the final index if `stride`
/// didn't already land on it — so a chunk's decimated mesh always reaches
/// its own far edge (where it has to meet the next chunk's own first
/// sampled vertex; see `LOD_STRIDE`'s doc comment), even for a
/// smaller-than-`CHUNK_QUADS` edge chunk where `stride` doesn't evenly
/// divide `len - 1`. `stride == 1` (the full-res case) always yields
/// `0..len` unchanged — no special-casing needed at the call site.
fn sample_indices(len: usize, stride: usize) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..len).step_by(stride).collect();
    if indices.last() != Some(&(len - 1)) {
        indices.push(len - 1);
    }
    indices
}

/// Builds one chunk's mesh — vertices sampled every `stride`-th cell (see
/// `sample_indices`) from the `cols`×`rows` region starting at
/// (`col_start`, `row_start`) in the world grid, positioned in *world*
/// space (not chunk-local), so the chunk entity's own `Transform` can stay
/// identity and every chunk's vertices line up seamlessly. Height comes from
/// `image_export::visual_height` and color from
/// `image_export::terrain_paint_color` — both shared with `game_worldgen`
/// so the mesh, the slope math that colors it, and anything else that needs
/// "how does this cell actually look" agree by construction. Also reports
/// whether the chunk has any land cell at all (`build_terrain_incrementally`
/// uses this to skip shadows on open-ocean chunks) and the mesh's world-space
/// `Aabb`, so a `stride = 1` and a decimated call for the same region can be
/// given the *same* `Aabb` explicitly — `VisibilityRange`'s own docs call out
/// that smooth LOD crossfading needs every tier positioned identically, and
/// a decimated mesh's own min/max height can differ slightly from the
/// full-res one (skipping a sampled peak), which would otherwise skew its
/// `Aabb` center and the crossfade distance along with it.
fn build_chunk_mesh(
    world: &World,
    sea_level: f32,
    col_start: usize,
    row_start: usize,
    cols: usize,
    rows: usize,
    stride: usize,
) -> (Mesh, Vec<f32>, bool, Aabb) {
    let world_width = world.width;
    let world_height = world.height;

    let col_indices = sample_indices(cols, stride);
    let row_indices = sample_indices(rows, stride);
    let out_cols = col_indices.len();
    let out_rows = row_indices.len();

    let mut positions = Vec::with_capacity(out_cols * out_rows);
    let mut colors = Vec::with_capacity(out_cols * out_rows);
    let mut uvs = Vec::with_capacity(out_cols * out_rows);
    let mut heights = Vec::with_capacity(out_cols * out_rows);
    let mut has_land = false;
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);

    for &local_row in &row_indices {
        let row = row_start + local_row;
        for &local_col in &col_indices {
            let col = col_start + local_col;

            let y = image_export::visual_height(world, sea_level, col, row);
            let x = (col as f32 - world_width as f32 / 2.0) * preset::METERS_PER_QUAD;
            let z = (row as f32 - world_height as f32 / 2.0) * preset::METERS_PER_QUAD;
            positions.push([x, y, z]);
            heights.push(y);
            min = min.min(Vec3::new(x, y, z));
            max = max.max(Vec3::new(x, y, z));

            let (class, [r, g, b]) = image_export::terrain_paint_color(world, sea_level, col, row);
            colors.push([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]);
            uvs.push(atlas_uv(class, x, z));

            let pos = (col as i64, row as i64);
            has_land |= !*world.hydrology.is_ocean.get(pos.0, pos.1)
                && !*world.hydrology.is_lake.get(pos.0, pos.1);
        }
    }

    let aabb = Aabb::from_min_max(min, max);
    let indices = grid_indices(out_cols, out_rows);

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh.compute_smooth_normals();

    (mesh, heights, has_land, aabb)
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
    distance: Res<CameraDistance>,
    settings: Res<CameraSettings>,
    mut terrain: Query<&mut Visibility, (With<TerrainChunk>, Without<WorldMapPlane>)>,
    mut world_map: Query<&mut Visibility, (With<WorldMapPlane>, Without<TerrainChunk>)>,
) {
    // Not `Transform.translation.y` — with the camera's pitch now varying
    // (see `camera::apply_camera_transform`), that's `distance *
    // pitch.sin()`, not distance itself, so it shrinks as pitch flattens
    // and would trigger this well past where it should. `CameraDistance`
    // and `zoom_fraction` are the same ones pitch/FOV are actually derived
    // from, so this crossover point stays consistent with how tilted/flat
    // the camera already looks at that zoom level.
    let show_world_map = zoom_fraction(distance.0, &settings) >= WORLD_MAP_ZOOM_FRACTION;

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
