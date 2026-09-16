# rts-game

A Bevy-based RTS/grand strategy game (see `rts-game-design-doc.md` for the design vision and
`rts-bevy-architecture-notes.md` for the architecture this workspace layout follows).

## Layout

A Cargo workspace, one crate per architectural boundary:

```
crates/
├── game/            # thin binary (`rts-game`): loads config, builds the App, configures
│                     # DefaultPlugins, wires in the library crates below, plus dev-only diagnostics
├── game_config/       # persisted user settings (graphics backend, camera speeds, keybinds) —
│                       # see "Configuration" below
├── game_core/           # sim ECS data: components, resources, events, GameState — zero
│                         # rendering deps
├── game_sim/              # deterministic simulation on FixedUpdate: movement, combat,
│                           # pathfinding, orders — ordered via the SimSet system set
├── game_render/             # presentation: camera, map, unit visuals — reads game_core
│                             # state, never the other way around; gated to GameState::InGame
├── game_ui/                   # main menu (Play → Solo/Multiplayer → New/Saves → world generator
│                               # preview), ESC/pause menu, keybind-rebind screen today; HUD,
│                               # selection box, minimap land here too, eventually
├── game_input/                  # raw input (keyboard, mouse) → semantic actions; key
│                                 # bindings live only here, see "Controls" below
├── game_assets/                   # asset loading, data-driven unit/building definitions (RON)
├── game_save/                       # save/load
├── game_net/                          # optional multiplayer — not a dependency of `game` yet
└── game_worldgen/                       # procedural cylindrical terrain generation — see
                                          # "World generation" below. No bevy dependency itself,
                                          # but game_ui depends on it to drive the in-game preview
                                          # screen; still standalone-runnable too (see below)
assets/                                    # game assets (models, textures, audio, ...) — currently
                                            # placeholders (colored meshes, default font), real
                                            # assets TBD
data/                                       # RON/JSON data: unit stats, tech trees, etc.
```

`game_core` and `game_sim` have no rendering dependencies — that boundary is what eventually
enables running the sim headless (dedicated server, replays, automated balance testing).
`game_render`/`game_ui` depend on `game_core` to read state, never the reverse.

Everything outside `game_core`'s `Unit`/state types, `game_render`'s camera/map/unit-spawning,
`game_input`'s action translation, `game_config`'s settings, and `game_ui`'s menus is still an
empty plugin stub — the crate boundaries exist, the gameplay logic doesn't yet.

## Game state

Nested states (`#[derive(States)]`/`#[derive(SubStates)]`) drive UI/world flow:

