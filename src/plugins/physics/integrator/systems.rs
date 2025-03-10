use avian3d::{
    dynamics::integrator::semi_implicit_euler::integrate_position,
    math::{AdjustPrecision, Vector},
    position::PreSolveAccumulatedTranslation,
    prelude::{mass_properties::components::GlobalAngularInertia, *},
};
use bevy::{
    ecs::query::QueryData,
    prelude::{Query, Res, Time, Without},
};

use super::helpers::integrate_velocity;
use crate::plugins::physics::{GlobalGravity, LocalGravity};

#[derive(QueryData)]
#[query_data(mutable)]
pub struct VelocityIntegrationQuery {
    rb: &'static RigidBody,
    pos: &'static Position,
    prev_pos: Option<&'static mut PreSolveAccumulatedTranslation>,
    rot: &'static Rotation,
    lin_vel: &'static mut LinearVelocity,
    ang_vel: &'static mut AngularVelocity,
    force: &'static ExternalForce,
    torque: &'static ExternalTorque,
    mass: &'static ComputedMass,
    angular_inertia: &'static ComputedAngularInertia,
    global_angular_inertia: &'static GlobalAngularInertia,
    lin_damping: Option<&'static LinearDamping>,
    ang_damping: Option<&'static AngularDamping>,
    max_linear_speed: Option<&'static MaxLinearSpeed>,
    max_angular_speed: Option<&'static MaxAngularSpeed>,
    local_gravity: Option<&'static LocalGravity>,
    gravity_scale: Option<&'static GravityScale>,
    locked_axes: Option<&'static LockedAxes>,
}

pub type RigidBodyActiveFilter = (Without<RigidBodyDisabled>, Without<Sleeping>);

#[allow(clippy::type_complexity)]
pub fn integrate_velocities(
    mut bodies: Query<VelocityIntegrationQuery, RigidBodyActiveFilter>,
    global_gravity: Res<GlobalGravity>,
    time: Res<Time>,
) {
    let delta_secs = time.delta_secs_f64().adjust_precision();

    bodies.par_iter_mut().for_each(|mut body| {
        if let Some(mut previous_position) = body.prev_pos {
            previous_position.0 = body.pos.0;
        }

        if body.rb.is_static() {
            if *body.lin_vel != LinearVelocity::ZERO {
                *body.lin_vel = LinearVelocity::ZERO;
            }
            if *body.ang_vel != AngularVelocity::ZERO {
                *body.ang_vel = AngularVelocity::ZERO;
            }
            return;
        }

        if body.rb.is_dynamic() {
            let locked_axes = body
                .locked_axes
                .map_or(LockedAxes::default(), |locked_axes| *locked_axes);

            // Apply damping
            if let Some(lin_damping) = body.lin_damping {
                if body.lin_vel.0 != Vector::ZERO && lin_damping.0 != 0.0 {
                    body.lin_vel.0 *= 1.0 / (1.0 + delta_secs * lin_damping.0);
                }
            }
            if let Some(ang_damping) = body.ang_damping {
                if body.ang_vel.0 != AngularVelocity::ZERO.0 && ang_damping.0 != 0.0 {
                    body.ang_vel.0 *= 1.0 / (1.0 + delta_secs * ang_damping.0);
                }
            }

            let external_force = body.force.force();
            let external_torque = body.torque.torque() + body.force.torque();
            let gravity = body.local_gravity.map_or(global_gravity.0, |local| local.0)
                * body.gravity_scale.map_or(1.0, |scale| scale.0);

            integrate_velocity(
                &mut body.lin_vel.0,
                &mut body.ang_vel.0,
                external_force,
                external_torque,
                *body.mass,
                body.angular_inertia,
                body.global_angular_inertia,
                *body.rot,
                locked_axes,
                gravity,
                delta_secs,
            );
        }

        // Clamp velocities
        if let Some(max_linear_speed) = body.max_linear_speed {
            let linear_speed_squared = body.lin_vel.0.length_squared();
            if linear_speed_squared > max_linear_speed.0.powi(2) {
                body.lin_vel.0 *= max_linear_speed.0 / linear_speed_squared.sqrt();
            }
        }
        if let Some(max_angular_speed) = body.max_angular_speed {
            {
                let angular_speed_squared = body.ang_vel.0.length_squared();
                if angular_speed_squared > max_angular_speed.0.powi(2) {
                    body.ang_vel.0 *= max_angular_speed.0 / angular_speed_squared.sqrt();
                }
            }
        }
    });
}

#[allow(clippy::type_complexity)]
pub fn integrate_positions(
    mut bodies: Query<
        (
            &RigidBody,
            &Position,
            Option<&mut PreSolveAccumulatedTranslation>,
            &mut AccumulatedTranslation,
            &mut Rotation,
            &LinearVelocity,
            &AngularVelocity,
            Option<&LockedAxes>,
        ),
        RigidBodyActiveFilter,
    >,
    time: Res<Time>,
) {
    let delta_secs = time.delta_secs_f64().adjust_precision();

    bodies.par_iter_mut().for_each(
        |(
            rb,
            pos,
            pre_solve_accumulated_translation,
            mut accumulated_translation,
            mut rot,
            lin_vel,
            ang_vel,
            locked_axes,
        )| {
            if let Some(mut previous_position) = pre_solve_accumulated_translation {
                previous_position.0 = pos.0;
            }

            if rb.is_static() || (lin_vel.0 == Vector::ZERO && *ang_vel == AngularVelocity::ZERO) {
                return;
            }

            let locked_axes = locked_axes.map_or(LockedAxes::default(), |locked_axes| *locked_axes);

            integrate_position(
                &mut accumulated_translation.0,
                &mut rot,
                lin_vel.0,
                ang_vel.0,
                locked_axes,
                delta_secs,
            );
        },
    );
}
