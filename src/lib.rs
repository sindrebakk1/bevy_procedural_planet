mod keybinds;
mod materials;
mod plugins;
mod state;
mod utils;

#[cfg(debug_assertions)]
mod debug;
mod physics;

use crate::materials::GlobalMaterialsPlugin;
use crate::plugins::asset_loader::AssetLoaderPlugin;
use crate::plugins::player::Player;
use crate::plugins::terrain::planet::Planet;
use crate::plugins::terrain::TerrainPlugin;
use crate::state::GameState;
use avian3d::PhysicsPlugins;
use bevy::app::{App, Plugin, Startup};
use bevy::color::Color;
use bevy::math::Vec3;
use bevy::pbr::AmbientLight;
use bevy::prelude::{AppExtStates, ClearColor, Commands};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

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
                TerrainPlugin::<Player>::default(),
            ))
            .add_systems(Startup, setup);

        #[cfg(debug_assertions)]
        {
            use debug::DebugPlugin;
            app.add_plugins(DebugPlugin);
        }
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Player,
        PanOrbitCamera {
            focus: Vec3::ZERO,
            radius: Some(2000.0),
            zoom_upper_limit: Some(1_000_000.0),
            zoom_lower_limit: 501.0,
            ..Default::default()
        },
    ));
    commands.spawn(Planet::new(1000.0));
}
