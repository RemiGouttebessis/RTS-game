# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project status

`rts-game` is a Rust binary crate (edition 2024) using Bevy 0.19.1. `src/main.rs` wires up `DefaultPlugins`
plus three local plugins: `camera` (pan/zoom 3D camera), `map` (ground plane + directional light), and
`units` (unit-related ECS logic). This is still early: extend the existing plugin structure rather than
introducing a new architecture unless the task calls for it.

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
