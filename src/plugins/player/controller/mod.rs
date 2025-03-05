pub mod config;
pub mod controls;

use avian3d::prelude::{Collider, LockedAxes, RigidBody};
use avian3d::schedule::{PhysicsSchedule, PhysicsSet};
use bevy::app::{App, Plugin, PostUpdate, Update};
use bevy::ecs::component::ComponentId;
use bevy::ecs::world::DeferredWorld;
use bevy::math::Vec3;
use bevy::prelude::{Component, Entity, IntoSystemConfigs};
use bevy::transform::TransformSystem;
use bevy_tnua::control_helpers::{
    TnuaCrouchEnforcer, TnuaSimpleAirActionsCounter, TnuaSimpleFallThroughPlatformsHelper,
};
use bevy_tnua::controller::TnuaController;
use bevy_tnua::{TnuaGhostSensor, TnuaUserControlsSystemSet};
use bevy_tnua_avian3d::TnuaAvian3dSensorShape;
use controls::{
    apply_camera_controls, apply_character_controls, grab_ungrab_mouse, ForwardFromCamera,
};

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, grab_ungrab_mouse)
            .add_systems(
                PostUpdate,
                apply_camera_controls
                    .after(PhysicsSet::Sync)
                    .before(TransformSystem::TransformPropagate),
            )
            .add_systems(
                PhysicsSchedule,
                apply_character_controls.in_set(TnuaUserControlsSystemSet),
            );
    }
}

#[derive(Component, Default, Debug)]
#[require(
    RigidBody(|| RigidBody::Dynamic),
    LockedAxes(|| LockedAxes::new().lock_rotation_x().lock_rotation_z()),
    TnuaController,
    ForwardFromCamera,
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
        .insert(TnuaCrouchEnforcer::new(0.5 * Vec3::Y, |cmd| {
            cmd.insert(TnuaAvian3dSensorShape(Collider::cylinder(0.5, 0.0)));
        }));
}
