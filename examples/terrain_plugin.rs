#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use avian3d::math::*;
use avian3d::parry::na::SimdBool;
use bevy::color::palettes::css::{DARK_SEA_GREEN, FOREST_GREEN, INDIAN_RED, OLIVE};
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use bevy::utils::HashSet;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_panorbit_camera::PanOrbitCamera;
use big_space::camera::{CameraController, CameraControllerPlugin};
use big_space::prelude::*;

use procedural_planet::materials::GlobalMaterialsPlugin;
use procedural_planet::plugins::player::controls::grab_ungrab_mouse;
use procedural_planet::plugins::terrain::body::ChunkCache;
use procedural_planet::plugins::terrain::cube_tree::{ChunkHash, CubeTree};
use procedural_planet::plugins::terrain::{Body, BodyPreset, GenerateMeshes, Radius};
use procedural_planet::plugins::terrain::mesh::ChunkMeshBuilder;
use procedural_planet::plugins::TerrainPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins((
            WorldInspectorPlugin::default(),
            WireframePlugin,
            BigSpacePlugin::<i64>::default(),
            FloatingOriginDebugPlugin::<i64>::default(),
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
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                toggle_wireframe.run_if(resource_changed::<ButtonInput<KeyCode>>),
                grab_ungrab_mouse,
            ),
        );

    app.run();
}

#[derive(Component, Default)]
struct PlayerCamera;

fn toggle_wireframe(
    mut wireframe_config: ResMut<WireframeConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::F1) {
        wireframe_config.global = !wireframe_config.global;
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 120_000.,
            shadows_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: 4,
            minimum_distance: 0.1,
            maximum_distance: 10_000.0,
            first_cascade_far_bound: 100.0,
            overlap_proportion: 0.2,
        }
            .build(),
    ));
    let mut camera_pos = Default::default();
    commands.spawn_big_space_default(|root: &mut GridCommands<i64>| {
        root.insert(Name::new("System"));
        root.with_grid_default(|planet| {
            let body_preset = BodyPreset::EARTH;
            camera_pos = Vector::X * body_preset.radius * 1.2;

            planet.insert((Body::from_preset(body_preset), Name::new("Planet")));

            let (camera_cell, camera_translation) =
                planet.grid().translation_to_grid(camera_pos);

            planet.spawn_spatial((
                Name::new("Player"),
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
