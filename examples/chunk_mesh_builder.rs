use avian3d::math::{Scalar, Vector2};
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use procedural_planet::{
    materials::GlobalMaterialsPlugin,
    math::Rectangle,
    plugins::{
        physics::GlobalGravity,
        terrain::{cube_tree::Axis, mesh::ChunkMeshBuilder},
    },
};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
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
        .insert_resource(GlobalGravity::ZERO)
        .add_systems(Startup, setup);

    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const RADIUS: Scalar = 10.0;
    for axis in Axis::ALL {
        let normal = Dir3::from(axis);
        let mesh = ChunkMeshBuilder::<5>::new(axis, RADIUS).build(Rectangle::from_corners(
            Vector2::new(-10.0, -10.0),
            Vector2::new(10.0, 10.0),
        ));
        let mesh_handle = meshes.add(mesh);
        let material_handle = materials.add(StandardMaterial::from_color(Color::srgb_from_array(
            normal.to_array(),
        )));
        commands.spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle),
            Transform::from_translation(normal * RADIUS as f32),
        ));
    }

    commands.spawn((
        PanOrbitCamera {
            radius: Some(20.0),
            ..Default::default()
        },
        Camera::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, -20.0)).looking_to(Dir3::Z, Dir3::Y),
    ));
}
