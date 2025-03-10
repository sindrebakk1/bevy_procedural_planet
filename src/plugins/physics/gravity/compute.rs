use super::*;

/// Query type for entities with a [`Parent`] component.
type ParentQuery<'w, 's> = Query<'w, 's, (Entity, &'static Parent)>;

type ComputeGravitiesChildQuery<'w, 's> = Query<
    'w,
    's,
    (
        Has<GravityField>,
        &'static GlobalTransform,
        Option<&'static mut LocalGravity>,
        Option<&'static Children>,
    ),
>;

#[allow(clippy::type_complexity)]
pub fn compute_local_gravities(
    root_query: Query<(Entity, &GravityField, &GlobalTransform, &Children)>,
    child_query: ComputeGravitiesChildQuery,
    parent_query: ParentQuery,
) {
    root_query.par_iter().for_each(
        |(
             entity,
             gravity_field,
             global_transform,
             children,
         )| {
            let gravitational_parameter = match *gravity_field {
                GravityField::Radial { gravitational_parameter } => gravitational_parameter,
                _ => return,
            };
            for (child, actual_parent) in parent_query.iter_many(children) {
                debug_assert_eq!(
                    actual_parent.get(), entity,
                    "Malformed gravitational hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
                );
                #[expect(unsafe_code, reason = "`propagate_recursive()` is unsafe due to its use of `Query::get_unchecked()`.")]
                unsafe {
                    compute_local_gravities_recursive(
                        gravitational_parameter,
                        global_transform,
                        &child_query,
                        &parent_query,
                        child,
                    );
                }
            }
        }
    );
}

unsafe fn compute_local_gravities_recursive(
    gravitational_parameter: Scalar,
    source: &GlobalTransform,
    child_query: &ComputeGravitiesChildQuery,
    parent_query: &ParentQuery,
    entity: Entity,
) {
    let Ok((has_field, global_transform, local_gravity, children)) =
        (unsafe { child_query.get_unchecked(entity) })
    else {
        return;
    };
    if has_field {
        return;
    };
    if local_gravity.is_some() {
        let vector_to_source = source.translation() - global_transform.translation();
        unsafe {
            local_gravity.unwrap_unchecked().0 = vector_to_source.normalize()
                * vector_to_source.length_squared()
                * gravitational_parameter;
        }
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
            compute_local_gravities_recursive(
                gravitational_parameter,
                source,
                child_query,
                parent_query,
                child,
            );
        }
    }
}
