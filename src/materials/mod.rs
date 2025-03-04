use bevy::app::{App, Plugin};

#[cfg(debug_assertions)]
pub mod debug;

pub struct GlobalMaterialsPlugin;

impl Plugin for GlobalMaterialsPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(debug_assertions)]
        {
            use crate::materials::debug::DebugMaterialsPlugin;
            app.add_plugins(DebugMaterialsPlugin);
        }
    }
}