- `GameState` (`game_core`) — `MainMenu` (default) → `InGame`. `game_render`'s map/unit spawning
  runs on `OnEnter`/`OnExit(GameState::InGame)` instead of `Startup` (spawns once on entry,
  despawns on exit so "Quit to Main Menu" → "Play" again doesn't duplicate the world); camera
  pan/zoom is gated `run_if(in_state(GameState::InGame))`. The 3D camera itself still spawns at
  `Startup` unconditionally — `bevy_ui` needs a camera present to render menus onto.
- `PauseState` (`game_core`, sub-state of `GameState::InGame`) — `Running`/`Paused`, toggled by
  Escape. A sub-state (not a third `GameState` variant) specifically so pausing/resuming never
  re-triggers `OnEnter(GameState::InGame)` and re-spawns the world.
- `MainMenuScreen` (`game_ui::main_menu`, private, sub-state of `GameState::MainMenu`) —
  `Root` (Play/Quit) → `PlayMode` (Solo/Multiplayer) → `SoloMode` (New/Saves) → `WorldGen`
  (settings on the left, live image preview on the right — see "World generation"). Resets to
  `Root` every time the menu opens.
- `PauseMenuScreen` (`game_ui::pause_menu`, private, sub-state of `PauseState::Paused`) —
  `Root`/`Keybinds`, which pause-menu screen is showing. Resets to `Root` every time the menu
  opens.

Every `game_ui` screen module (`main_menu`, `play_mode_menu`, `solo_mode_menu`, `worldgen_menu`,
`pause_menu`, `keybinds_menu`) spawns/despawns its own UI tree on its state's `OnEnter`/`OnExit` —
no leftover entities across transitions.

Escape is context-sensitive (`pause_menu::handle_escape`): cancel an in-progress keybind capture,
else back out of the keybinds screen to the pause root, else open/close the pause menu.

**Multiplayer and Saves are shown but not clickable** ("(WIP)", dimmed text with no `Button`
component) — `game_net` and `game_save` are still empty stubs, so making them look clickable would
be dishonest rather than just unfinished.

## Controls

| Input | Action |
|---|---|
| `W`/`A`/`S`/`D` or arrow keys | Pan camera (north/west/south/east) |
| Mouse scroll | Zoom in/out |
| `F3` | Toggle the performance overlay (hidden by default) |
| `Esc` | Open/close the pause menu (context-sensitive — see "Game state") |

`game_input` owns every binding above (`CameraPanAction`, `CameraZoomAction`,
`ToggleDebugOverlay`, `TogglePauseMenu`), reading the *primary* key for the first three from
`game_config::KeyBindings` (arrow keys are a fixed fallback for pan, and Escape is hardcoded — both
not user-configurable). Consumers (`game_render::CameraPlugin`, `game`'s diagnostics overlay,
`game_ui::pause_menu`) react to actions and never read `ButtonInput`/`MouseWheel` directly.
Input-reading systems run in `game_input::InputSet`; consumers order themselves `.after(InputSet)`
so they see the current frame's input.

**Rebinding at runtime:** open the pause menu (`Esc`) → Settings, click a bind, press the new key
(`Esc` cancels). `game_ui::keybinds_menu` writes straight into the `KeyBindings` resource and calls
`game_config::save()` immediately — no separate "apply"/"save" step.

## Configuration

`game_config` is the single source of truth for user-facing settings — graphics backend, camera
pan/zoom speed and height clamp, and keybinds — bundled into one `Settings` struct, serialized as
RON, and persisted to the OS config directory (e.g. `%APPDATA%\rts-game\config\settings.ron` on
Windows) via the `directories` crate, so settings survive across runs/launches. Gameplay constants
that aren't user-facing (unit spawn counts, ground size, ...) stay as plain `const`s in their own
crates — they don't belong in a settings file a player might edit.

`game_config::load()` is called once, directly in `main()`, *before* the `App` is built — the
graphics backend has to be known before `RenderPlugin` is configured, which happens inline in the
same `add_plugins(DefaultPlugins...)` call, ahead of any plugin's `build()`. The loaded `Settings`
are then handed to `game_config::GameConfigPlugin`, which inserts `GraphicsSettings`,
`CameraSettings`, and `KeyBindings` as resources for the rest of the app to read. `--backend=dx12`/
`--backend=vulkan` still works as a one-off CLI override (not persisted). `game_ui::keybinds_menu`
calls `game_config::save()` immediately after every successful rebind — the only place that calls
it today. Graphics/camera settings have no in-game editor yet, so they only change by hand-editing
`settings.ron`.

## World generation

`game_worldgen` generates the cylindrical world procedurally (design doc § World & Geography):
continents/oceans, mountain ranges, rivers, lakes, and Civ-style terrain/biomes/features. It's
deliberately not a `bevy` crate — pure computation over a grid, runnable standalone, in tests, and
later headless on a server, independent of the renderer. For now, output is an image, not mesh
terrain — that conversion is future work once the generation itself is in good shape.

**In-game**: Play → Solo → New opens `game_ui::worldgen_menu` — settings on the left (`-`/`+`
steppers, no text-input widget needed), a live preview `ImageNode` in the middle, a color legend
on the right (`image_export::legend()` — name + swatch for every terrain/feature/river/nation
color the preview can draw, so the key can never drift from the actual colors). Every single
change (View, Preset, Resolution, Continents, Sea level, Humidity, Temperature, Nations, Seed, or
Randomize Seed) regenerates and redraws the preview immediately — there's no separate "Generate"
button. Regeneration is synchronous on the click (fast enough at these map sizes not to need
threading — see Resolution below for the actual numbers); all settings persist in a plain
`Resource` while you navigate Back and return. It stays a preview only: nothing wires the
generated `World` into `game_render`'s map yet (see above). The seed steppers/Randomize are a
simple wrapping-multiply step, not real entropy — they just need to look different each time, not
be unpredictable.

**View** toggles the preview between the colored biome map and `image_export::elevation_hypsometric`
— a topographic-map-style rendering (deep blue → shallow blue below sea level, green → yellow →
brown → white above it) that actually reads as verticality, unlike flat grayscale
(`elevation_grayscale`, still available, mostly useful for raw sanity-checking).

Exposed knobs, and how each maps onto `Preset`:
- **Preset** cycles the four named presets below. Picking one calls
  `WorldGenSettings::apply_preset_defaults`, which **overwrites** Continents/Sea level/Humidity/
  Temperature with that preset's own values. This used to not happen — switching to "archipelago"
  kept whatever continent count was left over from the previous preset, so it never actually
  looked like an archipelago. Preset flavor that stays preset-only, not exposed as a control:
  mountain strength/belt shape, river threshold, main continent-noise octave count.
- **Resolution** (128-2048px, width; height is always half) — image detail only. Deliberately
  doesn't affect how much "world" exists: continent placement is computed from map *area*, which
  scales together with resolution (both dimensions scale, so the ratio is constant), so the same
  Continents setting produces the same relative landmasses regardless of resolution — a "Large"
  resolution is the same world at more pixels, not literally a bigger world. If you want more
  world, raise Continents/Nations. 2048×1024 (with the count-correction pass below) generates in
  ~2.2s, measured via the CLI — the cap is set where a per-click UI freeze stays reasonable.
- **Continents** (1-60) → `Preset::continent_count`, converted to noise-space frequency, then
  corrected toward the exact count after the fact — see "Continents: noise shape + a
  count-correction pass" below. Shown as `target (~actual)`: this gets close almost always and
  exact often, but not always — that's expected, not a bug (an exact-by-construction approach was
  tried and looked worse — see below).
