use avian3d::prelude::Gravity;
use bevy::prelude::*;

use procedural_planet::{
    materials::GlobalMaterialsPlugin,
    plugins::{
        physics::PhysicsPlugin,
        player::{Player, PlayerPlugin},
        terrain::{Body, TerrainPlugin},
    },
};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(GlobalMaterialsPlugin)
        .add_plugins(PhysicsPlugin::default())
        .add_plugins(TerrainPlugin::<Player>::default())
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
        use procedural_planet::debug::DebugPlugin;
        app.add_plugins(DebugPlugin);
    }

    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 100_000.0,
            ..Default::default()
        },
        Transform::default().looking_to(Vec3::new(-1.0, 0.0, -1.0), Dir3::Y),
    ));

    let body = commands.spawn(Body::new(100.0, 1_000_000.0)).id();

    commands
        .spawn((Player, Transform::from_xyz(0.0, 55.0, 0.0)))
        .set_parent(body);
}
