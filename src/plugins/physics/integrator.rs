use avian3d::dynamics::integrator::semi_implicit_euler::integrate_position;
use avian3d::dynamics::integrator::IntegrationSet;
use avian3d::math::{AdjustPrecision, Vector};
use avian3d::position::PreSolveAccumulatedTranslation;
use avian3d::prelude::mass_properties::components::GlobalAngularInertia;
use avian3d::prelude::*;
use bevy::{
    ecs::{
        intern::Interned,
        query::{QueryData, QueryFilter},
        schedule::ScheduleLabel,
    },
    prelude::*,
};
use super::helpers::{apply_locked_axes, apply_locked_axes_to_angular_inertia, integrate_velocity};
use super::gravity::LocalGravity;

/// Integrates Newton's 2nd law of motion, applying forces and moving entities according to their velocities.
///
/// This acts as a prediction for the next positions and orientations of the bodies. The [solver](dynamics::solver)
/// corrects these predicted positions to take constraints like contacts and joints into account.
///
/// Currently, only the [semi-implicit (symplectic) Euler](helpers) integration scheme
/// is supported. It is the standard for game physics, being simple, efficient, and sufficiently accurate.
///
/// The plugin adds systems in the [`IntegrationSet::Velocity`] and [`IntegrationSet::Position`] system sets.
pub struct CustomIntegratorPlugin {
    schedule: Interned<dyn ScheduleLabel>,
}

impl CustomIntegratorPlugin {
    /// Creates a [`CustomIntegratorPlugin`] with the schedule that is used for running the [`PhysicsSchedule`].
    ///
    /// The default schedule is [`SubstepSchedule`].
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
        }
    }
}

impl Default for CustomIntegratorPlugin {
    fn default() -> Self {
        Self::new(SubstepSchedule)
    }
}

impl Plugin for CustomIntegratorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Gravity>();

        app.configure_sets(
            self.schedule.intern(),
            (IntegrationSet::Velocity, IntegrationSet::Position).chain(),
        );

        app.add_systems(
            self.schedule.intern(),
            (
                integrate_velocities.in_set(IntegrationSet::Velocity),
                integrate_positions.in_set(IntegrationSet::Position),
            ),
        );

        app.add_systems(
            self.schedule.intern(),
            update_global_angular_inertia::<()>
                .in_set(IntegrationSet::Position)
                .after(integrate_positions),
        );

        app.get_schedule_mut(PhysicsSchedule)
            .expect("add PhysicsSchedule first")
            .add_systems(
                (
                    apply_impulses.before(SolverSet::Substep),
                    clear_forces_and_impulses.after(SolverSet::Substep),
                )
                    .in_set(PhysicsStepSet::Solver),
            );
    }
}

#[allow(clippy::type_complexity)]
/// Updates [`GlobalAngularInertia`] for entities that match the given query filter `F`.
fn update_global_angular_inertia<F: QueryFilter>(
    mut query: Populated<
        (
            &Rotation,
            &ComputedAngularInertia,
            &mut GlobalAngularInertia,
        ),
        (Or<(Changed<ComputedAngularInertia>, Changed<Rotation>)>, F),
    >,
) {
    query
        .par_iter_mut()
        .for_each(|(rotation, angular_inertia, mut global_angular_inertia)| {
            global_angular_inertia.update(*angular_inertia, rotation.0);
        });
}

#[derive(QueryData)]
#[query_data(mutable)]
struct VelocityIntegrationQuery {
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

type RigidBodyActiveFilter = (Without<RigidBodyDisabled>, Without<Sleeping>);

#[allow(clippy::type_complexity)]
fn integrate_velocities(
    mut bodies: Query<VelocityIntegrationQuery, RigidBodyActiveFilter>,
    global_gravity: Res<Gravity>,
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
            let gravity = match body.local_gravity {
                Some(local_gravity) => local_gravity.0 * body.gravity_scale.map_or(1.0, |scale| scale.0),
                None => global_gravity.0 * body.gravity_scale.map_or(1.0, |scale| scale.0),
            };

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
fn integrate_positions(
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

type ImpulseQueryComponents = (
    &'static RigidBody,
    &'static mut ExternalImpulse,
    &'static mut ExternalAngularImpulse,
    &'static mut LinearVelocity,
    &'static mut AngularVelocity,
    &'static Rotation,
    &'static ComputedMass,
    &'static GlobalAngularInertia,
    Option<&'static LockedAxes>,
);

fn apply_impulses(mut bodies: Query<ImpulseQueryComponents, RigidBodyActiveFilter>) {
    for (
        rb,
        impulse,
        ang_impulse,
        mut lin_vel,
        mut ang_vel,
        _rotation,
        mass,
        global_angular_inertia,
        locked_axes,
    ) in &mut bodies
    {
        if !rb.is_dynamic() {
            continue;
        }

        let locked_axes = locked_axes.map_or(LockedAxes::default(), |locked_axes| *locked_axes);

        let effective_inv_mass = apply_locked_axes(Vector::splat(mass.inverse()), locked_axes);
        let effective_angular_inertia = apply_locked_axes_to_angular_inertia(*global_angular_inertia, locked_axes);

        // Avoid triggering bevy's change detection unnecessarily.
        let delta_lin_vel = impulse.impulse() * effective_inv_mass;
        let delta_ang_vel = effective_angular_inertia.inverse()
            * (ang_impulse.impulse() + impulse.angular_impulse());

        if delta_lin_vel != Vector::ZERO {
            lin_vel.0 += delta_lin_vel;
        }
        if delta_ang_vel != AngularVelocity::ZERO.0 {
            ang_vel.0 += delta_ang_vel;
        }
    }
}

type ForceComponents = (
    &'static mut ExternalForce,
    &'static mut ExternalTorque,
    &'static mut ExternalImpulse,
    &'static mut ExternalAngularImpulse,
);
type ForceComponentsChanged = Or<(
    Changed<ExternalForce>,
    Changed<ExternalTorque>,
    Changed<ExternalImpulse>,
    Changed<ExternalAngularImpulse>,
)>;

/// Responsible for clearing forces and impulses on bodies.
///
/// Runs in [`PhysicsSchedule`], after [`PhysicsStepSet::SpatialQuery`].
pub fn clear_forces_and_impulses(mut forces: Query<ForceComponents, ForceComponentsChanged>) {
    for (mut force, mut torque, mut impulse, mut angular_impulse) in &mut forces {
        if !force.persistent {
            force.clear();
        }
        if !torque.persistent {
            torque.clear();
        }
        if !impulse.persistent {
            impulse.clear();
        }
        if !angular_impulse.persistent {
            angular_impulse.clear();
        }
    }
}
