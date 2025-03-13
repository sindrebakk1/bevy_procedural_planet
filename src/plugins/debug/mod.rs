use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use big_space::{debug::FloatingOriginDebugPlugin, precision::GridPrecision};
use std::marker::PhantomData;

use crate::keybinds::TOGGLE_WIREFRAME;

#[derive(Default)]
pub struct DebugPlugin<P: GridPrecision> {
    _marker: PhantomData<P>,
}

impl<P: GridPrecision> Plugin for DebugPlugin<P> {
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
            FloatingOriginDebugPlugin::<P>::default(),
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
