use avian3d::prelude::PhysicsSchedule;
use avian3d::PhysicsPlugins;
use bevy::app::{App, Plugin};
use bevy::prelude::Time;
use bevy_tnua::control_helpers::TnuaCrouchEnforcerPlugin;
use bevy_tnua::controller::TnuaControllerPlugin;
use bevy_tnua_avian3d::TnuaAvian3dPlugin;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugins::default())
            .insert_resource(Time::from_hz(144.0))
            .add_plugins(TnuaAvian3dPlugin::new(PhysicsSchedule))
            .add_plugins(TnuaControllerPlugin::new(PhysicsSchedule))
            .add_plugins(TnuaCrouchEnforcerPlugin::new(PhysicsSchedule));
    }
}
