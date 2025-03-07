use crate::constants::physics::EARTH_GRAVITATIONAL_ACCELERATION;
use bevy::prelude::*;
use std::ops::{Add, AddAssign};

/// Allows disabling  for a specific entity.
///
/// This can be used to let some other system  temporarily take control over a character.
///
/// This component is not mandatory - if omitted,  will just assume it is enabled for that
/// entity.
#[derive(Component, Default, Debug, PartialEq, Eq, Clone, Copy)]
pub enum ControllerToggle {
    /// Do not update the sensors, and do not apply forces from the motor.
    ///
    /// The controller system will also not run and won't update the motor components not the state
    /// stored in the `Controller` component. They will retain their last value from before
    /// `Toggle::Disabled` was set.
    Disabled,
    /// Update the sensors, but do not apply forces from the motor.
    ///
    /// The platformer controller system will still run and still update the motor components and
    /// state stored in the `Controller` component. only the system that applies the motor
    /// forces will be disabled.
    SenseOnly,
    #[default]
    /// The backend behaves normally - it updates the sensors and applies forces from the motor.
    Enabled,
}

/// Newtonian state of the rigid body.
///
///  takes the position and rotation of the rigid body from its `GlobalTransform`, but things
/// like velocity are dependent on the physics engine. The physics backend is responsible for
/// updating this component from the physics engine during
/// [`PipelineStages::Sensors`](crate::PipelineStages::Sensors).
#[derive(Component, Debug)]
pub struct RigidBodyTracker {
    pub translation: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
    /// Angular velocity as the rotation axis multiplied by the rotation speed in radians per
    /// second. Can be extracted from a quaternion using [`Quat::xyz`].
    pub angvel: Vec3,
    pub gravity: Vec3,
}

impl Default for RigidBodyTracker {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            velocity: Vec3::ZERO,
            angvel: Vec3::ZERO,
            gravity: Vec3::ZERO,
        }
    }
}

/// Distance from another collider in a certain direction, and information on that collider.
///
/// The physics backend is responsible for updating this component from the physics engine during
/// [`PipelineStages::Sensors`](crate::PipelineStages::Sensors), usually by casting a ray
/// or a shape in the `cast_direction`.
#[derive(Component, Debug)]
pub struct ProximitySensor {
    /// The cast origin in the entity's coord system.
    pub cast_origin: Vec3,
    /// The direction in world coord system (unmodified by the entity's transform)
    pub cast_direction: Dir3,
    ///  will update this field according to its need. The backend only needs to read it.
    pub cast_range: f32,
    pub output: Option<ProximitySensorOutput>,

    /// Used to prevent collision with obstacles the character squeezed into sideways.
    ///
    /// This is used to prevent <https://github.com/idanarye/bevy-/issues/14>. When casting,
    ///  checks if the entity the ray(/shape)cast hits is also in contact with the owner
    /// collider. If so,  compares the contact normal with the cast direction.
    ///
    /// For legitimate hits, these two directions should be opposite. If  casts downwards and
    /// hits the actual floor, the normal of the contact with it should point upward. Opposite
    /// directions means the dot product is closer to `-1.0`.
    ///
    /// Illigitimage hits hits would have perpendicular directions - hitting a wall (sideways) when
    /// casting downwards - which should give a dot product closer to `0.0`.
    ///
    /// This field is compared to the dot product to determine if the hit is valid or not, and can
    /// usually be left at the default value of `-0.5`.
    ///
    /// Positive dot products should not happen (hitting the ceiling?), but it's trivial to
    /// consider them as invalid.
    pub intersection_match_prevention_cutoff: f32,
}

impl Default for ProximitySensor {
    fn default() -> Self {
        Self {
            cast_origin: Vec3::ZERO,
            cast_direction: Dir3::NEG_Y,
            cast_range: 0.0,
            output: None,
            intersection_match_prevention_cutoff: -0.5,
        }
    }
}

/// Information from [`ProximitySensor`] that have detected another collider.
#[derive(Debug, Clone)]
pub struct ProximitySensorOutput {
    /// The entity of the collider detected by the ray.
    pub entity: Entity,
    /// The distance to the collider from [`cast_origin`](ProximitySensor::cast_origin) along the
    /// [`cast_direction`](ProximitySensor::cast_direction).
    pub proximity: f32,
    /// The normal from the detected collider's surface where the ray hits.
    pub normal: Dir3,
    /// The velocity of the detected entity,
    pub entity_linvel: Vec3,
    /// The angular velocity of the detected entity, given as the rotation axis multiplied by the
    /// rotation speed in radians per second. Can be extracted from a quaternion using
    /// [`Quat::xyz`].
    pub entity_angvel: Vec3,
}

/// Represents a change to velocity (linear or angular)
#[derive(Debug, Clone)]
pub struct VelChange {
    // The part of the velocity change that gets multiplied by the frame duration.
    //
    // In Rapier, this is applied using `ExternalForce` so that the simulation will apply in
    // smoothly over time and won't be sensitive to frame rate.
    pub acceleration: Vec3,
    // The part of the velocity change that gets added to the velocity as-is.
    //
    // In Rapier, this is added directly to the `Velocity` component.
    pub boost: Vec3,
}

impl VelChange {
    pub const ZERO: Self = Self {
        acceleration: Vec3::ZERO,
        boost: Vec3::ZERO,
    };

    pub fn acceleration(acceleration: Vec3) -> Self {
        Self {
            acceleration,
            boost: Vec3::ZERO,
        }
    }

    pub fn boost(boost: Vec3) -> Self {
        Self {
            acceleration: Vec3::ZERO,
            boost,
        }
    }

    pub fn cancel_on_axis(&mut self, axis: Vec3) {
        self.acceleration = self.acceleration.reject_from(axis);
        self.boost = self.boost.reject_from(axis);
    }

    pub fn calc_boost(&self, frame_duration: f32) -> Vec3 {
        self.acceleration * frame_duration + self.boost
    }
}

impl Default for VelChange {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Add<VelChange> for VelChange {
    type Output = VelChange;

    fn add(self, rhs: VelChange) -> Self::Output {
        Self::Output {
            acceleration: self.acceleration + rhs.acceleration,
            boost: self.boost + rhs.boost,
        }
    }
}

impl AddAssign for VelChange {
    fn add_assign(&mut self, rhs: Self) {
        self.acceleration += rhs.acceleration;
        self.boost += rhs.boost;
    }
}

/// Instructions on how to move forces to the rigid body.
///
/// The physics backend is responsible for reading this component during
/// [`PipelineStages::Sensors`](crate::PipelineStages::Sensors) and apply the forces to the
/// rigid body.
///
/// This documentation uses the term "forces", but in fact these numbers ignore mass and are
/// applied directly to the velocity.
#[derive(Component, Default, Debug)]
pub struct Motor {
    /// How much velocity to add to the rigid body in the current frame.
    pub lin: VelChange,

    /// How much angular velocity to add to the rigid body in the current frame, given as the
    /// rotation axis multiplied by the rotation speed in radians per second. Can be extracted from
    /// a quaternion using [`Quat::xyz`].
    pub ang: VelChange,
}

#[derive(Component)]
pub struct SubservientSensor {
    pub owner_entity: Entity,
}

#[derive(Component)]
pub struct ControllerGravity(pub Vec3);

impl Default for ControllerGravity {
    fn default() -> Self {
        Self(Vec3::NEG_Y * EARTH_GRAVITATIONAL_ACCELERATION)
    }
}
