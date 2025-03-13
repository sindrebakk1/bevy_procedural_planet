use super::{
    cube_tree::Axis,
    helpers::{spherical_uv, unit_cube_to_sphere},
};
use crate::math::Rectangle;
use avian3d::math::{Scalar, Vector, Vector2};
use bevy::{
    asset::RenderAssetUsages,
    math::Vec3Swizzles,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};

// #[allow(unused)]
// trait RectangleExtension {
//     fn inverse_lerp(&self, point: Vector2) -> Vector2;
//     fn contains_rect(&self, rhs: Rectangle) -> bool;
// }
//
// impl RectangleExtension for Rectangle {
//     #[inline]
//     fn inverse_lerp(&self, point: Vector2) -> Vector2 {
//         Vector2::new(
//             (point.x - self.min.x) / (self.max.x - self.min.x),
//             (point.y - self.min.y) / (self.max.y - self.min.y),
//         )
//     }
//     #[inline(always)]
//     fn contains_rect(&self, rhs: Rectangle) -> bool {
//         self.contains(rhs.min) && self.contains(rhs.max)
//     }
// }

#[derive(Clone, Copy, Debug)]
pub struct ChunkMeshBuilder<const SUBDIVISIONS: usize>
where
    [(); SUBDIVISIONS]:,
    [(); (SUBDIVISIONS + 2) * 2]:,
    [(); (((SUBDIVISIONS + 2) * 2) - 1).pow(2) * 6]:,
{
    axis: Axis,
    radius: Scalar,
}

#[allow(unused)]
impl<const SUBDIVISIONS: usize> ChunkMeshBuilder<SUBDIVISIONS>
where
    [(); SUBDIVISIONS + 2]:,
    [(); (SUBDIVISIONS + 2) * 2]:,
    [(); (((SUBDIVISIONS + 2) * 2) - 1).pow(2) * 6]:,
{
    const VERTEX_COUNT: usize = SUBDIVISIONS + 2;

    pub fn new(axis: Axis, radius: Scalar) -> Self {
        Self { axis, radius }
    }

    pub fn build(&self, bounds: Rectangle) -> Mesh {
        let mut positions: [[f32; 3]; (SUBDIVISIONS + 2) * 2] = [[0.0; 3]; (SUBDIVISIONS + 2) * 2];
        let mut normals: [[f32; 3]; (SUBDIVISIONS + 2) * 2] = [[0.0; 3]; (SUBDIVISIONS + 2) * 2];
        let mut uvs: [[f32; 2]; (SUBDIVISIONS + 2) * 2] = [[0.0; 2]; (SUBDIVISIONS + 2) * 2];
        let mut indices: [u32; (((SUBDIVISIONS + 2) * 2) - 1).pow(2) * 6] =
            [0; (((SUBDIVISIONS + 2) * 2) - 1).pow(2) * 6];

        let axis_normal = Vector::from(self.axis);
        let local_x = axis_normal.yzx();
        let local_y = axis_normal.cross(local_x);
        let size = Vector2::splat(self.radius * 2.0);

        let bounds_min = bounds.min / size;
        let bounds_max = bounds.max / size;

        let center_pos_on_cube = axis_normal
            + (bounds_min.x + bounds_max.x) * local_x
            + (bounds_min.y + bounds_max.y) * local_y;

        // True center of the current chunk mesh in relation to the center of the planet
        let center = unit_cube_to_sphere(center_pos_on_cube) * self.radius;

        let step_x = (bounds_max.x - bounds_min.x) / (Self::VERTEX_COUNT - 1) as Scalar;
        let step_y = (bounds_max.y - bounds_min.y) / (Self::VERTEX_COUNT - 1) as Scalar;

        for y in 0..Self::VERTEX_COUNT {
            for x in 0..Self::VERTEX_COUNT {
                let p_x = bounds_min.x + x as Scalar * step_x;
                let p_y = bounds_min.y + y as Scalar * step_y;
                let pos_on_cube = axis_normal + p_x * 2.0 * local_x + p_y * 2.0 * local_y;
                let normal = unit_cube_to_sphere(pos_on_cube);
                let pos = normal * self.radius;

                let index = x + y * Self::VERTEX_COUNT;

                #[cfg(feature = "f64")]
                {
                    positions[index] = (pos - center).as_vec3().to_array();
                    normals[index] = normal.as_vec3().to_array();
                    uvs[index] = spherical_uv(pos).as_vec2().to_array();
                }

                #[cfg(not(feature = "f64"))]
                {
                    positions[index] = (pos * self.radius).to_array();
                    normals[index] = normal.to_array();
                    uvs[index] = spherical_uv(pos).to_array();
                }

                if x < Self::VERTEX_COUNT - 1 && y < Self::VERTEX_COUNT - 1 {
                    match (p_x < 0.0, p_y < 0.0) {
                        (false, false) => {
                            let i = index * Self::VERTEX_COUNT;
                            indices[i] = index as u32;
                            indices[i + 1] = (index + Self::VERTEX_COUNT + 1) as u32;
                            indices[i + 2] = (index + Self::VERTEX_COUNT) as u32;

                            indices[i + 3] = index as u32;
                            indices[i + 4] = (index + 1) as u32;
                            indices[i + 5] = (index + Self::VERTEX_COUNT + 1) as u32;
                        }
                        (true, false) => {
                            let i = index * Self::VERTEX_COUNT;
                            indices[i] = index as u32;
                            indices[i + 1] = (index + 1) as u32;
                            indices[i + 2] = (index + Self::VERTEX_COUNT) as u32;

                            indices[i + 3] = (index + 1) as u32;
                            indices[i + 4] = (index + Self::VERTEX_COUNT + 1) as u32;
                            indices[i + 5] = (index + Self::VERTEX_COUNT) as u32;
                        }
                        (false, true) => {
                            let i = index * Self::VERTEX_COUNT;

                            indices[i] = index as u32;
                            indices[i + 1] = (index + 1) as u32;
                            indices[i + 2] = (index + Self::VERTEX_COUNT) as u32;

                            indices[i + 3] = (index + Self::VERTEX_COUNT) as u32;
                            indices[i + 4] = (index + 1) as u32;
                            indices[i + 5] = (index + Self::VERTEX_COUNT + 1) as u32;
                        }
                        (true, true) => {
                            let i = index * Self::VERTEX_COUNT;

                            indices[i] = index as u32;
                            indices[i + 1] = (index + Self::VERTEX_COUNT + 1) as u32;
                            indices[i + 2] = (index + Self::VERTEX_COUNT) as u32;

                            indices[i + 3] = (index + 1) as u32;
                            indices[i + 4] = (index + Self::VERTEX_COUNT + 1) as u32;
                            indices[i + 5] = index as u32;
                        }
                    }
                }
            }
        }

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(Indices::U32(Vec::from(indices)))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::from(positions))
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::from(normals))
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, Vec::from(uvs))
    }
}
