use bevy::asset::AssetPlugin;
use bevy::picking::mesh_picking::MeshPickingPlugin;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use game_config::Backend;

mod diagnostics;

use diagnostics::DevDiagnosticsPlugin;

/// `--backend=dx12`/`--backend=vulkan` overrides the persisted setting for
/// this run only (not saved) — handy for quickly A/B-testing a backend
/// without touching the settings file.
fn backend_override_from_args() -> Option<Backend> {
    let arg = std::env::args().find(|a| a.starts_with("--backend="));
    match arg.as_deref().map(|a| &a["--backend=".len()..]) {
        Some("vulkan") => Some(Backend::Vulkan),
        Some("dx12") => Some(Backend::Dx12),
        Some(other) => {
            eprintln!("Unknown --backend value '{other}', ignoring (use 'dx12' or 'vulkan')");
            None
        }
        None => None,
    }
}

fn main() {
    let mut settings = game_config::load();
    if let Some(backend) = backend_override_from_args() {
        settings.graphics.backend = backend;
    }
    let backends: Backends = settings.graphics.backend.into();

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "RTS Game".into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                        backends: Some(backends),
                        ..default()
                    })),
                    ..default()
                })
                .set(AssetPlugin {
                    // Bevy's own asset base path is `CARGO_MANIFEST_DIR` (set
                    // at compile time to *this* crate's own directory,
                    // `crates/game`) in dev builds, not the process's current
                    // working directory — unlike `game_assets`'s plain
                    // `std::fs::read_to_string("assets/civilizations.json")`,
                    // which resolves relative to whatever directory `cargo
                    // run` was invoked from (the workspace root, by
                    // convention here) and so "just worked" by coincidence.
                    // Bevy's `AssetServer` needs an explicit relative path
                    // back up to the one real `assets/` folder at the
                    // workspace root instead, or every `asset_server.load(..)`
                    // 404s looking in `crates/game/assets/` instead.
                    file_path: "../../assets".to_string(),
                    ..default()
                }),
        )
        .add_plugins(MeshPickingPlugin)
        .add_plugins(DevDiagnosticsPlugin)
        .add_plugins(game_config::GameConfigPlugin { settings })
        .add_plugins((
            game_core::CorePlugin,
            game_input::GameInputPlugin,
            game_sim::SimPlugin,
            game_render::GameRenderPlugin,
            game_ui::GameUiPlugin,
            game_assets::GameAssetsPlugin,
            game_save::SavePlugin,
        ))
        .run();
}
