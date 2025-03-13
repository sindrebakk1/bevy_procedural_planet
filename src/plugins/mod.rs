pub mod asset_loader;
pub mod physics;
pub mod player;
pub mod terrain;

#[cfg(debug_assertions)]
pub mod debug;

pub use {
    asset_loader::AssetLoaderPlugin, physics::PhysicsPlugin, player::PlayerPlugin,
    terrain::TerrainPlugin,
};

#[cfg(debug_assertions)]
pub use debug::DebugPlugin;
