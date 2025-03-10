use avian3d::{
    dynamics::integrator::IntegrationSet, math::Vector,
    prelude::mass_properties::components::GlobalAngularInertia, prelude::*,
};
use bevy::{
    ecs::{intern::Interned, query::QueryFilter, schedule::ScheduleLabel},
    prelude::*,
};

mod helpers;
mod systems;

use helpers::{apply_locked_axes, apply_locked_axes_to_angular_inertia};
use systems::{integrate_positions, integrate_velocities, RigidBodyActiveFilter};

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
        let effective_angular_inertia =
            apply_locked_axes_to_angular_inertia(*global_angular_inertia, locked_axes);

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
