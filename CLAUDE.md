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
  default → `InGame`) and `PauseState` (`Running`/`Paused`, a `SubStates` of `GameState::InGame` — kept
  separate from `GameState` specifically so pause/resume doesn't re-trigger `OnEnter(GameState::InGame)`
  world-spawning). Zero rendering deps. Gameplay-wise currently just the `Unit` marker component.
- `game_sim` — deterministic simulation on `FixedUpdate`: `MovementPlugin`, `CombatPlugin`,
  `PathfindingPlugin`, `OrdersPlugin`, ordered via the `SimSet` system set. All currently empty stubs — no
  sim logic has been implemented yet.
- `game_render` — presentation: `CameraPlugin` (pan/zoom 3D camera, driven by `game_input` actions,
  spawns at `Startup` but pan/zoom only run `in_state(GameState::InGame)`), `MapPlugin`/`UnitsPlugin`
  (ground/light/unit-cube placeholders, spawn on `OnEnter(GameState::InGame)` **and despawn on
  `OnExit`**, so leaving to the main menu and playing again doesn't duplicate the world; will move to
  reacting to `game_sim`-spawned units once `game_sim` owns spawning).
- `game_ui` — menu modules, each spawning/despawning its own UI tree on its state's `OnEnter`/`OnExit`:
  `main_menu` defines the private `MainMenuScreen` sub-state of `GameState::MainMenu` — `Root`
  (Play/Quit) → `play_mode_menu` (`PlayMode`: Solo/Multiplayer, Multiplayer shown dimmed/WIP, no
  `Button`) → `solo_mode_menu` (`SoloMode`: New/Saves, Saves shown dimmed/WIP) → `worldgen_menu`
  (`WorldGen`: left column of `-`/`+` stepper rows — View, Highlight, Split/Merge, Preset, Resolution,
  Continents, Sea level, Humidity, Temperature, Nations, Seed/Randomize — middle a live `ImageNode` preview in a
  fixed-size, clipped viewport with its own zoom (`-`/`+`, `x1.0`–`x6.0`), pan (a `<`/`^`/`v`/`>` pad)
  and Reset View controls underneath, right a color-key legend (`image_export::legend()`); *every*
  settings change regenerates immediately via `generate_image`, no separate Generate button. Picking a
  **Preset** calls `WorldGenSettings::apply_preset_defaults` to overwrite Continents/Sea level/
  Humidity/Temperature with that preset's own values — without this, switching presets kept stale
  values from whatever was selected before, which is why "archipelago" used to not look like an
  archipelago. **Resolution** only changes image detail (width/height in pixels, height always half
  width); it's deliberately decoupled from how much world exists — see `game_worldgen`'s entry below.
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
  changes that do). Separately,
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
- `game_assets`, `game_save` — empty plugin stubs, wired into `game` but with no systems yet.
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
