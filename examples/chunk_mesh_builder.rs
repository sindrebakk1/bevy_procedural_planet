#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

use avian3d::math::{Scalar, Vector2};
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use procedural_planet::{
    materials::GlobalMaterialsPlugin,
    math::Rectangle,
    plugins::{
        terrain::{cube_tree::Axis, mesh::ChunkMeshBuilder},
    },
};
use procedural_planet::math::quad_tree::Quadrant;
use procedural_planet::plugins::terrain::cube_tree::ChunkData;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(WorldInspectorPlugin::default())
        .add_plugins(GlobalMaterialsPlugin)
        .add_plugins(WireframePlugin)
        .add_plugins(PanOrbitCameraPlugin)
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

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const RADIUS: Scalar = 10.0;
    let mes_builder = ChunkMeshBuilder::<5>::new(RADIUS);
    for axis in Axis::ALL {
        let bounds = Rectangle::from_corners(Vector2::new(-10.0, -10.0), Vector2::new(10.0, 10.0));
        let mesh = mes_builder.build(
            &bounds,
            &ChunkData::new(axis, Quadrant::ROOT, false, 0, 10.0, &bounds)
        );
        let mesh_handle = meshes.add(mesh);
        let material_handle = materials.add(StandardMaterial::from_color(Color::srgb_from_array(
            axis.to_array_f32(),
        )));
        commands.spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle),
            Transform::from_translation(axis * 10.0f32),
        ));
    }

    commands.spawn(PanOrbitCamera {
        radius: Some(20.0),
        ..Default::default()
    });
}
