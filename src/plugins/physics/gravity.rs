use avian3d::math::Vector;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_tnua::{TnuaToggle};

#[derive(Component)]
pub struct LocalGravity(pub Vector);

pub fn update_local_gravity(
    mut query: Query<(&mut LinearVelocity, &LocalGravity, Option<&TnuaToggle>)>,
) {
    for (mut linear_velocity, gravity, tnua_toggle) in query.iter_mut() {
        match tnua_toggle {
            Some(TnuaToggle::Disabled) => continue,
            _ => linear_velocity.0 += gravity.0,
        };
    }
}
