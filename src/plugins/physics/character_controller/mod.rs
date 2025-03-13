use avian3d::{math::Vector, prelude::*, schedule::PhysicsSchedule};
use bevy::{
    ecs::{component::ComponentId, world::DeferredWorld},
    prelude::*,
};
use bevy_tnua::{
    control_helpers::{
        TnuaCrouchEnforcer, TnuaCrouchEnforcerPlugin, TnuaSimpleAirActionsCounter,
        TnuaSimpleFallThroughPlatformsHelper,
    },
    controller::{TnuaController, TnuaControllerPlugin},
    TnuaGhostSensor,
};
use bevy_tnua_avian3d::TnuaAvian3dSensorShape;

pub mod config;
mod overrides;

use overrides::TnuaOverridesPlugin;

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TnuaOverridesPlugin::new(PhysicsSchedule))
            .add_plugins(TnuaControllerPlugin::new(PhysicsSchedule))
            .add_plugins(TnuaCrouchEnforcerPlugin::new(PhysicsSchedule));
    }
}

#[derive(Component, Default, Debug)]
#[require(
    RigidBody(|| RigidBody::Dynamic),
    TnuaController,
    TnuaGhostSensor,
    TnuaSimpleFallThroughPlatformsHelper,
    TnuaSimpleAirActionsCounter,
)]
#[component(on_add = on_add_character_controller)]
pub struct CharacterController;

fn on_add_character_controller(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
    world
        .commands()
        .entity(entity)
        .insert(TnuaCrouchEnforcer::new(0.5 * Vector::Y, |cmd| {
            cmd.insert(TnuaAvian3dSensorShape(Collider::cylinder(0.5, 0.0)));
        }));
}
