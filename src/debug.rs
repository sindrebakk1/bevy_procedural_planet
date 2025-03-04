use crate::keybinds::TOGGLE_WIREFRAME;
use bevy::app::{App, Plugin, Update};
use bevy::color::{Color, Luminance};
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::input::ButtonInput;
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::prelude::{resource_changed, IntoSystemConfigs, KeyCode, Res, ResMut};
use bevy_inspector_egui::quick::WorldInspectorPlugin;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WireframeConfig {
            global: true,
            default_color: Color::WHITE.darker(0.2),
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
