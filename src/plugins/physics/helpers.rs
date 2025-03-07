//! The *semi-implicit* or *symplectic* Euler [integration](super) scheme.
//!
//! [Semi-implicit Euler](https://en.wikipedia.org/wiki/Semi-implicit_Euler_method)
//! integration is the most common integration scheme because it is simpler and more
//! efficient than implicit Euler integration, has great energy conservation,
//! and provides much better accuracy than explicit Euler integration.
//!
//! Semi-implicit Euler integration evalutes the acceleration at
//! the current timestep and the velocity at the next timestep:
//!
//! ```text
//! v = v_0 + a * Δt (linear velocity)
//! ω = ω_0 + α * Δt (angular velocity)
//! ```
//!
//! and computes the new position:
//!
//! ```text
//! x = x_0 + v * Δt (position)
//! θ = θ_0 + ω * Δt (rotation)
//! ```
//!
//! This order is opposite to explicit Euler integration, which uses the velocity
//! at the current timestep instead of the next timestep. The explicit approach
//! can lead to bodies gaining energy over time, which is why the semi-implicit
//! approach is typically preferred.

use super::*;
use avian3d::dynamics::integrator::semi_implicit_euler::{
    angular_acceleration, solve_gyroscopic_torque,
};

type AngularValue = Vector;
type TorqueValue = Vector;

/// Integrates velocity based on the given forces in order to find
/// the linear and angular velocity after `delta_seconds` have passed.
///
/// This uses [semi-implicit (symplectic) Euler integration](self).
///
#[allow(clippy::too_many_arguments)]
pub fn integrate_velocity(
    lin_vel: &mut Vector,
    ang_vel: &mut AngularValue,
    force: Vector,
    torque: TorqueValue,
    mass: ComputedMass,
    angular_inertia: &ComputedAngularInertia,
    global_angular_inertia: &GlobalAngularInertia,
    rotation: Rotation,
    locked_axes: LockedAxes,
    gravity: Vector,
    delta_seconds: Scalar,
) {
    // Compute linear acceleration.
    let lin_acc = linear_acceleration(force, mass, locked_axes, gravity);

    // Compute next linear velocity.
    // v = v_0 + a * Δt
    let next_lin_vel = *lin_vel + lin_acc * delta_seconds;
    if next_lin_vel != *lin_vel {
        *lin_vel = next_lin_vel;
    }

    // Compute angular acceleration.
    let ang_acc = angular_acceleration(torque, global_angular_inertia, locked_axes);

    // Compute angular velocity delta.
    // Δω = α * Δt
    let mut delta_ang_vel = ang_acc * delta_seconds;

    let delta_ang_vel_gyro = solve_gyroscopic_torque(
        *ang_vel,
        rotation.0,
        angular_inertia.tensor(),
        delta_seconds,
    );

    delta_ang_vel += apply_locked_axes(delta_ang_vel_gyro, locked_axes);

    if delta_ang_vel != AngularVelocity::ZERO.0 && delta_ang_vel.is_finite() {
        *ang_vel += delta_ang_vel;
    }
}

/// Computes linear acceleration based on the given forces and mass.
pub fn linear_acceleration(
    force: Vector,
    mass: ComputedMass,
    locked_axes: LockedAxes,
    gravity: Vector,
) -> Vector {
    // Effective inverse mass along each axis
    let effective_inverse_mass = apply_locked_axes(Vector::splat(mass.inverse()), locked_axes);

    if effective_inverse_mass != Vector::ZERO && effective_inverse_mass.is_finite() {
        // Newton's 2nd law for translational movement:
        //
        // F = m * a
        // a = F / m
        //
        // where a is the acceleration, F is the force, and m is the mass.
        //
        // `gravity` below is the gravitational acceleration,
        // so it doesn't need to be divided by mass.
        force * effective_inverse_mass + apply_locked_axes(gravity, locked_axes)
    } else {
        Vector::ZERO
    }
}

pub fn apply_locked_axes(mut vec: Vector, locked_axes: LockedAxes) -> Vector {
    if locked_axes.is_rotation_x_locked() {
        vec.x = 0.0;
    }
    if locked_axes.is_rotation_y_locked() {
        vec.y = 0.0;
    }
    if locked_axes.is_rotation_z_locked() {
        vec.z = 0.0;
    }
    vec
}

pub fn apply_locked_axes_to_angular_inertia(
    angular_inertia: impl Into<ComputedAngularInertia>,
    locked_axes: LockedAxes,
) -> ComputedAngularInertia {
    let mut angular_inertia = angular_inertia.into();

    if locked_axes.is_rotation_x_locked() {
        angular_inertia.inverse_tensor_mut().x_axis = Vector::ZERO;
    }
    if locked_axes.is_rotation_y_locked() {
        angular_inertia.inverse_tensor_mut().y_axis = Vector::ZERO;
    }
    if locked_axes.is_rotation_z_locked() {
        angular_inertia.inverse_tensor_mut().z_axis = Vector::ZERO;
    }

    angular_inertia
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use avian3d::dynamics::integrator::semi_implicit_euler::integrate_position;
    use avian3d::math::Quaternion;

    #[test]
    fn physics_extension_helpers() {
        let mut position = Vector::ZERO;
        let mut rotation = Rotation::default();

        let mut linear_velocity = Vector::ZERO;
        let mut angular_velocity = Vector::Z * 2.0;

        let mass = ComputedMass::new(1.0);
        let angular_inertia = ComputedAngularInertia::new(Vector::ONE);

        let gravity = Vector::NEG_Y * 9.81;

        // Step by 100 steps of 0.1 seconds
        for _ in 0..100 {
            integrate_velocity(
                &mut linear_velocity,
                &mut angular_velocity,
                default(),
                default(),
                mass,
                &angular_inertia,
                &GlobalAngularInertia::new(angular_inertia, rotation),
                rotation,
                default(),
                gravity,
                1.0 / 10.0,
            );
            integrate_position(
                &mut position,
                &mut rotation,
                linear_velocity,
                angular_velocity,
                default(),
                1.0 / 10.0,
            );
        }

        // Euler methods have some precision issues, but this seems weirdly inaccurate.
        assert_relative_eq!(position, Vector::NEG_Y * 490.5, epsilon = 10.0);

        assert_relative_eq!(
            rotation.0,
            Quaternion::from_rotation_z(20.0),
            epsilon = 0.01
        );

        assert_relative_eq!(linear_velocity, Vector::NEG_Y * 98.1, epsilon = 0.0001);
        assert_relative_eq!(angular_velocity, Vector::Z * 2.0, epsilon = 0.00001);
    }
}
