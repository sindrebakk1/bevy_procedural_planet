#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

pub mod constants;
pub mod keybinds;
pub mod materials;
pub mod math;
pub mod plugins;
pub mod state;

use avian3d::math::Vector;
use bevy::prelude::*;
use big_space::camera::CameraController;
use big_space::{camera::CameraControllerPlugin, prelude::*};

use materials::GlobalMaterialsPlugin;
use plugins::{
    terrain::body::{Body, BodyPreset},
    AssetLoaderPlugin, PhysicsPlugin, PlayerPlugin, TerrainPlugin,
};
use state::GameState;
use constants::terrain::CHUNK_SUBDIVISIONS;

#[cfg(feature = "f64")]
pub type Precision = i64;

#[cfg(not(feature = "f64"))]
pub type Precision = i32;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .insert_resource(ClearColor(Color::linear_rgb(0.1, 0.1, 0.1)))
            .insert_resource(AmbientLight {
                color: Color::WHITE,
                brightness: 1000.0,
            })
            .add_plugins((
                BigSpacePlugin::<Precision>::default(),
                PhysicsPlugin::default(),
                AssetLoaderPlugin,
                PlayerPlugin,
                GlobalMaterialsPlugin,
                TerrainPlugin::<OrbitCamera, CHUNK_SUBDIVISIONS>::default(),
                CameraControllerPlugin::<Precision>::default(),
            ))
            .add_systems(Startup, setup);

        #[cfg(debug_assertions)]
        {
            use plugins::DebugPlugin;
            app.add_plugins(DebugPlugin::<Precision>::default());
        }
    }
}

#[derive(Component, Default)]
pub struct OrbitCamera;

fn setup(mut commands: Commands) {
    commands.spawn_big_space_default(|root: &mut GridCommands<Precision>| {
        root.with_grid_default(|planet| {
            let body = Body::from_preset(BodyPreset::EARTH);
            let camera_pos = Vector::Y * (body.radius + 10.0);
            let (camera_cell, camera_translation) = planet.grid().translation_to_grid(camera_pos);
            planet.insert((body, Name::new("Planet")));

            planet.spawn_spatial((
                Camera3d::default(),
                Transform::from_translation(camera_translation),
                camera_cell,
                FloatingOrigin,
                CameraController::default(),
            ));
        });
    });
}
