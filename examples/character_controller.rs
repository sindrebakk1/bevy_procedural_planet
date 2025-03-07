use avian3d::collision::Collider;
use avian3d::prelude::RigidBody;
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use procedural_planet::plugins::physics::{LocalGravity, PhysicsPlugin};
use procedural_planet::plugins::player::tnua_controller::controls::ControllerCamera;
use procedural_planet::plugins::player::{Player, PlayerPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WorldInspectorPlugin::default())
        .insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.1)))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1000.0,
        })
        .add_plugins(PhysicsPlugin::default())
        .add_plugins(PlayerPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("Scene"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(128.0, 128.0))),
        MeshMaterial3d(materials.add(StandardMaterial::from_color(Color::srgb(0.3, 0.5, 0.3)))),
        Collider::half_space(Vec3::Y),
        RigidBody::Static,
    ));

    commands.spawn((
        Player,
        Transform::from_xyz(0.0, 2.0, 0.0),
        LocalGravity(Vec3::ZERO * -9.81),
    ));

    commands.spawn((
        PointLight {
            intensity: 1_000_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(2.0, 8.0, 2.0),
    ));

    commands.spawn((
        ControllerCamera,
        Camera3d::default(),
        Transform::from_xyz(-5.0, 3.5, 5.5).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
