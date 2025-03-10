use avian3d::{
    math::{AdjustPrecision, AsF32, Scalar, Vector},
    prelude::{mass_properties::components::GlobalAngularInertia, *},
    schedule::PhysicsStepSet,
};
use bevy::{
    ecs::schedule::{InternedScheduleLabel, ScheduleLabel},
    prelude::*,
};
use bevy_tnua::{
    data_for_backends::{
        TnuaGhostPlatform, TnuaGhostSensor, TnuaMotor, TnuaProximitySensor,
        TnuaProximitySensorOutput, TnuaRigidBodyTracker, TnuaToggle,
    },
    subservient_sensors::TnuaSubservientSensor,
    TnuaPipelineStages, TnuaSystemSet,
};
use bevy_tnua_avian3d::TnuaAvian3dSensorShape;

use crate::plugins::physics::{GlobalGravity, LocalGravity};

/// Add this plugin to use avian3d as a physics backend.
///
/// This plugin should be used in addition to `TnuaControllerPlugin`.
pub struct TnuaOverridesPlugin {
    schedule: InternedScheduleLabel,
}

impl TnuaOverridesPlugin {
    pub fn new(schedule: impl ScheduleLabel) -> Self {
        Self {
            schedule: schedule.intern(),
        }
    }
}

impl Plugin for TnuaOverridesPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            self.schedule,
            TnuaSystemSet
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
                .in_set(TnuaPipelineStages::Sensors),
        );
        app.add_systems(
            self.schedule,
            apply_motors_system.in_set(TnuaPipelineStages::Motors),
        );
    }
}

#[allow(clippy::type_complexity)]
fn update_rigid_body_trackers_system(
    gravity: Res<GlobalGravity>,
    mut query: Query<(
        &GlobalTransform,
        &LinearVelocity,
        &AngularVelocity,
        &mut TnuaRigidBodyTracker,
        Option<&LocalGravity>,
        Option<&TnuaToggle>,
    )>,
) {
    for (transform, linear_velocity, angular_velocity, mut tracker, local_gravity, tnua_toggle) in
        query.iter_mut()
    {
        match tnua_toggle.copied().unwrap_or_default() {
            TnuaToggle::Disabled => continue,
            TnuaToggle::SenseOnly => {}
            TnuaToggle::Enabled => {}
        }
        let (_, rotation, translation) = transform.to_scale_rotation_translation();
        *tracker = TnuaRigidBodyTracker {
            translation: translation.adjust_precision(),
            rotation: rotation.adjust_precision(),
            velocity: linear_velocity.0.adjust_precision(),
            angvel: angular_velocity.0.adjust_precision(),
            gravity: local_gravity
                .map_or(gravity.0, |local| local.0)
                .adjust_precision(),
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
        &mut TnuaProximitySensor,
        Option<&TnuaAvian3dSensorShape>,
        Option<&mut TnuaGhostSensor>,
        Option<&TnuaSubservientSensor>,
        Option<&TnuaToggle>,
    )>,
    collision_layers_entity: Query<&CollisionLayers>,
    other_object_query: Query<(
        Option<(&GlobalTransform, &LinearVelocity, &AngularVelocity)>,
        Option<&CollisionLayers>,
        Has<TnuaGhostPlatform>,
        Has<Sensor>,
    )>,
) {
    query.par_iter_mut().for_each(
        |(
            owner_entity,
            transform,
            mut sensor,
            shape,
            mut ghost_sensor,
            subservient,
            tnua_toggle,
        )| {
            match tnua_toggle.copied().unwrap_or_default() {
                TnuaToggle::Disabled => return,
                TnuaToggle::SenseOnly => {}
                TnuaToggle::Enabled => {}
            }

            // TODO: is there any point in doing these transformations as f64 when that feature
            // flag is active?
            let cast_origin = transform
                .transform_point(sensor.cast_origin.f32())
                .adjust_precision();
            let cast_direction = sensor.cast_direction;

            struct CastResult {
                entity: Entity,
                proximity: Scalar,
                intersection_point: Vector,
                normal: Dir3,
            }

            let owner_entity = if let Some(subservient) = subservient {
                subservient.owner_entity
            } else {
                owner_entity
            };

            let collision_layers = collision_layers_entity.get(owner_entity).ok();

            let mut final_sensor_output = None;
            if let Some(ghost_sensor) = ghost_sensor.as_mut() {
                ghost_sensor.0.clear();
            }
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
                                manifold.normal2.adjust_precision()
                            } else {
                                manifold.normal1.adjust_precision()
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

                let Ok((
                    entity_kinematic_data,
                    entity_collision_layers,
                    entity_is_ghost,
                    entity_is_sensor,
                )) = other_object_query.get(entity)
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
                            Vector::ZERO
                        };
                } else {
                    entity_angvel = Vector::ZERO;
                    entity_linvel = Vector::ZERO;
                }
                let sensor_output = TnuaProximitySensorOutput {
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

                if entity_is_ghost {
                    if let Some(ghost_sensor) = ghost_sensor.as_mut() {
                        ghost_sensor.0.push(sensor_output);
                    }
                    true
                } else if entity_is_sensor || excluded_by_collision_layers() {
                    true
                } else {
                    final_sensor_output = Some(sensor_output);
                    false
                }
            };

            let query_filter = SpatialQueryFilter::from_excluded_entities([owner_entity]);
            if let Some(TnuaAvian3dSensorShape(shape)) = shape {
                let (_, owner_rotation, _) = transform.to_scale_rotation_translation();
                let owner_rotation = Quat::from_axis_angle(
                    *cast_direction,
                    owner_rotation.to_scaled_axis().dot(*cast_direction),
                );
                spatial_query_pipeline.shape_hits_callback(
                    shape,
                    cast_origin,
                    owner_rotation.adjust_precision(),
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
                            normal: Dir3::new(shape_hit_data.normal1.f32())
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
                            normal: Dir3::new(ray_hit_data.normal.f32())
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
        &TnuaMotor,
        &mut LinearVelocity,
        &mut AngularVelocity,
        &ComputedMass,
        &GlobalAngularInertia,
        &mut ExternalForce,
        &mut ExternalTorque,
        Option<&TnuaToggle>,
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
        tnua_toggle,
    ) in query.iter_mut()
    {
        match tnua_toggle.copied().unwrap_or_default() {
            TnuaToggle::Disabled | TnuaToggle::SenseOnly => {
                *external_force = Default::default();
                return;
            }
            TnuaToggle::Enabled => {}
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
