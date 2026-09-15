use bevy::prelude::*;

/// Top-level game flow. Gate systems with `run_if(in_state(...))` or
/// `OnEnter`/`OnExit` schedules instead of ad-hoc boolean flags.
#[derive(States, Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
pub enum GameState {
    #[default]
    MainMenu,
    InGame,
}

/// Whether the sim/camera are active or suspended by the ESC menu. Only
/// exists while `GameState::InGame` — toggling it never touches `GameState`,
/// so `OnEnter(GameState::InGame)` world-spawning systems don't re-fire every
/// time the player pauses and resumes.
#[derive(SubStates, Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
#[source(GameState = GameState::InGame)]
pub enum PauseState {
    #[default]
    Running,
    Paused,
}
