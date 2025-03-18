use avian3d::math::*;
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use big_space::camera::{CameraController, CameraControllerPlugin};
use big_space::prelude::*;

use procedural_planet::materials::GlobalMaterialsPlugin;
use procedural_planet::plugins::terrain::{Body, BodyPreset};
use procedural_planet::plugins::TerrainPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(WorldInspectorPlugin::default())
        .add_plugins(WireframePlugin)
        .add_plugins((
            BigSpacePlugin::<i64>::default(),
            GlobalMaterialsPlugin,
            TerrainPlugin::<PlayerCamera, 5>::default(),
            CameraControllerPlugin::<i64>::default(),
        ))
        .insert_resource(WireframeConfig {
            global: true,
            default_color: Default::default(),
        })
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 200.0,
        })
        .add_systems(Startup, setup);

    app.run();
}

#[derive(Component, Default)]
struct PlayerCamera;

fn setup(mut commands: Commands) {
    commands.spawn_big_space_default(|root: &mut GridCommands<i64>| {
        root.insert(Name::new("System"));
        root.with_grid_default(|planet| {
            let body_preset = BodyPreset::MOON / 10.0;
            let camera_pos = Vector::Z * (body_preset.radius * 2.0);
            let (camera_cell, camera_translation) = planet.grid().translation_to_grid(camera_pos);
            planet.insert((Body::from_preset(body_preset), Name::new("Planet")));

            planet.spawn_spatial((
                PlayerCamera,
                Camera3d::default(),
                Transform::from_translation(camera_translation),
                camera_cell,
                FloatingOrigin,
                CameraController::default(),
            ));
        });
    });
}
