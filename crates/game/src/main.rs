use bevy::picking::mesh_picking::MeshPickingPlugin;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};

mod diagnostics;

use diagnostics::DevDiagnosticsPlugin;

fn backend_from_args() -> Backends {
    let arg = std::env::args().find(|a| a.starts_with("--backend="));
    match arg.as_deref().map(|a| &a["--backend=".len()..]) {
        Some("vulkan") => Backends::VULKAN,
        Some("dx12") => Backends::DX12,
        Some(other) => {
            eprintln!(
                "Unknown --backend value '{other}', defaulting to dx12 (use 'dx12' or 'vulkan')"
            );
            Backends::DX12
        }
        None => Backends::DX12,
    }
}

fn main() {
    let backends = backend_from_args();

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
                }),
        )
        .add_plugins(MeshPickingPlugin)
        .add_plugins(DevDiagnosticsPlugin)
        .add_plugins((
            game_core::CorePlugin,
            game_sim::SimPlugin,
            game_render::GameRenderPlugin,
            game_ui::GameUiPlugin,
            game_input::GameInputPlugin,
            game_assets::GameAssetsPlugin,
            game_save::SavePlugin,
        ))
        .run();
}
