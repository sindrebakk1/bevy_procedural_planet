use crate::state::GameState;
use bevy::app::{App, Plugin};
use bevy_asset_loader::loading_state::LoadingStateAppExt;
use bevy_asset_loader::prelude::LoadingState;

pub struct AssetLoaderPlugin;

impl Plugin for AssetLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_loading_state(
            LoadingState::new(GameState::Loading).continue_to_state(GameState::Running),
        );
    }
}
