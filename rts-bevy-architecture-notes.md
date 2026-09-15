# RTS Game in Rust + Bevy — Architecture Notes

## Bevy Version
- Bevy's API moves fast (relations, required components, observers in 0.14-0.15+).
- Pin to a released stable version; upgrade deliberately rather than tracking `main`, especially for a long-lived project.

## Core Architecture Patterns for RTS

- **Selection & commands as data, not direct calls.** Model selection as a resource/marker component (`Selected`). Issue commands (`MoveCommand`, `AttackCommand`) as components or events rather than mutating state directly from input systems. Keeps input, AI, and replay/networking decoupled.
- **Separate simulation from presentation.** Sim state (positions, health, orders) should be deterministic and independent of rendering. Run sim on `FixedUpdate`, interpolate visuals in `Update`. Pays off for replays, lockstep multiplayer, and debugging.
- **Spatial partitioning early.** Use a grid or quadtree for selection boxes, targeting, collision avoidance, fog of war — avoid O(n²) distance checks.
- **Pathfinding as its own system/plugin**, decoupled from movement execution. Flow fields work well for many units moving to a shared destination (cheaper than per-unit A* at scale).
- **Command pattern for player input** if replay/multiplayer matters — record input events with tick numbers instead of mutating world state directly.
- **Keep components small and orthogonal** (`Health`, `MoveTarget`, `AttackTarget`, `Selectable`, `Owner`) rather than one giant `Unit` component. Keeps queries cheap and avoids borrow-checker conflicts across overlapping systems — this is the actual ECS payoff for an RTS with hundreds of interacting entities.

## Useful Crates

| Purpose | Crate |
|---|---|
| Input abstraction | `leafwing-input-manager` |
| Unit AI / behavior trees | `big_brain` (or custom) |
| Debug UI / prototyping | `bevy_egui`; for shipping UI, `bevy_ui` directly or `sickle_ui` |
| Save games / data-driven defs | `serde` + `ron` or `bincode` |
| Multiplayer networking | `bevy_replicon` or `lightyear` (server-authoritative or rollback models) |
| Audio | `bevy_kira_audio` (Bevy's built-in audio is fairly basic) |

## Steam-Specific Notes

- `steamworks` crate for achievements, cloud saves, workshop support. Requires the Steamworks SDK and a Steam App ID (Steamworks account, $100 fee, some lead time — start early).
- Bevy compiles cleanly to a standalone native executable, which Steam wants — not a big blocker, but test the packaging pipeline early rather than at the end.

## Project Structure — Cargo Workspace

Recommended: a **workspace with multiple crates**, not a single monolithic binary. Costs setup time up front, pays off in incremental compile times, enforced boundaries, and easier multiplayer/replay/tooling additions later.

```
rts_game/
├── Cargo.toml                 # workspace root
├── crates/
│   ├── game/                  # thin binary: sets up App, adds plugins
│   ├── game_core/             # sim: components, resources, events (no rendering deps)
│   ├── game_sim/              # sim systems: movement, combat, pathfinding, AI
│   ├── game_render/            # presentation: sprites, camera, interpolation, VFX
│   ├── game_ui/                # bevy_ui / HUD, selection box, minimap
│   ├── game_input/             # input → command translation (leafwing-input-manager)
│   ├── game_net/                # optional: networking (replicon/lightyear)
│   ├── game_assets/             # asset loading, data-driven unit/building defs (RON)
│   └── game_save/               # save/load, serde
├── assets/
└── data/                        # RON/JSON unit stats, tech trees, etc.
```

**Key boundary:** `game_core` and `game_sim` have zero dependency on rendering — they only know ECS data and simulation logic. `game_render` and `game_ui` depend on `game_core` to read state, never the reverse. This is what enables running headless later (dedicated server, automated balance testing, replays) without dragging in a renderer.

### Plugin Structure

Each crate typically exposes one or a few `Plugin`s:

```rust
// game_sim/src/lib.rs
pub struct SimPlugin;
impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MovementPlugin,
            CombatPlugin,
            PathfindingPlugin,
            OrdersPlugin,
        ))
        .configure_sets(FixedUpdate, (
            SimSet::Input,
            SimSet::Orders,
            SimSet::Movement,
            SimSet::Combat,
            SimSet::Cleanup,
        ).chain());
    }
}
```

Use **system sets with explicit ordering** (`configure_sets`) rather than relying on insertion order — real ordering dependencies exist (orders before movement, combat resolution before death cleanup) and implicit-order bugs are hard to track down at scale (e.g. 200 units).

### Data Flow for the Sim

- **Fixed timestep for simulation** (`FixedUpdate`), variable/interpolated rendering (`Update`). Store previous + current transform, interpolate in a render system.
- **Commands as events or marker components**, not direct mutation — e.g. `MoveOrder { entity, target }` as an event consumed by `OrdersPlugin`, which sets a `MoveTarget` component that `MovementPlugin` reads. This indirection lets replay/networking/AI all issue orders through the same pipeline.
- **Data-driven unit definitions from day one.** Define stats, costs, build times in RON files (via `bevy_common_assets` or a custom asset loader), deserialized into a `UnitDef` resource/collection. Avoid hardcoding stats in Rust structs — balance tuning without recompiling matters a lot in practice.

### State Management

Use Bevy's `States` for high-level flow (`MainMenu`, `Loading`, `InGame`, `Paused`), and `SubStates` for phases within a state (e.g. placement mode vs. normal play). Keeps system scheduling (`run_if(in_state(...))`) clean instead of manual flag-checking everywhere.

### Practical Notes

- Keep the `game` binary crate thin — just `App::new().add_plugins(...)`. All logic in library crates so `game_sim` can be unit tested without a window/renderer, and tools (map editor, balance simulator) can reuse the sim crate directly.
- Don't over-split `game_core` at first (e.g. separate crates per unit type) — split further only when compile times or team boundaries actually demand it.
- If multiplayer is even a maybe, decide the authority model (lockstep deterministic sim vs. server-authoritative with snapshots) **before** building combat/pathfinding — it changes how much determinism the sim needs (fixed-point math vs. floats). Retrofitting determinism later is painful.

---

## Open Threads / Possible Next Steps
- Sketch actual `SimSet` ordering and event flow for a minimal move+attack loop.
- Data-driven unit definitions: RON schema + loader design.
- Networking architecture deep dive (lockstep vs. server-authoritative).
- Pathfinding / flow field implementation details.
