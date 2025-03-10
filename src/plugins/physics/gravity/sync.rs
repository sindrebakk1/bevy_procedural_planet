use super::*;
use bevy::{
    ecs::{query::QueryFilter, world::CommandQueue},
    utils::Parallel,
};

/// Query type for entities with a [`Parent`] component.
type ParentQuery<'w, 's> = Query<'w, 's, (Entity, &'static Parent)>;

/// Child query for [`prune_gravities_on_component_removed`]
type PruneOrphansChildQuery<'w, 's> = Query<
    'w,
    's,
    (
        Has<LocalGravity>,
        Has<GravityField>,
        Option<&'static Children>,
    ),
>;

/// Update or remove the [`LocalGravity`] component of orphaned entities.
///
/// This function removes the [`LocalGravity`] component from entities that are no longer part of the hierarchy.
///
/// # Type Parameters
/// - `C`: The component type that is removed from orphaned entities (e.g., [`Parent`]).
/// - `F`: The [`QueryFilter`] type that specifies which entities to query (e.g., [`With<LocalGravity>`] or [`Without<Parent>`]).
///
/// # Arguments
/// - `commands`: The commands to execute for entity modifications.
/// - `query`: The query for orphaned entities based on the `F` filter.
/// - `child_query`: A query for child entities to check their gravity component and hierarchy status.
/// - `parent_query`: A query for parent-child relationships in the hierarchy.
/// - `orphaned`: The [`RemovedComponents<C>`] that tracks which entities lost their [`Parent`] component.
pub fn prune_gravities_on_component_removed<C: Component, F: QueryFilter>(
    mut commands: Commands,
    query: Query<(Entity, Option<&Children>), F>,
    child_query: PruneOrphansChildQuery,
    parent_query: ParentQuery,
    mut removed: RemovedComponents<C>,
) {
    let mut entities: Vec<Entity> = Vec::new();
    let mut command_queue = CommandQueue::default();

    for (entity, children) in query.iter_many(removed.read()) {
        entities.push(entity);
        if let Some(children) = children {
            for (child, actual_parent) in parent_query.iter_many(children) {
                debug_assert_eq!(
                    actual_parent.get(), entity,
                    "Malformed gravitational hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
                );
                unsafe {
                    gather_entities_for_pruning_recursive(
                        &child_query,
                        &parent_query,
                        child,
                        &mut entities,
                    );
                }
            }
        }
    }

    for entity in entities.into_iter() {
        command_queue.push(move |world: &mut World| {
            world.entity_mut(entity).remove::<LocalGravity>();
        });
    }

    commands.append(&mut command_queue);
}

unsafe fn gather_entities_for_pruning_recursive(
    child_query: &PruneOrphansChildQuery,
    parent_query: &ParentQuery,
    entity: Entity,
    entities: &mut Vec<Entity>,
) {
    let Ok((has_gravity, has_field, children)) = (unsafe { child_query.get_unchecked(entity) })
    else {
        return;
    };
    if has_field {
        return;
    }
    if has_gravity {
        entities.push(entity)
    }
    let Some(children) = children else {
        return;
    };
    for (child, actual_parent) in parent_query.iter_many(children) {
        debug_assert_eq!(
            actual_parent.get(), entity,
            "Malformed gravitational hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
        );

        unsafe {
            gather_entities_for_pruning_recursive(child_query, parent_query, child, entities);
        }
    }
}

/// Child query for [`insert_local_gravities`]
type InsertGravitiesChildQuery<'w, 's> = Query<
    'w,
    's,
    (
        Has<LocalGravity>,
        Has<GravityField>,
        Option<&'static RigidBody>,
        Option<&'static Children>,
    ),
>;

pub fn insert_local_gravities(
    mut commands: Commands,
    query: Query<(Entity, &GravityField, &Children)>,
    child_query: InsertGravitiesChildQuery,
    parent_query: ParentQuery,
) {
    // let mut entities: Arc<Mutex<Vec<Entity>>> = Arc::new(Mutex::new(Vec::new()));
    let mut entities: Parallel<Vec<(Entity, Vector)>> = Parallel::default();
    query.par_iter().for_each_init(
        || entities.borrow_local_mut(),
        |local_entities, (
             entity,
             field,
             children,
         )| {
            let gravity_vector = match field {
                GravityField::Linear(vector) => *vector,
                _ => Vector::ZERO,
            };
            for (child, actual_parent) in parent_query.iter_many(children) {
                debug_assert_eq!(
                    actual_parent.get(), entity,
                    "Malformed gravitational hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
                );
                #[expect(unsafe_code, reason = "`propagate_recursive()` is unsafe due to its use of `Query::get_unchecked()`.")]
                unsafe {
                    gather_entities_for_insert_recursive(
                        gravity_vector,
                        &child_query,
                        &parent_query,
                        child,
                        local_entities,
                    )
                }
            }
        }
    );

    let mut command_queue = CommandQueue::default();

    entities.drain().for_each(|(entity, gravity_vector)| {
        command_queue.push(move |world: &mut World| {
            world
                .entity_mut(entity)
                .insert(LocalGravity(gravity_vector));
        });
    });

    commands.append(&mut command_queue);
}

unsafe fn gather_entities_for_insert_recursive(
    gravity_vector: Vector,
    child_query: &InsertGravitiesChildQuery,
    parent_query: &ParentQuery,
    entity: Entity,
    entities: &mut Vec<(Entity, Vector)>,
) {
    let Ok((has_gravity, has_field, rigid_body, children)) =
        (unsafe { child_query.get_unchecked(entity) })
    else {
        return;
    };
    if has_field {
        return;
    }
    if rigid_body.is_some_and(|rigid_body| !rigid_body.is_static()) && !has_gravity {
        entities.push((entity, gravity_vector));
    }
    let Some(children) = children else {
        return;
    };
    for (child, actual_parent) in parent_query.iter_many(children) {
        debug_assert_eq!(
            actual_parent.get(), entity,
            "Malformed gravitational hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
        );

        unsafe {
            gather_entities_for_insert_recursive(
                gravity_vector,
                child_query,
                parent_query,
                child,
                entities,
            );
        }
    }
}

/// Child query for [`propogate_linear_gravities`]
type PropogateChildQuery<'w, 's> =
    Query<'w, 's, (Option<&'static mut LocalGravity>, Option<&'static Children>)>;

#[allow(clippy::type_complexity)]
pub fn propogate_linear_gravities(
    query: Query<
        (Entity, Ref<GravityField>, &Children),
        Or<(Changed<GravityField>, Added<GravityField>)>,
    >,
    child_query: PropogateChildQuery,
    parent_query: ParentQuery,
) {
    query.par_iter().for_each(
        |(
            entity,
            gravity_field,
            children,
         )| {
            let gravity_vector = match *gravity_field {
                GravityField::Linear(vector) => vector,
                GravityField::Radial {..} => Vector::ZERO,
            };
            for (child, actual_parent) in parent_query.iter_many(children) {
                debug_assert_eq!(
                    actual_parent.get(), entity,
                    "Malformed gravitational hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
                );
                #[expect(unsafe_code, reason = "`propagate_recursive()` is unsafe due to its use of `Query::get_unchecked()`.")]
                unsafe {
                    propogate_linear_gravities_recursive(
                        gravity_vector,
                        &child_query,
                        &parent_query,
                        child,
                    );
                }
            }
        }
    );
}

unsafe fn propogate_linear_gravities_recursive(
    gravity_vector: Vector,
    child_query: &PropogateChildQuery,
    parent_query: &ParentQuery,
    entity: Entity,
) {
    let Ok((gravity, children)) = (unsafe { child_query.get_unchecked(entity) }) else {
        return;
    };
    if let Some(mut gravity) = gravity {
        gravity.0 = gravity_vector;
    }
    let Some(children) = children else {
        return;
    };
    for (child, actual_parent) in parent_query.iter_many(children) {
        debug_assert_eq!(
            actual_parent.get(), entity,
            "Malformed gravitational hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
        );

        unsafe {
            propogate_linear_gravities_recursive(gravity_vector, child_query, parent_query, child);
        }
    }
}
