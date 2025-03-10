use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use crate::keybinds::TOGGLE_WIREFRAME;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WireframeConfig {
            global: true,
            default_color: Color::WHITE.darker(0.4),
        })
        .add_plugins((
            FrameTimeDiagnosticsPlugin,
            LogDiagnosticsPlugin::default(),
            WireframePlugin,
            WorldInspectorPlugin::default(),
        ))
        .add_systems(
            Update,
            toggle_wireframe.run_if(resource_changed::<ButtonInput<KeyCode>>),
        );
    }
}

fn toggle_wireframe(
    mut wireframe_config: ResMut<WireframeConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(TOGGLE_WIREFRAME) {
        wireframe_config.global = !wireframe_config.global;
    }
}
