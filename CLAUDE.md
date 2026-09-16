# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project status

`rts-game` is a Rust Cargo workspace (edition 2024) using Bevy 0.19.1, following the crate layout in
`rts-bevy-architecture-notes.md`. The `game` crate (`crates/game`, binary name `rts-game`) is a thin
binary: it loads persisted settings via `game_config::load()` *before* the `App` is built (the graphics
backend has to be known for `RenderPlugin`), builds `DefaultPlugins`, adds `MeshPickingPlugin` (compiled
in by Bevy's default features but not added by `DefaultPlugins`) and a debug-only `DevDiagnosticsPlugin`
(in-game perf overlay, hidden by default, `F3` to toggle: FPS, frame time, sim tick rate, entity count —
see `crates/game/src/diagnostics/overlay.rs`), then wires in the library crates:

- `game_config` — persisted user settings (graphics backend, camera speeds, keybinds), serialized as RON
  to the OS config directory via the `directories` crate. See "Configuration" in `README.md`.
- `game_core` — sim ECS data (components/resources/events) plus state types: `GameState` (`MainMenu`
  default → `Loading` → `InGame`) and `PauseState` (`Running`/`Paused`, a `SubStates` of
  `GameState::InGame` — kept separate from `GameState` specifically so pause/resume doesn't re-trigger
  `OnEnter(GameState::InGame)` world-spawning). `Loading` sits between pressing Start and actually
  playing, while world generation and terrain mesh building both happen — see `game_ui::loading_menu`
  and `game_render::map`'s entries. Gameplay-wise currently just the `Unit` marker component, plus two
  resources shared between those two crates: `GeneratedWorld` (`Option<GeneratedWorldData>` — a
  `game_worldgen::World` plus the `sea_level` it was generated with, since `World` itself doesn't
  retain that number) and `TerrainBuildProgress` (`total_chunks`/`built_chunks`, for the loading
  screen's progress text). Depending on `game_worldgen` for `GeneratedWorld` doesn't compromise "zero
  rendering deps" (the reason this crate can eventually run headless) since `game_worldgen` has no
  bevy dependency of its own — it's sim/world data, not a renderer.
- `game_sim` — deterministic simulation on `FixedUpdate`: `MovementPlugin`, `CombatPlugin`,
  `PathfindingPlugin`, `OrdersPlugin`, ordered via the `SimSet` system set. All currently empty stubs — no
  sim logic has been implemented yet.
- `game_render` — presentation: `CameraPlugin` (pan/zoom 3D camera, driven by `game_input` actions,
  spawns at `Startup` but pan/zoom only run `in_state(GameState::InGame)`; zoom is smoothed — scroll
  input updates a `CameraZoomTarget` resource instantly, `apply_smooth_zoom` eases the camera's actual
  height toward it every frame via exponential smoothing, `1 - exp(-rate * dt)`, so the glide's speed
  doesn't depend on frame rate the way a plain `lerp(.., dt * rate)` would; **pan speed scales with
  that height too** — `pan_speed_scale` multiplies `CameraSettings::pan_speed` by
  `sqrt(camera_height / min_height)`, since covering the same *screen* distance at a high, zoomed-out
  camera means covering a lot more world than at `min_height`, and a flat pan speed reads as crawling
  once zoomed out; `sqrt` rather than linear so that scaling (up to `max_height / min_height`, 120x by
  default) doesn't make max-height panning uncontrollably fast. `CameraSettings::max_height` defaults
  to 600 (was 60) specifically to be able to zoom out over a real, `preset::METERS_PER_QUAD`-scaled
  world — **if you change `CameraSettings`' defaults, check `%APPDATA%/rts-game/config/settings.ron`
  (or platform equivalent) for a stale persisted copy that will silently override them**, the way this
  one already did once), `MapPlugin`/`UnitsPlugin`. `MapPlugin` builds **real, chunked heightmap
  terrain** from `game_core::GeneratedWorld`, but not on `OnEnter(GameState::InGame)` the way the
  placeholder ground plane used to — chunk meshes are built a handful at a time
  (`CHUNKS_PER_FRAME`, 8) every frame while `GameState::Loading`
  (`build_terrain_incrementally`/`ChunkBuildQueue`), specifically so `loading_menu` has real "X / Y
  chunks" progress to show instead of the whole build happening as one frame-blocking step; once the
  queue empties, this system is what sets `GameState::InGame` (not `new_game_menu`'s Start button,
  which only *starts* generation and moves to `Loading`). **Size is real now**: `worldgen_menu`'s Size
  stepper (still the `resolution` field internally) is the world's actual quad grid width at
  `preset::METERS_PER_QUAD` (2m) per quad — but deliberately **not** what the *live preview* image
  renders at (see `game_ui`'s entry, `PREVIEW_RESOLUTION`); it only matters at Start, when
  `new_game_menu` regenerates at exactly `WorldGenSettings::dimensions()` so what the preview *showed*
  (at its own fixed resolution) is still the same world/seed/preset, just built at its real size.
  **The grid is split into `CHUNK_QUADS`×`CHUNK_QUADS` (64) mesh entities** (`TerrainChunk`, adjacent
  chunks sharing their border vertices so there's no seam), not one giant mesh — Bevy's ordinary
  per-entity frustum culling then skips whatever's off-screen, which is the *only* optimization in
  place. There's no distance-based LOD (mesh decimation swapped in for far chunks) — pushing Size very
  high still gets slow to fly around, just less catastrophically than an unculled single mesh; real
  LOD is flagged as the next step, not attempted here. **Vertex color is flat per-`Terrain`**
  (`image_export::terrain_color`, now `pub`, read directly from `world.biome.terrain`) — *not* sampled
  from `image_export::biome_map`'s rendered PNG the way an earlier version did: that image bakes in
  elevation-band darkening, feature/river blending and ocean-depth tint tuned for a static top-down
  preview, none of which reads well as sparse per-vertex mesh color under real lighting. `biome_map`
  is still used, just only for the `WorldMapPlane` overview texture (see below) — a literal flat
  preview image, never the 3D mesh's own coloring. **Ocean/lake floors are flat-clipped**, not left to
  follow the raw (often barely-below-sea-level, noisy) elevation underneath them: `cell_height`
  returns one of three fixed depths (`OCEAN_FLOOR_DEPTH`/`COAST_FLOOR_DEPTH`/`LAKE_FLOOR_DEPTH`) for
  water cells instead of `(elevation - sea_level) * HEIGHT_SCALE`, land's formula — a two-step
  "continental shelf" (Coast shallower than open Ocean) rather than a jagged seabed.
  **`HEIGHT_SCALE` (45, was 6) is a deliberately dramatic vertical exaggeration**, not a realistic
  one — at `preset::METERS_PER_QUAD`'s 2m/quad horizontal scale, even a full elevation swing reads as
  barely-there without one, the same stylized exaggeration every Civ-style/RTS terrain view relies on
  for readability from a top-down camera; the flat water depths above were scaled up to match, so they
  still sit comfortably below land's relief instead of poking through it. `Mesh::compute_smooth_normals()`
  handles shading. **Terrain is teraformable**: each chunk carries a `TerrainChunk` component (mesh
  handle, its `col_start`/`row_start` offset into the world grid, and a CPU-side local `heights`
  cache) and an `.observe(teraform)` — left/right click (via `MeshPickingPlugin`, added once in
  `main.rs`; its `hit.position` is usable directly as world/mesh space since every chunk's own
  `Transform` is identity) raises/lowers every cell within a fixed radius, falling off linearly to the
  brush edge, then rewrites `Mesh::ATTRIBUTE_POSITION` and recomputes normals — one
  fixed-radius/fixed-strength brush, not a tuned tool, and it doesn't reach across a chunk boundary (a
  brush near the edge of the chunk that was actually clicked just clips there — a known seam
  limitation of per-chunk mutation without cross-chunk brush support). **Zoom out far enough and the
  detailed chunks swap for a `WorldMapPlane`** — a flat plane textured with `biome_map`, spawned once
  chunk building finishes, toggled by `Visibility` in `toggle_world_map` once the camera height
  crosses `WORLD_MAP_ZOOM_FRACTION` (75%) of the way from `CameraSettings::min_height` to
  `max_height`; one image swapped by visibility, not a continuous LOD transition. Fog of war, and
  dressing the terrain with trees/rocks/rivers (or shaders), are intentionally not part of any of this
  yet. `light`/unit-cube placeholders spawn on `OnEnter(GameState::Loading)`/`OnEnter(GameState::InGame)`
  respectively **and despawn on `OnExit(GameState::InGame)`** (everything shares the `MapEntity`
  marker regardless of which state spawned it), so leaving to the main menu and playing again doesn't
  duplicate the world; units will move to reacting to `game_sim`-spawned units once `game_sim` owns
  spawning.
- `game_ui` — menu modules, each spawning/despawning its own UI tree on its state's `OnEnter`/`OnExit`:
  `main_menu` defines the private `MainMenuScreen` sub-state of `GameState::MainMenu` — `Root`
  (Play/Quit) → `play_mode_menu` (`PlayMode`: Solo/Multiplayer, Multiplayer shown dimmed/WIP, no
  `Button`) → `solo_mode_menu` (`SoloMode`: New/Saves, Saves shown dimmed/WIP) → `new_game_menu`
  (`NewGame`: a **tabbed** screen, not a single one — despite the state's name surviving from when
  it was just the world-gen preview). `new_game_menu` owns the shell: a header (World/Players tab
  buttons + a Back button, all spawned once and staying on screen) over `NewGameContentArea`, an
  empty content node that `worldgen_menu`/`players_menu` mount their own UI tree into via
  `OnEnter`/`OnExit` of the private `NewGameTab` sub-state (`World`/`Players`, sourced from
  `MainMenuScreen::NewGame` — a sub-state of a sub-state, same nesting `pause_menu`'s
  `PauseMenuScreen` uses). This is deliberately unlike `PauseMenuScreen`'s Root/Keybinds pattern
  (navigate to a full sub-screen and back): tabs are meant to feel like flipping between panels of
  one dialog, so the header must survive the switch — Bevy runs a hierarchical state's OnEnter
  parent-before-child (and OnExit child-before-parent), which is exactly what lets
  `new_game_menu::spawn_shell` create `NewGameContentArea` before `worldgen_menu`/`players_menu`'s
  own `OnEnter(NewGameTab::_)` systems look it up and mount into it — if you add a third tab, its
  spawn system needs that same `Query<Entity, With<NewGameContentArea>>` lookup, not a new
  top-level root. The tab buttons' *active* state can't be shown via `BackgroundColor` — every
  `Button` in the app already gets hover/press coloring from `widgets::button_visuals`, which
  would stomp an "active" tint back to normal the instant the mouse left the button — so
  `new_game_menu::style_tabs` highlights the active tab's label via `TextColor` instead, a channel
  `button_visuals` never touches. The header is a bordered panel (`widgets::PANEL_BACKGROUND`/
  `PANEL_BORDER` — a warm dark-leather/gold-trim palette, aiming for a Civ-style read without
  needing actual art assets; deliberately scoped to `new_game_menu`/`players_menu` rather than
  replacing `widgets::BACKGROUND` app-wide, so the rest of the still-placeholder menus aren't
  dragged into a reskin nobody asked for). The header also has the **Start** button: builds the
  effective `Preset` from `WorldGenSettings` (same seed/preset the World tab last showed), calls
  `game_worldgen::generate` at `TERRAIN_WIDTH`×`TERRAIN_HEIGHT` (128×64 — *not*
  `WorldGenSettings`'s own resolution, see `game_render`'s entry for why), stores the result in
  `game_core::GeneratedWorld`, and sets `GameState::InGame` — the only place any of this gets
  wired together today; `PlayersSettings`'s roster isn't read by it yet (`game_sim` has no AI to
  hand nations to). `worldgen_menu` (left column of `-`/`+` stepper rows — View,
  Highlight, Split/Merge, Preset, Resolution, Continents, Sea level, Humidity, Temperature, Nations,
  Seed/Randomize — middle a live `ImageNode` preview in a
  fixed-size, clipped viewport with its own zoom (`-`/`+`, `x1.0`–`x6.0`), pan (a `<`/`^`/`v`/`>` pad)
  and Reset View controls underneath, right a color-key legend (`image_export::legend()`); *every*
  settings change regenerates, no separate Generate button — but not synchronously inline, and
  **deliberately not affected by Size at all**. `generate_image` (still the actual generation +
  rendering logic) runs on `AsyncComputeTaskPool` (`PendingGeneration`, polled once a frame by
  `poll_generation`) so a slow regeneration doesn't freeze the whole UI; a settings change while a
  previous run is still in flight just replaces the pending `Task`, dropping (cancelling) the stale
  one, and the preview shows a 1×1 gray placeholder image (`placeholder_image`) until the first run
  completes. **Size** (still the `resolution` field/`LabelKind::Resolution`/`WorldGenButton::Resolution*`
  internally — only the shown label changed) is the world's actual quad grid width — real terrain
  size, shown with its real size in km (`preset::METERS_PER_QUAD`), consumed by `game_render` at Start
  — but the live preview here always renders at its own fixed `PREVIEW_RESOLUTION` (512×256)
  regardless of Size, so changing Size never touches the preview image or triggers regeneration at
  all (`size_changed`, tracked separately from `changed` in `button_actions`, only updates Size's own
  label text). An earlier version made Size and the preview's resolution the same number, on the
  theory that "what you preview is what you get" should mean the preview reflects real size too — in
  practice that meant every Size tweak regenerated the *entire* preview for a change invisible at the
  small, fixed `ImageNode` display box (a bigger world doesn't look bigger in a 560×280 box, just
  finer-grained, which isn't visible there either), so a real, possibly-expensive regeneration for
  *zero* visible difference read as broken, not as a feature. Size only matters again at Start, and
  its range (128–8192, ~16km) is deliberately generous now that changing it is free — the cost moved
  entirely to Start's regeneration, which is why that has `loading_menu`'s progress screen. Picking a
  **Preset** calls `WorldGenSettings::apply_preset_defaults` to overwrite Continents/Sea level/
  Humidity/Temperature with that preset's own values — without this, switching presets kept stale
  values from whatever was selected before, which is why "archipelago" used to not look like an
  archipelago.
  **View** toggles the preview between the biome map and the hypsometric heightmap; **Highlight**
  overrides either view with `image_export::highlight_landmasses` (each landmass a distinct flat
  color, via `stats::label_landmasses`) — a verification view for checking the actual continent count
  and separation at a glance, which is otherwise hard to eyeball through biome coloring.
  **Split/Merge** toggles `WorldGenSettings::correct_continents` (see `game_worldgen`'s entry for what
  it controls) — off shows the raw noise-generated landmasses, letting you tell apart "the correction
  pass did this" from "the noise itself did this" while tuning a preset. Zoom/pan is a
  `PreviewZoom` resource (persists across screen visits, like `WorldGenSettings`) driving the
  displayed image node's size/position inside the clipped viewport — buttons only, deliberately not
  scroll-wheel/drag: `game_input`'s "consumers never read `ButtonInput`/`MouseWheel` directly" rule
  (see its entry below) covers this screen too, and every other control here is already button-driven.
  Zoom/pan changes never trigger regeneration (`button_actions` tracks that separately from settings
  changes that do)) and `players_menu` (`PlayersSettings` resource: slot 0 is always the local human
  player — fixed, not removable — every slot after it an AI added via "+ Add AI" (capped at 8 total)
  and removed via its own Remove button; every slot, human included, gets a Nation `-`/`+` stepper
  cycling Random plus whatever `game_assets::Civilizations` loaded (see `game_assets`'s entry — a
  JSON config, not a hardcoded list), with a color swatch next to each row from the picked
  civilization's `color`. The roster list is in a fixed-height (`LIST_HEIGHT`), `Overflow::scroll_y()`
  viewport — without a fixed height nothing ever overflows, so scrolling had nothing to actually
  scroll, which is why it silently didn't work before; `widgets::scroll_on_hover` (see below) is what
  actually moves it. The row list isn't diffed on add/remove/nation-change: `button_actions` despawns
  every child of `PlayerListArea` and respawns the whole list from `PlayersSettings` — the roster is
  capped at 8 rows, so a full rebuild is cheap and sidesteps tracking per-row entities through index
  shifts when a middle slot is removed) are today's two tabs. Both settings resources persist across
  tab switches and leaving/re-entering the screen (normal resources, not reset by `OnEnter`).
  `new_game_menu`'s Start button doesn't generate the world itself — it spawns a `PendingStart` task
  (same `AsyncComputeTaskPool` pattern as `worldgen_menu`'s preview) and switches to
  `GameState::Loading`; `loading_menu` (its own module: `OnEnter(GameState::Loading)` spawns a
  full-screen "Loading..." text plus a status line, `OnExit` despawns it) polls that task
  (`poll_world_generation`) and, once done, sets `game_core::GeneratedWorld` — from there
  `game_render::map`'s chunk-by-chunk mesh building takes over, and *that* system (not this one) is
  what finally sets `GameState::InGame`. The status line reads `GeneratedWorld`/
  `game_core::TerrainBuildProgress` each frame: "Generating world..." while the former is still
  `None` (world generation itself has no sub-progress to report — one `game_worldgen::generate` call,
  not restructured into stages), then "Building terrain: X / Y chunks" once the latter has something
  to show.
  `widgets.rs` gained `scroll_on_hover`: any node with both `ScrollPosition` and `Interaction` (the
  latter just for hover tracking — it doesn't need to be a `Button`) scrolls under the cursor on
  mouse wheel. This reads `MouseWheel` directly rather than going through `game_input` — unlike a
  semantic, rebindable game action (camera pan/zoom), list scrolling is UI-internal with no keybind
  to speak of, the same reasoning that already lets `keybinds_menu` read raw `ButtonInput<KeyCode>`
  directly for its rebind-capture flow. Bevy's own layout pass clamps the resulting `ScrollPosition`
  to the content's actual overflow every frame, so no manual bounds-checking was needed. Separately,
  `pause_menu` (Escape-toggled `PauseState::Paused`;
  defines the private `PauseMenuScreen` sub-state — `Root`/`Keybinds` — and `handle_escape`, context-
  sensitive: cancel a keybind capture, else back out of the keybinds screen, else open/close the pause
  menu), `keybinds_menu` (`PauseMenuScreen::Keybinds` — click a bind, press a new key, saved immediately
  via `game_config::save()`). All placeholder styling: default font, flat colors, no art assets. Shared
  button-hover/press styling lives in `widgets.rs`. HUD/selection box/minimap land here too, eventually.
- `game_input` — translates raw keyboard/mouse into semantic actions (`CameraPanAction`,
  `CameraZoomAction`, `ToggleDebugOverlay`, `TogglePauseMenu`), reading primary keys from
  `game_config::KeyBindings` where applicable (arrow keys and Escape are fixed, not user-configurable),
  ordered via the `InputSet` system set. Key/mouse bindings live only here; consumers react to actions,
  never read `ButtonInput`/`MouseWheel` directly. See "Controls" in `README.md`.
- `game_assets` — data-driven definitions. `Civilizations` (`civilizations.rs`) is the first real
  content: a `Startup` system does a blocking `std::fs::read_to_string` +
  `serde_json::from_str::<Vec<CivilizationDef>>` on `assets/civilizations.json` (path relative to the
  working directory, same convention Bevy's own `AssetPlugin` uses for its `assets/` folder), falling
  back to a small built-in list — not panicking, not an empty roster — if the file is missing or
  fails to parse, since this is player-facing flavor data, not something that should be able to brick
  the New Game screen. Plain `std::fs`/`serde_json` rather than Bevy's `AssetServer`: this only needs
  to exist as a `Resource` before `game_ui::players_menu` reads it, and a full custom `AssetLoader`
  (async, hot-reload-capable) is more machinery than a handful of name/color definitions need right
  now. Future unit/building definitions will likely be RON under `data/` instead — no particular
  reason both need the same format, JSON was what was asked for here.
- `game_save` — empty plugin stub, wired into `game` but with no systems yet.
- `game_net` — optional networking, exists in the workspace but is **not** a dependency of `game` yet
  (add it once an authority model — lockstep vs. server-authoritative — is decided).
- `game_worldgen` — procedural cylindrical terrain generation (continents, mountains, rivers, lakes,
  Civ-style terrain/biomes/features, nation-starting-position placement). Deliberately **no `bevy`
  dependency** — pure computation, called from `game_ui::worldgen_menu` (Play → Solo → New) for the
  in-game preview screen, and also runnable standalone via `cargo run -p game_worldgen --example
  generate -- --preset continents --seed 42 --out world.png` (also takes `--continents`/`--sea-level`/
  `--humidity`/`--temperature`/`--nations`, kept in sync with the UI's knobs) for faster
  parameter-tuning iteration without the game UI. Output is an image either way (colored biome map,
  or `image_export::elevation_hypsometric`'s topographic heightmap — `worldgen_menu`'s View toggle
  switches between them), not mesh terrain yet — that conversion is future work.
  **Continents are noise-shaped, then count-corrected — not a distance field.** A seed-point
  distance-field approach (grow each continent from one of `continent_count` well-spaced seeds) was
  tried first specifically to *guarantee* the exact count, and it worked — verified against targets
  1-24 — but every continent came out a recognizable blob; that's inherent to distance fields, not
  fixable by adding more edge noise. Reverted to pure `Fbm3` noise for the actual shape (same
  organic character as the rest of the generator), with `elevation::correct_continent_count`
  flood-filling the result afterward and merging the closest two landmasses (land bridge between
  centroids) or splitting the largest one (a gently wavy strait across its shorter axis, not a
  straight line — a straight cut reads as obviously artificial too) until the count matches or an
  iteration cap is hit. Gets close almost always, exact often, not guaranteed — that's the accepted
  tradeoff for organic shape over exact-by-construction. **Splits are a geodesic partition, not a
  coordinate-line cut**: an earlier version of `split_land` cut at the median of the component's
  cell coordinates along its longer axis (a gently wiggled line, to avoid reading as artificial) —
  but noise-generated continents are routinely concave (horseshoe bays, peninsulas wrapping back on
  themselves), and on those a coordinate-line cut carves a scratch across the landmass *without*
  actually disconnecting it, since the two "halves" stay joined around the open side.
  `label_land_components` then still reports one landmass, so `correct_continent_count` retries with
  a different jitter next iteration — and each failed attempt leaves another thin, pointless channel
  of ocean gouged into the continent that never finishes separating anything (a "river of ocean"
  running through otherwise-solid land). `split_land` now seeds two points at the landmass's
  approximate diameter (farthest-from-farthest via double BFS), then does a simultaneous
  multi-source BFS over just that component's cells so every cell is labeled by whichever seed's
  flood front reaches it first — a geodesic Voronoi split that follows the landmass's actual
  connectivity and so can't fail to separate it, however concave. **Both the split and the merge are
  wide, not a hairline.** The gap between the two BFS halves (a second multi-source BFS from the seam
  gives every cell its ring-distance from it) and `bridge_land`'s isthmus are both cleared/raised out
  to `strait_half_width` (map-scaled, `noise`-perturbed per cell so the edge isn't a uniform band) —
  at the old fixed 1-3px width, either operation "worked" in the sense that the landmass count came
  out right, but at any resolution big enough to look good otherwise the strait/isthmus was a handful
  of pixels swallowed by antialiasing, i.e. invisible. `bridge_land` also gives its isthmus a
  radial-dome elevation profile (high at the centerline, tapering to just-above-`sea_level` at the
  edge) plus tiny per-cell jitter (`cell_jitter`) rather than one flat constant everywhere, purely so
  it doesn't read as a dead-flat mesa in the elevation view — `hydrology::generate`'s priority-flood
  drainage (below) means this is cosmetic, not load-bearing for the merge reading as actual land; an
  earlier version *needed* an `ElevationMaps::forced_land` mask to stop the old lake-detection
  heuristic from misreading the whole isthmus as one big depression, which is no longer necessary and
  has been removed. **`worldgen_menu`'s "Split/Merge" toggle** (`WorldGenSettings::correct_continents`
  → `Preset::correct_continents`, read by `elevation::generate` to skip calling
  `correct_continent_count` at all) shows the raw noise-generated landmasses untouched — useful for
  telling "is this ugly because of the correction pass" from "is this ugly regardless." The CLI's
  equivalent is `--no-correct`. **If you touch `label_land_components` again**:
  `Grid::neighbors` returns *raw*, unwrapped offsets
  (e.g. `-1` past the west edge) meant to be passed straight to `Grid::get`/`set` (which wrap via
  `rem_euclid`) — casting such an offset directly to `usize` wraps `-1` to `usize::MAX` instead,
  corrupting any component touching the map's seam and panicking later ("attempt to negate with
  overflow") on the resulting garbage coordinate. Keep flood-fill walks in `i64` throughout
  (`stats::count_landmasses`'s pattern) and wrap-and-cast only once, at the point a result is
  stored. `Preset` (`preset.rs`) holds the
  "flavor" knobs (mountain strength/belt, river threshold, continent-noise octaves) plus
  `continent_count` directly; `sea_level`/`moisture_bias`/`temperature_bias` get overridden
  per-generation from user-facing values (`WorldGenSettings::effective_preset` in
  `worldgen_menu.rs`) rather than being baked into the `const` presets. **Mountain belts are chunky
  massifs, not hill speckle**: `elevation::generate`'s `BELT_THRESHOLD` (0.62, was 0.72) widens how
  much of the low-frequency belt mask actually counts as mountainous; every preset's
  `mountain_strength`/`mountain_belt_radius` were raised/lowered respectively on top of that for
  taller peaks packed into fewer, bigger contiguous ranges rather than many small thin ones.
  **Hydrology is priority-flood, not a per-cell sink heuristic**: `hydrology::generate` used to do
  steepest-descent flow direction (landing on `None`/a "sink" wherever no neighbor was strictly
  lower) and a separate "sink cell + near-equal-elevation neighbors" pass for lakes — both are local,
  one-hop heuristics blind to the terrain beyond a cell's immediate neighbors, so rivers dead-ended at
  every small pit instead of continuing downhill to the sea, and lake shape/extent was an
  approximation. It now runs a proper priority-flood depression fill (Barnes et al.) seeded from the
  ocean: repeatedly pop the lowest-`filled` unvisited cell from a min-heap and relax its neighbors to
  `max(their elevation, this cell's filled elevation)`, recording which cell each neighbor got relaxed
  from as its flow target. The result has no interior local minima except the ocean itself, so every
  land cell has a real, monotonically-draining path to the sea (rivers now actually trace the
  terrain's verticality end to end), and `filled > elevation` at a cell is, exactly and by
  construction, a real filled basin — that's the new `is_lake` rule, replacing the old heuristic
  entirely. `nations::place` is a
  separate post-`generate()` step (rejection-sampled land-only positions via `sampling.rs`,
  cylinder-wrap-aware spacing), not part of the pipeline — it only reads the finished `World`.
  `biome.rs` adds **Ice** (cold Ocean/Coast), **Savanna** (hot+dry) and **Steppe** (cold+dry —
  Desert's cold counterpart the way Savanna is Plains'/Grassland's warm one; both splits reuse the
  same `temp > 0.55` threshold) terrain, plus **Volcano**/**Oasis**/**Floodplains**/**Reef**
  features. Floodplains is river-adjacent Desert/Steppe specifically (vs. Marsh's general high
  ambient moisture). Reef is a feature *on* `Coast` — `biome_map`'s `Coast` match arm had to start
  rendering features at all for this, since it previously skipped feature rendering entirely (a
  latent bug, harmless only because nothing had ever put a feature on a water tile before).
  Volcano/Oasis/Reef are placed via a deterministic per-cell hash (`biome::cell_random`, reusing
  `noise::splitmix64`), not a noise field — they just need "unpredictable but reproducible here."
  Their odds (in `biome::generate`) were bumped well past "so rare you might never see one": Volcano
  0.015→0.05 (on `Mountains`), Oasis 0.03→0.09 (on `Desert`), Reef 0.12→0.22 (on warm `Coast`).
  Hand-rolled 3D Perlin noise (no `noise` crate) so the cylinder's seamless-X-wrap sampling
  (`noise::cylinder_point`) is under full control; `image` is a real dependency (both here and in
  `game_ui`, to build/read `RgbImage`). `stats::label_landmasses` flood-fills the actual generated
  terrain (cylinder-wrap-aware via `Grid::neighbors`) and gives each landmass a 0-based id (`-1` for
  ocean/lake/ice/coast and anything under the min-size threshold); `count_landmasses` is just its
  count, so the number `worldgen_menu` shows next to the target and the CLI prints always agrees with
  what `image_export::highlight_landmasses` (worldgen_menu's Highlight toggle) draws. Resolution goes
  up to 2048×1024
  (~2.2s to generate including the correction pass, measured) — Continents up to 60. See "World
  generation" in `README.md` before retuning presets.

This is still early: most crates are empty scaffolding. Extend the existing plugin/crate structure rather
than introducing new top-level crates or restructuring further unless the task calls for it.

## Commands

- Build: `cargo build`
- Run: `cargo run`
- Check (fast compile-check without producing a binary): `cargo check`
- Test: `cargo test`
- Run a single test: `cargo test <test_name>`
- Format: `cargo fmt`
- Lint: `cargo clippy`

## Environment notes (Windows on ARM64 / Snapdragon X)

The dev machine is ARM64 Windows (Snapdragon X). A few non-obvious things had to be fixed to get
`cargo build`/`cargo run` working here — if builds or graphics break again, check these first:

- **rustup toolchain must be `stable-aarch64-pc-windows-msvc`**, not `stable-aarch64-pc-windows-gnullvm`.
  The gnullvm target produced `.exe` files Windows refuses to load (`os error 193`,
  "not a valid Win32 application"). Check with `rustup show`; fix with
  `rustup default stable-aarch64-pc-windows-msvc` (then `cargo clean` before rebuilding, since object
  files from the two toolchains aren't compatible with each other).
- **Git Bash shadows the real linker.** Git for Windows ships its own `link.exe` (GNU coreutils' `link`,
  for hardlinks) in `usr/bin`, which sits ahead of MSVC's `link.exe` on PATH in Git Bash shells. This
  causes linker failures like `link: extra operand ... Try 'link --help'` — that error text is coreutils',
  not MSVC's. Build from a shell that has the real Visual Studio linker on PATH (e.g. after sourcing
  `vcvarsall.bat`), not a bare Git Bash session.
- **The default MSVC toolset may be missing the ARM64→ARM64 linker.** On this machine the newest installed
  toolset (14.51.36231) only ships `HostARM64\{x64,x86}\link.exe` — no native ARM64 target linker — while
  an older toolset (14.44.35207) has `HostARM64\arm64\link.exe`. If linking fails only for the native
  target, pin the older toolset: `vcvarsall.bat arm64 -vcvars_ver=14.44`.
- **Vulkan (wgpu's default backend here) flickers on the Adreno X1 GPU.** The window alternates between a
  light-gray blank frame and the actual scene. Forcing DX12 fixes it: run with the `WGPU_BACKEND=dx12`
  environment variable set (e.g. `WGPU_BACKEND=dx12 cargo run`).
