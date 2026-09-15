# rts-game

A Bevy-based RTS/grand strategy game (see `rts-game-design-doc.md` for the design vision and
`rts-bevy-architecture-notes.md` for the architecture this workspace layout follows).

## Layout

A Cargo workspace, one crate per architectural boundary:

```
crates/
├── game/            # thin binary (`rts-game`): builds the App, configures DefaultPlugins,
│                     # wires in the library crates below, plus dev-only diagnostics
├── game_core/        # sim ECS data: components, resources, events — zero rendering deps
├── game_sim/          # deterministic simulation on FixedUpdate: movement, combat,
│                       # pathfinding, orders — ordered via the SimSet system set
├── game_render/        # presentation: camera, map, unit visuals — reads game_core state,
│                        # never the other way around
├── game_ui/              # HUD, selection box, minimap
├── game_input/             # raw input (keyboard, mouse) → semantic actions; key
│                            # bindings live only here, see "Controls" below
├── game_assets/             # asset loading, data-driven unit/building definitions (RON)
├── game_save/                 # save/load
└── game_net/                   # optional multiplayer — not a dependency of `game` yet
assets/                          # game assets (models, textures, audio, ...)
data/                             # RON/JSON data: unit stats, tech trees, etc.
```

`game_core` and `game_sim` have no rendering dependencies — that boundary is what eventually
enables running the sim headless (dedicated server, replays, automated balance testing).
`game_render`/`game_ui` depend on `game_core` to read state, never the reverse.

Everything outside `game_core`'s `Unit` marker, `game_render`'s camera/map/unit-spawning, and
`game_input`'s action translation is still an empty plugin stub — the crate boundaries exist, the
gameplay logic doesn't yet.

## Controls

| Input | Action |
|---|---|
| `W`/`A`/`S`/`D` or arrow keys | Pan camera (north/west/south/east) |
| Mouse scroll | Zoom in/out |
| `F3` | Toggle the performance overlay |

`game_input` owns every binding above (`CameraPanAction`, `CameraZoomAction`,
`ToggleDebugOverlay`). Consumers (`game_render::CameraPlugin`, `game`'s diagnostics overlay) react
to those actions and never read `ButtonInput`/`MouseWheel` directly — rebinding a key means editing
one file (`game_input/src/camera.rs` or `game_input/src/debug.rs`), not hunting through gameplay
code. Input-reading systems run in `game_input::InputSet`; consumers order themselves
`.after(InputSet)` so they see the current frame's input.

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
