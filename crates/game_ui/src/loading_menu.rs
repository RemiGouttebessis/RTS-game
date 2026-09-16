use bevy::prelude::*;
use bevy::tasks::{block_on, poll_once};
use game_core::{GameState, GeneratedWorld, TerrainBuildProgress};

use crate::new_game_menu::PendingStart;
use crate::widgets::{BACKGROUND, TITLE_TEXT, fullscreen_menu_node};

#[derive(Component)]
struct LoadingRoot;

#[derive(Component)]
struct ProgressLabel;

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Loading), spawn_loading_screen)
        .add_systems(
            Update,
            (poll_world_generation, update_progress_label).run_if(in_state(GameState::Loading)),
        )
        .add_systems(OnExit(GameState::Loading), despawn_loading_screen);
}

fn spawn_loading_screen(mut commands: Commands) {
    commands
        .spawn((
            LoadingRoot,
            fullscreen_menu_node(),
            BackgroundColor(BACKGROUND),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Loading..."),
                TextFont::from_font_size(32.0),
                TextColor(TITLE_TEXT),
            ));
            parent.spawn((
                Text::new("Generating world..."),
                ProgressLabel,
                TextFont::from_font_size(18.0),
            ));
        });
}

/// Picks up `new_game_menu`'s background world-generation task once it's
/// done and hands the result to `GeneratedWorld` — from there,
/// `game_render::map` takes over (building terrain chunks a few per frame,
/// and it's what actually sets `GameState::InGame` once they're all built).
fn poll_world_generation(mut pending: ResMut<PendingStart>, mut generated: ResMut<GeneratedWorld>) {
    let Some(task) = pending.0.as_mut() else {
        return;
    };
    let Some(result) = block_on(poll_once(task)) else {
        return;
    };
    pending.0 = None;
    generated.0 = Some(result);
}

/// World generation itself has no sub-progress to report (one
/// `game_worldgen::generate` call, not restructured into stages here), so
/// this only has something more specific than "Generating world..." to say
/// once `game_render::map` has started reporting chunk-building progress.
fn update_progress_label(
    generated: Res<GeneratedWorld>,
    progress: Res<TerrainBuildProgress>,
    mut labels: Query<&mut Text, With<ProgressLabel>>,
) {
    let Ok(mut text) = labels.single_mut() else {
        return;
    };
    text.0 = if generated.0.is_none() {
        "Generating world...".to_string()
    } else if progress.total_chunks > 0 {
        format!(
            "Building terrain: {} / {} chunks",
            progress.built_chunks, progress.total_chunks
        )
    } else {
        "Preparing terrain...".to_string()
    };
}

fn despawn_loading_screen(mut commands: Commands, roots: Query<Entity, With<LoadingRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}
