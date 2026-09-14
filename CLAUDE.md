# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project status

`rts-game` is a fresh Rust binary crate (edition 2024) with no dependencies yet. `src/main.rs` currently
contains only the Rust Book's "guessing game" tutorial stub, not real RTS logic — there is no established
architecture to preserve yet. When implementing features, you are largely starting from scratch: make
reasonable structural choices (module layout, crate selection) rather than assuming existing conventions.

## Commands

- Build: `cargo build`
- Run: `cargo run`
- Check (fast compile-check without producing a binary): `cargo check`
- Test: `cargo test`
- Run a single test: `cargo test <test_name>`
- Format: `cargo fmt`
- Lint: `cargo clippy`
