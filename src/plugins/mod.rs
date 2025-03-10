pub mod asset_loader;
pub mod physics;
pub mod player;
pub mod terrain;

pub use {
    asset_loader::AssetLoaderPlugin, physics::PhysicsPlugin, player::PlayerPlugin,
    terrain::TerrainPlugin,
};
