use bevy::math::{Dir3, Quat, Vec2, Vec3};

/// Calculate the rotation around `around_axis` required to rotate the character from
/// `current_forward` to `desired_forward`.
pub fn rotation_arc_around_axis(
    around_axis: Dir3,
    current_forward: Vec3,
    desired_forward: Vec3,
) -> Option<f32> {
    let around_axis: Vec3 = around_axis.into();
    let rotation_plane_x = current_forward.reject_from(around_axis).try_normalize()?;
    let rotation_plane_y = around_axis.cross(rotation_plane_x);
    let desired_forward_in_plane_coords = Vec2::new(
        rotation_plane_x.dot(desired_forward),
        rotation_plane_y.dot(desired_forward),
    )
        .try_normalize()?;
    let rotation_to_set_forward =
        Quat::from_rotation_arc_2d(Vec2::X, desired_forward_in_plane_coords);
    Some(rotation_to_set_forward.xyz().z)
}
