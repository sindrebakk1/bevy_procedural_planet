use super::{
    cube_tree::Axis,
    helpers::{spherical_uv, unit_cube_to_sphere, AXIS_COORDINATE_FRAMES},
};
use crate::math::quad_tree::QuadTreeNode;
use crate::math::Rectangle;
use crate::plugins::terrain::cube_tree::{ChunkData, CubeTreeNode};
use avian3d::math::{Scalar, Vector, Vector2};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};

#[derive(Clone, Copy, Debug)]
pub struct ChunkMeshBuilder<const SUBDIVISIONS: usize>
where
    [(); (SUBDIVISIONS + 2).pow(2)]:,
    [(); (SUBDIVISIONS + 1).pow(2) * 6]:,
{
    radius: Scalar,
    size: Vector2,
}

#[allow(unused)]
impl<const SUBDIVISIONS: usize> ChunkMeshBuilder<SUBDIVISIONS>
where
    [(); (SUBDIVISIONS + 2).pow(2)]:,
    [(); (SUBDIVISIONS + 1).pow(2) * 6]:,
{
    const VERTEX_COUNT: usize = SUBDIVISIONS + 2;

    pub fn new(radius: Scalar) -> Self {
        Self {
            radius,
            size: Vector2::splat(radius * 2.0),
        }
    }

    pub fn build(&self, bounds: &Rectangle, chunk_data: &ChunkData) -> Mesh {
        let mut positions: [[f32; 3]; (SUBDIVISIONS + 2).pow(2)] =
            [[0.0; 3]; (SUBDIVISIONS + 2).pow(2)];
        let mut normals: [[f32; 3]; (SUBDIVISIONS + 2).pow(2)] =
            [[0.0; 3]; (SUBDIVISIONS + 2).pow(2)];
        let mut uvs: [[f32; 2]; (SUBDIVISIONS + 2).pow(2)] = [[0.0; 2]; (SUBDIVISIONS + 2).pow(2)];
        let mut indices: [u32; (SUBDIVISIONS + 1).pow(2) * 6] = [0; (SUBDIVISIONS + 1).pow(2) * 6];

        let axis = chunk_data.hash.axis();
        let (axis_normal, local_x, local_y) = AXIS_COORDINATE_FRAMES[&axis];

        let bounds_min = bounds.min / self.size;
        let bounds_max = bounds.max / self.size;

        let step_x = (bounds_max.x - bounds_min.x) / (Self::VERTEX_COUNT - 1) as Scalar;
        let step_y = (bounds_max.y - bounds_min.y) / (Self::VERTEX_COUNT - 1) as Scalar;

        let mut triangle_index = 0;

        for y in 0..Self::VERTEX_COUNT {
            for x in 0..Self::VERTEX_COUNT {
                let p_x = bounds_min.x + x as Scalar * step_x;
                let p_y = bounds_min.y + y as Scalar * step_y;

                let pos_on_cube = axis_normal + p_x * 2.0 * local_x + p_y * 2.0 * local_y;
                let normal = unit_cube_to_sphere(pos_on_cube);
                let pos = normal * self.radius;

                let index = x + (y * Self::VERTEX_COUNT);

                #[cfg(feature = "f64")]
                {
                    positions[index] = (pos - chunk_data.center).as_vec3().to_array();
                    normals[index] = normal.as_vec3().to_array();
                    uvs[index] = spherical_uv(pos).as_vec2().to_array();
                }

                #[cfg(not(feature = "f64"))]
                {
                    positions[index] = (pos).to_array();
                    normals[index] = normal.to_array();
                    uvs[index] = spherical_uv(pos).to_array();
                }

                if x < Self::VERTEX_COUNT - 1 && y < Self::VERTEX_COUNT - 1 {
                    let i = triangle_index * 6;
                    triangle_index += 1;
                    match (p_x < 0.0, p_y < 0.0) {
                        (false, false) => {
                            // First triangle: CCW winding
                            indices[i] = index as u32;
                            indices[i + 1] = (index + Self::VERTEX_COUNT) as u32;
                            indices[i + 2] = (index + Self::VERTEX_COUNT + 1) as u32;

                            // Second triangle: CCW winding
                            indices[i + 3] = index as u32;
                            indices[i + 4] = (index + Self::VERTEX_COUNT + 1) as u32;
                            indices[i + 5] = (index + 1) as u32;
                        }
                        (true, false) => {
                            // First triangle: CCW winding
                            indices[i] = index as u32;
                            indices[i + 1] = (index + Self::VERTEX_COUNT) as u32;
                            indices[i + 2] = (index + 1) as u32;

                            // Second triangle: CCW winding
                            indices[i + 3] = (index + 1) as u32;
                            indices[i + 4] = (index + Self::VERTEX_COUNT) as u32;
                            indices[i + 5] = (index + Self::VERTEX_COUNT + 1) as u32;
                        }
                        (false, true) => {
                            // First triangle: CCW winding
                            indices[i] = index as u32;
                            indices[i + 1] = (index + Self::VERTEX_COUNT) as u32;
                            indices[i + 2] = (index + 1) as u32;

                            // Second triangle: CCW winding
                            indices[i + 3] = (index + Self::VERTEX_COUNT) as u32;
                            indices[i + 4] = (index + Self::VERTEX_COUNT + 1) as u32;
                            indices[i + 5] = (index + 1) as u32;
                        }
                        (true, true) => {
                            // First triangle: CCW winding
                            indices[i] = index as u32;
                            indices[i + 1] = (index + Self::VERTEX_COUNT) as u32;
                            indices[i + 2] = (index + Self::VERTEX_COUNT + 1) as u32;

                            // Second triangle: CCW winding
                            indices[i + 3] = index as u32;
                            indices[i + 4] = (index + Self::VERTEX_COUNT + 1) as u32;
                            indices[i + 5] = (index + 1) as u32;
                        }
                    }
                }
            }
        }

        info_once!("uvs: {uvs:?}");

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
