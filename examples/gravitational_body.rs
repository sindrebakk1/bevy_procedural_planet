use avian3d::math::Vector;
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use big_space::prelude::*;

use procedural_planet::{
    materials::GlobalMaterialsPlugin,
    plugins::{
        physics::{GlobalGravity, PhysicsPlugin},
        player::{Player, PlayerPlugin},
        terrain::{Body, BodyPreset, TerrainPlugin},
    },
    Precision,
};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(GlobalMaterialsPlugin)
        .add_plugins(PhysicsPlugin::default())
        .add_plugins(BigSpacePlugin::<Precision>::default())
        .add_plugins(TerrainPlugin::<Player>::default())
        .add_plugins(PlayerPlugin)
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 200.0,
        })
        .insert_resource(GlobalGravity::ZERO)
        .add_systems(Startup, setup);

    #[cfg(debug_assertions)]
    {
        use procedural_planet::plugins::debug::DebugPlugin;
        app.add_plugins(DebugPlugin::<Precision>::default());
    }

    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        // PrimaryLight,
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

    commands.spawn_big_space_default(|root: &mut GridCommands<Precision>| {
        root.with_grid_default(|planet| {
            let body = Body::from_preset(BodyPreset::MOON);
            let player_pos = Vector::Y * (body.radius + 20.0);
            let (player_cell, player_pos) = planet.grid().translation_to_grid(player_pos);
            planet.insert((body, Name::new("Planet")));
            planet.spawn_spatial((Player, Transform::from_translation(player_pos), player_cell));
        });
    });
}
