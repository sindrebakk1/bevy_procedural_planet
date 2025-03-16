use avian3d::math::AdjustPrecision;
use big_space::prelude::{Grid, GridCell};

use super::*;
use crate::Precision;

/// Query type for entities with a [`Parent`] component.
type ParentQuery<'w, 's> = Query<'w, 's, (Entity, &'static Parent)>;

type ComputeGravitiesChildQuery<'w, 's> = Query<
    'w,
    's,
    (
        Has<GravityField>,
        &'static GridCell<Precision>,
        &'static Transform,
        Option<&'static mut LocalGravity>,
        Option<&'static Children>,
    ),
>;

#[allow(clippy::type_complexity)]
pub fn compute_local_gravities(
    root_query: Query<(
        Entity,
        &GravityField,
        &Grid<Precision>,
        &GridCell<Precision>,
        &Transform,
        &Children,
    )>,
    child_query: ComputeGravitiesChildQuery,
    parent_query: ParentQuery,
) {
    root_query.par_iter().for_each(
        |(
             entity,
             gravity_field,
             grid,
             grid_cell,
             transform,
             children,
         )| {
            if !gravity_field.is_radial() {
                return;
            }
            let source = grid.grid_position_double(grid_cell, transform);
            for (child, actual_parent) in parent_query.iter_many(children) {
                debug_assert_eq!(
                    actual_parent.get(), entity,
                    "Malformed gravitational hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
                );
                #[expect(unsafe_code, reason = "`propagate_recursive()` is unsafe due to its use of `Query::get_unchecked()`.")]
                unsafe {
                    compute_local_gravities_recursive(
                        grid,
                        gravity_field,
                        &source,
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
    parent_grid: &Grid<Precision>,
    parent_field: &GravityField,
    source: &Vector,
    child_query: &ComputeGravitiesChildQuery,
    parent_query: &ParentQuery,
    entity: Entity,
) {
    let Ok((has_field, grid_cell, transform, local_gravity, children)) =
        (unsafe { child_query.get_unchecked(entity) })
    else {
        return;
    };
    if has_field {
        return;
    };
    if let Some(mut local_gravity) = local_gravity {
        let vector_to_source = source
            - parent_grid
                .grid_position_double(grid_cell, transform)
                .adjust_precision();
        info!(
            "vec to source: {vector_to_source:?}, local gravity: {g:?}",
            g = vector_to_source.normalize()
                * parent_field.gravitational_acceleration(vector_to_source.length())
        );
        local_gravity.0 = vector_to_source.normalize()
            * parent_field.gravitational_acceleration(vector_to_source.length());
    };
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
                parent_grid,
                parent_field,
                source,
                child_query,
                parent_query,
                child,
            );
        }
    }
}
