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
├── game_ui/                   # main menu, ESC/pause menu, keybind-rebind screen today; HUD,
│                               # selection box, minimap land here too, eventually
├── game_input/                  # raw input (keyboard, mouse) → semantic actions; key
│                                 # bindings live only here, see "Controls" below
├── game_assets/                   # asset loading, data-driven unit/building definitions (RON)
├── game_save/                       # save/load
└── game_net/                          # optional multiplayer — not a dependency of `game` yet
assets/                                 # game assets (models, textures, audio, ...) — currently
                                         # placeholders (colored meshes, default font), real
                                         # assets TBD
data/                                    # RON/JSON data: unit stats, tech trees, etc.
```

`game_core` and `game_sim` have no rendering dependencies — that boundary is what eventually
enables running the sim headless (dedicated server, replays, automated balance testing).
`game_render`/`game_ui` depend on `game_core` to read state, never the reverse.

Everything outside `game_core`'s `Unit`/state types, `game_render`'s camera/map/unit-spawning,
`game_input`'s action translation, `game_config`'s settings, and `game_ui`'s menus is still an
empty plugin stub — the crate boundaries exist, the gameplay logic doesn't yet.

## Game state

Three nested states (`#[derive(States)]`/`#[derive(SubStates)]`) drive UI/world flow:

- `GameState` (`game_core`) — `MainMenu` (default) → `InGame`. `game_render`'s map/unit spawning
  runs on `OnEnter`/`OnExit(GameState::InGame)` instead of `Startup` (spawns once on entry,
  despawns on exit so "Quit to Main Menu" → "Play" again doesn't duplicate the world); camera
  pan/zoom is gated `run_if(in_state(GameState::InGame))`. The 3D camera itself still spawns at
  `Startup` unconditionally — `bevy_ui` needs a camera present to render menus onto.
- `PauseState` (`game_core`, sub-state of `GameState::InGame`) — `Running`/`Paused`, toggled by
  Escape. A sub-state (not a third `GameState` variant) specifically so pausing/resuming never
  re-triggers `OnEnter(GameState::InGame)` and re-spawns the world.
- `PauseMenuScreen` (`game_ui::pause_menu`, private, sub-state of `PauseState::Paused`) —
  `Root`/`Keybinds`, which pause-menu screen is showing. Resets to `Root` every time the menu
  opens.

`game_ui::main_menu`, `pause_menu`, and `keybinds_menu` each spawn/despawn their own UI tree on
their state's `OnEnter`/`OnExit` — no leftover entities across transitions.

Escape is context-sensitive (`pause_menu::handle_escape`): cancel an in-progress keybind capture,
else back out of the keybinds screen to the pause root, else open/close the pause menu.

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
