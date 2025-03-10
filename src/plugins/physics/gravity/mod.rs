use avian3d::{
    math::{Scalar, Vector},
    prelude::*,
};
use bevy::prelude::*;

pub mod compute;
pub mod parent_check;
pub mod sync;

use crate::constants::physics::G;
use compute::compute_local_gravities;
use parent_check::ValidGravityParentCheckPlugin;
use sync::{
    insert_local_gravities, propogate_linear_gravities, prune_gravities_on_component_removed,
};

pub type GlobalGravity = avian3d::dynamics::integrator::Gravity;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum SyncGravitiesSystem {
    /// Removes [`LocalGravity`] from orphaned entities, or from descendants of entities that have
    /// their [`GravityField`] removed. Adds [`LocalGravity`] to descendants of [`GravityField`] if
    /// they contain a non-static [`RigidBody`]
    Sync,
    /// Propagates changes in [`GravityField`] to children's [`LocalGravity`].
    /// This includes syncing [`LocalGravity`] components for children of entities
    /// with [`GravityField::Linear`] if the [`GravityField`] has been added or changed,
    ///
    /// NOTE: Children of entities with [`GravityField::Radial`] will have their [`LocalGravity`]
    /// initialized as [`LocalGravity::ZERO`]. The correct value will be computed during [`PhysicsSet::Prepare`].
    Propogate,
}

pub struct GravityPlugin;

impl Plugin for GravityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GlobalGravity>()
            .add_plugins(ValidGravityParentCheckPlugin)
            .configure_sets(
                PostStartup,
                (SyncGravitiesSystem::Sync, SyncGravitiesSystem::Propogate)
                    .chain()
                    .before(PhysicsSet::Prepare),
            )
            .add_systems(
                PostStartup,
                (
                    (
                        prune_gravities_on_component_removed::<Parent, With<LocalGravity>>,
                        prune_gravities_on_component_removed::<GravityField, ()>,
                        insert_local_gravities,
                    )
                        .in_set(SyncGravitiesSystem::Sync),
                    propogate_linear_gravities.in_set(SyncGravitiesSystem::Propogate),
                    compute_local_gravities.in_set(PhysicsSet::Prepare),
                ),
            )
            .configure_sets(
                PostUpdate,
                (SyncGravitiesSystem::Sync, SyncGravitiesSystem::Propogate)
                    .chain()
                    .before(PhysicsSet::Prepare),
            )
            .add_systems(
                PostUpdate,
                (
                    (
                        prune_gravities_on_component_removed::<Parent, With<LocalGravity>>,
                        prune_gravities_on_component_removed::<GravityField, ()>,
                        insert_local_gravities,
                    )
                        .in_set(SyncGravitiesSystem::Sync),
                    propogate_linear_gravities.in_set(SyncGravitiesSystem::Propogate),
                    compute_local_gravities.in_set(PhysicsSet::Prepare),
                ),
            );
    }
}

/// Represents a gravitational field that affects entities within its influence.
///
/// This component defines how gravity is applied to entities, either as a
/// constant force in a direction (`Linear`) or as a radial field (`Radial`)
/// that follows Newtonian gravity.
///
/// # Usage
/// - `GravityField::Linear(Vec3)`: Represents a uniform gravitational field,
///   like Earth's gravity pulling objects downward.
/// - `GravityField::Radial { gravitational_parameter }`: Represents a radial
///   gravity source (e.g., planets), where acceleration follows the inverse-square law.
#[derive(Component, Debug, Copy, Clone, PartialEq)]
#[require(Transform)]
pub enum GravityField {
    /// A uniform gravitational field that applies a constant force in a fixed direction.
    ///
    /// # Example
    /// ```
    /// use avian3d::math::Vector;
    /// use procedural_planet::plugins::physics::GravityField;
    ///
    /// let gravity = GravityField::Linear(Vector::new(0.0, -9.81, 0.0)); // Earth-like gravity
    /// ```
    Linear(Vector),

    /// A radial gravitational field that applies Newtonian gravity, where acceleration
    /// is proportional to `GM / r²`, where `r` is the distance from the source.
    ///
    /// - `gravitational_parameter` (GM) is the product of the gravitational constant
    ///   (`G`) and the mass (`M`) of the gravity source.
    ///
    /// # Example
    /// ```
    /// use procedural_planet::plugins::physics::GravityField;
    ///
    /// let planet_gravity = GravityField::Radial { gravitational_parameter: 398600.0 }; // Earth's GM in km³/s²
    /// ```
    Radial {
        /// The gravitational parameter (GM), where `G` is the gravitational constant
        /// and `M` is the mass of the gravity source. Determines the strength of
        /// the gravitational field.
        gravitational_parameter: Scalar,
    },
}

impl GravityField {
    pub fn new_linear(gravity_vector: Vector) -> Self {
        Self::Linear(gravity_vector)
    }

    pub fn new_radial(gravitational_parameter: Scalar) -> Self {
        Self::Radial {
            gravitational_parameter,
        }
    }

    pub fn radial_from_mass(mass_kg: Scalar) -> Self {
        Self::Radial {
            gravitational_parameter: G * mass_kg,
        }
    }

    pub fn gravitational_acceleration(&self, distance_m: Scalar) -> Scalar {
        match self {
            GravityField::Linear(gravity) => gravity.length(),
            GravityField::Radial {
                gravitational_parameter,
            } => gravitational_parameter / distance_m.powi(2),
        }
    }

    pub fn is_radial(&self) -> bool {
        match self {
            GravityField::Linear(_) => false,
            GravityField::Radial { .. } => true,
        }
    }

    pub fn is_linear(&self) -> bool {
        match self {
            GravityField::Linear(_) => true,
            GravityField::Radial { .. } => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TryFromGravityFieldError {
    IncorrectVariant(String),
}

#[derive(Component)]
#[require(Transform)]
pub struct LocalGravity(pub Vector);

#[allow(unused)]
impl LocalGravity {
    const ZERO: Self = LocalGravity(Vector::ZERO);

    pub fn new(gravity: Vector) -> Self {
        Self(gravity)
    }

    pub fn as_vec(&self) -> Vector {
        self.0
    }

    pub fn set(&mut self, gravity: Vector) {
        self.0 = gravity;
    }
}

impl From<LocalGravity> for Vector {
    fn from(value: LocalGravity) -> Self {
        value.0
    }
}
