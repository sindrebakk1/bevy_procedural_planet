pub mod constants;
pub mod keybinds;
pub mod materials;
pub mod math;
pub mod plugins;
pub mod state;

#[cfg(debug_assertions)]
pub mod debug;

use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};

use materials::GlobalMaterialsPlugin;
use plugins::{
    terrain::body::{Body, BodyPreset},
    AssetLoaderPlugin, PhysicsPlugin, PlayerPlugin, TerrainPlugin,
};
use state::GameState;

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
                PhysicsPlugin::default(),
                AssetLoaderPlugin,
                PanOrbitCameraPlugin,
                PlayerPlugin,
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
    let body_preset = BodyPreset::MOON / 10.0;
    commands.spawn((
        OrbitCamera,
        PanOrbitCamera {
            focus: Vec3::ZERO,
            radius: Some(body_preset.radius * 2.0),
            zoom_lower_limit: body_preset.radius + 5.0,
            ..Default::default()
        },
    ));
    commands.spawn(Body::from_preset(body_preset));
}
