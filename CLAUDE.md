# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project status

`rts-game` is a Rust Cargo workspace (edition 2024) using Bevy 0.19.1, following the crate layout in
`rts-bevy-architecture-notes.md`. The `game` crate (`crates/game`, binary name `rts-game`) is a thin
binary: it builds `DefaultPlugins`, adds `MeshPickingPlugin` (compiled in by Bevy's default features but
not added by `DefaultPlugins`) and a debug-only `DevDiagnosticsPlugin` (in-game perf overlay: FPS, frame
time, sim tick rate, entity count — see `crates/game/src/diagnostics/overlay.rs`), then wires in the
library crates:

- `game_core` — sim ECS data (components/resources/events), zero rendering deps. Currently just the `Unit`
  marker component.
- `game_sim` — deterministic simulation on `FixedUpdate`: `MovementPlugin`, `CombatPlugin`,
  `PathfindingPlugin`, `OrdersPlugin`, ordered via the `SimSet` system set. All currently empty stubs — no
  sim logic has been implemented yet.
- `game_render` — presentation: `CameraPlugin` (pan/zoom 3D camera, driven by `game_input` actions),
  `MapPlugin` (ground plane + light), `UnitsPlugin` (currently spawns unit visuals directly as a
  placeholder; will move to reacting to `game_sim`-spawned units once `game_sim` owns spawning).
- `game_input` — translates raw keyboard/mouse into semantic actions (`CameraPanAction`,
  `CameraZoomAction`, `ToggleDebugOverlay`), ordered via the `InputSet` system set. Key/mouse
  bindings live only here; consumers react to actions, never read `ButtonInput`/`MouseWheel`
  directly. See "Controls" in `README.md`.
- `game_ui`, `game_assets`, `game_save` — empty plugin stubs, wired into `game` but with no systems
  yet.
- `game_net` — optional networking, exists in the workspace but is **not** a dependency of `game` yet
  (add it once an authority model — lockstep vs. server-authoritative — is decided).

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