- **Sea level** (0.30-0.70), **Humidity** (±0.30 → `moisture_bias`), **Temperature** (±0.30 →
  `temperature_bias`) — each directly overrides the chosen preset's value; `WorldGenSettings::
  effective_preset` builds a modified copy of the preset every regeneration, it doesn't mutate the
  `const` presets.
- **Nations** (1-32) → not a `Preset` field at all. `game_worldgen::nations::place` scatters that
  many starting positions across habitable land (rejection-sampled apart from each other,
  cylinder-wrap-aware distance, land only — no Ocean/Coast/Lake/Ice/Snow), drawn onto the preview
  by `image_export::draw_nations` as white-disk-with-black-ring markers. Placement only, no
  borders or growth — it exists so "number of nations" visibly does something ahead of any real
  nation system.

**Continents: noise shape + a count-correction pass, not a distance field.** This has been through
two designs. The first replaced noise entirely with seed-point placement (`continent_count`
well-spaced seeds, each continent grown outward as a distance field) specifically to *guarantee*
the exact count — and it worked, verified directly across targets 1-24. But every continent came
out as a recognizable blob: a distance field is fundamentally circular no matter how much edge
noise gets layered on top, and that read as conspicuously artificial. The second (current) design
goes back to pure layered noise for the actual shape — the same organic, branching-coastline
character every other part of this generator has — and corrects the *count* afterward instead of
trying to guarantee it during generation:

1. `elevation::generate` builds continents from `Fbm3` noise exactly like before the seed-point
   detour (`continent_count` converted to a noise-space radius via the `2π` relationship — see
   `preset.rs`), which produces *roughly* the right number of landmasses with naturally irregular
   coastlines, same as any noise-driven terrain.
2. `elevation::correct_continent_count` then flood-fills the actual landmasses (wrap-aware) and,
   in a loop, **merges** the two closest ones (raises a land bridge along the line between their
   centroids) if there are too many, or **splits** the largest one (cuts a strait — a gently wavy
   line, not a ruler-straight one, across its shorter axis) if there are too few, until the count
   matches or an iteration cap is hit.

This gets close to the target almost always and exactly right often, while keeping the organic
shape — a real, if approximate, improvement over "exactly right but looks like circles."

**Splits bisect by area, not by bounding box.** The first version of `split_land` cut at the
midpoint of the component's bounding box — for a lopsided landmass (most of its mass off to one
side of that box) that carves off a small sliver instead of two comparable landmasses. It now cuts
at the **median** of the component's cell coordinates along the chosen axis (`elevation::median`):
sort the cells, cut where half of them are on each side. Median-by-cell-count is a direct area
bisector on a uniform grid, so the two resulting landmasses come out roughly equal in size — "an
ocean splits the continent into relatively same-area land," not "into whatever the geometry left
over." The cut line is also a gentle sine-wave meander (`wiggle`) rather than a straight line — a
dead-straight strait reads as obviously artificial the same way a dead-straight coastline would.

A prior version of this correction pass had a sharp bug worth knowing about if you touch this code:
`Grid::neighbors` returns *raw*, unwrapped offsets (e.g. `-1` just past the west edge) and relies
on `Grid::get`/`set` to wrap them via `rem_euclid` — casting such a raw offset straight to `usize`
(as an earlier version of the flood-fill did) wraps `-1` to `usize::MAX` instead, corrupting any
component that touched the map's seam and crashing later arithmetic on it. Keep flood-fill walks
in `i64` throughout (see `stats::count_landmasses`'s pattern) and only wrap-and-cast at the very
end, once, when storing a result.

`game_worldgen::stats::count_landmasses` (the read-only measurement used for the UI's
`target (~actual)` display and the CLI's printed count) uses the same flood-fill pattern, checked
against `Terrain` rather than raw elevation, and was the reference for getting the correction
pass's version right.

`worldgen_menu::rgb_to_bevy_image` is the bridge from `game_worldgen`'s plain `image::RgbImage`
output to a real `bevy::image::Image` asset: expand RGB → RGBA (alpha 255), wrap in
`Image::new(Extent3d, TextureDimension::D2, _, TextureFormat::Rgba8UnormSrgb, _)`, `Assets<Image>::add`
it, point an `ImageNode` at the resulting handle. `game_ui` is the one crate that bridges plain
worldgen output into Bevy-flavored data — `game_worldgen` itself still knows nothing about Bevy.

**Standalone CLI** (faster iteration when tuning generation parameters — no need to click through
the game's UI, or even build it):

```
cargo run -p game_worldgen --example generate -- --preset continents --seed 42 \
    --width 512 --height 256 --continents 4 --sea-level 0.5 --humidity 0.0 \
    --temperature 0.0 --nations 8 --out world.png
