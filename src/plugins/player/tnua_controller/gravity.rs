use avian3d::prelude::{AngularVelocity, ComputedMass, ExternalForce, ExternalTorque, LinearVelocity};
use avian3d::prelude::mass_properties::components::GlobalAngularInertia;
use bevy::prelude::*;
use bevy_tnua::{TnuaMotor, TnuaToggle};

#[derive(Component)]
pub struct ControllerGravity(pub Vec3);

pub fn apply_gravity(mut query: Query<(
    &mut LinearVelocity,
    &ControllerGravity,
    Option<&TnuaToggle>,
)>) {
    for (
        mut linear_velocity,
        gravity,
        tnua_toggle,
    ) in query.iter_mut()
    {
        match tnua_toggle {
            Some(TnuaToggle::Disabled) => continue,
            _ => linear_velocity.0 += gravity.0,
        };
    }
}