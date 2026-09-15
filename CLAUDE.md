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
  (`WorldGen`: left column of `-`/`+` stepper rows — Preset, Resolution, Continents, Sea level,
  Humidity, Temperature, Nations, Seed/Randomize — middle a live `ImageNode` preview, right a
  color-key legend (`image_export::legend()`); *every* settings change regenerates immediately via
  `generate_image`, no separate Generate button. Picking a **Preset** calls
  `WorldGenSettings::apply_preset_defaults` to overwrite Continents/Sea level/Humidity/Temperature
  with that preset's own values — without this, switching presets kept stale values from whatever
  was selected before, which is why "archipelago" used to not look like an archipelago.
  **Resolution** only changes image detail (width/height in pixels, height always half width);
  it's deliberately decoupled from how much world exists, since `continent_radius` is sampled as a
  fraction of one full loop of the cylinder regardless of pixel count — Continents/Nations control
  "how much world", not Resolution). Separately, `pause_menu` (Escape-toggled `PauseState::Paused`;
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
  parameter-tuning iteration without the game UI. Output is an image either way, not mesh terrain yet
  — that conversion is future work. `Preset` (`preset.rs`) holds the "flavor" knobs (mountain
  strength/belt, river threshold, octaves); `continent_radius`/`sea_level`/`moisture_bias`/
  `temperature_bias` get overridden per-generation from user-facing values
  (`WorldGenSettings::effective_preset` in `worldgen_menu.rs`) rather than being baked into the
  `const` presets. `nations::place` is a separate post-`generate()` step (rejection-sampled land-only
  positions, cylinder-wrap-aware spacing), not part of the pipeline — it only reads the finished
  `World`. Hand-rolled 3D Perlin noise (no `noise` crate) so the cylinder's seamless-X-wrap sampling
  (`noise::cylinder_point`) is under full control; `image` is a real dependency (both here and in
  `game_ui`, to build/read `RgbImage`). `stats::count_landmasses` flood-fills the actual generated
  terrain (cylinder-wrap-aware via `Grid::neighbors`) to report the *real* landmass count, since the
  `Continents` input is only an expected blob count for the noise field — sea level and randomness
  can split or merge blobs, so target and actual often differ (`worldgen_menu` shows both; the CLI
  prints the actual count too). See "World generation" in `README.md` before retuning presets — the
  radius-to-feature-count relationship is easy to misjudge by an order of magnitude (it already was,
  once).

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
