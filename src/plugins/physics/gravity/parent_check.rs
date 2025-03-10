use super::*;
use bevy::hierarchy::{on_hierarchy_reports_enabled, ReportHierarchyIssue};
use bevy::utils::HashSet;

#[allow(clippy::type_complexity)]
pub fn check_hierarchy_local_gravity_has_valid_parent(
    parent_query: Query<
        (Entity, &Parent, Option<&Name>),
        (
            With<LocalGravity>,
            Or<(Changed<Parent>, Added<LocalGravity>)>,
        ),
    >,
    component_query: Query<(), Or<(With<LocalGravity>, With<GravityField>)>>,
    mut already_diagnosed: Local<HashSet<Entity>>,
) {
    for (entity, parent, name) in &parent_query {
        let parent = parent.get();
        if !component_query.contains(parent) && !already_diagnosed.contains(&entity) {
            already_diagnosed.insert(entity);
            warn!(
                "warning[B0004]: {name} with the LocalGravity component has a parent without either LocalGravity or Gravity.\n\
                This will cause inconsistent behaviors!",
                name = name.map_or_else(|| format!("Entity {}", entity), |s| format!("The {s} entity")),
            );
        }
    }
}

#[derive(Default)]
/// Print a warning for each `Entity` with a `LocalGravity` component
/// whose parent doesn't have a `LocalGravity` or `GravityField` component.
///
/// See [`check_hierarchy_local_gravity_has_valid_parent`] for details.
pub struct ValidGravityParentCheckPlugin;

impl Plugin for ValidGravityParentCheckPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ReportHierarchyIssue<LocalGravity>>()
            .add_systems(
                Last,
                check_hierarchy_local_gravity_has_valid_parent
                    .run_if(on_hierarchy_reports_enabled::<LocalGravity>),
            );
    }
}
