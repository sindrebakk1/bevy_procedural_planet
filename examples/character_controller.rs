use avian3d::{collision::Collider, math::Vector, prelude::*};
use bevy::prelude::*;

use procedural_planet::{
    materials::GlobalMaterialsPlugin,
    plugins::{
        physics::{GravityField, PhysicsPlugin},
        player::{Player, PlayerPlugin},
    },
    Precision,
};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(GlobalMaterialsPlugin)
        .add_plugins(PhysicsPlugin::default())
        .add_plugins(PlayerPlugin)
        .insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.1)))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1000.0,
        })
        .insert_resource(Gravity::ZERO)
        .add_systems(Startup, setup);

    #[cfg(debug_assertions)]
    {
        use procedural_planet::plugins::DebugPlugin;
        app.add_plugins(DebugPlugin::<Precision>::default());
    }

    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let scene_entity = commands
        .spawn((
            Name::new("Scene"),
            Mesh3d(meshes.add(Plane3d::default().mesh().size(128.0, 128.0))),
            MeshMaterial3d(materials.add(StandardMaterial::from_color(Color::srgb(0.3, 0.5, 0.3)))),
            Collider::half_space(Vector::Y),
            RigidBody::Static,
            GravityField::Linear(Vector::Y * -9.81),
        ))
        .id();

    commands.spawn((
        PointLight {
            intensity: 1_000_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 20.0, 0.0),
    ));

    commands
        .spawn_empty()
        .set_parent(scene_entity)
        .insert((Player, Transform::from_xyz(0.0, 2.0, 0.0)));
}
