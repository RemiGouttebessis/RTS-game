use bevy::prelude::*;
use bevy::render::settings::Backends;
use serde::{Deserialize, Serialize};

/// Rendering backend choice. A small config-friendly stand-in for wgpu's
/// `Backends` bitflags (not `Serialize`), covering the two backends this
/// project cares about — see CLAUDE.md's environment notes for why (Vulkan
/// flickers on this dev machine's GPU; DX12 doesn't).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Backend {
    #[default]
    Dx12,
    Vulkan,
}

impl From<Backend> for Backends {
    fn from(backend: Backend) -> Self {
        match backend {
            Backend::Dx12 => Backends::DX12,
            Backend::Vulkan => Backends::VULKAN,
        }
    }
}

#[derive(Resource, Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct GraphicsSettings {
    pub backend: Backend,
}
