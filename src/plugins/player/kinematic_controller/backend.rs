use avian3d::{
    dynamics::rigid_body::mass_properties::components::GlobalAngularInertia, prelude::*,
    schedule::PhysicsStepSet,
};
use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use bevy::prelude::*;

use super::components::{
    ControllerToggle, Motor, ProximitySensor, ProximitySensorOutput, RigidBodyTracker,
    SubservientSensor,
};
use super::{CharacterControllerSystemSet, ControllerPipelineStages};

/// Add this plugin to use avian3d as a physics backend.
///
/// This plugin should be used in addition to `ControllerPlugin`.
pub struct Avian3dBackendPlugin {
    schedule: InternedScheduleLabel,
}

impl Avian3dBackendPlugin {
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
        }
    }
}

impl Plugin for Avian3dBackendPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            self.schedule,
            CharacterControllerSystemSet
                .before(PhysicsSet::Prepare)
                .before(PhysicsStepSet::First)
                .run_if(|physics_time: Res<Time<Physics>>| !physics_time.is_paused()),
        );
        app.add_systems(
            self.schedule,
            (
                update_rigid_body_trackers_system,
                update_proximity_sensors_system,
            )
                .in_set(ControllerPipelineStages::Sensors),
        );
        app.add_systems(
            self.schedule,
            apply_motors_system.in_set(ControllerPipelineStages::Motors),
        );
    }
}

/// Add this component to make [`ProximitySensor`] cast a shape instead of a ray.
#[derive(Component)]
pub struct Avian3dSensorShape(pub Collider);

fn update_rigid_body_trackers_system(
    gravity: Res<Gravity>,
    mut query: Query<(
        &GlobalTransform,
        &LinearVelocity,
        &AngularVelocity,
        &mut RigidBodyTracker,
    )>,
) {
    for (transform, linaer_velocity, angular_velocity, mut tracker) in query.iter_mut() {
        let (_, rotation, translation) = transform.to_scale_rotation_translation();
        *tracker = RigidBodyTracker {
            translation: translation.adjust_precision(),
            rotation: rotation.adjust_precision(),
            velocity: linaer_velocity.0.adjust_precision(),
            angvel: angular_velocity.0.adjust_precision(),
            gravity: gravity.0,
        };
    }
}

