use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource, Serialize, Deserialize, Clone, Copy, Debug)]
pub struct CameraSettings {
    pub pan_speed: f32,
    pub zoom_speed: f32,
    pub min_height: f32,
    pub max_height: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            pan_speed: 20.0,
            zoom_speed: 10.0,
            min_height: 5.0,
            max_height: 60.0,
        }
    }
}