cargo run -p game_worldgen --example generate -- --list-presets
```

`--continents`/`--sea-level`/`--humidity`/`--temperature`/`--nations` are the same knobs
`worldgen_menu` exposes, kept in sync here for CLI-based iteration without the game UI.

Writes `world.png` (the colored biome map) and `world.elevation.png` (the hypsometric heightmap —
see View above), and prints the actual measured landmass count (see "Continents" above).

**Presets** (`preset.rs`): `continents`, `pangaea`, `archipelago`, `highlands` — different values
for the same tunable knobs (continent count, sea level, mountain strength, river threshold,
moisture bias), not different generation logic. Add a preset by adding a `const`.

**Pipeline** (`lib.rs::generate`, one module per stage): `elevation` (seed-based continent shape +
mountain belts) → `climate` (temperature from latitude + altitude + `temperature_bias`, moisture
from noise + water proximity + `moisture_bias`) → `hydrology` (D8 flow accumulation → rivers;
local minima → lakes) → `biome` (Civ-style classification — see "Terrain and features" below).
`nations::place` is a separate step called after `generate()`, not part of the pipeline itself —
it only needs the finished `World` (specifically `biome.terrain`, to know what's habitable).

**Terrain and features** (`biome.rs`): base `Terrain` —
Ocean/Coast/Ice/Lake/Grassland/Plains/Savanna/**Steppe**/Desert/Tundra/Snow — plus `ElevationBand`
(Flat/Hills/Mountains) and `Feature` overlays —
Forest/Jungle/Marsh/Volcano/Oasis/**Floodplains**/**Reef**. Ice replaces Ocean/Coast below a
cold-temperature threshold (poles previously rendered as ordinary dark-blue ocean regardless of
temperature). Savanna and Steppe are the warm and cold counterparts of the same gap: the original
classification sent *any* low-moisture land to Desert regardless of temperature, so a
Sahara-style hot desert and a Central-Asian-style cold dry plain looked identical; Steppe is
Desert's cold half the way Savanna is Plains'/Grassland's warm half (same `temp > 0.55` threshold
both splits use, for consistency). Floodplains is a fertile strip specifically along a river
running through otherwise-arid land (Desert/Steppe) — distinct from Marsh, which comes from
generally high ambient moisture rather than one specific river. Reef is a feature *on* `Coast`
(warm, ~12% per-cell chance) rather than its own terrain, the way Volcano is a feature on
Mountains rather than its own elevation band — `biome_map`'s `Coast` branch had to change to
render features at all, since Coast previously skipped feature rendering entirely (harmless while
no water-tile feature existed, a bug in waiting once Reef needed to show up). Volcano (Mountains,
1.5%) and Oasis (Desert, 3%) are placed via a deterministic per-cell hash (`biome::cell_random`,
reusing `noise::splitmix64`), not a continuous noise field, since they just need to be
"unpredictable but reproducible here," not smoothly varying; Reef uses the same mechanism.

**Seamless cylinder wrap** (`noise.rs`): the map wraps in X (longitude) but not Y (latitude/poles).
Sampling 2D noise directly along X would show a seam at the wrap; instead, each column's X
coordinate maps to an angle and is sampled on a circle embedded in a hand-rolled 3D Perlin noise
field (`cylinder_point`) — walking all the way around returns exactly to the start, so it's
seamless by construction. No `noise`-crate dependency; this needed custom cylindrical sampling
control anyway, and hand-rolling avoided guessing at an unfamiliar crate's exact API. `image` is a
dependency (PNG output) — that one's a real "don't reinvent this" case, unlike noise generation.
`sampling.rs` shares point-scattering logic (rejection sampling, wrap-aware distance) between
continent-seed and nation placement.

Simplification worth knowing about: lakes are a heuristic (local elevation minima + a small ring
around them), not exactly-computed drainage basins (real depression-filling hydrology). Good
enough for a preview map; revisit if lake placement/shape ever needs to be precise.

## Non-default Bevy plugins in use

- `bevy::picking::mesh_picking::MeshPickingPlugin` (wired in `game`) — 3D mesh raycast picking.
  Compiled in by default (`bevy`'s default features enable `mesh_picking`), but not added to the
  `App` by `DefaultPlugins`. Needed for unit selection later; currently just registered so
  click/hover events fire on unit meshes for a future selection system to consume.
- `bevy::dev_tools::diagnostics_overlay::DiagnosticsOverlayPlugin` (needs the `bevy_dev_tools`
  Cargo feature, enabled on `game`'s `bevy` dependency) — in-game, draggable/collapsible
  performance panel. Wired up in `game`'s `diagnostics/overlay.rs`, active in debug builds only,
  showing:
  - **FPS** and **frame time (ms)** — from `FrameTimeDiagnosticsPlugin`.
  - **TPS** — sim ticks/sec, a custom diagnostic counting `FixedUpdate` runs against real time.
    Unlike `Time<Fixed>`'s delta (constant by definition), this drops below the configured rate if
    the sim can't keep up under load.
  - **Entity count** — from `EntityCountDiagnosticsPlugin`.

`game_config` also enables bevy's `serialize` Cargo feature (off by default) so `KeyCode` and
other bevy types implement `serde::{Serialize, Deserialize}`, needed to persist `KeyBindings`.