#[allow(clippy::type_complexity)]
fn update_proximity_sensors_system(
    spatial_query_pipeline: Res<SpatialQueryPipeline>,
    collisions: Res<Collisions>,
    mut query: Query<(
        Entity,
        &GlobalTransform,
        &mut ProximitySensor,
        Option<&Avian3dSensorShape>,
        Option<&SubservientSensor>,
        Option<&ControllerToggle>,
    )>,
    collision_layers_entity: Query<&CollisionLayers>,
    other_object_query: Query<(
        Option<(&GlobalTransform, &LinearVelocity, &AngularVelocity)>,
        Option<&CollisionLayers>,
        Has<Sensor>,
    )>,
) {
    query.par_iter_mut().for_each(
        |(owner_entity, transform, mut sensor, shape, subservient, toggle)| {
            match toggle.copied().unwrap_or_default() {
                ControllerToggle::Disabled => return,
                ControllerToggle::SenseOnly => {}
                ControllerToggle::Enabled => {}
            }

            let cast_origin = transform
                .transform_point(sensor.cast_origin)
                .adjust_precision();
            let cast_direction = sensor.cast_direction;

            struct CastResult {
                entity: Entity,
                proximity: f32,
                intersection_point: Vec3,
                normal: Dir3,
            }

            let owner_entity = if let Some(subservient) = subservient {
                subservient.owner_entity
            } else {
                owner_entity
            };

            let collision_layers = collision_layers_entity.get(owner_entity).ok();

            let mut final_sensor_output = None;

            let mut apply_cast = |cast_result: CastResult| {
                let CastResult {
                    entity,
                    proximity,
                    intersection_point,
                    normal,
                } = cast_result;

                // This fixes https://github.com/idanarye/bevy-tnua/issues/14
                if let Some(contacts) = collisions.get(owner_entity, entity) {
                    let same_order = owner_entity == contacts.entity1;
                    for manifold in contacts.manifolds.iter() {
                        if !manifold.contacts.is_empty() {
                            let manifold_normal = if same_order {
                                manifold.normal2
                            } else {
                                manifold.normal1
                            };
                            if sensor.intersection_match_prevention_cutoff
                                < manifold_normal.dot(cast_direction.adjust_precision())
                            {
                                return true;
                            }
                        }
                    }
                }

                // TODO: see if https://github.com/idanarye/bevy-tnua/issues/14 replicates in Avian,
                // and if figure out how to port its fix to Avian.

                let Ok((entity_kinematic_data, entity_collision_layers, entity_is_sensor)) =
                    other_object_query.get(entity)
                else {
                    return false;
                };

                let entity_linvel;
                let entity_angvel;
                if let Some((entity_transform, entity_linear_velocity, entity_angular_velocity)) =
                    entity_kinematic_data
                {
                    entity_angvel = entity_angular_velocity.0.adjust_precision();
                    entity_linvel = entity_linear_velocity.0.adjust_precision()
                        + if 0.0 < entity_angvel.length_squared() {
                            let relative_point = intersection_point
                                - entity_transform.translation().adjust_precision();
                            // NOTE: no need to project relative_point on the
                            // rotation plane, it will not affect the cross
                            // product.
                            entity_angvel.cross(relative_point)
                        } else {
                            Vec3::ZERO
                        };
                } else {
                    entity_angvel = Vec3::ZERO;
                    entity_linvel = Vec3::ZERO;
                }
                let sensor_output = ProximitySensorOutput {
                    entity,
                    proximity,
                    normal,
                    entity_linvel,
                    entity_angvel,
                };

                let excluded_by_collision_layers = || {
                    let collision_layers = collision_layers.copied().unwrap_or_default();
                    let entity_collision_layers =
                        entity_collision_layers.copied().unwrap_or_default();
                    !collision_layers.interacts_with(entity_collision_layers)
                };

                if entity_is_sensor || excluded_by_collision_layers() {
                    true
                } else {
                    final_sensor_output = Some(sensor_output);
                    false
                }
            };

            let query_filter = SpatialQueryFilter::from_excluded_entities([owner_entity]);
            if let Some(Avian3dSensorShape(shape)) = shape {
                let (_, owner_rotation, _) = transform.to_scale_rotation_translation();
                let owner_rotation = Quat::from_axis_angle(
                    *cast_direction,
                    owner_rotation.to_scaled_axis().dot(*cast_direction),
                );
                spatial_query_pipeline.shape_hits_callback(
                    shape,
                    cast_origin,
                    owner_rotation,
                    cast_direction,
                    &ShapeCastConfig {
                        max_distance: sensor.cast_range,
                        ignore_origin_penetration: true,
                        ..default()
                    },
                    &query_filter,
                    |shape_hit_data| {
                        apply_cast(CastResult {
                            entity: shape_hit_data.entity,
                            proximity: shape_hit_data.distance,
                            intersection_point: shape_hit_data.point1,
                            normal: Dir3::new(shape_hit_data.normal1)
                                .unwrap_or_else(|_| -cast_direction),
                        })
                    },
                );
            } else {
                spatial_query_pipeline.ray_hits_callback(
                    cast_origin,
                    cast_direction,
                    sensor.cast_range,
                    true,
                    &query_filter,
                    |ray_hit_data| {
                        apply_cast(CastResult {
                            entity: ray_hit_data.entity,
                            proximity: ray_hit_data.distance,
                            intersection_point: cast_origin
                                + ray_hit_data.distance * cast_direction.adjust_precision(),
                            normal: Dir3::new(ray_hit_data.normal)
                                .unwrap_or_else(|_| -cast_direction),
                        })
                    },
                );
            }
            sensor.output = final_sensor_output;
        },
    );
}

#[allow(clippy::type_complexity)]
fn apply_motors_system(
    mut query: Query<(
        &Motor,
        &mut LinearVelocity,
        &mut AngularVelocity,
        &ComputedMass,
        &GlobalAngularInertia,
        &mut ExternalForce,
        &mut ExternalTorque,
        Option<&ControllerToggle>,
    )>,
) {
    for (
        motor,
        mut linear_velocity,
        mut angular_velocity,
        mass,
        inertia,
        mut external_force,
        mut external_torque,
        toggle,
    ) in query.iter_mut()
    {
        match toggle.copied().unwrap_or_default() {
            ControllerToggle::Disabled | ControllerToggle::SenseOnly => {
                *external_force = Default::default();
                return;
            }
            ControllerToggle::Enabled => {}
        }
        if motor.lin.boost.is_finite() {
            linear_velocity.0 += motor.lin.boost;
        }
        if motor.lin.acceleration.is_finite() {
            external_force.set_force(motor.lin.acceleration * mass.value());
        }
        if motor.ang.boost.is_finite() {
            angular_velocity.0 += motor.ang.boost;
        }
        if motor.ang.acceleration.is_finite() {
            external_torque.set_torque(
                // NOTE: I did not actually verify that this is the correct formula. Nothing uses
                // angular acceleration yet - only angular impulses.
                inertia.value() * motor.ang.acceleration,
            );
        }
    }
}
