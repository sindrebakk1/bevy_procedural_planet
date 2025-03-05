pub mod constants;
pub mod keybinds;
pub mod materials;
pub mod plugins;
pub mod state;

#[cfg(debug_assertions)]
pub mod debug;

use avian3d::PhysicsPlugins;
use bevy::app::{App, Plugin, Startup};
use bevy::color::Color;
use bevy::math::Vec3;
use bevy::pbr::AmbientLight;
use bevy::prelude::{AppExtStates, ClearColor, Commands, Component};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

use crate::materials::GlobalMaterialsPlugin;
use crate::plugins::asset_loader::AssetLoaderPlugin;
// use crate::plugins::player::Player;
use crate::constants::physics::MOON_DIAMETER_M;
use crate::plugins::terrain::body::{Body, BodyPreset};
use crate::plugins::terrain::TerrainPlugin;
use crate::state::GameState;

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
                PhysicsPlugins::default(),
                AssetLoaderPlugin,
                PanOrbitCameraPlugin,
                GlobalMaterialsPlugin,
                TerrainPlugin::<OrbitCamera>::default(),
            ))
            .add_systems(Startup, setup);

        #[cfg(debug_assertions)]
        {
            use debug::DebugPlugin;
            app.add_plugins(DebugPlugin);
        }
    }
}

#[derive(Component, Default)]
pub struct OrbitCamera;

fn setup(mut commands: Commands) {
    commands.spawn((
        OrbitCamera,
        PanOrbitCamera {
            focus: Vec3::ZERO,
            radius: Some(MOON_DIAMETER_M),
            zoom_lower_limit: (MOON_DIAMETER_M / 2.0) + 5.0,
            ..Default::default()
        },
    ));
    commands.spawn(Body::from_preset(BodyPreset::MOON));
}
