use avian3d::math::{PI};
use avian3d::prelude::{RigidBody, Collider};
use avian3d::spatial_query::ShapeCaster;
use crate::physics::{GravitationalBody, EARTH_GRAVITY};
use bevy::ecs::component::ComponentId;
use bevy::ecs::world::DeferredWorld;
use bevy::hierarchy::Parent;
use bevy::math::{Dir3, Quat, Vec3};
use bevy::prelude::{Component, Entity, Transform};

/// A marker component indicating that an entity is using a character controller.
#[derive(Component, Default)]
#[require(
    RigidBody(|| RigidBody::Kinematic),
    MovementAcceleration(|| MovementAcceleration(30.0)),
    MovementDampingFactor(|| MovementDampingFactor(0.9)),
    JumpImpulse(|| JumpImpulse(7.0)),
    MaxSlopeAngle(|| MaxSlopeAngle(PI * 0.45))
)]
#[component(on_add = on_add_character_controller)]
pub struct CharacterController;

pub fn on_add_character_controller(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
    let parent = world
        .get::<Parent>(entity)
        .expect("expected Parent component to exist")
        .get();
    let radial_gravity = world.get::<GravitationalBody>(parent).is_some();
    let gravity_vector = match radial_gravity {
        true => {
            world
                .get::<Transform>(entity)
                .expect("expected character to have Transform component")
                .translation
                .normalize()
                * -1.0
                * EARTH_GRAVITY
        }
        false => Vec3::NEG_Y * EARTH_GRAVITY,
    };

    let collider = Collider::capsule(0.5, 1.0);
    let mut caster_shape = collider.clone();
    caster_shape.set_scale(Vec3::ONE * 0.99, 8);

    world.commands().entity(entity).insert((
        collider,
        ShapeCaster::new(caster_shape, Vec3::ZERO, Quat::default(), Dir3::NEG_Y)
            .with_max_distance(0.2),
        ControllerGravity(gravity_vector),
    ));

    if radial_gravity {
        world.commands().entity(entity).insert(RadialGravity);
    } else {
        world.commands().entity(entity).insert(LinearGravity);
    }
}

#[derive(Component)]
#[component(on_add = on_add_radial_gravity)]
pub struct RadialGravity;
fn on_add_radial_gravity(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
    world.commands().entity(entity).remove::<LinearGravity>();
}

#[derive(Component)]
#[component(on_add = on_add_linear_gravity)]
pub struct LinearGravity;

fn on_add_linear_gravity(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
    world.commands().entity(entity).remove::<RadialGravity>();
}

/// A marker component indicating that an entity is on the ground.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Grounded;

/// The acceleration used for character movement.
#[derive(Component)]
pub struct MovementAcceleration(pub f32);

/// The damping factor used for slowing down movement.
#[derive(Component)]
pub struct MovementDampingFactor(pub f32);

/// The strength of a jump.
#[derive(Component)]
pub struct JumpImpulse(pub f32);

/// The gravitational acceleration used for a character controller.
#[derive(Component)]
pub struct ControllerGravity(pub Vec3);

/// The maximum angle a slope can have for a character controller
/// to be able to climb and jump. If the slope is steeper than this angle,
/// the character will slide down.
#[derive(Component)]
pub struct MaxSlopeAngle(pub f32);
